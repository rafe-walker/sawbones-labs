# Research notes

Raw notes from 21 research lanes run by Claude research agents on 25 November 2026, in three rounds. The design document in [`../docs/project-document.md`](../docs/project-document.md) is the synthesis; these are the working notes behind it, kept so no detail is lost.

Read them as research notes, not as verified fact. Each lane's confidence and main known gap is listed below. Prices marked estimate or unverified have no checked page behind them.

## Round 1: portable test bench

| Lane | File | Confidence | Main gap |
| --- | --- | --- | --- |
| Prior art | [prior_art.md](round-1-portable-test-bench/prior_art.md) | Medium | Keyword searches, not a full crawl of the MCP registries. Most READMEs unread. |
| RF array | [rf_array.md](round-1-portable-test-bench/rf_array.md) | Medium | tinySA prices are from December 2025. Antennas, RF switch and Wi-Fi adapter unpriced. |
| Compute, power, case | [compute_power_enclosure.md](round-1-portable-test-bench/compute_power_enclosure.md) | Medium | Battery, converters and isolators unpriced. The spare Yellow's module, header and 12 V tolerance unchecked. |
| Measurement instruments | [measurement_instruments.md](round-1-portable-test-bench/measurement_instruments.md) | Medium | About ten verified prices; Amazon could not be read. |
| OBD-II and vehicle bus | [vehicle_obd.md](round-1-portable-test-bench/vehicle_obd.md) | Medium | No field report for either adapter on a first-generation Tacoma. |
| Solar and BMS | [solar_bms_comms.md](round-1-portable-test-bench/solar_bms_comms.md) | Medium | Forum pages would not load. Inverter baud rate and pinout come from sibling hardware, not EG4. |
| Audio, vision, extra senses | [audio_vision_extra_sensors.md](round-1-portable-test-bench/audio_vision_extra_sensors.md) | Medium | Used Theta, base TC001 and borescope prices unverified. |
| MWS and Rust stack | [mws_rust_stack.md](round-1-portable-test-bench/mws_rust_stack.md) | High | rmcp's Tasks support unconfirmed. |
| Fixed bench | [fixed_bench_observability.md](round-1-portable-test-bench/fixed_bench_observability.md) | Medium | Jumperless stock, SO-101 fit on the A1 mini, no Claude-specific solder test. |
| Old vehicles beyond OBD | [old_vehicles_beyond_obd.md](round-1-portable-test-bench/old_vehicles_beyond_obd.md) | Low | Search budget ran out. 5VZ-FE specifics are unsourced. |
| Module standard and fusion | [module_standard_and_fusion.md](round-1-portable-test-bench/module_standard_and_fusion.md) | Medium | No search available; about 70 primary pages fetched. Minor standards judged from prior knowledge. |

## Round 2: ground-up builds and the data store

| Lane | File | Confidence | Main gap |
| --- | --- | --- | --- |
| Ground-up radio and the ESP32 unlock | [diy_rf.md](round-2-diy-builds-and-data-store/diy_rf.md) | Medium | Small-part prices unverified. No sensitivity figures published for ESP-SDR. |
| Cheap meters and built instruments | [diy_instruments_and_meters.md](round-2-diy-builds-and-data-store/diy_instruments_and_meters.md) | Medium | AliExpress prices only; US retail runs higher. Nothing bench-tested. |
| Ground-up builds for other modules | [diy_other_modules.md](round-2-diy-builds-and-data-store/diy_other_modules.md) | Medium | Marketplace prices are estimates for 8 of 13 lines. |
| Knowledge store prior art | [knowledge_store_prior_art.md](round-2-diy-builds-and-data-store/knowledge_store_prior_art.md) | Medium | Many candidates judged from background knowledge. Nothing prototyped. |
| LoRa link and voice | [lora_backhaul_and_voice.md](round-2-diy-builds-and-data-store/lora_backhaul_and_voice.md) | Medium | Airtime figures are calculated, not measured. Outdoor access point unpriced. |
| Solar panel testing | [pv_panel_field_test.md](round-2-diy-builds-and-data-store/pv_panel_field_test.md) | Medium | Safety procedure not checked against a standard. Panel and string details unknown. |

A seventh lane in this round read the private IsoKron archive and the public kronicle-lite repo end to end. Its notes describe a private codebase and are kept out of this public repo. The ideas carried over from it are in the design document's knowledge store section.

## Round 3: agent tooling, Rust and naming

| Lane | File | Confidence | Main gap |
| --- | --- | --- | --- |
| Agent software tools on the Pi | [agent_software_tools_on_pi.md](round-3-agent-tooling-rust-and-naming/agent_software_tools_on_pi.md) | Medium | Many per-tool details are from prior knowledge. Active assessment tools were excluded at the time, not inventoried. |
| Rust on the Pi | [rust_on_the_pi.md](round-3-agent-tooling-rust-and-naming/rust_on_the_pi.md) | Medium | No rigorous Rust-against-Python benchmark on a Pi exists. |
| Store design against client limits | [store_ax_and_visuals.md](round-3-agent-tooling-rust-and-naming/store_ax_and_visuals.md) | Medium | Nothing tested on the owner's own clients. Two cited bugs may already be fixed. |
| Name availability | [name_availability.md](round-3-agent-tooling-rust-and-naming/name_availability.md) | High for crates, npm and domains | No trademark search. |

## Round 4: checks against the outside reviews

Run on 25 November 2026 alongside the [review response](../docs/reviews/review-round-1-response.md).

| Lane | File | What it settled |
| --- | --- | --- |
| Hardware fact check | [hardware_fact_check.md](round-4-review-checks/hardware_fact_check.md) | Sixteen hardware claims from the reviews checked against vendor documents. Most confirmed; the hub named in the first design is discontinued. |
| Prior-art verification | [prior_art_verification.md](round-4-review-checks/prior_art_verification.md) | Every source the reviewers cited exists. Three were stretched. No built prior art found for the trust layer. |
| Evidence viewer options | [evidence_viewer_options.md](round-4-review-checks/evidence_viewer_options.md) | Rerun as the borrowed viewer, Lichtblick as fallback, MCAP stays the recording format. |

More lanes from this round are added as they finish.

## Notes written before later decisions

Some notes predate decisions that changed the design. Where a note and [`../docs/DECISIONS.md`](../docs/DECISIONS.md) disagree, the decision log wins. The main cases: the tool fence was widened to allow active testing of owned gear, the LoRa pair moved to phase 2, and the knowledge store dropped kronicle-lite compatibility.

## Next research pass

The two low-coverage lanes (old vehicles, module standard), an inventory of active assessment tools for owned gear, then price checks on every estimated line before anything is ordered.
