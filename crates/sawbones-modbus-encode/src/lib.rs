//! # sawbones-modbus-encode: PROVISIONAL
//!
//! **This crate is provisional. The Sawbones design is still under review, and
//! every type here may change or be replaced without notice.**
//!
//! One typed Modbus bus write, and the one place it is turned into bytes
//! (#203, red-team card bus-014).
//!
//! The policy engine reasons about named signals and values; the bus guard
//! about wire bytes. Before this crate nothing proved that the frame the
//! guard passed encoded the action the policy engine classified: whoever
//! turned the action into a frame was a confused deputy. Now both sides use
//! the same [`BusWriteAction`]:
//!
//! - the **policy engine** resolves a named write through the write map to a
//!   [`BusWriteAction`] (bench port, unit address, function, register,
//!   register values) and binds a class 3 lease to its [`ActionDigest`];
//! - the **interlock** binds the rail B lease to the same 32 bytes;
//! - the **approval** the trust root signs carries the action itself and its
//!   digest;
//! - the **bus guard** checks the digest against the action, re-derives the
//!   frame with [`BusWriteAction::frame`] and passes a write only if the
//!   bytes on the wire are byte-identical to it.
//!
//! So a lease approves one action, and the only frame that can spend it is
//! the frame this crate encodes for that action.
//!
//! ## The encoding
//!
//! [`BusWriteAction::frame`] is a pure, deterministic function from the
//! action to one Modbus RTU request:
//!
//! - **write single register (0x06)**: unit, `0x06`, register (2 bytes),
//!   value (2 bytes), CRC: 8 bytes;
//! - **write multiple registers (0x10)**: unit, `0x10`, register (2 bytes),
//!   quantity (2 bytes), byte count (1 byte, twice the quantity), the values
//!   (2 bytes each), CRC: 9 bytes plus two per register.
//!
//! Every integer is big-endian, as on the wire, and the CRC-16/MODBUS goes
//! low byte first. The golden tests pin examples from the Modbus
//! specification.
//!
//! [`BusWriteAction::encode`] is the action's own canonical encoding,
//! [`ACTION_ENCODED_LEN`] bytes, which the approval message carries and
//! [`ActionDigest`] hashes behind [`ACTION_DOMAIN`]. It is injective: two
//! different actions never encode to the same bytes, and
//! [`BusWriteAction::decode`] refuses any bytes `encode` would not produce.
//!
//! ## What an action can be
//!
//! A [`BusWriteAction`] is built only by [`BusWriteAction::new`] or
//! [`BusWriteAction::decode`], which refuse a broadcast or reserved unit
//! address, no values or more than [`MAX_ACTION_WORDS`], a single write with
//! other than one value, and a range that runs past register 0xFFFF. So every
//! action encodes to a frame the bus guard's parser accepts.
//!
//! The crate reads no clock, allocates nothing and is `no_std`, so the same
//! code runs on the host, in the trust root and in the bus guard.

#![no_std]
#![forbid(unsafe_code)]

use sha2::{Digest, Sha256};

/// The highest individual unit (slave) address. 0 is broadcast and 248 to
/// 255 are reserved.
pub const MAX_UNIT_ADDRESS: u8 = 247;

/// The most register values one action writes. Enough for the widest value
/// the policy engine has (32 bits, two registers) and a little more. The
/// approval message has room for this many.
pub const MAX_ACTION_WORDS: usize = 4;

/// The length of [`BusWriteAction::encode`]: port, unit, function code,
/// register (2), value count, then [`MAX_ACTION_WORDS`] values of 2 bytes.
pub const ACTION_ENCODED_LEN: usize = 6 + 2 * MAX_ACTION_WORDS;

/// The longest frame an action encodes to: a write multiple registers of
/// [`MAX_ACTION_WORDS`] values.
pub const MAX_ACTION_FRAME_LEN: usize = 9 + 2 * MAX_ACTION_WORDS;

