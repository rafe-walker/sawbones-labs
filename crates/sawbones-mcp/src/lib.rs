//! # sawbones-mcp: PROVISIONAL
//!
//! **This crate is provisional. The Sawbones design is still under review,
//! and every type here may change or be replaced without notice.**
//!
//! The MCP surface v0 (#103): the agent's way onto the bench, served over
//! stdio on top of [`sawbones_supervisor`] and the simulated bench
//! ([`sim::SimBench`]).
//!
//! ## Tools named by hazard class
//!
//! Tool names are split by hazard class ([`ToolClass`]: read,
//! `invoke_safe`, `invoke_armed`, `invoke_hazard`) so a client's per-tool
//! permission rules land prompts only where the danger is. **v0 registers
//! only the read class**: `bench_status`, `inventory`, `describe`, `read`
//! and `session_query`. A constant assertion fails the build if [`TOOLS`]
//! holds anything else, and the reserved invoke names
//! ([`RESERVED_INVOKE_NAMES`]) are unknown tools. The definitions are
//! constants: nothing from a module or the session reaches a name, a
//! description or a schema, and no input field carries a class, a tier, an
//! approval or a lease.
//!
//! ## Results
//!
//! Every result is a host-written summary, a handle to stored data
//! ([`store::Store`]) and, where the tool returns module text, that text
//! fenced as untrusted data under a byte budget. Never raw samples: `read`
//! returns statistics and a handle, and `session_query` returns statistics
//! over a window of a handle.
//!
//! ## The taint latch
//!
//! Each `tools/call` is one turn: [`Supervisor::begin_turn`] before the tool
//! runs and [`Supervisor::end_turn`] once its result is made. A result can
//! take module content only through a builder that reports it to the latch
//! first ([`Supervisor::taint`] with [`Origin::Device`]): manifest text,
//! the values a module claims and the refusal codes it sends. Host-written
//! lines (enrollment, supervisor state, counts, handles, ticks) do not trip
//! it. Tool arguments never change a class, a lease or a permission: v0 has
//! no tool that could.
//!
//! [`Supervisor::begin_turn`]: sawbones_supervisor::Supervisor::begin_turn
//! [`Supervisor::end_turn`]: sawbones_supervisor::Supervisor::end_turn
//! [`Supervisor::taint`]: sawbones_supervisor::Supervisor::taint
//! [`Origin::Device`]: sawbones_supervisor::Origin::Device
//!
//! ## Why not `rmcp`
//!
//! The official Rust SDK builds on the pinned toolchain and passes
//! cargo-deny's licence and advisory checks, but it adds 33 crates, six of
//! them with build scripts and seven proc macros, and its own `build.rs`
//! runs `git config core.hooksPath .githooks` in the directory two levels
//! above its manifest when `.git` and `.githooks` exist there, which is the
//! repository root when dependencies are vendored. That cannot be
//! allow-listed under the build-code rule. The subset v0 needs is
//! JSON-RPC over lines with four methods, written here on `serde_json`.
//!
//! ## Not here
//!
//! Transport authentication (#211): anyone who can start the process or
//! reach its stdio can read the bench. The invoke tools. `plugin_status`,
//! `plugin_logs`, `compute`, `explain`, `ask_operator` and `wait_for`.
//! Structured content. A real bench: v0 serves only the simulation.

#![forbid(unsafe_code)]

mod envelope;
mod server;
pub mod sim;
pub mod store;
mod tools;

pub use envelope::{HOST_BUDGET_BYTES, UNTRUSTED_BUDGET_BYTES};
pub use server::{
    initialize_result, serve, Server, INSTRUCTIONS, MAX_MESSAGE_BYTES, PROTOCOL_VERSIONS,
    SERVER_NAME,
};
pub use tools::{find, list, ToolClass, ToolDef, MAX_SAMPLES, RESERVED_INVOKE_NAMES, TOOLS};
