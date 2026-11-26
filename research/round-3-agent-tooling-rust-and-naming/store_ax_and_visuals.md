# AX local data store for a frontier multimodal agent over MCP: limits, patterns, Rust building blocks (state: 2026-11-25)

Conventions used in these notes:
- **[DOC]** = documented by the vendor/spec and fetched this session. **[COMMUNITY]** = bug report, third-party doc or blog (lower confidence). **[DERIVED]** = computed by me from a documented formula. **[POINTER]** = stable reference URL given from prior knowledge, *not re-fetched this session*; verify before relying on it.
- Crate versions were read from the crates.io API on 2026-11-25.
- Wake-contract note: the project's `get_system_state` tool (isokron MCP server) failed to connect (HTTP 502) on every attempt, so no cursor check was done. This task was web research only and touched nothing on the chain.

---

## 1. Images through MCP to Claude in late 2026

### Takeaway
MCP 2026-07-28 tool results can carry text, image, audio, resource_link, embedded resource and `structuredContent`; in practice only **text and image blocks** are safe to depend on across Claude clients. Claude 4.7-and-later models see up to **2,576 px long edge / 4,784 visual tokens per image** (28x28 px per token), Claude Code caps a tool result at **25,000 tokens (images included)**, and several client bugs make "image + structuredContent" and "resource_link" unreliable, so `render` should return one pre-sized image plus a compact text block.

### Cited Findings

