//! The map itself: nodes, edges, rules, switches, and the validation that
//! turns a [`MapSpec`] into an [`IsolationMap`].

use std::collections::BTreeSet;
use std::fmt;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Schema label a [`MapSpec`] must carry. It says "provisional" on purpose:
/// the JSON shape may change once the design settles.
pub const MAP_SCHEMA: &str = "sawbones.isolation-map/0-provisional";

macro_rules! id_type {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            /// The name as a string slice.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl From<&str> for $name {
            fn from(s: &str) -> Self {
                Self(s.to_owned())
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

id_type!(
    /// The name of a [`Node`], unique within a map.
    NodeId
);
id_type!(
    /// The name of an [`Edge`], unique within a map.
    EdgeId
);
id_type!(
    /// The name of a switch: one fact about the bench that can be true or
    /// not, such as "the laptop's charger is plugged in".
    SwitchId
);

/// What a node is, as far as the checks care.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// The protective earth of the building's mains supply.
    MainsEarth,
    /// The computer that runs the supervisor.
    Brain,
    /// A module that measures something outside the bench.
    MeasuringModule,
    /// Anything else with a reference potential.
    #[default]
    Other,
}

/// One thing with a reference potential of its own: a board's ground, a
/// supply's output negative, a device under test, mains earth.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Node {
    /// The node's name.
    pub id: NodeId,
    /// What the node is.
    #[serde(default)]
    pub role: Role,
    /// For a [`Role::MeasuringModule`] only: the module must never share a
    /// ground domain with a [`Role::Brain`] node.
    #[serde(default)]
    pub must_float: bool,
    /// Free text for the operator. The checker ignores it.
    #[serde(default)]
    pub label: String,
}

impl Node {
    /// A node with the given name and role.
    pub fn new(id: &str, role: Role) -> Self {
        Self {
            id: id.into(),
            role,
            must_float: false,
            label: String::new(),
        }
    }

    /// Marks a measuring module as one that must float from the brain.
    #[must_use]
    pub fn must_float(mut self) -> Self {
        self.must_float = true;
        self
    }

    /// Sets the operator's label.
    #[must_use]
    pub fn label(mut self, label: &str) -> Self {
        label.clone_into(&mut self.label);
        self
    }
}

/// How an [`Edge`] joins its two nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Link {
    /// A galvanic connection: a cable's ground wire, a shared supply
    /// negative, a ground clip.
    Bonded,
    /// A barrier: a USB isolator, an isolated converter, a transformer.
    Isolated {
        /// The voltage the barrier is rated to stand, as declared by whoever
        /// wrote the map.
        rating_volts: u32,
        /// The largest voltage difference expected across the barrier, if it
        /// has been declared.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        max_difference_volts: Option<u32>,
    },
}

/// A connection or a barrier between two nodes.
///
/// An edge with a [`switch`](Edge::switch) is a *switched* edge: it exists
/// only while that switch is closed. An edge without one is always there.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Edge {
    /// The edge's name.
    pub id: EdgeId,
    /// One end.
    pub a: NodeId,
    /// The other end.
    pub b: NodeId,
    /// Whether the edge bonds or isolates.
    pub link: Link,
    /// The switch that must be closed for this edge to exist.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub switch: Option<SwitchId>,
    /// Free text for the operator. The checker ignores it.
    #[serde(default)]
    pub note: String,
}

impl Edge {
    fn new(id: &str, a: &str, b: &str, link: Link) -> Self {
        Self {
            id: id.into(),
            a: a.into(),
            b: b.into(),
            link,
            switch: None,
            note: String::new(),
        }
    }

    /// A galvanic connection between `a` and `b`.
    pub fn bonded(id: &str, a: &str, b: &str) -> Self {
        Self::new(id, a, b, Link::Bonded)
    }

    /// A barrier between `a` and `b` rated to `rating_volts`.
    pub fn isolated(id: &str, a: &str, b: &str, rating_volts: u32) -> Self {
        let link = Link::Isolated {
            rating_volts,
            max_difference_volts: None,
        };
        Self::new(id, a, b, link)
    }

