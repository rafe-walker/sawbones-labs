//! # sawbones-plugin-host: PROVISIONAL
//!
//! **This crate is provisional. The Sawbones design is still under review, and
//! every type here may change or be replaced without notice.**
//!
//! Plugin host v0 (#173): zero-import plugins in their own process.
//!
//! A plugin is a WebAssembly component (the WASI 0.2 component model) that
//! **imports nothing**: no clock, no randomness, no files, no sockets, no
//! handles. The v0 plugin kind is a data-first decoder: a captured frame of
//! bytes goes in, numbers come out (`wit/decoder.wit`). The Pi carries data
//! but does not vouch for it, and plugin output is untrusted data.
//!
//! ## What happens to a plugin
//!
//! 1. [`PluginHost::load`] scans the component on the host side
//!    ([`admission`]) and refuses it if it has **any** import, a nested
//!    component, a start function, a canonical built-in other than `lift`,
//!    an export other than `decode` and types, or a size over the committed
//!    limits. Then it asks a sandboxed child to compile it and check the
//!    `decode` signature, so a component that would not run is refused at
//!    load time, not at the first frame.
//! 2. [`WasmDecoder::decode`] spawns `sawbones-plugin-runner` for each batch
//!    of frames. The child sets up its sandbox ([`sandbox`]), builds the
//!    engine, installs a seccomp allow-list, and only then reads the
//!    component's bytes. It checks again that the compiled component
//!    imports nothing and instantiates it with a linker that defines
//!    nothing. Each frame runs in a fresh store and instance, with fuel, a
//!    memory cap, a table cap and an instance cap. The host kills the child
//!    at its wall-clock deadline.
//! 3. The child's reply is parsed as hostile input ([`wire`]), bounded in
//!    size, with exact lengths. Each value is checked against the host's
//!    [`DecoderManifest`] and becomes an [`Observation`](sawbones_reading::Observation),
//!    which has **no trust tier**. A plugin cannot name a tier, a unit or a
//!    quantity in text: it returns a quantity number and an integer mantissa
//!    and exponent. Any text the host keeps from the child (a compile error)
//!    is [`WorldText`].
//!
//! A crash, a trap, a loop or an allocation storm in the plugin ends in a
//! typed fault for that frame or that batch. The host process carries on.
//!
//! ## Other decoder kinds
//!
//! [`Decoder`] is the seam. [`WasmDecoder`] runs a plugin in a child;
//! [`regmap::RegisterMapDecoder`] runs a data-only register map (#174) in
//! process. Both go through [`decode_frames`], so the checks and the
//! conversion to observations are the same.
//!
//! ## Not here yet
//!
//! Logs and health counters (#175), the conformance suite (#176), state
//! snapshots, other plugin kinds (drivers, analyses, plans), shadow mode,
//! signed plugins, and the rest of the sandbox (#177): see [`sandbox`].

#![forbid(unsafe_code)]

pub mod admission;
pub mod decoder;
pub mod regmap;
#[doc(hidden)]
pub mod runner;
pub mod sandbox;
#[doc(hidden)]
pub mod wire;

use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

use sawbones_reading::WorldText;

pub use admission::AdmissionLimits;
pub use decoder::{
    decode_frames, DecodeContext, DecodeFault, Decoder, DecoderManifest, Frame, FrameFault,
    FrameOutcome, QuantitySpec, RawObservation, RawOutcome, RunFault,
};

use wire::{caps, FrameReply, RefusalCode, Reply, Request};