**MCP 2026-07-28 wire facts**
- [DOC] Tool results may hold several content items: `text`, `image` (base64 `data` + `mimeType`), `audio`, `resource_link` (uri, name, description, mimeType) and embedded `resource`; all accept optional annotations (`audience`, `priority`, `lastModified`) — [MCP tools spec 2026-07-28](https://modelcontextprotocol.io/specification/2026-07-28/server/tools)
- [DOC] `structuredContent` may now be any JSON value and must conform to `outputSchema` when one is declared; for backward compatibility a tool returning it SHOULD also put the serialized JSON in a text block (so the same data can reach the model twice) — [MCP tools spec](https://modelcontextprotocol.io/specification/2026-07-28/server/tools); [MCP changelog](https://modelcontextprotocol.io/specification/2026-07-28/changelog)
- [DOC] A resource link is a URI the client "can" subscribe to or fetch; nothing obliges a client to fetch it, and links are not guaranteed to appear in `resources/list` — [MCP tools spec](https://modelcontextprotocol.io/specification/2026-07-28/server/tools)
- [DOC] The spec states no size limit for image content. Protocol-level sessions and the `initialize` handshake are gone; any cross-call state must be an explicit server-minted handle passed as a normal tool argument — [MCP changelog](https://modelcontextprotocol.io/specification/2026-07-28/changelog)
- [DOC] Execution errors belong in the result with `isError: true` and actionable text (so the model can self-correct); protocol errors are for malformed requests — [MCP tools spec](https://modelcontextprotocol.io/specification/2026-07-28/server/tools)
- [DOC] Servers SHOULD return `tools/list` in deterministic order to help prompt caching — [MCP changelog](https://modelcontextprotocol.io/specification/2026-07-28/changelog)

**How Claude models count and resize images (applies to any image block, including ones inside tool results)**
- [DOC] Cost per image = `ceil(width/28) * ceil(height/28)` visual tokens — [Claude vision docs](https://platform.claude.com/docs/en/build-with-claude/vision)
- [DOC] Two tiers: **high-resolution** (Claude 4.7 and later): max long edge 2,576 px, max 4,784 visual tokens; **standard** (all other models): 1,568 px and 1,568 tokens. High-res is automatic, no header — [Claude vision docs](https://platform.claude.com/docs/en/build-with-claude/vision)
- [DOC] Examples: 1000x1000 = 1,296 tokens on both tiers; 1920x1080 = 2,691 tokens on high-res but shrunk to 1456x819 (1,560 tokens) on standard; 3840x2160 shrinks to 2576x1449 (4,784 tokens) on high-res — [Claude vision docs](https://platform.claude.com/docs/en/build-with-claude/vision)
- [DOC] The visual-token cap, not the edge cap, usually decides the final size; every image is then padded on the right/bottom to a multiple of 28 px — [Claude coordinates docs](https://platform.claude.com/docs/en/build-with-claude/vision-coordinates)
- [DOC] Hard limits: 8000x8000 px per image; 10 MB per image (base64-encoded) on the Claude API, 5 MB on Bedrock, 10 MB on claude.ai; 100 images per API request for 200k-context models and 600 for others; 20 per message on claude.ai; above 20 image blocks in one request a stricter per-image dimension cap applies (keep each side at or below 3,000 px) and images nested in `tool_result` blocks count toward that threshold — [Claude vision docs](https://platform.claude.com/docs/en/build-with-claude/vision)
- [DOC] Formats: JPEG, PNG, GIF, WebP; no animation (first frame only); image metadata (EXIF) is not read by the model — [Claude vision docs](https://platform.claude.com/docs/en/build-with-claude/vision)
- [DOC] Request-size limit quoted as 128 MB on the first-party API — [Claude vision docs](https://platform.claude.com/docs/en/build-with-claude/vision); the crop-tool cookbook instead speaks of a 32 MB request limit — [Claude cookbook: crop tool](https://platform.claude.com/cookbook/multimodal-crop-tool). The two Anthropic pages conflict; treat 32 MB as the conservative figure.
- [DOC] Base64 images are re-sent in the payload on every later turn of a conversation, which inflates request size and latency as images pile up — [Claude vision docs](https://platform.claude.com/docs/en/build-with-claude/vision)
- [DERIVED] Sizes that pass through un-resized on *both* tiers (so pixel coordinates stay 1:1 on any Claude model): 1092x1092 (1,521 tokens), 1344x896 (1,536), 1456x819 (1,560), 1270x952 (1,564). Largest high-res-tier sizes: 1932x1932 (4,761), 2576x1449 (4,784), 2212x1659 for 4:3 (4,740). Computed with the reference `resized_size()` from [Claude coordinates docs](https://platform.claude.com/docs/en/build-with-claude/vision-coordinates)

**Anthropic guidance on vision use**
- [DOC] Put images before text when possible; label multiple images ("Image 1:", "Image 2:") — [Claude vision docs](https://platform.claude.com/docs/en/build-with-claude/vision)
- [DOC] Lossy compression shrinks requests but artifacts (especially repeated JPEG passes) can make text unreadable; do not crop away key context just to enlarge text; images under 200 px, rotated or low-quality images raise hallucination risk — [Claude vision docs](https://platform.claude.com/docs/en/build-with-claude/vision)
- [DOC] Stated limitations: coordinate/localization output is approximate; counts of many small objects are approximate — [Claude vision docs](https://platform.claude.com/docs/en/build-with-claude/vision)

**Coordinates and pointing**
- [DOC] Claude works best with **absolute pixel coordinates** (origin top-left) and poorly with normalized 0-1 or 0-1000 coordinates; returned coordinates refer to the image *after* Claude's resize, so pre-resize to the exact processed size or rescale afterwards — [Claude coordinates docs](https://platform.claude.com/docs/en/build-with-claude/vision-coordinates)
- [DOC] Example trap: an A4 page at 130 DPI (1075x1520) is under the edge limit but costs 2,145 tokens, so a standard-tier model silently shrinks it to 924x1307 — [Claude coordinates docs](https://platform.claude.com/docs/en/build-with-claude/vision-coordinates)
- [DOC] Small elements lose precision when downscaled; the recommended fix is to crop the region of interest and send the crop (offsetting coordinates by the crop origin) or use a high-res-tier model — [Claude coordinates docs](https://platform.claude.com/docs/en/build-with-claude/vision-coordinates)
- [DOC] Anthropic's own tests found that **coordinate grid overlays and image tiling did not reliably improve click accuracy**; a zoom action (inspect a region at higher resolution before acting) is what they recommend for dense screens — [Claude blog: computer/browser use best practices, 2026-05-13](https://claude.com/blog/best-practices-for-computer-and-browser-use-with-claude)
- [DOC] Same post: each screenshot costs about 1,000-1,800 tokens; keep the 3 most recent images in full and prune older ones in batches (to keep the prompt cache prefix stable); a 16 px checkbox on a 4K screen shrinks to about 5 px at 1280x720 — [Claude blog](https://claude.com/blog/best-practices-for-computer-and-browser-use-with-claude)
- [DOC] A zoom/crop tool (pixel box in, magnified crop from the full-resolution original out, JPEG quality 92 with no chroma subsampling) raised accuracy on the 100-question Chartography chart benchmark from 29% to 73% for claude-fable-5 and from 13% to 44% for claude-sonnet-5, at roughly 12x cost per question ($0.08 to $0.95) — [Claude cookbook: crop tool](https://platform.claude.com/cookbook/multimodal-crop-tool)
- [DOC] Published bounding-box grounding on document pages is far below human level: in ViDoRe V3, human annotators agree at F1 0.602 while the best models reach 0.089 (Qwen3-VL-30B) and 0.065 (Gemini 3 Pro) — [ViDoRe V3, arXiv 2601.08620](https://arxiv.org/html/2601.08620v2)
- [COMMUNITY/research] Graph-of-Mark (an extension of set-of-mark that draws numbered marks plus relation edges on the image) improved VQA/localization accuracy by up to 11 percentage points on three open-source models; not tested on Claude — [arXiv 2603.06663](https://arxiv.org/abs/2603.06663)

**Claude Code (CLI on the MacBook)**
- [DOC] PNG/JPEG/GIF/WebP images returned by an MCP tool are shown to the model inline; the inline copy may be scaled down or compressed to fit model limits; since v2.1.283 the original bytes are also saved to a session `tool-results` file and the path is given to Claude — [Claude Code MCP docs](https://code.claude.com/docs/en/mcp)
- [DOC] Limits: warning above 10,000 tokens; default cap 25,000 tokens, changed with `MAX_MCP_OUTPUT_TOKENS`; **image-bearing results stay subject to that token cap** even when a tool declares `_meta["anthropic/maxResultSizeChars"]`; text-only results over 50,000 characters (or over the token cap) are written to a file and replaced by a path; the per-tool annotation can raise the text threshold up to 500,000 characters; HTTP/SSE responses over 16 MiB fail; `isError` text beyond about 11,000 characters is cut to first and last 5,000 — [Claude Code MCP docs](https://code.claude.com/docs/en/mcp)
- [DOC] Server instructions and upfront-loaded tool descriptions are cut at 4,096 characters; tool-search-loaded descriptions at 16,384; tool search (deferred tool loading) is on by default — [Claude Code MCP docs](https://code.claude.com/docs/en/mcp)
- [DOC] The v2 MCP runtime adds protocol revision 2026-07-28; tasks are public beta from v2.1.304 (`resultType: "task"`, poll interval clamped 250-30,000 ms, default 2,000); tool calls auto-move to background after 2 minutes; elicitation is only served in interactive terminal sessions — [Claude Code MCP docs](https://code.claude.com/docs/en/mcp)
- [DOC] Resources are reachable through `@server:uri` mentions and built-in list/read resource tools; MCP Apps `ui://` resources are for a host to render and are not content for Claude Code — [Claude Code MCP docs](https://code.claude.com/docs/en/mcp)
- [COMMUNITY] Bug report (Claude Code 2.1.123): when a result carries `structuredContent`, the whole `content` array, including image blocks, never reaches the model; workaround is to omit `structuredContent` on image-bearing results. Current status not confirmed — [claude-code issue 54737](https://github.com/anthropics/claude-code/issues/54737)
- [COMMUNITY] Bug report (Claude Code 2.1.117): `resource_link` blocks are silently ignored and PDF/Office blobs from `resources/read` are not processed, while images work. Current status not confirmed — [claude-code issue 53453](https://github.com/anthropics/claude-code/issues/53453)
- [COMMUNITY] An open report shows the error text "Tool result is too large. Maximum size is 1MB." (the version string in the report looks like a desktop-app build; which client is unclear) — [claude-code issue 66519](https://github.com/anthropics/claude-code/issues/66519)

**Claude desktop / web apps**
- [COMMUNITY] A third-party MCP server's compatibility guide rates Claude Desktop and claude.ai as rendering image content, MCP Apps and resources; Claude Code as "partial" for images and no MCP Apps; Claude Mobile unconfirmed; it reports a roughly **1 MB limit on tool-result content** in Desktop/claude.ai and works around it by always returning a WebP thumbnail capped at 512 px (page undated) — [image-generation-mcp client compatibility](https://pvliesdonk.github.io/image-generation-mcp/guides/client-compatibility)
- [COMMUNITY] In Claude Desktop 1.4758.0 (2026-04-24) tool-returned images display as a collapsed block the user must expand — [claude-code issue 53256](https://github.com/anthropics/claude-code/issues/53256)
- [DOC] Anthropic's announcement of 2026-07-28 says support is rolling out across Claude products "soon", without per-product dates; it notes Claude already renders MCP Apps UI and mentions MCP tunnels in research preview — [Claude blog 2026-07-28](https://claude.com/blog/bringing-mcp-2026-07-28-to-claude)

**Claude API (MCP connector)**
- [DOC] The connector (beta headers `mcp-client-2026-09-15`, `mcp-client-2025-11-20`) supports **tool calls only**, and the server must be reachable over public HTTPS (Streamable HTTP or SSE); local stdio servers are not connectable; `defer_loading` works with the tool search tool — [MCP connector docs](https://platform.claude.com/docs/en/agents-and-tools/mcp-connector)

### Inferences
- Budget one image per `render` call by default. At 4,784 tokens for a max-size image, five full-size images already approach Claude Code's 25,000-token cap, and how Claude Code estimates the token weight of an image is not documented.
- Base64 adds one third: a tool result limited to about 1 MB (Desktop/claude.ai, community figure) leaves room for roughly 700 KB of raw image. The API's 10 MB-encoded limit allows about 7.5 MB raw.
- Because localization is approximate and grid overlays did not help in Anthropic's tests, overlays should carry **numbered/labelled markers bound to store ids**, and the model should answer with ids, not coordinates. When coordinates are unavoidable, accept absolute pixels relative to a named render and convert server-side.
- Return the machine-readable companion data as a **text block** next to the image, not as `structuredContent`, until the Claude Code drop bug is confirmed fixed on the owner's version.
- Do not use `resource_link` as the only path to anything the model must see.
- A bench node on the LAN is reachable from Claude Code (HTTP on LAN or stdio via SSH) but not from the API connector or remote Claude apps without a public URL or tunnel.

### Gaps
- No documentation found for how Claude Code converts an image block into a token count against `MAX_MCP_OUTPUT_TOKENS`, nor what happens to an over-cap image result (the file-persist path is described only for results "with no image content"). Needs an empirical test on the owner's version.
- Fix status of issues 54737, 53453, 66519 and the same-turn image visibility report for Claude Desktop on Windows (issue 77338, title seen only) is unknown.
- No official statement of image limits for MCP tool results in Claude Desktop/web/mobile, nor of which 2026-07-28 features (MRTR, tasks) those apps support.
- ScreenSpot-Pro leaderboard numbers for current Claude models could not be extracted (the leaderboard table is dynamic; page last updated 2026-11-10) — [leaderboard](https://gui-agent.github.io/grounding-leaderboard)
- No published pointing-accuracy figure (in pixels or percent) for Claude 5.x on photos, maps or schematics was found.

---

## 2. Documents and diagrams as vision input

### Takeaway
Frontier models read ordinary tables from page images about as well as from text, but on hard pages table fidelity is only around 50 TEDS, schematic connectivity extraction is far below human, and the damaging errors are quiet ones (lost decimal point, flipped sign, wrong header-cell association, whole tables skipped). Native PDF blocks are an API/upload feature, not something an MCP tool result can deliver, and they cap page images at the standard tier, so the store should rasterize pages itself at about 196-199 DPI for a full page and send higher-DPI crops for tables.

### Cited Findings
- [DOC] Native PDF support converts every page to an image and supplies extracted text beside it; text costs roughly 1,500-3,000 tokens per page; **each page image is fitted to the standard tier (at most 1,568 visual tokens) on every model**; limit 600 pages per request (100 when the context window is under 1M tokens) — [Claude PDF docs](https://platform.claude.com/docs/en/build-with-claude/pdf-support)
- [DOC] PDFs enter through `document` content blocks (URL, base64 or Files API `file_id`); the docs describe no MCP route — [Claude PDF docs](https://platform.claude.com/docs/en/build-with-claude/pdf-support). Coordinates returned on native PDF pages cannot be mapped back reliably because raster size is not under caller control; Anthropic advises rasterizing pages yourself for coordinate work — [Claude coordinates docs](https://platform.claude.com/docs/en/build-with-claude/vision-coordinates)
- [COMMUNITY] PDF blobs returned through MCP resources were reported as not processed by Claude Code (v2.1.117) — [claude-code issue 53453](https://github.com/anthropics/claude-code/issues/53453)
- [DERIVED] Largest full-page rasters that avoid server-side resize: US Letter 1688x2184 px (about 199 DPI, 4,758 tokens) and A4 1624x2296 px (about 196 DPI, 4,756 tokens) on the high-res tier; Letter 952x1232 and A4 924x1306 (about 112 DPI, about 1,500 tokens) on the standard tier — computed from the formula in [Claude coordinates docs](https://platform.claude.com/docs/en/build-with-claude/vision-coordinates)
- Dr.DocBench (4,514 hard pages from 312 PDFs, mid-2026): overall scores GPT-5.5 61.94, Claude Opus 4.6 60.19, Gemini 3.1 Pro 60.13, versus the specialised parser MinerU 2.5 at 54.37; but on tables MinerU leads (TEDS 55.85 vs 51.26 Gemini, 49.21 Claude, 48.90 GPT) — [arXiv 2606.01393](https://arxiv.org/html/2606.01393)
- Same paper: Claude's reading-order edit distance worsens from 0.24 with one page in the window to 0.69 with fifteen; borderless tables and coloured backgrounds cut table scores; rotated text and multi-column layouts are harder — [arXiv 2606.01393](https://arxiv.org/html/2606.01393)
- PDF table-extraction benchmark (451 tables, March 2026): Gemini 3 Pro 9.55/10 and Gemini 3 Flash 9.50/10 lead; Claude Sonnet 4.6 was bimodal, often omitting a table entirely but extracting well when it did; meaning-changing errors observed include a lost decimal point and a flipped sign, and top models showed misaligned spanning cells, subtly altered values and wrong header-cell links — [arXiv 2603.18652](https://arxiv.org/html/2603.18652v1)
- TabVerse (June 2026, 700 questions): table QA from rendered images versus text is nearly identical for Gemini-3-Flash (about 65% either way); GPT-5.2 does 2-3 points better from text (57-58% vs 55-56%); structured text beats images on structure-centric tasks; HTML is the most robust text serialization, Markdown about equal for QA — [TabVerse, HF papers 2606.09578](https://huggingface.co/papers/2606.09578.md)
- OmniSch (1,854 real PCB schematics, 2026): zero-shot models reach symbol-detection F1 of 0.53-0.66 (Claude Sonnet 4.6: 0.586) but net-detection F1 of at most about 0.27 and graph IoU of at most 0.195, against human 0.987 and 0.870; box localization IoU is near zero for all LMMs (classical pipeline 0.88); lowering resolution costs 7-21 points of symbol F1 (Claude least, -7.3); giving the model a detector's bounding box to zoom into raises symbol-name F1 sharply (Claude 0.507 to 0.796) — [OmniSch, arXiv 2604.00270](https://arxiv.org/html/2604.00270v3)
- ViDoRe V3 (about 26,000 pages): with the same generator (Gemini 3 Pro), page-image context beats extracted-text context on hard questions by 2.4-2.8 points, and a hybrid of visual and textual retrieval is best (54.7% on hard queries, versus 64.7% with oracle pages) — [arXiv 2601.08620](https://arxiv.org/html/2601.08620v2)
- [DOC] The zoom-tool cookbook lists dense documents, schematics, scanned forms and label photos as cases where a crop-and-magnify step helps — [Claude cookbook: crop tool](https://platform.claude.com/cookbook/multimodal-crop-tool)

### Inferences
- Register maps, pinouts and wiring connectivity must not enter the store as trusted facts straight from a model's reading of an image. Treat model-read values as a low trust tier until cross-checked (PDF text layer, second independent read of a tight crop, checksum/row-count column, or the owner).
- The store should compute citation bounding boxes itself from the PDF text layer (character boxes) and never ask the model to supply them.
- One page (or one table crop) per image and per question; multi-page windows degrade reading order.
- For a table crop, pick DPI so the crop still fits 4,784 tokens: DPI_max is about sqrt(4784 x 784 / (width_in x height_in)); a 6.5 x 4 inch table can go to about 380 DPI, a half Letter page to about 280 DPI.
- Because the native PDF path gives the model at most about 112 DPI per page, self-rasterized pages at about 197 DPI carry roughly three times the pixels on current models.
- Offer the text layer of the same region in the same result (as HTML or Markdown table) so the model can compare modalities.

### Gaps
- No Anthropic statement of a recommended DPI for page images; the DPI figures above are derived from the token limits.
- No benchmark found specifically on register maps, connector pinout tables or vehicle wiring diagrams; OmniSch (PCB schematics) is the nearest.
- No 2026 result found for Claude 5.x-generation models on OmniDocBench-style parsing; the newest Claude in the cited tables is Opus 4.6 / Sonnet 4.6.
- No measured study of mitigations (two-pass verify, cell-by-cell re-read, checksum columns) was found; they remain engineering practice, not benchmarked results.

---

## 3. Map and layout rendering for an agent; Rust building blocks

### Takeaway
Published map benchmarks show counting, route finding, distance and fine local-scale reasoning as weak spots even for strong models, with marker-style maps the easiest kind, so the store should do geometry server-side and hand the model a simple labelled image plus the same features as text. The Rust ecosystem covers 2D rendering, tiles, vector geometry and PDF rasterization well; GeoTIFF/COG reading in pure Rust is young, and several libraries need a C dependency.

### Cited Findings

**How well models read maps**
- MapVerse (11,837 questions over 1,025 real maps, Feb 2026): best model Gemini 2.5 scored 80.0% on yes/no questions but 37.6% exact match on counting, 36.2% on single-entity answers and 49.0% on reasoning; marker maps were easiest (95.7% yes/no); models did better at continent/country scale than at county/district/building scale; downscaling the image cost up to about 50% of relative score — [arXiv 2602.10518](https://arxiv.org/html/2602.10518)
- CartoMapQA (2,251 questions on 853 OpenStreetMap maps, Dec 2025): best symbol-recognition accuracy 0.652 (Gemini 2.5 Pro preview); counting RMSE 1.367 at best; o3 found the shortest route in about 33% of cases; failure causes named are weak map semantics for line/area features, label OCR errors and limited route reasoning; Claude 3.7 Sonnet was the Claude tested — [arXiv 2512.03558](https://arxiv.org/html/2512.03558v1)
- PlanBench-V (223 planning maps, 1,629 questions, June 2026) reports the best 2026 model ahead of the best 2025 model by 27% but all models weak on tasks needing constraint-aware judgement — [arXiv 2606.05744](https://arxiv.org/abs/2606.05744)
- [DOC] Anthropic lists approximate localization and approximate counting of many small objects as standing limitations — [Claude vision docs](https://platform.claude.com/docs/en/build-with-claude/vision)

**Rust building blocks (crates.io, 2026-11-25)**
- 2D raster drawing: tiny-skia 0.12.0 — [crates.io](https://crates.io/crates/tiny-skia); SVG rendering: resvg 0.50.0 — [crates.io](https://crates.io/crates/resvg); image I/O: image 0.25.10 — [crates.io](https://crates.io/crates/image); image ops/drawing: imageproc 0.27.0 — [crates.io](https://crates.io/crates/imageproc); fast resize: fast_image_resize 6.2.0 — [crates.io](https://crates.io/crates/fast_image_resize)
- Text: fontdue 0.9.5 — [crates.io](https://crates.io/crates/fontdue); cosmic-text 0.19.0 (shaping/layout) — [crates.io](https://crates.io/crates/cosmic-text)
- Plots: plotters 0.3.7, last published 2024-09-08 (no release in over two years) — [crates.io](https://crates.io/crates/plotters)
- TIFF decode: tiff 0.11.3 — [crates.io](https://crates.io/crates/tiff); geotiff 0.1.0, last published 2025-06 with about 27k downloads (immature) — [crates.io](https://crates.io/crates/geotiff); async-tiff 0.4.0 (2026-09) — [crates.io](https://crates.io/crates/async-tiff); GDAL bindings gdal 0.19.0 (2025-12) — [crates.io](https://crates.io/crates/gdal)
- Tiles: pmtiles 0.28.1 (2026-11-13) — [crates.io](https://crates.io/crates/pmtiles); it reads and writes PMTiles v3, is Tokio-async with mmap/HTTP/S3/object_store backends, is MIT/Apache-2.0, and does not yet convert to/from MBTiles — [pmtiles-rs README](https://raw.githubusercontent.com/stadiamaps/pmtiles-rs/main/README.md)
- Vector/spatial: geo 0.33.1 — [crates.io](https://crates.io/crates/geo); rstar 0.14.0 (R-tree) — [crates.io](https://crates.io/crates/rstar); proj 0.32.0 (PROJ bindings) — [crates.io](https://crates.io/crates/proj); h3o 0.11.1 — [crates.io](https://crates.io/crates/h3o); geozero 0.15.1 — [crates.io](https://crates.io/crates/geozero)
- PDF: pdfium-render 0.9.6 (2026-11-21) — [crates.io](https://crates.io/crates/pdfium-render); it binds Pdfium dynamically at run time by default or statically via a feature, takes native prebuilt Pdfium from bblanchon/pdfium-binaries, serializes all calls behind a mutex because Pdfium is not thread-safe, renders pages to a target pixel width, and exposes character-level text queries such as `chars_inside_rect()` — [pdfium-render repo](https://github.com/ajrcarey/pdfium-render); mupdf bindings 0.8.0 — [crates.io](https://crates.io/crates/mupdf)
- Photos: kamadak-exif 0.6.2 — [crates.io](https://crates.io/crates/kamadak-exif); nom-exif 3.8.0 — [crates.io](https://crates.io/crates/nom-exif); perceptual hashing: image_hasher 3.1.1 (2026-02) is the maintained line, img_hash 3.2.0 has had no release since 2021 — [crates.io image_hasher](https://crates.io/crates/image_hasher), [crates.io img_hash](https://crates.io/crates/img_hash); libvips bindings 2.3.0 — [crates.io](https://crates.io/crates/libvips)
- 3D: gltf 1.4.1 (loader only), last published 2024-05 — [crates.io](https://crates.io/crates/gltf)
- Recordings: mcap 0.26.0 — [crates.io](https://crates.io/crates/mcap)
- MCP server SDK: rmcp 3.11.0 (2026-11-17) — [crates.io](https://crates.io/crates/rmcp)

**Region addressing and annotation formats**
- [DOC] IIIF Image API 3.0 URL shape: `{identifier}/{region}/{size}/{rotation}/{quality}.{format}`, applied in that order; region is `full`, `square`, `x,y,w,h` (pixels) or `pct:x,y,w,h`; size is `max`, `w,`, `,h`, `pct:n`, `w,h`, or `!w,h` (fit within), with `^` to allow upscaling; `info.json` advertises width, height, `maxWidth`/`maxArea`, preferred `sizes` and `tiles` with `scaleFactors` — [IIIF Image API 3.0](https://iiif.io/api/image/3.0/)
- [POINTER] W3C Web Annotation Data Model (targets with FragmentSelector `xywh=` or SvgSelector, body + motivation) — [W3C](https://www.w3.org/TR/annotation-model/); COCO region format (bbox `[x,y,w,h]`, polygon segmentation) — [COCO](https://cocodataset.org/#format-data); PMTiles v3 spec — [protomaps/PMTiles](https://github.com/protomaps/PMTiles/blob/main/spec/v3/spec.md); SQLite R*Tree module — [sqlite.org](https://www.sqlite.org/rtree.html); GeoPackage — [geopackage.org](https://www.geopackage.org/); Pdfium prebuilt binaries — [bblanchon/pdfium-binaries](https://github.com/bblanchon/pdfium-binaries); MuPDF licensing (AGPL or commercial) — [mupdf.com](https://mupdf.com/licensing)

### Inferences
- Render recipe supported by the evidence: marker-style map, north up, scale bar, one question per image, numbered pins with a text legend listing `pin -> id, name, lat/lon, local x/y in metres`; never colour-only encoding; do counts, distances, areas and bearings in Rust (`geo`) and print them, since counting and distance are the documented weak spots.
- Return the feature list as text beside the image (GeoJSON-like but compact, for example one line per feature). TabVerse and the map benchmarks together suggest the model reasons at least as well over the structured form for anything numeric.
- Zoomed insets beat one dense overview, because scores fall at local scale and with downscaling.
- A one-off orthomosaic is best pre-tiled once (on the Mac with GDAL or similar) into a PMTiles or tile-pyramid file that the Pi then reads with `pmtiles` + `image`; avoiding a GDAL/PROJ runtime dependency on the Pi keeps cross-compilation simple. For a single site, a local metric frame with a stored affine transform avoids `proj` entirely.
- Adopt IIIF's `region/size` vocabulary as the argument grammar for `render` crops and as the form of evidence pointers (`blob@hash#page=12&xywh=...`); it is a clean, already-specified interface.
- Headless glTF rendering on a Pi has no ready, proven pure-Rust path in the crates surveyed (`gltf` only loads); pre-render a few fixed views on the Mac and store them as images.
- `plotters` works but is stagnant; multi-panel signal plots can be drawn directly with tiny-skia plus a text crate, or as SVG rendered by resvg.

### Gaps
- No benchmark found for site plans, solar array layouts, floor plans or drone orthomosaics with current Claude models; FloorplanQA (arXiv 2507.07644) was located but not read.
- Licences of Pdfium (BSD-style/Apache) and of the pdfium-render crate, and availability of Linux aarch64 prebuilt Pdfium, were not confirmed this session (the repo page did not state them).
- Whether `tiff`/`async-tiff` handle the specific compression and tiling of the owner's orthomosaic export is untested.
- No source located on realistic performance of software 3D rendering on Pi-class hardware.

---

## 4. Agent-facing data tool design (AX)

### Takeaway
Vendor guidance and measured results agree: a small set of consolidated, well-described tools with response-size controls, semantic ids, clear continuation handles and corrective error text; under about 20 tools loaded at once; batching and code/SQL execution cut tokens by a third to more than 90% in the published cases. Compact notations such as TOON are not a safe default; Markdown or HTML tables are.

### Cited Findings
- Anthropic's tool-writing guide: offer a `response_format` enum (their example: 206 tokens detailed vs 72 concise); paginate, filter and truncate with sensible defaults; when truncating, tell the agent how to narrow; return semantic names instead of UUIDs (replacing UUIDs with names or 0-indexed ids reduced hallucination; no number given); consolidate chains into one tool (for example one `get_customer_context` in place of three calls); namespace by prefix; write errors that say what to fix and show a correct example; evaluate with dozens of realistic multi-call tasks and held-out sets — [Anthropic: writing tools for agents](https://www.anthropic.com/engineering/writing-tools-for-agents)
- Anthropic's advanced tool use post: 58 tools from five servers cost about 55K tokens of definitions; tool search cut definition tokens by 85% and raised MCP-eval accuracy from 49% to 74% (Opus 4) and 79.5% to 88.1% (Opus 4.5); it pays off above about 10 tools or 10K tokens of definitions, keeping 3-5 core tools always loaded; programmatic tool calling cut average tokens on research tasks from 43,588 to 27,297 (37%); adding input examples to tool definitions raised complex-parameter accuracy from 72% to 90% (1-5 examples per tool) — [Anthropic: advanced tool use](https://www.anthropic.com/engineering/advanced-tool-use)
- Code execution with MCP: letting the agent filter and join data in code took one workflow from about 150,000 tokens to about 2,000 (98.7%); cost is the need for a sandbox — [Anthropic: code execution with MCP](https://www.anthropic.com/engineering/code-execution-with-mcp)
- Context engineering post: sub-agents hand back condensed summaries of roughly 1,000-2,000 tokens; agents should hold lightweight identifiers and load data just in time; bloated or overlapping tool sets are a common failure — [Anthropic: effective context engineering, 2025-09-29](https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents)
- OpenAI's function-calling guide: aim for fewer than 20 functions available at the start of a turn (soft guideline), merge functions always called in sequence, do not make the model supply arguments the code already knows, use enums to make invalid states unrepresentable — [OpenAI function calling guide](https://developers.openai.com/api/docs/guides/function-calling)
- Microsoft Research survey of 1,470 MCP servers (Sept 2025): median tool response 98 tokens but mean 4,431 and maximum 557,766; 16 tools overflow a 128K window in one call; 775 tool names collide across servers ("search" on 32 servers); of 5,983 results not flagged as errors, 3,536 were judged to contain errors in their text; only 4 tools returned a resource link; a cited study found flattening nested parameters improved tool calling by 47% — [Microsoft Research: tool-space interference](https://microsoft.com/en-us/research/blog/tool-space-interference-in-the-mcp-era-designing-for-agent-compatibility-at-scale)
- Amplitude consolidated a 96-tool MCP server (which alone consumed 57% of a 200K window on connect) into object-plus-action tools (cohort tools 9 to 1, chart tools 16 to 4) and moved long instructions into on-demand skills; agents filled an optional "rationale" parameter about three quarters of the time — [Amplitude engineering blog](https://amplitude.com/blog/lessons-from-consolidating-our-mcp-server.md)
- [DOC] Claude Code specifics that bound description size: 4,096 characters for server instructions and upfront tool descriptions — [Claude Code MCP docs](https://code.claude.com/docs/en/mcp)
- Format study (June 2026; open models Qwen3-32B, Mistral-Small-24B, not frontier Claude): TOON saved 2-18% of tokens on average with input-only compression but lost about 1-9 points of accuracy on average, with one collapse from 89% to 53%; parallel tool calls broke when calls were emitted in TOON; the authors advise against TOON as a default in multi-turn agents, since two or three parse-failure retries erase the savings — [Notation Matters, arXiv 2605.29676](https://arxiv.org/html/2605.29676)
- Table serialization: HTML most robust, Markdown about equal for QA, LaTeX worst — [TabVerse](https://huggingface.co/papers/2606.09578.md)
- Text-to-SQL: on a small hand-picked set of eight BIRD queries, Claude Opus 4.5 and 4.6, Gemini 3 Pro/Flash and GLM-5 scored execution accuracy 1.00 (GPT-5.2 0.69); across 930 wrong translations the dominant error was returning superfluous columns (93% of output-format errors) and MAX/COUNT confusion (44% of aggregation errors); the authors caution it is not a full benchmark — [arXiv 2602.21480](https://arxiv.org/html/2602.21480v3)
- [DOC] MCP spec guidance for stateful tools: return an explicit opaque handle, state its lifetime in the tool description, return a clear tool error on expiry — [MCP tools spec](https://modelcontextprotocol.io/specification/2026-07-28/server/tools)

### Inferences
- The on-paper tool count (nine tools plus case files) sits inside every published comfort zone (under 20; near the 10-tool threshold where tool search starts to matter). Keep it there: fold case-file operations into `commit`/`get`/`find` types rather than adding tools.
- Every read tool should take `max_tokens` (or `detail: concise|detailed`) and end with a one-line footer: what was omitted, the continuation cursor, and the exact next call.
- Input examples in descriptions are worth their tokens for `commit` (complex nested operations) per the 72% to 90% result, but each upfront description must stay under 4,096 characters in Claude Code.
- Keep `commit` operation objects flat (the 47% flattening result) and enum-typed.
- Read-only SQL over projections is justified by the code-execution and text-to-SQL evidence for small schemas; guard against the known "extra columns" habit with a row and byte cap and by echoing column names and row count.
- For Claude Code specifically, offering a downloadable read-only snapshot (SQLite or Parquet over HTTP) lets the agent analyse in its own shell without routing rows through the context, which is the code-execution pattern.

### Gaps
- No controlled study found comparing "one SQL tool" against bespoke query tools on the same data, or "few large tools" against "many small tools" beyond the vendor numbers above.
- No token/accuracy comparison of CSV, TSV, Markdown and JSON on frontier Claude models was found; the TOON study used mid-size open models.
- Google's equivalent tool-design guidance was not retrieved.

---

## 5. Atomic batch writes for agents

### Takeaway
The best-fitting prior art is a hybrid: JSON:API Atomic Operations for all-or-nothing ordered batches with local ids, JMAP for state strings (`ifInState`), back-references and a change feed, and Stripe for idempotency keys that replay the stored outcome. MCP 2026-07-28 makes retries a normal event (a dropped stream must be re-issued), offers MRTR for human confirmation and a tasks extension for long jobs, but neither can be assumed on every Claude client, so propose/apply must work as two plain tool calls.

### Cited Findings
- JSON:API Atomic Operations: an ordered `atomic:operations` array of `{op: add|update|remove, ref|href, data}`; `lid` local ids let later operations refer to resources created earlier in the same request; processing is in order and **all-or-nothing**; success returns `atomic:results` aligned by position; failures return an `errors` array whose `source.pointer` identifies the offending operation — [JSON:API atomic extension](https://jsonapi.org/ext/atomic/)
- JMAP (RFC 8620): a request is an ordered list of `[method, args, callId]`; arguments prefixed `#` are back-references (`resultOf`, `name`, `path` as JSON Pointer) to earlier results; creation ids (`#tempId`) name records created earlier in the request; `/set` takes `ifInState` and fails with `stateMismatch` if the server state differs, and returns `oldState`/`newState` plus `created`/`updated`/`destroyed` and `notCreated`/`notUpdated`/`notDestroyed` maps of typed `SetError`s; **atomicity is per record, not per call**; `/changes` takes `sinceState` and returns id lists with `hasMoreChanges`, or `cannotCalculateChanges`; suggested minimum limits: 16 calls per request, 500 objects per `/set` and per `/get`, 10 MB request — [RFC 8620](https://datatracker.ietf.org/doc/html/rfc8620)
- Stripe idempotency: client-generated key up to 255 characters (UUIDv4 suggested); the first response for a key is stored once execution begins and replayed on retry, including 500 errors; a retry with different parameters is rejected; keys may be pruned after 24 hours; validation failures and concurrent-conflict rejections are not stored and may be retried — [Stripe API: idempotent requests](https://docs.stripe.com/api/idempotent_requests)
- [DOC] MCP 2026-07-28 removed SSE resumability and redelivery: a broken response stream loses the in-flight request and the client MUST re-issue it with a new request id — [MCP changelog](https://modelcontextprotocol.io/specification/2026-07-28/changelog)
- [DOC] MRTR: a `tools/call` may return `resultType: "input_required"` with `inputRequests` (elicitation form/url, sampling, roots) and an opaque `requestState`; the client retries the same call with `inputResponses` and the echoed state; servers MUST integrity-protect `requestState` when it affects logic, SHOULD bind it to principal, short expiry and a digest of the original request, MUST NOT assume the client will retry, and must enforce single-use themselves if needed — [MCP MRTR spec](https://modelcontextprotocol.io/specification/2026-07-28/basic/patterns/mrtr)
- [DOC] Sampling, Roots and Logging are deprecated in 2026-07-28 (twelve-month minimum window); the suggested replacement for sampling is calling a model provider API directly — [MCP changelog](https://modelcontextprotocol.io/specification/2026-07-28/changelog)
- [DOC] Tasks moved to the extension `io.modelcontextprotocol/tasks`: server may answer a call with a task handle (`taskId`, `status` in working/input_required/completed/failed/cancelled, `ttlMs`, `pollIntervalMs`); the client polls `tasks/get`, answers input via `tasks/update`, and `tasks/cancel` is best-effort; a tool result with `isError: true` still counts as `completed` — [AAIF: MCP tasks extension](https://aaif.io/blog/handle-long-running-mcp-tool-calls-with-the-mcp-tasks-extension); [MCP changelog](https://modelcontextprotocol.io/specification/2026-07-28/changelog)
- [DOC] Claude Code: tasks in public beta from v2.1.304; elicitation only in interactive sessions (non-interactive sessions ask the server to cancel a task that requests input); HTTP per-request timer is at least 60 s; calls auto-background after 2 minutes — [Claude Code MCP docs](https://code.claude.com/docs/en/mcp)
- [DOC] Tool annotations (read-only, destructive, idempotent, open-world hints) exist but clients must treat them as untrusted unless the server is trusted — [MCP tools spec](https://modelcontextprotocol.io/specification/2026-07-28/server/tools)
- [COMMUNITY, low confidence] A survey titled "I checked 489 MCP tools" (Oct 2026) is quoted in its comments as finding that none of 266 effectful tools accepted an idempotency key, and it shows a reproduced case where one intended GitHub call produced two issues; the article body could not be read — [hashnode post](https://hashnode.com/posts/what-happens-when-an-ai-agent-retries-a-tool-call-i-checked-489-mcp-tools/6acbe68ba40edb966d58f88a)
- "Resilient Write" (April 2026) documents an agent retrying an identical rejected write five times with no structured error to branch on, and proposes typed error envelopes that mark non-retriable failures, temp-file + fsync + hash-verify + rename writes, and hand-off envelopes with file hashes; its probability figures are estimates, not controlled trials — [arXiv 2604.10842](https://arxiv.org/html/2604.10842v1)
- [POINTER] Kubernetes server-side apply (field managers in `managedFields`, HTTP 409 conflicts naming the owning manager, `force`, `dryRun=All`) — [kubernetes.io](https://kubernetes.io/docs/reference/using-api/server-side-apply/) (page body truncated on fetch; details from prior knowledge); Datomic `:db/cas` compare-and-swap and transaction functions — [docs.datomic.com](https://docs.datomic.com/transactions/transaction-functions.html)

### Inferences
- Recommended `commit` semantics: ordered, flat, typed operations; all-or-nothing (JSON:API), not JMAP's per-record partial success, because an append-only chain entry should be a single unit; per-operation `expected_version` plus optional whole-store `if_in_state`; `lid` local ids for intra-batch references; errors as a list of `{op_index, code, message, current_version, fix_hint}` with nothing applied.
- Idempotency: key stored in the chain entry itself and kept forever (no 24-hour pruning needed at this scale); replay returns the original result and chain position; same key with a different payload hash is a hard error. This is required, not optional, given MCP's re-issue rule and Claude Code's backgrounding and timeouts.
- Dry-run should return a short plan: counts by operation type, per-operation one-line diffs (old to new with units), conflicts, trust-tier changes, and a `plan_token` (hash over the operations and base state). Apply is then `commit(plan_token=...)`, which costs few tokens and guarantees that what was reviewed is what is written (Terraform-style); the token goes stale if state moved.
- Human approval: use MRTR elicitation when the client declares the capability, but treat it as an enhancement; the baseline is propose (dry-run) then apply in two calls, with the owner approving in chat. `requestState` can carry the HMAC-signed plan hash.
- Long ingests: return an explicit job handle as an ordinary result and a poll tool path that works everywhere; additionally return a task handle when the client advertises the tasks extension.
- Because sampling is deprecated, the server cannot plan on borrowing the client's model for `ingest`; extraction must be deterministic server code, a direct model-API call configured by the owner, or the connected agent itself working through `render` and `commit`.

### Gaps
- No peer-reviewed measurement of how often current agents double-apply writes after timeouts was found; evidence is anecdotal.
- MRTR/elicitation and tasks support in Claude Desktop/web/mobile is not documented in the sources read.
- Whether rmcp 3.11.0 implements MRTR and the tasks extension was not verified.
- Git index/commit, Fossil/Dolt, GraphQL mutations, Notion/Linear/Airtable batch endpoints and CRDT change sets were not researched this session.

---

## 6. Agent working memory in a store

### Takeaway
Persisted structured notes across sessions are well supported by Anthropic's own results and harness write-ups (status in rigid JSON, progress log, read-first routine). Evidence that forcing an explicit hypothesis-prediction ritual improves outcomes is weak to negative, so case files should be light, typed and cheap, and scored automatically rather than relying on the model's discipline.

### Cited Findings
- Memory tool plus context editing improved an internal agentic-search evaluation by 39% over baseline (context editing alone 29%); in a 100-turn web-search evaluation context editing cut token use by 84% — [Claude blog: context management, 2025-09-29](https://claude.com/blog/context-management)
- The memory tool (`memory_20250818`) is client-side file operations (view, create, str_replace, insert, delete, rename) under `/memories`; when enabled the API injects a protocol telling the model to view memory before anything else, record progress as it goes, and assume its context may be reset at any moment; guidance includes capping file sizes, paging long views, expiring stale files and validating paths — [Claude memory tool docs](https://platform.claude.com/docs/en/agents-and-tools/tool-use/memory-tool)
- The documented multi-session pattern: an initializer session creates a progress log and a feature checklist; every later session starts by reading them and ends by updating them; mark an item done only after end-to-end verification — [Claude memory tool docs](https://platform.claude.com/docs/en/agents-and-tools/tool-use/memory-tool)
- Long-running harness write-up: without such state, agents ran out of context mid-feature, left undocumented half-work, declared the project finished early, or marked features done without testing; the fix was a 200+ item feature list in **JSON, where agents may only flip a `passes` flag** (JSON was chosen because models are less prone to rewriting it than Markdown), a progress file, git commits and a fixed session-start routine; compaction alone did not pass clear hand-off instructions — [Anthropic: effective harnesses for long-running agents, 2025-11-26](https://www.anthropic.com/engineering/effective-harnesses-for-long-running-agents)
- Structured note-taking let an agent keep exact tallies across thousands of steps and resume after context resets (the Pokemon example) — [Anthropic: effective context engineering](https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents)
- Counter-evidence: a preregistered ablation of a scientific-method agent loop (hypothesis, prediction, code, test, reconcile) found seven of ten loop variants did not beat a blind-sampling baseline, the full scaffold scored below blind sampling at 7-8x the token cost, and a bare prompt won on score per token; single author, one task domain (DNA motif discovery) — [The Little Scientist, arXiv 2608.16951](https://arxiv.org/abs/2608.16951)
- A "Hypothesis Evolution Protocol" harness that records hypothesis generation, testing and belief updates as explicit auditable operations reports qualitative benefits on materials-science tasks, growing with base-model capability; no numbers in the abstract — [arXiv 2607.09195](https://arxiv.org/abs/2607.09195)
- Hand-off envelopes carrying task state and file hashes so a new agent can resume and detect drift are proposed in — [Resilient Write, arXiv 2604.10842](https://arxiv.org/html/2604.10842v1)

### Inferences
- Case files are justified as a recovery and audit mechanism (resume, hand-off, "what did we already rule out"), not as a proven accuracy booster.
- Shape them like the harness feature list: fixed-schema records where routine updates only flip status fields or append results; free-text notes are a separate append-only field. This limits accidental rewrites.
- `orient` should surface open case files and their next step first, mirroring the "read memory before anything else" protocol.
- Predictions should be stored as typed facts with an explicit check (`predicate`, expected value/range, how to measure); the server marks them hit or miss when a matching result is committed, giving calibration data for free without extra agent effort.
- Keep the ritual optional: the Little Scientist result argues against mandatory multi-step scaffolds.

### Gaps
- No study found that scores explicit LLM predictions for calibration in equipment diagnosis or debugging.
- Letta/MemGPT memory-block evidence and Claude Code CLAUDE.md/auto-memory documentation were not retrieved this session.
- No comparison found of database-backed agent memory against file-based memory.

---

## 7. Scale and engine choices on a Pi-class box with NVMe

### Takeaway
SQLite through rusqlite, with content-addressed files beside it, covers the stated scale with wide margin on documented limits; the cliffs are in vector search (brute force past roughly a million vectors, late-interaction storage), single-writer design, and C dependencies, not in fact or document counts.

### Cited Findings
- SQLite limits: maximum database about 281 TB at 64 KiB pages, about 17.5 TB at the default 4 KiB page; default maximum BLOB/row 1 GB (hard ceiling about 2 GB); up to 125 attached databases; the size ceiling is untested by the developers for lack of hardware — [sqlite.org limits](https://www.sqlite.org/limits.html)
- Internal versus external blobs: reads are faster from inside the database below roughly 100 KB and faster from files above it, with 8 KiB or 16 KiB pages best for blob work; measured in 2011 on SATA with SQLite 3.7.8, and the page advises re-measuring on target hardware — [sqlite.org intern-v-extern-blob](https://www.sqlite.org/intern-v-extern-blob.html)
- Crate status (crates.io, 2026-11-25): rusqlite 0.43.0 (2026-11-15) — [crates.io](https://crates.io/crates/rusqlite); redb 5.0.0 (2026-11-18) — [crates.io](https://crates.io/crates/redb); fjall 3.1.13 (2026-10-25) — [crates.io](https://crates.io/crates/fjall); heed (LMDB) 0.23.1 — [crates.io](https://crates.io/crates/heed); rocksdb 0.25.0 — [crates.io](https://crates.io/crates/rocksdb); sled: last stable 0.34.7, still on 1.0.0-alpha builds with no publish since 2024-10 — [crates.io](https://crates.io/crates/sled); tantivy 0.27.0 (2026-10-17) — [crates.io](https://crates.io/crates/tantivy); lancedb 0.41.0 (2026-10-29) — [crates.io](https://crates.io/crates/lancedb); duckdb 1.20001.0 (2026-11-20) — [crates.io](https://crates.io/crates/duckdb); sqlite-vec stable 0.1.9 (2026-05), newest 0.1.10-alpha.4 — [crates.io](https://crates.io/crates/sqlite-vec); usearch 2.26.6 — [crates.io](https://crates.io/crates/usearch); hnsw_rs 0.3.5 — [crates.io](https://crates.io/crates/hnsw_rs); blake3 1.8.7 — [crates.io](https://crates.io/crates/blake3); zstd max stable 0.14.5 — [crates.io](https://crates.io/crates/zstd); zeekstd (seekable zstd) 0.6.3 — [crates.io](https://crates.io/crates/zeekstd)
- SQLite vector search state (article of Sept 2025 by the author of a competing extension, so read with that interest in mind): sqlite-vec is brute force over virtual tables; on 100,000 vectors of 384 dimensions on an M1 Pro a full scan took 67.84 ms with sqlite-vec, and 3.97 ms with an 8-bit-quantized, preloaded scan in sqlite-vector; libsql's DiskANN index build did not finish in hours — [The state of vector search in SQLite](https://marcobambini.substack.com/p/the-state-of-vector-search-in-sqlite)
- [POINTER] SQLite WAL — [sqlite.org/wal](https://www.sqlite.org/wal.html); FTS5 — [sqlite.org/fts5](https://www.sqlite.org/fts5.html); `PRAGMA query_only` — [sqlite.org/pragma](https://www.sqlite.org/pragma.html#pragma_query_only); authorizer callback — [sqlite.org/c3ref/set_authorizer](https://www.sqlite.org/c3ref/set_authorizer.html)

### Inferences
- At 10^5-10^7 facts, thousands of documents and 10^4-10^6 blob records, the SQLite file stays in the low tens of GB at most; hundreds of GB live in the blob directory. Both are orders of magnitude inside documented limits.
- Boring, robust combination: one SQLite database (WAL, `synchronous=FULL` for the chain, FTS5 and R*Tree as derived tables, JSON columns for operation payloads) as the system of record; blobs as immutable files named by BLAKE3 hash with two-level directory fan-out (`ab/cd/abcd...`), written temp + fsync + rename; thumbnails and small crops under about 100 KB may live in SQLite; every index (FTS, R*Tree, vectors, tiles) is rebuildable from chain + blobs.
- Replication to the Pi 5 then reduces to shipping chain entries in order and rsync-like copying of missing hash-named files; verification is re-hashing.
- Vectors: brute force (sqlite-vec or a plain BLOB column scanned in Rust) is fine to about 10^5 single vectors; plan a usearch/HNSW file as a derived index before 10^6. Expect a Pi to be several times slower than the M1 Pro figure above (assumption, unmeasured).
- Skip sled (never reached 1.0). redb and fjall are healthy but would add a second storage engine with no need. RocksDB adds a heavy C++ build. LanceDB and DuckDB are better placed on the Mac for analytics over exports than in the Pi's write path.
- Bit-rot: the application hash is the check that travels with replication; a scheduled re-hash ("scrub") that records its result as a chain entry gives detection on ext4; a checksumming filesystem adds repair only with redundancy.
- MCAP recordings: keep as blobs, index time ranges and channels in SQLite; seekable zstd (`zeekstd`) suits large text-like blobs needing range reads.

### Gaps
- No measured SQLite write throughput on Raspberry Pi/CM-class hardware with NVMe was found.
- LanceDB and DuckDB memory footprint and build status on aarch64 Linux were not verified.
- No sourced comparison of ext4, btrfs and ZFS on Pi-class boards; no sourced figures for tantivy versus FTS5 at this scale.
- The duckdb crate's unusual version number (1.20001.0) was not mapped to a DuckDB release.

---

## 8. Multimodal retrieval

### Takeaway
For manuals, page-image retrieval beats text retrieval before reranking, text retrieval with a strong reranker catches up, and a hybrid feeding page images to the reader is best; late-interaction page embeddings cost about 256 KiB per page unless pooled to about 8 KiB. At a few thousand manual pages plus photos, hybrid FTS + one dense vector per page/photo + captions written at ingest is enough, with embedding done on the Mac.

### Cited Findings
- ViDoRe V3: best visual retriever (ColEmbed-3B-v2) averages NDCG@10 59.8 versus 51.0 for the best text retriever (Qwen3-8B); late-interaction beats single-vector; a text reranker (zerank-2) adds 13.2 points to a text pipeline and makes it the top pipeline, while a visual reranker adds 0.2; hybrid top-5 visual + top-5 text gives the best end-to-end answers on hard queries (54.7%) — [arXiv 2601.08620](https://arxiv.org/html/2601.08620v2)
- Storage: ColPali v1.3 emits 1,024 patch vectors of 128 dimensions per page, ColQwen2.5 about 743; mean-pooling to 32 vectors per page keeps NDCG@5/@10 within about 0.01 of baseline for those models; a two-stage search (pooled first, full rerank) is 3.8-4.5x faster at a recall@100 cost of 0.02-0.09 — [Visual RAG Toolkit, arXiv 2602.12510](https://arxiv.org/html/2602.12510)
- [DERIVED] At 16-bit floats that is about 256 KiB per ColPali page (about 262 MB per 1,000 pages) unpooled and about 8 KiB per page pooled — from the vector counts in [arXiv 2602.12510](https://arxiv.org/html/2602.12510)
- Apple Silicon: the mlx-embeddings project lists SigLIP, ColQwen2.5 (late interaction), Qwen3-VL-Embedding-2B and a Llama Nemotron VL embedder as runnable under MLX; no throughput figures given — [mlx-embeddings](https://github.com/Blaizzy/mlx-embeddings)
- Core ML ports of SigLIP 2 base (patch16, 256 px) are published on Hugging Face — [example model card](https://huggingface.co/karaiman/siglip2-base-coreml)
- Rust inference runtimes: ort 2.0.0-rc.15 (ONNX Runtime bindings, still no stable 2.0) — [crates.io](https://crates.io/crates/ort); candle-core 0.11.1 — [crates.io](https://crates.io/crates/candle-core)

### Inferences
- For a few thousand manual pages, pooled late-interaction vectors (about 8 KiB per page) would total tens of MB, so storage is not the blocker; the blocker is running a 2-3B-parameter embedder, which belongs on the Mac at ingest time, not on the Pi. Queries then need a query-side encoder too, so the simpler default is FTS5 over the text layer and captions plus one dense vector per page/photo, with late interaction as a later derived index.
- Captioning each photo and page once with the frontier model at ingest (stored as text with trust tier "model caption") makes images findable by exact terms the owner will use ("12-pin Deutsch connector, green/white wire"), which pure image embeddings handle poorly; it is also portable to the Pi with no model.
- "Retrieve the page image and let the model read it" is supported for hard questions by ViDoRe V3, but only by 2-3 points over text; send both the crop and its text layer.

### Gaps
- No throughput numbers found for SigLIP 2, CLIP or ColQwen on Apple Silicon (MLX, Core ML or ONNX).
- Jina CLIP/v4, Nomic Embed Vision/Multimodal and Cohere Embed v4 were not researched this session.
- Audio embeddings (CLAP or similar) for "sounds like this" search were not researched; no source located.
- No published comparison of ingest-time captioning versus embedding for photo retrieval was found.

---

## 9. Synthesis requested for the design

### Takeaway
The on-paper tool set is sound in number and shape; the corrections are about what clients really deliver (image-only-plus-text results, no reliance on resource links, structuredContent, sampling or elicitation), about never trusting model-read diagram values without a cross-check, and about making retries harmless.

### Cited Findings
(All facts below are restated from sections 1-8, where sources are given; the key ones are repeated with links.)
- Image token formula, tiers and caps — [Claude vision docs](https://platform.claude.com/docs/en/build-with-claude/vision)
- Claude Code result caps and image handling — [Claude Code MCP docs](https://code.claude.com/docs/en/mcp)
- Content types, MRTR, deprecations — [MCP tools spec](https://modelcontextprotocol.io/specification/2026-07-28/server/tools), [MCP changelog](https://modelcontextprotocol.io/specification/2026-07-28/changelog)
- Zoom tool gain (29% to 73%) — [Claude cookbook](https://platform.claude.com/cookbook/multimodal-crop-tool)
- Schematic connectivity gap — [OmniSch](https://arxiv.org/html/2604.00270v3)

### Inferences

#### (1) Confirmed facts the design can rely on
1. Tool results can carry several text and image blocks; Claude Code shows PNG/JPEG/GIF/WebP tool images to the model as vision input and (v2.1.283+) saves the original to disk.
2. An image costs `ceil(w/28) x ceil(h/28)` tokens; Claude 4.7+ models take up to 2,576 px long edge and 4,784 tokens per image; older/standard-tier models 1,568 px and 1,568 tokens; oversize images are silently downscaled.
3. Claude Code: 10,000-token warning, 25,000-token default cap (`MAX_MCP_OUTPUT_TOKENS`), images count toward it; text over 50,000 characters goes to a file unless the tool declares `anthropic/maxResultSizeChars` (ceiling 500,000); 16 MiB HTTP response ceiling; 4,096-character limit on instructions and upfront descriptions.
4. Claude wants absolute pixel coordinates and its localization is approximate; cropping/zooming from the full-resolution original is the documented remedy and has large measured gains.
5. MCP 2026-07-28 is stateless: cursors, plan tokens, job ids and case ids must be explicit arguments. A dropped stream means the client re-issues the call.
6. MRTR (`input_required`) replaces server-initiated elicitation; tasks are an extension; sampling is deprecated.
7. The API MCP connector does tools only and needs a public HTTPS server.
8. Native PDF input is an API/upload feature capped at standard-tier page images; it is not available through MCP tool results.
9. SQLite's documented limits exceed the stated scale by orders of magnitude; blobs above about 100 KB are better as files.
10. Fewer than 20 tools, consolidated by outcome, with response-size knobs and corrective errors, matches Anthropic, OpenAI and field reports.

#### (2) Corrections and risks, tool by tool
- **orient**: Good. Model it on JMAP `/changes`: take `since` cursor, return counts and a bounded list, `has_more`, and an explicit "cannot calculate, re-brief" answer for cursors that are too old. Put open case files and stale-fact warnings first. Keep under about 1,500 tokens by default.
- **brief**: Good. Add `max_tokens` and `detail` parameters; default near the 1,000-2,000-token condensed-summary range; use names plus short stable ids, not bare UUIDs; state the store state string it reflects; end with next-call hints.
- **find**: Keep as one tool with a `mode` (key, text, semantic, hybrid). Return at most about 10 hits with id, title, one-line snippet, trust tier and age; cursor for more; on truncation say how to narrow. Hybrid text + vector should be the default for manuals. Do not promise late-interaction search on the Pi.
- **get**: Fine. Enforce a token budget across the batch (not just an id count), return per-id "omitted, call again with..." markers rather than failing, and never inline binaries; binaries go through `render`.
- **render**: The centrepiece, with the most corrections. (a) Return one image by default plus a text legend; do not attach `structuredContent` to image results (reported Claude Code drop bug); do not rely on `resource_link`. (b) Pre-size to exact model-processed dimensions and print the pixel size and the pixel-to-world transform in the text block. (c) Accept `tier: standard|high` (default high; the server cannot see which model is connected). (d) Make zoom first-class: every render returns a `render_id`; `render(zoom_of, region)` crops from the original at full resolution. (e) Do not spend effort on grid overlays for localization; use numbered pins bound to ids, and let the model answer with ids. (f) Do counting and measuring server-side and print results. (g) For manual pages rasterize yourself (about 197 DPI full page; higher for crops) and include the text layer of the same region. (h) Image-token accounting against Claude Code's cap is undocumented: test, and keep a fallback where the image is fetched over plain HTTP by URL. (i) Keep raw bytes under about 700 KB for Desktop/claude.ai compatibility (community-reported 1 MB result limit).
- **commit**: Good core. Make it all-or-nothing with local ids (JSON:API atomic), `expected_version` per operation and optional `if_in_state` (JMAP), idempotency key recorded in the chain entry and replayed forever with payload-hash check (Stripe), flat enum-typed operations with 1-3 examples in the description, errors listed per operation index with current version and a fix hint. Dry-run returns a compact plan plus `plan_token`; apply by token. Do not depend on elicitation or tasks for correctness.
- **ingest**: Highest risk. The server has no model of its own (sampling is deprecated), so "turn a PDF into proposed facts" must be deterministic extraction (text layer, CSV parse, EXIF) plus agent review through `render`, or an owner-configured direct model call. Model-read table values have documented silent error modes; schematic connectivity extraction is far below human. Proposed facts should enter at a "machine-extracted, unverified" trust tier with evidence boxes computed by the server; promotion requires a cross-check. Long ingests need a job handle that works without the tasks extension.
- **preflight**: Useful and cheap; keep it read-only and compact (per fact: current/stale/superseded/retracted, trust tier, age, last verification). Also run the same check inside `commit` and before any action tool, so skipping preflight cannot cause harm. Offer `render(evidence_of=fact)` for a visual double-check of critical values.
- **query**: Supported for small schemas. Open the connection read-only, enforce `query_only`, an authorizer allow-list, a statement timeout, and row/byte caps; return Markdown with column names, row count and a truncation note; publish the schema of the projections (short) in the description or via a `describe` mode. Expect superfluous columns and occasional aggregate mix-ups from the model. For Claude Code, also offer a read-only snapshot download for local analysis.
- **case files**: Keep, but as typed records inside the existing tools, not new tools. Rigid schema with status flips and appended results (the JSON feature-list lesson); surfaced first by `orient`; predictions stored as checkable facts and auto-scored on commit. Do not make a hypothesis ritual mandatory; evidence that it helps is weak to negative.

#### (3) Recommended defaults
- **Image sizes** (all [DERIVED] from the documented formula):
  - "Safe" default that no Claude model resizes: 1344x896 or 1456x819 landscape, 1092x1092 square (about 1,500-1,560 tokens).
  - Detailed site map / annotated wiring photo on the high tier: 2212x1659 (4:3) or 2576x1449 (16:9) or 1932x1932 (about 4,740-4,784 tokens).
  - Manual page, full: Letter 1688x2184, A4 1624x2296 (about 197 DPI, about 4,757 tokens); standard tier 952x1232 / 924x1306.
  - Manual crop at a cited box: scale so `ceil(w/28) x ceil(h/28)` is at most 4,784 and the long edge at most 2,576; cap at about 300-400 DPI.
  - Multi-panel signal plot: 1456x819 by default, with a numeric summary table in text.
  - Contact sheet of thumbnails: one 1344x896 image with labelled cells.
- **Formats**: PNG for pages, line art, maps with labels and plots; JPEG quality about 90-92 without chroma subsampling for photos and orthomosaic crops; WebP acceptable. One image per result by default, three at most. Images first, text after; label multiple images.
- **Page-raster DPI**: about 197 DPI for a full page on current models; about 112 DPI is all a standard-tier model or the native PDF path can use; crops up to 300-400 DPI.
- **Response token budgets**: orient at most 1,500; brief default 1,500 (max parameter up to about 6,000); find 10 hits, about 1,000-2,000 tokens; get default budget 4,000; query default 50 rows / 4,000 tokens; everything under the 10,000-token Claude Code warning unless the caller raises a limit; never above 25,000; text under 50,000 characters unless `anthropic/maxResultSizeChars` is declared for that tool.
- **Batch limits**: commit up to 500 operations and about 1 MB of JSON per call (JMAP's minimum is 500 objects, 10 MB); get up to 100 ids subject to the token budget; dry-run plan preview capped near 2,000 tokens with counts when larger.
- **SQL exposure rules**: read-only connection, `SELECT`/`WITH` only via authorizer, projections only (no raw chain tables), time limit about 2 s, 200 rows / 50,000 characters hard cap, Markdown output with header and row count, schema described in under 4,096 characters.
- **Descriptions**: each tool description and the server instructions under 4,096 characters, key search words first, 1-3 input examples for `commit` and `render`.

#### (4) Recommended engine/storage combination
- System of record: SQLite via `rusqlite` 0.43 (bundled SQLite, WAL, full sync for chain appends, 8 or 16 KiB pages), holding the hash chain, typed facts, projections, FTS5 text index, R*Tree spatial index, idempotency keys, case files and blob metadata.
- Blobs: immutable files named by `blake3` hash, two-level fan-out, temp + fsync + rename; optional `zstd`/`zeekstd` for compressible large files; periodic re-hash scrub logged to the chain; small thumbnails (under about 100 KB) may sit in SQLite.
- Rendering: `tiny-skia` + `resvg` + `cosmic-text` or `fontdue`, `image` + `fast_image_resize` + `imageproc`; PDF pages through `pdfium-render` (dynamic Pdfium, calls serialized); tiles through `pmtiles` (pyramid built once off-box); geometry through `geo` + `rstar`; EXIF through `kamadak-exif` or `nom-exif`; duplicates through `image_hasher`; recordings through `mcap`.
- Vectors: start with brute-force scan (`sqlite-vec` 0.1.9 or a BLOB column), move to a derived `usearch` index before about 10^6 vectors; embeddings and captions produced on the Mac at ingest.
- MCP server: `rmcp` 3.x (2026-07-28 feature coverage to be verified).
- Optional, off the write path: `tantivy` if FTS5 ranking proves inadequate; DuckDB/LanceDB on the Mac for analytics over exported Parquet.
- Avoid: sled; MuPDF unless AGPL is acceptable; GDAL/PROJ at runtime on the Pi unless truly needed.
- Cliffs: brute-force vectors past about 10^6; unpooled late-interaction vectors (about 256 KiB per page); single-writer contention if several agents write at once; Pdfium binary supply for aarch64; undocumented image-token accounting in clients; client bugs around structuredContent and resource links.

#### (5) Outside-the-box ideas
1. **Render handles with zoom**: every image has a `render_id`; zoom requests crop the original, so the model can dive from site map to array to a single junction box in three cheap calls.
2. **Answer with pins**: every marker on any image is a store id; tools accept pin labels from a given render as arguments.
3. **Evidence view**: `render(evidence_of=fact)` shows the manual crop with the exact cell highlighted next to the stored value, so a value can be checked against pixels before a risky action.
4. **Two-channel ingest**: server-extracted text-layer cells and the agent's visual read are compared cell by cell; only agreements auto-promote, disagreements become review items with both crops.
5. **Plan tokens**: dry-run returns a hash; apply-by-hash guarantees the reviewed batch is the written batch.
6. **Contact sheets**: one labelled grid image of many photos or pages for triage, at about 1,500 tokens instead of one image each.
7. **Diff renders**: "what changed since my cursor" as a before/after pair or a change-highlighted map.
8. **Numbers beside every picture**: plots come with server-computed min/max/mean/deviation-from-baseline; maps with counts and distances; the model never reads values off axes.
9. **Georeferenced answers**: each map result carries the pixel-to-metres/lat-lon transform and a `measure` option so spatial questions are computed, not estimated.
10. **Self-auditing footers**: each response ends with approximate tokens used, what was left out, and the exact next call.
11. **Prediction ledger**: hypotheses stored as checkable facts; the server scores them when results land and reports calibration per case file.
12. **Snapshot for code**: a read-only SQLite/Parquet snapshot URL so Claude Code analyses data in its own shell, keeping rows out of context.
13. **Built-in eval set**: ship realistic multi-call tasks with verifiers in the repo and tune tool descriptions against them, as Anthropic recommends.
14. **Look-alike search**: perceptual hash plus embedding answers "have I photographed this connector before" and flags duplicate uploads.
15. **Staleness as a visual**: facts past their verification age are drawn greyed or hatched on maps and flagged by preflight.
16. **Human view through MCP Apps**: an interactive map for the owner in hosts that render it, while the model receives the static image.
17. **Ingest-time captions**: one frontier-model caption per photo/page stored as searchable low-trust text, so FTS finds images by the owner's own words.

### Gaps
- The defaults in (3) are engineering recommendations derived from documented limits and published results; none has been validated against the owner's clients. The first test to run is whether a 4,784-token image and a three-image result pass Claude Code's cap on the installed version, and whether text + image + `structuredContent` all reach the model.
- Behaviour of Claude Desktop, web and mobile with 2026-07-28 servers (MRTR, tasks, image size limits) remains undocumented in the sources read.
