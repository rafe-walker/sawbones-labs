# Round 5, slice 1: the brain, build tools, RP2350-Zero, USB cables

Checked 26 November 2026 against manufacturer pages, schematics and project wikis. Nothing was bought or added to any list.

Amazon product pages refused to load. Where the exact listing could not be seen, the section says which page was used instead.

Verdict words: CONFIRMED (a source says so), WRONG (a source contradicts the list), UNVERIFIED (no source found; the section says what would settle it).

## Changes this forces

1. **Keep the RP2350-Zero. Keep the Pico debug probe.** The Zero has SWD, on three solder pads on its underside. Solder three thin wires to those pads before a board is mounted. No different board is needed for the trust root.
2. **Change the trust root's power wiring.** The Zero's 5V pin is the USB VBUS line, with no diode between them. Feeding the board from a hub port and a separate 5 V supply ties the two supplies together. Pick one source: power the trust root from its own 5 V supply only, and open the VBUS wire in its USB lead (data and ground only). The hub then cannot power-cycle the trust root.
3. **Replace or test the 12 V adapter for the hub.** The Facmogu plug is 5.5 x 2.5 mm. The hub's jack is 5.5 x 2.1 mm, centre positive. It may fit loosely. Either do a wiggle test on arrival or buy a true 5.5 x 2.1 mm, 12 V, 2 A or larger supply for the hub.
4. **Add a hub upstream cable, once the connector is known.** Phidgets does not say what the upstream socket is and does not list a cable in the box. Check the listing photos before ordering.
5. **Treat the kit's power supply and cooler as third-party.** The iRasptek kit ships iRasptek-branded parts, not the official Raspberry Pi 27 W supply or official Active Cooler. Either accept and test on arrival, or buy the three official parts separately (about $136 at PiShop, per the parts list).
6. **Replace the KingSpec drive, or accept a boot risk.** No source confirms this model on a Pi 5. The safe swap is the Raspberry Pi SSD 256 GB (2230), which fits the X1001.
7. **Hub switching is done with `uhubctl`, not a Phidget library.** Plan the supervisor around `uhubctl` and a udev rule. Test it on arrival, because Phidgets itself does not document the feature.
8. No change: clock battery (if the cell is marked ML2020), microSD, logic analyzer, hub power budget, six-of-seven port plan.

Still UNVERIFIED after this pass: hub upstream connector and box contents; whether the iRasptek supply offers 5 A; whether the iRasptek cooler clears the X1001; the Zero's real flash size (wiki says 4 MB, schematic shows a 2 MB chip); KingSpec boot on Pi 5; that the short USB-C cables carry data; sigrok packages on arm64; whether 24 MHz capture holds on the Pi.

---

## Waveshare RP2350-Zero (B0DYTVCF2H)

**Verdict: CONFIRMED for debug access and pin count. One conflict on flash size, UNVERIFIED.**

Source used: Waveshare's own schematic PDF and the pinout drawing on the Waveshare wiki. Both were read directly.

Pins:

- Edge pins, 23 in all, castellated with holes for 2.54 mm headers: 5V, GND, 3V3, GP0 to GP15, GP26 to GP29.
- Underside pads, 10 in all: GND and GP17 to GP25. These are small solder pads, not header holes.
- GP16 is not brought out. It drives the on-board RGB LED.
- So 20 GPIO are on header pins and 9 more are on underside pads. 29 in total. Four of the header pins are ADC inputs (GP26 to GP29).

Debug:

- SWD is present. Three pads on the underside, next to the USB socket, marked CLK, GND and DIO. The schematic calls this part SWD1 and wires it to the chip's SWCLK and SWDIO pins.
- Pads only. There is no header and no socket.

Buttons and LED:

- Two buttons: BOOT and RUN. RUN is the reset button.
- The LED is a WS2812B addressable RGB LED on GP16. There is no plain LED.

Power:

- USB VBUS goes straight to the 3.3 V regulator (ME6217C33M5G) and straight to the 5V pin. They are the same wire.
- The only diode on VBUS is an ESD clamp to ground (ESD5451N). It does not block current between USB and the 5V pin.
- The USB-C socket has the two 5.1 k resistors on CC1 and CC2, so a USB-C host will power it. Not needed for the USB-A cables here.

