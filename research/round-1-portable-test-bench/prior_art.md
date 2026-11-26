# Prior art for an AI-agent-operated portable hardware test bench (state as of 2026-11-25)

Method note for the report writer: repo stars / language / license / last-push values below come from the GitHub search API queried on 2026-11-25 and are attached to each repo link. "Stale" = no push in roughly 9+ months. "Listing only" = the item appeared in a directory or search result but its page was not opened, so only its existence is asserted. Each inventory line ends with a **Borrow:** note; those notes are my recommendation (inference), not a claim made by the source.

## Which MCP servers already exist for hardware / instruments / lab gear?

### Takeaway
Nearly every instrument class in the planned payload already has at least one MCP server, but they are small single-instrument projects (mostly Python, mostly under 50 stars, mostly created in 2026) that run on the host next to a USB/LAN instrument. The only Rust ones with traction are Adancurusul's probe-rs debugger server (222 stars) and serial server (97 stars), plus the official `rmcp` Rust SDK.

### Cited Findings

**Embedded debug / flashing**
- `Adancurusul/embedded-debugger-mcp`: Rust, MIT, 222 stars, last push 2026-07-16. 24 tools (probe mgmt, target control, memory/breakpoints, flash program/erase/verify, RTT). Works with any probe-rs probe (ST-Link, J-Link, DAPLink, Black Magic Probe, FTDI) and has an experimental OpenOCD backend validated on ESP32-S3. Flash erase is off unless enabled in config. Built on RMCP 0.3.2. **Borrow:** the whole crate as the "reflash an instrument's MCU" path; its erase-disabled-by-default config gate. — [GitHub](https://github.com/Adancurusul/embedded-debugger-mcp), [README](https://raw.githubusercontent.com/Adancurusul/embedded-debugger-mcp/main/README.md)
- `Klievan/jlink-mcp`: TypeScript, MIT, 46 stars, last push 2026-10-26; covers SEGGER J-Link, OpenOCD and Black Magic. **Borrow:** tool naming for multi-backend debug. — [GitHub](https://github.com/Klievan/jlink-mcp)
- `es617/dbgprobe-mcp-server`: Python, MIT, 11 stars, last push 2026-03-14 (borderline stale). **Borrow:** little beyond comparison. — [GitHub](https://github.com/es617/dbgprobe-mcp-server)
- `AsherDLL/mcpgulator`: Python, GPL-3.0, 4 stars, last push 2026-05-28; MCP for the JTAGulator (JTAG/UART/SWD/GPIO scans, `set_voltage`, `raw_command`); no confirmation gating documented. **Borrow:** pin-discovery tool vocabulary only. — [GitHub](https://github.com/AsherDLL/mcpgulator)
- `probe-rs/probe-rs` itself: Rust, Apache-2.0, 3,026 stars, pushed 2026-11-25. **Borrow:** library for native Rust flashing/RTT. — [GitHub](https://github.com/probe-rs/probe-rs)
- `esp-rs/espflash`: Rust, Apache-2.0, 764 stars, pushed 2026-11-24. **Borrow:** Rust-native ESP32 reflash of ESP-based instruments. — [GitHub](https://github.com/esp-rs/espflash)

**Serial / UART / BLE**
- `Adancurusul/serial-mcp-server`: Rust, MIT, 97 stars, last push 2026-07-07; MCP server + CLI for serial devices with JSON macro automation. **Borrow:** base for multimeter/BMS/ELM327 serial transports. — [GitHub](https://github.com/Adancurusul/serial-mcp-server)
- `es617/serial-mcp-server` (Python, MIT, 27 stars, last push 2026-03-07) and `es617/ble-mcp-server` (Python, MIT, 20 stars, last push 2026-03-30). **Borrow:** BLE tool surface for BLE BMS/meters. — [serial](https://github.com/es617/serial-mcp-server), [ble](https://github.com/es617/ble-mcp-server)
- `AdolphNB/uart-mcp`: Python, MIT, 3 stars, last push 2025-08-27 — stale. — [GitHub](https://github.com/AdolphNB/uart-mcp)

**Logic analyzers / sigrok**
- `KenosInc/sigrok-mcp-server`: Go, MIT, 17 stars, last push 2026-10-28. 11 tools; shells out to `sigrok-cli` and parses text into JSON; only `capture_data` acquires, the rest are read-only; `check_firmware_status` tool; firmware directory mounted read-only. **Borrow:** the subprocess-wrap pattern and the read-only/acquire split; pair with a Pico running sigrok-pico. — [GitHub](https://github.com/KenosInc/sigrok-mcp-server), [listing](https://mcpservers.org/servers/kenosinc/sigrok-mcp-server)
- `daedalus/mcp-sigrok`: Python, MIT, 3 stars, single push 2026-04-17. — [GitHub](https://github.com/daedalus/mcp-sigrok)
- Saleae ships an MCP server inside Logic 2 (local HTTP endpoint `http://127.0.0.1:10530`; agent can start captures, decode, export). Price, license and tool list are not on the page; requires Saleae hardware. **Borrow:** idea of the instrument app itself hosting MCP over HTTP. — [Saleae](https://www.saleae.com/mcp)

**Oscilloscopes / SCPI / bench instruments**
- Keysight "MCP Server for Instrument Control" (vendor, 2026): VISA over USB/LAN/GPIB; commands restricted to each instrument's Instrument Definition File; plain-language preview and approval before a command reaches hardware; class-specific safety rules; full audit log; mock mode; instrument software updates with per-step approval. License/price not stated. **Borrow:** the gating pattern (allow-list from a definition file + preview + audit + mock). — [Keysight help](https://helpfiles.keysight.com/kmsic/English/keysight_mcp_for_instrument_control/Content/overview.html)
- NI "AI Extensions": MCP servers for the NI platform; the Device Drivers MCP server covers DAQmx, DMMs, scopes, SMUs, RF (400+ devices claimed); LabVIEW and TestStand MCPs marked "coming soon"; page updated 2026-11-09; price/licensing not stated. **Borrow:** nothing directly (closed, NI hardware); confirms vendors are converging on MCP. — [NI](https://ni.com/aiextensions)
- `gloveboxes/rigol-mcp` (fork of `erebusnz/rigol-mcp`, MIT): Rigol DHO814 over Ethernet/SCPI, driven from GitHub Copilot; author states not all 897 command forms are hardware-tested (2026-09-11). **Borrow:** the "prefer numeric readings over screenshots" skill guidance. — [Microsoft Tech Community](https://techcommunity.microsoft.com/blog/EducatorDeveloperBlog/connecting-github-copilot-to-my-oscilloscope/4555545)
- `MagnusJohansson/siglent-sds-mcp`: TypeScript, MIT, 13 stars, last push 2026-11-16. — [GitHub](https://github.com/MagnusJohansson/siglent-sds-mcp)
- `mp911de/siglent-scpi-mcp`: TypeScript, Apache-2.0, 1 star, 2026-09-01; SCPI over TCP with Streamable HTTP transport. — [GitHub](https://github.com/mp911de/siglent-scpi-mcp)
- `sandraschi/oscilloscope-mcp` (Python, MIT, 8 stars, pushed 2026-11-26; USB PC scopes), `masahiro-999/oscilloscope-mcp` (Python, MIT, 3 stars; Rigol DS1000Z over LAN), `colingimenez/SCPI_MCP` (Python, MIT, 2 stars, 2026-08-30; generic SCPI). — [sandraschi](https://github.com/sandraschi/oscilloscope-mcp), [masahiro-999](https://github.com/masahiro-999/oscilloscope-mcp), [SCPI_MCP](https://github.com/colingimenez/SCPI_MCP)
- Listing only (PyPI): `lecroy-mcp`, `usbtmc-lite-mcp`, `rigol-dho824-mcp`, `labmcp-scpi`. — [lecroy-mcp](https://pypi.org/project/lecroy-mcp/), [usbtmc-lite-mcp](https://pypi.org/project/usbtmc-lite-mcp/), [rigol-dho824-mcp](https://pypi.org/project/rigol-dho824-mcp/)
- `amahpour/analog-discovery-mcp-server`: Python (FastMCP + WaveForms SDK), MIT, 5 stars, last push 2026-03-25; author calls it a demonstration. The Altium write-up (2026-02-02) records a real failure: a requested "3.3 V sine" came out 6.6 Vpp because the tool treated the value as peak, and advises writing large captures to files rather than returning JSON. **Borrow:** both lessons (explicit units in tool schemas; captures as file handles). — [GitHub](https://github.com/amahpour/analog-discovery-mcp-server), [Altium](https://resources.altium.com/p/building-mcp-server-analog-discovery)
- `sandraschi/multimeter-mcp`: Python, 0 stars, pushed 2026-11-20; USB/serial bench multimeters plus simulator. The only multimeter MCP found. **Borrow:** simulator idea. — [GitHub](https://github.com/sandraschi/multimeter-mcp)

**Power / energy profiling**
- `Ultrahuman-tech/ppk2-mcp`: Python, MIT, 2 stars, last push 2026-11-19; 12 tools for the Nordic PPK2. Two other PPK2 MCPs exist with 0 stars. — [GitHub](https://github.com/Ultrahuman-tech/ppk2-mcp), [zhixuan2333](https://github.com/zhixuan2333/PPK2-MCP), [schuschu](https://github.com/schuschu/ppk2-mcp)
- `juanqui/joulescope-mcp`: Python, 2 stars, single push 2026-05-16. — [GitHub](https://github.com/juanqui/joulescope-mcp)
- `B83C/adalm2000-mcp`: Python, 0 stars, 2026-07. — [GitHub](https://github.com/B83C/adalm2000-mcp)

**Bus Pirate / Jumperless / Flipper**
- `mplogas/buspirate-mcp`: Python, MIT, 2 stars, last push 2026-06-24. Bus Pirate 6 only (BPIO2 binary mode). 33 tools. Three tiers enforced at the server boundary: read-only, allowed-write, approval-write; `set_voltage`, `set_power`, `spi_write`, `i2c_write` block until a `_confirmed` parameter is supplied. **Borrow:** the three-tier classification (not the model-settable flag — see safety section). — [GitHub](https://github.com/mplogas/buspirate-mcp), [Glama](https://glama.ai/mcp/servers/mplogas/buspirate-mcp)
- `LesbianVelociraptor/jumperless-mcp`: Rust, MIT, 3 stars, last push 2026-05-12; bridges MCP clients to the Jumperless V5. **Borrow:** a second Rust MCP reference; agent-rewired breadboard as the bench's routing matrix. — [GitHub](https://github.com/LesbianVelociraptor/jumperless-mcp)
- `xunholy/promptzero`: Go, AGPL-3.0, 267 stars, last push 2026-11-04; "AI operated Flipper Zero". — [GitHub](https://github.com/xunholy/promptzero)
- `busse/flipperzero-mcp`: Python, MIT, 38 stars, last push 2025-12-29 — stale. `pogorelov-labs/flipper-ble-mcp` (Python, MIT, 1 star, 2026-06) drives a Flipper wirelessly over BLE. `jonastbrg/FlipperAgent` (Python, MIT, 10 stars, 2026-03). — [busse](https://github.com/busse/flipperzero-mcp), [flipper-ble](https://github.com/pogorelov-labs/flipper-ble-mcp), [FlipperAgent](https://github.com/jonastbrg/FlipperAgent)

**SDR / RF**
- `N-Erickson/AetherLink-SDR-MCP`: Python, MIT, 33 stars, last push 2026-11-10; v0.2.0 on PyPI 2026-11-04. RTL-SDR (stable/tested), HackRF RX+TX (declared "not yet verified on hardware"). Decoders: ADS-B (dump1090), POCSAG, AIS, Meteor LRPT (SatDump), ISM via rtl_433; spectrum scan, IQ/WAV record. Has HackRF transmit tools with no documented TX limits or confirmation. Tool count is internally inconsistent (34 / 33 / 28). **Borrow:** decoder set and tool names; do not copy its ungated TX. — [PyPI](https://pypi.org/project/aetherlink/), [GitHub](https://github.com/N-Erickson/AetherLink-SDR-MCP)
- `sandraschi/sdr-mcp`: Python, MIT, 10 stars, last push 2026-10-24; RTL-SDR spectrum/waterfall/FM/GNU Radio demod. — [GitHub](https://github.com/sandraschi/sdr-mcp)
- `Pranav-d33/gnuradio-mcp-server`: Python, MIT, 2 stars, 2026-06. — [GitHub](https://github.com/Pranav-d33/gnuradio-mcp-server)
- `meshtastic/meshtastic-mcp` (official org): Python, GPL-3.0, 23 stars, pushed 2026-11-25; MCP + agent skills to discover, drive, observe and test Meshtastic devices. **Borrow:** closest "agent tests and observes radio devices" precedent from a hardware vendor org. — [GitHub](https://github.com/meshtastic/meshtastic-mcp)
- Listing only: `bluespy-mcp` (blueSPY BLE protocol analyzer). — [PyPI](https://pypi.org/project/bluespy-mcp/)

**OBD-II / CAN**
- `farzadnadiri/Peykan`: Python, MIT, 26 stars, last push 2026-10-09 ("Talk to your car using AI"). — [GitHub](https://github.com/farzadnadiri/Peykan)
- `petrpatek/obd2-mcp-server`: Python, no license, 16 stars, single push 2026-05-16; ELM327 Bluetooth adapter. — [GitHub](https://github.com/petrpatek/obd2-mcp-server), [write-up](https://dev.to/petr_patek_12/diy-ai-car-diagnostics-with-a-15-bluetooth-adapter-and-python-3fhk)
- `janspoerer/mcp_obd_ii`: Python, no license, 2 stars, 2025-10-18 — stale. — [GitHub](https://github.com/janspoerer/mcp_obd_ii)
- `kruslim/canopy`: Python, 1 star, 2026-07. — [GitHub](https://github.com/kruslim/canopy)
- **Borrow (all):** PID/DTC tool vocabulary; none is Rust or licensed-and-active enough to depend on.

**Solar / BMS / Modbus**
- `lubosstrejcek/victron-tcp`: TypeScript, MIT, 7 stars, last push 2026-11-02. 32 tools + 23 prompts, 900+ registers, Modbus TCP and MQTT on LAN, read-only (MQTT writes "on roadmap"). Sibling `victron-vrm-mcp` (cloud, 52 tools) can write but destructive calls need a `confirm` flag. **Borrow:** register-map-as-named-tools approach. — [Victron Community](https://community.victronenergy.com/t/connect-your-victron-system-to-claude-ai-via-local-modbus-tcp-new-mcp-server/52999), [GitHub](https://github.com/lubosstrejcek/victron-tcp)
- Generic Modbus MCPs: `ezhuk/modbus-mcp` (Python, MIT, 4 stars, pushed 2026-11-13); `kukapay/modbus-mcp` (26 stars, last push 2025-05 — stale); `KerberosClaw/kc_modbus_mcp` (read/write PLC registers by name). — [ezhuk](https://github.com/ezhuk/modbus-mcp), [kukapay](https://github.com/kukapay/modbus-mcp), [kc_modbus_mcp](https://github.com/KerberosClaw/kc_modbus_mcp)
- Non-MCP BMS code: `syssi/esphome-jbd-bms` (C++, Apache-2.0, 267 stars, pushed 2026-11-21); `fl4p/batmon-ha` (Python, MIT, 537 stars, pushed 2026-11-24; JK/JBD/Daly/ANT/SOK over BLE). **Borrow:** BMS protocol decoders. — [esphome-jbd-bms](https://github.com/syssi/esphome-jbd-bms), [batmon-ha](https://github.com/fl4p/batmon-ha)

**KVM / Home Assistant / ESP**
- `KultivatorConsulting/pikvm_mcp_server`: TypeScript, GPL-3.0, 6 stars, last push 2026-11-17. — [GitHub](https://github.com/KultivatorConsulting/pikvm_mcp_server)
- `DustinTrap/kvm-pilot`: Python, Apache-2.0, 20 stars, pushed 2026-11-16; write-capable multi-plane (KVM + BMC + SSH). `shvartzj1/jetkvm-mcp`: Python, MIT, 28 stars, 2026-09. — [kvm-pilot](https://github.com/DustinTrap/kvm-pilot), [jetkvm-mcp](https://github.com/shvartzj1/jetkvm-mcp)
- `homeassistant-ai/ha-mcp`: Python, MIT, 8,128 stars, pushed 2026-11-26 — by far the largest hardware-adjacent MCP project. — [GitHub](https://github.com/homeassistant-ai/ha-mcp)
- ESPHome MCPs: `loryanstrant/ESPHome-MCP` (Python, MIT, 39 stars, 2026-10); `jrigling/esphome-mcp-integration` exposes "a full ESPHome development cycle to AI agents" (2 stars). **Borrow:** agent-driven compile/flash loop for ESP-based sensor nodes. — [ESPHome-MCP](https://github.com/loryanstrant/ESPHome-MCP), [esphome-mcp-integration](https://github.com/jrigling/esphome-mcp-integration)
- On-device ESP32 MCP: `ertgtct/mcpesp` (C++ library, 5 stars, last push 2025-11 — stale); `mcp-everywhere/esp32-ble-mcp-server` (1 star). — [mcpesp](https://github.com/ertgtct/mcpesp), [esp32-ble](https://github.com/mcp-everywhere/esp32-ble-mcp-server)

**SDKs / lists**
- `modelcontextprotocol/rust-sdk` (rmcp): Rust, 4,146 stars, pushed 2026-11-25. **Borrow:** the server framework. — [GitHub](https://github.com/modelcontextprotocol/rust-sdk)
- `punkpeye/awesome-mcp-servers`: 98,515 stars, pushed 2026-11-25. — [GitHub](https://github.com/punkpeye/awesome-mcp-servers)

### Inferences
- The ecosystem is wide but thin: one maintainer per instrument, Python/FastMCP by default, stdio transport on the host. A Rust, device-hosted, multi-instrument server would be new packaging rather than new capability for any single instrument.
- Realistic "depend on it" candidates are few: `embedded-debugger-mcp`, `serial-mcp-server` (both Rust/MIT), `sigrok-cli` (wrapped, not the MCP layer), `rtl_433`, probe-rs, espflash. Everything else is better treated as a reference for tool naming and failure modes.
- Vendor entries (Keysight, NI, Saleae) arriving in 2026 mean the "agent talks to instruments over MCP" idea is no longer novel by itself.

### Gaps
- GitHub keyword searches for an MCP server for Glasgow, Kismet, SocketCAN, rtl_433 (standalone), LabJack, Red Pitaya (public repo), a BLE BMS, or Raspberry Pi GPIO returned nothing relevant. That is absence from keyword search on one day, not proof none exist.
- No MCP server found for a microphone array / acoustic localization or a 360 camera; not searched in depth.
- Official MCP Registry, Glama and PulseMCP were not crawled systematically; items came from web search and GitHub search.
- Whether `smittix/intercept` exposes MCP was not checked.
- Listing-only items (lecroy-mcp, usbtmc-lite-mcp, labmcp-scpi, bluespy-mcp, "Digilent WaveForms MCP Server", Daqifi.Mcp) were not opened.

## Which open hardware multi-tools are closest in spirit; prices; agent-friendliness?

### Takeaway
The closest single device is BugBuster (open-hardware ESP32-S3 bench tool designed for AI control, 150 MCP tools, tool-layer safety, OTA of all its MCUs), but it is not for sale, AGPL-3.0, and self-described as an unreleased checkpoint. Among purchasable tools, Bus Pirate 5/6 and Flipper Zero already have agent bridges; Glasgow, Tigard and HydraBus have none that I found.

### Cited Findings
- **BugBuster** (`lollokara/BugBuster`): C firmware, AGPL-3.0 (commercial use needs a separate license), 191 stars, pushed 2026-11-23, created 2026-03-25. ESP32-S3 mainboard; 4-ch 24-bit ADC (V/I/R), 4-ch 16-bit DAC, waveform generator, 32-switch MUX matrix, level-shifted digital I/O, USB-PD input 5–20 V, adjustable 3–15 V rails with per-rail current monitoring; RP2040 logic-analyzer HAT (fork of raspberrypi/debugprobe, CMSIS-DAP SWD); a Power Profiler HAT (ESP32-P4 + C6). Python MCP server with 150 tools in 19 groups (Hackster's earlier article said 59). Transports: USB CDC binary protocol (COBS + CRC-16) and Wi-Fi HTTP REST (HTTP cannot stream ADC/LA data). Tauri v2 + Leptos (Rust/WASM) desktop app. All four MCUs OTA-update from a GitHub release with A/B rollback and SHA-256 verification. No price or BOM cost published; HAT PCBs were sponsored by JLCPCB; Altium files in repo. README calls the current state "an unreleased implementation checkpoint" with hardware acceptance pending. **Borrow:** safety model (below), OTA-with-rollback design, COBS/CRC framed protocol, MUX-matrix idea; AGPL means borrow ideas or accept AGPL. — [GitHub](https://github.com/lollokara/BugBuster), [Hackster](https://www.hackster.io/news/bugbuster-gives-ai-the-tools-to-debug-physical-hardware-f4fd22236ce4), [OSRTOS](https://osrtos.com/projects/bugbuster)
- **Glasgow Interface Explorer**: software repo Python, 0BSD, 2,245 stars, pushed 2026-11-25. iCE40HX8K FPGA, 16 I/O in two banks, each bank 1.8–5 V programmable at up to 150 mA. Crowd Supply revC page says "limited items in stock"; last campaign update 2024-06-24. Aluminum case alone is $49.00 at Mouser (48 in stock). No MCP server found. **Borrow:** applet architecture as a model for pluggable instrument drivers; use as the bench's reconfigurable digital front end. — [GitHub](https://github.com/GlasgowEmbedded/glasgow), [Crowd Supply](https://www.crowdsupply.com/1bitsquared/glasgow), [revC updates](https://www.crowdsupply.com/1bitsquared/glasgow-interface-explorer-revc/updates), [Mouser](https://www.mouser.com/en/c/embedded-solutions/computing/?m=1BitSquared)
- **Bus Pirate 5 / 5XL / 6**: launch prices with enclosure were $42.50 / $63.50 / $82.50 (Aug 2024, DirtyPCBs). All have 8 I/O at 1.2–5 V and a 1–5 V programmable supply with 0–500 mA sense and programmable fuse; only the 6 has the 8-pin look-behind buffer. US sellers named by the vendor: Blinkinlabs, Adafruit, Hacker Warehouse. Firmware: C, MIT, 278 stars, pushed 2026-11-20. Agent bridge: `mplogas/buspirate-mcp` (BP6 only). **Borrow:** BP6 as the low-voltage bus/PSU front end; its programmable fuse as a hardware interlock under the software gate. — [CNX Software](https://www.cnx-software.com/2024/08/16/bus-pirate-5xl-and-6-hardware-debugging-tools-utilize-raspberry-pi-rp2350a-and-rp2350b-microcontrollers/), [buspirate.com/get](https://buspirate.com/get/), [firmware](https://github.com/DangerousPrototypes/BusPirate5-firmware)
- **Flipper Zero**: $169.00 on the official store. Firmware GPL-3.0, 16,997 stars, pushed 2026-11-21. Several MCP bridges (USB, Wi-Fi, BLE). **Borrow:** sub-GHz/NFC/IR coverage if wanted; BLE-MCP pattern for a cable-free instrument. — [flipper.net](https://flipper.net/products/flipper-zero), [firmware](https://github.com/flipperdevices/flipperzero-firmware)
- **Jumperless V5**: $349 with free worldwide shipping (Crowd Supply); RP2350B; four programmable ±8 V supplies; on-board Python interpreter on the second core; includes a logic analyzer. Firmware repo 336 stars, last push 2026-09-25. Rust MCP bridge exists (3 stars). **Borrow:** software-defined wiring concept; too expensive for a $1,000 total budget. — [Crowd Supply](https://www.crowdsupply.com/architeuthis-flux/jumperless-v5), [GitHub](https://github.com/Architeuthis-Flux/JumperlessV5)
- **Tigard**: $49, limited stock (Crowd Supply). FT2232H; UART, SPI/JTAG, I2C, SWD; level shifting 1.6–5.5 V. Repo 908 stars, pushed 2026-10-08. No MCP found. **Borrow:** cheap, driverless FTDI path usable from OpenOCD/flashrom. — [Crowd Supply](https://www.crowdsupply.com/securinghw/tigard), [GitHub](https://github.com/tigard-tools/tigard)
- **HydraBus**: hydrafw Apache-2.0, 455 stars, last push 2026-01-30 (slowing). No MCP found. — [GitHub](https://github.com/hydrabus/hydrafw)
- **Black Magic Probe**: the `blackmagic-debug/blackmagic` GitHub repo (3,717 stars, GPL-3.0) is flagged ARCHIVED with last push 2026-03-07. Still usable through probe-rs (`embedded-debugger-mcp` lists it) and `jlink-mcp`. — [GitHub](https://github.com/blackmagic-debug/blackmagic), [embedded-debugger-mcp README](https://raw.githubusercontent.com/Adancurusul/embedded-debugger-mcp/main/README.md)
- **Raspberry Pi Debug Probe**: USB-to-SWD and USB-to-UART bridge, CMSIS-DAP compatible. Firmware repo 1,298 stars, pushed 2026-10-02; BugBuster's LA HAT is a fork of it. **Borrow:** cheapest probe-rs-compatible flasher to build into the case. — [Raspberry Pi](https://www.raspberrypi.com/products/debug-probe/), [GitHub](https://github.com/raspberrypi/debugprobe)
- **Pico as sigrok logic analyzer** (`pico-coder/sigrok-pico`): C, GPL-3.0, 1,131 stars, last push 2026-08-07; Pico/Pico 2 as logic analyzer and oscilloscope for sigrok. **Borrow:** near-free LA + scope channel, driven via sigrok-cli. — [GitHub](https://github.com/pico-coder/sigrok-pico)
- **Analog Discovery 3**: $379.00; 2 differential 14-bit scope channels, 125 MS/s, ±25 V; 2-ch AWG ±5 V; 16 digital I/O; ±0.5–5 V supplies. MCP: `amahpour/analog-discovery-mcp-server` (demo quality). — [Digilent](https://digilent.com/shop/analog-discovery-3/)
- **Red Pitaya STEMlab 125-14**: $489.70 at DigiKey (36 in stock). A custom Red Pitaya MCP server by Pablo Trujillo (Jul–Aug 2026) wraps SCPI as tools, used with STEMlab PRO Gen 2 for agent-driven FPGA verification; no repo link on the Red Pitaya page. — [DigiKey](https://www.digikey.com/en/products/detail/red-pitaya-d-d/IZD0007/6579245), [Red Pitaya blog](https://content.redpitaya.com/blog/ai-agent-fpga-verification-red-pitaya-mcp)
- **ADALM2000**: 2-ch 12-bit 100 MSPS scope, 2-ch 150 MSPS AWG, 16-ch 100 MS/s logic analyzer + pattern generator, 0 to ±5 V programmable supplies. One 0-star MCP exists. — [Analog Devices](https://www.analog.com/en/resources/evaluation-hardware-and-software/evaluation-boards-kits/adalm2000.html)
- **Joulescope JS220**: $999.00, sold out and discontinued; replaced by the JS320. Out of budget. — [Joulescope](https://www.joulescope.com/products/js220-joulescope-precision-energy-analyzer)
- **Nordic PPK2**: $107.30 at DigiKey, 1,331 in stock. MCP: `Ultrahuman-tech/ppk2-mcp`. **Borrow:** the budget current-profiling instrument with an existing agent bridge. — [DigiKey](https://www.digikey.com/en/products/detail/nordic-semiconductor-asa/NRF-PPK2/13557476)
- **HackRF** (context): HackRF One roughly $220–250 and HackRF Pro about $400 per a 2026 buyer's guide (aggregator; EU-reseller-based, indicative). — [guide](https://pasqualepillitteri.it/en/news/9558/hardware-hacking-gadgets-2026-guide)

### Inferences
- A budget-fit instrument core that reuses existing agent bridges: Bus Pirate 6 (~$83) + PPK2 (~$107) + Pi Debug Probe + Pico running sigrok-pico + RTL-SDR, leaving most of $1,000 for compute, battery, mic array, camera and case. Jumperless, Analog Discovery, Red Pitaya and Joulescope each consume a third to all of the budget.
- "Agent-friendliness" today mostly means "has a scriptable host protocol" (BPIO2, Glasgow applets/CLI, sigrok-cli, probe-rs, WaveForms SDK). No purchasable multi-tool exposes its own firmware version, health and calibration as agent tools; BugBuster is the nearest, via OTA-from-release.
- BugBuster is the project to study most closely and possibly collaborate with, but AGPL-3.0 and its unreleased status make it a design reference rather than a dependency.

### Gaps
- Current (Nov 2026) US street prices not confirmed for: Glasgow revC3 board, Bus Pirate 5/5XL/6 (only 2024 launch prices sourced), ADALM2000, LabJack (any model), HydraBus, Black Magic Probe, Raspberry Pi Debug Probe, Joulescope JS320. Vendor pages either hid prices behind JavaScript or were not fetched.
- LabJack: no price, spec or MCP data gathered.
- Why the Black Magic GitHub repo is archived (moved host vs. ended) was not verified.
- BugBuster BOM cost is unpublished; no third-party build reports found.

## Existing portable field-diagnostic kits / cyberdecks combining SDR + sensors + Pi, with reusable enclosure designs

### Takeaway
Plenty of Pelican-case Pi + SDR cyberdecks exist and a few publish printable panels, but they are human-operated terminals (screen + keyboard). None found is headless, agent-operated, or carries measurement instruments beyond radios.

### Cited Findings
- **Pelican-Deck** (Jake Simek): Pelican 1150, Raspberry Pi 4B + 7" display, 3D-printed face plate with fan vents, Ethernet/USB/HDMI routed to weatherproof external connectors, USB battery packs; print files on GitHub under GPL-3. **Borrow:** face-plate approach and weatherproof pass-through connector choice. — [Hackster](https://www.hackster.io/news/jake-simek-s-pelican-deck-is-an-open-source-3d-printed-framework-for-a-pelican-housed-raspberry-pi-0f820ccddd25)
- **cyzoonic "Cyberdeck1"** (2022): Pelican-type case, Pi 3B/3B+, RTL-SDR, an ESP32 handling keypad/buttons/pots; custom brackets so panels mount without drilling the case; parts list at github.com/CyZooNiC/cyberdeck. **Borrow:** no-penetration bracket mounting to keep the case sealed; MCU-as-panel-I/O split. — [Hackaday](https://hackaday.com/2022/03/12/rugged-cyberdeck-makes-the-case-for-keeping-things-water-tight), [Hackaday.io](https://hackaday.io/project/183892-cyberdeck1)
- **"Emergency IT" cyberdeck** (modding_me_softly): Pelican 1400, Pi 4 8GB plus a cold-spare Pi, HackRF One, GPS, Arduino Mega, two Anker packs for a claimed 48 h, Velcro mounting and deliberately no printed parts for field-swappability. **Borrow:** cold-spare compute and tool-free mounting. — [Hackster](https://www.hackster.io/news/emergency-it-cyberdeck-packs-sdr-wikipedia-48-hour-batteries-and-dual-redundant-raspberry-pis-05e0379c8c26)
- **SIGINT SDR Commander** (Hackaday.io, Aug 2024): Beelink N95 + Pi 4 + Pi 3; HackRF, KrakenRF (direction finding), AntSDR (passive radar), Ubertooth, LoRa/Zigbee modules; DragonOS; 42 V 500 Wh battery with DC-DC converters; ~6 kg; no CAD published, cost not stated. **Borrow:** power-distribution layout (one pack, per-rail converters); shows that a multi-SDR array outgrows a Pi and a $1,000 budget. — [Hackaday.io](https://hackaday.io/project/197414-sigint-sdr-commander)
- Listing only: "Raspberry Pi SDR Cyberdeck" (Hackaday.io 174301); Core Electronics "Raspberry Pi Cyberdeck (SDR Edition)"; MakerWorld "Modular Cyberdeck & Panel Generator – Apache 1800" (page fetch failed); Printables "My Cyberdeck" (model 1130214). — [Hackaday.io](https://hackaday.io/project/174301-raspberry-pi-sdr-cyberdeck), [Core Electronics](https://core-electronics.com.au/projects/raspberry-pi-cyberdeck-sdr/), [MakerWorld](https://makerworld.com/en/models/2735485-modular-cyberdeck-panel-generator-apache-1800), [Printables](https://www.printables.com/model/1130214-my-cyberdeck)
- Software stacks these rigs run: Kismet (C++, 2,276 stars, pushed 2026-11-25); Pwnagotchi maintained fork `jayofelony/pwnagotchi` (3,032 stars, last push 2026-09-28); `smittix/intercept` (Python, Apache-2.0, 2,486 stars, pushed 2026-11-24, a platform uniting SIGINT tools); `merbanan/rtl_433` (C, GPL-2.0, 7,956 stars, pushed 2026-11-25). **Borrow:** Kismet's REST/JSON capture pipeline and rtl_433 JSON output as ready-made RF telemetry sources; iNTERCEPT for decoder orchestration ideas. — [Kismet](https://github.com/kismetwireless/kismet), [pwnagotchi](https://github.com/jayofelony/pwnagotchi), [intercept](https://github.com/smittix/intercept), [rtl_433](https://github.com/merbanan/rtl_433)
- Acoustic array building blocks: `introlab/odas` (C, MIT, 1,098 stars, last push 2026-10-26; sound localization/tracking); `respeaker/usb_4_mic_array` (Apache-2.0, 206 stars, last push 2025-08 — stale; VAD/DOA/beamforming). **Borrow:** ODAS for direction-of-arrival. — [odas](https://github.com/introlab/odas), [respeaker](https://github.com/respeaker/usb_4_mic_array)

### Inferences
- Mechanical prior art is reusable (Pelican 1150/1400 or Harbor Freight Apache shells, printed face plates, sealed pass-throughs, bracket mounting). A headless sensor body can drop the screen and keyboard that dominate these designs and spend the volume on instruments and battery.
- Existing decks treat the radio as the payload and the human as the operator; the proposed project inverts both.

### Gaps
- Gridfinity-based case inserts and Printables/Thingiverse Pelican insert models were not researched; the one relevant MakerWorld page could not be fetched.
- DragonOS project status and Pi support in 2026 not verified.
- No sourced cost totals for any deck.
- No search done for commercial field kits (e.g., automotive or solar installer diagnostic cases).

## Commercial or research "LLM-operated lab bench / AI electronics debugging" products (2025–2026)

### Takeaway
2026 is the year test-and-measurement vendors and startups shipped agent interfaces (Keysight, NI, Saleae, SweepMe!, Liquid Instruments, BootLoop), all bench- or CI-bound and tied to their own hardware or software; public pricing was not found for any of them.

### Cited Findings
- **Keysight MCP Server for Instrument Control** (2026): natural-language to SCPI over VISA for Keysight analyzers, generators, scopes, supplies; approval preview, IDF allow-list, audit log, mock mode. Price/license not stated. — [Keysight](https://helpfiles.keysight.com/kmsic/English/keysight_mcp_for_instrument_control/Content/overview.html)
- **NI AI Extensions** (page updated 2026-11-09): MCP servers; Device Drivers server available (400+ devices); LabVIEW and TestStand servers "coming soon". Price not stated. — [NI](https://ni.com/aiextensions)
- **Saleae Logic 2 MCP**: built-in local MCP endpoint for coding agents to capture/decode/export. — [Saleae](https://www.saleae.com/mcp)
- **SweepMe! Autonomous Instrument Control** (announced 2026-09-18): agent connects to the bench over MCP; SweepMe! is the host that owns drivers, ports and safety rules; limits are set in the application rather than in a prompt; every command and value logged; aimed at ASIC/board bring-up; waiting list only, pricing not public. — [Silicon Saxony](https://silicon-saxony.de/en/sweepme-waiting-list-opens-for-autonomous-instrument-control/)
- **Liquid Instruments GenInst Studio** (2026-07-15): agentic AI turns plain-language requests into custom instruments deployed on Moku FPGA hardware; for existing Moku customers; price not stated. — [IT Brief](https://itbrief.news/story/liquid-instruments-launches-ai-tool-for-custom-test-gear)
- **BootLoop Test** (YC-backed, launched 2026-06-01): hardware-in-the-loop framework whose AI agent ingests PCB design files and datasheets, generates register-level tests and runs them on real hardware in CI; ITAR compliance mentioned; no MCP, hardware spec, price or open-source status stated. — [All About Circuits](https://www.allaboutcircuits.com/news/bootloop-unveils-tool-enabling-ai-powered-hardware-in-the-loop-testing/)
- **Red Pitaya + custom MCP** (Aug 2026): third-party engineer's agent generates an FPGA filter, builds it on a remote Vivado server, programs the FPGA and verifies with the Red Pitaya as both generator and scope. — [Red Pitaya](https://content.redpitaya.com/blog/ai-agent-fpga-verification-red-pitaya-mcp)
- **AutoProber** (`GainSec/AutoProber`, Apr 2026): Python, non-standard license, 345 stars, single push 2026-04-16. Agent-driven flying probe from a Genmitsu 3018 CNC, USB microscope and Siglent SDS1104X-E; agent maps the board and proposes probe points, a human approves each on a web dashboard; independent optical end stop monitored on scope channel 4; halts on ambiguous voltage/alarm/limit and never auto-resumes. **Borrow:** independent safety channel + latched stop + human approval of physical contact. — [GIGAZINE](https://gigazine.net/gsc_news/en/20260424-autoprober-diy-flying-probe/), [GitHub](https://github.com/GainSec/AutoProber)
- **BugBuster** (open hardware, see previous section) is the open counterpart to these products. — [GitHub](https://github.com/lollokara/BugBuster)
- **Research:** "Schema-Bound LLM Control of Scientific Instrumentation through MCP Skills" (dos Reis & Dravid, arXiv 2607.17012, 2026-07-19): open-source reference server with 30 typed tools, 5 live-job types, 6 skills, run against a simulator; 120 hardware-independent tests pass; local-LLM integration tests pass 12–15 of 15 per run. — [arXiv](https://arxiv.org/abs/2607.17012)
- Listing only (not opened): element14 "Control your oscilloscope with AI" webinar; TI "Smarter Hardware Debugging with CCStudio: AI + Logic Analyzer" video; Y Combinator company "movedot"; Nordic blog on AI-assisted nRF Connect SDK; Ezurio MCP Server. — [element14](https://community.element14.com/learn/events/c/e/1781), [TI](https://www.ti.com/video/6400034649112), [YC](https://www.ycombinator.com/companies/movedot), [Nordic](https://devzone.nordicsemi.com/nordic/nordic-blog/b/blog/posts/bringing-ai-assisted-development-to-nrf-connect-sdk-and-nrf-cloud), [Ezurio](https://www.ezurio.com/software/mcp)

### Inferences
- Commercial offerings expose instrument control (SCPI/driver calls) and, in Keysight's case, instrument software updates behind approval; none advertises exposing instrument health or calibration state to the agent, and none is portable or multi-domain (RF + audio + vehicle + energy).
- Every vendor keeps the safety policy in host software rather than in the prompt; that is the emerging norm to match.

### Gaps
- No public pricing found for Keysight MCP, NI AI Extensions, Saleae MCP, SweepMe!, GenInst Studio or BootLoop.
- "AI hardware engineer" startups beyond BootLoop (e.g., movedot) were not investigated.
- No peer-reviewed evaluation found of an LLM agent debugging real electronics with physical instruments; arXiv 2607.17012 runs against a simulator.

## Is there a published safety / write-gating standard or pattern for agents controlling physical instruments?

### Takeaway
No standard exists. MCP's own tool annotations are untrusted hints and say nothing about physical actuation; what exists is a set of converging vendor and project patterns (tiered tools, allow-lists, bounds checks, host-enforced approval, audit logs, hardware fuses, latched stops).

### Cited Findings
- MCP `ToolAnnotations` has four boolean hints: `readOnlyHint`, `destructiveHint`, `idempotentHint`, `openWorldHint`, with pessimistic defaults. The MCP blog (2026-03-16) states they are hints, not enforcement; a server can claim read-only and still delete; they do not stop prompt injection. Five open SEPs propose more annotations (#1913 trust/sensitivity, #1984 governance/UX, #1561 `unsafeOutputHint`, #1560 `secretHint`, #1487 `trustedHint`); none addresses physical or real-world actuation. — [MCP blog](https://blog.modelcontextprotocol.io/posts/2026-03-16-tool-annotations/)
- arXiv 2611.02527 (2026-11-03) audited 1,000 registry servers and 41 hosts/SDKs: about 90% of destructive tools have no server-side confirmation; servers that use elicitation usually proceed or fall back to a flag the model itself can set; 32 of 41 hosts have no default approval step; 60.7% of destructive tool/app pairs run with no enforced human approval; in injection tests four of five low-cost models executed an injected deletion in 479 of 960 runs. The authors recommend host-side approval configuration over reliance on servers. — [arXiv](https://arxiv.org/abs/2611.02527)
- arXiv 2607.17012 proposes a "schema-bound" tool surface: each request is validated against physical bounds before reaching the instrument adapter, and an adapter layer keeps the LLM from driving hardware directly. — [arXiv](https://arxiv.org/abs/2607.17012)
- BugBuster enforces safety in the tool layer: MUX exclusivity, automatic e-fuse arming, an 8 mA default current ceiling, confirmation for supplies above 12 V, risk gates on `mux_control` and `register_access`, rail locks, and a post-action fault check. — [GitHub](https://github.com/lollokara/BugBuster)
- buspirate-mcp: read-only / allowed-write / approval-write tiers at the server boundary; approval-write blocks until `_confirmed` is passed. — [Glama](https://glama.ai/mcp/servers/mplogas/buspirate-mcp)
- Keysight: definition-file allow-list, plain-language preview + approval, class-specific safety rules, stop-on-first-error default, full audit log, mock mode. — [Keysight](https://helpfiles.keysight.com/kmsic/English/keysight_mcp_for_instrument_control/Content/overview.html)
- SweepMe!: limits and permitted actions configured in the host application, not the prompt; all commands and values logged. — [Silicon Saxony](https://silicon-saxony.de/en/sweepme-waiting-list-opens-for-autonomous-instrument-control/)
- embedded-debugger-mcp: flash erase disabled unless enabled in config. — [README](https://raw.githubusercontent.com/Adancurusul/embedded-debugger-mcp/main/README.md)
- AutoProber: independent hardware stop channel, latched stop requiring explicit operator release, human approval of each probe target. — [GIGAZINE](https://gigazine.net/gsc_news/en/20260424-autoprober-diy-flying-probe/)
- Counter-examples: AetherLink exposes HackRF transmit tools with no documented limits or confirmation; mcpgulator exposes `set_voltage` and `raw_command` with no gating documented. — [AetherLink](https://pypi.org/project/aetherlink/), [mcpgulator](https://github.com/AsherDLL/mcpgulator)
- Practitioner cautions: check probe attenuation, voltage limits and grounding because the assistant cannot verify physical wiring (Rigol/Copilot post); a units ambiguity doubled an output amplitude (Altium/Analog Discovery). — [Microsoft Tech Community](https://techcommunity.microsoft.com/blog/EducatorDeveloperBlog/connecting-github-copilot-to-my-oscilloscope/4555545), [Altium](https://resources.altium.com/p/building-mcp-server-analog-discovery)

### Inferences
- A model-settable `_confirmed` / `confirm` argument (buspirate-mcp, victron-vrm-mcp) is exactly the fallback arXiv 2611.02527 identifies as weak. For a physical bench the approval must be out of band: host-side prompt, or a physical arm switch / button on the case that the server reads.
- A defensible composite pattern, assembled from the sources above: (1) tier every tool as observe / reversible-write / hazardous-write (RF transmit, rail enable, reflash, CAN/OBD write, inverter/BMS write); (2) bounds-check arguments in the server with explicit units; (3) hardware limits underneath (programmable fuse, current ceiling, TX power cap or no TX path); (4) out-of-band human arming for hazardous tier; (5) latched fault state with no auto-resume; (6) append-only audit log; (7) simulator/mock mode; (8) reflash only with A/B rollback and hash verification.
- Publishing that tiering as a small spec for "physical-actuation MCP tools" would itself be a contribution, since the MCP annotation work has not reached physical effects.

### Gaps
- No ISO/IEC/NIST/IEEE document specific to LLM agents operating test instruments was found; general standards (e.g., IEC 61010 for measurement equipment safety, FCC rules for RF transmission, vehicle-bus write risks) were not researched and are not agent-specific.
- The MCP elicitation spec text and the content of the open SEPs were not read directly.
- No incident reports found of an agent damaging hardware through an instrument MCP server.

## What is the honest gap — what combination does not exist as an open project today?

### Takeaway
Each ingredient exists separately; the combination does not. I found no open project that is (a) portable and battery powered, (b) multi-domain (RF + acoustic + imaging + vehicle + energy + electrical), (c) serves MCP from the device over wireless, (d) exposes and maintains its own instruments' firmware, health and calibration through the agent, and (e) is written in Rust with a published write-gating model.

### Cited Findings
- Closest overall: BugBuster — agent-first, tool-layer safety, Wi-Fi + USB, OTA with rollback for its own four MCUs — but it is a bench electrical debugger only (no RF, audio, camera, OBD, BMS), its MCP server is host-side Python, it is AGPL-3.0, and it is unreleased. — [GitHub](https://github.com/lollokara/BugBuster)
- Per-instrument MCP servers are host-side and single-domain; representative examples: sigrok (Go, stdio), AetherLink SDR (Python), Peykan OBD (Python), victron-tcp (TypeScript, read-only), ppk2-mcp (Python). — [sigrok-mcp-server](https://github.com/KenosInc/sigrok-mcp-server), [AetherLink](https://github.com/N-Erickson/AetherLink-SDR-MCP), [Peykan](https://github.com/farzadnadiri/Peykan), [victron-tcp](https://github.com/lubosstrejcek/victron-tcp), [ppk2-mcp](https://github.com/Ultrahuman-tech/ppk2-mcp)
- Rust MCP servers for hardware are limited to debug probes, serial and Jumperless. — [embedded-debugger-mcp](https://github.com/Adancurusul/embedded-debugger-mcp), [serial-mcp-server](https://github.com/Adancurusul/serial-mcp-server), [jumperless-mcp](https://github.com/LesbianVelociraptor/jumperless-mcp)
- Network-served instrument MCP is rare: Saleae (localhost HTTP), mp911de/siglent-scpi-mcp (Streamable HTTP), BugBuster (Wi-Fi REST under a host MCP server). — [Saleae](https://www.saleae.com/mcp), [siglent-scpi-mcp](https://github.com/mp911de/siglent-scpi-mcp), [BugBuster](https://github.com/lollokara/BugBuster)
- Portable Pi + SDR decks are human-operated terminals with no agent interface and no measurement instruments. — [Pelican-Deck](https://www.hackster.io/news/jake-simek-s-pelican-deck-is-an-open-source-3d-printed-framework-for-a-pelican-housed-raspberry-pi-0f820ccddd25), [SIGINT SDR Commander](https://hackaday.io/project/197414-sigint-sdr-commander)
- Agent-driven reflash exists for targets (embedded-debugger-mcp flash tools; ESPHome MCP dev cycle) and for one product's own MCUs (BugBuster OTA), not for a mixed fleet of third-party instruments. — [embedded-debugger-mcp README](https://raw.githubusercontent.com/Adancurusul/embedded-debugger-mcp/main/README.md), [esphome-mcp-integration](https://github.com/jrigling/esphome-mcp-integration), [BugBuster](https://github.com/lollokara/BugBuster)
- No standard for gating physical actions exists in MCP. — [MCP blog](https://blog.modelcontextprotocol.io/posts/2026-03-16-tool-annotations/), [arXiv 2611.02527](https://arxiv.org/abs/2611.02527)

### Inferences
What nobody appears to have shipped, in descending order of novelty:
1. **Instrument introspection as a first-class agent surface** — a uniform "identity / firmware version / self-test / calibration record / reflash with rollback" tool set across heterogeneous instruments, so the agent can diagnose and repair the bench itself. Commercial tools hide this; hobby servers ignore it.
2. **A device-hosted, wireless, multi-instrument MCP server** on a battery-powered Pi-class box. Existing servers assume the agent's host has the USB cable.
3. **Cross-domain field payload** — RF array + mic array + 360 camera + OBD-II + inverter/BMS + electrical measurement in one body with shared timebase and one tool namespace. No MCP exists at all for mic arrays or 360 cameras as far as I found.
4. **Rust end to end** for the server and drivers; only three small Rust hardware MCP servers exist.
5. **An explicit, published write-gating model for physical actuation**, with out-of-band human arming, which the MCP ecosystem lacks.
6. **Open enclosure + electronics + agent software released together**; decks publish cases, MCP projects publish code, none publishes both.

Not novel, and should be borrowed rather than rebuilt: MCP-to-SCPI, MCP-to-sigrok, MCP-to-probe-rs, MCP-to-serial, MCP-to-RTL-SDR decoders, MCP-to-ELM327, MCP-to-Modbus, Pelican face-plate mechanics, tiered-tool safety vocabulary.

Risks to the "gap" claim: BugBuster is moving fast (1,265 commits since March 2026) and could add wireless-native MCP or more HATs; vendors (Keysight, NI) could expose instrument health; the hobby MCP space produces new single-instrument servers monthly.

### Gaps
- The gap statement rests on keyword searches of GitHub and the web on 2026-11-25, not an exhaustive crawl of the MCP Registry, Glama, PulseMCP, Hackaday.io, Crowd Supply or Printables. A private or unindexed project could exist.
- Non-English projects (several Chinese-language ESP32/serial MCP repos appeared in results) were not examined.
- No check was made of ROS-based or lab-automation (SiLA 2, Bluesky/EPICS) agent bridges, which may hold relevant device-introspection patterns.
