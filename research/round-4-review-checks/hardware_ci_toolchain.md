# Sawbones review round 1: hardware CI toolchain

Checked 25 November 2026 against package registries (PyPI, crates.io, npm, Homebrew, conda-forge, Docker Hub, Launchpad), official docs and project repositories.
All source text is paraphrased. Anything marked UNVERIFIED is from memory or is an engineering estimate that was not confirmed against a source in this pass.
Run times are estimates until the jobs are actually run (all UNVERIFIED); they are for small jobs on a 4 vCPU runner.

## One-page summary

**Runner facts (verified).** Public-repo standard runners are free. `ubuntu-latest` is Ubuntu 24.04 x64 with 4 vCPU, 16 GB RAM, 14 GB SSD. There is also a free `ubuntu-24.04-arm` (4 vCPU, 16 GB), useful as a Pi 5 stand-in for build and throughput tests. `macos-latest` is macOS 26 on Apple Silicon (3 cores, 7 GB). Job limit 6 h, matrix limit 256 jobs, 20 concurrent jobs on the Free plan (5 macOS), artifact storage 500 MB, cache 10 GB per repo.

### Recommended minimal toolchain (one tool per job)

| Job | Tool (version, date) | Licence | CI install | Mac M3 | Run time | What CI can enforce |
|---|---|---|---|---|---|---|
| System description | One `system.toml` + JSON Schema + own checker (Python or Rust) | own (Apache-2.0) | pip / cargo | yes | < 1 min | Power budget per rail, port counts, every harness pin maps to a real module pin |
| Schematic, PCB | KiCad `kicad-cli` 10.0.8 (30 Oct 2026) | GPL-3.0+ | container `kicad/kicad:10.0` | yes (native app) | 2 to 4 min | ERC and DRC clean (exit code 5 on violations), schematic/PCB parity, BOM and netlist regenerate |
| Harness | WireViz 0.4.1 (Jul 2024) | GPL-3.0 | pip + apt graphviz | yes | < 1 min | YAML parses and renders; wire list diff; own cross-check against system description |
| Circuit simulation | ngspice (47 current; apt has 42) + spicelib 1.6.4 (Oct 2026) + pytest | BSD-style / GPL-3.0 | apt + pip | yes (brew) | 1 to 5 min | Fault current, trip point, corner frequency inside limits across Monte Carlo / worst case |
| Parts and assembly | build123d 0.13.0 (Sep 2026) | Apache-2.0 | pip | yes | 2 to 6 min | No interference, fits 180 mm bed, clearances, mass and centre of gravity, STEP/STL/3MF regenerate |
| Slicing | OrcaSlicer 2.5.2 CLI | AGPL-3.0 (UNVERIFIED) | AppImage (cache it) | yes | 3 to 10 min | Slices without error, print time and filament under budget |
| Thermal | Own lumped network in Python (optionally `ht` 1.2.0 correlations) | MIT | pip | yes | seconds | Internal air under limit at 45 C ambient for stated airflow; required CFM |
| RF bookkeeping | Own NumPy script (+ scikit-rf 2.2.0 if S-parameter files are used) | BSD | pip | yes | seconds | No clock/converter harmonic inside a receive passband; link margins; cascade NF |
| Antennas | PyNEC 2.4.1 (23 Nov 2026) | GPL-3.0 | pip wheel | yes (arm64 wheel) | < 1 min | SWR under 2:1 across band, gain and pattern inside tolerance of reference |
| Signal fixtures | NumPy/SciPy generators + `sigmf` 1.14.0; Rust tests on the detector | BSD / LGPL-3.0 | pip, cargo | yes | 2 to 5 min | Detection and false-alarm rates at set SNR; no regression |
| Mic array | pyroomacoustics 0.10.1 (May 2026) | MIT (UNVERIFIED) | pip | yes | 1 to 2 min | DOA error under N degrees at set SNR and reverberation |
| Firmware | `cargo test` on host + embedded-hal-mock 0.11.1 + cross build for `thumbv8m.main-none-eabihf` | MIT/Apache-2.0 | rustup | yes | 3 to 8 min | Logic tests pass, firmware builds, flash size under budget |
| Full-wave EM (optional, not per-commit) | openEMS 0.37.0 (Oct 2026) | GPL-3.0 (UNVERIFIED) | source build, cache | unconfirmed on Apple Silicon | 30 to 120 min | Relative coupling between layout options only |

Skip for now: KiBot (add later for diffs and fab packs), atopile / tscircuit / SKiDL, FreeCAD FEM, OpenFOAM or any CFD, SPLAT!/ITM, Renode/QEMU/Wokwi, SysML v2 tools, Capella.

### Ranked list

**HIGH: real design leverage before hardware.** These are deterministic, the models are adequate, and mistakes they catch are expensive.
1. System-description checks (power budget, port count, pin maps) tied to ERC/DRC and the harness list. Wrong pins, missing nets and over-budget rails are the most common first-build failures.
2. Parametric CAD with interference, bed-fit and slicing checks. Printed parts are geometry problems; geometry simulates exactly.
3. SPICE on protection and interlock circuits with tolerance runs. Fault currents, dividers and RC timing are well modelled.
4. Lumped thermal plus fan energy balance. Accuracy is rough, but the decision (sealed versus ventilated, how much airflow) has large margins.
5. Host-side firmware and protocol tests. Interlock state machines can be tested exhaustively.
6. Synthetic IQ and audio fixtures for the detection pipeline. Exact for the DSP, and they make every later change safe.
7. RF arithmetic: harmonic collision table, link budgets, cascade noise figure. Cheap, and it drives clock and placement choices.

**MEDIUM: ranks options, must be confirmed by measurement.**
8. NEC2 models of wire antennas.
9. Component hotspot temperatures and the fan operating point.
10. Mic-array geometry and DOA algorithm choice in simulation.
11. Analytic slot estimate and coarse openEMS runs for antenna-to-antenna coupling (compare layouts A and B, not absolute dB).
12. Friis / two-ray / log-distance propagation over the lot.
13. Receiver desensitisation bounds from published USB 3.0 measurements.
14. SPICE of current-limiter ICs and converters using vendor models.
15. Linear FEA of printed parts (stiffness ranking only).

