//! The module kinds: what each one is, its manifest, its class table and what
//! it measures.

use sawbones_module_protocol::{
    Capabilities, Channel, ClassTable, ConfigKey, HazardClass, ManifestDigest, ManifestHasher,
};

use crate::rng::mix;

/// The first config key that is a live reading (read only), not a setting.
pub const READING_KEY_BASE: u16 = 0x100;

/// The kinds of module #99 asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModuleKind {
    /// A DC meter: reads voltage (0x100, mV) and current (0x101, mA).
    DcMeter,
    /// A comms pod: reads frames received (0x100) and link quality (0x101).
    /// It can transmit, so its identity carries `can_emit`.
    CommsPod,
    /// An interlock controller: reads a heartbeat count (0x100) and rail A
    /// voltage (0x101, mV). Channel 1 is the class 3 rail B port.
    InterlockController,
}

impl ModuleKind {
    /// Every kind.
    pub const ALL: [ModuleKind; 3] = [
        ModuleKind::DcMeter,
        ModuleKind::CommsPod,
        ModuleKind::InterlockController,
    ];

    /// A short name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            ModuleKind::DcMeter => "dc-meter",
            ModuleKind::CommsPod => "comms-pod",
            ModuleKind::InterlockController => "interlock-controller",
        }
    }

    /// The manifest pages the module serves for `Describe`.
    #[must_use]
    pub const fn manifest(self) -> &'static [&'static [u8]] {
        match self {
            ModuleKind::DcMeter => &[
                b"name: dc-meter (simulated)\nreadings: 0x100 voltage mV, 0x101 current mA\n",
                b"config: 1 averaging class 1; 9 mains class 4\n",
            ],
            ModuleKind::CommsPod => &[
                b"name: comms-pod (simulated)\nreadings: 0x100 rx frames, 0x101 link quality\n",
                b"config: 1 channel class 1; 2 tx power class 2; 9 mains class 4\ncan_emit: yes\n",
            ],
            ModuleKind::InterlockController => &[
                b"name: interlock-controller (simulated)\nreadings: 0x100 heartbeat, 0x101 rail A mV\n",
                b"config: 1 period class 1; 9 mains class 4\nchannels: 0 rail A class 2; 1 rail B class 3\n",
            ],
        }
    }

    /// The digest of the manifest, as the module signs it.
    #[must_use]
    pub fn manifest_digest(self) -> ManifestDigest {
        let mut hasher = ManifestHasher::new();
        for page in self.manifest() {
            hasher.bytes(page);
        }
        hasher.finish()
    }

    /// What the module's signed identity says it is.
    #[must_use]
    pub const fn capabilities(self) -> Capabilities {
        match self {
            ModuleKind::DcMeter => Capabilities {
                can_emit: false,
                max_class: HazardClass::Class1,
            },
            ModuleKind::CommsPod => Capabilities {
                can_emit: true,
                max_class: HazardClass::Class2,
            },
            ModuleKind::InterlockController => Capabilities {
                can_emit: false,
                max_class: HazardClass::Class3,
            },
        }
    }

    /// The module's own class table.
    #[must_use]
    pub const fn table(self) -> KindTable {
        KindTable(self)
    }

    /// The true value of reading `key` at `tick`, with no fault applied.
    /// `None` if the kind has no such reading.
    #[must_use]
    pub fn truth(self, noise_seed: u64, key: u16, tick: u64) -> Option<i64> {
        let wobble = |span: u64| {
            // A small deterministic noise, so a stuck value is distinguishable.
            let n = mix(noise_seed ^ (u64::from(key) << 48) ^ tick) % (2 * span + 1);
            i64::try_from(n).unwrap_or(0) - i64::try_from(span).unwrap_or(0)
        };
        let ticks = i64::try_from(tick).unwrap_or(i64::MAX);
        match (self, key.checked_sub(READING_KEY_BASE)?) {
            (ModuleKind::DcMeter, 0) => Some(12_000 + wobble(5)),
            (ModuleKind::DcMeter, 1) => Some(1_500 + wobble(5)),
            (ModuleKind::CommsPod, 0) => Some(ticks / 10),
            (ModuleKind::CommsPod, 1) => Some(90 + wobble(3)),
            (ModuleKind::InterlockController, 0) => Some(ticks / 20),
            (ModuleKind::InterlockController, 1) => Some(12_000 + wobble(5)),
            _ => None,
        }
    }
}

/// A [`ModuleKind`]'s class table.
#[derive(Debug, Clone, Copy)]
pub struct KindTable(ModuleKind);

impl ClassTable for KindTable {
    fn config_class(&self, key: ConfigKey) -> Option<HazardClass> {
        match (self.0, key.0) {
            (_, 1) => Some(HazardClass::Class1),
            (ModuleKind::CommsPod, 2) => Some(HazardClass::Class2),
            (_, 9) => Some(HazardClass::Class4),
            _ => None,
        }
    }

    fn channel_class(&self, channel: Channel) -> Option<HazardClass> {
        match (self.0, channel.0) {
            (ModuleKind::CommsPod | ModuleKind::InterlockController, 0) => {
                Some(HazardClass::Class2)
            }
            (ModuleKind::InterlockController, 1) => Some(HazardClass::Class3),
            _ => None,
        }
    }
}
