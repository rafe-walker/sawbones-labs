//! Load-time admission, on the host side, before any child is spawned.
//!
//! This is a structural scan with `wasmparser`, not a full validation (the
//! runner validates and compiles inside its sandbox). It refuses, by
//! construction rather than by documentation:
//!
//! - anything that is not a component (a bare core module),
//! - **any component import**, at any depth,
//! - nested components, component start functions and every canonical
//!   built-in except `lift` (no `lower`, no resources, no async or thread
//!   built-ins), so a plugin has no way to reach anything outside itself,
//! - exports other than types and exactly one function, `decode`,
//! - size-driven inputs over committed limits (total bytes, core modules,
//!   functions, body size, locals, `br_table` targets), before any compile:
//!   red-team card sup-002.

use sawbones_reading::WorldText;
use wasmparser::{CanonicalFunction, ComponentExternalKind, Encoding, Operator, Parser, Payload};

use crate::LoadRefusal;

/// Size limits checked before compile. The defaults are far above what a
/// decoder needs and far below the sizes behind the 2026 Cranelift
/// advisories (sup-002).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdmissionLimits {
    /// Largest component, in bytes.
    pub max_component_bytes: usize,
    /// Most core modules inside the component.
    pub max_core_modules: u32,
    /// Most functions defined in one core module.
    pub max_functions: u32,
    /// Largest function body, in bytes.
    pub max_body_bytes: usize,
    /// Most locals in one function.
    pub max_locals: u32,
    /// Most targets in one `br_table`.
    pub max_br_table_targets: u32,
}

impl Default for AdmissionLimits {
    fn default() -> Self {
        Self {
            max_component_bytes: crate::wire::caps::MAX_COMPONENT_BYTES,
            max_core_modules: 4,
            max_functions: 10_000,
            max_body_bytes: 256 * 1024,
            max_locals: 10_000,
            max_br_table_targets: 4_096,
        }
    }
}

fn malformed(e: wasmparser::BinaryReaderError) -> LoadRefusal {
    LoadRefusal::Malformed {
        detail: WorldText::new(e.message(), 256),
    }
}

/// Scans `bytes` and refuses it if it breaks any rule above.
pub fn admit(bytes: &[u8], limits: &AdmissionLimits) -> Result<(), LoadRefusal> {
    if bytes.len() > limits.max_component_bytes {
        return Err(LoadRefusal::TooLarge {
            bytes: bytes.len(),
            max: limits.max_component_bytes,
        });
    }
    let mut first = true;
    let mut core_modules: u32 = 0;
    let mut func_exports: u32 = 0;
    for payload in Parser::new(0).parse_all(bytes) {
        let payload = payload.map_err(malformed)?;
        if first {
            first = false;
            match payload {
                Payload::Version {
                    encoding: Encoding::Component,
                    ..
                } => continue,
                _ => return Err(LoadRefusal::NotAComponent),
            }
        }
        match payload {
            Payload::ComponentImportSection(reader) => {
                if reader.count() > 0 {
                    return Err(LoadRefusal::HasImports {
                        count: reader.count(),
                    });
                }
            }
            Payload::ComponentSection { .. } => return Err(LoadRefusal::NestedComponent),
            Payload::ComponentStartSection { .. } => return Err(LoadRefusal::StartFunction),
            Payload::ComponentCanonicalSection(reader) => {
                for f in reader {
                    match f.map_err(malformed)? {
                        CanonicalFunction::Lift { .. } => {}
                        _ => return Err(LoadRefusal::CanonicalBuiltin),
                    }
                }
            }
            Payload::ComponentExportSection(reader) => {
                for export in reader {
                    let export = export.map_err(malformed)?;
                    match export.kind {
                        ComponentExternalKind::Type => {}
                        ComponentExternalKind::Func if export.name.name == "decode" => {
                            func_exports += 1;
                        }
                        _ => return Err(LoadRefusal::BadExport),
                    }
                }
            }
            Payload::ModuleSection { .. } => {
                core_modules += 1;
                if core_modules > limits.max_core_modules {
                    return Err(LoadRefusal::OverLimit {
                        what: "core modules",
                    });
                }
            }
            Payload::FunctionSection(reader) => {
                if reader.count() > limits.max_functions {
                    return Err(LoadRefusal::OverLimit { what: "functions" });
                }
            }
            Payload::CodeSectionEntry(body) => check_body(&body, limits)?,
            _ => {}
        }
    }
    if func_exports != 1 {
        return Err(LoadRefusal::BadExport);
    }
    Ok(())
}

fn check_body(
    body: &wasmparser::FunctionBody<'_>,
    limits: &AdmissionLimits,
) -> Result<(), LoadRefusal> {
    let range = body.range();
    if (range.end - range.start) as usize > limits.max_body_bytes {
        return Err(LoadRefusal::OverLimit {
            what: "function body bytes",
        });
    }
    let mut locals: u32 = 0;
    for local in body.get_locals_reader().map_err(malformed)? {
        let (count, _) = local.map_err(malformed)?;
        locals = locals.saturating_add(count);
        if locals > limits.max_locals {
            return Err(LoadRefusal::OverLimit { what: "locals" });
        }
    }
    let mut ops = body.get_operators_reader().map_err(malformed)?;
    while !ops.eof() {
        if let Operator::BrTable { targets } = ops.read().map_err(malformed)? {
            if targets.len() > limits.max_br_table_targets {
                return Err(LoadRefusal::OverLimit {
                    what: "br_table targets",
                });
            }
        }
    }
    Ok(())
}
