# Red-team catalog 02: attacks and failure modes on serial and CAN buses that touch write paths

Scope: Modbus RTU over RS-232/RS-485 now, CAN battery systems later. Defensive catalog for our own bench and our own crates. Read: `crates/sawbones-policy`, `crates/sawbones-busguard` (source, rustdoc, tests) and `docs/` in sawbones-labs. Note: `docs/THREAT-MODEL.md` does not exist in the repo yet (only ROADMAP says it is planned), so the crate docs were the baseline.

## Overview

What the crates do today:

- Bus guard: parses one Modbus RTU request (functions 0x03/0x04/0x06/0x10 only, exact length, CRC, quantity, byte count, no address overflow, units 1..=247). Reads need an allow-list rule. Writes need a rule, a live lease on the same port, and a SHA-256 of the exact frame bytes equal to the lease digest. The lease is single-use, expires on the guard clock, and ends on a backward clock, a restart, or a new allow-list.
- Policy engine: `classify` maps (port, signal) to a class via a write map of facts, raises it by setting kind and energy relevance, refuses unknown or unattested signals, and refuses any transmit without an attested `BenchOnly` fact. The write value plays no part.

Coverage legend used below: `blocked` means current code plus tests already enforce it (add the CI test if one is not already there). `partial` means part is enforced. `gap` means not enforced by the crates; the test is expected to fail or cannot be written until the feature exists.

Findings in short:

- The wire-level frame checks are strong. Broadcast, odd function codes, length games, CRC games, replay of a spent frame, and any single-byte change are all stopped.
- The weak joints are between layers. Policy reasons about names and values, the guard about addresses and bytes, and nothing yet ties them (bus-014, 015, 016, 017). A policy lease approves an `ActionDigest` while the guard needs a `FrameDigest`, so whoever encodes the frame is a confused deputy.
- Trust-root replay and rollback, and the decide-then-transmit window, are known and documented as open (bus-008, bus-009).
- Mastership is a static declaration; nothing observes a second master appearing (bus-020), and there are write paths that never cross the guard (bus-021).
- CAN has no code yet. The master/slave model in `Mastership` does not fit CAN, which has ID ownership and error-counter side effects (bus-025 to bus-029).

Sources actually opened are listed per entry. Several sources contradict the "obvious" claim (for example the NRM paper has little on RTU timing, and neither CISA advisory mentions Modbus); entries only claim what the page supports and say when a point is our own inference.

## Catalog

```toml
id = "bus-001"
title = "Broadcast and reserved unit addresses"
category = "protocol edge case: broadcast"
target = "bus guard (frame parser)"
how = "Unit 0 is broadcast: every slave acts and none replies, so a broadcast write cannot be verified and hits every device on the line. Units 248 to 255 are reserved. The parser rejects both for every function, read or write, before allow-list or lease logic runs."
test = "For functions 0x03, 0x04, 0x06 and 0x10, build a valid-CRC frame with unit 0, and with each of 248..=255. With an allow-list that covers the whole address space and a live lease whose digest equals that exact frame, decide() must return Block(Frame(Broadcast)) or Block(Frame(ReservedAddress(n))), and guard.lease() must be unchanged afterwards. Also assert AllowList::try_push refuses a Rule with unit 0 or above 247."
expected = "Blocked every time; lease neither spent nor altered."
severity = "high"
status = "blocked"
sources = ["https://millenia.cars.aps.anl.gov/software/epics/Modbus_over_serial_line_V1_02.pdf"]
```

```toml
id = "bus-002"
title = "Unsupported and dangerous function codes"
category = "protocol edge case: function code abuse"
target = "bus guard (Function::from_code), policy engine (no function concept)"
how = "Diagnostics (0x08) include Force Listen Only (sub-function 0x04), which makes a serial slave stop replying, and counter clears. Also outside the four supported codes: coil writes (0x05, 0x0F), mask write (0x16), read/write multiple (0x17, a read-named write), file records (0x14, 0x15), report slave ID (0x11), encapsulated interface (0x2B) and vendor ranges 65-72 and 100-110. Any of these can carry a write or take a device out of service, and the guard blocks them by never parsing them."
test = "For every function code 0..=255 except 3, 4, 6, 16: build a frame of length 8 and of length 12 with a correct CRC and unit 1, install a guard whose allow-list has a rule for every supported function over the full range and a lease for that exact frame. decide() must return Block(Frame(UnsupportedFunction(code))) in all cases. Add a named regression for 0x08 sub-function 0x0004, 0x16 and 0x17."
expected = "All 252 codes blocked with UnsupportedFunction; adding a new Function variant later must fail a snapshot test that lists the supported set."
severity = "high"
status = "blocked"
sources = ["https://industrialmonitordirect.com/blogs/knowledgebase/exploring-modbus-rtu-function-codes-timing-and-data-encoding", "https://tenable.com/plugins/nnm/41", "https://webdiis.unizar.es/~ricardo/files/papers/NRM-ICITST-16.pdf"]
```