Flash size, UNVERIFIED:

- The wiki text says 4 MB of flash.
- The schematic shows a W25Q16JVUXIQ, which is a 16 Mbit (2 MB) part.
- The two disagree. The schematic also carries a title typo, so it may be an older drawing. What settles it: read the flash ID on the board in hand (`picotool info -a`, or through the debug probe).

What it means for the build:

- The debug probe has something to connect to. Question 1 in the cross-reference ("Debug access") is closed: keep the Zero, keep the probe.
- The SWD pads face the prototype board once the Zero is soldered down on headers. Solder three thin wires to CLK, GND and DIO first and bring them to a 3-pin header on the prototype board. Do this on every board that will be debugged.
- The trust root's needs (display 2 pins, button, button LED, stop sense, arm sense, key, light strip data, rail enable, relay or heartbeat, rail feedback) come to about 11 or 12 pins. That fits in the 20 header pins without touching the underside pads.
- The trust root cannot take 5 V from both the hub port and its own supply as drawn. With no diode on the board, the separate supply would push current back into the hub port, and the hub switching that port off would not power the board down. A diode on the separate supply alone does not fix this, because the 5V pin still connects straight out through the USB socket. The clean fix is one power source: its own 5 V supply, with the VBUS wire opened in that board's USB lead. Modules 1 and 2 are not affected, since they are powered only through their isolators.
- Two firmware slots need the flash size settled first. With 2 MB, each slot is under 1 MB.

Sources:

- Schematic: https://files.waveshare.com/wiki/RP2350-Zero/RP2350_Zero.pdf
- Wiki, features and pinout drawing: https://www.waveshare.com/wiki/RP2350-Zero
- Zephyr board page (confirms 20 edge GPIO, the rest on solder points, BOOT and RESET): https://docs.zephyrproject.org/latest/boards/waveshare/rp2350_zero/doc/index.html

### Fallback board, only if soldering to pads is unwelcome

Not needed, since SWD is usable. For the record: the Waveshare RP2350-Plus has USB-C, the Pico 2 pin layout, BOOT and RESET buttons, and "DEBUG points" listed as an on-board item. Waveshare prices it at $5.99 to $6.99 direct. Whether its debug points are a header, and its Amazon US price, were not confirmed.

Source: https://www.waveshare.com/rp2350-plus.htm

---

## Raspberry Pi Pico H as the debug probe (B08X7HN2VG)

**Verdict: CONFIRMED. A Pico running the official debugprobe firmware is a sound probe for an RP2350.**

Source used: the raspberrypi/debugprobe repository and releases, Raspberry Pi's Debug Probe documentation, and the probe-rs changelog. The Amazon listing was not seen.

Facts:

- The official firmware builds for a plain Pico as `debugprobe_on_pico.uf2`. Its USB name is "Debugprobe on Pico (CMSIS-DAP)".
- Wiring on the Pico, from the firmware's own pin file: GP2 is SWCLK, GP3 is SWDIO, GP4 is UART TX, GP5 is UART RX. GP1 is an optional reset line. Any GND pin is ground.
- The current firmware release is 2.3.1. The 2.2.0 release fixed probe crashes across a Pi 5 reboot, so use a recent one.
- The probe is a generic CMSIS-DAP adapter. RP2350 support lives in the host tool, not the probe. probe-rs supports every CMSIS-DAP probe, and its changelog records RP235x flashing, reset and attach support.
- Both sides are 3.3 V. Raspberry Pi's guide says to connect ground first when the target has its own power.
- OpenOCD with an RP2350 target file was not checked. Raspberry Pi's page only shows the RP2040 commands. UNVERIFIED, and not needed if the Rust flow uses probe-rs.

What it means for the build:

