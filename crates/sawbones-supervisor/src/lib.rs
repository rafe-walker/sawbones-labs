//! # sawbones-supervisor: PROVISIONAL
//!
//! **This crate is provisional. The Sawbones design is still under review, and
//! every type here may change or be replaced without notice.**
//!
//! The skeleton of the host-side supervisor (#100): the process on the
//! Raspberry Pi that talks to modules over `sawbones-module-protocol`, runs
//! the policy engine and its preflight, and answers the interlock's
//! heartbeat. **The Pi carries data but does not vouch for it**: the trust
//! root and the modules check their own rules whatever the supervisor
//! believes, and the supervisor's gates only refuse sooner.
//!
//! There is no hardware I/O here and no clock. Every link is a trait
//! ([`ModuleTransport`], [`InterlockLink`], [`BusTransport`],
//! [`ChallengeSource`]) so tests use in-memory fakes, and time is the
//! [`Tick`] passed to [`Supervisor::step`].
//!
//! ## The registry
//!
//! A module appearing on USB grants nothing. Each [`Enrollment`] binds a
//! module id to its hub port, its key, its pinned manifest digest and the
//! most it may claim. When a device answers an identify, its signature is
//! checked against the enrolled key first, so a device that cannot sign can
//! only get itself refused ([`PortRefusal`]). A module that proves its id
//! and then turns out to be on another hub port, on two ports at once, with
//! signed capabilities beyond its enrollment (such as `can_emit`), or with a
//! manifest other than the pinned one is quarantined
//! ([`QuarantineReason`]): every port holding it is told to go to its safe
//! state, its facts are invalidated, and it stays out until a person
//! re-enrolls it ([`Supervisor::re_enroll`]).
//!
//! ## Deadlines
//!
//! Each request has [`LINK_TIMEOUT_TICKS`] to be answered; a verified
//! module is polled every [`POLL_PERIOD_TICKS`]. A module that misses its
//! deadline, or is unplugged, is lost, and its facts are invalidated
//! through the policy's [`FactLedger`](sawbones_policy::FactLedger): every
//! fact on the bus ports it carries ([`FactEvent::PortRewired`]) and every
//! fact it measured ([`FactEvent::ModuleReflashed`], the nearest event the
//! ledger has). A dependent write's preflight then fails, naming the fact.
//! An invalidation is sticky: the facts must be earned again. A lost device
//! is identified again after a poll period.
//!
//! ## The heartbeat
//!
//! [`Supervisor::step`] answers the trust root's challenge, once each, at
//! the start of every pass. Nothing else in the crate calls
//! [`InterlockLink::reply`], so a main loop that stalls stops the heartbeat
//! and the trust root drops every rail within its budget
//! (`2 * HEARTBEAT_PERIOD_TICKS` of the last good reply).
//!
//! ## One write path
//!
//! [`Supervisor::preflight`] checks the facts and classifies the request,
//! returning a [`PreflightToken`]. [`Supervisor::invoke`] takes the token by
//! value, has [`confirm`](sawbones_policy::confirm) consume it at the moment
//! of action, runs the supervisor's gates, and only then hands the bus
//! transport an [`Authorized`] action, which nothing else can make. There is
//! no path that skips the token: see the `compile_fail` examples on
//! [`Supervisor::invoke`] and [`Authorized`].
//!
//! Class 2 and 3 also need a connection plan checked by
//! [`Supervisor::arm`]: the isolation checker runs on it and any finding of
//! severity Stop refuses. A write that goes out with no reply leaves its
//! effect unknown, so its port takes nothing but a read of the same signal
//! until a read back answers ([`Outcome::ReadBack`]).
//!
//! ## Not here yet
//!
//! Drivers as child processes, the MCP server, module write commands
//! (config writes and actuations: the policy engine classifies bus requests
//! only, so the supervisor offers none), the manifest pages and their
//! fencing, signed ledger events (#172), USB interface-class authorization
//! (#177), and the module states for self-test, read-only, armed and
//! updating.

#![forbid(unsafe_code)]

mod io;
mod registry;
mod supervisor;
mod write;

pub use io::{
    BusReply, BusTransport, ChallengeSource, InterlockLink, InterlockReport, ModuleTransport,
    ToInterlock,
};
pub use registry::{
    ConfigError, Enrollment, Happened, HubPort, ModuleState, PortRefusal, PortState,
    QuarantineReason,
};
pub use supervisor::{ArmedPlan, Config, Supervisor, LINK_TIMEOUT_TICKS, POLL_PERIOD_TICKS};
pub use write::{ArmRefusal, Authorized, InvokeRefusal, Outcome, PendingWrite};

pub use sawbones_interlock::Tick;
pub use sawbones_policy::{EffectRequest, FactEvent, PreflightToken};