```toml
id = "bus-003"
title = "Write multiple (0x10) whose range reaches a protected register, and degenerate counts"
category = "protocol edge case: range and quantity abuse"
target = "bus guard (Rule::permits, parse)"
how = "A 0x10 write may start on a permitted register but carry a quantity that spills onto a protected neighbour. Zero quantity, a byte count that is not twice the quantity, and ranges that run past 0xFFFF are malformed and have crashed or leaked memory in real gateways. The guard checks the last register addressed, not just the first."
test = "Rule{0x06, regs 100..=101, max_quantity 2}, lease for each frame in turn. A 0x10 at start 100 qty 3 (reaches 102) must be Block(NotAllowListed). A 0x10 at start 101 qty 2 must block. Quantity 0, byte_count != 2*qty, start 0xFFFF qty 2 and qty 124 must give QuantityOutOfRange, ByteCountMismatch, AddressOverflow, QuantityOutOfRange. A 0x06 at 102 must block. Property test: for random start/qty, decide() passes only if every register in start..start+qty is inside some rule's first..=last."
expected = "Blocked with the stated reasons; the property holds for 10k random cases."
severity = "critical"
status = "blocked"
sources = ["https://i.blackhat.com/USA-20/Wednesday/us-20-Balduzzi-Industrial-Protocol-Gateways-Under-Analysis-wp.pdf", "https://seqred-website.pages.dev/en/blog/modbus-rtu/"]
```

```toml
id = "bus-004"
title = "Frame concatenation, trailing bytes, and split frames"
category = "protocol edge case: frame splitting and timing gaps"
target = "bus guard (parse), pod firmware framing"
how = "Modbus RTU frames end by silence (t3.5), and a gap over t1.5 inside a frame makes the receiver discard it. An attacker or buggy firmware can glue an approved read to an unapproved write in one buffer, or split an approved write so each half is judged alone. The guard rejects trailing bytes (LengthMismatch) and partial frames (Truncated), but the firmware, not the crate, decides where a buffer begins and ends."
test = "(a) decide() on read_frame ++ write_frame, and on write_frame ++ one extra byte, must return Block(Frame(LengthMismatch)). (b) For the approved write W, for every split index 1..len-1, decide(first half) and decide(second half) must both Block, and after all splits the lease must still be present and decide(W) must Pass exactly once. (c) Firmware-sim test: bytes arriving with a gap above t1.5 are dropped as one partial frame, never merged with the next burst."
expected = "(a) and (b) blocked by the crate today. (c) is a firmware gap: no framer exists in the crate."
severity = "high"
status = "partial"
sources = ["https://millenia.cars.aps.anl.gov/software/epics/Modbus_over_serial_line_V1_02.pdf", "https://industrialmonitordirect.com/blogs/knowledgebase/exploring-modbus-rtu-function-codes-timing-and-data-encoding"]
```

```toml
id = "bus-005"
title = "No minimum spacing between two passed frames"
category = "protocol edge case: timing gaps"
target = "bus guard / pod firmware transmit scheduler"
how = "The guard answers Pass per buffer and has no notion of when the last frame left. If two passed frames are sent with less than t3.5 of silence between them, a device may treat them as one malformed frame, or a lenient parser may find an inner frame the guard never judged as a single unit. Inference from the spec's frame-separation rule; no source documents an exploit on a specific device."
test = "Simulated line model: guard passes frame A then frame B within 1 ms at 9600 baud. The firmware scheduler under test must delay B until at least t3.5 (plus margin) after the last byte of A, and assert the line model sees two separate frames. Add a unit test for a pure function min_gap_ticks(baud, bits_per_char) with the spec's fixed values above 19200 baud."
expected = "Transmitter enforces the gap; a test that sends back to back fails today because no scheduler or gap helper exists."
severity = "medium"
status = "gap"
sources = ["https://millenia.cars.aps.anl.gov/software/epics/Modbus_over_serial_line_V1_02.pdf"]
```

