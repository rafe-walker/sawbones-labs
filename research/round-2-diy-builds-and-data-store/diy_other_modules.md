# Ground-up / hobby-built alternatives for the non-RF, non-multimeter modules of the agent-operated sensor pack (state: 25 Nov 2026)

Reading conventions used throughout:
- "Cited Findings" = confirmed on a page fetched in this session (source and page date given).
- Prices tagged **[UNVERIFIED-EST]** are the researcher's prior-knowledge estimates of AliExpress/Amazon street prices (2025 to early 2026). AliExpress/Amazon/eBay listing pages could not be fetched reliably in this session, so they are NOT confirmed Nov 2026 prices. They live only under "Inferences" or "Gaps".
- Prices tagged **[PLAN]** are the figures from the owner's first plan, not re-verified.
- Each module class ends with the requested three-column comparison (bought / ground-up / verdict, effort hours, main risk).

---

## 1. Vehicle interface from scratch (K-line ISO 9141-2 / KWP2000, CAN/TWAI, J1850)

### Takeaway
For a 2003 Tacoma (K-line only) a home-built ESP32 + L9637D adapter is a real saving (roughly USD 15-20 of parts versus USD 97 landed for a WiCAN Pro) and MIT-licensed Arduino/ESP32 firmware already exists; no embassy/esp-hal Rust K-line implementation was found, so the Rust port is new work. J1850 is the part not worth building.