/// Run limits for every plugin this host runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// Fuel for each frame. Roughly one unit per wasm instruction.
    pub fuel_per_frame: u64,
    /// Linear memory cap per frame, in bytes.
    pub memory_bytes: u64,
    /// Wall-clock deadline for a whole child run, compile included.
    pub deadline: Duration,
    /// Largest frame, in bytes. At most [`wire::caps::MAX_FRAME_BYTES`].
    pub max_frame_bytes: usize,
    /// Most frames per batch. At most [`wire::caps::MAX_FRAMES`].
    pub max_frames: usize,
    /// Most observations per frame. At most [`wire::caps::MAX_OBSERVATIONS`].
    pub max_observations: u32,
    /// Size limits checked before compile.
    pub admission: AdmissionLimits,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            fuel_per_frame: 10_000_000,
            memory_bytes: 16 * 1024 * 1024,
            deadline: Duration::from_secs(10),
            max_frame_bytes: 4096,
            max_frames: 16,
            max_observations: 256,
            admission: AdmissionLimits::default(),
        }
    }
}

/// Why a component was refused at load time.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum LoadRefusal {
    /// Over the size limit.
    #[error("component is {bytes} bytes, over the limit of {max}")]
    TooLarge {
        /// Its size.
        bytes: usize,
        /// The limit.
        max: usize,
    },
    /// A bare core module, or not wasm at all.
    #[error("not a WebAssembly component")]
    NotAComponent,
    /// The bytes do not parse.
    #[error("malformed component")]
    Malformed {
        /// The parser's message, untrusted.
        detail: WorldText,
    },
    /// The component imports something. Plugins import nothing.
    #[error("component has {count} imports; plugins import nothing")]
    HasImports {
        /// How many the host scan counted (0 when the child found them).
        count: u32,
    },
    /// The component nests another component.
    #[error("nested components are refused")]
    NestedComponent,
    /// The component has a start function.
    #[error("component start functions are refused")]
    StartFunction,
    /// The component uses a canonical built-in other than `lift`.
    #[error("canonical built-ins other than lift are refused")]
    CanonicalBuiltin,
    /// The exports are not exactly `decode` plus types.
    #[error("exports must be exactly one function, decode, plus types")]
    BadExport,
    /// A size-driven limit (sup-002).
    #[error("over the admission limit for {what}")]
    OverLimit {
        /// Which limit.
        what: &'static str,
    },
    /// The runner could not compile or validate it.
    #[error("component does not compile")]
    Compile {
        /// The compiler's message, untrusted.
        detail: WorldText,
    },
    /// `decode` is missing or does not match the decoder world.
    #[error("decode does not match the decoder world")]
    WrongSignature {
        /// The type checker's message, untrusted.
        detail: WorldText,
    },
    /// Instantiation failed for another reason.
    #[error("component does not instantiate")]
    Instantiate {
        /// The message, untrusted.
        detail: WorldText,
    },
    /// The trial run failed (see the inner error).
    #[error("trial run failed: {0}")]
    Run(#[from] PluginError),
}

/// Why a batch could not run at all.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum PluginError {
    /// The runner could not be started.
    #[error("could not start the plugin runner")]
    Spawn,
    /// The child missed its wall-clock deadline and was killed.
    #[error("plugin missed its deadline and was killed")]
    Deadline,
    /// The child exited without a reply: a crash, a kill, a sandbox
    /// violation (`SIGSYS`).
    #[error("plugin runner died (exit code {code:?}, signal {signal:?})")]
    Died {
        /// Exit code, if it exited.
        code: Option<i32>,
        /// Signal, if it was killed.
        signal: Option<i32>,
    },
    /// The reply did not parse, was too long or did not match the request.
    #[error("plugin runner sent a malformed reply")]
    BadReply,
    /// The child could not set up its sandbox, so nothing ran.
    #[error("plugin sandbox could not be set up")]
    Sandbox {
        /// The runner's message.
        detail: WorldText,
    },
    /// The component was refused by the runner (it passed the host scan).
    #[error("plugin refused by the runner: {0:?}")]
    Refused(RefusalKind),
    /// Too many frames, or a frame too large.
    #[error("batch over the host's limits")]
    BatchTooLarge,
}

