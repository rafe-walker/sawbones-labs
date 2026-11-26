//! Plugin host v0 (#173): load-time refusals, run limits, hostile output and
//! crashes. Every fixture is built here from WAT, so no wasm toolchain is
//! needed for these tests.

mod support;

use std::time::Duration;

use sawbones_plugin_host::wire::{RefusalCode, Reply, Request};
use sawbones_plugin_host::{
    DecodeFault, FrameFault, FrameOutcome, LoadRefusal, PluginError, PluginHost, RunFault,
};
use sawbones_reading::{Reading, Trust};
use support::*;

fn refused(outcome: &FrameOutcome) -> FrameFault {
    match outcome {
        FrameOutcome::Refused(f) => *f,
        FrameOutcome::Decoded(obs) => panic!("expected a refusal, got {obs:?}"),
    }
}

#[test]
fn a_good_plugin_decodes_into_observations_without_a_tier() {
    let host = host();
    let decoder = host
        .load(
            &decoder_component(&echo_len_core(), Shape::Decoder),
            manifest(),
        )
        .unwrap();
    let out = decoder.decode(&[frame(&[1, 2, 3])]).unwrap();
    let FrameOutcome::Decoded(obs) = &out[0] else {
        panic!("{out:?}")
    };
    assert_eq!(obs.len(), 1);
    assert_eq!(obs[0].quantity.as_str(), "battery_voltage");
    assert_eq!(obs[0].value.text(), "0.3");
    assert_eq!(obs[0].unit.as_ref().unwrap().as_str(), "V");
    assert_eq!(obs[0].source_id.as_str(), "bench/bms_1");
}

#[test]
fn plugin_with_an_import_is_refused() {
    let host = host();
    let bytes = decoder_component(&echo_len_core(), Shape::WithImport);
    assert!(matches!(
        host.load(&bytes, manifest()),
        Err(LoadRefusal::HasImports { count: 1 })
    ));
}

