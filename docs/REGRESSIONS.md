# Regression ledger

Every bug found becomes a check, so it cannot come back. One row per issue, newest last. See [`CONTRIBUTING.md`](../CONTRIBUTING.md) for the rule.

| Issue | What went wrong | The check that now prevents it |
| --- | --- | --- |
| #167 | GitHub Actions were referenced by movable tags, which can be repointed at different code. | Every action is pinned to a full commit SHA, and the `lint-workflows` workflow (zizmor) fails on unpinned actions. |
| #168 | A float was serialised beside the decimal text in hashed readings, so the hash could differ between machines. | The golden test `canonical_json_is_pinned_and_has_no_float` in `crates/sawbones-reading/src/reading.rs` pins the exact canonical JSON and asserts it contains no float. |
| #168 | A public `trust` field let any code choose its own trust tier. | The `compile_fail` doctest on `Reading` in `crates/sawbones-reading/src/reading.rs` fails the doc tests if `reading.trust` ever becomes assignable from outside the crate. |
| #187 | The value of a write never affected the class and had no limit, so a write-map entry could not refuse an out-of-range value. | `regression_187_*` tests in `crates/sawbones-policy/tests/regression_187.rs`: an entry's optional `ValueRange` (carrying the entry's tier) refuses out-of-range writes with `ValueOutOfRange`, and entries with no range behave as before. |
| #187 | Code could copy outside text into an `Ident` through `WorldText::as_untrusted_str` and use it as a port or signal name. | `regression_187_world_text_cannot_become_an_identifier` in the same file: `PortId` and `SignalId` are built only from 1 to 64 bytes of `a-z 0-9 _ -` starting with a letter, so instruction-shaped or out-of-set text is rejected. |
