#!/usr/bin/env python3
"""Hardware design checker (HW-001 .. HW-028).

Runs the script-checkable rules of hardware/p1/method.md against the
machine-readable design in hardware/<stage>/data/{parts,netlist,safety_paths}.toml.
Python 3.11+, standard library only.

    python3 -I ci/check_hardware.py [--data DIR]... [--json]

Exit status 1 if any rule FAILs. WARN and INFO never fail the run.
A reviewed exception to a rule is written in this file, not in the data, so
changing one is a guard change (ci/ is a protected path, see CONTRIBUTING.md).
Issue numbers in comments (#N) are the findings the rule exists for; the
matching tests are the test_regression_<N>_* tests in ci/test_check_hardware.py.
"""
from __future__ import annotations

import argparse
import fnmatch
import json
import re
import sys
import tomllib
from pathlib import Path

JOINTS = {
    "soldered-to-board", "wired-and-soldered", "screw-terminal", "dupont",
    "crimp-spade", "barrel", "usb", "ribbon", "panel-terminal",
}
VARIANTS = ("A", "B", "both")
EXIT_TESTS = (1, 2, 3, 4, 5, 6)  # the six P1 exit tests (#279)
STATES = ("power_up", "reset", "brownout", "unpowered", "wire_off")
LOAD_STATES = {"off", "off-detected", "on-bounded", "on"}
MARGIN_V = 0.3        # logic level margin (method.md C-04)
BUDGET = 0.8          # share of a source's rating that may be used (C-09)
BACKFEED_R_OHM = 1000  # series resistor that makes an unpowered back-feed safe (C-07)

# Fault rows that may carry verdict FAIL / load_state "on". Each needs a reason.
# Changing this table needs the guards-change label and the clean-room reviewer.
FAULT_FAIL_ALLOWLIST = {
    "A28": "Tool ground clipped to a switched lamp node bypasses the low-side MOSFET. "
           "Handled by procedure (analyzer ground to the star point only, HW-023), not by the design.",
}

RULE_NAMES = {
    "HW-000": "design files load",
    "HW-001": "every connection has an allowed joint type",
    "HW-002": "ids are unique and every endpoint exists",
    "HW-003": "paths, series elements and fault rows reference real things",
    "HW-004": "nets, nodes and domains are declared",
    "HW-005": "logic high level (driver VOH vs receiver VIH)",
    "HW-006": "logic low level (driver VOL vs receiver VIL)",
    "HW-007": "input over-voltage (driver high vs receiver absolute max)",
    "HW-008": "unpowered back-feed between supply groups",
    "HW-009": "pin drive current",
    "HW-010": "rail current budget (80 % of the source)",
    "HW-011": "fuse rating does not exceed the wire ampacity",
    "HW-012": "every converter input is behind a fuse",
    "HW-013": "every buck output has a crowbar",
    "HW-014": "I2C addresses are unique per bus",
    "HW-015": "isolation domains are crossed only through isolators or reviewed bridges",
    "HW-016": "fail-safe defaults: a series element is open in every default state",
    "HW-017": "no active-low module is driven directly by an MCU pin",
    "HW-018": "single-fault coverage: every series element has a fault row per variant",
    "HW-019": "fault verdicts and load states",
    "HW-020": "independent controllers in each path",
    "HW-021": "safety-path connections are observed by an exit test",
    "HW-022": "every exit test is observed; the analyzer host is powered in its tests",
    "HW-023": "analyzer probes within limits, analyzer ground on the star point",
    "HW-024": "no USB host port is used twice",
    "HW-025": "reserved, on-board and not-connected pins are unused",
    "HW-026": "trust root has no USB lead and only removable debug leads",
    "HW-027": "no-name parts and fuse bytes have a bench or datasheet check",
    "HW-028": "datasheet provenance (information only)",
}

POWER_RE = re.compile(r"\b(gnd|ground|vcc|supply|5v|3v3|vbus)\b", re.I)
GPIO_RE = re.compile(r"\b(GP\d+|GPIO\d+|PB\d+)\b")
ON_RE = re.compile(r"\b(energi[sz]\w*|held|closed|conduct\w*|on)\b", re.I)
OFF_RE = re.compile(r"\b(off|released?|open|de-?energi[sz]\w*|no output|stops?|tri-?state\w*)\b", re.I)


class Finding:
    def __init__(self, rule, status, message, ids=(), variants=("both",)):
        self.rule, self.status, self.message = rule, status, message
        self.ids = tuple(ids)
        self.variants = set(variants)

    def key(self):
        return (self.rule, self.status, self.message, self.ids)

    def variant_label(self):
        if "both" in self.variants or {"A", "B"} <= self.variants:
            return ""
        return "/".join(sorted(self.variants))

    def as_dict(self):
        return {"status": self.status, "message": self.message, "ids": list(self.ids),
                "variant": self.variant_label() or "all"}


