//! Reads a JSON isolation map and prints its ground domains and findings.
//!
//! Exit code 0 when no finding is a violation, 1 when one is, 2 when the map
//! could not be read.

use std::process::ExitCode;

use sawbones_isolation::IsolationMap;

fn main() -> ExitCode {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: isolation-check <map.json>");
        return ExitCode::from(2);
    };
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) => {
            eprintln!("{path}: {e}");
            return ExitCode::from(2);
        }
    };
    let map = match IsolationMap::from_json(&text) {
        Ok(map) => map,
        Err(e) => {
            eprintln!("{path}: {e}");
            return ExitCode::from(2);
        }
    };
    let report = map.check();
    for (n, domain) in report.domains.iter().enumerate() {
        let names: Vec<&str> = domain.iter().map(|id| id.as_str()).collect();
        println!("domain {}: {}", n + 1, names.join(" "));
    }
    for finding in &report.findings {
        let kind = if finding.is_violation() {
            "VIOLATION"
        } else {
            "note"
        };
        println!("{kind}: {finding}");
    }
    if report.is_clean() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}