```toml
id = "bus-006"
title = "CRC is an error check, not authentication"
category = "CRC collision and forgery"
target = "bus guard (lease digest)"
how = "CRC-16 detects noise only. Anyone who changes a payload can recompute the CRC, and a crafted pair of frames can share one. The guard does not rely on the CRC for writes: it binds the lease to SHA-256 of the full frame, so a recomputed or colliding CRC still yields a different digest."
test = "Approve write W. For every payload bit flip of W, recompute a correct CRC, decide(): Block(DigestMismatch), lease kept. Brute-force a second frame W2 (different value) with the same CRC as W over a free 2-byte tweak; decide(W2) must Block(DigestMismatch). Swap the two CRC bytes of W: Block(Frame(BadCrc))."
expected = "All blocked; approved W still passes once afterwards."
severity = "high"
status = "blocked"
sources = ["https://seqred-website.pages.dev/en/blog/modbus-rtu/", "https://industrialmonitordirect.com/blogs/knowledgebase/modbus-communication-error-safety-crc-protection-analysis", "https://attack.mitre.org/techniques/T0843/"]
```

```toml
id = "bus-007"
title = "Spent lease, retries after exception or timeout, and lease lifetime"
category = "replay of a captured write; lifetime abuse"
target = "bus guard (lease), policy engine (lease semantics)"
how = "After a write passes, an identical resend (agent retry after a busy or exception reply, or a replayed capture on the line side) needs a fresh press. A lease also dies on expiry, on a backward clock and on restart. Passing spends the lease at decision time, so a write with no reply leaves its effect unknown and the host must not assume it can resend."
test = "Grant lease for W, decide(W) Pass, decide(W) again Block(NoLease). Grant, advance clock to expiry: Block(LeaseExpired). Grant, clock goes backwards then decide: Block(ClockWentBackwards); grant on a backward clock: Err(ClockWentBackwards). Grant, restart(): Block(NoLease), allow-list intact. Grant at tick u64::MAX-1 must still expire. Host-level test (gap): after a Pass with no reply, the host marks the action outcome_unknown and refuses a new lease request until a readback succeeds."
expected = "Crate cases blocked. The outcome_unknown host rule does not exist yet."
severity = "high"
status = "partial"
sources = ["https://attack.mitre.org/techniques/T0830/", "https://industrialmonitordirect.com/blogs/knowledgebase/exploring-modbus-rtu-function-codes-timing-and-data-encoding"]
```

```toml
id = "bus-008"
title = "Replay or rollback of trust-root updates (grant and allow-list)"
category = "replay; unauthorized command message"
target = "bus guard (TrustRootEvent, BusGuard::apply)"
how = "TrustRootEvent::unverified_from_trust_root checks nothing, and the crate docs say replay protection waits for the signed envelope (#172). A captured grant can be re-sent for the same digest, and an older, broader allow-list can be re-sent to roll the guard back. In-process the event is move-only, which does not help once bytes cross the trust-root link."
test = "Future test, expected to fail today: build two signed envelopes with counters 5 and 6. Apply 5, 6, then 5 again: the third must be refused with a Replay error. Apply allow-list v2 then v1: refused as Rollback. Counter must survive restart(). Cross-device: an envelope signed for guard A is refused by guard B. Today, add a test that documents the gap by applying the same bytes twice through two constructions and asserting both succeed."
expected = "Refused once #172 lands; currently accepted (documented gap)."
severity = "critical"
status = "gap"
sources = ["https://attack.mitre.org/techniques/T0830/", "https://attack.mitre.org/techniques/T1692/001", "https://www.cisa.gov/news-events/ics-advisories/icsa-25-219-07"]
```

```toml
id = "bus-009"
title = "Time of check to time of use between decide() and the wire"
category = "man in the middle inside the pod"
target = "bus guard firmware contract"
how = "decide() takes a borrowed slice. If the firmware decides on a buffer that a DMA engine, second core or interrupt handler can still modify, the bytes sent may differ from the bytes judged. The docs say the firmware must copy, decide on its copy and send exactly that copy, but the type system does not enforce it."
test = "Host-side harness with a mutation hook that rewrites the shared buffer between decide() returning Pass and the transmit call. The firmware wrapper must send the decided copy: assert bytes on the simulated line equal the bytes passed to decide(). Better: change the API so decide() returns an Approved token that owns a copy, and transmit only accepts that token (compile_fail test for transmitting a raw slice)."
expected = "Wire bytes equal decided bytes; today nothing enforces it."
severity = "critical"
status = "gap"
sources = ["https://attack.mitre.org/techniques/T1692/001", "https://i.blackhat.com/USA-20/Wednesday/us-20-Balduzzi-Industrial-Protocol-Gateways-Under-Analysis-wp.pdf"]
```

