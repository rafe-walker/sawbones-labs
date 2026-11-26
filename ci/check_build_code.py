#!/usr/bin/env python3
"""Fail if a crate with a build script or a proc-macro target is not allow-listed.

[#201, sup-013] Build scripts and proc macros execute arbitrary code during a
normal build. cargo-deny's `bans.build.allow-build-scripts` covers build.rs
only, so this script covers both, from `cargo metadata`.

Usage: ci/check_build_code.py [allowlist-file]
"""
import json
import subprocess
import sys
from pathlib import Path

ALLOWLIST = Path(sys.argv[1] if len(sys.argv) > 1 else Path(__file__).with_name("build-code-allowlist.txt"))


def load_allowlist(path):
    names = set()
    for line in path.read_text().splitlines():
        line = line.split("#", 1)[0].strip()
        if line:
            names.add(line)
    return names


def build_code_crates(meta):
    found = {}
    for pkg in meta["packages"]:
        kinds = set()
        for target in pkg["targets"]:
            if "custom-build" in target["kind"]:
                kinds.add("build script")
            if "proc-macro" in target["kind"]:
                kinds.add("proc macro")
        if kinds:
            found.setdefault(pkg["name"], set()).update(kinds)
    return found


def main():
    out = subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--locked", "--all-features"],
        check=True, capture_output=True, text=True,
    ).stdout
    found = build_code_crates(json.loads(out))
    allowed = load_allowlist(ALLOWLIST)
    new = sorted(set(found) - allowed)
    for name in sorted(allowed - set(found)):
        print(f"note: {name} is allow-listed but no longer has build-time code; remove it from {ALLOWLIST.name}")
    if new:
        for name in new:
            print(f"error: {name} has a {' and a '.join(sorted(found[name]))} and is not in {ALLOWLIST.name}")
        print(
            "\nBuild scripts and proc macros run arbitrary code on every build. Review the "
            "crate's build.rs / macro source, then add its name to ci/build-code-allowlist.txt "
            "(and to deny.toml [bans.build] allow-build-scripts if it has a build.rs). See CONTRIBUTING.md."
        )
        return 1
    print(f"ok: {len(found)} crates with build-time code, all allow-listed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
