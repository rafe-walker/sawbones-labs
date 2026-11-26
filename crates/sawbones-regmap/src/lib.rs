//! # sawbones-regmap: PROVISIONAL
//!
//! **This crate is provisional. The Sawbones design is under review and
//! everything here may change.**
//!
//! Devices are described by data, not code (#174). A register map is a TOML
//! file: per register an address, a width, a type, a scale and offset, a unit,
//! byte and word order, a valid range, read or write access and, for a
//! writable register, its hazard class. A small first-party engine runs the
//! map; an agent's fix to a decoder is then a corrected fact, not new code.
//!
//! - [`decimal`] and [`decode`] are the decoder core: `no_std`, no
//!   allocation, no floats, and total (a value or a named error for every
//!   input). Build it for firmware with `--no-default-features`.
//! - With the default `std` feature, [`RegisterMap`] loads a map strictly and
//!   refuses anything ambiguous or unsafe: unknown keys, overlapping or
//!   aliased registers, a width that does not fit the type, a writable
//!   register with no class, no valid range or a class below what the policy
//!   engine would assign. It turns raw words into
//!   [`sawbones_reading::Observation`]s with exact decimal text and a
//!   declared unit, and it produces the policy engine's write-map entries so
//!   the map and the policy cannot drift apart.
//!
//! This crate loads maps only. It does not load plugins; a plugin host calls
//! [`RegisterMap::observe`] or [`RegisterMap::decode_block`] with the words it
//! read and gets back values or refusals.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

pub mod decimal;
pub mod decode;

pub use decimal::{Decimal, DecimalParseError, DecimalText};
pub use decode::{ByteOrder, Codec, CodecError, DataType, DecodeError, WordOrder};

#[cfg(feature = "std")]
mod map;

#[cfg(feature = "std")]
pub use map::{
    Access, MapError, ObservationContext, ReadError, Register, RegisterMap, Table, WriteInfo,
    MAP_SCHEMA, MAX_MAP_BYTES, MAX_REGISTERS,
};
