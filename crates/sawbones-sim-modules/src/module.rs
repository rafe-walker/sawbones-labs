//! A simulated module: firmware behaviour on top of [`ModuleLink`].

use core::num::NonZeroU64;
use ed25519_dalek::{Signer, SigningKey};
use sawbones_module_protocol::{
    Challenge, ConfigKey, DescriptionPage, Dropped, HealthReport, HealthStatus, IdentityClaim,
    IdentitySignature, ImageDigest, LogEntries, LogEntry, LogLevel, ManifestDigest, ManifestHasher,
    ModuleBytes, ModuleId, ModuleLink, RefusalCode, Rejection, Request, ResetMode, Response,
    SelfTestOutcome, SelfTestReport, TimeReport, MAX_FRAME_LEN, MAX_LOG_BYTES,
};

use crate::kinds::{ModuleKind, READING_KEY_BASE};
use crate::rng::Rng;

/// The simulated module clock runs at this rate: one tick is a millisecond.
pub const TICK_HZ: u32 = 1000;

/// A fault the module itself has, the switches #99 lists. None of them is a
/// protocol violation: the frames stay valid, which is the point. Only
/// something that knows what the reading should be can tell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ModuleFaults {
    /// Readings freeze at what they were when the fault began, and the
    /// module still says it is healthy.
    pub stuck: bool,
    /// Readings move away from the truth by this much per tick.
    pub drift_per_tick: i64,
    /// The input is unplugged: readings are 0 and health is `Degraded`.
    pub disconnected: bool,
    /// Readings are this many ticks old, and the module says nothing.
    pub stale_ticks: u64,
    /// The identity names this module id instead of its own, signed with its
    /// own key.
    pub wrong_identity: Option<ModuleId>,
    /// Every reply is this many ticks late.
    pub slow_reply_ticks: u64,
    /// `Health` is never answered.
    pub lost_heartbeat: bool,
    /// The identity reports `can_emit` set after it was signed.
    pub overclaim_can_emit: bool,
}

/// A reply and the tick it reaches the line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reply {
    /// The encoded frame.
    pub bytes: Vec<u8>,
    /// The tick it is sent.
    pub due: u64,
}

/// How many rejections a module remembers.
const REJECTION_LOG: usize = 256;

/// A simulated module of one [`ModuleKind`].
///
/// It follows the firmware's half of the protocol contract: sessions come
/// from its own random number generator, it goes to its safe state when a
/// new session opens, on `SafeState`, on reset and when no request has been
/// accepted for `link_timeout` ticks. It reads no clock; every call is given
/// the tick.
#[derive(Debug)]
pub struct SimModule {
    kind: ModuleKind,
    id: ModuleId,
    signing_seed: [u8; 32],
    link: ModuleLink,
    rng_seed: u64,
    rng: Rng,
    /// Config slots 0 to 15.
    pub config: [i64; 16],
    /// Output channels 0 and 1. Zero is the safe state.
    pub output: [i32; 2],
    /// How many times the module went to its safe state, boots included.
    pub safe_entries: u32,
    /// How many class 2 or 3 commands it executed (config writes and
    /// actuations).
    pub executed: u32,
    /// The last frames it did not act on, oldest first.
    pub rejections: Vec<Rejection>,
    boot_tick: u64,
    last_accept: u64,
    link_timeout: u64,
    latency: u64,
    faults: ModuleFaults,
    fault_onset: u64,
    manifest: Vec<Vec<u8>>,
}

impl SimModule {
    /// A module of `kind` enrolled as `id`. `signing_seed` is its device key
    /// and `rng_seed` seeds its session ids and noise. Both come from the
    /// caller: no key lives in this crate.
    #[must_use]
    pub fn new(kind: ModuleKind, id: ModuleId, signing_seed: [u8; 32], rng_seed: u64) -> Self {
        SimModule {
            kind,
            id,
            signing_seed,
            link: ModuleLink::new(id),
            rng_seed,
            rng: Rng::new(rng_seed),
            config: [0; 16],
            output: [0; 2],
            safe_entries: 1,
            executed: 0,
            rejections: Vec::new(),
            boot_tick: 0,
            last_accept: 0,
            link_timeout: 200,
            latency: 1,
            faults: ModuleFaults::default(),
            fault_onset: 0,
            manifest: kind.manifest().iter().map(|p| p.to_vec()).collect(),
        }
    }

    /// Serves `pages` as the manifest instead of the kind's own, and signs
    /// their digest: a module whose installed manifest says something else,
    /// such as text written to steer an agent (#103). A page longer than
    /// [`MAX_PAGE_LEN`](sawbones_module_protocol::MAX_PAGE_LEN) is never
    /// served.
    #[must_use]
    pub fn with_manifest(mut self, pages: Vec<Vec<u8>>) -> Self {
        self.manifest = pages;
        self
    }

    /// The digest of the manifest this module serves and signs.
    #[must_use]
    pub fn manifest_digest(&self) -> ManifestDigest {
        let mut hasher = ManifestHasher::new();
        for page in &self.manifest {
            hasher.bytes(page);
        }
        hasher.finish()
    }

