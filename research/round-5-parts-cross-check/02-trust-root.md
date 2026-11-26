# Round 5 parts cross-check: 02, the trust root (Rung 3)

Checked 26 November 2026. Scope: the Rung 3 rows of `docs/bench-core-cross-reference.md`. Nothing has been bought, and nothing was added to any cart or list.

How to read this file:

- **CONFIRMED** means the fact was read from a source during this check. The source link is given.
- **WRONG** means the parts list or the cross-reference assumed something a source contradicts.
- **UNVERIFIED** means no source settled it. What would settle it is stated, usually "measure the part in hand".
- "(from memory)" marks a fact from general knowledge that was not re-read from a source in this check. Treat it as UNVERIFIED until checked.
- Amazon product pages refused to load (robots block) and searches by ASIN returned nothing useful. So **no fact below comes from the actual Amazon listings**. For generic modules the common reference design for that module type was used, and each section says so.

## Changes this forces

1. **Rail switch (AOD4184 module): it is a low-side switch.** Fine for the lamp test. Not acceptable for any load that shares ground with the bench, because the load stays powered through the other ground path. Before such a load exists, replace it with a high-side switch (a P-channel MOSFET stage on the protoboard, or relay channel 2 in the positive line). Section 2.
2. **Add a heartbeat stage between the controller and the relay board.** A diode charge pump plus one NPN transistor. The relay input is driven by this stage, never directly by a GPIO. Parts to add: two small Schottky diodes, 100 nF and 1 µF capacitors, one 2N3904, a few resistors. Sections 3 and 4.
3. **Dual power: the RP2350-Zero has no diode on VBUS.** Do not feed its 5 V pin while a normal USB lead is plugged in. Power the trust root only from its own 5 V supply and use a USB lead with the VBUS wire cut. Drop the idea that the hub can power-cycle the trust root. Section 5.
4. **Add a 74AHCT125 (DIP) for the WS2812 data line.** Section 6.
5. **Add NPN transistors (2N3904) or one ULN2003A for the 12 V button LEDs.** Section 7.
6. **Add PC817 optocouplers for the "rail live" feedback.** Sensing across the load terminals works whichever side is switched. Section 2.
7. **XL4015: keep it, but stop calling it a hard cap.** It is a slow, regulating limit with one-fault failure modes. It cannot give 12 V out from 12 V in. If a true hard limit is wanted, add an eFuse board (TI TPS2596 class) from a distributor. None was found on Amazon. Section 1.
8. **Fuses: 2 A on the rail branch, 1 A on the trust-root 5 V branch.** A 3 A fuse on a 3 A adapter never blows. Take the trust-root 5 V branch off before the emergency stop. Section 11.
9. **No change:** relay module (use COM and NO), OLED (run it from 3.3 V), RP2350-Zero pin count (about 14 of 20 edge pins), emergency stop, key switch, toggle, indicator lights. All of these need a check with a meter on arrival.

## 1. XL4015 CC/CV buck module (B0H2HD391J)

**Verdict: CONFIRMED as a regulating current limit. WRONG as a "hard" hardware power cap by itself. Accuracy and drift are UNVERIFIED.**

Reference design used: the common red or blue XL4015 board with two or three trimmers and an LM358. The specific Amazon board was not seen.

