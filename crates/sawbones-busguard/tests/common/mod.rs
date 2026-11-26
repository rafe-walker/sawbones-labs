//! Frame builders and trust-root helpers shared by the tests.
#![allow(dead_code)]

use sawbones_busguard::{
    crc16_modbus, AllowList, FrameDigest, Function, LeaseGrant, Port, Rule, TrustRootEvent, Update,
};

/// `body` with its CRC-16/MODBUS appended, low byte first.
pub fn with_crc(body: &[u8]) -> Vec<u8> {
    let mut frame = body.to_vec();
    frame.extend_from_slice(&crc16_modbus(body).to_le_bytes());
    frame
}

/// A read request, 0x03 or 0x04.
pub fn read(unit: u8, function: u8, start: u16, quantity: u16) -> Vec<u8> {
    let [s0, s1] = start.to_be_bytes();
    let [q0, q1] = quantity.to_be_bytes();
    with_crc(&[unit, function, s0, s1, q0, q1])
}

/// A write single register request, 0x06.
pub fn write_single(unit: u8, register: u16, value: u16) -> Vec<u8> {
    let [r0, r1] = register.to_be_bytes();
    let [v0, v1] = value.to_be_bytes();
    with_crc(&[unit, 0x06, r0, r1, v0, v1])
}

/// A write multiple registers request, 0x10.
pub fn write_multiple(unit: u8, start: u16, values: &[u16]) -> Vec<u8> {
    let quantity = u16::try_from(values.len()).unwrap();
    let mut body = vec![unit, 0x10];
    body.extend_from_slice(&start.to_be_bytes());
    body.extend_from_slice(&quantity.to_be_bytes());
    body.push(u8::try_from(values.len() * 2).unwrap());
    for value in values {
        body.extend_from_slice(&value.to_be_bytes());
    }
    with_crc(&body)
}

/// A rule, for brevity.
pub fn rule(port: u8, unit: u8, function: Function, first: u16, last: u16, max: u16) -> Rule {
    Rule {
        port: Port(port),
        unit,
        function,
        first,
        last,
        max_quantity: max,
    }
}

/// An allow-list holding `rules`.
pub fn allow_list(rules: &[Rule]) -> AllowList {
    let mut list = AllowList::new();
    for &rule in rules {
        list.try_push(rule).unwrap();
    }
    list
}

/// The trust root replaces the allow-list.
pub fn replace(rules: &[Rule]) -> TrustRootEvent {
    TrustRootEvent::unverified_from_trust_root(Update::ReplaceAllowList(allow_list(rules)))
}

/// The trust root grants a lease for exactly `frame` on `port`.
pub fn grant(port: u8, frame: &[u8]) -> TrustRootEvent {
    TrustRootEvent::unverified_from_trust_root(Update::GrantLease(LeaseGrant {
        port: Port(port),
        digest: FrameDigest::of(frame),
    }))
}
