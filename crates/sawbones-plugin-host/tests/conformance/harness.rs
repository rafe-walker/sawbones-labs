//! Plugin conformance suite v0 (#176): the harness.
//!
//! A [`Subject`] is any decoder: a wasm plugin in the sandboxed host, or a
//! data-only decoder run in process. [`run_suite`] runs every check on it and
//! gives one [`Outcome`] per [`Check`]. A check never panics on a defect; it
//! returns [`Outcome::Fail`] with the reasons, so a fixture can say which
//! failures it expects, and the suite can be tested against deliberately
//! broken subjects (its own negative controls).
//!
//! Checks in v0 (numbers are the order in the issue):
//!
//! 1. `imports`: the component imports nothing.
//! 2. `determinism`: the same frames give the same outputs and the same fuel,
//!    twice in process, after a reload, and equal to a committed golden file
//!    (the golden file is what makes x86_64 and aarch64 agree: CI runs the
//!    suite on both and both compare with the same file).
//! 3. `flip`, `truncate`, `extend`: mutated frames give a fault, never a
//!    plausible reading.
//! 4. `effects`: every effect request resolves to a write-map entry.
//! 5. `no-strings`: output types contain no string.
//! 6. `snapshot`: snapshot and restore round-trip.
//! 7. `quota-fuel`, `quota-memory`: quotas are enforced.
//! 8. `manifest-text`: text is capped and fenced.

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::PathBuf;

use sawbones_plugin_host::{
    DecoderManifest, FrameFault, Limits, PluginHost, RawOutcome, RunFault, WasmDecoder,
};
use sawbones_policy::{PortId, SignalId, WriteMap};
use sawbones_reading::{Ident, Unit, WorldText};
use wasmparser::{
    ComponentDefinedType, ComponentExternalKind, ComponentType, ComponentTypeDeclaration,
    ComponentValType, Parser, Payload, PrimitiveValType,
};

/// A decoder run: frames in, one raw outcome per frame out, or why it could
/// not run at all.
pub type RunFn = Box<dyn Fn(&[Vec<u8>]) -> Result<Vec<RawOutcome>, String>>;
/// A decoder run under chosen limits (wasm only).
pub type LimitedRunFn = Box<dyn Fn(Limits, &[Vec<u8>]) -> Result<Vec<RawOutcome>, String>>;
/// Builds a fresh run from the subject's stored form: the restore half of a
/// snapshot.
pub type ReloadFn = Box<dyn Fn() -> Result<RunFn, String>>;

/// What the length of a valid frame means.
#[derive(Clone, Copy, Debug)]
pub enum LengthRule {
    /// Only these byte lengths are frames. Any other length must be refused.
    Exact,
    /// A block of 16-bit words of any length. An odd length must be refused;
    /// a shorter or longer block may decode, but only to the same readings
    /// the full frame gave for the registers it still covers (the original's
    /// observations are a prefix).
    WordBlock,
}

/// The checks, in order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Check {
    /// 1.
    Imports,
    /// 2.
    Determinism,
    /// 3, bit flips.
    Flip,
    /// 3, truncation.
    Truncate,
    /// 3, extension.
    Extend,
    /// 4.
    Effects,
    /// 5.
    NoStrings,
    /// 6.
    Snapshot,
    /// 7, fuel.
    QuotaFuel,
    /// 7, memory.
    QuotaMemory,
    /// 8.
    ManifestText,
}

impl Check {
    /// Every check.
    pub const ALL: [Check; 11] = [
        Check::Imports,
        Check::Determinism,
        Check::Flip,
        Check::Truncate,
        Check::Extend,
        Check::Effects,
        Check::NoStrings,
        Check::Snapshot,
        Check::QuotaFuel,
        Check::QuotaMemory,
        Check::ManifestText,
    ];

    /// A short name.
    pub fn name(self) -> &'static str {
        match self {
            Check::Imports => "imports",
            Check::Determinism => "determinism",
            Check::Flip => "flip",
            Check::Truncate => "truncate",
            Check::Extend => "extend",
            Check::Effects => "effects",
            Check::NoStrings => "no-strings",
            Check::Snapshot => "snapshot",
            Check::QuotaFuel => "quota-fuel",
            Check::QuotaMemory => "quota-memory",
            Check::ManifestText => "manifest-text",
        }
    }
}

/// What a check found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// The subject meets the check. The text says what was run.
    Pass(String),
    /// The check cannot apply to this kind of subject, and why.
    NotApplicable(String),
    /// The subject breaks the check. Each line is one finding.
    Fail(Vec<String>),
}

impl Outcome {
    fn fail(line: impl Into<String>) -> Self {
        Outcome::Fail(vec![line.into()])
    }
    /// Whether this is a failure.
    pub fn is_fail(&self) -> bool {
        matches!(self, Outcome::Fail(_))
    }
}

