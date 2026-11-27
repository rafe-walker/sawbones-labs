//! The plan builder (#111): the map comes from the bench config and the
//! modules' declarations, a declaration is a claim, and what is unknown
//! refuses.

use std::collections::BTreeMap;

use sawbones_isolation::{
    build_plan, BenchConfig, Connection, Edge, Finding, GroundDomain, MacPower, ModuleDeclaration,
    Node, PlanProblem, Role, TerminalSpec,
};

const SCOPE: u32 = 1;
const METER: u32 = 2;

fn config(mac: MacPower) -> BenchConfig {
    BenchConfig::new(mac)
        .node(Node::new("pi", Role::Brain))
        .node(Node::new("hub", Role::Other))
        .node(Node::new("floating-bus", Role::Other))
        .node(Node::new("truck-chassis", Role::Other))
        .edge(Edge::bonded("pi-usb", "pi", "hub"))
        .edge(Edge::isolated("scope-isolator", "hub", "scope", 1000))
        .edge(Edge::isolated("meter-isolator", "hub", "meter", 1000))
        .edge(Edge::bonded(
            "bus-tie",
            "floating-bus",
            GroundDomain::Pack.node(),
        ))
        .edge(Edge::bonded(
            "chassis-tie",
            "truck-chassis",
            GroundDomain::VehicleChassis.node(),
        ))
        .module(
            SCOPE,
            "scope",
            vec![TerminalSpec::ground("clip", GroundDomain::MainsEarth)],
        )
        .module(METER, "meter", vec![TerminalSpec::isolated("probe", 1000)])
}

fn declarations() -> BTreeMap<u32, ModuleDeclaration> {
    BTreeMap::from([
        (
            SCOPE,
            ModuleDeclaration::new(vec![TerminalSpec::ground("clip", GroundDomain::MainsEarth)]),
        ),
        (
            METER,
            ModuleDeclaration::new(vec![TerminalSpec::isolated("probe", 1000)]),
        ),
    ])
}

fn stops(
    config: &BenchConfig,
    decls: &BTreeMap<u32, ModuleDeclaration>,
    plan: &[Connection],
) -> Vec<Finding> {
    let map = build_plan(config, decls, plan).unwrap();
    map.check().violations().cloned().collect()
}

fn problems(
    config: &BenchConfig,
    decls: &BTreeMap<u32, ModuleDeclaration>,
    plan: &[Connection],
) -> Vec<PlanProblem> {
    build_plan(config, decls, plan).unwrap_err().problems
}

#[test]
fn the_bench_with_no_connections_is_clean_either_way_the_mac_is_powered() {
    for mac in [MacPower::OnBattery, MacPower::OnCharger] {
        assert!(stops(&config(mac), &declarations(), &[]).is_empty());
    }
}

#[test]
fn a_grounded_scope_clip_on_a_floating_bus_is_refused() {
    let plan = [Connection::new("scope/clip", "floating-bus")];
    let found = stops(&config(MacPower::OnBattery), &declarations(), &plan);
    assert!(
        matches!(&found[..], [Finding::IsolationBroken { path, .. }]
            if path.edges().any(|e| e.as_str() == "plan-1")),
        "{found:?}"
    );
}

#[test]
fn a_truck_chassis_tied_to_the_macs_charger_through_usb_is_refused() {
    let plan = [Connection::new("truck-chassis", "mac")];
    let on_charger = stops(&config(MacPower::OnCharger), &declarations(), &plan);
    assert!(
        matches!(&on_charger[..], [Finding::IsolationBroken { .. }]),
        "{on_charger:?}"
    );
    // The Mac floats on its battery: the same cable is harmless.
    assert!(stops(&config(MacPower::OnBattery), &declarations(), &plan).is_empty());
}

