# LoRa / Meshtastic backhaul for the portable sensor pack, alternatives at 150-250 m, and offline voice output (state: 25 Nov 2026)

Conventions used in these notes:
- "Computed" = my own arithmetic from the Semtech LoRa time-on-air formula (16-symbol preamble, explicit header, CRC on, low-data-rate-optimise only when symbol time > 16 ms), using the preset parameters published by Meshtastic. Not a measurement.
- "[BK]" = background knowledge that I could not confirm from a page fetched in this session. Treat as unverified until checked.
- Prices are USD, with source and date. Many store pages would not load; those prices are listed under Gaps.
- Not legal advice on FCC rules; the rule text is quoted, the application to specific Meshtastic presets is my inference.

---

## KQ1. Meshtastic in Nov 2026: firmware, boards, prices, presets, payload, airtime, regulation, modules, APIs

### Takeaway
Meshtastic is mature enough for this job: firmware 2.7.26 is the current stable/beta line (2.8.x is alpha), a packet carries at most roughly 230 bytes of application data, and in the US there is no legal duty-cycle cap, so the real limit is airtime per packet (computed: a 40-byte reading takes about 0.06 s on ShortFast and about 0.7 s on LongFast). A private channel with a private port number (256+) can carry arbitrary binary (CBOR/postcard) payloads, and there are first-party Python and Rust client libraries plus a Linux-native daemon.

### Cited Findings