/// The wasm side of a subject.
pub struct WasmSide {
    /// The component, as the host would admit it.
    pub component: Vec<u8>,
    /// Runs the decoder under the given limits.
    pub run_limited: LimitedRunFn,
}

/// One effect request a subject can make.
#[derive(Clone, Debug)]
pub struct EffectRequest {
    /// Port.
    pub port: PortId,
    /// Unit.
    pub unit: u8,
    /// Signal.
    pub signal: SignalId,
    /// The register the subject says it writes.
    pub register: u16,
    /// Its width.
    pub width: u8,
}

/// The effect side of a subject: what it can ask to write, and the write map
/// the policy engine would hold for it.
pub struct EffectSide {
    /// Every effect the subject can request.
    pub requests: Vec<EffectRequest>,
    /// The write map.
    pub write_map: WriteMap,
    /// How many entries the write map should have: no entry without a request.
    pub entries: usize,
}

/// A decoder under test.
pub struct Subject {
    /// A name for reports and golden files.
    pub name: String,
    /// The manifest.
    pub manifest: DecoderManifest,
    /// Valid frames. Each must decode.
    pub corpus: Vec<Vec<u8>>,
    /// What a frame's length means.
    pub length: LengthRule,
    /// Runs the decoder.
    pub run: RunFn,
    /// Restores a run from the stored form.
    pub reload: ReloadFn,
    /// The wasm side, if it is a plugin.
    pub wasm: Option<WasmSide>,
    /// The effect side, if it can request effects.
    pub effects: Option<EffectSide>,
    /// Compare with a committed golden file (outputs and fuel). Only for
    /// subjects whose bytes are built in the repo by code that does not move
    /// with the toolchain.
    pub golden: bool,
    /// Failures this subject is known to have, with the reason. A listed
    /// check that fails is reported as waived; a listed check that passes is
    /// an error, so a fixed defect cannot keep its waiver.
    pub waivers: Vec<(Check, &'static str)>,
}

/// One row of a report.
#[derive(Debug)]
pub struct Row {
    /// The check.
    pub check: Check,
    /// What it found.
    pub outcome: Outcome,
    /// The waiver reason, if the outcome is a failure the subject lists.
    pub waived: Option<&'static str>,
}

/// Runs every check.
pub fn run_suite(s: &Subject) -> Vec<Row> {
    Check::ALL
        .iter()
        .map(|&check| {
            let outcome = run_check(s, check);
            let waived = s
                .waivers
                .iter()
                .find(|(c, _)| *c == check)
                .map(|(_, why)| *why);
            Row {
                check,
                outcome,
                waived,
            }
        })
        .collect()
}

/// Runs one check.
pub fn run_check(s: &Subject, check: Check) -> Outcome {
    match check {
        Check::Imports => check_imports(s),
        Check::Determinism => check_determinism(s),
        Check::Flip => check_mutants(s, Mutation::Flip),
        Check::Truncate => check_mutants(s, Mutation::Truncate),
        Check::Extend => check_mutants(s, Mutation::Extend),
        Check::Effects => check_effects(s),
        Check::NoStrings => check_no_strings(s),
        Check::Snapshot => check_snapshot(s),
        Check::QuotaFuel => check_quota_fuel(s),
        Check::QuotaMemory => check_quota_memory(s),
        Check::ManifestText => check_manifest_text(s),
    }
}

/// The problems in a report: a failure with no waiver, or a waiver on a check
/// that no longer fails.
pub fn problems(subject: &str, rows: &[Row]) -> Vec<String> {
    let mut out = Vec::new();
    for r in rows {
        match (&r.outcome, r.waived) {
            (Outcome::Fail(lines), None) => out.push(format!(
                "{subject}: {} failed: {}",
                r.check.name(),
                lines.join("; ")
            )),
            (Outcome::Pass(_) | Outcome::NotApplicable(_), Some(why)) => out.push(format!(
                "{subject}: {} is waived ({why}) but passes: remove the waiver",
                r.check.name()
            )),
            _ => {}
        }
    }
    out
}

/// A report as text, one line per check.
pub fn render(subject: &str, rows: &[Row]) -> String {
    let mut out = String::new();
    for r in rows {
        let (tag, text) = match (&r.outcome, r.waived) {
            (Outcome::Pass(t), _) => ("pass", t.clone()),
            (Outcome::NotApplicable(t), _) => ("n/a ", t.clone()),
            (Outcome::Fail(l), Some(why)) => {
                let last = l.last().cloned().unwrap_or_default();
                ("WAIVED", format!("{why} [{last}]"))
            }
            (Outcome::Fail(l), None) => ("FAIL", l.join("; ")),
        };
        let _ = writeln!(out, "{subject:<18} {:<14} {tag:<6} {text}", r.check.name());
    }
    out
}

// ---------------------------------------------------------------------------
// Running, with batching.

/// Chunks `frames` into batches of at most `batch` and runs each.
pub fn run_batched<E: std::fmt::Debug>(
    frames: &[Vec<u8>],
    batch: usize,
    mut f: impl FnMut(&[Vec<u8>]) -> Result<Vec<RawOutcome>, E>,
) -> Result<Vec<RawOutcome>, String> {
    let mut out = Vec::new();
    for chunk in frames.chunks(batch.max(1)) {
        out.extend(f(chunk).map_err(|e| format!("{e:?}"))?);
    }
    Ok(out)
}

/// A wasm subject's run closures, from a component and manifest.
pub fn wasm_runs(
    host: PluginHost,
    component: Vec<u8>,
    manifest: DecoderManifest,
) -> (RunFn, LimitedRunFn, ReloadFn) {
    use sawbones_plugin_host::Decoder;
    let load = {
        let component = component.clone();
        let manifest = manifest.clone();
        move |host: &PluginHost| -> Result<WasmDecoder, String> {
            host.load(&component, manifest.clone())
                .map_err(|e| format!("{e}"))
        }
    };
    let run: RunFn = {
        let host = host.clone();
        let load = load.clone();
        let decoder = load(&host);
        Box::new(move |frames| {
            let d = decoder.as_ref().map_err(Clone::clone)?;
            let batch = host.limits().max_frames;
            run_batched(frames, batch, |chunk| {
                let refs: Vec<&[u8]> = chunk.iter().map(Vec::as_slice).collect();
                d.decode_raw(&refs)
            })
        })
    };
    let limited: LimitedRunFn = {
        let host = host.clone();
        let load = load.clone();
        Box::new(move |limits, frames| {
            let h = PluginHost::new(runner_path(&host), limits);
            let d = load(&h)?;
            run_batched(frames, h.limits().max_frames, |chunk| {
                let refs: Vec<&[u8]> = chunk.iter().map(Vec::as_slice).collect();
                d.decode_raw(&refs)
            })
        })
    };
    let reload: ReloadFn = {
        let host = host.clone();
        Box::new(move || {
            let d = load(&host)?;
            let batch = host.limits().max_frames;
            let run: RunFn = Box::new(move |frames| {
                run_batched(frames, batch, |chunk| {
                    let refs: Vec<&[u8]> = chunk.iter().map(Vec::as_slice).collect();
                    d.decode_raw(&refs)
                })
            });
            Ok(run)
        })
    };
    (run, limited, reload)
}

fn runner_path(_host: &PluginHost) -> PathBuf {
    match std::env::var_os("SAWBONES_PLUGIN_RUNNER") {
        Some(p) => PathBuf::from(p),
        None => PathBuf::from(env!("CARGO_BIN_EXE_sawbones-plugin-runner")),
    }
}

// ---------------------------------------------------------------------------
// Mutation.

/// Kinds of mutation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mutation {
    /// Flip one bit.
    Flip,
    /// Cut bytes off the end.
    Truncate,
    /// Add bytes to the end.
    Extend,
}