class Model:
    def __init__(self, data):
        self.parts_list = data.get("parts", {}).get("part", [])
        self.nets_list = data.get("netlist", {}).get("net", [])
        self.nodes_list = data.get("netlist", {}).get("node", [])
        self.conns = data.get("netlist", {}).get("connection", [])
        self.paths = data.get("paths", {}).get("path", [])
        self.faults = data.get("paths", {}).get("fault", [])
        self.parts, self.nets, self.nodes = {}, {}, {}
        for p in self.parts_list:
            self.parts.setdefault(p.get("id"), p)
        for n in self.nets_list:
            self.nets.setdefault(n.get("name"), n)
        for n in self.nodes_list:
            self.nodes.setdefault(n.get("name"), n)

    def endpoint(self, ep):
        """('node', name) or ('pin', part_id, pin) or None if malformed."""
        if not isinstance(ep, str) or ":" not in ep:
            return None
        head, tail = ep.split(":", 1)
        if not head or not tail:
            return None
        return ("node", tail) if head == "node" else ("pin", head, tail)

    def part_of(self, ep):
        e = self.endpoint(ep)
        if e and e[0] == "pin":
            return self.parts.get(e[1])
        return None

    def pin_function(self, ep):
        e = self.endpoint(ep)
        if not e or e[0] != "pin":
            return ""
        part = self.parts.get(e[1]) or {}
        for pin in part.get("pins", []):
            if pin.get("name") == e[2]:
                return pin.get("function", "")
        return ""

    def pin_exists(self, ep):
        e = self.endpoint(ep)
        if not e:
            return False
        if e[0] == "node":
            return e[1] in self.nodes
        part = self.parts.get(e[1])
        return bool(part) and any(p.get("name") == e[2] for p in part.get("pins", []))

    def supply_domains(self, part):
        return {g for g in (part.get("power_group"), part.get("supply_out_group")) if g}

    def view(self, variant):
        if variant == "both":
            return [c for c in self.conns if c.get("variant", "both") == "both"]
        return [c for c in self.conns if c.get("variant", "both") in ("both", variant)]


class Report:
    def __init__(self):
        self.findings, self.checked = [], {}

    def add(self, rule, status, message, ids=(), variant="both"):
        self.findings.append(Finding(rule, status, message, ids, (variant,)))

    def count(self, rule, n=1, variant=None):
        self.checked[rule] = max(self.checked.get(rule, 0), n) if variant else self.checked.get(rule, 0) + n

    def merged(self):
        by = {}
        for f in self.findings:
            if f.key() in by:
                by[f.key()].variants |= f.variants
            else:
                by[f.key()] = f
        return list(by.values())


def is_power_pin(m, ep):
    e = m.endpoint(ep)
    if not e or e[0] != "pin":
        return False
    fn = m.pin_function(ep)
    return bool(POWER_RE.search(e[2]) or POWER_RE.search(fn) or e[2].upper().startswith("USB")
                or re.search(r"contact|shunt|coil|usb", fn, re.I))


def num(x):
    return isinstance(x, (int, float)) and not isinstance(x, bool)


def margin_status(margin):
    if margin < -1e-9:
        return "FAIL"
    if margin < MARGIN_V - 1e-9:
        return "WARN"
    return "PASS"


def path_matches(pattern, path_id):
    return fnmatch.fnmatchcase(path_id or "", pattern or "")


def tokens(text):
    return [t for t in re.split(r"[|,/\s]+", text or "") if t]


def classify_default(text):
    """'closed' if the text says the load can be energised, 'open' if it says off, else 'na'/'unknown'."""
    t = (text or "").strip()
    if not t or t.lower().startswith("n/a"):
        return "na"
    if ON_RE.search(t):
        return "closed"
    if OFF_RE.search(t):
        return "open"
    return "unknown"


# ---------------------------------------------------------------- rules on files and ids

def rule_structure(m, rep):
    for kind, items, key in (("part", m.parts_list, "id"), ("net", m.nets_list, "name"),
                             ("node", m.nodes_list, "name"), ("connection", m.conns, "id"),
                             ("path", m.paths, "id"), ("fault", m.faults, "id")):
        seen = set()
        for it in items:
            k = it.get(key)
            if not k:
                rep.add("HW-002", "FAIL", f"a {kind} has no {key}", ())
            elif k in seen:
                rep.add("HW-002", "FAIL", f"duplicate {kind} {key} {k}", (k,))
            seen.add(k)
        rep.count("HW-002", len(items))
    for c in m.conns:
        cid = c.get("id", "?")
        if c.get("variant", "both") not in VARIANTS:
            rep.add("HW-002", "FAIL", f"{cid}: variant {c.get('variant')!r} is not one of A, B, both", (cid,))
        for side in ("from", "to"):
            ep = c.get(side)
            e = m.endpoint(ep)
            if e is None:
                rep.add("HW-002", "FAIL", f"{cid}: {side} {ep!r} is not PART:PIN or node:NAME", (cid,))
            elif e[0] == "node" and e[1] not in m.nodes:
                rep.add("HW-002", "FAIL", f"{cid}: {side} node {e[1]} is not declared", (cid,))
            elif e[0] == "pin" and e[1] not in m.parts:
                rep.add("HW-002", "FAIL", f"{cid}: {side} part {e[1]} is not declared", (cid,))
            elif e[0] == "pin" and not m.pin_exists(ep):
                rep.add("HW-002", "FAIL", f"{cid}: part {e[1]} has no pin {e[2]} (not an edge pin, or not declared)", (cid,))
        rep.count("HW-002")


def rule_joints(m, rep, view, v):
    for c in view:
        j = c.get("joint")
        if j in JOINTS:
            pass
        elif isinstance(j, str) and re.sub(r"[\s_]+", "-", j.strip().lower()) in JOINTS:
            rep.add("HW-001", "FAIL", f"{c['id']}: joint {j!r} must be spelled {re.sub(r'[\s_]+', '-', j.strip().lower())!r}", (c["id"],), v)
        else:
            rep.add("HW-001", "FAIL", f"{c['id']}: joint {j!r} is missing or not one of {sorted(JOINTS)}", (c["id"],), v)
    rep.count("HW-001", len(view), v)


