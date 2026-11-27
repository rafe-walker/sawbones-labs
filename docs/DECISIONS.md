# Decision log

Decisions made by the project owner, newest round last. The design document carries the reasoning; this file is the short record. Reversals are kept on purpose.

## 25 November 2026, first round

| Topic | Decision |
| --- | --- |
| Brain | Raspberry Pi 5 4 GB for the pack. The spare Home Assistant Yellow goes to the fixed bench and gets an NVMe drive. |
| Meters | Both in phase 1: a Bluetooth handheld and a built DC meter module. |
| Vehicle | Build our own K-line and CAN pod first, writing new firmware and protocol code as needed. |
| Mic array and radio | Build both, firmware in Rust wherever chip support is ready. One bought SDR stays as the reference. |
| Camera | An aimed camera first; hand-pointed is fine. A 360 camera waits. |
| Soldering | The agent coaches from iron telemetry and video, then proves joints electrically. |
| Knowledge store | A ground-up Rust rebuild on IsoKron's principles, designed for the agent first, with no duty to match kronicle-lite. Private and local. The EG4 inverter is the first device in it. |
| Solar read path | Solar Assistant's MQTT first. Native port drivers stay on the roadmap, and the EcoFlow Delta Pro gets a driver too. |
| Safety layer | Interlock is the physical-safety layer. It builds on existing open source if any fits, otherwise Rust from the ground up. |
| Module shape | A 42 mm grid and USB-C for now, open to change after more module research. |
| MWS Rust kit | Decide nearer 16 December 2026, when the MWS 0.9 comment window closes. |
| Standing rules | Every line gets a "build it cheaper and better" check. The build order is dogfooded: early tools build later tools. |

## 25 November 2026, second round

| Topic | Decision |
| --- | --- |
| Name | Sawbones is the working name. Still open to change. |
| LoRa pair | Moved from phase 1 to phase 2. Phase 1 stands at about $933 with the NVMe drive. |
| Security testing | Transmitting and active testing are allowed against owned gear on the owner's property. Jamming stays out. No exploit or malware code. |
| Rust rule | Rust where it makes sense. A tool that exists only in C or Python is used as it is. A port needs a reason and a test. |
| Approvals | Matched to danger in five hazard classes. Readings and touch-safe actions need no prompt. Hazardous or hard-to-undo actions need the owner's yes each time. |
| Trust rule | A hardware write rests only on facts that are measured or owner-attested. |
| 48 V bank | Touch-safe by voltage but not by energy. Anything that could short it is hazard class 3. |
| Repo | A public repo, `rafe-walker/sawbones-labs`, Apache-2.0. It can move to its own account later. |
| Roadmap | The phased build timeline is not laid out in the GitHub Project until three outside adversarial reviews have been worked through. (Done on the same day; P0 was then opened.) |

## 25 November 2026, after review round 1

| Topic | Decision |
| --- | --- |
| Review response | Four outside reviews worked through. Verdicts and changes are in [reviews/review-round-1-response.md](reviews/review-round-1-response.md). |
| P0 | A new first phase: all scaffolding, software and firmware that needs no hardware. |
| Hardware in software | P0 also designs, simulates and tests the hardware as files, on GitHub runners and on the Mac, and ends in a blueprint pack that can itself be re-checked and improved. |
| Bench Core | The first hardware is a much smaller Bench Core, dogfooded rung by rung, brain first. The seven-rung proposal awaits the owner's yes. |
| Custom code | Our own firmware and software wherever possible, in Rust where it makes sense. |
| IsoKron code-read note | Not published. What was needed from it is already in the design. |
| Parked idea | An overhead-aircraft module (ADS-B). |

## 26 November 2026

| Topic | Decision |
| --- | --- |
| P1 | Rungs 1 to 7 of the Bench Core are phase 1. |
| Buying | Amazon, from any seller. Watch the delivery estimate; nothing a month out. This replaces the US-stock question for the bench. |
| Merging | Claude merges and deploys once it has reviewed the change and all checks pass. |
| Reference meter | No meter bought for P1. The voltage reference module and the meter on hand cover the low-voltage Bench Core. A safety-rated meter waits for bank or mains work. |
| Cooling | Designed, simulated and tested in P0: aluminium as a heat spreader, and a fan switched by a temperature sensor. |
| Peak-heat charging | Not a design driver. The bench is rarely at peak ambient for long, and the pack can cool down and charge indoors. The thermal work targets running electronics, not charging at 45 C. |
| Before ordering | Every part is cross-checked against the parts it connects to and the build as a whole. The table is `docs/bench-core-cross-reference.md`; the checks behind it are in `research/round-5-parts-cross-check/`. |
| Work size | Agent work outside the cross-check is cut into small pieces, one crate or one check each, pushed early, so work can stop at any point without loss. |
| Solar Assistant run | The first run of the ingest on the real unit moves to 27 November. |
| Roadmap | P0 and P1 are cut into numbered stages, P0.1 to P0.8 and P1.1 to P1.7. Each stage is a milestone and ends in a check-in item: what the owner sees working and what he decides. No dates; order comes from what each stage needs first. See `docs/ROADMAP.md`. |

## 26 November 2026, design review round 2 (two of three reviews in)

