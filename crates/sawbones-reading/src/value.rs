//! A numeric value kept both as the text the source sent and as an `f64`.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Why a string was rejected as a [`DecimalValue`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DecimalValueError {
    /// The string was empty.
    #[error("value is empty")]
    Empty,
    /// The string was longer than [`DecimalValue::MAX_BYTES`].
    #[error("value is longer than {max} bytes", max = DecimalValue::MAX_BYTES)]
    TooLong,
    /// The string was not a plain decimal number.
    #[error("value is not a plain decimal number")]
    NotDecimal,
    /// The string was a decimal number too large to hold in an `f64`.
    #[error("value does not fit a finite f64")]
    NotFinite,
    /// A serialised value's `parsed` field disagreed with its `text` field.
    #[error("serialised value text and parsed number disagree")]
    Mismatch,
}

/// A number as the source wrote it, plus the same number parsed to `f64`.
///
/// The text is kept because it is the evidence: `53.20` and `53.2` parse to
/// the same double but say different things about the source's resolution.
///
/// Accepted text is an optional sign, digits with an optional decimal point
/// (at least one digit somewhere), and an optional exponent: `1823`, `-1550`,
/// `53.2`, `.5`, `1e3`. Nothing else: no `NaN`, no `inf`, no hex, no digit
/// separators, no surrounding whitespace. The parsed value is always finite.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "DecimalValueWire", into = "DecimalValueWire")]
pub struct DecimalValue {
    text: String,
    parsed: f64,
}

#[derive(Serialize, Deserialize)]
struct DecimalValueWire {
    text: String,
    #[serde(default)]
    parsed: Option<f64>,
}

impl TryFrom<DecimalValueWire> for DecimalValue {
    type Error = DecimalValueError;
    fn try_from(w: DecimalValueWire) -> Result<Self, Self::Error> {
        let v = DecimalValue::parse(&w.text)?;
        match w.parsed {
            Some(p) if p != v.parsed => Err(DecimalValueError::Mismatch),
            _ => Ok(v),
        }
    }
}

impl From<DecimalValue> for DecimalValueWire {
    fn from(v: DecimalValue) -> Self {
        DecimalValueWire {
            text: v.text,
            parsed: Some(v.parsed),
        }
    }
}

impl DecimalValue {
    /// Longest accepted number, in bytes.
    pub const MAX_BYTES: usize = 64;

    /// Parses a plain decimal number. The caller trims whitespace first if
    /// the source is known to pad its values.
    pub fn parse(s: &str) -> Result<Self, DecimalValueError> {
        if s.is_empty() {
            return Err(DecimalValueError::Empty);
        }
        if s.len() > Self::MAX_BYTES {
            return Err(DecimalValueError::TooLong);
        }
        if !is_plain_decimal(s.as_bytes()) {
            return Err(DecimalValueError::NotDecimal);
        }
        let parsed: f64 = s.parse().map_err(|_| DecimalValueError::NotDecimal)?;
        if !parsed.is_finite() {
            return Err(DecimalValueError::NotFinite);
        }
        Ok(Self {
            text: s.to_owned(),
            parsed,
        })
    }

    /// The number exactly as the source wrote it.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The number as a finite `f64`.
    pub fn as_f64(&self) -> f64 {
        self.parsed
    }
}

/// `[+-]? ( digits [. digits?] | . digits ) ( [eE] [+-]? digits )?`
fn is_plain_decimal(b: &[u8]) -> bool {
    let mut i = 0;
    if matches!(b.first(), Some(b'+' | b'-')) {
        i += 1;
    }
    let int_start = i;
    while i < b.len() && b[i].is_ascii_digit() {
        i += 1;
    }
    let int_digits = i - int_start;
    let mut frac_digits = 0;
    if i < b.len() && b[i] == b'.' {
        i += 1;
        let frac_start = i;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
        frac_digits = i - frac_start;
    }
    if int_digits + frac_digits == 0 {
        return false;
    }
    if i < b.len() && matches!(b[i], b'e' | b'E') {
        i += 1;
        if i < b.len() && matches!(b[i], b'+' | b'-') {
            i += 1;
        }
        let exp_start = i;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
        if i == exp_start {
            return false;
        }
    }
    i == b.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_the_shapes_a_meter_sends() {
        for (text, want) in [
            ("1823", 1823.0),
            ("-1550", -1550.0),
            ("53.2", 53.2),
            ("0", 0.0),
            ("0.0", 0.0),
            ("+7", 7.0),
            (".5", 0.5),
            ("5.", 5.0),
            ("1e3", 1000.0),
            ("2.5E-1", 0.25),
        ] {
            let v = DecimalValue::parse(text).unwrap_or_else(|e| panic!("{text}: {e}"));
            assert_eq!(v.as_f64(), want, "{text}");
            assert_eq!(v.text(), text);
        }
    }

    #[test]
    fn rejects_everything_else() {
        for bad in [
            "", " ", "Hybrid", "NaN", "nan", "inf", "-inf", "infinity", "0x10", "1,5", "1_000",
            "1 2", " 5", "5 ", "--5", "+-5", ".", "-", "e5", "1e", "1e+", "12W", "1.2.3", "٣",
        ] {
            assert!(DecimalValue::parse(bad).is_err(), "{bad:?} was accepted");
        }
    }

    #[test]
    fn rejects_numbers_that_overflow_to_infinity() {
        assert_eq!(
            DecimalValue::parse("1e999"),
            Err(DecimalValueError::NotFinite)
        );
    }

    #[test]
    fn rejects_overlong_input() {
        assert_eq!(
            DecimalValue::parse(&"1".repeat(65)),
            Err(DecimalValueError::TooLong)
        );
    }

    #[test]
    fn json_round_trip_keeps_the_text() {
        let v = DecimalValue::parse("53.20").unwrap();
        let json = serde_json::to_string(&v).unwrap();
        assert_eq!(json, r#"{"text":"53.20","parsed":53.2}"#);
        let back: DecimalValue = serde_json::from_str(&json).unwrap();
        assert_eq!(back, v);
    }

    #[test]
    fn json_with_disagreeing_fields_is_rejected() {
        assert!(serde_json::from_str::<DecimalValue>(r#"{"text":"53.2","parsed":99.0}"#).is_err());
        assert!(serde_json::from_str::<DecimalValue>(r#"{"text":"Hybrid","parsed":1.0}"#).is_err());
        // The parsed field is optional on input; the text is the evidence.
        let v: DecimalValue = serde_json::from_str(r#"{"text":"7"}"#).unwrap();
        assert_eq!(v.as_f64(), 7.0);
    }
}
