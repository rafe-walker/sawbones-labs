# Sawbones review round 1: hardware fact check

Checked 25 November 2026 against vendor documentation, datasheets and official repositories.
All source text is paraphrased. Prices are as seen on 25 Nov 2026 and exclude shipping and tariffs.
Verdict key: CONFIRMED / PARTLY / REFUTED / UNVERIFIED.

## Summary table

| # | Claim | Verdict | One-line finding |
|---|-------|---------|------------------|
| 1 | Pi 5 USB current limit 600 mA vs 1.6 A; override settings; header power | CONFIRMED (limit and settings); PARTLY (header power) | Figures and both setting names match the Raspberry Pi white paper. Header power is a designed-in path (Power HAT+), but which protections it skips is not spelled out for Pi 5. |
| 2 | Pi 5 has no analog audio jack | CONFIRMED | The 3.5 mm audio/composite jack was removed; no analog audio in or out on the board. |
| 3 | Pi 5 gadget mode uses the USB-C power port | CONFIRMED (port); PARTLY (separate power) | Raspberry Pi says the USB-C port becomes network plus power input. The only separate-power method it names is a hub that injects power on that same port. |
| 4 | Pi 5 RTC needs an external battery on J5 | CONFIRMED | RTC runs without a battery while powered; holding time through power-off needs a cell on the 2-pin battery connector. |
| 5 | Plugable USB3-HUB7BC on uhubctl list; 2.4 GHz warning | CONFIRMED, with a new problem | Listed with no caveat, and Plugable does print the warning. Plugable now marks the product discontinued. |
| 6 | Plugable USB2-HUB7BC exists; uhubctl support | PARTLY | It exists (12 V 5 A adapter) but is discontinued and is NOT on the uhubctl list. Only one listed 7+ port USB 2.0 hub is clearly buyable new at a sane price. |
| 7 | USB 3.0 2.4 GHz noise cause and whether it needs a SuperSpeed link | PARTLY | Intel: the noise is the 5 Gbit/s data spectrum, present once the link is established, idle or busy. Absence without a SuperSpeed link is a strong inference, not a quoted statement. |
| 8 | PiKVM V4 Mini capabilities | CONFIRMED | No USB host port, cannot drive USB-controlled switches, has ATX control, rated 0 to 50 C. No user GPIO header is documented. |
| 9 | RP2350 boot ROM has A/B plus try-before-you-buy; RP2040 lacks it | CONFIRMED | RP2350 datasheet sections 5.1.7 and 5.1.17 describe both. RP2040 datasheet has no partition, A/B or rollback feature. |
| 10 | Pico and Pico 2 are micro-USB, 51 mm long | CONFIRMED | Both datasheets: micro-USB B, 51 x 21 mm. USB-C alternatives listed below. |
| 11 | INA228 breakouts: 85 V, 15 milliohm, 10 A | CONFIRMED, with caveats | Both boards: 85 V, 15 milliohm, 10 A peak. Adafruit says 2.75 A continuous in the fine range. Only SparkFun documents an external-shunt jumper. |
| 12 | I2C isolator does not power the far side | CONFIRMED | ISO1540 and ADuM1250 each need a separate supply on each side; neither generates isolated power. |
| 13 | RTL-SDR Blog V3 dongles share serial 00000001; rtl_eeprom fixes it | CONFIRMED (medium confidence on the default) | The official tool has a serial option and its stock EEPROM template uses 00000001. The vendor guide itself does not state the shipped serial. |
| 14 | RTL-SDR Blog V3 range "500 kHz to 1.7 GHz" | PARTLY | Datasheet says 500 kHz to 1766 MHz, but below about 24 MHz it is direct sampling with real limits. |
| 15 | LiFePO4 45 C charge limit; 4-cell pack 10 V to 14.6 V | CONFIRMED (typical, not universal) | Cell datasheets give 2.5 V to 3.65 V per cell. One checked says 0 to 45 C charge, another 0 to 60 C. |
| 16 | Cheap GNSS boards break PPS out only to an LED | PARTLY | True of the common 4-pin NEO-6M board; some 5-pin clones do expose PPS. Two known-good boards listed. |

---

## 1. Raspberry Pi 5 USB current limit and power options

**Verdict: CONFIRMED for the limit and the settings. PARTLY for header power.**

