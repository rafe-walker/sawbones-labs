//! The tools, named by hazard class (#103). v0 registers only the read
//! class. The definitions are constants written here: nothing from a
//! module, a manifest or the session appears in a name, description or
//! schema (#204, cards inj-014 and inj-016), and no input field can carry a
//! class, a tier, an approval or a lease (card inj-018).
//!
//! The tool list is stable: it does not change per connection, per module
//! or as a side effect of a call, because clients cache it and may miss a
//! list-changed notification. Hot-plugged modules appear as data, through
//! `inventory` and `describe`, and `read` takes a module id that must name
//! a module that is available now; anything else gets an error that tells
//! the agent to call `inventory`.
//!
//! The list is still built through a registry ([`registry`]): the core
//! tools, then what each [`ToolSource`] contributes. v0 has no sources
//! ([`SOURCES`] is empty). The seam is for coarse events only, such as a
//! plugin installed under a signed approval, never for a module coming or
//! going, and a source must contribute constants, as the core tools are.

use serde::Deserialize;
use serde_json::{json, Value};

use sawbones_module_protocol::{
    ConfigKey, HazardClass, ManifestHasher, ModuleId, MAX_MANIFEST_PAGES,
};
use sawbones_supervisor::{
    Enrollment, ModuleQuery, ModuleState, Origin, PortRefusal, QuarantineReason, QueryOutcome,
    QueryRefusal, QueryReply, LINK_TIMEOUT_TICKS,
};

use crate::envelope::Envelope;
use crate::sim::SimBench;
use crate::store::{Handle, Record, Sample, Store};

/// The hazard class a tool's name puts it in, so a client's per-tool
/// permission rules land prompts only where the danger is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToolClass {
    /// Reads only: status, inventory, descriptions, readings, the session
    /// store. Class 0. The only class v0 has.
    Read,
    /// `invoke_safe`, class 0 and 1 actions. **Not in v0.**
    InvokeSafe,
    /// `invoke_armed`, class 2 actions behind the arm switch. **Not in
    /// v0.**
    InvokeArmed,
    /// `invoke_hazard`, class 3 actions behind a panel-pressed lease.
    /// **Not in v0.**
    InvokeHazard,
}

impl ToolClass {
    /// The class's name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            ToolClass::Read => "read",
            ToolClass::InvokeSafe => "invoke_safe",
            ToolClass::InvokeArmed => "invoke_armed",
            ToolClass::InvokeHazard => "invoke_hazard",
        }
    }
}

/// The tool names reserved for the invoke classes. None of them exists in
/// v0: listing does not show them and a call to one is an unknown tool.
pub const RESERVED_INVOKE_NAMES: [&str; 3] = ["invoke_safe", "invoke_armed", "invoke_hazard"];

type Run = fn(&mut SimBench, &mut Store, Value) -> Envelope;

/// One tool: a constant definition and the code behind it.
#[derive(Clone, Copy)]
pub struct ToolDef {
    /// The name the client calls.
    pub name: &'static str,
    /// Its hazard class.
    pub class: ToolClass,
    /// A short title.
    pub title: &'static str,
    /// What it does, for the model. Host-written.
    pub description: &'static str,
    schema: fn() -> Value,
    run: Run,
}

impl std::fmt::Debug for ToolDef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolDef")
            .field("name", &self.name)
            .field("class", &self.class)
            .finish_non_exhaustive()
    }
}

/// Every tool v0 registers.
pub const TOOLS: [ToolDef; 5] = [
    ToolDef {
        name: "bench_status",
        class: ToolClass::Read,
        title: "Bench status",
        description: "One status line for the simulated bench: modules by state, rails, fault, the taint latch and stored handles. Written by the host from its own state; it returns no module content. Call it first.",
        schema: no_args_schema,
        run: bench_status,
    },
    ToolDef {
        name: "inventory",
        class: ToolClass::Read,
        title: "Inventory",
        description: "Every enrolled module: id, hub port, the owner's label, state, the enrolled hazard-class ceiling and whether it may transmit. From the owner's enrollment and the supervisor's own state; it returns no module content.",
        schema: no_args_schema,
        run: inventory,
    },
    ToolDef {
        name: "describe",
        class: ToolClass::Read,
        title: "Describe a module",
        description: "One module's enrollment and its manifest pages, checked against the digest pinned at enrollment. The pages are the module's own text: they are returned fenced as untrusted data, stored behind a handle, and reported to the taint latch.",
        schema: describe_schema,
        run: describe,
    },
    ToolDef {
        name: "read",
        class: ToolClass::Read,
        title: "Read a module value",
        description: "Asks a module for a config value or a live reading (live readings are keys 256 and up), 1 to 64 times. Returns a summary of what the module claims, which is device-reported and not measured, and a handle to the samples. Reported to the taint latch.",
        schema: read_schema,
        run: read,
    },
    ToolDef {
        name: "session_query",
        class: ToolClass::Read,
        title: "Query the session store",
        description: "Without a handle: lists this session's handles. With a handle: its summary, and for a read, statistics over samples from..to. Never returns raw samples.",
        schema: session_query_schema,
        run: session_query,
    },
];

