//! # sawbones-solar-assistant: PROVISIONAL
//!
//! A **read-only** tap on the MQTT output of a Solar Assistant unit, turning
//! its messages into [`sawbones_reading::Reading`]s that carry provenance.
//!
//! **Status.** This is the first code in the Sawbones repository. It was
//! written from Solar Assistant's public documentation and from topic listings
//! other users have posted. **It has not yet been run against a real Solar
//! Assistant unit.** `docs/research/solar-assistant-mqtt.md` in the repository
//! lists what was verified and what is assumed. The wider design is under
//! review, so everything here may change.
//!
//! ## What is in the crate
//!
//! - [`parser`]: a pure function of (topic, payload) to a reading or a typed
//!   "ignored" result. No I/O. [`topic`], [`units`] and [`discovery`] are its
//!   parts.
//! - [`client`]: [`client::ReadOnlyMqtt`], an MQTT connection that can
//!   subscribe and receive and has no way to send an application message.
//! - [`ratio`]: the "twin input" ratio monitor, a pure function over two
//!   series of power samples. It reports observations, never diagnoses.
//! - [`summary`]: a streaming summary of a recording (counts, ranges, gaps).
//! - [`raw`]: the raw-capture line format, shared by `sa-tap record
//!   --raw-log` and the synthetic test fixture.
//! - [`backoff`]: reconnect delays.
//!
//! ## Trust
//!
//! The parser builds [`sawbones_reading::Observation`]s, which carry no trust
//! tier. The host attaches one with [`host_reading`], which labels every
//! value [`SOLAR_ASSISTANT_DEFAULT_TRUST`].

#![forbid(unsafe_code)]

pub mod backoff;
pub mod client;
pub mod discovery;
pub mod parser;
pub mod ratio;
pub mod raw;
pub mod summary;
pub mod topic;
pub mod units;

use sawbones_reading::{Observation, Reading, Trust};

/// The trust tier given to every value read from Solar Assistant:
/// [`Trust::Unverified`].
///
/// Solar Assistant is a third party's product reporting a third party's
/// reading of the inverter or battery. This project has not yet cross-checked
/// any of those values against an instrument of its own, so they do not
/// qualify as `Measured`, and nobody has attested to them, so they are not
/// `OperatorAttested` either. They stay `Unverified` until something earns
/// them a higher tier. Do not raise this default to make downstream code
/// more convenient.
pub const SOLAR_ASSISTANT_DEFAULT_TRUST: Trust = Trust::Unverified;

/// **Host side.** Turns a parsed observation into a [`Reading`] at
/// [`SOLAR_ASSISTANT_DEFAULT_TRUST`].
///
/// The parser never sets a tier; this is where the host does. It exists so
/// every host caller of this crate applies the same default.
pub fn host_reading(observation: Observation) -> Reading {
    Reading::from_observation_host_only(observation, SOLAR_ASSISTANT_DEFAULT_TRUST)
}

/// Topic prefix Solar Assistant uses unless the owner changes it.
pub const DEFAULT_TOPIC_PREFIX: &str = "solar_assistant";

/// Topic prefix of Home Assistant discovery messages unless changed.
pub const DEFAULT_DISCOVERY_PREFIX: &str = "homeassistant";
