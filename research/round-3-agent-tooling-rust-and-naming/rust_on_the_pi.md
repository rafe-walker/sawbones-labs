# Should all the software on the Pi 5 sensor/bench appliance be Rust? (state as of 2026-11-25)

Method notes for the report writer
- Crate versions, dates, licences and download counts below come from the crates.io API, queried 2026-11-25. Each is linked to its crates.io page. "Recent" downloads means crates.io's rolling 90-day figure.
- "Measured" = a number published by the source. "Self-reported" = one author's own figure with no method given. Anything I could not confirm in a source this session is in a Gaps list, not in Cited Findings.
- The project's IsoKron MCP server (wake contract `get_system_state`) returned HTTP 502 for the whole session, so no substrate briefing was read. This research does not write to the substrate.

## 1. What "others have started this" most plausibly refers to

### Takeaway
The owner is almost certainly thinking of the 2025-2026 run of mainstream "replace C with Rust" news: Ubuntu shipping Rust coreutils and sudo-rs by default, the Linux kernel dropping the "experimental" label on Rust, Git making Rust mandatory for 3.0, and Android's vulnerability numbers. In the bench-instrument niche the closest match is `oxrok`, a Claude-translated Rust port of the whole sigrok stack that also serves MCP. Every one of these is an incremental, test-gated replacement of specific components; none is "everything in Rust at once", and the Ubuntu rollout shows what breaks when behavioural parity is assumed instead of tested.

### Cited Findings