/// Hashed before an action's encoding in [`ActionDigest::of`]. A change to
/// the encoding must change this label, so no old digest can name a new
/// action.
pub const ACTION_DOMAIN: &[u8] = b"sawbones-modbus-encode/bus-write-action/v1\0";

/// CRC-16/MODBUS over `bytes`: reflected polynomial 0xA001, initial value
/// 0xFFFF. On the wire the low byte goes first.
#[must_use]
pub const fn crc16_modbus(bytes: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    let mut i = 0;
    while i < bytes.len() {
        crc ^= bytes[i] as u16;
        let mut bit = 0;
        while bit < 8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xA001
            } else {
                crc >> 1
            };
            bit += 1;
        }
        i += 1;
    }
    crc
}

/// The two write functions an action can use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum WriteFunction {
    /// 0x06, write single register: exactly one value.
    WriteSingleRegister,
    /// 0x10, write multiple registers: one to [`MAX_ACTION_WORDS`] values.
    WriteMultipleRegisters,
}

impl WriteFunction {
    /// The code on the wire.
    #[must_use]
    pub const fn code(self) -> u8 {
        match self {
            WriteFunction::WriteSingleRegister => 0x06,
            WriteFunction::WriteMultipleRegisters => 0x10,
        }
    }

    /// The function for a code, or `None` for any code but 0x06 and 0x10.
    #[must_use]
    pub const fn from_code(code: u8) -> Option<WriteFunction> {
        match code {
            0x06 => Some(WriteFunction::WriteSingleRegister),
            0x10 => Some(WriteFunction::WriteMultipleRegisters),
            _ => None,
        }
    }
}

/// Why an action could not be built or decoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActionError {
    /// Unit address 0: a broadcast, which every device on the line would act
    /// on.
    BroadcastUnit,
    /// Unit addresses 248 to 255 are reserved by the protocol.
    ReservedUnit(u8),
    /// No register values.
    NoValues,
    /// More than [`MAX_ACTION_WORDS`] register values.
    TooManyValues,
    /// A write single register with other than exactly one value.
    SingleNeedsOneValue,
    /// The register range runs past address 0xFFFF.
    AddressOverflow,
    /// Decoding: not exactly [`ACTION_ENCODED_LEN`] bytes.
    Length,
    /// Decoding: a function code other than 0x06 or 0x10.
    UnknownFunction(u8),
    /// Decoding: a non-zero byte in an unused value slot.
    NonZeroPadding,
}

/// One write to one unit on one bench port: the exact thing a class 3 lease
/// approves.
///
/// The fields are private and checked by [`BusWriteAction::new`], so every
/// value encodes to one well-formed frame. Not `Default`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BusWriteAction {
    port: u8,
    unit: u8,
    function: WriteFunction,
    register: u16,
    count: u8,
    words: [u16; MAX_ACTION_WORDS],
}

impl BusWriteAction {
    /// An action writing `values` from `register` onwards to `unit`, sent
    /// with `function` on bench port `port`.
    ///
    /// The bench port is the number both the interlock (rail B) and the bus
    /// guard use for the same physical port.
    ///
    /// # Errors
    ///
    /// An [`ActionError`] for a broadcast or reserved unit, no values, more
    /// than [`MAX_ACTION_WORDS`], a single write with other than one value,
    /// or a range past register 0xFFFF.
    pub const fn new(
        port: u8,
        unit: u8,
        function: WriteFunction,
        register: u16,
        values: &[u16],
    ) -> Result<BusWriteAction, ActionError> {
        if unit == 0 {
            return Err(ActionError::BroadcastUnit);
        }
        if unit > MAX_UNIT_ADDRESS {
            return Err(ActionError::ReservedUnit(unit));
        }
        if values.is_empty() {
            return Err(ActionError::NoValues);
        }
        if values.len() > MAX_ACTION_WORDS {
            return Err(ActionError::TooManyValues);
        }
        if matches!(function, WriteFunction::WriteSingleRegister) && values.len() != 1 {
            return Err(ActionError::SingleNeedsOneValue);
        }
        if register as u32 + values.len() as u32 - 1 > u16::MAX as u32 {
            return Err(ActionError::AddressOverflow);
        }
        let mut words = [0; MAX_ACTION_WORDS];
        let mut i = 0;
        while i < values.len() {
            words[i] = values[i];
            i += 1;
        }
        Ok(BusWriteAction {
            port,
            unit,
            function,
            register,
            // At most MAX_ACTION_WORDS, checked above.
            count: values.len() as u8,
            words,
        })
    }

