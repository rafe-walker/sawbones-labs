# Sawbones Labs P1: mock bench build

> **Revised after review 3** ([`review-3.md`](review-3.md), #297): crowbar re-wired (TL431A anode to GND, 2N3906 driver) behind a new 1 A fuse F3 (step 1); pump 2 and the K2 timing budget (steps 5, 6.3); Q64 and J_B2 deleted, hardware latch Q65 + Q66 with a panel RESET button (step 6.4); an optocoupler on the e-stop output at GP17 (step 3.1); tests 1, 2, 4 and 5 hosted on the Mac, D4 = RLY2 IN1 (step 9); corrected self-test expectations; shrouded 4-pin and 5-pin XH headers. Marked *revised after review 3*.
>
> **Revised after independent review** ([`independent-review.md`](independent-review.md), #296). Changed passages are marked. The main changes: the charge pumps are re-sized for the real 50 Hz toggle (step 5), the zener is replaced by an SCR crowbar (re-done in review 3), variant B is rewired (step 6.4), the SWD probe and every debug path are on the Mac only (step 7), the two cuts are on separate keyed relay modules, and tests 1, 2, 4 and 5 have corrected timings (4c is new).

A dry run of wiring the P1 bench, step by step, in the order a person would build it. Every connection lists its joint type. Steps that fail, or need a part not on the list, are marked **[FLAG]**. Part IDs: `#n` = line n of [`BOM.md`](BOM.md); `Mn` = missing part n in `compatibility.md` §6. Connection IDs match `data/netlist.toml`.

Joint types: **SB** soldered to board (component leg or header to perfboard), **WS** wired and soldered (wire to pad or perfboard), **ST** screw terminal, **DU** Dupont header, **CR** crimp/spade/ferrule, **BJ** barrel jack, **USB**, **RIB** ribbon/IDC, **PNL** panel-part screw terminal (lamp, switch, e-stop contact block screw).

Boards (all on Smraza perfboard #8, standoffs #4):

- **TR** trust-root board, 7×9 cm: RP2350-Zero #1, I2C header, PC817 sense bank, CH1 pump, firmware jumper, Pi UART header.
- **WD** watchdog board, 5×7 cm: ATtiny85 (U3, socket), MAX813L socket (U4), 3.3 V LDO, pump 2, Q_CLAMP, J_B.
- **PWR** power board, 5×7 cm: barrel adapter, F1, F2, 5 V buck, F3, SCR crowbar, TB-GND star (on the PWR board), TB-12V, TB-5V.
- **RLY1** relay module #17 (K1, channel 1) and **RLY2** a second, separate module M20 (K2, channel 1). *(revised after independent review)* Two modules, mounted apart on separate standoffs (B5).
- **MOS-A / MOS-B**: two AOD4184 modules #18.
- **INA**: INA228 #12. **OLED**: SSD1306 #19.
- **GUARD**: RP2350-Zero #2 + iiunius #9, 5×7 cm.
- **PANEL**: printed 22 mm panel (M16).

## Step 0. Before any wiring

1. Print the plate and panel (M16) on the A1 mini, in PETG, split to fit 180 mm. Holes: 4 × 22.3 mm (e-stop, key, 2 lamps on the top row), 4 × 22.3 mm (lamps), 1 × 16 mm (lease button), 1 toggle hole (measure the KN3C-101 bushing: UNVERIFIED, about 12 mm). **[FLAG: M16 must-have]**
2. Fit M2.5 nylon standoffs (#4) at 10 mm for Pi 5, TR, WD, PWR, RLY, MOS-A/B, INA, GUARD. Pi 5 uses its 58×49 mm M2.5 pattern.
3. Solder three wires to the RP2350-Zero #1 SWD pads (SWCLK, SWDIO, GND) **before** it is mounted; the pads are underneath (S-REPO cross-ref). Joint **WS**. End in a 3-pin female Dupont housing (**DU**). Do the same on RP2350-Zero #2.
4. Read and record each RP2350's package stepping (A2/A4).
5. Program the ATtiny85 off-board (Step 9) before it goes into its socket.

## Step 1. 12 V input and fusing (PWR board)

| Conn | From | To | Joint | Note |
|---|---|---|---|---|
| C101 | Facmogu 12 V adapter #27 plug | Barrel→terminal adapter M3 | **BJ** | **[FLAG: M3 missing]** |
| C102 | M3 + / − | PWR TB-12V-IN | **ST** (22 AWG red/black, ferrules M17) | |
| C103 | TB-12V-IN + | Inline fuse holder #15 lead A | **WS** + heat shrink (12 AWG → 22 AWG step-down) | Holder 1 = F1 (load) |
| C104 | Fuse holder F1 lead B | TB-12V-F1 | **WS** + heat shrink | **F1 = 1 A ATC [FLAG: M4 missing; kit is 5–30 A]** |
| C105 | TB-12V-IN + | Fuse holder F2 | **WS** + heat shrink | **F2 = 1 A [FLAG: M4]** |
| C106 | F2 out | Buck M1 IN+ | **ST** or **WS** | **[FLAG: M1 missing]** |
| C107 | TB-12V-IN − | TB-GND star | **ST** | The single star ground for the whole bench |
| C108 | Buck M1 OUT+ | **F3** 1 A genuine Littelfuse *(revised after review 3 R3-B2)* | **WS** | Set 5.05 V **before** connecting loads; record it |
| C108b | F3 out | TB-5V | **WS** | The crowbar and every load are downstream of F3, so a crowbar fire blows F3 (computed 0.02–0.5 s at the LM2596 limit) |
| C109 | TB-5V ↔ TB-GND | **SCR crowbar** *(revised after review 3 R3-B1)*: BT151-500R anode to +5 V, cathode to GND (clip-on heatsink). **TL431A anode to GND**, cathode through 1.0 kΩ to the base of **Q_CB 2N3906** (680 Ω base-emitter, emitter on +5 V); Q_CB collector through 100 Ω to the SCR gate; 1.0 kΩ and 10 nF gate to cathode. REF from a 12.0 kΩ 1 % (to +5 V) / 10.0 kΩ 1 % (to GND) divider with 1 nF REF to GND. Computed trip 5.375–5.65 V (HW-029). Replaces the review-2 anode-to-gate wiring, which tripped at 6–16 V | **SB** | **[FLAG: M6, M30 missing]** Bench acceptance before use: feed TB-5V from M2 (0–7 V) through F3 with the buck disconnected and ramp up slowly; it must fire at 5.375–5.65 V and not at 5.20 V for 10 minutes |
| C110 | TB-5V ↔ TB-GND | 10 µF + 100 nF | **SB** | |
| C111 | L_PWR lamp #16 | TB-12V-F1 to TB-GND | **PNL** + **ST** | "12 V present", before the e-stop |

Check at the end of step 1: meter shows 12.0 V on TB-12V-F1 and 5.05 V on TB-5V. Then pull F2 and confirm the 5 V rail drops to 0 V; check F3 continuity (a genuine 0257001.PXPV, not a pack-in fuse). Do not upsize F1 above 2 A in P1 (independent review, non-blocking 8).

## Step 2. The load power path (series chain)

All load-path wiring is 22 AWG silicone #7, landed in screw terminals, never Dupont.

| Conn | From | To | Joint |
|---|---|---|---|
| C201 | TB-12V-F1 | E-stop #23 NC1 terminal 1 | **ST** → **PNL** (ferrule) |
| C202 | E-stop NC1 terminal 2 | Arm toggle #21 pin 1 | **PNL** → **PNL** (toggle pins are solder lugs or spade: **CR** 6.3 mm spade if spade, else **WS**; UNVERIFIED) |
| C203 | Arm toggle pin 2 | Node ARM_OUT on TB-ARM (TR edge) | **CR**/**WS** → **ST** |
| C204 | TB-ARM | RLY1 CH1 COM (K1) | **ST** |
| C205 | RLY1 CH1 NO | Node MID (TB-MID) | **ST** |
| C206 | TB-MID | RLY2 CH1 COM (K2, the second relay module) | **ST** |
| C207 | RLY2 CH1 NO | INA228 IN+ | **ST** |
| C208 | INA228 IN− | Node RAIL_BUS (TB-BUS) | **ST** |
| C209 | TB-BUS | L_BUS indicator lamp #16 → TB-GND | **PNL** |
| C210 | TB-BUS | MOS-A "+" (supply in) and MOS-A load "+" | **ST** |
| C211 | MOS-A "−" (supply) | TB-GND | **ST** |
| C212 | Lamp A (#16, or M15 load) + | MOS-A load "+" | **PNL**/**ST** |
| C213 | Lamp A − | MOS-A LOAD (switched) | **PNL** → **ST** |
| C214 | TB-BUS | Key switch #22 NO terminal 1 | **ST** → **PNL** |
| C215 | Key NO terminal 2 | Node RAIL_B_HI (TB-BHI) | **PNL** → **ST** |
| C216 | TB-BHI | MOS-B "+" supply and load "+" | **ST** |
| C217 | MOS-B "−" | TB-GND | **ST** |
| C218 | Lamp B + / − | MOS-B "+" / LOAD | **PNL** → **ST** |

Notes:

- Do not join the MOS-x "+" supply terminal to the bench 12 V directly. Feeding it from TB-BUS (A) and TB-BHI (B) means no gate drive exists unless the upstream chain is closed. That gives a hardware dependency for free.
- The "+" terminals being joined on the module: UNVERIFIED, check continuity (S-D4184).
- **[FLAG: M15 should-have]** With pilot lamps as the loads, the current is 20 mA (40 µV on the INA228 shunt). Fit a 47 Ω 5 W resistor in parallel with each lamp for current evidence.
- **Lamp-negative (MOS LOAD terminal) is NOT ground.** Mark it red on the plate: no analyzer or meter ground clip ever goes there.

## Step 3. 5 V and 3.3 V rails, RP2350 trust root (TR board)

| Conn | From | To | Joint |
|---|---|---|---|
| C301 | TB-5V | TR 5 V input terminal → RP2350-Zero #1 5V pin | **ST** → **WS** → **SB** (Zero on female headers M19) |
| C302 | TB-GND | TR GND → Zero GND | **ST** → **WS** |
| C303 | Zero 3V3 pin | TR 3V3 bus (INA, OLED, sense pull-ups, lease-LED base) | **SB**/**WS** |
| C304 | TB-5V | RLY1 VCC and JD-VCC (jumper fitted) and RLY2 the same *(revised after independent review B5)*; IN2 of each module tied to VCC through 1 kΩ so it cannot float low | **ST** → keyed **JST-XH** 4-pin (not a bare Dupont strip) |
| C305 | TB-GND | RLY1 GND and RLY2 GND | **ST** → keyed **JST-XH** |

**The trust root's USB-C is never connected** (S-ZERO-SCH: no diode between VBUS and the 5 V pin). Flashing is over SWD only. Put tape over the USB-C socket.

### 3.1 RP2350-Zero #1 (trust root) pin assignment

This is a changed map. The firmware placeholders in `rp2350.rs` must move (compatibility.md §4, F1–F3).

| GPIO (edge) | Function | Direction | Circuit | Pull | Firmware name | Changed? |
|---|---|---|---|---|---|---|
| GP0 | PULSE UART0 TX → Pi | out | 1 kΩ series → Pi pin 10 | — | (new) | new |
| GP1 | PULSE UART0 RX ← Pi | in | 1 kΩ series ← Pi pin 8 | internal pull-up | (new) | new |
| GP2 | E-stop sense: NC2 to GND | in | NC2 contact; open = pressed | 10 kΩ ext. + int. pull-up | ESTOP | same |
| GP3 | Arm sense (PC817 U-ARM out to GND) | in | Opto on ARM_OUT | 10 kΩ ext. pull-up | ARM_SWITCH | same |
| GP4 | Key sense (PC817 U-KEY on RAIL_B_HI) | in | | 10 kΩ ext. pull-up | KEY_SWITCH | same |
| GP5 | Lease button NO to GND | in | #20 NO contact | 10 kΩ ext. pull-up | PANEL_BUTTON | same |
| GP6 | Firmware jumper to GND | in | 2-pin header + shunt (M18) | 10 kΩ ext. pull-up | FIRMWARE_JUMPER | same |
| GP7 | Rail A sense (PC817 across lamp A) | in | Opto emitter → GP7 | **4.7 kΩ ext. pull-down** (E9: 8.2 kΩ or less; *revised after independent review*) | RAIL_A_SENSE | same |
| GP8 | Q&A SPI1 RX ← ATtiny PB1 (DO) | in | 1 kΩ series | — | (new) | was RAIL_B_SENSE[0] |
| GP9 | SPI1 CSn, tied low | in | 1 kΩ to GND | — | (new) | was RAIL_B_SENSE[1] |
| GP10 | Q&A SPI1 SCK ← ATtiny PB2 | in | 1 kΩ series | — | (new) | was RAIL_B_SENSE[2] |
| GP11 | Q&A SPI1 TX → ATtiny PB0 (DI) | out | 1 kΩ series | — | (new) | was RAIL_B_SENSE[3] |
| GP12 | Rail A enable → MOS-A PWM | out | | **4.7 kΩ ext. pull-down** | RAIL_A_ENABLE | same |
| GP13 | Rail B port 0 enable → MOS-B PWM | out | | **4.7 kΩ ext. pull-down** | RAIL_B_ENABLE[0] | same |
| GP17 | *(Revised after review 3 R3-B5.)* ESTOP_OUT: optocoupler U-EOUT on the e-stop output, collector with a 10 kΩ pull-up (bottom pad, wired like the SWD pads). The arm sense is downstream of the e-stop, so without this the firmware cannot see the stop (HW-036). Option (b): make K0 a must | in | U-EOUT | 4.7 kΩ | — | — |
| GP14 | Charge pump 1 → K1 hold, toggled once per 10 ms tick (50 Hz); pad drive 8 mA | out | Pump stage (step 5) | — | — | — |
| GP15 | Lease-button LED driver | out | 10 kΩ → 2N3904 base | 100 kΩ pull-down | (new) | was RAIL_B_ENABLE[2] |
| GP26 | I2C1 SDA (INA228, OLED) | i/o | Module pull-ups to 3V3 | — | (new) | new |
| GP27 | I2C1 SCL | out | | — | (new) | new |
| GP28 | Rail B sense (PC817 across lamp B) | in | | **4.7 kΩ ext. pull-down** | RAIL_B_SENSE[0] | was GP8 |
| GP29 | MID sense (PC817 on node MID) | in | | **4.7 kΩ ext. pull-down** | (new) | new |
| GP16 | On-board WS2812 status (leave) | — | Board-internal | — | — | do not use as an output |
| SWD pads | SWCLK, SWDIO, GND → Pico probe on the **Mac** *(revised after independent review B4)* | — | Keyed 3-pin **JST-XH**, unplugged except for flashing and test 5a | — | — | — |

Set `RAIL_B_PORTS = 1` for P1, which frees GP9–11 and GP14–16. With no 5th rail-B pin left, `RAIL_B_ENABLE[1..3]` must not exist in the P1 build.

### 3.2 PC817 sense bank (TR board, all **SB**)

| Opto | LED side (12 V domain) | Output side (3V3 domain) |
|---|---|---|
| U-ARM | ARM_OUT → 4.7 kΩ → LED → TB-GND | Collector → GP3 (pull-up), emitter → GND. Live = low = "on" |
| U-KEY | RAIL_B_HI → 4.7 kΩ → LED → GND | Collector → GP4, emitter → GND |
| U-MID | MID → 4.7 kΩ → LED → GND | Collector → 3V3, emitter → GP29 (4.7 kΩ pull-down). Live = high |
| U-RA | Lamp A + → 4.7 kΩ → LED → Lamp A − (**across the load**) | Collector → 3V3, emitter → GP7. Live = high |
| U-RB | Lamp B + → 4.7 kΩ → LED → Lamp B − | Collector → 3V3, emitter → GP28 |

Joint: the 12 V sense wires from the nodes to TR are **ST** at both ends (TB on TR). Add a 1N4148 reverse-parallel across each opto LED to protect against reverse voltage. **[FLAG: M7 PC817 ×5 missing]**

### 3.3 I2C peripherals

| Conn | From | To | Joint |
|---|---|---|---|
| C331 | TR 3V3 / GND / GP26 / GP27 | INA228 VS / GND / SDA / SCL | **DU** (4-pin) |
| C332 | Same bus | OLED VCC / GND / SDA / SCL | **DU** |
| C333 | INA228 GND | TB-GND (it must share load ground for VBUS) | via TR GND |

Bring-up: run `i2cdetect`-equivalent from firmware. Expect 0x40 (INA228) and 0x3C (OLED): UNVERIFIED defaults.

## Step 4. Panel switches to the logic

| Conn | From | To | Joint |
|---|---|---|---|
| C401 | E-stop NC2 terminal 1 | TR GP2 terminal | **PNL** → **ST** |
| C402 | E-stop NC2 terminal 2 | TR GND | **PNL** → **ST** |
| C403 | Lease button #20 NO (C/NO lugs) | TR GP5 / GND | **WS** (lugs) → **ST** |
| C404 | Lease LED + | TB-12V-IN after F2 (logic 12 V; lights with the e-stop pressed) | **WS** → **ST** |
| C405 | Lease LED − | 2N3904 collector (TR), emitter GND, base 10 kΩ ← GP15 | **WS** → **SB** |
| C406 | Firmware jumper header | GP6 / GND | **SB** + shunt |

## Step 5. Relay drive stage 1 (RP2350 dead-man → K1) on TR

> **Revised after independent review (B1).** The firmware toggles GP14 once per 10 ms tick, a **50 Hz** square wave, not 1 kHz. The original values (100 nF, 1 µF, 100 kΩ, 10 kΩ) give HOLD1 ≈ 0.6 V at 50 Hz: **K1 never pulls in**. Keep 50 Hz; do not move the pump to PWM, PIO or a timer. Set the GP14 pad drive to 8 mA (**UNVERIFIED until #288**). *(Revised after review 3 R3-B3:)* HW-031 now runs an event model at the worst corner and computes the hold voltage and the drop-out window; the review-2 limits of 3 and 9 mA failed at that corner. The firmware pin is **GP14**, not GP17 (GP17 is the U-EOUT pad).

**First measure the relay module's input current `I_IN`:** short IN1 to GND through the meter on the mA range with VCC at 5.05 V.

| Ref | Set 1 (`I_IN` ≤ 2 mA) | Set 2 (2 mA < `I_IN` ≤ 5 mA) | Joint |
|---|---|---|---|
| R51 | 470 Ω, GP14 → P1A | same | **SB** |
| C51 | **1 µF film** (polyester or PP, not ceramic: an MLCC crack-short turns the pump into DC coupling, A17) | **2.2 µF film** | **SB** |
| D51, D52 | BAT43 (D51 anode GND, cathode P1B; D52 anode P1B, cathode HOLD1) | same | **SB** |
| C52 | **10 µF electrolytic**, HOLD1 → GND | **22 µF electrolytic** | **SB** |
| R52 | **47 kΩ**, HOLD1 → GND | 47 kΩ | **SB** |
| R53 | **10 kΩ**, HOLD1 → Q51 base | **4.7 kΩ** | **SB** |
| Q51 | 2N3904, emitter GND, collector → RLY1 IN1 | same | **SB** → keyed **JST-XH** to the module |
| Computed (HW-031, worst corner) | PUMP1 drop-out 66–203 ms | PUMP1B drop-out 132–313 ms | |

If `I_IN` is above 5 mA, do not use this stage: ask for a redesign (MOSFET gate stage). **[FLAG: M8, M9, M10, M25 missing.]** Measure the real drop-out from the last GP14 edge to contact open: K1 at most 0.34 s (the A14 bound of 340 ms).

Expected: 50 Hz square wave on GP14 → HOLD1 above 1.2 V → Q51 on → IN1 near 0.1 V → K1 closed. GP14 stuck: HOLD1 decays and K1 releases within about 0.07–0.31 s depending on the set (plus relay release time, UNVERIFIED). **K1 is therefore not the cut that meets the 120 ms frozen-RP2350 bound: K2 is** (step 6.3).

## Step 6. Watchdog board (WD): ATtiny85 + pump 2 + variant B provisions

### 6.1 Supply

| Conn | From | To | Joint |
|---|---|---|---|
| C601 | TB-5V | WD 5 V in, **straight from TB-5V, no diode** *(revised after independent review B3b: the 1N4007 left U4 below its reset threshold, so K2 could never close in variant B)*; keyed 2-pin **JST-XH** for polarity | keyed **JST-XH** → **SB** |
| C602 | WD 5 V | LDO M5 in, 3V3_WD out (1 µF/10 µF per LDO type) | **SB** |
| C603 | TB-GND | WD GND | **ST** |

**[FLAG: M5 missing]**

### 6.2 ATtiny85 (U3, in a DIP-8 socket M12) pin table

| Pin | Port | Function | Circuit | Joint |
|---|---|---|---|---|
| 1 | PB5 / RESET | Reset | 10 kΩ pull-up to 3V3_WD; momentary "watchdog reset" button to GND | **SB** |
| 2 | PB3 | Charge pump 2 out (about 1 kHz, only while the Q&A is good) | → R61 470 Ω → pump 2 | **SB** |
| 3 | PB4 | WD_TOGGLE out: toggles each good Q&A cycle (≈10 Hz). Drives a green LED through 1 kΩ (both variants) and the WDI level shifter (variant B) | | **SB** |
| 4 | GND | | | **SB** |
| 5 | PB0 / DI (ISP MOSI) | Q&A data in ← RP2350 GP11 | 1 kΩ series | **SB** + **ST** to TR |
| 6 | PB1 / DO (ISP MISO) | Q&A data out → RP2350 GP8 | 1 kΩ series | **SB** + **ST** |
| 7 | PB2 / USCK (ISP SCK) | Q&A clock out → RP2350 GP10 | 1 kΩ series | **SB** + **ST** |
| 8 | VCC | 3V3_WD | 100 nF + 10 µF | **SB** |

Q&A link wiring TR↔WD: 4 wires (GP8, GP10, GP11, GND), twisted, about 15 cm, landed **ST** both ends.

### 6.3 Pump 2 → K2 *(revised after independent review B1)*

R61 470 Ω, **C61 220 nF film, R63 4.7 kΩ**, D61/D62 BAT43, **C62 470 nF film, R62 47 kΩ** *(revised after review 3 R3-B3; was 1 µF / 100 kΩ)*, Q61 2N3904 → RLY2 IN1 (the second relay module). All **SB**; to the relay header, a **shrouded 4-pin B4B-XH-A** (K1 is the 5-pin B5B-XH-A, so the two plugs cannot swap). The node HOLD2 = C62 top. Computed drop-out (HW-031) 4–7.5 ms.

**K2 timing budget (HW-035, BUD_K2), revised after review 3:** the ATtiny Q&A period is **25 ms**, the firmware trips on the **first bad answer**, and the ATtiny clock is calibrated (OSCCAL, 1 % tolerance assumed). Stop of answers within 2 × 25 ms × 1.01 + pump drop-out 7.5 ms + relay release 10 ms ≈ 68 ms, under the 120 ms bound. The review-2 rail-B bound of 30 ms was wrong; it is 25 ms with the budget shown.

### 6.4 Variant B provisions (populated now, inert in variant A)

| Ref | Connection | Joint |
|---|---|---|
| U4 | DIP-8 socket (M12) for MAX813L. Pin 2 → WD 5 V, pin 3 → GND, 100 nF across | **SB** |
| R64 12 kΩ / R65 10 kΩ | 3V3_WD → R64 → U4 pin 4 (PFI) → R65 → GND. Trip 2.64–2.86 V | **SB** |
| Q62 2N3904 + R66 10 kΩ base (from PB4) + **R67 2.2 kΩ** pull-up to 5 V *(revised after independent review B3c: 10 kΩ left no WDI margin)* | Q62 collector → U4 pin 6 (WDI). Level shift 3.3 → 5 V, inverted | **SB** |
| D63, D64 **BAT43** *(revised after independent review B3d: 1N4148 leaves MR too high)* | U4 pin 8 (WDO) and pin 5 (PFO) each through a diode (cathode to the pin, anode to MR, pin 1). Diode-AND: either low pulls MR low | **SB** |
| R68 10 kΩ, R69 100 kΩ | U4 pin 7 (RESET, active-high) → R68 → Q63 base; R69 base → GND | **SB** |
| Q63 2N3904 (Q_CLAMP) | Emitter GND; collector → **R70 = 0 Ω** → J_B pin 1 *(revised after independent review B3e)* | **SB** |
| J_B | 2-pin header: pin 2 → the **Q61 base** (not HOLD2): Q63 Vce(sat) holds Q61 off whatever the pump does. **Variant A: open. Variant B: shunt fitted** | **SB** + shunt |
| Q65 2N3904 + Q66 2N3906 + R68 10 kΩ | *(Revised after review 3 R3-B4: replaces Q64 and J_B2, which are deleted.)* Cross-coupled latch. U4 RESET → 10 kΩ → Q65 base (100 kΩ base to GND). Q65 collector → 10 kΩ → Q66 base (47 kΩ to 5 V). Q66 collector → 10 kΩ → Q65 base (hold path) and → R68 10 kΩ → Q63 base. While latched, Q63 holds Q61 off and K2 open | **SB** |
| BTN_CLR | Panel RESET button (the spare CB16AY from the pack): NO → Q65 base, C → TB-GND. **Only a person clears the latch** | **PNL** + **SB** |

*(Revised after review 3 R3-B4.)* **RESET is a 140–280 ms pulse, not a latch:** WDO low pulls MR low, which clears the watchdog timer, so RESET releases and the cycle repeats. The pulse sets the Q65/Q66 hardware latch, so K2 stays open until a person presses the panel RESET button; the RP2350 firmware also latches `OutputMismatch` on any unexpected VBUS or MID loss while armed. Nothing drives the ATtiny RESET pin: a RESET driver wired to an ATtiny whose ISP pins sit on another part is a hazard (HW-038), so Q64 and J_B2 are gone. HW-033 requires `latch` to be `hardware` with a human-cleared latch part. Variant A = U4 empty, J_B open. Variant B = U4 fitted (after passing the §5.3 acceptance test in compatibility.md) and J_B shorted. The rail path is not rewired. **[FLAG: M11, M12]**

### 6.5 Fuses and lock (record in evidence)

The fuse bytes below are a design choice; check them against the datasheet fuse tables before writing.

- lfuse 0xE2: internal 8 MHz, CKDIV8 off, default start-up time.
- hfuse 0xCD: SPIEN on, WDTON on (watchdog always on), BODLEVEL 101 = 2.5–2.9 V.
- efuse 0xFF.
- Read all three back, then set lock 0xFC (mode 3; fuses lock too, S-T85).
- Read signature 0x1E 0x93 0x0B. *(Revised after independent review.)* The signature does not prove a genuine chip (clones copy it): also run the BOD trip test (ramp 3V3_WD down, expect reset at 2.5–2.9 V) and record both.
- Verify the flash **before** writing the lock: in lock mode 3 verification is disabled.

## Step 7. Pi 5, Pico probe, USB

### 7.1 Pi 5 header (40-pin)

| Pin | Signal | To | Joint |
|---|---|---|---|
| 6 | GND | TR GND (PULSE reference) | **DU** |
| 8 | GPIO14 TXD0 | 1 kΩ → RP2350 #1 GP1 | **DU** → TR **SB** |
| 10 | GPIO15 RXD0 | ← 1 kΩ ← RP2350 #1 GP0 | **DU** |
| 1 / 17 | 3V3 | Unused *(revised after independent review B4: the Pi is not used for ISP)* | — |
| 19 / 21 / 23 / 22 / 20 | GPIO10 / GPIO9 / GPIO11 / GPIO25 / GND | *(Revised after independent review B4.)* **No wiring.** The Pi gets no path to ATtiny ISP, SWD or BOOT. Program the ATtiny off-board from the Mac side, then remove the lead | — |

Configure `dtparam=uart0=on`, 3.3 V; device `/dev/ttyAMA0` (UNVERIFIED name on Pi 5).

### 7.2 Pico (Bicool #26) as the SWD probe, hosted on the Mac *(revised after independent review B4)*

Over SWD a host can halt the RP2350 and write RAM and flash, then toggle GP14, answer the public Q&A and drive GP12: one connected lead defeats K1, K2 and the MOSFET together. So the Pico runs the Raspberry Pi `debugprobe` firmware (UF2) and is USB-hosted **on the MacBook only, never on the Pi**. Wires: Pico GP2 = SWCLK, GP3 = SWDIO, GND (pin 3) → a **keyed JST-XH** 3-pin SWD lead to RP2350-Zero #1 (or #2). The lead is **unplugged** except while flashing and during test 5a, which is run from the Mac. The Pico connects to the Mac over micro-USB (**[FLAG: M13 missing]**). HW-030 fails any netlist where the AI-controlled Pi can reach a trust-root debug or programming pin. Longer term: RP2350 OTP debug-disable on the trust root (F6).

The ATtiny is programmed **off-board**, from the Mac side, and the ISP wiring is removed after use.

### 7.3 USB

| Pi port | Device | Cable | Joint |
|---|---|---|---|
| USB3 upper | Logic analyzer #25 | Its own USB-A/mini or micro lead | **USB** |
| USB3 lower | RP2350-Zero #2 (bus guard) | USB-A→C #3 (check it carries data) | **USB** |
| USB2 upper | *(Revised after independent review B4.)* Unused. The Pico probe is on the MacBook | — | — |
| USB2 lower | Waveshare USB-RS485 #10 | USB-A→B (M14 if not boxed) | **USB** |

The SH-G01A is not used in P1 (compatibility.md 2.5).

## Step 8. RS485 loopback (GUARD board), decision: second RP2350

The bus guard runs on **RP2350-Zero #2**, not on the trust root. The threat model puts one guard per pod, the trust root keeps its pins, and a guard crash cannot touch the rail logic.

| Conn | From | To | Joint |
|---|---|---|---|
| C801 | RP2350 #2 3V3 / GND | iiunius TTL VCC / GND | **SB**/**WS** |
| C802 | RP2350 #2 GP0 (UART0 TX) | iiunius TXD | **WS** |
| C803 | RP2350 #2 GP1 (UART0 RX) | iiunius RXD | **WS** |
| C804 | iiunius A+ / B− | Waveshare A+ / B− | **ST** both ends, twisted pair 0.5 m |
| C805 | iiunius GND-iso | Waveshare GND (RS485 side) | **ST** |
| C806 | RP2350 #2 USB-C | Pi USB3 lower | **USB** (bus powered; this board has no external 5 V, so there is no back-feed issue) |

Termination: Waveshare has 120 Ω on board (S-WS485). The iiunius one is UNVERIFIED. With the pair idle, measure A−B with the meter: it must be ≥ +200 mV. If not, remove one terminator. Pin names on the iiunius (TXD/RXD vs RO/DI) are UNVERIFIED; swap TX/RX if the loopback is silent.

## Step 9. Logic analyzer: ground and probe maps

**Ground lead → TB-GND star only, a short soldered lead to a TB-GND post; cut or insulate the analyzer's clip** *(revised after independent review)*. A meter on a current range across the MOSFET, or a lamp, is the same bypass (A28). Never lamp-negative (switched node, compatibility.md G3). All channels are 3.3 V nets, or ≤5.10 V on IN1/IN2 (inputs are 5.25 V absolute max, S-LA). Host: *(revised after review 3 R3-B6)* **the MacBook for tests 1, 2, 4 and 5** (the Pi is an AI host, or off, and HW-022 rejects evidence hosted there); the Pi may host tests 3 and 6. The analyzer buffer is verified input-only (HW-030 `input_only_verified`). **[FLAG: confirm sigrok/PulseView runs on the M3 Mac first.]**

| Ch | Test 1 boot/brownout | Test 2 heartbeat | Test 3 lease | Test 4 e-stop, Pi off | Test 5 watchdog cut | Test 6 bus guard |
|---|---|---|---|---|---|---|
| D0 | GP12 rail A enable | GP12 | GP13 rail B enable | GP2 e-stop sense | GP12 | Guard GP0 TX (wire-bound) |
| D1 | GP14 pump 1 | GP14 | GP5 lease button | GP12 | GP14 pump 1 | Guard GP1 RX |
| D2 | PB3 pump 2 | PB3 | Pi TX (GPIO14) | GP14 | PB3 pump 2 | RS485 A line (ground on GND-iso: bridges isolation, record it) or unused |
| D3 | RLY1 IN1 = Q51.C | RLY1 IN1 | RP TX (GP0) | RLY1 IN1 | RLY1 IN1 | GP12 (armed precondition) |
| D4 | RLY2 IN1 = Q61.C | RLY2 IN1 | GP12 | RLY2 IN1 | RLY2 IN1 | — |
| D5 | GP7 rail A sense | GP7 | GP28 rail B sense | GP7 rail A sense | GP7 | — |
| D6 | RP2350 3V3 (power marker) | Pi TX (GPIO14) | GP4 key sense | GP28 rail B sense | GP10 Q&A SCK | — |
| D7 | 3V3_WD (A) / U4 RESET via 10 kΩ (B) | RP TX (GP0) | GP15 lease LED | GP29 MID sense | GP11 Q&A answer (A) / U4 RESET (B) | — |

Probes: Dupont **DU** onto 2.54 mm test pins soldered on TR and WD (**SB**). Never use hook clips on screw terminals.

## Step 10. The six exit tests, walked through the mock

The timing bounds use the current placeholder constants: tick 10 ms, HEARTBEAT_PERIOD 10 ticks, so the loss bound is 2 periods = 200 ms (S-REPO interlock crate). Q&A period 25 ms *(revised after review 3)*; the budget is in step 6.3. Pump τ is from the step 5 values.

### Test 1. Rails off at boot and during brownout

- **Setup.** Analyzer on the **MacBook**. Trust-root and watchdog 5 V fed from the adjustable source M2 instead of the buck (move C301/C601 to M2's output; **[FLAG: M2 missing]**). Arm toggle ON, key ON, e-stop released, Pi running.
- **Steps.**
  1. Power-up, 20×.
  2. RESET button on the RP2350-Zero while armed.
  3. Slow sag 5.0 → 0 V → 5.0 V at about 0.1 V/s. *(Revised after review 3:)* step M2 in 50 mV steps through 4.4–4.8 V to find the U4 threshold, and take it to 7 V on the bench-acceptance check of the crowbar only with F3 fitted.
  4. Fast cut (switch M2's input).
  5. Hung Pi: `kill -STOP` the supervisor.
- **Expected.**
  - D0 never rises before the firmware sees the arm toggle go off then on. D3/D4 stay high (relays off). D5 stays low. The meter on lamp A reads 0 V. D6 marks the power edges.
  - On reset while armed: D0 falls within µs, D1 stops, D3 rises after K1's pump drop-out, **about 0.07–0.31 s at the 50 Hz toggle** *(revised after review 3)*, D5 falls with D0.
  - Sag: the RP2350 resets in its brownout window, and the ATtiny BOD trips at 2.5–2.9 V on 3V3_WD (D7, variant A). Variant B: U4 RESET rises at 4.50–4.75 V on the 5 V rail.
  - No glitch on D0, D3 or D4 at any point.
- **Mock result.** PASS with changes (M2, external pull-downs).

### Test 2. Lost P.U.L.S.E. heartbeat opens the rails within the bound

- **Setup.** Armed. Pi supervisor running. Analyzer on the **MacBook** *(revised after review 3 R3-B6)*.
- **Steps.** Stop the heartbeat only: pull the Pi TX Dupont (pin 8), or `kill -STOP`.
- **Expected.**
  - t0 = the last valid reply from the Pi (the Pi TX / PULSE reply edge), not an RP2350 line.
  - D0 (GP12) falls at ≤ t0 + 200 ms + 1 tick.
  - D1 stops at the same pass. D3 rises after K1's pump drop-out, **0.03–0.22 s** *(revised after independent review B1; was ≤ 20 ms)*.
  - D5 falls with D0 (the MOSFET opens in tens of µs).
  - A fault latches. After an RP2350 restart, the OLED still shows it.
  - The ATtiny (D2) keeps pumping. That is correct: the RP2350 is healthy, only the Pi is lost.
- **Mock result.** PASS.

### Test 3. Lease is single-use and bound to its action digest; replay refused

- **Setup.** Armed, key ON.
- **Steps.**
  1. The supervisor requests a class 3 lease for port 0 with digest X. Press the lease button (D1).
  2. The rail-B action runs once (D0 GP13 high, D5 high).
  3. Use the lease again → refused.
  4. Use it with digest Y → refused, and the lease ends.
  5. Request, wait past LEASE_TTL, then press → refused.
- **Expected.** D0 rises only after a fresh press while a request is pending. It falls on LeaseUsed. Steps 3–5 show no D0 edge. The refusals are decoded on D2/D3 (UART) and shown on the OLED. Key off at any time → D0 falls. Signed replay needs #172 (S-REPO THREAT-MODEL): until then step 3 is a link replay of an unsigned grant.
- **Mock result.** PASS (firmware-dependent).

### Test 4. The e-stop opens the rails with the Pi powered off (#289)

The test as written contradicts the heartbeat rule (compatibility.md F5). Mocked as two parts. Analyzer on the **MacBook**.

- **4a (power path, no Pi, no firmware help).**
  - Setup: the Pi fully unplugged. Fit the **bypass fixture**: three fused (1 A) clip leads across K1 COM–NO, K2 COM–NO and MOS-A/MOS-B LOAD–"−". It is labelled TEST ONLY and is removed after the test. TR and WD stay powered (they run without the Pi). Lamps A and B are lit through e-stop + arm + key only.
  - Step: press the e-stop.
  - Expected: D0 (GP2) rises. D5 and D6 (both rail senses) fall within the contact-opening time (< 10 ms after D0), and the lamps go dark. The meter across the e-stop NC1 terminals reads 12 V (open). This proves that no software and no Pi is in the stop path.
- **4b (full system).**
  - Steps: armed with the Pi on; press the e-stop, then unplug the Pi.
  - Expected: D5 falls within ms of D0, long before the 200 ms heartbeat bound. Nothing re-energises after the Pi dies (D1/D3/D4 stay off). Releasing the e-stop does not re-arm: a fault is latched and the arm toggle must go off then on.
- **4c (restart on release; added after independent review).** Press the e-stop, unplug the Pi, then release the e-stop. Expected: nothing re-energises (D1, D3, D4 static; D5 and D6 low) until the arm toggle is cycled, or until START is pressed if the recommended **K0** seal-in relay is fitted (open owner decision).
- The bypass fixture is **one** labelled harness (three fused 1 A leads on one plug); the firmware's arm-time check (MID or RAIL live before K1 closes: refuse) catches one left in place.
- **Mock result.** PASS WITH CHANGES (bypass fixture, Mac-hosted analyzer, test text rewritten, 4c).

### Test 5. The watchdog cut fires when the RP2350 is frozen or hung

- **Setup.** Armed. Pico probe on the trust root's SWD, **hosted on the MacBook**, keyed lead plugged only for this test *(revised after independent review B4)*. The mandatory arm-time per-cut self-test (B5) has passed: *(corrected after review 3 R3-B7, expectations computed by HW-037:)* K1 open gives MID dead and VBUS dead; K2 open gives MID live and VBUS dead; MOS_A off gives RAIL_A_SENSE dead and VBUS live.
- **Steps.**
  - **5a.** `probe-rs` halt of **both cores** of the RP2350 *(revised after review 3)*.
  - **5b.** Reset loop: hold the RESET button / a test build that reboots every 30 ms.
  - **5c.** Test firmware that answers the Q&A wrongly but keeps its own pump running.
  - **5d.** Pull the WD board's 5 V.
  - **5e.** Repeat 5a and 5c with a **K1 bypass link** (TEST ONLY), to prove K2 opens on its own.
  - **5g.** A39 as a documented limit: firmware that answers the Q&A while the safety logic is stalled. The bench stays ON; record it, do not claim a pass.
  - **5f (variant B only).** A test ATtiny build that keeps PB3 running but stops PB4: U4 RESET rises after 1.00–2.25 s, and K2 drops through the clamp.
- **Expected.**
  - 5a: D0 frozen, possibly high. D1 frozen → D3 rises after K1's drop-out (0.07–0.31 s). D6 (SCK) keeps clocking; the answers on D7 stop or go stale. D2 stops within ≤ 2 × 25 ms → D4 rises about 7.5 ms after that. *(Revised after review 3:)* **the 120 ms bound is met by K2 (about 68 ms with release), not by K1.** D5 falls at the first relay opening.
  - 5c/5e: D1 keeps toggling (K1 held or bypassed). D2 stops within ≤ 25 ms of the first wrong answer (the first bad answer trips it). D4 rises, D5 falls. **This capture is the proof of #144.**
  - 5d: D2 dead → D4 rises.
- **Mock result.** PASS (variant A). Variant B adds 5f.

### Test 6. The bus guard refuses any frame that is not byte-exact

- **Setup.** Trust root armed (D3 = GP12 high, as the precondition). The supervisor sends candidate frames to the guard over its USB CDC. The guard forwards passes to GP0 → iiunius → pair → Waveshare → Pi.
- **Steps.** Send the approved frame, then: a frame not allow-listed, a write with no lease, the approved frame with one byte changed, a bad CRC, a broadcast (unit 0).
- **Expected.**
  - Approved frame: D0 shows the bytes, decoded as UART at the configured baud (Modbus RTU framing 8N1 or 8E1). The Waveshare receives the identical bytes (compare SHA-256 against the lease digest's frame).
  - Every refused frame: no edge on D0 and nothing received by the Waveshare.
  - Optional D2 shows line A, with the analyzer ground bridged onto GND-iso; record that in the capture metadata.
- **Mock result.** PASS (termination/bias to be checked, step 8).

## Step 11. Flags summary (where the mock fails or needs a part)

1. Barrel jack has nowhere to land (M3).
2. No fuse ≤5 A (M4).
3. No 5 V converter (M1).
4. No crowbar against a shorted buck (M6).
5. No adjustable source for brownout or the MAX813L test (M2).
6. No PC817s (M7), 2N3904s (M8), Schottkys (M9), capacitors (M10), 1N4007 (M11), 2N3906 (M30), genuine fuses (M28), DIP sockets (M12).
7. No 3.3 V LDO for the watchdog (M5).
8. No micro-USB cable for the Pico (M13); USB-B cable unconfirmed (M14).
9. No plate or panel (M16, printable).
10. Firmware pin map conflicts: GP16 = WS2812, GP17 = pad, no I2C/Q&A/MID pins (F1–F3, #288).
11. *(Revised after independent review B1.)* Pump values re-sized for the real 50 Hz toggle; the original values never closed K1 (step 5, HW-031).
12. Test 4 as written is self-contradictory and needs a Mac-hosted analyzer (F5, #289); 4c added.
17. *(Added after review 3, #297.)* Crowbar re-done with a PNP driver and F3 (R3-B1, B2); K2 budget and 470 nF / 47 k pump 2 (R3-B3); Q64 and J_B2 deleted, hardware latch and panel RESET (R3-B4); e-stop output optocoupler on GP17, option (b) K0 as a must (R3-B5); tests 1, 2, 4, 5 on the Mac (R3-B6); self-test expectations and shrouded XH headers (R3-B7).
16. *(Added after independent review, #296.)* SCR crowbar instead of the zener (B2); variant B rewired (B3); SWD probe on the Mac only (B4); two relay modules and keyed connectors (B5); 4.7 kΩ E9 pull-downs.
13. "Firmware cannot change without the jumper" is bypassed by BOOT+USB and SWD (F6, #290).
14. ROADMAP P1.1 items (NVMe boot, hub port) have no parts (F8, #291).
15. Should-haves: dummy loads (M15), ferrules (M17), jumper shunts (M18), female headers (M19), K0 seal-in kit (M27, open owner decision).

## Verdict

```
COMPATIBILITY: PASS WITH CHANGES
WIRING MOCK: PASS WITH CHANGES
```

Changes required for PASS (both):

1. Add M1 (5 V buck), M2 (adjustable source), M3 (barrel→terminal ×2), M4 (1 A/2 A fuses), M5 (3.3 V LDO), M6 (BT151-500R SCR, TL431A and 2N3906 crowbar), M28 (genuine 1 A fuses for F1, F2, F3), M7 (PC817 ×5), M8 (2N3904), M9 (Schottky ×4), M10 (pump and decoupling capacitors), M11 (1N4007), M12 (DIP-8 sockets), M13 (micro-USB cable), M16 (plate and panel), and M14 if the Waveshare has no cable.
2. Never drive a relay input from a GPIO. Both relay channels go through charge-pump + 2N3904 stages (steps 5 and 6).
3. External pull-downs: 4.7 kΩ on GP12/GP13 and on GP7/GP28/GP29 (RP2350-E9, 8.2 kΩ or less; HW-032).
4. Firmware pin map per step 3.1: `CHARGE_PUMP` → GP14, `RAIL_B_PORTS = 1`, plus I2C1, SPI1 Q&A, MID_SENSE, lease-LED pins. Add stuck-output checks via MID_SENSE and INA228 VBUS, and a fault on INA228 I2C failure.
5. ATtiny at 3.3 V on its own LDO, synchronous Q&A link (ATtiny clock master); fuses written, read back and recorded before lock.
6. Trust root never on USB; SWD lead keyed, removable and out during tests; the Pico probe is on the Mac only.
7. Analyzer ground only at TB-GND; host it on the Mac for test 4.
8. Rewrite exit test 4 as 4a (bypass fixture) + 4b. Scope the firmware-jumper claim to the link update path, or burn OTP.
9. Variant B: U4 socket and J_B fitted, inert until a MAX813L passes the acceptance test (compatibility.md §5.3).
10. Owner decision needed: drop NVMe boot and the hub port from the P1.1 check-in, or add those parts.

Recommended, not required: M15, M17–M19, M24, M27 (K0; an open owner decision, depends on R3-B5 option (a) or (b)). M20 (second relay module) is now required.