- Three wires from the Pico (GP2, GP3, GND) to the Zero's CLK, DIO and GND pads. Two more (GP4, GP5) give a serial console if wanted.
- A micro-USB cable is still needed. It stays on the gaps list.
- probe-rs needs its udev rules file installed on the Pi so it runs without root.
- One probe serves one target at a time. Moving it between three boards means a 3-pin header on each.

Sources:

- Pin file: https://github.com/raspberrypi/debugprobe/blob/master/include/board_pico_config.h
- Repository and build names: https://github.com/raspberrypi/debugprobe
- Releases: https://github.com/raspberrypi/debugprobe/releases
- Wiring and voltage notes: https://www.raspberrypi.com/documentation/microcontrollers/debug-probe.html
- probe-rs probe setup: https://probe.rs/docs/getting-started/probe-setup/
- probe-rs changelog (RP235x entries): https://github.com/probe-rs/probe-rs/blob/master/CHANGELOG.md

---

## Phidgets HUB0003_0, 7-port USB 2 hub (B0CSLM56BC)

**Verdict: CONFIRMED for power input, port limit and switching method. UNVERIFIED for the upstream connector and box contents.**

Source used: the Phidgets product page and user guide, two reseller pages that repeat the Phidgets spec table, and the uhubctl README.

Power input:

- Barrel jack, 5.5 x 2.1 mm, centre positive.
- 8 V to 30 V DC. The parts list's "8 to 30 V" is right.
- The hub draws up to 3 A from its supply. It has a 5 A blade fuse.
- Phidgets recommends a 12 V 2 A supply to run all seven ports at full load.

Ports:

- 500 mA per port.
- High-speed USB 2.0.
- Downstream ports take USB Type-A plugs. Phidgets sells Type-A to Mini-B leads for plugging devices into it.

Upstream connector and cable, UNVERIFIED:

- Neither Phidgets nor the resellers name the upstream socket.
- No page lists a cable or power supply in the box. The user guide tells you to have a USB cable and a power supply on hand, which reads as "not included".
- One reseller describes its Type-A to Mini-B lead as being for Phidgets devices and hubs. That hints at Mini-B but is not proof.
- What settles it: the mechanical drawing on the Phidgets product page, or the Amazon listing photos.

How port power is switched:

- Phidgets' own pages say nothing about switching port power. The hub is not a Phidget device. There is no Phidget channel and no VINT involved. The Phidget library is not used.
- The evidence is the uhubctl project's list of hubs known to switch per port. It lists "Phidgets HUB0003_0, 7 ports, USB 2.0, ID 1A40:0201, released 2017", with no end-of-life date.
- So switching is the standard USB hub port-power request, sent with `uhubctl`.
- On Linux 6.0 and later, uhubctl uses the kernel's own per-port `disable` file. A udev rule is needed to run it without root. The README gives the rule.

From a Pi 5:

- uhubctl is a normal Linux tool and the README documents Pi use. Nothing ties the Phidgets hub to a host type.
- The Pi 5's own four ports switch as one group only. That does not affect a hub plugged into them.
- Address the hub by its location (`-l`), not by its ID. The ID 1A40:0201 is not unique to Phidgets.

Not checked: whether the powered hub pushes 5 V back up its upstream port into the Pi. Measure VBUS on the hub's upstream socket with the hub powered and unplugged from the Pi.

What it means for the build:

- The plan works: one port per board, each one switchable from the brain.
- Because the maker does not document switching, test it first thing: `uhubctl -l <location> -p <n> -a off`, then measure that port's 5 V.
- Phidgets sells it direct at $55.00, against $68.99 on the Amazon list.

Sources:

- Product page and spec table: https://phidgets.com/?prodid=952
- User guide: https://phidgets.com/docs/HUB0003_User_Guide
- Reseller spec copies: https://www.robotshop.com/products/7-port-usb-hub and https://www.nodna.de/Phidgets-7-Port-USB-Hub-HUB0003-0
- uhubctl README, supported hubs table and udev rules: https://github.com/mvp/uhubctl

---

## Facmogu 12 V 3 A adapter (B073WSWT34)

**Verdict: voltage and polarity CONFIRMED. Plug fit UNVERIFIED, and likely loose.**

