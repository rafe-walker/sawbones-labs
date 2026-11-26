//! Effect requests: what a plugin asks the host to do, as typed data.

use std::fmt;

use sawbones_reading::Ident;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Domain label at the start of every [`EffectRequest::canonical_bytes`]
/// encoding. It says "provisional" on purpose: a change to the encoding
/// must bump this label. Version 1 added the unit address (#203).
pub const REQUEST_ENCODING: &str = "sawbones.policy.request/1-provisional";

/// Why a string was rejected as a policy identifier ([`PortId`] or
/// [`SignalId`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum PolicyIdentError {
    /// The string was empty.
    #[error("policy identifier is empty")]
    Empty,
    /// The string was longer than [`MAX_POLICY_IDENT_BYTES`].
    #[error("policy identifier is longer than {MAX_POLICY_IDENT_BYTES} bytes")]
    TooLong,
    /// The string did not start with a lower-case ASCII letter.
    #[error("policy identifier must start with a lower-case ASCII letter")]
    BadStart,
    /// The string held a character outside `a-z 0-9 _ -`.
    #[error("policy identifier contains a character outside a-z 0-9 _ -")]
    BadCharacter,
}

/// Longest policy identifier, in bytes.
pub const MAX_POLICY_IDENT_BYTES: usize = 64;

/// The one rule every policy identifier passes: 1 to
/// [`MAX_POLICY_IDENT_BYTES`] bytes, a lower-case ASCII letter first, then
/// only `a-z 0-9 _ -`. No spaces, punctuation, upper case, `/`, `.` or
/// non-ASCII text, so an identifier cannot carry a sentence, markup or a
/// control character.
///
/// The rule limits shape, not meaning: a string like `ignore_previous_rules`
/// is a legal identifier. That is safe because the class depends on an
/// identifier only through an exact lookup in the write map, so an
/// identifier the write map does not list is refused as unknown.
fn validate_policy_ident(s: &str) -> Result<(), PolicyIdentError> {
    let bytes = s.as_bytes();
    let Some(first) = bytes.first() else {
        return Err(PolicyIdentError::Empty);
    };
    if bytes.len() > MAX_POLICY_IDENT_BYTES {
        return Err(PolicyIdentError::TooLong);
    }
    if !first.is_ascii_lowercase() {
        return Err(PolicyIdentError::BadStart);
    }
    if !bytes
        .iter()
        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'_' | b'-'))
    {
        return Err(PolicyIdentError::BadCharacter);
    }
    Ok(())
}

/// A bus port or a directly attached device the bench talks to, for example
/// `rs485-1`.
///
/// It is built only from a string that passes the policy identifier rule
/// (see [`PortId::parse`]), so it is a short label with no free text in it.
/// There is no conversion from [`sawbones_reading::WorldText`], and even
/// text copied out of one through `as_untrusted_str` must pass the rule:
///
/// ```compile_fail
/// use sawbones_policy::PortId;
/// use sawbones_reading::WorldText;
/// let port = PortId::new(WorldText::new("rs485-1", 64)); // error: expected `Ident`
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct PortId(String);

impl PortId {
    /// Wraps an identifier as a port id, applying the policy identifier
    /// rule on top of the [`Ident`] rule.
    pub fn new(id: Ident) -> Result<Self, PolicyIdentError> {
        Self::parse(id.as_str())
    }

    /// Validates a string against the policy identifier rule: 1 to 64
    /// bytes, a lower-case ASCII letter first, then only `a-z 0-9 _ -`.
    pub fn parse(s: &str) -> Result<Self, PolicyIdentError> {
        validate_policy_ident(s)?;
        Ok(Self(s.to_owned()))
    }

    /// The port id as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PortId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl TryFrom<String> for PortId {
    type Error = PolicyIdentError;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        validate_policy_ident(&s)?;
        Ok(Self(s))
    }
}

impl From<PortId> for String {
    fn from(p: PortId) -> Self {
        p.0
    }
}

/// A named signal on a port: a register, coil or setting as the write map
/// names it, for example `battery_low_cutoff`. Never a raw address.
///
/// Like [`PortId`], it is built only from a string that passes the policy
/// identifier rule and cannot be built from [`sawbones_reading::WorldText`]:
///
/// ```compile_fail
/// use sawbones_policy::SignalId;
/// use sawbones_reading::WorldText;
/// let signal = SignalId::new(WorldText::new("charge_voltage", 64)); // error
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct SignalId(String);

impl SignalId {
    /// Wraps an identifier as a signal name, applying the policy identifier
    /// rule on top of the [`Ident`] rule.
    pub fn new(id: Ident) -> Result<Self, PolicyIdentError> {
        Self::parse(id.as_str())
    }

    /// Validates a string against the policy identifier rule: 1 to 64
    /// bytes, a lower-case ASCII letter first, then only `a-z 0-9 _ -`.
    pub fn parse(s: &str) -> Result<Self, PolicyIdentError> {
        validate_policy_ident(s)?;
        Ok(Self(s.to_owned()))
    }

    /// The signal name as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SignalId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl TryFrom<String> for SignalId {
    type Error = PolicyIdentError;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        validate_policy_ident(&s)?;
        Ok(Self(s))
    }
}

impl From<SignalId> for String {
    fn from(p: SignalId) -> Self {
        p.0
    }
}

