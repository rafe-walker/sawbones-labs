# P1 bill of materials

The parts the P1 bench is built from, with the manufacturer, model, ASIN, quantity ordered, role and datasheet or maker page. `#` is the line number cited as `#n` in [compatibility.md](compatibility.md) and [wiring-mock.md](wiring-mock.md). The ASIN identifies the product listing only; it is not a link, and this repository carries no prices, images or affiliate links. Electrical facts are in [`data/parts.toml`](data/parts.toml), which is the machine-readable source; this table is its human-readable index.

## On the P1 list

| # | Maker | Model | ASIN | Qty | Role | Datasheet or maker page |
|---|---|---|---|---|---|---|
| 1 | Maxim/ADI (no-name marketplace pack) | MAX813L DIP-8 | B0BJKH7ZDB | 1 (10 pcs) | Variant B supervisor for the watchdog cut (socket U4) (U4_MAX813L) | https://www.analog.com/media/en/technical-documentation/data-sheets/MAX705-MAX813L.pdf |
| 2 | Microchip (Atmel) | ATTINY85-20PU | B06W9JBJJ6 | 1 (pack of 5) | Independent Q&A watchdog; drives charge pump 2 (K2) (U3_ATTINY85) | https://ece-classes.usc.edu/ee459/library/documents/Atmel-2586-AVR-8-bit-Microcontroller-ATtiny25-ATtiny45-ATtiny85_Datasheet.pdf |
| 3 | Umefly | UC01M004 | B08HV7GT4H | 1 (3-pack) | USB-A to C for RP2350 #2 (guard); never on the trust root (CBL_USBC_3PK) | none recorded |
| 4 | AvrestYPT | FY-365-A144 | B0FPMC9917 | 1 (365 pcs kit) | M2.5 nylon standoffs (HW_STANDOFF) | none recorded |
| 5 | JSZAAHZ | 102 pcs 2.54/5.08 mm terminal kit | B0H41RQ3LH | 1 (102 pcs kit) | Screw terminals (HW_TERMINALS) | none recorded |
| 6 | ELEGOO | EL-CP-004 | B01EV70C78 | 1 (120 pcs) | Logic and probe jumpers only; never the load path (HW_DUPONT) | none recorded |
| 7 | DEKIEVALE | 22 AWG silicone | B0CQP2JZSN | 1 (8 colours x 16.5 ft) | All rail and panel wiring (WIRE_22AWG) | none recorded |
| 8 | Smraza | 104 pcs proto kit | B07NM68FXK | 1 (104 pcs kit) | TR, WD, PWR, GUARD boards (PCB_PROTO) | none recorded |
| 9 | iiunius | Isolated TTL-RS485 (C) | B0DBCF4ZNZ | 1 | Guard-side RS485 transceiver (auto direction) (IU485) | none recorded |
| 10 | Waveshare | USB TO RS232/485/TTL (FT232RL, isolated) | B07L2VLY5D | 1 | Pi-side RS485 end of the bus-guard loopback (WS485) | https://waveshare.com/wiki/USB_TO_RS232/485/TTL |
| 11 | DSD TECH | SH-G01A (ADuM3160) | B093LKN3YH | 1 | Not used in P1 (kept for P2 module 1) (SHG01A) | https://www.desertcart.co.za/products/327395033-dsd-tech-sh-g01a-usb-isolator-with-adum3160-chip-12m |
| 12 | GODIYMODULES (TI INA228) | INA228 module, R002 shunt | B0GX5HM4X3 | 1 | High-side current and VBUS on RAIL_BUS; stuck-relay feedback (INA228) | https://www.ti.com/product/INA228 |
| 13 | Acxico (ADI AD584) | AD584 reference module | B0B4WN2Q91 | 1 | P1.4 ground truth; MAX813L PFI acceptance test source (AD584) | https://www.analog.com/en/products/ad584.html |
| 14 | Nilight | 50029R 5-30 A | B07Q9PL4R6 | 1 (120 fuses, 5-30 A) | Spares only: too large for the 22 AWG load path (FUSE_KIT) | none recorded |
| 15 | Anyongora holder + M4 fuse | Inline ATC holder, 1 A fuse | B0CL7MLY6T | 1 (4 holders + 40 fuses) | Load branch fuse; Logic branch fuse (taken before the e-stop); blown by the M6 crowbar (F1, F2) | none recorded |
| 16 | Gebildet | AD16-22D/S 12-24 V 20 mA | B0816FMP9B | 1 (12 pcs) | Loads (lamp A, lamp B) and indicators (L_PWR, L_BUS) (LAMP) | none recorded |
| 17 | SunFounder | 2-channel 5 V relay, opto, low-level trigger | B00E0NTPP4 | 1 | K1 (RP2350 dead-man) and K2 (ATtiny watchdog cut); COM-NO in series; Put K2 on its own board (common-cause split) (RLY, M20_RLY2) | https://components101.com/switches/5v-dual-channel-relay-module-pinout-features-applications-working-datasheet |
| 18 | DIYables | AOD4184 opto MOSFET module | B0G5NNG1VZ | 1 (5 modules) | Rail A enable (low side), GP12; Rail B port 0 enable (low side), GP13 (MOS_A, MOS_B) | https://protosupplies.com/product/d4184-mosfet-control-module/ |
| 19 | Hosyond | 0.96 in SSD1306 I2C | B09T6SJBV5 | 1 (5 pcs) | Trusted panel display (OLED) | none recorded |
| 20 | mxuteuk | CB16AY-11D-DC12V | B086SPXGTS | 1 (5 pcs) | Lease (panel approval) button; 12 V LED (BTN_LEASE) | none recorded |
| 21 | DaierTek | KN3C-101 + SAC-01-R | B07T6YNRZS | 1 | Arm switch, in series in the load path (SW_ARM) | none recorded |
| 22 | Shopcorp | XB2-BG21 | B08JWP92K4 | 1 | Key switch, in series on the rail-B branch (SW_KEY) | https://www.se.com/us/en/product/XB4BG21/ |
| 23 | mxuteuk | HB2-ES544 2NC latching | B07R8PTTDX | 1 (2 pcs) | NC1 in the load path; NC2 sense to GP2 (ESTOP) | none recorded |
| 24 | Waveshare | RP2350-Zero (RP2350A) | B0DYTVCF2H | 2 packs of 2 = 4 boards | Interlock trust root (#1 of 4); Bus guard (#2 of 4); USB-powered from the Pi (RP_TR, RP_GUARD) | https://www.waveshare.com/wiki/RP2350-Zero |
| 25 | HiLetgo | 24 MHz 8 ch (fx2lafw) | B077LSG5P2 | 1 | Timing evidence (LA) | https://sigrok.org/wiki/Noname_Saleae_Logic_clone |
| 26 | Bicool (Raspberry Pi) | Pico RP2040, headers | B08X7HN2VG | 1 | SWD debug probe (debugprobe firmware) (PICO) | none recorded |
| 27 | Facmogu | AL-1230 12 V 3 A | B073WSWT34 | 1 | Bench 12 V (load + logic branches) (PSU12) | none recorded |
| 28 | SanDisk | SDSQUA4-032G-GN6MN | B08L5HMJVW | 1 | Pi boot media (SD32) | none recorded |
| 29 | iRasptek (Raspberry Pi) | Pi 5 4 GB kit | B0FL21867J | 1 | Brain: supervisor, PULSE, analyzer host (tests 1,2,3,5,6) (PI5) | https://pip-assets.raspberrypi.com/categories/685-app-notes-guides-whitepapers/documents/RP-009856-WP-1-USB%20Power%20delivery%20on%20Raspberry%20Pi%205.pdf |

## Missing parts (not on the P1 list; see compatibility.md section 6)

| ID | Part | Priority | Role | ASIN | Datasheet or maker page |
|---|---|---|---|---|---|
| M1 | 12->5 V buck >=1 A (LM2596 or Pololu D24V10F5) | must | 5 V logic rail |  | none recorded |
| M2 | Adjustable buck 0-5.5 V (LM2596-ADJ, multi-turn) or bench supply | must | Brownout ramp (test 1), MAX813L acceptance |  | none recorded |
| M3 | Female 5.5x2.1 barrel to screw terminal, x2 | must | Land the 12 V adapter |  | none recorded |
| M4 | ATC 1 A and 2 A, 5 each | must | F1, F2 |  | none recorded |
| M5 | MCP1700-3302E/TO or AMS1117-3.3 module | must | 3V3_WD for the ATtiny (independent supply) |  | none recorded |
| M6 | 1N5339B 5.6 V 5 W | must | 5 V rail crowbar (blows F2 on buck short) |  | none recorded |
| M7 | PC817 DIP-4 x5 (+spares) | must | U-ARM, U-KEY, U-MID, U-RA, U-RB sense |  | none recorded |
| M8 | 2N3904 NPN | must | Q51, Q61 (pump holds), Q62 (WDI shift), Q63 (clamp), lease LED |  | none recorded |
| M9 | BAT43/BAT85/1N5817 | must | Pump diodes D51 D52 D61 D62 |  | none recorded |
| M10 | 100 nF film x4, 1 uF film x2, 100 nF ceramic x8, 10 uF x4 | must | Pumps, decoupling |  | none recorded |
| M11 | 1N4148 x4, 1N4007 x2 | must | MR diode-AND (variant B), WD reverse protection, opto LED reverse clamp |  | none recorded |
| M12 | DIP-8 turned-pin socket x4 | must | U3 ATtiny (off-board ISP), U4 MAX813L |  | none recorded |
| M13 | USB-A to micro-B data cable | must | Pico probe |  | none recorded |
| M14 | USB-A to USB-B cable | must | Waveshare adapter (if not in the box) |  | none recorded |
| M15 | 47 ohm 5 W wirewound x2 (or 12 V 194 bulb + socket x2) | should | Real current for INA228 evidence |  | none recorded |
| M16 | Printed PETG plate + 22 mm panel | must | Mounting |  | none recorded |
| M17 | 0.34 mm2 ferrules + crimper | should | Stranded wire into screw terminals |  | none recorded |
| M18 | 2.54 mm jumper shunts | should | Firmware jumper, J_B |  | none recorded |
| M19 | 2.54 mm female header strips | should | Socket the RP2350-Zero boards |  | none recorded |
| M20 | SunFounder 2-ch relay (second board) | should | Put K2 on its own board (common-cause split) | B00E0NTPP4 | none recorded |
| M21 | USB-C PD trigger 15 V | nice | AD584 10 V output |  | none recorded |
| M22 | ZB2-BE101 NO contact block | nice | Dry key sense |  | none recorded |
| M24 | Analog Devices MAX813LCPA+ x2 (DigiKey/Mouser) | should | Acceptance-test control and fallback for U4 |  | https://www.analog.com/media/en/technical-documentation/data-sheets/MAX705-MAX813L.pdf |

## Owned

| Part | Role |
|---|---|
| Apple MacBook Pro M3 | Analyzer host for exit test 4 |
| PINE64 PinePower Desktop | Fixed USB PD steps only (not a brownout source) |
| unknown (#11) ~$20 handheld | Static voltages, continuity, uA for MAX813L ICC |
