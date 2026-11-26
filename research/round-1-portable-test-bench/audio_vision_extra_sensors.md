# Microphone arrays, 360 cameras and additional senses for an AI-agent-operated portable field diagnostic pack (state as of 2026-11-25)

Method notes for the report writer (apply to every section):
- Adafruit prices and stock were pulled live from Adafruit's public product API (`https://www.adafruit.com/api/products`) on 2026-11-25; each is linked to its product page by ID. SparkFun prices were read from SparkFun's own catalog search pages the same day; Seeed, DFRobot, Pololu, Waveshare and Atlas Scientific prices from their product pages the same day.
- Rust driver status was read live from the crates.io API on 2026-11-25 (version, last-update date, download counts).
- Linux kernel driver status was read from the mainline `Kconfig` files (torvalds/linux `master`) on 2026-11-25.
- Amazon, eBay sold listings, MPB, Digi-Key, NHBS and miniDSP's own shop blocked automated price reads. Anything that would normally be priced from Amazon (borescopes, microscopes, MQ modules, SuperMini ESP32 boards, PS3 Eye, base Topdon TC001, used Theta V/Z1/SC2) is listed under Gaps with an explicit "unverified" label rather than as a cited finding.
- No facts in these notes came from user memory.

---

## 1. Recommended picks and ranked, priced list (objective)

### Takeaway
Best mic array is the Seeed ReSpeaker XVF3800 USB 4-mic board at USD 54.90: it reports direction of arrival and per-beam speech energy as numbers over a documented USB control interface, has a 64-bit Raspberry Pi host tool, and brings its own 12-LED ring and 5 W speaker amplifier, so it also covers the pack's "voice" and status ring. For vision, a 360 camera is the single most expensive item in this lane and the weakest value for close-up diagnosis; a pan-tilt autofocus camera plus laser pointer and a thermal sensor fits the budget better, with a used Ricoh Theta (V or Z1 for live video; X at USD 142-424 used) as the 360 option if the budget allows.

### Cited Findings