- Raspberry Pi's white paper (release 1, dated 5 Mar 2026) says that when no 5 V 5 A supply is detected the board caps total USB port current at 600 mA instead of 1.6 A, disables USB boot unless the power button is pressed, and shows a desktop warning.
- A supply with no USB-PD at all is treated as 5 V 3 A.
- Only 5 V PD profiles are requested. Higher-voltage profiles are ignored, and programmable (PPS) supplies are not supported.
- Two overrides are documented, and the paper names header power as one reason to use them:
  - `PSU_MAX_CURRENT` in the bootloader EEPROM config: skips PD negotiation and assumes the stated rating, normally 3000 or 5000.
  - `usb_max_current_enable=1` in `config.txt`: raises the USB allowance to 1600 mA. It is set automatically after a successful 5 V 5 A negotiation or when `PSU_MAX_CURRENT` is 5000.
- The general documentation repeats the 1.6 A / 600 mA split and adds that the fan header shares the same budget.
- Header power is a supported design path: the HAT+ specification defines a "Power HAT+" that feeds the board through the 5 V header pins, with a roughly 3 A class and a roughly 5 A class. For the 5 A class the firmware turns on `usb_max_current_enable` by itself. A Power HAT+ must only source current and must tolerate USB-C power being plugged in at the same time.
- Protections: the documentation says anything wired straight to the 5 V rail is invisible to the PMIC's current measurement, and that power arriving by a route other than the power input skips the board's protection circuitry (said in the context of back-powering hubs). It does not list the Pi 5 parts that header power skips.

Not verified: the exact Pi 5 input protection components bypassed by header power. No Pi 5 schematic was found at the expected address (404), and the Raspberry Pi forum threads where engineers discuss this returned an access block.

Practical reading for the bench: a non-PD 5 V rail feeding the header needs `PSU_MAX_CURRENT=5000` (or `usb_max_current_enable=1`) or the four USB ports share 600 mA. Even with it, the ceiling is 1.6 A across all four ports, so a powered hub is still needed for a case full of instruments.

Sources:
- https://pip-assets.raspberrypi.com/categories/685-app-notes-guides-whitepapers/documents/RP-009856-WP-1-USB%20Power%20delivery%20on%20Raspberry%20Pi%205.pdf
- https://www.raspberrypi.com/documentation/computers/raspberry-pi.html (power supply and bootloader configuration sections)
- https://www.raspberrypi.com/documentation/computers/getting-started.html (power supply table)
- https://datasheets.raspberrypi.com/hat/hat-plus-specification.pdf (chapter 4, Power HAT+s)

## 2. Raspberry Pi 5 has no analog audio jack

**Verdict: CONFIRMED.**

- Raspberry Pi's launch post states the four-pole 3.5 mm jack that carried analog audio and composite video was removed. Composite video survives only as two solder pads on the board edge.
- The product page and product brief list no analog audio at all; audio is via HDMI.
- The board never had an audio input, so a microphone or line-level capture needs a USB or I2S device.

Sources:
- https://www.raspberrypi.com/news/introducing-raspberry-pi-5/
- https://www.raspberrypi.com/products/raspberry-pi-5/
- https://cdn.sparkfun.com/assets/9/9/0/d/e/raspberry-pi-5-product-brief.pdf (distributor copy of the official brief)

## 3. Raspberry Pi 5 gadget mode and the USB-C port

**Verdict: CONFIRMED that it is the USB-C port. PARTLY on powering separately.**

- Raspberry Pi's article (21 Jan 2026) says gadget mode on Pi 4, Pi 5, Pi 500 and Pi 500+ uses the board's USB-C port. Once enabled, that port is network link plus power input and cannot host peripherals.
- It warns that some laptop ports cannot supply enough power for these models, causing reboots or link drops.
- Its remedy is the Raspberry Pi USB 3 Hub, or another powered hub, placed in line so that the hub supplies power while the laptop carries only data. Elsewhere it advises avoiding hubs and docks that interfere with role negotiation, so the powered hub is a specific exception.
- The article says nothing about powering through the 40-pin header or PoE while the USB-C port is in device mode, and nothing about whether laptop VBUS and a header supply may be connected at once.

Not verified: whether Raspberry Pi supports header power plus a laptop on USB-C simultaneously. The HAT+ rule that a power HAT must tolerate USB-C being connected covers the HAT side only. A laptop port and a bench rail both on the same 5 V net is the unanswered case; a data-only path (VBUS not connected) or the in-line hub avoids the question.

