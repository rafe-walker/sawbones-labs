//! Test fixtures: wrap a core module that exports `memory`, `cabi_realloc`
//! and `decode` into a component of the decoder world (`wit/decoder.wit`),
//! with optional deliberate defects. Also builds core modules from a small
//! WAT template.
//!
//! The canonical ABI layout the core `decode` must follow:
//! `decode(ptr: i32, len: i32) -> i32` returns a pointer to a 12-byte
//! return area: `u8` discriminant at +0 (0 ok, 1 err); for ok a list
//! pointer at +4 and length at +8; for err the enum value at +4.
//! An observation is 24 bytes: `u16` quantity at +0, `s64` mantissa at +8,
//! `s8` exponent at +16.

#![allow(dead_code)]

use std::path::PathBuf;

use sawbones_plugin_host::{
    DecodeContext, DecoderManifest, Frame, Limits, PluginHost, QuantitySpec,
};
use sawbones_reading::{
    Ident, Provenance, TimestampQuality, Transport, Unit, UnitBasis, UtcNanos, WorldText,
};
use wasm_encoder::{
    CanonicalOption, ComponentBuilder, ComponentExportKind, ComponentTypeRef, ComponentValType,
    ExportKind, PrimitiveValType,
};

/// Deliberate defects for refusal tests.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Shape {
    /// A well-formed decoder.
    Decoder,
    /// Also imports a function `now` (a clock): must be refused.
    WithImport,
    /// The observation record has an extra `trust` field: must be refused.
    ClaimsTrust,
    /// Also has a component start function: must be refused.
    WithStart,
    /// Also uses `canon resource.new`: must be refused.
    WithResourceBuiltin,
    /// Nests another component: must be refused.
    Nested,
}

pub fn decoder_component(core: &[u8], shape: Shape) -> Vec<u8> {
    let mut b = ComponentBuilder::default();
    if shape == Shape::WithImport {
        let (ty, mut f) = b.type_function(None);
        f.params(Vec::<(&str, ComponentValType)>::new())
            .result(Some(PrimitiveValType::U64.into()));
        b.import("now", ComponentTypeRef::Func(ty));
    }
    if shape == Shape::Nested {
        let inner = ComponentBuilder::default().finish();
        b.component_raw(None, &inner);
    }
    let module = b.core_module_raw(None, core);
    let instance = b.core_instantiate(None, module, Vec::<(&str, wasm_encoder::ModuleArg)>::new());
    let memory = b.core_alias_export(None, instance, "memory", ExportKind::Memory);
    let realloc = b.core_alias_export(None, instance, "cabi_realloc", ExportKind::Func);
    let decode_core = b.core_alias_export(None, instance, "decode", ExportKind::Func);

    let (decimal, enc) = b.type_defined(None);
    enc.record([
        ("mantissa", ComponentValType::from(PrimitiveValType::S64)),
        ("exponent", PrimitiveValType::S8.into()),
    ]);
    let decimal = b.export("decimal", ComponentExportKind::Type, decimal, None);

    let trust_ty = if shape == Shape::ClaimsTrust {
        let (t, enc) = b.type_defined(None);
        enc.enum_type(["measured", "unverified"]);
        Some(b.export("trust", ComponentExportKind::Type, t, None))
    } else {
        None
    };

    let (obs, enc) = b.type_defined(None);
    let mut fields = vec![
        ("quantity", ComponentValType::from(PrimitiveValType::U16)),
        ("value", ComponentValType::Type(decimal)),
    ];
    if let Some(t) = trust_ty {
        fields.push(("trust", ComponentValType::Type(t)));
    }
    enc.record(fields);
    let obs = b.export("observation", ComponentExportKind::Type, obs, None);

    let (fault, enc) = b.type_defined(None);
    enc.enum_type(["malformed", "unknown-frame", "unsupported"]);
    let fault = b.export("decode-fault", ComponentExportKind::Type, fault, None);

    let (list_obs, enc) = b.type_defined(None);
    enc.list(ComponentValType::Type(obs));
    let (result, enc) = b.type_defined(None);
    enc.result(
        Some(ComponentValType::Type(list_obs)),
        Some(ComponentValType::Type(fault)),
    );
    let (bytes, enc) = b.type_defined(None);
    enc.list(PrimitiveValType::U8);
    let (func_ty, mut f) = b.type_function(None);
    f.params([("frame", ComponentValType::Type(bytes))])
        .result(Some(ComponentValType::Type(result)));

    let decode = b.lift_func(
        None,
        decode_core,
        func_ty,
        [
            CanonicalOption::Memory(memory),
            CanonicalOption::Realloc(realloc),
        ],
    );
    if shape == Shape::WithResourceBuiltin {
        let r = b.type_resource(None, wasm_encoder::ValType::I32, None);
        b.resource_new(r);
    }
    if shape == Shape::WithStart {
        let (ty, mut f) = b.type_function(None);
        f.params(Vec::<(&str, ComponentValType)>::new())
            .result(None);
        let start = b.lift_func(None, realloc, ty, []);
        b.start(start, vec![], 0);
    }
    b.export("decode", ComponentExportKind::Func, decode, None);
    b.finish()
}

