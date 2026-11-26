//! The engine: one pure function from a request and the facts to a decision.

use std::fmt;

use sawbones_interlock::HazardClass;
use sawbones_modbus_encode::{BusWriteAction, WriteFunction};
use sawbones_reading::Trust;
use serde::{Serialize, Serializer};

use crate::facts::actionable;
use crate::{BusFacts, EffectRequest, Mastership, PortId, WriteMap, WriteValue};

/// The rank of a class, 0 to 4, for comparing classes.
pub const fn class_rank(class: HazardClass) -> u8 {
    match class {
        HazardClass::Class0 => 0,
        HazardClass::Class1 => 1,
        HazardClass::Class2 => 2,
        HazardClass::Class3 => 3,
        HazardClass::Class4 => 4,
    }
}

/// The higher of two classes.
const fn max_class(a: HazardClass, b: HazardClass) -> HazardClass {
    if class_rank(b) > class_rank(a) {
        b
    } else {
        a
    }
}

/// A class that can only go up. The engine starts from the entry's class and
/// raises it for each piece of context; there is no method that lowers it.
#[derive(Clone, Copy)]
struct Floor(HazardClass);

impl Floor {
    const fn raise(self, to: HazardClass) -> Self {
        Floor(max_class(self.0, to))
    }
}

/// The digest of the exact bus action a class 3 lease approves (#203): the
/// shared encoder's [`sawbones_modbus_encode::ActionDigest`] of the
/// [`BusWriteAction`] the request resolves to through the write map (bench
/// port, unit address, function, register, values).
///
/// The interlock's lease, the trust root's approval and the bus guard's lease
/// carry the same 32 bytes, and the guard passes only the one frame that
/// action encodes to. Any change to the port, unit, register, function or
/// value gives another digest, so a lease cannot be spent on a different
/// action.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ActionDigest([u8; 32]);

impl ActionDigest {
    /// The digest of a bus action.
    pub fn of(action: &BusWriteAction) -> Self {
        Self(action.digest().0)
    }

    /// The 32 digest bytes.
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// The digest as 64 lower-case hex digits.
    pub fn to_hex(&self) -> String {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let mut s = String::with_capacity(64);
        for b in self.0 {
            s.push(char::from(HEX[usize::from(b >> 4)]));
            s.push(char::from(HEX[usize::from(b & 0x0f)]));
        }
        s
    }
}

impl fmt::Debug for ActionDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ActionDigest({})", self.to_hex())
    }
}

impl fmt::Display for ActionDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_hex())
    }
}

impl Serialize for ActionDigest {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_hex())
    }
}

/// The same 32 bytes as the interlock's digest, for the lease request the
/// supervisor sends the trust root.
impl From<ActionDigest> for sawbones_interlock::ActionDigest {
    fn from(digest: ActionDigest) -> Self {
        sawbones_interlock::ActionDigest(digest.0)
    }
}

/// The same 32 bytes as the shared encoder's digest, which the approval and
/// the bus guard carry.
impl From<ActionDigest> for sawbones_modbus_encode::ActionDigest {
    fn from(digest: ActionDigest) -> Self {
        sawbones_modbus_encode::ActionDigest(digest.0)
    }
}

/// The register values and write function for a value: one register with
/// write single register (0x06) for a bool (0 or 1), u16 or i16; two
/// registers, high word first, with write multiple registers (0x10) for a
/// u32 or i32. Signed values are sent as their two's-complement bits.
fn register_words(value: WriteValue) -> (WriteFunction, [u16; 2], usize) {
    let single = |word: u16| (WriteFunction::WriteSingleRegister, [word, 0], 1);
    let double = |bytes: [u8; 4]| {
        (
            WriteFunction::WriteMultipleRegisters,
            [
                u16::from_be_bytes([bytes[0], bytes[1]]),
                u16::from_be_bytes([bytes[2], bytes[3]]),
            ],
            2,
        )
    };
    match value {
        WriteValue::Bool(b) => single(u16::from(b)),
        WriteValue::U16(n) => single(n),
        WriteValue::I16(n) => single(u16::from_be_bytes(n.to_be_bytes())),
        WriteValue::U32(n) => double(n.to_be_bytes()),
        WriteValue::I32(n) => double(n.to_be_bytes()),
    }
}

