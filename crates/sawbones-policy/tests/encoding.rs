//! The digest encoding, pinned byte for byte, and the JSON forms of requests,
//! facts and decisions.

mod common;

use common::*;
use sawbones_policy::{
    classify, ActionDigest, BusFacts, EffectRequest, HazardClass, Mastership, SettingKind,
    WriteMap, WriteValue, REQUEST_ENCODING,
};
use sawbones_reading::Trust;

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// The canonical bytes of one request, unit included (#203), and the digest
/// a class 3 lease for it is bound to: since #203 that is the digest of the
/// bus action the request resolves to, not of these bytes. If either fails,
/// bump [`REQUEST_ENCODING`] or the encoder's domain and update it
/// deliberately. The digest was checked against Python's hashlib over the
/// action's domain label and encoding.
#[test]
fn canonical_bytes_and_digest_are_pinned() {
    assert_eq!(REQUEST_ENCODING, "sawbones.policy.request/1-provisional");
    let request = write("rs485-1", "battery_low_cutoff", WriteValue::U16(4600));
    let expected = [
        "00000025",
        &hex(REQUEST_ENCODING.as_bytes()),
        "01",
        "00000007",
        &hex(b"rs485-1"),
        "01",
        "00000012",
        &hex(b"battery_low_cutoff"),
        "02",
        "11f8",
    ]
    .concat();
    assert_eq!(hex(&request.canonical_bytes()), expected);
    let read = read("rs485-1", "pv_power");
    let expected = [
        "00000025",
        &hex(REQUEST_ENCODING.as_bytes()),
        "02",
        "00000007",
        &hex(b"rs485-1"),
        "01",
        "00000008",
        &hex(b"pv_power"),
    ]
    .concat();
    assert_eq!(hex(&read.canonical_bytes()), expected);

    // The lease digest: write 4600 to the signal's register of unit 1 on the
    // port's bench port, as the test facts wire them.
    let action = expected_action(&request);
    assert_eq!(
        hex(&action.encode()),
        format!(
            "{:02x}0106{:04x}0111f8000000000000",
            bench_port_of("rs485-1"),
            register_of("battery_low_cutoff")
        )
    );
    assert_eq!(ActionDigest::of(&action).to_hex(), PINNED_LEASE_DIGEST);
}

const PINNED_LEASE_DIGEST: &str =
    "6dd15ce857eecfdf29733d8d020fd6d5e3479f3b3f9e0c18f78c8763847bd497";

/// The digest the interlock binds a lease to, and the one the approval and
/// the bus guard carry, are the same 32 bytes.
#[test]
fn the_interlock_digest_is_the_same_bytes() {
    let request = write("rs485-1", "battery_low_cutoff", WriteValue::U16(4600));
    let action = expected_action(&request);
    let digest = ActionDigest::of(&action);
    let for_interlock = sawbones_interlock::ActionDigest::from(digest);
    assert_eq!(&for_interlock.0, digest.as_bytes());
    let for_guard = sawbones_modbus_encode::ActionDigest::from(digest);
    assert_eq!(for_guard, action.digest());
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
        r#"{"effect":"bus_write","port":"rs485-1","unit":1,"signal":"s","value":{"u16":3}}"#
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
        r#"[{"port":"rs485-1","unit":2,"signal":"charge_voltage","register":4096,"width":1,"class":"class1",
             "kind":"charge_voltage","energy_relevant":true,"trust":"operator_attested"}]"#,
    )
    .unwrap();
    assert!(writes
        .get(&port("rs485-1"), 1, &signal("charge_voltage"))
        .is_none());
    let e = writes
        .get(&port("rs485-1"), 2, &signal("charge_voltage"))
        .unwrap();
    assert_eq!(e.register, 4096);
    assert_eq!(e.kind, SettingKind::ChargeVoltage);
    assert_eq!(e.class, HazardClass::Class1);

    let dup = r#"[
        {"port":"p","unit":1,"signal":"s","register":1,"width":1,"class":"class1","kind":"config","energy_relevant":false,"trust":"measured"},
        {"port":"p","unit":1,"signal":"s","register":2,"width":1,"class":"class3","kind":"config","energy_relevant":false,"trust":"measured"}
    ]"#;
    assert!(serde_json::from_str::<WriteMap>(dup).is_err());
    // The same signal of two units is two facts.
    let two_units = r#"[
        {"port":"p","unit":1,"signal":"s","register":1,"width":1,"class":"class1","kind":"config","energy_relevant":false,"trust":"measured"},
        {"port":"p","unit":2,"signal":"s","register":1,"width":1,"class":"class3","kind":"config","energy_relevant":false,"trust":"measured"}
    ]"#;
    assert_eq!(
        serde_json::from_str::<WriteMap>(two_units)
            .unwrap()
            .entries()
            .count(),
        2
    );
    // A missing unit or register is not an entry.
    let no_unit = r#"[{"port":"p","signal":"s","register":1,"width":1,"class":"class1","kind":"config",
        "energy_relevant":false,"trust":"measured"}]"#;
    assert!(serde_json::from_str::<WriteMap>(no_unit).is_err());
    // A missing width is not an entry either (#214): it is never guessed.
    let no_width = r#"[{"port":"p","unit":1,"signal":"s","register":1,"class":"class1",
        "kind":"config","energy_relevant":false,"trust":"measured"}]"#;
    assert!(serde_json::from_str::<WriteMap>(no_width).is_err());
    let no_register = r#"[{"port":"p","unit":1,"signal":"s","class":"class1","kind":"config",
        "energy_relevant":false,"trust":"measured"}]"#;
    assert!(serde_json::from_str::<WriteMap>(no_register).is_err());

    let extra = r#"[{"port":"p","unit":1,"signal":"s","register":1,"width":1,"class":"class1","kind":"config",
        "energy_relevant":false,"trust":"measured","override":"class0"}]"#;
    assert!(serde_json::from_str::<WriteMap>(extra).is_err());

    let buses: BusFacts = serde_json::from_str(
        r#"[{"port":"p","bench_port":0,"mastership":"bench_only","trust":"measured"}]"#,
    )
    .unwrap();
    assert_eq!(
        buses.get(&port("p")).unwrap().mastership,
        Mastership::BenchOnly
    );
    let dup = r#"[{"port":"p","bench_port":0,"mastership":"bench_only","trust":"measured"},
                  {"port":"p","bench_port":1,"mastership":"other_master_active","trust":"measured"}]"#;
    assert!(serde_json::from_str::<BusFacts>(dup).is_err());
    let one_wire = r#"[{"port":"p","bench_port":0,"mastership":"bench_only","trust":"measured"},
                  {"port":"q","bench_port":0,"mastership":"bench_only","trust":"measured"}]"#;
    assert!(serde_json::from_str::<BusFacts>(one_wire).is_err());
    let no_bench_port = r#"[{"port":"p","mastership":"bench_only","trust":"measured"}]"#;
    assert!(serde_json::from_str::<BusFacts>(no_bench_port).is_err());
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
    let action = expected_action(&request);
    assert_eq!(
        v["requirement"]["action_digest"],
        ActionDigest::of(&action).to_hex()
    );
    assert_eq!(
        v["requirement"]["action"],
        serde_json::json!({
            "port": bench_port_of("p"), "unit": 1, "function": "write_single_register",
            "register": register_of("limit"), "values": [1]
        })
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