#[test]
fn runner_refuses_an_import_even_without_the_host_scan() {
    // Drive the runner directly, skipping the host's admission scan: the
    // compiled type and the empty linker refuse it on their own.
    let request = Request {
        fuel_per_frame: 1_000_000,
        memory_bytes: 1 << 20,
        max_observations: 4,
        component: decoder_component(&echo_len_core(), Shape::WithImport),
        frames: vec![vec![1]],
    };
    let out = std::process::Command::new(runner())
        .env_clear()
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child.stdin.take().unwrap().write_all(&request.encode())?;
            child.wait_with_output()
        })
        .unwrap();
    assert!(out.status.success());
    match Reply::decode(&out.stdout, 1, 4).unwrap() {
        Reply::Refused { code, .. } => assert_eq!(code, RefusalCode::HasImports),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_core_module_is_not_a_plugin() {
    assert!(matches!(
        host().load(&echo_len_core(), manifest()),
        Err(LoadRefusal::NotAComponent)
    ));
    assert!(matches!(
        host().load(b"not wasm at all", manifest()),
        Err(LoadRefusal::NotAComponent | LoadRefusal::Malformed { .. })
    ));
}

#[test]
fn start_functions_builtins_and_nested_components_are_refused() {
    let host = host();
    let core = echo_len_core();
    assert!(matches!(
        host.load(&decoder_component(&core, Shape::WithStart), manifest()),
        Err(LoadRefusal::StartFunction)
    ));
    assert!(matches!(
        host.load(
            &decoder_component(&core, Shape::WithResourceBuiltin),
            manifest()
        ),
        Err(LoadRefusal::CanonicalBuiltin)
    ));
    assert!(matches!(
        host.load(&decoder_component(&core, Shape::Nested), manifest()),
        Err(LoadRefusal::NestedComponent)
    ));
}

#[test]
fn looping_plugin_is_stopped_by_fuel() {
    let core = core_wat("(loop $l (br $l)) (i32.const 0)", "");
    let decoder = host()
        .load(&decoder_component(&core, Shape::Decoder), manifest())
        .unwrap();
    let out = decoder.decode(&[frame(&[1])]).unwrap();
    assert_eq!(refused(&out[0]), FrameFault::Run(RunFault::OutOfFuel));
}

#[test]
fn looping_plugin_is_killed_at_the_wall_clock_deadline() {
    // Fuel high enough that only the deadline can stop it.
    let mut limits = test_limits();
    limits.fuel_per_frame = u64::MAX / 2;
    limits.deadline = Duration::from_secs(2);
    let host = PluginHost::new(runner(), limits);
    let core = core_wat("(loop $l (br $l)) (i32.const 0)", "");
    let decoder = host
        .load(&decoder_component(&core, Shape::Decoder), manifest())
        .unwrap();
    let started = std::time::Instant::now();
    assert_eq!(decoder.decode(&[frame(&[1])]), Err(PluginError::Deadline));
    assert!(started.elapsed() < Duration::from_secs(10));
}

#[test]
fn plugin_that_allocates_too_much_is_stopped_at_the_memory_cap() {
    let core = core_wat(
        "(loop $l (drop (memory.grow (i32.const 16))) (br $l)) (i32.const 0)",
        "",
    );
    let decoder = host()
        .load(&decoder_component(&core, Shape::Decoder), manifest())
        .unwrap();
    let out = decoder.decode(&[frame(&[1])]).unwrap();
    assert_eq!(refused(&out[0]), FrameFault::Run(RunFault::MemoryLimit));
}

#[test]
fn plugin_declaring_too_much_memory_never_runs() {
    // 1000 pages (64 MiB) is over the 4 MiB test cap.
    let text = r#"(module
      (memory (export "memory") 1000)
      (func (export "cabi_realloc") (param i32 i32 i32 i32) (result i32) (i32.const 1024))
      (func (export "decode") (param i32 i32) (result i32) (i32.const 0)))"#;
    let core = wat::parse_str(text).unwrap();
    let decoder = host()
        .load(&decoder_component(&core, Shape::Decoder), manifest())
        .unwrap();
    let out = decoder.decode(&[frame(&[1])]).unwrap();
    assert_eq!(refused(&out[0]), FrameFault::Run(RunFault::MemoryLimit));
}

#[test]
fn deep_recursion_is_a_stack_fault() {
    let core = core_wat(
        "(call $deep (i32.const 0))",
        "(func $deep (param i32) (result i32) (call $deep (i32.add (local.get 0) (i32.const 1))))",
    );
    let decoder = host()
        .load(&decoder_component(&core, Shape::Decoder), manifest())
        .unwrap();
    let out = decoder.decode(&[frame(&[1])]).unwrap();
    assert_eq!(refused(&out[0]), FrameFault::Run(RunFault::StackOverflow));
}

#[test]
fn oversized_output_is_refused() {
    // One more observation than the cap (256), all inside memory.
    let core = core_wat(&ok_list("(i32.const 1024)", "(i32.const 257)"), "");
    let decoder = host()
        .load(&decoder_component(&core, Shape::Decoder), manifest())
        .unwrap();
    let out = decoder.decode(&[frame(&[1])]).unwrap();
    assert_eq!(
        refused(&out[0]),
        FrameFault::Run(RunFault::TooManyObservations)
    );
}

#[test]
fn oversized_batches_are_refused_before_spawning() {
    let decoder = host()
        .load(
            &decoder_component(&echo_len_core(), Shape::Decoder),
            manifest(),
        )
        .unwrap();
    let big = vec![0u8; test_limits().max_frame_bytes + 1];
    assert_eq!(
        decoder.decode(&[frame(&big)]),
        Err(PluginError::BatchTooLarge)
    );
    let many: Vec<_> = (0..=test_limits().max_frames)
        .map(|_| frame(&[1]))
        .collect();
    assert_eq!(decoder.decode(&many), Err(PluginError::BatchTooLarge));
}

