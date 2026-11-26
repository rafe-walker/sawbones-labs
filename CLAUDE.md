# Instructions for AI agents working in this repo

This is a public repository. Read [`CONTRIBUTING.md`](CONTRIBUTING.md) first.

- **Regression rule.** Every bug or issue becomes a CI check or regression test (`regression_<issue>_<name>`) plus a row in `docs/REGRESSIONS.md`, in the same pull request as the fix. The fix PR says `Fixes #N`.
- **Pin every action** to a full 40-character commit SHA with a `# vX.Y.Z` comment. Never a tag or branch. Keep `permissions: contents: read` at the top of every workflow. Use `pull_request`, never `pull_request_target`.
- **Commit identity:** `Joshua Bryan <45343084+rafe-walker@users.noreply.github.com>`. End every commit message with the `Co-Authored-By` and `Claude-Session` trailers you were given.
- **Work on a branch** and open a draft pull request. Never merge your own PR: the coordinating agent merges after review and green checks.
- **Nothing private in this repo:** no addresses, secrets, keys, tokens or raw site recordings.
- Before pushing: `cargo fmt --all --check`, `cargo clippy --locked --all-targets -- -D warnings`, `cargo test --locked`.
