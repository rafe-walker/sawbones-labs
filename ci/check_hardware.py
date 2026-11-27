#!/usr/bin/env python3
"""Hardware design checker (HW-001 .. HW-038).

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
import math
import re
import sys
import tomllib
from pathlib import Path

JOINTS = {
    "soldered-to-board", "wired-and-soldered", "screw-terminal", "dupont",
    "crimp-spade", "barrel", "usb", "ribbon", "panel-terminal", "jst-xh",
}
UNKEYED_JOINTS = {"dupont"}  # a misplug can short a safety signal (independent review B5)
VARIANTS = ("A", "B", "both")
EXIT_TESTS = (1, 2, 3, 4, 5, 6)  # the six P1 exit tests (#279)
STATES = ("power_up", "reset", "brownout", "unpowered", "wire_off")
LOAD_STATES = {"off", "off-detected", "on-bounded", "on"}
MARGIN_V = 0.3        # logic level margin (method.md C-04)
BUDGET = 0.8          # share of a source's rating that may be used (C-09)
PUMP_MARGIN_V = 0.1   # hold voltage above the base turn-on level (HW-031)
LATCHES = {"hardware", "firmware"}   # HW-033: only a hardware latch makes a pulse output a permanent cut
SAFETY_DOMAINS = ("supply", "chain", "load")   # HW-034
TEXT_ON_RE = re.compile(r"\b(all held|re-?energi[sz]es|stays on|remains on|remain held)\b", re.I)  # HW-019 effect text that contradicts an off row
BACKFEED_R_OHM = 1000  # series resistor that makes an unpowered back-feed safe (C-07)

# Fault rows that may carry verdict FAIL / load_state "on". Each needs a reason.
# Changing this table needs the guards-change label and the clean-room reviewer.
FAULT_FAIL_ALLOWLIST = {
    "A39": "The RP2350 is the one decision-maker for K1, the MOSFET and (through the Q&A answers) K2: a stall that keeps its pump, its answers and GP12 alive "
           "holds every cut. Stated as a limit, not a pass (review 3, non-blocking 12; exit test 5g). Mitigation is firmware (no interrupts, PIO, PWM or DMA; "
           "one watchdog feed per pass) and the Pi heartbeat, neither of which is a hardware cut.",
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
    "HW-029": "rail clamp: voltage within every part's abs max, rating outlasts the fuse",
    "HW-030": "no AI-controlled host has a path to a trust-root debug or programming pin",
    "HW-031": "charge-pump hold voltage turns the driven stage on",
    "HW-032": "pull-down values respect the part's erratum limit",
    "HW-033": "a cut driven by a pulse output is latched in hardware, not assumed held",
    "HW-034": "no part joins nets in different safety domains unless it is a series element",
    "HW-035": "timing budgets are sums of their components and fit the bound",
    "HW-036": "an arm sense is not downstream of an unsensed human contact",
    "HW-037": "per-cut self-test expectations match the netlist topology",
    "HW-038": "no active output drives the reset of a controller whose debug pins are wired to another cut",
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
        self.pumps = data.get("paths", {}).get("pump", [])
        self.clamps = data.get("paths", {}).get("clamp", [])
        self.budgets = data.get("paths", {}).get("budget", [])
        self.senses = data.get("paths", {}).get("sense", [])
        self.selftests = data.get("paths", {}).get("selftest", [])
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
                        elif n not in seen and pp.get("kind") == "fuse":
                            seen.add(n)  # the crowbar may sit behind the buck's own output fuse
                            for x in pp.get("pins", []):
                                other = f"{pp['id']}:{x['name']}"
                                if other not in seen:
                                    seen.add(other)
                                    stack.append(other)
            if not found:
                rep.add("HW-013", "FAIL", f"{p['id']}: no crowbar on the output rail, so a shorted buck is not forced to blow its fuse", (p["id"],), v)
            else:
                crows = {pp["id"] for pp in m.parts_list if pp.get("kind") == "crowbar"}
                if crows and not any(c.get("part") in crows for c in m.clamps):
                    rep.add("HW-013", "FAIL", f"{p['id']}: the crowbar has no [[clamp]] record, so its voltage and rating are not checked (HW-029)", (p["id"],), v)
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
    for t in (2, 5):
        for h, cid in hosted.get(t, []):
            if h.get("ai_host"):
                rep.add("HW-022", "FAIL", f"{cid}: the evidence for exit test {t} is recorded by {h['id']}, an AI-controlled host: the proof would not be independent of the thing under test (review 3, B6)", (cid,))
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


def element_ids(p):
    return [e.get("id") for e in p.get("series_elements", [])]


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
        if ls in ("off", "off-detected"):
            rule_remaining_cuts(m, rep, f)
        elif ls == "on" and f.get("remaining_cuts"):
            rep.add("HW-019", "FAIL", f"fault {fid}: load_state is on but remaining_cuts {f['remaining_cuts']} are listed", (fid,))
        if vd not in ("PASS", "WARN", "FAIL"):
            rep.add("HW-019", "FAIL", f"fault {fid}: verdict {vd!r} is not PASS, WARN or FAIL", (fid,))
        elif vd == "FAIL":
            if allowed:
                rep.add("HW-019", "WARN", f"fault {fid} is an allow-listed FAIL: {FAULT_FAIL_ALLOWLIST[fid]}", (fid,))
            else:
                rep.add("HW-019", "FAIL", f"fault {fid} ({f.get('fault')}) has verdict FAIL and is not on the allow-list", (fid,))
        elif vd == "WARN":
            rep.add("HW-019", "WARN", f"fault {fid} ({f.get('fault')}) is a documented WARN", (fid,))
        elif vd == "PASS" and ls == "on":
            rep.add("HW-019", "FAIL", f"fault {fid}: verdict PASS with the load on", (fid,))
    for fid in FAULT_FAIL_ALLOWLIST:
        row = next((f for f in m.faults if f.get("id") == fid), None)
        if row is not None and (row.get("verdict") != "FAIL" or row.get("load_state") != "on"):
            rep.add("HW-019", "WARN", f"allow-list entry {fid} no longer matches a FAIL row with the load on", (fid,))
    rep.count("HW-019", len(m.faults))


def rule_remaining_cuts(m, rep, f):
    """An 'off' row names the cuts that still open the load. Each must exist on every path the row covers,
    must not be the faulted element, and for a row where the fault holds its controller on (fault_mode =
    holds) must not be controlled or fed by the faulted part (review 3, non-blocking 16)."""
    fid = f.get("id", "?")
    rc = f.get("remaining_cuts")
    if not isinstance(rc, list) or not rc:
        rep.add("HW-019", "FAIL", f"fault {fid}: load_state {f.get('load_state')} but no remaining_cuts: say which cut still opens the load", (fid,))
        return
    if TEXT_ON_RE.search(f.get("effect", "") or ""):
        rep.add("HW-019", "FAIL", f"fault {fid}: the effect text says the load is held or re-energised, but load_state is {f.get('load_state')}", (fid,))
    faulted = set(tokens(f.get("element")))
    holds = f.get("fault_mode") == "holds"
    for p in m.paths:
        if not path_matches(f.get("path"), p.get("id")):
            continue
        by_id = {e.get("id"): e for e in p.get("series_elements", [])}
        for pat in rc:
            hits = [i for i in by_id if fnmatch.fnmatch(i, pat)]
            if not hits:
                rep.add("HW-019", "FAIL", f"fault {fid}: remaining cut {pat!r} is not a series element of path {p['id']}", (fid, p["id"]))
            for i in hits:
                e = by_id[i]
                if len(faulted) == 1 and (i in faulted or e.get("part") in faulted):
                    rep.add("HW-019", "FAIL", f"fault {fid}: {i} is the faulted element and cannot also be a remaining cut", (fid, p["id"]))
                elif holds:
                    ctl = controllers(m, e.get("controlled_by", "")) | set(e.get("fed_by", []))
                    bad = ctl & faulted
                    if bad:
                        rep.add("HW-019", "FAIL", f"fault {fid}: {i} is named as a remaining cut but is controlled or fed by faulted part {', '.join(sorted(bad))}", (fid, p["id"]))


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
            if a.get("part") == b.get("part"):
                rep.add("HW-020", "FAIL", f"path {p['id']}: {a['id']} and {b['id']} have independent controllers but are on the same module {a['part']}: one slip on its header or one shorted input holds both cuts (common cause)", (p["id"], a["id"], b["id"]))
            elif a.get("board") and a.get("board") == b.get("board"):
                rep.add("HW-020", "FAIL", f"path {p['id']}: {a['id']} and {b['id']} share board {a['board']} (common cause)", (p["id"], a["id"], b["id"]))
            elif a.get("supply_net") and a.get("supply_net") == b.get("supply_net"):
                rep.add("HW-020", "WARN", f"path {p['id']}: {a['id']} and {b['id']} share supply {a['supply_net']}: safe only while the rail clamp (HW-029) holds", (p["id"], a["id"], b["id"]))
        fam = {}
        for e in els:
            for c in m.conns:
                for ep in (c.get("from"), c.get("to")):
                    if m.part_of(ep) is not None and m.part_of(ep)["id"] == e.get("part") and c.get("joint") in UNKEYED_JOINTS:
                        rep.add("HW-020", "FAIL", f"{c['id']}: {c['joint']} connector on {ep}, a signal of controlled element {e['id']} on path {p['id']}: use a keyed connector or a screw terminal", (c["id"], p["id"]), c.get("variant", "both"))
                    elif m.part_of(ep) is not None and m.part_of(ep)["id"] == e.get("part") and c.get("joint") == "jst-xh":
                        if not c.get("connector"):
                            rep.add("HW-020", "FAIL", f"{c['id']}: keyed connector on {ep} (element {e['id']}) names no connector family, so interchangeable plugs cannot be ruled out", (c["id"], e["id"]))
                        else:
                            fam.setdefault(e["id"], set()).add(c["connector"])
        for a, b in pairs:
            both = fam.get(a["id"], set()) & fam.get(b["id"], set())
            if both:
                rep.add("HW-020", "FAIL", f"path {p['id']}: independent cuts {a['id']} and {b['id']} use the same connector {sorted(both)[0]}: the plugs can be swapped and a swapped bench still looks right (review 3, B7)", (p["id"], a["id"], b["id"]))
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



def clamp_band(c):
    """Trip band (min, max volts) worked out from component values; (None, reason) if the data is incomplete."""
    trig = c.get("trigger")
    if trig == "zener":
        miss = [k for k in ("vz_min_v", "vz_max_v") if not num(c.get(k))]
        return (None, f"zener clamp needs {', '.join(miss)}") if miss else ((c["vz_min_v"], c["vz_max_v"]), "")
    if trig in ("tl431-gnd-pnp", "tl431-anode-gate"):
        need = ["r_top_ohm", "r_bot_ohm", "r_tol", "vref_min_v", "vref_max_v", "iref_max_ua", "vgt_max_v", "igt_max_ma", "r_gate_ohm"]
        if trig == "tl431-anode-gate":
            need += ["vgt_min_v", "igt_min_ma"]
        miss = [k for k in need if not num(c.get(k))]
        if miss:
            return None, f"{trig} clamp needs {', '.join(miss)}"
        t, rt, rb = c["r_tol"], c["r_top_ohm"], c["r_bot_ohm"]
        ratio_min = 1 + rt * (1 - t) / (rb * (1 + t))
        ratio_max = 1 + rt * (1 + t) / (rb * (1 - t))
        iref_v = c["iref_max_ua"] * 1e-6 * rt * (1 + t)
        if trig == "tl431-gnd-pnp":
            return (ratio_min * c["vref_min_v"], ratio_max * c["vref_max_v"] + iref_v), ""
        rg = c["r_gate_ohm"]
        lo = ratio_min * (c["vref_min_v"] + c["vgt_min_v"] + c["igt_min_ma"] * 1e-3 * rg)
        hi = ratio_max * (c["vref_max_v"] + c["vgt_max_v"] + c["igt_max_ma"] * 1e-3 * rg) + iref_v
        return (lo, hi), ""
    return None, f"trigger {trig!r} is not one of tl431-gnd-pnp, tl431-anode-gate, zener"


def fuse_clear_ms(fuse, amps):
    rating = fuse.get("rating_a")
    if not num(rating) or rating <= 0:
        return None
    usable = [r for r in sorted(fuse.get("clearing", []), key=lambda r: r["multiple"]) if r["multiple"] <= amps / rating + 1e-9]
    return usable[-1]["max_ms"] if usable else None


def clamp_sources(m, start):
    """Walk the netlist (not GND) from the clamp pin to every source: an adapter or the output of a converter.
    Returns [(source part, [fuses crossed])]."""
    adj = {}
    for c in m.conns:
        if m.nets.get(c.get("net"), {}).get("domain") == "GND":
            continue
        a, b = c.get("from"), c.get("to")
        if isinstance(a, str) and isinstance(b, str):
            adj.setdefault(a, set()).add(b)
            adj.setdefault(b, set()).add(a)
    found, seen = [], set()
    stack = [(start, (), False)]
    while stack:
        ep, fz, internal = stack.pop()
        key = (ep, tuple(f["id"] for f in fz), internal)
        if key in seen:
            continue
        seen.add(key)
        e = m.endpoint(ep)
        part = m.parts.get(e[1]) if e and e[0] == "pin" else None
        nxt = set(adj.get(ep, ()))
        if part is not None and ep != start:
            kind = part.get("kind")
            if kind == "fuse" and not internal:
                fz = fz + (part,)
                stack.extend((f"{part['id']}:{x['name']}", fz, True) for x in part.get("pins", []) if x["name"] != e[2])
                continue
            if kind == "fuse":
                pass
            elif kind == "adapter" or (kind == "buck" and re.fullmatch(r"OUT\+?", e[2])):
                found.append((part, list(fz)))
                continue
            else:
                continue
        for n in nxt:
            stack.append((n, fz, False))
    return found


def rule_clamps(m, rep):
    for c in m.clamps:
        cid = c.get("id", "?")
        part, net = m.parts.get(c.get("part")), m.nets.get(c.get("rail"))
        if part is None or net is None or not c.get("pin"):
            rep.add("HW-029", "FAIL", f"clamp {cid}: part {c.get('part')!r}, pin {c.get('pin')!r} or rail {c.get('rail')!r} is not declared", (cid,))
            continue
        band, why = clamp_band(c)
        if band is None:
            rep.add("HW-029", "FAIL", f"clamp {cid}: {why}: the trip band is worked out from component values, not typed in", (cid,))
            continue
        lo, hi = band
        for key, got in (("trip_min_v", lo), ("clamp_v_max", hi)):
            if not num(c.get(key)) or abs(c[key] - got) > 0.02:
                rep.add("HW-029", "FAIL", f"clamp {cid}: {key} is declared {c.get(key)} but the component values give {got:.2f} V ({lo:.2f} to {hi:.2f} V)", (cid,))
        if num(net.get("max_v")) and lo <= net["max_v"] + 1e-9:
            rep.add("HW-029", "FAIL", f"clamp {cid}: it can fire at {lo:.2f} V, inside the rail's normal range up to {net['max_v']} V", (cid,))
        if num(c.get("source_v")) and hi >= c["source_v"] - 1e-9:
            rep.add("HW-029", "FAIL", f"clamp {cid}: it may not fire below {hi:.1f} V, so a {c['source_v']:g} V fault on the rail may never trip it", (cid,))
        if c.get("trigger") == "tl431-gnd-pnp" and num(c.get("vce_sat_v")):
            ig = (lo - c["vce_sat_v"] - c["vgt_max_v"]) / c["r_gate_ohm"] * 1000.0
            if ig < 2 * c["igt_max_ma"]:
                rep.add("HW-029", "FAIL", f"clamp {cid}: gate drive at the lowest trip is {ig:.1f} mA, under twice the {c['igt_max_ma']} mA the SCR may need", (cid,))
        unknown = []
        for pid in c.get("protects", []):
            lim = (m.parts.get(pid) or {}).get("abs_max_supply_v")
            if not num(lim):
                unknown.append(pid)
            elif hi > lim + 1e-9:
                rep.add("HW-029", "FAIL", f"clamp {cid}: the rail can reach {hi:.2f} V before it acts, over the {lim} V absolute maximum of {pid}", (cid, pid))
        if unknown:
            rep.add("HW-029", "WARN", f"clamp {cid}: no abs_max_supply_v recorded for {', '.join(unknown)}; the clamp level is not checked against them", (cid,))
        if not num(c.get("rated_ms")) or c["rated_ms"] <= 0 or not num(c.get("rated_a")):
            rep.add("HW-029", "FAIL", f"clamp {cid}: rated_a and a positive rated_ms are required: a missing surge time cannot be waved through", (cid,))
            continue
        srcs = clamp_sources(m, f"{c['part']}:{c['pin']}")
        if not srcs:
            rep.add("HW-029", "FAIL", f"clamp {cid}: no source (adapter or converter output) is reachable from the clamp, so the fuse path cannot be checked", (cid,))
        for src, fuses in srcs:
            lo_a, hi_a = src.get("min_fault_a"), src.get("max_fault_a")
            if not num(lo_a) or not num(hi_a):
                rep.add("HW-029", "FAIL", f"clamp {cid}: source {src['id']} needs min_fault_a and max_fault_a (fold-back and hiccup set the low figure, the surge rating the high one)", (cid, src["id"]))
                continue
            if c["rated_a"] < hi_a - 1e-9:
                rep.add("HW-029", "FAIL", f"clamp {cid}: rated {c['rated_a']} A, below the {hi_a} A source {src['id']} can deliver", (cid, src["id"]))
            if not fuses:
                rep.add("HW-029", "FAIL", f"clamp {cid}: source {src['id']} reaches the clamped rail with no fuse in between: a live converter keeps feeding the fired crowbar and nothing blows", (cid, src["id"]))
                continue
            times = [(fuse_clear_ms(f, lo_a), f) for f in fuses]
            ok = [(t, f) for t, f in times if t is not None]
            if not ok:
                rep.add("HW-029", "FAIL", f"clamp {cid}: at {lo_a} A (source {src['id']}) no fuse on the path reaches a clearing multiple: the fault may never open a fuse", (cid, src["id"]))
                continue
            t, f = min(ok, key=lambda x: x[0])
            if t > c["rated_ms"]:
                rep.add("HW-029", "FAIL", f"clamp {cid}: rated for {c['rated_ms']} ms but {f['id']} can take up to {t} ms to clear at {lo_a} A from {src['id']}: the clamp can fail before the fuse blows", (cid, f["id"]))
    rep.count("HW-029", len(m.clamps))


def rule_ai_host_debug(m, rep, view, v):
    adj = {}
    for c in view:
        if m.nets.get(c.get("net"), {}).get("domain") == "GND":
            continue
        a, b = c.get("from"), c.get("to")
        if isinstance(a, str) and isinstance(b, str):
            adj.setdefault(a, set()).add(b)
            adj.setdefault(b, set()).add(a)
    hosts = [p for p in m.parts_list if p.get("ai_host")]
    for h in hosts:
        starts = [f"{h['id']}:{x['name']}" for x in h.get("pins", [])]
        seen, stack = set(starts), list(starts)
        while stack:
            ep = stack.pop()
            e = m.endpoint(ep)
            nxt = set(adj.get(ep, ()))
            part = m.parts.get(e[1]) if e and e[0] == "pin" else None
            if e and e[0] == "node" and (m.nodes.get(e[1]) or {}).get("drive_chain"):
                rep.add("HW-030", "FAIL", f"{h['id']} (AI-controlled host) reaches drive-chain node {e[1]}: a probe lead that can drive could energise a cut (review 3, B6)", (h["id"], e[1]), v)
            if part is not None and e[1] != h["id"]:
                if part.get("trust_root") and any(x.get("name") == e[2] and x.get("debug") for x in part.get("pins", [])):
                    rep.add("HW-030", "FAIL", f"{h['id']} (AI-controlled host) reaches debug or programming pin {ep} of trust-root part {part['id']}: code on the host could defeat every electronic cut", (h["id"], part["id"]), v)
                if any(x.get("name") == e[2] and x.get("drive") for x in part.get("pins", [])):
                    rep.add("HW-030", "FAIL", f"{h['id']} (AI-controlled host) reaches drive pin {ep} of {part['id']} (through a part that may drive its probes): it could energise a cut", (h["id"], part["id"]), v)
                if part.get("passthrough") or (part.get("kind") == "analyzer" and part.get("input_only_verified") is not True):
                    nxt |= {f"{part['id']}:{x['name']}" for x in part.get("pins", [])}
            for n in nxt:
                if n not in seen:
                    seen.add(n)
                    stack.append(n)
    rep.count("HW-030", len(hosts), v)


def sim_pump(P, k):
    """Event model of the charge pump at one corner of component values. Every rising edge of the drive pushes
    the series capacitor's charge into the hold capacitor; between edges the hold node decays through the bleed
    resistor and the base resistor. Returns (cycle-average hold voltage, hold-node function for dropout)."""
    T = P["toggle_period_us"] * 1e-6
    cs, ch = P["c_series_f"] * k["cs"], P["c_hold_f"] * k["ch"]
    rs, rb, r53 = P["r_series_ohm"] * k["rs"], P["r_bleed_ohm"] * k["rb"], P["r_base_ohm"] * k["r53"]
    vf, vpp, vbe, hfe = k["vf"], k["vpp"], k["vbe"], k["hfe"]
    amp = vpp - 2 * vf
    kk = 1 - math.exp(-T / (rs * cs))
    g1, g2 = 1 / rb, 1 / r53
    veq, tau = g2 * vbe / (g1 + g2), ch / (g1 + g2)

    def decay(v, t):
        if v > vbe and v > veq:
            t1 = tau * math.log((v - veq) / (vbe - veq)) if vbe > veq else float("inf")
            if t <= t1:
                return veq + (v - veq) * math.exp(-t / tau)
            return vbe * math.exp(-(t - t1) / (rb * ch))
        return v * math.exp(-t / (rb * ch))

    def inject(v):
        return v + cs * max(0.0, amp - v) * kk / ch

    v = 0.0
    n_warm = int(min(8000, max(60, 10 * ch * rb / (2 * T))))
    for _ in range(n_warm):
        v = decay(inject(v), 2 * T)
    samples = []
    for _ in range(4):
        v = inject(v)
        samples += [decay(v, 2 * T * i / 20) for i in range(20)]
        v = decay(v, 2 * T)
    v = inject(v)
    hold_avg = sum(samples) / len(samples)

    def drop_ms(hold_ma):
        vv, t, dt = v, 0.0, 1e-4
        while hfe * max(0.0, vv - vbe) / r53 * 1000.0 >= hold_ma and t < 5.0:
            vv = decay(vv, dt)
            t += dt
        return t * 1000.0
    return hold_avg, drop_ms


PUMP_FIELDS = ("toggle_period_us", "c_series_f", "c_series_tol", "r_series_ohm", "c_hold_f", "c_hold_tol_lo", "c_hold_tol_hi",
               "r_bleed_ohm", "r_base_ohm", "r_tol", "vpp_min_v", "vpp_max_v", "diode_vf_min_v", "diode_vf_max_v",
               "vbe_min_v", "vbe_max_v", "hfe_min", "hfe_max")


def pump_result(m, pu):
    """Worst-case numbers for one [[pump]]: hold voltage vs need, and the dropout window from the last edge."""
    relay = m.parts.get(pu.get("relay")) or {}
    miss = [f for f in PUMP_FIELDS if not num(pu.get(f))]
    hold_ma = relay.get("hold_ma")
    i_in = pu["load_ma"] if num(pu.get("load_ma")) else relay.get("input_current_ma")
    if miss or not num(i_in) or not (isinstance(hold_ma, list) and len(hold_ma) == 2):
        return None, "needs " + ", ".join(miss + ([] if num(i_in) else ["load_ma or the relay's input_current_ma"]) + ([] if isinstance(hold_ma, list) and len(hold_ma) == 2 else ["the relay's hold_ma [min, max]"]))
    ct, ht, rt = pu["c_series_tol"], (pu["c_hold_tol_lo"], pu["c_hold_tol_hi"]), pu["r_tol"]
    low = {"cs": 1 - ct, "ch": 1 - ht[0], "rs": 1 + rt, "rb": 1 - rt, "r53": 1 - rt, "vf": pu["diode_vf_max_v"], "vpp": pu["vpp_min_v"], "vbe": pu["vbe_max_v"], "hfe": pu["hfe_min"]}
    fast = dict(low)
    slow = {"cs": 1 + ct, "ch": 1 + ht[1], "rs": 1 - rt, "rb": 1 + rt, "r53": 1 + rt, "vf": pu["diode_vf_min_v"], "vpp": pu["vpp_max_v"], "vbe": pu["vbe_min_v"], "hfe": pu["hfe_max"]}
    hold, _ = sim_pump(pu, low)
    need = pu["vbe_max_v"] + pu["r_base_ohm"] * (1 + rt) * (i_in / 1000.0) / pu["hfe_min"] + PUMP_MARGIN_V
    _, dfast = sim_pump(pu, fast)
    _, dslow = sim_pump(pu, slow)
    half = pu["toggle_period_us"] / 1000.0
    return {"hold": hold, "need": need, "i_in": i_in, "freq": 1e6 / (2.0 * pu["toggle_period_us"]),
            "lo": max(0.0, dfast(hold_ma[1]) - half), "hi": dslow(hold_ma[0])}, ""


def rule_pumps(m, rep):
    for pu in m.pumps:
        pid = pu.get("id", "?")
        res, why = pump_result(m, pu)
        if res is None:
            rep.add("HW-031", "FAIL", f"pump {pid}: incomplete data, {why}", (pid,))
            continue
        if res["hold"] < res["need"]:
            rep.add("HW-031", "FAIL", f"pump {pid} ({pu.get('element')}): at {res['freq']:g} Hz and the worst corner of the component values the hold node averages {res['hold']:.2f} V, below the {res['need']:.2f} V needed to sink {res['i_in']:g} mA (base turn-on, hfe {pu['hfe_min']:g}, margin {PUMP_MARGIN_V} V): the relay may never pull in", (pid,))
        d = pu.get("dropout_ms")
        if not (isinstance(d, list) and len(d) == 2 and all(num(x) for x in d)):
            rep.add("HW-031", "FAIL", f"pump {pid}: dropout_ms [min, max] is not declared; the checker computes {res['lo']:.0f} to {res['hi']:.0f} ms", (pid,))
        elif d[0] > res["lo"] + 0.5 or d[1] < res["hi"] - 0.5:
            rep.add("HW-031", "FAIL", f"pump {pid}: declared dropout {d[0]:g} to {d[1]:g} ms does not cover the computed {res['lo']:.0f} to {res['hi']:.0f} ms (relay hold current to the last edge)", (pid,))
    rep.count("HW-031", len(m.pumps))


def rule_erratum_pulldowns(m, rep, view, v):
    n = 0
    for c in view:
        for ep in (c.get("from"), c.get("to")):
            part = m.part_of(ep)
            err = (part or {}).get("erratum")
            if not err or not num(err.get("max_pulldown_ohm")):
                continue
            pd = c.get("pulldown_ohm")
            if pd is not None:
                n += 1
                if num(pd) and pd > err["max_pulldown_ohm"] + 1e-9:
                    rep.add("HW-032", "FAIL", f"{c['id']}: {pd:g} ohm pull-down on {ep} is above the {err['max_pulldown_ohm']:g} ohm that overcomes erratum {err.get('id')} on stepping(s) {', '.join(err.get('affects_steppings', []))}", (c["id"],), v)
            elif c.get("safe_state") == "low":
                rep.add("HW-032", "WARN", f"{c['id']}: {ep} is read as safe when low but declares no pulldown_ohm (erratum {err.get('id')})", (c["id"],), v)
    rep.count("HW-032", n, v)


def rule_cut_semantics(m, rep):
    n = 0
    for p in m.paths:
        for e in p.get("series_elements", []):
            srcs = e.get("cut_sources")
            if not srcs:
                continue
            n += 1
            claim = e.get("cut_claim")
            for src in srcs:
                ep = m.endpoint(src)
                part = m.parts.get(ep[1]) if ep and ep[0] == "pin" else None
                sem = ((part or {}).get("output_semantics") or {}).get(ep[2] if ep else "")
                if part is None or sem is None:
                    rep.add("HW-033", "FAIL", f"path {p['id']}: cut source {src} of {e['id']} has no output_semantics in the part record", (p["id"], e["id"]))
                elif sem == "pulse" and claim == "held":
                    rep.add("HW-033", "FAIL", f"path {p['id']}: {e['id']} is assumed held off by {src}, but that output is a pulse (a watchdog timeout clears itself through MR and RESET releases after a delay)", (p["id"], e["id"]))
                elif sem == "pulse" and claim == "pulse-latched":
                    rule_latch(m, rep, p, e, src)
                elif claim not in ("held", "pulse-latched"):
                    rep.add("HW-033", "FAIL", f"path {p['id']}: {e['id']} cut_claim {claim!r} is not 'held' or 'pulse-latched'", (p["id"], e["id"]))
    rep.count("HW-033", n)


def rule_latch(m, rep, p, e, src):
    pid, eid = p["id"], e["id"]
    kind = e.get("latch")
    if kind not in LATCHES:
        rep.add("HW-033", "FAIL", f"path {pid}: {eid} latch {kind!r} is not one of {sorted(LATCHES)}", (pid, eid))
        return
    if kind == "firmware":
        rep.add("HW-033", "FAIL", f"path {pid}: {eid} relies on pulse output {src} and a firmware latch: the latch would sit in the decision-maker the cut is meant to be independent of (review 3, B4)", (pid, eid))
        return
    lp = m.parts.get(e.get("latch_part"))
    if lp is None:
        rep.add("HW-033", "FAIL", f"path {pid}: {eid} names latch_part {e.get('latch_part')!r}, which is not a part", (pid, eid))
        return
    others = set()
    for o in p.get("series_elements", []):
        if o is not e and o.get("controlled_by") not in (None, "", "human", "none"):
            others |= controllers(m, o["controlled_by"])
    mine = controllers(m, e.get("controlled_by", ""))
    ctl = controllers(m, lp.get("controlled_by", "")) | set(lp.get("fed_by", []))
    if lp.get("kind") != "latch":
        rep.add("HW-033", "FAIL", f"path {pid}: latch_part {lp['id']} is not kind 'latch'", (pid, eid))
    elif lp.get("clear_by") != "human":
        rep.add("HW-033", "FAIL", f"path {pid}: latch {lp['id']} is not cleared by a person (clear_by = {lp.get('clear_by')!r})", (pid, eid))
    elif not ctl or (ctl & (others - mine)):
        rep.add("HW-033", "FAIL", f"path {pid}: latch {lp['id']} is controlled or fed by {sorted(ctl & (others - mine)) or 'nothing declared'}: it must sit with this cut's own controller and not depend on another cut's", (pid, eid))
    elif not lp.get("board") or lp.get("board") != (m.parts.get(next(iter(mine), ""), {}) or {}).get("board"):
        rep.add("HW-033", "FAIL", f"path {pid}: latch {lp['id']} is on board {lp.get('board')!r}, not on the board of the cut's controller", (pid, eid))


def pump_by_ids(m, ids):
    ids = [ids] if isinstance(ids, str) else list(ids or [])
    return [pu for pu in m.pumps if pu.get("id") in ids], ids


def budget_sum(m, b):
    """Sum of the components of one budget: (total max ms, [(name, ms)], problem)."""
    parts, total = [], 0.0
    for comp in b.get("components", []):
        name = comp.get("name", "?")
        if num(comp.get("ms")):
            ms = comp["ms"]
        elif num(comp.get("periods")) and num(comp.get("period_ms")) and num(comp.get("tol")):
            ms = comp["periods"] * comp["period_ms"] * (1 + comp["tol"])
        elif "pump" in comp:
            pus, ids = pump_by_ids(m, comp["pump"])
            res = [pump_result(m, pu)[0] for pu in pus]
            if not pus or len(pus) != len(ids) or any(r is None for r in res):
                return None, parts, f"component {name!r} names pump(s) {ids} that are missing or incomplete"
            ms = max(r["hi"] for r in res)
        elif "relay" in comp:
            r = m.parts.get(comp["relay"]) or {}
            if not num(r.get("release_ms_max")):
                return None, parts, f"component {name!r}: relay {comp['relay']!r} has no release_ms_max"
            ms = r["release_ms_max"]
        else:
            return None, parts, f"component {name!r} has no ms, periods/period_ms/tol, pump or relay"
        parts.append((name, ms))
        total += ms
    return total, parts, ""


def rule_budgets(m, rep):
    sums = {}
    for b in m.budgets:
        bid = b.get("id", "?")
        if not num(b.get("bound_ms")) or not b.get("components"):
            rep.add("HW-035", "FAIL", f"budget {bid}: needs bound_ms and components", (bid,))
            continue
        total, parts, why = budget_sum(m, b)
        if total is None:
            rep.add("HW-035", "FAIL", f"budget {bid}: {why}", (bid,))
            continue
        sums[bid] = total
        if total > b["bound_ms"] + 1e-9:
            detail = " + ".join(f"{n} {ms:.1f}" for n, ms in parts)
            rep.add("HW-035", "FAIL", f"budget {bid} ({b.get('cut', '')}): {detail} = {total:.0f} ms, over the {b['bound_ms']:g} ms bound", (bid,))
    for f in m.faults:
        if not num(f.get("bound_ms")):
            continue
        fid, bid = f.get("id", "?"), f.get("budget")
        if bid not in sums:
            rep.add("HW-035", "FAIL", f"fault {fid}: bound_ms {f['bound_ms']:g} has no valid [[budget]] ({bid!r}): a timing bound is the sum of its parts, not a typed number", (fid,))
        elif f["bound_ms"] + 1e-9 < sums[bid]:
            rep.add("HW-035", "FAIL", f"fault {fid}: bound_ms {f['bound_ms']:g} is below the {sums[bid]:.0f} ms its budget {bid} adds up to", (fid,))
    rep.count("HW-035", len(m.budgets))


def ordered(p):
    return sorted(p.get("series_elements", []), key=lambda e: e.get("order", 0))


def rule_arm_sense(m, rep):
    n = 0
    sensed = {s.get("net") for s in m.senses}
    for s in m.senses:
        if s.get("function") != "arm":
            continue
        for p in m.paths:
            els = ordered(p)
            idx = next((i for i, e in enumerate(els) if s.get("net") in e.get("out_nets", [])), None)
            if idx is None:
                continue
            n += 1
            for e in els[:idx]:
                if e.get("controlled_by") == "human" and not (set(e.get("out_nets", [])) & sensed):
                    rep.add("HW-036", "FAIL", f"path {p['id']}: sense {s.get('id')} on {s['net']} (arm) is downstream of human contact {e['id']} whose output {e.get('out_nets')} is sensed nowhere: releasing {e['id']} looks like an arm cycle, so one sense fault re-arms (review 3, B5)", (p["id"], e["id"]))
    rep.count("HW-036", n)


def rule_selftest(m, rep):
    senses = {s.get("id"): s for s in m.senses}
    n = 0
    for p in m.paths:
        els = ordered(p)
        ids = [e["id"] for e in els]
        have = set()
        for t in m.selftests:
            if not path_matches(t.get("path"), p["id"]):
                continue
            tid = t.get("id", "?")
            if t.get("element") not in ids:
                rep.add("HW-037", "FAIL", f"self-test {tid}: element {t.get('element')!r} is not on path {p['id']}", (tid, p["id"]))
                continue
            have.add(t["element"])
            n += 1
            dead = set()
            for e in els[ids.index(t["element"]):]:
                dead |= set(e.get("out_nets", []))
            for sid, want in (t.get("expect") or {}).items():
                s = senses.get(sid)
                if s is None:
                    rep.add("HW-037", "FAIL", f"self-test {tid}: sense {sid!r} is not declared", (tid,))
                    continue
                got = "dead" if s.get("net") in dead else "live"
                if got != want:
                    rep.add("HW-037", "FAIL", f"self-test {tid} on {p['id']}: with {t['element']} open, {sid} ({s['net']}) is {got} in the netlist but the test expects {want}: the test would pass a wrong bench (review 3, B7)", (tid, p["id"]))
        for e in els:
            if e.get("controlled_by") not in (None, "", "human", "none") and e["id"] not in have:
                rep.add("HW-037", "FAIL", f"path {p['id']}: cut {e['id']} has no self-test: each cut must be proved to open on its own at arm time", (p["id"], e["id"]))
    rep.count("HW-037", n)


def rule_domains(m, rep):
    series_parts = {e.get("part") for p in m.paths for e in p.get("series_elements", [])}
    seen = {}
    for c in m.conns:
        d = (m.nets.get(c.get("net")) or {}).get("safety_domain")
        if d not in SAFETY_DOMAINS:
            continue
        for ep in (c.get("from"), c.get("to")):
            e = m.endpoint(ep)
            if e and e[0] == "pin":
                seen.setdefault(e[1], {}).setdefault(d, []).append(c.get("id"))
    for pid, ds in sorted(seen.items()):
        part = m.parts.get(pid) or {}
        if len(ds) > 1 and pid not in series_parts and not part.get("bridges_domains"):
            where = "; ".join(f"{d}: {', '.join(cs)}" for d, cs in sorted(ds.items()))
            rep.add("HW-034", "FAIL", f"part {pid} joins safety domains ({where}) but is not a series element: as one part in the graph it is a wire between them (review 3, N3)", (pid,))
    rep.count("HW-034", len(seen))


def rule_reset_write_path(m, rep, view, v):
    adj = {}

    def key(ep):
        e = m.endpoint(ep)
        return f"node:{e[1].split('.')[0]}" if e and e[0] == "node" and "." in e[1] else ep
    for c in view:
        if m.nets.get(c.get("net"), {}).get("domain") == "GND":
            continue
        a, b = c.get("from"), c.get("to")
        if isinstance(a, str) and isinstance(b, str):
            adj.setdefault(key(a), set()).add(key(b))
            adj.setdefault(key(b), set()).add(key(a))
    n = 0
    for part in m.parts_list:
        if not part.get("trust_root"):
            continue
        dbg = [x["name"] for x in part.get("pins", []) if x.get("debug")]
        resets = [x["name"] for x in part.get("pins", []) if x.get("debug") and re.search(r"reset", x.get("function", ""), re.I)]
        wired = [d for d in dbg if d not in resets and any(not (m.endpoint(k) or ("", part["id"]))[1] == part["id"] for k in adj.get(f"{part['id']}:{d}", ()))]
        for r in resets:
            n += 1
            start = f"{part['id']}:{r}"
            seen, stack = {start}, [start]
            drivers = set()
            while stack:
                ep = stack.pop()
                e = m.endpoint(ep)
                pp = m.parts.get(e[1]) if e and e[0] == "pin" else None
                if pp is not None and ep != start:
                    sem = pp.get("output_semantics") or {}
                    if e[2] in sem or m.pin_function(ep) in sem:
                        drivers.add(ep)
                    continue
                for nx in adj.get(ep, ()):
                    if nx not in seen:
                        seen.add(nx)
                        stack.append(nx)
            if drivers and wired:
                rep.add("HW-038", "FAIL", f"{part['id']}: output {', '.join(sorted(drivers))} can hold its RESET ({r}) low in a timed window while its programming pins {', '.join(wired)} are wired to another part: a compromised neighbour could erase and rewrite it (review 3, B4)", (part["id"],), v)
    rep.count("HW-038", n, v)


# ---------------------------------------------------------------- driver

PER_VIEW = (rule_joints, rule_declared, rule_levels, rule_overvoltage, rule_backfeed, rule_drive_current,
            rule_converters, rule_i2c, rule_isolation, rule_usb_ports, rule_pins, rule_trust_root,
            rule_probes, rule_safety_observed, rule_ai_host_debug, rule_erratum_pulldowns, rule_reset_write_path)
GLOBAL = (rule_structure, rule_paths, rule_rail_budget, rule_fuse_wire, rule_exit_tests, rule_failsafe,
          rule_active_low, rule_fault_coverage, rule_fault_rows, rule_independence, rule_acceptance,
          rule_provenance, rule_clamps, rule_pumps, rule_cut_semantics, rule_budgets, rule_arm_sense, rule_selftest,
          rule_domains)


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
