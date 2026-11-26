//! Each check on a map small enough to read at a glance.

use sawbones_isolation::{Edge, Finding, IsolationMap, MapError, MapSpec, Node, Role, Severity};

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
    // A loop inside the hub's domain touches nothing the map protects, so it
    // is a warning, not a stop.
    assert!(report.is_clean());
    assert_eq!(report.warnings().count(), 1);
    assert_eq!(report.findings[0].severity(), Severity::Warn);

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

#[test]
fn a_loop_through_something_protected_is_a_stop() {
    // Two cables between the brain and the hub: the loop touches the brain.
    let doubled = small()
        .edge(Edge::bonded("pi-usb-2", "pi", "hub"))
        .build()
        .unwrap();
    let report = doubled.check();
    let looped = report
        .findings
        .iter()
        .find(|f| matches!(f, Finding::GroundLoop { .. }));
    assert_eq!(looped.unwrap().severity(), Severity::Stop);
    assert!(!report.is_clean());

    // A loop that touches mains earth is a stop too.
    let earthed = MapSpec::new()
        .node(Node::new("earth", Role::MainsEarth))
        .node(Node::new("a", Role::Other))
        .edge(Edge::bonded("one", "earth", "a"))
        .edge(Edge::bonded("two", "earth", "a"))
        .build()
        .unwrap();
    assert!(!earthed.check().is_clean());
}

#[test]
fn severities_are_stop_warn_and_info() {
    let report = small().build().unwrap().check();
    assert!(report
        .findings
        .iter()
        .all(|f| f.severity() == Severity::Info));
    assert!(Severity::Info < Severity::Warn && Severity::Warn < Severity::Stop);

    let broken = small()
        .edge(Edge::bonded("clip", "hub", "meter"))
        .build()
        .unwrap()
        .check();
    assert!(broken.violations().count() >= 2);
    assert!(broken.violations().all(|f| f.severity() == Severity::Stop));

    let underrated = small()
        .edge(Edge::isolated("barrier", "earth", "meter", 100).max_difference(400))
        .build()
        .unwrap()
        .check();
    let found = underrated
        .findings
        .iter()
        .find(|f| matches!(f, Finding::UnderratedBarrier { .. }));
    assert_eq!(found.unwrap().severity(), Severity::Stop);
}

fn buses() -> MapSpec {
    MapSpec::new()
        .node(Node::new("pi", Role::Brain))
        .node(Node::new("mcu", Role::Other))
        .node(Node::new("laptop-probe", Role::Other))
        .node(Node::new("i2c", Role::Bus))
        .bus_master("i2c", "pi")
}

#[test]
fn one_master_on_a_bus_is_fine_and_two_are_a_stop() {
    let one = buses().build().unwrap().check();
    assert!(one.is_clean() && one.findings.is_empty());

    let two = buses().bus_master("i2c", "mcu").build().unwrap().check();
    assert!(!two.is_clean());
    match &two.findings[..] {
        [f @ Finding::BusMastership { bus, masters }] => {
            assert_eq!(bus.as_str(), "i2c");
            assert_eq!(names(masters), ["pi", "mcu"]);
            assert_eq!(f.severity(), Severity::Stop);
            assert_eq!(f.to_string(), "bus i2c has 2 masters: pi mcu");
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_switched_master_counts_only_while_its_switch_is_closed() {
    let map = buses()
        .bus_master_switched("i2c", "laptop-probe", "probe-on-bus")
        .build()
        .unwrap();
    assert!(map.check().is_clean());
    let probing = map.with_switch("probe-on-bus", true).unwrap();
    assert!(!probing.check().is_clean());
}

#[test]
fn bus_declarations_are_validated() {
    let not_a_bus = buses().bus_master("mcu", "pi").build();
    assert!(matches!(not_a_bus, Err(MapError::NotABus(_))));
    let unknown = buses().bus_master("i2c", "ghost").build();
    assert!(matches!(unknown, Err(MapError::UnknownNode(_))));
    let twice = buses().bus_master("i2c", "pi").build();
    assert!(matches!(twice, Err(MapError::DuplicateMaster { .. })));
    // The same master on the same bus, once always and once switched, is
    // not one master counted twice.
    let both = buses()
        .bus_master_switched("i2c", "pi", "s")
        .close("s")
        .build()
        .unwrap();
    assert!(both.check().is_clean());
    // A switch named only by a master claim is a known switch.
    assert!(buses().close("nowhere").build().is_err());
    // Round trip through JSON.
    let json = serde_json::to_string(&both).unwrap();
    assert_eq!(IsolationMap::from_json(&json).unwrap(), both);
}
