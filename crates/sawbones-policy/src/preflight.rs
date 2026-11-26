//! Fact scope, invalidation and the write preflight (#110).
//!
//! A write-map entry or a mastership fact is not enough on its own to act
//! on. It was earned on one device, with one firmware, over one interface,
//! under some conditions, by some method, at some time. When any of those
//! stops being true, the fact is stale. This module holds:
//!
//! - [`FactScope`]: what a fact applies to: the exact device (model, serial,
//!   firmware), the interface (port, bench port, unit), the conditions it
//!   holds under, and the method that earned it, with the instrument, its
//!   firmware, the procedure and the uncertainty for a measured fact.
//! - [`FactLedger`]: the facts a write rests on, each with its scope and a
//!   version, and the current state of the bench as the host has observed
//!   it: which device is at each port and unit, which measuring modules are
//!   qualified, and which conditions hold.
//! - [`FactEvent`]: what makes facts stale. A firmware, serial or model
//!   change, a lost device, a rewired port, a foreign bus master, a reflashed
//!   instrument, a condition that ends, an open contradiction and a break in
//!   the clock each invalidate the facts they touch. An invalidation is
//!   sticky: the fact must be recorded again, even if the cause goes away.
//! - [`preflight`]: the check that runs before a request is classified. It
//!   works out the facts the request relies on itself ([`relied_facts`]),
//!   never from a list the agent gives, and refuses with a
//!   [`PreflightRefusal`] naming the first fact that is missing, stale,
//!   contradicted or out of scope. If every fact holds it classifies the
//!   request and returns a [`PreflightToken`] bound to the request, the
//!   action digest, the fact versions, the ledger generation and the store's
//!   chain head.
//! - [`confirm`]: the check at the moment of action. It consumes the token
//!   and refuses if anything changed since the preflight.
//!
//! Everything here is deterministic and host-side. Time is a [`Tick`] the
//! caller passes in; there is no clock.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use sawbones_interlock::Tick;
use sawbones_reading::Trust;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::decide::{classify, ActionDigest, Decision, RefusalReason, Requirement};
use crate::facts::actionable;
use crate::request::validate_policy_ident;
use crate::{
    BusFact, BusFacts, EffectRequest, PolicyIdentError, PortId, SignalId, WriteEntry, WriteMap,
};

/// How long a [`PreflightToken`] may be presented to [`confirm`] after the
/// preflight that made it, in ticks. The same as the interlock's lease
/// lifetime.
pub const PREFLIGHT_TTL_TICKS: Tick = sawbones_interlock::LEASE_TTL_TICKS;

/// Longest [`ScopeLabel`], in bytes.
pub const MAX_SCOPE_LABEL_BYTES: usize = 64;

/// A name in a fact's scope: a device model, a measuring module, a
/// procedure, a condition or a unit. It passes the same rule as a
/// [`PortId`]: 1 to 64 bytes, a lower-case ASCII letter first, then only
/// `a-z 0-9 _ -`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ScopeName(String);

impl ScopeName {
    /// Validates a string against the policy identifier rule.
    pub fn parse(s: &str) -> Result<Self, PolicyIdentError> {
        validate_policy_ident(s)?;
        Ok(Self(s.to_owned()))
    }

    /// The name as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ScopeName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl TryFrom<String> for ScopeName {
    type Error = PolicyIdentError;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        validate_policy_ident(&s)?;
        Ok(Self(s))
    }
}

impl From<ScopeName> for String {
    fn from(n: ScopeName) -> Self {
        n.0
    }
}

/// Why a string was rejected as a [`ScopeLabel`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ScopeLabelError {
    /// The string was empty.
    #[error("scope label is empty")]
    Empty,
    /// The string was longer than [`MAX_SCOPE_LABEL_BYTES`].
    #[error("scope label is longer than {MAX_SCOPE_LABEL_BYTES} bytes")]
    TooLong,
    /// The string did not start with an ASCII letter or digit.
    #[error("scope label must start with an ASCII letter or digit")]
    BadStart,
    /// The string held a character outside `A-Z a-z 0-9 . _ - +`.
    #[error("scope label contains a character outside A-Z a-z 0-9 . _ - +")]
    BadCharacter,
}

/// A serial number or a firmware revision as a device or module reports it,
/// for example `SN0042` or `V1.2.3+b7`. 1 to 64 bytes, an ASCII letter or
/// digit first, then only `A-Z a-z 0-9 . _ - +`. A label is only ever
/// compared for equality; it is never looked up as a name.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ScopeLabel(String);

fn validate_scope_label(s: &str) -> Result<(), ScopeLabelError> {
    let bytes = s.as_bytes();
    let Some(first) = bytes.first() else {
        return Err(ScopeLabelError::Empty);
    };
    if bytes.len() > MAX_SCOPE_LABEL_BYTES {
        return Err(ScopeLabelError::TooLong);
    }
    if !first.is_ascii_alphanumeric() {
        return Err(ScopeLabelError::BadStart);
    }
    if !bytes
        .iter()
        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-' | b'+'))
    {
        return Err(ScopeLabelError::BadCharacter);
    }
    Ok(())
}

impl ScopeLabel {
    /// Validates a string against the label rule.
    pub fn parse(s: &str) -> Result<Self, ScopeLabelError> {
        validate_scope_label(s)?;
        Ok(Self(s.to_owned()))
    }

    /// The label as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ScopeLabel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl TryFrom<String> for ScopeLabel {
    type Error = ScopeLabelError;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        validate_scope_label(&s)?;
        Ok(Self(s))
    }
}

impl From<ScopeLabel> for String {
    fn from(l: ScopeLabel) -> Self {
        l.0
    }
}

