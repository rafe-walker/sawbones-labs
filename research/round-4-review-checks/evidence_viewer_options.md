# Evidence viewer options for Sawbones (borrow before building)

Research date: 25 November 2026. Sources are primary docs, repos and release notes fetched on that date; URLs are listed per section. Anything marked **UNVERIFIED** comes from memory or inference and was not confirmed against a source today. Paraphrased throughout.

## 1. Comparison table

| Tool | Licence / maintainer | Version (date) | Opens MCAP directly | Shows well | Pi 5 / LAN web | Drivable; selection back | Custom schemas without plugin |
|---|---|---|---|---|---|---|---|
| **Rerun** | Apache-2.0 + MIT; Rerun (company) | 0.43.2 (13 Nov 2026); releases roughly weekly in Oct-Nov 2026 | Yes (viewer, CLI, `rerun mcap convert`) | Scalars, images, video, tensors as heat maps, text log, state timeline, tables; audio file playback (unstable) | Web viewer (WASM) served from Pi on port 9090, rendered in the Mac browser | URL fragment `#when` / `#time_selection`, JS API, Python `ViewerClient`, MCP server; selection read-back is experimental | Yes: Protobuf and ROS 2 reflection, then manual visualizers or Lenses |
| **Lichtblick** | MPL-2.0; BMW AG (lichtblick-suite) | 1.30.0 (22 Oct, year inferred 2026); dev branch at 1.31.0 | Yes, local and remote URL | Scalars (Plot), images, log, state transitions, table, raw messages, 3D | Static web app / Docker image, or Electron desktop | URL params `ds`, `ds.url`, `time`, `layoutUrl`; desktop CLI `--source --time --defaultLayout`; no selection read-back found | Yes for Protobuf / JSON Schema / ROS / FlatBuffers in Plot, Table, Raw Messages |
| **Foxglove** | Proprietary since 2.0 (Mar 2024); Foxglove Technologies | 3.6.1 (23 Nov 2026) | Yes | Everything Lichtblick does plus Audio panel (waveform), video codecs, mono-image colour maps | Cloud web app or desktop; self-host only on Enterprise | Deep links (`time`, `ds=remote-file`, `layoutId`), `foxglove://`; desktop MCP not on Free plan | Yes (same schema families) |
| **PlotJuggler 4** | MPL-2.0; PlotJuggler org (D. Faconti) | 4.x line; dev version 4.2.0; beta 1 announced 2 Jul 2026 | Yes | Scalars (best in class: FFT, filters, run overlay), images, video, 2D markers, 3D | Desktop app; Linux builds are x86_64 only; macOS arm64 via conda/pixi | Agent plugin for Claude Code / Codex (v4.2+); selection read-back not confirmed | Yes for Protobuf, JSON, CBOR, MessagePack etc. |
| **Grafana** | AGPL-3.0; Grafana Labs | 13.3.x (date not confirmed) | No; no open MCAP data source found | Scalars, logs, annotations, heat maps of bucketed data | Server runs on Pi, browser on Mac | URL `from` / `to` (epoch ms), `var-` variables; no selection read-back | n/a (needs export to a database or Parquet) |
| **Perspective** | Apache-2.0; perspective-dev org (formerly FINOS-hosted) | 5.6.1 (23 Oct 2026) | No (Arrow / CSV / JSON) | Tables, pivots, 15+ chart types | Web component; engine in WASM or a Rust/Python/Node server | Config is JSON on the element; no timeline concept | n/a (columnar tables) |
| **sigrok PulseView** | GPL-3.0-or-later; sigrok project | 0.4.2 tarball (about six years old); git active, no new release confirmed | No (sigrok session files, VCD, CSV etc.) | Logic and analog traces with 131 protocol decoders | Qt desktop on Pi or Mac | No time-jump API found | Decoders are Python plugins |

Verdicts in one line each: Rerun is the primary; Lichtblick is the fallback that reads the same files; Foxglove is ruled out as a dependency (closed, account-bound) but its schemas and SDK are the interchange standard; PlotJuggler 4 is the specialist for deep trace analysis and run-versus-baseline overlays on the Mac; Grafana and Perspective do not fit the evidence-viewer job; PulseView is only useful for raw logic captures and only as a separate process.

