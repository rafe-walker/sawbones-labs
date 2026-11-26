//! Modbus RTU request frames: the CRC, the four supported function codes, and
//! a parser that refuses anything else with a typed reason.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// The longest Modbus RTU frame (serial line ADU), in bytes.
pub const MAX_FRAME_LEN: usize = 256;

/// The most registers one read (0x03 or 0x04) may ask for.
pub const MAX_READ_QUANTITY: u16 = 125;

/// The most registers one write multiple registers (0x10) may carry.
pub const MAX_WRITE_QUANTITY: u16 = 123;

/// The highest individual unit address. 0 is broadcast and 248 to 255 are
/// reserved.
pub const MAX_UNIT_ADDRESS: u8 = 247;

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

/// The function codes the guard understands. Every other code is blocked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum Function {
    /// 0x03, read holding registers.
    ReadHoldingRegisters,
    /// 0x04, read input registers.
    ReadInputRegisters,
    /// 0x06, write single register.
    WriteSingleRegister,
    /// 0x10, write multiple registers.
    WriteMultipleRegisters,
}

impl Function {
    /// The function for a code on the wire, or `None` if the guard does not
    /// support it.
    #[must_use]
    pub const fn from_code(code: u8) -> Option<Function> {
        match code {
            0x03 => Some(Function::ReadHoldingRegisters),
            0x04 => Some(Function::ReadInputRegisters),
            0x06 => Some(Function::WriteSingleRegister),
            0x10 => Some(Function::WriteMultipleRegisters),
            _ => None,
        }
    }

    /// The code on the wire.
    #[must_use]
    pub const fn code(self) -> u8 {
        match self {
            Function::ReadHoldingRegisters => 0x03,
            Function::ReadInputRegisters => 0x04,
            Function::WriteSingleRegister => 0x06,
            Function::WriteMultipleRegisters => 0x10,
        }
    }

    /// True for the two write functions, which need a lease as well as a
    /// rule.
    #[must_use]
    pub const fn is_write(self) -> bool {
        matches!(
            self,
            Function::WriteSingleRegister | Function::WriteMultipleRegisters
        )
    }
}

/// Why a byte string is not a frame the guard will consider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum FrameError {
    /// Fewer bytes than the function code needs.
    Truncated,
    /// More than [`MAX_FRAME_LEN`] bytes.
    TooLong,
    /// More bytes than the function code needs: trailing bytes after the CRC.
    LengthMismatch,
    /// The function code is not one of the four in [`Function`].
    UnsupportedFunction(u8),
    /// The CRC does not match the bytes before it.
    BadCrc,
    /// The quantity is zero or above the protocol's limit for the function.
    QuantityOutOfRange,
    /// The byte count of a write multiple registers is not twice its
    /// quantity.
    ByteCountMismatch,
    /// The register range runs past address 0xFFFF.
    AddressOverflow,
    /// Unit address 0: a broadcast. Every device on the line would act on it,
    /// so it is never passed, read or write.
    Broadcast,
    /// Unit addresses 248 to 255 are reserved by the protocol.
    ReservedAddress(u8),
}

/// A request frame that parsed and whose CRC checked.
///
/// Borrowed from the bytes it was parsed from. The fields are private, so the
/// only way to get one is [`parse`], and every value obeys its checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Frame<'a> {
    unit: u8,
    function: Function,
    start: u16,
    quantity: u16,
    byte_count: Option<u8>,
    payload: &'a [u8],
    crc: u16,
}

impl<'a> Frame<'a> {
    /// The unit (slave) address, 1 to [`MAX_UNIT_ADDRESS`].
    #[must_use]
    pub const fn unit(&self) -> u8 {
        self.unit
    }

    /// The function.
    #[must_use]
    pub const fn function(&self) -> Function {
        self.function
    }

    /// The first register addressed.
    #[must_use]
    pub const fn start(&self) -> u16 {
        self.start
    }

    /// How many registers are addressed: at least 1, and always 1 for a
    /// single write.
    #[must_use]
    pub const fn quantity(&self) -> u16 {
        self.quantity
    }

