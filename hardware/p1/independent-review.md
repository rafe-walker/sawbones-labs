# Sawbones Labs P1: independent clean-room hardware safety review

Reviewed 26 Nov 2026: `compatibility.md`, `wiring-mock.md`, `method.md`, `data/*.toml` and `parts.md`. I did not write the design, and I treated every claim in it as unverified. Datasheets were downloaded and read as text (pdftotext), not summarised from memory. The trust-root firmware (`rafe-walker/sawbones-labs` main, `firmware/trust-root/src/rp2350.rs`, `crates/sawbones-trust-root/src/lib.rs`) was read to check the toggle rate the hardware assumes. The charge-pump numbers come from a time-step simulation (Schottky and BJT exponential models; script not kept in the repo). Nothing was bought, pushed or posted.

```
INDEPENDENT REVIEW: APPROVE WITH CHANGES
```

The architecture is sound: e-stop in the power path, two dead-man relays from two different chips, an active-high MOSFET, and the rule that no GPIO drives a relay directly. But five things are wrong as drawn, and the owner should not solder the relay drive or order the crowbar parts until they are fixed:

1. The pump values are designed for 1 kHz, but the firmware toggles at 50 Hz, so K1 never pulls in.
2. The zener "crowbar" can die before the fuse blows.
3. The MAX813L in variant B gives 200 ms pulses, not a latched cut.
4. The SWD probe is hosted on the AI-controlled Pi.
5. Both cuts sit on one active-low relay board with a loose Dupont header.

Each has an exact fix below. None needs a redesign.

---

## 1. Claim verification log