### Cited Findings
- WiCAN Pro is listed at USD 89 plus USD 8 US shipping (= the plan's USD 97), status "available for pre-order"; it supports ISO 9141-2, ISO 14230-4 slow/fast, J1850 VPW and PWM, ISO 15765-4 CAN, J1939, raw CAN, MS-CAN, SW-CAN; an unnamed "dedicated OBD interpreter chip" does the protocols while an ESP32-S3 runs open-source firmware; it cannot be powered from USB-C, only from the OBD port — [Crowd Supply, fetched 25 Nov 2026](https://www.crowdsupply.com/meatpi-electronics/wican-pro)
- muki01/OBD2_K-line_Reader (MIT, 295 commits) runs on Arduino and ESP32 (also lists ESP8266, STM32, Pico), supports L9637D, MC33290, Si9241 and SN65HVDA195 transceivers plus an LM393 comparator circuit and a discrete-transistor circuit; implements ISO 9141-2 5-baud init, KWP2000 slow and fast init with auto-detect; reads live data, DTCs, freeze frame, VIN; has a Wi-Fi web-server build with OTA; companion OBD2_KLine_Library and a CAN reader/library exist. No tested vehicles are named and no parts cost is given — [GitHub muki01/OBD2_K-line_Reader](https://github.com/muki01/OBD2_K-line_Reader)
- iwanders/OBD9141 (MIT, 2015-, 72 commits) supports MC33290, SN65HVDA100 and SN65HVDA195 ("worked without problems" with the datasheet circuit), ISO 9141-2 and KWP slow init, has a `reader_esp32` example, and was "tested on one Kia car build in 2004" — no Toyota listed — [GitHub iwanders/OBD9141](https://github.com/iwanders/OBD9141)
- Toyota compatibility list: Tacoma 2000 and 2002 and Tundra 2000/2001/2003 are listed as "ISO 14230-4, ISO 9141-2"; summary says 1999-2003 Toyota = ISO 9141; pin 7 = K-line (SIL), pin 15 = L-line on earlier models, pin 16 = +12 V, pin 4 chassis ground, pin 5 signal ground; the list is stated to be incomplete — [pinouts.ru Toyota/Lexus OBD-II](https://pinouts.ru/CarElectronics/Toyota_lexus_obd2_diagnostic.shtml)
- Real-world K-line pitfall (2006 Honda Accord, Arduino Nano, OBD9141, discrete-transistor interface): 5-baud init succeeded but the ECU ignored back-to-back requests until a 50 ms delay was added between PID requests; a TJA1021 (LIN) transceiver's dominant timeout can block the 5-baud init whereas the L9637D reportedly has no such limit; a divider on RX was suggested for 3.3 V MCUs — [Arduino Forum, Oct 2024 to Jan 2025](https://forum.arduino.cc/t/struggling-with-k-line-iso-9141-2-communication/1307351)
- A BMW E36 builder using L9637D + ESP32-C3 SuperMini could not wake the ECU; one replier claimed the ESP32 needs a level shifter with the L9637D; no confirmed resolution on page 1 — [Arduino Forum, Aug 2025](https://forum.arduino.cc/t/problem-with-l9637d-and-esp32c3-supermini-or-arduino-nano/1402658)
- Rust: `automotive_diag` (MIT/Apache, v0.1.10, 10 Jan 2025, about 3.5k downloads/month) provides `no_std` structs/enums for UDS, KWP2000 and OBD-II but only definitions, no K-line init or transport — [lib.rs automotive_diag](https://lib.rs/crates/automotive_diag)
- esp-hal ships a TWAI (CAN) driver module (`src/twai/mod.rs` present in esp-hal 1.1.1; older per-chip HALs had `examples/twai.rs`) — [docs.rs esp-hal 1.1.1 twai](https://docs.rs/crate/esp-hal/1.1.1/source/src/twai/mod.rs), [esp32c3-hal twai example](https://docs.rs/crate/esp32c3-hal/latest/source/examples/twai.rs)
- For the JBD UP16S battery CAN bus: dbus-serialbattery supports only the Victron CAN dialect (verified with UP16S015 on CAN1, type-A cable) and recommends RS-485 over CAN because RS-485 exposes more detail such as per-cell voltages — [dbus-serialbattery docs](https://mr-manuel.github.io/venus-os_dbus-serialbattery_docs/next/general/connect)

### Inferences
- The 2003 Tacoma 5VZ-FE fits the "1999-2003 = ISO 9141" row, so a single K-line transceiver on pin 7 plus pins 4/5/16 is all the hardware needed; no L-line, CAN or J1850 hardware is required for this truck.
- The "ESP32 needs a level shifter" forum claim is contradicted in practice by muki01's ESP32 support with the L9637D; treat it as unconfirmed and check the L9637D datasheet VCC range before layout. A 3.3 V-native part (SN65HVDA195, also supported by both libraries) removes the doubt.
- Ground-up BOM **[UNVERIFIED-EST]**: ESP32-C3 SuperMini USD 2.50-3.50; L9637D USD 0.50-1.50; 510 ohm pull-up, series diode, TVS, polyfuse USD 1; 12 V to 5 V buck module USD 1-2; J1962 male plug with pigtail USD 4-6; SN65HVD230 CAN module USD 1-2 (adds newer cars and the JBD CAN bus); perfboard and printed shell USD 1-2. Total about USD 12-18.
- A Rust embassy port is tractable: bit-bang the 5-baud address byte (0x33) on a GPIO, switch the pin to UART at 10400 baud 8N1, discard the half-duplex echo, and honour inter-byte/inter-message timing (the cited 50 ms gap). No existing embassy/esp-hal implementation was found, so this would be an original, publishable crate; `automotive_diag` can supply the PID/service enums.
- J1850 VPW/PWM: the WiCAN Pro delegates this to a dedicated interpreter chip, which is a hint about effort. Dedicated J1850 transceiver ICs are obsolete/hard to source (researcher's prior knowledge, unverified); DIY needs discrete comparator/driver stages and tight bit timing. Not needed for the Tacoma.

| Bought part | Ground-up build | Verdict |
|---|---|---|
| WiCAN Pro USD 89 + 8 shipping (cited); vLinker USD 37 [PLAN] | ESP32-C3 + L9637D (or SN65HVDA195) + SN65HVD230 + J1962 pigtail, about USD 12-18 [UNVERIFIED-EST]; firmware from muki01 (MIT) or OBD9141 (MIT) first, Rust port after | **Build** for K-line + CAN. 6-10 h with existing Arduino firmware; 20-40 h more for an embassy Rust port. Main risk: 12 V transients/ground offsets killing the MCU, and Toyota init quirks with no named Toyota test in either library. **Buy** only if J1850 (pre-2008 GM/Ford) coverage is wanted. |

### Gaps
- No documented example was found of a 1996-2004 Toyota being read with a home-built K-line adapter (searches of GitHub/Arduino forum returned Kia, Honda, BMW cases only); Tacoma World threads were not reachable via search.
- vLinker MC+ current price not verified.
- AliExpress prices for L9637D, K-line breakout boards and J1962 pigtails not verified for Nov 2026.
- Could not confirm whether any ESP32 ELM327-emulator project (so standard OBD apps can talk to the DIY adapter) supports K-line; not reviewed.
- `ecu_diagnostics` and `rustbucket` Rust crates appeared in search results ([docs.rs](https://docs.rs/ecu_diagnostics/), [GitHub](https://www.github.com/KoffeinFlummi/rustbucket)) but were not reviewed; their `no_std`/K-line status is unknown.

---

## 2. Serial bus adapters from scratch and the "universal solar comms pod"

### Takeaway
As a plain USB-to-isolated-RS485 adapter, DIY saves only a few dollars against a USD 19 Waveshare and isolation is mandatory, so buy one as a reference; the real win is a Wi-Fi/ESP-NOW pod (ESP32 + MAX3232 + RS-485 transceiver, about USD 10-15) for which Apache-2.0 ESPHome components already speak the SMG-II inverter family and JBD BMS protocols.

### Cited Findings
- syssi/esphome-smg-ii (Apache-2.0, 102 commits): Modbus RTU over RS232 at 9600 baud; hardware = ESP32 or ESP8266 + MAX3232CSE RS232-TTL module + half an Ethernet cable; RJ45 pin 1 = TX, pin 2 = RX, pin 4 = 12 V VCC, pin 8 = GND (T-568B colours); supported list names ISolar/EASUN SMG II, SMG III 6.2 kW, PowMr POW-HVM5.5K-48V, Qoltec 53963, Anenji units — **EG4 3000EHV-48 and SUMRY are not listed**; several registers are read/write (output priority, charge current, thresholds, remote switch) — [GitHub syssi/esphome-smg-ii](https://github.com/syssi/esphome-smg-ii)
- Solar Assistant documents the EG4 3000EHV-48 as connecting via RS232 cable or Wi-Fi dongle and "connects as: sumry" — [Solar Assistant](https://solar-assistant.io/help/inverters/eg4/3000-EHV-48)
- syssi/esphome-jbd-bms is described as an "ESPHome component to monitor and control a JBD-BMS via UART-TTL or BLE" — [GitHub syssi/esphome-jbd-bms](https://www.github.com/syssi/esphome-jbd-bms)
- dbus-serialbattery: has a dedicated `LltJbd_Up16s` BMS type with daisy-chain addressing from 0x01; a separate adapter per BMS gives about 1 s refresh, master-only gives about 50 s; **the data link must be isolated** or current can flow through the data cable when the BMS opens the negative pole, damaging both ends; shielded cable and 120-130 ohm termination recommended beyond 10 m — [dbus-serialbattery docs](https://mr-manuel.github.io/venus-os_dbus-serialbattery_docs/next/general/connect)
- Heliograph (posted 27 Aug 2026): ESP32 C++/PlatformIO firmware speaking inverter-native protocols (Modbus RTU for Sungrow, Deye, Solis, Sofar, Growatt, Huawei, GoodWe; SunSpec; legacy framings), outputs MQTT/HA discovery, Modbus TCP, REST, Prometheus; read-only by design; reference hardware is a Waveshare ESP32-S3-RS485-CAN board at about EUR 25; new Modbus devices are added as data files — [Home Assistant Community](https://community.home-assistant.io/t/heliograph-esp32-firmware-that-speaks-your-inverters-native-protocol-mqtt-ha-modbus-tcp-rest-prometheus-looking-for-testers/1022820)
- `modbus-mcp` (kukapay) already exposes Modbus data to AI agents over MCP — [awesome-mcp-hardware](https://github.com/beriberikix/awesome-mcp-hardware)

### Inferences
- A battery- or inverter-powered wireless pod is galvanically isolated by construction (no copper back to the Pi), which satisfies the dbus-serialbattery isolation warning without an isolator IC; if the pod is USB-tethered, an isolated transceiver (ISO3082/ADM2587E class) or isolated module becomes mandatory.
- The SMG-II RJ45 carries 12 V on pin 4 (cited), so an inverter pod can self-power through a small buck converter; whether the EG4 3000EHV-48 RJ45 matches that pinout is unverified.
- EG4 3000EHV-48 = "sumry" in Solar Assistant while syssi's component targets SMG-II; the owner's stated SUMRY/SMG-II family mapping means the register map probably overlaps but must be proven on the real inverter (read-only first).
- Pod BOM **[UNVERIFIED-EST]**: ESP32-C3/C6 SuperMini USD 3-5; MAX3232 module USD 0.50-1; auto-direction RS-485 TTL module USD 0.50-1 (non-isolated) or isolated module USD 3-8; RJ45 breakout USD 1; buck module USD 1; printed shell. Total about USD 8-15 per pod.
- Because the Modbus maps are data, this is the most agent-tweakable module in the pack: Heliograph's "device = data file" pattern maps directly onto the project's manifest idea.

| Bought part | Ground-up build | Verdict |
|---|---|---|
| Waveshare isolated USB-RS232/485 USD 19 each [PLAN]; Waveshare ESP32-S3-RS485-CAN about EUR 25 (cited) | ESP32-C3/C6 + MAX3232 + RS-485 module + RJ45, about USD 8-15 [UNVERIFIED-EST], ESPHome first (syssi, Apache-2.0), Rust later | **Build** the wireless pods (4-8 h each on ESPHome; 15-30 h for a Rust Modbus RTU pod). **Buy** one isolated Waveshare USB adapter as the known-good reference. Main risk: non-isolated wiring across a 48 V battery/BMS; writing inverter registers. |

### Gaps
- No confirmed register map or community firmware specifically for the EG4 3000EHV-48 was found; DIY Solar Forum threads were not reachable.
- Whether syssi/esphome-jbd-bms covers the UP16S rack RS-485 protocol (versus the classic JBD UART protocol) was not confirmed; only dbus-serialbattery's `LltJbd_Up16s` driver is confirmed.
- Prices of isolated RS-485 modules and of ISO3082/ADM2587E/CA-IS3082 parts not verified.
- esp-link and "solar-bridge"-style projects were not reviewed.

---

## 3. Per-port power-switched USB hub: cheaper or home-built?

### Takeaway
The USD 50 Plugable USB3-HUB7BC is confirmed uhubctl-compatible and is the cheapest per-port option found; the open "smart" hubs cost far more per port (USD 139 for 3 ports, EUR 60 for 3 ports). Building a USB 3 hub is false economy, but a small home-built VBUS load-switch + current-sense board is a cheap, high-value add-on because no cheap hub reports per-port current.

### Cited Findings
- uhubctl compatibility table includes Plugable USB3-HUB7BC (7-port USB 3.0, 2109:0813, 2015), Plugable USBC-HUB7BC (works on 6 of 7 ports), AmazonBasics HU3641V1 (4-port), HU3770V1 (7-port), HU9003V1EBL (7-port), HU9002V1 (10-port), Rosonway RSH-518C (7-port), RSH-A37S (7-port), RSH-A10 (10-port), j5create JUH377; some listed hubs only partly work (Plugable USB3-HUB7C: only 2 charge ports; AmazonBasics U3-7HUB: 1 port; Rosonway RSH-ST07C: 4 ports) — [uhubctl README](https://github.com/mvp/uhubctl/)
- Raspberry Pi 5 onboard ports advertise per-port switching but are ganged: VBUS only drops when all four are off; Pi 4B is also ganged — [uhubctl README](https://github.com/mvp/uhubctl/)
- Some hubs cut data but not VBUS; the README says to test with a phone/USB lamp and names failures (Bytecc BT-UH340, Rosonway RSH-A13, RSH-A16); a USB 3 hub appears as two virtual hubs and both must be switched — [uhubctl README](https://github.com/mvp/uhubctl/)
- USB Insight Hub: USD 139 (+USD 8 US shipping), in stock; 3 downstream USB 3.0 ports, per-port power and separate D+/D- switching, per-port voltage/current metering at 1 mV / 1 mA resolution, programmable over-current 0-2000 mA, three 1.3 inch displays, ESP32-S3, JSON serial API and Wi-Fi web UI, STEMMA QT port, described as open source — [Crowd Supply, fetched 25 Nov 2026](https://www.crowdsupply.com/aerio-solutions/usb-insight-hub)
- YKUSH: EUR 59.99, 3 downstream USB 2.0 ports with individual VBUS switching, HID control with Linux/Windows CLI, open hardware and open firmware/software — [Yepkit](https://www.yepkit.com/products/ykush)
- DIY precedent: Jim Heaney's I2C-controlled USB 2.0 hub puts an MT9700 load switch on each port, driven by a PCA9557 I2C expander, with selectable 500 mA / 1 A limit; no current sensing — [Hackaday, 11 Sep 2023](https://hackaday.com/2023/09/11/flip-the-switch-on-this-i2c-controlled-usb-hub), repo [JimHeaney/i2c-usb-hub](https://github.com/JimHeaney/i2c-usb-hub)
- A 2026 Fab Academy modular cyberdeck design puts per-module power switching and module identity checks on a pogo-pin backplane (design only, not built) — [Fab Academy 2026](https://fabacademy.org/2026/labs/barcelona/students/pedro-rocha/projects/final-project/)

### Inferences
- Per-port cost: Plugable about USD 7/port; YKUSH about EUR 20/port; USB Insight Hub about USD 46/port. Only the Insight Hub gives the agent current/voltage telemetry and a documented JSON API.
- Nearly every planned module (ESP32/RP2040 full-speed devices, serial adapters, GNSS, mic array) is USB 2.0. A home-built **USB 2.0** backplane with load switches (TPS2041/AP2553/MT9700 class) and INA219/INA3221 sensing controlled by an RP2040 is feasible on a 2-layer board; a USB 3 hub (hub IC sourcing, 5 Gbit/s routing, 4-layer) is not worth it.
- Cheapest credible path: keep a uhubctl-listed hub for data and build a "power sidecar" (inline VBUS switch + current sense per port, RP2040, self-identifying module) for about USD 8-15 **[UNVERIFIED-EST]**. That gives the agent what uhubctl cannot: per-port current, inrush traces, brown-out detection and a hard cut independent of the hub's own firmware.
- Hub ICs with true per-port power pins (for example Microchip USB2514B) should enumerate as per-port-switchable to uhubctl if wired with real load switches (researcher's prior knowledge; unverified).

| Bought part | Ground-up build | Verdict |
|---|---|---|
| Plugable USB3-HUB7BC USD 50 [PLAN], confirmed on uhubctl list; USB Insight Hub USD 139 / 3 ports; YKUSH EUR 59.99 / 3 ports | RP2040 + per-port load switch + INA219/INA3221 sidecar, about USD 8-15 [UNVERIFIED-EST]; full DIY USB 2.0 hub PCB about USD 25-40 incl. boards | **Buy** the hub (or a cheaper uhubctl-listed 4/7-port, after a VBUS test). **Build** the current-sense/VBUS sidecar (8-15 h). Main risk: back-powering through data lines and hubs that claim switching but leave VBUS live. |

### Gaps
- Nov 2026 prices for AmazonBasics/Rosonway uhubctl-listed hubs not verified; several listed models are a decade old and availability is unconfirmed.
- YKUSH3 and YKUSHXS prices not shown on the fetched page; Cleware, Acroname and SwitchDoc USB PowerControl not reviewed.
- USB Insight Hub repo URL and licence not captured.

---

## 4. Compute cheaper than the Pi 5

### Takeaway
The plan's USD 110 looks high: Raspberry Pi 5 4 GB list price was USD 85 on 2 Oct 2026, and there are credible sub-USD 60 brains (Pi 5 1 GB USD 45, Pi 4 1 GB USD 35, used Dell Wyse 5070 USD 40-70, Pi Zero 2 W USD 15), but the 4 GB Pi 5 at USD 85 remains the right place to spend for the portable pack.

### Cited Findings
- Raspberry Pi list prices on 2 Oct 2026: Pi 5 16 GB USD 220, 8 GB USD 130, **4 GB USD 85**, 2 GB USD 77.50 (up from USD 65 on 1 Oct 2026), 1 GB USD 45; Pi 4 8 GB USD 115, 4 GB USD 75, 3 GB USD 83.75, 2 GB USD 67.50, 1 GB USD 35 — [Tom's Hardware, 2 Oct 2026](https://www.tomshardware.com/raspberry-pi/component-shortages-drive-raspberry-pi-prices-up-by-up-to-23-percent-escalating-lpddr4-lpddr5-costs-trigger-the-third-price-hike-of-the-year); 2 GB rises confirmed by [OMG! Ubuntu, 2 Oct 2026](https://www.omgubuntu.co.uk/2026/10/raspberry-pi-2gb-costs-how-much-now)
- February 2026 round added USD 10 (2 GB), 15 (4 GB), 30 (8 GB), 60 (16 GB); 1 GB models unchanged; Zero, Pi 3 and older products not expected to change because they use stockpiled LPDDR2 — [Raspberry Pi news, 2 Feb 2026](https://raspberrypi.com/news/more-memory-driven-price-rises)
- Pi Zero 2 W cited at USD 15 "if in stock"; Pi 5 1 GB under USD 50; an N100 mini PC kit rose from USD 159 (Mar 2025) to USD 246.99, matching a fully kitted Pi 5 16 GB; author expects 2026 to favour reusing used hardware — [Jeff Geerling, 5 Jan 2026](https://www.jeffgeerling.com/blog/2026/raspberry-pi-cheaper-than-mini-pc/)
- SBC inflation April 2026 versus launch: Pi 5 8 GB +119%, Banana Pi BPI-M4 Zero 2 GB USD 28.90 to USD 115, Orange Pi 5 Ultra 16 GB USD 125 to USD 309, Radxa ROCK 5B+ 8 GB USD 90 to USD 129.99 (sold out), FriendlyELEC NanoPi Zero2 1 GB unchanged at USD 18 — [CNX Software, 28 Apr 2026](https://www.cnx-software.com/2026/04/28/what-a-difference-two-years-make-comparing-sbc-prices-in-2024-and-2026/)
- Used thin clients (prices as of Feb 2026): Dell Wyse 5070 (Pentium J5005, 4-8 GB, 2x USB 2.0 + 4x USB 3.1) USD 40-70, about 5-7 W idle; HP t640 (Ryzen R1505G, 8 GB, 2x USB 2.0 + 4x USB 3 + USB-C) USD 50-80, about 6-8 W idle; HP t630 USD 30-60, about 10-14 W idle — [selfhosting.sh, 4 Mar 2026](https://selfhosting.sh/hardware/refurbished-thin-clients/)
- Pi 5 onboard USB ports cannot be power-switched individually — [uhubctl README](https://github.com/mvp/uhubctl/)

### Inferences
- The Pi 5 2 GB (USD 77.50) is poor value next to the 4 GB (USD 85). The meaningful cheaper steps are Pi 5 1 GB (USD 45) or Pi 4 1 GB (USD 35); a lean Rust MCP server fits in 1 GB, but camera pipelines plus multi-channel audio DOA plus logging would be tight.
- Alternative-brand SBCs have inflated as much or more than the Pi and bring vendor-kernel friction; with DRAM pricing as it is, there is no clear "cheap clone" win in the 2-4 GB class. Orange Pi Zero 3, Radxa Zero 3W/3E, Rock 3C, Milk-V Duo S, Luckfox Pico and BeagleY-AI prices could not be confirmed for Nov 2026.
- A used Wyse 5070 (x86, six USB ports, mainline kernel, 5-7 W) is the best sub-USD 60 brain for the **fixed** bench; it needs a 19 V supply and is bulkier, so it is a poor fit for the carry case.
- From the owner's stored gear notes (memory, not the task brief): he owns a spare Home Assistant Yellow and three PiKVM V4 Minis, all CM4-based. If the spare Yellow carries a CM4, that is a zero-cost brain for prototyping the MCP server before buying a Pi 5 (suitability of the Yellow carrier under plain Raspberry Pi OS unverified).
- ESP32-P4 as a no-Linux brain removes the uhubctl/libcamera/ALSA ecosystem the design leans on; reasonable as a pod, not as the host.

| Bought part | Ground-up / cheaper option | Verdict |
|---|---|---|
| Pi 5 4 GB: USD 110 [PLAN] versus USD 85 list (cited, 2 Oct 2026) | Pi 5 1 GB USD 45; Pi 4 1 GB USD 35; Zero 2 W USD 15; used Wyse 5070 USD 40-70; reuse an owned CM4 device USD 0 | **Buy** the Pi 5 4 GB at list (about USD 85) for the pack; consider a used thin client for the fixed bench. 0 h build effort. Main risk: paying kit/scalper pricing, or losing days to vendor BSP kernels on a "cheaper" board. |

### Gaps
- Could not confirm why the plan shows USD 110 (kit, reseller markup, or a price move after 2 Oct 2026); no post-October price change was found.
- Nov 2026 prices and mainline/Rust support notes for Orange Pi Zero 3/3B/5, Radxa Zero 3W/3E, Rock 3C, Milk-V Duo S, Luckfox Pico, BeagleY-AI not obtained (one price roundup failed to load).
- Dell 3040/3050 micro prices and power draw not obtained.

---

## 5. Audio: home-built multi-mic array versus ReSpeaker XVF3800

### Takeaway
For an agent that wants raw, sample-synchronous, wide-band channels, a USD 15-25 home-built I2S MEMS array on an RP2040 (PIO) or ESP32-S3 (TDM/dual I2S) is a credible match for capture and beats the ReSpeaker on flexibility, while losing its tuned voice DSP; open DOA software (ODAS, MIT) accepts arbitrary array geometries.

### Cited Findings
- reSpeaker XVF3800 (44 mm variant documented): four PDM MEMS mics; stock USB and I2S firmware are 48 kHz, 2-channel processed streams ("conference" and "ASR"); raw mic signals can be routed onto an output channel via `AUDIO_MGR_OP_R` and are quieter because they bypass enhancement; DOA angle (0-359 degrees) readable over USB or I2C — [Seeed wiki](https://wiki.seeedstudio.com/respeaker_44mm_introduction/)
- RP2040 PIO microphone-array library: captures a DFRobot "6+1" array through four data lines sharing one clock and one word-select line, with TinyUSB `usb_microphone_array` examples — [GitHub denisgav/microphone-array-library-for-pico](https://github.com/denisgav/microphone-array-library-for-pico)
- Espressif `usb_device_uac` component runs on ESP32-S2/S3/P4 with configurable mic/speaker channel counts and sample rates (example: 48 kHz, 16-bit mono); no maximum channel count documented — [ESP-IoT-Solution docs](https://docs.espressif.com/projects/esp-iot-solution/en/latest/usb/usb_device/usb_device_uac.html)
- TDM: ESP-IDF v5 I2S driver supports TDM on ESP32 and ESP32-S3 (S3 lacks the original ESP32's slot/BCLK limits); BCLK = rate x slots x bits (8 x 32-bit at 48 kHz = 12.288 MHz, near some MCU limits; 8 x 32-bit at 16 kHz = 4.096 MHz); all mics need the same BCLK/WS with minimal skew; a finger-snap equidistant from all mics is the suggested sync test; PDM mics need a PDM-to-TDM bridge or multiple PDM inputs — [Applied-EE embedded notes](https://applied-ee.github.io/embedded/docs/audio-projects/i2s-audio-pipelines/tdm-multichannel-audio/)
- ODAS (MIT, C) does sound-source localisation, tracking, separation and post-filtering on low-cost embedded hardware, with config files for arbitrary arrays (IntRoLab's own 8- and 16-input USB arrays are the reference) — [GitHub introlab/odas](https://github.com/introlab/odas)
- PS3 Eye: four-mic linear array usable on Linux via ALSA as 4 channels; cited at about USD 7.50 on Amazon in 2019; directional (not 360 degrees) and "don't expect wonders" — [CNX Software, 30 Aug 2019](https://www.cnx-software.com/2019/08/30/using-sony-ps3-eye-camera-as-an-inexpensive-microphone-array/)

### Inferences
- INMP441-class parts are plain I2S (two mics per data line via the L/R pin), so 4-6 mics need 2-3 data lines on a shared BCLK/WS. That is exactly what the RP2040 PIO approach does and is the simplest route to guaranteed sample-synchronous capture; on ESP32-S3, true TDM mics or two I2S ports sharing clocks are needed.
- USB full-speed isochronous payload tops out near 1 MB/s (1023 bytes per 1 ms frame; USB 2.0 spec, researcher's prior knowledge). That allows 8 ch x 48 kHz x 16-bit (768 B/ms), 4 ch x 96 kHz x 16-bit (768 B/ms) or 2 ch x 192 kHz x 16-bit, so an RP2040/ESP32-S3 can carry a 6-mic 48 kHz array or a 2-mic ultrasonic pair, but not 6 mics at 96 kHz; beyond that, stream over SPI/Ethernet or use a high-speed USB part (ESP32-P4, RP2350 with external PHY).
- BOM **[UNVERIFIED-EST]**: 6x INMP441 modules USD 9-15, RP2040-Zero or Pico USD 3-5, printed ring/line holder and wiring USD 2-3: about USD 15-25. ICS-43434 (flatter response) roughly doubles the mic cost. Ultrasonic work needs analogue/PDM parts (SPU0410, IM73A135, T3902) and a faster front end; treat as a later module.
- Honest comparison: DIY wins on raw synchronous channels, geometry freedom, sample-rate control and firmware the agent can modify; the XVF3800 wins on far-field speech (AEC, beamforming, noise suppression) with zero effort. If the pack's goal is diagnosing engine noises and locating sounds, raw channels matter more.
- A used PS3 Eye is the cheapest zero-effort 4-channel raw array and a good ODAS test source before building.

| Bought part | Ground-up build | Verdict |
|---|---|---|
| ReSpeaker XVF3800 USD 55 [PLAN]; PS3 Eye used about USD 5-10 (2019 figure cited; current price unverified) | 4-6x INMP441 + RP2040 (PIO, TinyUSB UAC) about USD 15-25 [UNVERIFIED-EST]; DOA with ODAS on the Pi | **Build** (10-20 h with C/TinyUSB example; more for Rust). Main risk: channel skew/slot misalignment and USB audio firmware debugging; INMP441 clone variability. |

### Gaps
- Whether embassy-usb offers a multichannel UAC microphone (capture) class in Nov 2026 was not checked; this decides whether the array can be pure Rust on day one.
- Existence of a 6-channel (raw + processed) XVF3800 firmware for the larger ReSpeaker board was not confirmed in this session.
- Current INMP441/ICS-43434/SPH0645/MSM261 and PS3 Eye prices not verified; pyroomacoustics and Acoular not reviewed; no measured noise-floor comparison between INMP441 arrays and the XVF3800 was found.

---

## 6. Cameras and cheap thermal

### Takeaway
Nothing here is worth building from the sensor up; the saving comes from buying UVC or clone modules. Phone-dongle thermal cameras (Topdon TC001, InfiRay P2 Pro) enumerate as standard UVC on Linux with raw thermal data and open-source viewers, which makes a used one the best step up from an MLX90640.

### Cited Findings
- InfiRay P2 Pro and Topdon TC001 (same hardware) are "picked up as a standard USB video device by Linux", exposing two streams, one with luminance-coded relative temperature and one carrying raw thermal data; open tools: PyThermalCamera (TC001), P2Pro-Viewer, Thermal-Camera-Redux — [Hackaday, 11 Jul 2023](https://hackaday.com/2023/07/11/phone-thermal-cameras-get-open-source-desktop-tools), [PyThermalCamera](https://github.com/leswright1977/PyThermalCamera), [P2Pro-Viewer](https://github.com/LeoDJ/P2Pro-Viewer)

### Inferences
- **[UNVERIFIED-EST]** street prices: ESP32-CAM (OV2640) USD 5-8; XIAO ESP32S3 Sense about USD 14; UVC autofocus modules (OV5640/IMX179) USD 10-25; USB endoscope USD 10-20; MLX90640 breakout USD 25-40 from marketplace sellers versus about USD 75 from Adafruit; AMG8833 USD 15-20; Topdon TC001 roughly USD 150-200 new.
- OV2640-class ESP32 cameras are fixed-focus and soft; adequate for "is the LED on / where is the probe" scatter cams, poor for reading SMD markings. Label and small-part reading wants autofocus plus a macro-capable lens: Camera Module 3 (autofocus, libcamera) at USD 29 [PLAN] or a UVC autofocus module are the credible low end; a USD 10-20 USB microscope/endoscope covers close-up work.
- AMG8833 (8x8) is too coarse to localise a hot component; MLX90640 (32x24) is the practical floor for an I2C module the agent can own end to end; a 256x192 UVC dongle is the practical floor for "real" thermal images.

| Bought part | Ground-up / cheaper option | Verdict |
|---|---|---|
| Camera Module 3 USD 29 [PLAN] | UVC autofocus module USD 10-25; ESP32-CAM USD 5-8 for scatter views [UNVERIFIED-EST] | **Buy** (Camera Module 3 or cheap UVC autofocus). 1-2 h. Risk: fixed-focus/low-quality lenses that cannot read labels. |
| MLX90640 USD 40 [PLAN] | Marketplace MLX90640 USD 25-40; used TC001/P2 Pro with open Linux tools [UNVERIFIED-EST] | **Buy cheap clone** MLX90640 module now; used UVC thermal later. 2-4 h driver work. Risk: marketplace boards with wrong FOV variant (A = 110 deg, B = 55 deg). |

### Gaps
- No Nov 2026 prices verified for any camera or thermal part, including used FLIR One / Seek Compact and Topdon/InfiRay clearance pricing.
- Open Linux driver status for FLIR One and Seek Compact not reviewed; HTPA 32x32 and Senxor/MI48 not reviewed.

---

## 7. Position/time cheaper than a USD 30 GNSS breakout

### Takeaway
An ATGM336H module is the honest cheap choice (about USD 4-6, unverified) and replaces the USD 30 breakout for position and time; cheap "u-blox NEO-6M/M8N" boards are mostly counterfeit, clone or relabelled silicon.

### Cited Findings
- "Most AliExpress 'u-blox' NEO-6M modules are counterfeit or refurbished, though they usually work"; the ATGM336H is suggested as an "honest-silicon alternative" — [atomic14, fake sensor guide](https://www.atomic14.com/esp32/sensors/fake-sensors/)
- Meshtastic developer reference: NEO-6 is the "most power hungry, least capable and lowest sensitivity module"; M8 supports GPS/GLONASS/Galileo/BeiDou/QZSS/SBAS with three concurrent; marketplace modules are mostly clones (Beitian, VKEL: can be functionally equivalent), counterfeits (real chip, substandard circuitry) or mislabels ("a 6010 with an M8 label"); "most modules seen on Amazon, eBay, Banggood, Temu or AliExpress are in one of the above three categories" — [Meshtastic GNSS modules](https://meshtastic.org/docs/development/reference/gnss-modules/)

### Inferences
- For this pack GNSS is a timestamp/location tag, not a survey instrument, so clone-grade accuracy is fine. The thing to check on a cheap board is whether PPS is broken out to a header pin (many route it only to an LED) so the Pi can discipline its clock with chrony/gpsd.
- **[UNVERIFIED-EST]**: ATGM336H with ceramic patch antenna USD 4-6; NEO-6M board USD 2-4; "NEO-M8N" board USD 6-12; active 28 dB patch antenna USD 3-5.

| Bought part | Ground-up / cheaper option | Verdict |
|---|---|---|
| GNSS breakout USD 30 [PLAN] | ATGM336H module + patch antenna USD 4-6 [UNVERIFIED-EST] wired to a C3/RP2040 or straight to Pi UART + PPS GPIO | **Buy cheap clone** (ATGM336H). 1-2 h. Risk: no PPS pin, weak passive antenna inside a case full of switching noise. |

### Gaps
- No verified Nov 2026 prices; no measured PPS jitter data for ATGM336H found in this session (an NTP-pool thread and a TAPR evaluation appeared in results but were not read).

---

## 8. Power from scratch

### Takeaway
A small ready-made 12 V LiFePO4 with built-in BMS (20 Ah, about 256 Wh, seen at USD 45-63 in 2025) is cheaper per Wh and far safer than a home-welded 18650/21700 pack; build the regulation, metering and safe-shutdown layer, not the battery.

### Cited Findings
- ECO-WORTHY 12 V 20 Ah LiFePO4 with BMS: USD 62.99 regular, USD 47.25 with coupon via Amazon (deal posted 12 Aug 2025, expired); commenters cite another 20 Ah brand at a normal USD 45 and a 15 Ah unit with display at USD 35 — [Slickdeals, 12 Aug 2025](https://slickdeals.net/f/18524746-eco-worthy-12v-20ah-lifepo4-lithium-battery-47-25-free-shipping)
- Tool-battery breakout adapters exist at about USD 10 (eBay listing title: "Dewalt 20V MAX Li-ion Batteries Convert DIY Output Adapter ... V2 $9.99") — [eBay store listing](https://www.ebay.com/str/poweruprc)

### Inferences
- 12.8 V x 20 Ah = 256 Wh, so the 120-200 Wh target is met by a 12-20 Ah unit at roughly USD 35-63 on 2025 pricing (Nov 2026 pricing unverified). That replaces the plan's USD 100 battery-plus-converters line with about USD 65-80 including converters and metering.
- A DIY 4S LiFePO4 or 3S/4S Li-ion pack needs matched genuine cells, a BMS board, nickel strip and a spot welder; cell cost alone approaches the finished battery's price. False economy and the one build with real fire risk.
- Tool-battery adapters (Ryobi/DeWalt/Milwaukee/Makita, roughly USD 10-20 each **[UNVERIFIED-EST]**) are attractive only if the owner already has packs; bare adapters generally have no low-voltage cutoff, so a cutoff module or supervised buck is required to avoid killing a pack.
- Pi 5 supply: the Pi 5 expects 5.1 V / 5 A and limits downstream USB current unless it negotiates a 5 A PD source or `usb_max_current_enable=1` is set (Raspberry Pi documentation; researcher's prior knowledge, not re-fetched). Since peripherals hang off a self-powered hub, a solid 5 V 5 A buck feeding the GPIO header/USB-C plus that config flag is the cheap route **[UNVERIFIED-EST]** USD 4-8.
- Agent-visible battery: an INA226/INA228 on the pack output (coulomb counting, about USD 1.50-5 **[UNVERIFIED-EST]**) on an RP2040 "power module" gives voltage, current, energy and a safe-shutdown signal; this is the part worth building and exposes the pack's own health as a first-class module.

| Bought part | Ground-up build | Verdict |
|---|---|---|
| 12 V LiFePO4 + converters USD 100 [PLAN]; 12 V 20 Ah LiFePO4 USD 45-63 (2025, cited) | DIY 18650/21700 pack + BMS; or tool-battery adapter about USD 10 (cited) + cutoff | **Buy** the LiFePO4 battery; **build** the buck/metering/shutdown module (6-10 h). Main risk: DIY cell packs (fire), tool packs without low-voltage cutoff, under-rated 5 V bucks browning out the Pi. |

### Gaps
- Nov 2026 Amazon prices for 12 V 6-20 Ah LiFePO4 not verified (2025 data only); battery prices may have moved with tariffs.
- Ryobi/Milwaukee/Makita adapter prices, PD trigger boards, IP2368/SW3518/IP5389 modules, MAX17048/BQ34Z100 gauges and specific 5 V 5 A buck modules proven with a Pi 5 were not verified.

---

## 9. Sensor breakouts from the cheap end, counterfeit risks, and a scatter-pod BOM

### Takeaway
Marketplace breakouts cut the sensor line by roughly 70-80% but several parts are routinely fake or mislabelled (DS18B20, BME280 sold as BMP280, NEO-6M, MPU6050, HX711, MH-Z19); the project's own identify/self-test service is the right defence, plus one known-good reference sensor.

### Cited Findings
- DS18B20: a 2019 study bought more than 1,000 probes/chips from over 70 marketplace vendors and found the "vast majority" fake/clone; genuine ROM pattern is 28-xx-xx-xx-xx-00-00-xx; fakes may fail parasite power, be noisy, exceed +/-0.5 C, lack EEPROM; test sketches at cpetrich/counterfeit_DS18B20 — [CNX Software, 13 Jul 2020](https://www.cnx-software.com/2020/07/13/fake-ds18b20-temperature-sensors-counterfeit-clones/), [repo](https://github.com/cpetrich/counterfeit_DS18B20)
- BME280 listings often ship BMP280 (no humidity): chip-ID register 0xD0 reads 0x60 for BME280, 0x58 for BMP280 — [atomic14](https://www.atomic14.com/esp32/sensors/fake-sensors/)
- Same guide: MPU6050 genuine silicon is end-of-life, boards are clones or recycled dies (fine for vibration, not orientation); HX711 clones strapped to 10 Hz with noisy regulators; MH-Z19 clones on black PCBs read low; ENS160/SGP30/CCS811/BME680 "eCO2" is an estimate, real CO2 needs NDIR/photoacoustic (SCD40/41, MH-Z19, S8); Sensirion SHT3x/4x mostly genuine with occasional reworked dies; recommends testing on arrival (I2C scan, chip ID, ROM dump) and keeping one known-good reference such as an SHT41 or SCD41 — [atomic14](https://www.atomic14.com/esp32/sensors/fake-sensors/)

### Inferences
- **[UNVERIFIED-EST]** marketplace single-unit prices (multi-packs lower): BMP280 USD 0.60-1; BME280 USD 2-3; BME688 USD 10-14; SHT40/41 USD 1.50-3; DS18B20 probe USD 1; MAX31855 USD 2.50-4; INA219 USD 1; INA226 USD 1.50; INA228 USD 3-5; LIS3DH USD 1.50; ADXL345 USD 1.20; MPU6050 USD 1; VL53L0X USD 1.50; VL53L1X USD 3.50-5; AS7341 USD 6-9; MLX90614 USD 5-8; HX711 USD 0.60 + load cell USD 2-4; MQ gas sensors USD 1-2; PMS5003 USD 12-15; SCD40 USD 12-15; SCD41 USD 15-20; VEML7700 USD 1.50-2.50. Typical Adafruit/SparkFun equivalents are USD 5-20 per the plan.
- Where the premium is worth paying: anything used as a calibration reference (one genuine SHT4x/SCD41, one genuine DS18B20 from a distributor), safety-relevant gas sensing, and parts where a wrong variant silently changes units (thermocouple amps, time-of-flight). Everywhere else buy cheap in 3-5 packs and let the manifest/self-test record chip ID and plausibility checks.
- Scatter pod BOM **[UNVERIFIED-EST]**: ESP32-C3 SuperMini USD 2.50-3.50; sensor USD 1-3; TP4056 USB-C charger/protection USD 0.40; 500-1000 mAh LiPo USD 3-5; switch, JST, wire USD 0.50; PETG clip (10-20 g) USD 0.30: about USD 8-12 per pod. A C3 SuperMini has a weak antenna on some batches (researcher's prior knowledge); ESP-NOW range should be tested.

| Bought part | Ground-up / cheaper option | Verdict |
|---|---|---|
| Adafruit/SparkFun breakouts USD 5-20 each [PLAN] | Marketplace breakouts USD 0.60-5 for most parts [UNVERIFIED-EST]; scatter pod about USD 8-12 | **Buy cheap clone** for most; **buy genuine** for 1-2 reference sensors. 1-2 h per sensor driver. Risk: fakes and mislabels (cited), mitigated by chip-ID self-test. |

### Gaps
- None of the per-part marketplace prices were verified for Nov 2026; tariff/de minimis changes may have raised small-parcel prices to the US.
- No source found quantifying counterfeit rates for INA219/INA226, VL53L0X, AS7341 or MLX90614.

---

## 10. Enclosure cheaper than USD 40

### Takeaway
The same Apache 3800 drops to USD 24.99 on Harbor Freight member/sale pricing, and the smaller 2800 runs USD 18-20 on sale; waiting for a sale beats any printed or plywood case on cost and ruggedness.

### Cited Findings
- Apache 3800 large case: USD 24.99 member price versus USD 40 regular (posted 7 Mar 2026) — [Slickdeals](https://slickdeals.net/f/19280394-apache-3800-weatherproof-protective-case-large-black-harbor-freight-members-only-price-24-99)
- Apache 2800 medium case: USD 17.99 sale ending 8 Mar 2026 (deal title) and USD 19.99 coupon in Oct 2024 — [Slickdeals](https://slickdeals.net/f/19280406-apache-2800-weatherproof-protective-case-medium-black-harbor-freight-sales-ends-mar-8-17-99), [Harbor Freight coupon, Oct 2024](https://go.harborfreight.com/coupons/2024/10/182977-58655)

### Inferences
- A printed-in-sections case on a 180 mm bed costs USD 15-25 in PETG for a much weaker, non-sealed shell; use the printer for the internal module grid instead. A Gridfinity-style module tray is roughly 30-60 g of PETG, about USD 0.50-1.20 each at USD 15-20/kg **[UNVERIFIED-EST]**; a full insert for an Apache 3800 is likely 300-500 g, USD 5-10.
- Used .50-cal ammo cans (USD 10-20 **[UNVERIFIED-EST]**) are tough but steel shields Wi-Fi/GNSS/ESP-NOW, a real problem for a wireless pack.

| Bought part | Ground-up / cheaper option | Verdict |
|---|---|---|
| Apache 3800 USD 40 regular (cited) | Same case on member/sale price USD 24.99 (cited); Apache 2800 USD 18-20 (cited); printed grid insert USD 5-10 | **Buy** the case on sale; **build** the printed grid. 4-8 h CAD/printing. Risk: outgrowing a 2800; RF shielding in metal cans. |

### Gaps
- Apache 1800 price and Nov 2026 regular prices not confirmed; surplus-case and Kydex/plywood build costs not researched.

---

## 11. Existing maker projects to learn from or fork

### Takeaway
No single project already combines a self-identifying module system with an agent-facing MCP host, but every layer has open precedent to borrow: K-line and solar firmware, DOA software, open switched hubs, and a fast-growing 2026 set of MCP-to-hardware servers that mostly lack the write-safety layer this project plans.

### Cited Findings
- Vehicle: muki01 OBD2 K-line reader/library and CAN siblings (MIT) — [GitHub](https://github.com/muki01/OBD2_K-line_Reader); OBD9141 (MIT) — [GitHub](https://github.com/iwanders/OBD9141); WiCAN Pro open firmware on ESP32-S3 (USD 89) — [Crowd Supply](https://www.crowdsupply.com/meatpi-electronics/wican-pro)
- Solar: syssi ESPHome components (SMG-II Apache-2.0; JBD, JK, Seplos, PACE) — [esphome-smg-ii](https://github.com/syssi/esphome-smg-ii), [esphome-jbd-bms](https://www.github.com/syssi/esphome-jbd-bms); Heliograph ESP32 multi-protocol inverter firmware (Aug 2026, read-only by design) — [HA Community](https://community.home-assistant.io/t/heliograph-esp32-firmware-that-speaks-your-inverters-native-protocol-mqtt-ha-modbus-tcp-rest-prometheus-looking-for-testers/1022820); dbus-serialbattery with UP16S driver — [docs](https://mr-manuel.github.io/venus-os_dbus-serialbattery_docs/next/general/connect)
- Audio: ODAS (MIT) — [GitHub](https://github.com/introlab/odas); RP2040 PIO mic-array library — [GitHub](https://github.com/denisgav/microphone-array-library-for-pico)
- Hubs: USB Insight Hub (open, ESP32-S3, JSON API, USD 139) — [Crowd Supply](https://www.crowdsupply.com/aerio-solutions/usb-insight-hub); YKUSH (open hardware, EUR 59.99) — [Yepkit](https://www.yepkit.com/products/ykush); I2C-controlled DIY hub — [GitHub](https://github.com/JimHeaney/i2c-usb-hub)
- Bench instruments: FOSSASIA Pocket Science Lab (open-hardware scope/multimeter/logic analyser board; launch price USD 65 in 2019; has a sigrok wiki page and a `pslab` Python package) — [CNX Software, 14 Jan 2019](https://www.cnx-software.com/2019/01/14/buy-pslab-electronics-lab-board/), [sigrok wiki](https://sigrok.org/wiki/PSLab_Pocket_Science_Lab), [PyPI pslab](https://pypi.org/project/pslab)
- Agent-to-hardware (MCP), curated list: mcp2serial (serial hardware, Pico first), mcp-micropython-bridge (ESP32/RP2040 over USB serial), embedded-debugger-mcp (probe-rs), stm32-mcp, saleae-logic2-mcp, scope-mcp (Keysight), tinymcp (Golioth), modbus-mcp, kicad-sch-api; no entries for USB hubs, CAN/OBD or bench automation — [awesome-mcp-hardware](https://github.com/beriberikix/awesome-mcp-hardware)
- MCP server running on a Pico W itself (C SDK, plain HTTP JSON on port 8000, humidity sensor as resource, PWM/relay as tools), 22 Sep 2026 — [element14](https://community.element14.com/technologies/embedded/b/blog/posts/connecting-ai-to-the-physical-world-with-mcp-and-a-pi-pico-w), [repo](https://github.com/shabaz123/pico_w_mcp)
- Adafruit MicroPython MCP server (`adafruit-mcp-micropython`) lets Claude Code run code, read sensors and toggle GPIO on Pico/ESP32 boards, including over WebREPL — [DEV Community summary](https://dev.to/gentic_news/adafruits-new-mcp-server-lets-claude-code-control-micropython-hardware-4h8h)
- MCP for a Rigol DP832 supply (Python/FastMCP, Jan 2026): hard voltage/current limits enforced in server code because prompt-stated limits "are still not 100% deterministic" — [Altium, Ari Mahpour](https://resources.altium.com/p/building-mcp-server), [repo](https://github.com/amahpour/rigol_dp832_mcp_server)
- "Schema-Bound LLM Control of Scientific Instrumentation through MCP Skills" (arXiv, 19 Jul 2026): validates tool requests against physical limits before they reach the instrument, 30 typed tools, tested only against a simulator — [arXiv 2607.17012](https://arxiv.org/abs/2607.17012)
- Modular field-kit design: Fab Academy 2026 "Modular CyberDeck" (Rock 5B+, pogo-pin backplane, per-module power switching and signature check, PETG mid-frame, about USD 525 BOM) is explicitly theoretical/not built — [Fab Academy](https://fabacademy.org/2026/labs/barcelona/students/pedro-rocha/projects/final-project/)
- ESP32 "Flipper alternative" firmware lines exist (Bruce, CapibaraZero) on boards such as the LilyGo T-Embed CC1101 (a "Plus" variant cited at EUR 66) — [BruceDevices/firmware](https://github.com/BruceDevices/firmware), [CNX Software on CapibaraZero, 4 Dec 2024](https://www.cnx-software.com/2024/12/04/capibarazero-firmware-enables-low-cost-flipper-zero-esp32-s3-based-alternatives/), [China-Gadgets](https://www.china-gadgets.de/lilygo-t-embed-cc1101-plus-flipper-zero-alternative-vorstellung/?c_order=desc)

### Inferences
- The MCP-hardware projects are almost all thin "run this on the board" bridges; the two that discuss safety (Altium, arXiv) converge on the same idea as the project's manifest: limits enforced server-side from a typed schema. A manifest carrying channels/units/limits/risk appears to be new ground rather than a duplicate.
- Qwiic/STEMMA QT, Grove and M5Stack units are ready-made module systems at roughly USD 5-20 per function; they solve connectors, not identity. A custom C3/RP2040 carrier with a manifest costs about USD 3-5 more silicon per module than a bare I2C breakout but gives self-identification, health and update, which the off-the-shelf ecosystems do not.
- PSLab is the closest open "lab in a box" to study for the fixed bench (multimeter/scope are another researcher's scope).

### Gaps
- Licence, activity dates and star counts were not captured for most repos (GitHub pages returned no commit dates).
- Not reviewed: hackaday.io cyberdeck builds (e.g. [SIGINT SDR Commander](https://hackaday.io/project/197414-sigint-sdr-commander)), RealDash/RaceCapture/Speeduino loggers, Victron/JK DIY rigs, SensorWatch, CircuitPython field tools, M5Stack/XIAO/Feather per-function pricing.
- No project was found where an LLM agent operates a home-built multi-instrument ESP32/Pico bench with per-module manifests.

---

## 12. Custom PCBs: when they beat hand-wired modules, cost to the US in late 2026

### Takeaway
For the pack's small carrier boards OSH Park (USD 5 per square inch for three boards, free shipping, no import duty) is now the default; JLCPCB orders to the US carry roughly 35-45% duty on top of board, assembly and shipping, so it wins only for larger boards or assembled runs.

### Cited Findings
- OSH Park: 2-layer prototype USD 5/sq in per set of 3 (9-12 days), Super Swift USD 10/sq in (4-5 business days), 4-layer USD 10/sq in per 3, medium run USD 1/sq in (100 sq in minimum); free shipping worldwide — [OSH Park services](https://docs.oshpark.com/services/)
- JLCPCB US tariff FAQ (updated 17 Mar 2026): Section 301 25%; "additional" and "reciprocal" tariffs 0% since 24 Feb 2026; provisional 10% from 24 Feb to 24 Jul 2026; Section 122 at 12.5% from 24 Jul 2026 (JLC collects 10%); regular duty 0-7.5%; stated total range about 35% to 92.5%; individuals must use DDP with duty prepaid at checkout — [JLCPCB FAQ](https://jlcpcb.com/help/article/us-tariff-policy-faq)
- Real orders: 50 assembled boards on 14 Apr 2026 = PCB USD 66.32 + assembly USD 1,302.52 + shipping USD 169.19 + customs USD 479.11 (about 35% of PCB + assembly); another user reports 43% on a PCB-only order under DDP around April 2026 and 53% on PCBA before the Supreme Court decision; brokerage about USD 20-40; one UPS shipment was billed USD 130 duty + USD 15 fee on a USD 44 order — [EEVblog forum, Apr to May 2026](https://www.eevblog.com/forum/manufacture/jlc-tarriffs-etc-to-us/)
- Altium (data as of 19-20 Mar 2026): de minimis suspended for China/Hong Kong (formal entry regardless of value; MPF minimum USD 33.58 per entry); bare PCBs (HTS 8534.00) were on the Section 301 exclusion list, so bare boards owed only the 10% Section 122 surcharge, while assembled boards are typically not excluded; "a $50 prototype PCB from China can exceed a 75% effective tax-and-fee rate" — [Altium](https://resources.altium.com/p/tariff-reset-pcb-workarounds). **Conflict:** EEVblog users report 43% actually collected on a PCB-only JLCPCB order in the same period.
- USTR extended all 178 Section 301 exclusions (due to expire 9-10 Nov 2026) through 9 Jan 2027 — [KPMG, Nov 2026](https://kpmg.com/us/en/taxnewsflash/news/2026/11/ustr-section-301-china-product-exclusions.html)
- KiCad agent tooling: "KiCad Agentic MCP" (AGPL-3.0, single Rust binary, 202 tools, KiCad 10 IPC API, verdict taken from `kicad-cli` ERC/DRC; author notes passing ERC/DRC does not make a design correct; Linux not live-tested) — [KiCad forum, 27 Aug 2026](https://forum.kicad.info/t/kicad-agentic-mcp-an-ai-assistant-that-edits-your-board-through-kicads-own-api-and-lets-kicad-decide-whether-the-result-is-good/71403), [repo](https://github.com/nevenfo/kicad-agentic-mcp); others: [kicad-mcp-pro](https://github.com/oaslananka/kicad-mcp-pro), JLCPCB's own guide to designing a PCB with Claude Code via KiCad MCP — [JLCPCB blog](https://jlcpcb.com/blog/kicad-mcp-how-to-design-a-pcb-with-claude-code)

### Inferences
- Rule of thumb for this project: a 1.5 x 1.0 inch transceiver carrier = 1.5 sq in = USD 7.50 for three at OSH Park, delivered, no customs paperwork. JLCPCB's nominal USD 2-5 for five small boards **[UNVERIFIED-EST]** becomes roughly USD 25-45 landed once courier shipping, duty and minimum fees are added, so OSH Park wins below about 5-8 sq in; JLCPCB wins for boards above that or when SMT assembly of fine-pitch parts (isolators, hub ICs) is wanted.
- Hand-wiring breakout modules on perfboard wins for the first unit of anything; a custom board pays off when (a) three or more identical modules are needed (scatter pods, serial pods), (b) the connector/mechanical fit into the printed grid matters, or (c) the part is not sold as a breakout (L9637D, isolated transceivers, load switches).
- `kicad-cli` ERC/DRC is runnable headless, so an agent-generated board can be gated in free GitHub CI on a public repo; KiBot (not reviewed here) is the usual wrapper for fab outputs. ERC/DRC is a floor, not proof of function.

| Bought part | Ground-up build | Verdict |
|---|---|---|
| Breakout modules hand-wired on perfboard (USD 1-3 of board/wire) | OSH Park USD 5/sq in per 3 (cited); JLCPCB + roughly 35-45% duty + USD 20-40 shipping/brokerage (cited ranges) | **Hand-wire** first units; **OSH Park** for small repeat boards; **JLCPCB** only for large or assembled runs. 4-10 h per board design. Risk: tariff rates have changed repeatedly in 2026 and the exclusion now ends 9 Jan 2027. |

### Gaps
- No verified late-Nov 2026 JLCPCB or PCBWay quote for "5 small boards shipped to the US, with and without assembly"; the latest real-order figures found are April-May 2026.
- Whether bare 2-layer PCBs are covered by the extended exclusions, and what JLCPCB collects today on bare boards, is unresolved (Altium versus EEVblog reports conflict).
- Other US fabs (beyond OSH Park) and KiBot were not reviewed.

---

## 13. Closing synthesis: revised bill of materials, best builds, false economies

### Takeaway
On the in-scope lines the cheapest credible mix comes to roughly USD 386 against USD 589 in the first plan (about a third less), or about USD 480 with cheap sensors, three scatter pods and consumables added; most of the saving comes from four items (vehicle interface, Pi at list price, GNSS, mic array), and several figures are unverified estimates.

### Cited Findings
- Confirmed anchor prices used below: Pi 5 4 GB USD 85 list — [Tom's Hardware, 2 Oct 2026](https://www.tomshardware.com/raspberry-pi/component-shortages-drive-raspberry-pi-prices-up-by-up-to-23-percent-escalating-lpddr4-lpddr5-costs-trigger-the-third-price-hike-of-the-year); WiCAN Pro USD 89 + 8 — [Crowd Supply](https://www.crowdsupply.com/meatpi-electronics/wican-pro); Plugable USB3-HUB7BC on the uhubctl list — [uhubctl](https://github.com/mvp/uhubctl/); Apache 3800 USD 24.99 member price — [Slickdeals, 7 Mar 2026](https://slickdeals.net/f/19280394-apache-3800-weatherproof-protective-case-large-black-harbor-freight-members-only-price-24-99); 12 V 20 Ah LiFePO4 USD 45-63 — [Slickdeals, 12 Aug 2025](https://slickdeals.net/f/18524746-eco-worthy-12v-20ah-lifepo4-lithium-battery-47-25-free-shipping); OSH Park USD 5/sq in per 3 — [OSH Park](https://docs.oshpark.com/services/)

### Inferences

**(a) Revised whole-pack bill of materials (in-scope lines only; RF and multimeter excluded).** C = cited price, E = unverified estimate, P = plan figure kept.

| Line | First plan (USD) | Revised choice | Revised (USD) | Basis |
|---|---|---|---|---|
| Compute | 110 | Raspberry Pi 5 4 GB at list | 85 | C |
| Switched hub | 50 | Plugable USB3-HUB7BC kept + DIY VBUS/current sidecar | 50 + 8 | P + E |
| GNSS | 30 | ATGM336H module with PPS | 5 | E |
| Vehicle | 97 | DIY ESP32-C3 + L9637D + SN65HVD230 + J1962 pigtail | 18 | E |
| Serial/solar | 38 | 1 Waveshare isolated USB adapter + 1 DIY wireless pod | 19 + 12 | P + E |
| Audio | 55 | DIY 6x INMP441 + RP2040 array | 20 | E |
| Camera | 29 | Camera Module 3 kept | 29 | P |
| Thermal | 40 | MLX90640 marketplace module kept | 40 | P |
| Power | 100 | 12 V 20 Ah LiFePO4 + 5 V 5 A buck + INA226 + fusing | 75 | C (2025) + E |
| Case | 40 | Apache 3800 at member/sale price | 25 | C |
| **Subtotal, like-for-like** | **589** | | **386** | |
| Sensors | 5-20 each | About 10 marketplace breakouts + 1 genuine reference | 25 | E |
| Scatter pods | not itemised | 3 pods at about USD 10 | 30 | E |
| Wire, connectors, perfboard, PETG | not itemised | | 40 | E |
| **Total** | | | **about 481** | |

Arithmetic check: 85 + 58 + 5 + 18 + 31 + 20 + 29 + 40 + 75 + 25 = 386; 386 + 25 + 30 + 40 = 481. Optional reference tools not included: vLinker USD 37 [PLAN] to cross-check the DIY K-line adapter; used PS3 Eye for ODAS bring-up.

**(b) Five builds with the most learning and agent-tweakability per dollar**
1. K-line + CAN vehicle pod (about USD 18): open MIT firmware to start, clear path to a first-of-its-kind embassy Rust K-line crate, directly useful on the Tacoma and the JBD CAN bus.
2. Wireless solar comms pod (about USD 12): Modbus maps are pure data the agent can extend; inherently isolated; real homestead value on the EG4/JBD gear.
3. Synchronous mic array on RP2040 PIO (about USD 20): raw channels, geometry and sample rate under firmware control; feeds ODAS on the Pi.
4. VBUS switch + current-sense sidecar (about USD 8-15): gives the agent per-port power telemetry and a hard cut that no cheap hub provides; a natural home for the physical-safety layer.
5. Power/fuel-gauge module plus scatter pods (about USD 5-12 each): the pack observing its own battery, and the smallest complete example of the module manifest/identify/health/update contract.

**(c) Five places where building is false economy**
1. A USB 3 hub from scratch: hub IC sourcing, 5 Gbit/s layout and 4-layer boards to replace a USD 50 part that already works with uhubctl.
2. A battery pack from loose 18650/21700 cells: cell + BMS + welder cost meets or exceeds a finished 12 V LiFePO4, with real fire risk.
3. J1850 VPW/PWM interface: obsolete transceivers and fiddly timing for buses the owner's truck does not have; buy a WiCAN Pro if ever needed.
4. Thermal imaging from low-pixel parts (AMG8833) or camera sensors from scratch: too coarse to be useful; buy MLX90640 or a used UVC thermal dongle.
5. A "cheaper" off-brand SBC as the brain: 2026 DRAM pricing erased the discount and vendor kernels cost days; the Pi 5 4 GB at list is only USD 85. Related: JLCPCB one-off boards under current duties when perfboard or OSH Park would do.

### Gaps
- The revised total rests on unverified marketplace estimates for 8 of 13 lines; a pricing pass against live AliExpress/Amazon carts is needed before quoting it as a budget.
- Shipping and import charges on a consolidated AliExpress order to Arizona in Nov 2026 were not determined and could add materially to the small-parts lines.
- The first plan's sensor quantity and the content of its USD 110 compute line are unknown, so the like-for-like saving (about USD 203) is the only firm comparison.