```toml
id = "bus-010"
title = "Over-broad write rules and holes inside permitted ranges"
category = "write to undocumented registers"
target = "bus guard (AllowList), trust-root allow-list authoring"
how = "Rule::is_valid accepts a write rule over 0..=0xFFFF with quantity 123, and any contiguous range covers undocumented holes between documented registers. The lease still limits a write to one approved frame, but a person approves what is shown to them, so a wide rule widens what a mistaken or tricked approval can reach."
test = "Add AllowList::validate_against(write_map_spans). A write Rule whose span includes any register absent from the map, or whose span exceeds the sum of mapped registers, must be rejected at build time. Test with rule 100..=140 where 117 is undocumented: rejected, while per-signal rules 100..=116 and 118..=140 are accepted. Test that a rule with function 0x10 and max_quantity above the largest mapped multi-register signal is rejected."
expected = "Rejected at build time; today try_push accepts both."
severity = "high"
status = "gap"
sources = ["https://attack.mitre.org/techniques/T0836/", "https://attack.mitre.org/techniques/T0806/", "https://i.blackhat.com/USA-20/Wednesday/us-20-Balduzzi-Industrial-Protocol-Gateways-Under-Analysis-wp.pdf"]
```

```toml
id = "bus-011"
title = "Agent sweeps registers or probes with writes"
category = "brute force I/O"
target = "policy engine (UnknownSignal), bus guard (allow-list, lease)"
how = "Community register-map discovery often means writing test values to guessed addresses. ATT&CK describes sweeping a range of points or hammering one point. Policy refuses a signal with no write-map entry, and the guard refuses an address no rule covers or any write with no lease."
test = "Policy: 200 random signal names with a map of 5 entries, every BusWrite to an unlisted name gives refused(UnknownSignal). Guard: with a read-only allow-list and no lease, send a 0x06 write to each address 0..=65535 on units 1..=3: zero Pass. With one lease for W, a sweep of 65536 frames yields exactly one Pass (W) and only if W is sent."
expected = "Zero or exactly one Pass; no bus traffic for refused requests."
severity = "high"
status = "blocked"
sources = ["https://attack.mitre.org/techniques/T0806/"]
```

```toml
id = "bus-012"
title = "Reads are not free: breadth, side effects and rate"
category = "denial of service; scanning"
target = "bus guard (read rules), policy engine (BusRead)"
how = "Reads pass on an allow-list rule alone, with no lease and no rate limit. A wide read rule lets an agent scan registers, a 125-register read ties up a slow slave, and some devices run side effects on certain reads (latched or clear-on-read values). ATT&CK DoS notes that fragile devices can stop answering under request floods or requests they cannot handle. Device-specific side effects are our inference, not from a cited source."
test = "Guard: 10,000 allowed reads inside 100 ticks. Add a per-port token bucket in the guard; reads beyond the configured rate must Block(RateLimited). Rule test: a read rule with max_quantity above the configured per-port ceiling is rejected at build time. Property: a read rule may never overlap a register tagged no_read in the write map."
expected = "Rate limit and no_read tag enforced; today unlimited."
severity = "medium"
status = "gap"
sources = ["https://attack.mitre.org/techniques/T0814/", "https://attack.mitre.org/techniques/T0806/"]
```

```toml
id = "bus-013"
title = "Driver-enable stuck on (babbling transmitter)"
category = "denial of service on a shared RS-485 bus"
target = "bus guard hardware path, pod firmware"
how = "On half-duplex RS-485 the transmit-enable pin decides who holds the line. If the pin is left asserted by a crash, a boot-time default or a firmware bug, the pod blocks every other node, including the inverter's own monitor, even though no byte was ever allowed. The crate says the pin is firmware's job and gives it no check."
test = "Pod simulation: assert DE high with no frame in flight. A watchdog must drop DE within one maximum-frame time (256 bytes at the configured baud plus t3.5). Reset-state test: after restart() and before first decide(), simulated DE is low. Assert DE only ever rises while a decided frame is being clocked out."
expected = "DE released by watchdog; low at reset. Neither exists in the crates."
severity = "medium"
status = "gap"
sources = ["https://attack.mitre.org/techniques/T0814/", "https://seqred-website.pages.dev/en/blog/modbus-rtu/"]
```