/// The exact device a fact is about: its model, its serial and its firmware
/// revision. A fact earned on one unit, or on one firmware, says nothing
/// about another.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceIdentity {
    /// The model, as the bench names it.
    pub model: ScopeName,
    /// The serial number.
    pub serial: ScopeLabel,
    /// The firmware revision.
    pub firmware: ScopeLabel,
}

/// The interface a fact was earned over.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Interface {
    /// The port.
    pub port: PortId,
    /// The bench port number the port was wired to.
    pub bench_port: u8,
    /// The Modbus unit, for a fact about one device. `None` for a fact about
    /// the bus as a whole, such as mastership.
    pub unit: Option<u8>,
}

/// The uncertainty of a measured fact: plus or minus `plus_minus` of `unit`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Uncertainty {
    /// The half-width of the interval.
    pub plus_minus: u64,
    /// The unit of `plus_minus`, for example `millivolt`.
    pub unit: ScopeName,
}

/// How a fact was earned. It must agree with the fact's trust tier: a
/// [`Trust::Measured`] fact is [`Method::Measured`] and a
/// [`Trust::OperatorAttested`] fact is [`Method::OperatorAttested`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Method {
    /// Measured by one of the bench's own instruments.
    Measured {
        /// The measuring module.
        instrument: ScopeName,
        /// Its firmware revision when it measured.
        instrument_firmware: ScopeLabel,
        /// The procedure followed.
        procedure: ScopeName,
        /// The uncertainty of the measurement. A measurement with no stated
        /// uncertainty cannot be written down.
        uncertainty: Uncertainty,
    },
    /// Attested by the owner.
    OperatorAttested,
}

/// How long a fact stays good with no event against it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Validity {
    /// Good for `ticks` after it was earned, then stale.
    For {
        /// Ticks after `earned_at`.
        ticks: Tick,
    },
    /// Good until an event invalidates it. Not allowed for mastership: a
    /// second master can start polling without any event the bench sees.
    UntilInvalidated,
}

/// What a fact applies to (#110). Preflight checks every part of it against
/// the request and the bench's current state, not only the trust tier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FactScope {
    /// The device the fact is about. Required for a write-map entry; `None`
    /// for a fact about the bus as a whole.
    pub device: Option<DeviceIdentity>,
    /// The interface it was earned over.
    pub interface: Interface,
    /// Conditions that held when it was earned and must still hold, for
    /// example `bank_disconnected`. Empty means none.
    pub conditions: BTreeSet<ScopeName>,
    /// How it was earned.
    pub method: Method,
}

/// Which fact a request relies on, by identity.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(tag = "fact", rename_all = "snake_case")]
pub enum FactKey {
    /// The write-map entry for a signal of a unit on a port.
    WriteEntry {
        /// The port.
        port: PortId,
        /// The unit.
        unit: u8,
        /// The signal.
        signal: SignalId,
    },
    /// The mastership fact for a port.
    Mastership {
        /// The port.
        port: PortId,
    },
}

impl FactKey {
    /// The port the fact is on.
    pub fn port(&self) -> &PortId {
        match self {
            FactKey::WriteEntry { port, .. } | FactKey::Mastership { port } => port,
        }
    }
}

impl fmt::Display for FactKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FactKey::WriteEntry { port, unit, signal } => {
                write!(f, "write entry {signal} of unit {unit} on {port}")
            }
            FactKey::Mastership { port } => write!(f, "mastership of {port}"),
        }
    }
}

/// The content of a fact: the exact write-map entry or mastership fact the
/// scope applies to. Preflight compares it with the write map and the bus
/// facts it is given, so a fact cannot vouch for an entry that has since
/// been edited.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FactContent {
    /// A write-map entry.
    WriteEntry(WriteEntry),
    /// A mastership fact.
    Mastership(BusFact),
}

impl FactContent {
    /// The fact's identity.
    pub fn key(&self) -> FactKey {
        match self {
            FactContent::WriteEntry(e) => FactKey::WriteEntry {
                port: e.port.clone(),
                unit: e.unit,
                signal: e.signal.clone(),
            },
            FactContent::Mastership(b) => FactKey::Mastership {
                port: b.port.clone(),
            },
        }
    }

    /// The fact's trust tier.
    pub const fn trust(&self) -> Trust {
        match self {
            FactContent::WriteEntry(e) => e.trust,
            FactContent::Mastership(b) => b.trust,
        }
    }
}

/// A fact with its scope, as the host records it in the ledger.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FactRecord {
    /// The fact.
    pub content: FactContent,
    /// What it applies to.
    pub scope: FactScope,
    /// When it was measured or attested.
    pub earned_at: Tick,
    /// How long it stays good.
    pub validity: Validity,
}

/// The version of a recorded fact: the ledger generation it was recorded
/// at. Recording the same fact again gives a new version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct FactVersion(pub u64);

/// The identity of one ledger, chosen by the host (for example a boot
/// nonce), so a token from one ledger is never accepted by another.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub struct LedgerId(pub u64);

/// The head of the store's hash chain when a preflight ran. A token is good
/// only against the same head.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChainHead(pub [u8; 32]);

/// What a measuring module was qualified against.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Reference {
    /// Another measuring module, itself qualified, and not the module being
    /// qualified.
    Module(ScopeName),
    /// An external reference standard, such as the voltage reference.
    Standard(ScopeName),
}