def rule_declared(m, rep, view, v):
    for n in m.nets_list:
        if not n.get("domain"):
            rep.add("HW-004", "FAIL", f"net {n.get('name')} has no domain", (n.get("name"),))
        if not n.get("isolation_domain"):
            rep.add("HW-004", "FAIL", f"net {n.get('name')} has no isolation_domain", (n.get("name"),))
    for n in m.nodes_list:
        if n.get("net") not in m.nets:
            rep.add("HW-004", "FAIL", f"node {n.get('name')} is on undeclared net {n.get('net')!r}", (n.get("name"),))
    for c in view:
        if c.get("net") not in m.nets:
            rep.add("HW-004", "FAIL", f"{c['id']}: net {c.get('net')!r} is not declared", (c["id"],), v)
        if not c.get("domain"):
            rep.add("HW-004", "WARN", f"{c['id']}: no voltage domain on the connection", (c["id"],), v)
    rep.count("HW-004", len(view), v)


def rule_paths(m, rep):
    for p in m.paths:
        pid = p.get("id", "?")
        if p.get("variant") not in VARIANTS:
            rep.add("HW-003", "FAIL", f"path {pid}: variant {p.get('variant')!r} is not one of A, B, both", (pid,))
        els = p.get("series_elements", [])
        if not els:
            rep.add("HW-003", "FAIL", f"path {pid} has no series elements", (pid,))
        seen = set()
        for e in els:
            eid = e.get("id", "?")
            if eid in seen:
                rep.add("HW-003", "FAIL", f"path {pid}: duplicate series element {eid}", (pid, eid))
            seen.add(eid)
            part = m.parts.get(e.get("part"))
            if part is None:
                rep.add("HW-003", "FAIL", f"path {pid}: series element {eid} references unknown part {e.get('part')!r}", (pid, eid))
                continue
            names = {x.get("name") for x in part.get("pins", [])}
            for pin in e.get("pins", []):
                if pin not in names:
                    rep.add("HW-003", "FAIL", f"path {pid}: series element {eid} lists pin {pin} that part {part['id']} does not have", (pid, eid))
            rep.count("HW-003")
    for f in m.faults:
        fid = f.get("id", "?")
        if f.get("variant") not in VARIANTS:
            rep.add("HW-003", "FAIL", f"fault {fid}: variant {f.get('variant')!r} is not one of A, B, both", (fid,))
        if not any(path_matches(f.get("path"), p.get("id")) for p in m.paths):
            rep.add("HW-003", "FAIL", f"fault {fid}: path {f.get('path')!r} matches no path", (fid,))
        rep.count("HW-003")


# ---------------------------------------------------------------- electrical rules on connections

def driver_receiver_pairs(m, view):
    """(connection, driver part, receiver part) for connections between two ordinary parts."""
    for c in view:
        if m.endpoint(c.get("from")) is None or m.endpoint(c.get("to")) is None:
            continue
        d, r = m.part_of(c.get("from")), m.part_of(c.get("to"))
        if not d or not r or d.get("kind") == "analyzer" or r.get("kind") == "analyzer":
            continue
        if m.nets.get(c.get("net"), {}).get("domain") == "GND":
            continue
        if is_power_pin(m, c["from"]) or is_power_pin(m, c["to"]):
            continue
        yield c, d, r


def rule_levels(m, rep, view, v):
    n5 = n6 = 0
    for c, d, r in driver_receiver_pairs(m, view):
        vih = r.get("vih_min_v")
        if vih is None and num(r.get("vih_frac")) and num(r.get("logic_v")):
            vih = r["vih_frac"] * r["logic_v"]
        if num(d.get("voh_min_v")) and num(vih):
            n5 += 1
            mg = d["voh_min_v"] - vih
            st = margin_status(mg)
            if st != "PASS":
                rep.add("HW-005", st, f"{c['id']}: {d['id']} VOH {d['voh_min_v']} V vs {r['id']} VIH {vih:.2f} V leaves {mg:+.2f} V (margin {MARGIN_V} V)", (c["id"],), v)
        vil = r.get("vil_max_v")
        if vil is None and num(r.get("vil_frac")) and num(r.get("logic_v")):
            vil = r["vil_frac"] * r["logic_v"]
        if num(d.get("vol_max_v")) and num(vil):
            n6 += 1
            mg = vil - d["vol_max_v"]
            st = margin_status(mg)
            if st != "PASS":
                rep.add("HW-006", st, f"{c['id']}: {d['id']} VOL {d['vol_max_v']} V vs {r['id']} VIL {vil:.2f} V leaves {mg:+.2f} V (margin {MARGIN_V} V)", (c["id"],), v)
    rep.count("HW-005", n5, v)
    rep.count("HW-006", n6, v)


def rule_overvoltage(m, rep, view, v):
    n = 0
    for c, d, r in driver_receiver_pairs(m, view):
        amax = r.get("abs_max_input_v")
        if not num(amax):
            continue
        highs = [x for x in (m.nets.get(c.get("net"), {}).get("max_v"), d.get("logic_v")) if num(x)]
        if not highs:
            continue
        n += 1
        high = max(highs)
        if high > amax + 1e-9:
            rep.add("HW-007", "FAIL", f"{c['id']}: {d['id']} drives up to {high} V into {r['id']} (absolute max {amax} V)", (c["id"],), v)
        elif (r.get("tolerance_condition") == "powered" and num(r.get("logic_v")) and high > r["logic_v"] + 0.5
              and d.get("power_group") != r.get("power_group")):
            rep.add("HW-007", "WARN", f"{c['id']}: {r['id']} tolerates {high} V only while powered and {d['id']} is in another supply group", (c["id"],), v)
    rep.count("HW-007", n, v)