    /// Sets how many silent ticks end the session and send the module to its
    /// safe state.
    #[must_use]
    pub fn with_link_timeout(mut self, ticks: u64) -> Self {
        self.link_timeout = ticks;
        self
    }

    /// The kind.
    #[must_use]
    pub const fn kind(&self) -> ModuleKind {
        self.kind
    }

    /// The id the module is enrolled as.
    #[must_use]
    pub const fn id(&self) -> ModuleId {
        self.id
    }

    /// The open session, if any.
    #[must_use]
    pub const fn session(&self) -> Option<NonZeroU64> {
        self.link.session()
    }

    /// The link's counters.
    #[must_use]
    pub const fn counters(&self) -> sawbones_module_protocol::LinkCounters {
        self.link.counters()
    }

    /// The faults now switched on.
    #[must_use]
    pub const fn faults(&self) -> ModuleFaults {
        self.faults
    }

    /// Switches `faults` on from tick `now`. Stuck readings freeze at their
    /// value at `now`.
    pub fn set_faults(&mut self, now: u64, faults: ModuleFaults) {
        self.faults = faults;
        self.fault_onset = now;
    }

    /// Powers the module off and on at tick `now`: the session is gone, the
    /// outputs and config are reset and the safe state is entered. With
    /// `reseed_rng` the session id generator restarts from its seed, which is
    /// what a firmware that draws sessions from a weak or unseeded source
    /// does: the same session ids come round again. Real firmware must use
    /// the hardware generator.
    pub fn reboot(&mut self, now: u64, reseed_rng: bool) {
        self.link = ModuleLink::new(self.id);
        self.config = [0; 16];
        self.output = [0; 2];
        self.safe_entries += 1;
        self.boot_tick = now;
        self.last_accept = now;
        if reseed_rng {
            self.rng = Rng::new(self.rng_seed);
        }
    }

    fn safe(&mut self) {
        self.output = [0; 2];
        self.safe_entries += 1;
    }

    /// Lets the module notice time passing: no accepted request for the link
    /// timeout ends the session and enters the safe state. Call it with
    /// every tick that passes.
    pub fn poll(&mut self, now: u64) {
        if self.link.session().is_some() && now.saturating_sub(self.last_accept) > self.link_timeout
        {
            self.link.end_session();
            self.safe();
        }
    }

    /// The value the module reports for reading `key` at `now`, faults
    /// applied.
    #[must_use]
    pub fn reading(&self, key: u16, now: u64) -> Option<i64> {
        let f = &self.faults;
        let seen_at = now.saturating_sub(f.stale_ticks);
        let at = if f.stuck { self.fault_onset } else { seen_at };
        let truth = self.kind.truth(self.rng_seed, key, at)?;
        if f.disconnected {
            return Some(0);
        }
        let elapsed = i64::try_from(now.saturating_sub(self.fault_onset)).unwrap_or(i64::MAX);
        Some(truth.saturating_add(f.drift_per_tick.saturating_mul(elapsed)))
    }

    /// The digest of the module's (simulated) firmware image.
    #[must_use]
    pub fn firmware_digest(&self) -> ImageDigest {
        let mut out = [0u8; 32];
        for (i, chunk) in out.chunks_mut(8).enumerate() {
            let word = crate::rng::mix(self.rng_seed ^ (i as u64) << 56);
            chunk.copy_from_slice(&word.to_le_bytes());
        }
        ImageDigest(out)
    }

    fn identity(&self, challenge: &Challenge) -> Option<IdentityClaim> {
        let session = self.link.session()?;
        let mut claim = IdentityClaim {
            module: self.faults.wrong_identity.unwrap_or(self.id),
            capabilities: self.kind.capabilities(),
            manifest_digest: self.manifest_digest(),
            firmware_digest: self.firmware_digest(),
            signature: IdentitySignature([0; 64]),
        };
        let signature = SigningKey::from_bytes(&self.signing_seed)
            .sign(&claim.signed_bytes(challenge, session));
        claim.signature = IdentitySignature(signature.to_bytes());
        if self.faults.overclaim_can_emit {
            claim.capabilities.can_emit = true;
        }
        Some(claim)
    }

