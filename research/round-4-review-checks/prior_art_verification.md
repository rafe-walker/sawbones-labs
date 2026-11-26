# Prior-art verification: review round 1

Checked 25 November 2026. Every row below was opened at its primary page in this session unless marked otherwise. Pages were read through a fetch tool that summarises page text, so star counts, commit counts and fine wording are as reported on that date and should be re-read before being cited publicly. Nothing here is answered from memory.

Legend: Exists = the thing was found at a primary URL. Claim accurate = whether the reviewer's description matches what the source says.

## Summary table

| Item | Exists | Claim accurate | URL |
|---|---|---|---|
| A. Anthropic Model Hardware Standard (MHS) | Yes | Yes, with one nuance (calibration) | https://anthropic.com/news/model-hardware-standard-research-preview |
| B. OpenMHP | Yes | Yes (software only; unaffiliated with Anthropic) | https://pypi.org/project/openmhp/ |
| C. LAP, arXiv 2606.03755 | Yes | Mostly; firmware provenance is weaker than claimed | https://arxiv.org/abs/2606.03755 |
| D. agentic-hil | Yes | Yes | https://github.com/agentic-hil/agentic-hil |
| E. BugBuster | Yes | Yes | https://github.com/lollokara/BugBuster |
| F1. MCP I/O Gateway | Yes (prototype, not for sale) | Yes | https://mcp-io.ai/en/ |
| F2. AgentBee | Yes (pre-shipping) | Partly: it approves software actions, not instrument writes | https://setup.agentbee.co.uk/ |
| G1. ESP32MCPServer | Yes | Yes | https://github.com/navado/ESP32MCPServer |
| G2. Lager | Yes | Yes (MCP runs on the box) | https://github.com/lagerdata/lager |
| G3. Sipeed NanoKVM-Go | Yes | Yes | https://en.wiki.sipeed.com/hardware/en/kvm/NanoKVM_Go/mcp.html |
| G4. Espressif MCP SDK | Yes | Yes | https://components.espressif.com/components/espressif/mcp-c-sdk |
| H1. Home Assistant MCP server | Yes | Yes for existence; it is a hub-level server, not device-hosted | https://www.home-assistant.io/integrations/mcp_server |
| H2. ESPHome-MCP | Yes | Yes (host-side, via the dashboard) | https://glama.ai/mcp/servers/loryanstrant/ESPHome-MCP |
| I1. MemLineage, arXiv 2605.14421 | Yes | Partly: Merkle log and provenance gate, not "trust tiers" | https://arxiv.org/abs/2605.14421 |
| I2. Portable Agent Memory, arXiv 2605.11032 | Yes | Partly: Merkle-DAG and capability access, no trust tiers | https://arxiv.org/abs/2605.11032 |
| J. LoopHIL, arXiv 2610.24231 | Yes | Yes | https://arxiv.org/abs/2610.24231 |
| K1. Authority Inversion, arXiv 2605.23938 | Yes | Yes, but only small open models were tested | https://arxiv.org/abs/2605.23938 |
| K2. arXiv 2511.13225 | Yes | Partly: speech spectrograms, not RF | https://arxiv.org/abs/2511.13225 |
| L. oxrok | Yes | Yes | https://github.com/oxrok/oxrok |
| M. IETF drafts | Yes (three candidates found) | Unverified which two were meant | see section M |
| N1. NI LabVIEW / TestStand MCP "coming soon" | Yes | Yes | https://ni.com/aiextensions |
| N2. Topdon "live vehicle data is next phase" | Source exists | No: not stated as a roadmap item | https://www.topdon.us/blogs/news/topdon-usa-announces-topfix-ai-as-initial-phase-of-more-advanced-ai-solutions-in-development |

No reviewer citation turned out to be invented. Three were distorted (C firmware provenance, K2 subject matter, N2 Topdon) and two were over-labelled (I1, I2 "trust-tiered").

---

## A. Anthropic Model Hardware Standard

**Exists: yes. Date: 27 August 2026, confirmed on the Anthropic page.** Page metadata shows a later modification on 13 October 2026; the body carries no dated update note, so what changed is unknown.

Sources opened: the Anthropic announcement (read twice with different questions), the landing site modelhardwarestandard.com, The Register (28 August 2026), The Next Web (27 August 2026), and a third-party tracker page last updated 22 November 2026.

### What the announcement covers