    /// The bench port the write goes out on.
    #[must_use]
    pub const fn port(&self) -> u8 {
        self.port
    }

    /// The unit (slave) address, 1 to [`MAX_UNIT_ADDRESS`].
    #[must_use]
    pub const fn unit(&self) -> u8 {
        self.unit
    }

    /// The write function.
    #[must_use]
    pub const fn function(&self) -> WriteFunction {
        self.function
    }

    /// The first register written.
    #[must_use]
    pub const fn register(&self) -> u16 {
        self.register
    }

    /// The register values, in register order.
    #[must_use]
    pub fn values(&self) -> &[u16] {
        &self.words[..usize::from(self.count)]
    }

    /// How many registers are written.
    #[must_use]
    pub const fn quantity(&self) -> u16 {
        self.count as u16
    }

    /// The action's canonical encoding, [`ACTION_ENCODED_LEN`] bytes, every
    /// integer big-endian:
    ///
    /// | Offset | Length | Field |
    /// | --- | --- | --- |
    /// | 0 | 1 | Bench port |
    /// | 1 | 1 | Unit address |
    /// | 2 | 1 | Function code, 0x06 or 0x10 |
    /// | 3 | 2 | First register |
    /// | 5 | 1 | Value count, 1 to [`MAX_ACTION_WORDS`] |
    /// | 6 | 2 each | The values, then zeros to the end |
    #[must_use]
    pub const fn encode(&self) -> [u8; ACTION_ENCODED_LEN] {
        let mut out = [0; ACTION_ENCODED_LEN];
        out[0] = self.port;
        out[1] = self.unit;
        out[2] = self.function.code();
        let [r0, r1] = self.register.to_be_bytes();
        out[3] = r0;
        out[4] = r1;
        out[5] = self.count;
        let mut i = 0;
        while i < self.count as usize {
            let [v0, v1] = self.words[i].to_be_bytes();
            out[6 + 2 * i] = v0;
            out[7 + 2 * i] = v1;
            i += 1;
        }
        out
    }

    /// Decodes an action, refusing anything [`BusWriteAction::encode`] would
    /// not produce.
    ///
    /// # Errors
    ///
    /// An [`ActionError`] saying which part is wrong.
    pub fn decode(bytes: &[u8]) -> Result<BusWriteAction, ActionError> {
        let bytes: &[u8; ACTION_ENCODED_LEN] = bytes.try_into().map_err(|_| ActionError::Length)?;
        let function =
            WriteFunction::from_code(bytes[2]).ok_or(ActionError::UnknownFunction(bytes[2]))?;
        let count = usize::from(bytes[5]);
        if count == 0 {
            return Err(ActionError::NoValues);
        }
        if count > MAX_ACTION_WORDS {
            return Err(ActionError::TooManyValues);
        }
        if bytes[6 + 2 * count..].iter().any(|&b| b != 0) {
            return Err(ActionError::NonZeroPadding);
        }
        let mut values = [0; MAX_ACTION_WORDS];
        for (value, pair) in values
            .iter_mut()
            .zip(bytes[6..6 + 2 * count].chunks_exact(2))
        {
            *value = u16::from_be_bytes([pair[0], pair[1]]);
        }
        BusWriteAction::new(
            bytes[0],
            bytes[1],
            function,
            u16::from_be_bytes([bytes[3], bytes[4]]),
            &values[..count],
        )
    }

    /// The digest of this action; see [`ActionDigest::of`].
    #[must_use]
    pub fn digest(&self) -> ActionDigest {
        ActionDigest::of(self)
    }

