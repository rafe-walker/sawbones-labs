#!/usr/bin/env python3
"""Flag crates added to Cargo.lock whose names look like other crates.

[#201, sup-014] The Aug 2026 Rust campaign added lookalike crates (proc-macro1
for proc-macro2) to legitimate manifests. For every package in Cargo.lock that
is absent from the base branch's Cargo.lock, compare its name with the popular
crates (ci/popular-crates.txt) and with every crate already in our tree. A name
at edit distance 1, or 2 when both names are at least 6 characters, that is not
identical to the reference name fails the check. '-' and '_' compare equal.

Usage: ci/check_typosquat.py BASE_LOCKFILE [HEAD_LOCKFILE]
  BASE_LOCKFILE  Cargo.lock from the base branch (git show origin/main:Cargo.lock)
  HEAD_LOCKFILE  defaults to Cargo.lock
"""
import sys
import tomllib
from pathlib import Path

CI = Path(__file__).parent


def norm(name):
    return name.lower().replace("_", "-")


def lock_names(path):
    data = tomllib.loads(Path(path).read_text())
    return {p["name"] for p in data.get("package", [])}


def read_list(path):
    out = set()
    for line in Path(path).read_text().splitlines():
        line = line.split("#", 1)[0].strip()
        if line:
            out.add(line)
    return out


def distance(a, b, limit):
    """Levenshtein distance, or limit+1 when it exceeds limit."""
    if abs(len(a) - len(b)) > limit:
        return limit + 1
    prev = list(range(len(b) + 1))
    for i, ca in enumerate(a, 1):
        cur = [i]
        for j, cb in enumerate(b, 1):
            cur.append(min(prev[j] + 1, cur[j - 1] + 1, prev[j - 1] + (ca != cb)))
        prev = cur
    return min(prev[-1], limit + 1)


def near(name, ref):
    """Return the distance if name is suspiciously close to ref, else None."""
    a, b = norm(name), norm(ref)
    limit = 2 if min(len(a), len(b)) >= 6 else 1
    d = distance(a, b, limit)
    if d <= limit and (d > 0 or name != ref):
        return d
    return None


def find_matches(added, existing, popular):
    problems = []
    for name in sorted(added):
        refs = (popular | existing) - {name}
        hits = sorted(
            (near(name, r), r) for r in refs if near(name, r) is not None
        )
        # An added crate that is itself a popular crate name is the real thing.
        if name in popular:
            continue
        if hits:
            problems.append((name, hits))
    return problems


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        return 2
    base = lock_names(sys.argv[1])
    head = lock_names(sys.argv[2] if len(sys.argv) > 2 else "Cargo.lock")
    added = head - base
    allow = read_list(CI / "typosquat-allow.txt")
    popular = read_list(CI / "popular-crates.txt")
    problems = [p for p in find_matches(added - allow, base, popular)]
    if not added:
        print("ok: no crates added to Cargo.lock")
        return 0
    print("added crates: " + ", ".join(sorted(added)))
    if problems:
        for name, hits in problems:
            best = ", ".join(f"{r} (distance {d})" for d, r in hits[:3])
            print(f"error: new dependency '{name}' looks like {best}")
        print(
            "\nPossible typosquat. Check on crates.io that this is the crate you meant, "
            "that it has history and the expected owner, then fix the name in Cargo.toml, "
            "or add the name to ci/typosquat-allow.txt with a reviewer's sign-off. See CONTRIBUTING.md."
        )
        return 1
    print("ok: no added crate looks like a popular or existing crate")
    return 0


if __name__ == "__main__":
    sys.exit(main())