## 2. Recommendation

**Primary: Rerun web viewer, served from the Pi, embedded in a small Sawbones page. Fallback: Lichtblick reading the same MCAP files.**

Reasons for Rerun:

- Licence fits. Apache-2.0 / MIT code can be linked or embedded by an Apache-2.0 project, not just run beside it.
- Rust all the way down. There is a Rust logging SDK, a Rust blueprint (layout) API, and Rust crates for serving the web viewer.
- MCAP is a first-class input. The viewer and CLI open `.mcap` directly, convert Foxglove Protobuf schemas to native views, and decode unknown Protobuf messages by reflection.
- It is the only candidate with a documented path for all three agent needs: open a recording at a time or time range (URL fragment), change the layout (blueprint), and read back what the user selected (JS `getSelection`, Python `get_selection`, an MCP tool; all experimental).
- The Pi only serves static WASM and data files. Rendering and GPU work happen in the Mac browser.
- One synchronised timeline across time series, tensor (heat map), image, video, text log and state timeline views.

Reasons for Lichtblick as fallback:

- It opens the same MCAP files with no conversion and understands the same Foxglove schemas, so the recorder does not need a second output format.
- MPL-2.0 is file-level copyleft; using it unmodified as a served web app or desktop app places no obligation on Sawbones code.
- It is the stable descendant of Foxglove Studio 1.x with URL parameters for source, time and layout.
- It is weaker on agent control (no time-range parameter, no selection read-back, no MCP) and has no audio or generic heat-map panel, which is why it is second.

What the recorder must do to suit both:

1. Keep MCAP as the source of truth. Do not record `.rrd` directly: its index footer is written only on clean shutdown and its column layout is versioned and migrated on load.
2. Use Protobuf message encoding with the schema embedded in each MCAP schema record. Rerun's documented reflection covers Protobuf and ROS 2 only; JSON-encoded channels are not described and would fall through to raw bytes.
3. Use Foxglove schemas where one exists (section 12). Use small custom Protobuf messages elsewhere, with flat numeric fields so they plot without code.
4. Put a `timestamp` field (Protobuf Timestamp) in custom messages too. Rerun builds a `timestamp` timeline from that field in addition to MCAP log time and publish time.
5. Name topics as stable paths (`/meter/ch2/volts`, `/bus/rs485/frames`); Rerun maps topic to entity path, and deep links refer to those paths.
6. Write short rotating segments. The web viewer downloads a non-`.rrd` file completely before loading it and caps itself at 2.5 GB by default (4 GiB hard limit for 32-bit WASM).
7. Flush on a timer so a power cut loses at most a few seconds (section 11).

What still has to be built (no viewer provides these):

- **Evidence page.** A small web page served by the supervisor: hypothesis and evidence matrix, action approval state, session list, with the Rerun viewer embedded through the `@rerun-io/web-viewer` package. Each evidence citation is a record of recording, timeline, start, end, entity paths.
- **"Show me" bridge.** A supervisor MCP tool that pushes such a record to the page over WebSocket; the page calls the viewer's open and set-time functions and applies a blueprint. The same record renders as a plain link for use without the page.
- **Selection hand-back.** A button that reads the viewer state and selection and posts them to the supervisor. Whether a user-dragged time range is exposed is not confirmed (see "Could not verify"); budget for a fallback of two "mark start / mark end" buttons using the readable time cursor.
- **Derived streams.** A post-processing step that writes a sidecar MCAP per segment: audio spectrogram and SDR waterfall as image rows, WAV chunks for playback, baseline traces re-timed onto the session timeline. Rerun has no waterfall view, no raw-PCM archetype and no cross-recording overlay; its docs list waterfall and log-axis plots as missing.
- **Default layouts.** One blueprint per session type, generated with the Rust blueprint API and saved as `.rbl`, plus an equivalent Lichtblick layout JSON for the fallback.
- **Optional conversion job.** `rerun mcap convert` to `.rrd` for long sessions, so the viewer can load on demand instead of holding the whole file.
- **Version pinning.** Pin the Rerun viewer version in the supervisor build; the JS API was renamed to camelCase in 0.43 and the control APIs are marked experimental.

