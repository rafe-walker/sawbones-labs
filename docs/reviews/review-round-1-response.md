# Review round 1: verdicts, changes and phases

25 November 2026. Response to four outside documents: an adversarial design review, an internal design review with a novelty assessment, a multi-review synthesis, and a "unified master specification v2.0" compiled from them.

All four reviewed the first version of the design, before the owner's second round of decisions. Some of their points were already resolved by then and are marked as such.

## Bottom line

The reviews are right about the shape of the problem and the size of phase 1. The principles survive. The first build shrinks to a **Bench Core**: a brain, a trust root, a ground-truth reference, two unlike modules and one real investigation, all read-only against the EG4 system.

I accept about three quarters of the points outright. I push back on eight, mostly where a reviewer's safety rule collides with a decision the owner made on purpose, or where the master specification promoted an untested idea into a requirement.

## Verdicts by theme

Accept = adopted as written. Change = adopted in a different form. Dispute = not adopted, reason given. Done = already resolved before the review arrived.

### Safety and authority

| Point | Verdict | What changes |
| --- | --- | --- |
| One global arm switch is not a safety architecture | Accept | The arm switch becomes one input. Added: an emergency stop that works with the Pi off, outputs off on boot, reset, brownout and crash, a watchdog outside the Pi, feedback that an output really de-energised, a latched fault that needs a human reset. |
| Every action needs its own human approval | Dispute | Collides with the owner's decision that approvals match danger. Kept the five hazard classes. Per-action approval on the pack applies to class 3 only. See "The rail is the permission" below. |
| A trusted panel: approve what the supervisor shows, not what the model says | Accept | A small display and button on the interlock controller. Used for class 3 approvals and for operator attestation. It moves from a nicety to part of the safety design. |
| The interlock controller is a trust root | Accept | It takes new firmware only with a physical jumper fitted. The agent can never reflash it. Without this the status lights could be made to lie. |
| "Measured" is too coarse a gate for a write | Accept | A fact carries its scope: exact device, firmware revision, conditions, and what invalidates it. Preflight checks the scope, not just the tier. "Confidence decays with time" is replaced by named invalidation conditions. |
| The agent must never reflash an instrument without approval | Change | A reflash is a trust reset, not an approval event. See below. Modules that can actuate, and anything without a rollback path, do need approval. |
| Manifest claims are not privileges | Accept | The module says what it can do. The host's own allowlist decides what it may do. A module appearing on USB grants nothing. |
| Secure-element identity for every module | Dispute | Out of proportion for one builder flashing his own boards. Host-side enrolment at first plug-in is enough until third-party modules exist. |
| Second-witness receipts | Accept | The reading that proves an action worked comes from a different module than the one that acted. |
| Text read from the world is a prompt-injection path | Accept | Network names, Bluetooth names, decoded packets, OCR and speech are typed as untrusted, length-capped, fenced in tool results, and can never raise a fact above "unverified". This was a real gap. |

### Connectivity

| Point | Verdict | What changes |
| --- | --- | --- |
| "No internet required" is false for a cloud model | Accept | Three named modes. Online: the pack joins the network the Mac is on, or tethers. Plan: the agent compiles a signed test plan while online and the pack runs it alone, with voice prompts. Recorder: triggers and ring buffers only. |
| The pack as a Wi-Fi access point by default | Accept | Last resort. It takes the Mac off the internet and transmits in a band the pack is meant to diagnose. |
| Tether over USB with the 5 V wire cut "so grounds stay isolated" (master spec) | Dispute | Cutting the power wire does not isolate anything. The ground wire and shield still join the Mac to the pack. The link needs a USB isolator, as the internal review said. |
| LoRa is not needed because Wi-Fi covers the lot | Done, premise wrong | Wi-Fi does not cover the lot. That was my error in the first draft, corrected by the owner. LoRa already moved to phase 2. The practical result is the same: plan mode is the main field mode and gets built first. |