def rule_backfeed(m, rep, view, v):
    n = 0
    for c, d, r in driver_receiver_pairs(m, view):
        dd, rd = m.supply_domains(d), m.supply_domains(r)
        if not dd or not rd or dd & rd:
            continue
        n += 1
        if d.get("isolator") or r.get("isolator"):
            continue
        if not (num(c.get("series_r_ohm")) and c["series_r_ohm"] >= BACKFEED_R_OHM):
            rep.add("HW-008", "WARN", f"{c['id']}: {d['id']} and {r['id']} are in different supply groups with no series resistor of {BACKFEED_R_OHM} ohm or more", (c["id"],), v)
    rep.count("HW-008", n, v)


def rule_drive_current(m, rep, view, v):
    load = {}
    for c, d, r in driver_receiver_pairs(m, view):
        if num(d.get("io_max_ma")) and num(r.get("input_current_ma")):
            load.setdefault((d["id"], c["from"]), []).append((c["id"], r["input_current_ma"]))
    for (did, pin), items in load.items():
        total = sum(x[1] for x in items)
        cap = m.parts[did]["io_max_ma"]
        ratio = total / cap
        ids = [x[0] for x in items]
        if ratio > 1 + 1e-9:
            rep.add("HW-009", "FAIL", f"{pin} sources {total} mA into its receivers, over its {cap} mA rating", ids, v)
        elif ratio > BUDGET + 1e-9:
            rep.add("HW-009", "WARN", f"{pin} sources {total} mA, {ratio:.0%} of its {cap} mA rating", ids, v)
    rep.count("HW-009", len(load), v)


# ---------------------------------------------------------------- power rules

def rule_rail_budget(m, rep):
    groups = {}
    for p in m.parts_list:
        if num(p.get("supply_ma_max")) and p.get("power_group"):
            groups.setdefault(p["power_group"], [0.0, [], []])
            groups[p["power_group"]][0] += p["supply_ma_max"]
            groups[p["power_group"]][1].append(p["id"])
    for p in m.parts_list:
        g = p.get("supply_out_group") or p.get("power_group")
        if num(p.get("supply_out_ma")) and g in groups:
            groups[g][2].append((p["id"], p["supply_out_ma"]))
    for g, (total, users, sources) in sorted(groups.items()):
        # a part that both consumes and sources the same group is not its own load
        if not sources:
            rep.add("HW-010", "FAIL", f"supply group {g} draws up to {total:g} mA but no part declares supply_out_ma for it", users)
            continue
        cap = min(x[1] for x in sources)
        src = ", ".join(x[0] for x in sources)
        ratio = total / cap
        if ratio > 1 + 1e-9:
            rep.add("HW-010", "FAIL", f"supply group {g}: {total:g} mA is over the {cap:g} mA of {src}", users)
        elif ratio > BUDGET + 1e-9:
            rep.add("HW-010", "WARN", f"supply group {g}: {total:g} mA is {ratio:.0%} of the {cap:g} mA of {src}", users)
    rep.count("HW-010", len(groups))


def rule_fuse_wire(m, rep):
    amps = [(p["id"], p["ampacity_a"]) for p in m.parts_list
            if p.get("kind") in ("wire", "terminal") and num(p.get("ampacity_a"))]
    fuses = [p for p in m.parts_list if p.get("kind") == "fuse" and num(p.get("rating_a"))]
    for f in fuses:
        if not amps:
            rep.add("HW-011", "WARN", f"fuse {f['id']} is {f['rating_a']} A but no wire or terminal declares an ampacity", (f["id"],))
            continue
        lo = min(amps, key=lambda x: x[1])
        if f["rating_a"] > lo[1] + 1e-9:
            rep.add("HW-011", "FAIL", f"fuse {f['id']} is {f['rating_a']} A but {lo[0]} carries only {lo[1]} A", (f["id"], lo[0]))
    rep.count("HW-011", len(fuses))


def graph(view):
    adj = {}
    for c in view:
        a, b = c.get("from"), c.get("to")
        if isinstance(a, str) and isinstance(b, str):
            adj.setdefault(a, set()).add(b)
            adj.setdefault(b, set()).add(a)
    return adj


def converter_input_pin(p):
    for pin in p.get("pins", []):
        if re.fullmatch(r"IN\+?", pin.get("name", "")):
            return pin["name"]
    return None


def converter_pins(p, pattern):
    return [pin["name"] for pin in p.get("pins", []) if re.fullmatch(pattern, pin.get("name", ""))]


def reaches_source_unfused(m, adj, start):
    """Walk upstream from a converter input; (reached_source, crossed_fuse)."""
    seen, stack = {start}, [start]
    internal = {start}
    source = fuse = False
    while stack:
        ep = stack.pop()
        nxt = set(adj.get(ep, ()))
        e = m.endpoint(ep)
        part = m.parts.get(e[1]) if e and e[0] == "pin" else None
        if part is not None and ep not in internal:
            if part.get("kind") == "adapter":
                source = True
            elif part.get("kind") == "fuse":
                fuse = True
                nxt = set()  # a fuse is never crossed
            elif part.get("kind") in ("buck", "ldo") and re.fullmatch(r"OUT\+?", e[2]):
                ip = converter_input_pin(part)
                if ip:
                    nxt.add(f"{part['id']}:{ip}")
                    internal.add(f"{part['id']}:{ip}")
            else:
                nxt = set()  # other parts are dead ends
        elif part is not None and part.get("kind") == "fuse":
            fuse = True
            nxt = set()
        for n in nxt:
            if n not in seen:
                seen.add(n)
                stack.append(n)
    return source, fuse


