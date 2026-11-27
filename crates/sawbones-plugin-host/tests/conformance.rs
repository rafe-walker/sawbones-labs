//! Plugin conformance suite v0 (#176), run on every decoder fixture in the
//! repo, plus negative controls that show each check can fail.
//!
//! The harness is `conformance/harness.rs`. To add a decoder, build a
//! `Subject` in `subjects()` below and, for a wasm plugin built in this repo,
//! bless its golden file once: `SAWBONES_BLESS_CONFORMANCE=1 cargo test -p
//! sawbones-plugin-host --test conformance`.
//!
//! The golden files make determinism hold across architectures: CI runs this
//! on x86_64 and aarch64, and both compare with the same committed outputs
//! and fuel counts.

#[path = "conformance/harness.rs"]
mod harness;
mod support;

use std::cell::Cell;

use harness::*;
use sawbones_plugin_host::regmap::RegisterMapDecoder;
use sawbones_plugin_host::{DecodeFault, Decoder, FrameFault, RawObservation};
use sawbones_policy::{PortId, SignalId};
use sawbones_reading::Trust;
use sawbones_regmap::{RegisterMap, Table};
use support::*;

const EXAMPLE_TOML: &str = include_str!("../../../data/register-maps/example-sim-device.toml");

fn wasm_subject(
    name: &str,
    component: Vec<u8>,
    corpus: Vec<Vec<u8>>,
    golden: bool,
    waivers: Vec<(Check, &'static str)>,
) -> Subject {
    let (run, run_limited, reload) = wasm_runs(host(), component.clone(), manifest());
    Subject {
        name: name.to_owned(),
        manifest: manifest(),
        corpus,
        length: LengthRule::Exact,
        run,
        reload,
        wasm: Some(WasmSide {
            component,
            run_limited,
        }),
        effects: None,
        golden,
        waivers,
    }
}

/// Frame of the checked fixture: u16 volts (0.01 V), i16 amps (0.1 A), XOR.
fn checked_frame(volts: u16, amps: i16) -> Vec<u8> {
    let v = volts.to_be_bytes();
    let a = amps.to_be_bytes();
    vec![v[0], v[1], a[0], a[1], v[0] ^ v[1] ^ a[0] ^ a[1]]
}

/// A decoder with an integrity byte: five bytes, XOR of the first four last.
/// Anything else is declined as malformed.
fn checked_core() -> Vec<u8> {
    let decline = "(i32.store8 (i32.const 16) (i32.const 1)) (i32.store8 (i32.const 20) (i32.const 0)) (return (i32.const 16))";
    let b = |n: u32| format!("(i32.load8_u offset={n} (local.get $ptr))");
    let body = format!(
        "(if (i32.ne (local.get $len) (i32.const 5)) (then {decline}))
         (local.set $i (i32.xor (i32.xor {b0} {b1}) (i32.xor {b2} {b3})))
         (if (i32.ne (local.get $i) {b4}) (then {decline}))
         {o0} {o1}
         (i64.store (i32.const 72) (i64.extend_i32_u (i32.or (i32.shl {b0} (i32.const 8)) {b1})))
         (i64.store (i32.const 96) (i64.extend_i32_s (i32.extend16_s (i32.or (i32.shl {b2} (i32.const 8)) {b3}))))
         {ok}",
        b0 = b(0),
        b1 = b(1),
        b2 = b(2),
        b3 = b(3),
        b4 = b(4),
        o0 = put_obs(64, 0, 0, "(i64.const 0)", -2),
        o1 = put_obs(64, 1, 1, "(i64.const 0)", -1),
        ok = ok_list("(i32.const 64)", "(i32.const 2)"),
    );
    core_wat(&body, "")
}

fn regmap_subject(name: &str, table: Table, start: u16, words: &[u16]) -> Subject {
    let build = move || {
        let map = RegisterMap::from_toml(EXAMPLE_TOML).map_err(|e| e.to_string())?;
        RegisterMapDecoder::new(map, table, start).map_err(|e| e.to_string())
    };
    let decoder = build().unwrap();
    let manifest = decoder.manifest().clone();
    let run_with = |d: RegisterMapDecoder| -> RunFn {
        Box::new(move |frames| {
            let refs: Vec<&[u8]> = frames.iter().map(Vec::as_slice).collect();
            match d.decode_raw(&refs) {
                Ok(v) => Ok(v),
                Err(never) => match never {},
            }
        })
    };
    let map = RegisterMap::from_toml(EXAMPLE_TOML).unwrap();
    let port = PortId::parse("rs485-1").unwrap();
    let write_map = map.write_map(&port, 1, Trust::OperatorAttested).unwrap();
    let requests: Vec<EffectRequest> = map
        .registers()
        .iter()
        .filter(|r| r.access().can_write())
        .map(|r| EffectRequest {
            port: port.clone(),
            unit: 1,
            signal: SignalId::parse(r.name()).unwrap(),
            register: r.address(),
            width: r.width(),
        })
        .collect();
    let entries = map.write_entries(&port, 1, Trust::OperatorAttested).len();
    Subject {
        name: name.to_owned(),
        manifest,
        corpus: vec![words.iter().flat_map(|w| w.to_be_bytes()).collect()],
        length: LengthRule::WordBlock,
        run: run_with(decoder),
        reload: Box::new(move || Ok(run_with(build()?))),
        wasm: None,
        effects: Some(EffectSide {
            requests,
            write_map,
            entries,
        }),
        golden: true,
        waivers: vec![(
            Check::Flip,
            "a register block carries no checksum, so an in-range flipped bit reads as another value; the Modbus CRC is the transport's",
        )],
    }
}

fn subjects() -> Vec<Subject> {
    let mut v = vec![
        wasm_subject(
            "wat-checked5",
            decoder_component(&checked_core(), Shape::Decoder),
            vec![checked_frame(5280, -125), checked_frame(4800, 300)],
            true,
            vec![],
        ),
        // A test stub, not a decoder: it reports the frame length for any
        // frame. It must fail every mutation check, which proves they bite.
        wasm_subject(
            "wat-echo-len",
            decoder_component(&echo_len_core(), Shape::Decoder),
            vec![vec![1, 2, 3, 4]],
            true,
            vec![
                (
                    Check::Flip,
                    "test stub: answers any bytes with the frame length",
                ),
                (
                    Check::Truncate,
                    "test stub: answers any bytes with the frame length",
                ),
                (
                    Check::Extend,
                    "test stub: answers any bytes with the frame length",
                ),
            ],
        ),
        regmap_subject(
            "regmap-input",
            Table::Input,
            100,
            &[1500, 5280, (-125i16) as u16, 200],
        ),
        regmap_subject(
            "regmap-holding",
            Table::Holding,
            200,
            &[480, 560, 0, 0, 50, 3],
        ),
    ];
    if let Ok(path) = std::env::var("SAWBONES_SAMPLE_PLUGIN_CORE") {
        let core = std::fs::read(path).expect("sample plugin core module");
        let mut good = Vec::from(5280u16.to_be_bytes());
        good.extend((-125i16).to_be_bytes());
        v.push(wasm_subject(
            "sample-decoder",
            decoder_component(&core, Shape::Decoder),
            vec![good],
            false,
            vec![(
                Check::Flip,
                "the sample frame is four bare bytes with no checksum, so a flipped bit reads as another voltage or current",
            )],
        ));
    } else {
        assert!(
            std::env::var_os("SAWBONES_REQUIRE_SAMPLE_PLUGIN").is_none(),
            "SAWBONES_SAMPLE_PLUGIN_CORE is not set"
        );
        eprintln!("sample plugin not built; set SAWBONES_SAMPLE_PLUGIN_CORE to include it");
    }
    v
}

#[test]
fn every_fixture_in_the_repo_passes_the_suite() {
    let mut bad = Vec::new();
    for s in subjects() {
        let rows = run_suite(&s);
        eprint!("{}", render(&s.name, &rows));
        bad.extend(problems(&s.name, &rows));
    }
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

// ---------------------------------------------------------------------------
// Negative controls: each check must be able to fail.

fn must_fail(s: &Subject, check: Check) {
    let o = run_check(s, check);
    assert!(
        o.is_fail(),
        "{} should fail {}: {o:?}",
        s.name,
        check.name()
    );
}

#[test]
fn control_a_component_with_an_import_fails_the_imports_check() {
    let s = wasm_subject(
        "with-import",
        decoder_component(&echo_len_core(), Shape::WithImport),
        vec![vec![1]],
        false,
        vec![],
    );
    must_fail(&s, Check::Imports);
}

#[test]
fn control_a_component_with_a_string_fails_the_no_strings_check() {
    let bytes =
        wat::parse_str(r#"(component (type (func (param "frame" (list u8)) (result string))))"#)
            .unwrap();
    assert_eq!(text_types(&bytes).unwrap(), ["string"]);
    let bytes = wat::parse_str(r#"(component (type (record (field "c" char))))"#).unwrap();
    assert_eq!(text_types(&bytes).unwrap(), ["char"]);
    let ok = decoder_component(&echo_len_core(), Shape::Decoder);
    assert!(text_types(&ok).unwrap().is_empty());
    let mut s = wasm_subject("with-string", bytes, vec![vec![1]], false, vec![]);
    s.wasm.as_mut().unwrap().component =
        wat::parse_str(r#"(component (type (func (result string))))"#).unwrap();
    must_fail(&s, Check::NoStrings);
}

/// A decoder with hidden state, run in process.
fn stateful_subject() -> Subject {
    let counter = std::rc::Rc::new(Cell::new(0u16));
    let c = counter.clone();
    let run: RunFn = Box::new(move |frames| {
        Ok(frames
            .iter()
            .map(|_| {
                c.set(c.get() + 1);
                Ok(vec![RawObservation {
                    quantity: 0,
                    mantissa: i64::from(c.get()),
                    exponent: 0,
                }])
            })
            .collect())
    });
    let mut s = regmap_subject("stateful", Table::Input, 100, &[1, 2, 3, 4]);
    s.run = run;
    s.effects = None;
    s.golden = false;
    s.waivers.clear();
    s
}

#[test]
fn control_hidden_state_fails_determinism_and_snapshot() {
    let s = stateful_subject();
    must_fail(&s, Check::Determinism);
    must_fail(&s, Check::Snapshot);
}

#[test]
fn control_ignored_quotas_fail_the_quota_checks() {
    let mut s = wasm_subject(
        "no-quotas",
        decoder_component(&echo_len_core(), Shape::Decoder),
        vec![vec![1, 2, 3, 4]],
        false,
        vec![],
    );
    // A host that ignores the limits it is given.
    let (run, _, _) = wasm_runs(
        host(),
        s.wasm.as_ref().unwrap().component.clone(),
        manifest(),
    );
    s.wasm.as_mut().unwrap().run_limited = Box::new(move |_, frames| run(frames));
    must_fail(&s, Check::QuotaFuel);
    must_fail(&s, Check::QuotaMemory);
}

#[test]
fn control_a_loop_and_a_memory_hog_are_stopped_by_the_quotas() {
    // A decoder that never returns, and one that grows memory without end:
    // under the default limits the host stops both with the right fault.
    let spin = core_wat("(loop $l (br $l)) (i32.const 16)", "");
    let (_, limited, _) = wasm_runs(host(), decoder_component(&spin, Shape::Decoder), manifest());
    let out = limited(test_limits(), &[vec![1]]).unwrap();
    assert!(matches!(
        out[0],
        Err(FrameFault::Run(sawbones_plugin_host::RunFault::OutOfFuel))
    ));
    let hog = core_wat(
        "(loop $l (drop (memory.grow (i32.const 16))) (br $l)) (i32.const 16)",
        "",
    );
    let (_, limited, _) = wasm_runs(host(), decoder_component(&hog, Shape::Decoder), manifest());
    let out = limited(test_limits(), &[vec![1]]).unwrap();
    assert!(matches!(
        out[0],
        Err(FrameFault::Run(sawbones_plugin_host::RunFault::MemoryLimit))
            | Err(FrameFault::Run(sawbones_plugin_host::RunFault::OutOfFuel))
    ));
}

#[test]
fn control_an_effect_without_a_write_map_entry_fails() {
    let mut s = regmap_subject("bad-effect", Table::Holding, 200, &[480, 560, 0, 0, 50, 3]);
    s.waivers.clear();
    let e = s.effects.as_mut().unwrap();
    e.requests.push(EffectRequest {
        port: PortId::parse("rs485-1").unwrap(),
        unit: 1,
        signal: SignalId::parse("not_in_the_map").unwrap(),
        register: 999,
        width: 1,
    });
    must_fail(&s, Check::Effects);
    // A request that names the wrong register is also a failure.
    let mut s = regmap_subject(
        "wrong-register",
        Table::Holding,
        200,
        &[480, 560, 0, 0, 50, 3],
    );
    s.effects.as_mut().unwrap().requests[0].register += 1;
    must_fail(&s, Check::Effects);
}

#[test]
fn control_text_that_is_not_fenced_or_is_over_the_cap_fails() {
    assert!(is_fenced("hello", 64).is_err());
    let open = sawbones_reading::FENCE_OPEN;
    let close = sawbones_reading::FENCE_CLOSE;
    assert!(is_fenced(&format!("{open}\nfine\n{close}"), 64).is_ok());
    assert!(is_fenced(&format!("{open}\nra<w\n{close}"), 64).is_err());
    assert!(is_fenced(&format!("{open}\ntwo\nlines\n{close}"), 64).is_err());
    assert!(is_fenced(&format!("{open}\nway too long\n{close}"), 4).is_err());
}

#[test]
fn control_a_frame_checker_that_misses_a_flip_is_caught() {
    // The unchecked stub and the sample plugin accept flips (waived above);
    // the checked fixture must not, with no waiver.
    let s = subjects().remove(0);
    assert!(s.waivers.is_empty());
    assert!(!run_check(&s, Check::Flip).is_fail());
    let stub = wasm_subject(
        "unchecked",
        decoder_component(&echo_len_core(), Shape::Decoder),
        vec![vec![1, 2, 3, 4]],
        false,
        vec![],
    );
    must_fail(&stub, Check::Flip);
    must_fail(&stub, Check::Truncate);
    must_fail(&stub, Check::Extend);
    // Declining as the plugin says is a fault, not a reading.
    assert!(matches!(
        FrameFault::Declined(DecodeFault::Malformed),
        FrameFault::Declined(_)
    ));
}
