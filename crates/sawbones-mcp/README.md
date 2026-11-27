# sawbones-mcp (PROVISIONAL)

The MCP surface v0 for the Sawbones bench ([#103](https://github.com/rafe-walker/sawbones-labs/issues/103)). It serves MCP over stdio, on top of `sawbones-supervisor`, against the simulated bench in `sawbones-sim-modules`: a DC meter (module 1), a comms pod (module 2) and an interlock controller (module 3), with the real interlock state machine behind them.

**v0 is read-only.** Tool names are split by hazard class (read, `invoke_safe`, `invoke_armed`, `invoke_hazard`) so a client's per-tool permission rules land prompts only where the danger is. Only the read class exists: the invoke names are unknown tools, and the build fails if a tool outside the read class is registered.

## Tools

| Tool | Arguments | Returns | Trips the taint latch |
| --- | --- | --- | --- |
| `bench_status` | none | One line: modules by state, rails, fault, the latch, stored handles. `orient` opens with it. | No: host state only |
| `inventory` | none | Each enrolled module: id, hub port, the owner's label, state, enrolled class ceiling, may transmit, measures | No: enrollment and supervisor state only |
| `describe` | `module` | The enrollment, the manifest checked against the digest pinned at enrollment, each page fenced as untrusted data, and an `mf-N` handle. Pages that do not hash to the pin are withheld. | Yes, before the result exists |
| `read` | `module`, `key`, `samples` (1 to 64, default 1) | Counts, the statistics of what the module claims (last, min, max, mean; device-reported, not measured) and an `rd-N` handle. Never the raw samples. | Yes, when any value or refusal code comes back |
| `session_query` | `handle`, `from`, `to`, all optional | No handle: the session's handles. A read handle: statistics over samples `from..to`. A manifest handle: its metadata. | Only for a read handle's statistics |

Every result is a host-written summary, a host line on the latch, then any outside text: one fence per item, escaped onto one line, within a 4096-byte budget, with anything left out counted by the host. The tool list and the `initialize` result are pinned in `tests/golden/`. Bless changes with `SAWBONES_BLESS=1 cargo test -p sawbones-mcp --test golden` and review the diff.

Each `tools/call` is one turn of the latch (`begin_turn` before the tool runs, `end_turn` once the result is made). The latch starts tripped and only an arm cycle or a fresh lease clears it, so in v0 it stays tripped on the simulated bench unless a test turns the switch.

Simulated time moves only while a call is served. Module keys and identify challenges come from `/dev/urandom` when the server starts. No key lives in the source.

## Running it from Claude Code on a Mac

Build and install the binary from a clone of this repository:

```sh
cargo install --locked --path crates/sawbones-mcp
```

That puts `sawbones-mcp` in `~/.cargo/bin`. Add it to Claude Code as a stdio server:

```sh
claude mcp add --transport stdio sawbones -- sawbones-mcp
```

Or, for one project, in `.mcp.json` at the project root:

```json
{
  "mcpServers": {
    "sawbones": {
      "type": "stdio",
      "command": "sawbones-mcp",
      "args": []
    }
  }
}
```

The read tools change nothing, so they can be allowed without a prompt in `.claude/settings.json`. Leave future invoke tools out of `allow`:

```json
{
  "permissions": {
    "allow": [
      "mcp__sawbones__bench_status",
      "mcp__sawbones__inventory",
      "mcp__sawbones__describe",
      "mcp__sawbones__read",
      "mcp__sawbones__session_query"
    ]
  }
}
```

The server takes no arguments, reads no file and needs no secrets.

## Not here yet

- **Transport authentication** ([#211](https://github.com/rafe-walker/sawbones-labs/issues/211)). Anything that can start the process or reach its stdio can read the bench. v0 has no write path.
- The invoke tools, and `plugin_status`, `plugin_logs`, `compute`, `explain`, `ask_operator`, `wait_for`.
- A real bench: v0 serves only the simulation.
- Structured content (`structuredContent`). Results are text only.

## Why a hand-written JSON-RPC subset, not `rmcp`

The official Rust SDK (`rmcp` 3.12.0, features `server` and `transport-io`) builds with `--locked` on the pinned toolchain, and its licences and advisories pass cargo-deny. It adds 33 crates to the tree, six with build scripts (`rmcp`, `getrandom`, `portable-atomic`, `portable-atomic-util`, `ref-cast`, `unicode-general-category`) and seven proc macros. `rmcp`'s own `build.rs` runs `git config core.hooksPath .githooks` in the directory two levels above its manifest whenever `.git` and `.githooks` exist there. With vendored dependencies that directory is this repository's root, so a pull request that adds `.githooks/` would get its hooks installed by the next build. That is not something to allow-list under the build-code rule. v0 needs JSON-RPC over lines with four methods, so it is written here on `serde_json`, and no dependency is new to the tree.