Facts from the chip datasheet (https://handsontec.com/dataspecs/IC/XL4015.pdf):

- Input 8 to 36 V. Output 1.25 to 32 V. Feedback reference 1.25 V, about 2 percent.
- Fixed 180 kHz. Duty cycle can reach 100 percent. Minimum dropout 0.3 V.
- Quiescent current 2.1 to 5 mA.
- The chip's own switch current limit is 7 A. On a shorted output the frequency folds back to 48 kHz.
- The datasheet shows no constant-current circuit. The adjustable limit is an add-on by the module maker.

Facts about the module:

- The add-on limit works like this: a 78L05 and a TL431 make a reference, a trimmer sets a fraction of it, and one half of an LM358 compares it with the voltage across a sense resistor. When the current is too high the op-amp pushes the chip's feedback pin up, and the output voltage falls. Stated range 0.2 to 5 A. Source: https://circuitdigest.com/electronic-circuits/diy-xl4015-dc-dc-buck-converter-module
- So the limit **regulates**. It does not trip and does not latch. The voltage sags until the current sits at the set value, for as long as the fault lasts.
- Seller figures: ripple 50 mV maximum, load regulation 0.5 percent, no reverse-polarity protection, over-temperature shutdown, short-circuit limit quoted as 8 A. Sources: https://www.handsontec.com/dataspecs/module/XL4015-5A-PS.pdf and https://www.phippselectronics.com/support/xl4015-5a-adjustable-cc-cv-step-down-dc-power-supply-module/
- Setting method: set the voltage first, then short the output through an ammeter and turn the current trimmer to the wanted value (Phipps page above).

UNVERIFIED, measure the part in hand:

- **Minimum limit and accuracy.** 0.2 A is one article's figure. No source gave accuracy or drift numbers. The sense resistor value is not confirmed (commonly 0.05 ohm, from memory). With a small sense voltage and an LM358, an error of several percent and some drift with temperature are to be expected (from memory). Measure the limit cold and warm.
- **Start-up into a load or a short.** No source measured it. The op-amp loop is slow and the output capacitor is large. In the first instant of a short the capacitor dumps into the fault, and the chip's own 7 to 8 A limit is the only bound until the loop catches up. Capture this with a scope or the INA228.
- **Ripple in current-limit mode**, and whether the limit oscillates with a lamp load.

What it means for the build:

- 12 V and 1.25 A is 15 W. The trimmer sits in the low quarter of its range, so setting is fiddly but possible.
- **12 V out from a 12 V adapter is not possible in regulation.** The chip needs at least 0.3 V, plus the sense resistor and inductor. Either call the rail "about 11 V" and set it there, or feed the limiter from a higher-voltage adapter.
- It is a sound **soft limit**: it holds average current at the set value. It is not a hard cap, for three reasons. It is slow. It lets the output capacitor discharge freely. And one failed part (op-amp, trimmer wiper, sense path) removes the limit, leaving only the 3 A adapter as the bound, which is 36 W.
- For the controller's 5 V supply the module is electrically fine (12 V in, 5 V out). The worry is the voltage trimmer. If a trimmer fails open the output may rise toward 12 V (from memory, depends on which leg the trimmer is in; UNVERIFIED for this board). A fixed 5 V converter is the safer choice for the trust root's own supply.

A better part for a hard limit:

- **eFuse evaluation board, TI TPS2596EVM.** 2.7 to 19 V, limit set by jumpers from 0.125 to 2 A. Source: https://www.ti.com/tool/TPS2596EVM. Sold by TI and distributors. **No eFuse breakout was found on Amazon US** (one search, TI and MIKROE boards only on distributor sites). Its accuracy and its trip-or-limit behaviour were not on the page: UNVERIFIED, read the TPS2596 datasheet for the exact variant.
- Two-line comparison. XL4015: adjustable and efficient, but slow, drifts, and fails to "no limit". eFuse: fast electronic cut at a set threshold, built for this job, but a distributor order and a second board. A resettable polyfuse is the cheap Amazon-available backstop: passive, but slow (seconds) and its trip point moves with temperature (from memory); a specific listing was not verified.
- Sensible layering: XL4015 as the measured soft limit, then an eFuse as the hard cut if the 15 W claim must survive one fault.

## 2. AOD4184 / "D4184" MOSFET module with PC817 (B0G5NNG1VZ)

**Verdict: CONFIRMED low-side switch. The open question in the cross-reference is answered, and the answer forces a design change for any grounded load.**

Reference design used: the common D4184 dual-MOSFET board. Source for all facts below unless noted: https://protosupplies.com/product/d4184-mosfet-control-module/

- **It switches the negative side.** The load's positive lead goes to the "+" terminal. The load's negative lead goes to the switched terminal, and the MOSFETs connect it to ground when on.
- **Terminals:** supply block with "+" and "−" for the load supply. Load block with "+" and the switched "LOAD" terminal. A two-pin logic block: PWM and GND. That the two "+" terminals are joined on the board is the standard layout but the page does not say so: UNVERIFIED, check with a continuity test.
- **Drive from 3.3 V:** the input is active-high through a 1 k resistor into the optocoupler. The stated input range is 2.5 to 20 V, and 3.3 V logic is listed as compatible. With nothing connected the gate is pulled low, so the switch is off. Confirm turn-on from a real 3.3 V pin on the bench.
- **The gate is driven from the load supply, at about half of it.** A 4.7 k and 4.7 k divider. At 12 V the gate gets about 6 V. The stated working range is 6 to 36 V, with 9 V or more advised for more than a few amps.
- **The logic ground and the load ground are not joined on the board.**
- **Maximum switching frequency:** not given by the reference page. Seller titles say 0 to 20 kHz (search result title only, page not read): UNVERIFIED. The slow optocoupler and the 4.7 k gate pull-down make it a poor fast PWM switch (from memory). This build only switches on and off, so it does not matter.

What it means:

- **Common ground defeats a low-side switch.** With the switch off, the load's positive lead is still at rail voltage. If the load has any other path to ground (a USB shield, a serial ground, a scope lead, a module that shares the spine ground), current returns that way and the load stays powered. The comms pod's transmit driver is exactly such a load.
- **A lamp is fine.** It is a two-wire load with no other ground. So the Rung 3 lamp test can use this module as bought.
- **Gate drive sags with the rail.** The rail is the limiter's output. When the limiter folds the voltage back during a fault, the gate voltage halves with it. Below about 6 V on the rail the MOSFET is only partly on and heats up. Bounded by the 1.25 A limit, but worth a test.
- **Sensing "is the rail live".** Measuring rail-plus to ground says nothing, because rail-plus is live whether the switch is on or off. Measuring the switched negative is ambiguous when no load is fitted or when the relay is also open. The clean way: a PC817 with a series resistor (about 4.7 k at 12 V, starting value) wired **across the load terminals**. Its output pulls a GPIO low only when the load really has voltage across it. This works for low-side or high-side switching and keeps 12 V away from the controller.
- **Before any grounded load:** switch the positive side instead. Two options. (a) A P-channel MOSFET with a small N-channel or NPN driver on the protoboard. Part choice not verified here; simulate it. (b) Use relay channel 2 as the controller's cut, in the positive line. No new parts, but both cuts are then the same technology on the same board.

## 3. Two-channel 5 V relay module with optocouplers and JD-VCC jumper (B00E0NTPP4)

**Verdict: CONFIRMED active-low, drops out when floating or unpowered. The 3.3 V drive question is side-stepped by the heartbeat stage. Which relay part SunFounder fits is UNVERIFIED.**

Reference design used: the common 2-channel optocoupled board. Source: https://protosupplies.com/product/relay-module-5v-x-2-relay-w-optoisolation/

- **Active-low.** Pulling IN to ground energises the relay. A high level or no connection releases it.
- Each input sinks under 5 mA. The board draws about 70 mA per energised relay, 140 mA with both.
- The reference page lists it as 3.3 V and 5 V logic compatible.
- **JD-VCC jumper.** Fitted, one 5 V supply feeds both the optocoupler side and the coils. Removed, the coils take 5 V on JD-VCC and GND, and the VCC pin powers only the input side.
- **Contacts at DC:** the page gives 10 A at 15 V DC, tested to 6 A, and warns that the 30 V DC figure printed on the relay is optimistic. 1.25 A at 12 V is well inside this.
- **State with the controller off or the pin floating:** coil released. Use the COM and NO terminals, so the rail is open in that state.

What it means:

- **Active-low is the wrong sense for a fail-safe input driven by a GPIO.** A pin stuck low holds the relay closed. So no GPIO drives IN directly. The heartbeat stage in section 4 ends in an open-collector transistor that pulls IN low only while the heartbeat is alive.
- With that stage, the board runs wholly from the trust root's 5 V. VCC and JD-VCC both at 5 V, jumper fitted or removed, it makes no difference. The 3.3 V question disappears, because the transistor pulls IN fully to ground.
- If a 3.3 V GPIO ever does drive IN directly, the textbook wiring is: jumper removed, JD-VCC to 5 V, VCC pin to the controller's 3.3 V, IN to the GPIO, grounds joined. **UNVERIFIED** whether this board pulls in reliably that way: the input LED and the optocoupler LED in series leave very little voltage at 3.3 V (from memory). Test before trusting it.
- What an unpowered RP2350 pin does when the relay board is still powered was not confirmed in this check (the Raspberry Pi datasheet site refused to load). The heartbeat stage removes the question.
- A welded contact cannot be seen without feedback. The rail-live sense in section 2 must sit after both cuts, and firmware should test each cut alone at arm time.
- Budget 140 mA plus LEDs on the 5 V supply.

## 4. Heartbeat watchdog

**Verdict: CONFIRMED that the diode charge pump (AC-coupled driver) is the smallest well-known circuit. Component values below are starting values, not verified. Simulate them.**

The three candidates:

1. **Diode charge pump.** The pin's square wave passes through a series capacitor. Two diodes pump it into a second capacitor. A resistor bleeds that capacitor. The stored voltage turns on a transistor. A pin stuck high or stuck low passes no AC, the capacitor bleeds away, the transistor turns off, the relay drops. Described by experienced posters as the classic answer: two capacitors, a dual Schottky diode, a bleed resistor, sized so a few stray edges cannot turn it on. Noted weak points: the series capacitor must not fail short, and repeated resets must not look like a heartbeat, so firmware waits before starting it. Source: https://www.eevblog.com/forum/microcontrollers/securing-a-critical-logic-level-against-failure/
   The same idea is the standard "charge pump" safety signal on hobby CNC machines: the controller software emits a square wave and the breakout board enables its outputs only while it is present. Sources: https://linuxcnc.org/docs/2.2/html/man/man9/charge_pump.9.html and the CNC4PC C11G manual, "Safety Charge Pump" page, https://www.manualslib.com/manual/1897728/Cnc4pc-C11g.html?page=7 (title seen, page not read).
2. **Retriggerable monostable.** A 74HC123 or 74HC4538 with one resistor and one capacitor for the timeout, plus an output transistor. Each edge restarts the timer. No edges, timer expires, relay drops. A published stand-alone watchdog uses the MC14538 (or 74HC4538) this way with a 2N3904 and a relay. Source: https://www.nutsvolts.com/magazine/article/stand_alone_watchdog_timers
3. **Dedicated watchdog chip** (TPS3430, MAX6369 family, STWD100; all from memory, not checked). Surface-mount only. Most give a short reset pulse on timeout, not a steady "healthy" level, so a latch is still needed.

Pick: **the charge pump.** It is six ordinary through-hole parts on protoboard. It simulates in ngspice with only stock diode, capacitor and transistor models. The monostable needs a digital model of the chip.

Starting circuit, to be simulated before it is built (my values, UNVERIFIED):

- GPIO, 220 ohm, 100 nF series capacitor.
- Two small Schottky diodes (BAT43 or 1N5817 class): one from ground to the capacitor's far side, one from there to the storage capacitor.
- 1 µF storage capacitor with 100 k across it.
- 10 k from the storage capacitor to the base of a 2N3904. Emitter to ground. Collector to the relay board's IN.
- A bipolar transistor is chosen on purpose. A 3.3 V swing through two diodes leaves under 3 V, which does not guarantee turn-on of a 2N7000 (threshold up to 3 V, from memory). A 2N3904 needs only about 0.7 V.
- Heartbeat of a few kHz. Hold-up after the last edge is tens of milliseconds with these values. Set it per hazard class, as the review response requires.

Two rules that matter more than the values:

- **Toggle the pin from the checked main loop, in software.** A PWM peripheral keeps running when the core hangs, and would hold the relay closed.
- Use a film or C0G series capacitor, or two in series, since a shorted series capacitor is the one fault that defeats the circuit.

## 5. Dual power on the RP2350-Zero

**Verdict: WRONG as designed. The board has no back-feed protection.**

- The Waveshare schematic shows the USB-C VBUS net going straight to the header pin and to the 3.3 V regulator (ME6217C33). No diode, no fuse, no polyfuse between them. Source: https://files.waveshare.com/wiki/RP2350-Zero/RP2350_Zero.pdf (read by a text extraction of the PDF; worth one look by eye).
- The related RP2040-Zero documentation says the board can be powered through USB or by 5 V on pin 1. It describes these as alternatives. Source: https://nuttx.apache.org/docs/13.0.0/platforms/arm/rp2040/boards/waveshare-rp2040-zero/index.html
- So with a normal USB lead plugged in and 5 V on the pin, the hub port and the local converter are tied output to output. With the hub port switched off or the Pi off, the local supply back-feeds the hub. The hub's port switch can no longer power-cycle the board either.

Why one Schottky diode is not enough here: a diode in the local 5 V feed stops the hub charging the local converter. It does not stop the local supply reaching the hub port, because the board joins VBUS and the header pin directly. The Pico-style fix works on a Pico because that board has its own diode between VBUS and its system rail (from memory; the Raspberry Pi datasheet site refused to load in this check). The Zero has no such rail.

Fixes, simplest first:

1. **Power the trust root only from its own 5 V. Break VBUS in the USB lead** (a lead or adapter with the power wire cut; not a "data blocker", which does the opposite). Data still works. The hub can no longer cut the trust root's power, which suits a trust root. UNVERIFIED: that the Rust USB stack enumerates without VBUS sensing on this board. Test it.
2. **Power multiplexer.** Pololu TPS2113A carrier: two inputs of 2.8 to 5.5 V, picks the higher, blocks reverse current into either source, about 10 dollars. Source: https://www.pololu.com/product/2596. It must sit in the USB power line, so the lead still has to be opened. Amazon availability not checked.
3. Two Schottky diodes, one in each feed, again with the USB lead opened. Costs about 0.3 to 0.4 V.

Also: feed the WS2812 strip and the relay board from the 5 V converter directly, not through the Zero's pin.

## 6. WS2812B data from 3.3 V logic

**Verdict: CONFIRMED that a level shifter should be added. The reason is weaker than assumed for new chips.**

- The current WorldSemi datasheet (WS2812B V5, 2019) gives the input-high threshold as 2.7 V minimum, as an absolute figure, and input-low as 0.7 V maximum. Supply 3.7 to 5.3 V. Reset is a low of more than 280 µs. Source: https://mm.digikey.com/Volume0/opasdata/d220001/medias/docus/8880/WS2812B.pdf
- Older datasheets gave 0.7 times the supply, which is 3.5 V at 5 V (from memory, not re-read). 3.3 V is below that.
- Which chip revision, or which clone, is on the SEZO strip is UNVERIFIED. It cannot be known before the strip is in hand.
- Adafruit's guide says a mismatch shows up as wrong colours, flicker or dead-looking pixels, worse with long wires, and names the 74AHCT125 as the fix. Source: https://learn.adafruit.com/neopixel-levelshifter/shifting-levels

Part to add: **one 74AHCT125 in a DIP package** (SN74AHCT125N), powered from 5 V. One gate for the data line. Tie the unused inputs to ground. Add a few hundred ohms in series at the strip's data input (from memory, Adafruit's general advice). These lights are the "truthful status" display, so marginal logic levels are not acceptable even if they seem to work.

