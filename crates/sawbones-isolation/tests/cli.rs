//! The `isolation-check` binary: what it prints and how it exits.

use std::path::Path;
use std::process::{Command, Output};

use sawbones_isolation::{Edge, IsolationMap};

fn run(map: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_isolation-check"))
        .arg(map)
        .output()
        .expect("the binary runs")
}

#[test]
fn exit_code_follows_the_violations() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/bench-core.json");
    let clean = run(&fixture);
    assert_eq!(clean.status.code(), Some(0));
    let out = String::from_utf8(clean.stdout).unwrap();
    assert!(out.contains("domain 1: mains-earth\n"), "{out}");
    assert!(
        out.contains("note: nothing is bonded to mains-earth\n"),
        "{out}"
    );

    let text = std::fs::read_to_string(&fixture).unwrap();
    let plan = IsolationMap::from_json(&text)
        .and_then(|m| m.with_edge(Edge::bonded("clip", "logic-analyzer", "module-1")))
        .unwrap();
    let bad = Path::new(env!("CARGO_TARGET_TMPDIR")).join("bench-core-with-clip.json");
    std::fs::write(&bad, serde_json::to_string(&plan).unwrap()).unwrap();
    let refused = run(&bad);
    assert_eq!(refused.status.code(), Some(1));
    let out = String::from_utf8(refused.stdout).unwrap();
    assert!(
        out.contains("STOP: module-1 and pi must be isolated but are bonded: module-1 -[clip]-"),
        "{out}"
    );

    // A warning alone does not fail the checker.
    let warned = IsolationMap::from_json(&text)
        .and_then(|m| m.with_edge(Edge::bonded("debug-ground", "debug-probe", "trust-root")))
        .unwrap();
    std::fs::write(&bad, serde_json::to_string(&warned).unwrap()).unwrap();
    let ok = run(&bad);
    assert_eq!(ok.status.code(), Some(0));
    let out = String::from_utf8(ok.stdout).unwrap();
    assert!(
        out.contains("WARN: ground loop between debug-probe"),
        "{out}"
    );

    let missing = run(Path::new("no-such-map.json"));
    assert_eq!(missing.status.code(), Some(2));
    std::fs::write(&bad, "{}").unwrap();
    assert_eq!(run(&bad).status.code(), Some(2));
}