| # | Claim (in the design) | Source | Result | Consequence |
|---|---|---|---|---|
| V1 | RP2350 bank-0 pads at reset: output high-Z, input disabled, pull-down on, isolation latched | RP2350 datasheet §9.3, Table 885 (ISO=1, IE=0, PDE=1, OD=0): https://datasheets.raspberrypi.com/rp2350/rp2350-datasheet.pdf | CONFIRMED | GP12/GP13 are off at POR/RUN/BOR. On a **watchdog** reset the pads return to the reset state only because the firmware's PSM WDSEL (`0x1ffff` minus the oscillators) resets RESETS → PADS/IO, and ISO is already clear. Keep WDSEL as written. |
| V2 | "Pads go to the reset state on any reset" (A2) | Same, §9.3/§9.7: pads return to reset only on BOR, RUN low, SWD CDBGRSTREQ and rescue. ISO bits are not reset by the PADS block reset | PARTLY WRONG | Correct for this firmware (see V1). A future build that sets ISO=1 (low-power states), or narrows WDSEL, would **hold outputs high through a watchdog reset**. Add a CI check that WDSEL includes RESETS and that ISO is never set. |
| V3 | SWD halt freezes GP12 high (A1) | Same, WATCHDOG CTRL PAUSE_DBG0/1, PAUSE_JTAG reset = 1 | CONFIRMED | A halted core never watchdog-resets, so GP12 stays high indefinitely. The cut then rests only on K1 and K2. That is correct, but it makes finding B4 critical. |
| V4 | E9: "fit external 10 kΩ pull-downs" covers E9 | Same, App. D.5.1 RP2350-E9: leakage ~120 µA holds the pad near 2.2 V; "pulling the pad low with 8.2 kΩ or less" overcomes it; affects A2 only | **WRONG (value)** | 10 kΩ is above the 8.2 kΩ limit. On A2 parts the GP7/GP28/GP29 senses can latch "live". That is the safe direction (fault, lost uptime), but it is not "covered". Use 4.7 kΩ. |
| V5 | "A4 fixes E9" | Same, stepping notes: "Stepping A3 … Fix RP2350-E9" | WRONG (minor) | The fix arrived in A3. Record the stepping: A2 = affected; A3/A4 = fixed. |
| V6 | E9 on outputs GP12/GP13 | Same, D.5.1 conditions (IE=1 and OE=0); the firmware clears IE on outputs | CONFIRMED not applicable | 4.7 kΩ on GP12/GP13 is harmless but not needed for E9. |
| V7 | WS2812 on GP16 | Zephyr `rp2350_zero-pinctrl.dtsi` (`PIO0_P16`): https://github.com/zephyrproject-rtos/zephyr/blob/main/boards/waveshare/rp2350_zero/ ; CircuitPython `pins.c` NEOPIXEL=GPIO16: https://github.com/adafruit/circuitpython/blob/main/ports/raspberrypi/boards/waveshare_rp2350_zero/pins.c | CONFIRMED | Remap `RAIL_B_ENABLE[3]` off GP16 (as proposed). |
| V8 | 20 edge GPIO = GP0–15 + GP26–29; GP17+ are pads | Waveshare wiki ("20 can be led out through pin headers, the rest … soldering"): https://www.waveshare.com/wiki/RP2350-Zero | Count CONFIRMED, exact mapping UNVERIFIABLE (image only) | Check the pinout card before soldering. 16 + 4 = 20 is consistent with the remap to GP14. |
| V9 | No diode between VBUS and the 5 V pin | Waveshare schematic https://files.waveshare.com/wiki/RP2350-Zero/RP2350_Zero.pdf (text extraction garbled) | UNVERIFIABLE (here) | Keep rule P7 (never plug USB into the trust root) regardless. |
| V10 | ATtiny85 hfuse 0xCD = SPIEN on, WDTON on, BODLEVEL 101 | ATtiny25/45/85 datasheet Table 20-4: https://ww1.microchip.com/downloads/en/DeviceDoc/Atmel-2586-AVR-8-bit-Microcontroller-ATtiny25-ATtiny45-ATtiny85_Datasheet.pdf | CONFIRMED | 0xCD = 1100 1101: RSTDISBL=1 (reset pin kept), DWEN=1 (off), SPIEN=0 (on), WDTON=0 (always on), EESAVE=1, BODLEVEL=101. |
| V11 | lfuse 0xE2 = 8 MHz internal, CKDIV8 off | Same, Table 20-5 | CONFIRMED | 1110 0010: CKDIV8=1 (off), CKOUT=1, SUT=10 (the default slow-rise start-up), CKSEL=0010 (8 MHz RC). |
| V12 | efuse 0xFF | Same, Table 20-3 | CONFIRMED | SELFPRGEN off, so the firmware cannot rewrite itself. Good. |
| V13 | lock 0xFC = mode 3, fuses locked; owner not locked out | Same, Table 20-2; note 4 on SPIEN ("not accessible in SPI programming mode") | CONFIRMED | LB2=LB1=0. Chip erase over ISP clears the lock bits. SPIEN cannot be cleared over ISP and RSTDISBL stays 1, so ISP is always recoverable. Verify the flash **before** writing the lock: in mode 3, verification is disabled. |
| V14 | BOD 101 = 2.5–2.9 V | Same, Table 21-7 | CONFIRMED | Footnote 1: parts are production-tested down to VBOT, so 2.5 V min vs the 2.7 V speed-grade floor is covered. |
| V15 | ATtiny85-20PU at 3.3 V, 8 MHz | Same, front page: "ATtiny25/45/85: 0–10 MHz @ 2.7–5.5 V" | CONFIRMED | OK. |
| V16 | WDTON = always on | Same, Table 8-1 (safety level 2: cannot be disabled; timeout changed only by timed sequence) | CONFIRMED | The WDT starts at the shortest timeout after reset. Firmware must `wdr` early or set WDP in the first few ms, or it reset-loops. That is safe (cut), but plan for it in bring-up. |
| V17 | ATtiny VIH 0.6·VCC, VOH ≥ 2.5 V at −5 mA, 3 V; port B tri-stated in reset | Same, DC table, §10 | CONFIRMED | Q&A levels are fine at 3.3 V. Note RESET VIH2 = 0.9·VCC. |
| V18 | MAX813L threshold 4.50/4.65/4.75 V; reset pulse 140/200/280 ms; WDT 1.00/1.60/2.25 s; WDI VIH 3.5 V / VIL 0.8 V; PFI 1.20–1.30 V; RESET active-high; DIP pinout 1 MR, 2 VCC, 3 GND, 4 PFI, 5 PFO, 6 WDI, 7 RESET, 8 WDO; floating WDI disables WDT | MAX705–MAX813L datasheet: https://www.analog.com/media/en/technical-documentation/data-sheets/MAX705-MAX813L.pdf | CONFIRMED | — |
| V19 | (implied by B1) WDO→MR gives a held cut | Same, pin description: "The internal watchdog timer clears whenever reset is asserted"; "MR … triggers a reset pulse"; "Simply connect WDO to MR" | **WRONG** | WDO low → MR low → reset asserted → timer cleared → WDO high → RESET releases after 140–280 ms → the timer restarts. The result is a **pulse train**: K2 is clamped about 200 ms in every 1.2–2.5 s, not held. See B3. |
| V20 | MAX813L abs max VCC | Same: VCC −0.3 to 6.0 V | CONFIRMED | Any 5 V clamp above 6.0 V kills U4. See B2. |
| V21 | WDI input current; MR pull-up and threshold | Same: WDI 50–150 µA at 5 V; MR 100–600 µA, VIL 0.8 V | CONFIRMED | R67 10 kΩ gives WDI-high down to 5.05 − 1.5 = 3.55 V vs VIH 3.5 V: no margin. A 1N4148 diode-AND gives MR ≈ 0.6 V + VOL, about 0.8 V at 600 µA: no margin. See B3. |
| V22 | MAX813L ICC ≤ 350 µA | Same: 350 µA max (C grade), 500 µA (E/M grade) | CONFIRMED, but grade-dependent | Acceptance step 1 must use 500 µA if the lot is E-grade. |
| V23 | AOD4184A fully on at 4.5 V gate | AOD4184A datasheet: VGS(th) 1.7–2.6 V; RDS(on) ≤ 9.5 mΩ at 4.5 V: https://www.aosmd.com/res/datasheets/AOD4184A.pdf | CONFIRMED | — |
| V24 | D4184 module: 1 kΩ + PC817 input, 2.5–20 V; 4.7k/4.7k gate divider ≈ 6 V at 12 V; 4.7 kΩ pull-down; low-side; no flyback; grounds not joined | Reference board https://protosupplies.com/product/d4184-mosfet-control-module/ | CONFIRMED for the reference; UNVERIFIED for the DIYables board | At PC817 CTR min, the gate is about 4–4.7 V at a 3.3 V input: still on. Measure the gate. |
| V25 | 2-ch relay module: active-low, input LED chain from VCC, about 70 mA/coil, JD-VCC jumper, flyback diodes | Reference https://protosupplies.com/product/relay-module-5v-x-2-relay-w-optoisolation/ ("<5 mA" input; HIGH **or no input** deactivates) | CONFIRMED for the reference; UNVERIFIABLE for SunFounder (Amazon blocks fetches; `parts.md` scrape says "15–20 mA drive") | The input current sets the pump design. Measure it (B1). |
| V26 | 2N3904 gain | onsemi 2N3903/2N3904: hFE min 40 @ 0.1 mA, 70 @ 1 mA, 100 @ 10 mA: https://www.onsemi.com/download/data-sheet/pdf/2n3903-d.pdf | CONFIRMED | Used for the pump margins below. |
| V27 | Pump: HOLD1 ≈ 2.3–2.6 V at 1 kHz (100 nF / 1 µF / 10 k / 100 k) | Simulation (V26 models) | **WRONG** | HOLD1 ≈ 1.7 V, Ib ≈ 100 µA → Ic capability 7 mA (β 70) to 15 mA (β 150). Enough for a 2–5 mA input; **not** for the 15–20 mA in the Amazon listing. |
| V28 | Pump "toggled from the main loop at about 1 kHz" | Firmware: `TICK_US = 10_000`; `toggle_charge_pump()` (one `gpio_out_xor`) once per tick in `sawbones-trust-root` lib.rs:372 | **WRONG** | The GP14 square wave is **50 Hz** (20 ms period). With the as-drawn values: HOLD1 ≈ 0.6 V, Ib ≈ 0.6 µA, **K1 never closes**. Fail-safe, but the bench does not work, and every "K1 drops in about 20 ms" timing in A1, A2 and tests 1, 2, 5 is wrong. See B1. |
| V29 | 1N5339B + 1 A ATC blows the fuse before the zener dies | onsemi 1N5333B series: 1N5339B IR(surge) 13.4 A, **8.3 ms square, non-repetitive**; IZM 865 mA; 5 W at TL = 25 °C, 3/8″ leads: https://www.onsemi.com/download/data-sheet/pdf/1n5333b-d.pdf . Littelfuse ATO 257 1 A: 200 % → 0.15–5 s, 350 % → 0.02–0.5 s, 600 % → ≤ 0.1–0.15 s; I²t 0.4 A²s; cold R 123 mΩ: https://projectlab2019.phas.ubc.ca/projectlab.engphys.ubc.ca/wp-content/phys253/files/datasheets-elec/bladefuse0257010PXPV_10A_BatteryFuse.pdf | **WRONG / unproven** | A 3 A-class adapter delivers about 2–5 A into the fault. The fuse then needs 0.02–5 s. Fig. 5/6 scaling puts the zener's capacity at that pulse width at about 1–4 A. Blowing the fuse inside 8.3 ms needs about 7 A (√(0.4/0.0083)), which the adapter cannot supply. The zener can therefore fail first. A short is the usual failure, but an open after overstress puts 12 V back on the rail. While it survives, it **clamps at about 6.5–8 V at amps (ZZT 1 Ω at 220 mA)**, above the MAX813L 6.0 V and MCP1700 6.5 V abs max (V30). The adapter's OCP/hiccup behaviour is UNVERIFIABLE. See B2. |
| V30 | LDO abs max | MCP1700: VDD +6.5 V: https://ww1.microchip.com/downloads/en/DeviceDoc/MCP1700-Low-Quiescent-Current-LDO-20001826E.pdf ; ME6217 (on RP2350-Zero): UNVERIFIABLE (no primary datasheet retrieved) | CONFIRMED / UNVERIFIABLE | The clamp must sit below 6.0 V. |
| V31 | INA228 VS 2.7–5.5 V; CM −0.3–85 V; ±163.84 / ±40.96 mV; offset ±1 µV | TI INA228: https://www.ti.com/lit/ds/symlink/ina228.pdf | CONFIRMED | Placement after K2 is correct: VBUS proves K1·K2. Use ADCRANGE=1. |
| V32 | Analyzer limits −0.5–5.25 V, VIH 2.0 V | sigrok clone page (carried, not re-read) | UNVERIFIED | Keep the 10 kΩ series rule on 5 V nets. |

