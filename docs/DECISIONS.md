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
| Roadmap | The phased build timeline is not laid out in the GitHub Project until three outside adversarial reviews have been worked through. |

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
| Put the nRF52840 Bluetooth dongle back into phase 1, about $11? | Yes. |

Facts only the owner can check: usable cell data at the far corners of the lot; which inverter is V1 and which is V2; panel labels and panels per string; how the EcoFlow array is wired against the 150 V input limit; the brand and model of the meter on hand.