Specialist tools to launch as separate processes when needed: PlotJuggler 4 for FFT, filtering and overlaying a run on a baseline; PulseView (GPL) only if raw logic-level captures are ever recorded.

## 3. Rerun

Sources: github.com/rerun-io/rerun (README, CHANGELOG.md), rerun.io/docs pages for MCAP import, message formats, decoders, viewer URLs, embed-web, host-web-viewer, control-the-viewer, callbacks, limit-ram, rrd-format, audio, tensor, roadmap, MCP server.

1. **Licence, maintainer, status.** Dual Apache-2.0 and MIT. Maintained by the company Rerun, which also sells a hosted Hub. Latest 0.43.2 on 13 Nov 2026, preceded by 0.43.0 (11 Nov), 0.42.0 (4 Nov), 0.41.0 (28 Oct), 0.40.0 (21 Oct). Very active. The README still warns that breaking changes should be expected.
2. **MCAP, .rrd, Rust SDK.**
   - Loads MCAP natively as of late 2026: `rerun file.mcap`, drag and drop, or `log_file_from_path` from the SDK. `rerun mcap convert in.mcap -o out.rrd` converts.
   - Decoders run per channel in priority order (ROS 2 typed, ROS 2 reflection, Protobuf reflection), with a raw-bytes fallback for anything else. Foxglove "lenses" then turn known Foxglove Protobuf messages into native types. MCAP metadata and attachments are carried across.
   - Foxglove schemas converted automatically: RawImage, CompressedImage, CompressedVideo, CameraCalibration, PointCloud, LocationFix(es), FrameTransform(s), PoseInFrame / PosesInFrame, Log, VoxelGrid. Not in the list: Grid, RawAudio, CompressedAudio.
   - `.rrd` is Rerun's own stream format: a header, then framed messages carrying Arrow IPC chunks (one entity path per chunk), optional LZ4 per chunk, and an optional footer index that enables random access and larger-than-memory loading. Framing is stable since 0.23; a file not closed cleanly has no footer and is read linearly. `rerun rrd optimize` can add a footer.
   - Rust logging SDK: yes (`rerun` crate), with sinks for spawn, gRPC connect, gRPC serve, save to file, stdout, and a function to serve the web viewer.
3. **Data kinds.**
   - Scalars: strong. Time series view with pan/zoom, linked time axes across plots, fixed windows, error bands. No logarithmic axis, no XY plots.
   - Images and video: strong (raw, encoded, H.264 stream).
   - 2-D arrays: tensor view with colour maps (turbo shown), slicing, and pan/zoom added in 0.43. It shows the tensor at the current time; there is no scrolling waterfall. Documented workaround: build an image from history.
   - Audio: experimental audio view shows a waveform and plays audio files (wav, flac, mp3, ogg, aac, m4a) aligned to the timeline. No raw-PCM type and no spectrogram; docs say to wrap PCM chunks as in-memory WAV.
   - Text: text log view with level filter; no text search and no custom columns yet. A dataframe view can show structured rows, which suits a bus transaction table better.
   - Markers: state timeline view (stable since 0.34) draws discrete states as coloured lanes; text log entries mark instants. No dedicated "event with duration" annotation type.