```toml
id = "bus-014"
title = "Confused deputy: policy approves a named action, the guard approves bytes, nothing links them"
category = "unauthorized command message via the encoder"
target = "policy engine (ActionDigest) and bus guard (FrameDigest)"
how = "A class 3 lease approves an ActionDigest of port, signal and value. The guard needs a FrameDigest of wire bytes. Whoever turns the action into a frame (plugin, host, trust root) decides which unit, register and bytes the digest covers. A plugin that encodes 'display_brightness=3' as a write to a battery limit gets a lease the person believes is harmless. No crate holds the signal-to-address map or the encoder."
test = "Future trusted encoder: encode(write_map_with_addresses, BusWrite) -> frame bytes, deterministic. Trust root grants a lease only for digest(encode(action)). Test: a plugin-supplied frame that differs from encode(action) in unit, address, width or value is refused before any lease is granted. Golden test: for every entry in a sample write map, encoded address range equals the entry's declared span. Today the test cannot be written; add a failing placeholder."
expected = "Lease only for the host-encoded frame; currently unenforced."
severity = "critical"
status = "gap"
sources = ["https://attack.mitre.org/techniques/T1692/001", "https://attack.mitre.org/techniques/T0836/"]
```

```toml
id = "bus-015"
title = "Register aliasing, multi-register signals and off-by-one addressing"
category = "register aliasing"
target = "policy engine (WriteEntry has no address), write map authoring, bus guard rules"
how = "WriteEntry carries a name, class and kind but no address, width or alias list. Two names can reach one register, a U32 signal spans two registers while only one may be classed, and documented one-based addresses (40001 style) versus wire zero-based addresses invite an off-by-one onto a protected neighbour. A real gateway let coil writes overwrite a holding register because the memory areas were not separate."
test = "Once WriteEntry gains address, width, unit: WriteMap::new must reject overlapping spans on one port and unit with different classes or kinds. A U32 signal must occupy two registers and both must inherit its class. Property test: shift every encoded address by +1 and -1; the result must hit another mapped entry of equal or higher class, or be refused by the guard rules. Coil-versus-register overlap on devices that alias them is a recorded per-device fact tested in the driver."
expected = "Overlap and neighbour shifts caught at load time; not possible today."
severity = "high"
status = "gap"
sources = ["https://i.blackhat.com/USA-20/Wednesday/us-20-Balduzzi-Industrial-Protocol-Gateways-Under-Analysis-wp.pdf", "https://attack.mitre.org/techniques/T0836/"]
```

```toml
id = "bus-016"
title = "Several units on one port share one signal name"
category = "unit address confusion"
target = "policy engine (WriteEntry, BusWrite, ActionDigest), bus guard"
how = "Paralleled inverters sit on one RS-485 line with different unit addresses. The policy key is (port, signal) and the ActionDigest has no unit, so a lease request for 'battery_low_cutoff' on rs485-1 is identical for unit 1 and unit 2. The guard digest includes the unit, so the person approves one thing and the encoder can pick either. One gateway also remapped unit IDs above 127 down to a lower one."
test = "Add unit to BusWrite and WriteEntry. Test: two units, same signal; ActionDigest differs by unit. A write-map entry for unit 1 must not authorise unit 2 (UnknownSignal). Today: assert that ActionDigest::of is equal for requests that differ only in target unit and mark the test as a known failing gap."
expected = "Distinct digests and per-unit entries; today they collide."
severity = "high"
status = "gap"
sources = ["https://millenia.cars.aps.anl.gov/software/epics/Modbus_over_serial_line_V1_02.pdf", "https://i.blackhat.com/USA-20/Wednesday/us-20-Balduzzi-Industrial-Protocol-Gateways-Under-Analysis-wp.pdf"]
```

```toml
id = "bus-017"
title = "Value-dependent hazard: the value plays no part in the class"
category = "modify parameter"
target = "policy engine (classify)"
how = "classify ignores WriteValue. A Config signal can still be dangerous at some values: a mode selector, a command register where one magic value means reset, a limit written to 0 or 0xFFFF, or a sign flip when I16 is sent as U16. ATT&CK describes parameter changes that are dangerous, out of bounds or invalid, and recommends validating content."
test = "Add per-entry value constraints (min, max, allowed set, width and signedness). Test: BusWrite U16(0xFFFF) to an entry limited to 0..=100 is refused (ValueOutOfRange); I16(-1) to a U16 entry is refused (WidthMismatch); Bool to a register entry refused; a command register permits only its listed values. Property test: class never decreases when the value changes."
expected = "Refused; today any value is classed the same and the crate docs say ranges are not here."
severity = "high"
status = "gap"
sources = ["https://attack.mitre.org/techniques/T0836/"]
```

