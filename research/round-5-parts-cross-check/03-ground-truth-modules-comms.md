# Round 5 cross-check, slice 3: ground truth, DC meter, comms pod

Checked 26 November 2026 against datasheets, maker wikis, EG4's manual and Solar Assistant's help pages. Nothing was bought. Nothing was measured on a real part.

Amazon product pages would not load (all three tried were refused). So no ASIN on this slice was confirmed from its own listing. Generic modules are judged from the chip datasheet and the common reference design, and that is said each time.

Verdict words: **CONFIRMED** (a source read this session says so), **WRONG** (the list or the open check is contradicted), **UNVERIFIED** (could not be confirmed; what would settle it is given).

## Changes this forces

1. **Module 2's main job is RS232, not RS485.** The repo's own notes name the inverter as an EG4 3000EHV-48. Its monitoring port is RS232 on an RJ45 jack, and Solar Assistant already sits on it. RS485 on this inverter is the battery (BMS) port. Build and prove the RS232 tap first.
2. **Replace the 12 V feed for the AD584 with 15 V** (a 15 V adapter, or a module with its own battery and boost converter). 12 V is below the datasheet minimum for the 10 V output.
3. **Drop the ADuM1201 3-pack.** Isolate the pod at USB only. The ADuM1201 plan needs an isolated supply that is not on the list.
4. **Replace the auto-direction RS485 module with a breakout that has DE and /RE on pins, DE wired to ground.** Or defer RS485 altogether until the RS232 tap works; it is not needed for the first investigation. If the listed module is kept, its TXD pin must be wired to VCC, not left open.
5. **Add: a second RJ45 path for a pass-through tap.** The inverter's RS232 jack is occupied. The 2-pack of breakouts, wired back to back on pins 1, 2 and 8 only, does this for one inverter. Add one short patch lead.
6. **Add: two-pole switching for the reference.** The calibration relay must break both the reference output and its return, or the reference's supply ground defeats Module 1's isolation.
7. **Add to the distributor order:** 100 ohm 0.1% and 10 ohm 0.1% resistors for the current check (details in section 2). No divider is needed for the voltage check.
8. **Add a rule, not a part:** the debug probe and the logic analyzer are not isolated. Clipping either to a module joins that module's ground to the Pi. Never do it while the module is wired to the inverter.
9. **No change:** INA228 breakout, DSD TECH isolator (two), MAX3232 module, Waveshare adapter, RJ45 breakouts.

---

## 1. AD584 voltage reference module (B0B4WN2Q91)

**Open check:** what supply it needs; whether the printed calibration values can be believed.
**Verdict: the 12 V plan is WRONG for the 10 V output. Which module variant this ASIN is: UNVERIFIED.**

