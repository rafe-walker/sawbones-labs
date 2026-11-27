# Bench build compatibility and safety review: the method

A repeatable checklist for any Sawbones bench build. A script ([`ci/check_hardware.py`](../../ci/check_hardware.py), rules HW-001 to HW-028, see the [README](README.md)) runs the mechanical checks against `data/parts.toml`, `data/netlist.toml` and `data/safety_paths.toml`. An agent or a person does the judgement checks. The P1 review (`compatibility.md`, `wiring-mock.md`) was produced with this procedure.

Field names in the data files are stable. The CI checker is written against them; renaming a field is a breaking change.

## 0. Procedure order

1. **Gather.** Read the parts list, the design intent (roadmap stage, issues, threat model, firmware pin map) and the owned-kit list.
2. **Record parts.** Write one `[[part]]` per part in `parts.toml`. Fill electrical fields from a datasheet or maker page and give its URL. Set `verified = true` only for fields read from a source; list them in `verified_fields`. Anything else is UNVERIFIED and goes on the bring-up list.
3. **Draw the netlist.** One `[[connection]]` per wire or joint in `netlist.toml`, each on a declared `[[net]]` with a voltage domain and an isolation domain.
4. **Declare safety paths.** Each path that can energise a load goes in `safety_paths.toml`, with its ordered series elements and what controls each. Mark variants with the `variant` field.
5. **Run the mechanical checks** (C-01…C-16 below). Each emits PASS/WARN/FAIL with the record ids involved.
6. **Run the judgement checks** (J-01…J-10). Write the result and reasoning into the review.
7. **Mock the build** in wiring order, and walk each exit test through it with the expected measurement.
8. **Emit the verdicts:** `COMPATIBILITY: PASS|PASS WITH CHANGES|FAIL` and `WIRING MOCK: …`, plus the exact change list for PASS.

Verdict roll-up:

- Any FAIL that has no listed change → **FAIL**.
- Any FAIL fixed by a listed change, or any WARN → **PASS WITH CHANGES**.
- Otherwise **PASS**.

## 1. Mechanical checks (script)

