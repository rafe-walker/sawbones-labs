//! The checker: ground domains, bonded paths and typed findings.

use std::collections::VecDeque;
use std::fmt;

use serde::Serialize;

use crate::map::{EdgeId, IsolationMap, Link, MapError, NodeId, Role};

/// One hop of a [`Path`]: the edge taken and the node it leads to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Step {
    /// The bonded edge crossed.
    pub via: EdgeId,
    /// The node reached.
    pub to: NodeId,
}

/// A walk over bonded edges from one node to another: the evidence attached
/// to a finding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Path {
    /// Where the walk starts.
    pub start: NodeId,
    /// Each hop in order.
    pub steps: Vec<Step>,
}

impl Path {
    /// Every node on the path, the start included.
    pub fn nodes(&self) -> impl Iterator<Item = &NodeId> {
        std::iter::once(&self.start).chain(self.steps.iter().map(|s| &s.to))
    }

    /// Every edge on the path, in order.
    pub fn edges(&self) -> impl Iterator<Item = &EdgeId> {
        self.steps.iter().map(|s| &s.via)
    }
}

impl fmt::Display for Path {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.start)?;
        for step in &self.steps {
            write!(f, " -[{}]- {}", step.via, step.to)?;
        }
        Ok(())
    }
}

/// How seriously a [`Finding`] should be taken.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    /// Information for the operator, such as what is bonded to mains earth.
    Info,
    /// Worth a look, but it breaks no declared rule: a ground loop entirely
    /// inside a domain that holds nothing the map protects.
    Warn,
    /// A declared rule is broken. A connection plan with one must not go
    /// ahead.
    Stop,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Info => "note",
            Self::Warn => "WARN",
            Self::Stop => "STOP",
        })
    }
}

/// One thing the checker found.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "finding", rename_all = "snake_case")]
pub enum Finding {
    /// Two nodes declared `must_be_isolated` share a ground domain.
    IsolationBroken {
        /// One node of the rule.
        a: NodeId,
        /// The other node of the rule.
        b: NodeId,
        /// The shortest bonded path from `a` to `b`.
        path: Path,
    },
    /// Two nodes are joined by more than one bonded path: a ground loop.
    GroundLoop {
        /// One end of the edge that closes the loop.
        a: NodeId,
        /// The other end of that edge.
        b: NodeId,
        /// The edge that closes the loop, as a path of one hop from `a` to
        /// `b`.
        first: Path,
        /// Another bonded path from `a` to `b` that shares no edge with
        /// `first`.
        second: Path,
        /// [`Severity::Stop`] if the loop touches mains earth, a brain, a
        /// module that must float, or a node named in an isolation rule;
        /// otherwise [`Severity::Warn`].
        severity: Severity,
    },
    /// A barrier is rated below the voltage difference declared across it.
    /// Reported whether or not the barrier's switch is closed.
    UnderratedBarrier {
        /// The barrier.
        edge: EdgeId,
        /// What it is rated to stand.
        rating_volts: u32,
        /// The largest difference declared across it.
        max_difference_volts: u32,
    },
    /// A domain contains mains earth. This is information for the operator,
    /// not a violation.
    EarthedDomain {
        /// The mains earth node.
        earth: NodeId,
        /// Every other node bonded to it. Empty when nothing is earthed.
        members: Vec<NodeId>,
    },
    /// A measuring module that must float shares a ground domain with the
    /// brain.
    ModuleNotFloating {
        /// The module.
        module: NodeId,
        /// The brain it is bonded to.
        brain: NodeId,
        /// The shortest bonded path from the module to the brain.
        path: Path,
    },
    /// A bus has two or more active masters.
    BusMastership {
        /// The bus.
        bus: NodeId,
        /// Every active master, in declaration order.
        masters: Vec<NodeId>,
    },
}

impl Finding {
    /// How seriously to take this finding.
    pub fn severity(&self) -> Severity {
        match self {
            Self::EarthedDomain { .. } => Severity::Info,
            Self::GroundLoop { severity, .. } => *severity,
            Self::IsolationBroken { .. }
            | Self::UnderratedBarrier { .. }
            | Self::ModuleNotFloating { .. }
            | Self::BusMastership { .. } => Severity::Stop,
        }
    }

    /// Whether this finding should stop a connection plan: its severity is
    /// [`Severity::Stop`].
    pub fn is_violation(&self) -> bool {
        self.severity() == Severity::Stop
    }
}

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IsolationBroken { a, b, path } => {
                write!(f, "{a} and {b} must be isolated but are bonded: {path}")
            }
            Self::GroundLoop {
                a,
                b,
                first,
                second,
                ..
            } => write!(f, "ground loop between {a} and {b}: {first} and {second}"),
            Self::UnderratedBarrier {
                edge,
                rating_volts,
                max_difference_volts,
            } => write!(
                f,
                "barrier {edge} is rated {rating_volts} V but may see {max_difference_volts} V"
            ),
            Self::EarthedDomain { earth, members } if members.is_empty() => {
                write!(f, "nothing is bonded to {earth}")
            }
            Self::EarthedDomain { earth, members } => {
                write!(f, "bonded to {earth}:")?;
                members.iter().try_for_each(|m| write!(f, " {m}"))
            }
            Self::ModuleNotFloating {
                module,
                brain,
                path,
            } => write!(f, "{module} must float but is bonded to {brain}: {path}"),
            Self::BusMastership { bus, masters } => {
                write!(f, "bus {bus} has {} masters:", masters.len())?;
                masters.iter().try_for_each(|m| write!(f, " {m}"))
            }
        }
    }
}

