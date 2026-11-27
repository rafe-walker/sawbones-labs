//! Building the connection plan's map (#111): from the bench config (fixed
//! wiring, the Mac's power state) and what each enrolled module declares
//! about its terminals, plus the connections being proposed.
//!
//! The rule that shapes everything here: **a module's declaration is a
//! claim, and the owner's bench config is the other side of it.** Where the
//! two disagree the stricter wins, and where either is missing the plan is
//! refused, because an unknown terminal is not a safe one.
//!
//! - A terminal both sides call isolated is isolated to the lower of the two
//!   ratings.
//! - A terminal either side puts in a ground domain is in that domain. A
//!   module that says "isolated" is not believed over a config entry that
//!   says otherwise, and a module that admits a domain is not talked out of
//!   it by a config that says isolated.
//! - Two different domains for one terminal is a conflict, and a refusal.
//! - A module with no declaration, a terminal the config lists and the module
//!   does not declare, a terminal the module declares and the config does not
//!   list, and an isolation claim the config does not confirm are all
//!   refusals ([`PlanProblem`]).
//!
//! Proposed connections are galvanic ([`Connection`]): the plan has no way
//! to add a barrier. Barriers come from the config.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::map::{
    Edge, IsolationMap, IsolationRule, MapError, MapSpec, Node, NodeId, Role, SwitchId,
};

/// Schema label a [`BenchConfig`] must carry. It says "provisional" on
/// purpose: the JSON shape may change once the design settles.
pub const BENCH_SCHEMA: &str = "sawbones.bench-config/0-provisional";

/// The node the builder adds for the building's protective earth.
pub const MAINS_EARTH: &str = "mains-earth";

/// The node the builder adds for the Mac.
pub const MAC: &str = "mac";

/// The switch the builder owns: closed while the Mac is on its charger.
pub const MAC_ON_CHARGER: &str = "mac-on-charger";

/// A reference potential a terminal can be tied to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GroundDomain {
    /// The negative of a battery pack under test.
    Pack,
    /// A vehicle's chassis.
    VehicleChassis,
    /// The negative of a battery bank.
    BankNegative,
    /// The protective earth of the building's mains.
    MainsEarth,
}

impl GroundDomain {
    /// Every domain, in a fixed order.
    pub const ALL: [GroundDomain; 4] = [
        Self::Pack,
        Self::VehicleChassis,
        Self::BankNegative,
        Self::MainsEarth,
    ];

    /// The node that stands for this domain. Mains earth is the
    /// [`MAINS_EARTH`] node.
    pub fn node(self) -> &'static str {
        match self {
            Self::Pack => "domain:pack",
            Self::VehicleChassis => "domain:vehicle-chassis",
            Self::BankNegative => "domain:bank-negative",
            Self::MainsEarth => MAINS_EARTH,
        }
    }
}

/// What is said about one terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum TerminalClaim {
    /// Galvanically isolated from the module's own ground, to this voltage.
    Isolated {
        /// The voltage the isolation stands.
        rating_volts: u32,
    },
    /// Not isolated: tied to this ground domain, and so to the module's own
    /// ground.
    Ground {
        /// The domain.
        domain: GroundDomain,
    },
}

/// One named terminal and what is said about it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerminalSpec {
    /// The terminal's name, unique on its module. No `/`.
    pub name: String,
    /// What is claimed.
    pub claim: TerminalClaim,
}

impl TerminalSpec {
    /// A terminal claimed isolated to `rating_volts`.
    pub fn isolated(name: &str, rating_volts: u32) -> Self {
        Self {
            name: name.to_owned(),
            claim: TerminalClaim::Isolated { rating_volts },
        }
    }

    /// A terminal tied to `domain`.
    pub fn ground(name: &str, domain: GroundDomain) -> Self {
        Self {
            name: name.to_owned(),
            claim: TerminalClaim::Ground { domain },
        }
    }
}

