//! The typed messages and their payload encodings.

use crate::authority::{rank, ActionBody, Authority, Grant};
use crate::identity::{IdentityClaim, IdentitySignature, ManifestDigest};
use crate::untrusted::ModuleBytes;
use crate::wire::{Reader, Writer};
use crate::{DecodeError, EncodeError, HazardClass, LeaseTarget, ModuleId};

/// The most bytes one [`UpdateChunk`] carries.
pub const MAX_CHUNK_LEN: usize = 256;

/// The most bytes one manifest page ([`DescriptionPage`]) carries.
pub const MAX_PAGE_LEN: usize = 256;

/// The most pages a manifest has. With [`MAX_PAGE_LEN`] this caps a
/// manifest at 4 KiB.
pub const MAX_MANIFEST_PAGES: u8 = 16;

/// The most log entries one [`Response::Log`] carries.
pub const MAX_LOG_ENTRIES: u8 = 16;

/// The length of one encoded [`LogEntry`].
pub const LOG_ENTRY_LEN: usize = 15;

/// The length of a `sawbones-approvals` message, as a [`ReleaseApproval`]
/// carries it. A test checks it equals `sawbones_approvals::MESSAGE_LEN`.
pub const APPROVAL_MESSAGE_LEN: usize = 91;

/// The length of an Ed25519 signature.
pub const APPROVAL_SIGNATURE_LEN: usize = 64;

/// The most bytes of encoded entries one [`Response::Log`] carries, and the
/// size of the buffer [`LogEntries::write`] takes.
pub const MAX_LOG_BYTES: usize = MAX_LOG_ENTRIES as usize * LOG_ENTRY_LEN;