---

## 2. Blocking findings (fix before soldering the stages concerned)

### B1. The charge pump is designed for a rate the firmware does not produce (V27, V28)

The firmware toggles GP14 once per 10 ms tick, which gives a 50 Hz square wave. The step-5 values only work at about 1 kHz, and even there HOLD is 1.7 V, not 2.3–2.6 V.

**Fix (K1 stage, step 5). Keep 50 Hz; do not move the pump to PWM, PIO or a timer.**

1. First, measure the relay module's input current. Short IN1 to GND through the meter on the mA range, with VCC at 5.05 V. Record it as `I_IN`.
2. If I_IN ≤ 3 mA:
   - C51 = **1 µF film** (polyester/PP, not ceramic: an MLCC crack-short turns the pump into DC coupling, row A17)
   - C52 = **10 µF electrolytic**
   - R52 = **47 kΩ**
   - R53 = **10 kΩ**
   - R51 = 470 Ω; D51/D52 = BAT43
   - Simulated: HOLD ≥ 1.23 V, Ib ≥ 52 µA (Ic 3.7–7.9 mA), drop-out 27–116 ms.
3. If 3 mA < I_IN ≤ 10 mA:
   - C51 = **2.2 µF film**
   - C52 = **22 µF electrolytic**
   - R52 = **47 kΩ**
   - R53 = **4.7 kΩ**
   - Simulated: Ib ≥ 130 µA (Ic 9–20 mA), drop-out 110–220 ms.