    /// The one Modbus RTU request frame this action is, CRC included. The
    /// port is not in the frame: it is the line the frame goes out on.
    #[must_use]
    pub const fn frame(&self) -> RtuFrame {
        let mut bytes = [0; MAX_ACTION_FRAME_LEN];
        bytes[0] = self.unit;
        bytes[1] = self.function.code();
        let [r0, r1] = self.register.to_be_bytes();
        bytes[2] = r0;
        bytes[3] = r1;
        let mut len = match self.function {
            WriteFunction::WriteSingleRegister => {
                let [v0, v1] = self.words[0].to_be_bytes();
                bytes[4] = v0;
                bytes[5] = v1;
                6
            }
            WriteFunction::WriteMultipleRegisters => {
                let [q0, q1] = (self.count as u16).to_be_bytes();
                bytes[4] = q0;
                bytes[5] = q1;
                bytes[6] = 2 * self.count;
                let mut i = 0;
                while i < self.count as usize {
                    let [v0, v1] = self.words[i].to_be_bytes();
                    bytes[7 + 2 * i] = v0;
                    bytes[8 + 2 * i] = v1;
                    i += 1;
                }
                7 + 2 * self.count as usize
            }
        };
        let (body, _) = bytes.split_at(len);
        let [c0, c1] = crc16_modbus(body).to_le_bytes();
        bytes[len] = c0;
        bytes[len + 1] = c1;
        len += 2;
        RtuFrame {
            // At most MAX_ACTION_FRAME_LEN, which fits a u8.
            len: len as u8,
            bytes,
        }
    }
}

/// The exact bytes of one request frame, from [`BusWriteAction::frame`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RtuFrame {
    len: u8,
    bytes: [u8; MAX_ACTION_FRAME_LEN],
}

impl RtuFrame {
    /// The frame's bytes, CRC included.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..usize::from(self.len)]
    }
}

/// SHA-256 of [`ACTION_DOMAIN`] followed by [`BusWriteAction::encode`]: the
/// one digest the policy engine's class 3 decision, the interlock's lease,
/// the trust root's approval and the bus guard's lease all carry for the
/// same action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ActionDigest(pub [u8; 32]);

impl ActionDigest {
    /// The digest of `action`.
    #[must_use]
    pub fn of(action: &BusWriteAction) -> ActionDigest {
        let mut hasher = Sha256::new();
        hasher.update(ACTION_DOMAIN);
        hasher.update(action.encode());
        ActionDigest(hasher.finalize().into())
    }
}

#[cfg(feature = "serde")]
mod serde_impls {
    use serde::ser::{SerializeStruct, Serializer};
    use serde::Serialize;

    use super::{ActionDigest, BusWriteAction, WriteFunction};

    impl Serialize for WriteFunction {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            serializer.serialize_str(match self {
                WriteFunction::WriteSingleRegister => "write_single_register",
                WriteFunction::WriteMultipleRegisters => "write_multiple_registers",
            })
        }
    }

    /// Serialize only. An action is made by [`BusWriteAction::new`] or
    /// decoded from a signed approval, never loaded from JSON.
    impl Serialize for BusWriteAction {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            let mut s = serializer.serialize_struct("BusWriteAction", 5)?;
            s.serialize_field("port", &self.port)?;
            s.serialize_field("unit", &self.unit)?;
            s.serialize_field("function", &self.function)?;
            s.serialize_field("register", &self.register)?;
            s.serialize_field("values", self.values())?;
            s.end()
        }
    }

    /// As 64 lower-case hex digits.
    impl Serialize for ActionDigest {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            const HEX: &[u8; 16] = b"0123456789abcdef";
            let mut out = [0u8; 64];
            for (pair, byte) in out.chunks_exact_mut(2).zip(self.0) {
                pair[0] = HEX[usize::from(byte >> 4)];
                pair[1] = HEX[usize::from(byte & 0x0f)];
            }
            // Only ASCII hex digits were written.
            let text = core::str::from_utf8(&out).map_err(serde::ser::Error::custom)?;
            serializer.serialize_str(text)
        }
    }
}