Facts (AD584 datasheet, <https://www.analog.com/media/en/technical-documentation/data-sheets/AD584.pdf>):

| Output | Minimum supply (output + 2.5 V) | Fed from a 12 V adapter |
| --- | --- | --- |
| 2.5 V | 5.0 V | Fine |
| 5 V | 7.5 V | Fine |
| 7.5 V | 10.0 V | Fine |
| 10 V | 12.5 V | **Out of spec** |

- Supply range is 4.5 to 30 V. The chip draws about 0.75 mA (1 mA at most).
- It can source 10 mA at room temperature, 5 mA over temperature. Short-circuit current is about 30 mA. It is a reference, not a power supply.
- Grades: J is within 30 mV at 10 V (0.3%) and 30 ppm per degree C. K is within 10 mV at 10 V (0.1%) and 15 ppm per degree C. Long-term drift is 25 ppm per 1000 hours. The L grade was removed from the current datasheet.

What the cheap modules really are (two independent teardowns: <https://markhennessy.co.uk/ad584_references/> and <https://www.siliconchip.com.au/Issue/2019/July/El+Cheapo+Modules%3A+AD584+Precision+Voltage+References>):

| Variant | Power | Chip seen | Calibration slip |
| --- | --- | --- | --- |
| Small bare board, about 32 mm | Direct DC input through a series protection diode | J grade | None |
| Bare board, about 50 mm, barrel jack | 15 to 24 V DC in | L grade | Printed. One reviewer found the same printed numbers on other people's units and calls it fictitious |
| Cased unit (sold as KKmoon and others) | Internal lithium cell, charged from 5 V USB, or 15 to 24 V DC. A boost converter makes about 13.5 V for the chip | K grade | Hand-written. Agreed with a lab meter to 0.02 to 0.04% in one teardown and was close in the other |

- Chips carry old date codes and are probably reclaimed. That is the reviewers' inference, not a proven fact.
- Measured samples were all inside 0.06%, better than the grade guarantees. That is three or four samples, not a promise.
- The cased unit's boost converter leaves about 30 mV peak-to-peak of switching spikes on the output (under 1 mV RMS).
- One bare board held its 10 V output within 0.2 mV when the input was dropped from 18 V to 12.5 V. Nobody tested 12.0 V.

What it means for the build:

- **A 12 V adapter cannot be relied on for the 10 V output.** With a protection diode in series it is worse. No variant wants 12 V: the bare ones want 15 V or more, the cased one wants 5 V USB for charging. Buy a 15 V adapter, or use the cased type on its battery.
- **The printed values cannot be believed without a check.** A hand-written slip is plausible. A printed one is not.
- **Be honest about what this reference proves.** The INA228's own voltage error is at most 0.05% gain plus 2.5 mV. That is tighter than an uncalibrated J or K chip, and tighter than most handheld meters. So the reference does not make the INA228 more accurate. It catches gross faults and drift: wrong scale, a dead input, a reading that has moved since last time. Record it that way in the store.
- **Temperature matters in an unheated space.** A J chip can move 0.09% over a 30 degree C swing. Log temperature with every check. The INA228 has an internal sensor good to 1 degree C.
- **Grounding.** A reference powered from a wall adapter shares that adapter's ground. If that adapter also feeds the hub, connecting the reference to Module 1 bridges the USB isolator. Switch both reference wires with a two-pole relay, or run the reference from its own battery or its own adapter.

UNVERIFIED: which variant B0B4WN2Q91 is, its input connector and stated input range, the chip grade. Settled by the listing photos or the part in hand. Measure all four outputs with the best meter available after 30 minutes of warm-up.

## 2. INA228 breakout (B0GX5HM4X3)

**Open check:** the fitted shunt and so the current range; that address and alert pins are brought out.
**Verdict: chip facts CONFIRMED from TI. The generic board's shunt and pins are UNVERIFIED.**

Chip facts (<https://www.ti.com/lit/ds/symlink/ina228.pdf>, <https://www.ti.com/product/INA228>):

- Bus voltage 0 to 85 V. Supply 2.7 to 5.5 V, about 0.64 mA.
- Shunt input full scale: 163.84 mV, or 40.96 mV in the fine range. 20-bit converter.
- Voltage path: gain error 0.05% at most, offset 2.5 mV at most. Shunt path: gain error 0.05% at most, offset 1 microvolt at most.
- I2C: 16 addresses, 0x40 to 0x4F, set by two pins (A0, A1). Logic high is anything from 1.2 V, so 3.3 V logic is fine.
- Alert pin is open-drain, active low by default. It needs a pull-up to 3.3 V.
- Works high-side or low-side.

The two documented boards (Adafruit and SparkFun, already confirmed in round 4; <https://learn.adafruit.com/adafruit-ina228-i2c-power-monitor.md?view=all>):

- Shunt is 15 milliohm, 0.1%. That gives 10.9 A full scale in the wide range and 2.73 A in the fine range. Adafruit says 10 A maximum and 2.75 A continuous in the fine range.
- Default address 0x40. A0 and A1 are solder jumpers. Alert is on a pin.
- Adafruit has 10 k pull-ups on SDA and SCL to its supply pin. So power the board from 3.3 V, not 5 V, and the pull-ups land at 3.3 V.
- A solder jumper ties VBUS to VIN+ for high-side use. It is open by default.

What it means for the build:

- Power the board from the RP2350's 3.3 V pin. Wire VBUS to the high side by hand or by the jumper.
- The ground of Module 1 becomes the ground of the circuit being measured. That is why it sits behind the USB isolator.
- Do not connect the 48 V bank or mains to it in the Bench Core. The 85 V limit is a chip limit, not a safety rating.

**Checking the voltage path against the AD584.** Connect each reference output to VBUS, reference return to module ground. No divider is needed: all four outputs are inside the 85 V range. Four points show offset, gain and linearity. Expected agreement, worst case at 10 V: 7.5 mV from the INA228 plus whatever the reference is off by.

**Checking the current path.** The AD584 cannot do this. It sources 5 to 10 mA. Across 15 milliohm that is under 0.15 mV, where the 1 microvolt offset is already about 1%. Use a real current through a known resistor instead:

1. Put a precision resistor in series with the shunt, fed from a 5 V or 12 V rail.
2. Read volts across the resistor on the VBUS input (already checked against the reference) and amps through the shunt.
3. Volts divided by amps should equal the resistor's marked value.

Resistors needed (distributor order; no part numbers were checked this session):

| Value | Tolerance | Power rating to buy | Use | Point it gives |
| --- | --- | --- | --- | --- |
| 100 ohm | 0.1%, 25 ppm or better | 1 W | From 5 V | 50 mA, 0.25 W in the resistor. 0.75 mV on the shunt |
| 10 ohm | 0.1% if affordable, else 1% and measured | 10 W or more, on a heat sink | From 5 V | 0.5 A, 2.5 W in the resistor |
| 10 ohm, same part | | 25 W or more | From 12 V | 1.2 A, 14.4 W. Short runs only |

The resistor warms and drifts, so take the reading in the first seconds. The cross-reference lists "0.1% resistors for the on-board reference divider". A divider is only needed if the on-board shunt is ever bypassed for an external one: 10 k over 100 ohm from the 10 V output gives 99 mV, inside the wide shunt range.

UNVERIFIED: the shunt value on the generic board (read the resistor: R015 is 15 milliohm, R002 is 2 milliohm, R100 is 100 milliohm), whether A0, A1 and alert reach header pins, whether it has I2C pull-ups and to which rail, whether the chip is a real TI part. Settled by the part in hand. If the shunt is not 15 milliohm, the current range changes in proportion.

## 3. DSD TECH SH-G01A USB isolator, ADuM3160 (B093LKN3YH), two

**Open check:** how much current the isolated side supplies; full-speed only.
**Verdict: speed limit CONFIRMED. 200 mA budget CONFIRMED from a reseller copy of the listing. Internal converter part and speed switch UNVERIFIED.**

Chip facts (<https://www.analog.com/media/en/technical-documentation/data-sheets/ADuM3160.pdf>):

- Low speed (1.5 Mbps) or full speed (12 Mbps) only. No high speed.
- The speed is fixed by two pins on the chip, one each side, strapped the same way. It does not detect speed by itself.
- 2500 V RMS for one minute across the barrier.
- **The chip carries no power across.** The far side needs its own 5 V.
- The upstream pull-up is controlled by a pin on the downstream side, not by the peripheral's own pull-up. When the downstream side loses power, the upstream side disconnects within 32 bit times.

Module facts (listing text as copied by a reseller, <https://www.desertcart.co.za/products/323644880-sh-g01a-usb-isolator-with-adum3160-chip-12m.md>; reference design at <https://github.com/wagiminator/ADuM3160-USB-Isolator>):

- The module makes its own isolated 5 V. It needs no external power. The listing says 200 mA in the spec table, 400 mA in the feature list, and 1 W maximum in the Q&A. **Plan on 200 mA.**
- Signal isolation 2500 V, power isolation 1500 V.
- The open reference design uses a 1 W, 5 V to 5 V isolated converter with an unregulated output. It needs at least 20 mA of load. Below that the output can climb above 5.5 V. The reference design adds a 5.1 V Zener for this.
- One buyer reports it gives only a little power. One reports a unit that stopped passing data after two months.

Answers:

- **Does cutting the hub port's power also cut the module?** Yes. The module has no other power input, so its isolated 5 V comes from the hub port's 5 V. Remove one and the other goes. CONFIRMED by the listing's "no external power" plus the chip carrying no power. Still worth one check with a meter on the real part.
- **Is 200 mA enough?** Known loads are small: isolator far side 8 mA at most, INA228 about 1 mA, a MAX485 board about 5 mA. The MAX3232's supply current was not read this session. The RP2350-Zero's own draw could not be read (the Raspberry Pi datasheet site was blocked). UNVERIFIED, but the margin is large. The one load that could matter is the calibration relay: a common 5 V cube relay coil takes far more than a small signal or reed relay. Choose a small one.
- **Speed match.** Waveshare lists the RP2350-Zero as USB 1.1 (<https://www.waveshare.com/wiki/RP2350-Zero>), which is full speed. The isolator must be set to full speed.

What to test on arrival:

1. Whether it has a speed switch or jumper, and that it is set to full speed. UNVERIFIED from the listing.
2. The isolated 5 V with the RP2350 board attached: it should sit between 4.75 and 5.25 V. Also with nothing attached, to see how high it floats.
3. **Firmware update through the isolator.** Because the upstream pull-up does not follow the board, the Pi may not notice when the RP2350 drops off and comes back as a bootloader. The datasheet does not say how re-enumeration behaves. Test it. If it fails, the fix is a USB port reset from the Pi, or a port power cycle from the hub.
4. The logic analyzer on the list is a high-speed USB device and will not work through this kind of isolator. So it cannot be isolated this way. See change 8.

## 4. Comms pod isolation plan

**Open check:** two plans are bought; keep one.
**Verdict: plan (a), isolate at USB. Drop the ADuM1201 pack (B0FTFGMKC3).**

ADuM1201 facts (<https://www.analog.com/media/en/technical-documentation/data-sheets/ADuM1200_1201.pdf>):

- Two channels, one in each direction. 2500 V RMS.
- Each side needs its own 2.7 to 5.5 V supply and its own ground. It makes no isolated power.
- It can translate between 3.3 V and 5 V sides.

| | (a) Isolate at USB | (b) Isolate the serial lines |
| --- | --- | --- |
| Barriers | One, bought and boxed | One per transceiver, hand-wired |
| Far-side power | Comes with the isolator | Needs an isolated DC-DC plus a regulator. Not on the list |
| What sits on the inverter's ground | The whole pod: RP2350 and both transceivers | Only the transceivers |
| Parts no longer needed | ADuM1201 pack | USB isolator for module 2 |
| Listen-only | Unaffected either way | Unaffected either way |

Plan (a) is simpler and has fewer hand-made joints across the barrier. Take it.

Two cautions that come with plan (a):

- **One pod, one ground.** If the RS232 jack and the BMS RS485 link are both wired to the pod, the pod joins their two grounds. Whether the inverter already joins them is not known. Connect one bus at a time in the Bench Core, or measure between the two grounds (DC and AC volts) before joining them. This is the one case where per-line isolation would earn its place later.
- **Any second cable defeats it.** The debug probe and the logic analyzer are on the hub without isolation. Use them on the pod only when the pod is unplugged from the inverter.

## 5. RS485 listen-only, and the listed "type C" module (B0DBCF4ZNZ)

**Open check:** the module has no transmit-enable pin, which conflicts with "listen-only is a wire".
**Verdict: conflict CONFIRMED. Replace or defer. Identity of the ASIN UNVERIFIED.**

The name matches Waveshare's "TTL TO RS485 (C)" (<https://waveshare.com/product/iot-communication/ttl-to-rs485-c.htm>). If that is the part:

- 3.3 to 5 V supply. It has its own power and signal isolation (no rating given).
- Pins are VCC, GND, TXD, RXD on one side and A, B, PE (signal ground) on the other. There is no DE, /RE or enable pin.
- The 120 ohm terminator is on the board but only joins the bus if a pad is soldered. Off by default.

Three ways to get a receive-only tap, best first:

1. **A transceiver breakout with DE and /RE on pins. Wire DE to ground and /RE to ground.** With DE low the driver is off and the bus pins are high impedance. This is stated in the chip datasheet (<https://www.ti.com/lit/ds/symlink/sn65hvd3082e.pdf>). A continuity meter proves it. To allow transmit later, DE is fed from the armed rail and nothing else.
2. **Keep the listed module, wire its TXD pin to VCC at the header, and run no wire from TXD to the RP2350.** Auto-direction boards turn the driver on when TXD goes low. A TXD held high can never do that. Do not leave TXD open: nothing read this session says the pin has a pull-up. UNVERIFIED until checked on the part (watch A and B with the bus disconnected).
3. A receiver-only chip. Cleanest in principle, but no ready-made module was found.

A specific Amazon US part with DE and /RE on pins: the classic blue MAX485 board, sold in 5-packs, for example **NewZoll "5pcs RS-485 Module, MAX485 Chip, 5V, 44x14mm"** (<https://www.amazon.com/MAX485-Module-RS-485-Instrument-Interface/dp/B01N8WLEV0>). The listing appeared in a search but its page could not be opened. The board itself is documented at <https://protosupplies.com/product/max485-ttl-to-rs-485-interface-module/>. It needs four things done:

- **It is a 5 V part** (4.75 to 5.25 V). Its RO output swings to 5 V. Put a divider on RO (10 k in series, 20 k to ground) before the RP2350 pin.
- **Remove R7**, the 120 ohm terminator. It is fitted by default.
- **Solder DE to ground at the board.** The board has 10 k pull-ups on its logic pins. If DE is one of them and is left open, the driver is on. Which pins have the pull-ups was not stated: check with a meter.
- Leave DI unconnected to the RP2350.

A 3.3 V alternative that needs no divider: SparkFun's RS-485 Transceiver Breakout, SP3485, $12.50, in stock at SparkFun (<https://www.sparkfun.com/sparkfun-transceiver-breakout-rs-485.html>). Its driver and receiver enables come out on one RTS pin. Which level means receive, and whether it has a terminator, were not on the page: read its schematic before buying. Amazon availability UNVERIFIED.

Does a passive receiver disturb the bus?

- **Load:** a standard receiver is one unit load, about 12 k. A bus is built for 32 of them. Low-load chips are one eighth of that. One more receiver on a two-node link is negligible.
- **Termination: do NOT add a second or third terminator.** The two ends already have theirs. Another 120 ohm cuts the signal.
- **Bias:** add none. The existing nodes hold the idle state. The blue board's two 20 k resistors are weak and harmless.
- **Ground:** the pod is isolated, so it must be given the bus's signal ground wire. Otherwise the receiver can float outside its minus 7 to plus 12 V range.
- Keep the tap wire short.

## 6. MAX3232 module (B0DLN3DQMN), RS232 listen-only

**Open check:** that its transmit output can be left unconnected.
**Verdict: CONFIRMED. Which pins the module brings out is UNVERIFIED.**

Facts (<https://www.ti.com/lit/ds/symlink/max3232.pdf>):

- Supply 3 to 5.5 V. Run it at 3.3 V and its logic pins match the RP2350.
- Two drivers and two receivers.
- Receiver inputs accept plus or minus 25 V. Input resistance is 3 to 7 k (5 k typical). Thresholds are 0.6 to 2.4 V, so it also reads a 0 to 5 V "pretend RS232" line. An open receiver input reads as idle.
- Driver outputs are short-circuit protected. Driver inputs have no pull-up and must sit at a fixed level.

What it means for the build:

- **Use the receivers only.** Wire each driver input to 3.3 V. Leave the driver outputs connected to nothing. A driver output is then at about minus 5 V into thin air, which is harmless. Do not ground a driver output.
- **Listen-only is a wire here by default:** the driver output pad simply has no wire.
- **Two directions need two receivers.** RS232 has one wire per direction. Inverter pin 1 carries the inverter's replies. Pin 2 carries Solar Assistant's requests. To see both, use both receivers and two UART inputs on the RP2350. The RP2350 has two hardware UARTs, so a later RS485 input would need a third, software-made one.
- **Tapping in parallel is acceptable, with one measurement.** Each RS232 driver then sees two receivers, about 2.5 k instead of 5 k. That is a little heavier than the 3 k the standard assumes. At 9600 baud on a short lead it should work. Prove it: measure the idle voltage on pin 1 and pin 2 before and after the tap is attached. It must stay well beyond 3 V in magnitude.

UNVERIFIED: whether the listed module brings out the second receiver. If not, use two modules. Settled by the part in hand.

## 7. The EG4 port

**Verdict: port type and occupant CONFIRMED from the repo's notes and vendor pages. Pinout and baud rate UNVERIFIED on the real unit.**

What the repo already holds (`research/round-1-portable-test-bench/solar_bms_comms.md`, `docs/project-document.md`):

- The inverter is an EG4 3000EHV-48. The owner confirmed on 25 November that Solar Assistant is set to "Sumry" for it.
- Solar Assistant already holds the RS232 port of each inverter.

What the sources say:

| Port on the 3000EHV-48 | Layer | Use | Source |
| --- | --- | --- | --- |
| RS-232, RJ45 jack | RS232 | WiFi dongle, or Solar Assistant | EG4 V2 manual <https://eg4electronics.com/wp-content/uploads/2024/04/EG4-3000EHV-48-V2-Manual.pdf>; Solar Assistant <https://origin.solar-assistant.io/help/inverters/eg4/3000-EHV-48/rs232> |
| USB / RS-485 | RS485 | Battery BMS link, PC tool | EG4 V2 manual |
| Parallel, RJ45 | CAN | Inverter-to-inverter | EG4 V2 manual |

- **Protocol:** Modbus RTU. Modbus ID is program 25, default 001 (EG4 manual). The register family matches the SMG-II siblings (repo notes).
- **Baud:** EG4 prints none. The sibling project uses 9600 (<https://github.com/syssi/esphome-smg-ii>). UNVERIFIED for this unit.
- **RS232 RJ45 pinout:** EG4 prints none. The sibling pinout is pin 1 inverter transmit, pin 2 inverter receive, **pin 4 plus 12 V**, pin 8 ground, others unused. UNVERIFIED for this unit.
- **BMS RS485 pinout:** EG4 prints the battery end only: pin 1 B, pin 2 A, pins 3 and 4 ground, wired pin to pin. The inverter end is not printed.
- Solar Assistant needs one cable per inverter. It does not give baud or pinout.

**PIN CARRYING POWER: pin 4 of the RS232 jack, about 12 V on the sibling design.** It powers the WiFi dongle. Leave it open on every breakout. Never plug a plain patch lead from this jack into a Pi, a HAT, a switch or any other RJ45 socket.

Is a second listener workable?

- **On RS232 (the port Solar Assistant uses): yes, as a parallel tap.** The pod's two receivers hang on pins 1 and 2 with pin 8 as reference. It sees Solar Assistant's requests on one wire and the inverter's replies on the other. It sends nothing. It must never become a second sender: two RS232 drivers on one wire fight each other.
- **On an RS485 pair: yes.** A receiver in parallel sees both sides of the conversation on the same two wires, requests and replies in turn. No terminator, no bias, ground wire connected.
- **Risk differs by link.** Unplugging Solar Assistant for a minute loses monitoring only. Disturbing the inverter-to-battery link is different: the repo records a case where a lost battery link made an inverter force a grid charge. Do the RS232 tap first. Whether the BMS link is even in use at this site is not recorded.

How to build the tap: the inverter's jack is occupied, so the tap is a pass-through. Two breakouts back to back. Carry pins 1, 2 and 8 across. Leave pin 4 open on both. The pod's wires join at the screw terminals.

Settle on the real unit, in this order, with nothing but a meter and one breakout:

1. Check the breakout's terminal numbers against the jack with a continuity test. Some are mirrored.
2. Meter each terminal against pin 8. Expect about 12 V on pin 4 and a steady negative voltage on pin 1.
3. Then attach the Waveshare adapter's receive pin to pin 1 and try 9600 8N1.

## 8. Waveshare isolated USB to RS232/485/TTL (B07L2VLY5D)

**Open check:** none beyond the inverter's pinout.
**Verdict: mostly CONFIRMED, with two corrections.**

Facts (<https://www.waveshare.com/usb-to-rs232-485-ttl.htm>, <https://www.waveshare.com/wiki/USB_TO_RS232/485/TTL>):

- **Chip:** the wiki title says FT232RL. The current product page says FT232RNL or CH343G depending on version. Which one this ASIN ships is UNVERIFIED. Check the listing title for "FT232".
- **Isolation:** power and signal isolation on board. No voltage rating was given on either page.
- **Connectors:** USB-B, a DB9 male for RS232, screw terminals for RS485 (A, B, ground) and TTL.
- **One interface at a time.** It cannot read RS232 and RS485 together.
- **RS485 direction is automatic.** There is no enable pin.
- **Linux:** listed as supported. The FTDI chip uses the kernel's standard driver. The CH343 version's Linux path was not checked.
- **Terminator:** the product page says pads only, nothing fitted. The wiki's FAQ talks about removing a 120 ohm resistor. The two disagree. Look at the board and remove any resistor across A and B before touching a live bus.

Receive-only use:

- **RS232: yes, by wiring.** Connect DB9 pin 2 (receive) and pin 5 (ground) only. Leave pin 3 (transmit) with no wire. It then hears one direction. Move the wire to hear the other.
- **RS485: by software only.** It stays silent as long as nothing writes to the port. That is discipline, not a wire. Acceptable for the bought reference, and say so in the record.
- It needs a USB-B cable. Check one is in the box.

## 9. RJ45 screw-terminal breakouts (B0GXHYR7QQ)

**Open check:** pinout to be confirmed on the real unit; one pin carries 12 V on the sibling.
**Verdict: open check stands, UNVERIFIED until measured. See section 7 for the procedure. No change to the part. The 2-pack makes one pass-through tap.**

## 10. Level and supply table

Module 1, DC meter:

| Connection | Side A | Side B | Agree? |
| --- | --- | --- | --- |
| Hub port to isolator | 5 V | 5 V in | Yes |
| Isolator to RP2350-Zero power | Isolated 5 V, unregulated, 200 mA | 5 V USB-C in | Yes. Measure it loaded and unloaded |
| Isolator to RP2350-Zero data | Full speed, if so set | Full speed | Yes, once the speed setting is confirmed |
| RP2350 3.3 V pin to INA228 supply | 3.3 V | 2.7 to 5.5 V | Yes |
| I2C SDA and SCL | 3.3 V | High from 1.2 V, tolerant to 5.5 V | Yes, if the board's pull-ups go to 3.3 V |
| INA228 alert to a GPIO | Open-drain | 3.3 V input with pull-up | Yes |
| AD584 output to INA228 VBUS | 2.5, 5, 7.5, 10 V | 0 to 85 V | Yes |
| AD584 load | Sources 5 to 10 mA | VBUS input draws microamps | Yes |
| 12 V adapter to AD584 supply | 12 V | Needs 12.5 V or more for the 10 V output | **No.** Use 15 V |
| AD584 return to module ground | Adapter ground | Isolated module ground | **No**, unless switched on both poles or battery powered |
| Calibration relay coil to isolated 5 V | Coil current | 200 mA budget | Depends on the relay. Pick a small one |

Module 2, comms pod:

| Connection | Side A | Side B | Agree? |
| --- | --- | --- | --- |
| Hub, isolator, RP2350 | As module 1 | As module 1 | Yes |
| RP2350 3.3 V to MAX3232 supply | 3.3 V | 3 to 5.5 V | Yes |
| MAX3232 receiver outputs to UART inputs | 3.3 V logic | 3.3 V | Yes |
| Inverter pin 1 to receiver 1 | RS232 level, not yet measured | Accepts plus or minus 25 V | Yes |
| Inverter pin 2 to receiver 2 | RS232 level from Solar Assistant's adapter | Accepts plus or minus 25 V | Yes |
| Inverter pin 8 to pod ground | Inverter comms ground | Isolated pod ground | Yes. This is why the pod is isolated |
| Inverter pin 4 | About 12 V | Nothing | **Leave open** |
| MAX3232 driver outputs | Minus 5 V idle | Nothing | Leave open |
| MAX3232 driver inputs | Need a fixed level | 3.3 V | Tie to 3.3 V |
| Blue MAX485 board supply | Needs 4.75 to 5.25 V | Isolated 5 V, unregulated | Marginal. Measure the rail |
| Blue MAX485 board RO to RP2350 | 5 V logic | 3.3 V pin | **No.** Add the divider, or use a 3.3 V board |
| Waveshare "type C" module, if kept | 3.3 to 5 V supply and logic | 3.3 V | Yes |
| RS485 A and B to receiver | Minus 7 to plus 12 V allowed | Bus levels | Yes, with the ground wire connected |
| Logic analyzer or debug probe to the pod | Pi ground | Inverter ground | **No.** Never while wired to the inverter |

## Everything still UNVERIFIED

| Item | What would settle it |
| --- | --- |
| Every ASIN's actual contents | The listing photos, then the part in hand |
| AD584 module variant, input range, chip grade | Part in hand; meter on all four outputs |
| INA228 board: shunt value, pins, pull-ups, genuine chip | Read the shunt marking; trace the pins |
| Isolator: speed switch, converter part, voltage loaded and unloaded | Part in hand; meter |
| Isolator: re-enumeration into the bootloader | Flash a board through it |
| RP2350-Zero current draw | Measure it (source page was blocked) |
| "Type C" module: maker, internal direction circuit, TXD pull-up | Part in hand |
| Blue MAX485 board: which pins have pull-ups | Meter |
| SparkFun breakout: enable polarity, terminator, Amazon stock | Its schematic; the store pages |
| MAX3232 module: second receiver on a pin | Part in hand |
| Waveshare adapter: chip version, isolation rating, terminator fitted | Listing title; look at the board |
| Inverter RS232 pinout, 12 V on pin 4, signal levels, baud rate | Meter and a listen-only capture on the real inverter |
| Inverter-end RS485 pinout; whether the BMS link is in use | Check on the real inverter |
| Whether RS232 ground and BMS RS485 ground are the same | Meter between them before joining |
| Parallel RS232 tap loading | Idle voltage on pins 1 and 2 before and after |
