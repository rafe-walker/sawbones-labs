# Field-testing a suspected failing solar panel with low-cost, agent-readable instruments (state: 2026-11-25)

Conventions used in these notes
- Prices are USD, read from the linked page on 2026-11-25 unless marked otherwise. "UNVERIFIED" = from the researcher's background knowledge or a search-result title only, not confirmed on a fetched page in this session.
- Hazard levels: **NONE** (no electrical contact), **LOW-VOLTAGE** (one isolated module, at most about 60 V DC open-circuit, at most about 15 A), **HAZARDOUS DC** (anything electrically connected to a string, combiner or inverter PV input: here probably 150-450 V DC).
- Instrument rule by level: NONE = anything. LOW-VOLTAGE = commercial handheld meters, cheap PV testers, and a home-built tester only if battery-powered and galvanically isolated from the Pi (wireless link, no USB tether). HAZARDOUS DC = only commercial CAT III 1000 V (or 1500 V) DC-rated meters, leads and clamps; never home-built, never USB-tethered.
- System facts given by the requester (not researched): about 7 kW ground-mount array (about 2022), peaks about 5,000 W, two paralleled EG4 3000EHV-48, six 5 kWh 48 V LiFePO4 batteries, Solar Assistant. Panel model, string layout, combiner/fusing and rapid-shutdown equipment are unknown.

---

## 1. Failure modes of crystalline-silicon modules/arrays in hot desert climates, their signatures, and "is 5 kW from 7 kW normal?"

### Takeaway
Fleet data put typical loss at about 0.5-0.9 %/year (faster in hot zones), so a 2022 array should have lost only about 2-4 % to ageing; a 5 kW peak from 7 kW STC is mostly explained by cell temperature, sub-1000 W/m2 irradiance, soiling/wiring, and (very likely here) inverter-side limits: off-grid curtailment when batteries are full and the EG4's 18 A PV input-current ceiling. A real fault shows up as an asymmetry (one string vs the other, one substring vs its neighbours), not as "below nameplate".

