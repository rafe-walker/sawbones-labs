# Bench Core parts list (P1)

Prices and delivery dates as seen on Amazon on 26 November 2026, delivering to southern Arizona. They will move. Every line was added to a private Amazon list from its product page, so the live list is the authority on price.

Total at one of each: **$679.77** for 36 lines. With the four lines that need two: about **$729**. Before tax.

"Skip if you have it" lines come to about $110.

## Rung 1: the brain

| Item | Qty | Price | Delivery | Note |
| --- | --- | --- | --- | --- |
| Raspberry Pi 5 4 GB kit: board, 27 W supply, active cooler (iRasptek) | 1 | $161.49 | 27 Nov | Bare boards are not sold on Amazon at a fair price. The same three parts are about $136 at PiShop. |
| Clock battery for Pi 5, ML2020 (Rome Tech) | 1 | $9.93 | 28 Nov | |
| Geekworm X1001 NVMe board | 1 | $13.90 | 28 Nov | Takes 2230 to 2280 drives. |
| KingSpec 256 GB NVMe, 2280 | 1 | $45.64 | 27 Nov | Budget drive. Fine for the Bench Core; revisit for the all-day recorder. |
| SanDisk Ultra 32 GB microSD | 1 | $22.28 | next day | For first boot and recovery. Skip if you have one. |
| Phidgets HUB0003, 7-port USB 2 hub | 1 | $68.99 | 30 Nov | The one hub confirmed for per-port power switching that is still made. Runs on 8 to 30 V. Four left when checked. |
| 12 V 3 A power adapter (Facmogu) | 2 | $9.99 | next day | One for the hub, one for the armed rail. Check the hub's power connector on arrival. |

## Rung 2: build tools

| Item | Qty | Price | Delivery | Note |
| --- | --- | --- | --- | --- |
| Raspberry Pi Pico with headers | 1 | $12.19 | 27 Nov | Flashed with the debug-probe firmware. Micro-USB cable needed. |
| HiLetgo 8-channel 24 MHz logic analyzer | 1 | $12.69 | 28 Nov | |

## Rung 3: the trust root

| Item | Qty | Price | Delivery | Note |
| --- | --- | --- | --- | --- |
| Waveshare RP2350-Zero, 2-pack, USB-C | 2 | $14.99 | 27 Nov | Four boards: interlock, DC meter, comms pod, spare. |
| 22 mm emergency stop, 2 NC, 2-pack | 1 | $11.99 | 27 Nov | |
| 22 mm key switch, 2 position | 1 | $13.99 | 28 Nov | For the class 3 rail. |
| Toggle switch with safety cover (DaierTek) | 1 | $6.39 | 27 Nov | The arm switch. |
| 16 mm momentary LED push buttons, 5-pack | 1 | $9.99 | 27 Nov | The panel approval button. |
| 0.96 inch OLED, I2C, 5-pack (Hosyond) | 1 | $11.99 | 28 Nov | The trusted panel display. |
| WS2812B LED strip, 1 m (SEZO) | 1 | $7.99 | 28 Nov | Cut to length for the status lights. |
| XL4015 buck modules with current limit, 3-pack | 1 | $7.99 | 28 Nov | The hardware cap on the armed rail. |
| AOD4184 isolated MOSFET switch module | 1 | $8.49 | 28 Nov | Rail switching. |
| 2-channel relay module (SunFounder) | 1 | $6.79 | 27 Nov | The fail-open cut. |
| 22 mm 12 to 24 V indicator lights, 12-pack | 1 | $9.99 | 27 Nov | Panel lamps. For the load test, a 12 V car bulb is a better dummy load. |
| Inline blade fuse holders, 4-pack | 1 | $7.99 | next day | |
| Blade fuse assortment, 120 pieces (Nilight) | 1 | $14.39 | next day | Skip if you have fuses. |

## Rung 4: ground truth

| Item | Qty | Price | Delivery | Note |
| --- | --- | --- | --- | --- |
| AD584 voltage reference module, 2.5, 5, 7.5 and 10 V | 1 | $15.49 | 28 Nov | The on-board reference. |

No meter is on the list. The Bench Core is low-voltage only, and the reference module does more for accuracy than a dearer meter would. The meter you own cross-checks it. A safety-rated meter is a later purchase, for when work moves to the battery bank or mains: a Fluke 101 was $71.40.

Precision 0.1% resistors were not found on Amazon and come from an electronics distributor.

## Rung 5: the DC meter module

| Item | Qty | Price | Delivery | Note |
| --- | --- | --- | --- | --- |
| INA228 breakout | 1 | $17.99 | next day | |
| USB isolator, ADuM3160 (DSD TECH SH-G01A) | 2 | $16.99 | 29 Nov | One per measuring module. |

## Rung 6: the comms pod

| Item | Qty | Price | Delivery | Note |
| --- | --- | --- | --- | --- |
| Waveshare isolated USB to RS232, RS485 and TTL | 1 | $33.99 | 27 Nov | The bought reference adapter. |
| Isolated TTL to RS485 module, type C | 1 | $13.43 | 4 to 7 Dec | The slowest line on the list. |
| MAX3232 RS232 to TTL module | 1 | $5.99 | next day | |
| ADuM1201 digital isolators, 3-pack | 1 | $8.09 | next day | |
| RJ45 screw-terminal adapters, 2-pack | 1 | $12.34 | next day | For the inverter's port. |

## Wiring and hardware

| Item | Qty | Price | Delivery | Note |
| --- | --- | --- | --- | --- |
| Double-sided prototype boards, 104 pieces | 1 | $13.99 | 27 Nov | Skip if you have them. |
| 22 AWG stranded wire, 8 colours | 1 | $17.90 | 28 Nov | Skip if you have it. |
| Dupont jumper wires, 120 pieces (ELEGOO) | 1 | $5.93 | same day | Skip if you have them. |
| Screw terminal block kit, 2.54 and 5.08 mm | 1 | about $10 | 3 Dec | |
| M2.5 nylon standoff kit | 1 | $9.99 | 27 Nov | |
| Short USB-C cables, 3-pack | 2 | $7.59 | 28 Nov | |

## Not on the list

- Solder, a breadboard and ordinary resistors, on the assumption they are on hand.
- 0.1% resistors, as above.
- A 12 V car bulb and holder for the load test.
- Anything for later phases: battery, case, radios, cameras, analog capture.

## Before ordering

The cross-check of every part against every other part (connectors, voltages, pin counts) is a P0 item and has not been done on this list yet. Known things to confirm: the hub's power connector, that the NVMe board and the active cooler fit together, and that the generic INA228 board exposes its alert pin.
