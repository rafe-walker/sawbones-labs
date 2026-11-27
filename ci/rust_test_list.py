#!/usr/bin/env python3
"""List the Rust tests cargo really builds and would run, as JSON for ci/conformance.py.

Builds every test binary `cargo test --workspace` runs (`--no-run`; not examples or
benches), skips every target with `harness = false` (its own main prints whatever
`--list` it likes), then runs each remaining libtest binary with `--list --format
terse`, once plain and once with `--ignored`. A test that does not compile, sits behind a never-true cfg, lives
in a file no `mod` reaches or in an uninvoked macro is not in cargo's list, so the
X-ray cannot count it. `#[ignore]` and `cfg_attr(..., ignore)` show up as ignored.
Limit: build.rs scripts and proc macros of the PR run while it builds, so against a
hostile PR this list can still be forged; conformance.py also needs the test in source.

Usage: python3 -I ci/rust_test_list.py OUT.json [--unlocked] [--root DIR]
Writes {"version": 1, "tests": [{"package", "src", "target", "name", "ignored"}]}.
Stdlib only. Listing runs the test binaries with --list only: nothing is executed.
"""
import json
import os
import re
import subprocess
import sys
import tomllib


def package_name(package_id):
    m = re.search(r"#(?:[^@#]*/)?([\w.-]+)@", package_id) or re.match(r"([\w.-]+) ", package_id)
    if m:
        return m.group(1)
    m = re.search(r"/([\w.-]+)#", package_id)
    return m.group(1) if m else package_id


def no_harness_targets(root, locked):
    """{(package, target name)} for every workspace target whose manifest says
    `harness = false` (lib, bin and test targets), via `cargo metadata` + tomllib."""
    cmd = ["cargo", "metadata", "--no-deps", "--format-version", "1"] + (["--locked"] if locked else [])
    r = subprocess.run(cmd, cwd=root, capture_output=True, text=True)
    if r.returncode != 0:
        raise SystemExit(f"cargo metadata failed: {r.stderr.strip()[:500]}")
    out = set()
    for pkg in json.loads(r.stdout)["packages"]:
        with open(pkg["manifest_path"], "rb") as fh:
            manifest = tomllib.load(fh)
        name = pkg["name"]
        lib = manifest.get("lib", {})
        if lib.get("harness") is False:
            out.add((name, lib.get("name") or name.replace("-", "_")))
        for kind in ("bin", "test"):
            for t in manifest.get(kind, []):
                if t.get("harness") is False and isinstance(t.get("name"), str):
                    out.add((name, t["name"]))
    return out


def listed(exe, cwd, *extra):
    r = subprocess.run([exe, "--list", "--format", "terse", *extra], cwd=cwd, capture_output=True, text=True)
    if r.returncode != 0:
        raise SystemExit(f"{exe} --list failed: {r.stderr.strip()[:500]}")
    return {line[:-len(": test")] for line in r.stdout.splitlines() if line.endswith(": test")}


def main(argv):
    args = list(argv)
    root = os.getcwd()
    if "--root" in args:
        i = args.index("--root")
        root = os.path.abspath(args[i + 1])
        del args[i:i + 2]
    locked = "--unlocked" not in args
    args = [a for a in args if a != "--unlocked"]
    if len(args) != 1:
        print(__doc__)
        return 2
    # Exactly what ci.yml runs (`cargo test --workspace --locked`): no --all-targets, so
    # examples and benches, which that job never runs, are not listed.
    cmd = ["cargo", "test", "--workspace", "--no-run", "--message-format=json"]
    if locked:
        cmd.append("--locked")
    r = subprocess.run(cmd, cwd=root, capture_output=True, text=True)
    if r.returncode != 0:
        sys.stderr.write(r.stderr[-4000:])
        print("::error::cargo could not build the workspace's tests, so no test can count as built.")
        return 1
    tests, seen, skipped = [], set(), []
    top = os.path.realpath(root)
    raw_harness = no_harness_targets(root, locked)
    for line in r.stdout.splitlines():
        try:
            msg = json.loads(line)
        except ValueError:
            continue
        if msg.get("reason") != "compiler-artifact" or not msg.get("executable") or not msg.get("profile", {}).get("test"):
            continue
        exe = msg["executable"]
        if exe in seen:
            continue
        seen.add(exe)
        pkg = package_name(msg["package_id"])
        if (pkg, msg["target"]["name"]) in raw_harness:
            skipped.append(f"{pkg}:{msg['target']['name']}")
            print(f"::warning::{pkg} target {msg['target']['name']} has harness = false: its tests cannot count")
            continue
        src = os.path.realpath(msg["target"]["src_path"])
        if not src.startswith(top + os.sep):
            continue  # a dependency, not ours
        src = os.path.relpath(src, top).replace(os.sep, "/")
        every, ignored = listed(exe, root), listed(exe, root, "--ignored")
        for name in sorted(every | ignored):
            tests.append({"package": package_name(msg["package_id"]), "src": src, "target": msg["target"]["name"],
                          "name": name, "ignored": name in ignored})
    with open(args[0], "w", encoding="utf-8") as fh:
        json.dump({"version": 1, "tests": tests, "skipped_harness_false": skipped}, fh, indent=0, sort_keys=True)
    print(f"{len(tests)} tests from {len(seen)} test binaries, {sum(t['ignored'] for t in tests)} ignored.")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
