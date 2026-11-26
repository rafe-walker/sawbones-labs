//! Helpers shared by the test files.

#![allow(dead_code)]

use sawbones_policy::{
    BusFact, BusListen, BusRead, BusWrite, EffectRequest, HazardClass, Mastership, PortId,
    SettingKind, SignalId, WriteEntry, WriteValue,
};
use sawbones_reading::{Ident, Trust};

pub const CLASSES: [HazardClass; 5] = [
    HazardClass::Class0,
    HazardClass::Class1,
    HazardClass::Class2,
    HazardClass::Class3,
    HazardClass::Class4,
];

pub const KINDS: [SettingKind; 5] = [
    SettingKind::Config,
    SettingKind::BatteryProtectionLimit,
    SettingKind::ChargeVoltage,
    SettingKind::MainsSwitching,
    SettingKind::PvStringSwitching,
];

pub const TIERS: [Trust; 5] = [
    Trust::Measured,
    Trust::OperatorAttested,
    Trust::Documented,
    Trust::AgentInferred,
    Trust::Unverified,
];

pub const MASTERSHIPS: [Mastership; 2] = [Mastership::BenchOnly, Mastership::OtherMasterActive];

pub fn port(s: &str) -> PortId {
    PortId::new(Ident::new(s).unwrap())
}

pub fn signal(s: &str) -> SignalId {
    SignalId::new(Ident::new(s).unwrap())
}

pub fn entry(
    p: &str,
    s: &str,
    class: HazardClass,
    kind: SettingKind,
    energy_relevant: bool,
    trust: Trust,
) -> WriteEntry {
    WriteEntry {
        port: port(p),
        signal: signal(s),
        class,
        kind,
        energy_relevant,
        trust,
    }
}

pub fn fact(p: &str, mastership: Mastership, trust: Trust) -> BusFact {
    BusFact {
        port: port(p),
        mastership,
        trust,
    }
}

pub fn write(p: &str, s: &str, value: WriteValue) -> EffectRequest {
    EffectRequest::BusWrite(BusWrite {
        port: port(p),
        signal: signal(s),
        value,
    })
}

pub fn read(p: &str, s: &str) -> EffectRequest {
    EffectRequest::BusRead(BusRead {
        port: port(p),
        signal: signal(s),
    })
}

pub fn listen(p: &str) -> EffectRequest {
    EffectRequest::BusListen(BusListen { port: port(p) })
}

/// SplitMix64: a small, well-known generator, so the randomised tests need
/// no dependency and every failure is reproducible from its seed.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    /// A value in `0..n`.
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }

    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }

    pub fn pick<T: Copy>(&mut self, items: &[T]) -> T {
        items[self.below(items.len())]
    }

    /// A name from a small pool, so that maps and requests collide often.
    pub fn name(&mut self, prefix: &str) -> String {
        format!("{prefix}-{}", self.below(6))
    }

    pub fn value(&mut self) -> WriteValue {
        match self.below(5) {
            0 => WriteValue::Bool(self.bool()),
            1 => WriteValue::U16(self.next_u64() as u16),
            2 => WriteValue::I16(self.next_u64() as i16),
            3 => WriteValue::U32(self.next_u64() as u32),
            _ => WriteValue::I32(self.next_u64() as i32),
        }
    }
}