/// The one bus action a write is: `value` to `register` of `unit` on bench
/// port `bench_port`. `None` if no frame can carry it (a two-register value
/// at register 0xFFFF, or a unit outside 1 to 247).
pub fn bus_action(
    bench_port: u8,
    unit: u8,
    register: u16,
    value: WriteValue,
) -> Option<BusWriteAction> {
    let (function, words, count) = register_words(value);
    BusWriteAction::new(bench_port, unit, function, register, &words[..count]).ok()
}

/// Why a request was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(tag = "reason", rename_all = "snake_case")]
pub enum RefusalReason {
    /// The request is hazard class 4, from its entry or its setting kind.
    /// Class 4 is never done by the agent, whatever the configuration.
    ClassNeverPermitted,
    /// The write map has no entry for this signal on this port.
    UnknownSignal,
    /// The write map's entry is below [`Trust::OperatorAttested`], so it is
    /// treated as no entry.
    EntryNotAttested {
        /// The entry's tier.
        trust: Trust,
    },
    /// The written value is outside the entry's allowed range, or is a
    /// different type than the range.
    ValueOutOfRange,
    /// The request transmits on a port with no mastership fact.
    NoMastershipFact,
    /// The port's mastership fact is below [`Trust::OperatorAttested`], so it
    /// is treated as no fact.
    MastershipNotAttested {
        /// The fact's tier.
        trust: Trust,
    },
    /// The request transmits on a port where another master is active.
    OtherMasterActive,
    /// No Modbus frame can carry the write: a two-register value at the last
    /// register.
    UnencodableWrite,
}

/// What a request needs before it may run.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(tag = "needs", rename_all = "snake_case")]
pub enum Requirement {
    /// Nothing: class 0 or 1.
    Nothing,
    /// Armed rail A for this port: class 2.
    RailA {
        /// The port the action is on.
        port: PortId,
    },
    /// Keyed rail B and a one-time lease from a panel press, bound to this
    /// exact action: class 3.
    RailBLease {
        /// The port the action is on.
        port: PortId,
        /// The one bus write the lease approves, resolved through the write
        /// map. The trust root's approval carries it, and the bus guard
        /// passes only the frame it encodes to.
        action: BusWriteAction,
        /// The digest of `action`, which the interlock and the bus guard bind
        /// the lease to.
        action_digest: ActionDigest,
    },
    /// Not done. The class is 4.
    Refused {
        /// Why.
        reason: RefusalReason,
    },
}

impl Requirement {
    /// The interlock lease a [`Requirement::RailBLease`] asks for: the
    /// action's bench port and digest. `None` for every other requirement.
    pub fn lease_target(&self) -> Option<sawbones_interlock::LeaseTarget> {
        match self {
            Requirement::RailBLease {
                action,
                action_digest,
                ..
            } => Some(sawbones_interlock::LeaseTarget {
                port: sawbones_interlock::Port(action.port()),
                action_digest: (*action_digest).into(),
            }),
            _ => None,
        }
    }
}

/// The engine's answer: a class and what that class needs.
///
/// Only [`classify`] makes one, and the two always agree: classes 0 and 1
/// need [`Requirement::Nothing`], class 2 [`Requirement::RailA`], class 3
/// [`Requirement::RailBLease`], and class 4 is [`Requirement::Refused`]. A
/// refused request is always reported as class 4, whatever made it so.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct Decision {
    class: HazardClass,
    requirement: Requirement,
}

impl Decision {
    /// The class of the request.
    pub const fn class(&self) -> HazardClass {
        self.class
    }

    /// What the request needs.
    pub const fn requirement(&self) -> &Requirement {
        &self.requirement
    }

    /// Whether the request was refused.
    pub const fn is_refused(&self) -> bool {
        matches!(self.requirement, Requirement::Refused { .. })
    }

    fn refused(reason: RefusalReason) -> Self {
        Self {
            class: HazardClass::Class4,
            requirement: Requirement::Refused { reason },
        }
    }

    /// The single place a class is turned into a requirement. `action` is
    /// the bus action of a write; only writes reach class 3.
    fn for_class(floor: Floor, request: &EffectRequest, action: Option<BusWriteAction>) -> Self {
        let port = request.port().clone();
        let requirement = match floor.0 {
            HazardClass::Class0 | HazardClass::Class1 => Requirement::Nothing,
            HazardClass::Class2 => Requirement::RailA { port },
            HazardClass::Class3 => {
                let Some(action) = action else {
                    return Self::refused(RefusalReason::UnencodableWrite);
                };
                Requirement::RailBLease {
                    port,
                    action,
                    action_digest: ActionDigest::of(&action),
                }
            }
            HazardClass::Class4 => return Self::refused(RefusalReason::ClassNeverPermitted),
        };
        Self {
            class: floor.0,
            requirement,
        }
    }
}