/// Every mutant of `frame` of one kind. Bit flips cover every bit of frames up
/// to 64 bytes and every 5th bit of longer ones.
pub fn mutants(frame: &[u8], kind: Mutation) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    match kind {
        Mutation::Flip => {
            let stride = if frame.len() <= 64 { 1 } else { 5 };
            for bit in (0..frame.len() * 8).step_by(stride) {
                let mut m = frame.to_vec();
                m[bit / 8] ^= 1 << (bit % 8);
                out.push(m);
            }
        }
        Mutation::Truncate => {
            for len in 0..frame.len() {
                out.push(frame[..len].to_vec());
            }
        }
        Mutation::Extend => {
            for extra in [1usize, 2, 3, 4, 8] {
                for fill in [0x00u8, 0xFF, 0xA5] {
                    let mut m = frame.to_vec();
                    m.extend(std::iter::repeat_n(fill, extra));
                    if m.len() <= 4096 {
                        out.push(m);
                    }
                }
            }
        }
    }
    out
}

fn hex(b: &[u8]) -> String {
    if b.is_empty() {
        return "-".to_owned();
    }
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn is_refused(o: &RawOutcome) -> bool {
    o.is_err()
}

fn check_mutants(s: &Subject, kind: Mutation) -> Outcome {
    let mut findings = Vec::new();
    let mut total = 0usize;
    let mut accepted = 0usize;
    for original in &s.corpus {
        let base = match (s.run)(std::slice::from_ref(original)) {
            Ok(mut v) if v.len() == 1 => v.remove(0),
            Ok(_) => return Outcome::fail("wrong number of outcomes"),
            Err(e) => return Outcome::fail(format!("could not run: {e}")),
        };
        let Ok(base_obs) = &base else {
            findings.push(format!(
                "corpus frame {} does not decode: {base:?}",
                hex(original)
            ));
            continue;
        };
        let ms = mutants(original, kind);
        total += ms.len();
        let outs = match (s.run)(&ms) {
            Ok(o) => o,
            Err(e) => return Outcome::fail(format!("could not run mutants: {e}")),
        };
        for (m, o) in ms.iter().zip(&outs) {
            if is_refused(o) {
                continue;
            }
            let allowed = match (kind, s.length) {
                (Mutation::Flip, _) => false,
                (_, LengthRule::Exact) => false,
                (_, LengthRule::WordBlock) => {
                    m.len() % 2 == 0 && o.as_ref().is_ok_and(|got| is_prefix(base_obs, got, kind))
                }
            };
            if !allowed {
                accepted += 1;
                if findings.len() < 4 {
                    findings.push(format!(
                        "{} {} of {} gave {:?}",
                        kind_word(kind),
                        hex(m),
                        hex(original),
                        o
                    ));
                }
            }
        }
    }
    if accepted > 0 || !findings.is_empty() {
        findings.push(format!("{accepted} of {total} mutants gave a reading"));
        return Outcome::Fail(findings);
    }
    Outcome::Pass(format!(
        "{total} {} mutants, all refused or consistent",
        kind_word(kind)
    ))
}

fn kind_word(k: Mutation) -> &'static str {
    match k {
        Mutation::Flip => "flip",
        Mutation::Truncate => "truncation",
        Mutation::Extend => "extension",
    }
}