```toml
id = "bus-018"
title = "Communication, firmware and reset registers have no setting kind"
category = "firmware-update function paths; modify parameter"
target = "policy engine (SettingKind), write map"
how = "SettingKind has Config, battery limits, charge voltage and two mains kinds. Slave address, baud rate, parity, remote on/off, factory reset and firmware-staging registers are not named, so they fall to Config (class 2 with rail A). Changing the slave address can duplicate another unit or lock the bench out, and a baud change breaks the listen-only pod's framing. A CISA advisory notes firmware installable over serial with no integrity check."
test = "Add kinds CommunicationSetting, FactoryReset, FirmwareUpdate, RemoteOnOff with minimum class 3 or 4. Test: an entry of kind CommunicationSetting with class 0 classifies as Class3 at least; FirmwareUpdate and FactoryReset classify as refused. Lint: loading a write map with any entry whose kind is Config but which is tagged in the driver's register metadata as comm or reset fails."
expected = "Raised or refused; today they are classed as Config."
severity = "high"
status = "gap"
sources = ["https://www.cisa.gov/news-events/ics-advisories/icsa-25-219-07", "https://attack.mitre.org/techniques/T0843/", "https://millenia.cars.aps.anl.gov/software/epics/Modbus_over_serial_line_V1_02.pdf"]
```

```toml
id = "bus-019"
title = "Mastership: no fact, unattested fact, or another master declared"
category = "man in the middle; bus contention"
target = "mastership facts (BusFacts), policy engine"
how = "A request that transmits needs an attested BenchOnly fact for its port. With no fact, a fact below OperatorAttested, or OtherMasterActive, classify refuses. RTU buses have one master and any device can transmit onto the line, so contention is a real risk."
test = "classify() for BusRead and BusWrite on: a port with no fact -> refused(NoMastershipFact); fact trust Documented, AgentInferred, Unverified -> refused(MastershipNotAttested); OtherMasterActive -> refused(OtherMasterActive); BusListen on all three -> Class0 Nothing. Duplicate facts for a port -> FactError."
expected = "All refused as listed; BusListen unaffected."
severity = "high"
status = "blocked"
sources = ["https://seqred-website.pages.dev/en/blog/modbus-rtu/", "https://imt.uoradea.ro/auo.fmte/files-2015-v1/Gabor%20JAKABOCZKI%20-%20VULNERABILITIES%20OF%20MODBUS%20RTU%20PROTOCOL%20-%20A%20CASE%20STUDY.pdf"]
```

```toml
id = "bus-020"
title = "Mastership is static: a second master appears after attestation"
category = "bus contention; man in the middle"
target = "mastership facts, listen-only pod"
how = "BenchOnly is a declaration. If the inverter's own monitor, a Solar Assistant dongle or a phone dongle starts polling later, the bench keeps transmitting into a contested bus and two requests can collide into a hybrid frame. The listen-only pod could see foreign request frames but nothing feeds that back. A half-duplex receiver also hears the bench's own frames, so the check must tell echo from foreign traffic."
test = "Add observe_traffic(port, captured_frames, own_tx_log) that downgrades the port's fact to OtherMasterActive when a valid request frame appears that is not in the own transmit log. Test with a capture containing a foreign 0x03 request: classify(BusRead) flips from RailA to refused(OtherMasterActive). Echo test: capture equals own tx log only: fact unchanged. Downgrade must persist until re-attested by the operator."
expected = "Fact downgraded and requests refused; no such function exists."
severity = "high"
status = "gap"
sources = ["https://millenia.cars.aps.anl.gov/software/epics/Modbus_over_serial_line_V1_02.pdf", "https://seqred-website.pages.dev/en/blog/modbus-rtu/"]
```

```toml
id = "bus-021"
title = "Write paths that never cross the guard, and stale reads before a write"
category = "unauthorized command message; modify parameter"
target = "policy engine (lease binding), docs"
how = "Inverters often have a second write path: a Wi-Fi or Bluetooth dongle, a cloud service or an app. The EG4 advisory describes unencrypted monitoring traffic that can be forged, including control commands. Another advisory describes command injection in an inverter's web interface. Such a change between our read and our approved write means the lease approves an action against state that no longer holds. The crate docs list state binding as not done."
test = "Add an optional precondition to BusWrite (expected_current: WriteValue) that is part of canonical_bytes. Test: ActionDigest differs with and without precondition; the host refuses to execute when a fresh readback differs from the precondition. Bus-level test (gap): the host records a before/after readback pair for every leased write and flags change by an unknown writer."
expected = "Stale-state writes refused; not possible today."
severity = "medium"
status = "gap"
sources = ["https://www.cisa.gov/news-events/ics-advisories/icsa-25-219-07", "https://cisa.gov/news-events/ics-advisories/icsa-25-044-17", "https://attack.mitre.org/techniques/T0836/"]
```