- The post is titled as a preview of the Model Hardware Standard. It describes a shared specification for AI agents to operate physical lab and manufacturing equipment (microscopes, liquid handlers, robot arms and similar). It began as joint work with HHMI Janelia Research Campus.
- **Driver.** A software driver sits between the computer and the device and uses a small set of primitives, read and write (for example get or set a temperature). Reviewer claim confirmed.
- **Discovery.** Each device is made discoverable in a standard format so agents and devices can find each other across a network. Confirmed.
- **Reference file.** The driver turns natural-language tags into a reference file describing what a device measures, what can be adjusted and what safety limits will be enforced. The tags can be written by hand or gathered by an agent interviewing the user. Confirmed.
- **Access paths.** Three: MCP, a command-line interface, and code files (so chained commands run without the model reasoning at each step). It is described as model-agnostic. Confirmed.
- **Availability.** Research preview for a first group of labs and manufacturers, by application through modelhardwarestandard.com. The spec is not public; Anthropic says it will open-source it after more work on safety evaluations and best practices. No date is given. Confirmed.
- **Named parties.** Labs: Genentech, University of Washington labs, Carnegie Mellon, HHMI Janelia, QuEra, Tetsuwan Scientific. Vendors: AWS, Automata, Danaher, Doosan Robotics, MBF Bioscience, QIAGEN, Tecan, Universal Robots. Early adopters: Hugging Face (adding support in LeRobot) and **Raspberry Pi** (enabling integration across several products after testing a camera driver). Reviewer claim on Raspberry Pi confirmed.
- **Safety statements.** Safety limits listed in the reference file; a planned physical safety roadmap; more safety evaluations to be built with launch partners; a statement that the model's physical reasoning still needs expert oversight; a stated limit that hardware without a programmable interface is not yet supported.

### What it does not cover

Not mentioned anywhere on the announcement or landing site: hardware interlocks, leases or time-bounded authority, a physical arm or enable switch, emergency stop, human approval steps, firmware (version, update or provenance), receipts, audit logs.

**Calibration nuance.** The reviewer said calibration is not mentioned. The word does appear, but only as an example of a task (laser calibration on a quantum computer). There is no instrument-calibration feature, cross-check or calibration metadata described. The reviewer's point holds in substance.

**Enforcement detail is absent.** The page says the limits get enforced but not where (driver, device, or firmware), what happens on a breach, or how this interacts with hardware safety chains. The Next Web made the same observation and noted the EU Machinery Regulation, applying from 20 January 2027, may treat such limit files as safety functions; that is the outlet's analysis, not Anthropic's statement.

### Published since, and status as of November 2026

- **No spec, repository, SDK or official documentation was found.** The announcement links only to the application site. The landing site has an application form and a diagram, no spec, no docs, no driver list.
- A search of docs.claude.com and platform.claude.com for the standard returned nothing official.
- The Register described the protocol as "as-yet-unpublished" at launch.
- A third-party explainer updated 22 November 2026 reports no open-source date, no new partners since launch, and no published safety roadmap or further safety evaluations. This is secondary and should be treated as indicative only.
- A search for a published physical safety roadmap returned only Anthropic's unrelated Responsible Scaling Policy pages.
- **Status: still a gated research preview, spec unpublished, no release date.**

Unofficial material exists (third-party "driver guides" and the independent OpenMHP project in item B). None of it is the Anthropic spec.

---

## B. OpenMHP

**Exists: yes.** PyPI package `openmhp`, version 0.12.0 released 19 November 2026, Apache-2.0, repository at github.com/kushalsinha/openmhp (README read through the raw file; the GitHub page itself did not load).

- Describes itself as the reference implementation of an open hardware protocol (it expands the name as Open Model Hardware Protocol) for agents operating physical devices. It states it is "not affiliated with, endorsed by, or sponsored by Anthropic".
- A device server sits beside each instrument and checks limits, interlocks, approvals, leases and state before vendor code runs. Only the device owner can change a package's limits. Claim confirmed.
- Each device package holds a card, operating instructions, a machine-readable descriptor, references, scripts and a driver. The descriptor's fields are not listed in the README.
- Extends MCP with durable long-running jobs and pushed telemetry. Wraps OPC UA, ROS 2, SiLA 2, PyLabRobot, MQTT and others.
- Approval example: a person approves a specific plan by its digest at a gateway.

**Gaps versus the claim and versus Sawbones.** All enforcement is software in the device server. Lease timeouts and expiry behaviour are not described. No audit log, calibration or firmware handling is described. It says it complements hardwired e-stops and PLC safety logic and does not replace them. Ships 81 device packages each with a simulated twin; which have run on real hardware is not stated. Pre-1.0, breaking changes expected.