#[test]
fn oversized_component_is_refused_unread() {
    let mut bytes = decoder_component(&echo_len_core(), Shape::Decoder);
    bytes.resize(5 * 1024 * 1024, 0);
    assert!(matches!(
        host().load(&bytes, manifest()),
        Err(LoadRefusal::TooLarge { .. })
    ));
}

#[test]
fn garbage_output_is_refused() {
    let cases: Vec<(&str, String, FrameFault)> = vec![
        (
            "bad result discriminant",
            "(i32.store8 (i32.const 16) (i32.const 7)) (i32.const 16)".into(),
            FrameFault::Run(RunFault::BadOutput),
        ),
        (
            "list out of bounds",
            ok_list("(i32.const 65000)", "(i32.const 1000)"),
            FrameFault::Run(RunFault::BadOutput),
        ),
        (
            "misaligned list",
            ok_list("(i32.const 1025)", "(i32.const 1)"),
            FrameFault::Run(RunFault::BadOutput),
        ),
        (
            "return area out of bounds",
            "(i32.const -8)".into(),
            FrameFault::Run(RunFault::BadOutput),
        ),
        (
            "bad enum value in the error",
            "(i32.store8 (i32.const 16) (i32.const 1)) (i32.store8 (i32.const 20) (i32.const 9)) (i32.const 16)".into(),
            FrameFault::Run(RunFault::BadOutput),
        ),
        (
            "quantity not in the manifest",
            format!(
                "{} {}",
                put_obs(64, 0, 999, "(i64.const 1)", 0),
                ok_list("(i32.const 64)", "(i32.const 1)")
            ),
            FrameFault::UnknownQuantity(999),
        ),
        (
            "exponent out of range",
            format!(
                "{} {}",
                put_obs(64, 0, 0, "(i64.const 1)", 100),
                ok_list("(i32.const 64)", "(i32.const 1)")
            ),
            FrameFault::BadValue,
        ),
        (
            "one bad value refuses the whole frame",
            format!(
                "{} {} {}",
                put_obs(64, 0, 0, "(i64.const 1)", 0),
                put_obs(64, 1, 5, "(i64.const 1)", 0),
                ok_list("(i32.const 64)", "(i32.const 2)")
            ),
            FrameFault::UnknownQuantity(5),
        ),
    ];
    let host = host();
    for (name, body, expected) in cases {
        let core = core_wat(&body, "");
        let decoder = host
            .load(&decoder_component(&core, Shape::Decoder), manifest())
            .unwrap();
        let out = decoder.decode(&[frame(&[1])]).unwrap();
        assert_eq!(refused(&out[0]), expected, "{name}");
    }
}

#[test]
fn a_declined_frame_is_reported_as_the_plugin_said() {
    let core = core_wat(
        "(i32.store8 (i32.const 16) (i32.const 1)) (i32.store8 (i32.const 20) (i32.const 1)) (i32.const 16)",
        "",
    );
    let decoder = host()
        .load(&decoder_component(&core, Shape::Decoder), manifest())
        .unwrap();
    let out = decoder.decode(&[frame(&[1])]).unwrap();
    assert_eq!(
        refused(&out[0]),
        FrameFault::Declined(DecodeFault::UnknownFrame)
    );
}

#[test]
fn plugin_cannot_claim_a_trust_tier() {
    // A component whose observation carries a `trust` field does not match
    // the decoder world, so it is refused before it runs.
    let bytes = decoder_component(&echo_len_core(), Shape::ClaimsTrust);
    assert!(matches!(
        host().load(&bytes, manifest()),
        Err(LoadRefusal::WrongSignature { .. })
    ));
    // What a good plugin returns has no tier: the host attaches one, and
    // the only tier it can sensibly attach to plugin output is unverified.
    let decoder = host()
        .load(
            &decoder_component(&echo_len_core(), Shape::Decoder),
            manifest(),
        )
        .unwrap();
    let FrameOutcome::Decoded(obs) = decoder.decode(&[frame(&[1])]).unwrap().remove(0) else {
        panic!()
    };
    let reading = Reading::from_observation_host_only(obs[0].clone(), Trust::Unverified);
    assert_eq!(reading.trust(), Trust::Unverified);
}

