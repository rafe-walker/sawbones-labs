//! Helpers shared by the test files.

#![allow(dead_code)]

use sawbones_policy::{
    bus_action, ActionDigest, BusFact, BusListen, BusRead, BusWrite, BusWriteAction, EffectRequest,
    HazardClass, Mastership, PortId, Requirement, SettingKind, SignalId, WriteEntry, WriteValue,
};
use sawbones_reading::Trust;

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

/// The unit the helpers' entries and requests name.
pub const UNIT: u8 = 1;

/// A bench port number for a port name: the sum of its bytes, so the small
/// name pools the tests use never collide.
pub fn bench_port_of(p: &str) -> u8 {
    p.bytes().fold(0u8, u8::wrapping_add)
}

/// A register for a signal name, likewise from its bytes, kept clear of the
/// last register so two-register values always encode.
pub fn register_of(s: &str) -> u16 {
    s.bytes().fold(0u16, |acc, b| {
        acc.wrapping_mul(31).wrapping_add(u16::from(b))
    }) % 0xFF00
}

pub fn port(s: &str) -> PortId {
    PortId::parse(s).unwrap()
}

pub fn signal(s: &str) -> SignalId {
    SignalId::parse(s).unwrap()
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
        unit: UNIT,
        signal: signal(s),
        register: register_of(s),
        class,
        kind,
        energy_relevant,
        trust,
        range: None,
    }
}

pub fn fact(p: &str, mastership: Mastership, trust: Trust) -> BusFact {
    BusFact {
        port: port(p),
        bench_port: bench_port_of(p),
        mastership,
        trust,
    }
}

pub fn write(p: &str, s: &str, value: WriteValue) -> EffectRequest {
    EffectRequest::BusWrite(BusWrite {
        port: port(p),
        unit: UNIT,
        signal: signal(s),
        value,
    })
}

pub fn read(p: &str, s: &str) -> EffectRequest {
    EffectRequest::BusRead(BusRead {
        port: port(p),
        unit: UNIT,
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

/// The bus action the helpers' facts resolve a helper write to: the port's
/// bench port, [`UNIT`], the signal's register and the value's words.
pub fn expected_action(request: &EffectRequest) -> BusWriteAction {
    let EffectRequest::BusWrite(w) = request else {
        panic!("only a write has a bus action: {request:?}")
    };
    bus_action(
        bench_port_of(w.port.as_str()),
        w.unit,
        register_of(w.signal.as_str()),
        w.value,
    )
    .unwrap()
}

/// The class 3 requirement for a helper write.
pub fn lease_for(request: &EffectRequest) -> Requirement {
    let action = expected_action(request);
    Requirement::RailBLease {
        port: request.port().clone(),
        action,
        action_digest: ActionDigest::of(&action),
    }
}
