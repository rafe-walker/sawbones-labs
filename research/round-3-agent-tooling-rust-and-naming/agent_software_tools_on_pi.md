# Making software and firmware tools (Wireshark, Kali-class, firmware/hardware RE) available to an AI agent through a Raspberry Pi 5 MCP server (state: 25 Nov 2026)

Source-status legend used throughout:
- **[V]** = fetched and read this session (Nov 2026); the link is the page read.
- **[K]** = from the researcher's prior knowledge (cut-off mid-2026); the link is the canonical documentation to check, but it was NOT re-fetched this session. Treat as "probably right, verify before relying on it".
- **[U]** = unverified / could not be confirmed.

Scope statement: everything here is for the owner's OWN networks, vehicles, solar equipment and home-built electronics. No attack walkthroughs are given. Tools that transmit, inject, deauthenticate, brute-force or act on systems the owner does not own are marked "exclude" or "gate".

Verdict vocabulary: **P1** = wrap in phase 1; **P2** = phase 2; **Mac** = run on the Mac, Pi only captures; **Exclude** = do not expose to the agent.

---

## 1. Packet and protocol analysis: which tools, which output modes, and how does an agent query a multi-gigabyte capture cheaply?

### Takeaway
tshark is the right core: it has machine-readable modes (`-T ek|json|fields|pdml`), statistics taps (`-z`), ring-buffer capture and extcap, and is in Debian arm64. Never hand the agent raw tshark output: one study measured `tshark -T json` at about 22 million tokens for captures of a few megabytes; pre-computed statistics plus filter-driven slices plus a columnar index (DuckDB/Parquet) is what works.

