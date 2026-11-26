# Red-team catalog 04: how hardware safety interlocks and their firmware fail or get bypassed

Lane: interlock hardware and firmware. Written 2026-11-26 for the Sawbones red-team catalog. Defensive only: every entry describes a failure class to test against our own bench, not a procedure against anyone's product.

## Overview

Read for this pass: `docs/bench-core-cross-reference.md`, the `sawbones-interlock` crate (src and tests), and the `sawbones-isolation` crate header. `docs/THREAT-MODEL.md` and `crates/sawbones-approvals` do not exist in the repo yet. Entries about approvals (replay, rollback) therefore test a design that is not written; they are marked `gap` by default.

What the current design already does well (checked in code or in the parts doc): outputs off at power-on and after `Event::Restart` until arm is seen off then on; e-stop and lost heartbeat latch a fault on that tick; a fault clears only with e-stop released, heartbeat healthy and arm off; lease needs a fresh button press, key, and expires or is spent; a backwards tick drops the lease; class 4 always refused; an exhaustive and random invariant test exists; the heartbeat drives the relay through a diode pump and a transistor, so a pin stuck high or low drops it; the trust root has its own 5 V; the isolation crate can flag a bonded path around a declared barrier.

What the research says is the common thread (see sources): safety layers fail through (a) an undetected first fault that sits silently until a second one arrives, (b) one signal or supply that several layers quietly share, (c) a "healthy" signal that is produced by something that cannot tell if the system is healthy, (d) a glitch at the moment a check runs, and (e) wiring that fails to the live state instead of the safe one. The RP2350 hacking challenge is a useful reminder that even a chip sold with glitch detectors lost to supply-rail glitching, a single skipped instruction, and a stale pointer that made the signature check hash the wrong data. Our trust root is that chip, so its firmware must not depend on its own secure-boot story as a safety layer.

Status key: `handled` (design or code covers it, a test should lock it in), `partial` (covered in intent or in part), `gap` (nothing covers it today). Test types: `software` (deterministic test against our crates), `ngspice` (simulation), `bench` (hardware acceptance test on the real bench).

Top gaps, in order: hw-008 (fault latch lives in RAM), hw-001/hw-025 (no welded-contact or stuck-output feedback yet), hw-009 (who may send ClearFault), hw-011 (heartbeat is a boolean the supervisor can fake), hw-016/hw-017 (approvals and update rollback not written).

## Catalog

```toml
id = "hw-001"
title = "Welded relay contacts leave the rail live after the cut"
category = "relay failure"
target = "rail switch (second independent cut, 2-channel relay module)"
how = "A relay that switched an inrush or inductive load can weld its contacts closed. The coil then drops but the rail stays live, and the interlock believes the cut worked. Relays with forcibly guided (mechanically linked) contacts and a monitoring circuit exist because a plain relay cannot reveal a weld by itself."
test = "bench: with the lamp load, de-energise the relay and read the post-contact sensing optocoupler; a hand-welded (jumpered) contact must raise a latched fault within one tick. software: add a feedback input to Inputs and a test that rail-commanded-off with feedback-live latches a fault."
expected = "Commanded off with output still sensed live latches a fault, drops the other cut, and refuses ClearFault until the sense reads dead."
severity = "high"
status = "gap"
sources = ["https://www.digikey.com/en/articles/elevating-functional-safety-with-relay-modules", "https://industrialmonitordirect.com/blogs/knowledgebase/safety-relay-welded-contact-detection-explained"]
```

```toml
id = "hw-002"
title = "MOSFET fails short, so the electronic cut is no cut"
category = "switch failure"
target = "rail switch (AOD4184 module)"
how = "A MOSFET that is overstressed by a load fault or a turn-off spike commonly fails shorted drain to source. Then no firmware state can turn the rail off, and only the relay in series saves the day. If the relay is also driven from the same enable, one cause takes out both cuts."
test = "bench: short drain-source on the module with the rail off; arm sequence must refuse (output feedback live while commanded off) and the relay path alone must open the rail. software: same as hw-001 test with the feedback taken before the relay."
expected = "A shorted MOSFET is detected before the next arm, and the relay still opens the rail on a lost heartbeat."
severity = "high"
status = "partial"
sources = ["https://forum.digikey.com/t/guide-to-selecting-and-controlling-a-mosfet-for-3-3-vdc-logic-applications/42606", "https://ez.analog.com/ez-blogs/b/engineerzone-spotlight/posts/an-introduction-to-machine-safety-standard-iso-13849"]
```