#[test]
fn a_module_that_says_isolated_is_not_believed_over_the_config() {
    let mut decls = declarations();
    decls.insert(
        SCOPE,
        ModuleDeclaration::new(vec![TerminalSpec::isolated("clip", 5000)]),
    );
    let plan = [Connection::new("scope/clip", "floating-bus")];
    let found = stops(&config(MacPower::OnBattery), &decls, &plan);
    assert!(matches!(&found[..], [Finding::IsolationBroken { .. }]));
}

#[test]
fn a_module_that_admits_a_domain_is_not_talked_out_of_it_by_the_config() {
    // The config says the meter's probe is isolated; the meter says it is
    // on mains earth. Earth wins: clipping the probe to the bus is refused.
    let mut decls = declarations();
    decls.insert(
        METER,
        ModuleDeclaration::new(vec![TerminalSpec::ground(
            "probe",
            GroundDomain::MainsEarth,
        )]),
    );
    let plan = [Connection::new("meter/probe", "floating-bus")];
    let found = stops(&config(MacPower::OnBattery), &decls, &plan);
    assert!(matches!(&found[..], [Finding::IsolationBroken { .. }]));
}

#[test]
fn two_isolation_claims_take_the_lower_rating() {
    let cfg = config(MacPower::OnBattery);
    let mut low = declarations();
    low.insert(
        METER,
        ModuleDeclaration::new(vec![TerminalSpec::isolated("probe", 300)]),
    );
    let map = build_plan(&cfg, &low, &[]).unwrap();
    let barrier = map
        .spec()
        .edges
        .iter()
        .find(|e| e.id.as_str() == "meter/probe-barrier")
        .unwrap();
    assert_eq!(
        barrier.link,
        sawbones_isolation::Link::Isolated {
            rating_volts: 300,
            max_difference_volts: None
        }
    );
    // And the other way round: the config's lower figure wins.
    let cfg = BenchConfig {
        modules: vec![],
        ..config(MacPower::OnBattery)
    }
    .module(SCOPE, "scope", vec![])
    .module(METER, "meter", vec![TerminalSpec::isolated("probe", 200)]);
    let map = build_plan(&cfg, &declarations_without_scope_terminals(), &[]).unwrap();
    assert!(map
        .spec()
        .edges
        .iter()
        .any(|e| e.id.as_str() == "meter/probe-barrier"
            && matches!(
                e.link,
                sawbones_isolation::Link::Isolated {
                    rating_volts: 200,
                    ..
                }
            )));
}

fn declarations_without_scope_terminals() -> BTreeMap<u32, ModuleDeclaration> {
    let mut d = declarations();
    d.insert(SCOPE, ModuleDeclaration::default());
    d
}

#[test]
fn a_missing_declaration_is_unknown_and_refused() {
    let mut decls = declarations();
    decls.remove(&SCOPE);
    assert_eq!(
        problems(&config(MacPower::OnBattery), &decls, &[]),
        vec![PlanProblem::NoDeclaration {
            module: "scope".into()
        }]
    );
    assert_eq!(
        problems(&config(MacPower::OnBattery), &BTreeMap::new(), &[]).len(),
        2
    );
}

#[test]
fn a_terminal_declared_without_the_config_or_the_config_without_a_declaration_is_refused() {
    let mut decls = declarations();
    decls.insert(
        METER,
        ModuleDeclaration::new(vec![
            TerminalSpec::isolated("probe", 1000),
            TerminalSpec::isolated("aux", 1000),
        ]),
    );
    assert_eq!(
        problems(&config(MacPower::OnBattery), &decls, &[]),
        vec![PlanProblem::UnconfirmedTerminal {
            module: "meter".into(),
            terminal: "aux".into()
        }]
    );
    decls.insert(METER, ModuleDeclaration::default());
    assert_eq!(
        problems(&config(MacPower::OnBattery), &decls, &[]),
        vec![PlanProblem::UndeclaredTerminal {
            module: "meter".into(),
            terminal: "probe".into()
        }]
    );
}

