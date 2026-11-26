#!/usr/bin/env python3
"""Fail a pull request that fixes a `bug` issue without a regression check.

Reads the PR title and body, finds "Fixes #N" style references, and for each
issue labelled `bug` requires that the diff:
  1. adds or changes a test or CI file that mentions the issue number, and
  2. adds a row for that issue to docs/REGRESSIONS.md.

Inputs (environment): PR_TITLE, PR_BODY, BASE_REF (git ref to diff against),
REPO and GH_TOKEN (for `gh api`). For local testing set FAKE_BUG_ISSUES to a
comma-separated list of issue numbers to treat as bugs instead of calling gh.
"""
import os
import re
import subprocess
import sys

KEYWORD = re.compile(r"\b(?:fixes|closes|resolves):?\s+#(\d+)\b", re.IGNORECASE)
TEST_PATH = re.compile(r"(^|/)tests/|^\.github/workflows/|^ci/")
LEDGER = "docs/REGRESSIONS.md"


def run(*cmd):
    return subprocess.run(cmd, check=True, capture_output=True, text=True).stdout


def referenced_issues(text):
    return sorted({int(n) for n in KEYWORD.findall(text)})


def is_bug(n):
    fake = os.environ.get("FAKE_BUG_ISSUES")
    if fake is not None:
        return str(n) in {s.strip() for s in fake.split(",")}
    out = run("gh", "api", f"repos/{os.environ['REPO']}/issues/{n}", "--jq", "[.labels[].name] | join(\",\")")
    return "bug" in out.strip().split(",")


def changed_files(base):
    return [f for f in run("git", "diff", "--name-only", f"{base}...HEAD").splitlines() if f]


def added_lines(base, path):
    diff = run("git", "diff", "-U0", f"{base}...HEAD", "--", path)
    return [l[1:] for l in diff.splitlines() if l.startswith("+") and not l.startswith("+++")]


def mentions(n, text):
    return re.search(rf"(regression_{n}(?![0-9])|#{n}(?![0-9]))", text) is not None


def is_test_code_in_src(path, added):
    # `src/` files count only when the added lines sit in test code.
    return "/src/" in f"/{path}" and path.endswith(".rs") and any("#[test]" in l or "#[cfg(test)]" in l or "mod tests" in l for l in added)


def main():
    base = os.environ.get("BASE_REF", "origin/main")
    text = f"{os.environ.get('PR_TITLE', '')}\n{os.environ.get('PR_BODY', '')}"
    issues = referenced_issues(text)
    if not issues:
        print("No Fixes/Closes/Resolves #N reference: nothing to check.")
        return 0
    files = changed_files(base)
    failures = []
    for n in issues:
        if not is_bug(n):
            print(f"#{n} is not labelled bug: skipped.")
            continue
        has_check = False
        for f in files:
            if f == LEDGER:
                continue
            added = added_lines(base, f)
            blob = "\n".join(added) + "\n" + f
            if not mentions(n, blob):
                continue
            if TEST_PATH.search(f) or is_test_code_in_src(f, added):
                has_check = True
                break
            # Test code in src/ may mention the issue away from the #[test] line.
            if "/src/" in f"/{f}" and f.endswith(".rs") and any(mentions(n, l) for l in added):
                has_check = True
                break
        ledger_ok = any(re.match(rf"\|\s*#{n}\s*\|", l) for l in added_lines(base, LEDGER)) if LEDGER in files else False
        if not has_check:
            failures.append(f"#{n} is a bug, but the diff adds no test or CI check that mentions it. Add `regression_{n}_<name>` (in a tests/ file or a #[test]) or a CI step that mentions #{n}.")
        if not ledger_ok:
            failures.append(f"#{n} is a bug, but the diff adds no row for it to {LEDGER}. Add a row starting `| #{n} |`.")
        if has_check and ledger_ok:
            print(f"#{n}: regression check and ledger row found.")
    if failures:
        for f in failures:
            print(f"::error::{f}")
        print("Every bug becomes a check. See CONTRIBUTING.md.")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
