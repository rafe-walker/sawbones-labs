# Old vehicles beyond OBD: AI-agent sensing, prior art, 5VZ-FE Tacoma specifics, AI-first design (state: 25 Nov 2026)

> **Method note for the report writer.** The shared web-search budget for this session ran out after two searches. Everything below marked with a link was read directly from a page (press articles, vendor pages, GitHub pages, Arizona statute, NHTSA API, Wikipedia). Where I could not open a source, the item sits under **Inferences** tagged **[EJ]** (my engineering judgement) or **[UR]** (unverified recollection from training data, no page read this session), or under **Gaps**. Do not present [EJ]/[UR] items as sourced facts. Prices are USD; "verified" means read on a vendor page this session, "est." means unverified.
> Not repeated here (covered by `vehicle_obd.md`): OBD adapters, Rust crates, ELM327 emulators, Mode 04 gating, K-line protocol identification.

---

## 1. Prior art: people using Claude / ChatGPT / other LLMs to diagnose and repair old vehicles (2025-2026)

### Takeaway
The "33-year-old Jeep" story is almost certainly Muhammad Zulali's 1994 Jeep Cherokee XJ video (published 10 Nov 2026, about 1.4 M views in five days): a ~USD 15 Arduino Nano bridge to the Chrysler SCI diagnostic plug, Claude Code doing protocol and diagnosis, Codex doing ECU memory work, released as `xj-scanner` with agent playbooks. What made it work was a cheap serial bridge plus an agent that could read live data and iterate; what nearly sank it was a wrong fuel-trim scale borrowed from another tool, blind tuning that made emissions worse, and a fix (changing the fuel map) that critics say masks a mechanical fault and collides with emissions-tampering law.

### Cited Findings