/// For a word block: a truncation's readings are a prefix of the original's
/// (fewer registers), an extension's readings begin with the original's.
fn is_prefix<T: PartialEq>(base: &[T], got: &[T], kind: Mutation) -> bool {
    match kind {
        Mutation::Truncate => got.len() <= base.len() && base[..got.len()] == *got,
        Mutation::Extend => got.len() >= base.len() && got[..base.len()] == *base,
        Mutation::Flip => false,
    }
}

// ---------------------------------------------------------------------------
// 1 and 5: static scans of the component.

/// Imports found in the component bytes, at any depth.
pub fn count_imports(component: &[u8]) -> Result<u32, String> {
    let mut n = 0u32;
    for payload in Parser::new(0).parse_all(component) {
        match payload.map_err(|e| e.message().to_owned())? {
            Payload::ComponentImportSection(r) => n += r.count(),
            Payload::ImportSection(r) => {
                // A core module's imports are wired inside the component; they
                // only matter if the component itself imports. Counted apart.
                let _ = r;
            }
            _ => {}
        }
    }
    Ok(n)
}

fn check_imports(s: &Subject) -> Outcome {
    let Some(w) = &s.wasm else {
        return Outcome::NotApplicable("data-only decoder: no component, nothing to import".into());
    };
    let found = match count_imports(&w.component) {
        Ok(n) => n,
        Err(e) => return Outcome::fail(format!("component does not parse: {e}")),
    };
    if found != 0 {
        return Outcome::fail(format!("component has {found} imports"));
    }
    // The compiled type must agree with the byte scan.
    let engine = match wasmtime::Engine::new(&sawbones_plugin_host::runner::engine_config(1 << 20))
    {
        Ok(e) => e,
        Err(e) => return Outcome::fail(format!("engine: {e}")),
    };
    match wasmtime::component::Component::new(&engine, &w.component) {
        Ok(c) => {
            let n = c.component_type().imports(&engine).len();
            if n != 0 {
                return Outcome::fail(format!("compiled component has {n} imports"));
            }
        }
        Err(e) => return Outcome::fail(format!("component does not compile: {e}")),
    }
    // And the host must admit it.
    if let Err(e) = (s.reload)() {
        return Outcome::fail(format!("the host refuses it: {e}"));
    }
    Outcome::Pass("byte scan and compiled type: zero imports; admitted by the host".into())
}

/// Text-bearing types found in a component's type section.
pub fn text_types(component: &[u8]) -> Result<Vec<String>, String> {
    let mut found = Vec::new();
    for payload in Parser::new(0).parse_all(component) {
        if let Payload::ComponentTypeSection(r) = payload.map_err(|e| e.message().to_owned())? {
            for ty in r {
                scan_type(&ty.map_err(|e| e.message().to_owned())?, &mut found);
            }
        }
    }
    Ok(found)
}

fn val_text(v: &ComponentValType, found: &mut Vec<String>) {
    if let ComponentValType::Primitive(p) = v {
        match p {
            PrimitiveValType::String => found.push("string".into()),
            PrimitiveValType::Char => found.push("char".into()),
            _ => {}
        }
    }
}

