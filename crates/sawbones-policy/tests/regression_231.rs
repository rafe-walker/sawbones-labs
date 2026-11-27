//! #231: tests for mutants the weekly mutation run found missed. Each test
//! fails with the named mutant applied.

mod common;

use common::*;
use sawbones_policy::{
    ActionDigest, BusFacts, FactKey, Mastership, ScopeLabel, ScopeLabelError, ScopeName,
    ValueRange, WriteValue, MAX_SCOPE_LABEL_BYTES,
};
use sawbones_reading::Trust;
use WriteValue::{Bool, I16, I32, U16, U32};

fn range(min: WriteValue, max: WriteValue) -> ValueRange {
    ValueRange::new(min, max).unwrap()
}

/// Mutants: every `delete match arm`, `&&` with `||` and `<=` with `>` in
/// `ValueRange::contains`. Both bounds are inclusive, for every value type,
/// a value either side is outside, and a value of another type is outside.
#[test]
fn regression_231_policy_range_contains_is_inclusive_for_every_type() {
    let cases: [(WriteValue, WriteValue, [WriteValue; 3], [WriteValue; 2]); 4] = [
        (
            U16(10),
            U16(20),
            [U16(10), U16(15), U16(20)],
            [U16(9), U16(21)],
        ),
        (
            I16(-5),
            I16(5),
            [I16(-5), I16(0), I16(5)],
            [I16(-6), I16(6)],
        ),
        (
            U32(100),
            U32(200),
            [U32(100), U32(150), U32(200)],
            [U32(99), U32(201)],
        ),
        (
            I32(-70_000),
            I32(70_000),
            [I32(-70_000), I32(0), I32(70_000)],
            [I32(-70_001), I32(70_001)],
        ),
    ];
    for (min, max, inside, outside) in cases {
        let r = range(min, max);
        for v in inside {
            assert!(r.contains(v), "{min:?}..={max:?} should hold {v:?}");
        }
        for v in outside {
            assert!(!r.contains(v), "{min:?}..={max:?} should not hold {v:?}");
        }
    }
    // Bool: false < true.
    assert!(range(Bool(false), Bool(true)).contains(Bool(false)));
    assert!(range(Bool(false), Bool(true)).contains(Bool(true)));
    assert!(range(Bool(false), Bool(false)).contains(Bool(false)));
    assert!(!range(Bool(false), Bool(false)).contains(Bool(true)));
    assert!(range(Bool(true), Bool(true)).contains(Bool(true)));
    assert!(!range(Bool(true), Bool(true)).contains(Bool(false)));
    // A single-value range, and a value of another type.
    assert!(range(U16(7), U16(7)).contains(U16(7)));
    assert!(!range(U16(7), U16(7)).contains(U16(8)));
    assert!(!range(U16(0), U16(100)).contains(I16(5)));
    assert!(!range(U16(0), U16(100)).contains(U32(5)));
    assert!(!range(I32(0), I32(100)).contains(U32(5)));
    assert!(!range(Bool(false), Bool(true)).contains(U16(1)));
}

/// Mutants: `BusFacts::facts -> empty()` and `From<BusFacts> for Vec -> default`.
#[test]
fn regression_231_policy_bus_facts_list_their_facts() {
    let a = fact("rs485-1", Mastership::BenchOnly, Trust::OperatorAttested);
    let b = fact("rs485-2", Mastership::OtherMasterActive, Trust::Measured);
    let facts = BusFacts::new([b.clone(), a.clone()]).unwrap();
    let listed: Vec<_> = facts.facts().cloned().collect();
    assert_eq!(listed, vec![a.clone(), b.clone()], "ordered by port");
    let vec: Vec<_> = facts.into();
    assert_eq!(vec, vec![a, b]);
    assert_eq!(BusFacts::default().facts().count(), 0);
}

/// Mutants: `as_str` returning `""` or `"xyzzy"`, and `Display` writing
/// nothing, on `ScopeName`, `ScopeLabel`, `PortId` and `SignalId`.
#[test]
fn regression_231_policy_identifiers_show_what_they_hold() {
    let name = ScopeName::parse("inverter-a").unwrap();
    assert_eq!(name.as_str(), "inverter-a");
    assert_eq!(name.to_string(), "inverter-a");
    let label = ScopeLabel::parse("V1.2.3+b7").unwrap();
    assert_eq!(label.as_str(), "V1.2.3+b7");
    assert_eq!(label.to_string(), "V1.2.3+b7");
    assert_eq!(port("rs485-1").to_string(), "rs485-1");
    assert_eq!(signal("charge_voltage").to_string(), "charge_voltage");
}

/// Mutant: `>` with `>=` in `validate_scope_label`. A label of exactly the
/// maximum length is valid; one byte more is too long.
#[test]
fn regression_231_policy_scope_label_length_limit_is_inclusive() {
    let at_max = "A".repeat(MAX_SCOPE_LABEL_BYTES);
    assert!(ScopeLabel::parse(&at_max).is_ok());
    let over = "A".repeat(MAX_SCOPE_LABEL_BYTES + 1);
    assert_eq!(ScopeLabel::parse(&over), Err(ScopeLabelError::TooLong));
}

/// Mutant: `Display for FactKey` writing nothing.
#[test]
fn regression_231_policy_fact_key_display_names_the_fact() {
    let write = FactKey::WriteEntry {
        port: port("rs485-1"),
        unit: 3,
        signal: signal("charge_voltage"),
    };
    assert_eq!(
        write.to_string(),
        "write entry charge_voltage of unit 3 on rs485-1"
    );
    let bus = FactKey::Mastership {
        port: port("rs485-1"),
    };
    assert_eq!(bus.to_string(), "mastership of rs485-1");
}

/// Mutants: `Debug` and `Display` for `ActionDigest` writing nothing.
#[test]
fn regression_231_policy_action_digest_formats_as_hex() {
    let request = write("rs485-1", "charge_voltage", WriteValue::U16(5600));
    let digest = ActionDigest::of(&expected_action(&request));
    let hex: String = digest
        .as_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    assert_eq!(hex.len(), 64);
    assert_eq!(digest.to_string(), hex);
    assert_eq!(format!("{digest:?}"), format!("ActionDigest({hex})"));
    assert_eq!(digest.to_hex(), hex);
}
