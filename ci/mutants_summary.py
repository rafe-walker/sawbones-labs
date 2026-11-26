#!/usr/bin/env python3
"""Per-crate cargo-mutants summary table (#220).

Usage: ci/mutants_summary.py DIR

DIR holds one sub-directory per shard, each a copy of cargo-mutants'
`mutants.out` named `<crate>--<shard>`. cargo-mutants writes caught.txt,
missed.txt, timeout.txt and unviable.txt as it goes, so a shard that hit its
time box still counts what it finished. Prints markdown: counts per crate,
then the missed mutants (candidate test gaps). Informational: always exits 0.
"""
import sys
from collections import defaultdict
from pathlib import Path

KINDS = ("caught", "missed", "timeout", "unviable")


def lines(path):
    if not path.is_file():
        return []
    return [l.strip() for l in path.read_text().splitlines() if l.strip()]


def main():
    root = Path(sys.argv[1])
    counts = defaultdict(lambda: dict.fromkeys(KINDS, 0))
    shards = defaultdict(int)
    missed = defaultdict(list)
    timeouts = defaultdict(list)
    for shard in sorted(p for p in root.iterdir() if p.is_dir()):
        crate = shard.name.split("--", 1)[0]
        shards[crate] += 1
        for kind in KINDS:
            found = lines(shard / f"{kind}.txt")
            counts[crate][kind] += len(found)
            if kind == "missed":
                missed[crate].extend(found)
            if kind == "timeout":
                timeouts[crate].extend(found)
    out = [
        "## Mutation testing (informational)",
        "",
        "| Crate | Shards | Caught | Missed | Timeout | Unviable | Caught of tested |",
        "| --- | ---: | ---: | ---: | ---: | ---: | ---: |",
    ]
    for crate in sorted(counts):
        c = counts[crate]
        tested = c["caught"] + c["missed"] + c["timeout"]
        score = f"{100.0 * c['caught'] / tested:.1f}%" if tested else "-"
        out.append(
            f"| `{crate}` | {shards[crate]} | {c['caught']} | {c['missed']} | "
            f"{c['timeout']} | {c['unviable']} | {score} |"
        )
    if not counts:
        out.append("| (no results) | | | | | | |")
    for title, table in (("Missed mutants (candidate test gaps)", missed), ("Timed-out mutants", timeouts)):
        out += ["", f"### {title}", ""]
        if not any(table.values()):
            out.append("None.")
        for crate in sorted(table):
            if table[crate]:
                out += [f"**`{crate}`** ({len(table[crate])})", ""]
                out += [f"- `{m}`" for m in sorted(table[crate])]
                out.append("")
    print("\n".join(out))


if __name__ == "__main__":
    main()