### Cited Findings
- NREL PV Fleet study (Jordan et al., 2022; about 1,700 sites, 7.2 GW, about 19,000 inverters): overall median system performance-loss rate -0.75 %/yr (inverter level), P90 -1.90 %/yr; by temperature zone for conventional c-Si ground mounts the medians are -0.48 (T3, cooler), -0.78 (T4) and -0.88 %/yr (T5, hotter); trend statistically significant. — [NREL fy22osti/81314](https://nrel.gov/docs/fy22osti/81314.pdf)
- Earlier NREL literature survey (Jordan & Kurtz): median 0.5 %/yr, mean 0.8 %/yr over 2,128 reported rates; a manufacturer recommendation cited there uses -0.7 %/yr for hot climates and -0.5 %/yr elsewhere. — [NREL fy13osti/56690](https://www.nrel.gov/docs/fy13osti/56690.pdf)
- IEA PVPS Task 13 failure review (2014): c-Si degrades roughly 0.8 %/yr; LID appears right after installation (about 2-4 %, normally already inside the rated power); field failures at about 8 years: junction box and cables 12 %, burn marks on cells 10 %, encapsulant failure 9 %; early failures are dominated by junction-box failure, glass breakage, defective cell interconnect, loose frame, delamination. — [IEA PVPS T13-01:2014](https://iea-pvps.org/wp-content/uploads/2020/01/IEA-PVPS_T13-01_2014_Review_of_Failures_of_Photovoltaic_Modules_Final.pdf)
- Signature table from the same report: bypass-diode failure = I-V steps, lower Voc, one substring warmer in IR, possible burn spot; cell cracks = inactive cell parts, part of a cell warmer; PID = lower Voc, sloped I-V, cells near the frame warmer; delamination/browning = lower Isc; burn marks/solder failure = heating along a busbar; glass breakage = hot spots and insulation loss; disconnected ribbons = drastic power drop. — [IEA PVPS T13-01:2014](https://iea-pvps.org/wp-content/uploads/2020/01/IEA-PVPS_T13-01_2014_Review_of_Failures_of_Photovoltaic_Modules_Final.pdf)
- Bypass-diode failure modes (PV-Tech technical paper): a short-circuited diode shorts its 20- or 24-cell substring, giving about one-third power and Voc loss in a three-diode module and a dark substring in EL; an open-circuit diode has no effect until a cell in that substring is shaded, at which point the shaded cell is reverse-biased by the others, with very high temperatures and fire risk. Causes: forward-bias over-temperature under partial shade, thermal runaway when shade is removed, long-term high-temperature reverse bias in hot climates, thermal cycling, ESD. Detection: Voc check and IR and I-V find shorted diodes; open diodes need I-V under deliberate partial shade or a dedicated diode checker; visual inspection is unreliable. No field failure-rate statistics are given. — [PV-Tech bypass diode faults](https://www.pv-tech.org/wp-content/uploads/legacy-publication-pdfs/75d3699f25-pv-bypass-diode-faults-current-testing-and-scope-for-future-test-development.pdf)
- Solmetric (Hernday, SolarPro): in a 72-cell, three-diode module one shorted diode costs about 10-12 V; shading a module that has a shorted diode produces a smaller-than-expected Voc drop (basis of the selective-shading test). — [Solmetric SolarPro article](https://www.solmetric.com/wp-content/uploads/2022/11/SolarPro-I-V-feature-article-Hernday.pdf)
- Kiwa PVEL 2026 Module Reliability Scorecard (lab stress tests of new products, not field rates): 87 % of manufacturers had at least one failure (83 % in 2025); junction-box-related failures 21 % of manufacturers, bypass-diode failures 17 %, delamination 45 %, breakage 60 %, power-loss failures 26 % (40 % of those after PID testing). — [Kiwa PVEL scorecard failures page](https://scorecard.pvel.com/failures)
- Soiling in the desert Southwest (NREL regional soiling stations): Arizona sites showed soiling rates of -0.02 %/day (Hyder), -0.05 %/day (Avra Valley) and -0.06 %/day (Gilbert) during dry periods, with average soiling ratio above 99 % over the monitoring periods; the soiling ratio is the Isc of a soiled reference cell divided by that of a cleaned one. — [NREL fy17osti/69131](https://www.nrel.gov/docs/fy17osti/69131.pdf)
- EG4 3000EHV-48 PV input: 5,000 W rated solar input per unit, max PV open-circuit voltage 500 V DC, MPPT range 120-450 V DC, one MPPT, max PV current draw 18 A, max solar charge current 80 A, RS-232/RS-485 comms, parallel up to 12 units. — [EG4 3000EHV-48 spec sheet](https://eg4electronics.com/wp-content/uploads/2024/05/EG4-3000EHV-48-Spec-Sheet.pdf)
- Fluke/Solmetric fault-to-curve mapping also lists PV connectors and junction-box corrosion as series-resistance faults (low Vmp/Voc) and PID as low Voc and Isc with a flattened curve. — [Fluke troubleshooting](https://www.fluke.com/en-us/learn/blog/renewable-energy/troubleshooting-pv-systems-using-i-v-curve-tracers); [Fluke I-V analysis](https://www.fluke.com/en-us/learn/blog/renewable-energy/analyze-iv-curves)

### Inferences
- Worked budget for "5 kW from 7 kW" (illustrative arithmetic, coefficients assumed, not measured): cell at 65 C with -0.35 %/C gives 0.86; four years at 0.5-0.9 %/yr plus initial LID gives about 0.95-0.97; soiling 0.97-0.99; DC wiring/mismatch 0.97-0.98. Product about 0.77-0.80, i.e. about 5.4-5.6 kW at a full 1000 W/m2 and about 5.0-5.2 kW at 930-950 W/m2. So 5.0 kW on a hot clear day is inside the normal band; on a cold clear spring day (cell 35-40 C) a healthy array should reach about 6.3-6.6 kW if nothing limits it.
- Two system-specific limits must be excluded before blaming a panel: (a) off-grid curtailment: an all-in-one only harvests load plus battery charging; once the bank is near full the MPPT backs off toward Voc and "peak PV" understates the array. Only samples taken while charging in bulk (battery not full, PV voltage near Vmp rather than Voc) are valid for diagnosis. (b) The 18 A PV current ceiling per unit: if any inverter has two strings in parallel of modern 10-14 A modules, the PV current flat-tops at 18 A and the daily curve is clipped; this is a design limit, not a failed panel. A flat-topped PV current trace at about 18 A is the signature.
- Expected string size: with a 500 V limit and about 40-50 V module Voc, strings are probably 6-10 modules, so string Voc is probably about 250-450 V. One shorted bypass diode is then about 10-13 V, roughly 3-5 % of string voltage: detectable only by comparing two like strings at the same instant or by tracking Voc against temperature.
- Signature cheat-sheet for the agent (synthesised from the sources above):
  - Lost about one-third of one module's voltage, substring warm 2-6 K (active/shorted diode), step in I-V: shorted bypass diode or dead substring.
  - No symptom until shade, then a very hot cell: open bypass diode (fire risk; needs shade-and-trace or reverse-current test).
  - One cell much hotter (above about 10-20 K): cracked/shunted cell, bird dropping, soiling patch.
  - Low Isc, normal Voc: soiling, browning/delamination, uniform degradation, irradiance reference wrong.
  - Low Vmp/Voc (soft vertical leg), hot connector or junction box: series resistance (MC4 crimp, mismatched connectors, solder bonds, corroded ribbon).
  - Low Imp/Isc (sloping top), patchwork IR pattern, worse at the negative end of the string: shunting/PID.
  - Whole string at zero current with normal Voc: blown fuse, open connector, open breaker; string at zero volts: open circuit upstream or short.
  - Whole module warmer than neighbours by a few K: module open-circuited (not delivering power).
  - Time-of-day notch that moves with the season: shading (vegetation, post, neighbouring row).
- Typical single-module values to expect on a label (UNVERIFIED, from general datasheet knowledge): 60-cell/120-half-cell 300-370 W: Voc about 40-41 V, Isc about 9.5-11.5 A; 72-cell/144-half-cell 400-450 W: Voc about 49-50 V, Isc about 11-11.5 A; 108-half-cell 400-430 W (182 mm cells): Voc about 37-39 V, Isc about 13.5-14 A. Cold-morning Voc of a 72-cell module can exceed 50 V.

### Gaps
- No desert-specific failure-mode frequency table (e.g. Sandia/NREL counts of diode vs connector vs backsheet failures in Arizona) was retrieved; the 2014 Task 13 percentages are old and not desert-specific.
- LeTID, backsheet-cracking (polyamide/AAA backsheets) and snail-trail specifics were not sourced in this session.
- Temperature coefficient range (-0.3 to -0.4 %/C) and 60-70 C Arizona cell temperatures are the requester's figures; not independently sourced here. PVWatts hourly output includes a modelled cell temperature (`tcell`) that can stand in.
- Whether the EG4 3000EHV-48 reports PV insulation/ground-fault status over Modbus, and its PV-side topology (isolated or not), was not found.

---

## 2. Zero-contact checks from data the system already has

### Takeaway
The strongest free diagnostic is ratiometric: compare the two inverters' PV voltage, current and power at the same instant (irradiance and temperature cancel), then compare the clear-day curve shape with a clear-sky/transposition model. Absolute performance ratio without an on-site sensor is only good for gross errors; an in-plane reference cell is what turns it into a few-percent measurement.

### Cited Findings
- Solar Assistant publishes over MQTT with topics of the form `solar_assistant/inverter_N/<metric>/state` (example `solar_assistant/inverter_1/load_power/state`), status on `solar_assistant/status`, optional Home Assistant discovery under `homeassistant/#`; MQTT is enabled on the Configuration tab and all topics can be listed with `mosquitto_sub -h <ip> -p 1883 -v -t '#'`. The help page does not enumerate PV topic names. — [Solar Assistant MQTT help](https://solar-assistant.io/help/integration/mqtt)
- The EG4 exposes RS-232 and RS-485; one MPPT per unit, so two units give two independently measured PV inputs. — [EG4 spec sheet](https://eg4electronics.com/wp-content/uploads/2024/05/EG4-3000EHV-48-Spec-Sheet.pdf)
- NREL PVWatts V8 API: API key required, 1,000 requests/hour; inputs system_capacity, module_type, losses, array_type, tilt, azimuth, lat/lon; with `timeframe=hourly` it returns `ac`, `dc`, `poa` (plane-of-array irradiance), `tcell` (module temperature) plus `dn`, `df`, `tamb`, `wspd`; weather is NSRDB typical-year (TMY) data, i.e. a typical day, not today's sky. — [PVWatts V8 docs](https://developer.nrel.gov/docs/solar/pvwatts/v8/)
- Open-Meteo Satellite Radiation API: GHI, DHI, DNI and global tilted irradiance (tilt and azimuth parameters, 20 % albedo, isotropic sky); for the Americas it uses NOAA GOES-19 at 2 km, updated every 10 minutes with about 20 minutes delay; archive from 2025-04-07; hourly by default, native interval on request. Pricing/key terms are on a separate page. — [Open-Meteo satellite radiation API](https://open-meteo.com/en/docs/satellite-radiation-api)
- Solcast hobbyist accounts: FAQ refers to a daily limit of 10 API requests and to PV power estimates (`PvEstimate`); details of sites/limits are not answered on the index page. — [Solcast hobbyist FAQ](https://kb.solcast.com.au/home-hobbyists-faqs)
- pvlib-rust 0.1.6 (published 2026-04-20, Apache-2.0, owner p-vbordei): Rust port covering solar position (SPA), clear sky (Ineichen, Haurwitz, Bird, Simplified Solis), transposition (Perez, Hay-Davies, Klucher, Reindl, isotropic), six temperature models, single-diode (Bishop88, De Soto/CEC/PVsyst parameters), PVWatts DC/AC and five IAM models; 337 tests; the "same accuracy as pvlib-python" statement is the author's own claim. — [pvlib-rust on docs.rs](https://docs.rs/crate/pvlib-rust/latest)
- NREL fleet analysis notes that PERC/SHJ systems showed faster loss below 600 W/m2, and that clipping, data shifts and missing data must be filtered before computing loss rates. — [NREL fy22osti/81314](https://nrel.gov/docs/fy22osti/81314.pdf)
- Solmetric: an irradiance reference should be in the plane of the array with spectral response matched to the modules; I-V testing is most reliable above 700 W/m2; a low temperature reading inflates expected Voc, a wrong irradiance reading falsifies expected Isc. — [Solmetric SolarPro article](https://www.solmetric.com/wp-content/uploads/2022/11/SolarPro-I-V-feature-article-Hernday.pdf)

### Inferences
- Agent checks, in order of diagnostic value, all hazard NONE:
  1. **Curtailment filter**: keep only samples where the battery is charging below absorb voltage and PV voltage is in the Vmp band (roughly 0.76-0.84 of Voc); when curtailed the PV voltage rises toward Voc. Diagnose only on filtered samples.
  2. **A/B string ratio**: V1/V2, I1/I2, P1/P2 between the two inverters, normalised by module count per string. Healthy like strings stay within a few percent all day. A constant voltage deficit of about (one-third module Vmp)/(string Vmp) = shorted bypass diode; a current deficit that is constant all day = soiling/degraded module limiting the string; a deficit only at certain hours = shading; a deficit that grows with current = series resistance.
  3. **Voc harvest**: each morning before the MPPT starts (and whenever curtailed to zero charge), the PV voltage reading is close to string Voc. Plot Voc against modelled cell temperature for months; a step down of about 10-13 V that persists is a shorted substring. Free daily "Voc test" with nobody touching anything.
  4. **Clipping detection**: flat-topped PV current near 18 A, or flat-topped PV power near 5,000 W per unit, or flat-topped total at (load + max charge current x battery voltage).
  5. **Clear-sky shape**: on a cloudless day compare P(t)/P_clear-sky(t) from pvlib (Ineichen + transposition + cell-temperature model). A smooth ratio that sags at midday = temperature or clipping; notches at fixed sun positions = shading; an overall offset = soiling/degradation/model error.
  6. **Performance ratio**: PR = measured DC energy / (nameplate kW x POA insolation in kWh/m2), temperature-corrected PR using modelled cell temperature. Use Open-Meteo GTI for today's actual sky and PVWatts hourly for the "typical day" sanity check.
- Realistic accuracy without an on-site sensor (engineering judgement, UNVERIFIED): clear-sky-day PR to about +/-5-8 % (aerosol and model uncertainty), satellite-based hourly PR worse on partly cloudy days; A/B ratio between two like strings about +/-1-2 % limited by inverter metering. A reference cell in the array plane plus back-of-module temperature should bring absolute PR to about +/-3-5 % after a one-time clear-day calibration.
- A 3-5 % voltage offset between strings also arises if the strings have different module counts, so the agent needs the string map first.
- Modbus polling at 1 Hz also captures the MPPT's own perturb-and-observe dither and any periodic sweep, which gives small free pieces of the in-situ string I-V curve around the MPP (dP/dV sign, local slope).

### Gaps
- Exact Solar Assistant PV topic names for the EG4 3000EHV-48 (expected to be of the form `pv_voltage`, `pv_current`, `pv_power`; UNVERIFIED) and their update rate must be discovered by subscribing to `#`.
- EG4 3000EHV-48 Modbus register map was not retrieved.
- No quantitative source was fetched for satellite/clear-sky irradiance uncertainty or for how much a reference cell improves PR; the figures above are judgement.
- RdTools (NREL degradation and soiling analysis) and pvanalytics (QA, clipping and shading detection) were not fetched in this session; they are Python libraries known to the researcher and would have to be called from, or ported to, the Rust stack. Forecast.Solar and the Solcast Home Assistant integration ([ha-solcast-solar](https://github.com/BJReplay/ha-solcast-solar)) exist but were not examined.

---

## 3. Visual, thermal, electroluminescence and UV inspection

### Takeaway
Thermography is the best zero-contact way to pick the suspect panel, but it needs load (operation near MPP), more than about 600 W/m2 and about 5x5 pixels per cell; a 32x24 MLX90640 only meets that at about 1 m or less (a third of a panel per frame) and is good for hot substrings, junction boxes and connectors, while a 256x192 camera can do whole panels from 2-3 m. On a ground mount, shoot the back sheet (emissivity 0.95, few reflections).

### Cited Findings
- IEA PVPS Task 13 IR/EL review: use irradiance above 600 W/m2; avoid wind and clouds; allow 5-15 minutes to stabilise after conditions change; emissivity about 0.85 for glass at viewing angles of 90-60 degrees and about 0.95 for the polymer backsheet at 90-45 degrees; aim for about 5x5 pixels per cell (pixel 32 mm or less for 160 mm cells); operation near MPP gives the most useful pattern, open circuit shows no pattern and short circuit shows reverse-bias effects; natural inhomogeneity can reach 8 K and differences of 1-2 K need careful interpretation; cell defects with delta-T above 15 C correlate strongly with reduced string current; a hot-spot example exceeds 20 K; active bypass diodes heat; loose combiner connections heat; PID shows warmer cells near the bottom frame/patchwork patterns toward the negative pole; the standards give no pass/fail criteria. — [IEA PVPS T13-10:2018 review](https://iea-pvps.org/wp-content/uploads/2020/01/Review_on_IR_and_EL_Imaging_for_PV_Field_Applications_by_Task_13.pdf)
- IEC TS 62446-3 conditions as summarised in an IEA PVPS presentation: irradiance above 600 W/m2, wind below 28 km/h, low cloud cover, geometric resolution no coarser than 3 cm/pixel. — [Spataru, IEA PVPS 2025 slides](https://iea-pvps.org/wp-content/uploads/2025/08/07_S.-Spataru_Framing-Drone-IRT-Conditions.pdf)
- An Australian training body's summary: irradiance above 600 W/m2 per IEC TS 62446-3:2017, camera at least 320x240, thermal sensitivity below 0.08 C, flag differences above 10 C between like components, shoot as near perpendicular as possible and avoid reflections. — [GSES thermal imaging article](https://www.gses.com.au/solar-pv-system-bess-thermal-imaging/)
- EL basics: silicon emits around 1150 nm; silicon CCD/CMOS sensors are the common detectors and InGaAs is used for special cases; consumer DSLR and mirrorless cameras can be modified; work in a darkened room, at sunset or at night. — [IEA PVPS T13-10:2018 review](https://iea-pvps.org/wp-content/uploads/2020/01/Review_on_IR_and_EL_Imaging_for_PV_Field_Applications_by_Task_13.pdf)
- Low-cost outdoor EL (FH Muenster poster, 2015): inexpensive DSLR with the stock IR filter removed and replaced by sun-blocking filters, exposures of typically 0.5-4 s, a high-voltage supply back-feeding a whole string, camera and supply controlled from a laptop; finds micro-cracks, broken cell connectors, shunted or missing bypass diodes, PID and hot spots. — [FH Muenster low-cost EL poster](https://de.fh-muenster.de/eti/downloads/personen/professoren/mertens/Poster_LowCostEL-EU-PVSEC-Hamburg_2015.pdf)
- A shorted bypass diode's substring appears dark in EL. — [PV-Tech bypass diode faults](https://www.pv-tech.org/wp-content/uploads/legacy-publication-pdfs/75d3699f25-pv-bypass-diode-faults-current-testing-and-scope-for-future-test-development.pdf)
- LG's warranty guidance asks for high-resolution front and back photos, a clear photo of the data label, zoomed photos of the defect, and a photo of the surroundings; an IR image is shown as a sample. — [LG RMA request guidelines](https://www.lg.com/us/solar/solar/files/resources/RMA_Request_Guidelines_Final.pdf)

### Inferences
- Photo checklist for the agent (hazard NONE): label (model, serial, Pmax, Voc, Isc, Vmp, Imp, max system voltage, max series fuse); full front square-on in diffuse light; full back; junction box and cable glands; both connectors and their mating halves (brand markings, to catch mixed "MC4-compatible" pairs); any browning, snail trails, burn marks, bubbles, backsheet chalking/cracks, cracked glass, bird droppings, dirt dams at the lower frame edge; the row and surroundings (vegetation, posts, shadows) with a timestamp; position in the array (row/column) from GNSS or a sketch.
- Resolution arithmetic (FOV values UNVERIFIED from datasheet memory: MLX90640 standard lens about 55x35 degrees; TC001 about 56x42 degrees): MLX90640 at 2 m sees about 2.1 m across, 65 mm per pixel, about 2.5 pixels per 166 mm cell: too coarse for hot cells, enough for a hot one-third substring strip (about 330 mm wide), a hot junction box, or a hot connector held close. At 1 m it is about 33 mm per pixel: meets the 5x5-per-cell guide but covers about 1.0 x 0.6 m per frame, so a panel needs 3-4 overlapping frames. A 256x192 camera at 3 m is about 12 mm per pixel (about 13 pixels per cell) and covers a whole panel; it still falls below the 320x240 that practitioners quote as the minimum for formal IEC-style reports.
- Thermal procedure for the agent: confirm clear sky and more than 600 W/m2 (model or reference cell); confirm the array is under load, not curtailed (switch on a large load or wait for bulk charging) for at least 10-15 minutes; shoot the back at 0-45 degrees off normal with emissivity 0.95; avoid own shadow; record delta-T against the median of neighbouring cells/modules. Triage (practitioner guidance, not a standard's pass/fail): below about 3 K ignore; 3-10 K note and retest; above 10 K investigate; above 20 K, or any hot connector/junction box, treat as a priority and a possible fire precursor. A hot connector should be compared with its neighbours carrying the same current.
- A fully hot one-third strip means that substring is not producing (diode conducting or shorted): check for shade/soil on it first; if it is clean and unshaded the substring or diode has failed.
- EL feasibility: forward-biasing one isolated panel at night needs about its Voc (40-50 V) at 10-100 % of Isc, i.e. a current-limited bench supply of about 60 V/5-10 A. Hazard LOW-VOLTAGE if a single disconnected panel and a current-limited supply are used. Never use the 48 V battery bank directly (no current limit; huge fault current). A de-filtered DSLR or a Raspberry Pi NoIR/HQ camera with an 850-950 nm long-pass filter and exposures of seconds to tens of seconds is a plausible low-cost detector, because silicon sensors retain a little response near 1000-1100 nm; the Pi-camera variant is an untested experiment (UNVERIFIED). String-level EL as on the poster requires a high-voltage supply and is HAZARDOUS DC: out of scope for DIY.
- Dark bypass-diode test (LOW-VOLTAGE, single isolated panel at night or fully covered): apply reverse polarity from a current-limited supply at 1-2 A with the voltage limit set low (below about 5 V). Three healthy Schottky diodes conduct at roughly 1-1.5 V total; if the supply hits its voltage limit with no current, a diode path is open. Reasoning from diode physics, UNVERIFIED as a field procedure; the commercial Hioki tester does a related thing (see section 4).
- Drone thermal: the owner's Mavic 4 Pro has no thermal sensor; a thermal drone is a separate purchase (out of scope). Its RGB camera is still useful for soiling, droppings, vegetation and shadow mapping.

### Gaps
- IEC TS 62446-3's own abnormality classes and delta-T tables were not retrieved (standard is paywalled); thresholds above are from Task 13 and practitioner summaries.
- UV-fluorescence imaging (365 nm torch at night to reveal crack patterns in aged EVA) was not sourced in this session; it is covered in the Task 13 review beyond the portion retrieved. UNVERIFIED.
- A pv-magazine item dated 2026-09-23, "Modified consumer camera can measure solar cell open-circuit voltage", was found but could not be fetched (redirect); details unknown. — [pv-magazine link](https://www.pv-magazine.com/2026/09/23/modified-consumer-camera-can-measure-solar-cell-open-circuit-voltage/)
- MLX90640 and TC001 field of view and noise figures were not verified against datasheets in this session.

---

## 4. Electrical tests on one isolated panel and on a string; instruments and prices

### Takeaway
Everything on an intact string is HAZARDOUS DC and limited to a clamp around an insulated conductor or a CAT III 1000 V+ rated meter at proper test points; the informative, safe hands-on work is on one isolated panel (Voc, Isc, I-V, diode test), where a USD 45-190 handheld PV tester or an isolated home-built tracer is acceptable. The cheapest agent-readable string-rated instrument found is the UNI-T UT219PV (USD 399, Bluetooth, CAT III 1500 V).

### Cited Findings
- UNI-T UT219PV: USD 399.00; measures to DC 2500 V and AC 1500 V; CAT III 1500 V / CAT IV 600 V; AC/DC current to 999.9 A; DC power; Bluetooth app logging; IP65; 9,999 counts. — [Transcat UT219PV](https://www.transcat.com/uni-t-ut219pv-solar-pro-ac-dc-clamp-meter)
- UNI-T UT503PV PV insulation tester: USD 598.00; 125/250/500/1000 V test voltages; PV to 1000 V DC; tests insulation of live PV strings without shorting; Bluetooth app and USB-C data. — [Transcat UT503PV](https://www.transcat.com/uni-t-ut503pv-solar-pro-pv-insulation-tester-for-solar-photovoltaic-systems)
- UNI-T UT505B insulation multimeter: USD 329.99; 50-1000 V insulation test; no data link listed. — [Transcat UT505B](https://www.transcat.com/uni-t-ut505b-advanced-insulation-resistance-dmm)
- Hioki FT4310 bypass diode tester: tests diodes for open and short faults from the string end in daylight without covering panels (string must be disconnected from the inverter side first); measures Voc, Isc and bypass-route resistance in about 2 s per string by applying about Isc + 1 A for 5 ms or less; up to 1000 V DC, 2-12 A strings; Bluetooth LE to a phone app; a shorted diode shows as about a 10 V Voc deficit. No price on the page. — [Hioki FT4310](https://www.hioki.com/sg-en/products/pv/solar-panel/id_6647)
- Elejoy EY1600W handheld "MPPT" panel tester: USD 89.00 (list 109.00); 20-120 V operating range (page also says max 60 V), up to 60 A, 1600 W; shows Pmax, Voc, Isc, voltage and current; powered from the panel under test; no accuracy figure, no data output listed. — [ShopSolarKits EY1600W](https://shopsolarkits.com/products/elejoy-1600w-mppt-solar-panel-tester-multimeter)
- FLIR PV48 solar panel tester and I-V curve tracer: USD 189.99; I-V curve up to 800 W per panel (distributor title: 0-60 V); Pmax, voltage, current, Voc, Isc, ambient temperature; supplied clips and MC4 leads rated CAT III 300 V; no data link stated. — [Transcat FLIR PV48](https://www.transcat.com/flir-pv48-solar-panel-i-v-curve-tracer); [TEquipment curve tracers](https://www.tequipment.net/Solar-Analzyers/Curve-Tracers/)
- Diode check by multimeter: a shorted bypass diode shows continuity in both directions; Fluke notes an open-circuit-voltage test alone may miss some diode faults. — [Fluke troubleshooting](https://www.fluke.com/en-us/learn/blog/renewable-energy/troubleshooting-pv-systems-using-i-v-curve-tracers)
- Cheap Bluetooth multimeters with reverse-engineered protocols: the ludwich66 project documents the 10- and 11-byte BLE protocols of the Aneng 9002/V05B/ST207/AN999S, BSIDE ZT-300AB/ZT-5B/ZT-5BQ, ZOYI ZT-300AB/ZT-5566S/ZT-5566SE and BABATools AD900 family; a Rust client exists (GPL-2.0, small, self-described as rough); Owon B41T+ has several open clients. — [ludwich66/Bluetooth-DMM](https://www.github.com/ludwich66/Bluetooth-DMM); [depau/bluetooth-dmm-rust](https://github.com/depau/bluetooth-dmm-rust); [art-ya/OwonB41T](https://github.com/art-ya/OwonB41T)
- Owon CM2100B: Bluetooth AC/DC mini clamp, 600 V, 100 A, 20,000 counts (from distributor listing titles; page fetch failed). — [Test Equipment Depot listing](https://www.testequipmentdepot.com/owon-cm2100b-smart-ac-dc-mini-bluetooth-clamp-meter-600v-100a-20000.html)
- LG asks, for low-power claims on string-inverter systems, for DC voltage and current at the inverter, module Voc, inverter type, string, isolation resistance at combiner and module, and time of measurement. — [LG RMA request guidelines](https://www.lg.com/us/solar/solar/files/resources/RMA_Request_Guidelines_Final.pdf)

### Inferences
- Test-by-test table (expected values are rules of thumb; use label values and coefficients once known):

| Test | Hazard | Instrument | Good | Bad |
|---|---|---|---|---|
| String operating V and I from inverter | NONE | Modbus/MQTT | Like strings within about 2-3 % | Persistent deficit (see section 2) |
| String current at combiner/disconnect, clamp on insulated conductor | HAZARDOUS DC (enclosure open, no conductor contact) | CAT III-rated DC clamp, ideally 1000 V; Bluetooth for logging | Parallel strings within about 5 % of each other; agrees with inverter | One string at 0 A (open/fuse) or low |
| String Voc at the DC disconnect/combiner terminals, disconnect open | HAZARDOUS DC | CAT III 1000 V DC meter with PV-rated probes only | N x module Voc x [1 + beta x (Tcell - 25)], strings within about 2-3 %; a deficit of about one-third module Voc = shorted substring | Deficit, zero, or reversed polarity |
| Fuse continuity | HAZARDOUS DC unless both sides isolated | Rated meter; fuse removed only with disconnect open and zero current confirmed | Near 0 ohm | Open |
| Insulation resistance of string to ground | HAZARDOUS DC (test set injects 500/1000 V) | PV-specific insulation tester; competent person | Megohms (IEC 62446-1 minimum commonly quoted as 1 megohm; UNVERIFIED) | Low reading, worse when wet at dawn = damaged insulation, cracked backsheet, rodent damage |
| Single-panel Voc (panel isolated, both leads free) | LOW-VOLTAGE | Bluetooth DMM, EY-class tester, PV48 | Label Voc corrected: about -0.25 to -0.30 %/C above 25 C, slight drop at low irradiance | About two-thirds or one-third of expected = shorted diode(s)/substring; near zero = open or dead |
| Single-panel Isc | LOW-VOLTAGE (arc risk if done wrongly) | Only through a tester or DC-rated switch; never by touching or pulling MC4s | Label Isc x G/1000 within about 5-10 % | Low = soiling, degradation, shading, cracked cells |
| Single-panel I-V curve | LOW-VOLTAGE | PV48, or isolated home-built tracer | Smooth knee, fill factor 0.70-0.80 | Steps, soft knee, slopes (section 5) |
| Bypass-diode test | LOW-VOLTAGE | Diode-test function at junction-box terminals if the box opens (many are potted); or the dark reverse-current test in section 3; or shade-one-substring during an I-V trace | About 0.2-0.5 V forward per Schottky, open in reverse | Near 0 V both ways = shorted; open both ways = open |
| Connector/cable voltage drop under load | HAZARDOUS DC | Do not probe; use thermal imaging and the Vmp/Voc ratio instead | Connector at cable temperature | Hot connector, low Vmp/Voc |

- What must never be done with a non-rated or USB-tethered instrument: anything touching a conductor that is still part of a string, a combiner busbar, a fuse holder, or the inverter PV terminals; the INA228 board, the 24-bit ADC module, the Pi's USB instruments and cheap PV testers (60-120 V inputs) all fall in this class. A cheap PV tester connected across a 300-450 V string will fail violently.
- Clamp choice: a 600 V CAT III Bluetooth DC clamp (Owon CM2100B class, roughly USD 60-70, UNVERIFIED price and CAT rating) is adequate for clamp-only current readings on insulated PV wire in a sub-500 V system and gives 0.01 A-class resolution; for any voltage measurement at string level use a 1000 V/1500 V-rated meter (UT219PV at USD 399; Fluke 393 FC is the premium equivalent, price UNVERIFIED at roughly USD 700-800).
- DC clamps drift: zero the clamp away from the conductor immediately before each reading and take readings on both strings within seconds of each other or time-align with inverter data.
- The capacitor-load tracer has a useful safety property: at the end of a sweep the capacitor sits at Voc and current is zero, so the panel can be unplugged with no load current.

### Gaps
- Prices for Fluke 393 FC, Hioki FT4310, Owon CM2100B and the EY800W (commonly listed around USD 40-60 on marketplaces; UNVERIFIED) were not confirmed on fetched pages.
- No fetched source confirms an open BLE protocol for the UT219PV, UT503PV or FLIR PV48; assume phone-app only until reverse-engineered.
- Accuracy of EY800W/EY1600W-class testers is not published on the pages found; treat readings as +/-5 % class until checked against a known panel.
- IEC 62446-1 test sequence and limits (polarity, Voc, Isc, insulation resistance thresholds) were not retrieved.

---

## 5. I-V curve tracing on a budget

### Takeaway
Commercial tracers run from about USD 190 (single-panel FLIR PV48) to USD 3,500-17,000 (string-rated Seaward, HT, Fluke/Solmetric); the open-source IV Swinger 2 shows a capacitor-load single-panel tracer can be built for about USD 50 with an Arduino, a relay, two 1000 uF 100 V capacitors, a 5 mohm shunt and a 12-bit SPI ADC. A purpose-built, isolated, wireless single-panel tracer is the right "PV test module" for this project.

### Cited Findings
- Distributor prices (TEquipment "Your Price"): Seaward PV210 complete kit USD 3,514.05; HT Instruments I-V500w USD 5,500.00; HT I-V600 USD 12,000.00; Fluke PVA-1500T2 USD 13,738.81; Fluke PVA-1500HE2 USD 17,314.34; Fluke IRR2-BT irradiance meter USD 555.33; FLIR PV48 USD 189.99. — [TEquipment curve tracers](https://www.tequipment.net/Solar-Analzyers/Curve-Tracers/)
- IV Swinger 2 (Chris Satterlee): open-source hardware and software I-V curve tracer for PV panels, described as "a $50 IV curve tracer"; repository has 559 commits, Arduino sketch plus Python application, docs for optional environmental sensors, hardware scaling, and design/theory; pre-built PCBs available from the author. — [csatt/IV_Swinger](https://github.com/csatt/IV_Swinger)
- IV Swinger 2 module-version build guide: electromechanical relay driven from Arduino pin D2, load capacitors 2 x 1000 uF 100 V, 5 mohm shunt, 47 ohm bleed resistor, MCP3202 ADC, TLV2462 op-amp, 15 A bypass diodes; software GPL v3, hardware TAPR Open Hardware License 1.0; PCBs about USD 25 for three (OSH Park) or about USD 5 for ten plus shipping (PCBWay); desktop app connects over USB. — [IV Swinger 2 PCB build guide](https://content.instructables.com/ORIG/F2T/I6P6/JS0OS2H3/F2TI6P6JS0OS2H3.pdf)
- Open IoT tracer (Gonzalez, Portalo, Calderon, Sensors 2021): Raspberry Pi 3B+ in Python driving a commercial programmable electronic load over RS-232, pyranometer and Pt-100 inputs, Grafana/MariaDB; sweeps take 24-320 s; no repository or licence published. — [Sensors 21(22):7650](https://pmc.ncbi.nlm.nih.gov/articles/PMC8620002)
- Arduino relay-switched resistor tracer (Machado et al., Applied Sciences 2025): Arduino Mega, 8 relays switching 100 W resistors, ACS712 5 A sensor, 10-bit ADC, up to 35 V; 23 points in about 45 s; current uncertainty about +/-2 %. — [Appl. Sci. 15(15):8186](https://www.mdpi.com/2076-3417/15/15/8186)
- Reading the curve (Fluke and Solmetric): steps/notches = mismatch from partial shade, uneven soiling, cracked cells, shorted bypass diodes; low Isc = soiling, shade, degradation or wrong irradiance reference; low Voc = wrong temperature, shorted diodes, PID; rounder knee = ageing; low Vmp/Voc = series resistance (conductors, connectors, solder bonds, junction box); low Imp/Isc = shunt paths, tapered shade/dirt, or irradiance changing during the sweep; healthy performance factor about 90-100 %; new c-Si fill factor 70-80 %. — [Fluke I-V analysis](https://www.fluke.com/en-us/learn/blog/renewable-energy/analyze-iv-curves); [Solmetric SolarPro article](https://www.solmetric.com/wp-content/uploads/2022/11/SolarPro-I-V-feature-article-Hernday.pdf)
- Solmetric practice: tracer should acquire the curve in under 1 s; test above 700 W/m2; in-plane spectrally matched irradiance sensor; 24-30 gauge thermocouple taped to the back, away from edges; an open bypass diode shows up as reduced Isc/steeper top under shade. — [Solmetric SolarPro article](https://www.solmetric.com/wp-content/uploads/2022/11/SolarPro-I-V-feature-article-Hernday.pdf)

### Inferences
- Slow resistive or electronic-load sweeps (tens of seconds) are poorly suited to the field: irradiance drifts during the sweep and the load must dissipate hundreds of watts. A capacitor load sweeps Isc to Voc in milliseconds with no heat sink.
- Sizing (first-order): sweep time is about C x Voc / Isc. With 2,000 uF, 40 V, 10 A: about 8 ms; at 20-50 kS/s that is 160-400 points. With 10,000 uF: about 40 ms (allows a slower ADC). Stored energy 0.5 C V^2 is 1.6 J at 40 V and 3.6 J at 60 V for 2,000 uF: small, but the capacitor must be bled to below 5 V before the next sweep and before anyone handles it (47 ohm gives a time constant of about 0.1 s).
- Parts for a single-panel tracer (about 60 V, 15 A): capacitor bank 2-10 mF at 100 V; DC-capable switching element (logic-level MOSFET at 100 V or more with low Rds, or a relay closed into a discharged capacitor so it never breaks current); precharge-free start from a fully bled capacitor; 2-5 mohm four-terminal shunt; divider for voltage; simultaneous-sampling ADC at 10-100 kS/s; reverse-polarity protection; input fuse; bleed resistor with its own MOSFET; non-conductive IP-rated enclosure; MC4 pigtails. Rough parts cost USD 70-110 including an ESP32 and a small Li-ion cell (researcher's estimate, UNVERIFIED), consistent with IV Swinger 2's roughly USD 50 claim for a USB-tethered Arduino version.
- Is "electronic load module + isolated meter module" a good modular fit? Partly. The isolated meter module (24-bit simultaneous ADC) is exactly the acquisition front end a tracer needs and should be reused; an INA228 is marginal (bus limit 85 V and conversion times of 50 us and up per channel, alternating rather than simultaneous; datasheet figures UNVERIFIED in this session). A general electronic-load module is the wrong load for PV tracing; the "load" should be a dedicated capacitor-sweep board. Recommended split: PV-test front end (switch, capacitors, shunt, divider, protection, voltage-refusal logic) + shared isolated ADC core + shared ESP32 radio.
- STC translation (IEC 60891 procedure 1 in simplified form, UNVERIFIED in this session): Isc_stc = Isc x (1000/G) / [1 + alpha x (Tc - 25)]; Voc_stc = Voc - beta x (Tc - 25) (plus a small logarithmic irradiance term); Pmax_stc approx Pmax x (1000/G) / [1 + gamma x (Tc - 25)]. With +/-5 % irradiance and +/-3 C temperature uncertainty the corrected Pmax is good to roughly +/-6-7 %: enough to prove a one-third loss or a 15 % loss, not a 3 % loss. Comparing the suspect with a neighbouring known-good panel measured minutes apart under the same sky removes most of that uncertainty and is the stronger evidence.
- The cheap EY800W/EY1600W-class "MPPT testers" report Voc, Isc and a maximum-power point but, per the product page found, no curve and no data port; the agent would have to read the LCD via the pack's camera (OCR) or have the user read it aloud. The FLIR PV48 adds a curve display at USD 190 but no documented data link.

### Gaps
- IV Swinger 2 maximum Voc/Isc ratings, sweep duration and point count were not present on the pages retrieved (researcher's recollection is roughly 80 V/10 A for the module version; UNVERIFIED). Date of last commit not retrieved.
- Seaward PV200, HT I-V400w and EKO MP-11 prices were not found on the pages fetched.
- How the FLIR PV48 and EY-series testers sweep internally (capacitive vs electronic load) and their accuracy are not documented on the pages found.
- IEC 60891:2021 procedure text was not retrieved.
- No fetched example of an ESP32/MOSFET open tracer with a public repo and licence other than IV Swinger.

---

## 6. Irradiance and temperature references; array geometry

### Takeaway
A small in-plane silicon reference cell (short-circuited through a shunt) plus a back-of-module temperature probe is the cheapest reference that is spectrally matched to the array; an Apogee SP-110 is about USD 280 and a professional irradiance meter about USD 555, so the DIY cell calibrated on a clear day is the pragmatic choice. Lux sensors are only a "sun is steady / sun is not steady" indicator.

### Cited Findings
- Apogee SP-110-SS self-powered silicon-cell pyranometer: USD 280.00 with 10 m cable at RainWise. — [RainWise SP-110-SS](https://rainwise.com/sp-110-ss-pvmet-replacement-sensor)
- Fluke IRR2-BT solar irradiance meter (Bluetooth): USD 555.33. — [TEquipment curve tracers](https://www.tequipment.net/Solar-Analzyers/Curve-Tracers/)
- Solmetric recommends an in-plane sensor whose spectral response matches the modules and advises against pyranometers and hand-held sensors for I-V work; thermocouple on the back of a representative module away from the cooler edges. — [Solmetric SolarPro article](https://www.solmetric.com/wp-content/uploads/2022/11/SolarPro-I-V-feature-article-Hernday.pdf)
- NREL soiling stations use two PV reference devices (one kept clean), midday readings at 500 W/m2 or more, and the ratio of irradiance-corrected Isc as the soiling ratio. — [NREL fy17osti/69131](https://www.nrel.gov/docs/fy17osti/69131.pdf)
- IV Swinger documents optional environmental sensors for its tracer. — [csatt/IV_Swinger](https://github.com/csatt/IV_Swinger)
- Open-Meteo provides tilted-plane irradiance for a given tilt and azimuth; PVWatts needs tilt and azimuth as inputs. — [Open-Meteo](https://open-meteo.com/en/docs/satellite-radiation-api); [PVWatts V8](https://developer.nrel.gov/docs/solar/pvwatts/v8/)

### Inferences
- DIY reference cell: a small encapsulated mono-Si cell (about 0.5-2 W) loaded by a precision shunt chosen so the cell voltage stays below about 50-100 mV (true short-circuit region), read by a spare ADC channel; add a DS18B20 on its back for temperature correction of Isc (about +0.05 %/C). Calibrate on a cloudless midday against the pvlib clear-sky POA value or a borrowed reference meter; expect about +/-5 % absolute after clear-sky calibration and about +/-1-2 % repeatability, which is what ratio tests need (judgement, UNVERIFIED).
- Mount the reference cell in the exact plane of the array (clip to a frame edge without shading cells); the IMU gives tilt, GNSS plus IMU/compass or a sun-shadow measurement gives azimuth; feed those to the clear-sky/transposition model. A magnetometer near steel racking is unreliable: prefer two GNSS points along a row or a timed shadow.
- BH1750/VEML7700 lux sensors respond to the photopic band only and saturate or need attenuation in full sun; use them for stability detection (is irradiance changing during a sweep?) rather than for W/m2.
- Back-of-module temperature: a DS18B20 or thin thermocouple taped at the centre of a cell near the middle of the module, covered with a small insulating patch; cell temperature is typically 1-3 C above the back-surface reading at full sun (rule of thumb, UNVERIFIED). Alternative with no sensor: infer cell temperature from the panel's own Voc once its healthy Voc is known.
- A permanently mounted reference-cell "scatter pod" (two cells, one wiped weekly) gives continuous PR and a soiling ratio by the NREL station method.

### Gaps
- Apogee SP-110 calibration uncertainty and sensitivity were not on the page fetched (commonly quoted as +/-5 %; UNVERIFIED).
- Prices and accuracy claims of low-cost RS-485/Modbus pyranometers from Chinese vendors (roughly USD 40-80) and the Davis 6450 were not verified in this session.
- No source was fetched quantifying DIY reference-cell accuracy.

---

## 7. Module-level comparison without disconnecting anything; shade test; dawn checks

### Takeaway
Use thermal imaging and a deliberate shade test to find the suspect while the string stays intact, then isolate only that one panel. Covering a healthy panel should remove about 1/N of the string's voltage and power; if covering a panel changes nothing, that panel was already contributing nothing.

### Cited Findings
- Selective shading: shading a module with a shorted bypass diode gives a smaller-than-expected voltage drop. — [Solmetric SolarPro article](https://www.solmetric.com/wp-content/uploads/2022/11/SolarPro-I-V-feature-article-Hernday.pdf)
- Open diodes show only under shade and are hazardous when the substring is shaded. — [PV-Tech bypass diode faults](https://www.pv-tech.org/wp-content/uploads/legacy-publication-pdfs/75d3699f25-pv-bypass-diode-faults-current-testing-and-scope-for-future-test-development.pdf)
- Thermography under load near MPP gives the most informative pattern; delta-T above 15 C correlates with reduced string current. — [IEA PVPS T13-10:2018](https://iea-pvps.org/wp-content/uploads/2020/01/Review_on_IR_and_EL_Imaging_for_PV_Field_Applications_by_Task_13.pdf)
- LG asks for isolation resistance at the combiner and module for low-power claims. — [LG RMA request guidelines](https://www.lg.com/us/solar/solar/files/resources/RMA_Request_Guidelines_Final.pdf)

### Inferences
- Shade-one-panel test (hazard NONE; an opaque cover over the whole panel, no electrical contact; keep the test to a minute or so per panel): with N modules in the string and the MPPT tracking, covering one healthy panel drops string Vmp by about one module Vmp plus about 1.5 V of diode drop and string power by about 1/N (11-17 % for 6-9 modules), current roughly unchanged. Outcomes: full 1/N drop = that panel was healthy; about two-thirds of the expected drop = one substring was already lost; no change = the panel was already fully bypassed; string current collapses instead of voltage stepping = a bypass diode is open (stop, uncover immediately, mark the panel as a hazard).
- Substring variant: shade one cell in each third of the panel in turn; each should cost about one-third of a module Vmp. This maps a suspect to a substring with no tools.
- If the two strings are paralleled into one MPPT, the shade response is diluted (the shaded string drops out of the shared MPP voltage) and the clamp on each string leg becomes the discriminating measurement.
- Two-clamp or walked-clamp comparison at the combiner (HAZARDOUS DC environment, insulated-conductor clamp only): readings within seconds, normalised by inverter PV current logged at the same timestamp.
- Dawn/dew check: insulation faults show up when modules are wet. Without an insulation tester the agent can watch for inverter ground-fault/isolation alarms at dawn and after rain, and correlate with humidity; a megger-style measurement is a rated-instrument, competent-person task.
- Most arrays have no exposed per-panel test points; the agent should not suggest back-probing MC4s or piercing insulation.

### Gaps
- No fetched source gives numeric expectations for the shade test; the figures above are circuit reasoning.
- Whether the EG4 3000EHV-48 raises PV isolation alarms is unknown.

---

## 8. Rapid shutdown, optimizers and microinverters

### Takeaway
If module-level electronics are present they change everything: strings read about 0-1 V per module until a keep-alive is present, and the vendor portal already holds per-panel data. A 2022 off-grid ground mount on all-in-one inverters most likely has none, but the agent must confirm by photo before interpreting any voltage.

### Cited Findings
- SunSpec rapid shutdown reduces module output to 1 V per module (versus the 80 V in-array limit of NEC 2017) through a multi-vendor communication scheme. — [SunSpec Rapid Shutdown fact sheet](https://sunspec.org/wp-content/uploads/2025/01/SunSpec-RapidShutdown-FactSheet-05-2019-revB_web-2.pdf)
- Tigo: TS4-F and TS4-2F drop to 0.6 V per unit within about 30 s of shutdown; TS4-S and TS4-O drop to zero; shutdown is triggered when the transmitter (CCA or RSS transmitter) loses AC power and stops sending the keep-alive. — [Tigo support article](https://support.tigoenergy.com/hc/en-us/articles/201873956)
- LG's claim process for microinverter systems asks for seven days of per-module DC voltage and current for the affected module and a good neighbour, exported from monitoring. — [LG RMA request guidelines](https://www.lg.com/us/solar/solar/files/resources/RMA_Request_Guidelines_Final.pdf)

### Inferences
- Decision rule for the agent: photograph the back of two panels. A box between panel and string wiring (Tigo, SolarEdge, APsystems, Enphase or similar) means: (a) a string voltage of about N x 0.6-1 V is normal in shutdown, not a fault; (b) per-panel data probably exists in the vendor system and should be read first; (c) the shade test and Voc reasoning apply per optimizer output, not per string; (d) a single-panel test must be made at the panel's own leads, upstream of the device.
- A low "safety voltage" string is not proof of a de-energised array: the modules themselves are live in daylight on the panel side of each device.
- Rapid-shutdown rules in NEC 690.12 concern PV on or in buildings; a detached ground mount is commonly outside that requirement (UNVERIFIED; point the owner to NEC 690.12 and the local authority, not legal advice).

### Gaps
- SunSpec keep-alive signalling details (PLC frequency, timing) and NEC 690.12 limits were not on the page fetched.
- Enphase, SolarEdge and APsystems data-access specifics were not researched.

---

## 9. Safety procedure the agent must enforce

### Takeaway
The agent's job is to keep the homeowner at hazard NONE for as long as possible, allow LOW-VOLTAGE work only on one isolated, covered panel, and refuse to guide any step that opens a connector or touches a conductor until isolation and zero current are confirmed by a reading it can see.

### Cited Findings
- Open bypass diodes under shade can drive cells to very high temperatures with fire risk. — [PV-Tech bypass diode faults](https://www.pv-tech.org/wp-content/uploads/legacy-publication-pdfs/75d3699f25-pv-bypass-diode-faults-current-testing-and-scope-for-future-test-development.pdf)
- EG4 PV input up to 500 V DC open-circuit, 120-450 V operating. — [EG4 spec sheet](https://eg4electronics.com/wp-content/uploads/2024/05/EG4-3000EHV-48-Spec-Sheet.pdf)
- Supplied leads of a single-panel tester are rated only CAT III 300 V, underlining that such tools are for single panels. — [Transcat FLIR PV48](https://www.transcat.com/flir-pv48-solar-panel-i-v-curve-tracer)
- Glass breakage is associated with insulation loss. — [IEA PVPS T13-01:2014](https://iea-pvps.org/wp-content/uploads/2020/01/IEA-PVPS_T13-01_2014_Review_of_Failures_of_Photovoltaic_Modules_Final.pdf)

### Inferences
- Facts the agent must collect before any hands-on step: panel label (manufacturer, model, serial, Pmax, Voc, Isc, Vmp, Imp, maximum system voltage, maximum series fuse; temperature coefficients from the datasheet); modules per string; strings per inverter and whether paralleled; combiner presence, fuse/breaker ratings and whether fuse holders are load-break (most are not); location and type of each DC disconnect; grounding/bonding of frames and racking; any module-level electronics; connector brand(s); conductor routing; tilt and azimuth.
- Isolation sequence for removing one panel from a string (the only disconnect the agent should guide):
  1. Do it at dawn/dusk or with the panel and its neighbours covered; dry conditions; a second person present; no jewellery; insulated gloves and eye protection; insulated tools.
  2. Turn off PV charging if the inverter allows, then open the DC disconnect for that string (photo of the handle in OFF).
  3. Verify zero current: DC clamp, zeroed, on the string conductor reads below 0.1 A, streamed or photographed. The agent proceeds only on seeing this.
  4. Only now open the panel's two connectors with the proper tool; cap or bag the string-side connector ends: they are still live to the rest of the string and to ground paths in daylight.
  5. Test the isolated panel (LOW-VOLTAGE). Treat a panel with broken glass, burnt junction box or cracked backsheet as not safe to handle energised: cover it and stop.
  6. Reconnect in reverse; connectors fully latched; check polarity before closing the disconnect.
- Steps the agent must refuse without confirmed isolation and zero-current reading: opening any MC4 or other connector; pulling a fuse or opening a non-load-break fuse holder; touching combiner terminals or busbars; any measurement on a string with a home-built, USB-tethered or sub-1000 V-rated instrument; shorting a string to measure Isc; using a megger; working on wet modules or standing water; working with visible damage to insulation; any work when the user is alone, impaired by heat, or cannot identify the disconnect.
- Always-refuse list (regardless of confirmation): connecting the home-built meter, the Pi or any USB device to string voltage; back-feeding a string from a supply; bypassing a fuse or ground-fault protection; continuing after any arc, smoke or burning smell (instruct: step back, open the disconnect if safely reachable, do not pull connectors).
- Why series strings are different: a DC arc at hundreds of volts does not self-extinguish at a zero crossing; a string cannot be "switched off" in daylight, only open-circuited; an open-circuited string still has full voltage to ground through any fault.
- Arizona heat: schedule hands-on work near sunrise; thermal and loaded tests need midday sun but are no-contact and can be short; dark glass and metal frames can exceed skin-burn temperatures in summer; hydration and shade breaks; snakes and insects under ground mounts and in combiner boxes.
- Scope/qualification pointer (not legal advice): the owner is an owner-builder; work inside combiners and string-level electrical testing is "qualified person" work under typical US practice. NEC Article 690 topics to read (UNVERIFIED section numbers from memory): 690.7 maximum voltage, 690.8/690.9 circuit sizing and overcurrent, 690.11 DC arc-fault protection, 690.12 rapid shutdown, 690.13/690.15 disconnecting means, 690.31 wiring, 690.33 connectors (same type and brand mated; "do not disconnect under load"), 690.41-690.47 grounding; NFPA 70E's 50 V shock threshold and energized-work rules.

### Gaps
- No safety standard text (NFPA 70E, NEC 690, IEC 62446-1) was fetched; the procedure is assembled from general practice and should be reviewed by a qualified solar electrician before the agent uses it.
- A Fluke article "Targeting Safety in Photovoltaic System Installation and Maintenance" is referenced by Fluke but was not retrieved.

---

## 10. Reporting: the panel test record and warranty evidence

### Takeaway
A manufacturer's RMA desk asks first for label and defect photos and for electrical evidence that isolates the module from the system (module Voc, string V/I at the inverter, a good neighbour for comparison, insulation resistance); an I-V curve with irradiance and temperature strengthens the case but is not what the one guideline found requires.

### Cited Findings
- LG RMA guidance: signed RMA form; high-resolution front and back photos; clear data-label photo; zoomed defect photos; site-surroundings photo; for low/no power: DC voltage and current at the string inverter, module Voc, inverter type, string, isolation resistance at combiner and module, time of measurement; for microinverters seven days of per-module V and I for the suspect and an adjacent good module; external-force breakage excluded; decisions in 2-5 business days; LG may want the module or just the label returned. — [LG RMA request guidelines](https://www.lg.com/us/solar/solar/files/resources/RMA_Request_Guidelines_Final.pdf)
- Hioki's tester app builds reports with photos and memos attached to each string result, an example of the record shape field tools use. — [Hioki FT4310](https://www.hioki.com/sg-en/products/pv/solar-panel/id_6647)
- Healthy performance factor about 90-100 %; fill factor 70-80 % for new c-Si. — [Fluke I-V analysis](https://www.fluke.com/en-us/learn/blog/renewable-energy/analyze-iv-curves); [Solmetric SolarPro article](https://www.solmetric.com/wp-content/uploads/2022/11/SolarPro-I-V-feature-article-Hernday.pdf)

### Inferences
- Proposed record (one JSON document plus attachments, content-hashed so it can serve as evidence):
  - Identity: manufacturer, model, serial, label photo, label ratings, purchase/install date, position (row/column, GNSS), string and inverter.
  - Conditions: timestamp, in-plane irradiance and its source (reference cell, model) and uncertainty, back-of-module temperature, ambient, wind estimate, sky state, tilt/azimuth, cleanliness (photo before/after wipe).
  - Measurements: Voc, Isc, Vmp, Imp, Pmax, fill factor, raw I-V points, instrument model/serial/firmware and calibration note; the same for a neighbouring reference panel within minutes.
  - Corrected values: STC-translated Voc/Isc/Pmax with the method and coefficients used, ratio to label and to the neighbour.
  - System evidence: string V/I/P from the inverter for both strings over at least seven days, shade-test result, insulation resistance if measured.
  - Imagery: visible front/back/junction box/connectors/defect close-ups, thermal image with emissivity, distance, load state and delta-T, EL/UV if taken.
  - Verdict: fault class, confidence, the observations that support it, what was not tested, safety status (in service / removed / hazard-tagged).
- Evidence weight: a suspect-versus-neighbour comparison under the same sky is more persuasive than an STC-corrected absolute number from a DIY tracer; warranties generally guarantee power at STC with a measurement tolerance, so a 30 % (substring) loss is easy to prove and a 5 % loss is not.

### Gaps
- Only one manufacturer's guideline (LG, which has left the module business) was retrieved; the actual panel maker's warranty terms, tolerance and required test method must be read once the label is known.

---

## 11. Existing open-source tools and data sources to borrow

### Takeaway
There is now a usable Rust path (pvlib-rust) for solar position, clear-sky, transposition, temperature and single-diode modelling; IV Swinger 2 is the reference open design for the tracer; cheap BLE multimeter protocols are documented; live irradiance can come free from Open-Meteo.

### Cited Findings
- pvlib-rust 0.1.6, Apache-2.0, 2026-04-20; fewer I/O sources than pvlib-python (TMY3, EPW, PVGIS, SAM) and fewer spectral models; 73 % documentation coverage. — [pvlib-rust on docs.rs](https://docs.rs/crate/pvlib-rust/latest)
- PVWatts V8 API: key, 1,000 requests/hour, hourly `poa`/`tcell`/`dc`. — [PVWatts V8 docs](https://developer.nrel.gov/docs/solar/pvwatts/v8/)
- Open-Meteo satellite radiation API with tilted irradiance and 10-minute GOES-19 updates. — [Open-Meteo](https://open-meteo.com/en/docs/satellite-radiation-api)
- IV Swinger 2: GPL v3 software, TAPR OHL 1.0 hardware. — [IV Swinger 2 PCB build guide](https://content.instructables.com/ORIG/F2T/I6P6/JS0OS2H3/F2TI6P6JS0OS2H3.pdf); [csatt/IV_Swinger](https://github.com/csatt/IV_Swinger)
- BLE multimeter protocol documentation and a GPL-2.0 Rust client. — [ludwich66/Bluetooth-DMM](https://www.github.com/ludwich66/Bluetooth-DMM); [depau/bluetooth-dmm-rust](https://github.com/depau/bluetooth-dmm-rust)
- Solar Assistant MQTT topic scheme. — [Solar Assistant MQTT help](https://solar-assistant.io/help/integration/mqtt)
- NREL's fleet method (basis of RdTools-style analysis) and the regional soiling-station method. — [NREL fy22osti/81314](https://nrel.gov/docs/fy22osti/81314.pdf); [NREL fy17osti/69131](https://www.nrel.gov/docs/fy17osti/69131.pdf)

### Inferences
- Licence hygiene for a public Rust-first repo: pvlib-rust (Apache-2.0) can be a dependency; IV Swinger software (GPL v3) and the Rust BLE DMM client (GPL-2.0) should be treated as references for clean-room re-implementation, or isolated as separate binaries, unless the project accepts GPL; the TAPR OHL hardware licence requires sharing derivative hardware designs.
- pvlib-rust is young (0.1.x, single owner); validate its clear-sky and SPA outputs against pvlib-python once for the site before trusting agent verdicts built on it.

### Gaps
- RdTools, pvanalytics, open PV fault-detection datasets and Home Assistant forecast integrations (Forecast.Solar, Solcast) were not examined in this session.

---

## 12. Deliverable 1: one-page staged procedure, "Is this panel failing?"

### Takeaway
Seven stages; the first four need no electrical contact and will localise most faults; only stage 6 involves opening connectors, on one panel, after confirmed isolation.

### Cited Findings
- Conditions and thresholds used below come from: thermography conditions and delta-T guidance — [IEA PVPS T13-10:2018](https://iea-pvps.org/wp-content/uploads/2020/01/Review_on_IR_and_EL_Imaging_for_PV_Field_Applications_by_Task_13.pdf); I-V interpretation and 700 W/m2 guidance — [Solmetric SolarPro article](https://www.solmetric.com/wp-content/uploads/2022/11/SolarPro-I-V-feature-article-Hernday.pdf); diode fault behaviour — [PV-Tech](https://www.pv-tech.org/wp-content/uploads/legacy-publication-pdfs/75d3699f25-pv-bypass-diode-faults-current-testing-and-scope-for-future-test-development.pdf); inverter limits — [EG4 spec sheet](https://eg4electronics.com/wp-content/uploads/2024/05/EG4-3000EHV-48-Spec-Sheet.pdf); evidence list — [LG RMA guidelines](https://www.lg.com/us/solar/solar/files/resources/RMA_Request_Guidelines_Final.pdf).

### Inferences
**Stage 0. Desk check from telemetry. Hazard NONE. Instruments: Modbus/MQTT, pvlib, Open-Meteo.**
- Filter out curtailed periods. Compare inverter 1 vs inverter 2 PV voltage, current, power; check for 18 A / 5,000 W flat tops; compare a clear day with the clear-sky model; look for time-of-day notches; trend morning Voc against temperature.
- Good: strings within about 3 %, smooth bell curve, PR consistent with temperature. Bad: persistent voltage deficit of about 10-13 V (substring), current deficit, notches, one string at zero. If only "below nameplate" with symmetric strings: explain derating, stop here.

**Stage 1. Walk-round and photographs. Hazard NONE. Instruments: pack camera, GNSS/IMU.**
- Label, front, back, junction box, connectors, defects, surroundings, shadows; record string map, combiner, disconnects, any module-level electronics.
- Bad: broken glass, burn marks, browned cells, melted or mismatched connectors, chewed cable, droppings/dirt dams, vegetation. Clean obvious soiling and re-run stage 0 before going further.

**Stage 2. Thermal scan under load. Hazard NONE. Instruments: MLX90640 at 1 m or less from the back (now), TC001-class later.**
- Conditions: more than 600 W/m2, steady sky, array charging in bulk for 10-15 minutes.
- Good: uniform within about 3 K. Bad: hot cell above 10 K (investigate), above 20 K (priority); a warm one-third strip (substring bypassed); hot junction box or connector (series resistance, fire precursor); whole panel warmer (open-circuited).

**Stage 3. Shade test. Hazard NONE. Instruments: opaque cover, inverter data at 1 Hz, spoken prompts.**
- Cover each suspect panel (and one known-good) for about a minute.
- Good: string voltage/power falls by about 1/N. Bad: less than expected (substring already lost); nothing (panel already bypassed); current collapses (open diode: uncover at once, hazard-tag).

**Stage 4. String comparison at the combiner/disconnect. Hazard HAZARDOUS DC environment; clamp on insulated conductors only. Instruments: CAT III-rated DC clamp (Bluetooth), nothing home-built.**
- Good: parallel strings within about 5 %, matching inverter data. Bad: a string at 0 A (fuse, open connector), low string.
- Optional, rated meter and competent person only: string Voc at the open disconnect; fuse continuity; insulation resistance.

**Stage 5. Decide.** If stages 0-4 point to one panel, continue; if they point to wiring, fuses, connectors or the inverter input, stop and treat that item (connector replacement and combiner work are qualified-person tasks).

**Stage 6. Isolate the one suspect panel. Hazard: HAZARDOUS DC until both of its connectors are open, then LOW-VOLTAGE.**
- Dawn/dusk or covered; second person; open DC disconnect (photo); clamp shows below 0.1 A (agent must see the reading); open the panel's two connectors with the tool; cap the string ends.

**Stage 7. Isolated-panel tests. Hazard LOW-VOLTAGE. Instruments: Bluetooth DMM, EY-class tester or FLIR PV48, later the isolated PV test module with reference cell and temperature probe.**
- Uncover in steady sun above 700 W/m2. Measure Voc, Isc (through a tester only), I-V curve; repeat on a neighbouring good panel within minutes; optional shade-one-substring during a trace to exercise each diode; optional night tests (reverse-current diode check, EL).
- Good: Voc within about 3 % and Isc within about 5-10 % of corrected label values and of the neighbour, fill factor 0.70-0.80, smooth knee. Bad: Voc at two-thirds (shorted diode/substring), steps, low fill factor (series resistance), sloping top (shunts/PID), low Isc after cleaning (degradation, browning).
- Write the panel test record; tag the panel in-service, removed or hazard; reconnect with latched connectors and correct polarity.

### Gaps
- Numeric good/bad bands are rules of thumb assembled from the cited practice notes; they should be tightened once the label, string map and a first set of baseline measurements exist.

---

## 13. Deliverable 2: bill of materials at about USD 30, 100 and 250

### Takeaway
USD 30 buys references (temperature, DIY irradiance cell, shade cover); USD 100 adds a handheld single-panel tester; USD 250 adds a Bluetooth DC clamp and the parts for the isolated tracer. String-rated voltage and insulation instruments sit above this budget (USD 399 and up).

### Cited Findings
- EY1600W tester USD 89. — [ShopSolarKits](https://shopsolarkits.com/products/elejoy-1600w-mppt-solar-panel-tester-multimeter)
- FLIR PV48 USD 189.99. — [Transcat](https://www.transcat.com/flir-pv48-solar-panel-i-v-curve-tracer)
- UT219PV USD 399; UT503PV USD 598; UT505B USD 329.99. — [Transcat UT219PV](https://www.transcat.com/uni-t-ut219pv-solar-pro-ac-dc-clamp-meter); [Transcat UT503PV](https://www.transcat.com/uni-t-ut503pv-solar-pro-pv-insulation-tester-for-solar-photovoltaic-systems); [Transcat UT505B](https://www.transcat.com/uni-t-ut505b-advanced-insulation-resistance-dmm)
- Apogee SP-110-SS USD 280; Fluke IRR2-BT USD 555.33. — [RainWise](https://rainwise.com/sp-110-ss-pvmet-replacement-sensor); [TEquipment](https://www.tequipment.net/Solar-Analzyers/Curve-Tracers/)
- IV Swinger 2 claimed build cost about USD 50; PCBs USD 5-25. — [csatt/IV_Swinger](https://github.com/csatt/IV_Swinger); [build guide](https://content.instructables.com/ORIG/F2T/I6P6/JS0OS2H3/F2TI6P6JS0OS2H3.pdf)

### Inferences
(Line prices without a link above are researcher estimates, UNVERIFIED.)
- **About USD 30 tier (all hazard NONE/LOW-VOLTAGE):** two waterproof DS18B20 probes and foil/Kapton tape (about 6); DIY reference cell: small encapsulated mono cell, precision shunt, clip mount (about 8-10); opaque shade cover (about 5-8); MC4 spanner pair and protective caps (about 6); 365 nm UV torch for night inspection (about 8, optional).
- **About USD 100 tier:** the above plus an EY800W-class tester (about 45-60) or the EY1600W (89) for single-panel Voc/Isc/Pmax, read by camera OCR or voice.
- **About USD 250 tier:** USD 30 tier (30) + EY1600W (89) or parts for the isolated tracer (about 70-110) + Bluetooth DC clamp of the Owon CM2100B class (about 60-70). Alternative at the same money: FLIR PV48 (190) + USD 30 tier + budget toward the clamp.
- **Above budget, for string-level work by a competent person:** UT219PV (399); insulation tester UT503PV (598) or UT505B (330, no data link); Apogee SP-110 (280) if a traceable irradiance reference is wanted.

### Gaps
- Marketplace prices for EY800W, Owon CM2100B, DS18B20, small reference cells and RS-485 pyranometers were not verified on fetched pages.

---

## 14. Deliverable 3: proposed open "PV test module" (single-panel I-V tracer with reference-cell and temperature inputs)

### Takeaway
A battery-powered, wireless, capacitor-sweep tracer for one panel, about USD 70-110 in parts, whose defining feature is refusal logic: it measures the connected voltage through a high-voltage-tolerant divider first and will not close its switch above 60 V, so plugging it into a string by mistake is survived and reported rather than catastrophic.

### Cited Findings
- Reference topology and parts values: relay, 2 x 1000 uF 100 V capacitors, 5 mohm shunt, 47 ohm bleed, MCP3202 ADC, 15 A diodes; GPL v3/TAPR OHL. — [IV Swinger 2 build guide](https://content.instructables.com/ORIG/F2T/I6P6/JS0OS2H3/F2TI6P6JS0OS2H3.pdf)
- Field requirements: sweep under 1 s, irradiance above 700 W/m2, matched in-plane reference, back-of-module temperature. — [Solmetric SolarPro article](https://www.solmetric.com/wp-content/uploads/2022/11/SolarPro-I-V-feature-article-Hernday.pdf)
- Commercial comparison point: FLIR PV48, 800 W/60 V class, USD 189.99, leads CAT III 300 V. — [Transcat](https://www.transcat.com/flir-pv48-solar-panel-i-v-curve-tracer)

### Inferences
- **Block sketch:** MC4 pigtails -> 20 A PV fuse -> reverse-polarity protection -> sense divider (rated for 600 V+ survival, megohm impedance) -> series MOSFET or relay (closed only into a fully bled capacitor) -> capacitor bank 2-10 mF/100 V -> 2-5 mohm Kelvin shunt -> return. Bleed MOSFET + 47 ohm resistor across the bank. Simultaneous-sampling ADC (shared "isolated meter" core) at 20-50 kS/s on V and I; two auxiliary channels: reference-cell shunt voltage and a second spare; 1-Wire bus for two DS18B20s (module back, reference cell). ESP32 (BLE/Wi-Fi/ESP-NOW, LoRa relay via the pack) on a small Li-ion cell. Non-conductive IP65 enclosure, no exposed metal, no external USB while PV leads are attached (charge port covered by the lead plug or interlocked).
- **Sequence:** measure open-circuit input -> checks pass -> sample reference cell and temperatures -> close switch, record I(t), V(t) from Isc to Voc (about 8-40 ms) -> open switch (current is already zero) -> sample reference cell again (reject the sweep if irradiance moved more than 2 %) -> bleed to below 5 V -> transmit raw points, Voc, Isc, Vmp, Imp, Pmax, fill factor, STC-translated values, neighbour ratio -> speak the result.
- **Hard limits enforced in firmware and hardware:**
  1. Refuse to sweep if input is above 60 V DC, below 5 V, or reversed; announce "this is not a single panel".
  2. Input stage must survive 600 V DC indefinitely with the switch open (divider, creepage, switch voltage rating or a series disconnect relay rated for it) so a string mis-connection is safe.
  3. Current limit by fuse (20 A) and by design (15 A panels); abort if Isc sample exceeds 16 A.
  4. One sweep per command; minimum interval to respect capacitor ripple/thermal limits; capacitor voltage must be below 5 V before the next sweep and before "safe to disconnect" is announced.
  5. Galvanic isolation: no conductive path to the Pi; wireless data only.
  6. Never used to interrupt current: the switch only closes into a discharged capacitor and opens at zero current.
  7. Requires the agent's stage-6 isolation confirmation token before arming.
  8. Not for strings, not for panels still connected to anything, not for wet conditions.
- **Rough parts cost (estimates, UNVERIFIED):** capacitors 6-12; MOSFETs/relay and drivers 6; shunt 2; ADC and front end 8-15; ESP32 5; Li-ion cell and charger 8; fuse and holder 6; MC4 pigtails 8; enclosure and glands 10-12; DS18B20 x2 6; reference cell and shunt 8-10; PCB 5-25. Total about USD 80-115.
- **Validation plan:** compare against a FLIR PV48 or a borrowed commercial tracer on two or three panels; check Voc against a calibrated DMM and Isc against a DC clamp; publish the comparison with the design.

### Gaps
- No detailed design-and-theory document for IV Swinger 2 (sweep timing, ADC rate, limits) was retrieved; consult it before finalising values.
- Component choices for 600 V input survival need a proper creepage/clearance and failure-mode review; nothing here is a verified safety design.

---

## 15. Deliverable 4: roadmap placement

### Takeaway
Phase 1 can already deliver a credible "is a panel failing, and which one" answer without new hardware by combining telemetry analytics, guided photography, coarse thermography and the shade test; the single-panel I-V measurement and trustworthy irradiance reference belong to a phase-2 PV test module; continuous per-string sensing, night imaging and rated-instrument integration are phase 3.

### Cited Findings
- Capabilities and limits drawn from sections 2-7 above: telemetry interfaces — [Solar Assistant MQTT](https://solar-assistant.io/help/integration/mqtt), [EG4 spec sheet](https://eg4electronics.com/wp-content/uploads/2024/05/EG4-3000EHV-48-Spec-Sheet.pdf); modelling — [pvlib-rust](https://docs.rs/crate/pvlib-rust/latest), [Open-Meteo](https://open-meteo.com/en/docs/satellite-radiation-api); thermography resolution guidance — [IEA PVPS T13-10:2018](https://iea-pvps.org/wp-content/uploads/2020/01/Review_on_IR_and_EL_Imaging_for_PV_Field_Applications_by_Task_13.pdf); tracer reference design — [csatt/IV_Swinger](https://github.com/csatt/IV_Swinger).

### Inferences
- **Phase 1 (planned modules only):** MQTT/Modbus ingestion with curtailment filter; A/B string ratio monitor and morning-Voc trend; clear-sky and satellite-irradiance PR with pvlib-rust; shading-notch and clipping detectors; guided photo capture with label OCR and array map (GNSS/IMU for tilt and azimuth); MLX90640 back-side thermal scan at 1 m or less for substrings, junction boxes, connectors; shade test with spoken prompts; back-of-module thermocouple; light sensor as a steadiness indicator; Bluetooth DMM for isolated single-panel Voc and diode test; panel test record schema; the safety interlock dialogue (refusal list). Explicitly excluded: home-built meter on any PV conductor.
- **Phase 2 (purpose-built PV test module and better optics):** isolated capacitor-sweep tracer with reference cell and temperature inputs; STC translation and neighbour-ratio verdicts; TC001-class thermography with per-cell delta-T; Bluetooth DC clamp at the combiner with time alignment to inverter data; reference-cell scatter pod on the array plane (LoRa); EY/PV48 reading by OCR as a cross-check; night reverse-current diode test with a current-limited supply.
- **Phase 3:** permanent per-string current pods (clamp-on Hall sensors on insulated conductors) and soiling-ratio pair; degradation trending over seasons; night EL with a modified camera and current-limited supply; integration of rated string-level tools if their BLE protocols can be read (UT219PV, UT503PV); warranty-pack generator; optional thermal-drone survey (purchase out of scope).

### Gaps
- Phase boundaries assume the planned module list in the request; the project's own roadmap document was not consulted.

---

## 16. Deliverable 5: outside-the-box ideas for agent-led PV diagnosis

### Takeaway
Most of the best ideas exploit things this particular system already does (two inverters as mutual references, daily curtailment, MPPT dither) or the pack's unusual sensors (SDR, scatter pods, voice), so that diagnosis happens with nobody touching a conductor.

### Cited Findings
- Building blocks referenced: two-cell soiling-ratio method — [NREL fy17osti/69131](https://www.nrel.gov/docs/fy17osti/69131.pdf); selective shading and diode behaviour — [Solmetric](https://www.solmetric.com/wp-content/uploads/2022/11/SolarPro-I-V-feature-article-Hernday.pdf), [PV-Tech](https://www.pv-tech.org/wp-content/uploads/legacy-publication-pdfs/75d3699f25-pv-bypass-diode-faults-current-testing-and-scope-for-future-test-development.pdf); pulsed bypass-route test concept — [Hioki FT4310](https://www.hioki.com/sg-en/products/pv/solar-panel/id_6647); load state and thermography — [IEA PVPS T13-10:2018](https://iea-pvps.org/wp-content/uploads/2020/01/Review_on_IR_and_EL_Imaging_for_PV_Field_Applications_by_Task_13.pdf).

### Inferences
1. **Twin-string ratiometry with change-point alarms.** Treat each inverter's string as the other's reference cell; run a CUSUM on V, I and P ratios. Add a "swap test": move the suspect panel to the other string and see whether the deficit follows it.
2. **Curtailment as a free daily Voc test.** Every time the bank is full the array floats near Voc; log it against modelled temperature and catch a one-third-module step months before anyone notices lost energy.
3. **Agent-controlled load for thermography.** Thermal patterns need current flowing; the agent switches on a large load (or delays charging to midday) so the array is at MPP for the 15 minutes before and during the scan, then confirms from telemetry that it was.
4. **Shadow wand with voice countdown.** A paddle on a pole shades one cell per substring while the agent watches 1 Hz string voltage and calls out "next"; result is a substring-resolution health map with zero electrical contact.
5. **Natural I-V sweeps.** Cloud edges, dawn ramps and the MPPT's own dither move the operating point; fit a single-diode model to (V, I, irradiance) points gathered over days to estimate string series resistance and its drift (connector degradation) without a tracer.
6. **Arc and bad-connector sniffing with the SDR.** Series arcs and loose DC connections radiate broadband noise; walk the array with the SDR logging the HF noise floor tagged by GNSS, and compare loaded versus curtailed states.
7. **Thermal time-lapse pod.** Park a scatter pod with the MLX90640 behind a suspect panel for a full day; hot spots that track irradiance, appear only when shaded, or persist when curtailed discriminate between cell defects, diode activity and resistive joints.
8. **Soiling-ratio pair on the array.** Two tiny reference cells, one wiped weekly, give a measured soiling loss and tell the agent when "clean the array" beats "test the panel".
9. **Night reverse-pulse diode check.** A current-limited low-voltage pulse through an isolated panel's bypass path at night verifies all three diodes conduct, an open-diode screen that daylight Voc tests cannot provide.
10. **Drone RGB shadow audit.** Use the existing camera drone for an orthomosaic and a time-of-day shadow simulation of posts, vegetation and row-to-row shading, then match predicted notches against the telemetry notches.
11. **Interlock by telemetry.** The agent only "unlocks" hands-on instructions after it has itself received a sub-0.1 A clamp reading and a disconnect-off photo, and it stores those as part of the test record.
12. **OCR bridge for dumb instruments.** The aimed camera reads the LCD of a cheap PV tester or rated clamp so that certified but closed instruments become agent-readable without any electrical or protocol coupling.

### Gaps
- None of these ideas was found documented as a tested method in the sources retrieved (other than the building blocks cited); each needs a small validation experiment. Idea 6 in particular is speculative: no source on SDR detection of PV connector arcing was searched for.