**LOW: not worth doing before hardware exists.**
16. Absolute shielding effectiveness of the real enclosure by full-wave simulation. Seams, cables and contact quality dominate and are not in the model; a printed plastic case is not a shield at all.
17. Simulating USB 3.0 or converter noise coupling into a receiver. The source (common-mode current on cables and connector shells) is unknown before layout. Measure-only.
18. CFD in CI. Hours of setup and run time for accuracy a calibrated lumped model matches.
19. Terrain-aware propagation at 250 m. ITM is specified for paths over 1 km; free terrain data is 30 m per pixel.
20. RP2350 instruction-level emulation in CI. No complete open emulator exists yet.
21. SysML v2 or Capella modelling; migrating to a code-first schematic tool.

---

## A. Electronics as files

### 1. KiCad `kicad-cli`, Docker, KiBot, KiAuto

- **Version.** KiCad 10.0.0 shipped March 2026; 10.0.8 is the newest stable (30 Oct 2026). Licence GPL-3.0 or later (UNVERIFIED, from memory).
- **CLI coverage (10.0 manual).** `sch erc`, `pcb drc`, `sch export` (netlist, bom, python-bom, pdf, svg, dxf, postscript), `pcb export` (gerbers, drill, position, step, stepz, glb, vrml, stl, brep, 3dpdf, ipc2581, odb, pdf, svg, statistics and more), `pcb render` (PNG or JPG, side, zoom, quality), `jobset run`, plus symbol and footprint export/upgrade.
- **Assertions.** `--exit-code-violations` on ERC and DRC returns 0 when clean and 5 when not. Severity filters exist (error, warning, exclusions). Reports come as text or JSON. DRC has `--schematic-parity` and `--refill-zones`. Netlist formats include `spice` and `spicemodel`, so a SPICE deck can be exported from the schematic for the simulation job.
- **Not found.** No simulate command appears in the CLI manual; plan to export the netlist and run ngspice directly. The manual does not say whether `pcb render` needs a display (UNVERIFIED either way).
- **Docker.** Official images are `kicad/kicad` on Docker Hub (also mirrored on GHCR), intended for `kicad-cli`; GUI use in the container is unsupported. Tags `10.0` / `10.0.8` are about 812 MB; `10.0-full` is about 1.45 GB. All tags seen are **amd64 only**. What `-full` adds (probably 3D models) is UNVERIFIED.
- **apt.** Ubuntu 24.04 ships KiCad 7.0.11, too old. Use the container (or the KiCad PPA, name UNVERIFIED).
- **Mac.** Homebrew cask `kicad` 10.0.8 is a universal build, so `kicad-cli` runs natively. Do not use the amd64 container on the Mac.
- **KiBot.** 1.9.1 (28 Jul 2026). KiCad 10 support is partial: 1.9.0 and 1.9.1 added some, and the unreleased 1.9.2 adds more plus fixes for KiCad 10 variants. Images `setsoft/kicad_auto:ki10` (about 590 MB) and `setsoft/kicad_auto_test:ki10` were updated 5 Nov 2026. The CI docs page still lists action tags only up to `INTI-CMNB/KiBot@v2_k8`; the repo has a `Dockerfile_k10`, so a k10 tag probably exists (UNVERIFIED). Licence conflict: PyPI says GPL-3.0, the GitHub repo shows AGPL-3.0.
- **KiAuto.** 2.3.10 (14 Jul 2026), Apache-2.0. Drives the GUI for things the CLI lacked; less needed now.
- **Other action.** `sparkengineering/kicad-action` (MIT): ERC, DRC, PDF, BOM, Gerbers, board image. KiCad version is not selectable and its roadmap says failure detection needs work.
- **Verdict.** Use plain `kicad-cli` in the official container. Add KiBot only when diffs, fab packs or cost reports are wanted.
- **Limits.** ERC checks pin types and connectivity, not function. It will not notice a wrong resistor value or a swapped pair that is electrically legal.

### 2. Code-first schematic tools

| Tool | Version / date | Licence | Output | Built-in checks | Maturity note |
|---|---|---|---|---|---|
| atopile | 0.15.9, 12 Sep 2026 | MIT | Updates a KiCad PCB file; BOM, fab and assembly data. No schematic output found. | Assertions with units and tolerances solved by the compiler; parametric part picking | 4.0k stars. Requires Python 3.14 exactly. Its docs landing page calls it pre-release. |
| SKiDL | 3.0.0, 16 Oct 2026 | MIT | KiCad netlist, editable KiCad schematic, SVG, XML BOM, DOT graph; can run SPICE | `ERC()` for unconnected pins, drive conflicts, power errors | 1.7k stars. Targets KiCad 5 to 10. Long-lived, one main author (UNVERIFIED). |
| tscircuit | npm 0.0.3296, published daily | MIT | Circuit JSON, Gerbers, pick-and-place, BOM; 3D and schematic viewers | None documented on the repo page | 2.7k stars. Still 0.0.x. Placement is manual and the autorouter is described as in progress. KiCad export path not confirmed. |

Also seen: circuit-synth 0.12.1 (Jan 2026, MIT). Not evaluated.

Verdict: none is needed. The boards here are small. A hand-drawn KiCad schematic checked by `kicad-cli` is the lower-risk source of truth. If code-first is wanted later, SKiDL is the least disruptive because it emits KiCad files and has an ERC.

### 3. Wiring harness as code

- **WireViz** 0.4.1 (13 Jul 2024), GPL-3.0. YAML in; Graphviz `.gv`, SVG, PNG, HTML and a tab-separated BOM out. Needs Graphviz (apt has 2.42; Homebrew has 16.x). No release for over two years, 161 open issues and 37 open pull requests, though a `dev` branch exists. Still the de facto standard.
- **What it asserts.** It fails on malformed YAML. How strictly it rejects bad pin references is UNVERIFIED. Treat it as a renderer and put the real checks in the system-description job: every wire end names an existing connector pin, gauge is adequate for the rail current, no pin used twice.
- **Alternatives.** `harnice` 0.12.3 (23 Nov 2026, MIT) is a new CLI harness tool: first release September 2026, one maintainer, too young to depend on. `wireviz-web` 0.4.3 (AGPL-3.0) is a web wrapper. Splice is a web GUI, not files-as-code (UNVERIFIED).
- **Verdict.** WireViz, with the YAML generated from (or checked against) the system description.

