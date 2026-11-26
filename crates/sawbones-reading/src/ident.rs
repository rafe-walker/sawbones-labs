//! Short validated strings: identifiers and unit symbols.

use std::fmt;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Why a string was rejected as an [`Ident`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum IdentError {
    /// The string was empty.
    #[error("identifier is empty")]
    Empty,
    /// The string was longer than [`Ident::MAX_BYTES`].
    #[error("identifier is longer than {max} bytes", max = Ident::MAX_BYTES)]
    TooLong,
    /// The string held a character outside `A-Z a-z 0-9 _ - . /`.
    #[error("identifier contains a character outside A-Z a-z 0-9 _ - . /")]
    BadCharacter,
}

/// A source id or quantity name: 1 to 128 bytes of `A-Z a-z 0-9 _ - . /`.
///
/// Identifiers are often *derived* from outside names (for Solar Assistant,
/// from MQTT topic segments). The character set and length limit keep them
/// from carrying free text, markup or control characters, which is why
/// `Ident` has a `Display` impl and [`crate::WorldText`] does not. The limit
/// constrains shape, not meaning: an identifier is still a name somebody else
/// chose, so treat it as a label and nothing more.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Ident(String);

impl Ident {
    /// Longest allowed identifier, in bytes.
    pub const MAX_BYTES: usize = 128;

    /// Validates and wraps an identifier.
    pub fn new(s: &str) -> Result<Self, IdentError> {
        if s.is_empty() {
            return Err(IdentError::Empty);
        }
        if s.len() > Self::MAX_BYTES {
            return Err(IdentError::TooLong);
        }
        if !s
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.' | b'/'))
        {
            return Err(IdentError::BadCharacter);
        }
        Ok(Self(s.to_owned()))
    }

    /// The identifier as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Ident {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl TryFrom<String> for Ident {
    type Error = IdentError;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::new(&s)
    }
}

impl From<Ident> for String {
    fn from(i: Ident) -> Self {
        i.0
    }
}

/// Why a string was rejected as a [`Unit`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum UnitError {
    /// The string was empty.
    #[error("unit is empty")]
    Empty,
    /// The string was longer than [`Unit::MAX_BYTES`].
    #[error("unit is longer than {max} bytes", max = Unit::MAX_BYTES)]
    TooLong,
    /// The string held whitespace, a control character or an angle bracket.
    #[error("unit contains whitespace, a control character or an angle bracket")]
    BadCharacter,
}

/// A unit symbol such as `W`, `V`, `kWh`, `%` or `°C`: 1 to 16 bytes with no
/// whitespace, control characters or angle brackets.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Unit(String);

impl Unit {
    /// Longest allowed unit symbol, in bytes of UTF-8.
    pub const MAX_BYTES: usize = 16;

    /// Validates and wraps a unit symbol.
    pub fn new(s: &str) -> Result<Self, UnitError> {
        if s.is_empty() {
            return Err(UnitError::Empty);
        }
        if s.len() > Self::MAX_BYTES {
            return Err(UnitError::TooLong);
        }
        if s.chars()
            .any(|c| c.is_whitespace() || c.is_control() || c == '<' || c == '>' || c == '\\')
        {
            return Err(UnitError::BadCharacter);
        }
        Ok(Self(s.to_owned()))
    }

    /// The unit symbol as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Unit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl TryFrom<String> for Unit {
    type Error = UnitError;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::new(&s)
    }
}

impl From<Unit> for String {
    fn from(u: Unit) -> Self {
        u.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_topic_shaped_names() {
        for ok in [
            "pv_power",
            "solar_assistant/inverter_1",
            "load_power_non-essential",
            "a.b",
        ] {
            assert!(Ident::new(ok).is_ok(), "{ok}");
        }
    }

    #[test]
    fn rejects_free_text_and_markup() {
        assert_eq!(Ident::new(""), Err(IdentError::Empty));
        assert_eq!(Ident::new("has space"), Err(IdentError::BadCharacter));
        assert_eq!(Ident::new("new\nline"), Err(IdentError::BadCharacter));
        assert_eq!(Ident::new("<tag>"), Err(IdentError::BadCharacter));
        assert_eq!(Ident::new("größe"), Err(IdentError::BadCharacter));
        assert_eq!(Ident::new(&"a".repeat(129)), Err(IdentError::TooLong));
    }

    #[test]
    fn ident_deserialisation_validates() {
        assert!(serde_json::from_str::<Ident>(r#""pv_power""#).is_ok());
        assert!(serde_json::from_str::<Ident>(r#""pv power""#).is_err());
    }

    #[test]
    fn units_accept_symbols_and_reject_prose() {
        for ok in ["W", "kWh", "%", "°C", "°F", "Hz", "VA"] {
            assert!(Unit::new(ok).is_ok(), "{ok}");
        }
        assert_eq!(Unit::new(""), Err(UnitError::Empty));
        assert_eq!(Unit::new("W now"), Err(UnitError::BadCharacter));
        assert_eq!(
            Unit::new("ignore_everything_above"),
            Err(UnitError::TooLong)
        );
        assert!(serde_json::from_str::<Unit>(r#""a b""#).is_err());
    }
}