/// The runner's refusal reasons, without their text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RefusalKind {
    /// Imports found by the compiled type or the empty linker.
    HasImports,
    /// Does not compile.
    Compile,
    /// `decode` signature mismatch.
    WrongSignature,
    /// Does not instantiate.
    Instantiate,
}

/// Runs plugins in sandboxed children.
#[derive(Debug, Clone)]
pub struct PluginHost {
    runner: PathBuf,
    limits: Limits,
}

/// An admitted decoder plugin, ready to run.
#[derive(Debug, Clone)]
pub struct WasmDecoder {
    host: PluginHost,
    component: Vec<u8>,
    manifest: DecoderManifest,
}

#[cfg(unix)]
fn signal_of(status: &ExitStatus) -> Option<i32> {
    use std::os::unix::process::ExitStatusExt;
    status.signal()
}

#[cfg(not(unix))]
fn signal_of(_: &ExitStatus) -> Option<i32> {
    None
}

impl PluginHost {
    /// A host that runs plugins with the runner binary at `runner` (the
    /// `sawbones-plugin-runner` built from this crate). Limits above the
    /// hard caps in [`wire::caps`] are lowered to them.
    pub fn new(runner: impl Into<PathBuf>, mut limits: Limits) -> Self {
        limits.max_frame_bytes = limits.max_frame_bytes.min(caps::MAX_FRAME_BYTES);
        limits.max_frames = limits.max_frames.min(caps::MAX_FRAMES);
        limits.max_observations = limits.max_observations.min(caps::MAX_OBSERVATIONS);
        limits.admission.max_component_bytes = limits
            .admission
            .max_component_bytes
            .min(caps::MAX_COMPONENT_BYTES);
        Self {
            runner: runner.into(),
            limits,
        }
    }

    /// The limits in force.
    pub fn limits(&self) -> &Limits {
        &self.limits
    }

    /// Admits a decoder component: the host-side scan, then a trial compile
    /// and signature check in a sandboxed child with no frames.
    pub fn load(
        &self,
        component: &[u8],
        manifest: DecoderManifest,
    ) -> Result<WasmDecoder, LoadRefusal> {
        admission::admit(component, &self.limits.admission)?;
        match self.run_child(component, &[]) {
            Ok(_) => {}
            Err(ChildError::Refused { code, detail }) => {
                let detail = WorldText::from_bytes_lossy(&detail, wire::MAX_DETAIL_BYTES);
                return Err(match code {
                    RefusalCode::HasImports => LoadRefusal::HasImports { count: 0 },
                    RefusalCode::Compile => LoadRefusal::Compile { detail },
                    RefusalCode::WrongSignature => LoadRefusal::WrongSignature { detail },
                    RefusalCode::Instantiate => LoadRefusal::Instantiate { detail },
                });
            }
            Err(ChildError::Plugin(e)) => return Err(LoadRefusal::Run(e)),
        }
        Ok(WasmDecoder {
            host: self.clone(),
            component: component.to_vec(),
            manifest,
        })
    }

    /// Runs the child on `frames` and returns its parsed reply.
    fn run_child(&self, component: &[u8], frames: &[&[u8]]) -> Result<Vec<FrameReply>, ChildError> {
        let request = Request {
            fuel_per_frame: self.limits.fuel_per_frame,
            memory_bytes: self.limits.memory_bytes,
            max_observations: self.limits.max_observations,
            component: component.to_vec(),
            frames: frames.iter().map(|f| f.to_vec()).collect(),
        }
        .encode();
        let max_reply = wire::max_reply_bytes(frames.len(), self.limits.max_observations);
        let bytes = self.spawn_and_wait(request, max_reply)?;
        match Reply::decode(&bytes, frames.len(), self.limits.max_observations) {
            Ok(Reply::Ran(frames)) => Ok(frames),
            Ok(Reply::Refused { code, detail }) => Err(ChildError::Refused { code, detail }),
            Ok(Reply::SandboxFailed { detail }) => Err(ChildError::Plugin(PluginError::Sandbox {
                detail: WorldText::from_bytes_lossy(&detail, wire::MAX_DETAIL_BYTES),
            })),
            Err(_) => Err(ChildError::Plugin(PluginError::BadReply)),
        }
    }