### Power, isolation and electrical

| Point | Verdict | What changes |
| --- | --- | --- |
| Pi 5 limits USB current without a detected 5 A supply | Accept | Supply sized for 5 A and the limit lifted in configuration. Checked; see "Verified facts". |
| The "12 V" rail swings with battery voltage | Accept | A regulated stage feeds the hub. Not needed in the Bench Core, which runs from mains. |
| Power arithmetic is wrong (25 to 45 W for 4 h is 100 to 180 Wh) | Dispute, small | The document's 120 to 210 Wh already included conversion loss. It did not say so, which is the real fault. A mode-by-mode budget measured on the assembled unit replaces the single figure. |
| INA228 breakouts are 10 A parts; two 3 kW inverters draw far more | Accept | The DC meter module measures volts through an impedance-limited tap and amps through a rated external shunt or clamp. "Volts, amps, energy" on the bank was volts only as drawn. |
| Isolating I2C data lines does not isolate the sensor | Accept | The isolated side gets its own isolated supply. |
| A fused sense lead on a big bank may not interrupt a fault | Accept | A high-value resistor at the tap, so the lead cannot carry fault current. |
| Site ports: permanent, identified, impedance-limited test connectors | Accept | The best new idea in the set. The dangerous step, attaching to live conductors, is done once and properly. Phase 2, after the comms pod exists. |
| An isolation map checked before any connection | Accept | Every terminal declares its ground domain. The supervisor refuses a connection plan that makes a loop or bridges a barrier. Software only, so it starts in P0. |
| Heat: 45 C shade, a closed black case, LiFePO4 charge limits | Accept | Filtered forced air, charge refused above the cell limit. Deferred with the case and battery to a later phase. |
| The PV procedure implies a low current reading makes a string safe | Accept | Rewritten. Stages 0 to 3 are no-contact and belong to the agent. Past that the agent does not direct the work. A current check is an arc check, never proof that voltage is absent. |
| The $80 to $115 curve tracer | Accept | Not approved as specified. A tracer that can meet a string by mistake needs a full safety design. |

### USB, radio and the parts list

| Point | Verdict | What changes |
| --- | --- | --- |
| About 17 USB devices for 7 switched ports | Accept | A port budget table. The cut-down radio set removes most of the excess. |
| USB 3 radiates across 2.4 GHz | Accept | Checked. Intel measured an idle USB 3 link as noisy as a busy one. Nothing in the Bench Core needs USB 3, so the hub is a USB 2 hub. Both Plugable hubs are now discontinued; see "Verified facts" for the replacement. |
| "Pi 5 chosen for USB 3" | Accept | The real reasons are the on-board clock, the NVMe slot and the processor. |
| PiKVM V4 Mini cannot be the pack's hands | Accept | Checked. The Mini has no USB host port and no documented user GPIO header. KVM only, bench only, out of the pack. The document's "GPIO drivers for relays and servos" line was wrong for this model and is withdrawn. |
| No audio path, no storage, no 5 A supply, no cooler, no clock battery in the parts list | Accept | All added. The Pi 5 has no analog audio at all. |
| An analog capture module is the missing core instrument | Change | Agreed on the need. A single 4-channel, 24-bit, 192 kHz, DC-coupled, isolated part is unlikely at hobby cost. Proposed as two bands. See below. |
| Pico is micro-USB and too long; RP2040 has no second firmware slot | Accept | USB-C RP2350 boards, whose boot ROM supports A/B slots, so rollback is real. |
| One SDR, one ESP32-C6, one ESP32-C5, no CC1101 | Accept | The raw-sample firmware stays an experiment. The C6 earns its place with ordinary sniffer firmware. All radio moves out of the Bench Core. |
| The recording light cannot be truthful as wired | Accept | Camera and microphone power go behind a load switch and the light hangs on the switched side. |
| "One microsecond-class time base for every stream" | Accept | Withdrawn. Each stream reports its own timestamp uncertainty. GNSS gives absolute time. The flash-click-edge event does the real alignment. |
| USB start-of-frame as a shared module clock, under 20 microseconds (master spec) | Dispute | The internal review offered this as an untested idea. The master spec made it a requirement and an exit gate. It goes back to being an experiment. |
| The interlock drops the rail in under 10 ms of a Pi hang (master spec) | Dispute | A number with no hazard behind it. The detection time is set per hazard class from what the load can do in that time. |
| RF "transmit is out of scope" contradicts Wi-Fi and LoRa | Done | Already rewritten by the owner's decision: communications radios are allowed, active tests on owned gear are allowed when armed, jamming never. Added from the review: an allowlist of bands, power and payloads for each. |
| Do not use the one-line legal summary as policy | Accept | Softened. It is orientation, not legal advice. |

