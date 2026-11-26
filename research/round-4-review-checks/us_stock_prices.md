# Sawbones first-hardware price and availability check (US stock)

Date of check: 2026-11-25. All prices USD, single-unit, before shipping and tax.

Rule applied: US sellers or US warehouses only. "Amazon (FBA)" means the listing showed "Ships from Amazon". Listings shown as merchant-fulfilled with a 1 to 3 week delivery estimate are flagged "origin unclear" because they may ship from overseas.

How prices were read:
- Adafruit: adafruit.com product API (same price and stock data as the product page). Stock is a count, or "in stock" when 100+.
- PiShop.us, SparkFun, Pololu, Plugable, Test Equipment Depot (TED), Amazon: product page fetched directly.
- Mouser, CanaKit, Harbor Freight, Bioenno, Seeed: product or search page fetched through a page reader.
- VERIFIED = the price was seen on a fetched seller page today. UNVERIFIED = no price seen; best lead given.
- SparkFun note: its static "In stock" label is overwritten by script; the add-to-cart button text ("Add to Cart" vs "Backorder") was used as the stock signal.
- Not readable today: DigiKey (US site), Micro Center, Vilros, Jameco, AutomationDirect, Saelig, Ham Radio Outlet, Dakota Lithium, Newegg, B&H.

Free-shipping thresholds actually seen: Plugable store $35; Amazon $35 on items shipped by Amazon; TED marks some items "ships FREE". None seen for PiShop, Adafruit, SparkFun, Pololu, Mouser, CanaKit, Bioenno.

Headline findings:
1. Pi 5 4 GB is $110 at PiShop and CanaKit and $130 at Adafruit. Memory-driven price rises are visible across the Pi range (256 GB Pi SSD $75.60, SSD kit $99).
2. Both Plugable hubs (USB3-HUB7BC and USB2-HUB7BC) are marked "This product is discontinued" on Plugable's own store. A replacement hub needs choosing.
3. Pololu D36V50F5 is on backorder; D24V50F5 (5 V 5 A) is in stock.
4. Pololu has no 12 V buck-boost at 3 A or more (largest is 2.5 A). Mean Well DDR-60G-12 at Mouser covers it.
5. No 12 V input module was confirmed to advertise a 5 V 5 A USB-PD profile. See line 6 notes.
6. OWON Bluetooth meters on Amazon are merchant-fulfilled (origin unclear). TED, a US dealer, lists the B35T and CM2100B.

## Brain and power

