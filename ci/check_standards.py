#!/usr/bin/env python3
"""Check the versioned working standards under standards/ (Python 3.11+, stdlib only).

Rules, per standards/<name>/<version>/requirements.toml:
  * the file parses as TOML and holds a [[requirement]] array;
  * every record has exactly the allowed keys, with the right types;
  * IDs are well formed (HWS-<AREA>-<n>), unique, and their area code matches
    the `area` field; an ID keeps the same area in every version;
  * level is MUST or SHOULD, verify is a non-empty list of test, analysis or
    inspection, version_added is a version that exists;
  * a removed requirement is marked `withdrawn_in`, never deleted: an ID that
    was in an earlier version must still be present in a later one;
  * the version has a section in standards/<name>/CHANGELOG.md.

A version is released when standards/<name>/<version>/RELEASED exists. The
RELEASED file lists `<sha256>  <relative path>` for every other file in the
folder, and a released folder is immutable: the check fails if a file in it
differs from its manifest, and (with --base REF) if any file in it changed
since REF, other than the commit that adds RELEASED itself.

Usage: python3 -I ci/check_standards.py [--root DIR] [--base REF]
Issue: guards for the HWS working standard (see standards/README.md).
"""
from __future__ import annotations

import argparse
import hashlib
import re
import subprocess
import sys
import tomllib
from pathlib import Path

ID_RE = re.compile(r"^HWS-([A-Z]{3})-([1-9][0-9]*)$")
VERSION_RE = re.compile(r"^[0-9]+\.[0-9]+(\.[0-9]+)?$")
LEVELS = {"MUST", "SHOULD"}
VERIFY = {"test", "analysis", "inspection"}
STR_KEYS = {"id", "area", "level", "title", "text", "rationale", "version_added"}
OPTIONAL = {"withdrawn_in"}
LIST_KEYS = {"borrows", "verify"}
ALLOWED = STR_KEYS | LIST_KEYS | OPTIONAL
RELEASED = "RELEASED"


def vkey(v: str) -> tuple[int, ...]:
    return tuple(int(p) for p in v.split("."))


def versions(std_dir: Path) -> list[str]:
    found = [p.name for p in std_dir.iterdir() if p.is_dir() and VERSION_RE.match(p.name)]
    return sorted(found, key=vkey)


def check_records(version: str, path: Path, all_versions: list[str]) -> tuple[list[str], dict[str, dict]]:
    errs: list[str] = []
    recs: dict[str, dict] = {}
    try:
        data = tomllib.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, tomllib.TOMLDecodeError) as e:
        return [f"{path}: cannot parse: {e}"], recs
    items = data.get("requirement")
    if not isinstance(items, list) or not items:
        return [f"{path}: needs a non-empty [[requirement]] array"], recs
    extra_top = set(data) - {"requirement"}
    if extra_top:
        errs.append(f"{path}: unexpected top-level keys {sorted(extra_top)}")
    for n, rec in enumerate(items, 1):
        where = f"{path} record {n}"
        if not isinstance(rec, dict):
            errs.append(f"{where}: not a table")
            continue
        rid = rec.get("id")
        where = f"{path} {rid if isinstance(rid, str) else f'record {n}'}"
        missing = (STR_KEYS | LIST_KEYS) - set(rec)
        if missing:
            errs.append(f"{where}: missing keys {sorted(missing)}")
        unknown = set(rec) - ALLOWED
        if unknown:
            errs.append(f"{where}: unknown keys {sorted(unknown)} (status is computed elsewhere, not stored)")
        for k in STR_KEYS | (OPTIONAL & set(rec)):
            if k in rec and (not isinstance(rec[k], str) or not rec[k].strip()):
                errs.append(f"{where}: {k} must be a non-empty string")
        for k in LIST_KEYS:
            v = rec.get(k)
            if k in rec and (not isinstance(v, list) or not all(isinstance(x, str) and x.strip() for x in v)):
                errs.append(f"{where}: {k} must be a list of non-empty strings")
        if isinstance(rid, str):
            m = ID_RE.match(rid)
            if not m:
                errs.append(f"{where}: id must look like HWS-ABC-1")
            elif rec.get("area") != m.group(1):
                errs.append(f"{where}: area {rec.get('area')!r} does not match the id")
            if rid in recs:
                errs.append(f"{where}: duplicate id")
            recs[rid] = rec
        if rec.get("level") not in LEVELS:
            errs.append(f"{where}: level must be MUST or SHOULD")
        v = rec.get("verify")
        if isinstance(v, list) and (not v or not set(v) <= VERIFY):
            errs.append(f"{where}: verify must be a non-empty list drawn from {sorted(VERIFY)}")
        if isinstance(rec.get("borrows"), list) and not rec["borrows"]:
            errs.append(f"{where}: borrows must not be empty")
        va = rec.get("version_added")
        if isinstance(va, str):
            if va not in all_versions:
                errs.append(f"{where}: version_added {va!r} is not a version folder")
            elif vkey(va) > vkey(version):
                errs.append(f"{where}: version_added {va} is later than the folder {version}")
        wi = rec.get("withdrawn_in")
        if isinstance(wi, str) and wi not in all_versions:
            errs.append(f"{where}: withdrawn_in {wi!r} is not a version folder")
    return errs, recs


