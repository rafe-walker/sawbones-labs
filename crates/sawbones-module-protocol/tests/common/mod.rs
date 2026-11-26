//! Shared test helpers: a deterministic generator, fixed test keys, random
//! messages and a small simulated module.
//!
//! The seeds here are TEST ONLY and public. They live under `tests/`, never
//! in `src/`, so no build of the crate contains them.
#![allow(dead_code)]

use core::num::NonZeroU64;

use ed25519_dalek::{Signer, SigningKey};
use sawbones_module_protocol::*;

/// TEST ONLY. The seed of module [`MODULE`].
pub const MODULE_SEED: [u8; 32] = *b"SAWBONES-TEST-KEY module-proto-7";
/// TEST ONLY. The seed of module [`OTHER_MODULE`].
pub const OTHER_SEED: [u8; 32] = *b"SAWBONES-TEST-KEY module-proto-8";

/// The module [`MODULE_SEED`] is enrolled as.
pub const MODULE: ModuleId = ModuleId(7);
/// The module [`OTHER_SEED`] is enrolled as.
pub const OTHER_MODULE: ModuleId = ModuleId(8);

pub fn key(seed: &[u8; 32]) -> ModuleKey {
    ModuleKey(SigningKey::from_bytes(seed).verifying_key().to_bytes())
}

/// A small xorshift generator, so every run sees the same sequence.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed | 1)
    }

    pub fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    pub fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }

    pub fn byte(&mut self) -> u8 {
        self.next() as u8
    }

    pub fn bool(&mut self) -> bool {
        self.next() & 1 == 1
    }

    pub fn array<const N: usize>(&mut self) -> [u8; N] {
        let mut out = [0u8; N];
        for b in &mut out {
            *b = self.byte();
        }
        out
    }

    pub fn fill(&mut self, out: &mut [u8]) {
        for b in out {
            *b = self.byte();
        }
    }
}

pub const MANIFEST: &[&[u8]] = &[
    b"name: dc-meter (simulated)\nchannels: 1\n",
    b"config: 1 averaging class 1; 2 range class 2; 3 trip class 3; 9 mains class 4\n",
];

pub fn manifest_digest() -> ManifestDigest {
    let mut h = ManifestHasher::new();
    for page in MANIFEST {
        h.bytes(page);
    }
    h.finish()
}

pub const FIRMWARE: ImageDigest = ImageDigest([0x5a; 32]);

/// An identity for `module`, signed with `seed`.
pub fn signed_identity(
    seed: &[u8; 32],
    module: ModuleId,
    capabilities: Capabilities,
    challenge: &Challenge,
    session: NonZeroU64,
) -> IdentityClaim {
    let mut claim = IdentityClaim {
        module,
        capabilities,
        manifest_digest: manifest_digest(),
        firmware_digest: FIRMWARE,
        signature: IdentitySignature([0; 64]),
    };
    let signature = SigningKey::from_bytes(seed).sign(&claim.signed_bytes(challenge, session));
    claim.signature = IdentitySignature(signature.to_bytes());
    claim
}

pub const CAPS: Capabilities = Capabilities {
    can_emit: false,
    max_class: HazardClass::Class3,
};

/// The simulated module's class table.
pub struct Table;

impl ClassTable for Table {
    fn config_class(&self, key: ConfigKey) -> Option<HazardClass> {
        match key.0 {
            1 => Some(HazardClass::Class1),
            2 => Some(HazardClass::Class2),
            3 => Some(HazardClass::Class3),
            9 => Some(HazardClass::Class4),
            _ => None,
        }
    }

    fn channel_class(&self, channel: Channel) -> Option<HazardClass> {
        match channel.0 {
            0 => Some(HazardClass::Class2),
            1 => Some(HazardClass::Class3),
            _ => None,
        }
    }
}

/// A simulated module that answers the protocol through [`ModuleLink`].
pub struct SimModule {
    pub link: ModuleLink,
    pub seed: [u8; 32],
    pub id: ModuleId,
    pub rng: Rng,
    pub config: [i64; 16],
    pub output: [i32; 2],
    pub safe_state_entries: u32,
    pub ticks: u64,
}

impl SimModule {
    pub fn new(seed: [u8; 32], id: ModuleId) -> Self {
        SimModule {
            link: ModuleLink::new(id),
            seed,
            id,
            rng: Rng::new(0x5eed_0000 + u64::from(id.0)),
            config: [0; 16],
            output: [0; 2],
            safe_state_entries: 0,
            ticks: 0,
        }
    }

    fn safe(&mut self) {
        self.output = [0; 2];
        self.safe_state_entries += 1;
    }

