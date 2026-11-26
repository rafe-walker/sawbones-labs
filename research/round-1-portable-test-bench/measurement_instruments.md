# Measurement instruments for an agent-operated portable test bench (state: 25 Nov 2026)

Reading guide for the report writer:
- "Verified" = fetched and read this session, with link. "Lead" = a repo/page whose existence and title were seen in search results but whose content was not opened. "Estimate" = price or fact from the researcher's prior knowledge, NOT checked this session; treat as needing a price check before purchase.
- Amazon pages could not be fetched directly; Amazon prices come from trackers (SpecPicks, Klarna) where available.
- All prices USD unless marked EUR.

---

## Recommended instrument set at two tiers (objective)

### Takeaway
A workable agent-readable bench fits the USD 200-350 measurement share if it is built around one SCPI bench DMM (OWON XDM1041/XDM1241, USD 139 verified), sigrok-class logic capture (fx2lafw + Pico), and permanently wired, isolated bus sensors (INA228 on a shunt for DC, PZEM-004T for AC) instead of probe-style instruments on hazardous circuits. Thermal imaging (Topdon TC001, about USD 199) and a bench scope (Rigol DHO804 USD 459, Analog Discovery 3 USD 379) do not fit inside the measurement share and belong to a stretch list.

### Cited Findings
Verified prices and facts used in the tiers:
- OWON XDM1041: USD 139.00 on Amazon (ASIN B0CYYZXWY9), tracker timestamp 2026-11-25 11:18 UTC; 55,000 counts, DCV accuracy "up to 0.05%", up to 65 readings/s, SCPI — [SpecPicks](https://specpicks.com/product/B0CYYZXWY9)
- SparkFun INA228 Qwiic breakout: USD 14.95, in stock, 0-85 V bus sense, 20-bit delta-sigma ADC, onboard shunt good for 10 A, pads for an external shunt, four I2C addresses — [SparkFun](https://www.sparkfun.com/sparkfun-current-sensor-ina228-qwiic.html)
- Shelly Pro EM-50: USD 95.99 sale (119.99 regular), 100-260 V AC supply, two CT channels 0-50 A, local RPC/JSON, MQTT, WebSocket, Modbus TCP, mJS scripting, "UL-Certified" badge — [Shelly US](https://us.shelly.com/products/shelly-pro-em-50)
- Topdon TC001 (256x192): USD 199.00 (JB Tools) to 207.89 (Tractor Supply) — [Klarna US](https://www.klarna.com/us/shopping/pl/cl345/3402637283/Power-Tools/Topdon-TOPDON-TC001-Portable-Thermal-Imaging-Camera/) (capture date not shown on page)
- Glasgow Interface Explorer revC: USD 179.00, 8 left in stock at 1BitSquared — [1BitSquared](https://1bitsquared.com/products/glasgow-c)
- Nordic PPK2: USD 91.42 at Future Electronics, but 0 stock there — [Future Electronics](https://www.futureelectronics.com/p/development-tools--analog-power/nrf-ppk2-nordic-semiconductor-1205212)
- Digilent Analog Discovery 3: USD 379.00 — [Digilent](https://digilent.com/shop/analog-discovery-3/)
- Rigol DHO804: USD 459.00, 160 in stock (DigiKey Marketplace), input rated CAT I 300 V — [DigiKey](https://www.digikey.com/en/products/detail/rigol-technologies/DHO804/21421243)
- DMMCheck Plus Rev 8: EUR 269.00 incl. VAT at Welectron (EU seller) — [Welectron](https://www.welectron.com/DMMCheck-Plus-Rev-8-Multimeter-Calibration-Reference)
- Micsig DP10007 differential probe: USD 289.99 list at SainSmart, marked discontinued/sold out there — [SainSmart](https://www.sainsmart.com/products/micsig-dp10007-high-voltage-differential-probe-kit)

### Inferences
Proposed tiers (researcher's synthesis; "est." prices are unverified and need a check).

**Tier 1 "Core" — about USD 230-250**

| # | Item | Role | Link to computer | Price |
|---|---|---|---|---|
| 1 | OWON XDM1241 (battery/USB variant) or XDM1041 | General DMM, readable state via `*IDN?`, `FUNC?`, `RANGE1?`, `RATE?` | USB-serial (CH340), 115200 8N1, flat SCPI | USD 139 verified for XDM1041; XDM1241 price not verified |
| 2 | fx2lafw 8-ch 24 MHz analyzer | Digital capture + protocol decode through sigrok | USB, sigrok `fx2lafw` | est. USD 10-15 |
| 3 | Raspberry Pi Pico / Pico 2 with sigrok-pico | Second analyzer with a few low-speed analog channels, firmware fully open | USB CDC, sigrok `raspberrypi-pico` | est. USD 5-8 |
| 4 | 2x INA228 breakout + external shunt (e.g. 100 A/75 mV) | 12 V and 48 V bus voltage/current/energy, 85 V rated | I2C (behind an I2C isolator) | USD 14.95 each verified; shunt est. USD 10-20 |
| 5 | PZEM-004T v3 with 100 A CT | 120 V AC V/I/P/PF/frequency/energy | Modbus RTU over UART | est. USD 15-20 |
| 6 | AD584 reference module + 0.1 % resistor set + small relay board | Calibration bus ground truth | GPIO-driven relays | est. USD 25-35 total |
| 7 | Full-speed USB isolator (ADuM3160/4160 class) | Break ground between Pi and any instrument touching a power system | USB | est. USD 10-15 |
| 8 | Pinecil (already owned; V2 only) | BLE telemetry + setpoint | BLE GATT | USD 0 |

**Tier 2 "Full" — Tier 1 plus about USD 100-150 (total about USD 350)**

| # | Item | Role | Link | Price |
|---|---|---|---|---|
| 9 | Victron SmartShunt 500 A (or Junctek KG-F as the cheap alternative) | Battery bank current/SOC with a published wired protocol | VE.Direct serial + BLE advertisements (Victron); RS485 (Junctek) | est. USD 100-130 (Victron), est. USD 45-60 (Junctek) |
| 10 | Riden RD6006 (or RK6006) module | Programmable source for cross-checks; Modbus; sigrok + Rust crate | USB-serial Modbus RTU | est. USD 60-80 |
| 11 | MLX90640 I2C thermal array (32x24) | Cheap agent-readable thermal sanity sensor | I2C | est. USD 45-75 |

**Stretch list (outside the measurement share)**: Topdon TC001 USD 199 (verified), Glasgow revC USD 179 (verified), ChargerLAB KM003C (est. USD 90-110; has a mainline Linux kernel driver), Nordic PPK2 about USD 91 (verified price, stock varies), CAT-rated handheld with a data link for the human operator (Brymen BM869s or EEVblog 121GW, prices not verified), Analog Discovery 3 USD 379, Rigol DHO804 USD 459.

Design principle behind the tiers: instruments that plug into the Pi by USB are restricted to low-energy, low-voltage work; anything on the 48 V bank or 120 V mains is a permanently installed, fused, isolated sensor that the agent only reads.

### Gaps
- No verified November 2026 price for: OWON XDM1241, fx2lafw clones, Pico 2, PZEM-004T/017, AD584 modules, USB isolator boards, Victron SmartShunt, Junctek monitors, Riden RD6006, MLX90640 breakouts, KM003C, UT61E+, 121GW, BM869s, DSLogic, Hantek 6022BE/BL. All "est." figures above come from the researcher's prior knowledge.
- Tariff/import-duty effects on AliExpress-sourced items in late 2026 were not checked.

---

## Multimeters with open data links

### Takeaway
The OWON XDM1041/XDM1241 is the only low-cost meter found that exposes identity, firmware version, function, range and rate over a plain text serial protocol, which makes it the best fit for agent self-diagnosis; its safety category is weak, so it is a bench meter, not a mains/battery-bank meter. For hazardous circuits the sigrok-supported CAT-rated handhelds are Brymen BM869s (CAT IV 1000 V, optical USB HID cable) and EEVblog 121GW (CAT III 600 V, BLE with published packet format, open schematic).

### Cited Findings
OWON bench meters (SCPI over USB serial)
- XDM1241 enumerates through a CH340 USB-serial chip at 115200 8N1; `*IDN?` returned `OWON,XDM1241,24412417,V4.3.0,3` (maker, model, serial, firmware) on a unit with firmware V4.3.0 — [sigrok-mcp-server device notes](https://kenosinc.github.io/sigrok-mcp-server/devices/owon-xdm1241)
- Working queries on that firmware: `MEAS?`/`MEAS1?` (main display), `MEAS2?` (sub display, `NONe` when off), `FUNC?`, `RANGE1?`, `AUTO1?`, `RATE?` (S/M/F) — [same](https://kenosinc.github.io/sigrok-mcp-server/devices/owon-xdm1241)
- Quirks: the command set is flat, so hierarchical forms such as `MEAS:VOLT:DC?` time out; `SYST:ERR?`, `CONF?` do not work; `*RST` gives no reply; set commands answer `OK`/`nOK` on newer firmware — [same](https://kenosinc.github.io/sigrok-mcp-server/devices/owon-xdm1241)
- sigrok's `scpi-dmm` driver does not recognise the XDM1241, so that project talks to it with a raw serial query tool — [same](https://kenosinc.github.io/sigrok-mcp-server/devices/owon-xdm1241)
- sigrok's hardware list shows Owon XDM2041 only as work-in-progress (RS232); XDM1041/XDM1241 were not found in the list — [sigrok Supported hardware](https://sigrok.org/wiki/Supported_hardware)
- XDM1041 ranges: 1000 V DC, 750 V AC, 10 A, 50 MΩ; USB only (XDM1241 adds RS232 per this seller); 200 x 86.5 x 64 mm, 450 g; EUR 90 ex VAT; listed there as 230 V mains powered; a buyer review on the page says it is "only CAT I/II rated" — [Eleshop](https://eleshop.eu/owon-xdm1041-multimeter.html)
- Conflict: the Amazon listing data describes the XDM1041 as "battery powered" — [SpecPicks](https://specpicks.com/product/B0CYYZXWY9); contradicted by the mains-powered description at [Eleshop](https://eleshop.eu/owon-xdm1041-multimeter.html)
- Python drivers exist: TheHWcave/OWON-XDM1041 (lead) — [relatedrepos](https://relatedrepos.com/gh/TheHWcave/OWON-XDM1041); ElDuderino/XDM1041Python (lead) — [GitHub mirror](https://github.meowingcats01.workers.dev/ElDuderino/XDM1041Python); `xdm1000` on PyPI (lead) — [PyPI](https://pypi.org/project/xdm1000/); OWON's XDM1000 programming manual PDF (lead) — [Eleshop PDF](https://static.eleshop.nl/mage/media/downloads/XDM1000_Digital_Multimeter_Programming_Manual.pdf)

UNI-T
- UT61E+ protocol: the meter sits behind a CP2110 HID-to-UART bridge that must be configured for 9600 8N1 via feature reports; host sends the 7 bytes `06 ab cd 03 5e 01 d9`, meter answers with a 19-byte frame (mode, range, digits, flags). Go library `mwuertinger/ut61ep`, GPL-2.0, uses hidapi — [GitHub](https://github.com/mwuertinger/ut61ep)
- Python logger for UT61E+ (lead) — [kevontheweb/ut61e_plus_logger](https://gh.r-mj.com/kevontheweb/ut61e_plus_logger); UT8803E reader (lead) — [philpagel/ut8803e](https://github.com/philpagel/ut8803e)
- UT61E+ is a 22,000-count meter; the older UT61 series page advertises CAT IV 600 V with CE/GS/cETLus marks for the non-plus models — [UNI-T](https://meters.uni-trend.com/product/ut61-series/)
- sigrok lists the older UT61B/C/D/E, UT60E, UT71C and UT181A as supported; UT61E+, UT161, UT8804 and UT8805 were not found in the list — [sigrok Supported hardware](https://sigrok.org/wiki/Supported_hardware)

Brymen
- sigrok supports the BM869 through `serial-dmm` with the BU-86X infrared-to-USB (HID) cable, and states the BM869s uses the same cable and protocol; safety category listed as CAT IV 1000 V; 50,000 counts (500,000 on DCV); vendor publishes the packet layout as PDFs — [sigrok Brymen BM869](https://www.sigrok.org/wiki/Brymen_BM869)
- Also in sigrok's supported list: Brymen BM257/BM257s, BM525s, BM829s, BM859s, BM857 — [sigrok Supported hardware](https://sigrok.org/wiki/Supported_hardware)

EEVblog 121GW
- CAT III 600 V with independent ETL listing to UL 61010; dual 50,000-count display; 0.05 % + 5 basic DCV; BLE plus microSD logging; open schematic; BLE packet format documents published; firmware updated by copying `EEVBlog.bin` to the SD card; MCU described as hackable with an internal programming header — [EEVblog product page](https://eevblog.com/product/121gw/)
- sigrok status: supported (driver id `eevblog-121gw` in serial-dmm); recent firmware sends 19-byte BLE packets; the BLE link is slow, around two samples per second, and occasionally unreliable; UART can also be tapped between MCU and BLE module — [OpenTraceLab/sigrok wiki mirror](https://opentracelab.github.io/website/hardware/multimeter/eevblog-121gw)

OWON BLE handhelds
- Community protocol work exists for B35/B35T and B41T+: [DeanCording/owonb35](https://github.com/DeanCording/owonb35), [inflex/owon-b35](https://github.com/inflex/owon-b35), [art-ya/OwonB41T](https://github.com/art-ya/OwonB41T), an npm web-Bluetooth client [owonb41t](https://www.npmjs.com/package/owonb41t), and an EEVblog thread on a Linux client — [EEVblog](https://eevblog.com/forum/testgear/linux-bluetooth-client-for-owon-b35-multimeter) (all leads; contents not opened)

Others in sigrok's supported list
- Fluke 287/289 (RS232 via IR cable), Agilent/Keysight U12xx (USB/Bluetooth), Mooshimeter (BLE), many Voltcraft VC-8xx/VC-9xx, Mastech MS8250B/D, Victor 70C/86C — [sigrok Supported hardware](https://sigrok.org/wiki/Supported_hardware)

### Inferences
- For the agent, the XDM1241's readable serial number and firmware string are the hook for self-diagnosis; there is no error queue (`SYST:ERR?` fails), so the driver must infer faults from timeouts and `nOK`.
- Driving the XDM through raw serial is trivial in Rust with the `serialport` or `tokio-serial` crates; no dedicated Rust crate for XDM1041/1241 or UT61E+ was found. The UT61E+ needs `hidapi` (works on ARM64 Linux via hidraw/libusb) and a port of the 19-byte parser.
- The 121GW is the closest thing to a firmware-hackable DMM (open schematic, user-updatable firmware, exposed programming header), but its roughly 2 Hz BLE link is slow for an agent loop.
- A sensible split: XDM1241 as the Pi-attached bench meter; a CAT-rated handheld (BM869s or 121GW) reserved for the human on mains/battery work, with its data link as a bonus.

### Gaps
- XDM1241 power arrangement (internal battery vs USB vs mains) and its official CAT marking were not confirmed from an OWON datasheet; the two retail sources disagree on XDM1041 power.
- No source found for: OW18E protocol, Aneng/Zoyi ZT-703S / ZT-5B BLE protocols, HoldPeak links, UT8804/UT8805 protocol, UT61E+ official CAT rating and whether the USB cable is bundled, Keysight U12xx price.
- Prices for UT61E+, BM869s (+BU-86X cable), 121GW not verified. One review site has withdrawn its BM869s buying link — [hobbyistscope](https://hobbyistscope.pages.dev/multimeters/brymen-bm869s).
- No evidence of replacement open firmware for any DMM beyond the 121GW's vendor-supported updates.

---

## DC and AC current sensing, battery monitors, mains energy and power quality

### Takeaway
For 100 A+ battery cables the cleanest agent-readable options are a shunt read by an INA228 (85 V rated, I2C) or a Victron SmartShunt (documented VE.Direct serial plus decryptable BLE broadcasts, with maintained Rust crates). For 120 V circuits the PZEM-004T v3 (Modbus) and Shelly Pro EM-50 (local RPC/Modbus TCP, UL badge) cover RMS quantities; neither provides waveform-level power quality, which needs a DIY isolated sampler.

### Cited Findings
Victron
- Rust crate `victron_ble` 1.2.0 (MIT) decodes Victron BLE advertisements for Solar Charger, Battery Monitor, Inverter, VE.Bus and AC Charger; needs the per-device encryption key copied once from VictronConnect (Settings > Product Info > Encryption data) and "Instant Readout" enabled; Bluetooth feature supports Linux and macOS; a no_std mode parses manufacturer data supplied by the caller — [docs.rs victron_ble](https://docs.rs/victron_ble)
- Rust crates for the wired VE.Direct protocol exist: `vedirect` 0.2.0 — [docs.rs](https://docs.rs/crate/vedirect/latest) and `ve_direct` 0.1.4 — [docs.rs](https://docs.rs/crate/ve_direct/0.1.4/source) (leads)
- Python and ESPHome decoders: [victron-ble (Python)](https://github.com/orymate/victron-ble), [esphome-victron_ble](https://github.com/Fabian-Schmidt/esphome-victron_ble), and a Home Assistant core integration — [Home Assistant](https://www.home-assistant.io/integrations/victron_ble) (leads)

Junctek/Juntek
- ESPHome component for the KG-F series (tested on KG110F) reads voltage, current, battery percent and temperature through the monitor's RS485 port at 115200 baud via an RS485 adapter; device address configurable — [tfyoung/esphome-junctek_kgf](https://github.com/tfyoung/esphome-junctek_kgf)
- Further implementations (leads): [f3sty/junctek](https://github.com/f3sty/junctek), a BLE-based monitor script [AnalogThinker/junctek_monitor](https://gh.r-mj.com/AnalogThinker/junctek_monitor), Home Assistant threads on RS485 — [HA community](https://community.home-assistant.io/t/juntek-kg-f-series-on-ha/443741) and BLE — [HA community](https://community.home-assistant.io/t/juntek-battery-monitor-bluetooth-integration/626109)

INA228
- 85 V bus input, 20-bit ADC, gain error ±0.05 % max, power accuracy 0.5 % FS, energy/charge accumulation, external shunt supported, USD 14.95 — [SparkFun](https://www.sparkfun.com/sparkfun-current-sensor-ina228-qwiic.html)
- Adafruit sells an equivalent "INA228 I2C 85V 20-bit" STEMMA QT board with a CircuitPython driver — [Adafruit guide](https://learn.adafruit.com/adafruit-ina228-i2c-power-monitor.md), [driver](https://github.com/adafruit/Adafruit_CircuitPython_INA228)
- Rust: an `ina229` crate (SPI sibling) 0.1.0 exists — [docs.rs](https://docs.rs/crate/ina229/0.1.0); `embedded-devices` 0.10.2 is a multi-device embedded-hal(-async) driver collection — [docs.rs](https://docs.rs/crate/embedded-devices/latest) (leads; INA228 coverage inside `embedded-devices` not confirmed)

PZEM
- PZEM-004T v3 (AC): 80-260 V, 0-10 A or 0-100 A (CT) models, power factor, 45-65 Hz frequency, energy; 0.5 % class figures. PZEM-017 (DC): 0.05-300 V, shunt options 50/100/200/300 A, 1 % accuracy; below 7 V it needs a separate USB supply that must not be a PC USB port — [ESPEasy P102 docs](https://espeasy.readthedocs.io/en/latest/Plugin/P102.html)
- Rust crate `pzem004t` 0.1.7 (2020-03-12, MIT/Apache-2.0) is an embedded-hal 0.2 driver with a desktop CLI; reads voltage, current, power, energy, frequency, power factor, alarm — [docs.rs](https://docs.rs/crate/pzem004t/latest/)
- ESPHome `pzemac` component and several Arduino/ESP-IDF libraries exist — [ESPHome](https://esphome.io/components/sensor/pzemac.html), [mandulaj/PZEM-004T-v30](https://www.github.com/mandulaj/PZEM-004T-v30) (leads)

Shelly
- Pro EM-50: two 50 A CT channels, ±1 % for 5-50 A, Ethernet + Wi-Fi + BT, local RPC/JSON, MQTT, WebSocket, Modbus TCP, scripting; reports V, I, active/apparent power, PF, energy; frequency is not listed as a measured quantity — [Shelly US](https://us.shelly.com/products/shelly-pro-em-50)

Power quality
- Analog Devices documents harmonic analysis on the ADE9000 metering AFE — [AN-1483](https://www.analog.com/en/app-notes/an-1483.html) (lead)
- A TU Delft thesis describes software for a Class-S power quality analyzer built on a Raspberry Pi — [TU Delft repository](https://repository.tudelft.nl/record/uuid:1b2538aa-e4aa-4bc9-9b29-608b65c13a18) (lead)

### Inferences
- "No closed phone app in the loop" caveat: Victron BLE needs VictronConnect once to read the encryption key. The wired VE.Direct text protocol avoids that entirely and is the better primary link; BLE then serves as an independent second channel for cross-checking.
- An INA228 on a 100 A/75 mV shunt gives the agent a tweakable instrument (conversion time, averaging, shunt calibration register, alert limits) that a sealed battery monitor does not; pair it with an I2C isolator (ISO1540/ADuM1250 class) so the Pi never shares ground with the bank.
- High-side placement is within the INA228's 85 V common-mode rating for a 48 V nominal bank (up to roughly 60 V), leaving margin; low-side placement removes the common-mode concern altogether.
- The PZEM-017 note about not powering from a PC USB port suggests its measurement side is not isolated from its low-voltage supply input; treat it as non-isolated until proven otherwise and prefer INA228 + isolator for the 48 V bank.
- PZEM-004T/Shelly give RMS values only. THD, sags and frequency excursions on inverter output need sampled waveforms: the cheapest open route is an isolated low-voltage replica of the line (small AC-AC transformer or resistor divider into an AMC1301/AMC3330 isolated amplifier) plus a CT, digitised by an RP2040/RP2350 ADC, ADS131M0x, or a USB audio interface, with FFT on the Pi.
- Split-core hall sensors (YHDC HSTS016L class) into an ADC are attractive for clip-on DC current without breaking the cable, but zero drift and earth-field/adjacent-conductor sensitivity make them a trend sensor rather than a reference; a shunt remains the ground truth.

### Gaps
- No verified price or protocol documentation fetched for Victron SmartShunt hardware itself, Junctek KH-F series, HSTS016L / Tamura / CR Magnetics sensors, Emporia Vue, IoTaWatt (availability in 2026 unknown), emonTx/emonPi.
- No BLE clamp meter with a documented, reverse-engineered protocol was confirmed this session.
- No ready-made, cheap, open power-quality capture product was found; only the building blocks above.
- Whether the old `pzem004t` crate matches the v3 Modbus register map is not stated in its docs.

---

## Oscilloscope / logic analyzer / mixed-signal for headless agent use

### Takeaway
Headless digital capture is solved cheaply by sigrok: fx2lafw clones, DSLogic, Kingst and Hantek 6022BE/BL are in the supported list, and the Pico firmware driver is in mainline libsigrok since September 2023. A real analog scope that an agent can drive (Rigol DHO804 over SCPI, Analog Discovery 3 via WaveForms SDK) costs USD 379-459 and none of the USB scopes is isolated, so floating 48 V work needs a differential probe or isolated front end.

### Cited Findings
- sigrok's supported logic analyzers include Saleae Logic/Logic16 and generic fx2lafw clones ("Noname Saleae clone", "Lcsoft Mini Board", MCU123 clones), DSLogic / Basic / Plus / Pro, Kingst LA2016/LA5016/LA5032, Hantek 4032L and 6022BL, BeagleLogic, SUMP-compatible devices; DSLogic U3Pro16 is listed only as planned — [sigrok Supported hardware](https://sigrok.org/wiki/Supported_hardware)
- sigrok's supported oscilloscopes include Hantek 6022BE (2 ch, 48 MS/s, 20 MHz), Rigol DS1000E/DS1000Z/DS2000, Siglent SDS1000X/SDS2000X, Hameg HMO; Rigol DHO800 was not found; Rigol MSO5000 is in progress — [sigrok Supported hardware](https://sigrok.org/wiki/Supported_hardware)
- sigrok-pico (RP2040 firmware) was merged into mainline libsigrok on 28 Sep 2023 as the `raspberrypi-pico` driver; sigrok-cli 0.7.2 predates it, so a current build is required. A work-in-progress fork adds RP2350B boards with 32 digital channels (Pico 2 / RP2350A with 21 channels is untested) and a buffer twice the RP2040 size — [c1570/sigrok-rp2350](https://github.com/c1570/sigrok-rp2350)
- Other Pico analyzers (leads): gusmanb LogicAnalyzer, 24 channels at 100 Msps on a Pico — [Hackster](https://www.hackster.io/news/agustin-gimenez-bernad-s-logicanalyzer-turns-a-raspberry-pi-pico-into-a-4-24-channel-100msps-marvel-6674515a398f); an RP2350 "V6.0 400 MHz 24 channel" board sold on Tindie — [Tindie](https://www.tindie.com/products/trilobytesretro/logic-analyzer-v60-rp2350-2-400mhz-24-channel); `pysigrok` Pico hardware plugin — [GitHub](https://github.com/pysigrok/hardware-raspberrypi-pico); Rust-firmware "ula" micro logic analyzer for RP2040 — [dotcypress/ula](https://GitHub.com/dotcypress/ula)
- Glasgow revC: USD 179 at 1BitSquared, 8 in stock, FPGA-based, applet software usable from the command line — [1BitSquared](https://1bitsquared.com/products/glasgow-c)
- Analog Discovery 3: USD 379; two 14-bit differential scope channels at 125 MS/s with ±25 V input range, two AWG channels, 16 digital I/O, programmable ±5 V supplies; WaveForms SDK in C/C++/Python — [Digilent](https://digilent.com/shop/analog-discovery-3/)
- ARM64 status for WaveForms: a user ran WaveForms 3.20.1 on a CM4 with 64-bit Raspberry Pi OS Bullseye; version 3.21.3 installed but failed for want of `libQt5SerialPort.so.5`; 32-bit images failed — [Digilent forum](https://forum.digilent.com/topic/27415-cant-run-waveforms-3213-on-rpi-cm4/)
- An MCP server for the Analog Discovery 2 was published 2 Feb 2026 (Python, FastMCP, WaveForms SDK). Reported lessons: the model interpreted "3.3 V sine" as amplitude and produced 6.6 Vpp; large captures should be written to files rather than returned as JSON because of token cost — [Altium/Octopart](https://resources.altium.com/p/building-mcp-server-analog-discovery)
- Rigol DHO804: 70 MHz, 4 channels, 1.25 GSa/s, 25 Mpts, USD 459, input CAT I 300 V, 12 V DC powered — [DigiKey](https://www.digikey.com/en/products/detail/rigol-technologies/DHO804/21421243)
- ThunderScope: open-source 4-channel 1 GS/s scope streaming over PCIe/Thunderbolt; PCIe card USD 950 (the enclosure version's price is garbled on the page); developer-edition units were due at the maker by end of 2024 with Mouser fulfilment; Linux and Windows supported, ARM64 not mentioned; client is ngscopeclient — [Crowd Supply](https://crowdsupply.com/eevengers/thunderscope)
- Micsig DP10007 differential probe: 700 V differential at 100X (70 V at 10X), 100 MHz, 600 V CAT I / 1000 V CAT II to earth, powered from 5 V USB; USD 289.99 list, discontinued at SainSmart — [SainSmart](https://www.sainsmart.com/products/micsig-dp10007-high-voltage-differential-probe-kit); listed at EUR 166.12 ex VAT by an EU seller (search-result title only) — [Eleshop](https://eleshop.eu/micsig-dp10007-differential-probe.html)

### Inferences
- Best value for the stated budget: fx2lafw clone (digital, 8 ch) plus a Pico/Pico 2 running sigrok-pico (digital + a few slow ADC channels, firmware modifiable; the owner could later rewrite it in embassy Rust). Both are plain USB devices handled by libsigrok on ARM64 Linux.
- ThunderScope (price, Thunderbolt/PCIe host requirement) and DHO804/AD3 (price) fall outside a USD 200-350 measurement share. A Pi 5/CM4 PCIe lane is Gen 2 x1, well below ThunderScope's Gen 2 x4 design point.
- The DHO804 runs from 12 V DC, which suits a battery-powered bench, but its BNC shells are tied to chassis/USB ground and its inputs are CAT I: fine for electronics, not for direct connection to mains or a floating battery bank without a differential probe.
- The AD3's differential ±25 V inputs still reference USB ground through high-value resistors; they are not galvanically isolated and do not cover a 48-60 V bus without an external divider/isolated front end.
- Glasgow is the most "hackable" digital interface tool (open gateware, Python applets) and complements rather than replaces a logic analyzer.

### Gaps
- Not researched to source this session: DSLogic U2Basic/Plus/U3Pro current prices and DSView-vs-sigrok status, Hantek 6022BE/BL prices and OpenHantek6022 CLI use, OWON HDS200/HDS2000 SCPI command sets, FNIRSI handheld data links, Rigol DHO800 SCPI/LAN details and Android hacks, Siglent, Red Pitaya, Espotek Labrador, Scoppy, LabNation SmartScope, haasoscope, ADALM2000/libm2k on aarch64.
- Current (late-2026) ThunderScope shipping status and final prices were not confirmed; the campaign page text appears to date from late 2024.
- Whether upstream libsigrok's `raspberrypi-pico` driver works unmodified with the RP2350 fork is not stated.

---

## Source/measure and power tools (supplies, loads, USB meters, Pine64 gear)

### Takeaway
Riden RD60xx supplies are the best-supported cheap programmable source (sigrok driver, Python libraries, and a Rust crate updated on 21 Nov 2026 that can even flash firmware). The Pinecil V2 is fully agent-readable and its setpoint is writable over BLE; no PinePower model documents any data interface. The ChargerLAB KM003C stands out among USB meters because it has a mainline Linux hwmon driver, an MIT-licensed protocol study, a Rust implementation and an experimental custom Rust firmware.

### Cited Findings
Programmable supplies
- sigrok supports the RDTech RD series (RD6006 named; USB/serial/Wi-Fi), RDTech DPS series (USB/BT), Korad KAxxxxP (USB/RS232), Rigol DP700/DP800 and others; Hanmatek HM305P and Siglent SPD3303 are work-in-progress — [sigrok Supported hardware](https://sigrok.org/wiki/Supported_hardware)
- Rust crate `ruiden` 0.5.1 (published 2026-11-21): library + CLI over Modbus RTU using `tokio-modbus`; auto-detects RD6006/RD6006P, RK6006, RD6012/RD6012P, RD6018, RD6024 (RD6030 scaling unverified); sets V/I, toggles output, bulk-reads measurements, decodes temperatures and CV/CC and OVP/OCP status, handles presets and RTC, and includes a bootloader firmware flasher (`ruiden flash`); calibration registers intentionally have no high-level API — [docs.rs ruiden](https://docs.rs/crate/ruiden/latest)
- Riden register documentation and tools (leads): vendor protocol PDF — [RD6006 protocol](https://redmine.laas.fr/attachments/download/4768/RD6006_prtotocol_en.pdf), Joy-IT Modbus document — [Joy-IT](https://www.joy-it.net/files/files/Produkte/JT-PS1440-C-Pro/MODBUS_Protocol_RD60xx_PSxx.pdf), Python [ShayBox/Riden](https://www.github.com/ShayBox/Riden), [rd6006](https://github.com/jbdk24/rd6006), alternative Wi-Fi dongle firmware exposing Modbus TCP/SCPI [riden-dongle](https://github.com/morgendagen/riden-dongle), [riden-flashtool](https://github.com/bdd/riden-flashtool)
- Alientek DP100: open USB-HID implementation (lead) — [lessu/open_dp100](https://github.com/lessu/open_dp100)

Electronic loads and USB meters in sigrok
- Loads: ATORCH DL24MP-150W, ITECH IT8500 series, Maynuo M9812, ZKETECH EBD-USB, Arachnid Labs Re:load Pro supported; Siglent SDL10x0 in progress — [sigrok Supported hardware](https://sigrok.org/wiki/Supported_hardware)
- USB/energy meters: ATORCH J7-C, RDTech TC66C, RDTech UM series (UM24C shown) supported — [sigrok Supported hardware](https://sigrok.org/wiki/Supported_hardware)
- Signal generators: Joy-IT JDS6600 and Rigol DG800/DG900/DG1000Z supported; MHS-5200A in progress; FY6900 not found — [sigrok Supported hardware](https://sigrok.org/wiki/Supported_hardware)

ChargerLAB KM003C
- The Linux kernel ships a `powerz` hwmon driver for the POWER-Z KM003C; channel labels mirror the device display, with signed bus current — [kernel.org hwmon/powerz](https://docs.kernel.org/hwmon/powerz.html)
- MIT-licensed protocol research repo documents ADC and PD-capture packets, USB IDs (VID 0x5FC9, PID 0x0063, bulk endpoints 0x01/0x81), authenticated memory-read/streaming commands; links a Rust implementation (`km003c-rs`), a Python package `km003c` with Rust bindings, and a separate custom Rust firmware project (`km003c-firmware`) — [okhsunrog/km003c-protocol-research](https://github-users.yhajoa.workers.dev/okhsunrog/km003c-protocol-research)
- ChargerLAB hosts a KM003C/KM002C technical-support page (lead) — [ChargerLAB](https://www.chargerlab.com/km003c-km002c-technical-support)

Nordic PPK2
- Rust crate `ppk2` 0.1.3 (2025-06-13), library + `ppk2-cli`, self-described as incomplete but usable in source-meter mode; derived from Nordic's nrfconnect-ppk and the Python ppk2-api — [docs.rs ppk2](https://docs.rs/crate/ppk2/latest)
- The Python `ppk2-api` is packaged for aarch64 in Alpine edge (lead) — [Alpine packages](https://pkgs.alpinelinux.org/package/edge/testing/aarch64/py3-ppk2-api); a Nordic DevZone thread reports trouble running `nrfutil` on aarch64 Linux (lead) — [DevZone](https://devzone.nordicsemi.com/f/nordic-q-a/123735/nrfutil-on-aarch64-linux-issue-with-arch)

Pine64 gear
- IronOS BLE exists on Pinecil V2: a live-data service (tip temperature, setpoint, DC input voltage, handle temperature, power level, power source, tip resistance, uptime, last-movement time, max settable temperature, raw tip reading, hall sensor, operating mode, estimated watts; each a 32-bit value), a settings service (one 16-bit characteristic per setting, plus save and factory-reset characteristics), and a bulk service (all live data in one blob, firmware build ID, device serial/MAC, unique ID). BLE mode is Off / On (read-write) / Read-only and can only be changed on the iron; notifications are incomplete so clients poll — [IronOS Bluetooth docs](https://ralim.github.io/IronOS/Bluetooth)
- Python `pynecil` (MIT) reads live data and device info and writes settings; its README example sets the setpoint with `client.write(CharSetting.SETPOINT_TEMP, 350)` — [tr4nt0r/pynecil](https://github.com/tr4nt0r/pynecil)
- Home Assistant has a core IronOS integration and there is a Pinecil-to-MQTT bridge (leads) — [Home Assistant](https://www.home-assistant.io/integrations/iron_os), [pinecil2mqtt](https://www.github.com/excieve/pinecil2mqtt)
- PinePower: Pine64's documentation for the 65 W, Desktop (120 W) and Desktop V2 (165 W) describes ports and on-device displays but no USB-serial/UART/I2C/BLE data interface, no named MCU, no firmware source and no schematic; the Desktop teardown notes a HUSB339 PD controller — [Pine64 PinePower docs](https://pine64.org/documentation/PinePower/_full/)
- A Pine64 forum thread is titled "Firmware Update for PinePower Desktop Power Cycling Issue" and Hackster reported a Desktop V2 "0 V" display flaw (leads; content not opened) — [Pine64 forum](https://forum.pine64.org/showthread.php?tid=19205), [Hackster](https://www.hackster.io/news/pine64-warns-of-pinepower-desktop-v2-0v-flaw-but-says-it-s-purely-a-display-issue-f195d173f4a4)

### Inferences
- The Pinecil can serve as an agent-controlled heat source and as a crude voltmeter on whatever supply feeds it (DC input voltage is in the live data); a Rust client is a straightforward `btleplug` port of pynecil's UUID table. Only the V2 hardware has BLE, so the owner's Pinecil revision matters.
- A PinePower cannot be read directly, but anything it powers through USB-C can be instrumented in-line (KM003C or an INA228), and a USB-PD sink controller with I2C (AP33772/HUSB238 class) would let the Pi request PD voltages from it, turning it into a coarse programmable source. Whether either PinePower offers PPS was not checked.
- The existence of a firmware update for the PinePower Desktop implies some flashing path exists, but nothing public documents it as an interface.
- Riden + `ruiden` gives the agent readable instrument state (model detection, internal temperature, protection flags) and a controllable stimulus for cross-checks; an RK6006/RD6006 fed from the 48 V bank or a USB-PD source keeps it portable.

### Gaps
- No protocol sources opened for FNIRSI FNB58, AVHzY CT-3, WITRN, Joulescope, Kiprim, East Tester ET54xx SCPI, Kunkin loads, Atorch DL24 framing, FY6900 serial protocol. Prices for all items in this section except PPK2 are unverified.
- `ruiden`'s licence and whether it exposes the firmware-version register are not stated on its docs page.
- KM003C sampling rates and the content of ChargerLAB's official protocol document were not confirmed.

---

## Component and continuity tools (LCR/ESR, insulation, battery internal resistance)

### Takeaway
Scriptable component testers are thin on the ground at this price: sigrok covers the DER EE DE-5000 (serial via its IR adapter) and the UNI-T UT612 (USB HID); the YR1035+ has community serial notes that were not verified. Nothing sourced this session shows a data link on the FNIRSI LCR-P1 or on cheap insulation testers.

### Cited Findings
- sigrok-supported LCR meters: DER EE DE-5000 (serial), UNI-T UT612 (USB/HID), Mastech MS5308, PeakTech 2165/2170, Voltcraft 4080; Agilent U1732B and BK 879B only planned — [sigrok Supported hardware](https://sigrok.org/wiki/Supported_hardware)
- YR1035/YR1035+ serial/UART notes exist as GitHub gists (leads, unopened) — [gist 1](https://gist.github.com/enkiusz/6408645efd622b8a638a14957cd37f47), [gist 2](https://gist.github.com/ludwich66/fd98f5b5a96d4c5991cbac34a5e82b86); an independent review of the YR1035 exists — [lygte-info](https://lygte-info.dk/review/InternalResistanceMeterYR1035%20UK.html)
- An open Arduino "Ultimate Battery Tester" project measures capacity/ESR with serial output (lead) — [ArminJo](https://github.com/ArminJo/Ultimate-Battery-Tester)
- DMMCheck Plus doubles as an LCR sanity reference: 0.001-1 µF capacitors and 0.001-1 mH inductors supplied with measured values within 0.05 % — [Welectron](https://www.welectron.com/DMMCheck-Plus-Rev-8-Multimeter-Calibration-Reference)

### Inferences
- With an agent in the loop, component testing can be synthesised from instruments already on the bench: a programmable source plus INA228/DMM gives resistance, diode curves and capacitor leakage; a signal source plus two ADC channels gives impedance at a few frequencies. That is cheaper and more hackable than buying a closed LCR handheld.
- Battery internal resistance of the installed bank can be estimated in place from naturally occurring load steps (ΔV/ΔI) logged by the INA228 or SmartShunt, avoiding a separate 1 kHz AC-IR tester for trend purposes.
- Insulation (megger) testing applies 250-1000 V and should remain a human-operated, human-triggered measurement even if a readable instrument is found.

### Gaps
- No verified data-link information for FNIRSI LCR-P1, LCR-T4/T7 transistor testers (serial mod), insulation testers, or the YR1035+ protocol details and price. DE-5000 price and whether its IR-USB adapter is bundled were not checked.

---

## Thermal imaging readable by an agent on Linux

### Takeaway
The Topdon TC001 (256x192, about USD 199-208) and its InfiRay P2 Pro sibling enumerate as ordinary USB video devices and have several open readers, including a Rust crate updated on 13 Nov 2026 that returns absolute temperatures. The MLX90640 I2C array is the low-cost fallback with far lower resolution.

### Cited Findings
- Rust crate `topdon_thermal_rs` 0.2.1 (2026-11-13, MIT): locates the TC001 by USB ID on Linux, configures raw streaming, and exposes a visual image, a colour-mapped image and per-pixel temperatures (`temperatures()`, `average_temperature()`); depends on system OpenCV — [docs.rs](https://docs.rs/topdon_thermal_rs)
- The TC001 is based on the InfiRay P2 Pro; Linux sees a standard USB video device carrying one luminance image and one stream that encodes raw thermal data. Open tools: PyThermalCamera (Les Wright, Python, TC001), P2Pro-Viewer (LeoDJ, P2 Pro), Thermal-Camera-Redux (92es) — [Hackaday](https://hackaday.com/2023/07/11/phone-thermal-cameras-get-open-source-desktop-tools)
- P2Pro-Viewer is Python, MIT, work in progress; vendor-command control (shutter, gain, emissivity) is on its roadmap rather than finished, and its Linux path was untested by the author — [LeoDJ/P2Pro-Viewer](https://www.github.com/LeoDJ/P2Pro-Viewer)
- An EEVblog thread tracks Thermal Camera Redux testing on Raspberry Pi/other SBCs with the TC001 and clones (lead) — [EEVblog](https://www.eevblog.com/forum/thermal-imaging/rasbpi-and-topdon-tc001-%28guinea-pig%29-tester-requested/)
- TC001 pricing: USD 199.00 to 207.89 at two US merchants, 256x192 — [Klarna US](https://www.klarna.com/us/shopping/pl/cl345/3402637283/Power-Tools/Topdon-TOPDON-TC001-Portable-Thermal-Imaging-Camera/)
- MLX90640 breakouts and Pi guides exist from Pimoroni and Adafruit; a Pi add-on fusing MLX90640 with an RGB camera ("PitFusion") was announced in May 2025 (leads) — [Pimoroni](https://shop.pimoroni.com/en-ch/products/mlx90640-thermal-camera-breakout), [Adafruit guide](https://learn.adafruit.com/raspberry-pi-thermal-camera.md?view=all), [CNX Software](https://www.cnx-software.com/2025/05/16/pitfusion-thermal-imager-for-raspberry-pi-combines-melexis-mlx90640-sensor-and-rgb-camera/)

### Inferences
- For an agent, the TC001 class is the right pick when budget allows: UVC means no vendor SDK, it works over V4L2 on ARM64, and the Rust crate removes the Python dependency (its OpenCV requirement is the main packaging cost on a CM4).
- At roughly USD 200 the TC001 alone consumes most of a USD 200-350 measurement share; an MLX90640 (32x24) is enough to spot a hot lug, breaker or MOSFET at close range and reads directly over I2C.
- Readable instrument state on these cameras is limited: open tools focus on frames; firmware version/serial readout via vendor commands is not established in the sources read.

### Gaps
- Not verified: TC001 Plus and TS001 Linux compatibility, Thermal Master P2/P3, InfiRay T2S+, FLIR Lepton + PureThermal pricing, exact frame layout/temperature formula (the commonly described 256x384 dual-image layout was not confirmed in the pages read), a maintained Rust MLX90640 crate, and current MLX90640 breakout prices. Amazon price for the TC001 not confirmed (tracker listed other merchants).

---

## Build-it-yourself measurement front ends

### Takeaway
The INA228 (85 V, 20-bit, USD 14.95 breakout) is the strongest ready-made building block for the 12 V/48 V use cases, and RP2040/RP2350 boards already have open instrument firmware (sigrok-pico in C, "ula" in Rust). A DIY front end is where agent tweakability is highest, since ranges, filtering and calibration constants live in firmware the owner controls.

### Cited Findings
- INA228 breakout specs and price — [SparkFun](https://www.sparkfun.com/sparkfun-current-sensor-ina228-qwiic.html); Adafruit equivalent and CircuitPython driver — [Adafruit](https://learn.adafruit.com/adafruit-ina228-i2c-power-monitor.md)
- Rust driver landscape (leads): `ina229` 0.1.0 — [docs.rs](https://docs.rs/crate/ina229/0.1.0); `ina226` 0.3.0 — [docs.rs](https://docs.rs/crate/ina226/latest/features); `ina233` — [docs.rs](https://docs.rs/ina233); `embedded-devices` 0.10.2 collection — [docs.rs](https://docs.rs/crate/embedded-devices/latest)
- Open Pico instrument firmware: sigrok-pico (mainline libsigrok driver since 2023-09-28) and its RP2350 fork — [c1570/sigrok-rp2350](https://github.com/c1570/sigrok-rp2350); Rust RP2040 logic analyzer "ula" — [dotcypress/ula](https://GitHub.com/dotcypress/ula); a two-Pico-2 modular bench tool "Picotronix" and a Silicon Chip Pico mixed-signal analyser (leads) — [Hackster](https://www.hackster.io/news/thort-werx-s-picotronix-is-a-low-cost-modular-bench-tool-powered-by-a-pair-of-raspberry-pi-pico-2s-a4e32cdbe817), [Silicon Chip](https://www.siliconchip.com.au/Issue/2024/September/Pico+Mixed-Signal+Analyser+%28PicoMSA%29)
- BayLibre ACME (I2C power probes) is an existing open multi-channel power-measurement design supported in sigrok — [sigrok Supported hardware](https://sigrok.org/wiki/Supported_hardware)
- TI's ISOUSB211 is a reinforced-isolation USB repeater (described as 5000 Vrms) supporting high/full/low speed, with an evaluation module — [TI ISOUSB211DPEVM](https://www.ti.com/tool/ISOUSB211DPEVM)

### Inferences
- A sensible DIY "power head": INA228 + external shunt + I2C isolator + isolated DC-DC, in a printed enclosure with fused sense leads. One for the 48 V bank, one for 12 V vehicle work. All registers (ADC config, shunt cal, alert thresholds, die temperature, manufacturer/device ID) are readable, so the agent can verify the sensor's identity and health.
- For mains or higher-voltage waveform capture, an AMC1301/AMC3330-class isolated amplifier behind a resistive divider feeding an RP2350 or ADS131M0x keeps the Pi isolated; this is the piece that enables THD/sag analysis. It is a soldering project, well within "decent soldering skills", but the divider and creepage layout are safety-critical.
- An embassy-rs RP2350 firmware exposing a small documented USB protocol (or SCPI-like text) would give every DIY head a uniform identity/self-test/calibration-constant interface the MCP server can rely on.

### Gaps
- No sourced prices or Rust crates confirmed for ADS1115, ADS1256, ADS131M0x, INA229/INA238 breakouts, AMC1301/AMC3330 boards; no specific open-source multimeter project or µCurrent-style design was verified this session.
- A confirmed, maintained Rust `ina228` driver was not found by name.

---

## Self-diagnosis and calibration ground truth; prior art for agent-driven instruments

### Takeaway
Cheap AD584 modules are adequate ground truth for 4.5-digit checks when the hand-labelled values are used, and a DMMCheck Plus gives a broader certified artifact (5 V ±0.007 %, 0.1 % resistors, 1 mA, AC, frequency) at EUR 269. Several MCP servers for instruments already exist (sigrok-mcp-server, LabMCP, Keysight, Analog Discovery, Rigol), but none found performs automated cross-verification between instruments, so a relay "calibration bus" would be new ground.

### Cited Findings
References
- A reviewed AD584 module's four outputs sat about 310-540 ppm above nominal but within about 2-12 ppm of the values written on its label, all inside datasheet limits; datasheet tempco is ±5 ppm/K (5 V and above) and ±10 ppm/K (2.5 V); the reviewer judged it fit for checking 4.5-digit meters and no more, noting no Kelvin sense and an unpopulated output filter cap — [xDevs](https://xdevs.com/review/ad584_vref_test/)
- DMMCheck Plus Rev 8: 5 V DC ±0.007 %; 100 Ω/1 kΩ/10 kΩ/100 kΩ 0.1 % 10 ppm resistors; 1 mA DC ±0.1 %; 5 V RMS square-wave AC ±0.1 %; 1 mA AC ±0.2 %; 100 Hz ±0.02 %; capacitors and inductors with measured values; 9 V battery; ships with a measured-values sheet; EUR 269 incl. VAT — [Welectron](https://www.welectron.com/DMMCheck-Plus-Rev-8-Multimeter-Calibration-Reference)
- EEVblog has a thread on a low-cost reference setup for 5.5-digit DMMs (lead) — [EEVblog](https://www.eevblog.com/forum/metrology/low-cost-reference-setup-to-check-and-calibrate-55-5-digit-dmms/msg6124221)

Prior art: agents driving instruments
- sigrok-mcp-server (KenosInc) wraps `sigrok-cli` and exposes tools including `scan_devices`, `capture_data`, `decode_protocol`, `check_firmware_status`, `serial_query` and `get_device_profile`, with per-device notes such as the XDM1241 profile — [project docs](https://kenosinc.github.io/sigrok-mcp-server/)
- LabMCP (K-Dense): open-source Python collection of 32 connectors / 411 tools covering SCPI, Modbus, EPICS, SiLA 2, power supplies, DAQ and oscilloscopes; safety flags include `--simulate`, `--read-only`, `--limit` for setpoint bounds, approval flags on actions that energise or move things, and an audit log; every connector is still marked simulated (not confirmed on hardware); no general cross-instrument verification feature is described — [README](https://raw.githubusercontent.com/k-dense-ai/lab-instrument-mcps/HEAD/README.md)
- Analog Discovery MCP server write-up (Feb 2026), with the peak-vs-peak-to-peak mistake and token-cost lessons — [Altium](https://resources.altium.com/p/building-mcp-server-analog-discovery)
- Other published instrument MCP servers (leads): Keysight "MCP for instrument control" — [Keysight](https://helpfiles.keysight.com/kmsic/English/keysight_mcp_for_instrument_control/Content/overview.html); `lablink-mcp` — [PyPI](https://pypi.org/project/lablink-mcp/); `teasel-server` — [PyPI](https://pypi.org/project/teasel-server/); `usbtmc-lite-mcp` — [PyPI](https://pypi.org/project/usbtmc-lite-mcp/); `mcpyvisa` — [PyPI](https://pypi.org/project/mcpyvisa/); `lecroy-mcp` — [PyPI](https://pypi.org/project/lecroy-mcp/0.1.8/); a Microsoft blog on connecting Copilot to an oscilloscope — [Microsoft](https://techcommunity.microsoft.com/blog/EducatorDeveloperBlog/connecting-github-copilot-to-my-oscilloscope/4555545); a Rigol scope MCP article — [IMD](https://industrialmonitordirect.com/blogs/knowledgebase/rigol-oscilloscope-mcp-server-claude-ai-instrument-control)

### Inferences
Proposed self-test / calibration scheme the agent can run (researcher's design, not sourced):
1. **Identity and state sweep (every boot).** Query each instrument's identity and firmware (XDM `*IDN?`, Pinecil build ID/serial, Riden model registers, INA228 manufacturer/device ID, SmartShunt product/firmware fields, camera USB descriptors). Compare with a stored manifest; flag changed firmware, missing devices, low battery, protection flags.
2. **Zero and open checks.** Relays short each voltage input (offset) and open it (leakage/bias); shunt inputs shorted for current zero; ohms with leads shorted for lead/relay resistance.
3. **Reference injection.** Route AD584 2.5/5/7.5/10 V (using labelled values) to the DMM, INA228 bus input and any ADC head; route 0.1 % resistors to DMM ohms; derive a known current as Vref/Rref and pass it in series through DMM mA range and an INA228 shunt; feed a crystal-derived clock (Pi PWM/GPIO or an AD9833) to DMM frequency and to the logic analyzers to check timebases.
4. **Swept cross-check.** Programmable supply steps voltage and current; every instrument that can see the node logs simultaneously; the agent fits gain/offset per instrument against the DMM-plus-reference chain, flags outliers by majority agreement, and stores results to build drift history.
5. **Thermal check.** A matte-black plate with a contact sensor (DS18B20/TMP117 class) and a small heater gives the thermal camera/MLX90640 a known-temperature, known-emissivity target; the Pinecil's reported tip temperature is a secondary, uncalibrated cross-reference.
6. **Reporting.** Each measurement the agent returns carries the instrument's last-verified timestamp and residual error, so downstream reasoning can weight it.

Hardware notes for the calibration bus: use small signal relays (reed or latching) rather than analog muxes for ohms and current paths, because mux on-resistance and leakage corrupt those; a CMOS mux is acceptable for voltage into high-impedance inputs. Keep the bus below about 12 V, physically separate from any terminals that can see battery or mains potential, and interlock so that the bus relays cannot close while a hazardous-voltage head is connected.

Additional points:
- The AD3 MCP lesson generalises: tool schemas should carry explicit units and conventions (Vpp vs amplitude, RMS vs peak, sign of current), and bulk waveforms should be stored to files with only summaries returned.
- LabMCP's read-only mode, setpoint limits, approval flags and audit log are a ready template for this project's safety layer.
- The combination of sigrok-mcp-server's device-profile idea with an automated cross-check bus appears to be unoccupied territory based on what was found.

### Gaps
- No sourced prices for AD584 modules, LM4040/REF5050 boards, 0.01 %/0.1 % resistor sets, AD9833 boards or FY6900; DMMCheck Plus US-dollar price from the US maker (voltagestandard.com) not fetched.
- No published prior art was found for automated relay-routed cross-verification of hobby instruments; absence here reflects a limited search, not proof that none exists.
- sigrok-mcp-server's language, licence and ARM64 status are not stated on its docs page.

---

## Safety: CAT ratings, fusing, isolation, and what stays human-only

### Takeaway
Every USB-attached instrument in this class is ground-referenced to the Pi and carries a low measurement category (Rigol DHO804 inputs: CAT I 300 V; OWON XDM1041 reportedly CAT I/II), so none should be clipped onto a battery bank or mains by an agent. Agent access to hazardous circuits should be through fixed, fused, isolated sensors; probing, connecting and disconnecting on those circuits stays with the human using a properly rated handheld (BM869s CAT IV 1000 V, 121GW CAT III 600 V ETL-listed).

### Cited Findings
- Rigol DHO804 input rating: CAT I 300 V — [DigiKey](https://www.digikey.com/en/products/detail/rigol-technologies/DHO804/21421243)
- OWON XDM1041: 1000 V DC / 750 V AC ranges, but a buyer review states it is only CAT I/II rated; no category is given in the seller's specs — [Eleshop](https://eleshop.eu/owon-xdm1041-multimeter.html)
- Brymen BM869/BM869s: CAT IV 1000 V — [sigrok wiki](https://www.sigrok.org/wiki/Brymen_BM869)
- EEVblog 121GW: CAT III 600 V with independent ETL certification to UL 61010 — [EEVblog](https://eevblog.com/product/121gw/)
- Micsig DP10007 differential probe: 700 V differential, 600 V CAT I / 1000 V CAT II to earth, USB powered — [SainSmart](https://www.sainsmart.com/products/micsig-dp10007-high-voltage-differential-probe-kit)
- TI ISOUSB211: reinforced-isolation USB repeater described as 5000 Vrms, high/full/low speed — [TI](https://www.ti.com/tool/ISOUSB211DPEVM); the bare IC is stocked at DigiKey as a 480 Mbps single-channel part — [DigiKey](https://www.digikey.com/en/products/detail/texas-instruments/ISOUSB211DPR/16982078)
- PZEM-017 documentation warns against powering its USB input from a PC port when measuring below 7 V — [ESPEasy docs](https://espeasy.readthedocs.io/en/latest/Plugin/P102.html)
- Pinecil BLE can be locked to read-only on the device itself, with writes rejected — [IronOS docs](https://ralim.github.io/IronOS/Bluetooth)
- LabMCP advises keeping instruments' own interlocks active and treats software limits as an extra layer, not a certified safety system — [LabMCP README](https://raw.githubusercontent.com/k-dense-ai/lab-instrument-mcps/HEAD/README.md)

### Inferences
- Battery-bank hazard is fault current, not voltage: a 48 V lithium rack can source thousands of amps into a slipped probe or a ground loop. Measurement categories are defined around mains transients and do not by themselves certify a meter for high-energy DC; what matters is HRC fusing in the leads/sensor taps, short exposed tips, and current-limited sense wiring (inline fuses or series resistors placed at the bank end of every sense lead).
- Ground-loop scenario to design out: Pi USB ground tied to battery negative through an unisolated instrument, while the Pi is also connected to something bonded to earth or to a different point in the system (charger, inverter chassis, vehicle body, a second instrument). The USB shield then becomes a fault path. Mitigations: run the bench from its own battery, isolate USB (ADuM3160/4160 class for full-speed serial devices; ISOUSB211 class for high-speed devices such as logic analyzers and cameras), isolate I2C/RS485 links, and use isolated DC-DC converters for sensor heads.
- Full-speed isolators will not pass high-speed (480 Mbps) devices; an fx2lafw analyzer or UVC camera behind an ADuM3160 will fail to enumerate or fall back, hence the two isolator classes.
- Suggested human-only list: attaching or moving any probe/clamp on mains or the battery bank; opening panels and enclosures; anything inside the inverter or PinePower mains side; insulation (megger) tests; enabling a supply or load above a low-energy threshold (for example more than 30 V or more than a few amps) without confirmation; changing BLE mode on the Pinecil; firmware flashing of instruments; overriding an interlock.
- Suggested agent-allowed list: reading all sensors; driving the calibration bus; setting a current-limited bench supply inside a configured envelope; setting Pinecil temperature within a cap and only while the human has acknowledged presence; logic/analog capture on low-voltage boards.
- For 120 V measurement, UL-listed enclosed devices with CTs (Shelly Pro EM-50) installed once by the human are safer than exposed modules such as the PZEM-004T, which has mains on open screw terminals and should be boxed and fused.

### Gaps
- No standards text (IEC 61010 category definitions) or authoritative guidance on DC battery-bank measurement safety was fetched; the reasoning above is the researcher's engineering judgement.
- No verified prices or isolation ratings for ready-made ADuM3160/ADuM4160 or ISOUSB211 dongles; TI's EVM page shows no public price.
- CAT-rating honesty of UNI-T/Aneng/Zoyi meters (teardown evidence on fuses and clearances) was not researched.

---

## Ideas we may not have thought of (electrical sensing)

### Takeaway
The cheapest gains come from treating existing equipment and physical side-channels as instruments: USB-PD negotiation as a programmable source, the Pinecil and battery/BMS telemetry as cross-check channels, and audio-rate or RF sampling for power quality and arc detection.

### Cited Findings
- The Pinecil's live data already includes DC input voltage, estimated watts and tip resistance, readable over BLE — [IronOS docs](https://ralim.github.io/IronOS/Bluetooth)
- The KM003C exposes bus current/voltage to Linux through a kernel hwmon driver, so it appears as standard sysfs sensors without user-space protocol code — [kernel.org](https://docs.kernel.org/hwmon/powerz.html)
- Custom Rust firmware for the KM003C is under development alongside the protocol research — [km003c-protocol-research](https://github-users.yhajoa.workers.dev/okhsunrog/km003c-protocol-research)
- `ruiden` can flash Riden firmware from the command line, which opens the door to agent-managed firmware state on the supply — [docs.rs](https://docs.rs/crate/ruiden/latest)
- Glasgow's reconfigurable FPGA applets cover unusual or obsolete interfaces from the command line — [1BitSquared](https://1bitsquared.com/products/glasgow-c)

### Inferences
Researcher's ideas (unsourced):
1. **USB-PD as an instrument bus.** An I2C PD-sink controller lets the Pi request specific voltages from the PinePower (or any PD/PPS charger), giving a coarse programmable source for free; an in-line KM003C or INA228 closes the loop.
2. **Existing power-system telemetry as instruments.** BMS cell voltages, inverter/charger readings and battery-monitor data are independent measurements of the same bank; the agent can cross-check them against its own INA228 and flag a drifting BMS channel or shunt.
3. **In-place battery impedance tracking.** Correlate voltage and current steps during normal load changes to trend pack internal resistance over months, per pack if per-pack currents are available.
4. **Audio-interface power-quality digitiser.** A small AC-AC transformer (isolated voltage replica) plus a CT into a 24-bit USB audio interface or I2S ADC gives 48-192 kHz two-channel capture for THD, flicker, sag and frequency tracking at negligible cost.
5. **Bus ripple spectroscopy.** AC-couple the 48 V bus into an isolated ADC and watch the spectrum: MPPT and inverter switching lines, 120 Hz ripple amplitude and its change over time are health indicators for capacitors and connections.
6. **Connection-resistance mapping.** Millivolt drop across each lug, breaker and fuse under known current (from the shunt) gives micro-ohm-level joint resistance; combine with thermal imaging to catch loose terminations before they heat.
7. **Fuse voltage-drop method for vehicles.** Measuring millivolts across blade fuses identifies parasitic drains circuit by circuit without pulling fuses; easily scripted with the bench DMM and a lookup table.
8. **CAN/OBD-II via SocketCAN.** A gs_usb/candleLight adapter makes vehicle bus data a native Linux interface; pair with cranking-voltage waveform capture for relative-compression and starter health checks.
9. **Arc and EMI sniffing.** A near-field probe or short antenna into an RTL-SDR, or a simple AM-band detector, picks up series-arc and noisy-converter signatures; an ultrasonic MEMS microphone adds an acoustic channel for arcing and coil whine.
10. **Insulation/ground-fault monitor for the floating DC system.** A switched high-value resistor bridge from each rail to earth, read by an isolated ADC, estimates leakage to ground continuously, in the manner of EV isolation monitors.
11. **Non-contact live detection as an interlock.** A capacitive voltage sensor near the work area lets the agent refuse to authorise "safe to touch" prompts when a conductor is energised.
12. **Printed clip-on magnetic current sensor.** A hall or fluxgate IC in a 3D-printed split clamp around a battery cable gives contactless DC current for quick surveys, calibrated against the shunt by the agent.
13. **Temperature points on the bus.** Thermocouple/RTD channels on lugs and breakers provide continuous data between thermal-camera sessions and act as emissivity-independent references for the camera.
14. **Time-domain reflectometry.** A fast edge from a Pico GPIO plus the logic analyzer or scope locates cable faults and measures cable length on low-voltage runs.
15. **Golden-signature regression.** Store reference captures (startup rails, inrush, ripple spectra, thermal images) for known-good equipment so the agent can diff future captures instead of interpreting from scratch.
16. **Uniform instrument contract.** Give every DIY head the same small protocol: identity, firmware hash, self-test, calibration constants with date, temperature, and error counters, so the MCP layer treats bought and built instruments alike.

### Gaps
- None of these ideas was validated against prior art or costed this session. Whether either PinePower supports PPS is unknown.