/// The result of [`IsolationMap::check`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Report {
    /// The ground domains: each inner list is one set of nodes joined by
    /// bonded edges, in the order the map declares them.
    pub domains: Vec<Vec<NodeId>>,
    /// Everything found, grouped by check.
    pub findings: Vec<Finding>,
}

impl Report {
    /// The findings that should stop a connection plan: those with
    /// [`Severity::Stop`].
    pub fn violations(&self) -> impl Iterator<Item = &Finding> {
        self.findings.iter().filter(|f| f.is_violation())
    }

    /// The findings with [`Severity::Warn`].
    pub fn warnings(&self) -> impl Iterator<Item = &Finding> {
        self.findings
            .iter()
            .filter(|f| f.severity() == Severity::Warn)
    }

    /// Whether there are no violations.
    pub fn is_clean(&self) -> bool {
        self.violations().next().is_none()
    }
}

/// The bonded edges that exist in the current switch configuration, as
/// adjacency lists over node indices. Each entry is (neighbour, edge index).
struct Bonds<'a> {
    map: &'a IsolationMap,
    adjacent: Vec<Vec<(usize, usize)>>,
}

impl<'a> Bonds<'a> {
    fn new(map: &'a IsolationMap) -> Self {
        let spec = map.spec();
        let mut adjacent = vec![Vec::new(); spec.nodes.len()];
        for (e, edge) in spec.edges.iter().enumerate() {
            if edge.link == Link::Bonded && map.is_active(edge) {
                // Validation guarantees both ends exist.
                if let (Some(a), Some(b)) = (map.index(&edge.a), map.index(&edge.b)) {
                    adjacent[a].push((b, e));
                    adjacent[b].push((a, e));
                }
            }
        }
        Self { map, adjacent }
    }

    /// Breadth-first search from `from` over the edges `allowed` accepts.
    /// Returns, for each node reached, the node and edge it was reached by.
    fn reach(&self, from: usize, allowed: impl Fn(usize) -> bool) -> Vec<Option<(usize, usize)>> {
        let mut came_from = vec![None; self.adjacent.len()];
        let mut seen = vec![false; self.adjacent.len()];
        seen[from] = true;
        let mut queue = VecDeque::from([from]);
        while let Some(node) = queue.pop_front() {
            for &(next, edge) in &self.adjacent[node] {
                if !seen[next] && allowed(edge) {
                    seen[next] = true;
                    came_from[next] = Some((node, edge));
                    queue.push_back(next);
                }
            }
        }
        came_from
    }

    /// The shortest path from `from` to `to` over the edges `allowed`
    /// accepts, if there is one.
    fn path(&self, from: usize, to: usize, allowed: impl Fn(usize) -> bool) -> Option<Path> {
        let spec = self.map.spec();
        let came_from = self.reach(from, allowed);
        let mut steps = Vec::new();
        let mut at = to;
        while at != from {
            let (previous, edge) = came_from[at]?;
            steps.push(Step {
                via: spec.edges[edge].id.clone(),
                to: spec.nodes[at].id.clone(),
            });
            at = previous;
        }
        steps.reverse();
        Some(Path {
            start: spec.nodes[from].id.clone(),
            steps,
        })
    }

    /// The connected components, each as node indices in declaration order.
    fn domains(&self) -> Vec<Vec<usize>> {
        let mut assigned = vec![false; self.adjacent.len()];
        let mut domains = Vec::new();
        for start in 0..self.adjacent.len() {
            if assigned[start] {
                continue;
            }
            let came_from = self.reach(start, |_| true);
            let members: Vec<usize> = (0..self.adjacent.len())
                .filter(|&n| n == start || came_from[n].is_some())
                .collect();
            members.iter().for_each(|&n| assigned[n] = true);
            domains.push(members);
        }
        domains
    }
}

impl IsolationMap {
    fn index(&self, id: &NodeId) -> Option<usize> {
        self.spec().nodes.iter().position(|n| &n.id == id)
    }

    fn require(&self, id: &str) -> Result<usize, MapError> {
        let id = NodeId::from(id);
        self.index(&id).ok_or(MapError::UnknownNode(id))
    }

    /// The ground domains in the current switch configuration: the sets of
    /// nodes joined by bonded edges. A node bonded to nothing is a domain of
    /// one.
    pub fn domains(&self) -> Vec<Vec<NodeId>> {
        let nodes = &self.spec().nodes;
        let name = |d: Vec<usize>| d.into_iter().map(|n| nodes[n].id.clone()).collect();
        Bonds::new(self).domains().into_iter().map(name).collect()
    }

