# Ground-up / ultra-cheap RF sensing for the agent-operated sensor pack (state: 25 Nov 2026)

Conventions used in these notes
- "Fetched" = page read this session (Nov 2026). Prices are USD as shown on the fetched page, with the page's own date where it had one.
- "(unverified)" = not confirmed from a fetched source this session; it comes from the researcher's background knowledge and must be re-checked before it is relied on. All such items sit under Inferences or Gaps, never under Cited Findings.
- Effort figures (hours, skill) are the researcher's estimates, not sourced.
- Verdict vocabulary: "phase 1", "phase 2", "not worth it".

---

## 1. What exactly is the "ESP32 SDR unlock", and can one or several such chips be modules in the pack?

### Takeaway
The unlock is real and is called ESP-SDR (ESPARGOS team, public around 1 Oct 2026), with two independent parallel finds (h0m3us3r's eSpDR on the ESP32-S3, and Twotoz's C5VRX on the ESP32-C5): an undocumented debug path in almost every Wi-Fi-capable ESP32 dumps raw 10-bit I/Q from the radio's ADC into SRAM at up to 80 MS/s. It is a receive-only 2.4 GHz (plus 5 GHz on the C5) spectrum/IQ tool, not a general SDR: it does not tune below about 1.84 GHz, so it does nothing for 315/433/915 MHz, LoRa or HF inverter noise, and on most chips it gives gappy snapshots rather than a continuous stream.

### Cited Findings

