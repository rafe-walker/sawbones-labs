//! The in-memory link: a module, a line with a fault on it, and what is in
//! flight.

use sawbones_module_protocol::{
    crc32c, decode_request, encode_request, encode_response, ConfigKey, Request, RequestFrame,
    RequestHeader, Response, ResponseFrame, ResponseHeader, CHECK_LEN, MAX_FRAME_LEN,
};

use crate::fault::{Dir, Fault, Scenario};
use crate::module::SimModule;
use crate::rng::Rng;

/// How many frames the line remembers for replay, per direction.
const RECORD_LIMIT: usize = 4096;

/// A module on a line that may be faulty.
///
/// Time never passes by itself: the caller says what tick it is in every
/// call. Frames in flight carry the tick they arrive.
#[derive(Debug)]
pub struct SimLink {
    /// The module on the far end.
    pub module: SimModule,
    scenario: Option<Scenario>,
    rng: Rng,
    to_host: Vec<(u64, Vec<u8>)>,
    held: [Option<Vec<u8>>; 2],
    recorded: [Vec<Vec<u8>>; 2],
    fired: bool,
    /// Frames offered to the line, per direction (`[to module, to host]`).
    pub offered: [u64; 2],
    /// Frames the module saw.
    pub reached_module: u64,
}

impl SimLink {
    /// A line to `module` with `scenario` on it, if any. A module-fault
    /// scenario switches the fault on from its first tick.
    #[must_use]
    pub fn new(mut module: SimModule, scenario: Option<Scenario>) -> Self {
        if let Some(s) = scenario {
            if let Some(faults) = s.module_faults() {
                module.set_faults(s.from_tick, faults);
            }
        }
        SimLink {
            module,
            scenario,
            rng: Rng::new(scenario.map_or(0, |s| s.seed)),
            to_host: Vec::new(),
            held: [None, None],
            recorded: [Vec::new(), Vec::new()],
            fired: false,
            offered: [0; 2],
            reached_module: 0,
        }
    }

    /// The scenario on the line.
    #[must_use]
    pub const fn scenario(&self) -> Option<Scenario> {
        self.scenario
    }

    /// Frames recorded going `dir`, oldest first.
    #[must_use]
    pub fn recorded(&self, dir: Dir) -> &[Vec<u8>] {
        &self.recorded[dir.index()]
    }

    /// The host puts a frame on the line at tick `now`.
    pub fn send_from_host(&mut self, now: u64, frame: Vec<u8>) {
        self.offered[0] += 1;
        self.record(Dir::ToModule, &frame);
        for (at, frame) in self.transit(Dir::ToModule, now, frame) {
            self.deliver_to_module(at, &frame);
        }
    }

    /// Somebody else on the line puts a frame to the module, bypassing the
    /// host. The module's reply, if any, goes to the host like any other.
    pub fn inject_to_module(&mut self, now: u64, frame: &[u8]) {
        self.deliver_to_module(now, frame);
    }

    /// Somebody else on the line puts a frame to the host at tick `now`.
    pub fn inject_to_host(&mut self, now: u64, frame: Vec<u8>) {
        self.to_host.push((now, frame));
    }

    /// Reboots the module at `now`. See [`SimModule::reboot`].
    pub fn reboot_module(&mut self, now: u64, reseed_rng: bool) {
        self.module.reboot(now, reseed_rng);
    }

    /// The earliest frame that has reached the host by tick `now`, if any.
    pub fn next_for_host(&mut self, now: u64) -> Option<Vec<u8>> {
        let (index, _) = self
            .to_host
            .iter()
            .enumerate()
            .filter(|(_, (at, _))| *at <= now)
            .min_by_key(|(i, (at, _))| (*at, *i))?;
        Some(self.to_host.remove(index).1)
    }

    /// How many frames are on their way to the host.
    #[must_use]
    pub fn in_flight(&self) -> usize {
        self.to_host.len()
    }

    /// Lets the line do what happens by itself at tick `now`: the module
    /// notices time, a one-shot attack fires, a babbler babbles. Call it once
    /// per tick.
    pub fn pump(&mut self, now: u64) {
        self.module.poll(now);
        let Some(s) = self.scenario else { return };
        match s.fault {
            Fault::IdentifyReplay if !self.fired && now >= s.from_tick => {
                let captured = self.recorded[0].iter().find(|f| {
                    matches!(
                        decode_request(f),
                        Ok(RequestFrame {
                            request: Request::Identify { .. },
                            ..
                        })
                    )
                });
                if let Some(frame) = captured.cloned() {
                    self.fired = true;
                    self.inject_to_module(now, &frame);
                }
            }
            Fault::ReplayAcrossReboot { weak_session_rng } if !self.fired && now >= s.from_tick => {
                self.fired = true;
                let old_requests = self.recorded[0].clone();
                let old_replies = self.recorded[1].clone();
                self.reboot_module(now, weak_session_rng);
                for frame in old_requests {
                    let identify = matches!(
                        decode_request(&frame),
                        Ok(RequestFrame {
                            request: Request::Identify { .. },
                            ..
                        })
                    );
                    if !identify {
                        self.inject_to_module(now, &frame);
                    }
                }
                for frame in old_replies {
                    self.inject_to_host(now, frame);
                }
            }
            Fault::Babble { per_tick } if s.active(s.dir, now) => {
                for _ in 0..per_tick {
                    let frame = self.junk(s.dir);
                    match s.dir {
                        Dir::ToModule => self.deliver_to_module(now, &frame),
                        Dir::ToHost => self.to_host.push((now, frame)),
                    }
                }
            }
            Fault::StuckLine { level } if s.active(s.dir, now) => {
                let run = vec![level; 32];
                match s.dir {
                    Dir::ToModule => self.deliver_to_module(now, &run),
                    Dir::ToHost => self.to_host.push((now, run)),
                }
            }
            _ => {}
        }
    }

