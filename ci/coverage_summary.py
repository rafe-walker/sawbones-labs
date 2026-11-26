#!/usr/bin/env python3
"""Turn a cargo-llvm-cov JSON summary into a per-crate line-coverage markdown table."""
import json
import re
import sys
from collections import defaultdict

data = json.load(open(sys.argv[1]))["data"][0]
covered = defaultdict(int)
total = defaultdict(int)
for f in data["files"]:
    m = re.search(r"/crates/([^/]+)/", f["filename"])
    crate = m.group(1) if m else "(other)"
    lines = f["summary"]["lines"]
    covered[crate] += lines["covered"]
    total[crate] += lines["count"]

print("## Line coverage by crate (informational, no threshold)\n")
print("| Crate | Covered | Total | Lines |")
print("|---|---:|---:|---:|")
for c in sorted(total):
    pct = 100.0 * covered[c] / total[c] if total[c] else 0.0
    print(f"| {c} | {covered[c]} | {total[c]} | {pct:.1f}% |")
all_c, all_t = sum(covered.values()), sum(total.values())
print(f"| **workspace** | {all_c} | {all_t} | {100.0 * all_c / all_t if all_t else 0:.1f}% |")
