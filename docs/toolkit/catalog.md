# Field toolkit catalog

The per-family tables behind the [field toolkit plan](README.md) (issue [#250](https://github.com/rafe-walker/sawbones-labs/issues/250)). The three research reports were merged and de-duplicated: a tool that appeared in more than one report has one row here, and where the reports disagreed the row says so.

## How to read the tables

- **Pi 5 fit.** "Yes" means plain C, C++, Go, Python or Rust on Linux with no x86-only code, so an aarch64 build is expected. It is not a test result; nothing has been run on a Pi 5. "Heavy" means it should not run live on the 4 GB Pi. "Mac" means it runs offline on the owner's Mac or on the fixed bench box.
- **Passive or active.** Passive means it only receives or reads. Active means it transmits, probes, writes or sits in the path.
- **How we run it.** *Wrap*: sandboxed native tool plus typed Rust adapter. *Native service*: a system service our Rust code talks to. *Port*: our own Rust, with the tool as the test oracle. *Own code*: nothing to port. *Offline*: Mac or bench box, on stored files proven by hash. *Reference*: used to cross-check, never run against live gear. *Skip* or *Reject*: not used. *Never*: class 4.
- **Hazard class.** 0 look, 1 safe, 2 armed, 3 keyed, 4 never (see [README section 5](README.md#5-hazard-classes-by-tool-family)). "Offline" analysis is class 0. Classes are proposals until the owner confirms the open conflicts in README section 10.
- **Status and verified.** **v**: checked against a project page, registry or package page during the research. **u**: from background knowledge and not re-checked; confirm before it drives a decision. Dates with no year were shown without a year on the release page. "Year not shown" is not a gap we filled by guessing.
- **Kali packages.** Kali and Debian share names, and Raspberry Pi OS Lite is Debian-based. A package name is a hint only; confirm with `apt-cache policy` on the real image.
- Copyleft tools (GPL, AGPL, NPSL) are always child processes. Our Apache-2.0 daemon links only permissive crates.

## 1. SDR and sub-GHz radio

| Tool | Purpose | Licence | Pi 5 fit | Passive / active | How we run it | Class | Status / verified |
| --- | --- | --- | --- | --- | --- | --- | --- |
| rtl_433 | Decode 433/315/868/915 MHz sensors, tyre-pressure sensors, weather stations, meters; replay a recording with `-r`; JSON output | GPL-2.0 (v) | Yes (v, arm64 listed) | Passive | Wrap (static CI build, no network, memory cap). Do not port the whole tool; maybe a few decoders later as WebAssembly | 0 | Active: 25.12 (Dec 2025) and Debian 26.07 (v). A 2026 overflow in one decoder was triggerable by a crafted signal (second-hand). Version pinning is part of the contract |
| rtl-sdr tools (`rtl_test`, `rtl_power`, `rtl_fm`, `rtl_sdr`, `rtl_tcp`) | Dongle drivers; raw IQ; wideband power survey; FM demodulation | GPL-2.0 (u) | Yes (u) | Passive | Native now; port the sweep to Rust later (`rtl-sdr-rs`, `rustfft`) with the `rtl_power` CSV as oracle | 0 | Upstream slow; the RTL-SDR Blog fork tracks V3 and V4 fixes (u). Rust `rtl-sdr-rs` 0.4.3, MPL-2.0, Oct 2026 (v) |
| SoapySDR and modules | Hardware abstraction for many SDRs | Boost-1.0 (u) | Yes (u) | Passive receive; can transmit on TX-capable hardware | Dependency of other builds only; Rust `soapysdr` 0.5.1 if a non-RTL device is added | 0 | No formal releases on the project page (v). Rust crate May 2026 (v) |
| readsb (ADS-B and Mode S) | Aircraft table from transponder broadcasts | GPL-3.0 (v) | Builds on Pi; page documents 32-bit notes, aarch64 not mentioned (v) | Passive | Wrap `aircraft.json` when the parked overhead-aircraft module is un-parked; consider `rs1090` later | 0 | No readable releases (v). Rust `rs1090` 0.8.1 and `adsb_deku` 0.8.0, MIT (v). Module is parked in DECISIONS |
| multimon-ng | Pager, DTMF, EAS and similar from audio | GPL-2.0 (u) | Yes (u) | Passive | Not in the image by default; if enabled, counts only, never message text | 0 for tone-only modes; pager text stays off | Pager text is other people's message content (legal section) |
| LoRa (our own link) | Receive on the SX1262 pods; the remote link transmits | Varies | Mixed | Receive passive; transmit on our own link | Write fresh in Rust (`lora-phy` 3.0.1, MIT or Apache-2.0, on the microcontroller). Kismet's LoRa source only if a sniffer is needed | 1 receive; 2 transmit on our channel | `lora-phy` July 2024 (v). Not a sniffer. A plain RTL-SDR does not decode LoRa in real time well (u) |
| `rtl_power` and sweeps (survey) | Power per frequency bin; A/B runs for noise | GPL-2.0 (u) | Yes (u) | Passive | See rtl-sdr row; the reference SDR stops near 1.75 GHz, so cards say what was not covered | 0 | Easy to reimplement with an FFT |
| GNU Radio | Signal-processing flowgraphs | GPL-3.0 (u) | Yes but heavy (u) | Depends on the flowgraph | Offline or lab only; not in the pack image | n/a | 3.10.12.0, Feb, year not shown (v) |
| gqrx, SDR++ | GUI receivers | GPL-3.0 (u) | GUI; poor fit headless | Passive | Reject inside the trust boundary | n/a | SDR++ is on rolling nightlies (v) |
| inspectrum | Offline IQ viewer | GPL-3.0 (u) | Yes (u); best on the Mac | Passive (offline) | Offline, by hash | 0 | In Kali (v) |
| Universal Radio Hacker | Capture, reverse engineer and replay signals | GPL-3.0 (v) | Heavy GUI | Can transmit | Reject: do not adopt | n/a | Archived (read-only) by its owner on 29 Mar 2026 (v) |
| HackRF tools (`hackrf_sweep`, `hackrf_transfer`) | Sweep; transmit arbitrary IQ | GPL-2.0 (v) | Yes (u) | `hackrf_sweep` passive; `hackrf_transfer` active | Out of scope: no HackRF in the design | 3 if a transmit-capable SDR is ever added | In Kali (v) |
| futuresdr, `hackrfone` | Rust SDR runtime; HackRF driver | Apache-2.0; MIT | n/a | n/a | Watch only; skip | n/a | futuresdr 0.10.0 says early state (v) |
| `sigmf` crate | IQ metadata sidecars | MIT | n/a | n/a | Use, or write the small JSON by hand | n/a | 0.1.1, Nov 2025, young (v) |

## 2. Wi-Fi

| Tool | Purpose | Licence | Pi 5 fit | Passive / active | How we run it | Class | Status / verified |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Kismet | One process and one REST and event API for Wi-Fi, BLE, Zigbee, sensor and ADS-B sources, Remote ID, GPS-tagged logs | GPL-2.0 (u) | Yes (u). Pi 5 onboard chip is not a monitor-mode choice | Passive capture by design | Wrap its REST API; the "look" tool for all wireless bands. Own namespace per source, each pinned to one adapter, data-frame payload logging off | 0 capture and census; 1 for channel lock and source start or stop | In Kali (v). Release pages unreadable; check the packaged version (v) |
| aircrack-ng suite | Monitor setup, capture, injection, key recovery | GPL-2.0 (u) | Yes (u) | `airodump-ng` passive; `aireplay-ng` injects | Wrap three binaries only: `airmon-ng`, `aireplay-ng` (owned target), `aircrack-ng` (offline, on the Mac). Never free arguments | `airodump-ng` 0; `aireplay-ng` 3; offline cracking 1 with an owned-BSSID gate | 1.7, 10 May, year not shown (v); slow cadence |
| hcxdumptool and hcxtools | Capture handshakes and PMKIDs; convert for hashcat | MIT (v) | Yes (u) | Active by default (associates, disconnects clients) (v) | `hcxdumptool`: wrap with active vectors off for capture mode; class 3 profile for the active audit. `hcxtools`: offline on the Mac | Capture-only 0; active vectors on 3 | hcxdumptool 8.0.0, 25 Oct, year not shown (v). Author warns about mt76 drivers (v); our chosen adapter is that family, so injection is unproven |
| bettercap | Recon and MITM framework; Wi-Fi, BLE and network modules | GPL-3.0 (v) | Yes (u) | Recon passive; `wifi.deauth`, `wifi.assoc`, `ble.write`, ARP and DNS spoofing active | Do not wrap. Recon is covered by Kismet and BlueZ; the audit goes through the wrapped class 3 tools | Spoofing and deauth: 4. Recon only: 0 (not used) | v2.41.7, 11 May, year not shown (v). Reports differ on class 3 versus 4 for its deauth; this plan picks 4 |
| iw and nl80211 | Scan, survey dump, channel and monitor mode | ISC (u) | Yes | `iw scan` sends probe requests; `survey dump` receive only (u) | Native now; port to `wl-nl80211` 0.9.0 (MIT, Nov 2026 (v)) once the card format is fixed. Oracle: `iw` output | Survey 0; active scan 2 (disputed with class 1, README section 10) | In Kali (v) |
| mdk4 | 802.11 flooding and attack modes | GPL-3.0 (u) | Yes (u) | Active, disruptive | Never | 4 | In Kali (v) |
| wifite, reaver, bully, pixiewps, cowpatty, asleap, fern-wifi-cracker | Automated WEP, WPA and WPS attacks; offline PSK cracking | Mixed GPL (u) | Mostly yes (u) | Mostly active | Skip: redundant with the wrapped tools | n/a | In Kali (v) |
| wifiphisher, eaphammer, wifipumpkin3, hostapd-mana, airgeddon | Rogue access point and credential capture | Mixed | n/a | Active, rogue AP | Never | 4 | In Kali (v) |
| wavemon, horst | Signal and 802.11 monitors | GPL (u) | Yes (u) | Passive | Skip: redundant with Kismet and `iw` | n/a | `horst` in Kali (v) |
| iperf3 | Throughput between two owned hosts | BSD-3 (u) | Yes | Sends bulk traffic | Thin typed wrapper, owned hosts only | 1 | JSON output (u) |

## 3. Bluetooth and BLE

| Tool | Purpose | Licence | Pi 5 fit | Passive / active | How we run it | Class | Status / verified |
| --- | --- | --- | --- | --- | --- | --- | --- |
| BlueZ (`bluetoothctl`, `btmon`, `hcitool`) | Pair, scan, GATT client; HCI trace | GPL-2.0 (u) | Yes (ships on Pi OS) | Scan and connect transmit; `btmon` passive | Native service; our Rust code uses `bluer` 0.17.4 (BSD-2, Jun 2025 (v)). Typed verbs: scan, connect, read, write | Scan 1; connect and read on an owned device 1; write to a battery or inverter 3 | `hcitool` is deprecated (u). Writes always return a before and after read |
| btleplug, bluest | Cross-platform BLE central libraries | MIT/Apache/BSD (v) | Yes | Active as a client | Mac side and existing meter clients (`btleplug` 0.13.5, Oct 2026 (v)). It cannot sniff | 1 | v |
| Kismet BLE sources | Passive advertising census via the HCI or a sniffer dongle | GPL-2.0 (u) | Yes | Passive | Part of the Kismet wrap | 0 | Sources include Ubertooth, TI CC2540, nRF51822, Sniffle on a Sonoff CC2652 (v) |
| nRF Sniffer for BLE | BLE sniffing into Wireshark | Nordic proprietary terms (u) | Python plugin runs on Linux (u) | Passive | Run native, or Kismet's nRF source; for the nRF52840 dongle on the shopping list | 0 | Active (u) |
| Sniffle | BLE 4 and 5 sniffer that follows connections | GPL-3.0 (u) | Python host (u) | Passive (has advertise and relay modes (u)) | Optional, only if a TI CC26xx board is bought | 0 | v1.11.0, 12 Jul 2025 (v). Does not support the nRF52840 dongle (v) |
| btlejack | Sniff, jam and hijack BLE connections | MIT (v) | Python | Sniff passive; jam and hijack active | Never | 4 | Needs micro:bits (v) |
| Ubertooth | BLE and Classic sniffing | GPL-2.0 (v) | Yes (u) | Passive | Skip: stagnant, not recommended to buy | n/a | Last release 2020-12 (v) |
| bluelog, btscanner, blue-hydra, crackle | Discovery, logging, offline cracking of captured legacy pairing | Mixed | Mixed | Passive or offline | Covered by BlueZ and Kismet; `crackle` is offline analysis | 0 | In Kali (v). Legacy pairing can be cracked offline; Secure Connections cannot (u) |
| bluesnarfer, redfang, spooftooph | Data theft, address brute force, identity cloning | Mixed | n/a | Active | Never | 4 | In Kali (v) |

## 4. Zigbee, Thread and other 802.15.4

| Tool | Purpose | Licence | Pi 5 fit | Passive / active | How we run it | Class | Status / verified |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Kismet Zigbee sources | Passive 802.15.4 capture | GPL-2.0 (u) | Yes | Passive | Part of the Kismet wrap; nRF52840 source | 0 | Hardware list incl. nRF52840, CC2531, CatSniffer V3 (v) |
| nRF Sniffer for 802.15.4 | Wireshark capture on nRF52840 | Nordic terms (u) | Yes (u) | Passive | Native, only if Kismet fails | 0 | v |
| KillerBee | Zigbee capture and injection | BSD (v) | Python | Capture passive; injectors active | Capture tools only if Kismet fails; no injection tools in the image | Capture 0; inject 3; flooders 4 | Hardware list dated; not the nRF52840 (v) |
| zigbee2mqtt | Zigbee to MQTT gateway | GPL-3.0 (v) | Yes (u) | Joins the network | Out of scope unless the site adopts Zigbee devices | n/a | Not a sniffer (v) |

## 5. GPS and other receivers

| Tool | Purpose | Licence | Pi 5 fit | Passive / active | How we run it | Class | Status / verified |
| --- | --- | --- | --- | --- | --- | --- | --- |
| gpsd | Share GNSS fix and time; PPS | BSD-2 (u) | Yes | Receive only | Native service; Rust `gpsd_proto` 1.0.0 or its JSON socket. Fix and time go into every capture card | 0 | Project page not fetched; Debian `gpsd` (u). Crates (v) |
| NOAA weather radio | SAME alerts and voice | Public broadcast | Yes | Passive | Low priority; decode with multimon-ng if ever enabled | 0 | v |
| Kismet Open Drone ID source | Remote ID detection | GPL-2.0 (u) | Yes | Passive | Part of the Kismet wrap; supports the owner's drone work | 0 | v |

## 6. Network capture and discovery

| Tool | Purpose | Licence | Pi 5 fit | Passive / active | How we run it | Class | Status / verified |
| --- | --- | --- | --- | --- | --- | --- | --- |
| dumpcap and tshark (Wireshark) | Capture and dissect, including Modbus/TCP and CAN | GPL-2.0+ (u) | Yes; dissect off-Pi where possible | `dumpcap` passive; `tshark` offline is local only | Wrap. The capture helper alone holds raw-network rights. Dissect stored files on the Mac or bench box. Counts come from our Rust parser; tshark is the cross-check | 0 | Active; flaw count rose sharply in 2026 (second-hand). In Kali and Debian (v) |
| tcpdump and libpcap | Light capture, BPF filters | BSD-3 (u) | Yes | Passive | Capture alternative to `dumpcap`; Rust `pcap` 2.5.0 (v) | 0 | In Kali (v) |
| Zeek | Protocol logs from traffic | BSD-3 (u) | Heavy on 4 GB; Mac | Passive | Offline on stored captures, one verb | 0 | Source package in Kali tracker (v) |
| Suricata | Intrusion detection with a Modbus parser | GPL-2.0 (u) | Heavy live; fine with `-r` | Passive in IDS mode | Offline on stored captures | 0 | In Debian (v) |
| ntopng | Traffic dashboard | GPL-3.0 community (u) | RAM hungry | Passive | Skip: dashboard, poor agent fit | n/a | In Debian (v) |
| nDPI | Protocol identification library | LGPL-3.0 (u) | Yes | Passive | Skip until a need appears | n/a | No maintained Rust binding found (v) |
| nmap and NSE | Host and port discovery, service detection | NPSL, custom, not OSI-approved (u) | Yes | Sends | Wrap, own namespace pinned to one interface, forced rate, owned targets only. Do not bundle in a distributed image without reading the licence | Discovery on owned hosts 1; NSE scripts outside the `safe` and `discovery` categories 2; exploit, brute-force and denial-of-service categories 4 | In Kali and Debian (v) |
| arp-scan | ARP sweep of the local segment | GPL-3.0 (u) | Yes | Sends (link-local) | Wrap; port the "what is on my network" diff to Rust later | 1 | In Kali (v). Trivial to port |
| netdiscover | ARP discovery | GPL (u) | Yes | Passive mode listens; active sends | Passive mode is covered by the capture path | 0 | In Kali (v) |
| masscan | Very fast port scanning | AGPL-3.0 (u) | Yes | Sends at high rate | Skip: overkill, AGPL | n/a | In Kali (v) |
| RustScan | Port scanner handing off to nmap | GPL-3.0 (u) | Yes | Sends | Optional | 1 | Added to Kali in 2026.3 (v) |
| p0f | Passive OS fingerprinting | LGPL (u) | Yes | Passive | Skip: dated | n/a | 3.09b from 2016 (u) |
| mitmproxy | Intercept HTTP(S) of one owned device | MIT (u) | Yes | Sits in the path | Wrap, one device on the isolated test access point only. Verbs: destinations contacted, uses TLS, validates certificates | 2 | In Kali (v) |
| dnsmasq | DHCP and DNS for the isolated test access point | GPL (u) | Yes | Serves | Configured by supervisor files only | Config change 2 | v |
| dig, mtr | DNS queries; path and loss tests | MPL-2.0 (dig); GPL-2.0 (mtr) (u) | Yes | Sends | Thin typed wrappers, JSON output | 1 | v |
| Rust capture crates | Parse and summarise captures | MIT or Apache-2.0 | Yes | n/a | Use: `pcap-file` 3.0.0, `pcap-parser` 0.17, `etherparse` 0.21.1, `libwifi` 0.5.0, `radiotap` 1.3.0 (last 2020), `hickory-proto` 0.26.4. Wrap `rtshark` 4.3.0 to keep Wireshark's GPL out | n/a | Versions and dates (v) |

## 7. Modbus, serial, CAN, OBD-II and battery management

| Tool | Purpose | Licence | Pi 5 fit | Passive / active | How we run it | Class | Status / verified |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Passive Modbus decode (tshark dissector, sigrok decoder, our own) | Read frames from a listen-only pod | Mixed | Yes | Passive | Port the part we need (frame parser in the supervisor); tshark and sigrok as second instruments | 0 | Listen-only pod is in P1 |
| Modbus reads and writes from our own code | Reads through the guard; writes through our encoder | `tokio-modbus` MIT or Apache-2.0 | Yes | Sends | Rust (`tokio-modbus` 0.17.0 for TCP, our RTU path for the guard) with `sawbones-modbus-encode`. Never `mbpoll`, pymodbus or NSE for writes | Reads 1; protection limits, charge voltages, inverter output 3; non-protective settings 2 (see README section 10, item 3) | v |
| Modbus discovery sweep | Find units and registers | n/a | Yes | Sends reads | Port to Rust as a read-only sweep (functions 0x03 and 0x04 only), rate limited, no broadcast; never while another master is polling | 1 | The guard allows 0x03, 0x04, 0x06 and 0x10 only; identify and diagnostic functions are blocked on RS485 by design |
| mbpoll, pymodbus (client, REPL, simulator), modbus-cli | Modbus master CLIs and a device simulator | GPL-3.0 (mbpoll); BSD-3 (pymodbus) | Yes | Sends | Mac or bench only, against the pymodbus simulator. The simulator doubles as a fake inverter for CI | n/a (offline) | `mbpoll` slow; `modbus-cli` dormant (u) |
| nmap `modbus-discover`, plcscan | Find Modbus slaves | NPSL; unverified | Yes | Sends | Nmap only on Modbus/TCP segments, never RS485; drop plcscan | 1 | plcscan dormant (u) |
| minicom, picocom, interceptty, slsnif | Serial terminals and sniffers | GPL | Yes | Terminals send keystrokes; sniffers add a hop | Skip as agent tools; our listen-only pod covers capture. Keep `picocom` for a human console | Listening 0; typing into a live device 2 | `interceptty`, `slsnif` dormant (u) |
| `serialport` and `tokio-serial` crates | Serial capture and listen-only reads | MPL-2.0; MIT | Yes | Passive | Use | 0 | 4.10.1 and 5.5.0 (v) |
| SunSpec `pysunspec2`, `eg4-bridge`, `PythonProtocolGateway` | Register-map references for solar gear | Apache-2.0 (v); MIT (v); not named (v) | Yes | Sends | Reference only: cross-check the owner's register map, never run against the live unit. A map taken from them is "documented", not "measured" | 0 as reading | `eg4-bridge` is a fork of an unmaintained tool (v). EG4 appears to use its own map, not SunSpec (u) |
| Solar Assistant (MQTT) | The decided first read path | Commercial | Already on the owner's unit | Reads on its own | Decided in DECISIONS | n/a | v |
| can-utils (`candump`, `cansend`, `cangen`, isotp, j1939) | SocketCAN tools | GPL-2.0, dual with BSD-3 in parts (u) | Yes | `candump` passive; others send | Port the part we need: `socketcan` 4.0.0 plus `can-dbc` 10.0.0. Raw `cansend` and `cangen` never exposed | `candump` and decode 0; read request 1; any send 2 or 3 by target, and none until the CAN guard exists | Active (u) |
| cantools, python-can | DBC decode and CAN library | MIT; LGPL-3 (u) | Yes | Passive decode | Reference and cross-check | 0 | In Debian (v) |
| SavvyCAN | GUI for CAN reverse engineering | MIT (u) | GUI; Mac | Mostly passive | Mac, human use | n/a | Slow (u) |
| CaringCaribou | UDS and XCP discovery and fuzzing | GPL-3.0 (v) | Yes | Sends; fuzzer can wedge an ECU | Do not expose to the agent; human-run against a spare ECU or simulator at most | Fuzzer or UDS write on anything real 3; ECU flash or emissions 4 | In Kali's CARsenal set (v) |
| ICSim | Fake instrument cluster on a virtual CAN bus | Not confirmed | Yes with a display | Local virtual bus | Use in P0 CI for the CAN pipeline | 0 | Dormant (u) |
| python-OBD, freediag, ELM327 dongles, our K-line pod | OBD-II and K-line diagnostics | GPL-2.0 (v for python-OBD); GPL-2.0 (u) | Yes | Sends | Our own Rust driver on our K-line pod (decided); freediag and python-OBD as references. Typed verbs: read PIDs, read DTCs, snapshot. Cards carry the scale definition | Reads (modes 01, 02, 03, 09) 1; clear codes 2 (engine off); actuator tests 2 (engine off), 3 (running); ECU writes not offered; emission changes 4 | Low activity (u); `obd2` crate 0.1.0 immature (v); no mature Linux J2534 stack found (gap) |
| Kali CARsenal | Car-hacking set for NetHunter | Mixed | Not for the Pi | Sends | Skip | n/a | Renamed and rewritten (v) |
| BMS tools (JBD, JK, Daly, Pylontech, PACE, SOK) | Read cell voltages and protection state; some write limits | Varies | Yes | Reads send; some write limits | Read-only through our own drivers; Bluetooth BMS apps stay a human tool. Survey after the owner confirms battery models | Reads 1; protection-limit writes 3 | Survey not done (owner fact) |
| J1939, UDS and other crates | Protocol support | `j1939` GPL-3.0-only; `uds-rs` and `rodbus` non-standard (v) | n/a | n/a | Wrap or check licence first; `rmodbus` 0.13.0 (Apache-2.0) for firmware and simulators | n/a | Licence check needed |

## 8. Firmware, hardware debugging and reverse engineering

| Tool | Purpose | Licence | Pi 5 fit | Passive / active | How we run it | Class | Status / verified |
| --- | --- | --- | --- | --- | --- | --- | --- |
| sigrok-cli, libsigrok, libsigrokdecode | Logic capture, protocol decoders, and many bought meters and scopes | GPL-3.0+ | Yes (v, `arch: any`) | Passive capture; some devices accept set-commands that write | Wrap (child process; typed "capture N seconds on channels X with decoder Y"). No maintained Rust binding; do not port | Capture 0; read a meter 0; set a range or mode 1 or 2 by device | Debian carries 0.7.2 (2021) (v); stable tarballs old (u). A "sigrok MCP server" exists on PyPI (title only seen); it is the thin-bridge pattern we reject |
| PulseView | GUI for sigrok | GPL-3.0+ | GUI; Mac | Passive | Mac, human use | n/a | v |
| OpenOCD | JTAG and SWD debug and flash | GPL-2.0 | Yes | Sends to target | Wrap as a sandboxed child. Verbs: identify chip, read flash, verify | Read and identify an owned board 1; write with recovery path and backup 2; no recovery path 3 | In Kali (v) |
| probe-rs | SWD and JTAG flash and debug (CLI and Rust library) | MIT or Apache-2.0 (v) | Yes | Sends to target | Use as a Rust library (0.32.0, Jul 2026 (v)) | As OpenOCD | v |
| flashrom | Read, write, verify SPI and I2C flash | GPL-2.0 | Yes | Writes to a chip | Wrap | As OpenOCD | In Kali (v) |
| esptool, avrdude, picotool | Flash and fuses for ESP, AVR and RP2040 and RP2350 | GPL-2.0+; GPL-2.0+; BSD-3 (v for picotool) | Yes | Writes, including fuses and one-time-programmable bits | Wrap as sandboxed children; the pack's own modules flash through the module protocol instead. Rust `espflash` 4.7.0 (v) | Burns that cannot be undone (fuse, OTP, eFuse) 3; others as OpenOCD | picotool in the Kali tracker (v) |
| binwalk v3 | Firmware signature scan and extraction | MIT | Yes | Offline | Link the Rust crate (3.1.0) or run as a child | 0 | Already Rust (v) |
| unblob | Extract many firmware container formats | MIT (v) | Yes (v, arm64 listed) | Offline | Sandboxed child, offline | 0 | v |
| Ghidra | Disassembly and decompiling | Apache-2.0 | Heavy; Mac | Offline | Offline on the Mac, on bytes the Pi captured, proven by hash | 0 | In Kali (v) |
| radare2, rizin | Reverse engineering frameworks | LGPL-3.0 and GPL parts (u) | Yes | Offline | Mac or bench, headless, typed "analyze this hash" verb | 0 | In Kali (v) |
| i2c-tools, spi-tools | Raw I2C and SPI on the Pi | GPL-2.0 | Yes | Sends; `i2cdetect` probes can upset some chips | Use `i2cdev` and `spidev` crates in our own drivers; do not expose `i2cdetect` | Reads on the Pi's own header 1; probing a live external bus 2 | v |
| Bus Pirate 5, 5XL, 6 | Multi-protocol tool with a FlatBuffers binary interface (BPIO2) | MIT, hardware and docs also MIT (v) | Yes (USB serial) | Sends; can drive voltages | Bench instrument over BPIO2; write a Rust client when needed. Never on the bank side | Passive modes 1; driving voltage or pull-ups on a live bus 2 | Active (v) |
| ChipWhisperer | Power analysis and glitching | GPL-3 software (u) | Unverified on Pi 5 | Sends glitches | Human-run on the bench against owned RP2350 test builds | Owned dev board off the bank, shielded, own supply 2; anything wired to the bank, a BMS or the trust root 4 | CW312T-RP2350 target exists (v) |
| JTAGulator | Find JTAG and UART pins on unknown boards | Unverified | Serial control | Drives pins | Human-run; rarely needed | n/a | Dormant (u) |

## 9. Audio and acoustics

| Tool | Purpose | Licence | Pi 5 fit | Passive / active | How we run it | Class | Status / verified |
| --- | --- | --- | --- | --- | --- | --- | --- |
| ffmpeg and ffprobe | Capture, resample, decode, `astats`, `ebur128` filters | LGPL-2.1+ by default; GPL if built with GPL parts (u) | Yes (u) | Passive | Wrap (first adapter), pinned apt, `ffprobe -of json` | 0 | Very active (u) |
| alsa-utils (`arecord`, `amixer`) | Multichannel capture, level control | GPL-2 (u) | Yes | Passive | Wrap, or `alsa` and `cpal` crates in our own code | 0; setting a level 1 | Active (u) |
| sox | Convert, filter, spectrogram | GPL-2 tool; LGPL-2.1 library (u) | Yes | Passive | Skip: unmaintained, covered by ffmpeg and our own operations | n/a | No upstream release since 2015 (u) |
| ODAS | Direction of arrival and tracking for a mic array | MIT (v) | Pi-class target (u) | Passive | Optional, only for the home-built raw-channel array; the chosen array chip reports direction itself | 0 | 243 commits, no tagged releases (v) |
| Audacity | GUI editor | GPL | Needs a display | n/a | Reject: not an agent-operable command line | n/a | v |
| Acoustic leak, arc and corona detection | Find leaks and arcing above 20 kHz | No packaged tool (u) | n/a | Passive | Own `compute` operations: band energy, crest factor, kurtosis, persistence. The owner's recordings are the oracle | 0 | Build item |
| Spectrum and DSP | Spectra, peaks, spectrograms | `rustfft`, `realfft` MIT or Apache-2.0 | Yes | Passive | Own `compute` operations | 0 | `rustfft` 6.4.1 (v) |

## 10. Camera, vision and thermal

| Tool | Purpose | Licence | Pi 5 fit | Passive / active | How we run it | Class | Status / verified |
| --- | --- | --- | --- | --- | --- | --- | --- |
| v4l-utils (`v4l2-ctl`) | List and set controls on UVC and V4L2 devices | GPL-2+ tools; LGPL libs (u) | Yes | Passive read; setting a control writes the camera | Wrap | Read 0; setting a control 1 | Active (u) |
| libcamera and rpicam-apps | Pi CSI camera stack | LGPL-2.1+; BSD-2 (u) | Yes; no hardware H.264 encoder on Pi 5 (u) | Passive | Wrap | 0 | First-party for Pi 5 (u) |
| OpenCV | Image operations | Apache-2.0 since 4.5 (u) | Yes (u) | Passive | Link for our own operations only where needed; keep out of the core ring, prefer pure Rust `image` crates | 0 | 4.14.0 and 5.0.0 on the releases page, year not shown (v) |
| tesseract | General OCR | Apache-2.0 (v) | Yes | Passive | Wrap on image handles; result always agent-inferred | 0 (result trips the latch) | Stable 5 line (v) |
| PaddleOCR | Higher-accuracy OCR | Apache-2.0 (v) | Use ONNX models from Rust; do not ship its runtime | Passive | Later, via `ort` or `rten` | 0 | 3.7.0, 11 Jun 2026 (v); ARM support not stated (v) |
| ocrs | Pure-Rust OCR | Apache-2.0 or MIT (v) | Yes | Passive | Candidate | 0 | Early preview, more errors than commercial engines, Latin script only (v) |
| ssocr | Seven-segment display reader | GPL-3 (u) | Yes | Passive | Own code instead (about a page over a cropped image) | 0 | Small and stable (u). No accuracy figures; first P1 task is a labelled set of the owner's displays |
| Frigate NVR | Camera recording, detection, MQTT events | MIT (v) | Heavy CPU detector on 4 GB (u) | Passive | Reject: second web server, auth and database inside the trust boundary. Borrow ideas only | n/a | 0.17.2 (v) |
| Obico | Print-failure detection | AGPL-3.0 (u) | Heavy | Passive | Reject; use frame difference and a bed-plane crop in our own operations plus the printer's local status | n/a | u |
| MLX90640 driver (Melexis) | Read and compensate the 32 by 24 thermal array | Apache-2.0 (v) | Yes (I2C) | Passive | Belongs on the module; port candidate for the compensation maths with the vendor code as oracle | 0 | 61 commits shown; vendor reference code (v) |
| flirpy, libuvc and v4l2 Y16 path | Parse FLIR Lepton, Boson and UVC radiometric frames | flirpy licence file not read (v partial); libuvc BSD (u) | Yes (flirpy tested on aarch64 and Pi (v)) | Passive | Later purchase; our own parser over `nusb` | 0 | 243 commits, no releases (v) |

## 11. Environmental and electrical measurement

| Tool | Purpose | Licence | Pi 5 fit | Passive / active | How we run it | Class | Status / verified |
| --- | --- | --- | --- | --- | --- | --- | --- |
| sigrok-cli (meters) | See section 8 | GPL-3.0+ | Yes | Passive | See section 8 | See section 8 | See section 8 |
| I2C and 1-Wire environment sensors (BME280, DS18B20) | Temperature, humidity, pressure | Kernel drivers GPL-2 (u) | Yes | Passive | Own drivers via `linux-embedded-hal`, `bme280` | 0 | u |

## 12. Forensics and evidence

| Tool | Purpose | Licence | Pi 5 fit | Passive / active | How we run it | Class | Status / verified |
| --- | --- | --- | --- | --- | --- | --- | --- |
| exiftool | Read and write metadata in photos, video and many formats | Artistic or GPL (u) | Yes (Perl) | Read flags passive on a copy; write flags modify the file | Wrap, read only, on blob handles; pinned apt | Read 0; write 2 | Very active, one maintainer (u) |
| sleuthkit | Walk disk images and filesystems (`fls`, `icat`, `mmls`) | Mixed IPL, CPL, GPL, MIT or BSD (v) | Yes | Passive on an image; never on a live mounted disk | Wrap, analysis only, read-only blob mount, no network | 0 | In Kali (v) |
| hashdeep, md5deep | Recursive hashing and audit | Public domain and GPL mix (u) | Yes | Passive | Port: BLAKE3 manifests (`b3sum` and `sha256sum` as oracle) | 0 | Latest 4.4, likely dormant since 2014 (u) |
| plaso and log2timeline | Super-timeline from many logs and artifacts | Apache-2.0 (v) | Memory-hungry (u) | Passive | Offline on the Mac for an owned phone or laptop image | 0 | Release 20261027 (v) |
| Our own session ledger | Hash-chained, append-only record of every call | Ours | n/a | n/a | Already the design in the MCP surface | n/a | Design |

## 13. General utilities (not tools for the agent)

| Tool | Purpose | Licence | Pi 5 fit | Passive / active | How we run it | Class | Status / verified |
| --- | --- | --- | --- | --- | --- | --- | --- |
| jq | Reshape JSON | MIT (u) | Yes | Passive | Not exposed; `serde_json` in the adapter replaces it | n/a | u |
| sqlite3 | Local queries | Public domain (u) | Yes | Passive | Not exposed; `rusqlite` is already the pick | n/a | u |
| gnuplot | Plots | Own permissive licence (u) | Yes | Passive | Not exposed; the store's render path covers it | n/a | u |
| DuckDB | Query Parquet captures | MIT (u) | Yes (u) | Passive | Not exposed; named in the design document for capture queries | n/a | u |

## Rust crates, summary

- **Use now in our own code:** `pcap-file`, `etherparse`, `socketcan`, `can-dbc`, `serialport`, `tokio-serial`, `tokio-modbus`, `rmodbus` (firmware and simulator), `probe-rs`, `espflash`, `nusb`, `object`, `goblin`, `i2cdev`, `spidev`, `linux-embedded-hal`, `binwalk`, `hickory-proto`, `bluer`, `wl-nl80211`, `gpsd_proto`, `rustfft` (all with dates and licences in the research, v).
- **Wrap, do not link:** `rtshark` (keeps Wireshark GPL out), `j1939` (GPL-3.0-only), `pktparse` (LGPL-3, last release 2021).
- **Check the licence first:** `rodbus` and `uds-rs`.
- **Gaps with nothing to embed:** sigrok bindings, nDPI bindings, a J2534 stack, a mature OBD or K-line crate, BLE sniffing, Zigbee sniffing.

## Hardware this toolkit assumes

| Need | For | Note |
| --- | --- | --- |
| Managed switch with port mirroring, or a two-network-card inline bridge | Passive capture of the home network and IoT | A Pi cannot sniff a switched network without one |
| Second Wi-Fi adapter with monitor mode | Wi-Fi capture | Injection on the chosen adapter is unproven |
| Listen-only RS485 pod and an isolated transmit pod with the bus guard | Modbus | In P1 and [#169](https://github.com/rafe-walker/sawbones-labs/issues/169) |
| Isolated CAN pod | Battery and inverter CAN | CAN guard does not exist yet ([#259](https://github.com/rafe-walker/sawbones-labs/issues/259)) |
| K-line interface | Older vehicle without CAN | Our own pod is decided |
| Logic analyser or the cheap open-tooling scope | sigrok | Per the project document |
| Debug probe, SOIC clip, USB-UART | OpenOCD, probe-rs, flashrom, esptool | Owned boards only |
| RTL-SDR (reference), nRF52840 dongle | Radio census, BLE and Zigbee sniffing | The dongle is a later-phase question in DECISIONS |
| ChipWhisperer and target boards | Glitching our own RP2350 builds | Separate purchase; not in the Bench Core |
