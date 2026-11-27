# P1 hardware design and safety review

The safety-engineering record for the P1 bench ("Prove the Interlock on the bench"): which parts, how they connect, how each load path fails, and a checker that runs on every pull request so the design cannot drift into an unsafe state.

| File | What it is |
|---|---|
| [`compatibility.md`](compatibility.md) | Interface matrix, single-fault tables, supervisor decision, missing parts, substitutions |
| [`wiring-mock.md`](wiring-mock.md) | The build in wiring order, with joint types, pin tables, analyzer maps and the six exit tests walked through |
| [`method.md`](method.md) | The reusable procedure, mechanical and judgement checks, field names |
| [`BOM.md`](BOM.md) | Maker, model, ASIN, quantity, role and datasheet for each part |
| [`data/parts.toml`](data/parts.toml), [`data/netlist.toml`](data/netlist.toml), [`data/safety_paths.toml`](data/safety_paths.toml) | The design in machine-readable form. The checker reads these |

## Run the checker

```sh
python3 ci/check_hardware.py            # every hardware/*/data directory
python3 ci/check_hardware.py --json     # machine-readable
python3 -I -m unittest discover -s ci -p 'test_check_hardware.py'
```

Python 3.11 or newer, standard library only. The output is one line per rule (HW-001 to HW-028) with PASS, WARN, FAIL or INFO, then the findings with record ids. Exit status 1 on any FAIL. WARN means a documented condition (an unverified datasheet value, a marginal level, a known common cause); INFO is the datasheet-provenance share and never fails.

The workflow is [`.github/workflows/hardware.yml`](../../.github/workflows/hardware.yml). It runs on pull requests and pushes to main that touch `hardware/` or `ci/*hardware*`.

## What a rule change needs

`ci/` is a protected path (see [CONTRIBUTING](../../CONTRIBUTING.md)). The only reviewed exception to a rule is the fault allow-list in `ci/check_hardware.py` (`A28`, an analyzer ground on a switched node, handled by procedure). Every finding of the review has a `test_regression_<issue>_*` test and a row in [`docs/REGRESSIONS.md`](../../docs/REGRESSIONS.md).

## Data corrections made when the checker went in

The checker found these gaps in the first draft of the data. Each is supported by the review text.

- Isolation: the two isolated RS485 sides are one floating domain (`ISO_RS485`, compatibility.md 2.6), so both isolators name it. The guard board's USB link (C721) is a real ground bridge and now says so with a reason.
- Variant B inherits every variant A fault row except A10 (compatibility.md 3.3), so those rows are `variant = "both"` and their `path` is a glob.
- Fault rows A31 (fuse) and A32 (key switch) were missing. They are WARN until the clean-room reviewer signs them.
- Connection C103 is seen by test 4a (lamps lit through the whole chain); C310 and C311 carry the module's on-board 1 kilohm; C710 and C711 are removable; C724 is the Mac-hosted analyzer link for test 4.
- Added fields: part `kind`, `supply_out_group`, `trust_root`, `off_in_tests`, `counterfeit_risk`, `acceptance_passed`, `fuse_bytes`; connection `removable`; series element `pins`.

## Provenance

Only fields read from a datasheet or maker page are marked verified. The rest is on the bring-up list (compatibility.md section 8).