Firmware line
- The Meshtastic firmware API lists v2.7.26.54e0d8d ("Beta") as the newest stable-channel release, with v2.7.15 before it; the newest alpha is v2.8.2.d5f1619 (v2.8.1 before it). No dates on that page. — [api.meshtastic.org firmware list](https://api.meshtastic.org/github/firmware/list)
- v2.7.26 is a bug-fix/maintenance release (LoRa RX LED fix, RAK6421 and Station G3 meshtasticd configs, T-Deck touch fix; no new features); the tracker page showed it as "5 months ago" at capture time. — [newreleases.io v2.7.26](https://newreleases.io/project/github/meshtastic/firmware/release/v2.7.26.54e0d8d)
- Next-hop routing for direct messages exists since v2.6; adaptive hop limits (`HopScalingModule`) appear only on the develop/2.8 branch. — [Meshtastic mesh algorithm docs](https://meshtastic.org/docs/overview/mesh-algo/); [irab Meshtastic vs MeshCore technical page](https://irab.github.io/mesh/technical.html)

Presets (bandwidth / spreading factor / coding rate / raw data rate / link budget at 22 dBm, 0 dB antenna)
- Short Turbo 500 kHz / SF7 / 4/5 / 21.88 kbps / 140 dB; Short Fast 250 kHz / SF7 / 4/5 / 10.94 kbps / 143 dB; Short Slow 250 / SF8 / 4/5 / 6.25 kbps / 145.5 dB; Medium Turbo 500 / SF9 / 4/5 / 7.03 kbps / 145 dB; Medium Fast 250 / SF9 / 4/5 / 3.52 kbps / 148 dB; Medium Slow 250 / SF10 / 4/5 / 1.95 kbps / 150.5 dB; Long Turbo 500 / SF11 / 4/8 / 1.34 kbps / 150 dB; Long Fast 250 / SF11 / 4/5 / 1.07 kbps / 153 dB; Long Moderate 125 / SF11 / 4/8 / 0.34 kbps / 156 dB. — [Meshtastic radio settings](https://meshtastic.org/docs/overview/radio-settings/)
- The same page now also lists region-limited narrowband presets (Lite Fast/Slow at 125 kHz, Narrow Fast/Slow at 62.5 kHz, Tiny Fast/Slow at 15.6 kHz). — [Meshtastic radio settings](https://meshtastic.org/docs/overview/radio-settings/)
- US region: 902-928 MHz, max +30 dBm; the page states a duty cycle (10%) only for EU 868, none for the US. — [Meshtastic radio settings](https://meshtastic.org/docs/overview/radio-settings/)
- Meshtastic's own guidance: move off LongFast above about 60 nodes; dense meshes should use MediumFast/MediumSlow or ShortFast/ShortSlow; all nodes must share the preset. Wellington's LongFast mesh peaked above 65% channel utilisation. — [Meshtastic blog: why your mesh should switch from LongFast](https://meshtastic.org/blog/why-your-mesh-should-switch-from-longfast/)

Packet size and reliability
- Header is 16 raw bytes; payload is "up to 239 bytes of actual packet data", which includes 12 bytes when PKC/AEAD encryption is used; oversize packets are not sent. — [Meshtastic mesh algorithm docs](https://meshtastic.org/docs/overview/mesh-algo/)
- Other first-party numbers differ slightly: Serial module SIMPLE mode "up to 237 bytes per packet" ([Serial module docs](https://meshtastic.org/docs/configuration/module/serial/)); `SERIAL_APP` comment "Maximum packet size of 240 bytes" ([portnums.proto](https://github.com/meshtastic/protobufs/blob/master/meshtastic/portnums.proto)); the Python API raises "Data payload too big" when `len(data) > Constants.DATA_PAYLOAD_LEN` ([Meshtastic Python API](https://python.meshtastic.org/mesh_interface.html)). A third-party project built on Meshtastic uses 231 B as its MTU and 215 B body. — [voicetastic Constants and Limits](https://github.com/voicetastic/voicetastic-core/wiki/Constants-and-Limits)
- ACK/retry: `want_ack` flag makes a packet reliable; direct messages get a real ACK from the recipient; a reliable broadcast is sent at most 3 times in total and a reliable direct message at most 5 times, after which the local node generates a NAK. — [Meshtastic mesh algorithm docs](https://meshtastic.org/docs/overview/mesh-algo/). A code-reading comparison gives intermediate retries 2 (v2.6.4, v2.7.26) or 3 (develop) and original-sender retries 3 — [irab technical page](https://irab.github.io/mesh/technical.html) (the two sources do not agree exactly).
- Hop limit: default 3, maximum 7. — [irab technical page](https://irab.github.io/mesh/technical.html); [Seeed MeshCore vs Meshtastic](https://www.seeedstudio.com/blog/2026/03/23/meshcore-vs-meshtastic/)
- Channel access is CSMA/CA with channel-activity detection and a contention window that grows with channel utilisation; packets carry a 32-bit sender-assigned packet id. — [Meshtastic mesh algorithm docs](https://meshtastic.org/docs/overview/mesh-algo/)
- TX priorities in firmware: ACK 120, alerts 110, reliable DMs 70, default 64, background telemetry/position 10; LongFast airtime is about 560 ms per typical packet. — [irab technical page](https://irab.github.io/mesh/technical.html)

Port numbers and custom binary data
- Ranges: 0-63 core, 64-127 registered third-party, 256-511 "for your private applications that you don't want to register publically"; `PRIVATE_APP = 256` ("Private applications should use portnums >= 256"); max portnum 511. — [portnums.proto](https://github.com/meshtastic/protobufs/blob/master/meshtastic/portnums.proto)
- Relevant built-in ports: TEXT_MESSAGE_APP 1, REMOTE_HARDWARE_APP 2, DETECTION_SENSOR_APP 10, ALERT_APP 11, REPLY_APP 32 (ping), IP_TUNNEL_APP 33 (IP packets, handled by the Python API), SERIAL_APP 64, STORE_FORWARD_APP 65, RANGE_TEST_APP 66, TELEMETRY_APP 67, RETICULUM_TUNNEL_APP 76 (fragmented RNS packets), CAYENNE_APP 77, LORA_OTA_APP 79 ("signed firmware updates over lora"), PAGING_APP 38 (acknowledged paging), MESH_BEACON_APP 37. — [portnums.proto](https://github.com/meshtastic/protobufs/blob/master/meshtastic/portnums.proto)
- Python API: `sendData(data, destinationId, portNum=PRIVATE_APP, wantAck=False, wantResponse=False, onResponse=None, channelIndex=0, hopLimit=None, pkiEncrypted=False, ...)`; received packets are published on pubsub topic `meshtastic.receive.data.{portnum}`. — [Meshtastic Python API](https://python.meshtastic.org/mesh_interface.html)

Modules
- Telemetry: device, environment, air-quality, health and power metrics; default interval 1800 s; power metrics read INA219, INA226, INA260 (addresses 0x40/0x41/0x43) and INA3221 (0x42, 3 channels) current/voltage sensors over I2C. — [Telemetry module docs](https://meshtastic.org/docs/configuration/module/telemetry/)
- Serial: modes SIMPLE (raw bytes to a channel named "serial", up to 237 B), PROTO (full protobuf client API on a UART), TEXTMSG, NMEA, CALTOPO, WS85, VE_DIRECT (Victron), MS_CONFIG, LOG, LOGTEXT; baud 110-921600, default 38400. — [Serial module docs](https://meshtastic.org/docs/configuration/module/serial/)
- Store & Forward: server must be an ESP32 with onboard PSRAM (T-Beam, T3S3 named); text messages only are discussed; about 11,000 records; default return max 25 messages, window 240 min; heartbeat every 15 min. — [Store & Forward docs](https://meshtastic.org/docs/configuration/module/store-and-forward-module/)
- Range Test (interval location messages logged to CSV; needs GPS), Remote Hardware (read/set GPIO over the mesh), Detection Sensor (GPIO state to text alert), Canned Message, External Notification, Neighbor Info, Traceroute. — [Module configuration index](https://meshtastic.org/docs/configuration/module/)
- MQTT: needs a node with Wi-Fi/Ethernet (or "proxy to client"); per-channel uplink/downlink flags; JSON mode sends/consumes unencrypted JSON and is not supported on nRF52; "encryption enabled" off sends plaintext to the broker. — [MQTT module docs](https://meshtastic.org/docs/configuration/module/mqtt/)

Client libraries and Linux-native daemon
- Rust: first-party `meshtastic` crate 0.1.9 (released 2026-05-25; 0.1.8 on 2025-11-23; 0.1.0 in Sept 2023), GPL-3.0, tokio-based, desktop-oriented; connects over USB serial and TCP, BLE behind the `bluetooth-le` feature. — [docs.rs meshtastic](https://docs.rs/crate/meshtastic/latest); [github.com/meshtastic/rust](https://github.com/meshtastic/rust)
- `meshtasticd` (Linux native): only SPI radios work; UART HATs and SX1302/SX1303 concentrator HATs are not supported. Tested HATs: MeshAdv-Pi v1.1, Adafruit RFM9x, Elecrow RFM95; RAK6421 is given as an example. The Waveshare SX1262 LoRaWAN HAT is "not recommended for deployment" (hardware limitation affecting longer messages; use CLIENT_MUTE if you must). USB radios: Elecrow MeshStick and Pinedio CH341 adapter (CH341 support since 2.5.18). — [meshtasticd hardware docs](https://meshtastic.org/docs/software/linux/hardware/)

Boards and prices found this session
- Heltec WiFi LoRa 32 V4.3 R8 (ESP32-S3R8, 8 MB PSRAM, 16 MB flash, SX1262, 28 dBm on 863-928 MHz, USB-C, solar and GNSS headers, antenna included): $21.90-27.50 depending on options; V4.3 R2 variant $17.90; L76K GNSS add-on $8.00. — [heltec.org WiFi LoRa 32 V4](https://heltec.org/project/wifi-lora-32-v4/) (fetched 25 Nov 2026)
- Amazon US price bands (article dated 29 Oct 2026): Heltec V4 $25-40; Heltec Mesh Node T114 (nRF52840 + SX1262, BLE only) $35-45, GNSS version $40-50; LilyGo T-Beam Supreme (ESP32-S3 + SX1262 + L76K) $50-70; LilyGo T-Deck Plus $75-100; RAK WisBlock Mini Starter Kit (RAK4631 nRF52840 + SX1262) $28-40; SenseCAP T1000-E (nRF52840 + LR1110) $45-60; SenseCAP Solar Node P1-Pro $130-160. The same article advises buying the V4 over the V3 (V3 tops out at 21 dBm; USB bridge quirks). — [computingforgeeks best Meshtastic device](https://computingforgeeks.com/best-meshtastic-device/)
- Seeed XIAO ESP32S3 + Wio-SX1262 kit: $9.90 with a discount code (article Jan-Mar 2025; includes both antennas, no USB cable); the author measured about 500 m urban range versus about 2 km for a T1000-E and calls it best for close-range/dev use; Heltec V3 was about $21 at that time. — [adrelien XIAO kit review](https://adrelien.com/xiao-esp32s3-wio-sx1262-kit-the-cheapest-meshtastic-node/)
- Elecrow MeshStick (CH341A USB-to-SPI + SX1262 with TCXO, +22 dBm, SMA antenna included, EEPROM serial; for `meshtasticd` on Linux incl. Raspberry Pi): $32.97. — [Rokland MeshStick listing](https://store.rokland.com/collections/new-arrivals-1/products/elecrow-meshstick-usb-to-spi-sx1262-tcxo-lora-usb-stick-meshtastic-lora-mesh-node) (fetched 25 Nov 2026)
- Waveshare "SX1262 LoRa HAT" 915M: $25.99, but it is the UART (9600 bps default) transparent-modem HAT. — [Waveshare SX1262 LoRa HAT](https://www.waveshare.com/sx1262-868m-lora-hat.htm?sku=16807) — which is the class `meshtasticd` does not support ([meshtasticd hardware docs](https://meshtastic.org/docs/software/linux/hardware/)).

US regulation (47 CFR 15.247, 902-928 MHz)
- Frequency-hopping systems: at least 50 hopping channels if 20 dB bandwidth < 250 kHz (25 if >= 250 kHz); average occupancy on any frequency <= 0.4 s per 20 s (10 s for the wider class); max 20 dB channel bandwidth 500 kHz; 1 W with >= 50 channels, 0.25 W with 25-49. — [47 CFR 15.247 (Cornell LII)](https://www.law.cornell.edu/cfr/text/47/15.247)
- Digital-modulation systems: 6 dB bandwidth at least 500 kHz; peak conducted power 1 W; power spectral density <= 8 dBm in any 3 kHz. Antenna gain above 6 dBi requires an equal power reduction. Hybrid systems must meet 0.4 s occupancy with DTS off and the PSD limit with hopping off. No duty-cycle percentage appears in the section. — [47 CFR 15.247 (Cornell LII)](https://www.law.cornell.edu/cfr/text/47/15.247)

Computed airtime per Meshtastic packet (PHY bytes = 16-byte header + Data protobuf with about 6 bytes of overhead around the app payload)

| Preset | ACK-size (24 B) | 40 B app payload (62 B) | 100 B app (122 B) | 200 B app (222 B) | 255 B max |
|---|---|---|---|---|---|
| ShortTurbo | 17 ms | 32 ms | 53 ms | 89 ms | 102 ms |
| ShortFast | 35 ms | 63 ms | 107 ms | 178 ms | 204 ms |
| MediumFast | 119 ms | 201 ms | 345 ms | 570 ms | 642 ms |
| LongFast | 436 ms | 723 ms | 1,174 ms | 1,911 ms | 2,157 ms |
| LongModerate | 1,249 ms | 2,298 ms | 4,133 ms | 7,016 ms | 7,934 ms |

(Computed; consistent with the "about 560 ms per LongFast packet" figure in the [irab technical page](https://irab.github.io/mesh/technical.html) and with third-party pacing of 150 ms ShortTurbo / 250 ms ShortFast / 500 ms MediumFast / 900 ms LongFast per chunk in [voicetastic](https://github.com/voicetastic/voicetastic-core/wiki/Constants-and-Limits).)

### Inferences
- Best-case application throughput (computed, 100% airtime, no ACKs, 200 B payloads): LongFast about 0.84 kbps, MediumFast about 2.8 kbps, ShortFast about 9 kbps, ShortTurbo about 18 kbps. With an ACK per packet and CSMA gaps, expect roughly half of that.
- Realistic messages per minute if the pack self-limits to 10% airtime (a courtesy figure, not a US legal one): 40 B readings: about 8/min on LongFast, 30/min on MediumFast, 95/min on ShortFast, 185/min on ShortTurbo. 200 B messages: about 3/min LongFast, 34/min ShortFast, 67/min ShortTurbo.
- At 150-250 m the pack does not need LongFast at all. Free-space loss at 915 MHz over 250 m is about 80 dB (computed); ShortTurbo's 140 dB link budget at 22 dBm leaves about 60 dB of margin, so ShortFast/ShortTurbo at reduced power is the right operating point, and LongFast only matters for the later 1-5 km case (free-space loss at 5 km about 106 dB; margin about 47 dB on LongFast before terrain and Fresnel losses).
- FCC reading (inference, not advice): only the 500 kHz presets (Short/Medium/Long Turbo) meet the 500 kHz minimum bandwidth of the digital-modulation clause on their face; a single-frequency 250 kHz LongFast packet of 0.7-2 s is longer than the 0.4 s hopping dwell figure. Certified modules are the vendor's responsibility, but for a two-node private link ShortTurbo is the cleanest choice on both regulatory and throughput grounds.
- Because Meshtastic allows one client connection per node, the process that owns the serial/TCP link to the node (HA integration or the project's own daemon) must be chosen deliberately (see KQ5).
- The $25.99 Waveshare UART HAT is a trap for this project: it is cheap and labelled SX1262, but cannot run `meshtasticd`.

### Gaps
- Exact value of `DATA_PAYLOAD_LEN` not shown on the fetched pages ([BK]: 233 in the 2.5+ protobufs). Design to <= 200 B per message and the discrepancy does not matter.
- Meshtastic's own airtime throttles ([BK]: telemetry/position suppressed above about 25% channel utilisation, polite limits at 40%, and a hard 10%/hour TX airtime rule only in duty-cycle regions such as EU 868) were not found on the pages fetched; verify in the firmware docs before relying on them.
- Any firmware-side rate limit on packets injected by a client over the serial API (queue depth, minimum spacing) was not found. Community throughput reports for PRIVATE_APP binary traffic: none found; voicetastic's pacing table is the closest evidence.
- Prices not verified this session: Heltec V3 (2026), LilyGo T-Beam (classic), T-Deck (non-Plus), T-Echo, RAK WisBlock 4631 starter kit direct, Station G2, MeshAdv-Pi, Waveshare SX1262 LoRaWAN (SPI) HAT, RAK6421 HAT. Rokland and Heltec catalogue pages did not load.
- No verified information on Meshtastic support for ESP32-C6 or nRF54-based boards, or on cheap LR1121 boards (only the LR1110-based T1000-E was confirmed). PKC direct messages "since 2.5", AES-256 channel keys, and channel encryption being unauthenticated AES-CTR are [BK]; the docs fetched only confirm that PKC/AEAD adds 12 bytes.
- SX1262 current draw ([BK]: about 118 mA at +22 dBm TX, about 5 mA RX, per Semtech datasheet) not fetched; Heltec's page gives only "<20 uA" sleep.

---

## KQ2. Can MCP or a compact command/telemetry protocol run over it? What fits, and prior art

### Takeaway
Do not run MCP itself over LoRa: JSON-RPC sessions, tool lists and schemas are kilobytes, and LoRa gives roughly 1-18 kbps at best. Instead terminate MCP at the bench and let a bench-side MCP server expose "remote pack" tools that are backed by a compact binary request/response protocol (readings under 40 bytes, commands under 40 bytes) on a private Meshtastic port. Prior art exists for each piece (MCP servers that drive a Meshtastic node, LLM bots over Meshtastic, MCP over MQTT, fragment+FEC transports over Meshtastic), but nothing found carries MCP natively over LoRa.

### Cited Findings
- `ConsentirDev/meshtastic.mcp`: a Python (3.10+) MCP server that lets Claude operate a Meshtastic node (tools: connect, send_message, get_messages, get_nodes, send_telemetry_request, traceroute, set_owner, reboot_node, ...), connecting by serial, TCP or Bluetooth; MIT; 6 stars, 2 commits, no releases. — [github.com/ConsentirDev/meshtastic.mcp](https://github.com/ConsentirDev/meshtastic.mcp)
- `mr-tbot/mesh-api` (v0.7.5.4 beta, 183 stars): routes Meshtastic and MeshCore messages to LLMs (12 providers incl. Claude, Ollama), APIs and Home Assistant's conversation API (optional `PIN=XXXX` prefix); "automatically splits long AI responses into configurable chunks with delays to reduce radio congestion". — [github.com/mr-tbot/mesh-api](https://github.com/mr-tbot/mesh-api)
- MCP over MQTT (EMQX): an MCP implementation on top of MQTT with broker-based service discovery, load balancing and central auth; SDKs listed for Erlang, ESP32 C, C (Paho), Python, TypeScript, Kotlin. Not presented as an official MCP transport. — [EMQX MCP over MQTT overview](https://docs.emqx.com/en/emqx/latest/emqx-ai/mcp-over-mqtt/overview.html)
- Fragment + forward-error-correction over Meshtastic already exists in a third-party project: 231 B MTU, 215 B body, 16 B header incl. 4 B integrity tag, up to 255 chunks (54,825 B per message), Reed-Solomon parity 10-100% chosen by mesh loss, NACK rounds, per-preset pacing. — [voicetastic Constants and Limits](https://github.com/voicetastic/voicetastic-core/wiki/Constants-and-Limits)
- Meshtastic itself now reserves ports for fragmented transports: `RETICULUM_TUNNEL_APP = 76` ("Fragmented RNS Packet"), `LORA_OTA_APP = 79` (signed firmware over LoRa), and `IP_TUNNEL_APP = 33` (raw IP packets via the Python API). — [portnums.proto](https://github.com/meshtastic/protobufs/blob/master/meshtastic/portnums.proto)
- Reticulum, by design, works down to 5 bit/s with a 500-byte MTU, sets up an encrypted link in 3 packets / 297 bytes, and offers request/response and "Resources" (auto-compressed, sequenced, integrity-checked, retransmitted transfers of arbitrary size) over a link. — [Reticulum manual: Understanding Reticulum](https://reticulum.network/manual/understanding.html)
- Home Assistant's MCP Server (since 2025.2) exposes tools, one prompt and one read-only resource at `/api/mcp` with OAuth 2.0 + PKCE; sampling and notifications are not supported. — [Home Assistant MCP Server integration](https://www.home-assistant.io/integrations/mcp_server/)

### Inferences
- Size check (computed): a reading message of about 35 B (see deliverable 3) rides in a 57-62 B PHY packet: about 32 ms ShortTurbo, 63 ms ShortFast, 0.2 s MediumFast, 0.7 s LongFast. A 200 B payload: 89 ms ShortTurbo vs 1.9 s LongFast.
- Request/response latency estimate (computed airtime plus an assumed 0.1-0.3 s of USB/serial, CSMA and processing overhead): ShortFast about 0.3-0.6 s round trip for a 30 B request and 40 B reply with ACKs; LongFast about 2-4 s. If a packet is lost, Meshtastic's retry timer adds seconds, so the bench tool should have a 10-15 s timeout on LongFast and about 3 s on Short presets.
- Why not MCP over LoRa: a single `tools/call` JSON-RPC request is typically 150-400 B (1-2 packets), which is tolerable, but `initialize` + `tools/list` with JSON Schemas for a pack with dozens of tools is easily 10-50 kB. At LongFast's best case of about 0.84 kbps that is 2-8 minutes of continuous airtime per session start; on ShortTurbo 5-25 s. Streaming, notifications and resources are worse. The fix is the SCHC idea (static context known to both ends): sync tool schemas and a phrase/unit dictionary over Wi-Fi, identify them by hash, and send only tool id + CBOR/postcard arguments over the radio.
- MCP-over-MQTT is a useful pattern at the bench (broker = Mosquitto on the Yellow) but does not shrink the messages, so it helps Tier 1 (IP), not the LoRa tier.
- [BK] RFC 9011 (SCHC over LoRaWAN) and the LoRaWAN FUOTA fragmented-data-block transport (TS004) are the standards-track prior art for "compress against a shared context" and "fragments with erasure coding"; not fetched this session.

### Gaps
- No project was found that carries MCP natively over LoRa/Meshtastic or a "compact MCP" binary encoding; the search surfaced an IETF draft for MCP over MoQT (media over QUIC) but it was not read.
- Seeed's "MeshClaw" (appeared in search results as a Meshtastic + agent project) was not read.
- No measured latency numbers for LLM-over-Meshtastic bots were found.

---

## KQ3. Alternatives and when each wins (150-250 m now, 1-5 km later)

### Takeaway
At 150-250 m line of sight, ordinary Wi-Fi with one outdoor directional access point at the bench end carries full IP (and therefore full MCP) at megabits per second and is the simplest high-rate path; ESP-NOW (especially its Long Range mode) is the cheapest option for the ESP32 scatter pods; HaLow is the right technology for 1 km-class IP but is still painful on Raspberry Pi OS in November 2026. Reticulum is the one stack that spans all of these media with the same addressing and built-in resumable transfers, and is the most interesting long-term fit; MeshCore offers nothing extra for a two-node private link.

### Cited Findings

Ordinary Wi-Fi with an outdoor CPE
- TP-Link CPE210: 2.4 GHz 2x2, 9 dBi directional antenna with 65 degree horizontal / 35 degree vertical beamwidth, 0-27 dBm adjustable TX power, modes AP / Client / AP Router / WISP, IPX5, passive 24 V PoE (cable up to 60 m), 10.5 W max, vendor range claim "5 km+". — [TP-Link CPE210](https://www.tp-link.com/us/business-networking/outdoor-radio/cpe210)
- Reference measurement with bare ESP32-C6 PCB antennas at both ends: Wi-Fi peaked at 12 Mbps at 30 m and was under 6 Mbps at 150 m in an open field. — [Espressif developer blog: ESP-NOW for outdoor applications](https://developer.espressif.com/blog/esp-now-for-outdoor-applications/) (20 Dec 2024; community-authored, not verified by Espressif)

ESP-NOW and ESP-NOW Long Range
- Same test (ESP32-C6-DevKitM-1, PCB antennas, open field): normal ESP-NOW near 100% delivery to 150 m and about 60% at 300 m, throughput 400 kbps close-in falling to 50 kbps at 300 m, latency < 20 ms; ESP-NOW-LR near 100% to 450 m and about 40% at 900 m, 100 kbps at 150 m falling to about 10 kbps at 900 m, latency < 25 ms. In forest, normal ESP-NOW fell below 50% delivery at 125 m and LR at 400 m. — [Espressif developer blog](https://developer.espressif.com/blog/esp-now-for-outdoor-applications/)

Wi-Fi HaLow (802.11ah)
- Morse Micro MM8108 USB adapters (MM8108-RD09 reference design, USB-A and USB-C, up to 43.3 Mbps PHY at 8 MHz, up to 26 dBm, SMA antenna): $58.00 at Mouser (article 14 Sep 2026; a reader reports a 140-unit minimum order); Linux needs the driver built against a patched kernel with S1G support, or Morse Micro's OpenWrt fork. — [CNX Software](https://www.cnx-software.com/2026/09/14/morse-micro-wi-fi-halow-usb-adapters-add-802-11ah-connectivity-to-pcs-and-routers/)
- Seeed Wio-WM6108 mini-PCIe module (Quectel FGH100M-H / MM6108, 902-928 MHz, up to 32.5 Mbps link rate, "up to 1 km"): $14.90 at launch (Mar 2025), antenna kit $4.39; needs an mPCIe HAT and a custom OpenWrt image on a Pi 4; XIAO HaLow add-on also $14.90 (ESP-IDF + Morse MM-IoT-SDK). — [CNX Software, 7 Mar 2025](https://www.cnx-software.com/2025/03/07/seeed-studio-wi-fi-halow-mini-pcie-module-for-raspberry-pi-add-on-board-for-xiao-boards/). A reseller listing title now marks the Wio-WM6108 as discontinued. — [The Pi Hut listing (title only)](https://thepihut.com/products/wio-wm6108-wi-fi-halow-mini-pcie-module)
- Raspberry Pi OS reality (community guide and replies, Dec 2025-Feb 2026): build the Pi kernel with Morse Micro's 6.12 patches (two conflict and need manual fixes), build the out-of-tree driver, install firmware, write a device-tree overlay for SPI modules (USB modules skip this); Trixie is harder; AP mode needs hostap_s1g or the OpenWrt fork; one user measured about 0.8 Mbps (1 MHz channel) to 8.3 Mbps (4 MHz) at about 20 feet between a Pi 5 and another node; kernel panics were reported with BATMAN_V. — [Morse Micro community build thread](https://community.morsemicro.com/t/build-thread-halow-for-raspberry-pi-os/1124)
- A mainline Linux driver (`mm81x`, under `drivers/net/wireless/morsemicro`) is now in the kernel wireless tree with commits dated Sep-Oct 2026 (monitor mode, fixes, "mm81x 7.4 PR #1" merge). — [kernel tree commit feed](https://git.zx2c4.com/wireguard-linux/atom/drivers/net/wireless/morsemicro?h=stable)
- Throughput vs range, vendor-run test (Morse Micro MM6108, Jan 2024): about 11 Mbps at 500 m and about 1 Mbps at 3 km. — [Tom's Hardware](https://www.tomshardware.com/networking/super-long-range-wi-fi-works-at-a-range-of-18-miles-halow-standard-aces-a-real-world-test-despite-high-interference)
- Andreas Spiess's 1 km comparison (as summarised by Hackaday): 5 GHz Wi-Fi with directional antennas had substantially higher bandwidth than HaLow; HaLow's clear advantage was penetration through masonry/floors. — [Hackaday, 26 May 2024](https://hackaday.com/2024/05/26/so-whats-all-this-halow-long-range-wifi-about-then)

Reticulum / RNode
- Requirements and features: half-duplex channel, >= 5 bit/s, 500-byte MTU; link setup 3 packets / 297 bytes; keep-alive about 0.45 bit/s; encryption always on (X25519, Ed25519, AES-256-CBC + HMAC-SHA256, forward secrecy on links); delivery proofs signed by the destination; Resources for large transfers. Runs over LoRa (RNode), serial, KISS packet modems, Ethernet/Wi-Fi and IP tunnels. — [Reticulum manual: Understanding](https://reticulum.network/manual/understanding.html)
- RNode firmware supports Heltec LoRa32 v3 and v4, Heltec T114, LilyGo T-Beam / T-Beam Supreme / T3S3 / T-Deck / T-Echo, RAK4631 and others (XIAO ESP32S3 not listed); the manual recommends Wi-Fi-class point-to-point radios (Ubiquiti airMAX, MikroTik) for backbone and LoRa RNodes for wide-area coverage, and `AutoInterface` works over any bridged-Ethernet Wi-Fi with no DHCP/DNS. — [Reticulum manual: Hardware](https://reticulum.network/manual/hardware.html)
- Rust: `reticulum-rs` 0.17.0 (2026-11-22; 28 releases since Feb 2026; FreeTAKTeam/LXMF-rs monorepo; EPL-2.0) with core, transport ("transport, interface, receipt, and resource layers") and RPC crates plus a Meshtastic tunnel interface; its own docs say it is not a complete drop-in replacement for Python Reticulum/LXMF and that Sideband/Columba compatibility needs separate interop evidence. A separate `reticulum` crate 0.1.0 also exists. — [docs.rs reticulum-rs](https://docs.rs/crate/reticulum-rs/0.9.6); [docs.rs reticulum](https://docs.rs/crate/reticulum/latest)

MeshCore
- MIT-licensed C++ multi-hop routing library and firmware; roles: Companion (BLE/USB/Wi-Fi to an app, does not repeat), Repeater, Room Server (BBS), Sensor (telemetry and alerting); client libraries meshcore.js and the Python meshcore-cli; 3.9k stars. — [github.com/meshcore-dev/MeshCore](https://github.com/meshcore-dev/MeshCore)
- Flood routes relayed once per repeater, direct routes relayed only by the repeater matching the next path hash; token-bucket duty cycle 33% (v1.6) / 50% (v1.18, configurable); multipart payloads since v1.18. — [irab technical page](https://irab.github.io/mesh/technical.html)
- Seeed's comparison: Meshtastic default 3 / max 7 hops vs MeshCore up to 64; MeshCore suits fixed backbones, Meshtastic suits mobile zero-setup groups; runs on the same boards (reflashable). Seeed describes MeshCore as "freemium & partially proprietary". — [Seeed blog, 23 Mar 2026](https://www.seeedstudio.com/blog/2026/03/23/meshcore-vs-meshtastic/); conflicts with the MIT statement on the [MeshCore repo](https://github.com/meshcore-dev/MeshCore) (likely the firmware library is MIT while some apps are not; unresolved).
- A Home Assistant integration for MeshCore exists. — [meshcore-dev/meshcore-ha](https://github.com/meshcore-dev/meshcore-ha) (search hit, not read)

Raw LoRa in Rust
- `lora-phy` 3.0.1 (released 2024-07-11; SX126x and SX127x; embedded-hal-async; one API for LoRaWAN and point-to-point), developed in the lora-rs/embassy ecosystem; no newer release since. — [docs.rs lora-phy](https://docs.rs/crate/lora-phy/latest)

BLE long range
- Nordic's own nRF52840 test at 0 dBm TX: about 655-682 m on the 1 Mbps PHY and about 1,300 m on Coded PHY (125 kbps), the latter partly over sea water. — [Nordic blog: Tested by Nordic, Bluetooth Long Range](https://blog.nordicsemi.com/getconnected/tested-by-nordic-bluetooth-long-range)

Zigbee/Thread via the Yellow
- The Yellow's onboard radio is a Silicon Labs MGM210P (Zigbee 3.0 / OpenThread / Matter, 2.4 GHz, up to +20 dBm). — [Home Assistant Yellow](https://www.home-assistant.io/yellow)

Comparison table (cells marked * are estimates or [BK], not sourced this session)

| Link | Cost per end | Usable data rate | Open-terrain range | Power draw | Carries IP / full MCP? | Linux ARM64 + Rust maturity |
|---|---|---|---|---|---|---|
| Meshtastic (SX1262, ShortFast/ShortTurbo) | $10-33 (XIAO kit $9.90, Heltec V4 $21.90-27.50, MeshStick $32.97) | 9-18 kbps best case (computed); 230 B packets | km-class*; 250 m trivial | ~0.1-0.6 W* | No (IP tunnel exists but impractical); compact protocol only | Good: `meshtasticd`, Python lib, `meshtastic` crate 0.1.9 |
| Meshtastic LongFast | same | ~0.84 kbps best case (computed) | 1-10+ km* | same | No | same |
| Reticulum over RNode (same boards) | $18-28 | Set by chosen SF/BW; same PHY limits as above | same as LoRa | same | Not IP, but links, RPC-style requests and resumable Resources; same stack also runs over Wi-Fi | Python reference is solid; Rust `reticulum-rs` 0.17 is young |
| MeshCore | $18-45 | LoRa-class | LoRa-class | low | No | Python/JS companion libs; no Rust found |
| Raw LoRa P2P (own protocol) | $10-25 | Up to SF5-7 / 500 kHz* | LoRa-class | low | No | `lora-phy` 3.0.1 (2024) on MCUs; on Linux via SPI is DIY |
| ESP-NOW (normal) | $3-6* | 50-400 kbps measured | ~150 m reliable, 300 m marginal (PCB antennas) | ESP32 active ~80-300 mA peak* | No (needs gateway); 250 B frames* | Needs an ESP32 bridge; Rust via esp-hal/esp-wifi* |
| ESP-NOW Long Range | $3-6* | 100 kbps at 150 m, ~10 kbps at 900 m measured | ~450 m reliable, ~900 m marginal | same | No | Espressif-only PHY; same as above |
| Wi-Fi + outdoor CPE (CPE210-class) at bench | ~$40-60* for one unit; pack uses onboard Wi-Fi | Mbps to tens of Mbps* at 250 m LOS | 250 m with one directional end*; km with a pair | CPE up to 10.5 W; pack Wi-Fi ~1 W* | Yes: full MCP, sync, video | Excellent (standard networking) |
| Wi-Fi HaLow (MM6108/MM8108) | $15-60 per radio plus HAT/adapters | ~1-10 Mbps (11 Mbps at 500 m, 1 Mbps at 3 km vendor) | 1-3 km | ~1 W* | Yes | Poor today: out-of-tree driver + kernel patches on Pi OS; mainline `mm81x` only just landing |
| BLE Coded PHY (nRF52840 / ESP32-C6) | $5-15* | <= 125 kbps PHY | several hundred m to ~1.3 km at 0 dBm (Nordic) | very low | No (GATT/L2CAP) | BlueZ + `btleplug`*; workable but fiddly |
| Thread/Zigbee 2.4 GHz (Yellow's MGM210P) | $0 at bench; $5-10* per pod | 250 kbps PHY* | tens of m to ~100 m* per hop | very low | Thread carries IPv6 (6LoWPAN) but not practical for MCP | HA-native; fine for pods near buildings, not for the far corner |
| LoRaWAN + private ChirpStack | $100+* gateway | 51-242 B frames*, class A latency | km | low | No | Overkill for one mobile node |
| Amazon Sidewalk | n/a | very low | n/a | n/a | No | Needs Amazon gateways and cloud; unsuitable for an off-grid local link* |
| DECT NR+ (nRF91x1) | $100+* dev kits | Mbps-class claims* | km claims* | medium | Possible | Immature for hobby Linux use* |
| APRS / packet radio (ham) | $30-150* | 1.2-9.6 kbps* | many km | 1-5 W | No | Needs a licence (owner's status unknown) and encryption is prohibited [BK]; note only |

### Inferences
- Link budget sanity check for Wi-Fi at 250 m (computed): 2.4 GHz free-space loss about 88 dB. CPE at 27 dBm + 9 dBi to a 0 dBi pack antenna gives about -52 dBm at the pack; the Pi 5's roughly 15 dBm* back into the 9 dBi dish gives about -64 dBm at the CPE. Both are comfortably inside high-rate 802.11n sensitivity, so one directional outdoor AP at the bench end, aimed down the long axis of the property, should give full-rate IP to the far corner as long as the path is clear of the container and shed. The Espressif data (under 6 Mbps at 150 m with two PCB antennas) shows why the directional end matters.
- For the project's ESP32-C3/C6 scatter pods, ESP-NOW (normal mode inside about 150 m, LR mode beyond) is the natural pod-to-gateway link at no extra hardware cost; LoRa is only needed for pods past about 400 m or behind metal structures.
- HaLow would be the "one radio does everything out to 1-3 km with IP" answer, but in Nov 2026 it costs days of kernel work on a Pi and the cheap Seeed module looks discontinued. Revisit once `mm81x` reaches the Raspberry Pi OS kernel.
- Reticulum's value here is architectural: the same destination hash is reachable over Wi-Fi (AutoInterface) when in range and over an RNode when not, with link-layer proofs, request/response and resumable Resources already built. That is the project's "graceful degradation" requirement delivered by an existing stack. Costs: the mature implementation is Python (the Rust port is young), and an RNode-flashed board is not a Meshtastic node, so it gives up the community mesh, phone apps and HA integration.

### Gaps
- No US street prices verified for TP-Link CPE210/CPE510, Ubiquiti NanoStation loco, eero outdoor units, ALFA Tube-AHM, Heltec HaLow boards (the Heltec HaLow category page returned no product data).
- ESP-NOW LR PHY rates ([BK]: 1/2 Mbps and 1/4 Mbps, about -105 dBm sensitivity per ESP-IDF docs) and the ESP-NOW frame limit ([BK]: 250 B v1, 1470 B v2) were not on the pages fetched.
- No independent (non-vendor) HaLow throughput measurement at 200 m-1 km was found; Jeff Geerling's HaLow tests were searched for but not located.
- Thread/Zigbee, Sidewalk, DECT NR+ and APRS rows are mostly background knowledge.
- Reticulum's achievable bit rate over RNode and the `rncp`/`rnsh` utilities ([BK]: file copy and remote shell over Reticulum) were not confirmed from fetched pages.

---

## KQ4. For this property: is LoRa needed? Primary and fallback

### Takeaway
No, LoRa is not needed to reach 250 m: one outdoor directional Wi-Fi access point on the home LAN gives full IP and full MCP at thousands of times LoRa's rate. LoRa/Meshtastic still earns a place as the cheap (about $45-55 for both ends), infrastructure-free fallback and as a reusable building block (beyond-property range, fence-line pods, RF test target, community mesh). Recommended: Wi-Fi primary, Meshtastic on a private ShortFast/ShortTurbo channel as fallback, local store-and-forward underneath both.

### Cited Findings
- Meshtastic link budgets of 140 dB (ShortTurbo) to 153 dB (LongFast) at 22 dBm. — [Meshtastic radio settings](https://meshtastic.org/docs/overview/radio-settings/)
- Bare-antenna Wi-Fi drops below 6 Mbps at 150 m, while ESP-NOW-LR holds near 100% delivery to 450 m; a 9 dBi CPE claims 5 km+ in bridge pairs. — [Espressif developer blog](https://developer.espressif.com/blog/esp-now-for-outdoor-applications/); [TP-Link CPE210](https://www.tp-link.com/us/business-networking/outdoor-radio/cpe210)
- A LoRa node with INA219/INA226/INA3221 sensors reports voltage/current as stock "power metrics" telemetry with no custom code. — [Telemetry module docs](https://meshtastic.org/docs/configuration/module/telemetry/)

### Inferences
- Arguments for still including the LoRa path: (1) works with zero infrastructure and with the eero/Starlink down; (2) reaches well beyond the lot (neighbouring parcels, the road, a vehicle) where no AP will ever be; (3) the same $10-25 boards serve far-fence pods; (4) lower standby power than keeping the pack's Wi-Fi associated at the edge of range; (5) a known, controllable 915 MHz emitter is exactly what the SDR array's LoRa-sniffing work needs as a test target; (6) doubles as off-grid text messaging for the household; (7) metal structures (container, shed, array) that shadow 2.4 GHz are far less of a problem at 915 MHz with 60 dB of spare margin.
- Arguments against making it primary: 3-5 orders of magnitude less throughput, no IP, new protocol code to write and secure, and RF coexistence work with the SDR receivers (KQ7).
- The simplest thing that satisfies the owner's first example (multimeter values from a failing solar panel) is not LoRa at all: carry the MacBook, or let the pack join the home Wi-Fi through an outdoor AP. LoRa is for when neither is available or wanted.
- A zero-code first demo of the LoRa path: put an INA226/INA3221 on the pack's Meshtastic node and let stock power telemetry flow to Home Assistant entities, before any custom protocol exists.

### Gaps
- Actual 2.4 GHz and 915 MHz propagation on the site (container, metal shed and array positions relative to the far corner) is unknown; a one-afternoon Range Test with two nodes would settle it.
- Whether the owner's eero model has a spare Ethernet port near the bench for an outdoor AP was not checked.

---

## KQ5. Bench side on the Home Assistant Yellow, and how Claude sees the readings

### Takeaway
The Yellow offers two USB 2.0 host ports and no documented GPIO header, so attach the bench radio by USB (an ESP32 Meshtastic node as a serial device, or a CH341 MeshStick under `meshtasticd`) or let an ESP32 node join Wi-Fi and publish to the Mosquitto broker. Because a Meshtastic node accepts one client at a time and Home Assistant will not decode a private binary port, the project's own bench daemon should own the radio, verify and de-duplicate messages, write them to the knowledge store, publish MQTT (with discovery) for Home Assistant, and expose MCP tools to Claude.

### Cited Findings
- Yellow hardware: 2x USB 2.0 Type-A host, 1x USB-C device port (UART debug by default), M.2 M-key for NVMe (PCIe x1), MGM210P radio, stereo DAC with 3.5 mm line-out, 12 V/2 A barrel or PoE+, Gigabit Ethernet, RTC; about 1.5 W idle, 5-9 W under load; no GPIO/expansion header is mentioned; the product is no longer in production but still receives updates. — [Home Assistant Yellow](https://www.home-assistant.io/yellow)
- Official integration `meshtastic/home-assistant` (install as a HACS custom repository): connects by TCP, serial (USB) or Bluetooth (also via ESPHome Bluetooth proxy); exposes node metrics as sensors/binary sensors and device trackers; `notify.mesh_*` entities and actions to send direct/broadcast text, request telemetry/position, traceroute; device triggers on message sent/received; logbook; a built-in MQTT client proxy and a bundled web client. Stated limits: "Meshtastic firmware allows only one connection at a time"; add a delay of a couple of seconds before actions triggered by Meshtastic events; the web-client proxy is unauthenticated to anyone who can reach HA. — [github.com/meshtastic/home-assistant](https://github.com/meshtastic/home-assistant)
- MQTT route: node publishes JSON to topics like `msh/US/2/json/LongFast/!<nodeid>`; HA sensors use `value_template` on `from` and `payload.*` fields (battery_level, voltage, temperature, text, ...); to send from HA publish `{"from": <nodenum>, "type": "sendtext", "payload": "..."}` to `msh/US/2/json/mqtt/!<nodeid>`, which requires a channel named "mqtt" with downlink enabled; JSON is not available on nRF52 nodes. — [Meshtastic docs: Home Assistant via MQTT](https://meshtastic.org/docs/software/integrations/mqtt/home-assistant/)
- `meshtasticd` needs an SPI radio or a CH341 USB radio and root/GPIO/SPI permissions. — [meshtasticd hardware docs](https://meshtastic.org/docs/software/linux/hardware/)
- Home Assistant MCP Server: endpoint `/api/mcp` (per-toolset URLs such as `/api/mcp/assist`), OAuth 2.0 with PKCE, tool sets "Device control" (entities exposed to voice assistants) and "Home Assistant" (admin), a read-only option, one resource (`homeassistant://assist/context-snapshot`); no sampling or notifications; Claude Code connects with `claude mcp add --transport http home-assistant <url>`; claude.ai connectors reach HA from the cloud, so HA must be internet-reachable for those. — [Home Assistant MCP Server integration](https://www.home-assistant.io/integrations/mcp_server/)
- The Rust `meshtastic` crate speaks the client protobuf API over serial or TCP. — [github.com/meshtastic/rust](https://github.com/meshtastic/rust)

### Inferences
- Wiring options at the bench, in order of preference:
  1. ESP32-S3 node (Heltec V4) on a USB port of the Yellow, owned by a Rust bench daemon over serial protobuf. Works the same on Raspberry Pi OS (plain process/systemd) and on HA OS (as an add-on container with the serial device mapped, the same way Zigbee USB dongles are passed through). The node can sit on a 2-3 m USB cable to get the antenna near a window or outside.
  2. Same node placed high/outdoors on its own power, joined to Wi-Fi; the daemon connects by TCP (port 4403 [BK]) or consumes MQTT protobuf topics from Mosquitto. Best antenna placement, but depends on Wi-Fi.
  3. MeshStick + `meshtasticd` on the Yellow under Raspberry Pi OS; the daemon connects to `meshtasticd` by TCP on localhost. Fewer moving parts in firmware terms, but on HA OS it needs a custom add-on, and the radio dies with the host.
  An SX1262 on GPIO/SPI is not realistic on the Yellow (no documented header); the M.2 slot is PCIe-only.
- Use the HACS integration or the JSON MQTT route only for the stock data types (text, device and power telemetry, position). The project's private-port binary messages should be decoded by the daemon and re-published as retained MQTT topics with HA MQTT discovery, e.g. `pack/<pack_id>/reading/<channel>` with attributes `{value, unit, uncertainty, t_measured, t_received, seq, record_id, link: "lora", verified: false}`.
- How Claude gets the data: (a) preferred, the bench daemon is itself an MCP server (streamable HTTP on the bench LAN) with tools such as `pack_remote.latest(channel)`, `pack_remote.history(since)`, `pack_remote.request_reading(channel)` (queues a radio request and returns when the reply or a timeout arrives), `pack_remote.say(phrase_id | text)`, `pack_remote.link_status()`; every result carries provenance (link tier, age, sequence gap count, verified flag). (b) Alternatively expose the MQTT-discovered entities to Assist and use Home Assistant's own MCP Server; simpler, but it has no notifications and no notion of provenance or record hashes.
- Time-stamping: keep two times per record, `t_measured` from the pack clock (in the message) and `t_received` from the bench clock; the pack's clock comes from GNSS or NTP while on Wi-Fi, and the bench should also log the pack's reported clock offset whenever both links are up. Order by (boot_id, seq), not by arrival.
- De-duplication and late arrivals: key every message by (pack_id, boot_id, seq); Meshtastic retries and MQTT bridging can deliver duplicates, so the daemon keeps a seen-set per boot and ignores repeats; gaps in seq are recorded as "missing over radio" rather than silently skipped.
- Mapping LoRa summaries to full-resolution records: the radio message carries `record_id` = the first 8 bytes of the content hash of the full record already written to the pack's hash-chained store. The bench stores the summary as a provisional record ("radio-reported, unverified"). When the pack returns to Wi-Fi and the chain syncs, the bench recomputes the hash of the full record, matches the 8-byte prefix and the value, and promotes it to "verified"; a mismatch is a loud integrity event.

### Gaps
- Whether a maintained `meshtasticd` add-on for HA OS exists was not checked.
- Whether the HACS integration surfaces packets on private ports as events was not confirmed (assumed not).
- Whether the Yellow's PCB has usable unpopulated GPIO pads was not confirmed; the official page lists none.
- The TCP API port number is [BK].

---

## KQ6. Reliability and integrity over a lossy link; which commands to allow

### Takeaway
Use Meshtastic's ACK/retry for single packets, add an application sequence number, a boot id and an 8-byte truncated MAC to every message, and treat anything larger than one packet as a fragmented, erasure-coded transfer that is optional and resumable. Allow only read-only requests and speech over the radio; keep actuation for the IP tier. Part 15 has no restriction on encryption; amateur-band operation would.

### Cited Findings
- Reliable direct messages are retried (up to 5 transmissions total per the docs) and then NAKed locally; packets have a 32-bit id; PKC/AEAD adds 12 bytes. — [Meshtastic mesh algorithm docs](https://meshtastic.org/docs/overview/mesh-algo/)
- A working fragment/FEC design over Meshtastic: chunks of 48-215 B by preset, Reed-Solomon parity shards (10-100% by loss), NACK windows of 3 s, 4-byte per-chunk integrity tag, pacing at about airtime + 30%. — [voicetastic Constants and Limits](https://github.com/voicetastic/voicetastic-core/wiki/Constants-and-Limits)
- Reticulum provides signed delivery proofs per packet and integrity-verified, retransmitting Resources per link. — [Reticulum manual: Understanding](https://reticulum.network/manual/understanding.html)
- Meshtastic's licensed (ham) mode lifts power limits in software. — [Meshtastic radio settings](https://meshtastic.org/docs/overview/radio-settings/)
- 15.247 contains power, bandwidth, PSD and dwell rules but nothing on content, encryption or automatic control. — [47 CFR 15.247](https://www.law.cornell.edu/cfr/text/47/15.247)

### Inferences
- Layering: (1) LoRa's own coding rate 4/5 handles bit errors within a packet; (2) Meshtastic `want_ack` handles whole-packet loss for single messages; (3) for multi-packet objects add packet-level erasure coding (Reed-Solomon or simple XOR parity every N fragments) so that one lost fragment does not cost a full ACK round. At 250 m with 40-60 dB margin loss will be near zero, so start with ACK+retry only and add parity when the 1-5 km case arrives.
- Fragment budget (computed): a 2 kB evidence summary is 12 fragments of 180 B: about 3-4 s on ShortFast, about 35-60 s on LongFast including ACKs. A 6 kB thumbnail is 34 fragments: about 10 s on ShortFast, about 2 minutes on LongFast. Both are fine as explicit, agent-requested transfers; neither should be automatic.
- MAC length: every message should carry a truncated MAC (HMAC-SHA-256 or BLAKE3 keyed hash) under a pack-bench key that is independent of the Meshtastic channel key. 8 bytes (64 bits) is a reasonable default for both readings and commands: an online forger gets one guess per packet at roughly 1-10 packets per second, so 2^64 is out of reach; 4 bytes is acceptable for bulk readings whose full records will be verified after sync [BK: LoRaWAN uses a 4-byte MIC]. Do not rely on channel encryption for authenticity ([BK]: Meshtastic channel encryption is AES-CTR without authentication; PKC direct messages are authenticated).
- Replay protection: include (boot_id, seq) under the MAC; the receiver rejects seq <= last accepted for that boot_id; commands additionally carry a short validity window (expiry in seconds from the bench clock) so a delayed command cannot fire later.
- Commands allowed over the radio: read-only and advisory only: `get_reading(channel)`, `get_status`, `start/stop a pre-approved measurement plan by id`, `resend(seq range)`, `fetch(record_id, detail level)`, `say(phrase id or text)`, `earcon(id)`, `ping`. Not allowed over the radio: anything that energises outputs or changes the device under test (relay/load switching, power-supply set-points, SDR transmit), firmware/config changes, shell access. Those wait for the authenticated IP tier and the project's physical-safety layer. A slow, lossy, half-duplex link is the wrong place for actions that need confirmation loops.
- Encryption: fine under Part 15. If the owner ever moves this to amateur frequencies (higher power via `is_licensed`), messages may not be encrypted or obscured and automatic control has its own rules [BK: 47 CFR 97.113 and 97.221]; the design should then switch to signed-but-plaintext messages, which the MAC/ID scheme above already supports.

### Gaps
- Part 97 specifics were not fetched this session; the owner's licence status is unknown.
- No source found for measured packet loss of Meshtastic at 100-300 m with reduced power.
- NIST/RFC guidance on minimum truncated-MAC length was not fetched ([BK]: RFC 2104 suggests at least half the hash length and not under 80 bits for general use; many constrained protocols accept 32-64 bits with rate limiting).

---

## KQ7. Power and hardware integration on the pack; coexistence with the SDR receivers

### Takeaway
Make the pack's LoRa radio a USB-attached module (an ESP32-S3 + SX1262 board at about $22-28, or a $10 XIAO kit) spoken to over serial protobuf from Rust; it keeps the Pi's GPIO header free, survives Pi crashes, and can be reflashed to RNode/MeshCore/raw LoRa later. The serious integration issue is RF: a 22 dBm transmitter centimetres from an RTL-SDR antenna can exceed the dongle's roughly +10 dBm damage level, so run the link at low power (it has 40-60 dB to spare at 250 m), separate the antennas, and blank or filter the SDRs during transmit.

### Cited Findings
- Maximum RF input levels quoted for SDRs: RTL-SDR +10 dBm, HackRF -5 dBm, Pluto and BladeRF +2.5 dBm, SDRplay RSP1A 0 dBm; a band-pass filter is recommended against strong out-of-band signals; an LNA reduces the tolerable input by its gain. — [OneSDR: how not to break your SDR](https://www.onesdr.com/2020/01/29/how-not-to-break-your-software-defined-radio-hardware)
- The RTL-SDR Blog V3 has an ESD diode plus a BAV99 clipper on the front end; it covers 500 kHz-1766 MHz (so 915 MHz is in-band and cannot be removed by the tuner). — [RTL-SDR Blog V3 datasheet](https://www.mouser.com/datasheet/2/813/RTL_SDR_Blog_V3_Datasheet-3305974.pdf)
- A community thread describes a >50 dB SAW "shield" for hardening an RTL-SDR V4 around 915 MHz against de-sensing. — [RTL-SDR discourse thread (title only, not read)](https://rtl-sdr.discourse.group/t/hardening-the-v4-for-915mhz-solving-urban-de-sensing-with-a-50db-saw-shield/83)
- Heltec V4: 28 dBm high-power variant on 863-928 MHz, IPEX LoRa antenna connector, 8-pin GNSS header (L76K $8.00), solar input header, USB-C. — [heltec.org WiFi LoRa 32 V4](https://heltec.org/project/wifi-lora-32-v4/)
- MeshStick: CH341A + SX1262 with TCXO, +22 dBm, SMA, 36.6 g, works with `meshtasticd` on Raspberry Pi. — [Rokland MeshStick listing](https://store.rokland.com/collections/new-arrivals-1/products/elecrow-meshstick-usb-to-spi-sx1262-tcxo-lora-usb-stick-meshtastic-lora-mesh-node)
- The Meshtastic time base can come from GPS (firmware sets time from GPS every 30 minutes as of 2.7.26). — [newreleases.io v2.7.26](https://newreleases.io/project/github/meshtastic/firmware/release/v2.7.26.54e0d8d)

### Inferences
- Coupling estimate (computed, free-space approximation, 2 dBi antennas): 915 MHz path loss is about 12 dB at 10 cm and about 32 dB at 1 m. A 22 dBm LoRa transmit therefore puts roughly +14 dBm into an SDR antenna 10 cm away (above the RTL-SDR's +10 dBm limit) and about -6 dBm at 1 m (safe from damage, but far above the level where an 8-bit RTL-SDR saturates). A 28 dBm Heltec V4 at full power is 6 dB worse.
- Mitigations, cheapest first: (1) set LoRa TX power to the minimum that works, e.g. 0-5 dBm on ShortFast at 250 m still leaves about 35-40 dB of margin and drops the coupled power by 17-22 dB; (2) put the LoRa antenna on the opposite end of the case from the SDR antennas, cross-polarised if possible; (3) TX blanking in software: the pack daemon knows when it hands a packet to the node, so it marks SDR capture windows as "self-TX" (or pauses capture and sets tuner gain to minimum) for the 30-200 ms burst; with the SDRs listening on 915 MHz these bursts are also free calibration signals; (4) a 915 MHz band-stop/notch or switched attenuator ahead of SDRs that are not meant to watch 915 MHz; (5) never transmit with SDR LNAs/bias-tees enabled on a nearby antenna.
- USB module vs SPI HAT: USB serial to an ESP32-S3 node costs one USB port and about 0.1-0.5 W*, keeps the 40-pin header free for the pack's other modules, and the node keeps relaying/answering pings while the Pi reboots. A `meshtasticd` + SPI/CH341 radio saves one MCU and puts the whole stack under Linux control (easier logging and agent-tweakable config), at the cost of root GPIO/SPI access and losing the radio when the Pi is down. Given the project's "agent can fix its own instruments" goal, the USB node with its own flashable firmware is the more modular choice, and a second firmware (RNode) is a reflash away.
- Antenna in a plastic case: plastic is RF-transparent, but batteries, the Pi, SDR shields and wiring detune a small whip. Use a bulkhead SMA on the case lid with a short 915 MHz whip held vertical, away from the SDR antenna ports; avoid stubby coil antennas shipped with the cheapest kits for the 1-5 km case.
- GNSS sharing: give GNSS to the Pi (gpsd, PPS if available) as the pack's time and position source and let the pack daemon inject time/position into the node, or fit the $8 L76K to the node and read position back over the client API; one receiver is enough.

### Gaps
- RTL-SDR's absolute maximum input is from a secondary article, not the datasheet (the datasheet gives none). No source found measuring RTL-SDR desense from a co-located 915 MHz LoRa transmitter.
- Measured current draw of Heltec V4 / XIAO kit under Meshtastic not found.
- The SAW-filter thread was not read; filter part numbers and prices not gathered.

---

## KQ8. Voice output (and voice input) on the pack

### Takeaway
The cheapest reliable way for a Pi 5 to speak short results offline is a subprocess call to espeak-ng (instant, robotic, a few MB) or Piper (natural, under a second per sentence on a Pi 5), with frequently used phrases pre-rendered to WAV and numbers slotted in, plus distinct tones (earcons) for good/bad/complete. Over the radio the agent sends a tiny "say" command (phrase id plus numeric slots, or free text under about 180 bytes); voice input goes the other way as locally transcribed text.

### Cited Findings
- Piper (`en_US-lessac-medium`) on a Raspberry Pi 5: "full sentence generation takes under a second"; faster-whisper Tiny transcribes in 2-3 s on a Pi 5; Whisper Tiny + Piper + a small LLM together use about 3-4 GB of RAM. — [dev.to: Raspberry Pi 5 local voice AI, what works in 2026](https://dev.to/pat9000/raspberry-pi-5-local-voice-ai-what-works-in-2026-21ic)
- Qualitative survey (11 Sep 2026): Piper voices are typically well under 100 MB and widely reported real-time on a Pi 4; "medium" is the recommended default tier; espeak-ng needs only a few MB; Kokoro is 82 M parameters / about 327 MB with no documented Pi real-time benchmark found by the author. — [PromptQuorum: best local TTS on Raspberry Pi](https://www.promptquorum.com/power-local-llm/best-local-tts-raspberry-pi)
- KittenTTS runs on Pi 4 and Pi 5 (Pi 5 faster; no timings given) using an Adafruit Voice Bonnet with two 1 W speaker outputs. — [Adafruit: Speech synthesis on Raspberry Pi with KittenTTS](https://learn.adafruit.com/speech-synthesis-on-raspberry-pi-with-kittentts.md)
- Meshtastic text payloads and the Python API share the same packet limit (about 230 B), and the HA example caps its send box at 190 characters. — [Meshtastic docs: Home Assistant via MQTT](https://meshtastic.org/docs/software/integrations/mqtt/home-assistant/)

### Inferences
- Recommended voice stack: (1) earcons first: three or four short synthesized tone patterns (rising two-tone = good, low buzz = bad, triple beep = complete, single tick = "reading taken") in the 1-3 kHz band where small speakers are most efficient and wind noise (mostly below 500 Hz) is weakest; (2) a phrase cache of pre-rendered Piper WAVs ("voltage received", "reading stable", "move to the next panel", digits, units) concatenated at run time so speech needs no synthesis delay; (3) Piper as a subprocess for free text when the agent sends it; (4) espeak-ng as the always-works fallback. From Rust, spawn `piper`/`espeak-ng` and play through ALSA (`aplay`) or a Rust audio crate; no model needs to live in the Rust process.
- Hardware: the planned ReSpeaker XVF3800's amplified output (5 W per the project brief) is the obvious speaker driver, since the mic array's echo canceller needs the playback reference anyway. Fallback: a MAX98357A I2S amplifier (about $6*, 3 W class D) with a 3 W 4-ohm speaker, or any USB speaker. For outdoor use at arm's length to a few metres, 3-5 W into an efficient 40-50 mm driver in a ported enclosure is adequate; beyond that, rely on earcons.
- Triggering speech over a slow link: `say` command = 1 B type + 2 B phrase id + up to 3 numeric slots (value, unit enum) = under 16 B before seq/MAC, e.g. phrase 17 "voltage received: {v} volts" with v = 18.4. Free text falls back to UTF-8 up to about 180 B. The pack should speak local events without waiting for the bench at all ("reading stable, 18.4 volts, sent") and only use bench-originated speech for the agent's instructions.
- Voice input over a slow link: run speech-to-text on the pack (whisper.cpp / faster-whisper tiny or base; Vosk or Moonshine as lighter options [BK]) and send only the transcript text (a 10-word utterance is about 60 B, one packet), tagged with the STT confidence so the bench agent can ask for a repeat. A push-to-talk button on the pack avoids wake-word errors in wind.

### Gaps
- No measured Piper real-time factor per voice tier on a Pi 5, and no Pi benchmarks for Kokoro, KittenTTS, Moonshine or Vosk, were found; the only numbers are the soft claims above. Benchmark on the actual pack.
- Speaker/amplifier prices (MAX98357A, USB speakers) and required outdoor sound pressure levels were not sourced.
- Rust crates for TTS/STT (bindings for Piper, whisper) were not surveyed.

---

## KQ9. Requested end-of-notes deliverables (link stack, bill of materials, message format, limits, ideas)

### Takeaway
Build three tiers: full MCP over IP (pack Wi-Fi or an outdoor AP), compact signed telemetry/commands over a private Meshtastic channel, and always-on local recording with later hash-verified sync. The LoRa path costs about $45-55 for both ends, a reading fits in 35 bytes, and the things that will not work over LoRa are anything streaming, large or safety-critical.

### Cited Findings
- All numbers below draw on the sources cited in KQ1-KQ8: preset table ([Meshtastic radio settings](https://meshtastic.org/docs/overview/radio-settings/)), packet limits and ACKs ([mesh algorithm docs](https://meshtastic.org/docs/overview/mesh-algo/)), private ports ([portnums.proto](https://github.com/meshtastic/protobufs/blob/master/meshtastic/portnums.proto)), Rust crate ([docs.rs meshtastic](https://docs.rs/crate/meshtastic/latest)), board prices ([heltec.org](https://heltec.org/project/wifi-lora-32-v4/), [Rokland](https://store.rokland.com/collections/new-arrivals-1/products/elecrow-meshstick-usb-to-spi-sx1262-tcxo-lora-usb-stick-meshtastic-lora-mesh-node), [adrelien](https://adrelien.com/xiao-esp32s3-wio-sx1262-kit-the-cheapest-meshtastic-node/)), FCC rule ([47 CFR 15.247](https://www.law.cornell.edu/cfr/text/47/15.247)), Yellow hardware ([home-assistant.io/yellow](https://www.home-assistant.io/yellow)), HA MCP ([mcp_server](https://www.home-assistant.io/integrations/mcp_server/)).

### Inferences

#### (1) Recommended link stack, in tiers

| Tier | Link | When | What the agent can do |
|---|---|---|---|
| 0 | USB-C / Ethernet tether at the bench | Docked | Everything, plus firmware and bulk sync |
| 1 | IP over Wi-Fi: pack's own AP to the MacBook, or pack as a client of an outdoor directional AP on the home LAN | MacBook carried along, or pack within about 250 m line of sight of the AP | Full MCP (all tools, streaming, images, audio), live knowledge-store sync, actuation under the safety layer |
| 1b (later) | IP over HaLow, or Reticulum over whatever is up | 0.3-3 km, once Pi OS drivers mature | As tier 1 at 1-10 Mbps |
| 2 | Meshtastic private channel, ShortFast or ShortTurbo, private port 256+, compact signed messages | No IP path; up to km-class range | Receive readings (1-10 per second sustained on Short presets), request a reading or a pre-approved test plan, get status, trigger speech/earcons, pull a 2 kB summary or tiny thumbnail on request. Read-only; no actuation |
| 2b | Same, LongFast | 1-5 km or obstructed | About one reading every 1-3 s, round trips of 2-4 s, summaries in about a minute |
| 3 | Store-and-forward on the pack (hash-chained store), radio carries only record ids/hash prefixes | Always on underneath | Nothing live beyond tier 2; on return to tier 0/1 the bench verifies every radio-reported summary against the full record |
| 4 | No link at all | Radio failure | Pack runs its loaded test plan autonomously, speaks results, records everything for later sync |

Link selection should be automatic: the bench MCP server exposes one "pack" surface and labels every answer with tier, age and verified/unverified, so the agent always knows how much to trust and how much it may ask for.

#### (2) Bill of materials for the LoRa path

Recommended (USB nodes at both ends, stock Meshtastic firmware):

| Item | Qty | Unit price | Source / date |
|---|---|---|---|
| Heltec WiFi LoRa 32 V4.3 (ESP32-S3R8 + SX1262, 902-928 MHz, antenna included) for the pack | 1 | $21.90-27.50 | heltec.org, 25 Nov 2026 (Amazon band $25-40, Oct 2026) |
| Same board for the bench (USB to the Yellow, or Wi-Fi to MQTT) | 1 | $21.90-27.50 | same |
| USB-C cables (pack: C-to-C or A-to-C short; bench: A-to-C 2-3 m) | 2 | about $5 each (unverified) | any |
| Optional: bulkhead SMA pigtail + 915 MHz whip for the pack case | 1 | about $8-12 (unverified) | any |
| Optional: INA226 or INA3221 breakout on the pack node for zero-code power telemetry | 1 | about $3-8 (unverified) | any |
| Optional: L76K GNSS for the pack node | 1 | $8.00 | heltec.org, 25 Nov 2026 |

Total: about $44-55 for the two radios (verified prices), about $55-75 with cables and antenna, about $65-90 with the optional sensor and GNSS; shipping not included.

Budget variant: 2x Seeed XIAO ESP32S3 + Wio-SX1262 kit at about $9.90 each (2025 price, verify) = about $20, with weaker antennas and no display. Linux-native variant: 2x Elecrow MeshStick at $32.97 = $65.94 with `meshtasticd` on the Pi 5 and on the Yellow under Raspberry Pi OS. Avoid the $25.99 Waveshare UART "SX1262 LoRa HAT" and the Waveshare LoRaWAN HAT.

Configuration: US region, one private channel with its own 256-bit key, preset ShortFast (or ShortTurbo), a non-default frequency slot, hop limit 0-1, both nodes CLIENT_MUTE-style so they do not relay public traffic, TX power reduced to about 0-10 dBm on the property, position and device-telemetry broadcasts turned down, direct messages with `want_ack`.

#### (3) Proposed compact message format (postcard or CBOR on a private port, one message per packet)

Common envelope (13 bytes): `ver_type` u8 (4 bits version, 4 bits type) | `boot_id` u16 | `seq` u16 | ... body ... | `mac` 8 B (truncated keyed hash over ver_type..body plus sender id).

Reading (type 1), fixed binary layout:

| Field | Bytes | Notes |
|---|---|---|
| ver_type | 1 | |
| boot_id | 2 | increments on each pack boot; with seq gives ordering and replay protection |
| seq | 2 | per-boot counter |
| record_id | 8 | first 8 bytes of the content hash of the full-resolution record in the pack's store (id and hash prefix in one) |
| t_measured | 4 | Unix seconds (pack clock) |
| t_frac | 1 | 1/256 s |
| channel | 1 | instrument/channel id from the synced dictionary |
| unit | 1 | enum from the synced dictionary (V, A, ohm, degC, dBm, ...) |
| value | 4 | f32 |
| uncertainty | 2 | f16, same unit |
| flags | 1 | stable / overrange / estimated / more-detail-available |
| mac | 8 | |
| Total | 35 B | plus about 6 B protobuf wrapper + 16 B Meshtastic header = 57 B on air: about 30 ms ShortTurbo, 60 ms ShortFast, 0.7 s LongFast |

Batch reading (type 2): one envelope, then N x (channel 1, unit 1, value 4, uncertainty 2, dt 1) = 9 B each, so about 20 channels per packet with one shared MAC and one `record_id` for the batch.

Command, bench to pack (type 8): ver_type 1 | boot_id 2 | seq 2 | cmd 1 | expiry_s 1 | args 0-24 (postcard) | mac 8 = 15-39 B. Commands: 0x01 ping, 0x02 get_status, 0x10 get_reading(channel), 0x11 run_plan(plan_id), 0x12 stop_plan, 0x20 resend(seq_from, seq_to), 0x21 fetch(record_id, level), 0x30 say_phrase(phrase_id, slots...), 0x31 say_text(utf8 <= 180 B), 0x32 earcon(id). Reply (type 9): echoes cmd and seq, status code, optional body.

Fragment (type 12): ver_type 1 | xfer_id 2 | idx 1 | total 1 | payload <= 180 B | tag 4 B; whole-object hash sent in the first fragment; optional XOR/Reed-Solomon parity fragments; receiver replies with a bitmap NACK.

Beacon (type 15, every few minutes): boot_id, seq, chain-head hash prefix 8 B, battery, store backlog count, mac: about 28 B. This gives the bench a tamper-evident commitment to everything recorded so far, even with nothing else delivered.

Shared context (synced over Wi-Fi, identified by an 8-byte hash carried in the status reply): channel table, unit enum, phrase table, plan table, tool-schema hashes. If the context hashes differ, the bench falls back to self-describing CBOR or refuses commands.

#### (4) Limits to expect, stated plainly: what will NOT work over LoRa

- Full MCP sessions: no tool listing, schemas, resources, streaming or notifications over the radio. The agent talks MCP to the bench; the bench talks the compact protocol to the pack.
- Anything continuous or large: microphone-array audio, SDR captures or spectra, camera frames or video, thermal-array streams (one low-resolution frame on explicit request is possible on Short presets; a stream is not), oscilloscope-style waveforms, log files, firmware or model updates.
- Fast or closed-loop control: round trips are about 0.3-0.6 s on Short presets and 2-4 s on LongFast, longer with retries; nothing that needs a confirm-before-act loop should run here, and actuation is deliberately not offered.
- High-rate multimeter logging: about 1-10 readings per second sustained on Short presets, about one every 1-3 s on LongFast. The pack still records at full rate locally; the radio carries summaries.
- Guaranteed delivery or ordering: packets can be lost, duplicated or late; the bench sees gaps and fills them after sync.
- Verified data in real time: radio readings are "reported, unverified" until the full records sync over IP and the hashes match.
- Phone-app convenience on the same node: while the bench daemon holds the node's single client connection, the Meshtastic phone app or HA integration cannot also connect to that node.
- Free coexistence with the SDRs: every LoRa transmit will blind nearby SDR receivers for the burst unless power is kept low and capture is blanked.
- The public mesh as a transport: do not put this traffic on the default LongFast channel; use a private channel and slot.

#### (5) Outside-the-box ideas for remote / disconnected operation

1. Send the intent ahead, not the RPC: while on Wi-Fi the agent compiles a signed, bounded "field test plan" (a small state machine: measure X, if below Y ask the human to shade the panel, re-measure, compare). The pack executes it offline; the radio carries only step ids and results. The human hears the plan's prompts through the speaker, becoming the agent's hands.
2. Radio notarisation: the periodic beacon carries the head hash of the pack's chain. The bench timestamps and stores it, so the pack's entire offline session is committed to a second machine in near real time for 28 bytes every few minutes, long before the data itself arrives.
3. Progressive resolution on demand: every reading advertises "more detail available"; the agent can ask for min/max/mean, then a 16-point envelope, then a wavelet/delta-coded slice of a specific 10-second window. Airtime is spent only where the agent is curious.
4. The bench SDR as an independent witness: the bench's SDR array records each pack transmission's RSSI, SNR and frequency offset. That is physical-layer provenance for the reading (it really came from that radio, from that direction), a continuous calibration target for the SDR array, and, combined with the pack's GNSS track, a free propagation map of the property showing where the container and shed shadow each band.
5. Zero-code first milestone: an INA226/INA3221 on the pack's Meshtastic node sends panel voltage/current as stock power telemetry straight into Home Assistant entities. The "failing solar panel" example works on day one with no custom protocol, and the custom path replaces it later.
6. Schema-hash "static context MCP": tool definitions, units and phrases are synced over Wi-Fi and referenced over the radio by short ids under a context hash (the SCHC idea applied to MCP). A `tools/call` becomes tool id + postcard arguments in under 30 bytes, and the bench re-inflates it into a normal MCP result with provenance.
7. Burst mode by negotiation: for a one-off larger pull, both ends agree over the Meshtastic channel to hop to ShortTurbo (or, on RNode/raw firmware, an even faster setting) for a fixed number of seconds, move the object, then return. The slow robust channel is the control plane; the fast fragile one is the data plane.
8. Scatter pods as stepping stones and data mules: ESP-NOW-LR pods along the fence line relay or cache pack messages when the pack is out of sight of the bench, and the owner's phone (Meshtastic app over Bluetooth to the pack node) can act as a second display and a manual data mule.
9. Speech as an out-of-band acknowledgement channel: the pack says a short checksum word ("reading four-seven, kilo") that the human can repeat to the agent later, a human-verifiable link between what was heard in the field and what is in the store.
10. Sentinel mode: leave the pack at the far spot overnight on a slow beacon (LongFast, one summary every few minutes at milliwatt average power), then let it bulk-sync when carried back into Wi-Fi range in the morning.
11. Same boards, second personality: keep an RNode image for the Heltec boards so the pair can be reflashed to Reticulum when a job needs resumable file transfer or remote shell over the same radios, and treat "which firmware is on the radio module" as another agent-managed, logged piece of state.

### Gaps
- The message format is a proposal; byte counts are by construction, airtime figures are computed, and nothing here has been tested on hardware.
- Accessory prices in the BOM are estimates, flagged as unverified.
- Idea 7 depends on firmware support for fast preset switching without a reboot (not confirmed; Meshtastic LoRa config changes normally reboot the node [BK]).
- Idea 4 assumes the bench SDR array is running during field sessions; no source, design suggestion only.