macro_rules! byte_enum {
    (
        $(#[$meta:meta])*
        pub enum $name:ident ($field:literal) {
            $( $(#[$vmeta:meta])* $variant:ident = $value:literal, )+
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum $name {
            $( $(#[$vmeta])* $variant = $value, )+
        }

        impl $name {
            /// Every value, in wire order.
            pub const ALL: &'static [$name] = &[$($name::$variant,)+];

            /// The wire byte.
            #[must_use]
            pub const fn to_u8(self) -> u8 {
                self as u8
            }

            /// The value for a wire byte, if there is one.
            #[must_use]
            pub const fn from_u8(value: u8) -> Option<$name> {
                match value {
                    $( $value => Some($name::$variant), )+
                    _ => None,
                }
            }

            #[allow(dead_code)]
            pub(crate) fn read(r: &mut Reader<'_>) -> Result<$name, DecodeError> {
                let value = r.u8()?;
                $name::from_u8(value).ok_or(DecodeError::BadTag { field: $field, value })
            }
        }
    };
}

byte_enum! {
    /// The kind byte of a request, host to module.
    pub enum RequestKind ("request kind") {
        /// [`Request::Identify`].
        Identify = 0x01,
        /// [`Request::Describe`].
        Describe = 0x02,
        /// [`Request::Health`].
        Health = 0x03,
        /// [`Request::SelfTest`].
        SelfTest = 0x04,
        /// [`Request::Time`].
        Time = 0x05,
        /// [`Request::ConfigRead`].
        ConfigRead = 0x06,
        /// [`Request::ConfigWrite`].
        ConfigWrite = 0x07,
        /// [`Request::LogRead`].
        LogRead = 0x08,
        /// [`Request::StageUpdate`].
        StageUpdate = 0x09,
        /// [`Request::Reset`].
        Reset = 0x0a,
        /// [`Request::SafeState`].
        SafeState = 0x0b,
        /// [`Request::Actuate`].
        Actuate = 0x0c,
    }
}

byte_enum! {
    /// The kind byte of a response, module to host. The high bit is set, so a
    /// frame reflected back to its sender never decodes as the other
    /// direction.
    pub enum ResponseKind ("response kind") {
        /// [`Response::Identity`].
        Identity = 0x81,
        /// [`Response::Description`].
        Description = 0x82,
        /// [`Response::Health`].
        Health = 0x83,
        /// [`Response::SelfTest`].
        SelfTest = 0x84,
        /// [`Response::Time`].
        Time = 0x85,
        /// [`Response::ConfigValue`].
        ConfigValue = 0x86,
        /// [`Response::Log`].
        Log = 0x87,
        /// [`Response::UpdateStaged`].
        UpdateStaged = 0x88,
        /// [`Response::ResetAccepted`].
        ResetAccepted = 0x89,
        /// [`Response::SafeStateEntered`].
        SafeStateEntered = 0x8a,
        /// [`Response::Actuated`].
        Actuated = 0x8b,
        /// [`Response::Refused`].
        Refused = 0x8c,
    }
}

impl RequestKind {
    /// The response that answers this request. [`ResponseKind::Refused`]
    /// answers any request too.
    #[must_use]
    pub const fn expected_reply(self) -> ResponseKind {
        match self {
            RequestKind::Identify => ResponseKind::Identity,
            RequestKind::Describe => ResponseKind::Description,
            RequestKind::Health => ResponseKind::Health,
            RequestKind::SelfTest => ResponseKind::SelfTest,
            RequestKind::Time => ResponseKind::Time,
            RequestKind::ConfigRead | RequestKind::ConfigWrite => ResponseKind::ConfigValue,
            RequestKind::LogRead => ResponseKind::Log,
            RequestKind::StageUpdate => ResponseKind::UpdateStaged,
            RequestKind::Reset => ResponseKind::ResetAccepted,
            RequestKind::SafeState => ResponseKind::SafeStateEntered,
            RequestKind::Actuate => ResponseKind::Actuated,
        }
    }
}

byte_enum! {
    /// Why a module refused a request it decoded.
    pub enum RefusalCode ("refusal code") {
        /// The module is busy with something else.
        Busy = 1,
        /// No such config key or output channel.
        UnknownKey = 2,
        /// The authority carried is below the class the module's own table
        /// gives the action.
        AuthorityTooLow = 3,
        /// The action is hazard class 4 in the module's table. Never done.
        ClassNeverPermitted = 4,
        /// The lease's digest is not this action's on this module.
        LeaseNotForThisAction = 5,
        /// Activation asked for, but no complete image is staged.
        NothingStaged = 6,
        /// The release approval did not verify or names another image.
        BadApproval = 7,
        /// An update chunk out of order or for another image.
        BadUpdateChunk = 8,
        /// A value outside what the module accepts.
        OutOfRange = 9,
        /// The module is faulted.
        Faulted = 10,
        /// The interlock rail the authority needs is not sensed live.
        RailNotLive = 11,
    }
}

byte_enum! {
    /// A module's own view of its health. A claim, like everything it says.
    pub enum HealthStatus ("health status") {
        /// Working.
        Ok = 0,
        /// Working, with a problem it reports in `fault_code`.
        Degraded = 1,
        /// Not working.
        Faulted = 2,
        /// In its safe state.
        SafeState = 3,
    }
}

byte_enum! {
    /// The outcome of a self-test. A pass raises no trust.
    pub enum SelfTestOutcome ("self-test outcome") {
        /// The module says it passed.
        Passed = 0,
        /// The module says it failed.
        Failed = 1,
    }
}

byte_enum! {
    /// A log entry's level.
    pub enum LogLevel ("log level") {
        /// Debug.
        Debug = 0,
        /// Information.
        Info = 1,
        /// A warning.
        Warn = 2,
        /// An error.
        Error = 3,
    }
}

/// A fresh value from the host that the module's identity signature covers,
/// so an old signature cannot answer a new challenge. The host draws it from
/// its own random number generator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Challenge(pub [u8; 16]);

/// A config key. Its meaning and hazard class come from the module's
/// manifest; the module enforces the class from its own table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ConfigKey(pub u16);

/// An output channel on a module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Channel(pub u8);

/// The SHA-256 digest of a firmware image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ImageDigest(pub [u8; 32]);

/// A config write: the key, the value and the authority for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ConfigWrite {
    /// The key.
    pub key: ConfigKey,
    /// The value.
    pub value: i64,
    /// What the write carries for the interlock.
    pub authority: Authority,
}

impl ConfigWrite {
    /// A config write for `module`, with the authority `grant` asks for. A
    /// lease's digest is computed for exactly this write.
    #[must_use]
    pub fn new(module: ModuleId, key: ConfigKey, value: i64, grant: Grant) -> ConfigWrite {
        let authority = grant.authority(module, ActionBody::ConfigWrite { key, value });
        ConfigWrite {
            key,
            value,
            authority,
        }
    }

    /// What a lease for this write approves.
    #[must_use]
    pub const fn body(&self) -> ActionBody {
        ActionBody::ConfigWrite {
            key: self.key,
            value: self.value,
        }
    }
}

/// Drive an output channel: the channel, the setting and the authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Actuate {
    /// The channel.
    pub channel: Channel,
    /// The setting, in the channel's units from the manifest.
    pub setting: i32,
    /// What the command carries for the interlock.
    pub authority: Authority,
}

impl Actuate {
    /// An actuation on `module`, with the authority `grant` asks for. A
    /// lease's digest is computed for exactly this actuation.
    #[must_use]
    pub fn new(module: ModuleId, channel: Channel, setting: i32, grant: Grant) -> Actuate {
        let authority = grant.authority(module, ActionBody::Actuate { channel, setting });
        Actuate {
            channel,
            setting,
            authority,
        }
    }

    /// What a lease for this actuation approves.
    #[must_use]
    pub const fn body(&self) -> ActionBody {
        ActionBody::Actuate {
            channel: self.channel,
            setting: self.setting,
        }
    }
}

/// One piece of a firmware image, staged into the module's inactive slot.
/// Staging changes nothing that runs; only [`ResetMode::ActivateStaged`]
/// does.
///
/// Bounds, checked on encode and decode: 1 to [`MAX_CHUNK_LEN`] bytes, and
/// `offset + bytes.len()` no more than `image_len`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UpdateChunk<'a> {
    /// The digest of the whole image.
    pub image_digest: ImageDigest,
    /// The whole image's length.
    pub image_len: u32,
    /// Where this chunk starts in the image.
    pub offset: u32,
    /// The chunk.
    pub bytes: &'a [u8],
}

/// A trust-root release approval, as `sawbones-approvals` encodes and signs
/// it: the message bytes and the Ed25519 signature. The firmware verifies it
/// with that crate before activating anything; this crate only carries it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ReleaseApproval {
    /// The approval message.
    pub message: [u8; APPROVAL_MESSAGE_LEN],
    /// The signature over it.
    pub signature: [u8; APPROVAL_SIGNATURE_LEN],
}

/// How to reset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResetMode {
    /// Restart the running image.
    Restart,
    /// Boot the staged image. A reflash makes every fact the module produced
    /// unverified again until it passes the known-answer check (#110).
    ActivateStaged {
        /// The staged image's digest, which the approval must name.
        image_digest: ImageDigest,
        /// The trust root's countersign for the release.
        release: ReleaseApproval,
    },
}

/// A request, host to module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Request<'a> {
    /// Open a session and prove who you are. Sent in session 0; the module
    /// answers from a fresh session with [`Response::Identity`].
    Identify {
        /// The host's fresh challenge.
        challenge: Challenge,
    },
    /// Send this page of your manifest.
    Describe {
        /// The page, from 0, below [`MAX_MANIFEST_PAGES`].
        page: u8,
    },
    /// Report your health.
    Health,
    /// Run your self-test. Class 1: a test that drives anything past logic
    /// power is an [`Request::Actuate`] instead.
    SelfTest,
    /// Report your clock.
    Time {
        /// The host's clock, in microseconds, echoed back.
        host_time_us: u64,
    },
    /// Read a config value.
    ConfigRead {
        /// The key.
        key: ConfigKey,
    },
    /// Write a config value.
    ConfigWrite(ConfigWrite),
    /// Send log entries.
    LogRead {
        /// The first entry index wanted.
        from: u32,
        /// The most entries wanted, 1 to [`MAX_LOG_ENTRIES`].
        max: u8,
    },
    /// Stage a piece of a firmware image.
    StageUpdate(UpdateChunk<'a>),
    /// Reset.
    Reset(ResetMode),
    /// Go to your safe state now. Always class 0: it only ever makes things
    /// safer.
    SafeState,
    /// Drive an output.
    Actuate(Actuate),
}

impl Request<'_> {
    /// The kind byte.
    #[must_use]
    pub const fn kind(&self) -> RequestKind {
        match self {
            Request::Identify { .. } => RequestKind::Identify,
            Request::Describe { .. } => RequestKind::Describe,
            Request::Health => RequestKind::Health,
            Request::SelfTest => RequestKind::SelfTest,
            Request::Time { .. } => RequestKind::Time,
            Request::ConfigRead { .. } => RequestKind::ConfigRead,
            Request::ConfigWrite(_) => RequestKind::ConfigWrite,
            Request::LogRead { .. } => RequestKind::LogRead,
            Request::StageUpdate(_) => RequestKind::StageUpdate,
            Request::Reset(_) => RequestKind::Reset,
            Request::SafeState => RequestKind::SafeState,
            Request::Actuate(_) => RequestKind::Actuate,
        }
    }

    /// The authority carried, for a command that carries one.
    #[must_use]
    pub const fn authority(&self) -> Option<Authority> {
        match self {
            Request::ConfigWrite(w) => Some(w.authority),
            Request::Actuate(a) => Some(a.authority),
            _ => None,
        }
    }

    /// What a lease would approve, for a command that can carry one.
    #[must_use]
    pub const fn action_body(&self) -> Option<ActionBody> {
        match self {
            Request::ConfigWrite(w) => Some(w.body()),
            Request::Actuate(a) => Some(a.body()),
            _ => None,
        }
    }

    /// The interlock lease target this request spends, for a class 3
    /// command. The supervisor asks the interlock for exactly this target and
    /// reports `LeaseUsed` with it.
    #[must_use]
    pub const fn lease_target(&self) -> Option<LeaseTarget> {
        match self.authority() {
            Some(authority) => authority.lease_target(),
            None => None,
        }
    }

    /// The hazard class of a request whose class is fixed by its kind. A
    /// config write and an actuation have the class the module's own table
    /// gives them, so they return `None`.
    #[must_use]
    pub const fn fixed_class(&self) -> Option<HazardClass> {
        match self {
            Request::Identify { .. }
            | Request::Describe { .. }
            | Request::Health
            | Request::Time { .. }
            | Request::ConfigRead { .. }
            | Request::LogRead { .. }
            | Request::SafeState => Some(HazardClass::Class0),
            Request::SelfTest | Request::StageUpdate(_) | Request::Reset(_) => {
                Some(HazardClass::Class1)
            }
            Request::ConfigWrite(_) | Request::Actuate(_) => None,
        }
    }

    /// Checks the bounds decoding checks.
    pub(crate) fn validate(&self) -> Result<(), DecodeError> {
        match self {
            Request::Describe { page } if *page >= MAX_MANIFEST_PAGES => {
                Err(DecodeError::OutOfBounds { field: "page" })
            }
            Request::LogRead { max, .. } if *max == 0 || *max > MAX_LOG_ENTRIES => {
                Err(DecodeError::OutOfBounds { field: "log max" })
            }
            Request::StageUpdate(chunk) => chunk.validate(),
            Request::ConfigWrite(ConfigWrite { authority, .. })
            | Request::Actuate(Actuate { authority, .. }) => match authority {
                Authority::Lease(target) if !target.port.exists() => {
                    Err(DecodeError::OutOfBounds {
                        field: "lease port",
                    })
                }
                _ => Ok(()),
            },
            _ => Ok(()),
        }
    }

    pub(crate) fn encode_payload(&self, w: &mut Writer<'_>) -> Result<(), EncodeError> {
        self.validate()?;
        match self {
            Request::Identify { challenge } => w.put(&challenge.0),
            Request::Describe { page } => w.u8(*page),
            Request::Health | Request::SelfTest | Request::SafeState => Ok(()),
            Request::Time { host_time_us } => w.u64(*host_time_us),
            Request::ConfigRead { key } => w.u16(key.0),
            Request::ConfigWrite(c) => {
                w.u16(c.key.0)?;
                w.i64(c.value)?;
                c.authority.encode(w)
            }
            Request::LogRead { from, max } => {
                w.u32(*from)?;
                w.u8(*max)
            }
            Request::StageUpdate(chunk) => {
                w.put(&chunk.image_digest.0)?;
                w.u32(chunk.image_len)?;
                w.u32(chunk.offset)?;
                w.sized(chunk.bytes, MAX_CHUNK_LEN, "chunk")
            }
            Request::Reset(ResetMode::Restart) => w.u8(0),
            Request::Reset(ResetMode::ActivateStaged {
                image_digest,
                release,
            }) => {
                w.u8(1)?;
                w.put(&image_digest.0)?;
                w.put(&release.message)?;
                w.put(&release.signature)
            }
            Request::Actuate(a) => {
                w.u8(a.channel.0)?;
                w.i32(a.setting)?;
                a.authority.encode(w)
            }
        }
    }
}