4. **Where it runs.** `rerun --serve-web` hosts the web viewer on port 9090; the viewer is the native viewer compiled to WASM and runs in the Mac browser. Python wheels exist for Linux aarch64 (glibc 2.28+). Chromium uses WebGPU with WebGL fallback; Safari and Firefox use WebGL. The native viewer wants Vulkan (OpenGL fallback; software OpenGL is discouraged), so running the native viewer on the Pi itself is the wrong shape; serve from the Pi, render on the Mac. Native default memory limit is 75% of RAM and respects cgroup limits since 0.43.
5. **Driving it.**
   - URL: the web viewer takes one or more `url=` parameters (file over HTTP or a gRPC endpoint). `.rrd` and `.rbl` stream in; other importable types are downloaded first. A fragment can set the cursor (`#when=timeline@time`) and a range (`#time_selection=timeline@start..end`). In the 0.31 source the fragment was honoured only when exactly one `url` parameter was present.
   - JS API (experimental): open and close recordings, get state, set time and play state, set location, `getSelection` (0.43), events for selection change, time update, timeline change, recording open.
   - Python `ViewerClient` (experimental): connect to a running viewer, open, set time, get state, get selection, get and set blueprint as JSON, screenshots. No Rust client is documented.
   - MCP: `rerun mcp` (stdio) lets an agent inspect an open viewer, apply layouts, set overrides, read settings, and query a catalog. Tool names seen in the changelog include a get-selection tool and a transform-graph tool.
   - Blueprint time panel accepts a time selection range and loop mode, so a layout can carry a highlighted region.
   - Selection read-back returns entities, instances, components, views, containers and recordings. A time range is not listed in that description.
   - CORS: when the viewer page and data come from different origins, the data host must allow the page origin. Serving both from the supervisor avoids this.
6. **Custom schemas.** Unknown Protobuf (and ROS 2) messages are decoded by reflection into queryable struct columns; scalar fields can be added to a time series view by hand or by blueprint component mapping, and Lenses can map them to native types. Native types are "archetypes" (Scalars, Image, Tensor, TextLog, StateChange, BarChart, AssetAudio, and so on).
7. **Limits and annoyances.** Fast release cadence with breaking API changes; control and selection APIs experimental; no waterfall, spectrogram, log axis or text search; cross-recording overlay is not offered, and linking different entities across views is listed as missing; web viewer memory ceiling; audio view marked unstable.

## 4. Lichtblick

Sources: github.com/lichtblick-suite/lichtblick (README, releases, `package.json`, `appURLState.ts`, panel index), lichtblick-suite.github.io/docs.

1. MPL-2.0. Source headers name BMW AG as copyright holder; the project lives in the lichtblick-suite organisation. v1.30.0 released 22 Oct (the page omits the year; Electron 44 in its dependency updates implies 2026). Recent releases on 8 Sep, 6 Aug, 28 Jul, 13 Jul. Active.
2. Opens MCAP directly, local (drag and drop, multiple files merged on one timeline) and remote by URL. Also ROS 1 bags and live connections.
3. Built-in panels from the source index: 3D, Image, Plot, State Transitions, Log, Table, Raw Messages, Gauge, Indicator, Pie Chart, Map, Diagnostics, Topic Graph, Data Source Info, Variable Slider, User Scripts, Publish, Call Service, Parameters, Teleop. Scalars, images, log lines and state lanes are covered. No audio panel. No generic heat-map panel; a 2-D array has to be presented as a single-channel image. v1.30 added a delta measuring mode for Plot and State Transitions.
4. Web build needs Chrome 111 or later. Runs as a Docker image on port 8080 or as an Electron desktop app. Whether the published image is built for arm64 was not confirmed; the web build is static files, so the supervisor could serve them. WebGL is needed for the 3D panel only (**UNVERIFIED**).
5. URL state parsed by the app: `ds`, `ds.*` (including repeated `ds.url`), `time`, `layoutUrl`, and an MCAP bundle id. Desktop CLI: `--source` (files or a directory), `--time` (UNIX seconds or date string), `--defaultLayout` (by name). There is a single time point, not a range. No documented way to read a user selection out. v1.29.1 added a get-message-at-time call to the extension API.
6. Message schemas accepted: Protobuf, JSON Schema, ROS 1, ROS 2, TypeScript, FlatBuffers. Any decoded message works in Plot, Table and Raw Messages via message paths. Domain panels (Image, Log, 3D) need Foxglove or ROS schemas, or a message-converter extension (TypeScript).
7. Annoyances: user documentation is thin and partly defers to Foxglove's; custom panels mean a TypeScript extension; no comparison of two recordings; large multi-file sessions had out-of-memory fixes as recently as v1.28.1.

## 5. Foxglove

Sources: foxglove.dev/pricing, docs.foxglove.dev (pricing, changelog, shareable links, audio panel, image panel, standalone licence, schema list), 2024 announcement thread on the Open Robotics forum.

