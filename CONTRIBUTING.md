# Contributing

## The rule: every bug becomes a check

**Every bug or issue we find becomes a CI check or a regression test, so it cannot happen again.** A fix is not done until the thing that would have caught it is in the repo.

- Add the test (or CI check) in the same pull request as the fix.
- Name a regression test `regression_<issue number>_<short_name>`, for example `regression_168_no_float_in_canonical_json`. Put it in a file under a crate's `tests/` directory, or as a `#[test]` function inside the crate.
- If the bug is not a Rust bug (a workflow, a doc, a config), the check is a CI step, a linter rule or a script under `ci/`. Mention the issue number in it, as `#N`.
- Add a row for the issue to [`docs/REGRESSIONS.md`](docs/REGRESSIONS.md): the issue, what went wrong, and the check that now prevents it.

## Pull requests

- A fix pull request says `Fixes #N` in its description, where N is the issue.
- For an issue labelled `bug`, the `regression-guard` workflow fails the pull request unless the diff adds or changes a test or CI file that mentions the issue number, and adds the row to `docs/REGRESSIONS.md`.
- CI must be green to merge. No exceptions for "flaky" or "unrelated" red checks: fix the cause or open an issue and fix that first.
- Use `pull_request` in workflows, never `pull_request_target`.

## GitHub Actions are pinned

Every `uses:` in `.github/workflows/` is pinned to a full 40-character commit SHA, with the version in a comment:

```yaml
- uses: actions/checkout@11d5960a326750d5838078e36cf38b85af677262 # v4.4.0
```

Never use a tag or branch (`@v4`, `@main`): tags can be moved. Look the SHA up with `git ls-remote https://github.com/OWNER/REPO 'refs/tags/vX*'` and use the peeled `^{}` SHA for annotated tags. Dependabot keeps the pins current. Workflows start with `permissions: contents: read` and widen per job only when needed.

## Build scripts, proc macros and new dependencies

A crate with a `build.rs` or a proc macro runs code on every build, so each one is reviewed by name.

- **Allow-list.** `deny.toml` `[bans.build]` allows only the listed crates to have a `build.rs` (`cargo deny check` fails otherwise). cargo-deny does not see proc macros, so `ci/check_build_code.py` also checks every crate with a build script or proc-macro target against `ci/build-code-allowlist.txt`. To add a dependency that trips either check, read its `build.rs` or macro source, then add its name to the list(s) in the same PR and say in the description what you reviewed.
- **Typosquats.** On pull requests, `ci/check_typosquat.py` compares each crate newly added to `Cargo.lock` with `ci/popular-crates.txt` and with the crates already in the tree. A name within edit distance 1 (2 for names of 6+ characters) of another crate fails. Check the crate on crates.io (owner, history), then fix the name or, if it is the right crate, add it to `ci/typosquat-allow.txt`.
- Both run in `.github/workflows/supply-chain.yml`. These files are protected paths: label the PR `guards-change`.

## Before you push

```sh
cargo fmt --all --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
```