```toml
id = "hw-003"
title = "3.3 V gate drive only partly turns the MOSFET on"
category = "switch failure"
target = "rail switch (MOSFET gate drive)"
how = "Datasheet on-resistance is quoted at a high gate voltage. At 3.3 V a MOSFET can be only partly enhanced, so its resistance is several times higher and it dissipates heat at rail current, drifting toward failure or an unreliable on-state across temperature and parts. The parts doc says the AOD4184 turns on from 3.3 V; that is a statement about turning on, not about its on-resistance at 2 A."
test = "bench: at the 2 A fuse limit and 3.3 V drive, measure drain-source voltage and case temperature over 30 minutes, cold and warm; compare with the worst-case datasheet curve. ngspice: gate-threshold sweep with min and max Vgs(th)."
expected = "Vds at rated rail current stays under the budgeted value with margin at worst-case threshold; otherwise add a gate driver or a different part."
severity = "medium"
status = "partial"
sources = ["https://forum.digikey.com/t/guide-to-selecting-and-controlling-a-mosfet-for-3-3-vdc-logic-applications/42606"]
```

```toml
id = "hw-004"
title = "Floating gate or enable line while the controller is in reset or high-impedance"
category = "power sequencing"
target = "rail switch and relay driver, enable lines"
how = "During power-up, reset, programming and after a crash a microcontroller pin is high-impedance. Without a pulldown at the gate side of the switch and at the relay stage the line can float high and turn the output on. The pulldown must sit on the pin side of the series resistor, otherwise it forms a divider and weakens the gate drive."
test = "bench: hold the RP2350 in reset (BOOTSEL and RUN low), power the rail input, and read the rail output with a meter; repeat while the debug probe reflashes. ngspice: gate node with the pin modelled as open circuit."
expected = "Rail output is dead in every controller state that is not actively driving the pin."
severity = "high"
status = "partial"
sources = ["https://forum.digikey.com/t/guide-to-selecting-and-controlling-a-mosfet-for-3-3-vdc-logic-applications/42606"]
```

```toml
id = "hw-005"
title = "Low-side (negative) switching leaves the load live to ground"
category = "rail topology"
target = "rail switch, isolation map"
how = "A switch in the negative leg still leaves the load's positive wire at rail voltage. Any second ground path (a probe, a shared meter, the device under test's own ground) can then complete the circuit around the cut. The parts doc already moves the cut to the positive side before the rail feeds anything with a shared ground."
test = "software: add a rail-switch node to the isolation map and a rule that a cut in the negative leg with a bonded return elsewhere is a finding. bench: with the cut open, connect a ground clip to the load return and check for current."
expected = "No supply path to the load with the positive-side cut open, and the checker flags a negative-side cut that has a second return."
severity = "high"
status = "partial"
sources = ["https://forum.digikey.com/t/guide-to-selecting-and-controlling-a-mosfet-for-3-3-vdc-logic-applications/42606"]
```

```toml
id = "hw-006"
title = "Brownout makes outputs and logic undefined, not off"
category = "power glitch"
target = "trust root supply, reset, interlock state machine"
how = "When a supply sags without reaching zero, supervisor and logic outputs can glitch or sit at undefined levels, and low-voltage processors have little logic-level margin. A sag that does not trip the reset can leave the RP2350 running with corrupted reads. A sag that resets it must also leave every output dead while it comes back."
test = "bench: slew the trust root 5 V from 5.0 down to 2.5 V and back at several rates with the lamp on rail A; log the rail output and the heartbeat pin on the logic analyzer. ngspice: model the diode pump and gate through the ramp."
expected = "Rail A and B are off at every point of every ramp and re-armed only by the arm sequence; no ramp produces a pulse on the output."
severity = "high"
status = "partial"
sources = ["https://www.analog.com/en/resources/technical-articles/how-glitchfree-supervisors-aid-in-highreliability-applications.html", "https://www.eevblog.com/forum/microcontrollers/securing-a-critical-logic-level-against-failure/"]
```