/// Something the host observed that may make facts stale, or that changes
/// what the bench knows about its devices, modules and conditions.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum FactEvent {
    /// The device at a port and unit identified itself. If its model, serial
    /// or firmware differs from what the ledger knew there, or the ledger
    /// knew no device there, every fact about that place is invalidated. If
    /// the same model and serial was known at another place, the device has
    /// moved: that place is forgotten and its facts invalidated too.
    DeviceIdentified {
        /// The port.
        port: PortId,
        /// The unit.
        unit: u8,
        /// What it reported.
        identity: DeviceIdentity,
    },
    /// The device at a port and unit stopped answering or was unplugged.
    DeviceLost {
        /// The port.
        port: PortId,
        /// The unit.
        unit: u8,
    },
    /// A port was rewired, unplugged or plugged in. Every fact on it is
    /// invalidated and every device on it forgotten.
    PortRewired {
        /// The port.
        port: PortId,
    },
    /// A request the bench did not send was heard on a port: another master
    /// is active. The port's mastership fact is invalidated.
    ForeignMasterObserved {
        /// The port.
        port: PortId,
    },
    /// A measuring module was reflashed. Every fact it produced is
    /// invalidated, it is no longer qualified, and nor is any module that
    /// was qualified against it, directly or through others, with all their
    /// facts.
    ModuleReflashed {
        /// The module.
        module: ScopeName,
        /// Its new firmware revision.
        firmware: ScopeLabel,
    },
    /// A measuring module passed a known-answer check against an independent
    /// reference. Facts it produced before are not restored; they must be
    /// measured again.
    ModuleQualified {
        /// The module.
        module: ScopeName,
        /// Its firmware revision during the check.
        firmware: ScopeLabel,
        /// What it was checked against.
        reference: Reference,
    },
    /// A condition started or stopped holding. When one stops, every fact
    /// earned under it is invalidated.
    ConditionChanged {
        /// The condition.
        condition: ScopeName,
        /// Whether it holds now.
        holds: bool,
    },
    /// Evidence disagrees with a fact. Preflight refuses it until the fact
    /// is recorded again.
    ContradictionOpened {
        /// The fact.
        fact: FactKey,
    },
    /// The tick source restarted or jumped. Nothing was watching for the
    /// time it cannot account for, so every fact is invalidated and every
    /// device identity, module qualification and condition forgotten.
    ClockDiscontinuity,
}

/// Why a fact was invalidated.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(tag = "cause", rename_all = "snake_case")]
pub enum InvalidationCause {
    /// The device at its place changed model, serial or firmware, or moved.
    DeviceChanged {
        /// The port.
        port: PortId,
        /// The unit.
        unit: u8,
    },
    /// The device at its place was lost.
    DeviceLost {
        /// The port.
        port: PortId,
        /// The unit.
        unit: u8,
    },
    /// Its port was rewired.
    PortRewired {
        /// The port.
        port: PortId,
    },
    /// Another master was heard on its port.
    ForeignMaster {
        /// The port.
        port: PortId,
    },
    /// The instrument that measured it, or one its qualification rested on,
    /// was reflashed.
    InstrumentReflashed {
        /// The module that was reflashed.
        module: ScopeName,
    },
    /// A condition it was earned under ended.
    ConditionEnded {
        /// The condition.
        condition: ScopeName,
    },
    /// The clock jumped or restarted.
    ClockDiscontinuity,
}

/// Which part of a fact's scope did not match.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ScopeField {
    /// The port.
    Port,
    /// The unit.
    Unit,
    /// The bench port number the port is wired to.
    BenchPort,
    /// A write-map entry fact that names no device.
    DeviceNotStated,
    /// A fact about a device that is about the bus, or the other way round.
    DeviceNotExpected,
    /// No device is known at the place.
    DeviceUnknown,
    /// The device's model.
    DeviceModel,
    /// The device's serial.
    DeviceSerial,
    /// The device's firmware.
    DeviceFirmware,
}

/// What is wrong with one fact.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Error)]
#[serde(tag = "problem", rename_all = "snake_case")]
pub enum FactProblem {
    /// The ledger has no record of it.
    #[error("not recorded")]
    Missing,
    /// The write map or bus facts given do not hold exactly the recorded
    /// content: it was edited, removed or never recorded as it stands.
    #[error("the write map or bus facts differ from the recorded fact")]
    ContentMismatch,
    /// It was invalidated and not recorded again since.
    #[error("invalidated: {by:?}")]
    Invalidated {
        /// Why.
        by: InvalidationCause,
    },
    /// Evidence disagrees with it.
    #[error("an open contradiction")]
    OpenContradiction,
    /// Its tier is below operator-attested.
    #[error("tier {trust:?} is not actionable")]
    NotActionable {
        /// The tier.
        trust: Trust,
    },
    /// Its method does not match its tier.
    #[error("method does not match tier {trust:?}")]
    MethodDoesNotMatchTier {
        /// The tier.
        trust: Trust,
    },
    /// The instrument that measured it is not qualified now.
    #[error("instrument {module} is not qualified")]
    InstrumentNotQualified {
        /// The module.
        module: ScopeName,
    },
    /// The instrument's firmware is not the one it measured with.
    #[error("instrument {module} firmware differs")]
    InstrumentFirmwareMismatch {
        /// The module.
        module: ScopeName,
    },
    /// The tick given is earlier than the latest tick the ledger has seen.
    #[error("the clock is behind the ledger")]
    ClockBehindLedger,
    /// It was earned after the tick given.
    #[error("earned in the future")]
    EarnedInFuture,
    /// It was earned before the last clock discontinuity.
    #[error("earned before the last clock discontinuity")]
    EarnedBeforeClockDiscontinuity,
    /// Its validity ran out.
    #[error("expired: earned at {earned_at}, good for {valid_for} ticks")]
    Expired {
        /// When it was earned.
        earned_at: Tick,
        /// For how long it was good.
        valid_for: Tick,
    },
    /// Part of its scope does not match the request or the bench.
    #[error("out of scope: {field:?}")]
    OutOfScope {
        /// Which part.
        field: ScopeField,
    },
    /// A condition it was earned under does not hold now.
    #[error("condition {condition} does not hold")]
    ConditionNotHolding {
        /// The condition.
        condition: ScopeName,
    },
    /// A mastership fact must expire ([`Validity::For`]).
    #[error("a mastership fact must expire")]
    MastershipNeedsExpiry,
}