Source: https://www.raspberrypi.com/news/usb-gadget-mode-in-raspberry-pi-os-ssh-over-usb/

## 4. Raspberry Pi 5 real-time clock and battery

**Verdict: CONFIRMED.**

- The product page lists the RTC as powered from an external battery.
- The documentation says the RTC works without a backup cell while the board has power, but keeping time through a power cut needs a cell on the battery connector.
- The official part is a rechargeable lithium-manganese cell on a 2-pin JST plug. The launch post also mentions a supercapacitor as an option.
- Charging is off by default and is enabled with `dtparam=rtc_bbat_vchg=<microvolts>`. Non-rechargeable lithium cells and lithium-ion cells must not be used.

The "J5" designator itself was not seen in the text retrieved; the documentation calls it the battery connector. Treat the label as unverified, the fact as confirmed.

Sources:
- https://www.raspberrypi.com/products/raspberry-pi-5/
- https://www.raspberrypi.com/documentation/computers/raspberry-pi.html (real-time clock section)
- https://www.raspberrypi.com/news/introducing-raspberry-pi-5/

## 5. Plugable USB3-HUB7BC

**Verdict: CONFIRMED on both points, but the hub is discontinued.**

- uhubctl's compatibility table lists the USB3-HUB7BC: 7 ports, USB 3.0, ID `2109:0813`, released 2015, with no restriction noted. Two siblings carry restrictions: USB3-HUB7C switches only its two charge ports, and USBC-HUB7BC switches six of seven.
- Plugable's page gives the adapter as 12 V 5 A (60 W) and warns that 2.4 GHz devices such as wireless receivers, Bluetooth and Wi-Fi adapters may fail close to USB 3.0 devices or hubs.
- The same page now states the product is discontinued. The listed price was $49.95.
- uhubctl notes two general risks: a USB 3 hub appears as paired USB 2 and USB 3 hubs that must both be switched, and some hubs cut data without actually dropping VBUS.
- uhubctl also notes the Pi 5's own four ports are ganged: they claim per-port switching, but power only drops when all are off.

Sources:
- https://github.com/mvp/uhubctl (README table and notes)
- https://plugable.com/products/usb3-hub7bc

## 6. Plugable USB2-HUB7BC and other USB 2.0 hubs with 7 or more ports

**Verdict: PARTLY. It exists, but it is discontinued and uhubctl does not list it.**

- Plugable's page describes a 7-port USB 2.0 hub with a 12 V 5 A (60 W) adapter, built on two Terminus FE1.1s hub chips, listed at $34.95 and marked discontinued. The page was fetched once; a repeat fetch was rate-limited.
- It is absent from the uhubctl table. No Plugable USB 2.0 hub is listed at all, so per-port switching on it is unproven.

USB 2.0 hubs with 7 or more ports that uhubctl does list:

| Hub | Ports | uhubctl note | Buyable new in the US? (25 Nov 2026) |
|-----|-------|--------------|--------------------------------------|
| Phidgets HUB0003_0 | 7 | none | Yes. $55.00 direct from Phidgets (stock shown as 1000+); $64.82 at RobotShop US (1 left). Needs 8 to 30 V DC input; 12 V 2 A recommended. Ports rated 500 mA each. |
| Moxa UPort 407 | 7 | none | Yes, but industrial pricing: $467.10 at RSP Supply. |
| Coolgear USBG-12U2ML | 12 | none | No. Vendor page marks it end-of-life since August 2023 and points to a USB 3 replacement. |
| D-Link DUB-H7 | 7 | only revisions A, D and E work; B, C, F, G do not | Unverified. Still on D-Link regional sites; the revision a buyer would receive cannot be chosen. |
| Microchip EVB-USB2517 | 7 | none | Unverified. Evaluation board, not a cased hub. |
| Maplin A08CQ | 7 | red ports only | No. Marked end-of-life 2011. |
| Targus PAUH212 / PAUH212U | 7 | none | No. Marked end-of-life 2009. |

The Phidgets hub is the only clear fit: listed, USB 2.0, 7 ports, in stock, and it accepts a 12 V rail directly.

Sources:
- https://plugable.com/products/usb2-hub7bc
- https://github.com/mvp/uhubctl
- https://phidgets.com/?prodid=952
- https://www.robotshop.com/products/7-port-usb-hub
- https://rspsupply.com/p-9743-moxa-uport-407.aspx
- https://www.usbgear.com/USBG-12U2ML.html

