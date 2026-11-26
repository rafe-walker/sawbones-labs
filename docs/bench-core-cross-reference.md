# Bench Core cross-reference

Every part on the [Bench Core parts list](bench-core-parts.md), with why it is there, what it belongs to, how it works in the system, and what it has to agree with. Started 26 November 2026.

The "open check" column is the cross-check itself: each entry is something that must be confirmed against a datasheet or the part in hand before ordering or wiring. Entries were written from the design and general knowledge of these parts. A separate check against product pages and datasheets is under way, and its findings replace the entries here as they land.

## How the Bench Core fits together

- **The brain** is the Pi 5. It runs the supervisor, records to the NVMe drive, and reaches every board through one switched USB hub.
- **The hub** gives each board its own port that the brain can power off and on.
- **The trust root** is one RP2350 board with the stop, arm and key switches, the panel display and button, and the armed rail. It decides whether anything may energise. It has its own 5 V supply so it runs with the Pi off.
- **The armed rail** is a 12 V supply passed through the stop switch and the arm switch, then a current-limited converter, then a switch the trust root controls.
- **Ground truth** is the voltage reference module, cross-checked by a meter.
- **Module 1** is an RP2350 board with the INA228, behind a USB isolator.
- **Module 2** is an RP2350 board with the serial transceivers, behind a USB isolator, wired to the inverter's port through an RJ45 breakout.
- **Build tools** are the debug probe and the logic analyzer, both on the hub.

Hub port budget: trust root, module 1, module 2, debug probe, logic analyzer, bought serial adapter. Six of seven ports.

## Rung 1: the brain

| Item | Part of | Why it is needed | How it works in the system | Has to agree with | Open check |
| --- | --- | --- | --- | --- | --- |
| Pi 5 4 GB kit: board, 27 W supply, cooler | Brain | Runs the supervisor, the recorder and the MCP server. Flashes and tests every other board | Mains to the 27 W supply to the Pi. Ethernet to the LAN. One USB port to the hub | NVMe board, hub, clock battery | That the kit's supply is a real 5 V 5 A unit, so the Pi's USB current limit is lifted |
| Clock battery | Brain | Keeps time across power cuts, so recordings are stamped correctly before the network is up | Plugs into the Pi's battery connector | Pi 5 | That it is the rechargeable type and charging is enabled in the Pi's settings |
| Geekworm X1001 NVMe board | Brain | The recorder writes all day. A card would wear out | Sits on the Pi and connects by the PCIe ribbon | Pi 5, cooler, drive length | That it fits with the active cooler in place, and whether it sits above or below the Pi |
| KingSpec 256 GB NVMe, 2280 | Brain | Storage for sessions and the knowledge store | Mounts on the X1001. The Pi boots from it | X1001 | That the Pi 5 boots from this drive. Some drives are not recognised |
| microSD 32 GB | Brain, bring-up | First boot, firmware update, and a rescue system | Used once to set boot order to NVMe, then kept as recovery | Pi 5 | None |
| Phidgets HUB0003, 7-port USB 2 | Brain | Per-port power switching, so the brain can power-cycle one hung board | Upstream to a Pi USB 2 port. Each board on its own port | 12 V adapter, the Pi, six boards | Its power connector and polarity. Whether it ships with an upstream cable. That switching works from the Pi |
| 12 V 3 A adapter, two | Brain and trust root | One powers the hub. One is the source of the armed rail | Barrel plug to the hub. The second feeds the stop and arm switches | Hub input range, rail converter | That the plug fits the hub. A barrel socket to screw-terminal adapter is needed for the rail and is not on the list |

## Rung 2: build tools

| Item | Part of | Why it is needed | How it works in the system | Has to agree with | Open check |
| --- | --- | --- | --- | --- | --- |
| Raspberry Pi Pico with headers | Build tools | A debug probe, so the brain can flash, halt and inspect an RP2350 | Runs the debug-probe firmware. USB to the hub, three wires to the target's debug pins | RP2350-Zero debug access | Whether the RP2350-Zero brings its debug pins out at all. If not, flashing is over USB only and this probe has nothing to connect to. Needs a micro-USB cable, which is not on the list |
| 8-channel 24 MHz logic analyzer | Build tools | Eyes on every wire: serial, I2C, the heartbeat, the rail-enable line | USB to the hub. The brain captures and decodes through sigrok | Signal voltages on the boards | That its inputs are safe at 5 V. It must never touch the RS232 side, which swings beyond that |

