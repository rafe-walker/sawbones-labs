//! The runner child's sandbox, on Linux.
//!
//! What is real in v0, in the order the runner applies it:
//!
//! 1. **Spawned bare** (host side, [`crate::PluginHost`]): empty
//!    environment, working directory `/`, stdin and stdout pipes, stderr to
//!    `/dev/null`, no arguments.
//! 2. **Dies with the host**: `PR_SET_PDEATHSIG` is `SIGKILL`.
//! 3. **Not dumpable**: `PR_SET_DUMPABLE` 0, so a same-user process cannot
//!    `ptrace` it or read its `/proc/<pid>/mem`, and no core file is written.
//! 4. **No inherited descriptors**: `/proc/self/fd` must hold only 0, 1 and
//!    2. Any other descriptor (a key file, a socket) makes the runner refuse
//!    to run at all. Fail closed.
//! 5. **Resource limits**: no core files, no file writes (`RLIMIT_FSIZE` 0),
//!    no new processes or threads (`RLIMIT_NPROC` 0, which root ignores),
//!    at most [`MAX_OPEN_FILES`] descriptors, [`ADDRESS_SPACE_BYTES`] of
//!    address space and [`CPU_SECONDS`] of CPU, a backstop behind fuel and
//!    the host's wall-clock deadline.
//! 6. **seccomp allow-list, after the engine is built and before the
//!    component's bytes are parsed**: `no_new_privs`, then a filter that
//!    checks the architecture first and kills the process on any system
//!    call outside [`ALLOWED_SYSCALLS`] (memory management, reading and
//!    writing the two pipes, futex, exit). A second filter answers the
//!    `open` family with `EPERM`, so a library that tries to open a file
//!    (a coverage runtime at exit) fails cleanly instead of killing the run.
//!    No `open` succeeds, no socket, no `ptrace`, no `execve`, no `clone`,
//!    no `kill`.
//!
//! What is **not** done yet ([#177](https://github.com/rafe-walker/sawbones-labs/issues/177)):
//! a separate unprivileged user (the child runs as the host's own user),
//! namespaces (mount, network, PID, user), Landlock, cgroups. Unprivileged
//! user namespaces are restricted on the CI runners' Ubuntu image and
//! Landlock is absent from the Pi 5 kernel defconfig, so neither is relied
//! on here. On a target other than Linux on x86-64 or aarch64 the runner
//! refuses to run.

/// Address-space cap for the runner, in bytes.
pub const ADDRESS_SPACE_BYTES: u64 = 1024 * 1024 * 1024;
/// CPU-time cap for the runner, in seconds.
pub const CPU_SECONDS: u64 = 30;
/// Descriptor cap for the runner.
pub const MAX_OPEN_FILES: u64 = 8;

/// Why the sandbox could not be set up. Carries only our own text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SandboxError(pub String);

#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
mod linux {
    use std::collections::BTreeMap;

    use rustix::process::{self, DumpableBehavior, Resource, Rlimit, Signal};
    use seccompiler::{
        BpfProgram, SeccompAction, SeccompCmpArgLen, SeccompCmpOp, SeccompCondition, SeccompFilter,
        SeccompRule, TargetArch,
    };

    use super::*;

    #[cfg(target_arch = "x86_64")]
    const ARCH: TargetArch = TargetArch::x86_64;
    #[cfg(target_arch = "aarch64")]
    const ARCH: TargetArch = TargetArch::aarch64;

    /// System calls the runner may make once the filter is on, by name.
    /// Anything else kills the process (`SIGSYS`). Keep this list short and
    /// keep it in step with the test that pins it.
    pub const ALLOWED_SYSCALLS: &[(&str, i64)] = &[
        ("read", libc::SYS_read),
        ("write", libc::SYS_write),
        ("close", libc::SYS_close),
        ("mmap", libc::SYS_mmap),
        ("munmap", libc::SYS_munmap),
        ("mprotect", libc::SYS_mprotect),
        ("madvise", libc::SYS_madvise),
        ("mremap", libc::SYS_mremap),
        ("brk", libc::SYS_brk),
        ("futex", libc::SYS_futex),
        ("sched_yield", libc::SYS_sched_yield),
        ("getrandom", libc::SYS_getrandom),
        ("clock_gettime", libc::SYS_clock_gettime),
        // Wasmtime flushes the instruction cache this way on aarch64.
        ("membarrier", libc::SYS_membarrier),
        ("rt_sigreturn", libc::SYS_rt_sigreturn),
        ("rt_sigprocmask", libc::SYS_rt_sigprocmask),
        ("sigaltstack", libc::SYS_sigaltstack),
        ("exit", libc::SYS_exit),
        ("exit_group", libc::SYS_exit_group),
    ];

    /// System calls answered with `EPERM` instead of a kill.
    #[cfg(target_arch = "x86_64")]
    pub const EPERM_SYSCALLS: &[(&str, i64)] = &[
        ("open", libc::SYS_open),
        ("openat", libc::SYS_openat),
        ("openat2", libc::SYS_openat2),
    ];
    /// System calls answered with `EPERM` instead of a kill.
    #[cfg(target_arch = "aarch64")]
    pub const EPERM_SYSCALLS: &[(&str, i64)] =
        &[("openat", libc::SYS_openat), ("openat2", libc::SYS_openat2)];

    fn err(what: &str, e: impl std::fmt::Display) -> SandboxError {
        SandboxError(format!("{what}: {e}"))
    }

