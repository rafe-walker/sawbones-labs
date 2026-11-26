# Vehicle diagnostic interfaces (OBD-II / CAN / K-line) for an agent-operated portable test bench

Research date: 2026-11-25. Prices are USD as shown on the cited page on that date. "Prior knowledge" marks things I did not re-verify against a source this session; treat those as unverified.

Owner context used (from the coordinator's brief plus stored user memory): the primary vehicle is a 2003 Toyota Tacoma Double Cab, 3.4L V6 5VZ-FE, originally a 2WD automatic PreRunner that the owner converted to manual transmission and 4WD. The owner lives in Cochise County, Arizona.

## Objective: recommended vehicle interface (primary + budget) and read/gate tiers

### Takeaway
Buy an STN-based adapter, not an ELM327 clone: the OBDLink EX (USB, $69.95) is the dependable primary for a Pi-hosted pack, the WiCAN Pro ($89 + shipping) is the open-firmware alternative, and a vLinker MC or a forum-verified Veepeak/BAFX dongle is the budget fallback. No adapter in the primary or alternative slot was found forum-verified on a 1st-gen Tacoma; the adapters with direct owner reports on 1st-gen trucks are BAFX, Carista, PLX Kiwi 3, Veepeak Mini and UltraGauge.

### Cited Findings
- OBDLink EX: $69.95, in stock, free US shipping, USB 2.0, covers ISO 15765-4, ISO 14230-4, ISO 9141-2, J1850 VPW, J1850 PWM and Ford MS-CAN; 8-30 V operating range, 6.5 mA at 12 V, sleep draw under 1 mA, overvoltage protection to 100 V. Vendor lists Android and Windows only (Linux not mentioned). — [OBDLink EX product page](https://www.obdlink.com/products/obdlink-ex/)
- OBDLink line-up prices: CX $79.95 (BLE), EX $69.95, LX $89.95, MX+ $139.95. — [OBDLink products](https://www.obdlink.com/products/)
- OBDLink protocol coverage: MX+ and EX and LX each cover all five legislated protocols (MX+ adds MS-CAN and SW-CAN, EX adds MS-CAN); the CX covers only CAN, KWP2000 and ISO 9141-2 (no J1850). MX+ and LX are Bluetooth v3.0 (classic), CX is BLE. — [OBDLink adapter comparison](https://support.obdlink.com/support/solutions/articles/43000713351)
- OBDLink SX (the old $49.95 USB model) is discontinued and out of stock. — [OBDLink SX page](https://www.obdlink.com/products/obdlink-sx/)
- WiCAN Pro: $89 before shipping; ESP32-S3 running open-source firmware plus a separate "dedicated OBD chip" (not named); ISO 9141-2, ISO 14230-4 slow and fast init, J1850 PWM and VPW, ISO 15765-4, J1939, raw CAN, MS-CAN, GMLAN single-wire, WWH-OBD. — [MeatPi WiCAN Pro](https://www.meatpi.com/products/wican-pro)
- WiCAN Pro shipping from Crowd Supply is $8 to the US; it speaks ELM327 v2.3, ELM329 v2.2, STN and VT instruction sets; has WiFi, BLE, USB, SD-card logging, IMU, slcan/SocketCAN output; the USB-C port cannot power the unit, which must be plugged into an OBD port. — [Crowd Supply WiCAN Pro](https://www.crowdsupply.com/meatpi-electronics/wican-pro)
- WiCAN firmware is GPL v3; the Pro supports MQTT and MQTTS, HTTP/HTTPS endpoints with an HTTP API reference, automation scripts, vehicle profiles and custom PIDs, sleep under 1 mA with configurable voltage threshold and wake on voltage or motion. The original (non-Pro) WiCAN has no K-line or J1850. — [meatpiHQ/wican-fw](https://github.com/meatpiHQ/wican-fw)
- Vgate vLinker MC WiFi: $36.99 (list $44.90), sold in US and Canada only; J1850 PWM/VPW, ISO 9141-2, ISO 14230-4 slow and fast, ISO 15765-4, raw CAN, J1939, SW-CAN, MS-CAN; accepts both ELM AT commands and STN ST commands. — [Vgate vLinker MC WiFi](https://vgatemall.com/products-detail/i-37)
- Owner reports on 1st-gen Tacomas: BAFX works on a 1996 3RZ; Carista (Bluetooth) works on a 1999 3RZ with DashCommand, OBDFusion and EngineLink; PLX Kiwi 3 works on a 2000 V6; Veepeak Mini Bluetooth works on a 2000 4WD; UltraGauge works on 1995.5/1996 trucks. One 1995 3.4L owner had several dongles advertised as "9141 compatible" fail. — [Tacoma World 1st-gen dongle thread](https://www.tacomaworld.com/posts/30109781)

### Inferences
- Recommended primary: OBDLink EX over USB. The computer is inside the pack, so a wired serial link removes Bluetooth pairing and RF problems, and the STN chip family carries the extended ST command set for adapter introspection. Status on K-line Toyota: assumed from the vendor's protocol claim, not forum-verified in this research.
- Recommended open alternative: WiCAN Pro. It is the only shipping, multi-protocol (K-line + J1850 + CAN) adapter found whose host firmware is open source, and it already exposes MQTT/HTTP that an MCP server can wrap. Two caveats: the OBD interpreter chip itself is a closed part, and no K-line Toyota field report was found.
- Budget: vLinker MC WiFi ($36.99) if STN-style commands matter; a Veepeak Mini or BAFX Bluetooth dongle if the only goal is the cheapest unit with an actual 1st-gen Tacoma owner report. Their current prices were not retrieved (prior knowledge puts both under $30).
- Best fit inside the $30-120 vehicle share: EX alone ($69.95), or WiCAN Pro alone (about $97 delivered). Buying both exceeds the share (about $167).
- Bluetooth is the weakest transport for this design. BLE-only OBDLink CX also drops J1850, so it is the wrong OBDLink for a multi-vehicle legacy tool.
- Proposed access tiers for the agent (recommendation, not sourced):
  - Tier 0, free: adapter-only commands that never touch the bus (identity, firmware version, supply voltage, current protocol, settings dump).
  - Tier 1, free with ignition on: Mode 01 live data and readiness status, Mode 02 freeze frame, Mode 03 stored DTCs, Mode 07 pending DTCs, Mode 09 vehicle info, Mode 06 monitor results where supported, passive bus monitoring (listen-only CAN, K-line sniffing).
  - Tier 2, ask first: forcing protocol or init type, custom headers, manufacturer-specific read-only requests (Toyota enhanced data, UDS ReadDataByIdentifier, KWP ReadDataByLocalId), high-rate polling while the engine is running, any raw frame transmit on a CAN bus.
  - Tier 3, human confirmation every time, vehicle stationary: Mode 04 clear, Mode 08 on-board control, UDS/KWP IO control and routines (actuator tests), security access, session changes beyond default.
  - Tier 4, not exposed: ECU reprogramming, write-memory, coding/variant writes, immobilizer or key functions, any transmit while moving.
  - Enforce the tiers in the MCP server (allow-list of services by mode byte), not in the prompt, and log every transmitted byte.

### Gaps
- Linux behaviour of the OBDLink EX (which USB-serial chip, device node, baud) was not confirmed from a source; the vendor lists only Android and Windows.
- Current Amazon prices for Veepeak Mini, BAFX, Carista, PLX Kiwi 3 and vLinker FS USB / MC+ were not retrieved.
- No source names the OBD interpreter chip inside the WiCAN Pro.
- No field report was found for OBDLink EX/LX/MX+ or WiCAN Pro on a 1996-2004 Tacoma.

## Which OBD-II protocol does a 2003 Tacoma 3.4L use, what is available, and is Techstream-level data feasible?

### Takeaway
The evidence points to ISO 9141-2 on the K-line (DLC3 pin 7, 10.4 kbaud), with no CAN, so the truck is limited to the generic J1979 services at a few requests per second. Techstream over a Mini VCI is possible in principle but looks marginal on 1st-gen trucks, and no open-source Linux path to it was found.

### Cited Findings
- Tacoma World members state the truck uses ISO 9141 on early models and communicates on pin 7 (SIL, K-line); for a 2003 Tacoma the populated pins were given as 4 (chassis ground), 5 (signal ground), 7 (SIL), 13 (TC) and 16 (battery via the 7.5 A OBD fuse). That thread's vehicle is a 2003 2.7L. — [Tacoma World: 1st gen OBD2 communication error](https://www.tacomaworld.com/threads/1st-gen-obd2-communication-error.865784)
- On a 2003 3.4L truck, pin 4 is chassis ground, pin 5 is sensor ground and pin 16 is constant power; a Craftsman 14063 generic scanner had previously communicated with that truck. — [Tacoma World post, 2003 3.4L no-communication](https://www.tacomaworld.com/posts/29185082)
- A third-party compatibility table lists Tacoma 2000 and 2002 as "ISO 14230-4, ISO 9141-2" and a 2003 Tundra 4.7 as ISO 9141. The same table shows the Toyota DLC with pin 7 as SIL (K-line), pin 13 as TC, pin 9 as a tach signal (marked uncertain), pins 6/14 CAN on later models and pins 2/10 J1850 on earlier models only. The table labels itself incomplete. — [pinouts.ru Toyota OBD-II](https://pinouts.ru/CarElectronics/Toyota_lexus_obd2_diagnostic.shtml)
- ISO 9141-2 runs on pin 7 (optional L-line pin 15) as 10,400 bps 8-N-1 UART idling high; CAN on pins 6/14 became universal on US vehicles from 2008. — [SparkFun OBD-II protocols](https://learn.sparkfun.com/tutorials/getting-started-with-obd-ii/obd-ii-protocols)
- K-line throughput: the author of a direct K-line library measured up to 20 requests per second on one car versus at most 6 readings per second through a Bluetooth OBD dongle; the ECU response wait dominates each request. — [OBD9141](https://github.com/iwanders/OBD9141)
- python-OBD implements Mode 06 only for CAN (ISO 15765-4), so monitor test results are not available through it on a K-line vehicle. — [python-OBD command tables](https://python-obd.readthedocs.io/en/latest/Command%20Tables/)
- Techstream cables: the Mini VCI v1.4.1 carries a K-line chip and is the version recommended for K-line Toyotas; v2.0.4 clones have bugs on K-line cars; the Xhorse MVCI Pro failed to connect to a K-line-only 2002-2005 Land Cruiser 100; VXDIAG has only partial K-line support; the Tactrix Openport 2.0 has problems with some K-line devices. This is a cable vendor's blog. — [obdii365: which Techstream cable works on K-line Toyota](https://blog.obdii365.com/2023/12/20/which-techstream-cable-work-on-k-line-toyota/)
- A 2002 Tacoma PreRunner double cab owner using a Mini VCI v2.0.4 saw Techstream V10, V12 and V14 crash during the health check, saw only "Engine" and "SRS Airbag" systems listed, and reported that a cheap OBD-II reader showed more codes than Techstream. — [Tacoma World Techstream thread](https://www.tacomaworld.com/posts/24310841)

### Inferences
- The 2003 3.4L protocol is not stated outright by any single source found, but the 2003 2.7L report (pin 7, ISO 9141), the 2000/2002 Tacoma table rows, the 2003 Tundra row and the pre-2008 CAN timeline all agree. Confidence is high that the truck is K-line; whether it answers 5-baud ISO 9141-2 init, KWP2000 fast init, or both should be confirmed on first connection by reading the adapter's reported protocol number.
- Pins 6 and 14 should be empty on this truck, so raw CAN tools (CANable, panda, PCAN) contribute nothing to it.
- Expect only generic Mode 01 data from a 5VZ-FE ECU: RPM, speed, coolant and intake temperature, MAF, throttle position, short and long fuel trims per bank, O2 sensor voltages, timing advance, calculated load, fuel system status, plus Modes 02, 03, 07 and readiness. This list is prior knowledge, not sourced; the actual supported-PID bitmap (Mode 01 PID 00/20/40) must be read from the truck.
- Realistic polling budget is roughly 3-6 PIDs per second through an ELM/STN-style adapter. The agent should poll a small prioritised set (for example RPM plus both short-term trims) and not a full dashboard.
- Because the truck was converted from automatic to manual, the ECU may be an automatic-transmission calibration. Transmission-related DTCs or implausible gear/shift data could be artefacts of the swap; the agent should be told this before interpreting codes.
- Pin 13 (TC) jumpered to ground is Toyota's legacy blink-code path for ABS and SRS on this era of truck (prior knowledge). A relay-switched TC jumper with an optical pickup on the warning lamps would give the agent body-system codes that generic OBD cannot reach. This is a write-like action and belongs in a gated tier.
- Techstream-level data is not a good fit for this project: Techstream is Windows software, the reliable cable is a specific clone revision, and the one 1st-gen Tacoma report found was a failure.

### Gaps
- No source found that names the protocol for the 2003 3.4L specifically, or the list of supported PIDs for the 5VZ-FE.
- No documentation of Toyota enhanced (manufacturer-specific) K-line requests for this ECU was found, and no open-source decoder for them.
- No working open-source J2534 stack for Mini VCI on Linux was found. A GitHub resource list exists ([jakka351/OpenJ2534](https://GitHub.com/jakka351/OpenJ2534)) but was not reviewed.
- The pinouts.ru row for a 2006 Tacoma 4.0 ("KWP FAST") conflicts with the common understanding that 2nd-gen Tacomas use CAN; treat that table as indicative only.

## Adapter options and their openness

### Takeaway
There are three workable families: closed but well-documented STN adapters (OBDLink, Vgate vLinker), the open-firmware WiCAN Pro wrapped around a closed OBD chip, and fully open but CAN-only hardware (CANable, panda). Nothing fully open and mature covers K-line and J1850 today.

### Cited Findings
- STN2120 supports the five legislated protocols plus raw CAN, GMLAN single-wire CAN, Ford MS-CAN and J1939; it accepts the ELM327 AT command set plus an extended ST command set; it has a battery-voltage sense input with a calibration offset (STVCAL) and voltage-change wake trigger (STSLVG), sleep/wake triggers, a secure bootloader for firmware updates, and UART rates up to a theoretical 8 Mbps. Command details live in a separate reference manual. — [STN2120 datasheet](https://www.scantool.net/scantool/downloads/206/stn2120-ds.pdf)
- STN1110 supports the five legislated protocols, raw CAN and J1939, with ELM327 compatibility and the ST command set. — [STN1110 datasheet](https://cdn.sparkfun.com/assets/6/4/9/f/6/stn1110-ds.pdf)
- Fake ELM327 units reporting "v2.1" were found to contain a single unmarked 16-pin chip doing everything, were described as extremely buggy and not feature-complete, and cannot be told apart from listing photos; a genuine design uses a separate microcontroller, Bluetooth chip and CAN transceiver. The article does not say which protocols are missing. — [CNX Software: fake ELM327 adapters](https://www.cnx-software.com/2022/08/27/beware-of-fake-elm327-obd-ii-bluetooth-adapters/)
- OpenDiag: $60 on Tindie, ESP32-S3, lists CAN, ISO 9141-2 and J1850 PWM/VPW with USB and BLE and SLCAN raw CAN; seller based in Los Gatos, California. — [Tindie OpenDiag](https://www.tindie.com/products/opendiag/opendiag-open-source-obd2-interface/)
- OpenDiag firmware is GPL-3.0, ESP-IDF based, exposes two USB serial ports (data link plus debug console) and has voltage sensing; KWP2000 (ELM protocols 4 and 5), J1939 and automatic protocol search are not implemented; the repository shows one commit, no stars and no releases. — [maximus64/opendiag-firmware](https://github.com/maximus64/opendiag-firmware)
- comma red panda: $99, out of stock, CAN FD on all buses, free US shipping; firmware MIT licensed. — [comma shop: panda](https://comma.ai/shop/panda)
- panda speaks CAN and CAN FD (no K-line or LIN mentioned), firmware is built with vehicle-specific safety logic, and transmitting requires explicitly setting a safety mode; Linux is supported with udev rules. — [commaai/panda](https://github.com/commaai/panda)
- CANable 2.0 is the current model with CAN and CAN FD; stock firmware is slcan (serial), alternative candleLight firmware makes it a native gs_usb SocketCAN device but without FD frames; sold through Openlight Labs. — [canable.io](https://canable.io/)
- OBD9141 is an MIT-licensed Arduino/Teensy/ESP32 library for ISO 9141-2 and KWP over K-line using MC33290, SN65HVDA100 or SN65HVDA195 transceivers; it reads and clears DTCs and includes an ECU simulator class; it was tested on a single 2004 Kia. — [OBD9141](https://github.com/iwanders/OBD9141)
- The Mini VCI is described by a Tacoma World member as K-line only, not CAN. — [Tacoma World Techstream thread](https://www.tacomaworld.com/posts/24310841)

### Inferences
- Verified versus assumed on K-line Toyotas:
  - Verified by 1st-gen Tacoma owners: BAFX, Carista, PLX Kiwi 3, Veepeak Mini Bluetooth, UltraGauge, Craftsman 14063 handheld.
  - Assumed from vendor protocol lists: OBDLink EX/LX/MX+/CX, vLinker MC/FS/MC+, WiCAN Pro.
  - Assumed and immature: OpenDiag (ISO 9141-2 only on K-line, no auto search).
  - Reported failing: unnamed ELM327 clones on a 1995 3.4L; Mini VCI v2.0.4 with Techstream on a 2002.
  - Not applicable to this truck: CANable, panda, PCAN-USB, CANtact, original WiCAN (CAN only).
- A DIY K-line tap (ESP32 or Teensy + one K-line transceiver, OBD9141 as a starting point) is the only route to fully open firmware on the Tacoma. It costs a few dollars in parts (prior knowledge) and would let the agent see raw bytes and timing, and it could reach roughly three times the request rate of a dongle. It is a development project, not a purchase.
- Adapter self-inspection through the ELM/STN command set gives the agent identity and version strings, measured supply voltage, the negotiated protocol and bus monitoring. python-OBD's ELM_VERSION and ELM_VOLTAGE commands confirm the first two exist; the exact ST commands were not confirmed from the reference manual this session.
- The WiCAN Pro adds something the STN dongles lack: the agent can also inspect the adapter's own host firmware state (WiFi, MQTT, sleep thresholds, logs) through HTTP, and the firmware can be modified.
- For 2008+ vehicles a second, CAN-only interface is useful for passive sniffing. The WiCAN Pro's slcan mode covers this without another purchase.

### Gaps
- Not researched to a citable level this session: Macchina M2/A0/P1 availability, Freematics ONE+ protocols and price, CANtact Pro, Carloop, PEAK PCAN-USB price, CANable 2.0 price and stock, Teensy-based projects, ESP32 ELM327-emulator firmware projects, L9637D-based builds.
- Whether any shipping red/white panda variant still has a K-line/LIN driver was not confirmed; current documentation mentions only CAN and CAN FD.
- The OBDLink ST command reference manual (FRPM) was not fetched, so specific ST commands for filters, monitoring and voltage are not cited here.

## Linux and Rust software, and which outputs suit wrapping

### Takeaway
For the K-line Tacoma there is no mature Rust crate; the practical route is a thin Rust ELM327/STN serial driver of the project's own, with python-OBD as a reference implementation. For CAN vehicles the Rust ecosystem is good: the `automotive` and `ecu_diagnostics` crates over SocketCAN, plus the kernel ISO-TP and J1939 sockets.

### Cited Findings
- `automotive` 0.6.0 (MIT): async CAN and CAN FD adapter interface for SocketCAN (Linux), comma panda, PEAK PCAN-USB FD, J2534 (Windows only) and Vector XL; ISO-TP and DoIP transports; UDS, XCP and CCP clients; Python bindings. — [docs.rs automotive](https://docs.rs/automotive/latest/automotive/)
- `ecu_diagnostics` (latest 0.107.5, 2026-08-21; docs.rs build of that version failed): OBD2 (services 01, 02, 09 working), KWP2000 and UDS; hardware back-ends J2534 passthru (with battery-voltage read), SocketCAN, SLCAN and PCAN-USB; transports are CAN, ISO-TP and VW TP2, with LIN and J1850 marked work in progress and K-line not listed. — [docs.rs ecu_diagnostics](https://docs.rs/ecu_diagnostics/)
- `obd2` crate: ELM327 only, over an FTDI serial link; latest release 0.2.0-pre3 dated May 2023; about 32 downloads a month. — [lib.rs obd2](https://lib.rs/crates/obd2)
- A `socketcan-isotp` crate exists for kernel ISO-TP sockets. — [lib.rs socketcan-isotp](https://lib.rs/crates/socketcan-isotp)
- The kernel's ISO-TP socket (CAN_ISOTP) handles segmentation and flow control so applications exchange only payloads; it is the transport for UDS on CAN and for emissions diagnostics. — [Linux kernel ISO 15765-2 docs](https://docs.kernel.org/networking/iso15765-2.html)
- python-OBD targets ELM327 adapters, is described as suitable for Raspberry Pi, returns unit-bearing values through Pint, has an async connection class and custom commands. — [python-OBD docs](https://python-obd.readthedocs.io/en/latest/)
- python-OBD command coverage: Modes 01, 02, 03, 04, 06 (CAN only, experimental), 07, 09 (experimental), plus adapter commands ELM_VERSION and ELM_VOLTAGE; CLEAR_DTC erases codes and freeze frame. — [python-OBD command tables](https://python-obd.readthedocs.io/en/latest/Command%20Tables/)
- OpenVehicleDiag is a Rust, GPL-3.0 diagnostic application using J2534-2 and SocketCAN on Linux with KWP2000, UDS and OBD; its latest listed release is 1.0.5 from May 2021. — [rnd-ash/OpenVehicleDiag](https://github.com/rnd-ash/OpenVehicleDiag)
- AndrOBD documents the factors that set update speed (adapter timeout, protocol, baud rate, ECU response time) and offers ELM adaptive timing modes or a software-learned timeout. — [AndrOBD wiki: data item update time](https://github.com/fr3ts0n/AndrOBD/wiki/Data-item-update-time)

### Inferences
- Suggested stack: one Rust "ELM/STN serial" back-end (serial port or TCP socket, text protocol) for all legacy-protocol vehicles, and one SocketCAN back-end (`socketcan` + kernel ISO-TP, or `automotive`) for 2008+ vehicles. Both feed the same typed PID/DTC decoder and the same MCP tool surface.
- The ELM text protocol is trivially machine-readable (hex lines), which makes it easy to wrap, to record and replay, and to expose raw to the agent for troubleshooting.
- `ecu_diagnostics` is the closest thing to a Rust KWP2000/UDS reference, but its KWP2000 runs over CAN transports, so it does not talk to the Tacoma's K-line.
- Machine-readable outputs suitable for wrapping (prior knowledge, not re-verified): can-utils `candump -L` log lines, SocketCAN frames directly, opendbc DBC files for decoding manufacturer CAN signals, Torque-style custom PID CSV files as a community source of enhanced PIDs, rtl_433 JSON. SavvyCAN, Cabana and CANdevStudio are GUI tools better suited to a human at a desk than to an agent.

### Gaps
- Not fetched this session, so not cited: can-utils, the `socketcan` crate's current version, udsoncan, pyOBD, SavvyCAN, Cabana/opendbc, CANdevStudio, OVMS, Torque PID file format.
- No Rust crate was found that implements ISO 9141-2 / KWP2000 directly on a UART K-line transceiver.
- The kernel version in which CAN_ISOTP became mainline is not stated in the fetched page (prior knowledge: 5.10).

## Simulators and test fixtures

### Takeaway
Software CI needs no hardware: the Python ELM327-emulator presents a fake adapter on a pty or TCP port. For bench tests of a physical adapter, a hardware ECU simulator covering K-line costs about $155, or can be built for a few dollars from the OBD9141 simulator class.

### Cited Findings
- ELM327-emulator simulates an ELM327 adapter and multiple ECUs; it exposes a pseudo-terminal, a TCP port (35000 is conventional), a Bluetooth RFCOMM server, a serial port, an SLCAN interface and a K-line interface; it includes a "car" scenario modelled on a Toyota Auris Hybrid, supports ISO-TP, UDS and the KWP2000 data link layer, has a plugin framework and has been tested with python-OBD. The licence badge reads CC BY-NC-SA 4.0. — [Ircama/ELM327-emulator](https://github.com/Ircama/ELM327-emulator)
- Tindie OBD-II ECU simulator: $155 (1-4 units), simulates ISO 9141-2, ISO 14230-4 fast and CAN 250/500k standard/extended (no J1850), PIDs 0x00-0x5F adjustable through configuration software, J1979 Modes 1-9 and A, ships from Taiwan. — [Tindie OBD-II ECU simulator](https://www.tindie.com/products/kevinliang/obd-ii-ecu-simulator-2/)
- Freematics OBD-II Emulator MK2 emulates CAN 250/500k (11/29-bit), ISO 9141-2, KWP2000 fast and 5-baud, and J1850 PWM, with adjustable PIDs, settable DTCs and VIN, a USB GUI and a serial TTL control port; DFRobot lists it as discontinued. — [DFRobot Freematics emulator](https://www.dfrobot.com/product-1352.html)
- OBD9141 ships a simulator class alongside its reader class for K-line testing. — [OBD9141](https://github.com/iwanders/OBD9141)

### Inferences
- GitHub Actions plan: run ELM327-emulator on a TCP port in the job, point the Rust ELM/STN back-end at it, and assert on decoded PIDs, DTCs, freeze frame and readiness. For the CAN back-end use a virtual CAN interface (`vcan`) with a small responder (prior knowledge; hosted runners may need the vcan module loaded).
- The emulator's non-commercial share-alike licence is fine as a CI dependency invoked at arm's length, but its code should not be copied into a permissively licensed repository. Check the LICENSE file before relying on this.
- A recorded-transcript fixture (capture real request/response lines from the Tacoma once, replay in tests) is the cheapest high-fidelity K-line fixture and captures the truck's real timing and quirks.
- A $10-class bench fixture: Teensy or ESP32 plus K-line transceiver running the OBD9141 simulator, powered from the bench supply at 12 V, gives the physical adapter something to initialise against, including 5-baud init.

### Gaps
- Current price and stock of the Freematics emulator from Freematics' own store were not retrieved.
- No cheap (under $50) off-the-shelf hardware simulator covering K-line was confirmed.
- Whether ELM327-emulator's stock scenarios include an ISO 9141-2 vehicle with realistic timing was not confirmed.

## Beyond OBD for an old truck

### Takeaway
On a 2003 K-line truck the ECU gives a thin, slow view, so direct sensing adds more diagnostic value per dollar than a better OBD adapter. Only TPMS reception was sourced this session; the remainder of this section is engineering judgement.

### Cited Findings
- rtl_433 with an RTL-SDR decodes TPMS broadcasts; one user found three of four sensors transmitting and located the dead one by comparing RSSI at each wheel; TPMS commonly transmits around 315 MHz. — [rtl-sdr.com: testing TPMS sensors with rtl_433](https://www.rtl-sdr.com/?p=55166)
- rtl_433 decoded the sensors of a 2008 Toyota Tacoma; commenters note sensors send pressure, optionally temperature, and a unique ID. — [Hackaday: SDR listens in to your tires](https://hackaday.com/2022/04/09/sdr-listens-in-to-your-tires)
- The Toyota DLC lists pin 9 as a tachometer signal, marked uncertain. — [pinouts.ru Toyota OBD-II](https://pinouts.ru/CarElectronics/Toyota_lexus_obd2_diagnostic.shtml)

### Inferences
- A 2003 Tacoma predates the US TPMS mandate (prior knowledge), so rtl_433 TPMS is useful for other vehicles, not this truck, unless aftermarket sensors are fitted.
- Highest-value additions for a "fuel or spark first" diagnostician, in rough order (all prior knowledge, unsourced):
  - Battery and charging analysis from the 12 V socket or battery posts: resting voltage, cranking dip, alternator ripple (AC component reveals a failed diode), relative compression from cranking current or voltage ripple. Needs only an ADC sampling at a few kHz.
  - Fuel pressure transducer (0-100 psi, 0.5-4.5 V ratiometric) teed into the rail: key-on prime, running, and leak-down.
  - Inductive pickup or capacitive clamp on a plug wire or coil primary for RPM and per-cylinder spark presence, independent of the ECU.
  - Low-amp current clamp on injector and coil circuits read by a scope channel: injector pintle hump, coil dwell and saturation.
  - Manifold vacuum/MAP transducer: cranking vacuum, idle stability, valve problems show as periodic pulses.
  - Wideband O2 controller with serial output in the tailpipe (clamp-on) for true lambda; the stock narrowband data over K-line is slow.
  - Accelerometer on the block or frame plus a contact microphone: misfire and bearing signatures via order analysis against RPM.
  - K-type thermocouples (exhaust runners for per-cylinder contribution, radiator in/out, catalyst in/out).
- If DLC pin 9 really carries a tach signal on this truck, it is a zero-cost RPM source that works even when OBD communication is down. Verify with a scope before relying on it.

### Gaps
- 14point7 Spartan pages failed to load, so wideband controller model, serial format and price are not cited.
- No sources were gathered for fuel pressure transducers, ignition pickups, current clamps, ripple analysis or vibration analysis; prices and part numbers are missing.

## Other buses: J1939, NMEA 2000, RV-C, LIN

### Takeaway
J1939, NMEA 2000 and RV-C all ride on ordinary CAN, so any SocketCAN interface covers them electrically and Linux has an in-kernel J1939 stack. LIN is a different physical layer and needs its own transceiver.

### Cited Findings
- The Linux J1939 stack provides kernel-managed address claiming, NAME and source-address tracking, PGN filtering and transport-protocol sessions on the same CAN devices and socket family as SocketCAN; it names NMEA 2000, ISO 11783 (ISOBUS) and MilCAN as related protocols that differ at the application level, and leaves NMEA 2000 fast-packet transport to user space. RV-C is not mentioned. — [Linux kernel J1939 docs](https://docs.kernel.org/6.12/networking/j1939.html)
- The STN2120, the WiCAN Pro and the vLinker MC all list SAE J1939 among supported protocols. — [STN2120 datasheet](https://www.scantool.net/scantool/downloads/206/stn2120-ds.pdf); [MeatPi WiCAN Pro](https://www.meatpi.com/products/wican-pro); [Vgate vLinker MC WiFi](https://vgatemall.com/products-detail/i-37)

### Inferences
- The same CAN hardware covers these buses but the connectors differ: J1939 uses 9-pin Deutsch, NMEA 2000 uses DeviceNet Micro-C, RV-C uses its own drop connectors (prior knowledge). Budget for adapter cables, not a new interface.
- RV-C is a 250 kbit/s, 29-bit-identifier protocol structured like J1939 (prior knowledge), so a raw SocketCAN capture plus a user-space decoder is the expected approach.
- Listen-only mode should be the default on all of these; a diagnostic tool that claims an address on a tractor or boat network is an active participant.

### Gaps
- No source gathered on RV-C, NMEA 2000 decoders (for example canboat), or LIN hardware for Linux.

## Safety and legal: bus writes, Mode 04, gating

### Takeaway
Mode 04 is the main hazard in the standard service set: it wipes the evidence (codes, freeze frame, test results) and resets readiness monitors, which blocks an OBD emissions test until drive cycles complete. Arizona tests only in the Phoenix and Tucson areas, so the owner's exposure is low, but the tool is open source and other users will be in test areas.

### Cited Findings
- Service 04 clears confirmed and pending DTCs, freeze frame data, on-board monitoring test results (Service 06) including misfire counts, and resets I/M readiness bits, MIL status, and the distance/time/warm-up counters since clear; an ECU may refuse it with the engine running by returning 7F 04 22 (conditions not correct); clearing does not repair the fault. — [x-engineer: OBD Service 04](https://x-engineer.org/?p=5201)
- Arizona emissions testing applies to the Phoenix and Tucson control areas, including people who regularly commute into them; 1996+ gasoline vehicles under 8,500 lb GVWR get an OBD test; a manufacturer-defined drive cycle is needed to return monitors to ready. — [Arizona emissions FAQ (myazcar.com)](https://myazcar.com/faqs)
- The panda firmware model blocks transmission until a safety mode is explicitly selected. — [commaai/panda](https://github.com/commaai/panda)
- python-OBD exposes CLEAR_DTC as an ordinary command alongside read commands. — [python-OBD command tables](https://python-obd.readthedocs.io/en/latest/Command%20Tables/)

### Inferences
- Libraries treat clear as just another command, so gating has to be built into the MCP server. Follow the panda pattern: transmit capability is off until deliberately enabled, per session.
- Before any Mode 04, the server should automatically snapshot DTCs, freeze frame, readiness and Mode 06 (where available) to disk, show the human what will be lost, and warn about emissions readiness. Refuse if RPM is non-zero or speed is non-zero.
- Cochise County is outside the two test areas named by the state FAQ (inference from the area description; the county was not named on the page), so readiness resets are unlikely to affect the owner unless the truck commutes into Tucson.
- On the K-line Tacoma the blast radius of a bad request is small (one ECU, diagnostic line only). On CAN vehicles a malformed or high-rate frame can disturb every module on the bus, and manufacturer actuator tests can move hardware; that is where the strictest tier belongs.
- Physical safety matters as much as protocol safety: the pack hangs from the DLC near the driver's knees and pedals. A short extension cable with strain relief, and a "parked only" rule for anything beyond passive reads, are cheap mitigations.
- An agent polling continuously with the key on and engine off will flatten the battery; use the adapter's measured voltage as a watchdog and stop below a threshold.

### Gaps
- No primary legal source was gathered on emissions tampering rules or on liability for third-party tools writing to vehicle buses.
- The official ADEQ/ADOT emissions page failed to load; the area description comes from the testing contractor's FAQ.
- No documented incident data on OBD dongles causing faults while driving was gathered.

## Ideas we may not have thought of

### Takeaway
The most useful additions are ones that give the agent ground truth independent of the ECU and let it test its own tooling without the truck.

### Cited Findings
- A direct K-line implementation reached about 20 requests per second where a Bluetooth dongle managed 6. — [OBD9141](https://github.com/iwanders/OBD9141)
- The WiCAN Pro has an onboard IMU, SD-card logging and wake-on-motion. — [Crowd Supply WiCAN Pro](https://www.crowdsupply.com/meatpi-electronics/wican-pro)
- ELM327-emulator can run as a Bluetooth or TCP "adapter" and supports plugins for custom ECU behaviour. — [Ircama/ELM327-emulator](https://github.com/Ircama/ELM327-emulator)

### Inferences
- K-line protocol analyser mode: a second K-line transceiver wired receive-only lets the agent watch the adapter's own init sequence and byte timing, so it can tell "adapter is broken" from "truck isn't answering" from "wiring fault". This is the adapter-troubleshooting capability the brief asks for, and it costs one transceiver chip.
- DLC health check before any protocol attempt: measure pin 16 voltage, pin 4 and pin 5 resistance to battery negative, and pin 7 idle voltage (a healthy K-line idles near battery voltage). Both 1st-gen Tacoma no-communication threads found were wiring or ground faults, not adapter faults, so this check would have diagnosed them directly.
- Breakout harness: an OBD pass-through with every pin on a labelled header and switchable series resistors, so the scope and multimeter in the pack can attach to any DLC pin while the adapter stays connected.
- Fault-injection ECU: extend the simulator so the agent can practise against known faults (slow ECU, wrong checksum, dropped bytes, low supply voltage) and be scored, which doubles as a regression suite for the driver.
- Relative compression test with no extra sensors: disable fuel, crank, and sample battery voltage at a few kHz; uneven dips map to weak cylinders. Pair with an ignition pickup to label cylinder 1.
- "Record a drive" mode: log the small K-line PID set plus IMU, GPS, microphone and vacuum to SD, then let the agent analyse offline. This sidesteps the slow live data rate.
- Toyota blink-code reader: relay on TC plus a photodiode on the dash lamps to capture ABS/SRS codes, behind a confirmation gate.
- Dash and gauge camera: a cheap camera pointed at the cluster gives the agent MIL state, gauge sweep and warning lamps on vehicles where no bus exposes them.
- Smoke-free vacuum leak hunt: propane or brake-clean enrichment is risky, but the agent can watch short-term fuel trim and a MAP transducer while the human pinches hoses one at a time, and call out which one moved the numbers.
- Community PID packs: ship a small open data file format for enhanced PIDs and DBC references per vehicle, so other owners can contribute what their ECU answers; record the Mode 01 supported-PID bitmap of every vehicle the pack meets.

### Gaps
- None of these ideas was validated against a source or a prototype; costs are not estimated.
