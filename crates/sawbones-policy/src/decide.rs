//! The engine: one pure function from a request and the facts to a decision.

use std::fmt;

use sawbones_interlock::HazardClass;
use sawbones_reading::Trust;
use serde::{Serialize, Serializer};
use sha2::{Digest, Sha256};

use crate::facts::actionable;
use crate::{BusFacts, EffectRequest, Mastership, PortId, WriteMap};

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

/// SHA-256 over [`EffectRequest::canonical_bytes`]: the exact action a class
/// 3 lease approves. Any change to any field of the request gives another
/// digest, so a lease cannot be spent on a different action.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ActionDigest([u8; 32]);

impl ActionDigest {
    /// The digest of a request.
    pub fn of(request: &EffectRequest) -> Self {
        Self(Sha256::digest(request.canonical_bytes()).into())
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
        /// The digest of the exact request the lease approves.
        action_digest: ActionDigest,
    },
    /// Not done. The class is 4.
    Refused {
        /// Why.
        reason: RefusalReason,
    },
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

    /// The single place a class is turned into a requirement.
    fn for_class(floor: Floor, request: &EffectRequest) -> Self {
        let port = request.port().clone();
        let requirement = match floor.0 {
            HazardClass::Class0 | HazardClass::Class1 => Requirement::Nothing,
            HazardClass::Class2 => Requirement::RailA { port },
            HazardClass::Class3 => Requirement::RailBLease {
                port,
                action_digest: ActionDigest::of(request),
            },
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
/// - **A bus write** needs a write-map entry for its port and signal, at
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
/// Context only ever raises a class. The value of a write plays no part.
pub fn classify(request: &EffectRequest, writes: &WriteMap, buses: &BusFacts) -> Decision {
    let floor = match request {
        EffectRequest::BusListen(_) => Floor(HazardClass::Class0),
        EffectRequest::BusRead(_) => Floor(HazardClass::Class0).raise(HazardClass::Class2),
        EffectRequest::BusWrite(w) => {
            let Some(entry) = writes.get(&w.port, &w.signal) else {
                return Decision::refused(RefusalReason::UnknownSignal);
            };
            if !actionable(entry.trust) {
                return Decision::refused(RefusalReason::EntryNotAttested { trust: entry.trust });
            }
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
    }

    Decision::for_class(floor, request)
}

/// Whether carrying out the request puts anything on the bus.
const fn transmits(request: &EffectRequest) -> bool {
    match request {
        EffectRequest::BusWrite(_) | EffectRequest::BusRead(_) => true,
        EffectRequest::BusListen(_) => false,
    }
}
