//! The facts the engine decides from: the write map and the bus mastership
//! facts.

use std::collections::BTreeMap;

use sawbones_interlock::HazardClass;
use sawbones_reading::Trust;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{PortId, SignalId};

/// What kind of setting a signal is. Some kinds carry a minimum class of
/// their own, which no write-map entry can go below.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SettingKind {
    /// An ordinary configuration setting. No minimum of its own; marking it
    /// energy-relevant makes it at least class 2.
    Config,
    /// A battery protection limit: a cut-off voltage, a current limit, a
    /// temperature limit. At least class 3: changeable only with a physical
    /// panel press for that one action (owner decision, 26 November 2026).
    BatteryProtectionLimit,
    /// A charge voltage. At least class 3, like a battery protection limit.
    ChargeVoltage,
    /// Switching mains. Class 4: never done by the agent.
    MainsSwitching,
    /// Switching a PV string. Class 4: never done by the agent.
    PvStringSwitching,
}

impl SettingKind {
    /// The lowest class a signal of this kind can have.
    pub const fn minimum_class(self) -> HazardClass {
        match self {
            SettingKind::Config => HazardClass::Class0,
            SettingKind::BatteryProtectionLimit | SettingKind::ChargeVoltage => HazardClass::Class3,
            SettingKind::MainsSwitching | SettingKind::PvStringSwitching => HazardClass::Class4,
        }
    }
}

/// One writable signal, as a fact: where it is, what class writing it is,
/// what kind of setting it is, and how much the entry itself has earned.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WriteEntry {
    /// The port or device the signal is on.
    pub port: PortId,
    /// The signal's name.
    pub signal: SignalId,
    /// The class of writing this signal, as the entry states it. The engine
    /// may raise it, never lower it.
    pub class: HazardClass,
    /// What kind of setting the signal is.
    pub kind: SettingKind,
    /// Whether writing the signal can change an energy path. If so, writing
    /// it is at least class 2, whatever its kind.
    pub energy_relevant: bool,
    /// How much this entry has earned. Only [`Trust::Measured`] and
    /// [`Trust::OperatorAttested`] entries are acted on; any other tier is
    /// treated as no entry at all.
    pub trust: Trust,
}

/// Why a set of facts was rejected.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum FactError {
    /// Two write-map entries name the same signal on the same port.
    #[error("two write-map entries for signal {signal} on port {port}")]
    DuplicateWriteEntry {
        /// The port.
        port: PortId,
        /// The signal.
        signal: SignalId,
    },
    /// Two mastership facts name the same port.
    #[error("two mastership facts for port {port}")]
    DuplicateBusFact {
        /// The port.
        port: PortId,
    },
}

/// Every writable signal the bench knows about, at most one entry per signal
/// per port. A signal with no entry cannot be written.
///
/// Deserialises from a JSON array of [`WriteEntry`] and rejects duplicates,
/// so a map that loads is a map with one answer per signal.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "Vec<WriteEntry>", into = "Vec<WriteEntry>")]
pub struct WriteMap {
    entries: BTreeMap<(PortId, SignalId), WriteEntry>,
}

impl WriteMap {
    /// Builds a write map. Fails if two entries name the same signal on the
    /// same port, since the engine would not know which to believe.
    pub fn new(entries: impl IntoIterator<Item = WriteEntry>) -> Result<Self, FactError> {
        let mut map = BTreeMap::new();
        for entry in entries {
            let key = (entry.port.clone(), entry.signal.clone());
            if map.contains_key(&key) {
                return Err(FactError::DuplicateWriteEntry {
                    port: entry.port,
                    signal: entry.signal,
                });
            }
            map.insert(key, entry);
        }
        Ok(Self { entries: map })
    }

    /// The entry for a signal on a port, if there is one.
    pub fn get(&self, port: &PortId, signal: &SignalId) -> Option<&WriteEntry> {
        self.entries.get(&(port.clone(), signal.clone()))
    }

    /// Every entry, ordered by port and then signal.
    pub fn entries(&self) -> impl Iterator<Item = &WriteEntry> {
        self.entries.values()
    }
}

impl TryFrom<Vec<WriteEntry>> for WriteMap {
    type Error = FactError;
    fn try_from(entries: Vec<WriteEntry>) -> Result<Self, Self::Error> {
        Self::new(entries)
    }
}

impl From<WriteMap> for Vec<WriteEntry> {
    fn from(map: WriteMap) -> Self {
        map.entries.into_values().collect()
    }
}

/// Who may transmit on a bus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mastership {
    /// The bench is the only master. Its requests cannot collide with anyone
    /// else's.
    BenchOnly,
    /// Another master is active on the bus, for example an inverter's own
    /// monitor polling it. The bench must not transmit.
    OtherMasterActive,
}

/// A declared fact about who masters one bus port.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BusFact {
    /// The port.
    pub port: PortId,
    /// Who may transmit on it.
    pub mastership: Mastership,
    /// How much this fact has earned. As for a [`WriteEntry`], only
    /// [`Trust::Measured`] and [`Trust::OperatorAttested`] count.
    pub trust: Trust,
}

/// The mastership facts for every port, at most one per port. A port with no
/// fact is a port the bench must not transmit on.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "Vec<BusFact>", into = "Vec<BusFact>")]
pub struct BusFacts {
    facts: BTreeMap<PortId, BusFact>,
}

impl BusFacts {
    /// Builds the set. Fails if two facts name the same port.
    pub fn new(facts: impl IntoIterator<Item = BusFact>) -> Result<Self, FactError> {
        let mut map = BTreeMap::new();
        for fact in facts {
            if map.contains_key(&fact.port) {
                return Err(FactError::DuplicateBusFact { port: fact.port });
            }
            map.insert(fact.port.clone(), fact);
        }
        Ok(Self { facts: map })
    }

    /// The fact for a port, if there is one.
    pub fn get(&self, port: &PortId) -> Option<&BusFact> {
        self.facts.get(port)
    }

    /// Every fact, ordered by port.
    pub fn facts(&self) -> impl Iterator<Item = &BusFact> {
        self.facts.values()
    }
}

impl TryFrom<Vec<BusFact>> for BusFacts {
    type Error = FactError;
    fn try_from(facts: Vec<BusFact>) -> Result<Self, Self::Error> {
        Self::new(facts)
    }
}

impl From<BusFacts> for Vec<BusFact> {
    fn from(facts: BusFacts) -> Self {
        facts.facts.into_values().collect()
    }
}

/// Whether a tier is high enough for a fact to be acted on. An exhaustive
/// match, so a new tier cannot be added without deciding here.
pub(crate) const fn actionable(trust: Trust) -> bool {
    match trust {
        Trust::Measured | Trust::OperatorAttested => true,
        Trust::Documented | Trust::AgentInferred | Trust::Unverified => false,
    }
}