| Line | Item and part number | Seller | Price (USD) | In stock? | URL | Date seen | Status |
|---|---|---|---|---|---|---|---|
| 1 | Raspberry Pi 5 4 GB (SC1111) | PiShop.us | 110.00 | yes | https://www.pishop.us/product/raspberry-pi-5-4gb/ | 2026-11-25 | VERIFIED |
| 1 | Raspberry Pi 5 4 GB (PI5-4GB, board only) | CanaKit | 110.00 | yes | https://www.canakit.com/raspberry-pi-5-4gb.html | 2026-11-25 | VERIFIED |
| 1 | Raspberry Pi 5 4 GB (Adafruit 5812) | Adafruit | 130.00 | yes (26) | https://www.adafruit.com/product/5812 | 2026-11-25 | VERIFIED |
| 2 | Raspberry Pi Active Cooler (SC1148) | PiShop.us | 12.95 | yes | https://www.pishop.us/product/raspberry-pi-active-cooler/ | 2026-11-25 | VERIFIED |
| 2 | Official Pi 5 Active Cooler (Adafruit 5815) | Adafruit | 13.50 | yes | https://www.adafruit.com/product/5815 | 2026-11-25 | VERIFIED |
| 3 | RTC Battery for Raspberry Pi 5 (J5 plug) | PiShop.us | 5.50 | yes | https://www.pishop.us/product/rtc-battery-for-raspberry-pi-5/ | 2026-11-25 | VERIFIED |
| 3 | Raspberry Pi 5 RTC Battery (Adafruit 5817) | Adafruit | 5.00 | yes | https://www.adafruit.com/product/5817 | 2026-11-25 | VERIFIED |
| 4 | Raspberry Pi M.2 HAT+ (SC1166) | PiShop.us | 12.00 | yes | https://www.pishop.us/product/raspberry-pi-m-2-hat-for-pi-5/ | 2026-11-25 | VERIFIED |
| 4 | Raspberry Pi M.2 HAT+ (Adafruit 5902) | Adafruit | 17.40 | yes (90) | https://www.adafruit.com/product/5902 | 2026-11-25 | VERIFIED |
| 4 | Pimoroni NVMe Base for Pi 5 (PIM699), US-stocked | PiShop.us | 18.95 | yes | https://www.pishop.us/product/nvme-base-for-raspberry-pi-5-nvme-base/ | 2026-11-25 | VERIFIED |
| 4 | Micron 2550 256 GB M.2 2230 NVMe (OEM pull, third-party seller) | Amazon (FBA) | 59.49 | yes (6 left) | https://www.amazon.com/dp/B0H6J41YJR | 2026-11-25 | VERIFIED |
| 4 | WD PC SN740 256 GB M.2 2230 NVMe (OEM, third-party seller) | Amazon (FBA) | 74.98 | yes (14 left) | https://www.amazon.com/dp/B0HPFVXCKZ | 2026-11-25 | VERIFIED |
| 4 | Raspberry Pi NVMe SSD 256 GB, 2230 (Adafruit 6090) | Adafruit | 75.60 | no | https://www.adafruit.com/product/6090 | 2026-11-25 | VERIFIED |
| 4 | Raspberry Pi SSD Kit 256 GB, HAT+ plus SSD (SC1675) | PiShop.us | 99.00 | no | https://www.pishop.us/product/raspberry-pi-m-2-hat-ssd-kit-for-raspberry-pi-5-256gb/ | 2026-11-25 | VERIFIED |
| 5 | Raspberry Pi 27 W USB-C PSU, US, black (SC1158) or white (SC1153) | PiShop.us | 12.95 | yes | https://www.pishop.us/product/raspberry-pi-27w-usb-c-power-supply-black-us/ | 2026-11-25 | VERIFIED |
| 5 | Official Raspberry Pi 27 W PD PSU (Adafruit 5814) | Adafruit | 14.04 | yes | https://www.adafruit.com/product/5814 | 2026-11-25 | VERIFIED |
| 6 | Pololu D24V50F5, 5 V 5 A step-down (item 2851), plain buck | Pololu | 32.95 | yes | https://www.pololu.com/product/2851 | 2026-11-25 | VERIFIED |
| 6 | Pololu D36V50F5, 5 V 5.5 A step-down (item 4091), plain buck | Pololu | 44.95 | no (backorder allowed) | https://www.pololu.com/product/4091 | 2026-11-25 | VERIFIED |
| 6 | Geekworm X1203 UPS board for Pi 5, 6 to 18 V input, feeds Pi by pogo pins | Amazon (FBA) | 46.00 | yes (delivery estimate was mid-January) | https://www.amazon.com/dp/B0D633TFMF | 2026-11-25 | VERIFIED |
| 6 | Geekworm X1202 UPS board for Pi 5, 6 to 18 V input, 4x 18650 | Amazon (FBA) | 55.00 | yes (delivery estimate Dec 14 to 28) | https://www.amazon.com/dp/B0CRZ4ZXQW | 2026-11-25 | VERIFIED |
| 6 | Konnected DC to USB-C PD converter, 10 to 30 V in, 65 W max | Amazon (FBA) | 19.96 | yes | https://www.amazon.com/dp/B0GS7D6GC2 | 2026-11-25 | VERIFIED |
| 6 | UCTRONICS 9 to 36 V to 5 V 5 A buck, USB outputs, no PD | Amazon (FBA) | 12.74 | yes | https://www.amazon.com/dp/B07XXWQ49N | 2026-11-25 | VERIFIED |
| 7 | Mean Well DDR-60G-12, 9 to 36 V in, 12 V 5 A out, DIN rail | Mouser | 34.20 | yes (429) | https://www.mouser.com/en/ProductDetail/MEAN-WELL/DDR-60G-12?qs=r5DSvlrkXmL%2FTh7ZYKXayA%3D%3D | 2026-11-25 | VERIFIED |
| 7 | Victron Orion-Tr 12/12-9 isolated DC-DC, 110 W (third-party seller) | Amazon (FBA) | 63.75 | yes | https://www.amazon.com/dp/B01LYZZRBQ | 2026-11-25 | VERIFIED |
| 7 | Pololu S13V25F12, 12 V 2.5 A buck-boost (item 4984), below the 3 A target | Pololu | 18.95 | yes | https://www.pololu.com/product/4984 | 2026-11-25 | VERIFIED |
| 7 | Pololu S18V20F12, 12 V about 2 A buck-boost (item 2577), below target | Pololu | 29.95 | yes | https://www.pololu.com/product/2577 | 2026-11-25 | VERIFIED |
| 7 | DROK 9 to 36 V to 12 V 5 A buck-boost stabiliser (budget option) | Amazon (FBA) | 16.99 | yes | https://www.amazon.com/dp/B081RG8XP5 | 2026-11-25 | VERIFIED |

Notes for line 6 (background, not re-verified today): the Pi 5 treats a supply as 5 A only when a USB-PD source advertises a 5 V 5 A profile. Fed through the 40-pin header or pogo pins there is no negotiation, so the usual approach is `PSU_MAX_CURRENT=5000` in the bootloader config (or `usb_max_current_enable=1`). The Geekworm boards work that way. The Konnected listing does not state a 5 V 5 A profile; most PD sources offer 5 V 3 A, so treat it as unconfirmed. Simplest reliable path today: Pololu D24V50F5 into the header plus the config setting.

Notes for line 4: the M.2 HAT+ takes 2230 and 2242 drives only. PiShop's in-stock 256 GB drive is a 2280 and does not fit.

## USB