## 7. LED push buttons (B086SPXGTS) and indicator lights (B0816FMP9B)

**Verdict: UNVERIFIED for both listings. The need for a driver is certain.**

- Neither listing could be read. LED voltage, LED current, contact arrangement and pin layout are all UNVERIFIED. Settle by reading the label and measuring: feed the LED pins from 12 V through an ammeter.
- Typical for this class (from memory): 16 mm buttons with one NO and one NC contact and a separate LED pair with a built-in resistor, 10 to 20 mA. 22 mm indicator lamps of the AD16 type draw about 20 mA.
- A GPIO cannot drive a 12 V LED. It gives 3.3 V and a few milliamps.
- **Driver to add:** a 2N3904 NPN per LED, with 4.7 k from the GPIO to the base, emitter to ground, LED between 12 V and the collector. It turns on fully from 3.3 V. A 2N7000 is not guaranteed to turn on fully at 3.3 V (from memory). For more than two or three LEDs use one ULN2003A in DIP instead.
- Feed the button LED from the always-on 12 V, not from the armed rail, so a pending approval can be shown while disarmed.
- The indicator lamps go straight across the rails with no GPIO. At 12 V they are at the bottom of their 12 to 24 V range and may be dim. The lamp on the armed rail goes across the load terminals. It also bleeds the limiter's output capacitor after a cut, which is useful.
- The button contact goes from a GPIO to ground with a pull-up (see section 10 for why not a pull-down).