## Rung 3: the trust root

| Item | Part of | Why it is needed | How it works in the system | Has to agree with | Open check |
| --- | --- | --- | --- | --- | --- |
| Waveshare RP2350-Zero, four boards | Trust root, module 1, module 2, spare | The microcontroller for every built board. Two firmware slots, so a failed update rolls back | USB-C to a hub port. Runs our Rust firmware | Debug probe, pin count per role | Pin count against the trust root's needs: display, button, switches, lights, rail enable, rail feedback, heartbeat. Debug pin access, as above |
| Emergency stop, 22 mm, two NC contacts | Trust root | Cuts the armed rail by hand with the Pi and the controller both off | One contact in series with the rail's 12 V feed. The second tells the controller it was pressed | 12 V rail current, panel hole size | Contact rating at 12 V DC. That both contacts are normally closed |
| Key switch, 22 mm | Trust root | A deliberate, removable enable for the higher hazard class | Read by the controller as an input in the Bench Core. Later it gates a second rail | Panel hole size | Whether the key can be removed in both positions |
| Toggle switch with safety cover | Trust root | The arm switch. One input to the layered interlock | In series with the rail feed after the stop switch. Its lamp shows armed | 12 V rail current | Contact rating. How its built-in lamp is wired |
| Momentary LED push buttons | Trust root | The panel approval button. A press grants one lease | Contact to a controller input. Its LED is lit by the controller for a pending approval | 3.3 V logic, 12 V LED | The LED is a 12 V type and needs a driver transistor from 3.3 V logic |
| OLED 0.96 inch, I2C | Trust root | The trusted panel. Shows the pending action in the supervisor's words | I2C to the controller | 3.3 V logic | That the module runs at 3.3 V. Its I2C address |
| WS2812B strip | Trust root | Status lights driven by the controller, so they stay truthful if the Pi hangs | 5 V supply, one data line from the controller | 3.3 V logic | The strip expects a 5 V data level and the controller gives 3.3 V. A level shifter is needed and is not on the list |
| XL4015 converters, three | Trust root | One is the hardware cap on the armed rail. One makes 5 V for the controller so it runs with the Pi off | 12 V in. The rail unit is set to a fixed voltage and current limit by its trimmers | 12 V adapter, rail load | The range and stability of the current limit. It is a regulator, not a precision limiter, so the limit is measured, not assumed |
| AOD4184 MOSFET switch module | Trust root | The controller's switch on the armed rail | Driven by a controller pin through its optocoupler | 3.3 V drive, rail current | Whether it switches the positive or the negative side. Negative-side switching leaves the load live to ground, which matters for feedback sensing |
| 2-channel relay module | Trust root | A second, independent cut that opens when unpowered | Normally-open contact in series with the rail. Held closed only while the heartbeat is healthy | 5 V coil supply, 3.3 V logic | These boards are driven active-low from 5 V. From a 3.3 V pin they can fail to switch off unless the coil supply jumper is split |
| Indicator lights, 12 to 24 V | Trust root | Lamps wired straight to the rails, which software cannot switch off | One across the 12 V feed, one across the armed rail | 12 V rails | None |
| Inline fuse holders and fuses | Trust root | A fuse at the source of each rail | One after each 12 V adapter | Wire gauge, rail current | Fuse values chosen from the measured rail limit |

## Rung 4: ground truth

| Item | Part of | Why it is needed | How it works in the system | Has to agree with | Open check |
| --- | --- | --- | --- | --- | --- |
| AD584 voltage reference module | Ground truth | A known voltage on the bench. Every measuring module is checked against it at start-up and after any reflash | Its outputs feed the calibration pair through a relay on each module | Meter on hand, INA228 input range | What supply it needs: the 10 V output needs well above 10 V in. Whether the printed calibration values can be believed. It is cross-checked by a meter either way |

## Rung 5: the DC meter module

