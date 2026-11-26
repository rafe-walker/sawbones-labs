//! Exact decimal numbers for the decoder: an integer mantissa and a count of
//! decimal places. No floating point anywhere.

use core::cmp::Ordering;
use core::fmt;

/// Most digits a decimal read from a map may have, leading zeros included.
pub const MAX_DIGITS: usize = 18;
/// Most decimal places a decimal read from a map may have.
pub const MAX_SCALE: u8 = 9;

/// Why a string was refused as a [`Decimal`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecimalParseError {
    /// Not `[+-]digits[.digits]`: empty, an exponent, a stray character, or
    /// no digit at all.
    NotDecimal,
    /// More than [`MAX_DIGITS`] digits.
    TooManyDigits,
    /// More than [`MAX_SCALE`] decimal places.
    TooManyPlaces,
}

impl fmt::Display for DecimalParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::NotDecimal => "not a plain decimal number",
            Self::TooManyDigits => "more than 18 digits",
            Self::TooManyPlaces => "more than 9 decimal places",
        })
    }
}

/// `mantissa / 10^scale`, exactly. `Decimal::new(5320, 2)` is `53.20`; the
/// trailing zero is kept because it states the resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Decimal {
    mantissa: i128,
    scale: u8,
}

/// `10^n` as an `i128`, or `None` if it does not fit (n above 38).
pub(crate) const fn pow10(n: u8) -> Option<i128> {
    if n > 38 {
        return None;
    }
    let mut r: i128 = 1;
    let mut i = 0;
    while i < n {
        r *= 10;
        i += 1;
    }
    Some(r)
}

impl Decimal {
    /// Zero, with no decimal places.
    pub const ZERO: Decimal = Decimal {
        mantissa: 0,
        scale: 0,
    };
    /// One, with no decimal places.
    pub const ONE: Decimal = Decimal {
        mantissa: 1,
        scale: 0,
    };

    /// `mantissa / 10^scale`, or `None` if `scale` is above 38 (the most
    /// places an `i128` mantissa can carry).
    pub const fn new(mantissa: i128, scale: u8) -> Option<Self> {
        if scale > 38 {
            None
        } else {
            Some(Self { mantissa, scale })
        }
    }

    /// The integer mantissa.
    pub const fn mantissa(&self) -> i128 {
        self.mantissa
    }

    /// The number of decimal places.
    pub const fn scale(&self) -> u8 {
        self.scale
    }

    /// Parses `[+-]digits[.digits]`: no exponent, no spaces, at least one
    /// digit, at most [`MAX_DIGITS`] digits and [`MAX_SCALE`] places.
    pub fn parse(s: &str) -> Result<Self, DecimalParseError> {
        let b = s.as_bytes();
        let mut i = 0;
        let negative = match b.first() {
            Some(b'-') => {
                i = 1;
                true
            }
            Some(b'+') => {
                i = 1;
                false
            }
            _ => false,
        };
        let mut mantissa: i128 = 0;
        let mut digits = 0usize;
        let mut places = 0usize;
        let mut seen_point = false;
        while i < b.len() {
            match b[i] {
                c @ b'0'..=b'9' => {
                    digits += 1;
                    if digits > MAX_DIGITS {
                        return Err(DecimalParseError::TooManyDigits);
                    }
                    if seen_point {
                        places += 1;
                        if places > usize::from(MAX_SCALE) {
                            return Err(DecimalParseError::TooManyPlaces);
                        }
                    }
                    mantissa = mantissa * 10 + i128::from(c - b'0');
                }
                b'.' if !seen_point => seen_point = true,
                _ => return Err(DecimalParseError::NotDecimal),
            }
            i += 1;
        }
        if digits == 0 {
            return Err(DecimalParseError::NotDecimal);
        }
        Ok(Self {
            mantissa: if negative { -mantissa } else { mantissa },
            // places <= MAX_SCALE, checked above.
            scale: places as u8,
        })
    }

    /// Whether the value is zero, however many places it has.
    pub const fn is_zero(&self) -> bool {
        self.mantissa == 0
    }

    /// Compares by value: `1.0` equals `1.00`. Total: it never overflows.
    pub fn cmp_value(&self, other: &Decimal) -> Ordering {
        match self.scale.cmp(&other.scale) {
            Ordering::Equal => self.mantissa.cmp(&other.mantissa),
            Ordering::Less => cmp_scaled(self, other),
            Ordering::Greater => cmp_scaled(other, self).reverse(),
        }
    }

    /// The text form, in a fixed buffer, with no allocation.
    pub fn to_text(&self) -> DecimalText {
        let mut t = DecimalText {
            buf: [0; DecimalText::CAPACITY],
            len: 0,
        };
        // The buffer holds any decimal: 39 digits, a sign and a point.
        let _ = fmt::Write::write_fmt(&mut t, format_args!("{self}"));
        t
    }
}