impl<'a> Request<'a> {
    pub(crate) fn decode_payload(
        kind: RequestKind,
        payload: &'a [u8],
    ) -> Result<Request<'a>, DecodeError> {
        let mut r = Reader::new(payload);
        let request = match kind {
            RequestKind::Identify => Request::Identify {
                challenge: Challenge(r.array()?),
            },
            RequestKind::Describe => Request::Describe { page: r.u8()? },
            RequestKind::Health => Request::Health,
            RequestKind::SelfTest => Request::SelfTest,
            RequestKind::Time => Request::Time {
                host_time_us: r.u64()?,
            },
            RequestKind::ConfigRead => Request::ConfigRead {
                key: ConfigKey(r.u16()?),
            },
            RequestKind::ConfigWrite => Request::ConfigWrite(ConfigWrite {
                key: ConfigKey(r.u16()?),
                value: r.i64()?,
                authority: Authority::decode(&mut r)?,
            }),
            RequestKind::LogRead => Request::LogRead {
                from: r.u32()?,
                max: r.u8()?,
            },
            RequestKind::StageUpdate => Request::StageUpdate(UpdateChunk {
                image_digest: ImageDigest(r.array()?),
                image_len: r.u32()?,
                offset: r.u32()?,
                bytes: r.sized(MAX_CHUNK_LEN, "chunk")?,
            }),
            RequestKind::Reset => match r.u8()? {
                0 => Request::Reset(ResetMode::Restart),
                1 => Request::Reset(ResetMode::ActivateStaged {
                    image_digest: ImageDigest(r.array()?),
                    release: ReleaseApproval {
                        message: r.array()?,
                        signature: r.array()?,
                    },
                }),
                value => {
                    return Err(DecodeError::BadTag {
                        field: "reset mode",
                        value,
                    })
                }
            },
            RequestKind::SafeState => Request::SafeState,
            RequestKind::Actuate => Request::Actuate(Actuate {
                channel: Channel(r.u8()?),
                setting: r.i32()?,
                authority: Authority::decode(&mut r)?,
            }),
        };
        r.finish()?;
        request.validate()?;
        Ok(request)
    }
}