### 4. Circuit simulation in CI

- **ngspice.** Release 47 files are dated 11 Aug 2026. Ubuntu 24.04 apt has 42; Homebrew has 47 with Apple Silicon bottles; conda-forge is stuck at 41 with no Apple Silicon build. Version 42 is fine for small analog circuits.
- **Python wrappers.** PySpice 1.5 dates from May 2021 (stale). InSpice 1.7.0.7 (Sep 2026, GPL-3.0) is an active fork. **spicelib** 1.6.4 (4 Oct 2026, GPL-3.0) is the best fit: netlist editor, parallel batch runner with timeouts, raw-file reader, and `Montecarlo` and `WorstCaseAnalysis` helper classes. It drives the ngspice command-line binary. ngspice has no `.step`, so sweeps are loops in Python or a `.control` block.
- **How to assert.** Export the netlist with `kicad-cli`, run with spicelib, read traces into NumPy, assert in pytest. Examples that work well:
  - Peak fault current through a protective resistor stays under its pulse rating with supply at maximum and resistance at minimum tolerance.
  - A filter's -3 dB corner stays within 5% across 500 Monte Carlo runs with 1% resistors and 5% capacitors (expect this to fail with 5% or 10% capacitors, which is the point).
  - Interlock comparator trips between two thresholds over tolerance and temperature coefficient.
- **Run time.** A few hundred runs of a 20-node circuit take seconds to a couple of minutes.
- **Limits.** Results are as good as the device models. Vendor models for eFuses, load switches and converter ICs are often encrypted, LTspice-only or behavioural, so "the current limiter trips at X" is a medium-confidence result. No layout parasitics, no thermal effects, no EMC.
- **Licence note.** Scripts that import GPL libraries (spicelib, PyNEC, WireViz) sit alongside Apache-2.0 code without problem when they are separate test tools, but do not link them into the shipped Rust code. Not legal advice.

## B. Mechanical and printed parts as code

### 5. CadQuery and build123d

- **Versions.** CadQuery 2.8.0 (21 Jun 2026), build123d 0.13.0 (21 Sep 2026); both Apache-2.0, both on the OCP kernel. `cadquery-ocp` 8.0.1 has wheels for Linux x86-64, Linux arm64 and macOS arm64, so `pip install` works on the runner and the Mac. conda-forge lags for build123d (0.9.1).
- **build123d exports.** STEP, STL, 3MF (through its `Mesher`), glTF, BREP, SVG, DXF.
- **Interference.** build123d has `Compound.do_children_intersect(include_parent=False, tolerance=1e-5)`. It screens pairs by bounding box, then intersects the shapes and returns a flag, the offending pair and the shared volume. That is a ready-made CI assertion. CadQuery's assembly docs show a constraint solver and STEP export but no interference method; there you intersect pairs yourself and test the volume.
- **Other checks.** Bounding box against 180 x 180 x 180 mm; `volume`; `center(CenterOf.MASS)`. Mass from volume times density overstates printed mass because infill is not solid: take filament grams from the slicer instead. Centre of gravity for the assembly needs per-part masses (use slicer grams plus datasheet masses for bought parts).
- **Rendering.** Neither library documents headless PNG in the pages read. Safe option: SVG projections (no GPU needed, diff-friendly), converted to PNG if wanted. Shaded PNG through VTK (wheels 9.7.2) or `f3d` 4.0.1 (BSD, wheels for Linux and macOS arm64) should work under `xvfb-run` (UNVERIFIED on the runner).
- **Choice.** build123d, for the built-in interference check and 3MF export. `ocp-vscode` 4.3.0 is the local viewer.
- **Limits.** CAD is exact; printers are not. Fit depends on a clearance constant (often 0.2 to 0.3 mm, UNVERIFIED for this printer) that only a printed tolerance coupon can set.

### 6. Slicing in CI

- **OrcaSlicer** 2.5.2. Its wiki states the CLI can run headless for automation and CI. Typical call: load the model, `--load-settings "process.json;printer.json"`, `--load-filaments`, `--arrange 1`, `--slice 0`, `--export-3mf`. `--export-statistics` writes a per-plate JSON with print time and filament totals. `--outputdir`, `--info`, `--no-check` and per-setting overrides exist. A secondary source lists return codes: 0 success, -2 invalid parameters, -100 slicing error, -101 G-code path conflict, plus codes for missing files, bad presets and out-of-bed G-code.
- **Bambu Studio** 02.09.02.62 (24 Nov 2026). Same CLI family (`--slice`, `--load-settings`, `--load-filaments`, `--export-3mf`, `--arrange`, `--orient`). Its wiki warns that the settings files must be full, flattened configs, not the partial inheriting profiles shipped under `resources/profiles/BBL`. The same trap likely applies to OrcaSlicer (UNVERIFIED).
- **PrusaSlicer** 2.9.7 (apt has 2.7.2). CLI has `--export-gcode`, `--load`, `--info`, `--repair`, `--export-3mf`. Print time and filament appear as G-code comments (UNVERIFIED). Whether it ships a Bambu A1 mini profile is UNVERIFIED; assume not.
- **A1 mini profile.** Export a flattened machine, process and filament JSON once from the OrcaSlicer GUI and commit them. Exact stock profile names are UNVERIFIED.
- **Headless caveats.** Whether the Linux build truly needs no display is not stated in the wiki pages read; wrap in `xvfb-run` to be safe. The Linux AppImage may need extraction on the runner (UNVERIFIED). The community image `linuxserver/orcaslicer` is a GUI-over-web image, not a CLI image.
- **Mac.** All three have macOS builds (OrcaSlicer is universal).
- **Assertions.** Exit code zero, print time and grams under budget per part, part count per plate. Slicer warnings such as floating regions may not change the exit code (UNVERIFIED); parse the log.

### 7. Printability and structural checks

