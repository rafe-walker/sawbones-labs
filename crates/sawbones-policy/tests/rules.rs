//! Table tests: one row per rule, against one fixed set of facts.

mod common;

use common::*;
use sawbones_policy::{
    classify, BusFacts, Decision, EffectRequest, FactError, HazardClass, Mastership, RefusalReason,
    Requirement, SettingKind, WriteEntry, WriteMap, WriteValue,
};
use sawbones_reading::Trust;

use HazardClass::{Class0, Class1, Class2, Class3, Class4};
use SettingKind::{
    BatteryProtectionLimit, ChargeVoltage, Config, MainsSwitching, PvStringSwitching,
};

/// The bench's port, mastered by the bench alone.
const BENCH: &str = "rs485-1";
/// A port where an inverter's own monitor is also a master.
const SHARED: &str = "rs485-shared";
/// A port with no mastership fact.
const NO_FACT: &str = "rs485-nofact";
/// A port whose mastership fact an agent inferred.
const GUESSED: &str = "rs485-guessed";

fn writes() -> WriteMap {
    WriteMap::new([
        entry(
            BENCH,
            "battery_low_cutoff",
            Class2,
            BatteryProtectionLimit,
            true,
            Trust::OperatorAttested,
        ),
        entry(
            BENCH,
            "charge_voltage",
            Class1,
            ChargeVoltage,
            false,
            Trust::Measured,
        ),
        entry(
            BENCH,
            "grid_relay",
            Class0,
            MainsSwitching,
            false,
            Trust::OperatorAttested,
        ),
        entry(
            BENCH,
            "pv_string_1",
            Class1,
            PvStringSwitching,
            true,
            Trust::Measured,
        ),
        entry(
            BENCH,
            "display_brightness",
            Class1,
            Config,
            false,
            Trust::OperatorAttested,
        ),
        entry(
            BENCH,
            "export_limit",
            Class0,
            Config,
            true,
            Trust::OperatorAttested,
        ),
        entry(
            BENCH,
            "factory_reset",
            Class3,
            Config,
            false,
            Trust::OperatorAttested,
        ),
        entry(
            BENCH,
            "firmware_mode",
            Class4,
            Config,
            false,
            Trust::Measured,
        ),
        entry(
            BENCH,
            "guessed_register",
            Class1,
            Config,
            false,
            Trust::AgentInferred,
        ),
        entry(
            BENCH,
            "imported_register",
            Class1,
            Config,
            false,
            Trust::Unverified,
        ),
        entry(
            BENCH,
            "manual_register",
            Class1,
            Config,
            false,
            Trust::Documented,
        ),
        entry(
            SHARED,
            "display_brightness",
            Class1,
            Config,
            false,
            Trust::OperatorAttested,
        ),
        entry(
            NO_FACT,
            "display_brightness",
            Class1,
            Config,
            false,
            Trust::OperatorAttested,
        ),
        entry(
            NO_FACT,
            "grid_relay",
            Class0,
            MainsSwitching,
            false,
            Trust::OperatorAttested,
        ),
        entry(
            GUESSED,
            "display_brightness",
            Class1,
            Config,
            false,
            Trust::OperatorAttested,
        ),
    ])
    .unwrap()
}

fn buses() -> BusFacts {
    BusFacts::new([
        fact(BENCH, Mastership::BenchOnly, Trust::OperatorAttested),
        fact(
            SHARED,
            Mastership::OtherMasterActive,
            Trust::OperatorAttested,
        ),
        fact(GUESSED, Mastership::BenchOnly, Trust::AgentInferred),
    ])
    .unwrap()
}

/// What a row expects, without the port and digest, which the check fills
/// in from the request.
#[derive(Debug, Clone, Copy)]
enum Expect {
    Nothing(HazardClass),
    RailA,
    Lease,
    Refused(RefusalReason),
}

fn check(name: &str, request: EffectRequest, expect: Expect) {
    let decision = classify(&request, &writes(), &buses());
    let port = request.port().clone();
    let (class, requirement) = match expect {
        Expect::Nothing(class) => (class, Requirement::Nothing),
        Expect::RailA => (Class2, Requirement::RailA { port }),
        Expect::Lease => (Class3, lease_for(&request)),
        Expect::Refused(reason) => (Class4, Requirement::Refused { reason }),
    };
    assert_eq!(decision.class(), class, "{name}: class");
    assert_eq!(decision.requirement(), &requirement, "{name}: requirement");
}