impl UpdateChunk<'_> {
    fn validate(&self) -> Result<(), DecodeError> {
        if self.bytes.is_empty() || self.bytes.len() > MAX_CHUNK_LEN {
            return Err(DecodeError::OutOfBounds { field: "chunk" });
        }
        // The length fits in a u32: it is at most MAX_CHUNK_LEN.
        let len = u32::try_from(self.bytes.len())
            .map_err(|_| DecodeError::OutOfBounds { field: "chunk" })?;
        match self.offset.checked_add(len) {
            Some(end) if end <= self.image_len => Ok(()),
            _ => Err(DecodeError::OutOfBounds {
                field: "chunk range",
            }),
        }
    }
}

/// What a module claims it can do. Covered by the identity signature, so a
/// changed claim with no new signature does not verify.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Capabilities {
    /// The module can transmit or radiate (review round 2). A transmitter
    /// needs a rail and the policy's approval for any emission.
    pub can_emit: bool,
    /// The highest hazard class of any command the module accepts. Never
    /// class 4: that does not encode.
    pub max_class: HazardClass,
}

impl Capabilities {
    pub(crate) fn validate(&self) -> Result<(), DecodeError> {
        if matches!(self.max_class, HazardClass::Class4) {
            Err(DecodeError::ClassNeverPermitted)
        } else {
            Ok(())
        }
    }

