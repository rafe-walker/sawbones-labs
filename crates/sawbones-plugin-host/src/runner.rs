//! The child side: `sawbones-plugin-runner`.
//!
//! Reads one [`Request`] from stdin, sets up the sandbox, compiles the
//! component from its `.wasm` bytes (never a precompiled artifact), checks
//! it imports nothing, and runs `decode` once per frame, each in a fresh
//! store and instance, with fuel and a memory cap. Writes one [`Reply`] to
//! stdout and exits 0. Exits 2 if it cannot even read the request.
//!
//! The host treats everything this process says as untrusted.

use std::io::{Read, Write};

use wasmtime::component::{Component, ComponentType, Lift, Linker, WasmList};
use wasmtime::{Config, Engine, ResourceLimiter, Store, Strategy, Trap};

use crate::sandbox;
use crate::wire::{
    caps, DecodeFault, FrameReply, RawObservation, RefusalCode, Reply, Request, RunFault,
    MAX_DETAIL_BYTES,
};

/// Wasm stack per call, in bytes.
pub const MAX_WASM_STACK: usize = 256 * 1024;
/// Most core instances, tables and memories one component may create.
pub const MAX_INSTANCES: usize = 8;
/// See [`MAX_INSTANCES`].
pub const MAX_TABLES: usize = 4;
/// See [`MAX_INSTANCES`].
pub const MAX_MEMORIES: usize = 1;
/// Most table elements.
pub const MAX_TABLE_ELEMENTS: usize = 10_000;

#[derive(ComponentType, Lift)]
#[component(record)]
struct Decimal {
    mantissa: i64,
    exponent: i8,
}

#[derive(ComponentType, Lift)]
#[component(record)]
struct Observation {
    quantity: u16,
    value: Decimal,
}

// The variants are built by lifting, never by name.
#[allow(dead_code)]
#[derive(ComponentType, Lift, Clone, Copy)]
#[component(enum)]
#[repr(u8)]
enum Fault {
    #[component(name = "malformed")]
    Malformed,
    #[component(name = "unknown-frame")]
    UnknownFrame,
    #[component(name = "unsupported")]
    Unsupported,
}

type DecodeResult = (Result<WasmList<Observation>, Fault>,);

/// Caps memory and tables, and remembers whether a cap was hit so the
/// resulting trap can be reported as a memory limit.
struct Limiter {
    memory_bytes: usize,
    hit: bool,
}

impl ResourceLimiter for Limiter {
    fn memory_growing(
        &mut self,
        _current: usize,
        desired: usize,
        _maximum: Option<usize>,
    ) -> wasmtime::Result<bool> {
        if desired > self.memory_bytes {
            self.hit = true;
            wasmtime::bail!("memory cap");
        }
        Ok(true)
    }
    fn table_growing(
        &mut self,
        _current: usize,
        desired: usize,
        _maximum: Option<usize>,
    ) -> wasmtime::Result<bool> {
        if desired > MAX_TABLE_ELEMENTS {
            self.hit = true;
            wasmtime::bail!("table cap");
        }
        Ok(true)
    }
    fn instances(&self) -> usize {
        MAX_INSTANCES
    }
    fn tables(&self) -> usize {
        MAX_TABLES
    }
    fn memories(&self) -> usize {
        MAX_MEMORIES
    }
}

/// The engine configuration: Cranelift only (never Winch), fuel, NaN
/// canonicalisation, explicit bounds checks instead of signal handlers,
/// a small fixed memory reservation, no pooling, no threads, no SIMD.
pub fn engine_config(memory_bytes: u64) -> Config {
    let mut c = Config::new();
    c.strategy(Strategy::Cranelift);
    c.consume_fuel(true);
    c.cranelift_nan_canonicalization(true);
    c.signals_based_traps(false);
    c.memory_reservation(memory_bytes);
    c.memory_guard_size(0);
    c.memory_reservation_for_growth(0);
    c.guard_before_linear_memory(false);
    c.memory_init_cow(false);
    c.max_wasm_stack(MAX_WASM_STACK);
    // Threads are not compiled in (no `threads` feature).
    c.wasm_simd(false);
    c.wasm_relaxed_simd(false);
    c.wasm_memory64(false);
    c.wasm_multi_memory(false);
    c.wasm_component_model(true);
    c.wasm_backtrace_max_frames(None);
    c.debug_info(false);
    c
}

fn detail(e: &impl std::fmt::Display) -> Vec<u8> {
    let mut text = e.to_string().into_bytes();
    text.truncate(MAX_DETAIL_BYTES);
    text
}

fn refused(code: RefusalCode, e: &impl std::fmt::Display) -> Reply {
    Reply::Refused {
        code,
        detail: detail(e),
    }
}

/// Builds the engine, then locks the process down, then handles the
/// component. Split out so the order is in one place.
fn serve(request: &Request) -> Reply {
    if let Err(e) = sandbox::prepare() {
        return Reply::SandboxFailed {
            detail: e.0.into_bytes(),
        };
    }
    let engine = match Engine::new(&engine_config(request.memory_bytes)) {
        Ok(engine) => engine,
        Err(e) => return Reply::SandboxFailed { detail: detail(&e) },
    };
    if let Err(e) = sandbox::lock_down() {
        return Reply::SandboxFailed {
            detail: e.0.into_bytes(),
        };
    }
    run(&engine, request)
}