    fn spawn_and_wait(&self, request: Vec<u8>, max_reply: usize) -> Result<Vec<u8>, ChildError> {
        let mut child = Command::new(&self.runner)
            .env_clear()
            .current_dir("/")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|_| PluginError::Spawn)?;
        let (Some(mut stdin), Some(stdout)) = (child.stdin.take(), child.stdout.take()) else {
            let _ = child.kill();
            let _ = child.wait();
            return Err(PluginError::Spawn.into());
        };
        let writer = std::thread::spawn(move || {
            // A child that dies early closes the pipe; that shows up below.
            let _ = stdin.write_all(&request);
        });
        let limit = max_reply as u64 + 1;
        let reader = std::thread::spawn(move || {
            let mut out = Vec::new();
            let _ = stdout.take(limit).read_to_end(&mut out);
            out
        });
        let start = Instant::now();
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break Some(status),
                Ok(None) if start.elapsed() >= self.limits.deadline => break None,
                Ok(None) => std::thread::sleep(Duration::from_millis(2)),
                Err(_) => break None,
            }
        };
        let status = match status {
            Some(status) => status,
            None => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = writer.join();
                let _ = reader.join();
                return Err(PluginError::Deadline.into());
            }
        };
        let _ = writer.join();
        let bytes = reader.join().unwrap_or_default();
        // Over the limit is a bad reply whatever the exit status: the reader
        // stopped, so a flooding child dies of SIGPIPE.
        if bytes.len() > max_reply {
            return Err(PluginError::BadReply.into());
        }
        if !status.success() {
            return Err(PluginError::Died {
                code: status.code(),
                signal: signal_of(&status),
            }
            .into());
        }
        Ok(bytes)
    }
}

enum ChildError {
    Refused { code: RefusalCode, detail: Vec<u8> },
    Plugin(PluginError),
}

impl From<PluginError> for ChildError {
    fn from(e: PluginError) -> Self {
        ChildError::Plugin(e)
    }
}

impl WasmDecoder {
    /// Decodes a batch of frames: one child, a fresh instance per frame.
    pub fn decode(&self, frames: &[Frame]) -> Result<Vec<FrameOutcome>, PluginError> {
        decode_frames(self, frames)
    }

    /// The component's bytes, as admitted.
    pub fn component(&self) -> &[u8] {
        &self.component
    }
}

impl Decoder for WasmDecoder {
    type Error = PluginError;

    fn manifest(&self) -> &DecoderManifest {
        &self.manifest
    }

    fn decode_raw(&self, frames: &[&[u8]]) -> Result<Vec<RawOutcome>, PluginError> {
        let limits = &self.host.limits;
        if frames.len() > limits.max_frames
            || frames.iter().any(|f| f.len() > limits.max_frame_bytes)
        {
            return Err(PluginError::BatchTooLarge);
        }
        let replies = match self.host.run_child(&self.component, frames) {
            Ok(r) => r,
            Err(ChildError::Plugin(e)) => return Err(e),
            Err(ChildError::Refused { code, .. }) => {
                return Err(PluginError::Refused(match code {
                    RefusalCode::HasImports => RefusalKind::HasImports,
                    RefusalCode::Compile => RefusalKind::Compile,
                    RefusalCode::WrongSignature => RefusalKind::WrongSignature,
                    RefusalCode::Instantiate => RefusalKind::Instantiate,
                }))
            }
        };
        Ok(replies
            .into_iter()
            .map(|r| match r {
                FrameReply::Values(v) => Ok(v),
                FrameReply::Declined(f) => Err(FrameFault::Declined(f)),
                FrameReply::Fault(f) => Err(FrameFault::Run(f)),
            })
            .collect())
    }
}