1. Proprietary. The open-source Studio edition ended with 2.0 in March 2024; later development is closed. Latest app release 3.6.1 on 23 Nov 2026 (3.0.0 was 18 Aug 2026). Very active. MCAP and the SDK with its schemas remain open (MIT).
2. Opens MCAP directly.
3. Broadest panel set: 22 panels including Plot, Image (raw, compressed, video; turbo and rainbow colour maps for single-channel images with min/max scaling), Log, State Transitions, Table, Audio (RawAudio and CompressedAudio as waveforms with playback; no spectrogram).
4. Free plan: 3 users, 10 GB storage, 5 devices, 1 project. Not on Free: personal layouts, comparison mode, embedded viewer, local MCP server, personal access tokens. The Grafana data source and offline standalone licence are Enterprise. At the 2.0 change the maintainers said the desktop app asks for sign-in at first launch and then works offline for local files; current behaviour not re-confirmed.
5. Deep links: `time` (RFC 3339), `ds=remote-file` with `ds.url`, `layoutId`, `openIn=desktop`, and a `foxglove://open` scheme. A built-in agent sidebar (beta) and a desktop MCP server exist, the latter on paid plans.
6. Same schema families as Lichtblick.
7. Cannot be shipped or self-hosted with an open project; depends on an account and vendor terms that have changed before. Fine as an optional viewer a user may choose, because the recorder's files are compatible.

## 6. PlotJuggler

Sources: github.com/PlotJuggler/PlotJuggler (README, `versions.env`), July 2026 beta announcement on the Open Robotics forum.

1. MPL-2.0. Maintained by its original author under the PlotJuggler organisation. Now on the 4.x line (Qt 6.12; dev version 4.2.0); 4.0 beta 1 was announced 2 Jul 2026. Active. The exact latest release tag and date were not retrieved.
2. Opens MCAP directly, with a parallel lazy loader. Also CSV, Parquet, ULog, MP4.
3. Scalars: the strongest of the group (drag and drop, zoom, filters, derivatives, integration, FFT, Lua and Python transforms). Loads several recordings, aligns their timelines and overlays signals, which is the baseline-overlay requirement. Version 4 adds raw and encoded images, video, 2D annotations and markers, and 3D. Heat maps and audio are not mentioned.
4. Desktop application. Linux packages are x86_64 (apt, AppImage); macOS arm64 via conda/pixi; Windows. No arm64 Linux build is listed, so treat it as a Mac-side tool reading files from the Pi. The repo carries WebAssembly build settings, but a supported browser build was not confirmed. Rendering is described as GPU-accelerated.
5. An official agent plugin for Claude Code and Codex can open logs, compute over signals, create derived series and markers, build plots and take screenshots (needs 4.2+). Selection read-back not confirmed.
6. Message parsers for Protobuf, JSON, CBOR, BSON, MessagePack, Arrow and ROS; fields appear as plottable series without a plugin.
7. Version 4 is new and was labelled early beta in July; no Pi build; not a web viewer.

## 7. Grafana