    /// Handles one frame and writes the reply, if any, into `out`.
    pub fn handle(&mut self, bytes: &[u8], out: &mut [u8]) -> Option<usize> {
        self.ticks += 1;
        let rng = &mut self.rng;
        let accepted = match self.link.accept(bytes, || rng.next(), &Table) {
            Ok(accepted) => accepted,
            Err(Rejection::Drop(_)) => return None,
            Err(Rejection::Refuse { reply_to, code }) => {
                return self
                    .link
                    .reply(reply_to, &Response::Refused(code), out)
                    .ok();
            }
        };
        if accepted.new_session {
            self.safe();
        }
        let mut log_buf = [0u8; MAX_LOG_BYTES];
        let counters = self.link.counters();
        let response = match accepted.request {
            Request::Identify { challenge } => {
                let session = self.link.session()?;
                Response::Identity(signed_identity(
                    &self.seed, self.id, CAPS, &challenge, session,
                ))
            }
            Request::Describe { page } => match MANIFEST.get(usize::from(page)) {
                Some(text) => Response::Description(DescriptionPage {
                    page,
                    page_count: MANIFEST.len() as u8,
                    text: ModuleBytes::new(text)?,
                }),
                None => Response::Refused(RefusalCode::OutOfRange),
            },
            Request::Health => Response::Health(HealthReport {
                status: HealthStatus::Ok,
                uptime_ticks: self.ticks,
                fault_code: 0,
                bad_frames: counters.bad_frames,
                refused_frames: counters.dropped + counters.refused,
            }),
            Request::SelfTest => Response::SelfTest(SelfTestReport {
                outcome: SelfTestOutcome::Passed,
                code: 0,
            }),
            Request::Time { host_time_us } => Response::Time(TimeReport {
                host_time_us,
                module_ticks: self.ticks,
                tick_hz: 1000,
            }),
            Request::ConfigRead { key } => match self.config.get(usize::from(key.0)) {
                Some(&value) => Response::ConfigValue { key, value },
                None => Response::Refused(RefusalCode::UnknownKey),
            },
            Request::ConfigWrite(w) => match self.config.get_mut(usize::from(w.key.0)) {
                Some(slot) => {
                    *slot = w.value;
                    Response::ConfigValue {
                        key: w.key,
                        value: w.value,
                    }
                }
                None => Response::Refused(RefusalCode::UnknownKey),
            },
            Request::LogRead { from, max } => {
                let entries: Vec<LogEntry> =
                    (from..from.saturating_add(u32::from(max)).min(from + 3))
                        .map(|index| LogEntry {
                            index,
                            tick: u64::from(index) * 10,
                            level: LogLevel::Info,
                            code: 100 + index as u16,
                        })
                        .collect();
                Response::Log(LogEntries::write(&entries, &mut log_buf).ok()?)
            }
            Request::StageUpdate(chunk) => Response::UpdateStaged {
                received: chunk.offset + chunk.bytes.len() as u32,
                complete: chunk.offset + chunk.bytes.len() as u32 == chunk.image_len,
            },
            Request::Reset(_) => Response::ResetAccepted,
            Request::SafeState => {
                self.safe();
                Response::SafeStateEntered
            }
            Request::Actuate(a) => {
                let slot = self.output.get_mut(usize::from(a.channel.0))?;
                *slot = a.setting;
                Response::Actuated {
                    channel: a.channel,
                    setting: a.setting,
                }
            }
        };
        self.link.reply(accepted.seq, &response, out).ok()
    }
}

/// A random authority, with a valid lease port.
pub fn random_authority(rng: &mut Rng) -> Authority {
    match rng.below(3) {
        0 => Authority::None,
        1 => Authority::RailA,
        _ => Authority::Lease(LeaseTarget {
            port: Port(rng.below(sawbones_interlock::RAIL_B_PORTS as u64) as u8),
            action_digest: ActionDigest(rng.array()),
        }),
    }
}

/// A random valid request. Chunk bytes borrow from `pool`.
pub fn random_request<'a>(rng: &mut Rng, pool: &'a [u8]) -> Request<'a> {
    match rng.below(12) {
        0 => Request::Identify {
            challenge: Challenge(rng.array()),
        },
        1 => Request::Describe {
            page: rng.below(u64::from(MAX_MANIFEST_PAGES)) as u8,
        },
        2 => Request::Health,
        3 => Request::SelfTest,
        4 => Request::Time {
            host_time_us: rng.next(),
        },
        5 => Request::ConfigRead {
            key: ConfigKey(rng.next() as u16),
        },
        6 => Request::ConfigWrite(ConfigWrite {
            key: ConfigKey(rng.next() as u16),
            value: rng.next() as i64,
            authority: random_authority(rng),
        }),
        7 => Request::LogRead {
            from: rng.next() as u32,
            max: 1 + rng.below(u64::from(MAX_LOG_ENTRIES)) as u8,
        },
        8 => {
            let len = 1 + rng.below(MAX_CHUNK_LEN.min(pool.len()) as u64) as usize;
            let offset = rng.below(1 << 20) as u32;
            let image_len = offset + len as u32 + rng.below(1 << 20) as u32;
            Request::StageUpdate(UpdateChunk {
                image_digest: ImageDigest(rng.array()),
                image_len,
                offset,
                bytes: &pool[..len],
            })
        }
        9 => Request::Reset(if rng.bool() {
            ResetMode::Restart
        } else {
            ResetMode::ActivateStaged {
                image_digest: ImageDigest(rng.array()),
                release: ReleaseApproval {
                    message: rng.array(),
                    signature: rng.array(),
                },
            }
        }),
        10 => Request::SafeState,
        _ => Request::Actuate(Actuate {
            channel: Channel(rng.byte()),
            setting: rng.next() as i32,
            authority: random_authority(rng),
        }),
    }
}

