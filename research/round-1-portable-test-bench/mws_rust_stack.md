# MWS, the Rust MCP/embedded stack, and free CI for an agent-operated portable test bench (state: 25 Nov 2026)

Method note: the MWS repository (`isokron-ai/mcp-write-safety`, HEAD `8990c1d`, 24 Nov 2026) and its two sibling repos were cloned and read directly. Crate versions and dates were pulled from the crates.io API on 25 Nov 2026. Everything labelled "Inference" is the researcher's interpretation, not something a source says. MWS quotes are kept under 15 words.

---

## 1. MWS: what it actually requires of an MCP server, how it is tested, and what it does not say

### Takeaway
MWS 0.9 is a public-comment draft (16 Nov - 16 Dec 2026) of 37 requirements (26 MUST, 11 SHOULD) in ten areas about whether a write has the effect asked for, on the object named, and is reported truthfully. It has no conformance levels, no "pass", no badge, and it does NOT require dry-run, confirmation/elicitation, undo, or audit logs; it never mentions physical, irreversible or hardware effects, and it puts authentication/authorization out of scope.

### Cited Findings

**Identity, status, licence**
- MWS = "MCP Write-Safety Requirements", a spec for MCP servers whose tools create, update and delete state; status "MWS 0.9, a draft for public comment", published 16 Nov 2026, comments open until 16 Dec 2026; every requirement has status `draft` — [README](https://github.com/isokron-ai/mcp-write-safety)
- Licences: code (`packages/`, `examples/`, `integrations/`) Apache-2.0; spec text and result shape CC BY 4.0; the name and mark are covered by neither (see TRADEMARKS.md); contributions use DCO sign-off — [README](https://github.com/isokron-ai/mcp-write-safety)
- Independent draft, not an MCP-project product, not endorsed by MCP maintainers; steward IsoKron states a commercial interest; all cited findings come from IsoKron's own five audits of November 2026; drafted with AI assistance; no outside review before the comment window — [spec/mws.md §1, §12](https://github.com/isokron-ai/mcp-write-safety/blob/main/spec/mws.md)
- 1.0 exit criteria include an executable test per active requirement and runs on at least five non-sample servers, at least one by an outside tester; these are "not met in 0.9" — [spec/mws.md §1](https://github.com/isokron-ai/mcp-write-safety/blob/main/spec/mws.md)
- Requirement IDs are stable and never renumbered; wording, levels and conditions may change before 1.0 — [spec/mws.md §1, §13](https://github.com/isokron-ai/mcp-write-safety/blob/main/spec/mws.md)
- Checked against MCP revisions `2025-11-25` and `2026-07-28`; reviewed against each new MCP revision within 60 days — [spec/mws.md §11.1, §13.5](https://github.com/isokron-ai/mcp-write-safety/blob/main/spec/mws.md)

**Scope**
- "It adds no protocol messages, no tool annotations and no capabilities" (layer above MCP) — [spec/mws.md §2](https://github.com/isokron-ai/mcp-write-safety/blob/main/spec/mws.md)
- In scope: create/update/delete tools; behaviour under two writers, stale reads, repeats and part-way failure; what tools report and declare (schemas, annotations); settings meant to stop writes; credentials in results — [spec/mws.md §2](https://github.com/isokron-ai/mcp-write-safety/blob/main/spec/mws.md)
- Out of scope: "authentication, authorization and transport security"; prompt injection/tool poisoning; client/agent/model behaviour; read-only tools (except input, annotation, credential rules); performance/cost — [spec/mws.md §2](https://github.com/isokron-ai/mcp-write-safety/blob/main/spec/mws.md)

**The 37 requirements (paraphrased; exact sentences in NORMATIVE-SENTENCES.md)** — [NORMATIVE-SENTENCES.md](https://github.com/isokron-ai/mcp-write-safety/blob/main/spec/NORMATIVE-SENTENCES.md)
- UPD (update semantics): UPD-1 MUST — a field-patch update changes only the fields named; no unsent defaults written. UPD-2 MUST — a list-element edit hits the element the caller addressed as read, or refuses.
- CONC (concurrency/preconditions): CONC-1 MUST — never bypass a backend concurrency control unless an explicit override argument (default off) is set. CONC-2 MUST — replace/delete/version-accepting tools refuse a stale write with a conflict wherever the backend can detect the change. CONC-3 MUST — two concurrent writes to different parts of one object must not both report success if either change is lost. CONC-4 MUST — a conflict is not applied and is returned as `isError: true` saying it was not applied. CONC-5 SHOULD — where staleness cannot be detected, the tool description says so. CONC-6 SHOULD — a conflict error tells the caller to re-read, or carries current state.
- TGT (targeting): TGT-1 MUST — a scope argument is honoured or the call rejected; never applied elsewhere. TGT-2 MUST — empty/missing/match-all selector or a container path must not mean "everything" without an explicit opt-in argument. TGT-3 MUST — a create must not overwrite an existing same-named object without an explicit overwrite argument (run/start/launch/deploy/install/clone under a caller-chosen name count as creates).
- RES (truthful results): RES-1 MUST — "report success only when the state the caller asked for is in effect". RES-2 MUST — whole or partial failure is never success; the error identifies what remains applied or says the outcome is unknown; must not call a settled outcome unknown; must not invite a blind repeat of an open-outcome write; backend refusals are `isError: true`. RES-3 MUST — a create returns the identifier of each created object even if a later step fails. RES-4 SHOULD — a receipt identifying object, changed fields and new version.
- ATOM (multi-step/multi-item): ATOM-1 MUST — incomplete multi-item/multi-step writes are not success and name which items/steps applied. ATOM-2 SHOULD — multi-object writes are all-or-nothing.
- RETRY: RETRY-1 SHOULD — a create whose repeat makes a second object accepts an idempotency key. RETRY-2 MUST (if a key is accepted) — same key + same args applied at most once and answered with the first outcome where known; different key = new write; same key + different args refused. RETRY-3 MUST — a tool that answers `input_required` (MCP 2026-07-28) must not already have put any of the write into effect.
- IN (inputs): IN-1 MUST no meaning-changing coercion; IN-2 MUST no silent drop of undefined arguments; IN-3 MUST enforce declared schema constraints (`required`, `minimum`, `maximum`, `enum`, `pattern`, etc.); IN-4 MUST validation failures are `isError: true` naming the argument and reason; IN-5 SHOULD declare every fixed enforced constraint in the schema; IN-6 SHOULD reject wrong types rather than convert; IN-7 SHOULD reject undefined arguments.
- CFG (safety settings): CFG-1 MUST — an unrecognized value of a read-only/write-restricting setting must not leave writes enabled. CFG-2 MUST — an unknown tool name in a deny-list means refuse to start/serve or disable all write tools. CFG-3 MUST — read-only mode refuses every write tool regardless of annotations. CFG-4 SHOULD — unknown name in an allow-list refuses start/serve.
- ANN (annotations): ANN-1 MUST — "A tool that declares `readOnlyHint: true` MUST NOT change state." ANN-2 MUST — `destructiveHint: false` tools never delete or remove/replace data the caller did not name. ANN-3 MUST — `idempotentHint: true` tools have no further effect on an identical repeat. ANN-4 SHOULD — declare `readOnlyHint` on every tool and `destructiveHint` on every non-read-only tool. ANN-5 SHOULD (marked at risk) — `destructiveHint: false` only for additive updates.
- SEC: SEC-1 MUST — no credential (token, secret, signed/token URL) in any tool result, success or error.

**Things the brief assumed that MWS does NOT make normative**
- Dry-run/preview: appears once, in informative Annex B-7 ("Validate, then write, then undo"), as a mitigation where the backend offers it; not a requirement — [spec/mws.md Annex B](https://github.com/isokron-ai/mcp-write-safety/blob/main/spec/mws.md)
- Undo: only in informative mitigation patterns; no requirement — [spec/mws.md Annex B](https://github.com/isokron-ai/mcp-write-safety/blob/main/spec/mws.md)
- Confirmation/elicitation: not required. The only rules are RETRY-3 (no write before an `input_required` answer) and tester rule 7.3(10); whether an accepted confirmation counts as an explicit argument is "open for comment" — [spec/mws.md §7.3](https://github.com/isokron-ai/mcp-write-safety/blob/main/spec/mws.md)
- Audit log: no requirement (grep of the spec finds zero occurrences of "audit log") — [spec/mws.md](https://github.com/isokron-ai/mcp-write-safety/blob/main/spec/mws.md)
- Receipts: SHOULD only (RES-4), and only for create/update; "a receipt for a delete" is a listed candidate, not a requirement — [spec/mws.md §12](https://github.com/isokron-ai/mcp-write-safety/blob/main/spec/mws.md)
- Idempotency keys: SHOULD on creates only (RETRY-1); widening to any append/send/increment is a stated candidate — [spec/mws.md §12](https://github.com/isokron-ai/mcp-write-safety/blob/main/spec/mws.md)
- Conformance levels: none. The spec "defines no state called 'compliant', 'conformant', 'certified' or 'passing'", no levels, profiles, score or badge; testers MUST NOT report totals, percentages or grades — [spec/mws.md §5.2](https://github.com/isokron-ai/mcp-write-safety/blob/main/spec/mws.md)
- Physical side effects: the words "physical", "irreversible", "hardware", "device", "firmware" and "actuator" do not occur anywhere in `spec/mws.md` (grep, HEAD 8990c1d). "Backend" is defined with software examples: "a hosted service, a database or a cluster" — [spec/mws.md §4](https://github.com/isokron-ai/mcp-write-safety/blob/main/spec/mws.md)

**Annotations and MCP relation**
- MWS adds no annotations; it treats each explicit hint as a claim that can be false. Cautious declarations are never wrong (NF-2). MCP schema defaults: `readOnlyHint` false, `destructiveHint` true, `idempotentHint` false, `openWorldHint` true — [spec/mws.md §11.3](https://github.com/isokron-ai/mcp-write-safety/blob/main/spec/mws.md)
- MWS reads MCP 2026-07-28's removal of stream resumability as "making repeated writes routine", which is its stated reason for area RETRY — [spec/mws.md §11.4](https://github.com/isokron-ai/mcp-write-safety/blob/main/spec/mws.md)
- Neither MCP revision defines an idempotency key for `tools/call`; MWS is neutral on the carrier (tool argument or `_meta` field) — [spec/mws.md §11.5](https://github.com/isokron-ai/mcp-write-safety/blob/main/spec/mws.md)
- A write that returns a task (Tasks extension) is judged on what the terminal task carries; "accepted/pending" results for declared-asynchronous operations are not violations (NF-19) — [spec/mws.md §10, §11.7](https://github.com/isokron-ai/mcp-write-safety/blob/main/spec/mws.md)

**Optional result shape `write-result/0.1` (Annex B-9)**
- "It is a pattern, not a requirement." Goes in `structuredContent.mws`; required members `shape` and `outcome`; `outcome` ∈ `applied | not_applied | partial | unknown | accepted`; `repeat` ∈ `no | yes | same_key | same_version | read_first`; other members include `object` (type/id/version/scope), `changed` (JSON Pointers), `replayed`, `items` (per index/step/id), `applied_count`, `code`, `argument_errors`, `conflict`, `read_with` (tool + arguments), `version_argument` — [write-result-0.1.schema.json](https://github.com/isokron-ai/mcp-write-safety/blob/main/spec/shapes/write-result-0.1.schema.json)
- Reserved codes include `conflict`, `locked`, `version_required`, `invalid_argument`, `not_found`, `idempotency_key_reused`, `in_progress`, `key_not_known`, `unavailable`, `not_permitted`, `not_sent`, `no_answer`, `backend_error`, `internal_error` — [write-result-0.1.schema.json](https://github.com/isokron-ai/mcp-write-safety/blob/main/spec/shapes/write-result-0.1.schema.json)

**How conformance is tested**
- Unit of conformance is one requirement × one write path/tool/setting; the only defined output is a "conformance statement" listing every requirement with verdicts plus the declaration used — [spec/mws.md §5](https://github.com/isokron-ai/mcp-write-safety/blob/main/spec/mws.md)
- Seven verdicts: `violated`, `platform`, `held`, `not observed`, `not tested`, `not applicable`, `self-attested`; "There is no verdict called 'pass'." — [spec/mws.md §8](https://github.com/isokron-ai/mcp-write-safety/blob/main/spec/mws.md)
- Three test methods: black-box (MCP client only, optionally launching the server), proxy-assisted (interposes between server and backend to hold/fail/cut a named backend request), review-only — [spec/mws.md §7](https://github.com/isokron-ai/mcp-write-safety/blob/main/spec/mws.md)
- Tester rule: "A tester MUST write only to objects it created for the test", in a system the operator declared a test system; destructive TGT-2 forms need an operator declaration that every reachable object is disposable — [spec/mws.md §7.3](https://github.com/isokron-ai/mcp-write-safety/blob/main/spec/mws.md)
- Limitation: "Some backends cannot be interposed on." Where a server writes inside its own process or over a protocol the interposer does not speak, forced-interleaving and fault-injection tests are `not tested` unless the server offers cooperative hooks (which must be disclosed in results) — [spec/mws.md §12, §7.3](https://github.com/isokron-ai/mcp-write-safety/blob/main/spec/mws.md)
- The declaration (Annex E) can state, "for a backend that does not answer in HTTP, the answers that report success or a refusal" — i.e. non-HTTP backends are anticipated — [spec/mws.md Annex E](https://github.com/isokron-ai/mcp-write-safety/blob/main/spec/mws.md)

**Tooling**
- `mcp-writecheck` (npm `@mcp-write-safety/writecheck` 0.1.0, Apache-2.0, Node 22+): a preview that executes the test of 14 requirements in full and 23 in part; makes real writes (up to ~1,800 for the sample server; default budget 20,000); `plan` calls no tool and prints a plan hash; `run` needs `--plan-hash` and `--confirm-target`; "No model, no account, no telemetry" — [writecheck README](https://github.com/isokron-ai/mcp-write-safety/blob/main/packages/writecheck/README.md)
- It can start a stdio server (`server.stdio`), launch a Streamable HTTP server (`server.launch`), or target a URL someone else started (then launch checks and the gate are unavailable) — [MANIFEST.md](https://github.com/isokron-ai/mcp-write-safety/blob/main/packages/writecheck/MANIFEST.md)
- The "backend gate" for proxy-assisted tests has two routes: `rewrite_env: BACKEND_URL` (tester starts a loopback forwarder in place of the real backend URL) and `node_import: true` (a Node `--import` hook wrapping `fetch`, Node servers only) — [writecheck README](https://github.com/isokron-ai/mcp-write-safety/blob/main/packages/writecheck/README.md)
- Outputs per run: `report.md`, `results.json`, `junit.xml` (only `held` passes, only `violated` fails), `capture/`, `history.jsonl`, `ledger.jsonl`, `SHA256SUMS`, optional `ssh-keygen -Y sign` signature — [writecheck README](https://github.com/isokron-ai/mcp-write-safety/blob/main/packages/writecheck/README.md)
- Sample server `tiny-tracker` ships a correct build and deliberately broken builds, one defect per requirement; it is the tester's self-test — [writecheck README](https://github.com/isokron-ai/mcp-write-safety/blob/main/packages/writecheck/README.md)
- GitHub Action `isokron-ai/mcp-writecheck-action` (status 0.1 preview; `@v0` or pin a SHA; pinned to spec repo commit `3af41c6`): modes `plan` (default; no tool call, no write, no secret), `catalog` (verdicts needing only `tools/list`, e.g. ANN-4), `full` (real writes; requires `confirm-target` and `plan-hash`); `fail-on: none|must|any`; writes a job summary and uploads a results artifact; prints no total/score/badge; needs Node 22 via `actions/setup-node` — [mcp-writecheck-action README](https://github.com/isokron-ai/mcp-writecheck-action)
- Helper kits exist for TypeScript (`@mcp-write-safety/kit` 0.1.0) and Python (`mcp-write-safety-kit` 0.1.0) only: version check, retry keys, strict inputs, result builders, unknown outcomes, annotations. No Rust kit is listed — [mcp-write-safety-kits README](https://github.com/isokron-ai/mcp-write-safety-kits)
- Two agent skills ship in the spec repo: `mcp-write-safe-builder` (the 37 requirements as build rules) and `mcp-write-safe-caller` — [builder SKILL.md](https://github.com/isokron-ai/mcp-write-safety/blob/main/integrations/skills/mcp-write-safe-builder/SKILL.md)
- Separate tool: isokron.ai/mcp-check grades a pasted `tools/list` client-side in the browser (76 checks; rubric 2.2.0; MCP revision 2026-07-28; Anthropic and OpenAI directory rules; poisoning/hidden text; schema quality; write-surface design reported as advice that lists guards such as retry keys, version checks, confirm or dry-run). Also `npx @isokron/mcp-check`, an MCP server mode, and Action `isokron-ai/mcp-check@v1` that reads tool definitions from source without running the server — [isokron.ai/mcp-check](https://isokron.ai/mcp-check)

### Inferences (researcher's interpretation of how MWS maps onto physical-instrument tools; none of this is in MWS)
- Vocabulary mapping: MWS "backend" = the instrument, bus or node firmware; "platform API" = the raw protocol (SCPI, Modbus, CAN, SWD); "object" = an addressable unit of bench state (a PSU channel config, a relay bank, a node's firmware slot, a capture job, a saved profile); "version" = a supervisor-minted generation counter per object, or a content hash (firmware image hash, config hash).
- RES-1 implies read-back verification as the default for every actuation: `set_voltage` reports `applied` only after `VOLT?`/`MEAS?` (or the node's own state report) confirms. A relay tool that cannot read back contact state cannot honestly say more than "command sent"; that should be `accepted` or disclosed, not "success".
- RES-2 is the single most important rule for hardware: a serial/BLE/Wi-Fi timeout after bytes left the host is an open outcome. The result must be `outcome: unknown` with `read_with` naming the read that settles it, never "failed, try again". A pulse, a motor step, a CAN frame injection or an RF burst repeated on "failure" is a second physical event.
- RETRY: MWS only asks for keys on creates, but on a bench almost every interesting action is non-idempotent (pulse, step, transmit, inject, start-capture, flash). Accepting an idempotency key on every non-idempotent write tool goes beyond 0.9 and matches its stated candidate. The key record must be durable (SQLite, fsync before effect) because writecheck step (e) restarts/kills the server and Wi-Fi drops make re-issued requests normal.
- RETRY-3 maps cleanly onto "arm/confirm": a tool that asks the human to confirm via `input_required` must not have closed the relay, erased flash or keyed the transmitter before the retry arrives. Draft-then-commit (prepare a plan object, then commit by handle) satisfies it.
- CONC: the "other writer" on a bench is a human turning a front-panel knob, a second agent session, or a node rebooting. Most instruments have no version check, so the supervisor must (a) serialize per instrument (pattern B-4), (b) derive a version from read-back state (B-3), and (c) say in the tool description that front-panel changes are detected best-effort only (CONC-5). Returning the state version from every read tool and requiring it on replace/delete-class tools (e.g. "load profile", "erase slot") follows CONC-2.
- TGT-1: instrument id, channel, bus and node id are scope arguments. Never fall back to "the default PSU" when the named one is missing. TGT-2: "all outputs off", "erase all nodes", "delete all captures" must not be reachable through an empty selector; make the emergency safe-state a separately named tool whose stated purpose is the whole bench (analogous to NF-18's container-delete carve-out). TGT-3: `flash_firmware(node, slot)`, `save_profile(name)`, `start_capture(name)` are creates under a caller-chosen name; refuse a collision unless `overwrite: true`.
- ATOM-1: power-cycle (off, wait, on) and flash (erase, write, verify, reset) are composite writes. A failure after "off" or after "erase" must name exactly which steps applied and what state the hardware is in. ATOM-2 (all-or-nothing) is only achievable physically where there is a real rollback, e.g. A/B firmware slots with swap-on-verify; elsewhere report `partial` honestly.
- IN: voltage/current/frequency/power limits belong in the JSON Schema as `minimum`/`maximum`/`enum` (IN-5) and must be enforced server-side (IN-3); `additionalProperties: false`; no coercion of `"5"` to 5 and no silent unit conversion (IN-1, IN-6). Put the unit in the argument name (`voltage_mV`) so there is nothing to coerce.
- CFG: a physical arm switch and a `--read-only` flag are "write-restricting settings". Unknown or unreadable switch state must fail closed (CFG-1); in read-only mode every write tool refuses regardless of annotations (CFG-3); a misspelled tool in a deny-list must stop the server or disable all writes (CFG-2).
- ANN: many instrument "reads" are not read-only: `SYST:ERR?` and `*CLS` clear queues, some Modbus registers clear on read, a DMM measurement can switch range/function, an SDR capture retunes the radio, attaching a debugger can halt a core. Those tools must not carry `readOnlyHint: true` (ANN-1). Absolute setpoints can honestly be `idempotentHint: true`; relative steps, pulses, transmits and flashes cannot (ANN-3). `destructiveHint: false` is defensible only for strictly additive actions such as starting a new capture.
- SEC-1 on a bench: never return Wi-Fi PSKs, NetworkManager secrets, API tokens, signing keys or bearer tokens in config dumps, logs or `nmcli`-style outputs.
- Testing: the MWS tester's "disposable backend" for this project is a simulated bench (vcan, Modbus/SCPI simulators, emulated nodes), never real hardware carrying real loads. Because Rust drivers run in-process or over serial/USB/CAN, the HTTP-style `rewrite_env` gate will not interpose and the Node `--import` hook does not apply; proxy-assisted records will be `not tested` unless (a) simulators are reached over a TCP/HTTP hop the gate can forward, or (b) the supervisor offers disclosed cooperative fault hooks. This should shape the driver-transport abstraction from day one.
- The Action's `plan` and `catalog` modes are cheap enough for every PR; `full` belongs on `main` against the simulated bench started inside the job.
- There is a gap a Rust project would have to fill itself: no Rust kit. A small `mws` crate (write-result/0.1 builder, durable key store, strict schema validation, per-object version/lock) is both a project need and a natural upstream contribution.
- Because MWS is in its comment window until 16 Dec 2026 and says nothing about physical effects, a "physical/irreversible effects" comment or proposal (e.g. read-back as the truth source, keys on every non-idempotent actuation, safe-state reporting on partial failure) is timely.

### Gaps
- Whether writecheck's `rewrite_env` loopback forwarder can forward raw TCP (e.g. SCPI-over-TCP, Modbus TCP) or only HTTP was not determined; the README describes it only as a "loopback forwarder" for a backend URL.
- Whether `isokron-ai/mcp-check@v1` can read tool definitions from Rust (`rmcp` `#[tool]`) source was not determined; the page only says C# is not yet read.
- writesafety.dev was not fetched directly. Annexes A, C, D and the full per-requirement test methods (§9) were not read line by line; only the normative sentences, §§1-8, 10-13 and Annex B/E headings were.
- No third-party review or adoption of MWS was found or searched for; the spec itself says none existed at publication.

---

## 2. Current MCP specification (latest revision 2026-07-28) and how Claude connects on an offline LAN

### Takeaway
The latest MCP revision is 2026-07-28: stateless (no `initialize`, no sessions), server-to-client requests replaced by `input_required` round trips, tasks moved to an official extension, resource subscriptions moved to `subscriptions/listen`, and Sampling/Roots/Logging deprecated. Claude Code can reach an HTTP MCP server on a LAN directly with a static bearer header or via stdio (including over SSH); claude.ai/Desktop "custom connectors" cannot, because they connect from Anthropic's cloud.

### Cited Findings

**Protocol state**
- 2026-07-28 removes protocol-level sessions and `Mcp-Session-Id`; list endpoints no longer vary per connection; cross-call state uses explicit server-minted handles passed as tool arguments (SEP-2567) — [MCP changelog 2026-07-28](https://modelcontextprotocol.io/specification/2026-07-28/changelog)
- The `initialize` handshake is removed; every request carries protocol version and client capabilities in `_meta`; servers MUST implement `server/discover` (SEP-2575) — [MCP changelog](https://modelcontextprotocol.io/specification/2026-07-28/changelog)
- HTTP GET stream and `resources/subscribe`/`unsubscribe` are replaced by `subscriptions/listen`, one long-lived POST-response stream for opted-in notifications (`toolsListChanged`, `promptsListChanged`, `resourcesListChanged`, `resourceSubscriptions`) — [MCP changelog](https://modelcontextprotocol.io/specification/2026-07-28/changelog)
- `ping`, `logging/setLevel` and `notifications/roots/list_changed` are removed — [MCP changelog](https://modelcontextprotocol.io/specification/2026-07-28/changelog)
- Multi Round-Trip Requests (SEP-2322) replace server-initiated `elicitation/create`, `sampling/createMessage` and `roots/list`: the server returns `resultType: "input_required"` with `inputRequests`; the client retries with `inputResponses` and `requestState`; all results now carry `resultType` — [MCP changelog](https://modelcontextprotocol.io/specification/2026-07-28/changelog)
- SSE resumability (`Last-Event-ID`) is removed: a broken response stream loses the in-flight request and the client must re-issue it with a new request ID — [MCP changelog](https://modelcontextprotocol.io/specification/2026-07-28/changelog)
- Roots, Sampling and Logging are deprecated (SEP-2577), with a minimum twelve-month deprecation window; suggested migration for logging is stderr or OpenTelemetry; OTel trace context keys (`traceparent`, `tracestate`, `baggage`) are documented for `_meta` (SEP-414) — [MCP changelog](https://modelcontextprotocol.io/specification/2026-07-28/changelog)
- `tools/list`, `resources/list`, `resources/read` etc. must carry `ttlMs` and `cacheScope`; tools SHOULD be listed in deterministic order — [MCP changelog](https://modelcontextprotocol.io/specification/2026-07-28/changelog)
- `inputSchema`/`outputSchema` may use any JSON Schema 2020-12 keywords; `structuredContent` may be any JSON value (SEP-2106) — [MCP changelog](https://modelcontextprotocol.io/specification/2026-07-28/changelog)
- Tier-1 SDKs (TypeScript, Python, Go, C#) supported 2026-07-28 at release; the Rust SDK supported it in beta at that date — [MCP blog, 28 Jul 2026](https://blog.modelcontextprotocol.io/posts/2026-07-28/)

**Transports**
- Streamable HTTP: one MCP endpoint accepting POST; each request answered with a single JSON object or an SSE stream scoped to that request; `MCP-Protocol-Version`, `Mcp-Method` and (for `tools/call`, `resources/read`, `prompts/get`) `Mcp-Name` headers are required and validated against the body — [Streamable HTTP 2026-07-28](https://modelcontextprotocol.io/specification/2026-07-28/basic/transports/streamable-http)
- Servers MUST validate `Origin` (DNS rebinding), SHOULD bind to localhost when running locally, SHOULD authenticate all connections — [Streamable HTTP 2026-07-28](https://modelcontextprotocol.io/specification/2026-07-28/basic/transports/streamable-http)
- Closing the SSE response stream MUST be treated by the server as cancellation of that request; servers are encouraged to send SSE comment keep-alives on long-lived streams — [Streamable HTTP 2026-07-28](https://modelcontextprotocol.io/specification/2026-07-28/basic/transports/streamable-http)
- stdio remains a standard transport (`notifications/cancelled` is now used only on stdio); the legacy HTTP+SSE transport is Deprecated — [Streamable HTTP 2026-07-28](https://modelcontextprotocol.io/specification/2026-07-28/basic/transports/streamable-http)

**Authorization**
- Authorization is OPTIONAL; HTTP transports SHOULD follow the spec; stdio implementations SHOULD NOT and instead take credentials from the environment — [Authorization 2026-07-28](https://modelcontextprotocol.io/specification/2026-07-28/basic/authorization)
- Based on OAuth 2.1 (draft-13); MCP servers MUST implement Protected Resource Metadata (RFC 9728); clients MUST send the RFC 8707 `resource` parameter; servers MUST validate token audience; Client ID Metadata Documents are preferred and Dynamic Client Registration is deprecated; 403 + `insufficient_scope` enables step-up — [Authorization 2026-07-28](https://modelcontextprotocol.io/specification/2026-07-28/basic/authorization)
- The tool list MAY vary by the authorization presented (e.g. only the tools the caller's scopes permit), but MUST NOT vary per connection — [Tools 2026-07-28](https://modelcontextprotocol.io/specification/2026-07-28/server/tools)

**Tools, structured output, binary content**
- Tool results may contain text, image (base64 + mimeType), audio (base64 + mimeType), `resource_link` and embedded resources, plus `structuredContent`; if `outputSchema` is given, servers MUST return conforming structured results; a tool returning structured content SHOULD also return the serialized JSON as text — [Tools 2026-07-28](https://modelcontextprotocol.io/specification/2026-07-28/server/tools)
- Clients MUST treat annotations as untrusted unless from trusted servers; there SHOULD always be a human in the loop able to deny tool invocations — [Tools 2026-07-28](https://modelcontextprotocol.io/specification/2026-07-28/server/tools)
- Servers MUST validate all tool inputs, implement access controls, rate limit tool invocations and sanitize outputs — [Tools 2026-07-28](https://modelcontextprotocol.io/specification/2026-07-28/server/tools)
- Non-normative "Stateful Tools" guidance: return an opaque handle from a creation tool, state its lifetime in the description, and return a tool execution error for expired/unknown handles — [Tools 2026-07-28](https://modelcontextprotocol.io/specification/2026-07-28/server/tools)
- Resources: `resources/read` returns `text` or base64 `blob`; optional `size`; annotations `audience`, `priority`, `lastModified`; custom URI schemes allowed; updates arrive as `notifications/resources/updated` on a `subscriptions/listen` stream when the server declares `subscribe` — [Resources 2026-07-28](https://modelcontextprotocol.io/specification/2026-07-28/server/resources)

**Tasks (long-running operations)**
- Tasks left the core and became the official extension `io.modelcontextprotocol/tasks` (SEP-2663, Final, created 27 Apr 2026): methods `tasks/get`, `tasks/update`, `tasks/cancel`; `resultType: "task"`; statuses `working | input_required | completed | cancelled | failed`; `ttlMs`, `pollIntervalMs`; creation is server-directed but a server MUST NOT return a task to a client that did not declare the extension; cancellation is cooperative; progress notifications are not supported on tasks — [SEP-2663](https://modelcontextprotocol.io/seps/2663-tasks-extension)
- A server MUST NOT return `CreateTaskResult` until the task is durably created — [SEP-2663](https://modelcontextprotocol.io/seps/2663-tasks-extension)

**Claude Code and Claude Desktop on a LAN**
- Claude Code transports: HTTP (streamable, recommended for remote), SSE (deprecated), stdio, and WebSocket via JSON config; add with `claude mcp add --transport http <name> <url> --header "Authorization: Bearer ..."`; a `headersHelper` command can mint headers per connection — [Claude Code MCP docs](https://code.claude.com/docs/en/mcp)
- If `headers.Authorization` is set, Claude Code does not fall back to OAuth; on the v2 runtime OAuth token endpoints must be HTTPS or `localhost`/`127.0.0.1`/`::1` — [Claude Code MCP docs](https://code.claude.com/docs/en/mcp)
- Claude Code's v2 runtime (MCP TypeScript SDK 2.0) adds protocol revision 2026-07-28; it probes HTTP and stdio servers for the newer revision; overrides `MCP_SDK_GENERATION`, `MCP_PROTOCOL_NEGOTIATION` — [Claude Code MCP docs](https://code.claude.com/docs/en/mcp)
- Output limits: warning above 10,000 tokens per tool output; default cap 25,000 tokens (`MAX_MCP_OUTPUT_TOKENS`); text results over 50,000 characters are saved to a file — [Claude Code MCP docs](https://code.claude.com/docs/en/mcp)
- Timeouts: `MCP_TOOL_TIMEOUT` default about 28 hours; idle timeout 5 minutes for HTTP/SSE/WS and 30 minutes for stdio; calls running past 2 minutes are auto-backgrounded — [Claude Code MCP docs](https://code.claude.com/docs/en/mcp)
- Claude Code shows elicitation dialogs in interactive sessions and refetches lists on `list_changed` — [Claude Code MCP docs](https://code.claude.com/docs/en/mcp)
- claude.ai / Claude Desktop / Cowork custom connectors connect "from Anthropic's cloud infrastructure, rather than from your local device", so the server must be reachable from the public internet; local servers configured in `claude_desktop_config.json` are a separate mechanism that uses the local network — [Claude support: custom connectors](https://support.claude.com/en/articles/11175166-get-started-with-custom-connectors-using-remote-mcp.md)
- `mcp-remote` is a local stdio proxy to remote HTTP/SSE MCP servers for clients that only speak stdio; flags include `--header`, `--header-file`, `--allow-http` (private networks only), `--transport`; custom CA via `NODE_EXTRA_CA_CERTS` — [mcp-remote README](https://github.com/geelen/mcp-remote)

### Inferences
- Design for 2026-07-28 first and keep 2025-11-25 compatibility: stateless handlers, explicit handles for captures/debug sessions/arm tokens, MRTR for human confirmation, Tasks extension for flashing/long captures/sweeps, `subscriptions/listen` for "resource changed" pings (not for the telemetry stream itself).
- Do not use Sampling (deprecated) for on-device summarization; summarise deterministically in the supervisor.
- MCP gives no streaming-sensor primitive. The token caps (10k warn / 25k cap) mean telemetry must be returned as compact statistics plus a `resource_link`, and bulk data (IQ, audio, images, logic captures) must stay on the device and be fetched out-of-band (plain HTTPS file endpoint or `scp`), with only small PNG plots/spectrograms inlined as image content.
- Removing resumability makes a flaky Wi-Fi link a write-safety problem, not just a UX one: long or physical operations should return a task handle so the result survives a dropped stream, and every actuation should carry an idempotency key.
- Simplest offline LAN paths for Claude Code on the MacBook: (1) `--transport http https://bench.local:8443/mcp` with a static bearer token header (no OAuth server needed, since OAuth is optional); (2) stdio over SSH, e.g. a server entry whose command is `ssh pi@bench.local bench-mcp --stdio`, which gets encryption and authentication from SSH keys and falls under the spec's "credentials from the environment" rule. For Claude Desktop the only LAN-capable route is a local stdio entry (SSH or `mcp-remote`), not a custom connector.
- The 5-minute HTTP idle timeout and 2-minute auto-backgrounding in Claude Code argue for progress notifications or tasks on anything slower than a minute.

### Gaps
- Claude Code's documented handling of self-signed TLS certificates / private CAs for HTTP MCP servers was not found on the fetched portion of the docs page (the page was truncated at 100k of 151k characters). `NODE_EXTRA_CA_CERTS` is documented for `mcp-remote` only; whether Claude Code honours it is unverified.
- Whether Claude Code or Claude Desktop implement the Tasks extension client-side, or act on `notifications/resources/updated`, was not confirmed.
- The spec gives no size guidance for base64 image/audio/blob payloads; none was found.
- "SSH as stdio transport" is an inference from the stdio transport model; no Anthropic doc explicitly describing it was fetched.

---

## 3. Rust MCP SDK: `rmcp`, maturity, alternatives

### Takeaway
`rmcp` (official, `modelcontextprotocol/rust-sdk`) is at 3.11.0 (17 Nov 2026), Apache-2.0, with Streamable HTTP server, stdio, OAuth, elicitation and tool macros behind feature flags, and it tracks the 2026-07-28 revision; Tasks-extension support was not confirmed from its docs.

### Cited Findings
- `rmcp` 3.11.0 published 2026-11-17, Apache-2.0, ~55.2M total downloads (32.4M recent); `rmcp-macros` 3.11.0 same date — [crates.io/crates/rmcp](https://crates.io/crates/rmcp)
- Feature flags: `server` (default), `client`, `macros` (default; `#[tool]`/`#[prompt]`), `schemars`, `auth` (OAuth 2.0), `auth-enterprise-managed`, `elicitation`, `skills-fs` (SEP-2640 skills), `transport-io` (server stdio), `transport-child-process`, `transport-async-rw`, `transport-streamable-http-client(-reqwest)`, `transport-streamable-http-server` (`StreamableHttpService`); TLS via rustls by default — [docs.rs rmcp 3.11.0](https://docs.rs/crate/rmcp/3.11.0)
- The crate docs link the 2026-07-28 specification — [docs.rs rmcp 3.11.0](https://docs.rs/crate/rmcp/3.11.0)
- Release notes for 3.3-3.5 (Sep-Oct 2026) mention `ServerHandler::negotiate_initialize`, `ServerConfig`/`ClientConfig`, discover lifecycle fixes, SEP-2243 header handling, Origin validation enforcement, default cache hints for 2026-07-28 peers, and `LATEST` / `LATEST_WITH_INITIALIZE` constants — [rust-sdk releases](https://github.com/modelcontextprotocol/rust-sdk/releases)
- At the 28 Jul 2026 spec release the Rust SDK was described as supporting the new spec in beta, while TypeScript, Python, Go and C# were Tier 1 — [MCP blog](https://blog.modelcontextprotocol.io/posts/2026-07-28/)
- Alternatives on crates.io: `rust-mcp-sdk` 2.0.0 (2026-08-27, MIT, ~356k downloads); `pmcp` 2.23.0 (2026-10-28, MIT, ~168k downloads) — [crates.io/crates/rust-mcp-sdk](https://crates.io/crates/rust-mcp-sdk), [crates.io/crates/pmcp](https://crates.io/crates/pmcp)
- HTTP stack companions: `axum` 0.8.9, `rustls` 0.23.46, `rcgen` 0.14.10 (certificate generation), `tokio` 1.54.1 — [crates.io/crates/axum](https://crates.io/crates/axum), [crates.io/crates/rustls](https://crates.io/crates/rustls), [crates.io/crates/rcgen](https://crates.io/crates/rcgen), [crates.io/crates/tokio](https://crates.io/crates/tokio)

### Inferences
- `rmcp` is the only sensible default: two orders of magnitude more downloads than alternatives, official repo, weekly-ish releases through autumn 2026. The fast cadence (3.3 to 3.11 in about ten weeks) means pin an exact version and budget for upgrades.
- Constants like `LATEST_WITH_INITIALIZE` and `negotiate_initialize` suggest rmcp serves both the stateless 2026-07-28 era and legacy `initialize` clients from one server, which is what a bench needs while clients migrate.
- OAuth via `auth` is available but unnecessary for an offline LAN; a tower/axum middleware checking a bearer token (or mTLS) in front of `StreamableHttpService` is simpler.

### Gaps
- Tasks-extension support, MRTR (`input_required`) server ergonomics, `subscriptions/listen` support, and `outputSchema`/structured-output macro support in rmcp 3.11 were not confirmed; the docs.rs summary does not mention tasks. Check the repo README and examples before committing to tasks for flashing.
- Discrepancy: the GitHub releases page fetched showed 3.5.1 (5 Oct) as "latest" while crates.io and docs.rs show 3.11.0 (17 Nov 2026). Probably a stale/partial releases page; crates.io is taken as authoritative.
- Rmcp's SDK tier as of November 2026 was not found.
- No verified list of production Rust MCP servers was obtained. One lead only: `codanna` ships a doc named `mcp-https-self-signed.md` on docs.rs (seen in search results, not read).

---

## 4. Rust on the Linux supervisor (Pi-class ARM64): crates and a driver/plugin model

### Takeaway
Every interface in the brief has a maintained Rust crate except VISA/USBTMC and sigrok, which should be handled by raw SCPI over serial/TCP and by wrapping `sigrok-cli`. Several commonly cited crates are stale and have live successors (`rusb`→`nusb`, `gpio-cdev`→`gpiocdev`, `cross`→`cargo-zigbuild`, `v4l`→`nokhwa`/GStreamer).

### Cited Findings (versions and last-publish dates from crates.io, 25 Nov 2026)
- Async runtime: `tokio` 1.54.1 (2026-10-21) — [crates.io](https://crates.io/crates/tokio)
- Serial: `serialport` 4.10.1 (2026-09-08, MPL-2.0); `tokio-serial` 5.5.0 (2026-06-15) — [serialport](https://crates.io/crates/serialport), [tokio-serial](https://crates.io/crates/tokio-serial)
- Modbus: `tokio-modbus` 0.17.0 (2025-10-22) — [crates.io](https://crates.io/crates/tokio-modbus)
- CAN: `socketcan` 4.0.0 (2026-09-05); `embedded-can` 0.4.1 (2022) — [socketcan](https://crates.io/crates/socketcan), [embedded-can](https://crates.io/crates/embedded-can)
- BLE: `btleplug` 0.13.5 (2026-10-10, cross-platform); `bluer` 0.17.4 (last published 2025-06-06, BlueZ-specific) — [btleplug](https://crates.io/crates/btleplug), [bluer](https://crates.io/crates/bluer)
- USB: `nusb` 0.2.9 (2026-10-24); `rusb` 0.9.4 (last published 2024-04-27) — [nusb](https://crates.io/crates/nusb), [rusb](https://crates.io/crates/rusb)
- SDR: `soapysdr` 0.5.1 (2026-05-05); `rtl-sdr-rs` 0.4.3 (2026-10-27); `futuresdr` 0.10.0 (2026-10-20); `seify` 0.26.2 (2026-10-26) — [soapysdr](https://crates.io/crates/soapysdr), [rtl-sdr-rs](https://crates.io/crates/rtl-sdr-rs), [futuresdr](https://crates.io/crates/futuresdr), [seify](https://crates.io/crates/seify)
- Cameras: `v4l` 0.14.0 (last published 2023-05-12); `nokhwa` 0.10.11 (2026-05-15) — [v4l](https://crates.io/crates/v4l), [nokhwa](https://crates.io/crates/nokhwa)
- Audio: `cpal` 0.18.2 (2026-08-16); `alsa` 0.12.1 (2026-07-31) — [cpal](https://crates.io/crates/cpal), [alsa](https://crates.io/crates/alsa)
- Media pipelines: `gstreamer` 0.25.4 (2026-09-21) — [crates.io](https://crates.io/crates/gstreamer)
- Linux HAL / sensors / GPIO: `linux-embedded-hal` 0.5.0 (2026-08-24); `industrial-io` 0.6.1 (2025-09-13); `gpiocdev` 0.8.0 (2025-11-19); `gpio-cdev` 0.6.0 (last published 2023-09-12) — [linux-embedded-hal](https://crates.io/crates/linux-embedded-hal), [industrial-io](https://crates.io/crates/industrial-io), [gpiocdev](https://crates.io/crates/gpiocdev), [gpio-cdev](https://crates.io/crates/gpio-cdev)
- D-Bus (systemd, NetworkManager, BlueZ): `zbus` 6.5.0 (2026-11-20) — [crates.io](https://crates.io/crates/zbus)
- SCPI/VISA: `scpi` 1.0.1 (last published 2024-05-15); `visa-rs` 0.7.1 (2026-09-22); `usbtmc` 0.1.0 (last published 2021); no crate named `easy-scpi` exists — [scpi](https://crates.io/crates/scpi), [visa-rs](https://crates.io/crates/visa-rs), [usbtmc](https://crates.io/crates/usbtmc)
- sigrok: the `sigrok` crate is 0.3.1 from 2016, GPL-3.0, ~7k downloads; no `libsigrok` crate exists — [crates.io/crates/sigrok](https://crates.io/crates/sigrok)
- Plugins: `wasmtime` 51.0.0 (2026-11-20); `extism` 1.41.0 (2026-10-09) — [wasmtime](https://crates.io/crates/wasmtime), [extism](https://crates.io/crates/extism)
- DFU from the host: `dfu-nusb` 0.2.0 (2026-05-10); `dfu-libusb` 0.6.2 (2026-06-19) — [dfu-nusb](https://crates.io/crates/dfu-nusb), [dfu-libusb](https://crates.io/crates/dfu-libusb)
- Crypto/audit primitives: `blake3` 1.8.7, `ed25519-dalek` 3.0.0 — [blake3](https://crates.io/crates/blake3), [ed25519-dalek](https://crates.io/crates/ed25519-dalek)

### Inferences
- Recommended picks: `tokio` + `tokio-serial`; `tokio-modbus`; `socketcan` (async feature); `nusb` (pure Rust, no libusb to cross-compile) over `rusb`; `gpiocdev` over `gpio-cdev`; `zbus` for NetworkManager/systemd/BlueZ control; `btleplug` if the same code should also run on the MacBook, `bluer` if peripheral/GATT-server roles on the Pi are needed (note its last publish is 17 months old); `seify`/`futuresdr` for SDR with `soapysdr` as the hardware escape hatch; GStreamer for cameras and multichannel audio where pipelines matter, `cpal`/`alsa` for simple capture.
- SCPI: skip VISA. Speak SCPI directly over TCP sockets (port 5025), serial, or USBTMC via `nusb` in a ~300-line transport crate; that also makes a mock trivial. `visa-rs` requires a vendor VISA runtime (background knowledge, unverified here).
- sigrok: wrap `sigrok-cli` as a subprocess driver; this also keeps GPL-3 code out of the Apache/MIT process.
- Driver model: three tiers. (1) In-tree Rust `trait Instrument` drivers for anything timing-sensitive or needing raw device access (USB, CAN, SWD). (2) Subprocess drivers speaking a small JSON-RPC/postcard protocol over stdio for foreign tools (`sigrok-cli`, `probe-rs`, `espflash`, vendor CLIs) and for crash isolation; this is also the tier an agent can rewrite and hot-swap safely. (3) WASM components (wasmtime/WASI 0.2) only for pure protocol logic (SCPI dialects, decoders), where host-provided "transport" imports give real capability sandboxing; WASI has no standard USB/serial/CAN, so making every driver WASM would mean writing custom host interfaces for everything.
- Every driver should be self-describing: `describe()` (capabilities, limits with units, which operations are read-only/idempotent/destructive), `health()`, `self_test()`, `safe_state()`, and a `state_version()`; MCP tool schemas, annotations and MWS declaration entries are generated from that one description so they cannot drift.
- Because MWS proxy-assisted tests need an interposable hop, define a `Transport` trait (serial/TCP/USB/CAN/mock) with a fault-injection wrapper (hold, fail, cut the Nth write) that is compiled in for test builds and disclosed as a cooperative hook.

### Gaps
- No maintained, pure-Rust USBTMC or VISA-free SCPI client crate was identified; `scpi` 1.0.1 appears to be a device-side parser (background knowledge, not verified) and is 18 months stale.
- aarch64 build/runtime status of each crate on Raspberry Pi OS was not individually verified.
- WASI support for USB/serial/CAN host interfaces was not researched; the statement above is background knowledge.

---

## 5. Data plane: transport, time-series storage on a Pi, and LLM-friendly telemetry

### Takeaway
A Rust-native bus (Zenoh) plus embedded storage (SQLite for state/audit, Parquet/Arrow or DuckDB for samples) fits a single Pi better than a database server; the binding constraint for the agent is Claude Code's 10k/25k-token tool-output limits, so the server must return summaries and links, not samples.

### Cited Findings
- Zenoh is a pub/sub/query protocol unifying data in motion, data at rest and computations; peer-to-peer, brokered or hybrid topologies chosen at runtime; wire overhead about 4-6 bytes; designed to run down to microcontrollers; Eclipse incubating project — [zenoh.io](https://zenoh.io/docs/overview/what-is-zenoh/)
- `zenoh` crate 1.10.2 (2026-10-28), EPL-2.0 OR Apache-2.0 — [crates.io/crates/zenoh](https://crates.io/crates/zenoh)
- MQTT in Rust: `rumqttd` 0.20.0 (broker; last published 2025-09-29), `rumqttc` 0.25.1 (client; 2025-11-21) — [rumqttd](https://crates.io/crates/rumqttd), [rumqttc](https://crates.io/crates/rumqttc)
- NATS client: `async-nats` 0.53.0 (2026-11-17) — [crates.io](https://crates.io/crates/async-nats)
- Embedded storage/analytics crates: `rusqlite` 0.43.0 (2026-11-15); `duckdb` 1.20001.0 (2026-11-20); `arrow` and `parquet` 60.2.0 (2026-11-21); `polars` 0.55.2 (2026-08-06) — [rusqlite](https://crates.io/crates/rusqlite), [duckdb](https://crates.io/crates/duckdb), [arrow](https://crates.io/crates/arrow), [parquet](https://crates.io/crates/parquet), [polars](https://crates.io/crates/polars)
- InfluxDB 3 Core is the open-source InfluxDB 3: persists Parquet, runs on object storage or local disk with no dependencies, has an embedded Python VM for plugins/triggers, supports SQL and InfluxQL and v1/v2 write APIs; Enterprise adds historical query capability, HA and read replicas; docs banner announces InfluxDB 3.12 — [InfluxDB 3 Core docs](https://docs.influxdata.com/influxdb3/core/)
- Tracing/metrics: `opentelemetry` 0.33.1 (2026-10-09); MCP documents OTel trace-context propagation in `_meta` — [crates.io](https://crates.io/crates/opentelemetry), [MCP changelog](https://modelcontextprotocol.io/specification/2026-07-28/changelog)
- Plot rendering: `plotters` 0.3.7 (last published 2024-09-08, ~285M downloads) — [crates.io](https://crates.io/crates/plotters)
- Agent-side budget: Claude Code warns at 10,000 tokens per tool output and caps at 25,000 by default — [Claude Code MCP docs](https://code.claude.com/docs/en/mcp)
- MCP tool results can carry base64 images and `resource_link`s — [Tools 2026-07-28](https://modelcontextprotocol.io/specification/2026-07-28/server/tools)

### Inferences
- Bus: Zenoh is the natural choice for an all-Rust system (same library on Pi and host, queryable storage, tiny overhead) and can reach MCUs later. MQTT via `rumqttd` is the pragmatic choice if ESPHome/Meshtastic-class nodes that already speak MQTT are in scope; `rumqttd`'s last release is 14 months old. NATS adds a non-Rust server binary for little gain on one box.
- Storage: SQLite (WAL) for bench state, idempotency-key records, object versions and the audit chain (these need transactions and fsync semantics); append-only Parquet segments (Arrow writer) for sample data, queried with DuckDB in-process. This avoids running a DB daemon on a Pi, keeps data as portable files, and lets the agent ask SQL questions cheaply. InfluxDB 3 Core is the closest "server" equivalent (same Parquet substrate) but adds a Python VM and has unverified long-range-query limits.
- LLM-friendly telemetry pattern: a `query_telemetry` tool returns (a) per-channel stats over the window (n, min, max, mean, std, p05/p50/p95, first/last, slope), (b) LTTB/min-max downsample capped at N points (N around 200), (c) anomaly windows (threshold/z-score/change-point) with timestamps, (d) an optional small PNG plot as image content, and (e) a `resource_link` + SQL handle for the raw data. Units and sample rate go in the schema, not the prose. Make token cost predictable by letting the caller pass `max_points`.
- For IQ/audio/logic captures: store on device, return capture id, duration, sample rate, size, SHA-256, a spectrogram/waveform PNG, and decoded summaries; never base64 the capture into context.

### Gaps
- No sources were fetched for QuestDB, VictoriaMetrics or TimescaleDB on ARM64/Pi; no comparison data.
- InfluxDB 3 Core's exact query-window or compaction limits and licence were not confirmed from the page fetched.
- No published prior art specifically on "LLM-friendly telemetry" summaries was found or fetched; the pattern above is the researcher's design, not a cited practice.
- The `duckdb` crate's unusual version string (1.20001.0) was not mapped to an upstream DuckDB release.

---

## 6. Embedded Rust for nodes and instruments; the Pi as a permanent debug probe

### Takeaway
Embassy + esp-hal 1.x + probe-rs is a coherent 2026 stack for ESP32, RP2040/RP2350 and nRF52, and probe-rs is both a CLI and a Rust library, so the supervisor can flash, reset, read memory and stream RTT/defmt for every node programmatically; probe-rs also has a Linux-host SWD backend that can bit-bang SWD from the Pi's own GPIO.

### Cited Findings
- Embassy crates: `embassy-executor` 0.10.0, `embassy-rp` 0.10.0, `embassy-stm32` 0.6.0, `embassy-boot` 0.7.0, `embassy-usb` 0.6.0 (all 2026-03-20); `embassy-nrf` 0.11.0 (2026-06-16) — [embassy-executor](https://crates.io/crates/embassy-executor), [embassy-rp](https://crates.io/crates/embassy-rp), [embassy-nrf](https://crates.io/crates/embassy-nrf), [embassy-boot](https://crates.io/crates/embassy-boot), [embassy-usb](https://crates.io/crates/embassy-usb)
- `esp-hal` 1.2.2 (2026-09-18): bare-metal `no_std` HAL for ESP32, C2, C3, C5, C6, C61, H2, P4, S2, S3; 1.x has an `unstable` API tier that can break in minor releases; dual MIT/Apache-2.0; the repo carries `hil-test` directories and a HIL CI badge — [esp-hal README](https://github.com/esp-rs/esp-hal), [crates.io/crates/esp-hal](https://crates.io/crates/esp-hal)
- ESP radio/RTOS crates: `esp-radio` 0.18.0 stable with 1.0.0-beta.1 published; `esp-rtos` 0.4.0 (2026-08-26); `esp-hal-embassy` 0.9.1 last published 2025-10-14 — [esp-radio](https://crates.io/crates/esp-radio), [esp-rtos](https://crates.io/crates/esp-rtos), [esp-hal-embassy](https://crates.io/crates/esp-hal-embassy)
- std-on-ESP-IDF path: `esp-idf-svc` 0.54.1 (2026-11-25) — [crates.io](https://crates.io/crates/esp-idf-svc)
- `espflash` 4.7.0 (2026-10-29) — [crates.io](https://crates.io/crates/espflash)
- BLE: `trouble-host` 0.8.0 (2026-08-25); `nrf-sdc` 0.4.0 (2026-08-21); `nrf-softdevice` 0.1.0 last published 2024-01-15 — [trouble-host](https://crates.io/crates/trouble-host), [nrf-sdc](https://crates.io/crates/nrf-sdc), [nrf-softdevice](https://crates.io/crates/nrf-softdevice)
- Logging: `defmt` 1.2.0, `defmt-rtt` 1.3.1 (2026-10-15) — [defmt](https://crates.io/crates/defmt), [defmt-rtt](https://crates.io/crates/defmt-rtt)
- Host-device RPC: `postcard` 1.2.0 (2026-11-21); `postcard-rpc` 0.12.1 (2025-12-04); `ergot` 0.12.0 (2025-11-10) — [postcard](https://crates.io/crates/postcard), [postcard-rpc](https://crates.io/crates/postcard-rpc), [ergot](https://crates.io/crates/ergot)
- `probe-rs` 0.32.0 and `probe-rs-tools` 0.32.0 (2026-07-22) — [crates.io/crates/probe-rs](https://crates.io/crates/probe-rs)
- probe-rs supports ARM, RISC-V and Xtensa cores over SWD or JTAG; probes: DAPLink (CMSIS-DAP), ST-Link, J-Link, FTDI, ESP32 USB-JTAG, WLink, Blackmagic; it is usable as a library (list probes, attach, halt, read/write memory, flash ELF/BIN/IHEX) and offers `probe-rs run` (flash, reset, RTT/defmt, panics), `cargo-flash`, `cargo-embed`, GDB and VS Code DAP debugging — [probe-rs README](https://cdn.jsdelivr.net/gh/probe-rs/probe-rs@master/README.md)
- probe-rs has a `probe-rs-linux` plugin that lets a Linux host act as an SWD probe, with `linuxgpiod` and `linuxspidevswd` backends, and its README has a "remote-server usage" section — [probe-rs README](https://cdn.jsdelivr.net/gh/probe-rs/probe-rs@master/README.md)
- `embedded-test` 0.7.3 (2026-10-30): on-target test harness for ARM, RISC-V, Xtensa; `probe-rs run` acts as a libtest-compatible runner; all tests flashed once, device reset before each test, selection and pass/fail via semihosting; supports `#[init]`, async tests with embassy, `#[should_panic]`, `#[ignore]`, `#[timeout]` (default 60 s) — [embedded-test README](https://github.com/probe-rs/embedded-test), [crates.io](https://crates.io/crates/embedded-test)
- Raspberry Pi Debug Probe: RP2040-based, USB to SWD plus USB to UART, CMSIS-DAP compatible, 3.3 V nominal I/O, ships with case, USB cable and three debug cables, in production until at least January 2030 — [raspberrypi.com Debug Probe](https://www.raspberrypi.com/products/debug-probe/)
- Raspberry Pi announced the Debug Probe as a "plug-and-play debug kit for $12" — [Raspberry Pi news](https://www.raspberrypi.com/news/raspberry-pi-debug-probe-a-plug-and-play-debug-kit-for-12/)
- OTA/bootloader crates: `embassy-boot` 0.7.0; DFU host crates `dfu-nusb` 0.2.0 — [embassy-boot](https://crates.io/crates/embassy-boot), [dfu-nusb](https://crates.io/crates/dfu-nusb)
- `embedded-hal` is 1.0.0 (2024-01-09); `embedded-hal-mock` 0.11.1 (2024-06-02) — [embedded-hal](https://crates.io/crates/embedded-hal), [embedded-hal-mock](https://crates.io/crates/embedded-hal-mock)

### Inferences
- "Pi as a permanent debug probe for every node": two workable shapes. (a) One cheap CMSIS-DAP probe per node on a powered USB hub (a Pico running debugprobe firmware works the same as the boxed product), selected by probe serial; simplest and electrically robust. (b) `probe-rs-linux` bit-banging SWD from Pi GPIO/SPI: zero extra hardware but a few GPIOs per node and slower. ESP32-C3/C6/S3 need no probe at all: their built-in USB-JTAG is a probe-rs-supported probe, and `espflash` covers UART flashing.
- Link `probe-rs` as a library inside a dedicated `benchd-probe` subprocess (crash isolation, exclusive USB access) rather than shelling out per call; expose `flash`, `reset`, `halt/resume`, `read_mem`, `rtt_attach` (defmt decoded with the ELF), and a GDB/DAP port for deep sessions. All of those except `read_mem` on a halted-safe target are writes in MWS terms (attach can halt a core), so annotate cautiously.
- Firmware identity/attestation: embed git SHA, dirty flag, build timestamp, target, and a BLAKE3 of the image in a fixed linker section and return it over postcard-rpc (`get_build_info`) and RTT; keep the supervisor's "what should be running" record as {ELF hash, source commit, toolchain, GitHub attestation id}. After any flash, RES-1-style truth comes from the node's own report plus a probe read-back/verify, not from the flasher's exit code.
- OTA: `embassy-boot` A/B swap with rollback for nRF/RP/STM32 is the closest thing to an all-or-nothing flash (helps ATOM-2); keep SWD as the unbrick path so the agent can always recover.
- BLE: use TrouBLE (`trouble-host`) with `nrf-sdc` on nRF52 and with `esp-radio` on ESP32-C6; `nrf-softdevice` has not had a crates.io release since January 2024.
- `postcard-rpc` is the leanest way to give each node a typed, schema-versioned RPC that the supervisor can reflect into MCP tools; its last release is about a year old, so check the repo for activity before depending on it.

### Gaps
- Current retail price of the Debug Probe and of generic CMSIS-DAP clones was not verified (only the $12 launch price from the 2023 announcement title).
- Per-chip maturity details (ESP32-C6 Wi-Fi 6/802.15.4, RP2350 support level in `embassy-rp` 0.10, embassy-nrf on nRF54) were not verified beyond version numbers.
- The `probe-rs-linux` plugin's README (wiring, speed, remote-server mode details) was not read; the probe.rs "Raspberry Pi" knowledge-base page turned out to be about debugging a Pi 4B over JTAG, not using a Pi as a probe.
- Whether `embedded-test` runs under QEMU/Renode via semihosting without a probe was not confirmed.
- Search-result titles suggest probe-rs has a Glasgow probe driver (`src/probe/glasgow/mod.rs` in 0.31.0); not read.

---

## 7. Existing instruments with open firmware an agent could modify

### Takeaway
Three candidates were verified: IronOS (Pinecil and others; GPL-3.0; CI-built dev firmware), Bus Pirate 5/6 (MIT; RP2040/RP2350; CMake/pico-sdk with Docker Compose and Nix; CI artifacts), and Flipper Zero via `flipperzero-rs` (MIT; Rust `no_std` external apps). The rest of the brief's list was not verified in this pass.

### Cited Findings
- IronOS supports Pinecil V1/V2, Miniware TS100/TS101/TS80/TS80P/MHP30 and several Sequre models; community code GPL-3.0; a root `Makefile`; development builds come from the latest successful GitHub Actions run on `dev`; stable releases on GitHub Releases (README names v2.22 stable while linking v2.23 assets) — [IronOS README](https://github.com/Ralim/IronOS)
- Bus Pirate 5 firmware: RP2040 (BP5) and RP2350 (BP5 XL, BP6); MIT with some LGPL3/BSD parts; CMake + Raspberry Pi Pico SDK; Docker Compose and a Nix flake provided; a GitHub Actions "CMake" workflow builds `.uf2` artifacts in about 4-7 minutes; protocols include 1-Wire, I2C, SPI, UART, MIDI, serial LEDs — [BusPirate5-firmware README](https://github.com/DangerousPrototypes/BusPirate5-firmware)
- `flipperzero-rs`: crates `flipperzero`, `flipperzero-sys`, `flipperzero-rt`, `flipperzero-alloc`; builds without the firmware toolchain; `no_std` (`core` + `alloc`) on `thumbv7em-none-eabihf`; apps are external apps on the SD card; supports SDK 88.2 (firmware 1.5.1) in crate 0.17.x; MIT — [flipperzero-rs README](https://github.com/flipperzero-rs/flipperzero)
- probe-rs supports ESP32 USB-JTAG, CMSIS-DAP and Blackmagic probes, relevant to which instruments the Pi can reflash/debug without vendor tools — [probe-rs README](https://cdn.jsdelivr.net/gh/probe-rs/probe-rs@master/README.md)

### Inferences
- "Reproducible in CI" in practice means: pinned toolchain in a container or Nix flake, no network fetches at build time beyond pinned inputs, and a CI workflow that already emits artifacts. By that test Bus Pirate 5 (Nix flake + Docker Compose + CI) is the strongest verified candidate; IronOS (Makefile + CI artifacts per device/language) is close. Neither claims bit-for-bit reproducibility in the READMEs read.
- Bus Pirate 5 is the best "agent-modifiable instrument" for a first bench: RP2040/RP2350 (SWD via the Debug Probe, UF2 fallback), MIT licence, and it is itself a protocol tool. The Pinecil V2 is a good second: real power electronics with safety-relevant firmware, good for exercising interlocks.
- For C/C++ instrument firmware, the Rust project should treat each as a pinned git submodule built in a container job, attested, and flashed by the same supervisor path as Rust nodes.

### Gaps
- Not verified in this pass (no source fetched): Glasgow, HackRF / PortaPack Mayhem, Meshtastic, ESPHome, candleLight, tinySA, NanoVNA, OpenHantek, sigrok fx2lafw, WiCAN: licences, build systems, CI and reproducibility. Background knowledge (unverified, do not cite): Glasgow is Python/Amaranth gateware with FX2 firmware; Mayhem, Meshtastic (PlatformIO), NanoVNA (ChibiOS) and WiCAN (ESP-IDF) are GPL-family; candleLight is small STM32 C firmware; ESPHome generates firmware from YAML.
- Whether any of these projects publish bit-for-bit reproducible builds was not established.
- Rust-on-Flipper covers apps only; modifying Flipper's own firmware remains C.

---

## 8. CI and free services: GitHub Actions, simulation, hardware-in-the-loop, hosting

### Takeaway
For a public repo, standard GitHub-hosted runners (including native ARM64 Linux) are free and unlimited, which covers building the aarch64 supervisor natively, all firmware, simulation and MWS checks; real-hardware HIL should not run on a self-hosted runner attached to the public repo, because GitHub itself says such runners "should almost never be used for public repositories".

### Cited Findings

**GitHub Actions**
- "Use of the standard GitHub-hosted runners is free and unlimited on public repositories." Standard public-repo runners: `ubuntu-24.04` (x64, 4 CPU, 16 GB RAM, 14 GB SSD); `ubuntu-24.04-arm` / `ubuntu-22.04-arm` (arm64, 4 CPU, 16 GB, 14 GB); `windows-11-arm`; `macos-latest` (arm64, 3 CPU, 7 GB) — [GitHub-hosted runners reference](https://docs.github.com/en/actions/reference/runners/github-hosted-runners)
- Larger runners are for organizations/enterprises on Team or Enterprise Cloud plans — [GitHub-hosted runners reference](https://docs.github.com/en/actions/reference/runners/github-hosted-runners)
- 2026 pricing: hosted-runner rates cut by up to 39% from 1 Jan 2026; a $0.002/min cloud platform charge was announced (self-hosted from 1 Mar 2026) but the self-hosted change was postponed; usage on public repositories remains free and is not affected — [GitHub: 2026 pricing changes for Actions](https://github.com/resources/insights/2026-pricing-changes-for-github-actions)
- Caches: 10 GB default per repository; entries unused for 7 days are evicted; beyond 10 GB is billed only if opted in; rate limits 200 uploads / 1,500 downloads per minute — [GitHub dependency caching reference](https://docs.github.com/en/actions/reference/workflows-and-actions/dependency-caching)
- Artifact attestations: signed build provenance (workflow, repo, commit, trigger) and optional SBOM; SLSA v1.0 Build Level 2 by default and Level 3 with reusable workflows; public repos use the Sigstore Public Good Instance with a public transparency log; verified with `gh attestation verify` — [GitHub artifact attestations](https://docs.github.com/en/actions/concepts/security/artifact-attestations)

**Self-hosted runner risk and mitigations**
- GitHub's hardening guide: self-hosted runners "should almost never be used for public repositories" because anyone can open a pull request and compromise the environment; self-hosted runners can be persistently compromised; required-review environments do not isolate jobs on the runner; ephemeral/just-in-time runners run at most one job but reused hardware can still leak state; use runner groups to limit scope — [GitHub secure use reference](https://docs.github.com/en/actions/reference/security/secure-use)
- Ferrous Systems' HIL pattern (Mar 2021): Raspberry Pi 4 runner + nRF52840-DK; the self-hosted runner lives in a private repo; the public repo pushes approved code to the private repo's branch via a deploy key and triggers a `workflow_dispatch` HIL workflow whose logs are proxied back; only reviewer-approved code (bors `staging`) reaches the runner; tools were `defmt-test`, `probe-run`, `flip-link` — [Ferrous Systems: GHA HIL tests](https://ferrous-systems.com/blog/gha-hil-tests/)
- esp-hal maintains `hil-test` suites and a HIL CI badge in its public repo — [esp-hal README](https://github.com/esp-rs/esp-hal)

**Simulation**
- Wokwi CI runs simulations in the cloud via `wokwi-cli`, checks serial output, supports scripted automation scenarios; quotas per rolling 30 days: Free 50 min, Hobby 200, Hobby+ 500, Pro 2000; per-session wall-clock limit 5 min on Free — [Wokwi CI docs](https://docs.wokwi.com/wokwi-ci/getting-started)
- Wokwi plans (annual billing): Community €0 (open source), Hobby €5.6/mo, Hobby+ €8.1/mo, Pro €20/seat/mo — [Wokwi pricing](https://wokwi.com/pricing)
- `antmicro/renode-test-action` (Apache-2.0; usage shown as `@v5`) runs Robot Framework tests in Renode with inputs `renode-revision`, `tests-to-run`, `artifacts-path` — [renode-test-action](https://github.com/antmicro/renode-test-action)
- `embedded-test` drives on-target tests through `probe-rs run` with libtest-compatible output — [embedded-test README](https://github.com/probe-rs/embedded-test)

**Cross-compilation, fuzzing, release tooling**
- `cargo-zigbuild` 0.24.1 (2026-11-04); `cross` 0.2.5 last published to crates.io 2023-02-04 — [cargo-zigbuild](https://crates.io/crates/cargo-zigbuild), [cross](https://crates.io/crates/cross)
- `cargo-fuzz` 0.13.2 (2026-06-09) — [crates.io](https://crates.io/crates/cargo-fuzz)
- OSS-Fuzz accepts projects that "have a significant user base and/or be critical to the global IT infrastructure"; has a Rust integration guide — [OSS-Fuzz: accepting new projects](https://google.github.io/oss-fuzz/getting-started/accepting-new-projects/)
- `cargo-dist` 0.34.0 (2026-11-02); `cargo-binstall` 1.28.0 (2026-11-21) — [cargo-dist](https://crates.io/crates/cargo-dist), [cargo-binstall](https://crates.io/crates/cargo-binstall)

**MWS in CI**
- `mcp-writecheck-action`: `plan` on every PR (no writes, no secrets), `catalog` optionally, `full` on main against a disposable backend the job starts; needs Node 22; pin a full commit SHA — [mcp-writecheck-action README](https://github.com/isokron-ai/mcp-writecheck-action)
- `isokron-ai/mcp-check@v1` grades tool definitions from source without running the server — [isokron.ai/mcp-check](https://isokron.ai/mcp-check)

### Inferences
- CI design, all on free hosted runners:
  1. `ubuntu-24.04`: fmt, clippy, `cargo deny`, unit tests; firmware builds for `thumbv6m`/`thumbv8m` (RP), `thumbv7em` (nRF52), `riscv32imc/imac` (ESP32-C3/C6) and Xtensa (S3, via the esp toolchain); host tests of `no_std` protocol crates with `embedded-hal-mock`.
  2. `ubuntu-24.04-arm`: build and test the supervisor natively for aarch64 (no cross needed); produce the release binary/deb and attest it. Keep `cargo-zigbuild` only for musl/static or old-glibc targets.
  3. "Simulated bench" integration job: bring up `vcan0` (kernel module availability on hosted runners must be checked), a Modbus TCP simulator, a mock SCPI TCP instrument, an ELM327 emulator on a pty, and emulated nodes; start the supervisor against them; run MCP integration tests and `mcp-writecheck` `full` with the simulators as the disposable backend.
  4. Firmware-in-simulation: Renode Robot tests for nRF52/RP-class firmware; Wokwi CI for ESP32 smoke tests, budgeted tightly (50 free minutes per 30 days is about ten 5-minute sessions).
  5. Nightly: `cargo-fuzz` on parsers (SCPI responses, postcard frames, CAN/Modbus decoders) with a time box; apply to OSS-Fuzz only once the project has users.
  6. Release: tag-driven; `actions/attest-build-provenance` on supervisor binaries and every firmware image; firmware ELF hash recorded so the bench can verify "what is running" against an attestation.
- HIL: run it on the bench Pi but never as a runner registered to the public repo for PR events. Options in order of safety: (a) Ferrous pattern, private mirror repo with the runner, triggered only after maintainer approval/merge, logs proxied back (note the owner's rule that private repos get no GitHub-hosted CI is compatible, since the job runs on the self-hosted Pi and postponed platform charges do not currently apply); (b) a pull model with no runner at all: the bench polls for attested release artifacts from `main`, verifies the attestation, flashes, runs `embedded-test`, and posts a signed result as a commit status/check via a narrowly scoped token. (b) removes inbound code execution from PRs entirely and doubles as the agent's own "rebuild and reflash" path.
- If a runner is used at all: ephemeral/JIT registration per job, dedicated SD image or container reset between jobs, no long-lived secrets on the Pi, runner on its own VLAN, `workflow_dispatch`/`push`-to-protected-branch triggers only, and a hardware interlock so CI jobs cannot energize loads.
- Docs on GitHub Pages with mdBook; crates on crates.io; container images and firmware bundles on GHCR/Releases. KiCad/KiBot for PCB outputs and OpenSCAD/CadQuery/build123d for printable parts fit the same hosted-runner model.

### Gaps
- Concurrency limits for free-plan public repos, artifact retention defaults and artifact size limits were not verified (the cache page does not cover them).
- GA status of ARM64 hosted runners is not stated on the reference page (they are listed as standard runners).
- Whether `vcan`/`can-gw` kernel modules load on GitHub-hosted Ubuntu runners was not verified.
- Wokwi's supported-chip list and whether the free Community plan's CI minutes differ for open source were not confirmed; Espressif's QEMU fork and QEMU Cortex-M options were not researched.
- Not researched (no sources fetched): Interrupt/Memfault HIL posts, ELM327-emulator, specific Modbus/SCPI simulators, Codecov, Depot or other OSS plans, Cloudflare Pages/R2 free tiers, Hackaday.io/Printables, KiBot, mdBook details. Treat mentions above as unverified suggestions.
- Whether the postponed self-hosted platform charge will return, and when, is unknown; GitHub gave no new date.

---

## 9. Security model for an agent with physical write access over Wi-Fi

### Takeaway
MWS deliberately excludes authentication and authorization, and MCP makes authorization optional, so the bench's security model is entirely the project's own design; the cited sources supply building blocks (Origin validation, per-request credentials, scope-filtered tool lists, rate limiting, read-only mode semantics, signed result ledgers) but no prior art for hardware interlocks was found.

### Cited Findings
- MWS out of scope: "authentication, authorization and transport security"; the builder skill warns not to treat MWS as a security review — [spec/mws.md §2](https://github.com/isokron-ai/mcp-write-safety/blob/main/spec/mws.md), [builder SKILL.md](https://github.com/isokron-ai/mcp-write-safety/blob/main/integrations/skills/mcp-write-safe-builder/SKILL.md)
- MWS does cover safety settings: read-only mode must refuse every write tool; unknown setting values must not leave writes enabled; bad deny-lists must stop the server or disable writes (CFG-1 to CFG-4) — [NORMATIVE-SENTENCES.md](https://github.com/isokron-ai/mcp-write-safety/blob/main/spec/NORMATIVE-SENTENCES.md)
- MCP: servers MUST validate `Origin`, SHOULD bind to localhost when local, SHOULD authenticate all connections — [Streamable HTTP 2026-07-28](https://modelcontextprotocol.io/specification/2026-07-28/basic/transports/streamable-http)
- MCP: servers MUST validate inputs, implement access controls, rate limit tool invocations, sanitize outputs; clients SHOULD prompt for confirmation on sensitive operations and log tool usage — [Tools 2026-07-28](https://modelcontextprotocol.io/specification/2026-07-28/server/tools)
- MCP: the tool list MAY vary by the authorization presented on the request (scope-filtered catalogs) — [Tools 2026-07-28](https://modelcontextprotocol.io/specification/2026-07-28/server/tools)
- MCP authorization: optional; OAuth 2.1 with audience-bound tokens (RFC 8707), 403 `insufficient_scope` step-up; stdio uses environment credentials — [Authorization 2026-07-28](https://modelcontextprotocol.io/specification/2026-07-28/basic/authorization)
- Closing the response stream is a cancellation signal the server must honour — [Streamable HTTP 2026-07-28](https://modelcontextprotocol.io/specification/2026-07-28/basic/transports/streamable-http)
- MWS notes a candidate gap: a composite write interrupted by cancellation leaves part applied with no result delivered — [spec/mws.md §12](https://github.com/isokron-ai/mcp-write-safety/blob/main/spec/mws.md)
- Precedent for tamper-evident records: `mcp-writecheck` writes `ledger.jsonl`, `SHA256SUMS` and an optional `ssh-keygen -Y sign` signature per run — [writecheck README](https://github.com/isokron-ai/mcp-write-safety/blob/main/packages/writecheck/README.md)
- Claude Code can send a static `Authorization` header or run a `headersHelper` to mint headers per connection — [Claude Code MCP docs](https://code.claude.com/docs/en/mcp)

### Inferences
- Layered model:
  1. Network: bench is its own Wi-Fi AP or a dedicated SSID/VLAN; MCP endpoint bound to that interface only; TLS with a bench-local CA (`rcgen`) or SSH-tunnelled stdio; strict `Origin` allow-list.
  2. Authentication: per-client bearer tokens (hashed at rest) or mTLS; SSH keys for the stdio path. No OAuth server needed offline; keep the door open for RFC 9728 metadata later.
  3. Capability scoping: tokens carry scopes per tool class and per instrument (`read`, `actuate:lowpower`, `actuate:mains`, `rf:tx`, `flash:node/*`, `admin`); the catalog is filtered by scope (allowed by MCP) and each call re-checked; limits (max V/A/W, frequency bands, duty cycle) are part of the capability and mirrored into schemas (MWS IN-5).
  4. Arming: a physical arm switch read through GPIO and wired in series with the relay/load supply, so software cannot energize outputs when disarmed; switch state is a write-restricting setting (fail closed on unreadable state, per CFG-1). High-energy actions also need a short-lived arm token (a server-minted handle with stated lifetime) and, optionally, human confirmation through `input_required` (with no effect before the retry, per RETRY-3).
  5. Dead-man: an independent hardware watchdog (MCU or supervisor IC) that drops all relays if the supervisor stops petting it; the supervisor stops petting when the agent link heartbeat or an active task's lease expires. Stream close = cancellation = return to safe state, with the partial state reported truthfully on the next read.
  6. Rate limiting: token buckets per tool and per actuator (switching cycles/minute, flash erase counts, RF duty cycle), enforced below the MCP layer in the driver.
  7. Audit: append-only SQLite/JSONL log where each record includes the hash of the previous one (BLAKE3), periodically signed with an on-device Ed25519 key and exportable; record caller identity, tool, arguments (secrets redacted, per SEC-1), idempotency key, pre/post state versions, outcome, firmware hashes. This is also the durable key store RETRY-2 needs.
  8. Firmware trust: the agent may build and flash, but the bench only auto-flashes images whose hash matches a GitHub attestation or that were built locally in a recorded, hashed build; SWD recovery path always available.
- The self-modifying loop (agent rebuilds and reflashes the instruments it then trusts for read-back) is the main novel risk: keep at least one measurement path whose firmware the agent cannot change (e.g. an independent INA/ADC on the supervisor, or a locked reference meter) so RES-1 read-backs are not self-certified.

### Gaps
- No prior art (papers, products, open projects) on AI agents with physical write access, hardware arm switches for MCP servers, or lab-automation MCP safety was searched for or found in this pass; the model above is the researcher's design.
- No source was fetched on hash-chained audit logs for MCP servers beyond the writecheck ledger precedent.
- Threats from prompt injection via instrument output (e.g. text decoded from a bus or a display capture steering the agent) were not researched; MWS excludes them and MCP only says to sanitize outputs.

---

## 10. Proposed crate/workspace layout and the ten design decisions to make early (required by the brief; researcher's synthesis)

### Takeaway
One Cargo workspace split into host (`std`, aarch64/x86_64), shared `no_std` protocol crates, firmware crates per MCU family, and simulators; the earliest decisions are the ones MWS testing and safety depend on: object/version model, outcome reporting, idempotency storage, interposable transports, and where the hardware interlock sits.

### Cited Findings
- Constraints the layout must satisfy: rmcp features for stdio and Streamable HTTP server — [docs.rs rmcp 3.11.0](https://docs.rs/crate/rmcp/3.11.0); MWS write-result shape in `structuredContent.mws` — [write-result schema](https://github.com/isokron-ai/mcp-write-safety/blob/main/spec/shapes/write-result-0.1.schema.json); proxy-assisted tests need an interposable server-to-backend hop or disclosed cooperative hooks — [spec/mws.md §7, §12](https://github.com/isokron-ai/mcp-write-safety/blob/main/spec/mws.md); free native ARM64 hosted runners — [GitHub-hosted runners](https://docs.github.com/en/actions/reference/runners/github-hosted-runners); probe-rs usable as a library — [probe-rs README](https://cdn.jsdelivr.net/gh/probe-rs/probe-rs@master/README.md)

### Inferences

**Workspace layout (proposal)**
```
bench/
├─ Cargo.toml                  # workspace; host members default, firmware excluded from default-members
├─ crates/
│  ├─ bench-mws/               # write-result/0.1 types+builder, outcome enum, strict JSON-Schema validation,
│  │                           # idempotency-key store trait, object version/lock helpers (candidate upstream "Rust kit")
│  ├─ bench-core/              # Instrument/Driver traits, capability descriptor, units/limits types, safe-state model
│  ├─ bench-transport/         # Transport trait: serial, tcp, usb (nusb), socketcan, ble, mock + fault-injection wrapper
│  ├─ bench-store/             # SQLite: state versions, idempotency records, hash-chained audit log; Parquet segment writer
│  ├─ bench-telemetry/         # Zenoh/MQTT ingest, downsampling, stats, anomaly windows, PNG plots (LLM summaries)
│  ├─ bench-proto/             # no_std: postcard-rpc endpoint/topic schemas, build-info struct, shared by host+firmware
│  ├─ bench-mcp/               # rmcp server: tool generation from descriptors, annotations, MRTR confirm, tasks, resources
│  ├─ bench-auth/              # bearer/mTLS, scopes->catalog filter, rate limits, arm tokens
│  ├─ bench-interlock/         # GPIO arm switch, watchdog petting, relay safe-state (gpiocdev)
│  ├─ bench-probe/             # probe-rs library wrapper: flash/reset/RTT/defmt/mem, espflash + DFU paths (runs as subprocess)
│  └─ drivers/
│     ├─ drv-scpi/  drv-modbus/  drv-can/  drv-ble/  drv-sdr/  drv-camera/  drv-audio/  drv-iio/
│     └─ drv-subprocess/       # stdio JSON-RPC shim for sigrok-cli and other foreign tools; optional wasm host later
├─ bins/
│  ├─ benchd/                  # supervisor daemon (HTTP MCP endpoint + data plane + interlock)
│  ├─ bench-mcp-stdio/         # stdio entry point (for `ssh pi@bench bench-mcp-stdio`)
│  └─ benchctl/                # human CLI: arm/disarm, tokens, audit export/verify
├─ firmware/                   # separate workspace(s), per-target .cargo/config.toml
│  ├─ node-common/             # no_std: build-info section, postcard-rpc server, defmt setup
│  ├─ node-rp/ (embassy-rp)   node-nrf/ (embassy-nrf + trouble)   node-esp/ (esp-hal + esp-rtos + esp-radio)
│  ├─ bootloaders/             # embassy-boot configs
│  └─ tests/                   # embedded-test suites (HIL)
├─ sim/
│  ├─ sim-scpi/  sim-modbus/  sim-can/ (vcan)  sim-node/ (postcard-rpc over TCP/pty)  sim-bench/ (compose all)
│  └─ renode/  wokwi/          # platform files, Robot tests, scenarios
├─ third_party/                # pinned submodules: BusPirate5-firmware, IronOS … + container build recipes
├─ mws/                        # writecheck.yaml manifest, declaration, expected results for the simulated bench
├─ hardware/                   # KiCad, CAD sources
├─ docs/                       # mdBook
└─ .github/workflows/          # ci.yml, firmware.yml, sim.yml, mws.yml, fuzz.yml, release.yml (+ hil in private mirror or pull model)
```

**Ten design decisions to make early**
1. Object and version model for physical state: what are the MWS "objects" (channel configs, relay banks, firmware slots, captures, profiles), who mints versions, and which tools require a version. Everything in CONC/UPD/TGT/RES hangs on this.
2. Truth source for results: is "applied" defined by instrument read-back, by an independent measurement path the agent cannot reflash, or by command acknowledgement? Decide per tool class and encode it in the `outcome` logic (RES-1/RES-2), including the `unknown` path for timeouts.
3. Idempotency policy and durability: keys on every non-idempotent actuation (beyond MWS 0.9's create-only SHOULD), carried as a tool argument, recorded in SQLite before the effect, retained for a stated period, surviving kill/restart.
4. Where the interlock lives: a hardware arm switch and independent watchdog in the power path versus software-only gating; and what "safe state" is for each instrument on link loss, cancellation or supervisor crash.
5. Target MCP revision and long-operation model: 2026-07-28 stateless design with handles, MRTR for confirmations and the Tasks extension for flashing/captures, plus a fallback for clients that lack tasks; confirm rmcp 3.11 coverage before committing.
6. Transport/connection and auth model for the offline LAN: HTTPS + static bearer (or mTLS) versus stdio over SSH as the primary path for Claude Code; scope vocabulary and whether the tool catalog is scope-filtered.
7. Driver architecture: in-process Rust traits vs subprocess vs WASM, the self-describing capability descriptor that generates tool schemas/annotations/MWS declaration, and a `Transport` abstraction with a disclosed fault-injection hook so MWS proxy-assisted tests are possible.
8. Data plane and storage: Zenoh vs MQTT for node telemetry; SQLite + Parquet/DuckDB embedded vs a TSDB server; and the fixed "LLM summary" contract (stats, capped downsample, anomaly windows, plot, resource link) with a token budget per call.
9. Firmware identity and update path: mandatory build-info section, postcard-rpc `get_build_info`, attestation-verified images, A/B boot with rollback, one debug probe per node vs GPIO SWD, and which reference instrument firmware is off-limits to the agent.
10. CI and HIL trust boundary: what runs on free hosted runners against the simulated bench (including `mcp-writecheck` full mode) versus on real hardware, and whether HIL uses a private-mirror self-hosted runner or a runner-less pull model with attested artifacts. Decide before the first PR from a fork can arrive.

### Gaps
- The layout is untested; crate boundaries (especially `bench-mws` vs `bench-mcp`) should be revisited after a first vertical slice (one SCPI PSU simulator, one RP2040 node, flash + read-back + writecheck run).
- Whether the MWS steward would accept a Rust kit or a hardware-oriented annex/proposal is unknown; the comment window closes 16 Dec 2026.
