//! Each check on a map small enough to read at a glance.

use sawbones_isolation::{Edge, Finding, IsolationMap, MapError, MapSpec, Node, Role};

/// Earth, a brain with a hub, and one floating meter behind an isolator.
fn small() -> MapSpec {
    MapSpec::new()
        .node(Node::new("earth", Role::MainsEarth))
        .node(Node::new("pi", Role::Brain))
        .node(Node::new("hub", Role::Other))
        .node(Node::new("meter", Role::MeasuringModule).must_float())
        .edge(Edge::isolated("supply", "earth", "pi", 3000))
        .edge(Edge::bonded("pi-usb", "pi", "hub"))
        .edge(Edge::isolated("usb-isolator", "hub", "meter", 1000))
        .must_be_isolated("meter", "pi")
}

fn names(ids: &[sawbones_isolation::NodeId]) -> Vec<&str> {
    ids.iter().map(|n| n.as_str()).collect()
}

#[test]
fn domains_are_the_bonded_components() {
    let map = small().build().unwrap();
    let domains = map.domains();
    let domains: Vec<Vec<&str>> = domains.iter().map(|d| names(d)).collect();
    assert_eq!(domains, [vec!["earth"], vec!["pi", "hub"], vec!["meter"]]);
    assert!(map.bonded_path("meter", "pi").unwrap().is_none());
    assert!(map.check().is_clean());
}

#[test]
fn a_bridged_barrier_is_reported_with_the_shortest_path() {
    let map = small()
        .node(Node::new("probe", Role::Other))
        .edge(Edge::bonded("probe-usb", "hub", "probe"))
        .edge(Edge::bonded("probe-clip", "probe", "meter"))
        .build()
        .unwrap();
    let report = map.check();
    let broken = report.findings.iter().find_map(|f| match f {
        Finding::IsolationBroken { a, b, path } => Some((a.as_str(), b.as_str(), path)),
        _ => None,
    });
    let (a, b, path) = broken.expect("the rule is broken");
    assert_eq!((a, b), ("meter", "pi"));
    assert_eq!(
        path.to_string(),
        "meter -[probe-clip]- probe -[probe-usb]- hub -[pi-usb]- pi"
    );
    let floating = report.findings.iter().any(|f| {
        matches!(f, Finding::ModuleNotFloating { module, brain, .. }
            if module.as_str() == "meter" && brain.as_str() == "pi")
    });
    assert!(floating);
    assert!(!report.is_clean());
}

