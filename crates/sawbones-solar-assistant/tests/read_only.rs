//! A tripwire for the read-only guarantee.
//!
//! The real guarantee is structural: `ReadOnlyMqtt` keeps the MQTT client in
//! a private field and offers no way to send an application message. This
//! test guards the inside of that wall. It scans the crate's own source and
//! fails if anything that could send a message, or arrange for the broker to
//! send one on our behalf, ever appears.

use std::fs;
use std::path::{Path, PathBuf};

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("source directory is readable") {
        let path = entry.expect("directory entry is readable").path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

fn sources() -> Vec<(PathBuf, String)> {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    rust_files(&src, &mut files);
    assert!(
        files.len() >= 8,
        "expected to find the crate's source files"
    );
    files
        .into_iter()
        .map(|p| {
            let text = fs::read_to_string(&p).expect("source file is readable");
            (p, text)
        })
        .collect()
}

#[test]
fn no_source_file_can_send_an_mqtt_message() {
    // Built from pieces so this file would pass its own scan.
    let forbidden = [
        format!(".{}(", "publish"),
        format!(".{}(", "try_publish"),
        format!("{}_bytes", "publish"),
        format!("set_{}", "last_will"),
        format!("Last{}", "Will"),
        format!(".{}(", "unsubscribe"),
        format!(".{}(", "ack"),
    ];
    for (path, text) in sources() {
        for needle in &forbidden {
            assert!(
                !text.contains(needle.as_str()),
                "{} contains {needle:?}: this crate must stay read-only",
                path.display()
            );
        }
    }
}

#[test]
fn only_the_client_module_touches_the_mqtt_library() {
    for (path, text) in sources() {
        if path.file_name().is_some_and(|n| n == "client.rs") {
            continue;
        }
        for needle in ["rumqttc::", "use rumqttc", "AsyncClient", "EventLoop"] {
            assert!(
                !text.contains(needle),
                "{} mentions {needle:?}: only client.rs may use the MQTT library",
                path.display()
            );
        }
    }
}

#[test]
fn the_client_module_exposes_only_subscribe_and_poll() {
    let (_, text) = sources()
        .into_iter()
        .find(|(p, _)| p.file_name().is_some_and(|n| n == "client.rs"))
        .expect("client.rs exists");
    let start = text
        .find("impl ReadOnlyMqtt {")
        .expect("ReadOnlyMqtt has an impl block");
    let block = &text[start..];
    let end = block.find("\n}\n").expect("the impl block ends");
    let mut public: Vec<&str> = block[..end]
        .lines()
        .map(str::trim)
        .filter(|l| l.starts_with("pub "))
        .filter_map(|l| l.split("fn ").nth(1))
        .filter_map(|l| l.split('(').next())
        .collect();
    public.sort_unstable();
    assert_eq!(public, ["new", "poll", "subscribe"]);
    // And the inner client is not reachable from outside.
    assert!(text.contains("\n    client: AsyncClient,"));
    assert!(text.contains("\n    eventloop: EventLoop,"));
    assert!(!text.contains("pub client:"));
    assert!(!text.contains("pub eventloop:"));
    assert!(!text.contains("impl Deref"));
    assert!(!text.contains("impl std::ops::Deref"));
}