/// What a module says about its own terminals: a claim, believed no further
/// than the bench config backs it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModuleDeclaration {
    /// Every terminal the module says it has.
    pub terminals: Vec<TerminalSpec>,
}

impl ModuleDeclaration {
    /// A declaration of these terminals.
    pub fn new(terminals: Vec<TerminalSpec>) -> Self {
        Self { terminals }
    }
}

/// What the owner's config says about one enrolled module.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModuleWiring {
    /// The module's id, as enrolled.
    pub id: u32,
    /// The module's node on the map. Unique, no `/`.
    pub name: String,
    /// The module must never share a ground domain with the brain. Defaults
    /// to true: a module is a measuring module until the owner says it is
    /// not.
    #[serde(default = "yes")]
    pub must_float: bool,
    /// Every terminal the module has, as the owner wired it.
    pub terminals: Vec<TerminalSpec>,
}

fn yes() -> bool {
    true
}

/// The Mac's power state: on its charger it is tied to mains earth, on
/// battery it floats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MacPower {
    /// Plugged in: bonded to mains earth through the charger.
    OnCharger,
    /// On its battery: floating.
    OnBattery,
}

/// The owner's description of the fixed bench: what is wired, what each
/// enrolled module's terminals are, and the Mac's state. Read from a JSON
/// file with [`BenchConfig::from_json`] or assembled with the builder
/// methods.
///
/// The builder adds the nodes [`MAINS_EARTH`], [`MAC`] and one per ground
/// domain, and the edges and rules that follow from the Mac's state and the
/// modules' terminals. It also declares every pair of ground domains
/// [`must_be_isolated`](Self::must_be_isolated) from each other: they are
/// different domains by definition, so a path between two is a finding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BenchConfig {
    /// Schema label; must be [`BENCH_SCHEMA`].
    pub schema: String,
    /// Whether the Mac is on its charger.
    pub mac: MacPower,
    /// The fixed points of the bench besides modules, the Mac, mains earth
    /// and the ground domains: the brain, the hub, a truck chassis. At
    /// least one has [`Role::Brain`].
    pub nodes: Vec<Node>,
    /// The fixed wiring, among these nodes, the modules' nodes, the Mac and
    /// the builder's nodes. [`MAC_ON_CHARGER`] is not for the config to use.
    pub edges: Vec<Edge>,
    /// More pairs that must never share a ground domain.
    #[serde(default)]
    pub must_be_isolated: Vec<IsolationRule>,
    /// The fixed wiring's switches that are closed.
    #[serde(default)]
    pub closed_switches: BTreeSet<SwitchId>,
    /// Every enrolled module.
    pub modules: Vec<ModuleWiring>,
}

impl BenchConfig {
    /// A config with the Mac in this state and nothing else.
    pub fn new(mac: MacPower) -> Self {
        Self {
            schema: BENCH_SCHEMA.to_owned(),
            mac,
            nodes: Vec::new(),
            edges: Vec::new(),
            must_be_isolated: Vec::new(),
            closed_switches: BTreeSet::new(),
            modules: Vec::new(),
        }
    }

    /// Reads a JSON config. Nothing in it is checked beyond its shape:
    /// [`build_plan`] checks it every time.
    ///
    /// # Errors
    ///
    /// The JSON error, if the text is not a config.
    pub fn from_json(text: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(text)
    }

    /// Adds a fixed node.
    #[must_use]
    pub fn node(mut self, node: Node) -> Self {
        self.nodes.push(node);
        self
    }

    /// Adds a piece of fixed wiring.
    #[must_use]
    pub fn edge(mut self, edge: Edge) -> Self {
        self.edges.push(edge);
        self
    }

    /// Declares that `a` and `b` must never share a ground domain.
    #[must_use]
    pub fn must_be_isolated(mut self, a: &str, b: &str) -> Self {
        self.must_be_isolated.push(IsolationRule {
            a: a.into(),
            b: b.into(),
        });
        self
    }

