//! The decoder core: raw register words in, exact decimal out. `no_std`, no
//! allocation, no floats, and total: every input gives a value or a named
//! error, never a panic or a wrapped number.

use core::cmp::Ordering;
use core::fmt;

use crate::decimal::{pow10, Decimal};

/// The number type a register (or register group) holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "std", derive(serde::Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "lowercase"))]
pub enum DataType {
    /// Unsigned 16 bits, one register.
    U16,
    /// Signed 16 bits (two's complement), one register.
    I16,
    /// Unsigned 32 bits, two registers.
    U32,
    /// Signed 32 bits, two registers.
    I32,
    /// Unsigned 64 bits, four registers.
    U64,
    /// Signed 64 bits, four registers.
    I64,
}

impl DataType {
    /// How many 16-bit registers the type occupies.
    pub const fn words(self) -> u8 {
        match self {
            Self::U16 | Self::I16 => 1,
            Self::U32 | Self::I32 => 2,
            Self::U64 | Self::I64 => 4,
        }
    }

    /// Whether the type is signed.
    pub const fn is_signed(self) -> bool {
        matches!(self, Self::I16 | Self::I32 | Self::I64)
    }

    /// The smallest raw value of the type.
    pub const fn min_raw(self) -> i128 {
        match self {
            Self::U16 | Self::U32 | Self::U64 => 0,
            Self::I16 => i16::MIN as i128,
            Self::I32 => i32::MIN as i128,
            Self::I64 => i64::MIN as i128,
        }
    }

    /// The largest raw value of the type.
    pub const fn max_raw(self) -> i128 {
        match self {
            Self::U16 => u16::MAX as i128,
            Self::I16 => i16::MAX as i128,
            Self::U32 => u32::MAX as i128,
            Self::I32 => i32::MAX as i128,
            Self::U64 => u64::MAX as i128,
            Self::I64 => i64::MAX as i128,
        }
    }
}

/// Which byte of a 16-bit word comes first on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "std", derive(serde::Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "snake_case"))]
pub enum ByteOrder {
    /// High byte first. The Modbus standard.
    #[default]
    Big,
    /// Low byte first: each word is byte-swapped.
    Little,
}

/// Which word of a multi-register value comes first on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "std", derive(serde::Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "snake_case"))]
pub enum WordOrder {
    /// Most significant word first.
    #[default]
    Big,
    /// Least significant word first.
    Little,
}

/// Why decoding (or encoding) was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeError {
    /// The slice did not hold exactly the type's number of words.
    WrongWordCount {
        /// Words the type occupies.
        expected: u8,
        /// Words given.
        got: usize,
    },
    /// The scaled value does not fit the decoder's arithmetic.
    Overflow,
    /// The scaled value is outside the register's valid range. The value is
    /// refused, never clamped.
    OutOfRange,
    /// A raw value outside the register type, for encoding.
    RawOutsideType,
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongWordCount { expected, got } => {
                write!(f, "expected {expected} register words, got {got}")
            }
            Self::Overflow => f.write_str("scaled value overflows"),
            Self::OutOfRange => f.write_str("value is outside the register's valid range"),
            Self::RawOutsideType => f.write_str("raw value is outside the register type"),
        }
    }
}

/// Why a [`Codec`] was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodecError {
    /// The scale is zero, so no raw value could be recovered from a value.
    ZeroScale,
    /// The valid range's minimum is above its maximum.
    RangeInverted,
    /// Some raw value of the type overflows the arithmetic once scaled, so
    /// the decoder would not be total.
    CanOverflow,
}

impl fmt::Display for CodecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::ZeroScale => "scale is zero",
            Self::RangeInverted => "valid range minimum is above its maximum",
            Self::CanOverflow => "a raw value of the type overflows once scaled",
        })
    }
}

/// How one register (group) turns into a number: `value = raw * scale +
/// offset`, then checked against the valid range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Codec {
    data_type: DataType,
    byte_order: ByteOrder,
    word_order: WordOrder,
    scale: Decimal,
    offset: Decimal,
    min: Option<Decimal>,
    max: Option<Decimal>,
}

impl Codec {
    /// Builds a codec. Refuses a zero scale, an inverted range, and any
    /// combination for which a raw value of the type can overflow, so
    /// [`Codec::decode`] never meets arithmetic it cannot do.
    pub fn new(
        data_type: DataType,
        byte_order: ByteOrder,
        word_order: WordOrder,
        scale: Decimal,
        offset: Decimal,
        min: Option<Decimal>,
        max: Option<Decimal>,
    ) -> Result<Self, CodecError> {
        if scale.is_zero() {
            return Err(CodecError::ZeroScale);
        }
        if let (Some(lo), Some(hi)) = (min, max) {
            if lo.cmp_value(&hi) == Ordering::Greater {
                return Err(CodecError::RangeInverted);
            }
        }
        let c = Self {
            data_type,
            byte_order,
            word_order,
            scale,
            offset,
            min,
            max,
        };
        // The arithmetic is monotone in raw, so the two ends of the type are
        // the worst cases.
        if c.scaled(data_type.min_raw()).is_err() || c.scaled(data_type.max_raw()).is_err() {
            return Err(CodecError::CanOverflow);
        }
        Ok(c)
    }