## 7. USB 3.0 interference in the 2.4 GHz band

**Verdict: PARTLY. Cause confirmed from Intel. The "no SuperSpeed link, no noise" part is inference.**

What Intel's paper (April 2012, document 841692) says:
- USB 3.0 signals at 5 Gbit/s with scrambled data. The resulting spectrum is broadband from DC to 5 GHz and is strong across 2.4 to 2.5 GHz.
- It radiates from the host connector, the device connector and the cable.
- Measured rise in the 2.4 GHz noise floor: about 20 dB next to a USB 3.0 hard drive, about 25 dB next to a laptop's USB 3.0 socket.
- Their test mouse was fine with the drive unplugged and lagged at 3 feet and beyond once it was connected. The effect was the same whether data was moving or the link was merely established.
- A receiver in the socket stacked directly above the USB 3.0 port stopped responding entirely. Moving it away on a USB 2.0 extension lead restored it.
- Full shielding of the drive cut the noise about 12 dB; a better-grounded host connector cut it at least 10 dB.

What this means for the two questions asked:
- **Hub connected through a USB 2.0 port or cable (no SuperSpeed link):** Intel attributes the noise to the SuperSpeed data spectrum and reports no problem with the device absent. A USB 2.0 cable has no SuperSpeed pairs, so that spectrum cannot be generated on it. Plugable's support article agrees that USB 2.0 ports do not have the problem, while warning that a USB 2.0 port beside an active USB 3.0 port can still be hit. Neither source tests a USB 3.0 hub running on a USB 2.0 uplink. Confidence: high on the physics, but it is inference, not a quoted measurement.
- **USB 3.0 hub on a USB 3.0 host port with only USB 2.0 devices downstream:** the hub's own SuperSpeed uplink to the host still exists, and Intel found an idle established link as noisy as a busy one. So the uplink cable and both its connectors should be assumed noisy. Whether the link drops to a quiet low-power state when nothing SuperSpeed is attached depends on the host and operating system and was not verified. Confidence: medium.

Design consequence: the dependable way to remove this noise source is to have no SuperSpeed link in the case at all (USB 2.0 hub, or a USB 3.0 hub on a USB 2.0 uplink). Verify on the built unit; an RTL-SDR cannot check it directly because 2.4 GHz is above its range.

Sources:
- https://www.intel.com/content/www/us/en/content-details/841692/usb-3-0-radio-frequency-interference-impact-on-2-4-ghz-wireless-devices-white-paper.html
- https://cdrdv2.intel.com/v1/dl/getContent/841692?fileName=usb30radiofrequencyinterference.pdf
- https://kb.plugable.com/usb-hub/wireless-usb-device-not-working-on-usb-30-hub

## 8. PiKVM V4 Mini

**Verdict: CONFIRMED on all four hardware points. GPIO is effectively "no" for this use.**

- **USB host ports:** none. PiKVM's ezCoo guide says the V4 Mini has no USB host port and calls it a single-host device. Apart from power, the Mini's USB-C sockets are the emulation (OTG) port and a serial console port.
- **Multiport switches:** the multiport page excludes the Mini from the PiKVM Switch and says it works only with switches controlled over the network, such as TESmart. USB-controlled switches need a host port.
- **GPIO for relays or servos:** no user GPIO header is documented for the Mini. The GPIO guide mentions an internal PWM-capable pin on V4 boards for servo use, without saying it is reachable on the Mini. The USB relay driver needs a host port, which the Mini lacks. Treat relays and servos as unsupported unless the case is opened and the pin is confirmed.
- **ATX control:** yes. An RJ45 ATX control port is on the rear, and the datasheet describes it as a power control or auxiliary port.
- **Temperature:** datasheet gives 0 to 50 C operating and -20 to 60 C storage. Peak draw is listed as up to 12 W; power input is 5.1 V 3 A on USB-C.

Sources:
- https://docs.pikvm.org/v4/
- https://docs.pikvm.org/v4/v4mini_datasheet.pdf
- https://docs.pikvm.org/multiport/
- https://docs.pikvm.org/ezcoo/
- https://docs.pikvm.org/gpio/
- https://github.com/pikvm/pikvm (docs folder, same text)

## 9. RP2040 versus RP2350 boot ROM

**Verdict: CONFIRMED.**