```toml
id = "hw-007"
title = "Power sequencing: enable path alive before the trust root, or USB back-feed"
category = "power sequencing"
target = "trust root power, hub port switching, rail enable"
how = "The RP2350-Zero 5 V pin is the USB supply with no diode, so a board with its own supply that also takes USB power can back-feed or be fed from two sources. The parts doc cuts the USB power wire. The rail's input switches and the controller come up at different times, so the order of events at power-on (adapter first, Pi first, controller first) must each leave the rail dead."
test = "bench: run all six power-up orders and one hot-plug of the trust root USB lead, measuring the rail output and the 5 V pin with a meter and logic analyzer. software: restart from every State with every input combination and check outputs are off (the crate's invariants test already covers this)."
expected = "Rail dead in all orderings; the trust root never powered from USB; USB cable power wire cut, verified with a meter."
severity = "high"
status = "partial"
sources = ["https://www.analog.com/en/resources/technical-articles/how-glitchfree-supervisors-aid-in-highreliability-applications.html"]
```

```toml
id = "hw-008"
title = "Latched fault lives in RAM and a reboot erases it"
category = "interlock state machine"
target = "interlock state machine (sawbones-interlock State)"
how = "State::default and POWER_ON are AwaitArmOff. The crate says a restart does not clear a latched fault, but only if the caller keeps the State value across the restart. A brownout or watchdog reset on real hardware loses RAM, so a fault caused by an e-stop or a lost heartbeat becomes AwaitArmOff after the reboot, and arm off then on re-arms with no ClearFault. The firmware must store the latch in retained memory or derive it again from hardware."
test = "software: model a restart as State::POWER_ON in a test and show that a fault then silently disappears (this documents the gap); add a persisted-latch API and a test that reboot from Faulted restarts in Faulted. bench: press e-stop, pull the trust root 5 V, restore it, and cycle the arm switch."
expected = "After any reset a latched fault is still latched and needs ClearFault with e-stop released, heartbeat healthy and arm off."
severity = "high"
status = "gap"
sources = ["https://www.analog.com/en/resources/technical-articles/how-glitchfree-supervisors-aid-in-highreliability-applications.html", "https://interrupt.memfault.com/blog/firmware-watchdog-best-practices"]
```

```toml
id = "hw-009"
title = "ClearFault accepted from the supervisor, not from a person"
category = "interlock state machine"
target = "interlock state machine, trusted panel"
how = "Event::ClearFault is documented as a person's action, but nothing in the crate says where the event comes from. If the Pi (which the agent can reach) can send it, a compromised or confused supervisor can clear a fault the moment the preconditions happen to hold, such as an instant of healthy heartbeat with the arm switch off. The same question applies to RequestArm and LeaseUsed."
test = "software: add a source tag to Event (Panel or Supervisor) and a test that ClearFault from the supervisor is refused in every state. bench: send the clear command over USB from the Pi and confirm the panel button is the only way."
expected = "Only a physical panel action clears a fault; a supervisor message gets a refusal and a log line."
severity = "high"
status = "gap"
sources = ["https://industrialmonitordirect.com/blogs/knowledgebase/emergency-stop-switch-contact-configuration-ncno-rationale"]
```

```toml
id = "hw-010"
title = "A timer or hardware PWM keeps the heartbeat pin toggling after the firmware hangs"
category = "watchdog defeat"
target = "watchdog circuit (charge pump to relay)"
how = "A charge pump turns the relay on only while the pin toggles, which defeats stuck-high and stuck-low faults but not a pin that is toggled by a timer interrupt, DMA, PIO or PWM peripheral while the main logic is dead. The watchdog literature calls this feeding unconditionally. The toggle must be produced by the interlock loop itself, once per good pass, and never by a peripheral that runs on its own."
test = "software: in firmware, a test build that hangs the main loop but leaves interrupts running must drop the relay; host-side, assert the toggle function is called only from the step loop (grep-style lint). bench: halt the core with the debug probe while PWM runs and watch the relay."
expected = "A stopped or looping interlock drops the relay within the watchdog window; peripherals cannot generate the heartbeat."
severity = "high"
status = "partial"
sources = ["https://www.EmbeddedRelated.com/showarticle/1276.php", "https://www.eevblog.com/forum/microcontrollers/securing-a-critical-logic-level-against-failure/", "https://interrupt.memfault.com/blog/firmware-watchdog-best-practices"]
```

