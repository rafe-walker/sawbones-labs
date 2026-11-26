//! Regression tests for #214 (red-team cards bus-015 and bus-017): a
//! signal's register width comes from its write-map entry, never from the
//! type of the value written.

mod common;

use common::*;
use sawbones_policy::{
    bus_action, bus_action_for_width, classify, BusFacts, FactError, HazardClass, Mastership,
    RefusalReason, Requirement, SettingKind, ValueRange, WriteEntry, WriteMap, WriteValue,
};
use sawbones_reading::Trust;

fn buses() -> BusFacts {
    BusFacts::new([fact("p", Mastership::BenchOnly, Trust::OperatorAttested)]).unwrap()
}

fn map(width: u8, class: HazardClass, range: Option<ValueRange>) -> WriteMap {
    WriteMap::new([WriteEntry {
        width,
        range,
        ..entry(
            "p",
            "s",
            class,
            SettingKind::Config,
            false,
            Trust::OperatorAttested,
        )
    }])
    .unwrap()
}

const ONE: [WriteValue; 3] = [
    WriteValue::Bool(true),
    WriteValue::U16(7),
    WriteValue::I16(-7),
];
const TWO: [WriteValue; 2] = [WriteValue::U32(70_000), WriteValue::I32(-70_000)];

fn refused_width(map: &WriteMap, value: WriteValue, expected: u8, got: u8) {
    let d = classify(&write("p", "s", value), map, &buses());
    assert_eq!(
        d.requirement(),
        &Requirement::Refused {
            reason: RefusalReason::WidthMismatch { expected, got }
        },
        "{value:?}"
    );
    assert_eq!(d.class(), HazardClass::Class4, "{value:?}");
}

#[test]
fn regression_214_two_register_value_into_one_register_signal_is_refused() {
    // With no range (the case #212 left open) and with one, at every class
    // that transmits: the neighbouring register is never reached.
    for class in [
        HazardClass::Class0,
        HazardClass::Class2,
        HazardClass::Class3,
    ] {
        for range in [
            None,
            Some(ValueRange::new(WriteValue::U16(0), WriteValue::U16(9)).unwrap()),
        ] {
            let m = map(1, class, range);
            for v in TWO {
                refused_width(&m, v, 1, 2);
            }
        }
    }
}

#[test]
fn regression_214_one_register_value_into_two_register_signal_is_refused() {
    for class in [
        HazardClass::Class0,
        HazardClass::Class2,
        HazardClass::Class3,
    ] {
        let m = map(2, class, None);
        for v in ONE {
            refused_width(&m, v, 2, 1);
        }
    }
}

#[test]
fn regression_214_width_is_checked_before_range() {
    // A U32 into a one-register signal limited by a U16 range says "wrong
    // width", not "out of range": the refusal names the real problem.
    let range = Some(ValueRange::new(WriteValue::U16(0), WriteValue::U16(9)).unwrap());
    refused_width(
        &map(1, HazardClass::Class2, range),
        WriteValue::U32(5),
        1,
        2,
    );
}

#[test]
fn regression_214_a_value_of_the_entry_s_width_is_classified_as_before() {
    for (width, values) in [(1u8, &ONE[..]), (2u8, &TWO[..])] {
        for v in values {
            let d = classify(
                &write("p", "s", *v),
                &map(width, HazardClass::Class3, None),
                &buses(),
            );
            let Requirement::RailBLease { action, .. } = d.requirement() else {
                panic!("not a lease: {d:?}");
            };
            assert_eq!(action.quantity(), u16::from(width), "{v:?}");
            // The digest is unchanged: it is the one the value-derived
            // action has (the width is not hashed, the count already is).
            assert_eq!(
                *action,
                bus_action(bench_port_of("p"), 1, register_of("s"), *v).unwrap()
            );
            assert_eq!(
                Some(*action),
                bus_action_for_width(bench_port_of("p"), 1, register_of("s"), width, *v)
            );
        }
    }
}

#[test]
fn regression_214_the_action_for_a_width_refuses_any_other_value() {
    for v in TWO {
        assert_eq!(bus_action_for_width(0, 1, 0x10, 1, v), None);
    }
    for v in ONE {
        assert_eq!(bus_action_for_width(0, 1, 0x10, 2, v), None);
    }
}

#[test]
fn regression_214_a_width_must_be_one_or_two() {
    for width in [0u8, 3, 4, 255] {
        let e = WriteEntry {
            width,
            ..entry(
                "p",
                "s",
                HazardClass::Class2,
                SettingKind::Config,
                false,
                Trust::Measured,
            )
        };
        assert_eq!(
            WriteMap::new([e]),
            Err(FactError::InvalidWidth {
                port: port("p"),
                width,
                signal: signal("s"),
            })
        );
    }
}

#[test]
fn regression_214_width_survives_a_json_round_trip() {
    let m = map(2, HazardClass::Class2, None);
    let json = serde_json::to_string(&m).unwrap();
    assert!(json.contains("\"width\":2"), "{json}");
    assert_eq!(serde_json::from_str::<WriteMap>(&json).unwrap(), m);
    // A width of 3 does not load.
    let bad = json.replace("\"width\":2", "\"width\":3");
    assert!(serde_json::from_str::<WriteMap>(&bad).is_err());
}