def rule_converters(m, rep, view, v):
    adj = graph(view)
    n12 = n13 = 0
    for p in m.parts_list:
        if p.get("kind") not in ("buck", "ldo"):
            continue
        ip = converter_input_pin(p)
        if ip is None:
            continue
        n12 += 1
        start = f"{p['id']}:{ip}"
        if start not in adj:
            rep.add("HW-012", "WARN", f"{p['id']}: input {ip} is not connected in the netlist", (p["id"],), v)
        else:
            source, fuse = reaches_source_unfused(m, adj, start)
            if source:
                rep.add("HW-012", "FAIL", f"{p['id']} ({p['kind']}) is fed from the supply with no fuse in between: a shorted converter puts the supply voltage on everything it feeds", (p["id"],), v)
            elif not fuse:
                rep.add("HW-012", "WARN", f"{p['id']}: no fuse and no supply found upstream of the input", (p["id"],), v)
        if p.get("kind") == "buck":
            n13 += 1
            outs = converter_pins(p, r"OUT\+?")
            found = False
            for o in outs:
                seen, stack = {f"{p['id']}:{o}"}, [f"{p['id']}:{o}"]
                while stack:
                    ep = stack.pop()
                    for n in adj.get(ep, ()):
                        pp = m.part_of(n)
                        if pp is not None and pp.get("kind") == "crowbar":
                            found = True
                        if n not in seen and (pp is None):
                            seen.add(n)
                            stack.append(n)
            if not found:
                rep.add("HW-013", "FAIL", f"{p['id']}: no crowbar on the output rail, so a shorted buck is not forced to blow its fuse", (p["id"],), v)
    rep.count("HW-012", n12, v)
    rep.count("HW-013", n13, v)


# ---------------------------------------------------------------- buses, isolation, USB

def parse_addr(x):
    try:
        return int(str(x), 0)
    except ValueError:
        return None


def rule_i2c(m, rep, view, v):
    bus = {}
    for c in view:
        for a, b in ((c.get("from"), c.get("to")), (c.get("to"), c.get("from"))):
            part = m.part_of(a)
            e = m.endpoint(a)
            if part and "i2c" in part.get("interfaces", []) and e and e[2] == "SDA":
                bus.setdefault(b, []).append((part, c["id"]))
    for master, devs in bus.items():
        seen = {}
        for part, cid in devs:
            addr = parse_addr(part.get("i2c_addr"))
            if addr is None:
                rep.add("HW-014", "WARN", f"{part['id']} is on the bus from {master} with no usable i2c_addr", (part["id"],), v)
                continue
            if addr in seen and seen[addr] != part["id"]:
                rep.add("HW-014", "FAIL", f"{part['id']} and {seen[addr]} both use I2C address {addr:#04x} on the bus from {master}", (part["id"], seen[addr]), v)
            seen[addr] = part["id"]
            if "i2c_addr" not in part.get("verified_fields", []):
                rep.add("HW-014", "WARN", f"{part['id']}: I2C address {addr:#04x} is not verified against a datasheet or the part in hand", (part["id"],), v)
    rep.count("HW-014", len(bus), v)


def endpoint_domains(m, ep):
    e = m.endpoint(ep)
    if not e:
        return None
    if e[0] == "node":
        net = m.nets.get((m.nodes.get(e[1]) or {}).get("net"))
        return {net["isolation_domain"]} if net and net.get("isolation_domain") else None
    part = m.parts.get(e[1])
    if not part:
        return None
    if part.get("isolator") and part.get("isolator_domains"):
        return set(part["isolator_domains"])
    return {part["isolation_domain"]} if part.get("isolation_domain") else None


def rule_isolation(m, rep, view, v):
    for p in m.parts_list:
        if p.get("isolator") and len(p.get("isolator_domains", [])) < 2:
            rep.add("HW-015", "FAIL", f"{p['id']} is flagged isolator but does not declare two isolator_domains", (p["id"],), v)
    n = 0
    for c in view:
        a, b = endpoint_domains(m, c.get("from")), endpoint_domains(m, c.get("to"))
        if a is None or b is None:
            continue
        n += 1
        crossing = not (a & b)
        bridged = c.get("bridges_isolation") is True
        if crossing and not bridged:
            rep.add("HW-015", "FAIL", f"{c['id']}: joins {sorted(a)} to {sorted(b)} with no isolator on those domains and no bridges_isolation", (c["id"],), v)
        elif crossing and not str(c.get("reason", "")).strip():
            rep.add("HW-015", "FAIL", f"{c['id']}: bridges_isolation is set with no reason", (c["id"],), v)
        elif bridged and not crossing:
            rep.add("HW-015", "WARN", f"{c['id']}: bridges_isolation is set but the endpoints share a domain", (c["id"],), v)
    rep.count("HW-015", n, v)


def rule_usb_ports(m, rep, view, v):
    hosts = {p["id"] for p in m.parts_list if sum(1 for x in p.get("pins", []) if str(x.get("name", "")).startswith("USB")) >= 2}
    use = {}
    for c in view:
        for ep in (c.get("from"), c.get("to")):
            e = m.endpoint(ep)
            if e and e[0] == "pin" and e[1] in hosts and e[2].startswith("USB"):
                use.setdefault(ep, []).append(c["id"])
    for ep, ids in use.items():
        if len(ids) > 1:
            rep.add("HW-024", "FAIL", f"host port {ep} is used by {len(ids)} connections", ids, v)
    rep.count("HW-024", len(use), v)


