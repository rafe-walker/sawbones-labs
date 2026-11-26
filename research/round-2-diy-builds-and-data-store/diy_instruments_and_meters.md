# Cheapest agent-readable multimeters and ground-up DIY electrical test instruments (state as of 2026-11-25)

Conventions used below
- "Ali" = AliExpress search-result "from" price fetched 2026-11-25. These are lowest-variant listing prices, exclude shipping and US import duty, and multi-model listings (e.g. "OW18A/B/D/E") may show the cheapest variant. Treat as indicative, not quotes. USD 1.09 / 0.33 "welcome deal" prices were discarded.
- Amazon US and Walmart could not be read (robots rules), so there are no first-hand Amazon prices in these notes.
- Verdict tags: **P1** = phase 1, **P2** = phase 2, **SKIP**.
- Nothing here was bench-tested by the researcher; protocol claims come from the linked repos.

## 1. Cheap bought handheld DMMs with BLE or USB/serial data and known protocols

### Takeaway
Two cheap BLE families have well-documented, pairing-free protocols usable from Linux: the Zotek-made Aneng/ZOYI/BSIDE family (ZT-300AB about USD 23-28 on Ali; a Rust client already exists) and OWON's OW18/B35/B41/CM2100B (OW18 series from about USD 43 on Ali, about USD 70-85 at US retail). The UNI-T UT61E+ (USB HID via CP2110, about USD 43-45 on Ali, USD 125-140 US retail) is the only one whose open client is shown reporting range, mode, hold and a low-battery flag in one structured reply.

### Cited Findings

