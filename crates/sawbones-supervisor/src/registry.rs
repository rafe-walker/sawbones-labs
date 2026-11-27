//! The module registry: who is enrolled where, and what each hub port holds.

use sawbones_module_protocol::{
    Capabilities, HostError, IdentityError, ManifestDigest, ModuleId, ModuleKey, RefusalCode,
};
use sawbones_policy::{PortId, ScopeName};

/// A port on the brain's USB hub, numbered from 0. A module is bound to the
/// hub port it was enrolled on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct HubPort(pub u8);

/// What the owner enrolled for one module: the host's allow-list entry. A
/// module appearing on USB grants nothing; this decides.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Enrollment {
    /// The module's id.
    pub module: ModuleId,
    /// The hub port it was enrolled on. Seen on any other, it is
    /// quarantined.
    pub hub_port: HubPort,
    /// Its public identity key.
    pub key: ModuleKey,
    /// Its manifest digest, pinned at install.
    pub manifest: ManifestDigest,
    /// The most it may claim: a signed identity with `can_emit` where this
    /// has none, or a higher class, is quarantined.
    pub capabilities: Capabilities,
    /// Its name in the fact ledger, if it measures. Its facts are
    /// invalidated when it is lost, quarantined or reflashed.
    pub instrument: Option<ScopeName>,
    /// The bus ports it carries, for a comms pod. Every fact on them is
    /// invalidated when it is lost or quarantined, and no action on them is
    /// carried out unless it is available.
    pub carries: Vec<PortId>,
}

/// Why a module id is quarantined. A quarantine is sticky: it ends only
/// when a person re-enrolls the module ([`Supervisor::re_enroll`]).
///
/// [`Supervisor::re_enroll`]: crate::Supervisor::re_enroll
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QuarantineReason {
    /// Two hub ports proved the same id at once: someone holds a copy of
    /// its key, and neither can be told from the other.
    DuplicateId {
        /// The port it was already verified on.
        first: HubPort,
        /// The port that proved it again.
        second: HubPort,
    },
    /// It proved its identity on a hub port other than the enrolled one.
    WrongHubPort {
        /// The enrolled port.
        enrolled: HubPort,
        /// The port it was seen on.
        seen: HubPort,
    },
    /// Its signed capabilities exceed what was enrolled: it can emit where
    /// it was enrolled as unable to, or claims a higher class.
    CapabilityExceeded {
        /// What was enrolled.
        enrolled: Capabilities,
        /// What it signed.
        claimed: Capabilities,
    },
    /// Its signed manifest digest is not the one pinned at install.
    ManifestMismatch,
    /// It was lost [`MAX_LOSSES`](crate::MAX_LOSSES) times within
    /// [`LOSS_WINDOW_TICKS`](crate::LOSS_WINDOW_TICKS): something keeps
    /// ending its sessions (an `Identify` replay, card hw-031) or its link
    /// keeps failing. A person should look before it is trusted again.
    RepeatedLoss {
        /// How many losses in the window.
        losses: u32,
    },
    /// Its port sent more than [`FLOOD_BUDGET`](crate::FLOOD_BUDGET) frames
    /// the host could not take within one poll period (card hw-033).
    Flooding {
        /// How many in the window.
        bad_frames: u32,
    },
}

/// Why the supervisor stopped talking to the device on a hub port, short of
/// a quarantine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PortRefusal {
    /// It claims an id nobody enrolled. It grants nothing.
    Unenrolled(ModuleId),
    /// Its identity did not verify against the enrolled key.
    BadIdentity(IdentityError),
    /// It refused to identify.
    IdentifyRefused(RefusalCode),
    /// It claims an id that is quarantined.
    Quarantined(ModuleId),
    /// Before it proved an identity, its port sent more than
    /// [`FLOOD_BUDGET`](crate::FLOOD_BUDGET) frames the host could not take
    /// within one poll period.
    Flooding,
}

/// What one hub port holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PortState {
    /// A device is there and an identify is outstanding. Untrusted.
    Opening,
    /// A module whose identity verified against its enrollment, on its
    /// enrolled port.
    Verified(ModuleId),
    /// The device missed a deadline. The supervisor identifies it again
    /// after [`POLL_PERIOD_TICKS`](crate::POLL_PERIOD_TICKS).
    Lost,
    /// The supervisor no longer talks to it until it is unplugged (or, for a
    /// quarantine, re-enrolled).
    Refused(PortRefusal),
}

/// Where an enrolled module stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModuleState {
    /// Not seen on its enrolled port.
    Absent,
    /// A device on its enrolled port has not proved its identity yet.
    Discovered,
    /// Verified on its enrolled port and answering. Its claims are still
    /// claims: trust in its readings returns only after the known-answer
    /// check (#178).
    Available,
    /// It missed a deadline or was unplugged. Its facts were invalidated.
    Lost,
    /// The device on its enrolled port was refused.
    Refused(PortRefusal),
    /// Quarantined until a person re-enrolls it.
    Quarantined(QuarantineReason),
}

/// Why a [`Config`](crate::Config) was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigError {
    /// Two enrollments for one module id.
    DuplicateModule(ModuleId),
    /// Two modules enrolled on one hub port.
    DuplicateHubPort(HubPort),
    /// Two modules carry one bus port.
    SharedBusPort(PortId),
    /// The bench config lists a module that is not enrolled.
    BenchModuleNotEnrolled(ModuleId),
    /// An enrolled module is not in the bench config: its terminals would
    /// be missing from every plan.
    ModuleNotInBench(ModuleId),
    /// A declaration for a module that is not enrolled.
    UnknownModule(ModuleId),
}

/// Something that happened on one [`Supervisor::step`], for the log.
///
/// [`Supervisor::step`]: crate::Supervisor::step
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Happened {
    /// A heartbeat challenge was answered.
    HeartbeatAnswered,
    /// A device appeared on a hub port.
    Discovered(HubPort),
    /// The device on a hub port went away.
    Unplugged(HubPort),
    /// A module verified on its enrolled port.
    Available {
        /// The module.
        module: ModuleId,
        /// Its port.
        port: HubPort,
    },
    /// The device on a hub port was refused.
    Refused {
        /// The port.
        port: HubPort,
        /// Why.
        why: PortRefusal,
    },
    /// A module id was quarantined.
    Quarantined {
        /// The module.
        module: ModuleId,
        /// Why.
        reason: QuarantineReason,
    },
    /// A verified module missed a deadline or was unplugged.
    Lost {
        /// The module.
        module: ModuleId,
        /// Its port.
        port: HubPort,
    },
    /// A device's identify missed its deadline.
    IdentifyTimedOut(HubPort),
    /// Facts were invalidated through the ledger, in key order per event.
    FactsInvalidated(Vec<sawbones_policy::FactKey>),
    /// A frame from a hub port was not taken.
    FrameDropped {
        /// The port.
        port: HubPort,
        /// Why.
        error: HostError,
    },
    /// The armed connection plan was dropped: the trust root reported a
    /// fault, or rail A going off. Arming again re-runs the isolation check.
    PlanDropped,
}
