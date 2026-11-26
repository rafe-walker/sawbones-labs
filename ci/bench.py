#!/usr/bin/env python3
"""Instruction-count benchmarks with a checked-in baseline (#220).

Runs each case of `sawbones-bench` under `valgrind --tool=callgrind`, counts
the instructions (Ir) executed inside the measured function only, and
compares per-iteration Ir with ci/bench_baseline.json.

  ci/bench.py check            compare; exit 1 if any case is more than the
                               threshold above its baseline (default 10%)
  ci/bench.py bless            rewrite ci/bench_baseline.json from this run
  ci/bench.py check --bin PATH use a prebuilt binary (default: build release)

A markdown table goes to $GITHUB_STEP_SUMMARY when set, and to stdout.
Instruction counts are deterministic for one binary, but differ between
compilers, targets and libc versions: bless on the CI runner type (x86_64
ubuntu-latest), by taking the `bench-baseline` artifact of a CI run.
"""
import argparse
import json
import os
import re
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BASELINE = ROOT / "ci" / "bench_baseline.json"
ITERATIONS_RE = re.compile(r"pub const ITERATIONS: u32 = (\d+);")
THRESHOLD_PCT = 10.0
TOGGLE = "*sawbones_bench_measure*"


def build():
    subprocess.run(
        ["cargo", "build", "--release", "--locked", "-p", "sawbones-bench"],
        check=True, cwd=ROOT,
    )
    return ROOT / "target" / "release" / "sawbones-bench"


def cases(binary):
    out = subprocess.run([str(binary), "list"], check=True, capture_output=True, text=True)
    return out.stdout.split()


def iterations():
    src = (ROOT / "crates" / "sawbones-bench" / "src" / "main.rs").read_text()
    return int(ITERATIONS_RE.search(src).group(1))


def measure(binary, case, per):
    with tempfile.TemporaryDirectory() as tmp:
        out = Path(tmp) / "callgrind.out"
        subprocess.run(
            ["valgrind", "--tool=callgrind", f"--callgrind-out-file={out}",
             f"--toggle-collect={TOGGLE}", "--cache-sim=no", "--branch-sim=no",
             str(binary), "run", case],
            check=True, capture_output=True, text=True,
        )
        for line in out.read_text().splitlines():
            if line.startswith("totals:"):
                return int(line.split()[1]) // per
    raise SystemExit(f"no totals line in callgrind output for {case}")


def render(rows, threshold):
    lines = [
        "## Instruction-count benchmarks",
        "",
        f"Ir per iteration against `ci/bench_baseline.json`. A case fails above +{threshold:g}%.",
        "",
        "| Case | Baseline | Now | Change | Result |",
        "| --- | ---: | ---: | ---: | --- |",
    ]
    for name, base, now, pct, verdict in rows:
        b = "-" if base is None else f"{base:,}"
        c = "-" if pct is None else f"{pct:+.2f}%"
        lines.append(f"| `{name}` | {b} | {now:,} | {c} | {verdict} |")
    return "\n".join(lines) + "\n"


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("mode", choices=["check", "bless"])
    ap.add_argument("--bin")
    ap.add_argument("--threshold", type=float, help="percent; default: threshold_percent in the baseline")
    args = ap.parse_args()

    binary = Path(args.bin) if args.bin else build()
    per = iterations()
    now = {case: measure(binary, case, per) for case in cases(binary)}

    if args.mode == "bless":
        BASELINE.write_text(json.dumps(
            {"unit": "Ir per iteration", "threshold_percent": THRESHOLD_PCT, "cases": now},
            indent=2, sort_keys=True) + "\n")
        print(f"wrote {BASELINE.relative_to(ROOT)}")
        return 0

    data = json.loads(BASELINE.read_text())
    base = data["cases"]
    threshold = args.threshold if args.threshold is not None else data["threshold_percent"]
    rows, failed = [], False
    for name, value in sorted(now.items()):
        if name not in base:
            rows.append((name, None, value, None, "NEW: not in baseline (fail)"))
            failed = True
            continue
        pct = (value - base[name]) * 100.0 / base[name]
        if pct > threshold:
            verdict, failed = "REGRESSION", True
        elif pct < -threshold:
            verdict = "faster: re-bless"
        else:
            verdict = "ok"
        rows.append((name, base[name], value, pct, verdict))
    for name in sorted(set(base) - set(now)):
        rows.append((name, base[name], 0, None, "MISSING: case removed (fail)"))
        failed = True
    text = render(rows, threshold)
    print(text)
    summary = os.environ.get("GITHUB_STEP_SUMMARY")
    if summary:
        with open(summary, "a") as f:
            f.write(text)
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