- RP2350 datasheet section 5.1.7 (A/B versions): two partitions can be paired so a new image is written to the spare one while the current one stays bootable; the boot ROM normally picks the higher version.
- Section 5.1.17 (try before you buy): after a flash-update reboot the ROM boots the new image under a watchdog. The image has 16.7 seconds to confirm itself. If it does not, the board reboots into the previous image. If it does, the old image is invalidated and the new one becomes the default.
- A new image that fails validation or signature checking is also skipped in favour of the old one.
- Sections 5.1.11 and 5.1.16 cover anti-rollback and deliberate downgrade.
- RP2040 datasheet (section 2.8): the boot ROM loads a small second-stage loader from flash and offers USB mass-storage and PICOBOOT. A full-text search finds no partition table, A/B or rollback feature. On RP2040 any such scheme must be a custom bootloader in flash.

Sources:
- https://datasheets.raspberrypi.com/rp2350/rp2350-datasheet.pdf (redirects to https://pip-assets.raspberrypi.com/categories/1214-rp2350/documents/RP-008373-DS-4-rp2350-datasheet.pdf)
- https://datasheets.raspberrypi.com/rp2040/rp2040-datasheet.pdf

## 10. Pico connector and size, and USB-C alternatives

**Verdict: CONFIRMED.** Both the Pico and Pico 2 datasheets give a micro-USB B port and a 51 x 21 mm board, 1 mm thick, with the connector overhanging one end.

Small RP2350 boards with USB-C (25 Nov 2026):

| Board | Size (mm) | Price | Stock seen | Seller |
|-------|-----------|-------|------------|--------|
| SparkFun Pro Micro RP2350 | 33.0 x 17.8 | $16.95 | In stock | SparkFun (US) |
| Adafruit Feather RP2350 with HSTX | 50.8 x 22.8 | $12.50 ($15.50 with PSRAM) | 58 in stock | Adafruit (US) |
| SparkFun Thing Plus RP2350 | 58.4 x 22.9 | $30.95 | In stock | SparkFun (US) |
| Seeed XIAO RP2350 | 21 x 17.8 | $4.90 | Listed as available at DigiKey; count not seen | Seeed, DigiKey |
| Pimoroni Pico Plus 2 | about 53 x 21 | $14.95 | Out of stock at Adafruit | Adafruit (US), Pimoroni (UK) |
| Waveshare RP2350-Zero | not verified | $4.79 to $5.79 | Not shown | Waveshare (ships from China) |

Notes: the XIAO's connector type was not stated on the Seeed pages retrieved (third-party coverage calls it USB-C). The Pico Plus 2 keeps the Pico pin layout, which matters if a carrier board is planned.

Sources:
- https://datasheets.raspberrypi.com/pico/pico-datasheet.pdf
- https://datasheets.raspberrypi.com/pico/pico-2-datasheet.pdf
- https://www.sparkfun.com/sparkfun-pro-micro-rp2350.html
- https://www.adafruit.com/product/6000
- https://www.sparkfun.com/sparkfun-thing-plus-rp2350.html
- https://www.seeedstudio.com/Seeed-XIAO-RP2350-p-5944.html
- https://www.adafruit.com/product/6244
- https://www.waveshare.com/RP2350-Zero.htm

## 11. INA228 breakouts

**Verdict: CONFIRMED, with two caveats.**

| | Adafruit 5832 | SparkFun INA228 Qwiic |
|---|---|---|
| Bus voltage | up to 85 V DC, no AC | 0 to 85 V |
| Fitted shunt | 15 milliohm, 0.1% | 15 milliohm |
| Stated current | 10 A in the coarse range; 2.75 A continuous in the fine range | 10 A at the default range |
| External shunt | Not documented | Yes: open the SHUNT solder jumper and fit an external resistor on the through-hole pads |
| Price | $14.95, in stock | $14.95, in stock |

- The 10 A figure follows from the chip's wide input range (about 164 mV) across 15 milliohm. The narrow range (about 41 mV) gives finer resolution and a ceiling near 2.7 A.
- At 10 A a 15 milliohm shunt dissipates 1.5 W, so 10 A should be read as a measurement limit, not a comfortable continuous rating. Neither vendor page gives a thermal rating.
- SparkFun's shop page is internally inconsistent (one line says 10 milliohm); its hardware guide says 15 milliohm.
- On the Adafruit board an external shunt would mean removing the fitted resistor. Adafruit does not describe this.