```toml
id = "hw-011"
title = "heartbeat_healthy is a boolean the supervisor or a low-priority task can fake"
category = "watchdog defeat"
target = "interlock state machine input, supervisor link"
how = "The crate takes heartbeat_healthy as a boolean decided outside it. If the supervisor heartbeat is a plain repeated message, a stuck process or a replay of old frames keeps it healthy while the agent is doing nothing sensible. A single low-priority task that feeds unconditionally can mask a deadlocked task, and a heartbeat that never checks any content tells you only that something is sending."
test = "software: heartbeat module with a challenge-response (trust root sends a nonce, supervisor returns it signed or hashed with a counter); tests for replayed, late, duplicated and out-of-order frames. bench: freeze the supervisor with SIGSTOP and replay a captured heartbeat."
expected = "Heartbeat is healthy only for a fresh response to the latest challenge within the window, and the window is derived from a stated stopping-time budget."
severity = "high"
status = "gap"
sources = ["https://interrupt.memfault.com/blog/firmware-watchdog-best-practices", "https://www.EmbeddedRelated.com/showarticle/1276.php", "https://cwe.mitre.org/data/definitions/294.html"]
```

```toml
id = "hw-012"
title = "Charge-pump heartbeat stage fails in a way that holds the relay"
category = "watchdog defeat"
target = "watchdog circuit"
how = "A discrete pump has its own failure modes: a leaky or shorted capacitor, a shorted diode, a transistor that fails short, a toggle rate that is too fast (the pump saturates at any frequency above some value), or relay coil flyback and supply noise that look like pulses. A stuck-toggling pin faster than the intended range should not be accepted as healthy. The thread notes the pump is slow and that software, not a timer, should do the toggling."
test = "ngspice: model the pump over a sweep of toggle frequency (including far above nominal), duty, supply voltage 4.5 to 5.5 V, and each single component fault (open, short, drift of 50 percent); record relay-on time after the toggle stops. bench: repeat on the built stage with a signal generator."
expected = "Relay drops within the budgeted time after toggling stops, and each single component fault either drops the relay or is caught by a self-test at arm time."
severity = "medium"
status = "partial"
sources = ["https://www.eevblog.com/forum/microcontrollers/securing-a-critical-logic-level-against-failure/", "https://industrialmonitordirect.com/blogs/knowledgebase/safety-relay-welded-contact-detection-explained"]
```

```toml
id = "hw-013"
title = "Incomplete reset tree: the CPU resets, the output stages and peripherals do not"
category = "watchdog defeat"
target = "trust root, module boards, rail enable"
how = "When the watchdog fires and only the CPU resets, peripherals and external latches stay in their faulted state, and the task gets stuck again when it talks to them. For us this means a latched driver, a pump capacitor still charged, or a module that keeps its output after the controller restarts."
test = "bench: induce a watchdog reset (infinite loop test build) with rail A and the lamp on and check that every output stage, LED driver and I2C device is dead or reinitialised, and that the relay drops without waiting on the CPU."
expected = "After a watchdog or brownout reset the whole output path, not just the CPU, is in its safe state before firmware runs."
severity = "medium"
status = "partial"
sources = ["https://www.EmbeddedRelated.com/showarticle/1276.php"]
```

```toml
id = "hw-014"
title = "Time of check to time of use between lease, approval and the physical act"
category = "timing race"
target = "interlock state machine, supervisor-to-trust-root path"
how = "A lease that is valid when the supervisor checks it can end, or the key or arm switch can go off, before the action reaches the rail. The crate drops a lease on key-off, arm-off, fault and TTL, but the supervisor acts on its own earlier view. The leases are also not yet bound to a target or parameters, so a lease granted for one action can be spent on another. Standard TOCTOU guidance: shorten the gap, recheck after the use, and lock before the check."
test = "software: step-level test with a fault on the same tick as LeaseUsed (LeaseUsed must not report success); a test that the trust root, not the supervisor, enforces the action at use time. When leases are bound to a target, test spending on a different target. bench: remove the key between lease and action."
expected = "The trust root decides at the moment of use and returns the result; the supervisor never acts on a cached permit."
severity = "high"
status = "partial"
sources = ["https://cwe.mitre.org/data/definitions/367.html"]
```

