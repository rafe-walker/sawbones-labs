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
//! is identified again after a poll period; one lost [`MAX_LOSSES`] times
//! within [`LOSS_WINDOW_TICKS`] (for example by repeated `Identify`
//! replays) is quarantined. The supervisor takes at most
//! [`MAX_FRAMES_PER_STEP`] frames from a port per pass, so a flood cannot
//! stall the main loop, and a port that sends more than [`FLOOD_BUDGET`]
//! frames it cannot take in one poll period is quarantined (or refused, if
//! it has proved no identity).
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
//! returning a [`PreflightToken`]. [`Supervisor::latch_check`] checks the
//! taint latch for the open turn, returning a [`LatchPass`].
//! [`Supervisor::invoke`] takes both by value, has
//! [`confirm`](sawbones_policy::confirm) consume the token at the moment of
//! action, checks the latch again, runs the supervisor's gates, and only
//! then hands the bus transport an [`Authorized`] action, which nothing else
//! can make. There is no path that skips the token or the latch: see the
//! `compile_fail` examples on [`Supervisor::invoke`], [`LatchPass`] and
//! [`Authorized`].
//!
//! ## The taint latch
//!
//! Once outside content has entered the agent's context, class 2 and higher
//! actions are refused until a fresh approval seen after it (#210): an arm
//! cycle for class 2, a lease granted after the taint for class 3. Classes 0
//! and 1 pass. The MCP layer opens each turn with [`Supervisor::begin_turn`],
//! reports every input that is not host-authored with
//! [`Supervisor::admit`] or [`Supervisor::taint`] before the model sees it,
//! and gets a [`LatchPass`] from [`Supervisor::latch_check`] for each
//! action; [`Supervisor::invoke`] takes the pass by value and checks again.
//! The latch starts tripped, a new turn inherits it, and nothing but the
//! trust root's reports of those approvals clears it. The contract and its
//! reasons are in the [`latch`] module.
//!
//! Class 2 and 3 also need a connection plan checked by
//! [`Supervisor::arm`]. The supervisor builds the plan's map itself
//! (`sawbones_isolation::build_plan`) from the owner's bench config (fixed
//! wiring, the Mac on its charger or on battery), the terminals each
//! available module declared ([`Supervisor::declare_terminals`]) and the
//! connections proposed; a caller cannot pass a map. A module's declaration
//! is a claim: the stricter of it and the config wins, and a module,
//! terminal or isolation claim without both sides refuses the plan
//! ([`ArmRefusal::Plan`]). The isolation checker then runs on the built map
//! and any finding of severity Stop refuses. A write that goes out with no reply leaves its
//! effect unknown, so its port takes nothing but a read of the same signal
//! until a read back answers ([`Outcome::ReadBack`]).
//!
//! ## Not here yet
//!
//! A re-check when the wiring changes after arming (no event source), and
//! telling the trust root about a refused plan. Also: drivers as child processes, the MCP server (the latch models its turn
//! boundary, but nothing calls it yet), module write commands
//! (config writes and actuations: the policy engine classifies bus requests
//! only, so the supervisor offers none), the manifest pages and their
//! fencing, signed ledger events (#172), USB interface-class authorization
//! (#177), and the module states for self-test, read-only, armed and
//! updating.

#![forbid(unsafe_code)]

mod io;
pub mod latch;
mod registry;
mod supervisor;
mod write;

pub use io::{
    BusReply, BusTransport, ChallengeSource, InterlockLink, InterlockReport, ModuleTransport,
    ToInterlock,
};
pub use latch::{
    FreshApproval, Grounds, LatchPass, LatchRefusal, MeasuredNumber, Origin, Taint, TaintSource,
    TurnId, TurnInput,
};
pub use registry::{
    ConfigError, Enrollment, Happened, HubPort, ModuleState, PortRefusal, PortState,
    QuarantineReason,
};
pub use supervisor::{
    ArmedPlan, Config, Supervisor, FLOOD_BUDGET, LINK_TIMEOUT_TICKS, LOSS_WINDOW_TICKS,
    MAX_FRAMES_PER_STEP, MAX_LOSSES, POLL_PERIOD_TICKS,
};
pub use write::{ArmRefusal, Authorized, InvokeRefusal, Outcome, PendingWrite};

pub use sawbones_interlock::Tick;
pub use sawbones_policy::{EffectRequest, FactEvent, PreflightToken};
