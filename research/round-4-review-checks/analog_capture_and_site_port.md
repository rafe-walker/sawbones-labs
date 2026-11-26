# Analog capture module and battery "site port": design research

Researched 25 November 2026. All prices are single-quantity USD seen on that date unless marked otherwise.
Items marked UNVERIFIED come from background knowledge and were not confirmed against a datasheet or vendor page in this pass.

**This is design research, not electrical safety advice. Fitting any tap to a battery busbar is work for a qualified person, done with the bank isolated, and nothing here claims compliance with IEC 61010, UL or any electrical code.**

---

## Summary

### Part 1: analog capture. Verdict: the two-band suspicion is right, with one caveat

- No DC-accurate, simultaneous, 4-channel part reaches 192 kHz per channel with 16 or more effective bits for under $25 in single quantity. The sub-$25 16-bit SAR parts that look like they qualify (TI ADS8584S, the AD7606 family) have a built-in analog low-pass at 16 to 24 kHz, so they cannot see the 18 to 60 kHz inverter band at all.
- The caveat: a single part does cover everything if $35 to $39 is acceptable. AD7768-4 (4 ch, 24-bit, 256 kSPS, 108 dB, about 110 kHz bandwidth, DC-coupled) was $35.06 at Mouser; ADS127L14 (4 ch, 512 kSPS) was $38.53. The reviewers' one-module idea is not impossible, just not cheap, and it forces high-speed USB.
- The real constraint is USB, not the ADC. 4 ch x 192 kHz x 24 bit is 18.4 Mbit/s of payload. The RP2350 has full-speed USB only (12 Mbit/s raw, roughly 8 Mbit/s usable), so that stream cannot leave an RP2350 module live under any firmware.

**Recommended design: two modules, split by band.**

1. **LF/DC module (build first):** RP2350 + TI ADS131M04 (4 ch simultaneous, 24-bit, DC-coupled, 4 to 32 kSPS, $6.84), 1 Mohm attenuated +/-50 V inputs, isolated as a whole at the USB port with a full-speed isolator plus a small isolated DC-DC. Data is 48 to 384 kB/s, which fits full-speed USB on a plain bulk endpoint in embassy-usb. Covers cranking voltage and current, pickup coil, vibration to a few kHz and mains harmonics to the 50th. Estimated parts: **$55 to $70**.
2. **AC band, 10 Hz to about 90 kHz:** buy first, build later. A Behringer UMC202HD ($86.90) on the Pi is a working 2-channel 24-bit/192 kHz capture today through ALSA, with an external attenuator. The later module is a PCM1865 or AK5574 audio ADC ($7 to $15) either wired to the Pi 5 I2S pins through a digital isolator, or behind a high-speed USB bridge. Estimated parts for the built version: **$60 to $90**.

"Switchable AC/DC coupling" then mostly disappears as a relay problem: the DC-coupled band is module 1, the AC-coupled band is module 2.

### Part 2: site port. Verdict: put the protective impedance at the busbar, on both poles, and keep the full bus voltage off the connector

**Recommended design:** a small potted tap board bolted at the DC bus with two high-voltage film resistors in series in each of the positive and negative legs (single-fault tolerant), a bottom resistor that makes it a fixed divider (about 50:1, so the lead and connector carry about 1.1 V, not 58 V), an M12 A-coded female panel receptacle, and a 1-Wire EEPROM (DS28E07 or DS2431) holding the port identity, divider ratio and calibration factor. The instrument measures differentially or floats. Estimated parts: **$20 to $25** plus a cordset.

Step-up option where ripple fidelity and full galvanic isolation matter: add a TI AMC3330 isolated amplifier on the tap board, powered from the instrument side. Estimated parts **$35 to $45** on a custom board; the TI evaluation module is about $65.

A fuse alone is the weaker answer. Small fuses with a real DC interrupt rating exist (10x38 mm midget types at 10 kA DC), but a resistor at the lug removes the fault energy instead of interrupting it.

---

## 1. Candidate ADC parts

DK = DigiKey, M = Mouser. "DC?" means suitable for accurate DC, not merely passing it.