Identity, people, dates
- Several ESP32 chips carry an undocumented feature through which firmware skips the fixed Wi-Fi/Bluetooth modem and captures raw IQ baseband; rtl-sdr.com reported it on 1 Oct 2026, crediting the ESPARGOS team — [rtl-sdr.com](https://www.rtl-sdr.com/various-projects-independently-find-hidden/)
- Florian Euchner, Zoltan Doczi (Z2Labs, credited with the original S3 "Turbo Mode" spectrum work) and "14sea" are the listed ESP-SDR contributors; the repo also thanks h0m3us3r, whose 5/6 LO-divider investigation informed the lower tuning range — [ESP-SDR README](https://raw.githubusercontent.com/ESPARGOS/esp-sdr/main/README.md)
- Discovery route: reverse engineering of the `adctrig` function in Espressif's `librftest` library with LLM help; debug registers point the modem's sample-dump engine at internal SRAM. ESPARGOS assumes Espressif left the path in for factory/on-wafer test — [espargos.net/espsdr](https://espargos.net/espsdr/)
- h0m3us3r (Reddit) showed ESP32-S3 continuous IQ at 80 MS/s days before ESPARGOS, uploading to GitHub on 26 Sept 2026; ESPARGOS says it had been working independently for months — [rtl-sdr.com](https://www.rtl-sdr.com/various-projects-independently-find-hidden/), [espargos.net/espsdr](https://espargos.net/espsdr/)
- C5VRX (GitHub user Twotoz), first uploaded 13 Aug 2026, used the same undocumented IQ stream on an ESP32-C5 to demodulate 5.8 GHz analog FPV video on-chip — [rtl-sdr.com](https://www.rtl-sdr.com/various-projects-independently-find-hidden/)
- Mainstream coverage followed on 3 Oct 2026 (Hackaday) and 4 Oct 2026 (CNX Software) — [Hackaday](https://hackaday.com/2026/10/03/the-esp32-an-sdr-in-itself/), [CNX Software](https://www.cnx-software.com/2026/10/04/esp-sdr-firmware-turns-esp32-into-a-2-4-5-ghz-software-defined-radio-sdr/)

Which chips, what numbers (ESP-SDR README table)
- Supported: ESP32, C2, C3, C5, C6, C61, H2, S2, S3, S31. Not yet: H21, H4. Impossible: P4 (no radio) — [ESP-SDR README](https://raw.githubusercontent.com/ESPARGOS/esp-sdr/main/README.md)
- Sample rates: 80/40/16 MS/s on ESP32, C2, C3, C6, S2, S3; 80 down to 4 MS/s on C5, C61, S31; 32/16/10.667/6.4 MS/s on the H2 — [ESP-SDR README](https://raw.githubusercontent.com/ESPARGOS/esp-sdr/main/README.md)
- Sample format: one complex sample per 32-bit word, signed 10-bit I and 10-bit Q, 8 bits of RX gain index — [espargos.net/espsdr](https://espargos.net/espsdr/)
- Tuning: 2.2–2.7 GHz, and 4.8–6.0 GHz only on the C5; analog RX bandwidth roughly 13–54 MHz depending on chip and filter setting — [espargos.net/espsdr](https://espargos.net/espsdr/)
- Extended low end: the firmware accepts 100–6000 MHz but the PLL locks only near 2.4 GHz (and 5 GHz on C5); ESP32/C2/C3/S2/S3 reach about 1.85 GHz with a 5/6 LO divider, the C6 about 1.95 GHz — [ESP-SDR README](https://raw.githubusercontent.com/ESPARGOS/esp-sdr/main/README.md)
- eSpDR measured 1841.67–2790 MHz accepted on an S3, with PLL lock failing above 2781 MHz on that board; the response is not flat and out-of-band signals alias in — [eSpDR wiki](https://github.com/h0m3us3r/eSpDR/wiki)
- Snapshot depth: up to 16,380 samples per burst (8,190 on C2, 12,284 on S2); gain is not calibrated in dB — [ESP-SDR README](https://raw.githubusercontent.com/ESPARGOS/esp-sdr/main/README.md)
- The modem produces 2,560 Mbit/s (80 MS/s x 32 bit); UART carries about 3 Mbit/s, so getting samples off the chip is the bottleneck — [espargos.net/espsdr](https://espargos.net/espsdr/), [Hackaday](https://hackaday.com/2026/10/03/the-esp32-an-sdr-in-itself/)

Continuous versus snapshot
- Gap-free recording exists only on the continuous-capture chips (C6, C61, S3, S31, over native USB); elsewhere the FFT sees a small share of airtime, often a few percent or less — [ESP-SDR README](https://raw.githubusercontent.com/ESPARGOS/esp-sdr/main/README.md)
- rtl-sdr.com's summary: with only an ESP32 and snapshot export it is a spectrum analyzer, and demodulating continuous data is not possible — [rtl-sdr.com](https://www.rtl-sdr.com/various-projects-independently-find-hidden/)
- ESP32-S3 continuous I/Q is the 16 MS/s stream decimated to 15.625–250 kS/s over USB via `esp-sdr-bridge` — [ESP-SDR README](https://raw.githubusercontent.com/ESPARGOS/esp-sdr/main/README.md)
- Jochen (DA2JH) FPGA-free eSpDR fork: S3 decimates 16 MS/s on-chip to 250 kS/s (333 kS/s optional), int8 IQ over USB Serial/JTAG, a Python bridge poses as `rtl_tcp` for SDR++/GNU Radio; about ±100 kHz usable; spurs at exactly 2400.000 and 2440.000 MHz from the 40 MHz crystal; no AGC, no calibrated levels; one board tested; 0BSD — [jochenhammes/eSpDR](https://github.com/jochenhammes/eSpDR)
- ESP32-S31 streaming: up to 20 MS/s over high-speed USB and up to 40 MS/s over Gigabit Ethernet, 8-bit IQ, through the receive-only SoapyESPSDR driver for GNU Radio/Gqrx on Linux; flagged experimental — [espargos.net/espsdr](https://espargos.net/espsdr/), [ESP-SDR README](https://raw.githubusercontent.com/ESPARGOS/esp-sdr/main/README.md). Earlier press gave lower figures (8–16 MS/s) — [CNX Software](https://www.cnx-software.com/2026/10/04/esp-sdr-firmware-turns-esp32-into-a-2-4-5-ghz-software-defined-radio-sdr/)
- Full-rate route: h0m3us3r's eSpDR needs an Alchitry Au (Artix-7) FPGA plus FT600 USB3 bridge to move 80 MS/s, 10-bit IQ to a Linux host; README calls it a proof of concept; 0BSD; code written by AI models under the author's direction — [eSpDR wiki](https://github.com/h0m3us3r/eSpDR/wiki)

Host side, licence, maturity
- Host link is native USB or UART0 at 2 Mbaud with a line-based protocol and CRC32-protected binary payloads; a browser viewer/flasher (ESP-WebSDR) shows spectrum and waterfall — [ESP-SDR README](https://raw.githubusercontent.com/ESPARGOS/esp-sdr/main/README.md), [espargos.net/espsdr](https://espargos.net/espsdr/)
- Licence: GPL-3.0-or-later for ESP-SDR and ESP-WebSDR; the README states the capture mechanism itself is not copyrighted, only the code — [ESP-SDR README](https://raw.githubusercontent.com/ESPARGOS/esp-sdr/main/README.md)
- Built on ESP-IDF, each target pinned to an IDF commit; the S31 build needs `idf.py --preview`; no mention of Rust — [ESP-SDR README](https://raw.githubusercontent.com/ESPARGOS/esp-sdr/main/README.md)
- Parts of the code are AI-generated and not fully hand-reviewed — [CNX Software](https://www.cnx-software.com/2026/10/04/esp-sdr-firmware-turns-esp32-into-a-2-4-5-ghz-software-defined-radio-sdr/)
- Known practical snags: CH340 bridges failing at 2 Mbaud, brown-outs when powered from a UART adapter's 3.3 V pin, ModemManager resetting Espressif USB devices on Linux — [ESP-SDR README](https://raw.githubusercontent.com/ESPARGOS/esp-sdr/main/README.md)
- No sensitivity, noise figure, dynamic range or ENOB figure is published; a Hackaday commenter asked for ENOB and got no answer — [Hackaday](https://hackaday.com/2026/10/03/the-esp32-an-sdr-in-itself/)
- No Espressif statement appears in the coverage; whether future silicon or IDF could close the path is commenter speculation only — [Hackaday](https://hackaday.com/2026/10/03/the-esp32-an-sdr-in-itself/)

Transmit (flag, do not exploit)
- ESPARGOS says the chips can emit arbitrary signals but ships no transmitter because of misuse risk (jamming) — [espargos.net/espsdr](https://espargos.net/espsdr/)
- Community TX forks already exist and are labelled experimental: esp-sdr-zephyr (IQ TX), esp32-sdr-trx (NBFM/SSB/RTTY on 13 cm by steering the PHY test tone), ESP32SDR (OOK/ASK/FSK) — [CNX Software](https://www.cnx-software.com/2026/10/04/esp-sdr-firmware-turns-esp32-into-a-2-4-5-ghz-software-defined-radio-sdr/), [rtl-sdr.com](https://www.rtl-sdr.com/various-projects-independently-find-hidden/)

Derivative projects that matter to this pack
- ESPARGOS One captures phase-coherent IQ across eight antennas and can now direction-find any 2.4 GHz signal (BLE, Zigbee, 802.11b), but that firmware is closed for now — [espargos.net/espsdr](https://espargos.net/espsdr/)
- Adafruit (11 Nov 2026) runs ESP-SDR on the C6 coprocessor of a Metro ESP32-P4: the C6 does capture plus FFT and ships CRC-checked 1024-bin frames over UART at 2 Mbaud; the pattern is exactly "radio module emits spectra, host draws" — [Adafruit guide](https://learn.adafruit.com/esp-sdr-on-metro-esp32-p4?view=all)
- C5VRX status late Oct 2026: one positive NTSC observation, not range-validated, GPL-3.0-only, ESP-IDF 6.0.2, XIAO ESP32-C5 — [Twotoz/C5VRX](https://github.com/Twotoz/C5VRX)
- CardputerADV-SDR ports ESP-SDR to the M5Stack Cardputer ADV and LilyGo T-Dongle S3 — [rtl-sdr.com comments](https://www.rtl-sdr.com/various-projects-independently-find-hidden/)
- Hackaday's own verdict: not a general-purpose SDR in the RTL-SDR sense — [Hackaday](https://hackaday.com/2026/10/03/the-esp32-an-sdr-in-itself/)

Conflicts between sources
- Hackaday says Bluetooth-only chips lack the feature; the ESP-SDR README (primary, newer) lists the ESP32-H2 as supported at up to 32 MS/s — [Hackaday](https://hackaday.com/2026/10/03/the-esp32-an-sdr-in-itself/) vs [ESP-SDR README](https://raw.githubusercontent.com/ESPARGOS/esp-sdr/main/README.md)
- S31 streaming rate: 8–16 MS/s in early-October press vs 20 (USB) / 40 (GbE) MS/s on the project's own pages; treat the project pages as current.

### Inferences
- Verified vs hype. Verified by primary repos: raw 10-bit IQ, up to 80 MS/s, 2.2–2.7 GHz on nearly all Wi-Fi ESP32s, 4.8–6.0 GHz on C5, RX-only official firmware, GPL-3 code, browser demo. Hype: "ESP32 is now a $5 SDR" in the RTL-SDR sense. It cannot tune sub-GHz or HF, has no published sensitivity or dynamic-range data, and only C6/C61/S3/S31 give continuous capture (S3 at 250 kS/s, S31 at 20–40 MS/s).
- Fit for this pack (module roles):
  - 2.4 GHz wideband waterfall module (C6 or C3 class board): shows non-Wi-Fi energy that promiscuous mode is blind to (microwave ovens, FHSS RC links, analog video, Bluetooth hopping, Zigbee). Verdict: phase 1; it is a firmware flash on boards the pack needs anyway.
  - 5 GHz / 5.8 GHz waterfall module (C5): the only sub-USD 10 way found to see 5.8 GHz energy, which matters for drone video/control links. Verdict: phase 1.
  - ESP32-S31 CoreBoard as a continuous 2.4 GHz SDR to the Pi 5 through SoapySDR: the only variant that behaves like an SDR. Verdict: phase 2 (experimental driver, preview IDF).
  - Several ESP32s as a phase-coherent array: only ESPARGOS hardware with a shared reference does this and its firmware is closed. Verdict: not worth it for now.
  - FPGA + FT600 eSpDR at 80 MS/s: costs more than an RTL-SDR and still only 2.4 GHz. Verdict: not worth it.
- Rust status: no Rust (esp-hal / esp-radio) port of the IQ capture was found. Practical path for a Rust-first project: treat ESP-SDR as a prebuilt GPL-3 firmware image the agent flashes onto a module, and write the host-side protocol client (line protocol + CRC32 binary frames) in Rust on the Pi. A native esp-hal port would mean re-implementing the debug-register sequence after PHY init; the 0BSD eSpDR code is the licence-friendlier reference for that than GPL-3 ESP-SDR. Effort estimate for a native port: 40+ hours, advanced; for flash-and-use: 1–2 hours, beginner.
- Because the feature is undocumented and unanswered by Espressif, pin known-good board/chip revisions and firmware hashes in the module registry, and keep the role optional.

### Gaps
- No measured noise figure, sensitivity, spur-free dynamic range or ENOB for any chip; needs a bench test against a known source.
- No demonstrated decode of BLE / Zigbee / Wi-Fi from ESP-SDR IQ was found (the S31 stream should permit it in GNU Radio, but nobody was found showing it).
- No statement from Espressif on support, removal, or licensing.
- Whether esp-radio's PHY init can coexist with the debug capture in a no_std build is untested (no source found).
- The Reddit thread and the three TX fork repos were not opened; details come from rtl-sdr.com and CNX summaries.
- An unverified Sipeed teaser of a cheap SoC with 2.2–3 GHz and 4.4–6 GHz coverage and USB 3 output is mentioned by CNX's editor; no part number exists — [CNX Software](https://www.cnx-software.com/2026/10/04/esp-sdr-firmware-turns-esp32-into-a-2-4-5-ghz-software-defined-radio-sdr/)

---

## 2. What can an array of ESP32-class boards do as radio sensors without IQ, and what firmware can be borrowed?

### Takeaway
A handful of USD 5 boards covers Wi-Fi (2.4 GHz and, on the C5, 5 GHz) frame capture, BLE advertising, 802.15.4 (Zigbee/Thread), CSI motion sensing and drone Remote ID, and the Rust stack (esp-radio 1.0.0-beta.2 plus esp-csi-rs) already exposes sniffer, CSI, BLE and 802.15.4, although all of those except plain Wi-Fi are still marked unstable. One radio does one job at a time, which fits the "one small reflashable module per role" design.

### Cited Findings

Board prices
- NiceMCU ESP32-C5 Mini: USD 4.93 shipped on AliExpress (28 Apr 2026); dual-band Wi-Fi 6, BLE 5, 802.15.4, on-board antenna plus IPEX connector, USB-C — [CNX Software](https://www.cnx-software.com/2026/04/28/esp32-c5-mini-usb-c-board-offers-dual-band-wifi-6-and-up-to-14x-gpio-pins-for-iot-projects/)
- Seeed XIAO ESP32-C5: about USD 13 per the SignalSweep README (Oct 2026) — [Circuit Digest](https://circuitdigest.com/news/signalsweep-open-source-ble-and-wi-fi-scanner-on-esp32-c5-that-can-detect-airtags-flock-cameras-and-drones)
- ESP32-S31-Function-CoreBoard-1: USD 19.70 on AliExpress (25 May 2026); 2.4 GHz Wi-Fi 6, BT 5.4 incl. direction finding, 802.15.4, Gigabit Ethernet, high-speed USB; mass production announced by July 2026 — [CNX Software](https://www.cnx-software.com/2026/05/22/esp32-s31-development-boards-iot-smart-audio-and-hmi-applications/?amp=1)

Rust (esp-rs) status
- esp-radio 1.0.0-beta.2: `wifi` is the only stable driver; `sniffer`, `csi`, `esp-now`, `ble`, `ieee802154` and `coex` are feature flags marked unstable, and the 802.15.4 feature cannot be enabled together with Wi-Fi — [esp-radio docs](https://docs.espressif.com/projects/rust/esp-radio/1.0.0-beta.2/esp32c6/esp_radio/index.html)
- esp-csi-rs: no_std CSI collection on ESP32, C3, C5 (both bands), C6, S3; sniffer/station/softAP/ESP-NOW modes; built on esp-hal ~1.1, esp-radio and embassy; Apache-2.0; 0.13.0 released 11 Oct 2026; self-described as early development; S3 cannot act as emitter — [docs.rs esp-csi-rs](https://docs.rs/crate/esp-csi-rs/0.10.1)

802.15.4 (Zigbee/Thread)
- Espressif published (9 Nov 2026) a Zephyr sample turning ESP32-H2, C6 or C5 into a promiscuous 802.15.4 sniffer that prints one text line per frame (hex, RSSI, LQI, timestamp) on UART, with a Python extcap for Wireshark; original ESP32 and S-series have no 802.15.4 radio — [Espressif developer blog](https://developer.espressif.com/blog/2026/11/zephyr-ieee802154-wireshark-sniffer/)
- Its limits: only CRC-valid frames, no PHY-level view, one channel per capture, no channel change mid-capture — [Espressif developer blog](https://developer.espressif.com/blog/2026/11/zephyr-ieee802154-wireshark-sniffer/)

Wi-Fi channel energy / survey
- A plain ESP32 in promiscuous mode can step channels 1–13 with 20 ms dwell (about 4 sweeps/s) and report per-channel RSSI, but this depends on decodable frames and is not a swept analyzer — [huecodes ESP32 Spectrum Analyzer](https://huecodes.github.io/hardware/spectrum-analyzer)
- The Wifydra (DEF CON 31, lozaning): 1 ESP32 "dom" plus 14 ESP8266 "sub" nodes watching 2.4 GHz channels simultaneously, about USD 90, about 10 W, open firmware/BOM/Gerbers — [InfoconDB](https://infocondb.org/con/def-con/def-con-31/the-wifydra-multiheaded-rf-panopticon)

Drone Remote ID
- SignalSweep (GPL-3.0-or-later, Oct 2026) on a XIAO ESP32-C5 decodes ASTM F3411 Remote ID over Bluetooth, Wi-Fi beacon and Wi-Fi NAN on both Wi-Fi bands, plus trackers; it sends scan requests unless receive-only mode is set — [Circuit Digest](https://circuitdigest.com/news/signalsweep-open-source-ble-and-wi-fi-scanner-on-esp32-c5-that-can-detect-airtags-flock-cameras-and-drones)
- Colonel Panic's Mesh Mapper uses an ESP32-C3 or S3 for Remote ID detection, linked by serial to a Heltec LoRa 32 V3 for Meshtastic relay; breakout USD 15, assembled kit USD 100 — [CNX Software](https://www.cnx-software.com/2025/06/05/map-remote-id-enabled-drones-with-esp32-c3-s3-and-meshtastic-lora-modules/)

BLE beyond advertising (not an ESP32, but USD-class)
- Kismet documents the Sonoff ZBDongle-P (TI CC2652P) flashed with NCC Group's Sniffle as a BLE 4.x/5 source: one advertising channel per dongle (about a third of legacy adverts), optional extended-advertising and coded-PHY capture — [Kismet docs](https://kismetwireless.net/docs/readme/datasources/bluetooth-sniffle-ble/)
- Kismet's datasource list has no ESP32 capture source; it does list nRF52840, TI CC2531, CatSniffer v3, Sniffle, rtl_433, Open Drone ID / Remote ID and LoRa "LoraPipe" — [Kismet docs](https://kismetwireless.net/docs/readme/datasources/datasources/)

Firmware to borrow (agent-drivable?)
- ESP32 Bit Pirate (renamed from ESP32 Bus Pirate; geo-tp; MIT; 6.2k stars): ESP32-S3 boards only; modes include Wi-Fi sniff, BLE sniff, sub-GHz analyze/record/replay, RF24 scan, FM analyze, LoRa sniff and a Meshtastic shell; USB serial CLI, Wi-Fi web CLI and Python scripting over serial; no JSON API documented — [GitHub](https://github.com/geo-tp/ESP32-Bit-Pirate), [rtl-sdr.com 11 Aug 2026](https://www.rtl-sdr.com/esp32-bit-pirate-updates-new-lora-and-meshtastic-analysis-features/), [Espressif developer blog](https://developer.espressif.com/blog/2026/08/esp32-bit-pirate/)
- Bit Pirate also carries transmit features (deauth, replay, FM broadcast, BLE spoofing) — [GitHub](https://github.com/geo-tp/ESP32-Bit-Pirate)

### Inferences
- Role map per USD 5 board (one role at a time; the agent reflashes to change role):

| Role | Chip | Senses | Firmware today | Rust status | Effort (est.) | Verdict |
|---|---|---|---|---|---|---|
| Wi-Fi 2.4 GHz monitor (fixed channel or hop) | C3/C6/S3 | mgmt/control/data frame headers, RSSI | esp-radio `sniffer`; Bit Pirate; Marauder-class | unstable feature, usable | 4–8 h, intermediate | phase 1 |
| Wi-Fi 5 GHz monitor | C5 | same on 5 GHz | esp-csi-rs lists C5 dual band | unstable | 4–8 h | phase 1 |
| BLE scanner (legacy + extended adv) | any | adverts, RSSI, trackers | esp-radio `ble` | unstable | 4–8 h | phase 1 |
| 802.15.4 sniffer | C6/H2/C5 | Zigbee/Thread frames, RSSI/LQI | Espressif Zephyr sniffer (text over UART) | esp-radio `ieee802154` unstable | 2 h (Zephyr) / 8 h (Rust) | phase 1 |
| Remote ID receiver | C5 preferred | F3411 over BLE, Wi-Fi beacon, NAN | SignalSweep (GPL-3) | not found in Rust | 2 h flash / 20 h Rust port | phase 1 (flash), phase 2 (Rust) |
| CSI presence/motion | pair of boards | motion between two nodes | esp-csi-rs | native no_std Rust | 6–10 h | phase 2 |
| ESP-SDR waterfall | C6/C5/S3 | raw 2.4/5 GHz energy | ESP-SDR (GPL-3, ESP-IDF) | host client only | 1–2 h | phase 1 |

- "One board per channel" (Wifydra style, 14 boards) is overkill here; three boards parked on channels 1/6/11 plus one hopper cover almost all real 2.4 GHz traffic for about USD 15–20 (unverified board prices).
- For an agent, line-oriented serial output is the easiest contract: the Zephyr 802.15.4 sniffer, ESP-SDR and Bit Pirate all speak text or framed binary over USB serial, so a single Rust "module host" on the Pi can wrap all of them behind MCP tools.
- Typical ESP32-C3 and C6 "super mini" boards cost about USD 2–6 each on AliExpress (unverified; no C3/C6 price was fetched this session).
- An ESP32 cannot follow BLE connections or capture wide-channel/MIMO Wi-Fi data frames the way a real monitor-mode adapter does (unverified, background knowledge); the USD 11 nRF52840 dongle or a Sniffle dongle remains the cheap answer for BLE, and the mt7921au for serious 802.11 capture.

### Gaps
- ESP32 Marauder, Bruce, GhostESP, Theengs and Espressif's own IDF sniffer examples were not opened this session; their licences, APIs and current state are unverified here.
- Current prices for C3/C6 super-mini boards and for the Sonoff ZBDongle-P were not fetched.
- BLE Channel Sounding / CTE direction-finding support on these chips in ESP-IDF or Rust was not researched.
- Whether SignalSweep emits serial JSON an agent can read directly is not stated (configuration is via a browser/Android app).

---

## 3. Cheap sub-GHz and other transceiver chips as receivers

### Takeaway
A CC1101 or SX127x on an ESP32 running rtl_433_ESP / OpenMQTTGateway decodes a large share of 315/433/868/915 MHz sensors and already emits JSON, but it hears roughly half the range of an RTL-SDR, handles OOK or FSK (not both at once), and only sees what its fixed demodulator is configured for. An SX1262 adds LoRa/Meshtastic sniffing and RSSI scanning. These chips are detectors and protocol receivers, not spectrum instruments.

### Cited Findings
- rtl_433_ESP supports CC1101, SX1276/SX1278 on ESP32 (C3/S3/C5 via custom SPI pins); ships 223 OOK and 153 FSK decoders refreshed from rtl_433 on 14 Aug 2026; GPL-3.0 — [rtl_433_ESP](https://github.com/NorthernMan54/rtl_433_ESP)
- OOK and FSK cannot run at once because the transceiver's own demodulator is used; mode is a compile-time define — [rtl_433_ESP](https://github.com/NorthernMan54/rtl_433_ESP)
- The author states the transceiver is less sensitive than an RTL-SDR, with about half the range or less; only five sensors are author-tested, the rest best effort; default RSSI gate is -82 dBm — [rtl_433_ESP](https://github.com/NorthernMan54/rtl_433_ESP)
- SX1262 is not listed as supported by rtl_433_ESP — [rtl_433_ESP](https://github.com/NorthernMan54/rtl_433_ESP)
- OpenMQTTGateway wraps this as receive-only "RTL_433" with JSON output and runtime retune by MQTT (`{"frequency":315.026}`); CC1101 bands 300–348, 387–464, 779–928 MHz — [OpenMQTTGateway docs](https://docs.openmqttgateway.com/use/rf)
- OpenMQTTGateway's docs say only OOK decoders are currently available, which conflicts with rtl_433_ESP's FSK decoder list — [OpenMQTTGateway docs](https://docs.openmqttgateway.com/use/rf) vs [rtl_433_ESP](https://github.com/NorthernMan54/rtl_433_ESP)
- Bit Pirate's Aug 2026 release added SX1262: LoRa packet RX/TX, RSSI monitor, frequency-activity scan, simple waterfall, channel-activity detection, full parameter control and a Meshtastic analysis shell; new boards Heltec WiFi LoRa 32 V4 and Vision Master T190 — [rtl-sdr.com](https://www.rtl-sdr.com/esp32-bit-pirate-updates-new-lora-and-meshtastic-analysis-features/)
- M5Stack sells the same radios as snap-on caps: SX1262 + GNSS cap USD 13.78, CC1101 + NFC cap USD 18.95 (fetched Nov 2026) — [M5Stack shop](https://shop.m5stack.com/collections/for-cardputer-adv)
- LTDZ (two ADF4351 + AD8307 log detector + STM32; 35 MHz–4.4 GHz): USD 46 in 2022; noise floor about -77 dBm, fixed wide RBW, reads 13 dB low, PC software needs an activation code — [Silicon Chip](https://www.siliconchip.com.au/Issue/SC/2022/January/El+Cheapo+Modules%3A+LTDZ+Spectrum+Analyser)
- ATS-Mini is an open ESP32-S3 + SI4732 pocket receiver with community firmware; LilyGo's T-Embed SI4732 lists at USD 38.96 (sold out, Nov 2026) — [ATS-Mini docs](https://esp32-si4732.github.io/ats-mini/), [LilyGo](https://lilygo.cc/en-de/collections/t-embed-series)

### Inferences

| Part | Typical cost | What it senses | Limits | Rust / ARM64 | Effort (est.) | Verdict |
|---|---|---|---|---|---|---|
| CC1101 module + ESP32 | USD 2–5 module (unverified) | 300–348 / 387–464 / 779–928 MHz OOK/2-FSK/GFSK sensors, RSSI sweep | about half RTL-SDR range, one modulation at a time, RBW 58–812 kHz (unverified) | rtl_433_ESP is C++/Arduino (GPL-3); Rust `cc1101` drivers exist but no decoder library (unverified) | 3–5 h, basic soldering | phase 1 |
| SX1262 module or Heltec V3/V4 | USD 5–8 module, about USD 20 board (unverified) | LoRa/Meshtastic packets, FSK, RSSI scan, CAD | cannot see LoRa below its configured SF/BW without scanning combos | Rust `lora-phy` (embassy) supports SX126x (unverified) | 3–6 h | phase 1 |
| SX1276/78 (LoRa32 boards) | about USD 15–20 (unverified) | rtl_433_ESP target + LoRa | older chip | same | 2–4 h | phase 1 alternative to CC1101 |
| LR1121 | about USD 10–15 (unverified) | sub-GHz + 2.4 GHz LoRa, GNSS/Wi-Fi scan | driver maturity | Rust support unknown | 8+ h | phase 2 |
| nRF24L01+ | about USD 1–2 (unverified) | 2.4 GHz channel-busy bit, 1 MHz steps, 126 channels (unverified) | single threshold near -64 dBm, no level (unverified) | trivial SPI | 2 h | not worth it: ESP-SDR on the same USD 5 board gives a real spectrum |
| Si4732/Si4735 module | about USD 5–10 chip module (unverified); USD 39 as T-Embed SI4732 | LW/MW/SW/FM tuned RSSI and SNR, i.e. an HF noise-floor logger | not a spectrum analyzer, audio-bandwidth receiver | I2C; Rust driver status unknown | 4–8 h | phase 2 (noise logger idea) |
| RFM69 / Si4463 | about USD 3–6 (unverified) | FSK/OOK sensors | overlaps CC1101 | fine | 3–5 h | not worth it alongside CC1101 |
| AD8317/AD8318 log detector | about USD 8–15 (unverified) | wideband total power 1 MHz–8 GHz (unverified) | no frequency selectivity; needs filter or swept LO | ADC read | 3 h | phase 2 as a "something strong is transmitting nearby" tripwire |
| LTDZ / ADF4351 + AD8307 | USD 46 (2022) | 35 MHz–4.4 GHz swept power | -77 dBm floor, wide RBW, licence-gated software | serial protocol | 2 h | not worth it: tinySA basic is better for about the same money |
| RTL8720 / BW16 | about USD 5–8 (unverified) | 5 GHz Wi-Fi | superseded by ESP32-C5 at USD 4.93 with Rust support | none in Rust | — | not worth it |

- The agent-friendly choice for sub-GHz is OpenMQTTGateway or a thin serial build of rtl_433_ESP, because output is already JSON and the frequency is settable at runtime.

### Gaps
- No fetched price for bare CC1101, SX1262, nRF24L01+, Si4732, AD8318, RFM69 modules.
- nRF24 scanner article failed to load; the RPD threshold and channel count above are background knowledge.
- Realistic sensitivity numbers in dBm for CC1101 vs RTL-SDR on the same sensor were not found; only the author's "about half the range" statement.
- SX1262 "spectrum scan" via RadioLib and the LR1121 were not researched from primary sources.

---

## 4. Ready-made hackable ESP32 multi-radio boards (USD 20–70)

### Takeaway
The best-supported "several modules in one" boards are ESP32-S3 handhelds that Bit Pirate already targets (LilyGo T-Embed CC1101 / Plus, M5 Cardputer ADV with radio caps, Heltec LoRa 32 V4); they save soldering but cost USD 50+ against roughly USD 10 of loose parts doing the same sensing, and they ship with transmit-capable firmware.

### Cited Findings
- LilyGo T-Embed CC1101 Plus: from USD 54.64 on lilygo.cc (fetched Nov 2026) — [LilyGo](https://lilygo.cc/en-de/collections/t-embed-series)
- A T-Embed CC1101 (ESP32-S3 + CC1101 + PN532 NFC + LCD + 1300 mAh battery) deal at USD 49.25 appears in a Slickdeals listing title (date not established) — [Slickdeals](https://slickdeals.net/f/19732302-lilygo-t-embed-esp32-s3-cc1101-low-power-sub-1-ghz-rf-transceiver-pn532-nfc-rfid-i2c-module-lcd-board-built-in-battery-1300mah-49-25)
- Bit Pirate runs on T-Embed CC1101 / Plus, M5 Cardputer and Cardputer ADV, M5 StickS3, Heltec WiFi LoRa 32 V4, XIAO ESP32-S3, S3 Zero/Super Mini and generic S3 devkits — [GitHub](https://github.com/geo-tp/ESP32-Bit-Pirate)
- Cardputer ADV radio caps: LoRa SX1262 USD 13.78, CC1101 + NFC USD 18.95 — [M5Stack shop](https://shop.m5stack.com/collections/for-cardputer-adv)
- ESP-SDR already has a Cardputer ADV / T-Dongle S3 port — [rtl-sdr.com comments](https://www.rtl-sdr.com/various-projects-independently-find-hidden/)
- Kismet has a dedicated "Zigbee: Catsniffer v3" datasource — [Kismet docs](https://kismetwireless.net/docs/readme/datasources/bluetooth-sniffle-ble/)
- Mesh Detect breakout (XIAO C3/S3 carrier for Remote ID + Meshtastic alerts): USD 15 board, USD 100 assembled kit — [CNX Software](https://www.cnx-software.com/2025/06/05/map-remote-id-enabled-drones-with-esp32-c3-s3-and-meshtastic-lora-modules/)

### Inferences

| Board | Price | Radios | Firmware openness / agent API | Verdict |
|---|---|---|---|---|
| LilyGo T-Embed CC1101 (Plus) | USD 49–55 | S3 Wi-Fi/BLE, CC1101, NFC, IR | Bit Pirate (MIT, serial CLI + Python), Bruce (unverified); also ESP-SDR-capable S3 | phase 2: convenient handheld, but duplicates USD 10 of modules |
| M5 Cardputer ADV + caps | caps USD 13.78 / 18.95; base price not fetched | S3 + SX1262 or CC1101 | Bit Pirate, ESP-SDR port | phase 2 |
| Heltec WiFi LoRa 32 V3/V4 | about USD 18–25 (unverified) | S3 + SX1262 | Meshtastic, Bit Pirate LoRa mode | phase 1 as the LoRa/Meshtastic module if he prefers no soldering |
| Electronic Cats CatSniffer v3 | price not retrieved; roughly USD 80–100 (unverified) | CC1352P (sub-GHz + 2.4 GHz 802.15.4/BLE) + SX1262 + RP2040 | open hardware, Sniffle + Kismet datasource | phase 2: the best single "protocol sniffer" board, but pricey for this budget |
| Flipper Zero and clones | about USD 170–200 genuine (unverified) | CC1101, NFC, IR | open firmware | not worth it at this budget |
| HackBat, PortaRF, T-Deck, T-Beam, T-Lora Pager | not researched | — | — | gap |

- Receive-only posture: every one of these boards ships firmware with deauth, replay or jamming-adjacent features. For this project, build a receive-only firmware profile or gate TX commands in the MCP layer rather than trusting stock menus.

### Gaps
- Prices for Cardputer ADV base unit, CatSniffer v3, Heltec V3/V4, T-Deck, T-Beam and T-Lora Pager were not retrieved (Tindie page failed; others not fetched).
- lilygo.cc shows "T-Embed CC1101 from USD 4.51", which is almost certainly an accessory variant and not the board price; treat as unresolved.
- Bruce, GhostESP and Marauder serial/JSON interfaces were not verified this session.

---

## 5. True DIY and cheap SDRs

### Takeaway
None of the true-DIY SDRs beats a USD 35–40 RTL-SDR on cost per unit of capability; they are worth building for learning (Pi Pico Rx for HF) but not as the pack's phase-1 receiver, and CaribouLite, the obvious Pi HAT, does not work on a Raspberry Pi 5 at all.

### Cited Findings
- RTL-SDR Blog prices (fetched Nov 2026): V3 dongle USD 34.95–39.95, V3 with dipole kit USD 44.95–49.95, V4L dongle USD 37.95, FM band-stop filter USD 16.95, AM high-pass filter USD 16.95, wideband LNA USD 19.95; V3 reaches 500 kHz–28.8 MHz by direct sampling, V4/V4L by a built-in upconverter — [rtl-sdr.com store](https://www.rtl-sdr.com/buy-rtl-sdr-dvb-t-dongles/)
- CaribouLite: "not supported on RPI5" because the RP1 I/O chip dropped the SMI interface; supported on Pi 1–4 and Zero; 30 MHz–6 GHz plus sub-1 GHz channel, 2.5 MHz bandwidth, 4 MS/s, TX-capable up to 14 dBm; 108 open issues — [CaribouLite GitHub](https://github.com/cariboulabs/cariboulite)
- Pi Pico Rx (Jon Dawson): Tayloe QSD + RP2040 ADC at 500 kS/s, 0–30 MHz, CW/SSB/AM/FM, 250 kHz front-end bandwidth, realistic MDS a little better than -100 dBm, OLED spectrum scope; USB described only for firmware loading on the page read — [101-things docs](https://101-things.readthedocs.io/en/latest/radio_receiver.html)
- MSi.SDR (MSi2500 + MSi001 RSP1 clone): about USD 40 (2022); one comparison found it clearly better than an RTL-SDR V3 at HF but far worse at VHF (under 20 dB vs about 40 dB SNR on the same signal), with spurs, erratic gain steps and weak filtering — [chrismalau.com](http://chrismalau.com/2022/02/19/msi-vs-rtl.html)
- HackRF clone: Great Scott Gadgets' test of one clone found very poor receive sensitivity and up to 22 dB less TX power above 1 GHz; the clone's designer disputes the unit tested; a 2021 reader comment cites USD 105 clone vs USD 339.95 genuine — [rtl-sdr.com](https://www.rtl-sdr.com/comparing-a-hackrf-clone-against-the-original/)
- tinySA basic: about USD 49 on AliExpress, USD 57.76 on Amazon (Dec 2025); 100 kHz–350 MHz low input, 240–960 MHz high input, RBW 2.6–640 kHz, USB-C CDC serial control; no LNA. tinySA Ultra about USD 139, Ultra+ ZS-406 about USD 138, ZS-407 about USD 178 — [CNX Software](https://www.cnx-software.com/2025/12/15/tinysa-is-a-low-cost-handheld-spectrum-analyzer-with-built-in-signal-generator/)
- Kismet lists RTL-SDR-based datasources for rtl_433, ADS-B, AIS and 900 MHz Zigbee — [Kismet docs](https://kismetwireless.net/docs/readme/datasources/datasources/)

### Inferences

| Option | Cost | Range / bandwidth | Linux ARM64 | Build effort (est.) | Verdict |
|---|---|---|---|---|---|
| RTL-SDR Blog V3 (or V4L) | USD 35–40 | 500 kHz–1.7 GHz (upper limit unverified), about 2.4 MS/s IQ | mature (rtl-sdr, SoapySDR, rtl_433, Kismet) | none | phase 1, keep one |
| tinySA basic | USD 49–58 | 100 kHz–960 MHz swept, calibrated dBm | USB serial; scriptable | none | phase 1 or 2, replaces the Ultra+ |
| Pi Pico Rx | parts about USD 15–30 (unverified) | 0–30 MHz, 250 kHz window | standalone; PC link unverified | 8–15 h, PCB + SMD | phase 2 (learning build) |
| MSi.SDR clone | about USD 40 (2022 price) | wide, 14-bit ADC per source | driver/legal status unverified (libmirisdr / SoapyMiri not checked) | none | not worth it: no better than RTL-SDR for this use, shakier software |
| HackRF clone | USD 105 in 2021; current price unverified | 1 MHz–6 GHz, 20 MS/s, TX | good | none | not worth it at this budget; quality lottery; TX device |
| CaribouLite | price not found | 30 MHz–6 GHz, 2.5 MHz | Pi 4 only | — | not worth it: incompatible with Pi 5 |
| ESP32-S31 + SoapyESPSDR | USD 19.70 | 2.4 GHz only, 20–40 MS/s | experimental Soapy driver | 2–4 h | phase 2 |
| uSDX, T41, FOBOS, Malahit clones, iCE40 + ADC SDRs, Pluto+/LibreSDR/ANTSDR E200, NanoVNA-as-receiver | not researched | — | — | — | gap (expected "not worth it" for phase 1 on cost) |

- A spare Raspberry Pi 4 or CM4 would make CaribouLite usable, but it remains a TX-capable board with a large open-issue count.

### Gaps
- No current prices or status found for uSDX, T41, FOBOS, Malahit clones, Pluto+/LibreSDR/ANTSDR E200, FPGA-based DIY SDRs, or current HackRF clone/PortaPack pricing; none of these were opened.
- MSi2500 Linux driver and legal status not checked.
- Whether current Pi Pico Rx firmware offers USB audio/IQ/CAT to a host, or runs on RP2350, is not stated on the page read.
- RTL-SDR V3 upper frequency limit and sample rate are from background knowledge.

---

## 6. Interference hunting: cheapest useful picture from about 100 kHz to 1 GHz

### Takeaway
Solar inverter and optimizer noise starts as 18–200 kHz switching fundamentals whose harmonics run through HF into low VHF, so the cheapest useful kit is (a) one RTL-SDR V3 in direct sampling with a home-made coax H-field probe for 0.5–30 MHz and up, and (b) a wire loop into a 192 kHz audio ADC for the fundamentals themselves; the tinySA basic at about USD 49 is the cheapest calibrated 100 kHz–960 MHz sweep and makes the USD 138–178 Ultra+ unnecessary for this job.

### Cited Findings
- ARRL: PV inverters radiate from the DC wiring with an 18–60 kHz fundamental switching frequency; power optimizers run 39–200 kHz; harmonics of both extend through HF into the lower VHF bands; optimizers still radiate when "off" in sunlight; the panels themselves are not the source — [ARRL solar RFI page](https://www.arrl.org/solar)
- ARRL advises recording time-of-day variation, affected bands, a quantitative level or spectrum snapshot, distance, grounding and cable-loop details — [ARRL solar RFI page](https://www.arrl.org/solar)
- A near-field probe made from 30 cm of semi-rigid coax (centre conductor looped back to the shield) on an RTL-SDR in direct sampling showed a 10 MHz OCXO clearly; commenters warn to switch off bias tee and suggest a 50-ohm series resistor and insulation — [rtl-sdr.com](https://www.rtl-sdr.com/creating-an-emc-probe-using-an-rtl-sdr-and-semi-rigid-coax/)
- A hand-held passive tuned loop (1–20 MHz, about 100 kHz bandwidth, with attenuator) on a portable receiver let one ham direction-find many noise sources; he found an SDR plus laptop cumbersome and plans to pair the loop with a tinySA, whose stock whip is useless at HF; tinySA MDS about -90 to -100 dBm — [fallows.ca](https://fallows.ca/wp/radio/radio-frequency-interference/rfi-spectrum-hunter-loop)
- tinySA basic covers 100 kHz–350 MHz and 240–960 MHz with 2.6–640 kHz RBW for about USD 49 — [CNX Software](https://www.cnx-software.com/2025/12/15/tinysa-is-a-low-cost-handheld-spectrum-analyzer-with-built-in-signal-generator/)
- V3 direct sampling spans 500 kHz–28.8 MHz behind a 25 MHz low-pass; extra HF filtering may be needed against strong AM broadcast — [rtl-sdr.com store](https://www.rtl-sdr.com/buy-rtl-sdr-dvb-t-dongles/)
- MSi.SDR clone was clearly better than the V3 at HF in one test — [chrismalau.com](http://chrismalau.com/2022/02/19/msi-vs-rtl.html)
- LTDZ-class swept detector boards start at 35 MHz with a -77 dBm floor — [Silicon Chip](https://www.siliconchip.com.au/Issue/SC/2022/January/El+Cheapo+Modules%3A+LTDZ+Spectrum+Analyser)

### Inferences

| Method | Cost | Coverage | Strength | Weakness | Verdict |
|---|---|---|---|---|---|
| RTL-SDR V3 direct sampling + DIY H-probe / loop | USD 35–40 + about USD 5 coax | 0.5–28.8 MHz, then 24 MHz–1.7 GHz in normal mode | waterfall shows the comb of switching harmonics; agent can diff spectra with a load on/off; same dongle does everything else | 8-bit ADC, poor dynamic range and aliasing in direct sampling (unverified); nothing below 500 kHz | phase 1 |
| Wire loop + 192 kHz audio ADC (USB sound card or a spare mic-array channel) | USD 0–15 (unverified) | about 1–96 kHz | captures the 18–60 kHz inverter and 39–96 kHz optimizer fundamentals directly, which identifies which device is switching | needs a 192 kHz-capable ADC; near-field only | phase 1 (almost free) |
| tinySA basic | USD 49–58 | 100 kHz–960 MHz, calibrated | fast wide sweeps, absolute dBm, USB serial for the agent, standalone screen for walking around | no LNA, clones exist, no demodulation | phase 1 if budget allows, else phase 2; choose over Ultra+ |
| AM/SW portable radio as sniffer | about USD 10–20 (unverified) | MW/SW | proven walk-around technique with the ear as detector | no data for the agent | phase 1 only if one is already owned |
| Si4732 tuned RSSI/SNR sweep | USD 5–39 | 150 kHz–30 MHz + FM (unverified range) | cheap, tiny, always-on noise-floor trend logger | coarse RSSI, narrow channel, not a spectrum | phase 2 |
| AD8318 + swept LO / LTDZ | USD 46+ | from 35 MHz | — | misses the whole HF problem band, poor floor | not worth it |
| MSi.SDR clone | about USD 40 | HF better than V3 | 14-bit | software risk, worse at VHF | not worth it |

- The noise sources are his own EG4 inverters and chargers at arm's length, so near-field probing is the main technique and receiver sensitivity is not the limiting factor; dynamic range and repeatability are. That favours the cheap options.
- Method the agent can automate: baseline sweep, switch one device (inverter, MPPT, charger, optimizer string) and re-sweep, difference the two, attribute spurs whose spacing equals that device's switching frequency.

### Gaps
- No source was fetched that directly benchmarks RTL-SDR V3 direct sampling against a tinySA basic for RFI work.
- ARRL's page gives no guidance on charge controllers specifically, and no spur-spacing signature.
- Price and availability of 192 kHz USB audio interfaces were not checked.
- tinySA clone avoidance details were not read (the tinySA wiki "where to buy" page was not opened).

---

## 7. Antennas, filters and probes better built than bought

### Takeaway
Only one build was verified from sources this session (the semi-rigid-coax H-field probe); the rest of this section is engineering judgement and needs a dedicated pass for printable designs.

### Cited Findings
- H-field probe from 30 cm semi-rigid coax, loop soldered back to shield, works with an RTL-SDR in direct sampling; insulate it and disable bias tee — [rtl-sdr.com](https://www.rtl-sdr.com/creating-an-emc-probe-using-an-rtl-sdr-and-semi-rigid-coax/)
- Passive tuned HF loop with wooden handle and 3D-printed instrument bracket used for neighbourhood RFI direction finding — [fallows.ca](https://fallows.ca/wp/radio/radio-frequency-interference/rfi-spectrum-hunter-loop)
- Bought reference prices to beat: RTL-SDR Blog dipole kit USD 17.95, FM band-stop USD 16.95, AM high-pass USD 16.95, wideband LNA USD 19.95 — [rtl-sdr.com store](https://www.rtl-sdr.com/buy-rtl-sdr-dvb-t-dongles/)
- The ESP32-C5 Mini has an IPEX connector, so external home-made antennas attach without board rework — [CNX Software](https://www.cnx-software.com/2026/04/28/esp32-c5-mini-usb-c-board-offers-dual-band-wifi-6-and-up-to-14x-gpio-pins-for-iot-projects/)

### Inferences (all unverified engineering judgement)
- Build: quarter-wave ground planes and dipoles from wire on SMA bulkhead connectors for 433 / 915 MHz (about USD 1–2 each, 30 min each); 3D-printed dipole centre and telescopic-whip holder; 2.4 GHz Yagi or cantenna on a printed boom for direction finding with an ESP32 module (2–3 h); untuned shielded loop for VLF/HF probing (1 h); E-field stub and H-field loop probe pair from coax (1 h); SMA 50-ohm terminator and a simple pi attenuator from 0805 resistors (1 h).
- Buy: SPF5189Z LNA boards (about USD 3–8, unverified) and SAW filter boards for 433/915 MHz (about USD 3–10, unverified) are cheaper bought as modules than built; defer the USD 16.95 FM band-stop filter until strong FM overload is actually observed at his rural site.
- A wire discone or PCB log-periodic/Vivaldi is a phase-2 item; a single wideband antenna matters only once the RTL-SDR is used for general survey beyond sensors.

### Gaps
- No Printables/Thingiverse designs were located or checked; no measured performance for printed-boom Yagis, wire discones or cheap PCB Vivaldi/LPDA boards was gathered.
- SPF5189Z, SAW-filter and attenuator prices are unverified.

---

## 8. Prior art: multi-radio rigs built from ESP32 / CC1101 / nRF-class parts

### Takeaway
Plenty of prior art proves each cheap-chip role separately (multi-channel Wi-Fi, Remote ID, sub-GHz JSON gateways, 802.15.4 sniffing, 2.4 GHz spectrum), and Bit Pirate comes closest to a unified scriptable tool, but nobody was found publishing an agent-driven, self-identifying, reflashable module array; that integration is the project's own contribution.

### Cited Findings
- The Wifydra: 1 ESP32 + 14 ESP8266, simultaneous 2.4 GHz channels, about USD 90, open BOM and Gerbers; follows Mike Spicer's WiFi Cactus / Kraken — [InfoconDB](https://infocondb.org/con/def-con/def-con-31/the-wifydra-multiheaded-rf-panopticon)
- JHewitt wardriver rev3: two ESP32-WROOM-32U devkits plus GPS, SIM800L, LCD, microSD; logs Wi-Fi and Bluetooth to WiGLE-format CSV; GPL-3.0 — [GitHub](https://github.com/JosephHewitt/wardriver_rev3)
- Mesh Mapper / Mesh Detect: ESP32-C3/S3 Remote ID detection relayed over Meshtastic to a Flask map, KML output — [CNX Software](https://www.cnx-software.com/2025/06/05/map-remote-id-enabled-drones-with-esp32-c3-s3-and-meshtastic-lora-modules/)
- SignalSweep: single XIAO ESP32-C5, dual-band Wi-Fi + BLE, Remote ID and tracker detection, GPL-3 — [Circuit Digest](https://circuitdigest.com/news/signalsweep-open-source-ble-and-wi-fi-scanner-on-esp32-c5-that-can-detect-airtags-flock-cameras-and-drones)
- ESP32 Bit Pirate: one S3 plus optional CC1101, nRF24, SX1262 and an external "radio expander", serial/web CLI, Python scripting, MIT — [GitHub](https://github.com/geo-tp/ESP32-Bit-Pirate), [rtl-sdr.com](https://www.rtl-sdr.com/esp32-bit-pirate-updates-new-lora-and-meshtastic-analysis-features/)
- OpenMQTTGateway + rtl_433_ESP: ESP32 + CC1101/SX127x publishing decoded sensors as JSON — [OpenMQTTGateway docs](https://docs.openmqttgateway.com/use/rf)
- ESPARGOS: eight ESP32s as a phase-coherent 2.4 GHz array for direction finding — [espargos.net/espsdr](https://espargos.net/espsdr/)
- Adafruit ESP-SDR waterfall on a C6 coprocessor feeding a host MCU over UART — [Adafruit guide](https://learn.adafruit.com/esp-sdr-on-metro-esp32-p4?view=all)
- Kismet as the aggregation layer for cheap sniffers (Sniffle dongle, nRF52840, CC2531, CatSniffer, rtl_433, Remote ID) — [Kismet docs](https://kismetwireless.net/docs/readme/datasources/datasources/)
- Search also surfaced, unread: justcallmekoko's ESP32DualBandWardriver, D3h420's WarMachine, CiferTech's ESP32-DIV and nRFBox, lukeswitz's AntiHunter, nyanBOX (ESP32 + three nRF24) — [ESP32DualBandWardriver releases](https://github.com/justcallmekoko/ESP32DualBandWardriver/releases), [WarMachine](https://github.com/D3h420/WarMachine), [ESP32-DIV](https://cifertech.github.io/ESP32-DIV), [nRFBox](https://github.com/cifertech/nRFBox), [AntiHunter](https://github.com/lukeswitz/AntiHunter)

### Inferences
- Reuse order for this project: (1) Kismet on the Pi 5 as a ready aggregation and REST layer for the bought/USB sniffers; (2) OpenMQTTGateway-style JSON for sub-GHz; (3) Espressif's Zephyr sniffer text protocol for 802.15.4; (4) ESP-SDR's framed protocol for spectra; (5) native Rust (esp-radio) modules for Wi-Fi/BLE where the project wants to own the firmware.
- Rust-first gap list: no Rust rtl_433 decoder library for transceiver chips, no Rust Remote ID receiver firmware, no Rust ESP-SDR. Each is a publishable contribution.

### Gaps
- The unread projects listed above were not evaluated for cost, licence or results.
- No distributed ESP32 RF-mapping grid with published accuracy results was found in this pass.
- Kismet's Remote ID datasource page (what hardware it expects) was not located.

---

## 9. Legal note

### Takeaway
Home-built receivers in quantities of five or fewer for personal use need no FCC equipment authorization but must still follow good engineering practice and must not cause harmful interference; nearly every cheap chip in this survey can also transmit, so receive-only has to be enforced in firmware and in the MCP layer.

### Cited Findings
- 47 CFR 15.23: devices that are not marketed, not built from a kit, and built in quantities of five or less for personal use need no equipment authorization; the builder is expected to use good engineering practice to meet the technical standards, and 15.5 (no harmful interference) still applies — [Cornell LII](https://www.law.cornell.edu/cfr/text/47/15.23)
- ESP-SDR's authors withheld a transmitter because arbitrary-signal TX could be misused for jamming, while community TX forks exist — [espargos.net/espsdr](https://espargos.net/espsdr/), [CNX Software](https://www.cnx-software.com/2026/10/04/esp-sdr-firmware-turns-esp32-into-a-2-4-5-ghz-software-defined-radio-sdr/)
- Bit Pirate includes deauth, sub-GHz replay, FM broadcast and BLE spoofing; SignalSweep sends scan requests unless set receive-only; CaribouLite and HackRF are transmitters — [Bit Pirate GitHub](https://github.com/geo-tp/ESP32-Bit-Pirate), [Circuit Digest](https://circuitdigest.com/news/signalsweep-open-source-ble-and-wi-fi-scanner-on-esp32-c5-that-can-detect-airtags-flock-cameras-and-drones), [CaribouLite GitHub](https://github.com/cariboulabs/cariboulite)

### Inferences
- Publishing the design as open source for others to build is different from marketing devices; selling assembled modules or kits would fall outside 15.23 (unverified legal reading; not legal advice).
- TX-capable parts to flag in the module registry: every ESP32 (Wi-Fi/BLE/802.15.4 and, with forks, arbitrary 2.4 GHz), CC1101, SX1262/SX127x, nRF24L01+, nRF52840, CC2652P. Receive-only by construction: RTL-SDR, tinySA basic in analyzer mode (it has a signal-generator mode), Si4732, AD8318, sound-card loop.

### Gaps
- No source was fetched on interception law (content of others' communications) or on FCC scanning-receiver rules; those need a separate check.

---

## 10. Recommended "cheapest credible" array, and the three closing deliverables

### Takeaway
Keep exactly one RTL-SDR, replace the second dongle, the Wi-Fi adapter and the tinySA Ultra+ with about USD 35 of ESP32-C6/C5 boards and CC1101/SX1262 modules, add a home-made probe and loop, and optionally a USD 49 tinySA basic: roughly USD 85–115 (USD 135–175 with the tinySA basic) against USD 266–336 for the bought array, losing mainly calibrated measurements above 960 MHz, a second simultaneous SDR channel and serious 802.11 capture.

### Cited Findings (prices used in the totals)
- RTL-SDR Blog V3 USD 34.95–39.95; FM filter USD 16.95 — [rtl-sdr.com store](https://www.rtl-sdr.com/buy-rtl-sdr-dvb-t-dongles/)
- ESP32-C5 Mini USD 4.93 — [CNX Software](https://www.cnx-software.com/2026/04/28/esp32-c5-mini-usb-c-board-offers-dual-band-wifi-6-and-up-to-14x-gpio-pins-for-iot-projects/)
- ESP32-S31 CoreBoard USD 19.70 — [CNX Software](https://www.cnx-software.com/2026/05/22/esp32-s31-development-boards-iot-smart-audio-and-hmi-applications/?amp=1)
- tinySA basic about USD 49; Ultra+ about USD 138–178 — [CNX Software](https://www.cnx-software.com/2025/12/15/tinysa-is-a-low-cost-handheld-spectrum-analyzer-with-built-in-signal-generator/)
- SX1262 cap USD 13.78, CC1101 cap USD 18.95 as upper-bound "no soldering" prices — [M5Stack shop](https://shop.m5stack.com/collections/for-cardputer-adv)
- Transceiver receivers reach about half the range of an RTL-SDR and cannot run OOK and FSK together — [rtl_433_ESP](https://github.com/NorthernMan54/rtl_433_ESP)
- ESP-SDR covers only 2.2–2.7 GHz (4.8–6.0 GHz on C5) — [espargos.net/espsdr](https://espargos.net/espsdr/)

### Inferences

Recommended ground-up array (phase 1)

| # | Module | Qty | Unit cost | Role |
|---|---|---|---|---|
| 1 | RTL-SDR Blog V3 (or V4L) | 1 | USD 35–40 | the only true IQ receiver below 1.7 GHz: rtl_433, ADS-B, HF/VHF noise, unknown signals |
| 2 | ESP32-C6 super-mini | 3 | about USD 3–6 (unverified) | reflashable roles: Wi-Fi 2.4 monitor, BLE scan, 802.15.4 sniff, ESP-SDR 2.4 GHz waterfall (C6 has continuous capture) |
| 3 | ESP32-C5 mini | 2 | USD 4.93 | 5 GHz Wi-Fi monitor, Remote ID (SignalSweep), ESP-SDR 5.8 GHz waterfall |
| 4 | CC1101 module on a spare ESP32 | 1 | about USD 3 (unverified) | always-on 433 or 915 MHz JSON sensor decoder (rtl_433_ESP), frees the RTL-SDR for other work |
| 5 | SX1262 module or Heltec LoRa 32 | 1 | about USD 6–20 (unverified) | LoRa/Meshtastic sniff, RSSI scan |
| 6 | nRF52840 dongle (from the earlier list) | 1 | USD 11 | BLE and 802.15.4 sniffer with Kismet/Wireshark support |
| 7 | DIY coax H/E probes, wire dipoles, VLF/HF loop | — | about USD 10 materials (unverified) | interference hunting and per-band antennas |
| 8 | Optional: tinySA basic | 1 | USD 49–58 | calibrated 100 kHz–960 MHz sweeps |
| 9 | Optional phase 2: ESP32-S31 CoreBoard | 1 | USD 19.70 | continuous 2.4 GHz IQ to GNU Radio |

(a) Bought array vs ground-up array

| | Bought array (earlier pass) | Ground-up array (this pass) |
|---|---|---|
| Parts | 2x RTL-SDR V3, FM filter, nRF52840 dongle, mt7921au Wi-Fi adapter, tinySA Ultra+ | 1x RTL-SDR V3, 3x C6, 2x C5, CC1101, SX1262, nRF52840 dongle, DIY probes/antennas; tinySA basic optional |
| Total | about USD 266–336 | about USD 85–115 core; about USD 135–175 with tinySA basic; about USD 155–195 with S31 too (C6/CC1101/SX1262/probe prices unverified) |
| Sub-GHz sensors | two SDRs, full rtl_433, two bands at once | one SDR plus one CC1101 decoder at about half range |
| LoRa/Meshtastic | SDR only (awkward) | native SX1262 packet sniff: better |
| Wi-Fi | full monitor mode incl. Wi-Fi 6/6E-class capture | header-level promiscuous capture on parked channels, 2.4 and 5 GHz: weaker per frame, wider in simultaneous coverage |
| BLE / Zigbee / Thread | nRF52840 only | nRF52840 plus several C6/C5 sniffers in parallel: better |
| Remote ID | via Wi-Fi adapter + BLE dongle | dedicated C5 module on both bands: better |
| 2.4 / 5.8 GHz raw spectrum | tinySA Ultra+ to 5.4–7.3 GHz, calibrated | ESP-SDR waterfalls, uncalibrated, undocumented feature |
| HF–UHF interference | Ultra+ sweeps + RTL-SDR | RTL-SDR + probes + VLF loop; calibrated only if tinySA basic is added, and then only to 960 MHz |
| Calibrated dBm above 960 MHz | yes | no |
| Agent reflash / self-test | limited (closed dongle firmware) | high: every ESP32 module is reflashable and can loop-test against its neighbours |
| Rust-first fit | host-side only | firmware and host |

(b) What a USD 40 RTL-SDR does that a pile of USD 5 chips cannot
- It delivers raw IQ continuously anywhere from 500 kHz to about 1.7 GHz. The cheap chips either return decoded packets from a fixed modem (CC1101, SX1262, nRF24, ESP32 Wi-Fi/BLE/802.15.4) or, with ESP-SDR, raw IQ only around 2.4 and 5 GHz.
- It can therefore receive signals nobody built a chip modem for: unknown or proprietary sensors, mixed OOK and FSK at the same time across about 2 MHz, ADS-B, AIS, NOAA weather radio, pagers, airband, FM broadcast, P25/trunking control channels, and the broadband comb of inverter harmonics at HF/VHF.
- It lets the agent look at a signal before knowing what it is (waterfall, bandwidth, modulation, timing), which is the core of interference hunting; a CC1101 gives one RSSI number per tuned channel at 58 kHz or wider resolution.
- It has roughly twice the range of a CC1101/SX127x on the same sensors, per the rtl_433_ESP author.
- Its software is mature on ARM64 Linux (rtl_433, SoapySDR, Kismet datasources), with nothing to flash.
- What it cannot do, and the chips can: anything at 2.4 or 5 GHz, native LoRa demodulation at low SNR, many channels at once for a few dollars each, and being reflashed by the agent.

(c) Outside-the-box ideas for ultra-cheap RF sensing
1. VLF loop into the mic-array ADC. A few dozen turns of wire into a spare 192 kHz audio channel records the 18–60 kHz inverter and 39–96 kHz optimizer fundamentals directly, time-aligned with the pack's acoustic data (coil whine and RF from the same converter in one capture).
2. Lock-in interference attribution. The agent toggles one charger or inverter at a slow known rate and correlates every spectrum bin with that on/off pattern; anything that follows the pattern belongs to that device. Works with any receiver, including coarse RSSI chips.
3. Telemetry-keyed noise fingerprints. Join RF noise level to the EG4 inverter and battery telemetry the pack already reads (PV watts, load, charge stage) to learn which operating state produces which spurs, without switching anything.
4. A USD 20 networked 2.4 GHz SDR. ESP32-S31 CoreBoard over Gigabit Ethernet at up to 40 MS/s via SoapyESPSDR, placed remotely on a cable instead of on a USB lead.
5. Frequency-translating front end for ESP-SDR. A cheap mixer plus ADF4351 LO that moves a sub-GHz or HF slice up into 1.85–2.7 GHz would turn the ESP32's 16–80 MS/s ADC into a wideband sub-GHz receiver. Nobody was found doing this; speculative, phase 3.
6. C5 as a 5.8 GHz drone-video tripwire. ESP-SDR spectrum on a USD 4.93 C5 flags analog/digital FPV energy that carries no Remote ID; C5VRX shows the chip can even demodulate analog video.
7. Switched-beam direction finding. Four USD 5 boards behind four printed 2.4 GHz cantennas pointing N/E/S/W; compare RSSI of the same frame or the same ESP-SDR bin for a bearing, no phase coherence needed.
8. Walk-the-property synthetic array. One module plus GPS and the agent's logging builds an RF heat map over time, instead of buying many nodes (wardriver precedent).
9. Si4732 as a permanent HF noise-floor logger. A USD 5–10 chip stepping through LW/MW/SW and logging RSSI/SNR every minute gives day/night and load-correlated noise trends for almost nothing.
10. Reuse the spare Home Assistant Yellow's built-in Zigbee/Thread radio as an 802.15.4 sniffer at zero cost (unverified: needs sniffer firmware for its Silicon Labs module; the spare Yellow comes from the owner's stored gear list).
11. CSI as a tamper/presence sensor between two pack modules with esp-csi-rs (pure no_std Rust), giving the agent motion awareness without a camera.
12. AD8318 log-detector tripwire. A USD 10 broadband power detector on an ADC pin tells the agent "something nearby is transmitting strongly" across 1 MHz–8 GHz so it can wake the right narrowband module (unverified part specs).
13. Kismet as free glue. Let Kismet ingest the Sniffle/nRF52840/rtl_433/Remote ID sources and expose one REST/JSON surface that the MCP server wraps, instead of writing a collector per radio.

### Gaps
- C3/C6 super-mini, bare CC1101, bare SX1262 and probe-material prices are unverified, so the ground-up total is an estimate; the stated range already spans the low and high guesses for those parts.
- The mt7921au vs ESP32 promiscuous capture comparison is from background knowledge, not a fetched source.
- Ideas 5, 7, 10 and 12 have no prior-art source behind them and need a feasibility check before any spend.
- The earlier pass's nRF52840 (USD 11) and mt7921au (USD 30) prices were taken from the task brief, not re-verified.