- **Overhang and wall thickness.** `trimesh` 5.2.2 (MIT): watertight test, then overhang area as the share of faces whose normal points more than 45 degrees below horizontal. Wall thickness is better asserted on the CAD parameters (for example wall at least 1.2 mm) than measured on the mesh.
- **FEA.** FreeCAD 1.1.4 has an FEM workbench that drives CalculiX and Gmsh and is scriptable (the tutorial page could not be read; headless use through `freecadcmd` is UNVERIFIED). FreeCAD is not in Ubuntu 24.04 apt; use conda-forge (which also has Apple Silicon builds) or the AppImage. A lighter path is Gmsh 4.15.2 (pip) to mesh the STEP, then CalculiX (`calculix-ccx` 2.21 in apt, 2.24 on conda-forge with Apple Silicon) and parse the result file.
- **Honest limit.** Printed plastic is anisotropic, weaker between layers, infill-dependent, and creeps when warm (UNVERIFIED figures, well known in general). Isotropic linear FEA ranks designs by stiffness and shows stress concentrations. It does not predict failure load. Worth it only for a latch, a carry handle or a long shelf; otherwise use beam formulas.

### 8. Gridfinity generators

- **cq-gridfinity** (PyPI `cqgridfinity` 0.5.7, 21 Nov 2024, MIT). CadQuery. Baseplates, boxes, drawer spacers, rugged boxes; 42 mm grid and 7 mm height units; CLI scripts; STEP, STL, SVG. No release for two years.
- **gridfinity_build123d** (MIT, git install only, about 100 stars). Bins, baseplates and compartments in a compositional API; has tests, mypy and CI.
- **gfthings** 0.9.0 is CC-BY-NC-SA: incompatible with an Apache-2.0 repo. Avoid.
- **Verdict.** With build123d chosen, start from gridfinity_build123d pinned to a commit, or write the 42 mm base profile directly (small).

## C. Thermal

### 9. Lumped network versus CFD

- **Lumped model.** Internal air to wall (convection plus radiation), wall conduction, wall to ambient (convection plus radiation). A published method uses flat-plate natural-convection correlations for top, bottom and sides and solves the surface temperatures iteratively; its author calls the result an initial estimate to be refined by test. `ht` 1.2.0 (MIT) supplies correlations.
- **Worked numbers (own arithmetic, UNVERIFIED inputs).** A 250 x 200 x 100 mm box has 0.19 m2 of surface. With a combined outside coefficient near 9 W/m2K, 3 mm plastic at 0.2 W/mK and an inside film near 6 W/m2K, total resistance is about 1.5 K/W. Sealed at 30 W that is a rise of about 46 K: internal air near 91 C at 45 C ambient, above the softening point of common filaments. With a fan, air rise is power over (density x heat capacity x flow): 30 W at a real 5 CFM gives about 11 K, so about 56 C inside.
- **So the design answer arrives from 20 lines of Python:** sealed is not viable at 20 to 40 W in plastic; forced ventilation is needed; size the fan for real delivered flow.
- **Realistic accuracy (judgement, UNVERIFIED).** Sealed lumped: rise within about 30 to 50%. Ventilated bulk air: within about 20% if delivered flow is known, but delivered flow is commonly well under the fan's free-air rating. Uncalibrated CFD by a non-specialist is not clearly better; calibrated CFD may reach 10 to 20% on bulk air and still depends on component models for hotspots.
- **CFD tooling.** OpenFOAM Foundation v14 (14 Jul 2026). Ubuntu 24.04 apt carries a 2019-era build; current packages come from the project's own repository. macOS is supported through a Multipass VM; conda-forge builds are Linux-only. A conjugate heat transfer case needs meshing, radiation and buoyancy setup and runs for tens of minutes to hours.
- **Verdict.** Lumped model in CI, asserted at 45 C ambient with the worst-case power. CFD in CI is not worth it. If airflow routing becomes a question, a one-off local run is enough.
- **Cannot tell.** Hotspots on specific ICs, dead-air pockets, dust-filter loading, solar gain on the case.

## D. RF and interference

### 10. Antenna modelling (NEC2)

- **Engines.** PyNEC 2.4.1 (23 Nov 2026, GPL-3.0) wraps nec2++ and has wheels for Linux x86-64 and macOS arm64. `nec2c` 1.3.1 and `xnec2c` 4.4.12 are in Ubuntu apt but not in Homebrew core. PyNEC is the one to use; its examples include a monopole over ground, a dipole SWR sweep and an optimiser loop with SciPy.
- **Fit.** Quarter-wave ground planes, dipoles, collinears and small Yagis at 433, 915 and 1090 MHz are thin-wire structures, which is what NEC2 does well. Runs take seconds.
- **Assertions.** Feed impedance and SWR across the band, peak gain, pattern shape (for example gain toward the horizon), front-to-back ratio.
- **Limits.** NEC2 cannot model PCB or chip antennas on dielectric, and thick or tightly spaced conductors break its thin-wire assumptions (UNVERIFIED specifics). Real SWR shifts with the feedline, the mount and nearby objects. Expect resonance within a few percent (UNVERIFIED) and treat gain as free-space.

### 11. Full-wave EM