#[test]
fn a_crash_in_the_plugin_does_not_take_down_the_host() {
    // Frame 1 traps; frame 2 in the same batch, and a later batch, still run.
    let core = core_wat(
        &format!(
            "(if (i32.eq (local.get $len) (i32.const 1)) (then unreachable)) {} (i64.store (i32.const 72) (i64.extend_i32_u (local.get $len))) {}",
            put_obs(64, 0, 0, "(i64.const 0)", 0),
            ok_list("(i32.const 64)", "(i32.const 1)")
        ),
        "",
    );
    let decoder = host()
        .load(&decoder_component(&core, Shape::Decoder), manifest())
        .unwrap();
    let out = decoder.decode(&[frame(&[1]), frame(&[1, 2])]).unwrap();
    assert_eq!(refused(&out[0]), FrameFault::Run(RunFault::Trap));
    assert!(matches!(&out[1], FrameOutcome::Decoded(o) if o[0].value.text() == "2"));
    let again = decoder.decode(&[frame(&[1, 2, 3])]).unwrap();
    assert!(matches!(&again[0], FrameOutcome::Decoded(o) if o[0].value.text() == "3"));
}

#[test]
fn each_frame_gets_a_fresh_instance() {
    // sup-004: a frame of length 1 fills memory with a canary; any other
    // frame reports what it finds there. A fresh instance finds zero.
    let core = core_wat(
        &format!(
            "(if (i32.eq (local.get $len) (i32.const 1))
               (then (i64.store (i32.const 4096) (i64.const 0x5A5A5A5A5A5A5A5A))))
             (local.set $x (i64.load (i32.const 4096)))
             {} (i64.store (i32.const 72) (local.get $x)) {}",
            put_obs(64, 0, 0, "(i64.const 0)", 0),
            ok_list("(i32.const 64)", "(i32.const 1)")
        ),
        "",
    );
    let decoder = host()
        .load(&decoder_component(&core, Shape::Decoder), manifest())
        .unwrap();
    let out = decoder.decode(&[frame(&[1]), frame(&[1, 2])]).unwrap();
    assert!(matches!(&out[0], FrameOutcome::Decoded(o) if o[0].value.text() != "0"));
    assert!(matches!(&out[1], FrameOutcome::Decoded(o) if o[0].value.text() == "0"));
}

#[test]
fn admission_limits_are_checked_before_compile() {
    let host = host();
    // A br_table over the limit (4096 targets).
    let targets = "0 ".repeat(5000);
    let core = core_wat(
        &format!("(block (br_table {targets} 0 (i32.const 0))) (i32.const 0)"),
        "",
    );
    assert!(matches!(
        host.load(&decoder_component(&core, Shape::Decoder), manifest()),
        Err(LoadRefusal::OverLimit {
            what: "br_table targets"
        })
    ));
    // Too many locals (10,000).
    let core = core_wat(
        "(i32.const 0)",
        "(func $big (local i64 i64) (local i32) ) (func $many (local i32) (local i32))",
    );
    assert!(host
        .load(&decoder_component(&core, Shape::Decoder), manifest())
        .is_ok());
    let locals = "(local i32) ".repeat(10_001);
    let core = core_wat("(i32.const 0)", &format!("(func $many {locals})"));
    assert!(matches!(
        host.load(&decoder_component(&core, Shape::Decoder), manifest()),
        Err(LoadRefusal::OverLimit { what: "locals" })
    ));
}