    /// Closes a switch of the fixed wiring.
    #[must_use]
    pub fn close(mut self, switch: &str) -> Self {
        self.closed_switches.insert(switch.into());
        self
    }

    /// Adds an enrolled module that must float, with its terminals.
    #[must_use]
    pub fn module(mut self, id: u32, name: &str, terminals: Vec<TerminalSpec>) -> Self {
        self.modules.push(ModuleWiring {
            id,
            name: name.to_owned(),
            must_float: true,
            terminals,
        });
        self
    }

    /// Sets the Mac's state.
    #[must_use]
    pub fn mac_power(mut self, mac: MacPower) -> Self {
        self.mac = mac;
        self
    }
}

/// One proposed galvanic connection between two nodes of the bench: a lead,
/// a clip, a USB cable's ground. An endpoint is a node name from the config
/// or the builder (`mac`, `mains-earth`, `domain:pack`) or a module terminal
/// written `module/terminal`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Connection {
    /// One end.
    pub a: String,
    /// The other end.
    pub b: String,
}

impl Connection {
    /// A connection between `a` and `b`.
    pub fn new(a: &str, b: &str) -> Self {
        Self {
            a: a.to_owned(),
            b: b.to_owned(),
        }
    }
}

/// One reason the plan's map could not be built. Every one of them refuses
/// the plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Error)]
#[serde(tag = "problem", rename_all = "snake_case")]
pub enum PlanProblem {
    /// An enrolled module has no declaration: its terminals are unknown.
    #[error("module {module} has no declaration, so its terminals are unknown")]
    NoDeclaration {
        /// The module's name in the config.
        module: String,
    },
    /// The config lists a terminal the module did not declare.
    #[error("module {module} did not declare terminal {terminal}, which the config lists")]
    UndeclaredTerminal {
        /// The module.
        module: String,
        /// The terminal.
        terminal: String,
    },
    /// The module declared a terminal the config does not list: the config
    /// has not confirmed it, and an isolation claim is not believed alone.
    #[error("module {module} declared terminal {terminal}, which the config does not list")]
    UnconfirmedTerminal {
        /// The module.
        module: String,
        /// The terminal.
        terminal: String,
    },
    /// The module and the config put one terminal in different ground
    /// domains.
    #[error(
        "terminal {module}/{terminal} is in {config:?} by the config but {claimed:?} by the module"
    )]
    ConflictingDomains {
        /// The module.
        module: String,
        /// The terminal.
        terminal: String,
        /// The config's domain.
        config: GroundDomain,
        /// The module's domain.
        claimed: GroundDomain,
    },
    /// A terminal is named twice on one module, in the config or in the
    /// declaration.
    #[error("module {module} names terminal {terminal} twice")]
    DuplicateTerminal {
        /// The module.
        module: String,
        /// The terminal.
        terminal: String,
    },
    /// A proposed connection names something that is not on the bench.
    #[error("connection endpoint {endpoint} is not on the bench")]
    UnknownEndpoint {
        /// The endpoint as written.
        endpoint: String,
    },
    /// The config or the connections do not make a valid map: a duplicate
    /// or empty name, a self-connection, a reserved name, no brain.
    #[error("the bench config or plan is not a valid map: {why}")]
    Invalid {
        /// What is wrong.
        why: String,
    },
}

/// The plan's map could not be built, for these reasons.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Error)]
#[error("the connection plan cannot be built: {} problem(s)", problems.len())]
pub struct PlanRefusal {
    /// Every reason found, in a fixed order.
    pub problems: Vec<PlanProblem>,
}

fn invalid(why: impl Into<String>) -> PlanProblem {
    PlanProblem::Invalid { why: why.into() }
}

fn from_map_error(error: &MapError) -> PlanProblem {
    invalid(error.to_string())
}