| ID | Name | Inputs | Rule | PASS / WARN / FAIL |
|---|---|---|---|---|
| C-01 | Every connection has a joint type | `connection.joint` | `joint ∈ {soldered-to-board, wired-and-soldered, screw-terminal, dupont, crimp-spade, barrel, usb, ribbon, panel-terminal}` | FAIL if missing or unknown |
| C-02 | Endpoints exist | `connection.from`, `connection.to` (`PART_ID:PIN` or `node:NAME`), `part.id`, `part.pins[].name`, `node.name` | Each endpoint names a declared part and pin, or a declared `[[node]]` (board terminal or stage node) | FAIL if not |
| C-03 | Net and domain declared | `connection.net`, `net.domain`, `net.isolation_domain` | Every connection's net is declared with a domain | FAIL if not |
| C-04 | Logic high level | Driver `part.voh_min_v` at its supply; receiver `part.vih_min_v` (absolute, or `vih_frac` × receiver supply) | `VOH(min) ≥ VIH(min) + margin_v` (default margin 0.3 V) | PASS ≥ margin; WARN 0–margin; FAIL < 0 |
| C-05 | Logic low level | `vol_max_v`, `vil_max_v` | `VOL(max) + margin_v ≤ VIL(max)` | as C-04 |
| C-06 | Input over-voltage | Net `nominal_v` (or `max_v`); receiver `abs_max_input_v`; receiver `tolerance_condition` | Net max ≤ receiver absolute max. If the tolerance holds only while powered (`tolerance_condition = "powered"`), the driver must be in the same `power_group` | FAIL over; WARN if conditional and not proven |
| C-07 | Unpowered back-feed | Connections between parts in different `power_group`s | A series resistor (`series_r_ohm ≥ 1000`) or an isolator sits on every such logic line | WARN if missing |
| C-08 | Pin drive current | Driver `io_max_ma`; load `input_current_ma` (summed over the net's receivers) | Sum ≤ 80 % of the driver's rating | WARN 80–100 %; FAIL > 100 % |
| C-09 | Rail current budget | Every part on a supply net: `supply_ma_max`; source `supply_out_ma` | Σ ≤ 80 % of the source rating | WARN 80–100 %; FAIL > 100 % or no source |
| C-10 | Fuse protects wire | `fuse.rating_a`; wire `ampacity_a` on the fused net | Fuse rating ≤ the smallest wire ampacity downstream, and fuse rating < source short-circuit current | FAIL if not |
| C-11 | I2C addresses unique | Parts with `interfaces` containing `i2c`; `i2c_addr`; the bus net | Unique per bus | FAIL on a clash; WARN if any address is unverified |
| C-12 | Isolation crossing | `net.isolation_domain` on both ends of each connection | A connection joins two different isolation domains only through a part with `isolator = true` that declares both domains, or the connection carries `bridges_isolation = true` with a `reason` | FAIL otherwise |
| C-13 | Safety output default state | For each series element in `safety_paths.toml`: its driver part's `default_state.{power_up, reset, brownout, unpowered, wire_off}` and the element's `trigger_polarity` | In every state, the element is open (de-energised) | FAIL if any state energises |
| C-14 | Single-fault coverage | `safety_paths.fault[]` per `variant` | Every row in the fault catalogue (§3) has an entry per path and variant, and `load_state ∈ {off, off-detected, on-bounded}`. `on-bounded` needs `bound_ms` | FAIL if any `load_state = on`; WARN on missing rows or `on-bounded` |
| C-15 | Independence of series elements | `series_elements[].controlled_by`, `.supply_net`, `.board` | At least two electronically controlled elements on each path have different `controlled_by`. Shared `board` or `supply_net` between two such elements is a WARN (common cause) | FAIL if all share one controller |
| C-16 | Exit-test observability | `connection.observed_by`, the roadmap's exit test list | Every exit test is observed by at least one connection. Every analyzer probe is on a net whose `nominal_v ≤ la.abs_max_input_v`, and its ground is on a net flagged `star_ground = true` | FAIL on a probe over limit or a ground off the star; WARN on an unobserved test |

Also mechanical:

- **C-17 Unverified count:** list every field outside `verified_fields`. These form the bring-up list.
- **C-18 Port budget:** count USB devices per host against `host.usb_ports`, and their current against the host limit.

### Mapping to the CI rules

| Method check | CI rule | Note |
|---|---|---|
| C-01 | HW-001 | `panel-terminal` is an allowed joint |
| C-02 | HW-002, HW-003 | endpoints, ids, series elements, fault rows |
| C-03 | HW-004 | |
| C-04, C-05 | HW-005, HW-006 | per connection, driver `from`, receiver `to` |
| C-06 | HW-007 | |
| C-07 | HW-008 | |
| C-08 | HW-009 | |
| C-09 | HW-010 | |
| C-10 | HW-011, HW-012, HW-013 | fuse vs wire, converter fuse, buck crowbar |
| C-11 | HW-014 | |
| C-12 | HW-015 | |
| C-13 | HW-016, HW-017 | open in every default state; no active-low module on an MCU pin |
| C-14 | HW-018, HW-019 | a variant `both` row covers A and B |
| C-15 | HW-020 | |
| C-16 | HW-021, HW-022, HW-023 | |
| C-17 | HW-028 | information only |
| C-18 | HW-024 | host port used twice |
| J-06, J-08, J-09 (partly) | HW-025, HW-026, HW-027 | reserved pins, trust-root bypass paths, no-name parts and fuse bytes |

## 2. Judgement checks (agent or human)

| ID | Name | Question | PASS / WARN / FAIL |
|---|---|---|---|
| J-01 | Trigger polarity vs firmware off level | Does the firmware's "off" level, its reset level and an unpowered driver all leave every load de-energised? Look especially for active-low modules on MCU pins | FAIL if any one energises |
| J-02 | Common-cause supplies | What happens to every brain at once when a shared rail goes (a) away, (b) low, (c) high (regulator short)? | FAIL if (c) leaves outputs undefined with no crowbar or fuse |
| J-03 | Probe and tool grounds | Can any tool ground (analyzer, scope, meter COM, debugger) touch a switched node and bypass a switch? | FAIL if the procedure allows it |
| J-04 | Hardware vs software series | Are the stop, arm and key in the power path, or only sensed? | FAIL if only sensed |
| J-05 | Test feasibility | Can each exit test be run as written with the parts on hand? Does a test kill its own recorder, or contradict another rule? | FAIL / redefine |
| J-06 | Bypass paths | Firmware-lock claims vs BOOT buttons, debug ports, USB bootloaders | WARN if the claim is wider than the hardware enforces |
| J-07 | Timing budgets | Do dead-man hold-up times and watchdog timeouts fit the stated stopping bound, at the actual toggle rate? | WARN if the values assume a rate not yet fixed |
| J-08 | Counterfeit risk | No-name or marketplace silicon on a safety path? | WARN plus a bench-acceptance test against the datasheet limits |
| J-09 | Pin map vs board | Do firmware pins exist on edge pins, and avoid on-board peripherals (LEDs, flash, USB)? | FAIL / remap |
| J-10 | Scope vs roadmap | Does every check-in item have the parts it needs? | WARN with an owner question |

## 3. Single-fault catalogue (run for every path and variant)

Each fault gets one row per path and variant in `safety_paths.toml`:

- Each controller: frozen, reset, reset-loop, brownout, unpowered, wrong-answer or glitched (keeps toggling).
- Each controller's outputs: state at power-up before firmware runs.
- Each supervisor or watchdog: frozen, reset, supply lost, misconfigured (fuses, OTP).
- Each relay: contact welded, coil open.
- Each transistor or MOSFET driver: collector-emitter or drain-source short, open.
- Each coupling capacitor: short.
- Host computer (Pi) off or hung.
- Each wire: off (power path, enable line, sense line, ground).
- Each shared rail: lost, low, high.
- Bridges between neighbouring channels.
- Tool ground misplaced.
- Supply overvoltage.

## 4. Counterfeit / acceptance test template

For every no-name IC on a safety path:

1. Test it off-board.
2. Measure each datasheet limit that the safety argument relies on (thresholds, timeouts, polarities, quiescent current).
3. Use a genuine distributor part as the control.
4. Lot rule: more than 20 % of the lot failing any step, or any polarity mismatch, rejects the lot.
5. Store each chip's numbers as measured facts.

## 5. Output files

| File | Content |
|---|---|
| `compatibility.md` | Interface matrix, safety tables, supervisor decision, missing parts, substitutions |
| `wiring-mock.md` | Ordered build with joint types, pin tables, analyzer maps, exit-test walkthroughs, verdicts |
| `data/parts.toml` | `[[part]]` records |
| `data/netlist.toml` | `[[net]]` and `[[connection]]` records |
| `data/safety_paths.toml` | `[[path]]`, `[[path.series_elements]]` and `[[fault]]` records |

### Stable field names

The checker also reads these optional fields, added with the CI (renaming any of them is a breaking change too):
parts: `kind` (`adapter`, `fuse`, `buck`, `ldo`, `crowbar`, `wire`, `terminal`, `analyzer`), `supply_out_group`, `trust_root`, `off_in_tests`, `counterfeit_risk`, `acceptance_passed`, `fuse_bytes`;
connections: `removable`;
series elements: `pins`.
A fault row's `path` is a glob on path ids, and `variant = "both"` means the row holds in variant A and in variant B.


- **parts.toml `[[part]]`:**
  - Identity: `id`, `maker`, `model`, `asin`, `qty`, `status` (on-list, owned, missing-must, missing-should, missing-nice), `role`, `board`.
  - Supply: `supply_min_v`, `supply_max_v`, `supply_ma_max`, `supply_out_ma`, `power_group`.
  - Logic levels: `logic_v`, `voh_min_v`, `vol_max_v`, `vih_min_v`, `vih_frac`, `vil_max_v`, `vil_frac`, `abs_max_input_v`, `tolerance_condition`.
  - Currents: `io_max_ma`, `input_current_ma`.
  - Behaviour: `trigger_polarity`, `default_state` {`power_up`, `reset`, `brownout`, `unpowered`, `wire_off`}.
  - Interfaces: `interfaces`, `i2c_addr`, `isolation_domain`, `isolator`, `isolator_domains`, `pins` [{`name`, `function`}].
  - Ratings: `ampacity_a`, `rating_a`.
  - Provenance: `datasheet_url`, `verified`, `verified_fields`, `notes`.
- **netlist.toml:**
  - `[[net]]`: `name`, `domain`, `nominal_v`, `max_v`, `isolation_domain`, `star_ground`.
  - `[[node]]`: `name`, `board`, `net`, `notes`.
  - `[[connection]]`: `id`, `from`, `to`, `net`, `domain`, `joint`, `variant` (A, B, both), `series_r_ohm`, `bridges_isolation`, `reason`, `observed_by` (list of exit test numbers), `notes`.
- **safety_paths.toml:**
  - `[[path]]`: `id`, `load`, `variant`, `description`.
  - `[[path.series_elements]]`: `order`, `id`, `part`, `kind`, `controlled_by`, `supply_net`, `board`, `trigger_polarity`, `open_when`.
  - `[[fault]]`: `id`, `path`, `variant`, `fault`, `element`, `effect`, `load_state`, `bound_ms`, `detected_by`, `verdict`.