Tracked in #166. The plugin and store index design itself is written up after the third review.

| Topic | Decision |
| --- | --- |
| Plugin reviews | Every agent-written plugin change gets a second, clean-room review before merge: a separate model that sees the diff, the spec and the test results, never the data that triggered the change. The owner's yes is needed only when a change touches permissions or hazard classes. This narrows the earlier merge rule for plugin changes. |
| Field fixtures | A recording captured in the field can only test that a decoder rejects bad input cleanly. What a frame means comes only from a second instrument or from the owner. |
| Split-key changes | One pull request may change a decoder or its safety tests, never both. CI enforces it. Changing the safety tests needs the owner's yes. |
| Device-reported tier | A new trust tier between measured and documented. Numbers a device reports about itself are evidence about itself; they can back a class 2 action only when one of the bench's own instruments agrees. |
| Too clean to be true | Each module manifest states its sensor's noise floor. A stream quieter than physics allows is flagged as stuck. |
| Panel shows what it approves | The trusted display shows the action and a short code for the state it was checked against. Any change voids the approval. |
| Index headline metric | How close a small model gets to a large one using the index. |
| Plugin logs and reports | Plugins emit structured logs and health counters the agent can read. The agent can spot problems in a plugin and file a report, so plugins and the product improve. See #166 for the shape. |

## 26 November 2026, review round 2, approvals and battery limits

| Topic | Decision |
| --- | --- |
| Approval scale | Approvals scale with risk. Repo work (code, docs, tests, simulation, CI, merges) needs no panel or phone approval; agent-written plugin changes still get the clean-room reviewer. Signed approvals (panel press, or a phone passkey for anything not involving energy) start when testing on the real Pi starts, and cover what runs on the device: installing core releases, plugin permission growth, native installs, and operator attestations. Nothing here may slow early development. |
| Battery protection limits | Class 3, not class 4: the agent may propose a change, and it happens only after the owner confirms it with a physical panel press for that one action. Charge voltages are also class 3. The review document said class 4; the repo's design document (class 3) was right. Class 4 stays: mains work and string switching. |
| Round 2 capture | No separate response document. Review round 2 changes are work items #167 to #180 in their roadmap stages, plus comments on the affected items; summary in #166. |

## 26 November 2026, the trust-root chip

| Topic | Decision |
| --- | --- |
| Trust-root microcontroller | Keep the RP2350. It costs about a dollar, the firmware already builds for it in CI with Rust, it has signed boot, OTP, TrustZone and glitch detectors, and it shares a toolchain with the RP2040 pods. The published breaks ([THREAT-MODEL.md](THREAT-MODEL.md) section 4a) all needed the board on the attacker's bench. A different general-purpose part (STM32, NXP LPC55, Nordic nRF) has the same class of published glitch attacks; a lockstep safety part (TI Hercules, Infineon AURIX) costs several times more and has weak Rust support. Neither is worth it now. |
| The external cut | Required before any real power switching. A separate watchdog part, about a dollar, with its own supply monitor, asks the RP2350 questions; the relay stays on only while the answers come back right and on time. A frozen, glitched or misbehaving RP2350 drops the rail by itself. It must work with any controller chosen later. Tracked in [#144](https://github.com/rafe-walker/sawbones-labs/issues/144). |
| Later add-ons | Not now. A secure element for module keys, if key secrecy is ever needed. A second controller from a different maker that must agree before an output turns on, decided in P1. |

## Reversals and corrections

| Was | Now | Why |
| --- | --- | --- |
| Hardware budget of $1,000 at most | $1,000 is the starting budget; the build is phased | Owner clarified. |
| Safety layer called LockSafe | Interlock | LockSafe was a misremembered name. |
| Knowledge store byte-compatible with kronicle-lite | Ground-up Rust rebuild, no compatibility duty | Owner wants the best store for this purpose. |
| Phase-1 budget with no multimeter | Two meters in phase 1 | An error in the first budget, caught by the owner. |
| "Wi-Fi already reaches the whole lot" | It does not; an added outdoor access point would | An error in the first Remote mode text. |
| LoRa pair in stage 1 | Phase 2 | Not needed yet; money is better spent elsewhere. |
| Receive-only tool fence, no transmit or active tools | Transmit and active testing on owned gear, behind the arm switch | Owner needs to audit his own devices. |
| Per-action approval for outputs | Five hazard classes matched to danger | The owner should approve hazards, not readings. |
| New GitHub account for the venture | Personal account `rafe-walker` | It is a personal project for now. |

## Open

| Question | Recommendation |
| --- | --- |
| Arm means yes for the session for class 2; a panel button for class 3 only? | Yes. |
| Keep the ground-up Rust store, starting as the bare chain, with no kronicle-lite stopgap? | Yes. |
| Thrust stand as Interlock's first real customer, after a lamp and a bare motor? | Yes. |
| Put the nRF52840 Bluetooth dongle back in, about $11? Now a later-phase question, since all radio left the Bench Core. | Yes, with the radio work. |

Facts only the owner can check: usable cell data at the far corners of the lot; which inverter is V1 and which is V2; panel labels and panels per string; how the EcoFlow array is wired against the 150 V input limit; the brand and model of the meter on hand.