/// What a terminal is, once the config and the module's claim are put
/// together.
fn merge(
    module: &str,
    terminal: &str,
    config: TerminalClaim,
    claimed: TerminalClaim,
) -> Result<TerminalClaim, PlanProblem> {
    use TerminalClaim::{Ground, Isolated};
    match (config, claimed) {
        (Isolated { rating_volts: a }, Isolated { rating_volts: b }) => Ok(Isolated {
            rating_volts: a.min(b),
        }),
        (Ground { domain: a }, Ground { domain: b }) if a == b => Ok(Ground { domain: a }),
        (Ground { domain: a }, Ground { domain: b }) => Err(PlanProblem::ConflictingDomains {
            module: module.to_owned(),
            terminal: terminal.to_owned(),
            config: a,
            claimed: b,
        }),
        // Either side putting the terminal in a domain puts it there: a
        // module's "isolated" is not believed over the config, and the
        // config's "isolated" does not talk a module out of a domain.
        (Ground { domain }, Isolated { .. }) | (Isolated { .. }, Ground { domain }) => {
            Ok(Ground { domain })
        }
    }
}

fn duplicate_terminal(module: &str, specs: &[TerminalSpec], out: &mut Vec<PlanProblem>) -> bool {
    let mut seen = BTreeSet::new();
    let mut found = false;
    for t in specs {
        if !seen.insert(t.name.as_str()) {
            found = true;
            out.push(PlanProblem::DuplicateTerminal {
                module: module.to_owned(),
                terminal: t.name.clone(),
            });
        }
    }
    found
}