### Module standard

| Point | Verdict | What changes |
| --- | --- | --- |
| USB-C alone cannot carry the arm line, the calibration bus or a sync pulse | Accept | A small spine beside USB-C: armed rail, calibration pair, sync. |
| Lock nothing until three unlike modules exist | Accept | The 42 mm grid and the spine stay provisional. The standard is written down after the third module. This matches the owner's "subject to change". |
| The master spec fixes a 42 mm tray and a pogo spine now | Dispute | It contradicts the reviews it compiles. The first spine is a four-wire harness. |
| Conformance levels and governance in phase 1 | Accept | Deferred until a second builder exists. |
| One `invoke` tool hides risk from the client's permission prompt | Accept | Tools are named by hazard class. See below. |
| A discovery refresh rule after hot-plug, reset or reflash | Accept | Added. |

### Knowledge store and evidence

| Point | Verdict | What changes |
| --- | --- | --- |
| Run kronicle-lite first, port to Rust later | Dispute | The owner decided on a ground-up Rust store, and the reviewers were objecting to an older plan (a byte-compatible port before the first measurement). Their real point stands: do not build a database before there is data. Store v0 is the hash chain and the blob folder and nothing else. It grows by need. |
| A chain that can forget | Accept | The chain holds a salted commitment. The value and its salt live in a deletable store with per-artifact keys. Delete both and the chain still verifies. |
| Fork detection, checkpoints, key rotation | Accept, later | Needed when a second node syncs. One node for now. Canonical encoding is specified in P0. |
| Full-text search and exact lookups, no vector model on the Pi | Accept | Vectors return only if a retrieval benchmark shows the need. |
| Self-test is not calibration | Accept | The words change: "self-test passed", not "calibrated". One reputable reference meter is bought. Measurement classes A to D say what each reading may be used for. |
| A shared reference can make every instrument agree while all are wrong | Accept | Two independent references: the on-board one and the bought meter. |
| An experiment object: hypothesis, predictions, abort conditions, falsification | Accept | It is also the plan-mode file format, so it is built once. Our own research found forced predict-then-test on every step does not pay, so it is the unit of an investigation, not a ritual per reading. |
| Fusion claims are too strong | Accept | Results are ranked hypotheses with supporting, contradicting and missing evidence, and "what would prove this wrong". |
| Numbers for claims, plots for orientation | Accept | A diagnosis cites a computed value. Anything read off an image is "agent-inferred". |
| Build a ten-view local dashboard | Change | Borrow one. Rerun is the pick: open source under Apache-2.0 and MIT, it opens our MCAP recordings directly, has a Rust SDK, serves a web viewer from the Pi, and can be told to jump to a time range. Lichtblick is the fallback. We build only the small page no viewer has: the hypothesis matrix and approval state. |
| A diagnosis benchmark with planted faults | Accept | Starts in the simulator in P0. A change to the evidence format has to hold or raise the score. |
| Data lifecycle: gap events, disk-full behaviour, a data budget per stream | Accept | A capture never loses samples silently. |
| Emission ledger, self-noise fingerprint, quiet mode | Accept | With the radio work, after the Bench Core. |
| Fault-injector interposer | Accept | Phase 2. It is the missing hop for write-safety fault tests on serial links. |