| Item | Part of | Why it is needed | How it works in the system | Has to agree with | Open check |
| --- | --- | --- | --- | --- | --- |
| INA228 breakout | Module 1 | The first real instrument: volts, amps, power and energy on a low-voltage circuit | I2C to its RP2350 board, which answers the module protocol | 3.3 V logic, voltage reference | The fitted shunt value and so the current range. That the address and alert pins are brought out |
| USB isolator, ADuM3160, two | Modules 1 and 2 | Breaks the ground path between a measuring module and the brain | Between the hub port and the module. The hub's port switch still cuts the module's power | Hub, RP2350 USB speed | How much current the isolated side can supply. These are full-speed only, which matches the RP2350 |

## Rung 6: the comms pod

| Item | Part of | Why it is needed | How it works in the system | Has to agree with | Open check |
| --- | --- | --- | --- | --- | --- |
| Waveshare isolated USB to RS232, RS485, TTL | Reference for module 2 | The bought reference. Proves the inverter's port and settings before the built pod is trusted | USB to the hub. Read by the brain directly | Inverter port levels and pinout | None beyond the inverter's pinout |
| Isolated TTL to RS485 module, type C | Module 2 | The RS485 side of the built pod | Between the RP2350's serial pins and the bus | 3.3 V logic, the listen-only rule | This type switches direction by itself and has no transmit-enable pin. The rule that the transmitter is powered only from the armed rail needs a transceiver with an enable pin, or a gate in the transmit line. A design conflict to resolve |
| MAX3232 module | Module 2 | The RS232 side of the built pod, for the inverter's serial port | Between the RP2350's serial pins and the inverter's RS232 lines | 3.3 V logic | That its transmit output can be left unconnected for listen-only |
| ADuM1201 isolators, three | Module 2 | Isolation of the serial lines inside the pod | Between the RP2350 and a transceiver | Isolated supply on the far side | Redundant while the whole pod is isolated at USB. They need an isolated supply on the bus side, which is not on the list. Keep one plan and drop the other |
| RJ45 screw-terminal adapters | Module 2 | The inverter's port is an RJ45 socket | A patch lead from the inverter to the adapter, then individual wires to the pod | The inverter's pinout | The pinout must be confirmed on the real unit before anything is connected. One pin carries 12 V on the sibling model and is left open |

## Wiring and hardware

| Item | Part of | Why it is needed | How it works in the system | Has to agree with | Open check |
| --- | --- | --- | --- | --- | --- |
| Prototype boards | All built boards | Hand-wired first, before any board is ordered | One per module and one for the trust root | RP2350-Zero pin spacing | None |
| 22 AWG stranded wire | Rails and signals | Rail and panel wiring | | Fuse values | Rated for the rail current with margin |
| Dupont jumper wires | Bring-up | Temporary connections and logic analyzer leads | | | Not for the rails |
| Screw terminal blocks | All built boards | Field connections that can be remade | On the edge of each prototype board | Wire gauge | None |
| M2.5 standoffs | Brain and boards | Mounts the Pi, its drive board and the prototype boards to the bench plate | | Bench plate design | None |
| Short USB-C cables, six | Hub to boards | One per RP2350 board | USB-A at the hub, USB-C at the board | Hub port type | That they carry data, not charge only |

## Gaps found so far

Things the list does not have and the build needs. None is ordered yet.

| Gap | Why |
| --- | --- |
| A level shifter for the light strip | 3.3 V logic into a 5 V data input |
| Driver transistors and resistors | For the 12 V button LEDs and general pull-ups |
| Barrel socket to screw-terminal adapters | To bring the second 12 V adapter onto the rail wiring |
| A micro-USB cable | For the Pico used as the debug probe |
| A 12 V bulb and holder | A real load for testing the rail's limit |
| An RS485 transceiver with an enable pin | If the listen-only rule is kept as designed |
| 0.1% resistors | For the on-board reference divider |
| Possibly a different RP2350 board for the trust root | If the Zero has no debug pins |

## Questions the cross-check has raised

1. **Debug access.** If the RP2350-Zero has no debug pins, either the trust root uses a board that does, or flashing and testing go over USB only and the debug probe is dropped.
2. **Isolation plan for the comms pod.** Isolate the whole pod at USB, or isolate the serial lines inside it. The list currently buys parts for both.
3. **Listen-only by wiring.** The RS485 module on the list cannot have its transmitter disabled by a rail.
4. **Which side the rail switch breaks.** Feedback sensing and the lamp wiring depend on it.