- **openEMS.** v0.37.0 released 9 Oct (2026 by context). FDTD with a Python interface. It is not in Ubuntu 24.04 apt (the last packaged build was an old snapshot in 22.04 and 23.10). The documented route is a source build with the project's update script and its Python flag; macOS uses a Homebrew tap that builds from HEAD, with Apple Silicon not mentioned. Build time on a runner is UNVERIFIED (expect 10 to 20 min; cache it).
- **(a) Coupling between two antennas a few centimetres apart.** Yes, this is a normal two-port S21 problem. Own estimate: a 310 x 260 x 160 mm domain at 2 mm cells is about 1.6 million cells, a few hundred MB. Run time depends on how long energy rings in the box: an open or lossy plastic case settles fast (minutes); a closed metal box is a high-Q cavity and can need 100,000 or more time steps (30 to 60 min or more). Refining to 1 mm cells multiplies cells by 8 and halves the time step, which pushes past a sensible CI budget.
- **(b) Shielding of a conductive box with slots and holes.** Possible in principle, with the same cavity problem plus thin slots that force a fine local mesh. It fits in 16 GB; it does not reliably fit in a per-commit job.
- **Published examples.** None found for enclosure shielding with openEMS specifically. The official tutorials are antennas, lines and filters. A forum thread on using it for enclosure emissions drew the advice that testing a prototype may be quicker and that beginner documentation is thin. The general method is well published elsewhere: one FDTD study of a server-style enclosure with a 252-hole airflow panel (0.5 x 0.5 x 1 cm cells, 0.3 to 1.2 GHz) compared against chamber measurements and found radiation scales with the number of apertures, with little mutual coupling at 1 cm spacing.
- **Alternatives.** Meep 1.35/1.36 (Oct to Nov 2026, GPL-2.0+; pip wheels including macOS arm64) is FDTD aimed at photonics, with less convenience for ports and metal boxes. Palace 0.17.0 (June 2026) is a frequency-domain FEM solver with driven and eigenmode analyses, technically a good fit for cavities, but it installs through Spack and is heavy. Elmer 26.2.1 (Apr 2026) has no Ubuntu package and a steeper workflow. Antmicro's gerber2ems (Apache-2.0) wraps openEMS for PCB traces and warns that whole boards are very expensive; not needed for full-speed USB.
- **Analytic slot estimate.** Shielding is roughly 20 log10(wavelength / (2 x slot length)), zero at a half-wave slot (formula from memory, UNVERIFIED against a primary source in this pass). At 2.4 GHz the wavelength is 125 mm: a 3 mm hole gives about 26 dB, a 10 mm slot 16 dB, a 30 mm slot 6 dB, a 62 mm slot nothing. An array of n holes costs roughly 10 log10(n) more (UNVERIFIED). A reference tutorial adds: many small holes beat a few large ones; a deep hole adds waveguide-below-cutoff attenuation of about 30 dB per depth-to-width ratio; seams leak more than holes; one unfiltered wire through the wall can cancel the benefit; and an enclosure with openings can make emissions worse at some frequencies.
- **When the estimate is good enough.** As a design rule (keep the longest opening under about 6 mm for roughly 20 dB at 2.4 GHz), always. As a prediction of achieved shielding, only when the source is far from the opening and the box is not resonant. A 200 x 150 mm metal box has its first cavity mode near 1.25 GHz (own arithmetic), so at 2.4 GHz it is overmoded and the true figure can swing by 10 to 20 dB either way.
- **Bigger point.** If the noise source and the receiver are inside the same box, wall shielding is irrelevant; separation, internal partitions and cable quality matter. A 3D-printed case shields nothing unless lined.

### 12. RF system bookkeeping

- **scikit-rf** 2.2.0 (18 Oct 2026, BSD). S-parameter networks, cascading, de-embedding; it has a noisy-network tutorial (page could not be read, details UNVERIFIED). Useful once Touchstone files for LNAs and filters are in the repo. No built-in IP3 cascade was confirmed.
- **Cascade NF and IP3** are closed-form sums; 30 lines of NumPy with asserts is enough.
- **Harmonic collision table.** No maintained open tool was found worth adopting; write it. For each source (crystals, USB clock, SDR reference, converter switching frequency) list harmonics and flag any inside a receive passband. Own arithmetic to show the value: 12 MHz harmonics fall at 432, 912, 924 and 1092 MHz; 48 MHz at 432 and 912 MHz; a 28.8 MHz SDR reference at 432, 921.6 and 1094.4 MHz. Those sit inside the 915 MHz band or within about 2 to 5 MHz of 433.92 and 1090 MHz, close enough to land in a wide SDR capture window. Converter fundamentals are far below these bands; their risk is broadband edge ringing, which this table cannot predict.
- **Desense estimate.** Receiver floor is -174 dBm/Hz plus bandwidth plus noise figure; added in-band noise raises it by 10 log10(1 + ratio). A 20 dB rise costs a factor of ten in free-space range.
- **Assertions.** No listed harmonic inside a guarded band; link margin above a threshold; cascade NF under a limit.

### 13. USB 3.0 and switching-converter noise

- **What is known.** Intel's white paper: the 5 Gbit/s scrambled data spectrum is broadband from near DC to about 5 GHz, so it covers 2.4 GHz and cannot be filtered out. It measured the 2.4 GHz noise floor rising by nearly 20 dB near an external USB 3.0 drive and about 25 dB at a receptacle. Shielding the drive bought about 12 dB; an improved, chassis-bonded connector at least 10 dB. A wireless mouse failed at about 1 m with a drive attached; moving the receiver away from the port fixed it. The effect was present whether the drive was idle or busy.
- **Can it be simulated before building?** Not credibly. The radiator is common-mode current on cable shields and connector shells, set by details that do not exist before layout and are not in any model you can write. The same holds for converter noise, where loop area, switch-node ringing and cable common-mode currents dominate.
- **What EMC engineers do pre-layout (rules of thumb, UNVERIFIED as a set).** Frequency planning (section 12). Keep USB 3.0 ports, cables and devices away from 2.4 GHz antennas, or run the hub at USB 2.0 speed where 5 Gbit/s is not needed. Use fully shielded cables and connectors bonded to chassis. Keep converter hot loops small; choose switching frequencies and spread-spectrum options with the receive bands in mind; filter every cable leaving a shielded volume. Budget a desense allowance and plan the measurement.
- **Verdict.** Bound it with published numbers, design by rule, measure first thing.

### 14. Propagation over a 4 acre lot

- **Free-space loss at 250 m (own arithmetic).** 433 MHz: 73 dB. 915 MHz: 80 dB. 1090 MHz: 81 dB. 2.44 GHz: 88 dB.
- **Two-ray breakpoint** (4 x height x height / wavelength) with both antennas at 1.5 m: about 13 m at 433 MHz and 72 m at 2.4 GHz. Beyond it loss grows much faster than free space, so low antennas at 433 MHz will underperform Friis badly.
- **Terrain tools.** SPLAT! 1.4.2 dates from 2014 (GPL-2, in apt, command-line) and uses 3 or 1 arc-second terrain. Signal-Server is archived and unmaintained; forks exist. NTIA's ITM reference code states a minimum path of more than 1 km and antenna heights from 0.5 m. `itmlogic` 1.2 (MIT) is a Python port. Radio Mobile is a Windows GUI (UNVERIFIED).
- **Verdict.** At 250 m the lot is a handful of terrain pixels and inside ITM's stated minimum distance. Use Friis, two-ray and a log-distance model with an exponent range (about 2.5 to 3.5, UNVERIFIED) and a vegetation margin, in the same Python file as the link budgets. Terrain tools are not worth it at this scale.

