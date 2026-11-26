//! # sawbones-redteam: test-only
//!
//! The red-team attack catalog and its ratchet. The catalog lives in
//! `redteam/catalog/*.toml`, one file per lane, each an array of `[[attack]]`
//! tables. This crate loads those files and checks them; its `tests/` run the
//! deterministic attack tests against the real Sawbones crates. Nothing here
//! ships: the crate is `publish = false` and holds no logic the bench uses.
//!
//! The ratchet has three rules, all checked on every `cargo test`:
//!
//! 1. An attack is `blocked` only if its `test` names a Rust test or a CI job
//!    that exists in this repository.
//! 2. An attack that is `partial` or `gap` carries the number of the item that
//!    tracks the fix.
//! 3. The summary table in `redteam/README.md` matches the catalog.
//!
//! See `redteam/README.md` for the card format and how to add a card.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

/// The lanes: the catalog file name (without `.toml`), the id prefix its
/// cards use, and a short title.
pub const LANES: [(&str, &str, &str); 4] = [
    ("agent-injection", "inj", "Agent injection"),
    ("bus", "bus", "Bus"),
    ("supply-chain", "sup", "Supply chain"),
    ("interlock-hardware", "hw", "Interlock hardware"),
];

/// How bad it is if the attack works.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Harm to people, equipment or the record, or loss of a safeguard.
    High,
    /// A safeguard is weakened, or the harm needs another failure first.
    Medium,
    /// Hardening: little harm by itself.
    Low,
}

/// Where the defence stands today.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    /// A real test or CI job on main proves it.
    Blocked,
    /// Part of it is proven; the rest is tracked by `issue`.
    Partial,
    /// Not blocked; the fix is tracked by `issue`.
    Gap,
}

impl Status {
    /// All statuses, in the order the summary table lists them.
    pub const ALL: [Status; 3] = [Status::Blocked, Status::Partial, Status::Gap];

    /// The word used in the catalog and the summary.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Status::Blocked => "blocked",
            Status::Partial => "partial",
            Status::Gap => "gap",
        }
    }
}

/// One attack card.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Attack {
    /// Stable id: lane prefix, a dash and three digits, such as `bus-003`.
    pub id: String,
    /// One line.
    pub title: String,
    /// The class of attack.
    pub category: String,
    /// The component of ours that is attacked.
    pub target: String,
    /// A defensive description of how the attack works.
    pub how: String,
    /// What should happen.
    pub expected: String,
    /// How bad it is.
    pub severity: Severity,
    /// Public sources, as URLs.
    pub sources: Vec<String>,
    /// Where the defence stands.
    pub status: Status,
    /// For `blocked`: the test or CI job that proves it. Otherwise: the test
    /// to write.
    pub test: String,
    /// The open item that tracks the fix. Required unless `blocked`.
    #[serde(default)]
    pub issue: Option<u64>,
    /// Optional: what the re-check against the code found.
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LaneFile {
    attack: Vec<Attack>,
}

/// The repository root.
#[must_use]
pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The catalog directory.
#[must_use]
pub fn catalog_dir() -> PathBuf {
    repo_root().join("redteam/catalog")
}

/// Loads one lane's file.
///
/// # Errors
///
/// A readable message if the file is missing or does not parse.
pub fn load_lane(file_stem: &str) -> Result<Vec<Attack>, String> {
    let path = catalog_dir().join(format!("{file_stem}.toml"));
    let text = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let parsed: LaneFile = toml::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(parsed.attack)
}

/// Loads every lane, in [`LANES`] order, with the lane's file stem.
///
/// # Errors
///
/// The first lane that fails to load.
pub fn load_all() -> Result<Vec<(&'static str, Vec<Attack>)>, String> {
    LANES
        .iter()
        .map(|(stem, _, _)| load_lane(stem).map(|attacks| (*stem, attacks)))
        .collect()
}