```toml
id = "hw-015"
title = "A frozen or manipulated tick keeps leases and requests alive"
category = "timing race"
target = "interlock state machine, tick source"
how = "The crate drops a lease if the tick goes backwards but a tick that stops moving makes every TTL infinite, and a tick that jumps forward expires leases early. The tick must come from a hardware timer that the control loop cannot pause, and the loop must fault if the tick does not advance."
test = "software: a test with a constant now across many steps, with a lease live (expected: fault or expiry by a separate stall counter), plus wraparound near u64 max and very large jumps. bench: pause the core on the debug probe and see that the relay path still drops."
expected = "A stalled tick is detected as a fault; TTLs use a free-running timer."
severity = "medium"
status = "gap"
sources = ["https://interrupt.memfault.com/blog/firmware-watchdog-best-practices"]
```

```toml
id = "hw-016"
title = "Replay of a signed approval or a captured panel exchange"
category = "approvals"
target = "approvals (signed approvals with replay counters)"
how = "Without a per-message counter or nonce a captured valid approval can be sent again. Sequence numbers must be inside what is signed so they cannot be changed along with the content. Counter state must survive a restart, or an old approval becomes fresh after a reboot; a counter reset to zero on reboot is a replay window."
test = "software: when sawbones-approvals exists, property tests for duplicate, reordered, stale and counter-wrapped approvals, and for a restart with the counter store wiped or rolled back. Until then record this as a requirement. bench: replay a captured approval after a trust root reboot."
expected = "Every approval is accepted at most once, including across reboots, and the counter store fails closed if it cannot be read."
severity = "high"
status = "gap"
sources = ["https://cwe.mitre.org/data/definitions/294.html"]
```

```toml
id = "hw-017"
title = "Firmware rollback to an older signed image"
category = "firmware"
target = "firmware update path, trust root"
how = "An old signed image with a known fault is still validly signed, so accepting it defeats the fix. RP2350 boot ROM has a version check that blocks older signed software, but it only protects us if the OTP configuration is programmed. The roadmap says firmware cannot change without the jumper; both the jumper and the version rule need tests."
test = "bench: attempt to flash an older signed build with the jumper fitted; it must be refused or must not arm. software: version-compare function with tests for equal, lower, and wrapped versions."
expected = "Older images are refused; the trust root reports its image version on the panel."
severity = "medium"
status = "gap"
sources = ["https://pip-assets.raspberrypi.com/categories/1260-security/documents/RP-009377-WP-1-Understanding%20RP2350_s%20security%20features.pdf", "https://cwe.mitre.org/data/definitions/294.html"]
```

```toml
id = "hw-018"
title = "Firmware jumper left in, bridged, or read as present by a floating pin"
category = "firmware"
target = "firmware jumper, interlock state machine"
how = "The jumper is the one physical gate to reprogramming. A jumper left fitted turns that gate into nothing; a floating or noisy input can read as fitted. The interlock crate lists the jumper as not written yet. The jumper state also changes what the chip allows: RP2350 USB boot interfaces can be disabled in OTP, which also blocks USB updates, so the update path and the mitigation compete."
test = "software: Inputs gets a jumper_fitted bool; a test that any fitted jumper means no rail on and a fault if it appears while armed. bench: fit and remove the jumper with the lamp armed; read the pin with a floating input."
expected = "Jumper fitted means rails are off and the panel says so; the input has a defined pull and is debounced."
severity = "medium"
status = "gap"
sources = ["https://raspberrypi.com/news/security-through-transparency-rp2350-hacking-challenge-results-are-in", "https://pip-assets.raspberrypi.com/categories/1260-security/documents/RP-009377-WP-1-Understanding%20RP2350_s%20security%20features.pdf"]
```

