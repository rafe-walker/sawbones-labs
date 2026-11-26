//! Repository checks for the plugin host: red-team cards sup-001, sup-002,
//! sup-003 and sup-006.

use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Every file under `dir` (skipping build output and VCS data).
fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name == "target" || name == ".git" || name == "node_modules" {
            continue;
        }
        if path.is_dir() {
            walk(&path, out);
        } else {
            out.push(path);
        }
    }
}

fn lock_versions(name: &str) -> Vec<(u64, u64, u64)> {
    let lock = std::fs::read_to_string(repo_root().join("Cargo.lock")).unwrap();
    let mut out = Vec::new();
    let mut lines = lock.lines();
    while let Some(line) = lines.next() {
        if line == format!("name = \"{name}\"") {
            let v = lines.next().unwrap();
            let v = v.trim_start_matches("version = \"").trim_end_matches('"');
            let mut parts = v.split(['.', '-']).map(|p| p.parse::<u64>().unwrap_or(0));
            out.push((
                parts.next().unwrap_or(0),
                parts.next().unwrap_or(0),
                parts.next().unwrap_or(0),
            ));
        }
    }
    out
}

fn floor(line_key: &str) -> (u64, u64, u64) {
    let text = std::fs::read_to_string(repo_root().join("ci/min-wasmtime.txt")).unwrap();
    let line = text
        .lines()
        .map(str::trim)
        .find(|l| l.starts_with(line_key))
        .unwrap_or_else(|| panic!("{line_key} missing from ci/min-wasmtime.txt"));
    let v = line.split_whitespace().nth(1).unwrap();
    let mut parts = v.split('.').map(|p| p.parse::<u64>().unwrap());
    (
        parts.next().unwrap(),
        parts.next().unwrap(),
        parts.next().unwrap(),
    )
}

/// sup-001, sup-002: the lock never resolves to a wasmtime or Cranelift
/// below the advisory floor, and Winch is not in the tree.
#[test]
fn wasmtime_and_cranelift_are_at_or_above_the_advisory_floor() {
    for (krate, key) in [
        ("wasmtime", "wasmtime"),
        ("cranelift-codegen", "cranelift-codegen"),
    ] {
        let versions = lock_versions(krate);
        assert!(!versions.is_empty(), "{krate} not in Cargo.lock");
        let min = floor(key);
        for v in versions {
            assert!(v >= min, "{krate} {v:?} is below the floor {min:?}");
        }
    }
    assert!(
        lock_versions("winch-codegen").is_empty(),
        "Winch must not be in the tree"
    );
}

/// sup-003: no text type in any plugin world.
#[test]
fn wit_worlds_have_no_text() {
    let mut files = Vec::new();
    walk(&repo_root(), &mut files);
    let wits: Vec<_> = files
        .iter()
        .filter(|p| p.extension().is_some_and(|e| e == "wit"))
        .collect();
    assert!(!wits.is_empty(), "the decoder world should be found");
    for path in wits {
        let text = std::fs::read_to_string(path).unwrap();
        for (n, line) in text.lines().enumerate() {
            let code = line.split("//").next().unwrap_or("");
            for word in code.split(|c: char| !(c.is_ascii_alphanumeric() || c == '-')) {
                assert!(
                    !matches!(word, "string" | "char" | "flags"),
                    "{}:{}: `{word}` in a plugin world",
                    path.display(),
                    n + 1
                );
            }
        }
    }
}

/// sup-006: nothing loads precompiled code, and no precompiled artifact is
/// in the repository.
#[test]
fn no_precompiled_code_is_loaded_or_committed() {
    let mut files = Vec::new();
    walk(&repo_root(), &mut files);
    for path in &files {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        assert!(
            !matches!(ext, "cwasm" | "wasm"),
            "{}: compiled wasm must not be committed",
            path.display()
        );
        if ext == "rs" && !path.ends_with("tests/static_checks.rs") {
            let text = std::fs::read_to_string(path).unwrap();
            for banned in [
                "::deserialize_file(",
                "::deserialize_raw(",
                "Module::deserialize(",
                "Component::deserialize(",
                "deserialize_open_file(",
            ] {
                assert!(!text.contains(banned), "{}: calls {banned}", path.display());
            }
        }
    }
}

/// sup-002, sup-004: the wasmtime features are pinned to the smallest set:
/// no Winch, no pooling allocator (one fresh process per batch and a fresh
/// instance per frame instead), no threads, no async, no GC, no cache of
/// precompiled code, no WAT parser in the host.
#[test]
fn wasmtime_features_are_pinned_to_the_smallest_set() {
    let manifest =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml")).unwrap();
    let line = manifest
        .lines()
        .find(|l| l.starts_with("wasmtime = "))
        .expect("wasmtime dependency line");
    assert!(line.contains("default-features = false"), "{line}");
    assert!(
        line.contains(r#"features = ["runtime", "cranelift", "component-model", "std"]"#),
        "wasmtime features changed: {line}"
    );
}
