# Red-team attack catalog

Work item [#196](https://github.com/rafe-walker/sawbones-labs/issues/196). A growing list of ways to break or get around Sawbones' safeguards, kept as data, with the test that proves each one is blocked. Every card describes a defensive test against our own components. Nothing here is a working exploit for anyone else's system.

The source notes the first 119 cards came from are in [`research/round-6-red-team/`](../research/round-6-red-team/). The cards in [`catalog/`](catalog/) are the maintained form: statuses there were re-checked against the code on main.

## Summary

<!-- summary:start (generated: REDTEAM_BLESS=1 cargo test -p sawbones-redteam) -->

| Lane | Blocked | Partial | Gap | Total |
| --- | --- | --- | --- | --- |
| Agent injection (`agent-injection.toml`) | 9 | 13 | 9 | 31 |
| Bus (`bus.toml`) | 11 | 6 | 12 | 29 |
| Supply chain (`supply-chain.toml`) | 6 | 9 | 15 | 30 |
| Interlock hardware (`interlock-hardware.toml`) | 4 | 27 | 6 | 37 |
| **All lanes** | **30** | **55** | **42** | **127** |

<!-- summary:end -->

A test fails if this table is stale. Regenerate it with `REDTEAM_BLESS=1 cargo test -p sawbones-redteam`.

## The ratchet

Run on every `cargo test` by the [`sawbones-redteam`](../crates/sawbones-redteam/) crate:

1. Every catalog file parses, ids are unique, and every field below is present.
2. A `blocked` card names a Rust test or a CI job that exists in the repository and is not ignored. An attack moves to `blocked` only with a real proof.
3. A `partial` or `gap` card carries the number of the item that tracks the fix. Nothing is a gap without a tracked fix.
4. The summary above matches the catalog.

An extra check, ignored by default, confirms the tracking items are still open: `GITHUB_TOKEN=... cargo test -p sawbones-redteam -- --ignored`. It needs network.

## Lanes

| File | Prefix | What it covers |
| --- | --- | --- |
| [`agent-injection.toml`](catalog/agent-injection.toml) | `inj` | Manipulating the agent through the physical world, its tools and its notes |
| [`bus.toml`](catalog/bus.toml) | `bus` | Industrial bus write paths: the bus guard, the policy engine, CAN and Modbus |
| [`supply-chain.toml`](catalog/supply-chain.toml) | `sup` | Plugin sandbox, dependencies, GitHub Actions, the pipeline and its reviewers |
| [`interlock-hardware.toml`](catalog/interlock-hardware.toml) | `hw` | Interlock failure modes in the state machine, the wiring and the chip |

## Card format

Each file is an array of `[[attack]]` tables. Unknown fields are refused.

| Field | Meaning |
| --- | --- |
| `id` | Lane prefix, a dash and three digits, such as `bus-003`. Never reused. |
| `title` | One line. |
| `category` | The class of attack. |
| `target` | The component of ours that is attacked. |
| `how` | A defensive description of how the attack works. |
| `expected` | What should happen: blocked, and for which reason. |
| `severity` | `high`, `medium` or `low`. The research's `critical` is `high`. |
| `sources` | A list of public https URLs. |
| `status` | `blocked`, `partial` or `gap`. |
| `test` | For `blocked`: the proof (see below). For the others: a description of the test to write. |
| `issue` | Required for `partial` and `gap`: the number of the open item that tracks the fix. |
| `note` | Optional. What the re-check against the code found, when it differs from the research. |

### Statuses

- `blocked`: a real test or CI job on main proves it. The whole property is proven, not part of it.
- `partial`: some of it is proven and the rest is not. `issue` tracks the rest.
- `gap`: not blocked. `issue` tracks the fix, and `test` says what to write.

### Proofs

A `blocked` card's `test` is one or more proofs joined by ` + `:

- `crate-name::test_name`: a `#[test]` function in a `.rs` file under `crates/crate-name/`. Attack tests written for this catalog are in [`crates/sawbones-redteam/tests/`](../crates/sawbones-redteam/tests/) and are named `redteam_<lane prefix>_<number>_<name>`.
- `ci:job-id`: a job in a workflow under `.github/workflows/`, such as `ci:zizmor`.
- `ci:ci/path`: a script or checker under [`ci/`](../ci/).

Compile-fail doctests are not proofs here, because the checker finds `#[test]` functions only. Pair them with a test.

## How to add a card

1. Pick the lane and the next free number. Append an `[[attack]]` table with every field.
2. If the code already blocks it, write the deterministic test (no model, no network, no clock), name it `redteam_<prefix>_<number>_<name>`, and set `status = "blocked"` with that test name. Or cite an existing test, if it proves the whole property.
3. If it is not blocked, set `status` to `partial` or `gap`, set `issue` to an open item that tracks the fix (open one if none fits), and describe the test to write in `test`.
4. Run `REDTEAM_BLESS=1 cargo test -p sawbones-redteam` to refresh the summary, then the usual `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings` and `cargo test --workspace --locked`.

To move a card to `blocked` when the fix lands, land the test in the same pull request, change `status`, and remove `issue`.

## Call sites of `as_untrusted_str`

[`as_untrusted_str.allow`](as_untrusted_str.allow) lists every file that calls `WorldText::as_untrusted_str`, the one place outside text leaves its wrapper (card `inj-002`). A test fails on a new call site or on a stale entry, so each new site is a reviewed event.