def rule_pins(m, rep, view, v):
    n = 0
    for c in view:
        for ep in (c.get("from"), c.get("to")):
            if not m.pin_exists(ep) or m.endpoint(ep)[0] != "pin":
                continue
            n += 1
            fn = m.pin_function(ep)
            if re.search(r"reserved|on-board|not connected", fn, re.I):
                rep.add("HW-025", "FAIL", f"{c['id']}: {ep} is {fn!r} and must stay unused", (c["id"],), v)
    rep.count("HW-025", n, v)


def rule_trust_root(m, rep, view, v):
    n = 0
    for c in view:
        for ep in (c.get("from"), c.get("to")):
            part = m.part_of(ep)
            e = m.endpoint(ep)
            if not part or not part.get("trust_root") or e[0] != "pin":
                continue
            n += 1
            if e[2].upper().startswith("USB"):
                rep.add("HW-026", "FAIL", f"{c['id']}: the trust root {part['id']} has a USB connection: BOOT+USB bypasses the firmware jumper and USB back-feeds its 5 V pin", (c["id"],), v)
            elif e[2].upper().startswith("SW") and c.get("removable") is not True:
                rep.add("HW-026", "FAIL", f"{c['id']}: SWD lead on {part['id']} is not removable (removable = true): SWD bypasses the firmware jumper", (c["id"],), v)
    rep.count("HW-026", n, v)


# ---------------------------------------------------------------- analyzer and exit tests

def analyzers(m):
    return {p["id"] for p in m.parts_list if p.get("kind") == "analyzer"}


def rule_probes(m, rep, view, v):
    an, n = analyzers(m), 0
    for c in view:
        for mine, other in ((c.get("from"), c.get("to")), (c.get("to"), c.get("from"))):
            e = m.endpoint(mine)
            if not e or e[0] != "pin" or e[1] not in an:
                continue
            part, oe = m.parts[e[1]], m.endpoint(other)
            if oe and oe[0] == "node":
                net = m.nets.get((m.nodes.get(oe[1]) or {}).get("net"))
            else:
                net = m.nets.get(c.get("net"))
            if e[2].upper().startswith("USB"):
                continue
            n += 1
            if net is None:
                continue
            if e[2].upper() == "GND":
                if not net.get("star_ground"):
                    rep.add("HW-023", "FAIL", f"{c['id']}: analyzer ground goes to net {net['name']}, not the star ground (a switched node would bypass its switch)", (c["id"],), v)
            elif num(part.get("abs_max_input_v")) and num(net.get("nominal_v")) and net["nominal_v"] > part["abs_max_input_v"] + 1e-9:
                rep.add("HW-023", "FAIL", f"{c['id']}: probe on net {net['name']} at {net['nominal_v']} V is over the analyzer's {part['abs_max_input_v']} V limit", (c["id"],), v)
    rep.count("HW-023", n, v)


def rule_exit_tests(m, rep):
    seen = set()
    for c in m.conns:
        ob = c.get("observed_by", [])
        for t in ob if isinstance(ob, list) else []:
            if t not in EXIT_TESTS:
                rep.add("HW-022", "FAIL", f"{c['id']}: observed_by names test {t!r}, not an exit test {EXIT_TESTS}", (c["id"],))
            else:
                seen.add(t)
    for t in EXIT_TESTS:
        if t not in seen:
            rep.add("HW-022", "WARN", f"exit test {t} is observed by no connection", ())
    an = analyzers(m)
    off_tests = {t for p in m.parts_list for t in p.get("off_in_tests", [])}
    hosted = {}
    for c in m.conns:
        for a, b in ((c.get("from"), c.get("to")), (c.get("to"), c.get("from"))):
            ea, eb = m.endpoint(a), m.endpoint(b)
            if ea and eb and ea[0] == "pin" and eb[0] == "pin" and eb[1] in an and eb[2].upper().startswith("USB") and ea[1] in m.parts:
                host = m.parts[ea[1]]
                for t in c.get("observed_by", []):
                    hosted.setdefault(t, []).append((host, c["id"]))
                    if t in host.get("off_in_tests", []):
                        rep.add("HW-022", "FAIL", f"{c['id']}: exit test {t} is observed through analyzer host {host['id']}, which is powered off in that test", (c["id"],))
    for t in sorted(off_tests):
        if not any(t not in h.get("off_in_tests", []) for h, _ in hosted.get(t, [])):
            rep.add("HW-022", "FAIL", f"exit test {t} runs with a host powered off and no analyzer host link on another machine is observed by it", ())
    rep.count("HW-022", len(EXIT_TESTS))


def rule_safety_observed(m, rep, view, v):
    n = 0
    for p in m.paths:
        for e in p.get("series_elements", []):
            for pin in e.get("pins", []):
                ep = f"{e.get('part')}:{pin}"
                touching = [c for c in view if ep in (c.get("from"), c.get("to"))]
                if not touching:
                    rep.add("HW-021", "WARN", f"path {p['id']}: series pin {ep} ({e['id']}) appears in no connection", (p["id"], e["id"]), v)
                for c in touching:
                    n += 1
                    ob = c.get("observed_by")
                    if not (isinstance(ob, list) and any(t in EXIT_TESTS for t in ob)):
                        rep.add("HW-021", "FAIL", f"{c['id']}: on the safety path {p['id']} ({e['id']}) but observed_by names no exit test", (c["id"], p["id"]), v)
    rep.count("HW-021", n, v)


