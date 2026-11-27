#!/usr/bin/env python3
"""Regression guard: a bug fix must add a real check, and checks must not shrink.

For every issue the PR closes (any GitHub closing keyword in the title or body)
that is labelled `bug`, the diff must:
  1. add or change a Rust test function (a `#[test]` fn, or any fn in a file
     under `tests/`) whose name or comment-free body contains `regression_<N>`,
     or add a non-comment line naming `#<N>` or `regression_<N>` in a CI
     workflow or a script under `ci/`; a comment alone does not count, and
  2. add a row `| #<N> |` to docs/REGRESSIONS.md.

Independently of any issue, the PR fails unless it carries the label
`guards-change` when it:
  - deletes or renames away a Rust test function,
  - adds `#[ignore]`,
  - removes a CI job or step from a workflow,
  - edits this script or its workflow.

Inputs (environment): PR_TITLE, PR_BODY, PR_LABELS (JSON array of label names),
BASE_REF (git ref to diff against), REPO and GH_TOKEN (for `gh api`). For local
testing set FAKE_BUG_ISSUES to a comma-separated list of issue numbers to treat
as bugs instead of calling gh.
"""
import collections
import json
import os
import re
import subprocess
import sys

GUARD_LABEL = "guards-change"
LEDGER = "docs/REGRESSIONS.md"
GUARD_FILES = {"ci/regression_guard.py", ".github/workflows/regression-guard.yml"}
CI_PATH = re.compile(r"^\.github/workflows/|^ci/")
TESTS_DIR = re.compile(r"(^|/)tests/")

CLOSING = r"(?:close[sd]?|fix(?:e[sd])?|resolve[sd]?)"


def run(*cmd):
    return subprocess.run(cmd, check=True, capture_output=True, text=True).stdout


def referenced_issues(text, repo=None):
    """Issue numbers of `repo` closed by `text`, for every closing keyword."""
    repo = (repo or os.environ.get("REPO") or "rafe-walker/sawbones-labs").lower()
    found = set()
    ref = rf"(?:(?P<repo>[\w.-]+/[\w.-]+))?#(?P<n>\d+)\b"
    url = rf"https://github\.com/(?P<urepo>[\w.-]+/[\w.-]+)/issues/(?P<un>\d+)\b"
    # re.ASCII: Arabic-Indic and other non-ASCII digits must not parse as an issue number.
    pattern = re.compile(rf"(?<![\w-]){CLOSING}\b:?\s+(?:{ref}|{url})", re.IGNORECASE | re.ASCII)
    for m in pattern.finditer(text):
        if m.group("n"):
            if m.group("repo") is None or m.group("repo").lower() == repo:
                found.add(int(m.group("n")))
        elif m.group("urepo").lower() == repo:
            found.add(int(m.group("un")))
    return sorted(found)


def is_bug(n):
    fake = os.environ.get("FAKE_BUG_ISSUES")
    if fake is not None:
        return str(n) in {s.strip() for s in fake.split(",")}
    out = run("gh", "api", f"repos/{os.environ['REPO']}/issues/{n}", "--jq", "[.labels[].name] | join(\",\")")
    return "bug" in out.strip().split(",")


def git_show(rev, path):
    r = subprocess.run(["git", "show", f"{rev}:{path}"], capture_output=True, text=True)
    return r.stdout if r.returncode == 0 else None


def changed_files(base):
    return [f for f in run("git", "diff", "--name-only", "--no-renames", f"{base}...HEAD").splitlines() if f]


def added_lines(base, path):
    diff = run("git", "diff", "-U0", "--no-renames", f"{base}...HEAD", "--", path)
    return [l[1:] for l in diff.splitlines() if l.startswith("+") and not l.startswith("+++")]


# ---------------------------------------------------------------- Rust parsing

_TOKEN = re.compile(r'//[^\n]*|/\*.*?\*/|r(#*)".*?"\1|"(?:\\.|[^"\\])*"|\'(?:\\.[^\']*|[^\\\'])\'', re.DOTALL)


def _blank(m, keep_strings):
    s = m.group(0)
    if s.startswith("/"):
        return re.sub(r"[^\n]", " ", s)
    return s if keep_strings else re.sub(r"[^\n]", "_", s)


def rust_views(src):
    """Two same-length views of `src`: (code with comments blanked,
    code with comments and string contents blanked)."""
    return (_TOKEN.sub(lambda m: _blank(m, True), src), _TOKEN.sub(lambda m: _blank(m, False), src))


_FN = re.compile(r"((?:#\[[^\]]*\]\s*)*)(?:pub(?:\([^)]*\))?\s+)?(?:async\s+|const\s+|unsafe\s+)*fn\s+(\w+)")
_TEST_ATTR = re.compile(r"#\[\s*(?:[\w]+::)*test\b")


def rust_fns(src, in_tests_dir=False):
    """Return {name: (is_test, comment-free body text)} for the fns in `src`."""
    code, skeleton = rust_views(src)
    out = {}
    for m in _FN.finditer(skeleton):
        start = m.start()
        brace = skeleton.find("{", m.end())
        semi = skeleton.find(";", m.end())
        if brace == -1 or (semi != -1 and semi < brace):
            continue
        depth, i = 0, brace
        while i < len(skeleton):
            c = skeleton[i]
            depth += c == "{"
            depth -= c == "}"
            i += 1
            if depth == 0:
                break
        is_test = bool(_TEST_ATTR.search(m.group(1)))
        out[m.group(2)] = (is_test, " ".join(code[start:i].split()), in_tests_dir or is_test)
    return out