### Scope and order

| Point | Verdict | What changes |
| --- | --- | --- |
| Phase 1 is five projects; ship a Bench Core first | Accept | Phases below. |
| The first value needs no hardware | Accept | It becomes P0. |
| Buy a proven K-line adapter and mic array instead of building | Dispute, deferred | The owner chose to build both, and the internal review agrees on K-line. Neither is in the Bench Core, so nothing is decided now. New standing rule from the owner's own SDR choice: one bought reference per domain, to score the built one against. |
| The thrust stand as the first Interlock demonstration | Change | It is the right first real customer. It is the wrong first test. A new safety layer is proved on a lamp and a fault injector, then on a motor with no propeller, then on thrust. |
| Approve about $300 to $450 of demonstrator hardware | Accept | See the Bench Core list. |
| The master spec's $900 "complete" total | Dispute | Every line is an unpriced estimate presented as a total. The first document at least marked its estimates. |
| The novelty claims are overstated | Accept | Every citation the reviewers gave exists. See "Where this sits". |

## Where I push back, and why

**1. Approving everything.** The reviewers' tier 2 puts "changing a sensor range" behind a button press. The owner ruled the other way on purpose: hazards get approved, readings and small reversible things do not. Both can be true if the limit is physical. See the next section.

**2. Approving every reflash.** The original brief was an agent that can diagnose and fix its own instruments. The reviewers' worry is real: a bad reflash could leave a sensor that reports plausible lies. Our own principle already answers it. A firmware change invalidates every fact that module produced. The module drops to "unverified" and has to pass the calibration cross-check against an independent instrument before anything trusts it again. Sensing-only modules with a rollback slot can be reflashed by the agent. Modules on the armed rail need approval. The interlock controller needs a jumper.

**3. The thrust stand first.** You test the brake before the hill.

**4. The master specification.** It reads as settled and is the least reliable of the four. It turned an idea its own source called untested into an exit gate, claimed a cut wire gives isolation, and put a price on things nobody has priced. I used it as a list of proposals, not as a baseline.

## New in this round

These are mine. None is prototyped.

**The rail is the permission.** Each hazard class gets a power rail whose limit is set by hardware.

| Class | Rail | What bounds it | Who says yes |
| --- | --- | --- | --- |
| 0 and 1 | None. Sensing modules have logic power only | Nothing to bound | Nobody |
| 2 | Armed rail A: low voltage, with a hardware power cap in the region of 15 W | A current limiter on the interlock board | The arm switch, once per session |
| 3 | Rail B: higher energy, per target | A keyed enable, a contactor, a lease that expires | A button press on the trusted panel, per action |
| 4 | No rail exists | Not offered | Not applicable |

What the agent can do without asking is bounded by copper, not by policy. A class-2 module cannot exceed class-2 energy whatever its firmware believes.

**Listen-only is a wire, not a promise.** The comms pod's transmit driver is powered from the armed rail. Disarmed, it cannot drive the bus at all.

**Calibrate or measure, never both.** A module's input relay connects it either to the calibration pair or to the thing under test. A changeover contact makes the two exclusive, so the shared calibration pair can never join two ground domains through a module.

**The isolation map is a typed graph, and the Mac is on it.** Terminals are nodes, galvanic connections are edges, and a forbidden loop is a cycle across domains. A MacBook on its charger is tied to mains earth; on battery it floats. The agent can read which, and the check uses it.

**Two roads to the same inverter.** Solar Assistant is one path to the EG4's numbers. Our own listen-only tap is a second. Where they disagree, one of them is wrong, and finding out which is the first real cross-check the bench performs. It is also the first second witness.

