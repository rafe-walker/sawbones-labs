# Low-cost receive-focused RF sniffing array for a Pi-class portable field diagnostic pack (state: 25 Nov 2026)

Reading notes for the report writer:
- All prices are USD unless marked. "Fetched 2026-11-25" means the vendor/article page was read on that day; where the page itself is older, its date is given.
- Search tooling returned links without snippets, so every number below comes from a page that was actually fetched. Anything I know only from background knowledge and could not verify in this session is in a Gaps list and labelled "unverified background".
- Two items came from the owner's stored memory rather than the web: he owns a DJI Mavic 4 Pro (used below only as a suggested known Remote ID test emitter) and two Home Assistant Yellows, one a spare (used only in the ideas section).

## Key Question 1: SDR choice (RTL-SDR V3/V4/V4L, Airspy, HackRF One/Pro, KrakenSDR, others)

### Takeaway
The RTL-SDR Blog V4 is discontinued (tuner chip no longer made); in Nov 2026 the practical cheap picks are the RTL-SDR Blog V3 (about $35-40, in stock on Amazon US, works with stock drivers) or the new limited-run V4L ($37.95, not yet on Amazon US, needs a driver built from source on Raspberry Pi OS). HackRF Pro ($400) and KrakenSDR ($499-750) do not fit a $150-300 RF share; one or two RTL-SDRs plus a tinySA Ultra+ give more diagnostic value per dollar than any single wideband SDR at this budget.