## C. LAP (Lab Agent Protocol), arXiv 2606.03755

**Exists: yes.** "LAP: An Agent-to-Instrument Protocol for Autonomous Science", submitted 2 June 2026, five authors, paper licence CC BY 4.0. Abstract and the first two thirds of the HTML were read.

- Four primitives: signed InstrumentCard, reservations, a safety-gating handshake with operator confirmation tokens, and typed measurement results carrying calibration and uncertainty.
- **InstrumentCard:** identity, instrument class, per-capability schemas with units and bounds, a safety class, physical limits, an interlocks list, a calibration reference, and a JWS signature.
- **Leases:** yes, the paper uses the word. Reservations are time-boxed, must be renewed, carry an expiry and a monotonic epoch, and lapsed leases are rejected with a defined error.
- **Interlocks and approval:** tasks with unsatisfied interlocks must be refused or aborted. Safety classes S0 to S3; S2 and S3 need a single-use operator token bound to the task, instrument and a hash of the parameters; S3 also needs an interlock attestation.
- **Unimplemented:** confirmed. The paper says it "has no normative or implemented status yet". No code or evaluation.

**Distortion.** "Firmware provenance" is overstated. Firmware appears only as a field in the provenance of a measurement result, not as a hash or provenance record on the card. The last third of the paper (including the calibration appendix and limitations) was not read.

## D. agentic-hil

**Exists: yes.** Apache-2.0, 18 stars, about 2,180 commits, Python package on PyPI, MCP server over stdio.

- Purpose: lets a coding agent develop firmware and test it on real boards (flash, reset, serial, CAN) through bounded MCP tools.
- Hardware actions are leased machine-wide and each action has a timeout. Lease duration is not stated on the page.
- The "interlock" is a configuration interlock: two dangerous options are interlocked so flashing is refused while either is on. It is not a physical interlock.
- Authoritative configuration lives outside the workspace, out of reach of the agent's file tools. Confirmed.
- SHA-256 audit chain. Confirmed.
- Backends: OpenOCD, pyOCD, STM32CubeProgrammer, esptool; worked example on an ST Nucleo board.

All four reviewer claims are accurate. Everything is host software; nothing is enforced in target-side hardware.

## E. BugBuster

**Exists: yes.** AGPL-3.0, 190 stars, about 1,265 commits. Open hardware bench instrument built around an ESP32-S3 mainboard with optional logic-analyzer (RP2040) and power-profiler (ESP32-P4 and C6) boards.

- README presents it as an instrument to hand to an AI, with about 150 MCP tools. Confirmed.
- All four microcontrollers update over the air from a GitHub release; the power profiler uses dual-slot A/B with rollback. Confirmed.
- Limits are enforced in the tool layer below the prompt: a default current ceiling, an explicit confirm flag above 12 V, and risk gates on mux and register access. Confirmed.

Not confirmed: whether the MCP server runs on the device or on a host. Limits appear to be tool-layer software, not a separate hardware rail. No leases, arm switch or independent receipts were described.

## F. MCP I/O Gateway and AgentBee

**MCP I/O Gateway: exists, at mcp-io.ai.** Small Wi-Fi boxes, each its own MCP server over Streamable HTTP, giving a model keyboard and mouse, camera, microphone, HDMI capture, GPIO and (in development) isolated industrial I/O.

- **Physical arm/disarm button: confirmed.** Short press toggles; when disarmed the box refuses all actions; a status LED shows state. Note it boots armed, the opposite of a fail-safe default.
- The site says relay outputs are never to be wired into safety chains.
- Status: prototype or in development, not orderable, seeking production partners. No maker named, no licence stated.

**AgentBee: exists, but not on this site.** The I/O Gateway page does not mention it. It is a separate project (agentbee.co.uk) by the founder of a security company, described in a June 2026 article as a small hardware key: a critical action is shown, a person approves or denies, and unplugging the key blocks the action. Ships an MCP package. At the time of the article it was on a waiting list. It targets software actions (deleting data, pushing code, moving money), not instrument outputs. An unrelated product with the same name exists at agentbee.net (education recruitment vetting).

## G. Device-hosted MCP servers

All four exist and all four genuinely host the MCP server on the device.