/// Compiles and runs. Everything from here on happens under the filter.
pub fn run(engine: &Engine, request: &Request) -> Reply {
    let component = match Component::new(engine, &request.component) {
        Ok(c) => c,
        Err(e) => return refused(RefusalCode::Compile, &e),
    };
    // Second layer behind the host's admission scan: the compiled type.
    let imports = component.component_type().imports(engine).len();
    if imports != 0 {
        return refused(RefusalCode::HasImports, &format!("{imports} imports"));
    }
    // Third layer: a linker that defines nothing.
    let linker: Linker<Limiter> = Linker::new(engine);
    let pre = match linker.instantiate_pre(&component) {
        Ok(pre) => pre,
        Err(e) => return refused(RefusalCode::HasImports, &e),
    };
    // Check the signature once, with an empty store, before any frame.
    {
        let mut store = new_store(engine, request);
        match pre.instantiate(&mut store) {
            Ok(instance) => {
                if let Err(e) =
                    instance.get_typed_func::<(&[u8],), DecodeResult>(&mut store, "decode")
                {
                    return refused(RefusalCode::WrongSignature, &e);
                }
            }
            Err(e) if store.data().hit => {
                let _ = e;
                // Too big to even instantiate: every frame would fail the same way.
                return Reply::Ran(vec![
                    FrameReply::Fault(RunFault::MemoryLimit);
                    request.frames.len()
                ]);
            }
            Err(e) => {
                if let Some(fault) = trap_fault(&e, false) {
                    return Reply::Ran(vec![FrameReply::Fault(fault); request.frames.len()]);
                }
                return refused(RefusalCode::Instantiate, &e);
            }
        }
    }
    let frames = request
        .frames
        .iter()
        .map(|frame| run_frame(engine, &pre, request, frame))
        .collect();
    Reply::Ran(frames)
}

fn new_store(engine: &Engine, request: &Request) -> Store<Limiter> {
    let memory_bytes = usize::try_from(request.memory_bytes).unwrap_or(usize::MAX);
    let mut store = Store::new(
        engine,
        Limiter {
            memory_bytes,
            hit: false,
        },
    );
    store.limiter(|l| l);
    // Fuel cannot fail to set when consume_fuel is on.
    let _ = store.set_fuel(request.fuel_per_frame);
    store
}

/// Maps a wasmtime error to a fault, if it is a trap.
fn trap_fault(e: &wasmtime::Error, limit_hit: bool) -> Option<RunFault> {
    if limit_hit {
        return Some(RunFault::MemoryLimit);
    }
    match e.downcast_ref::<Trap>() {
        Some(Trap::OutOfFuel) => Some(RunFault::OutOfFuel),
        Some(Trap::StackOverflow) => Some(RunFault::StackOverflow),
        Some(_) => Some(RunFault::Trap),
        None => None,
    }
}

fn run_frame(
    engine: &Engine,
    pre: &wasmtime::component::InstancePre<Limiter>,
    request: &Request,
    frame: &[u8],
) -> FrameReply {
    let mut store = new_store(engine, request);
    let instance = match pre.instantiate(&mut store) {
        Ok(i) => i,
        Err(e) => {
            let hit = store.data().hit;
            return FrameReply::Fault(trap_fault(&e, hit).unwrap_or(RunFault::Trap));
        }
    };
    let func = match instance.get_typed_func::<(&[u8],), DecodeResult>(&mut store, "decode") {
        Ok(f) => f,
        Err(_) => return FrameReply::Fault(RunFault::BadOutput),
    };
    let result = match func.call(&mut store, (frame,)) {
        Ok((result,)) => result,
        Err(e) => {
            let hit = store.data().hit;
            // An error that is not a trap is a lift failure: bad output.
            return FrameReply::Fault(trap_fault(&e, hit).unwrap_or(RunFault::BadOutput));
        }
    };
    match result {
        Err(fault) => FrameReply::Declined(match fault {
            Fault::Malformed => DecodeFault::Malformed,
            Fault::UnknownFrame => DecodeFault::UnknownFrame,
            Fault::Unsupported => DecodeFault::Unsupported,
        }),
        Ok(list) => {
            // Count first, lift after: an oversized list is never lifted.
            if list.len() > request.max_observations as usize {
                return FrameReply::Fault(RunFault::TooManyObservations);
            }
            let mut values = Vec::with_capacity(list.len());
            let items = match list.iter(&mut store) {
                Ok(items) => items,
                Err(_) => return FrameReply::Fault(RunFault::BadOutput),
            };
            for item in items {
                match item {
                    Ok(o) => values.push(RawObservation {
                        quantity: o.quantity,
                        mantissa: o.value.mantissa,
                        exponent: o.value.exponent,
                    }),
                    // Lifting an element only reads memory: any error is bad output.
                    Err(_) => return FrameReply::Fault(RunFault::BadOutput),
                }
            }
            FrameReply::Values(values)
        }
    }
}

/// The runner's `main`. Returns the process exit code.
pub fn main() -> i32 {
    let mut input = Vec::new();
    let limit = (caps::MAX_REQUEST_BYTES as u64) + 1;
    if std::io::stdin()
        .lock()
        .take(limit)
        .read_to_end(&mut input)
        .is_err()
        || input.len() > caps::MAX_REQUEST_BYTES
    {
        return 2;
    }
    let request = match Request::decode(&input) {
        Ok(r) => r,
        Err(_) => return 2,
    };
    drop(input);
    let reply = serve(&request).encode();
    let mut out = std::io::stdout().lock();
    if out.write_all(&reply).and_then(|()| out.flush()).is_err() {
        return 3;
    }
    0
}