4. If I_IN > 10 mA, do not use this stage. Ask for a redesign (MOSFET gate stage).
5. Set GP14 pad DRIVE to 8 mA in firmware (peak about 3 mA through 470 Ω).

**ATtiny K2 stage (step 6.3, PB3 toggled in software at about 1 kHz):**

- C61 = **220 nF film**, R63 = **4.7 kΩ**; keep C62 1 µF and R62 100 kΩ.
- Simulated: Ib ≥ 200 µA, drop-out 8–15 ms.
- PB3 is also the Timer1 /OC1B pin. Make it a firmware rule (and a CI grep) that PB3 is bit-banged inside the validated Q&A loop and never driven by Timer0/Timer1 hardware.

**Re-baseline the timing everywhere:**

- K1 drop-out is now about 0.03–0.22 s.
- The frozen-RP2350 bound (A1, 120 ms) is met by **K2** (2 × 50 ms Q&A + about 15 ms), not by K1.
- Update the "D3 rises ≤ 20 ms" lines in tests 1, 2 and 5, and the A1/A2 effects.
- Simulate in ngspice (#123), then measure on the bench.

### B2. The zener is a clamp, not a crowbar, and is not rated to outlast a 1 A ATO at adapter-limited current (V29, V30, V20)

**Fix: replace M6 (1N5339B) with an SCR crowbar on TB-5V. Keep F2 = 1 A ATO ahead of the buck.**

- **SCR:** BT151-500R (TO-220, 12 A, IGT ≤ 15 mA). Anode to TB-5V, cathode to TB-GND.
- **Trigger:** TL431A (TO-92).
  - Cathode to TB-5V; anode to the SCR gate.
  - REF from a divider: **12.0 kΩ 1 %** from TB-5V to REF, **10.0 kΩ 1 %** from REF to GND.
  - **1 nF** from REF to GND.
- **Gate:** **220 Ω** plus **10 nF** from gate to cathode.
- Trip = 2.495 × 2.2 = **5.49 V nominal (5.32–5.66 V worst case)**. That is above the 5.05–5.10 V set point and below the MAX813L 6.0 V and MCP1700 6.5 V abs max.
- When it fires, the rail drops to about 1.2 V. All logic is unpowered and the coils release. Then either F2 blows (≥ 350 % → ≤ 0.5 s) or the adapter hiccups. Both outcomes are safe. The SCR survives the adapter's short-circuit current indefinitely.
- **Bench acceptance:** feed TB-5V (buck disconnected) from M2 through a 1 A fuse and ramp it up slowly. Accept if the SCR fires at 5.32–5.66 V and does not fire at 5.20 V for 10 minutes.
- Update A26 and A29: it is the SCR, not the zener, that also catches a bumped buck trimpot.

### B3. Variant B does not hold K2 off, and as wired it can never arm (V19, V21)

**(a) Pulses, not a latch.** B1's "OFF after ≤ 2.25 s" is wrong: the clamp is released after 140–280 ms each cycle (V19).

- **Fix:** add **Q64 2N3904**: base ← U4 pin 7 (RESET) via 10 kΩ, 100 kΩ base→GND, collector → **ATtiny pin 1 (RESET)**, emitter GND. Put it behind a second jumper pole J_B2 (open in variant A).
- A watchdog timeout then really resets the ATtiny. PB3 tri-states and the ATtiny needs a fresh Q&A before it pumps again.
- **And** the RP2350 firmware must latch `OutputMismatch` on any unexpected VBUS/MID loss while armed. That system-level latch turns the first K2 dropout into a permanent off until re-arm.
- Rewrite B1 as "OFF within ≤ 2.25 s, then latched by the RP2350".

**(b) The 1N4007 in C601 drops about 0.7–0.9 V.** U4 VCC is then about 4.2 V, below the 4.50–4.75 V threshold, so U4 sits permanently in reset and **K2 can never close in variant B**. The same diode leaves the WDI pull-up at about 2.7 V, below VIH 3.5 V.

- **Fix:** delete the 1N4007 and feed WD 5 V straight from TB-5V. Use a keyed 2-pin JST-XH for polarity instead.

**(c) R67 10 kΩ WDI pull-up:** worst-case high is 3.55 V vs VIH 3.5 V.

- **Fix:** R67 = **2.2 kΩ**. Worst case is then 4.7 V; the 2N3904 sinks 2.3 mA.

**(d) Diode-AND into MR:** 1N4148 gives MR at about 0.8 V vs VIL 0.8 V.

- **Fix:** D63/D64 = **BAT43**. MR is then about 0.45 V.

**(e) Q63 clamp through R70 1 kΩ into HOLD2:** with the B1 pump values, the pump can push about 0.6 mA into the clamp, which leaves HOLD2 near 0.6 V.

- **Fix:** **R70 = 0 Ω, and move the J_B pin 2 connection from HOLD2 to the Q61 base**. Q63 Vce(sat) < 0.2 V then holds Q61 off regardless of the pump.
- B11 ("can only cut") still holds.

**(f) B6 is wrong as written.** With R67 fitted, an open Q62 holds WDI high, so the watchdog times out. That is a cut, not a disabled watchdog. Only a broken WDI wire floats the pin.

### B4. The SWD probe is hosted on the Pi: the AI host gets write access to the trust root (wiring-mock §7.2/§7.3)

Over SWD the Pi can halt, write RAM and flash, and run code on the RP2350. That code can toggle GP14, answer the ATtiny's Q&A (the protocol is in the public repo) and drive GP12. One connected lead therefore defeats **K1, K2 and the MOSFET together**. F6 scopes this only "for the test".

- **Fix:**
  - The Pico debugprobe is USB-hosted **only on the MacBook**, never on the Pi.
  - The 3-pin SWD lead is a keyed JST-XH that is unplugged except during flashing and test 5a, and test 5a is run from the Mac.
  - The same rule applies to the Pi-header ATtiny ISP wiring: off-board, removed after use.
- Add a fault row: "debug/ISP channel reachable from the supervisor host → all electronic cuts defeated → FAIL; procedure + physical disconnect".
- Longer term: OTP debug-disable on the trust root (F6).

### B5. Both dead-man cuts on one active-low board with a loose Dupont header

On a SunFounder module, any short of IN to GND energises that relay. That includes an off-by-one 4-pin Dupont housing (GND/IN1/IN2/VCC), a stray strand, or a perfboard solder splash. One misplug grounds IN1: K1 is held by a wire, and only MID_SENSE notices. A18 (IN1–IN2 bridge) is the same class of fault and defeats the ATtiny's cut.

- **Fix:**
  - Make M20 a **Must**: K1 = CH1 of module 1, K2 = CH1 of module 2. Leave the CH2s unused, with IN2 tied to VCC through 1 kΩ so it cannot float low.
  - Land each module's IN/VCC/GND on a **keyed** connector (JST-XH 4-pin) or screw terminals, not a bare Dupont strip.
  - Mount the modules apart, on separate standoffs.
  - The arm-time per-cut self-test (F4) is then **required** before P1 sign-off, not optional:
    - drop K1 alone and check MID live, VBUS dead;
    - send one wrong Q&A answer and check VBUS dead;
    - drop the MOSFET alone and check RAIL_A_SENSE dead.

---

## 3. Non-blocking findings

1. **E9 pull-downs (V4):** change the GP7/GP28/GP29 10 kΩ pull-downs to **4.7 kΩ**. The PC817 emitter output still reaches > 2.5 V: 0.7 mA needs CTR ≥ 30 % at 2.3 mA.
2. **K2 is a liveness monitor, not a second decision-maker.** The ATtiny never sees the Pi heartbeat, the e-stop or the arm switch. An RP2350 that is alive and keeps answering correctly but makes the wrong decision (a logic bug, or the Q&A answered from a path that keeps running while the safety logic does not) defeats K1, K2 and the MOSFET. Rules:
   - The answer must be a function of the ATtiny's unpredictable challenge (LFSR seeded from WDT/RC jitter).
   - At most one SPI TX FIFO word per pass, so a pre-filled 8-deep PL022 FIFO cannot outlive a hang.
   - The answer must be produced in the same pass as the pump toggle.
   - Word the evidence as "two cuts, one decision-maker".
   - For P2, give the ATtiny its own view of the e-stop (one more PC817).
3. **E-stop does not prevent restart on release.** NC1 cuts power (good). Preventing restart on release relies entirely on RP2350 firmware (ESTOP_SENSE plus the arm off/on rule). That is single-fault-tolerant at the sensing level (NC2 and ARM_SENSE) but not in hardware, which is required before P2 loads (ISO 13850 practice). **Recommended:**
   - Add **K0**, a bare 12 V SPDT relay (SRD-12VDC-SL-C) with a seal-in: ESTOP NC1 → K0 COM; K0 NO → LATCH_OUT → ARM.
   - Coil: LATCH_OUT → **NC2** → coil → GND, 1N4007 across the coil.
   - START: a spare CB16AY (5 in the pack) NO button between ESTOP_OUT and LATCH_OUT.
   - Move the e-stop sense to a PC817 on ESTOP_OUT (same GP2 polarity: pull-up, low = released).
   - This gives two hardware channels (NC1 and NC2) and no auto-restart after an e-stop or a power loss. The coil adds about 0.1–0.15 A on F1.
4. **Exit tests 4a/4b: accepted, with conditions.**
   - 4a validly proves that the stop path contains no software.
   - The bypass must be **one** labelled harness (three fused 1 A leads on one plug). The firmware's arm-time check (MID/RAIL live before K1 closes → refuse) catches a harness left in place.
   - Add **4c**: with the e-stop pressed, unplug the Pi, then release the e-stop → nothing re-energises (D1, D3, D4 static; D5/D6 low) until the arm is cycled, or START is pressed if K0 is fitted.
   - The Mac-hosted analyzer then needs its own TB-GND lead (the Pi path is gone).
5. **Analyzer and meter grounds:** the HiLetgo ground is already tied to GND_SYS through Pi USB and header pin 6, so the clip adds only risk. **Cut or insulate the clip**, and use a soldered short lead to a TB-GND post. A meter on the **current** range across the MOSFET or a lamp is the same bypass (A28): add that to the rule.
6. **RP2350 brownout (A3):** the BOD watches DVDD only (CONFIRMED, §6). Between about 1.8 and 3.3 V on IOVDD the chip runs in spec, so outputs stay under firmware control. The partial-gate region only heats the AOD4184. No change needed; correct the A3 text.
7. **WDTON start-up:** the first `wdr` or WDP setup must come before the shortest timeout, or the ATtiny reset-loops. That is safe, but it will look like a broken K2 at bring-up.
8. **F1 sizing:** 2 × 47 Ω dummies (0.51 A) + lamps (0.06 A) + K0 (≈ 0.12 A) ≈ 0.7 A on a 1 A ATO (≈ 70 %). Acceptable. Do **not** upsize F1 above 2 A in P1: K1 and K2 carry the same downstream short current in series, so a large fault current is a common-cause weld of both.
9. **Bus guard (RP2350 #2) on Pi USB:** its firmware must not expose the USB reset/BOOTSEL interface. Otherwise the AI host can reflash the guard with picotool and test 6 proves nothing.
10. **D4184 drive:** with a 4.7 kΩ pull-down plus the 1 kΩ+LED load at 4 mA drive, VOH sags. Set GP12/GP13 DRIVE to 8 mA and measure the gate ≥ 4.5 V at a 3.3 V input.
11. **MAX813L acceptance test (§5.3):** use the ICC limit for the grade you have (C 350 µA / E 500 µA). Run step 2 with VCC from M2 set ≥ 4.9 V, not through a diode.
12. **ATtiny85 from a marketplace seller:** the signature read (0x1E 0x93 0x0B) does not prove genuineness (clones copy it). Also run the BOD trip test: ramp 3V3_WD down and expect reset at 2.5–2.9 V. Record both.
13. **Bad data in `safety_paths.toml`:** B1 (wrong mechanism and bound), B6 (wrong), and A1 (K1 timing) are wrong. Rows are missing (section 4).

## 4. Single faults and common causes missing from tables A/B

| New row | Fault | Effect | Verdict as designed → with fixes |
|---|---|---|---|
| A31 | Relay IN shorted to GND (Dupont off-by-one, strand, splash) | That relay held regardless of its pump | WARN (K1: MID_SENSE; K2: invisible) → PASS with B5 keyed connectors + arm-time self-test |
| A32 | Relay module opto or driver transistor C-E short | Same as a welded relay | Same as A11/A12 |
| A33 | Relay coil open / module VCC lost | Relay open | PASS (can't arm) |
| A34 | Debug or ISP channel connected to the supervisor host | All three electronic cuts defeated by code | **FAIL** → PASS with B4 |
| A35 | ATtiny PB3 driven by timer hardware | Pump survives a CPU hang until the WDT | WARN → PASS with the B1 firmware rule |
| A36 | E-stop pressed then released with a firmware sense fault | Load re-energises on release | WARN → PASS with K0 (non-blocking 3) |
| A37 | Buck trimpot bumped high (5.5–12 V) | As A26 | PASS only with the B2 SCR |
| A38 | Key switch contacts welded or shorted | Rail B gated by MOS-B and lease only | PASS (as A13) |
| A39 | RP2350 Q&A answered while the safety logic is stalled (dual-core, IRQ or FIFO) | K1, K2 and the MOSFET all held | WARN → PASS with non-blocking 2 rules |
| B12 | Variant-B WDO→MR pulse train | K2 cycles 200 ms off / ≤ 2.25 s on | **FAIL** (claim) → PASS with B3(a) |
| B13 | U4 VCC through 1N4007 | U4 always in reset; K2 never closes | Availability FAIL → fixed by B3(b) |

Common causes checked across K1+K2:

| Common cause | Result |
|---|---|
| Shared 5 V rail lost or low | Safe: the coils are held only while the pumps run |
| Shared 5 V rail high | Safe **only with B2** |
| Shared ground lifted (module, WD board) | Safe: no coil or opto return; ATtiny phantom power is under BOD |
| Shared module, header or JD-VCC | **Not safe → B5** |
| Both relays welded by one downstream short | Safe at F1 ≤ 2 A (non-blocking 8) |
| Opto LED shorted | Safe (the IN chain still needs a sink) |
| Shared decision-maker | **Residual → non-blocking 2** |
| Shared host debug access | **Not safe → B4** |

## 5. Parts to add or swap

Amazon availability and delivery times for the listings below are UNVERIFIED (Amazon blocks automated fetches). Search the exact part numbers.

| Change | Part | Qty | Where |
|---|---|---|---|
| Swap M6 → crowbar | BT151-500R SCR, TO-220 | 2 (1 spare) | Amazon packs exist; genuine WeEn at DigiKey/Mouser |
| Add | TL431A TO-92 | 2 | Amazon pack, or TI TL431AILP at DigiKey |
| Add | 1 % 12.0 kΩ and 10.0 kΩ; 220 Ω; 1 nF and 10 nF ceramic | 2 each | Probably in the owner's assortment (check that it is 1 %) |
| Change M20 to Must | Second SunFounder 2-ch module (B00E0NTPP4) | 1 | Amazon |
| Add | JST-XH 2.54 mm connector kit with crimper (relay headers, SWD lead, WD 5 V) | 1 kit | Amazon |
| M10 additions (B1) | Film capacitors 1 µF and 2.2 µF (63 V polyester/PP) ×4 each, 220 nF film ×2; electrolytic 10 µF and 22 µF ×4 each | — | Amazon film and electrolytic assortment kits |
| M9 additions | BAT43 or BAT85 ×2 more (MR diode-AND) | — | Amazon |
| Values | 2.2 kΩ (R67), 4.7 kΩ (senses, R53/R63), 0 Ω link (R70), plus one more 2N3904 (Q64) | — | Owned / M8 |
| Recommended (non-blocking 3) | SRD-12VDC-SL-C bare relay (K0) + 1N4007; a spare CB16AY from #20 as START; one more PC817 | 1 | Amazon |
| Remove | 1N4007 in C601 (B3b); 1N5339B (superseded) | — | — |
| Keep (distributor only) | MAX813LCPA+ ×2 (M24): genuine only from DigiKey/Mouser | 2 | DigiKey/Mouser |
| Optional (distributor) | ATTINY85-20PU, genuine Microchip, ×2 as known-good references for the BOD/fuse acceptance | 2 | DigiKey/Mouser |

## 6. Bring-up measurements this review adds

- Relay module `I_IN` (B1 branch choice).
- K1 and K2 drop-out from the last pump edge to contact open: K1 ≤ 0.25 s at 50 Hz, K2 ≤ 25 ms.
- SCR crowbar trip voltage, and no trip at 5.20 V.
- U4 VCC ≥ 4.9 V.
- WDI-high ≥ 4.5 V.
- MR-low ≤ 0.5 V.
- ATtiny BOD trip point.
- RP2350 stepping (A2 affected; A3/A4 fixed).