/// What is wrong with a token presented to [`confirm`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Error)]
#[serde(rename_all = "snake_case")]
pub enum TokenProblem {
    /// It was made by another ledger.
    #[error("made by another ledger")]
    OtherLedger,
    /// It was made for another request.
    #[error("made for another request")]
    OtherRequest,
    /// The tick given is earlier than the preflight.
    #[error("the clock is behind the token")]
    ClockBehindToken,
    /// More than [`PREFLIGHT_TTL_TICKS`] passed since the preflight.
    #[error("expired")]
    Expired,
    /// The store's chain head moved.
    #[error("the chain head moved")]
    ChainHeadMoved,
    /// The ledger changed: a fact was recorded or invalidated, or a device,
    /// module or condition changed.
    #[error("the ledger changed")]
    LedgerChanged,
    /// The same check now gives another decision or other fact versions.
    #[error("the decision changed")]
    DecisionChanged,
}

/// Why a preflight or a confirm refused. Every refusal from a fact names the
/// fact.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Error)]
#[serde(tag = "refusal", rename_all = "snake_case")]
pub enum PreflightRefusal {
    /// A fact the request relies on does not hold.
    #[error("{fact}: {problem}")]
    Fact {
        /// The fact.
        fact: FactKey,
        /// What is wrong with it.
        problem: FactProblem,
    },
    /// Every fact holds, and the engine refused the request.
    #[error("refused by the engine: {reason:?}")]
    Classified {
        /// Why.
        reason: RefusalReason,
    },
    /// The token presented to [`confirm`] is no good.
    #[error("token: {problem}")]
    Token {
        /// What is wrong with it.
        problem: TokenProblem,
    },
}

impl PreflightRefusal {
    /// The fact the refusal names, if it is about one.
    pub fn fact(&self) -> Option<&FactKey> {
        match self {
            PreflightRefusal::Fact { fact, .. } => Some(fact),
            _ => None,
        }
    }
}

/// Why [`FactLedger::record`] refused a record.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Error)]
#[serde(tag = "error", rename_all = "snake_case")]
pub enum RecordError {
    /// The tick given is earlier than the latest tick the ledger has seen.
    /// The ledger treats it as a clock discontinuity.
    #[error("the clock went backwards")]
    ClockWentBackwards,
    /// The record's scope does not match its own content.
    #[error("scope does not match the fact: {field:?}")]
    ScopeDoesNotMatchContent {
        /// Which part.
        field: ScopeField,
    },
    /// The record would not pass preflight now.
    #[error("{problem}")]
    Fact {
        /// What is wrong.
        problem: FactProblem,
    },
}

/// Why [`FactLedger::apply`] refused an event. Only events that raise trust
/// can be refused; an event that invalidates is always applied.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Error)]
#[serde(tag = "error", rename_all = "snake_case")]
pub enum EventRefusal {
    /// A module cannot be its own reference.
    #[error("a module cannot qualify against itself")]
    SelfReference,
    /// The reference module is not qualified.
    #[error("reference module {reference} is not qualified")]
    ReferenceNotQualified {
        /// The reference.
        reference: ScopeName,
    },
    /// The firmware named is not the module's current firmware.
    #[error("module {module} firmware differs from its last reflash")]
    FirmwareMismatch {
        /// The module.
        module: ScopeName,
    },
}

/// What an applied event did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Applied {
    /// The facts it invalidated, in key order. Facts already invalidated are
    /// not listed again.
    pub invalidated: Vec<FactKey>,
}

#[derive(Debug)]
struct Stored {
    record: FactRecord,
    version: FactVersion,
    invalidated: Option<InvalidationCause>,
}

#[derive(Debug)]
struct ModuleState {
    firmware: ScopeLabel,
    qualified: Option<Reference>,
}

/// The facts a write rests on, with their scope and versions, and the
/// bench's current state as the host observed it.
///
/// It has no clock: every call takes the current [`Tick`]. A tick earlier
/// than the latest one seen is a clock discontinuity. It cannot be cloned or
/// deserialised, so its state comes only from [`FactLedger::record`] and
/// [`FactLedger::apply`].
#[derive(Debug)]
pub struct FactLedger {
    id: LedgerId,
    generation: u64,
    latest: Tick,
    epoch_start: Tick,
    facts: BTreeMap<FactKey, Stored>,
    contradictions: BTreeSet<FactKey>,
    devices: BTreeMap<(PortId, u8), DeviceIdentity>,
    modules: BTreeMap<ScopeName, ModuleState>,
    conditions: BTreeSet<ScopeName>,
}

impl FactLedger {
    /// An empty ledger: no facts, no devices, no qualified modules, no
    /// conditions.
    pub fn new(id: LedgerId) -> Self {
        Self {
            id,
            generation: 0,
            latest: 0,
            epoch_start: 0,
            facts: BTreeMap::new(),
            contradictions: BTreeSet::new(),
            devices: BTreeMap::new(),
            modules: BTreeMap::new(),
            conditions: BTreeSet::new(),
        }
    }

    /// The ledger's identity.
    pub const fn id(&self) -> LedgerId {
        self.id
    }