def test_fn_names(src, in_tests_dir=False):
    return [n for n, (is_test, _, _) in rust_fns(src, in_tests_dir).items() if is_test]


# ------------------------------------------------------------- workflow parsing


def workflow_items(text):
    """Job ids and step identities of a workflow, by indentation (no YAML lib)."""
    items = set()
    in_jobs = False
    job = None
    for line in text.splitlines():
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        if re.match(r"^\S", line):
            in_jobs = line.startswith("jobs:")
            job = None
            continue
        if not in_jobs:
            continue
        m = re.match(r"^  ([\w-]+):\s*$", line)
        if m:
            job = m.group(1)
            items.add(f"job {job}")
            continue
        m = re.match(r"^\s+-\s+(?:name|uses|run):\s*(.*?)\s*$", line)
        if m and job:
            ident = re.sub(r"\s+#.*$", "", m.group(1))[:100]
            items.add(f"step {job}: {ident}")
    return items


def non_comment(lines, prefixes=("#", "//")):
    return [l for l in lines if l.strip() and not l.lstrip().startswith(prefixes)]


# ------------------------------------------------------------------ the checks


def find_removals(base_rev, files):
    """What the PR removes: lists of strings describing each removal."""
    removed = []
    for f in files:
        if f.startswith(".github/workflows/") and f.endswith((".yml", ".yaml")):
            before, after = git_show(base_rev, f), git_show("HEAD", f)
            lost = workflow_items(before or "") - workflow_items(after or "")
            for item in sorted(lost):
                removed.append(f"CI {item} removed from {f}")
    # Test fns compare by name across all changed Rust files so that a move
    # between files is not a deletion.
    b, h = collections.Counter(), collections.Counter()
    where = {}
    for f in files:
        if not f.endswith(".rs"):
            continue
        for rev, c in ((base_rev, b), ("HEAD", h)):
            src = git_show(rev, f)
            if src is None:
                continue
            for n in test_fn_names(src, bool(TESTS_DIR.search(f))):
                c[n] += 1
                if rev == base_rev:
                    where.setdefault(n, f)
    for n in sorted((b - h).elements()):
        removed.append(f"test function `{n}` removed (was in {where[n]})")
    for f in files:
        if f.endswith(".rs"):
            for l in non_comment(added_lines(base_rev, f), ("//", "*", "/*")):
                if re.search(r"#\[\s*ignore\b|cfg_attr\([^\]]*\bignore\b", l):
                    removed.append(f"`#[ignore]` added in {f}: {l.strip()}")
    return removed


def has_real_check(n, base_rev, files):
    """A changed Rust test fn naming regression_<n>, or a CI step/script naming it."""
    needle = re.compile(rf"regression_{n}(?![0-9])")
    for f in files:
        if f == LEDGER or f in GUARD_FILES:
            continue
        if f.endswith(".rs"):
            head = git_show("HEAD", f)
            if head is None:
                continue
            base = git_show(base_rev, f)
            tests_dir = bool(TESTS_DIR.search(f))
            before = rust_fns(base, tests_dir) if base else {}
            for name, (is_test, body, counts) in rust_fns(head, tests_dir).items():
                if not counts or before.get(name, (None, None))[1] == body:
                    continue
                if needle.search(name) or needle.search(body):
                    return f"{f}::{name}"
        elif CI_PATH.search(f):
            ref = re.compile(rf"(#{n}(?![0-9])|regression_{n}(?![0-9]))")
            if any(ref.search(l) for l in non_comment(added_lines(base_rev, f))):
                return f
    return None


def main():
    base = os.environ.get("BASE_REF", "origin/main")
    base_rev = run("git", "merge-base", base, "HEAD").strip()
    labels = set(json.loads(os.environ.get("PR_LABELS") or "[]"))
    text = f"{os.environ.get('PR_TITLE', '')}\n{os.environ.get('PR_BODY', '')}"
    files = changed_files(base)
    errors = []

    guard_edits = sorted(GUARD_FILES & set(files))
    removals = find_removals(base_rev, files)
    if (guard_edits or removals) and GUARD_LABEL not in labels:
        for f in guard_edits:
            errors.append(f"{f} is changed: the PR needs the `{GUARD_LABEL}` label and the clean-room reviewer.")
        for r in removals:
            errors.append(f"{r}: the PR needs the `{GUARD_LABEL}` label.")
    elif guard_edits or removals:
        for item in guard_edits + removals:
            print(f"NOTICE (allowed by `{GUARD_LABEL}`): {item}")

    issues = referenced_issues(text)
    if not issues:
        print("No closing keyword for an issue of this repo: no regression check needed.")
    for n in issues:
        if not is_bug(n):
            print(f"#{n} is not labelled bug: skipped.")
            continue
        found = has_real_check(n, base_rev, files)
        ledger_ok = LEDGER in files and any(re.match(rf"\|\s*#{n}\s*\|", l) for l in added_lines(base_rev, LEDGER))
        if not found:
            errors.append(f"#{n} is a bug, but the diff adds no real check for it: add a Rust test fn (#[test], or any fn under tests/) whose name or body contains `regression_{n}`, or a CI step or ci/ script naming #{n}. A comment does not count.")
        if not ledger_ok:
            errors.append(f"#{n} is a bug, but the diff adds no row for it to {LEDGER}. Add a row starting `| #{n} |`.")
        if found and ledger_ok:
            print(f"#{n}: regression check ({found}) and ledger row found.")
    if errors:
        for e in errors:
            print(f"::error::{e}")
        print("Every bug becomes a check, and checks do not shrink silently. See CONTRIBUTING.md.")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