    pub(crate) fn class_byte(&self) -> u8 {
        rank(self.max_class)
    }

    pub(crate) fn decode(r: &mut Reader<'_>) -> Result<Capabilities, DecodeError> {
        let can_emit = r.bool()?;
        let max_class = match r.u8()? {
            0 => HazardClass::Class0,
            1 => HazardClass::Class1,
            2 => HazardClass::Class2,
            3 => HazardClass::Class3,
            4 => return Err(DecodeError::ClassNeverPermitted),
            value => {
                return Err(DecodeError::BadTag {
                    field: "max class",
                    value,
                })
            }
        };
        Ok(Capabilities {
            can_emit,
            max_class,
        })
    }
}

/// One page of a module's manifest. The text is the module's, untrusted. The
/// host hashes every page with a [`ManifestHasher`](crate::ManifestHasher)
/// and compares the digest with the signed one and the one pinned at
/// install.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DescriptionPage<'a> {
    /// This page, from 0.
    pub page: u8,
    /// How many pages, 1 to [`MAX_MANIFEST_PAGES`].
    pub page_count: u8,
    /// The page's bytes, at most [`MAX_PAGE_LEN`].
    pub text: ModuleBytes<'a>,
}

impl DescriptionPage<'_> {
    fn validate(&self) -> Result<(), DecodeError> {
        if self.page_count == 0 || self.page_count > MAX_MANIFEST_PAGES {
            return Err(DecodeError::OutOfBounds {
                field: "page count",
            });
        }
        if self.page >= self.page_count {
            return Err(DecodeError::OutOfBounds { field: "page" });
        }
        Ok(())
    }
}