```toml
id = "hw-019"
title = "Supply glitching of the trust root's secure state"
category = "voltage glitching"
target = "trust root microcontroller (RP2350)"
how = "The challenge found that cutting the separately powered OTP supply for tens of microseconds right after a guard-word read re-enabled cores that should be disabled (E16), and that a voltage glitch could skip an instruction in the USB reboot path (E20). Cheap voltage glitch rigs were shown to be enough. Our threat model has the agent, not a person with a soldering iron, as the adversary, so this is mainly about not leaning on chip secure boot as a safety layer."
test = "bench: not run as an attack. Review step: the safety function must be correct with secure boot assumed broken, using the physical layers (relay, fuse, charge pump) as the backstop. Enable the documented mitigations where they do not cost us the update path, and record which are open."
expected = "A fully compromised trust root firmware still cannot hold rail A on without the heartbeat toggle, and cannot exceed the fuse limit."
severity = "medium"
status = "partial"
sources = ["https://raspberrypi.com/news/security-through-transparency-rp2350-hacking-challenge-results-are-in", "https://www.hackster.io/news/raspberry-pi-announces-the-winners-of-its-rp2350-capture-the-flag-contest-confirms-vulnerabilities-fcaa05053022", "https://www.keysight.com/blogs/en/tech/nwvs/2025/03/26/security-highlight-rp2350-hardware-attacks"]
```

```toml
id = "hw-020"
title = "Glitch detectors off by default and weak at slow clocks; debug left enabled"
category = "voltage glitching"
target = "trust root OTP configuration, debug pads"
how = "The RP2350 glitch detectors are disabled by default and less effective at slow clocks such as boot ROM, and the challenge showed many glitches go undetected even at the highest sensitivity. Debug is enabled by default, and the parts doc has three debug pads under the board. Debug access lets an attacker or a mistake halt the core while the relay path is held."
test = "bench: acceptance checklist item that OTP glitch-detector bits, boot flags and debug disable are read back and recorded before the bench is declared safe, or a signed note that they are open on purpose; halt the core over SWD with the relay held and confirm the heartbeat stage drops it."
expected = "Documented OTP state; halting the core drops the relay."
severity = "low"
status = "partial"
sources = ["https://pip-assets.raspberrypi.com/categories/1260-security/documents/RP-009377-WP-1-Understanding%20RP2350_s%20security%20features.pdf", "https://www.keysight.com/blogs/en/tech/nwvs/2025/03/26/security-highlight-rp2350-hardware-attacks"]
```

```toml
id = "hw-021"
title = "One skipped instruction or one flipped bool turns the interlock on"
category = "firmware fault"
target = "interlock state machine firmware build"
how = "The challenge's boot ROM fault was one skipped instruction. For us the matching risk is a single corrupted bool, an enum with an out-of-range value, or an optimiser that merges checks, ending in an output on. Safety firmware commonly keeps outputs in a form where a single bit error reads as off, and checks them twice."
test = "software: proptest that the output drive function maps every non-valid bit pattern of State and Outputs to off (transmute test in a test-only unsafe module); compile with the release profile and test again. bench: none."
expected = "No single-bit or single-skip fault in the rail-enable decision yields rail on; the enable needs two independent conditions in the output path."
severity = "medium"
status = "gap"
sources = ["https://www.keysight.com/blogs/en/tech/nwvs/2025/03/26/security-highlight-rp2350-hardware-attacks", "https://pip-assets.raspberrypi.com/categories/1260-security/documents/RP-009377-WP-1-Understanding%20RP2350_s%20security%20features.pdf"]
```

```toml
id = "hw-022"
title = "Module enumerates as a keyboard or other unexpected USB class"
category = "USB impersonation"
target = "module identity, brain USB stack"
how = "A USB isolator passes the data lines, so it keeps ground and power apart but does nothing about what a device claims to be. A module with a bug or modified firmware can present as a keyboard and type into the Pi, which operating systems trust by default. Allowlisting by vendor ID is a stated mitigation but IDs can be copied."
test = "software: supervisor-side enumeration test that accepts only an allowlist keyed by hub port plus a descriptor set, and rejects HID and mass-storage classes on module ports (udev rules tested in a VM with a USB gadget). bench: flash a spare RP2350 as a HID device on a module port."
expected = "A device with an unexpected class is not bound and is logged; no input events reach the Pi."
severity = "medium"
status = "gap"
sources = ["https://netwrix.com/en/resources/blog/badusb-attack-prevention", "https://www.analog.com/en/products/ADuM3160.html"]
```

```toml
id = "hw-023"
title = "Module claims another module's identity or capabilities"
category = "USB impersonation"
target = "module identity, module protocol"
how = "If a module is trusted because it says it is the meter, a replaced or confused module can feed readings the supervisor treats as ground truth. The ID has to be tied to something the module cannot copy by saying it: a per-device key, the hub port, and the reference check at start-up."
test = "software: module-protocol tests for duplicate IDs, an ID that changes port, and an unsigned capability change; bench: swap two modules between ports and run the reference check."
expected = "A module that is not where or what it was enrolled as is quarantined until a person re-enrolls it."
severity = "medium"
status = "gap"
sources = ["https://netwrix.com/en/resources/blog/badusb-attack-prevention"]
```