const V: WriteValue = WriteValue::U16(1);

#[test]
fn every_rule_in_one_table() {
    use RefusalReason::*;
    let rows: Vec<(&str, EffectRequest, Expect)> = vec![
        // Setting kinds with a minimum class.
        (
            "battery limit",
            write(BENCH, "battery_low_cutoff", V),
            Expect::Lease,
        ),
        (
            "charge voltage",
            write(BENCH, "charge_voltage", V),
            Expect::Lease,
        ),
        (
            "mains switching",
            write(BENCH, "grid_relay", V),
            Expect::Refused(ClassNeverPermitted),
        ),
        (
            "pv string",
            write(BENCH, "pv_string_1", V),
            Expect::Refused(ClassNeverPermitted),
        ),
        // Config: the entry's class, raised to 2 by energy and transmission.
        (
            "energy config",
            write(BENCH, "export_limit", V),
            Expect::RailA,
        ),
        (
            "plain config",
            write(BENCH, "display_brightness", V),
            Expect::RailA,
        ),
        (
            "class 3 config",
            write(BENCH, "factory_reset", V),
            Expect::Lease,
        ),
        (
            "class 4 config",
            write(BENCH, "firmware_mode", V),
            Expect::Refused(ClassNeverPermitted),
        ),
        // Unknown signals and unattested entries are class 4.
        (
            "unknown signal",
            write(BENCH, "no_such_register", V),
            Expect::Refused(UnknownSignal),
        ),
        (
            "signal on another port",
            write(SHARED, "charge_voltage", V),
            Expect::Refused(UnknownSignal),
        ),
        (
            "agent-inferred entry",
            write(BENCH, "guessed_register", V),
            Expect::Refused(EntryNotAttested {
                trust: Trust::AgentInferred,
            }),
        ),
        (
            "unverified entry",
            write(BENCH, "imported_register", V),
            Expect::Refused(EntryNotAttested {
                trust: Trust::Unverified,
            }),
        ),
        (
            "documented entry",
            write(BENCH, "manual_register", V),
            Expect::Refused(EntryNotAttested {
                trust: Trust::Documented,
            }),
        ),
        // Writes transmit, so they need the bench as sole master too.
        (
            "write, other master",
            write(SHARED, "display_brightness", V),
            Expect::Refused(OtherMasterActive),
        ),
        (
            "write, no fact",
            write(NO_FACT, "display_brightness", V),
            Expect::Refused(NoMastershipFact),
        ),
        (
            "write, guessed fact",
            write(GUESSED, "display_brightness", V),
            Expect::Refused(MastershipNotAttested {
                trust: Trust::AgentInferred,
            }),
        ),
        // Class 4 is reported as such even where the bus would also refuse.
        (
            "class 4, no fact",
            write(NO_FACT, "grid_relay", V),
            Expect::Refused(ClassNeverPermitted),
        ),
        // Reads that transmit.
        ("read, bench master", read(BENCH, "pv_power"), Expect::RailA),
        (
            "read, other master",
            read(SHARED, "pv_power"),
            Expect::Refused(OtherMasterActive),
        ),
        (
            "read, no fact",
            read(NO_FACT, "pv_power"),
            Expect::Refused(NoMastershipFact),
        ),
        (
            "read, guessed fact",
            read(GUESSED, "pv_power"),
            Expect::Refused(MastershipNotAttested {
                trust: Trust::AgentInferred,
            }),
        ),
        // Passive listening needs nothing, on any port.
        ("listen, bench", listen(BENCH), Expect::Nothing(Class0)),
        (
            "listen, other master",
            listen(SHARED),
            Expect::Nothing(Class0),
        ),
        ("listen, no fact", listen(NO_FACT), Expect::Nothing(Class0)),
    ];
    for (name, request, expect) in rows {
        check(name, request, expect);
    }
}

/// The review document had battery protection limits at class 4; the owner
/// decided class 3 with a panel press per action (26 November 2026).
#[test]
fn regression_171_battery_limit_is_class_3_not_4() {
    let request = write(BENCH, "battery_low_cutoff", WriteValue::U16(4600));
    let decision = classify(&request, &writes(), &buses());
    assert_eq!(decision.class(), Class3);
    assert_eq!(decision.requirement(), &lease_for(&request));
}