    /// The shortest bonded path from `a` to `b`, or `None` if they are in
    /// different ground domains. Naming a node the map does not have is an
    /// error, so a mistyped name cannot read as "isolated".
    pub fn bonded_path(&self, a: &str, b: &str) -> Result<Option<Path>, MapError> {
        let (a, b) = (self.require(a)?, self.require(b)?);
        Ok(Bonds::new(self).path(a, b, |_| true))
    }

    /// Runs every check against the current switch configuration.
    pub fn check(&self) -> Report {
        let spec = self.spec();
        let bonds = Bonds::new(self);
        let domains = bonds.domains();
        let id = |n: usize| spec.nodes[n].id.clone();
        let mut findings = Vec::new();

        // 1. Declared isolation that a bonded path defeats.
        for rule in &spec.must_be_isolated {
            if let (Some(a), Some(b)) = (self.index(&rule.a), self.index(&rule.b)) {
                if let Some(path) = bonds.path(a, b, |_| true) {
                    findings.push(Finding::IsolationBroken {
                        a: rule.a.clone(),
                        b: rule.b.clone(),
                        path,
                    });
                }
            }
        }

        // 2. Ground loops. Edges are taken in the order the map declares
        // them; one that joins two nodes already joined closes a loop.
        let mut kept = vec![false; spec.edges.len()];
        for (e, edge) in spec.edges.iter().enumerate() {
            if edge.link != Link::Bonded || !self.is_active(edge) {
                continue;
            }
            let (Some(a), Some(b)) = (self.index(&edge.a), self.index(&edge.b)) else {
                continue;
            };
            match bonds.path(a, b, |other| kept[other]) {
                Some(second) => {
                    let protected = |n: &NodeId| {
                        let node = &spec.nodes[self.index(n).unwrap_or(0)];
                        matches!(node.role, Role::MainsEarth | Role::Brain)
                            || node.must_float
                            || spec.must_be_isolated.iter().any(|r| &r.a == n || &r.b == n)
                    };
                    let first = Path {
                        start: edge.a.clone(),
                        steps: vec![Step {
                            via: edge.id.clone(),
                            to: edge.b.clone(),
                        }],
                    };
                    let severity = if second.nodes().chain(first.nodes()).any(protected) {
                        Severity::Stop
                    } else {
                        Severity::Warn
                    };
                    findings.push(Finding::GroundLoop {
                        a: edge.a.clone(),
                        b: edge.b.clone(),
                        first,
                        second,
                        severity,
                    });
                }
                None => kept[e] = true,
            }
        }

        // 3. Barriers rated below what they may see.
        for edge in &spec.edges {
            if let Link::Isolated {
                rating_volts,
                max_difference_volts: Some(max_difference_volts),
            } = edge.link
            {
                if rating_volts < max_difference_volts {
                    findings.push(Finding::UnderratedBarrier {
                        edge: edge.id.clone(),
                        rating_volts,
                        max_difference_volts,
                    });
                }
            }
        }

        // 4. What is earthed.
        for domain in &domains {
            let is_earth = |n: &&usize| spec.nodes[**n].role == Role::MainsEarth;
            if let Some(&earth) = domain.iter().find(is_earth) {
                findings.push(Finding::EarthedDomain {
                    earth: id(earth),
                    members: domain
                        .iter()
                        .filter(|&&n| n != earth)
                        .map(|&n| id(n))
                        .collect(),
                });
            }
        }

        // 5. A module that must float, bonded to the brain.
        for domain in &domains {
            for &module in domain.iter().filter(|&&n| spec.nodes[n].must_float) {
                for &brain in domain
                    .iter()
                    .filter(|&&n| spec.nodes[n].role == Role::Brain)
                {
                    if let Some(path) = bonds.path(module, brain, |_| true) {
                        findings.push(Finding::ModuleNotFloating {
                            module: id(module),
                            brain: id(brain),
                            path,
                        });
                    }
                }
            }
        }

        // 6. A bus with two active masters.
        let mut seen: Vec<&NodeId> = Vec::new();
        for claim in &spec.bus_masters {
            if seen.contains(&&claim.bus) {
                continue;
            }
            seen.push(&claim.bus);
            let masters: Vec<NodeId> = spec
                .bus_masters
                .iter()
                .filter(|c| c.bus == claim.bus && self.is_claim_active(c))
                .map(|c| c.master.clone())
                .collect();
            let mut distinct = masters.clone();
            distinct.sort();
            distinct.dedup();
            if distinct.len() > 1 {
                findings.push(Finding::BusMastership {
                    bus: claim.bus.clone(),
                    masters,
                });
            }
        }

        Report {
            domains: domains
                .into_iter()
                .map(|d| d.into_iter().map(id).collect())
                .collect(),
            findings,
        }
    }
}
