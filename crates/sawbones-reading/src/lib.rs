//! # sawbones-reading: PROVISIONAL
//!
//! **This crate is provisional. The Sawbones design is still under review, and
//! every type here may change or be replaced without notice.** It exists so the
//! first ingest path (rung 0, a read-only tap on Solar Assistant's MQTT output)
//! has something typed to produce. Do not build on it as if it were stable.
//!
//! What is here:
//!
//! - [`Observation`]: what a decoder or plugin builds: one numeric value from
//!   one source, with its unit, when it arrived, how good that timestamp is
//!   and a [`Provenance`] record saying exactly where it came from. It has
//!   no trust tier, and a plugin cannot give it one.
//! - [`Reading`]: an observation plus a [`Trust`] tier, made only by the host
//!   (`Reading::from_observation_host_only`). Its fields are private.
//! - [`WorldText`]: a wrapper for any string that originated outside this
//!   project's own code. It has no `Display` impl, so it cannot be dropped into
//!   a prompt or a log line by accident. Use [`WorldText::fenced`] to render it
//!   inside a clear untrusted-data marker.
//! - Small validated newtypes ([`Ident`], [`Unit`], [`DecimalValue`],
//!   [`UtcNanos`]) so a `Reading` that deserialises is also a `Reading` whose
//!   fields have a known shape.
//!
//! What is deliberately not here: the tool surface, the knowledge store, the
//! module standard and any rule about which trust tier may justify which
//! action. Those are still being designed.

#![forbid(unsafe_code)]

mod ident;
mod reading;
mod time;
mod value;
mod world_text;

pub use ident::{Ident, IdentError, Unit, UnitError};
pub use reading::{
    Observation, Provenance, Reading, TimestampQuality, Transport, Trust, UnitBasis, READING_SCHEMA,
};
pub use time::UtcNanos;
pub use value::{DecimalValue, DecimalValueError};
pub use world_text::{WorldText, FENCE_CLOSE, FENCE_OPEN};