    /// Makes the edge exist only while `switch` is closed.
    #[must_use]
    pub fn switched_by(mut self, switch: &str) -> Self {
        self.switch = Some(switch.into());
        self
    }

    /// Declares the largest voltage difference expected across a barrier.
    /// Does nothing to a bonded edge.
    #[must_use]
    pub fn max_difference(mut self, volts: u32) -> Self {
        if let Link::Isolated {
            max_difference_volts,
            ..
        } = &mut self.link
        {
            *max_difference_volts = Some(volts);
        }
        self
    }

    /// Sets the operator's note.
    #[must_use]
    pub fn note(mut self, note: &str) -> Self {
        note.clone_into(&mut self.note);
        self
    }
}

/// A declaration that two nodes must never share a ground domain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IsolationRule {
    /// One node.
    pub a: NodeId,
    /// The other node.
    pub b: NodeId,
}

/// The plain description of a bench: what a JSON map file holds, and what the
/// builder methods assemble. Nothing in it has been checked yet; call
/// [`MapSpec::build`] to get an [`IsolationMap`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MapSpec {
    /// Schema label; must be [`MAP_SCHEMA`].
    pub schema: String,
    /// Everything with a reference potential.
    pub nodes: Vec<Node>,
    /// Every connection and barrier.
    pub edges: Vec<Edge>,
    /// Pairs of nodes that must never share a ground domain.
    #[serde(default)]
    pub must_be_isolated: Vec<IsolationRule>,
    /// The switches that are closed. Every other switch is open.
    #[serde(default)]
    pub closed_switches: BTreeSet<SwitchId>,
}

impl Default for MapSpec {
    fn default() -> Self {
        Self::new()
    }
}

impl MapSpec {
    /// An empty description carrying [`MAP_SCHEMA`].
    pub fn new() -> Self {
        Self {
            schema: MAP_SCHEMA.to_owned(),
            nodes: Vec::new(),
            edges: Vec::new(),
            must_be_isolated: Vec::new(),
            closed_switches: BTreeSet::new(),
        }
    }

    /// Adds a node.
    #[must_use]
    pub fn node(mut self, node: Node) -> Self {
        self.nodes.push(node);
        self
    }

    /// Adds an edge.
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

    /// Closes a switch.
    #[must_use]
    pub fn close(mut self, switch: &str) -> Self {
        self.closed_switches.insert(switch.into());
        self
    }

    /// Validates the description.
    pub fn build(self) -> Result<IsolationMap, MapError> {
        IsolationMap::new(self)
    }
}