## 8. Emergency stop (B07R8PTTDX), key switch (B08JWP92K4), covered toggle (B07T6YNRZS)

**Verdict: UNVERIFIED for the emergency stop and key switch. Partly CONFIRMED for the toggle.**

Emergency stop:

- The listing could not be read. "Two NC contacts" comes only from the parts list. Check both contacts with a meter on arrival.
- Generic 22 mm emergency stops of this style use clip-on contact blocks marked for AC, commonly 10 A (from memory). **A DC rating is usually not published, and none was found.** At 12 V and under 3 A this is a light duty for such contacts, but that is a judgement, not a rating.
- The second contact carries only a logic signal. Power contacts can be unreliable at very small currents (from memory). Sense it from 12 V through a divider at a few milliamps, not from 3.3 V at microamps.
- Panel hole: 22 mm class, nominally 22.3 mm (seen in distributor listing titles for branded 22 mm units; not confirmed for this part).

Key switch:

- The listing could not be read. Contact arrangement, DC rating, and **whether the key comes out in both positions are all UNVERIFIED.** Only the part in hand settles it.
- It matters. If the key comes out in the ON position, someone can enable the class 3 rail and walk away with the key. The design should assume that until the part shows otherwise.

Covered toggle (DaierTek):

- Daier's own page for this switch family: 20 A at 12 V DC, SPST on-off, three quick-connect terminals (6.3 mm), 12 V LED, 12 mm panel hole. Source: https://daierswitches.com/products/20a-12v-on-off-lighted-toggle-switch-with-safety-cover-and-mounting-panel
- That the Amazon ASIN is this exact model is UNVERIFIED.
- **The page does not say how the lamp is wired**, and a buyer review on it complains about exactly that. The usual arrangement for three-terminal lit toggles is supply, load and ground, with the LED between load and ground so it lights when the switch is on (from memory). UNVERIFIED: find it with a meter.
- If so, the lamp means "arm switch on and 12 V present up to here". It does not mean the rail is live. The lamp needs its own ground wire.
- Whether closing the cover forces the switch off is not stated. Check in hand.
- This is the one Rung 3 switch with a published DC rating, and 20 A is far above the need.

