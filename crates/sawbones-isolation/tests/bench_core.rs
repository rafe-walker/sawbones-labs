//! The Bench Core as designed, and the connection plans that must be refused.

use sawbones_isolation::{Edge, Finding, IsolationMap, Report};

const BENCH_CORE: &str = include_str!("../fixtures/bench-core.json");

fn bench_core() -> IsolationMap {
    IsolationMap::from_json(BENCH_CORE).expect("the fixture is a valid map")
}

/// The names of everything bonded to mains earth.
fn earthed(report: &Report) -> Vec<&str> {
    let mut found = report.findings.iter().filter_map(|f| match f {
        Finding::EarthedDomain { members, .. } => Some(members),
        _ => None,
    });
    let members = found.next().expect("mains earth is on the map");
    assert!(found.next().is_none(), "one mains earth, one listing");
    members.iter().map(|n| n.as_str()).collect()
}

#[test]
fn as_designed_the_modules_are_isolated_from_the_brain() {
    let map = bench_core();
    for module in ["module-1", "module-2"] {
        assert!(map.bonded_path(module, "pi").unwrap().is_none());
        assert!(map.bonded_path(module, "mains-earth").unwrap().is_none());
    }
    assert!(map.bonded_path("module-1", "module-2").unwrap().is_none());
    // Each module shares a domain with the thing it is wired to, and with
    // nothing else.
    let domains = map.domains();
    let domain_of = |name: &str| {
        let domain = domains
            .iter()
            .find(|d| d.iter().any(|n| n.as_str() == name));
        let domain = domain.expect("every node is in a domain");
        domain.iter().map(|n| n.as_str()).collect::<Vec<_>>()
    };
    assert_eq!(domain_of("module-1"), ["module-1", "dut"]);
    assert_eq!(domain_of("module-2"), ["module-2", "inverter-comms"]);
    assert_eq!(
        domain_of("pi"),
        [
            "pi",
            "hub",
            "hub-12v-adapter",
            "trust-root",
            "rail-12v-adapter",
            "debug-probe",
            "logic-analyzer",
            "serial-adapter"
        ]
    );

    let report = map.check();
    assert!(report.is_clean(), "{:?}", report.findings);
    // Laptop on battery: nothing on the bench is earthed.
    assert!(earthed(&report).is_empty());
}

#[test]
fn the_logic_analyzer_clip_on_module_1_breaks_its_isolation() {
    let plan = bench_core()
        .with_edge(Edge::bonded(
            "logic-analyzer-clip",
            "logic-analyzer",
            "module-1",
        ))
        .unwrap();
    let report = plan.check();
    assert!(!report.is_clean());

    let wanted = "module-1 -[logic-analyzer-clip]- logic-analyzer \
                  -[hub-usb-logic-analyzer]- hub -[pi-usb-hub]- pi";
    let broken = report.findings.iter().any(|f| {
        matches!(f, Finding::IsolationBroken { a, b, path }
            if a.as_str() == "module-1" && b.as_str() == "pi" && path.to_string() == wanted)
    });
    assert!(broken, "{:?}", report.findings);
    let not_floating = report.findings.iter().any(|f| {
        matches!(f, Finding::ModuleNotFloating { module, brain, path }
            if module.as_str() == "module-1" && brain.as_str() == "pi"
                && path.to_string() == wanted)
    });
    assert!(not_floating, "{:?}", report.findings);

    // Module 2 is untouched, so every violation names module 1.
    assert!(plan.bonded_path("module-2", "pi").unwrap().is_none());
    assert_eq!(report.violations().count(), 2);
    // The device under test now shares the brain's ground.
    assert!(plan.bonded_path("dut", "pi").unwrap().is_some());
}

#[test]
fn a_laptop_on_its_charger_over_usb_earths_the_brain() {
    let map = bench_core();

    // On its charger but reaching the bench over Ethernet: only the laptop
    // is earthed.
    let charging = map.with_switch("laptop-charger", true).unwrap();
    assert_eq!(earthed(&charging.check()), ["laptop"]);

    // Add a USB cable and the brain's whole domain is on mains earth.
    let tethered = charging.with_switch("laptop-usb", true).unwrap();
    let report = tethered.check();
    let on_earth = earthed(&report);
    for name in ["pi", "hub", "trust-root", "logic-analyzer", "laptop"] {
        assert!(on_earth.contains(&name), "{name} should be earthed");
    }
    let path = tethered.bonded_path("pi", "mains-earth").unwrap();
    assert_eq!(
        path.expect("the brain is earthed").to_string(),
        "pi -[laptop-usb]- laptop -[laptop-charger]- mains-earth"
    );
    // The isolators still hold, so this is reported but refuses nothing.
    assert!(!on_earth.contains(&"module-1") && !on_earth.contains(&"dut"));
    assert!(report.is_clean(), "{:?}", report.findings);

    // On battery over the same cable, nothing is earthed.
    let on_battery = tethered.with_switch("laptop-charger", false).unwrap();
    assert!(earthed(&on_battery.check()).is_empty());
}

#[test]
fn a_clip_plus_a_charging_laptop_earths_the_device_under_test() {
    let plan = bench_core()
        .with_switch("laptop-charger", true)
        .and_then(|m| m.with_switch("laptop-usb", true))
        .and_then(|m| m.with_edge(Edge::bonded("clip", "logic-analyzer", "module-1")))
        .unwrap();
    let report = plan.check();
    assert!(!report.is_clean());
    assert!(earthed(&report).contains(&"dut"));
}

#[test]
fn both_serial_taps_on_the_inverter_stay_isolated_and_make_no_loop() {
    let both = bench_core()
        .with_switch("serial-adapter-connected", true)
        .unwrap();
    let report = both.check();
    assert!(report.is_clean(), "{:?}", report.findings);
    assert!(both.bonded_path("inverter-comms", "pi").unwrap().is_none());
}

#[test]
fn the_debug_probe_ground_wire_to_the_trust_root_is_a_ground_loop() {
    let plan = bench_core()
        .with_edge(Edge::bonded("debug-ground", "debug-probe", "trust-root"))
        .unwrap();
    let loops: Vec<_> = plan
        .check()
        .findings
        .into_iter()
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
            "debug-probe -[debug-ground]- trust-root".to_owned(),
            "debug-probe -[hub-usb-debug-probe]- hub -[hub-usb-trust-root]- trust-root".to_owned()
        )]
    );
}

#[test]
fn the_fixture_declares_no_voltage_differences_yet() {
    // Nothing on the Bench Core has a declared maximum difference across a
    // barrier, so the rating check has nothing to compare. This test fails
    // when that changes, as a reminder to assert on the result.
    assert!(!BENCH_CORE.contains("max_difference_volts"));
}