```toml
id = "bus-022"
title = "Spoofed or missing replies: the guard judges requests only"
category = "spoof reporting message; man in the middle"
target = "bus guard (documented non-goal), host response parser"
how = "A device on the line can answer in place of a slave, alter responses, or silence the real slave with a Force Listen Only request and then answer for it. A formal-model paper shows exactly this on Modbus. An exception reply (function code with the high bit set) or no reply means the write may not have applied. The guard does not look at replies, so write success and read truth are taken on trust unless the host checks."
test = "Response parser tests (host side): reply with wrong unit, wrong function, bad CRC, or exception bit set -> outcome write_unconfirmed. After a leased write, require an allow-listed readback; a mismatch marks the action failed. Duplicate-reply test: two different valid replies to one request in one window -> flag as possible second responder and set mastership to contested."
expected = "Unconfirmed writes never reported as success; no response parser exists in these crates."
severity = "high"
status = "gap"
sources = ["https://webdiis.unizar.es/~ricardo/files/papers/NRM-ICITST-16.pdf", "https://attack.mitre.org/techniques/T0830/", "https://seqred-website.pages.dev/en/blog/modbus-rtu/"]
```

```toml
id = "bus-023"
title = "Gateways that rewrite what the guard saw"
category = "man in the middle; protocol translation"
target = "bus guard placement, topology facts"
how = "Gateways between Ethernet and serial have turned a read into a serial write, forwarded malformed frames, fanned one write into several, and remapped unit IDs. A guard in front of such a gateway judges a message that the gateway then changes. The guard is only meaningful on the final hop, directly before the line driver, as the crate describes."
test = "Topology validation in the bench config: a port's guard must be the last element before the line driver; a gateway model that mutates function or unit placed after the guard must fail validation. Simulation: gateway converts read to write after the guard; test harness flags the divergence between decided bytes and line bytes."
expected = "Config rejected; no topology check today."
severity = "medium"
status = "gap"
sources = ["https://i.blackhat.com/USA-20/Wednesday/us-20-Balduzzi-Industrial-Protocol-Gateways-Under-Analysis-wp.pdf", "https://seqred-website.pages.dev/en/blog/modbus-rtu/"]
```

```toml
id = "bus-024"
title = "Policy facts: BusRead of unknown signals, mislabelled kinds, unattested entries"
category = "write map integrity; unauthorized command message"
target = "policy engine (classify), write map"
how = "classify looks up the write map only for writes. A BusRead of any signal name is class 2 with rail A and no lookup, so a read of an unlisted or side-effecting register is permitted by policy. A battery limit entered with kind Config is classed from the entry's own class, so the kind is only as honest as its author. Entries below OperatorAttested are ignored, which is correct."
test = "(a) BusWrite with entry trust Documented, AgentInferred, Unverified -> refused(EntryNotAttested): blocked today. (b) BusRead of a signal with no entry: expected refused(UnknownSignal); today Class2 RailA (gap). (c) Authoring lint, run outside classify so the rule 'class never depends on what a name says' holds: warn when kind is Config and the register metadata marks protection or voltage."
expected = "(a) blocked; (b) and (c) need new code."
severity = "medium"
status = "partial"
sources = ["https://attack.mitre.org/techniques/T0836/", "https://attack.mitre.org/techniques/T1692/001"]
```

```toml
id = "bus-025"
title = "Listen-only pod that is not passive on the wire"
category = "unintended transmission"
target = "listen-only pod firmware, RS-485 and CAN transceivers"
how = "A pod meant to listen must have no path to transmit. On RS-485 that is the DE pin and a receiver-only wiring option. On CAN a controller in normal mode drives the ACK bit and error frames, which count toward other nodes' error counters, so only a controller in a silent or listen-only mode is passive. The research on CAN error handling shows how error behaviour moves a victim's counters."
test = "Firmware simulation: in listen-only mode, across 10k received frames (including malformed ones), the TX pin and DE pin never leave idle; the CAN peripheral configuration read back from the driver reports silent mode; an arm-switch-off test asserts the same. Compile-time: the listen-only build exposes no transmit function."
expected = "Zero transmit edges; no firmware crate exists yet to test."
severity = "high"
status = "gap"
sources = ["https://beerkay.github.io/papers/Berkay2021CanoxUSENIXSecurity.pdf", "https://rtcl.eecs.umich.edu/rtclweb/assets/publications/2016/ktcho_busoff_CCS_16.pdf"]
```