# ---------------------------------------------------------------- safety path rules

def rule_failsafe(m, rep):
    for p in m.paths:
        els = [e for e in p.get("series_elements", []) if m.parts.get(e.get("part"), {}).get("default_state")]
        for s in STATES:
            cls = {e["id"]: classify_default(m.parts[e["part"]]["default_state"].get(s, "")) for e in els}
            if "open" not in cls.values():
                rep.add("HW-016", "FAIL", f"path {p['id']}: no series element is open in state {s} ({cls or 'no element has a default_state'})", (p["id"],))
        rep.count("HW-016", len(STATES))


def rule_active_low(m, rep):
    n = 0
    for part in m.parts_list:
        if not str(part.get("trigger_polarity", "")).lower().startswith("active-low"):
            continue
        n += 1
        used_in = [f"{p['id']}/{e['id']}" for p in m.paths for e in p.get("series_elements", []) if e.get("part") == part["id"]]
        for c in m.conns:
            for mine, other in ((c.get("from"), c.get("to")), (c.get("to"), c.get("from"))):
                e, o = m.endpoint(mine), m.endpoint(other)
                if not e or not o or e[0] != "pin" or e[1] != part["id"] or o[0] != "pin" or o[1] == part["id"]:
                    continue
                drv = m.parts.get(o[1])
                if drv and "gpio" in drv.get("interfaces", []) and (GPIO_RE.search(o[2]) or GPIO_RE.search(m.pin_function(other))):
                    rep.add("HW-017", "FAIL",
                            f"{c['id']}: active-low {part['id']} input {e[2]} is driven directly by MCU pin {other}: the firmware's off level (low) energises it, and an unpowered MCU can back-feed it"
                            + (f" (series elements {', '.join(used_in)})" if used_in else ""), (c["id"], part["id"]),
                            variant=c.get("variant", "both"))
    rep.count("HW-017", n)


def covering_rows(m, path, element):
    need = [path.get("variant")] if path.get("variant") in ("A", "B") else ["A", "B"]
    out = []
    for v in need:
        rows = [f for f in m.faults
                if path_matches(f.get("path"), path.get("id")) and f.get("variant") in (v, "both")
                and ({element.get("id"), element.get("part")} & set(tokens(f.get("element"))))]
        out.append(rows)
    return out


def rule_fault_coverage(m, rep):
    n = 0
    for p in m.paths:
        for e in p.get("series_elements", []):
            n += 1
            for v, rows in zip(([p.get("variant")] if p.get("variant") in ("A", "B") else ["A", "B"]), covering_rows(m, p, e)):
                if not rows:
                    rep.add("HW-018", "FAIL", f"path {p['id']}: series element {e['id']} has no single-fault row for variant {v}", (p["id"], e["id"]), v)
    rep.count("HW-018", n)


def rule_fault_rows(m, rep):
    for f in m.faults:
        fid, ls, vd = f.get("id", "?"), f.get("load_state"), f.get("verdict")
        allowed = fid in FAULT_FAIL_ALLOWLIST
        if ls not in LOAD_STATES:
            rep.add("HW-019", "FAIL", f"fault {fid}: load_state {ls!r} is not one of {sorted(LOAD_STATES)}", (fid,))
        elif ls == "on" and not allowed:
            rep.add("HW-019", "FAIL", f"fault {fid} ({f.get('fault')}): the load stays on", (fid,))
        elif ls == "on-bounded":
            if not num(f.get("bound_ms")):
                rep.add("HW-019", "FAIL", f"fault {fid}: on-bounded needs bound_ms", (fid,))
            else:
                rep.add("HW-019", "WARN", f"fault {fid} ({f.get('fault')}): load on for up to {f['bound_ms']} ms", (fid,))
        if vd not in ("PASS", "WARN", "FAIL"):
            rep.add("HW-019", "FAIL", f"fault {fid}: verdict {vd!r} is not PASS, WARN or FAIL", (fid,))
        elif vd == "FAIL":
            if allowed:
                rep.add("HW-019", "WARN", f"fault {fid} is an allow-listed FAIL: {FAULT_FAIL_ALLOWLIST[fid]}", (fid,))
            else:
                rep.add("HW-019", "FAIL", f"fault {fid} ({f.get('fault')}) has verdict FAIL and is not on the allow-list", (fid,))
        elif vd == "WARN":
            rep.add("HW-019", "WARN", f"fault {fid} ({f.get('fault')}) is a documented WARN", (fid,))
    for fid in FAULT_FAIL_ALLOWLIST:
        row = next((f for f in m.faults if f.get("id") == fid), None)
        if row is not None and row.get("verdict") != "FAIL":
            rep.add("HW-019", "WARN", f"allow-list entry {fid} no longer matches a FAIL row", (fid,))
    rep.count("HW-019", len(m.faults))


def controllers(m, text):
    return {t for t in re.findall(r"[A-Za-z0-9_]+", text or "") if t in m.parts}


def rule_independence(m, rep):
    for p in m.paths:
        els = [e for e in p.get("series_elements", []) if e.get("controlled_by") not in (None, "", "human", "none")]
        ctl = {e["id"]: controllers(m, e["controlled_by"]) or {e["controlled_by"]} for e in els}
        pairs = [(a, b) for i, a in enumerate(els) for b in els[i + 1:] if not (ctl[a["id"]] & ctl[b["id"]])]
        if not pairs:
            rep.add("HW-020", "FAIL", f"path {p['id']}: no two controlled series elements have independent controllers", (p["id"],))
            continue
        for a, b in pairs:
            shared = [k for k in ("board", "supply_net") if a.get(k) and a.get(k) == b.get(k)]
            if shared:
                rep.add("HW-020", "WARN", f"path {p['id']}: {a['id']} and {b['id']} have independent controllers but share {' and '.join(shared)} (common cause)", (p["id"], a["id"], b["id"]))
        rep.count("HW-020")