    /// The register type.
    pub const fn data_type(&self) -> DataType {
        self.data_type
    }

    /// The scale.
    pub const fn scale(&self) -> Decimal {
        self.scale
    }

    /// The offset.
    pub const fn offset(&self) -> Decimal {
        self.offset
    }

    /// The valid range's minimum, if there is one.
    pub const fn min(&self) -> Option<Decimal> {
        self.min
    }

    /// The valid range's maximum, if there is one.
    pub const fn max(&self) -> Option<Decimal> {
        self.max
    }

    /// The raw integer the words hold, before scaling.
    pub fn raw(&self, words: &[u16]) -> Result<i128, DecodeError> {
        let n = usize::from(self.data_type.words());
        if words.len() != n {
            return Err(DecodeError::WrongWordCount {
                expected: self.data_type.words(),
                got: words.len(),
            });
        }
        let mut acc: u64 = 0;
        for k in 0..n {
            let w = match self.word_order {
                WordOrder::Big => words[k],
                WordOrder::Little => words[n - 1 - k],
            };
            let w = match self.byte_order {
                ByteOrder::Big => w,
                ByteOrder::Little => w.swap_bytes(),
            };
            acc = (acc << 16) | u64::from(w);
        }
        Ok(match self.data_type {
            DataType::U16 | DataType::U32 | DataType::U64 => i128::from(acc),
            DataType::I16 => i128::from(acc as u16 as i16),
            DataType::I32 => i128::from(acc as u32 as i32),
            DataType::I64 => i128::from(acc as i64),
        })
    }

    /// `raw * scale + offset`, exactly.
    pub fn scaled(&self, raw: i128) -> Result<Decimal, DecodeError> {
        let d = self.scale.scale().max(self.offset.scale());
        let a = pow10(d - self.scale.scale())
            .and_then(|f| raw.checked_mul(self.scale.mantissa())?.checked_mul(f))
            .ok_or(DecodeError::Overflow)?;
        let b = pow10(d - self.offset.scale())
            .and_then(|f| self.offset.mantissa().checked_mul(f))
            .ok_or(DecodeError::Overflow)?;
        let m = a.checked_add(b).ok_or(DecodeError::Overflow)?;
        Decimal::new(m, d).ok_or(DecodeError::Overflow)
    }

    /// Words to exact value, checked against the valid range.
    pub fn decode(&self, words: &[u16]) -> Result<Decimal, DecodeError> {
        let v = self.scaled(self.raw(words)?)?;
        if let Some(lo) = self.min {
            if v.cmp_value(&lo) == Ordering::Less {
                return Err(DecodeError::OutOfRange);
            }
        }
        if let Some(hi) = self.max {
            if v.cmp_value(&hi) == Ordering::Greater {
                return Err(DecodeError::OutOfRange);
            }
        }
        Ok(v)
    }

    /// The register words for a raw integer: the inverse of [`Codec::raw`].
    /// Returns the buffer and how many words of it are used.
    pub fn words_for_raw(&self, raw: i128) -> Result<([u16; 4], usize), DecodeError> {
        if raw < self.data_type.min_raw() || raw > self.data_type.max_raw() {
            return Err(DecodeError::RawOutsideType);
        }
        let n = usize::from(self.data_type.words());
        // Two's complement in the low 64 bits; the cast truncates.
        let bits = raw as u64;
        let mut out = [0u16; 4];
        for k in 0..n {
            let shift = 16 * (n - 1 - k);
            let w = (bits >> shift) as u16;
            let w = match self.byte_order {
                ByteOrder::Big => w,
                ByteOrder::Little => w.swap_bytes(),
            };
            match self.word_order {
                WordOrder::Big => out[k] = w,
                WordOrder::Little => out[n - 1 - k] = w,
            }
        }
        Ok((out, n))
    }