**Tool names are hazard classes.** `read` and `invoke_safe` for classes 0 and 1, `invoke_armed` for class 2, `invoke_hazard` for class 3. Claude Code's permission rules work per tool name, so the client can allow the first two always and the prompts land only where the danger is.

**Analog capture in two bands.** A DC-coupled simultaneous converter for DC to a few kilohertz (cranking, power quality, vibration), and an audio-class 192 kHz converter for the band above (inverter switching noise). Two cheap parts in place of one rare one. Part choices are being researched.

**Store v0 is the chain.** An append-only hash-chained log and a content-addressed folder. No index until something needs to be found.

**The interlock is model-checked before it exists.** Its logic is a pure state machine in a Rust crate with no hardware in it. Every acceptance test the reviewers listed (replayed approval, expired lease, lost heartbeat, power-up state) runs on a laptop in P0.

**The aircraft module is a health check.** Aircraft transponders are a free, known signal. If the pack hears them, the 1090 MHz chain works.

## Where this sits

Anthropic's Model Hardware Standard is real. It was announced as a research preview on 27 August 2026 with HHMI Janelia. It defines a standard driver with read and write operations, device discovery, and a generated reference file of what a device measures, adjusts and limits. Agents reach it over MCP, a command line or code. It is not public yet. Raspberry Pi and Hugging Face are named early adopters. The announcement says nothing about hardware interlocks, leases, firmware, or a physical arm switch.

The reviewers' read is right. The sensing namespace is the least novel part of this design and the most work. The trust layer is the most novel and the smallest: the calibration cross-check, evidence-gated writes, and a physical interlock with independent receipts. The design should present itself as the physical and instrument-health layer that sits beside that standard and MWS, not as a rival platform. Claims to drop: that a device-hosted MCP server on a battery box is new, and that no safety standard exists for agents that actuate hardware.

A separate check opened every source the reviewers cited. None was invented. Three were stretched: the spectrogram study is about speech, not radio; the two "hash-chained memory" papers are provenance logs without trust tiers; and one vendor claim about live vehicle data is not supported by what the vendor published. As of late November the standard is still a gated preview with no public specification.

The same check looked for built prior art on the trust layer and found none for: an armed rail shared across modules, a lease held in instrument firmware that fails safe, a per-action receipt from an independent sensor, a calibration cross-check that gates the agent's trust, evidence-tier gating of physical writes, or an automated ground-domain map. Permanent identified test ports are old practice in other trades; binding a port's identity to an agent-readable map looks new.

## Phases

### P0: everything that needs no hardware

The brain is mostly software, and it can run on the Mac against simulated modules today. P0 is that brain, the firmware that will go on the first boards, and the scaffolding around both.

| Area | Work |
| --- | --- |
| Scaffolding | Cargo workspace, CI, licence gate, contribution rules |
| First data | Read-only Solar Assistant ingest, recorder, twin-input ratio monitor. Then run it on the real unit |
| Brain | Supervisor skeleton: module registry, out-of-process drivers, health, inventory states |
| Modules without hardware | The module protocol crate shared by host and firmware. Simulated modules, including faulty ones |
| Trust root | Interlock state machine as a pure crate, tested exhaustively. Hazard-class policy engine |
| Agent surface | MCP tools named by hazard class, read-only first. MWS result shapes. Untrusted-text type |
| Evidence | Session recorder with gap events, replay, evidence bundles, borrowed viewer |
| Knowledge | Store v0, canonical encoding, fact scope and invalidation, preflight |
| Safety in software | Isolation map and connection-plan checker |
| Field mode | Experiment plan format and an offline runner against the simulator |
| Proof | Diagnosis benchmark with planted faults |
| Firmware | RP2350 firmware core building in CI, A/B slot layout, logic tested on the host |
| Paper | Design document v2, threat model, acceptance checklist, Bench Core parts list at US stock |
| Hardware in software | The pack model, budgets, circuit and antenna simulation, interference tests, printed parts and wiring as code. See the next section |

