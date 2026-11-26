//! The default build contains no private key.
//!
//! Every test seed starts with one marker. This test checks the source: the
//! marker appears only in `src/test_keys.rs`, that module is compiled only
//! with the `test-keys` feature, and the feature is not on by default. The CI
//! job `test-keys` checks the output: it builds the crate without the
//! feature for the firmware target and fails if the marker is in any
//! artifact, after checking that a build with the feature does contain it.

use std::fs;
use std::path::Path;

use sawbones_approvals::test_keys::ALL_SEEDS;

const MARKER: &str = concat!("SAWBONES-", "TEST-KEY");

#[test]
fn every_test_seed_carries_the_marker() {
    for seed in ALL_SEEDS {
        assert!(seed.starts_with(MARKER.as_bytes()));
    }
}

#[test]
fn the_default_build_contains_no_private_key() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut sources = 0;
    for entry in fs::read_dir(root.join("src")).unwrap() {
        let path = entry.unwrap().path();
        let text = fs::read_to_string(&path).unwrap();
        sources += 1;
        if path.file_name().unwrap() == "test_keys.rs" {
            assert!(text.contains(MARKER));
        } else {
            assert!(
                !text.contains(MARKER),
                "{} holds a test seed outside test_keys.rs",
                path.display()
            );
        }
    }
    assert!(sources >= 2);

    let lib = fs::read_to_string(root.join("src/lib.rs")).unwrap();
    assert!(
        lib.contains("#[cfg(feature = \"test-keys\")]\npub mod test_keys;"),
        "test_keys must be compiled only with the test-keys feature"
    );
    assert_eq!(lib.matches("mod test_keys").count(), 1);

    let manifest = fs::read_to_string(root.join("Cargo.toml")).unwrap();
    let default = manifest
        .lines()
        .find(|line| line.starts_with("default"))
        .expect("a default feature list");
    assert_eq!(default.trim(), "default = []");
}