```toml
id = "hw-024"
title = "Flatlined or spoofed sensor reads dead while the rail is live"
category = "sensor failure"
target = "measuring modules (INA228), interlock inputs"
how = "A sensor that is stuck at a plausible value, returns the last value after a bus failure, or reads zero on a shorted shunt can make a live rail look dead and a hazard look absent. Plausibility checks (changing within noise, correlated with the commanded state, agreeing with a second source) are the usual detection. The voltage reference module and the meter cross-check are our second source."
test = "software: reading-validator tests for constant value over N samples, no-change after command step, out-of-range, and timestamp gaps. bench: disconnect the shunt and the I2C line under load and see the validator fault; compare against the reference at start-up."
expected = "Unchanging or implausible readings raise a fault and the interlock does not use them to declare the rail dead."
severity = "medium"
status = "partial"
sources = ["https://ez.analog.com/ez-blogs/b/engineerzone-spotlight/posts/an-introduction-to-machine-safety-standard-iso-13849"]
```

```toml
id = "hw-025"
title = "Output feedback fails blind or is only checked once"
category = "sensor failure"
target = "output feedback (optocoupler on the rail), arm sequence"
how = "Feedback that reads live-when-on and dead-when-off can fail to a constant dead reading (a degraded optocoupler, an open sense resistor), which would hide a welded relay or a shorted MOSFET. A test pulse or a check at every arm confirms the feedback can read live before it is trusted to read dead. The ISO category 3 idea is read-back of the output device status plus a second channel."
test = "software: feedback state machine with a self-test step at arm (rail on must read live within a window, else fault). bench: open the sense resistor and confirm arming is refused."
expected = "Arming is refused unless the feedback has been seen both live and dead in this power cycle."
severity = "high"
status = "gap"
sources = ["https://ez.analog.com/ez-blogs/b/engineerzone-spotlight/posts/an-introduction-to-machine-safety-standard-iso-13849", "https://industrialmonitordirect.com/blogs/knowledgebase/safety-relay-welded-contact-detection-explained"]
```

```toml
id = "hw-026"
title = "Ground loop through a probe, analyzer, shield or shared meter defeats the isolator"
category = "isolation"
target = "isolation map, USB isolators, debug probe, logic analyzer"
how = "An isolator breaks the ground path only if nothing else bridges it. The debug probe and logic analyzer share the brain's ground, and a USB cable shield, a scope clip or a bench meter can add a path. The isolator is also rated, not infinite: the ADuM3160 is a 2.5 kV part, and its own evaluation board is not recommended for safety-related testing."
test = "software: the sawbones-isolation checker already finds a bonded path around a declared barrier; add test maps for probe plus module, shield bond, and a shared meter. bench: continuity and leakage between the isolated module ground and brain ground with the whole bench connected, repeated after every re-wire."
expected = "Every real configuration the bench is used in is modelled and checks clean; the bench has a measured continuity step in its acceptance list."
severity = "medium"
status = "handled"
sources = ["https://www.analog.com/en/products/ADuM3160.html"]
```

```toml
id = "hw-027"
title = "E-stop wiring faults: normally-open by mistake, bypass, cross-fault, single channel"
category = "e-stop wiring"
target = "e-stop and arm chain in the rail feed"
how = "A normally-closed series contact fails safe on a wire break or lost power, a normally-open one does not. A bypassed single channel is detected only if the second channel is monitored, and a vibrated-open contact can look normal if the backup is slow. The two-contact part gives us one contact in the feed and one to the controller, which is a 2-channel structure only if the controller treats a disagreement as a fault."
test = "software: Inputs gets estop_feed_open and estop_sense; a test that disagreement for more than a few ticks latches a fault. bench: acceptance test with each wire cut, each contact jumpered, and each of the two contacts bypassed while the lamp is armed; confirm the polarity of the sense input (cut wire reads as pressed)."
expected = "Any single wiring fault opens the feed or latches a fault; reset is blocked while the channels disagree."
severity = "high"
status = "partial"
sources = ["https://industrialmonitordirect.com/blogs/knowledgebase/emergency-stop-switch-contact-configuration-ncno-rationale"]
```