    /// A counter that goes up on every change of state.
    pub const fn generation(&self) -> u64 {
        self.generation
    }

    /// The latest tick the ledger has seen.
    pub const fn latest_tick(&self) -> Tick {
        self.latest
    }

    /// The version of a recorded fact, if there is one, stale or not.
    pub fn version(&self, key: &FactKey) -> Option<FactVersion> {
        self.facts.get(key).map(|s| s.version)
    }

    /// Why a recorded fact was invalidated, if it was.
    pub fn invalidated_by(&self, key: &FactKey) -> Option<&InvalidationCause> {
        self.facts.get(key).and_then(|s| s.invalidated.as_ref())
    }

    /// Whether a contradiction is open against a fact.
    pub fn is_contradicted(&self, key: &FactKey) -> bool {
        self.contradictions.contains(key)
    }

    /// The device known at a port and unit.
    pub fn device_at(&self, port: &PortId, unit: u8) -> Option<&DeviceIdentity> {
        self.devices.get(&(port.clone(), unit))
    }

    /// Whether a measuring module is qualified now.
    pub fn is_qualified(&self, module: &ScopeName) -> bool {
        self.modules
            .get(module)
            .is_some_and(|m| m.qualified.is_some())
    }

    /// Whether a condition holds now.
    pub fn condition_holds(&self, condition: &ScopeName) -> bool {
        self.conditions.contains(condition)
    }

    fn bump(&mut self) -> u64 {
        self.generation = self.generation.saturating_add(1);
        self.generation
    }

    /// Takes a new tick. A tick behind the latest is a clock discontinuity.
    fn advance(&mut self, now: Tick, out: &mut Vec<FactKey>) -> bool {
        if now < self.latest {
            self.discontinuity(now, out);
            false
        } else {
            self.latest = now;
            true
        }
    }

    fn discontinuity(&mut self, now: Tick, out: &mut Vec<FactKey>) {
        self.invalidate_where(|_, _| true, &InvalidationCause::ClockDiscontinuity, out);
        self.devices.clear();
        self.conditions.clear();
        for m in self.modules.values_mut() {
            m.qualified = None;
        }
        self.latest = now;
        self.epoch_start = now;
        self.bump();
    }

    /// Invalidates every live fact for which `pred` holds. Bumps the
    /// generation once if any was.
    fn invalidate_where(
        &mut self,
        pred: impl Fn(&FactKey, &FactRecord) -> bool,
        cause: &InvalidationCause,
        out: &mut Vec<FactKey>,
    ) {
        let mut any = false;
        for (key, stored) in &mut self.facts {
            if stored.invalidated.is_none() && pred(key, &stored.record) {
                stored.invalidated = Some(cause.clone());
                out.push(key.clone());
                any = true;
            }
        }
        if any {
            self.bump();
        }
    }

    /// Invalidates every fact about the device at a place.
    fn invalidate_place(
        &mut self,
        port: &PortId,
        unit: u8,
        cause: &InvalidationCause,
        out: &mut Vec<FactKey>,
    ) {
        self.invalidate_where(
            |key, record| {
                let keyed = matches!(key, FactKey::WriteEntry { port: p, unit: u, .. } if p == port && *u == unit);
                let scoped = &record.scope.interface.port == port
                    && record.scope.interface.unit == Some(unit);
                keyed || scoped
            },
            cause,
            out,
        );
    }

    /// Records a fact, or a new version of it. Recording clears an
    /// invalidation and an open contradiction on it. The record must pass
    /// every check that does not depend on a request: its scope matches its
    /// content, its tier and method agree, its instrument is qualified with
    /// the same firmware, its device is the one known at its place, its
    /// conditions hold and it was earned in the current clock epoch, at or
    /// before `now`, and is not already expired.
    pub fn record(&mut self, record: FactRecord, now: Tick) -> Result<FactVersion, RecordError> {
        let mut ignored = Vec::new();
        if !self.advance(now, &mut ignored) {
            return Err(RecordError::ClockWentBackwards);
        }
        if let Some(field) = scope_content_mismatch(&record) {
            return Err(RecordError::ScopeDoesNotMatchContent { field });
        }
        let fact = |problem| RecordError::Fact { problem };
        if let FactContent::Mastership(_) = record.content {
            if record.validity == Validity::UntilInvalidated {
                return Err(fact(FactProblem::MastershipNeedsExpiry));
            }
        }
        self.standing_problem(&record, now)
            .map_or(Ok(()), |p| Err(fact(p)))?;
        if let Some(device) = &record.scope.device {
            let unit = record.scope.interface.unit.unwrap_or(0);
            if let Some(field) =
                device_mismatch(device, self.device_at(&record.scope.interface.port, unit))
            {
                return Err(fact(FactProblem::OutOfScope { field }));
            }
        }
        let key = record.content.key();
        let version = FactVersion(self.bump());
        self.contradictions.remove(&key);
        self.facts.insert(
            key,
            Stored {
                record,
                version,
                invalidated: None,
            },
        );
        Ok(version)
    }