    /// The raw integers that bracket a value: `(floor, ceil)` of
    /// `(value - offset) / scale`. They are equal exactly when the value is
    /// reachable. Used to turn an engineering range into a raw range.
    pub fn raw_floor_ceil(&self, value: Decimal) -> Result<(i128, i128), DecodeError> {
        let d = value.scale().max(self.offset.scale());
        let num = pow10(d - value.scale())
            .and_then(|f| value.mantissa().checked_mul(f))
            .zip(pow10(d - self.offset.scale()).and_then(|f| self.offset.mantissa().checked_mul(f)))
            .and_then(|(a, b)| a.checked_sub(b))
            .and_then(|n| n.checked_mul(pow10(self.scale.scale())?))
            .ok_or(DecodeError::Overflow)?;
        let den = pow10(d)
            .and_then(|f| f.checked_mul(self.scale.mantissa()))
            .ok_or(DecodeError::Overflow)?;
        // Normalise to a positive denominator.
        let (num, den) = if den < 0 {
            (num.checked_neg().ok_or(DecodeError::Overflow)?, -den)
        } else {
            (num, den)
        };
        let q = num.div_euclid(den);
        let r = num.rem_euclid(den);
        Ok((q, if r == 0 { q } else { q + 1 }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> Decimal {
        Decimal::parse(s).unwrap()
    }

    fn codec(t: DataType, scale: &str, offset: &str) -> Codec {
        Codec::new(
            t,
            ByteOrder::Big,
            WordOrder::Big,
            d(scale),
            d(offset),
            None,
            None,
        )
        .unwrap()
    }

    #[test]
    fn scale_and_offset_are_exact() {
        let c = codec(DataType::I16, "0.1", "-40");
        // raw 653 -> 65.3 - 40 = 25.3
        assert_eq!(c.decode(&[653]).unwrap().to_text().as_str(), "25.3");
        let c = codec(DataType::U16, "0.01", "0");
        assert_eq!(c.decode(&[5320]).unwrap().to_text().as_str(), "53.20");
    }

    #[test]
    fn sign_follows_the_type() {
        let s = codec(DataType::I16, "1", "0");
        let u = codec(DataType::U16, "1", "0");
        assert_eq!(s.decode(&[0xFFFF]).unwrap().to_text().as_str(), "-1");
        assert_eq!(u.decode(&[0xFFFF]).unwrap().to_text().as_str(), "65535");
        let s32 = codec(DataType::I32, "1", "0");
        assert_eq!(
            s32.decode(&[0xFFFF, 0xFFFE]).unwrap().to_text().as_str(),
            "-2"
        );
    }

    #[test]
    fn byte_and_word_order() {
        let mk = |b, w| {
            Codec::new(DataType::U32, b, w, Decimal::ONE, Decimal::ZERO, None, None).unwrap()
        };
        let words = [0x0102u16, 0x0304];
        let v = |c: Codec| c.raw(&words).unwrap();
        assert_eq!(v(mk(ByteOrder::Big, WordOrder::Big)), 0x0102_0304);
        assert_eq!(v(mk(ByteOrder::Big, WordOrder::Little)), 0x0304_0102);
        assert_eq!(v(mk(ByteOrder::Little, WordOrder::Big)), 0x0201_0403);
        assert_eq!(v(mk(ByteOrder::Little, WordOrder::Little)), 0x0403_0201);
    }

    #[test]
    fn wrong_word_count_is_refused() {
        let c = codec(DataType::U32, "1", "0");
        assert_eq!(
            c.decode(&[1]),
            Err(DecodeError::WrongWordCount {
                expected: 2,
                got: 1
            })
        );
        assert!(c.decode(&[]).is_err());
        assert!(c.decode(&[1, 2, 3]).is_err());
    }

    #[test]
    fn out_of_range_is_refused_not_clamped() {
        let c = Codec::new(
            DataType::U16,
            ByteOrder::Big,
            WordOrder::Big,
            d("0.1"),
            Decimal::ZERO,
            Some(d("10.0")),
            Some(d("60")),
        )
        .unwrap();
        assert_eq!(c.decode(&[99]), Err(DecodeError::OutOfRange));
        assert_eq!(c.decode(&[100]).unwrap().to_text().as_str(), "10.0");
        assert_eq!(c.decode(&[600]).unwrap().to_text().as_str(), "60.0");
        assert_eq!(c.decode(&[601]), Err(DecodeError::OutOfRange));
    }

    #[test]
    fn construction_refuses_unsound_codecs() {
        let z = Codec::new(
            DataType::U16,
            ByteOrder::Big,
            WordOrder::Big,
            d("0"),
            Decimal::ZERO,
            None,
            None,
        );
        assert_eq!(z, Err(CodecError::ZeroScale));
        let inv = Codec::new(
            DataType::U16,
            ByteOrder::Big,
            WordOrder::Big,
            Decimal::ONE,
            Decimal::ZERO,
            Some(d("5")),
            Some(d("4")),
        );
        assert_eq!(inv, Err(CodecError::RangeInverted));
        // 2^64 * 10^18 * 10^5 does not fit an i128.
        let big = Codec::new(
            DataType::U64,
            ByteOrder::Big,
            WordOrder::Big,
            d("99999999999999.9999"),
            d("0.000000001"),
            None,
            None,
        );
        assert_eq!(big, Err(CodecError::CanOverflow));
    }

    #[test]
    fn raw_floor_ceil_brackets_a_value() {
        let c = codec(DataType::U16, "0.1", "0");
        assert_eq!(c.raw_floor_ceil(d("12.3")).unwrap(), (123, 123));
        assert_eq!(c.raw_floor_ceil(d("12.34")).unwrap(), (123, 124));
        assert_eq!(c.raw_floor_ceil(d("-0.05")).unwrap(), (-1, 0));
        // A negative scale flips the direction.
        let n = codec(DataType::I16, "-0.5", "10");
        // raw = (v - 10) / -0.5; v = 9 -> 2
        assert_eq!(n.raw_floor_ceil(d("9")).unwrap(), (2, 2));
    }
}