/// The value of a bus write, as a typed number. There is no string variant
/// and no float: a value is something the digest can encode exactly.
///
/// The value plays no part in the class. Whether a value is in range for its
/// signal is the write-map entry's [`crate::ValueRange`], checked by
/// [`crate::classify`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WriteValue {
    /// A coil or flag.
    Bool(bool),
    /// One unsigned 16-bit register.
    U16(u16),
    /// One signed 16-bit register.
    I16(i16),
    /// An unsigned 32-bit value (two registers).
    U32(u32),
    /// A signed 32-bit value (two registers).
    I32(i32),
}

impl WriteValue {
    /// How many 16-bit registers this value would occupy on the wire. This
    /// is the width of the value, not of any signal: the signal's width is
    /// the write-map entry's (#214), and a write is refused if they differ.
    pub const fn width(self) -> u8 {
        match self {
            WriteValue::Bool(_) | WriteValue::U16(_) | WriteValue::I16(_) => 1,
            WriteValue::U32(_) | WriteValue::I32(_) => 2,
        }
    }
}

/// Write one value to one named signal of one unit on one port.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BusWrite {
    /// The port the signal is on.
    pub port: PortId,
    /// The Modbus unit (slave) address of the device, so two paralleled
    /// devices on one bus are never confused (#203). Only a write-map entry
    /// for this unit can authorise the write.
    pub unit: u8,
    /// The signal, as the write map names it.
    pub signal: SignalId,
    /// The value to write.
    pub value: WriteValue,
}

/// Read one named signal by sending a request on the bus, as a Modbus read
/// does. This transmits.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BusRead {
    /// The port to send the read on.
    pub port: PortId,
    /// The Modbus unit (slave) address of the device read.
    pub unit: u8,
    /// The signal to read.
    pub signal: SignalId,
}

/// Listen to a port without transmitting anything.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BusListen {
    /// The port to listen on.
    pub port: PortId,
}

/// Something a plugin asks the host to do. Plugins never expose tools of
/// their own; they emit one of these and the policy engine classifies it.
///
/// Every field is a typed value. There is no variant that writes a raw
/// address, and nothing here holds [`sawbones_reading::WorldText`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "effect", rename_all = "snake_case")]
pub enum EffectRequest {
    /// Write a named signal.
    BusWrite(BusWrite),
    /// Read a named signal, transmitting a request.
    BusRead(BusRead),
    /// Listen passively.
    BusListen(BusListen),
}

impl EffectRequest {
    /// The port this request acts on.
    pub fn port(&self) -> &PortId {
        match self {
            EffectRequest::BusWrite(w) => &w.port,
            EffectRequest::BusRead(r) => &r.port,
            EffectRequest::BusListen(l) => &l.port,
        }
    }

    /// The stable binary encoding of this exact request, for records of
    /// what a plugin asked for.
    ///
    /// A lease is not bound to this: it is bound to the
    /// [`crate::ActionDigest`] of the bus action the request resolves to
    /// through the write map, which the bus guard can check against the wire
    /// (#203).
    ///
    /// The layout, with every integer big-endian:
    ///
    /// - [`REQUEST_ENCODING`] as a length-prefixed string;
    /// - one tag byte: `1` bus write, `2` bus read, `3` bus listen;
    /// - each field in declaration order. A string is a `u32` byte length and
    ///   then its bytes; a unit is one byte. A [`WriteValue`] is one tag byte
    ///   (`1` bool, `2` u16, `3` i16, `4` u32, `5` i32) and then its
    ///   fixed-width bytes (a bool is one byte, `0` or `1`).
    ///
    /// The length prefixes make the encoding injective: two different
    /// requests never encode to the same bytes. The golden test in this crate
    /// pins the exact bytes; changing them is a schema change and must bump
    /// [`REQUEST_ENCODING`].
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(96);
        put_str(&mut out, REQUEST_ENCODING);
        match self {
            EffectRequest::BusWrite(w) => {
                out.push(1);
                put_str(&mut out, w.port.as_str());
                out.push(w.unit);
                put_str(&mut out, w.signal.as_str());
                put_value(&mut out, w.value);
            }
            EffectRequest::BusRead(r) => {
                out.push(2);
                put_str(&mut out, r.port.as_str());
                out.push(r.unit);
                put_str(&mut out, r.signal.as_str());
            }
            EffectRequest::BusListen(l) => {
                out.push(3);
                put_str(&mut out, l.port.as_str());
            }
        }
        out
    }
}

fn put_str(out: &mut Vec<u8>, s: &str) {
    // Identifiers are at most 128 bytes and the label is a constant, so the
    // length always fits a u32; saturate rather than panic all the same.
    let len = u32::try_from(s.len()).unwrap_or(u32::MAX);
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(s.as_bytes());
}

fn put_value(out: &mut Vec<u8>, v: WriteValue) {
    match v {
        WriteValue::Bool(b) => {
            out.push(1);
            out.push(u8::from(b));
        }
        WriteValue::U16(n) => {
            out.push(2);
            out.extend_from_slice(&n.to_be_bytes());
        }
        WriteValue::I16(n) => {
            out.push(3);
            out.extend_from_slice(&n.to_be_bytes());
        }
        WriteValue::U32(n) => {
            out.push(4);
            out.extend_from_slice(&n.to_be_bytes());
        }
        WriteValue::I32(n) => {
            out.push(5);
            out.extend_from_slice(&n.to_be_bytes());
        }
    }
}
