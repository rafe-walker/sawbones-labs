//! Frame builders and trust-root helpers shared by the tests.
#![allow(dead_code)]

use sawbones_busguard::{
    crc16_modbus, parse, AllowList, BusWriteAction, Function, LeaseGrant, Port, Rule,
    TrustRootEvent, Update, WriteFunction,
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

/// The action `frame` is the write of, on `port`, if it is a well-formed
/// write the shared encoder can express.
pub fn action_for(port: u8, frame: &[u8]) -> Option<BusWriteAction> {
    let parsed = parse(frame).ok()?;
    let function = match parsed.function() {
        Function::WriteSingleRegister => WriteFunction::WriteSingleRegister,
        Function::WriteMultipleRegisters => WriteFunction::WriteMultipleRegisters,
        _ => return None,
    };
    let values: Vec<u16> = parsed
        .payload()
        .chunks_exact(2)
        .map(|pair| u16::from_be_bytes([pair[0], pair[1]]))
        .collect();
    let action =
        BusWriteAction::new(port, parsed.unit(), function, parsed.start(), &values).ok()?;
    // The encoder must give back exactly the frame it was read from.
    assert_eq!(action.frame().as_bytes(), frame);
    Some(action)
}

/// The trust root grants a lease for the action `frame` encodes, on `port`.
pub fn grant_action(action: BusWriteAction) -> TrustRootEvent {
    TrustRootEvent::unverified_from_trust_root(Update::GrantLease(LeaseGrant::for_action(action)))
}

/// The trust root grants a lease for exactly `frame` on `port`. Panics if
/// `frame` is not a write an action can express.
pub fn grant(port: u8, frame: &[u8]) -> TrustRootEvent {
    try_grant(port, frame).unwrap_or_else(|| panic!("no action is the frame {frame:02X?}"))
}

/// As [`grant`], or `None` if no action encodes to `frame`.
pub fn try_grant(port: u8, frame: &[u8]) -> Option<TrustRootEvent> {
    action_for(port, frame).map(grant_action)
}

/// A frame sink that records what it sent, for the transmit tests. Clones
/// share one record, so a test keeps one clone and hands the other to the
/// line.
#[derive(Debug, Default, Clone)]
pub struct Wire {
    pub sent: std::rc::Rc<std::cell::RefCell<Vec<Vec<u8>>>>,
}

impl Wire {
    /// Every frame sent so far.
    pub fn frames(&self) -> Vec<Vec<u8>> {
        self.sent.borrow().clone()
    }
}

impl sawbones_busguard::FrameSink for Wire {
    type Error = core::convert::Infallible;
    fn send_frame(&mut self, bytes: &[u8]) -> Result<(), Self::Error> {
        self.sent.borrow_mut().push(bytes.to_vec());
        Ok(())
    }
}

/// `Port(port)`, for brevity.
pub const fn port(port: u8) -> Port {
    Port(port)
}