fn scan_type(ty: &ComponentType, found: &mut Vec<String>) {
    match ty {
        ComponentType::Defined(d) => match d {
            ComponentDefinedType::Primitive(p) => val_text(&ComponentValType::Primitive(*p), found),
            ComponentDefinedType::Record(fields) => {
                for (_, v) in fields.iter() {
                    val_text(v, found);
                }
            }
            ComponentDefinedType::Variant(cases) => {
                for c in cases.iter() {
                    if let Some(v) = &c.ty {
                        val_text(v, found);
                    }
                }
            }
            ComponentDefinedType::List(v) | ComponentDefinedType::Option(v) => val_text(v, found),
            ComponentDefinedType::Tuple(vs) => {
                for v in vs.iter() {
                    val_text(v, found);
                }
            }
            ComponentDefinedType::Result { ok, err } => {
                for v in [ok, err].into_iter().flatten() {
                    val_text(v, found);
                }
            }
            ComponentDefinedType::Flags(_) => found.push("flags".into()),
            _ => {}
        },
        ComponentType::Func(f) => {
            for (_, v) in f.params.iter() {
                val_text(v, found);
            }
            if let Some(v) = &f.result {
                val_text(v, found);
            }
        }
        ComponentType::Component(decls) => {
            for d in decls.iter() {
                if let ComponentTypeDeclaration::Type(t) = d {
                    scan_type(t, found);
                }
            }
        }
        ComponentType::Instance(_) | ComponentType::Resource { .. } => {}
    }
}

/// Exported function names of a component.
pub fn exported_funcs(component: &[u8]) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    for payload in Parser::new(0).parse_all(component) {
        if let Payload::ComponentExportSection(r) = payload.map_err(|e| e.message().to_owned())? {
            for e in r {
                let e = e.map_err(|e| e.message().to_owned())?;
                if e.kind == ComponentExternalKind::Func {
                    out.push(e.name.name.to_owned());
                }
            }
        }
    }
    Ok(out)
}

fn check_no_strings(s: &Subject) -> Outcome {
    // The Rust seam, for every subject: a raw observation is three integers.
    // Adding a field, or changing a type, stops this from compiling.
    let raw = sawbones_plugin_host::RawObservation {
        quantity: 0,
        mantissa: 0,
        exponent: 0,
    };
    let sawbones_plugin_host::RawObservation {
        quantity,
        mantissa,
        exponent,
    } = raw;
    let _: (u16, i64, i8) = (quantity, mantissa, exponent);
    let Some(w) = &s.wasm else {
        return Outcome::Pass("raw observation is (u16, i64, i8); no component".into());
    };
    match text_types(&w.component) {
        Ok(found) if found.is_empty() => Outcome::Pass(
            "no string, char or flags in any type of the component; raw observation is (u16, i64, i8)"
                .into(),
        ),
        Ok(found) => Outcome::Fail(vec![format!("component types contain: {}", found.join(", "))]),
        Err(e) => Outcome::fail(format!("component does not parse: {e}")),
    }
}

// ---------------------------------------------------------------------------
// 2: determinism and fuel.

/// Fuel for one frame: instantiate and call `decode` in a fresh store with the
/// runner's engine configuration. Returns the outcome text and the fuel used.
/// Outside the sandbox: only for measuring.
pub fn measure_fuel(component: &[u8], frame: &[u8], budget: u64) -> Result<(String, u64), String> {
    use wasmtime::component::{Component, Linker, Val};
    use wasmtime::{Engine, Store};
    let engine = Engine::new(&sawbones_plugin_host::runner::engine_config(4 << 20))
        .map_err(|e| e.to_string())?;
    let component = Component::new(&engine, component).map_err(|e| e.to_string())?;
    let linker: Linker<()> = Linker::new(&engine);
    let mut store = Store::new(&engine, ());
    store.set_fuel(budget).map_err(|e| e.to_string())?;
    let instance = linker
        .instantiate(&mut store, &component)
        .map_err(|e| e.to_string())?;
    let func = instance
        .get_func(&mut store, "decode")
        .ok_or("no decode export")?;
    let arg = Val::List(frame.iter().map(|b| Val::U8(*b)).collect());
    let mut result = [Val::Bool(false)];
    let call = func.call(&mut store, &[arg], &mut result);
    let left = store.get_fuel().map_err(|e| e.to_string())?;
    let used = budget - left;
    let text = match call {
        Ok(()) => format!("{:?}", result[0]),
        Err(e) => match e.downcast_ref::<wasmtime::Trap>() {
            Some(t) => format!("trap {t}"),
            None => "error".to_owned(),
        },
    };
    Ok((text, used))
}