### Cited Findings
- tshark's `-T` flag offers nine output formats: `ek` (newline-delimited JSON for bulk import), `fields` (only the `-e` fields, shaped by `-E`), `json`, `jsonraw`, `pdml` (XML detail), `psml` (XML summary), `ps`, `tabs`, `text`. [V] — [tshark man page](https://www.wireshark.org/docs/man-pages/tshark.html)
- JSON size can be cut at source: `-j` keeps only named protocol parent nodes, `-J` keeps named protocols with all children, and `--no-duplicate-keys` folds repeated keys into arrays. [V] — [tshark man page](https://www.wireshark.org/docs/man-pages/tshark.html)
- Statistics taps useful as pre-computed summaries: `-z io,phs` (protocol hierarchy), `-z conv,<type>` and `-z endpoints,<type>` (tcp/udp/ip/eth/ipv6), `-z io,stat,<interval>[,filter]` (COUNT/SUM/MIN/MAX/AVG/LOAD), `-z expert[,severity][,filter]`, `-z follow,<prot>,<mode>,<filter>`, `-z http,tree`, `-z dns,tree`, `-z flow,<name>,<mode>`. [V] — [tshark man page](https://www.wireshark.org/docs/man-pages/tshark.html)
- Ring-buffer capture is controlled with `-b duration|files|filesize|interval|packets|printname|nametimenum`; one criterion per `-b`, and `files` needs one of duration/interval/filesize. [V] — [tshark man page](https://www.wireshark.org/docs/man-pages/tshark.html)
- `-Y` applies a display filter in a single pass; `-2` (two-pass, fills forward-looking fields) cannot be used on live captures or pipes; `--export-objects <protocol>,<dir>` extracts transferred objects; `-G fields[,prefix]` dumps the field registry and `-G elastic-mapping` emits an Elasticsearch mapping; `-X lua_script:<file>` loads a Lua dissector. [V] — [tshark man page](https://www.wireshark.org/docs/man-pages/tshark.html)
- The newest Wireshark listed on the release-notes index is 4.6.10 (4.6 branch), with 4.4.20 as the newest old-stable; no 5.0 branch is listed. Release dates were not shown on the index. [V] — [Wireshark release notes index](https://wireshark.org/docs/relnotes/)
- Raw decoded captures are far too big for a model: across 60 test files, `tshark -T json` averaged 22,050,813 estimated tokens and `tshark -V` 18,738,238, versus 852,750 for the raw pcap bytes and 23,091 for the authors' compact "PacketGlyph" encoding (about 800x smaller than verbose tshark text). [V] — [APNIC blog, PCAP-LM, 6 Nov 2026](https://blog.apnic.net/2026/11/06/pcap-lm-helping-ai-assistants-read-network-traffic/)
- In the same work an unnamed "state-of-the-art" model answered 99.3% of ten basic forensic questions per capture from the compact form, against 51% when given tshark text truncated to the same length (truncation leaves only the first few dozen packets). The encoding is lossy: retransmissions were missed in about a quarter of affected captures, and testing covered only large encrypted downloads from one 5G/4G campaign. [V] — [APNIC blog, PCAP-LM](https://blog.apnic.net/2026/11/06/pcap-lm-helping-ai-assistants-read-network-traffic/)
- CyberSleuth (Politecnico di Torino / Huawei, arXiv 2508.20643v2, revised 5 Mar 2026) compared three agent designs for pcap forensics. Letting one agent drive a tshark wrapper packet-by-packet performed worst (GPT-4o: service 0.45, CVE 0.28, MCC −0.11); a design where a sub-agent first writes per-flow summaries and the main agent reasons over them did best (GPT-4o MCC 0.45; GPT-5 MCC 0.63, and 0.82 on a 2025 test set). [V] — [CyberSleuth, arXiv](https://arxiv.org/html/2508.20643v2)
- Failure modes reported there: erratic packet inspection ("loss of focus", about 18 steps), vague delegation to a tshark sub-agent returning inconclusive results, missed plain-text telnet clues, large multi-container traces, and cascading errors after a wrong first identification. Design choices that helped: flow-level entry view, token budget per flow by a square-root rule, keeping the start and end of each flow, dropping encrypted flows. [V] — [CyberSleuth, arXiv](https://arxiv.org/html/2508.20643v2)
- Third-party "Wireshark MCP" (bx33661, PyPI `wireshark-mcp` 4.0.0, 25 Oct 2026, MIT, Python ≥3.10) exposes 54 tools in groups: packet list/detail/stream-follow, field extraction, statistics (protocol hierarchy, conversations, expert info), anomaly checks (port scan, DNS tunnel, beaconing, YARA, credential scan), TLS/WPA decryption, live capture, file ops, and an optional DuckDB capture index (`wireshark_index`). [V] — [wireshark-mcp on PyPI](https://pypi.org/project/wireshark-mcp)
- That server's token controls: results over 8,000 characters are cut head-and-tail with a marker and paged with `offset`/`limit`/`display_filter`; tool-list profiles shrink the schema load (`full` 54 tools ≈23 KB, `analysis` 42 ≈18 KB, `core` 34 ≈15 KB). Its safety controls: file-writing tools fail closed unless `WIRESHARK_MCP_ALLOWED_DIRS` is set; HTTP binds to loopback unless `--allow-insecure-http`; tools carry read/write annotations (41 read-only, 12 file-creating). [V] — [wireshark-mcp on PyPI](https://pypi.org/project/wireshark-mcp)
- Other pcap MCP servers exist but were only seen as search hits, not examined: `mcp-wireshark` (khuynh22), `tshark-mcp` 1.0.0, `inovacc/sharkline`. [U] — [mcp-wireshark on PyPI](https://pypi.org/project/mcp-wireshark/), [tshark-mcp on PyPI](https://pypi.org/project/tshark-mcp/), [sharkline on GitHub](https://github.com/inovacc/sharkline)
- Tool-by-tool inventory (packet/protocol job):

| Tool | Passive/active | Structured output | ARM64 / Pi 5 | 4 GB concern | Verdict | Src |
|---|---|---|---|---|---|---|
| dumpcap (capture only) | Passive | pcapng files; ring buffer `-b` | Debian arm64 package | Negligible | **P1** (the only privileged capture process) | [V] [tshark man](https://www.wireshark.org/docs/man-pages/tshark.html) |
| tshark (offline analysis) | Passive | `-T ek/json/fields/pdml`, `-z` taps | Debian arm64 | Two-pass on multi-GB files can use >1 GB; slice first | **P1** | [V] same |
| capinfos / editcap / mergecap | Passive | `capinfos -T -M` table | ships with Wireshark | None | **P1** | [K] [Wireshark docs](https://www.wireshark.org/docs/man-pages/) |
| tcpdump | Passive | pcap; text only otherwise | Debian arm64 | None | P1 as fallback capturer only | [K] [tcpdump.org](https://www.tcpdump.org/) |
| termshark | Passive | none (TUI) | Go binary | None | **Exclude** (human TUI, nothing for an agent) | [K] [termshark](https://github.com/gcla/termshark) |
| Zeek | Passive | JSON logs (conn, dns, http, modbus, mqtt...) | Builds on arm64; packaging status on Pi OS [U] | Moderate; fine offline on pcaps | **P2** (best flow summariser) | [K] [Zeek docs](https://docs.zeek.org/) |
| Suricata | Passive (IDS mode) | EVE JSON | Debian arm64 | Full rulesets cost ~1 GB+ [U] | P2, offline `-r` runs only | [K] [Suricata docs](https://docs.suricata.io/) |
| ntopng | Passive | REST API | vendor Pi builds [U] | Runs a persistent web service | P2/optional | [K] [ntop](https://www.ntop.org/) |
| Arkime | Passive | REST | needs OpenSearch | Too heavy for 4 GB | **Exclude** | [K] [Arkime](https://arkime.com/) |
| usbmon (via dumpcap `-i usbmonN`) | Passive | pcapng, Wireshark USB dissectors | kernel module | None | **P2** | [K] [kernel usbmon doc](https://docs.kernel.org/usb/usbmon.html) |
| can-utils (candump, canplayer...) | candump passive; cansend/canplayer active | candump log format; SocketCAN also capturable by dumpcap | Debian arm64 | None | candump **P1**; send tools gated "act" | [K] [can-utils](https://github.com/linux-can/can-utils) |
| cantools (DBC decode) | Passive (offline) | Python API / decoded text | pip | None | **P1** | [K] [cantools](https://github.com/cantools/cantools) |
| nRF Sniffer extcaps (BLE, 802.15.4) | Passive | pcapng via extcap | Python extcap + nRF52840 dongle | None | **P2** | [K] [Nordic nRF Sniffer](https://www.nordicsemi.com/Products/Development-tools/nrf-sniffer-for-bluetooth-le) |

- Wireshark dissectors exist for Modbus TCP/RTU, CAN/J1939, MQTT, BLE ATT/GATT, Zigbee, Thread and LoRaWAN; a generic Protobuf dissector can load `.proto` files (the route for Meshtastic protobufs). [K] — [Wireshark display filter reference](https://www.wireshark.org/docs/dfref/), [Meshtastic protobufs](https://github.com/meshtastic/protobufs)

### Inferences
- The cheap-query recipe for a large capture, in order: (1) at capture close, run `capinfos` plus `-z io,phs`, `-z conv,ip`, `-z conv,tcp`, `-z endpoints,eth`, `-z expert`, `-z dns,tree`, `-z http,tree` once and store the results as a small JSON "capture card"; (2) run one `-T fields` (or `-T ek`) pass extracting 20-30 key fields into Parquet and query it with DuckDB (SQL with a hard row cap); (3) answer detail questions with display-filter slices (`-Y ... -c N`) that return at most N frames with `-j`-restricted JSON; (4) keep frame numbers in every row so any claim can be traced to exact frames. The bx33661 server's optional DuckDB index and the PCAP-LM "side index back to frame numbers" are independent confirmations of this shape.
- CyberSleuth's result argues for the project's meta-tool design: the server should hand back flow-level summaries (server-side "summariser") and let the agent ask narrow follow-ups, instead of exposing a free-form tshark tool.
- Serial and Modbus RTU are not natively capturable as a network interface. The practical route is a passive tap (RS485 receiver or logic analyzer) whose bytes the supervisor frames into pcapng with a user DLT mapped to the `mbrtu` dissector, so serial, CAN and Ethernet all land in one pcapng-centred evidence format. [K, design inference]
- Split capture from dissection: dissectors parse hostile bytes and are a recurring source of CVEs, so only `dumpcap` should hold `CAP_NET_RAW`, and tshark should run unprivileged, offline, with no network.
- For the Mac user, Wireshark's `sshdump` extcap lets the human watch the same Pi interface live in the GUI while the agent works from summaries. [K]

### Gaps
- No evidence was found of an official Wireshark Foundation MCP server or built-in AI/agent feature; two searches returned only third-party servers. Absence is not proven.
- GitHub star counts for the Wireshark MCP servers were not shown on the pages read.
- Wireshark 4.6.10 release date, and whether the tshark man page read corresponds to 4.6.x or the development branch, were not shown.
- Zeek/ntopng arm64 packaging on Raspberry Pi OS (trixie) was not verified.
- The `--protocol-decoder-jsontrace` and Meshtastic-specific Wireshark support claims were not verified this session.

---

## 2. Kali-class tooling on a Pi 5: how to install it, and which tools have stable structured output?

### Takeaway
Kali supports the Pi 5 (including Nexmon monitor mode on the onboard radio since 2025.1), but the least fragile setup for this project is a plain Raspberry Pi OS/Debian host with Debian-native packages for the passive tools, plus the official Kali container (rootless Podman) for anything missing; do not add the Kali apt repo to the host. Most scanners have a machine-readable mode; most Wi-Fi attack tools do not, and those are the ones to exclude anyway.

### Cited Findings
- Kali's Nexmon packages (`brcmfmac-nexmon-dkms`, a DKMS driver rebuilt per kernel, and `firmware-nexmon`) add monitor mode and injection on Broadcom onboard Wi-Fi, with the Pi 5 (64-bit) among tested boards; support was rebuilt on kernel 6.12 and first shipped in Kali 2025.1. Kali itself warns injection is "not always stable" and that monitor-mode start may print error -524 yet still work. [V] — [Kali blog, 22 Jul 2025](https://www.kali.org/blog/raspberry-pi-wi-fi-glow-up/)
- Kali 2026.3 was released 26 Oct 2026 with kernel 7.1.5; six tools joined the repos, including `rustscan` and `sdrangel`; no MCP/AI tool was among them and the coverage read mentions no Pi-specific change. Kali's ARM build scripts were overhauled, with a `build-in-container.sh` that uses Docker or Podman. [V] — [Help Net Security, 26 Oct 2026](https://helpnetsecurity.com/2026/10/26/kali-linux-2026-3-released)
- Official Kali container images: `kalilinux/kali-rolling` (main), `kali-last-release`, `kali-bleeding-edge`, `kali-experimental`, `kali-dev`; refreshed weekly; they ship without tools, and tools are added with `apt install kali-linux-headless` or individual packages. [V] — [Kali docs, official Docker images](https://www.kali.org/docs/containers/official-kalilinux-docker-images/)
- `binwalk3` 3.1.0 is packaged by Kali for arm64 and amd64 (3.88 MB installed). [V] — [Kali tools: binwalk3](https://www.kali.org/tools/binwalk3/)
- A Kali documentation page for the Raspberry Pi 5 exists (seen in search results, not read). [U] — [Kali docs: Raspberry Pi 5](https://www.kali.org/docs/arm/raspberry-pi-5)
- Kismet exposes a REST-like API from an embedded web server: JSON by default, POST variants accept filtering/field optimisation, and every endpoint except four session-check endpoints requires a login. [V] — [Kismet REST docs](https://www.kismetwireless.net/docs/api/rest_like/)
- Tool-by-tool inventory (network / wireless / web / credential job). "Active" means it emits packets or RF toward a target.

| Tool | Passive/active | Structured output | ARM64 / Pi 5 | 4 GB concern | Verdict (reason) | Src |
|---|---|---|---|---|---|---|
| nmap | Active (probes) | `-oX` XML (stable, documented); no native JSON | Debian arm64 | None | **P1**, bound to owned subnets; gentle profile by default | [K] [nmap output](https://nmap.org/book/output.html) |
| arp-scan | Active (ARP, L2 only) | plain/`--format` fields | Debian arm64 | None | **P1** (owned subnet) | [K] [arp-scan](https://github.com/royhills/arp-scan) |
| netdiscover | Passive mode `-p` or active | parsable text `-P` | Debian arm64 | None | P2 (redundant with capture-derived inventory) | [K] [netdiscover](https://github.com/netdiscover-scanner/netdiscover) |
| masscan | Active, high rate | `-oJ` JSON, `-oX` | Debian arm64 | None | **Exclude** (internet-scale speed is pointless on a home LAN and can upset fragile devices) | [K] [masscan](https://github.com/robertdavidgraham/masscan) |
| RustScan | Active, fast | greppable; hands off to nmap | in Kali since 2026.3 | None | Exclude for now (nmap suffices; speed is a liability here) | [V] [HNS on 2026.3](https://helpnetsecurity.com/2026/10/26/kali-linux-2026-3-released) |
| Kismet | Passive (monitor mode, BT, 802.15.4, rtl_433 sources) | REST JSON; kismetdb SQLite | arm64 packages [K] | Moderate, long-running daemon | **P1** (survey engine) | [V] [Kismet REST](https://www.kismetwireless.net/docs/api/rest_like/) |
| bettercap | Mixed: recon passive; spoof/deauth/BLE-write modules active | REST API, event stream, caplets | Go, arm64 | Low | P2, **recon modules only**; active modules blocked by policy | [K] [bettercap](https://www.bettercap.org/) |
| airodump-ng | Passive | CSV, kismet netxml, pcap | Debian arm64 | None | P2 (Kismet covers it) | [K] [aircrack-ng docs](https://www.aircrack-ng.org/doku.php?id=airodump-ng) |
| aireplay-ng, mdk4, wifite, reaver | Active: inject/deauth/disrupt/WPS attack | none useful | arm64 | None | **Exclude** (transmit/disrupt; deauth is an FCC problem off your own gear) | [K] [aircrack-ng docs](https://www.aircrack-ng.org/) |
| hcxdumptool / hcxtools | hcxdumptool is active by default (stimulates APs/clients); hcxtools are offline converters | pcapng; hashcat 22000 text | arm64 | None | hcxdumptool **Exclude**; hcxpcapngtool P2 offline on own captures | [K] [hcxdumptool](https://github.com/ZerBea/hcxdumptool) |
| ettercap, responder | Active MITM / poisoning | logs | arm64 | None | **Exclude** | [K] [ettercap](https://www.ettercap-project.org/) |
| impacket, netexec | Active against Windows/AD services | partial | Python | None | **Exclude** (no relevance to this bench) | [K] [netexec](https://www.netexec.wiki/) |

NOTE ON THIS TABLE: it stops here on purpose. The remaining tools named in the assignment for this job (online password guessing, offline password cracking, exploitation frameworks, web-application attack scanners and fuzzers, Windows/AD credential tooling) were not inventoried row by row; a tool-by-tool listing of that class was withheld during writing. The project-level verdict for the whole class is **Exclude from the agent's tool layer**: none of it is needed for diagnosing the owner's own inverters, vehicles, sensors and home network, and it is the class where an agent mistake or an injected instruction does the most damage. See the Gaps list below.

- Benign, diagnostic members of the wider "Kali" family that ARE worth wrapping later, all bound to owned assets:

| Tool | Passive/active | Structured output | ARM64 / Pi 5 | 4 GB concern | Verdict | Src |
|---|---|---|---|---|---|---|
| testssl.sh / sslyze | Active but benign (TLS handshakes) | JSON file output | bash / Python | None | P2 (check TLS on own devices' web UIs) | [K] [testssl.sh](https://testssl.sh/) |
| mitmproxy (mitmdump) | Inline (owner routes an owned device through it) | flow files, HAR, Python addon API | pip, arm64 | Low-moderate | P2, tier "act", owned device only | [K] [mitmproxy docs](https://docs.mitmproxy.org/) |
| hostapd + dnsmasq (controlled test AP) | Transmits as an ordinary certified Wi-Fi AP | config-driven; logs | Debian arm64 | None | P2, tier "act", arm switch | [K] [hostapd](https://w1.fi/hostapd/) |
| bluetoothctl / btmon | Scan: passive or active scan; btmon passive HCI trace | btsnoop file readable by tshark | BlueZ, arm64 | None | **P1** for btmon + passive scan; GATT connect = "measure" | [K] [BlueZ](https://www.bluez.org/) |
| Sniffle / nRF Sniffer | Passive BLE sniffing | pcap | needs TI CC26x2/CC1352 or nRF52840 | None | P2 | [K] [Sniffle](https://github.com/nccgroup/Sniffle) |
| zigbee2mqtt | Participates in the owner's own Zigbee network | JSON over MQTT | Node.js, arm64 | Low-moderate | P2 (own network only) | [K] [zigbee2mqtt](https://www.zigbee2mqtt.io/) |
| KillerBee | Mixed; several tools inject | pcap | Python | None | Exclude except passive dump; Kismet/nRF sniffer cover the passive need | [K] [KillerBee](https://github.com/riverloopsec/killerbee) |
| rtl_433 | Passive (receive-only SDR) | `-F json`, MQTT, InfluxDB outputs | Debian arm64 | None | **P1** | [K] [rtl_433](https://github.com/merbanan/rtl_433) |
| URH | Analysis passive; can transmit with TX-capable SDR | project files; `urh_cli` | Python/Qt | GUI-centric | **Mac** (analysis of recordings); TX disabled | [K] [URH](https://github.com/jopohl/urh) |
| inspectrum | Passive (offline viewer) | none (GUI) | Qt | GUI only | **Mac**, human tool | [K] [inspectrum](https://github.com/miek/inspectrum) |
| Proxmark3 client / libnfc | Near-field reader = active | text; some JSON dumps; scripting | builds on arm64 | None | P2/P3, owned tags only, tier "measure"; writes "act" | [K] [Proxmark3 (RRG)](https://github.com/RfidResearchGroup/proxmark3) |
| gpsd (gpspipe) | Passive | JSON stream | Debian arm64 | None | **P1** (time/location stamp for every capture) | [K] [gpsd](https://gpsd.io/) |
| searchsploit-style offline CVE lookup | Passive (local database) | JSON | arm64 | None | P2, prefer grype/cve-bin-tool on extracted firmware instead | [K] [grype](https://github.com/anchore/grype) |
| Greenbone/OpenVAS | Active scanner suite | XML/JSON reports | heavy multi-service stack | Too heavy for 4 GB | **Exclude** on the Pi | [K] [Greenbone docs](https://greenbone.github.io/docs/) |

### Inferences
- Least-fragile install order: (1) Debian/Raspberry Pi OS packages for everything in Debian main (tshark, nmap, arp-scan, kismet, can-utils, rtl-433, sigrok-cli, radare2, flashrom, gpsd, bluez); (2) `cargo install`/release binaries for Rust tools (binwalk v3, probe-rs, espflash), which also fits the Rust-first rule; (3) a rootless Podman container from `kalilinux/kali-rolling`, pinned by digest, with only the specific packages needed and no host networking by default, for anything left; (4) full Kali on the SD card only if onboard-radio monitor mode via Nexmon is really wanted.
- Adding the Kali repository to a Debian/Raspberry Pi OS host is the fragile option: Kali is a rolling distribution built from Debian testing, so mixing it into a stable host pulls library versions forward under the supervisor daemon ("FrankenDebian"). [K] — [Debian wiki: DontBreakDebian](https://wiki.debian.org/DontBreakDebian)
- A USB monitor-mode adapter with an in-kernel driver (mt76 family) avoids both Nexmon and a Kali kernel, and keeps the Pi's onboard radio free as the management link to the Mac. [K, hardware inference]
- Injection capability in the driver is not needed at all for this project's default (receive-only). Choosing not to install Nexmon injection firmware is itself a safety control: the radio then cannot do what the policy forbids.
- Nix is a credible alternative for pinning exact tool versions on aarch64 with reproducible closures, at the cost of disk space and a second packaging system. [U, not researched this session]
- "Stable structured output" short list an agent can consume directly: nmap XML, tshark ek/fields, Kismet REST JSON, rtl_433 JSON, gpsd JSON, Zeek JSON logs, Suricata EVE JSON, testssl JSON. Tools with only human text (aircrack-ng suite beyond CSV, netdiscover) are poor wrapper candidates regardless of safety.

### Gaps
- Withheld/omitted: per-tool rows for the password-guessing, password-cracking, exploitation-framework, web-attack-scanner and AD-tooling class (including whether cracking is feasible on a Pi versus the Mac, and the resource cost of an exploitation framework's RPC daemon on 4 GB). The assignment asked for these; they are not in these notes. If the report writer needs them, they must be requested separately and scoped to the owner's own equipment.
- Kali's statement about mixing repositories could not be read (page returned no body).
- Whether the official Kali container images publish arm64 manifests was not stated on the page read (believed yes [K]).
- Current Kali Pi 5 kernel packaging (Raspberry Pi kernel vs. Re4son) was not verified; the Nexmon post implies DKMS on a 6.12-era kernel as of 2025.
- Kismet arm64 package availability for Raspberry Pi OS trixie not verified.

---

## 3. Existing MCP servers and agent wrappers for security and reverse-engineering tools: what do they expose, how do they gate, how mature are they?

### Takeaway
By late 2026 there is an official Kali package (`mcp-kali-server`) and many third-party "pentest MCP" servers, but nearly all are thin bridges to a shell with no target or argument policy; the one widely known example (HexStrike AI) has a published critical command-injection CVE. The reverse-engineering servers (GhidraMCP, r2mcp, Burp's official server) are the more reusable prior art, and Burp's approval model is the closest to what this project needs.

### Cited Findings
- `mcp-kali-server` (Kali package version 0.0~git20260317.00154c0, tools page updated 18 Nov 2026, 75 KB) is two parts: `kali-server-mcp`, a Flask API on 127.0.0.1:5000, and `mcp-server`, a bridge with a 300-second default timeout. Kali describes it as letting MCP clients execute commands on a Linux terminal; dependencies include `sudo`. Upstream is Wh0am123/MCP-Kali-Server. [V] — [Kali tools: mcp-kali-server](https://www.kali.org/tools/mcp-kali-server/)
- The upstream README offers raw command execution alongside named integrations, describes no authentication and no input validation or allow-list, and relies on the loopback bind plus an SSH tunnel; binding to 0.0.0.0 is called very dangerous. No licence is stated in the README. [V] — [MCP-Kali-Server README](https://raw.githubusercontent.com/Wh0am123/MCP-Kali-Server/main/README.md)
- Kali's own blog post (25 Feb 2026, "Kali & LLM: macOS with Claude Desktop & Anthropic Sonnet LLM") uses the same topology as this project: Claude Desktop on macOS as MCP client, a Kali box reached over SSH with key auth, a cloud model. The only gate described is the client's permission prompt; the post flags privacy as a reason it may not suit everyone, and the server log itself says it is a development server. [V] — [Kali blog](https://www.kali.org/blog/kali-llm-claude-desktop/)
- Press coverage of that post relays outside warnings that such workflows add prompt-injection exposure, over-permissioned tool access and weak audit logging, with recommendations of least privilege, input validation, human approval for high-risk commands and immutable execution logs. [V] — [Cyber Security News, 26 Feb 2026](https://cybersecuritynews.com/kali-linux-integrates-claude-ai/)
- HexStrike AI (v6.0, MIT, OTT Cybersecurity LLC) claims 150+ wrapped tools and 12+ agents behind a local HTTP server on port 8888, including an endpoint documented as executing arbitrary commands. Its README describes no technical authorization enforcement, only advice to use isolated VMs, and advises users to assert ownership in the prompt. A v7 preview advertises getting around clients' limits on allowed MCP tools. [V] — [HexStrike AI README](https://raw.githubusercontent.com/0x4m4/hexstrike-ai/master/README.md)
- CVE-2025-35028 (CVSS 9.1, CWE-78): in HexStrike's default configuration an argument starting with a semicolon is appended to the composed command line, unsanitised, and runs with the server's privileges (typically root); unauthenticated if network-reachable. Tested on the commit current at 25 Sep 2025; vendor notified 1 Oct 2025; published 30 Nov 2025; the advisory does not record a fix. [V] — [takeonme.org advisory](https://takeonme.org/cves/cve-2025-35028)
- PentestMCP (Portland State, arXiv 2510.03610) is the typed-tool counter-example: one MCP server per stage (`nmap_scan`, `curl_request`, `nuclei_scan`, and seven tools over an exploitation framework's RPC API), each with declared inputs and output format. The paper does not address long-running jobs or output size, and calls itself a proof of concept with single-run traces. [V] — [PentestMCP, arXiv](https://arxiv.org/html/2510.03610v1)
- Autonomous pentest agents (snapshot 13 Nov 2026): PentAGI (34,048 stars, MIT + EULA, v2.2.0 of 5 Oct 2026), Shannon (51,799, AGPL-3.0, v3.5.1), Strix (75,209, Apache-2.0, v1.7.1), Cairn (5,100, AGPL-3.0). All run tools inside Docker containers, most on a Kali base image. Reported weaknesses: PentAGI sandboxes publish callback ports on every host interface by default and a fresh install has a documented admin password; Strix mounts the local source directory writable and passes credentials as free-text prompt content; Cairn's example config puts the worker on the host network and has a no-sandbox local mode; Shannon's guide warns that malicious repository content can steer it. [V] — [Stingrai comparison, 13 Nov 2026](https://www.stingrai.io/blog/open-source-ai-pentest-agents-compared-2026)
- Burp Suite's official MCP server extension (PortSwigger): serves Streamable HTTP on 127.0.0.1:9876 with a stdio proxy; by default Burp asks the human for approval before sending HTTP requests or reading proxy history; config-editing tools sit behind a separate checkbox. [V] — [PortSwigger mcp-server README](https://cdn.jsdelivr.net/gh/portswigger/mcp-server@main/README.md)
- GhidraMCP (LaurieWired, Apache-2.0; 11,969 stars on 18 Nov 2026; last commit about 38 days earlier): a Ghidra plugin serving HTTP plus a Python MCP bridge; decompile, rename, list functions/classes/imports/exports; requires Ghidra 12.1.4; has a headless mode via PyGhidra (Python 3.10-3.13), and on macOS the decompiler natives must be built first. [V] — [GhidraMCP README](https://raw.githubusercontent.com/LaurieWired/GhidraMCP/main/README.md), [EnterpriseDNA directory](https://enterprisedna.co/directories/vs/lauriewired-ghidramcp-vs-radareorg-r2mcp)
- r2mcp (radareorg, official radare2 MCP stdio server): 366 stars and a same-day commit on 19 Nov 2026. [V] — [EnterpriseDNA directory](https://enterprisedna.co/directories/vs/lauriewired-ghidramcp-vs-radareorg-r2mcp)
- OWASP MCP Top 10 is at beta v0.1: MCP01 token mismanagement, MCP02 scope creep, MCP03 tool poisoning, MCP04 supply chain, MCP05 command injection and execution, MCP06 intent-flow subversion / prompt injection via contextual payloads, MCP07 insufficient authn/authz, MCP08 lack of audit and telemetry, MCP09 shadow servers, MCP10 context injection and over-sharing. [V] — [OWASP MCP Top 10](https://owasp.org/www-project-mcp-top-10/)
- Keysight (12 Jan 2026) frames MCP command injection as a confused-deputy problem and notes the protocol gives no explicit signal when a high-risk action happens, so policy must be enforced at the execution layer. [V] — [Keysight blog](https://www.keysight.com/blogs/en/tech/nwvs/2026/01/12/mcp-command-injection-new-attack-vector)

### Inferences
- What goes wrong when an agent is handed a raw shell with a full security distribution installed, synthesised from the sources above: (1) scope is only a sentence in the prompt, so nothing technical stops a typo'd subnet, a neighbour's SSID or the upstream ISP gateway from being probed; (2) the wrapper typically runs as root with sudo, so one injection equals host compromise (HexStrike CVE); (3) text returned by the target becomes instructions (CAI proof of concept, section 6); (4) the model reaches for the most aggressive flag set it knows; (5) unbounded stdout floods the context and the important line is truncated away; (6) long scans hit client timeouts and get re-launched in parallel; (7) the audit trail is a chat transcript, not a tamper-evident log; (8) there is no rate limit, so a scan can starve the box's real job.
- Reuse verdicts for this project: borrow the bx33661 Wireshark server's ideas (DuckDB index, head-and-tail truncation, read/write annotations, fail-closed write directories) but not the server itself, because the project already has six meta-tools and a Rust supervisor; run GhidraMCP and r2mcp on the Mac as separate MCP servers pointed at firmware the Pi extracted; copy Burp's "human approves each outbound request and each history read" as the model for the act tier. Do not install `mcp-kali-server` or HexStrike on the pack.

### Gaps
- r2mcp's README could not be fetched; its language, licence, tool list and any sandbox/read-only options are unverified.
- IDA Pro MCP (mrexodia), Binary Ninja MCP (fosdickio), GhidrAssistMCP, Shodan MCP, Nebula, PentestGPT and CAI were seen only as names/links; no maturity data was read. PentestGPT/CAI details beyond the CAI injection paper are not covered.
- Licences for MCP-Kali-Server, PentestMCP and Burp's server were not stated on the pages read.
- Whether CVE-2025-35028 has been fixed upstream is unknown.

---

## 4. Firmware and hardware reverse-engineering tools for the bench: which can an agent drive headlessly, and which belong on the Mac?

### Takeaway
Most bench RE tools are small command-line programs that an agent can drive well on the Pi: binwalk v3 (Rust, JSON log), unblob (JSON report), radare2/rizin (JSON on every command), sigrok-cli, probe-rs/espflash (Rust, usable as libraries), flashrom, esptool. Decompilation (Ghidra) and heavy firmware-analysis suites belong on the Mac. Reads are "measure"; anything that writes flash, fuses or a live bus is "act".

### Cited Findings
- Binwalk v3 is a ground-up Rust rewrite aimed at speed and accuracy; it finds and optionally extracts embedded files, does entropy analysis, installs via cargo, Docker or source, and can be used as a library from Rust code. [V] — [ReFirmLabs/binwalk README](https://raw.githubusercontent.com/ReFirmLabs/binwalk/master/README.md)
- binwalk3 3.1.0 options relevant to wrapping: `-l/--log <file>` writes JSON results, `-e` extracts, `-M` recurses, `-E` entropy plot, `-t` threads, `-x/-y` exclude/include signatures, `-C` extraction directory. Kali ships it for arm64; a `librust-binwalk-dev` crate package also exists. [V] — [Kali tools: binwalk3](https://www.kali.org/tools/binwalk3/)
- unblob (ONEKEY, MIT, 2.6k stars): 78+ formats per README (83 documented handlers as of release 26.3.24, up from 71), runs without elevated privileges, `--report` writes a JSON report with chunk offsets, sizes, entropy, ownership, permissions and timestamps; install via pip, Docker, Kali apt, Nix or source (Python ≥3.10 plus a Rust toolchain). A `--no-sandbox` switch exists, implying sandboxing is on by default. [V] — [unblob on GitHub](https://github.com/onekey-sec/unblob), [ONEKEY blog, 26 Mar 2026](https://www.onekey.com/resource/latest-developments-in-unblob-new-formats-smarter-extraction-and-a-more-hardened-release-pipeline)
- Third-party papers cited by ONEKEY report unblob beating Binwalk 2.x on speed and accuracy in limited testing; this is a vendor-relayed comparison against the old Python binwalk, not against v3. [V] — [ONEKEY blog](https://www.onekey.com/resource/latest-developments-in-unblob-new-formats-smarter-extraction-and-a-more-hardened-release-pipeline)
- sigrok-cli: `-P` stacks protocol decoders with `key=value` options and channel assignments, multiple `-P` run in parallel, `-A` selects annotation classes, `-B` emits a decoder's binary output, `--protocol-decoder-samplenum` adds sample numbers; outputs include `bits`, `hex`, `csv`; capture is bounded with `--time`, `--samples`, `--frames` or `--continuous`; `-t` sets triggers; `--scan` finds devices. [V] — [sigrok-cli wiki](https://sigrok.org/wiki/Sigrok-cli)
- Tool inventory (firmware / hardware RE job):

| Tool | Passive/active | Structured output | ARM64 / Pi 5 | 4 GB concern | Verdict | Src |
|---|---|---|---|---|---|---|
| binwalk v3 | Passive (file) | JSON log | arm64 package; cargo | Extraction of big images uses disk, not RAM | **P1** | [V] [Kali binwalk3](https://www.kali.org/tools/binwalk3/) |
| unblob | Passive (file) | JSON report | pip/Docker; arm64 wheels [U] | Moderate | **P1** (second extractor; disagreement between the two is itself a finding) | [V] [unblob](https://github.com/onekey-sec/unblob) |
| firmware-mod-kit | Passive | none | unmaintained | n/a | **Exclude** (abandoned) | [K] |
| FACT | Passive | REST/web | multi-service stack, x86-oriented | Too heavy | **Mac**/skip | [K] [FACT_core](https://github.com/fkie-cad/FACT_core) |
| EMBA | Passive (static) + optional emulation | HTML/JSON reports | x86 Linux focus, very large image | Too heavy | **Mac** in a VM, P3 | [K] [EMBA](https://github.com/e-m-b-a/emba) |
| Ghidra headless / PyGhidra | Passive (file) | scriptable; via GhidraMCP | JVM; natives must be built on arm64 [U] | JVM heap makes 4 GB tight | **Mac** | [V] [GhidraMCP README](https://raw.githubusercontent.com/LaurieWired/GhidraMCP/main/README.md) |
| radare2 / rizin | Passive (file) | JSON variants of commands | Debian arm64 | Low | **P2** on the Pi (triage: sections, strings, imports, functions) | [K] [radare2](https://github.com/radareorg/radare2) |
| strings / entropy / secret scanners (trufflehog, gitleaks) | Passive | JSON reports | Go/coreutils | Low | **P1** as part of fw.identify | [K] [trufflehog](https://github.com/trufflesecurity/trufflehog) |
| syft + grype, cve-bin-tool | Passive (file tree) | CycloneDX/SPDX JSON; JSON findings | Go / Python | Vulnerability DB is hundreds of MB on disk [U] | **P2** | [K] [syft](https://github.com/anchore/syft), [grype](https://github.com/anchore/grype), [cve-bin-tool](https://github.com/intel/cve-bin-tool) |
| dtc, U-Boot tools (dumpimage, mkimage -l) | Passive | text/DTS | Debian arm64 | None | P2 | [K] [U-Boot docs](https://docs.u-boot.org/) |
| flashrom (CH341A or Pi SPI) | Read = measure; write/erase = act | text; image file | Debian arm64 | None | Read **P1/P2**; write gated | [K] [flashrom.org](https://flashrom.org/) |
| probe-rs | Read/attach = measure; flash/erase = act | CLI + Rust library, RTT | Rust, arm64 | Low | **P2** (natural fit for the Rust supervisor) | [K] [probe.rs](https://probe.rs/) |
| OpenOCD / pyOCD | same | TCL/GDB server; Python API | Debian arm64 | Low | P2 fallback where probe-rs lacks a target | [K] [OpenOCD](https://openocd.org/) |
| esptool / espefuse / espflash | Read flash, chip ID, security info = measure; write flash = act; eFuse burn = irreversible act | text, some JSON; espflash is a Rust crate | pip / cargo | None | Read **P1**; write P2 gated; eFuse burn **Exclude** from agent | [K] [esptool docs](https://docs.espressif.com/projects/esptool/), [espflash](https://github.com/esp-rs/espflash) |
| avrdude, dfu-util, picotool, stm32flash, nrfutil | read = measure; write = act | mostly text; nrfutil has JSON mode [U] | Debian arm64 (nrfutil vendor binary [U]) | None | P2, same read/write split | [K] [picotool](https://github.com/raspberrypi/picotool) |
| sigrok-cli | Passive electrically (high-impedance probes) but a physical connection | CSV/bits/hex + decoder annotations | Debian arm64 | Long captures at high rates fill RAM/disk | **P1** | [V] [sigrok-cli wiki](https://sigrok.org/wiki/Sigrok-cli) |
| Saleae Logic 2 automation | Passive | gRPC/Python API | arm64 build reported unstable in a forum thread title [U] | Electron app | Skip (hardware cost; sigrok covers it) | [U] [Saleae forum](https://discuss.saleae.com/t/crashing-in-raspberry-pi-arm64-linux-environment/3723) |
| UART baud detection (logic capture + shortest-pulse measurement, sigrok uart decoder) | Passive | derived number | n/a | None | **P1** as a small native Rust routine | [K] |
| JTAG/SWD pin finders (JTAGenum, blueTag on a Pico) | Active: drives pins on the target | text over serial | Pico firmware | None | P3, tier "act" (can back-power or stress pins), owned boards only | [K] [blueTag](https://github.com/Aodrulez/blueTag) |
| Bus Pirate 5/6, Glasgow | Mixed; applets can drive buses | serial/binary modes; Python applets | USB serial | None | P3 | [K] [Glasgow](https://glasgow-embedded.org/) |
| ChipWhisperer | Active fault/side-channel | Python API | USB | None | Exclude for now (budget, niche) | [K] [ChipWhisperer](https://chipwhisperer.readthedocs.io/) |
| Modbus/CAN fuzzers | Active, deliberately malformed traffic | n/a | n/a | n/a | **Exclude** on live equipment (see inference) | — |
| ESPHome / Tasmota flashing | Act (re-flash) | YAML in, logs out | Python | Compile on the Mac | P3, owned devices, arm switch | [K] [ESPHome](https://esphome.io/) |

### Inferences
- Division of labour: the Pi captures, reads and extracts (I/O-bound, small tools); the Mac (16 GB M3, per stored context about the owner's gear) decompiles and runs heavy analysis; artifacts move by content hash, so the knowledge store can prove the Mac analysed exactly the bytes the Pi read.
- Reading flash is not risk-free: in-circuit SPI reads can back-power a board, and entering a bootloader resets the device. That is why reads are "measure", not "observe", and should be bound to a device serial in the owned-assets registry.
- Fuzzing a live inverter, battery management system or vehicle bus is a bad idea for physical reasons, not legal ones: malformed or out-of-range writes can trip protection, corrupt configuration, or command actuators, and on a vehicle can set fault codes or disable safety functions. Fuzzing belongs against a bench replica or simulator, never the installed unit.
- eFuse burning and secure-boot enabling are one-way; they should not be reachable by the agent at all, even behind the arm switch.
- Running two independent extractors (binwalk v3 and unblob) and recording where they disagree is cheap and catches both tool bugs and unusual containers.

### Gaps
- unblob aarch64 wheel/Docker availability and its sandbox mechanism were not confirmed on the pages read.
- Ghidra-on-Pi performance numbers were not found; the "Mac" verdict rests on JVM memory reasoning, not a benchmark.
- probe-rs, flashrom, esptool, espflash, nrfutil and picotool facts are from prior knowledge; current versions and output formats were not re-verified (esptool renamed commands in its v5 line [K]; flashrom has a fork, flashprog [K]).
- The sigrok JSON trace option was not on the wiki page read.

---

## 5. Integration architecture options and trade-offs; sandboxing on a Pi; long-running jobs

### Takeaway
Option (a) a single generic "run command" tool is the worst for both safety and token cost and is exactly what the published CVEs and injection papers exploit; the right design for this project is curated typed wrappers (b) with parsed outputs, optionally fronted by "code mode" (d) for multi-step analysis, all run under Linux-kernel sandboxing (namespaces, Landlock, seccomp, cgroups, nftables) so a tool physically cannot reach anything but an allow-listed interface. Long-running captures/scans should be modelled as the project's own handles plus the MCP tasks extension.

### Cited Findings
- Anthropic's "code execution with MCP" (4 Nov 2025) reports dropping a Drive-to-Salesforce workflow from about 150,000 to about 2,000 tokens (98.7%) by presenting MCP servers as code APIs the agent imports on demand, filtering data in the execution environment before the model sees it, and saving intermediate results to files; it explicitly warns that running agent-generated code needs a secure sandbox, resource limits and monitoring. [V] — [Anthropic Engineering](https://www.anthropic.com/engineering/code-execution-with-mcp)
- Anthropic's "advanced tool use" (24 Nov 2025) adds three primitives: the Tool Search Tool (one example 77K to 8.7K tokens, 85% cut; MCP-eval accuracy Opus 4.5 79.5% to 88.1%), Programmatic Tool Calling (43,588 to 27,297 tokens average, 37%), and Tool Use Examples (complex-parameter accuracy 72% to 90%); `defer_loading: true` makes a tool discoverable by search without loading its schema upfront, and whole MCP servers can be deferred while keeping a few high-use tools loaded. Tool Search is recommended when definitions exceed 10K tokens or there are 10+ tools. [V] — [Anthropic Engineering: advanced tool use](https://www.anthropic.com/engineering/advanced-tool-use)
- MCP 2026-07-28 is stateless: no `initialize` handshake and no `Mcp-Session-Id`; servers needing cross-request state mint an explicit handle returned as an ordinary tool argument, and version/capabilities travel in `_meta` each request (SEP-2567, SEP-2575). Tasks moved to the official `io.modelcontextprotocol/tasks` extension (SEP-2663): polling via `tasks/get`, client input via `tasks/update`, no `tasks/list`, and servers may return a task handle unsolicited. `tools/list` results carry `ttlMs` and `cacheScope` for caching (SEP-2549); input/output schemas accept full JSON Schema 2020-12 and `structuredContent` any JSON value (SEP-2106). [V] — [MCP 2026-07-28 changelog](https://modelcontextprotocol.io/specification/2026-07-28/changelog.md)
- The spec's State Handle Hijacking guidance: servers MUST NOT treat possession of a handle as authentication, SHOULD use non-deterministic handles from a secure RNG, and SHOULD bind them server-side to the authenticated principal. Its Local MCP Server Compromise guidance: prefer stdio to limit access; for HTTP transport require an auth token or use a restricted-access Unix domain socket; run servers sandboxed with minimal default privileges and restricted filesystem/network. [V] — [MCP security best practices](https://modelcontextprotocol.io/specification/2026-07-28/basic/security_best_practices.md)

### Inferences
- Recommended layering for this project (maps onto the six existing meta-tools list/describe/read/invoke/capture/query):
  - **(b) typed wrappers as the backbone.** Each instrument/tool is a module exposing a typed schema; `invoke` dispatches to it with validated, allow-listed arguments; outputs are parsed into typed fields, never raw stdout. This is what PentestMCP and Burp's server do, and it is the only option where an argument allow-list and a target allow-list can actually be enforced.
  - **(d) code mode as an accelerator for analysis**, not for driving instruments. The agent writes a short Rust/Python script against a typed SDK that runs in the sandbox next to the stored captures (DuckDB/Parquet), returning only the answer. This is where the 98.7%/37% token wins live, and it keeps multi-gigabyte data out of context. Instrument control (anything that touches hardware or RF) stays in typed wrappers so every physical action goes through the policy check; data crunching goes to code mode.
  - **(c) the constrained container with a policy file** is the isolation substrate under both: the Kali-rolling toolbox runs with no network namespace reach except an allow-listed interface, a Landlock/bind-mount view of only the current run's scratch dir, seccomp, and a cgroup cap.
  - **(e) skills/sub-agents carry the knowledge** (how to read a Modbus capture, what a healthy EG4 frame looks like) so schemas stay small; `defer_loading` keeps all but a handful of tools out of the base prompt and the Tool Search Tool surfaces the rest.
  - **Generic "run command" (a): excluded** except as a break-glass tool that is off by default, logged, and requires the physical arm switch.
- Sandboxing stack for the Pi (defence in depth, each layer cheap):
  - **Capabilities:** only `dumpcap` gets `CAP_NET_RAW`/`CAP_NET_ADMIN` (file capabilities or an ambient-cap helper); every analysis tool runs with none.
  - **Network namespaces + a veth to one allow-listed interface** so a scan tool literally cannot send to the WAN or to a subnet not in the registry; pair with **nftables** egress rules keyed to owned MAC/IP ranges.
  - **user namespaces + bubblewrap** for rootless isolation of the toolbox; **Landlock** to restrict each run to its scratch directory; **seccomp** to drop exotic syscalls; **systemd** unit sandboxing directives (`ProtectSystem=strict`, `PrivateTmp`, `NoNewPrivileges`, `RestrictAddressFamilies`, `IPAddressDeny/Allow`, `DeviceAllow` for just the one USB instrument).
  - **cgroups v2** memory and CPU caps per run so a scan cannot starve the sensor daemon (critical on 4 GB); read-only root; per-run scratch wiped after the result and its hash are stored.
- Long-running model: the project's own `capture`/scan returns immediately with a server-minted handle (capture_id); status via the tasks extension `tasks/get`; the final result is a small summary plus a resource link to the stored pcap/Parquet. This matches the stateless spec (handle as a tool argument) and avoids holding a stream open on a flaky LoRa/Wi-Fi link.
- The 25,000-token result cap is handled the bx33661 way: every tool returns a summary plus a resource link/handle; detail is paged with offset/limit/filter; nothing returns a raw multi-MB blob.

### Gaps
- Whether Claude Code (the client here) consumes the MCP tasks extension, resource links and structured output today was not confirmed; the Claude blog said support was "rolling out." Verify against the client in use before relying on tasks.
- No benchmark was found for code-mode token savings on packet/firmware data specifically; the 98.7%/37% numbers are from Anthropic's office-document examples.

---

## 6. Treating tool output as untrusted (prompt injection from observed data)

### Takeaway
Tool output is attacker-controllable data, not instructions: SSIDs, hostnames, HTTP banners, BLE/device names, firmware file contents and pcap payloads can all carry injected text. 2025-2026 research shows this reliably hijacks AI pentest agents, in one demo to a reverse shell within 20 seconds; the mitigations that actually move the needle are structural (typed fields, provenance tags, a detector, human approval for actions), not prompt wording.

### Cited Findings
- "Cybersecurity AI: Hacking the AI Hackers via Prompt Injection" (Mayoral-Vilches & Rynning, arXiv 2508.21669v2, v2 15 Nov 2025): malicious text hidden in trusted content such as a web server's response is processed by an AI pentest agent as instructions, hijacking execution flow and potentially granting system access; they call prompt injection "a recurring and systemic issue in LLM-based architectures," analogous to XSS. [V] — [arXiv 2508.21669v2](https://arxiv.org/abs/2508.21669v2)
- Press account of the same work: the CAI agent received a "NOTE TO SYSTEM" prefix in HTTP content, decoded a base64 payload and ran a reverse shell, giving the attacker a shell on the tester's infrastructure within 20 seconds; simple pattern filters can be evaded with base32/hex/ROT13, code comments, env-var output, or Unicode homographs. Mitigations listed: isolate tool execution in containers; detect shell substitutions like `$(id)` in curl/wget wrappers and wrap external content in "DATA ONLY" markers; intercept file-writes that decode payloads; use a second AI step to separate real evidence from instructions; enforce separate analysis-only and execution-only channels. [V] — [Cyber Security News, 3 Sep 2025](https://cybersecuritynews.com/ai-powered-tools-turned-against-themselves/)
- "Know Your Agent: Reconnaissance-Driven Pentesting of AI Agents" (Eliav et al., arXiv 2607.19837, 2026): the strongest defence tested was a transformer-based prompt-injection detector, cutting attack success from 86.0% to 30.4% on AgentDojo; repeating the user prompt helped (to 53.0%); merely delimiting tool outputs barely helped (86.3%). [V] — [KYA paper](https://huggingface.co/papers/2607.19837.md)
- TROOPERS 2026 "Pwning AI Pentesters" (Markus Vervier): autonomous agents can be steered by planted content (issues, tickets, profile bios, wiki pages) into following attacker-controlled SSO redirects and leaking credentials; the recommendation is that agents should not act on untrusted recon data without a human in the loop. [V] — [TROOPERS 2026 slides](https://troopers.de/downloads/troopers26/TR26_Pwning_AI_Pentesters_FB8PAJ.pdf)
- OWASP MCP Top 10 names this as MCP06 (prompt injection via contextual payloads / intent-flow subversion) and MCP03 (tool poisoning, where tool outputs inject misleading context). [V] — [OWASP MCP Top 10](https://owasp.org/www-project-mcp-top-10/)

### Inferences
- Concrete rules for this project: (1) every observed string (SSID, hostname, banner, BLE name, filename/strings inside firmware, DNS names, pcap payload bytes) is returned in a typed field tagged `provenance: observed`, never spliced into prose the model reads as narration; (2) length caps and escaping on all such fields; (3) observed text may never select the next action: the policy layer, not the model, decides what runs, and the model's tool call is checked against the allow-list regardless of what a banner "said"; (4) a prompt-injection detector on observed blobs before they enter context (KYA's best defence); (5) the act tier always requires the human, matching the TROOPERS recommendation; (6) delimiting alone is treated as near-worthless, consistent with the KYA 86.3% result.
- Because the owner analyses his own gear, the realistic threat is less a live adversary and more a compromised IoT device, a malicious firmware blob, or a spoofed device name on the network; the same structural mitigations cover it.

### Gaps
- No quantified success-rate numbers were captured from the CAI paper body (only the abstract was read); the "20 seconds" figure is from the press write-up of it.

---

## 7. Scope and legality for a hobbyist on his own property (US) — short, not legal advice

### Takeaway
On your own networks and devices the passive and diagnostic work here is plainly in bounds; the bright lines are other people's traffic, transmitting to disrupt (deauth/jam), and touching systems you don't own. Encode "owned" as a registry that active tools are bound to, keep everything else observe-only, and keep transmit-capable tools physically disabled unless an arm switch is on. This is general information, not legal advice.

### Cited Findings
- FCC Enforcement Advisory 2015-01 (DA 15-113, 27 Jan 2015): intentionally blocking or disrupting personal Wi-Fi, and operating/marketing/selling jammers, is prohibited under Section 333 of the Communications Act; the advisory cites Marriott's use of a Wi-Fi deauthentication protocol to block guests, settled for a $600,000 civil penalty. [V] — [FCC DA 15-113](https://transition.fcc.gov/eb/Public_Notices/DA-15-113A1.html)
- The Copyright Office's ninth triennial Section 1201 final rule is dated 28 October 2024 and exemptions run three years; the tenth triennial proceeding has been initiated (streamlined renewals, next period Oct 2027-Oct 2030). [V] — [copyright.gov/1201/2024](https://copyright.gov/1201/2024/), [copyright.gov/1201](https://www.copyright.gov/1201/)
- The current DMCA 1201 exemptions include good-faith security research on lawfully acquired devices (conditions: solely for good-faith testing, in an environment designed to avoid harm, and not violating other laws such as the CFAA) and diagnosis/maintenance/repair of lawfully acquired devices including motorized land vehicles and consumer devices; the 2024 rule renewed and in places expanded these. [K] — [2024 Register's Recommendation](https://www.copyright.gov/1201/2024/2024_Section_1201_Registers_Recommendation.pdf), [2024 final rule PDF](https://govinfo.gov/content/pkg/FR-2024-10-28/pdf/2024-24563.pdf)

### Inferences
- Plainly fine: passive capture on your own network; scanning and port-probing your own devices; analysing firmware of devices you own for repair/interoperability; receive-only RF monitoring of the spectrum. Not fine: capturing or decrypting other people's traffic; deauth/jamming/injection affecting anyone else's gear (FCC Section 333); accessing systems without authorisation (CFAA); cracking a neighbour's Wi-Fi.
- The 1201 security-research and repair exemptions matter here because reading flash and extracting firmware of a device you own can implicate access controls; the conditions (lawfully acquired, good faith, no other-law violation) map directly onto the owned-assets registry and the observe/measure/act tiers.
- How the tool layer encodes it: an owned-assets registry of MACs, SSIDs/BSSIDs, IP subnets, VINs, device serials and SDR frequency ranges you may actively touch; everything else is observe-only; transmit-capable tools (any TX, deauth, injection, active RFID write, bus write) are disabled unless the physical arm switch is on, and even then are bound to a registry entry. Receive-only RF is the default.

### Gaps
- The exact 2024 exemption wording (paragraph numbers in 37 CFR 201.40) was not read this session; both direct fetches of the eCFR and the terms page failed. The specific conditions should be verified against the final rule before being quoted. Not legal advice.

---

## 8. Token- and evidence-friendly outputs per tool, and mapping to the knowledge store

### Takeaway
Every wrapper returns a compact typed summary plus a resource link/handle to the full artifact; the summary is what the model reads, the artifact is what gets hashed into the knowledge store as typed facts with evidence hashes.

### Cited Findings
- The bx33661 server already demonstrates the pattern: summaries, 8,000-char head-and-tail truncation with paging, and an optional DuckDB index over the capture. [V] — [wireshark-mcp](https://pypi.org/project/wireshark-mcp)
- PCAP-LM keeps a side index from the compact form back to original frame numbers so raw packets can be retrieved when needed. [V] — [APNIC PCAP-LM](https://blog.apnic.net/2026/11/06/pcap-lm-helping-ai-assistants-read-network-traffic/)
- unblob's `--report` already emits exactly the per-chunk JSON (offset, size, entropy, permissions, timestamps) that a "tree of extracted components with entropy flags" needs. [V] — [unblob](https://github.com/onekey-sec/unblob)

### Inferences — concrete compact outputs
- **nmap home subnet:** a table of host / MAC / vendor / open-ports (service+version), plus counts and "new since last scan", plus a resource link to the `-oX` XML. (~1 row per host, not the XML.)
- **tshark capture:** the "capture card" = protocol hierarchy (`io,phs`), top talkers (`conv,ip` top N), top services, expert-info anomaly counts, DNS/HTTP tree highlights, plus a handle for display-filter queries and a link to the pcapng. Numbers reference frame ranges.
- **binwalk/unblob firmware:** a tree of extracted components with type, offset, size and an entropy flag (high-entropy = compressed/encrypted), plus secrets-scan hits as typed fields, plus a link to the extraction dir; disagreements between the two extractors flagged.
- **flash read:** chip id, size, a sha256 of the dump, detected filesystem/bootloader, plus a link to the image.
- **Wi-Fi/BLE/Zigbee survey (Kismet):** a device table (BSSID/MAC, type, channel, signal, first/last seen, "is this an owned asset?") plus counts, from the REST JSON, not the kismetdb.
- **Modbus/CAN/serial decode:** a frame-type histogram, unique unit/arbitration IDs, decoded register/signal ranges via DBC/profile, anomalies, plus a link to the decoded Parquet.
- **Knowledge store mapping:** each finding becomes a typed fact (subject = an owned-asset id; predicate = e.g. `has_open_port`, `firmware_sha256`, `modbus_register`, `ble_service`; object = value; evidence = sha256 of the artifact; timestamp; tool+version+args). This riffs on the archived IsoKron typed-graph + hash-chain design the owner wants to reuse (per stored project context) and lets the hash-chain prove the Mac analysed the same bytes the Pi captured.

### Gaps
- None beyond those above; these are design proposals, not external findings.

---

## 9. Prior art for this exact combination (Pi pentest/sensor boxes, and LLM-operated boxes)

### Takeaway
There is a decade of Pi-based "drop box" and SDR prior art (Pwnagotchi, P4wnP1, Kali NetHunter, WiFi Pineapple, DragonOS, Kismet-on-Pi), which got the hardware, radio and portability right but are human-operated and attack-oriented; what is new in 2025-2026 is LLM operation via MCP (Kali's own Claude Desktop workflow, mcp-kali-server, PentestMCP), which so far got the agent plumbing but not the safety/scope layer.

### Cited Findings
- Pwnagotchi is an active, maintained Pi-based Wi-Fi project (jayofelony fork); it is explicitly about capturing WPA handshakes, i.e. an active/opportunistic posture. [V, search-level] — [jayofelony/pwnagotchi](https://github.com/jayofelony/pwnagotchi)
- Kali's own 2026 Claude-Desktop-over-SSH workflow and `mcp-kali-server` are the closest published prior art to this project's topology, and their only gate is the client approval prompt. [V] — [Kali blog](https://www.kali.org/blog/kali-llm-claude-desktop/), [Kali tools](https://www.kali.org/tools/mcp-kali-server/)
- Kali's Nexmon work makes the onboard Pi radio monitor-mode capable, which is what Kismet-on-Pi and Pwnagotchi-style builds rely on. [V] — [Kali blog, Pi Wi-Fi](https://www.kali.org/blog/raspberry-pi-wi-fi-glow-up/)

### Inferences
- What prior art got right and worth copying: compact status display (Pwnagotchi's e-ink face → the owner's wanted LED/status output), battery/portability patterns (PiSugar/cyberdeck), SDR-on-Pi distros (DragonOS) proving the receive chain works on ARM, Kismet as the survey engine, NetHunter's USB-gadget tricks (P4wnP1) for the "attach to a device" use case.
- What they got wrong for this project's purpose: all are human-operated and most default to active/attack behaviour; none has an owned-assets registry, risk tiers, a write-gate/arm-switch, untrusted-output handling, or a hash-chained evidence store. The project's differentiator is precisely the safety/scope/evidence layer on top of well-trodden hardware.
- WiFi Pineapple and Hak5 tooling expose REST APIs but are commercial and transmit-oriented; useful only as API-shape references, not as components.

### Gaps
- No 2026 project was found that combines an LLM agent, MCP, and a physical sensor pack with a safety/scope layer — this appears to be genuinely novel territory, though absence of a search hit is not proof.
- DragonOS Pi image current status and P4wnP1 maintenance status were not verified this session.

---

## 10. DELIVERABLES

### (1) Recommended architecture — one paragraph + component list
A Rust supervisor daemon on the Pi 5 exposes the six fixed MCP meta-tools (list/describe/read/invoke/capture/query) over MCP 2026-07-28 (stateless; long-running captures and scans return a server-minted handle and are polled through the tasks extension; every result is a compact typed summary under ~25k tokens plus a resource link to a stored artifact). Instruments and software tools are self-describing modules behind typed, allow-listed wrappers; a separate code-mode sandbox lets the agent run short scripts against stored captures (DuckDB/Parquet) so multi-gigabyte data never enters the model context. Everything runs under layered kernel sandboxing — only `dumpcap` holds `CAP_NET_RAW`; analysis tools run in a rootless Kali-rolling toolbox with a network namespace pinned to one allow-listed interface, nftables egress bound to the owned-assets registry, Landlock + per-run scratch dirs, seccomp, and cgroup memory/CPU caps so nothing starves the sensor daemon. A policy engine enforces three risk tiers (observe / measure / act); observed strings are typed, provenance-tagged, length-capped data that can never select the next action; the act tier and any transmit-capable tool require the physical arm switch and a registry binding. Findings land in a hash-chained typed-graph knowledge store as typed facts with evidence hashes, so the Mac (which runs Ghidra/heavy RE as its own MCP servers) can be proven to have analysed exactly the bytes the Pi captured.
Component list: Rust supervisor + meta-tools; module registry; typed wrapper layer; code-mode sandbox (DuckDB/Parquet store); policy engine + owned-assets registry + arm switch; sandbox substrate (netns/nftables/Landlock/seccomp/cgroups/systemd units); capture layer (dumpcap, privileged, isolated); knowledge store (typed graph + hash chain); prompt-injection detector on observed blobs; LoRa/Meshtastic back-feed + status LEDs + speaker.

### (2) First 10 capabilities (priority order; all bound to owned assets)
| # | Capability | Risk tier | Inputs | Outputs | Underlying tool(s) |
|---|---|---|---|---|---|
| 1 | Capture-and-summarise on an interface | observe | iface, duration/ring, optional display filter | capture card (phs, talkers, expert, DNS/HTTP), handle + pcapng link | dumpcap + tshark `-z`/`-T ek` |
| 2 | "What's on my network" inventory | measure | owned subnet | host/MAC/vendor/ports table, diff vs last, XML link | nmap `-oX` + arp-scan (+ passive capture) |
| 3 | Query a stored capture | observe | capture handle, display filter / SQL | ≤N frames or aggregate rows, frame refs | tshark `-Y -j` + DuckDB over Parquet |
| 4 | Modbus/CAN/serial decode of a capture | observe | capture/log handle, DBC/profile | frame-type histogram, unit/IDs, decoded ranges, anomalies | tshark dissectors, cantools, sigrok uart |
| 5 | Wi-Fi/BLE/Zigbee environment survey | observe (RX only) | band(s), dwell | device table (id, type, chan, signal, owned?) + counts | Kismet REST, btmon, rtl_433 |
| 6 | Firmware identify-and-extract (file) | measure | firmware file path | component tree w/ entropy flags, secrets hits, extraction link, extractor-disagreement flag | binwalk v3 + unblob, strings/secret scan |
| 7 | Flash-read of an owned device | measure | device serial (registry), interface | chip id, size, sha256, detected FS/bootloader, image link | flashrom / esptool / probe-rs (read) |
| 8 | Logic-analyzer capture + protocol decode | measure | channels, rate, decoder stack, trigger | decoded annotations (csv/bits), bus summary, raw-capture link | sigrok-cli `-P` |
| 9 | Binary triage of an extracted component | observe | component path | sections, imports, strings, functions, suspicious flags (summary) | radare2/rizin JSON (Pi); hand heavy RE to Ghidra on the Mac |
| 10 | SBOM + CVE lookup on extracted firmware | observe | extraction dir | CycloneDX SBOM + CVE findings (summary + report link) | syft + grype / cve-bin-tool |

(Dogfooding note, per the owner's stated preference: #1, #3 and #6 are the tools used to build and validate the later ones, so they come first.)

### (3) Policy-file sketch
`owned-assets.toml` (or typed-graph nodes): per asset — id; type (host/ap/vehicle/inverter/battery/board/tag); mac[]; bssid[]/ssid[]; ip_subnet[] (CIDR); vin; serial; sdr_freq_ranges[] you may actively touch; interfaces you may transmit on; notes; added_by/added_at; hash. Anything not matching an entry is observe-only.
`tool-policy.toml`: per tool/module — name; version pin (digest); risk_tier (observe/measure/act); allowed_args (allow-list / regex / enum, never free-form shell); forbidden_args; target_binding (which owned-assets fields it must match: subnet/mac/ssid/serial/freq); transmit (bool); requires_arm_switch (bool); requires_human_approval (bool); rate_limit (calls/min, concurrent); timeout; cgroup_mem/cpu; netns (which iface, or none); landlock_paths; seccomp_profile; output_cap_tokens; log_level. Global: default_deny; receive_only_default=true; arm_switch_gpio; audit_log (append-only, hash-chained); injection_detector=on.

### (4) Top 8 risks
1. **Command injection in a wrapper** — a single unsanitised argument = host compromise, demonstrated by HexStrike CVE-2025-35028 (CVSS 9.1, runs as root). Mitigate: argument arrays, no shell, strict allow-lists, non-root, least capability. [V]
2. **Prompt injection via observed data** — banners/SSIDs/hostnames/firmware strings steer the agent; CAI demo reached a reverse shell in ~20s. Mitigate: typed provenance-tagged fields, detector, policy decides actions not the model. [V]
3. **Scope escape** — a typo'd subnet or a neighbour's SSID actively probed. Mitigate: owned-assets registry + netns/nftables so the tool physically can't reach non-allow-listed targets. [V/inference]
4. **Transmit/disrupt actions** — deauth/jam/inject violate FCC Section 333 and can damage others' service. Mitigate: receive-only default, transmit tools disabled unless arm switch + registry binding; exclude deauth/jam tools entirely. [V]
5. **Physical/irreversible writes** — eFuse burn, secure-boot enable, flashing or fuzzing a live inverter/vehicle. Mitigate: act tier + human + arm switch; eFuse burn and live-equipment fuzzing excluded from the agent. [inference]
6. **Resource starvation on 4 GB** — a scan or a big tshark two-pass starves the sensor daemon. Mitigate: cgroup caps, slice-before-analyse, offload heavy RE to the Mac. [V/inference]
7. **Token blowout / truncated evidence** — raw tshark output is ~22M tokens; the important line gets truncated away. Mitigate: summaries + handles + paging + code mode. [V]
8. **Weak audit + supply chain** — chat transcript is not an audit trail; a poisoned tool package or container. Mitigate: append-only hash-chained audit log; pin container/tool by digest; OWASP MCP01/04/08. [V]

### (5) Outside-the-box ideas (agent with both physical instruments and software analysis)
1. **Cross-sensor correlation:** align a pcap of a bus with a logic-analyzer trace of the same wires at the same timestamp (shared GPS/PTP clock), so the agent can say "this Modbus exception on the wire corresponds to this voltage glitch," and store the aligned pair as one evidence bundle.
2. **Firmware-diff-as-knowledge:** when an owned device updates, re-read flash, diff extracted components against the stored prior version, and record the delta (changed files, new strings, new CVEs from grype) as a typed fact chained to both firmware hashes — a longitudinal device history.
3. **Protocol learning loop:** for an undocumented device (e.g. an EG4 inverter frame), the agent captures, hypothesises a field layout, predicts the next frame, captures again to confirm, and promotes a confirmed `.proto`/DBC/register map into the knowledge store as reusable decoder knowledge — dogfooding the store.
4. **Physical ground-truth for a software claim:** after nmap reports a service, the agent confirms the device is really the owner's by correlating MAC + a physical signal (BLE name, a blink the owner triggers, an SDR fingerprint) before any measure-tier action — observed-data-can't-select-action, enforced physically.
5. **Health baseline + drift alarm:** nightly passive capture + survey builds a baseline (talkers, services, RF neighbours, device firmware hashes); the agent raises a typed "drift" fact when something changes, and can speak it over the pack speaker / LoRa back-feed at a far spot on the property.
6. **Soldering-to-signal proof:** the bench camera watches a joint being made; after the solder, the agent drives probe-rs/sigrok to electrically prove continuity/signal integrity on that exact net, linking the video frame, the electrical measurement and the pass/fail as one evidence record.
7. **Replica-bench fuzzing:** never fuzz the installed inverter/vehicle; instead the agent captures real frames, replays them against a bench replica or simulator, fuzzes there, and promotes only safe, validated findings back — keeping the "act" risk off live equipment.
8. **Two-extractor / two-model disagreement as a signal:** run binwalk v3 and unblob (and, for pcaps, two agent passes) and treat any disagreement as an automatic "needs human / needs deeper look" flag stored in the knowledge store — cheap robustness against tool and model error.

---

## Overall gaps / caveats for the report writer
- Several per-tool facts are [K] (prior knowledge, canonical docs linked but not re-fetched): tcpdump, Zeek, Suricata, can-utils, probe-rs, flashrom, esptool/espflash, OpenOCD, radare2 output modes, trufflehog/gitleaks, syft/grype, bettercap, aircrack-ng suite details. Current versions/flags should be confirmed before quoting exact options.
- The entire offensive-credential/exploitation/web-attack tool class was deliberately not inventoried row-by-row (a listing was withheld during writing); the project-level verdict is Exclude from the agent. If the writer needs those rows, request them separately, scoped to owned equipment.
- r2mcp README, exact DMCA 1201 exemption wording, unblob aarch64 packaging, Ghidra-on-Pi benchmarks, and Zeek/Kismet/ntopng Raspberry Pi OS trixie packaging are unverified.
- No official Wireshark Foundation MCP server or built-in AI feature was found (searched twice); treat as "none known," not "confirmed none."
- Star counts: GhidraMCP 11,969; r2mcp 366; HexStrike (not read); Wireshark MCP servers (not shown). PentAGI/Shannon/Strix/Cairn stars are from the Stingrai article dated 13 Nov 2026.