/// Compares `lo` (the one with fewer places) with `hi` by scaling `lo` up.
fn cmp_scaled(lo: &Decimal, hi: &Decimal) -> Ordering {
    let shifted = pow10(hi.scale - lo.scale).and_then(|f| lo.mantissa.checked_mul(f));
    match shifted {
        Some(m) => m.cmp(&hi.mantissa),
        // `lo` scaled up exceeds every i128 mantissa, so it is larger in
        // magnitude than `hi`; its sign decides.
        None if lo.mantissa < 0 => Ordering::Less,
        None => Ordering::Greater,
    }
}

impl fmt::Display for Decimal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut digits = [0u8; 40];
        let mut n = 0;
        let mut m = self.mantissa.unsigned_abs();
        loop {
            digits[n] = b'0' + (m % 10) as u8;
            n += 1;
            m /= 10;
            if m == 0 {
                break;
            }
        }
        let scale = usize::from(self.scale);
        // At least one digit before the point.
        while n <= scale {
            digits[n] = b'0';
            n += 1;
        }
        if self.mantissa < 0 {
            f.write_str("-")?;
        }
        for k in (0..n).rev() {
            f.write_str(core::str::from_utf8(&digits[k..=k]).unwrap_or("?"))?;
            if k == scale && scale > 0 {
                f.write_str(".")?;
            }
        }
        Ok(())
    }
}

/// A [`Decimal`] rendered as text in a fixed buffer.
#[derive(Clone, Copy)]
pub struct DecimalText {
    buf: [u8; Self::CAPACITY],
    len: usize,
}

impl DecimalText {
    /// Buffer size in bytes: sign, 39 digits and a point, with room to spare.
    pub const CAPACITY: usize = 48;

    /// The text, for example `-12.50`.
    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.buf[..self.len]).unwrap_or("")
    }
}

impl fmt::Write for DecimalText {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let end = self.len + s.len();
        if end > Self::CAPACITY {
            return Err(fmt::Error);
        }
        self.buf[self.len..end].copy_from_slice(s.as_bytes());
        self.len = end;
        Ok(())
    }
}

impl fmt::Debug for DecimalText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.as_str())
    }
}

impl PartialEq for DecimalText {
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_round_trips_and_keeps_trailing_zeros() {
        for s in [
            "0",
            "53.20",
            "-0.05",
            "7",
            "-12.5",
            "0.000000001",
            "123456789012345678",
        ] {
            let d = Decimal::parse(s).unwrap();
            assert_eq!(d.to_text().as_str(), s);
        }
        assert_eq!(Decimal::new(-5, 2).unwrap().to_text().as_str(), "-0.05");
        assert_eq!(Decimal::new(5, 3).unwrap().to_text().as_str(), "0.005");
    }

    #[test]
    fn parse_refuses_everything_else() {
        for bad in [
            "", "-", ".", "1e3", " 1", "1 ", "1.2.3", "--1", "NaN", "0x1", "1,5",
        ] {
            assert_eq!(
                Decimal::parse(bad),
                Err(DecimalParseError::NotDecimal),
                "{bad:?}"
            );
        }
        assert_eq!(
            Decimal::parse("1234567890123456789"),
            Err(DecimalParseError::TooManyDigits)
        );
        assert_eq!(
            Decimal::parse("0.0000000001"),
            Err(DecimalParseError::TooManyPlaces)
        );
    }

    #[test]
    fn compare_is_by_value_and_never_overflows() {
        let a = Decimal::parse("1.0").unwrap();
        let b = Decimal::parse("1.00").unwrap();
        assert_eq!(a.cmp_value(&b), Ordering::Equal);
        let huge = Decimal::new(i128::MAX, 0).unwrap();
        let tiny = Decimal::new(1, 9).unwrap();
        assert_eq!(huge.cmp_value(&tiny), Ordering::Greater);
        assert_eq!(tiny.cmp_value(&huge), Ordering::Less);
        let neg = Decimal::new(i128::MIN, 0).unwrap();
        assert_eq!(neg.cmp_value(&tiny), Ordering::Less);
        assert_eq!(tiny.cmp_value(&neg), Ordering::Greater);
    }

    #[test]
    fn extreme_mantissa_renders() {
        assert!(Decimal::new(i128::MIN, 9)
            .unwrap()
            .to_text()
            .as_str()
            .starts_with('-'));
        assert!(!Decimal::new(i128::MAX, 38)
            .unwrap()
            .to_text()
            .as_str()
            .is_empty());
    }
}