| Line | Item and part number | Seller | Price (USD) | In stock? | URL | Date seen | Status |
|---|---|---|---|---|---|---|---|
| 8 | Plugable USB3-HUB7BC, 7-port USB 3.0, 60 W adapter | Plugable store | 49.95 | no (discontinued) | https://plugable.com/products/usb3-hub7bc | 2026-11-25 | VERIFIED |
| 8 | Plugable USB2-HUB7BC, 7-port USB 2.0, 60 W adapter | Plugable store | 34.95 | no (discontinued) | https://plugable.com/products/usb2-hub7bc | 2026-11-25 | VERIFIED |
| 8 | Plugable USB2-HUB7BC, residual Amazon listing, no featured price | Amazon | not shown | 15 left, seller unclear | https://www.amazon.com/dp/B00L2LK164 | 2026-11-25 | UNVERIFIED |
| 9 | Adafruit USB Isolator, ADuM3160, low/full speed, 100 mA (Adafruit 2107) | Adafruit | 34.95 | yes (66) | https://www.adafruit.com/product/2107 | 2026-11-25 | VERIFIED |
| 9 | DSD TECH SH-G01L USB 2.0 high-speed (480 Mbps) isolator, chip not stated | Amazon (FBA) | 39.99 | yes | https://www.amazon.com/dp/B0CSYNSZ3K | 2026-11-25 | VERIFIED |
| 9 | GeeekPi 4-port USB 2.0 high-speed isolator, ADuM3165 | Amazon (FBA) | 49.99 | yes | https://www.amazon.com/dp/B0FY6KMTB3 | 2026-11-25 | VERIFIED |
| 9 | DSD TECH SH-G01C USB-C 480 Mbps isolator | Amazon (FBA) | 45.99 | yes | https://www.amazon.com/dp/B0DS1VDT6L | 2026-11-25 | VERIFIED |
| 9 | Topping HS02 USB 2.0 high-speed isolator (audio-market, third-party seller) | Amazon (FBA) | 84.15 | yes | https://www.amazon.com/dp/B0BTZTDVMK | 2026-11-25 | VERIFIED |

Note for line 8: the Plugable store still shows the prices but has no add-to-cart, only a back-in-stock sign-up. Plugable's current hub catalogue page could not be read today, so no successor part is confirmed.

## Microcontrollers and module parts