/// Why a description was rejected as an [`IsolationMap`].
#[derive(Debug, Error)]
pub enum MapError {
    /// The text was not JSON of the expected shape.
    #[error("map is not valid JSON of the expected shape: {0}")]
    Json(#[from] serde_json::Error),
    /// The schema label was not [`MAP_SCHEMA`].
    #[error("map schema is {found:?}, expected {MAP_SCHEMA:?}")]
    Schema {
        /// The label the description carried.
        found: String,
    },
    /// A node, edge or switch name was empty.
    #[error("a node, edge or switch name is empty")]
    EmptyName,
    /// Two nodes share a name.
    #[error("node {0} is declared twice")]
    DuplicateNode(NodeId),
    /// Two edges share a name.
    #[error("edge {0} is declared twice")]
    DuplicateEdge(EdgeId),
    /// An edge, a rule or a query names a node the map does not have.
    #[error("node {0} is not on the map")]
    UnknownNode(NodeId),
    /// An edge joins a node to itself.
    #[error("edge {0} joins a node to itself")]
    SelfEdge(EdgeId),
    /// An isolation rule names the same node twice.
    #[error("an isolation rule names {0} on both sides")]
    RuleSameNode(NodeId),
    /// A switch is named that no edge uses, which is most likely a typing
    /// mistake.
    #[error("switch {0} is not used by any edge")]
    UnknownSwitch(SwitchId),
    /// `must_float` is set on a node that is not a measuring module.
    #[error("node {0} is marked must_float but is not a measuring module")]
    MustFloatNotModule(NodeId),
}

/// A validated isolation map.
///
/// Every edge and rule names nodes that exist, names are unique, and every
/// closed switch is one some edge uses. The checks live in
/// [`IsolationMap::check`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "MapSpec", into = "MapSpec")]
pub struct IsolationMap {
    spec: MapSpec,
}

impl IsolationMap {
    /// Validates a description.
    pub fn new(spec: MapSpec) -> Result<Self, MapError> {
        if spec.schema != MAP_SCHEMA {
            return Err(MapError::Schema { found: spec.schema });
        }
        let mut nodes = BTreeSet::new();
        for node in &spec.nodes {
            if node.id.as_str().is_empty() {
                return Err(MapError::EmptyName);
            }
            if !nodes.insert(&node.id) {
                return Err(MapError::DuplicateNode(node.id.clone()));
            }
            if node.must_float && node.role != Role::MeasuringModule {
                return Err(MapError::MustFloatNotModule(node.id.clone()));
            }
        }
        let known = |id: &NodeId| {
            if nodes.contains(id) {
                Ok(())
            } else {
                Err(MapError::UnknownNode(id.clone()))
            }
        };
        let mut edges = BTreeSet::new();
        let mut switches = BTreeSet::new();
        for edge in &spec.edges {
            if edge.id.as_str().is_empty() || edge.switch.as_ref().is_some_and(|s| s.0.is_empty()) {
                return Err(MapError::EmptyName);
            }
            if !edges.insert(&edge.id) {
                return Err(MapError::DuplicateEdge(edge.id.clone()));
            }
            known(&edge.a)?;
            known(&edge.b)?;
            if edge.a == edge.b {
                return Err(MapError::SelfEdge(edge.id.clone()));
            }
            switches.extend(edge.switch.as_ref());
        }
        for rule in &spec.must_be_isolated {
            known(&rule.a)?;
            known(&rule.b)?;
            if rule.a == rule.b {
                return Err(MapError::RuleSameNode(rule.a.clone()));
            }
        }
        if let Some(stray) = spec.closed_switches.iter().find(|s| !switches.contains(s)) {
            return Err(MapError::UnknownSwitch(stray.clone()));
        }
        Ok(Self { spec })
    }

    /// Reads and validates a JSON description.
    pub fn from_json(text: &str) -> Result<Self, MapError> {
        Self::new(serde_json::from_str(text)?)
    }

    /// The description this map was built from.
    pub fn spec(&self) -> &MapSpec {
        &self.spec
    }

    /// A copy of this map with one more edge: the way to ask "what if this
    /// were connected?" before anything is wired.
    pub fn with_edge(&self, edge: Edge) -> Result<Self, MapError> {
        Self::new(self.spec.clone().edge(edge))
    }

    /// A copy of this map with one switch closed or opened.
    pub fn with_switch(&self, switch: &str, closed: bool) -> Result<Self, MapError> {
        let mut spec = self.spec.clone();
        if closed {
            spec.closed_switches.insert(switch.into());
        } else {
            spec.closed_switches.remove(&SwitchId::from(switch));
        }
        Self::new(spec)
    }

    /// Whether an edge exists in the current switch configuration.
    pub(crate) fn is_active(&self, edge: &Edge) -> bool {
        edge.switch
            .as_ref()
            .is_none_or(|s| self.spec.closed_switches.contains(s))
    }
}

impl TryFrom<MapSpec> for IsolationMap {
    type Error = MapError;
    fn try_from(spec: MapSpec) -> Result<Self, Self::Error> {
        Self::new(spec)
    }
}

impl From<IsolationMap> for MapSpec {
    fn from(map: IsolationMap) -> Self {
        map.spec
    }
}