// v0 registers only the read class, and this fails the build otherwise.
const _: () = {
    let mut i = 0;
    while i < TOOLS.len() {
        assert!(matches!(TOOLS[i].class, ToolClass::Read));
        i += 1;
    }
};

/// Something that contributes tools on a coarse event (a plugin
/// installed, later). Its tools must be constants, read class in v0, with
/// no text from a module or the session in them. The registry keeps only
/// read-class tools from it, and never one whose name is already taken.
pub trait ToolSource: Sync {
    /// Its tools, for the bench as it is now.
    fn tools(&self, bench: &SimBench) -> Vec<&'static ToolDef>;
}

/// The sources v0 builds its registry from, after the core tools: none.
pub static SOURCES: [&dyn ToolSource; 0] = [];

/// The registry: the core tools, then each source's, read class only, the
/// first of a name kept. In v0 it is exactly [`TOOLS`].
#[must_use]
pub fn registry(bench: &SimBench) -> Vec<&'static ToolDef> {
    let mut out: Vec<&'static ToolDef> = TOOLS.iter().collect();
    for source in &SOURCES {
        for tool in source.tools(bench) {
            if tool.class == ToolClass::Read && !out.iter().any(|t| t.name == tool.name) {
                out.push(tool);
            }
        }
    }
    out
}

/// The tool named `name`, if the registry has one.
#[must_use]
pub fn find(bench: &SimBench, name: &str) -> Option<&'static ToolDef> {
    registry(bench).into_iter().find(|t| t.name == name)
}

/// The `tools/list` result.
#[must_use]
pub fn list(bench: &SimBench) -> Value {
    let tools: Vec<Value> = registry(bench)
        .into_iter()
        .map(|t| {
            json!({
                "name": t.name,
                "title": t.title,
                "description": t.description,
                "inputSchema": (t.schema)(),
                "annotations": {
                    "title": t.title,
                    "readOnlyHint": true,
                    "destructiveHint": false,
                    "idempotentHint": true,
                    "openWorldHint": false,
                },
            })
        })
        .collect();
    json!({ "tools": tools })
}

/// Runs `tool` with `arguments`.
pub(crate) fn run(
    tool: &ToolDef,
    bench: &mut SimBench,
    store: &mut Store,
    arguments: Value,
) -> Envelope {
    (tool.run)(bench, store, arguments)
}

fn no_args_schema() -> Value {
    json!({ "type": "object", "properties": {}, "additionalProperties": false })
}

fn module_property() -> Value {
    json!({ "type": "integer", "minimum": 1, "maximum": 4_294_967_295_u32, "description": "The module id, from inventory." })
}

fn describe_schema() -> Value {
    json!({
        "type": "object",
        "properties": { "module": module_property() },
        "required": ["module"],
        "additionalProperties": false,
    })
}

fn read_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "module": module_property(),
            "key": { "type": "integer", "minimum": 0, "maximum": 65535, "description": "The config key; live readings are 256 and up." },
            "samples": { "type": "integer", "minimum": 1, "maximum": MAX_SAMPLES, "default": 1, "description": "How many times to ask." },
        },
        "required": ["module", "key"],
        "additionalProperties": false,
    })
}

fn session_query_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "handle": { "type": "string", "pattern": "^(rd|mf)-[1-9][0-9]{0,8}$", "description": "A handle from an earlier result." },
            "from": { "type": "integer", "minimum": 0, "description": "First sample, for a read handle." },
            "to": { "type": "integer", "minimum": 0, "description": "One past the last sample, for a read handle." },
        },
        "additionalProperties": false,
    })
}

/// The most samples one `read` asks for.
pub const MAX_SAMPLES: u8 = 64;