| Line | Item and part number | Seller | Price (USD) | In stock? | URL | Date seen | Status |
|---|---|---|---|---|---|---|---|
| 10 | Waveshare RP2350-Zero (USB-C), sold by Waveshare | Amazon (FBA) | 10.99 | yes (8 left) | https://www.amazon.com/dp/B0DNQFHD17 | 2026-11-25 | VERIFIED |
| 10 | Adafruit Feather RP2350 with HSTX (USB-C) (Adafruit 6000) | Adafruit | 12.50 | yes (20) | https://www.adafruit.com/product/6000 | 2026-11-25 | VERIFIED |
| 10 | Adafruit Feather RP2350 with 8 MB PSRAM (Adafruit 6130) | Adafruit | 15.50 | yes | https://www.adafruit.com/product/6130 | 2026-11-25 | VERIFIED |
| 10 | SparkFun Pro Micro RP2350 (USB-C) (DEV-24870) | SparkFun | 16.95 | yes | https://www.sparkfun.com/sparkfun-pro-micro-rp2350.html | 2026-11-25 | VERIFIED |
| 10 | Seeed XIAO RP2350 (USB-C) (102010550) | Mouser | 5.75 | yes (702) | https://www.mouser.com/en/ProductDetail/Seeed-Studio/102010550?qs=efUn273yAhdcBij%2FcOPqUA%3D%3D | 2026-11-25 | VERIFIED |
| 10 | Seeed XIAO RP2350 (102010550), warehouse not stated | Seeed Studio | 4.90 | unknown | https://www.seeedstudio.com/Seeed-XIAO-RP2350-p-5944.html | 2026-11-25 | VERIFIED |
| 10 | Raspberry Pi Pico 2 (SC1631), connector is Micro-USB, not USB-C | PiShop.us | 5.00 | yes | https://www.pishop.us/product/raspberry-pi-pico-2/ | 2026-11-25 | VERIFIED |
| 10 | Raspberry Pi Pico 2 (DEV-26124), Micro-USB | SparkFun | 5.75 | yes | https://www.sparkfun.com/raspberry-pi-pico-2.html | 2026-11-25 | VERIFIED |
| 11 | Adafruit INA228 breakout, 85 V, 20-bit (Adafruit 5832) | Adafruit | 14.95 | yes | https://www.adafruit.com/product/5832 | 2026-11-25 | VERIFIED |
| 11 | SparkFun Current Sensor INA228, Qwiic (SEN-31438) | SparkFun | 14.95 | yes | https://www.sparkfun.com/sparkfun-current-sensor-ina228-qwiic.html | 2026-11-25 | VERIFIED |
| 12 | Ohmite SHD1-100C075DE shunt, 100 A 75 mV | Mouser | 32.94 | yes (4) | https://www.mouser.com/en/ProductDetail/Ohmite/SHD1-100C075DE?qs=i6Zrxxj4WsNScugAxnWbqQ%3D%3D | 2026-11-25 | VERIFIED |
| 12 | Antrader FL-2 type shunt, 100 A 75 mV (generic) | Amazon (FBA) | 9.49 | yes | https://www.amazon.com/dp/B07HCVH97H | 2026-11-25 | VERIFIED |
| 13 | Adafruit NAU7802 24-bit ADC, STEMMA QT (Adafruit 4538) | Adafruit | 5.95 | yes | https://www.adafruit.com/product/4538 | 2026-11-25 | VERIFIED |
| 13 | SparkFun Qwiic Scale NAU7802 (SEN-15242) | SparkFun | 5.95 | no (backorder) | https://www.sparkfun.com/sparkfun-qwiic-scale-nau7802.html | 2026-11-25 | VERIFIED |
| 13 | Bar load cell, 4-wire, 5 kg (Adafruit 4541) | Adafruit | 3.95 | yes | https://www.adafruit.com/product/4541 | 2026-11-25 | VERIFIED |
| 13 | Bar load cell, 4-wire, 10 kg (Adafruit 4542) | Adafruit | 3.95 | yes | https://www.adafruit.com/product/4542 | 2026-11-25 | VERIFIED |
| 13 | Load cell 10 kg straight bar TAL220 (SEN-13329) | SparkFun | 12.95 | no (backorder) | https://www.sparkfun.com/load-cell-10kg-straight-bar-tal220.html | 2026-11-25 | VERIFIED |
| 14 | Monochrome 0.96 in 128x64 OLED, I2C, STEMMA QT (Adafruit 326) | Adafruit | 17.50 | yes | https://www.adafruit.com/product/326 | 2026-11-25 | VERIFIED |
| 14 | Monochrome 1.3 in 128x64 OLED, I2C, STEMMA QT (Adafruit 938) | Adafruit | 19.95 | yes | https://www.adafruit.com/product/938 | 2026-11-25 | VERIFIED |
| 14 | E-paper: 2.13 in 250x122 tri-colour with SRAM (Adafruit 4947) | Adafruit | 19.95 | yes | https://www.adafruit.com/product/4947 | 2026-11-25 | VERIFIED |
| 14 | E-paper: 2.13 in monochrome E-Ink Bonnet for Pi (Adafruit 4687) | Adafruit | 19.95 | yes (51) | https://www.adafruit.com/product/4687 | 2026-11-25 | VERIFIED |
| 15 | E-stop, 22 mm, twist release, 1 NC: Schneider XB5AS8442 | Mouser | 91.51 | yes (92) | https://www.mouser.com/en/ProductDetail/Schneider-Electric/XB5AS8442?qs=sGAEpiMZZMvgy%2FGGoChzD6als7KiYc%2FAvCVYoICEooDcHAHhQm8n0A%3D%3D | 2026-11-25 | VERIFIED |
| 15 | E-stop, 22 mm, 40 mm head, turn reset, 1NO+1NC: Omron A22NE-M-P102-N | Mouser | 71.90 | yes (4) | https://www.mouser.com/en/ProductDetail/Omron-Automation-and-Safety/A22NE-M-P102-N?qs=Zz7%252BYVVL6bEW74WFcG0r4g%3D%3D | 2026-11-25 | VERIFIED |
| 15 | E-stop, 22 mm, NC mushroom, latching: APIELE XB2-01ZS (generic) | Amazon (FBA) | 9.99 | yes | https://www.amazon.com/dp/B079FKJG26 | 2026-11-25 | VERIFIED |
| 15 | Key switch, 22 mm, 2-position maintained, 1NO: APIELE XB2-10Y/21 | Amazon (FBA) | 13.00 | yes | https://www.amazon.com/dp/B088PRMK7R | 2026-11-25 | VERIFIED |
| 15 | Key switch, 22 mm, 2-position maintained, 1NO+1NC: APIELE | Amazon (FBA) | 13.00 | yes | https://www.amazon.com/dp/B07L5D7TXF | 2026-11-25 | VERIFIED |
| 16 | Waveshare USB to RS485/422 industrial isolated converter (FT232RL), sold by Waveshare | Amazon (FBA) | 25.99 | yes | https://www.amazon.com/dp/B0BTBR71PC | 2026-11-25 | VERIFIED |
| 16 | MikroE RS485 Isolator 4 Click, TI ISO1450 (MIKROE-6316) | Mouser | 23.00 | yes (34) | https://www.mouser.com/en/ProductDetail/Mikroe/MIKROE-6316?qs=vOcB1WHNNXKGniq4dw3kLQ%3D%3D | 2026-11-25 | VERIFIED |
| 16 | Waveshare USB to RS232/RS485/TTL industrial isolated converter | PiShop.us | 27.95 | no | https://www.pishop.us/product/usb-to-rs232-rs485-ttl-industrial-isolated-converter/ | 2026-11-25 | VERIFIED |
| 16 | Isolated RS232: Waveshare 2-channel isolated RS232 HAT (17498) | PiShop.us | 16.95 | yes | https://www.pishop.us/product/2-channel-isolated-rs232-expansion-hat-for-raspberry-pi/ | 2026-11-25 | VERIFIED |
| 16 | Isolated RS232: MikroE RS232 Isolator 2 Click (MIKROE-4536) | Mouser | 32.00 | yes (3) | https://www.mouser.com/en/ProductDetail/Mikroe/MIKROE-4536?qs=DRkmTr78QAS3FKIepLFw%252BQ%3D%3D | 2026-11-25 | VERIFIED |
| 16 | Non-isolated reference: SparkFun USB to RS-485 Converter (BOB-09822) | SparkFun | 25.95 | yes | https://www.sparkfun.com/sparkfun-usb-to-rs-485-converter.html | 2026-11-25 | VERIFIED |
| 17 | Precision LM4040 reference breakout, 2.048 V and 4.096 V (Adafruit 2200) | Adafruit | 7.50 | yes (64) | https://www.adafruit.com/product/2200 | 2026-11-25 | VERIFIED |
| 17 | 0.1% resistor: TE YR1B10KCC, 10 k, 15 ppm, through-hole (each) | Mouser | 0.94 | yes (26,147) | https://www.mouser.com/en/ProductDetail/TE-Connectivity-Holsworthy/YR1B10KCC?qs=VHcS2MTj4gZAHnFpyW81RQ%3D%3D | 2026-11-25 | VERIFIED |
| 17 | DMMCheck Plus (5.0000 V ref, 0.1% 10 ppm resistors, 1 mA source) | dmmcheckplus.com (US, NC) | 199.00 (lead only) | unknown | https://www.dmmcheckplus.com | 2026-11-25 | UNVERIFIED |
| 18 | Adafruit MAX31856 universal thermocouple amplifier (Adafruit 3263) | Adafruit | 17.50 | yes | https://www.adafruit.com/product/3263 | 2026-11-25 | VERIFIED |