Sources:
- https://www.adafruit.com/product/5832
- https://learn.adafruit.com/adafruit-ina228-i2c-power-monitor
- https://www.sparkfun.com/sparkfun-current-sensor-ina228-qwiic.html
- https://docs.sparkfun.com/SparkFun_Qwiic_Current_Sensor_INA2XX/hardware/hardware%20overview

## 12. I2C digital isolators and power

**Verdict: CONFIRMED.** The TI ISO1540 has separate supply pins for each side (3 V to 5.5 V each, each referenced to its own ground) and contains no power converter; TI suggests an external transformer driver for the isolated side. The ADuM1250 is the same arrangement (two supplies, 3.0 V to 5.5 V).

Sources:
- https://www.ti.com/lit/ds/symlink/iso1540.pdf
- https://www.analog.com/media/en/technical-documentation/data-sheets/ADUM1250_1251.pdf

## 13. RTL-SDR Blog V3 serial numbers

**Verdict: CONFIRMED, medium confidence on the exact default.**

- `rtl_eeprom` has an option to write a new serial string (`-s`), per the Debian manual page and the RTL-SDR Blog driver source.
- That source's stock EEPROM template for the generic Realtek identity, which the V3 presents, uses serial 00000001. This supports the claim but is not a vendor statement about what ships.
- A widely used ADS-B setup guide says most RTL-SDRs ship with identical serials and walks through re-serialising one dongle at a time, then replugging.
- The V3 user guide retrieved does not mention the shipped serial. It documents `rtl_eeprom` only for the bias-tee flag.
- Cautions from the vendor's quick-start guide: interrupting an EEPROM write can leave a dongle unrecognised (a recovery tool exists), and on the newer V4 model the manufacturer and product strings must not be changed.

Sources:
- https://manpages.debian.org/trixie/rtl-sdr/rtl_eeprom.1.en.html
- https://github.com/rtlsdrblog/rtl-sdr-blog (src/rtl_eeprom.c)
- https://sdr-enthusiasts.gitbook.io/ads-b/setting-up-rtl-sdrs/re-serialise-sdrs
- https://www.rtl-sdr.com/rtl-sdr-quick-start-guide/
- https://www.rtl-sdr.com/rtl-sdr-blog-v-3-dongles-user-guide/

## 14. RTL-SDR Blog V3 frequency range

**Verdict: PARTLY. The headline range is the vendor's own, but it joins two different receive paths.**

- The V3 datasheet states 500 kHz to 1766 MHz, with 500 kHz to 24 MHz in direct sampling mode. "1.7 GHz" is a slight understatement.
- The tuner chip only reaches down to about 24 MHz. Everything below that bypasses the tuner and feeds the converter directly.
- Direct sampling caveats, all from the vendor:
  - No gain control, so strong signals overload it easily; dynamic range is worse than an upconverter.
  - The converter samples at 28.8 MHz, so signals alias as mirror images around 14.4 MHz. Filters are advised.
  - A 24 MHz low-pass filter sits in the path; the datasheet puts the usable top of this mode at about 24 MHz, although the user guide quotes 28.8 MHz.
  - Newer batches attenuate below about 2 MHz to protect against AM broadcast overload.
  - It must be switched on in software (Q-branch), automatic only with the vendor's driver fork.
  - A VHF antenna performs poorly at HF; a long wire with a matching transformer is recommended.
- Instantaneous bandwidth is at most about 2.4 MHz stable, wherever it is tuned.
- Top end: ordinary dongles lose lock above about 1.5 GHz when hot. The V3 addresses this with a thermal pad to its metal case.

Accurate wording: tuner coverage about 24 MHz to 1.766 GHz, plus a limited HF mode from about 500 kHz to 24 MHz.

Sources:
- https://www.rtl-sdr.com/wp-content/uploads/2018/02/RTL-SDR-Blog-V3-Datasheet.pdf
- https://www.rtl-sdr.com/rtl-sdr-blog-v-3-dongles-user-guide/
- https://www.rtl-sdr.com/buy-rtl-sdr-dvb-t-dongles/

## 15. LiFePO4 temperature and voltage window

**Verdict: CONFIRMED as typical, not universal.**