- **ESP32MCPServer:** MIT, 66 stars. MCP server runs on the ESP32 over Streamable HTTP and WebSocket. Reads I2C sensors, NMEA 0183, NMEA 2000 and OBD-II over CAN, with bus history buffers and OTA. Token auth exists but is off by default. Read-oriented.
- **Lager:** Apache-2.0, 6 stars. The Lager box runs an MCP server on port 8100 inside its own container. Controls power supplies, loads, scopes, debug probes, LabJack I/O, USB hubs, a robot arm. No software limits, locks or interlocks are described.
- **Sipeed NanoKVM-Go:** vendor wiki documents an MCP endpoint served from the device over HTTPS with an API key. A switch turns MCP off. Human confirmation is recommended to the user, not built in.
- **Espressif MCP SDK:** `espressif/mcp-c-sdk`, Apache-2.0, latest 2.2.0, published in Espressif's own registry. Lets an ESP32 host an MCP server over HTTP. Streamable HTTP and OAuth support are incomplete.

## H. Home Assistant and ESPHome

- **Home Assistant MCP Server integration:** exists, introduced in 2025.2. Exposes device control for exposed entities and, for administrators, a broader set that can read entities, logs and history and perform any action. OAuth 2.0 with PKCE. The page says nothing about ESPHome firmware. It runs on the hub, so it is not a device-hosted server.
- **ESPHome-MCP:** exists (loryanstrant, MIT). Tools for device version comparison, log streaming, validate, and install or update (which compile and flash over the air). Runs on a host and talks to the ESPHome dashboard. Claim confirmed, with the caveat that compile is not a standalone tool.
- **Newer and not cited by reviewers:** ESPHome 2026.10 (21 October 2026) added an optional MCP endpoint inside the bundled Device Builder that can edit configs, validate, compile and install. No approval gate is described. This is first-party and supersedes much of the third-party project.

## I. Agent memory papers

- **MemLineage (2605.14421):** exists, submitted 14 May 2026. Defence against memory poisoning. Each entry carries signed provenance in an RFC 6962 style Merkle log, plus a record of which retrieved entries influenced it. A gate blocks sensitive actions justified by externally sourced memory. Implemented with a test harness. "Hash-chained" is roughly right; "trust-tiered" is the reviewer's gloss. The paper distinguishes external from internal ancestry and applies a threshold, which is a two-level provenance gate, not graded evidence tiers.
- **Portable Agent Memory (2605.11032):** exists, submitted 10 May 2026, single author. Protocol for moving agent memory between vendors, with content-addressed entries in a Merkle-DAG and capability-based access. Python SDK reported, Apache-2.0. No trust tiers. No repository link on the arXiv page.

Neither paper connects memory trust to physical actuation.

## J. LoopHIL, arXiv 2610.24231

**Exists: yes.** Submitted 20 October 2026. An agent controls a multi-device RF bench and checks results with separate instruments.

- Hardware: two USRP B210s (one transmits, one acts as receiver and spectrum analyser), an MSP430 backscatter tag, a TI CC1310 receiver, a Saleae logic analyser, a Joulescope energy meter.
- Verification ladder that halts at the first failure and attributes faults to layers. Confirmed.
- Each run logs code versions and hashes of flashed images. Confirmed (hash algorithm not named).
- Code-enforced limits on transmit power, frequency, duty cycle and sideband emissions, computed before a command reaches a device. No numeric caps given.
- Reported result: completion rose from 54% to 79% overall with hardware in the loop. Small models sometimes reported unverified claims.
- Does not use MCP. No physical interlock, lease or human approval; it runs without a person in the loop. No code repository given.

This is the closest implemented precedent for "an independent instrument confirms what the agent says happened".

## K. Model-behaviour findings

- **Authority Inversion (2605.23938):** exists, submitted 28 April 2026. Reports that when a user's statement conflicts with numeric sensor data, models side with the statement, with near-zero trust in the sensor on numeric tasks, regardless of model size within the tested range. **Limit on the claim:** four open models between 4B and 35B parameters, 576 conflict cases, activity-recognition style datasets. It says nothing about frontier models.
- **arXiv 2511.13225:** exists, submitted 17 November 2025, titled "Seeing isn't Hearing". Vision-language models rarely beat chance at a multiple-choice task of reading spectrograms. **Distortion:** these are speech spectrograms and waveforms of spoken English words, read as a phonetician would. It is not a study of RF spectrograms or waterfall displays. It supports caution, not a direct finding about SDR plots.

## L. oxrok

**Exists: yes.** github.com/oxrok/oxrok, GPL-3.0-or-later, 2 stars, about 1,214 commits, on docs.rs.

