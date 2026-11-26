//! The ratchet: the catalog parses, is well formed, and every claim in it is
//! backed by something in the repository.

use sawbones_redteam::{
    load_all, load_lane, proof_problem, proof_problems, repo_root, schema_problems, summary_table,
    with_summary, Status, LANES,
};

fn lanes() -> Vec<(&'static str, Vec<sawbones_redteam::Attack>)> {
    load_all().unwrap_or_else(|e| panic!("the catalog does not parse: {e}"))
}

#[test]
fn every_catalog_file_parses_and_has_cards() {
    for (stem, _, _) in LANES {
        let attacks = load_lane(stem).unwrap_or_else(|e| panic!("{e}"));
        assert!(!attacks.is_empty(), "{stem} has no cards");
    }
}

#[test]
fn the_schema_holds_and_ids_are_unique() {
    let problems = schema_problems(&lanes());
    assert!(problems.is_empty(), "\n{}", problems.join("\n"));
}

#[test]
fn a_blocked_card_names_a_test_or_job_that_exists() {
    let problems = proof_problems(&lanes());
    assert!(problems.is_empty(), "\n{}", problems.join("\n"));
}

#[test]
fn a_partial_or_gap_card_has_an_issue_and_no_card_is_blocked_without_a_proof() {
    for (_, attacks) in lanes() {
        for a in attacks {
            match a.status {
                Status::Blocked => assert!(!a.test.trim().is_empty(), "{}", a.id),
                Status::Partial | Status::Gap => {
                    assert!(a.issue.is_some_and(|n| n > 0), "{} has no issue", a.id)
                }
            }
        }
    }
}

#[test]
fn the_proof_checker_refuses_names_that_do_not_exist() {
    // Controls: a checker that accepts everything would make the ratchet empty.
    assert!(proof_problem("sawbones-busguard::a_write_with_any_byte_changed_is_blocked").is_none());
    assert!(proof_problem("ci:zizmor").is_none());
    assert!(proof_problem("sawbones-busguard::no_such_test_anywhere").is_some());
    assert!(proof_problem("sawbones-nope::a_write_with_any_byte_changed_is_blocked").is_some());
    assert!(proof_problem("ci:no-such-job").is_some());
    assert!(proof_problem("ci:ci/no_such_script.sh").is_some());
    assert!(proof_problem("a test name with no crate").is_some());
    // A helper that is not a test does not count.
    assert!(proof_problem("sawbones-busguard::with_crc").is_some());
}

#[test]
fn the_readme_summary_matches_the_catalog() {
    let path = repo_root().join("redteam/README.md");
    let readme = std::fs::read_to_string(&path).expect("redteam/README.md");
    let want = with_summary(&readme, &summary_table(&lanes()))
        .expect("README.md is missing the summary marks");
    if std::env::var_os("REDTEAM_BLESS").is_some() {
        std::fs::write(&path, &want).expect("write README.md");
        return;
    }
    assert_eq!(
        readme, want,
        "the summary in redteam/README.md is stale: run `REDTEAM_BLESS=1 cargo test -p sawbones-redteam`"
    );
}

/// Needs `GITHUB_TOKEN` and network, so it is ignored by default. Run with
/// `cargo test -p sawbones-redteam -- --ignored`.
#[test]
#[ignore = "needs GITHUB_TOKEN and network"]
fn every_tracking_issue_is_open() {
    let Some(token) = std::env::var_os("GITHUB_TOKEN") else {
        eprintln!("GITHUB_TOKEN is not set: nothing checked");
        return;
    };
    let token = token.to_string_lossy().into_owned();
    let mut issues: Vec<u64> = lanes()
        .iter()
        .flat_map(|(_, attacks)| attacks.iter().filter_map(|a| a.issue))
        .collect();
    issues.sort_unstable();
    issues.dedup();
    let mut closed = Vec::new();
    for n in issues {
        let out = std::process::Command::new("curl")
            .args(["-fsS", "-H", "Accept: application/vnd.github+json", "-H"])
            .arg(format!("Authorization: Bearer {token}"))
            .arg(format!(
                "https://api.github.com/repos/rafe-walker/sawbones-labs/issues/{n}"
            ))
            .output()
            .expect("curl");
        assert!(out.status.success(), "could not read issue #{n}");
        let v: serde_json::Value = serde_json::from_slice(&out.stdout).expect("json");
        if v["state"] != "open" {
            closed.push(n);
        }
    }
    assert!(closed.is_empty(), "tracking items are not open: {closed:?}");
}