Zotek family: Aneng AN9002 / V05B / ST207 / AN999S, ZOYI and BSIDE ZT-300AB / ZT-5B / ZT-5BQ / ZT-5566S
- Models documented as one protocol family: clamp (Aneng ST207, BSIDE/ZOYI ZT-5BQ), small (Aneng V05B, ZT-5B), big (Aneng AN9002, ZT-300AB, BABATools AD900, HOKI-M9), desk (Aneng AN999S, ZOYI ZT-5566S) — [ludwich66 wiki](https://github.com/ludwich66/Bluetooth-DMM/wiki)
- Transport: BLE service 0000fff0, notify characteristic 0000fff4 (handle 0x0009); the worked example is a plain gatttool connect followed by notifications, with no pairing step shown — [Bluetooth Analyses](https://github.com/ludwich66/Bluetooth-DMM/wiki/Bluetooth---Analyses)
- Payload: 10 or 11 bytes, XOR-ed bytewise with a fixed published key (41 21 73 55 a2 c1 32 71 66 aa 3b ...); after XOR the bits are raw LCD segments plus annunciators (Hold, Auto, AC/DC, units, Min/Max, Delta, Buzzer, a "Batt" bit and an "H.V" bit on the clamp layout) — [Protocol all Variants](https://github.com/ludwich66/Bluetooth-DMM/wiki/Protocol-all-Variants)
- So the link is a display mirror: range is implied by decimal point and unit segments; there is no documented command channel to change mode or range — [Protocol all Variants](https://github.com/ludwich66/Bluetooth-DMM/wiki/Protocol-all-Variants)
- Update rate about 2.6 readings/s, not adjustable; BLE module is an F9788, main chip under a blob — [justanotherelectronicsblog](https://justanotherelectronicsblog.com/?p=930)
- Rust: "A Rust-based DMM client for cheap Bluetooth Low Energy digital multimeters from AliExpress"; protocol library and CLI described as functional, GUI unfinished; workspace crates btdmm_comm, cli, gui; not on crates.io — [depau/bluetooth-dmm-rust](https://github.com/depau/bluetooth-dmm-rust)
- Python (bleak) port, 11-byte variant tested on a real device, 10-byte only against documented values — [Shiro-Nek0/Bluetooth-DMM.py](https://github.com/Shiro-Nek0/Bluetooth-DMM.py); logger for AN9002 — [riktw/AN9002_info](https://github.com/riktw/AN9002_info)
- Auto power off on AN9002 is disabled by holding SEL while switching on (five beeps), quoted from the manual — [hcet14 fork README](https://github.com/hcet14/Aneng_AN9002_Bluetooth_DMM_data_logger_-fork-_and_big_mechanical-electrical_design_flaw)
- Reported hardware flaw: AN9002 input terminals lose contact after about 100 mating cycles because of a slotted jack design; fix is solder or replacement sockets — [hcet14 fork README](https://github.com/hcet14/Aneng_AN9002_Bluetooth_DMM_data_logger_-fork-_and_big_mechanical-electrical_design_flaw)
- Prices (Ali, 2026-11-25): ZOYI ZT-300AB (6000 counts) USD 23.42 and 28.41; one AN9002 listing at USD 43.16 — [Ali ZT-300AB search](https://www.aliexpress.com/w/wholesale-ZOYI-ZT-300AB-bluetooth-multimeter.html), [Ali AN9002 search](https://www.aliexpress.com/w/wholesale-ANENG-AN9002-bluetooth-multimeter.html)
- Aneng's own US-facing store (2026-11-25) no longer lists the AN9002; its current "Smart" models run USD 29.99-69.99 (M129 4000 counts USD 29.99, 620A 6000 counts USD 45.99) and their data protocols are not in any repo found — [anengtools.com product feed](https://anengtools.com/products.json)

OWON OW18E / OW18B / B35T+ / B41T+ / CM2100B
- One Linux CLI covers B35T+, B41T+, OW18E and CM2100B, with JSON output, one-shot mode and logging; it shells out to gatttool and is normally run with sudo; an ESP32 client is in the repo — [sercona/Owon-Multimeters](https://github.com/sercona/Owon-Multimeters)
- BLE service 0000fff0, characteristic 0000fff4 in the ESP32 client; notify handle 0x001b on OW18E and CM2100B, 0x002e on B35/B41 — [sercona source](https://github.com/sercona/Owon-Multimeters)
- Payload is 6 bytes (three 16-bit words): function code (4 bits), scale (3 bits), decimal position (3 bits) and the numeric reading; decoded functions include DC/AC V, DC/AC A, ohms, capacitance, diode, continuity, frequency, duty, temperature, hFE, NCV — [sercona source](https://github.com/sercona/Owon-Multimeters)
- Example log timestamps are about 0.5 s apart (about 2 readings/s) — [sercona README](https://github.com/sercona/Owon-Multimeters)
- B35/B35+/B35T+ (post mid-2017, Semic CS7729CN-001 chip) have a second client with interactive remote control of the meter's buttons over BLE (Select, Auto, Range, Hold, Backlight, Delta, Hz/Duty, Min/Max, Bluetooth off) and start/download of offline recordings; needs gattlib — [DeanCording/owonb35](https://github.com/DeanCording/owonb35)
- Older B35 units with the FS9922 chip use a different client — [inflex/owon-b35](https://github.com/inflex/owon-b35)
- B41T+ Windows client also sends simulated key presses and text commands such as `*READlen?` / `*READ1?` for offline data — [art-ya/OwonB41T](https://github.com/art-ya/OwonB41T)
- B35T teardown: 250 V 5x20 mm fuses (1 A glass, 20 A ceramic), auto power off about 15 minutes, display 3 updates/s, "Bluetooth cannot automatic recover a broken connection", about 200 h battery with Bluetooth; reviewer notes the protocol is documented on GitHub — [lygte-info B35T](https://lygte-info.dk/review/DMMOwon%20B35T%20UK.html)
- B41T+ teardown: voltage input "only has PTC's, not MOV's", 250 V fuses, auto power off about 30 minutes, 2 updates/s, offline logging runs in the Bluetooth module and can only be started from a phone/BLE client — [lygte-info B41T+](https://lygte-info.dk/review/DMMOwon%20B41T%20UK.html)
- OW18E: 4.5 digits (19999), true RMS, 9 V battery, 20 A for 10 s max — [Elektor review, 2020](https://www.elektormagazine.com/news/review-owon-ow18e-bluetooth-multimeter)
- OW18E CAT IV 600 V / CAT III 1000 V is self-declared with no fuse ratings published and is flagged doubtful; US price about USD 70-85 (checked November 2026) — [computingforgeeks, 2026-11-07](https://computingforgeeks.com/best-multimeter-electronics/)
- Prices (Ali, 2026-11-25): OW18 series (A/B/D/E listing) from USD 43.09 and 45.79; B35T+ USD 59.99-62.00; B41T+ from USD 64.99; CM2100B (20000-count AC/DC clamp) USD 37.79-44.88 — [Ali OW18E](https://www.aliexpress.com/w/wholesale-OWON-OW18E.html), [Ali B35T+](https://www.aliexpress.com/w/wholesale-OWON-B35T-bluetooth-multimeter.html), [Ali B41T+](https://www.aliexpress.com/w/wholesale-OWON-B41T-multimeter.html), [Ali CM2100B](https://www.aliexpress.com/w/wholesale-OWON-CM2100B-clamp-meter-bluetooth.html)
- OWON XDM1041 on Ali (2026-11-25): "XDM1041 XDM1241 ... 55000 Counts 4.5 Digit Bench Multimeter" listings from USD 84.79-90.09 (variant unconfirmed, shipping/duty excluded) — [Ali XDM1041](https://www.aliexpress.com/w/wholesale-OWON-XDM1041-bench-multimeter.html)
- XDM1041 SCPI command list was community-derived by testing; firmware 25052021 has a bug where any non-query command returns "OK" three times, even for nonsense input — [TheHWcave/OWON-XDM1041](https://github.com/TheHWcave/OWON-XDM1041)

UNI-T UT61E+ / UT161 / UT60BT / UT181A / UT61E
- UT61E+ over the CP2110 HID bridge from Python on Linux; README says it should also work for UT60BT, UT61B+, UT61D+, UT161B/D/E with unit tables adjusted; sample output fields: mode, range, display, overload, unit, max, min, hold, rel, auto, battery, hvwarning, dc, peak — [ljakob/unit_ut61eplus](https://github.com/ljakob/unit_ut61eplus)
- Go client: CP2110 set to 9600 8N1; host sends a request, meter answers with a 19-byte message carrying mode, range, value, sign (request/response, not a free-running stream) — [mwuertinger/ut61ep](https://github.com/mwuertinger/ut61ep)
- UT61E+ 22,000 counts, about USD 125-140 in the US; CAT IV 600 V / CAT III 1000 V self-declared, no fuse ratings published, current jacks marked 250 V; the UT161E (about USD 162) is the ETL-certified sibling — [computingforgeeks](https://computingforgeeks.com/best-multimeter-electronics/)
- Ali (2026-11-25): "UT61B/D/E Plus" listings from USD 42.85 and 44.83 (cheapest variant may be the UT61B+) — [Ali search](https://www.aliexpress.com/w/wholesale-OWON-OW18E.html)
- Older UT61E (ES51922A chip): sigrok-supported; teardown found MOVs missing and 240 V fuses, reviewer says the CAT rating "is definitely not correct" — [sigrok](https://sigrok.org/wiki/Supported_hardware), [lygte-info UT61E](https://lygte-info.dk/review/DMMUNI-T%20UT61E%20UK.html)
- UT181A is sigrok-supported over USB but costs USD 488 at UNI-T's US store — [sigrok](https://sigrok.org/wiki/Supported_hardware), [UNI-T US store feed](https://www.uni-trendus.com/products.json)
- UNI-T's US store lists the UT8802E 4.5-digit bench meter at USD 129 and UT8803E at USD 168 (2026-11-25) — [UNI-T US store feed](https://www.uni-trendus.com/products.json)

sigrok-classic USB/RS232 meters
- Supported in libsigrok: UNI-T UT60E, UT61B/C/D/E, UT71C, UT181A; Victor 70C and 86C (USB); TekPower TP4000ZC (RS232); many Voltcraft VC-8xx/9xx; Mastech MS8250B/D; EEVblog 121GW (BLE, SD); Brymen BM257/BM52x/BM82x/BM85x/BM869 — [sigrok supported hardware](https://sigrok.org/wiki/Supported_hardware)
- Not found in the sigrok list as fetched: any OWON handheld, UT61E+, UT60BT, Victor 86B, HoldPeak, Aneng, BSIDE; OWON XDM2041 is "work in progress" — [sigrok supported hardware](https://sigrok.org/wiki/Supported_hardware)
- HoldPeak HP-90EPC (USB): Ali USD 43.63-59.26 (2026-11-25) — [Ali HP-90EPC](https://www.aliexpress.com/w/wholesale-HoldPeak-HP-90EPC-USB-multimeter.html)
- EEVblog 121GW: HY3131 chip, BLE112 module, auto power off about 30 minutes changeable in setup, firmware updates from microSD, CAT III 600 V — [lygte-info 121GW](https://lygte-info.dk/review/DMMEEVBlog%20121GW%20UK.html); listed at 180 and out of stock on the EEVblog store feed (store currency not confirmed) — [eevblog.store feed](https://www.eevblog.store/products.json); about USD 235 per [computingforgeeks](https://computingforgeeks.com/best-multimeter-electronics/)

FNIRSI / ZOYI scope-meters / Kaiweets / Pokit
- FNIRSI official store (2026-11-25): DMT-99 USD 42.49, S1 USD 29.99, 2C53T USD 103.24-106.24, 2C23T USD 79.99-89.24, DSO-TC3 USD 43.99-49.99, DSO-TC4 USD 79.99, DSO-510 USD 43.99, LCR-P1 USD 36.80 — [fnirsi.com feed](https://www.fnirsi.com/products.json)
- An open, clean-room FreeRTOS firmware exists for the 2C53T (AT32F403A + Gowin FPGA): multimeter data flow and generator controls work, scope acquisition is still in development; first flash needs the case opened; no remote-control API is described — [OpenScope 2C53T](https://osrtos.com/projects/openscope-2c53t)
- ZOYI ZT-703S (2-ch 50 MHz scope + 25,000-count DMM + generator) USD 66.89 and ZT-702S USD 69.35 on Ali (2026-11-25) — [Ali ZOYI search](https://www.aliexpress.com/w/wholesale-ZOYI-ZT-5B-bluetooth-multimeter.html)
- Kaiweets "Smart" meters: KM601S 10,000 counts USD 59.99, KM602 USD 55.99, HT118E 20,000 counts USD 55.99 (2026-11-25) — [kaiweets.com feed](https://kaiweets.com/products.json)
- Pokit Meter and Pokit Pro: open cross-platform Qt library and CLI (multimeter, DSO, data logger), mostly tested on Linux; Pokit Pro capacitance mode is undocumented by Pokit and unsupported — [pcolby/dokit](https://github.com/pcolby/dokit)

Rust coverage for bought meters
- `oxrok` 0.3.0-alpha.1 (created 2026-10-23): a Rust port of the sigrok stack (88 driver modules, 193 driver ids, mostly multimeters) that also "serves the Model Context Protocol over stdio"; by its own README it is an LLM translation and "most of it has never touched the device it drives"; hardware-run so far: fx2lafw clone, Rigol DS1102E, LogicPort, and a Pico with sigrok-pico; serial DMM drivers only against microcontroller stand-ins — [oxrok on crates.io](https://crates.io/crates/oxrok)
- Other crates found (2026-11-25): `bm78x` (Brymen BM78xBT BLE), `fluke-connect-client`, `sigrok` 0.3.1 bindings last updated 2016; no crate named for OWON, Aneng or Pokit meters — [crates.io search](https://crates.io/search?q=multimeter)

Per-option summary (cost / link / state reporting / verdict)

| Meter | Cost (date, source) | Counts | Link + open client | Reports range/mode/batt? | APO | Verdict |
|---|---|---|---|---|---|---|
| ZOYI ZT-300AB / Aneng AN9002 family | USD 23-43 Ali | 6000 | BLE notify, XOR'd LCD segments; Rust + Python clients | Mode/units/Auto/Hold; Batt bit documented on clamp layout | Disable: hold SEL at power-on (AN9002 manual) | **P1** cheapest real DMM with a Rust client; read-only |
| OWON OW18E / OW18B | USD 43-46 Ali; USD 70-85 US | 20,000 | BLE notify, 6-byte numeric; C CLI + ESP32 client | Function, scale, decimal; no battery flag documented | Not documented | **P1** at USD 100 tier |
| OWON B35T+ | USD 60-62 Ali | 6000 | BLE; remote button control | Function/scale; remote Range/Select | 15 min (B35T) | P2 (pricier, fewer counts; remote control is the draw) |
| OWON B41T+ | from USD 65 Ali | 22,000 | BLE; remote keys + offline log | as B35 | 30 min | P2 |
| OWON CM2100B clamp | USD 38-45 Ali | 20,000 | BLE, same CLI | as OW18E | Not documented | **P1/P2** for DC clamp current on 12 V / 48 V |
| UNI-T UT61E+ | USD 43-45 Ali (variant?); USD 125-140 US | 22,000 | USB HID CP2110, polled; Python + Go | Yes: mode, range, hold, rel, auto, battery, hvwarning | Not documented | **P1 alt** if Ali price holds; richest state |
| HoldPeak HP-90EPC | USD 44-59 Ali | n/f | USB; no driver found | n/f | n/f | SKIP |
| Victor 70C/86C, TP4000ZC | n/f | n/f | sigrok drivers | chipset-dependent | n/f | SKIP unless found used |
| FNIRSI DMT-99 / S1 | USD 42.49 / 29.99 | 10,000 (DMT-99, Ali title) | no data protocol found | n/a | n/f | SKIP |
| FNIRSI 2C53T | USD 103-106 | n/f | open firmware in progress | n/a | n/f | P2 watch |
| Pokit Pro | n/f | n/f | BLE, dokit (Qt/C++) | yes via dokit status | n/f | P2 |
| EEVblog 121GW | 180-235 | n/f | BLE, sigrok driver | yes | 30 min, configurable | SKIP on cost |

### Inferences
- The Zotek-family protocol being a segment mirror is an advantage for an agent: every annunciator the human sees is in the packet, and decoding is a table lookup that ports to Rust `no_std` trivially.
- OWON's 6-byte numeric packet is the smallest decode job of all; a `bluer`/`btleplug` client is an afternoon's work, and removes the sercona CLI's gatttool and sudo dependence.
- None of the cheap BLE meters expose firmware version or error state; "instrument state" is limited to mode, range, hold and at best a battery bit. Only the UT61E+ client shows a battery flag as a first-class field.
- B35T's "cannot automatically recover a broken connection" plus 15-30 minute auto power off means unattended reading needs APO defeated and a reconnect loop in the bridge.
- US retail for these meters now sits well above Ali listing prices (OW18E USD 70-85 vs USD 43-46), so the budget answer depends on whether the owner will order from AliExpress and absorb duty/shipping.
- The XDM1041 showing from about USD 85-90 on Ali undercuts the USD 139 figure in the earlier plan; if that variant price is real after shipping and duty, it may beat a USD 100 home-built set on multimeter capability alone.

### Gaps
- No Amazon US prices could be read; all handheld prices are AliExpress listings or third-party articles.
- ZT-5B, BSIDE ZT-5BQ, Aneng V05B, ST207, AN999S, UT60BT, Victor 86B/86C, TP4000ZC, Mustool, Richmeters, Voltcraft BLE models, Pokit Meter/Pro: no reliable current price found.
- Auto-power-off defeat for OWON OW18/B35/B41 and UT61E+ not confirmed from a source.
- BLE update rate and reconnect behaviour of the OW18E specifically not measured in any source found.
- Whether OW18E accepts the B35-style remote button commands is not documented.
- Parkside PDM-300 hack, Fluke IR adapter DIY, UT60BT BLE protocol, Voltcraft VC BLE: not researched to a citable source.
- lygte-info has no review of the OW18E or AN9002; safety remarks for those rest on sibling models.

## 2. Meters with hackable firmware or UART pads (DTM0660 / DM1106EN / FS9922 / ES51922 / HY3131)

### Takeaway
The DTM0660 "enable RS232" mod is real but hit-and-miss per model: configuration lives in a 24C02 EEPROM (auto-power-off time at 0xFB, RS-232 enable bit in 0xFA), yet several meters lack the hardware or never emit data. It is a learning project, not a dependable phase-1 data link.

### Cited Findings
- DTM0660L meters named: ennoLogic eM860T, UNI-T UT139C, Velleman 4100, Tekpower TP40, Voltcraft VC-523; config and calibration sit in an external EEPROM; sigrok can read the stream once RS232 is enabled — [EEVblog thread](https://www.eevblog.com/forum/testgear/hacking-dtm0660l-based-multimeters/)
- Same thread: a VC-523 owner disabled auto power off via EEPROM successfully but got no UART data even with a pull-up; a Zeast282 owner could not enable UART — [EEVblog thread](https://www.eevblog.com/forum/testgear/hacking-dtm0660l-based-multimeters/)
- UT210E (DTM0660L): EEPROM address 0xFA is a bitmap including RS-232 enable (OEM 0xEF), 0xFB is minutes before auto power off (0 = disabled, default 15); RS-232 is enabled in the config but "the hardware just isn't present"; blindly writing 0xCE to 0xFA clobbers other bits; EEPROM is reachable on unpopulated header CZ1 and can be edited with a Bus Pirate over I2C; calibration data lives in the same EEPROM — [af3556/UT210E](https://github.com/af3556/UT210E)
- FS9922-DMM4 example (Twintex TM287): long-press REL enters RS232 mode, 2400 baud, 14-byte ASCII packets with range/AC-DC/unit flags, wired through a PS2701 optocoupler to a USB-TTL adapter — [vuquangtrong blog](https://vuquangtrong.github.io/blog/other/add-rs232-on-dmm)
- 121GW uses the HY3131 with a BLE112 module and takes firmware from microSD — [lygte-info 121GW](https://lygte-info.dk/review/DMMEEVBlog%20121GW%20UK.html)

### Inferences
- An optocoupler on the TX line is the right pattern for any meter serial tap: the meter's logic ground is at the COM-jack potential, so a direct UART to a Pi ties the bench ground to whatever the black probe touches.
- A more reliable variant of this idea is tapping the UART between the DMM chip and the BLE module inside an Aneng/OWON meter with an optocoupler, but no source confirming such a UART on the AN9002 was found.
- Verdict: **P2** as a teaching module (Bus Pirate + SOIC clip + EEPROM diff is a good agent exercise); **SKIP** as the phase-1 meter.

### Gaps
- No source found for an ESP32/nRF board retrofitted inside a specific cheap meter with published firmware.
- No open replacement firmware for a commercial handheld DMM was found other than the in-progress 2C53T project; 121GW community firmware status not verified.
- Exact TX pin and frame format for DTM0660 not captured from a primary source.

## 3. Open-hardware DIY multimeter and measurement front ends

### Takeaway
A ground-up DC "multimeter" for 12 V and 48 V systems is cheap and well supported in Rust: an INA228 (85 V, 20-bit, USD 6.50-15) on a Pico gives volts, amps, power, energy and charge. A full auto-ranging DMM with ohms, AC and protection is a multi-year project even for experienced builders and should not be the phase-1 meter.

### Cited Findings
- Adafruit INA228: USD 14.95, in stock, up to 85 VDC common mode high or low side, "Not for AC voltages", 15 milliohm 0.1% shunt, 10 A max, 20-bit, claimed better than 1% accuracy, I2C — [Adafruit 5832](https://www.adafruit.com/product/5832)
- Adafruit catalog 2026-11-25: INA237 (85 V, 16-bit) USD 9.95; INA238 USD 10.95; INA260 USD 9.95; INA3221 triple USD 10.95; ADS1115 USD 14.95; MCP3421 18-bit USD 5.95; NAU7802 24-bit USD 5.95; LM4040 2.048/4.096 V reference breakout USD 7.50; Pico USD 4.00; Pico 2 USD 6.25 (out of stock) — [Adafruit product API](https://www.adafruit.com/api/products)
- Ali medians 2026-11-25: INA228 module about USD 6.49; ADS1115 about USD 2.23; ADS1220 about USD 3.14; ACS758 hall module about USD 3.38 — [Ali INA228](https://www.aliexpress.com/w/wholesale-INA228-module.html), [Ali ADS1115](https://www.aliexpress.com/w/wholesale-ADS1115-module.html), [Ali ADS1220](https://www.aliexpress.com/w/wholesale-ADS1220-module.html), [Ali ACS758](https://www.aliexpress.com/w/wholesale-ACS758-current-sensor-module.html)
- M5Stack INA226 units (1 A and 10 A) USD 6.80; ADS1110 ADC unit USD 5.06 — [M5Stack feed](https://shop.m5stack.com/products.json)
- Rust drivers on crates.io (2026-11-25): `ina228` 0.2.0, `ina228-driver` 0.1.1 (no_std, sync + async), `ina229` 1.0.0, `ina226` 0.3.0, `ads1x1x` 0.3.0, `embedded-ads1220` 0.1.0, `mcp3x6x` 0.3.1 — [crates.io ina228](https://crates.io/crates/ina228), [ina228-driver](https://crates.io/crates/ina228-driver), [ads1x1x](https://crates.io/crates/ads1x1x), [embedded-ads1220](https://crates.io/crates/embedded-ads1220), [mcp3x6x](https://crates.io/crates/mcp3x6x)
- No crate found for ADS131M04, MCP3421, HLW8032, INA238 (2026-11-25) — [crates.io](https://crates.io/search?q=ads131m04)
- STM32 open-source multimeter (MartinD-CZ): ±60 V / ±6 V / ±600 mV / ±60 mV, DC or RMS; ±250 / ±2500 mA; simultaneous V and I; continuity with resistance; simple component test; frequency to 10 MHz; isolated USB serial port; firmware update over USB — [MartinD-CZ repo](https://github.com/MartinD-CZ/STM32F1-open-source-multimeter)
- ESP32 Wi-Fi multimeter (INA226): about 40 V full scale, 1638 mA / 78 mA ranges with 50 µA / 2.4 µA resolution, auto-range, 1 Hz-40 MHz counter, websocket control — [har-in-air/esp32_multi_meter](https://github.com/har-in-air/esp32_multi_meter)
- HydraMeter (John Duffy): RP2040-based open multimeter under a CERN licence; Rev 1 working November 2023 as an "MVP"; resistance-mode protection not functional in Rev 1; Rev 2 in progress in May 2025; front end uses 1 MΩ of series resistors plus MOV/GDT; makes no category-rating claims; MCP3464 considered as ADC — [hackaday.io HydraMeter](https://hackaday.io/project/176607-hydrameter/), [Hackaday 2024-12-25](https://hackaday.com/2024/12/25/open-source-multimeter-raises-the-bar-for-diy-tools)
- BugBuster: open hardware (ESP32-S3 + RP2040 HAT) with 4-ch 24-bit ADC for voltage/current/resistance, 4-ch DAC, logic analyzer, USB-PD, adjustable rails with current monitoring, and an MCP server exposing 59 tools with protections enforced at the tool layer — [Hackster](https://www.hackster.io/news/bugbuster-gives-ai-the-tools-to-debug-physical-hardware-f4fd22236ce4)

Build cost / difficulty / verdict

| Build | Parts cost | Measures | Effort | Honest safety claim | Verdict |
|---|---|---|---|---|---|
| INA228 + Pico (W) DC monitor | USD 10-20 | 0-85 V DC bus, shunt current, power, energy, charge | Low (module + I2C + Rust crate) | Extra-low-voltage DC only; non-isolated unless USB isolator or wireless; fused sense leads | **P1** |
| INA228 + external 75 mV shunt | + shunt | 100 A-class DC current | Low-medium (Kelvin wiring, calibration) | Permanently installed, fused taps; not a probing tool | **P1/P2** |
| ADS1220 / MCP3421 isolated DC voltmeter + ohms/continuity/diode | USD 15-30 | ±60 V DC, ohms by ratiometric current source, continuity, diode | Medium (divider, protection, calibration) | Bench low-voltage only | **P2** |
| Full auto-ranging DMM (HY3131 / MCP3464 class) | n/f | everything | High, multi-year (HydraMeter) | None beyond what is tested | SKIP |
| M5Stack INA226 / ADC units | USD 5-7 each | as chips | Very low | as above | P2 convenience |

### Inferences
- "Up to 85 V" on the INA228 covers a 16S LiFePO4 bank at full charge with margin, which is why it, not the 26 V INA219 or 36-40 V INA226, is the right chip for the 48 V use case.
- A hobby front end built from a PTC, MOV and clamp diodes can honestly be described as "survives accidental overvoltage on the bench up to the tested level"; it cannot be described with a CAT rating, because that requires creepage/clearance design, HRC fusing and transient testing that none of the surveyed DIY projects claim.
- Even commercial cheap meters fail this bar (see lygte teardowns in section 1), so a home build should be labelled for extra-low-voltage use only.
- MartinD-CZ's design (±60 V, isolated USB) is the closest existing template for a phase-2 "open DMM module".

### Gaps
- No verified build cost for HydraMeter or MartinD-CZ boards.
- PSLab v5/v6, EEZ DIB modules, "uMeter", OpenDMM-style and HY3131-specific open projects: not verified to a source.
- ADS1256 and HLW8032 module prices not captured (AliExpress fetch blocked).
- No independent accuracy-versus-reference data found for INA228 hobby builds.

## 4. DIY and ultra-cheap oscilloscopes / logic analyzers with open firmware

### Takeaway
A USD 4 Pico running sigrok-pico is a credible open logic analyzer (21 channels, up to 120 Msps) but only a token scope (3 channels, 500 ksps, about 7 bits, 0-3.3 V). Real analog bandwidth with a host API still means buying: Espotek Labrador at USD 29 is the cheapest open option, a Hantek 6022BE (sigrok-supported, 20 MHz) the cheapest "real" one.

### Cited Findings
- sigrok-pico: 21 digital channels (D2-D22) and 3 analog (ADC0-2) with mixed capture; USB CDC serial; libsigrok driver `raspberrypi_pico` merged to mainline September 2023; Pico 2 firmware builds provided — [pico-coder/sigrok-pico](https://github.com/pico-coder/sigrok-pico)
- Digital up to 120 Msps limited by PIO for captures that fit on-device (400K samples at 1-4 channels down to 50K at 15-21 channels), falling to 167-500 ksps with RLE when streaming; analog up to 500 ksps; analog "7-bit (128 divisions, ~20mV resolution)"; RP2040 ADC ENOB about 8 bits — [USER_GUIDE](https://github.com/pico-coder/sigrok-pico/blob/main/USER_GUIDE.md)
- "Voltages outside 0V-3.3V can damage the device"; ≥1 kΩ series resistors recommended — [USER_GUIDE](https://github.com/pico-coder/sigrok-pico/blob/main/USER_GUIDE.md)
- gusmanb LogicAnalyzer: 24 channels per Pico, up to 120 by daisy-chaining five, up to 100 MHz (400 Ms/s "blast" mode reported on newer firmware), level-shifter board design, terminal capture app and CLI, sigrok decoders supported — [gusmanb/logicanalyzer](https://github.com/gusmanb/logicanalyzer)
- Scoppy: Pico/Pico W/Pico 2 scope with Android app as the only client described; 2 MS/s analog (was 500 kS/s), 25 MS/s logic; 0-3.3 V unless a front end is added — [fhdm-dev/scoppy](https://github.com/fhdm-dev/scoppy), [Scoppy docs](https://oscilloscope.fhdm.xyz/)
- Espotek Labrador: USD 29 shipped; 2-ch scope sharing 750 ksps, ±20 V, 1 MΩ, AC/DC coupling, 8-bit (12-bit at 375 ksps single channel); 2-ch arbitrary generator 1 MSPS, 0.15-9.5 V; 4.5-12 V 0.75 W supply; 2-ch 3 MSPS logic analyzer; multimeter ±20 V; runs on Linux and Raspberry Pi; GPL v3 software, open hardware — [espotek.com](https://espotek.com/labrador/)
- Hantek 6022BE: sigrok-supported, 2 channels, 48 MS/s, 20 MHz; 6022BL adds logic — [sigrok](https://sigrok.org/wiki/Supported_hardware); one Ali listing at USD 54.99, median of matching listings about USD 83 (2026-11-25) — [Ali 6022BE](https://www.aliexpress.com/w/wholesale-Hantek-6022BE.html)
- Hantek HT201 20:1 attenuator about USD 10.88 on Ali — [Ali HT201](https://www.aliexpress.com/w/wholesale-HT201-attenuator-20-1.html)
- FX2 (CY7C68013A) boards: Ali median about USD 4.73 (2026-11-25) — [Ali FX2](https://www.aliexpress.com/w/wholesale-CY7C68013A-logic-analyzer-24MHz-8CH.html); Rust crate `fx2la` 0.1.0 "Driver for FX2-based fx2lafw logic analyzers with nusb" — [crates.io fx2la](https://crates.io/crates/fx2la)
- DSO138 with DLO-138 open firmware: two analog + two digital channels, serial dump of captured waveform, 2K depth, second analog channel needs a duplicated front end, fastest timebase 20 µs/div — [ardyesp/DLO-138](https://github.com/ardyesp/DLO-138); DSO138/DSO150 kits about USD 18-20 on Ali — [Ali DSO150](https://www.aliexpress.com/w/wholesale-DSO150-oscilloscope.html)
- FNIRSI DSO152/153/510 multi-model listing from USD 13.90 on Ali; DSO-510 USD 43.99 official — [Ali DSO152](https://www.aliexpress.com/w/wholesale-FNIRSI-DSO152-oscilloscope.html), [fnirsi.com feed](https://www.fnirsi.com/products.json)
- Glasgow revC USD 179-228 — [1BitSquared](https://1bitsquared.com/products/glasgow-c); Digilent Analog Discovery 2 USD 374.95 at Adafruit (backordered) — [Adafruit API](https://www.adafruit.com/api/products)
- Differential probes on Ali: 10 MHz 100x "gain split" probes USD 36-79; Hantek HT8050/HT8100 HV differential USD 141.19 — [Ali differential probe](https://www.aliexpress.com/w/wholesale-high-voltage-differential-probe-oscilloscope.html); EEVblog HVP70 listed at 305 — [eevblog.store feed](https://www.eevblog.store/products.json)

| Option | Cost | Real capability | Open? Rust/ARM64? | Verdict |
|---|---|---|---|---|
| Pico + sigrok-pico | USD 4 | 21 ch LA to 120 Msps; 3 ch 500 ksps 7-bit 0-3.3 V | Open; libsigrok; oxrok driver hardware-tested | **P1** |
| FX2 board + fx2lafw | USD 5 | 8 ch 24 MHz LA (sigrok classic) | Open fw; `fx2la` crate | **P1** spare |
| Espotek Labrador | USD 29 | ±20 V, 750 ksps scope + gen + PSU + LA | Open; Linux/RPi app | **P1 (USD 100 tier) / P2** |
| Hantek 6022BE | USD 55-83 Ali | 20 MHz, 48 MS/s, 2 ch | sigrok + open firmware | P2 |
| DSO138 + DLO-138 | USD 18-20 | about 200 kHz class, serial dump | Open fw | P2 learning kit |
| Scoppy | USD 4 + app | 2 MS/s | Android only | SKIP (phone in loop) |
| FNIRSI DSO152/TC3/2C53T | USD 14-106 | standalone | No data link (2C53T open fw early) | SKIP |
| Glasgow / AD2 | USD 179 / 375 | far more | Open (Glasgow) | SKIP for budget |

### Inferences
- For 12 V automotive work the Pico ADC needs a front end before it is useful: a divider or 20:1 attenuator for cranking/charging waveforms, AC coupling for ripple, and clamp protection; ignition primary and injector flyback spikes run to hundreds of volts and need a proper attenuator such as the HT201.
- For 48 V battery systems a ground-referenced USB scope shares ground with the Pi and anything else on USB; a differential probe or an isolated, battery-powered capture node is the safe pattern. The cheapest bought HV differential probe found is about USD 141, which is more than the rest of the phase-1 electrical set.
- A Rust embassy rewrite of the sigrok-pico device side (same serial protocol) is a tractable "own firmware" project because the host driver already exists in both libsigrok and oxrok.

### Gaps
- RP2350 32-channel analyzers, tomeko miniscope, ESP32 scope projects, WeAct STM32H7 scopes, Haasoscope, ThunderScope, ScopeFun, EEZ DIB, ADALM2000 price, Hantek 6022BE Amazon price: not verified.
- Analog bandwidth (as opposed to sample rate) for sigrok-pico and Labrador is not stated in the sources.
- Whether Scoppy's Pico protocol is documented for a headless client was not found.
- gusmanb's README records the Pico 2 "E9" GPIO erratum stopping that port at one point; current RP2350 status not confirmed.

## 5. DIY power monitors, programmable supplies and loads

### Takeaway
The cheapest agent-controlled supply is a USB-PD PPS sink board in front of a charger: PicoPD Pro (RP2040 + AP33772S, USD 18.20) with two Rust AP33772S crates available. For measurement, INA228 nodes cover volts/amps/energy; the Nordic PPK2 (USD 99.95) already has a Rust driver and an MCP server crate.

### Cited Findings
- PicoPD Pro: USD 18.20 (sale, 8 left), RP2040 + AP33772S, PD 3.1 fixed/PPS/AVS up to 30 V 5 A, PPS in 100 mV / 250 mA steps, built-in voltage/current/temperature reading, VBUS switch, Arduino and CircuitPython libraries, open source — [Tindie](https://www.tindie.com/products/centylab/picopd-pro-usb-ppsavs-dev-board)
- PocketPD: USD 70 assembled; open firmware (PlatformIO, RP2040); 20 mV / 50 mA PPS steps; live V/I/P and energy screens; no remote-control interface described in the README — [Tindie listing via PicoPD page](https://www.tindie.com/products/centylab/picopd-pro-usb-ppsavs-dev-board), [CentyLab/PocketPD](https://github.com/CentyLab/PocketPD)
- Rust: `ap33772s-rs` 0.1.9 (embedded-hal), `ap33772s-driver` 0.1.4, `husb238` 0.1.1 (sync + async), `fusb302b` 0.2.0 — [ap33772s-rs](https://crates.io/crates/ap33772s-rs), [ap33772s-driver](https://crates.io/crates/ap33772s-driver), [husb238](https://crates.io/crates/husb238), [fusb302b](https://crates.io/crates/fusb302b)
- Adafruit USB-C PD "dummy" breakout with I2C (HUSB238) USD 5.95 — [Adafruit API](https://www.adafruit.com/api/products); CH224K modules about USD 3.05 median on Ali — [Ali CH224K](https://www.aliexpress.com/w/wholesale-CH224K-PD-module.html)
- OpenDPS: open replacement firmware for the DPS5005; an active fork adds an ESPHome component — [kanflo/opendps](https://github.com/kanflo/opendps); DPS5005 modules USD 24-30 on Ali — [Ali DPS5005](https://www.aliexpress.com/w/wholesale-DPS5005-power-supply-module.html)
- Riden RD6006: Modbus over USB serial with documented registers — [Baldanos/rd6006](https://github.com/Baldanos/rd6006); sigrok lists RDTech DPS and RD series as supported — [sigrok](https://sigrok.org/wiki/Supported_hardware); UniSoft custom firmware is closed source — [EEVblog thread](https://eevblog.com/forum/testgear/custom-firmware-reference-document-for-riden-rd60xx-power-supplies)
- Alientek DP100: USB HID, Python scripts on Linux — [palzhj/pydp100](https://github.com/palzhj/pydp100); USD 56-61 on Ali — [Ali DP100](https://www.aliexpress.com/w/wholesale-Alientek-DP100-power-supply.html)
- FNIRSI DPS-150: USD 69.99-89.99; protocol documented (USB CDC, 9600 8N1, framed commands, set V/I, output on/off, push stream), Kaitai spec and Python library, verified on hardware 2026-03-29 — [fnirsi.com feed](https://www.fnirsi.com/products.json), [DPS-150 protocol](https://philipp-iot.github.io/fnirsi_protocol/)
- XY6020L modules from about USD 10 on Ali (median USD 22) — [Ali XY6020L](https://www.aliexpress.com/w/wholesale-XY6020L.html)
- Atorch DL24 load: ESPHome BLE component for DL24/DL24P and others; sigrok supports the DL24MP-150W; USD 22-31 on Ali — [syssi/esphome-atorch-dl24](https://github.com/syssi/esphome-atorch-dl24), [sigrok](https://sigrok.org/wiki/Supported_hardware), [Ali DL24](https://www.aliexpress.com/w/wholesale-DL24-electronic-load-150W.html)
- Nordic PPK2 USD 99.95 — [Adafruit API](https://www.adafruit.com/api/products); Rust `ppk2` 0.1.3 and `mcp-nordic-ppk2` 0.2.0 ("MCP server for the Nordic Power Profiler Kit II") — [ppk2](https://crates.io/crates/ppk2), [mcp-nordic-ppk2](https://crates.io/crates/mcp-nordic-ppk2)
- CurrentRanger USD 145 base; nA/µA/mA ranges at 1 mV per unit, auto-ranging, SAMD21 open firmware, logging via Bluetooth serial or USB "use USB isolator" — [LowPowerLab shop](https://lowpowerlab.com/shop/product/152), [CurrentRanger repo](https://github.com/LowPowerLab/CurrentRanger)
- EEVblog µCurrent Gold listed at 308, out of stock — [eevblog.store feed](https://www.eevblog.store/products.json)

### Inferences
- PD-PPS from a wall charger gives roughly 3.3-21 V programmable output with the charger providing mains isolation; that is the best "supply the agent controls" per dollar, and it is a clean embassy-on-RP2040 project.
- Verdicts: PicoPD Pro or bare AP33772S/HUSB238 board **P1** (USD 100 tier) or **P2**; INA228 node **P1**; DL24 load **P2**; DP100 / DPS-150 / RD6006 **P2** (bought, documented protocols); OpenDPS **P2** (best open-firmware supply); PPK2 **P2**; CurrentRanger and µCurrent **SKIP** on cost; UniSoft **SKIP** (closed).
- Stored gear notes say the owner already has a Pine64 PinePower bench supply, which lowers the priority of any bought bench supply (source: user memory, not verified here).

### Gaps
- RD6006 current price, MiniWare MDP-P906 protocol, Joulescope alternatives, open-source DIY e-load designs, PD Micro / "PDTricker": not verified.
- No Rust crate found for DPS/RD Modbus, DP100 or DPS-150 other than oxrok's untested rdtech ports.
- Whether PocketPD exposes any serial control is not stated.

## 6. Signal sources and component testers

### Takeaway
An AD9833 (about USD 8-11) or Si5351 (about USD 1 Ali, USD 7.95 Adafruit) on a Pico is a cheap, fully open signal source with Rust drivers. The AVR "transistor tester" clones (GM328A about USD 8-15) run open firmware that can be rebuilt with UART output, which makes them the one component tester worth owning for an agent bench.

### Cited Findings
- AD9833 modules about USD 7.98-11.50 on Ali (2026-11-25) — [Ali AD9833](https://www.aliexpress.com/w/wholesale-AD9833-module.html); Rust `ad983x` 1.0.0 — [crates.io ad983x](https://crates.io/crates/ad983x)
- Si5351A breakout USD 7.95 at Adafruit; about USD 1 on Ali; Rust `si5351` 0.2.0 — [Adafruit API](https://www.adafruit.com/api/products), [Ali Si5351](https://www.aliexpress.com/w/wholesale-Si5351-clock-generator-module.html), [crates.io si5351](https://crates.io/crates/si5351)
- FY6900 from USD 64.49 on Ali; sigrok supports the Joy-IT JDS6600 but FeelTech FY is not in the list as fetched — [Ali FY6900](https://www.aliexpress.com/w/wholesale-FY6900-signal-generator.html), [sigrok](https://sigrok.org/wiki/Supported_hardware)
- Transistortester is OSHW; "all the inexpensive Transistortesters sold on your favourite shopping platform are based on this project and can be updated with the original OSHW firmware versions" — [madires/Transistortester-Warehouse](https://github.com/madires/Transistortester-Warehouse)
- PC tools need m-firmware v1.33m or later with UART input and output enabled (and `UI_SERIAL_COMMANDS`) plus a USB-UART adapter — [EEVblog thread](https://www.eevblog.com/forum/testgear/transistor-tester-windows-pc-software/)
- GM328A listings about USD 7.81-15 on Ali — [Ali GM328A](https://www.aliexpress.com/w/wholesale-GM328A-transistor-tester.html); FNIRSI LCR-P1 USD 36.80, no data link found — [fnirsi.com feed](https://www.fnirsi.com/products.json)
- References on Ali (median of matching listings): AD584 about USD 19, LM399 about USD 32 — [Ali AD584](https://www.aliexpress.com/w/wholesale-AD584-voltage-reference.html), [Ali LM399](https://www.aliexpress.com/w/wholesale-LM399-voltage-reference.html); Adafruit LM4040 breakout USD 7.50 — [Adafruit API](https://www.adafruit.com/api/products)

### Inferences
- GM328A + m-firmware with serial commands: **P1 (USD 100 tier) / P2**, about USD 10-15 plus a USB-UART adapter and an AVR programmer; AD9833/Si5351 source: **P2** (or P1 at near-zero cost); FY6900: **P2/SKIP** at USD 64+; LCR-P1 and DSO-TC3: **SKIP** (no link).
- A calibration bus (one LM4040 or AD584 reference plus a few 0.1% resistors on a relay or analog mux) lets the agent self-check every DIY voltmeter and ohmmeter channel and detect drift, which is worth more than extra digits.

### Gaps
- UART pin, baud rate and output format for the m-firmware on specific clones not captured.
- DIY ESR meters and curve tracers, FY3200 protocol, 0.01% resistor pricing: not researched to a source.
- AD584 / LM399 medians mix bare chips, kits and cased units.

## 7. All-in-one "lab on a board" options and agent-friendliness

### Takeaway
A Bus Pirate 5 (USD 51.50 from the US distributor) is the most agent-friendly single purchase: open firmware and a FlatBuffers binary interface that returns per-pin millivolt readings, PSU voltage/current, firmware version and error strings. It works as a low-voltage multimeter-plus-supply-plus-protocol tool only inside 0-5 V; it cannot replace a multimeter for 12 V, 48 V, resistance or AC.

### Cited Findings
- Prices 2026-11-25: Bus Pirate 5 USD 51.50, Bus Pirate 5 Bundle USD 72.85, Bus Pirate 6 USD 86.00, probe cable set USD 10.95 at Blinkinlabs (USA) — [Blinkinlabs feed](https://shop.blinkinlabs.com/products.json); Lab401 lists Bus Pirate 5 at 59 / 89 (store currency not confirmed), out of stock — [Lab401 feed](https://lab401.com/products.json); Adafruit's feed shows the Bus Pirate 5 base kit at 129.20 with zero stock, which looks anomalous — [Adafruit API](https://www.adafruit.com/api/products)
- Hardware: 1-5 V programmable supply with current sense and resettable fuse (page gives both 0-500 mA and "up to 300 mA"), buffered IO 1.2-5 V, analog voltage measurement on all IO pins, 10K pull-ups, logic analyzer support — [Bus Pirate hardware docs](https://docs.buspirate.com/docs/overview/hardware/)
- BPIO2: FlatBuffers over the second USB serial port, COBS framed; status returns mV per IO pin, PSU set/measured voltage and current, over-current flag, hardware and firmware version, git hash, build date; requests for I2C/SPI/UART/1-Wire, pull-ups, PSU enable and current limit, self-test, reset; every response carries an error string; Python demo library and "a crate is listed in the BPIO2 repository" for Rust — [BPIO2 docs](https://docs.buspirate.com/docs/binmode-reference/protocol-bpio2/)
- ESP32 Bus Pirate: open firmware for common ESP32 boards with serial and Wi-Fi web CLI; I2C/SPI/UART/1-Wire/DIO and radio modes; legacy branch deprecated in favour of a "pioarduino" branch — [geo-tp/ESP32-Bus-Pirate](https://github.com/geo-tp/ESP32-Bus-Pirate)
- Jumperless V5: USD 349; ten GPIO, four programmable ±8 V supplies, voltage/current/resistance measurement channels, Python module, open hardware and software — [Crowd Supply](https://www.crowdsupply.com/architeuthis-flux/jumperless-v5)
- Glasgow revC USD 179-228 — [1BitSquared](https://1bitsquared.com/products/glasgow-c); Flipper Zero from 149.17 at Lab401 (currency not confirmed) — [Lab401 feed](https://lab401.com/products.json)
- Labrador (USD 29) also qualifies as an all-in-one: scope, generator, supply, logic, ±20 V voltmeter — [espotek.com](https://espotek.com/labrador/)

### Inferences
- Bus Pirate 5: **P1 at the USD 100 tier** as the low-voltage electronics instrument; a stock Pico running the open ESP32/Pico-class alternatives is the USD 60 substitute for protocol work.
- Jumperless V5 matches the owner's "breadboard the agent can read" wish but at USD 349 is a **P2+** item; Glasgow **SKIP/P3**; Flipper **SKIP** for measurement.

### Gaps
- Bus Pirate ADC resolution and absolute accuracy not stated in the docs fetched; 5XL price not found.
- PSLab price and current availability, Red Pitaya clones, Sipeed tools, M5Stack Vmeter/Ameter units (not in the first 250 feed items): not verified.
- Lab401 and EEVblog store currencies unconfirmed.

## 8. Isolation and safety building blocks

### Takeaway
Isolation parts are cheap (USB isolator boards about USD 6 on Ali, USD 34.95 from Adafruit; I2C isolator USD 8.95; B0505S isolated DC-DC about USD 1), so every home-built front end can and should float. Mains voltage sensing and anything that probes a battery bank's bus bars should be bought, isolated and permanently installed, not home-made.

### Cited Findings
- Adafruit USB isolator (100 mA, low/full speed) USD 34.95; ISO1540 I2C isolator USD 8.95 — [Adafruit API](https://www.adafruit.com/api/products)
- Ali medians 2026-11-25: ADuM3160 USB isolator boards about USD 5.61; B0505S 1 W isolated DC-DC about USD 0.81-1.04 (1000 VDC isolation per listing) — [Ali ADuM3160](https://www.aliexpress.com/w/wholesale-ADUM3160-USB-isolator.html), [Ali B0505S](https://www.aliexpress.com/w/wholesale-B0505S-isolated-DC-DC.html)
- BL0942 mains energy-metering modules with UART output about USD 4.19-4.29 on Ali — [Ali BL0942](https://www.aliexpress.com/w/wholesale-BL0942-module.html); `embedded-devices` crate matches a BL0942 search — [crates.io](https://crates.io/search?q=bl0942)
- OpenEnergyMonitor measures mains voltage through an AC-AC adapter so that the measurement is made "safely (requiring no high voltage work)", the transformer isolating the circuit from mains; signal is divided and biased to mid-rail for the ADC — [OpenEnergyMonitor docs](https://docs.openenergymonitor.org/electricity-monitoring/voltage-sensing/measuring-voltage-with-an-acac-power-adapter.html)
- Cheap commercial meters themselves miss their printed ratings: UT61E MOVs absent and 240 V fuses; B41T+ no MOVs; B35T 250 V fuses — [lygte UT61E](https://lygte-info.dk/review/DMMUNI-T%20UT61E%20UK.html), [lygte B41T+](https://lygte-info.dk/review/DMMOwon%20B41T%20UK.html), [lygte B35T](https://lygte-info.dk/review/DMMOwon%20B35T%20UK.html)
- Meters whose category ratings are treated as credible start around the Brymen BM235 (about USD 180 per the article; 127 on the EEVblog store feed) — [computingforgeeks](https://computingforgeeks.com/best-multimeter-electronics/), [eevblog.store feed](https://www.eevblog.store/products.json)
- EEVblog store fuse packs (HRC 1000 V) at 9 each; Brymen silicone probes 13 — [eevblog.store feed](https://www.eevblog.store/products.json)

### Inferences
- Never DIY: direct resistive-divider mains sensing on a non-isolated MCU; home-made HV differential probes; un-fused sense leads on a 48 V LiFePO4 bank (the hazard is fault current and arc, not shock voltage); any probe-style instrument marketed with a CAT rating; opening a clamp-on CT secondary under load.
- Safe patterns: plug-in AC-AC adapter or a sealed, certified energy-monitor module for 120 V; split-core CTs for AC current; hall clamps or permanently bolted shunts with fused Kelvin taps for battery current; INA228 nodes that are battery/Wi-Fi powered or behind a USB isolator; an attenuator plus differential probe for scope work.
- The cheap BLE handhelds are inherently isolated from the host (battery + radio), which is a real safety advantage over any USB-tethered DIY front end, even though their CAT markings are not credible for mains distribution work.
- A 100 mA USB isolator will not power a Pico W plus sensors reliably at high speed; an isolated DC-DC (B0505S) plus ISO1540 on the I2C side is the cheaper isolation boundary for an INA228 node.

### Gaps
- AMC1301/AMC1311, ACS37800, HLW8032, ATM90E32 board prices and ISOUSB211 boards: not verified.
- No citable primary source captured for IEC 61010 touch-voltage thresholds or CAT definitions; the safety rules above are the researcher's engineering judgement.
- Fused CAT-rated test lead set prices in USD not captured.

## 9. Prior art: agent- or script-driven benches from cheap parts

### Takeaway
LLM-driven instrument control over MCP is now common for SCPI scopes and Digilent hardware, and two open projects (BugBuster, oxrok) already target cheap or open hardware; none found pairs an agent with a sub-USD-50 BLE multimeter, so that bridge is open ground.

### Cited Findings
- oxrok ships an MCP server with tools for scan/open/describe, capture start/status/stop, measure, find, decoder stacks and PNG capture views; a Pico running sigrok-pico and an fx2lafw clone are among the few devices it has run on real hardware — [oxrok](https://crates.io/crates/oxrok)
- BugBuster: open hardware debug platform whose MCP server exposes 59 tools, with current limits, e-fuse protection, voltage confirmation and routing restrictions enforced at the tool layer — [Hackster](https://www.hackster.io/news/bugbuster-gives-ai-the-tools-to-debug-physical-hardware-f4fd22236ce4)
- Analog Discovery 2 MCP server (Python FastMCP, 2026-02-02): lessons were that the model confused amplitude with peak-to-peak (3.3 V request produced 6.6 Vpp), large captures should go to files rather than JSON, and tool output needs independent verification; no safety interlocks discussed — [Altium](https://resources.altium.com/p/building-mcp-server-analog-discovery)
- `mcp-nordic-ppk2` crate: Rust MCP server for the PPK2 — [crates.io](https://crates.io/crates/mcp-nordic-ppk2)
- Home Assistant side: ESPHome BLE component for Atorch loads/meters — [syssi/esphome-atorch-dl24](https://github.com/syssi/esphome-atorch-dl24); an ESPHome config for the OWON B35T on M5Stack exists in a public config repo (content not reviewed) — [commander1024 config](https://git.commander1024.de/Commander1024/homeassistant-config/src/branch/main/esphome/archive/owon-b35t-m5stack-core2.yaml)

### Inferences
- The AD2 lesson (units ambiguity) argues for MCP tool schemas that carry explicit units and for a calibration bus the agent can use to sanity-check itself.
- BugBuster's tool-layer interlocks are the nearest published analogue to a physical-safety layer for an agent bench.

### Gaps
- No published accuracy comparison of a cheap BLE meter or INA228 build against a reference meter in an automated bench was found.
- Other MCP instrument servers surfaced by search (Saleae, Siglent, Rigol, lablink-mcp, usbtmc-lite-mcp) were not opened.

## 10. Recommended phase-1 sets, what they cannot do versus the XDM1041, and build-versus-buy

### Takeaway
About USD 60 buys a real BLE multimeter with a Rust client, an 85 V INA228 DC meter node, a Pico logic analyzer and a USB isolator; about USD 100 upgrades the meter to 20,000 counts and adds a Bus Pirate 5 or an agent-controlled PD supply and component tester. Both sets give up resolution, host-commanded ranging and mains-powered always-on operation relative to the XDM1041, and the XDM1041 itself appears on AliExpress from about USD 85-90.

### Cited Findings
- Component prices used below are those cited in sections 1-8 (Ali listings and store feeds, 2026-11-25).
- XDM1041: 55,000 counts, 4.5 digit, true RMS bench meter (listing title) — [Ali XDM1041](https://www.aliexpress.com/w/wholesale-OWON-XDM1041-bench-multimeter.html); SCPI over its serial link, with a known firmware quirk — [TheHWcave/OWON-XDM1041](https://github.com/TheHWcave/OWON-XDM1041)

### Inferences

(a) Recommended sets (researcher's synthesis)

Set A, about USD 60 (AliExpress sourcing)

| Item | Role | Approx. cost |
|---|---|---|
| ZOYI ZT-300AB (or AN9002-family equivalent) | Real handheld DMM: V AC/DC, A, ohms, continuity, diode, capacitance, temperature; BLE to a Rust bridge | USD 23-28 |
| INA228 module + Raspberry Pi Pico | Open-firmware DC volt/amp/power/energy node to 85 V (12 V and 48 V systems via fused taps) | USD 6.50-15 + 4 |
| Second Pico with sigrok-pico | 21-ch logic analyzer, 3-ch slow scope (0-3.3 V) | USD 4 |
| FX2 board | Spare 8-ch sigrok logic analyzer | about USD 5 |
| ADuM3160 USB isolator + B0505S | Float the INA228 node | about USD 7 |
| Inline fuses, leads, 75 mV shunt | Safe taps | about USD 5-10 (not priced from a source) |
| Total | | about USD 55-75 |

Set B, about USD 100

| Item | Role | Approx. cost |
|---|---|---|
| OWON OW18E/OW18B (or UT61E+ if the Ali price is real) | 20,000-22,000-count DMM; numeric protocol (OWON) or full state over USB HID (UT61E+) | USD 43-46 |
| INA228 + Pico, sigrok-pico Pico, isolator | As Set A | about USD 22-30 |
| One of: Bus Pirate 5 | Low-voltage per-pin voltmeter, 1-5 V supply with current readback, I2C/SPI/UART, rich status over BPIO2 | USD 51.50 (pushes total to about USD 120) |
| or: PicoPD Pro + GM328A + AD9833 | Agent-controlled 3.3-21 V PPS supply, component tester with serial firmware, signal source | USD 18.20 + 8-15 + 8-11 |
| Total | | about USD 100-120 |

Alternative worth pricing before committing: XDM1041 from AliExpress at about USD 85-90 plus Set A's INA228 and Pico parts.

(b) What each set cannot do versus the OWON XDM1041 (USD 139 US)

| Capability | XDM1041 | Set A | Set B |
|---|---|---|---|
| Resolution | 55,000 counts | 6000 counts | 20,000-22,000 counts |
| Host sets function and range | Yes, SCPI | No (BLE is a display mirror; a human turns the dial) | No on OW18E as documented; UT61E+ and OWON B35/B41 report or accept more |
| Always-on unattended reading | Mains powered | Battery; APO must be defeated at each power-on; BLE reconnect logic needed | Same |
| Reading rate over link | Not verified here | about 2.6/s | about 2/s (OWON) |
| Instrument state (firmware, errors) | SCPI identity; buggy "OK" replies on one firmware | Mode, units, Hold, Auto, battery bit only | OWON: function/scale only; UT61E+: mode, range, hold, battery, HV warning |
| Wired, deterministic link | Yes | No (BLE) for DMM; yes for INA228/Pico | UT61E+ yes; OWON no |
| Galvanic isolation from host | Not verified | Inherent (battery + radio) | Inherent (BLE) |
| Open, reflashable firmware | No | DMM no; INA228 node, Picos yes | DMM no; Bus Pirate, Picos, PicoPD yes |
| Things the sets add that the XDM1041 lacks | n/a | Continuous 85 V DC energy/charge logging, logic analysis, portability | Plus programmable supply with current readback, protocol tool, component tester |

(c) Worth building from scratch as open-firmware modules versus false economy

Worth building (best learning and agent-tweakability per dollar)
- INA228 DC power/energy node in Rust on embassy (USD 10-20); extend to external shunts for 100 A.
- BLE-meter bridge: Rust decoder for OWON 6-byte and Zotek XOR/segment packets feeding the MCP server.
- Pico logic analyzer/slow scope, starting from sigrok-pico and optionally re-implementing the device firmware in Rust.
- USB-PD PPS supply controller on RP2040 + AP33772S (Rust crates exist).
- AD9833 / Si5351 signal source.
- Calibration bus: LM4040 or AD584 reference, 0.1% resistors, mux, so the agent can self-test every channel.
- Component tester: flash the open m-firmware with serial commands into a GM328A.
- Phase 2: isolated ±60 V DC voltmeter/ohms/continuity module on an ADS1220 or MCP3421, modelled on the MartinD-CZ design.

False economy (buy, or do not attempt)
- A full auto-ranging handheld-class DMM with protection (HydraMeter shows the timescale).
- Any mains-connected front end; buy an isolated metering module or use an AC-AC adapter and CTs.
- HV differential probes (cheapest bought about USD 141).
- A 20 MHz+ oscilloscope front end; buy a Hantek 6022BE or Labrador.
- nA-class current measurement (CurrentRanger USD 145, PPK2 USD 99.95 already have open tooling).
- A bench supply from scratch (PD-PPS boards, DPS5005 + OpenDPS, DP100 and DPS-150 are cheap and documented).
- EEPROM/UART mods as the primary meter data link (inconsistent across models).

### Gaps
- Set totals are sums of indicative listing prices; shipping, duty and Amazon-sourced equivalents are not included and could add 30-80% for US delivery (OW18E is USD 70-85 at US retail versus USD 43-46 on Ali).
- XDM1041 accuracy, reading rate, CAT rating and isolation of its USB/serial port were not verified from a primary source, so those cells are marked not verified.
- No set was assembled or tested; BLE reliability, reconnect behaviour and actual accuracy against a reference remain unmeasured.