    /// The checks on a record that do not depend on a request: tier and
    /// method, instrument, time and conditions.
    fn standing_problem(&self, record: &FactRecord, now: Tick) -> Option<FactProblem> {
        let trust = record.content.trust();
        if !actionable(trust) {
            return Some(FactProblem::NotActionable { trust });
        }
        let agrees = matches!(
            (trust, &record.scope.method),
            (Trust::Measured, Method::Measured { .. })
                | (Trust::OperatorAttested, Method::OperatorAttested)
        );
        if !agrees {
            return Some(FactProblem::MethodDoesNotMatchTier { trust });
        }
        if let Method::Measured {
            instrument,
            instrument_firmware,
            ..
        } = &record.scope.method
        {
            match self.modules.get(instrument) {
                Some(m) if m.qualified.is_some() => {
                    if &m.firmware != instrument_firmware {
                        return Some(FactProblem::InstrumentFirmwareMismatch {
                            module: instrument.clone(),
                        });
                    }
                }
                _ => {
                    return Some(FactProblem::InstrumentNotQualified {
                        module: instrument.clone(),
                    })
                }
            }
        }
        if now < self.latest {
            return Some(FactProblem::ClockBehindLedger);
        }
        if record.earned_at > now {
            return Some(FactProblem::EarnedInFuture);
        }
        if record.earned_at < self.epoch_start {
            return Some(FactProblem::EarnedBeforeClockDiscontinuity);
        }
        if let Validity::For { ticks } = record.validity {
            if now >= record.earned_at.saturating_add(ticks) {
                return Some(FactProblem::Expired {
                    earned_at: record.earned_at,
                    valid_for: ticks,
                });
            }
        }
        if let Some(c) = record
            .scope
            .conditions
            .iter()
            .find(|c| !self.conditions.contains(*c))
        {
            return Some(FactProblem::ConditionNotHolding {
                condition: c.clone(),
            });
        }
        None
    }

    /// Applies an event at tick `now`. An event that invalidates is always
    /// applied. Only [`FactEvent::ModuleQualified`] can be refused, and then
    /// nothing changes but the tick.
    pub fn apply(&mut self, event: FactEvent, now: Tick) -> Result<Applied, EventRefusal> {
        let mut out = Vec::new();
        self.advance(now, &mut out);
        match event {
            FactEvent::DeviceIdentified {
                port,
                unit,
                identity,
            } => {
                if self.device_at(&port, unit) != Some(&identity) {
                    let moved: Vec<(PortId, u8)> = self
                        .devices
                        .iter()
                        .filter(|((p, u), d)| {
                            (p != &port || *u != unit)
                                && d.model == identity.model
                                && d.serial == identity.serial
                        })
                        .map(|(place, _)| place.clone())
                        .collect();
                    for (p, u) in moved {
                        self.devices.remove(&(p.clone(), u));
                        let cause = InvalidationCause::DeviceChanged {
                            port: p.clone(),
                            unit: u,
                        };
                        self.invalidate_place(&p, u, &cause, &mut out);
                    }
                    let cause = InvalidationCause::DeviceChanged {
                        port: port.clone(),
                        unit,
                    };
                    self.invalidate_place(&port, unit, &cause, &mut out);
                    self.devices.insert((port, unit), identity);
                    self.bump();
                }
            }
            FactEvent::DeviceLost { port, unit } => {
                self.devices.remove(&(port.clone(), unit));
                let cause = InvalidationCause::DeviceLost {
                    port: port.clone(),
                    unit,
                };
                self.invalidate_place(&port, unit, &cause, &mut out);
                self.bump();
            }
            FactEvent::PortRewired { port } => {
                self.devices.retain(|(p, _), _| p != &port);
                let cause = InvalidationCause::PortRewired { port: port.clone() };
                self.invalidate_where(
                    |key, record| key.port() == &port || record.scope.interface.port == port,
                    &cause,
                    &mut out,
                );
                self.bump();
            }
            FactEvent::ForeignMasterObserved { port } => {
                let cause = InvalidationCause::ForeignMaster { port: port.clone() };
                self.invalidate_where(
                    |key, _| matches!(key, FactKey::Mastership { port: p } if p == &port),
                    &cause,
                    &mut out,
                );
                self.bump();
            }
            FactEvent::ModuleReflashed { module, firmware } => {
                // Every module whose qualification rests on this one,
                // directly or through others, loses it too.
                let mut fallen: BTreeSet<ScopeName> = BTreeSet::from([module.clone()]);
                loop {
                    let more: Vec<ScopeName> = self
                        .modules
                        .iter()
                        .filter(|(name, m)| {
                            !fallen.contains(*name)
                                && matches!(&m.qualified, Some(Reference::Module(r)) if fallen.contains(r))
                        })
                        .map(|(name, _)| name.clone())
                        .collect();
                    if more.is_empty() {
                        break;
                    }
                    fallen.extend(more);
                }
                for name in &fallen {
                    if let Some(m) = self.modules.get_mut(name) {
                        m.qualified = None;
                    }
                }
                self.modules.insert(
                    module.clone(),
                    ModuleState {
                        firmware,
                        qualified: None,
                    },
                );
                let cause = InvalidationCause::InstrumentReflashed { module };
                self.invalidate_where(
                    |_, record| {
                        matches!(&record.scope.method, Method::Measured { instrument, .. } if fallen.contains(instrument))
                    },
                    &cause,
                    &mut out,
                );
                self.bump();
            }
            FactEvent::ModuleQualified {
                module,
                firmware,
                reference,
            } => {
                if let Reference::Module(r) = &reference {
                    if r == &module {
                        return Err(EventRefusal::SelfReference);
                    }
                    if !self.is_qualified(r) {
                        return Err(EventRefusal::ReferenceNotQualified {
                            reference: r.clone(),
                        });
                    }
                }
                if let Some(m) = self.modules.get(&module) {
                    if m.firmware != firmware {
                        return Err(EventRefusal::FirmwareMismatch { module });
                    }
                }
                self.modules.insert(
                    module,
                    ModuleState {
                        firmware,
                        qualified: Some(reference),
                    },
                );
                self.bump();
            }
            FactEvent::ConditionChanged { condition, holds } => {
                if holds {
                    if self.conditions.insert(condition) {
                        self.bump();
                    }
                } else {
                    let was = self.conditions.remove(&condition);
                    let cause = InvalidationCause::ConditionEnded {
                        condition: condition.clone(),
                    };
                    self.invalidate_where(
                        |_, record| record.scope.conditions.contains(&condition),
                        &cause,
                        &mut out,
                    );
                    if was {
                        self.bump();
                    }
                }
            }
            FactEvent::ContradictionOpened { fact } => {
                if self.contradictions.insert(fact) {
                    self.bump();
                }
            }
            FactEvent::ClockDiscontinuity => self.discontinuity(now, &mut out),
        }
        out.sort();
        out.dedup();
        Ok(Applied { invalidated: out })
    }
}