Notes:
- Line 12: 75 mV is the usual rating of the generic FL-2 shunts; US-made shunts are more often 50 mV or 100 mV. All three fit the INA228 on its wider range.
- Line 15: no key switch from an industrial brand was priced. Mouser lists IDEC YW1K-2AE10 as quote-only. AutomationDirect (GCX series) is the likely low-cost US source but its site could not be read.
- Line 17: voltagestandard.com now points DMMCheck Plus sales to dmmcheckplus.com. That shop renders by script; $199.00 with "InStock" appears only in the site's embedded offer data, so it is a lead, not a confirmed price.

## Radio and senses

| Line | Item and part number | Seller | Price (USD) | In stock? | URL | Date seen | Status |
|---|---|---|---|---|---|---|---|
| 19 | RTL-SDR Blog V3, dongle only, sold by RTL-SDR Blog | Amazon (FBA) | 37.95 | yes | https://www.amazon.com/dp/B0129EBDS2 | 2026-11-25 | VERIFIED |
| 19 | RTL-SDR Blog V3 with dipole antenna kit, sold by RTL-SDR Blog | Amazon (FBA) | 47.95 | yes | https://www.amazon.com/dp/B011HVUEME | 2026-11-25 | VERIFIED |
| 20 | Espressif ESP32-C6-DevKitC-1-N8 (Adafruit 5672) | Adafruit | 9.95 | yes (85) | https://www.adafruit.com/product/5672 | 2026-11-25 | VERIFIED |
| 20 | Espressif ESP32-C6-DevKitC-1-N8 | Mouser | 9.18 | yes (990) | https://www.mouser.com/en/ProductDetail/Espressif-Systems/ESP32-C6-DevKitC-1-N8?qs=8Wlm6%252BaMh8TjnOR8RwmaBw%3D%3D | 2026-11-25 | VERIFIED |
| 20 | Espressif ESP32-C5-DevKitC-1-N8R8 | Mouser | 15.00 | yes (1,431) | https://www.mouser.com/en/ProductDetail/Espressif-Systems/ESP32-C5-DevKitC-1-N8R8?qs=6avfeC6zeS67aD1hu9NHBw%3D%3D | 2026-11-25 | VERIFIED |
| 20 | SparkFun Thing Plus ESP32-C5 (WRL-30678) | SparkFun | 24.95 | yes | https://www.sparkfun.com/sparkfun-thing-plus-esp32-c5.html | 2026-11-25 | VERIFIED |
| 21 | Nordic nRF52840 Dongle (PCA10059) | Mouser | 10.83 | yes (3,979) | https://www.mouser.com/ProductDetail/Nordic-Semiconductor/nRF52840-Dongle?qs=gTYE2QTfZfTbdrOaMHWEZg%3D%3D | 2026-11-25 | VERIFIED |
| 21 | nRF52840 Dongle PCA10059 (third-party seller, about double list) | Amazon (FBA) | 21.98 | yes | https://www.amazon.com/dp/B0F2J95GDR | 2026-11-25 | VERIFIED |
| 22 | Raspberry Pi Camera Module 3, standard | CanaKit | 26.95 | unknown (add-to-cart shown) | https://www.canakit.com/raspberry-pi-camera-module-3-wide.html | 2026-11-25 | VERIFIED |
| 22 | Raspberry Pi Camera Module 3, standard | PiShop.us | 29.25 | yes | https://www.pishop.us/product/raspberry-pi-camera-module-3/ | 2026-11-25 | VERIFIED |
| 22 | Raspberry Pi Camera Module 3 Wide | CanaKit | 37.95 | unknown (add-to-cart shown) | https://www.canakit.com/raspberry-pi-camera-module-3-wide.html | 2026-11-25 | VERIFIED |
| 22 | Raspberry Pi Camera Module 3 Wide | PiShop.us | 38.50 | no | https://www.pishop.us/product/raspberry-pi-camera-module-3-wide/ | 2026-11-25 | VERIFIED |
| 22 | Raspberry Pi Camera Module 3 Wide (Adafruit 5658) | Adafruit | 38.50 | no | https://www.adafruit.com/product/5658 | 2026-11-25 | VERIFIED |
| 22 | Camera Cable for Raspberry Pi 5 (22-pin to 15-pin) | PiShop.us | 3.95 | yes | https://www.pishop.us/product/camera-cable-for-raspberry-pi-5/ | 2026-11-25 | VERIFIED |
| 22 | Pi 5 FPC camera cable, three lengths (Adafruit 5818, 5819, 5820) | Adafruit | 2.50 to 3.16 | yes | https://www.adafruit.com/product/5818 | 2026-11-25 | VERIFIED |
| 23 | MLX90640 IR array, 55 degree (SEN-16429, bare sensor module, special price) | SparkFun | 35.95 | yes | https://www.sparkfun.com/ir-array-sensor-55-degree-fov-mlx90640.html | 2026-11-25 | VERIFIED |
| 23 | MLX90640 IR array, 110 degree (SEN-29459) | SparkFun | 64.95 | yes | https://www.sparkfun.com/ir-array-sensor-110-degree-fov-mlx90640.html | 2026-11-25 | VERIFIED |
| 23 | Adafruit MLX90640 breakout, 55 degree (4469) and 110 degree (4407) | Adafruit | 74.95 each | no (both) | https://www.adafruit.com/product/4469 | 2026-11-25 | VERIFIED |
| 23 | SparkFun MLX90640 Qwiic breakout, 110 degree (SEN-14843) | SparkFun | 39.95 | no | https://www.sparkfun.com/sparkfun-ir-array-breakout-110-degree-fov-mlx90640-qwiic.html | 2026-11-25 | VERIFIED |
| 24 | Seeed reSpeaker XVF3800 4-mic USB array (101991441), warehouse not stated | Seeed Studio | 54.90 | unknown | https://www.seeedstudio.com/ReSpeaker-XVF3800-USB-Mic-Array-p-6488.html | 2026-11-25 | VERIFIED |
| 24 | Seeed reSpeaker XVF3800 (101991441) | Mouser | 54.90 | no (0 on hand, 223 on order) | https://www.mouser.com/en/ProductDetail/Seeed-Studio/101991441?qs=FmjOKN4Os86QyhqbRziMPQ%3D%3D | 2026-11-25 | VERIFIED |
| 24 | Seeed reSpeaker XVF3800 with case, sold by Seeed Studio Official | Amazon (FBA) | 67.92 | yes (1 left, delivery Dec 13 to 27) | https://www.amazon.com/dp/B0GVJ5YQ58 | 2026-11-25 | VERIFIED |
| 24 | Adafruit I2S MEMS microphone breakout, ICS-43434 (Adafruit 6049) | Adafruit | 8.95 | yes | https://www.adafruit.com/product/6049 | 2026-11-25 | VERIFIED |
| 25 | Adafruit Ultimate GPS Breakout, PPS pin (Adafruit 746) | Adafruit | 29.95 | yes (52) | https://www.adafruit.com/product/746 | 2026-11-25 | VERIFIED |
| 25 | Adafruit Ultimate GPS with GLONASS, PA1616D (Adafruit 5440) | Adafruit | 29.95 | yes (14) | https://www.adafruit.com/product/5440 | 2026-11-25 | VERIFIED |
| 25 | SparkFun GPS Breakout NEO-M9N, U.FL, Qwiic (GPS-15712) | SparkFun | 70.95 | yes | https://www.sparkfun.com/sparkfun-gps-breakout-neo-m9n-u-fl-qwiic.html | 2026-11-25 | VERIFIED |
| 25 | SparkFun GPS Breakout NEO-M9N, SMA, Qwiic (GPS-17285) | SparkFun | 76.50 | yes | https://www.sparkfun.com/sparkfun-gps-breakout-neo-m9n-sma-qwiic.html | 2026-11-25 | VERIFIED |

