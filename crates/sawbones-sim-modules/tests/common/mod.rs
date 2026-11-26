//! Shared test setup. The seeds here are TEST ONLY and public: they live under
//! `tests/`, never in `src/`.
#![allow(dead_code)]

use ed25519_dalek::SigningKey;
use sawbones_module_protocol::{Actuate, Channel, ConfigKey, Grant, ModuleId, ModuleKey, Request};
use sawbones_sim_modules::{
    scenarios, HostHarness, ModuleKind, Outcome, Scenario, SimLink, SimModule, Summary,
};

/// TEST ONLY. The simulated module's device key seed.
pub const DEVICE_SEED: [u8; 32] = *b"SAWBONES-TEST-KEY sim-modules-01";
/// TEST ONLY. Another device's key seed.
pub const OTHER_SEED: [u8; 32] = *b"SAWBONES-TEST-KEY sim-modules-02";
/// TEST ONLY. Seeds the module's session ids and noise.
pub const MODULE_RNG: u64 = 0x5eed_0001;
/// TEST ONLY. Seeds the host's challenges.
pub const HOST_RNG: u64 = 0x5eed_0002;
/// TEST ONLY. Seeds the scenarios.
pub const SCENARIO_SEED: u64 = 0x5eed_0003;

pub const MODULE: ModuleId = ModuleId(1);
pub const OTHER: ModuleId = ModuleId(2);
/// Scenarios start here; the session is opened before.
pub const FROM: u64 = 100;
/// The module's link timeout in these tests.
pub const LINK_TIMEOUT: u64 = 1000;

pub fn key_of(seed: &[u8; 32]) -> ModuleKey {
    ModuleKey(SigningKey::from_bytes(seed).verifying_key().to_bytes())
}

pub struct Bench {
    pub host: HostHarness,
    pub line: SimLink,
}

pub fn module(kind: ModuleKind) -> SimModule {
    SimModule::new(kind, MODULE, DEVICE_SEED, MODULE_RNG).with_link_timeout(LINK_TIMEOUT)
}

pub fn host_for(kind: ModuleKind) -> HostHarness {
    HostHarness::new(MODULE, key_of(&DEVICE_SEED), kind.capabilities(), HOST_RNG)
}

/// A clean bench with the session open and trusted.
pub fn open(kind: ModuleKind) -> Bench {
    let mut b = Bench {
        host: host_for(kind),
        line: SimLink::new(module(kind), None),
    };
    let out = b.host.open(&mut b.line, 20);
    assert!(matches!(out, Outcome::Replied(_)), "{out:?}");
    assert!(b.host.trusted());
    b
}

pub fn scenario(name: &str) -> Scenario {
    scenarios(SCENARIO_SEED, FROM, OTHER)
        .into_iter()
        .find(|s| s.name == name)
        .unwrap_or_else(|| panic!("no scenario {name}"))
}

/// A bench for `kind` with `scenario` on the line. The session is opened
/// before the scenario starts, unless `open_first` is false.
pub fn with(kind: ModuleKind, scenario: Scenario, open_first: bool) -> Bench {
    let mut b = Bench {
        host: host_for(kind),
        line: SimLink::new(module(kind), Some(scenario)),
    };
    if open_first {
        let out = b.host.open(&mut b.line, 20);
        assert!(matches!(out, Outcome::Replied(_)), "{out:?}");
        assert!(b.host.trusted());
    }
    b
}

/// A bench with the named scenario, opened, and advanced to the first tick
/// the scenario is active.
pub fn armed(name: &str) -> Bench {
    armed_scenario(scenario(name))
}

pub fn armed_scenario(s: Scenario) -> Bench {
    let mut b = with(ModuleKind::InterlockController, s, true);
    assert!(b.host.now < FROM);
    // Anything the scenario does by itself waits for the first active tick.
    b.host.idle(&mut b.line, FROM - 1 - b.host.now);
    assert!(b.host.refused.is_empty(), "{:?}", b.host.refused);
    // The scenario's first tick.
    b.host.now = FROM;
    b
}

pub fn actuate(b: &mut Bench, channel: u8, setting: i32, timeout: u64) -> Outcome {
    let a = Actuate::new(MODULE, Channel(channel), setting, Grant::RailA);
    b.host.call(&mut b.line, &Request::Actuate(a), timeout)
}

pub fn health(b: &mut Bench, timeout: u64) -> Outcome {
    b.host.call(&mut b.line, &Request::Health, timeout)
}

pub fn read(b: &mut Bench, key: u16, timeout: u64) -> Outcome {
    b.host.call(
        &mut b.line,
        &Request::ConfigRead {
            key: ConfigKey(key),
        },
        timeout,
    )
}

pub fn value(out: Outcome) -> i64 {
    match out {
        Outcome::Replied(Summary { value: Some(v), .. }) => v,
        other => panic!("not a value: {other:?}"),
    }
}
