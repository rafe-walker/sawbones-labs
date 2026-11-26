//! The sample decoder plugin in `plugins/sample-decoder`, built from Rust
//! for wasm32 (#173).
//!
//! CI builds it for `wasm32-unknown-unknown` and sets
//! `SAWBONES_SAMPLE_PLUGIN_CORE` to the core module, and
//! `SAWBONES_REQUIRE_SAMPLE_PLUGIN=1` so a missing file fails. Locally,
//! without the wasm target, the test has nothing to load and passes.

mod support;

use sawbones_plugin_host::{DecodeFault, FrameFault, FrameOutcome};
use support::*;

#[test]
fn the_rust_sample_plugin_decodes_and_declines() {
    let Ok(path) = std::env::var("SAWBONES_SAMPLE_PLUGIN_CORE") else {
        assert!(
            std::env::var_os("SAWBONES_REQUIRE_SAMPLE_PLUGIN").is_none(),
            "SAWBONES_SAMPLE_PLUGIN_CORE is not set"
        );
        eprintln!("sample plugin not built; set SAWBONES_SAMPLE_PLUGIN_CORE to run this test");
        return;
    };
    let core = std::fs::read(&path).expect("sample plugin core module");
    let decoder = host()
        .load(&decoder_component(&core, Shape::Decoder), manifest())
        .expect("the sample plugin imports nothing and matches the decoder world");
    // 52.80 V and -12.5 A.
    let volts = 5280u16.to_be_bytes();
    let amps = (-125i16).to_be_bytes();
    let good = [volts[0], volts[1], amps[0], amps[1]];
    let out = decoder.decode(&[frame(&good), frame(&[1, 2, 3])]).unwrap();
    let FrameOutcome::Decoded(obs) = &out[0] else {
        panic!("{out:?}")
    };
    let got: Vec<(&str, &str, &str)> = obs
        .iter()
        .map(|o| {
            (
                o.quantity.as_str(),
                o.value.text(),
                o.unit.as_ref().unwrap().as_str(),
            )
        })
        .collect();
    assert_eq!(
        got,
        [
            ("battery_voltage", "52.80", "V"),
            ("battery_current", "-12.5", "A")
        ]
    );
    assert_eq!(
        out[1],
        FrameOutcome::Refused(FrameFault::Declined(DecodeFault::Malformed))
    );
}