/// The part of a record's scope that disagrees with its own content.
fn scope_content_mismatch(record: &FactRecord) -> Option<ScopeField> {
    let scope = &record.scope;
    match &record.content {
        FactContent::WriteEntry(e) => {
            if scope.interface.port != e.port {
                Some(ScopeField::Port)
            } else if scope.interface.unit != Some(e.unit) {
                Some(ScopeField::Unit)
            } else if scope.device.is_none() {
                Some(ScopeField::DeviceNotStated)
            } else {
                None
            }
        }
        FactContent::Mastership(b) => {
            if scope.interface.port != b.port {
                Some(ScopeField::Port)
            } else if scope.interface.unit.is_some() {
                Some(ScopeField::Unit)
            } else if scope.interface.bench_port != b.bench_port {
                Some(ScopeField::BenchPort)
            } else if scope.device.is_some() {
                Some(ScopeField::DeviceNotExpected)
            } else {
                None
            }
        }
    }
}

/// How the device in a scope differs from the device known at its place.
fn device_mismatch(scope: &DeviceIdentity, known: Option<&DeviceIdentity>) -> Option<ScopeField> {
    let Some(known) = known else {
        return Some(ScopeField::DeviceUnknown);
    };
    if known.model != scope.model {
        Some(ScopeField::DeviceModel)
    } else if known.serial != scope.serial {
        Some(ScopeField::DeviceSerial)
    } else if known.firmware != scope.firmware {
        Some(ScopeField::DeviceFirmware)
    } else {
        None
    }
}

/// The facts a request relies on, worked out from the request alone: the
/// manifest of preconditions. The agent never supplies this list.
///
/// - a bus write relies on its write-map entry and its port's mastership;
/// - a bus read transmits, so it relies on its port's mastership;
/// - passive listening relies on nothing.
pub fn relied_facts(request: &EffectRequest) -> Vec<FactKey> {
    match request {
        EffectRequest::BusWrite(w) => vec![
            FactKey::WriteEntry {
                port: w.port.clone(),
                unit: w.unit,
                signal: w.signal.clone(),
            },
            FactKey::Mastership {
                port: w.port.clone(),
            },
        ],
        EffectRequest::BusRead(r) => vec![FactKey::Mastership {
            port: r.port.clone(),
        }],
        EffectRequest::BusListen(_) => Vec::new(),
    }
}

/// The unit a request addresses, if any.
const fn request_unit(request: &EffectRequest) -> Option<u8> {
    match request {
        EffectRequest::BusWrite(w) => Some(w.unit),
        EffectRequest::BusRead(r) => Some(r.unit),
        EffectRequest::BusListen(_) => None,
    }
}

/// Checks one relied-on fact. The order is fixed, so a refusal always names
/// the same first problem.
fn check_fact(
    key: &FactKey,
    request: &EffectRequest,
    writes: &WriteMap,
    buses: &BusFacts,
    ledger: &FactLedger,
    now: Tick,
) -> Result<FactVersion, FactProblem> {
    let Some(stored) = ledger.facts.get(key) else {
        return Err(FactProblem::Missing);
    };
    let record = &stored.record;
    let current = match key {
        FactKey::WriteEntry { port, unit, signal } => writes
            .get(port, *unit, signal)
            .map(|e| FactContent::WriteEntry(e.clone())),
        FactKey::Mastership { port } => buses.get(port).map(|b| FactContent::Mastership(b.clone())),
    };
    if current.as_ref() != Some(&record.content) || &record.content.key() != key {
        return Err(FactProblem::ContentMismatch);
    }
    if let Some(by) = &stored.invalidated {
        return Err(FactProblem::Invalidated { by: by.clone() });
    }
    if ledger.contradictions.contains(key) {
        return Err(FactProblem::OpenContradiction);
    }
    if let FactContent::Mastership(_) = record.content {
        if record.validity == Validity::UntilInvalidated {
            return Err(FactProblem::MastershipNeedsExpiry);
        }
    }
    if let Some(problem) = ledger.standing_problem(record, now) {
        return Err(problem);
    }
    let out = |field| Err(FactProblem::OutOfScope { field });
    let scope = &record.scope;
    if &scope.interface.port != request.port() {
        return out(ScopeField::Port);
    }
    let unit = request_unit(request);
    match (key, scope.interface.unit) {
        (FactKey::WriteEntry { .. }, None) => return out(ScopeField::Unit),
        (_, Some(u)) if Some(u) != unit => return out(ScopeField::Unit),
        _ => {}
    }
    if buses.get(request.port()).map(|b| b.bench_port) != Some(scope.interface.bench_port) {
        return out(ScopeField::BenchPort);
    }
    match (&scope.device, key) {
        (None, FactKey::WriteEntry { .. }) => return out(ScopeField::DeviceNotStated),
        (Some(_), FactKey::Mastership { .. }) => return out(ScopeField::DeviceNotExpected),
        (Some(device), _) => {
            let known = unit.and_then(|u| ledger.device_at(request.port(), u));
            if let Some(field) = device_mismatch(device, known) {
                return out(field);
            }
        }
        (None, _) => {}
    }
    Ok(stored.version)
}