/// How many passes a question may take: an outstanding health poll, the
/// send, and the question's own deadline.
const ASK_TICKS: u64 = 2 * LINK_TIMEOUT_TICKS + 4;

/// The most handles `session_query` lists.
const LIST_LIMIT: usize = 32;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct NoArgs {}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DescribeArgs {
    module: u32,
}

const fn one() -> u8 {
    1
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReadArgs {
    module: u32,
    key: u16,
    #[serde(default = "one")]
    samples: u8,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct QueryArgs {
    #[serde(default)]
    handle: Option<String>,
    #[serde(default)]
    from: Option<u32>,
    #[serde(default)]
    to: Option<u32>,
}

/// Parses tool arguments, or a fixed host-written error naming the tool and
/// what it takes. Nothing the client sent is echoed back.
fn args<T: for<'de> Deserialize<'de>>(
    arguments: Value,
    tool: &str,
    expected: &str,
) -> Result<T, Envelope> {
    let arguments = if arguments.is_null() {
        json!({})
    } else {
        arguments
    };
    serde_json::from_value(arguments).map_err(|_| {
        Envelope::error(&format!(
            "invalid arguments for {tool}: it takes {expected}, and no other field"
        ))
    })
}

/// The error for a full session store.
fn store_full() -> Envelope {
    Envelope::error(
        "the session store has used every handle; start a new session (restart the server)",
    )
}

fn class_digit(class: HazardClass) -> u8 {
    sawbones_policy::class_rank(class)
}

fn yes_no(b: bool) -> &'static str {
    if b {
        "yes"
    } else {
        "no"
    }
}

/// A host name for a module's state: enum codes only. A refusal or a
/// quarantine names its kind, not what the device claimed.
fn state_name(state: ModuleState) -> &'static str {
    match state {
        ModuleState::Absent => "absent, nothing on its hub port",
        ModuleState::Discovered => "discovered, identity not proved yet",
        ModuleState::Available => "available",
        ModuleState::Lost => "lost",
        ModuleState::Refused(r) => match r {
            PortRefusal::Unenrolled(_) => "refused: claims an id nobody enrolled",
            PortRefusal::BadIdentity(_) => "refused: identity did not verify",
            PortRefusal::IdentifyRefused(_) => "refused: it would not identify",
            PortRefusal::Quarantined(_) => "refused: its id is quarantined",
            PortRefusal::Flooding => "refused: flooding before it identified",
        },
        ModuleState::Quarantined(q) => match q {
            QuarantineReason::DuplicateId { .. } => "quarantined: one id on two ports",
            QuarantineReason::WrongHubPort { .. } => "quarantined: on the wrong hub port",
            QuarantineReason::CapabilityExceeded { .. } => {
                "quarantined: signed more than it was enrolled for"
            }
            QuarantineReason::ManifestMismatch => "quarantined: manifest is not the pinned one",
            QuarantineReason::RepeatedLoss { .. } => "quarantined: lost too often",
            QuarantineReason::Flooding { .. } => "quarantined: flooding",
        },
    }
}

fn label(bench: &SimBench, module: ModuleId) -> &str {
    bench
        .label(module)
        .map_or("-", sawbones_policy::ScopeName::as_str)
}

fn enrollment(bench: &SimBench, module: ModuleId) -> Option<Enrollment> {
    bench
        .supervisor
        .enrolled()
        .find(|e| e.module == module)
        .cloned()
}

fn bench_status(bench: &mut SimBench, store: &mut Store, arguments: Value) -> Envelope {
    if let Err(e) = args::<NoArgs>(arguments, "bench_status", "no arguments") {
        return e;
    }
    let mut counts = [0usize; 5];
    let modules: Vec<ModuleId> = bench.supervisor.enrolled().map(|e| e.module).collect();
    for module in &modules {
        let i = match bench.supervisor.module_state(*module) {
            Some(ModuleState::Available) => 0,
            Some(ModuleState::Lost | ModuleState::Absent | ModuleState::Discovered) | None => 1,
            Some(ModuleState::Refused(_)) => 2,
            Some(ModuleState::Quarantined(_)) => 3,
        };
        counts[i] += 1;
    }
    let outputs = bench.interlock.outputs();
    let rail_b = outputs
        .rail_b
        .map_or("off".to_owned(), |p| format!("on for port {}", p.0));
    let mut env = Envelope::one_line();
    env.host(format!(
        "bench (simulated) t={}: {} enrolled, {} available, {} lost or not yet seen, {} refused, {} quarantined",
        bench.now(),
        modules.len(),
        counts[0],
        counts[1],
        counts[2],
        counts[3],
    ));
    env.host(format!(
        "rail A {}, rail B {rail_b}, fault {}",
        if outputs.rail_a { "on" } else { "off" },
        if outputs.fault.is_some() {
            "latched"
        } else {
            "none"
        },
    ));
    env.host(format!("{} stored handle(s)", store.len()));
    env
}

