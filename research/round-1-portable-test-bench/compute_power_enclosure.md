# Compute, power, connectivity and enclosure platform for a portable Pi-class field diagnostic pack (state: November 2026)

Evidence labels used throughout: **[DOC]** = vendor/primary documentation fetched during this research; **[PRESS]** = trade press; **[COMMUNITY]** = forum/blog/mailing-list report; **[UNVERIFIED-BG]** = researcher background knowledge that was NOT confirmed against a source in this pass (only appears under Inferences/Gaps and must be checked before purchase).

---

## 1. Home Assistant Yellow exact hardware; is it a sensible brain?

### Takeaway
The Yellow is a 12 V-powered CM4/CM5 carrier with only two USB 2.0 host ports, an NVMe-capable M.2 M-key slot, an MGM210P 802.15.4 radio, a PCM5121 line-out DAC and a battery-backed PCF85063A RTC. It never exposes USB 3 (even with a CM5 fitted the RP1 USB 3 ports are not wired), so it is a workable zero-cost brain for RTL-SDR-class radios, audio, serial adapters and a logic analyzer, but a poor host for wideband SDRs plus USB cameras at the same time.

### Cited Findings
- [DOC] Product is discontinued ("no longer in production", still gets software updates); the pre-assembled Standard model is no longer made and the kit is listed out of stock — [Home Assistant Yellow product page](https://www.home-assistant.io/yellow/)
- [DOC] Accepts all 32 CM4 variants (quad Cortex-A72 1.5 GHz, up to 8 GB RAM, up to 32 GB eMMC); recommended minimum 2 GB RAM / 16 GB eMMC; CM4 Lite needs an NVMe SSD because it has no eMMC — [Home Assistant Yellow product page](https://www.home-assistant.io/yellow/)
- [DOC] CM5 is supported from Home Assistant OS 14; the CM5 cannot boot from the Yellow's USB 2.0 ports (ports work after boot), so first install needs rpiboot — [HA blog, 2024-11-27](https://www.home-assistant.io/blog/2024/11/27/home-assistant-yellow-gets-cm5-support/)
- [DOC] CM5 install procedure: JP1 jumper to USB, USB-C cable, hold recovery button, run `rpiboot -d mass-storage-gadget64`, flash, return JP1 to UART; use only the screws from the bag marked CM5 (or none) because other screws can damage the module — [Nabu Casa support: Yellow Kit with CM5](https://support.nabucasa.com/hc/en-us/articles/25606333033501-Home-Assistant-Yellow-Kit-with-CM5)
- [COMMUNITY] Jeff Geerling measured roughly a 2x speedup after swapping CM4 for CM5 in a Yellow (voice-model inference 2.8 s -> 1.3 s) and flagged the wide-head screws sitting close to a CM5 capacitor — [Jeff Geerling, 2024](https://www.jeffgeerling.com/blog/2024/home-assistant-yellow-instant-2x-iot-speedup-cm5/)
- [DOC] USB: 2 x USB 2.0 Type-A host ports; 1 x USB-C 2.0 *device* port wired to a Silicon Labs CP2102N bridge, which is a debug UART by default or CM4 USB recovery (rpiboot) when switched — [Home Assistant Yellow product page](https://www.home-assistant.io/yellow/)
- [DOC] Device tree for Yellow: on CM5 the two RP1 USB ports are disabled because they are "not connected on Yellow"; the remaining controller runs `dr_mode = "host"`; on CM4 it is `dr_mode = "otg"` — [Yellow device-tree patch (squashed from home-assistant/operating-system)](https://git.omada.cafe/chimera-linux/cports/raw/branch/linux-613/main/linux-rpi/patches/home-assistant-yellow.patch)
- [DOC] M.2: M-key socket, PCIe x1, NVMe, 2230/2242/2260/2280; Google Coral does not work in it; M.2 SATA SSDs are not supported; CM4 link is PCIe Gen 2 x1 — [Yellow product page](https://www.home-assistant.io/yellow/), [Yellow FAQ](https://yellow.home-assistant.io/faq/)
- [DOC] Radio: Silicon Labs MGM210P Mighty Gecko module, 2.4 GHz, up to +20 dBm, 1024 kB flash / 96 kB RAM, ships with Zigbee 3.0 EZSP firmware which "can be upgraded"; supports Zigbee 3.0, OpenThread, Matter — [Yellow product page](https://www.home-assistant.io/yellow/)
- [DOC] Radio appears as serial `/dev/ttyAMA1`; ZHA officially supported, Zigbee2MQTT experimental on this radio, OpenThread Border Router add-on experimental, multiprotocol experimental and can be unreliable — [Yellow FAQ](https://yellow.home-assistant.io/faq/)
- [DOC] Radio UART has hardware flow control: CM4 uart4 on GPIO8-11 (`uart-has-rtscts`); CM5 uart3 on GPIO8/9 with CTS/RTS on GPIO10/11 — [Yellow device-tree patch](https://git.omada.cafe/chimera-linux/cports/raw/branch/linux-613/main/linux-rpi/patches/home-assistant-yellow.patch)
- [DOC] RTC: NXP PCF85063A at I2C 0x51, backed by a CR2032 (10+ year estimate) — [Yellow device-tree patch](https://git.omada.cafe/chimera-linux/cports/raw/branch/linux-613/main/linux-rpi/patches/home-assistant-yellow.patch), [Yellow FAQ](https://yellow.home-assistant.io/faq/)
- [DOC] Audio: TI PCM5121 DAC at I2C 0x4c over I2S; stereo 2 V RMS line-out on 3.5 mm, SNR 106 dB. Output only; no audio input — [Yellow device-tree patch](https://git.omada.cafe/chimera-linux/cports/raw/branch/linux-613/main/linux-rpi/patches/home-assistant-yellow.patch), [Yellow product page](https://www.home-assistant.io/yellow/)
- [DOC] Device tree exposes I2C1 on GPIO2/3, a UART on GPIO14/15, SPI0 with two chip selects (spidev), and I2S shared with the DAC; debug console is a separate UART on GPIO12/13 (to the CP2102N); two buttons on GPIO26 (blue) and GPIO27 (red) — [Yellow device-tree patch](https://git.omada.cafe/chimera-linux/cports/raw/branch/linux-613/main/linux-rpi/patches/home-assistant-yellow.patch)
- [DOC] Power: 12 V / 2 A barrel jack (5.5 x 2.1 mm); PoE variant is PoE+ 802.3at Class 3/4 jumper-selectable; typical draw ~1.5 W idle, ~2.5 W idle with NVMe, ~5-9 W under load — [Yellow product page](https://www.home-assistant.io/yellow/)
- [DOC] Size: enclosure 123 x 123 x 36 mm, board 120 x 120 mm; Gigabit Ethernet — [Yellow product page](https://www.home-assistant.io/yellow/)
- [DOC] Hardware design is open (KiCad 6 sources, sheets for Audio, CM4, ControlRTC, Ethernet, PCIe, PoE, Power, Radio, SDCard, USB; a `cern_ohl_w_v2.txt` licence file is present) — [NabuCasa/yellow on GitHub](https://github.com/NabuCasa/yellow)
- [COMMUNITY] Raspberry Pi OS runs on a Yellow by copying `bcm2711-rpi-cm4-ha-yellow.dtb` into `/boot`, setting `device_tree=bcm2711-rpi-cm4-ha-yellow.dtb`, `otg_mode=1` under `[cm4]` for USB host, `enable_uart=1`, `dtoverlay=miniuart-bt`, console on `ttyAMA2`; radio then shows up as `/dev/ttyAMA1`. The poster disabled audio and did not report on RTC or NVMe — [HA community thread "Pi OS (instead of HA OS) on Yellow?"](https://community.home-assistant.io/t/pi-os-instead-of-ha-os-on-yellow/473695/10)
- [DOC] The Yellow device tree is carried as an out-of-tree patch (originating in the HAOS repo, re-based for kernel 6.12 by a Chimera Linux maintainer), covering both CM4 and CM5 — [Yellow device-tree patch](https://git.omada.cafe/chimera-linux/cports/raw/branch/linux-613/main/linux-rpi/patches/home-assistant-yellow.patch)
- [DOC] Thermal trip points defined in the CM5 device tree are 50 / 60 / 67.5 / 75 C mapped to fan states, but the PWM fan node is `disabled` — [Yellow device-tree patch](https://git.omada.cafe/chimera-linux/cports/raw/branch/linux-613/main/linux-rpi/patches/home-assistant-yellow.patch)

### Inferences
- A CM5 upgrade does not fix the Yellow's main weakness for this project. All host USB on the Yellow is one USB 2.0 high-speed bus behind a hub regardless of module, so every instrument shares a nominal 480 Mbit/s.
- USB 2.0 budget arithmetic (sample rate x 2 bytes for 8-bit IQ, x 4 bytes for 16-bit IQ): an RTL-SDR at 2.4 Msps is ~38 Mbit/s; three of them ~115 Mbit/s; a HackRF at 20 Msps is ~320 Mbit/s and effectively consumes the whole bus on its own. So Yellow-as-brain is viable for 2-3 RTL-SDR-class receivers + mic array + serial/CAN adapters + a low-rate logic analyzer, and not viable for a wideband SDR running concurrently with UVC cameras or a fast logic analyzer.
- The M.2 slot is the only high-speed path. Two ways to use it: (1) NVMe for local IQ/video recording, (2) a PCIe USB 3 host card on an M.2 M-key adapter to add real USB 3 ports [UNVERIFIED-BG: such cards are widely used on CM4 carriers, but no report of one in a Yellow was found, and a Lite CM4 would then have nothing to boot from].
- Because the Yellow already takes 12 V, it drops straight onto a 12 V battery/vehicle rail with no PD trigger, which simplifies the power tree compared with a Pi 5 (5 V / 5 A).
- The CP2102N USB-C port gives the Mac a zero-network serial console into the brain: a useful last-resort link that needs no Wi-Fi.
- Running non-HAOS Linux means maintaining the Yellow DTB yourself (copy from a HAOS build or apply the patch). It is a community-level path, not a vendor-supported one. Armbian/plain Debian were not found documented.

### Gaps
- Which CM4 is in the owner's two Yellows is unknown. [UNVERIFIED-BG] The pre-assembled "Standard" Yellow shipped with a non-wireless CM4 (2 GB / 16 GB eMMC); if so there is no onboard Wi-Fi/BT and the access-point plan needs a USB Wi-Fi adapter (costing one of only two USB 2.0 ports) or a module swap. Check with `cat /proc/device-tree/model` / look for the antenna section on the module.
- Physical GPIO/expansion header (populated or not, pitch, pin count) was not confirmed: the device tree shows I2C1/UART/SPI0 being assigned, but the product page and FAQ do not mention a header. Check the KiCad schematic before planning a GNSS PPS input or I2C power sensors on the Yellow.
- Operating temperature range is not published on the product page or FAQ.
- Sniffer firmware for the MGM210P: no source confirmed that the Yellow's radio can run 802.15.4 sniffer firmware. [UNVERIFIED-BG] The MGM210P is an EFR32MG21 module and Silicon Labs' RCP/multiprotocol builds exist; a Wireshark-capable sniffer build would need to be verified.
- Acceptable input-voltage range around 12 V (e.g. 13.3-14.6 V from a LiFePO4 pack or a running vehicle) not found; read the Power sheet of the schematic or put a 12 V regulator in front.
- Whether the JP1 "USB" position lets the CM4 act as a USB Ethernet gadget through the USB-C port in normal operation was not established.

---

## 2. PiKVM V4 Mini exact hardware and off-label uses

### Takeaway
Each V4 Mini is a fanless CM4 (2 GB, wireless, Lite) with a 1920x1200@60 HDMI capture input, one USB 2.0 OTG gadget port and an RJ-45 ATX port, all drivable over a documented HTTP/WebSocket API that already includes snapshot-with-OCR, typed text, virtual USB media and GPIO/command hooks. That makes one of them a ready-made "eyes and hands" appliance for the agent; the limits are no USB host ports, a 0-50 C rating and a USB-C "serial console" that is for managing the PiKVM itself rather than serving consoles of other gear.

### Cited Findings
- [DOC] V4 Mini ships with CM4102000 (2 GB RAM, Wi-Fi/Bluetooth, no eMMC; OS on microSD), 128x32 OLED; reseller price USD 275 in stock — [PiShop.us PiKVM V4 Mini](https://www.pishop.us/product/pikvm-v4-mini/)
- [DOC] HDMI capture up to 1920x1200 @ 60 Hz with audio; MJPEG and H.264 encoding — [PiKVM V4 Mini datasheet](https://docs.pikvm.org/v4/v4mini_datasheet.pdf), [PiShop.us](https://www.pishop.us/product/pikvm-v4-mini/)
- [DOC] 2K and 4K input are not supported on any PiKVM; V4 expects 1920x1200@60 and can encode 60 fps H.264 — [PiKVM FAQ](https://docs.pikvm.org/faq/)
- [DOC] The capture troubleshooting service is named `kvmd-tc358743`, i.e. the Toshiba TC358743 HDMI-to-CSI bridge family — [PiKVM V4 docs](https://docs.pikvm.org/v4/)
- [DOC] One USB 2.0 OTG (USB-C) port emulating keyboard, mouse, mass storage "and other devices"; RJ-45 ATX port described as for "power control or auxiliary use"; a separate USB-C "serial console management port"; RTC held up by an internal supercapacitor; microSD; optional 802.11 b/g/n with an SMA antenna mounting hole — [PiKVM V4 Mini datasheet](https://docs.pikvm.org/v4/v4mini_datasheet.pdf)
- [DOC] No USB host ports for external devices on the Mini; RJ-45 serial console, USB 3 host ports, LTE slot and 12 V input exist only on the V4 Plus — [PiShop.us](https://www.pishop.us/product/pikvm-v4-mini/), [PiKVM V4 docs](https://docs.pikvm.org/v4/)
- [DOC] Power: 5.1 V / 3 A USB-C, up to 12 W (2.6 A), idle about 2.65 W; operating 0-50 C, storage -20-60 C; 120 x 68 x 24 mm, 0.35 kg; fanless with vent holes — [PiKVM V4 Mini datasheet](https://docs.pikvm.org/v4/v4mini_datasheet.pdf), [PiShop.us](https://www.pishop.us/product/pikvm-v4-mini/)
- [DOC] PiKVM OS is Arch Linux ARM with its own package repo. Root is read-only; `rw` / `ro` toggle it; packages install with pacman; permanent RW mode, desktops and Docker are unsupported — [PiKVM FAQ](https://docs.pikvm.org/faq/)
- [DOC] API auth: `X-KVMD-User` / `X-KVMD-Passwd` headers, HTTP Basic, or a login cookie. Endpoints include `/api/atx/power` and `/api/atx/click`; `/api/hid/print` (type text), `send_key`, `send_shortcut`, absolute/relative mouse; `/api/msd/write`, `write_remote`, `set_params`, `set_connected`; `/api/gpio/switch` and `/pulse`; `/api/streamer/snapshot` returning JPEG or, with `ocr=true`, Tesseract text for a chosen region; `/api/ws` event socket (with `stream=0` to avoid keeping the streamer up); Redfish under `/redfish/v1`; Prometheus metrics at `/api/export/prometheus/metrics` — [PiKVM API docs](https://docs.pikvm.org/api/)
- [DOC] GPIO plugin types in kvmd: `gpio` (libgpiod), `hidrelay` (cheap USB HID relays), `ezcoo`, `ipmi`, `amt`, `wol`, `cmd` (run an arbitrary command), `pwm`, `servo`, `hue`, `anelpwr` (network PDU), `extron`; any of them can be bound to web-UI buttons and reached through the GPIO API. On V4, PWM uses CM4 GPIO12 (the CN5 NeoPixel pin); pin 25 is the V4-Mini locator LED — [PiKVM GPIO docs](https://docs.pikvm.org/gpio/)
- [DOC] USB Ethernet gadget: enable `otg.devices.ethernet` in `/etc/kvmd/override.yaml` and the `kvmd-otgnet` service; drivers ecm (Linux/macOS default), eem, ncm, rndis5, rndis; target gets DHCP on 172.30.30.0/24; firewall blocks target-to-PiKVM traffic by default with optional `allow_tcp` and `forward_iface` to route the target onto PiKVM's uplink. Page states it applies to "V2+" — [PiKVM USB Ethernet docs](https://docs.pikvm.org/usb_ethernet/)
- [COMMUNITY/secondary] The gadget has 9 endpoints: keyboard 1, absolute mouse 1, relative mouse 1, mass storage 2, Ethernet 3, serial (CDC-ACM) 3, microphone 2. V4 defaults use 5. `kvmd-otgconf` enables/disables functions live; VID/PID, manufacturer, product, serial and mass-storage inquiry strings are overridable in `override.yaml` — [DeepWiki summary of pikvm/pikvm USB features](https://deepwiki.com/pikvm/pikvm/9-usb-features)
- [DOC] The OTG link is USB 2.0 only; some BIOSes cannot handle HID + mass storage on one composite device; some targets need relative mouse mode — [PiKVM FAQ](https://docs.pikvm.org/faq/)
- [DOC] Tailscale publishes a PiKVM integration guide — [Tailscale docs: Access PiKVM from anywhere](https://tailscale.com/docs/integrations/pikvm.md)
- CONFLICT: the DeepWiki page says the V4 Mini has "no USB Ethernet, since it lacks a USB-A port" — [DeepWiki](https://deepwiki.com/pikvm/pikvm/9-usb-features); the official page scopes the feature to "PiKVM V2+" without excluding the Mini — [PiKVM USB Ethernet docs](https://docs.pikvm.org/usb_ethernet/). DeepWiki is machine-generated and the stated reason does not match how a gadget function works, so treat the official doc as authoritative and test on the unit.

### Inferences
Concrete off-label roles in this pack, each mapped to a documented capability (the role itself is the researcher's proposal, not something a source reported):
- **Screen reader for anything with HDMI out.** `GET /api/streamer/snapshot?ocr=true` returns text, so the agent can read a BIOS screen, a router's local console, an embedded board's framebuffer, or an instrument's display without spending vision tokens; the JPEG is there when layout matters. Candidates: HDMI-out oscilloscopes and thermal cameras, a drone remote controller's HDMI feed, an inverter/charge-controller with an HDMI or mini-PC front end, a game-camera or DVR/NVR local UI. Input must be 1920x1200@60 or lower.
- **Hands on a headless target.** `hid/print`, key events and mouse let the agent drive setup menus, bootloaders and kiosk UIs on a PC, NVR, SBC under test or a router with a local console.
- **USB device impersonation for host testing.** Custom VID/PID/strings plus selectable functions let the agent present a specific keyboard, a flash drive with a chosen image (`msd/write` then `set_connected`), a CD-ROM, a CDC-ACM serial port, a USB NIC or a USB microphone to a device under test, then yank and re-insert it in software. Useful for "does this inverter/printer/car head unit accept this firmware stick" loops.
- **Sneakernet replacement.** Mass-storage emulation with a writable image lets the agent drop files onto, or pull logs from, equipment that only speaks "USB stick".
- **Network injection.** The USB Ethernet gadget with `forward_iface` gives a target without working networking a routed link through the pack.
- **Serial link without an adapter.** The CDC-ACM gadget gives a Linux/Windows target a serial port whose far end is on the PiKVM (console for a headless SBC that has a USB host port).
- **Isolated power/reset actuator.** The ATX RJ-45 is documented for "auxiliary use"; with the ATX breakout it should act as a pair of momentary contact outputs plus two status inputs for power-cycling a target or sensing an LED [UNVERIFIED-BG: opto-isolation and pin ratings of the ATX port were not confirmed].
- **Extra actuators via plugins.** `hidrelay` is not usable on the Mini as shipped (no USB host port), but `cmd`, `wol`, `ipmi`, `anelpwr` and `hue` work over the network, so a PiKVM button/API call can trigger a script on the brain or a network PDU.
- **Second vantage point.** A V4 Mini with Wi-Fi is itself a small Linux box on the pack's network: it exports Prometheus metrics and can be scripted, so the brain can treat it as a remote probe left attached to a machine in the shed while the pack moves on.
- Integration should be over the HTTP API from the brain (Rust `reqwest`/WebSocket client) rather than by installing the MCP stack on PiKVM OS; the read-only root and no-Docker policy make it a poor place for custom services.
- Thermal: a fanless unit rated to 50 C is the most heat-fragile item in the pack. It needs airflow and should not be run in a closed case in the sun.
- Three units draw ~8 W idle together. One in the pack is enough; the other two are better deployed on the fixed bench/shed.

### Gaps
- Reports of other people using PiKVM hardware off-label: the search surfaced only thread titles (a Revolution Pi forum thread on using an industrial Pi as a KVM gadget, and pikvm/pikvm issue #1713 asking for a generic serial gadget) and no write-ups of instrument capture or USB-host fuzzing with PiKVM. Treat this as "no reliable sources found", not as "nobody does it".
- CM4 GPIO access inside the V4 Mini (free pins, headers) is not documented beyond GPIO12/CN5 and pin 25.
- HDCP behaviour of the capture input was not documented in the FAQ. [UNVERIFIED-BG] HDCP-protected sources are expected not to capture.
- The capture chip variant on V4 (TC358743 vs a successor) is only implied by a service name.
- Whether Raspberry Pi OS can be put on the Mini's SD card and still use the capture bridge and OTG was not researched.

---

## 3. If buying a brain: Pi 5 vs CM5 vs RK3588 boards vs N100/N150

### Takeaway
After four memory-driven price rounds, a Raspberry Pi 5 8 GB lists at USD 175 and the 16 GB at USD 385, while 16 GB N150 mini PCs on Amazon US start around USD 281-349. Inside a USD 300 ceiling that also has to cover hub, battery and case, the Pi 5 4 GB (USD 110) is the only new brain that leaves room for everything else; the 8 GB fits only if the battery is deferred.

### Cited Findings
- [DOC] 2 Feb 2026 increase: +USD 10 (2 GB), +15 (4 GB), +30 (8 GB), +60 (16 GB) across Pi 4, Pi 5, CM4 and CM5; 1 GB models held (Pi 5 1 GB USD 45); an earlier rise came about two months before; Raspberry Pi called the situation temporary and said it intends to reverse the rises — [Raspberry Pi news, "More memory-driven price rises"](https://www.raspberrypi.com/news/more-memory-driven-price-rises/)
- [PRESS] As of 2 Nov 2026: Pi 5 16 GB USD 385 (launch USD 120; fourth rise, +80 from 305), Pi 5 8 GB USD 175, Pi 5 4 GB USD 110 — [Hardware Busters, 2026-11-02](https://hwbusters.com/news/raspberry-pi-5-16gb-price-jumps-to-385-and-the-extra-8gb-now-costs-more-than-a-whole-8gb-board/)
- [PRESS] Same round: CM5 16 GB USD 425 (+40), Pi 500+ 16 GB held at USD 485; about a month earlier Pi 5 2 GB went to USD 77.50 and Pi 4 2 GB to USD 67.50 — [Tom's Hardware, 2026-11-02](https://www.tomshardware.com/raspberry-pi/16gb-raspberry-pi-5-hiked-to-usd385-more-than-triple-its-launch-price-makers-are-already-shifting-to-pico-esp32-for-small-projects-mini-pcs-for-more-ambitious-tasks)
- [DOC] US reseller confirms Pi 5 8 GB at USD 175.00 — [PiShop.us Pi 5 8GB](https://www.pishop.us/product/raspberry-pi-5-8gb/)
- NOTE (stale page): raspberrypi.com's own Pi 5 page still showed USD 305 for the 16 GB when fetched, which is the pre-November figure — [Raspberry Pi 5 product page](https://www.raspberrypi.com/products/raspberry-pi-5/)
- [DOC] Pi 5 I/O: 2 x USB 3.0 capable of simultaneous 5 Gbit/s, 2 x USB 2.0, PCIe 2.0 x1 (needs an M.2 HAT), 5 V / 5 A USB-C PD supply recommended, onboard RTC that needs an external battery — [Raspberry Pi 5 product page](https://www.raspberrypi.com/products/raspberry-pi-5/)
- [DOC] Without a PD source that negotiates 5 A, a Pi 5 caps USB current at the lower limit; `usb_max_current_enable=1` in config.txt or `PSU_MAX_CURRENT=5000` in the EEPROM overrides it — [Waveshare UPS HAT (E) wiki](https://www.waveshare.com/wiki/UPS_HAT_(E))
- [DOC] Pi 5's four onboard ports are power-ganged: VBUS only drops when all are switched off, so the board cannot power-cycle one instrument by itself — [uhubctl README](https://github.com/mvp/uhubctl)
- [COMMUNITY] CM5 with an Ettus USRP B205mini in SDR++ sustained 2/4/8/16 MHz bandwidth; board power 5.2 W idle, 6.6 W at 2 MHz, 8.5 W at 16 MHz; audio broke up only when a full `stress-ng` load ran alongside 16 MHz — [element14 CM5 RoadTest](https://community.element14.com/products/roadtest/b/blog/posts/raspberry-pi-cm5-roadtest---usrp-bandwidth-stress-test)
- [COMMUNITY] One unresolved report (3 Jan 2026, no replies) of an RTL-SDR Blog V4 on Pi 5 where `rtl_sdr`/`rtl_fm`/`rtl_power` write zero bytes although `rtl_test` streams, with the same dongle fine on Pi 3/4 — [osmocom-sdr mailing list](https://lists.osmocom.org/hyperkitty/list/osmocom-sdr@lists.osmocom.org/thread/2SJRDEDAQ3PRCKYCSZOBZ4YEKPF7FQHA)
- [PRESS] Amazon US N150 mini PCs, 16 GB / 500-512 GB, 1 Nov 2026: cheapest in stock USD 280.79 (MegaPC), Bmax USD 284.99 (sold out by 6 Nov), Beelink Mini S13 USD 349 (class median, down from 399 in September), fanless MeLE with Win 11 Pro USD 549.99; a barebone N150 at USD 235 ends up near USD 585 once a USD 239.99 16 GB SO-DIMM and SSD are added — [ComputingForGeeks, 2026-11-06](https://computingforgeeks.com/mini-pc-prices/)
- [COMMUNITY] Jeff Geerling's N150-vs-Pi 5 comparison (written when a 16 GB GMKtec G3 Plus was USD 159): N150 about 1.5-2x faster, Pi 5 lower idle power and more work per watt in HPL, which he says matters for solar/battery/PoE setups; the tested N150 needed an added fan — [Jeff Geerling, 2025](https://www.jeffgeerling.com/blog/2025/intel-n100-better-value-raspberry-pi)

### Inferences
- Price ladder for a new brain in Nov 2026: Pi 5 2 GB USD 77.50 < Pi 5 4 GB USD 110 < Pi 5 8 GB USD 175 < N150 16 GB ~USD 281-349 < Pi 5 16 GB USD 385. The 16 GB Pi is no longer rational for this project; an N150 box is cheaper and faster.
- For a sensor body whose heavy reasoning runs on the Mac, 4 GB is enough for device drivers, an MCP server in Rust, a few GStreamer/SoapySDR pipelines and ring buffers. 8 GB only matters if on-device ML or large in-RAM IQ captures are planned.
- Pi 5 vs N150 for a battery pack: the Pi takes 5 V, idles lower, has GPIO for PPS and I2C power sensors and a native CSI path; the N150 has more and better-behaved USB 3 ports and x86 driver coverage but needs 12-19 V, more watts, an external MCU for GPIO/I2C, and by itself consumes the whole USD 300 tier.
- [UNVERIFIED-BG] The Pi 5's two USB 3 ports sit on two separate controllers in the RP1 chip, so putting one powered hub on each gives two independent high-speed trees; most hobby SDRs (RTL-SDR, HackRF, Airspy) are USB 2.0 high-speed devices, so the practical win over the Yellow is two-plus buses and a faster CPU rather than SuperSpeed itself.
- CM5 + carrier is not attractive here: the owner's existing CM carriers (Yellow, PiKVM) do not expose CM5 USB 3, and a CM5 plus a new IO board costs more than a Pi 5 of the same RAM.

### Gaps
- Orange Pi 5 / 5 Plus and Radxa Rock 5 (RK3588/RK3588S) November 2026 prices, USB layout and SDR/camera driver maturity were not researched in this pass. [UNVERIFIED-BG] They use the same LPDDR4/5 that drove Pi prices up, and their mainline/vendor-kernel split makes them a riskier choice for a Rust-on-Linux project that leans on USB peripherals.
- Current prices for Pi 5 accessories (Active Cooler, M.2 HAT+, RTC battery) and CM5 8 GB / CM4 variants were not captured.
- Pi 5 operating temperature limits and throttle points were not on the product page. [UNVERIFIED-BG] Throttling begins around 80-85 C SoC temperature; an active cooler is effectively mandatory at 40 C+ ambient.
- No measured data on how many simultaneous SDR streams a Pi 5 sustains through hubs; the RTL-SDR V4 zero-output report is a single unconfirmed case.

---

## 4. USB topology: hubs, per-port power switching, isolation, monitoring

### Takeaway
Per-instrument power cycling has to come from an external hub that genuinely switches VBUS; uhubctl's list names several current 7- and 10-port USB 3 hubs, and the Plugable USB3-HUB7BC (USD 49.95, fed by a 12 V / 5 A adapter) is a good fit because it can live on the pack's 12 V rail. Isolation and per-port current measurement were not sourced in this pass.

### Cited Findings
- [DOC] uhubctl-listed USB 3 hubs with per-port power switching include: Plugable USB3-HUB7BC (7 port, 2109:0813), Plugable USBC-HUB7BC (6 of 7 ports switch), AmazonBasics HU9003V1EBL / HUC9003V1EBL (7 port, 2109:2817), AmazonBasics HU9002V1SBL family (10 port), Rosonway RSH-518C (7), RSH-A37S (7), RSH-A10 (10, 0BDA:0411), RSH-ST10C-6 (10), j5create JUH377 (7), UUGear MEGA4 (4 port, built for the Pi 4B), StarTech HB30A4AIB (4, with a warning), VirtualHere 4-port USB3 hub — [uhubctl README](https://github.com/mvp/uhubctl)
- [DOC] Caveats recorded in the same table: Plugable USB3-HUB7C and USB3-HUB10-C2 "only 2 charge ports work"; Rosonway RSH-ST07C only 4 ports; Inateck HB2025A and Rosonway RSH-A104 switch in USB 2 mode only; AmazonBasics U3-7HUB only 1 port; Targus ACH155 port 3 controls all — [uhubctl README](https://github.com/mvp/uhubctl)
- [DOC] A USB 3 hub appears as two logical hubs (USB 2 and USB 3) and both must be switched; uhubctl does this automatically unless `-e` is given — [uhubctl README](https://github.com/mvp/uhubctl)
- [DOC] Some hubs drop data but leave VBUS live; uhubctl does not count those as supported and suggests testing with a USB light/fan/phone — [uhubctl README](https://github.com/mvp/uhubctl)
- [DOC] Pi 4B needs VL805 firmware 00137ad or later and switches its four ports as a gang; Pi 5 likewise gangs all four; the README has no CM4 section — [uhubctl README](https://github.com/mvp/uhubctl)
- [DOC] Plugable USB3-HUB7BC: USD 49.95; 60 W adapter rated 12 V / 5 A; seven downstream ports each rated 1.5 A (BC 1.2); two VIA VL811+ hub chips (firmware 9095). The product page itself does not advertise per-port switching — [Plugable product page](https://plugable.com/products/usb3-hub7bc)

### Inferences
- Layout for a Yellow brain: one switched 7-port hub on one Yellow USB port carrying all instruments (all at USB 2.0 speed), second Yellow port kept for a Wi-Fi adapter or the highest-rate device.
- Layout for a Pi 5 brain: one switched hub per USB 3 port, SDRs split across the two trees, cameras and logic analyzer on the opposite tree from the widest SDR, low-rate serial/CAN/RS-485 adapters on the Pi's own USB 2 ports.
- A hub with a 12 V input can run from the battery rail through a small 12 V regulator instead of its wall adapter, so the hub also acts as the 5 V distribution point for instruments (7 x 1.5 A).
- uhubctl gives on/off only. Per-port current/voltage visibility needs separate sensors: an I2C INA3221 (3 channels) or INA226/INA228 per rail on the feed lines of the instruments that matter, or simply one sensor on the hub's 12 V input for total instrument power [UNVERIFIED-BG].
- Hub chip firmware matters: the README's notes are specific to hardware revisions, so buy from a seller with easy returns and run the lamp test on every port on day one.
- Since the daisy-chained VL811+ design is two 4-port hubs in series, devices on the far tier sit one hub deeper; keep SDRs on the first tier.

### Gaps
- USB isolators: nothing sourced. [UNVERIFIED-BG] Low-cost ADuM3160/4160 isolators are limited to full speed (12 Mbit/s), which suits serial/CAN/RS-485 adapters and a multimeter but not SDRs, cameras or logic analyzers; high-speed (480 Mbit/s) isolators based on newer isolator ICs cost several times more. For a pack that connects to a 48 V battery bank and vehicle chassis, isolating the *measurement* adapters (isolated RS-485/CAN dongles, isolated USB-serial) is usually cheaper than isolating the hub.
- Current Amazon US prices for the AmazonBasics, Rosonway and UUGear hubs were not captured.
- No source found testing these hubs specifically with multiple SDRs on a Pi, or their RF noise.
- Off-the-shelf hubs with built-in per-port current telemetry were not researched.

---

## 5. Power architecture

### Takeaway
A 25-45 W load for 4+ hours needs roughly 120-210 Wh at the battery, which is beyond a single flight-legal USB-C power bank and beyond a Pi UPS HAT, so a 12 V LiFePO4 pack as the main bus (Yellow and a 12 V-input hub run from it directly; 5 V bucks feed PiKVM/Pi) is the natural design; a Waveshare UPS HAT (E) is the one sourced option that gives a Pi 5 its own I2C-readable battery and clean shutdown.

### Cited Findings
- [DOC] Yellow input is 12 V / 2 A on a 5.5 x 2.1 mm barrel; draw 1.5-9 W — [Yellow product page](https://www.home-assistant.io/yellow/)
- [DOC] PiKVM V4 Mini input is 5.1 V / 3 A USB-C, 2.65 W idle, 12 W peak — [PiKVM V4 Mini datasheet](https://docs.pikvm.org/v4/v4mini_datasheet.pdf)
- [DOC] Pi 5 wants 5 V / 5 A over USB-C PD — [Raspberry Pi 5 product page](https://www.raspberrypi.com/products/raspberry-pi-5/)
- [DOC] Plugable USB3-HUB7BC runs from 12 V / 5 A — [Plugable product page](https://plugable.com/products/usb3-hub7bc)
- [DOC] Waveshare UPS HAT (E): 4 x 21700 cells (not included, 5000 mAh typical), 5 V output rated 6 A, bidirectional USB-C with PD 3.0 charging up to 40 W, fuel-gauge IC reporting battery voltage, current, power and remaining percent plus USB-C port status over I2C, pogo-pin connection to Pi 5 / 4B / 3B+ — [Waveshare UPS HAT (E) wiki](https://www.waveshare.com/wiki/UPS_HAT_(E)), [Waveshare product page](https://www.waveshare.com/ups-hat-e.htm)
- [DOC] Same HAT: writing 0x55 to I2C register 0x01 makes it cut Pi power after 30 s and restart the Pi when a charger appears; vendor demo shuts down within 60 s on low battery with no external power — [Waveshare UPS HAT (E) wiki](https://www.waveshare.com/wiki/UPS_HAT_(E))
- [DOC] Victron's isolated Orion-Tr range includes 48 V-input models 48/12-9A (110 W) and 48/12-20A (240 W); IP43 when mounted terminals-down; input fuse fitted only on 12/24 V-input models — [Victron Orion-Tr isolated DC-DC page](https://www.victronenergy.com/dc-dc-converters/orion-tr-dc-dc-converters-isolated)
- [COMMUNITY] CM5 board-level draw while streaming a USB SDR: 5.2 W idle to 8.5 W at 16 MHz — [element14 CM5 RoadTest](https://community.element14.com/products/roadtest/b/blog/posts/raspberry-pi-cm5-roadtest---usrp-bandwidth-stress-test)

### Inferences
- Energy arithmetic: 25 W x 4 h = 100 Wh and 45 W x 4 h = 180 Wh at the load; at ~85% conversion that is ~118-212 Wh from the battery. A 12.8 V 10 Ah LiFePO4 (128 Wh) covers the low end; 12.8 V 20 Ah (256 Wh) covers the high end with margin.
- A UPS HAT (E) with four 5000 mAh cells is ~72 Wh (4 x 3.6 V x 5 Ah). It carries a ~8-10 W Pi 5 for hours but only ~1.5-3 h of the full 25-45 W pack, and its 5 V / 6 A (30 W) output cannot feed brain plus hub plus PiKVM. Its role is brain-only ride-through and self-reported battery state, not the main pack battery.
- A 100 Wh (airline-limit) USB-C PD power bank delivers roughly 85 Wh after conversion: ~3.4 h at 25 W, under 2 h at 45 W. Good as a hot-swap or top-up source, insufficient alone for the stated goal.
- Proposed 12 V bus: battery -> fused Anderson Powerpole distribution -> (a) Yellow directly or via a 12 V regulator, (b) 12 V-input USB hub, (c) 12 V -> 5.1 V / 5 A buck for a Pi 5, (d) 12 V -> 5.1 V / 3 A buck for one PiKVM, (e) fans. One INA226/INA228 on the battery lead gives pack voltage, current and coulomb count to the brain over I2C; an INA3221 gives three branch rails. Safe shutdown is then a small Rust daemon acting on the bus-voltage threshold [sensor choices UNVERIFIED-BG].
- External inputs feed the same bus through a charger/regulator: vehicle 12 V socket (11-15 V with transients, so through a buck-boost/DC-DC charger rather than straight onto electronics), and the 48 V shed bank through an isolated 48 -> 12 V converter such as the Orion-Tr 48/12-9A. Isolation on the 48 V input is worth paying for because the pack will also be clipped onto equipment that shares that bank's negative.
- With a Pi 5, a PD trigger is unnecessary if a 12 -> 5.1 V / 5 A buck feeds the GPIO header or USB-C and `usb_max_current_enable=1` is set; PD "decoy" boards are only needed to pull 12/15/20 V out of a USB-C power bank into the 12 V bus.
- The owner's PinePower desktop supply and PinePower V2 are bench/dock supplies (mains-powered), useful for the fixed bench side, not for the field bus [capabilities UNVERIFIED-BG].

### Gaps
- No November 2026 prices were captured for: 12 V LiFePO4 packs (10/20 Ah), USB-C PD power banks, PD trigger boards, 12 -> 5 V buck modules, INA226/INA228/INA3221 breakouts, Waveshare UPS HAT (E) (price is not shown on the vendor pages fetched), 21700 cells, or the Victron 48/12-9A. All power-side prices in the recommendation below are placeholders to verify.
- Power-tool battery adapters (which brands, low-voltage cutoff behaviour) were not researched. [UNVERIFIED-BG] Most bare tool-battery adapters have no under-voltage cutoff, which makes them a worse fit than a LiFePO4 pack with its own BMS unless a cutoff board is added.
- Victron 48 V-input model electrical specs (input range, efficiency, temperature derating) were not on the page fetched; read the datasheet.
- Yellow's tolerance to 13-14.6 V is unknown (see section 1).
- LiFePO4 charge behaviour and BMS high-temperature cutoffs in a 60 C+ vehicle interior were not researched.

---

## 6. Timing and location: GNSS with PPS, chrony, RTC

### Takeaway
A cheap serial GNSS module with a PPS pin plus `pps-gpio` and chrony gets a Pi to roughly microsecond-level time, and every candidate board already has an RTC of some kind for holdover between fixes.

### Cited Findings
- [COMMUNITY] Recipe: Adafruit Ultimate GPS (~USD 40) or GT-U7 (~USD 12, 2021 prices); PPS to GPIO18; `dtoverlay=pps-gpio,gpiopin=18`, `enable_uart=1`, `init_uart_baud=9600`; chrony `refclock SHM 0 delay 0.325 refid NMEA` and `refclock PPS /dev/pps0 refid PPS`; offsets of a few hundred ns reported, settling to microsecond level within about a minute — [Austin's Nerdy Things, 2021](https://austinsnerdythings.com/2021/04/19/microsecond-accurate-ntp-with-a-raspberry-pi-and-pps-gps/)
- [DOC] Yellow: PCF85063A RTC with CR2032 — [Yellow device-tree patch](https://git.omada.cafe/chimera-linux/cports/raw/branch/linux-613/main/linux-rpi/patches/home-assistant-yellow.patch)
- [DOC] Pi 5: onboard RTC needing an external battery — [Raspberry Pi 5 product page](https://www.raspberrypi.com/products/raspberry-pi-5/)
- [DOC] PiKVM V4 Mini: RTC with supercapacitor backup — [PiKVM V4 Mini datasheet](https://docs.pikvm.org/v4/v4mini_datasheet.pdf)

### Inferences
- With no internet in the field, GNSS+PPS on the brain makes it the stratum-1 source; the Mac and PiKVM can sync to it over the pack Wi-Fi so captures, serial logs and screen snapshots share one timeline.
- On a Pi 5 the PPS/UART wiring is trivial (40-pin header). On the Yellow it depends on the unverified header (section 1); the fallback is a USB GNSS receiver, which gives position and ~millisecond time but no hardware PPS.
- A USB GNSS dongle costs a hub port and loses PPS precision; a UART+PPS module costs GPIO pins instead. Prefer the latter where pins exist.

### Gaps
- u-blox M8 vs M10 module options, 2026 prices and counterfeit risk on marketplace modules were not researched.
- PPS over USB (via DCD on a USB-serial adapter) accuracy was not researched.
- GNSS antenna placement through a hard case (bulkhead SMA with active antenna) was not sourced.

---

## 7. Wireless link to the Mac with no internet

### Takeaway
A wireless CM4/Pi 5 can host the pack's own access point and Raspberry Pi now ships an official USB-gadget Ethernet package (ECM on macOS) for Pi 5/CM5, so a Pi 5 brain gets a one-cable wired fallback for free; on a Yellow the equivalent fallbacks are Gigabit Ethernet and the built-in USB-C serial console.

### Cited Findings
- [DOC] `rpi-usb-gadget` (Raspberry Pi OS Trixie images from 20 Oct 2025, or `sudo apt install rpi-usb-gadget && sudo rpi-usb-gadget on`) supports Pi Zero family, 3A+, 4B, 5, 500/500+ and CM5; CM4 is "technically supported" but needs manual setup. Uses CDC-ECM for macOS/Linux and RNDIS for Windows; macOS sees a USB Ethernet device and can share internet to it — [Raspberry Pi news: USB gadget mode](https://www.raspberrypi.com/news/usb-gadget-mode-in-raspberry-pi-os-ssh-over-usb/)
- [DOC] Caveats: connect straight to the computer, some laptop ports cannot power a Pi 4/5 (reboots, link drops), hubs/docks may break OTG negotiation, a powered hub can inject power while passing data, and the gadget port can no longer host peripherals — [Raspberry Pi news: USB gadget mode](https://www.raspberrypi.com/news/usb-gadget-mode-in-raspberry-pi-os-ssh-over-usb/)
- [DOC] Yellow's USB-C port is a CP2102N serial bridge (debug console) by default — [Yellow product page](https://www.home-assistant.io/yellow/)
- [DOC] PiKVM V4 Mini CM4 has Wi-Fi/Bluetooth (CM4102000) — [PiShop.us](https://www.pishop.us/product/pikvm-v4-mini/)
- [DOC] PiKVM Wi-Fi guidance: use 2.4 GHz channels 1-11 and set the regulatory domain to stop random disconnects — [PiKVM FAQ](https://docs.pikvm.org/faq/)
- [DOC] Tailscale documents a PiKVM integration — [Tailscale docs](https://tailscale.com/docs/integrations/pikvm.md)

### Inferences
- Throughput arithmetic for what crosses the link: raw 8-bit IQ at 2.4 Msps is ~38 Mbit/s (fits a decent 5 GHz link), 10 Msps 16-bit is ~320 Mbit/s (does not). Design the MCP server to record/decimate/FFT on the pack and send results, with raw IQ pulled as files over Ethernet or the USB gadget link when needed. H.264 from PiKVM and cameras is a few Mbit/s and is not the constraint.
- Topology proposal: brain `wlan0` = AP for the Mac; brain `eth0` = direct cable to the in-pack PiKVM (or a 5-port switch if two PiKVMs ride along); a second USB Wi-Fi adapter (monitor-mode capable) reserved for sniffing so the AP radio never has to leave its channel.
- Discovery: Avahi/mDNS name for the brain and a fixed AP subnet; PiKVM reached by static address on the wired side and proxied by the MCP server so the Mac needs one endpoint.
- Security: the MCP endpoint should be mutually authenticated (mTLS or WireGuard between Mac and pack) independent of Wi-Fi encryption; PiKVM's API uses header/basic credentials, so keep it on the wired back-side network and never expose it on the AP directly.
- Tailscale on the brain is a reasonable "when internet exists" path and costs nothing when offline.

### Gaps
- Access-point specifics were not sourced: NetworkManager/hostapd configuration on current Raspberry Pi OS, realistic AP throughput of the CM4/Pi 5 onboard radio, and WPA3-SAE support in AP mode. [UNVERIFIED-BG] The onboard Broadcom radio is single-stream 802.11ac and WPA3 AP mode on it has a history of being unreliable; plan for WPA2-PSK plus an encrypted/authenticated application layer unless testing proves otherwise.
- Whether the Yellow can expose a USB Ethernet gadget at all (section 1 gap).
- Which USB Wi-Fi chipsets have dependable in-kernel monitor/injection support in late 2026 was not researched.

---

## 8. Enclosure, panels, materials and heat

### Takeaway
Harbor Freight's Apache 3800 (USD 39.99, interior about 378 x 270 x 156 mm) and 4800 (USD 59.99, about 454 x 327 x 168 mm) are IP65 and the cheapest credible shells; printed panels must be tiled for the 180 mm bed, and material is the real constraint: Bambu rates ASA/ABS/PC "not recommended" on the open A1 mini, while PETG's ~69 C heat-deflection point equals the measured dashboard temperature of a car parked in Arizona sun.

### Cited Findings
- [DOC] Apache 3800: USD 39.99, in-store only; interior 14-7/8 x 10-5/8 x 6-1/8 in (the page's highlights give 14-15/16 x 10-9/16 x 6-11/16 in); exterior 16-1/2 x 13 x 6-3/4 in; 5.95 lb shipping weight; IP65 — [Harbor Freight Apache 3800](https://www.harborfreight.com/3800-weatherproof-protective-case-large-black-63927.html)
- [DOC] Apache 4800: USD 59.99; interior 17-7/8 x 12-7/8 x 6-5/8 in; exterior 20-5/16 x 15-1/2 x 7-3/8 in; 10.34 lb shipping weight; IP65 "watertight and dustproof" — [Harbor Freight Apache 4800](https://www.harborfreight.com/4800-weatherproof-protective-case-x-large-black-64250.html)
- [DOC] Apache range prices: 1800 USD 14.99, 2800 USD 29.99, 3800 USD 39.99, 4800 USD 59.99, 5800 rolling USD 89.99 (member price 69.99 shown as expired 10/29), 9800 rifle case USD 149.99 — [Harbor Freight search listing](https://www.harborfreight.com/search?q=apache%20weatherproof%20protective%20case)
- [DOC] Bambu's material table: PLA heat-deflection 2/5 stars, prints open; PETG 3/5, A1 mini OK (enclosure helps layer strength), bed 60-80 C; ABS and ASA 3/5, bed 90-100 C, "not recommended" on A1 mini; PC 4/5, bed 100-120 C, not recommended on A1 mini; PET-CF and PAHT-CF 5/5 but need hardened steel nozzles and are not recommended on A1 mini — [Bambu Lab filament guide table](https://wiki.bambulab.com/en/general/filament-guide-material-table)
- [DOC] Bambu PETG HF: HDT 69 C at 0.45 MPa, Vicat 70 C, nozzle 230-260 C, bed 65-75 C — [Bambu Lab PETG HF](https://us.store.bambulab.com/products/petg-hf)
- [DOC] Bambu ASA: USD 29.99 per kg, HDT 100 C at 0.45 MPa, Vicat 106 C, marketed for UV/ageing resistance, "Enclosure Printers Required", bed 80-100 C, nozzle 240-270 C — [Bambu Lab ASA](https://us.store.bambulab.com/products/asa-filament)
- [COMMUNITY/academic] Tempe, Arizona, days in the 100s F, cars parked one hour: in sun, cabin air averaged 116 F, dashboard 157 F, steering wheel 127 F, seat 123 F; in shade, cabin ~100 F and dashboard 118 F (Vanos et al., *Temperature*, 2018, DOI 10.1080/23328940.2018.1468205) — [ASU News](https://news.asu.edu/20180516-discoveries-asu-study-hot-cars-can-hit-deadly-temperatures-within-one-hour)
- [DOC] PiKVM V4 Mini is rated 0-50 C operating, -20-60 C storage — [PiKVM V4 Mini datasheet](https://docs.pikvm.org/v4/v4mini_datasheet.pdf)

### Inferences
- Unit conversions: 157 F = 69 C (dashboard), 116 F = 47 C (cabin air), 123 F = 51 C (seat). PETG HF's 69 C HDT has zero margin on a dashboard and little inside a dark case in direct sun; PLA is ruled out. PETG parts are acceptable only if the pack rides in shade/footwell/boot and parts are not preloaded (no thin clips or cantilevers holding weight).
- Material strategy on an A1 mini: print brackets, cable guides and instrument cradles in PETG; make the load-bearing top panel from sheet stock (3 mm aluminium or ABS sheet cut to the case opening) with printed inserts; if ASA is wanted, either order those few parts from a print service or print small ASA parts inside a tent/draft shield and accept the vendor's "not recommended" [the tent workaround is UNVERIFIED-BG].
- Tiling for a 180 x 180 mm bed: the 3800 opening (~378 x 270 mm) needs a 3 x 2 grid of ~126 x 135 mm tiles; the 4800 opening (~454 x 327 mm) needs 3 x 2 at ~151 x 163 mm. A 2 x 2 layout does not fit either case on this printer. Tiles should bolt to a sub-frame (aluminium angle or 2020 extrusion) rather than to each other.
- Sizing: three or more USB instruments, a 123 mm-square Yellow, a 120 x 68 mm PiKVM, a 7-port hub and a 20 Ah LiFePO4 brick will not fit a 2800; the 3800 is the minimum and the 4800 gives room for airflow and a dock connector panel for USD 20 more (and about 4 lb more).
- Heat: an IP65 case with the lid shut is an oven. At 25-45 W internal dissipation plus solar load the interior will exceed the PiKVM's 50 C limit, and the stored-in-vehicle case can exceed its 60 C storage limit on a dashboard. Operate lid-open in shade, or fit filtered intake/exhaust (accepting a drop from IP65 to dust-resistant) with tach-reporting PWM fans the brain can monitor, and add internal temperature sensors so the agent can throttle or shut down. Choose the orange/tan case colour over black where offered.
- Bulkheads worth fitting: Anderson Powerpole (DC in), panel RJ45 (dock/PoE), SMA bulkheads for SDR and GNSS antennas, a panel USB-C for the Mac tether. Keep caps on unused ports for dust.

### Gaps
- Apache 2800 dimensions (page fetch failed) and all Pelican 1200/1400, Nanuk, Seahorse and ammo-can dimensions and prices were not captured.
- No sourced data on PETG long-term creep or UV ageing in vehicle interiors, or on ASA print success rates on an A1 mini. [UNVERIFIED-BG] The A1 mini's heated bed tops out around 80 C, at the bottom of ASA's stated 80-100 C range.
- Printables/Gridfinity/MOLLE/DIN-rail case-insert designs sized for Apache cases were not surveyed.
- Filter media, fan models with tach output and measured in-case temperatures for similar builds were not researched.
- LiFePO4 and Li-ion cells have their own temperature limits for charging/storage in a hot vehicle; not researched.

---

## 9. Docking to a fixed bench

### Takeaway
No sources were gathered specifically on docking; the low-cost options below follow from the interfaces already documented for the candidate boards.

### Cited Findings
- [DOC] Yellow has Gigabit Ethernet and an optional PoE+ (802.3at) assembly variant — [Yellow product page](https://www.home-assistant.io/yellow/), [NabuCasa/yellow](https://github.com/NabuCasa/yellow)
- [DOC] A Pi 5 or CM5 can present itself as USB Ethernet to a Mac over one USB-C cable, with the stated power caveats — [Raspberry Pi news: USB gadget mode](https://www.raspberrypi.com/news/usb-gadget-mode-in-raspberry-pi-os-ssh-over-usb/)
- [DOC] Isolated 48 V -> 12 V converters exist in 110 W and 240 W sizes for feeding a 12 V bus from the shed bank — [Victron Orion-Tr isolated](https://www.victronenergy.com/dc-dc-converters/orion-tr-dc-dc-converters-isolated)

### Inferences
- Simplest robust dock: two connectors on the case wall, Anderson Powerpole for 12 V in (from the bench's 48 -> 12 V isolated converter or the PinePower desktop supply) and a panel RJ45 into the pack's switch/brain. Ethernet carries all data to the bench network; power charges the pack battery and runs the load. No pogo pins, no alignment problem, tolerant of dust.
- If the owner's spare Yellow is the PoE variant, a single PoE+ cable can power the Yellow itself (not the whole 25-45 W pack) and carry data: a tidy "brain stays alive on the bench" mode.
- A single USB-C/Thunderbolt cable dock only works naturally with a Pi 5/CM5 brain in gadget mode and cannot both power a 45 W pack and avoid the laptop-port power caveats; treat USB-C as a convenience tether, not the dock.
- Pogo-pin docks are the least attractive option in a dusty environment.

### Gaps
- No sourced examples of pack-to-bench docking designs, pogo connector ratings, or Powerpole/RJ45 bulkhead part prices.
- Whether either of the owner's Yellows is the PoE variant is unknown.

---

## 10. Recommended architecture at two spend levels

### Takeaway
Tier A reuses the spare Yellow as brain plus one PiKVM V4 Mini and spends about USD 100 of sourced-price items (switched hub + Apache 3800) with the remainder on a small 12 V LiFePO4 pack and wiring; Tier B adds a Pi 5 4 GB (USD 110) as brain to get two independent USB 3 trees, gadget-mode tethering and header GPIO, keeping the Yellow for the bench.

### Cited Findings
- Sourced prices used below: Pi 5 4 GB USD 110 and 8 GB USD 175 — [Hardware Busters, 2026-11-02](https://hwbusters.com/news/raspberry-pi-5-16gb-price-jumps-to-385-and-the-extra-8gb-now-costs-more-than-a-whole-8gb-board/); Plugable USB3-HUB7BC USD 49.95 — [Plugable](https://plugable.com/products/usb3-hub7bc); Apache 3800 USD 39.99 and 4800 USD 59.99 — [Harbor Freight 3800](https://www.harborfreight.com/3800-weatherproof-protective-case-large-black-63927.html), [Harbor Freight 4800](https://www.harborfreight.com/4800-weatherproof-protective-case-x-large-black-64250.html); cheapest 16 GB N150 mini PC USD 280.79 — [ComputingForGeeks, 2026-11-06](https://computingforgeeks.com/mini-pc-prices/); GT-U7 GNSS about USD 12 (2021 figure) — [Austin's Nerdy Things](https://austinsnerdythings.com/2021/04/19/microsecond-accurate-ntp-with-a-raspberry-pi-and-pps-gps/)

### Inferences
**Tier A: "reuse everything", target under USD 150**
- Brain: spare Home Assistant Yellow with its CM4, Raspberry Pi OS Lite 64-bit using the Yellow DTB (community-documented), NVMe in the M.2 slot only if one is already owned. Rust MCP server here.
- Eyes/hands: one PiKVM V4 Mini on stock PiKVM OS, wired by Ethernet to the Yellow, driven through its HTTP/WebSocket API. Other two PiKVMs stay on the bench/shed.
- USB: one uhubctl-listed switched hub (Plugable USB3-HUB7BC, USD 49.95) on one Yellow port, fed from the 12 V bus; second Yellow port for a Wi-Fi adapter if the CM4 has no radio, otherwise for the highest-rate instrument.
- Power: 12 V LiFePO4 pack as the bus, Yellow and hub from 12 V, one 12 -> 5.1 V buck for the PiKVM, one INA-class sensor on the battery lead. External input by Powerpole from vehicle or the 48 V shed via an isolated converter (converter is outside this tier's budget).
- Case: Apache 3800 (USD 39.99), sheet-stock top panel with PETG printed tiles/brackets.
- Sourced subtotal: USD 89.94 (hub + case). Remaining ~USD 60 must cover battery, buck, sensor, connectors and GNSS; those prices are unverified. A 10-12 Ah pack is the realistic fit at this level, giving roughly 4 h at the low end of the load range.
- Known limits of Tier A: single shared USB 2.0 bus; brain OS is a community-supported configuration; possible lack of onboard Wi-Fi; PPS input depends on an unverified header.

**Tier B: "buy a new brain", target under USD 300**
- Brain: Raspberry Pi 5 4 GB (USD 110) with active cooling; 8 GB (USD 175) only if on-device ML or large RAM captures are required, in which case the battery has to come out of a later budget.
- USB: switched 7-port hub on one USB 3 port (USD 49.95); second USB 3 port direct to the widest SDR or to a second hub later; Pi's USB 2 ports for serial/CAN adapters.
- Power: same 12 V LiFePO4 bus, 12 -> 5.1 V / 5 A buck into the Pi with `usb_max_current_enable=1`; optional Waveshare UPS HAT (E) if brain-only ride-through and a ready-made I2C fuel gauge are wanted.
- Timing: UART+PPS GNSS module on the 40-pin header with chrony.
- Link: onboard Wi-Fi as AP, USB-C gadget Ethernet as the cabled fallback to the Mac.
- Reuse: one PiKVM V4 Mini in the pack as in Tier A; the spare Yellow becomes the bench/dock node (802.15.4 radio, line-out audio, NVMe store) or rides along on Ethernet as an auxiliary.
- Case: Apache 4800 (USD 59.99) if the Yellow rides along, otherwise the 3800.
- Sourced subtotal with 4 GB + hub + 3800: USD 199.93, leaving about USD 100 for a 20 Ah pack, bucks, sensors, GNSS and cooler (prices unverified). With the 8 GB board the same subtotal is USD 264.93.
- An N150 mini PC is excluded at this tier: the box alone is USD 281+, draws more power and still needs a microcontroller for PPS/I2C.

**Decision rule between the tiers:** stay on Tier A unless the instrument list includes a wideband SDR (HackRF/Airspy/USRP class) to be run alongside cameras or a fast logic analyzer; that combination is what the Yellow's single USB 2.0 bus cannot carry.

### Gaps
- Every power-side price and the Pi 5 Active Cooler price are unverified; the tier totals are therefore "sourced subtotal + estimate", not confirmed bills of materials.
- Tier A's viability depends on three checks on the owner's actual hardware: CM4 variant in the spare Yellow (wireless? eMMC?), presence of a usable GPIO header, and tolerance of the 12 V input to LiFePO4 voltages.
- No end-to-end test evidence exists for either configuration with the stated instrument set.