    fn limit(resource: Resource, value: u64, what: &str) -> Result<(), SandboxError> {
        process::setrlimit(
            resource,
            Rlimit {
                current: Some(value),
                maximum: Some(value),
            },
        )
        .map_err(|e| err(what, e))
    }

    /// Steps 2 to 5. Run before the engine is built.
    pub fn prepare() -> Result<(), SandboxError> {
        process::set_parent_process_death_signal(Some(Signal::KILL))
            .map_err(|e| err("parent death signal", e))?;
        // If the host died before the line above, init adopted us: stop.
        if process::getppid().is_none_or(|p| p.is_init()) {
            return Err(SandboxError("host process is gone".into()));
        }
        process::set_dumpable_behavior(DumpableBehavior::NotDumpable)
            .map_err(|e| err("dumpable", e))?;
        check_descriptors()?;
        limit(Resource::Core, 0, "RLIMIT_CORE")?;
        limit(Resource::Fsize, 0, "RLIMIT_FSIZE")?;
        limit(Resource::Nproc, 0, "RLIMIT_NPROC")?;
        limit(Resource::Nofile, MAX_OPEN_FILES, "RLIMIT_NOFILE")?;
        limit(Resource::As, ADDRESS_SPACE_BYTES, "RLIMIT_AS")?;
        limit(Resource::Cpu, CPU_SECONDS, "RLIMIT_CPU")?;
        Ok(())
    }

    /// Refuses to run with any descriptor beyond 0, 1 and 2.
    fn check_descriptors() -> Result<(), SandboxError> {
        let dir = std::fs::read_dir("/proc/self/fd").map_err(|e| err("/proc/self/fd", e))?;
        let mut extra = Vec::new();
        for entry in dir {
            let entry = entry.map_err(|e| err("/proc/self/fd", e))?;
            let name = entry.file_name();
            let n: u32 = name
                .to_str()
                .and_then(|s| s.parse().ok())
                .ok_or_else(|| SandboxError("odd entry in /proc/self/fd".into()))?;
            if n <= 2 {
                continue;
            }
            // The directory handle doing this listing shows up too.
            let target = std::fs::read_link(entry.path()).unwrap_or_default();
            let is_listing = target
                .to_str()
                .is_some_and(|t| t.starts_with("/proc/") && t.ends_with("/fd"));
            if !is_listing {
                extra.push(n);
            }
        }
        if extra.is_empty() {
            Ok(())
        } else {
            Err(SandboxError(format!(
                "inherited file descriptors {extra:?}"
            )))
        }
    }

    /// The allow-list filter: kill on anything not listed.
    pub fn allow_list_program() -> Result<BpfProgram, SandboxError> {
        let rules: BTreeMap<i64, Vec<SeccompRule>> = ALLOWED_SYSCALLS
            .iter()
            .chain(EPERM_SYSCALLS)
            .map(|(_, nr)| (*nr, Vec::new()))
            .collect();
        let filter = SeccompFilter::new(
            rules,
            SeccompAction::KillProcess,
            SeccompAction::Allow,
            ARCH,
        )
        .map_err(|e| err("seccomp filter", e))?;
        filter.try_into().map_err(|e| err("seccomp compile", e))
    }

    /// The second filter: `EPERM` for the open family, allow the rest (the
    /// first filter still decides the rest).
    pub fn eperm_program() -> Result<BpfProgram, SandboxError> {
        // A condition that always holds (argument 0 >= 0), because a rule
        // needs at least one condition.
        let always = || {
            SeccompRule::new(vec![SeccompCondition::new(
                0,
                SeccompCmpArgLen::Qword,
                SeccompCmpOp::Ge,
                0,
            )
            .map_err(|e| err("seccomp condition", e))?])
            .map_err(|e| err("seccomp rule", e))
        };
        let mut rules = BTreeMap::new();
        for (_, nr) in EPERM_SYSCALLS {
            rules.insert(*nr, vec![always()?]);
        }
        let filter = SeccompFilter::new(
            rules,
            SeccompAction::Allow,
            SeccompAction::Errno(libc::EPERM as u32),
            ARCH,
        )
        .map_err(|e| err("seccomp filter", e))?;
        filter.try_into().map_err(|e| err("seccomp compile", e))
    }

    /// Step 6. Run after the engine is built, before the component is read.
    pub fn lock_down() -> Result<(), SandboxError> {
        let allow = allow_list_program()?;
        let eperm = eperm_program()?;
        // apply_filter sets no_new_privs first. The allow-list does not list
        // seccomp(2), so it goes on last: once it is on, no filter can be
        // added or loosened. The kernel applies the strictest answer of the
        // two filters to every call.
        seccompiler::apply_filter_all_threads(&eperm).map_err(|e| err("seccomp install", e))?;
        seccompiler::apply_filter_all_threads(&allow).map_err(|e| err("seccomp install", e))?;
        Ok(())
    }
}

#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
pub use linux::{
    allow_list_program, eperm_program, lock_down, prepare, ALLOWED_SYSCALLS, EPERM_SYSCALLS,
};

/// On other targets there is no sandbox, so the runner refuses to run.
#[cfg(not(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
)))]
pub fn prepare() -> Result<(), SandboxError> {
    Err(SandboxError(
        "no sandbox for this target: refusing to run plugins".into(),
    ))
}

/// On other targets there is no sandbox, so the runner refuses to run.
#[cfg(not(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
)))]
pub fn lock_down() -> Result<(), SandboxError> {
    Err(SandboxError(
        "no sandbox for this target: refusing to run plugins".into(),
    ))
}