- Power Sonic 26650 cell: 3.2 V nominal, charge to 3.65 V (3.7 V absolute maximum), recommended discharge cut-off 2.5 V (protection must act by 2.0 V), charge 0 to 45 C, discharge -20 to 60 C.
- Hithium 280 Ah prismatic cell: same 2.5 V to 3.65 V window above freezing, but charging allowed from 0 to 60 C.
- Four cells in series therefore span 10.0 V to 14.6 V (4 x 2.5 V to 4 x 3.65 V), 12.8 V nominal. Anything fed from the pack must accept that whole window.
- The lower charging limit of 0 C is the more consistent figure across datasheets and matters as much as the upper one for a case left in a cold vehicle.
- The 45 C charge ceiling is plausible for a closed case in hot sun, so the specific cell's datasheet should set the charge-inhibit threshold.

Sources:
- https://cdn.power-sonic.com/documents/PSL-FP-IFR26650PC-Data-Sheet.pdf
- https://cdn.shopify.com/s/files/1/0603/8283/8841/files/HITHIUM_280Ah_LiFePO4_Cell_Product_Specification.pdf

## 16. PPS on cheap GNSS boards

**Verdict: PARTLY. It depends on the board variant, so it must be checked per listing.**

- The common 4-pin NEO-6M board (sold as GY-NEO6MV2 or GY-GPS6MV2: power, ground, transmit, receive) does not bring the time pulse to a pin. A reseller's support thread confirms a wire must be soldered to the module's pin 3 to reach the pulse. (That the pulse drives only the LED on this variant is the usual description but was not in the thread.)
- Some 5-pin NEO-6M clones do expose PPS on the header as well as the LED; one US reseller's listing documents exactly that (that listing was sold out when checked).
- No primary source was found for NEO-M8N clone boards as a class.

Two boards that definitely break out PPS (25 Nov 2026):

| Board | PPS | Price | Stock |
|-------|-----|-------|-------|
| Adafruit Ultimate GPS Breakout (product 746) | Dedicated PPS pin; 3.3 V pulse once a second | $29.95 | 52 in stock |
| SparkFun GPS Breakout NEO-M9N, SMA (Qwiic) | PPS pin broken out alongside reset, safeboot and interrupt; stated time-pulse accuracy 30 ns | $76.50 | Page shows both "in stock" and "backorder" |

Sources:
- https://forum.core-electronics.com.au/t/u-blox-neo-6m-gps-module-018-gy-gps6mv2-comments/173
- https://protosupplies.com/product/neo-6m-gps-module/
- https://www.adafruit.com/product/746
- https://learn.adafruit.com/adafruit-ultimate-gps/pinouts
- https://www.sparkfun.com/sparkfun-gps-breakout-neo-m9n-sma-qwiic.html
- https://learn.sparkfun.com/tutorials/sparkfun-gps-neo-m9n-hookup-guide/all

---

## Could not verify

1. **Which Pi 5 protections header power bypasses (claim 1).** Tried: the white paper (silent), the hardware documentation (general statements only), a Pi 5 schematic address (404), and two Raspberry Pi forum threads (access blocked for automated fetch: forums.raspberrypi.com topics 382946 and 395698).
2. **Header power at the same time as a laptop on USB-C in gadget mode (claim 3).** The gadget-mode article does not address it. Only the in-line powered hub method is documented.
3. **The "J5" label for the RTC battery connector (claim 4).** The fact is confirmed; the designator was not in the retrieved text.
4. **Noise from a USB 3.0 hub on a USB 2.0 uplink, and SuperSpeed link behaviour when only USB 2.0 devices are attached (claim 7).** Intel's paper does not test either case. The USB 3 specification itself was not opened. Conclusions are inference.
5. **uhubctl behaviour of the Plugable USB2-HUB7BC (claim 6).** Not listed either way. A second fetch of the Plugable page was rate-limited (HTTP 429), so its discontinued status rests on one read.
6. **US availability and current hardware revision of the D-Link DUB-H7, and availability of the Microchip EVB-USB2517 (claim 6).**
7. **A user-accessible GPIO pin on the PiKVM V4 Mini (claim 8).** Documentation implies an internal pin on V4 boards but does not confirm it for the Mini.
8. **Shipped serial of RTL-SDR Blog V3 units from a vendor statement (claim 13).** Supported by driver source and community guides only.
9. **Waveshare RP2350-Zero dimensions and US stock; Seeed XIAO RP2350 connector type and DigiKey quantity (claim 10).** Dimension drawings are images; a US reseller page failed to load.
10. **PPS breakout on NEO-M8N clone boards (claim 16).** No primary source found.
11. **Continuous current rating of either INA228 breakout at 10 A (claim 11).** Neither vendor gives a thermal figure.