Source used: a Newegg listing for the same Facmogu adapter (its page address carries this ASIN). The Amazon page was not seen.

Facts:

- 12 V DC, 3 A, 36 W. Centre positive.
- The plug is 5.5 mm outside, 2.5 mm inside. The listing says a spring contact lets it also work in 2.1 mm sockets.
- The listing claims UL certification. Not checked further.
- About 2 m of cable.

What it means for the build:

- 12 V sits inside the hub's 8 to 30 V range. Polarity matches.
- The hub's jack has a 2.1 mm pin. A 2.5 mm plug goes on but can sit loose. A loose plug on the hub means every board browns out together.
- Either wiggle-test it on arrival with a meter on a hub port, or buy a supply with a true 5.5 x 2.1 mm plug for the hub and keep the Facmogu for the armed rail.
- The barrel socket to screw-terminal adapter for the rail (already on the gaps list) should be one made for 2.5 mm plugs, or checked the same way.

Source: https://www.newegg.com/cheezee-b073wswt34c3fg/p/2RC-0EE1-000H1

---

## USB power budget for the hub

**Verdict: CONFIRMED. The supply covers it with wide margin.**

- Worst case by the hub's own limits: seven ports at 500 mA is 3.5 A at 5 V, or 17.5 W. Phidgets says a 12 V 2 A supply (24 W) covers that. The 12 V 3 A adapter gives 36 W.
- The plan uses six ports. Worst case is 3 A at 5 V, or 15 W.
- Real draw will be far lower. A bare RP2350 board, a Pico and a logic analyzer each draw well under 100 mA. That figure is general knowledge, not measured. Measure on the bench.
- Each port is capped at 500 mA. Nothing hung off a board's 5V pin may exceed that. The light strip is the one to watch: do not power it from a hub-fed 5V pin.
- The hub has its own supply, so the Pi's USB current limit is not in play for the boards.

Sources: https://phidgets.com/?prodid=952 and https://phidgets.com/docs/HUB0003_User_Guide

---

## Raspberry Pi 5 4 GB kit, iRasptek (B0FL21867J)

**Verdict: the assumption that the kit has the official 27 W supply and official cooler is WRONG on the evidence found. Whether the supply offers 5 A is UNVERIFIED.**

Source used: a Newegg listing for the iRasptek "Basic Starter Kit for Raspberry Pi 5 4GB RAM, includes 4GB board, 27W PD power supply and active cooler". Same brand and same three parts as the Amazon line. The Amazon page was not seen, so the match is by description.

Facts:

- That listing names the parts as an "iRasptek 27W USB-C Power Supply" and an "iRasptek Active Cooler". Neither is presented as the official Raspberry Pi part.
- The listing gives no output ratings for the supply.
- Raspberry Pi's documentation: with a supply that offers 5 A, the Pi 5 allows 1.6 A in total to its USB ports. With any lesser supply it allows 600 mA and shows a warning. USB boot is off by default on a 3 A supply.

What it means for the build:

- The risk is small for this bench. The hub has its own supply and the drive is on PCIe, not USB. A Pi limited to 600 mA of USB current still runs this design.
- What settles it: the supply's label should read 5.1 V 5 A. After first boot, `vcgencmd get_config usb_max_current_enable` should return 1, with no low-power warning.
- If certainty matters more than one-day delivery, buy the board, official 27 W supply and official Active Cooler separately.

Sources:

- Kit listing: https://www.newegg.com/p/3C6-012R-00WW7
- Power supply rules: https://www.raspberrypi.com/documentation/computers/raspberry-pi.html (Power supply section)

---

## Geekworm X1001 NVMe board (B0CPPGGDQT)

**Verdict: CONFIRMED with the official cooler. UNVERIFIED with the iRasptek cooler.**

Source used: Geekworm's wiki and product page, and Jeff Geerling's Pi PCIe database.

Facts:

