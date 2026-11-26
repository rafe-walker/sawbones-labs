# Solar inverter / charge controller / BMS communications for a Raspberry-Pi-class portable diagnostic pack (state: November 2026)

Reading guide for the report writer

- "Cited Findings" = things I read on a fetched page this session. Each is tagged **[MANUAL]** (vendor manual / spec sheet / official doc), **[CODE]** (open-source code or its README), **[VENDOR-BLOG]**, or **[FORUM]** (user reports; lower confidence).
- "Inferences" = my reasoning from the cited findings, plus clearly labelled **[BACKGROUND, NOT VERIFIED THIS SESSION]** items recalled from prior knowledge. Background items must be re-checked before anyone wires or codes against them.
- Search tooling returned link lists without snippets, and DIY Solar Forum pages refused to load (client error) on every attempt, so forum coverage is thinner than requested. That is called out in Gaps.
- Biggest correction to the brief: **the EG4 3000EHV-48 does not appear to be a Voltronic/Axpert PI30 unit.** Evidence points to a SUMRY-family unit speaking Modbus RTU with the "SMG-II" register map. See Key Question 1.

---

## Key Question 1 — EG4 3000EHV-48: what is it, what does it speak, pinouts, parallel addressing, writable settings, OSS support, USB HID

### Takeaway
The 3000EHV-48 is handled by Solar Assistant as a "Sumry" inverter (not Voltronic), and the community register map for it (`eg4_3000ehv_v1`) is the same Modbus RTU layout used by ISolar/EASUN "SMG-II" inverters, so the right starting points are HotNoob/PythonProtocolGateway and syssi/esphome-smg-ii, not mpp-solar/PI30. Parallel units are not addressed QPGSn-style; Solar Assistant wants one cable per inverter, and the inverter exposes a settable Modbus ID (program 25).

### Cited Findings