Ubuntu / uutils / sudo-rs / ntpd-rs
- Ubuntu 25.10 shipped rust-coreutils as default "to maximise real-world testing ahead of the LTS". Ubuntu 26.04 LTS ships uutils 0.8.0 but keeps GNU `cp`, `mv` and `rm` because of 8 open TOCTOU issues (as of 2026-04-22). Target for 100% Rust coreutils is 26.10 — [Ubuntu Discourse, "An update on rust-coreutils", 2026-04-22](https://discourse.ubuntu.com/t/an-update-on-rust-coreutils/80773)
- A paid external audit (Zellic, two rounds, Dec 2025 to Mar 2026) found 113 issues in uutils; 44 CVEs were issued (CVE-2026-35338 to CVE-2026-35381); Zellic sent 30 upstream PRs — [Ubuntu Discourse](https://discourse.ubuntu.com/t/an-update-on-rust-coreutils/80773)
- Thread reports after release: uutils `install` replacing a symlink in `/sbin` (2026-05-06), and a user citing Launchpad bug 2158615 that 0.8.0 "can corrupt files" (2026-08-18; user report, not confirmed by me) — [Ubuntu Discourse](https://discourse.ubuntu.com/t/an-update-on-rust-coreutils/80773)
- Ubuntu 26.10 moved the last three holdouts (`cp`, `mv`, `rm`) to uutils; in July 2026 uutils `cp` was added to the archive, broke live image builds, and was pulled and fixed. GNU tools remain installable as `coreutils-from-gnu`. ntpd-rs is targeted as default for Ubuntu 27.04 — [It's FOSS, 2026-09-15](https://itsfoss.com/news/ubuntu-rustification-coreutils-migration/)
- What broke in 25.10: uutils `date -r` returned the current time instead of the file's mtime, which silently stopped Ubuntu's automatic update checks and some users' backup scripts; fixed by a patched package — [It's FOSS, 2025-10-27](https://itsfoss.com/news/ubuntu-25-10-rust-automatic-upgrade-bug/)
- sudo-rs (default in 25.10) had two security bugs disclosed in Nov 2025 (password echo on prompt timeout; `targetpw` ignored with cached auth). The project lead said neither involved memory safety — [The Register, 2025-11-13](https://www.theregister.com/2025/11/13/ubuntu_rust_sudo_hole/)
- Current releases: sudo-rs 0.2.15 (2026-08-31), ntpd 1.9.0 (2026-11-13), hickory-dns 0.26.4 (2026-11-24), zlib-rs 0.6.9 (2026-11-24; about 157 M recent downloads), libbz2-rs-sys 0.2.7 (2026-11-19), rav1d 1.1.0 (2025-05-07), rustls 0.23.46 (2026-11-12) — [sudo-rs](https://crates.io/crates/sudo-rs), [ntpd](https://crates.io/crates/ntpd), [hickory-dns](https://crates.io/crates/hickory-dns), [zlib-rs](https://crates.io/crates/zlib-rs), [libbz2-rs-sys](https://crates.io/crates/libbz2-rs-sys), [rav1d](https://crates.io/crates/rav1d), [rustls](https://crates.io/crates/rustls)

Linux kernel
- At the December 2025 Maintainers Summit the kernel's Rust "experiment" was declared a success and the experimental tag dropped. Named code: Android binder merged for 6.18; Nova (NVIDIA) partly merged; an ashmem Rust module ships in Android 16 on 6.12 kernels; DRM was "about a year away" from requiring Rust for new drivers. Caveats: only LLVM-supported architectures; core-kernel dependence on Rust "another year or two" away — [LWN, 2025-12-13](https://lwn.net/Articles/1050174/)
- The Rust project's 2026 roadmap says the kernel still relies on unstable compiler features and aims to build on stable Rust only during 2026-2027 — [Rust project goals 2026, Rust for Linux](https://goals.rust-lang.org/2026/roadmap-rust-for-linux.html)
- On a Raspberry Pi 5 the stock kernel (`6.12.47+rpt-rpi-2712`) does not set `CONFIG_RUST`; a Rust-enabled kernel has to be built from the `rpi-6.12.y` tree with LLVM (about 1-2 hours natively), prebuilt kernels lack the metadata for out-of-tree Rust modules, and 6.12's Rust support is too thin for device-tree drivers — [HackMD "rust-kernel-rpi5" (undated)](https://hackmd.io/@akshaim/rust-kernel-rpi5)

Git, Android, government guidance
- Git: Rust optional since 2.49; enabled by default in both build systems from 2.55 (builds fail without Rust unless disabled); mandatory in 3.0, which has no release date; the last pre-3.0 release becomes an LTS — [git-scm.com BreakingChanges](https://git-scm.com/docs/BreakingChanges)
- Android (2025 data, published 2025-11-13): memory-safety bugs fell below 20% of all Android vulnerabilities for the first time; historical C/C++ density about 1,000 memory-safety vulnerabilities per MLOC versus an estimated 0.2 per MLOC for Rust (based on about 5 M lines and one near-miss); Rust changes had about a 4x lower rollback rate, 25% less time in review and 20% fewer revisions; about 4% of Android Rust is inside `unsafe{}`; the one near-miss (CVE-2025-48530, CrabbyAVIF) was a buffer overflow in unsafe Rust caught before release — [Google Security Blog](https://blog.google/security/rust-in-android-move-fast-fix-things/)
- CISA and NSA issued joint guidance in June 2025 calling memory-safe languages the most comprehensive mitigation for memory-safety vulnerabilities; the page gives no deadline — [CISA alert, 2025-06-24](https://www.cisa.gov/news-events/alerts/2025/06/24/new-guidance-released-reducing-memory-related-vulnerabilities); earlier joint guide: [The Case for Memory Safe Roadmaps](https://www.cisa.gov/case-memory-safe-roadmaps)

Bench instruments and MCP (closest analogue to this project)
- `oxrok` (first release 2026-10-23; latest stable 0.2.0 on 2026-11-08; 0.3.0-alpha.1 on 2026-11-14; GPL-3.0-or-later; 351 downloads) — [crates.io](https://crates.io/crates/oxrok)
- Its README says: "oxrok is a translation of the sigrok C and Python code into Rust, done by a large language model (Claude), with a human directing the work", and under Status: "Early, and most of it has never touched the device it drives." 88 driver modules / 193 driver ids on 2026-11-09; `oxrok mcp` serves MCP over stdio — [oxrok README](https://github.com/oxrok/oxrok)
- Correction to the brief's "mostly untested" framing: oxrok is mostly untested on hardware, but it is not untested. It runs differential tests against C libsigrok at a pinned commit, on emulated device models, requiring identical output and identical USB transaction logs, plus an adversarial review by a second agent. Real-hardware runs: LogicPort LA1034, an fx2lafw clone, a Rigol DS1102E, a Pico with sigrok-pico — [oxrok README](https://github.com/oxrok/oxrok)
- oxrok's own stated limits: a device model built from the C driver "can share the C's misunderstanding of the device"; matching it "proves parity with libsigrok on the paths the cases reach, not correct behaviour on hardware"; the LogicPort bench found three quirks emulation could not; timing is not real — [oxrok README](https://github.com/oxrok/oxrok)
- Official Rust MCP SDK: `rmcp` 3.11.0 (2026-11-17), Apache-2.0, repo `modelcontextprotocol/rust-sdk`, about 32 M recent downloads — [crates.io](https://crates.io/crates/rmcp)
- The older `sigrok` bindings crate is 0.3.1 from 2016 — [crates.io](https://crates.io/crates/sigrok)

Security / network tooling in Rust
- binwalk 3.1.0 (the Rust rewrite line), MIT, last crate release 2024-10-31 — [crates.io](https://crates.io/crates/binwalk)
- RustScan 2.4.1 (2025-02-23, GPL-3.0-only), feroxbuster 2.14.0 (2026-11-12, MIT), trippy 0.13.0 (2025-05-05), sniffnet 1.6.0 (2026-11-16), bandwhich 0.23.1 (2024-10-08), netscanner 0.6.43 (2026-07-06), rustnet-monitor 1.7.0 (2026-10-06) — [rustscan](https://crates.io/crates/rustscan), [feroxbuster](https://crates.io/crates/feroxbuster), [trippy](https://crates.io/crates/trippy), [sniffnet](https://crates.io/crates/sniffnet), [bandwhich](https://crates.io/crates/bandwhich), [netscanner](https://crates.io/crates/netscanner), [rustnet-monitor](https://crates.io/crates/rustnet-monitor)
- Arti (Tor in Rust) 2.8.0 (2026-11-04); Cloudflare Pingora 0.9.1 (2026-11-03) — [arti](https://crates.io/crates/arti), [pingora](https://crates.io/crates/pingora)
- Fish shell 4.0 is the Rust rewrite release — [fishshell.com "new in 4.0"](https://fishshell.com/blog/new-in-40/)
- Rust SDR is still small: futuresdr 0.10.0 (2026-10-20; about 2,200 recent downloads; the repo describes itself as "An experimental asynchronous SDR runtime"), seify 0.26.2, rtl-sdr-rs 0.4.3 (MPL-2.0), soapysdr 0.5.1 bindings — [futuresdr](https://crates.io/crates/futuresdr), [FutureSDR repo](https://github.com/FutureSDR/FutureSDR), [seify](https://crates.io/crates/seify), [rtl-sdr-rs](https://crates.io/crates/rtl-sdr-rs), [soapysdr](https://crates.io/crates/soapysdr)
- A RustConf 2026 talk decoded AM/FM and ADS-B from an RTL-SDR in Rust as a hobby project; the speaker's first ADS-B decoder recovered about a sixth of the available messages and a robust one "took considerable effort" — [LWN, 2026-09-24](https://lwn.net/Articles/1095721/)
- probe-rs 0.32.0 (2026-07-22) — [crates.io](https://crates.io/crates/probe-rs)
- Meshtastic's Rust crate is 0.1.9 (2026-05-25), GPL-3.0; a Rust Reticulum crate exists at 0.1.0 (2025-10-14), MIT — [meshtastic](https://crates.io/crates/meshtastic), [reticulum](https://crates.io/crates/reticulum)

C-to-Rust translation research
- DARPA TRACTOR's independent evaluator (MIT Lincoln Laboratory) published a public benchmark in Sept 2026 measuring correctness, safety, idiomaticity and performance; the abstract reports no translator scores — [arXiv 2609.25121](https://arxiv.org/abs/2609.25121); programme page: [DARPA TRACTOR](https://www.darpa.mil/research/programs/translating-all-c-to-rust)
- DepTrans (Apr-Jun 2026): repository-level C-to-Rust with a fine-tuned model reaches 60.7% compilation success and 43.5% computational accuracy, and builds 7 of 15 industrial C projects — [arXiv 2604.02852](https://arxiv.org/abs/2604.02852)
- SafeTrans: iterative compile/run repair lifts success from 54% to 80% (gpt-4o) over 2,653 C programs; some vulnerability classes in the C carry over into the Rust — [arXiv 2505.10708](https://arxiv.org/abs/2505.10708)
- With compile-and-equivalence feedback loops, differences between LLMs shrink — [arXiv 2512.02567](https://arxiv.org/abs/2512.02567)

### Inferences
- The owner's "I heard some others have started this" maps best onto (a) Ubuntu's coreutils/sudo-rs switch, which got wide coverage and is the nearest thing to "all the userland in Rust", (b) the kernel and Git announcements, and (c) for this specific domain, oxrok. The report should name all three and say plainly that they are component-by-component replacements.
- The Ubuntu story is the single most useful precedent for this project, in both directions: Rust removed memory-corruption bugs but 44 CVEs and several regressions came from logic and behavioural-parity gaps (TOCTOU, `date -r`, `install` vs symlinks). Rewrites fail on semantics, not on memory.
- oxrok is direct evidence that an agent-driven port of a large C instrument stack is feasible in weeks, and equally direct evidence of the ceiling: without the hardware, parity with the C on emulated paths is the strongest claim available.

### Gaps
- Not re-verified this session (background knowledge only; treat as unverified): Bottlerocket OS, Firecracker, System76 COSMIC, Redox OS, Asterinas, Tock, Oxide Hubris, Ferrocene qualification status, c2rust status, RustHound/rustcat/rcat/sniffglue/findomain/x8/rustbuster, Suricata's Rust parsers, Raspberry Pi Ltd's own use of Rust, hobby Pi OSes in Rust. Canonical home pages for the writer to link if wanted: bottlerocket.dev, firecracker-microvm.github.io, system76.com/cosmic, redox-os.org, asterinas.github.io, tockos.org, hubris.oxide.computer, ferrocene.dev, probe.rs.
- The eBPF sniffer "oryx": the crates.io crate named `oryx` (0.1.0, 2021) is a different project; I did not locate the sniffer's own release data.
- No "rtl_433 in Rust" port or crate was found (no crate named `rtl_433`; search returned only C forks).
- I recall a first kernel CVE in Rust code (rust_binder) shortly after the Dec 2025 summit; not confirmed this session. The LWN article says none had been issued at that time.
- A gbhackers page titled "CISA publishes guide to address memory safety vulnerabilities" appeared with a September 2026 date in search results; content not retrieved, so any 2026 CISA update is unverified. A "roadmap by 1 Jan 2026" expectation was not on the CISA pages I fetched.
- Phoronix pages on the Zellic audit and Linux 7.0 would not load; the facts above come from Ubuntu Discourse and LWN instead.

## 2. Concrete benefits for this appliance

### Takeaway
The benefits with real evidence are: removal of memory-corruption bugs in code that parses hostile input (radio, packets, serial), one shared type definition across firmware and host, small steady memory use, and easy cross-built single binaries. The "Rust is better for AI agents" claim is mixed: a compiler in the loop reliably fixes syntax and type errors but does not fix behaviour, and benchmark results on agents writing Rust point both ways.

### Cited Findings

Memory safety on hostile input
- rtl_433 (the planned ISM decoder) has a 2026 CVE of exactly the class Rust removes: CVE-2026-117609, stack-based buffer overflow in the Proflame 2 decoder, versions 21.12 to 25.12, CVSS 7.1, triggerable by a crafted OOK radio signal from a nearby transmitter — [VulnCheck advisory, 2026-10-27](https://www.vulncheck.com/advisories/rtl-433-through-25.12-stack-based-buffer-overflow-via-proflame-2-decoder)
- Wireshark vulnerability counts by year: 31 (2023), 19 (2024), 7 (2025), 121 (2026) — [stack.watch](https://stack.watch/product/wireshark/wireshark/)
- In the 50 most recent Wireshark entries on that page, the labels were: heap overflow 8, out-of-bounds read 5, dangling pointer 4, type confusion 3, integer overflow 3, buffer over-read 2, missing initialisation 2, classic/stack overflow 2; and non-memory classes: stack exhaustion 7, NULL dereference 5, algorithmic complexity 3, infinite loop 2 — [stack.watch](https://stack.watch/product/wireshark/wireshark/)
- Android's measured result for new Rust versus C/C++ (density, rollback, review time) is in section 1 — [Google Security Blog](https://blog.google/security/rust-in-android-move-fast-fix-things/)

Footprint on a small Pi
- One self-reported long-run measurement: a Rust home-automation service on a Pi Zero 2 W idled at 12-18 MB RSS over 15 days with a 3.1 MB ARM64 binary and 0.5-2% idle CPU; the same author puts Zigbee2MQTT (Node.js) at about 100 MB and Mosquitto (C) at about 10 MB — [dev.to, 2025-12-22](https://dev.to/scaraude/home-automation-in-3mb-building-a-rust-system-for-raspberry-pi-zero-56d6)

Energy
- The widely shared language energy rankings do not show causation. A controlled study (ASE 2025) finds that language implementation has no significant effect on energy beyond its effect on execution time, once core count and memory activity are controlled — [van Kempen et al., arXiv 2410.05460](https://arxiv.org/abs/2410.05460)

Shared types and deployment
- postcard 1.2.0 (2026-11-21) and postcard-rpc 0.12.1 (2025-12-04) give one `no_std` message definition usable on ESP32/RP2040 and the Pi; embassy-executor 0.10.0 (2026-03-20) and esp-hal 1.2.2 (2026-09-18) are the firmware side — [postcard](https://crates.io/crates/postcard), [postcard-rpc](https://crates.io/crates/postcard-rpc), [embassy-executor](https://crates.io/crates/embassy-executor), [esp-hal](https://crates.io/crates/esp-hal)
- GitHub-hosted arm64 Linux runners (`ubuntu-24.04-arm`, 4 vCPU) are generally available and free for public repositories — [GitHub changelog, 2025-08-07](https://github.blog/changelog/2025-08-07-arm64-hosted-runners-for-public-repositories-are-now-generally-available/)
- cargo-zigbuild 0.24.1 (2026-11-04) for cross/musl builds — [crates.io](https://crates.io/crates/cargo-zigbuild)
- Supply-chain tooling is current: cargo-audit 0.23.0 (2026-11-10), cargo-deny 0.22.1 (2026-11-24), cargo-vet 0.10.3 (2026-10-22), cargo-auditable 0.7.8 (2026-11-25) — [cargo-audit](https://crates.io/crates/cargo-audit), [cargo-deny](https://crates.io/crates/cargo-deny), [cargo-vet](https://crates.io/crates/cargo-vet), [cargo-auditable](https://crates.io/crates/cargo-auditable)
- Ubuntu is trialling cargo-auditable builds for well-known Rust packages including sudo-rs — [Ubuntu Discourse](https://discourse.ubuntu.com/t/an-update-on-rust-coreutils/80773)

Suitability for AI coding agents: evidence for
- SWE-bench Multilingual (300 tasks, 9 languages; SWE-agent + Claude 3.7 Sonnet): Rust had the highest resolution rate at 58.14%; C/C++ the lowest at 28.57%; Go 30.95%; JS/TS 34.88%; overall 42.67% — [swebench.com/multilingual](https://swebench.com/multilingual)
- Giving a model a compiler raised compile success by 5.3 to 79.4 percentage points across 16 models on 699 C tasks; syntax errors fell 75% and undefined-reference errors 87% — [arXiv 2601.12146](https://arxiv.org/abs/2601.12146)
- Anthropic's agent-built C compiler was written in Rust: about 100,000 lines, 16 parallel agents, nearly 2,000 sessions, just under USD 20,000, about 99% on most compiler test suites, compiles Linux 6.9 — [Anthropic engineering, 2026-02-05](https://www.anthropic.com/engineering/building-c-compiler)

Suitability for AI coding agents: evidence against
- Multi-SWE-bench (best agent, Claude 3.7 Sonnet): Python 52.20% resolved versus Rust 15.90%, C++ 14.73%, C 8.59%, Go 7.48%; the authors cite Python-tuned agent methods, harder issues and "intricate type systems" — [arXiv 2504.02605](https://arxiv.org/html/2504.02605v1)
- Rust-SWE-bench (500 tasks, 34 repos): ReAct-style agents resolved at most 21.2%; main failure causes were understanding repository-wide structure and complying with Rust's strict type and trait semantics; a Rust-specific agent reached 28.6% — [arXiv 2602.22764](https://arxiv.org/abs/2602.22764)
- The same compiler-in-the-loop study found program semantics were not improved by compiler access — [arXiv 2601.12146](https://arxiv.org/abs/2601.12146)
- A benchmark-driven LLM port of the Codex CLI agent from Rust (648K lines) to Python (41K lines) matched or beat the Rust original on SWE-bench Verified (73.8% vs 70.0%) and trailed on Terminal-Bench (42.5% vs 47.5%); the authors argue that where API latency dominates, Python's brevity wins. Single case study, v1 preprint — [arXiv 2604.11518](https://arxiv.org/abs/2604.11518)
- Carlini rates the agent-written compiler's Rust as "reasonable" but not near expert quality, and reports that new features "frequently broke existing functionality" until strict CI was added — [Anthropic engineering](https://www.anthropic.com/engineering/building-c-compiler)

### Inferences
- My tally of the stack.watch sample (not a figure from the page): about 29 of 50 recent Wireshark bugs are classes safe Rust prevents outright (overflows, out-of-bounds, dangling pointers, type confusion, uninitialised reads); NULL dereference becomes an `Option` at compile time; stack exhaustion, algorithmic complexity and infinite loops survive a Rust rewrite as denial of service. So roughly half to two-thirds of dissector-class bugs go away and the rest do not.
- The 2026 jump in Wireshark CVEs (7 to 121) means C parser bugs are now being found far faster than before. The cause is not stated on the page; AI-assisted bug-finding is my guess. For this appliance the practical reading is that C decoders fed by radio or network input must be treated as exploitable and isolated.
- That argues for sandboxing the wrapped C tools at least as strongly as it argues for rewriting them. A crash or compromise inside an unprivileged `rtl_433` child process that can only emit JSON on a pipe is contained; the same overflow inside a C library linked into the supervisor is not.
- Energy: on a Pi that is mostly idle between captures, language choice is a second-order effect. The real levers are fewer always-on interpreter processes (RAM, wakeups) and duty-cycling radios. No source measured this for a Pi appliance.
- The three agent benchmarks are not comparable with each other (different task pools and difficulty), so the honest summary is: no settled evidence that agents are more accurate in Rust than Python; good evidence that a strict compiler removes one whole class of agent error cheaply; and good evidence that behaviour still needs tests.
- For this project the agent argument that survives is specific: Rust lets the safety layer and protocol types be encoded so that wrong states fail to compile, and CI can gate on that without hardware.

### Gaps
- No rigorous Rust-versus-Python/Node daemon benchmark on a Pi 4/5 (startup time, RSS, CPU) was found; only the self-reported Pi Zero figures above. Startup time has no measurement at all.
- No published figure for the share of rtl_433 or tcpdump CVEs that are memory-safety bugs. Other rtl_433 CVE ids surfaced by search (CVE-2022-25050, CVE-2022-27419, CVE-2025-34450) were not read.
- No source measured GC pauses in Python capture loops on a Pi, tokio long-run reliability on ARM64, or Pi 5 idle power; these stay as engineering judgement.
- No sandboxing-specific evidence was collected (seccomp/landlock/systemd hardening for Rust daemons).

## 3. Concrete costs and risks

### Takeaway
The dominant risk is behavioural, not technical: mature decoders carry years of undocumented device quirks that a port silently loses, and the Ubuntu, oxrok and C-to-Rust research results all show that LLM or human rewrites pass the tests they were given and fail on what the tests never covered. The second risk is licence: several of the tools and even some Rust crates in the plan are GPL, which an Apache-2.0 daemon can only use across a process boundary.

### Cited Findings

Lost field hardening and parity gaps
- Ubuntu's rewrite of small, well-specified utilities with an existing GNU test suite still produced 113 audit issues, 44 CVEs, a silently broken update mechanism and a late `cp` regression (section 1) — [Ubuntu Discourse](https://discourse.ubuntu.com/t/an-update-on-rust-coreutils/80773), [It's FOSS](https://itsfoss.com/news/ubuntu-rustification-coreutils-migration/)
- oxrok's lessons file records three verification failures typical of agent ports: a device model whose timing was copied from the C driver being used to "prove" a C bug (circular); a false code comment about a dependency (`rusb`) that a shared method was built on, causing hangs in 5 of 8 review runs and 27 false failures; and a review that found 20 of one model's 50 checks never fired in any test — [oxrok README](https://github.com/oxrok/oxrok)
- State-of-the-art automatic C-to-Rust still fails on roughly half of real repositories (43.5% computational accuracy; 7 of 15 industrial projects build), and translated code can keep the original's vulnerability classes — [arXiv 2604.02852](https://arxiv.org/abs/2604.02852), [arXiv 2505.10708](https://arxiv.org/abs/2505.10708)
- Even a hand-written Rust ADS-B decoder by an experienced developer first recovered about one sixth of available messages — [LWN](https://lwn.net/Articles/1095721/)
- Cursor's agent-built Rust browser (about 3 M lines in a week) drew criticism for a failing CI build, an 88% Actions job failure rate cited by one critic, and bloat relative to Servo and Ladybird at about 1 M lines each — [The Register, 2026-01-22](https://www.theregister.com/2026/01/22/cursor_ai_wrote_a_browser/)
- Anthropic's compiler author: the task verifier must be "nearly perfect, otherwise Claude will solve the wrong problem" — [Anthropic engineering](https://www.anthropic.com/engineering/building-c-compiler)

Wrapping is not rewriting
- Several "Rust" options in the plan are Rust bindings over C/C++: whisper-rs 0.16.0, ort 2.0.0-rc.15 (still a release candidate), pdfium-render 0.9.6, opencv 0.102.0, libcamera 0.8.0, alsa 0.12.1, pipewire 0.10.1, soapysdr 0.5.1, pcap 2.5.0, rusqlite 0.43.0 — [whisper-rs](https://crates.io/crates/whisper-rs), [ort](https://crates.io/crates/ort), [pdfium-render](https://crates.io/crates/pdfium-render), [opencv](https://crates.io/crates/opencv), [libcamera](https://crates.io/crates/libcamera), [alsa](https://crates.io/crates/alsa), [pipewire](https://crates.io/crates/pipewire), [soapysdr](https://crates.io/crates/soapysdr), [pcap](https://crates.io/crates/pcap), [rusqlite](https://crates.io/crates/rusqlite)
- Android's one Rust near-miss was in `unsafe` code (about 4% of their Rust) — [Google Security Blog](https://blog.google/security/rust-in-android-move-fast-fix-things/)

Licence
- GPL items in or near the plan: `meshtastic` crate GPL-3.0; `oxrok` GPL-3.0-or-later; `rustscan` GPL-3.0-only — [meshtastic](https://crates.io/crates/meshtastic), [oxrok](https://crates.io/crates/oxrok), [rustscan](https://crates.io/crates/rustscan)
- Piper's maintained home (`OHF-Voice/piper1-gpl`) is GPL-3.0 and embeds espeak-ng; it offers a CLI, an HTTP server and a C/C++ `libpiper` — [piper1-gpl](https://github.com/OHF-Voice/piper1-gpl)
- Weak-copyleft crates that are fine to link but need file-level care: `serialport` (MPL-2.0), `rtl-sdr-rs` (MPL-2.0) — [serialport](https://crates.io/crates/serialport), [rtl-sdr-rs](https://crates.io/crates/rtl-sdr-rs)

Ecosystem gaps (by crate state)
- Solar modelling: `pvlib-rust` 0.1.6, created 2026-03-21, 470 total downloads, single author — [crates.io](https://crates.io/crates/pvlib-rust)
- Speech: `sherpa-rs` 0.6.8 last released 2025-10-05; `piper-rs` 0.2.0 (2026-05-21, about 18k recent downloads) — [sherpa-rs](https://crates.io/crates/sherpa-rs), [piper-rs](https://crates.io/crates/piper-rs)
- BLE: `bluer` 0.17.4 last released 2025-06-06 (official BlueZ bindings, Linux only); `btleplug` 0.13.5 (2026-10-10) — [bluer](https://crates.io/crates/bluer), [btleplug](https://crates.io/crates/btleplug)
- Plotting: `plotters` 0.3.7 last released 2024-09-08 (heavily used, slow-moving) — [crates.io](https://crates.io/crates/plotters)
- Raw packets: `pnet` 0.35.0 last released 2024-05-30 — [crates.io](https://crates.io/crates/pnet)
- Pi GPIO: `rppal` 0.22.1 last released 2024-12-11 — [crates.io](https://crates.io/crates/rppal)
- Vehicle protocols: `obd2` 0.1.0 (2023, 10 recent downloads), `iso-tp` 0.1.0-alpha.5 (2023), `uds-rs` 0.3.0 (2026-04), `automotive` 0.6.0 (2026-11-06, 472 recent downloads); no crate named `kline` or `can-isotp` exists — [obd2](https://crates.io/crates/obd2), [iso-tp](https://crates.io/crates/iso-tp), [uds-rs](https://crates.io/crates/uds-rs), [automotive](https://crates.io/crates/automotive)

Build cost
- Native builds on a Pi 5 are slow: a kernel build takes about 1-2 hours — [HackMD](https://hackmd.io/@akshaim/rust-kernel-rpi5)
- Kernel-side Rust is not a practical option on the Pi's vendor kernel today (section 1) — [HackMD](https://hackmd.io/@akshaim/rust-kernel-rpi5)

### Inferences
- The strongest argument against "all Rust": the value of rtl_433, Wireshark, Kismet and sigrok is their catalogue of device and protocol knowledge, added by many people with the actual hardware in hand. An agent can translate the code but cannot own the devices. A port is then permanently behind upstream on new devices and can only be verified against the C tool it is trying to replace, so the C tool has to stay installed anyway as the oracle.
- "Engineering effort is cheap with agents" does not make rewrites cheap, because the cost that remains is verification, and verification needs recorded captures or hardware. Budget rewrites by the size of the test corpus available, not by lines of code.
- Because GPL tools dominate this domain (rtl_433, Kismet, sigrok, Wireshark, readsb, Piper, Meshtastic), an out-of-process driver architecture is required for licence reasons alone, independent of the language question. Porting GPL C to Rust yields a GPL derivative (oxrok's choice), so a port does not solve the licence problem either.
- A Rust crate that wraps a C library inherits that library's bugs and adds an FFI boundary; for hostile-input parsers, a separate sandboxed process is the safer packaging than in-process bindings.

### Gaps
- The FSF GPL FAQ would not load, so the "separate programs communicating over pipes/sockets are normally separate works" position is stated from general knowledge and not cited. The writer should cite gnu.org/licenses/gpl-faq.html#MereAggregation after checking it. This is not legal advice.
- Licences of rtl_433, Kismet, readsb, sigrok, tshark, nmap, ODAS, libcamera and espeak-ng were not re-verified this session.
- No measurement of Rust workspace compile time on a Pi 5 or of CI minutes for a workspace this size was found. Binary-size data is limited to the single 3.1 MB example.
- Maturity of geospatial/map rendering crates and of Rust PipeWire/ALSA capture for mic arrays was not investigated beyond version numbers.
- I believe `rppal` was retired by its maintainer in 2025; not confirmed, only the last-release date is.

## 4. The middle path: Rust supervisor, out-of-process drivers, Rust crates where mature

### Takeaway
A Rust supervisor that owns the MCP surface, safety layer, recording and storage, with each proven C/C++/Python tool run as an unprivileged child process behind a narrow typed protocol, gets nearly all the benefits and avoids the two big costs. Rust-native crates are mature enough today for buses, storage, recording, networking, rendering and flashing; they are not mature for SDR decoding, solar modelling, speech and Meshtastic.

### Cited Findings

Mature enough to adopt now (version, last release)
- MCP: rmcp 3.11.0 (2026-11-17) — [crates.io](https://crates.io/crates/rmcp)
- Async runtime / web: tokio 1.54.1 (2026-10-21), axum 0.8.9 — [tokio](https://crates.io/crates/tokio), [axum](https://crates.io/crates/axum)
- Modbus: tokio-modbus 0.17.0 (2025-10-22) — [crates.io](https://crates.io/crates/tokio-modbus)
- CAN: socketcan 4.0.0 (2026-09-05) — [crates.io](https://crates.io/crates/socketcan)
- USB: nusb 0.2.9 (2026-10-24), pure Rust; oxrok uses nusb as its default USB backend — [nusb](https://crates.io/crates/nusb), [oxrok README](https://github.com/oxrok/oxrok)
- Serial: serialport 4.10.1 (2026-09-08), tokio-serial 5.5.0 (2026-06-15) — [serialport](https://crates.io/crates/serialport), [tokio-serial](https://crates.io/crates/tokio-serial)
- GPIO: gpiocdev 0.8.0 (2025-11-19), linux-embedded-hal 0.5.0 (2026-08-24) — [gpiocdev](https://crates.io/crates/gpiocdev), [linux-embedded-hal](https://crates.io/crates/linux-embedded-hal)
- Flashing/debug: probe-rs 0.32.0 (2026-07-22) — [crates.io](https://crates.io/crates/probe-rs)
- Storage/search: rusqlite 0.43.0 (2026-11-15), redb 5.0.0 (2026-11-18), fjall 3.1.13, tantivy 0.27.0 (2026-10-17), blake3 1.8.7, ed25519-dalek 3.0.0 — [rusqlite](https://crates.io/crates/rusqlite), [redb](https://crates.io/crates/redb), [fjall](https://crates.io/crates/fjall), [tantivy](https://crates.io/crates/tantivy), [blake3](https://crates.io/crates/blake3), [ed25519-dalek](https://crates.io/crates/ed25519-dalek)
- Recording: mcap 0.26.0 (2026-10-19), Foxglove's own crate — [crates.io](https://crates.io/crates/mcap)
- MQTT: rumqttc 0.25.1 (2025-11-21), rumqttd 0.20.0 (2025-09-29) — [rumqttc](https://crates.io/crates/rumqttc), [rumqttd](https://crates.io/crates/rumqttd)
- Packet summaries: etherparse 0.21.1 (2026-11-23), pcap-parser 0.17.0 (2025-07-25); tshark wrapper rtshark 4.3.0 (2026-04-04) — [etherparse](https://crates.io/crates/etherparse), [pcap-parser](https://crates.io/crates/pcap-parser), [rtshark](https://crates.io/crates/rtshark)
- Time, TLS, QUIC: jiff 1.0.3 (2026-11-24), rustls 0.23.46, quinn 0.11.12 (2026-09-14) — [jiff](https://crates.io/crates/jiff), [rustls](https://crates.io/crates/rustls), [quinn](https://crates.io/crates/quinn)
- Rendering: image 0.25.10, resvg 0.50.0 (2026-11-11), tiny-skia 0.12.0, plotters 0.3.7 — [image](https://crates.io/crates/image), [resvg](https://crates.io/crates/resvg), [tiny-skia](https://crates.io/crates/tiny-skia), [plotters](https://crates.io/crates/plotters)
- PDF: pdfium-render 0.9.6 (2026-11-21), lopdf 0.47.0 (2026-11-04) — [pdfium-render](https://crates.io/crates/pdfium-render), [lopdf](https://crates.io/crates/lopdf)
- Embeddings/inference: fastembed 7.8.0 (2026-11-22), candle-core 0.11.1, tract-onnx 0.23.10 (2026-11-24, pure Rust), burn 0.22.1 — [fastembed](https://crates.io/crates/fastembed), [candle-core](https://crates.io/crates/candle-core), [tract-onnx](https://crates.io/crates/tract-onnx), [burn](https://crates.io/crates/burn)
- Data: polars 0.55.2, ndarray 0.17.2 — [polars](https://crates.io/crates/polars), [ndarray](https://crates.io/crates/ndarray)
- Firmware/host shared types: postcard 1.2.0, postcard-rpc 0.12.1 — [postcard](https://crates.io/crates/postcard), [postcard-rpc](https://crates.io/crates/postcard-rpc)

Usable with caution
- BLE on Linux: bluer 0.17.4 (no release since 2025-06) — [crates.io](https://crates.io/crates/bluer)
- Speech-to-text: whisper-rs 0.16.0 (2026-03-12; bindings to whisper.cpp) — [crates.io](https://crates.io/crates/whisper-rs)
- ONNX Runtime: ort 2.0.0-rc.15 (still RC after years) — [crates.io](https://crates.io/crates/ort)
- Camera: libcamera 0.8.0 (2026-11-05; about 15k recent downloads) — [crates.io](https://crates.io/crates/libcamera)
- Audio: alsa 0.12.1, cpal 0.18.2, pipewire 0.10.1 — [alsa](https://crates.io/crates/alsa), [cpal](https://crates.io/crates/cpal), [pipewire](https://crates.io/crates/pipewire)
- Vehicle CAN stack: automotive 0.6.0 (active, small user base) — [crates.io](https://crates.io/crates/automotive)

Immature in late 2026 (do not depend on in phase 1-2)
- pvlib-rust 0.1.6; sherpa-rs 0.6.8; piper-rs 0.2.0; reticulum 0.1.0; meshtastic 0.1.9 (also GPL-3.0); futuresdr 0.10.0 ("experimental"); obd2 0.1.0; iso-tp alpha — links in sections 1 and 3

Wrapped-tool interfaces
- oxrok exposes sigrok-equivalent capture through a CLI and an MCP stdio server, so it can be used as a separate GPL process without linking — [oxrok README](https://github.com/oxrok/oxrok)
- Piper offers CLI and HTTP server modes — [piper1-gpl](https://github.com/OHF-Voice/piper1-gpl)

### Inferences
- "Strangler" replacement (wrap first, record sessions, then port one hot path with the wrapped tool as oracle) fits where (a) the tool is a pure function of recorded input, so differential testing needs no hardware, and (b) only a small subset is needed. Best fits: the handful of rtl_433 device decoders the owner actually uses (replay recorded IQ/pulse files through both), pvlib functions for the failing-panel test (compare against pvlib on a grid of inputs), packet summaries (etherparse vs tshark fields on the same pcap), and kronicle-lite (the owner's own TypeScript tool, per project memory, so he controls the spec and can publish conformance vectors).
- It does not fit where correctness depends on live hardware timing or on a long tail of devices: Kismet, full sigrok, full Wireshark dissection, ODAS beamforming, readsb's demodulator.
- The MCAP session recorder is what makes later ports cheap: every wrapped tool's raw input and decoded output recorded from day one becomes the differential corpus. This is the dogfooding path the owner asked for.
- The narrow driver protocol should be the same postcard/JSON schema whether the driver is a Rust crate, a wrapped C binary or an ESP32 module, so a wrapped tool can later be swapped for a Rust one without touching the supervisor.

### Gaps
- Whether rtl_433, Kismet and readsb output schemas are stable enough to pin was not checked this session.
- No evaluation of Rust PDFium binary availability for ARM64 or its licence terms.
- fastembed/ort memory use on a 4 GB Pi 5 not measured.
- No evidence found on rumqttd as a production broker on a Pi.

## 5. What to write fresh in Rust, and what not to rewrite in phase 1-2

### Takeaway
Write fresh in Rust where nothing exists and correctness is the product: the knowledge store, the safety/interlock layer, the manifest and conformance tooling, the owner's own device protocols, the fusion engine and evidence rendering. Leave the big multi-device decoders alone and run them as sandboxed children.

### Cited Findings
- No K-line crate was found by name on crates.io (`kline` and `can-isotp` do not exist); the generic `obd2` crate is a 2023 0.1.0 with 10 recent downloads — [obd2](https://crates.io/crates/obd2), [crates.io API lookups 2026-11-25](https://crates.io/crates/iso-tp)
- Modbus building block for JBD/EG4 drivers exists and is mature: tokio-modbus 0.17.0 — [crates.io](https://crates.io/crates/tokio-modbus)
- Hash-chain and index building blocks exist: blake3 1.8.7, rusqlite 0.43.0, redb 5.0.0, tantivy 0.27.0 — [blake3](https://crates.io/crates/blake3), [rusqlite](https://crates.io/crates/rusqlite), [redb](https://crates.io/crates/redb), [tantivy](https://crates.io/crates/tantivy)
- Proof tooling for the safety layer exists: kani-verifier 0.68.0 (2026-09-16), proptest 1.12.0 (2026-10-10) — [kani-verifier](https://crates.io/crates/kani-verifier), [proptest](https://crates.io/crates/proptest)
- rtl_433's decoders are a live source of memory bugs (CVE-2026-117609) — [VulnCheck](https://www.vulncheck.com/advisories/rtl-433-through-25.12-stack-based-buffer-overflow-via-proflame-2-decoder)
- The only full sigrok port in Rust is weeks old, alpha, GPL, and by its own account mostly unproven on hardware — [oxrok README](https://github.com/oxrok/oxrok)
- An earlier note in this project found Rust BLE multimeter crates limited to `bm78x` and `fluke-connect-client`, with none for OWON/Aneng/Pokit — [crates.io search](https://crates.io/search?q=multimeter)

### Inferences
- Write fresh (central to the project, no adequate crate): knowledge store; Interlock safety layer (small, state-machine shaped, suits property tests and Kani); module manifest and conformance suite; K-line / ISO 9141 / KWP2000 host and adapter firmware; JBD UP16S and EG4 / SMG-II drivers over tokio-modbus; compact LoRa telemetry protocol as a `no_std` crate shared with firmware; Bluetooth multimeter bridge over bluer; fusion/trigger engine; evidence-bundle rendering.
- Do not rewrite in phase 1-2: rtl_433, Kismet, readsb, sigrok/libsigrokdecode, tshark/Wireshark dissectors, nmap, ODAS, Piper/espeak-ng, libcamera/rpicam, pvlib, whisper.cpp, ONNX Runtime, PDFium, the kernel and Wi-Fi drivers.
- The safety layer is the one place where "everything in Rust, no FFI, no `unsafe`" should be an absolute rule (`#![forbid(unsafe_code)]`), because its whole value is being small enough to verify.

### Gaps
- The claim that no Rust K-line crate exists rests on name lookups and the brief's earlier research, not on an exhaustive search.
- JBD UP16S and EG4/SMG-II register maps and any existing Rust drivers were out of scope here.

## 6. Operating-system level

### Takeaway
No credible all-Rust userland exists for a Pi 5 appliance in late 2026, and Ubuntu's partial version is still shaking out. The reliable shape is a boring Debian-family or Buildroot image on the Raspberry Pi vendor kernel, read-only root with A/B slots and the firmware's `tryboot` fallback, systemd as init, and the Rust supervisor as one static binary. C stays in the kernel, firmware and Wi-Fi drivers regardless.

### Cited Findings
- NixOS on Pi 5 (wiki edited 2026-07-18): only `nixos-unstable` images boot it ("NixOS 26.05 images do not include the required Pi 5-family boot files"); it uses a pinned Raspberry Pi downstream kernel; the stock U-Boot chain cannot complete an NVMe boot; the community UEFI port was archived in Feb 2025 — [NixOS wiki](https://wiki.nixos.org/wiki/NixOS_on_ARM/Raspberry_Pi_5)
- RAUC A/B updates work on Pi 5 without U-Boot by using the firmware's `tryboot` flag and `autoboot.txt`; this needs the downstream kernel (`vcio` is not upstream), used a custom RAUC backend not yet merged, and relies on a `boot_partition` conditional the author calls undocumented — [Bootlin, 2025-05-13](https://bootlin.com/?p=19769)
- Home Assistant OS uses Buildroot plus RAUC (the page names Pi 5 builds; it does not mention the Yellow) with A/B slots, signed bundles, automatic fallback after failed boots, and `tryboot` on Pi 5 — [Home Assistant developer docs](https://developers.home-assistant.io/docs/operating-system/update-system)
- Ubuntu documents an A/B boot scheme for Raspberry Pi — [Ubuntu docs (title only, not read)](https://ubuntu.com/hardware/docs/boards/explanations/piboot-ab/)
- Monitor mode on the Pi 5's onboard Wi-Fi requires Nexmon: patched Broadcom firmware plus a patched `brcmfmac` driver built by DKMS against the running kernel (`brcmfmac-nexmon-dkms`, `firmware-nexmon`), tested on Pi 5 64-bit — [Kali blog, 2025-07-22](https://www.kali.org/blog/raspberry-pi-wi-fi-glow-up/)
- The Pi vendor kernel ships without Rust enabled (section 1) — [HackMD](https://hackmd.io/@akshaim/rust-kernel-rpi5)
- Ubuntu 26.04 LTS userland is Rust for most coreutils and sudo; `build-essential` hard-depends on `coreutils-from-uutils` — [Ubuntu Discourse](https://discourse.ubuntu.com/t/an-update-on-rust-coreutils/80773)

### Inferences
- Recommended base: Raspberry Pi OS Lite (64-bit) for phase 1 because every wrapped tool, DKMS driver and vendor kernel feature works there with no porting; move to a Buildroot or Yocto image with RAUC and `tryboot` A/B once the software set is stable. HAOS proves that pattern on this hardware.
- Power-loss posture: read-only root (squashfs or ext4 mounted ro, or overlayfs with tmpfs upper), a separate journaled data partition for MCAP and the knowledge store, append-only writes with fsync at chain-commit points, and a hash-chain check on boot. The knowledge store's hash chain doubles as corruption detection.
- NixOS gives the best reproducibility story but on Pi 5 it is unstable-channel-only with NVMe boot gaps; not a phase-1 choice for an appliance that must just boot.
- A Rust init or supervisor in place of systemd buys nothing here. systemd already provides watchdog, restart policy, sandboxing directives and socket activation for the child drivers; the project's own supervisor should sit under it.
- If the pack runs Ubuntu 26.04/26.10 it would get a largely Rust coreutils userland for free, and with it the parity bugs; Debian-based Pi OS avoids that variable.

### Gaps
- Bottlerocket-on-Pi, Alpine-on-Pi 5 with musl, Mender and systemd-sysupdate were not researched this session.
- Raspberry Pi OS's built-in overlay/read-only option and SD-versus-NVMe wear guidance were not sourced.
- No evidence gathered on Rust init systems.
- Nexmon stability on the Pi 5 across kernel updates was not checked.

## 7. AI-agent development workflow

### Takeaway
The published agent-built Rust projects agree on one thing: output quality tracks the quality of the automated oracle, not the language. For this project that means recorded-capture differential tests, strict CI that blocks regressions, and independent review of the tests themselves, with Rust's compiler and lints as the cheap first gate.

### Cited Findings
- Anthropic compiler project practices: an external oracle (GCC torture tests, differential compilation against GCC with delta debugging); CI that blocks commits breaking existing tests, added after agents kept regressing features; terse test output with details in log files; a `--fast` deterministic sample mode; README/progress files because each agent starts cold — [Anthropic engineering](https://www.anthropic.com/engineering/building-c-compiler)
- oxrok practices: C oracle built with `--coverage` at a pinned commit; scenario files run under both tools on a fresh emulated device; byte-identical wire logs required; gcov share of the C driver reached is reported per driver (60% to 92%); explicit `--oracle-differs` / `--oracle-bug` flags that fail once the two sides agree again; hardware-in-the-loop checks kept separately; a second agent reviews each port against the C; "a unit test per check" after finding checks that never fired — [oxrok README](https://github.com/oxrok/oxrok)
- Cursor's browser shows the failure mode when volume is the metric: millions of lines, failing CI — [The Register](https://www.theregister.com/2026/01/22/cursor_ai_wrote_a_browser/)
- Rust-specific agent weakness: repository-wide structure and trait/type compliance; issue reproduction is "critical"; dynamic tracing tooling lifted resolution from 21.2% to 28.6% — [arXiv 2602.22764](https://arxiv.org/abs/2602.22764)
- Feedback loops (compile + behavioural equivalence) reduce the gap between stronger and weaker models — [arXiv 2512.02567](https://arxiv.org/abs/2512.02567)
- Tool versions: cargo-nextest 0.9.152 (2026-11-13), proptest 1.12.0, cargo-fuzz 0.13.2 (2026-06-09), insta 1.49.0 (2026-10-03), kani-verifier 0.68.0, cargo-deny 0.22.1 — [cargo-nextest](https://crates.io/crates/cargo-nextest), [proptest](https://crates.io/crates/proptest), [cargo-fuzz](https://crates.io/crates/cargo-fuzz), [insta](https://crates.io/crates/insta), [kani-verifier](https://crates.io/crates/kani-verifier), [cargo-deny](https://crates.io/crates/cargo-deny)
- FutureSDR ships an `AGENTS.md` in its crate source, a sign that Rust projects now document conventions for coding agents — [docs.rs futuresdr source](https://docs.rs/crate/futuresdr/0.0.41/source/AGENTS.md)

### Inferences
- Workspace shape that follows from the evidence: many small crates with narrow public types (helps agents with the "repository-wide structure" weakness); `#![forbid(unsafe_code)]` by default with `unsafe` confined to named `-sys`/FFI crates run under Miri; clippy with warnings denied; nextest; proptest on every codec round-trip; cargo-fuzz targets on every parser that sees radio, serial or network bytes; insta snapshots for rendered evidence and MCP tool output; Kani harnesses only on the interlock state machine.
- The tests must not be written by the same agent pass from the same source as the code (oxrok's circular-oracle and never-fired-check lessons). Use the original tool's output on recorded sessions as ground truth, and have a second agent audit that each assertion can fail.
- Hardware-free CI is achievable for most of this project: protocol codecs, Modbus/CAN drivers against simulated devices, replay of MCAP sessions, firmware logic on host. Real-hardware checks remain necessary and should be tracked as an explicit list per driver, as oxrok does.

### Gaps
- No source was found for prusti/creusot/verus use in agent-written projects, or for Miri/coverage practice specifics in 2026.
- No published experience report of an agent-written Rust embedded or instrument daemon running in the field long-term; oxrok is the nearest and is weeks old.
- No data on CI minutes for comparable workspaces.

## 8. Synthesis: the six closing items requested

### Takeaway
Rust for everything the project itself authors on the Pi; proven non-Rust tools kept as sandboxed child processes; ports only behind differential tests.

### Cited Findings
- All facts used below are cited in sections 1-7; links are repeated where the item is a "see also".

### Inferences

(1) Direct answer: should all the Pi code be Rust?
All the code this project writes for the Pi should be Rust: supervisor, MCP server, drivers for his own instruments, recorder, knowledge store, safety layer, fusion and rendering. That is where the measured benefits land (no memory-corruption class in code he owns, one type definition shared with the ESP32/RP2040 firmware, a single cross-built binary, small steady memory, a compiler and lints that catch a whole class of agent mistakes before CI). All the code that runs on the Pi should not be Rust in phase 1-2. rtl_433, Kismet, readsb, sigrok, tshark, nmap, ODAS, Piper and pvlib hold years of device-specific knowledge that a port cannot verify without the devices; several are GPL, so they must sit behind a process boundary from an Apache-2.0 daemon anyway; and the Ubuntu coreutils rollout (113 audit findings, 44 CVEs, a silently broken updater) and oxrok's own README show that rewrites fail on behaviour, not memory. Run those tools as unprivileged children that speak a narrow typed protocol, record everything they see and say into MCAP, and port a component only when a recorded-capture differential test against the original exists and there is a concrete reason (a bug class, a licence, a footprint problem, or a feature the agent needs inside the decoder).

(2) Component verdict table

| Component | Verdict | Reason |
|---|---|---|
| Supervisor daemon + MCP server | Write fresh in Rust, on `rmcp` 3.11 + tokio | Core of the product; official SDK is mature and Apache-2.0 |
| Physical-safety / interlock layer | Write fresh in Rust, `forbid(unsafe_code)`, proptest + Kani | Correctness is the point; small enough to verify |
| Knowledge store (hash-chained) | Write fresh in Rust on rusqlite or redb + blake3 + tantivy | Nothing off the shelf; building blocks mature |
| kronicle-lite (TypeScript, owner's own) | Port later with differential tests and conformance vectors | Owner controls the spec; avoids a Node runtime on a 4 GB Pi; until then run out-of-process |
| Module manifest + conformance tooling | Write fresh in Rust | The standard he wants to set; types are the spec |
| Shared firmware/host message types | Use existing crates (postcard, postcard-rpc), own schema crate `no_std` | One definition on ESP32/RP2040 and Pi |
| Firmware (ESP32/RP2040) | Rust (embassy 0.10, esp-hal 1.2), as planned | Already decided; both current |
| Session recorder | Use existing crate (`mcap` 0.26) | Foxglove-maintained |
| Modbus drivers (JBD UP16S, EG4/SMG-II) | Write fresh on `tokio-modbus` 0.17 | No existing driver; bus crate mature |
| CAN / OBD-II over CAN | Use `socketcan` 4.0; evaluate `automotive` 0.6; write thin OBD layer | Bus crate mature; OBD crates are tiny or stale |
| K-line / ISO 9141 / KWP2000 | Write fresh (host + adapter firmware) | No crate found; needed for the 2003 Tacoma class |
| USB instruments | Use `nusb` 0.2.9 | Pure Rust, no libusb |
| Serial instruments | Use `serialport` / `tokio-serial` | Mature; MPL-2.0 is link-compatible |
| BLE multimeter bridge | Write fresh on `bluer` 0.17.4 | Open ground; note bluer's slow release cadence |
| GPIO / status LEDs | Use `gpiocdev` 0.8 | Kernel chardev API; avoid stale `rppal` |
| MCU flashing | Use `probe-rs` 0.32 (plus espflash) | Replaces OpenOCD for supported targets |
| MQTT to Home Assistant | Use `rumqttc` 0.25 | Mature client |
| Compact LoRa telemetry protocol | Write fresh, `no_std` | His own protocol; shared with firmware |
| Meshtastic link | Wrap out-of-process (Python or the GPL Rust client as a separate bridge) | Crate is GPL-3.0 and 0.1.x |
| rtl_433 | Wrap out-of-process, sandboxed, JSON out; port individual decoders later with IQ-replay differential tests | Hundreds of field-hardened decoders; live memory-safety CVE argues for isolation |
| Kismet | Wrap out-of-process | Large, hardware-coupled, GPL |
| readsb (ADS-B) | Wrap out-of-process | Robust demodulation is hard-won (Rust talk recovered 1/6 at first) |
| sigrok-cli | Wrap out-of-process; trial `oxrok` as an alternative child process for the Pico/fx2 only | oxrok is alpha, GPL, hardware-proven on 4 device types |
| tshark / Wireshark dissectors | Wrap out-of-process (e.g. via `rtshark`); use `etherparse`/`pcap-parser` for simple summaries | Thousands of dissectors; never rewrite |
| nmap | Wrap out-of-process (XML output) | No Rust equivalent of its fingerprint database |
| ODAS (mic array) | Wrap out-of-process; revisit later | DSP tuning is the value; no Rust equivalent found |
| Piper / espeak-ng (TTS) | Wrap out-of-process (CLI or HTTP) | GPL-3.0; piper-rs is 0.2.0 |
| Speech-to-text | Use `whisper-rs` 0.16 in a separate worker process | Bindings over C++; isolate the FFI |
| Camera | Wrap `rpicam-apps` as a child process; consider `libcamera` crate 0.8 later | Vendor stack is C++; bindings are young |
| Embeddings | Use `fastembed` 7.8 (ONNX Runtime underneath) or `candle` | Mature enough; measure RAM on 4 GB first |
| pvlib (solar modelling) | Wrap Python out-of-process; port the few functions needed with differential tests against pvlib | pvlib-rust is 0.1.6 with 470 downloads |
| Fusion / trigger engine | Write fresh in Rust | Concurrency across sensors is Rust's strength |
| Evidence-bundle rendering | Write fresh using `image`, `plotters`, `resvg`/`tiny-skia` | Mature crates; snapshot-testable |
| PDF page rendering for manuals | Use `pdfium-render` 0.9.6 (wraps PDFium) | Mature binding; C++ underneath |
| TLS / QUIC / time | Use `rustls`, `quinn`, `jiff` 1.0 | Mature |
| OS, init, coreutils, sudo | Leave alone (Pi OS Lite or Buildroot + systemd) | No all-Rust userland worth adopting; parity bugs still surfacing in Ubuntu |
| Kernel, Wi-Fi monitor-mode drivers, firmware | Leave alone | Vendor kernel has no Rust enabled; Nexmon is patched C via DKMS |

(3) Benefits that are real for this project, ranked
1. Types as the contract: one `no_std` schema crate shared by firmware, drivers, supervisor and MCP tools, so an agent cannot ship mismatched ends of a protocol.
2. Memory safety in the code that owns the safety layer and touches hostile bytes (evidence: Android's density and rollback numbers; rtl_433 and Wireshark CVE classes).
3. A cheap, strict first verifier for agent-written code (compiler, clippy, `forbid(unsafe_code)`), which measurably removes syntax/type/reference errors, though not behavioural ones.
4. Single cross-built binary per component, built and tested on free native arm64 CI runners; no interpreter or venv drift on an offline device.
5. Supply-chain hygiene in one toolchain: cargo-deny for licences (catches GPL crates entering an Apache-2.0 build), cargo-audit, cargo-vet, auditable binaries.
6. Small, steady memory and no interpreter-per-daemon overhead on 4 GB (evidence is thin: one self-reported 12-18 MB example).
7. Concurrency without data races for multi-sensor capture (language guarantee; no appliance-specific measurement found).
8. Energy: marginal. Only through less CPU time; not a language property.

(4) Traps
- Treating "it compiles and passes its own tests" as parity. Ubuntu's `date -r` and oxrok's never-fired checks are the pattern.
- Letting the same agent derive the tests from the same C source as the port (circular oracle).
- Counting a Rust binding over a C/C++ library as "Rust" for safety purposes.
- Linking GPL code (the `meshtastic` crate, oxrok, RustScan, libpiper) into the Apache-2.0 supervisor.
- Porting a multi-device decoder without owning the devices, then carrying the upstream-tracking burden forever.
- Depending on 0.1-0.2 crates with a single author for anything on the critical path (pvlib-rust, piper-rs, reticulum, obd2).
- Compiling on the Pi, or building a custom Rust-enabled kernel, instead of cross-building in CI.
- Measuring progress in lines produced (Cursor browser) and not in recorded sessions that replay identically.
- Assuming a Rust rewrite removes denial-of-service: stack exhaustion, infinite loops and panics on malformed input survive; fuzz for them.
- Chasing an "all-Rust OS" on a board whose vendor kernel, firmware and Wi-Fi stack are C and will stay C.

(5) What "others have started" most likely refers to
- Ubuntu's Rust userland: [rust-coreutils update](https://discourse.ubuntu.com/t/an-update-on-rust-coreutils/80773), [26.10 completes the switch](https://itsfoss.com/news/ubuntu-rustification-coreutils-migration/), [sudo-rs default in 25.10](https://discourse.ubuntu.com/t/sudo-rs-is-now-default-for-questing-quokka/66497)
- Linux kernel: [Rust experiment concluded](https://lwn.net/Articles/1050174/), [2026 roadmap](https://goals.rust-lang.org/2026/roadmap-rust-for-linux.html)
- Git 3.0 requiring Rust: [BreakingChanges](https://git-scm.com/docs/BreakingChanges)
- Android's results: [Rust in Android](https://blog.google/security/rust-in-android-move-fast-fix-things/)
- In his own niche: [oxrok](https://github.com/oxrok/oxrok), a Claude-translated Rust sigrok with a built-in MCP server; and the official [Rust MCP SDK](https://crates.io/crates/rmcp)
- Security tooling rewrites: [binwalk v3](https://crates.io/crates/binwalk), [RustScan](https://crates.io/crates/rustscan), [feroxbuster](https://crates.io/crates/feroxbuster), [sniffnet](https://crates.io/crates/sniffnet), [Arti](https://crates.io/crates/arti)
- Agent-built Rust systems: [Anthropic's C compiler](https://www.anthropic.com/engineering/building-c-compiler), [Cursor's browser](https://www.theregister.com/2026/01/22/cursor_ai_wrote_a_browser/)
- Government push: [CISA/NSA June 2025 guidance](https://www.cisa.gov/news-events/alerts/2025/06/24/new-guidance-released-reducing-memory-related-vulnerabilities); translation research: [DARPA TRACTOR benchmark](https://arxiv.org/abs/2609.25121)

(6) Recommended engineering policy
1. Everything this project authors for the Pi is Rust; anything else needs a written reason.
2. No rewrite of an existing tool without a recorded-capture differential test against the original, run in CI, with the original pinned to a commit.
3. Third-party non-Rust tools run as unprivileged child processes behind one typed driver protocol; they are never linked into the supervisor.
4. The safety layer and the knowledge-store core are `#![forbid(unsafe_code)]`, have no FFI, and carry property tests; the interlock state machine also carries Kani harnesses.
5. `unsafe` and FFI live only in named wrapper crates, run under Miri where possible, and each wrapper runs in its own process when it parses untrusted input.
6. Every parser of radio, serial, BLE or network bytes has a cargo-fuzz target and must return errors, not panic.
7. cargo-deny gates licences on every PR: no GPL/AGPL crate in the Apache-2.0 dependency graph; GPL tools are separate programs only.
8. Record first: every driver writes raw input and decoded output to MCAP from its first version, so the corpus for future ports exists before anyone proposes one.
9. Tests are not accepted from the same agent pass as the code they test; a second reviewer checks that each assertion can fail, and each driver keeps an explicit "verified on real hardware" list.
10. Build and test on CI arm64 runners; the Pi receives signed binaries into an A/B slot; nothing is compiled on the device.

### Gaps
- The verdicts for ODAS, readsb, Kismet and nmap rest on the general evidence about rewrites plus crate searches; I did not find a Rust alternative for each and did not exhaustively search for one.
- The recommended policy is my synthesis; no source prescribes it as a set.
- Facts taken from the owner's stored project context and not from web sources: kronicle-lite being his own TypeScript tool, the 2003 Tacoma, the spare Home Assistant Yellow, and the dogfooding and failing-solar-panel requirements.