fn inventory(bench: &mut SimBench, _store: &mut Store, arguments: Value) -> Envelope {
    if let Err(e) = args::<NoArgs>(arguments, "inventory", "no arguments") {
        return e;
    }
    let mut env = Envelope::default();
    let enrolled: Vec<Enrollment> = bench.supervisor.enrolled().cloned().collect();
    env.host(format!(
        "inventory: {} enrolled module(s), from the owner's enrollment and the supervisor's state",
        enrolled.len()
    ));
    for e in &enrolled {
        let state = bench
            .supervisor
            .module_state(e.module)
            .map_or("unknown", state_name);
        env.host(format!(
            "module {} '{}' on hub port {}: {state}; enrolled ceiling class {}, may transmit: {}, measures: {}",
            e.module.0,
            label(bench, e.module),
            e.hub_port.0,
            class_digit(e.capabilities.max_class),
            yes_no(e.capabilities.can_emit),
            yes_no(e.instrument.is_some()),
        ));
    }
    env
}

/// Asks one question and runs the bench until it has an outcome.
fn ask(
    bench: &mut SimBench,
    module: ModuleId,
    query: ModuleQuery,
) -> Result<QueryOutcome, QueryRefusal> {
    let id = bench.supervisor.query(module, query)?;
    for _ in 0..ASK_TICKS {
        bench.advance(1);
        if let Some(outcome) = bench.supervisor.take_answer(id) {
            return Ok(outcome);
        }
    }
    Ok(QueryOutcome::TimedOut)
}

fn refusal_text(module: ModuleId, refusal: QueryRefusal) -> String {
    match refusal {
        QueryRefusal::NotEnrolled => format!(
            "module {} is not enrolled; call inventory for the modules on the bench now",
            module.0
        ),
        QueryRefusal::Unavailable(state) => format!(
            "module {} is not available: {}; call inventory for the modules present now",
            module.0,
            state_name(state)
        ),
        QueryRefusal::Busy => format!(
            "module {} has too many questions waiting; try again",
            module.0
        ),
    }
}

fn describe(bench: &mut SimBench, store: &mut Store, arguments: Value) -> Envelope {
    let a: DescribeArgs = match args(
        arguments,
        "describe",
        "module (an integer id from inventory)",
    ) {
        Ok(a) => a,
        Err(e) => return e,
    };
    let module = ModuleId(a.module);
    let Some(e) = enrollment(bench, module) else {
        return Envelope::error(&refusal_text(module, QueryRefusal::NotEnrolled));
    };
    let mut env = Envelope::default();
    env.host(format!(
        "describe: module {} '{}' on hub port {}; enrolled ceiling class {}, may transmit: {}; manifest digest pinned at enrollment",
        module.0,
        label(bench, module),
        e.hub_port.0,
        class_digit(e.capabilities.max_class),
        yes_no(e.capabilities.can_emit),
    ));
    let mut pages = Vec::new();
    let mut count = 1u8;
    let mut page = 0u8;
    while page < count {
        match ask(bench, module, ModuleQuery::Describe { page }) {
            Err(refusal) => {
                env.host(refusal_text(module, refusal));
                env.set_error();
                return env;
            }
            Ok(QueryOutcome::Answered(reply)) => match reply.into_claim() {
                QueryReply::Page(p) => {
                    if page == 0 {
                        count = p.page_count().min(MAX_MANIFEST_PAGES);
                    }
                    pages.push(p);
                }
                other => {
                    let what = match other {
                        QueryReply::Refused(code) => format!("refused with {code:?}"),
                        QueryReply::Unexpected(kind) => format!("answered with {kind:?}"),
                        _ => "answered with something else".to_owned(),
                    };
                    env.claim(
                        &mut bench.supervisor,
                        Origin::Device,
                        format!("the module {what} for page {page}"),
                    );
                    env.set_error();
                    return env;
                }
            },
            Ok(QueryOutcome::TimedOut) => {
                env.host(format!(
                    "module {} did not answer for page {page}; call inventory for its state",
                    module.0
                ));
                env.set_error();
                return env;
            }
            Ok(QueryOutcome::Dropped) => {
                env.host(format!(
                    "module {} went away before answering; call inventory for the modules present now",
                    module.0
                ));
                env.set_error();
                return env;
            }
        }
        page += 1;
    }
    let mut hasher = ManifestHasher::new();
    for p in &pages {
        hasher.bytes(p.as_untrusted_bytes());
    }
    let digest_matches = hasher.finish() == e.manifest;
    let bytes: usize = pages.iter().map(|p| p.as_untrusted_bytes().len()).sum();
    let n = pages.len();
    let Ok(handle) = store.put(
        bench.now(),
        Record::Manifest {
            module,
            pages: pages.clone(),
            digest_matches,
        },
    ) else {
        return store_full();
    };
    if !digest_matches {
        env.host(format!(
            "manifest: {n} page(s), {bytes} bytes, which do not hash to the digest pinned at enrollment; text withheld; handle {handle}"
        ));
        return env;
    }
    env.host(format!(
        "manifest: {n} page(s), {bytes} bytes, hash matches the digest pinned at enrollment; handle {handle}"
    ));
    for (i, p) in pages.iter().enumerate() {
        env.fenced(
            &mut bench.supervisor,
            Origin::Device,
            &format!("manifest page {i} (the module's text, untrusted):"),
            &p.world_text(),
        );
    }
    env
}