Recommended picks (prices verified 2026-11-25):
- Mic array, best: ReSpeaker XVF3800 USB 4-mic array, USD 54.90, in stock — [Seeed product page](https://www.seeedstudio.com/ReSpeaker-XVF3800-USB-Mic-Array-p-6488.html). The variant with a XIAO ESP32S3 fitted is USD 57.86 — [Seeed product page](https://www.seeedstudio.com/ReSpeaker-XVF3800-4-Mic-Array-With-XIAO-ESP32S3-p-6489.html)
- Mic array, budget: ReSpeaker Lite (USB 2-mic array with onboard processing), USD 24.90, in stock — [Seeed product page](https://www.seeedstudio.com/ReSpeaker-Lite-p-5928.html). A used Sony PS3 Eye (4 mics) was about USD 7.50 on Amazon in 2019 — [CNX Software](https://www.cnx-software.com/2019/08/30/using-sony-ps3-eye-camera-as-an-inexpensive-microphone-array/)
- 360 camera, priced option actually verified: Ricoh Theta X used at Adorama USD 424 (mint), USD 156 (flawed), USD 142 (for parts); new USD 496.95 — [Adorama](https://www.adorama.com/used-ricoh-theta-x-360-degree-225-touchscreen-spherical-camera/p/vdcirctx)
- 360 camera, DIY budget parts: Raspberry Pi Camera Module 3 Wide (120 degrees) USD 38.50 each (Wide NoIR in stock; standard Wide out of stock), standard Camera Module 3 USD 29.25 — [Adafruit 5660](https://www.adafruit.com/product/5660), [Adafruit 5658](https://www.adafruit.com/product/5658), [Adafruit 5657](https://www.adafruit.com/product/5657)
- Pan-tilt alternative: Mini Pan-Tilt Kit assembled with micro servos USD 18.95 — [Adafruit 1967](https://www.adafruit.com/product/1967)

Ranked extras, tier A "must have (cheap, high diagnostic value)":

| Item | Price (USD) | Interface | Linux / Rust driver status | Concrete diagnostic use |
|---|---|---|---|---|
| ICS-43434 I2S MEMS mic (wideband companion to the 16 kHz array) | 8.95 — [Adafruit 6049](https://www.adafruit.com/product/6049) | I2S (needs a Pi I2S overlay or an MCU front end) | Capture through ALSA with [cpal](https://crates.io/crates/cpal) | Full-band spectrograms of pump, compressor, bearing and relay noise |
| LIS3DH 3-axis accelerometer (wired vibration pod) | 4.95 — [Adafruit 2809](https://www.adafruit.com/product/2809) | I2C/SPI | Kernel `IIO_ST_ACCEL_3AXIS` — [accel Kconfig](https://raw.githubusercontent.com/torvalds/linux/master/drivers/iio/accel/Kconfig); crate [lis3dh](https://crates.io/crates/lis3dh) 0.5.0 (updated 2026-09) | Clip to pump, mini split compressor, alternator: imbalance and looseness spectra |
| SHT41 temp/humidity | 5.95 — [Adafruit 5776](https://www.adafruit.com/product/5776) | I2C | Kernel hwmon `SENSORS_SHT4x` — [hwmon Kconfig](https://raw.githubusercontent.com/torvalds/linux/master/drivers/hwmon/Kconfig); crate [sht4x](https://crates.io/crates/sht4x) | Mini split supply/return air delta-T, enclosure condensation risk |
| BME688 temp/humidity/pressure/gas | 19.95 — [Adafruit 5046](https://www.adafruit.com/product/5046) | I2C/SPI | Kernel `BME680` — [chemical Kconfig](https://raw.githubusercontent.com/torvalds/linux/master/drivers/iio/chemical/Kconfig); crate [bosch-bme680](https://crates.io/crates/bosch-bme680) 1.0.4 | Relative VOC "sniff" for burnt electronics, fuel, solvent; barometric altitude |
| DS18B20 1-Wire temperature probes | 3.95 bare — [Adafruit 374](https://www.adafruit.com/product/374); high-temp waterproof 19.95 — [Adafruit 3846](https://www.adafruit.com/product/3846) | 1-Wire | crate [ds18b20](https://crates.io/crates/ds18b20) | Strings on battery cells, water lines, refrigerant lines |
| Thermocouple amplifier MCP9601 or MAX31856 | 14.95 — [Adafruit 5165](https://www.adafruit.com/product/5165); 17.50 — [Adafruit 3263](https://www.adafruit.com/product/3263) | I2C / SPI | Kernel `MCP9600`, `MAX31856` — [temperature Kconfig](https://raw.githubusercontent.com/torvalds/linux/master/drivers/iio/temperature/Kconfig); crates [mcp9600](https://crates.io/crates/mcp9600), [max31856](https://crates.io/crates/max31856) (0.0.1, 2021) | Exhaust manifold, propane burner, flue temperatures |
| MLX90614 non-contact IR thermometer | 18.95 — [SparkFun](https://www.sparkfun.com/infrared-thermometer-mlx90614.html) | I2C (SMBus) | Kernel `MLX90614` — [temperature Kconfig](https://raw.githubusercontent.com/torvalds/linux/master/drivers/iio/temperature/Kconfig); crate [mlx9061x](https://crates.io/crates/mlx9061x) | Spot temperature along the laser pointer axis (breakers, lugs, bearings) |
| MLX90640 32x24 thermal array (budget thermal imager) | 39.95 (110 degree, Qwiic) — [SparkFun](https://www.sparkfun.com/sparkfun-ir-array-breakout-110-degree-fov-mlx90640-qwiic.html); Adafruit's 74.95 breakout is out of stock — [Adafruit 4407](https://www.adafruit.com/product/4407) | I2C | crate [mlx9064x](https://crates.io/crates/mlx9064x) | Hot connections, failed bypass diodes, coil frosting pattern |
| VEML7700 lux or TSL2591 light sensor | 4.95 — [Adafruit 4162](https://www.adafruit.com/product/4162); 6.95 — [Adafruit 1980](https://www.adafruit.com/product/1980) | I2C | Kernel `TSL2591` — [light Kconfig](https://raw.githubusercontent.com/torvalds/linux/master/drivers/iio/light/Kconfig); crates [veml7700](https://crates.io/crates/veml7700), [tsl2591](https://crates.io/crates/tsl2591) | Relative irradiance at the panel plane for "is the array underperforming" checks |
| IR transceiver (receive + transmit) | 5.95 — [Adafruit 5990](https://www.adafruit.com/product/5990); high-power emitter 3.95 — [Adafruit 5639](https://www.adafruit.com/product/5639) | GPIO / rc-core | crates [cir](https://crates.io/crates/cir) ("Linux Infrared Tooling"), [irp](https://crates.io/crates/irp), [infrared](https://crates.io/crates/infrared) | Command the mini split into test modes and learn its remote |
| GNSS receiver | 29.95 (USB) — [Adafruit 4279](https://www.adafruit.com/product/4279); 29.95 (I2C/UART Qwiic) — [Adafruit 4415](https://www.adafruit.com/product/4415); MAX-M10S 45.95 — [SparkFun](https://www.sparkfun.com/sparkfun-gnss-receiver-breakout-max-m10s-qwiic.html) | USB / UART / I2C | crates [nmea](https://crates.io/crates/nmea), [ublox](https://crates.io/crates/ublox), [gpsd_proto](https://crates.io/crates/gpsd_proto) | Geotag and timestamp every reading; disciplined clock |
| Laser diode (pointer) plus line laser | 5.95 — [Adafruit 1054](https://www.adafruit.com/product/1054); line 8.95 — [Adafruit 1057](https://www.adafruit.com/product/1057) | GPIO | n/a | Show the user where the fault is; line laser for geometry by triangulation |
| ESP32-C3 / C6 dev boards for remote pods | 9.95 each — [Adafruit 5337](https://www.adafruit.com/product/5337), [Adafruit 5672](https://www.adafruit.com/product/5672); XIAO ESP32S3 7.49 — [Seeed](https://www.seeedstudio.com/XIAO-ESP32S3-p-5627.html) | Wi-Fi / BLE / ESP-NOW | crate [esp-hal](https://crates.io/crates/esp-hal) 1.2.2 (2026-09) | Scatter clip-on temperature, vibration and light nodes |
| I2C multiplexer for hot-swap sensor bus | 6.95 — [Adafruit 5626](https://www.adafruit.com/product/5626); 4-channel 3.95 — [Adafruit 5664](https://www.adafruit.com/product/5664) | I2C | crate [xca9548a](https://crates.io/crates/xca9548a) 1.0.0 | Isolate address clashes; per-port fault isolation |
| TDS/EC probe for hauled water | 11.80 — [DFRobot](https://www.dfrobot.com/product-1662.html) | Analog | via ADC ([ads1x1x](https://crates.io/crates/ads1x1x)) | Stored-water quality trend, RO filter health |

Tier B "strong add":

| Item | Price (USD) | Interface | Linux / Rust driver status | Concrete diagnostic use |
|---|---|---|---|---|
| SCD41 true CO2 | 49.95 — [Adafruit 5190](https://www.adafruit.com/product/5190); SCD40 44.95 — [Adafruit 5187](https://www.adafruit.com/product/5187) | I2C | Kernel `SCD4X` — [chemical Kconfig](https://raw.githubusercontent.com/torvalds/linux/master/drivers/iio/chemical/Kconfig); crate [scd4x](https://crates.io/crates/scd4x) 0.6.0 (2026-11) | Combustion appliance venting, enclosed-space ventilation |
| VL53L5CX 8x8 ToF array | 19.95 — [Adafruit 6503](https://www.adafruit.com/product/6503), [Pololu 3417](https://www.pololu.com/product/3417); 25.95 — [SparkFun](https://www.sparkfun.com/sparkfun-qwiic-mini-tof-imager-vl53l5cx.html) | I2C | No mainline kernel driver (only `VL53L0X_I2C`, `VL53L1X_I2C`) — [proximity Kconfig](https://raw.githubusercontent.com/torvalds/linux/master/drivers/iio/proximity/Kconfig); crate [vl53l5cx-uld](https://crates.io/crates/vl53l5cx-uld) 0.2.0 (2026-10) | Coarse depth map for scale; distance for IR-thermometer spot-size correction |
| AS7341 10-channel spectral sensor | 18.95 — [Adafruit 4698](https://www.adafruit.com/product/4698); AS7343 19.95 — [Adafruit 6477](https://www.adafruit.com/product/6477) | I2C | No kernel driver in [light Kconfig](https://raw.githubusercontent.com/torvalds/linux/master/drivers/iio/light/Kconfig); crate [as7341](https://crates.io/crates/as7341) 0.1.0 (2026-04); no `as7343` crate | Flame colour (blue vs yellow), LED/lamp spectrum, sunlight vs shade |
| MMC5983MA magnetometer | 18.50 — [SparkFun](https://www.sparkfun.com/sparkfun-micro-magnetometer-mmc5983ma-qwiic.html) | I2C/SPI | Kernel `MMC5983` — [magnetometer Kconfig](https://raw.githubusercontent.com/torvalds/linux/master/drivers/iio/magnetometer/Kconfig); crate [mmc5983ma](https://crates.io/crates/mmc5983ma) (0.1.0-rc1, 2024) | DC current-carrying cable detection, buried ferrous objects, heading |
| TMAG5273 3D Hall sensor (cheap alternative) | 5.95 — [Adafruit 6489](https://www.adafruit.com/product/6489) | I2C | Kernel `TI_TMAG5273` — [magnetometer Kconfig](https://raw.githubusercontent.com/torvalds/linux/master/drivers/iio/magnetometer/Kconfig); crate [tmag5273](https://crates.io/crates/tmag5273) | Relay/solenoid actuation check, DC current presence near a conductor |
| NAU7802 24-bit bridge ADC plus load cell | 5.95 — [Adafruit 4538](https://www.adafruit.com/product/4538); cells 3.95 — [Adafruit 4543](https://www.adafruit.com/product/4543) | I2C | Kernel `NAU7802` and `HX711` — [adc Kconfig](https://raw.githubusercontent.com/torvalds/linux/master/drivers/iio/adc/Kconfig); crates [nau7802](https://crates.io/crates/nau7802) (alpha, 2020), [hx711](https://crates.io/crates/hx711) 0.7.0 | Weigh a propane cylinder for fill level; belt tension; spring force |
| MPRLS ported pressure sensor 0-25 PSI | 29.95 — [Adafruit 3965](https://www.adafruit.com/product/3965) | I2C | Kernel `MPRLS0025PA` — [pressure Kconfig](https://raw.githubusercontent.com/torvalds/linux/master/drivers/iio/pressure/Kconfig) | Water pump pressure-switch cut-in/cut-out via an air-side tap |
| pH kit and turbidity | 29.50 — [DFRobot](https://www.dfrobot.com/product-1025.html); 9.90 — [DFRobot](https://www.dfrobot.com/product-1394.html) | Analog | via ADC | Hauled-water acceptance tests |
| Water flow meters | 9.95 plastic, 24.95 brass — [Adafruit 828](https://www.adafruit.com/product/828), [Adafruit 833](https://www.adafruit.com/product/833) | Pulse | GPIO counter | Pump delivery rate vs spec |
| ISM330DHCX 6-DoF (higher-bandwidth vibration + pack orientation) | 19.95 — [Adafruit 4502](https://www.adafruit.com/product/4502) | I2C/SPI | Kernel `IIO_ST_LSM6DSX` lists ism330dhcx — [st_lsm6dsx Kconfig](https://raw.githubusercontent.com/torvalds/linux/master/drivers/iio/imu/st_lsm6dsx/Kconfig) | Orientation tag for every image; mid-band vibration |
| AudioMoth USB Microphone (ultrasonic) | 79.99, sold out at time of check — [GroupGets](https://groupgets.com/products/audiomoth-usb-microphone) | USB audio | Listed as supported on Linux incl. Raspberry Pi — [GroupGets](https://groupgets.com/products/audiomoth-usb-microphone) | Compressed-air/propane leak hiss, arcing and corona above 20 kHz |
| GM-702B CO and GM-602B H2S gas sensor boards (Qwiic) | 11.95 each — [SparkFun GM-702B](https://www.sparkfun.com/sparkfun-carbon-monoxide-gas-sensor-gm-702b-qwiic.html), [SparkFun GM-602B](https://www.sparkfun.com/sparkfun-hydrogen-sulfide-gas-sensor-gm-602b-qwiic.html) | I2C (Qwiic) | not checked | CO screening near appliances (not a life-safety alarm) |
| MQ-8 hydrogen, MQ-7 CO bare sensors | 7.25, 7.50 — [SparkFun MQ-8](https://www.sparkfun.com/hydrogen-gas-sensor-mq-8.html), [SparkFun MQ-7](https://www.sparkfun.com/carbon-monoxide-sensor-mq-7.html) | Analog (heater) | via ADC | Relative battery off-gassing sniff |
| Split-core CT / non-invasive current clamp | 10.95 (100 A) — [SparkFun](https://www.sparkfun.com/split-core-current-transformer-100a-to-50ma.html); 14.95 (30 A) — [SparkFun](https://www.sparkfun.com/non-invasive-current-sensor-30a.html) | Analog | via ADC / audio input | Pump and compressor start-current signature on a pod |
| Air velocity sensor FS3000 | 19.95 — [SparkFun](https://www.sparkfun.com/renesas-fs3000-air-velocity-sensor-module.html) | I2C | not checked | Mini split discharge airflow, fan health |

Tier C "nice later":

| Item | Price (USD) | Interface | Linux / Rust driver status | Concrete diagnostic use |
|---|---|---|---|---|
| Topdon TC001 Max dual-lens thermal camera | 263 on sale (list 320), Amazon, 2026-11-10 — [PCWorld](https://www.pcworld.com/article/3274688/this-usb-c-thermal-camera-finds-home-heat-leaks-and-its-57-off.html) | USB (UVC) | TC001-family readers: [PyThermalCamera](https://github.com/leswright1977/PyThermalCamera), crate [topdon_thermal_rs](https://crates.io/crates/topdon_thermal_rs) | 256x192 radiometric imaging |
| 2D lidar LD19 | 95.99, marked discontinued — [Waveshare](https://www.waveshare.com/dtof-lidar-ld19.htm); RPLIDAR A1 99.95 — [Adafruit 4010](https://www.adafruit.com/product/4010) | UART | crate [ld06](https://crates.io/crates/ld06) (0.1.1, 2022) | Room/engine-bay floor plan for spatial referencing |
| AS3935 lightning detector | 34.95 — [SparkFun](https://www.sparkfun.com/sparkfun-lightning-detector-as3935.html) | I2C/SPI | Kernel `AS3935` — [proximity Kconfig](https://raw.githubusercontent.com/torvalds/linux/master/drivers/iio/proximity/Kconfig); crate [as3935](https://crates.io/crates/as3935) (pre-release, 2019) | Storm approach before working on arrays/antennas |
| Geiger counter kit | 99.95 — [Adafruit 483](https://www.adafruit.com/product/483) | Pulse | GPIO | Curiosity-grade; low diagnostic value here |
| SM-24 geophone | 69.95 — [SparkFun](https://www.sparkfun.com/geophone-sm-24-with-insulating-disc.html) | Analog | via ADC / audio input | Ground-borne pump/well vibration, footstep or vehicle detection |
| LC29H dual-band GNSS HAT (optional RTK) | 39.99-54.99 — [Waveshare](https://www.waveshare.com/lc29h-gps-hat.htm); LG290P RTK 99.95-189.95 — [SparkFun](https://www.sparkfun.com/sparkpnt-gnss-flex-module-lg290p.html) | UART | NMEA via [nmea](https://crates.io/crates/nmea) | Centimetre-class mapping of buried lines and panels |
| Atlas Scientific EZO pH circuit / kits | 45.99 circuit, 174.99 pH kit, 259.99 conductivity K1.0 kit — [Atlas EZO pH](https://atlas-scientific.com/embedded-solutions/ezo-ph-circuit/), [Atlas EC kit](https://atlas-scientific.com/kits/conductivity-k-1-0-kit/) | I2C/UART | Kernel `ATLAS_PH_SENSOR`, `ATLAS_EZO_SENSOR` — [chemical Kconfig](https://raw.githubusercontent.com/torvalds/linux/master/drivers/iio/chemical/Kconfig) | Lab-grade water chemistry |
| SEN55 particulate/VOC/RH/T; PMSA003I | 36.95 — [SparkFun](https://www.sparkfun.com/sensirion-particle-voc-humidity-and-temperature-sensor-sen55.html); 39.95 — [Adafruit 4505](https://www.adafruit.com/product/4505) | I2C | Kernel `SPS30`, `PMS7003`; crates [sen5x](https://crates.io/crates/sen5x), [pmsx003](https://crates.io/crates/pmsx003) | Dust loading on panels/filters, smoke |
| Anemometer (analog) | 44.95 — [Adafruit 1733](https://www.adafruit.com/product/1733) | Analog | via ADC | Site wind for antenna/array work |
| Raspberry Pi Global Shutter Camera | 50.00 (API stock flag non-positive) — [Adafruit 5702](https://www.adafruit.com/product/5702) | CSI | libcamera | Strobed imaging of belts, fans, pulleys |
| miniDSP UMA-16 v2 (true acoustic camera) | EUR 384.90 incl. camera — [Audiophonics](https://www.audiophonics.fr/en/measurement-microphones/minidsp-uma-16-v2-camera-p-21306.html) | USB UAC2 | Acoular-compatible — [miniDSP](https://www.minidsp.com/products/usb-audio-interface/uma-16-microphone-array) | Sound-source heat map over video |

### Inferences
- A lane build that stays near USD 300: XVF3800 (54.90) + ICS-43434 (8.95) + MLX90640 Qwiic (39.95) + Camera Module 3 (29.25) + pan-tilt kit (18.95) + laser (5.95) + BME688 (19.95) + SHT41 (5.95) + MLX90614 (18.95) + MCP9601 (14.95) + two DS18B20 (7.90) + LIS3DH (4.95) + VEML7700 (4.95) + IR transceiver (5.95) + GNSS (29.95) + two ESP32-C3 pods (19.90) + mux (6.95) + TDS (11.80) comes to about USD 310 before cables and a USB endoscope. Adding any 360 camera pushes the lane past its ceiling unless the thermal imager or GNSS moves to another lane.
- Value per dollar ranks: temperature probes and IR spot thermometer > accelerometer pod > wideband mic > mic array with DOA > low-res thermal array > environmental > spectral/magnetic > 360 camera > lidar.

### Gaps
- No verified November 2026 prices for used Ricoh Theta V, Z1 or SC2: MPB and eBay blocked automated reads, and the one eBay catalogue page reached showed only new-old-stock Theta V at USD 665-713 plus shipping ([eBay](https://www.ebay.com/p/20057853099)), which is not a useful market price. Unverified prior-knowledge estimate: used Theta V roughly USD 150-250, SC2 USD 120-200, Z1 USD 500+.
- Base Topdon TC001 (single-lens) and InfiRay P2 Pro current prices were not obtained; only the TC001 Max sale price is verified.
- Current price and availability of the PS3 Eye, USB borescopes, USB microscopes and MQ breakout modules on Amazon were not verifiable.

---

## 2. Mic arrays: hardware, raw-channel access, DOA/beamforming software, and whether a cheap acoustic camera is realistic

### Takeaway
The XVF3800 board is the strongest fit because the agent can read DOA and beam energy as numbers and reroute the USB output channels to raw microphones over a documented control protocol, but all XMOS voice arrays are capped at 16 kHz, so they locate sounds without characterising machinery spectra; a separate wideband mic is needed. A true acoustic camera needs a 16-plus-mic planar array (about EUR 385), so within budget the realistic substitute is an azimuth marker drawn on the camera image from the 4-mic DOA.

### Cited Findings

ReSpeaker XVF3800 USB 4-mic array (2025):
- Four PDM MEMS mics in a circle, 12 addressable WS2812 LEDs, 3.5 mm output and a JST connector for a 5 W speaker driven by an onboard class-D amplifier — [Seeed wiki](https://wiki.seeedstudio.com/respeaker_xvf3800_introduction/)
- Maximum sample rate 16 kHz; mics rated 64 dBA SNR with a 120 dB acoustic overload point; board diameter 99 mm; launch price USD 49.99 in July 2025 — [CNX Software](https://www.cnx-software.com/2025/07/29/respeaker-xmos-xvf3800-4-mic-array-board-features-esp32-s3-module-works-over-usb/)
- Three USB firmware images are published for v2.1.0: a standard image, a 16 kHz 6-channel image, and a 48 kHz 2-channel image. Seeed's FAQ says the raw microphone channels in the 6-channel firmware may be silent until enabled with ALSA mixer controls (`amixer cset`) — [Seeed wiki](https://wiki.seeedstudio.com/respeaker_xvf3800_introduction/)
- Control runs over USB or I2C through the `xvf_host` tool, shipped for `rpi_64bit`, `linux_x86_64`, `jetson`, `mac_arm64` and `win32`, with a cross-platform Python version; an LED effect mode 4 shows DOA on the ring — [reSpeaker XVF3800 host_control README](https://raw.githubusercontent.com/respeaker/reSpeaker_XVF3800_USB_4MIC_ARRAY/master/host_control/README.md)
- XMOS documents `AEC_AZIMUTH_VALUES` (azimuth of two focused beams, a free-running beam and the auto-selected beam), `AEC_SPENERGY_VALUES` (speech energy per beam), and `AUDIO_MGR_OP_L/R` output routing whose categories include raw microphone data before amplification (sources 0-3), amplified mic data, beamformed outputs and AEC residuals — [XMOS XVF3800 user guide](https://xmos.com/documentation/XM-014888-PC-5/html/modules/fwk_xvf/doc/user_guide/03_using_the_host_application.html)
- Health-style commands available: `VERSION` (confirms control link), GPIO read commands, a full parameter dump, and DFU recovery via a safe-mode factory image — [Seeed wiki](https://wiki.seeedstudio.com/respeaker_xvf3800_introduction/), [XMOS XVF3800 user guide](https://xmos.com/documentation/XM-014888-PC-5/html/modules/fwk_xvf/doc/user_guide/03_using_the_host_application.html)
- Firmware openness: the Seeed GitHub repository README is a stub and shows no licence; firmware is distributed as DFU binaries — [reSpeaker XVF3800 README](https://raw.githubusercontent.com/respeaker/reSpeaker_XVF3800_USB_4MIC_ARRAY/master/README.md), [Seeed wiki](https://wiki.seeedstudio.com/respeaker_xvf3800_introduction/)

ReSpeaker USB Mic Array v2.0 (XVF3000):
- Four ST MP34DT01 mics; factory 6-channel firmware gives channel 0 processed audio, channels 1-4 raw mics, channel 5 playback loopback; 16 kHz maximum; DOA, VAD and DSP parameters exposed through `tuning.py` over USB vendor control transfers; 12 RGB LEDs — [Seeed wiki](https://wiki.seeedstudio.com/ReSpeaker_Mic_Array_v2.0)
- The Seeed store URL linked from that wiki (`ReSpeaker-Mic-Array-v2.0-p-3053`) returned "Page Not Found" on 2026-11-25, as did the store URL for the 6-mic circular HAT kit (`p-3067`) — [Seeed store link](https://www.seeedstudio.com/ReSpeaker-Mic-Array-v2.0-p-3053.html), [Seeed store link](https://www.seeedstudio.com/ReSpeaker-6-Mic-Circular-Array-Kit-for-Raspberry-Pi-p-3067.html)

miniDSP:
- UMA-8: seven MEMS mics in a circle on an XVF3000; RAW firmware exposes 8 channels, DSP firmware gives a 2-channel beamformed output; UAC2 and driverless on Mac and Linux; miniDSP's page showed it out of stock with "a new version is incoming" — [miniDSP UMA-8](https://www.minidsp.com/products/usb-audio-interface/uma-8-microphone-array)
- UMA-8 v2 retail: EUR 188.95, 24-bit at 11-48 kHz, ALSA-compliant on Linux, HID control interface — [SoundImports](https://www.soundimports.eu/en/minidsp-uma-8.html)
- UMA-16 v2: 16 Knowles SPH1668LM4H mics in a uniform rectangular array with a centre hole for a camera, 16 raw channels up to 48 kHz/24-bit, UAC2, positioned for acoustic imaging and listed as Acoular-compatible — [miniDSP UMA-16](https://www.minidsp.com/products/usb-audio-interface/uma-16-microphone-array)
- UMA-16 v2 retail with camera: EUR 384.90 — [Audiophonics](https://www.audiophonics.fr/en/measurement-microphones/minidsp-uma-16-v2-camera-p-21306.html)

Cheap and DIY options:
- PS3 Eye: four mics, about USD 7.50 in 2019, appears as a 4-channel ALSA capture device; a reader config places the mics on one axis about 2 cm apart; reviewers found it more directional than a circular array and fiddly to configure — [CNX Software](https://www.cnx-software.com/2019/08/30/using-sony-ps3-eye-camera-as-an-inexpensive-microphone-array/)
- DIY element prices: ICS-43434 I2S breakout USD 8.95, SPH0645 I2S breakout USD 6.95, PDM MEMS breakout USD 4.95 — [Adafruit 6049](https://www.adafruit.com/product/6049), [Adafruit 3421](https://www.adafruit.com/product/3421), [Adafruit 4346](https://www.adafruit.com/product/4346)
- Raspberry Pi RP2040/RP2350 and ESP32 have maintained async Rust HALs for building a custom array front end — [embassy-rp](https://crates.io/crates/embassy-rp), [esp-hal](https://crates.io/crates/esp-hal)

Software:
- ODAS: MIT-licensed C library for sound source localisation, tracking, separation and post-filtering, written to run on low-cost embedded hardware; has a web GUI companion — [ODAS README](https://raw.githubusercontent.com/introlab/odas/master/README.md)
- Acoular: BSD-licensed Python beamforming package offering delay-and-sum, Capon, MUSIC, CLEAN-SC, DAMAS and time-domain methods that output acoustic "photographs" — [Acoular README](https://raw.githubusercontent.com/acoular/acoular/master/README.md)
- pyroomacoustics: Python package with reference implementations of STFT, beamforming and direction-finding algorithms plus a room simulator — [pyroomacoustics README](https://raw.githubusercontent.com/LCAV/pyroomacoustics/master/README.rst)
- whisper.cpp: dependency-free C/C++ Whisper inference with ARM NEON optimisation, CPU-only mode and built-in voice activity detection — [whisper.cpp README](https://raw.githubusercontent.com/ggml-org/whisper.cpp/master/README.md)
- openWakeWord: open-source wake-word library with pretrained models and a documented custom-model training path; last tagged release noted in its README is v0.6.0 (February 2024) — [openWakeWord README](https://raw.githubusercontent.com/dscripka/openWakeWord/main/README.md)
- BirdNET-Analyzer: maintained classifier with Linux `.deb`/AppImage builds; models published as BirdNET 2.4 and 3.0 — [BirdNET-Analyzer README](https://raw.githubusercontent.com/birdnet-team/BirdNET-Analyzer/main/README.md)
- Rust building blocks, all current on crates.io: audio capture [cpal](https://crates.io/crates/cpal) 0.18.2 (2026-08); FFT [rustfft](https://crates.io/crates/rustfft) 6.4.1 and [realfft](https://crates.io/crates/realfft) 3.5.1 (2026-10); [spectrum-analyzer](https://crates.io/crates/spectrum-analyzer) 2.0.0 (2026-09); mel spectrograms [mel_spec](https://crates.io/crates/mel_spec) 0.5.0 (2026-10); Whisper bindings [whisper-rs](https://crates.io/crates/whisper-rs) 0.16.0; sherpa-onnx bindings [sherpa-rs](https://crates.io/crates/sherpa-rs); ONNX inference [ort](https://crates.io/crates/ort) and pure-Rust [tract-onnx](https://crates.io/crates/tract-onnx) (2026-11); WAV I/O [hound](https://crates.io/crates/hound); pitch [pitch-detection](https://crates.io/crates/pitch-detection); synthesis for test tones [fundsp](https://crates.io/crates/fundsp)

Acoustic camera feasibility:
- A hobby acoustic camera was built from a MATRIX Creator (8 MEMS mics, FPGA front end), a Raspberry Pi 3B+ and a Pi camera using time-domain beamforming over a 21 x 16 scan grid at 48 kHz; the author reports quantisation error from nearest-sample delays and gives no frame rate — [Hoi-Jeon/Acoustic-Camera](https://github.com/Hoi-Jeon/Acoustic-Camera)
- miniDSP's UMA-16 application note demonstrates locating the dominant source and overlaying it on live video — [miniDSP UMA-16](https://www.minidsp.com/products/usb-audio-interface/uma-16-microphone-array)

### Inferences
- With every XMOS firmware image limited to 16 kHz for multi-channel capture, the array's usable band tops out at 8 kHz. Bearing whine, relay chatter and arcing have diagnostic content above that, so the array should be treated as the "where is it" sensor and a 48 kHz-plus I2S or USB mic as the "what is it" sensor.
- Four mics on a roughly 10 cm circle give azimuth only, with coarse resolution at low frequencies (a 60 Hz transformer hum is not localisable). Elevation and a 2D heat map need a planar array of 16 or more elements.
- A budget "acoustic camera" is still achievable in a weaker form: if the mic array and the camera share a known orientation, a DOA azimuth maps to an image column (in an equirectangular panorama it is literally the x-coordinate), so the pack can draw a marker band on the picture it sends to the agent.
- Because the XVF3800 exposes raw-mic routing, the pack can run its own GCC-PHAT or ODAS pipeline on raw channels and cross-check the chip's built-in DOA, which doubles as a sensor health check.
- A speaker chirp played through the board's amplifier and recorded on all four raw channels gives a per-mic gain and delay self-test with no extra hardware.
- For feeding the agent: mel or log-frequency spectrogram PNGs plus a small JSON summary (dominant peaks, harmonic series fit, DOA, event timestamps) are the practical outputs; Whisper handles the user's spoken notes.

### Gaps
- The exact channel map of the XVF3800 16 kHz 6-channel firmware is not published on the wiki page or README reached; the wiki only implies raw mic channels are present. Needs a bench check with `arecord -c 6`.
- No source found confirming XVF3800 firmware source availability; treat the DSP firmware as closed binaries with an open, documented control interface.
- Sipeed 6+1 array, Matrix Voice status, ReSpeaker 6-mic HAT current availability, Teensy audio and XMOS DIY arrays were not researched to a citable standard. Prior knowledge (unverified): Matrix Labs products are discontinued; the Seeed Pi HAT arrays depend on an out-of-tree `seeed-voicecard` kernel driver that has lagged behind new Raspberry Pi kernels, which is a reason to prefer USB audio class devices.
- No maintained Rust crate for multi-channel DOA/beamforming was identified; the search covered FFT and ML crates only.
- No BirdNET-style pretrained classifier for machinery faults was identified; general audio-tagging models would have to be run through ONNX.

---

## 3. Ultrasonic and structure-borne sound

### Takeaway
The only open, ready-made ultrasonic capture device verified is the AudioMoth USB Microphone (384 kHz, MIT-licensed firmware, USD 79.99 but sold out at GroupGets); for vibration, cheap I2C/SPI accelerometers with mainline Linux drivers cover most machinery faults, while the purpose-built wideband IIS3DWB has neither a mainline kernel driver nor a Rust crate.

### Cited Findings
- AudioMoth USB Microphone: USD 79.99, records up to 384 kHz, stated to work on Windows, macOS and Linux including Raspberry Pi; listed as sold out on 2026-11-25 — [GroupGets](https://groupgets.com/products/audiomoth-usb-microphone)
- Its firmware is published under the MIT licence by Open Acoustic Devices — [AudioMoth-USB-Microphone README](https://raw.githubusercontent.com/OpenAcousticDevices/AudioMoth-USB-Microphone/main/README.md)
- Dodotronic Ultramic: MEMS ultrasonic sensor with selectable 192, 200 or 250 kHz sampling, no driver installation, listed as compatible with Ubuntu and Debian — [NHBS](https://nhbs.com/ultramic-usb-ultrasound-microphone)
- Accelerometer breakouts: LIS3DH USD 4.95, ADXL343 USD 5.95, ADXL345 USD 17.50, MPU-6050 USD 12.95, LSM6DSOX USD 11.95, ISM330DHCX USD 19.95, ICM-20948 USD 19.95 — [Adafruit 2809](https://www.adafruit.com/product/2809), [Adafruit 4097](https://www.adafruit.com/product/4097), [Adafruit 1231](https://www.adafruit.com/product/1231), [Adafruit 3886](https://www.adafruit.com/product/3886), [Adafruit 4438](https://www.adafruit.com/product/4438), [Adafruit 4502](https://www.adafruit.com/product/4502), [Adafruit 4554](https://www.adafruit.com/product/4554)
- KX134 wide-range accelerometer breakout USD 35.95 — [SparkFun](https://www.sparkfun.com/sparkfun-triple-axis-accelerometer-breakout-kx134-qwiic.html)
- Mainline Linux IIO has drivers for ADXL345, ADXL355, ADXL372, ADXL380, KX022A-family and the ST 3-axis family, but no IIS3DWB entry — [accel Kconfig](https://raw.githubusercontent.com/torvalds/linux/master/drivers/iio/accel/Kconfig)
- Mainline IMU drivers exist for MPU-6050 (`INV_MPU6050`), ICM-426xx (`INV_ICM42600`), ICM-456xx and the LSM6DSx family — [inv_mpu6050 Kconfig](https://raw.githubusercontent.com/torvalds/linux/master/drivers/iio/imu/inv_mpu6050/Kconfig), [inv_icm42600 Kconfig](https://raw.githubusercontent.com/torvalds/linux/master/drivers/iio/imu/inv_icm42600/Kconfig), [st_lsm6dsx Kconfig](https://raw.githubusercontent.com/torvalds/linux/master/drivers/iio/imu/st_lsm6dsx/Kconfig)
- Rust crates: [lis3dh](https://crates.io/crates/lis3dh) 0.5.0 (2026-09), [adxl343](https://crates.io/crates/adxl343) 0.8.0, [adxl355](https://crates.io/crates/adxl355) 0.2.3 (2020), [icm426xx](https://crates.io/crates/icm426xx) 0.4.0 (2025-10), [mpu6050](https://crates.io/crates/mpu6050), [lsm6dso](https://crates.io/crates/lsm6dso); crates.io has no crate named `iis3dwb` — [crates.io search](https://crates.io/crates/iis3dwb)
- ST sells the IIS3DWB as the STEVAL-MKI208V1K adapter kit; ST's page showed no price and no distributor stock — [ST](https://www.st.com/en/evaluation-tools/steval-mki208v1k.html)
- Structure-borne pickups: enclosed piezo element USD 0.95; 600 mm piezoelectric ribbon sensor USD 24.95; SM-24 geophone USD 69.95 — [Adafruit 1739](https://www.adafruit.com/product/1739), [Adafruit 4931](https://www.adafruit.com/product/4931), [SparkFun](https://www.sparkfun.com/geophone-sm-24-with-insulating-disc.html)
- Linux IIO sensors can be read from Rust through the [industrial-io](https://crates.io/crates/industrial-io) crate (0.6.1, 2025-09)

### Inferences
- For a remote wired pod, an SPI accelerometer on a short cable to an ESP32-C3 that streams blocks over Wi-Fi avoids long SPI runs; the LIS3DH at a few kHz output rate is adequate for imbalance, misalignment and looseness on pumps and compressors but not for early bearing defects, which need 5-10 kHz bandwidth.
- A piezo disc epoxied to a magnet and fed into a spare audio ADC channel is the cheapest wideband contact mic and reuses the entire audio spectrogram pipeline.
- Given the AudioMoth stock problem, the fallback is a 192 kHz USB audio interface plus an ultrasonic-capable analogue MEMS mic; this is a build task, not an off-the-shelf purchase.

### Gaps
- Frequency response and pricing for Knowles SPU0410LR5H, Infineon IM73A135/IM69D, TDK ICS-41350 and suitable breakouts were not verified (Digi-Key blocked). Prior knowledge (unverified): the SPU0410LR5H is the analogue MEMS element commonly used in bat detectors and is usable to about 80 kHz.
- Dodotronic Ultramic and Pettersson prices were not shown on the pages reached.
- No citable price for mechanic's "chassis ears" kits, clip-on contact mics or stethoscopes.
- ADXL355/ADXL357 and ICM-42688 breakout prices were not found at Adafruit or SparkFun; SparkFun's search returned no ICM-42688 or ADXL355 breakout.
- No source located for a DIY heterodyne or direct-sampling leak detector design specific to propane or arcing.

---

## 4. 360 cameras with open control and live streaming on Linux; stitching; feeding 360 imagery to a vision LLM; pan-tilt vs 360

### Takeaway
Ricoh Theta is the only 360 family with a documented open path on Linux: the V and Z1 stream stitched 4K H.264 over USB through community libuvc builds, but a Raspberry Pi handles that stream poorly, so on this pack the Theta is best used for stills via its API. Insta360's SDK is USB-desktop oriented and cannot drive webcam mode, GoPro exposes USB preview through Open GoPro but 360-model support was not confirmed, and research on multimodal models shows they struggle with raw 360 panoramas, which argues for serving perspective crops.

### Cited Findings

Ricoh Theta:
- Theta V and Z1 stream up to 3840x1920 at 30 fps with in-camera stitching over USB, Wi-Fi or Ethernet; the Z1 can charge while streaming; the SC2 is listed as not supporting streaming — [Theta community document](https://theta360developers.github.io/community-document/)
- The Theta V supports Wi-Fi client mode (joins a router); Wi-Fi streaming on the V uses a plug-in, and an RTSP plug-in exists for the V — [Theta community document](https://theta360developers.github.io/community-document/)
- Linux USB streaming needs Ricoh's patched libuvc (`libuvc-theta`) or the `gstthetauvc` GStreamer source; `gstthetauvc` offers 2K (1920x960) and 4K (3840x1920) H.264 modes for V/Z1 and avoids a v4l2loopback device — [gstthetauvc README](https://raw.githubusercontent.com/nickel110/gstthetauvc/master/README.md), [libuvc-theta README](https://raw.githubusercontent.com/ricohapi/libuvc-theta/theta_uvc/README.md)
- The community's Linux guide (updated March 2025) covers V, Z1 and X, reports about 360 ms latency with `gstthetauvc`, and says a Raspberry Pi works well with the USB API but gives a poor 4K streaming experience even on a Pi 4 because its H.264 hardware decoder does not handle 4K — [theta360.guide Linux Getting Started](https://community.theta360.guide/t/linux-getting-started/10945), [theta-linux equipment notes](https://codetricity.github.io/theta-linux/equipment)
- Theta X on Linux enumerates as a UVC 1.5 device (USB ID 05ca:2717); a troubleshooting thread shows users failing to get a video node until desktop auto-mounting is disabled and the camera is switched to live mode over PTP, with a Jetson-experienced moderator advising 2K on a Pi — [theta360.guide Theta X thread](https://community.theta360.guide/t/theta-x-usb-live-stream-on-linux-with-opencv/9992)
- Theta X pricing: USD 496.95 new; used USD 424 (mint), USD 156 (flawed), USD 142 (parts) — [Adorama](https://www.adorama.com/used-ricoh-theta-x-360-degree-225-touchscreen-spherical-camera/p/vdcirctx)

Insta360:
- Insta360 offers a Camera SDK and a Media SDK; the desktop SDK connects only over USB, while Android adds Wi-Fi and Bluetooth; cameras also answer the Open Spherical Camera (OSC) API, which lacks preview, live streaming, activation and firmware-update functions — [Insta360 developer integration guide](https://onlinemanual.insta360.com/developer/en-us/resource/integration)
- The SDK cannot control webcam mode; X5, X4 and X4 Air expose a "USB camera" option in their USB mode menu — [Insta360 developer integration guide](https://onlinemanual.insta360.com/developer/en-us/resource/integration)
- The reverse-engineered Linux projects found (`insta360-linux`, the vrwallace controller) target the Insta360 Link gimbal webcams through UVC extension units, not the X-series 360 cameras — [EdenCoder/insta360-linux README](https://raw.githubusercontent.com/EdenCoder/insta360-linux/main/README.md)

GoPro:
- Per the Open GoPro FAQ, BLE handles discovery and setup, the camera acts as its own Wi-Fi access point rather than joining another network, and over USB it offers webcam and preview streaming via a network (NCM) interface at 720p or 1080p — [Open GoPro FAQ](https://gopro.github.io/OpenGoPro/faq)
- A Python SDK and a Kotlin SDK are the maintained reference clients — [Open GoPro repository README](https://raw.githubusercontent.com/gopro/OpenGoPro/main/README.md)

DIY dual-camera 360:
- StereoPi demonstrated a 360 panorama from two back-to-back 200-degree fisheye Pi cameras on a Compute Module 3, stitched with a Hugin template and `nona`/`enblend`; it is a batch process taking about 50 seconds per image pair and needs the cameras rigidly fixed — [StereoPi project log](https://hackaday.io/project/162954-stereopi-diy-stereoscopic-camera-with-raspberry/log/157524-stitching-360-panorama-with-stereopi)
- The Raspberry Pi 5 has two CSI camera ports selected with `--camera 0/1`; readers report two identical sensors sometimes showing as one until overlays are set by hand — [The Pi Hut tutorial](https://thepihut.com/blogs/raspberry-pi-tutorials/how-to-use-two-camera-modules-with-raspberry-pi-5)

360 imagery and vision LLMs:
- A 2026 benchmark of seven multimodal LLMs on 7K 360-degree images (360Bench) found clear shortcomings in 360 perception; its training-free remedy applies step-specific spherical image transformations and merges results into a scene graph, which consistently improved the base model — [arXiv 2603.16179](https://arxiv.org/abs/2603.16179)

Rust plumbing:
- Camera and video crates current on crates.io: [v4l](https://crates.io/crates/v4l), [nokhwa](https://crates.io/crates/nokhwa) 0.10.11 (2026-05), [gstreamer](https://crates.io/crates/gstreamer) 0.25.4 (2026-09), [opencv](https://crates.io/crates/opencv) 0.102.0 (2026-11); USB control via [nusb](https://crates.io/crates/nusb)

### Inferences
- For an agent that reasons over stills, live 4K video is unnecessary. Triggering a still over the Theta USB API (PTP) and downloading the stitched equirectangular JPEG sidesteps the Pi's H.264 decode weakness and keeps the Pi's single Wi-Fi radio free for the uplink.
- GoPro's access-point-only Wi-Fi and Theta SC2-class cameras that lack client mode both conflict with a pack whose Wi-Fi is the agent uplink; USB control is the safe design assumption for any 360 camera.
- Serving the panorama to the agent as an MCP "virtual PTZ" tool (inputs yaw, pitch, field of view; output a rectilinear crop) plus one low-resolution overview strip follows the benchmark's finding that raw equirectangular input is handled badly, and keeps token cost bounded. A cubemap of six 90-degree faces is the simplest fixed tiling.
- Pan-tilt versus 360: diagnosis usually needs pixels on a small target (a label, a terminal, a leak), which a USD 29 autofocus camera on a USD 19 pan-tilt delivers better than any consumer 360 camera, and the same mount aims the laser pointer and the IR thermometer. A 360 camera's value is context and "what changed around me". Recommended order: pan-tilt first, 360 second.
- A pan-tilt sweep can itself produce a stitched panorama of a static scene with Hugin tools, giving 360 coverage without a 360 camera at the cost of tens of seconds.
- Two 120-degree Camera Module 3 Wide units do not cover a full sphere; a DIY 360 needs 190-degree-plus fisheye modules as in the StereoPi build.

### Gaps
- Whether GoPro MAX or MAX 2 are in Open GoPro's supported-camera list could not be confirmed; the specification page did not render for automated reading. The FAQ's access-point-only statement may also predate newer firmware features for joining a home network (prior knowledge, unverified).
- Insta360 SDK platform list (Linux, ARM64) and whether access requires an application were not stated on the page reached. Prior knowledge (unverified): the SDK is distributed after an application form and has historically shipped Linux x86-64 builds, with no official ARM64 Linux build.
- Theta SC2 Wi-Fi API capability (stills and MJPEG live preview through the OSC-style WebAPI) and Theta plug-in support by model were not verified in this session.
- Raspberry Pi 5 performance decoding Theta 2K/4K H.264 is not documented in the sources reached (the Pi notes predate or omit the Pi 5).
- Kandao QooCam, ELP dual-lens USB modules, Arducam multi-camera adapters and Luxonis OAK depth cameras were not researched.
- No verified source for ffmpeg `v360` filter usage on ARM64, though it is the standard tool for equirectangular-to-cubemap or perspective conversion (prior knowledge).

---

## 5. Other vision: borescope, microscope, thermal, high-speed, illumination, lidar/ToF, AprilTags

### Takeaway
The Topdon TC001 / InfiRay P2 Pro family is the open thermal choice because it enumerates as a plain UVC device with reverse-engineered radiometric decoding in Python and Rust, and the 32x24 MLX90640 at about USD 40 is the budget fallback; generic USB endoscopes are also UVC but often output only uncompressed 640x480.

### Cited Findings
- InfiRay-type phone thermal cameras are picked up by Linux as standard USB video devices delivering two streams, one carrying raw thermal data; the Topdon TC001 is based on the InfiRay P2 Pro design — [Hackaday](https://hackaday.com/2023/07/11/phone-thermal-cameras-get-open-source-desktop-tools)
- PyThermalCamera reads the TC001's 256x192 feed on Debian and Raspberry Pi without sending any commands to the camera, and reports centre, min, max and average scene temperatures — [PyThermalCamera README](https://raw.githubusercontent.com/leswright1977/PyThermalCamera/main/README.md)
- P2Pro-Viewer reverse-engineered the vendor command protocol (pseudo-colour done; shutter control, range switching and emissivity still on its roadmap) and labels its Linux support as untested — [P2Pro-Viewer README](https://raw.githubusercontent.com/LeoDJ/P2Pro-Viewer/main/README.md)
- A Rust crate for the TC001 on Linux exists: `topdon_thermal_rs` 0.2.1, last updated August 2025, about 1,800 downloads — [crates.io](https://crates.io/crates/topdon_thermal_rs)
- Topdon TC001 Max (dual-lens) was USD 263 on Amazon on 2026-11-10 against a USD 320 list price — [PCWorld](https://www.pcworld.com/article/3274688/this-usb-c-thermal-camera-finds-home-heat-leaks-and-its-57-off.html)
- Low-resolution thermal arrays: MLX90640 Qwiic 110-degree USD 39.95, bare 55-degree module USD 35.95 — [SparkFun](https://www.sparkfun.com/sparkfun-ir-array-breakout-110-degree-fov-mlx90640-qwiic.html), [SparkFun](https://www.sparkfun.com/ir-array-sensor-55-degree-fov-mlx90640.html); AMG8833 8x8 USD 44.95 — [Adafruit 3538](https://www.adafruit.com/product/3538); FLIR Lepton breakout board (module extra) USD 51.35 — [SparkFun](https://www.sparkfun.com/flir-lepton-breakout-board-v2.html)
- A teardown of a cheap USB endoscope found a standard UVC device giving only uncompressed YUY2 up to 640x480 at 30 fps, standard UVC image controls, and LEDs dimmed by a hardware thumbwheel rather than software — [improwis reverse-engineering notes](https://www.improwis.com/projects/reveng_USBendoscope)
- ToF arrays: VL53L5CX 8x8 USD 19.95 (Adafruit, Pololu) to USD 32.50 (SparkFun full-size); VL53L7CX 8x8 USD 19.95; single-zone VL53L1X USD 14.95 — [Adafruit 6503](https://www.adafruit.com/product/6503), [Pololu 3417](https://www.pololu.com/product/3417), [SparkFun](https://www.sparkfun.com/sparkfun-qwiic-tof-imager-vl53l5cx.html), [Adafruit 6574](https://www.adafruit.com/product/6574), [Adafruit 3967](https://www.adafruit.com/product/3967)
- 2D lidar: Waveshare lists the LD19 at USD 95.99 and marks it discontinued; Waveshare also documents a D500 kit built on the LDROBOT STL-19P — [Waveshare LD19](https://www.waveshare.com/dtof-lidar-ld19.htm), [Waveshare D500 wiki](https://www.waveshare.com/wiki/D500_LiDAR_Kit); Slamtec RPLIDAR A1 USD 99.95 — [Adafruit 4010](https://www.adafruit.com/product/4010)
- AprilTag detection is available in Rust through bindings to the reference C library — [apriltag crate](https://crates.io/crates/apriltag)
- Raspberry Pi Global Shutter Camera USD 50.00; Camera Module 3 NoIR variants USD 25.00-38.50 — [Adafruit 5702](https://www.adafruit.com/product/5702), [Adafruit 5659](https://www.adafruit.com/product/5659), [Adafruit 5660](https://www.adafruit.com/product/5660)
- Illumination and structured light parts: 5 mW red line laser USD 8.95, cross laser USD 8.95, TTL-modulated laser USD 18.95, 25-pack 5 mm IR LEDs USD 7.95, 12-LED NeoPixel ring USD 8.95 — [Adafruit 1057](https://www.adafruit.com/product/1057), [Adafruit 1058](https://www.adafruit.com/product/1058), [Adafruit 1056](https://www.adafruit.com/product/1056), [Adafruit 388](https://www.adafruit.com/product/388), [Adafruit 1643](https://www.adafruit.com/product/1643)

### Inferences
- Because the TC001 reader needs no control commands, the pack can treat it as a V4L2 source and decode temperatures itself; frame arrival and a plausible min/max range form a basic health check.
- A printed AprilTag card with a scale bar and colour patches placed in the scene gives the agent metric scale, pose and white balance in one cheap accessory.
- Strobing the NeoPixel ring or the TTL laser at a commanded frequency turns the rolling-shutter Pi camera into a stroboscopic tachometer without buying the global-shutter camera.
- A 2D lidar is poor value for diagnosis; the 8x8 ToF array gives enough depth to scale thermal and visual observations.

### Gaps
- No verified current prices for USB borescopes (single, dual-lens, articulating) or USB microscopes; Adafruit's USB microscope (USD 79.95) is flagged unavailable. Prior knowledge (unverified): 1080p dual-lens UVC endoscopes sell for roughly USD 25-45 and basic UVC microscopes for USD 20-40 on Amazon; many "Wi-Fi" endoscopes require a phone app and should be avoided in favour of plain USB UVC models.
- Whether any articulating borescope exposes articulation control over USB is unknown; those seen are mechanically articulated by hand.
- No kernel V4L2 radiometric driver was identified for the TC001/P2 Pro; decoding is user-space.
- VL53L8 availability and price were not checked.

---

## 6. Environmental and chemical sensing

### Takeaway
Temperature, humidity, pressure, CO2, particulate, light and thermocouple sensing are cheap, I2C, and mostly have both mainline Linux IIO/hwmon drivers and Rust crates; combustible-gas and CO sensing is the weak spot, where low-cost metal-oxide parts behave as relative indicators that drift with humidity and temperature, and no affordable open refrigerant sensor was identified.

### Cited Findings
- Prices: BME280 USD 14.95, BME680 USD 18.95, BME688 USD 19.95, SHT41 USD 5.95, SHT45 USD 12.50, SCD40 USD 44.95, SCD41 USD 49.95, SCD30 USD 58.95, STCC4+SHT41 CO2 board USD 27.50, SGP41 USD 19.95, ENS160 USD 21.95, PMSA003I USD 39.95-44.95 — [Adafruit 2652](https://www.adafruit.com/product/2652), [Adafruit 3660](https://www.adafruit.com/product/3660), [Adafruit 5046](https://www.adafruit.com/product/5046), [Adafruit 5776](https://www.adafruit.com/product/5776), [Adafruit 5665](https://www.adafruit.com/product/5665), [Adafruit 5187](https://www.adafruit.com/product/5187), [Adafruit 5190](https://www.adafruit.com/product/5190), [Adafruit 4867](https://www.adafruit.com/product/4867), [Adafruit 6478](https://www.adafruit.com/product/6478), [Adafruit 6455](https://www.adafruit.com/product/6455), [Adafruit 5606](https://www.adafruit.com/product/5606), [Adafruit 4505](https://www.adafruit.com/product/4505)
- Sensirion SEN55 module USD 36.95 — [SparkFun](https://www.sparkfun.com/sensirion-particle-voc-humidity-and-temperature-sensor-sen55.html); SEN6x adapter breakout USD 4.50 — [Adafruit 6331](https://www.adafruit.com/product/6331)
- Mainline IIO chemical drivers include BME680, SCD30, SCD4X, SGP30, SGP40, ENS160, CCS811, PMS7003, SPS30, AGS02MA, MH-Z19B and the Atlas pH and EZO families — [chemical Kconfig](https://raw.githubusercontent.com/torvalds/linux/master/drivers/iio/chemical/Kconfig)
- hwmon carries SHT3x, SHT4x, AHT10, INA2xx, INA238 and INA3221 drivers — [hwmon Kconfig](https://raw.githubusercontent.com/torvalds/linux/master/drivers/hwmon/Kconfig)
- Rust crates: [bme280](https://crates.io/crates/bme280) 0.5.1, [bosch-bme680](https://crates.io/crates/bosch-bme680) 1.0.4, [scd4x](https://crates.io/crates/scd4x) 0.6.0 (2026-11), [sht4x](https://crates.io/crates/sht4x), [sen5x](https://crates.io/crates/sen5x) (2026-04), [sen6x](https://crates.io/crates/sen6x), [sgp40](https://crates.io/crates/sgp40) 1.0.0, [sgp41](https://crates.io/crates/sgp41) (2021), [ens160](https://crates.io/crates/ens160), [pmsx003](https://crates.io/crates/pmsx003) 1.0.0 (2025)
- Combustible and toxic gas parts priced: MQ-8 hydrogen USD 7.25, MQ-7 CO USD 7.50, MEMS GM-702B CO Qwiic board USD 11.95, GM-602B H2S Qwiic board USD 11.95, MiCS5524 CO/alcohol/VOC USD 14.95, AGS02MA TVOC USD 4.95 — [SparkFun MQ-8](https://www.sparkfun.com/hydrogen-gas-sensor-mq-8.html), [SparkFun MQ-7](https://www.sparkfun.com/carbon-monoxide-sensor-mq-7.html), [SparkFun GM-702B](https://www.sparkfun.com/sparkfun-carbon-monoxide-gas-sensor-gm-702b-qwiic.html), [SparkFun GM-602B](https://www.sparkfun.com/sparkfun-hydrogen-sulfide-gas-sensor-gm-602b-qwiic.html), [Adafruit 3199](https://www.adafruit.com/product/3199), [Adafruit 5593](https://www.adafruit.com/product/5593)
- A peer-reviewed field test of the Figaro TGS2611-E00 for methane found water vapour, temperature and run time explain most of its baseline (R-squared above 0.9), that the response drifts enough to need re-fitting roughly every 10 days, and that it could separate 2 ppm from 10 ppm but not 2 from 3 ppm; the unfiltered TGS2600 did not respond to methane in that range — [Atmospheric Measurement Techniques 17, 2103 (2024)](https://amt.copernicus.org/articles/17/2103/2024/)
- Light and spectral: VEML7700 USD 4.95, TSL2591 USD 6.95, BH1750 USD 4.50, LTR390 UV USD 4.95, AS7331 UVA/B/C USD 22.50 (out of stock), AS7341 USD 18.95, AS7343 USD 19.95 (Adafruit) / USD 21.95 (SparkFun), AS7265x 18-channel triad USD 69.95 — [Adafruit 4162](https://www.adafruit.com/product/4162), [Adafruit 1980](https://www.adafruit.com/product/1980), [Adafruit 4681](https://www.adafruit.com/product/4681), [Adafruit 4831](https://www.adafruit.com/product/4831), [Adafruit 6476](https://www.adafruit.com/product/6476), [Adafruit 4698](https://www.adafruit.com/product/4698), [Adafruit 6477](https://www.adafruit.com/product/6477), [SparkFun AS7343](https://www.sparkfun.com/sparkfun-spectral-sensor-as7343-qwiic.html), [SparkFun AS7265x](https://www.sparkfun.com/sparkfun-triad-spectroscopy-sensor-as7265x-qwiic.html)
- Mainline light drivers include TSL2591, BH1750, LTR390, VEML6030, VEML6075 and AS73211, with no AS7341/AS7343 entry — [light Kconfig](https://raw.githubusercontent.com/torvalds/linux/master/drivers/iio/light/Kconfig)
- Lightning: SparkFun AS3935 breakout USD 34.95; mainline driver `AS3935` — [SparkFun](https://www.sparkfun.com/sparkfun-lightning-detector-as3935.html), [proximity Kconfig](https://raw.githubusercontent.com/torvalds/linux/master/drivers/iio/proximity/Kconfig)
- Radiation: Geiger counter kit USD 99.95 — [Adafruit 483](https://www.adafruit.com/product/483); RadiaCode-10x detectors have an MIT-licensed Python library with USB and Bluetooth connectivity and spectrum acquisition — [cdump/radiacode README](https://raw.githubusercontent.com/cdump/radiacode/master/README.md)
- Wind: analogue-output anemometer USD 44.95 — [Adafruit 1733](https://www.adafruit.com/product/1733)
- Non-contact IR: MLX90614 USD 18.95 (SparkFun), MLX90632 USD 17.50 (Adafruit); mainline drivers `MLX90614`, `MLX90632`, `MLX90635` — [SparkFun](https://www.sparkfun.com/infrared-thermometer-mlx90614.html), [Adafruit 6403](https://www.adafruit.com/product/6403), [temperature Kconfig](https://raw.githubusercontent.com/torvalds/linux/master/drivers/iio/temperature/Kconfig)
- Thermocouples and RTDs: MAX31856 universal amplifier USD 17.50, MCP9600 USD 15.95, MCP9601 USD 14.95, MAX31865 PT100/PT1000 amplifier USD 14.95; mainline drivers `MAXIM_THERMOCOUPLE`, `MAX31856`, `MAX31865`, `MCP9600` — [Adafruit 3263](https://www.adafruit.com/product/3263), [Adafruit 4101](https://www.adafruit.com/product/4101), [Adafruit 5165](https://www.adafruit.com/product/5165), [Adafruit 3328](https://www.adafruit.com/product/3328), [temperature Kconfig](https://raw.githubusercontent.com/torvalds/linux/master/drivers/iio/temperature/Kconfig)
- Water: DFRobot Gravity TDS USD 11.80, lab pH kit USD 29.50, turbidity USD 9.90 — [DFRobot TDS](https://www.dfrobot.com/product-1662.html), [DFRobot pH](https://www.dfrobot.com/product-1025.html), [DFRobot turbidity](https://www.dfrobot.com/product-1394.html); Atlas Scientific EZO pH circuit USD 45.99, lab pH probe USD 99.99, pH kit USD 174.99, conductivity K1.0 kit USD 259.99, inline isolator USD 27.99 — [Atlas EZO pH](https://atlas-scientific.com/embedded-solutions/ezo-ph-circuit/), [Atlas EC kit](https://atlas-scientific.com/kits/conductivity-k-1-0-kit/)
- Water pressure/flow/level: MPRLS 0-25 PSI ported sensor USD 29.95, Qwiic MicroPressure USD 37.95, flow meters USD 6.50-24.95, 12-inch eTape level sensor USD 39.95, capacitive soil sensor USD 7.50 — [Adafruit 3965](https://www.adafruit.com/product/3965), [SparkFun MicroPressure](https://www.sparkfun.com/sparkfun-qwiic-micropressure-sensor.html), [Adafruit 5066](https://www.adafruit.com/product/5066), [Adafruit 833](https://www.adafruit.com/product/833), [Adafruit 464](https://www.adafruit.com/product/464), [Adafruit 4026](https://www.adafruit.com/product/4026)

### Inferences
- The methane field study implies hobby metal-oxide parts (MQ series, TGS) should be presented to the agent as "relative change versus a fresh-air baseline taken minutes ago", never as ppm. A sniff-wand workflow (baseline, move to fitting, watch for a step) fits that limitation and is how a leak would be found anyway.
- Nothing in this lane should be described as a safety alarm for propane, CO or hydrogen; a listed standalone alarm remains necessary.
- Atlas EZO costs roughly four to six times the DFRobot analogue kits; its advantages are a kernel driver, isolated digital output and stored calibration. For occasional hauled-water checks the DFRobot TDS probe is the value choice.
- A lux sensor is only a proxy for plane-of-array irradiance; a small reference solar cell read through a shunt would track panel-relevant irradiance better (see outside-the-box ideas).
- AS3935 "disturber" events may double as a crude indicator of nearby arcing or switching noise.

### Gaps
- SPEC Sensors, Alphasense and Winsen ZE07-CO electrochemical CO modules, NDIR methane sensors, Figaro TGS6810 catalytic sensors, dedicated hydrogen sensors beyond the MQ-8, and any refrigerant (R-410A/R-32) sensor: no prices or interface details were retrieved. Prior knowledge (unverified): heated-diode or infrared refrigerant detectors are sold only as closed handheld tools; UV dye plus a 365-395 nm light is the cheap open alternative.
- SEN66 module price not found (SparkFun search returned only SEN55).
- Whether the mainline `VEML6030` driver also binds VEML7700, and whether `BMP280` covers BMP3xx/BMP5xx, was not confirmed from the Kconfig symbol lists alone.
- Low-pressure propane manometer (inches of water column) sensors and oil/fuel pressure transducer prices were not researched; media compatibility of MEMS pressure sensors with propane is a safety question left open.

---

## 7. Magnetic / field / position

### Takeaway
A sub-USD-20 magnetometer with a mainline driver (MMC5983MA) and a USD 30 GNSS module cover wire-finding and geotagging; UWB and RTK are available but are "later" items for this budget.

### Cited Findings
- Magnetometer prices: MMC5983MA Qwiic USD 18.50 — [SparkFun](https://www.sparkfun.com/sparkfun-micro-magnetometer-mmc5983ma-qwiic.html); MMC5603 USD 5.95, LIS2MDL USD 7.95, LIS3MDL USD 12.50, MLX90393 USD 9.95, TMAG5273 USD 5.95 — [Adafruit 5579](https://www.adafruit.com/product/5579), [Adafruit 4488](https://www.adafruit.com/product/4488), [Adafruit 4479](https://www.adafruit.com/product/4479), [Adafruit 4022](https://www.adafruit.com/product/4022), [Adafruit 6489](https://www.adafruit.com/product/6489)
- Mainline magnetometer drivers include `MMC5983`, `SENSORS_RM3100` (I2C and SPI), `TI_TMAG5273`, `QMC5883L` and the ST 3-axis family — [magnetometer Kconfig](https://raw.githubusercontent.com/torvalds/linux/master/drivers/iio/magnetometer/Kconfig)
- Rust: [mmc5983ma](https://crates.io/crates/mmc5983ma) (async, 0.1.0-rc1), [mlx90393](https://crates.io/crates/mlx90393) (2026-09), [tmag5273](https://crates.io/crates/tmag5273), [lis3mdl](https://crates.io/crates/lis3mdl) (2019); crates.io has no `rm3100` crate — [crates.io](https://crates.io/crates/rm3100)
- GNSS prices: Adafruit Ultimate GPS USB USD 29.95, Mini GPS PA1010D (UART + I2C, Qwiic) USD 29.95, Ultimate GPS HAT for Pi 5 USD 29.95, SparkFun MAX-M10S USD 45.95, Waveshare LC29H dual-band HAT USD 39.99-54.99 with optional RTK, SparkFun LG290P quad-band RTK USD 99.95-189.95 — [Adafruit 4279](https://www.adafruit.com/product/4279), [Adafruit 4415](https://www.adafruit.com/product/4415), [Adafruit 2324](https://www.adafruit.com/product/2324), [SparkFun MAX-M10S](https://www.sparkfun.com/sparkfun-gnss-receiver-breakout-max-m10s-qwiic.html), [Waveshare LC29H](https://www.waveshare.com/lc29h-gps-hat.htm), [SparkFun LG290P](https://www.sparkfun.com/sparkfun-quadband-gnss-rtk-breakout-lg290p-qwiic.html)
- Rust GNSS crates: [ublox](https://crates.io/crates/ublox) 0.10.0 (UBX protocol, 2026-03), [nmea](https://crates.io/crates/nmea) 0.8.0 (2026-08), [gpsd_proto](https://crates.io/crates/gpsd_proto)
- Barometric sensors: BMP580 USD 7.95, BMP388 USD 9.95, SPA06-003 USD 4.95; mainline `BMP280`, `DPS310`, `MS5611` drivers — [Adafruit 6411](https://www.adafruit.com/product/6411), [Adafruit 3966](https://www.adafruit.com/product/3966), [Adafruit 6420](https://www.adafruit.com/product/6420), [pressure Kconfig](https://raw.githubusercontent.com/torvalds/linux/master/drivers/iio/pressure/Kconfig)
- UWB in Rust: [dw3000-ng](https://crates.io/crates/dw3000-ng) 1.0.2 (2024) and the older [dw1000](https://crates.io/crates/dw1000)
- BLE scanning from Rust on Linux: [bluer](https://crates.io/crates/bluer) (official BlueZ interface) and [btleplug](https://crates.io/crates/btleplug) 0.13.5 (2026-10)

### Inferences
- A magnetometer on a wand finds DC current paths (battery cables, PV strings under load) by the field step when a load is switched; pairing it with an agent-controlled relay load makes the detection synchronous and far more sensitive than a static reading.
- For AC wiring, a pickup coil or the magnetometer sampled fast enough to see 60 Hz is needed; a coil into an audio input is cheaper and reuses the spectrogram pipeline.
- BLE beacon RSSI gives room-level position only; for "which pod is where", an AprilTag on each pod seen by the camera is cheaper and more accurate than UWB.

### Gaps
- RM3100 breakout price and US stock not found (SparkFun search returned nothing).
- Non-contact AC voltage sensing, EMF/ELF probes and wire-tracer tone kits: no open, Linux-readable product identified; these look like DIY analogue front ends (capacitive plate or coil into an ADC).
- UWB module prices (DWM3000/DWM3001) not found.

---

## 8. Mechanical / force

### Takeaway
Load cells with a NAU7802 or HX711 front end are the standout cheap mechanical sense (both have mainline drivers), with propane-cylinder weighing as the obvious application; optical tachometry can be built from break-beam or reflective IR parts for a few dollars.

### Cited Findings
- NAU7802 24-bit ADC USD 5.95 at both Adafruit and SparkFun; HX711 USD 9.95 (Adafruit) or USD 4.95 (SparkFun); strain-gauge load cells 1-20 kg USD 3.95 — [Adafruit 4538](https://www.adafruit.com/product/4538), [SparkFun Qwiic Scale](https://www.sparkfun.com/sparkfun-qwiic-scale-nau7802.html), [Adafruit 5974](https://www.adafruit.com/product/5974), [SparkFun HX711](https://www.sparkfun.com/sparkfun-load-cell-amplifier-hx711.html), [Adafruit 4543](https://www.adafruit.com/product/4543)
- Mainline ADC drivers include `HX711` and `NAU7802` — [adc Kconfig](https://raw.githubusercontent.com/torvalds/linux/master/drivers/iio/adc/Kconfig); crates [hx711](https://crates.io/crates/hx711) 0.7.0 (2025-11) and [nau7802](https://crates.io/crates/nau7802) (0.1.0-alpha, 2020)
- Optical tach parts: IR break-beam pairs USD 2.95-5.95, reflective IR sensor USD 1.95, T-slot photo interrupter with cable USD 3.95, Hall sensor USD 2.00 — [Adafruit 2167](https://www.adafruit.com/product/2167), [Adafruit 2168](https://www.adafruit.com/product/2168), [Adafruit 2349](https://www.adafruit.com/product/2349), [Adafruit 3985](https://www.adafruit.com/product/3985), [Adafruit 158](https://www.adafruit.com/product/158)
- Pressure: MPRLS 0-25 PSI USD 29.95 with mainline `MPRLS0025PA` driver; Honeywell ABP2/HSC and Sensirion SDP500 differential families also have mainline drivers — [Adafruit 3965](https://www.adafruit.com/product/3965), [pressure Kconfig](https://raw.githubusercontent.com/torvalds/linux/master/drivers/iio/pressure/Kconfig)
- Force and flex parts are mostly out of stock or discontinued at Adafruit (long flex sensor USD 12.95, out of stock) — [Adafruit 182](https://www.adafruit.com/product/182)

### Inferences
- A hanging scale made from one 50 kg-class load cell tells the agent a propane cylinder's fill level from tare weight stamped on the collar, which is more trustworthy than any gas sensor for "is the tank empty".
- A reflective IR sensor aimed at a paint mark, or the camera with a strobed LED, gives RPM without contact; the vehicle lane's OBD data covers engine RPM on the truck if it is OBD-II.

### Gaps
- Automotive-style 0.5-4.5 V pressure transducers (fuel, oil, water, vacuum) and torque sensing: no prices or sources retrieved.
- A higher-capacity S-type or crane-scale load cell for full propane cylinders was not priced.

---

## 9. Actuators and outputs (hands and voice)

### Takeaway
Most of the "voice" is already paid for with the XVF3800 (speaker amplifier, 12-LED ring, playback path), so the remaining actuator spend is small: an IR transceiver for the mini split, a laser on a pan-tilt, a MOSFET/relay channel or two, and a small display.

### Cited Findings
- The XVF3800 board includes a class-D amplifier for a 5 W speaker, a 3.5 mm output and a 12-LED WS2812 ring with host-commanded effects including a DOA display mode — [Seeed wiki](https://wiki.seeedstudio.com/respeaker_xvf3800_introduction/), [reSpeaker host_control README](https://raw.githubusercontent.com/respeaker/reSpeaker_XVF3800_USB_4MIC_ARRAY/master/host_control/README.md)
- Standalone audio out: MAX98357A I2S 3 W amplifier USD 5.95, PAM8302 2.5 W amplifier USD 3.95, enclosed 3 W 4-ohm speaker USD 3.95, Qwiic buzzer USD 10.50, piezo driver amplifier USD 4.95 — [Adafruit 3006](https://www.adafruit.com/product/3006), [Adafruit 2130](https://www.adafruit.com/product/2130), [Adafruit 3351](https://www.adafruit.com/product/3351), [SparkFun Qwiic Buzzer](https://www.sparkfun.com/sparkfun-qwiic-buzzer.html), [Adafruit 5791](https://www.adafruit.com/product/5791)
- IR: transceiver USD 5.95, high-power emitter USD 3.95, receiver USD 4.95 — [Adafruit 5990](https://www.adafruit.com/product/5990), [Adafruit 5639](https://www.adafruit.com/product/5639), [Adafruit 5939](https://www.adafruit.com/product/5939)
- IRremoteESP8266 maintains a per-brand protocol table with a "detailed A/C support" column covering many air-conditioner makes — [IRremoteESP8266 SupportedProtocols](https://raw.githubusercontent.com/crankyoldgit/IRremoteESP8266/master/SupportedProtocols.md)
- Flipper-IRDB is a maintained community collection of IR capture files, with new contributions licensed CC0 — [Flipper-IRDB README](https://raw.githubusercontent.com/Lucaslhm/Flipper-IRDB/main/README.md)
- Rust IR tooling: `cir` ("Linux Infrared Tooling", 0.1.4, 2025) and `irp` (IRP encoder/decoder, 0.3.4, 2025), plus the embedded `infrared` crate — [cir](https://crates.io/crates/cir), [irp](https://crates.io/crates/irp), [infrared](https://crates.io/crates/infrared)
- Switching outputs: single MOSFET driver USD 3.95, 4-channel MOSFET board USD 9.95, STEMMA relay USD 6.95, I2C 8-channel solenoid driver USD 14.95, four-outlet controllable AC relay module USD 39.95 — [Adafruit 5648](https://www.adafruit.com/product/5648), [Adafruit 5757](https://www.adafruit.com/product/5757), [Adafruit 4409](https://www.adafruit.com/product/4409), [Adafruit 6318](https://www.adafruit.com/product/6318), [Adafruit 2935](https://www.adafruit.com/product/2935)
- Motion: assembled mini pan-tilt kit USD 18.95, micro servo USD 5.95; PCA9685 servo driver has a 1.0.0 Rust crate — [Adafruit 1967](https://www.adafruit.com/product/1967), [Adafruit 169](https://www.adafruit.com/product/169), [pwm-pca9685](https://crates.io/crates/pwm-pca9685)
- Indicators and displays: NeoPixel ring 12 USD 8.95 / 16 USD 9.95; 0.96-inch 128x64 OLED (Qwiic) USD 17.50; 1.3-inch SH1106 SPI OLED USD 6.50; bare 2.13-inch monochrome e-ink panel USD 8.50; 2.13-inch tri-colour e-ink with SRAM USD 19.95; haptic driver DRV2605L USD 7.95 — [Adafruit 1643](https://www.adafruit.com/product/1643), [Adafruit 1463](https://www.adafruit.com/product/1463), [Adafruit 326](https://www.adafruit.com/product/326), [Adafruit 5228](https://www.adafruit.com/product/5228), [Adafruit 6383](https://www.adafruit.com/product/6383), [Adafruit 4947](https://www.adafruit.com/product/4947), [Adafruit 2305](https://www.adafruit.com/product/2305)
- Rust display and LED crates: [ssd1306](https://crates.io/crates/ssd1306) 0.10.0, [epd-waveshare](https://crates.io/crates/epd-waveshare) 0.6.0, [smart-leds](https://crates.io/crates/smart-leds), [ws2812-spi](https://crates.io/crates/ws2812-spi)
- USB HID injection hardware: Trinkey QT2040 (RP2040 USB key with Qwiic) USD 8.95, KB2040 USD 9.95; host-side HID access via [hidapi](https://crates.io/crates/hidapi) — [Adafruit 5056](https://www.adafruit.com/product/5056), [Adafruit 5302](https://www.adafruit.com/product/5302)
- Other actuators: 5 V electromagnets USD 7.50-19.95, Peltier module USD 19.95, UV 400 nm LEDs 10-pack USD 4.95 — [Adafruit 3872](https://www.adafruit.com/product/3872), [Adafruit 1331](https://www.adafruit.com/product/1331), [Adafruit 1793](https://www.adafruit.com/product/1793)
- The official Rust MCP SDK (`rmcp`, 3.11.0, updated 2026-11-17) is available for exposing all of these as tools — [rmcp](https://crates.io/crates/rmcp)

### Inferences
- Every actuator can serve as a built-in self-test stimulus for a sensor: speaker chirp for mics, IR LED flash for the NoIR camera and IR receiver, a resistor heater for the thermal array, a servo-swung magnet for the magnetometer, a strobe for the light sensor. Designing these pairs in gives the agent a way to diagnose its own body.
- A signal injector for wire tracing is just the audio output through a coupling capacitor and current-limit resistor, with a pickup coil on a pod as the receiver; this needs isolation design before touching any live circuit.
- An agent-switched resistive load (MOSFET plus power resistor) belongs with the measurement lane but pairs with the magnetometer and current pods here.

### Gaps
- No Linux-native open database of mini split IR codes was verified beyond the two projects above; whether a specific mini split brand is covered must be checked against the IRremoteESP8266 table.
- Laser safety class and eye-safety constraints for an agent-aimed pointer were not researched.
- Programmable electronic load options were left to the measurement-instruments lane.

---

## 10. Remote wireless sensor pods and hot-swap sensor ecosystems

### Takeaway
USD 7-10 ESP32-C3/C6/S3 boards with the now-1.x `esp-hal` Rust HAL make agent-reflashable pods practical, and Theengs' open decoder library lets the pack read more than 140 off-the-shelf BLE sensors without any phone app.

### Cited Findings
- Pod MCU prices: ESP32-C3 DevKitM USD 9.95, ESP32-C6 DevKitC USD 9.95, ESP32-C3 Rust dev kit USD 19.95, QT Py ESP32-S3 USD 12.50, Feather nRF52840 Express USD 24.95, Pico 2 W USD 7.00-8.00 (out of stock), XIAO ESP32S3 USD 7.49 — [Adafruit 5337](https://www.adafruit.com/product/5337), [Adafruit 5672](https://www.adafruit.com/product/5672), [Adafruit 5787](https://www.adafruit.com/product/5787), [Adafruit 5426](https://www.adafruit.com/product/5426), [Adafruit 4062](https://www.adafruit.com/product/4062), [Adafruit 6315](https://www.adafruit.com/product/6315), [Seeed XIAO ESP32S3](https://www.seeedstudio.com/XIAO-ESP32S3-p-5627.html)
- `esp-hal` is a bare-metal `no_std` HAL at version 1.2.2 (September 2026) covering ESP32, C2, C3, C5, C6, C61, H2, S2, S3 and newer parts — [esp-hal crate](https://crates.io/crates/esp-hal), [esp-hal README](https://raw.githubusercontent.com/esp-rs/esp-hal/main/README.md)
- Two radio crates exist: `esp-wifi` (described as a Wi-Fi, Bluetooth and ESP-NOW driver; last release October 2025) and the newer `esp-radio` (1.0.0-beta.1, September 2026), which by dates and description appears to be its successor — [esp-wifi](https://crates.io/crates/esp-wifi), [esp-radio](https://crates.io/crates/esp-radio)
- Embassy provides the async executor and HALs for nRF and RP2040/RP235x — [embassy-executor](https://crates.io/crates/embassy-executor), [embassy-nrf](https://crates.io/crates/embassy-nrf), [embassy-rp](https://crates.io/crates/embassy-rp); the std-based alternative is [esp-idf-hal](https://crates.io/crates/esp-idf-hal) (updated 2026-11-23)
- Theengs Decoder turns BLE advertisements from more than 140 devices into JSON and runs on microcontrollers or a Linux host; Theengs Gateway is a Python BLE-to-MQTT bridge for Raspberry Pi; OpenMQTTGateway is the ESP32 firmware built on the same decoder — [Theengs Decoder README](https://raw.githubusercontent.com/theengs/decoder/development/README.md), [OpenMQTTGateway README](https://raw.githubusercontent.com/1technophile/OpenMQTTGateway/development/README.md)
- A Rust parser for RuuviTag broadcasts exists (`ruuvi-sensor-protocol` 0.7.0, July 2026); no crate named `bthome` exists — [ruuvi-sensor-protocol](https://crates.io/crates/ruuvi-sensor-protocol), [crates.io](https://crates.io/crates/bthome)
- MQTT from Rust: [rumqttc](https://crates.io/crates/rumqttc) 0.25.1
- Hot-swap I2C: 100 mm STEMMA QT/Qwiic cables USD 0.95; 8-channel muxes USD 6.95 (Adafruit PCA9548, SparkFun TCA9548A), 4-channel USD 3.95; USB-plug sensors such as the SHT41 Trinkey USD 10.95 — [Adafruit 4210](https://www.adafruit.com/product/4210), [Adafruit 5626](https://www.adafruit.com/product/5626), [SparkFun Qwiic Mux](https://www.sparkfun.com/sparkfun-qwiic-mux-breakout-8-channel-tca9548a.html), [Adafruit 5664](https://www.adafruit.com/product/5664), [Adafruit 5912](https://www.adafruit.com/product/5912)
- Linux-side embedded-hal: `linux-embedded-hal` 0.5.0 (August 2026) implements the embedded-hal 1.0 traits so the same driver crates run on the Pi and on pods; `rppal` covers Pi GPIO/I2C/SPI/PWM/UART — [linux-embedded-hal](https://crates.io/crates/linux-embedded-hal), [embedded-hal](https://crates.io/crates/embedded-hal), [rppal](https://crates.io/crates/rppal)

### Inferences
- Because the same `embedded-hal` driver crates compile for both the Pi and ESP32/nRF pods, one Rust sensor driver layer can serve the hub and the pods, and the agent can move a sensor between them without new code.
- I2C "auto-discovery" in the Qwiic/STEMMA QT world is an address scan plus a WHO_AM_I/chip-ID read; a table mapping address and ID to driver is something the project must maintain, since several popular parts share addresses (hence the mux).
- An agent that can rebuild and reflash pod firmware needs a wired recovery path (USB serial bootloader on a hub port) so a bad over-the-air image cannot strand a pod.
- Off-the-shelf BLE thermometers decoded by Theengs are the cheapest way to get many sealed, battery-powered temperature/humidity points; custom pods are worth building only for vibration, current and light.

### Gaps
- Prices and decoder coverage for specific Govee, Xiaomi, Inkbird and RuuviTag models were not verified individually; the device list page was not opened.
- ESP-NOW and Thread (802.15.4 on C6/H2) maturity in `esp-radio` 1.0 beta was not assessed beyond the crate descriptions.
- ESPHome was not researched; it is YAML/C++ rather than Rust and would be a borrow-ideas source only.
- Grove ecosystem pricing was not covered.

---

## 11. Per-item driver and self-health summary

### Takeaway
The best-supported parts under both Linux and Rust are the ST accelerometers/IMUs, Sensirion SHT4x/SCD4x, Bosch BME280/BME680, the thermocouple amplifiers, INA-series monitors, NAU7802/HX711 and u-blox GNSS; the notable holes are IIS3DWB, AS7341/AS7343, VL53L5CX (kernel) and RM3100 (Rust).

### Cited Findings
- Parts with a mainline kernel driver and a Rust crate: LIS3DH, ADXL345/ADXL355, MPU-6050, ICM-426xx, LSM6DSx/ISM330DHCX, BME680/688, SCD4x, SHT4x, SGP40, ENS160, TSL2591, MLX90614, MAX31856, MCP9600, HX711, NAU7802, MMC5983MA, TMAG5273, AS3935, PMS-series particulate, INA219/226/228/3221 — [accel Kconfig](https://raw.githubusercontent.com/torvalds/linux/master/drivers/iio/accel/Kconfig), [chemical Kconfig](https://raw.githubusercontent.com/torvalds/linux/master/drivers/iio/chemical/Kconfig), [temperature Kconfig](https://raw.githubusercontent.com/torvalds/linux/master/drivers/iio/temperature/Kconfig), [magnetometer Kconfig](https://raw.githubusercontent.com/torvalds/linux/master/drivers/iio/magnetometer/Kconfig), [hwmon Kconfig](https://raw.githubusercontent.com/torvalds/linux/master/drivers/hwmon/Kconfig), [lis3dh](https://crates.io/crates/lis3dh), [scd4x](https://crates.io/crates/scd4x), [ina228](https://crates.io/crates/ina228), [ina219](https://crates.io/crates/ina219), [ina3221](https://crates.io/crates/ina3221)
- Parts with a Rust crate but no mainline kernel driver found: AS7341 ([as7341](https://crates.io/crates/as7341)), VL53L5CX ([vl53l5cx-uld](https://crates.io/crates/vl53l5cx-uld)), MLX90640 ([mlx9064x](https://crates.io/crates/mlx9064x)), Topdon TC001 ([topdon_thermal_rs](https://crates.io/crates/topdon_thermal_rs)) — checked against [light Kconfig](https://raw.githubusercontent.com/torvalds/linux/master/drivers/iio/light/Kconfig) and [proximity Kconfig](https://raw.githubusercontent.com/torvalds/linux/master/drivers/iio/proximity/Kconfig)
- Parts with a kernel driver but no Rust crate found: RM3100 ([magnetometer Kconfig](https://raw.githubusercontent.com/torvalds/linux/master/drivers/iio/magnetometer/Kconfig)), LTR390 ([light Kconfig](https://raw.githubusercontent.com/torvalds/linux/master/drivers/iio/light/Kconfig)), Atlas EZO ([chemical Kconfig](https://raw.githubusercontent.com/torvalds/linux/master/drivers/iio/chemical/Kconfig))
- Parts with neither found: IIS3DWB, AS7343 — [accel Kconfig](https://raw.githubusercontent.com/torvalds/linux/master/drivers/iio/accel/Kconfig), [crates.io iis3dwb](https://crates.io/crates/iis3dwb), [crates.io as7343](https://crates.io/crates/as7343)
- Several Rust drivers are stale (last release 2019-2021): `adxl355`, `max31855`, `max31856`, `nau7802`, `tsl2591`, `sgp41`, `as3935`, `vl53l1x`, `bmp388`, `scd30`, `sps30`, `mlx9064x` — [adxl355](https://crates.io/crates/adxl355), [max31855](https://crates.io/crates/max31855), [nau7802](https://crates.io/crates/nau7802), [as3935](https://crates.io/crates/as3935), [mlx9064x](https://crates.io/crates/mlx9064x)
- Kernel IIO devices are reachable from Rust without writing a driver via [industrial-io](https://crates.io/crates/industrial-io)
- XVF3800 health hooks: version query, GPIO state reads, full parameter dump, per-beam energy readout, raw-mic routing — [XMOS XVF3800 user guide](https://xmos.com/documentation/XM-014888-PC-5/html/modules/fwk_xvf/doc/user_guide/03_using_the_host_application.html)

### Inferences
- Where a mainline driver exists, the least-effort path on the Pi is a device-tree overlay plus `industrial-io`; where only a crate exists, drive the bus directly through `linux-embedded-hal`. Stale pre-1.0 `embedded-hal` crates may need a small port to the 1.0 traits.
- A generic health layer costs little: bus presence (address ACK), chip-ID match, reading within physical bounds, reading changes when its paired actuator fires, and noise floor within an expected band.

### Gaps
- Sensor-specific built-in self-test features were not verified against datasheets in this session. From prior datasheet knowledge (unverified here): LIS3DH, ADXL355 and ISM330DHCX have electrostatic self-test modes; SCD4x has a `perform_self_test` command; SEN5x/SPS30 expose a device-status register with fan and laser error bits; MAX31856 and MCP9601 report open/short thermocouple faults; MMC5983MA SET/RESET pulses allow offset self-check; u-blox receivers report antenna status and a jamming indicator; VL53L5CX returns a per-zone target-status code; BME68x reports heater-stability and gas-valid bits; NAU7802 has internal offset/gain calibration; AS3935 supports antenna-tuning calibration. MQ-series, piezo pickups, analogue TDS/pH/turbidity probes and DS18B20 (beyond CRC and the 85 C power-on value) have no meaningful self-test.

---

## 12. Outside-the-box sensing and actuation ideas (not in the brief's list)

### Takeaway
Seventeen low-cost ideas, most of which reuse the audio ADC, the camera or an actuator already in the pack; they are design proposals, not sourced facts.

### Cited Findings
- Supporting part prices only: pan-tilt kit USD 18.95 — [Adafruit 1967](https://www.adafruit.com/product/1967); electromagnets from USD 7.50 — [Adafruit 3872](https://www.adafruit.com/product/3872); UV 400 nm LEDs USD 4.95 per 10 — [Adafruit 1793](https://www.adafruit.com/product/1793); AS7341 USD 18.95 — [Adafruit 4698](https://www.adafruit.com/product/4698); FS3000 air velocity USD 19.95 — [SparkFun](https://www.sparkfun.com/renesas-fs3000-air-velocity-sensor-module.html); load cell USD 3.95 — [Adafruit 4543](https://www.adafruit.com/product/4543); Sensirion SDP500-family differential pressure has a mainline driver — [pressure Kconfig](https://raw.githubusercontent.com/torvalds/linux/master/drivers/iio/pressure/Kconfig)

### Inferences
1. DOA-on-panorama: paint the mic array's azimuth as a band on the camera or 360 image, giving a one-dimensional acoustic camera for the price of a calibration step. Use: "the squeal is coming from the left side of the engine bay".
2. Scanning single-pixel ultrasonic imager: a 40 kHz transducer in a short tube on the pan-tilt, raster-scanned to build a leak or arcing heat map over the photo. Use: locating a propane fitting or air leak the agent can circle on an image.
3. Automated tap test: a small solenoid or electromagnet "knocker" plus the contact mic measures resonance. Use: liquid level in a propane cylinder or water tank through the wall, loose fasteners, delaminated panels.
4. Pickup-coil "current stethoscope": a telephone pickup coil or small ferrite coil into a spare audio input lets the agent see injector pulses, ignition events, inverter switching ripple and relay coil actuation as spectrograms without breaking any circuit. Use: old-truck misfire and intermittent relay diagnosis.
5. Photodiode "light listener" into the audio ADC: measures flicker, PWM dimming, 120 Hz ripple, IR remote bursts and strobe timing. Use: detecting inverter or alternator ripple through lamp flicker, verifying the IR blaster actually fired.
6. Video motion magnification: run Eulerian or phase-based magnification on a few seconds of ordinary camera video. Use: making a vibrating bracket, pulsing hose or flexing mount visible to the agent as an image.
7. UV-dye leak imaging: 365-400 nm LED ring plus the camera and fluorescent dye in oil, coolant or refrigerant. Use: pinpointing slow leaks that no gas sensor in this budget can find.
8. Flame-quality sensing: AS7341 spectral ratio (blue versus yellow/NIR) aimed at a burner. Use: flagging incomplete propane combustion or a dirty orifice without touching the appliance.
9. Reference solar cell: a small encapsulated cell with a shunt resistor as a true plane-of-array irradiance sensor, clipped to the array frame on a pod. Use: separating "low sun" from "underperforming string".
10. Differential pressure sensing (SDP-class): a few-pascal sensor across a filter, flue or doorway. Use: mini split filter loading, flue draft, propane low-side pressure in inches of water column (with a suitable isolated sensor).
11. BME688 heater-profile "smell fingerprints": step the hotplate through temperatures and log the resistance curve, building a library of smells. Use: distinguishing hot electrolyte, burnt insulation, raw fuel and mercaptan as classes rather than one VOC number.
12. Calibration and scale card: a printed card with AprilTag, ruler, grey and colour patches, and a thermal-contrast patch (foil plus matte tape). Use: metric measurement, colour-true fluid and corrosion assessment, visual-to-thermal image registration.
13. Stimulus-response self-test matrix: every sense gets a paired actuator (chirp, IR flash, heater resistor, swung magnet, strobe) run at boot. Use: the agent can prove a sensor works before trusting a null reading.
14. Human event marker: a BLE button pod or a spoken "mark" the user triggers when the fault occurs. Use: aligning all sensor logs to the moment "it made the noise" for intermittent faults.
15. Laser-speckle vibrometry: point the laser at a surface and analyse speckle flicker with the camera or a photodiode. Use: non-contact vibration frequency of a hot or moving part that a pod cannot be clipped to.
16. Thermal transient testing: pulse a known load (agent-switched) and watch the thermal array's rate of rise. Use: finding a high-resistance joint that looks normal at steady state.
17. Capacitive level strip: copper tape electrodes on the outside of a plastic tank read by a capacitance-to-digital chip on a pod. Use: stored-water level without drilling.

### Gaps
- None of these ideas was validated against published builds in this session; items 2, 3, 6, 15 and 17 in particular need a bench proof before entering a bill of materials.
- 24/60 GHz mmWave radar modules (non-contact vibration, presence, level through plastic) were considered but not researched or priced.