### 15. Synthetic signals and fixtures

- **Generators.** Plain NumPy/SciPy is enough and is deterministic from a seed: OFDM-like bursts with Wi-Fi timing, GFSK packets hopping across channels, LoRa chirps, a comb at multiples of an inverter switching frequency with jitter, coloured and impulsive noise.
- **Channel.** Apply noise, frequency and sample-rate offset, a short multipath filter, gain steps, clipping and 8-bit quantisation in the same script. That covers what a low-cost SDR does to a signal. Heavier channel simulators are unnecessary here.
- **Rust.** `futuresdr` 0.10.0 (20 Oct 2026, Apache-2.0) includes ports of LoRa, 802.11 and 802.15.4 transceivers, but its README calls it experimental and early-stage. Use `rustfft` 6.4.1 directly for the detector; consider FutureSDR's LoRa and Wi-Fi blocks only as waveform references. A `sigmf` crate exists (0.1.1, small).
- **GNU Radio** 3.10.12 (apt has 3.10.9; Homebrew and conda-forge have Apple Silicon builds). Runs headless as Python flowgraphs. A large install for what NumPy already does; keep it for local exploration.
- **SigMF.** A recording is a JSON metadata file plus a binary sample file sharing a base name; the metadata has global fields (datatype, sample rate), capture segments (centre frequency) and annotations. Spec v1.2.0, CC-BY-SA-4.0. Python package `sigmf` 1.14.0 (LGPL-3.0+). Annotations double as ground truth for tests.
- **Public recordings.** IQEngine is a web toolkit for analysing and sharing recordings; its dataset list and licences could not be read (UNVERIFIED). Prefer generated fixtures in CI, plus a few short real captures added later as release assets.
- **Assertions.** Detection probability at a set SNR, false alarms per hour on noise-only input, time and frequency estimate error, classifier confusion, and real-time factor measured on the arm64 runner.
- **Limits.** Tests prove the DSP, not the radio. Real captures add front-end overload, images, spurs and drifting gain that synthetic data lacks.

## E. Audio and vibration

### 16. Microphone array simulation

- **pyroomacoustics** 0.10.1 (1 May 2026). DOA algorithms: SRP-PHAT, MUSIC, NormMUSIC (recommended over plain MUSIC), CSSM, WAVES, TOPS, FRIDA. `circular_2D_array(center, M, phi0, radius)` builds the geometry; a 7-mic array is six on a circle plus a centre point appended. Room simulation uses image sources with optional ray tracing (UNVERIFIED from the pages read, known from memory). `acoular` 26.8 (BSD) is an alternative aimed at beamforming maps.
- **Assertions.** Azimuth error under N degrees for sources every 30 degrees, at set SNR, in free field and in a reverberant room; two-source separation; behaviour near the spatial-aliasing frequency (about speed of sound over twice the mic spacing: near 4 kHz for 43 mm spacing, own arithmetic).
- **Use.** Choosing radius, mic count and algorithm before buying; regression-testing the DOA code.
- **Limits.** No mic gain or phase mismatch unless you add it, no diffraction around the case, no wind, no structure-borne vibration, no clock skew between channels. Outdoor accuracy will be worse than simulated.

## F. Firmware without hardware

### 17. Emulators and host-side testing

| Emulator | Version | RP2040 | RP2350 (Cortex-M33) | CI use |
|---|---|---|---|---|
| Wokwi | cloud service | yes | **Partial**: one M33 core only; GPIO, UART, ADC, I2C/SPI master, PWM, DMA, timers; USB serial only; new RP2350 PIO instructions missing; no RISC-V cores | `wokwi/wokwi-ci-action@v1` with a token secret; simulation runs on Wokwi's servers |
| Renode | 1.17.0 (7 Sep 2026) | Not in recent release notes or the board list excerpt read. A community add-on exists, marked frozen, tied to Renode 1.16.1, no USB, partial PIO | Not found | Has a macOS arm64 build |
| QEMU | 11.1.x | No Pico machine in mainline. An RFC patch series (July 2026) adds RP2040 without USB, PIO, ADC, I2C or PWM | Not found | n/a |
| rp2040js | 1.4.2 (14 Nov 2026), MIT | yes; loads UF2, GDB server, exit code on expected serial text | No mention | Runs locally under Node |

- **Wokwi CI terms.** The free tier allows 50 simulation minutes per rolling 30 days with a 5 minute session cap; paid tiers 200, 500 and 2,000. The pricing page offers a free Community licence for open source but lists CI minutes only for the Pro tier. Open-source CI terms beyond the free 50 minutes are not documented. Firmware is uploaded to their cloud.
- **Answer.** In late 2026 only Wokwi simulates the RP2350's M33 core, partially, and with no USB beyond serial. That is not enough to test USB instrument firmware, and the quota is small. Do not build CI around emulation.
- **What works now.**
  - Split firmware into hardware-free `no_std` crates (protocol, state machines, scaling, interlock logic) and thin board glue. Run `cargo test` on the host; property-test the interlock state machine.
  - `embedded-hal-mock` 0.11.1 (MIT/Apache-2.0) mocks embedded-hal 0.2 and 1.0, with async mocks, using expected-transaction lists checked by `done()`. Last release June 2024.
  - Embassy's executor runs on std and embassy-time has std and mock time drivers, so async tasks and timeouts can run on the host. The Embassy book has no dedicated host-testing guide.
  - Cross-build for the M33 target in CI and assert flash size. Versions: embassy-rp 0.10.0, embassy-executor 0.10.0 (Mar 2026), rp235x-hal 0.4.0.
- **Later, hardware in the loop.** probe-rs 0.32.0 (Jul 2026), defmt-test 0.5.1 (Oct 2026) and embedded-test 0.7.3 (Oct 2026) run tests on a real board from a self-hosted runner with a debug probe.

### 18. One system description driving checks