/// Summary statistics of samples, host-computed from module claims.
fn stats(samples: &[Sample]) -> Option<String> {
    let last = samples.last()?.value;
    let min = samples.iter().map(|s| s.value).min()?;
    let max = samples.iter().map(|s| s.value).max()?;
    let sum: i128 = samples.iter().map(|s| i128::from(s.value)).sum();
    let n = i128::try_from(samples.len()).ok()?;
    let thousandths = sum * 1000 / n;
    let sign = if thousandths < 0 { "-" } else { "" };
    let abs = thousandths.unsigned_abs();
    Some(format!(
        "last {last}, min {min}, max {max}, mean {sign}{}.{:03}",
        abs / 1000,
        abs % 1000
    ))
}

fn read(bench: &mut SimBench, store: &mut Store, arguments: Value) -> Envelope {
    let a: ReadArgs = match args(
        arguments,
        "read",
        "module (an integer id from inventory), key (an integer, 0 to 65535) and samples (optional, 1 to 64)",
    ) {
        Ok(a) => a,
        Err(e) => return e,
    };
    if a.samples == 0 || a.samples > MAX_SAMPLES {
        return Envelope::error(&format!("samples must be 1 to {MAX_SAMPLES}"));
    }
    let module = ModuleId(a.module);
    let mut samples = Vec::new();
    let (mut asked, mut refused, mut unexpected, mut timed_out, mut dropped) =
        (0u32, 0u32, 0u32, 0u32, 0u32);
    let mut codes: Vec<String> = Vec::new();
    for _ in 0..a.samples {
        match ask(
            bench,
            module,
            ModuleQuery::Read {
                key: ConfigKey(a.key),
            },
        ) {
            Err(refusal) => {
                if asked == 0 {
                    return Envelope::error(&refusal_text(module, refusal));
                }
                break;
            }
            Ok(outcome) => {
                asked += 1;
                match outcome {
                    QueryOutcome::Answered(reply) => match reply.into_claim() {
                        QueryReply::Value { value, .. } => samples.push(Sample {
                            tick: bench.now(),
                            value,
                        }),
                        QueryReply::Refused(code) => {
                            refused += 1;
                            let code = format!("{code:?}");
                            if !codes.contains(&code) {
                                codes.push(code);
                            }
                        }
                        QueryReply::Unexpected(_) | QueryReply::Page(_) => unexpected += 1,
                    },
                    QueryOutcome::TimedOut => {
                        timed_out += 1;
                        break;
                    }
                    QueryOutcome::Dropped => {
                        dropped += 1;
                        break;
                    }
                }
            }
        }
    }
    let values = samples.len();
    let summary = stats(&samples);
    let Ok(handle) = store.put(
        bench.now(),
        Record::Read {
            module,
            key: a.key,
            samples,
            asked,
        },
    ) else {
        return store_full();
    };
    let mut env = Envelope::default();
    env.host(format!(
        "read: module {} '{}' key 0x{:04x}: {asked} asked, {values} answered with a value, {refused} refused, {unexpected} unexpected, {timed_out} timed out, {dropped} dropped; handle {handle}",
        module.0,
        label(bench, module),
        a.key,
    ));
    if let Some(summary) = summary {
        env.claim(
            &mut bench.supervisor,
            Origin::Device,
            format!("values the module claims (device-reported, not measured): {summary}"),
        );
    }
    if !codes.is_empty() {
        env.claim(
            &mut bench.supervisor,
            Origin::Device,
            format!("refusal codes the module sent: {}", codes.join(", ")),
        );
    }
    if unexpected > 0 {
        // Any answer is the module's claim, whatever its kind.
        env.claim(
            &mut bench.supervisor,
            Origin::Device,
            format!("the module answered {unexpected} time(s) with something other than a value or a refusal"),
        );
    }
    if values == 0 {
        env.set_error();
        if timed_out + dropped > 0 {
            env.host(format!(
                "module {} stopped answering; call inventory for the modules present now",
                module.0
            ));
        } else {
            env.host(format!(
                "module {} gave no value for key 0x{:04x}; its manifest (describe) lists its readings and settings",
                module.0, a.key
            ));
        }
    }
    env
}