Identity / protocol family
- **[MANUAL-level vendor doc: Solar Assistant]** Solar Assistant's device page for the 3000-EHV-48 says it "connects as" `sumry`, and lists two connection methods: RS232 cable and WiFi dongle (the dongle path is named `eybond-sumry`). — [Solar Assistant: EG4 3000EHV-48](https://solar-assistant.io/help/inverters/eg4/3000-EHV-48)
- **[MANUAL-level vendor doc: Solar Assistant]** For the RS232 route, Solar Assistant tells users to pick "Sumry" as the inverter model, plug into the port labelled RS232 or COM, and use either a "Voltronic USB RS232 cable" or the WiFi-dongle cable plus a USB serial cable; the example adapter shown is an FTDI RS232 USB device. — [Solar Assistant: 3000EHV-48 RS232](https://origin.solar-assistant.io/help/inverters/eg4/3000-EHV-48/rs232)
- **[CODE]** PythonProtocolGateway ships a protocol named `eg4_3000ehv_v1` (alongside `eg4_v58`, Growatt, Sigineer, SRNE `srne_v3.9`, and a 16-channel AC current module `hdhk_16ch_ac_module`) and talks Modbus RTU through the inverter USB port or a USB adapter on an RS485 port. — [HotNoob/PythonProtocolGateway](https://github.com/HotNoob/PythonProtocolGateway)
- **[CODE]** That project's EG4 device note says to reach the 3000EHV via an RS485/Modbus port using a USB-RS485 adapter (EG4-supplied or generic) and an RJ45 cable or three wires; baud for the 3000EHV is not given (19200 is listed only for the 6000XP and 18kPV); if a Pi CAN/RS485 HAT throws ModbusIOException, swap A/B. — [PythonProtocolGateway EG4 device doc](https://raw.githubusercontent.com/HotNoob/PythonProtocolGateway/main/documentation/devices/EG4.md)
- **[CODE]** The `eg4_3000ehv_v1` holding-register map: fault bitmask 100–101, warning bitmask 108–109, serial number ASCII 186–197, live values 201–234 (201 working mode, 202 mains V x0.1, 203 mains Hz x0.01, 208 inverter power W, 210/211 output V/A, 213 output active power, 215/216 battery V/A x0.1, 219 PV V, 223 PV W, 225 load %, 226 DCDC temp, 227 inverter temp, 229 battery %), settings 300–337, remote control 406–426, rated power 643. — [eg4_3000ehv_v1.holding_registry_map.csv](https://raw.githubusercontent.com/HotNoob/PythonProtocolGateway/main/protocols/eg4/eg4_3000ehv_v1.holding_registry_map.csv)
- **[CODE]** syssi/esphome-smg-ii documents the same numbering (100 fault code, 201 operation mode, 202 mains voltage x0.1 V, 229 battery SOC, 300 output mode R/W, 301 output priority R/W with 0=UTI 1=SOL 2=SBU 3=SUB, 643 rated power) as Modbus RTU over RS232 at 9600 baud, for ISolar/EASUN SMG II, SMG III 6.2 kW, PowMr POW-HVM5.5K-48V, Qoltec 53963 and Anenji units. EG4/Sumry are not named on that page. — [syssi/esphome-smg-ii](https://github.com/syssi/esphome-smg-ii)

Ports and pinouts
- **[MANUAL]** V2 manual: the RS-232 port is an RJ45 jack used for the WiFi dongle (a 5 ft RJ11-to-DB9 RS232 cable is in the box); a shared "USB / RS-485" port is described as a PC-application port or RS485 BMS port and a 3 ft RS485-to-USB cable is included; the parallel port is RJ45 and runs CAN; one 3 A / 250 VAC dry contact signals battery-voltage warning. The spec table contradicts itself ("No USB Port" next to "RS-485 Yes | USB Type A"). — [EG4 3000EHV-48 V2 manual](https://eg4electronics.com/wp-content/uploads/2024/04/EG4-3000EHV-48-V2-Manual.pdf)
- **[MANUAL]** The only pinout printed in either manual is the battery-side RS485 lead: pin 1 RS485-B, pin 2 RS485-A, pins 3 and 4 GND, pin 5 NC. RS232 and parallel-port pinouts and baud rates are not printed. — [EG4 3000EHV-48 V2 manual](https://eg4electronics.com/wp-content/uploads/2024/04/EG4-3000EHV-48-V2-Manual.pdf); [V1 manual RevA](https://signaturesolar.com/content/documents/EG4/EG4%203000EHV-48%20Inverter%20Users%20Manual%20-%20Final%20RevA%20with%20WiFi%20Startup.pdf)
- **[MANUAL]** V1 (older) manual: RS232 RJ45 jack for WiFi, RS485 BMS port, parallel RJ45 (CAN, built-in kit), USB listed as PC port but spec table again says no USB port; app is called "SmartClient" with a "Wi-Fi Plug Pro" datalogger; screenshots show "Eybond" WiFi SSIDs; program 25 is "Modbus ID". V2 names the app SmartESS. — [V1 manual RevA](https://signaturesolar.com/content/documents/EG4/EG4%203000EHV-48%20Inverter%20Users%20Manual%20-%20Final%20RevA%20with%20WiFi%20Startup.pdf); [V2 manual](https://eg4electronics.com/wp-content/uploads/2024/04/EG4-3000EHV-48-V2-Manual.pdf)
- **[CODE, for the sibling SMG-II hardware, not EG4-verified]** SMG-II RJ45 RS232 pinout: pin 1 TX, pin 2 RX, pin 4 = +12 V supply, pin 8 GND, others unused. Some 6 kW firmware variants instead use a USB-A-shaped COM port (D+/D- carry TX/RX), need a wake-up frame `01 AA 06 DE A2`, and use different register addresses. — [syssi/esphome-smg-ii](https://github.com/syssi/esphome-smg-ii)

Parallel units
- **[MANUAL-level vendor doc]** In parallel installs Solar Assistant requires a separate cable to each inverter. — [Solar Assistant: 3000EHV-48 RS232](https://origin.solar-assistant.io/help/inverters/eg4/3000-EHV-48/rs232)
- **[MANUAL]** Up to 12 units in parallel (36 kW single phase); parallel link is CAN on RJ45; all units share one battery bank; only one unit should be wired to the BMS and set to the lithium protocol, the rest stay on AGM. Modbus ID is settable 001–247 via program 25. — [V2 manual](https://eg4electronics.com/wp-content/uploads/2024/04/EG4-3000EHV-48-V2-Manual.pdf)

Settings (front-panel programs) and writable registers
- **[MANUAL]** Programs: 01 output source priority (Utility/Solar/SBU/SUB), 02 max total charge current 10–80 A (default 60), 05 battery type (AGM, Flooded, User, several "customized protocol" entries, EG4 protocol shown as "Li4"/"LI X" in V2 and "Li2"/"LI1" inconsistently in V1), 11 max utility charge current 2–80 A (default 30), 12 back-to-utility voltage (44.0–57.2 V, default 46), 13 back-to-battery voltage, 16 charger source priority, 25 Modbus ID, 26 bulk voltage (48.0–62.0 V, default 56.4), 27 float (default 54.0), 28 AC output mode (single/parallel/phase), 29 low DC cut-off (40.0–54.0 V, default 42.0), 30 PV judge condition, 32 bulk time, 33–39 equalization, 43–45 SOC set points (lithium with comms). — [V2 manual](https://eg4electronics.com/wp-content/uploads/2024/04/EG4-3000EHV-48-V2-Manual.pdf); [V1 manual](https://signaturesolar.com/content/documents/EG4/EG4%203000EHV-48%20Inverter%20Users%20Manual%20-%20Final%20RevA%20with%20WiFi%20Startup.pdf)
- **[CODE]** Registers flagged writable in the community map: 108–109, 300–303 (output mode, output priority, input voltage range, buzzer), 305–310, 313, 320–321 (output voltage, output frequency), 323–327 (323 = battery over-voltage protection point), 329, 331–337 (331 charging priority, 332 max charge current, 335 equalization time), 406 (turn-on mode), 420 (remote switch); 426 "exit fault mode" is write-only. — [eg4_3000ehv_v1 register map](https://raw.githubusercontent.com/HotNoob/PythonProtocolGateway/main/protocols/eg4/eg4_3000ehv_v1.holding_registry_map.csv)
- **[CODE]** esphome-smg-ii exposes the same R/W set as controls (300, 301, 302, 303, 320, charging voltages/currents 323–334, 406, 420; 426 write-only). — [syssi/esphome-smg-ii](https://github.com/syssi/esphome-smg-ii)

Solar Assistant coexistence
- **[MANUAL-level vendor doc]** Solar Assistant runs an MQTT broker (examples use port 1883); readings appear as `solar_assistant/inverter_1/<name>/state`; Home Assistant discovery is published under `homeassistant/#`; writes go to `.../set` topics but are disabled until "Allow setting changes" is enabled in the MQTT Broker config; `.../set_verified` writes, reads back and retries up to three times; a batch of 14 serial settings takes about 20–30 s. — [Solar Assistant MQTT](https://solar-assistant.io/help/integration/mqtt)

Voltronic PI30 (for contrast; applies to EG4 6500EX and Axpert clones, not shown to apply to the 3000EHV)
- **[CODE/doc]** PI30/PI30MAX is ASCII over RS232 at 2400 8N1, CR-terminated, CRC before the CR, replies start with `(`, setters answer ACK/NAK. Queries include QPI, QID, QVFW, QPIRI, QPIGS, QPIGS2, QPGSn (parallel unit status), QMOD, QPIWS, QFLAG, QDI, QET/QEY/QEM/QED, QBMS. Setters include POP, PCP/PPCP, PBT, POPM, MNCHGC/MUCHGC, PBCV/PBDV/PSDV/PCVV/PBFT, PBEQ*, PF (factory defaults), PBATCD, PBATMAXDISC, DAT. — [powermon PI30MAX protocol doc](https://jblance.github.io/powermon/protocoldocs.inverter.pi30max.html)
- **[CODE]** jblance/mpp-solar (Python, MIT, 0.16+ needs Python 3.10+) lists PIP-4048MS, IPS-4000WM, Axpert MKS 5KVA, LV5048, JK BMS (BLE extra), Victron VE.Direct (SmartShunt tested) and Daly BMS. — [jblance/mpp-solar](https://github.com/jblance/mpp-solar)
- Solar Assistant has separate RS232 and micro-USB help pages for the EG4 6500EX-48, i.e. that model is treated differently from the 3000EHV. — [Solar Assistant: 6500EX-48 RS232](https://solar-assistant.io/help/inverters/eg4/56/rs232)

### Inferences
- Three independent signals agree that this is the SMG-II/Sumry Modbus family and not PI30: Solar Assistant's "Sumry" selection, the Eybond/SmartESS dongle, and a register map identical in numbering to esphome-smg-ii. I rate this high-confidence but not vendor-confirmed; neither EG4 manual names the OEM.
- Likely serial settings for the RS232 RJ45: 9600 8N1 Modbus RTU, slave ID = program 25 value (factory value probably 1). Derived from the SMG-II sibling, not from an EG4 document. First bench step: read holding register 201 and 643 at 9600 baud with IDs 1..3.
- Solar Assistant's "Voltronic USB RS232 cable" works here because that cable's RJ45 pinout (TX on 1, RX on 2, GND on 8) matches the SMG-II pinout above. That is why the port looks Voltronic even though the protocol is not.
- **Pin 4 of the RS232 RJ45 probably carries +12 V for the WiFi dongle.** A breakout or home-made cable must leave pin 4 unconnected (or use it deliberately as a fused dongle-power source). Never plug a straight Ethernet patch lead from this jack into a Pi HAT RJ45, a CAN port, or a PoE/Ethernet device.
- The owner's two paralleled inverters each need their own RS232 link (two adapters) if polled directly. Whether both can instead be multi-dropped on the RS485 port with different Modbus IDs is plausible (program 25 exists) but unverified; the RS485 port is also the BMS port, so it may be occupied.
- The PythonProtocolGateway note (RS485 port) and Solar Assistant (RS232 port) disagree on which port to use. Both may work; treat RS232 as the documented-by-SA path.
- Because Solar Assistant already holds both RS232 ports at the owner's site, the pack's lowest-risk read path at home is subscribing to Solar Assistant's MQTT broker; direct RS232 is for field use on other people's systems or when SA is down.
- mpp-solar, inverter-tools, ha-voltronic and ESPHome `pipsolar` are PI30-family tools and should be scoped to "popular gear" (Axpert/MPP Solar/EG4 6500EX), not to the owner's 3000EHV.

### Gaps
- No vendor protocol PDF for the 3000EHV-48 was found; baud rate, default slave ID and RS232 RJ45 pinout are inferred from SMG-II sibling hardware.
- OEM identity (Sumry) rests on Solar Assistant's naming; no EG4 statement found.
- Whether the owner has V1 or V2 hardware, and whether the V1 has a different register map, is unknown. The brief's "EX3000" naming does not match either manual; confirm the label on the unit.
- USB HID: no evidence of a Voltronic-style HID interface; the manuals are self-contradictory about USB. Unresolved.
- Firmware quirks: none found from a source I could load. DIY Solar Forum threads on this model (for example "Monitoring for new EG4 3kW inverters - solar assistant compatible?") returned client errors.
- inverter-tools, ha-voltronic, ESPHome pipsolar: not fetched; no claim made about them beyond being PI30 tools (background).
- Which lithium protocol number (Li2 / Li4 / "LI X") matches the Eco-Worthy/JBD packs is not established.

---

## Key Question 2 — Eco-Worthy 48 V 100 Ah rack batteries and the JBD UP16S BMS: ports, pinouts, protocols, addressing, software, diagnostics

### Takeaway
The UP16S has an isolated RS485 pair (9600 baud), a CAN port (500 kbit/s) and a non-isolated RS232 UART on RJ11; the only open implementation of its native protocol I found is the `lltjbd_up16s.py` driver in mr-manuel's dbus-serialbattery, which documents a Modbus-like custom framing (functions 0x78 read / 0x79 write / 0x45) with full per-cell data. That driver is the reference to port for "jbd-up"; no Rust or standalone library surfaced.

### Cited Findings

Hardware (JBD spec sheet for UP16S010)
- **[MANUAL]** RS485: isolated, RJ45 8P8C, pins 1 and 8 = B, 2 and 7 = A, 3 and 6 = GND, 4 and 5 NC, default 9600 bps. A second (parallel) RS485 jack repeats the same pattern on pins 9–16. — [JBD UP16S010 spec sheet](https://cdn.shopify.com/s/files/1/0606/5199/5298/files/JBD-UP16S010-L16S-200A-200A-B-U-R-C-A03.pdf)
- **[MANUAL]** CAN: RJ45, numbered on the stacked 16-pin scheme as pin 12 CANL, pin 13 CANH, pin 15 GND; default 500 kbps. — [JBD UP16S010 spec sheet](https://cdn.shopify.com/s/files/1/0606/5199/5298/files/JBD-UP16S010-L16S-200A-200A-B-U-R-C-A03.pdf)
- **[MANUAL]** RS232: plain UART, **not isolated**, RJ11 6P6C, pin 3 TX, pin 4 RX, pin 5 GND. The "isolated UART" option is listed as none on this variant. — [JBD UP16S010 spec sheet](https://cdn.shopify.com/s/files/1/0606/5199/5298/files/JBD-UP16S010-L16S-200A-200A-B-U-R-C-A03.pdf)
- **[MANUAL]** DIP: 4-bit binary address 0–15, each pack unique; 0000 is labelled MASTER (also the setting for a lone pack on RS485). Bluetooth: not supported on this variant. LCD standard. History storage: standard, capacity unstated. — [JBD UP16S010 spec sheet](https://cdn.shopify.com/s/files/1/0606/5199/5298/files/JBD-UP16S010-L16S-200A-200A-B-U-R-C-A03.pdf)
- **[MANUAL]** Passive balancing, 20–60 mA, starts around 3.50 V with 30 mV spread. 20 A current-limit module engages after charge over-current. No heating-film support. Sleep after 5 min idle; comms or current wakes it; button 3–6 s wake/sleep, 6–10 s reset. Typical protections: cell OV 3.65 V (release 3.38), cell UV 2.70 V (release 2.95), charge OC 230 A, discharge OC 230 A / 650 A, short circuit 1800 A, FET over-temp 115 C, 200 A continuous. — [JBD UP16S010 spec sheet](https://cdn.shopify.com/s/files/1/0606/5199/5298/files/JBD-UP16S010-L16S-200A-200A-B-U-R-C-A03.pdf)

Native protocol (open-source driver)
- **[CODE]** `lltjbd_up16s.py`: 9600 8N1; request and response share an 8-byte header (address, function, start address, end address, payload length) followed by payload and a CRC-16 (init 0xFFFF, poly 0xA001, low byte first). Not standard Modbus despite the naming. — [dbus-serialbattery lltjbd_up16s.py](https://github.com/mr-manuel/venus-os_dbus-serialbattery/blob/master/dbus-serialbattery/bms/lltjbd_up16s.py)
- **[CODE]** Commands: PackStatus fn 0x78 start 0x1000 len 0xA0 (main telemetry); PackParams1 fn 0x78 start 0x1C00 (model, serial, dates); PackParams2 fn 0x78 start 0x2006 (high-resolution SOC, lifetime charge/discharge Ah); ProductInformation fn 0x78 start 0x2810 (model, project code, firmware); IndividualPackStatus fn 0x45 (per-pack charge/discharge current limits, direct connection only); SetSoc fn 0x79 at 0x2006 with payload prefix `11 4A 42 44` then SOC x100. — [dbus-serialbattery lltjbd_up16s.py](https://github.com/mr-manuel/venus-os_dbus-serialbattery/blob/master/dbus-serialbattery/bms/lltjbd_up16s.py)
- **[CODE]** Data available: pack voltage (raw/100), current ((raw-300000)/100 A), SOC, remaining/full/rated capacity, SOH, MOSFET and ambient temperature ((raw-500)/10 C), up to 16 cell voltages (mV), up to 4 temperature probes, min/max/avg cell, charge and discharge current limits, cycle count, fault bits 0–27, alarm bits 0–18, MOSFET charge/discharge flags, per-cell balance bitmask, firmware version, serial number. — [dbus-serialbattery lltjbd_up16s.py](https://github.com/mr-manuel/venus-os_dbus-serialbattery/blob/master/dbus-serialbattery/bms/lltjbd_up16s.py)
- **[CODE]** Addressing in the driver: one-byte address, address 1 treated as master; master returns stack-aggregated limits. Tested on UP16S015 firmware 10.2.1 and 12.1.7. Only write implemented is SOC. Bluetooth is not implemented in this driver. — [dbus-serialbattery lltjbd_up16s.py](https://github.com/mr-manuel/venus-os_dbus-serialbattery/blob/master/dbus-serialbattery/bms/lltjbd_up16s.py)
- **[CODE docs]** dbus-serialbattery connection notes: for UP16S set `BMS_TYPE = LltJbd_Up16s`; addresses start at 1 with 1 as master and every battery must be uniquely identifiable; refresh is about 1 s for a directly connected pack versus about 50 s when data is relayed through the daisy chain on RS485-1 or RS232; RS232 is listed as untested; the UP16S "UART communication module" (Bluetooth) is a supported route; JBD CAN works only in the BMS's "Victron" CAN dialect. — [dbus-serialbattery: connect and prepare](https://mr-manuel.github.io/venus-os_dbus-serialbattery_docs/next/general/connect)
- **[CODE docs]** The driver notes that on ordinary ports per-pack detail is only readable from the pack physically plugged into that port, while the Bluetooth/WiFi UART port behaves differently. — [dbus-serialbattery lltjbd_up16s.py](https://github.com/mr-manuel/venus-os_dbus-serialbattery/blob/master/dbus-serialbattery/bms/lltjbd_up16s.py)

Eco-Worthy specifics
- **[VENDOR-BLOG]** Eco-Worthy says packs link to each other over RS485 with one master; inverter link is RS485 or CAN; V2.0 and V3.0 add Bluetooth or WiFi; compatibility is claimed with Growatt, Deye, Victron, Voltronic, SRNE and Pylontech protocol; protocol choice is by DIP or "upper computer" software; max parallel 15 (V2.0) or 32 (V3.0); PC tool is the "JBD host computer" over USB serial, with firmware upgrade; Solar Assistant and Home Assistant via BMS_BLE-HA are named as third-party options. — [Eco-Worthy blog: communication capabilities](https://ca.eco-worthy.com/blogs/blog/article-page-may-27-09-33-17)
- **[VENDOR-BLOG]** Master pack address is 1 and slaves run 2..32; packs daisy-chain on the RS485-2 ports; the inverter protocol is selected in the mobile app, host PC software, or the ESM-100 external display. — [Eco-Worthy: master-slave communication](https://au.eco-worthy.com/blogs/lithium-battery/how-to-configure-master-slave-communication-between-batteries)
- **[VENDOR-BLOG]** Host PC software (JBD tools) shows pack V/I/SOC, per-cell voltage, temperatures, balance state, protection states, a history/protection log exportable to CSV, password-gated parameter editing (voltage/current thresholds, temperature limits, chemistry, capacity) and, in some versions, firmware upgrade; a dedicated RS232/RS485-to-USB cable is recommended. — [Eco-Worthy: host PC software](https://au.eco-worthy.com/blogs/lithium-battery/how-to-use-the-host-pc-software); guide PDF: [APP/BMS Tool Guide](https://cdn.shopify.com/s/files/1/0593/1096/4906/files/APP_BMS_Tool_Guide-20250909.pdf?v=1757404968)
- **[CODE]** BMS_BLE-HA (Apache-2.0, built on the `aiobmsble` library) lists ECO-WORTHY, JBD and its rebrands, JK, Daly, Seplos v2/v3, EG4 LL, Renogy, Redodo, Pylontech and others over BLE. It is monitoring-only by design; the author declines control functions because a BMS is safety-critical and undocumented. It exposes cell voltages, temperatures, cycles, balancer state with cell bitmask, and a problem code. — [patman15/BMS_BLE-HA](https://github.com/patman15/BMS_BLE-HA)
- **[CODE]** syssi/esphome-jbd-bms targets classic JBD BMSs over UART-TTL or BLE (not the UP series specifically). — [syssi/esphome-jbd-bms](https://www.github.com/syssi/esphome-jbd-bms)

### Inferences
- A six-pack stack has two ways to get full per-cell data from every pack: (a) plug the pack's adapter into each battery's spare RS485 jack in turn (about 1 s refresh, a natural job for a portable pack), or (b) sit on the master and accept roughly 50 s per-pack relay latency, or use the Bluetooth/WiFi UART port if the owner's units have it.
- DIP/address conventions conflict between sources: the JBD sheet calls 0000 the master, while Eco-Worthy and the driver both say master = address 1. Likely firmware-generation dependent. "jbd-up" should probe addresses 0..16 rather than assume.
- In 8-pin terms the CAN signals on the JBD sheet (stacked pins 12, 13, 15) probably land on pins 4 (CANL), 5 (CANH) and 7 (GND) of the upper jack. That is the reverse of the Pylontech convention cited in Key Question 4 (CAN-H on 4, CAN-L on 5), so verify with a meter or scope before trusting either.
- The non-isolated RS232 UART is the port most likely to hurt an un-isolated Pi. Use an isolated adapter there even for "just a quick look".
- "jbd-up" opportunity: I found no Rust crate and no standalone library for the UP16S native protocol; the dbus-serialbattery driver is embedded in a Venus OS service. A clean Rust crate plus a simulator (see Key Question 7) would be new work rather than duplication. Event-log and parameter read/write commands are not in the open driver and would need capture of the JBD PC tool traffic.
- The one battery of another brand probably speaks a different native protocol (PACE, Seplos, etc.); on a shared stack it is normally left out of the JBD master/slave chain. Needs identification on site.

### Gaps
- Could not load the Eco-Worthy manual on device.report or the DIY Solar Forum resource "JBD-ES-UP software" (client errors), so Eco-Worthy's own RJ45 pin table and DIP diagrams are missing; the pinouts above are from the JBD board spec sheet.
- Which inverter-facing protocols the UP16S emulates on RS485 and CAN, and their menu names, are only described at brand level by Eco-Worthy. No protocol PDF found.
- A PDF titled "Communication Protocol RS485" surfaced for a "48V SLPO series 15S/16S" battery (Modbus RTU 9600, registers 0–36 live data, 1000–1500 warning records, 5000–5019 device info). Its vendor is not stated and I could not tie it to Eco-Worthy or JBD; do not assume it applies. — [Communication_Protocol_RS485.pdf](https://cdn.shopify.com/s/files/1/0788/7644/2962/files/Communication_Protocol_RS485.pdf)
- No source found for the UP16S protection-event-log command set, parameter write commands, or firmware update protocol.
- Solar Assistant's own notes on Eco-Worthy/JBD UP16S were not located.
- UP16S015 vs UP16S010 differences (Bluetooth module, port count) not documented in what I could read.
- jbd-tool and batmon-ha: not checked this session.

---

## Key Question 3 — EG4 hybrid solar mini split and EcoFlow Delta Pro: local/open interfaces

### Takeaway
Both have workable local paths: the mini split is reported to be a Tuya-based (Deye-built) unit readable and controllable over local Tuya protocol 3.4, with an existing Rust-to-MQTT bridge; the original Delta Pro is supported by the local-BLE Home Assistant integration ha-ef-ble.

### Cited Findings
- **[FORUM]** Users report the EG4 hybrid mini split is a rebranded Deye unit on Tuya architecture; EG4's "Solar Aircon" app and the Tuya / Smart Life apps both drive it. — [HA community: EG4 Hybrid Mini Split AC Integration](https://community.home-assistant.io/t/eg4-hybrid-mini-split-ac-integration/589783)
- **[FORUM]** LocalTuya / Xtend Tuya give local climate control once the device local key is obtained through a Tuya developer account. The 12K hybrid uses Tuya protocol 3.4 (3.3 fails with stream-closed errors); the 9K and 12K share product ID `gpjca2vt` and a 21-datapoint map. — [same thread](https://community.home-assistant.io/t/eg4-hybrid-mini-split-ac-integration/589783)
- **[FORUM/CODE]** A Rust bridge, tfoote000/eg4heatpumptuyatomqtt, connects locally and publishes every datapoint, including hidden solar and energy ones, to MQTT; forks add HA discovery and per-device protocol settings. — [same thread](https://community.home-assistant.io/t/eg4-hybrid-mini-split-ac-integration/589783); [tfoote000/eg4heatpumptuyatomqtt](https://github.com/tfoote000/eg4heatpumptuyatomqtt)
- **[FORUM]** Known problems: the WiFi module stops answering LAN connections after repeated probing until the breaker is cycled; running the phone app alongside a local client causes drops; an RS485 header was seen inside the head unit but nobody has a working Modbus path; the unit overshoots setpoints by 7–8 F. — [same thread](https://community.home-assistant.io/t/eg4-hybrid-mini-split-ac-integration/589783)
- **[CODE]** rabits/ha-ef-ble (Apache-2.0) lists the original "Delta Pro" plus Delta Pro 3, Delta Pro Ultra, Delta 2/3 families, River 2/3, Smart Home Panel 2/3, PowerStream and more; it works locally over Bluetooth without internet; setup needs the device bound to an EcoFlow account and the account's User ID; it offers sensors plus switches/sliders/selects for outputs and charge settings; EcoFlow devices accept only one BLE connection, so the phone app and the integration cannot be connected together; reverse-engineering notes live in rabits/ef-ble-reverse. — [rabits/ha-ef-ble](https://github.com/rabits/ha-ef-ble)

### Inferences
- The mini split fits the pack as a network device, not a wired one: LAN + local key, polled gently (the module locks up when probed hard).
- For the Delta Pro, BLE from the Pi's radio is the no-extra-hardware path; the pack must release the connection when the owner wants the phone app.
- **[BACKGROUND, NOT VERIFIED THIS SESSION]** EcoFlow also has a cloud MQTT/HTTP developer API (tolwi/hassio-ecoflow-cloud) and older Delta Pro firmware exposed a local TCP port used by vwt12eh8/hassio-ecoflow, which later firmware removed (a discussion in that repo is titled "Do not perform a firmware update"). The extra battery is reported through the main unit.

### Gaps
- Whether the User ID step in ha-ef-ble needs internet once, and the BLE authentication/encryption details for the original Delta Pro, were not on the page read.
- OEM = Deye for the mini split is a forum claim only.
- Per-datapoint list for the mini split (PV watts, grid watts, compressor state) was not extracted.

---

## Key Question 4 — The "popular profiles" map: physical layer, connector, baud, read/write, best OSS

### Takeaway
Three physical layers cover nearly everything: RS485 (mostly Modbus RTU at 9600 or 19200, plus ASCII Pylontech/PACE-style framing), CAN at 500 kbit/s with the Pylontech/SMA 0x35x frame set, and BLE; RS232/TTL UART fills in older inverters and VE.Direct. Only a minority of this map was verified against a page this session, so the table below separates verified rows from recalled ones.

### Cited Findings (verified this session)
- **Pylontech CAN** — Pylontech battery RJ45: CAN-H on pin 4, CAN-L on pin 5, ground on pin 6 (not required); Linux bring-up in the emulator example uses 500000 bit/s; one CAN port per battery so a splitter may be needed; use twisted pair; exactly two 120 ohm terminators at the bus ends; some SN65HVD230 boards need their on-board terminator removed. A receive-only transceiver is enough for monitoring. — [OpenDTU-OnBattery wiki: Pylontech battery](https://github.com/helgeerbe/OpenDTU-OnBattery/wiki/Pylontech-battery)
- **Pylontech CAN frame IDs seen on a live battery-inverter link**: 0x351, 0x355, 0x356, 0x359, 0x35C, 0x35E from the battery and 0x305 as the inverter's heartbeat reply; 0x355 holds SOC and SOH as two little-endian 16-bit values; BMS frames repeat about once per second. — [HA community: reading Pylontech-Solis CAN](https://community.home-assistant.io/t/reading-can-bus-communication-between-pylontech-battery-and-solis-inverter/420788)
- **Victron VE.Direct** — four-pin port (power, TX, RX, GND); logic level is 3.3 V on some products (BMV-700, MPPT RS, Multi RS, VE.Direct inverters) and 5 V on others (all regular MPPTs); the power pin supplies only about 10 mA average; TEXT frames are periodic name/value records with a modulo-256 checksum; HEX messages are colon-prefixed request/response and pause TEXT output for a few seconds; Victron recommends its own isolated VE.Direct-USB or VE.Direct-RS232 interfaces because the products have very little protection. — [Victron VE.Direct protocol FAQ](https://www.victronenergy.com/live/vedirect_protocol:faq); whitepaper: [Data communication with Victron Energy products](https://www.victronenergy.com/upload/documents/Whitepaper-Data-communication-with-Victron-Energy-products_EN.pdf); [Victron open source index](https://www.victronenergy.com/live/open_source:start)
- **Victron BLE Instant Readout** — the Rust crate `victron_ble` 1.2.0 (MIT) decodes the encrypted advertisements for Solar Charger, Battery Monitor, Inverter, VE.Bus and AC Charger device types; it needs the per-device encryption key from VictronConnect (Settings > Product Info) and "Instant Readout" enabled. — [docs.rs victron_ble](https://docs.rs/victron_ble)
- **Voltronic/Axpert/MPP Solar PI30** — RS232 2400 8N1 ASCII, see Key Question 1. — [powermon PI30MAX](https://jblance.github.io/powermon/protocoldocs.inverter.pi30max.html); [jblance/mpp-solar](https://github.com/jblance/mpp-solar)
- **SMG-II / Sumry / PowMr HVM / Anenji / EASUN SMG** — Modbus RTU over RS232 at 9600 on RJ45 (1 TX, 2 RX, 4 +12 V, 8 GND), read/write. — [syssi/esphome-smg-ii](https://github.com/syssi/esphome-smg-ii)
- **EG4 6000XP / 18kPV (LuxPower family)** — Modbus RTU over RS485 at 19200 in PythonProtocolGateway (`eg4_v58`). — [PythonProtocolGateway EG4 doc](https://raw.githubusercontent.com/HotNoob/PythonProtocolGateway/main/documentation/devices/EG4.md)
- **Growatt, Sigineer, SRNE** — Modbus RTU maps shipped in PythonProtocolGateway (Growatt 2020+ v0.14, `sigineer_v0.11`, `srne_v3.9`); Victron GX and Sol-Ark maps exist but are marked untested. — [HotNoob/PythonProtocolGateway](https://github.com/HotNoob/PythonProtocolGateway)
- **BMS over RS485 / CAN / UART (Venus OS driver)** — dbus-serialbattery supports JKBMS and JKBMS PB (inverter BMS), JBD (also LLT Power, Overkill Solar, Xiaoxiang-app BMSs), Daly (CAN recommended, RS485 slower), Daren (DR-1363 protocol over RS485), EG4 LiFePOWER (also Revov / Tian Power), EG4 LL, Felicity, KS48100, LiTime/Redodo, Heltec/YYBMS Modbus, PACE, Renogy, Seplos and Seplos v3 (v3 is a different, Modbus protocol), generic MQTT. — [dbus-serialbattery supported BMS](https://mr-manuel.github.io/venus-os_dbus-serialbattery_docs/next/general/supported-bms)
- **Addressing details from the same project** — Daly RS485 board numbers 1–16 map to Modbus addresses 0x40–0x4F (CAN 0x01–0x10) and must be set with the Windows BmsMonitor tool; Renogy battery addresses run 0x30–0x3F; EG4 LiFePower and JK PB use DIP addresses (all-off on JK PB means master mode); JK PB daisy-chains on its RS485-2 ports and the JK RS485 port silk-screen is wrong (docs carry a corrected pinout). — [dbus-serialbattery: connect](https://mr-manuel.github.io/venus-os_dbus-serialbattery_docs/next/general/connect)
- **BMS over BLE** — aiobmsble / BMS_BLE-HA covers JBD, JK (hardware v6+), Daly, Seplos v2/v3, Eco-Worthy, EG4 LL, Renogy, Redodo, LiTime, Pylontech, ANT, Daren, Felicity, Topband, Vatrer, SOK and more, read-only; JK firmware 19.30+ has BlueZ compatibility trouble. — [patman15/BMS_BLE-HA](https://github.com/patman15/BMS_BLE-HA)
- **ESPHome components worth porting** — syssi publishes esphome-jbd-bms (UART/BLE), esphome-seplos-bms (UART/RS485), esphome-pace-bms (RS485 Modbus), esphome-jk-bms (RS485/BLE), esphome-smg-ii. — [esphome-jbd-bms](https://www.github.com/syssi/esphome-jbd-bms); [esphome-seplos-bms](https://www.github.com/syssi/esphome-seplos-bms); [esphome-pace-bms](https://github.com/syssi/esphome-pace-bms); [esphome-smg-ii](https://github.com/syssi/esphome-smg-ii)

### Inferences — recalled profile table **[BACKGROUND, NOT VERIFIED THIS SESSION; verify each row before wiring or coding]**

| System | Physical / connector | Typical serial settings | R/W | OSS to look at |
|---|---|---|---|---|
| Pylontech CAN (de facto battery-to-inverter) | CAN on RJ45 | 500 kbit/s, 11-bit IDs 0x351 (charge V, charge/discharge A limits), 0x355 (SOC/SOH), 0x356 (V/I/T), 0x359 (alarms), 0x35C (charge/discharge enable flags), 0x35E (name) | Broadcast; inverter only acks | OpenDTU-OnBattery, Venus OS, esphome-jk-bms-can |
| Pylontech RS485 | RS485 on RJ45 | ASCII-hex frames starting `~`, 115200 (some packs 9600) | Read; few commands | pylontech python libs, Venus OS |
| PACE / Seplos v2 / many rack packs | RS485 on RJ45 | Same `~` ASCII framing (YD/T 1363 lineage), 9600 or 19200 | Read + parameter writes with PC tool | esphome-pace-bms, esphome-seplos-bms, dbus-serialbattery |
| Seplos v3, JK PB | RS485 | Modbus RTU, 19200 (Seplos v3), 115200 (JK PB) | Read + writes | dbus-serialbattery, esphome-jk-bms |
| JBD classic (non-UP) | TTL UART / BLE | 9600, 0xDD-framed, cmds 0x03/0x04/0x05 | Read + MOSFET control + EEPROM | esphome-jbd-bms, aiobmsble |
| Daly | UART/RS485/CAN/BLE | 9600, 0xA5-framed 13-byte | Read + some writes | dbus-serialbattery, python-daly-bms |
| EG4 LL (v1/v2) | RS485 RJ45 | Modbus RTU 9600, DIP = ID | Read | dbus-serialbattery, Solar Assistant |
| EG4 LifePower4 | RS485 RJ45 | Tian Power ASCII protocol, 9600 | Read | dbus-serialbattery |
| Victron VE.Direct | TTL UART, JST-PH 4-pin | 19200 8N1 | TEXT read; HEX get/set | vedirect crates, Venus OS vedirect-interface |
| Victron VE.Can | CAN on RJ45 | 250 kbit/s, NMEA 2000 / J1939 based | Read/write | Venus OS, canboat |
| Victron VE.Bus | RS485-like proprietary on RJ45; needs MK3-USB | via MK3 | Read/write (power-affecting) | Venus OS mk2-dbus; treat as human-only |
| SunSpec Modbus | RS485 or TCP 502 | "SunS" marker at 40000, model chain | Read; some control models | pysunspec2 |
| SunSpec Rapid Shutdown | Power-line carrier on the PV DC conductors | keep-alive tone pair near 131 and 144 kHz | Not a data bus; detect presence only | none known |
| Growatt SPF / Sigineer | RS485 RJ45 or USB-B | Modbus RTU 9600, ID 1 | Read/write | PythonProtocolGateway, grott (cloud dongle) |
| Sol-Ark / Deye / Sunsynk | RS485 on RJ45 (and RS232 dongle port) | Modbus RTU 9600, ID 1 | Read/write | kellerza/sunsynk |
| EG4 18kPV / 6000XP / 12000XP (LuxPower) | RS485 RJ45; WiFi dongle TCP | Modbus RTU 19200; dongle protocol on TCP 8000 | Read/write | lxp-bridge (Rust), PythonProtocolGateway |
| Schneider XW / XW Pro | Xanbus (CAN, RJ45) and Modbus TCP via gateway | 250 kbit/s Xanbus | Read/write through gateway | limited |
| SMA | Speedwire / Modbus TCP (SunSpec); Sunny Island CAN to BMS uses the 0x35x set | - | Read; grid-guard-gated writes | pysma, SBFspot |
| Renogy Rover / Wanderer | RS232 on RJ12 (BT-1) or RS485 on RJ45 (BT-2) | Modbus RTU 9600, ID 1 (255 broadcast) | Read; some writes | cyrils/renogy-bt, renogy-modbus libs |
| Epever Tracer | RS485 on RJ45 (pins 3-4 B, 5-6 A, 7-8 GND, 1-2 supply) | Modbus RTU 115200, ID 1 | Read/write | epever-tracer libs |
| Outback (Mate3s / AXS) | Ethernet | SunSpec Modbus TCP | Read/write | pymate, outback_mate3 |
| Midnite Classic | Ethernet (and RS232 on RJ11) | Modbus TCP 502 | Read/write | classic monitor tools |
| Enphase IQ Gateway | LAN HTTPS | token-authenticated JSON API | Read; limited control | pyenphase |
| Tigo | RS485 between CCA and TAP | proprietary | Passive read | taptap (Rust) |
| APsystems | EZ1 local HTTP API; ECU local TCP | - | Read; EZ1 power limit | apsystems-ez1 libs |

Other inferences
- Pylontech and SMA-style CAN pin assignments differ between vendors (Pylontech: H on 4, L on 5; many inverters use 4/5 the same way, some BMSs flip them, Victron BMS-Can uses 7/8). A breakout with screw terminals beats any fixed "universal" cable.
- The Rust crates that already exist cluster on Victron (BLE, VE.Direct), LuxPower (lxp-bridge), Tigo (taptap) and the EG4 mini split bridge; Modbus-based inverters need only tokio-modbus plus a register table.

### Gaps
- Every row in the recalled table is unverified this session: baud rates, IDs and pinouts for Pylontech RS485, PACE, Seplos, Daly, JK, EG4 LL/LifePower4, VE.Can, VE.Bus, SunSpec, Growatt, Sol-Ark/Deye, LuxPower dongle, Schneider, SMA, Renogy, Epever, Outback, Midnite, Enphase, Tigo and APsystems.
- No official Pylontech RS485 or CAN protocol PDF was fetched.
- VE.Direct baud rate and connector type were not on the Victron FAQ page I read (it defers to the whitepaper).
- Whether third parties may write Victron settings through VE.Direct HEX was not stated on the FAQ page.

---

## Key Question 5 — Hardware adapters: isolated USB-RS485 / RS232 / CAN, Pi HATs, RJ45 breakouts, why isolation matters, prices

### Takeaway
Roughly USD 60–80 of Waveshare parts covers the owner's gear: one isolated RS485+CAN HAT (USD 21.99) and two isolated USB RS232/485 converters (USD 18.99 each), plus breakout boards. Isolation is not optional on 48 V stacks because a BMS that opens the negative leg can route pack current through the data ground.

### Cited Findings
- **[MANUAL-level docs]** dbus-serialbattery warns that the data link to a BMS must be isolated: if the BMS disconnects the negative pole, return current can flow through the data cable and damage both the computer and the BMS. — [dbus-serialbattery: connect](https://mr-manuel.github.io/venus-os_dbus-serialbattery_docs/next/general/connect)
- **[MANUAL]** Victron says galvanic isolation on VE.Direct interfaces matters because the products themselves have very little protection. — [Victron VE.Direct FAQ](https://www.victronenergy.com/live/vedirect_protocol:faq)
- **[MANUAL]** On the JBD UP16S the RS485 port is isolated but the RS232 UART is not. — [JBD UP16S010 spec sheet](https://cdn.shopify.com/s/files/1/0606/5199/5298/files/JBD-UP16S010-L16S-200A-200A-B-U-R-C-A03.pdf)
- **Waveshare USB TO RS232/485 (B)**, SKU 26547: USD 18.99 (qty 1) on waveshare.com; FT232RNL; power and signal isolation (voltage rating not stated on page); isolated RS232 and RS485 both on screw terminals (no DB9); automatic RS485 direction control; jumper-selectable 120 ohm terminator; 300–921600 bps; TVS, 600 W surge, 15 kV ESD, resettable fuse; DIN-rail case; Linux supported. — [Waveshare USB TO RS232/485 (B)](https://www.waveshare.com/product/iot-communication/usb-to-rs232-485-b.htm)
- **Waveshare Isolated RS485 CAN HAT (B)**, SKU 23227: USD 21.99; two RS485 channels (SC16IS752 + SP3485) and one CAN channel (MCP2515 + SN65HVD230-class transceiver) over SPI; digital isolator plus isolated DC-DC; TVS and resettable fuse; 8–28 V DC input that can also power the Pi; 40-pin header. Pi 5 / CM4 are not named explicitly and SocketCAN is not mentioned on the page. — [Waveshare RS485 CAN HAT (B)](https://www.waveshare.com/rs485-can-hat-b.htm)
- **Waveshare USB-CAN-A**, SKU 23635: USD 18.99; STM32F103; 5 kbit/s–1 Mbit/s; appears as a USB virtual COM port with its own serial protocol (SocketCAN not mentioned); has "silent" and "silent loopback" modes; isolation is not claimed, only a TVS. — [Waveshare USB-CAN-A](https://www.waveshare.com/usb-can-a.htm)
- Other candidates that exist but whose price/spec pages I did not open: Waveshare USB-CAN-B (dual channel, "industrial isolation"), Waveshare USB TO 4CH RS485 (B) (isolated, CH344L), Waveshare 2-CH isolated CAN HAT, Waveshare FT232RNL USB TO RS232/485/422/TTL isolated converter. — [USB-CAN-B](https://www.waveshare.com/product/iot-communication/usb-can-b.htm); [USB TO 4CH RS485 (B)](https://www.waveshare.com/product/usb-to-4ch-rs485-b.htm); [2-CH CAN HAT](https://www.waveshare.com/product/raspberry-pi/hats/interface-power/2-ch-can-hat.htm); [USB TO RS232/485/422/TTL](https://www.waveshare.com/product/usb-to-rs232-485-422-ttl.htm)
- **[CODE docs]** dbus-serialbattery's recommended isolated UART adapters are Victron's own VE.Direct-to-USB cables (affiliate-linked on that page). — [dbus-serialbattery: connect](https://mr-manuel.github.io/venus-os_dbus-serialbattery_docs/next/general/connect)

### Inferences
- Suggested kit inside the USD 40–100 share (prices from waveshare.com, Amazon not checked):
  1. Isolated RS485 CAN HAT (B), USD 21.99: battery RS485 (JBD native), a second RS485 for a sniff tap or Modbus inverter, and CAN for Pylontech-style listening. MCP2515 gives native SocketCAN on Linux (standard `mcp2515` overlay; not stated on the product page).
  2. Two USB TO RS232/485 (B), USD 37.98: one per paralleled inverter RS232 port, and either can be re-purposed as isolated RS485.
  3. RJ45 screw-terminal breakout boards (x3–4), one RJ11/RJ12 6P6C breakout for the JBD RS232 jack, short Cat5e patch leads, an inline RJ45 splitter for bus taps, 120 ohm resistors. Prices not sourced; typically a few dollars each.
  Subtotal for sourced items: about USD 60.
- "Universal solar comms pigtail kit" concept: do the pin mapping in screw terminals or a small patch panel inside the pack, and keep per-vendor profile cards (JBD RS485 1/2/3, EG4 battery lead 1/2/3, SMG-II RS232 1/2/8 with 4 isolated, Pylontech CAN 4/5/6). Label any jack that carries supply voltage (EG4 RS232 pin 4, Epever pins 1–2).
- The HAT's two RS485 channels and CAN channel most likely share one isolated ground. They are isolated from the Pi but not from each other, so do not connect them to two systems at different potentials at once.
- The USB-CAN-A is a poor fit: not isolated and not SocketCAN. A gs_usb/candleLight adapter with isolation, or the HAT, is preferable.
- For the non-isolated JBD RS232 jack use the isolated RS232 terminals on the USB TO RS232/485 (B). RS232 on that jack is true RS232-level per the "RS232" label, but the sheet also calls it UART; measure idle voltage on TX before connecting (negative idle = RS232, +3.3/5 V idle = TTL).
- A HAT does not fit a Home Assistant Yellow or PiKVM enclosure; USB adapters are the portable-across-hosts choice.

### Gaps
- No Amazon / US-stock prices were verified; all prices are waveshare.com list prices on the day of research. Amazon listings typically run a few dollars higher.
- DSD TECH SH-U11, DTECH, Innomaker USB2CAN and isolated CANable variants: product pages and prices not retrieved.
- Isolation voltage ratings were not stated on the pages read.
- Pi 5 compatibility of the SPI HAT (overlay names, interrupt pins) not confirmed.
- RJ45 breakout and pigtail prices not sourced.

---

## Key Question 6 — Passive sniffing and port sharing

### Takeaway
CAN between battery and inverter can be tapped in parallel and read without transmitting, provided the tap adds no termination and never breaks the link; at the owner's site the cleanest "share" with Solar Assistant is its MQTT broker rather than splitting serial ports.

### Cited Findings
- **[FORUM]** A working passive CAN tap: CAN pins 4 and 5 were paralleled across two sockets of a wall faceplate so the battery-inverter cable passes through and a second lead goes to a listener (a USR-CANET200 CAN-to-Ethernet box); the author describes it as listen-only with nothing written to battery or inverter. — [HA community: Pylontech-Solis CAN](https://community.home-assistant.io/t/reading-can-bus-communication-between-pylontech-battery-and-solis-inverter/420788)
- **[FORUM]** Battery and inverter already terminate the bus, so the tap's terminators stay off; the battery-inverter link has to stay intact even with the tap powered down; one user who lost the CAN link saw the inverter treat the battery as flat and start a full grid charge, another saw charge and discharge stop. — [same thread](https://community.home-assistant.io/t/reading-can-bus-communication-between-pylontech-battery-and-solis-inverter/420788)
- **[CODE wiki]** OpenDTU-OnBattery only needs a receive-capable CAN module for Pylontech monitoring; cheap SN65HVD230 boards may need their built-in terminator removed before the battery keeps talking. — [OpenDTU-OnBattery wiki](https://github.com/helgeerbe/OpenDTU-OnBattery/wiki/Pylontech-battery)
- **[MANUAL-level]** Solar Assistant republishes everything it reads over MQTT (port 1883 in the examples; subscribe to `#`), and write topics are off by default. — [Solar Assistant MQTT](https://solar-assistant.io/help/integration/mqtt)
- **[CODE]** On the UP16S, detail for other packs relayed through a master is slow (about 50 s) while a direct connection is about 1 s. — [dbus-serialbattery: connect](https://mr-manuel.github.io/venus-os_dbus-serialbattery_docs/next/general/connect)
- **[CODE]** The Rust `socketcan` crate (4.0.0, MIT) exposes CAN control modes and interface parameters through its netlink module. — [docs.rs socketcan](https://docs.rs/socketcan/latest/socketcan/)

### Inferences
- **[BACKGROUND]** CAN: put the interface in listen-only mode (`ip link set can0 type can bitrate 500000 listen-only on`) so the controller neither ACKs nor sends error frames. MCP2515 supports this. A listen-only node on a two-node bus is invisible; a normal-mode node at the wrong bitrate will inject error frames and can knock the BMS link down.
- **[BACKGROUND]** RS485: a passive tap is A/B (and reference ground through the isolated side) wired in parallel to a receiver whose driver is never enabled. With auto-direction USB adapters simply never write to the port; for certainty, cut or leave unconnected the adapter's DE/driver path, or use a receive-only transceiver. Keep the stub short and add no terminator. Decoding then means parsing both master requests and slave replies from one stream (Modbus RTU needs gap-based framing; 3.5-character silent intervals are unreliable through USB adapters, so use CRC-sliding-window resync instead).
- A second bus master on a Modbus/JBD RS485 segment causes collisions with the inverter or Solar Assistant. Rule for the agent: sniff by default; become master only on a segment with no other master (for example the unused RS485 jack of a pack that is not the inverter-facing master, and only if the BMS tolerates it).
- **[BACKGROUND]** Sharing one serial device among programs: `ser2net` or `socat` can expose a tty over TCP, but two independent masters still interleave requests; a proper share needs a request-serialising proxy (Modbus gateway such as mbusd, or a small Rust broker that owns the port and offers a queue). Linux does not arbitrate two processes opening the same tty.
- For the owner's site: (1) Solar Assistant MQTT for inverter data and whatever battery data SA already reads, (2) CAN/RS485 listen-only tap on the inverter-BMS link if closed-loop is in use, (3) direct master access to individual packs through their free RS485 jacks during a diagnostic session.

### Gaps
- No sourced write-up of passive RS485 (as opposed to CAN) sniffing on an inverter-BMS link was retrieved.
- Unknown whether the owner's inverters run closed-loop with the battery master (and over which port), or how Solar Assistant is currently attached to the batteries; this decides which segments are free.
- ser2net / socat / mbusd behaviour not sourced this session.

---

## Key Question 7 — Rust and other software, fixtures and simulators

### Takeaway
Rust has solid transport crates (tokio-modbus 0.17, socketcan 4.0) and Victron decoders (victron_ble, several VE.Direct crates), but I found no Rust implementations of PI30, Pylontech, SMG-II/Sumry or JBD (classic or UP16S), which is where "jbd-up" and "deadzone" can add something new. tokio-modbus's server features make in-process device simulators for CI straightforward.

### Cited Findings
- `tokio-modbus` 0.17.0, MIT OR Apache-2.0: async RTU and TCP clients by default, optional sync clients, and RTU, TCP and RTU-over-TCP **server** features; serial via tokio-serial. — [docs.rs tokio-modbus](https://docs.rs/tokio-modbus/latest/tokio_modbus/)
- `socketcan` 4.0.0, MIT: optional tokio and smol async support, CAN FD types, netlink interface configuration enabled by default. — [docs.rs socketcan](https://docs.rs/socketcan/latest/socketcan/)
- `victron_ble` 1.2.0, MIT (docs.rs shows a 6 Nov 2026 publish date): `open_stream` on Linux and macOS via optional `bluer` / `bluest`; with the bluetooth feature off it still offers `parse_manufacturer_data` and builds `no_std`; optional serde. — [docs.rs victron_ble](https://docs.rs/victron_ble)
- VE.Direct crates exist under three names: `vedirect` 0.2.0, `ve_direct` 0.1.5, `vedirect_rs` 0.2.3. — [docs.rs vedirect](https://docs.rs/crate/vedirect/latest); [docs.rs ve_direct](https://docs.rs/crate/ve_direct/latest/source/); [docs.rs vedirect_rs](https://docs.rs/crate/vedirect_rs/0.2.3)
- Rust project in the owner's own equipment space: eg4heatpumptuyatomqtt (local Tuya to MQTT for the EG4 mini split). — [tfoote000/eg4heatpumptuyatomqtt](https://github.com/tfoote000/eg4heatpumptuyatomqtt)
- Python references read this session: PythonProtocolGateway (CSV register maps per protocol, Modbus RTU to MQTT; fork of growatt2mqtt) — [repo](https://github.com/HotNoob/PythonProtocolGateway); mpp-solar (MIT) — [repo](https://github.com/jblance/mpp-solar); powermon protocol docs — [PI30MAX](https://jblance.github.io/powermon/protocoldocs.inverter.pi30max.html); dbus-serialbattery (Venus OS driver, per-BMS Python modules) — [UP16S driver](https://github.com/mr-manuel/venus-os_dbus-serialbattery/blob/master/dbus-serialbattery/bms/lltjbd_up16s.py); aiobmsble via BMS_BLE-HA (Apache-2.0) — [repo](https://github.com/patman15/BMS_BLE-HA); ha-ef-ble (Apache-2.0) — [repo](https://github.com/rabits/ha-ef-ble).
- OpenDTU-OnBattery ships a Pylontech CAN emulator setup for Linux (`can0` at 500000) that alters the keep-alive message each second, usable as a test source. — [OpenDTU-OnBattery wiki](https://github.com/helgeerbe/OpenDTU-OnBattery/wiki/Pylontech-battery)

### Inferences
- Porting plan by value for the owner: (1) UP16S native protocol from `lltjbd_up16s.py` into a `no_std`-friendly Rust codec crate ("jbd-up"); (2) SMG-II/`eg4_3000ehv_v1` register table on tokio-modbus (the CSV maps in PythonProtocolGateway can be consumed almost directly as data files, licence permitting); (3) Pylontech CAN 0x35x decoder on socketcan; (4) reuse victron_ble and a VE.Direct crate as-is; (5) PI30 codec for Axpert-family field work.
- CI fixtures without hardware: tokio-modbus RTU server over a pseudo-terminal pair (`socat -d -d pty,raw pty,raw`) for Modbus devices; Linux `vcan` plus recorded candump logs for CAN; byte-level golden frames captured from the real UP16S for the custom protocol; mpp-solar has a "test" port and dbus-serialbattery has per-driver parsing that can provide expected values. (vcan, socat ptys and mpp-solar's test port are background knowledge.)
- Structure protocol crates as pure codecs (bytes in, typed frames out) separate from transports; that keeps them testable, sniff-capable (decode both directions) and reusable on ESP32.
- **[BACKGROUND, NOT VERIFIED]** Other Rust prior art: `serialport` and `tokio-serial` crates; `btleplug` / `bluer` for BLE; celsworth/lxp-bridge (LuxPower/EG4 18kPV dongle protocol); willglynn/taptap (Tigo). Python: pymodbus, pysolarmanv5 (Deye/Solarman loggers), keshavdv/victron-ble, pysunspec2, pyenphase.

### Gaps
- Absence of Rust crates for PI30, Pylontech, JBD, Seplos, PACE is based on a single crates-oriented search that returned only Victron results; not an exhaustive crates.io audit.
- Licences of PythonProtocolGateway's register CSVs and of dbus-serialbattery were not confirmed on the pages read (PPG has a LICENSE file whose type was not shown).
- Version/maintenance status of mpp-solar versus its successor powermon was not determinable from the page.
- serialport crate, btleplug/bluer versions, pymodbus, pysolarmanv5, victron-ble (Python) not fetched.

---

## Key Question 8 — Complementary physical sensing for solar troubleshooting

### Takeaway
Open low-cost I-V curve tracer designs exist (IV Swinger 2 at about USD 50 in parts, plus several 2024–2025 Arduino-class academic designs), and a cheap Modbus multi-channel AC current module is already supported in the same gateway that supports the owner's inverter; the rest of the sensing list below is engineering judgement, not sourced research.

### Cited Findings
- IV Swinger 2 is published as a roughly USD 50 Arduino-based I-V curve tracer build (51-step write-up). — [Instructables: IV Swinger 2](https://www.instructables.com/IV-Swinger-2-a-50-IV-Curve-Tracer)
- Recent open-access designs: an Arduino-based tracer with automated load switching (Applied Sciences 2025) — [MDPI Appl. Sci. 15(15) 8186](https://www.mdpi.com/2076-3417/15/15/8186); a portable low-cost tracer intended for on-line, in-situ module inspection — [PMC11279202](https://www.ncbi.nlm.nih.gov/pmc/articles/PMC11279202/); a low-cost I-V and P-V prototype from Heriot-Watt — [research portal](https://researchportal.hw.ac.uk/en/publications/a-low-cost-prototype-i-v-and-p-v-curve-tracer-for-pv-modules/).
- PythonProtocolGateway already carries a protocol for a 16-channel AC current monitoring module (`hdhk_16ch_ac_module`), i.e. per-circuit AC current over Modbus is a solved, cheap add-on. — [HotNoob/PythonProtocolGateway](https://github.com/HotNoob/PythonProtocolGateway)
- BMS data itself gives per-cell voltages, four temperatures, MOSFET temperature, balance bitmask, cycle count and fault/alarm bits on the UP16S. — [dbus-serialbattery UP16S driver](https://github.com/mr-manuel/venus-os_dbus-serialbattery/blob/master/dbus-serialbattery/bms/lltjbd_up16s.py)

### Inferences (engineering judgement, **[BACKGROUND]**)
- DC clamp (Hall, 0–100 A or more) on each battery cable and each PV string: lets the agent cross-check BMS and inverter current readings, find a pack that is not sharing, and spot a weak string. Clamp with analog output into an isolated ADC is safer than inline shunts on 48 V/150–500 V circuits.
- PV string voltage: the owner's inverter takes up to 500 Voc. Anything that touches PV conductors needs a properly rated isolated divider/isolation amplifier, fusing and CAT III 600 V/1000 V leads. IV Swinger 2 is a single-module/low-voltage instrument in its stock form; string-level tracing at several hundred volts is a different safety class and should be human-operated.
- Irradiance and module temperature: a small reference cell (short-circuit current through a shunt) or a pyranometer-substitute photodiode, plus a stick-on thermistor/DS18B20 on the module back; with these two values the agent can compute expected string power and flag under-performance.
- Thermal imaging of lugs, breakers, bus bars, MC4s and battery terminals under load: low-cost Lepton/MLX90640-class sensors show hot joints; best paired with the DC clamp so the agent knows the load at capture time.
- AC power quality on inverter output: isolated voltage sensing plus CT sampled at a few kHz gives THD, frequency stability, sag on motor start and neutral-ground voltage; relevant because the 3000EHV is 120 V only and paralleled.
- Arc/noise detection: wideband current probe or small loop antenna on the PV home-run looking for broadband noise in the 10–500 kHz band; also shows MPPT switching ripple and inverter EMI.
- Ground-fault / insulation: insulation-resistance (megger-style) testing injects high voltage and should stay human-only; a passive check of PV+ and PV- to ground voltage balance through high-value isolated dividers is agent-safe and flags a leaning array.
- Battery voltage at the terminals versus at the inverter under load (two isolated voltage taps) yields cable/joint resistance directly.

### Gaps
- IV Swinger 2 voltage/current limits, licence and repository were not readable (page returned metadata only).
- No sourced part numbers or prices for clamps, reference cells, thermal sensors or power-quality front ends; another researcher may be covering sensors.
- No sourced open design for DC arc detection or agent-safe insulation monitoring was found.

---

## Key Question 9 — Write-risk tiering

### Takeaway
The tools the owner already relies on treat writes as exceptional: Solar Assistant ships with MQTT writes disabled and offers verified writes, and the leading BLE BMS library refuses control entirely; the agent should follow the same posture with a tiered gate, keeping BMS protection limits, battery-type/voltage settings, output mode and firmware as human-only.

### Cited Findings
- Solar Assistant: setting changes over MQTT are off by default and must be enabled explicitly; `set_verified` writes, reads back and retries up to three times. — [Solar Assistant MQTT](https://solar-assistant.io/help/integration/mqtt)
- BMS_BLE-HA's author deliberately leaves out control because a BMS is safety-critical and vendor documentation is missing, and warns against using the data for safety functions. — [patman15/BMS_BLE-HA](https://github.com/patman15/BMS_BLE-HA)
- Writable 3000EHV-family registers include output mode (300), output priority (301), input range (302), output voltage and frequency (320–321), battery over-voltage protection point (323), charging priority (331), max charge current (332), equalization (335), turn-on mode (406), remote switch (420) and fault-mode exit (426). — [eg4_3000ehv_v1 register map](https://raw.githubusercontent.com/HotNoob/PythonProtocolGateway/main/protocols/eg4/eg4_3000ehv_v1.holding_registry_map.csv)
- EG4 manual constraints relevant to writes: bulk default 56.4 V (settable up to 62.0 V), low DC cut-off default 42.0 V (40.0–54.0), charge current 10–80 A per unit; in parallel only one unit is on the lithium/BMS protocol; AC output mode (single / parallel / phase) is program 28. — [EG4 3000EHV-48 V2 manual](https://eg4electronics.com/wp-content/uploads/2024/04/EG4-3000EHV-48-V2-Manual.pdf)
- PI30 setters on Voltronic-family units include factory reset (PF), battery type (PBT), output mode (POPM), bulk/float/cut-off voltages, charge currents, charge/discharge enable (PBATCD), max discharge current and data-log erase. — [powermon PI30MAX](https://jblance.github.io/powermon/protocoldocs.inverter.pi30max.html)
- JBD UP16S: the open driver's only write is SOC override (function 0x79 with a "JBD" magic prefix); the vendor PC tool additionally edits protection thresholds, chemistry and capacity behind a password and can flash firmware. JBD typical protection values: cell OV 3.65 V, cell UV 2.70 V, charge OC 230 A, short circuit 1800 A. — [dbus-serialbattery UP16S driver](https://github.com/mr-manuel/venus-os_dbus-serialbattery/blob/master/dbus-serialbattery/bms/lltjbd_up16s.py); [Eco-Worthy host PC software](https://au.eco-worthy.com/blogs/lithium-battery/how-to-use-the-host-pc-software); [JBD UP16S010 spec sheet](https://cdn.shopify.com/s/files/1/0606/5199/5298/files/JBD-UP16S010-L16S-200A-200A-B-U-R-C-A03.pdf)
- EG4 mini split: aggressive probing hangs the WiFi module until a breaker cycle. — [HA community thread](https://community.home-assistant.io/t/eg4-hybrid-mini-split-ac-integration/589783)
- ha-ef-ble exposes switches and sliders for EcoFlow outputs and charge parameters. — [rabits/ha-ef-ble](https://github.com/rabits/ha-ef-ble)

### Inferences — proposed tiers (my synthesis)
- **Tier 0, free: passive.** Listen-only CAN, receive-only RS485, BLE advertisements, Solar Assistant MQTT subscribe.
- **Tier 1, free with rate limits: active reads.** Modbus reads, PI30 queries, JBD 0x78 reads, BLE connect-and-read. Risks are bus contention with another master, waking sleeping BMSs (raises idle draw from 0.4 mA to tens of mA class), stealing the single BLE slot from the owner's phone app, and hanging fragile WiFi modules. Gate on "no other master detected" and back off on errors.
- **Tier 2, agent-allowed with confirmation and automatic read-back/rollback: reversible comfort settings.** Buzzer, backlight, display return, clock, energy-counter reads, mini-split setpoint/mode, EcoFlow AC/DC output toggles when nothing critical is powered.
- **Tier 3, human approval per change, bounded by a site profile: operating-mode settings.** Output source priority, charger source priority, max charge current and max utility charge current (within battery and wiring limits), back-to-utility / back-to-battery points, SOC set points, BMS SOC override. Use verified writes, one setting at a time, log before/after, and never during a fault state.
- **Tier 4, human-only (agent may draft the change but not send): anything that can damage hardware or drop loads.** Battery type / BMS protocol selection, bulk/float/equalization voltages and enabling equalization on lithium, low-DC cut-off, battery over-voltage point, AC output voltage/frequency, AC output mode (single/parallel/phase) on paralleled units, Modbus ID changes, remote on/off and turn-on mode, factory reset, fault-mode exit, any BMS protection threshold, capacity/chemistry, balance settings, MOSFET force on/off, BMS address or inverter-protocol selection, charge/discharge enable.
- **Tier 5, never from the pack: firmware.** Inverter, BMS, dongle and EcoFlow firmware flashing, and any undocumented register or function code. Add a hard deny-list in the MCP server rather than relying on prompt-level policy.
- Parallel-inverter specific hazard: writing a setting to one unit of a parallel pair that must match on both (output mode, voltage, frequency, battery type) can fault the pair; treat "must match across units" as a precondition check.
- Closed-loop hazard: changing the inverter battery type or the BMS inverter-protocol breaks BMS-inverter communication, and inverters differ in how they fail when it is lost (the cited forum cases include a forced grid charge and a halt of charging).
- The MCP server should expose read tools and write tools as separate tool families so that annotations, approval prompts and audit logs can differ by tier.

### Gaps
- No vendor document was found that ranks 3000EHV settings by risk or states which registers take effect immediately versus after restart.
- Behaviour of the owner's inverters on loss of BMS comms was not sourced for this model.
- No source located for known-bad writes on the UP16S (bricking reports, password values); DIY Solar Forum was unreachable.

---

## Ideas we may not have thought of (solar/battery sensing)

### Takeaway
The cheapest high-value additions are things that turn data the pack can already reach into diagnostics: per-pack sequential interrogation of the UP16S stack, bus-health measurements on the comms lines themselves, and cross-checks between independent sources.

### Cited Findings
- Direct per-pack connection gives about 1 s data versus about 50 s relayed through the master, and per-pack current limits are only available on a direct connection. — [dbus-serialbattery: connect](https://mr-manuel.github.io/venus-os_dbus-serialbattery_docs/next/general/connect); [UP16S driver](https://github.com/mr-manuel/venus-os_dbus-serialbattery/blob/master/dbus-serialbattery/bms/lltjbd_up16s.py)
- The UP16S exposes lifetime charge/discharge Ah, cycle count, SOH, per-cell balance bitmask and a history store. — [UP16S driver](https://github.com/mr-manuel/venus-os_dbus-serialbattery/blob/master/dbus-serialbattery/bms/lltjbd_up16s.py); [JBD UP16S010 spec sheet](https://cdn.shopify.com/s/files/1/0606/5199/5298/files/JBD-UP16S010-L16S-200A-200A-B-U-R-C-A03.pdf)
- Passive balancing on this BMS is only 20–60 mA. — [JBD UP16S010 spec sheet](https://cdn.shopify.com/s/files/1/0606/5199/5298/files/JBD-UP16S010-L16S-200A-200A-B-U-R-C-A03.pdf)
- Solar Assistant already republishes its view over MQTT. — [Solar Assistant MQTT](https://solar-assistant.io/help/integration/mqtt)
- Victron devices broadcast encrypted state over BLE that a Pi can decode without any wiring. — [docs.rs victron_ble](https://docs.rs/victron_ble)

### Inferences (ideas)
1. **Pack-walk mode.** The agent asks the human to move one RS485 lead from battery to battery (or uses a 6-way isolated mux later) and builds a per-pack report: cell spread at rest and under load, balance activity, cycle count, lifetime Ah, firmware version, address. Flags the odd pack and firmware mismatches across the six.
2. **Current-sharing audit.** Compare each pack's reported current during the same load window (or a DC clamp walked across the six pack cables). Unequal sharing points to cable-length/lug problems; 20–60 mA balancers cannot fix a pack that is routinely worked harder.
3. **Derived cell internal resistance.** From 1 s cell-voltage and current samples during a load step (well pump, mini split start) compute per-cell delta-V over delta-I. A rising cell shows up long before a protection trip. Software only.
4. **Three-way disagreement detector.** Inverter-reported battery voltage vs BMS pack voltage vs a pack-side voltage tap: differences under load equal cable and breaker drop; differences at rest equal calibration error. Same idea for SOC (BMS vs Solar Assistant vs coulomb count).
5. **Comms-bus health as a sensor.** Measure RS485 idle bias, differential amplitude, termination (DC resistance A-B with the bus idle), CRC error rate and reply latency; on CAN, error counters and bus load. Many "battery comms lost" faults are wiring, and nobody instruments the bus.
6. **Protocol fingerprinting.** Listen-only auto-baud plus signature matching (`~` ASCII frames, Modbus RTU CRC, 0xDD JBD, 0xA5 Daly, PI30 `(`, CAN 0x35x set, VE.Direct text labels) so the pack can announce what is on a wire before anyone tells it. Useful on strangers' systems and a good open-source contribution.
7. **BLE census.** A passive scan lists every Victron, JBD, JK, Eco-Worthy, EcoFlow and Renogy radio in range with signal strength, giving an instant inventory at a new site without touching anything.
8. **Rapid-shutdown keep-alive detector.** A clamp-on pickup on a PV home-run that only reports whether the SunSpec RSD power-line keep-alive is present, answering "are the panels being held off?" without opening anything. (Signal parameters are background knowledge and need verification.)
9. **String comparison instead of absolute measurement.** With two or more strings, compare string currents clamp-to-clamp at the same instant; a string that is low relative to its neighbour needs no irradiance sensor to be flagged.
10. **Night-time checks.** Reverse current into strings after dark (failed blocking / leakage), inverter idle draw, BMS sleep/wake behaviour, and per-pack self-discharge spread are all measurable when nothing else is happening.
11. **Morning insulation trend.** PV+ and PV- to ground voltage ratio logged at dawn when dew is present; a drifting ratio is an early ground-fault indicator, measured passively.
12. **Generator/utility input characterisation.** Frequency and voltage stability of the AC input while the inverter qualifies it, explaining "won't accept generator" complaints.
13. **Acoustic and vibration pickup.** A contact microphone on the inverter chassis or a breaker catches fan-bearing wear, relay chatter and arcing crackle; cheap, and correlates well with thermal images.
14. **Event-synchronised capture.** Trigger thermal snapshot + waveform capture + BMS dump when the inverter warning bitmask (registers 108–109 on the owner's units) changes, so intermittent faults leave evidence.
15. **Golden-frame library as a by-product.** Every capture session saves raw bus traffic as test fixtures for jbd-up / deadzone CI, so the simulators improve with field use.
16. **Emulated-BMS test source.** The same CAN/RS485 hardware can play a Pylontech-style BMS to an inverter on the bench (not on a live system) to test how an inverter reacts to limits and alarms; valuable for diagnosing closed-loop problems, but it is a Tier 4 capability.

### Gaps
- These ideas are engineering proposals; none was validated against a published design this session.
- SunSpec RSD signal frequencies and detectability with a clamp-on pickup need a sourced check.
- Whether the UP16S history store is readable over RS485 (rather than only by the PC tool) is unknown.