def rule_acceptance(m, rep):
    for p in m.parts_list:
        if p.get("counterfeit_risk") and p.get("acceptance_passed") is not True:
            rep.add("HW-027", "WARN", f"{p['id']}: no-name part with counterfeit risk, no bench acceptance result recorded", (p["id"],))
        fb = p.get("fuse_bytes")
        if isinstance(fb, dict) and fb.get("datasheet_checked") is not True:
            rep.add("HW-027", "WARN", f"{p['id']}: fuse bytes {', '.join(f'{k}={v}' for k, v in fb.items() if k != 'datasheet_checked')} are not yet checked against the datasheet", (p["id"],))
    rep.count("HW-027", len(m.parts_list))


def rule_provenance(m, rep):
    ps = m.parts_list
    ver = sum(1 for p in ps if p.get("verified") is True)
    rep.add("HW-028", "INFO", f"{ver} of {len(ps)} parts ({(100 * ver / len(ps)) if ps else 0:.0f} %) have at least one field read from a datasheet or maker page")
    rep.count("HW-028", len(ps))


# ---------------------------------------------------------------- driver

PER_VIEW = (rule_joints, rule_declared, rule_levels, rule_overvoltage, rule_backfeed, rule_drive_current,
            rule_converters, rule_i2c, rule_isolation, rule_usb_ports, rule_pins, rule_trust_root,
            rule_probes, rule_safety_observed)
GLOBAL = (rule_structure, rule_paths, rule_rail_budget, rule_fuse_wire, rule_exit_tests, rule_failsafe,
          rule_active_low, rule_fault_coverage, rule_fault_rows, rule_independence, rule_acceptance,
          rule_provenance)


def check(data):
    m = Model(data)
    rep = Report()
    for fn in GLOBAL:
        fn(m, rep)
    for v in ("both", "A", "B"):
        view = m.view(v)
        for fn in PER_VIEW:
            fn(m, rep, view, v)
    return summarise(rep)


def load(dirs):
    data, problems = {}, []
    for name, key in (("parts.toml", "parts"), ("netlist.toml", "netlist"), ("safety_paths.toml", "paths")):
        try:
            data[key] = tomllib.loads((Path(dirs) / name).read_text(encoding="utf-8"))
        except (OSError, tomllib.TOMLDecodeError) as exc:
            problems.append(f"{Path(dirs) / name}: {exc}")
            data[key] = {}
    return data, problems


def summarise(rep):
    order = {"FAIL": 0, "WARN": 1, "INFO": 2, "PASS": 3}
    rules = []
    by = {}
    for f in rep.merged():
        by.setdefault(f.rule, []).append(f)
    for rid in sorted(RULE_NAMES):
        fs = sorted(by.get(rid, []), key=lambda f: (order[f.status], f.message))
        status = fs[0].status if fs else "PASS"
        if status == "INFO":
            status = "INFO"
        rules.append({"id": rid, "name": RULE_NAMES[rid], "status": status,
                      "checked": rep.checked.get(rid, 0), "findings": fs})
    return rules


def result_of(rules):
    return "FAIL" if any(r["status"] == "FAIL" for r in rules) else "PASS"


def render(rules, label=""):
    lines = []
    if label:
        lines.append(f"== {label}")
    for r in rules:
        lines.append(f"{r['id']} {r['status']:<4} {r['name']} ({r['checked']} checked)")
        for f in r["findings"]:
            if f.status == "PASS":
                continue
            tag = f.variant_label()
            lines.append(f"       {f.status}: {f.message}" + (f" [variant {tag}]" if tag else ""))
    counts = {s: sum(1 for r in rules if r["status"] == s) for s in ("PASS", "WARN", "FAIL", "INFO")}
    lines.append(f"RESULT {result_of(rules)}: {counts['PASS']} PASS, {counts['WARN']} WARN, {counts['FAIL']} FAIL, {counts['INFO']} INFO")
    return "\n".join(lines)


def jsonable(rules, label):
    return {"design": label, "result": result_of(rules),
            "rules": [{"id": r["id"], "name": r["name"], "status": r["status"], "checked": r["checked"],
                       "findings": [f.as_dict() for f in r["findings"] if f.status != "PASS"]} for r in rules]}


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--data", action="append", help="a hardware/<stage>/data directory (default: all under hardware/)")
    ap.add_argument("--json", action="store_true", help="machine-readable output")
    args = ap.parse_args(argv)
    root = Path(__file__).resolve().parent.parent
    dirs = [Path(d) for d in args.data] if args.data else sorted(root.glob("hardware/*/data"))
    if not dirs:
        print("no hardware/*/data directory found", file=sys.stderr)
        return 1
    failed, out = False, []
    for d in dirs:
        data, problems = load(d)
        if problems:
            rules = [{"id": "HW-000", "name": RULE_NAMES["HW-000"], "status": "FAIL", "checked": 3,
                      "findings": [Finding("HW-000", "FAIL", p) for p in problems]}]
        else:
            rules = check(data)
        failed |= result_of(rules) == "FAIL"
        out.append((str(d), rules))
    if args.json:
        print(json.dumps([jsonable(r, d) for d, r in out], indent=2))
    else:
        print("\n\n".join(render(r, d) for d, r in out))
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