### P0 also designs the hardware in software

Added at the owner's direction: before anything is laid out or bought, the hardware is described, simulated and tested as files in the repo, on free GitHub runners and on the Mac. P0 ends with a blueprint pack that CI can regenerate and re-check on every change.

**One model, everything generated from it.** A single machine-readable description of the pack: modules, ports, rails, ground domains, emitters, receivers, heat loads and board outlines. Each module's manifest is its entry. From that one file CI derives the port budget, the power budget by mode, the isolation map, the spectrum ledger, the wiring drawings, the tray and panel geometry, and the parts list with prices. Change one module and the whole pack is re-proved. The manifest stops being a description of the hardware and becomes its blueprint.

| Track | What is simulated or checked | What it can honestly tell us |
| --- | --- | --- |
| Budgets | Ports, current per port, power by mode with reserve, data per stream, cost | Hard pass or fail. This would have caught the 17-devices-on-7-ports error |
| Schematics | Interlock board, spine, DC meter, comms pod as schematic files with rule checks | Hard pass or fail on wiring errors |
| Circuits | Fault current through the protective resistor at a tap, the armed rail's current limit, front-end attenuator and filter, ground-loop error at the converter, converter ripple | Good, within component tolerance. Run with tolerances varied |
| Wiring | Harness drawings and wire lists generated from the model | Exact |
| Printed parts | Trays, bench plate and panel as code: fit, collisions between parts, printer bed size, a slicer run for time and filament | Exact for geometry. Strength is a rough guide |
| Firmware | Module and interlock firmware on mocked hardware, and in an emulator where the chip is supported | Good for logic, blind to timing and analog |
| Spectrum ledger | Every emitter against every receiver: converter harmonics, clocks, USB, Wi-Fi, Bluetooth, LoRa | Finds collisions on paper. Says nothing about how strong they are |
| Interference pipeline | Synthetic and public recordings with planted Wi-Fi bursts, Bluetooth hops, inverter combs and broadband noise, run through our detection code | Tests our software fully. Tests no hardware |
| Antennas | Simple wire antennas for 433, 915 and 1090 MHz | Good for match and pattern |
| Shielding, with and without | Aperture arithmetic first, full-wave runs for a short list of enclosure options | Ranks options. The numbers are not to be trusted until measured |
| Heat | A thermal model of the case at 45 C | Ranks options, with a wide error band |
| Sound | A simulated microphone array to test direction finding before one exists | Good for the algorithm, optimistic about the real room |

Two rules keep this honest.

1. **A simulation result is never "measured".** Model outputs enter the store as "agent-inferred" like anything else the agent worked out.
2. **Every prediction is written down before the hardware exists**, next to the one P1 measurement that will check it. The bench then measures its own shielding, heat and noise with its own instruments, and the models are corrected. A model that has been checked against the thing it describes is worth keeping. One that has not is a guess with a graph.

A plain plastic case shields nothing. The shielding question is really which internal metal can or foil lining to add and how much the fan and connector openings give back, and that is what the simulation ranks.

### P1, proposed: the Bench Core

Mains-powered, on the bench, read-only. No battery, no case, no radios, no cameras, no vehicle.