/// Builds the map of the connection plan from the bench config, the
/// modules' declarations (by module id: pass only the modules whose
/// declaration is currently vouched for) and the proposed connections.
///
/// The result is an [`IsolationMap`] ready for [`IsolationMap::check`].
/// Nothing the caller passes can make it more permissive than the config:
/// connections are galvanic only, and a declaration can only add to what the
/// config says, never take away.
///
/// # Errors
///
/// A [`PlanRefusal`] with every [`PlanProblem`] found.
#[allow(clippy::too_many_lines)]
pub fn build_plan(
    config: &BenchConfig,
    declarations: &BTreeMap<u32, ModuleDeclaration>,
    connections: &[Connection],
) -> Result<IsolationMap, PlanRefusal> {
    let mut problems = Vec::new();
    if config.schema != BENCH_SCHEMA {
        problems.push(invalid(format!(
            "bench config schema is {:?}, expected {BENCH_SCHEMA:?}",
            config.schema
        )));
    }
    if config
        .closed_switches
        .contains(&SwitchId::from(MAC_ON_CHARGER))
        || config.edges.iter().any(|e| {
            e.switch
                .as_ref()
                .is_some_and(|s| s.as_str() == MAC_ON_CHARGER)
        })
    {
        problems.push(invalid(format!(
            "{MAC_ON_CHARGER} is the builder's switch: the Mac's state comes from `mac`"
        )));
    }
    if !config.nodes.iter().any(|n| n.role == Role::Brain) {
        problems.push(invalid("the config has no brain node"));
    }

    let mut spec = MapSpec::new();
    spec.nodes.push(Node::new(MAINS_EARTH, Role::MainsEarth));
    spec.nodes.push(Node::new(MAC, Role::Other));
    for domain in GroundDomain::ALL {
        if domain != GroundDomain::MainsEarth {
            spec.nodes.push(Node::new(domain.node(), Role::Other));
        }
    }
    spec.nodes.extend(config.nodes.iter().cloned());
    spec.edges.extend(config.edges.iter().cloned());
    spec.edges
        .push(Edge::bonded("mac-charger", MAC, MAINS_EARTH).switched_by(MAC_ON_CHARGER));
    spec.closed_switches = config.closed_switches.clone();
    if config.mac == MacPower::OnCharger {
        spec.closed_switches.insert(MAC_ON_CHARGER.into());
    }

    // Every pair of domains is a pair that must not meet.
    let domains = GroundDomain::ALL;
    for (i, a) in domains.iter().enumerate() {
        for b in &domains[i + 1..] {
            spec.must_be_isolated.push(IsolationRule {
                a: a.node().into(),
                b: b.node().into(),
            });
        }
    }
    for rule in &config.must_be_isolated {
        let known = |r: &IsolationRule| {
            spec.must_be_isolated
                .iter()
                .any(|k| (k.a == r.a && k.b == r.b) || (k.a == r.b && k.b == r.a))
        };
        if !known(rule) {
            spec.must_be_isolated.push(rule.clone());
        }
    }

    let mut module_ids = BTreeSet::new();
    for wiring in &config.modules {
        let name = wiring.name.as_str();
        if name.contains('/') {
            problems.push(invalid(format!("module name {name:?} has a /")));
        }
        if !module_ids.insert(wiring.id) {
            problems.push(invalid(format!(
                "module id {} is in the config twice",
                wiring.id
            )));
        }
        let mut node = Node::new(name, Role::MeasuringModule);
        node.must_float = wiring.must_float;
        spec.nodes.push(node);

        let Some(declaration) = declarations.get(&wiring.id) else {
            problems.push(PlanProblem::NoDeclaration {
                module: name.to_owned(),
            });
            continue;
        };
        let dup_config = duplicate_terminal(name, &wiring.terminals, &mut problems);
        let dup_claim = duplicate_terminal(name, &declaration.terminals, &mut problems);
        if dup_config || dup_claim {
            continue;
        }
        let claimed: BTreeMap<&str, TerminalClaim> = declaration
            .terminals
            .iter()
            .map(|t| (t.name.as_str(), t.claim))
            .collect();
        for terminal in &wiring.terminals {
            let t = terminal.name.as_str();
            if t.contains('/') {
                problems.push(invalid(format!("terminal name {t:?} on {name} has a /")));
                continue;
            }
            let Some(claim) = claimed.get(t) else {
                problems.push(PlanProblem::UndeclaredTerminal {
                    module: name.to_owned(),
                    terminal: t.to_owned(),
                });
                continue;
            };
            let merged = match merge(name, t, terminal.claim, *claim) {
                Ok(m) => m,
                Err(p) => {
                    problems.push(p);
                    continue;
                }
            };
            let node = format!("{name}/{t}");
            spec.nodes.push(Node::new(&node, Role::Other));
            match merged {
                TerminalClaim::Isolated { rating_volts: 0 } => {
                    problems.push(invalid(format!("terminal {node} is isolated to 0 V")));
                }
                TerminalClaim::Isolated { rating_volts } => spec.edges.push(Edge::isolated(
                    &format!("{node}-barrier"),
                    name,
                    &node,
                    rating_volts,
                )),
                TerminalClaim::Ground { domain } => {
                    spec.edges
                        .push(Edge::bonded(&format!("{node}-module"), name, &node));
                    spec.edges.push(Edge::bonded(
                        &format!("{node}-ground"),
                        &node,
                        domain.node(),
                    ));
                }
            }
        }
        let listed: BTreeSet<&str> = wiring.terminals.iter().map(|t| t.name.as_str()).collect();
        for terminal in &declaration.terminals {
            if !listed.contains(terminal.name.as_str()) {
                problems.push(PlanProblem::UnconfirmedTerminal {
                    module: name.to_owned(),
                    terminal: terminal.name.clone(),
                });
            }
        }
    }

    let nodes: BTreeSet<&NodeId> = spec.nodes.iter().map(|n| &n.id).collect();
    for (i, c) in connections.iter().enumerate() {
        let mut ok = true;
        for end in [&c.a, &c.b] {
            if !nodes.contains(&NodeId::from(end.as_str())) {
                ok = false;
                problems.push(PlanProblem::UnknownEndpoint {
                    endpoint: end.clone(),
                });
            }
        }
        if ok {
            spec.edges
                .push(Edge::bonded(&format!("plan-{}", i + 1), &c.a, &c.b));
        }
    }

    if !problems.is_empty() {
        return Err(PlanRefusal { problems });
    }
    spec.build().map_err(|e| PlanRefusal {
        problems: vec![from_map_error(&e)],
    })
}