/// A battery limit or charge voltage entry stated at class 0, 1 or 2 is
/// still class 3: the kind's minimum wins.
#[test]
fn regression_171_battery_kinds_cannot_be_stated_below_class_3() {
    for kind in [BatteryProtectionLimit, ChargeVoltage] {
        for class in [Class0, Class1, Class2] {
            let writes =
                WriteMap::new([entry(BENCH, "limit", class, kind, false, Trust::Measured)])
                    .unwrap();
            let decision = classify(&write(BENCH, "limit", V), &writes, &buses());
            assert_eq!(decision.class(), Class3, "{kind:?} stated as {class:?}");
        }
    }
}

/// A Modbus read sends a request, so it is not class 1.
#[test]
fn regression_171_transmitting_read_is_not_class_1() {
    let decision = classify(&read(BENCH, "pv_power"), &writes(), &buses());
    assert_eq!(decision.class(), Class2);
    assert_eq!(
        decision.requirement(),
        &Requirement::RailA { port: port(BENCH) }
    );
    // And with no mastership fact at all, it does not happen.
    let decision = classify(&read(BENCH, "pv_power"), &writes(), &BusFacts::default());
    assert!(decision.is_refused());
}

/// A register with no measured or operator-attested entry is class 4.
#[test]
fn regression_171_unattested_register_is_class_4() {
    for trust in [Trust::Documented, Trust::AgentInferred, Trust::Unverified] {
        let writes = WriteMap::new([entry(BENCH, "reg", Class0, Config, false, trust)]).unwrap();
        let decision = classify(&write(BENCH, "reg", V), &writes, &buses());
        assert_eq!(decision.class(), Class4, "{trust:?}");
        assert!(decision.is_refused(), "{trust:?}");
    }
}

#[test]
fn energy_relevant_entries_are_at_least_class_2_and_never_lowered() {
    for kind in KINDS {
        for class in CLASSES {
            let writes =
                WriteMap::new([entry(BENCH, "s", class, kind, true, Trust::Measured)]).unwrap();
            let decision = classify(&write(BENCH, "s", V), &writes, &buses());
            assert!(
                class_at_least(&decision, Class2),
                "{kind:?} at {class:?}: {decision:?}"
            );
        }
    }
}

fn class_at_least(decision: &Decision, floor: HazardClass) -> bool {
    sawbones_policy::class_rank(decision.class()) >= sawbones_policy::class_rank(floor)
}

#[test]
fn an_empty_world_refuses_every_transmission() {
    let none = WriteMap::default();
    let no_buses = BusFacts::default();
    assert!(classify(&write(BENCH, "x", V), &none, &no_buses).is_refused());
    assert!(classify(&read(BENCH, "x"), &none, &no_buses).is_refused());
    assert!(!classify(&listen(BENCH), &none, &no_buses).is_refused());
}

#[test]
fn duplicate_facts_are_rejected() {
    let dup = WriteMap::new([
        entry(BENCH, "s", Class1, Config, false, Trust::Measured),
        entry(BENCH, "s", Class3, Config, false, Trust::Measured),
    ]);
    assert_eq!(
        dup,
        Err(FactError::DuplicateWriteEntry {
            port: port(BENCH),
            unit: UNIT,
            signal: signal("s"),
        })
    );
    let dup = BusFacts::new([
        fact(BENCH, Mastership::BenchOnly, Trust::Measured),
        fact(BENCH, Mastership::OtherMasterActive, Trust::Measured),
    ]);
    assert_eq!(dup, Err(FactError::DuplicateBusFact { port: port(BENCH) }));
}

#[test]
fn the_value_plays_no_part_in_the_class() {
    let values = [
        WriteValue::Bool(true),
        WriteValue::U16(0),
        WriteValue::I16(-1),
        WriteValue::U32(u32::MAX),
        WriteValue::I32(i32::MIN),
    ];
    for s in ["display_brightness", "battery_low_cutoff", "grid_relay"] {
        let classes: Vec<_> = values
            .iter()
            .map(|v| {
                // Each entry states the width of the value written (#214).
                let map = WriteMap::new(writes().entries().map(|e| WriteEntry {
                    width: v.width(),
                    ..e.clone()
                }))
                .unwrap();
                classify(&write(BENCH, s, *v), &map, &buses()).class()
            })
            .collect();
        assert!(classes.windows(2).all(|w| w[0] == w[1]), "{s}: {classes:?}");
    }
}