def manifest(folder: Path) -> dict[str, str]:
    out = {}
    for p in sorted(folder.rglob("*")):
        if p.is_file() and p.name != RELEASED:
            out[p.relative_to(folder).as_posix()] = hashlib.sha256(p.read_bytes()).hexdigest()
    return out


def check_released(folder: Path) -> list[str]:
    marker = folder / RELEASED
    if not marker.is_file():
        return []
    want: dict[str, str] = {}
    for line in marker.read_text(encoding="utf-8").splitlines():
        if not line.strip():
            continue
        m = re.fullmatch(r"([0-9a-f]{64})  (\S.*)", line)
        if not m:
            return [f"{marker}: each line must be '<sha256>  <relative path>'"]
        want[m.group(2)] = m.group(1)
    have = manifest(folder)
    errs = []
    for name in sorted(set(want) | set(have)):
        if name not in have:
            errs.append(f"{folder}: released file {name} is missing")
        elif name not in want:
            errs.append(f"{folder}: file {name} is not in the RELEASED manifest")
        elif want[name] != have[name]:
            errs.append(f"{folder}: released file {name} was modified")
    return errs


def git(root: Path, *args: str) -> subprocess.CompletedProcess:
    return subprocess.run(["git", "-C", str(root), *args], capture_output=True, text=True)


def check_released_unchanged(root: Path, base: str) -> list[str]:
    """Fail if a folder released at BASE has any change since."""
    if git(root, "rev-parse", "--verify", "--quiet", f"{base}^{{commit}}").returncode != 0:
        return [f"base ref {base!r} does not resolve, so released folders cannot be compared"]
    listing = git(root, "ls-tree", "-r", "--name-only", base, "--", "standards")
    errs = []
    for name in listing.stdout.splitlines():
        parts = name.split("/")
        if len(parts) == 4 and parts[3] == RELEASED:
            folder = "/".join(parts[:3])
            diff = git(root, "diff", "--name-status", "--no-renames", f"{base}...HEAD", "--", folder)
            for line in diff.stdout.splitlines():
                errs.append(f"{folder} was released at {base}; it must not change: {line}")
    return errs


def check_all(root: Path, base: str | None = None) -> list[str]:
    errs: list[str] = []
    top = root / "standards"
    if not top.is_dir():
        return [f"{top}: no standards directory"]
    seen_area: dict[str, str] = {}
    for std in sorted(p for p in top.iterdir() if p.is_dir()):
        vs = versions(std)
        if not vs:
            errs.append(f"{std}: no version folders")
            continue
        changelog = std / "CHANGELOG.md"
        text = changelog.read_text(encoding="utf-8") if changelog.is_file() else ""
        if not text:
            errs.append(f"{changelog}: missing or empty")
        previous: dict[str, dict] = {}
        for v in vs:
            folder = std / v
            req = folder / "requirements.toml"
            if not req.is_file():
                errs.append(f"{req}: missing")
                continue
            e, recs = check_records(v, req, vs)
            errs += e
            if not re.search(rf"^##\s+\[?{re.escape(v)}\]?(\s|$)", text, re.M):
                errs.append(f"{changelog}: no '## {v}' section")
            for rid, rec in recs.items():
                key = f"{std.name}/{rid}"
                if seen_area.setdefault(key, str(rec.get("area"))) != rec.get("area"):
                    errs.append(f"{req} {rid}: an id never changes area")
            for rid in previous:
                if rid not in recs:
                    errs.append(f"{req}: {rid} from an earlier version is gone; mark it withdrawn_in instead of deleting")
            previous = recs
            errs += check_released(folder)
    if base:
        errs += check_released_unchanged(root, base)
    return errs


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--root", default=".", type=Path)
    ap.add_argument("--base", default=None, help="git ref to compare released folders against")
    args = ap.parse_args(argv)
    errs = check_all(args.root.resolve(), args.base)
    for e in errs:
        print(f"error: {e}")
    if errs:
        return 1
    print("standards: ok")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