- An experimental Rust port of the sigrok stack (drivers, protocol decoders, CLI-compatible front end). Not part of the sigrok project; the C code is used only as a test oracle.
- `oxrok mcp` serves MCP over stdio. Confirmed.
- The translation was done by a language model under human direction, and the README is frank about it: "Early, and most of it has never touched the device it drives." Only four drivers are listed as run on real hardware; the rest are tested against a Linux-only emulator.

Licence claim confirmed. Any linking of this code into a non-GPL design needs a licence check.

## M. IETF drafts

Reviewers named none. Three individual Internet-Drafts match the description. None has IETF standing.

1. **draft-sunyi-hacp-protocol-00**, "HACP: A Capability-Contract Protocol for AI Agents and Edge Hardware", 1 May 2026. **Expired 2 November 2026.** JSON-RPC protocol for agents operating GPIO, I2C, UART and sensors through a host server that owns the hardware, enforces policy and writes an audit trail. Tools carry a risk level 0 to 3, with a per-session risk cap and per-tool timeouts. No leases, no human approval workflow, no interlocks. Claims a C reference implementation with an MCP bridge (self-reported).
2. **draft-jimenez-t2trg-iot-agent-01**, "Agentic AI Operation of Constrained RESTful Environments", 3 July 2026, expires 4 January 2027, Thing-to-Thing Research Group. Three actuator tiers (autonomous, confirmation required, restricted); devices should enforce their own safety bounds independently of the agent; the write tool reads back and reports whether the observed value matches. No leases.
3. **draft-das-agentic-effectuation-boundary-00**, 19 September 2026, expires 23 March 2027. Threat model and "execution finality": an act stays non-effective until a protected enforcement point checks binding, freshness, policy, replay and provenance. Time-bounded handles with expiry. Mentions hardware-bound capabilities and device-side sinks as options. The same author has several sibling drafts that were not opened.

Which two the reviewers meant cannot be determined. HACP and the Jimenez draft are the best fit for "hardware" and "actuation".

## N. Vendor moves

- **NI:** confirmed. The NI AI Extensions page lists MCP servers. Device Drivers MCP is available now (400+ devices). LabVIEW MCP and TestStand MCP are both marked coming soon. SystemLink MCP is not marked that way. No dates. Separately, a community member has released an unofficial TestStand MCP beta on the NI forum; that is not an NI product.
- **Topdon:** **not supported as stated.** The 5 March 2026 press release calls TopFix AI the first phase of a larger AI strategy and does not name a next phase. An executive quote in it gestures at real-time data and predictive insight as a direction. A 23 September 2026 trade article describes the roadmap as personalised diagnosis, an on-device model for offline use, and subscription software, and does not name live vehicle data as the next phase. A German trade interview from June 2026 surfaced in search but was not opened.

---

## Overlap answer

### Parts with real, implemented overlap

- **Agent operates bench instruments over MCP.** BugBuster, Lager, agentic-hil, oxrok and NI's driver server all do this today. This part of the design is not novel.
- **MCP served from the device itself.** ESP32MCPServer, Espressif's SDK, NanoKVM-Go, Lager and the I/O Gateway prototype. Established pattern.
- **Software limits below the prompt.** BugBuster (tool layer), OpenMHP (device server), LoopHIL (pre-command checks), MHS (limits in the reference file). Established.
- **Per-device descriptor that states capabilities and limits.** MHS reference file, OpenMHP descriptor, LAP InstrumentCard, HACP tool list. Crowded, and MHS will likely set the convention once published.
- **Software leases.** agentic-hil (implemented, host-side), OpenMHP (implemented, server-side), LAP (specified with renewal, expiry and epochs).
- **Operator config the agent cannot edit, and a hash-chained log.** agentic-hil implements both.
- **Human approval bound to a specific action.** LAP tokens (spec), OpenMHP plan-digest approval, AgentBee hardware key, Jimenez draft tiers.
- **A physical enable button on an agent-facing device.** The I/O Gateway prototype has one per box.
- **Firmware update with rollback, and firmware hashes in run records.** BugBuster and LoopHIL respectively.
- **Checking the agent's claim against a separate instrument.** LoopHIL does this in practice with a second radio, a logic analyser and an energy meter.
- **Tamper-evident agent memory with provenance gating of sensitive actions.** MemLineage (implemented), Portable Agent Memory.

### Parts with no implemented prior art found