/// Every schema problem in the catalog: shape, ids, issue numbers. An empty
/// list means the catalog is well formed. It does not look at the test tree.
#[must_use]
pub fn schema_problems(lanes: &[(&str, Vec<Attack>)]) -> Vec<String> {
    let mut problems = Vec::new();
    let mut seen = BTreeSet::new();
    for (stem, attacks) in lanes {
        let prefix = LANES
            .iter()
            .find(|(s, _, _)| s == stem)
            .map_or("?", |(_, p, _)| *p);
        if attacks.is_empty() {
            problems.push(format!("{stem}: the lane has no attacks"));
        }
        for a in attacks {
            let id = &a.id;
            if !seen.insert(id.clone()) {
                problems.push(format!("{id}: duplicate id"));
            }
            if !id_is_well_formed(id, prefix) {
                problems.push(format!(
                    "{id}: an id is `{prefix}-` and three digits in lane {stem}"
                ));
            }
            for (name, value) in [
                ("title", &a.title),
                ("category", &a.category),
                ("target", &a.target),
                ("how", &a.how),
                ("expected", &a.expected),
                ("test", &a.test),
            ] {
                if value.trim().is_empty() {
                    problems.push(format!("{id}: `{name}` is empty"));
                }
            }
            if a.sources.is_empty() {
                problems.push(format!("{id}: needs at least one source"));
            }
            for s in &a.sources {
                if !s.starts_with("https://") || s.contains(char::is_whitespace) {
                    problems.push(format!("{id}: source is not an https URL: {s}"));
                }
            }
            match (a.status, a.issue) {
                (Status::Blocked, _) => {}
                (_, None) => problems.push(format!(
                    "{id}: status {} needs an `issue`: no gap without a tracked fix",
                    a.status.word()
                )),
                (_, Some(0)) => problems.push(format!("{id}: `issue` must be a real number")),
                (_, Some(_)) => {}
            }
        }
    }
    problems
}

fn id_is_well_formed(id: &str, prefix: &str) -> bool {
    id.strip_prefix(prefix)
        .and_then(|rest| rest.strip_prefix('-'))
        .is_some_and(|digits| digits.len() == 3 && digits.bytes().all(|b| b.is_ascii_digit()))
}

/// The proofs a `blocked` card names, split on ` + `.
#[must_use]
pub fn proofs(test: &str) -> Vec<&str> {
    test.split(" + ").map(str::trim).collect()
}

/// Why a named proof does not exist, or `None` if it does.
///
/// A proof is one of:
///
/// - `crate-name::test_name`: a `#[test]` function, not ignored, in a `.rs`
///   file under `crates/crate-name/`;
/// - `ci:job-id`: a job key in a workflow under `.github/workflows/`;
/// - `ci:ci/path`: a file under `ci/` (a script or checker run by CI).
#[must_use]
pub fn proof_problem(proof: &str) -> Option<String> {
    let root = repo_root();
    if let Some(ci) = proof.strip_prefix("ci:") {
        if ci.starts_with("ci/") {
            return (!root.join(ci).is_file()).then(|| format!("no such file: {ci}"));
        }
        return (!workflow_jobs(&root).contains(ci))
            .then(|| format!("no workflow job with the id `{ci}`"));
    }
    let Some((krate, name)) = proof.split_once("::") else {
        return Some(format!(
            "`{proof}` is neither `crate::test_name` nor `ci:job`"
        ));
    };
    if !is_ident(name) || krate.contains(['/', '.']) {
        return Some(format!("`{proof}` is not a valid proof name"));
    }
    let dir = root.join("crates").join(krate);
    if !dir.is_dir() {
        return Some(format!("no crate `{krate}` under crates/"));
    }
    let mut files = Vec::new();
    rust_files(&dir, &mut files);
    for file in files {
        if let Ok(text) = fs::read_to_string(&file) {
            if defines_live_test(&text, name) {
                return None;
            }
        }
    }
    Some(format!(
        "no `#[test]` (not ignored) named `{name}` in crate `{krate}`"
    ))
}