Sources: grafana.com docs (what's new, dashboard URL variables), grafana.com plugin page for the Foxglove data source, repo LICENSE.

1. AGPL-3.0, Grafana Labs. Docs mark 13.3 as latest (a 13.3.3 branch exists; main is 13.4.0-pre). Release date not retrieved. Active.
2. No MCAP data source was found. The Foxglove data source (v0.0.10) is commercial, needs an Enterprise Foxglove plan, and queries Foxglove's cloud API. A local setup would need scalars exported to SQLite, DuckDB/Parquet or a time-series database.
3. Good for scalar time series, logs, annotations and heat maps of bucketed values (**UNVERIFIED**, from memory). Poor for images, audio and sample-accurate sub-millisecond inspection.
4. Server runs on a Pi and is viewed from the Mac browser (**UNVERIFIED** for current arm64 packages). No GPU needed.
5. URL parameters `from` and `to` (epoch milliseconds or relative), `time` with `time.window`, and `var-name` variables. No selection read-back.
6. Not applicable; it sees database columns.
7. Adds a database and an export pipeline for a weaker result; dashboard time range is millisecond-granular. AGPL is acceptable only as a separate process. Not recommended for evidence review; possibly useful later for long-term bench health trends.

## 8. Perspective

Sources: github.com/perspective-dev/perspective (README, releases, licence).

1. Apache-2.0. Repo now under the perspective-dev organisation. v5.6.1 on 23 Oct 2026; v5.6.0 on 14 Oct. Active.
2. No MCAP. Inputs are Arrow, CSV and JSON.
3. A data grid, pivot table and WebGL chart component (15+ chart types) over a streaming query engine. Excellent for a large filterable bus transaction table; no timeline playback, images or audio.
4. Engine runs in the browser as WASM or on a server with Python, Node.js or Rust bindings over WebSocket, so a Rust supervisor on the Pi could host tables for a Mac browser.
5. Embeddable web component with a JSON configuration; no notion of a shared time cursor.
6. Not applicable.
7. Would be one more component to integrate. Consider it only if Rerun's dataframe view proves too limited for the bus log.

## 9. sigrok PulseView

Sources: sigrok.org wiki (PulseView, protocol decoders), sigrok-devel mailing list (Jan 2026), Debian package tracker.

1. GPL-3.0-or-later; sigrok project volunteers. Last release tarball is 0.4.2, described on the developer list in January 2026 as nearly six years old. Development continues in git and a service release was proposed, not confirmed. Debian uploaded 0.4.2-5 in March 2026.
2. No MCAP. It reads sigrok session files and imports formats such as VCD and CSV (**UNVERIFIED**, from memory). A converter from MCAP would have to be written.
3. Logic and analog waveforms with stacked protocol decoders: 131 decoders including CAN, UART, Modbus RTU over RS485 and LIN. No K-line / ISO 9141 / KWP2000 decoder appears in the supported list.
4. Qt desktop app; Debian packages exist, so it can run on the Pi or the Mac. No web mode.
5. No API or URL for jumping to a time range was found; a file can be given on the command line (**UNVERIFIED**).
6. Decoders are Python plugins.
7. Sawbones records already-decoded frames from transceivers, so there is nothing for PulseView to decode. It becomes relevant only if a logic analyser instrument is added; then export `.sr` and launch it as a separate GPL process.

## 10. Other candidates looked at

- **Foxglove SDK, Rust crate `foxglove`** (MIT, 0.31.0 on 13 Nov 2026, MSRV 1.88). Not a viewer: a recorder-side library that writes MCAP with the Foxglove schemas as typed Rust structs and can serve live data. Worth using in the recorder for the standard schemas instead of hand-maintaining Protobuf definitions.
- **Rerun open-source catalog server** (`rerun server`): serves a directory of `.rrd` files with per-recording links; an option once sessions are converted.
- **Perfetto UI**: trace viewer with documented deep links (`visStart`, `visEnd` in nanoseconds) and a postMessage loader. Timeline-and-tracks only; no images or heat maps. Licence not confirmed today (**UNVERIFIED**: Apache-2.0).
- **Sonic Visualiser** and **inspectrum** (both GPL, **UNVERIFIED**): mature desktop tools for audio spectrograms and SDR captures. Possible separate-process helpers if derived spectrogram images are not enough.
- No better primary candidate was found.

## 11. MCAP Rust crate (`mcap` on crates.io)

Sources: docs.rs/mcap, repo `rust/mcap/Cargo.toml`, `README.md`, `src/write.rs`.

- **Version and status.** 0.26.0, published 19 Oct 2026. MIT. Rust 1.81 minimum. Lives in the main foxglove/mcap repo; the repo now also contains a Rust CLI crate (0.4.0, not published to crates.io). Actively maintained.
- **Features.** Default features `zstd` and `lz4`; optional `tokio` for async reading. Builds without compression using no default features.
- **Chunking.** On by default with a 1 MiB target uncompressed chunk size, configurable or disabled. Unchunked mode exists for very small systems but gives up compression and indexing.
- **Compression.** Zstd or LZ4 per chunk, with level and thread count options.
- **Indexing.** Options to emit message indexes, chunk indexes, attachment and metadata indexes, statistics, summary offsets, and repeated schema and channel records in the summary. CRCs for chunks, data section, summary and attachments are switchable.
- **Other writer features.** Attachments (streamed), metadata records, private records, schema and channel de-duplication, a non-seeking mode for stream outputs.
- **Reading.** A simple message reader, plus I/O-independent readers: linear, indexed (random access through the summary) and summary-only. Memory scales with the largest chunk, not the file.
- **Crash and power-loss behaviour.** The summary and footer are written by `finish()`. `flush()` closes the current chunk and flushes the writer, so a later interruption leaves everything up to that point streamable. The crate states resilience as a design goal: every valid message should be recoverable from an incomplete file or chunk. The `mcap recover` CLI command rebuilds indexes for unfinished or unindexed files.
- **Recorder guidance.** Keep chunking and CRCs on; call `flush()` on a short timer and sync the file; rotate by size and time and call `finish()` at rotation; at boot, run recovery on any segment without a footer before serving it. Data after the last flushed chunk is lost on power cut. Whether `flush()` also forces data to stable storage depends on the underlying writer; the supervisor must call the OS sync itself.

## 12. Schemas the recorder should use

Sources: Foxglove SDK schema list and `.proto` files, Rerun message-format table, Foxglove panel docs.

| Stream | Schema | Rerun | Lichtblick | Notes |
|---|---|---|---|---|
| Meter readings, any scalar | Custom Protobuf with `timestamp` and flat numeric fields | Reflection, then time series via blueprint mapping | Plot panel by message path | No standard scalar schema exists in either ecosystem |
| Camera frames | `foxglove.CompressedImage` (JPEG/PNG) or `foxglove.CompressedVideo` (H.264) | Encoded image / video stream | Image panel (video support in the fork **UNVERIFIED**) | Standard |
| Thermal frames | `foxglove.RawImage`, single channel (`mono16` or `32FC1`) | Image or depth image; colour maps apply to depth images and tensors | Image panel; colour-map support for mono images **UNVERIFIED** in the fork | Avoid `foxglove.Grid` |
| Spectrogram, waterfall | `foxglove.RawImage` rows built by the derive step | Image view | Image panel | Derived, not recorded live |
| Power-spectrum frame | Custom Protobuf: timestamp, start frequency, bin width, repeated float power | Reflection; needs a Lens or converter to a bar chart or tensor | Plot panel against array index (**UNVERIFIED**) | No standard schema |
| Audio | `foxglove.RawAudio` (`pcm-s16`, sample rate, channel count) | Not converted; derive WAV chunks for the audio view | No panel | Standard schema, plays only in proprietary Foxglove |
| Bus frames | `foxglove.Log` line per transaction plus a custom Protobuf frame record | Text log, and dataframe view for the structured record | Log panel, Table panel | Two channels: human line and structured row |
| Event markers, triggers | `foxglove.Log` plus a small state message (string or enum) | Text log; state timeline after mapping | Log panel; State Transitions panel | `foxglove.Event` (start, end, metadata) exists but no viewer support was confirmed |

- **Audio standard:** yes, `foxglove.RawAudio` and `foxglove.CompressedAudio` (Opus, AAC and others). RawAudio carries interleaved little-endian samples, a format string, sample rate and channel count.
- **Power-spectrum standard:** none found in the Foxglove schema list (50 schemas) or as a Rerun MCAP mapping. Rerun's own types for it are Tensor and BarChart.
- **`foxglove.Grid`:** a posed grid for the 3D scene (origin pose, cell size, packed fields). Rerun does not convert it, so it is a poor choice for thermal or waterfall data.
- **Schema definition styles.** Foxglove publishes each schema as Protobuf, JSON Schema, ROS 1, ROS 2, TypeScript and FlatBuffers. Rerun defines archetypes (bundles of Arrow components). Choose Protobuf: it is the one encoding that Rerun reflection, Lichtblick and PlotJuggler all decode.
- The MCAP `protobuf` schema record is expected to hold a binary file-descriptor set (**UNVERIFIED** today, from the MCAP spec as remembered); the `foxglove` Rust crate handles this for its own types.

## 13. Experiment and test-plan formats

**OpenTAP.** Licence MPL-2.0; an open project with Keysight as the main backer, which sells an editor and bundle on top. The downloads page lists 9.35 as latest (date not found; 9.34 was announced 2 Sep 2026). Installers target Windows 10+, Ubuntu 24.04+ and Apple-silicon macOS; a Linux arm64 build was not confirmed, and the engine runs on .NET (docs include a .NET 9 migration page). A test plan is a tree of test steps saved as an XML `.TapPlan` file: the serializer writes each step's public properties, child steps nest under parents (which decide sequential or parallel execution), instruments and DUTs are referenced through properties, and each step yields a verdict on an ordered scale from not-set through pass, inconclusive, fail, aborted to error. Plans run from an editor or `tap run`; result listeners write logs, CSV or databases. A Rust runtime cannot execute a plan natively, because every step is a .NET plugin class identified by type. It could import a restricted subset by parsing the XML and mapping a fixed list of known step types onto its own handlers, or simply run `tap` as a separate process. The ideas worth borrowing are the step tree, the verdict ordering, externally settable parameters and result listeners. There is no published schema for the XML that was found today.

**Labgrid.** Licence LGPL-2.1-or-later; started at Pengutronix, now a community project. Latest 26.0.1 on 14 Oct 2026; Python 3.10+. It is a Python library for controlling embedded boards and explicitly not a test framework. What it defines declaratively is the environment, not the experiment: a YAML file lists targets, each with resources (for example a serial port) and drivers bound to them (serial, shell, power). Tests are ordinary pytest functions run with an option naming the environment file; a coordinator, exporters and a client share lab hardware across machines. There is no plan file to compile or execute offline. A Rust runtime could read the environment YAML as an inventory description, but cannot run labgrid tests without Python. The borrowable idea is the resource / driver / target split.

**Others, briefly.**
- sigrok session files are capture archives, not plans (**UNVERIFIED**).
- pytest-embedded and OpenHTF express tests as Python code, not data (**UNVERIFIED**).
- Lager's format was not checked.
- testerkit (formerly litmus-test; Apache-2.0, alpha, mid-2026) pairs YAML sequence, limit and sweep files with pytest bodies. It is the closest in spirit but still needs Python.
- IEEE 1671 ATML is a formal XML standard for test descriptions, heavy and tooling-poor (**UNVERIFIED**).

**Conclusion.** No existing open format is both agent-compilable and executable offline by a Rust device runtime. Define a small Sawbones plan schema (JSON with a JSON Schema, steps as data referring to a closed set of instrument verbs), borrowing OpenTAP's step tree and verdict scale and labgrid's resource and driver vocabulary. Keep an importer for a subset of `.TapPlan` as a possible later nicety, not a dependency.

## 14. Could not verify

- Whether a time range the user drags in Rerun's time panel can be read back through `getSelection`, `getState` or the blueprint. The documented selection contents do not list it.
- Whether Rerun still restricts `#when` / `#time_selection` fragments to links with exactly one `url` parameter (seen in 0.31 source), which matters when a recording and a blueprint are opened together.
- Exact tool list of the Rerun MCP server, and whether it can set the time cursor or open a recording by itself.
- Rerun handling of JSON-encoded MCAP channels (assumed raw fallback), and which RawImage encodings map to depth image with colour map.
- Rerun native viewer on Raspberry Pi 5 graphics drivers. Not needed for the recommended setup.
- Lichtblick: release year of v1.30.0 (shown without year), arm64 Docker image, CompressedVideo and mono colour-map support in the Image panel, Plot against array index, `foxglove.Log` support in its Log panel.
- Foxglove: whether the current desktop app still opens local files after a one-time sign-in on the Free plan; Linux arm64 desktop build.
- PlotJuggler 4: latest release tag and date, status of the WebAssembly build, any API to read a user's selection, heat-map or audio support.
- Grafana: 13.3 release date, current arm64 packaging, panel capabilities stated from memory.
- PulseView: input format list and command-line options; whether the proposed 2026 service release happened.
- OpenTAP: 9.35 release date, Linux arm64 support, minimum .NET runtime, an example of the XML element layout.
- GitHub API and release feeds were not reachable from this environment, so several release dates come from rendered pages or changelog files instead.