## 9. OLED, 0.96 inch I2C (B09T6SJBV5)

**Verdict: UNVERIFIED for the Hosyond listing. The common reference design is fine at 3.3 V.**

- A datasheet for a 0.96 inch 128 by 64 I2C module of this type gives: SSD1306 driver, address 0x3C, about 20 mA, an on-board regulator so it accepts 5 V. Source: https://mm.digikey.com/Volume0/opasdata/d220001/medias/docus/5858/333099%20Display%20OLED%20I2C%20White%200.96%20INCH%20SSD1306%20datasheet.pdf. That is another maker's board, not Hosyond's.
- 0.96 inch modules are normally SSD1306 or the compatible SSD1315. SH1106 is normal on 1.3 inch modules (from memory).
- Generic four-pin boards usually carry pull-ups on SDA and SCL to their VCC pin (from memory). Value and presence UNVERIFIED for this board.
- What to do: power it from the Zero's 3.3 V pin, so any pull-ups go to 3.3 V. Run an I2C scan on arrival to confirm 0x3C. Read the silk before wiring, because the order of VCC and GND differs between makers. Measure SDA to VCC with a meter to find the pull-up value.

## 10. GPIO count on the RP2350-Zero

**Verdict: CONFIRMED. It fits on the edge pins with room to spare.**