fn is_ident(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

/// Job ids across the workflows: keys indented two spaces under `jobs:`.
fn workflow_jobs(root: &Path) -> BTreeSet<String> {
    let mut jobs = BTreeSet::new();
    let Ok(entries) = fs::read_dir(root.join(".github/workflows")) else {
        return jobs;
    };
    for entry in entries.flatten() {
        let Ok(text) = fs::read_to_string(entry.path()) else {
            continue;
        };
        let mut in_jobs = false;
        for line in text.lines() {
            if !line.starts_with(' ') && !line.trim().is_empty() {
                in_jobs = line.trim_end() == "jobs:";
                continue;
            }
            if in_jobs && line.starts_with("  ") && !line.starts_with("   ") {
                if let Some(key) = line.trim().strip_suffix(':') {
                    jobs.insert(key.to_owned());
                }
            }
        }
    }
    jobs
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().is_some_and(|n| n == "target") {
                continue;
            }
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// True if `text` defines `fn name(` directly under a `#[test]` attribute and
/// no attribute on it mentions `ignore`.
fn defines_live_test(text: &str, name: &str) -> bool {
    let lines: Vec<&str> = text.lines().collect();
    let needle = format!("fn {name}(");
    for (i, line) in lines.iter().enumerate() {
        let t = line.trim_start();
        let t = t.strip_prefix("pub ").unwrap_or(t);
        if !t.starts_with(&needle) {
            continue;
        }
        let mut has_test = false;
        let mut ignored = false;
        for prev in lines[..i].iter().rev() {
            let p = prev.trim();
            if !p.starts_with("#[") {
                break;
            }
            has_test |= p == "#[test]";
            ignored |= p.contains("ignore");
        }
        if has_test && !ignored {
            return true;
        }
    }
    false
}

/// Every problem with the proofs of `blocked` cards.
#[must_use]
pub fn proof_problems(lanes: &[(&str, Vec<Attack>)]) -> Vec<String> {
    let mut problems = Vec::new();
    for (_, attacks) in lanes {
        for a in attacks.iter().filter(|a| a.status == Status::Blocked) {
            for proof in proofs(&a.test) {
                if let Some(why) = proof_problem(proof) {
                    problems.push(format!("{}: blocked, but {why}", a.id));
                }
            }
        }
    }
    problems
}

/// Counts of cards by lane stem and status.
#[must_use]
pub fn counts(lanes: &[(&str, Vec<Attack>)]) -> BTreeMap<(String, Status), usize> {
    let mut map = BTreeMap::new();
    for (stem, attacks) in lanes {
        for a in attacks {
            *map.entry(((*stem).to_owned(), a.status)).or_insert(0) += 1;
        }
    }
    map
}

/// The markdown summary table for the README: one row per lane and a total.
#[must_use]
pub fn summary_table(lanes: &[(&str, Vec<Attack>)]) -> String {
    let counts = counts(lanes);
    let get = |stem: &str, s: Status| counts.get(&(stem.to_owned(), s)).copied().unwrap_or(0);
    let mut out = String::from(
        "| Lane | Blocked | Partial | Gap | Total |\n| --- | --- | --- | --- | --- |\n",
    );
    let mut totals = [0usize; 3];
    for (stem, _, title) in LANES {
        let row: Vec<usize> = Status::ALL.iter().map(|s| get(stem, *s)).collect();
        for (t, r) in totals.iter_mut().zip(&row) {
            *t += r;
        }
        out.push_str(&format!(
            "| {title} (`{stem}.toml`) | {} | {} | {} | {} |\n",
            row[0],
            row[1],
            row[2],
            row.iter().sum::<usize>()
        ));
    }
    out.push_str(&format!(
        "| **All lanes** | **{}** | **{}** | **{}** | **{}** |\n",
        totals[0],
        totals[1],
        totals[2],
        totals.iter().sum::<usize>()
    ));
    out
}

/// Marks around the generated summary in `redteam/README.md`.
pub const SUMMARY_START: &str =
    "<!-- summary:start (generated: REDTEAM_BLESS=1 cargo test -p sawbones-redteam) -->";
/// End mark of the generated summary.
pub const SUMMARY_END: &str = "<!-- summary:end -->";

/// The README text with its summary replaced by `table`, or `None` if the
/// marks are missing.
#[must_use]
pub fn with_summary(readme: &str, table: &str) -> Option<String> {
    let start = readme.find(SUMMARY_START)?;
    let end = readme.find(SUMMARY_END)?;
    if end < start {
        return None;
    }
    Some(format!(
        "{}{SUMMARY_START}\n\n{table}\n{}",
        &readme[..start],
        &readme[end..]
    ))
}