fn random_class(rng: &mut Rng) -> HazardClass {
    [
        HazardClass::Class0,
        HazardClass::Class1,
        HazardClass::Class2,
        HazardClass::Class3,
    ][rng.below(4) as usize]
}

/// A random valid response. Page text borrows from `pool`; log entries are
/// written into `log_buf`.
pub fn random_response<'a>(
    rng: &mut Rng,
    pool: &'a [u8],
    log_buf: &'a mut [u8; MAX_LOG_BYTES],
) -> Response<'a> {
    match rng.below(12) {
        0 => Response::Identity(IdentityClaim {
            module: ModuleId(rng.next() as u32),
            capabilities: Capabilities {
                can_emit: rng.bool(),
                max_class: random_class(rng),
            },
            manifest_digest: ManifestDigest(rng.array()),
            firmware_digest: ImageDigest(rng.array()),
            signature: IdentitySignature(rng.array()),
        }),
        1 => {
            let page_count = 1 + rng.below(u64::from(MAX_MANIFEST_PAGES)) as u8;
            let len = rng.below(MAX_PAGE_LEN.min(pool.len()) as u64 + 1) as usize;
            Response::Description(DescriptionPage {
                page: rng.below(u64::from(page_count)) as u8,
                page_count,
                text: ModuleBytes::new(&pool[..len]).unwrap(),
            })
        }
        2 => Response::Health(HealthReport {
            status: HealthStatus::ALL[rng.below(HealthStatus::ALL.len() as u64) as usize],
            uptime_ticks: rng.next(),
            fault_code: rng.next() as u16,
            bad_frames: rng.next() as u32,
            refused_frames: rng.next() as u32,
        }),
        3 => Response::SelfTest(SelfTestReport {
            outcome: SelfTestOutcome::ALL[rng.below(2) as usize],
            code: rng.next() as u16,
        }),
        4 => Response::Time(TimeReport {
            host_time_us: rng.next(),
            module_ticks: rng.next(),
            tick_hz: 1 + rng.below(u64::from(u32::MAX)) as u32,
        }),
        5 => Response::ConfigValue {
            key: ConfigKey(rng.next() as u16),
            value: rng.next() as i64,
        },
        6 => {
            let count = rng.below(u64::from(MAX_LOG_ENTRIES) + 1) as usize;
            let mut index = rng.below(1000) as u32;
            let entries: Vec<LogEntry> = (0..count)
                .map(|_| {
                    index += 1 + rng.below(5) as u32;
                    LogEntry {
                        index,
                        tick: rng.next(),
                        level: LogLevel::ALL[rng.below(4) as usize],
                        code: rng.next() as u16,
                    }
                })
                .collect();
            Response::Log(LogEntries::write(&entries, log_buf).unwrap())
        }
        7 => Response::UpdateStaged {
            received: rng.next() as u32,
            complete: rng.bool(),
        },
        8 => Response::ResetAccepted,
        9 => Response::SafeStateEntered,
        10 => Response::Actuated {
            channel: Channel(rng.byte()),
            setting: rng.next() as i32,
        },
        _ => Response::Refused(RefusalCode::ALL[rng.below(RefusalCode::ALL.len() as u64) as usize]),
    }
}

/// A valid request frame's bytes.
pub fn request_bytes(header: RequestHeader, request: Request<'_>) -> Vec<u8> {
    let mut out = [0u8; MAX_FRAME_LEN];
    let len = encode_request(&RequestFrame { header, request }, &mut out).unwrap();
    out[..len].to_vec()
}

/// A valid response frame's bytes.
pub fn response_bytes(header: ResponseHeader, response: Response<'_>) -> Vec<u8> {
    let mut out = [0u8; MAX_FRAME_LEN];
    let len = encode_response(&ResponseFrame { header, response }, &mut out).unwrap();
    out[..len].to_vec()
}

/// Rewrites a frame's check value after its bytes were changed.
pub fn fix_crc(frame: &mut [u8]) {
    let n = frame.len();
    if n >= CHECK_LEN {
        let crc = crc32c(&frame[..n - CHECK_LEN]);
        frame[n - CHECK_LEN..].copy_from_slice(&crc.to_le_bytes());
    }
}
