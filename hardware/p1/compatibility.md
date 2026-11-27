# Sawbones Labs P1: compatibility and safety review

Hardware safety review of the P1 parts list ([`BOM.md`](BOM.md), 29 lines) for "Prove the Interlock on the bench". Written 26 Nov 2026. Nothing is bought. The review is adversarial: it looks for every way the bench can fail unsafe.

Companion files: [`wiring-mock.md`](wiring-mock.md) (mock build), [`method.md`](method.md) (the reusable procedure), [`BOM.md`](BOM.md), and the machine-readable design [`data/parts.toml`](data/parts.toml), [`data/netlist.toml`](data/netlist.toml), [`data/safety_paths.toml`](data/safety_paths.toml). [`ci/check_hardware.py`](../../ci/check_hardware.py) runs the mechanical checks of the method against that design on every pull request ([README](README.md)). Findings that need a change outside the review carry an issue number: #288 pin map, #289 exit test 4, #290 firmware jumper, #291 ROADMAP P1.1, #292 ATtiny fuse bytes, #293 MAX813L acceptance, #294 the findings now enforced by CI.

Legend: **PASS** works as listed. **WARN** works with a condition, or relies on an unverified fact. **FAIL** unsafe or unworkable as listed. "UNVERIFIED" means no source settled it and the part in hand must be measured.

## Verdict up front