```toml
id = "hw-028"
title = "Arm and key switch inputs: bounce, stuck-on, contact wear at DC"
category = "input fault"
target = "interlock inputs (arm switch, key switch, panel button)"
how = "The arm rule needs the switch seen off then on; a stuck-closed input at power-on is correctly held in AwaitArmOff, but contact bounce or a noisy input can fake an off-then-on edge and re-arm. DC contacts also wear and weld at 12 V with inductive loads, and the contact ratings are still unverified in the parts doc."
test = "software: tests for debounce windows around the arm edge (a one-tick off glitch must not count as a deliberate cycle if the policy needs a minimum off time). bench: contact-rating check at 12 V and 2 A with the lamp and with an inductive load, and count cycles."
expected = "A minimum off time is enforced before arm; ratings are verified from the part in hand."
severity = "low"
status = "partial"
sources = ["https://industrialmonitordirect.com/blogs/knowledgebase/emergency-stop-switch-contact-configuration-ncno-rationale"]
```

```toml
id = "hw-029"
title = "Converter fails to full input voltage or the soft limit does not trip"
category = "rail failure"
target = "armed rail (XL4015 current limit, 15 V adapter, 2 A fuse)"
how = "A buck converter whose switch fails short passes the input to the output, so the rail would carry 15 V rather than 12 V into a module that has no protection. The parts doc already names the converter as a slow soft limit and the fuse as the hard limit. The fuse protects wires, not downstream modules, and a rail overvoltage is a separate fault with no sensing today."
test = "ngspice: converter shorted-switch case with the fuse and load. bench: apply 15 V to the rail output with a current-limited supply into the lamp and a sacrificial module and see which part fails first; add an overvoltage clamp or crowbar test."
expected = "Overvoltage on the rail is clamped or opens a fuse faster than a module is damaged, and the rail voltage is monitored."
severity = "medium"
status = "gap"
sources = ["https://www.analog.com/en/resources/technical-articles/how-glitchfree-supervisors-aid-in-highreliability-applications.html"]
```

```toml
id = "hw-030"
title = "Undetected first fault accumulates until a second one defeats both layers"
category = "functional safety practice"
target = "whole safety stack (relay, MOSFET, charge pump, feedback, e-stop)"
how = "ISO 13849 category 3 tolerates one fault without losing the safety function but does not prevent fault accumulation, and category 4 goes further. The point for a hobby bench is that a welded relay unseen for a month, then a shorted MOSFET, equals no cut. Diagnostic coverage is the share of dangerous failures that are detected; a proof test at every arm raises it cheaply. Common cause matters too: one 5 V rail, one adapter or one pin feeding both layers is one fault."
test = "bench: a written fault-injection table (each single fault on each layer, expected detection point) run at acceptance and after any rewiring, and a power-on self-test at arm that exercises the relay, MOSFET and feedback. software: a model that enumerates pairs of component faults and asserts the unsafe set is empty."
expected = "Every single fault is detected before the next arm; the pair table shows no pair of undetected faults that leaves the rail live."
severity = "high"
status = "gap"
sources = ["https://ez.analog.com/ez-blogs/b/engineerzone-spotlight/posts/an-introduction-to-machine-safety-standard-iso-13849", "https://www.digikey.com/en/articles/elevating-functional-safety-with-relay-modules"]
```

## Notes on source quality and caveats

- Every URL listed in a `sources` field was opened for this pass. One caveat is the RP2350 challenge detail, where the Raspberry Pi results page and Hackster disagree slightly on who found which erratum (the EMFI work credited to Hextree is E21 in the Raspberry Pi page, and Hackster says it was a commissioned test). The catalog only relies on the attack classes.
- hw-005, hw-029 and parts of hw-002 rest on general electronics knowledge plus the repo docs; the opened sources support the wiring-failure framing but not those specific component claims.
- The DigiKey MOSFET guide, the e-stop and welded-contact articles are practitioner explainers, not the standards themselves (IEC 61508, ISO 13849 were not opened in full text).
- Two pages named in the brief do not exist yet in the repo: `docs/THREAT-MODEL.md` and `crates/sawbones-approvals`.
