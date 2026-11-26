# Fixed bench observability: soldering vision, agent-readable breadboard, low-cost robot arm (state as of November 2026)

Rating key used throughout: **DO NOW** (cheap, proven), **PHASE 2**, **PIE IN THE SKY** (with reason).
Prices are USD from the cited page on the fetch date (2026-11-25) unless a different date is given. Items marked "(background knowledge, unverified)" were not confirmed by a fetched source and should be treated as leads, not facts.

---

## 1. Soldering and bench cameras, vision-LLM joint judging, thermal view, Pinecil telemetry

### Takeaway
A fixed camera plus Pinecil V2 BLE telemetry is the cheapest high-value observability on the bench (about USD 25-60 of new hardware), but published evidence says general vision LLMs are unreliable at fine-grained solder/PCB defect localization and at reading 3D wiring from photos, so the agent should be used as a process coach and gross-defect reviewer (bridges, missing joints, wrong part, dwell time), with electrical verification as ground truth. Realistic feedback is per-joint or per-step review seconds after the fact, not real-time correction mid-joint.

### Cited Findings

**Camera hardware and prices**
- Raspberry Pi Camera Module 3: "from $25"; Sony IMX708, 4608 x 2592 (11.9 MP effective), phase-detect autofocus, 1080p50 / 720p120 video, HDR mode up to 3 MP. — [Raspberry Pi](https://raspberrypi.com/products/camera-module-3)
- Plugable USB2-Micro-250X USB microscope: $59.95; presents as a standard UVC camera (works on Linux kernel 2.6.26+ and Raspberry Pi OS); 2 MP stills (1600x1200) but video is only 640x480 at 30 Hz; built-in dimmable LED halo. — [Plugable](https://plugable.com/products/usb2-micro-250x)
- Andonstar AD246S-M (7-inch LCD, 3 lenses, HDMI output, "2160P" recording): $288.00 on Newegg, ships from Hong Kong. The listing does not state the HDMI output resolution. — [Newegg](https://www.newegg.com/p/3A1-003D-00015)
- PiKVM V4 Mini has a rear HDMI video input, a USB 2.0 OTG connector, a front USB 2.0 serial-console port, RJ-45 ATX control and 1 Gb Ethernet; the SD card is read-only by default (`rw`/`ro` to toggle). The quickstart page does not state the max capture resolution. — [PiKVM V4 docs](https://docs.pikvm.org/v4/)
- A public solder-joint dataset (SolDef_AI) was shot with a cheap portable USB microscope (Cainda, 2560x1440, 30 fps) with a diffused LED ring on a flexible arm; each part was shot top-down, at 45 degrees, and axonometric. The top view was used for component-position labels and the 45-degree view for solder-joint labels. — [SolDef_AI, MDPI JMMP 2024](https://mdpi.com/2504-4494/8/3/117)
- Luxonis published a solder-defect example using an OAK4 camera on an AMScope SM-7 stereo microscope via a 3D-printed mount; the model was trained on a public Roboflow PCB-defect set plus only about 50 custom microscope photos. No accuracy metrics were reported. — [Luxonis docs](https://docs.luxonis.com/overview/industry-examples/electronics/solder_defect_detection)

**Can a vision LLM judge joints? Published evidence**
- UniPCB (Jan 2026) is a PCB inspection benchmark with 6,581 images and 23,359 QA pairs over 14 subtasks. Average scores: PCB-GPT (fine-tuned Qwen2.5-VL-7B) 67.3, GPT-5 66.5, Gemini 2.5 Pro 64.9, Gemini 2.5 Flash 59.8, Qwen3-VL about 58.5. Claude was not among the models evaluated. The authors report hallucinations and inaccurate defect localization from general-purpose models, and that good semantic answers did not translate into accurate spatial localization. On the external PCB-Bank set, PCB-GPT reached F1 52.1 zero-shot vs 33.0 for GPT-5 (61.0 vs 50.3 one-shot). — [UniPCB, arXiv 2601.19222](https://arxiv.org/html/2601.19222)
- InspectVLM (Aug 2025; drone inspection of wind/solar/roof assets, not PCBs) concluded that unified VLMs "do not meet the accuracy, reliability, or robustness required for industrial-grade inspection": competitive on classification (75.9% vs ResNet-50 66.7%) but far behind on fine defect boxes (16.5 precision vs 46.1 for Faster R-CNN), and its yes/no anomaly answers collapsed to always "yes" during training. — [InspectVLM, arXiv 2508.01921](https://arxiv.org/html/2508.01921v1)
- SolDef_AI: 1,150 images of SMT joints (0603/0805/1206 passives only; no through-hole), classes good / excess solder / insufficient solder / spike; a Mask R-CNN baseline got only 42.7 detection mAP on the solder-joint subset, with insufficient solder the hardest class. Dataset on Kaggle, metadata lists CC BY 3.0. — [SolDef_AI](https://mdpi.com/2504-4494/8/3/117)
- Other open resources that surfaced in search but were not examined in detail: Roboflow Universe "Soldering Defects" dataset — [Roboflow](https://universe.roboflow.com/kolzek/soldering-defects); IEEE DataPort "Solder joint dataset" — [IEEE DataPort](https://ieee-dataport.org/documents/solder-joint-dataset); a YOLO variant with hybrid-attention FPN for solder joints — [arXiv 2401.01214](https://arxiv.org/pdf/2401.01214); SolderNet (explainable AOI) — [arXiv 2211.10274](https://arxiv.org/pdf/2211.10274).
- In a university lab study where students used ChatGPT/Gemini with photos to debug breadboard/PCB analog circuits, nearly all factual errors followed image analysis: wrong rail for a clip, a bent wire missed from a top-down shot, wrong pin numbers, including one wrong diagnosis asserted at "99%" confidence. The authors recommend telling users that off-the-shelf LLMs are unreliable at reading breadboard/PCB images, especially 3D wiring. — [Chat Debugging, arXiv 2608.02955](https://arxiv.org/pdf/2608.02955)

**Feedback loop and cost (real deployments)**
- Kevin McAleer's "self-documenting workshop" (published 2026-07-01): overhead USB-C webcam, OpenCV change detection to drop static frames, Claude Sonnet 4.6 on frames during the session and Claude Opus 4.7 once at the end, local faster-whisper transcription on a Mac. API cost was about GBP 0.40 per 90-minute session. A current clamp on the soldering iron tells the system when soldering is happening so no vision tokens are spent to detect it. It identified components and build steps accurately; it invented plausible but wrong details when it could not read code on a screen. — [Wevolver/Arduino](https://www.wevolver.com/article/the-self-documenting-workshop-by-kevin-mcaleer)
- CircuitSense (March 2026 hackathon project) ran its structured vision pass on the camera feed once every 15 seconds while a separate voice model handled conversation; it reports no measured latency and no failures. — [DEV Community](https://dev.to/kryptonus_vicky/building-circuitsense-how-i-built-a-dual-engine-ai-lab-partner-that-sees-your-hardware-3mf4)

**Thermal view**
- Adafruit MLX90640 breakout (110 degree FoV): $74.95, 24x32 pixels, I2C, practical max about 16 Hz, target range -40 to 300 C, accuracy +/-2 C over 0-100 C; out of stock on the fetch date. — [Adafruit](https://www.adafruit.com/products/4469)
- Hackaday's Aug 2025 coverage of a DIY MLX90640 + ESP32 camera ("well under $100") judged 24x32 adequate for "which part is heating up" but low for detail; commenters cite 256x192 sensors at roughly 3-4x the price and warn that cheap cameras advertise upscaled resolution ("TISR", "X3", "super resolution"). — [Hackaday](https://hackaday.com/2025/08/10/cheap-thermal-camera-fits-the-bill)

**Pinecil V2 BLE telemetry (owner already has the iron)**
- IronOS exposes three BLE services on the Pinecil V2: Live (`d85ef000-168e-4a71-AA55-33e27f9bc533`), Settings (`f6d80000-5a10-4eba-AA55-33e27f9bc533`) and Bulk (`9eae1000-9d0d-48c5-AA55-33e27f9bc533`). Live values are unsigned 32-bit little-endian: tip temperature, setpoint, DC input voltage, handle temperature, power level, power source, tip resistance, uptime, time of last movement, max settable temperature, raw tip reading, hall sensor, operating mode, estimated wattage. A single bulk read returns all live values. Notifications are not fully implemented, so clients must poll. — [IronOS Bluetooth docs](https://ralim.github.io/IronOS/Bluetooth)
- BLE mode in the iron's Advanced menu is Off / On (read+write) / Read-only; in read-only mode all writes are rejected and only the physical device can change the mode. — [IronOS Bluetooth docs](https://ralim.github.io/IronOS/Bluetooth)
- BLE apps need IronOS 2.21 or newer on the V2. Existing clients: PineSAM (browser UI via local script), Joric's Web-Bluetooth page with graphing, an ESPHome ESP32 relay, `pynecil` (Python, MIT, `get_live_data()`, settings writes), and the official Home Assistant `iron_os` integration (since HA 2024.8). — [Pine64 development projects](https://pine64.org/documentation/Pinecil/Development_projects); [pynecil](https://github.com/tr4nt0r/pynecil)
- `pinecil2mqtt` also exists (surfaced in search, not examined). — [ecosyste.ms listing](https://awesome.ecosyste.ms/projects/github.com%2Fexcieve%2Fpinecil2mqtt)

### Inferences
- **Recommended camera layout (DO NOW, about $25-50):** one Camera Module 3 on a printed gooseneck/boom at roughly 45 degrees to the board (the angle the SolDef_AI authors used for joint labels), plus optionally a second top-down unit for board-level context. The autofocus IMX708 at 12 MP gives enough pixels to crop to a single joint; frame rate is irrelevant for judging a finished joint (stills suffice), and 1080p30-50 is ample for process review. A printable diffuser ring matters more than the sensor because bare solder is specular; cross-polarized lighting (polarizer film on LEDs plus a rotated analyzer on the lens) is the standard glare cure (background knowledge, unverified; no priced source fetched).
- **USB UVC microscope (DO NOW/PHASE 2, about $60):** fine for SMD close-ups, but check video resolution before buying: the Plugable unit streams only 640x480 video, so the agent should request stills. Cheap "1600x" coin microscopes are the same class.
- **HDMI microscope through PiKVM capture (PHASE 2, about $288):** technically clean (microscope HDMI out into the PiKVM V4 Mini HDMI in; the agent grabs snapshots through kvmd) and gives the human a lag-free screen, but it costs more than a quarter of the whole project budget. It also consumes one PiKVM's capture input. Good later upgrade, not a first purchase.
- **Global-shutter camera, HQ camera with C-mount macro, trinocular scope camera:** not needed for joint review of a stationary board. PHASE 2 at most; no prices were verified in this pass.
- **What the vision LLM can credibly do:** process checks (was flux applied, was the pad and lead heated together, how long was dwell, did solder flow to both pad and lead), gross defects (bridges, unsoldered pins, missed joints, wrong orientation/part, lifted lead), and before/after diffs. **What it cannot be trusted for:** subtle insufficient-wetting/cold joints, hidden joints, and exact pin identification from one view. UniPCB, InspectVLM and Chat Debugging all point the same way; no published evaluation of Claude specifically on solder joints was found.
- **Hybrid path (PHASE 2):** a small YOLO/Mask R-CNN detector fine-tuned on a few hundred of the owner's own joints (Luxonis got a working demo from about 50 custom images plus a public set) would localize joints and flag candidates; the LLM then reasons over tight crops. The public datasets are SMT-only, so a through-hole hand-soldering set would have to be self-collected; the bench itself can collect it automatically.
- **Iron telemetry is the cheapest "sensor" on the bench (DO NOW, $0):** polling the Pinecil bulk characteristic gives tip temperature, power and motion timestamps. A dip in tip temperature with a simultaneous power spike marks a joint being made, giving per-joint dwell time and tip-recovery time without any vision. McAleer's project used a current clamp for the same purpose; BLE gives a richer signal for free. Set the iron's BLE mode to read-only so the agent can observe but never change the setpoint.
- **Rust integration:** no Rust Pinecil BLE client was found; the protocol is simple (poll one bulk characteristic, parse u32 LE fields), so a native client in the Rust MCP server is a small task; `pynecil` is the reference implementation. The CM4 in a Raspberry Pi class sensor pack has BLE on board (background knowledge).
- **Thermal camera (PHASE 2, about $75-100 for 24x32; roughly $250-400 for 256x192, unverified):** useful for appliance repair and finding shorted/hot parts and for monitoring a hot plate, not for judging a hand-solder joint (pixels are far larger than a joint at any practical distance, and shiny solder has low emissivity; the emissivity point is background knowledge).
- **Latency:** with a cloud multimodal model the practical loop is "user finishes joint or says 'check', frame is cropped, answer returns in a few seconds". No reliable measured figure was found; the only concrete numbers are CircuitSense's 15-second scan cadence and McAleer's change-gated frames. Design for post-joint review and spoken summaries, not live intervention.
- **Fume/heat:** keep the camera out of the flux plume (oblique mounting does this) and put a cheap clear filter over the lens; flux residue on optics is the long-term failure mode (background knowledge, unverified).

### Gaps
- No published evaluation of Claude models on solder-joint or PCB-defect benchmarks was found (UniPCB omitted Claude).
- No open through-hole hand-soldering dataset with IPC-A-610 class labels was confirmed; IPC-A-610 criteria themselves were not retrieved (paid standard).
- PiKVM V4 Mini maximum HDMI capture resolution/frame rate, and the Andonstar's HDMI output mode, were not confirmed from primary pages.
- Prices for Raspberry Pi HQ camera, Global Shutter camera, Arducam autofocus modules, Eakins trinocular cameras, ring lights and polarizer film were not verified.
- No measured end-to-end latency for a camera-to-Claude-to-speech loop was found.

---

## 2. Agent-readable breadboards

### Takeaway
The Jumperless V5 is the only shipping product that makes a breadboard natively agent-readable and agent-writable, and a Rust MCP server for it already exists (`jumperless-mcp`, 42 tools), but at about USD 349+ it is more than a third of the total project budget, so it is a Phase 2 purchase. The do-now path is a DIY "instrumented rails + declared netlist" breadboard (current/voltage telemetry on the rails, a handful of ADC taps, and an agent-maintained netlist), because the evidence says LLMs cannot reliably read breadboard wiring from photos.

### Cited Findings

**Jumperless V5 (Architeuthis Flux)**
- Crowd Supply lists the V5 with probe at $349 (page text still carries campaign-era wording about free shipping). The campaign raised $101,305 and closed 2024-11-07. — [Crowd Supply product page](https://www.crowdsupply.com/architeuthis-flux/jumperless-v5)
- The final campaign update (2024-11-07) said the price would rise after the campaign with shipping charged separately (ordering before close saved $20), that shipping to backers was targeted for mid-February 2025, and that Crowd Supply would stock it "at least through April 2026". — [Crowd Supply update](https://crowdsupply.com/architeuthis-flux/jumperless-v5/updates/a-farewell-to-campaigns)
- Mouser has an Architeuthis Flux manufacturer page listing the Jumperless V5 (four +/-8 V 300 mA supplies, selectable Nano header pins); price and stock were not visible on the fetched page. — [Mouser](https://www.mouser.in/new/architeuthis-flux/)
- Specs from the product page: analog crossbar matrix with about 85 ohm per connection; four individually programmable +/-8 V supplies at 300 mA each; -9 V to +9 V range with overvoltage protection; ten GPIO; 451 RGB LEDs under the holes; a probe; measures voltage, current, resistance, frequency and digital data; onboard Python interpreter with calls such as `connect`, `measure`, `outputSine`; netlist-style serial commands (example: `f 5-gnd, 23-adc_1, 4-20, gpio_2-nano_reset,`); a desktop app that follows Wokwi projects; hardware, firmware and app are open source. — [Crowd Supply product page](https://www.crowdsupply.com/architeuthis-flux/jumperless-v5)
- `jumperless-mcp` 0.1.0 (Rust, MIT, by Samantha Edward) is an MCP server that holds the V5's USB serial port and wraps the on-device MicroPython API via raw REPL. 42 tools: board state get/set (with ASCII view), connect/disconnect/clear, slots, `dac_set/get`, `adc_get`, `adc_get_stats`, `ina_get_current/voltage/power`, GPIO set/get/direction, waveform generator, OLED and probe tools, LED overlays, and `dev_exec_python`. Prebuilt binaries cover Windows x86_64, macOS arm64 and Linux x86_64 (not Linux arm64); only one process can hold the port; works around a `get_state()` bug on firmware 5.6.6.2. — [docs.rs README](https://docs.rs/crate/jumperless-mcp/latest/source/README.md)

**Original Jumperless**
- $299 on Tindie, shown in stock; CH446Q crosspoint switches at about 85 ohm per connection, rated 15 mA per connection and 100 mA per chip; MCP4822 DAC (0-5 V and +/-8 V outputs); four 12-bit ADCs; two INA219 current/voltage sensors; +/-8 V range. — [Tindie](https://www.tindie.com/products/architeuthisflux/jumperless/)

**Sandwizz**
- As of May 2024 the Sandwizz (Microaware) was unreleased with a Kickstarter planned for June 2024; Hackaday flagged vague documentation and no price, and noted the open-source Jumperless was already purchasable. — [Hackaday](https://hackaday.com/2024/05/13/sandwizz-promises-to-reinvent-the-breadboard)

**Instrument boards**
- Bus Pirate 5 base kit: $129.20 at Adafruit (marked "no longer stocked" there); speaks 1-Wire/I2C/SPI/UART from a serial terminal, shows voltage on every pin, has a 1-5 V 400 mA programmable supply with 0-500 mA current limit and current monitoring. — [Adafruit](https://www.adafruit.com/product/6017)

**Why vision alone is not enough**
- PCEval (Jan 2026): on 50 Arduino projects, LLMs did well on logical circuits and code but poorly on physical breadboard layouts: Claude 3.7 Sonnet 65.6% logical vs 13.6% physical; GPT-4o 26.8% physical; best model o3-mini 45.2% physical. Pin conflicts (several connections assigned to one hole) were the main failure. Feeding error logs back raised o3-mini's overall score from 61.7% to 76.5%. Evaluation used the Wokwi simulator. — [PCEval, arXiv 2601.02404](https://arxiv.org/html/2601.02404)
- Chat Debugging: image misreads of breadboards (rails, pins, bent wires) were the dominant LLM error source. — [arXiv 2608.02955](https://arxiv.org/pdf/2608.02955)
- The image-to-netlist research that surfaced (SINA, 96.47% netlist accuracy) works on drawn schematic images, not photos of breadboards. — [SINA, arXiv 2601.22114](https://arxiv.org/abs/2601.22114)

### Inferences
- **Jumperless V5 (PHASE 2; about $349+ shipping, availability unconfirmed):** best technical fit by a wide margin: the agent can make and break connections, set rail voltages, read voltage/current, and light LEDs under holes to show the human where to plug a part, and the MCP server is already Rust (same stack as the project). Limits that matter for this bench: about 85 ohm per hop and (on the original's CH446Q) 15 mA per connection, +/-9 V ceiling, so it suits logic/sensor prototyping, not motor, LED-strip or drone power work. For the Pi-class pack, build `jumperless-mcp` from source with cargo (no arm64 binary published), or run it on the Mac.
- **Original Jumperless ($299):** not worth it versus V5 at about $50 more.
- **DIY instrumented breadboard (DO NOW, roughly $15-40, prices unverified):** (a) INA219/INA226 high-side sensors on each power rail; (b) an analog mux (CD74HC4067 class) feeding one ADC, with 8-16 flying taps the user plugs into nodes of interest; (c) a Pico/ESP32 running MicroPython or a Rust firmware as the "breadboard probe", exposing GPIO, I2C scan, UART sniff and ADC to the MCP server. This reads data from the board but does not know the topology.
- **Topology via declared netlist plus electrical verification (DO NOW, software only):** the agent keeps the intended netlist (from a KiCad/Wokwi/Fritzing design or from the user's spoken description) and verifies it by measurement (continuity between tap points, expected rail voltages, I2C address scan, current draw in range) instead of by reading a photo. PCEval's finding that error-log feedback lifted success by about 15 points supports a measure-and-correct loop.
- **Computer-vision netlist from a breadboard photo (PIE IN THE SKY for now):** no mature open project was found, and the two studies above show current VLMs mis-assign holes and rails. A fixed overhead camera with a known breadboard position (hole grid registered once) would make hole-level localization tractable with classic CV, but occlusion by components and arcing jumpers remains. Reasonable as an assistive cross-check ("I see something in row 23; your netlist says row 22"), not as ground truth.
- **Bus Pirate 5/6 (PHASE 2, about $130):** valuable as a protocol tool and pin-voltage monitor; overlaps with a Pico-based probe at ten times the price. Buy only when bus debugging becomes frequent.
- **Digital twin:** the Jumperless app already polls Wokwi projects; for a DIY board, the equivalent is diffing a KiCad netlist against measured node voltages.

### Gaps
- Jumperless V5 actual November 2026 price and stock (Crowd Supply, Mouser, any Tindie listing) not confirmed; the only stock statement found dates from November 2024.
- Whether the V5 uses the same CH446Q limits (15 mA per connection) as the original was not confirmed on a V5 page.
- Sandwizz: no evidence of shipment found; current status unknown.
- Prices and agent-integration status for Analog Discovery 3, ADALM2000, Espotek Labrador, Glasgow, Bus Pirate 6, and Pico-based scopes/logic analyzers were not verified in this pass.
- No project that reconstructs a netlist from a breadboard photograph was confirmed (a Devpost entry "PinPal" surfaced but was not examined: [Devpost](https://devpost.com/software/pinpal-smpl9r)).

---

## 3. Robot arms under about USD 300, control stacks, and what "controlled through the PiKVM" can mean

### Takeaway
The SO-101 follower (about USD 122 in parts plus filament, six Feetech STS3215 bus servos, LeRobot-native, existing MCP servers, Rust servo crates) is the only arm that fits the budget, and it is suited to coarse helper tasks (hold, hand over, press, reposition a camera or light), not to probing test points, SMD placement or soldering. Precision electronics actuation is better served by a Cartesian gantry (the AutoProber pattern) or by fixtures, and "through the PiKVM" realistically means the PiKVM supplies eyes and simple GPIO/servo/relay actuation while the servo bus hangs off another Linux host.

### Cited Findings

**SO-100 / SO-101 (TheRobotStudio + Hugging Face)**
- Official US BOM totals: follower only $121.94; leader + follower $229.88. Six STS3215 servos per arm at $13.89 each (US). Follower uses the 7.4 V 1/345-gear servo (stall torque 16.5 kg-cm at 6 V); a 12 V variant has 30 kg-cm. — [SO-ARM100 README](https://github.com/TheRobotStudio/SO-ARM100)
- Printing: plate files are provided for 220x220 mm beds and 205x250 mm beds, and a table of individual part files is also provided; sample printers listed are "Prusa MINI+, UP Plus 2, Creality Ender 3, Bambu Lab A/P/X-series"; PLA+, 0.4 mm nozzle at 0.2 mm layers, 15% infill, supports everywhere except slopes over 45 degrees. The README does not mention the A1 mini or a 180 mm bed. Optional printable add-ons include an overhead camera mount, wrist camera mounts (32x32 UVC modules, RealSense), and a TPU compliant gripper. — [SO-ARM100 README](https://github.com/TheRobotStudio/SO-ARM100)
- Hugging Face's estimate for DIY non-printed parts is about $130; Seeed Studio's motor kit opened pre-orders at $220 with printed parts $35 extra. SO-101 improved wiring and removed the need to pull gears during assembly. — [Hackster](https://www.hackster.io/news/hugging-face-launches-the-so-101-an-upgraded-low-cost-3d-printable-autonomous-robot-arm-532360f441eb)
- A 2026 buying guide calls SO-ARM101 the lowest entry cost of the LeRobot platforms and the easiest to collect data for (one fixed scene camera plus host); Koch v1.1 uses Dynamixel servos; policy training may need an NVIDIA GPU. — [OpenELAB](https://openelab.io/blogs/learn/lerobot-hardware-buying-guide-so-arm101-vs-koch-v1-1-vs-openarm-vs-lekiwi-vs-xlerobot)

**Waveshare RoArm-M2-S**
- 4-DOF, ESP32, 12-bit magnetic encoders (0.088 degree joint feedback), ST3215 bus servos (30 kg-cm at 12 V), JSON commands over UART/USB/HTTP/ESP-NOW, ROS 2 support, open-source control code, 12 V 5 A supply, workspace up to 1090 mm diameter. Waveshare states unidirectional positioning repeatability of about +/-4 mm under the same load. Listed $179.99-$299.99 across variants. — [Waveshare](https://www.waveshare.com/roarm-m2-s.htm)
- Street price in Nov 2024: $223.99 on Amazon, $206.07 on AliExpress; RoArm-M2-Pro $332.11; payload 0.5 kg at 0.5 m. — [CNX Software](https://www.cnx-software.com/2024/11/23/waveshare-roarm-m2-esp32-robotic-arms-offer-four-degrees-of-freedom-various-servo-options/)

**Arms above budget**
- Elephant Robotics (Americas store, 2026-07-29): myCobot 280 is $499 (Arduino), $649 (M5), $759 (Pi), $809 (Jetson Nano); cheapest desktop arm is the myPalletizer 260 M5 at $299. — [GrabARobot price guide](https://grabarobot.com/blog/elephant-robotics-price-guide-2026/)
- LumenPnP v4 pick-and-place: $1,995, ships in under 3 weeks, handles parts down to 0402. — [Opulo](https://www.opulo.io/products/lumenpnp)

**Gantry / flying-probe alternative**
- AutoProber (GainSec, April 2026): an AI agent drives a SainSmart Genmitsu 3018-PROVer V2 CNC carrying a USB microscope and a spring pogo probe on a printed toolhead; it photographs the board in tiles with XYZ positions, stitches a map, labels parts, and proposes probe targets on a dashboard. A human must approve every target. Safety: an independent optical endstop monitored on an oscilloscope channel (at least 10 Hz), feed-hold on any ambiguity, no automatic recovery. No accuracy figures or cost given. License is PolyForm Noncommercial (source-available, not open source). — [GitHub](https://github.com/gainsec/autoprober); [GIGAZINE](https://gigazine.net/gsc_news/en/20260424-autoprober-diy-flying-probe/)
- Other leads surfaced but not examined: apertus pcb-prober — [GitHub](https://github.com/apertus-open-source-cinema/pcb-prober); CircuitScout — [Hackaday.io](https://hackaday.io/project/192272-circuitscout/details); Make's PantoProbe manual probe holder — [Make](https://makezine.com/2017/10/05/build-simple-pantaprobe-high-precision-electronics-troubleshooting).

**Control stacks**
- `robot_MCP` (Ilia Larchenko): MCP server for SO-ARM100/101 and LeKiwi arm, stdio/SSE/streamable-HTTP, tested with Claude Desktop, Cursor, Windsurf. The author reports Claude gave the best results, Gemini middling, GPT poor (qualitative only), and warns that camera images returned over MCP count as input tokens and get expensive. A LeRobot update broke compatibility (joint states are now normalized, requiring a degree mapping after each recalibration). — [mcpservers.org listing](https://mcpservers.org/servers/IliaLarchenko/robot_MCP)
- A second SO-101 MCP server (`titansage02/so101-mcp`) is listed; not examined. — [LobeHub](https://lobehub.com/en/mcp/titansage02-so101-mcp)
- Rust: `rustypot` (Pollen Robotics, Apache-2.0, v1.14.0 dated 2026-11-17) supports Dynamixel families and Feetech STS3215 with sync read/write and Python bindings. — [docs.rs](https://docs.rs/crate/rustypot/latest). The `st3215` crate (v0.0.2, March 2025) is self-described as "works for me" maturity, sans-io, targeting embassy on ESP32. — [docs.rs](https://docs.rs/crate/st3215/latest). Other crates exist (`sts3215-controller`, `cu-feetech`) but were not examined. — [docs.rs](https://docs.rs/crate/sts3215-controller/latest/source/Cargo.toml.orig)
- SmolVLA: 450 M parameters; Hugging Face states it can run on CPU or a MacBook and train on one consumer GPU; real-world SO-100 success 78.3% with community-data pretraining vs 51.7% without (on their benchmark tasks); async inference finished tasks about 30% faster; fine-tune example is 20,000 steps at batch 64. — [Hugging Face blog](https://huggingface.co/blog/smolvla). An MLX port for Apple Silicon exists (not examined). — [PyPI mlx-smolvla](https://pypi.org/project/mlx-smolvla/)

**PiKVM as controller**
- kvmd's GPIO subsystem has drivers for raw GPIO (libgpiod), USB HID relays, hardware PWM, a `servo` driver built on PWM (configured in angles; the documented SG90 example presses at 45 degrees and releases at 20), a `cmd` driver that runs a command with arguments as the `kvmd` user (no shell operators), plus IPMI, Wake-on-LAN, Philips Hue, ANEL PDUs, ezCoo and Extron serial switchers. On V4 hardware PWM is enabled with `dtoverlay=pwm,pin=12,func=4`; the fan conflict does not apply to the V4 Mini (no fan). Channels appear as LEDs/switches/buttons in the web UI. — [PiKVM GPIO docs](https://docs.pikvm.org/gpio)
- The V4 Mini's listed ports are HDMI in, USB 2.0 OTG, a front USB 2.0 serial-console port, RJ-45 ATX, Ethernet; USB 3.0 host ports are listed only for the V4 Plus. — [PiKVM V4 docs](https://docs.pikvm.org/v4/)

### Inferences
- **Does the SO-101 print on a Bambu A1 mini (180 mm cube)?** Probably yes using the individual part files rather than the pre-arranged 220 mm plates: the README names the Prusa MINI+ as a sample printer, and that machine also has a 180x180x180 mm build volume (Prusa spec is background knowledge, unverified here). No source explicitly confirms the A1 mini; check the longest link in the slicer before buying servos.
- **SO-101 follower only (PHASE 2, about $122 parts + roughly $10-15 filament; fits the $300 tier):** buy the follower alone and skip the leader: the agent commands joint targets through MCP, and teleoperation/imitation learning (which needs the leader, another $108) is a later step. Feasible uses: third hand that holds and re-presents a board to the camera, moves a camera/light to a requested viewpoint, presses buttons/toggles on a device under test, hands over a tool, flips a board (with a fixture). No repeatability figure is published; with hobby bus servos and printed links, expect millimetre-class error and visible backlash (the comparable Waveshare arm with the same servo family quotes +/-4 mm).
- **Task vs precision reality check:** 0.1-inch headers are 2.54 mm pitch and 0805/0603 parts are 2.0/1.6 mm long (background knowledge), so a +/-4 mm class arm cannot place a multimeter probe on a chosen pin open-loop, and cannot place SMD parts. Camera-in-the-loop servoing can improve this but LLM-in-the-loop servoing is slow and token-hungry (robot_MCP's own warning). **Hand soldering by a hobby servo arm is PIE IN THE SKY:** it needs sub-millimetre positioning, controlled contact force and coordinated solder feed, with a 300 C+ tool; nothing found shows this working at hobby cost.
- **RoArm-M2-S (PHASE 2 alternative, about $180-225):** no printing, ESP32 with a JSON/HTTP API that is trivial to wrap in an MCP tool, but 4-DOF, a 1 m class reach that is oversized for a bench, and not LeRobot-native. Choose it only if avoiding the print/assembly work matters more than ecosystem.
- **Gantry / flying probe (PHASE 2 to PIE IN THE SKY depending on scope):** for "put a probe on this test point" a Cartesian machine is the right tool, and AutoProber is direct proof that an agent-driven 3018 CNC + USB microscope + pogo pin works with human approval gates. A 3018-class CNC costs roughly $150-250 (unverified). AutoProber's noncommercial license means its code cannot be merged into an open-source project; copy the architecture (tile-scan, target proposal, human approval, independent safety channel), not the code. Repurposing the owner's A1 mini as the motion platform is unverified and risky (closed firmware, heated bed, his only printer).
- **OpenPnP / LumenPnP ($1,995):** out of budget; PIE IN THE SKY for this project.
- **Koch v1.1, myCobot 280 ($499+), PAROL6, Annin AR4, Thor, Arctos, Moveo, used Dobot:** all exceed the lane budget or were not price-verified; none offers an advantage over SO-101 for an agent-first bench at under $300. Later phase at best.
- **VLA on the MacBook M3 16 GB (PHASE 2+):** SmolVLA inference is plausible on Apple Silicon per Hugging Face's own claim and the existence of an MLX port; fine-tuning on the Mac is likely slow and the buying guide points to an NVIDIA GPU. Larger VLAs (pi0, OpenVLA, GR00T) were not verified for 16 GB Macs. More important: a VLA needs 50+ teleoperated demonstrations per task (typical LeRobot practice; background knowledge) which needs the leader arm. For this bench, scripted joint-space moves and named poses exposed as MCP tools deliver value far sooner than a learned policy.
- **What "controlled through the PiKVM" realistically means:**
  1. **PiKVM as eyes (best fit):** HDMI capture of an HDMI microscope or camera; agent pulls snapshots from kvmd.
  2. **PiKVM as simple actuator (DO NOW, a few dollars):** kvmd's built-in `servo` driver on GPIO12 hardware PWM is literally a button-pressing finger (SG90 class), and `hidrelay`/GPIO can switch relays for power to a device under test. ATX outputs are opto/relay-style closures meant for power/reset buttons and can be reused for low-voltage switching (confirm ratings in the V4 Mini datasheet first).
  3. **PiKVM as HID injector:** USB OTG keyboard/mouse emulation into a device under test or into another controller.
  4. **PiKVM as servo-bus host (poor fit):** the Mini exposes OTG and a console port rather than general USB host ports, the OS is a read-only appliance image, and kvmd owns the hardware. Running a Feetech bus from it is possible in principle (CM4 UART or a `cmd` hook) but fights the appliance. Put the servo bus adapter on the Pi sensor pack or the spare Home Assistant Yellow (a CM4 carrier with USB host; background knowledge) and let that host run the Rust `rustypot` driver behind the MCP server.
- **Safety for an agent-driven arm near a hot iron (design requirements, inference):** physical e-stop that cuts servo power (not software); a separate servo supply switched by a relay the agent cannot override; torque/current and speed caps in firmware (STS3215 exposes load and current; background knowledge); a software keep-out volume around the iron stand and the human's hand zone; "arm moves only when the Pinecil reports idle/in-stand or the user holds a dead-man pedal"; every motion plan announced and human-approved at first (AutoProber's approval gate and no-auto-recovery rule are a good template); bolt or clamp the base so a stall cannot tip it into the work.

### Gaps
- No measured repeatability/backlash figure for SO-100/SO-101 was found.
- No explicit confirmation that all SO-101 parts fit a 180 mm bed (A1 mini).
- RoArm-M3 pricing, Koch v1.1 BOM, PAROL6/AR4/Thor/Arctos/Moveo costs, used Dobot/uArm/xArm 1S prices were not verified.
- Which pins/headers are physically accessible on the PiKVM V4 Mini (GPIO12, UART) and the electrical ratings of its ATX outputs were not confirmed (datasheet PDF exists: [v4mini datasheet](https://docs.pikvm.org/v4/v4mini_datasheet.pdf), not read).
- Whether pi0/OpenVLA/GR00T run usefully on an M3 with 16 GB was not researched; ROS 2/MoveIt support for SO-101 was not verified.
- Price of a 3018-class CNC in Nov 2026 not verified.

---

## 4. Other bench observability worth adding

### Takeaway
The best cheap additions are things with a documented local API: a metering smart plug for appliance repair (about USD 21), iron telemetry (free), a parts inventory with an existing MCP server (free software), and local voice. The owner's PinePower supplies have no documented data interface that this research could find, so supply telemetry needs either a programmable supply with an open protocol or an inline INA226 logger.

### Cited Findings
- Shelly Plug US Gen4: $21.24 single (regular $24.99; listed as pre-order shipping in 2-3 weeks on the fetch date), 15 A / 1800 W (resistive), measures power, energy, voltage and current, local MQTT / WebSocket / HTTPS / webhooks, on-device scripting, Matter by default with optional Zigbee firmware. — [Shelly US](https://us.shelly.com/products/shelly-plug-us-gen4-white)
- Riden RD60xx supplies speak Modbus with published register documentation and several open libraries (Python `Riden`, `rd6006`, an ESP "riden-dongle", a Modbus-TCP bridge). — [Joy-IT Modbus protocol PDF](https://www.joy-it.net/files/files/Produkte/JT-PS1440-C-Pro/MODBUS_Protocol_RD60xx_PSxx.pdf); [ShayBox/Riden](https://www.github.com/ShayBox/Riden); [riden-dongle](https://github.com/morgendagen/riden-dongle)
- Alientek DP100: USB HID protocol reverse-engineered from the vendor DLL; open drivers include `webdp100` (WebHID, Apache-2.0; needs the supply switched to "USBD" mode and a USB-A to USB-A cable; udev rule for Linux), `pydp100` and `open_dp100`. — [webdp100](https://github.com/scottbez1/webdp100); [pydp100](https://github.com/palzhj/pydp100); [open_dp100](https://github.com/lessu/open_dp100)
- IronOS also supports the Miniware MHP30 hot plate, but Adafruit deprecated its flashing guide because the manufacturer reportedly changed the chipset and flashing may brick newer units. Nothing found on MHP50 open firmware. — [Adafruit Learn](https://learn.adafruit.com/installing-ironos-on-a-mhp30-mini-hotplate.md)
- An open reflow-controller project (OmniReflow) surfaced; not examined. — [GitHub](https://github.com/pfalcon/OmniReflow)
- InvenTree has an MCP server (hvkshetry/inventree-mcp, Python, MIT): 12 tools covering 117 operations, including parts, stock, barcode, label and attachment operations; Glama marks maintenance as inactive. A second InvenTree MCP plugin and a Part-DB MCP endpoint also exist (not examined). — [Glama](https://glama.ai/mcp/servers/hvkshetry/inventree-mcp); [InvenTree MCP plugin](https://glama.ai/mcp/servers/eljefedelrodeodeljefe/inventree-mcp-plugin); [Part-DB docs](https://docs.part-db.de/api/mcp.html)
- McAleer's workshop agent used local faster-whisper on a Mac for speech and plans a directional microphone next, i.e. workshop audio capture was a weak point. — [Wevolver/Arduino](https://www.wevolver.com/article/the-self-documenting-workshop-by-kevin-mcaleer)
- Local push-to-talk / wake-word dictation tools for macOS built on Whisper exist in quantity (for example `whisper-ptt`, `brabble`, `heyvox`); not examined. — [whisper-ptt](https://inlet.github.io/whisper-ptt/); [brabble](https://github.com/steipete/brabble); [heyvox](https://pypi.org/project/heyvox/)
- Ted Yapo's 2018 AR Workbench (projector + camera) estimated about $200 for a minimal system; the projector (800x480 LED unit, about $80) was "the limiting factor"; it needed a close-up lens to focus at bench distance; projector light contaminates the camera image (workarounds: blank during capture, or IR markers); board height distorts the projection; the project was shelved before calibration was finished. — [Hackaday.io](https://hackaday.io/project/90456-ar-workbench)
- UW/CMU's ARDW (2022) tracked the PCB with cameras, projected overlays, integrated with KiCad as a plugin, highlighted probe points and recorded measurements; it was judged expensive, bulky, and inaccurate on very small components. — [Hackster](https://www.hackster.io/news/this-augmented-reality-workbench-provides-magical-pcb-debugging-functions-447254d0d259)

### Inferences (with ratings)
- **Metering smart plug for device under test (DO NOW, $21):** gives the agent live watts/amps/volts for any appliance being repaired and a remote off switch; power signature before/after repair is a cheap regression test. Works alongside Home Assistant on the Yellow.
- **Dim-bulb tester / isolation transformer / GFCI for mains repair (DO NOW for safety, cost unverified):** not observability, but a metering plug downstream of a series-bulb limiter lets the agent see the fault current collapse without the human touching live parts. The agent should treat mains work as observe-and-advise only.
- **Pinecil-driven fume extractor (DO NOW/PHASE 2):** fan on when BLE says tip is hot and moving; a VOC/PM sensor in the duct corroborates "soldering in progress" and logs exposure. IronOS docs themselves mention fume extractors as a BLE monitoring use.
- **Bench supply telemetry (PHASE 2):** no data interface for PinePower Desktop/V2 was found. Options: inline INA226 on the output (a few dollars, DO NOW) or a programmable supply with open protocol (Riden RD6006 class via Modbus, or Alientek DP100 via USB HID; both have open drivers; prices not verified). The Jumperless V5's own rails would cover low-power cases.
- **Parts inventory with MCP (DO NOW, software only):** InvenTree on the Mac or Yellow plus the existing MCP server lets the agent answer "do we have a 10k 0805" and decrement stock; bin QR labels read by the overhead camera close the loop.
- **Voice interface (DO NOW, $0-20):** local Whisper on the M3 plus a USB foot pedal (appears as a keyboard key; price unverified) for push-to-talk avoids wake-word false triggers in a noisy shop. Speak answers through the Mac.
- **Hot plate/reflow (PHASE 2):** MHP30 + IronOS is risky on current hardware revisions; an open controller with a thermocouple the agent can read is safer. Thermal camera pairs with this.
- **Projected AR (PIE IN THE SKY for now):** both prior systems struggled with projector resolution, close focus, calibration and small parts. A servo-steered laser dot or LEDs under the board gives 80% of "point at this" for under $15 (see section 7).
- **ESD strap monitor, LCR tweezers with data link, label printer, bench scale (PHASE 2):** useful but no priced, API-verified products were confirmed in this pass.
- **Automatic build logging (DO NOW, software):** McAleer's result (about GBP 0.40 per 90-minute session, usable tutorial drafts) shows change-gated stills + transcript + end-of-session summarization into the repo is cheap and works today.

### Gaps
- PinePower Desktop / PinePower V2 data/telemetry capability: no documentation found either way.
- Current prices for Riden RD6006, Alientek DP100 (LCSC page was dead), MHP50, LCR tweezers, USB foot pedals, air-quality sensors, bench scales, label printers not verified.
- ESD wrist-strap monitors with a data output: nothing found.
- Shelly metering accuracy not stated on the product page.

---

## 5. Prior art for "AI watches you work at the bench" (2025-2026): what worked and what failed

### Takeaway
Several 2026 projects prove the pattern (camera + speech + multimodal LLM, with cheap non-vision signals gating the expensive vision calls), and they agree on the failure mode: the model confabulates when it cannot actually see or read the thing, so instruments and structured data must outrank pixels.

### Cited Findings
- **Self-documenting workshop (Kevin McAleer, July 2026):** built in about four hours on an Arduino UNO Q + overhead webcam + Mac; Claude Sonnet 4.6 per-frame, Opus 4.7 for the end-of-session write-up; iron current clamp as activity signal; about GBP 0.40 per 90 minutes. Worked: component ID, step ordering, photo selection. Failed: invented code details it could not read from the screen; glossed over a ten-minute debugging detour. Planned fixes: direct file access instead of reading the screen, directional mic. — [Wevolver/Arduino](https://www.wevolver.com/article/the-self-documenting-workshop-by-kevin-mcaleer)
- **CircuitSense (March 2026):** Gemini Live voice model plus a separate Gemini 2.5 Flash vision pass every 15 s; modes for inventory, physical-fault diagnosis (burnt traces, cold solder, swollen caps), wiring diagrams and teaching; open repo. Self-reported successes only, no error analysis. — [DEV Community](https://dev.to/kryptonus_vicky/building-circuitsense-how-i-built-a-dual-engine-ai-lab-partner-that-sees-your-hardware-3mf4)
- **AutoProber (April 2026):** agent-driven CNC flying probe with mandatory human approval of each probe target and an independent hardware safety channel; 335 GitHub stars; noncommercial license. — [GitHub](https://github.com/gainsec/autoprober)
- **Chat Debugging study (2025 data, 17 students):** LLMs often listed the true root cause among their first suggestions without fine-tuning, yet image misreads and overconfidence derailed sessions, and a correct answer did not guarantee the human fixed the circuit in time. The authors suggest agentic or 3D visual tools to overcome image limits. — [arXiv 2608.02955](https://arxiv.org/pdf/2608.02955)
- **PCEval (Jan 2026):** LLMs handle logical circuits and code far better than physical breadboard layouts; feeding back simulator error logs helps. — [arXiv 2601.02404](https://arxiv.org/html/2601.02404)
- **robot_MCP (2025):** Claude was the best of three model families at driving an SO-100 through MCP with camera images; token cost of images is the stated practical limit. — [mcpservers.org](https://mcpservers.org/servers/IliaLarchenko/robot_MCP)
- **Imperial College / Gracia "PCB Hand-Soldering Basics" (Aug 2026):** thirteen Gaussian-splat lessons letting learners view hand soldering from any angle in VR; shows that fine tool/joint detail can be captured volumetrically, but capture hardware is not described. — [Hackaday](https://hackaday.com/2026/08/18/watch-soldering-up-close-and-from-any-angle-in-vr/)
- **AR workbenches:** Ted Yapo's AR Workbench (2018, shelved before calibration) and UW/CMU ARDW (2022, KiCad plugin, worked but costly and weak on small parts). — [Hackaday.io](https://hackaday.io/project/90456-ar-workbench); [Hackster](https://www.hackster.io/news/this-augmented-reality-workbench-provides-magical-pcb-debugging-functions-447254d0d259)
- **Luxonis solder-defect demo:** small custom dataset + edge camera on a stereo microscope; no metrics. — [Luxonis](https://docs.luxonis.com/overview/industry-examples/electronics/solder_defect_detection)

### Inferences
- The designs that worked share three traits worth copying: (1) a cheap non-vision trigger decides when to spend vision tokens (current clamp; here Pinecil BLE and frame differencing); (2) the model reads source data directly whenever it exists (files, serial logs, instrument values) rather than photographing a screen; (3) a human approval gate sits in front of any physical action.
- The consistent failure is confident confabulation from ambiguous images. The MCP server should return confidence-relevant metadata with each frame (focus score, glare fraction, pixels per mm) and the agent prompt should require an "I cannot see X; show me Y" response path.
- No product or project was found that specifically coaches hand soldering in real time with an LLM; this bench would be early prior art, which is a reasonable differentiator for an open-source project.

### Gaps
- Arduino's "Nibsy" documentation project (May 2026) could not be fetched (redirect loop): [Arduino blog](https://blog.arduino.cc/2026/05/13/nibsy-makes-manual-project-documentation-obsolete/).
- No commercial "AI soldering coach" product was identified; absence is not proof.
- No study measuring whether AI bench coaching improves solder quality or repair success was found.

---

## 6. What is achievable at about USD 50, 150 and 300 for this lane, and what is later

### Takeaway
About USD 50 buys the core loop (one autofocus camera, iron telemetry, metered outlet, a servo "finger" on the PiKVM); USD 150 adds a second view/microscope and a DIY instrumented breadboard; USD 300 adds the SO-101 follower arm. The Jumperless V5 and an HDMI microscope each cost more than the entire USD 300 tier and belong to a later phase.

### Cited Findings (prices reused from above)
- Camera Module 3 from $25 — [Raspberry Pi](https://raspberrypi.com/products/camera-module-3)
- Shelly Plug US Gen4 $21.24 — [Shelly US](https://us.shelly.com/products/shelly-plug-us-gen4-white)
- Plugable UVC microscope $59.95 — [Plugable](https://plugable.com/products/usb2-micro-250x)
- MLX90640 breakout $74.95 (out of stock) — [Adafruit](https://www.adafruit.com/products/4469)
- SO-101 follower BOM $121.94; leader + follower $229.88 — [SO-ARM100 README](https://github.com/TheRobotStudio/SO-ARM100)
- RoArm-M2-S $179.99-$223.99 — [Waveshare](https://www.waveshare.com/roarm-m2-s.htm); [CNX Software](https://www.cnx-software.com/2024/11/23/waveshare-roarm-m2-esp32-robotic-arms-offer-four-degrees-of-freedom-various-servo-options/)
- Jumperless V5 $349 (campaign-era listing) — [Crowd Supply](https://www.crowdsupply.com/architeuthis-flux/jumperless-v5)
- Andonstar AD246S-M $288 — [Newegg](https://www.newegg.com/p/3A1-003D-00015)
- Bus Pirate 5 $129.20 — [Adafruit](https://www.adafruit.com/product/6017)
- myCobot 280 from $499; LumenPnP $1,995 — [GrabARobot](https://grabarobot.com/blog/elephant-robotics-price-guide-2026/); [Opulo](https://www.opulo.io/products/lumenpnp)
- kvmd `servo`, `pwm`, `hidrelay`, `cmd` drivers are built in — [PiKVM GPIO docs](https://docs.pikvm.org/gpio)

### Inferences (proposed tiers; small-part prices marked ~ are unverified estimates)
- **Tier about $50 (all DO NOW):** Camera Module 3 ($25) on a printed boom with printed diffuser; Pinecil V2 BLE polling (owned, $0); Shelly metering plug ($21); SG90 servo button-presser on PiKVM GPIO12 (~$3); INA219/INA226 on breadboard rails (~$5); local Whisper push-to-talk on the Mac ($0); change-gated build logging to the repo ($0). Total about $55.
- **Tier about $150 (adds about $95):** second view, either a second Camera Module 3 top-down ($25) or the UVC microscope ($60); DIY breadboard probe (Pico/ESP32 + 16-channel analog mux + pogo/flying taps, ~$15); 2-servo laser pointer "agent finger" (~$10); WS2812 multi-angle light ring for glare-free and photometric shots (~$10); HX711 load cell under the mat (~$10); VOC sensor in the fume path (~$10).
- **Tier about $300 (adds about $135-150):** SO-101 follower only ($122 + filament), driven from the Pi pack or spare HA Yellow via `rustypot`, exposed as named-pose MCP tools, with a hardware e-stop and relay-switched servo supply (~$10). Alternative use of the same money: MLX90640 ($75) plus a programmable supply with open protocol (price unverified).
- **Later phase (each breaks the lane budget):** Jumperless V5 ($349+); HDMI microscope into PiKVM ($288); leader arm + SmolVLA fine-tuning (+$108 and GPU time); 3018-class CNC flying probe (~$150-250 plus scope integration); 256x192 thermal camera; projected AR; pick-and-place ($1,995).
- **Sequencing note:** the task brief sets USD 1,000 for the portable pack and bench together; the $300 tier here assumes this lane gets about 30% of it. If the lane gets less, drop the arm first: every pre-arm item improves what the agent knows, while the arm only changes what it can do, and observability was the stated priority.

### Gaps
- Small-part prices (servos, INA boards, mux, load cell, LEDs, laser module, pedal) are estimates, not sourced.
- Filament cost and print time for the SO-101 not sourced.

---

## 7. Outside-the-box bench observability and actuation ideas (not in the brief)

### Takeaway
The cheapest wins come from treating existing tools as sensors (the iron, the light, the mat, the outlet) and from letting the human be the manipulator while the agent points and verifies, which sidesteps the precision limits of a sub-USD 300 arm.

### Cited Findings (facts these ideas build on)
- Pinecil BLE exposes tip temperature, power level, estimated wattage and time of last movement, pollable in one bulk read. — [IronOS Bluetooth docs](https://ralim.github.io/IronOS/Bluetooth)
- kvmd has a built-in angle-based `servo` driver and USB HID relay driver. — [PiKVM GPIO docs](https://docs.pikvm.org/gpio)
- A current clamp on the iron was enough to tell an agent when soldering was happening, saving vision tokens. — [Wevolver/Arduino](https://www.wevolver.com/article/the-self-documenting-workshop-by-kevin-mcaleer)
- Projector-based AR suffered from projector resolution/focus limits, camera contamination and calibration burden. — [Hackaday.io AR Workbench](https://hackaday.io/project/90456-ar-workbench)
- Jumperless V5 uses 451 RGB LEDs under the holes to communicate with the user in place. — [Crowd Supply](https://www.crowdsupply.com/architeuthis-flux/jumperless-v5)
- Agent-proposed, human-approved probe targets on a stitched board map worked in AutoProber. — [GitHub](https://github.com/gainsec/autoprober)
- LLM success on physical circuits improved markedly when error logs were fed back. — [PCEval](https://arxiv.org/html/2601.02404)
- The SO-101 project ships printable overhead and wrist camera mounts. — [SO-ARM100 README](https://github.com/TheRobotStudio/SO-ARM100)

### Inferences (the ideas; all are original proposals, not found as existing products)
1. **Joint-event detector from iron thermodynamics (DO NOW, $0).** Each joint shows up as a tip-temperature sag plus a wattage spike plus a motion pause. Segment the session into numbered joints from BLE data alone, record dwell time and recovery time per joint, and attach the matching video frames. The agent can then say "joint 14: 9 s dwell, twice your median; check for a lifted pad", a quantitative process metric no camera can give.
2. **Photometric light ring (DO NOW/PHASE 2, ~$10).** A ring of individually addressable LEDs around the lens, fired one sector at a time over 6-8 frames with the camera fixed. Differences between frames reveal fillet shape (concave wetted vs convex blob) and remove specular glare by per-pixel median. Feeds the LLM a glare-free composite plus a shape map instead of one shiny photo. (Tiered/colored lighting is how commercial AOI reads fillets; background knowledge.)
3. **The "agent's finger": two-servo laser dot (DO NOW, ~$10).** A 5 mW-class laser module on a pan-tilt pair, calibrated against the overhead camera with fiducials, lets the agent point at the pad, pin or part it is talking about. kvmd's servo driver or the Pi can run it. Far cheaper and safer than an arm near the iron, and it avoids the projector problems prior AR benches hit. Use a low-power module and keep the beam below eye level.
4. **Fiducial work mat (DO NOW, ~$2 of printing).** AprilTag/ArUco markers at the corners of the mat give every frame metric scale and pose, so the agent can speak in millimetres, register before/after frames exactly for diffing, calibrate the laser pointer, and later hand coordinates to an arm or gantry.
5. **Human-as-manipulator probe docks (DO NOW, ~$15).** Two or three spring-loaded probe arms (PCBite-style, printable) permanently wired to ADC/INA channels. The agent lasers the test point, the human parks a probe there once, and the agent then reads it continuously. This delivers "place a probe on a test point" today with zero robot precision.
6. **Load cell under the mat (PHASE 2, ~$10).** A weighed work surface detects parts placed or removed, counts identical parts by weight for inventory, measures how hard the user presses a connector or the iron, and timestamps "hands on the board" versus "hands off" for build logs.
7. **Acoustic and vibration channel (PHASE 2, ~$5-10).** A contact mic on the bench plus the room mic: relay clicks, motor/bearing noise, arcing, capacitor whine and buzzer tones become spectrogram snippets the agent can compare before and after a repair. Especially useful for homestead appliance repair where "it sounds wrong" is the symptom.
8. **Golden power signature per appliance (DO NOW with the metering plug).** Record inrush and steady-state watts for each appliance when healthy; on a later fault or after a repair the agent diffs the trace. A repaired device must match its golden trace before it leaves the bench.
9. **Spoken shadow netlist (DO NOW, software).** As the user builds, they say "red from row 12 to plus rail"; the agent maintains the netlist and immediately verifies each statement electrically through the rail sensors and probe taps, flagging mismatches. This replaces photo-based topology reading, the thing LLMs do worst, with the two things they do well: language and reasoning over measurements.
10. **Under-board LED row indicator for a plain breadboard (PHASE 2, ~$10).** A translucent breadboard over an addressable LED matrix or strip gives a rough version of the Jumperless "light the hole" guidance for a few dollars; row-level, not hole-level, accuracy.
11. **Printed bed-of-nails jigs for recurring boards (PHASE 2, ~$10 per jig in pogo pins).** For the home-sensor boards built repeatedly, the A1 mini prints a fixture with pogo pins wired to the probe mux. Dropping a board in the jig gives the agent every test point at once: actuation solved by fixture instead of robot.
12. **Arm as camera/light operator, not as hands (PHASE 2, with the SO-101).** Mount a wrist camera on the SO-101 (mounts are in the repo) and let the agent choose its own viewpoint: orbit a joint for three oblique views, or hold a light at a raking angle. Millimetre-class error is harmless for viewpoint control, so this plays to a cheap arm's strengths and directly raises inspection quality.
13. **Iron-left-on and presence interlock (DO NOW, software + metering plug).** Combine Pinecil "time of last movement", tip temperature and a human-presence sensor (the owner is building these anyway): hot iron + no presence for N minutes triggers a spoken warning, then cuts the outlet. A safety feature that falls out of the telemetry already being collected.
14. **Frame-quality metadata as a first-class MCP field (DO NOW, software).** Every image the MCP server returns carries focus score, glare fraction, pixels per mm and time since last motion; the agent is instructed to refuse to judge a joint below thresholds and ask for a re-shot. This targets the documented confabulation failure directly.

### Gaps
- None of these ideas were validated against an existing implementation; component prices are estimates.
- Laser eye-safety classification and suitable modules were not researched.
- Whether translucent breadboards with adequate light transmission are readily available was not checked.