- 29 GPIO in total. 20 are on the edge pins. The other 9 are solder points only. Sources: https://www.waveshare.com/wiki/RP2350-Zero and https://docs.zephyrproject.org/latest/boards/waveshare/rp2350_zero/doc/index.html
- The pad table for the RP2040-Zero, whose outline this board shares, lists the edge pads as 5 V, ground, 3.3 V, then GPIO 29, 28, 27, 26, 15, 14 and down to 0. So the 20 edge GPIO are 0 to 15 and 26 to 29. Source: https://nuttx.apache.org/docs/13.0.0/platforms/arm/rp2040/boards/waveshare-rp2040-zero/index.html. The RP2350-Zero schematic lists the same GPIO names and puts the on-board RGB LED on GPIO 16. The exact pin-by-pin order on the RP2350-Zero was read from a picture-only pinout, so confirm against the silk.
- Analog inputs are GPIO 26 to 29 only.
- Default I2C in the Zephyr board file is GPIO 4 and 5.
- The schematic shows a three-pin debug connector (SWCLK, ground, SWDIO). So debug access appears to exist as pads. Where they are on the board is UNVERIFIED. This bears on the debug-probe question in the cross-reference.

Signals:

| # | Signal | Direction | Suggested pin type |
| --- | --- | --- | --- |
| 1 | I2C SDA (OLED) | both | GPIO 4 |
| 2 | I2C SCL (OLED) | out | GPIO 5 |
| 3 | Approval button | in | digital, pull-up |
| 4 | Button LED | out | digital, to transistor |
| 5 | Emergency stop sense | in | digital, through divider or optocoupler |
| 6 | Arm switch sense | in | digital, through divider or optocoupler |
| 7 | Key switch sense | in | digital, pull-up |
| 8 | WS2812 data | out | digital, to 74AHCT125 |
| 9 | Rail switch drive | out | digital, external pull-down |
| 10 | Relay heartbeat | out | digital, software-toggled |
| 11 | Rail-live feedback | in | digital from optocoupler, or analog on GPIO 26 |
| 12 | Feedback between the two cuts | in | analog on GPIO 27, to tell which cut failed |
| 13 | Firmware-update jumper | in | digital, pull-up |
| 14 | Spare | | |