#[test]
fn a_second_bonded_path_is_a_ground_loop_with_both_paths() {
    let map = small()
        .node(Node::new("probe", Role::Other))
        .node(Node::new("board", Role::Other))
        .edge(Edge::bonded("probe-usb", "hub", "probe"))
        .edge(Edge::bonded("board-usb", "hub", "board"))
        .build()
        .unwrap();
    assert!(map.check().is_clean());

    let looped = map
        .with_edge(Edge::bonded("probe-clip", "probe", "board"))
        .unwrap();
    let report = looped.check();
    let loops: Vec<_> = report
        .findings
        .iter()
        .filter_map(|f| match f {
            Finding::GroundLoop { first, second, .. } => {
                Some((first.to_string(), second.to_string()))
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        loops,
        [(
            "probe -[probe-clip]- board".to_owned(),
            "probe -[probe-usb]- hub -[board-usb]- board".to_owned()
        )]
    );
    assert!(!report.is_clean());

    // Two cables between the same pair are a loop too.
    let doubled = map
        .with_edge(Edge::bonded("pi-usb-2", "pi", "hub"))
        .unwrap();
    assert_eq!(doubled.check().violations().count(), 1);
}

#[test]
fn a_barrier_rated_below_its_declared_difference_is_reported() {
    let enough = small()
        .node(Node::new("pack", Role::Other))
        .edge(Edge::isolated("pod", "hub", "pack", 1000).max_difference(1000))
        .build()
        .unwrap();
    assert!(enough.check().is_clean());

    let short = small()
        .node(Node::new("pack", Role::Other))
        .edge(Edge::isolated("pod", "hub", "pack", 50).max_difference(60))
        .build()
        .unwrap();
    let report = short.check();
    let found: Vec<_> = report.violations().collect();
    assert!(matches!(
        found[..],
        [Finding::UnderratedBarrier { edge, rating_volts: 50, max_difference_volts: 60 }]
            if edge.as_str() == "pod"
    ));
    assert_eq!(
        found[0].to_string(),
        "barrier pod is rated 50 V but may see 60 V"
    );
}

#[test]
fn the_earthed_domain_is_listed_and_is_not_a_violation() {
    let floating = small().build().unwrap().check();
    assert!(floating.findings.iter().any(|f| matches!(
        f,
        Finding::EarthedDomain { members, .. } if members.is_empty()
    )));

    let map = small()
        .edge(Edge::bonded("earth-strap", "earth", "hub"))
        .build()
        .unwrap();
    let report = map.check();
    let earthed = report.findings.iter().find_map(|f| match f {
        Finding::EarthedDomain { earth, members } => Some((earth.as_str(), names(members))),
        _ => None,
    });
    assert_eq!(earthed, Some(("earth", vec!["pi", "hub"])));
    assert!(report.is_clean());
}

#[test]
fn a_switched_edge_exists_only_while_its_switch_is_closed() {
    let open = small()
        .edge(Edge::bonded("lead", "hub", "meter").switched_by("lead-fitted"))
        .build()
        .unwrap();
    assert!(open.check().is_clean());
    let closed = open.with_switch("lead-fitted", true).unwrap();
    assert!(!closed.check().is_clean());
    assert!(closed
        .with_switch("lead-fitted", false)
        .unwrap()
        .check()
        .is_clean());
}

#[test]
fn bad_descriptions_are_refused() {
    let unknown = small().edge(Edge::bonded("x", "hub", "nowhere")).build();
    assert!(matches!(unknown, Err(MapError::UnknownNode(n)) if n.as_str() == "nowhere"));
    let twice = small().node(Node::new("pi", Role::Other)).build();
    assert!(matches!(twice, Err(MapError::DuplicateNode(_))));
    let same_edge = small().edge(Edge::bonded("pi-usb", "pi", "meter")).build();
    assert!(matches!(same_edge, Err(MapError::DuplicateEdge(_))));
    let to_self = small().edge(Edge::bonded("x", "hub", "hub")).build();
    assert!(matches!(to_self, Err(MapError::SelfEdge(_))));
    let stray = small().close("not-a-switch").build();
    assert!(matches!(stray, Err(MapError::UnknownSwitch(_))));
    let not_module = small()
        .node(Node::new("x", Role::Other).must_float())
        .build();
    assert!(matches!(not_module, Err(MapError::MustFloatNotModule(_))));
    let rule = small().must_be_isolated("pi", "pi").build();
    assert!(matches!(rule, Err(MapError::RuleSameNode(_))));

    let map = small().build().unwrap();
    assert!(matches!(
        map.bonded_path("meter", "pie"),
        Err(MapError::UnknownNode(_))
    ));
}

#[test]
fn json_round_trips_and_rejects_unknown_fields_and_schemas() {
    let map = small().build().unwrap();
    let text = serde_json::to_string(&map).unwrap();
    assert_eq!(IsolationMap::from_json(&text).unwrap(), map);

    let typo = text.replacen("\"must_float\"", "\"must_flaot\"", 1);
    assert!(matches!(
        IsolationMap::from_json(&typo),
        Err(MapError::Json(_))
    ));
    let other = text.replacen("isolation-map/0", "isolation-map/9", 1);
    assert!(matches!(
        IsolationMap::from_json(&other),
        Err(MapError::Schema { .. })
    ));
}