| Part | Ch | Sampling | Bits | Max rate / ch | DC? | Interface | Price (25 Nov 2026) | Cheap US breakout? |
|---|---|---|---|---|---|---|---|---|
| TI ADS131M04 | 4 | simultaneous delta-sigma | 24 | 32 kSPS (DK listing); TI table says 64 kSPS | Yes; inputs accept below-ground signals (-1.3 V) | SPI | $6.84 M | Not found from a US seller (UNVERIFIED); TI EVM only |
| TI ADS131M08 | 8 | simultaneous | 24 | 32 kSPS | Yes | SPI | $12.69 DK / $12.81 M | As above |
| TI ADS131E08 | 8 | simultaneous | 24 (16 at top rate, UNVERIFIED) | 64 kSPS (UNVERIFIED) | Yes | SPI | $20.18 DK / $19.81 M | No |
| TI ADS131A04 | 4 | simultaneous | 24 | 128 kSPS | Yes | SPI | $16.25 DK / $14.76 M | No; TI EVM |
| TI ADS127L11 | 1 | n/a | 24 | 400 kSPS wideband (UNVERIFIED) | Yes | SPI | $19.18 DK / $19.44 M | No |
| TI ADS127L14 (added) | 4 | simultaneous | 24 | 512 kSPS wideband, 1365 kSPS low-latency | Yes; 112 dB DR at 200 kSPS | SPI config + frame-sync data port | $38.53 M | No |
| TI ADS8684 | 4 | multiplexed SAR | 16 | 500 kSPS total, so 125 kSPS/ch | Yes, +/-10 V inputs | SPI | $19.06 DK / $19.05 M | No |
| TI ADS8688 | 8 | multiplexed SAR | 16 | 500 kSPS total (UNVERIFIED) | Yes | SPI | $23.30 M | No |
| TI ADS8584S (added) | 4 | simultaneous SAR | 16 | 330 kSPS | Yes, +/-10 V, 1 Mohm, clamp | SPI / parallel | $23.53 to $26.14 DK, $25.28 M | No |
| TI ADS8588S (added) | 8 | simultaneous SAR | 16 | 200 kSPS | Yes | SPI / parallel | $24.65 (reel-cut) to $38.10 | No |
| ADI AD7606 | 8 | simultaneous SAR | 16 | 200 kSPS | Yes, +/-10 / +/-5 V, 1 Mohm | SPI / parallel | $52.01 M (AD7606B $56.52 M) | Modules: $40 on Tindie from a Shenzhen seller; commonly $20 to $35 from importers on Amazon/eBay (UNVERIFIED) |
| ADI AD7768-4 | 4 | simultaneous delta-sigma | 24 | 256 kSPS, 110.8 kHz BW, 108 dB | Yes | serial DCLK/DRDY + 4 data lines (UNVERIFIED) | $35.06 M (ADI 1ku $16.74) | No; FMC eval board |
| ADI AD7771 | 8 | simultaneous | 24 | 128 kSPS (UNVERIFIED) | Yes | SPI / data port | $29.06 ADI/Newark list, no stock; not listed at DK/M on Octopart | No |
| ADI AD7134 (added) | 4 | simultaneous | 24 | 1.5 MSPS | Yes | serial | not listed at DK/M on Octopart; $23 to $28 at brokers | No |
| ADI AD7380-4 (added) | 4 | simultaneous SAR | 16 | 4 MSPS | Yes | SPI | $58.13 DK / $44.66 M | No |
| Cirrus CS5381 | 2 | simultaneous | 24 | 216 kHz | Audio class | I2S | not at DK/M on Octopart; $5 to $22 at brokers | No |
| Cirrus CS5368 | 8 | simultaneous | 24 | 192 kHz class (UNVERIFIED) | Audio class | I2S/TDM | not at DK/M on Octopart; $15 to $28 at brokers | No |
| AKM AK5572 | 2 | simultaneous | 32-bit word | 768 kHz | Audio class | I2S/TDM/DSD | $8.72 DK and M | No |
| AKM AK5574 | 4 | simultaneous | 32-bit word | 768 kHz | Audio class | I2S/TDM | $14.70 DK / $14.39 M | No |
| TI PCM1808 | 2 | simultaneous | 24 | 96 kHz (UNVERIFIED) | No: fixed high-pass (UNVERIFIED) | I2S | $1.59 DK and M | Yes, $5 to $10 generic modules on Amazon (UNVERIFIED) |
| TI PCM1865 | 4 | simultaneous | 24 | 192 kHz, 110 dB SNR | Audio class | I2S/TDM, I2C/SPI control | $7.28 DK / $6.98 M (reel-cut); $12.81 tube | PCM1863 is on the HiFiBerry DAC+ ADC Pro Pi HAT (UNVERIFIED) |
| TI PCM4204 | 4 | simultaneous | 24 | 216 kHz | Audio class | I2S/TDM | $18.85 to $22.93 DK | No |
| ESS ES9822 | 2 | simultaneous | 32-bit word | 192 kHz+ (UNVERIFIED) | Audio class | I2S | Not stocked at DK/M (UNVERIFIED) | No |
| Microchip MCP3564 | 4 diff / 8 SE | multiplexed | 24 | 153.6 kSPS total | Yes | SPI | $5.44 M (MCP3564R UQFN) | Breakouts exist (UNVERIFIED) |

Notes that change the ranking:

- **Built-in filters on the +/-10 V SAR parts.** The ADS8584S datasheet gives the input low-pass as 24 kHz (+/-10 V range) and 16 kHz (+/-5 V range), with 92.7 dB SNR without oversampling, about 15 effective bits. The AD7606 has the same architecture (second-order filter; cutoff near 22 kHz is UNVERIFIED). They are excellent sub-20 kHz DC parts with a free 1 Mohm protected front end, and useless for the 18 to 60 kHz band.
- **AD7606 modules cost less than the chip.** A genuine AD7606 is $52 in single quantity; modules sell for $20 to $40. Treat module chips as unknown provenance.
- **"Audio class" and DC.** Most of these converters have a digital high-pass that can be switched off (UNVERIFIED per part: PCM186x by register, PCM4204 and AK557x by pin). With it off and a DC-coupled driver they pass DC, but offset, drift and gain are not specified for DC work. Good for waveform shape, not for absolute volts.
- **Multiplexed parts** (ADS8684/8688, MCP3564) lose phase alignment between voltage and current channels; avoid for cranking V/I and power quality.

Sources: https://www.ti.com/product/ADS131M04 , https://www.ti.com/product/ADS131A04 , https://www.ti.com/product/ADS127L14 , https://www.ti.com/product/ADS8584S , https://www.ti.com/lit/ds/symlink/ads8584s.pdf , https://www.analog.com/en/products/ad7768-4.html , https://www.analog.com/en/products/ad7606.html , https://www.ti.com/product/PCM1865 , https://www.tindie.com/products/exlene/ad7606-data-acquisition-module/ , Octopart part pages (https://octopart.com/part/texas-instruments/ADS131M04IPWR and equivalents for each part).

## 2. Is there a DC-coupled, simultaneous, 4-channel, 192 kHz, 16-effective-bit part under $25?

**No.** Plainly:

| Requirement | ADS8584S ($23.53+) | ADS131A04 ($14.76) | PCM4204 / AK5574 ($14 to $23) | AD7768-4 ($35.06) | ADS127L14 ($38.53) |
|---|---|---|---|---|---|
| 4 ch simultaneous | yes | yes | yes | yes | yes |
| 192 kSPS per channel | yes (330k) | no (128k) | yes | yes (256k) | yes (512k) |
| Bandwidth to 60 to 100 kHz | **no, 24 kHz filter** | no | yes | yes (110 kHz) | yes |
| 16 effective bits | no (about 15) | at low rates only | yes (AC) | yes | yes |
| DC accuracy specified | yes | yes | **no** | yes | yes |
| Under $25 single qty | borderline | yes | yes | no | no |

The cheapest part that meets every technical line is the AD7768-4 at about $35 (it would be $16.74 at 1,000 pieces).

**Best two-band design**

| | LF/DC band | AC band |
|---|---|---|
| Converter | ADS131M04, $6.84 | PCM1865 ($7 to $13) or AK5574 ($14.50) |
| Range | DC to about 4 kHz at 16 kSPS; 8 kHz at 32 kSPS (sinc-filter -3 dB points, UNVERIFIED) | about 5 Hz to 90 kHz at 192 kHz |
| Coupling | DC | AC (series capacitor), gain stage for small ripple on a large DC level |
| Data rate, 4 ch | 192 kB/s at 16 kSPS | 2.3 MB/s |
| Transport | RP2350 full-speed USB, bulk endpoint | Pi 5 I2S direct, or high-speed USB bridge |
| Covers | cranking V/I, pickup coil, vibration, mains harmonics | inverter switching noise, ripple, acoustic/ultrasonic |

Why the split is still worth it even though one $35 chip could do both: the single-chip route needs high-speed USB, four precision differential drivers and a larger board before it measures anything, while the LF module is a small, cheap, full-speed RP2350 design that covers most of the listed jobs by itself. The AC band can be filled today with a bought audio interface.

Middle option worth knowing: the ADS131A04 at 128 kSPS is a DC-accurate single chip to roughly 30 to 50 kHz, but 4 ch x 128 kSPS x 24 bit is 1.5 MB/s, still over what full-speed USB carries.

## 3. Getting 4 x 192 kHz x 24 bit to the Pi

Arithmetic: 4 x 192,000 x 3 bytes = 2.30 MB/s = 18.4 Mbit/s payload.

| Link | Ceiling | Result |
|---|---|---|
| USB full speed, isochronous | 1023 bytes per 1 ms frame, about 1.0 MB/s (UNVERIFIED figure, USB 2.0 spec) | Does not fit. 2 ch x 96 kHz x 24 bit (576 B/ms) fits; 2 ch x 192 kHz x 24 bit (1152 B/ms) does not; 2 ch x 192 kHz x 16 bit (768 B/ms) fits |
| USB full speed, bulk | about 1.0 to 1.2 MB/s best case (UNVERIFIED) | Does not fit |
| USB high speed | 480 Mbit/s | Fits with large margin |

**RP2350 has no high-speed USB.** The product brief lists one USB 1.1 controller and PHY: full-speed device, full/low-speed host. (HSTX is a video-style output, not USB.)

Options:

| Option | How | For | Against |
|---|---|---|---|
| Stay on RP2350, reduce the stream | 2 ch at 96 kHz, or burst capture into RAM/PSRAM then upload | No new parts; fine for triggered captures | Not continuous at full spec; about 0.2 s of 4-ch burst in internal RAM (estimate) |
| FT232H / FT2232H in synchronous FIFO mode, fed by RP2350 PIO | Byte-wide FIFO to a high-speed FTDI chip (tens of MB/s, UNVERIFIED) | Keeps RP2350 and Rust; proven pattern (Haasoscope Pro uses an FTDI link) | Not class-compliant; host needs libftdi/D2XX-style access |
| Cypress FX2LP slave FIFO | Same idea, the chip sigrok's fx2lafw targets | Open firmware ecosystem | 8051 firmware to maintain |
| A high-speed-capable MCU | STM32 with HS PHY, i.MX RT (Teensy 4), CH32V305, ESP32-P4 (all UNVERIFIED for Embassy maturity) | Clean single-chip USB Audio Class 2 device | Breaks the "RP2350 everywhere" rule |
| ADC straight to the Pi 5 over I2S | Audio ADC as an ALSA capture device | No USB bottleneck, no firmware; HiFiBerry ships an 8-input 192 kHz/24-bit add-on for the Pi 5, proof that the Pi 5 takes multi-channel I2S capture | Not a plug-in USB module; isolation must be done on the I2S lines; that add-on only works stacked on their DAC8x |
| ADC straight to the Pi over SPI | ADS131M04 on spidev | Simple wiring | No sample FIFO in the ADC (UNVERIFIED), so Linux scheduling latency drops samples at higher rates; use a microcontroller in between |

