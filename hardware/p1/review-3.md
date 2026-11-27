# Sawbones Labs P1: third independent hardware safety review

Reviewed 26 Nov 2026 against branch `hw/p1-design-ci` (hardware/p1, ci/check_hardware.py, docs/ROADMAP.md, docs/THREAT-MODEL.md, issue #144, firmware/trust-root). Process: I formed my own view from the design docs and the data first, derived the load paths from `netlist.toml`, checked the datasheets, and ran a time-step simulation of both charge pumps (Schottky and BJT exponential models, worst-case corners). Only after that did I read `independent-review.md` (review 2). Nothing was bought, pushed, posted or labelled.

```
REVIEW 3: APPROVE WITH CHANGES
```

The architecture is still right: NC1 in the power path, two dead-man relays driven by two different chips, an active-high low-side MOSFET, and no GPIO on a relay input. But **three of the review-2 fixes are wrong or add a new fault**, and the checker passes all of them, because every number it checks is typed in by hand rather than worked out:

1. **The SCR crowbar trips at about 6 V to 16 V, not 5.32 to 5.66 V.** The TL431's anode sits on the SCR gate, and Vref is measured from REF to the anode. With a typical BT151 it fires near 7.8 V, which is above the MAX813L's 6.0 V and the ME6217's 7.0 V absolute maximums. With a worst-case BT151 it may not fire even at 12 V.
2. **When the buck is still working (trimpot bumped high), the crowbar current never passes through F2.** The LM2596 current-limits at 3.5 to 7.5 A into the SCR at about 1.3 V, so F2 sees about 0.5 A and never blows. The SCR then dissipates 5 to 10 W with no heatsink and fails, possibly open.
3. **Variant B's Q64 gives a glitched or compromised RP2350 a way to reprogram the ATtiny over ISP.** Q64 holds the ATtiny RESET low for 140 to 280 ms after every power-up and every watchdog timeout, and the RP2350 is wired to the ATtiny's ISP pins.
4. **K2 does not meet the 120 ms bound.** The real figure is 125 to 141 ms once relay release time and the hold tail are counted. The A14 bound (30 ms) is stale: the real figure is about 140 to 270 ms.
5. **E-stop release can re-arm rail A after a single sense fault.** ARM_SENSE sits downstream of NC1, so releasing the e-stop looks like an arm off-then-on cycle.
6. **The AI host still has a possible write path, and it records the proof.** The logic analyzer is hosted on the Pi and probes the relay IN pins. Its FX2 firmware is uploaded by the host. HW-030 cannot see this.
7. **The mandatory per-cut self-test (B5) is wrong.** It expects "K1 dropped gives MID live", which is the signature of K2 open. With identical unkeyed 4-pin XH plugs, the self-test would pass a bench whose relay modules are swapped and fail a correct one.

Do not solder the PWR-board crowbar, the WD-board variant-B section or the relay headers, and do not order M6 as listed, until R3-B1 to R3-B7 are in.

---

## 1. Blocking findings

### R3-B1. The TL431 "anode to gate" crowbar does not trip at 5.32 to 5.66 V

**Evidence.** The TL431 regulates REF relative to its **anode** (TI TL431 datasheet: REF is the "threshold relative to common anode"). In C109c/C109d the cathode is on +5 V and the anode drives the BT151 gate through 220 ohm, so the anode rises with the gate. The SCR fires when:

```
V5 x R2/(R1+R2) - Vref  >=  VGT + IGT x 220 ohm
=> V5_trip = (1 + R1/R2) x (Vref + VGT + IGT x 220)

Divider (12.0 k / 10.0 k, 1 %): ratio 2.176 to 2.224.  TL431A Vref: 2.470 to 2.520 V.
BT151-500R: IGT 2 mA typ, 15 mA max; VGT 0.6 V typ, 1.5 V max (25 C); VGT 0.25 V min at 125 C.

Most sensitive SCR (hot): 2.176 x (2.470 + 0.25 + ~0.04) =  6.0 V
Typical SCR:              2.200 x (2.495 + 0.60 + 0.44)  =  7.8 V
Worst-case SCR:           2.224 x (2.520 + 1.50 + 3.30)  = 16.3 V
At 12 V on the rail: 12/2.2 - 2.495 - VGT = 1.46 to 2.36 V across 220 ohm, so IG = 6.6 to 10.7 mA.
That is below the 15 mA IGT max, so a shorted buck may never fire a worst-case part.
```

The data claims 5.32 to 5.66 V (`[[clamp]] CL1`). That is the formula for a TL431 with its anode grounded. Even the typical 7.8 V is above the MAX813L abs max (6.0 V), the MCP1700 (6.5 V) and the RP2350-Zero's ME6217 (7.0 V abs max, 6.5 V operating).

**Fix (PNP-buffered TL431 crowbar; all parts are in the 2N3904/2N3906 packs and the assortment):**

- TL431A anode to **GND**; REF from 12.0 k (to +5 V) / 10.0 k (to GND), 1 %; 1 nF REF to GND (keep).
- TL431A cathode → **1.0 kohm** → base of **Q_CB, 2N3906 PNP**. Emitter on +5 V (TB-5V side of the crowbar). **680 ohm** base to emitter, so the PNP stays off until the TL431 sinks about 0.9 mA, which is inside its regulating region (Imin ≤ 1 mA).
- Q_CB collector → **100 ohm** → BT151 gate. **1.0 kohm + 10 nF** gate to cathode (dV/dt and leakage immunity; datasheet dV/dt rises from 50 to 200 V/µs with RGK = 100 ohm).
- Trip = Vref × (1 + R1/R2) + Iref × R1 = **5.37 to 5.66 V** (Iref ≤ 4 µA adds ≤ 0.05 V). Gate drive at trip: (5.4 − 0.2 − 1.5)/100 = **≥ 37 mA**, more than twice the 15 mA IGT max.
- M2 must reach **0 to 7 V** (not 0 to 5.5 V) so the acceptance ramp can pass 5.66 V. Accept: fires at 5.37 to 5.66 V; does not fire at 5.20 V for 10 minutes; fires at 12 V with the buck removed and the rail fed from the adapter through F2.

**Checker rule (HW-029, rewritten).** Give `[[clamp]]` its topology: `trigger = "tl431-gnd-pnp" | "tl431-anode-gate" | "zener"` plus `r_top_ohm`, `r_bot_ohm`, `r_tol`, `vref_min_v`, `vref_max_v`, `igt_max_ma`, `vgt_max_v`, `r_gate_ohm`. Compute trip_min and trip_max from these, and **FAIL if the computed numbers differ from the declared `trip_min_v`/`clamp_v_max`**. FAIL `tl431-anode-gate` outright unless `(trip_max computed with VGT and IGT x Rg) < min(abs_max_supply_v)`.

### R3-B2. Crowbar current from a live buck bypasses F2; the SCR is not rated to hold it

**Evidence.** Row A37 (trimpot bumped high) and any regulating-high fault: the LM2596 keeps regulating into the fired SCR. Its switch current limit is 3.5 to 7.5 A over temperature, and it folds back to about 30 kHz into a short (onsemi LM2596 datasheet).

```
SCR: 4 to 7 A x VT ~1.3 V            = 5 to 9 W continuous
TO-220 in free air, Rth j-a 60 K/W   => a 300 to 550 K rise: the part is destroyed
F2 input current: 1.3 V x 5 A / (12 V x ~0.7) ~ 0.8 A, which never reaches 135 % of 1 A
```

A destroyed SCR can fail open, and then the high rail comes back. Review 2's claim that "the SCR survives the adapter's short-circuit current indefinitely" is wrong for this case. It is also only true for a few seconds in the shorted-buck case: 5 A × 1.4 V = 7 W.

**Fix.** Add **F3, a 1 A ATO** (the third holder from the #15 pack, with a genuine Littelfuse 257 1 A fuse) between **buck OUT+ and TB-5V**, with the crowbar and every load **downstream** of F3. Into F3 the LM2596 limit is ≥ 3.5× its rating, which clears in 0.02 to 0.5 s (Littelfuse 257: 350 % gives 0.020 to 0.5 s). Keep F2 for the shorted-switch case. Fit a small clip-on TO-220 heatsink as a margin. If the Pololu D24V10F5 is chosen instead of the LM2596, check that its current limit is ≥ 3.5 A, or F3 will not clear. If it is not, use a 500 mA fast 5×20 mm fuse for F3.

**Checker rule (HW-029 / new HW-034).** For every `[[clamp]]`, walk the netlist from the clamp to every source **including converters** (`kind = buck` gets `max_fault_a` and `min_fault_a` = its current limit). FAIL unless each source path crosses a fuse whose clearing time at the **minimum** fault current (not `max_fault_a`) is ≤ the clamp's `rated_ms`. Also FAIL on `rated_ms = 0` or a missing `rated_ms`: today `c.get("rated_ms", 0) and ...` silently skips the time check, and CL1 has `rated_ms = 0`. Fault current for clearing must use `min_fault_a` (fold-back and hiccup), and the clamp's surge rating must use `max_fault_a`. Today one number, an UNVERIFIED 5 A, is used for both.

### R3-B3. The K2 cut misses the 120 ms bound; the dropout figures are wrong

**Evidence** (time-step simulation; BAT43 VF 0.26 V typ / 0.33 V max at 2 mA; 2N3904 β 150 typ / 60 worst-case at 0 C; Vbe 0.65 / 0.70 V at 1 mA; ATtiny VOH 3.3 / 3.05 V; film −10 %, electrolytic +20 %):

| Stage | HOLD | Ic capability | Dropout: last edge to Q sink < relay hold current (0.15 to 0.5 mA) |
|---|---|---|---|
| Pump 1 set 1 (1 µF/10 µF/47 k/10 k, 50 Hz) | 1.21 to 1.26 V | 2.9 to 8.3 mA | 93 to 128 ms |
| Pump 1 set 2 (2.2 µF/22 µF/47 k/4.7 k) | 1.32 to 1.42 V | 7.3 to 22 mA | 194 to 246 ms |
| Pump 2 as designed (220 nF/1 µF/100 k/4.7 k, 0.9 to 1.1 kHz) | 1.62 to 1.89 V | 11 to 36 mA | **13 to 21 ms** (data: 8 to 15) |
| Pump 2 proposed (220 nF/**470 nF**/**47 k**/4.7 k) | 1.62 to 1.80 V | 11 to 34 mA | **7.6 to 9.4 ms** |

The relay releases only when the coil falls below its hold level. The SRD-05VDC-SL-C is guaranteed to release only at ≤ 10 % (0.5 V), and its release time is ≤ 10 ms. The module opto plus driver keeps the coil held at Q sink currents far below the 2 to 3 mA pull-in current, so the tail is long.

```
K2 bound, as designed: 2 x 50 ms Q&A x 1.10 (factory RC +/-10 %) + 21 ms pump + 10 ms release = 141 ms
                       nominal: 100 + 15 + 10 = 125 ms     claimed: ~115 ms (release time omitted)
Proposed:              2 x 25 ms x 1.01 (OSCCAL-calibrated) + 9.4 ms + 10 ms                ~ 70 ms
A14 (MOS_B short):     detected on the next tick (10 ms), then K1 must open: 10 + 93..246 + 10 = 113 to 266 ms
                       (data: on-bounded 30 ms, a figure left over from the 1 kHz design)
```

Set 2 also cannot reach its stated range: at β 60 it sinks 7.3 mA, not 10 mA. Limit set 2 to I_IN ≤ 6 mA, or use R53 = 3.3 k. That is availability, not safety.

**Fix.** C62 = **470 nF film**, R62 = **47 kohm**. Q&A period **25 ms**, trip on the **first** missing or wrong answer. Calibrate OSCCAL to ±1 % at bring-up and record it. Set A1 `bound_ms = 120` only after measuring 5a/5e on the LA. Set A14 `bound_ms = 270` (or have the firmware also send a deliberate wrong answer to drop K2 on that tick: then about 10 + 25 + 19 ≈ 55 ms). Fix netlist notes C312/C613, which still list the original failing values (100 nF / 1 µF / 100 k).

**Checker rule (HW-031, extended).** Use corner values, not typicals. Add fields `vpp_min_v`, `diode_vf_max_v` (at the peak current), `vbe_max_v`, `hfe_min` at the temperature floor, `c_series_tol`, and relay `hold_ma` and `release_ms`. Integrate the transient as the simulation above does, then: (1) FAIL if the computed hold is below need at the worst corner (the current constants give 1.17 V against 1.13 V needed; with VF 0.33, Vbe 0.68 and β 50 it FAILs: I reproduced this by editing the data); (2) compute `dropout_ms`, FAIL if it differs from the declared value; (3) FAIL any fault row whose `bound_ms` < detection + computed dropout + `release_ms` of the cut it names.

### R3-B4. Variant B Q64 opens an ISP reflash path from the RP2350 to the ATtiny

**Evidence.** The RP2350 is wired permanently to the ATtiny's ISP pins: GP11→PB0/MOSI, PB1/MISO→GP8, PB2/SCK↔GP10 (C610 to C612; pins flagged `debug = true`). Serial programming needs only RESET held low ("apply power ... while RESET and SCK are set to 0"; Programming Enable after 20 ms). Chip Erase clears the lock bits ("Lock bits can be erased to 1 with the Chip Erase command, only"), so lock mode 3 is no barrier. SPIEN stays programmed. In variant B, Q64 holds RESET low for 140 to 280 ms:

- for 140 to 280 ms after every power-up (U4's power-on reset), and
- again 1.00 to 2.25 s after PB4 stops toggling. The RP2350 can arrange that just by answering wrongly, and an erased ATtiny never toggles PB4, so the windows repeat.

A glitched or compromised RP2350 can therefore erase the ATtiny and write 8 KB in about 5 windows (128 pages × 4.5 ms ≈ 0.6 s), with firmware that pumps PB3 unconditionally. THREAT-MODEL §4a requires that "a glitch which makes the firmware misbehave cannot by itself keep a rail on: the last cut must not depend on the MCU". Q64 makes K2 depend on it. Review 2's "latch" is also not a K2 latch: it is the RP2350 firmware latching OutputMismatch, which is the decision-maker the watchdog is meant to be independent of.

**Fix.** Delete **Q64 and J_B2** (and C629/C630). Latch the clamp in hardware instead:

- **Q65 2N3904 + Q66 2N3906** cross-coupled thyristor latch on WD 5 V:
  - U4 RESET → 10 k → Q65 base, with 100 k from Q65 base to GND;
  - Q65 collector → 10 k → Q66 base, with 47 k from Q66 base to 5 V;
  - Q66 collector → 10 k → Q65 base (the hold path), and Q66 collector → R68 10 k → Q63 base (the clamp).
- Clear the latch with a panel **RESET** momentary (spare CB16AY NO) from Q65 base to GND. It needs a person.
- The U4 power-on pulse sets the latch at every power-up, so K2 stays open after any power loss until a person presses RESET. That also gives "no restart after power loss".
- Every latch fault either clamps (cut) or does not (variant A behaviour), so B11 still holds.
- The ATtiny RESET pin is then driven only by its 10 k pull-up and the human button.

**Checker rule (new HW-035).** No controller of one cut may reach a `debug = true` pin of another cut's controller while any active part (a transistor node, or an output of a part with `output_semantics`) drives that controller's RESET pin. Today it would FAIL on C610 to C612 together with C629. Separately, HW-033 must stop accepting any non-empty `latch` string: `latch = "x"` passes today (I reproduced this). Require `latch_part` to be a netlist part on the cut's own board, with `controlled_by` disjoint from every other series element of the path.

### R3-B5. E-stop release re-arms rail A after one sense fault

**Evidence.** U-ARM senses ARM_OUT, which is downstream of NC1. Pressing the e-stop therefore makes ARM_SENSE go off, and releasing it makes ARM_SENSE come back: exactly the "arm seen off then on" sequence the firmware accepts as a fresh arm (X2). The only thing that tells the two apart is GP2 (NC2). One fault, such as the NC2 sense wire shorted to the adjacent GND terminal (C402 lands NC2.2 on TR GND next to C401), makes GP2 read "released" forever. A press-and-release then re-arms, and rail A "follows the arm switch" (THREAT-MODEL §4), so the load restarts on release. Test 4c and row A36 cover this only with fault-free sensing. The fault-free case is the one ISO 13850 practice does not accept as the guarantee.

**Fix (pick one; the first is cheap).**

- **(a)** Add **U-EOUT, a PC817** from **ESTOP_OUT** (between NC1 and the arm toggle) → 4.7 k → LED → GND, read on **GP17** (bottom pad, wire it like the SWD pads) with a 10 k pull-up. Firmware rules:
  - ESTOP_OUT dead while GP2 reads released latches `SenseMismatch`;
  - an arm off-then-on counts only if ESTOP_OUT stayed live throughout;
  - ESTOP_OUT dead at any time latches the e-stop fault.
- **(b)** Make K0 (M27) a Must, with the e-stop sense moved to ESTOP_OUT as review 2 proposed.

**Checker rule (new HW-036).** For each `[[node]]` sensed by an `arm`-function input, list the human series contacts upstream of it. FAIL if any other human contact lies between the source and the sensed node with no sensed node of its own between them.

### R3-B6. The Pi-hosted logic analyzer is an unmodelled write path, and the AI host records the evidence

**Evidence.** In tests 1, 2, 3, 5 and 6 the analyzer is hosted on the Pi (`ai_host = true`), and its probes sit on RLY IN1/IN2 (D3/D4), the pump nodes (GP14, PB3) and GP12. The FX2's 8051 firmware (fx2lafw) is uploaded by the host at every enumeration. The board's 74LVC245A input buffer is what makes it input-only, but whether its DIR and OE pins are hard-tied is UNVERIFIABLE (sigrok lists the buffer, not its wiring), and buffer-less clones exist. If the FX2 can drive a probe:

- pulling IN1 or IN2 low energises a relay directly (the module is active-low);
- with the RP2350 in reset (pads high-Z, 4.7 k pull-downs), driving GP14 or PB3 pumps the relay;
- driving GP12 high turns the MOSFET on.

A compromised Pi is an assumed attacker. HW-030 does not see any of this: the analyzer is not `passthrough`, and **no probe connection is in the netlist** (only the analyzer ground, C730, and D2 for test 6, C806). Separately, the #144 proof capture (test 5c/5e) is written by the agent-controlled host, so its evidence is not independent of the thing under test.

**Fix.**

- Host the analyzer on the **Mac for tests 1, 2, 4 and 5** (the Pi stays as the device under test).
- Before first use, verify with a meter: the 74LVC245 DIR pin is tied by copper to the level that sets the input direction, and OE is tied to GND. Otherwise fit an input-only 74LVC244 buffer board, or put 10 k series resistors on IN1/IN2 probes, which limits a drive to about 0.4 mA, too little to pull the module's input chain in. Record the result.
- If any agent session has a shell on the Mac, treat the Mac as `ai_host` too. The SWD lead must then be **physically unplugged** except while a person is flashing, and test 5a is run by a person.
- Remap **D4 to RLY2 IN1** (= Q61.C) in every test. D4 is still mapped to "RLY IN2", which is tied to VCC through 1 k since B5, so test 5's "D4 rises" can never show K2 opening.

**Checker rule (HW-030, extended).** Add every probe as a `[[connection]]` with `removable = true` and `tests = [...]`. Mark `kind = "analyzer"` parts `passthrough = true` unless `input_only_verified = true`. Run HW-030 against every node in a path's drive chain (relay IN, pump node, MOSFET PWM), not only `debug` pins. Add `evidence_host` per exit test, and FAIL if it is an `ai_host` for tests 2 and 5.

### R3-B7. The mandatory per-cut self-test is specified wrongly, and the relay plugs can swap

**Evidence.** Wiring-mock test 5 setup (copied from review 2, B5) says "K1 alone dropped shows MID live and VBUS dead". In the topology ARM_OUT → K1 → MID → K2 → RAIL_BUS, an open K1 makes **MID dead**. "MID live, VBUS dead" is the signature of an open K2, so as written no step proves K1 can open.

Both relay modules and both driver boards use the same 4-pin JST-XH, so the plugs can swap. A swapped bench passes the self-test as written, and a correct bench fails it, which invites someone to "fix" it by swapping.

An XH housing crimped onto the modules' bare 2.54 mm pin headers is **not keyed**. The key is the shrouded header, so the housing can still sit one pin off, which grounds IN1 (A31).

**Fix.**

- Expected states:
  - drop K1 (stop GP14): MID dead, VBUS dead;
  - one wrong Q&A answer (K2): MID live, VBUS dead;
  - drop MOS_A: RAIL_A_SENSE dead, VBUS live.
- Desolder each module's straight header. Fit **B4B-XH-A** (shrouded) on RLY1 and **B5B-XH-A** with one blank position on RLY2, with matching housings, so the two cannot swap.

**Checker rule (new HW-037).** Add a `[[selftest]]` record per cut: the element opened and the expected sense states. The checker computes MID, VBUS and RAIL_x from the netlist with that element open and FAILs on any mismatch. Note: the netlist must first stop aliasing all four lamps onto one `LAMP` part. Today `LAMP:+` joins TB-12V-F1 (before the e-stop) to MOS_A:L+ with no series element in between, so any connectivity rule would give wrong answers (see N3).

---

## 2. Non-blocking findings

1. **HW-031 constants are optimistic** (diode 0.25 V, Vbe 0.6 V, β 70, Vpp 3.3 V). Set 1 has 0.04 V of margin at typical values and fails at worst case: 1.21 V hold, 2.9 mA sink against I_IN 3 mA. Measure I_IN. If it is above 2.5 mA, use set 2.
2. **The firmware does not match the data yet.** `CHARGE_PUMP = 17` is still a pad, and no 8 mA pad drive is set in `rp2350.rs`, but `parts.toml` `io_max_ma = 8` says the firmware sets it. Mark it UNVERIFIED until #288 lands.
3. **Data hygiene:**
   - PI5 `pins` still lists ISP pins 19/21/23/22 and `USB2A = PICO`; PICO's USB pin says "micro-USB to Pi"; path PB_* still says "HOLD2 pulled low".
   - The netlist omits MOS_A/B GNDL, every PC817 LED tap, L_BUS (C209), WD GND (C603), the Q&A GND, the SWD GND (the Mac ground bond), the analyzer probes and GP15.
   - One `LAMP` part for four lamps joins TB-12V-F1 to MOS_A:L+ in the graph.
   - Add a rule that fails a series-free path from a source net to a load terminal: it would catch the aliasing today.
4. **GP15** (lease LED) has a 100 k pull-down. On an A2 part, the E9 leakage (up to 120 µA) can bias the 2N3904 and light the trusted panel's lease LED at reset. Use 4.7 k. HW-032 only checks connections that declare a pull-down, so GP15 is invisible to it.
5. **E9 stepping.** Raspberry Pi says A4 fixes E9 and that A3 is an intermediate stepping (not sold as silicon; about 30,000 used on Pico 2 boards). Review 2 cites the datasheet for A3. Record the actual stepping and keep 4.7 k regardless.
6. **Q&A link:**
   - CSn tied low means no resynchronisation. One SCK glitch, or an RP2350 watchdog reset in mid-frame, leaves the bits permanently out of line: K2 stays open, which is safe but needs a power cycle. Re-initialise the SSP after an SCK-idle gap of more than half the period, polled in the main loop.
   - The ATtiny has no RNG. Seed xorshift32 at boot from ADC LSB noise on the internal temperature sensor plus WDT-against-RC jitter, mixed with an EEPROM boot counter, so the challenge sequence does not repeat after a reset. Use 32-bit challenges.
   - Bind the answer to the RP2350's pass counter, and have the ATtiny check that the counter strictly increases.
   - Add an ATtiny-side lint like `ci/check_charge_pump.py`: PB3 and PB4 set only by `PORTB` writes inside the Q&A loop, with no TCCR0/TCCR1/GTCCR compare outputs.
   - The RP2350 side already bans interrupts, PIO, PWM, DMA and core 1 (`ci/check_charge_pump.py`, CONFIRMED). The Q&A must also respect that lint, so at most one TX FIFO word per pass.
7. **Back-feed through the 1 k series resistors** (UART C701/C702, Q&A C610 to C612): about 3 mA into an unpowered board's 3V3 through its ESD diodes, enough to half-power it. Use 4.7 k on the UART at 115200 baud and 10 k on the Q&A at a few kHz.
8. **Adapter hiccup.** A hiccup or fold-back adapter can stop F2 from ever clearing. The Littelfuse 1 A melting I²t is 0.4 A²s, while 3.3 A × 20 ms is only 0.22 A²s per burst. The SCR (IH ≤ 20 mA) then unlatches and refires on every retry. With R3-B1 each retry is clamped within microseconds, so this is acceptable. Measure the Facmogu adapter's short-circuit behaviour and record `min_fault_a`.
9. **The fuse clearing table** (`F1`/`F2` `clearing`) is Littelfuse 257 data, but F1 and F2 are unbranded fuses from the holder pack. The 600 % / 150 ms row is not in the Littelfuse datasheet, which stops at 350 %. Buy genuine 0257001.PXPV, or delete the table.
10. **E-stop mechanics.** NC1 and NC2 are one contact block on one actuator. If the block detaches from the operator (a known e-stop failure), both stay closed while the button is pressed: a common cause the sense cannot see. Press-test the e-stop at the start of every session (test 4a short form). Prefer a self-monitoring contact block for P2.
11. **M15 dummy loads.** 47 ohm 5 W at 3.06 W runs above 150 C at the surface, well above PETG's glass transition (about 80 C). Mount the resistors on standoffs or an aluminium strip. Without them, the 20 mA pilot lamps are a dry-circuit load for the SRD contacts, so contact resistance is unreliable. Fit M15 for the relay tests.
12. **Test text:**
    - Test 2's t0 is the last valid reply from the **Pi** (D6), not D7.
    - Test 1's thresholds ("U4 RESET at 4.50 to 4.75 V", "BOD at 2.5 to 2.9 V") cannot be read off a digital analyzer. Step M2 in 50 mV steps with a 2 s dwell, read the meter, and log the analyzer state per step.
    - Test 5a: halt **both** cores, and set the precondition that GP12 is high (lamp lit) so D5 can show the cut.
    - Add 5g: the residual A39 ("two cuts, one decision-maker") stated as a limit, not a pass.
13. **Bus guard.** Review 2 non-blocking 9 (no USB reset/BOOTSEL interface in the guard firmware) is not carried into the design docs or into any rule.
14. **Ground bonds.** The Mac bonds to bench GND through the SWD lead and the analyzer. Plug USB in before bench power, and never hot-plug a probe ground.
15. **HW-016** classifies default states by regex. The word "on" in "pull-down on" reads as "closed". It passes today only because the relay and MOSFET texts avoid the word. Use an enum field.
16. **HW-019** trusts `load_state`. A39's effect says "K1, K2 and the MOSFET are all held", and A36's says "re-energises on release", yet both are `load_state = "off"`. Require `remaining_cuts = [...]` on every `off` row, and check each named cut against the faulted controller.

---

## 3. Assessment of the review-2 fixes

| Review-2 item | As implemented | Verdict | Why |
|---|---|---|---|
| B1 pump re-size for 50 Hz | Step 5 sets 1/2, PUMP1/PUMP2 records, HW-031 | **INCOMPLETE** | Right direction, and K1 now pulls in. But: K2's dropout leaves out the ≤ 10 ms relay release and the hold tail, so the 120 ms bound is missed (R3-B3); set 2 cannot sink 10 mA at worst case; set 1 has 0.04 V of margin; netlist C312/C613 still hold the old values; A14 bound stale; HW-031 uses typical values and never checks dropout |
| B2 SCR crowbar | TL431A anode → gate, 12 k/10 k, 220 ohm | **WRONG** | Trips at 6 to 16 V (R3-B1); no fuse in the live-buck crowbar path (R3-B2); "survives indefinitely" false; acceptance source tops out at 5.5 V |
| B3a Q64 resets the ATtiny | Q64 + J_B2, C629/C630 | **WRONG (adds a fault)** | Opens an ISP reflash window (R3-B4). The "latch" is RP2350 firmware |
| B3b delete 1N4007 | C601 direct | **CORRECT** | U4 VCC 5.05 V against a 4.75 V maximum threshold |
| B3c R67 2.2 k | — | **CORRECT** | WDI high ≥ 5.05 − 150 µA × 2.2 k = 4.72 V against VIH 3.5 V |
| B3d BAT43 diode-AND | — | **CORRECT** | MR ≤ 0.40 (VOL at 1.2 mA) + about 0.28 (BAT43 at 0.6 mA) = 0.68 V against VIL 0.8 V. Margin 0.12 V: measure. Reverse leakage is irrelevant: both diodes sit near 0 V reverse when idle |
| B3e R70 0 ohm to the Q61 base | C627 | **CORRECT** | Q63 sinks about 0.35 mA from HOLD2 through R63; Vce(sat) about 0.05 V holds Q61 off |
| B3f B6 rewording | — | **CORRECT** | — |
| B4 SWD probe on the Mac | C710/C711/C722, HW-030 | **INCOMPLETE** | Stale PI5/PICO pins; the analyzer write path and evidence host are unmodelled (R3-B6); the Mac is human-only only if no agent has a shell there |
| B5 two modules, keyed connectors, per-cut self-test | RLY2, C304b to C307, HW-020 | **INCOMPLETE** | The self-test spec is wrong (R3-B7); an XH housing on bare pins is not keyed; identical plugs can swap; analyzer D4 still on IN2 |
| E9 4.7 k pull-downs | C324 to C326, C310/C311, HW-032 | **CORRECT** | 120 µA × 4.7 k = 0.56 V < VIL. GP15 missed (N4); the A3 stepping claim is unconfirmed (N5) |
| NB2 Q&A rules | A39 WARN | **INCOMPLETE** | Not in data, firmware or CI; the row says `off` while its effect says all held |
| NB3 K0 seal-in | M27, open owner decision | **INCOMPLETE** | The underlying hazard is a single fault (R3-B5) |
| NB4 test 4c | Added | **CORRECT** (text) | Analyzer channels stale (D4) |
| NB5 analyzer and meter grounds | C730, A28 | **CORRECT** | — |
| NB8 F1 ≤ 2 A | A40 | **CORRECT** | — |
| NB9 guard BOOTSEL | — | **INCOMPLETE** | Not carried (N13) |
| NB10 8 mA drive | `io_max_ma = 8` | **INCOMPLETE** | The firmware does not set it yet (N2) |

---

## 4. Parts to add, swap or remove

| Change | Part | Qty | Where |
|---|---|---|---|
| Add (R3-B1) | 2N3906 PNP, TO-92 | 1 pack (uses 2, plus 1 for R3-B4) | Amazon "2N3906 100 pcs" packs; DigiKey 2N3906BU |
| Add (R3-B1) | 680 ohm, 1.0 k, 100 ohm (¼ W); keep 12.0 k/10.0 k 1 % | 2 each | Assortment / 1 % metal-film kit |
| Change (R3-B1) | M2 adjustable source, 0 to 7 V (LM2596-ADJ set range) | — | Same module, re-specified |
| Add (R3-B2) | F3: third inline ATO holder (already in the #15 pack) + genuine Littelfuse **0257001.PXPV** 1 A ×5 (also for F1 and F2) | 5 | DigiKey/Mouser (genuine, so the clearing table applies) |
| Add (R3-B2) | Clip-on TO-220 heatsink | 1 | Amazon |
| Swap (R3-B3) | C62 1 µF → **470 nF film 63 V**; R62 100 k → **47 k** | 2 each | Amazon polyester film assortment (0.47 µF) |
| Remove (R3-B4) | Q64, J_B2, C629/C630 | — | — |
| Add (R3-B4) | Q65 2N3904 + Q66 2N3906 latch; 10 k ×3, 100 k, 47 k; RESET button (spare CB16AY NO from #20) | 1 set | Owned packs |
| Add (R3-B5a) | 1 more PC817 (U-EOUT), 4.7 k, 10 k; a wire to the GP17 pad | 1 | M7 10-pack already covers it |
| Add (R3-B6) | Optional input-only buffer (74LVC244 breakout) or 10 k series probe leads | 1 | Amazon / DigiKey SN74LVC244AN |
| Add (R3-B7) | JST-XH **shrouded** straight headers B4B-XH-A and B5B-XH-A with housings | 2 each | Included in most Amazon XH kits; check the kit lists shrouded headers |
| Swap (N4) | GP15 pull-down 100 k → 4.7 k | 1 | Assortment |
| Swap (N7) | UART series 1 k → 4.7 k; Q&A series 1 k → 10 k | 5 | Assortment |
| Keep | MAX813LCPA+ ×2 genuine (M24) | 2 | DigiKey/Mouser only |

---

## 5. Claim log

| # | Claim | Source | Result |
|---|---|---|---|
| 1 | TL431 REF is referenced to the anode; TL431A Vref 2.470 to 2.520 V; Imin ≤ 1 mA; Iref ≤ 4 µA | https://www.ti.com/lit/ds/symlink/tl431.pdf | CONFIRMED |
| 2 | Crowbar trips at 5.32 to 5.66 V (anode → gate topology) | Same + BT151 datasheet, maths in R3-B1 | **WRONG** (6.0 to 16.3 V) |
| 3 | BT151-500R IGT 2/15 mA, VGT 0.6/1.5 V, IH 7/20 mA, IL 10/40 mA, ITSM 100 A, I²t 50 A²s, Rth j-a 60 K/W | https://www.uv.es/~maicasj/lcsp/ds/bt151.pdf | CONFIRMED |
| 4 | "The SCR survives the adapter's short-circuit current indefinitely" | Same (60 K/W) + LM2596 limit | **WRONG** |
| 5 | LM2596 current limit 3.5 to 7.5 A over temperature, with frequency fold-back into a short | https://www.onsemi.com/download/data-sheet/pdf/Lm2596-d.pdf | CONFIRMED |
| 6 | ATO 257 1 A: 200 % 0.10 to 5 s, 350 % 0.02 to 0.5 s, melting I²t 0.4 A²s | https://www.farnell.com/datasheets/5428.pdf | CONFIRMED |
| 7 | ATO 1 A: 600 % ≤ 150 ms (the `clearing` table) | Same (the table stops at 350 %) | UNVERIFIABLE |
| 8 | MAX813L abs max 6.0 V; threshold 4.50/4.65/4.75 V; reset 140 to 280 ms; WDT 1.00 to 2.25 s; WDI VIH 3.5 V, 50 to 150 µA; MR VIL 0.8 V, 100 to 600 µA, 150 ns min pulse; timer cleared on reset; WDO low below threshold; PFI 1.20 to 1.30 V; WDO VOL 0.4 V at 1.2 mA; ICC 350 µA (C) / 500 µA (E/M) | https://www.analog.com/media/en/technical-documentation/data-sheets/MAX705-MAX813L.pdf | CONFIRMED |
| 9 | WDO→MR gives a pulse train, not a latch | Same | CONFIRMED |
| 10 | E9: up to 120 µA leakage; ≤ 8.2 k pull-down; A2 affected | https://hackaday.com/2024/09/20/raspberry-pi-rp2350-e9-erratum-redefined-as-input-mode-leakage-current | CONFIRMED |
| 11 | E9 fixed in A3 and A4 | https://www.raspberrypi.com/news/rp2350-a4/ (says A4 fixes E9; A3 is "intermediate") | UNVERIFIABLE for A3 |
| 12 | RP2350 5 V tolerant only while IOVDD is powered | Same | CONFIRMED |
| 13 | ATtiny85 BODLEVEL 101 = 2.5/2.7/2.9 V; hfuse layout; 0xCD/0xE2/0xFF/0xFC decode | ATtiny25/45/85 datasheet (Atmel 2586), https://ece-classes.usc.edu/ee459/library/documents/Atmel-2586-AVR-8-bit-Microcontroller-ATtiny25-ATtiny45-ATtiny85_Datasheet.pdf | CONFIRMED |
| 14 | ATtiny serial programming needs RESET low; Chip Erase clears the lock bits (fuses unchanged) | Same, §20.1, §20.5.1 | CONFIRMED |
| 15 | ATtiny RC ±10 % factory, ±1 % user-calibrated; 0 to 10 MHz at 2.7 to 5.5 V | Same, Table 21-2 | CONFIRMED |
| 16 | SRD-05VDC-SL-C coil 71.4 mA (70 ohm), pull-in ≤ 75 %, release ≥ 10 % (0.5 V), operate and release ≤ 10 ms | https://cdn.shopify.com/s/files/1/0695/1347/8453/files/SRDsongle.pdf | CONFIRMED |
| 17 | K2 drop-out 8 to 15 ms; the 120 ms bound met at about 115 ms | Simulation (R3-B3) + #16 | **WRONG** (13 to 21 ms + ≤ 10 ms release; 125 to 141 ms) |
| 18 | Pump set 1 HOLD ≥ 1.23 V | Simulation | **WRONG** at worst case (1.21 V) |
| 19 | Pump set 2 good to I_IN 10 mA | Simulation (β 60) | **WRONG** (7.3 mA) |
| 20 | A14 on-bounded 30 ms | Simulation + #16 | **WRONG** (113 to 266 ms) |
| 21 | MCP1700 abs max 6.5 V; operating 2.3 to 6.0 V; ±3 % | https://alliance.seas.upenn.edu/~medesign/wiki/uploads/Courses/MCP1700.pdf | CONFIRMED |
| 22 | ME6217 (RP2350-Zero LDO) abs max | https://xonstorage.blob.core.windows.net/pdf/microone_me6217c33p5g_apr22_xonlink.pdf | CONFIRMED: 7.0 V abs max, 6.5 V operating (review 2 had UNVERIFIABLE) |
| 23 | INA228 CM −0.3 to 85 V; VS 2.7 to 5.5 V; ±163.84/±40.96 mV; ±1 µV; 16 addresses | https://www.ti.com/product/INA228 | CONFIRMED |
| 24 | Analyzer has a 74LVC245A, −0.5 to 5.25 V, VIH 2.0 V | https://sigrok.org/wiki/Noname_Saleae_Logic_clone | CONFIRMED |
| 25 | Analyzer is input-only (DIR and OE hard-tied) | Same | UNVERIFIABLE (measure) |
| 26 | The firmware toggles the pump once per 10 ms pass from the main loop, with no IRQ/PIO/PWM/DMA/core 1 | repo `crates/sawbones-trust-root/src/lib.rs`, `ci/check_charge_pump.py` | CONFIRMED |
| 27 | The firmware sets GP14 drive to 8 mA; the pump is on GP14 | repo `firmware/trust-root/src/rp2350.rs` (`CHARGE_PUMP = 17`, no drive setting) | **WRONG** (not yet; #288) |
| 28 | RP2350 watchdog 50 ms, fed once per pass | Same, `WATCHDOG_US = 50_000` | CONFIRMED |
| 29 | RP2350 watchdog pauses under debug (PAUSE_DBG0/1 = 1 at reset) | https://docs.embassy.dev/rp-pac/7.0.0/rp235x/watchdog/regs/struct.Ctrl.html (bits exist; reset value not shown) | UNVERIFIABLE here (review 2 cites the datasheet) |
| 30 | #144: the watchdog cut must drop the rail "with no help from" a frozen, reset-looping, glitched or wrong-answering RP2350 | `gh api repos/rafe-walker/sawbones-labs/issues/144` | CONFIRMED; Q64 breaks it (R3-B4) |
| 31 | "K1 dropped shows MID live" (self-test) | Netlist topology C204 to C207 | **WRONG** |
| 32 | `ci/check_hardware.py` passes: 27 PASS, 6 WARN, 0 FAIL; 83 unit tests OK | Run on the branch | CONFIRMED. Feeding the real crowbar numbers into CL1 → HW-029 FAIL; worst-case pump constants → HW-031 FAIL; `latch = "x"` → HW-033 still PASS |