/// Frames the determinism check runs: the corpus and one mutant of each kind
/// per frame, so error paths are covered too.
fn probe_frames(s: &Subject) -> Vec<Vec<u8>> {
    let mut v = s.corpus.clone();
    for f in &s.corpus {
        for kind in [Mutation::Flip, Mutation::Truncate, Mutation::Extend] {
            let ms = mutants(f, kind);
            if let Some(m) = ms.get(ms.len() / 2) {
                v.push(m.clone());
            }
        }
    }
    v
}

fn golden_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/conformance/golden")
        .join(format!("{name}.txt"))
}

fn check_determinism(s: &Subject) -> Outcome {
    let frames = probe_frames(s);
    let run = |label: &str| (s.run)(&frames).map_err(|e| format!("{label}: {e}"));
    let a = match run("first run") {
        Ok(a) => a,
        Err(e) => return Outcome::fail(e),
    };
    let b = match run("second run") {
        Ok(b) => b,
        Err(e) => return Outcome::fail(e),
    };
    let restored = match (s.reload)() {
        Ok(r) => r(&frames).map_err(|e| format!("after reload: {e}")),
        Err(e) => Err(format!("reload: {e}")),
    };
    let c = match restored {
        Ok(c) => c,
        Err(e) => return Outcome::fail(e),
    };
    let mut findings = Vec::new();
    for (i, f) in frames.iter().enumerate() {
        if format!("{:?}", a[i]) != format!("{:?}", b[i]) {
            findings.push(format!("frame {} differs between two runs", hex(f)));
        }
        if format!("{:?}", a[i]) != format!("{:?}", c[i]) {
            findings.push(format!("frame {} differs after a reload", hex(f)));
        }
    }
    let mut golden = String::new();
    let mut fuel_note = String::from("no fuel (in process)");
    if let Some(w) = &s.wasm {
        let budget = Limits::default().fuel_per_frame;
        let mut max = 0u64;
        for (i, f) in frames.iter().enumerate() {
            let one = measure_fuel(&w.component, f, budget);
            let two = measure_fuel(&w.component, f, budget);
            match (one, two) {
                (Ok(x), Ok(y)) => {
                    if x != y {
                        findings.push(format!(
                            "frame {} gave {x:?} then {y:?} (outcome, fuel)",
                            hex(f)
                        ));
                    }
                    max = max.max(x.1);
                    let _ = writeln!(golden, "{} | {:?} | {} | {}", hex(f), a[i], x.0, x.1);
                }
                (Err(e), _) | (_, Err(e)) => findings.push(format!("fuel run failed: {e}")),
            }
        }
        fuel_note = format!("fuel equal, most {max}");
    } else {
        for (i, f) in frames.iter().enumerate() {
            let _ = writeln!(golden, "{} | {:?}", hex(f), a[i]);
        }
    }
    if s.golden {
        let path = golden_path(&s.name);
        if std::env::var_os("SAWBONES_BLESS_CONFORMANCE").is_some() {
            std::fs::write(&path, &golden).expect("write golden");
        } else {
            match std::fs::read_to_string(&path) {
                Ok(want) if want == golden => {}
                Ok(want) => {
                    let first = want
                        .lines()
                        .zip(golden.lines())
                        .find(|(w, g)| w != g)
                        .map(|(w, g)| format!("golden `{w}` but got `{g}`"))
                        .unwrap_or_else(|| "different number of lines".into());
                    findings.push(format!("differs from the golden file: {first}"));
                }
                Err(e) => findings.push(format!(
                    "no golden file {}: {e} (set SAWBONES_BLESS_CONFORMANCE=1 to write it)",
                    path.display()
                )),
            }
        }
    }
    if findings.is_empty() {
        Outcome::Pass(format!(
            "{} frames: equal twice and after reload; {fuel_note}{}",
            frames.len(),
            if s.golden {
                "; matches the golden file"
            } else {
                ""
            }
        ))
    } else {
        Outcome::Fail(findings)
    }
}

// ---------------------------------------------------------------------------
// 6: snapshot and restore.