/// Classifies one effect request against the write map and the bus facts.
///
/// A pure function: no I/O, no clock, and the answer depends only on the
/// arguments. The rules, in order:
///
/// - **Passive listening** transmits nothing and is class 0.
/// - **A bus write** needs a write-map entry for its port, unit and signal, at
///   [`Trust::Measured`] or [`Trust::OperatorAttested`]; otherwise it is
///   refused as class 4. Its class is the highest of the entry's class, the
///   setting kind's minimum ([`crate::SettingKind::minimum_class`]), class 2
///   if the entry is energy-relevant, and class 2 because a write transmits.
/// - **Anything that transmits** (a write or a read) is at least class 2, so
///   it needs rail A, and needs an actionable mastership fact for its port
///   saying the bench is the only master. With no fact, a fact below
///   operator-attested, or another master active, it is refused.
/// - **Class 4** is refused. Nothing in the facts can make it otherwise.
///
/// - **Value limits**: if the entry has a [`crate::ValueRange`] and the
///   written value is outside it (or of another type), the write is refused
///   with [`RefusalReason::ValueOutOfRange`]. An entry with no range accepts
///   any value. Class 4 is still reported first.
///
/// - **The bus action**: a write is resolved to one [`BusWriteAction`]: the
///   port's bench port number from its bus fact, the request's unit, the
///   entry's register, and the value as register words (see
///   [`bus_action`]). If no frame can carry it, it is refused with
///   [`RefusalReason::UnencodableWrite`]. A class 3 decision carries that
///   action and its [`ActionDigest`].
///
/// Context only ever raises a class. The value of a write never changes the
/// class; it can only make the write refused.
pub fn classify(request: &EffectRequest, writes: &WriteMap, buses: &BusFacts) -> Decision {
    let mut write_entry = None;
    let floor = match request {
        EffectRequest::BusListen(_) => Floor(HazardClass::Class0),
        EffectRequest::BusRead(_) => Floor(HazardClass::Class0).raise(HazardClass::Class2),
        EffectRequest::BusWrite(w) => {
            let Some(entry) = writes.get(&w.port, w.unit, &w.signal) else {
                return Decision::refused(RefusalReason::UnknownSignal);
            };
            if !actionable(entry.trust) {
                return Decision::refused(RefusalReason::EntryNotAttested { trust: entry.trust });
            }
            if let Some(range) = entry.range {
                if matches!(entry.class, HazardClass::Class4)
                    || matches!(entry.kind.minimum_class(), HazardClass::Class4)
                {
                    return Decision::refused(RefusalReason::ClassNeverPermitted);
                }
                if !range.contains(w.value) {
                    return Decision::refused(RefusalReason::ValueOutOfRange);
                }
            }
            write_entry = Some((entry, w));
            let mut floor = Floor(entry.class).raise(entry.kind.minimum_class());
            if entry.energy_relevant {
                floor = floor.raise(HazardClass::Class2);
            }
            floor.raise(HazardClass::Class2)
        }
    };

    // Report class 4 before any bus question: it is refused regardless.
    if matches!(floor.0, HazardClass::Class4) {
        return Decision::refused(RefusalReason::ClassNeverPermitted);
    }

    let mut action = None;
    if transmits(request) {
        let Some(fact) = buses.get(request.port()) else {
            return Decision::refused(RefusalReason::NoMastershipFact);
        };
        if !actionable(fact.trust) {
            return Decision::refused(RefusalReason::MastershipNotAttested { trust: fact.trust });
        }
        match fact.mastership {
            Mastership::BenchOnly => {}
            Mastership::OtherMasterActive => {
                return Decision::refused(RefusalReason::OtherMasterActive)
            }
        }
        if let Some((entry, w)) = write_entry {
            let Some(resolved) = bus_action(fact.bench_port, entry.unit, entry.register, w.value)
            else {
                return Decision::refused(RefusalReason::UnencodableWrite);
            };
            action = Some(resolved);
        }
    }

    Decision::for_class(floor, request, action)
}

/// Whether carrying out the request puts anything on the bus.
const fn transmits(request: &EffectRequest) -> bool {
    match request {
        EffectRequest::BusWrite(_) | EffectRequest::BusRead(_) => true,
        EffectRequest::BusListen(_) => false,
    }
}
