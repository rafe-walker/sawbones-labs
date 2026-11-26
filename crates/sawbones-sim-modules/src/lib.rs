//! # sawbones-sim-modules: PROVISIONAL, test support
//!
//! Simulated Sawbones modules that speak `sawbones-module-protocol` over an
//! in-memory link, so every supervisor and interlock test in P0 runs with no
//! hardware attached (#99). Host side, `std`, never published.
//!
//! - [`SimModule`]: a DC meter, a comms pod or an interlock controller
//!   ([`ModuleKind`]), with the fault switches #99 lists ([`ModuleFaults`]).
//! - [`SimLink`]: the line between host and module, with one named
//!   [`Scenario`] on it: bit flips, truncation, loss, duplication,
//!   reordering, delay, babbling, replay (also across a reboot), an
//!   `Identify` replay, an identity that names another module or overclaims
//!   `can_emit`, oversized length fields, a silent or stuck line.
//! - [`HostHarness`]: the host end, with a simulated clock and a deadline for
//!   every call.
//!
//! Nothing here reads a clock, spawns a thread or draws from the operating
//! system: time is a tick the caller passes in, and every random choice comes
//! from a seed the caller passes in. The same seed gives the same run.
//!
//! No key or seed lives in this crate. Tests keep their fixed seeds under
//! `tests/`. These simulated modules carry no real device key and can never
//! attach to a live session: they exist only on the in-memory link.

#![forbid(unsafe_code)]

mod fault;
mod host;
mod kinds;
mod link;
mod module;
mod rng;

pub use fault::{scenarios, Dir, Fault, Scenario};
pub use host::{HostHarness, Outcome, Summary};
pub use kinds::{KindTable, ModuleKind, READING_KEY_BASE};
pub use link::SimLink;
pub use module::{ModuleFaults, Reply, SimModule, TICK_HZ};
pub use rng::{mix, Rng};