#[test]
fn two_different_domains_for_one_terminal_conflict() {
    let mut decls = declarations();
    decls.insert(
        SCOPE,
        ModuleDeclaration::new(vec![TerminalSpec::ground("clip", GroundDomain::Pack)]),
    );
    assert_eq!(
        problems(&config(MacPower::OnBattery), &decls, &[]),
        vec![PlanProblem::ConflictingDomains {
            module: "scope".into(),
            terminal: "clip".into(),
            config: GroundDomain::MainsEarth,
            claimed: GroundDomain::Pack
        }]
    );
}

#[test]
fn a_duplicate_terminal_is_refused() {
    let mut decls = declarations();
    decls.insert(
        METER,
        ModuleDeclaration::new(vec![
            TerminalSpec::isolated("probe", 1000),
            TerminalSpec::ground("probe", GroundDomain::Pack),
        ]),
    );
    assert!(matches!(
        &problems(&config(MacPower::OnBattery), &decls, &[])[..],
        [PlanProblem::DuplicateTerminal { .. }]
    ));
}

#[test]
fn a_connection_can_only_name_what_is_on_the_bench_and_not_itself() {
    let plan = [Connection::new("scope/clip", "nowhere")];
    assert_eq!(
        problems(&config(MacPower::OnBattery), &declarations(), &plan),
        vec![PlanProblem::UnknownEndpoint {
            endpoint: "nowhere".into()
        }]
    );
    let plan = [Connection::new("mac", "mac")];
    assert!(matches!(
        &problems(&config(MacPower::OnBattery), &declarations(), &plan)[..],
        [PlanProblem::Invalid { .. }]
    ));
}

#[test]
fn the_config_cannot_set_the_macs_switch_or_leave_out_the_brain() {
    let cfg = config(MacPower::OnBattery).close("mac-on-charger");
    assert!(matches!(
        &problems(&cfg, &declarations(), &[])[..],
        [PlanProblem::Invalid { .. }]
    ));
    let mut cfg = config(MacPower::OnBattery);
    cfg.nodes.retain(|n| n.role != Role::Brain);
    cfg.edges.retain(|e| e.id.as_str() != "pi-usb");
    assert!(problems(&cfg, &declarations(), &[])
        .iter()
        .any(|p| matches!(p, PlanProblem::Invalid { .. })));
}

#[test]
fn a_config_node_cannot_shadow_a_builder_node() {
    let cfg = config(MacPower::OnBattery).node(Node::new("mains-earth", Role::Other));
    assert!(matches!(
        &problems(&cfg, &declarations(), &[])[..],
        [PlanProblem::Invalid { .. }]
    ));
}

#[test]
fn a_config_reads_from_json_and_builds_the_same_map() {
    let json = r#"{
      "schema": "sawbones.bench-config/0-provisional",
      "mac": "on_charger",
      "nodes": [ { "id": "pi", "role": "brain" }, { "id": "truck-chassis" } ],
      "edges": [
        { "id": "chassis-tie", "a": "truck-chassis", "b": "domain:vehicle-chassis",
          "link": { "kind": "bonded" } }
      ],
      "modules": [
        { "id": 2, "name": "meter",
          "terminals": [ { "name": "probe", "claim": { "kind": "isolated", "rating_volts": 1000 } } ] }
      ]
    }"#;
    let cfg = BenchConfig::from_json(json).unwrap();
    assert!(cfg.modules[0].must_float, "must_float defaults to true");
    let decls = BTreeMap::from([(
        2,
        ModuleDeclaration::new(vec![TerminalSpec::isolated("probe", 1000)]),
    )]);
    let found = stops(&cfg, &decls, &[Connection::new("truck-chassis", "mac")]);
    assert!(matches!(&found[..], [Finding::IsolationBroken { .. }]));
    assert!(BenchConfig::from_json(r#"{"schema":"x","mac":"on_charger","bogus":1}"#).is_err());
}

#[test]
fn a_wrong_schema_label_is_refused() {
    let mut cfg = config(MacPower::OnBattery);
    cfg.schema = "other".into();
    assert!(matches!(
        &problems(&cfg, &declarations(), &[])[..],
        [PlanProblem::Invalid { .. }]
    ));
}