    /// The last register addressed. The parser refuses a range that would run
    /// past 0xFFFF, so this is exact.
    #[must_use]
    pub const fn end(&self) -> u16 {
        self.start.saturating_add(self.quantity.saturating_sub(1))
    }

    /// The byte count field. Present only on a write multiple registers.
    #[must_use]
    pub const fn byte_count(&self) -> Option<u8> {
        self.byte_count
    }

    /// The register values written, big-endian as on the wire: two bytes for
    /// a single write, `byte_count` bytes for a multiple write, empty for a
    /// read.
    #[must_use]
    pub const fn payload(&self) -> &'a [u8] {
        self.payload
    }

    /// The CRC as received, already checked.
    #[must_use]
    pub const fn crc(&self) -> u16 {
        self.crc
    }
}

/// Parses one Modbus RTU request frame, CRC included.
///
/// The checks run in this order, so the first reason found is the one
/// returned: length bounds, function code, exact length for that function,
/// CRC, then the fields (quantity, byte count, register range, unit address).
///
/// # Errors
///
/// A [`FrameError`] for anything that is not exactly one well-formed request
/// with one of the four supported function codes.
pub fn parse(bytes: &[u8]) -> Result<Frame<'_>, FrameError> {
    if bytes.len() > MAX_FRAME_LEN {
        return Err(FrameError::TooLong);
    }
    // Unit address, function code and the two CRC bytes at the least.
    if bytes.len() < 4 {
        return Err(FrameError::Truncated);
    }
    let code = bytes[1];
    let function = Function::from_code(code).ok_or(FrameError::UnsupportedFunction(code))?;

    let expected_len = match function {
        Function::ReadHoldingRegisters
        | Function::ReadInputRegisters
        | Function::WriteSingleRegister => 8,
        Function::WriteMultipleRegisters => {
            // The byte count sits at offset 6; the frame is 9 bytes plus it.
            match bytes.get(6) {
                Some(&count) => 9 + usize::from(count),
                None => return Err(FrameError::Truncated),
            }
        }
    };
    if bytes.len() < expected_len {
        return Err(FrameError::Truncated);
    }
    if bytes.len() > expected_len {
        return Err(FrameError::LengthMismatch);
    }

    let (body, crc_bytes) = bytes.split_at(expected_len - 2);
    let crc = u16::from_le_bytes([crc_bytes[0], crc_bytes[1]]);
    if crc16_modbus(body) != crc {
        return Err(FrameError::BadCrc);
    }

    let unit = body[0];
    let start = u16::from_be_bytes([body[2], body[3]]);
    let (quantity, byte_count, payload) = match function {
        Function::ReadHoldingRegisters | Function::ReadInputRegisters => {
            let quantity = u16::from_be_bytes([body[4], body[5]]);
            if quantity == 0 || quantity > MAX_READ_QUANTITY {
                return Err(FrameError::QuantityOutOfRange);
            }
            (quantity, None, &body[6..6])
        }
        Function::WriteSingleRegister => (1, None, &body[4..6]),
        Function::WriteMultipleRegisters => {
            let quantity = u16::from_be_bytes([body[4], body[5]]);
            if quantity == 0 || quantity > MAX_WRITE_QUANTITY {
                return Err(FrameError::QuantityOutOfRange);
            }
            let count = body[6];
            if u16::from(count) != quantity * 2 {
                return Err(FrameError::ByteCountMismatch);
            }
            (quantity, Some(count), &body[7..])
        }
    };
    if u32::from(start) + u32::from(quantity) - 1 > u32::from(u16::MAX) {
        return Err(FrameError::AddressOverflow);
    }
    if unit == 0 {
        return Err(FrameError::Broadcast);
    }
    if unit > MAX_UNIT_ADDRESS {
        return Err(FrameError::ReservedAddress(unit));
    }

    Ok(Frame {
        unit,
        function,
        start,
        quantity,
        byte_count,
        payload,
        crc,
    })
}