/// A module's health report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HealthReport {
    /// Its status.
    pub status: HealthStatus,
    /// Ticks since it booted.
    pub uptime_ticks: u64,
    /// A fault code from the manifest's table, 0 for none.
    pub fault_code: u16,
    /// Frames it could not decode.
    pub bad_frames: u32,
    /// Frames it dropped or refused (replays, wrong session, authority).
    pub refused_frames: u32,
}

/// A self-test's outcome. Never a reason to trust a module more.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SelfTestReport {
    /// What the module says happened.
    pub outcome: SelfTestOutcome,
    /// A code from the manifest's table.
    pub code: u16,
}

/// A module's clock, against the host's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TimeReport {
    /// The host's clock from the request, echoed.
    pub host_time_us: u64,
    /// The module's monotonic tick count.
    pub module_ticks: u64,
    /// The module's ticks per second. Never 0.
    pub tick_hz: u32,
}

/// One log entry. Codes only, never text: the manifest maps a code to its
/// meaning, so a module cannot put instructions in its log.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LogEntry {
    /// The entry's index in the module's log.
    pub index: u32,
    /// The module tick it was written at.
    pub tick: u64,
    /// Its level.
    pub level: LogLevel,
    /// The event code.
    pub code: u16,
}

impl LogEntry {
    fn encode(&self) -> [u8; LOG_ENTRY_LEN] {
        let mut out = [0u8; LOG_ENTRY_LEN];
        let mut w = Writer::new(&mut out);
        // The buffer is exactly one entry long, so these cannot fail.
        let _ = w.u32(self.index);
        let _ = w.u64(self.tick);
        let _ = w.u8(self.level.to_u8());
        let _ = w.u16(self.code);
        out
    }

    fn decode(bytes: &[u8]) -> Result<LogEntry, DecodeError> {
        let mut r = Reader::new(bytes);
        let entry = LogEntry {
            index: r.u32()?,
            tick: r.u64()?,
            level: LogLevel::read(&mut r)?,
            code: r.u16()?,
        };
        r.finish()?;
        Ok(entry)
    }
}

/// Up to [`MAX_LOG_ENTRIES`] log entries with strictly rising indexes, held
/// as their checked encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LogEntries<'a> {
    bytes: &'a [u8],
}

impl<'a> LogEntries<'a> {
    /// No entries.
    pub const EMPTY: LogEntries<'static> = LogEntries { bytes: &[] };

