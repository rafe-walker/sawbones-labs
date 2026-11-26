# Bench Core cross-reference

Every part on the [Bench Core parts list](bench-core-parts.md), with why it is there, what it belongs to, how it works in the system, and what it has to agree with. Started 26 November 2026.

The "open check" column is what had to be confirmed for each part. The last column is what the check found. A summary of the changes this forces is [at the end](#what-the-check-found-26-november).

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

| Item | Part of | Why it is needed | How it works in the system | Has to agree with | Open check | Result of the check |
| --- | --- | --- | --- | --- | --- | --- |
| Pi 5 4 GB kit: board, 27 W supply, cooler | Brain | Runs the supervisor, the recorder and the MCP server. Flashes and tests every other board | Mains to the 27 W supply to the Pi. Ethernet to the LAN. One USB port to the hub | NVMe board, hub, clock battery | That the kit's supply is a real 5 V 5 A unit, so the Pi's USB current limit is lifted | **Unverified.** The kit's supply and cooler are third-party, not Raspberry Pi's own. If the supply does not offer 5 V at 5 A, the Pi limits its USB current. Test on arrival, or add the official 27 W supply. |
| Clock battery | Brain | Keeps time across power cuts, so recordings are stamped correctly before the network is up | Plugs into the Pi's battery connector | Pi 5 | That it is the rechargeable type and charging is enabled in the Pi's settings | **Confirmed.** Charging is off by default and is turned on with one config line. That the cell is the rechargeable type is checked on the part. |
| Geekworm X1001 NVMe board | Brain | The recorder writes all day. A card would wear out | Sits on the Pi and connects by the PCIe ribbon | Pi 5, cooler, drive length | That it fits with the active cooler in place, and whether it sits above or below the Pi | **Unverified.** Fit under the kit's third-party cooler is not documented. Check on assembly. |
| KingSpec 256 GB NVMe, 2280 | Brain | Storage for sessions and the knowledge store | Mounts on the X1001. The Pi boots from it | X1001 | That the Pi 5 boots from this drive. Some drives are not recognised | **Risk.** Not on any known-good list for booting a Pi 5. Swap for the Raspberry Pi SSD 256 GB, or accept the risk. |
| microSD 32 GB | Brain, bring-up | First boot, firmware update, and a rescue system | Used once to set boot order to NVMe, then kept as recovery | Pi 5 | None | No change. |
| Phidgets HUB0003, 7-port USB 2 | Brain | Per-port power switching, so the brain can power-cycle one hung board | Upstream to a Pi USB 2 port. Each board on its own port | 12 V adapter, the Pi, six boards | Its power connector and polarity. Whether it ships with an upstream cable. That switching works from the Pi | **Confirmed:** 8 to 30 V in, on a 5.5 x 2.1 mm centre-positive jack, 500 mA a port. Port switching is done with `uhubctl`; Phidgets does not document it, so it is tested on arrival. Upstream connector and cable are not confirmed. |
| 12 V 3 A adapter, two | Brain and trust root | One powers the hub. One is the source of the armed rail | Barrel plug to the hub. The second feeds the stop and arm switches | Hub input range, rail converter | That the plug fits the hub. A barrel socket to screw-terminal adapter is needed for the rail and is not on the list | **Wrong plug.** The adapter is 5.5 x 2.5 mm and the hub wants 2.1 mm, so the fit is loose. Buy a true 2.1 mm supply for the hub. For the rail's supply see the XL4015 row. |

## Rung 2: build tools

| Item | Part of | Why it is needed | How it works in the system | Has to agree with | Open check | Result of the check |
| --- | --- | --- | --- | --- | --- | --- |
| Raspberry Pi Pico with headers | Build tools | A debug probe, so the brain can flash, halt and inspect an RP2350 | Runs the debug-probe firmware. USB to the hub, three wires to the target's debug pins | RP2350-Zero debug access | Whether the RP2350-Zero brings its debug pins out at all. If not, flashing is over USB only and this probe has nothing to connect to. Needs a micro-USB cable, which is not on the list | **Confirmed** as a probe for the RP2350 with probe-rs. Needs a micro-USB cable. Its ground is the brain's ground: see the rule below. |
| 8-channel 24 MHz logic analyzer | Build tools | Eyes on every wire: serial, I2C, the heartbeat, the rail-enable line | USB to the hub. The brain captures and decodes through sigrok | Signal voltages on the boards | That its inputs are safe at 5 V. It must never touch the RS232 side, which swings beyond that | **Confirmed:** inputs to 5.25 V. Its ground is the brain's ground, so it never goes on an isolated module that is wired to the inverter. |

## Rung 3: the trust root

| Item | Part of | Why it is needed | How it works in the system | Has to agree with | Open check | Result of the check |
| --- | --- | --- | --- | --- | --- | --- |
| Waveshare RP2350-Zero, four boards | Trust root, module 1, module 2, spare | The microcontroller for every built board. Two firmware slots, so a failed update rolls back | USB-C to a hub port. Runs our Rust firmware | Debug probe, pin count per role | Pin count against the trust root's needs: display, button, switches, lights, rail enable, rail feedback, heartbeat. Debug pin access, as above | **Confirmed, two cautions.** Debug access is three solder pads underneath: wire them before the board is mounted. The 5 V pin is the USB supply with no diode, so a board with its own supply must not also take USB power. Pin count is enough: 14 of 20 edge pins. |
| Emergency stop, 22 mm, two NC contacts | Trust root | Cuts the armed rail by hand with the Pi and the controller both off | One contact in series with the rail's 12 V feed. The second tells the controller it was pressed | 12 V rail current, panel hole size | Contact rating at 12 V DC. That both contacts are normally closed | **Unverified.** Contact count and DC rating come from the part in hand. |
| Key switch, 22 mm | Trust root | A deliberate, removable enable for the higher hazard class | Read by the controller as an input in the Bench Core. Later it gates a second rail | Panel hole size | Whether the key can be removed in both positions | **Unverified.** Key removal positions come from the part in hand. |
| Toggle switch with safety cover | Trust root | The arm switch. One input to the layered interlock | In series with the rail feed after the stop switch. Its lamp shows armed | 12 V rail current | Contact rating. How its built-in lamp is wired | **Unverified.** Lamp wiring and rating come from the part in hand. |
| Momentary LED push buttons | Trust root | The panel approval button. A press grants one lease | Contact to a controller input. Its LED is lit by the controller for a pending approval | 3.3 V logic, 12 V LED | The LED is a 12 V type and needs a driver transistor from 3.3 V logic | **Gap confirmed.** One transistor per LED: 2N3904s or a ULN2003A. LED voltage from the part in hand. |
| OLED 0.96 inch, I2C | Trust root | The trusted panel. Shows the pending action in the supervisor's words | I2C to the controller | 3.3 V logic | That the module runs at 3.3 V. Its I2C address | **Confirmed** at 3.3 V. Pull-ups on the board are checked on the part. |
| WS2812B strip | Trust root | Status lights driven by the controller, so they stay truthful if the Pi hangs | 5 V supply, one data line from the controller | 3.3 V logic | The strip expects a 5 V data level and the controller gives 3.3 V. A level shifter is needed and is not on the list | **Gap confirmed.** Add a 74AHCT125 level shifter. |
| XL4015 converters, three | Trust root | One is the hardware cap on the armed rail. One makes 5 V for the controller so it runs with the Pi off | 12 V in. The rail unit is set to a fixed voltage and current limit by its trimmers | 12 V adapter, rail load | The range and stability of the current limit. It is a regulator, not a precision limiter, so the limit is measured, not assumed | **Role changed.** It limits current by regulating, slowly. It is not a hard cap. It also cannot make 12 V from 12 V. Feed the rail from a 15 V adapter and add a 2 A fuse as the hard limit. The limit's accuracy is measured, not assumed. |
| AOD4184 MOSFET switch module | Trust root | The controller's switch on the armed rail | Driven by a controller pin through its optocoupler | 3.3 V drive, rail current | Whether it switches the positive or the negative side. Negative-side switching leaves the load live to ground, which matters for feedback sensing | **Confirmed:** it switches the negative side and turns on from 3.3 V. Fine for the lamp test. Before the rail feeds anything that shares a ground, the controller's cut moves to the positive side. |
| 2-channel relay module | Trust root | A second, independent cut that opens when unpowered | Normally-open contact in series with the rail. Held closed only while the heartbeat is healthy | 5 V coil supply, 3.3 V logic | These boards are driven active-low from 5 V. From a 3.3 V pin they can fail to switch off unless the coil supply jumper is split | **Confirmed** active-low. It is not driven from a pin directly. A small heartbeat stage, a diode pump and one transistor, drives it, so a pin stuck high or low drops the relay. |
| Indicator lights, 12 to 24 V | Trust root | Lamps wired straight to the rails, which software cannot switch off | One across the 12 V feed, one across the armed rail | 12 V rails | None | No change. Voltage checked on the part. |
| Inline fuse holders and fuses | Trust root | A fuse at the source of each rail | One after each 12 V adapter | Wire gauge, rail current | Fuse values chosen from the measured rail limit | **Values set:** 2 A on the rail branch, 1 A on the trust root's branch. The trust root's branch is taken before the stop switch, so the stop never powers the controller down. |

## Rung 4: ground truth

| Item | Part of | Why it is needed | How it works in the system | Has to agree with | Open check | Result of the check |
| --- | --- | --- | --- | --- | --- | --- |
| AD584 voltage reference module | Ground truth | A known voltage on the bench. Every measuring module is checked against it at start-up and after any reflash | Its outputs feed the calibration pair through a relay on each module | Meter on hand, INA228 input range | What supply it needs: the 10 V output needs well above 10 V in. Whether the printed calibration values can be believed. It is cross-checked by a meter either way | **12 V is too low** for the 10 V output, which needs 12.5 V. On 15 V all four outputs work. Grade and accuracy come from the part in hand. |

## Rung 5: the DC meter module

| Item | Part of | Why it is needed | How it works in the system | Has to agree with | Open check | Result of the check |
| --- | --- | --- | --- | --- | --- | --- |
| INA228 breakout | Module 1 | The first real instrument: volts, amps, power and energy on a low-voltage circuit | I2C to its RP2350 board, which answers the module protocol | 3.3 V logic, voltage reference | The fitted shunt value and so the current range. That the address and alert pins are brought out | No change. Shunt value and pins come from the part in hand. Add 100 ohm and 10 ohm 0.1% resistors for the current check. |
| USB isolator, ADuM3160, two | Modules 1 and 2 | Breaks the ground path between a measuring module and the brain | Between the hub port and the module. The hub's port switch still cuts the module's power | Hub, RP2350 USB speed | How much current the isolated side can supply. These are full-speed only, which matches the RP2350 | No change. Plan on 200 mA downstream. Speed switch set to full speed. |

## Rung 6: the comms pod

| Item | Part of | Why it is needed | How it works in the system | Has to agree with | Open check | Result of the check |
| --- | --- | --- | --- | --- | --- | --- |
| Waveshare isolated USB to RS232, RS485, TTL | Reference for module 2 | The bought reference. Proves the inverter's port and settings before the built pod is trusted | USB to the hub. Read by the brain directly | Inverter port levels and pinout | None beyond the inverter's pinout | No change. |
| Isolated TTL to RS485 module, type C | Module 2 | The RS485 side of the built pod | Between the RP2350's serial pins and the bus | 3.3 V logic, the listen-only rule | This type switches direction by itself and has no transmit-enable pin. The rule that the transmitter is powered only from the armed rail needs a transceiver with an enable pin, or a gate in the transmit line. A design conflict to resolve | **Drop from this order.** The inverter's monitoring port is RS232. RS485 is the battery port, a later job, and it needs a part with an enable pin. |
| MAX3232 module | Module 2 | The RS232 side of the built pod, for the inverter's serial port | Between the RP2350's serial pins and the inverter's RS232 lines | 3.3 V logic | That its transmit output can be left unconnected for listen-only | **Now the main part of module 2.** Its two receivers listen to both directions of the link. Its transmitter outputs stay unconnected. |
| ADuM1201 isolators, three | Module 2 | Isolation of the serial lines inside the pod | Between the RP2350 and a transceiver | Isolated supply on the far side | Redundant while the whole pod is isolated at USB. They need an isolated supply on the bus side, which is not on the list. Keep one plan and drop the other | **Drop.** The pod is isolated at USB. These would need their own isolated supply. |
| RJ45 screw-terminal adapters | Module 2 | The inverter's port is an RJ45 socket | A patch lead from the inverter to the adapter, then individual wires to the pod | The inverter's pinout | The pinout must be confirmed on the real unit before anything is connected. One pin carries 12 V on the sibling model and is left open | **Used as a pass-through tap,** two back to back, so Solar Assistant stays connected. Three signal pins only, and the pin that carries 12 V on the sibling model is left open. The pinout is confirmed on the real unit first. |

## Wiring and hardware

| Item | Part of | Why it is needed | How it works in the system | Has to agree with | Open check | Result of the check |
| --- | --- | --- | --- | --- | --- | --- |
| Prototype boards | All built boards | Hand-wired first, before any board is ordered | One per module and one for the trust root | RP2350-Zero pin spacing | None | No change. |
| 22 AWG stranded wire | Rails and signals | Rail and panel wiring | | Fuse values | Rated for the rail current with margin | No change at 2 A. |
| Dupont jumper wires | Bring-up | Temporary connections and logic analyzer leads | | | Not for the rails | No change. |
| Screw terminal blocks | All built boards | Field connections that can be remade | On the edge of each prototype board | Wire gauge | None | No change. |
| M2.5 standoffs | Brain and boards | Mounts the Pi, its drive board and the prototype boards to the bench plate | | Bench plate design | None | No change. |
| Short USB-C cables, six | Hub to boards | One per RP2350 board | USB-A at the hub, USB-C at the board | Hub port type | That they carry data, not charge only | **Unverified.** Test each for data. The trust root's lead has its power wire cut. |

## What the check found (26 November)

Three checks against datasheets and maker pages are done. The notes are in [research/round-5-parts-cross-check](../research/round-5-parts-cross-check/). Amazon would not show its listings to the checkers, so anything specific to one listing is marked "part in hand".

### Changes to the list

Proposed. The Amazon list is not changed until the owner agrees.

| Action | Part | Why |
| --- | --- | --- |
| Drop | ADuM1201 isolators | The comms pod is isolated at USB. |
| Drop for now | Isolated RS485 module | The inverter's monitoring port is RS232. This was also the slowest delivery on the list. |
| Swap | KingSpec NVMe for the Raspberry Pi SSD 256 GB | Known to boot a Pi 5. |
| Swap | The hub's 12 V adapter for one with a true 5.5 x 2.1 mm plug | The listed plug is 2.5 mm and fits loosely. |
| Swap | The rail's 12 V adapter for a 15 V one, 2.1 mm plug | Lets the rail be a real 12 V after the converter, and runs the reference's 10 V output. |
| Add | 74AHCT125 level shifter | Light strip data. |
| Add | 2N3904 transistors, or a ULN2003A | Button LEDs and the heartbeat stage. |
| Add | PC817 optocouplers | Sensing whether the rail is live. |
| Add | Small diodes, capacitors and resistors | The heartbeat stage. Values come from simulation. |
| Add | 100 ohm and 10 ohm 0.1% resistors | Checking the meter module's current path. |
| Add | Barrel socket to screw-terminal adapters | Bringing an adapter onto the rail wiring. |
| Add | Micro-USB cable | The debug probe. |
| Add | 12 V bulb and holder | The rail's test load. |
| Add | A second relay board | Two-pole switching of the reference, and a positive-side cut. |
| Add | The hub's upstream cable | Once its connector is known. |
| Maybe | Official Raspberry Pi 27 W supply | If the kit's supply does not offer 5 A. |

### Changes to the design

| Change | Why |
| --- | --- |
| The trust root is powered only from its own 5 V. The power wire in its USB lead is cut. | The board has no diode between USB power and its 5 V pin. The hub cannot power-cycle the trust root, which suits its job. |
| Module 2 is an RS232 tap first. | The inverter's monitoring port is RS232 on an RJ45 socket, and Solar Assistant already uses it. The pod listens in parallel through a pass-through. |
| The relay is driven by a heartbeat stage, not a pin. | A pin stuck high or low must drop the relay. The stage is simulated before it is built. |
| The current-limited converter is the soft limit and a fuse is the hard one. | The converter regulates slowly and does not trip. A fast electronic fuse is a later upgrade and is not sold on Amazon. |
| The reference is switched on both wires. | Switching one wire leaves the reference's ground joined to the meter module and bridges its isolation. |
| The debug probe and logic analyzer never touch a module that is wired to the inverter. | Their grounds are the brain's ground. Connecting them defeats the isolator. The isolation checker in the repo catches this case. |
| The controller's cut moves to the positive side before the rail feeds modules. | The MOSFET board switches the negative side, which leaves a shared-ground load connected. |

### The four questions, answered

1. **Debug access.** Yes. Three solder pads under the RP2350-Zero. The probe stays on the list.
2. **Isolation plan for the comms pod.** Isolate at USB. Drop the line isolators.
3. **Listen-only by wiring.** For RS232: connect receivers only. For RS485, later: a transceiver with its enable pin wired to ground.
4. **Which side the rail switch breaks.** The negative side. See the design change above.

### Still to settle with the part in hand

Switch contact ratings and key positions. LED voltages. The voltage reference's grade. The INA228 board's shunt. The hub's upstream connector and its port switching from the Pi. Whether the kit's supply gives 5 A. Whether each USB-C cable carries data. The inverter's exact pinout and baud rate, read on the real unit before anything is connected.