- It mounts on top of the Pi 5.
- Geekworm states it supports the official Pi 5 Active Cooler underneath.
- It stands on three 17 mm spacers, which are in the box. The box also has a 30 mm PCIe ribbon and the screws.
- It takes 2230, 2242, 2260 and 2280 drives. There is no fixed standoff for the three shorter lengths. Geekworm suggests an M2 x 6 mm screw with two nuts, and says a copper pillar has been in the box since April 2024.
- Which face of the board the drive sits on is not stated by Geekworm. Jeff Geerling's page says the drive is held over the top of the Pi. The exact face is UNVERIFIED and does not change the build.
- Power comes through the ribbon, up to 5 W. There is a separate 5 V socket for hungry drives. Its cable is sold separately.
- Geekworm does not call it a HAT+ board. Raspberry Pi's documentation says boards that are not HAT+ need PCIe switched on by hand (`dtparam=pciex1`) and need `PCIE_PROBE=1` in the bootloader settings to boot from the drive. The boot order for NVMe first is `BOOT_ORDER=0xf416`. Plan for these steps.

Drives Geekworm lists as trouble: WD Green SN350, WD Green SN580, WD Black SN770, WD Black SN850, Inland TN446, Corsair MP600, and drives with Phison or Polaris controllers. KingSpec is not named either way.

What it means for the build:

- The stack is: Pi, cooler, 17 mm gap, X1001, drive. Fine with the official cooler.
- The iRasptek cooler's height is unknown. If it matches the official cooler it fits. Check on arrival, before the drive is mounted.
- The microSD card is needed for the first boot and the bootloader settings. Its row stands.

Sources:

- Wiki: https://wiki.geekworm.com/X1001
- Product page: https://geekworm.com/collections/raspberry-pi/products/x1001
- Geerling's entry: https://pipci.jeffgeerling.com/hats/geekworm-x1001-nvme-hat.html
- NVMe boot settings: https://www.raspberrypi.com/documentation/computers/raspberry-pi.html (NVMe SSD boot section)

---

## KingSpec 256 GB NVMe 2280 (B0D1R7HDJF)

**Verdict: UNVERIFIED. Neither known-good nor known-bad.**

Source used: Geekworm's wiki, SunFounder's Pi 5 drive list, and Jeff Geerling's database. The Amazon listing was not seen, so the exact KingSpec model and controller are unknown. Pimoroni's notes were not checked.

Facts:

- Geekworm's trouble list does not name KingSpec.
- SunFounder's Pi 5 list has a "KingSpec M.2 NVMe SSD (1TB)" under verified working. It is a different capacity and no model number is given.
- Jeff Geerling's database has a KingSpec 128 GB 2242 drive. It works on the Compute Module 4. Its Pi 5 status is "untested".
- No source confirms this 256 GB 2280 drive boots a Pi 5.

Known-good swaps:

- Raspberry Pi SSD 256 GB. M.2 2230, PCIe Gen 3, made by Raspberry Pi for the Pi 5, in production until at least January 2032. It fits the X1001 using the short-drive mounting. Price was not shown on the page.
- From SunFounder's verified list at 256 GB: Samsung PM991, Silicon Power P34A60, PNY CS1031, ADATA Legend 700.

What it means for the build:

- The brain boots from this drive, so a drive that is not recognised stops the whole bring-up.
- Swap to the Raspberry Pi SSD 256 GB unless the price gap is large. If the KingSpec is kept, test it the day it arrives, inside the return window.

Sources:

- https://wiki.geekworm.com/X1001
- https://docs.sunfounder.com/projects/pironman5/en/latest/pironman5/compitable_nvme_ssd.html
- https://pipci.jeffgeerling.com/cards_m2/kingspec-m2-nvme-ssd-128gb.html
- https://www.raspberrypi.com/products/ssd/

---

## Pi 5 clock battery, Rome Tech ML2020 (B0D9MJM3V7)

**Verdict: CONFIRMED for the type the Pi needs and the setting. UNVERIFIED that this listing's cell is really an ML type.**

Source used: Raspberry Pi's hardware documentation. The Amazon listing was not seen. The parts list describes it as ML2020.

Facts:

