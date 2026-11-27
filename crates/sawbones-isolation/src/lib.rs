//! # sawbones-isolation: PROVISIONAL
//!
//! **This crate is provisional. The Sawbones design is still under review, and
//! every type here may change or be replaced without notice.**
//!
//! The isolation map: a typed graph that says how everything on the bench is
//! electrically connected, and a checker that finds unintended ground paths
//! before anything is wired.
//!
//! - A [`Node`] is something with a reference potential of its own: mains
//!   earth, the brain's ground, a module's ground, the device under test.
//! - An [`Edge`] either bonds two nodes ([`Link::Bonded`]) or is a barrier
//!   between them ([`Link::Isolated`]). An edge with a switch exists only
//!   while that switch is closed, which is how "the laptop's charger is
//!   plugged in" gets onto the map.
//! - A ground domain is a set of nodes joined by bonded edges.
//!
//! [`IsolationMap::check`] returns a [`Report`] of typed [`Finding`]s: a declared
//! isolation that a bonded path defeats, a ground loop, a barrier rated below
//! the voltage declared across it, what is bonded to mains earth, a
//! measuring module that must float but is bonded to the brain, and a bus
//! with two masters. Each finding has a [`Severity`]: `Stop` for a broken
//! rule, `Warn` for a loop that stays inside a harmless domain, `Info` for
//! what is earthed. A map
//! is written as JSON ([`IsolationMap::from_json`]) or assembled in code:
//!
//! ```
//! use sawbones_isolation::{Edge, Finding, MapSpec, Node, Role};
//!
//! let map = MapSpec::new()
//!     .node(Node::new("pi", Role::Brain))
//!     .node(Node::new("hub", Role::Other))
//!     .node(Node::new("meter", Role::MeasuringModule).must_float())
//!     .edge(Edge::bonded("pi-usb", "pi", "hub"))
//!     .edge(Edge::isolated("usb-isolator", "hub", "meter", 1000))
//!     .build()
//!     .unwrap();
//! assert!(map.check().is_clean());
//!
//! // What if a ground clip joined the meter to the hub?
//! let plan = map.with_edge(Edge::bonded("clip", "hub", "meter")).unwrap();
//! let report = plan.check();
//! assert!(matches!(
//!     report.violations().next(),
//!     Some(Finding::ModuleNotFloating { .. })
//! ));
//! ```
//!
//! [`build_plan`] builds the map of a connection plan from the owner's
//! [`BenchConfig`] (fixed wiring, the Mac on its charger or on battery), the
//! terminals each module declares, and the connections proposed. A module's
//! declaration is a claim: the stricter of it and the config wins, and a
//! missing declaration refuses the plan.
//!
//! What this crate does not know: anything the map does not say. It checks a
//! description, not the bench. A connection nobody declared is invisible to
//! it, and a rating is only the number somebody typed.

#![forbid(unsafe_code)]

mod check;
mod map;
mod plan;

pub use check::{Finding, Path, Report, Severity, Step};
pub use map::{
    BusMaster, Edge, EdgeId, IsolationMap, IsolationRule, Link, MapError, MapSpec, Node, NodeId,
    Role, SwitchId, MAP_SCHEMA,
};
pub use plan::{
    build_plan, BenchConfig, Connection, GroundDomain, MacPower, ModuleDeclaration, ModuleWiring,
    PlanProblem, PlanRefusal, TerminalClaim, TerminalSpec, BENCH_SCHEMA, MAC, MAC_ON_CHARGER,
    MAINS_EARTH,
};