Notes:
- Line 22: the two CanaKit rows come from one page that lists all variants. An "Arducam for Raspberry Pi Camera Module 3 Wide" bundle is also on Amazon (FBA) at $30.00, https://www.amazon.com/dp/B0C5D97DRJ; whether it is the official board was not confirmed.
- Line 23: the in-stock SparkFun parts are the bare TO-can sensor modules, not Qwiic breakouts. Every MLX90640 breakout board checked was out of stock (Adafruit both, SparkFun Qwiic 110, PiShop Pimoroni at $70.95).

## Meters

| Line | Item and part number | Seller | Price (USD) | In stock? | URL | Date seen | Status |
|---|---|---|---|---|---|---|---|
| 26 | OWON B35T (TED part B35T, model B-35T) | Test Equipment Depot | 70.00 | unknown (add-to-cart shown) | https://www.testequipmentdepot.com/owon-b35t-digital-multimeter-with-bluetooth-true-rms.html | 2026-11-25 | VERIFIED |
| 26 | OWON B35T+, seller VIVITEQ, merchant-fulfilled, origin unclear | Amazon | 69.00 | yes (delivery estimate Dec 7) | https://www.amazon.com/dp/B07SFLTRCB | 2026-11-25 | VERIFIED |
| 26 | OWON B41T+, seller VIVITEQ, merchant-fulfilled, origin unclear | Amazon | 78.00 | yes (delivery estimate Dec 8) | https://www.amazon.com/dp/B07CNH55FK | 2026-11-25 | VERIFIED |
| 26 | OWON OW18E, third-party seller, merchant-fulfilled, origin unclear | Amazon | 82.21 | 1 left (delivery estimate Dec 17) | https://www.amazon.com/dp/B0CBTYYP2M | 2026-11-25 | VERIFIED |
| 26 | ANENG AN9002, sold by ANENG Stores | Amazon (FBA) | 42.99 | unknown (delivery estimate read as January) | https://www.amazon.com/dp/B0BH4B56N4 | 2026-11-25 | VERIFIED |
| 26 | ZT-5B (generic listing), merchant-fulfilled, "ships within 4 to 5 days" | Amazon | 100.76 | origin unclear, fails the US-stock rule | https://www.amazon.com/dp/B0F3J75R4T | 2026-11-25 | VERIFIED |
| 27 | Klein Tools MM420, true RMS, CAT III 600 V | Test Equipment Depot | 64.98 | unknown (add-to-cart shown) | https://www.testequipmentdepot.com/klein-tools-mm420-digital-multimeter-trms-auto-ranging-600v-temp.html | 2026-11-25 | VERIFIED |
| 27 | Fluke 107, CAT III 600 V | Amazon (FBA) | 95.00 | yes | https://www.amazon.com/dp/B00HEAMMIC | 2026-11-25 | VERIFIED |
| 27 | Klein Tools MM720 | Amazon (sold by Amazon.com) | 99.97 | yes | https://www.amazon.com/dp/B0B57PDQJC | 2026-11-25 | VERIFIED |
| 27 | Fluke 106 (U.S.), CAT III 600 V (third-party seller) | Amazon (FBA) | 93.07 | yes | https://www.amazon.com/dp/B0GPF3FDT7 | 2026-11-25 | VERIFIED |
| 27 | Fluke 101 (third-party seller; an Asia-market model) | Amazon (FBA) | 70.90 | yes | https://www.amazon.com/dp/B00HE6MIJY | 2026-11-25 | VERIFIED |
| 27 | Fluke 107 ESP (US dealer price, above budget) | Test Equipment Depot | 155.81 | yes | https://www.testequipmentdepot.com/fluke-107-esp-palm-sized-cat-iii-digital-multimeter.html | 2026-11-25 | VERIFIED |
| 27 | EEVblog Brymen BM235 (above budget) | Amazon | 180.00 | not checked | https://www.amazon.com/dp/B01JZ1ADCO | 2026-11-25 | UNVERIFIED |
| 28 | OWON CM2100B Bluetooth clamp meter, 600 V, 100 A, "ships FREE" | Test Equipment Depot | 84.29 | unknown (add-to-cart shown) | https://www.testequipmentdepot.com/owon-cm2100b-smart-ac-dc-mini-bluetooth-clamp-meter-600v-100a-20000.html | 2026-11-25 | VERIFIED |
| 28 | OWON CM2100B, third-party seller, merchant-fulfilled, origin unclear | Amazon | 78.49 | yes (delivery estimate Dec 16) | https://www.amazon.com/dp/B0CBTYFT2V | 2026-11-25 | VERIFIED |

