# Prior art and design space: local-first, hash-verified, versioned equipment knowledge store for an AI agent (Rust, Raspberry Pi class, two-node sync) — state as of 2026-11-25

Reading guide. "Cited Findings" are things fetched and read this session, with the source linked. "Inferences" are the researcher's reasoning and, where marked **[BK]**, background knowledge that was *not* re-verified this session (treat as unverified until checked). IsoKron and kronicle-lite were deliberately not researched (another reviewer has them).

---

## 1. Content-addressed and verifiable storage: what each candidate guarantees, and a verdict

### Takeaway
No single off-the-shelf system gives integrity + full history + authenticity + offline two-node sync + Rust + small footprint; the proven pieces that do compose are (a) a plain content-addressed blob directory keyed by BLAKE3, (b) an append-only signed log with Merkle checkpoints in the Certificate-Transparency "tlog" style, and (c) SQLite as a rebuildable index. The pieces most tempting to adopt wholesale (iroh-blobs, iroh-docs, Willow, cr-sqlite, DoltLite) are each either explicitly not production-quality, not history-preserving, unfinished, or very young as of November 2026.

### Cited Findings

**iroh (Rust, n0-computer)**
- iroh 1.0 shipped 2026-06-15 as the first stable release after 65 versions; the stability promise covers the wire protocol and the language APIs, and a wire-breaking change will only happen at a major version — [iroh blog, "Iroh 1.0"](https://iroh.computer/blog/v1)
- The 1.0 promise is about the connectivity layer only: `iroh-blobs` is at 0.105.0 (2026-11-13), depends on iroh ^1.0.0, and its own page says the current line is not yet production quality and points production users to iroh-blobs 0.35. Transfer is a request/response over a QUIC stream where the provider answers with BLAKE3 verified-streaming data — [docs.rs iroh-blobs](https://docs.rs/crate/iroh-blobs/latest)
- iroh 0.35 gets no further releases and its public relay support ends 2026-12-31 — so "use iroh-blobs 0.35 for production" is a dead end for a new project — [iroh blog, "Iroh 1.0"](https://iroh.computer/blog/v1)
- `iroh-docs` is at 0.101.0 (2026-06-15, iroh ^1): a multi-dimensional key-value store where an entry is identified by namespace + author + key, replicas reconcile with range-based set reconciliation (Aljoscha Meyer's paper), and the content bytes travel separately (via blobs) — [docs.rs iroh-docs](https://docs.rs/crate/iroh-docs/latest)
- LAN discovery is opt-in via two separate crates in the 1.x era: `iroh-mdns-address-lookup` (0.5, swarm-discovery based, suited to long-running endpoints) and `iroh-mdns-peer-lookup` (0.1, standard mDNS/DNS-SD); two devices on one LAN can connect directly without a relay, but the docs' examples still use the N0 preset (DNS + relays) and do not show a fully offline configuration — [iroh docs, local address lookup](https://docs.iroh.computer/connecting/local-address-lookup.md); default address lookup (DNS/pkarr via dns.iroh.link) needs internet — [iroh docs, address lookup](https://docs.iroh.computer/concepts/address-lookup.md)
- `bao-tree` 0.16.1 (2026-08-26, MIT/Apache-2.0, owned by the iroh maintainers) is the Merkle-tree crate behind BLAKE3 verified streaming: runtime-configurable chunk groups, multi-range queries in one request, pre-order/post-order/custom outboards (post-order suits append-only files), wire-compatible with the original `bao` crate — [docs.rs bao-tree](https://docs.rs/crate/bao-tree/latest)

**BLAKE3**
- BLAKE3 is internally a Merkle tree, which is what enables parallel hashing, incremental update and verified streaming; the Rust crate ships SSE2/SSE4.1/AVX2/AVX-512/NEON/WASM implementations; it also has `keyed_hash` and `derive_key` modes; licence CC0 or Apache-2.0 — [BLAKE3 repo](https://github.com/BLAKE3-team/BLAKE3)

**Transparency logs (tlog-tiles, checkpoints, Tessera, Sigsum)**
- Tessera 1.0 (announced 2025-09-22, Google TrustFabric) is a Go *library* (not a service) for C2SP tlog-tiles logs; it has a POSIX-filesystem backend, reports >10,000 writes/s on POSIX, and has built-in witnessing — [transparency.dev blog](https://blog.transparency.dev/tessera-v1-0-is-here)
- A C2SP checkpoint is a signed note whose body is three lines — origin, tree size, base64 RFC 6962 root hash — followed by signature lines; plain Ed25519 log signatures are supported, plus timestamped cosignatures (Ed25519 and ML-DSA-44); a log or cosigner must never sign a checkpoint inconsistent with one it already signed, and "inconsistent" means no consistency proof can be built between the two — [c2sp.org/tlog-checkpoint](https://c2sp.org/tlog-checkpoint)
- A Rust implementation exists: `tlog_tiles` 0.2.0 (2025-04-18, BSD-3-Clause, Cloudflare research / `cloudflare/azul`), a port of Go's tlog package implementing tlog-tiles and tlog-checkpoint — [docs.rs tlog_tiles](https://docs.rs/crate/tlog_tiles/latest). Last release is 19 months old as of this writing.
- Sigsum is a minimal transparency system for *signing-key usage*: accepted signatures must appear in a public log, and the threat model assumes the attacker may control the signer's key, the log and a threshold of witnesses — [sigsum.org](https://www.sigsum.org/)

**Fossil**
- A Fossil repository is an unordered set of hash-named artifacts; sync is set exchange — `igot` announces, `gimme` requests, `file`/`cfile` delivers (optionally as a delta) — looping over stateless HTTP POST round trips until the client holds everything the server has; "phantoms" track known-but-missing artifacts; unversioned files sync separately and keep only the newest-mtime copy; login is a SHA1-based nonce/password signature — [Fossil sync protocol](https://fossil-scm.org/home/doc/trunk/www/sync.wiki)

**Dolt / DoltLite (Prolly trees)**
- DoltLite is a fork of SQLite that swaps the B-tree layer for a Prolly tree in a single-file chunk store; it reached Beta at 0.50.0 on 2026-08-31, five months after launch; branches, merges, diffs, rebases, cherry-picks, push/pull/clone are supported; file-backed reads are at parity with SQLite, batched writes ~10% slower, small autocommit writes ~3.1x slower; there is no WAL/journal sidecar; it passes 100% of sqllogictest and 99.46% of SQLite's TCL tests — [DoltHub blog, DoltLite beta](https://dolthub.com/blog/2026-08-31-doltlite-beta)
- DoltLite is Apache-2.0, written in C under SQLite's `btree.h` seam, ships prebuilt Linux arm64 binaries and .deb packages, has a Rust crate (`cargo add doltlite`), supports `file://` and HTTP remotes (`doltlite-remotesrv`), and exposes `dolt_diff_<table>`, `dolt_history_<table>`, `dolt_at_<table>('rev')` and conflict tables — [dolthub/doltlite](https://github.com/dolthub/doltlite)

**Radicle (Rust, peer-to-peer git)**
- Radicle 1.8.0 "Drosera" (2026-03-30) hardened *signed references*: each node's refs entry now carries a monotonic feature level (`none` → `root` → `parent`), the node detects downgrade attempts, and a configurable minimum level can be enforced on fetch — [Radicle 1.8.0 release notes](https://radicle.dev/2026/03/30/radicle-1.8.0)

**restic / rustic**
- `rustic_core` 0.13.0 (2026-08-16, MIT/Apache-2.0) is the library behind rustic and reads/writes the restic repository format (encrypted, deduplicated); its own page warns the API is early-stage and will change — [docs.rs rustic_core](https://docs.rs/crate/rustic_core/latest)

**Willow / Earthstar**
- Latest Willow status entry is 2025-10-26: Meadowcap is Final, the Drop Format is Candidate, and the sync protocol (now "Confidential Sync", formerly WGPS) was *demoted to Proposal* until the Rust implementation is finished; no 2026 news entries — [willowprotocol.org changes](https://willowprotocol.org/more/changes/)

**Syncthing (Block Exchange Protocol)**
- BEP verifies per-block hashes (blocks 128 KiB–16 MiB), authenticates devices by TLS certificate (device ID = SHA-256 of the cert in the reference implementation) and tracks per-file version vectors; it has *no whole-folder root hash and no history* — [BEP v1 spec](https://docs.syncthing.net/specs/bep-v1.html)

**cr-sqlite / SQLite session extension**
- cr-sqlite (MIT) merges rows by primary key with per-column CRDTs; last-write-wins is the default available column type, counters and rich text were still unimplemented; the repo shows no published releases on its front page — [vlcn-io/cr-sqlite](https://github.com/vlcn-io/cr-sqlite)
- rusqlite 0.43.0 (2026-11-15) bundles SQLite 3.54.1 and exposes the `session` (changesets), `backup`, `blob`, `hooks` and `preupdate_hook` features — [docs.rs rusqlite](https://docs.rs/crate/rusqlite/latest)

**QLDB (managed ledger) as a cautionary tale**
- AWS ended QLDB support on 2025-07-31 and pointed users at Aurora PostgreSQL + pgAudit; AWS's own guidance conceded Aurora does not keep a permanent immutable change record, so the audit trail has to be produced and kept outside the database — [InfoQ, 2024-07](https://www.infoq.com/news/2024/07/aws-kill-qldb)

**gitoxide**
- `gix` is at 0.91.0 (2026-11-22), i.e. actively released but still pre-1.0 — [docs.rs gix](https://docs.rs/crate/gix/latest)

**MCAP**
- MCAP integrity is CRC32 only (chunk `uncompressed_crc`, `data_section_crc`, `summary_crc`, attachment `crc`); the spec defines no cryptographic or file-level content hash; chunks may be `zstd`, `lz4` or uncompressed; chunk and message indexes allow time-range reads; attachments and metadata records exist; the spec does not define recovery of truncated files — [MCAP spec](https://mcap.dev/spec)

### Inferences

Verdict table. "V" = verified this session (see findings above); **[BK]** = background knowledge, unverified this session.

| Candidate | Integrity / history / authenticity | Sync model | Rust maturity | Fit for 2–3 offline devices | Verdict |
|---|---|---|---|---|---|
| Git object model + packfiles | Merkle DAG over whole history; SHA-1 default, SHA-256 object format exists but interop is limited [BK]; authenticity via signed commits/tags | fetch/push; verifies objects on receipt [BK] | `gix` 0.91 (V), pre-1.0; shelling out to `git` is the boring option | Excellent for text facts; poor for multi-GB binaries | **Adopt for the "simplest thing" variant**; borrow the object model otherwise |
| git-annex / Git LFS / DVC | Content-addressed big files beside git; annex tracks which device holds which blob [BK] | annex: many transports, checksummed [BK] | None in Rust (Haskell / Go / Python) [BK] | git-annex's location tracking is exactly "which node has this recording" | **Borrow** (location tracking, numcopies); adopt only in the git variant |
| restic / rustic / borg / kopia | CDC dedup + encryption + snapshot tree; proves backup integrity, not fact-level history | push to repo | `rustic_core` 0.13, API unstable (V) | Good as a *backup target*, wrong shape as a queryable truth store | **Adopt for off-site backup only** |
| casync / desync, OSTree, Nix store | Content-addressed chunk/tree distribution for OS images [BK] | pull | n/a | Wrong granularity | **Avoid** (borrow nothing new) |
| IPFS / IPLD (CID, DAG-CBOR, CAR) | Self-describing hashes (multihash), canonical DAG-CBOR, CAR as a portable archive [BK] | bitswap / gateways; heavy daemon | Rust IPLD crates exist [BK] | Daemon far too heavy for a Pi on battery | **Borrow**: multihash-style self-describing digests, CAR-like single-file export. **Avoid** the network stack and IPFS Cluster |
| iroh endpoint 1.x | Mutual auth by Ed25519 endpoint keys, QUIC | direct LAN, mDNS crates | 1.0 stable wire (V) | Good; works LAN-only with the mDNS crates, but fully offline config must be tested (V: docs don't show it) | **Adopt as transport, optional** (plain rustls/quinn is the lower-dependency alternative) |
| iroh-blobs | BLAKE3 verified streaming, range requests | pull by hash | 0.105, "not production quality" (V) | Functionally ideal | **Borrow now / adopt when stable**: use `bao-tree` directly and keep the on-disk layout your own |
| iroh-docs | Signed entries keyed (namespace, author, key); newest entry per key wins; no history | range-based set reconciliation | 0.101 (V) | Good sync, but it is a mutable KV, not a ledger | **Borrow** set reconciliation; **avoid** as source of truth |
| Perkeep | Content-addressed blobs + signed "claims" that mutate permanodes — the closest *conceptual* ancestor of "signed facts over immutable blobs" [BK]; Go, slow-moving [BK] | blob sync | none | — | **Borrow idea** (permanode + signed claim) |
| Noms / Dolt / DoltLite | Prolly trees: history-independent Merkle index → cheap diff/merge/sync of *tables*; DoltLite beta (V) | push/pull/clone, file + HTTP remotes (V) | C with Rust crate (V) | Runs on arm64 Linux (V); 5-month-old engine, no WAL sidecar (V) | **Strong candidate to adopt as the index/state layer later; do not make it the only copy of truth yet** |
| Irmin, TerminusDB, XTDB, Datomic | Git-like or bitemporal immutable stores [BK]; OCaml / Prolog+Rust / JVM / proprietary-JVM | varies | none usable embedded on a Pi | JVM stores too heavy for 4 GB shared with everything else | **Borrow**: Datomic's "facts are never updated, only asserted/retracted with a transaction id" and XTDB's valid-time vs transaction-time split |
| Fossil | Hash-named artifacts, single SQLite file, wiki + tickets + sync built in (V) | stateless HTTP set exchange (V) | none (C) | Works; SHA1-based login is dated (V); big binaries bloat the repo [BK] | **Viable alternative "all-in-one"**; borrow the igot/gimme/phantom protocol shape |
| Pijul, Jujutsu | Patch theory / operation log over git [BK] | — | Rust, but they are VCS front-ends | No advantage for an agent-written store | **Avoid** (borrow jj's *operation log* idea: every mutation of the repo itself is logged and undoable) |
| Merkle Mountain Ranges | Append-only accumulator with compact inclusion proofs [BK] | — | several small crates [BK] | Equivalent benefit to RFC 6962 trees here | **Avoid**; use RFC 6962 so tlog tooling works |
| CT-style logs (tlog-tiles, checkpoints) | Append-only, inclusion + consistency proofs, signed tree heads; static files (V) | copy immutable tiles + checkpoint | `tlog_tiles` 0.2 (V), small enough to vendor | Excellent: sync is "copy files, compare signed root" | **Adopt the formats** (checkpoint, tiles); write/own the small amount of Rust |
| Sigstore Rekor / Trillian | Public, service-shaped logs [BK] | online | — | Cloud dependency | **Avoid** (optional public anchoring only) |
| Sigsum | Minimal log + witness cosigning (V) | online log, offline-verifiable proofs [BK] | Go | Could anchor head hashes publicly | **Borrow** the witness idea: bench, pack and Mac cosign each other |
| SQLite + hash chaining | Hash chain in a table; simplest ledger | whatever moves rows | `rusqlite` 0.43 (V) | Excellent | **Adopt** as the index/materialised view; chain lives in the log files, mirrored in SQLite |
| immudb, QLDB-style ledgers | Verifiable DB server (immudb, Go [BK]); QLDB shut down 2025-07-31 (V) | client/server | none | Server process; QLDB shows the risk of renting your audit trail | **Avoid** |
| Hypercore / Dat, Secure Scuttlebutt | Per-writer append-only signed logs, sparse replication (Hypercore), gossip (SSB) [BK]; JS-centric | log replication | weak Rust story [BK] | The *model* (one signed log per device) is exactly right | **Borrow the model**, not the code |
| Automerge / Yjs / Loro | CRDT documents; automatic merge of concurrent edits [BK] | any | Rust-native (Automerge, Loro) [BK] | Solves a problem this project does not have: facts must not be auto-merged | **Avoid for facts**. Only justified for free-text notes edited on two devices at once |
| Willow / Earthstar | Destructive edits, capabilities; sync spec demoted to Proposal (V) | not finished | Meadowcap 0.1 only (V) | "Truly destructive editing" is the opposite of an audit trail | **Avoid** (research project) |
| Syncthing | Block hashes + TLS device IDs; no root hash, no history (V) | continuous folder sync | Go daemon | Fine as a *dumb transport* for an already self-verifying store | **Acceptable transport for v0**, never the integrity layer |
| rsync / zsync / rclone | Transfer checksums only [BK] | push/pull | n/a | Same as Syncthing: transport only | **Transport only** |
| Litestream / LiteFS | SQLite WAL shipping to object storage / FUSE replication [BK] | primary→replica | Go | Single-primary; not offline-two-writer | **Avoid** |
| cr-sqlite, sqlite-sync | Column-level LWW CRDT (V) | changeset exchange | C/Rust ext | LWW is explicitly unacceptable for facts | **Avoid** |
| SQLite session extension | Changesets with conflict callbacks (V: available in rusqlite) | apply changeset | available | Could move index rows, but the log already does that | **Avoid** (redundant) |
| ElectricSQL / PowerSync | Postgres-centred, cloud-leaning [BK] | server-mediated | — | Needs a server | **Avoid** |
| DuckDB + Parquet (+ Iceberg/Delta) | Columnar analytics over immutable files [BK] | file copy | `duckdb` crate, heavy build [BK] | Useful on the bench/Mac for analysing captures; too heavy to be the Pi's store | **Adopt Parquet for tabular captures; DuckDB as an optional analysis tool on the bench only** |
| lakeFS, Oxen.ai, Xet-style chunking | Git-for-data servers; CDC dedup for ML datasets [BK] | server | — | Server-shaped | **Avoid**; borrow CDC |

- A short way to say what the three adopted layers each *prove*: the blob hash proves "these bytes are the bytes I cited"; the Merkle log proves "this is the complete, ordered set of statements and nothing was removed or reordered"; the signature on the checkpoint proves "device X committed to exactly this state".
- MCAP's CRC32-only integrity (V) means the store must add its own cryptographic hash over each recording; MCAP will not do it.
- iroh-docs' identity triple (namespace, author, key) is a useful confirmation that "one writer identity per device, never overwrite another author's entries" is the mainstream shape for small-group sync — but iroh-docs keeps only the latest entry per key, so it cannot be the audit trail.

### Gaps
- Not verified this session (all marked [BK] above): current versions/status of git-annex, DVC, borg, kopia, casync, Perkeep, Irmin, TerminusDB, XTDB, immudb, Hypercore, SSB, Automerge 3, Loro, Litestream, LiteFS, ElectricSQL, PowerSync, lakeFS, Oxen, Jujutsu, Pijul, OSTree, Sigstore Rekor v2, and the SHA-256 object-format interop state of git/gix.
- cr-sqlite's maintenance state could not be confirmed (the repo page showed no release list or last-commit date); sqlite-sync (sqliteai) licence not confirmed.
- Whether iroh 1.x + the mDNS crates connect two endpoints with *no* internet and *no* relay configured is not shown in the docs; needs a bench test before depending on it.
- DoltLite's hash function, binary size, memory use on a 4 GB Pi and power-loss behaviour (no WAL sidecar) were not found.

---

## 2. "Lossless transfer with proof", opportunistic sync and conflict handling

### Takeaway
"Lossless with proof" decomposes into four checkable claims — each blob is bit-identical (hash), each blob is checked while streaming so a bad chunk is rejected immediately (BLAKE3/bao), the *set* of statements is complete and ordered (Merkle root + consistency proof), and the peer really committed to that state (signed checkpoint) — and all four are available from small, existing Rust crates. Conflicts are avoided structurally by giving each device its own single-writer log and never auto-merging facts.

### Cited Findings
- Verified streaming: the provider sends BLAKE3 verified-stream encoded data on the same QUIC stream as the request, so corruption is detected during transfer rather than after — [docs.rs iroh-blobs](https://docs.rs/crate/iroh-blobs/latest); `bao-tree` supports several non-overlapping byte ranges in one query (e.g. bytes 0–1000 and 5000–6000), which is the primitive for resume and for partial fetch — [docs.rs bao-tree](https://docs.rs/crate/bao-tree/latest)
- Completeness and non-equivocation: a checkpoint commits to tree size and RFC 6962 root; a signer must not sign two checkpoints between which no consistency proof exists — [c2sp.org/tlog-checkpoint](https://c2sp.org/tlog-checkpoint)
- tlog-tiles logs are designed so reads are highly cacheable and can be served straight from storage, with a POSIX filesystem backend — [Tessera 1.0](https://blog.transparency.dev/tessera-v1-0-is-here)
- Fossil's protocol shows the minimal "sync until complete" loop for hash-named artifacts (igot / gimme / file, phantoms for known-missing) over stateless request/response — [Fossil sync](https://fossil-scm.org/home/doc/trunk/www/sync.wiki)
- Range-based set reconciliation is the mechanism iroh-docs uses to find differences between replicas — [docs.rs iroh-docs](https://docs.rs/crate/iroh-docs/latest)
- Mutual authentication by key: iroh 1.0 addresses endpoints by public key ("dial keys, not IPs") — [iroh 1.0](https://iroh.computer/blog/v1); Syncthing does the equivalent with TLS-certificate-derived device IDs — [BEP v1](https://docs.syncthing.net/specs/bep-v1.html)
- LAN discovery crates for iroh 1.x and their caveats (leaks local addresses to connected peers; two crates use incompatible discovery formats) — [iroh local address lookup](https://docs.iroh.computer/connecting/local-address-lookup.md)
- The MCP 2026-07-28 revision removed stream resumability: a broken response stream loses the in-flight request and the client must re-issue it as a new request — [MCP 2026-07-28 changelog](https://modelcontextprotocol.io/specification/2026-07-28/changelog) (relevant to read-your-writes and retries, see section 8)

### Inferences
- **Proof ladder (what to actually implement).**
  1. *Blob identity*: `blake3(raw bytes)`; the receiver recomputes. Proves bit-exactness.
  2. *In-flight rejection + resume*: serve blobs through `bao-tree` with a stored outboard; the receiver verifies each chunk group against the root hash it already knows from the log and can resume by requesting the missing ranges. A flaky Wi-Fi link at the edge of range therefore never yields a silently truncated 2 GB recording.
  3. *Set completeness*: each device's log has a Merkle root. After pulling entries N..M the receiver recomputes the root and compares it with the signed checkpoint. Equality proves it holds every entry, in order. A consistency proof from its previous checkpoint proves the peer did not rewrite history (e.g. after an SD-card restore).
  4. *Authenticity*: the checkpoint is Ed25519-signed by the device key. Pinning the three device public keys in a signed "membership" entry makes this TOFU-free.
  5. *Closure*: a sync is "complete" only when every blob hash referenced by any log entry up to the checkpoint is present locally, or is explicitly recorded as "not carried on this device" (git-annex-style location tracking). The pack should be allowed to hold *all facts* but only *some blobs*.
- **Because tlog-tiles is a static-file layout, sync can degrade gracefully**: QUIC when in range, `rsync`/Syncthing/USB stick otherwise. The verification step is identical regardless of how the files arrived. This is the strongest argument for choosing a static, self-verifying on-disk format over a sync *protocol*.
- **Transport choice.** On a LAN neither hole-punching nor relays are needed. Two reasonable options: (a) `iroh` 1.x endpoint + one mDNS crate + a tiny custom ALPN — gains key-addressed dialing and a stable wire for ~one dependency; (b) `quinn` + `rustls` with mutually pinned raw public keys/self-signed certs + `mdns-sd` for discovery [BK for crate names] — fewer moving parts, no relay code in the binary. Either is "boring"; neither is a research project. Noise (e.g. `snow`) is a third option but offers nothing QUIC+TLS does not here.
- **Conflict model.** Single-writer-per-log makes log-level conflicts impossible. Fact-level disagreement is then a *query result*, not a merge failure: if two unsuperseded assertions exist for the same (subject, predicate, context) with different values, the materialised view marks the slot `contested` and returns both with their evidence. Resolution is a new signed assertion that supersedes both (by the agent with a measurement, or by the owner). Nothing is ever lost and last-writer-wins never happens implicitly.
- **Clock independence.** Order within a device log is by sequence number; cross-device ordering should use "which checkpoints of the other device had I seen when I wrote this" (a per-entry vector of peer tree sizes) rather than wall-clock time, since the pack's clock may be wrong when offline [BK: standard version-vector reasoning].
- **Read-your-writes across sync**: a write result returns `(device, seq)`; a later read on another node can state `min_seen: {pack: 4812}` and get a truthful "not yet synced past pack@4790" instead of stale data.

### Gaps
- No measured throughput for BLAKE3 verified streaming on a Pi 5 over Wi-Fi was found.
- Crate names/versions for `quinn`, `rustls`, `snow`, `mdns-sd` were not re-verified this session.
- Whether `tlog_tiles` 0.2.0 exposes proof generation/verification sufficient for this use (vs. needing to port more of Go's `tlog`) was not checked at source level.

---

## 3. Object model for equipment knowledge: prior art and a minimal typed-fact model

### Takeaway
No existing schema covers "everything an agent learned about a device, with evidence"; the usable prior art is narrow and per-protocol (OBDb signal sets, SunSpec models, an early "OpenAPI for Modbus", Kaitai Struct, W3C WoT Thing Descriptions) plus one generic envelope that fits claims-about-hashed-artifacts very well (in-toto Statement: subject digest + predicate type + predicate). The recommended model is a small typed fact with unit, context, evidence-by-hash, method and supersession — with protocol descriptions stored as first-class executable artifacts in their native formats rather than re-modelled.

### Cited Findings
- **OBDb**: community database of OBD parameters and DTCs, one repo per make-model (740 repositories), each with `signalsets/v3/default.json` plus year-range override files; mostly CC BY-SA 4.0; repos updated as recently as 2026-09-07; a pinned `SAEJ1979` repo holds the standard PIDs. A Toyota Tacoma repo was not confirmed from the page viewed — [github.com/OBDb](https://github.com/OBDb)
- **OpenModbusSpecs** (Stekker, MIT): YAML device profiles validated by a JSON Schema (v0.3.0 pre-release), registers keyed by address with name, type, length, unit, register type, byte order and scaling; self-described as "OpenAPI for Modbus"; very small (27 commits, 1 star) and seeking maintainers — [stekker/OpenModbusSpecs](https://github.com/stekker/OpenModbusSpecs)
- **Kaitai Struct** 0.11 (2025-09-07): declarative `.ksy` binary-format descriptions compiled to parsers for C++, C#, Go, Java, JavaScript, Lua, Nim, Perl, PHP, Python, Ruby and Rust; compiler is GPLv3+, runtimes mostly MIT — [kaitai.io](https://kaitai.io/)
- **LLMs writing format descriptions** (LangSec workshop 2025, Narf Industries): across 20 formats and six description languages, models most reliably produced compiling *Rust nom* parsers; Kaitai Struct and Hammer also often compiled; no model produced a compiling DFDL or DaeDaLus spec; Claude 3.5 Sonnet had the best compile rates; the generated Modbus and JPEG parsers were *too permissive* (accepted malformed input) — [Anantharaman & Varadharaju](https://langsechq.gitlab.io/spw25/papers/anantharaman-llm-parser-generation.pdf)
- **W3C WoT Thing Description 1.1** (Recommendation, 2023-12-05): a Thing exposes Properties, Actions and Events; *forms* bind each to a concrete protocol; JSON-LD 1.1 based, so a TD is also RDF; Thing Models act as templates — [W3C TD 1.1](https://www.w3.org/TR/wot-thing-description11/)
- **in-toto attestation framework** v1.2: four layers — Predicate (typed metadata), Statement (binds predicate to subjects), Envelope (DSSE: authentication + serialisation), Bundle (several attestations); subjects are identified by a `DigestSet`, generalised in 1.1.0 to any immutable identifier — [in-toto/attestation](https://github.com/in-toto/attestation/blob/main/spec/README.md)
- **Bitemporal facts in agent memory**: Zep/Graphiti stores four timestamps per fact edge — created/expired on the ingestion timeline and valid/invalid on the event timeline — and invalidates (rather than deletes) contradicted edges — [Zep paper, arXiv 2501.13956](https://arxiv.org/abs/2501.13956)
- **Evidence-first memory**: Eywa stores immutable source evidence before deriving facts and validates extracted memories against source support — [arXiv 2605.30771](https://arxiv.org/abs/2605.30771); SOMEM keeps never-altered Source records with editable topic documents whose every statement links back to sources, and its ablation shows removing Source causes the largest accuracy drop — [arXiv 2611.11929](https://arxiv.org/abs/2611.11929)
- **Markdown observation syntax**: Basic Memory writes facts as `- [category] fact #tag` and relations as wikilinks inside Markdown files with frontmatter — [Basic Memory README](https://raw.githubusercontent.com/basicmachines-co/basic-memory/main/README.md)

### Inferences
- **Do not invent protocol formats.** Store protocol knowledge in the format the ecosystem already executes, as hashed artifacts, and let facts point at them: `.ksy` (framing/binary layouts), DBC (CAN) [BK], OBDb signalset JSON (OBD-II), SunSpec model JSON where a device is SunSpec-compliant [BK], an OpenModbusSpecs-style YAML for proprietary Modbus maps (its schema is tiny and MIT; vendor the schema rather than depend on the project), sigrok protocol-decoder / Wireshark dissector references where they exist [BK]. Given the LangSec result, the agent should prefer generating `.ksy` or Rust `nom` and must *test them against captures including malformed ones*, since permissiveness was the observed failure.
- **Borrow selectively from asset schemas [BK, unverified this session]**: NetBox's Device → Interface → Cable → termination model is the right shape for wiring/pinouts; InvenTree's Part vs StockItem split maps to "model facts" vs "this unit's facts" (serial, calibration); IEEE 1451 TEDS is the precedent for a sensor carrying its own calibration record; AAS / IEC 61406 digital nameplate is the precedent for an identity link + submodels but is far heavier than needed; Device Tree is the precedent for compatible-string matching ("this board is an X, so facts about X apply"); PROV-O's Entity / Activity / Agent with `wasDerivedFrom` / `wasGeneratedBy` is the vocabulary to mirror in field names so an RDF export stays possible; nanopublications are the precedent for "one assertion + its provenance + its publication info, individually signed and addressable"; RO-Crate is a ready-made way to export a bundle (manual + facts + captures) as a self-describing folder.
- **Minimal typed fact (recommended fields).**

```
fact_id        ulid (stable handle for the *slot history*; never reused)
assertion_hash blake3 of the canonical bytes of this assertion (the version / ETag)
subject        entity id (e.g. dev:eg4-6000xp#unit-1, or model:eg4-6000xp, or iface:…/rs485-a)
predicate      namespaced key from a small registry (modbus.baud, conn.pinout, fw.version, cal.offset …)
value          typed: number | string | bool | enum | ref(entity) | ref(blob) | struct
unit           UCUM / SI string when numeric; required for numbers
context        optional scope under which this holds: {fw: "…", hw_rev: "…", config_hash: "…"}
evidence[]     {blob: b3:…, locator: page+bbox | byte range | mcap topic+time range | register addr, role: supports|contradicts}
method         measured | read-from-device | vendor-doc | inferred | owner-stated | imported
asserted_by    device key id + agent/model id or "owner"; session id
asserted_at    wall clock (advisory) + (device, seq) (authoritative)
valid_from / valid_to   event-time interval (when it was true of the world)
supersedes[]   assertion hashes this replaces (never edit in place)
status         derived, not stored: current | superseded | retracted | contested | stale
verified       {by: measurement blob, at, procedure_ref} or null
confidence     0..1 as asserted + decay class (see section 9 ideas)
depends_on[]   assertion hashes this was derived from (drives invalidation)
verify_procedure  optional ref to an executable check (tool call template + expected result)
visibility     public | private  (see licensing note in section 4)
```

- The in-toto Statement shape (`subject[].digest`, `predicateType`, `predicate`) wrapped in DSSE is a near drop-in envelope for the *evidence-bearing* subset of facts ("about blob X, claim Y, signed by Z"), and reusing it buys existing verification tooling. For facts whose subject is a device rather than a blob, in-toto 1.1's "any immutable identifier" subject makes it legal but slightly awkward; a native canonical-CBOR/JCS record with the same field names is simpler. This is a judgement call, flagged in "hard-to-change decisions".
- Model-level vs unit-level facts must be separated from day one (the modified 2003 Tacoma is the clearest case: OBDb-style generic signals for the platform vs unit-specific deviations).

### Gaps
- NetBox, InvenTree, Snipe-IT, Home Assistant device registry, Device Tree, IEEE 1451 TEDS, AAS/IEC 61406, OPC UA companion specs, SunSpec JSON models, ESPHome/Zigbee2MQTT definitions, sigrok, Wireshark, DBC, ImHex patterns, PROV-O, nanopublications, RO-Crate, schema.org and Datasette were *not* fetched this session; statements about them are background knowledge.
- No machine-readable Modbus map for EG4 inverters was located in this pass (search returned only SunSpec material and forum threads).
- Whether OBDb has a Toyota-Tacoma repo covering the 2003 model year is unconfirmed; pre-CAN 2003 Toyotas use ISO 9141-2/K-line [BK], which may limit OBDb coverage.
- WoT Modbus binding template status and any TD 2.0 draft were not confirmed from the page fetched.

---

## 4. Knowledge formats, retrieval on a Pi, agent-memory evidence, and PDF/manual ingestion

### Takeaway
At this scale (thousands of facts, hundreds of manuals) the published evidence favours immutable sources + plain lexical/hybrid search + an agent that searches iteratively over heavy "memory" frameworks or GraphRAG; on the Pi that means SQLite FTS5 or tantivy plus a small or static embedding model, with expensive steps (PDF layout parsing, contextual chunk descriptions, reranking) done once on the bench/Mac and shipped to the pack as hashed derived artifacts.

### Cited Findings

**What helps agents (evidence)**
- Letta (2025-08-12): an agent with only file tools (grep, search, open/close) scored 74.0% on LoCoMo with GPT-4o mini, above Mem0's best reported graph variant (68.5%); Letta's reading is that tool-use skill matters more than the retrieval mechanism and that such benchmarks may not measure much — [Letta blog](https://letta.com/blog/benchmarking-ai-agent-memory)
- Anthropic contextual retrieval (2024-09-19): prepending chunk-specific context cut top-20 retrieval failure by 35% (5.7% → 3.7%); adding contextual BM25, 49% (→ 2.9%); adding a reranker, 67% (→ 1.9%); for knowledge bases under ~200k tokens the advice is to skip retrieval and put everything in the prompt with caching — [Anthropic engineering](https://www.anthropic.com/engineering/contextual-retrieval)
- GraphRAG at small/medium scale: Samsung AI Warsaw (v3 2026-06-08) found plain VectorRAG beat standard GraphRAG on all four datasets and was close to the best graph systems at a fraction of the cost; on the *eManual* dataset factual-correctness F1 was 0.479 (VectorRAG) vs 0.342 (GraphRAG) vs 0.468 (HippoRAG2); GraphRAG indexing COVID-QA took ~25.9M prompt tokens vs ~1.47M for their entity-in-vector approach — [arXiv 2603.29875](https://arxiv.org/abs/2603.29875). A separate benchmark paper opens from the observation that GraphRAG frequently underperforms vanilla RAG on real tasks — [arXiv 2506.05690](https://arxiv.org/abs/2506.05690)
- TierMem (2026-02-20): query a fast summary tier first and escalate to an immutable raw log only when needed; 0.851 vs 0.873 (raw-only) LoCoMo accuracy with 54.1% fewer input tokens and 60.7% lower latency — [arXiv 2602.17913](https://arxiv.org/abs/2602.17913)
- SOMEM (arXiv 2611.11929): agent-edited Markdown topic documents over immutable sources, with Graph/Timeline/Recent views *derived automatically* by the runtime and every edit validated; reported +2 to +3 points over the strongest baselines on LoCoMo, BEAM-100K and LongMemEval-S — [arXiv 2611.11929](https://arxiv.org/abs/2611.11929)
- Zep reports LongMemEval gains over full-context (63.8% vs 55.4% with gpt-4o-mini) and ~90% lower latency, using LLM-driven edge invalidation — [arXiv 2501.13956](https://arxiv.org/abs/2501.13956)
- Vendor-reported leaderboards have converged near the ceiling (e.g. Agent Zero Memory 95.6% LongMemEval / 93.6% LoCoMo, using each competitor's self-reported numbers and cautioning these are not controlled comparisons) — [arXiv 2608.29606](https://arxiv.org/pdf/2608.29606)

**Search engines and vector stores**
- tantivy 0.27.0 (2026-10-17, MIT): BM25 scoring as in Lucene, mmap directory, incremental indexing — [docs.rs tantivy](https://docs.rs/crate/tantivy/latest)
- sqlite-vec: latest stable v0.1.9 (2026-03-31); ANN indexes (rescore, DiskANN, experimental IVF) exist only in 0.1.10 alphas (latest alpha.4, May 2026); still pre-1.0 — [sqlite-vec releases](https://github.com/asg017/sqlite-vec/releases)
- SQLite now has its own in-tree ANN extension, **vec1** (v0.8, page updated 2026-10-01): IVFADC + OPQ, L2 and cosine, portable C with NEON on ARM, float32 only, single-threaded queries, no HNSW/DiskANN yet — [sqlite.org vec1](https://sqlite.org/vec1/doc/trunk/doc/vec1.md)
- fastembed (Rust) 7.8.0 (2026-11-22, Apache-2.0): ONNX Runtime via `ort` (pinned to a 2.0.0 release candidate), default model bge-small-en-v1.5; also MiniLM, nomic-embed v1/v1.5, EmbeddingGemma-300m incl. a 4-bit variant, SPLADE++ sparse, and BGE/Jina rerankers; the page makes no explicit aarch64 claim — [docs.rs fastembed](https://docs.rs/crate/fastembed/latest)

**Embedding models and Pi throughput**
- Only one sourced Pi 5 number was found: bekko-a8m (7.67M active parameters, 124 MiB ONNX) embeds ~33 documents/s on a Raspberry Pi 5, so 10,000 documents take about five minutes, whereas multilingual-e5-large needs about two hours for the same job on that board (2026-07-29) — [Hugging Face blog, Bekko](https://huggingface.co/blog/hotchpotch/bekko-embedding)
- Static embeddings (model2vec "potion"): potion-base-32M scores 52.13 average MTEB vs 55.93 for all-MiniLM-L6-v2 (93.2% of it), but retrieval specifically is weaker (32.67; potion-retrieval-32M 35.06; MiniLM 42.92) — [model2vec results](https://minish.ai/packages/model2vec/results/)
- EmbeddingGemma 1: 308M parameters, <200 MB RAM quantised, 2K context, Matryoshka 768→128 dims. The Google page now lists an "EmbeddingGemma 2" as current (Apache-2.0, 8,192-token context incl. source code, 768-dim with truncation to 512/256/128) — [Google AI, EmbeddingGemma](https://ai.google.dev/gemma/docs/embeddinggemma) (version-2 parameter counts as summarised by the fetch tool looked inconsistent; re-check before quoting)

**Graph stores**
- Kuzu's repository was archived 2025-10-10 at v0.11.3; LadybugDB ("formerly known as Kuzu") is the active successor at v0.19.1 as of 2026-08-12 — [OneUptime, 2026-08-12](https://oneuptime.com/blog/post/2026-08-12-kuzu-archived-pin-0-11-3-fork-or-migrate/view)
- cozo's last crates.io release is 0.7.6 from 2023-12-11 — effectively dormant — [docs.rs cozo](https://docs.rs/crate/cozo/latest)
- oxigraph 0.5.13 (2026-10-22, MIT/Apache-2.0): SPARQL 1.1 query/update, RocksDB default backend, RDF 1.2 behind a feature flag — [docs.rs oxigraph](https://docs.rs/crate/oxigraph/latest)

**Agent "memory" products**
- Basic Memory (Python, AGPL-3.0, v0.23): Markdown files are the source of truth with a local SQLite index; ~17 MCP tools; local multi-device sync is left to Git or Syncthing, built-in sync is a paid cloud feature — [Basic Memory README](https://raw.githubusercontent.com/basicmachines-co/basic-memory/main/README.md)

**PDF ingestion**
- Docling: MIT code (models carry their own licences), unified `DoclingDocument` with lossless JSON export, table structure and OCR support, Granite Docling VLM pipeline, runs locally/air-gapped, has an MCP server, supports Linux ARM64 — [docling-project/docling](https://github.com/docling-project/docling)

### Inferences
- **Scale check.** A bench owner's corpus is perhaps 10^2–10^3 documents and 10^3–10^5 facts/chunks. At 10^5 vectors of 256–384 dims, exact brute-force search is a few tens of MB and millisecond-to-tens-of-ms on a Pi 5 [BK estimate]; an ANN index is unnecessary. That makes sqlite-vec *stable* (brute force) sufficient and keeps the alpha ANN code out of the trust base. vec1 is worth watching because it will ship with SQLite itself, but 0.8 is too young to depend on.
- **Recommended retrieval stack on the Pi**: SQLite FTS5 (bundled with rusqlite [BK: FTS5 is in the bundled build]) for facts and page text — one file, same transaction as the fact index, survives power loss with WAL. tantivy is the upgrade path if ranking features or index size demand it; it adds a second on-disk index to keep consistent. Hybrid = FTS5 BM25 + small-embedding cosine, fused with reciprocal rank fusion [BK], no reranker on the Pi.
- **Embeddings**: compute on the bench/Mac (bge-small-class or EmbeddingGemma via fastembed), store vectors as a *derived artifact keyed by (chunk hash, model id)*, and sync them to the pack so the pack only has to embed the *query*. For query-time embedding on the Pi, a ~8–25M-parameter model at tens of docs/s is plenty; a static model (potion) is the fallback when battery matters, accepting the documented retrieval-quality drop. Equipment queries are dominated by exact tokens (register addresses, part numbers, connector names), which is BM25's home ground — lexical search should carry most of the load.
- **Skip**: GraphRAG/LightRAG/HippoRAG-style LLM-built graphs (evidence above: more cost, no gain on manual-like corpora); a graph database (Kuzu archived, cozo dormant; SQLite tables `entity`, `edge` with recursive CTEs cover device→interface→cable traversals at this size); Letta/Zep/mem0/Cognee as dependencies (Python/servers, and conversation-memory shaped). **Borrow**: Zep's bitemporal edges, TierMem's summary-then-raw escalation, SOMEM's derived views, Anthropic's contextual chunk prefixes (generate once on the Mac, hash, sync).
- **"Under 200k tokens, don't retrieve"** applies per device: a device's whole fact sheet + register map is usually far below that, so `get(device, depth=summary|full)` returning a compact, hash-stamped "device card" is often better than search.
- **PDF pipeline (bench/Mac only, never on the pack)**: (1) store the original PDF by BLAKE3 + SHA-256; (2) cheap text layer via pdfium/poppler [BK]; (3) Docling for layout + tables (register maps) → `DoclingDocument` JSON stored as a derived blob with `derived_from` = PDF hash and tool+model version; (4) page renders (PNG/WebP) as blobs so pinout diagrams can be shown to a vision model and cited; (5) every extracted fact cites `{blob, page, bbox}`. A register table extracted by a parser is `method: vendor-doc` and `verified: null` until the agent reads those registers on the real device.
- **Licence note (not legal advice, [BK])**: vendor manuals are copyrighted and many forbid redistribution; a public GitHub repo should carry *facts and protocol descriptions* (plus the manual's hash, title, revision and source URL), not the PDFs. This needs a hard `visibility` split in the store so a public export can never include private blobs, and extracted text/page renders must inherit `private` from their source.

### Gaps
- No sourced Pi 5 throughput for bge-small, gte-small, nomic-embed, EmbeddingGemma or potion models was found; the only Pi 5 figure is for bekko-a8m. model2vec's speed figures are in a chart not readable as text.
- Marker, MinerU, unstructured, Surya, PaddleOCR, tesseract: not fetched (the one comparison article failed to load). Licences from background knowledge only — Marker/Surya GPL-family with commercial terms, MinerU AGPL [BK, verify] — which matters for a permissively licensed Rust project that wants to bundle them; Docling's MIT licence is confirmed.
- Docling's bounding-box provenance fields and CPU-only speed on ARM were not confirmed from the page fetched.
- LanceDB, usearch, hnsw_rs, Qdrant edge, DuckDB VSS, late chunking, rerankers on ARM, mem0/Cognee/Letta current state: not researched in depth this session.
- LongMemEval/LoCoMo scores cited from 2026 preprints are self-reported and LLM-judged; one preprint names a reader model the researcher could not corroborate.

---

## 5. Stale-belief detection and verifiable citations

### Takeaway
There is active 2026 work on "provenance-aware" agent memory, but none of the papers found uses cryptographic hashes or content addressing — hash-pinned beliefs with a server-side "are my premises still current?" check appears to be open ground, and it can be built from old, boring patterns: ETag/If-Match, change feeds with cursors, bitemporal "as-of" reads, and build-system-style dependency invalidation. MCP 2026-07-28 supplies the carriers (resource links, `subscriptions/listen`, `ttlMs`, `structuredContent`) but no version/ETag field of its own.

### Cited Findings
- Agent Zero Memory: answers are "citation-locked" — only items actually opened during the query can be cited, otherwise the system abstains; updates are non-destructive timestamped beliefs; forgetting/decay is future work; hashing is not mentioned — [arXiv 2608.29606](https://arxiv.org/pdf/2608.29606)
- Eywa: immutable evidence first, derived facts validated against source support, deterministic retrieval with no LLM calls; "stale state" is listed as one cause of wrong answers; no hashing in the abstract — [arXiv 2605.30771](https://arxiv.org/abs/2605.30771)
- TierMem: verified findings are written back as summary units *linked to raw sources*; no hashes, no staleness mechanism described — [arXiv 2602.17913](https://arxiv.org/abs/2602.17913)
- ElephantBroker: names missing provenance/trust tracking as the gap in flat memory and claims "verifiable" memory, without a tamper-evidence mechanism in the abstract — [arXiv 2603.25097](https://arxiv.org/abs/2603.25097)
- Retrieval *admissibility* (Univ. of Arkansas at Little Rock): relevance ≠ permission to use; a superseded value retrieved for a current-state question is "relevant but inadmissible"; text-only verifiers could not detect such violations at a 1% false-denial limit, whereas metadata-based checks preserved required evidence; memory IDs should be tracked from retrieval through prompt exposure to the final answer — [arXiv 2610.07309](https://arxiv.org/abs/2610.07309)
- MCP 2026-07-28 carriers:
  - Tool results may include `resource_link` items (`uri`, `name`, `description`, `mimeType`) that the client can fetch or subscribe to, and they need not appear in `resources/list` — [MCP tools spec](https://modelcontextprotocol.io/specification/2026-07-28/server/tools)
  - `structuredContent` may be any JSON value and must conform to `outputSchema` when one is declared — [MCP tools spec](https://modelcontextprotocol.io/specification/2026-07-28/server/tools)
  - Per-resource change notifications now arrive via one long-lived `subscriptions/listen` stream (opt-in `resourceSubscriptions`), replacing `resources/subscribe`; the notification carries only the `uri` — [MCP changelog](https://modelcontextprotocol.io/specification/2026-07-28/changelog), [MCP resources spec](https://modelcontextprotocol.io/specification/2026-07-28/server/resources)
  - `resources/read` and list results must carry `ttlMs` and `cacheScope` (freshness hints); annotations offer `lastModified`, `priority`, `audience`; there is no ETag/version field in the resource schema — [MCP changelog](https://modelcontextprotocol.io/specification/2026-07-28/changelog), [MCP resources spec](https://modelcontextprotocol.io/specification/2026-07-28/server/resources)
  - The protocol is now stateless (no `initialize`, no session id); cross-call state must be an explicit server-minted handle passed as a tool argument — [MCP changelog](https://modelcontextprotocol.io/specification/2026-07-28/changelog)

### Inferences
- **The admissibility paper is the best external justification for this design**: staleness cannot be judged reliably from text; it needs metadata. Here the metadata is cryptographic — every item the agent sees carries `id@hash`, and "is it still current?" is an exact comparison, not a judgement.
- **Pattern set to implement (all conventional):**
  1. *Version token on everything*: every returned fact/blob/card carries `ref = <id>@<short-hash>`; the full hash is retrievable. This is an ETag.
  2. *If-Match on writes*: `supersede`/`retract` require `expected: <assertion_hash>`; mismatch returns a truthful conflict with the current head.
  3. *Premise check*: `verify(refs[])` → for each: `current | superseded_by:<ref> | retracted | contested | unknown_here`. Cheap (a hash lookup), so the agent can call it before any actuation and at session start.
  4. *Change feed with cursor*: `changes(since)` where the cursor is literally the set of per-device checkpoint sizes `{bench: 10412, pack: 4812}` — an explicit handle, which is what stateless MCP wants. Returns compact rows (ref, subject, predicate, kind).
  5. *As-of reads*: `get(..., as_of: cursor|time)` replays the log prefix; trivial with an append-only log and a bitemporal index.
  6. *Dependency invalidation*: `depends_on` edges form a DAG; when an input is superseded, mark dependants `stale` (dirty-bit propagation as in build systems / Salsa's red-green algorithm [BK]) rather than deleting them; a "stale" fact is still returned but flagged, and re-derivation or re-measurement clears it. A full truth-maintenance system (justification-based TMS [BK]) is more than is needed; dirty-bit propagation plus `contested` slots covers it.
  7. *Subscriptions*: expose `ks://fact/<id>` and `ks://device/<id>/card` as resources so a long-running client can `subscriptions/listen` and get `notifications/resources/updated` when the head changes; since the notification has only a URI, the client then calls `verify` or `get`.
  8. *TTL as honesty*: immutable `ks://blob/b3/<hash>` and `ks://assertion/<hash>` resources can advertise a very long `ttlMs`; mutable heads advertise a short one. This uses the spec's cache hints truthfully.
- Because MCP gives no native version field, the hash must live in the URI (immutable resources) and in `structuredContent` (mutable heads). Putting the hash in the URI also makes `resource_link`s self-verifying.

### Gaps
- No 2025–2026 paper was found that pins agent context to content hashes or uses Merkle/transparency-log structures for agent memory; the search was not exhaustive (two queries), so absence is weak evidence.
- Salsa and truth-maintenance-system references were not fetched this session.
- MCP `subscriptions/listen` details (acknowledgement, cancellation) and the caching utility page were not read in full.

---

## 6. Compression, canonical encoding, hashing choice, layout and sizing

### Takeaway
Hash the canonical *uncompressed* bytes and treat compression as a storage detail so hashes survive re-compression; use BLAKE3 for blobs (verified streaming) and keep SHA-256 where interop demands it (RFC 6962 log tree, git, OpenTimestamps, in-toto); do not content-chunk already-compressed recordings; and plan disk by recordings, not by facts or manuals.

### Cited Findings
- zstd seekable format in Rust: `zeekstd` 0.6.2 (2025-12-20, BSD-2-Clause) — independent frames plus a seek table in a skippable frame, so a mid-file read decodes about one frame; ordinary zstd decoders still read the file — [docs.rs zeekstd](https://docs.rs/crate/zeekstd/latest)
- Content-defined chunking: `fastcdc` 5.0.0 (2026-08-22) with v2016 and v2020 algorithms, streaming `StreamCDC` and async `AsyncStreamCDC` — [docs.rs fastcdc](https://docs.rs/crate/fastcdc/latest)
- MCAP chunks are individually compressed (zstd or lz4) and indexed by time/channel; integrity is CRC32 only — [MCAP spec](https://mcap.dev/spec)
- BLAKE3 Rust crate has a NEON implementation; the project claims it is much faster than the SHA-2 family — [BLAKE3 repo](https://github.com/BLAKE3-team/BLAKE3)
- The CT checkpoint format fixes the tree hash to RFC 6962 (SHA-256) — [c2sp.org/tlog-checkpoint](https://c2sp.org/tlog-checkpoint)
- in-toto subjects use a `DigestSet` (several named digests for one artifact) — [in-toto attestation](https://github.com/in-toto/attestation/blob/main/spec/README.md)
- rustic_core reads/writes the restic repository format (encrypted, deduplicated backups) — [docs.rs rustic_core](https://docs.rs/crate/rustic_core/latest)
- DoltLite small autocommit writes are ~3.1x slower than SQLite (≈400 µs vs ≈125 µs) — a data point on the write cost of prolly-tree versioning — [DoltHub](https://dolthub.com/blog/2026-08-31-doltlite-beta)

### Inferences
- **Dual digest at ingest**: compute BLAKE3 and SHA-256 once, store both in the blob's metadata record (a `DigestSet`, as in-toto does). BLAKE3 is the address and the verified-streaming root; SHA-256 is for the outside world (git-lfs pointers, OpenTimestamps, `sha256sum` by a human, Sigstore-style tooling). On a Pi 5 the SHA-256 cost is small because the Cortex-A76 has ARMv8 crypto extensions [BK, unverified this session]; BLAKE3 uses NEON.
- **Canonical form of small records**: two defensible choices — RFC 8785 JCS (canonical JSON: human-readable, greppable, agents read it natively; float formatting rules are the main trap) or deterministic CBOR (RFC 8949 §4.2: compact, binary-safe, unambiguous integers/bytes). Recommendation: **JCS for facts** (they are small, the agent and owner will read them, and "files are the truth" debugging matters more than bytes), with numbers that need exactness carried as strings + unit. `postcard` is not self-describing and should not be the hashed form [BK].
- **Compression policy**:
  - Facts/log entries: pack into immutable segment files, zstd with a trained dictionary per segment generation (small-JSON records compress several times better with a dictionary [BK]); the dictionary is itself a hashed blob referenced by the segment header. Hash is over each record's canonical bytes, never over the compressed segment.
  - PDFs, JPEG/HEIC photos, FLAC/Opus audio, MCAP-with-zstd-chunks: already compressed → store raw, no CDC (content-defined chunking finds almost nothing to dedup in compressed streams [BK]).
  - Uncompressed scope captures / WAV / raw CSV: convert tabular captures to Parquet+zstd on the bench (keep the raw capture hash as `derived_from`), or zstd-seekable (`zeekstd`) when random access is needed.
  - FastCDC only where near-duplicate large files are expected (successive firmware images, repeated full-disk dumps). Otherwise whole-file addressing is simpler and makes verified streaming straightforward.
- **Record MCAP with zstd chunk compression at capture time on the pack** so the blob is final when written; never re-compress after hashing.
- **On-disk layout** (all immutable except `heads/` and the SQLite cache):

```
store/
  blobs/b3/ab/cd/<hash>            raw bytes (or .zst with sidecar noting original hash)
  blobs/b3/ab/cd/<hash>.obao       bao outboard for blobs above a size threshold
  logs/<device-key-id>/tile/...    tlog-tiles (entries bundled + Merkle tiles)
  logs/<device-key-id>/checkpoint  signed note (latest)
  logs/<device-key-id>/witness/... cosignatures from the other devices
  cache/index.sqlite               facts, entities, edges, FTS5, vectors — rebuildable
  cache/derived/...                page renders, DoclingDocument JSON, embeddings (also hashed)
```

- **Power-loss rules** [BK, standard practice]: write blob to temp → fsync file → rename → fsync directory; append log entries then publish the checkpoint last (the checkpoint is the commit point; entries beyond it are ignored/rolled forward on restart); SQLite in WAL mode with `synchronous=FULL` for the index, and the index is disposable anyway. Prefer NVMe on the Pi 5 for the store; consumer SD cards may lie about flush and wear faster.
- **Sizing (illustrative arithmetic, assumptions stated, not sourced)**: 300 manuals × 15 MB ≈ 4.5 GB; page renders + extraction ≈ same again; 50,000 facts × ~1 KB ≈ 50 MB before compression; photos 2,000 × 4 MB ≈ 8 GB. Recordings dominate: one hour of 48 kHz 6-channel 24-bit audio ≈ 3.1 GB raw; a camera topic can be tens of GB/hour. Conclusion: facts + manuals fit on anything; the pack needs a retention/pinning policy for recordings from day one, and the bench (plus a third device) is where full history lives.
- **Backup**: the store is already an append-only set of immutable files, so `restic`/`rustic` to an encrypted off-site or USB target is incremental by nature; backing up `blobs/` + `logs/` is sufficient (the cache is rebuilt). A restore is verified by recomputing roots against the last witnessed checkpoints.

### Gaps
- RFC 8785, RFC 8949, zstd dictionary documentation and the `zstd`/`ciborium`/`serde_jcs` crate statuses were not fetched this session.
- No sourced figures for SD vs NVMe endurance or flush honesty on Pi 5; no sourced BLAKE3-vs-SHA-256 numbers on Cortex-A76.
- Real recording volumes depend on the sensor pack's topics and were not available; the sizing above is arithmetic on assumed inputs.

---

## 7. Signing and trust

### Takeaway
Per-device Ed25519 keys signing CT-style checkpoints, with the other devices cosigning as witnesses, gives offline-verifiable authenticity and fork/rollback detection with no external service; hardware key storage on a Pi is available and cheap but typical TPMs do not do Ed25519, so it forces an algorithm decision early.

### Cited Findings
- Checkpoints support Ed25519 log signatures and timestamped cosignatures; ML-DSA-44 cosignatures are the spec's preferred (SHOULD) cosignature type; cosigners must refuse checkpoints inconsistent with what they signed before — [c2sp.org/tlog-checkpoint](https://c2sp.org/tlog-checkpoint)
- Sigsum's design goal: misuse of a signing key becomes detectable even if the key, the log and some witnesses are compromised — [sigsum.org](https://www.sigsum.org/)
- Radicle's signed refs show the same failure mode handled in production: downgrade/rollback detection by inspecting history, with a monotonic feature level — [Radicle 1.8.0](https://radicle.dev/2026/03/30/radicle-1.8.0)
- OpenTimestamps proves data existed before a point in time by anchoring to Bitcoin through free public calendar servers (no account); clients exist in Python, JavaScript, Java and a Rust library — [opentimestamps.org](https://opentimestamps.org/)
- A new open-hardware I2C TPM 2.0 board for Raspberry Pi (ANAVI, Infineon OPTIGA SLB 9673, ~$20, CC BY-SA 4.0 KiCad files, works on Pi 5 with the `tpm-slb9673` overlay) was announced 2026-11-19, shipping by March 2027; the earlier ANAVI board used SPI — [CNX Software](https://www.cnx-software.com/2026/11/19/raspberry-pi-sbcs-get-an-open-source-hardware-i2c-tpm-2-0-security-module/)
- in-toto's envelope layer (DSSE) is the standard wrapper for signed statements about digests — [in-toto attestation](https://github.com/in-toto/attestation/blob/main/spec/README.md)

### Inferences
- **Key hierarchy**: one offline *owner root* key (kept on the Mac/YubiKey or printed) signs a small `members` entry listing each device's public key and role; devices sign their own log checkpoints. Device loss = owner signs a `revoke` entry stating the last trusted tree size for that device; everything after it is untrusted, everything before stays valid. Rotation = new key added by owner + old key signs a hand-over entry.
- **Witnessing between your own devices**: when the pack syncs, the bench verifies consistency and cosigns the pack's checkpoint (and vice versa); the Mac is the third witness. A device restored from an old SD image will then be *unable* to produce a checkpoint consistent with what the others cosigned — rollback becomes detectable without any cloud.
- **Signature formats**: use C2SP signed notes for checkpoints (so standard tlog tooling verifies them). For human-facing verification, also publish the head as a signed git tag or `ssh-keygen -Y sign` signature [BK] in the public repo. minisign/signify are fine but add nothing over signed notes + SSH signatures here [BK].
- **Hardware keys [BK, verify]**: TPM 2.0 parts (SLB 9670/9672/9673) generally offer RSA and NIST P-256, not Ed25519; ATECC608 is P-256 only; YubiKeys do Ed25519 via OpenPGP/FIDO2 (`sk-ssh-ed25519`). Options: (a) software Ed25519 device keys in a root-only file — adequate for a single-owner bench, and it keeps tlog/iroh compatibility; (b) hardware P-256 keys via TPM with ECDSA checkpoint signatures (a non-standard note key type); (c) hybrid — Ed25519 device key sealed/wrapped by the TPM. Recommend (a) now, designed so (c) can be added, with the owner root on a YubiKey.
- **Public anchoring (optional, off by default)**: periodically stamp the bench checkpoint with OpenTimestamps and/or commit the checkpoint text to the public repo. This proves "this state existed by date D" to third parties; it adds an internet dependency only for the anchoring step and never for verification of content.
- **Sign assertions, not just heads?** Signing each assertion (DSSE) lets a fact be exported and verified alone (e.g. shared publicly); signing only checkpoints is cheaper and sufficient inside the store because inclusion proofs tie any entry to a signed head. Recommendation: checkpoint signatures always; per-assertion DSSE envelope generated on export.

### Gaps
- TPM/ATECC608/YubiKey algorithm support and Rust crates (`tss-esapi`, `at-cryptoauth`) were not verified this session; the search surfaced them but the pages were not read.
- OpenTimestamps offline-verification requirements (needs Bitcoin block headers) and its licence were not stated on the page fetched.
- tlog-witness / tlog-cosignature specs were not read.

---

## 8. Closest existing projects (adopt/fork candidates) and MCP surface design

### Takeaway
Nothing found does the whole job. Least new code for ~80% is **git (canonical files + signed tags) with a rebuildable SQLite index**; the most interesting single-component shortcut is **DoltLite** (versioned, diffable, push/pull SQLite, Apache-2.0, arm64) but it is a months-old beta. The MCP surface should be about eight tools returning refs, hashes and one-line summaries, with bodies behind resource links and every write carrying an expected version and an idempotency key.

### Cited Findings
| Project | Repo / licence / language / activity | What it gives | Missing for this use |
|---|---|---|---|
| DoltLite | [dolthub/doltlite](https://github.com/dolthub/doltlite) — Apache-2.0, C, Rust crate; Beta 0.50.0 on 2026-08-31 ([blog](https://dolthub.com/blog/2026-08-31-doltlite-beta)) | SQLite-compatible engine with commits, branches, merge with conflict tables, `dolt_diff_*`, `dolt_history_*`, point-in-time reads, file/HTTP remotes; Linux arm64 builds | No signing story found; blobs still need an external CAS; five months old; no WAL sidecar; write latency ~3x SQLite for small autocommits |
| Fossil | [fossil-scm.org](https://fossil-scm.org/home/doc/trunk/www/sync.wiki) — C, BSD-2 [BK] | Hash-addressed artifacts, wiki, tickets, unversioned files and sync in one SQLite file | Not Rust; SHA1-based login; unversioned files keep only newest copy; large binaries [BK] |
| Radicle | [radicle.dev](https://radicle.dev/2026/03/30/radicle-1.8.0) — Rust, 1.8.0 on 2026-03-30 | Peer-to-peer git with per-node *signed refs* and downgrade detection | Git-shaped (big binaries); a network daemon more than a store |
| iroh + iroh-blobs + iroh-docs | [iroh 1.0](https://iroh.computer/blog/v1), [iroh-blobs 0.105](https://docs.rs/crate/iroh-blobs/latest), [iroh-docs 0.101](https://docs.rs/crate/iroh-docs/latest) — Rust, MIT/Apache | Key-addressed QUIC, verified blob transfer, signed multi-writer KV with set reconciliation | blobs not production-quality by its own statement; docs has no history |
| Basic Memory | [README](https://raw.githubusercontent.com/basicmachines-co/basic-memory/main/README.md) — Python, AGPL-3.0, v0.23 | "Markdown files are truth, SQLite is index" + MCP tools, observation/relation syntax | No hashing, signing, provenance or evidence model; AGPL; Python on the Pi; sync is "use git/Syncthing" or paid cloud |
| Tessera / tlog-tiles | [Tessera 1.0](https://blog.transparency.dev/tessera-v1-0-is-here) (Go), [tlog_tiles 0.2.0](https://docs.rs/crate/tlog_tiles/latest) (Rust, BSD-3) | Append-only verifiable log on a POSIX filesystem | Only the log layer |
| OBDb, OpenModbusSpecs, Kaitai | [OBDb](https://github.com/OBDb) (CC BY-SA), [OpenModbusSpecs](https://github.com/stekker/OpenModbusSpecs) (MIT), [Kaitai](https://kaitai.io/) (GPLv3+ compiler, MIT runtimes) | Ready-made *content formats* for vehicle signals, Modbus maps, binary framing | Not stores |
| LadybugDB (Kuzu successor) | v0.19.1 as of 2026-08-12 — [OneUptime](https://oneuptime.com/blog/post/2026-08-12-kuzu-archived-pin-0-11-3-fork-or-migrate/view) | Embedded property graph | Unneeded at this scale |

MCP facts relied on below:
- Stateless protocol; state handles must be explicit tool arguments, opaque, with stated lifetime, and expired handles should produce a tool execution error the model can recover from — [MCP tools spec, "Stateful Tools"](https://modelcontextprotocol.io/specification/2026-07-28/server/tools)
- Business-logic and validation failures belong in the result with `isError: true` so the model can self-correct; protocol errors are for malformed requests/unknown tools — [MCP tools spec](https://modelcontextprotocol.io/specification/2026-07-28/server/tools)
- A broken stream loses the in-flight request; the client must re-issue it with a new request id — [MCP changelog](https://modelcontextprotocol.io/specification/2026-07-28/changelog)
- Tool lists should be returned in deterministic order to help prompt caching — [MCP changelog](https://modelcontextprotocol.io/specification/2026-07-28/changelog)
- `structuredContent` + `outputSchema`; a tool returning structured content should also return the serialised JSON as text for backward compatibility — [MCP tools spec](https://modelcontextprotocol.io/specification/2026-07-28/server/tools)
- `resource_link` content items; `ttlMs`/`cacheScope` on reads — [MCP tools spec](https://modelcontextprotocol.io/specification/2026-07-28/server/tools), [MCP resources spec](https://modelcontextprotocol.io/specification/2026-07-28/server/resources)

### Inferences
- **Which gets 80% with least new code?** `git` + one-JSON-file-per-fact + signed tags + `git-annex` (or plain rsync'd `blobs/` dir with a manifest) + a ~500-line indexer into SQLite/FTS5. Git already supplies: content addressing, full history, Merkle root (commit id), signatures (SSH-signed tags), verified transfer of objects, fetch-when-in-range, a change feed (`git diff --name-status A..B`), and as-of reads (`git show rev:path`). The other 20% — per-fact version tokens, verified *streaming* of multi-GB blobs, partial blob carriage with honest "not here", conflict-free concurrent writes, compact proofs, a clean Rust-only binary on the Pi — is what the recommended architecture adds (section 9).
- **DoltLite as the index instead of plain SQLite** would give SQL-level diff/history/as-of "for free" and a ready push/pull. It is the most promising thing to *prototype against*, but making a 2026-03 engine the only home of an audit trail contradicts "prefer boring"; using it as a rebuildable cache keeps the option open at zero risk.
- **MCP tool set (8 tools; names illustrative).**

| Tool | Purpose | Key inputs | Result (structuredContent) |
|---|---|---|---|
| `ks_orient` | Session start: where am I, what changed | `since?` cursor | node id, head cursor `{bench:N, pack:M}`, counts, contested/stale counts, ≤N changed refs, sync age per peer |
| `ks_search` | Hybrid search over facts, pages, captures | `q`, `scope?` (device/kind), `k≤10`, `as_of?` | rows: `ref`, `kind`, one-line summary, score; no bodies |
| `ks_get` | Read by `id` or `id@hash`; device card | `ref`, `depth: summary\|full`, `as_of?` | fact(s) with status + evidence refs; blobs as `resource_link` (`ks://blob/b3/<hash>#page=41`) |
| `ks_changes` | Change feed | `since` cursor, `filter?`, `limit` | rows + `next` cursor |
| `ks_verify` | Premise check / integrity | `refs[]` and/or `blob hashes[]`, `deep?` | per ref: `current \| superseded_by \| retracted \| contested \| stale \| unknown_here`; per blob: `present+verified \| absent \| corrupt` |
| `ks_assert` | Propose a fact (new slot or new version) | fact body, `evidence[]`, `expected?` (hash of current head or `"none"`), `idempotency_key` | `outcome: created \| replayed \| conflict \| rejected`, `ref`, `(device, seq)`, cursor |
| `ks_supersede` / retract | Replace or withdraw | `target ref@hash` (required), new body or reason, `idempotency_key` | same outcome shape |
| `ks_attach` | Register evidence already on disk / captured by another bench tool | path or capture handle, media type, `derived_from?`, `idempotency_key` | blob hashes (b3 + sha256), size, `ref` |

- **Write semantics (maps to the owner's write-safety standard).**
  - *Truthful outcome*: the result states exactly one of `created | replayed | conflict | rejected`, and `created` is returned only after the entry is in the log and the checkpoint is fsynced. It also reports `durable: local` vs `witnessed_by: [bench]` so the agent never assumes a pack-only write has reached the bench.
  - *Optimistic concurrency*: `expected` is the assertion hash of the head the agent last saw (or `"none"` for a new slot). Mismatch → `isError: true`, `outcome: conflict`, with the current head ref and a one-line diff.
  - *Idempotent retries*: required `idempotency_key`; the server stores key → result in the log itself (so it survives restarts and syncs). Because 2026-07-28 removed stream redelivery, a retry after a dropped connection is the normal path, and the replay must return the original result with `outcome: replayed`.
  - *Content-derived idempotency as a backstop*: asserting byte-identical canonical content against the same head is naturally a no-op (same hash).
- **Token efficiency rules**: default `depth=summary`; never inline blob bytes; short refs (`f:01J…@9c1e2a`) with full hashes on request; hard caps (`k≤10`, `limit≤50`) with a `next` cursor; stable field order and deterministic tool order for prompt-cache hits; one-line `summary` precomputed at write time; a compact text rendering in `content` alongside `structuredContent`.
- **Bodies via resources**: `ks://blob/b3/<hash>` (immutable; long `ttlMs`), `ks://assertion/<hash>` (immutable), `ks://fact/<id>` and `ks://device/<id>/card` (mutable heads; short `ttlMs`; subscribable). The two MCP servers (bench, pack) expose the same surface; `ks_orient` tells the agent which node it is talking to and how far behind its peer view is.
- **Read-your-writes across sync**: every write returns `(device, seq)`; every read accepts `min_seen` and answers truthfully `behind: {pack: 4790 < 4812}` rather than silently serving older state.

### Gaps
- Not researched/verified this session: Datasette + sqlite-history, TerminusDB, Logseq DB, Anytype (any-sync), AFFiNE, Obsidian-vault approaches, IPFS Cluster on Pis, Jujutsu-backed stores, and any dedicated "homelab digital twin" or hardware-reverse-engineering notebook project. No project in those categories surfaced in the searches run; a targeted search could still find one.
- Fossil's licence and current version were not confirmed from the page fetched.
- MCP `tool annotations` (`readOnlyHint`, `idempotentHint`, etc.) are mentioned in the 2026-07-28 tools page only generically; their exact fields were not confirmed for this revision.

---

## 9. Synthesis requested by the brief: recommended architecture, simplest variant, hard-to-change decisions, outside-the-box ideas

### Takeaway
Build "immutable files are the truth, SQLite is a cache": a BLAKE3 blob directory, one signed append-only tlog per device, a rebuildable SQLite/FTS5 index, pull-based sync that is verified by comparing signed Merkle roots, and an eight-tool MCP surface with If-Match and idempotency keys. Start with the git-based variant if speed matters; the recommended design adds per-fact versions, verified streaming of large blobs, conflict-free multi-device writes and rollback detection.

### Cited Findings
(Each layer below relies on findings already cited in sections 1–8; the key ones are repeated here for the writer's convenience.)
- iroh 1.0 stable wire (2026-06-15) vs iroh-blobs 0.105 "not yet production quality" — [iroh blog](https://iroh.computer/blog/v1), [docs.rs iroh-blobs](https://docs.rs/crate/iroh-blobs/latest)
- bao-tree 0.16.1 for BLAKE3 verified streaming with range queries — [docs.rs bao-tree](https://docs.rs/crate/bao-tree/latest)
- C2SP checkpoint = origin + size + RFC 6962 root, Ed25519-signed; cosignatures — [c2sp.org/tlog-checkpoint](https://c2sp.org/tlog-checkpoint); Rust `tlog_tiles` 0.2.0 — [docs.rs](https://docs.rs/crate/tlog_tiles/latest)
- rusqlite 0.43.0 bundling SQLite 3.54.1 — [docs.rs rusqlite](https://docs.rs/crate/rusqlite/latest); tantivy 0.27.0 — [docs.rs tantivy](https://docs.rs/crate/tantivy/latest); sqlite-vec stable 0.1.9 — [releases](https://github.com/asg017/sqlite-vec/releases); fastembed 7.8.0 — [docs.rs fastembed](https://docs.rs/crate/fastembed/latest)
- fastcdc 5.0.0, zeekstd 0.6.2 — [docs.rs fastcdc](https://docs.rs/crate/fastcdc/latest), [docs.rs zeekstd](https://docs.rs/crate/zeekstd/latest)
- MCAP has CRC32 only — [MCAP spec](https://mcap.dev/spec)
- Plain files + search competitive with memory frameworks; GraphRAG not worth it on manual-like corpora — [Letta](https://letta.com/blog/benchmarking-ai-agent-memory), [arXiv 2603.29875](https://arxiv.org/abs/2603.29875)
- MCP 2026-07-28: stateless, no stream resumption, `resource_link`, `subscriptions/listen`, `ttlMs` — [changelog](https://modelcontextprotocol.io/specification/2026-07-28/changelog)
- LLM-written protocol parsers compile best as Rust nom/Kaitai but tend to be over-permissive — [LangSec 2025 paper](https://langsechq.gitlab.io/spw25/papers/anantharaman-llm-parser-generation.pdf)

### Inferences

#### (1) Recommended architecture, layer by layer

| Layer | Choice (crate / format) | What it proves or provides | Alternatives considered |
|---|---|---|---|
| Blob store | Directory CAS, address = BLAKE3 of raw bytes (`blake3`); SHA-256 recorded alongside (`sha2`); `.obao` outboard via `bao-tree` for large blobs; atomic temp→fsync→rename | Bit-exact identity of every manual, photo, recording; chunk-level verification and range resume | iroh-blobs store (not production-quality yet); git-annex (not Rust); CDC chunk store (`fastcdc`) — only for genuinely near-duplicate large files |
| Record encoding | RFC 8785 canonical JSON for assertions and entities (`serde_json` + a JCS canonicaliser); numbers with units; no floats where exactness matters | Stable, reproducible hashes independent of compression or key order; human/agent-readable truth | Deterministic CBOR (more compact, less readable); DSSE/in-toto Statement (on export) |
| Log | One append-only log per device in C2SP tlog-tiles layout; RFC 6962 Merkle tree (SHA-256); checkpoint = signed note (Ed25519, `ed25519-dalek`), using/porting `tlog_tiles` | Completeness and order of all statements; no silent deletion or rewrite (consistency proofs); authorship per device | Single shared hash chain (forces one writer); git commits (simplest variant); iroh-docs (no history); Hypercore (no Rust) |
| Trust | Owner root key signs `members`; devices cosign each other's checkpoints (witness files); optional OpenTimestamps / public-repo anchoring of the bench head | Authenticity; rollback/fork detection after SD restore; optional third-party "existed by date" | Sigstore/Rekor (cloud); TPM-held P-256 keys (algorithm mismatch) |
| State / index | SQLite via `rusqlite` (bundled, WAL, `synchronous=FULL`): tables for assertions, slots (current head per subject+predicate+context), entities, edges, evidence, deps; FTS5; vectors via sqlite-vec brute force | Fast queries, as-of reads, contested/stale status. Disposable: rebuilt from logs + blobs, so it proves nothing by itself | DoltLite as the cache (prototype later); tantivy for FTS; oxigraph (RDF) — unnecessary; graph DBs — avoid |
| Sync | Pull-based: (a) fetch peer checkpoint; (b) verify consistency with last seen; (c) fetch new tiles/entries; (d) recompute root = signed root; (e) fetch missing blobs by hash with verified streaming, priority-ordered; (f) cosign. Transport: `iroh` 1.x endpoint + mDNS crate, or `quinn`+`rustls` with pinned keys; fallback: rsync/USB of the same files | Identical-and-complete copy, proven by root equality + per-blob hashes; corrupt chunks rejected mid-transfer; resumable | Syncthing (transport only); CRDT sync (wrong semantics); Fossil-style igot/gimme (borrowed in spirit for blobs) |
| Conflicts | Structural avoidance (single writer per log); `contested` slots surfaced to agent; resolution = new superseding assertion | No fact is ever overwritten by timestamp | LWW (rejected), CRDT merge (rejected) |
| Provenance | Typed fact with evidence-by-hash + locator, method, asserted_by, bitemporal validity, supersedes, depends_on, verify_procedure (section 3) | Every belief is traceable to raw bytes and to who/what asserted it; derived facts go stale when inputs change | PROV-O/RDF natively (heavier), nanopublications (export format) |
| Protocol knowledge | Native executable artifacts as blobs: `.ksy`, DBC, OBDb signalset JSON, Modbus-map YAML (OpenModbusSpecs-style schema), plus tests that replay captures | Protocol claims are *checkable* against recorded traffic | Bespoke schema (rejected) |
| Ingestion (bench/Mac) | Original PDF by hash → text layer → Docling (tables, layout) → page renders; all derived artifacts hashed with `derived_from` + tool version | Citations to page + bbox of an immutable original | Marker/MinerU (licence caution), VLM-only parsing |
| Retrieval | FTS5 BM25 + small-model embeddings (computed on bench via `fastembed`, shipped as derived artifacts; query-time embedding on Pi), RRF; contextual chunk prefixes generated once; device cards for whole-device reads | Good-enough recall without GPU; exact-token queries handled lexically | GraphRAG family (rejected on evidence), rerankers on Pi (skip) |
| Compression | zstd (+trained dictionary) for log segments and text; store already-compressed media raw; Parquet+zstd or zstd-seekable (`zeekstd`) for raw captures; MCAP with zstd chunks at record time | Space efficiency without touching hashes | Re-compressing media (never) |
| MCP surface | Eight tools (section 8); refs = `id@hash`; resource links for bodies; `expected` + `idempotency_key` on writes; cursor = per-device tree sizes; subscriptions for heads | Token-efficient, stateless-friendly, truthful writes, stale-belief checks | Many fine-grained tools (token cost), implicit sessions (removed from MCP) |
| Backup | `restic`/`rustic` of `blobs/` + `logs/` to encrypted third device/off-site | Recoverability; restore verified against witnessed checkpoints | — |

Research-project flags: Willow/Confidential Sync; iroh-blobs ≥1.0 timing; DoltLite as sole store; ML-DSA cosignatures; TPM-backed Ed25519; any LLM-built knowledge graph. Everything in the table above is otherwise built from stable primitives (hash, Merkle tree, Ed25519, SQLite, QUIC/TLS, files).

#### (2) "Simplest thing that works" and what the recommended design adds

**Simplest variant**: a git repository of canonical files (`facts/<entity>/<predicate>.json`, `entities/*.json`, `protocols/*.ksy|*.yaml`), large files in a sibling `blobs/` directory named by SHA-256 with a `blobs.manifest` committed to git (or git-annex), SSH-signed commits and tags (`git tag -s` with `gpg.format=ssh` [BK]), `git fetch` between bench and pack over SSH when in range plus `rsync --checksum` for `blobs/`, and a small Rust indexer that rebuilds SQLite/FTS5 after every pull. MCP tools shell out to git for history/as-of and use the index for search.

What it already proves: integrity and full history (commit id is a Merkle root over everything), authenticity (signed tags), verified object transfer, change feed and as-of reads.

Exactly what the recommended architecture adds beyond it:
1. **Per-fact version tokens** independent of repo commits (an ETag per assertion rather than "the repo changed").
2. **Verified streaming + resume for multi-GB blobs** (bad chunk rejected mid-transfer; range resume), instead of copy-then-checksum.
3. **Conflict-free concurrent writes** (per-device logs) instead of git merges — and `contested` as a first-class state rather than a merge conflict the agent must resolve in a working tree.
4. **Partial carriage with honesty**: the pack holds all facts but only pinned blobs, and `ks_verify` can say "absent here, present on bench" (git-annex gives this too, but not in Rust).
5. **Rollback/fork detection by cross-witnessing** (git has signed tags but no rule that peers refuse a non-fast-forward of a *signed* history unless you build it).
6. **Compact inclusion/consistency proofs** and an explicit cursor = tree sizes (useful for "prove fact X was in the store at checkpoint N" without shipping history).
7. **BLAKE3 + SHA-256 rather than git's SHA-1 default**, and no dependency on the `git` binary/`gix` in the Pi image.
8. **Idempotency records inside the log** so retried writes replay exactly across restarts and sync.

If items 2, 3 and 5 are not yet needed (e.g. only the bench writes at first), the git variant is a legitimate v0, and its files can be replayed into the log later *provided the canonical encoding and IDs are chosen now* (see next list).

#### (3) Eight decisions that are hard to change later
1. **What exactly is hashed**: canonical *uncompressed* bytes; hash function(s) and digest naming (`b3:` / `sha256:` prefixes, multihash-like). Changing it re-addresses everything and breaks every stored citation.
2. **Canonical encoding of assertions** (JCS vs deterministic CBOR, number/unit representation). It fixes all assertion hashes.
3. **Identifier scheme**: stable entity IDs, slot identity = (subject, predicate, context), ULID vs content-derived IDs, and the model-vs-unit split. Renaming subjects later invalidates links and evidence.
4. **Log topology and signing**: one single-writer log per device; Ed25519 vs P-256 (hardware-key compatibility); key-ID derivation; checkpoint/note format.
5. **Where truth lives**: logs + blobs are authoritative, every database is a rebuildable cache. Reversing this later (DB as truth) forfeits the audit trail.
6. **Mutation semantics**: assertions are immutable; change = supersede/retract with explicit links; bitemporal fields (valid-time vs log order) present from the first entry.
7. **Evidence locator grammar**: how a citation points *inside* a blob (PDF page + bbox coordinate system, MCAP topic + time range, byte ranges, register address) — every fact embeds these.
8. **Public/private boundary**: `visibility` on blobs and assertions, inheritance to derived artifacts, and a separate public export — retrofitting this after vendor manuals are mixed in is painful and legally risky.

(Close ninth: the MCP write contract — `expected`, `idempotency_key`, outcome vocabulary — because agents' prompts and habits will be built around it.)

#### (4) Outside-the-box ideas for an agent-first equipment knowledge base
1. **Protocol descriptions as unit-tested artifacts.** Each `.ksy`/register map carries a test manifest of captured sessions (by hash), *including deliberately malformed frames*, and the description's "verified" status is the replay result. Directly counters the over-permissive-parser failure seen in the LangSec study.
2. **Facts that carry their own re-verification procedure.** `verify_procedure` is a parameterised tool-call template + acceptance band ("read holding 0x0015 @ 9600 8N1, expect 0x01E0±5"). Any agent session with bench access can re-earn the `verified` flag and produces fresh evidence as a by-product.
3. **Confidence with a half-life per predicate class.** Firmware version: weeks. Calibration offset: per stated interval. Connector pinout: effectively never. Decay generates a ranked "re-measure" worklist the bench works through when idle ("night rounds"), so the store gets *more* trustworthy with no human effort.
4. **Premise-bound actuation.** Every plan that will write to hardware lists its premises (`ref@hash`…); the actuating MCP tool takes that list and refuses if `ks_verify` reports any as superseded/contested/stale. If-Match applied to the physical world: "you may write register 40021 only while your belief about its scaling is current."
5. **Negative knowledge as first-class facts.** "Tried 19200 baud: no response", "writing 0x00 to reg X latched a fault, needed power cycle" — with evidence. A `hazard` predicate class is consulted automatically before writes, giving the agent scar tissue it cannot forget.
6. **Behavioural fingerprints for device recognition.** Store baseline signatures (Modbus inter-frame timing, idle current, boot banner hash, acoustic spectrum). The pack can then say "this is the EG4 I know, firmware unchanged" — or "same model, behaves differently", which automatically flips context-scoped facts to *unverified in this context*.
7. **Hash-keyed public errata.** When a measurement contradicts a manual, emit an erratum fact whose subject is the *manual's SHA-256 + page + bbox*. The public repo can then publish errata for a document it does not redistribute; anyone holding the byte-identical PDF can apply them. Community knowledge without copyright exposure.
8. **Prediction-before-measurement ("surprise ledger").** Before reading a value the agent records what it expects; the store keeps the error. High-surprise slots are where the model of the device is wrong; aggregate surprise calibrates how far to trust `inferred` facts from that agent/model version.
9. **Generated simulator twin.** From register-map facts, generate a Modbus/CAN simulator so the agent can rehearse a write sequence offline on the pack; divergence between simulator and real device on first contact becomes candidate facts.
10. **Trip manifests.** Before leaving range the agent declares the job ("Tacoma + inverter at site B"); the bench computes the closure of needed facts, cited manual *pages* (not whole PDFs) and baselines, pins them to the pack, and `ks_orient` on the pack reports coverage gaps honestly.
11. **Mutual witnessing as a tamper/rollback alarm.** Bench, pack and Mac cosign each other's checkpoints; a device that boots from an older image cannot produce a consistent head, and the agent is told "this node's memory was rolled back to before X" instead of silently trusting it.
12. **Queryable wiring photos.** Annotated regions (bbox → interface/pin entity) so "which wire is on pin 3?" returns a crop of a hashed photo; changing a wiring fact marks the photo evidence stale and queues a re-photograph prompt.
13. **Counter-evidence is stored, not discarded.** Evidence items have a role (`supports`/`contradicts`); a fact with accumulating contradictions degrades visibly before anyone formally supersedes it.

### Gaps
- The architecture is a reasoned synthesis; none of it was prototyped or benchmarked on a Pi 5 in this research. The first bench tests should be: fully offline iroh/QUIC connection with mDNS; BLAKE3+bao throughput over Wi-Fi; SQLite FTS5 + brute-force vectors at 10^5 rows; power-cut during blob write and during checkpoint publish.
- Crate choices marked only by name (`ed25519-dalek`, `sha2`, `serde_json`, a JCS canonicaliser, `quinn`, `rustls`, `mdns-sd`) were not version-checked this session.
- Legal position on redistributing vendor manuals and on facts-vs-expression is the researcher's general understanding, not sourced or jurisdiction-specific.
- The owner's write-safety standard was taken from the brief's one-line description; its full text was not consulted.