    /// Handles one frame that reached the module at tick `now`, and returns
    /// its reply, if it sends one.
    pub fn handle(&mut self, now: u64, bytes: &[u8]) -> Option<Reply> {
        self.poll(now);
        let table = self.kind.table();
        let rng = &mut self.rng;
        let accepted = match self.link.accept(bytes, || rng.next_u64(), &table) {
            Ok(accepted) => accepted,
            Err(Rejection::Drop(why)) => {
                self.remember(Rejection::Drop(why));
                return None;
            }
            Err(Rejection::Refuse { reply_to, code }) => {
                self.remember(Rejection::Refuse { reply_to, code });
                self.last_accept = now;
                return self.send(now, reply_to, &Response::Refused(code));
            }
        };
        self.last_accept = now;
        if accepted.new_session {
            self.safe();
        }
        let mut log_buf = [0u8; MAX_LOG_BYTES];
        let uptime = now.saturating_sub(self.boot_tick);
        let counters = self.link.counters();
        // A copy, so the reply does not borrow the module while it is sent.
        let (served_page, page_count) = match accepted.request {
            Request::Describe { page } => (
                self.manifest.get(usize::from(page)).cloned(),
                self.manifest.len(),
            ),
            _ => (None, 0),
        };
        let response = match accepted.request {
            Request::Identify { challenge } => Response::Identity(self.identity(&challenge)?),
            Request::Describe { page } => match served_page.as_deref() {
                Some(text) => Response::Description(DescriptionPage {
                    page,
                    page_count: u8::try_from(page_count).ok()?,
                    text: ModuleBytes::new(text)?,
                }),
                None => Response::Refused(RefusalCode::OutOfRange),
            },
            Request::Health => {
                if self.faults.lost_heartbeat {
                    return None;
                }
                let degraded = self.faults.disconnected;
                Response::Health(HealthReport {
                    status: if degraded {
                        HealthStatus::Degraded
                    } else {
                        HealthStatus::Ok
                    },
                    uptime_ticks: uptime,
                    fault_code: u16::from(degraded),
                    bad_frames: counters.bad_frames,
                    refused_frames: counters.dropped.saturating_add(counters.refused),
                })
            }
            Request::SelfTest => Response::SelfTest(SelfTestReport {
                outcome: SelfTestOutcome::Passed,
                code: 0,
            }),
            Request::Time { host_time_us } => Response::Time(TimeReport {
                host_time_us,
                module_ticks: uptime,
                tick_hz: TICK_HZ,
            }),
            Request::ConfigRead { key } => self.read(key, now),
            Request::ConfigWrite(w) => match self.config.get_mut(usize::from(w.key.0)) {
                Some(slot) if w.key.0 < READING_KEY_BASE => {
                    *slot = w.value;
                    self.executed += 1;
                    Response::ConfigValue {
                        key: w.key,
                        value: w.value,
                    }
                }
                _ => Response::Refused(RefusalCode::UnknownKey),
            },
            Request::LogRead { from, max } => {
                let end = from
                    .saturating_add(u32::from(max))
                    .min(from.saturating_add(3));
                let entries: Vec<LogEntry> = (from..end)
                    .map(|index| LogEntry {
                        index,
                        tick: u64::from(index) * 10,
                        level: LogLevel::Info,
                        code: u16::try_from(100 + index % 1000).unwrap_or(0),
                    })
                    .collect();
                Response::Log(LogEntries::write(&entries, &mut log_buf).ok()?)
            }
            Request::StageUpdate(chunk) => {
                let end = chunk
                    .offset
                    .saturating_add(u32::try_from(chunk.bytes.len()).ok()?);
                Response::UpdateStaged {
                    received: end,
                    complete: end == chunk.image_len,
                }
            }
            Request::Reset(ResetMode::Restart) => {
                let reply = self.send(now, accepted.seq, &Response::ResetAccepted);
                self.reboot(now, false);
                return reply;
            }
            // The simulation holds no trust root, so it never activates.
            Request::Reset(ResetMode::ActivateStaged { .. }) => {
                Response::Refused(RefusalCode::BadApproval)
            }
            Request::SafeState => {
                self.safe();
                Response::SafeStateEntered
            }
            Request::Actuate(a) => {
                let slot = self.output.get_mut(usize::from(a.channel.0))?;
                *slot = a.setting;
                self.executed += 1;
                Response::Actuated {
                    channel: a.channel,
                    setting: a.setting,
                }
            }
        };
        self.send(now, accepted.seq, &response)
    }

    fn read(&self, key: ConfigKey, now: u64) -> Response<'static> {
        let value = if key.0 >= READING_KEY_BASE {
            self.reading(key.0, now)
        } else {
            self.config.get(usize::from(key.0)).copied()
        };
        match value {
            Some(value) => Response::ConfigValue { key, value },
            None => Response::Refused(RefusalCode::UnknownKey),
        }
    }

    fn send(&mut self, now: u64, reply_to: u32, response: &Response<'_>) -> Option<Reply> {
        let mut out = [0u8; MAX_FRAME_LEN];
        let n = self.link.reply(reply_to, response, &mut out).ok()?;
        Some(Reply {
            bytes: out[..n].to_vec(),
            due: now
                .saturating_add(self.latency)
                .saturating_add(self.faults.slow_reply_ticks),
        })
    }

    fn remember(&mut self, rejection: Rejection) {
        if self.rejections.len() == REJECTION_LOG {
            self.rejections.remove(0);
        }
        self.rejections.push(rejection);
    }

    /// How many of the module's remembered rejections were drops for `why`.
    #[must_use]
    pub fn dropped_for(&self, pick: impl Fn(&Dropped) -> bool) -> usize {
        self.rejections
            .iter()
            .filter(|r| matches!(r, Rejection::Drop(d) if pick(d)))
            .count()
    }
}