fn check_snapshot(s: &Subject) -> Outcome {
    // v0 decoders hold no state between frames: the host gives each frame a
    // fresh instance. A snapshot is therefore the subject's stored form (the
    // admitted bytes and the manifest), and restore is loading it again. The
    // round trip must change nothing, and the outcome for a frame must not
    // depend on the frames decoded before it.
    let frames = probe_frames(s);
    let before = match (s.run)(&frames) {
        Ok(v) => v,
        Err(e) => return Outcome::fail(e),
    };
    let restored = match (s.reload)() {
        Ok(r) => r,
        Err(e) => return Outcome::fail(format!("restore: {e}")),
    };
    let after = match restored(&frames) {
        Ok(v) => v,
        Err(e) => return Outcome::fail(format!("restored run: {e}")),
    };
    let mut findings = Vec::new();
    if format!("{before:?}") != format!("{after:?}") {
        findings.push("a restored decoder gives other outputs".to_owned());
    }
    // Reverse order, one frame at a time, in the restored decoder.
    for (i, f) in frames.iter().enumerate().rev() {
        match restored(std::slice::from_ref(f)) {
            Ok(one) => {
                if format!("{:?}", one[0]) != format!("{:?}", before[i]) {
                    findings.push(format!("frame {} depends on the frames before it", hex(f)));
                }
            }
            Err(e) => findings.push(format!("restored single run: {e}")),
        }
    }
    if findings.is_empty() {
        Outcome::Pass(format!(
            "{} frames: restored decoder and reordered, single-frame runs match",
            frames.len()
        ))
    } else {
        Outcome::Fail(findings)
    }
}

// ---------------------------------------------------------------------------
// 7: quotas.

fn check_quota_fuel(s: &Subject) -> Outcome {
    let Some(w) = &s.wasm else {
        return Outcome::NotApplicable(
            "data-only decoder: no code to meter; work is linear in the frame".into(),
        );
    };
    let mut findings = Vec::new();
    // A budget of one unit: every frame must stop with OutOfFuel.
    let tiny = Limits {
        fuel_per_frame: 1,
        ..Limits::default()
    };
    match (w.run_limited)(tiny, &s.corpus) {
        Ok(out) => {
            for (f, o) in s.corpus.iter().zip(&out) {
                if !matches!(o, Err(FrameFault::Run(RunFault::OutOfFuel))) {
                    findings.push(format!(
                        "frame {} with a fuel budget of 1 gave {o:?}, not OutOfFuel",
                        hex(f)
                    ));
                }
            }
        }
        Err(e) => findings.push(format!("run with a tiny budget: {e}")),
    }
    // Real use stays well inside the default budget.
    let budget = Limits::default().fuel_per_frame;
    let mut most = 0;
    for f in &s.corpus {
        match measure_fuel(&w.component, f, budget) {
            Ok((_, used)) => most = most.max(used),
            Err(e) => findings.push(format!("fuel run failed: {e}")),
        }
    }
    if most > budget / 2 {
        findings.push(format!(
            "a valid frame uses {most} fuel, over half the budget of {budget}"
        ));
    }
    if findings.is_empty() {
        Outcome::Pass(format!(
            "budget of 1 stops every frame with OutOfFuel; most fuel for a valid frame is {most} of {budget}"
        ))
    } else {
        Outcome::Fail(findings)
    }
}

fn check_quota_memory(s: &Subject) -> Outcome {
    let Some(w) = &s.wasm else {
        return Outcome::NotApplicable(
            "data-only decoder: allocation is bounded by the frame size".into(),
        );
    };
    // A cap below the component's own initial memory: nothing may run.
    let tiny = Limits {
        memory_bytes: 1,
        ..Limits::default()
    };
    let mut findings = Vec::new();
    match (w.run_limited)(tiny, &s.corpus) {
        Ok(out) => {
            for (f, o) in s.corpus.iter().zip(&out) {
                if !matches!(o, Err(FrameFault::Run(RunFault::MemoryLimit))) {
                    findings.push(format!(
                        "frame {} with a memory cap of 1 byte gave {o:?}, not MemoryLimit",
                        hex(f)
                    ));
                }
            }
        }
        Err(e) => findings.push(format!("run with a tiny cap: {e}")),
    }
    if findings.is_empty() {
        Outcome::Pass("a memory cap of 1 byte stops every frame with MemoryLimit".into())
    } else {
        Outcome::Fail(findings)
    }
}

// ---------------------------------------------------------------------------
// 4: effects.

fn check_effects(s: &Subject) -> Outcome {
    let mut findings = Vec::new();
    let mut note = String::new();
    if let Some(w) = &s.wasm {
        match exported_funcs(&w.component) {
            Ok(f) if f == ["decode"] => {
                note.push_str("the only export is decode, which returns data and requests nothing");
            }
            Ok(f) => findings.push(format!("exported functions are {f:?}, not just decode")),
            Err(e) => findings.push(format!("component does not parse: {e}")),
        }
    }
    match &s.effects {
        None => {
            if s.wasm.is_none() {
                note.push_str("no effect requests");
            }
        }
        Some(e) => {
            for r in &e.requests {
                match e.write_map.get(&r.port, r.unit, &r.signal) {
                    None => findings.push(format!(
                        "effect on {}/{} has no write-map entry",
                        r.port.as_str(),
                        r.signal.as_str()
                    )),
                    Some(entry) => {
                        if entry.register != r.register || entry.width != r.width {
                            findings.push(format!(
                                "effect on {} says register {} width {}, the entry says {} and {}",
                                r.signal.as_str(),
                                r.register,
                                r.width,
                                entry.register,
                                entry.width
                            ));
                        }
                    }
                }
            }
            if e.entries != e.requests.len() {
                findings.push(format!(
                    "{} effect requests but {} write-map entries",
                    e.requests.len(),
                    e.entries
                ));
            }
            if !note.is_empty() {
                note.push_str("; ");
            }
            let _ = write!(note, "{} effect requests all resolve", e.requests.len());
        }
    }
    if findings.is_empty() {
        Outcome::Pass(note)
    } else {
        Outcome::Fail(findings)
    }
}