    fn record(&mut self, dir: Dir, frame: &[u8]) {
        let log = &mut self.recorded[dir.index()];
        if log.len() < RECORD_LIMIT {
            log.push(frame.to_vec());
        }
    }

    fn deliver_to_module(&mut self, at: u64, frame: &[u8]) {
        self.reached_module += 1;
        let Some(reply) = self.module.handle(at, frame) else {
            return;
        };
        self.offered[1] += 1;
        self.record(Dir::ToHost, &reply.bytes);
        for (due, bytes) in self.transit(Dir::ToHost, reply.due, reply.bytes) {
            self.to_host.push((due, bytes));
        }
    }

    /// Junk for a babbler: noise, and frames that decode, with the open
    /// session or a made-up one and any sequence numbers.
    fn junk(&mut self, dir: Dir) -> Vec<u8> {
        let session = match self.module.session() {
            Some(s) if self.rng.below(2) == 0 => s.get(),
            _ => self.rng.next_u64() | 1,
        };
        let seq = u32::try_from(self.rng.below(1 << 16)).unwrap_or(1) + 1;
        let mut out = [0u8; MAX_FRAME_LEN];
        match self.rng.below(3) {
            0 => {
                let len = 1 + usize::try_from(self.rng.below(60)).unwrap_or(1);
                (0..len).map(|_| self.rng.byte()).collect()
            }
            _ => {
                let n = match dir {
                    Dir::ToModule => encode_request(
                        &RequestFrame {
                            header: RequestHeader { session, seq },
                            request: Request::ConfigRead { key: ConfigKey(1) },
                        },
                        &mut out,
                    ),
                    Dir::ToHost => encode_response(
                        &ResponseFrame {
                            header: ResponseHeader {
                                session,
                                seq,
                                reply_to: seq,
                            },
                            response: Response::ConfigValue {
                                key: ConfigKey(1),
                                value: 0,
                            },
                        },
                        &mut out,
                    ),
                };
                out[..n.unwrap_or(0)].to_vec()
            }
        }
    }

    /// What a frame offered to the line at `now` going `dir` becomes.
    fn transit(&mut self, dir: Dir, now: u64, frame: Vec<u8>) -> Vec<(u64, Vec<u8>)> {
        let Some(s) = self.scenario.filter(|s| s.active(dir, now)) else {
            return vec![(now, frame)];
        };
        match s.fault {
            Fault::BitFlip { flips } => {
                let mut f = frame;
                let bits = f.len() * 8;
                let mut chosen: Vec<usize> = Vec::new();
                while chosen.len() < usize::from(flips).min(bits) {
                    let bit = usize::try_from(self.rng.below(bits as u64)).unwrap_or(0);
                    if !chosen.contains(&bit) {
                        chosen.push(bit);
                        f[bit / 8] ^= 1 << (bit % 8);
                    }
                }
                vec![(now, f)]
            }
            Fault::Truncate => {
                let keep = usize::try_from(self.rng.below(frame.len() as u64)).unwrap_or(0);
                vec![(now, frame[..keep].to_vec())]
            }
            Fault::Drop { one_in } => {
                if self.rng.below(u64::from(one_in)) == 0 {
                    Vec::new()
                } else {
                    vec![(now, frame)]
                }
            }
            Fault::Duplicate { copies } => (0..=copies).map(|_| (now, frame.clone())).collect(),
            Fault::Reorder => match self.held[dir.index()].take() {
                Some(earlier) => vec![(now, frame), (now, earlier)],
                None => {
                    self.held[dir.index()] = Some(frame);
                    Vec::new()
                }
            },
            Fault::Delay { ticks } => vec![(now.saturating_add(ticks), frame)],
            Fault::ReplayOld { lag } => {
                let log = &self.recorded[dir.index()];
                // A replayed `Identify` is its own attack: Fault::IdentifyReplay.
                let old = log
                    .len()
                    .checked_sub(1 + lag.max(1))
                    .map(|i| log[i].clone())
                    .filter(|f| {
                        !matches!(
                            decode_request(f),
                            Ok(RequestFrame {
                                request: Request::Identify { .. },
                                ..
                            })
                        )
                    });
                let mut out = vec![(now, frame)];
                out.extend(old.map(|f| (now, f)));
                out
            }
            Fault::Oversize { declared, fix_crc } => {
                let mut f = frame;
                if f.len() >= 6 {
                    f[4..6].copy_from_slice(&declared.to_le_bytes());
                    if fix_crc && f.len() > CHECK_LEN {
                        let body = f.len() - CHECK_LEN;
                        let crc = crc32c(&f[..body]);
                        f[body..].copy_from_slice(&crc.to_le_bytes());
                    }
                }
                vec![(now, f)]
            }
            Fault::Silent | Fault::StuckLine { .. } => Vec::new(),
            _ => vec![(now, frame)],
        }
    }
}
