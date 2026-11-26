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
}

impl Finding {
    /// Whether this finding should stop a connection plan. Only
    /// [`Finding::EarthedDomain`] is not a violation.
    pub fn is_violation(&self) -> bool {
        !matches!(self, Self::EarthedDomain { .. })
    }
}

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IsolationBroken { a, b, path } => {
                write!(f, "{a} and {b} must be isolated but are bonded: {path}")
            }
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
    /// The findings that should stop a connection plan.
    pub fn violations(&self) -> impl Iterator<Item = &Finding> {
        self.findings.iter().filter(|f| f.is_violation())
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

        Report {
            domains: domains
                .into_iter()
                .map(|d| d.into_iter().map(id).collect())
                .collect(),
            findings,
        }
    }
}