**USB Audio Class on RP2350 in Rust:** embassy-usb has a UAC1 speaker class, and a UAC1 device-to-host "source" class with an isochronous IN endpoint and feedback endpoint was merged on 21 April 2026 (PR #5600), with an STM32F1 example. So it is newly possible, young, and UAC1 only. No RP example was confirmed, and embassy-rp's isochronous endpoint handling could not be checked in this pass. Even if it works perfectly it is capped by full speed (2 ch x 96 kHz x 24 bit). **Verdict: use a vendor bulk endpoint for the LF module now (mature in embassy-usb, and the agent reads it with a small Rust host crate); treat UAC1 capture as an experiment, not a dependency.** TinyUSB in C has had UAC2 device examples for longer (UNVERIFIED).

Sources: https://pip-assets.raspberrypi.com/categories/1214-rp2350/documents/RP-008374-DS-1-rp2350-product-brief.pdf , https://www.raspberrypi.com/documentation/microcontrollers/silicon.html , https://docs.embassy.dev/embassy-usb/git/default/class/uac1/index.html , https://github.com/embassy-rs/embassy/pull/5600 , https://www.hifiberry.com/docs/data-sheets/datasheet-adc8x-add-on/ , https://github.com/drandyhaas/HaasoscopePro

## 4. Isolation

What an isolated front end needs: (a) power across the barrier, (b) data across the barrier, (c) creepage on the board, (d) a decision about what "isolated" means between channels.

| Function | Part | Notes | Price |
|---|---|---|---|
| USB isolator, full/low speed | TI ISOUSB111 | Fits an RP2350 module | not listed at DK/M on Octopart; $3.84 to $8.36 at other sellers |
| USB isolator, full/low speed | ADI ADuM3160 / ADuM4160 | The classic part; many $10 to $20 dongles use it (UNVERIFIED) | UNVERIFIED |
| USB isolator, high speed | ADI ADuM3165 | 480 Mbit/s, 3.75 kV rms | ADI 1ku $5.70; $7.55 at Richardson/Arrow; eval board $74.27 M |
| USB isolator, high speed | TI ISOUSB211 | 480 Mbit/s | $15.15 DK; EVM $70.00 M / $70.60 DK |
| Digital isolator, 4 ch | TI ISO7741 | 100 Mbit/s class; easily carries a 12.3 MHz I2S bit clock or 8 to 25 MHz SPI | $3.56 DK and M |
| Isolator + DC-DC in one | TI ISOW7741 | 4 ch at 100 Mbit/s plus up to 0.55 W isolated power (110 mA at 5 V to 5 V) | not at DK/M on Octopart; $4.80 to $6.58 elsewhere |
| Isolated DC-DC, chip scale | ADI ADuM5020 | About 0.5 W (UNVERIFIED), very high internal switching frequency, out of band | $7.32 M |
| Isolated DC-DC, module | B0505S-1WR3 | 5 V to 5 V, 1 W, unregulated; **switches near 100 kHz, inside the measurement band** | $0.42 to $0.69 at LCSC-type sellers |

**Which is simpler? Isolating the whole module at USB.** One isolator chip and one DC-DC, no timing budget on SPI or I2S, the microcontroller floats with the ADC, and firmware does not change. For the full-speed LF module that is about $5 to $12 of parts. Isolating at the ADC instead needs six or seven lines across the barrier for an ADS131M04 (SCLK, DIN, DOUT, CS, DRDY, CLKIN, RESET), which means two isolator packages, plus the same DC-DC; its one advantage is that the microcontroller's noise stays off the analog ground.

For the AC band the choice flips with the transport: a high-speed USB isolator ($7.50 to $15) for a USB module, or one ISO7741 on the I2S lines for a Pi-attached ADC, with the audio master clock generated on the isolated side so jitter does not cross the barrier (design judgement, UNVERIFIED).

Three cautions:

1. **Filter the isolated supply.** A B0505S-style converter puts its ripple at about 100 kHz, exactly where this module measures. Follow it with an LC filter and LDO, or use a converter that switches far above the band.
2. **Group isolation is not channel isolation.** A 4-channel converter has one ground. Battery negative, engine block and a transformer secondary are three different references. Per-channel differential attenuators (high common-mode difference amplifiers such as the AD8479/INA149 class, UNVERIFIED) or per-channel isolated amplifiers (section 10) are needed for that; "isolated front end" alone does not deliver it.
3. **Bought audio interfaces need a high-speed isolator.** Full-speed-only isolator dongles will not carry a 192 kHz class-2 audio device (UNVERIFIED for the specific interfaces below).

Sources: https://www.analog.com/en/products/adum3165.html , https://www.ti.com/product/ISOW7741 , Octopart pages for ISOUSB211, ISOUSB111, ISO7741DWR, ADUM5020, B0505S-1WR3, ISOW7741DFMR.

## 5. Input front end for +/-50 V with AC/DC coupling

Sketch, one channel (LF/DC band, ADS131M04):

```
IN+ o--[R1a 490k]--[R1b 490k]--+-----+-----------+----o to ADC AINxP (or buffer)
                               |     |           |
                            [R2 20k] [C2 3.3n]  BAV199 clamps to +/- rails
                               |     |           |
IN- o--------------------------+-----+-----------+----o AINxN / AGND
```

- **Attenuator:** 980 kohm over 20 kohm gives 50:1, so +/-50 V becomes +/-1.0 V against the ADS131M04's +/-1.2 V full scale at gain 1. Input resistance is 1 Mohm, the scope standard, so ordinary x10 probes extend the range to +/-500 V.
- **Two series resistors on top** share voltage (two 1206 or MELF parts) and give single-fault tolerance. With 1 Mohm in series, a 400 V ignition-primary spike drives only 0.4 mA into the clamps, so no TVS or gas tube is needed for survival.
- **Clamp:** low-leakage dual diode (BAV199 class) to the rails after the divider.
- **Anti-alias filter:** for a delta-sigma converter one RC pole is enough because the digital filter does the steep part; R2 with 3.3 nF gives about 2.5 kHz. Choose C2 to suit the output data rate.
- **Buffer or calibrate:** the divider's 20 kohm source resistance works against the ADC's own input impedance (a few hundred kohm at low gain, UNVERIFIED), a gain error of several percent that is stable and removable by calibration. For better than about 0.5%, add a CMOS/JFET-input buffer per channel; that needs a small negative rail for bipolar signals.
- **AC/DC coupling:** a 100 nF film or C0G capacitor rated 250 V or more ahead of the divider gives a 1.6 Hz corner against 1 Mohm; a small signal relay shorts it for DC. Mechanical relays are preferred over solid-state ones here because of leakage and off-capacitance (UNVERIFIED generalisation). In the two-band design this relay is optional: the AC band has its own capacitor-coupled path with switchable gain (x1/x10/x100), which is what AC coupling is really for: seeing 50 mV of ripple on a 58 V bus.
- **AC band driver:** audio converters want a differential drive with a second-order filter near 100 to 150 kHz from a fully differential amplifier (THS4551/OPA1632 class, UNVERIFIED).
- **If an ADS8584S/AD7606 is used instead:** the 1 Mohm input, clamp (the ADS8584S tolerates +/-15 V before its clamp conducts) and filter are inside the chip; only an external 5:1 divider is needed for +/-50 V, and the chip's 1 Mohm then becomes part of the divider.

Typical input impedance to quote for the module: **1 Mohm in parallel with about 15 to 25 pF**.

## 6. Buy-instead stand-ins

| Device | Price (25 Nov 2026) | What it gives | Driver openness | What it cannot do |
|---|---|---|---|---|
| Behringer UMC202HD | $86.90, Sweetwater | 2 inputs, 24-bit/192 kHz | USB audio class on Linux/ALSA (UNVERIFIED) | No DC; uncalibrated gain knobs (needs a reference tone); mic/line/instrument-level inputs, so external attenuator for 50 V; not isolated |
| Behringer UMC204HD | $97.98, Sweetwater | **Still 2 inputs** (2-in/4-out) | same | Same limits; does not add capture channels. Four inputs needs the UMC404HD (price UNVERIFIED) |
| Focusrite Scarlett 2i2 4th gen | $189.99 sale, $224.99 regular, Focusrite US store | 2 simultaneous channels, 24-bit/192 kHz | USB 2.0 audio; class compliance not stated on the page | Same limits, at twice the price |
| Digilent Analog Discovery 3 | $379.00, Digilent | 2 differential channels, 14-bit, 125 MS/s, +/-25 V, plus generator and logic | WaveForms SDK: free, scriptable (C/Python), not open source | 14-bit only; not isolated; +/-25 V; limited continuous streaming (UNVERIFIED) |
| ADALM2000 | $274.94, DigiKey (719 in stock) | 2 ch, 12-bit, 100 MS/s | Scopy on Linux; libm2k/libiio open source (UNVERIFIED) | 12-bit; not isolated |
| PicoScope 2204A | $189.00 Newark listing, zero stock | 2 ch, 10 MHz, 8-bit (UNVERIFIED), generator | Vendor software runs on Linux and Raspberry Pi OS; SDK is closed | 8-bit; closed driver; about +/-20 V |
| Hantek 6022BE | about $60 to $75 (UNVERIFIED; no price page readable) | 2 ch, 8-bit, 48 MS/s, 20 MHz | **sigrok `hantek-6xxx` driver with open firmware** | 8-bit; no hardware trigger; +/-5 V without a x10 probe; not isolated |
| MCC 172 Pi HAT | $399.00, Digilent | 2 ch IEPE, 24-bit, 51.2 kS/s | Open C/Python library on GitHub | AC-coupled only; +/-5 V; vibration/acoustic use only |
| ADI EVAL-CN0540-ARDZ | $223.33, ADI | 1 ch, 24-bit, DC-coupled IEPE, 54 kHz analog bandwidth | Linux and no-OS drivers published | One channel; Arduino-format shield, not USB |
| AD7606 module + any MCU | $20 to $40 | 8 ch simultaneous 16-bit, +/-10 V, 1 Mohm | SPI, trivial | 22 to 24 kHz internal filter; unknown chip provenance; needs firmware |

Best stand-in pairing: **UMC202HD for the AC band now**, and an **AD7606 module on an RP2350** (or straight to the ADS131M04 module) for the DC band. The Hantek is the only cheap scope with a fully open driver path, but 8 bits is too coarse for ripple-on-DC work.

Sources: https://www.sweetwater.com/store/detail/UMC202HD--behringer-u-phoria-umc202hd-usb-audio-interface , https://www.sweetwater.com/store/detail/UMC204HD--behringer-u-phoria-umc204hd-usb-audio-interface , https://us.focusrite.com/products/scarlett-2i2 , https://digilent.com/shop/analog-discovery-3/ , https://www.digikey.com/en/products/detail/analog-devices-inc/ADALM2000/ADALM2000-ND/7019661 , https://octopart.com/search?q=PP906 , https://sigrok.org/wiki/Hantek_6022BE , https://digilent.com/shop/mcc-172-iepe-measurement-daq-hat-for-raspberry-pi/ , https://www.analog.com/en/resources/reference-designs/circuits-from-the-lab/cn0540.html

## 7. Open-hardware designs to borrow from

| Design | What is useful | Files / licence |
|---|---|---|
| EEZ DIB MIO168 (+ AFE boards) | Closest relative: a mixed I/O instrument module with four simultaneously captured analog inputs added by an AFE expansion board, on-board isolated supply. Ranging and protection ideas | **KiCad v7** project; TAPR open hardware licence; OSHWA certified. ADC part not confirmed here |
| FreeEEG32 | Multi-chip simultaneous delta-sigma capture into an STM32H7 over USB (uses AD7771, UNVERIFIED) | **KiCad**; AGPL-3.0 |
| OpenBCI Cyton | ADS1299 analog front end + PIC32; SPI, DRDY handling and layout of a TI simultaneous delta-sigma | .sch/.pcb files in the OpenBCI V3 hardware repository; format and licence not confirmed |
| ADI CN0540 | Reference-grade DC-coupled driver for the AD7768 family with IEPE current source | Schematic, BOM, Gerbers, Allegro layout from ADI |
| ThunderScope | 4-channel scope front end (attenuator, coupling, protection) | MIT; file format not confirmed (KiCad UNVERIFIED) |
| Haasoscope Pro | High-rate streaming over an FTDI high-speed link | MIT; **Eagle 9.6.2**, not KiCad |
| sigrok fx2lafw + Hantek 6022BE | Open FX2 high-speed streaming firmware and host driver | sigrok project |
| "openEMS" | This name is an electromagnetic field solver and an energy-management software project, not a DAQ board (UNVERIFIED). The reviewer may have meant OpenEnergyMonitor's emonTx/emonPi open hardware (UNVERIFIED) | n/a |

No published KiCad USB DAQ built specifically on the ADS131M04 or AD7606 was confirmed in this pass.

Sources: https://github.com/eez-open/dib-mio168 , https://github.com/eez-open/dib-mio-afe3 , https://certification.oshwa.org/hr000008.html , https://github.com/neuroidss/FreeEEG32-beta , https://docs.openbci.com/Cyton/CytonSpecs/ , https://github.com/EEVengers/ThunderScope , https://github.com/drandyhaas/HaasoscopePro

---

## 8. Accepted practice for sense leads, and fuses with real DC interrupt ratings

Practice found:

- **Victron, Wiring Unlimited:** lithium banks can deliver far higher short-circuit current than lead-acid, and every lithium installation needs at least one fuse whose interrupt rating is at or above the bank's prospective short-circuit current. It also warns that many DC fuses suit 12 and 24 V but not necessarily 48 V. Voltage-sense wires go directly to the battery terminals in 0.75 mm2 cable.
- **Victron SmartShunt:** ships with two 1.5 m red supply/aux leads, each with a fuse built into the lead. The fuse value is not stated on the installation page. Community guidance is to place such a fuse as close to the battery as reasonably possible, because the hazard is at the battery end.
- **Shunt monitors generally:** the shunt sits in the negative conductor and the only positive connection is that thin fused lead. This is the existing, accepted version of a "sense tap".
- **BMS sense harnesses:** thin wires, often protected by a small fuse or series resistor at the cell end in better designs (UNVERIFIED generalisation).

Interrupt ratings:

| Fuse | Voltage rating | DC interrupt rating | Comment |
|---|---|---|---|
| Class T (Blue Sea A3T) | 125 V DC | 20,000 A | The usual main fuse for lithium banks |
| Class T (Eaton Bussmann, per Victron's table) | 160 V DC | 200 kA | |
| NH blade | 250 V | 25 kA | |
| Blue Sea ANL (per Victron's table) | 80 V | 6,000 A | Many ANL types are only 32 V |
| Littelfuse MEGA (per Victron's table) | 70 V / 58 V | 2,500 A / 1,000 A | |
| MRBF terminal fuse (Blue Sea) | 58 V DC max | 10,000 A at 14 V, 5,000 A at 32 V, **2,000 A at 58 V** | Lowest rating is 30 A, far too big to protect a sense lead, and 58 V is at the bank's top-of-charge voltage |
| Littelfuse KLKD, 10x38 mm midget | 600 V AC/DC, 0.1 to 30 A | **10 kA DC** (UL 248-19), 50 kA DC (UL 248-14) | The right class of small fuse for a sense lead, in a touch-safe holder |
| Bussmann DMM-B-44/100 | 1000 V AC/DC, 0.44 A | 10 kA | Multimeter fuse; shows what instrument makers fit |
| 5x20 mm glass | commonly 250 V AC | tens of amps to about 1.5 kA, often no DC rating (UNVERIFIED) | Not suitable at a battery bank |

Scale of the problem (estimate, UNVERIFIED): a 16-cell bank of 280 Ah cells at roughly 0.25 milliohm each is about 4 milliohm; with busbars and BMS perhaps 5 to 10 milliohm, so 5 to 10 kA prospective. That is above the MRBF's 2 kA at 58 V and inside a midget fuse's 10 kA.

Sources: https://www.victronenergy.com/media/pg/The_Wiring_Unlimited_book/en/dc-wiring.html , https://www.victronenergy.com/media/pg/SmartShunt/en/installation.html , https://communityarchive.victronenergy.com/questions/177506/victron-smart-shunt-fuse-location.html , https://bluesea.com/products/5178 , https://www.bluesea.com/products/5117 , https://www.mouser.com/datasheet/2/240/KLKD_Datasheet-3311512.pdf , https://elcodis.com/parts/3386339/DMM-B-44-100.html

## 9. Series resistor at the busbar tap

Why it works: the resistor does not have to interrupt anything. At 60 V a dead short on the lead draws 0.6 mA through 100 kohm (36 mW) or 60 microamps through 1 Mohm (3.6 mW).

Properties that matter, in order:

1. **Working (limiting element) voltage** far above the bus, to survive surges on the DC bus. Vishay VR25 is rated 1600 V, VR37 3500 V.
2. **Failure mode and single-fault tolerance.** Use two resistors in series per leg, each rated for full voltage and each alone keeping the current harmless. This mirrors the protective-impedance idea in section 11.
3. **Power in the worst fault** (lead shorted indefinitely): V squared over R. Trivial at 100 kohm and above; it becomes the design driver below about 10 kohm (360 mW at 10 kohm, 3.6 W at 1 kohm).
4. **Flammability.** The VR25 family datasheet reports passing both the cheesecloth and needle-flame tests, with UL 1676 and IEC 62368-1 recognition.
5. **Fusible resistors are the wrong tool at these values.** They are made in low values (UNVERIFIED: typically up to some kohm) and need real current to open. They only matter for a low-value, wide-band tap.
6. **Creepage and construction.** Resistor body directly at the lug so the unprotected conductor is a few millimetres; leads sleeved; board potted or boxed. On both the positive and negative taps, because a negative sense lead that touches the positive bus is the same fault.
7. **Temperature coefficient** if the resistor is a large fraction of the total: VR25 is up to +/-200 ppm/K, so use a 25 to 50 ppm metal film type where ratio accuracy matters.

Measurement error from a plain series resistor (reading = true x Rin / (Rs + Rin)):

| Series R | Into 1 Mohm input | Into 10 Mohm input | Bandwidth with about 220 pF of lead + input |
|---|---|---|---|
| 10 kohm | -0.99% | -0.10% | about 72 kHz |
| 100 kohm | -9.09% | -0.99% | about 7 kHz |
| 1 Mohm | -50.0% | -9.09% | about 0.7 kHz |

Two consequences. First, a plain 100 kohm to 1 Mohm series tap is a **DC and low-frequency port only**; it will hide the 18 to 60 kHz inverter ripple. Second, the error depends on whose input is plugged in.

Calibrating it out: store the series resistance (or better, a measured correction factor per instrument class) in the port's ID chip; the instrument multiplies by (Rs + Rin)/Rin using its own known Rin. Commission once by comparing against a reference meter at the busbar.

**Better arrangement: make the tap a complete divider.** Two 249 kohm resistors in each leg (996 kohm total) and a 20 kohm resistor across the output, on the tap board, about 51:1. The lead then carries about 1.1 V from a 20 kohm source: instrument loading error falls to -1.9% (1 Mohm) or -0.2% (10 Mohm), bandwidth with 220 pF rises to about 37 kHz, a few picofarads across the top legs (1 kV C0G) flattens it further in the way a x10 probe is compensated, and the connector never sees bus voltage. A lead shorted to the opposite pole passes about 0.12 mA, or 0.23 mA with one resistor failed short.

Example parts:

| Part | Value / rating | Price |
|---|---|---|
| Vishay VR25000001004JA100 | 1 Mohm, 5%, 0.25 W, 1.6 kV | GBP 0.27 single at Farnell UK (25 Nov 2026) |
| Vishay VR25, 1% variants | 100 kohm to 15 Mohm, 1.6 kV | similar (UNVERIFIED) |
| Vishay VR37 | 100 kohm to 33 Mohm, 0.5 W, 3.5 kV | UNVERIFIED |
| Low-tempco metal film, MELF or axial (Vishay MBB0207 / MRS25 class) | 25 to 50 ppm/K | UNVERIFIED |
| 2 W flameproof film (Vishay PR02 class) for a 2 x 4.7 kohm wide-band variant | survives the 0.4 to 0.8 W fault dissipation | UNVERIFIED |

Sources: https://www.vishay.com/docs/28907/vr25vr37vr68.pdf , https://uk.farnell.com/vishay/vr25000001004ja100/res-1m-5-250mw-axial-metal-film/dp/2614567

## 10. Isolated amplifiers as a permanently installed tap

| Part | Input | Gain error | Isolation | High-side supply | Price |
|---|---|---|---|---|---|
| TI AMC3330 | +/-1 V, high impedance | +/-0.2% max | reinforced, 4250 V rms / 6000 V peak, 1200 V rms working | **none needed: integrated DC-DC, single 3.3 or 5 V on the low side** | IC $3.11 to $4.93 at non-franchised sellers (not listed at DK/M on Octopart); AMC3330EVM $64.69 M / $67.38 DK |
| TI AMC1311 / AMC1311B | 0 to 2 V, high impedance | +/-1% / +/-0.2% | reinforced, 5000 V rms / 7000 V peak | needs a floating high-side supply | AMC1311EVM $67.27 M |
| TI ISO224 | +/-12 V | +/-0.3% (B grade) | reinforced, 5000 V rms | 4.5 to 18 V on the high side | UNVERIFIED |
| Broadcom ACPL-C87x | 0 to 2 V, optical | 0.5 to 3% by grade (UNVERIFIED) | reinforced (UNVERIFIED) | needs a high-side supply | UNVERIFIED |

How it fits: the tap board carries the resistive divider from section 9, with a 15 kohm bottom resistor so 58 V becomes about 0.86 V, feeding an AMC3330. The connector then carries only an isolated differential signal and a 5 V feed from the instrument. Advantages: the port is galvanically isolated, so the instrument needs no isolation of its own and no ground loop forms; nothing draws from the battery except the divider's 60 microamps or so; bandwidth is in the hundreds of kHz (UNVERIFIED), enough for inverter ripple. Costs: an amplifier's offset and noise (roughly 12 to 13 effective bits, UNVERIFIED) instead of a passive tap's, and an active part living permanently at the bus. The AMC3330 is the natural pick because the others need a second, floating supply derived from the battery.

Two non-amplifier alternatives: a digital monitor such as an INA228 behind an I2C isolator for slow, accurate V and I (UNVERIFIED), and simply reading the DC value from the shunt monitor or BMS that is already installed, which needs no new tap at all.

Sources: https://www.ti.com/product/AMC3330 , https://www.ti.com/product/AMC1311 , https://www.ti.com/product/ISO224 , Octopart pages for AMC3330 and AMC1311EVM.

## 11. IEC 61010 and UL practice on test points and protective impedance (general terms)

Summary only; the standard's text was not available, only its table of contents and secondary articles. No compliance is claimed.

- IEC 61010-1 clause 6 covers protection against electric shock. Its structure includes limit values for accessible parts in normal and single-fault conditions (6.3), impedance as a primary means of protection (6.4.4), **protective impedance** as an additional means under single fault (6.5.4), current- or voltage-limiting devices (6.5.6), and connections to external circuits including terminals that are hazardous live (6.6).
- Voltage levels reported for accessible parts: 30 V rms, 42.4 V peak or 60 V DC in normal condition; 50 V rms, 70 V peak or 120 V DC in single-fault condition. A trade-association note gives 0.5 mA as the normal-condition touch current limit for sinusoidal current.
- UNVERIFIED from background: the DC touch current limit is 2 mA in normal condition and 15 mA under single fault; lower voltage levels apply in wet locations (about 35 V DC); and protective impedance must be either one high-integrity component or a combination arranged so that one component failure still keeps the limits. IEC 61010-031 and -2-030 cover probe assemblies and measuring circuits.
- What this means for the site port, in general terms: a 16-cell LiFePO4 bank peaks around 58.4 V, just under the 60 V DC dry-condition level, so by that yardstick the dominant hazard is **energy** (arc, burn, fire from thousands of amps), not shock. A resistor pair that limits a fault to microamps addresses the energy hazard directly, and two components in series follow the single-fault logic of the protective-impedance idea. Margin to 60 V is under 2 V, so treating the bus as hazardous is the conservative reading.

Sources: https://assets.vde-verlag.de/iec-normen/preview-pdf/info_iec61010-1%7Bed3.1%7Db.pdf (contents only) , https://www.designworldonline.com/selv-pelv-en-61010-ul-61010-regulations-and-power-supplies/ , https://www.gambica.org.uk/static/a5857f02-3e66-4583-aeaff447988775e1/75df6f02-f14b-42a9-be31fe2c417426e4/GAMBICA-position-on-touch-current-limits.pdf

## 12. Identifying the port, and connectors

ID chip:

| Part | Type | Wires | Price |
|---|---|---|---|
| DS2431 | 1-Wire 1 kbit EEPROM with factory unique ID (UNVERIFIED detail) | 2 (data + ground, parasitic power) | $2.14 M (TO-92) |
| DS28E07 | 1-Wire 1 kbit EEPROM, sturdier ESD rating (UNVERIFIED) | 2 | $2.11 M |
| I2C EEPROM with serial number (24AA02E48 / AT24CS class) | I2C | 4 | under $1 (UNVERIFIED) |

1-Wire wins on pin count and tolerates being hot-plugged on a cable. Store: port UUID, nominal voltage, divider ratio or series resistance, calibration factor and date, checksum. **The ID chip's ground must be the instrument's reference, never battery negative**, unless the tap is the isolated variant.

Connectors:

| Connector | Keyed | Panel mount | Rating | Price | Comment |
|---|---|---|---|---|---|
| M12 A-coded, 5-pin (Amphenol LTW M12A-05PFFC-SH8B15, female sockets, wire leads) | yes | yes, bulkhead | **60 V AC/DC, 4 A**, IP68/IP69K | $7.46 DigiKey | Rating equals the bus maximum, so use it only with the divider or isolated tap (pins see about 1 V). 4-pin versions are rated higher (UNVERIFIED) |
| GX12 / GX16 "aviation" | yes | yes | no agency rating; sellers quote 125 V or more (UNVERIFIED) | $2 to $4 a pair (UNVERIFIED) | The usual chassis half has exposed male pins: wrong gender for an energised port |
| Neutrik XLR | yes | yes | about 50 V (UNVERIFIED), below the bus voltage | $3 to $8 (UNVERIFIED) | Avoid for bus voltage; fine with the divider tap. speakON is rated higher (UNVERIFIED) |
| Molex Micro-Fit 3.0 | yes, polarised with latch | panel-mount housings exist (UNVERIFIED) | up to 10.5 A; 600 V (voltage UNVERIFIED) | under $2 a mated set (UNVERIFIED) | Cheap and adequate indoors; not sealed, limited mating cycles |

Recommended: M12 A-coded 5-pin **female** receptacle on the tap enclosure, with a cap. Pin plan: 1 = sense +, 3 = sense -, 2 = 1-Wire data, 4 = 1-Wire/instrument reference, 5 = shield. Molded M12 cordsets are widely stocked.

Sources: https://www.digikey.com.mx/en/products/detail/amphenol-ltw/M12A-05PFFC-SH8B15/7102619 , https://www.molex.com/en-us/products/connectors/wire-to-board-connectors/micro-fit-connectors , Octopart pages for DS2431 and DS28E07.

---

## Could not verify

- DigiKey single-quantity prices for several parts (ADS131M04, AD7768-4, AD7606, ADS8688, ADS127L14): DigiKey pages truncated before the price; Mouser or Octopart figures used instead.
- Franchised (DigiKey/Mouser) prices for AD7771, AD7134, CS5381, CS5368, ISOUSB111, ISOW7741, AMC3330 IC, ADuM3165 IC, B0505S: only other sellers appeared.
- ES9822 price and availability; ADuM3160/ADuM4160 price; ISO224 and ACPL-C87x prices and evaluation boards.
- Whether embassy-rp supports isochronous endpoints today, and whether the merged UAC1 source class is in a released embassy-usb version (GitHub pages were rate-limited or blocked).
- High-pass-disable behaviour and DC performance of each audio converter (PCM1808, PCM186x, PCM4204, AK557x, CS53xx).
- AD7606 filter cutoff (the equivalent TI ADS8584S figure of 24 kHz / 16 kHz was confirmed from its datasheet).
- ADS131M04 maximum rate: TI's product table says 64 kSPS, DigiKey's listing says 32 kSPS; and its input impedance and lack of a FIFO.
- Full-speed USB throughput ceilings (taken from the USB 2.0 specification from memory).
- Amazon/eBay prices for AD7606 modules, PCM1808 modules, Hantek 6022BE, UMC404HD, GX12/GX16, Neutrik and Micro-Fit parts; PicoScope 2204A US price with stock (Newark listed $189 with none in stock) and its bit depth.
- HiFiBerry ADC8x/DAC8x price, input coupling, and how many I2S data lines it uses.
- The fuse value in Victron's SmartShunt/BMV supply lead.
- IEC 61010-1 touch-current limits for DC and single fault, wet-location levels and the exact protective-impedance wording (standard text is paywalled).
- ADC part and input ranges of the EEZ MIO168 AFE boards; ADC on FreeEEG32; file formats for OpenBCI Cyton and ThunderScope; what "openEMS" the reviewer meant.
- Bandwidth and noise figures for AMC3330, AMC1311 and ISO224 (TI product pages did not state them).
- Prospective short-circuit current of the specific bank: the 5 to 10 kA figure is an estimate from typical cell resistance, not a measurement.