    /// Encodes `entries` into `buf` for a module to send.
    ///
    /// # Errors
    ///
    /// [`EncodeError::Invalid`] if there are more than [`MAX_LOG_ENTRIES`]
    /// or the indexes do not strictly rise.
    pub fn write(
        entries: &[LogEntry],
        buf: &'a mut [u8; MAX_LOG_BYTES],
    ) -> Result<LogEntries<'a>, EncodeError> {
        if entries.len() > usize::from(MAX_LOG_ENTRIES) {
            return Err(EncodeError::Invalid(DecodeError::OutOfBounds {
                field: "log entries",
            }));
        }
        let mut w = Writer::new(buf);
        let mut last: Option<u32> = None;
        for entry in entries {
            if last.is_some_and(|l| entry.index <= l) {
                return Err(EncodeError::Invalid(DecodeError::OutOfBounds {
                    field: "log index",
                }));
            }
            last = Some(entry.index);
            w.put(&entry.encode())?;
        }
        let len = w.len();
        let bytes: &'a [u8] = buf;
        Ok(LogEntries {
            bytes: bytes.get(..len).unwrap_or(&[]),
        })
    }

    fn check(bytes: &'a [u8]) -> Result<LogEntries<'a>, DecodeError> {
        let mut last: Option<u32> = None;
        for chunk in bytes.chunks(LOG_ENTRY_LEN) {
            let entry = LogEntry::decode(chunk)?;
            if last.is_some_and(|l| entry.index <= l) {
                return Err(DecodeError::OutOfBounds { field: "log index" });
            }
            last = Some(entry.index);
        }
        Ok(LogEntries { bytes })
    }

    /// How many entries.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.bytes.len() / LOG_ENTRY_LEN
    }

    /// True if there are none.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    /// The entries, in order.
    pub fn iter(&self) -> impl Iterator<Item = LogEntry> + 'a {
        // Every chunk was checked when this value was made.
        self.bytes
            .chunks_exact(LOG_ENTRY_LEN)
            .filter_map(|chunk| LogEntry::decode(chunk).ok())
    }
}

/// A response, module to host. On the host it arrives inside
/// [`FromModule`](crate::FromModule): every field is the module's claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Response<'a> {
    /// Who the module says it is, signed over the host's challenge.
    Identity(IdentityClaim),
    /// A page of the manifest.
    Description(DescriptionPage<'a>),
    /// Its health.
    Health(HealthReport),
    /// A self-test's outcome.
    SelfTest(SelfTestReport),
    /// Its clock.
    Time(TimeReport),
    /// A config value, after a read or a write.
    ConfigValue {
        /// The key.
        key: ConfigKey,
        /// The value now held.
        value: i64,
    },
    /// Log entries.
    Log(LogEntries<'a>),
    /// An update chunk was staged.
    UpdateStaged {
        /// How many bytes of the image are staged.
        received: u32,
        /// The whole image is staged and its digest matched.
        complete: bool,
    },
    /// The module will reset after sending this.
    ResetAccepted,
    /// The module is in its safe state.
    SafeStateEntered,
    /// An output was driven.
    Actuated {
        /// The channel.
        channel: Channel,
        /// The setting now held.
        setting: i32,
    },
    /// The module refused the request.
    Refused(RefusalCode),
}

impl Response<'_> {
    /// The kind byte.
    #[must_use]
    pub const fn kind(&self) -> ResponseKind {
        match self {
            Response::Identity(_) => ResponseKind::Identity,
            Response::Description(_) => ResponseKind::Description,
            Response::Health(_) => ResponseKind::Health,
            Response::SelfTest(_) => ResponseKind::SelfTest,
            Response::Time(_) => ResponseKind::Time,
            Response::ConfigValue { .. } => ResponseKind::ConfigValue,
            Response::Log(_) => ResponseKind::Log,
            Response::UpdateStaged { .. } => ResponseKind::UpdateStaged,
            Response::ResetAccepted => ResponseKind::ResetAccepted,
            Response::SafeStateEntered => ResponseKind::SafeStateEntered,
            Response::Actuated { .. } => ResponseKind::Actuated,
            Response::Refused(_) => ResponseKind::Refused,
        }
    }

    pub(crate) fn validate(&self) -> Result<(), DecodeError> {
        match self {
            Response::Identity(claim) => claim.capabilities.validate(),
            Response::Description(page) => page.validate(),
            Response::Time(t) if t.tick_hz == 0 => {
                Err(DecodeError::OutOfBounds { field: "tick hz" })
            }
            _ => Ok(()),
        }
    }

    pub(crate) fn encode_payload(&self, w: &mut Writer<'_>) -> Result<(), EncodeError> {
        self.validate()?;
        match self {
            Response::Identity(c) => {
                w.u32(c.module.0)?;
                w.bool(c.capabilities.can_emit)?;
                w.u8(c.capabilities.class_byte())?;
                w.put(&c.manifest_digest.0)?;
                w.put(&c.firmware_digest.0)?;
                w.put(&c.signature.0)
            }
            Response::Description(p) => {
                w.u8(p.page)?;
                w.u8(p.page_count)?;
                w.sized(p.text.as_untrusted_bytes(), MAX_PAGE_LEN, "page text")
            }
            Response::Health(h) => {
                w.u8(h.status.to_u8())?;
                w.u64(h.uptime_ticks)?;
                w.u16(h.fault_code)?;
                w.u32(h.bad_frames)?;
                w.u32(h.refused_frames)
            }
            Response::SelfTest(s) => {
                w.u8(s.outcome.to_u8())?;
                w.u16(s.code)
            }
            Response::Time(t) => {
                w.u64(t.host_time_us)?;
                w.u64(t.module_ticks)?;
                w.u32(t.tick_hz)
            }
            Response::ConfigValue { key, value } => {
                w.u16(key.0)?;
                w.i64(*value)
            }
            Response::Log(entries) => {
                // At most MAX_LOG_ENTRIES, by construction.
                let count = u8::try_from(entries.len()).map_err(|_| {
                    EncodeError::Invalid(DecodeError::OutOfBounds {
                        field: "log entries",
                    })
                })?;
                w.u8(count)?;
                w.put(entries.bytes)
            }
            Response::UpdateStaged { received, complete } => {
                w.u32(*received)?;
                w.bool(*complete)
            }
            Response::ResetAccepted | Response::SafeStateEntered => Ok(()),
            Response::Actuated { channel, setting } => {
                w.u8(channel.0)?;
                w.i32(*setting)
            }
            Response::Refused(code) => w.u8(code.to_u8()),
        }
    }
}