- **A physical arm rail wired to the modules.** The only physical arm found is a per-box button on a prototype that toggles the box's own software state and boots armed. Nothing found has a shared hardware enable line that removes actuation capability from several modules at once, independent of their software.
- **A firmware lease that fails safe.** Every lease found lives in host or server software. None was found where the instrument's own firmware holds the lease and drops outputs to a safe state when it lapses or contact is lost. Generic watchdog and dead-man timers are standard embedded practice, and a few open issues on small projects ask for dead-man relays, but none tie this to an agent lease. Those issues were seen in search results only and not opened.
- **A receipt from an independent sensor.** LoopHIL verifies with separate instruments and the Jimenez draft reads back from the same device. Nothing found issues a recorded, per-action receipt from a sensor outside the commanded path as a condition of calling the action done. ADF-EA specifies evidence requirements per action but was evaluated in simulation.
- **On-board calibration cross-check gating the agent's trust.** LAP puts a calibration reference and validity date on results, as a spec. Nothing found has instruments checked against an on-board reference with the outcome limiting what the agent may rely on.
- **Evidence-tier gating of physical writes.** MemLineage gates sensitive actions on memory provenance, in software agents only. Nothing found links the evidence grade of a stored fact to permission for a physical output.
- **Isolation or ground-domain map checked before connections.** See below. Nothing found.

### Specific searches requested

**Ground loop or isolation-domain checking by an instrument server or test framework.** Searches across labgrid, OpenHTF, PyMeasure and test-executive terms returned no framework or server that models ground or isolation domains and checks a planned connection against them. What exists is adjacent:

- Tektronix patent US 9,194,888 (priority 2012) and its family: an instrument automatically determines whether a probe has a sound ground connection to the device under test, by several electrical methods. This is single-probe ground continuity, not a domain map.
- Vendor application notes on floating measurements and isolated-input scopes: guidance for people, no automation.
- A hand checklist for identifying voltage, grounding and isolation domains before interfacing equipment: manual only.

Conclusion: an automated ground-domain map checked before connection appears to be unclaimed in open tooling. Patent coverage was not searched beyond the one family above.

**Permanent identified test ports ("site ports").** The general idea has long-standing industrial precedent, so it should not be presented as new in itself:

- SAE J1298, hydraulic systems diagnostic port sizes and locations for self-propelled work machines, first issued 1980 with a 2017 version listed.
- ISO 8925:1989 appeared in results as a related diagnostic-port standard; its page was not opened.
- Grace permanent electrical safety devices: permanently mounted, labelled test points through an enclosure door for meter measurements without opening the panel.

No source found uses the term "site port", and none found binds a permanent port's identity to a machine-readable map that an agent consults. The novelty, if any, is in that binding, not in the port.

### Reading for the design

The instrument-access layer, descriptor, software limits, software leases and approval tokens are all occupied ground, and the unpublished Anthropic spec sits squarely on the descriptor and driver layer. The physical safety layer and the trust-gating ideas are where no implemented prior art was found. Because the Anthropic spec is unpublished and its safety roadmap is pending, it could move into limits enforcement and safety evaluation at any time; nothing published suggests it covers a hardware arm rail, firmware leases, receipts or calibration.

---

## Could not verify

- **MHS spec contents.** The specification is unpublished; everything in section A comes from the announcement, the landing page and press. What changed in the 13 October 2026 page modification is unknown.
- **MHS status after launch.** Rests on one third-party page updated 22 November 2026 plus the absence of search hits. No official post-launch statement was found.
- **Raspberry Pi's own statement.** Confirmed on Anthropic's page only. A search for a Raspberry Pi announcement returned nothing relevant.
- **OpenMHP on GitHub.** The repository page failed to load and the API was not available; details come from PyPI and the raw README. Stars, commit history and real-hardware validation are unverified.
- **LAP, last third of the paper.** Calibration appendix and limitations section not read.
- **BugBuster MCP location.** Whether the server runs on the device or a host was not established.
- **GitHub activity dates.** Last-commit dates did not render for agentic-hil, BugBuster, ESP32MCPServer or oxrok.
- **MemLineage and Portable Agent Memory code.** No repository link on either arXiv page.
- **IETF drafts.** Reviewers did not name theirs; the three listed are candidates. Sibling drafts by the third author were not opened.
- **Topdon.** No source found states live vehicle data as the next AI phase. The June 2026 German interview was not opened.
- **NI community TestStand MCP.** One forum page failed to load; the mirror did not show a date.
- **AgentBee.** Shipping status, price and licence not stated on the setup page.
- **Dead-man relay issues on small GitHub projects and ISO 8925.** Seen in search results, not opened.
- **OBD-II diagnostic connector standards** as a further site-port precedent: not checked in this session.