/// The proof that a preflight passed, for one request against one state of
/// the facts. It is not `Clone`, cannot be serialised or deserialised, and
/// only [`preflight`] makes one; [`confirm`] consumes it.
#[derive(Debug)]
#[must_use = "a preflight token is presented to confirm before acting"]
pub struct PreflightToken {
    ledger: LedgerId,
    generation: u64,
    chain_head: ChainHead,
    request: Vec<u8>,
    decision: Decision,
    facts: Vec<(FactKey, FactVersion)>,
    checked_at: Tick,
}

impl PreflightToken {
    /// The decision the request was classified to.
    pub const fn decision(&self) -> &Decision {
        &self.decision
    }

    /// The digest of the bus action a class 3 decision approves.
    pub fn action_digest(&self) -> Option<ActionDigest> {
        match self.decision.requirement() {
            Requirement::RailBLease { action_digest, .. } => Some(*action_digest),
            _ => None,
        }
    }

    /// The facts relied on, with the versions checked.
    pub fn facts(&self) -> &[(FactKey, FactVersion)] {
        &self.facts
    }

    /// The ledger generation checked against.
    pub const fn generation(&self) -> u64 {
        self.generation
    }

    /// The chain head checked against.
    pub const fn chain_head(&self) -> ChainHead {
        self.chain_head
    }

    /// The tick of the preflight.
    pub const fn checked_at(&self) -> Tick {
        self.checked_at
    }

    /// The first tick at which [`confirm`] refuses the token as expired.
    pub const fn expires_at(&self) -> Tick {
        self.checked_at.saturating_add(PREFLIGHT_TTL_TICKS)
    }
}

/// Checks the facts a request relies on, then classifies it (#110).
///
/// The facts come from [`relied_facts`], never from the caller. Each one, in
/// that order, must be recorded in the ledger with exactly the content the
/// write map or bus facts hold now; not invalidated; not contradicted; at an
/// actionable tier with a method that agrees; measured, if it was, by an
/// instrument qualified now with the same firmware; earned in the current
/// clock epoch and not expired at `now`, with `now` not behind the ledger;
/// in scope for the request's port, unit and bench port and for the device
/// known there now; and earned under conditions that all still hold. The
/// first fact that fails is named in the refusal.
///
/// If every fact holds, the request is classified with [`classify`]. A
/// refusal from the engine is returned as [`PreflightRefusal::Classified`].
/// Otherwise the answer is a token bound to the request, the decision and
/// its action digest, the fact versions, the ledger and its generation, and
/// `chain_head`.
pub fn preflight(
    request: &EffectRequest,
    writes: &WriteMap,
    buses: &BusFacts,
    ledger: &FactLedger,
    chain_head: ChainHead,
    now: Tick,
) -> Result<PreflightToken, PreflightRefusal> {
    let mut facts = Vec::new();
    for key in relied_facts(request) {
        match check_fact(&key, request, writes, buses, ledger, now) {
            Ok(version) => facts.push((key, version)),
            Err(problem) => return Err(PreflightRefusal::Fact { fact: key, problem }),
        }
    }
    let decision = classify(request, writes, buses);
    if let Requirement::Refused { reason } = decision.requirement() {
        return Err(PreflightRefusal::Classified { reason: *reason });
    }
    Ok(PreflightToken {
        ledger: ledger.id(),
        generation: ledger.generation(),
        chain_head,
        request: request.canonical_bytes(),
        decision,
        facts,
        checked_at: now,
    })
}

/// A request that passed [`confirm`]: what it may do now.
#[derive(Debug)]
pub struct Confirmed {
    decision: Decision,
}

impl Confirmed {
    /// The decision.
    pub const fn decision(&self) -> &Decision {
        &self.decision
    }

    /// The interlock lease a class 3 decision asks for.
    pub fn lease_target(&self) -> Option<sawbones_interlock::LeaseTarget> {
        self.decision.requirement().lease_target()
    }
}

/// The check at the moment of action. It consumes the token and refuses
/// unless the token was made by this ledger for this exact request, `now` is
/// at or after the preflight and before [`PreflightToken::expires_at`], the
/// chain head is the same, the ledger has not changed at all, and the same
/// preflight run again gives the same decision on the same fact versions.
pub fn confirm(
    token: PreflightToken,
    request: &EffectRequest,
    writes: &WriteMap,
    buses: &BusFacts,
    ledger: &FactLedger,
    chain_head: ChainHead,
    now: Tick,
) -> Result<Confirmed, PreflightRefusal> {
    let refuse = |problem| Err(PreflightRefusal::Token { problem });
    if token.ledger != ledger.id() {
        return refuse(TokenProblem::OtherLedger);
    }
    if token.request != request.canonical_bytes() {
        return refuse(TokenProblem::OtherRequest);
    }
    if now < token.checked_at {
        return refuse(TokenProblem::ClockBehindToken);
    }
    if now >= token.expires_at() {
        return refuse(TokenProblem::Expired);
    }
    if chain_head != token.chain_head {
        return refuse(TokenProblem::ChainHeadMoved);
    }
    if ledger.generation() != token.generation {
        return refuse(TokenProblem::LedgerChanged);
    }
    let again = preflight(request, writes, buses, ledger, chain_head, now)?;
    if again.decision != token.decision || again.facts != token.facts {
        return refuse(TokenProblem::DecisionChanged);
    }
    Ok(Confirmed {
        decision: token.decision,
    })
}
