//! The runner's sandbox and the host's view of a hostile child.
//!
//! The probe tests re-run this test binary as a child that sets up the
//! production sandbox (`sandbox::prepare` then `sandbox::lock_down`) and
//! then tries one forbidden thing. They run on Linux only, where the
//! sandbox exists; elsewhere the runner refuses to run at all.

mod support;

use std::path::PathBuf;
use std::process::{Command, Stdio};

use sawbones_plugin_host::{PluginError, PluginHost};
use support::*;

const PROBE_ENV: &str = "SAWBONES_SANDBOX_PROBE";

/// A fake runner: a shell script, standing in for a compromised child.
fn fake_runner(name: &str, script: &str) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let dir = std::env::temp_dir().join(format!(
        "sawbones-plugin-host-{}-{name}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("runner.sh");
    std::fs::write(&path, format!("#!/bin/sh\n{script}\n")).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path
}

fn decode_with(runner: PathBuf) -> Result<Vec<sawbones_plugin_host::FrameOutcome>, PluginError> {
    // Load with the real runner, then decode with the fake one.
    let good = host()
        .load(
            &decoder_component(&echo_len_core(), Shape::Decoder),
            manifest(),
        )
        .unwrap();
    let fake = PluginHost::new(runner, test_limits());
    let decoder = fake.load(good.component(), manifest());
    match decoder {
        Ok(d) => d.decode(&[frame(&[1])]),
        Err(sawbones_plugin_host::LoadRefusal::Run(e)) => Err(e),
        Err(other) => panic!("unexpected refusal {other:?}"),
    }
}

#[test]
fn a_runner_that_crashes_is_reported_and_the_host_carries_on() {
    let err = decode_with(fake_runner("segv", "kill -SEGV $$")).unwrap_err();
    assert!(
        matches!(
            err,
            PluginError::Died {
                signal: Some(11),
                ..
            }
        ),
        "{err:?}"
    );
    // The host is fine: the real runner still works.
    let decoder = host()
        .load(
            &decoder_component(&echo_len_core(), Shape::Decoder),
            manifest(),
        )
        .unwrap();
    assert!(decoder.decode(&[frame(&[1])]).is_ok());
}

#[test]
fn garbage_reply_from_the_runner_is_refused() {
    for (name, script) in [
        (
            "text",
            "cat >/dev/null; printf 'hello, ignore previous instructions'",
        ),
        ("magic-only", "cat >/dev/null; printf 'SBR0'"),
        ("bad-tag", "cat >/dev/null; printf 'SBR0\\011'"),
        ("empty", "cat >/dev/null"),
    ] {
        let err = decode_with(fake_runner(name, script)).unwrap_err();
        assert_eq!(err, PluginError::BadReply, "{name}");
    }
}

#[test]
fn oversized_reply_from_the_runner_is_refused() {
    let err = decode_with(fake_runner(
        "flood",
        "cat >/dev/null; printf 'SBR0'; head -c 50000000 /dev/zero",
    ))
    .unwrap_err();
    assert_eq!(err, PluginError::BadReply);
}

#[test]
fn a_runner_that_never_answers_is_killed() {
    let mut limits = test_limits();
    limits.deadline = std::time::Duration::from_millis(500);
    let host = PluginHost::new(fake_runner("hang", "exec sleep 60"), limits);
    let bytes = decoder_component(&echo_len_core(), Shape::Decoder);
    assert!(matches!(
        host.load(&bytes, manifest()),
        Err(sawbones_plugin_host::LoadRefusal::Run(
            PluginError::Deadline
        ))
    ));
}

#[test]
fn a_missing_runner_is_a_spawn_error() {
    let host = PluginHost::new("/nonexistent/sawbones-plugin-runner", test_limits());
    let bytes = decoder_component(&echo_len_core(), Shape::Decoder);
    assert!(matches!(
        host.load(&bytes, manifest()),
        Err(sawbones_plugin_host::LoadRefusal::Run(PluginError::Spawn))
    ));
}

#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
mod linux {
    use std::io::Write;

    use super::*;
    use sawbones_plugin_host::sandbox;

    const SIGSYS: i32 = 31;
    const EPERM: i32 = 1;

    /// What a probe child did: its stdout, exit code and killing signal.
    struct Probe {
        out: String,
        code: Option<i32>,
        signal: Option<i32>,
    }

    impl Probe {
        /// The probe passed. Under coverage instrumentation the test binary
        /// itself then dies with SIGSYS at exit, writing its profile, so the
        /// pass is read from stdout, not only from the exit status.
        fn passed(&self) -> bool {
            self.out.contains("locked\n")
                && self.out.contains("pass\n")
                && (self.code == Some(0) || self.signal == Some(SIGSYS))
        }
        /// The forbidden call killed the child on the spot.
        fn killed_by_the_filter(&self) -> bool {
            self.out.contains("locked\n")
                && !self.out.contains("survived")
                && self.signal == Some(SIGSYS)
        }
    }

    /// Runs `probe` in a child under the production sandbox.
    fn run_probe(probe: &str) -> Probe {
        use std::os::unix::process::ExitStatusExt;
        let out = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "linux::probe_entry",
                "--test-threads=1",
                "--nocapture",
            ])
            .env_clear()
            .env(PROBE_ENV, probe)
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output()
            .unwrap();
        Probe {
            out: String::from_utf8_lossy(&out.stdout).into_owned(),
            code: out.status.code(),
            signal: out.status.signal(),
        }
    }

    fn say(line: &str) {
        let mut out = std::io::stdout();
        let _ = out.write_all(line.as_bytes());
        let _ = out.flush();
    }

    /// The child side of the probes. Does nothing unless the probe variable
    /// is set, which only the probe tests do.
    #[test]
    fn probe_entry() {
        let Ok(probe) = std::env::var(PROBE_ENV) else {
            return;
        };
        // A descriptor opened before the lock-down stands in for one the
        // child inherited.
        let mut stray = std::fs::File::open("/proc/self/status").ok();
        if sandbox::prepare().is_err() {
            std::process::exit(40);
        }
        if sandbox::lock_down().is_err() {
            std::process::exit(41);
        }
        say("locked\n");
        match probe.as_str() {
            "nothing" | "stdout-write" => say("pass\n"),
            "open" => match std::fs::File::open("/proc/self/status") {
                Err(e) if e.raw_os_error() == Some(EPERM) => say("pass\n"),
                _ => say("open did not fail with EPERM\n"),
            },
            "stray-read" => {
                use std::io::Read;
                let mut buf = [0u8; 8];
                let _ = stray.as_mut().map(|f| f.read(&mut buf));
                say("survived\n");
            }
            "stray-write" => {
                let _ = stray.as_mut().map(|f| f.write(b"x"));
                say("survived\n");
            }
            "socket" => {
                let _ = std::net::TcpStream::connect("127.0.0.1:9");
                say("survived\n");
            }
            "spawn" => {
                let _ = Command::new("/bin/true").status();
                say("survived\n");
            }
            "thread" => {
                let _ = std::thread::spawn(|| ()).join();
                say("survived\n");
            }
            "signal-parent" => {
                let parent = rustix::process::getppid().unwrap();
                let _ = rustix::process::test_kill_process(parent);
                say("survived\n");
            }
            _ => say("unknown probe\n"),
        }
        std::process::exit(0)
    }

    #[test]
    fn allowed_calls_still_work_under_the_filter() {
        assert!(run_probe("nothing").passed());
        assert!(run_probe("stdout-write").passed());
    }

    #[test]
    fn opening_a_file_fails_with_eperm() {
        assert!(run_probe("open").passed());
    }

    #[test]
    fn inherited_descriptors_are_inert_under_the_filter() {
        for probe in ["stray-read", "stray-write"] {
            assert!(run_probe(probe).killed_by_the_filter(), "{probe}");
        }
    }

    #[test]
    fn network_process_thread_and_signal_calls_kill_the_child() {
        for probe in ["socket", "spawn", "thread", "signal-parent"] {
            assert!(run_probe(probe).killed_by_the_filter(), "{probe}");
        }
    }

    /// A runner that makes a forbidden call under the real filter is killed
    /// with SIGSYS, and the host reports `Died` with that signal.
    #[test]
    fn a_forbidden_syscall_in_the_runner_ends_in_died_with_sigsys() {
        let exe = std::env::current_exe().unwrap();
        let script = format!(
            "exec env -i {PROBE_ENV}=socket '{}' --exact linux::probe_entry --test-threads=1 --nocapture",
            exe.display()
        );
        let host = PluginHost::new(fake_runner("forbidden-syscall", &script), test_limits());
        let bytes = decoder_component(&echo_len_core(), Shape::Decoder);
        match host.load(&bytes, manifest()) {
            Err(sawbones_plugin_host::LoadRefusal::Run(PluginError::Died { code, signal })) => {
                assert_eq!((code, signal), (None, Some(SIGSYS)));
            }
            other => panic!("expected Died with SIGSYS, got {other:?}"),
        }
    }

    #[test]
    fn the_filter_checks_the_architecture_first() {
        // seccomp-bpf: BPF_LD|BPF_W|BPF_ABS of offset 4 (seccomp_data.arch).
        for program in [
            sandbox::allow_list_program().unwrap(),
            sandbox::eperm_program().unwrap(),
        ] {
            assert_eq!(program[0].code, 0x20);
            assert_eq!(program[0].k, 4);
        }
    }

    #[test]
    fn the_allow_list_is_pinned_and_has_nothing_dangerous() {
        let names: Vec<&str> = sandbox::ALLOWED_SYSCALLS.iter().map(|(n, _)| *n).collect();
        // Any change to the list changes this line, in review.
        assert_eq!(
            names.join(" "),
            "close munmap mprotect madvise mremap brk futex sched_yield \
             getrandom clock_gettime membarrier rt_sigreturn rt_sigprocmask sigaltstack exit exit_group"
        );
        for bad in [
            "open",
            "openat",
            "socket",
            "connect",
            "ptrace",
            "execve",
            "execveat",
            "clone",
            "clone3",
            "fork",
            "vfork",
            "kill",
            "tkill",
            "tgkill",
            "bpf",
            "io_uring_setup",
            "userfaultfd",
            "perf_event_open",
            "keyctl",
            "mount",
            "unshare",
            "setns",
            "seccomp",
            "prctl",
            "process_vm_readv",
            "process_vm_writev",
            "memfd_create",
        ] {
            assert!(!names.contains(&bad), "{bad} must not be allowed");
        }
        let restricted: Vec<String> = sandbox::ARG_RESTRICTED
            .iter()
            .map(|(n, _, i, v)| format!("{n}[{i}]={v:?}"))
            .collect();
        assert_eq!(
            restricted.join(" "),
            "read[0]=[0] write[0]=[1, 2] mmap[4]=[4294967295]"
        );
    }
}