- **COMPATIBILITY: PASS WITH CHANGES.** No part has to be thrown away. But nine items needed to build a safe bench are missing (5 V supply, low-value fuses, sense optocouplers, transistor/diode/capacitor stage parts, 3.3 V LDO for the watchdog, 5 V over-voltage clamp, barrel-to-terminal adapters, a micro-USB cable, an adjustable source for the brownout test). Three firmware placeholders also need moving (details in section 4).
- The three findings that would have made the bench **fail unsafe** if built the obvious way (each is now a CI check, #294):
  1. **Driving the SunFounder relay input straight from an RP2350 pin is unsafe.** The module is active-low. The firmware's "off" level is low, so a direct connection turns the relay ON when the firmware means off. An unpowered RP2350 also leaks current backwards through its pin, which can energise the relay. Rule: a relay channel is driven only by the charge-pump transistor stage, never by a pin.
  2. **A failed 12 V→5 V converter puts 12 V on the logic rail.** The usual way a buck converter fails is with its high-side switch shorted. That 12 V reaches the RP2350, the ATtiny and the relay coils at the same moment, and all their outputs become undefined. This is a common-cause fault. Fix: a 1 A fuse ahead of the converter plus a 5.6 V zener crowbar on the 5 V rail, so that fault blows the fuse.
  3. **Clipping the logic-analyzer ground to a lamp terminal turns the lamp on.** The AOD4184 switches the negative side, so the lamp's negative terminal is not ground. The analyzer's ground is the system ground, so the clip bypasses the MOSFET. Rule: analyzer ground goes only to the star ground point, and probes go only on logic nets.
- **Watchdog supervisor:** variant A (ATtiny85 BOD + its own watchdog + charge-pump hold) is enough for P1 on its own, **provided** the ATtiny fuses are set and read back (BOD is disabled at the factory). The MAX813L (owner decision: stays on the order) is fitted as variant B, an extra layer that can only cut. It can never close the relay, so it cannot make the bench less safe. It has real limits: a 5 V-only threshold, a 1.0–2.25 s timeout, VIH(WDI) 3.5 V, and a floating WDI disables its watchdog. Section 5.

## 1. Sources

Every spec below cites one of these. Repo files are in this repository (main, read 26 Nov); paths are relative to the repository root.

| Key | Source |
|---|---|
| S-ZERO | Waveshare RP2350-Zero wiki: https://www.waveshare.com/wiki/RP2350-Zero (20 GPIO on edge pins, rest on solder pads; BOOT/RESET buttons; USB-C) |
| S-ZERO-SCH | Waveshare RP2350-Zero schematic, https://files.waveshare.com/wiki/RP2350-Zero/RP2350_Zero.pdf (VBUS wired straight to the 5 V pin and the LDO, no diode; read in repo `research/round-5-parts-cross-check/02-trust-root.md` §5) |
| S-ZERO-DTS | Zephyr board files for rp2350_zero: https://github.com/zephyrproject-rtos/zephyr/blob/main/boards/waveshare/rp2350_zero/rp2350_zero.dtsi (WS2812 on GPIO16; header map GP0–22, 26–29; UART0 GP0/GP1; I2C0 GP4/5, I2C1 GP6/7) |
| S-ZERO-LDO | Core Electronics listing: https://core-electronics.com.au/waveshare-rp2350-zero-mini.html (ME6217C33M5G LDO, 800 mA max) |
| S-RP-A4 | Raspberry Pi, "RP2350 A4": https://www.raspberrypi.com/news/rp2350-a4/ (A4 fixes E9 high-side leakage; officially 5 V tolerant only while IOVDD is powered; A2 withdrawn, rollout uneven) |
| S-E9 | Hackaday on E9: https://hackaday.com/2024/09/20/raspberry-pi-rp2350-e9-erratum-redefined-as-input-mode-leakage-current |
| S-RP-RST | EEVblog thread, "RP2350: GPIOs are pulled-down upon reset": https://www.eevblog.com/forum/microcontrollers/rp2350-gpios-are-pulled-down-upon-reset/ (pull-down on at reset, about 35–189 kΩ depending on IOVDD). Reading only, not the datasheet table: UNVERIFIED in detail |
| S-FW | Repo [`firmware/trust-root/src/rp2350.rs`](../../firmware/trust-root/src/rp2350.rs) (pin placeholders GP2–GP17; outputs low then enabled then ISO released; inputs pulled to the safe reading) |
| S-T85 | ATtiny25/45/85 datasheet (Atmel 2586): https://ece-classes.usc.edu/ee459/library/documents/Atmel-2586-AVR-8-bit-Microcontroller-ATtiny25-ATtiny45-ATtiny85_Datasheet.pdf (BODLEVEL 111 disabled by default, 110=1.7–2.0 V, 101=2.5–2.9 V, 100=4.1–4.5 V; RC factory ±10 %, user ±1 %; VIH 0.6·VCC; VOH ≥2.5 V at −5 mA, 3 V; port pins tri-stated in reset; WDTON = always on; WDT from 128 kHz osc, 16 ms–8 s; ships CKDIV8 = 1 MHz; lock mode 2/3 locks fuses; 10 MHz at 2.7–5.5 V) |
| S-MAX | MAX705–MAX813L datasheet Rev 11: https://www.analog.com/media/en/technical-documentation/data-sheets/MAX705-MAX813L.pdf and https://www.analog.com/en/products/max813l.html (VCC 1.1–5.5 V; reset threshold 4.50/4.65/4.75 V; reset pulse 140/200/280 ms; WDT 1.00/1.60/2.25 s; WDI VIH 3.5 V, VIL 0.8 V; PFI 1.20/1.25/1.30 V; RESET active-high; WDO active-low; MR active-low with 100–600 µA pull-up; a floating WDI disables the watchdog; ICC ≤350 µA; DIP-8 pins 1 MR, 2 VCC, 3 GND, 4 PFI, 5 PFO, 6 WDI, 7 RESET, 8 WDO) |
| S-INA | TI INA228: https://www.ti.com/product/INA228 (VS 2.7–5.5 V; CM −0.3 to 85 V; VBUS 0–85 V; ±163.84 / ±40.96 mV; offset ±1 µV; 16 addresses) |
| S-AD584 | ADI AD584: https://www.analog.com/en/products/ad584.html (10/7.5/5/2.5 V, 10 mA). Headroom (10 V needs ≥12.5 V in) from repo cross-reference |
| S-RLY | Generic 2-ch opto relay module: https://components101.com/switches/5v-dual-channel-relay-module-pinout-features-applications-working-datasheet and https://protosupplies.com/product/relay-module-5v-x-2-relay-w-optoisolation/ (active-low; input LED and opto LED in series from VCC; about 70 mA per coil; JD-VCC jumper). Reference design, not the SunFounder board itself: UNVERIFIED for this part |
| S-D4184 | D4184 module reference: https://protosupplies.com/product/d4184-mosfet-control-module/ (1 kΩ into the PC817; 2.5–20 V input; gate from load supply through a 4.7k/4.7k divider, about 6 V at 12 V; 4.7 kΩ gate pull-down; low-side; no flyback diode; logic and load grounds not joined; load supply 6–36 V) |
| S-WS485 | Waveshare USB TO RS232/485/TTL wiki: https://waveshare.com/wiki/USB_TO_RS232/485/TTL (FT232RL; on-board 120 Ω RS485 terminator; VCC out 3.3/5 V at 50 mA max; one mode at a time) |
| S-SHG01A | SH-G01A listing copy: https://www.desertcart.co.za/products/327395033-dsd-tech-sh-g01a-usb-isolator-with-adum3160-chip-12m (full and low speed only, no 480 Mbps; 200 mA downstream) |
| S-LA | sigrok, noname Saleae Logic clone: https://sigrok.org/wiki/Noname_Saleae_Logic_clone (CY7C68013A, 74LVC245A buffer, −0.5 to 5.25 V absolute max, VIH 2.0 V, VIL 0.8 V, fx2lafw supported) |
| S-PI5USB | Raspberry Pi white paper RP-009856, "USB power delivery on Raspberry Pi 5": https://pip-assets.raspberrypi.com/categories/685-app-notes-guides-whitepapers/documents/RP-009856-WP-1-USB%20Power%20delivery%20on%20Raspberry%20Pi%205.pdf (600 mA total to USB unless 5 V 5 A is negotiated, then 1.6 A) |
| S-PINEPWR | PINE64 PinePower Desktop: https://pine64.org/documentation/PinePower/Versions/PinePower_Desktop/ (USB-C PD 5/9/12/15/20 V, USB-A; **no user-adjustable output, no barrel output**) |
| S-XB | Schneider XB4BG21 (the original the Shopcorp XB2-BG21 copies): https://www.se.com/us/en/product/XB4BG21/ (2 positions, stay put, 1 NO). Clone contact count and key-removal position: UNVERIFIED |
| S-REPO | Repo: [`docs/ROADMAP.md`](../../docs/ROADMAP.md) (P1, P1.1–P1.6), issue #144, [`docs/THREAT-MODEL.md`](../../docs/THREAT-MODEL.md) §5, [`docs/bench-core-parts.md`](../../docs/bench-core-parts.md), [`docs/bench-core-cross-reference.md`](../../docs/bench-core-cross-reference.md), [`research/round-5-parts-cross-check/02-trust-root.md`](../../research/round-5-parts-cross-check/02-trust-root.md), [`crates/sawbones-interlock/src/lib.rs`](../../crates/sawbones-interlock/src/lib.rs), [`crates/sawbones-busguard/src/lib.rs`](../../crates/sawbones-busguard/src/lib.rs) |
| S-WDR | Owner research note on the watchdog cut (ATtiny85 Q&A + charge pump + MAX813L proposal); not in this repo |

## 2. Interface matrix

One row per electrical interface. "Domain" names are the ones used in `data/netlist.toml`.

### 2.1 Power

| # | Interface | Check | Result | Reason / source |
|---|---|---|---|---|
| P1 | Facmogu 12 V 3 A adapter → load + logic wiring | Connector | **FAIL as listed** | 5.5×2.5 / 2.1 mm plug and nothing to land it on. Add a female barrel to screw-terminal adapter (missing part M3). S-REPO cross-ref |
| P2 | 12 V → load branch fuse | Fuse value vs wire | **FAIL as listed** | Nilight kit is 5–30 A only (BOM.md #14); the 40 fuses with the holders are unspecified. 22 AWG silicone is listed at 3 A (#7), so a 5 A fuse does not protect it. Need 1 A and 2 A ATC/ATO (M4). Repo set 2 A rail / 1 A trust root; for a pilot-lamp or ≤0.3 A load use **1 A** on both |
| P3 | Inline holder (12 AWG leads) ↔ 22 AWG | Joint | WARN | Step-down splice: solder plus heat shrink (shrink comes with #7), or a 12-10→22-18 step-down butt splice. Not a screw-terminal squeeze of mismatched gauges |
| P4 | 12 V → 5 V for RP2350, relay coils, ATtiny LDO, MAX813L | Source exists | **FAIL as listed** | No converter on the P1 list (the XL4015s were dropped). Must-have M1: 12→5 V buck, ≥1 A, set 5.05–5.10 V |
| P5 | 5 V rail over-voltage (buck fails shorted) | Fail-safe | **FAIL as listed** | ME6217C33 and the ATtiny LDO see 12 V; every MCU output undefined at once. Fix: F2 1 A ahead of the buck + 1N5339B 5.6 V 5 W zener on the 5 V rail (crowbar, blows F2). M6 |
| P6 | 5 V budget | ≤80 % of source | PASS (with M1) | Relay 2×70 mA + LEDs ≈ 150 mA (S-RLY), RP2350-Zero + 3V3 loads ≈ 60 mA, ATtiny LDO ≈ 5 mA, MAX813L ≤0.35 mA (S-MAX) → ≈ 215 mA, i.e. 22 % of a 1 A buck |
| P7 | RP2350-Zero 5 V pin + USB-C at the same time | Back-feed | **FAIL if done** | No diode between VBUS and the 5 V pin (S-ZERO-SCH). Rule: the trust root takes power only from the bench 5 V. Its USB-C stays unplugged; it is flashed over SWD. The PULSE link uses the Pi header UART, not USB |
| P8 | RP2350-Zero 3V3 out → INA228, OLED, PC817 pull-ups, lease-LED base | Budget | PASS | ME6217C33 800 mA (S-ZERO-LDO); load about 40 mA |
| P9 | ATtiny85 supply independence | Own supply | WARN → PASS with M5 | Feeding the ATtiny from the RP2350's 3V3 makes one LDO a common cause. It would fail safe on undervoltage, but not on overvoltage. Give it its own 3.3 V LDO from 5 V (M5: MCP1700-3302E TO-92 or AMS1117-3.3 module) |
| P10 | AD584 module input | Headroom | WARN | 12 V gives the 2.5/5/7.5 V outputs. 10 V needs ≥12.5 V (S-REPO cross-ref). Nice-to-have: USB-C PD trigger at 15 V from the PinePower (S-PINEPWR) |
| P11 | Brownout ramp source | Exists | **FAIL as listed** | Exit test 1 needs a slow and a fast sag of the trust-root supply. The PinePower has fixed PD steps only (S-PINEPWR). The Pi kit supply is not adjustable. Must-have M2: an adjustable buck (e.g. LM2596-ADJ module with a multi-turn pot, plus a voltmeter) or a real bench supply. The ROADMAP's "a bench supply" line is not on the list |
| P12 | Pi 5 supply (third-party 27 W kit) | 5 A negotiation | WARN | Without 5 V 5 A, USB gets 600 mA total (S-PI5USB). P1 USB load is about 300 mA (section 2.5), so this passes either way |
| P13 | Relay module coil supply (VCC/JD-VCC) | Topology | PASS | With the transistor stage, VCC = JD-VCC = 5 V and the jumper fitted is fine (S-REPO 02-trust-root §3) |
| P14 | AOD4184 module gate drive | Supply | PASS (lamp) / WARN | Gate is about ½ of the load supply, so about 6 V at 12 V (S-D4184). AOD4184A is fully on at 4.5 V gate (datasheet https://www.aosmd.com/res/datasheets/AOD4184A.pdf, not re-read: UNVERIFIED). Fine for ≤1 A |

### 2.2 Logic levels and drive

| # | Driver → receiver | Rule | Result | Reason / source |
|---|---|---|---|---|
| L1 | RP2350 GPIO (3.3 V) → SunFounder IN (5 V-fed, active-low) **direct** | Default state de-energises the load | **FAIL** | A low or pulled-down pin turns the relay ON. The firmware drives every output low when off (S-FW `OUTPUT_MASK`). An unpowered RP2350 (still on the 5 V rail) back-feeds through its ESD diode: (5 − 1.2 − ~1.8 − 0.6)/1 kΩ ≈ 1.4 mA through the opto, which can pull the relay in. A 3.3 V high leaves 1.7 V across two LEDs in series. That is probably off, but it is not proven (S-RLY, UNVERIFIED for this board). **Banned.** |
| L2 | Charge pump → 2N3904 open collector → relay IN | Same | PASS | The transistor pulls IN low only while the pin toggles. Stuck high, stuck low, unpowered, reset or wire off all leave it off. A shorted transistor is a single fault covered by the second cut (section 3) |
| L3 | RP2350 GPIO → AOD4184 PWM input (active-high, 1 kΩ + PC817) | Drive + default | PASS / WARN | 3.3 V gives (3.3−1.2)/1 k ≈ 2.1 mA, within the RP2350's 4 mA default drive. Unpowered, reset or pulled down gives no LED current, so off (S-D4184). WARN: PC817 CTR at 2 mA with a 4.7k/4.7k gate network is marginal; measure gate ≥4.5 V at a 3.3 V input. WARN (A2 stepping E9): a pad can float near 2.1 V, giving about 0.9 mA of LED current, so the MOSFET may be partly on. Fit an external 4.7 kΩ pull-down on GP12/GP13 (S-E9, S-RP-A4) |
| L4 | RP2350 ↔ ATtiny85 Q&A link (both 3.3 V) | VOH ≥ VIH | PASS | ATtiny VIH 0.6·3.3 = 1.98 V; ATtiny VOH ≥2.5 V at 5 mA (S-T85); RP2350 is 3.3 V CMOS. Add 1 kΩ series resistors on each line, so either chip can be unpowered |
| L5 | ATtiny85 at **5 V** ↔ RP2350 | Same | FAIL (do not do) | RP2350 3.3 V high vs VIH 0.6·5.0 = 3.0 V leaves 0.3 V margin. A 5 V output into an unpowered RP2350 pad is outside the "IOVDD powered" condition (S-RP-A4). Keep the ATtiny at 3.3 V |
| L6 | ATtiny PB4 (3.3 V) → MAX813L WDI (variant B) | VOH ≥ VIH | **FAIL direct** | WDI VIH min 3.5 V (S-MAX). Needs a 2N3904 inverter with a 10 kΩ pull-up to 5 V (inverting does not matter; edges are edges) |
| L7 | MAX813L RESET (active-high, 5 V push-pull) → ATtiny RESET (active-low) | Polarity | **FAIL direct** | Wrong polarity, and it would also block ISP. Use RESET only to drive the clamp transistor on the pump-2 hold node (section 5) |
| L8 | Pi 5 GPIO14/15 UART0 (3.3 V) ↔ RP2350 UART0 GP1/GP0 | Levels / Pi-off | PASS / WARN | Both 3.3 V. WARN: exit test 4 powers the Pi off with the RP2350 live, and an RP2350 TX into an unpowered Pi pin back-powers it. Fit 1 kΩ series resistors on both lines and a pull-up on the RP2350 RX (S-RP-A4 on unpowered pads) |
| L9 | 12 V nodes → RP2350 sense inputs | Never >3.3 V | PASS with M7 | Only through PC817s (4.7 kΩ on the 12 V side gives 2.3 mA). No dividers into the trust root (S-REPO 02-trust-root §2) |
| L10 | RP2350 GPIO → 12 V LED in the CB16AY button | Level | PASS with 2N3904 | The LED is a 12 V type (BOM.md #20). NPN low-side switch, 10 kΩ base resistor. LED current UNVERIFIED |
| L11 | RP2350 GP16 | Free pin? | **WARN** | GP16 drives the on-board WS2812 (S-ZERO-DTS). The firmware placeholder puts `RAIL_B_ENABLE[3]` on GP16. Remap (section 4, #288) |
| L12 | RP2350 GP17 | Edge pin? | **WARN** | GP17 is a solder pad, not an edge pin (S-ZERO, S-ZERO-DTS). The firmware's `CHARGE_PUMP` = GP17 needs a wire soldered to a pad. Remap to GP14 (#288) |
| L13 | RP2350 input pull-downs (rail sense) | E9 | WARN | On A2 parts a pulled-down input can latch about 2.1 V (S-E9). A4 fixes it, but rollout is uneven and the stepping is printed on the package (S-RP-A4). Fit external 10 kΩ pull-downs on every "pulled low = safe" input regardless. Record the stepping as a fact |

### 2.3 I2C

| # | Check | Result | Reason |
|---|---|---|---|
| I1 | INA228 + SSD1306 share I2C1 (GP26 SDA / GP27 SCL) | PASS | INA228 is 0x40–0x4F (16 addresses, S-INA); module default UNVERIFIED (expect 0x40 with A0 = A1 = GND). SSD1306 is 0x3C or 0x3D (Hosyond default UNVERIFIED, expect 0x3C). No overlap |
| I2 | Pull-ups and VCC | WARN | Power both at 3.3 V so the pull-ups go to 3.3 V. Two module pull-up sets in parallel (about 2.4–5 kΩ) are fine at 400 kHz. Measure on arrival |
| I3 | INA228 failure | WARN (firmware gap) | The firmware does not read the INA228 yet. A missing or NACKing INA228 must latch a fault, not read as "0 V = rail dead" |
| I4 | INA228 resolution at the pilot-lamp load | WARN | 20 mA × 2 mΩ (R002) = 40 µV. That is 512 LSB at ±40.96 mV range against a ±1 µV offset (S-INA): measurable but small. Use a ≥0.25 A dummy load (M15) for meaningful current evidence |

### 2.4 UART, Q&A link, RS485

| # | Interface | Result | Reason |
|---|---|---|---|
| U1 | PULSE: Pi `/dev/ttyAMA0` (GPIO14/15, header pins 8/10) ↔ RP2350 UART0 | PASS | 3.3 V both ends. Device name on Pi 5 UNVERIFIED (enable with `dtparam=uart0=on`; the Pi 5 debug UART is the separate 3-pin JST, not GPIO14/15) |
| U2 | Q&A: ATtiny USI three-wire **master** (PB2 USCK, PB1 DO, PB0 DI) ↔ RP2350 SPI1 **slave** (GP10 SCK, GP8 RX, GP11 TX, GP9 CSn tied low) | PASS (design) / WARN | A synchronous link because the ATtiny RC oscillator is only ±10 % from the factory (S-T85), outside a UART's ±2–3 %. With the ATtiny as clock master, its frequency error does not matter. WARN: RP2350 SPI slave with CSn held low needs CPHA = 1; verify on the bench. Fallback: software UART with OSCCAL calibrated to ±1 % (S-T85). Any link failure **trips the cut**, so a link fault costs uptime, never safety |
| U3 | Bus-guard loopback: RP2350 #2 UART0 → iiunius TTL-RS485 (C) → A/B pair → Waveshare USB-RS485 → Pi | PASS / WARN | iiunius TTL side accepts 3.3–5 V (BOM.md #9). Auto-direction, no DE pin (S-REPO cross-ref): every TX byte goes on the wire, which makes GP0 the wire's content. WARN: Waveshare has an on-board 120 Ω terminator (S-WS485); the iiunius terminator and bias are UNVERIFIED. With two 120 Ω on a 0.5 m pair, check idle A−B ≥ +200 mV; if not, remove one terminator. Join the two isolated RS485 grounds (Waveshare GND-iso ↔ iiunius GND-iso) to give the pair a common reference |
| U4 | Waveshare adapter USB connector | WARN | Listed as USB-B (BOM.md #10). Whether a cable is in the box: UNVERIFIED. Add a USB-A→USB-B cable if not (M14) |
| U5 | Bus guard "transmitter powered only from the armed rail" | WARN (scope) | The iiunius is powered from RP2350 #2, not from rail A. For P1 that is acceptable because the threat-model test is against the guard firmware (S-REPO THREAT-MODEL §5). Note it as a P2 item |

### 2.5 USB topology (Pi 5: 2 × USB 3 blue, 2 × USB 2 black)

| Port | Device | Speed | Current (est.) | Result |
|---|---|---|---|---|
| USB3 upper (blue) | HiLetgo logic analyzer | High speed 480 M required for 24 MHz | about 100 mA | PASS. **Never through the SH-G01A** (full speed only, S-SHG01A) |
| USB3 lower (blue) | RP2350 #2 bus guard (USB-C cable, #3) | Full speed | about 30 mA + iiunius about 60 mA | PASS |
| USB2 upper | Pico debug probe (micro-USB, M13) | Full speed | about 25 mA | PASS. Its ground is Pi ground |
| USB2 lower | Waveshare USB-RS485 | Full speed | about 50–80 mA | PASS |
| — | Trust root RP2350 #1 | not connected | — | PASS (P7: no USB on the trust root) |
| — | SH-G01A | not used in P1 | — | WARN: no P1 role. Anything you isolate with it gets re-grounded by the analyzer or SWD probe in every test. Keep it for P2 module 1 |
| Total | | | ≈ 300 mA | PASS even at the 600 mA limit (S-PI5USB) |

Four ports, four devices, no hub. The Phidgets hub was dropped from P1, so the P1.1 check-in item "power-cycles one hub port" cannot be shown, and "boots from NVMe" cannot be shown either: the NVMe board and drive are not on the P1 list. **Scope conflict with ROADMAP P1.1: flag to the owner (#291; also #142 and #159).**

### 2.6 Grounds and isolation domains

| Domain | Members | Ground |
|---|---|---|
| GND_SYS | 12 V adapter negative, load returns, 5 V buck, trust root, watchdog board, relay module, INA228, OLED, MOSFET module logic and load grounds, Pi (via header pin 6 and USB shields), Pico, logic analyzer, RP2350 #2 | One star point: terminal TB-GND on the trust-root plate |
| ISO_WS485 | Waveshare RS485/TTL side | Isolated from the Pi (S-WS485) |
| ISO_IU485 | iiunius bus side | Isolated from RP2350 #2 (BOM.md #9) |
| ISO_WS485 + ISO_IU485 | Joined by the GND-iso wire of the loopback | One floating bus ground. In `data/` both isolators name it ISO_RS485 |

| # | Check | Result | Reason |
|---|---|---|---|
| G1 | Only crossed through an isolator | PASS | The RS485 domains are crossed only by the two isolated modules. The optional analyzer capture of line A (test 6) deliberately bridges ISO→GND_SYS and must be written down in the capture record |
| G2 | MOSFET module logic GND ≠ load GND on the board (S-D4184) | WARN | Both go to GND_SYS. The opto isolation is not used, by design. A missing logic-GND wire leaves the input LED without a return, so the switch fails off |
| G3 | Analyzer ground placement | **FAIL if misplaced** | Lamp-negative is a switched node. An analyzer ground clip there shorts the MOSFET and lights the lamp. Ground lead to TB-GND only |
| G4 | Ground loops | WARN | Pi GND reaches GND_SYS through header pin 6, the analyzer ground, the Pico SWD ground and the RP2350 #2 USB. These are parallel paths between the same node, not a loop between different potentials. Harmless at DC and 12 V; noise only |
| G5 | Test 4 analyzer host (#289) | **FAIL as written** | With the Pi off, a Pi-hosted analyzer records nothing. Host the analyzer on the MacBook for test 4 (sigrok/PulseView on macOS: UNVERIFIED on Apple Silicon, check before the test) |

### 2.7 Logic analyzer input limits

| # | Check | Result |
|---|---|---|
| A1 | All probed nets ≤5.25 V (S-LA) | PASS with rules: 3.3 V nets direct; relay IN nets at 5 V are direct only if the buck is set ≤5.10 V, otherwise use a 10 kΩ series resistor; never 12 V, never RS232 |
| A2 | Thresholds VIH 2.0 V / VIL 0.8 V (S-LA) | PASS for 3.3 V logic. The PC817 outputs swing full 0–3.3 V with 10 kΩ |
| A3 | Input buffer present | WARN: the sigrok page shows a 74LVC245A; some clones omit it. Check the board in hand |

### 2.8 Panel parts

| # | Part | Use | Result |
|---|---|---|---|
| X1 | HB2-ES544 e-stop, 2 NC, latching (#23) | NC1 in the 12 V load path; NC2 is the sense to GP2 | PASS. DC breaking rating UNVERIFIED (AC 660 V 10 A printed); at 12 V ≤1 A it is far inside any rating. Positive-opening action on a clone: UNVERIFIED |
| X2 | DaierTek KN3C-101 arm toggle, SPST 2-pin, 20 A 12 VDC (#21) | In series in the load path; sensed after it by a PC817 | PASS. With no second contact, the sense is "e-stop closed AND arm closed AND 12 V present". A released e-stop makes arm-sense rise; the firmware's latched e-stop fault plus the "arm seen off then on" rule stop that from re-arming |
| X3 | Shopcorp XB2-BG21 key, 2-position maintained (#22) | In series on the rail-B branch; sensed after it | PASS / WARN: 1 NO expected (S-XB); clone UNVERIFIED. Its sense only reads while rail A is live. Nice-to-have: an extra NO contact block (ZB2-BE101 class) for a dry sense contact |
| X4 | CB16AY-11D 12 V LED button 1NO1NC (#20) | Lease button: NO to GP5. LED via 2N3904 | PASS. Nice-to-have: wire the NC to a spare GPIO for a two-channel button (NO and NC must disagree) |
| X5 | Gebildet AD16-22D/S 12–24 V lamps, 20 mA (#16) | Status lamps wired straight across rails, plus the load | PASS. Polarity-free (AC/DC) per listing, UNVERIFIED |

## 3. Safety analysis of the rail chain

### 3.1 The topology this review concludes is correct

```
12V adapter ─ F0? none (3 A adapter) ─┬─ F2 1A ─ 5V buck ─ D_Z 5.6V ─ 5V rail (logic; BEFORE the e-stop, so the stop never kills the controller)
                                      └─ F1 1A ─ ESTOP NC1 ─ ARM toggle ─ K1 (relay CH1, NO; held by RP2350 charge pump GP14)
                                               ─ K2 (relay CH2, NO; held by ATtiny charge pump PB3 [+ MAX813L clamp in variant B])
                                               ─ INA228 shunt (IN+ → IN−) ─ RAIL_BUS
RAIL_BUS ─ Lamp A ─ AOD4184 #1 (rail A enable, GP12, low side) ─ GND
RAIL_BUS ─ KEY NO ─ RAIL_B_HI ─ Lamp B ─ AOD4184 #2 (rail B port 0 enable, GP13, low side) ─ GND
Indicator lamps: L_PWR across F1 output (before the e-stop), L_BUS across RAIL_BUS.
Sense (PC817, all to the RP2350): ARM_SENSE at ARM output; MID_SENSE between K1 and K2; RAIL_A_SENSE across Lamp A; RAIL_B_SENSE across Lamp B;
KEY_SENSE at RAIL_B_HI; ESTOP_SENSE = NC2 to GND. INA228 VBUS = RAIL_BUS.
```

Why this order:

- **E-stop NC1 sits directly in the load power path, first after the fuse.** It works with the Pi, the RP2350 and the ATtiny all dead. NC contacts fail safe for a wire off.
- **Arm and key are in hardware series, not only sensed.** Software cannot energise rail A without a hand on the arm toggle, or rail B without the key.
- **Three electronically controlled series elements on every lamp path, from two different brains:** K1 (RP2350 dead-man pump), K2 (ATtiny Q&A dead-man pump), and the AOD4184 (RP2350 level). The two RP2350 elements are different technologies: a toggling AC hold and a DC level. The ATtiny is a different vendor, core and supply.
- **Both relays use COM–NO,** so a de-energised coil is an open rail.
- **No relay channel is ever driven straight from a GPIO** (L1).
- **Feedback covers each element:** MID_SENSE sees K1; INA228 VBUS / L_BUS sees K1·K2; RAIL_x_SENSE (PC817 across the lamp) sees the MOSFET. A firmware self-test at arm time is needed to prove K2 opens (below). It is not written yet.

### 3.2 Single-fault table, variant A (ATtiny85 BOD + WDT + charge pump only)

The table assumes the bench is armed (K1, K2 closed, rail A commanded on) unless it says otherwise. "Lamp" means both load lamps unless one is named.

| # | Single fault | What happens | Lamp | Detected by | Verdict |
|---|---|---|---|---|---|
| A1 | RP2350 frozen (hang, or halted by SWD) | GP12 and GP13 freeze at their last level (they may stay HIGH, MOSFET on). GP14 pump stops, so K1 drops after the hold-up τ (about 10–20 ms at 1 kHz values). No answers reach the ATtiny, so PB3 stops and K2 drops within about 2 Q&A periods | OFF | MID_SENSE, VBUS (if alive), ATtiny trip LED | PASS |
| A2 | RP2350 reset (button, watchdog, reset loop) | Pads go to the reset state: pull-down on, isolated (S-RP-RST, S-FW). GP12/13 are pulled low (plus the external 4.7 kΩ), so the MOSFET turns off. The pump stops, so K1 drops. The ATtiny sees no answer, so K2 drops. After boot the firmware writes outputs low before enabling them, then latches FaultStateUnknown (S-FW, interlock crate) | OFF | Fault record | PASS |
| A3 | RP2350 brownout | The chip's internal brownout reset applies, as in A2. If the 3V3 sags below about 1.6 V, the AOD4184 input LED cannot conduct | OFF | — | PASS |
| A4 | RP2350 unpowered (LDO dead, 5 V present) | GP12 at 0 V: no opto current. Pump dead. Back-feed path: only if a relay IN were wired direct, which is banned (L1) | OFF | ATtiny trips | PASS |
| A5 | GPIO state at power-up, before firmware | Pull-down, ISO set; the firmware sets outputs low before clearing ISO (S-FW). External pull-downs cover E9 | OFF | Test 1 capture | PASS |
| A6 | Pi off / supervisor hung | No PULSE reply, so after ≤2 × HEARTBEAT_PERIOD the firmware drops GP12/13 and stops GP14. The ATtiny is unaffected, so K2 stays | OFF | Fault latched | PASS |
| A7 | ATtiny frozen | PB3 stuck high or low, no AC, so K2 drops | OFF | MID_SENSE high + VBUS 0 | PASS |
| A8 | ATtiny reset or BOD (fuse 101 = 2.5–2.9 V) | Port B tri-stated in reset (S-T85), so the pump stops and K2 drops | OFF | — | PASS |
| A9 | ATtiny supply lost (LDO dead, wire off) | As A8 | OFF | — | PASS |
| A10 (#292) | **ATtiny fuses left at factory** (BOD disabled, CKDIV8 = 1 MHz, WDTON off) | At low VCC the AVR may run erratically and might keep toggling PB3. K1 and the MOSFET are still under RP2350 control | OFF (layer degraded) | **Bring-up step: read fuses back with avrdude and record them** | WARN |
| A11 | K1 contacts welded | K2 and the MOSFET still cut. MID_SENSE reads live while K1 is commanded off, so the firmware latches OutputMismatch (needs the MID_SENSE firmware addition) | OFF | MID_SENSE | PASS (firmware gap) |
| A12 | K2 contacts welded | K1 and the MOSFET still cut. Only visible through the arm-time self-test: the RP2350 sends one deliberately wrong answer and checks that VBUS falls to 0 | OFF | Arm-time self-test (**not written**) | WARN |
| A13 | AOD4184 #1 drain-source short | Rail A is on whenever armed, which matches its commanded state while armed. When the firmware disarms, K1 drops, so the lamp goes off. Disarmed: K1 and K2 are open | OFF when not armed | RAIL_A_SENSE vs command after disarm | PASS |
| A14 | AOD4184 #2 (rail B) short | Lamp B lights when armed + key on, with no lease. RAIL_B_SENSE reads live while commanded off, so on that tick the firmware drops GP12, stops the pump (K1 drops) and latches | ON for ≤1 tick + τ, then OFF | RAIL_B_SENSE | PASS (bounded window) |
| A15 | Q_HOLD1 (K1 driver) collector-emitter short | K1 is held permanently; K2 and the MOSFET still cut | OFF | MID_SENSE (when commanded off) | PASS |
| A16 | Q_HOLD2 short | K2 is held permanently; K1 and the MOSFET cut | OFF | Arm-time self-test only | WARN |
| A17 | Pump series capacitor shorts (either pump) | DC on the pin becomes base drive, so that relay follows the pin level. A frozen-high pin then holds that relay. The other relay still cuts | OFF | Per-cut self-test | PASS |
| A18 | Solder bridge between IN1 and IN2 (same module) | **Common cause:** the K1 stage also holds K2, so the ATtiny cut is defeated. A frozen RP2350 still drops both. A glitched RP2350 that keeps pumping but answers wrong is not cut by K2. Lamp lights only if the RP2350 also keeps GP12 on: two faults | OFF (single fault) | Arm-time self-test | WARN → split onto two relay modules (S1) |
| A19 | E-stop NC1 fails to open | NC2 opens, so ESTOP_SENSE makes the firmware drop everything. A welded NC1 and a dead RP2350 together is two faults | OFF | ESTOP_SENSE vs VBUS | PASS |
| A20 | E-stop wire off (power path) | Open circuit | OFF | VBUS 0, L_BUS dark | PASS |
| A21 | ESTOP_SENSE wire off | Pull-up reads pressed, so a fault latches | OFF | Fault | PASS |
| A22 | Arm toggle welded closed | Arm sense stays on and is never seen off-then-on after a restart, so nothing arms | OFF | — | PASS |
| A23 | Any wire off in the series chain | Open circuit | OFF | VBUS / sense | PASS |
| A24 | GP12 / GP13 wire off, or MOSFET logic GND off | Opto LED not driven | OFF | Sense: commanded on but dead, so a fault after OUTPUT_SETTLE | PASS |
| A25 | Sense wire off (RAIL_x_SENSE) | Reads dead. A later stuck-on would be hidden: two faults. Commanded on but dead latches a fault at the next arm | OFF | Arm-time | PASS |
| A26 | **5 V buck fails shorted (12 V on 5 V rail)** | Without M6: every MCU output undefined at once. That is a common cause across both brains, and relay coils at 12 V could pull in if any driver is on. With M6: the zener crowbar blows F2 (1 A), so all logic dies and the rail opens | OFF **only with M6** | — | FAIL as listed → PASS with M6 |
| A27 | INA228 dead or NACK | No VBUS. The firmware must latch a fault (firmware gap) | OFF | I2C error | WARN |
| A28 | Analyzer ground clipped to lamp-negative | Bypasses the MOSFET (G3) | **ON** | — | FAIL (procedure rule) |
| A29 | 12 V adapter overvoltage (24 V class) | Lamps rated 12–24 V; buck input ≥24 V; ATtiny/RP2350 behind the 5 V crowbar | OFF / safe | — | PASS (buck rating UNVERIFIED) |
| A30 | 5 V rail lost | Coils de-energise; RP2350 and ATtiny unpowered | OFF | — | PASS |

### 3.3 Single-fault table, variant B (variant A + MAX813L)

Variant B changes only the K2 hold: a MAX813L in socket U4 (5 V domain) drives clamp transistor Q_CLAMP. Jumper J_B connects the clamp to the pump-2 hold node. Q_CLAMP can only **pull the hold node down**. Every row of table A still holds. These rows are added or changed:

| # | Single fault | What happens | Lamp | Verdict |
|---|---|---|---|---|
| B1 | ATtiny runaway that keeps toggling PB3 but stops toggling PB4 | WDO goes low after 1.00–2.25 s (S-MAX), MR is pulled low, RESET goes high, the clamp drops K2 | OFF after ≤2.25 s | PASS (slow layer) |
| B2 | ATtiny fuses at factory (A10) and the 3.3 V supply sagging | PFI (12 k/10 k divider, trip 2.64–2.86 V) drives PFO low, which resets through MR and clamps K2 | OFF | PASS (closes A10) |
| B3 | 5 V rail below 4.50–4.75 V | RESET goes high and clamps K2 | OFF | PASS |
| B4 | Power-up | RESET is held for 140–280 ms after VCC > threshold, so K2 is held off during boot | OFF | PASS |
| B5 (#293) | MAX813L counterfeit or wrong threshold | Layer B unreliable; layer A unchanged | per A | WARN → bench acceptance test (section 5.3) |
| B6 | WDI wire off or level shifter dead-open | A floating WDI **disables the watchdog** (S-MAX); PFI and VCC monitoring still work | per A | WARN (latent; test with the test firmware) |
| B7 | Q_CLAMP open, or J_B removed | Layer B is gone; variant A remains | per A | WARN (latent) |
| B8 | Q_CLAMP shorted | K2 can never close: lost uptime, safe | OFF | PASS |
| B9 | RESET stuck high | Same as B8 | OFF | PASS |
| B10 | MAX813L pulled from the socket while J_B is fitted | Q_CLAMP base has a 100 kΩ pull-down, so it stays off and the circuit falls back to variant A | per A | PASS |
| B11 | Can variant B add an unsafe single fault? | No: every output only sinks the hold node or asserts reset | — | PASS |

## 4. Firmware and design conflicts this review found (for the trust-root issue, not hardware)

Issue map: F1 to F3 are #288, F5 is #289, F6 is #290, F8 is #291. F4 and F7 are firmware and evidence notes for #144.

| # | Conflict | Fix |
|---|---|---|
| F1 (#288) | `CHARGE_PUMP = 17` is a bottom pad (S-ZERO-DTS) | Move to GP14 |
| F2 (#288) | `RAIL_B_ENABLE[3] = 16` is the on-board WS2812 data pin | For P1, set `RAIL_B_PORTS = 1` (port 0 only), which frees GP9–11 and GP14–16 |
| F3 (#288) | No pins for I2C (INA228/OLED), the Q&A link, MID_SENSE or the lease LED | Pin map in `wiring-mock.md` §3: I2C1 GP26/27, SPI1 slave GP8–11, MID_SENSE GP29, RAIL_B_SENSE[0] GP28, lease LED GP15 |
| F4 | Firmware does not use INA228 VBUS or MID_SENSE for stuck-output detection, and has no arm-time per-cut self-test | Needed to close A11, A12, A16, A27 |
| F5 (#289) | THREAT-MODEL test "Stop works with the Pi powered off … press the stop with rails on" contradicts the heartbeat rule: with the Pi off the rails are already off | Redefine as 4a (bypass fixture, Pi off) + 4b (armed, stop pressed, then Pi off). See `wiring-mock.md` §8 |
| F6 (#290) | "Firmware cannot change without the jumper" | True only for the update path over the link. The BOOT button + USB, and SWD, bypass it (S-ZERO). For P1: the trust root has no USB lead, and the SWD lead goes through a removable connector that is out for the test. Otherwise, RP2350 OTP secure boot and debug disable (irreversible) is the only real fix: say so in the HWS row |
| F7 | #144 "hardware-limited armed rail": the XL4015 was dropped from P1 | The 1 A fuse is the only hard limit. Fine for the lamp proof; write it in the evidence |
| F8 (#291) | ROADMAP P1.1 check-in: NVMe boot and hub-port power cycling | NVMe kit and Phidgets hub not on the P1 list: drop from P1.1 or add the parts |

## 5. The watchdog supervisor question

### 5.1 Does ATtiny85 BOD + WDT + charge pump make the MAX813L redundant for P1?

**For P1: yes, as a matter of safety.** The MAX813L still adds diversity, and the owner has decided to fit it as variant B. Reasoning:

- **The real fail-safe is the charge-pump hold, not any supervisor.** A frozen, reset, browned-out or unpowered ATtiny produces no AC on PB3, so K2 drops in tens of milliseconds. No supervisor IC is needed for that (A7–A9).
- **The ATtiny's own BOD is its supply monitor** (fuse 101: 2.5–2.9 V at a 3.3 V supply; S-T85). While VCC is under the threshold the chip is held in reset with port B tri-stated, which is exactly "cut". That meets #144's "its own supply monitor".
- **WDTON (always-on WDT, 128 kHz independent oscillator)** turns a hang into a reset, which also cuts. It mainly helps recovery.
- **What the MAX813L cannot do here:**
  - Its threshold is 4.50–4.75 V, so it only watches a 5 V rail. It cannot watch the 3.3 V ATtiny directly; only PFI with a divider can.
  - Its watchdog is 1.00–2.25 s, 10–100× slower than the pump.
  - WDI needs ≥3.5 V, so a level shifter is required.
  - A floating WDI disables its watchdog.
  - Its RESET is active-high, the wrong polarity for the ATtiny.
  - It shares the ATtiny's software as the source of WDI edges, so a runaway loop that still toggles defeats it too.
  - The Amazon pack is no-name, with counterfeit risk.
- **What it does add (variant B):** a hardware layer that does not depend on the ATtiny fuses being right (B2). It watches the 5 V rail (B3), holds K2 off for 200 ms at boot (B4), and is diverse silicon. And it can only cut (B11).
- **Condition for variant A to stand alone:** the fuses `lfuse=0xE2 hfuse=0xCD efuse=0xFF` must be written, read back and recorded before lock bits mode 3 (`lock=0xFC`) is set. hfuse 0xCD = SPIEN on, WDTON on, BODLEVEL 101. The fuse bytes are a design choice: check them against the datasheet fuse tables before writing (#292). Also measure the pump drop-out time. Without the fuse read-back, A10 is open.

### 5.2 Recommendation

1. Build and prove **variant A** first (it stands alone; nothing waits for the MAX813L on 14 Dec).
2. Fit **variant B** later, with no rewiring of the rail path:
   - U4 is a DIP-8 socket on the watchdog board, already wired to +5 V, GND, the PFI divider, the WDI level shifter, the diode-AND into MR, and RESET → Q_CLAMP.
   - J_B is a 2-pin jumper between the Q_CLAMP collector (through 1 kΩ) and the pump-2 hold node.
   - Variant A = U4 empty, J_B open. Variant B = U4 fitted after it passes the acceptance test, J_B fitted.
3. Order **2 × genuine MAX813LCPA+ from DigiKey or Mouser** (about $5–7 each, ships in days; MAX813LCPA listed at DigiKey per S-WDR). Use them as known-good references for the acceptance test, and as the parts actually fitted if the Amazon lot fails.
4. **Prime-fast alternatives:** none found. No Adafruit, SparkFun or Pololu breakout of a supervisor IC came up in search (TI TPS3823 and Microchip MCP1xx parts are distributor-only, SOT-23/TO-92; MCP130 TO-92 availability UNVERIFIED). A second, cheaper independent dev board is not better than the ATtiny: it brings its own regulator, bootloader and USB attack surface.

### 5.3 MAX813L bench-acceptance (counterfeit) test (#293)

Test every chip from the Amazon lot off-board on the breadboard, against the datasheet limits (S-MAX). Run a genuine DigiKey part through the same steps as the control.

| Step | Setup | Measure | Accept |
|---|---|---|---|
| 1 | VCC 5.00 V from the adjustable buck (M2), meter in series on the µA range, WDI floating, MR open | ICC | ≤350 µA |
| 2 | Analyzer D0 on RESET (pin 7) through 10 kΩ, D1 on VCC through a 2:1 divider; switch VCC 0 → 5 V | RESET high time after VCC passes the threshold | 140–280 ms |
| 3 | Ramp VCC down slowly (≤50 mV/s) from 5.0 V; meter on VCC | VCC at which RESET goes high | 4.50–4.75 V |
| 4 | Ramp back up | Release point minus trip point | about 40 mV hysteresis (typ.) |
| 5 | WDI driven 10 Hz from a Pico GPIO through a 2N3904 inverter, 10 kΩ pull-up to 5 V; then stop the toggles (pin held) | Time from last edge to WDO (pin 8) low | 1.00–2.25 s |
| 6 | As 5, but release WDI to high-Z (remove the transistor) | WDO stays high: three-state detection works | no timeout |
| 7 | PFI from a 10 kΩ pot across 2.5 V (AD584 output); meter on PFI | PFI voltage at which PFO (pin 5) switches | 1.20–1.30 V |
| 8 | MR to GND through the meter | MR pull-up current | 100–600 µA |
| 9 | Pull MR low for 1 s | RESET high while MR low + 140–280 ms | yes |

Lot rule: if more than 2 of 10 chips fail any step, or any chip shows the wrong polarity (an active-low RESET means a mislabelled MAX813 variant), treat the whole lot as suspect and fit the DigiKey parts. Record each chip's numbers as measured facts.

## 6. Missing parts

"Owned?" is a judgement against the owner's stated kit (breadboard, resistor/LED assortment, $20 meter, Pinecil V2, PinePower, MacBook, Bambu A1 mini).

| ID | Part | Why | Spec | Owned? | Priority |
|---|---|---|---|---|---|
| M1 | 12→5 V buck module | Logic rail (P4) | Input ≥24 V max, output 5.0–5.1 V, ≥1 A (e.g. LM2596 module, or Pololu D24V10F5 genuine) | No | **Must** |
| M2 | Adjustable source for brownout and MAX813L tests | Test 1, §5.3 (P11) | Second LM2596-ADJ module with a multi-turn pot (0–5.5 V out of 12 V), or a 30 V/5 A bench supply | No (PinePower is fixed-step) | **Must** |
| M3 | Female barrel jack → screw terminal adapters, ×2 | Land the 12 V adapter (P1) | 5.5×2.1 mm (fits 2.1 tip; 2.5 also fine for the female) | No | **Must** |
| M4 | ATC/ATO blade fuses 1 A and 2 A, 5 each | F1, F2 (P2) | 32 V ATC | No (kit is 5–30 A) | **Must** |
| M5 | 3.3 V LDO for the watchdog board | Independent ATtiny supply (P9) | MCP1700-3302E/TO (TO-92, 250 mA) or AMS1117-3.3 module; 1 µF in/out (MCP1700) or 10 µF (AMS1117) | No | **Must** |
| M6 | 5 V rail crowbar | Buck-short fault (A26) | 1N5339B 5.6 V 5 W axial zener (with F2 1 A). Alt: 1N5338B 5.1 V if the buck is set ≤4.9 V (not recommended) | No | **Must** |
| M7 | PC817 optocouplers (DIP-4) | Sense: arm, key, mid, rail A, rail B (L9) | ×5 + spares (10-pack); 4.7 kΩ ¼ W on the 12 V side; 10 kΩ on the logic side | No | **Must** |
| M8 | 2N3904 NPN | Pump holds ×2, Q_CLAMP, WDI level shift, lease LED | ×5 used, buy a 50-pack | Unlikely | **Must** |
| M9 | Schottky diodes BAT43/BAT85 or 1N5817 | Two per pump | ×4 + spares | No | **Must** |
| M10 | Capacitors | Pumps and decoupling | 100 nF film/C0G ×4 (pump series; **not** Y5V); 1 µF film or X7R ×2 (store); 100 nF ceramic ×8; 10 µF electrolytic ×4 | No | **Must** |
| M11 | 1N4148 ×4, 1N4007 ×2 | Diode-AND into MR (variant B); reverse protection on the watchdog 5 V input | — | No | **Must** (1N4148 for B) |
| M12 | DIP-8 sockets ×4 | ATtiny (removable for off-board ISP), U4 MAX813L, spares | Turned-pin | No | **Must** |
| M13 | Micro-USB data cable | Pico debug probe | USB-A to micro-B, data | Maybe | **Must** |
| M14 | USB-A to USB-B cable | Waveshare adapter, if not in the box | — | Maybe | Must if missing |
| M15 | Dummy load per rail | Real current for INA228 evidence (I4) | 47 Ω 5 W wirewound ×2 (255 mA, 3.1 W), or a 12 V 194/T10 bulb + socket ×2 | No | Should |
| M16 | Mounting plate + 22 mm panel | Holds everything; 22.3 mm holes for e-stop, key, lamps; 16 mm for the button; 12 mm toggle (UNVERIFIED, measure) | Print on the A1 mini in PETG (180 mm bed: split panel and plate) | Printer owned, filament assumed | **Must** |
| M17 | Ferrules 0.34 mm² (22 AWG) + crimper | Stranded silicone wire into screw terminals (do not tin it) | — | No | Should |
| M18 | 2.54 mm jumper shunts | Firmware jumper, J_B, test-mode links | — | Maybe in header kits | Should |
| M19 | Female header strips 2.54 mm | Socket the RP2350-Zero boards on perfboard | — | Maybe in Smraza kit (#8) | Should |
| M20 | Second 2-ch relay module | One cut per board (A18 common cause) | Same SunFounder B00E0NTPP4 | No | Should |
| M21 | USB-C PD trigger board, 15 V | AD584 10 V output (P10) | ZY12PDN-class, set 15 V | No | Nice |
| M22 | Extra NO contact block for the key switch | Dry key sense independent of rail A (X3) | ZB2-BE101 class | No | Nice |
| M23 | Solder, flux | Everything | — | Assumed (repo says so) | Verify |
| M24 | 2 × genuine MAX813LCPA+ (DigiKey/Mouser) | Reference and fallback for variant B | DIP-8 | No | Should (owner keeps B) |

ATtiny programming needs no new hardware. Program it **off-board** on the breadboard: pull it from its socket, program from the Pi 5 header with `avrdude -c linuxgpio` (avrdude ≥7.x with libgpiod; Pi 5 support UNVERIFIED), or with the Pico running an ArduinoISP-type sketch (UNVERIFIED on RP2040). Off-board programming means the Pi cannot reflash the watchdog in place, which suits its independence.

## 7. Substitutions

| Part | Swap? | Reason |
|---|---|---|
| SunFounder 2-ch low-level relay (#17) | **Keep, behind the transistor stage only.** Add a second board (M20) | With an open-collector driver, active-low is the right sense (it releases on everything). The hazard was only direct GPIO drive (L1). A high-level or jumper-selectable module is no safer behind a driver, and directly driven it still fails on a pin stuck high. No swap needed |
| MAX813L (#1, no-name) | Keep (owner), test per §5.3, back up with genuine parts (M24) | Counterfeit risk |
| Nilight fuses 5–30 A (#14) | Add M4 | Too large for 22 AWG |
| AOD4184 modules (#18) | Keep for P1 (lamp, no shared ground) | Low-side. Before any grounded load (P2 pods), move the controller's cut to the high side (S-REPO cross-ref) |
| SH-G01A (#11) | Keep, no P1 role | See 2.5 |
| iiunius (#9) | Keep for the P1.5 loopback | Auto-direction is fine for the guard proof; the listen-only/DE-pin problem is P2 |
| 12 V 3 A adapter (#27) | Keep | Adequate for lamps and the logic branch |

## 8. Unverified facts to check with the part in hand (bring-up list)

RP2350 stepping (A2/A4, printed on the package). SunFounder relay: input-chain resistor, flyback diode present, which relay part. AOD4184 module: gate voltage at 3.3 V input, "+" terminals joined. INA228 module: I2C default address, pull-ups, whether VBUS is tied to IN−, shunt value (R002). OLED address and pull-ups. E-stop contacts both NC and positive-opening. Key switch contact type and key-removal position. Toggle hole size. Button LED current. Lamp polarity. Waveshare: USB cable included, terminal names. iiunius: terminator and bias, TTL pin names. LA: 74LVC245 fitted. Pi kit PSU: 5 A negotiation. Every USB-C cable carries data. ATtiny85 authenticity (read the signature 0x1E 0x93 0x0B).
