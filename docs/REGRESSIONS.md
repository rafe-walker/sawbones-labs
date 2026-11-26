# Regression ledger

Every bug found becomes a check, so it cannot come back. One row per issue, newest last. See [`CONTRIBUTING.md`](../CONTRIBUTING.md) for the rule.

| Issue | What went wrong | The check that now prevents it |
| --- | --- | --- |
| #167 | GitHub Actions were referenced by movable tags, which can be repointed at different code. | Every action is pinned to a full commit SHA, and the `lint-workflows` workflow (zizmor) fails on unpinned actions. |
| #168 | A float was serialised beside the decimal text in hashed readings, so the hash could differ between machines. | The golden test `canonical_json_is_pinned_and_has_no_float` in `crates/sawbones-reading/src/reading.rs` pins the exact canonical JSON and asserts it contains no float. |
| #168 | A public `trust` field let any code choose its own trust tier. | The `compile_fail` doctest on `Reading` in `crates/sawbones-reading/src/reading.rs` fails the doc tests if `reading.trust` ever becomes assignable from outside the crate. |