fn session_query(bench: &mut SimBench, store: &mut Store, arguments: Value) -> Envelope {
    let a: QueryArgs = match args(
        arguments,
        "session_query",
        "handle (optional, rd-N or mf-N), from and to (optional integers, with a read handle)",
    ) {
        Ok(a) => a,
        Err(e) => return e,
    };
    let Some(text) = a.handle else {
        if a.from.is_some() || a.to.is_some() {
            return Envelope::error("from and to need a handle");
        }
        let mut env = Envelope::default();
        env.host(format!(
            "session: {} stored handle(s), newest first",
            store.len()
        ));
        for (handle, tick, record) in store.newest_first().take(LIST_LIMIT) {
            env.host(match record {
                Record::Read { module, key, samples, asked } => format!(
                    "{handle} read: module {} key 0x{key:04x}, {} sample(s) of {asked} asked, t={tick}",
                    module.0,
                    samples.len()
                ),
                Record::Manifest { module, pages, digest_matches } => format!(
                    "{handle} manifest: module {}, {} page(s), digest {}, t={tick}",
                    module.0,
                    pages.len(),
                    if *digest_matches { "matches" } else { "does not match" }
                ),
            });
        }
        if store.len() > LIST_LIMIT {
            env.host(format!(
                "{} older handle(s) not listed",
                store.len() - LIST_LIMIT
            ));
        }
        return env;
    };
    let Some(handle) = Handle::parse(&text) else {
        return Envelope::error("not a handle: expected rd-N or mf-N");
    };
    let Some((tick, record)) = store.get(handle) else {
        return Envelope::error("no such handle in this session");
    };
    let mut env = Envelope::default();
    match record {
        Record::Read {
            module,
            key,
            samples,
            ..
        } => {
            let len = u32::try_from(samples.len()).unwrap_or(u32::MAX);
            let from = a.from.unwrap_or(0);
            let to = a.to.unwrap_or(len).min(len);
            if from >= to {
                return Envelope::error(&format!("empty window: {len} sample(s) stored"));
            }
            let window = &samples[from as usize..to as usize];
            env.host(format!(
                "{handle}: module {} key 0x{key:04x}, samples {from}..{to} of {len}, taken by t={tick}",
                module.0
            ));
            if let Some(summary) = stats(window) {
                env.claim(
                    &mut bench.supervisor,
                    Origin::Device,
                    format!("values the module claims (device-reported, not measured): {summary}"),
                );
            }
        }
        Record::Manifest {
            module,
            pages,
            digest_matches,
        } => {
            if a.from.is_some() || a.to.is_some() {
                return Envelope::error("from and to are for a read handle");
            }
            let bytes: usize = pages.iter().map(|p| p.as_untrusted_bytes().len()).sum();
            env.host(format!(
                "{handle}: module {} manifest, {} page(s), {bytes} bytes, digest {}, taken at t={tick}; call describe for the text",
                module.0,
                pages.len(),
                if *digest_matches { "matches" } else { "does not match" }
            ));
        }
    }
    env
}