### Cited Findings
- RTL-SDR Blog V4 is end-of-line: Rafael Micro stopped making the R828D tuner, the maker's stockpile ran out and the last chips sourced were faulty; post dated 14 May 2026. V3 stays in production; a V5 is only at early scoping — [RTL-SDR Blog V4 End Of Line](https://www.rtl-sdr.com/?p=65937)
- Marketplace "V4" listings are now frequently fakes (V4-style case around a V3-clone board); buy only from the official reseller list — [RTL-SDR Blog V4 End Of Line](https://www.rtl-sdr.com/?p=65937)
- Official store prices (fetched 2026-11-25): V4L dongle only $37.95; V4L with dipole kit $47.95; V4Lc (USB-C) $37.95; V3 dongle only $34.95-39.95; V3 with dipole kit $44.95-49.95; V3C (USB-C) $34.95-39.95. V3 is in stock on Amazon US; V4L is "not yet stocked" on Amazon US (AliExpress/eBay/site cart only). The original V4 no longer appears in the product list — [RTL-SDR Blog store](https://www.rtl-sdr.com/buy-rtl-sdr-dvb-t-dongles/)
- V4L design: R828S tuner (one fewer RF input than R828D), diplexed rather than triplexed input, so less front-end filtering but slightly better sensitivity; two-tone performance on par with V3; HF upconverter retained; 1 PPM TCXO; 4.5 V bias tee. The R828S stockpile is expected to last roughly a year; V5 news not expected until 2027 — [V4L now available](https://rtl-sdr.com/rtl-sdr-blog-v4l-lite-now-available-for-purchase)
- V4L Linux driver status: upstream Osmocom rtl-sdr has merged V4L support, but a commenter reports Raspberry Pi OS and Ubuntu 26.04 LTS packages do not yet include it, so a source build is needed. The users guide says to purge distro `librtlsdr`, build `github.com/osmocom/rtl-sdr` with cmake, install udev rules and blacklist `dvb_usb_rtl28xxu` — [V4L now available](https://rtl-sdr.com/rtl-sdr-blog-v4l-lite-now-available-for-purchase); [V4L Users Guide](https://www.rtl-sdr.com/V4L)
- V4L software caveats: SDR++ and SDRAngel are listed as not supported at this time; Osmocom/Soapy-based software may need reinstalling after the driver update — [V4L Users Guide](https://www.rtl-sdr.com/V4L)
- HF coverage on the RTL-SDR Blog dongles is 500 kHz-28.8 MHz (direct sampling on V3, upconverter on V4/V4L). Bias tee is software-switched (`rtl_biast -b 1`), and can supply up to 180 mA — [RTL-SDR Blog store](https://www.rtl-sdr.com/buy-rtl-sdr-dvb-t-dongles/); [V4L Users Guide](https://www.rtl-sdr.com/V4L)
- Airspy list prices (post dated 29 Jun 2026): R2 $169, Mini $99, HF+ Discovery $169, SpyVerter R2 $49 (20% summer sale took these to $135.20 / $79.20 / $135.20 / $39.20) — [Airspy 2026 Summer Sale](https://www.rtl-sdr.com/airspy-2026-summer-sale-20-off-all-airspy-products/)
- HackRF Pro: $400 at SparkFun (stock status on the page is inconsistent: "In stock" and "Backorder" both shown); 100 kHz-6 GHz operating, tunable 0 Hz-7.1 GHz; up to 20 Msps 8-bit I/Q, 40 Msps in 4-bit mode, 16-bit extended precision at low rates; USB 2.0 High-Speed with Type-C; built-in TCXO; half-duplex transceiver with a "facility to hardware-disable transmit mode"; runs existing HackRF One software — [SparkFun HackRF Pro](https://www.sparkfun.com/hackrf-pro.html); [Great Scott Gadgets HackRF Pro](https://greatscottgadgets.com/hackrf/pro)
- Genuine HackRF One is getting hard to buy new in the US: Adafruit dropped it (last listed $339.95) and SparkFun marks it retired; the same guide advises against unbranded marketplace clones (article updated 14 Oct 2026; affiliate source) — [HackRF One/PortaPack guide 2026](https://pasqualepillitteri.it/en/news/9488/hackrf-one-portapack-guide-2026)
- PortaPack H4M + HackRF (Rev 10 clone) all-in-one is EUR 395 at Lab401 with EU and US stock; ships with Mayhem firmware, 20 dB LNA and antennas — [Lab401 PortaPack H4M](https://lab401.com/products/portapack-h4m)
- KrakenSDR: 5 coherent RX channels, 24-1766 MHz; Crowd Supply lists $499 with case, $199 for five magnetic whips, recommends a 5 V / 2.4 A+ USB-C supply and a Raspberry Pi 4 image — [Crowd Supply KrakenSDR](https://www.crowdsupply.com/krakenrf/krakensdr). The RTL-SDR Blog store page lists KrakenSDR at $750 + shipping — [RTL-SDR Blog store](https://www.rtl-sdr.com/buy-rtl-sdr-dvb-t-dongles/). These two prices conflict; I could not determine which is current.
- New 2026 hardware: QuadRF (MoonRF) is a four-channel phase-coherent 4.9-6.0 GHz board that mates to a Raspberry Pi 5 over MIPI and runs an "RF camera" overlaying signal direction on video; no price in the article — [Hackaday, 20 Jun 2026](https://hackaday.com/2026/06/20/seeing-the-world-in-radio-waves-with-the-quadrf/)

### Inferences
- For a Pi OS (ARM64) pack that must work unattended, the V3 is the lower-risk dongle today: it is the model still in stable production, is stocked on Amazon US, and avoids the V4L's build-from-source driver step and SDR++ incompatibility. The V4L is worth it only if HF via upconverter matters more than driver friction.
- HackRF Pro's 20 Msps of 8-bit I and Q is 40 MB/s (320 Mbit/s) of sample data, most of what a USB 2.0 High-Speed link can carry (calculation from the cited sample format). It should have a host port to itself; it also overshoots the RF budget by itself.
- Airspy Mini ($99) adds dynamic range, not coverage, over an RTL-SDR; with a $150-300 RF share that money is better spent on the tinySA Ultra+ (covers to 5.4 or 7.3 GHz) for interference hunting.
- Every HackRF-family device and PortaPack is a transmitter. If one is ever added, the HackRF Pro's hardware TX-disable is the feature to cite for a "receive-only by default" policy.

### Gaps
- LimeSDR Mini 2.0, ADALM-Pluto, SDRplay (RSP1B/RSPdx), Nooelec NESDR, uSDR, Fobos SDR: no current US price or ARM64 driver status was fetched this session. Unverified background: SDRplay needs a closed-source API binary; Pluto and LimeSDR are transmit-capable and typically cost more than the whole RF share.
- RTL-SDR dongle current draw, maximum stable sample rate on a Pi, and V3/V4L tuner upper frequency were not stated on the pages fetched (unverified background: roughly 24-1766 MHz tuner range, about 2.4 Msps stable, about 0.3 A draw).
- hackrf_sweep sweep speed and HackRF One/Pro USB current draw are not given in the official docs page or product pages fetched.
- Cheap multi-RTL coherent builds (shared clock, noise-source calibration) as a KrakenSDR substitute: no source fetched; KrakenRF's DAQ/DSP code is open source and described as usable by DIY builders ([Crowd Supply](https://www.crowdsupply.com/krakenrf/krakensdr)), but I found no verified parts cost.

## Key Question 2: Dedicated protocol sniffers (BLE, 802.15.4, Wi-Fi monitor mode, sub-GHz/LoRa)

### Takeaway
Cheap single-purpose radios beat an SDR for packet-level work: an mt7921au USB adapter ($25-77) covers Wi-Fi 2.4/5/6 GHz monitor mode with an in-kernel driver, an $11 nRF52840 dongle covers 802.15.4/Zigbee/Thread (and BLE), a Sonoff CC2652P dongle running Sniffle covers BLE 5 including coded PHY and extended advertising, and an ESP32-C5 board ($15-25) adds a scriptable dual-band Wi-Fi 6 + BLE + 802.15.4 node. Kismet has first-class datasources for all of these.

### Cited Findings
- mt7921au (Wi-Fi 6E, 2.4/5/6 GHz) adapters with in-kernel support from Linux 5.18: Brostrend AX9L about $34-35, EDUP EP-AX1672 about $25-30, Panda PAU0F about $30, ALFA AWUS036AXML about $70-77 (also carries Bluetooth; 6.12+ recommended), Netgear A8000 about $59-79 (needs a udev workaround before kernel 6.4). All support active monitor — [morrownr USB-WiFi in-kernel list](https://github.com/morrownr/USB-WiFi/blob/main/home/USB_WiFi_Adapters_that_are_supported_with_Linux_in-kernel_drivers.md)
- Wi-Fi 7 (mt7925): Netgear A9000 about $90, supported since kernel 6.7 (6.12+ recommended), monitor and active monitor tested OK, USB ID only mainlined in 6.17 (backported to 6.12; otherwise udev workaround) — [morrownr USB-WiFi in-kernel list](https://github.com/morrownr/USB-WiFi/blob/main/home/USB_WiFi_Adapters_that_are_supported_with_Linux_in-kernel_drivers.md)
- Dual-band fallback: ALFA AWUS036ACM (mt7612u) about $43, in-kernel since 4.19, about 380 mA under heavy load, reported to work well on a Pi 4B; ALFA AWUS036ACHM (mt7610u) about $49-50 — [morrownr USB-WiFi in-kernel list](https://github.com/morrownr/USB-WiFi/blob/main/home/USB_WiFi_Adapters_that_are_supported_with_Linux_in-kernel_drivers.md)
- An Oct 2026 buyer's guide prices AWUS036AXML about $70 and AWUS036ACM about $50, calls mt7921u less mature than mt76x2u, says 6 GHz channels only appear once the regulatory domain is set, reports no 6 GHz monitor test results of its own, and warns high-power Realtek adapters can brown out a Pi or unpowered hub — [computingforgeeks, 16 Oct 2026](https://computingforgeeks.com/best-wifi-adapter-kali-linux/)
- mt76 monitor-mode issues are tracked in a separate repository, and US regulatory rules can block 6 GHz AP mode on USB adapters — [morrownr USB-WiFi in-kernel list](https://github.com/morrownr/USB-WiFi/blob/main/home/USB_WiFi_Adapters_that_are_supported_with_Linux_in-kernel_drivers.md)
- nRF52840 Dongle: $11.02 at DigiKey with 2,523 in stock (fetched 2026-11-25); BLE, Bluetooth mesh, Thread, Zigbee, 802.15.4, ANT, 2.4 GHz proprietary — [DigiKey nRF52840 dongle](https://www.digikey.com/en/product-highlight/n/nordic-semi/nrf52840-usb-dongle)
- nRF Sniffer for 802.15.4 ships as a Wireshark extcap (Python + pySerial, Wireshark 4.0+), tested on the nRF52840 Dongle/DK, nRF5340 DK and nRF54LM20 Dongle; Ubuntu 20.04 is among the tested hosts — [NordicSemiconductor/nRF-Sniffer-for-802.15.4](https://github.com/NordicSemiconductor/nRF-Sniffer-for-802.15.4)
- Sniffle hardware list: TI CC26x2R, CC2652RB, CC1352R, CC1352P, CC2652R7, CC1352P7, CC2651P3, CC1354P10 LaunchPads, the SONOFF CC2652P USB Dongle Plus, and Electronic Cats CatSniffer V3. Features: 1M/2M/coded PHY, BT5 extended advertising, connection following with channel-map/PHY updates; Python 3.9+ host scripts, Wireshark extcap, pcap output. It also contains active (transmitting) roles: scanner, initiator, advertiser, relay — [nccgroup/Sniffle](https://github.com/nccgroup/Sniffle)
- Sniffle baud trap: default link is 2,000,000 baud; Sonoff dongles with the plain CP2102 bridge top out at 921600 and need the `_1M` firmware build plus `-b 921600`; flashing the wrong variant can brick the device — [nccgroup/Sniffle](https://github.com/nccgroup/Sniffle)
- Kismet's Sniffle BLE source (tested on Sonoff ZBDongle-P): `source=sniffle_ble:name=...,device=/dev/serial/by-id/...`; parks on one advertising channel (37/38/39, no hopping, so about a third of adverts per dongle); `auxadv=true` adds BLE 5 extended advertising including Remote ID broadcasts; `phy=coded` sources do not see 1M adverts — [Kismet Sniffle BLE](https://kismetwireless.net/docs/readme/datasources/bluetooth-sniffle-ble/)
- Kismet datasource families (from the docs menu): Wi-Fi (Linux, HaLow MM8108, WiFi Coconut, bladeRF2), Bluetooth (Linux HCI, Ubertooth, TI CC2540, nRF51822, WCH BLE Analyzer Pro, Sniffle), Zigbee/802.15.4 (nRF52840, TI CC2531, CatSniffer v3, KW41Z, Freaklabs, RZUSBSTICK, 900 MHz via RTL-SDR), LoRa (LoraPipe), rtl_433 sensors, ADS-B and AIS via RTL-SDR, drones (Open Drone ID / Remote ID and AntSDR DroneID), cellular modem sources, radiation detectors — [Kismet datasources](https://kismetwireless.net/docs/readme/datasources/datasources/)
- ESP32-C5: dual-band (2.4 + 5 GHz) Wi-Fi 6, Bluetooth 5 LE, 802.15.4 (Zigbee 3.0/Thread 1.3); ESP32-C5-DevKitC-1 about $15 on AliExpress (Apr 2025 article); no 6 GHz. A reader cites Espressif's FAQ that the single RF path cannot do 2.4 and 5 GHz at once — [CNX Software](https://www.cnx-software.com/2025/04/30/esp32-c5-mass-production-esp32-c5-devkitc-1-board/). US-stock option: SparkFun Thing Plus ESP32-C5 $24.95, in stock (fetched 2026-11-25) — [SparkFun](https://www.sparkfun.com/sparkfun-thing-plus-esp32-c5.html)
- CatSniffer v3 combines a CC1352 (P7), an SX1262 LoRa radio and an RP2040; covers BLE 5/4.x, 802.15.4 (Zigbee/Thread), LoRa and sub-GHz (G)FSK at 433/868/915 MHz; runs Sniffle for BLE; host tools are the `catnip` CLI, Wireshark extcap and a Kismet source — [Electronic Cats RFSAM](https://electroniccats.github.io/RFSAM/tools/catsniffer). An older article priced the first CatSniffer at $129 pre-order (circa 2021) — [Hackster](https://www.hackster.io/news/go-on-the-prowl-for-iot-signals-with-catsniffer-32d4c88d3f88)
- Meshtastic's Python CLI/API (GPL-3.0) exposes what the device UI/app exposes and delivers received packets through publish-subscribe events; a "consistent output format for shell scripting" is still listed as a roadmap goal — [meshtastic/python](https://github.com/meshtastic/python)

### Inferences
- A cheap mt7921au stick (EDUP/Panda/Brostrend, $25-35) gives the same chipset and tri-band monitor capability as the $70-77 ALFA AWUS036AXML; the ALFA mainly adds better antennas/RF and a second Bluetooth HCI interface.
- Because a Sniffle dongle sits on one advertising channel, one dongle is a sampler, not a census. For "what BLE devices are near this truck" the Pi's own HCI radio (Kismet Linux HCI source) plus one Sniffle dongle with `auxadv=true` is a reasonable minimum; three dongles give full advert coverage.
- The nRF52840 dongle is the cheapest verified 802.15.4 sniffer and is supported by both Nordic's extcap and Kismet, so it should be the default Zigbee/Thread radio; the Sonoff/Sniffle dongle is then dedicated to BLE.
- Sniffle, CatSniffer, ESP32 and nRF firmware can all transmit. A receive-only pack should ship only sniffer firmware builds and expose only the receive scripts (`sniff_receiver.py`, not `advertiser.py`/`initiator.py`/relay) through the MCP server.

### Gaps
- Sonoff ZBDongle-P current US price: not fetched.
- Ubertooth One: Kismet still lists it as a datasource, but I did not verify its production/maintenance status (unverified background: it only handles classic-BT/BLE 4 era work and is not recommended for new builds).
- CC1101 USB sticks, RFM95/SX126x breakouts, Flipper Zero (price, CLI scripting), Heltec/other Meshtastic node prices: nothing fetched this session. Flipper and CC1101 tools are transmit-capable (unverified background).
- Whether the Home Assistant Yellow's onboard Silicon Labs radio can be used as a passive 802.15.4 sniffer without disturbing its Zigbee/Thread role was not researched.
- No source was found for real-world 6 GHz monitor-mode capture quality on mt7921au/mt7925 on a Raspberry Pi specifically.
- nRF Sniffer for Bluetooth LE (Nordic's BLE sniffer firmware for the same dongle) was not fetched; only the 802.15.4 sniffer was verified.

## Key Question 3: Agent-readable test equipment (tinySA Ultra/Ultra+, VNAs, others)

### Takeaway
The tinySA Ultra+ is the best-value "instrument" for an agent: about $138 (ZS406, to 5.4 GHz) or about $178 (ZS407, to 7.3 GHz), battery powered, with a documented USB serial console (`scan`, `scanraw`, `hop`, `selftest`, `vbat`) and a built-in signal generator that can double as a cabled reference source for checking the other radios.

### Cited Findings
- tinySA family (Dec 2025 article; AliExpress prices before tax/shipping): Basic about $49 (AliExpress) / $57.76 (Amazon), 100 kHz-350 MHz and 240-960 MHz; Ultra ZS405 about $139, to 800 MHz normal / 5.3 GHz ultra mode; Ultra+ ZS406 about $138, to 900 MHz normal / 5.4 GHz ultra, 5000 mAh, over 3 h; Ultra+ ZS407 about $178, to 7.3 GHz ultra. Amazon prices across the family ranged $57.76-$260.99 — [CNX Software, 15 Dec 2025](https://www.cnx-software.com/2025/12/15/tinysa-is-a-low-cost-handheld-spectrum-analyzer-with-built-in-signal-generator/)
- Ultra models add a switchable 20 dB LNA, a 0-31 dB step attenuator (not usable together with the LNA), microSD, and a harmonic mode reaching roughly 10-12 GHz; above the normal range the unit relies on harmonic mixing, which can show ghost signals and has no guaranteed amplitude accuracy beyond the rated ultra range. The built-in generator is not a calibrated signal source — [CNX Software, 15 Dec 2025](https://www.cnx-software.com/2025/12/15/tinysa-is-a-low-cost-handheld-spectrum-analyzer-with-built-in-signal-generator/)
- Serial protocol: device enumerates as a USB CDC virtual COM port (drivers in most Linux kernels). USB IDs: Ultra firmware V1.4273+ uses VID 0x1781 / PID 0x0A44; older firmware and the basic tinySA use 0x0483 / 0x5740, so detection code must check both — [tinySA wiki: USB interface](https://tinysa.org/wiki/pmwiki.php?n=Main.USBInterface)
- Useful commands: `scan start stop [points] [outmask]` (text; max 290 points basic / 450 Ultra), `scanraw` (binary 16-bit; dBm = value/32 - 174 on Ultra, - 128 on basic), `hop` (Ultra only: level at listed frequencies), `marker`, `sweep`, `output on|off` and `level -76..13` (generator), `selftest 0 0..9`, `info`, `version`, `vbat`, `capture` (screen dump). Parameter error checking is limited, and sweep-dependent commands return "keypad active" if an on-screen keypad is open — [tinySA wiki: USB interface](https://tinysa.org/wiki/pmwiki.php?n=Main.USBInterface)
- An unofficial Python wrapper (`tsapython`, GPL-2.0, v3.0.0) covers most documented commands for Basic/Ultra/Ultra+; it documents quirks: `scan` output can contain a stray `:` artifact, and `scanraw` is "finicky" on Linux and can freeze the screen until `resume` is sent — [LC-Linkous/TinySA_Python](https://github.com/LC-Linkous/TinySA_Python)
- VNAs (Apr 2026 roundup; no per-model prices): entry units start around $48-50, premium portables $180-260; NanoVNA-H and H4 cover 9 kHz-1.5 GHz, NanoVNA-F V2 50 kHz-3 GHz, LiteVNA-64 50 kHz-6.3 GHz; all are driven from a PC with NanoVNASaver-class software — [logix4u VNA roundup](https://www.logix4u.net/best-vector-network-analyzers-for-antenna-designers/)

### Inferences
- The ZS407 is the model that covers all of 5 GHz and 6 GHz Wi-Fi in its rated range; the ZS406/ZS405 stop at 5.4/5.3 GHz, i.e. mid 5 GHz band. For eero/Starlink-router Wi-Fi interference work the extra roughly $40 for the ZS407 is justified.
- Starlink user-terminal downlink sits in Ku band, outside every SDR in this budget; the tinySA Ultra's 10-12 GHz harmonic mode is the only item here that can even indicate energy there, and only qualitatively (amplitude not guaranteed, ghosts possible).
- A Rust MCP tool should talk to the tinySA with the text `scan`/`hop` commands rather than `scanraw`, given the documented Linux flakiness of `scanraw`, and should strip the `:` artifact.
- The tinySA has its own battery, so it does not load the Pi's USB power budget beyond charging current, and it keeps working as a hand-held when the Pi is down.

### Gaps
- tinySA trusted US sellers and current Amazon US price per model: the wiki "where to buy" page was blocked by a bot check; only the Dec 2025 AliExpress/Amazon figures above are verified.
- What each `selftest` index tests and the pass/fail output format are not documented on the wiki page fetched.
- Serial baud/framing for tinySA is not stated (unverified background: irrelevant for USB CDC).
- NanoVNA/LiteVNA serial command set and exact Nov 2026 prices were not fetched (unverified background: NanoVNA firmware exposes a similar text console with `scan`/`data`/`frequencies`).
- RF Explorer analyzers, cheap AD8317/AD8318 RF power meters and calibrated noise sources: nothing fetched.

## Key Question 4: Software to wrap rather than rewrite; machine-readable outputs

### Takeaway
Three tools already give stable structured output and should be wrapped, not rewritten: Kismet (REST/JSON over all packet radios), rtl_433 (JSON/MQTT/HTTP for sub-GHz sensors and TPMS, with built-in noise and stats reporting), and readsb (JSON files and a JSON TCP port for ADS-B). Sweeps come from `hackrf_sweep`-style CSV or the tinySA console. Rust SDR access is available through seify (FutureSDR), with its SoapySDR backend recommended for production.

### Cited Findings
- rtl_433 targets 433.92/868/315/345/915 MHz ISM devices with roughly 390 decoders, including many TPMS families (Toyota, Ford, Schrader, Renault, Honda, BMW, TRW; a few such as Jeep and Jansite TY02S are off by default). Outputs: JSON, CSV, KV, MQTT, InfluxDB, syslog, an HTTP API server, rtl_tcp pass-through. Inputs: RTL-SDR and SoapySDR (LimeSDR, PlutoSDR, HackRF One, SoapyRemote) — [merbanan/rtl_433](https://github.com/merbanan/rtl_433)
- rtl_433 hopping and health options: multiple `-f` with `-H` hop interval (default 600 s) or `-E hop`; `-M level` adds frequency, RSSI, SNR and noise per decode; `-M noise[:secs]` emits a noise-floor estimate (default every 10 s); `-M stats[:level][:interval]` emits decoder statistics (default every 600 s). The README warns network outputs are meant for trusted local networks — [merbanan/rtl_433](https://github.com/merbanan/rtl_433)
- Kismet's rtl_433 source is a small C helper that reads rtl_433's JSON and builds device records; it requires an RTL-SDR (not SoapySDR devices), defaults to 433.920 MHz, takes `channel=915MHz`, and only has signal levels (dBFS) when rtl_433 runs with `-M level`. After release 2026-10-R1 the helper no longer runs suid-root, so the Kismet user needs USB access via udev rules. Multiple RTL-SDRs need unique serials (`rtl_eeprom`) — [Kismet rtl_433 source](https://kismetwireless.net/docs/readme/datasources/sensor-sdrrtl433)
- Kismet datasource REST endpoints: `GET /datasource/all_sources.json` (running state, driver/config, packet counts, history; readonly role), `GET /datasource/by-uuid/{UUID}/source.json`, and admin-role POSTs for `add_source.cmd`, `set_channel.cmd`, `set_hop.cmd`, `pause_source.cmd`, `resume_source.cmd`, `close_source.cmd`, `open_source.cmd`; JSON, ekjson and prettyjson serializations — [Kismet datasources API](https://kismetwireless.net/docs/api/datasources)
- Kismet's API docs are organized into logins/sessions and keys, devices and device views, datasources, channels, alerts, GPS, system status, and an eventbus; an "API changes" page tracks breaking changes — [Kismet API index](https://kismetwireless.net/docs/api)
- readsb (wiedehopf fork): `--write-json` directory with `aircraft.json`, `--net-json-port` TCP stream (one JSON object per position), `--net-api-port` HTTP API, Beast/SBS/raw outputs, `--stats-every`, `--write-prom` Prometheus file; RTL-SDR is the documented device type, with SoapySDR/HackRF/bladeRF/Pluto present as build options — [wiedehopf/readsb](https://github.com/wiedehopf/readsb)
- hackrf_sweep output rows: date, time, Hz low, Hz high, bin width, sample count, then one dB value per bin; options include `-f min:max` (MHz), `-w` bin width 2445-5,000,000 Hz, `-1` one-shot, `-N` sweep count, `-B` binary — [HackRF tools docs](https://hackrf.readthedocs.io/en/latest/hackrf_tools.html)
- Rust: seify 0.20.0/0.20.1 (released 24 Jun 2026) is FutureSDR's hardware-abstraction crate with feature-flag backends `soapy` (default), `rtlsdr`, `hackrfone`, `bladerf1`, `hydrasdr`, `aaronia_http`, `dummy`; its docs say the native Rust drivers are still experimental and recommend the SoapySDR backend for production — [seify on docs.rs](https://docs.rs/crate/seify/0.20.0)
- Remote ID receive paths: Kismet has an "Open Drone ID / Remote ID" datasource and an AntSDR DroneID source; the Sniffle source with `auxadv=true` captures BLE 5 extended-advertising Remote ID — [Kismet datasources](https://kismetwireless.net/docs/readme/datasources/datasources/); [Kismet Sniffle BLE](https://kismetwireless.net/docs/readme/datasources/bluetooth-sniffle-ble/)
- Other Remote ID projects: OpenDroneID (Android receiver + C core library; Wi-Fi NAN and Bluetooth 5), proto17/dji_droneid (SDR decoder for DJI's proprietary DroneID), Mesh-Mapper (ESP32 Wi-Fi/BT Remote ID to a Flask map plus Meshtastic alerts), DragonSync (Remote ID + DJI DroneID gateway emitting CoT, MQTT/Home Assistant) — [Open source drone detection survey](https://www.jaredwatkins.com/research/drone-detection/open-source/)
- Mesh-Mapper hardware: ESP32-C3 or ESP32-S3 feeding a Heltec LoRa 32 V3 over serial; Mesh Detect breakout $15 on Tindie, assembled kit $100; repo github.com/colonelpanichacks/drone-mesh-mapper — [CNX Software, 5 Jun 2025](https://www.cnx-software.com/2025/06/05/map-remote-id-enabled-drones-with-esp32-c3-s3-and-meshtastic-lora-modules/)

### Inferences
- Architecture that falls out of the above: one long-running Kismet instance owns Wi-Fi, BLE (HCI + Sniffle), 802.15.4 (nRF52840), Remote ID and optionally rtl_433/ADS-B; the Rust MCP server is a thin client of Kismet's REST API plus direct wrappers for rtl_433 (JSON lines on stdout or its HTTP server), readsb (`aircraft.json`), and the tinySA serial console. This minimizes custom DSP.
- Each RTL-SDR can be claimed by only one process at a time, so "rtl_433 parked on 433/915" and "ADS-B or sweep" need two dongles or an agent-controlled scheduler that stops one service before starting another. Giving each dongle a unique EEPROM serial is a prerequisite for deterministic assignment.
- Use seify's `soapy` backend (C dependency) rather than the pure-Rust drivers for anything unattended, per the crate's own guidance; for a V4L dongle the underlying librtlsdr must be the freshly built one.
- rtl_433's `-M noise` output is a ready-made per-band noise-floor trend for inverter/charger interference at 315/433/915 MHz without any extra code.

### Gaps
- Exact Kismet JSON field names for per-source health (running/error/packet counters) are not in the docs page fetched; they must be read from a live `all_sources.json`.
- rtl_power, soapy_power, SDR++ server, GNU Radio, rtlsdr-rs / soapysdr-rs crates, dump1090-fa, Sparrow-wifi, WiGLE export, btmon, tshark extcap/`-T ek` JSON, whsniff/Zigbee sniffers: not fetched this session; no verified statements about their current output stability or ARM64 packaging. (Unverified background: tshark `-T ek`/`-T json` and rtl_power CSV are long-stable.)
- gr-gsm legality: no source fetched. See Key Question 9 for the statutory reasoning that puts cellular traffic off limits.
- Latest tagged rtl_433 release/version was not shown on the README.

## Key Question 5: Direction finding and location

### Takeaway
Coherent direction finding (KrakenSDR, $499-750 plus about $199 antennas, 24-1766 MHz only) does not fit this budget or a one-person carry pack; a GPS-tagged signal-strength walk with one directional antenna, logged through Kismet and/or tinySA level readings, is the practical method.

### Cited Findings
- KrakenSDR is a 5-channel coherent RTL-based receiver (24-1766 MHz) using correlative interferometry / MUSIC-style algorithms; open-source DAQ and DSP, web UI, closed-source Android app for bearings, mapping and turn-by-turn; reference host is a Pi 4 with a ready SD image; supply recommendation 5 V / 2.4 A+ — [Crowd Supply KrakenSDR](https://www.crowdsupply.com/krakenrf/krakensdr)
- Price conflict: $499 (Crowd Supply) vs $750 + shipping (RTL-SDR Blog store, fetched 2026-11-25); five magnetic whips $199 — [Crowd Supply KrakenSDR](https://www.crowdsupply.com/krakenrf/krakensdr); [RTL-SDR Blog store](https://www.rtl-sdr.com/buy-rtl-sdr-dvb-t-dongles/)
- KrakenRF lists planned (not shipped) products: fixed PCB antenna arrays, an all-in-one unit, a coherent downconverter for higher bands, and Yagi/TDoA-based finders — [Crowd Supply KrakenSDR](https://www.crowdsupply.com/krakenrf/krakensdr)
- QuadRF does phase-coherent 4-channel DF only in 4.9-6.0 GHz, paired to a Pi 5 — [Hackaday, 20 Jun 2026](https://hackaday.com/2026/06/20/seeing-the-world-in-radio-waves-with-the-quadrf/)
- Kismet supports GPS and can attach a "meta" GPS to a datasource via the REST API, so sightings can be location-tagged per source — [Kismet datasources](https://kismetwireless.net/docs/readme/datasources/datasources/)
- tinySA Ultra's `hop` command returns the measured level in dBm at specified frequencies, suitable for repeated spot measurements — [tinySA wiki: USB interface](https://tinysa.org/wiki/pmwiki.php?n=Main.USBInterface)

### Inferences
- KrakenSDR's top frequency (1766 MHz) excludes every 2.4/5/6 GHz emitter (Wi-Fi, BLE, Zigbee, drone links), which are a large share of this pack's targets; it mainly helps with sub-GHz and VHF/UHF hunts.
- For a one-person tool: (1) coarse location from a GPS-tagged RSSI heatmap while walking the property (Kismet per-device signal + GPS; tinySA `hop` + GPS for non-packet noise); (2) final approach with a directional antenna "peak/null" sweep; (3) last metre with near-field probes. All three are within the $300 tier.
- The agent can direct the human ("turn 30 degrees left, walk 10 m") from live RSSI, which substitutes operator guidance for expensive coherent hardware.

### Gaps
- No verified source on accuracy of RSSI walking surveys or on cheap DIY coherent multi-RTL builds in 2026.
- Kismet GPS/gpsd configuration details were not fetched.

## Key Question 6: Antennas, LNAs, filters, and agent-controlled antenna switching

### Takeaway
The verified low-cost kit is the RTL-SDR Blog accessory line: dipole kit ($17.95), broadcast-FM band-stop ($16.95), bias-tee wideband LNA 100 MHz-6 GHz ($19.95), and the YouLoop passive loop ($29.97) for HF noise work. GPIO-controllable SP4T/SP6T switch boards exist as cheap modules, but I could not verify their prices; lab-grade USB switches (Mini-Circuits) publish no prices on the page.

### Cited Findings
- RTL-SDR Blog accessories (fetched 2026-11-25): dipole antenna kit $17.95; Wideband LNA V2 $19.95 (100 MHz-6 GHz, bias-tee powered, 3-5 V at 90 mA); broadcast FM band-stop filter $16.95 (rejects 88-108 MHz); broadcast AM high-pass $16.95; YouLoop passive loop $29.97 sale / $39.95 regular (10 kHz-30 MHz HF, usable to 300 MHz); active L-band patch $59.95 (1525-1660 MHz); SMA adapter sets $14.95-18.95. LNA, filters and dipole kit are stocked on Amazon US; the YouLoop has no Amazon listing — [RTL-SDR Blog store](https://www.rtl-sdr.com/buy-rtl-sdr-dvb-t-dongles/)
- The RTL-SDR bias tee supplies 4.5 V and up to 180 mA, enough for the Wideband LNA V2 (90 mA) — [RTL-SDR Blog store](https://www.rtl-sdr.com/buy-rtl-sdr-dvb-t-dongles/); [V4L Users Guide](https://www.rtl-sdr.com/V4L)
- Candidate switch parts seen in current listings: HMC7992 non-reflective SP4T module (0.1-6 GHz, 3-5 V supply, about 45 dB isolation, about 0.6 dB insertion loss per listing title) — [Walmart.ca listing](https://www.walmart.ca/en/ip/HMC7992-Non-Reflective-SP4T-RF-Switch-Module-PCB-Material-0-1-6GHz-Frequency-Range-3V-5V-Power-Supply-45dB-Isolation-0-6dB-Insertion-Loss-58dBm-IIP3/4LT6JIAL4ULJ); MikroElektronika RF Switch Click (PE42442 SP4T, 6 GHz, MIKROE-6914) — [RS listing](https://uk.rs-online.com/web/p/communication-wireless-development-tools/0842275); pSemi PE42540 SP4T eval board (10 Hz-8 GHz) — [DigiKey EK42540-08](https://www.digikey.com.au/en/products/detail/psemi/EK42540-08/6706893). Only the listing titles were read; prices were not retrieved.
- Mini-Circuits' USB solid-state switch family (USB-1SP4T, USB-1SP8T, USB-2SP4T variants; USB/Ethernet/SPI control) lists no prices on its web page — [Mini-Circuits](https://www.minicircuits.com/WebStore/RF-Solid-State-Compact-Switch.html)
- The Lab401 PortaPack bundle shows what a typical carry antenna set looks like: 40 MHz-6 GHz telescopic, 700-2700 MHz, 2.4/5/5.8 GHz blade, plus a 20 dB 50 MHz-6 GHz LNA — [Lab401 PortaPack H4M](https://lab401.com/products/portapack-h4m)

### Inferences
- An SP4T on two or three Pi GPIO lines in front of one RTL-SDR lets the agent choose between, for example, dipole (VHF/UHF), 915/433 whip, 1090 MHz antenna and a near-field probe or 50-ohm terminator. A terminated port is valuable in itself: it gives the agent a "receiver noise only" reference to tell internal spurs from real signals.
- The RTL-SDR Wideband LNA starts at 100 MHz, so it does not help with HF inverter/charger noise; the YouLoop (passive, 10 kHz-30 MHz) into a V3 in direct-sampling mode or into the tinySA is the better HF path.
- Near solar inverters, FM broadcast overload is unlikely at a rural site, but the $16.95 FM notch is cheap insurance for the RTL-SDR's limited dynamic range when an LNA is in line.
- Solid-state switch boards will add roughly 0.6-1 dB loss per pass (per the HMC7992 listing figure) and their own isolation limits; fine for surveying, but the health check should measure through-loss on each port with the tinySA generator.

### Gaps
- Verified US prices for SP4T/SP6T/SP8T switch modules and for any USB-native cheap switch: not obtained (eBay page fetch failed). Unverified background: bare HMC/PE-series modules sell for roughly $10-40; Mini-Circuits USB switches cost several hundred dollars.
- Discone, log-periodic PCB antennas, Yagis/patches for 2.4/5 GHz hunting, 915 MHz whips, 1090 MHz antennas: no prices or reviews fetched.
- RTL-SDR Blog ADS-B LNA price is not on the store page text retrieved.

## Key Question 7: EMI / near-field probes for locating noisy electronics

### Takeaway
Near-field H/E probe sets are cheap enough to include at either tier: generic four-probe kits have sold for about $5, the RF Explorer kit is $89, and DIY semi-rigid coax loops perform comparably. Paired with the tinySA Ultra's built-in 20 dB LNA they are the most direct tool for pinning inverter, charger and DC-DC converter noise to a specific box or cable.

### Cited Findings
- A generic kit of three H-field loops plus one E-field stub was bought for about $5 on sale and tested against a small spectrum analyzer; a reader summary of the test says all but the smallest loop clearly showed an Arduino's 16 MHz clock, and the largest also picked up a broadcast station. Commenters note cheap probes respond to both common-mode and differential-mode signals — [Hackaday, 3 Jan 2023](https://hackaday.com/2023/01/03/near-field-emi-probes-any-good)
- DIY options named there: a loop of semi-rigid coax, coax with the centre conductor soldered back to the shield, and the OpenFieldProbe stripline PCB design — [Hackaday, 3 Jan 2023](https://hackaday.com/2023/01/03/near-field-emi-probes-any-good)
- RF Explorer Near Field Antenna Kit: 25/15/5 mm H-loops plus a 10 mm E-probe with semi-rigid coax and case, $89 via Seeed Studio, stated to 6 GHz (a later post says 7 GHz). An experienced tester measured about 6 dB ripple from 100-1000 MHz on the 25 mm loop and said his home-made semi-rigid probes were about as good as lab-brand probes; smaller loops trade sensitivity for spatial resolution; a broadband amplifier is described as a common companion accessory — [EEVblog forum thread](https://eevblog.com/forum/testgear/new-affordable-emc-probe-set-at-a-bargain-price-from-ariel-rocholl_s-lab)
- tinySA Ultra models have a switchable 20 dB LNA built in — [CNX Software, 15 Dec 2025](https://www.cnx-software.com/2025/12/15/tinysa-is-a-low-cost-handheld-spectrum-analyzer-with-built-in-signal-generator/)
- The interference-identification carve-out in federal wiretap law covers intercepting a transmission that is causing harmful interference to lawful stations or consumer electronics, to the extent needed to find the source — [18 U.S.C. 2511(2)(g)(iv)](https://www.law.cornell.edu/uscode/text/18/2511)

### Inferences
- Workflow for inverter/charger noise: (1) rtl_433 `-M noise` and tinySA sweeps show which bands are raised; (2) switch the suspect device off/on under agent direction and difference the sweeps; (3) walk the YouLoop or large H-loop along cables to find where the field peaks; (4) small loop to pin the component. Probe readings are relative, not calibrated field strength, which is fine for A/B and "hot/cold" hunting.
- Switch-mode inverter and MPPT noise is concentrated from tens of kHz to tens of MHz, so the instrument's low end matters more than its top end here: tinySA (from 100 kHz) and RTL-SDR HF modes (from 500 kHz) cover it; a 100 MHz+ LNA does not.
- A clip-on ferrite RF current probe (DIY) would measure common-mode current on PV and battery cables more repeatably than a hand-held loop; worth a line in the BOM as a DIY item.

### Gaps
- TekBox TBPS01 price and contents: the distributor page returned no content.
- No 2026 test of specific Amazon near-field kits with a tinySA was found; frequency ranges printed on cheap kits (e.g. "9 kHz-3 GHz" in listing titles) are unverified.
- No source fetched on typical inverter emission spectra; the frequency statement above is unverified background.

## Key Question 8: USB and power realities on a Pi

### Takeaway
A Raspberry Pi 5 gives only 600 mA total to its USB ports unless it negotiates a 5 V / 5 A supply (then 1.6 A); several SDRs, a Wi-Fi adapter and serial sniffers will exceed the lower limit, so plan on a powered hub or a 5 A-capable pack rail and treat USB brown-out as a first-class health check.

### Cited Findings
- Pi 5: with a supply that does not advertise 5 V / 5 A, total downstream USB current is capped at 600 mA; with 5 V / 5 A it is 1.6 A. `usb_max_current_enable=1` raises the limit to 1.6 A and is set automatically when 5 A is negotiated or when `PSU_MAX_CURRENT=5000` is set in the EEPROM config (the route for packs powering via GPIO/non-PD rails). The paper gives totals only, no per-port limits — [Raspberry Pi white paper: USB power delivery on Pi 5](https://pip-assets.raspberrypi.com/categories/685-app-notes-guides-whitepapers/documents/RP-009856-WP-1-USB%20Power%20delivery%20on%20Raspberry%20Pi%205.pdf)
- Known loads: AWUS036ACM about 380 mA under heavy load; RTL-SDR bias tee can source up to 180 mA to an LNA on top of the dongle's own draw; Wideband LNA V2 takes 90 mA; KrakenSDR wants a 2.4 A+ supply of its own — [morrownr USB-WiFi list](https://github.com/morrownr/USB-WiFi/blob/main/home/USB_WiFi_Adapters_that_are_supported_with_Linux_in-kernel_drivers.md); [V4L Users Guide](https://www.rtl-sdr.com/V4L); [RTL-SDR Blog store](https://www.rtl-sdr.com/buy-rtl-sdr-dvb-t-dongles/); [Crowd Supply KrakenSDR](https://www.crowdsupply.com/krakenrf/krakensdr)
- High-power Wi-Fi adapters can brown out a Pi or an unpowered hub and should be on a powered port — [computingforgeeks, 16 Oct 2026](https://computingforgeeks.com/best-wifi-adapter-kali-linux/)
- HackRF Pro is a USB 2.0 High-Speed device streaming up to 20 Msps of 8-bit I/Q — [SparkFun HackRF Pro](https://www.sparkfun.com/hackrf-pro.html)
- Sniffle dongles need a 2,000,000-baud (or 921,600) UART link; serial devices should be addressed by `/dev/serial/by-id/` for stable naming — [nccgroup/Sniffle](https://github.com/nccgroup/Sniffle); [Kismet Sniffle BLE](https://kismetwireless.net/docs/readme/datasources/bluetooth-sniffle-ble/)

### Inferences
- Tier-A load estimate: one Wi-Fi adapter (about 0.4 A) plus two RTL-SDRs plus an LNA plus three low-power serial dongles is already past 600 mA and plausibly near or over 1.6 A, so a powered USB hub fed from the pack's 5 V rail is the safe default on both Pi 5 and CM4 carriers.
- USB data bandwidth is not the limit for this BOM: RTL-SDRs, serial sniffers and the tinySA are all low-rate devices; only a HackRF-class SDR needs its own port.
- CPU, not USB, will bound concurrency: Kismet with several sources plus rtl_433 plus readsb is realistic on a Pi 5/CM4; wideband FFT sweeps should be scheduled, not continuous.
- USB 3 ports and cables radiate broadband noise around 2.4 GHz; keep 2.4 GHz sniffers on short extension leads away from USB 3 devices and the Pi itself (unverified background, but a common cause of deaf BLE/Zigbee dongles).

### Gaps
- CM4 USB current limits depend on the carrier board; not researched.
- Thermal behavior of RTL-SDRs, mt7921au adapters and the Pi in Arizona summer conditions (enclosure temperatures, L-band lock loss on hot RTL-SDRs, throttling): no source fetched. Recommend the pack log dongle-adjacent temperature and Pi `vcgencmd get_throttled` state as part of health checks (unverified background).
- Measured current draw for RTL-SDR V3/V4L, nRF52840, Sonoff dongle, ESP32-C5 and tinySA charging: not verified.

## Key Question 9: US legal boundaries (short section; not legal advice)

### Takeaway
Passive reception of unencrypted, publicly accessible radio signals and RF energy measurement is broadly lawful; intercepting encrypted or carrier (cellular) communications, or disclosing/using others' private traffic, is not; any jamming is flatly illegal, and transmitting outside certified/licensed operation is an FCC violation.

### Cited Findings
- Federal wiretap law makes intentional interception of wire, oral or electronic communications a crime (fine and up to five years), subject to exceptions — [18 U.S.C. 2511](https://www.law.cornell.edu/uscode/text/18/2511)
- Exceptions in 2511(2)(g) cover: electronic communications on systems configured to be readily accessible to the general public; broadcasts for the public and distress traffic; government, public-safety and private land-mobile systems that are readily accessible to the public; amateur, CB and GMRS; marine and aeronautical systems; and interception of a signal causing harmful interference, to the extent needed to identify its source — [18 U.S.C. 2511(2)(g)](https://www.law.cornell.edu/uscode/text/18/2511)
- A radio communication is not "readily accessible to the general public" when it is scrambled or encrypted, uses modulation whose essential parameters are withheld for privacy, rides a subcarrier, or is carried by a common carrier (other than tone-only paging), among other categories — [18 U.S.C. 2510(16)](https://www.law.cornell.edu/uscode/text/18/2510)
- The Communications Act separately bars a person not authorized by the sender from intercepting a radio communication and divulging it, or from using a communication they are not entitled to for their own or another's benefit; exempt are transmissions for the general public, distress traffic, and amateur/CB. Willful violations carry up to $2,000 and six months; commercial-gain violations far more — [47 U.S.C. 605](https://www.law.cornell.edu/uscode/text/47/605)
- Part 15 devices may not be used to overhear or record others' private conversations unless all parties consent — [47 CFR 15.9](https://www.law.cornell.edu/cfr/text/47/15.9)
- Jammers: operating, marketing, selling or importing them is illegal (47 U.S.C. 301, 302a(b), 333; 18 U.S.C. 1362, 1367), with fines, seizure and possible imprisonment; the FCC states there is no exemption for use inside a business, classroom, residence or vehicle. GPS blockers are expressly included — [FCC jammer enforcement](https://www.fcc.gov/general/jammer-enforcement)
- Operating a radio transmitter requires a license or other authorization (47 U.S.C. 301) — [FCC jammer enforcement](https://www.fcc.gov/general/jammer-enforcement)
- A 2026 buyer's guide summarizes the practical line the same way: receiving is generally lawful; transmitting on licensed or restricted bands needs authorization; using captured signals to access systems without permission risks CFAA liability — [HackRF One/PortaPack guide 2026](https://pasqualepillitteri.it/en/news/9488/hackrf-one-portapack-guide-2026)

### Inferences
- Inside the lines for this pack: spectrum sweeps and noise measurements; ADS-B (aeronautical); Remote ID (a mandated public broadcast); rtl_433 decoding of unencrypted weather/TPMS/sensor beacons; Wi-Fi/BLE/802.15.4 metadata surveys (beacons, advertisements, channel use, RSSI) especially of the owner's own networks; LoRa/Meshtastic traffic on the owner's own mesh; GNSS reception.
- Outside the lines: decrypting or cracking other people's Wi-Fi/Zigbee/BLE traffic; capturing and keeping payload from networks the owner does not control; any cellular interception (carrier traffic is excluded from "readily accessible" by definition, which is the reason gr-gsm-style decoding of live networks should be left out of the tool set); replaying captured sub-GHz codes at devices the owner does not own; any deauth/jamming "tests".
- Design consequence: the MCP server should expose survey/metadata tools by default, gate payload capture behind an explicit "own network only" allow-list (BSSID/PAN ID/mesh key supplied by the owner), and expose no transmit primitives. The only intentional emitters should be FCC-certified radios doing normal things (the Pi's own Wi-Fi/BLE, the owner's drone) and the tinySA generator into a cable/attenuator, never an antenna.

### Gaps
- Case law on capturing unencrypted Wi-Fi payload was not fetched. Unverified background: the Ninth Circuit (which includes Arizona) held in Joffe v. Google (2013) that Wi-Fi payload data is not exempt "radio communication" under the Wiretap Act, which supports the conservative "metadata only on networks you do not own" rule above.
- Arizona state interception statutes were not researched.
- FCC Part 15 limits on unintentional/incidental radiators (relevant when the owner's own inverter is the noise source) were not fetched.

## Key Question 10: Recommended RF bill of materials at two tiers, with data paths and self-tests

### Takeaway
About $150 buys a complete packet-and-sensor survey array (two RTL-SDR V3s, tri-band mt7921au Wi-Fi, nRF52840, ESP32-C5, FM notch); about $300-350 adds a tinySA Ultra+ as the calibrated-enough wideband eye and cabled reference source, plus an LNA and near-field probes. Everything listed is receive-first and runs on ARM64 Linux with in-kernel or open drivers.

### Cited Findings
Tier A, about $148-178 (prices as cited in earlier sections):
- RTL-SDR Blog V3 with dipole kit, $44.95-49.95: 500 kHz-28.8 MHz direct sampling plus VHF/UHF; software rtl_433 (JSON/MQTT/HTTP), readsb (aircraft.json), Kismet rtl_433/ADS-B sources; ARM64: standard librtlsdr, Pi images documented — [RTL-SDR Blog store](https://www.rtl-sdr.com/buy-rtl-sdr-dvb-t-dongles/); [merbanan/rtl_433](https://github.com/merbanan/rtl_433); [wiedehopf/readsb](https://github.com/wiedehopf/readsb)
- Second RTL-SDR Blog V3, dongle only, $34.95-39.95: lets rtl_433 stay parked on 433/915 MHz while the other does ADS-B or sweeps — [RTL-SDR Blog store](https://www.rtl-sdr.com/buy-rtl-sdr-dvb-t-dongles/)
- Broadcast FM band-stop filter, $16.95 — [RTL-SDR Blog store](https://www.rtl-sdr.com/buy-rtl-sdr-dvb-t-dongles/)
- mt7921au tri-band adapter (EDUP EP-AX1672 about $25-30, Panda PAU0F about $30, or Brostrend AX9L about $34-35): Wi-Fi 2.4/5/6 GHz monitor mode; in-kernel `mt7921u` since 5.18; data via Kismet REST — [morrownr USB-WiFi list](https://github.com/morrownr/USB-WiFi/blob/main/home/USB_WiFi_Adapters_that_are_supported_with_Linux_in-kernel_drivers.md)
- nRF52840 Dongle, $11.02: 802.15.4/Zigbee/Thread sniffing through Kismet's nRF52840 source or Nordic's Wireshark extcap — [DigiKey](https://www.digikey.com/en/product-highlight/n/nordic-semi/nrf52840-usb-dongle); [Kismet datasources](https://kismetwireless.net/docs/readme/datasources/datasources/); [Nordic sniffer](https://github.com/NordicSemiconductor/nRF-Sniffer-for-802.15.4)
- ESP32-C5 board, about $15 (DevKitC-1, AliExpress) or $24.95 (SparkFun Thing Plus, US stock): scriptable 2.4/5 GHz Wi-Fi 6 + BLE 5 + 802.15.4 node for custom firmware (e.g. Remote ID listener emitting JSON over USB serial) — [CNX Software](https://www.cnx-software.com/2025/04/30/esp32-c5-mass-production-esp32-c5-devkitc-1-board/); [SparkFun](https://www.sparkfun.com/sparkfun-thing-plus-esp32-c5.html)
- Bluetooth/BLE at no cost through the Pi's own controller as a Kismet "Linux HCI" source — [Kismet datasources](https://kismetwireless.net/docs/readme/datasources/datasources/)
- Arithmetic: low end 44.95 + 34.95 + 16.95 + 25 + 11.02 + 15 = $147.87; high end 49.95 + 39.95 + 16.95 + 35 + 11.02 + 24.95 = $177.82 (sum of the cited prices; excludes hub, cables, GNSS and the Sniffle dongle, whose prices were not verified).

Tier B, about $306-346 (Tier A plus):
- tinySA Ultra+ ZS406, about $138 (to 5.4 GHz), or ZS407, about $178 (to 7.3 GHz): wideband sweeps 100 kHz up, built-in LNA, harmonic-mode peek at 10-12 GHz, signal generator; agent data path is the USB serial console (`scan`, `hop`, `marker`) — [CNX Software, 15 Dec 2025](https://www.cnx-software.com/2025/12/15/tinysa-is-a-low-cost-handheld-spectrum-analyzer-with-built-in-signal-generator/); [tinySA wiki](https://tinysa.org/wiki/pmwiki.php?n=Main.USBInterface)
- RTL-SDR Blog Wideband LNA V2, $19.95 — [RTL-SDR Blog store](https://www.rtl-sdr.com/buy-rtl-sdr-dvb-t-dongles/)
- Near-field probe set: generic kit (about $5 on sale in the cited test) or RF Explorer kit ($89) — [Hackaday](https://hackaday.com/2023/01/03/near-field-emi-probes-any-good); [EEVblog](https://eevblog.com/forum/testgear/new-affordable-emc-probe-set-at-a-bargain-price-from-ariel-rocholl_s-lab)
- Optional: YouLoop $29.97 for HF noise hunting; ALFA AWUS036AXML ($70-77) instead of the budget mt7921au stick; Sonoff CC2652P dongle with Sniffle for BLE 5 coded PHY/extended advertising and Remote ID over BLE — [RTL-SDR Blog store](https://www.rtl-sdr.com/buy-rtl-sdr-dvb-t-dongles/); [morrownr USB-WiFi list](https://github.com/morrownr/USB-WiFi/blob/main/home/USB_WiFi_Adapters_that_are_supported_with_Linux_in-kernel_drivers.md); [Kismet Sniffle BLE](https://kismetwireless.net/docs/readme/datasources/bluetooth-sniffle-ble/)
- Arithmetic: Tier A + ZS406 + LNA = 147.87 + 138 + 19.95 = $305.82 (low) to 177.82 + 138 + 19.95 = $335.77 (high). With the ZS407 instead: $345.82 low; dropping the second RTL-SDR brings the ZS407 build to $310.87. Probe set and YouLoop are extra.

Self-test and health-check hooks that are documented:
- RTL-SDR: enumerate and read serials with `rtl_test`; set unique serials with `rtl_eeprom`; toggle bias tee with `rtl_biast`; rtl_433 `-M noise` (noise floor every 10 s) and `-M stats` (decoder counts) as liveness and sensitivity trend — [Kismet rtl_433 source](https://kismetwireless.net/docs/readme/datasources/sensor-sdrrtl433); [V4L Users Guide](https://www.rtl-sdr.com/V4L); [merbanan/rtl_433](https://github.com/merbanan/rtl_433)
- ADS-B chain: readsb `--stats-every` and `--write-prom` output — [wiedehopf/readsb](https://github.com/wiedehopf/readsb)
- All Kismet-owned radios: poll `/datasource/all_sources.json` for running state and packet counts; recover with `open_source.cmd` / `resume_source.cmd` — [Kismet datasources API](https://kismetwireless.net/docs/api/datasources)
- tinySA: `selftest 0 0..9`, `vbat`, `version`, `info`; detect on both USB ID pairs — [tinySA wiki](https://tinysa.org/wiki/pmwiki.php?n=Main.USBInterface)
- Sniffle dongle: confirm correct baud/firmware variant; fixed advertising channel is expected behaviour, not a fault — [Kismet Sniffle BLE](https://kismetwireless.net/docs/readme/datasources/bluetooth-sniffle-ble/)
- Pi power: confirm `usb_max_current_enable` / 5 A negotiation state at boot — [Raspberry Pi white paper](https://pip-assets.raspberrypi.com/categories/685-app-notes-guides-whitepapers/documents/RP-009856-WP-1-USB%20Power%20delivery%20on%20Raspberry%20Pi%205.pdf)

### Inferences
- Coverage map for Tier A: Wi-Fi 2.4/5/6 GHz (mt7921au via Kismet); BLE (Pi HCI; add Sniffle for BLE 5 long range); Zigbee/Thread (nRF52840); 315/433/915 MHz sensors and TPMS (RTL-SDR + rtl_433); ADS-B (RTL-SDR + readsb); Remote ID (Wi-Fi beacon via monitor adapter/Kismet, BLE via Sniffle or ESP32-C5); HF-to-1.7 GHz narrowband spectrum (RTL-SDR). Not covered in Tier A: calibrated wideband spectrum above about 1.7 GHz, LoRa packet decode, GNSS health.
- Tier B closes the wideband gap (tinySA) and adds the cabled-reference self-test: tinySA generator at a known level into the SP4T/attenuator and then each receiver, so the agent can compare measured level against expected and flag a deaf front end, a bad cable or a failed LNA. This is conducted, not radiated, so it stays inside the receive-only policy.
- "Known ambient emitter" checks need no extra hardware: the owner's own AP/eero beacons (Wi-Fi), the Pi's own BLE advertisements (BLE chain), the HA Yellow's Zigbee/Thread traffic (802.15.4), overflying aircraft (1090 MHz message rate), and a known TPMS or weather sensor (rtl_433). The agent should keep a per-site baseline and alert on deviation rather than on absolute thresholds.
- Skip at this budget: HackRF One/Pro, PortaPack, KrakenSDR, Airspy R2. Revisit a HackRF Pro only if raw IQ above 1.7 GHz becomes necessary (for example DJI DroneID decoding), and then with hardware TX disable.
- LoRa/Meshtastic: the lowest-effort sensor is a Meshtastic node on the owner's mesh read through the Python API (packet events), accepting that it only sees packets it can decode with the channel keys it holds; a CatSniffer or Kismet LoraPipe source is the sniffer-grade option.

### Gaps
- Unpriced but needed: powered USB hub, SMA cables/attenuators, USB GNSS receiver, Sonoff ZBDongle-P, RF switch board, Meshtastic node. Budget roughly another $50-80 for these (estimate, not sourced).
- tinySA prices are Dec 2025 AliExpress figures before tax/shipping/tariffs; Amazon US Nov 2026 prices were not verified and may be higher.
- No end-to-end report was found of this exact combination (Kismet + rtl_433 + readsb + tinySA) running concurrently on a Pi 5/CM4.
- GNSS: no receiver price fetched. Verified only at a high level: low-cost monitors built on u-blox ZED-F9P/F9T with a Raspberry Pi use AGC, C/N0, spectral power and the receiver's own interference indicators, and the authors found that combining power metrics with cross-constellation position checks was necessary to catch events — [GPS World](https://www.gpsworld.com/innovation-monitoring-gnss-interference-and-spoofing-a-low-cost-approach/); u-blox's newest ZED-X20P exposes jamming-state and spoofing-state outputs and prefers reporting no fix over a deceptive one (vendor-reported, development firmware) — [u-blox Jammertest 2025 blog](https://www.u-blox.com/en/blogs/tech/gnss-jamming-spoofing-detection-jammertest-2025-andoya). Which cheap USB modules (M8/M10 class) expose equivalent flags was not verified.

## Key Question 11: Ideas we may not have thought of

### Takeaway
The cheapest gains are procedural rather than hardware: a terminated reference port, A/B differencing under agent control, baseline-and-drift tracking, and reusing equipment the owner already has as known emitters and extra sensors.

### Cited Findings
- rtl_433 can emit a periodic noise-floor estimate and per-decode RSSI/SNR, which makes long-term "is 433/915 getting noisier when the charger runs" trending nearly free — [merbanan/rtl_433](https://github.com/merbanan/rtl_433)
- tinySA Ultra's harmonic mode reaches about 10-12 GHz, albeit uncalibrated and with possible ghost responses — [CNX Software, 15 Dec 2025](https://www.cnx-software.com/2025/12/15/tinysa-is-a-low-cost-handheld-spectrum-analyzer-with-built-in-signal-generator/)
- Kismet already ingests non-RF sensors and odd radios through the same API: MQTT sources, radiation detectors (Radiacode, Bluetooth Geiger), Wi-Fi HaLow (MM8108), AIS, cellular modem sources, and tracker/battery/distance views for Bluetooth devices — [Kismet datasources](https://kismetwireless.net/docs/readme/datasources/datasources/)
- Kismet can replay kismetdb and pcap files as sources, so recorded field sessions can be re-analysed offline by the agent — [Kismet datasources](https://kismetwireless.net/docs/readme/datasources/datasources/)
- ESP32-based Remote ID listeners can relay detections over a Meshtastic LoRa mesh, giving property-wide coverage from $15-class nodes — [CNX Software, 5 Jun 2025](https://www.cnx-software.com/2025/06/05/map-remote-id-enabled-drones-with-esp32-c3-s3-and-meshtastic-lora-modules/)
- The YouLoop is a passive 10 kHz-30 MHz loop for $29.97 — [RTL-SDR Blog store](https://www.rtl-sdr.com/buy-rtl-sdr-dvb-t-dongles/)
- Low-cost GNSS interference monitoring works best by fusing several indicators (AGC, C/N0, spectrum, cross-constellation position consistency) rather than one flag — [GPS World](https://www.gpsworld.com/innovation-monitoring-gnss-interference-and-spoofing-a-low-cost-approach/)

### Inferences
- 50-ohm terminator on one RF-switch port: lets the agent subtract the receiver's own spurs and verify gain settings before blaming the environment.
- Agent-directed A/B tests: the agent asks the human (or a smart relay/Home Assistant) to switch the inverter, charger, Starlink or a single eero node off and on while it records sweeps, then reports the spectral difference. This turns a cheap receiver into a diagnostic instrument and is the single most useful workflow for the stated inverter/charger problem.
- Use the owner's own Remote-ID-broadcasting drone (he owns a DJI Mavic 4 Pro, per stored memory) as the known emitter to validate the Remote ID receive chain and measure its range on the property.
- Read the devices' own radio diagnostics instead of sniffing them: router/mesh APIs for per-client RSSI and channel utilization, Home Assistant ZHA/Zigbee2MQTT link-quality and energy-scan data from the HA Yellow, inverter/charge-controller Modbus or BLE telemetry, and Starlink's local diagnostics. These are lawful, structured, and often more informative than off-air capture (existence and formats of these APIs not verified this session).
- The owner has two Home Assistant Yellows, one of them a spare (per stored memory). The spare could become the fixed sentinel or a dedicated 802.15.4 listener without touching the live Zigbee/Thread network (whether its Silicon Labs radio has usable sniffer firmware was not verified).
- YouLoop as a giant H-field probe for walking PV strings and battery cables at HF, feeding either the tinySA or an RTL-SDR V3 in direct sampling.
- DIY clamp-on RF current probe (split ferrite plus a few turns to SMA) for common-mode noise on DC cabling; pairs with the same sweep tooling.
- A second, fixed "sentinel" node (CM4 + one RTL-SDR + rtl_433 + noise logging, or an ESP32-C5) left at the property gives time-correlated context for the portable pack: was the noise there before the truck arrived?
- Time-synchronised logging of RF noise against power events (inverter load, MPPT state, generator start) is likely to explain more interference than any direction-finding hardware; make timestamps and GPS position mandatory fields in every MCP tool result.
- Per-site "RF fingerprint" baselines (channel occupancy, noise floor per band, list of expected emitters) stored by the agent, with drift alerts, also double as a radio health check: if every band drops at once, suspect the antenna or cable, not the environment.
- Lightning/static and arcing faults (loose PV connectors, failing MC4s) show up as broadband impulsive noise; a simple AM-detector-style impulse counter on HF via the RTL-SDR could flag DC arc faults early (speculative; no source).
- Passive Wi-Fi/BLE channel-utilization measurement on the mt7921au (survey dump / airtime counters) is more actionable for mesh-placement advice than packet capture, and carries no payload-privacy risk (driver support for these counters not verified).

### Gaps
- None of the "read the device's own diagnostics" interfaces (eero, Starlink, inverter brands) were verified this session.
- No source on arc-fault detection with cheap SDRs was fetched; treat as an experiment.