- The official battery is a rechargeable lithium manganese coin cell with a two-pin JST-SH plug.
- Raspberry Pi advises against non-rechargeable cells and says lithium-ion cells must not be used.
- Charging is off by default.
- To switch it on, add `dtparam=rtc_bbat_vchg=3000000` to `/boot/firmware/config.txt` and reboot.
- To check, read `/sys/class/rtc/rtc0/charging_voltage`.

What it means for the build:

- Look at the marking on the cell when it arrives. "ML" means rechargeable. "CR" means not rechargeable.
- Only add the charging line if the cell is an ML type. Charging a CR cell is unsafe.

Source: https://www.raspberrypi.com/documentation/computers/raspberry-pi.html (Real Time Clock section)

---

## microSD 32 GB

**Verdict: no change.** It is needed for first boot and for the bootloader settings the X1001 requires. See the X1001 section.

---

## 8-channel 24 MHz logic analyzer, HiLetgo (B077LSG5P2)

**Verdict: CONFIRMED safe on 3.3 V and 5 V logic, from the common design. Sample rate on the Pi and arm64 packages UNVERIFIED.**

Source used: the sigrok wiki pages for fx2lafw and for a clone of the same design, and SparkFun's page for its version of the same analyzer. These units are all copies of one design. The HiLetgo board itself was not seen.

Facts:

- The design is a Cypress FX2 chip with a 245-type buffer on the inputs.
- Maximum input is 5.25 V. Both sigrok and SparkFun give that figure.
- Thresholds: sigrok lists high at 1.4 V and low at 0.8 V. SparkFun lists high at 2.0 V. Either way, 3.3 V logic reads correctly.
- Input loading is light: over 100 k ohm, about 5 pF.
- sigrok supports this family fully through the open fx2lafw firmware. It shows up with USB ID 0925:3881.
- Top rate is 24 MHz across 8 channels. It can be set down to 20 kHz.
- Triggers are done in software. There is no on-board memory, so every sample streams over USB as it is taken.
- sigrok ships udev rules so it runs without root.
- SparkFun warns that one pin beside the inputs on some of these units carries a clock, not ground. Use the pin marked GND.

UNVERIFIED:

- sigrok on arm64. The sigrok pages do not mention ARM, and the Debian package page would not load. What settles it: on the Pi, `sudo apt install sigrok-cli sigrok-firmware-fx2lafw`, then `sigrok-cli --scan`.
- Real sample rate on the Pi through the hub. At 24 MHz the unit sends 24 MB every second, about half of what USB 2 high speed can carry. Whether that holds on this hub is a bench test. It does not matter for this build: serial at 115200 baud, I2C at 400 kHz and a heartbeat line are all captured well at 1 to 4 MHz.

What it means for the build:

- Safe on every 3.3 V and 5 V signal on the boards.
- Never on the RS232 side, and never on the 12 V rail. Both exceed 5.25 V.
- It likely needs a Mini-B USB lead. Whether one is in the box was not confirmed.

Sources:

- https://sigrok.org/wiki/Fx2lafw
- https://sigrok.org/wiki/VKTECH_saleae_clone
- https://sparkfun.com/products/retired/15033

---

## Short USB-A to USB-C cables (B08HV7GT4H)

**Verdict: UNVERIFIED.**

The listing could not be loaded and no other page for this ASIN was found.

- Connector types are right for the job: USB-A at the hub, USB-C at the Zero.
- Whether they carry data is not confirmed. What settles it: the listing should say "data" or "sync" or give a speed such as 480 Mbps. On arrival, hold BOOT on a Zero, plug it in with each cable, and check the board appears as a drive.
- The trust root's cable needs its VBUS wire opened if the one-power-source fix above is taken.
- Count check: three Zeros on the hub plus a spare is four. Six cables is enough.

---

## Cables the list still lacks, from this slice

| Cable | For | Status |
| --- | --- | --- |
| Hub upstream lead | Pi to the hub | Type UNVERIFIED. Probably USB-A to Mini-B. Confirm before ordering |
| Micro-USB lead | Pico debug probe | Already on the gaps list. Still needed |
| Mini-B lead | Logic analyzer | May be in the analyzer's box. Not confirmed |