impl<'a> Response<'a> {
    pub(crate) fn decode_payload(
        kind: ResponseKind,
        payload: &'a [u8],
    ) -> Result<Response<'a>, DecodeError> {
        let mut r = Reader::new(payload);
        let response = match kind {
            ResponseKind::Identity => Response::Identity(IdentityClaim {
                module: ModuleId(r.u32()?),
                capabilities: Capabilities::decode(&mut r)?,
                manifest_digest: ManifestDigest(r.array()?),
                firmware_digest: ImageDigest(r.array()?),
                signature: IdentitySignature(r.array()?),
            }),
            ResponseKind::Description => {
                let page = r.u8()?;
                let page_count = r.u8()?;
                let text = r.sized(MAX_PAGE_LEN, "page text")?;
                Response::Description(DescriptionPage {
                    page,
                    page_count,
                    text: ModuleBytes::new(text)
                        .ok_or(DecodeError::OutOfBounds { field: "page text" })?,
                })
            }
            ResponseKind::Health => Response::Health(HealthReport {
                status: HealthStatus::read(&mut r)?,
                uptime_ticks: r.u64()?,
                fault_code: r.u16()?,
                bad_frames: r.u32()?,
                refused_frames: r.u32()?,
            }),
            ResponseKind::SelfTest => Response::SelfTest(SelfTestReport {
                outcome: SelfTestOutcome::read(&mut r)?,
                code: r.u16()?,
            }),
            ResponseKind::Time => Response::Time(TimeReport {
                host_time_us: r.u64()?,
                module_ticks: r.u64()?,
                tick_hz: r.u32()?,
            }),
            ResponseKind::ConfigValue => Response::ConfigValue {
                key: ConfigKey(r.u16()?),
                value: r.i64()?,
            },
            ResponseKind::Log => {
                let count = r.u8()?;
                if count > MAX_LOG_ENTRIES {
                    return Err(DecodeError::OutOfBounds {
                        field: "log entries",
                    });
                }
                let bytes = r.take(usize::from(count) * LOG_ENTRY_LEN)?;
                Response::Log(LogEntries::check(bytes)?)
            }
            ResponseKind::UpdateStaged => Response::UpdateStaged {
                received: r.u32()?,
                complete: r.bool()?,
            },
            ResponseKind::ResetAccepted => Response::ResetAccepted,
            ResponseKind::SafeStateEntered => Response::SafeStateEntered,
            ResponseKind::Actuated => Response::Actuated {
                channel: Channel(r.u8()?),
                setting: r.i32()?,
            },
            ResponseKind::Refused => Response::Refused(RefusalCode::read(&mut r)?),
        };
        r.finish()?;
        response.validate()?;
        Ok(response)
    }
}