Safety category and protocol notes for line 26 (what could and could not be confirmed):

| Meter | Category rating | Evidence today | Open protocol or client |
|---|---|---|---|
| OWON B35T / B35T+ | Sold as a 1000 V CAT III class meter | Independent teardown (lygte-info.dk) calls the rating "way optimistic": 250 V glass fuse on mA, 250 V ceramic on 20 A, no MOVs. Probes marked CAT III with caps, CAT II without. Body marking not captured. | Yes: github.com/DeanCording/owonb35, inflex/owon-b35, cransom/b35t-reader |
| OWON B41T+ | Not captured from a fetched page | Same reviewer: "The CAT rating cannot be correct with 250V fuses"; PTCs only, no MOVs | Yes: github.com/art-ya/OwonB41T, sercona/Owon-Multimeters |
| OWON OW18E | Not captured from a fetched page | Elektor review page read did not state it | Covered by the OWON multi-model projects (not confirmed per model) |
| ANENG AN9002 | Not captured; the sister ZT-5BQ listing states "600V CAT III" | No independent certification found | Yes: github.com/ludwich66/Bluetooth-DMM lists Aneng 9002 with several clients |
| ZT-5B (ZOYI / BSIDE) | Not captured; same family as above | No independent certification found | Yes: same repository lists ZT-5B and BSIDE ZT-5B |
| OWON CM2100B (line 28) | 600 V per dealer title; category not shown on the page read | None | Not confirmed |

Reading: none of the Bluetooth meters has an independently confirmed category rating. Treat them as low-energy bench instruments and keep the line 27 meter for anything mains-connected. The B35T+ has the best mix of US dealer availability and open clients, but confirm with the dealer that the unit is the "+" (BLE) revision.

Note for line 27: the Fluke 107 Amazon offer showed "Ships from Amazon" with mixed seller fields (Amazon.com and a third party); confirm the seller at checkout. The Klein MM420 category rating was read from the dealer page text.

## Portable (later stage)

| Line | Item and part number | Seller | Price (USD) | In stock? | URL | Date seen | Status |
|---|---|---|---|---|---|---|---|
| 29 | Bioenno BLF-1220A, 12 V 20 Ah LiFePO4, integrated PCM (US, California) | Bioenno Power | 192.99 | yes | https://www.bioennopower.com/products/12v-20ah-lfp-battery-pvc-blf-1220a | 2026-11-25 | VERIFIED |
| 29 | ExpertPower 12 V 20 Ah LiFePO4, built-in BMS, sold by ExpertPower Direct | Amazon (FBA) | 129.99 | yes (5 left) | https://www.amazon.com/dp/B07X523S96 | 2026-11-25 | VERIFIED |
| 29 | GOLDENMATE 12 V 20 Ah LiFePO4, IP67 (budget import brand, US warehouse) | Amazon (FBA) | 63.74 | yes | https://www.amazon.com/dp/B0DJ2K2HQM | 2026-11-25 | VERIFIED |
| 30 | Apache 3800 weatherproof case, black (item 63927) | Harbor Freight | 39.99 | yes, in-store only | https://www.harborfreight.com/3800-weatherproof-protective-case-large-black-63927.html | 2026-11-25 | VERIFIED |
| 30 | Apache 3800-size case, third-party resale (about double) | Amazon (FBA) | 73.00 | yes (12 left) | https://www.amazon.com/dp/B079KTVRC9 | 2026-11-25 | VERIFIED |
| 31 | Noctua NF-A6x25 FLX, 60 mm 12 V fan, sold by Noctua | Amazon (FBA) | 16.95 | yes | https://www.amazon.com/dp/B009NQMESS | 2026-11-25 | VERIFIED |
| 31 | 60 mm fan dust filter cover set (generic) | Amazon (FBA) | 7.69 | yes | https://www.amazon.com/dp/B09R43LD4Y | 2026-11-25 | VERIFIED |