```toml
id = "bus-026"
title = "CAN: our own transmissions pushing a battery node toward bus-off"
category = "denial of service; CAN error handling"
target = "future CAN guard, CAN pod firmware"
how = "CAN nodes that see errors raise an error counter and go bus-off past 255. Research shows a transmitter that collides with a periodic message can push a healthy node into error-passive and then bus-off, and that message authentication does not help because reception breaks first. A bench that transmits on a live battery bus, even by mistake, can take a BMS off the bus."
test = "CAN simulator with TEC/REC per node. Test: bench transmits an ID that a simulated BMS also sends, at the same moment: the guard must refuse any ID in the BMS-owned set (see bus-027). Property: with the bench's allowed transmit set and rate, no simulated node's TEC rises above a bound over 1M ticks. Bench must stop transmitting when it observes a rising error-frame rate."
expected = "No collisions with owned IDs; counters stay low. No CAN crate exists yet."
severity = "high"
status = "gap"
sources = ["https://rtcl.eecs.umich.edu/rtclweb/assets/publications/2016/ktcho_busoff_CCS_16.pdf", "https://beerkay.github.io/papers/Berkay2021CanoxUSENIXSecurity.pdf"]
```

```toml
id = "bus-027"
title = "CAN has no master: ID ownership, extended-vs-standard aliasing, spoofed BMS limit frames"
category = "spoof reporting message; mastership facts"
target = "mastership facts (Mastership enum), future CAN allow-list"
how = "Mastership has BenchOnly and OtherMasterActive, which describe Modbus. On CAN every node transmits, and a battery system's charge limits are carried in frames produced by the BMS. Sending one of those IDs spoofs the BMS to the inverter. CAN also separates 11-bit and 29-bit identifiers, so the same number can name different frames. A battery-system survey notes CAN lacks authentication, so an attacker on the bus can read, replay or alter battery data."
test = "Define CanKey{id, extended, fd, rtr}. Test: allow-list entry for standard 0x351 does not permit extended 0x351 or an RTR frame with that ID. Per-port fact id_owner(id) -> Producer(BMS); policy refuses a bench BusWrite to any signal whose frame ID is owned by another producer unless class 3 with lease. DLC above 8 on classic CAN refused."
expected = "Refused; the types do not exist today."
severity = "high"
status = "gap"
sources = ["https://mdpi.com/2313-0105/9/7/382", "https://rtcl.eecs.umich.edu/rtclweb/assets/publications/2016/ktcho_busoff_CCS_16.pdf", "https://attack.mitre.org/techniques/T1692/001"]
```

```toml
id = "bus-028"
title = "CAN replay of recorded frames and simulator-to-live confusion"
category = "replay"
target = "future CAN guard, sim and real drivers"
how = "CAN battery frames carry no counter or MAC in typical designs, so a recorded charge-limit or enable frame replays as valid. Our own golden recordings and simulators are the likely source: a replay feature meant for CI must not be reachable against a live bus. Replay of battery data is listed among CAN integrity attacks in the BMS survey."
test = "Type test: the replay API takes a SimBus only; passing a LiveBus is a compile_fail test. Runtime: a replay request on a live port classifies as class 3 with lease and exact-frame digest, rate-limited to one frame. Guard test: a recorded frame with the same ID but a changed payload byte is blocked by digest."
expected = "Replay cannot reach a live bus without a lease."
severity = "high"
status = "gap"
sources = ["https://mdpi.com/2313-0105/9/7/382", "https://attack.mitre.org/techniques/T0830/"]
```

```toml
id = "bus-029"
title = "CAN diagnostic services: floods, session lock, and gateways that pass UDS"
category = "unauthorized command message; denial of service"
target = "future CAN guard (service allow-list)"
how = "Diagnostic requests carried on CAN can starve a node of cycles (Read Data by Identifier floods), lock out other tools by holding a session open with a spoofed source address, and jam multi-frame transfers with flow-control abuse. A tested gateway blocked J1939 injection yet passed UDS. Our own guard must allow-list diagnostic service IDs, source and target addresses, and rate. Higher-risk services such as ECU reset and download are in ISO 14229, which we did not open; treat their listing as a to-verify item."
test = "Guard test: allow-list contains only named services; a request with any other service ID, a functional (broadcast-style) target address, or Tester Present from a non-bench source is blocked. Rate: more than N Read Data by Identifier requests per second blocked. Session: bench opens at most one session and closes it; test asserts no session is left open after a failed transfer."
expected = "Blocked; none of this exists yet."
severity = "medium"
status = "gap"
sources = ["https://www.ndss-symposium.org/wp-content/uploads/vehiclesec2024-46-paper.pdf", "https://arxiv.org/html/2510.25375"]
```

## Top gaps (summary)

1. bus-014: no trusted encoder links the policy ActionDigest to the guard FrameDigest.
2. bus-008: trust-root grant and allow-list replay and rollback (known, #172).
3. bus-009: decide-then-transmit buffer window not enforced by types.
4. bus-017: write values and widths play no part in policy.
5. bus-016 (with bus-020): no unit in the write map or digest; mastership never observed.
