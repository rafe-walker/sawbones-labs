//! The register-map decoder (#174) through the plugin host's `Decoder`
//! seam: the same manifest checks and conversion as a wasm plugin, and the
//! same observations `RegisterMap::observe` gives.

mod support;

use sawbones_plugin_host::regmap::RegisterMapDecoder;
use sawbones_plugin_host::{decode_frames, DecodeFault, Decoder, FrameFault, FrameOutcome};
use sawbones_regmap::{ObservationContext, RegisterMap, Table};
use support::frame;

const EXAMPLE: &str = include_str!("../../../data/register-maps/example-sim-device.toml");

fn words_to_bytes(words: &[u16]) -> Vec<u8> {
    words.iter().flat_map(|w| w.to_be_bytes()).collect()
}

#[test]
fn register_map_runs_through_the_same_decoder_seam() {
    let map = RegisterMap::from_toml(EXAMPLE).unwrap();
    // Input registers 101 to 104 (wire 100 to 103): power, voltage,
    // current, temperature. energy_total (105-106) is not wholly inside.
    let words = [1500u16, 5280, (-125i16) as u16, 200];
    let decoder = RegisterMapDecoder::new(map.clone(), Table::Input, 100).unwrap();
    let f = frame(&words_to_bytes(&words));
    let Ok(out) = decode_frames(&decoder, std::slice::from_ref(&f));
    let FrameOutcome::Decoded(obs) = &out[0] else {
        panic!("{out:?}")
    };
    let ctx = ObservationContext {
        source_id: f.context.source_id.clone(),
        received_at: f.context.received_at,
        source_timestamp: None,
        timestamp_quality: f.context.timestamp_quality,
        provenance: f.context.provenance.clone(),
    };
    let names = [
        "pv_power",
        "battery_voltage",
        "battery_current",
        "heatsink_temperature",
    ];
    assert_eq!(obs.len(), names.len());
    for (i, name) in names.iter().enumerate() {
        let direct = map.observe(name, &words[i..=i], &ctx).unwrap();
        assert_eq!(obs[i], direct, "{name}");
    }
    assert_eq!(obs[1].value.text(), "52.80");
    assert!(decoder.manifest().len() >= names.len());
}

#[test]
fn register_map_frame_with_a_bad_value_is_refused_whole() {
    let map = RegisterMap::from_toml(EXAMPLE).unwrap();
    let decoder = RegisterMapDecoder::new(map, Table::Input, 100).unwrap();
    // Battery voltage 99.99 V is outside the map's valid range.
    let bad = frame(&words_to_bytes(&[1500, 9999, 0, 200]));
    let odd = frame(&[0, 1, 2]);
    let Ok(out) = decode_frames(&decoder, &[bad, odd]);
    for o in out {
        assert_eq!(
            o,
            FrameOutcome::Refused(FrameFault::Declined(DecodeFault::Malformed))
        );
    }
}
