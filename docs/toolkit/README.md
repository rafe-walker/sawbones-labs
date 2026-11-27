# Field toolkit plan

This is the plan for issue [#250](https://github.com/rafe-walker/sawbones-labs/issues/250): which open-source tools (many of them from Kali Linux) the S.A.W. pack and the T.A.B.L.E. bench use, how each one runs, how dangerous each one is, and how the agent gets at it. The per-tool tables are in [catalog.md](catalog.md). Decisions this plan builds on are in [DECISIONS.md](../DECISIONS.md) and [THREAT-MODEL.md](../THREAT-MODEL.md).

Scope is testing, auditing and reproducing problems on the owner's own devices and on the owner's own off-grid site. Nothing here is for other people's systems.

**Status.** This is a plan, not built software. Every item below is "not built" until its follow-up issue closes. Nothing was run on a Raspberry Pi 5 for this research, so every footprint statement is a prediction. Facts the research could not confirm are marked **(unverified)** in this file and in the catalog.

## 1. The short version

We wrap: each tool runs as an ordinary program in a sandbox under a typed Rust adapter that owns the arguments, limits, targets and parsing. The agent never types a command line. We split every tool into acquisition (touches a device; carries the hazard class) and analysis (a pure function over a stored file). We port only what passes a six-part test (section 3). Heavy analysis runs offline on the owner's Mac or the fixed bench box on files the Pi captured and proved by hash. GUI and server tools stay outside the trust boundary. The agent sees a summary plus a handle, and C.O.R.T.E.X. checks every target against the owner's list of owned assets.

## 2. The decision: wrap, with a sandbox underneath

Three options were compared: run the tool generically in a sandbox (the supervisor stores bytes and knows only size, hash and exit code), wrap it (the same sandboxed tool plus a typed Rust adapter that produces typed rows, a summary card and fenced text), or port it (our own Rust or a zero-import WebAssembly plugin, typed from the start).

"Generic" is acceptable only for "make a file and keep it". Anything the agent has to reason about needs the adapter. A port trades the tool's bugs for our own, and then we own every new device and protocol forever. The project's standing rule already says a tool that exists only in C or Python is used as it is ([DECISIONS.md](../DECISIONS.md), second round, "Rust rule"); this plan supplies the mechanics.

Why a sandbox is not optional: tools that decode bytes off the air or a wire are written in C and have a record of memory bugs. A 2026 overflow in a sub-GHz sensor decoder was triggerable by a crafted radio signal, and the packet analyser's flaw count rose sharply that year (both second-hand, via [project-document.md](../project-document.md)). The sandbox has risks too: two critical code-generator bugs in the WebAssembly runtime were published on 19 October 2026, one specific to aarch64 ([Wasmtime advisories](https://github.com/bytecodealliance/wasmtime/security/advisories)). So the wall is built twice: an outer wall for the tool and an inner wall for the parser.

### How a wrapped tool runs

1. `plan` checks scope, hazard class, quota and isolation.
2. The launcher verifies the binary's hash and makes an empty job directory.
3. It builds the arguments from a fixed template with typed slots. The agent cannot add, drop or reorder a token.
4. The tool starts inside an outer wall (a systemd template unit: resource caps, device allow-list, network off or pinned to one interface, read-only filesystem, one writable directory) and an inner wall (no new privileges, resource limits, a per-tool seccomp filter, an unprivileged user).
5. Output passes through bounded pipes; the process is killed at the byte limit.
6. A parser child with no files, network or process rights turns the bytes into typed rows. The host treats its reply as hostile bytes.
7. The host writes a capture record and blobs to C.H.A.R.T., reports any text to the taint latch, and checks the effect contract (for a passive job, that nothing transmitted).

For the outer wall we chose static systemd template units plus a small Rust launcher over bubblewrap or nsjail: no dependence on unprivileged user namespaces (unreliable on the CI runners, per repo notes), first-class resource caps, and a unit file that can be reviewed against a golden copy. The seccomp crate is already a plugin-host dependency. This is reasoned, not benchmarked. Tracked in [#253](https://github.com/rafe-walker/sawbones-labs/issues/253).

Native tools are a closed first-party set in the core ring, as [#177](https://github.com/rafe-walker/sawbones-labs/issues/177) says. A native adapter is not a plugin. Plugins stay zero-import WebAssembly.

## 3. When we port

Port a function only if all six hold:

1. It is a pure function of bytes or samples (no device, no network, no clock).
2. Its input is hostile or comes off the air.
3. The spec is small and stable.
4. A differential oracle exists: the original tool on recorded input.
5. The licence allows it: permissive source, or written from the protocol description.
6. Wrapping has a demonstrated cost: a security flaw, a footprint problem or a speed problem.

A port of GPL code is a derived work and would have to be GPL itself. A decoder written from the protocol description and tested against the original may be different, but that is a question for a lawyer before anyone relies on it.

What passes today: hash manifests (replacing hashdeep; oracle `sha256sum`); capture summary counts (oracle: tshark on a golden capture); the Wi-Fi scan and survey (replacing the `iw` child, oracle `iw` output); the spectrum sweep (oracle `rtl_power` CSV); passive Modbus, CAN and K-line parsing, with tshark, sigrok and cantools as second instruments; thermal compensation maths (oracle: the vendor's Apache-2.0 code); later, a few sub-GHz decoders for the owner's own sensors as WebAssembly. Our own `compute` operations (spectra, direction of arrival, leak and arc scoring, a seven-segment reader) are not ports, because no tool does this. Never ported, always wrapped: Kismet, aircrack-ng, hcxdumptool, ffmpeg, OpenCV, tesseract, sleuthkit, plaso, exiftool. The sub-GHz sensor tool has over 390 decoders, so it is not ported whole.

## 4. Acquisition and analysis

Acquisition needs a device, interface or bus, runs on the Pi in the sandbox, and carries the hazard class, scope check, consent and legal risk. Analysis needs a stored blob by hash, runs on the Pi for small jobs or offline on the Mac or bench box for heavy ones, and is replayable in CI. Example: capture with `dumpcap`, then summarise offline; record IQ, then decode with the tool's replay mode; capture a frame, then OCR the handle. Ghidra, Zeek, Suricata, inspectrum, GNU Radio, plaso, hashcat, SavvyCAN and PulseView never run on the 4 GB Pi.

Rejected inside the trust boundary: Audacity, Frigate, Obico, ntopng, gqrx, SDR++ and similar GUI or server tools. Frigate is a good product in the wrong shape: a second web server and database inside a pack whose security depends on a small trusted core.

## 5. Hazard classes by tool family

The scale is the project's own: **0** look (receive or read only), **1** safe (touch-safe and reversible), **2** armed (the session arm switch), **3** keyed (a panel press for that one action), **4** never (no row exists; not shipped enabled). A tool's class depends on the operation, so the table gives the range and the catalog gives the row.

| Family | Class 0 | Class 1 | Class 2 | Class 3 |
| --- | --- | --- | --- | --- |
| Sub-GHz and SDR | Receive, sweep, record, decode sensors | Our own LoRa link, receive | Our own LoRa link, transmit | Any transmit tool, if a transmit-capable SDR is ever added |
| Wi-Fi | Passive capture, survey, census | Not used (active scans are disputed, section 10) | Active scan; any transmit | Deauth or injection against an owned test access point; handshake capture with active vectors |
| Bluetooth, BLE, Zigbee | Passive census and sniffing | Scan; connect and read on an owned device | Non-protective writes | Any write to a battery or inverter |
| Network | Capture on an owned port; offline analysis | Discovery of owned hosts at a forced rate; dig, mtr, iperf3 | Scripts beyond the safe categories; proxy on one owned device | Not used |
| Modbus and serial | Listen-only capture and decode | Reads through the guard | Non-protective settings | Protection limits, charge voltages, inverter output |
| CAN, OBD-II, K-line | Receive and decode | Read services | Clear codes and actuator tests, engine off | Actuator tests on a running engine; fuzzing anything real |
| Firmware and hardware | Offline analysis; logic capture | Identify and read an owned board | Write with a recovery path and backup | Write with no recovery path; fuse, OTP or eFuse burns |
| Audio, camera, thermal, forensics | Record, capture, OCR, thermal read, hash, read metadata | Set a device control | Write file metadata | Not used |

### Class 4: never enabled

Not in the pack image, no capability row, and the agent's tool list says "blocked, class 4" with a reason. Nothing in the manifest format can add one.

- Wi-Fi: `mdk4` (beacon and deauth floods); rogue access point and credential capture suites (`wifiphisher`, `eaphammer`, `wifipumpkin3`, `hostapd-mana`, `airgeddon`).
- Bluetooth and 802.15.4: `btlejack` jam and hijack modes; `bluesnarfer`; `redfang`; the injecting and flooding tools of KillerBee (`zbfakebeacon`, `zbassocflood`, `zbpanidconflictflood`).
- Network: `bettercap` ARP and DNS spoofing and `wifi.deauth`; exploit, brute-force and denial-of-service NSE script categories.
- Radio generally: jammers of any kind, GNSS or cellular transmission, anything built to block authorised radio.
- Vehicle and bus: ECU flashing, emission-control changes, any generic raw-write tool (`cansend`, `cangen`, a raw Modbus write) that bypasses the guard.
- Mains work and string switching.
- Any tool that harvests other people's credentials or data by design, and any exploit or malware code.

The plan does not wrap `bettercap` at all (its recon is covered by Kismet and BlueZ); the owned-access-point audit goes through the wrapped `aireplay-ng` and `hcxdumptool` class 3 profiles. This settles a difference between two research reports (section 10).

## 6. How T.R.I.A.G.E. exposes capabilities

The T.R.I.A.G.E. tool list is fixed. The current MCP specification does not allow a per-connection tool list, and the project decided that modules and plugins appear as data, never as new tools ([DECISIONS.md](../DECISIONS.md), 26 November). So a wrapped tool never adds a tool. It adds a row.

- **Compiled vocabulary.** Capability names (for example `rf.listen.433`, `capture.wifi.survey`, `net.scan.owned`, `audio.analyze.spectrum`) come from a list compiled into the core. A manifest can only choose from it. No manifest text reaches a tool name, description, enum or example. Tracked in [#264](https://github.com/rafe-walker/sawbones-labs/issues/264).
- **Host-minted ids.** The first-party adapter is enrolled like a module, so each capability row gets a host-minted id such as `m7.a0`.
- **The flow uses only existing verbs:** `bench_status`, then `inventory{kind: capability}`, then `describe{id}`, then `plan`. For class 0 and 1, `invoke_safe` starts a job. For class 2 and 3, `invoke_armed` or `invoke_keyed` runs with the arm switch, the latch check and a lease. Then `wait_for` on a `job-` handle, then `get`, `compute` and `find` on the result.
- **Summary plus handle.** A job returns a typed summary card (counts, top-N tables, histograms, noise floors, detections) and a handle: store path, content hash, format, time range, position fix and clock source. Follow-up verbs ask questions of the stored data and return another card. A card defaults to under 10,000 tokens. The reason is a 2026 study cited in the design document, where raw packet-analyser JSON for a few megabytes averaged about 22 million tokens while compact summaries answered 99.3 percent of forensic questions (second-hand, not re-checked).
- **Raw captures stay in C.H.A.R.T.** Packet captures, IQ, audio, frames and firmware images are content-addressed blobs. They never travel inside an MCP result. The Mac pulls them by hash when needed.
- **Capture records.** Every job stores versions, binary hash, argument digest, the scope facts relied on, times on the shared clock and the sandbox profile digest, so a result can be reproduced or challenged offline.
- **Outside text is fenced and trips the taint latch.** Network names, device names, banners, file names, metadata strings, OCR text and transcripts are marked as coming from outside our code, capped in length (a starting figure of 64 bytes), stripped of control characters and delivered as separate data fields, never as free text. Any such text, and any image, reaching the agent's turn trips the latch before the result is returned, so the next class 2 or 3 action needs a fresh arm cycle or lease ([THREAT-MODEL.md](../THREAT-MODEL.md)). Summaries made only of numbers our own parser extracted are treated conservatively (they trip it) until the owner decides otherwise.
- **OCR is never a measurement.** An OCR or vision result is agent-inferred and can never become a reading. Any value that backs an action needs an electrical cross-check.
- **Failure cards say what was not seen**, and every row carries an owned, unknown or neighbour flag so audit questions become filters.

## 7. Scope enforcement by C.O.R.T.E.X.

The agent never types an address, a frequency or a device name. An adapter argument is a closed choice, a reference to an owner-attested asset or channel in C.H.A.R.T., or a number inside an attested range.

1. **Owned-assets list:** identifiers, ranges, and the frequencies and channels the owner may use, with power limits, stored as owner-attested facts (never "measured").
2. **At plan time** C.O.R.T.E.X. refuses a missing, stale or out-of-scope reference by name, with what would need to change ([#256](https://github.com/rafe-walker/sawbones-labs/issues/256)).
3. **At run time** the kernel backs it: a network namespace with only the allowed interface and routes limited to owned ranges, a USB allow-list limited to the leased device, no network unless declared. Neither wall trusts the other.
4. **Passive means passive, and we check.** After a passive job the supervisor compares transmit counters before and after; a change fails the job and blocks the adapter ([#257](https://github.com/rafe-walker/sawbones-labs/issues/257)).
5. **Limits live in C.O.R.T.E.X. and the cgroup, not the prompt:** maximum duration, output size from rate times duration, a forced rate on anything active, one user per radio or interface, quotas on transmit and active operations.
6. **Resource caps are a safety item.** The interlock heartbeat is answered from the supervisor main loop, so a runaway tool could drop the rails. Tools are pinned to a subset of cores under a hard memory cap and are shed first when hot ([#263](https://github.com/rafe-walker/sawbones-labs/issues/263), [#248](https://github.com/rafe-walker/sawbones-labs/issues/248)).
7. **Never offered:** jamming, deauthentication floods, exploit and malware scripts. No row, and no manifest field can add one.

## 8. Legal defaults

These are plain facts and defaults we chose. They are **not legal advice**. Rules can change and courts differ, so check them before relying on them. Receiving radio needs no licence, but what you do with what you receive can still be regulated.

**Transmitting (FCC).** Operating a jammer is illegal in the United States with no exemption for private land ([FCC](https://www.fcc.gov/general/jammer-enforcement)); GPS blockers count and satellite spoofing is barred. Deliberately blocking someone's Wi-Fi, even with ordinary deauthentication frames, was treated as unlawful interference when a hotel did it to guests' hotspots (a $600,000 penalty, [FCC DA 15-113](https://transition.fcc.gov/eb/Public_Notices/DA-15-113A1.html)). A deauthentication aimed at our own client on our own test access point is a test; one that reaches a neighbour's device is what the FCC penalised. Unlicensed devices must not cause harmful interference and must accept it ([47 CFR 15.5](https://www.law.cornell.edu/cfr/text/47/15.5)); power limits are in [47 CFR 15.247](https://www.law.cornell.edu/cfr/text/47/15.247), for example 1 W for digitally modulated systems in 902-928, 2400-2483.5 and 5725-5850 MHz. A high-gain antenna can move a certified radio outside its certification, and transmitting outside unlicensed rules generally needs a licence (both unverified). Replaying a captured signal is a transmission. Receiving needs no licence.

**Capturing other people's traffic (Wiretap Act).** Intercepting the contents of an electronic communication is a federal crime unless an exception applies, with up to five years ([18 U.S.C. 2511](https://www.law.cornell.edu/uscode/text/18/2511)). Exceptions include being a party, a party's consent, and systems the public can readily reach. In *Joffe v. Google* (2013) the Ninth Circuit held that unencrypted Wi-Fi payload data was not covered by the "radio communication" or "readily accessible" exceptions ([summary](https://en.wikipedia.org/wiki/Joffe_v._Google,_Inc.)); that binds the Ninth Circuit only. Exempt radio includes public broadcasts, distress calls, amateur, citizens band, family radio and readily accessible public-safety systems; pager text, cellular and private data are not exempt. State interception laws also apply; the state statute was not read (unverified). Capturing that a device exists is lower risk than capturing contents, but advertisements can identify a person's device.

**Computer access (CFAA).** Federal law covers access to a "protected computer" without authorization and commands that damage one ([18 U.S.C. 1030](https://www.law.cornell.edu/uscode/text/18/1030)), which in practice reaches any internet-connected device. Scan only what is on the owned list, and stay off the utility's side of any meter and every neighbour's device.

**Vehicles.** The DMCA section 1201 exemptions for repairing your own vehicle are limited, and emission-control changes fall under separate law (the 2024 text was not read, unverified). Emission changes are class 4.

**Defaults that follow**

1. Filter at acquisition. Wi-Fi surveys keep management and beacon frames only; on the owner's own network, capture only owned addresses; drop data-frame payloads from devices not on the list (Kismet data-frame logging off, small snap length for captures).
2. Keep metadata on unknown devices minimal: count, vendor prefix, signal range, first and last seen, a per-site salted address hash. No names or message bodies in cards.
3. Pager, cellular and voice decoding is off; the pager decoder is not in the image.
4. Keep audio as features or a short window around a non-speech trigger; the recording light stays hard-wired (proposed, not yet a repo rule).
5. Any transmit or inject operation is class 2 at least, with an owned-target check and a power and duration ceiling. Deauth or injection against an owned test access point is class 3, and the panel shows target, action and stop condition.
6. Tests that could drop the site's link run on a dedicated test access point and client, never the live network.

## 9. The adapter manifest, in summary

One TOML file per adapter, pinned by hash inside the core release. Nothing is read from a module or from the agent. Schema name: `sawbones.adapter/1`. It is a proposal; the follow-up [#255](https://github.com/rafe-walker/sawbones-labs/issues/255) builds and tests it.

- `[adapter]`: schema, id, family, and the vocabulary names it may use.
- `[tool]`, `[tool.binary]`, `[tool.licence]`: exact version, source (apt package or CI-built static binary), SHA-256 per architecture checked at every launch, SPDX licence, process linking, copyleft flag (checked in CI).
- `[[operation]]`: id, name, class floor (tighten-only), and flags for transmits, active, writes a device, repeatable. Transmit or device-write forces at least class 2.
- `[operation.argv]` and `.inputs`: a fixed token template with typed slots (closed choice, fact reference or bounded number); no shell, no free text.
- `[operation.limits]`, `.output`, `.scope`, `.effect`: maximum duration, bytes, rate and quota; parser id (code compiled into the core) with row and text caps and the quantities it may produce; what scope it requires (enforced by C.O.R.T.E.X. only); the post-job contract, for example "transmit counter unchanged".
- `[sandbox]`: a profile from a closed set, network setting, device classes, one writable directory, seccomp digest, cgroup caps.
- `[legal]` (ids into a docs file, never shown to the agent) and `[conformance]` (fixtures, malformed corpus, differential oracle, simulated source, fuzz target; all required for admission).

Two sourcing tiers: pinned apt packages for big, complex tools (ffmpeg, tesseract, exiftool, sleuthkit), and CI-built static binaries with a recorded hash for small, high-risk ones (the sub-GHz sensor tool, sigrok if used). Automatic upgrades are off. The conformance suite gets an adapter subject that reuses the plugin host's admission, wire and sandbox patterns. Adapter changes that touch a manifest, a sandbox profile or a hazard class need the clean-room reviewer, and the owner's yes when a class or permission grows.

## 10. Open conflicts

These are real disagreements between the issue, the project document and the research. They need the owner's call (the one-line questions are at the end of the pull request).

1. **Active scans: class 1 or 2?** [#250](https://github.com/rafe-walker/sawbones-labs/issues/250) says anything that scans actively is armed or keyed; the approvals table puts scanning an owned device in class 1. The research suggests class 1 only for plain host and port discovery on owned addresses at a forced rate with no exploit scripts, and class 2 for other active work (Wi-Fi probe requests included; one report said class 1). This plan uses class 2 until decided.
2. **Parser-derived numbers and the taint latch.** Letting counts stay clean would cut latch fatigue but needs a threat-model change. Default: they trip it.
3. **Class 2 Modbus writes versus the guard's lease.** Class 2 is covered by the arm switch, but the guard passes a write only under a lease that follows a panel press. Part of [#261](https://github.com/rafe-walker/sawbones-labs/issues/261).
4. **Adapter parsers:** first-party Rust in a sandbox (this plan), or widen the plugin world to carry bounded text.
5. **Transmit radio on a power-gated hub port:** depends on hub wiring; no decision.
6. **Capability names versus the golden tool list:** safe only from the compiled vocabulary; the MCP design lead should confirm.
7. **`bettercap`:** one report said class 3 for deauth, another class 4. This plan does not wrap it and uses class 4.
8. **The guard covers RS485 only.** No write is offered on Modbus/TCP, CAN, K-line, I2C, SPI or USB flashing until a guard exists ([#259](https://github.com/rafe-walker/sawbones-labs/issues/259), [#260](https://github.com/rafe-walker/sawbones-labs/issues/260)).
9. **Single-master collisions** when Solar Assistant already polls ([#261](https://github.com/rafe-walker/sawbones-labs/issues/261)).
10. **Hardware:** whether the chosen Wi-Fi adapter can inject (its driver family has a warning from hcxdumptool's author; passive capture needs only monitor mode); whether a TI CC26xx board is wanted, since Sniffle does not support the nRF52840 dongle on the list; which neighbours' devices are in range, which sets how hard to minimise metadata.
11. **Licences** marked unverified need checking at the pinned version, and any port of GPL decoder code needs a legal read first.

## 11. Follow-up issues

| Issue | What |
| --- | --- |
| [#253](https://github.com/rafe-walker/sawbones-labs/issues/253) | Sandboxed tool launcher: systemd template units and a seccomp launcher |
| [#254](https://github.com/rafe-walker/sawbones-labs/issues/254) | Parser sandbox for adapter output |
| [#255](https://github.com/rafe-walker/sawbones-labs/issues/255) | Adapter manifest admission and hash pinning |
| [#256](https://github.com/rafe-walker/sawbones-labs/issues/256) | C.O.R.T.E.X. scope enforcement for capabilities |
| [#257](https://github.com/rafe-walker/sawbones-labs/issues/257) | Effect contracts and the passive-proof transmit-counter check |
| [#258](https://github.com/rafe-walker/sawbones-labs/issues/258) | First adapters, in order: ffmpeg audio, rtl_433, Kismet API, tesseract, exiftool, sleuthkit, sigrok-cli, tshark and dumpcap capture summariser |
| [#259](https://github.com/rafe-walker/sawbones-labs/issues/259) | CAN guard before any CAN write |
| [#260](https://github.com/rafe-walker/sawbones-labs/issues/260) | Modbus/TCP guard |
| [#261](https://github.com/rafe-walker/sawbones-labs/issues/261) | Bus collision handling when Solar Assistant already polls the RS485 bus |
| [#262](https://github.com/rafe-walker/sawbones-labs/issues/262) | Licence and SBOM checks for wrapped tools |
| [#263](https://github.com/rafe-walker/sawbones-labs/issues/263) | Pi 5 resource measurements and a cgroup budget that protects the P.U.L.S.E. heartbeat |
| [#264](https://github.com/rafe-walker/sawbones-labs/issues/264) | Compiled capability vocabulary, capability rows and the job handle |

No existing open issue duplicated these. They sit beside [#177](https://github.com/rafe-walker/sawbones-labs/issues/177) (native capture adapters), [#173](https://github.com/rafe-walker/sawbones-labs/issues/173) (plugin host), [#204](https://github.com/rafe-walker/sawbones-labs/issues/204) (golden tool definitions), [#248](https://github.com/rafe-walker/sawbones-labs/issues/248) (load shedding), [#65](https://github.com/rafe-walker/sawbones-labs/issues/65) (inventory of active assessment tools; this plan answers its tool-inventory part, though per-device fencing of the Bluetooth, Wi-Fi and network tools lands with the adapters) and [#103](https://github.com/rafe-walker/sawbones-labs/issues/103) and [#245](https://github.com/rafe-walker/sawbones-labs/pull/245) (the MCP surface).

## 12. What the research could not confirm

Nothing was run on a Pi 5, so footprints and timing are predictions. Kali package names are hints. Several release dates lack a year, and some project release pages were unreadable, so check packaged versions at install. Many licences are from memory ("u" in the catalog). No per-battery-model survey exists; it waits on the owner confirming the installed models.

## Sources

The three research reports behind this plan cite project pages, package pages, crates.io entries and the legal sources linked in section 8. Main project sources: [Kismet datasources](https://www.kismetwireless.net/docs/readme/datasources/), [rtl_433](https://github.com/merbanan/rtl_433), [hcxdumptool](https://github.com/ZerBea/hcxdumptool), [Universal Radio Hacker archive notice](https://github.com/jopohl/urh), [Sniffle releases](https://github.com/nccgroup/Sniffle/releases), [bubblewrap](https://github.com/containers/bubblewrap), [nsjail](https://github.com/google/nsjail), [Kali tools](https://www.kali.org/tools/), and the [GitHub-hosted runner reference](https://docs.github.com/en/actions/reference/runners/github-hosted-runners).