/// A core module with a bump allocator, one page of memory (more may be
/// set) and the given `decode` body (params `$ptr`, `$len`, result i32,
/// one i32 local `$i` and one i64 local `$x`). `extra` adds module fields.
pub fn core_wat(decode_body: &str, extra: &str) -> Vec<u8> {
    let text = format!(
        r#"(module
  (memory (export "memory") 1)
  (global $bump (mut i32) (i32.const 32768))
  (func (export "cabi_realloc") (param $old i32) (param $old_size i32) (param $align i32) (param $new_size i32) (result i32)
    (local $p i32)
    (local.set $p (i32.and (i32.add (global.get $bump) (i32.sub (local.get $align) (i32.const 1)))
                           (i32.xor (i32.sub (local.get $align) (i32.const 1)) (i32.const -1))))
    (global.set $bump (i32.add (local.get $p) (local.get $new_size)))
    (if (i32.gt_u (global.get $bump) (i32.mul (memory.size) (i32.const 65536)))
      (then (if (i32.eq (memory.grow (i32.add (i32.shr_u (i32.sub (global.get $bump) (i32.mul (memory.size) (i32.const 65536))) (i32.const 16)) (i32.const 1))) (i32.const -1))
        (then unreachable))))
    (local.get $p))
  (func (export "decode") (param $ptr i32) (param $len i32) (result i32)
    (local $i i32) (local $x i64)
    {decode_body})
  {extra}
)"#
    );
    wat::parse_str(text).expect("fixture WAT parses")
}

/// Writes one observation at `base + 24 * index` in WAT.
pub fn put_obs(base: u32, index: u32, quantity: u32, mantissa: &str, exponent: i32) -> String {
    let at = base + 24 * index;
    format!(
        "(i32.store16 (i32.const {at}) (i32.const {quantity}))
         (i64.store (i32.const {m}) {mantissa})
         (i32.store8 (i32.const {e}) (i32.const {exponent}))",
        m = at + 8,
        e = at + 16
    )
}

/// Sets the return area at 16 to "ok, list at `ptr` of length `len`" and
/// returns it.
pub fn ok_list(ptr: &str, len: &str) -> String {
    format!(
        "(i32.store8 (i32.const 16) (i32.const 0))
         (i32.store (i32.const 20) {ptr})
         (i32.store (i32.const 24) {len})
         (i32.const 16)"
    )
}

/// A decoder that returns one observation: quantity 0, mantissa = frame
/// length, exponent -1.
pub fn echo_len_core() -> Vec<u8> {
    core_wat(
        &format!(
            "{} (i64.store (i32.const 72) (i64.extend_i32_u (local.get $len))) {}",
            put_obs(64, 0, 0, "(i64.const 0)", -1),
            ok_list("(i32.const 64)", "(i32.const 1)")
        ),
        "",
    )
}

pub fn runner() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sawbones-plugin-runner"))
}

pub fn host() -> PluginHost {
    PluginHost::new(runner(), test_limits())
}

pub fn test_limits() -> Limits {
    Limits {
        fuel_per_frame: 2_000_000,
        memory_bytes: 4 * 1024 * 1024,
        ..Limits::default()
    }
}

pub fn manifest() -> DecoderManifest {
    DecoderManifest::new(vec![
        QuantitySpec {
            quantity: Ident::new("battery_voltage").unwrap(),
            unit: Some(Unit::new("V").unwrap()),
            unit_basis: UnitBasis::DeclaredBySource,
        },
        QuantitySpec {
            quantity: Ident::new("battery_current").unwrap(),
            unit: Some(Unit::new("A").unwrap()),
            unit_basis: UnitBasis::DeclaredBySource,
        },
    ])
    .unwrap()
}

pub fn frame(bytes: &[u8]) -> Frame {
    Frame {
        bytes: bytes.to_vec(),
        context: DecodeContext {
            source_id: Ident::new("bench/bms_1").unwrap(),
            received_at: UtcNanos::from_unix_nanos(1_795_631_400_000_000_000),
            timestamp_quality: TimestampQuality::HostReceive {
                uncertainty_ns: 1_000_000,
            },
            provenance: Provenance {
                transport: Transport::Mqtt,
                broker: "bench.local:1883".to_owned(),
                topic: WorldText::new("bench/bms_1/frame", 256),
                raw_payload: WorldText::from_bytes_lossy(bytes, 256),
                retained: false,
            },
        },
    }
}