- **SysML v2.** OMG's tool list names open-source options: the pilot implementation, Eclipse SysON (web application, release 2026.11) and OpenMBEE components. Sensmetry's Syside offers scripting; its Python package (`syside` 0.11.0) is proprietary-licensed. No listed tool advertises a simple CLI validator. These are modelling environments; you would still write the power-budget rule yourself.
- **Capella.** Eclipse desktop tool for the Arcadia method; heavyweight and GUI-centred (UNVERIFIED, from memory).
- **atopile** is the nearest thing to "systems as code" with a units-and-tolerance solver, but it covers the electrical domain only.
- **JITX** is commercial (UNVERIFIED).
- **Nothing found** that is light, open and spans electrical plus mechanical.
- **Recommendation for one person.** A single `system.toml` listing modules, ports, rails, grounds, bays and connectors, validated by a schema, with a small checker that:
  - sums worst-case current per rail against supply and connector ratings;
  - counts USB ports and bays against modules;
  - checks every harness wire end against a declared pin and every declared pin against the KiCad netlist;
  - emits the constants that the WireViz YAML, the build123d models, the thermal model and the RF budget import.
  This is a few hundred lines and is the highest-leverage piece in the whole toolchain.

## G. Putting it together

### 19. Workflow sketch

Per push or pull request (path-filtered so only affected jobs run; total wall time about 10 minutes in parallel):

| Job | Runs on | Steps | Est. time |
|---|---|---|---|
| `sysdesc` | ubuntu-latest | Validate `system.toml`; budgets; cross-checks; emit generated constants | < 1 min |
| `electronics` | ubuntu-latest, container `kicad/kicad:10.0` | ERC, DRC with parity, netlist, BOM, PDF, board render, STEP; upload artefacts | 2 to 4 min |
| `harness` | ubuntu-latest | apt graphviz; WireViz render; wire-list diff | < 1 min |
| `spice` | ubuntu-latest | apt ngspice; pytest with spicelib; Monte Carlo | 1 to 5 min |
| `mech` | ubuntu-latest | build123d: build, interference, bed fit, mass/CoG; export STEP/STL/3MF/SVG | 2 to 6 min |
| `slice` | ubuntu-latest | Cached OrcaSlicer under xvfb; slice; assert time and grams | 3 to 10 min |
| `budgets` | ubuntu-latest | Thermal model, harmonic table, link budgets, antenna sweeps (PyNEC) | 1 to 2 min |
| `dsp` | ubuntu-latest and ubuntu-24.04-arm | Generate fixtures; detector tests; mic-array DOA tests; real-time factor on arm64 | 2 to 5 min |
| `firmware` | ubuntu-latest | fmt, clippy, host tests, cross build, size budget | 3 to 8 min |

Scheduled or manual only: openEMS coupling comparisons (30 to 120 min, cached build), large Monte Carlo sets, any FEA.

Local-only on the Mac: GUI work in KiCad and OrcaSlicer, the build123d viewer, exploratory openEMS, FreeCAD FEM, GNU Radio experiments. A weekly `macos-latest` smoke job can prove the Python and Rust jobs still run on Apple Silicon (note that runner has only 7 GB).

**Will not run natively on Apple Silicon, or is unconfirmed:**
- KiCad and KiBot Docker images: amd64 only, emulated and slow on the Mac. Use the native KiCad app's CLI.
- OpenFOAM: Linux packages only; Multipass VM on macOS.
- conda-forge ngspice: no Apple Silicon build. Use Homebrew.
- openEMS: source build through a Homebrew tap; Apple Silicon not confirmed.
- Palace (Spack build) and Elmer (no Homebrew formula): unconfirmed.
- `nec2c`, `xnec2c`, SPLAT!, CalculiX: not in Homebrew core. PyNEC has a native wheel; CalculiX is on conda-forge.
- Wokwi CI: cloud service, platform-independent but quota-limited.

### 20. Ranking with reasons

See the summary for the list. The reasoning behind the tiers:

- **High** items share three properties: the physics is simple or exact (connectivity, geometry, linear circuits, energy balance, arithmetic, software logic); inputs are known before purchase (datasheets, dimensions); and an error is costly to find late (a re-spun board, a reprinted case, a cooked enclosure).
- **Medium** items have sound physics but depend on inputs you cannot know exactly yet (real antenna surroundings, delivered airflow, mic mismatch, vendor model fidelity, ground and vegetation). They are good for choosing between options and for sizing margins.
- **Low** items fail one of two tests. Either the dominant mechanism is absent from any model you can build now (seam leakage, common-mode cable currents), or the tool costs more than the measurement it replaces (CFD, terrain propagation at 250 m, immature emulators, modelling frameworks).

### 21. First validating measurement for each medium item

| Medium item | First measurement on real hardware | What it corrects |
|---|---|---|
| NEC2 antenna models | Vector network analyser sweep of each built antenna on its real mount with its real cable | Resonant length factor, SWR bandwidth |
| Hotspots and fan operating point | Before any electronics exist: power resistors as a dummy load at 20, 30 and 40 W in the printed case with the real fan; log inlet, exhaust, internal air and one surface | Case thermal resistance and delivered airflow |
| Mic-array DOA | Outdoors in quiet air, noise bursts from a speaker at known bearings every 30 degrees at a few metres; also a co-located source to measure channel mismatch | Achievable bearing error; per-channel calibration |
| Slot estimate and antenna coupling | Noise-floor delta per band on the actual SDRs with each internal source switched on and off, the bench on battery; then S21 between the two antenna ports at the chosen spacing | Real isolation; whether partitions or lining are needed |
| Propagation | Walk test: fixed low-power transmitter, logging received level with position every 25 m out to 250 m, per band | Path-loss exponent and shadowing margin |
| USB 3.0 desense bound | Sensitivity test of the 2.4 GHz receiver (packet error against a stepped attenuator) with the SuperSpeed link up and down | Actual desense in dB at the real spacing |
| Limiter and converter SPICE | Resistive load steps on the built circuit, capturing trip current and trip time on a scope | Model parameters for the protection path |
| Printed-part FEA | Known weight on a printed part or coupon, deflection measured with a dial gauge, at room temperature and warm | Effective modulus, creep allowance |

One more for a high item: print a tolerance coupon (holes, slots, a snap-fit gauge) on the real printer and filament before printing any enclosure part, and set the clearance constants from it.

## Could not verify

