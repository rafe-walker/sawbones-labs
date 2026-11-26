# Module/plug-in standard (A) and time-synchronized multi-sensor fusion and logging (B) — state as of 25 November 2026

Research-method note for the report writer: the shared web-search budget for this turn was already exhausted when this task started, so no search-engine queries ran. Everything under "Cited Findings" comes from primary pages fetched directly during this session (spec pages, project docs, arXiv abstracts). Anything I know from prior knowledge but could not re-verify this session is placed under "Inferences" and labelled "(prior knowledge, unverified this session)" or listed under "Gaps". Verdict lines use the requested vocabulary: **adopt**, **borrow idea**, **avoid**.

---

## A1. Electrical/mechanical module ecosystems and their lessons (which use self-ID and how)

### Takeaway
The successful hobby ecosystems are tiny specs (one connector, one voltage, one pinout) with free naming rules; almost none of them self-identify, and the few that do (Raspberry Pi HAT+ ID EEPROM, mikroBUS Click ID, BeagleBone capes) store only an identity plus a pointer to a host-side description, not the description itself. That "small ID on the module, rich description resolved by the host" split is the pattern to copy; for mechanics, the Framework expansion-card model (USB-C as the only electrical interface, open CAD under CC BY) is the closest precedent for a hot-pluggable instrument bay.