| Rung | Build | Verified by | Done when |
| --- | --- | --- | --- |
| 1 | The brain: Pi 5, cooler, clock battery, NVMe, proper supply, switched hub | Its own health telemetry | The P0 supervisor runs on it unchanged, power-cycles a port, and records Solar Assistant for four hours with no gaps |
| 2 | Build tools: debug probe and logic analyzer | The brain | The brain can flash, debug and watch the wires of every later board |
| 3 | Trust root: interlock controller, emergency stop, armed rail, panel display and button, jumper | The brain and the logic analyzer | Outputs stay off through boot, reset, brownout and a hung Pi. A replayed approval is refused. The stop works with the Pi off. Tested on a lamp |
| 4 | Ground truth: one bought reference meter and the on-board reference | Each other | The on-board reference is characterised against the meter and stored as the first measured facts |
| 5 | Module 1: DC meter | The reference | It agrees within a stated uncertainty and detects a disconnected, stale or out-of-range input. A reflash drops it to unverified until it re-earns trust |
| 6 | Module 2: isolated listen-only RS485 and RS232 pod | The logic analyzer and Solar Assistant | It reads the EG4 directly, and the two paths are compared |
| 7 | The investigation | Everything above | One real EG4 question answered end to end: a named experiment, a session, an evidence bundle, facts in the store, and what would prove it wrong |

Why the brain first: every later rung is flashed, powered, timed and recorded by it, and the P0 software is waiting for it. Moving that software from the Mac to the Pi is the first hardware test and the cheapest one.

Why this order: each rung is checked by the one before, so no instrument is ever trusted on its own word.

The third unlike module, analog capture, opens P2. The module standard is written down after it. The thrust stand, site port, fault injector, senses, radio, portability and vehicle follow.

## Verified facts

Checked against primary sources on 25 November 2026. Full notes are in [`research/round-4-review-checks/`](../../research/round-4-review-checks/).

| Claim | Result |
| --- | --- |
| Pi 5 limits USB to 600 mA total without a detected 5 A supply | Confirmed. 1.6 A with one. Two settings lift it when the Pi is powered another way |
| Pi 5 has no analog audio | Confirmed |
| Pi 5 gadget mode uses the USB-C power port | Confirmed. Powering from the header while a laptop is on that port is not something Raspberry Pi documents. Unverified |
| Pi 5 clock needs its own battery | Confirmed |
| Plugable USB 3 hub supports per-port switching and warns about 2.4 GHz | Confirmed, but the hub is now discontinued |
| Plugable USB 2 sibling | Exists, discontinued, and not on the per-port switching list |
| A USB 2 hub that does switch per port | Phidgets HUB0003, about $55, which also accepts 8 to 30 V directly |
| USB 3 noise needs an active data transfer | Refuted. An idle link is as noisy |
| PiKVM V4 Mini | No USB host port, no documented user GPIO, rated 0 to 50 C |
| RP2350 supports A/B firmware slots with rollback; RP2040 does not | Confirmed from the datasheets |
| Pico and Pico 2 | Micro-USB, 51 mm long. USB-C RP2350 boards in US stock from about $5 |
| INA228 breakouts | 85 V, 10 A peak on the fitted shunt, far less continuous. One of the two documents an external shunt |
| RTL-SDR V3 range | 500 kHz to 1766 MHz on paper. Below about 24 MHz it is a cruder direct-sampling mode |
| Cheap GNSS boards and the pulse pin | Partly. The common 4-pin board lacks it. Named boards that have it start at about $30 |
| LiFePO4 | 10.0 to 14.6 V for four cells. Charge limit is 45 C on some data sheets, 60 C on others |

Two results change the parts list. The hub named in the first design is discontinued, and its replacement runs straight from battery voltage, which removes the separate 12 V regulator the reviews asked for.

## Decisions this needs from the owner

| Question | My recommendation |
| --- | --- |
| Lay P1 into the project as proposed above? | Yes |
| Parts ship from US stock for the bench too, as you ruled for the drone? | Yes |
| Approvals: keep "arm means yes for the session" for class 2, and add a panel button for class 3 only? | Yes |
| Knowledge store: keep your Rust ground-up decision, starting as the bare chain, with no kronicle-lite stopgap? | Yes |
| PiKVMs stay on the fixed bench, none in the pack? | Yes |
| The thrust stand as Interlock's first real customer in P2, after a lamp and a bare motor? | Yes |
| Publish the four review documents in the repo alongside this response? | Your call |
