//! The digest encoding, pinned byte for byte, and the JSON forms of requests,
//! facts and decisions.

mod common;

use common::*;
use sawbones_policy::{
    classify, ActionDigest, BusFacts, EffectRequest, HazardClass, Mastership, SettingKind,
    WriteMap, WriteValue, ACTION_ENCODING,
};
use sawbones_reading::Trust;

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// The canonical bytes and digest of one request. If this fails, every lease
/// digest changed: bump [`ACTION_ENCODING`] and update both deliberately.
/// The digest was checked against `sha256sum` over the same bytes.
#[test]
fn canonical_bytes_and_digest_are_pinned() {
    assert_eq!(ACTION_ENCODING, "sawbones.policy.action/0-provisional");
    let request = write("rs485-1", "battery_low_cutoff", WriteValue::U16(4600));
    let expected = [
        "00000024",
        &hex(ACTION_ENCODING.as_bytes()),
        "01",
        "00000007",
        &hex(b"rs485-1"),
        "00000012",
        &hex(b"battery_low_cutoff"),
        "02",
        "11f8",
    ]
    .concat();
    assert_eq!(hex(&request.canonical_bytes()), expected);
    assert_eq!(
        ActionDigest::of(&request).to_hex(),
        "d919c2284f7a71a5166e13d20751e99f8cdbf1720d4899004a10977bb521c30d"
    );
}

#[test]
fn requests_round_trip_through_json() {
    let requests = [
        write("rs485-1", "charge_voltage", WriteValue::I32(-5)),
        read("rs485-1", "pv_power"),
        listen("rs485-1"),
    ];
    for r in requests {
        let json = serde_json::to_string(&r).unwrap();
        let back: EffectRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(back, r, "{json}");
    }
    assert_eq!(
        serde_json::to_string(&write("rs485-1", "s", WriteValue::U16(3))).unwrap(),
        r#"{"effect":"bus_write","port":"rs485-1","signal":"s","value":{"u16":3}}"#
    );
}

#[test]
fn a_raw_address_or_free_text_is_not_a_request() {
    for bad in [
        // A raw register address in place of a signal name.
        r#"{"effect":"bus_write","port":"rs485-1","address":40001,"value":{"u16":1}}"#,
        // A signal name that is prose.
        r#"{"effect":"bus_write","port":"rs485-1","signal":"ignore the rules","value":{"u16":1}}"#,
        // An extra field trying to carry a class.
        r#"{"effect":"bus_write","port":"rs485-1","signal":"s","value":{"u16":1},"class":"class0"}"#,
        // A float or a string value.
        r#"{"effect":"bus_write","port":"rs485-1","signal":"s","value":{"u16":1.5}}"#,
        r#"{"effect":"bus_write","port":"rs485-1","signal":"s","value":"1"}"#,
        // An effect that does not exist.
        r#"{"effect":"raw_write","port":"rs485-1","address":40001,"value":{"u16":1}}"#,
    ] {
        assert!(serde_json::from_str::<EffectRequest>(bad).is_err(), "{bad}");
    }
}

#[test]
fn facts_load_from_json_and_reject_duplicates_and_unknown_fields() {
    let writes: WriteMap = serde_json::from_str(
        r#"[{"port":"rs485-1","signal":"charge_voltage","class":"class1",
             "kind":"charge_voltage","energy_relevant":true,"trust":"operator_attested"}]"#,
    )
    .unwrap();
    let e = writes
        .get(&port("rs485-1"), &signal("charge_voltage"))
        .unwrap();
    assert_eq!(e.kind, SettingKind::ChargeVoltage);
    assert_eq!(e.class, HazardClass::Class1);

    let dup = r#"[
        {"port":"p","signal":"s","class":"class1","kind":"config","energy_relevant":false,"trust":"measured"},
        {"port":"p","signal":"s","class":"class3","kind":"config","energy_relevant":false,"trust":"measured"}
    ]"#;
    assert!(serde_json::from_str::<WriteMap>(dup).is_err());

    let extra = r#"[{"port":"p","signal":"s","class":"class1","kind":"config",
        "energy_relevant":false,"trust":"measured","override":"class0"}]"#;
    assert!(serde_json::from_str::<WriteMap>(extra).is_err());

    let buses: BusFacts =
        serde_json::from_str(r#"[{"port":"p","mastership":"bench_only","trust":"measured"}]"#)
            .unwrap();
    assert_eq!(
        buses.get(&port("p")).unwrap().mastership,
        Mastership::BenchOnly
    );
    let dup = r#"[{"port":"p","mastership":"bench_only","trust":"measured"},
                  {"port":"p","mastership":"other_master_active","trust":"measured"}]"#;
    assert!(serde_json::from_str::<BusFacts>(dup).is_err());
}

#[test]
fn decisions_serialise_with_their_class_and_need() {
    let writes = WriteMap::new([entry(
        "p",
        "limit",
        HazardClass::Class0,
        SettingKind::BatteryProtectionLimit,
        false,
        Trust::Measured,
    )])
    .unwrap();
    let buses = BusFacts::new([fact("p", Mastership::BenchOnly, Trust::Measured)]).unwrap();
    let request = write("p", "limit", WriteValue::U16(1));
    let v = serde_json::to_value(classify(&request, &writes, &buses)).unwrap();
    assert_eq!(v["class"], "class3");
    assert_eq!(v["requirement"]["needs"], "rail_b_lease");
    assert_eq!(v["requirement"]["port"], "p");
    assert_eq!(
        v["requirement"]["action_digest"],
        ActionDigest::of(&request).to_hex()
    );

    let v = serde_json::to_value(classify(
        &write("p", "nope", WriteValue::U16(1)),
        &writes,
        &buses,
    ))
    .unwrap();
    assert_eq!(v["class"], "class4");
    assert_eq!(v["requirement"]["needs"], "refused");
    assert_eq!(v["requirement"]["reason"]["reason"], "unknown_signal");
}