- KiCad: licence text; whether `pcb render` needs a display; contents of the `-full` image; PPA name; whether any CLI simulation command exists (none seen).
- KiBot: exact action tag for KiCad 10; which licence applies (PyPI and GitHub disagree).
- atopile: CI action name, schematic output, KiCad version requirement (docs pages did not load). tscircuit: KiCad export and built-in checks. JITX and Capella: nothing read in this pass.
- WireViz: which invalid inputs raise errors. Splice: nature of the tool.
- ngspice: release notes for 46 and 47 (only the file dates were seen). InSpice: API stability.
- CadQuery and build123d: a documented headless PNG path; f3d or VTK offscreen on the runner.
- OrcaSlicer: licence; display requirement on Linux; AppImage behaviour on runners; structure of any result file beyond the statistics JSON; return codes (seen only in a secondary source); stock A1 mini profile names; whether warnings alter the exit code. PrusaSlicer: console binary, exit status, Bambu profiles, where time and filament are reported.
- FreeCAD FEM: headless scripting details (tutorial page blocked). Printed-part strength ratios.
- Thermal: every accuracy percentage is engineering judgement; no source with measured-versus-model error for a small plastic case was found. Filament softening temperatures.
- openEMS: licence; build time; Apple Silicon build; cells-per-second on a runner; memory per cell. All run-time figures in section 11 are own estimates. No published openEMS enclosure-shielding example was found; absence is not proof none exists.
- The slot-aperture formula and the multi-aperture correction were not confirmed against a primary text in this pass. Palace and Elmer licences and macOS support.
- scikit-rf noise and IP3 features (tutorial page failed to load). NEC2 accuracy figures and geometric limits.
- IQEngine dataset list and licences; any specific public SigMF recording suitable as a fixture. Radio Mobile. Path-loss exponent range.
- pyroomacoustics licence and room-simulation details (from memory).
- Renode: full board list (page truncated) and the platform directory (blocked), so absence of RP2040/RP2350 in mainline is likely but not proven. QEMU: status of the RP2040 RFC after July 2026.
- Wokwi: open-source CI allowance; Rust firmware loading for Pico 2.
- All job run times.

## Sources

GitHub runners and limits: https://docs.github.com/en/actions/reference/runners/github-hosted-runners , https://docs.github.com/en/actions/reference/limits , https://github.com/actions/runner-images
KiCad: https://www.kicad.org/blog/2026/10/KiCad-10.0.8-Release/ , https://docs.kicad.org/10.0/en/cli/cli.html , https://www.kicad.org/download/docker/ , https://hub.docker.com/r/kicad/kicad
KiBot and actions: https://kibot.readthedocs.io/en/latest/Changelog.html , https://kibot.readthedocs.io/en/latest/usage_with_ci_cd.html , https://github.com/INTI-CMNB/KiBot , https://hub.docker.com/r/setsoft/kicad_auto , https://github.com/sparkengineering/kicad-action
Code-first: https://github.com/atopile/atopile , https://pypi.org/project/atopile/ , https://github.com/devbisme/skidl , https://github.com/tscircuit/tscircuit
Harness: https://github.com/wireviz/WireViz , https://pypi.org/project/harnice/
SPICE: https://sourceforge.net/projects/ngspice/files/ng-spice-rework/ , https://github.com/nunobrum/spicelib
CAD: https://build123d.readthedocs.io/en/latest/import_export.html , https://build123d.readthedocs.io/en/latest/_modules/topology/composite.html , https://cadquery.readthedocs.io/en/latest/assy.html
Slicers: https://github.com/OrcaSlicer/OrcaSlicer/wiki/cli_mode , https://github.com/OrcaSlicer/OrcaSlicer/wiki/cli_actions , https://github.com/OrcaSlicer/OrcaSlicer/wiki/cli_misc , https://deepwiki.com/SoftFever/OrcaSlicer/10.2-cli-mode-and-headless-operation , https://github.com/bambulab/BambuStudio/wiki/Command-Line-Usage , https://manpages.debian.org/testing/prusa-slicer/prusa-slicer.1.en.html
Gridfinity: https://github.com/michaelgale/cq-gridfinity , https://github.com/Ruudjhuu/gridfinity_build123d
Thermal: https://www.heatsinkcalculator.com/blog/?p=567 , https://openfoam.org/download/
Antennas and EM: https://pypi.org/project/PyNEC/ , https://github.com/thliebig/openEMS-Project/releases , https://docs.openems.de/install.html , https://cecas.clemson.edu/cvel/pdf/EMCS99-035.pdf , https://learnemc.com/practical-em-shielding , https://learnemc.com/shielding-theory , https://eevblog.com/forum/rf-microwave/openems-and-far-field-radiated-emissions , https://github.com/antmicro/gerber2ems , https://github.com/NanoComp/meep/releases , https://github.com/awslabs/palace/releases , https://github.com/ElmerCSC/elmerfem/releases
USB 3.0 noise: https://www.usb.org/sites/default/files/327216.pdf
Propagation: https://www.qsl.net/kd2bd/splat.html , https://github.com/Cloud-RF/Signal-Server , https://github.com/NTIA/itm
Signals: https://github.com/sigmf/SigMF , https://github.com/FutureSDR/FutureSDR , https://www.iqengine.org/
Acoustics: https://pyroomacoustics.readthedocs.io/en/pypi-release/pyroomacoustics.doa.html , https://pyroomacoustics.readthedocs.io/en/pypi-release/pyroomacoustics.beamforming.html
Firmware: https://docs.wokwi.com/parts/board-pi-pico-2 , https://docs.wokwi.com/wokwi-ci/getting-started , https://docs.wokwi.com/wokwi-ci/github-actions , https://wokwi.com/pricing , https://github.com/renode/renode/releases , https://github.com/matgla/Renode_RP2040 , https://patchew.org/QEMU/20260705220320.90395-1-gilles.grimaud@univ-lille.fr/ , https://www.qemu.org/docs/master/system/target-arm.html , https://github.com/wokwi/rp2040js , https://docs.rs/embedded-hal-mock/latest/embedded_hal_mock/ , https://embassy.dev/book/
Systems modelling: https://www.omg.org/sysml/sysmlv2/sysml-tool/ , https://mbse-syson.org
Versions, dates, licences and platform builds were read from the PyPI, crates.io, npm, Homebrew, conda-forge, Docker Hub and Launchpad APIs on 25 Nov 2026.