Note for line 30: Harbor Freight lists the case as not available for online purchase, so it needs a store visit.

## Bench Core subtotal

Scope: lines 1 to 9, one RP2350 board, one INA228, the OLED, the e-stop, the key switch and one reference meter. One option per line, cheapest in-stock verified choice unless stated.

| Line | Choice | Seller | Price (USD) | Basis |
|---|---|---|---|---|
| 1 | Raspberry Pi 5 4 GB | PiShop.us | 110.00 | verified |
| 2 | Active Cooler | PiShop.us | 12.95 | verified |
| 3 | RTC battery | PiShop.us | 5.50 | verified |
| 4 | M.2 HAT+ | PiShop.us | 12.00 | verified |
| 4 | Micron 2550 256 GB 2230 NVMe | Amazon | 59.49 | verified (OEM pull, third-party seller) |
| 5 | 27 W USB-C PSU | PiShop.us | 12.95 | verified |
| 6 | Pololu D24V50F5 5 V 5 A buck | Pololu | 32.95 | verified |
| 7 | Mean Well DDR-60G-12 | Mouser | 34.20 | verified |
| 8 | Plugable USB3-HUB7BC | Plugable | 49.95 | ESTIMATE: listed price of a discontinued product |
| 8 | Plugable USB2-HUB7BC | Plugable | 34.95 | ESTIMATE: listed price of a discontinued product |
| 9 | Adafruit USB isolator (ADuM3160) | Adafruit | 34.95 | verified |
| 9 | DSD TECH SH-G01L high-speed isolator | Amazon | 39.99 | verified |
| 10 | Adafruit Feather RP2350 | Adafruit | 12.50 | verified |
| 11 | Adafruit INA228 | Adafruit | 14.95 | verified |
| 14 | Adafruit 0.96 in 128x64 OLED | Adafruit | 17.50 | verified |
| 15 | APIELE 22 mm NC e-stop | Amazon | 9.99 | verified (generic brand) |
| 15 | APIELE 22 mm 2-position key switch | Amazon | 13.00 | verified (generic brand) |
| 27 | Fluke 107 | Amazon | 95.00 | verified |

- Verified items: $517.92
- Estimated items (two discontinued Plugable hubs at last listed price): $84.90
- **Bench Core subtotal: $602.82** before shipping and tax

Swings worth knowing:
- Industrial-brand e-stop from Mouser instead of the generic one: plus $61.91 (Omron) or plus $81.52 (Schneider).
- Klein MM420 from TED instead of Fluke 107: minus $30.02 (stock not confirmed).
- Pi 5 from Adafruit instead of PiShop or CanaKit: plus $20.00.
- XIAO RP2350 from Mouser instead of the Feather: minus $6.75.
- Pololu D36V50F5 instead of D24V50F5: plus $12.00 and a backorder wait.

## Lines or sub-items not verified

- Line 6: no 12 V input module confirmed to present a 5 V 5 A USB-PD profile to the Pi 5. Prices for candidates are verified; the 5 A recognition is not. Lead: Geekworm X1203 / X1202 / X1209 with `PSU_MAX_CURRENT=5000`, or a plain Pololu buck into the header with the same setting.
- Line 8: both Plugable hubs are discontinued at the maker. Prices shown are the last listed prices. No successor confirmed. Lead: the residual Amazon listing for USB2-HUB7BC (no featured price), and Plugable's current hub range.
- Line 12: no 50 A 75 mV shunt was priced (only 100 A). Lead: Mouser search across the Ohmite SHD1 series.
- Line 15: no industrial-brand key switch priced. Leads: AutomationDirect GCX series (site unreadable today), IDEC YW1K-2AE10 (quote-only at Mouser).
- Line 17: DMMCheck Plus price not confirmed on a rendered page. Lead: $199.00 in the dmmcheckplus.com embedded offer data. No REF5050 or ADR4550 breakout was priced; voltagestandard.com lists a REF5050-based VREF5-01 without a price.
- Line 22: CanaKit stock for both camera variants not stated on the page. Camera Module 3 Wide is out of stock at PiShop and Adafruit.
- Line 23: no MLX90640 breakout board in stock anywhere checked; only SparkFun bare sensor modules.
- Line 24: Seeed's own store shows the price but not stock or shipping warehouse. Mouser has none on hand.
- Line 26: printed category ratings not confirmed for B41T+, OW18E, AN9002, ZT-5B. US-warehouse stock not confirmed for any OWON Amazon listing. ZT-5B found only from an overseas-looking merchant.
- Line 27: Brymen BM235 seen only in a search listing at $180.00, above the budget, product page not opened.
- Line 28: OWON CM2100B stock at TED not stated; category rating not shown.
- Line 31: no 50 mm option priced; the 60 mm Noctua and a generic filter set are verified.
- Sellers that could not be read at all today: DigiKey, Micro Center, Vilros, Jameco, AutomationDirect, Saelig, Ham Radio Outlet, Dakota Lithium.