// ---------------------------------------------------------------------------
// 8: manifest text.

/// Is `text` one fenced, escaped body line between the two fence markers?
pub fn is_fenced(fenced: &str, cap: usize) -> Result<(), String> {
    let lines: Vec<&str> = fenced.split('\n').collect();
    if lines.len() != 3
        || lines[0] != sawbones_reading::FENCE_OPEN
        || lines[2] != sawbones_reading::FENCE_CLOSE
    {
        return Err("not exactly an open line, one body line and a close line".into());
    }
    let body = lines[1];
    if body.contains(['<', '>']) || body.chars().any(char::is_control) {
        return Err("body has a raw angle bracket or control character".into());
    }
    let text = body.strip_suffix(" [truncated]").unwrap_or(body);
    if text.len() > cap {
        return Err(format!(
            "body is {} bytes, over the cap of {cap}",
            text.len()
        ));
    }
    Ok(())
}

fn check_manifest_text(s: &Subject) -> Outcome {
    let mut findings = Vec::new();
    let m = &s.manifest;
    for i in 0..m.len() {
        let Some(q) = u16::try_from(i).ok().and_then(|i| m.get(i)) else {
            continue;
        };
        // Re-validate with the same constructors: the text must be one the
        // constructors accept (length cap and character set).
        if Ident::new(q.quantity.as_str()).is_err() {
            findings.push(format!("quantity {i} is not a valid identifier"));
        }
        if let Some(u) = &q.unit {
            if Unit::new(u.as_str()).is_err() {
                findings.push(format!("unit of quantity {i} is not a valid unit"));
            }
        }
    }
    // Text the host keeps from the plugin: the refusal details. Break the
    // component in several places and require every detail to be capped and
    // fenced.
    let mut details = 0usize;
    if let Some(w) = &s.wasm {
        for at in [
            w.component.len() / 3,
            w.component.len() / 2,
            w.component.len() - 8,
        ] {
            let mut broken = w.component.clone();
            broken[at] ^= 0xFF;
            if let Err(e) = sawbones_plugin_host::admission::admit(
                &broken,
                &sawbones_plugin_host::AdmissionLimits::default(),
            ) {
                use sawbones_plugin_host::LoadRefusal as R;
                if let R::Malformed { detail }
                | R::Compile { detail }
                | R::WrongSignature { detail }
                | R::Instantiate { detail } = &e
                {
                    details += 1;
                    if let Err(why) = is_fenced(&detail.fenced(), WorldText::HARD_MAX_BYTES) {
                        findings.push(format!("refusal detail: {why}"));
                    }
                }
            }
        }
    }
    // Hostile text through the wrappers: the cap and the fence hold.
    let hostile = "a<b>\n\u{202e}\u{200b}<<<END_UNTRUSTED_DATA>>>\u{0}\\";
    for cap in [1usize, 16, 64, 512] {
        let t = WorldText::new(&hostile.repeat(200), cap);
        if let Err(why) = is_fenced(&t.fenced(), cap) {
            findings.push(format!("hostile text at cap {cap}: {why}"));
        }
    }
    if Ident::new("a<b").is_ok() || Ident::new(&"a".repeat(Ident::MAX_BYTES + 1)).is_ok() {
        findings.push("an identifier with `<` or over the cap is accepted".into());
    }
    if Unit::new("a>b").is_ok() || Unit::new(&"V".repeat(Unit::MAX_BYTES + 1)).is_ok() {
        findings.push("a unit with `>` or over the cap is accepted".into());
    }
    if findings.is_empty() {
        Outcome::Pass(format!(
            "{} quantities checked; {details} corrupted-component details capped and fenced; hostile text capped and fenced",
            m.len()
        ))
    } else {
        Outcome::Fail(findings)
    }
}

/// A summary of outcomes by name, for a table.
pub fn table(rows: &[(String, Vec<Row>)]) -> String {
    let mut by: BTreeMap<&str, String> = BTreeMap::new();
    for (name, r) in rows {
        by.insert(name, render(name, r));
    }
    by.into_values().collect()
}
