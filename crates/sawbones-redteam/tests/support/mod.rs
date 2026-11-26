//! Frame builders shared by the bus attack tests.
#![allow(dead_code)]

use sawbones_busguard::{
    crc16_modbus, parse, AllowList, BusGuard, BusWriteAction, Function, LeaseGrant, Port, Rule,
    TrustRootEvent, Update, WriteFunction,
};

pub fn with_crc(body: &[u8]) -> Vec<u8> {
    let mut frame = body.to_vec();
    frame.extend_from_slice(&crc16_modbus(body).to_le_bytes());
    frame
}

pub fn read(unit: u8, function: u8, start: u16, quantity: u16) -> Vec<u8> {
    let [s0, s1] = start.to_be_bytes();
    let [q0, q1] = quantity.to_be_bytes();
    with_crc(&[unit, function, s0, s1, q0, q1])
}

pub fn write_single(unit: u8, register: u16, value: u16) -> Vec<u8> {
    let [r0, r1] = register.to_be_bytes();
    let [v0, v1] = value.to_be_bytes();
    with_crc(&[unit, 0x06, r0, r1, v0, v1])
}

pub fn write_multiple(unit: u8, start: u16, values: &[u16]) -> Vec<u8> {
    let mut body = vec![unit, 0x10];
    body.extend_from_slice(&start.to_be_bytes());
    body.extend_from_slice(&u16::try_from(values.len()).unwrap().to_be_bytes());
    body.push(u8::try_from(values.len() * 2).unwrap());
    for v in values {
        body.extend_from_slice(&v.to_be_bytes());
    }
    with_crc(&body)
}

pub fn rule(unit: u8, function: Function, first: u16, last: u16, max: u16) -> Rule {
    Rule {
        port: Port(0),
        unit,
        function,
        first,
        last,
        max_quantity: max,
    }
}

pub fn list(rules: &[Rule]) -> AllowList {
    let mut l = AllowList::new();
    for &r in rules {
        l.try_push(r).unwrap();
    }
    l
}

/// A guard holding `rules`, with no lease.
pub fn guard(rules: &[Rule]) -> BusGuard {
    let mut g = BusGuard::new();
    g.apply(
        TrustRootEvent::unverified_from_trust_root(Update::ReplaceAllowList(list(rules))),
        0,
    )
    .unwrap();
    g
}

/// The action `frame` is the write of on port 0, if the shared encoder can
/// express it (#203).
pub fn action_for(frame: &[u8]) -> Option<BusWriteAction> {
    let parsed = parse(frame).ok()?;
    let function = match parsed.function() {
        Function::WriteSingleRegister => WriteFunction::WriteSingleRegister,
        Function::WriteMultipleRegisters => WriteFunction::WriteMultipleRegisters,
        _ => return None,
    };
    let values: Vec<u16> = parsed
        .payload()
        .chunks_exact(2)
        .map(|p| u16::from_be_bytes([p[0], p[1]]))
        .collect();
    let action = BusWriteAction::new(0, parsed.unit(), function, parsed.start(), &values).ok()?;
    (action.frame().as_bytes() == frame).then_some(action)
}

/// Grants a lease on port 0 at tick `now` for the action `frame` encodes.
/// No lease can name a frame the encoder cannot produce (a broadcast, a
/// malformed or oversized frame, a read), so for those the guard gets a live
/// lease for an unrelated write instead: the attack then runs against a
/// guard holding a lease, as before #203.
pub fn lease(g: &mut BusGuard, frame: &[u8], now: u64) {
    let action = action_for(frame).unwrap_or_else(|| {
        BusWriteAction::new(0, 1, WriteFunction::WriteSingleRegister, 0xFFF0, &[0xBEEF]).unwrap()
    });
    g.apply(
        TrustRootEvent::unverified_from_trust_root(Update::GrantLease(LeaseGrant::for_action(
            action,
        ))),
        now,
    )
    .unwrap();
}