### Cited Findings
- **Raspberry Pi HAT+ (spec build dated 2024-12-05):** firmware probes ID_SD/ID_SC (GPIO0/1) for a 24Cxx EEPROM whose I2C address encodes the HAT+ class (Standard = 0x50, Stackable = 0x51, Power HAT+ uses two further address suffixes for ~3 A and ~5 A modes). Required EEPROM content is a product UUID, vendor string and product string; product ID/version and custom data are optional; the EEPROM holds only the *name* of a device-tree overlay, with the overlay file living on the host filesystem. The original HAT spec is deprecated; HAT+ is less prescriptive mechanically (must mate with the 40-pin header and align at least one mounting hole) and at most three HAT+ boards (one Standard, one Stackable, one Power) can be combined. — [Raspberry Pi HAT+ specification PDF](https://pip-assets.raspberrypi.com/categories/1215-raspberry-pi-hat/documents/RP-008281-DS-1-hat-plus-specification.pdf)
- **mikroBUS Click ID:** an ID chip on the Click board is read over 1-Wire; its EEPROM has permanent data locked at production plus a configurable area holding a *manifest* (vendor/product strings, mikroBUS pin-state descriptor, device descriptor with driver name, protocol, register address and IRQ settings). Developers write a `.mnfs` file, convert it to binary and write it through the 1-Wire sysfs path; on Linux the kernel then loads the right driver automatically ("ID mode" vs "regular mode"). MIKROE's tutorials on this date from December 2023 to August 2024 and show BeaglePlay as the host. — [MIKROE ClickID page](https://www.mikroe.com/clickid)
- **Qwiic (SparkFun):** 4-pin JST connector (SM04B-SRSS-TB or equivalent), 3.3 V only, fixed pinout GND/3.3V/SDA/SCL with a fixed wire colour scheme; daisy-chaining via two connectors is encouraged but optional; I2C is short-range (SparkFun reports roughly 1 m, with a differential I2C breakout for longer runs); conservative cable current limit 226 mA. The Qwiic name can be used without royalty or attribution provided the board uses the same connector, 3.3 V, colours and pinout. SparkFun says Qwiic and STEMMA QT are not the same thing (STEMMA allows 3.3 V and 5 V) and advises against labelling a connector "Qwiic/Stemma". The page defines no self-identification mechanism. — [SparkFun Qwiic](https://www.sparkfun.com/qwiic)
- **Feather / FeatherWing (Adafruit):** 0.9 in x 2.0 in board, 16-pin + 12-pin headers 0.8 in apart whose GPIO positions must not move; 3.3 V logic throughout; fixed UART/I2C/SPI pin positions; VBUS, VBAT and an EN pin that must depower the 3.3 V rail; mainboards require LiPo charging with a JST-PH connector. Adafruit maintains the spec informally, explicitly invites third parties to build compatible boards, and defines no electronic ID — incompatibilities are to be "clearly documented". — [Adafruit Feather specification](https://learn.adafruit.com/adafruit-feather/feather-specification)
- **Framework Expansion Cards:** the repository publishes reference designs with `Mechanical` and `Electrical` folders and CAD so third parties can build cards; licensed CC BY 4.0 (© 2021 Framework Computer Inc). The built cards (USB-C, USB-A, HDMI, DP, MicroSD, storage) show the bay is just a recessed USB-C port. — [FrameworkComputer/ExpansionCards](https://github.com/FrameworkComputer/ExpansionCards)
- **Flipper Zero expansion modules:** UART on the GPIO header (USART pins 13/14 or LPUART 15/16); the module announces itself by pulling its RX line low, the host answers with a heartbeat, and the module must request a baud rate within 250 ms (link starts at 9600 baud); frames are 1-byte header + payload + XOR checksum (HEARTBEAT, STATUS, BAUD RATE, CONTROL, DATA up to 64 bytes); a CONTROL frame opens a protobuf RPC session, and a missed heartbeat drops the link so re-plugging simply re-runs the handshake. — [Flipper expansion protocol docs](https://developer.flipper.net/flipperzero/doxygen/expansion_protocol.html)
- **Gridfinity:** the community specification site documents 6 mm x 2 mm magnets and M3 screws in bin corners, calls itself "a work in progress" and directs contributors to a Discord and a GitHub repo for the site. — [gridfinity.xyz specification](https://gridfinity.xyz/specification/)
- **ESPHome / Home Assistant devices:** ESPHome (site shows version 2026.11.0) describes each sensor in YAML with `unit_of_measurement`, `device_class`, `state_class`, `accuracy_decimals` and an ordered `filters` pipeline. — [ESPHome sensor component](https://esphome.io/components/sensor/)
- **USB hub per-port power as the module-bay enabler:** `uhubctl` only works on hubs that implement per-port power switching *and* actually cut VBUS. On Raspberry Pi 5 all four USB ports are ganged into one power group (switching one back on restores all); Pi 4B works with a minimum VL805 firmware. USB 3 hubs appear as two virtual hubs (USB2 + USB3) and both must be toggled. Listed working hubs include Plugable USB3-HUB7BC and StarTech HB30A4AIB. — [uhubctl README](https://raw.githubusercontent.com/mvp/uhubctl/master/README.md)

One-line verdicts supported by the cited pages:
- Raspberry Pi HAT+ ID EEPROM — **borrow idea**: UUID + vendor/product strings + "name of host-side description" in a cheap I2C EEPROM is the right amount of on-module data; do not adopt the 40-pin form factor as the module standard because it allows only three boards and no hot-plug.
- mikroBUS / Click ID — **borrow idea**: the manifest-in-1-Wire-EEPROM concept (pin states + driver binding) is the best existing self-ID for "dumb" breakout boards; the socket itself is worth supporting through an adapter module rather than as the platform's own interface.
- Qwiic / STEMMA QT — **adopt** (as the internal sensor connector inside a module, 3.3 V only), because naming is royalty-free and the ecosystem is large; do not rely on it for identification because it has none.
- Feather / FeatherWing — **adopt** (as a recommended MCU carrier format for wireless pods), because third-party boards are invited and the pinout is frozen; it has no self-ID.
- Framework expansion cards — **borrow idea**: "the connector is the standard" (USB-C plug, open CAD, CC BY) gives hot-plug, power and identification with no custom electrical spec.
- Flipper Zero expansion protocol — **borrow idea**: heartbeat + timeout + automatic re-handshake is a simple, proven hot-plug state machine for UART-attached modules.
- Gridfinity — **adopt** (for mechanics on the fixed bench and the pack's tray), with the caveat that its spec is community-maintained and unfinished, so the project should pin the exact dimensions it uses in its own document.
- ESPHome YAML — **borrow idea**: declarative per-sensor metadata (unit, device class, state class, display precision) compiled into firmware.

### Inferences
- (prior knowledge, unverified this session) Remaining ecosystems, one line each:
  - Grove (Seeed) — **avoid** as a standard: one 4-pin connector carries analog, digital, UART or I2C at 3.3 V or 5 V depending on the board, so the connector does not tell you what is safe to plug in.
  - Pmod (Digilent) — **borrow idea**: its spec defines typed interfaces (SPI/I2C/UART/GPIO variants) on 6/12-pin headers; no electronic ID.
  - MicroMod (SparkFun) — **borrow idea**: reuses a commodity connector (M.2) for a swappable processor; no ID EEPROM; overkill for sensor modules.
  - BeagleBone capes — **borrow idea**: I2C EEPROM at a small set of addresses names the cape and the bootloader/kernel applies a device-tree overlay; the same idea HAT+ later simplified.
  - Arduino shields — **avoid**: no ID, no pin-conflict management, 5 V/3.3 V split.
  - M5Stack units and Seeed XIAO — **borrow idea**: cheap, enclosed, single-purpose modules are what hobbyists actually buy; they are good *inside* a module, not as the standard.
  - Eurorack (Doepfer A-100) — **borrow idea**: a shared power bus plus rail mounting created a multi-vendor ecosystem with almost no specification; the lesson to avoid is its unkeyed power header (reversed cables destroy modules), so any project power connector must be keyed and reverse-protected.
  - DIN-rail modules — **borrow idea** for the fixed bench (35 mm rail is the norm for inverter/BMS/RS485 gear); not for the portable pack.
  - PC/104 and CubeSat PC/104 kits — **avoid**: stack-through bus connectors are costly and every board must pass every signal.
  - Pocket Science Lab (FOSSASIA) — **borrow idea**: an open multi-instrument USB board is a candidate *module*, not a module standard.
  - Sensor Watch, modular cyberdeck conventions, tinyGo/Wio — **borrow idea** at most; I know of no formal, versioned module spec from these communities.
  - Home Assistant/ESPHome devices — **adopt** as an interop target (publish module readings to Home Assistant on the fixed bench), not as the module contract.
- Gridfinity's commonly cited grid is 42 mm x 42 mm with 7 mm height units (prior knowledge, unverified this session; the fetched spec page did not state it). On that basis a 4 x 4 baseplate is 168 mm and fits a 180 mm bed, while 5 x 5 (210 mm) does not; larger trays must be printed in interlocking sections.
- Because the Pi 5's own ports are ganged, "agent can power-cycle one module" requires an external hub with true per-port VBUS switching; this is a bill-of-materials decision, not a software one.
- The self-ID mechanisms in use across these ecosystems reduce to four: (1) I2C EEPROM at a reserved address (HAT+, capes), (2) 1-Wire EEPROM with a manifest (Click ID), (3) the bus's own enumeration (USB descriptors, as with Framework cards), (4) a handshake spoken by a microcontroller on the module (Flipper). A new standard should define all four as alternative "identity carriers" that resolve to the same manifest.

### Gaps
- Exact Framework expansion-card dimensions and any electrical requirements were not on the repository landing page; they are in the `Mechanical`/`Electrical` folders, which I did not open.
- Which mikroBUS pin carries the Click ID 1-Wire signal, and the state of mainline-Linux mikroBUS/Greybus manifest support, were not stated on the page fetched.
- I could not verify current specs for Grove, Pmod, MicroMod, M5Stack, XIAO, Eurorack, PC/104, Pocket Science Lab, Sensor Watch or cyberdeck conventions this session (no search budget); verdicts for those are from prior knowledge.
- No source found for an existing open "USB-C instrument module bay" standard for hobby test gear other than Framework's laptop cards.

---

## A2. Field buses for modules: hot-plug + power + identification with the least custom work

### Takeaway
USB is the only bus that gives hot-plug, 5 V power, enumeration and a firmware-update class for free on a Raspberry-Pi-class host, and a free open-source PID is obtainable from pid.codes; CAN (DroneCAN/OpenCyphal) is the best-documented model of what a *self-describing node* must offer (health heartbeat, GetInfo with unique ID, dynamic node-ID allocation, parameters, firmware update, time sync) and should be copied as a service list even where CAN itself is not used. Wireless pods fit best behind a gateway module rather than as first-class bus citizens.

### Cited Findings
- **DroneCAN application-level functions:** every node must periodically broadcast `NodeStatus` (uptime, health OK/warning/error/critical, mode, vendor code) and is considered offline after 3000 ms of silence; `GetNodeInfo` returns status, software version, hardware version (with a 128-bit unique ID) and a reverse-DNS-style node name such as `com.manufacturer.project.product`; dynamic node-ID allocation has an allocator hand out IDs keyed by that unique ID, with allocatees sending the ID in stages via anonymous frames; firmware update is `BeginFirmwareUpdate` followed by the node pulling the image with `file.Read` 256 bytes at a time and reporting a software-update mode; `param.GetSet` reads/writes named parameters and returns the stored value plus default/min/max; `param.ExecuteOpcode` saves or erases; `RestartNode` restarts remotely. — [DroneCAN specification, section 6](https://dronecan.github.io/Specification/6._Application_level_functions/)
- **DroneCAN time sync:** a master broadcasts `GlobalTimeSync` whose payload is the exact transmit timestamp of the *previous* broadcast; slaves compare that with their own receive timestamp of the previous message to estimate phase error; master redundancy uses lowest node ID with a 2200 ms takeover timeout. — [DroneCAN specification, section 6](https://dronecan.github.io/Specification/6._Application_level_functions/)
- **OpenCyphal:** formerly UAVCAN (renamed 2022); transports are Cyphal/CAN, Cyphal/UDP and Cyphal/serial; data types are defined in DSDL with bounded variable-length fields so worst-case timing is computable; the standard service library covers diagnostics, file transfer and configuration management; the specification can be implemented without licence or approval, reference implementations are MIT-licensed (libcanard, libudpard, PyCyphal stable; libcyphal beta), and tooling includes Nunavut (DSDL transcompiler), Yakut (CLI) and Yukon (GUI). — [opencyphal.org](https://opencyphal.org/)
- **OpenCyphal regulated namespaces:** standard service namespaces exist for `uavcan.register`, `uavcan.pnp`, `uavcan.file`, `uavcan.node`, `uavcan.internet` and `uavcan.time`; `uavcan.si.unit` and `uavcan.si.sample` provide unscaled SI-unit types, the latter time-stamped. — [OpenCyphal public_regulated_data_types](https://github.com/OpenCyphal/public_regulated_data_types)
- **USB identity for open hardware:** pid.codes assigns free PIDs under VID 0x1209 by pull request to `pidcodes/pidcodes.github.com`; prerequisites are a public source repository, modifiable PCB files and/or source for a USB device, and a recognised open-source (hardware) licence with a LICENSE file; PIDs in the 0xxx and 1xxx ranges are reserved; the registry is explicitly not endorsed by USB-IF, whose own Vendor ID costs USD 6,000 according to the site. — [pid.codes](https://pid.codes/), [pid.codes how-to](https://pid.codes/howto/)
- **IO-Link (IEC 61131-9) as a powered point-to-point sensor bus:** 3- or 5-conductor unshielded cable up to 20 m, 24 V at up to 200 mA from the master on class A ports, 4.8/38.4/230.4 kbaud, device woken from plain switching mode into IO-Link mode by a current pulse, and devices carry identification, parameters and diagnostics. — [Wikipedia: IO-Link](https://en.wikipedia.org/wiki/IO-Link)
- **Zenoh-pico:** a C library for constrained devices that interoperates with the Rust Zenoh implementation; supported platforms include ESP-IDF, Arduino, Zephyr 3.7+, FreeRTOS and Raspberry Pi Pico (W / 2 W); transports include UDP unicast/multicast, TCP and serial (UART; USB CDC on Pico marked experimental); client mode needs a router, peer mode uses UDP multicast; dual-licensed EPL-2.0/Apache-2.0. — [zenoh-pico README](https://raw.githubusercontent.com/eclipse-zenoh/zenoh-pico/main/README.md)
- **Qwiic/I2C limits:** I2C was designed for short on-board distances; SparkFun reports about 1 m. — [SparkFun Qwiic](https://www.sparkfun.com/qwiic)

Verdicts supported by the cited pages:
- USB (device descriptors + hub per-port power) — **adopt** as the primary wired module bus: enumeration is the identification, and pid.codes solves the VID/PID problem for open hardware.
- DroneCAN — **borrow idea** (service list: status heartbeat, GetNodeInfo, allocation by unique ID, GetSet params, BeginFirmwareUpdate, GlobalTimeSync); **avoid** adopting the wire protocol unless the project wants ArduPilot-ecosystem peripherals.
- OpenCyphal — **borrow idea** (DSDL-style bounded, versioned, SI-typed data and the `register` concept); adopt only if a CAN FD backbone is chosen for the fixed bench.
- IO-Link — **borrow idea** (wake-up from "dumb" mode to "smart" mode on the same wires, device description resolved from a registry); **avoid** the 24 V physical layer for a hobby pack.
- Zenoh / zenoh-pico — **borrow idea** / optional adopt for Wi-Fi pods on the fixed bench; not needed for USB modules.
- I2C/Qwiic as an inter-module bus — **avoid** (no ID, address conflicts, ~1 m reach); keep it inside a module.

### Inferences
- (prior knowledge, unverified this session) OpenCyphal's `uavcan.node.GetInfo` carries protocol version, hardware version, software version, a VCS revision ID, a 16-byte unique ID, a node name and an optional software-image CRC; its `uavcan.pnp` allocation and `uavcan.register.Access/List` services are the Cyphal equivalents of DroneCAN's allocation and `param.GetSet`. `canadensis` is a Rust Cyphal implementation. The fetched pages confirm the namespaces exist but not these field lists.
- (prior knowledge, unverified this session) Other buses, one line each:
  - RS485/Modbus RTU — **adopt** only as a *field* interface to inverters/BMS; it has no discovery, no hot-plug and no self-description, so a gateway module must supply the manifest.
  - 1-Wire — **adopt** narrowly as an identity carrier (ID EEPROM), not as a data bus.
  - Single Pair Ethernet 10BASE-T1S with PoDL — **avoid** for now: multidrop plus power is attractive but PHYs, magnetics and Linux support are not hobby-priced or hobby-stocked.
  - Ethernet + PoE — **adopt** for fixed-bench cameras and remote nodes (the Pi 5 needs a PoE HAT+; HAT+ defines a Power class for this); too heavy per module for the pack.
  - BLE GATT — **borrow idea**: characteristics with presentation-format descriptors (unit, exponent) are a compact self-description; use BLE for pod provisioning and low-rate data.
  - ESP-NOW — **adopt** as a pod-to-gateway link for ESP32 pods (connectionless, low latency), but it is vendor-specific, so wrap it behind the same RPC as USB.
  - Thread/Matter — **borrow idea** (device type = required set of clusters; cluster = attributes + commands + events with a global revision attribute); **avoid** implementing Matter itself: certification and commissioning overhead buys nothing for a test bench.
  - MAVLink — **avoid** as the module RPC (fixed message set oriented to vehicles); keep as a possible *instrument* (drone telemetry input).
- Ranking for "hot-plug + power + identification with least custom work" on a Pi-class host: (1) USB with a per-port-switched hub; (2) CAN FD with Cyphal/DroneCAN-style services over a keyed 4-wire power+CAN connector (needs a CAN interface and termination discipline); (3) PoE Ethernet; (4) everything else needs a gateway module.
- A pragmatic rule that keeps the standard small: **every module is a USB device or sits behind one** (a "gateway module" for RS485, CAN, K-line, ESP-NOW pods, Qwiic chains). The gateway presents child modules to the host, the same way a USB hub presents ports.

### Gaps
- Current OpenCyphal specification version/date and the exact `uavcan.pnp`/`uavcan.node.GetInfo` definitions were not on the pages fetched (the spec is a PDF I did not open).
- No verified 2026 data on 10BASE-T1S hobby hardware availability, Matter spec version, or ESP-NOW latency/throughput figures.
- USB-specific details (BOS/WebUSB/MS OS 2.0 descriptors, DFU class behaviour, serial-number string conventions) are from prior knowledge only.

---

## A3. Self-description standards: best base for an "agent-readable datasheet"

### Takeaway
W3C WoT Thing Description 1.1 (a W3C Recommendation since 5 December 2023) is the best structural base: it already models properties/actions/events with JSON-Schema data types, `unit`, `readOnly`/`writeOnly`, and per-action `safe` and `idempotent` flags, and its Thing Model gives a class-vs-instance split. It lacks metrology (uncertainty, calibration, absolute-maximum ratings), health/self-test and firmware provenance, which should be added as a namespaced extension drawing on IEEE 1451 TEDS (identity + calibration, embedded or "virtual"), Linux IIO (channel typing), Home Assistant (device/state classes) and MCP tool annotations (pessimistic-by-default risk hints).

### Cited Findings
- **WoT Thing Description 1.1:** W3C Recommendation dated 05 December 2023, superset of TD 1.0 (2020). Three affordance types: Properties (with `observable`), Actions (with `input`, `output`, `safe`, `idempotent`, `synchronous`), Events (with `subscription`, `data`, `cancellation`). Data schemas use a JSON Schema draft-7 subset including `readOnly`, `writeOnly` and `unit`. `version` has a mandatory `instance` and optional `model`, with semantic versioning recommended. Forms bind affordances to protocols (HTTP, CoAP, MQTT). A Thing Model (`tm:ThingModel`) describes a device class without communication details and supports versioning, extension/import, composition, optional affordances and placeholders. Security schemes are declared in `securityDefinitions`. — [W3C WoT Thing Description 1.1](https://www.w3.org/TR/wot-thing-description11/)
- **IEEE 1451:** a family of smart-transducer interface standards whose TEDS holds identification, calibration, correction data and manufacturer information; TEDS can be embedded (typically EEPROM in the transducer) or *virtual* (a data file accessible to the measurement system, extending TEDS to legacy sensors). Listed members: 1451.0-2007, 1451.1-1999, 1451.2-1997, 1451.3-2003, 1451.4-2004 (mixed-mode communication and TEDS formats), 1451.5-2007 (wireless), 1451.7-2010 (RFID). The standards are paywalled. — [Wikipedia: IEEE 1451](https://en.wikipedia.org/wiki/IEEE_1451)
- **IO-Link IODD:** an XML device-description file describing a device's parameters, used by engineering/master tools to configure it; the community runs an "IODDfinder" lookup so tools can fetch the right IODD for a device; devices report detailed diagnostics (example: optical sensor becoming dirty). — [Wikipedia: IO-Link](https://en.wikipedia.org/wiki/IO-Link)
- **Linux IIO channel model:** each channel is a `struct iio_chan_spec` with a `type` (voltage, temperature, intensity...), index or modifier to distinguish channels, per-channel vs shared attribute masks (e.g. RAW, PROCESSED, sampling frequency), a `scan_type` describing buffer layout (signedness, `realbits`, `storagebits`, `shift`, endianness) and event specs; sysfs names follow the pattern `in_voltage0_raw`, `in_intensity_ir_raw`, `in_illuminance_input`; buffers carry an aligned 64-bit timestamp. — [Linux kernel IIO core docs](https://docs.kernel.org/driver-api/iio/core.html)
- **Home Assistant sensor entity model:** `device_class` says what is measured and constrains units (temperature: °C/°F/K; power: mW…TW; energy: J…MWh; battery: %); `native_unit_of_measurement` must use defined constants because near-matches such as "KWh" are treated as different units; `state_class` is `measurement`, `total` or `total_increasing` and drives statistics; `suggested_display_precision` affects display without rounding stored state. — [Home Assistant developer docs: sensor entity](https://contributors.home-assistant.io/core/entities/sensor/)
- **MCP tool schema (2025-11-25 and 2026-07-28 revisions):** a tool has `name`, `title`, `description`, `inputSchema` and optional `outputSchema` (JSON Schema, 2020-12 by default), `annotations`, and `structuredContent` results; clients "MUST consider tool annotations to be untrusted unless they come from trusted servers". — [MCP tools spec 2025-11-25](https://modelcontextprotocol.io/specification/2025-11-25/server/tools), [MCP tools spec 2026-07-28](https://modelcontextprotocol.io/specification/2026-07-28/server/tools.md)
- **MCP annotation semantics and defaults:** `readOnlyHint` (default false), `destructiveHint` (default true), `idempotentHint` (default false), `openWorldHint` (default true); defaults are deliberately pessimistic; annotations can drive confirmation prompts and feed policy engines but cannot stop an untrusted server from lying or enforce anything — guarantees belong in deterministic controls. Open proposals include SEP-1913 (trust and sensitivity annotations), SEP-1984 (comprehensive tool annotations), SEP-1862 (tool resolution/preflight checks), plus `unsafeOutputHint`, `secretHint` and `trustedHint`. — [MCP blog, "Tool Annotations as Risk Vocabulary", 2026-03-16](https://blog.modelcontextprotocol.io/posts/2026-03-16-tool-annotations/), [Tool Annotations Interest Group charter, 2026-04-20](https://modelcontextprotocol.io/community/interest-groups/tool-annotations.md)
- **DroneCAN naming and parameter description:** node names are reverse-DNS strings limited to lowercase letters, digits, dots, dashes and underscores; parameter reads can return default, min and max alongside the value. — [DroneCAN specification, section 6](https://dronecan.github.io/Specification/6._Application_level_functions/)
- **OpenCyphal SI typing:** `uavcan.si.unit` types are type-safe wrappers for unscaled SI quantities; `uavcan.si.sample` adds a timestamp. — [OpenCyphal public_regulated_data_types](https://github.com/OpenCyphal/public_regulated_data_types)
- **Klipper data dictionary:** the MCU firmware embeds a zlib-compressed JSON dictionary generated at build time (commands, responses, enumerations, constants, software version); only the `identify` command and its response are hard-coded, and the host downloads the dictionary in chunks at connect. — [Klipper protocol docs](https://www.klipper3d.org/Protocol.html)
- **openDAQ (v3.40.4, November 2026):** an Apache-2.0 data-acquisition SDK in which devices publish their own structure and properties into a shared model; modules are of three kinds (device modules, protocol integrations such as OPC UA, function blocks); streamed data is timestamped and signals share a time base; written in C++ with Python/C#/C/Delphi bindings. — [opendaq.com](https://opendaq.com/)
- **sigrok session metadata:** a `.sr` file's INI-style `metadata` lists per-device `driver`, `samplerate`, `unitsize`, probe and analog channel names. — [sigrok wiki: file format v2](https://sigrok.org/wiki/File_format:Sigrok/v2)

Verdicts supported by the cited pages:
- W3C WoT Thing Description 1.1 + Thing Model — **adopt** as the manifest's structural base (affordances, `safe`/`idempotent`, `readOnly`, `unit`, model/instance versioning), as a JSON profile without requiring full JSON-LD processing.
- IEEE 1451 TEDS — **borrow idea** (identity + calibration travelling with the transducer; "virtual TEDS" for legacy/off-the-shelf devices); **avoid** adopting the binary formats because the standards are paywalled and tooling is scarce in the hobby world.
- IO-Link IODD + IODDfinder — **borrow idea**: small ID on the device, full description fetched from a registry keyed by vendor/device ID.
- Linux IIO — **borrow idea** / partial adopt: reuse its channel-type vocabulary and raw-vs-processed distinction, and consume IIO devices directly on the host where drivers exist.
- Home Assistant entity model — **borrow idea**: `device_class` + constrained units + `state_class`; also a concrete interop target.
- MCP tool annotations — **adopt** on the MCP surface, generated *from* the manifest; treat as hints and enforce write safety in the gateway.
- Klipper dictionary — **borrow idea**: firmware carries (or hashes) its own interface description, produced by the build.
- openDAQ — **borrow idea** (device/signal/function-block model with shared time domain); **avoid** as a dependency for a Rust-first project because it is a C++ SDK.
- sigrok channel model — **borrow idea**; consume sigrok as a driver for logic analyzers and multimeters rather than re-implementing it.

### Inferences
- (prior knowledge, unverified this session) Other description standards, one line each:
  - IEEE 1451.4 mixed-mode TEDS in a 1-Wire EEPROM with templates — **borrow idea**: a 64-bit "basic TEDS" (manufacturer, model, version, serial) is the minimal identity record.
  - IEEE 1451.0 revision and ISO/IEC/IEEE 21451 series — **avoid** adopting; could not confirm the latest revision year.
  - OGC SensorML — **avoid**: XML, geospatial-observation oriented, heavy for agents.
  - OPC UA information models — **borrow idea** (typed nodes with EngineeringUnits and range properties); **avoid** the stack.
  - USB HID report descriptors — **borrow idea**: logical/physical min/max, unit and unit exponent packed into a few bytes show a manifest can be tiny; unreadable by humans and agents.
  - USBTMC / SCPI `*IDN?` and SCPI-99, LXI, IVI — **adopt** on the driver side for bench instruments that already speak SCPI; **borrow idea**: mandatory `*IDN?`, `*RST`, `*TST?` map to identify/reset/self-test in the module core.
  - Matter device types/clusters — **borrow idea** (conformance = required clusters per device type).
  - Home Assistant MQTT discovery and Zigbee2MQTT external converters — **borrow idea**: a discovery payload/"converter" file that anyone can contribute is how those ecosystems scaled device support.
  - OpenTelemetry semantic conventions — **borrow idea**: a governed attribute-naming registry; use for host-side driver telemetry.
  - CycloneDX / SPDX — **adopt** for firmware SBOM and hardware BOM references inside the manifest.
  - OpenAPI / JSON Schema — **adopt** JSON Schema 2020-12 for all payloads (matches MCP's default dialect, cited above).
  - UCUM — **adopt** for unit strings (case-sensitive codes avoid the "KWh" vs "kWh" class of error that Home Assistant warns about).
- Proposed manifest contents (design recommendation; detailed in the synthesis section): identity; hardware and firmware provenance; channels with quantity kind, UCUM unit, range, resolution, sample rate, uncertainty, calibration record; absolute-maximum ratings and isolation/safety limits; operations with risk tier, `safe`/`idempotent`, preconditions and interlocks; health and self-test; failure modes and troubleshooting hints; time-sync capability; power budget; data-stream schema references.
- WoT TD has no native field for measurement uncertainty, calibration traceability, absolute-maximum ratings, self-test or firmware build provenance; these need a project namespace. This is the genuinely new part of the standard and the part most useful to an AI agent.
- A two-layer manifest matches every precedent above: **Module Model** (class-level, lives in the repo/registry, versioned with semver) and **Module Instance** (serial, calibration data, firmware hash, measured self-test results, read from the device). Small devices carry only a UID + model ID + model hash; the host resolves the rest ("virtual TEDS").

### Gaps
- No W3C TD 2.0 status was visible in the portion of the TD 1.1 document fetched; whether a TD 2.0 draft exists as of November 2026 is unverified.
- IEEE 1451 newest revisions, TEDS type names (Meta-TEDS, TransducerChannel TEDS, Calibration TEDS) and 1451.4 template details were not on the page fetched.
- SensorML, OPC UA, USB HID, SCPI/LXI/IVI, Matter, Zigbee2MQTT, OpenTelemetry, CycloneDX/SPDX and UCUM were not fetched this session.
- I found no existing published "agent-readable datasheet" standard for instruments; absence is not proven, only not found with the limited tooling.

---

## A4. Firmware and RPC for modules; reproducible builds and attestation

### Takeaway
For a Rust-first stack, postcard-rpc (v0.12.1) over embassy-usb is the natural module RPC: endpoints and topics are keyed by a hash of path plus schema, so interface drift is detected by construction; Embassy supplies time, USB, networking and a power-fail-safe bootloader with rollback. The module-core service list should be lifted from DroneCAN/Cyphal, the "thin MCU, smart host, build-generated dictionary" split from Klipper, and firmware identity should be proven with GitHub artifact attestations (SLSA Build L2) checked against a hash the module reports.

### Cited Findings
- **postcard-rpc:** version 0.12.1, MIT/Apache-2.0; wire types implement serde plus `postcard_schema::Schema`; an *Endpoint* is a request type + response type + unique path with exactly one response per request; a *Topic* is a one-way message type + path that either side can send; host `HostClient` and device `Server` are generic over `WireTx`/`WireRx` traits implementable for USB, TCP, I2C or UART; modules include `standard_icd`, `host_client`, `server`; dependencies include Embassy crates, `nusb` (USB), `tokio-serial`, and `wasm-bindgen`/`web-sys` for web use. — [docs.rs postcard-rpc](https://docs.rs/postcard-rpc/latest/postcard_rpc/)
- **postcard-rpc keys:** every message starts with an 8-byte `Key` that is an FNV1a-64 digest of the path followed by the message schema, plus a varint sequence number that lets multiple requests be in flight and complete out of order; changing either path or schema yields a very different key; COBS framing is provided for serial. — [postcard-rpc overview.md](https://raw.githubusercontent.com/jamesmunns/postcard-rpc/main/docs/overview.md)
- **Embassy:** async Rust embedded framework with HALs for STM32, nRF52/53/91 and RP2040, ESP32 via the esp-rs project; `embassy-time` gives global `Instant`/`Duration`/`Timer`; `embassy-usb` is a device stack with CDC-ACM, HID and a builder for custom classes; `embassy-boot` is a bootloader for power-fail-safe updates with trial boots and rollback; `embassy-net` covers Ethernet/IP/TCP/UDP/DHCP; BLE on nRF52 via nrf-softdevice. — [embassy.dev](https://embassy.dev/)
- **Klipper's split:** the protocol aims to be low-latency, low-bandwidth and low-complexity for the microcontroller; the host invokes MCU functions and vice versa; commands are declared in firmware source, IDs are assigned at build time, and the host learns everything from the downloaded dictionary; MCU-to-host responses are not retransmitted so higher-level host code must tolerate loss. — [Klipper protocol docs](https://www.klipper3d.org/Protocol.html)
- **DroneCAN core services** (status heartbeat with health/mode, GetNodeInfo, parameters with save/erase, restart, firmware update pulled by the node, time sync). — [DroneCAN specification, section 6](https://dronecan.github.io/Specification/6._Application_level_functions/)
- **Meshtastic module API:** modules subclass `MeshModule` → `SinglePortModule` (raw bytes on one port number) → `ProtobufModule` (one protobuf type); instantiating a module registers it; port numbers work like UDP ports with `PRIVATE_APP` for development and a separate registry of assigned ports; periodic work uses cooperative `OSThread::runOnce()`. — [Meshtastic module API docs](https://meshtastic.org/docs/development/device/module-api/)
- **Flipper expansion protocol** (heartbeat/timeout handshake, baud negotiation, then protobuf RPC). — [Flipper expansion protocol docs](https://developer.flipper.net/flipperzero/doxygen/expansion_protocol.html)
- **GitHub artifact attestations:** cryptographically signed claims of build provenance (workflow, repository, commit SHA, triggering event) signed via Sigstore; public repositories use the Sigstore public-good instance with a public transparency log; on their own they provide SLSA v1.0 Build Level 2, and Level 3 needs a reusable workflow; verification is `gh attestation verify`; SBOM attestations are supported; attestations link an artifact to source and build instructions but do not make it secure. — [GitHub Docs: artifact attestations](https://docs.github.com/en/actions/concepts/security/artifact-attestations)
- **ESPHome** structures a device as declared components with per-sensor metadata (version 2026.11.0). — [ESPHome sensor component](https://esphome.io/components/sensor/)

Verdicts supported by the cited pages:
- postcard-rpc — **adopt** as the module RPC for project firmware (USB bulk and COBS serial; the same ICD crate shared by firmware and host).
- Embassy (+ embassy-usb, embassy-time, embassy-boot) — **adopt** as the firmware base.
- Klipper — **borrow idea**: keep MCU firmware thin and generate the interface dictionary from source at build time.
- DroneCAN/Cyphal — **borrow idea**: the mandatory-service checklist.
- Meshtastic module API — **borrow idea**: a registry of port numbers with a private range for development.
- GitHub artifact attestations — **adopt** for firmware and host binaries (free for public repositories' use of the public Sigstore instance per the cited page).

### Inferences
- (prior knowledge, unverified this session) postcard-rpc's `standard_icd` includes a ping endpoint and schema-discovery support so a host can enumerate a device's endpoints/topics; the docs page confirms the module exists but not its contents. If absent, the project should define `module/describe` itself.
- (prior knowledge, unverified this session) One line each for the rest:
  - embedded-hal 1.0 / embedded-hal-async traits — **adopt** so sensor driver crates are MCU-independent.
  - defmt — **adopt** for firmware logging (compact, host-decoded), exposed as a `log` topic.
  - CBOR/CoAP — **borrow idea**; use CBOR only where a self-describing encoding is needed (e.g., manifest blobs on device).
  - Protobuf/nanopb — **avoid** as the primary wire format in a Rust-only firmware stack; **adopt** protobuf on the host/log side if Foxglove schemas are used.
  - micro-ROS — **avoid**: pulls in the ROS 2/DDS toolchain.
  - Firmata — **avoid**: host-driven pin toggling with no typing, timing or description.
  - MAVLink — **avoid** as module RPC.
  - SCPI parser crates in Rust — **adopt** optionally as an alternate text front-end on bench-style modules so they are usable without the host stack.
  - TinyUSB — fine for C firmware from third parties; the standard should specify the wire behaviour, not the library.
  - probe-rs, espflash, UF2/picotool, nRF DFU — the agent-driven reflash path is: build in a pinned container, verify hash/attestation, flash through embassy-boot DFU over the module's own USB link, with SWD via probe-rs as the recovery path on the fixed bench.
- Minimum "module core" firmware services (recommendation; full list in the synthesis): `identify`, `describe`, `health` (heartbeat topic), `selftest`, `time` (sync + capture timestamping), `config` (get/set/save/erase with min/max/default), `log`, `update` (begin/write/commit/rollback), `reset`, and `safe_state`.
- Key-per-schema hashing (postcard-rpc) and build-generated dictionaries (Klipper) both imply the same rule for the standard: **the interface description is a build artifact, and its hash is part of firmware identity**. The host should refuse write operations when the reported interface hash is not one it has a manifest for.
- Reproducibility target: firmware built from a tagged commit with a pinned toolchain yields a bit-identical image whose SHA-256 the module reports in `identify`; CI publishes that image with an attestation; the host verifies `reported hash == attested hash` before trusting calibration-relevant code. The device cannot *prove* what it runs without a secure element, so this is integrity against accidents and drift, not against a malicious module — the manifest should say so.

### Gaps
- postcard-rpc's exact schema-discovery API, WebUSB status and supported embassy-usb versions were not confirmed.
- Embassy's licence, RP235x support and BLE host stack ("trouble") status were not on the page fetched.
- No verified source this session for Rust reproducible-build practice on thumbv6m/thumbv7em/riscv targets (e.g., path remapping, `--locked`, container pinning), or for secure-element-based firmware attestation on hobby MCUs.
- embedded-hal, defmt, CBOR/CoAP, nanopb, micro-ROS, Firmata, probe-rs were not fetched.

---

## A5. Host-side plug-in models and MCP gateway/aggregation

### Takeaway
Recommend **out-of-process drivers** (one small Rust binary per driver family, supervised by a gateway, speaking a narrow driver protocol over stdio or a Unix socket) behind a single MCP gateway that exposes a small, stable set of meta-tools plus on-demand discovery — not one MCP tool per instrument function. Process isolation is the pattern used by Kismet datasources and Viam modules; the July 2026 MCP revision (stateless, no per-connection tool lists, explicit handles) and Anthropic's measured token costs both push in the same direction. WebAssembly components are not ready for USB/serial device access (wasi-usb is still Phase 2) but are a plausible later home for pure-compute decoders.

### Cited Findings
- **Kismet datasources:** each capture source is an independent process, launched by the server with a pipe pair passed on the command line, speaking a protobuf protocol (probe source, open source, configure, data report, error/warning report); the same messages run over TCP for remote capture; separation lets the helper run with elevated privilege and keeps heavy work out of the main loop; helpers can be written in any language. — [Kismet datasource developer docs](https://www.kismetwireless.net/docs/dev/datasources/)
- **Viam modules:** a driver module implements a standard resource API (sensor, camera, arm...), runs as a separate process alongside `viam-server` with its own dependencies and can crash without taking the server down; models are identified by a `namespace:module:model` triplet; `meta.json` carries registry metadata and lists models; config is validated by a module-supplied method that also declares dependencies; distribution is via a registry with private/public visibility and tag-triggered cloud builds; the generator supports Python and Go. — [Viam docs: write a driver module](https://docs.viam.com/build-modules/write-a-driver-module/)
- **openDAQ module types** (device modules, protocol integrations, function blocks) around a common core. — [opendaq.com](https://opendaq.com/)
- **WASI status:** WASI 0.2.0 shipped January 2024 with point releases 0.2.1–0.2.12; WASI 0.3.0 (native async) shipped 11 June 2026 and 0.3.1 on 11 August 2026, supported and enabled by default in Wasmtime 46+ and in jco; 0.3.2 was scheduled for 13 October 2026 and 0.3.3 for 8 December 2026; no date is given for WASI 1.0. — [wasi.dev roadmap](https://wasi.dev/roadmap)
- **wasi-usb:** Phase 2 of the WASI process, champion Merlijn Sebrechts; goal is low-level USB device control; portability targets include Linux aarch64 with a Raspberry Pi 4 as reference hardware; two independent implementations are planned. — [WebAssembly/wasi-usb README](https://raw.githubusercontent.com/WebAssembly/wasi-usb/main/README.md)
- **Token cost of tool definitions:** a five-server MCP setup with 58 tools costs about 55K tokens before any conversation; Anthropic observed 134K tokens of tool definitions internally; with the Tool Search Tool (`defer_loading: true`) context use fell from ~77K to ~8.7K tokens (85% reduction) and MCP-eval accuracy rose from 49% to 74% (Opus 4) and 79.5% to 88.1% (Opus 4.5); programmatic tool calling cut average tokens on complex tasks by 37%; tool-use examples raised complex-parameter accuracy from 72% to 90%. Published 24 November 2025. — [Anthropic Engineering: advanced tool use](https://www.anthropic.com/engineering/advanced-tool-use)
- **Code execution with MCP:** presenting tools as files in a filesystem that the agent reads on demand (progressive disclosure), or a `search_tools` tool with selectable detail level, reduced one workflow from about 150,000 to 2,000 tokens (98.7%); intermediate data stays in the execution environment unless logged; requires a sandbox. Published 4 November 2025. — [Anthropic Engineering: code execution with MCP](https://www.anthropic.com/engineering/code-execution-with-mcp)
- **MCP 2026-07-28 revision:** removes protocol-level sessions and the initialize handshake (stateless; each request carries version and capabilities in `_meta`); adds mandatory `server/discover`; `tools/list` must not vary per connection (it may vary by authorization) and should be returned in deterministic order to improve prompt-cache hits; list results carry `ttlMs` and `cacheScope`; cross-call state uses explicit server-minted handles passed as ordinary tool arguments; `resources/subscribe` is replaced by a single `subscriptions/listen` stream; tasks moved to an official extension (`io.modelcontextprotocol/tasks`) with polling via `tasks/get`; server-initiated requests are replaced by "multi round-trip requests" (`input_required` results); Roots, Sampling and Logging are deprecated; OpenTelemetry trace-context propagation in `_meta` is documented. — [MCP changelog 2026-07-28](https://modelcontextprotocol.io/specification/2026-07-28/changelog.md)
- **Aggregation and naming:** tool-name uniqueness is scoped to a server; clients or proxies aggregating multiple servers should disambiguate, for example by prefixing with a server identifier, and must not rely on the server `name` being unique. — [MCP tools spec 2026-07-28](https://modelcontextprotocol.io/specification/2026-07-28/server/tools.md)
- **Docker MCP Gateway:** an open-source centralized proxy that runs MCP servers as isolated containers started on demand and injects credentials; its experimental "Dynamic MCP" exposes management tools (`mcp-find`, `mcp-add`, `mcp-remove`, `mcp-config-set`, `mcp-exec`, `code-mode`) so agents add servers mid-session; dynamically added servers last only for the session and code mode is described as not yet reliable. — [Docker MCP Gateway docs](https://docs.docker.com/ai/mcp-catalog-and-toolkit/mcp-gateway/), [Docker Dynamic MCP docs](https://docs.docker.com/ai/mcp-catalog-and-toolkit/dynamic-mcp/)
- **Meshtastic** compiles modules into firmware and registers them by instantiation. — [Meshtastic module API docs](https://meshtastic.org/docs/development/device/module-api/)

Verdicts supported by the cited pages:
- Subprocess drivers with a narrow IPC protocol (Kismet, Viam pattern) — **adopt**.
- WebAssembly components (WASI 0.3 / Wasmtime 46+) — **borrow idea now, adopt later** for sandboxed pure-compute plug-ins (protocol decoders, feature extractors); **avoid** for hardware drivers until wasi-usb or equivalent leaves Phase 2 and ships in a runtime.
- Containers per driver (Docker MCP Gateway style) — **avoid** on a Pi-class portable host for USB drivers (device pass-through and start-up cost); **borrow idea**: on-demand start and `find`/`add` meta-tools.
- MCP tool search / deferred loading / progressive disclosure — **adopt** as the default surface design.
- One-MCP-server-per-driver aggregated by a gateway — **borrow idea**: keep driver protocol MCP-shaped (describe/read/invoke/subscribe) so a driver *can* be run standalone as an MCP server for debugging, but do not make the agent face N servers.

### Inferences
- (prior knowledge, unverified this session) One line each: Rust `dyn Trait` with `libloading` dynamic libraries — **avoid** (no stable Rust ABI; a crashing driver takes down the host); Home Assistant integrations/add-ons — **borrow idea** (integration = in-process Python, add-on = container; the split mirrors "driver" vs "service"); Telegraf `execd` external plugins — **borrow idea** (long-lived child process speaking line protocol on stdio); GStreamer plugins — **adopt** as a library for camera/audio pipelines inside the relevant driver, not as the plug-in model; sigrok protocol decoders (stackable Python decoders) and Wireshark dissectors — **borrow idea** for a "decoder" plug-in class distinct from "driver"; Kismet — cited above.
- Recommended architecture (one choice, as requested):
  1. **Gateway** (Rust, single binary): owns MCP, write-safety policy (MWS), the module registry, time service, recorder and the supervision tree.
  2. **Drivers** as child processes: `probe` (given a hot-plug event with USB VID/PID/serial or a serial path, say whether you claim it), `open`, `describe` (return manifest instance), `read`/`invoke`/`subscribe`, `health`, `close`. Wire format: length-prefixed frames on stdio or a Unix socket; control messages JSON, bulk sample data on a separate shared-memory or socket channel so audio/IQ never passes through JSON.
  3. **Sim drivers** implement the same protocol backed by a recorded session (see B5).
  4. **Decoders/analyzers** as a second plug-in class operating on recorded streams (candidate for WASM later).
- MCP surface shape given the cited constraints: a fixed set of roughly 10–15 tools (for example `modules.list`, `modules.describe`, `channel.read`, `op.invoke`, `capture.start/stop/status`, `session.query`, `evidence.render`, `firmware.build/flash`, `tool.search`) whose arguments name the module and operation. Because `tools/list` may no longer vary per connection and should be deterministic for caching, hot-plugging a module should change *resources* (the module list and manifests) rather than the tool list. Risk tier is then computed per call from the manifest's operation entry, and the gateway — not the annotation — enforces it.
- Long captures map to the MCP tasks extension; capture/session identifiers map to the "explicit handle" pattern the 2026-07-28 revision prescribes.
- The stateless revision and deprecation of Sampling/Logging mean the server should not depend on server-initiated LLM calls or MCP logging; use OpenTelemetry for host telemetry.

### Gaps
- I could not survey third-party MCP gateway/aggregator projects (e.g., community proxies) or compare their namespacing and tool-filtering features this session; only Docker's gateway docs were fetched.
- No measured data on Wasmtime 46+ performance/footprint on Raspberry Pi 5, or on any shipping runtime implementation of wasi-usb / wasi-i2c / serial access.
- Telegraf, GStreamer, sigrok decoder and Wireshark dissector plug-in docs were not fetched.
- Client support for `subscriptions/listen`, the tasks extension and deferred tool loading in specific MCP clients as of November 2026 was not verified.

---

## A6. Governance of a small open standard

### Takeaway
The hobby standards that spread are short, free to implement, free to name (Qwiic: no royalty, no attribution, four hard rules), and published with reference designs; the ones that stay healthy at scale (MCP, OpenCyphal) add dated or versioned releases, a proposal process, and executable conformance tests tied to each normative MUST. For this project: spec text under CC BY 4.0 or Apache-2.0, hardware under CERN-OHL-P/W, code under Apache-2.0/MIT, USB PIDs via pid.codes, OSHWA self-certification for reference modules, and a conformance suite that runs against simulated modules in GitHub Actions.

### Cited Findings
- **Qwiic naming rules:** the name may be used without royalties or attribution if the board uses the same 4-pin connector style, 3.3 V, the colour scheme and the pinout; SparkFun supplies logo and footprint files. — [SparkFun Qwiic](https://www.sparkfun.com/qwiic)
- **Feather governance:** Adafruit maintains the spec and updates it "whenever we can"; third parties are encouraged; Adafruit only guarantees its own boards. — [Adafruit Feather specification](https://learn.adafruit.com/adafruit-feather/feather-specification)
- **Gridfinity:** specification is community-maintained, self-described as a work in progress, coordinated via Discord and a GitHub repo for the site. — [gridfinity.xyz specification](https://gridfinity.xyz/specification/)
- **OpenCyphal licensing:** the specification is open to implement, distribute and use with no licensing or approval; reference implementations are MIT; website content is CC BY 4.0. — [opencyphal.org](https://opencyphal.org/)
- **Framework:** expansion-card reference designs are CC BY 4.0. — [FrameworkComputer/ExpansionCards](https://github.com/FrameworkComputer/ExpansionCards)
- **MCP governance:** established as "Model Context Protocol a Series of LF Projects, LLC"; code and specification contributions under Apache-2.0, documentation (excluding specifications) under CC BY 4.0; contributors keep copyright; hierarchy of contributors, maintainers, core maintainers and lead maintainers (BDFL model), membership by individual not company; spec changes go through Specification Enhancement Proposals; working groups produce deliverables and interest groups frame problems. — [MCP governance](https://modelcontextprotocol.io/community/governance.md)
- **MCP versioning and lifecycle:** revisions are identified by date (2024-11-05, 2025-03-26, 2025-06-18, 2025-11-25, 2026-07-28, plus a rolling draft); the 2026-07-28 revision adopted a feature lifecycle (Active/Deprecated/Removed) with a minimum twelve-month deprecation window and a deprecated-features registry. — [MCP docs index](https://modelcontextprotocol.io/llms.txt), [MCP changelog 2026-07-28](https://modelcontextprotocol.io/specification/2026-07-28/changelog.md)
- **MCP conformance:** SEP-2484 (created 2026-03-27, status Final) requires that a standards-track change with observable protocol behaviour cannot become Final until a conformance scenario is merged with a YAML traceability file mapping every MUST/MUST NOT/SHOULD/SHOULD NOT to a check or a documented exclusion; spec text stays authoritative over tests; SDK tiers are measured against a pinned conformance release (Tier 1 = 100% pass, Tier 2 = 80%); a scaffolding command `npx @modelcontextprotocol/conformance new-scenario --sep <n>` is referenced; any implementation may run the suite. — [SEP-2484](https://modelcontextprotocol.io/seps/2484-conformance-tests-required-for-final-seps.md)
- **MCP registry:** the project runs an official server registry with publishing, versioning, moderation policy and GitHub Actions automation pages. — [MCP docs index](https://modelcontextprotocol.io/llms.txt)
- **pid.codes:** free PIDs under VID 0x1209 by pull request for projects with public sources and recognised open licences. — [pid.codes how-to](https://pid.codes/howto/)
- **OSHWA certification:** self-certification through an online licence agreement; everything under the creator's control (hardware, software, documentation) must be open per the OSHW definition; closed third-party components are allowed if their datasheets are accessible; each product/version gets its own registration code; certification is renewed yearly; enforcement escalates from a cure period (0–90 days) to public listing at 120 days and fines from 180 days. — [OSHWA certification requirements](https://certification.oshwa.org/requirements.html)
- **CERN OHL v2:** three variants — CERN-OHL-S (strongly reciprocal), CERN-OHL-W (weakly reciprocal), CERN-OHL-P (permissive). — [CERN OHL](https://cern-ohl.web.cern.ch/)
- **Viam registry model:** `namespace:module:model` identifiers, with visibility levels and tag-triggered builds. — [Viam docs](https://docs.viam.com/build-modules/write-a-driver-module/)

### Inferences
- Recommended licence split (consistent with the precedents cited): specification text and manifest JSON Schemas under Apache-2.0 (as MCP does for its spec, giving a patent grant) or CC BY 4.0 (as OpenCyphal's site and Framework's designs do); reference hardware under CERN-OHL-P for connector/footprint reference designs meant to be copied into commercial modules and CERN-OHL-W for complete reference modules; firmware and host code Apache-2.0 OR MIT (matching postcard-rpc and Rerun dual licensing cited elsewhere in these notes).
- Naming: copy Qwiic's rule — the compatibility mark is free to use if and only if a short list of testable requirements is met — and tie the right to use the mark at each conformance level to passing the public test suite. A trademark registration is the only enforcement lever a spec has; whether to register is a cost decision for the owner (not researched).
- Identifier registry: three namespaces resolved in one Git repository by pull request, mirroring pid.codes and Viam — (1) vendor namespace in reverse-DNS form (DroneCAN's node-name convention), (2) module model ID `vendor.family.model` with semver, (3) optional USB PID under 0x1209 obtained from pid.codes. CI validates manifests against the JSON Schema on every pull request.
- Versioning: semver for the manifest schema and module models (WoT TD recommends semver for Thing versions, cited in A3); dated or semver releases of the spec with a stated deprecation window (MCP's twelve-month minimum is a reasonable ceiling for a hobby project; a shorter window can be chosen while pre-1.0).
- Conformance tooling runnable in GitHub Actions (no hardware): (a) JSON Schema validation of manifests; (b) a "module-core conformance" harness that speaks the module RPC to firmware compiled for the host or run under an emulator, checking each MUST with a traceability file in the SEP-2484 style; (c) a driver-protocol harness run against sim drivers; (d) the MCP project's own conformance suite against the gateway. Hardware-in-the-loop conformance runs on the fixed bench and publishes signed results.
- Spec size discipline: Qwiic and Feather fit on a page; HAT+ is a short PDF. A v0.1 that fits in under ten pages with one reference module and one reference host is more likely to be adopted than a complete framework.

### Gaps
- OSHWA certification cost, UID format and accepted licence list were not on the page fetched; CERN OHL v2 release date and OSI status likewise.
- How Gridfinity's de facto standard is versioned (if at all), and its licence, were not confirmed.
- OpenCyphal's specification versioning rules and DSDL data-type versioning policy were not fetched.
- No research was done on trademark registration costs or processes.

---

## B1. Time synchronization across heterogeneous sensors on a small Linux host

### Takeaway
Use one host clock as the session timebase, discipline it with chrony (GNSS PPS when available), timestamp every sample at the earliest point the kernel offers, and — following Lab Streaming Layer — record raw device timestamps plus periodic clock-offset measurements and correct after the fact rather than rewriting timestamps at capture. Hardware PTP exists on Pi CM4/CM5 NICs (10–15 ns between two CM4s) but is unnecessary for a single-host pack; the limiting errors are USB audio (~1 ms after delay compensation), end-of-frame camera stamps, and serial-poll latency, which a deliberately injected light/sound/electrical sync event can measure per session.

### Cited Findings
- **chrony:** with hardware timestamping on a LAN, "sub-microsecond accuracy and stability of a few tens of nanoseconds might be possible"; message-based GPS timing is accurate only to milliseconds whereas PPS is much more accurate; PPS is configured as `refclock PPS /dev/pps0 lock NMEA refid GPS` together with an NMEA/gpsd SOCK source because PPS needs another source to number the seconds (locked source must be within 0.4 s as of chrony 4.1); default behaviour is slewing, with `makestep 1 3` to step only during the first updates; `rtcsync` copies system time to an RTC every 11 minutes; without an RTC, `-s` restores time from the drift file's modification time; chrony suits isolated networks and can serve a `local` reference. — [chrony FAQ](https://chrony-project.org/faq.html)
- **PTP on Raspberry Pi:** the CM4's BCM54210PE NIC supports PTP hardware timestamping; two CM4s on a LAN synchronised with `ptp4l` to about 10–15 ns; the CM4 IO board exposes SYNC_IN/SYNC_OUT pins, of which the author found one usable for PPS in *or* out; a 2024 update says PTP works out of the box on CM5 (article dated 6 July 2022). — [Jeff Geerling: PTP on CM4](https://www.jeffgeerling.com/blog/2022/ptp-and-ieee-1588-hardware-timestamping-on-raspberry-pi-cm4)
- **Pi 5 grandmaster build:** a Raspberry Pi 5 with a TimeHAT (Intel i226 NIC with PTP hardware timestamping, about USD 200) and a u-blox Neo-M9N GNSS M.2 module (about USD 195) with PPS in/out on SMA/U.FL ran stably as a GPS-disciplined time server using chrony (post dated 28 March 2025). — [Jeff Geerling: DIY PTP grandmaster](https://www.jeffgeerling.com/blog/2025/diy-ptp-grandmaster-clock-raspberry-pi)
- **V4L2 camera timestamps:** `v4l2_buffer.timestamp` for capture is when the first data byte was captured, on `CLOCK_MONOTONIC` when the MONOTONIC flag is set; the timestamp *source* flag distinguishes end-of-frame (SRC_EOF, often read slightly late in software) from start-of-exposure (SRC_SOE); a `sequence` counter counts frames including drops, but frames dropped by remote hardware such as over USB may not be counted. — [Linux V4L2 buffer docs](https://docs.kernel.org/userspace-api/media/v4l/buffer.html)
- **ALSA audio timestamps:** ALSA exposes `trigger_tstamp`, `tstamp`, `audio_tstamp` and `delay`; applications can select CLOCK_REALTIME, CLOCK_MONOTONIC or CLOCK_MONOTONIC_RAW (the last excludes NTP corrections); the ratio of elapsed system time to audio time tracks long-term drift between audio and system clocks; for USB audio, uncompensated DMA timestamps showed about 5 ms offset from buffering, and about 1 ms accuracy after delay compensation thanks to the USB frame counter. — [Linux ALSA timestamping docs](https://docs.kernel.org/sound/designs/timestamping.html)
- **SocketCAN:** after reading a frame, `SIOCGSTAMP` returns a receive timestamp with 1 µs resolution set at reception; CAN FD is enabled per socket; `vcan` provides virtual CAN interfaces with no hardware. — [Linux SocketCAN docs](https://docs.kernel.org/networking/can.html)
- **Linux IIO:** buffered scans carry an aligned 64-bit timestamp; `iio_get_time_ns()` supplies event timestamps. — [Linux IIO core docs](https://docs.kernel.org/driver-api/iio/core.html)
- **Lab Streaming Layer:** each sample is stamped with the sender's local monotonic clock and timestamps are not altered by default; clock offsets between sender and receiver are measured every few seconds with short NTP-like UDP exchanges; the XDF recording stores raw timestamps *and* all offset measurements, and importers fit a line per stream (assuming constant drift) to map onto the recorder's clock and de-jitter regularly sampled streams; for devices with non-LSL clocks the documented options are a shared hardware pulse aligned by cross-correlation or a software bridge followed by offline regression; a per-stream `offset_mean` header field holds known constant latency. — [LSL time synchronization docs](https://labstreaminglayer.readthedocs.io/info/time_synchronization.html)
- **Bus-level sync pattern:** DroneCAN's `GlobalTimeSync` sends the precise transmit time of the previous message so receivers can compute phase error from their own receive timestamps. — [DroneCAN specification, section 6](https://dronecan.github.io/Specification/6._Application_level_functions/)
- **Klipper** schedules MCU actions against an MCU clock value and reports `status clock=` from the MCU, i.e., the host maintains a mapping between host time and each MCU's clock. — [Klipper protocol docs](https://www.klipper3d.org/Protocol.html)
- **SigMF:** each capture segment can carry `core:datetime`, an RFC 3339 UTC timestamp for the sample at `sample_start`, with arbitrary fractional-second precision. — [SigMF specification v1.2.6](https://sigmf.org/)
- **openDAQ** lists PTP among device protocols and shows signals sharing one time base. — [opendaq.com](https://opendaq.com/)

### Inferences
- Expected achievable alignment per sensor class on a Pi-class host (engineering estimates combining the cited figures with prior knowledge; only the USB-audio, CAN-resolution and CM4-PTP numbers are sourced):
  - Host clock to UTC with GNSS PPS + chrony: microsecond class (prior knowledge, unverified this session); without GNSS, absolute time is whatever NTP last gave, but *relative* alignment inside one host is unaffected.
  - CAN frames via SocketCAN: timestamp resolution 1 µs (cited); real accuracy with USB CAN adapters is limited by USB delivery, typically sub-millisecond to ~1 ms unless the adapter timestamps in hardware (prior knowledge).
  - USB audio / mic array: ~1 ms after ALSA delay compensation (cited); sample-clock drift of tens of ppm must be tracked with the system-time/audio-time ratio (cited mechanism).
  - SDR IQ: sample counting is exact within a stream; absolute start time has USB-latency uncertainty (ms class) unless the SDR has a PPS/timestamp input; tuner clock error is tens of ppm for low-cost dongles (prior knowledge).
  - UVC/CSI cameras: one frame period worst case (33 ms at 30 fps) if only end-of-frame software stamps are available; CSI cameras with start-of-exposure stamps or a strobe line reach sub-millisecond (prior knowledge; the SOE/EOF distinction is cited).
  - K-line OBD / Modbus polls: the response describes a value sampled somewhere inside the request–response window; record both request-send and response-receive host times and report the midpoint with ± half-window uncertainty (tens of ms for K-line at 10.4 kbaud; prior knowledge).
  - Wireless pods (ESP-NOW/BLE): two-way timestamp exchange or reference-broadcast schemes give roughly millisecond-class sync with application-level timestamps and tens of microseconds with radio-interrupt-level timestamps (prior knowledge of the FTSP/RBS literature, unverified this session).
- Recommended clock design:
  1. Session timebase = host nanoseconds since Unix epoch from the chrony-disciplined clock, with the *monotonic-raw* reading stored alongside so later analysis is immune to slews/steps (the ALSA and chrony pages both warn that realtime/NTP-corrected clocks are adjusted).
  2. Every stream stores (a) the device's own counter or sample index and (b) host receive time; the recorder also logs periodic `(device_time, host_time, round_trip)` offset samples per stream. Mapping device time to session time is a fitted line per stream, stored as metadata and recomputable — the LSL/XDF design.
  3. Module-core `time` service: host sends its time, module replies with its receive/transmit ticks (NTP-style four-timestamp exchange), plus an optional hardware sync input that latches the module's counter on a pulse.
  4. A **sync-event module**: on command, simultaneously flashes an LED, emits a click through a small speaker/piezo and puts an edge on a shared sync line and a spare ADC/logic-analyzer channel. Detecting that event in each stream gives a measured per-stream latency (LSL's "shared hardware pulse aligned by cross-correlation" method). Run at session start and end to also estimate drift.
  5. Optional GNSS PPS into a host GPIO for absolute time and for multi-host setups (pack + fixed bench); add PTP only if a second host with a PTP-capable NIC needs sub-microsecond agreement.
- For the user's example (engine cranking: audio + OBD + battery voltage), the dominant error is the OBD poll window, not the clocks; battery voltage sampled by a module with a counter-latched ADC and audio at ~1 ms are far tighter than K-line PID updates. The evidence bundle should therefore show per-stream time uncertainty explicitly.

### Gaps
- No sourced accuracy figure this session for GNSS PPS + chrony on a Raspberry Pi 5 GPIO, for gPTP on Pi hardware, or for Pi 5 on-board NIC PTP support specifically.
- No sourced measurements for ESP-NOW or BLE time-sync accuracy, nor for RBS/FTSP-style protocols on ESP32/nRF52 with Embassy; these need a targeted literature check.
- SocketCAN hardware-timestamp (`SO_TIMESTAMPING`) support and specific USB-CAN adapter behaviour were not covered by the page fetched.
- No source fetched on SDR sample-clock drift figures or on camera hardware-strobe support for specific Pi camera modules, 360 or thermal cameras.

---

## B2. Logging/container formats and tools for multi-modal time series

### Takeaway
MCAP is the best session container: MIT-licensed, a maintained Rust crate (0.26.0, docs dated 19 October 2026), nanosecond `log_time`/`publish_time` on every message, arbitrary schemas per channel, chunked and indexed, plus attachments and metadata records; an interrupted file can be rebuilt with `mcap recover`. Pair it with Rerun (0.44.0, MIT/Apache-2.0, Rust SDK, opens `.mcap` directly) or Lichtblick (MPL-2.0 fork of the discontinued open-source Foxglove Studio) for viewing, keep SigMF sidecars for wideband IQ, and treat MF4, sigrok `.sr`, pcapng, Parquet and HDF5 as import/export formats rather than the primary store.

### Cited Findings
- **MCAP record model:** Header (profile, library), Schema (name, encoding, data), Channel (topic, message encoding, metadata, schema ID), Message (channel ID, sequence, `log_time`, `publish_time`, data), Chunk (batched records, optional zstd/lz4 compression, time range, CRC), Message Index, Chunk Index, Attachment (name, media type, data, CRC; cannot be inside a chunk), Metadata (named string map), Statistics, Summary Offset, Data End, Footer. Timestamps are uint64 nanoseconds since a user-defined epoch; `publish_time` must equal `log_time` if unavailable. Summary and indexes are optional and readers can scan the data section sequentially, but a file without a footer is malformed. Opcodes 0x80–0xFF are reserved for private use. — [MCAP specification](https://mcap.dev/spec)
- **MCAP concepts:** messages are organised by channel, and a schema (for example Protobuf or JSON Schema) describes the messages on a channel. — [MCAP concepts guide](https://mcap.dev/guides/concepts)
- **MCAP libraries and licence:** C++, Go, Python, JavaScript/TypeScript, Swift and Rust (crate `mcap`); MIT licence. — [foxglove/mcap on GitHub](https://github.com/foxglove/mcap)
- **Rust crate:** `mcap` 0.26.0 (docs dated 19 October 2026), MIT; `Writer`/`WriteOptions` (compression, profile), optional lz4 and zstd; streaming `MessageReader` whose memory use depends on the largest record or chunk; a `sans_io` module with `LinearReader` (raw records from any byte source), `IndexedReader` (random access through the summary) and `SummaryReader`; attachment type supported. — [docs.rs mcap](https://docs.rs/mcap/latest/mcap/)
- **MCAP CLI:** `info`, `cat`, `filter`, `merge`, `sort`, `du`, `doctor`, `list`, `get`, `add` (attachment/metadata in place), `compress`/`decompress`, `convert` (ROS 1 bag and ROS 2 db3 to MCAP) and `recover`; `mcap recover` reads a potentially corrupt or truncated file and writes a valid copy by rebuilding chunk indexes and the summary, exiting 0 for full recovery, 3 for lossy recovery (records discarded or file truncated mid-record) and 1 if nothing was recoverable; `mcap filter -y <topic>` copies selected data to a new file. — [MCAP CLI guide](https://mcap.dev/guides/cli)
- **Rerun:** version 0.44.0 is the latest documented; SDKs for Python, C++ and Rust; `.rrd` is its recording format; data is logged against named timelines; archetypes include images, encoded images, video streams, audio assets, scalars, tensors, points, meshes, transforms and text logs, with 2D, 3D, time-series, tensor and audio views; the viewer opens `.mcap` files; a local catalog server (`rerun server`) exposes recordings as queryable segments returning DataFusion DataFrames; Rerun Hub is the commercial managed offering. — [Rerun docs: what is Rerun](https://rerun.io/docs/getting-started/what-is-rerun)
- **Rerun licence and stability:** Apache-2.0 and MIT; "the API is still evolving. Expect breaking changes"; open-core with the promise that everything in the repository stays open source; known limits include slowdowns with very many entities and multi-million-point clouds. — [rerun-io/rerun on GitHub](https://github.com/rerun-io/rerun)
- **Rerun + MCAP:** ROS 2 messages and Foxglove Protobuf schemas are converted to Rerun archetypes on import; unknown Protobuf/ROS 2 messages are decoded by schema reflection into queryable components; `message_log_time` and `message_publish_time` timelines are created for every message; `rerun mcap convert input.mcap -o output.rrd` with `--timeline-type` and `--timestamp-offset-ns` options; ROS 1 is not supported. — [Rerun MCAP reference](https://rerun.io/docs/reference/mcap)
- **Foxglove licensing:** on 10 March 2024 Foxglove 2.0 merged Studio and Data Platform and discontinued development of the open-source edition of Foxglove Studio, stating MCAP and its other open-source projects were unaffected. Current pricing: Free plan with 10 GB storage, 3 visualization users and 5 devices; Pro from USD 20/month plus usage; free Academic plan with a .edu/.ac address; local `.mcap`/`.bag` files can be opened without internet. — [Foxglove 2.0 announcement](https://foxglove.dev/blog/foxglove-2-0-unifying-robotics-observability), [Foxglove pricing](https://foxglove.dev/pricing)
- **Lichtblick:** an MPL-2.0 visualization and diagnosis tool for robotics that began as a fork of Foxglove Studio; runs in the browser, as a desktop app (Linux/Windows/macOS) or via Docker; about 1.2k stars and 752 forks at fetch time. — [lichtblick-suite/lichtblick on GitHub](https://github.com/lichtblick-suite/lichtblick)
- **SigMF v1.2.6:** a recording is a binary `.sigmf-data` file plus a JSON `.sigmf-meta` file with `global` (required `datatype` such as `cf32_le`, `sample_rate`, `hw`, `recorder`, `version`), `captures` (`sample_start`, `frequency`, `datetime`) and `annotations` (sample ranges with optional frequency edges, label, comment); archives are POSIX tar `.sigmf`; a `.sigmf-collection` lists multiple recordings with SHA-512 hashes of their metadata (multi-channel/phased-array example); namespaced extensions are declared in `core:extensions`; the spec repository is CC-BY-SA-4.0. — [SigMF specification](https://sigmf.org/), [sigmf/SigMF on GitHub](https://github.com/sigmf/SigMF)
- **sigrok `.sr` v2:** a ZIP containing `version`, an INI `metadata` file and chunked `logic-1-N` / `analog-1-<chan>-N` binary files; the page mentions no absolute timestamps; a v3 format is under development. — [sigrok wiki: file format v2](https://sigrok.org/wiki/File_format:Sigrok/v2)
- **asammdf:** a Python parser/editor for ASAM MDF versions 2, 3 and 4 (`.mf4`) at documentation version 8.8.8, with bus-logging extraction (`extract_bus_logging()`), `export()`, `concatenate()`, `stack()`, `cut()`, `resample()` and a GUI with CAN/LIN/FlexRay bus trace; requires Python 3.10+. — [asammdf documentation](https://asammdf.readthedocs.io/en/latest/)
- **XDF (LSL):** stores raw per-stream timestamps and clock-offset measurements for post-hoc synchronisation, read by pyxdf, MNE-Python and EEGLAB. — [LSL time synchronization docs](https://labstreaminglayer.readthedocs.io/info/time_synchronization.html)

Verdicts supported by the cited pages:
- MCAP — **adopt** as the session container.
- Rerun — **adopt** as the primary viewer and Rust logging SDK for live inspection; **avoid** `.rrd` as the archival format while the project states that breaking changes should be expected.
- Lichtblick — **adopt** as an alternative open viewer for MCAP.
- Foxglove (app) — **borrow idea** / optional: usable free at small scale but closed-source since March 2024; do not make it a dependency of an open standard. Foxglove's open message schemas remain useful because Rerun and Lichtblick both understand them.
- SigMF — **adopt** for IQ recordings (sidecar files referenced from the MCAP session).
- sigrok `.sr` — **borrow idea** / import-export only: no absolute time in the documented format.
- ASAM MDF4 via asammdf — **adopt** as an export target for automotive users; **avoid** as primary store (Python-centric open tooling, automotive scope).
- XDF — **borrow idea**: store clock-offset measurements in the recording.

### Inferences
- (prior knowledge, unverified this session) Other candidates, one line each: ROS 2 bags — **avoid** (MCAP is already the default ROS 2 bag storage, and the project does not need ROS); PlotJuggler — **adopt** as an optional scalar-plot viewer that reads MCAP; HDF5 — **avoid** as the live recording format (not robust to power loss mid-write without care) but fine as an analysis export; Parquet/Arrow + DuckDB — **adopt** as the *derived* analytics layer for scalar telemetry (columnar, SQL-queryable, cheap to slice for an LLM); Zarr — **avoid** (cloud array store, wrong shape); pcapng — **adopt** for network and raw CAN/USB captures that Wireshark should open, referenced from the session; InfluxDB/QuestDB — **adopt** optionally on the fixed bench for long-term monitoring dashboards, not for session capture; OpenTelemetry — **adopt** for the host software's own traces/metrics, not for sensor data.
- Session layout recommendation (meets "one timebase, appendable on power loss, easy to slice"):
  - `session.mcap` — all scalar telemetry, bus frames (CAN, Modbus transactions, K-line bytes), events/triggers, clock-offset samples, compressed audio or PCM blocks, and compressed video frames, each as a channel with a registered schema; chunk size small (order of 1 MB or 1 s) with zstd, and `fsync` per chunk.
  - `iq/*.sigmf-data` + `.sigmf-meta` — wideband IQ, with an MCAP message per capture segment pointing at file, sample offset and `core:datetime`.
  - `manifests/` — the manifest instance of every module present, stored as MCAP Metadata/Attachment records at finalisation and also as plain JSON files so a truncated session still has them.
  - `derived/` — Parquet tables and rendered evidence images, reproducible from the above.
- Power-loss behaviour: the MCAP spec says the footer is mandatory, so a file cut off by power loss is technically malformed until `mcap recover` (or an equivalent linear scan with the Rust `LinearReader`) rebuilds indexes; loss is bounded by the last unflushed chunk. Because attachments cannot live inside chunks and are single records, large evolving binaries (IQ, long video) should be separate files or chunked messages, not attachments. Rotating to a new MCAP file every few minutes bounds recovery time and makes ring-buffer deletion trivial.
- Slicing for an LLM: topic selection is native (`mcap filter`, `IndexedReader`); time-range reads use the chunk index. A session-level JSON index (channels, units, sample counts, time span, detected events) costs a few hundred tokens and lets the agent request exactly the slice it needs.
- Message encoding choice inside MCAP is an early, sticky decision: Protobuf with Foxglove-compatible schemas gives free visualisation in Rerun and Lichtblick (cited: both ingest Foxglove Protobuf schemas); JSON+JSON Schema is easiest for agents but bulky; raw postcard bytes with the postcard schema are most natural for Rust firmware but opaque to third-party viewers. A reasonable split is Protobuf for anything a viewer should draw (images, audio, scalars, logs) and JSON for low-rate events.

### Gaps
- Whether the Rust `mcap` crate itself exposes a recovery helper equivalent to the Go CLI's `recover`, and the writer's behaviour/options for flush-per-chunk, were not confirmed from the docs page.
- MCAP's registry of well-known schema/message encodings and Foxglove schema names (e.g., compressed video, raw audio) were not fetched; audio support in Foxglove schemas as of late 2026 is unverified.
- Lichtblick's latest release version/date and its maintainers were not on the page fetched; PlotJuggler's 2026 status was not fetched.
- Rerun `.rrd` format-stability guarantees and Raspberry Pi/aarch64 viewer performance were not found.
- No verification of HDF5, Parquet/Arrow, Zarr, pcapng, InfluxDB/QuestDB/DuckDB details this session.

---

## B3. Event detection and cross-modal correlation on a Pi-class device

### Takeaway
No surveyed framework provides a general cross-sensor "black box" trigger for a hobby instrument pack; each is single-domain (Frigate for video, openDAQ for industrial DAQ, Viam for robots). The practical design is a rolling ring buffer of short MCAP segments with cheap streaming detectors (thresholds, change-point, spectral bands) raising trigger events that pin pre/post windows across *all* streams, with heavier analysis (matrix profile, cross-correlation, order tracking) run on the pinned slice afterwards.

### Cited Findings
- **Frigate (docs version 0.18):** a local NVR with real-time object detection that records with retention based on detected objects, runs detection in separate processes, communicates over MQTT for integration, has "review items", and links to audio detectors; CPU-only detection is for testing. — [Frigate documentation](https://docs.frigate.video/)
- **STUMPY:** a BSD-3-Clause Python library computing the matrix profile, used for motif discovery, anomaly/discord discovery, semantic segmentation (FLUSS regime changes), time-series chains and streaming data; depends on NumPy, Numba and SciPy; supports Python 3.11+. — [STUMPY documentation](https://stumpy.readthedocs.io/en/latest/)
- **openDAQ (3.40.4):** function blocks process signals inside the SDK; signals share a time base. — [opendaq.com](https://opendaq.com/)
- **Viam:** data capture is tied to standard resource APIs implemented by modules. — [Viam docs](https://docs.viam.com/build-modules/write-a-driver-module/)
- **Linux IIO:** the kernel's own model already pairs buffers with *triggers* (a trigger fires, a poll function captures a scan) and channel event specs (thresholds). — [Linux IIO core docs](https://docs.kernel.org/driver-api/iio/core.html)
- **Lab Streaming Layer:** cross-correlation against a shared hardware pulse is the documented way to align a stream whose clock is not otherwise synchronised. — [LSL time synchronization docs](https://labstreaminglayer.readthedocs.io/info/time_synchronization.html)
- **Multimodal-LLM anomaly detection research:** the VisualTimeAnomaly benchmark evaluates zero-shot multimodal LLMs on point-wise, range-wise and variate-wise anomalies including irregular sampling, and proposes TSAD-Agents, a multi-agent framework that calls *traditional methods and multimodal LLMs as tools* and switches between text and image inputs (arXiv 2502.17812, v2 dated 17 February 2026). — [Xu et al., "Can Multimodal LLMs Perform Time Series Anomaly Detection?"](https://arxiv.org/abs/2502.17812)
- **SigMF annotations** mark sample ranges with frequency edges and labels — a ready-made event record for RF detections. — [SigMF specification](https://sigmf.org/)

Verdicts supported by the cited pages:
- Frigate — **borrow idea** (pre/post-event retention driven by detections, MQTT event bus); **adopt** as-is only for fixed-bench camera NVR duty.
- STUMPY / matrix profile — **adopt** for offline discord/motif analysis of a pinned slice; **avoid** as an always-on detector on a Pi (Python + Numba JIT).
- openDAQ function blocks, IIO triggers — **borrow idea** (triggers and processing blocks as first-class, described objects).
- TSAD-Agents pattern — **borrow idea**: classical detectors propose, the LLM adjudicates.

### Inferences
- (prior knowledge, unverified this session) One line each: Scrypted — **borrow idea** (camera plug-in hub; video only); Edge Impulse — **borrow idea** (on-device feature extraction + small classifier per sensor; proprietary cloud tooling, so avoid as a dependency); Node-RED and Home Assistant automations — **adopt** optionally on the fixed bench as a user-facing rule layer fed by MQTT; **avoid** as the capture trigger path (latency, not sample-accurate); NVIDIA Holoscan — **avoid** (GPU-centric); GStreamer — **adopt** inside camera/audio drivers (it has timestamped buffers and pre-roll ring-buffer elements); Viam — **borrow idea** only.
- Ring-buffer "black box" design:
  - Every stream is always recorded into rotating MCAP segments (for example 5–10 s each) in a RAM-backed or wear-tolerant directory with a retention budget per stream class (minutes for IQ/video, hours for scalars).
  - A trigger is itself a message on an `events` channel: `{id, t_trigger, source_stream, rule, pre_s, post_s, severity}`. On trigger the recorder hard-links/copies the segments covering `[t - pre, t + post]` for *all* streams into a persistent session and writes an event manifest. Because all streams share the timebase, cross-sensor capture needs no per-sensor logic.
  - Sources of triggers: module-side (firmware raises an event topic with its own timestamp, e.g., voltage dip detected at the ADC), host-side streaming detectors, and agent/manual ("mark now").
- Trigger rule language: keep it declarative and small — per-channel predicates (`>`, `<`, rate-of-change, band energy above threshold, state change, bus-frame match by ID/mask, absence/timeout) combined with AND/OR/within-window sequencing, compiled to a streaming evaluator in Rust. Store the rule text in the session so a replay can re-evaluate it (golden-session tests, B5).
- Lightweight always-on detectors that fit a Pi 5 in Rust: rolling z-score/EWMA, CUSUM change-point, short-time FFT band energies and spectral flux for audio and vibration, RMS/crest factor, simple order tracking by resampling against an RPM signal (from OBD or a tach pulse) for rotating machinery, and normalised cross-correlation between two streams over a bounded lag window to estimate delay or confirm co-occurrence (for example cranking current dip versus starter-motor sound onset).
- Heavier, on-demand analysis on the pinned slice (seconds to minutes of data): matrix profile discords, spectrogram rendering, audio embeddings, and LLM review.
- Rust audio feature extraction (FFT crates, mel filterbanks) exists in the ecosystem but I could not verify specific crate status this session; treat crate selection as open.

### Gaps
- No source fetched for Frigate's pre/post capture configuration keys, its Raspberry Pi support, or audio-detector details.
- No verified sources for Scrypted, Edge Impulse, Node-RED, Home Assistant automations, Holoscan or GStreamer pre-roll elements this session.
- No benchmark data for matrix-profile or streaming change-point detection on Raspberry Pi 5, and no survey of Rust crates for audio features, CUSUM or matrix profile.
- I found no existing open "multi-sensor trigger language" standard; absence is not proven.

---

## B4. Presenting fused evidence to an LLM cheaply

### Takeaway
Published results favour rendering time series as plots for multimodal models over pasting numbers as text — one Google study reports up to 90% lower API cost and up to 120–150% better task performance for plots versus raw text — and the strongest agent pattern is hybrid: classical detectors and statistics first, images second, raw data only by reference. MCP already supports image and audio content blocks, resource links and (in the 2026-07-28 revision) a subscription stream, and Anthropic's own measurements show large savings from keeping bulk data out of context and filtering in code.

### Cited Findings
- **Plots vs text:** "Plots Unlock Time-Series Understanding in Multimodal Models" (Daswani et al., arXiv 2410.02637, v2 28 November 2024) finds that presenting time series to GPT and Gemini family models as plots outperforms raw text, with up to a 120% performance increase on synthetic zero-shot tasks, up to 150% on real-world consumer-health tasks (fall detection, activity recognition, readiness), and up to a 90% reduction in model API cost, without additional training. — [arXiv 2410.02637](https://arxiv.org/abs/2410.02637)
- **Vision models for time series (survey):** Ni et al. (arXiv 2502.08869, revised 30 August 2025) argue vision models are a promising alternative to LLM text approaches because LLMs must bridge continuous series to discrete tokens and struggle to model correlations among variates explicitly; the survey taxonomises ways of imaging time series and notes open pre-/post-processing challenges. — [arXiv 2502.08869](https://arxiv.org/abs/2502.08869)
- **Multimodal LLM anomaly detection:** VisualTimeAnomaly benchmark plus TSAD-Agents, which uses traditional methods and multimodal LLMs as tools and switches between text and image inputs (arXiv 2502.17812, v2 17 February 2026). — [arXiv 2502.17812](https://arxiv.org/abs/2502.17812)
- **Time-series foundation models:** Chronos (arXiv 2403.07815; v3 4 November 2024) scales and quantises values into a fixed vocabulary and trains T5-family models from about 20M to 710M parameters, performing comparably to or better than per-dataset-trained methods zero-shot on a 42-dataset benchmark. — [arXiv 2403.07815](https://arxiv.org/abs/2403.07815)
- **MCP content types:** tool results can contain text, base64 `image` (e.g., `image/png`), base64 `audio` (e.g., `audio/wav`), `resource_link` (a URI the client can fetch or subscribe to, not necessarily listed in `resources/list`) and embedded resources; all support annotations for `audience` and `priority`; `structuredContent` plus `outputSchema` returns validated JSON. — [MCP tools spec 2025-11-25](https://modelcontextprotocol.io/specification/2025-11-25/server/tools)
- **MCP subscriptions (2026-07-28):** `subscriptions/listen` opens one long-lived stream on which the server sends only opted-in notification types, including `notifications/resources/updated` for listed resource URIs; the server holds no subscription state across reconnects; notifications carry only the URI, so the client re-reads the resource. — [MCP subscriptions pattern](https://modelcontextprotocol.io/specification/2026-07-28/basic/patterns/subscriptions.md)
- **MCP caching and state:** list and read results carry `ttlMs`/`cacheScope`; cross-call state uses explicit handles; long-running work uses the tasks extension with `tasks/get` polling. — [MCP changelog 2026-07-28](https://modelcontextprotocol.io/specification/2026-07-28/changelog.md)
- **Keeping bulk data out of context:** with code execution, an agent filtered 10,000 spreadsheet rows down to five before the model saw them, and one workflow dropped from about 150,000 to 2,000 tokens; programmatic tool calling reduced roughly 200 KB of raw data to about 1 KB reaching the model. — [Anthropic: code execution with MCP](https://www.anthropic.com/engineering/code-execution-with-mcp), [Anthropic: advanced tool use](https://www.anthropic.com/engineering/advanced-tool-use)
- **Rerun** can serve recordings as queryable segments returning DataFrames, i.e., a ready query layer for slices. — [Rerun docs](https://rerun.io/docs/getting-started/what-is-rerun)

### Inferences
- "Evidence bundle" design (recommendation), ordered from cheapest to most expensive so the agent stops as early as possible:
  1. **Index (text, ~200–500 tokens):** session ID/handle, time span, modules present with firmware hashes, channels with units and sample rates, per-stream time uncertainty, list of events with timestamps.
  2. **Summary statistics (structured JSON):** per channel over the window — min/max/mean/std, first/last, slope, dominant frequency, count of threshold crossings, missing-data fraction; cross-stream lags from cross-correlation; rule evaluations that fired.
  3. **One aligned multi-panel figure (image):** shared x-axis in seconds relative to the trigger, one panel per stream class (scalars overlaid with units on labelled axes, bus-derived signals as step plots, audio as a mel spectrogram, event markers as vertical lines across all panels), rendered at a fixed modest resolution. This is the format the plots-vs-text study supports, and it makes cross-modal coincidence visible in one glance.
  4. **Zoom tools:** `evidence.render(session, t0, t1, channels, layout)` to re-render at higher time resolution on request.
  5. **Raw by reference:** `resource_link` URIs to the MCAP slice, a WAV clip and Parquet tables; a short audio clip as an MCP `audio` block only if the client model accepts audio.
- Token-cost reasoning: a numeric dump of even 10 s of 1 kHz data is tens of thousands of tokens, while one image is on the order of a thousand or so tokens depending on resolution (prior knowledge of vision-token pricing, unverified this session — check the current model documentation before quoting numbers). The 90% cost reduction reported in arXiv 2410.02637 is consistent with that order of magnitude.
- Time-series foundation models (Chronos; also TimesFM and Moirai from prior knowledge, unverified this session) are forecasting models: useful on the host as a cheap "expected vs observed" residual detector for slow telemetry (battery, solar, temperatures), not as an explanation layer. A 20M-parameter Chronos variant is plausibly runnable on a Pi 5 CPU for low-rate channels, but that is untested here.
- Audio: text-audio embedding models such as CLAP (prior knowledge, unverified this session) allow zero-shot tagging ("engine cranking", "relay click", "bearing whine") on the host and give the agent a label + confidence instead of audio tokens; spectrogram images remain the model-agnostic fallback.
- Always attach provenance to evidence: which module, firmware hash, calibration date, clock model and uncertainty. An agent that sees "12.1 V ±0.05 V at t = 3.214 s ±1 ms (module X, cal 2026-09)" reasons better than one that sees a bare number, and this is exactly what the manifest (A3) makes available.

### Gaps
- No fetched source with direct token-count comparisons for specific Claude models between images, text tables and audio; no fetched source for current image-token pricing.
- TimesFM, Moirai, CLAP and audio-LLM papers were not fetched this session; claims about them are prior knowledge.
- Full-text results of the VisualTimeAnomaly paper (which anomaly types multimodal LLMs handle best, robustness to missing data) were not available from the abstract page.
- No published "evidence bundle" standard for sensor data found; the design above is original synthesis.
- MCP guidance on size limits for base64 image/audio blocks and on chunked/binary resource reads in the 2026-07-28 revision was not fetched.

---

## B5. Record-and-replay for development and CI

### Takeaway
Because the recommended session format (MCAP) is the same thing a replay driver reads, record-and-replay falls out of the architecture: a "sim driver" implements the driver protocol from a recorded session, the gateway and MCP surface run unchanged, and GitHub Actions can run golden-session regression tests and the MCP conformance suite with no hardware. Linux already provides virtual CAN; the MCP project supplies a conformance harness and a traceability-file pattern worth copying for the module standard itself.

### Cited Findings
- **Virtual CAN:** the `vcan` driver provides CAN interfaces with no hardware, created with `ip link add type vcan`. — [Linux SocketCAN docs](https://docs.kernel.org/networking/can.html)
- **MCAP tooling for fixtures:** `mcap filter` extracts topics into a new file, `mcap merge` and `mcap sort` combine and order, `mcap doctor` validates structure, `mcap recover` repairs truncated files, `mcap add` attaches metadata. — [MCAP CLI guide](https://mcap.dev/guides/cli)
- **Rust MCAP reading:** streaming `MessageReader` with bounded memory and `IndexedReader` for random access. — [docs.rs mcap](https://docs.rs/mcap/latest/mcap/)
- **Rerun MCAP conversion with time offset:** `rerun mcap convert` supports `--timestamp-offset-ns`. — [Rerun MCAP reference](https://rerun.io/docs/reference/mcap)
- **Process-isolated drivers enable substitution:** Kismet datasources and Viam modules are separate processes speaking a fixed protocol, so a different binary can stand in. — [Kismet datasource docs](https://www.kismetwireless.net/docs/dev/datasources/), [Viam docs](https://docs.viam.com/build-modules/write-a-driver-module/)
- **MCP conformance suite:** the `modelcontextprotocol/conformance` repository holds scenario-and-check tests in TypeScript runnable by any implementation; tiers are measured against pinned conformance releases; a traceability YAML maps each normative statement to a check or documented exclusion; an earlier golden-trace proposal (SEP-1627) was *not* carried forward in favour of scenarios and checks. — [SEP-2484](https://modelcontextprotocol.io/seps/2484-conformance-tests-required-for-final-seps.md)
- **Deterministic tool lists:** MCP 2026-07-28 asks servers to return tools in deterministic order and forbids per-connection variation, which makes snapshot-testing the MCP surface practical. — [MCP tools spec 2026-07-28](https://modelcontextprotocol.io/specification/2026-07-28/server/tools.md)
- **Attested build outputs** (firmware/host binaries) can be verified in CI with `gh attestation verify`. — [GitHub Docs: artifact attestations](https://docs.github.com/en/actions/concepts/security/artifact-attestations)
- **Privacy by keeping data out of the model:** Anthropic describes tokenising personal data in the MCP client so it flows between systems without entering model context. — [Anthropic: code execution with MCP](https://www.anthropic.com/engineering/code-execution-with-mcp)
- **postcard-rpc transports are pluggable** (`WireTx`/`WireRx` traits, including TCP), so firmware logic can be exercised against a host-side transport. — [docs.rs postcard-rpc](https://docs.rs/postcard-rpc/latest/postcard_rpc/)

### Inferences
- Three replay levels, each catching different bugs:
  1. **Stream replay** (cheapest): sim driver reads channels from a golden MCAP and re-emits them to the gateway at recorded, scaled or as-fast-as-possible speed using a virtual clock. Tests the recorder, triggers, evidence rendering and MCP surface. Deterministic if all gateway code takes time from an injected clock rather than the OS.
  2. **Transaction replay** (for request/response instruments): the golden session stores request→response pairs for Modbus, OBD/K-line and SCPI; the sim driver answers matching requests and fails loudly on unseen ones ("cassette" style). For CAN, replay onto `vcan` so the real SocketCAN driver code runs.
  3. **Firmware-in-the-loop**: module-core firmware compiled for the host (or run in an emulator) behind a TCP/pipe `WireTx`/`WireRx`, so identify/health/self-test/update logic and the conformance harness run in CI.
- Golden-session regression tests: for each fixture, assert (a) the same trigger events fire at the same timestamps, (b) summary statistics match within tolerance, (c) rendered evidence images match a perceptual-hash tolerance, (d) the MCP tool list and manifests match snapshots. Store rule text and manifest hashes inside the session (B2/B3) so a fixture is self-contained.
- Fixture hygiene: keep fixtures small (seconds, a handful of channels) via `mcap filter`; store large ones with Git LFS or release assets; record the firmware hash and manifest version so stale fixtures are detected when a schema changes.
- Privacy scrubbing before publishing (no authoritative source found; engineering recommendations): GPS — drop, truncate precision or translate to a fixed synthetic origin; audio — publish spectrogram/feature channels instead of PCM where possible, otherwise manually review and cut speech segments (automatic voice-activity detection can flag candidates); video — blur faces/plates or publish only thermal/low-resolution derivatives; identifiers — hash or zero VINs in OBD frames, Wi-Fi/BLE MAC addresses and SSIDs in RF captures, serial numbers in manifests; IQ — note that wideband recordings can contain third-party communications and may be subject to local law. Implement scrubbing as a deterministic MCAP→MCAP transform with a manifest of what was removed, and never publish a session that has not passed it.
- CI matrix that needs no hardware: schema validation of all manifests; module-core conformance (firmware-in-the-loop); driver-protocol conformance against sim drivers; golden-session tests; MCP conformance suite against the gateway; attestation generation for release artifacts. Hardware-in-the-loop runs on the fixed bench as a self-hosted job only for release candidates.

### Gaps
- No fetched sources on deterministic-simulation testing practice in Rust (virtual clocks/schedulers), on emulators for ESP32/RP2040/nRF52 suitable for CI, or on `canplayer`/`candump` (the kernel page did not describe them).
- No authoritative sources found this session on privacy scrubbing of audio/video/GPS/RF recordings or on the legal status of publishing IQ captures.
- I did not verify whether the MCP conformance suite already supports the 2026-07-28 revision's stateless transport for server testing.

---

## Synthesis 1: draft outline of the module standard (layers, manifest, firmware services, conformance levels)

### Takeaway
A six-layer standard in which only the top three layers (identity, module-core RPC, manifest) are mandatory and everything below is "recommended profiles" keeps the spec short enough to adopt, lets off-the-shelf instruments participate through host-side "virtual" manifests, and puts the novel value — an agent-readable, safety-annotated datasheet — where no existing standard reaches.

### Cited Findings
- The on-module identity record can be as small as a UUID plus vendor and product strings plus the name of a host-side description (HAT+). — [Raspberry Pi HAT+ specification PDF](https://pip-assets.raspberrypi.com/categories/1215-raspberry-pi-hat/documents/RP-008281-DS-1-hat-plus-specification.pdf)
- A node should report health periodically, expose name/unique-ID/versions, accept parameters with min/max/default, support restart and firmware update, and take part in time sync (DroneCAN). — [DroneCAN specification, section 6](https://dronecan.github.io/Specification/6._Application_level_functions/)
- Affordances with `safe`/`idempotent`/`readOnly`/`unit` and a model/instance split already exist in a W3C Recommendation. — [W3C WoT Thing Description 1.1](https://www.w3.org/TR/wot-thing-description11/)
- MCP annotation defaults are pessimistic and annotations are untrusted hints; enforcement belongs in deterministic controls. — [MCP blog, 2026-03-16](https://blog.modelcontextprotocol.io/posts/2026-03-16-tool-annotations/)
- Interface hash as identity (postcard-rpc key = hash of path + schema). — [postcard-rpc overview.md](https://raw.githubusercontent.com/jamesmunns/postcard-rpc/main/docs/overview.md)
- Legacy sensors can be covered with a "virtual TEDS" file held by the measurement system. — [Wikipedia: IEEE 1451](https://en.wikipedia.org/wiki/IEEE_1451)
- Conformance should be executable, with each MUST traced to a check. — [SEP-2484](https://modelcontextprotocol.io/seps/2484-conformance-tests-required-for-final-seps.md)

### Inferences
**Layer 0 — Mechanical (recommended profile, not mandatory).** Footprints on a 42 mm Gridfinity-compatible grid in 1x1, 1x2, 2x2 sizes; maximum single printed part 168 mm (4 units) so it fits a 180 mm bed; connector on a defined face; label area with module ID, QR code to the manifest, and a risk-colour band (for example mains/high-voltage modules visibly distinct). Fixed-bench variant: DIN-rail clip adapter for the same footprints.

**Layer 1 — Electrical (two profiles).**
- *Profile U (primary):* USB-C receptacle or captive plug; USB 2.0 full/high speed; 5 V from the host hub; declared maximum current in the manifest and in the USB descriptor; host hub must have true per-port VBUS switching. Optional sync line profile (a 3.5 mm or 2-pin keyed connector carrying a 3.3 V sync edge) for modules that need sub-millisecond alignment.
- *Profile G (gateway-attached):* anything else (Qwiic 3.3 V I2C chains, RS485, CAN, K-line, ESP-NOW/BLE pods) attaches to a Profile U gateway module that enumerates children. Any project-defined power connector must be keyed and reverse-polarity protected.

**Layer 2 — Identification.** Every module has a 128-bit UID and a model ID `vendor.family.model` (reverse-DNS vendor, lowercase/digits/dot/dash/underscore) plus model version and manifest hash. Identity carriers, any one of: USB descriptors (VID/PID, serial string = UID, interface string naming the standard and version); module-core `identify` over the RPC; I2C or 1-Wire ID EEPROM holding UID + model ID + manifest hash (for passive boards behind a gateway); host-side binding file for unmodified commercial devices ("virtual" identity keyed on USB VID/PID/serial).

**Layer 3 — Module-core RPC (minimum firmware services).** Transport-independent, specified as message schemas; reference implementation in postcard-rpc over embassy-usb, COBS-serial fallback.
1. `identify` → UID, model ID, hardware revision, firmware semver, firmware image SHA-256, source repo URL + commit, interface (ICD) hash, manifest hash, standard version.
2. `describe` → manifest instance (or its hash + a chunked fetch) — may be omitted on tiny devices if the host can resolve by hash.
3. `health` (periodic topic, ≥1 Hz; host declares the module offline after a timeout) → state {ok, warning, error, critical}, mode {operational, initialising, maintenance, updating, safe}, uptime, supply voltage/temperature where available, error flags.
4. `selftest` → list of named checks with pass/fail/measured value/limits; declares whether it disturbs outputs.
5. `time` → four-timestamp exchange against the module's monotonic counter; optional latch-on-sync-pulse; all data topics carry the module counter value.
6. `config` → list/get/set/save/erase of named parameters with type, unit, min, max, default, and "requires restart"; set returns the value actually applied.
7. `log` (topic) → structured log lines (defmt-style) with level.
8. `update` → begin(size, hash) / write(offset, bytes) / commit / confirm-or-rollback after trial boot; refuses while outputs are active.
9. `reset` and `safe_state` → reboot; and an unconditional "all outputs to declared safe state" operation plus a host-heartbeat watchdog for any module with actuators or sources.
10. Module-specific endpoints/topics, each named in the manifest.

**Layer 4 — Host driver contract.** Out-of-process driver with `probe / open / describe / read / invoke / subscribe / health / close`; bulk data on a side channel; drivers for commercial devices ship the virtual manifest; sim drivers implement the same contract from recorded sessions.

**Layer 5 — Agent-facing manifest.** JSON, validated by a published JSON Schema (2020-12), structurally a WoT-TD-style document (properties, actions, events) with a project namespace for the additions. Required fields:
- *Identity:* `standard_version`, `model_id`, `model_version`, `title`, `description` (plain language: what it measures, what it must never be connected to), `vendor`, `license`, `links` (schematic, source, datasheets of key parts).
- *Instance:* `uid`, `hardware_rev`, `firmware {version, image_sha256, repo, commit, build_recipe, attestation_url}`, `icd_hash`.
- *Channels (properties):* for each — `name`, `quantity_kind`, `unit` (UCUM), `data_type`, `range {min, max}`, `resolution`, `sample_rate {min, max, default}`, `accuracy`/`uncertainty` (expression with conditions, e.g., ±(% of reading + offset), coverage factor), `calibration {date, method, reference, due, coefficients}`, `bandwidth`/`settling_time`, `timestamp_source {module_counter | host_receive | request_window}` and stated time uncertainty, `readOnly`.
- *Operations (actions):* for each — `name`, input/output schema, `risk_tier` (mapped to the project's MWS write-safety classes; at minimum: observe / configure-reversible / actuate-energy / destructive-or-irreversible / firmware), `safe`, `idempotent`, `reversible`, `preconditions`, `interlocks`, `requires_confirmation`, `effects` (what physically changes), `timeout`, `expected_duration`.
- *Events:* names, schemas, trigger semantics.
- *Limits:* `absolute_maximum_ratings` per terminal (voltage, current, RF input power, temperature), `isolation` (galvanic? rated voltage), `measurement_category` where relevant, `safe_state` definition, `host_watchdog_timeout`.
- *Power:* supply range, typical/peak current, inrush, battery behaviour.
- *Health & self-test:* list of health signals, self-test names, expected values.
- *Failure modes & troubleshooting:* structured list `{symptom, likely_causes, checks, remedy}` written for an agent.
- *Data streams:* topic names, message schema references, typical data rate (bytes/s), recommended retention class.
- *Time:* sync capabilities and expected accuracy.
- *Compliance:* conformance level claimed, test-report link, OSHWA UID if any.

**Conformance levels.**
- **L0 Described** — any device with a host driver and a schema-valid virtual manifest (covers RTL-SDR dongles, USB multimeters, OBD adapters, inverters over Modbus).
- **L1 Identified** — module carries UID + model ID + manifest hash on an identity carrier; host resolves and verifies the manifest automatically on hot-plug.
- **L2 Core** — implements all mandatory module-core services (identify, health, selftest, time, config, log, reset, safe_state where applicable) and passes the module-core conformance suite.
- **L3 Agent-maintainable** — adds `update` with rollback, reproducible firmware build with published attestation matching the reported image hash, a sim driver plus at least one golden session, and complete risk/limits/troubleshooting sections.
Each level's right to use the compatibility mark is tied to passing the public test suite for that level.

### Gaps
- The mapping from manifest `risk_tier` to the project's MWS classes depends on the MWS definition, which is outside this assignment's sources.
- Connector choice for the optional sync line and for any non-USB power rail is not researched.
- Uncertainty-expression syntax (how to encode "±(0.5% rdg + 2 counts) from 18–28 °C" machine-readably) needs its own design pass; no existing compact JSON convention was found this session.

---

## Synthesis 2: recommended fusion pipeline and the 8 decisions that are hard to change later

### Takeaway
One disciplined host clock, raw device timestamps plus recorded offset measurements, MCAP segments in a ring buffer with event-pinned pre/post windows, SigMF sidecars for IQ, Rerun or Lichtblick for humans, and a tiered evidence bundle (index → statistics → one aligned multi-panel image → raw by reference) for the agent. Eight choices — bus, mechanics, identity/licensing, manifest base, core RPC, host process model/MCP shape, time model, and session format — lock in early because modules, recordings and third-party implementations will all encode them.

### Cited Findings
- Record raw timestamps and offset measurements; correct post hoc. — [LSL time synchronization docs](https://labstreaminglayer.readthedocs.io/info/time_synchronization.html)
- chrony PPS configuration and slewing/stepping behaviour. — [chrony FAQ](https://chrony-project.org/faq.html)
- MCAP timestamps, chunks, optional indexes, attachments outside chunks, mandatory footer. — [MCAP specification](https://mcap.dev/spec)
- `mcap recover` for truncated files. — [MCAP CLI guide](https://mcap.dev/guides/cli)
- Rust `mcap` crate 0.26.0. — [docs.rs mcap](https://docs.rs/mcap/latest/mcap/)
- SigMF capture `datetime` and annotations. — [SigMF specification](https://sigmf.org/)
- Rerun opens MCAP and understands Foxglove Protobuf schemas; Lichtblick is MPL-2.0. — [Rerun MCAP reference](https://rerun.io/docs/reference/mcap), [Lichtblick on GitHub](https://github.com/lichtblick-suite/lichtblick)
- Plots beat text for multimodal models at up to 90% lower cost. — [arXiv 2410.02637](https://arxiv.org/abs/2410.02637)
- MCP image/audio/resource-link content, stateless handles, subscriptions stream, tasks extension. — [MCP tools spec 2026-07-28](https://modelcontextprotocol.io/specification/2026-07-28/server/tools.md), [MCP changelog 2026-07-28](https://modelcontextprotocol.io/specification/2026-07-28/changelog.md)
- Tool-definition token costs and deferred loading savings. — [Anthropic: advanced tool use](https://www.anthropic.com/engineering/advanced-tool-use)
- Pi 5 USB ports are power-ganged; per-port control needs a suitable external hub. — [uhubctl README](https://raw.githubusercontent.com/mvp/uhubctl/master/README.md)

### Inferences
**Recommended fusion pipeline**
1. **Clock.** Host clock disciplined by chrony; optional GNSS PPS on a GPIO for absolute time and multi-host agreement; RTC for cold starts off-grid. Session time = nanoseconds since Unix epoch, with monotonic-raw stored alongside. Per-stream clock model (offset + drift, with uncertainty) fitted from periodic offset samples; never overwrite raw device timestamps. A sync-event module (LED + click + electrical edge) fired at session start/end and on demand to measure per-stream latency.
2. **Bus (internal data plane).** Drivers push timestamped messages to the gateway over local IPC; the gateway fans out to recorder, trigger engine, live viewer and MCP subscriptions. No network middleware is required on the pack; Zenoh or MQTT bridges are optional for the fixed bench and Home Assistant.
3. **Store.** Rotating MCAP segments (zstd chunks, flushed per chunk) in a ring buffer with per-stream retention classes; IQ in SigMF sidecars referenced from MCAP; manifests embedded and duplicated as JSON; clock-offset samples and trigger rules stored as channels/metadata; `recover` on startup for any unfinalised segment; derived Parquet for scalar analytics.
4. **Triggers.** Declarative rules over channels (threshold, rate, band energy, bus-frame match, timeout, sequences within a window) evaluated by a streaming Rust engine, plus module-raised and agent-raised events; a trigger pins `[t − pre, t + post]` across all streams into a persistent session and writes an event record.
5. **Evidence bundle.** Index → statistics (including cross-stream lags and time uncertainty) → one aligned multi-panel image (scalars, bus-derived steps, spectrogram, event markers on a shared axis) → zoom re-renders → raw slices by `resource_link`. Capture and session handles are explicit tool arguments; long captures run as MCP tasks; live status via `subscriptions/listen`.
6. **Replay.** The same sessions drive sim drivers for CI and for contributors without hardware; published fixtures pass a deterministic privacy-scrub transform first.

**The 8 decisions to make early (hard to change later)**
1. **Primary module bus and power.** USB-C/5 V with a per-port-switched hub as the one mandatory wired profile (gateways for everything else) versus a CAN FD + power backbone. Every module PCB, enclosure and the hub BOM depend on it.
2. **Mechanical grid and connector face.** 42 mm grid, module size classes, which face carries the connector, and label/QR placement, constrained to parts ≤168 mm for the 180 mm printer. Printed trays and third-party modules freeze this.
3. **Identity, naming and licensing.** UID format, `vendor.family.model` namespace and registry repository, USB VID/PID route (pid.codes), the standard's name/mark rules, and the licence trio for spec/hardware/code. IDs get burned into devices and recordings; licences are near-impossible to change after outside contributions.
4. **Manifest schema base and vocabulary.** WoT-TD-style JSON with the project namespace, UCUM units, quantity-kind list, the risk-tier enumeration and its MWS mapping, and semver rules. Every module, driver, recording and agent prompt will embed it.
5. **Module-core RPC and schema evolution.** Wire format (postcard-rpc keying), the mandatory service list, how new fields are added without breaking old firmware, and the interface-hash rule. Deployed firmware in the field speaks this indefinitely.
6. **Host driver process model and MCP surface shape.** Out-of-process driver protocol, and a fixed meta-tool MCP surface (module/operation as arguments, discovery by search) versus generated per-instrument tools. Driver authors and agent prompts/skills will both be written against it; MCP's stateless revision makes the handle-based design the durable one.
7. **Time model.** Epoch and resolution, "raw device time + recorded offsets" versus corrected-at-capture, how request-window uncertainty is represented, and the sync-event convention. Recordings made under one model cannot be fully converted to another.
8. **Session format and topic conventions.** MCAP layout, segment rotation, channel naming (`/module_uid/channel` or similar), message encodings/schemas (Protobuf with viewer-compatible schemas versus JSON versus postcard), and the sidecar rule for IQ/video. Golden sessions, replay drivers, viewers and published datasets all depend on it.

### Gaps
- The assignment's cost ceiling (USD 1,000) was not costed here; hub, GNSS and sync-module prices were not researched (the only prices found were Geerling's ~USD 200 TimeHAT and ~USD 195 GNSS module, which are beyond what the pack needs).
- Whether the project's MWS standard already defines risk tiers that should replace the tier names sketched above is unknown to this researcher.
- Because the web-search budget was exhausted, third-party MCP gateways, wireless time-sync measurements, privacy-scrubbing practice, and several self-description standards (SensorML, OPC UA, Matter, SCPI/LXI/IVI, UCUM, CycloneDX/SPDX) remain unverified; a follow-up search pass on those items is recommended before the final report treats the corresponding "prior knowledge" lines as fact.