14 of 20 edge pins. Six left over. No underside pads are needed.

Two notes for the wiring:

- **Use pull-ups with the switch to ground, not internal pull-downs.** Early RP2350 chips (A2 stepping) leak current on inputs, which defeats internal pull-downs. The A4 stepping fixes it. Source: https://hackaday.com/2025/07/31/raspberry-pi-rp2350-a4-stepping-addresses-e9-current-leakage-bug. Which stepping these Waveshare boards carry is UNVERIFIED (read the chip marking or the chip ID in firmware).
- Put an external pull-down on each drive pin (signals 9 and 10) so outputs stay off through boot and reset, as the review response requires.

## 11. Fuses (holders B0CL7MLY6T, assortment B07Q9PL4R6) and 22 AWG wire

**Verdict: values suggested below. The contents of the two listings are UNVERIFIED.**

How blade fuses behave (Littelfuse ATOF series, 32 V DC). Source: https://www.mouser.com/catalog/specsheets/littelfuse_littelfuse_atof_datasheet.pdf

- At 110 percent of rating: never opens (100 hours minimum).
- At 135 percent: anywhere from under a second to 10 minutes.
- At 200 percent: 0.1 to 5 seconds.
- At 350 percent: under half a second.
- Standard values start at 1, 2, 3, 4, 5, 7.5 A.

What follows:

- The adapter can give about 3 A. A **3 A fuse would need over 4 A to open. It never blows.** Fitting one is decoration.
- 22 AWG copper is not at risk at 3 A in open wiring (from memory: common chassis-wiring tables allow well above that). So the adapter already protects the wire. The fuses are there to limit fault energy and to give a part that fails open.
- Loads on the rail adapter: the limiter at its 15 W cap draws about 1.4 A at 12 V. The trust root's 5 V converter with relays, display and a few LEDs draws about 0.2 to 0.3 A at 12 V (estimate from the figures in sections 3 and 9).

Suggested layout from the rail adapter:

- Adapter, then a split into two branches.
- **Trust-root branch: 1 A fuse**, then the 5 V converter. This branch leaves **before** the emergency stop, so the controller stays alive and can report when the stop is pressed.
- **Rail branch: 2 A fuse**, then emergency stop, arm switch, limiter. At a 3 A fault this is 150 percent, so it opens in seconds to minutes. It is a backstop, not a fast cut. This is one more reason the hard cap needs an eFuse if the 15 W claim must hold after one fault.
- Hub adapter: the hub's barrel lead would have to be cut to add a fuse. If fused at all, 2 A. The hub's real draw should be measured first. This feed is outside this slice.

UNVERIFIED, check the listings or the parts in hand:

- That the assortment includes 1 A and 2 A fuses. Many kits start at 2 A or 5 A.
- That the fuses and the holders are the same size family (standard blade or mini).
- The holders' lead gauge. They usually come with much thicker pigtails than 22 AWG, so a proper splice is needed.
- The 22 AWG listing itself (copper or copper-clad aluminium, insulation rating).

## Gaps to add to the list (none ordered)

| Gap | Section |
| --- | --- |
| 74AHCT125, DIP | 6 |
| 2N3904 transistors and a resistor assortment | 4, 7 |
| Small Schottky diodes, 100 nF film or C0G and 1 µF capacitors | 4 |
| PC817 optocouplers | 2, 10 |
| A USB lead or adapter with VBUS cut | 5 |
| A P-channel MOSFET for a high-side rail switch (before any grounded load) | 2 |
| A fixed 5 V converter for the trust root, in place of an adjustable one | 1 |
| An eFuse board from a distributor, if a hard cap is required | 1 |
| 1 A and 2 A blade fuses, if the kit lacks them | 11 |

## Sources that could not be read

- All Amazon product pages (robots block).
- Raspberry Pi datasheet site (blocked). This leaves the RP2350's 5 V tolerance and unpowered-pin behaviour unconfirmed in this check.
- done.land XL4015 page (site error).