**The Jeep story (primary and press)**
- Owner: YouTuber Muhammad Zulali (@mzulali); vehicle: 1994 Jeep Cherokee XJ 4.0L, engine computer part number 56030117; video title "Claude reverse engineered my 32-year-old Jeep. Then it started controlling it." — [Tom's Hardware, 13 Nov 2026](https://www.tomshardware.com/tech-industry/artificial-intelligence/1994-jeep-cherokee-owner-hacks-32-year-old-car-with-claude-ai-to-pass-california-smog-tests-enthusiast-reverse-engineered-the-ancient-ecu-for-diagnostics-and-control); [xj-scanner repo](https://github.com/MZULALI/xj-scanner); video: https://youtu.be/WPj3aQ-xxpI
- Video date 10 Nov 2026; 1,417,084 views when read on 15 Nov 2026; repo had 252 stars on 15 Nov per this article (a secondary analysis based on YouTube auto-captions and the repo) — [FourWeekMBA](https://fourweekmba.com/ai-claude-codex-got-his-32-year-old-jeep-past-smog-owner-says/)
- Failure mode: high NOx on the California dynamometer test at 25 mph; a new O2 sensor did not fix it; after "blind" adjustments it failed at both 25 and 50 mph; each test cost close to USD 100 — [Tom's Hardware](https://www.tomshardware.com/tech-industry/artificial-intelligence/1994-jeep-cherokee-owner-hacks-32-year-old-car-with-claude-ai-to-pass-california-smog-tests-enthusiast-reverse-engineered-the-ancient-ecu-for-diagnostics-and-control); [HotHardware](https://hothardware.com/news/driver-uses-claude-to-hack-1994-jeep-into-passing-smog-test)
- His 15 mph NOx reading, previously passing, went from about 80 to 900 during the attempts — [FourWeekMBA](https://fourweekmba.com/ai-claude-codex-got-his-32-year-old-jeep-past-smog-owner-says/)
- Hardware: Arduino Nano clone (~USD 3) on a breadboard, three wires to the under-hood 6-pin diagnostic plug, laptop over USB; total parts about USD 15 — [Tom's Hardware](https://www.tomshardware.com/tech-industry/artificial-intelligence/1994-jeep-cherokee-owner-hacks-32-year-old-car-with-claude-ai-to-pass-california-smog-tests-enthusiast-reverse-engineered-the-ancient-ecu-for-diagnostics-and-control); [TechEBlog, 17 Nov 2026](https://www.techeblog.com/claude-reverse-engineer-old-jeep/)
- Protocol: Chrysler SCI at roughly 7,800 baud with electrically inverted signalling relative to a normal serial port; the Nano handled the inversion — [Tom's Hardware](https://www.tomshardware.com/tech-industry/artificial-intelligence/1994-jeep-cherokee-owner-hacks-32-year-old-car-with-claude-ai-to-pass-california-smog-tests-enthusiast-reverse-engineered-the-ancient-ecu-for-diagnostics-and-control)
- First contact failed until resistors in the breadboard circuit were pulled; the first live reading (coolant temperature, battery voltage, no codes) arrived at 1:30 a.m. — [TechEBlog](https://www.techeblog.com/claude-reverse-engineer-old-jeep/); [FourWeekMBA](https://fourweekmba.com/ai-claude-codex-got-his-32-year-old-jeep-past-smog-owner-says/)
- Data the model got: live ECU parameters logged on test drives at 25 and 50 mph (O2 sensor, fuel trim, codes), plus a byte-by-byte dump of the 32 KB ECU program that Claude analysed off the car — [TechEBlog](https://www.techeblog.com/claude-reverse-engineer-old-jeep/); [Tom's Hardware](https://www.tomshardware.com/tech-industry/artificial-intelligence/1994-jeep-cherokee-owner-hacks-32-year-old-car-with-claude-ai-to-pass-california-smog-tests-enthusiast-reverse-engineered-the-ancient-ecu-for-diagnostics-and-control)
- First agent actuation: Claude proposed running the fuel pump, the owner agreed, the pump ran about 10 s until he asked it to stop — [FourWeekMBA](https://fourweekmba.com/ai-claude-codex-got-his-32-year-old-jeep-past-smog-owner-says/)
- Fuel pressure itself was read with a physical gauge on the rail test port, because these Chrysler ECUs have no fuel pressure sensor (reader comment) — [The Autopian, 15 Nov 2026](https://www.theautopian.com/i-guess-ai-can-fix-a-jeep-cherokee-xj-now/)
- Documented error: an early "running rich, 22% extra fuel" conclusion was wrong. Sources attribute it to a scaling taken from the open-source ChryslerScanner tool / data from other models, not to a free-form hallucination — [The Autopian](https://www.theautopian.com/i-guess-ai-can-fix-a-jeep-cherokee-xj-now/); [The Drive, 15 Nov 2026](https://www.thedrive.com/news/cant-pass-emissions-with-your-pre-obd-ii-car-have-claude-tune-it-for-you); [FourWeekMBA](https://fourweekmba.com/ai-claude-codex-got-his-32-year-old-jeep-past-smog-owner-says/)
- Root cause as told: a California-specific calibration that trims fuel at smog-test loads; a normal reflash was impossible (ECU built Dec 1993, a few months before flash-capable units), so Codex injected a small routine via the diagnostic link (runs ~90 times/s) and then found a way to persist it in EEPROM — [FourWeekMBA](https://fourweekmba.com/ai-claude-codex-got-his-32-year-old-jeep-past-smog-owner-says/); [TechEBlog](https://www.techeblog.com/claude-reverse-engineer-old-jeep/)
- Division of labour in the owner's words: Claude did most of the diagnostic work, Codex nearly all the memory work, he "just did the wiring and the driving" — [Tom's Hardware](https://www.tomshardware.com/tech-industry/artificial-intelligence/1994-jeep-cherokee-owner-hacks-32-year-old-car-with-claude-ai-to-pass-california-smog-tests-enthusiast-reverse-engineered-the-ancient-ecu-for-diagnostics-and-control)
- Owner's own caveats: video description says he is not a mechanic and some engine statements may be wrong; repo says hobby project, use at your own risk — [FourWeekMBA](https://fourweekmba.com/ai-claude-codex-got-his-32-year-old-jeep-past-smog-owner-says/); [xj-scanner](https://github.com/MZULALI/xj-scanner)
- Critiques: Autopian readers say the reflash hides a real fault (weak pump, clogged filter/injectors, vacuum leak, dead EGR, tired catalyst); Tom's Hardware commenters say the same; The Drive notes it took substantial programming skill — [The Autopian](https://www.theautopian.com/i-guess-ai-can-fix-a-jeep-cherokee-xj-now/); [Tom's Hardware](https://www.tomshardware.com/tech-industry/artificial-intelligence/1994-jeep-cherokee-owner-hacks-32-year-old-car-with-claude-ai-to-pass-california-smog-tests-enthusiast-reverse-engineered-the-ancient-ecu-for-diagnostics-and-control); [The Drive](https://www.thedrive.com/news/cant-pass-emissions-with-your-pre-obd-ii-car-have-claude-tune-it-for-you)
- Legal angle raised by secondary coverage: California anti-tampering rules (Vehicle Code 27156) bar changes to the certified emission configuration; the video does not address this; no smog report was shown — [FourWeekMBA](https://fourweekmba.com/ai-claude-codex-got-his-32-year-old-jeep-past-smog-owner-says/); [remio](https://www.remio.ai/post/claude-helps-diagnose-a-1994-jeeps-smog-failure)
- **Conflicts between accounts:** (a) rich vs lean: Autopian says the CA map ran lean ("low fuel, high NOx"), TechEBlog says the computer "added too much fuel" yet the fix "restored the missing fuel"; (b) outcome: Tom's Hardware/HotHardware/The Drive say it passed, TechEBlog says "nearly passed"; (c) cost: USD 3 (The Drive, the Nano only) vs USD 15 (full parts list); (d) GitHub stars: 252 per FourWeekMBA vs "1 star, 1 commit" on the repo page as fetched on 25 Nov 2026 (possibly a rendering artefact or a re-created repo) — [Autopian](https://www.theautopian.com/i-guess-ai-can-fix-a-jeep-cherokee-xj-now/); [TechEBlog](https://www.techeblog.com/claude-reverse-engineer-old-jeep/); [xj-scanner](https://github.com/MZULALI/xj-scanner)

**What `xj-scanner` ships (the most directly reusable prior art)**
- Scope: reads fault codes and live data, logs drives to CSV, runs the ECU's built-in actuator tests; tested on one car only; other 1991-95 Chrysler/Jeep SBEC units "should be close"; GPL-3.0; Python scanner + two Arduino sketches (`xj_bridge`, `xj_sniff`) + log summarizer, parameter sweep and part-number checker — [xj-scanner](https://github.com/MZULALI/xj-scanner)
- Wiring: Nano D10, D11, GND to plug cavities 4, 3, 2; cavity 6 carries 12 V key-on and must not be connected; ground and 12 V wires share a colour on the test car, so colour is not evidence — [xj-scanner](https://github.com/MZULALI/xj-scanner)
- Credits: parameter IDs, scaling formulas and code text from László Dániel's ChryslerScanner (GPLv3); the corrected fuel-trim scale from disassembled SBEC code (AR1972/jeep_2.5L) — [xj-scanner](https://github.com/MZULALI/xj-scanner)
- Agent instructions (`AGENTS.md`, with `CLAUDE.md` pointing to it; skills `hookup`, `scan`, `first-contact`): run the simulated "DEMO" Jeep first; have the user meter every cavity key-on before any wire goes in; treat documented voltages as expectations from one car and stop if they differ; reading is safe; three actions need explicit confirmation with an explanation (erase codes, which also wipes learned fuel; reset fuel trim; actuator tests, key-on engine-off, short runs, say what will click or spray first); "don't guess at a diagnosis", report what the data does and does not show; check the parameters doc before trusting odd values (fuel trim and vehicle speed have scaling quirks); `--o2` mode samples the O2 sensor at about 20 readings/s — [AGENTS.md](https://github.com/MZULALI/xj-scanner/blob/main/AGENTS.md)
- ChryslerScanner: open hardware and software for Chrysler CCD / PCI / SCI buses (Arduino Mega / ESP32), GPL-3.0, 91 stars, 474 commits — [ChryslerScanner](https://github.com/laszlodaniel/ChryslerScanner)

**OBD + MCP projects (LLM reads the car through tools)**
- Petr Pátek, "DIY AI Car Diagnostics with a $15 Bluetooth Adapter and Python": 2010 Ford Focus CC roof fault; Vgate vLinker FD (BLE); Python MCP server `petrpatek/obd2-mcp-server` with read-only tools `read_dtc`, `get_live_data`, `get_vehicle_info`, `list_modules`, `get_freeze_frame`, `explain_dtc` and a guarded `clear_dtc`; mock mode, 28 tests. Pain points: hours lost because the adapter was BLE (GATT) not classic serial; body modules on manufacturer buses not reachable with generic OBD, so the module scan was simulated from FORScan data; DTC database scraped and not authoritative; Claude's hypothesis (roof precondition failing because of a broken window regulator) was plausible but not yet confirmed by a repair — [dev.to](https://dev.to/petr_patek_12/diy-ai-car-diagnostics-with-a-15-bluetooth-adapter-and-python-3fhk)
- Jan Spörer (18 Oct 2025): Python FastMCP server on top of `pyobd`, aimed at Claude Code, built to get Mercedes transmission-fluid data his scanner lacked; no failures described — [blog](https://janspoerer.github.io/post/2025/10/18/mcp-obd-ii.html)
- Hugging Face "Vehicle Diagnostic Assistant" (MCP 1st Birthday hackathon): MCP server embedded in a W600-based OBD device, LangGraph agent, NHTSA VIN decode, PID history, ELM327 simulator for the demo; YouTube-search tool did not work in the hosted demo; MIT — [README](https://huggingface.co/spaces/MCP-1st-Birthday/Vehicle-Diagnostic-Assistant/blob/main/README.md)
- "IndAutomation" MCP: includes `obd_mcp_server.py` with `read_dtcs`, `clear_dtcs`, `get_vehicle_info`, `read_pid`, `get_freeze_frame`, `diagnose_dtc`; 106 DTCs incl. Toyota-specific; mock or ELM327 mode; MIT; "built by a working field tech" — [mcpservers.org](https://mcpservers.org/servers/repairxpert/indautomation)

### Inferences
- [EJ] The owner's "33-year-old Jeep" is this 1994 XJ story (press says 32-year-old). No other 2026 Jeep/Claude emissions story surfaced.
- [EJ] Why it worked: (1) a three-wire TTL-level serial bridge is enough for many pre-OBD-II ECUs; (2) an open definitions file (ChryslerScanner) gave the agent a starting map; (3) the agent could close the loop itself (log a drive, read the log, change one thing); (4) the repo encodes hard-won procedure as agent playbooks.
- [EJ] Why it nearly failed, and what this project should copy: a channel's scaling was trusted without an independent check. Every derived quantity the sensor pack exposes should carry provenance (definition source, vehicle it was validated on) and, where possible, a cross-check from a second sensor (wideband lambda vs fuel trim, gauge vs transducer).
- [EJ] All OBD-MCP projects found are OBD-only, CAN-era and Python. None fuses audio, scope or pressure data, none handles ISO 9141-2 pacing explicitly, none has a vehicle profile for modified vehicles. A Rust MCP "sensor body" with synchronized multi-modal capture has no direct precedent in what I could read.
- [EJ] The Jeep fix changed emission calibration. An open project should keep ECU writes out of the default tool set and state that it will not help defeat emission controls; diagnosis of the underlying fault is the supported path.

### Gaps
- Could not open the YouTube video page, the Cybernews article, or any Hacker News / Reddit thread; could not search for other cases (carbureted cars, OBD-I smog failures, ChatGPT photo-based diagnosis, LLM + automotive scope projects). Absence here is absence of search, not evidence that none exist.
- No measured NOx values (ppm vs limit) were published in any text source read; they appear only in a screenshot.
- No independent confirmation of the smog pass, and no account of what the persistent EEPROM routine actually changes.
- `docs/protocol.md`, `docs/parameters.md` and the three SKILL.md playbooks of xj-scanner were not read.
- No documented case found of an LLM misdiagnosing from audio or photos; the only documented error class is "wrong scaling from borrowed definitions".

---

## 2. Pre-OBD-II and manufacturer-specific diagnostics an open tool could support

### Takeaway
A generic front end with three building blocks, (a) open-collector K-line/single-wire UART with selectable pull-up (5 V / 12 V) and inversion, (b) an optocoupled "lamp watcher" input, (c) a switched jumper output, covers most pre-1996 systems: GM ALDL, Chrysler SCI, VAG KW1281, Toyota/Honda blink codes. Each is a definitions-file problem more than a hardware problem.

### Cited Findings
- OBD-I codes are commonly read by bridging connector pins and counting Check Engine lamp flashes as two-digit numbers; connectors and procedures were manufacturer-specific and often under the hood — [Wikipedia: On-board diagnostics](https://en.wikipedia.org/wiki/On-board_diagnostics)
- GM ALDL: 160 baud early version is one-way PWM-style; the later 8192 baud version is bidirectional half-duplex UART on a single wire, mostly request-driven, with actuator tests and overrides; logic levels are inverted relative to RS-232 (logic one near 0 V, zero at +5 or +12 V); common connectors are the 12-pin and 5-pin types; basic use needs ground, sometimes battery, and one data line; USB-to-ALDL cables and Bluetooth modules exist — [Wikipedia: ALDL](https://en.wikipedia.org/wiki/ALDL)
- GM "OBD 1.5" (1994-96) puts the data stream on pin 9 of a J1962-style connector — [Wikipedia: On-board diagnostics](https://en.wikipedia.org/wiki/On-board_diagnostics)
- Some 1989-1995 Ford (DCL) and some 1989-1995 Toyota/Lexus models offered a live data stream before OBD-II — [Wikipedia: On-board diagnostics](https://en.wikipedia.org/wiki/On-board_diagnostics)
- Chrysler/Jeep 1991-95 SCI: readable with a ~USD 3 Arduino Nano and three wires at about 7,800 baud, inverted levels — [Tom's Hardware](https://www.tomshardware.com/tech-industry/artificial-intelligence/1994-jeep-cherokee-owner-hacks-32-year-old-car-with-claude-ai-to-pass-california-smog-tests-enthusiast-reverse-engineered-the-ancient-ecu-for-diagnostics-and-control); open definitions and hardware in [ChryslerScanner](https://github.com/laszlodaniel/ChryslerScanner) (CCD/PCI/SCI, GPL-3.0) and [xj-scanner](https://github.com/MZULALI/xj-scanner)
- VAG KW1281: `kw1281test` (C#, .NET 10, MIT, 270 stars) runs on Windows/macOS/Linux with a "dumb" USB-KKL cable (FTDI + D2xx on macOS/Linux): fault codes, group reads, basic settings, adaptation, actuator tests, EEPROM/RAM/ROM dumps — [kw1281test](https://github.com/gmenounos/kw1281test)
- Toyota pre-1996 blink codes: ignition on, engine off, throttle closed, battery above 11 V; jumper TE1 to E1 in the engine check connector; no fault gives a steady blink about twice per second; codes are two digits with a 1.5 s gap between digits and 2.5 s between codes; table of 2-digit codes (e.g. 21 O2 sensor, 25 lean, 26 rich, 52 knock sensor, 71 EGR) — [troublecodes.net](https://www.troublecodes.net/toyota/)
- OBD-II K-line physical layer for reference: ISO 9141-2 and KWP2000 share pin 7 (K) and optional pin 15 (L), 10.4 kbit/s UART, K idles high — [Wikipedia: On-board diagnostics](https://en.wikipedia.org/wiki/On-board_diagnostics)

### Inferences
- [EJ] Ranking by cost-to-support with one generic front end:
  1. **Blink-code reader (Toyota, Honda, early Nissan/Mazda/Mitsubishi, GM "paperclip", Ford KOEO via lamp/STO)**: an optocoupler across the MIL (or a camera watching the lamp) plus a relay/transistor to close the jumper. Under USD 5. A camera-only version needs zero electrical contact: the agent tells the user which two pins to bridge, then counts flashes from video.
  2. **GM ALDL 8192**: single-wire half-duplex UART, level shift only; definitions come from the TunerPro ADX ecosystem [UR].
  3. **VAG KW1281 / KWP1281 and BMW/others on K-line**: same K-line transceiver the pack already needs for ISO 9141-2 (5-baud init, then 1200-10400 baud).
  4. **Chrysler SCI**: UART at 7812.5 baud (and 62500 high-speed) [UR for the exact rates], inverted; proven by xj-scanner.
  5. **GM ALDL 160 baud**: bit-banged PWM; easy on a microcontroller or logic analyzer.
  6. **Ford EEC-IV**: self-test output is pulse codes on STO (blink reader works); the DCL data link is RS-485-like and model-specific [UR].
  7. **Nissan Consult I, Subaru SSM1, Honda (pre-OBD-II serial), BMW ADS**: all TTL/K-line class serial with odd baud rates or init; each needs its own definition set [UR].
- [EJ] The existing logic analyzer + sigrok is the right reverse-engineering tool for unknown single-wire protocols; the agent can measure bit time, polarity and idle level before any driver is written (xj-scanner's `xj_sniff` does the same job).
- [EJ] Adopt xj-scanner's rule as a platform rule: before any pin is driven, the user meters every cavity key-on and the agent compares against expected values; a 12 V cavity is never connected to logic.

### Gaps
- Not read this session (no search): TunerPro/ALDLdroid/OpenALDL pages, Renix Engine Monitor projects (1987-90 Jeep 4.0), Ford EEC-IV readers, Nissan Consult I tools, FreeSSM/SSM1, BMW ADS, Honda serial. Names above are leads from memory only.
- Exact SCI baud rates, Renix frame format, and Toyota pre-1996 "M-OBD" live-data stream details were not sourced.

---

## 3. Toyota specifics: 2003 Tacoma Double Cab, 5VZ-FE, manual + 4WD swap

### Takeaway
Sourced facts confirm the engine layout (3.4 L, three-coil wasted spark, belt-driven cams, non-interference) and that no factory Double Cab had a manual, so the truck's ECU/harness history matters to every code interpretation. The detailed 5VZ-FE fault patterns, DLC1 presence on a 2003, the TC-CG procedure, and TIS prices could not be sourced this session and need a factory-manual check before the agent relies on them.

### Cited Findings
- 5VZ-FE: 3,378 cc V6, 93.5 mm bore, 82 mm stroke, 9.6:1, DOHC 24-valve with shim-over-bucket tappets, sequential multi-port injection, **wasted-spark ignition with three coils**, belt-driven camshafts, non-interference; 190 hp at 4800 rpm and 220 lb-ft at 3600 rpm in the Tacoma — [Wikipedia: Toyota VZ engine](https://en.wikipedia.org/wiki/Toyota_VZ_engine)
- Rear-bank spark plug access on DOHC VZ engines requires removing the intake plenum (as stated for most DOHC VZ variants) — [Wikipedia: Toyota VZ engine](https://en.wikipedia.org/wiki/Toyota_VZ_engine)
- First-gen Tacoma: Double Cab (MY2001-2004) and PreRunner were sold only with an automatic; the V6 was paired with the R150F manual or A340F (4WD) / A340E (2WD) automatics; ABS became standard for MY2003; 4WD and PreRunner use six-lug hubs — [Wikipedia: Toyota Tacoma](https://en.wikipedia.org/wiki/Toyota_Tacoma)
- 2003 Tacoma recalls on record: 05V225000 front lower ball joints (surface scratch causing wear, looseness, possible separation from the knuckle; dealer replaces both; recall began July 2005); 12V542000 spare-tire carrier lift plate corrosion; 03V189000 Double Cab fuel inlet hose protector — [NHTSA recalls API](https://api.nhtsa.gov/recalls/recallsByVehicle?make=toyota&model=tacoma&modelYear=2003)
- Frame corrosion campaign: Toyota extended corrosion coverage to 2001-2004 frames (replacement if rusted through) — [Wikipedia: Toyota Tacoma](https://en.wikipedia.org/wiki/Toyota_Tacoma)
- Toyota TIS (techinfo.toyota.com) covers most 1990-and-later Toyotas; the **Standard** level includes repair manuals, wiring diagrams, service bulletins and training; **Professional Diagnostic** adds Techstream software and calibrations; payment is handled by a third-party site and the landing page shows no prices — [Toyota TIS](https://techinfo.toyota.com/)
- Operation CHARM (charm.li) hosts free manuals for 1982-2013 vehicles across about 60 makes including Toyota, states nothing about its source or licence, and points to a successor site "LEMON" claiming coverage through 2025 — [charm.li](https://charm.li/)
- Arizona emissions testing applies only in "Area A" (parts of Maricopa, Pinal, Yavapai counties) and "Area B" (part of Pima County); neither includes Cochise County — [A.R.S. 49-541](https://www.azleg.gov/ars/49/00541.htm)
- Arizona's test for vehicles in those areas includes an OBD check and a check-engine-lamp function check; 1967-and-later vehicles are in scope — [myazcar.com](https://www.myazcar.com/)
- Arizona statute defines "tampering" as removing, defeating or altering a factory emission control device — [A.R.S. 49-541](https://www.azleg.gov/ars/49/00541.htm)
- Generic code meanings relevant to this engine (P0440/P0441/P0446 EVAP, P0171 lean, P0300 misfire, P0325/P0330 knock sensor 1/2, P0420 catalyst, P1130/P1135 A/F sensor, P0401 EGR) — [Tacoma World code list](https://www.tacomaworld.com/threads/help-check-light-why.68572/)
- Toyota pre-1996 jumper method is TE1-E1 at the engine check connector; 1996+ needs a scan tool for engine codes — [troublecodes.net](https://www.troublecodes.net/toyota/)

### Inferences
All items below are [EJ]/[UR]; verify against the factory manual before encoding them as agent ground truth.
- **Swap artefacts to pre-declare in the vehicle profile.** Because the truck left the factory as a 2WD automatic, the profile must record which ECU and harness are installed now. If the automatic-transmission ECU is still fitted, expect standing transmission codes (shift solenoid circuit, e.g. P0750-series), park/neutral switch logic oddities (affects idle-up and starter interlock), and torque-converter-related PIDs that mean nothing. If a manual ECU was fitted, expect clutch-start switch wiring instead. Speed signal source (transfer case / ABS / combination meter) decides whether P0500 is real. The agent should treat codes on a user-maintained "known artefact" list as background and flag only changes.
- **ABS/SRS/4WD blink codes.** On OBD-II-era Toyotas the chassis ECUs still flash codes when TC is bridged to CG (DLC3 pins 13 and 4) [UR]. ABS is standard on MY2003 (sourced above), so an optocoupler or camera on the ABS lamp plus a fused jumper gives ABS codes with no scan tool. Whether a 2003 Tacoma still has the under-hood DLC1 (with TE1, E1, TC, +B, FP, VF1 terminals) is unconfirmed.
- **Ignition layout is a gift for sensing.** Three coils, each firing its own plug directly and a companion plug through a short HT lead [UR]. Three capacitive pickups on the three leads see all six events; one inductive pickup on a single lead is a clean cylinder-ID sync for relative compression, vacuum and audio analysis. Firing order 1-2-3-4-5-6 [UR].
- **Common 5VZ-FE fault patterns and the evidence that separates them** [UR for the fault list, EJ for the tests]:

| Fault | Typical codes | Distinguishing evidence from the pack |
|---|---|---|
| Coil / HT lead / plug | P030x pair on companion cylinders | Coil primary current ramp (low-amp clamp) flat or short; secondary pickup shows high firing kV or no spark line; misfire audible in tailpipe mic at half-order |
| MAF contamination | P0171, sometimes none | MAF g/s low at idle and at a WOT pull versus baseline; long-term trim positive at high flow, near zero at idle; wideband confirms lean under load |
| Vacuum leak | P0171 | Trim positive at idle and shrinking with RPM; manifold vacuum low/unsteady; ultrasonic mic or smoke + camera localises |
| Upstream O2 or A/F sensor / heater | P0125, P0133, P1130-P1135 | Heater current step with low-amp clamp; back-probed sensor switching rate (narrowband) or pumping current; compare against tailpipe wideband |
| EVAP (canister, VSVs, cap) | P0440, P0441, P0446 | Smoke test with camera; VSV coil resistance and click on command (mic); vent/purge valve current |
| Knock sensor or its under-intake pigtail | P0325, P0330 | Sensor circuit resistance/bias voltage at the ECU connector; scope the sensor while tapping the block; ignition timing pulled back (timing PID) and flat power |
| Timing belt wear / jumped tooth | none or cam/crank correlation | Crank vs cam sensor phase on two scope channels compared with the truck's own baseline; relative compression even but low on one bank |
| Valve cover / rear main / cam seal leaks | none | Thermal + visible camera, borescope; oil on HT leads causes misfire |
| Lower ball joints | none | Recall 05V225 status by VIN (sourced); play test with camera/IMU on the knuckle while user rocks the wheel |
| Fuel pump / filter / regulator | P0171 under load | Rail pressure transducer under load; pump current waveform (commutator ripple gives pump speed) |
| Catalyst | P0420 | Downstream O2 switching vs upstream; inlet/outlet temperature delta with thermal camera or thermocouples |

- **Legal ground truth for the agent.** Best to worst: (1) Toyota TIS Standard short-term subscription [UR: about USD 25-30 for two days historically], save PDFs of the 2003 Tacoma repair manual and EWD for personal use; (2) public-library databases (ChiltonLibrary, ALLDATA, Mitchell) if the owner's library card includes one; (3) a used paper factory manual set; (4) charm.li exists and covers the year range but its licence is unstated, so an open project should not bundle or scrape it.
- **Emissions relevance for this truck.** If the truck is registered in Cochise County it sits outside both Arizona test areas (statute sourced above), so smog-prep features serve other users of the project more than the owner. Readiness monitors still matter as a self-check that the engine management is healthy.

### Gaps
- No factory-manual page was read. Unverified: DLC1 presence on a 2003; TC-CG procedure and code tables for ABS/SRS; DLC3 pin 9 tach; fuel pressure spec and whether the rail has a test port (I believe it has none and needs a banjo adapter at the pulsation damper or filter [UR]); firing order; timing belt interval; knock sensor harness details; whether this truck's upstream sensors are A/F type (California/50-state calibrations differ).
- Toyota TIS prices and term lengths: not shown on the landing page.
- ChiltonLibrary / ALLDATA / Mitchell library access terms: Gale product URL returned 404; local library holdings not checked.
- Toyota readiness drive-cycle patterns: no source read.
- Tacoma World and Toyota-4Runner.org thread-level evidence on common faults: forum index pages gave nothing usable; Toyota-4Runner.org refused the fetch.
- NHTSA lists recalls for the model year, not whether this VIN had the ball-joint recall completed.

---

## 4. Non-OBD sensing methods: ranked, priced, agent-readable

### Takeaway
On a K-line truck the highest value per dollar comes from signals the pack can sample fast and tie to a cylinder: battery voltage and starter current during cranking, one ignition sync pickup, a low-amp clamp, manifold vacuum and fuel pressure transducers, and the microphones already in the pack. Professional versions of each method exist as PicoScope guided tests, which give the agent a public description of the procedure and of what "bad" looks like.

### Cited Findings

**Electrical**
- Relative compression: a high-amp clamp on either battery cable, fuel injection disabled, crank about 5 s; starter current rises as each cylinder comes up on compression, so a repeating low peak marks a weak cylinder; the test is comparative only — [PicoScope AGT-004](https://www.picoauto.com/library/automotive-guided-tests/charging-starting/starting/AGT-004-relative-compression-gasoline/)
- Alternator ripple: measure at the alternator B+ terminal (the battery damps the waveform); healthy output is a steady train of pulses; more than 500 mV peak-to-peak can upset other systems; a failed diode shows as sharp, usually downward spikes; missing pulses suggest windings or diodes — [PicoScope AGT-002](https://www.picoauto.com/library/automotive-guided-tests/charging-starting/charging/AGT-002-alternator-ripple-without-ecm-control/)
- Pico's charging/starting set also includes alternator voltage and current and a clamp-method parasitic drain test — [PicoScope guided tests: charging/starting](https://www.picoauto.com/library/automotive-guided-tests/charging-starting)
- Clamp and probe prices at a US automotive-scope dealer (AESwave, in stock): low-range current probe with BNC USD 80 (sale, list 99); high-amp probe with BNC USD 80 (sale, list 99); Pico 60 A clamp USD 133; Pico 600 A clamp USD 168; ESi 695 0-80 A probe USD 148; ESi 697 400 A probe USD 113; "Fuse Buddy" current loops USD 30; ESi 804 10-piece back-probe kit USD 42 — [AESwave current probes](https://www.aeswave.com/collections/lab-scopes-accessories-current-probes)
- Hantek CC-65 clamp: DC 10 mA-20 A at 100 mV/A and up to 65 A at 10 mV/A, AC/DC to 20 kHz, ±(1.5 % + 5 mA) on the low range, 9 V battery, 9 mm jaw, BNC or banana versions — [Hantek CC-65](https://www.hantek.com/products/accessories/CC-65)
- Ignition pickups (AESwave): capacitive secondary pickup from USD 45; capacitive COP paddle lead from USD 75; inductive trigger pickup from USD 80; Pico secondary pickup USD 65; Pico COP probe USD 165; universal secondary kit USD 79-85 — [AESwave ignition](https://www.aeswave.com/collections/lab-scopes-accessories-ignition)
- Hantek HT25: capacitive secondary-ignition clip for plug leads, up to 10,000:1, 2.5 m lead, BNC — [Hantek HT25](https://www.hantek.com/products/accessories/HT25)

**Pressure and flow**
- Running intake-manifold pressure: each induction stroke adds a small depression; a reduced or missing pull at regular intervals points to a worn intake cam lobe, blow-by, head gasket leak or a misfire; pressure bumps at regular intervals point to a leaking intake valve (or exhaust valve via EGR) or heavy blow-by via PCV; an overall shift suggests intake restriction, vacuum leak or blocked exhaust — [PicoScope AGT-427](https://www.picoauto.com/library/automotive-guided-tests/pressure-transducers/wps500x-pressure-transducer/AGT-427-intake-manifold-pressure-running/)
- Pico's pressure test set shows what is diagnosable with one transducer: crankcase pressure (cranking/running), exhaust pressure (cranking/running), intake manifold (cranking/running/snap), in-cylinder (cranking/idle/snap), oil pressure; a second sensor type (FirstLook pulse sensor) is used for exhaust, intake and crankcase pulses against an ignition sync — [PicoScope guided tests: pressure transducers](https://www.picoauto.com/library/automotive-guided-tests/pressure-transducers)
- Professional transducer prices: Pico WPS500X USD 973 bare, kits USD 1,327-1,952; Pico PQ334 14-bar transducer USD 420; fuel-pressure test hoses with Schrader fittings USD 42; exhaust adapter USD 51; vacuum tap USD 48; universal compression hose from USD 35 — [AESwave pressure transducers](https://www.aeswave.com/collections/lab-scopes-accessories-pressure-transducers)

**Sound and vibration**
- Commercial NVH reference: Pico NVH kits pair a mic (TA572, USD 243) and a 3-axis accelerometer (TA573, USD 337) with OBD speed (via a USD 575 J2534 interface) so software can match measured frequencies to wheels, engine orders, propshaft and bearings; propshaft balancing uses an optical sensor kit (USD 514) — [PicoAuto NVH](https://www.picoauto.com/products/noise-vibration-and-balancing/nvh-overview)
- Kit prices with a 4425A scope: NVH Essentials Starter USD 2,544, Standard USD 3,193, Advanced USD 4,476 — [AESwave PC scopes](https://www.aeswave.com/collections/lab-scopes-pc-based-scopes)

**Thermal and optical**
- Topdon TC001 thermal camera: 256x192, 25 Hz, -20 to 550 °C, USB-C video; listed at USD 289 and sold out on the manufacturer's US store (25 Nov 2026) — [Topdon](https://www.topdon.us/products/tc001)

**Chemical**
- 14point7 Spartan 3 Lite v2 wideband controller: USD 82.50 without sensor, USD 135 with Bosch LSU 4.9; 0-5 V linear output (0 V = 0.68 lambda, 5 V = 1.36 lambda), simulated narrowband output, UART, no CAN; 10 ms response — [14point7 product page](https://www.14point7.com/products/spartan-3-lite-v2)
- Its UART is 5 V logic, 9600 8N1, ASCII commands (GETHW, GETFW, SETTYPE, SETPERF, SETLAMZEROV/SETLAMFIVEV, DOCAL, MEMRESET ...); power is switched 12 V through a 5 A fuse, live only with the engine running; install pre-catalyst, about 2 ft from the exhaust port, sensor tilted between 10 and 2 o'clock; the manual does not address tailpipe use and says the product is not for public roads — [Spartan 3 Lite v2 manual](https://cdn.shopify.com/s/files/1/0189/1312/files/Spartan_3_Lite_v2_User_Manual_Dec_19_2023.pdf?v=1703126168)
- Spartan 3 v2 (full): USD 139 without sensor, USD 191.50 with LSU 4.9; adds CAN output and an on-board FTDI USB-serial converter; 10 ms response — [14point7 Spartan 3 v2](https://www.14point7.com/products/spartan-3-v2)
- Fuel-trim scaling can be wrong in borrowed definitions (the Jeep "22 % rich" error), which argues for an independent lambda measurement — [The Autopian](https://www.theautopian.com/i-guess-ai-can-fix-a-jeep-cherokee-xj-now/)

**RF**
- `rtl_433` decodes ISM-band devices at 315/433.92/868/915 MHz with JSON and MQTT output and includes TPMS decoders for Toyota ([88], and PMV-107J [110]), Schrader, Ford, Nissan, Honda and others; GPL-2.0 — [rtl_433](https://github.com/merbanan/rtl_433)

### Inferences

**Ranked list [EJ]. Costs marked "verified" come from the citations above; all others are estimates.**

**Phase 1 (cheap, high value; target USD 50-150 beyond the OBD adapter)**

| # | Method | Parts | Interface to the pack | Cost | What it answers |
|---|---|---|---|---|---|
| 1 | Cranking capture: battery voltage sag and ripple at 1-10 kS/s, plus cranking audio | Fused leads to the battery, resistive divider into an ADC/scope channel; existing mics | ADC or scope, shared clock | ~USD 5 | Battery/starter health, cranking RPM, even/uneven compression (ripple periodicity), slow-crank vs no-fuel vs no-spark triage. Works without a clamp. |
| 2 | Ignition sync pickup on one HT lead (and optionally all three) | Capacitive clip (AESwave from USD 45 verified; Hantek HT25 est. USD 15-25) or DIY foil/coax clamp | Scope channel or comparator to a GPIO timestamp | USD 5-45 | Cylinder identification for every other waveform; RPM at event rate, independent of K-line; spark presence and relative firing kV per coil |
| 3 | Low-amp current clamp | Hantek CC-65 (est. USD 60-75) or AESwave low-range probe (USD 80 verified) | Scope channel, 100 mV/A | USD 60-80 | Coil primary ramps, injector pintle hump, fuel pump commutator ripple (pump speed, worn segments), O2/A-F heater current, parasitic draw down to tens of mA, alternator output up to 65 A |
| 4 | Manifold vacuum transducer | 1-bar automotive MAP sensor or 0-15 psia 0.5-4.5 V sensor teed into a vacuum port (est. USD 15-25) | ADC, 1 kS/s or more | ~USD 20 | Vacuum level and steadiness, per-cylinder intake pulls (valve sealing, cam wear), restricted exhaust on snap throttle, cranking vacuum |
| 5 | Fuel pressure transducer | 0-100 psi 0.5-4.5 V stainless sensor, 1/8 NPT, plus the right adapter for this rail (est. USD 15-25 + adapter) | ADC | USD 20-50 | Key-on prime, running pressure, drop under load, leak-down after shut-off (regulator, check valve, injector leak) |
| 6 | Back-probe pins and fused jumpers | T-pins/back-probe set (ESi 804 USD 42 verified; generic est. USD 10-15) | Scope/DMM | USD 10-42 | TPS sweep glitches, MAF signal, O2 switching direct from the sensor, crank/cam correlation, voltage drop tests |
| 7 | Contact mic / stethoscope probe | Piezo disc on a magnet or a steel rod into an electret capsule (est. USD 5-10) | Audio ADC channel on the shared clock | ~USD 10 | Injector tick per cylinder, valvetrain and bearing noise localisation, idler/tensioner bearings |
| 8 | Thermocouples on exhaust runners | 2-6 K-type probes + MAX31855/MAX31856 boards (est. USD 8-12 per channel) | SPI | USD 20-60 | Dead cylinder (cold runner), catalyst inlet/outlet delta, cheaper and loggable while driving, unlike a hand-held thermal camera |

A sensible first purchase inside USD 150: items 1, 2 (DIY or HT25), 3, 4, 5 and generic back-probe pins.

**Phase 2**

| Method | Parts | Cost | What it answers |
|---|---|---|---|
| High-amp clamp (starter, charging) | AESwave high-amp probe USD 80 verified; Hantek CC-650 est. USD 75-95; Pico 600 A USD 168 verified | USD 80-170 | True relative compression in amps, starter draw, voltage drop per amp on cables, full alternator output |
| Wideband lambda with serial out | Spartan 3 Lite v2 + LSU 4.9, USD 135 verified; tailpipe clamp or a welded bung | USD 135-200 | Ground-truth lambda to audit fuel trims and scaling. A tailpipe clamp reads post-catalyst gas diluted at idle, so treat it as rich/lean under load, not as a smog number; a pre-cat bung is the accurate install. |
| Thermal camera | Topdon TC001 class (USD 289 at Topdon, sold out; earlier project notes had about USD 199) | USD 200-290 | Runner temperatures, radiator cold spots, dragging brake, bearing/U-joint heat after a drive, hot relays and connectors |
| Borescope | USB/Wi-Fi 5.5 mm dual-lens (est. USD 30-60) | USD 30-60 | Cylinder walls, piston crowns, valve faces through the plug hole |
| Smoke machine | DIY (glow plug or heating wick + mineral oil + low-pressure regulator, est. USD 30) or budget unit (est. USD 80-150) | USD 30-150 | EVAP and intake leaks; the camera spots the plume; pressure-decay logging quantifies leak size |
| Accelerometers | 3-axis digital IMU on the block, transfer case, diff, seat rail (est. USD 5-20 each) + GNSS speed | USD 20-60 | Order analysis: tyre order vs driveshaft order vs engine order; U-joint second-order vibration |
| Exhaust and crankcase pulse sensor | Piezo "pulse" sensor or a low-range differential pressure sensor in the tailpipe/dipstick tube (est. USD 15-40) | USD 15-40 | Misfire per cylinder at the tailpipe, leaking exhaust valve, ring blow-by per cylinder against the ignition sync |
| Ultrasonic leak listening | 40 kHz transducer or a 192 kS/s MEMS mic channel (est. USD 10-40) | USD 10-40 | Vacuum, exhaust-manifold and tyre leaks; arcing HT leads |
| Timing strobe + camera | Inductive timing light (est. USD 25-40) | USD 25-40 | Base timing and timing scatter on video |
| SDR ignition and RF checks | Existing SDR array | USD 0 | Broadband impulse from each spark as a non-contact spark indicator; alternator whine; key-fob presence; TPMS on vehicles that have it (rtl_433, sourced). A 2003 Tacoma probably has no direct TPMS [UR]. |

**Specialist / later**

| Method | Parts | Cost | Notes |
|---|---|---|---|
| In-cylinder pressure ("running compression") | Pico WPS500X USD 973 verified; budget in-cylinder transducers exist [UR] est. USD 100-300 | USD 100-1,000 | The richest mechanical test (valve timing events, exhaust restriction, ring seal) but rear-bank plugs on this engine sit under the plenum (sourced), so access is costly |
| Exhaust gas analysis | Used 4/5-gas shop analyzer (est. USD 300-1,500 plus sensor and calibration upkeep) or DIY NDIR CO2 + electrochemical O2 | USD 100-1,500 | DIY CO2/O2 gives combustion completeness and dilution; HC, CO at ppm level and NOx are hard to do credibly at hobby cost; numbers will not equal a dyno smog test (different load, different sampling) |
| Combustion-gas block test | Chemical tester (est. USD 30-50) + camera for colour change | USD 30-50 | Head gasket confirmation |
| Cooling system pressure with logging | Hand pump tester (est. USD 50-90) + the fuel-pressure transducer re-used | USD 50-90 | Leak-down curve, pressure pulses on running engine (gasket) |
| A/C pressures | Manifold gauges or two transducers | USD 50-100 | Out of core scope |
| Chassis: toe estimate, U-joint angles | IMU/inclinometer on driveshaft, pinion and transfer-case flanges; string or laser + camera for toe | USD 10-50 | Directly relevant to a 4WD swap: operating angles should be near-equal and opposite; mismatch produces a second-order driveshaft vibration |
| NVH kit benchmark | Pico NVH Essentials Starter USD 2,544 verified | n/a | Reference for what the open version should match |

**Cross-cutting [EJ]**
- Every waveform needs a cylinder reference. One ignition pickup on a known cylinder turns "one pull is weak" into "cylinder 4 is weak". Budget it first.
- On this K-line truck, derive RPM from the ignition pickup, alternator ripple or audio rather than polling it; reserve K-line polling for slow context (coolant, trims, timing, load).
- Voltage-only relative compression is weaker than the current method (battery internal resistance scales the ripple) but costs nothing and is the natural first agent-guided test.
- Mic-based detection is strongest as comparison against the same truck's baseline, not against a generic model.

### Gaps
- No academic source could be opened (arXiv returned errors three times; no search). Unverified leads to check: Siegel et al., "Engine Misfire Detection with Pervasive Mobile Audio" (2016) and related smartphone-audio vehicle diagnosis work from the same group; any Kaggle/Zenodo engine-fault audio datasets; published accuracy figures.
- Not priced from a page: Hantek CC-65/CC-650/HT25/HT201 street prices, generic pressure transducers, MAP sensors, borescopes, smoke machines, timing lights, used gas analyzers, AEM X-Series and Innovate LC-2 widebands (Innovate blocks automated fetches), Steelman ChassisEAR, cheap in-cylinder transducers. Amazon listings were not checked.
- No source read for: DIY NDIR accuracy against smog-test numbers, SDR-based spark detection, cranking-audio RPM estimation, mic-based knock detection, spark-plug reading from photos.
- Whether a wideband in a tailpipe clamp gives usable readings on this truck was not sourced; the maker's manual only covers pre-catalyst installation.

---

## 5. Automotive oscilloscope options that are open / agent-readable, and probe accessories

### Takeaway
For headless, agent-driven capture on a Pi-class board, the only cheap automotive-adjacent scope with confirmed open tooling is the Hantek 6022BE (libsigrok `hantek-6xxx`, OpenHantek6022 with Raspberry Pi packages), but it is 8-bit, two-channel, software-triggered and limited to ±5 V at the input, so attenuators are mandatory. The Hantek 1008C offers eight 12-bit channels on paper but is Windows-first; PicoScope is the reference tool with the best library, but costs USD 1,057+ for automotive models and does not list arm64 Linux drivers.

### Cited Findings
- Hantek 6022BE: 2 channels, 8-bit (AD9288), 20 MHz, up to 48 MS/s, 1 MΩ inputs, full-scale ranges of ±5 V down to ±0.5 V; libsigrok driver `hantek-6xxx`, status supported, needs fx2lafw firmware 0.1.4+ loaded at each plug-in; software trigger only — [sigrok wiki](https://sigrok.org/wiki/Hantek_6022BE)
- OpenHantek6022: GPL-3.0, 1.1k stars; supports 6022BE/BL and clones; Linux is the supported OS with Raspberry Pi packages; sample rates 100 S/s to 30 MS/s (48 MS/s unsupported due to USB instability); gain steps give ±5 V max at 1x; export to CSV/JSON only from the GUI, no headless capture option listed — [OpenHantek6022](https://github.com/OpenHantek/OpenHantek6022)
- Hantek 1008C: 8 channels, 12-bit, 2.4 MS/s, 1 MΩ, 400 V max (DC + AC peak), USB-powered, programmable 8-channel LVTTL generator to 250 kHz that can simulate crank/cam signals; Windows software; an SDK download exists but is undocumented on the page; ships with one ignition probe — [Hantek 1008C](https://www.hantek.com/products/automotive-diagnostic-equipment/Hantek1008C)
- Hantek's current automotive range: 1008B/1008C, 6074BE kit, DSO3064 kits, 2D82AUTO handheld; accessories CC-65 and CC-650 clamps, HT25 ignition probe, HT25COP coil-on-plug probe, HT201 20:1 attenuator, HT30A leads, HT306 six-way breakout leads; no prices on the manufacturer site — [Hantek products](https://www.hantek.com/products)
- PicoScope automotive prices (AESwave, US): 2-channel 4225A scope USD 1,057, starter kit USD 1,208; 4-channel 4425A scope USD 1,691, starter kit USD 1,989, standard kit USD 3,514 — [AESwave PC scopes](https://www.aeswave.com/collections/lab-scopes-pc-based-scopes)
- Pico Linux support: Ubuntu and openSUSE on x86-64 officially; drivers and APIs (not the GUI) for 32-bit Raspberry Pi OS (armhf) with limited testing; arm64 is not mentioned — [Pico Linux downloads](https://www.picotech.com/downloads/linux)
- PicoScope 2204A (general-purpose, not automotive): 2 channels, 10 MHz; price not on the page, a customer review cites GBP 99 with probes; the PicoScope Automotive software has 160+ guided tests but is tied to automotive models — [Pico 2000 overview](https://www.picotech.com/oscilloscope/2000/picoscope-2000-overview); [PicoAuto guided tests](https://www.picoauto.com/library/automotive-guided-tests)
- The project's earlier notes already established sigrok coverage (fx2lafw analyzers, Pico running sigrok-pico, Hantek 6022BE) and that no cheap USB scope is isolated — `measurement_instruments.md` in this folder, citing [sigrok supported hardware](https://sigrok.org/wiki/Supported_hardware)

### Inferences
- [EJ] **Most value for about USD 100:** if the pack already has a scope-class input (6022BE or equivalent on sigrok), spend on probes: one low-amp clamp (CC-65 class, est. USD 60-75), two 20:1 attenuators (HT201, est. USD 10-15 each), one capacitive ignition clip (HT25, est. USD 15-25), generic back-probe pins (est. USD 10). If there is no scope yet, a Hantek 6022BE (est. USD 65-80) + one attenuator + DIY ignition clip + back-probe pins fits USD 100 and the clamp moves to the next purchase.
- [EJ] For automotive work, channel count and resolution matter more than bandwidth: almost everything of interest is below 100 kHz. A 16-bit multi-channel audio-rate or 100 kS/s ADC (USB audio interface with DC-coupled inputs, or an ADS131/ADS8688-class board) on the pack's shared clock would beat an 8-bit scope for cranking, vacuum, pressure, ripple and current-ramp work, and it streams continuously, which a USB scope in sigrok does not do well.
- [EJ] 6022BE cautions: ±5 V input means a 12 V system clips at 1x; injector and coil primary flyback reaches 60-400 V, so always use the 20:1 attenuator or a 10x/100x probe; scope ground is USB ground, so on a battery-powered Pi the pack floats (good), but two grounded leads at different chassis points can pass starter return current through the leads (bad). One ground reference, short, at the battery negative.
- [UR] Hantek 1008C has community Linux/Python drivers on GitHub but is not in mainline sigrok; treat as a reverse-engineering task. FNIRSI and Owon handhelds, DSO138: closed or screenshot-only data paths, poor fit for an agent.
- [EJ] The 1008C's crank/cam signal generator idea is worth copying in the pack's own firmware: a simulated crank/cam output lets the agent test an ECU input circuit or the pack's own decoder without the engine running.

### Gaps
- Street prices for every Hantek item (6022BE, 1008C, 6074BE kit, CC-65, CC-650, HT25, HT25COP, HT201) and for PicoScope 2204A in USD were not verified; Amazon was not checked.
- sigrok status of Hantek 1008C and 6074BE, and libps2000/4000a availability on arm64, not confirmed.
- No read of FNIRSI, Owon HDS, DSO138 documentation this session.

---

## 6. Waveform libraries and reference data an agent could use legally

### Takeaway
Free, public procedure text exists (PicoScope guided tests), open per-vehicle signal definitions exist (OBDb, CC-BY-SA), and open protocol definition sets exist for some legacy ECUs (ChryslerScanner, GPLv3), but there is no confirmed open, redistributable known-good waveform library. The practical answer is the truck's own recorded baseline plus licensed manual access for specifications.

### Cited Findings
- PicoScope publishes 160+ guided tests on the public web with connection instructions, example waveform descriptions, typical values in some cases (e.g. the 500 mV ripple limit) and fault interpretations; categories: charging/starting, sensors, actuators, ignition, communications, system tests, pressure transducers — [PicoAuto guided tests](https://www.picoauto.com/library/automotive-guided-tests)
- Pico's Waveform Library holds "thousands" of user-uploaded waveforms as reference material with free cloud storage; the page read does not state access conditions or export rights — [PicoAuto waveform library](https://www.picoauto.com/library/waveform-library)
- OBDb/Toyota-Tacoma: community signal-set definitions organised by model year under `signalsets/v3`, licence CC-BY-SA-4.0, 1,304 commits — [OBDb Toyota-Tacoma](https://github.com/OBDb/Toyota-Tacoma)
- ChryslerScanner (GPL-3.0) is the definition source xj-scanner re-used for parameter IDs, scaling and code text; one of those scalings was wrong for that ECU — [xj-scanner](https://github.com/MZULALI/xj-scanner); [The Autopian](https://www.theautopian.com/i-guess-ai-can-fix-a-jeep-cherokee-xj-now/)
- Scraped DTC databases are a known weak point: the author of one OBD MCP server says his is scraped from public references and not authoritative — [dev.to](https://dev.to/petr_patek_12/diy-ai-car-diagnostics-with-a-15-bluetooth-adapter-and-python-3fhk)
- Paid training references sold alongside scope gear: Brandon Steckler's "Pressure Waveform Acquisition and Analysis from the Inside Out" from USD 89; John Thornton's in-cylinder pressure transducer book USD 165; "Engine Relationship Charts" USD 24 — [AESwave](https://www.aeswave.com/collections/lab-scopes-accessories-pressure-transducers)
- NHTSA exposes recalls by make/model/year through a public JSON API — [NHTSA API](https://api.nhtsa.gov/recalls/recallsByVehicle?make=toyota&model=tacoma&modelYear=2003)
- Factory data routes: Toyota TIS Standard (manuals, wiring, TSBs) — [Toyota TIS](https://techinfo.toyota.com/); charm.li with unstated licence — [charm.li](https://charm.li/)

### Inferences
- [EJ] Treat Pico's guided-test pages as reading material the agent may consult and link to at run time; do not copy their text or images into the repository.
- [EJ] Ship an open "test recipe" format instead (YAML/JSON): connection points, sample rate, duration, pass/fail features, safety notes, with the project's own wording. Seed it from first principles and from the owner's truck.
- [EJ] Ship a waveform exchange format with metadata (vehicle profile, sensor, gain, sync channel, engine temperature, altitude) and invite contributions under CC-BY-SA or CC0. This is the missing public asset and a reason for others to adopt the project.
- [EJ] Definitions files (PIDs, scaling) need per-entry provenance and a "validated on" list, because the one documented LLM-diagnosis error in the prior art came from an unvalidated scaling.
- [EJ] OBDb likely has little for a 2003 K-line Tacoma (its focus is CAN-era vehicles); check `generations.yaml` before relying on it.

### Gaps
- Not read: Pico Waveform Library terms (it has historically required a registered automotive PicoScope [UR]), iATN, Autonerdz, rotkee.com, any openly licensed DTC database (candidate repos on GitHub not verified), the "Engine Performance Diagnostics" reference.
- OBDb coverage for 2001-2004 Tacoma not confirmed.

---

## 7. AI-first workflow design, emissions-test prep, and safety

### Takeaway
The prior art points to five design rules: simulate first, verify every physical connection before trusting or driving it, keep reads free and gate anything that actuates or erases, never state a diagnosis beyond what the data shows, and give the model compact, provenance-tagged summaries rather than raw streams. The sensor pack's unique contribution is synchronized multi-modal evidence compared against the same vehicle's healthy baseline.

### Cited Findings
- xj-scanner's agent rules: prove the laptop side on a simulated vehicle; meter every connector cavity key-on before connecting; stop if measurements differ from expectations; explicit confirmation with an explanation for erasing codes (which also wipes learned fuel), resetting trims and actuator tests (key on, engine off, short runs, announce what will click or spray); do not guess at a diagnosis; check the parameter doc before trusting odd readings; a `summarize.py` tool condenses logs — [AGENTS.md](https://github.com/MZULALI/xj-scanner/blob/main/AGENTS.md)
- Wire colour is not evidence: on the Jeep, ground and 12 V wires were the same colour — [xj-scanner](https://github.com/MZULALI/xj-scanner)
- MCP as an audit boundary: typed tool inputs/outputs let the author tell tool-data errors from model-reasoning errors; read-only profile by default, `clear_dtc` guarded and unused, actuator tests/coding/resets excluded, mock mode for development — [dev.to](https://dev.to/petr_patek_12/diy-ai-car-diagnostics-with-a-15-bluetooth-adapter-and-python-3fhk)
- Blind tuning without a root cause made the Jeep's emissions worse and cost about USD 100 per retest — [Tom's Hardware](https://www.tomshardware.com/tech-industry/artificial-intelligence/1994-jeep-cherokee-owner-hacks-32-year-old-car-with-claude-ai-to-pass-california-smog-tests-enthusiast-reverse-engineered-the-ancient-ecu-for-diagnostics-and-control)
- A physical gauge was needed where the ECU had no sensor (fuel pressure) — [The Autopian](https://www.theautopian.com/i-guess-ai-can-fix-a-jeep-cherokee-xj-now/)
- Pico's guided tests are explicitly a basis for further investigation, not a verdict, and warn that each vehicle may need different settings — [PicoScope AGT-004](https://www.picoauto.com/library/automotive-guided-tests/charging-starting/starting/AGT-004-relative-compression-gasoline/)
- Alternator ripple should be measured at B+, not at the battery, because the battery hides faults: an example of a placement rule the agent must enforce — [PicoScope AGT-002](https://www.picoauto.com/library/automotive-guided-tests/charging-starting/charging/AGT-002-alternator-ripple-without-ecm-control/)
- Arizona: testing only in the Phoenix and Tucson areas; the test includes an OBD check and MIL function check; repair assistance up to USD 550 for gasoline vehicles after the owner pays the first USD 150; free gas-cap replacement in Maricopa and Pima counties — [myazcar.com](https://www.myazcar.com/); [ADEQ VECS](https://azdeq.gov/VECS)
- Spartan wideband: power only while the engine runs; do not plug or unplug the sensor while powered — [Spartan manual](https://cdn.shopify.com/s/files/1/0189/1312/files/Spartan_3_Lite_v2_User_Manual_Dec_19_2023.pdf?v=1703126168)
- Earlier project notes: practitioners advise numeric readings over screenshots and warn that an assistant cannot verify probe attenuation, limits or grounding — `prior_art.md` in this folder

### Inferences
All [EJ].

**Vehicle profile file (per vehicle, versioned)**
- Identity: year, model, engine, ECU part number and calibration ID, protocol, K-line polling budget.
- Modifications: original and current transmission and drivetrain, which ECU/harness is installed, deleted or added sensors, tyre size and axle ratio (for speed-based order analysis).
- Known artefact codes and PIDs with reasons, so the agent discounts them and reports only changes.
- Sensor-pack hookup map for this vehicle: where each probe goes, connector views, photos of the correct placement, expected key-on voltages per pin.
- Specs with provenance (source document, page), and a "validated on this vehicle" flag per PID scaling.

**Guided test plans**
- Each test is a recipe: goal, preconditions (engine cold/hot, key state, transmission in neutral, wheels chocked), probe placements, capture settings, features to extract, safety notes.
- Placement verification loop: the agent shows a reference photo, the user clips on, the camera frame goes back to the model ("is the clamp around the single cable to the starter, arrow toward the starter?"), then an electrical sanity check confirms it (key-on voltage in the expected window, clamp reads near zero at rest and positive on load, ignition pickup shows one event per two crank revolutions). Vision alone is not sufficient; the electrical check is the gate.
- Old-school order built in: the first plan is "fuel, spark, air, compression" in under ten minutes: cranking capture (voltage, audio, ignition pickup), fuel pressure prime, manifold vacuum, then branches.

**Baselines**
- Record a healthy-truck session set: cold start, warm idle, 2,500 rpm hold, snap throttle, cranking with fuel disabled, a standard drive loop, charging system under load. Store features (not only raw files) per test.
- Re-record after every repair so the baseline tracks the truck.

**Capture-on-event for intermittents**
- Continuous ring buffer (30-120 s) on all fast channels; triggers: misfire counter change, new pending DTC, knock-retard jump, voltage dip, audio anomaly score, driver button or voice mark ("mark that").
- Persist the window with pre-trigger, GNSS speed and position, and the slow K-line context.

**Cross-modal correlation examples**
- Knock or ping: audio band energy near the block's knock frequency, gated by the ignition sync, against RPM/load and the timing-advance PID. Audible knock with no timing pull points at the knock sensor circuit; timing pull with no audible knock points at a noisy sensor or mechanical rattle.
- Slow or no start: cranking RPM from audio and ripple, battery sag, starter current, ignition pickup events, fuel pressure rise. One capture separates battery/starter, no spark, no fuel and low compression.
- Misfire: K-line misfire counter (slow) against coil current ramp per coil, secondary pickup, tailpipe pulse and runner temperature. Agreement across three sensors identifies the cylinder; disagreement identifies a sensor or scaling problem.
- Driveline vibration after the 4WD swap: accelerometer spectrum against GNSS speed; a peak that tracks driveshaft speed (tyre speed x axle ratio) at first order suggests imbalance or runout, second order suggests U-joint angle or phasing.
- Charging complaint: ripple at B+, alternator current, battery voltage, belt-squeal audio on load step.

**Presenting evidence to a vision LLM**
- Never the raw stream. Give a JSON summary per test (features, units, baseline values, deltas, confidence, provenance) plus one composite image when shape matters: stacked panels on a shared time axis (audio spectrogram, RPM, voltage/current, pressure), cylinder markers from the ignition sync, baseline overlaid in grey.
- Angle-domain resampling (0-720 degrees) overlays for per-cylinder comparison are easier for a model to read than time-domain traces.
- Let the model request zoomed re-renders or raw windows through MCP tools; keep images under the model's useful resolution and label axes explicitly.
- Every number carries how it was obtained; the tool layer flags clipped, unsynchronised or unvalidated channels so the model can decline to conclude.

**Tool tiers**
- Free: read, capture, analyse. Confirm with explanation: clear codes (state that readiness monitors and fuel trims reset), actuator tests, anything that energises a pump, coil or injector. Not offered by default: ECU writes, calibration changes, emission-control defeat.

**Emissions-test prep**
- Before a test: no MIL, no pending codes, readiness monitors complete, fuel trims within a modest band, wideband or sensor data showing closed-loop operation, catalyst temperature delta sensible, EVAP smoke test if EVAP codes ever appeared, gas cap seal check.
- Never clear codes shortly before a test; monitors reset and the vehicle shows "not ready".
- On a failed report: map each failed gas to mechanisms (HC: misfire, lean misfire, oil; CO: rich; NOx: lean, EGR inoperative, high combustion temperature, advanced timing, tired catalyst) and test the mechanism rather than adjusting fuelling.

**Safety rules for a running engine with leads and a carry case**
- Outdoors or with exhaust extraction; never run the engine in a closed space; a CO alarm in the case is cheap.
- Route every lead away from belts, fan, pulleys and exhaust before starting; the agent asks for a camera check of lead routing as a precondition for any engine-running test. Remove loose clothing, jewellery, lanyards.
- Case sits on the ground or a fender cover, never on the engine or battery, never under the hood during a drive unless strapped; nothing in the driver's footwell; no probe, cable or person near the pedals; a helper, not the driver, watches the screen.
- Parking brake on, wheels chocked, transmission in neutral (manual), before any remote or agent-prompted cranking; with the clutch-start interlock status of the swap recorded in the profile.
- Fused test leads at the battery; one ground point; never back-probe airbag (yellow) connectors; never pierce insulation where a back-probe will do; seal any pierced point.
- Secondary ignition is tens of kV: capacitive clips only, fitted with the engine off; the 20:1 attenuator on any primary or injector measurement.
- Fuel work: relieve pressure, eye protection, rag around fittings, extinguisher at hand, no fuel-pressure sensor installation on a hot engine.
- Hot parts: thermal camera and thermocouples before hands; cooling system closed when hot.
- The agent states the stop condition before each test and stops on any out-of-family reading (the latched-stop pattern noted in `prior_art.md`).

### Gaps
- No source read on Toyota readiness drive cycles, on state rules about monitor readiness counts, or on how to interpret an Arizona failed-test report.
- No published evaluation was found of how well current vision LLMs read spectrograms, scope traces or probe-placement photos; the presentation advice above is untested judgement.
- No source on shop-safety standards was read; the safety list is practice-based judgement.

---

## 8. Ten outside-the-box ideas for AI-first old-vehicle diagnosis

### Takeaway
The pack's edge over a scan tool is time-aligned multi-sensor evidence and an agent that can direct the human; the ideas below lean on that rather than on more OBD data.

### Cited Findings
- The building blocks these ideas rely on are documented above: three-coil wasted-spark layout ([Wikipedia](https://en.wikipedia.org/wiki/Toyota_VZ_engine)), blink-code timing ([troublecodes.net](https://www.troublecodes.net/toyota/)), starter-current compression signature ([PicoScope AGT-004](https://www.picoauto.com/library/automotive-guided-tests/charging-starting/starting/AGT-004-relative-compression-gasoline/)), intake-pulse interpretation ([PicoScope AGT-427](https://www.picoauto.com/library/automotive-guided-tests/pressure-transducers/wps500x-pressure-transducer/AGT-427-intake-manifold-pressure-running/)), crank/cam signal simulation as a product feature ([Hantek 1008C](https://www.hantek.com/products/automotive-diagnostic-equipment/Hantek1008C)), demo-vehicle-first agent playbooks ([xj-scanner AGENTS.md](https://github.com/MZULALI/xj-scanner/blob/main/AGENTS.md)).

### Inferences
All [EJ]; none verified as novel.
1. **"Engine fingerprint" from one cranking event.** A single 5-second fuel-disabled crank captured on every channel (voltage, current, audio, vacuum, crankcase pulse, ignition sync) and resampled to crank angle becomes a one-page health card. Re-run monthly; the agent reports drift per cylinder before any code sets.
2. **Camera-read blink codes with zero wiring.** The agent tells the user which two pins to bridge, watches the dash lamp through the camera, decodes the flashes and timing, and reads ABS/SRS/engine codes on any pre-CAN Toyota or Honda. It doubles as a universal "legacy dashboard reader" (gauges, warning lamps, odometer) for cars with no data port at all.
3. **Acoustic time-of-arrival localisation under the hood.** With a synchronized mic array, a tick, squeal or hiss can be triangulated to a region of the engine bay and drawn onto the camera image as a heat spot, an "acoustic camera" for bearing, injector, exhaust-leak and vacuum-leak hunting. The ultrasonic band makes leaks stand out.
4. **Tap-test teaching.** The agent asks the user to tap named components with a screwdriver handle while it records; it learns the transfer path from each location to each mic and accelerometer, then uses that to attribute later noises. The same tap on the block verifies the knock sensor circuit on a scope channel.
5. **Wasted-spark trick for compression without a transducer.** On a wasted-spark engine the firing voltage of the compression-stroke plug is higher than its exhaust-stroke partner. Comparing the secondary pickup's two events per coil over a cycle gives a relative in-cylinder pressure indicator from outside the engine, including the rear bank that is buried under the plenum.
6. **The pack as a signal source, not only a listener.** Inject a known stimulus and observe the response: a simulated crank/cam pattern to test the ECU's inputs with the engine off, a small current step to measure battery internal resistance, a tone through a speaker to find resonant heat shields and loose trim, a smoke pulse with pressure-decay timing to size an EVAP leak.
7. **Narrated test drives.** The driver talks ("there, that shudder") while the ring buffer runs; speech-to-text timestamps on the shared clock become event markers, so the model analyses exactly the seconds the human noticed, with GNSS speed, accelerometer orders and audio aligned. No screen interaction while driving.
8. **Differential diagnosis as a wager.** Before each test the agent writes down its candidate faults with probabilities and what each predicts for the next measurement; the tool layer scores the predictions. This exposes over-confident reasoning early, picks the next test by expected information per minute of user effort, and leaves an audit trail that makes "the AI guessed" visible.
9. **Swap-aware digital twin of the harness.** Photograph every connector and splice of the conversion once; the agent builds a pin-level map of what is factory, what is adapted and what is unused, tied to the wiring diagram. Later, "P0500" opens the actual speed-signal path on this truck with photos of each test point, and the profile is shareable with others doing the same 2WD-auto to 4WD-manual conversion.
10. **Community baseline exchange keyed by engine, not by car.** Owners upload angle-domain fingerprints and fault-confirmed captures for an engine family (5VZ-FE, Jeep 4.0, small-block Chevy) under an open licence. The agent compares a truck against the population as well as its own past, and each confirmed repair becomes a labelled example. This is the open counterpart of the vendor waveform libraries and the dataset the audio-diagnosis research field lacks.

### Gaps
- None of the ten was checked against existing products or patents; the wasted-spark firing-voltage method (idea 5) is a known scope technique in principle but its reliability with a cheap capacitive pickup is untested.
