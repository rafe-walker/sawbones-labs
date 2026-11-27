#!/usr/bin/env python3
"""Tests for ci/check_hardware.py. Run: python3 -I -m unittest discover -s ci -p 'test_*.py'

A positive test on the real P1 design, a negative test for every rule built from a small
synthetic design, and the test_regression_<issue>_* tests for the findings of
hardware/p1/compatibility.md (#294 and the issues it links)."""
import contextlib
import copy
import io
import importlib.util
import json
import tempfile
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("check_hardware", HERE / "check_hardware.py")
ch = importlib.util.module_from_spec(spec)
spec.loader.exec_module(ch)

REAL = HERE.parent / "hardware" / "p1" / "data"


def pins(*names):
    return [{"name": n, "function": f} for n, f in names]


def baseline():
    """A small design that passes every rule: adapter, fuse, buck with crowbar, an MCU, a watchdog,
    a relay behind a driver node, a MOSFET module, a switch, an analyzer and a USB host."""
    parts = [
        {"id": "ADP", "kind": "adapter", "supply_out_ma": 3000.0, "max_fault_a": 5.0, "power_group": "12V", "isolation_domain": "G",
         "pins": pins(("+", "out"), ("-", "gnd"))},
        {"id": "FZ", "kind": "fuse", "rating_a": 1.0, "clearing": [{"multiple": 2.0, "max_ms": 5000}, {"multiple": 3.5, "max_ms": 500}, {"multiple": 6.0, "max_ms": 150}], "pins": pins(("A", "in"), ("B", "out"))},
        {"id": "BK", "kind": "buck", "supply_out_ma": 1000.0, "power_group": "5V",
         "pins": pins(("IN+", "in"), ("IN-", "gnd"), ("OUT+", "out"), ("OUT-", "gnd"))},
        {"id": "ZN", "kind": "crowbar", "pins": pins(("K", "5V rail"), ("A", "gnd"))},
        {"id": "WR", "kind": "wire", "ampacity_a": 3.0},
        {"id": "MCU", "interfaces": ["gpio"], "logic_v": 3.3, "voh_min_v": 2.8, "vol_max_v": 0.4, "vih_min_v": 2.0,
         "vil_max_v": 0.8, "io_max_ma": 4.0, "supply_ma_max": 40.0, "power_group": "5V",
         "supply_out_ma": 800.0, "supply_out_group": "3V3", "isolation_domain": "G",
         "abs_max_input_v": 5.5, "tolerance_condition": "powered", "abs_max_supply_v": 6.0,
         "erratum": {"id": "E9", "affects_steppings": ["A2"], "max_pulldown_ohm": 8200},
         "pins": pins(("GP1", "out"), ("GP2", "probe point"), ("GP16", "on-board LED (reserved)"),
                      ("USB", "usb"), ("SWCLK", "debug"))},
        {"id": "WD", "output_semantics": {"2": "pulse"}, "interfaces": ["gpio"], "logic_v": 3.3, "voh_min_v": 2.5, "vol_max_v": 0.5, "io_max_ma": 40.0,
         "power_group": "5V", "isolation_domain": "G", "pins": pins(("1", "PB4 toggle"), ("2", "PB3 pump"))},
        {"id": "REL", "input_current_ma": 3.0, "trigger_polarity": "active-low (IN low = energised)", "isolation_domain": "G",
         "default_state": {s: "released" for s in ch.STATES}, "interfaces": ["gpio"],
         "pins": pins(("IN1", "K1 drive"), ("K1.COM", "contact"), ("K1.NO", "contact"))},
        {"id": "MOSM", "trigger_polarity": "active-high", "vih_min_v": 2.0, "input_current_ma": 2.0,
         "power_group": "12V", "isolation_domain": "G", "interfaces": ["gpio"],
         "default_state": {s: "off" for s in ch.STATES},
         "pins": pins(("PWM", "logic in"), ("LOAD", "switched -"))},
        {"id": "SW", "isolation_domain": "G", "pins": pins(("1", "in"), ("2", "out"))},
        {"id": "AN", "kind": "analyzer", "abs_max_input_v": 5.25, "isolation_domain": "G",
         "pins": pins(("D0", "probe"), ("GND", "to the star only"), ("USB", "usb to host"))},
        {"id": "HOST", "isolation_domain": "G", "pins": pins(("USB1", "port"), ("USB2", "port"))},
    ]
    nets = [{"name": n, "domain": d, "nominal_v": v, "max_v": mx, "isolation_domain": "G", **({"star_ground": True} if n == "GND" else {})}
            for n, d, v, mx in (("GND", "GND", 0.0, 0.0), ("V12", "12V", 12.0, 13.0), ("V5", "5V", 5.0, 5.1),
                                ("LOG", "3V3", 3.3, 3.6), ("USB", "USB", 5.0, 5.25), ("LAMPN", "SWITCHED", 12.0, 13.0))]
    nodes = [{"name": "TBGND", "board": "P", "net": "GND"}, {"name": "TB12", "board": "P", "net": "V12"},
             {"name": "TB5", "board": "P", "net": "V5"}, {"name": "Q1.C", "board": "P", "net": "LOG"},
             {"name": "LAMPNODE", "board": "P", "net": "LAMPN"}]

    def conn(cid, a, b, net="LOG", joint="dupont", ob=(), **kw):
        return {"id": cid, "from": a, "to": b, "net": net, "domain": "x", "joint": joint, "variant": "both",
                "observed_by": list(ob), **kw}

    conns = [
        conn("C1", "ADP:+", "node:TB12", "V12", "barrel"),
        conn("C2", "node:TB12", "FZ:A", "V12", "wired-and-soldered"),
        conn("C3", "FZ:B", "BK:IN+", "V12", "screw-terminal"),
        conn("C4", "BK:OUT+", "node:TB5", "V5", "wired-and-soldered"),
        conn("C5", "ZN:K", "node:TB5", "V5", "soldered-to-board"),
        conn("C10", "node:TB12", "SW:1", "V12", "panel-terminal", [1, 2, 3]),
        conn("C11", "SW:2", "REL:K1.COM", "V12", "screw-terminal", [4]),
        conn("C12", "REL:K1.NO", "MOSM:LOAD", "V12", "screw-terminal", [5, 6]),
        conn("C13", "node:Q1.C", "REL:IN1", "V5", "jst-xh", [1]),
        conn("C14", "MCU:GP1", "MOSM:PWM", "LOG", "screw-terminal", [1], series_r_ohm=1000),
        conn("C20", "AN:D0", "MCU:GP2", "LOG", "dupont"),
        conn("C21", "AN:GND", "node:TBGND", "GND", "dupont"),
        conn("C22", "HOST:USB1", "AN:USB", "USB", "usb"),
    ]
    elements = [
        {"order": 1, "id": "SW", "part": "SW", "pins": ["1", "2"], "controlled_by": "human", "board": "PANEL", "supply_net": "V12"},
        {"order": 2, "id": "K", "part": "REL", "pins": ["K1.COM", "K1.NO"], "controlled_by": "MCU (pump)", "board": "RLY", "supply_net": "V5"},
        {"order": 3, "id": "Q", "part": "MOSM", "pins": ["LOAD"], "controlled_by": "WD (level)", "board": "M", "supply_net": "V12"},
    ]
    paths = [{"id": "PA", "load": "lamp", "variant": "A", "series_elements": copy.deepcopy(elements)},
             {"id": "PB", "load": "lamp", "variant": "B", "series_elements": copy.deepcopy(elements)}]
    faults = [{"id": f"F{i}", "path": "P?", "variant": "both", "fault": "x", "element": e, "load_state": "off",
               "verdict": "PASS"} for i, e in enumerate(("SW", "K", "Q"), 1)]
    pumps = [{"id": "PU", "element": "K", "driver": "MCU", "relay": "REL", "toggle_period_us": 10000, "vpp_v": 3.3,
              "c_series_f": 1e-6, "r_series_ohm": 470.0, "diode_vf_v": 0.25, "c_hold_f": 10e-6, "r_bleed_ohm": 47000.0,
              "r_base_ohm": 10000.0, "vbe_v": 0.6, "hfe_min": 70.0}]
    clamps = [{"id": "CL", "part": "ZN", "kind": "scr", "rail": "V5", "trip_min_v": 5.3, "clamp_v_max": 5.6, "rated_a": 12.0,
               "rated_ms": 0, "fuse": "FZ", "source": "ADP", "protects": ["MCU"]}]
    return {"parts": {"part": parts}, "netlist": {"net": nets, "node": nodes, "connection": conns},
            "paths": {"path": paths, "fault": faults, "pump": pumps, "clamp": clamps}}


def run(data):
    return ch.check(data)


def findings(data, rule, status=None):
    for r in run(data):
        if r["id"] == rule:
            return [f for f in r["findings"] if status is None or f.status == status]
    raise KeyError(rule)


def part(d, pid):
    return next(p for p in d["parts"]["part"] if p["id"] == pid)


def conn(d, cid):
    return next(c for c in d["netlist"]["connection"] if c["id"] == cid)


def add_conn(d, cid, a, b, net="LOG", joint="dupont", **kw):
    d["netlist"]["connection"].append({"id": cid, "from": a, "to": b, "net": net, "domain": "x", "joint": joint,
                                       "variant": "both", "observed_by": [1], **kw})


class Base(unittest.TestCase):
    def fails(self, d, rule):
        self.assertTrue(findings(d, rule, "FAIL"), f"{rule} should FAIL")

    def warns(self, d, rule):
        self.assertTrue(findings(d, rule, "WARN"), f"{rule} should WARN")

    def ok(self, d, rule):
        self.assertFalse(findings(d, rule, "FAIL") + findings(d, rule, "WARN"), f"{rule} should be clean")


class RealData(Base):
    EXPECTED_WARN = {"HW-005", "HW-014", "HW-019", "HW-020", "HW-027", "HW-029"}

    def test_real_p1_design_passes(self):
        data, problems = ch.load(REAL)
        self.assertEqual(problems, [])
        rules = ch.check(data)
        self.assertEqual([r["id"] for r in rules if r["status"] == "FAIL"], [])
        self.assertEqual({r["id"] for r in rules if r["status"] == "WARN"}, self.EXPECTED_WARN)
        self.assertEqual(len(rules), 34)

    def test_cli_exit_codes_and_json(self):
        with contextlib.redirect_stdout(io.StringIO()) as out:
            self.assertEqual(ch.main(["--data", str(REAL)]), 0)
            self.assertEqual(ch.main(["--data", str(REAL), "--json"]), 0)
            with tempfile.TemporaryDirectory() as t:
                Path(t, "parts.toml").write_text("not toml [[[")
                self.assertEqual(ch.main(["--data", t]), 1)
        self.assertIn("RESULT FAIL", out.getvalue())

    def test_json_shape(self):
        data, _ = ch.load(REAL)
        rules = ch.check(data)
        out = json.loads(json.dumps(ch.jsonable(rules, "p1")))
        self.assertEqual(out["result"], "PASS")
        self.assertEqual(out["rules"][1]["id"], "HW-001")

    def test_a28_is_the_only_allow_listed_fail(self):
        data, _ = ch.load(REAL)
        fails = [f["id"] for f in data["paths"]["fault"] if f["verdict"] == "FAIL"]
        self.assertEqual(fails, ["A28"])
        self.assertEqual(set(ch.FAULT_FAIL_ALLOWLIST), {"A28"})


class Synthetic(Base):
    def test_baseline_is_clean(self):
        rules = run(baseline())
        self.assertEqual([r["id"] for r in rules if r["status"] in ("FAIL", "WARN")], [])

    def test_hw001_joint(self):
        d = baseline(); conn(d, "C1")["joint"] = "glue"; self.fails(d, "HW-001")
        d = baseline(); del conn(d, "C1")["joint"]; self.fails(d, "HW-001")
        d = baseline(); conn(d, "C1")["joint"] = "Screw Terminal"; self.fails(d, "HW-001")

    def test_hw002_endpoints_and_ids(self):
        d = baseline(); conn(d, "C1")["to"] = "node:NOPE"; self.fails(d, "HW-002")
        d = baseline(); conn(d, "C1")["from"] = "GHOST:+"; self.fails(d, "HW-002")
        d = baseline(); conn(d, "C1")["from"] = "ADP:9"; self.fails(d, "HW-002")
        d = baseline(); conn(d, "C1")["from"] = "ADP"; self.fails(d, "HW-002")
        d = baseline(); d["netlist"]["connection"].append(copy.deepcopy(conn(d, "C1"))); self.fails(d, "HW-002")
        d = baseline(); conn(d, "C1")["variant"] = "C"; self.fails(d, "HW-002")

    def test_hw003_references(self):
        d = baseline(); d["paths"]["path"][0]["series_elements"][1]["part"] = "GHOST"; self.fails(d, "HW-003")
        d = baseline(); d["paths"]["path"][0]["series_elements"][1]["pins"] = ["NOPE"]; self.fails(d, "HW-003")
        d = baseline(); d["paths"]["fault"][0]["path"] = "ZZ*"; self.fails(d, "HW-003")

    def test_hw004_declared(self):
        d = baseline(); conn(d, "C1")["net"] = "GHOST"; self.fails(d, "HW-004")
        d = baseline(); del d["netlist"]["net"][1]["domain"]; self.fails(d, "HW-004")
        d = baseline(); d["netlist"]["node"][0]["net"] = "GHOST"; self.fails(d, "HW-004")

    def test_hw005_logic_high(self):
        d = baseline(); part(d, "MCU")["voh_min_v"] = 1.5; self.fails(d, "HW-005")
        d = baseline(); part(d, "MCU")["voh_min_v"] = 2.2; self.warns(d, "HW-005")  # 0.2 V margin
        d = baseline(); p = part(d, "MOSM"); del p["vih_min_v"]; p["vih_frac"] = 0.7; p["logic_v"] = 5.0
        part(d, "MCU")["voh_min_v"] = 3.0; self.fails(d, "HW-005")  # 3.0 V vs 0.7 x 5 V

    def test_hw006_logic_low(self):
        d = baseline(); part(d, "MOSM")["vil_max_v"] = 0.3; self.fails(d, "HW-006")
        d = baseline(); part(d, "MOSM")["vil_max_v"] = 0.6; self.warns(d, "HW-006")

    def test_hw007_overvoltage(self):
        d = baseline(); part(d, "MOSM")["abs_max_input_v"] = 3.0; self.fails(d, "HW-007")
        d = baseline()
        add_conn(d, "C30", "MOSM:PWM", "MCU:GP2", series_r_ohm=1000)
        part(d, "MOSM")["logic_v"] = 5.0
        self.warns(d, "HW-007")  # a 5 V driver into a pin that tolerates it only while powered

    def test_hw008_backfeed(self):
        d = baseline(); del conn(d, "C14")["series_r_ohm"]; self.warns(d, "HW-008")
        d = baseline(); part(d, "MOSM")["isolator"] = True; part(d, "MOSM")["isolator_domains"] = ["G", "H"]
        del conn(d, "C14")["series_r_ohm"]; self.ok(d, "HW-008")

    def test_hw009_drive_current(self):
        d = baseline(); part(d, "MOSM")["input_current_ma"] = 5.0; self.fails(d, "HW-009")
        d = baseline(); part(d, "MOSM")["input_current_ma"] = 3.5; self.warns(d, "HW-009")

    def test_hw010_budget(self):
        d = baseline(); part(d, "MCU")["supply_ma_max"] = 1200.0; self.fails(d, "HW-010")
        d = baseline(); part(d, "MCU")["supply_ma_max"] = 900.0; self.warns(d, "HW-010")
        d = baseline(); del part(d, "BK")["supply_out_ma"]; self.fails(d, "HW-010")

    def test_hw011_fuse_wire(self):
        d = baseline(); part(d, "FZ")["rating_a"] = 5.0; self.fails(d, "HW-011")

    def test_hw012_unfused_converter(self):
        d = baseline()
        d["netlist"]["connection"] = [c for c in d["netlist"]["connection"] if c["id"] not in ("C2", "C3")]
        add_conn(d, "C3b", "ADP:+", "BK:IN+", "V12")
        self.fails(d, "HW-012")

    def test_hw013_crowbar(self):
        d = baseline()
        d["netlist"]["connection"] = [c for c in d["netlist"]["connection"] if c["id"] != "C5"]
        self.fails(d, "HW-013")

    def test_hw014_i2c(self):
        d = baseline()
        for pid in ("MOSM", "SW"):
            p = part(d, pid); p["interfaces"] = ["i2c"]; p["i2c_addr"] = "0x40"; p["verified_fields"] = ["i2c_addr"]
            p["pins"].append({"name": "SDA", "function": "I2C"})
        add_conn(d, "C40", "MCU:GP1", "MOSM:SDA"); add_conn(d, "C41", "MCU:GP1", "SW:SDA")
        self.fails(d, "HW-014")
        part(d, "SW")["i2c_addr"] = "0x41"
        self.ok(d, "HW-014")
        del part(d, "SW")["verified_fields"]
        self.warns(d, "HW-014")

    def test_hw015_isolation(self):
        d = baseline(); part(d, "SW")["isolation_domain"] = "H"; self.fails(d, "HW-015")
        d = baseline(); part(d, "SW")["isolation_domain"] = "H"; conn(d, "C10")["bridges_isolation"] = True
        self.fails(d, "HW-015")  # no reason
        conn(d, "C10")["reason"] = "reviewed"; conn(d, "C11")["bridges_isolation"] = True; conn(d, "C11")["reason"] = "reviewed"
        self.ok(d, "HW-015")
        d = baseline(); part(d, "SW")["isolator"] = True; self.fails(d, "HW-015")  # isolator with no domains

    def test_hw016_failsafe(self):
        d = baseline(); part(d, "REL")["default_state"]["power_up"] = "energised"
        part(d, "MOSM")["default_state"]["power_up"] = "on"
        self.fails(d, "HW-016")
        d = baseline(); part(d, "MOSM")["default_state"]["wire_off"] = "conducts"
        part(d, "REL")["default_state"]["wire_off"] = "coil held"
        self.fails(d, "HW-016")

    def test_hw016_one_open_element_is_enough(self):
        d = baseline(); part(d, "REL")["default_state"]["brownout"] = "coil may stay held"
        self.ok(d, "HW-016")

    def test_hw017_active_low_gpio(self):
        d = baseline(); add_conn(d, "C50", "MCU:GP1", "REL:IN1", "V5"); self.fails(d, "HW-017")

    def test_hw018_fault_rows(self):
        d = baseline(); d["paths"]["fault"] = [f for f in d["paths"]["fault"] if f["element"] != "K"]; self.fails(d, "HW-018")
        d = baseline()
        for f in d["paths"]["fault"]:
            if f["element"] == "Q":
                f["variant"] = "A"
        self.fails(d, "HW-018")  # variant B path has no row for Q
        d = baseline()
        for f in d["paths"]["fault"]:
            if f["element"] == "Q":
                f["element"] = "MOSM"  # a row may name the part id instead
        self.ok(d, "HW-018")

    def test_hw019_verdicts(self):
        d = baseline(); d["paths"]["fault"][0]["verdict"] = "FAIL"; self.fails(d, "HW-019")
        d = baseline(); d["paths"]["fault"][0]["load_state"] = "on"; self.fails(d, "HW-019")
        d = baseline(); d["paths"]["fault"][0]["load_state"] = "on-bounded"; self.fails(d, "HW-019")
        d["paths"]["fault"][0]["bound_ms"] = 30; self.warns(d, "HW-019")
        d = baseline(); d["paths"]["fault"][0]["load_state"] = "maybe"; self.fails(d, "HW-019")

    def test_hw019_allow_list_is_the_only_exception(self):
        d = baseline(); f = d["paths"]["fault"][0]; f["id"] = "A28"; f["load_state"] = "on"; f["verdict"] = "FAIL"
        self.assertFalse(findings(d, "HW-019", "FAIL"))
        self.warns(d, "HW-019")

    def test_hw020_independence(self):
        d = baseline()
        for p in d["paths"]["path"]:
            p["series_elements"][2]["controlled_by"] = "MCU (level)"
        self.fails(d, "HW-020")

    def test_hw021_safety_connection_observed(self):
        d = baseline(); conn(d, "C11")["observed_by"] = []; self.fails(d, "HW-021")
        d = baseline(); conn(d, "C11")["observed_by"] = [9]; self.fails(d, "HW-021")

    def test_hw022_exit_tests(self):
        d = baseline(); conn(d, "C12")["observed_by"] = [5]; self.warns(d, "HW-022")  # test 6 unobserved
        d = baseline(); conn(d, "C12")["observed_by"] = [5, 6, 7]; self.fails(d, "HW-022")

    def test_hw023_probes(self):
        d = baseline(); d["netlist"]["net"][4]["nominal_v"] = 12.0
        add_conn(d, "C60", "AN:D0", "node:TB12", "V12"); self.fails(d, "HW-023")
        d = baseline(); conn(d, "C21")["to"] = "node:LAMPNODE"; conn(d, "C21")["net"] = "LAMPN"; self.fails(d, "HW-023")

    def test_hw024_usb_host_port_reuse(self):
        d = baseline(); add_conn(d, "C70", "HOST:USB1", "MCU:USB", "USB", "usb"); self.fails(d, "HW-024")

    def test_hw025_reserved_pin(self):
        d = baseline(); add_conn(d, "C80", "MCU:GP16", "MOSM:PWM"); self.fails(d, "HW-025")

    def test_hw026_trust_root(self):
        d = baseline(); part(d, "MCU")["trust_root"] = True; add_conn(d, "C90", "HOST:USB2", "MCU:USB", "USB", "usb")
        self.fails(d, "HW-026")
        d = baseline(); part(d, "MCU")["trust_root"] = True; add_conn(d, "C91", "WD:1", "MCU:SWCLK")
        self.fails(d, "HW-026")
        conn(d, "C91")["removable"] = True; self.ok(d, "HW-026")

    def test_hw027_checks(self):
        d = baseline(); part(d, "WD")["counterfeit_risk"] = True; self.warns(d, "HW-027")
        part(d, "WD")["acceptance_passed"] = True; self.ok(d, "HW-027")
        d = baseline(); part(d, "WD")["fuse_bytes"] = {"lfuse": "0xE2", "datasheet_checked": False}; self.warns(d, "HW-027")
        part(d, "WD")["fuse_bytes"]["datasheet_checked"] = True; self.ok(d, "HW-027")

    def test_hw028_provenance_is_information_only(self):
        d = baseline(); part(d, "MCU")["verified"] = True
        fs = findings(d, "HW-028")
        self.assertEqual([f.status for f in fs], ["INFO"])
        self.assertEqual(ch.result_of(run(d)), "PASS")

    def test_variants_are_checked_independently(self):
        d = baseline(); add_conn(d, "C95", "MCU:GP16", "MOSM:PWM", variant="B")
        fs = findings(d, "HW-025", "FAIL")
        self.assertEqual({f.variant_label() for f in fs}, {"B"})
        d = baseline(); conn(d, "C1")["variant"] = "A"; del conn(d, "C1")["joint"]
        self.assertEqual({f.variant_label() for f in findings(d, "HW-001", "FAIL")}, {"A"})


class Regressions(Base):
    """One test per safety finding of hardware/p1/compatibility.md. #294 is the tracking issue."""

    def test_regression_294_active_low_relay_driven_directly_by_gpio(self):
        d = baseline(); add_conn(d, "C50", "MCU:GP1", "REL:IN1", "V5")
        fs = findings(d, "HW-017", "FAIL")
        self.assertTrue(fs and "active-low" in fs[0].message)

    def test_regression_294_unfused_buck_feeding_all_brains(self):
        d = baseline()
        d["netlist"]["connection"] = [c for c in d["netlist"]["connection"] if c["id"] not in ("C2", "C3")]
        add_conn(d, "C3b", "ADP:+", "BK:IN+", "V12")
        self.fails(d, "HW-012")

    def test_regression_294_buck_without_crowbar(self):
        d = baseline()
        d["netlist"]["connection"] = [c for c in d["netlist"]["connection"] if c["id"] != "C5"]
        self.fails(d, "HW-013")

    def test_regression_294_analyzer_ground_on_switched_lamp_node(self):
        d = baseline(); conn(d, "C21")["to"] = "node:LAMPNODE"; conn(d, "C21")["net"] = "LAMPN"
        self.fails(d, "HW-023")

    def test_regression_294_analyzer_ground_fault_a28_is_the_documented_fail(self):
        data, _ = ch.load(REAL)
        row = next(f for f in data["paths"]["fault"] if f["id"] == "A28")
        self.assertEqual((row["load_state"], row["verdict"]), ("on", "FAIL"))
        self.assertIn("A28", ch.FAULT_FAIL_ALLOWLIST)

    def test_regression_294_fuse_larger_than_wire_ampacity(self):
        d = baseline(); part(d, "FZ")["rating_a"] = 5.0
        self.fails(d, "HW-011")

    def test_regression_294_mcu_cannot_drive_max813l_wdi_directly(self):
        d = baseline()
        part(d, "WD")["voh_min_v"] = 2.5
        d["parts"]["part"].append({"id": "U4", "vih_min_v": 3.5, "vil_max_v": 0.8, "logic_v": 5.0, "isolation_domain": "G",
                                   "power_group": "5V", "pins": pins(("6", "WDI"))})
        add_conn(d, "C97", "WD:1", "U4:6", "V5", series_r_ohm=1000)
        self.fails(d, "HW-005")

    def test_regression_294_two_cuts_on_one_relay_board_share_a_common_cause(self):
        d = baseline()
        for p in d["paths"]["path"]:
            p["series_elements"][2]["board"] = "RLY"; p["series_elements"][2]["supply_net"] = "V5"
            p["series_elements"][1]["board"] = "RLY"; p["series_elements"][1]["supply_net"] = "V5"
        self.fails(d, "HW-020")  # revised after independent review B5: a shared board is a FAIL, not a WARN

    def test_regression_294_series_element_without_a_fault_row(self):
        d = baseline(); d["paths"]["fault"] = [f for f in d["paths"]["fault"] if f["element"] != "Q"]
        self.fails(d, "HW-018")

    def test_regression_294_fault_row_with_fail_verdict_off_the_allow_list(self):
        d = baseline(); d["paths"]["fault"][1]["verdict"] = "FAIL"
        self.fails(d, "HW-019")

    def test_regression_294_trust_root_never_powered_through_usb(self):
        d = baseline(); part(d, "MCU")["trust_root"] = True; add_conn(d, "C90", "HOST:USB2", "MCU:USB", "USB", "usb")
        self.fails(d, "HW-026")

    def test_regression_288_firmware_pin_on_board_ws2812_is_refused(self):
        d = baseline(); add_conn(d, "C80", "MCU:GP16", "MOSM:PWM")
        fs = findings(d, "HW-025", "FAIL")
        self.assertTrue(fs and "reserved" in fs[0].message)

    def test_regression_288_bottom_pad_gp17_is_not_an_edge_pin(self):
        data, _ = ch.load(REAL)
        self.assertNotIn("GP17", [p["name"] for p in next(x for x in data["parts"]["part"] if x["id"] == "RP_TR")["pins"]])
        d = baseline(); add_conn(d, "C81", "MCU:GP17", "MOSM:PWM")
        self.fails(d, "HW-002")

    def test_regression_288_real_pin_map_has_the_missing_signals(self):
        data, _ = ch.load(REAL)
        used = {c["from"] for c in data["netlist"]["connection"]} | {c["to"] for c in data["netlist"]["connection"]}
        for pin in ("RP_TR:GP26", "RP_TR:GP27", "RP_TR:GP8", "RP_TR:GP10", "RP_TR:GP11", "RP_TR:GP29", "RP_TR:GP28", "RP_TR:GP14"):
            self.assertIn(pin, used)
        self.assertNotIn("RP_TR:GP16", used)

    def test_regression_289_analyzer_hosted_by_a_powered_off_pi(self):
        d = baseline(); part(d, "HOST")["off_in_tests"] = [4]; conn(d, "C22")["observed_by"] = [4]
        self.fails(d, "HW-022")

    def test_regression_289_test_4_needs_a_host_that_stays_on(self):
        d = baseline(); part(d, "HOST")["off_in_tests"] = [4]
        self.fails(d, "HW-022")  # no analyzer link observed by test 4 from another machine
        d["parts"]["part"].append({"id": "MAC", "isolation_domain": "G", "pins": pins(("USB", "usb"))})
        add_conn(d, "C23", "MAC:USB", "AN:USB", "USB", "usb", observed_by=[4])
        self.assertFalse(findings(d, "HW-022", "FAIL"))

    def test_regression_289_real_design_hosts_test_4_on_the_mac(self):
        data, _ = ch.load(REAL)
        c = next(c for c in data["netlist"]["connection"] if c["id"] == "C724")
        self.assertEqual((c["from"], c["observed_by"]), ("MAC:USBA", [4]))
        c720 = next(c for c in data["netlist"]["connection"] if c["id"] == "C720")
        self.assertNotIn(4, c720["observed_by"])

    def test_regression_290_swd_lead_on_the_trust_root_must_be_removable(self):
        d = baseline(); part(d, "MCU")["trust_root"] = True; add_conn(d, "C91", "WD:1", "MCU:SWCLK")
        self.fails(d, "HW-026")

    def test_regression_290_real_trust_root_has_no_usb_and_removable_swd(self):
        data, _ = ch.load(REAL)
        for c in data["netlist"]["connection"]:
            ends = (c["from"], c["to"])
            self.assertNotIn("RP_TR:USB", ends)
            if any(e in ("RP_TR:SWCLK", "RP_TR:SWDIO") for e in ends):
                self.assertIs(c.get("removable"), True)

    def test_regression_292_attiny_fuse_bytes_need_a_datasheet_check(self):
        data, _ = ch.load(REAL)
        fb = next(p for p in data["parts"]["part"] if p["id"] == "U3_ATTINY85")["fuse_bytes"]
        self.assertEqual((fb["lfuse"], fb["hfuse"], fb["efuse"], fb["lock"]), ("0xE2", "0xCD", "0xFF", "0xFC"))
        self.assertFalse(fb["datasheet_checked"])
        self.assertTrue([f for f in findings(data, "HW-027", "WARN") if "U3_ATTINY85" in f.ids])

    def test_regression_293_no_name_max813l_needs_an_acceptance_result(self):
        data, _ = ch.load(REAL)
        self.assertTrue([f for f in findings(data, "HW-027", "WARN") if "U4_MAX813L" in f.ids])
        d = baseline(); part(d, "WD")["counterfeit_risk"] = True
        self.warns(d, "HW-027")


class IndependentReview(Base):
    """Regression tests for the independent clean-room review (#296): one per blocking finding."""

    def real(self):
        data, _ = ch.load(REAL)
        return data

    def test_regression_296_pump_at_the_firmware_rate_with_the_original_values_never_pulls_in(self):
        d = baseline(); pu = d["paths"]["pump"][0]
        pu.update(c_series_f=100e-9, c_hold_f=1e-6, r_bleed_ohm=100000.0)  # the values drawn for 1 kHz
        fs = findings(d, "HW-031", "FAIL")
        self.assertTrue(fs and "50 Hz" in fs[0].message)

    def test_regression_296_the_original_pump_values_were_fine_at_1_khz_so_rate_is_what_matters(self):
        d = baseline(); pu = d["paths"]["pump"][0]
        pu.update(c_series_f=100e-9, c_hold_f=1e-6, r_bleed_ohm=100000.0, toggle_period_us=500)
        self.ok(d, "HW-031")

    def test_regression_296_pump_set_1_does_not_serve_a_15_ma_relay_input(self):
        d = baseline(); part(d, "REL")["input_current_ma"] = 15.0
        self.fails(d, "HW-031")

    def test_regression_296_real_pumps_are_checked_and_pass(self):
        data = self.real()
        self.assertEqual(len(data["paths"]["pump"]), 2)
        self.assertFalse(findings(data, "HW-031", "FAIL"))
        k1 = data["paths"]["pump"][0]
        self.assertEqual(k1["toggle_period_us"], 10000)  # one toggle per 10 ms tick: 50 Hz

    def test_regression_296_real_pump_with_the_original_values_fails(self):
        data = self.real()
        data["paths"]["pump"][0].update(c_series_f=100e-9, c_hold_f=1e-6, r_bleed_ohm=100000.0)
        self.fails(data, "HW-031")

    def test_regression_296_zener_clamp_can_fail_before_the_fuse_and_clamps_too_high(self):
        d = baseline(); c = d["paths"]["clamp"][0]
        c.update(kind="zener", clamp_v_max=8.0, rated_a=13.4, rated_ms=8.3)
        msgs = " ".join(f.message for f in findings(d, "HW-029", "FAIL"))
        self.assertIn("absolute maximum", msgs)
        self.assertIn("before", msgs)

    def test_regression_296_real_zener_design_fails(self):
        data = self.real()
        data["paths"]["clamp"][0].update(kind="zener", clamp_v_max=8.0, rated_a=13.4, rated_ms=8.3)
        self.fails(data, "HW-029")

    def test_regression_296_clamp_must_not_fire_in_normal_operation(self):
        d = baseline(); d["paths"]["clamp"][0]["trip_min_v"] = 5.0
        self.fails(d, "HW-029")

    def test_regression_296_clamp_rating_below_the_fault_current(self):
        d = baseline(); d["paths"]["clamp"][0]["rated_a"] = 2.0
        self.fails(d, "HW-029")

    def test_regression_296_crowbar_without_a_clamp_record_is_unchecked(self):
        d = baseline(); d["paths"]["clamp"] = []
        self.fails(d, "HW-013")

    def _with_probe(self, host_ai=True):
        d = baseline()
        part(d, "MCU")["trust_root"] = True
        for x in part(d, "MCU")["pins"]:
            if x["name"] == "SWCLK":
                x["debug"] = True
        part(d, "HOST")["ai_host"] = host_ai
        d["parts"]["part"].append({"id": "PROBE", "kind": "probe", "passthrough": True, "isolation_domain": "G",
                                   "pins": pins(("USB", "usb"), ("SWD", "swd"))})
        add_conn(d, "C96", "HOST:USB2", "PROBE:USB", "USB", "usb")
        add_conn(d, "C97", "PROBE:SWD", "MCU:SWCLK", "LOG", "jst-xh", removable=True)
        return d

    def test_regression_296_ai_host_with_a_probe_reaches_swd(self):
        self.fails(self._with_probe(), "HW-030")

    def test_regression_296_probe_on_a_host_that_is_not_ai_controlled_is_allowed(self):
        self.ok(self._with_probe(host_ai=False), "HW-030")

    def test_regression_296_ai_host_with_an_isp_wire_reaches_the_watchdog(self):
        d = baseline()
        d["parts"]["part"].append({"id": "TINY", "trust_root": True, "isolation_domain": "G",
                                   "pins": [{"name": "5", "function": "MOSI", "debug": True}]})
        part(d, "HOST")["ai_host"] = True
        part(d, "HOST")["pins"].append({"name": "GPIO10", "function": "GPIO10 MOSI"})
        add_conn(d, "C98", "HOST:GPIO10", "TINY:5", "LOG", "dupont")
        self.fails(d, "HW-030")

    def test_regression_296_real_pi_has_no_path_to_debug_pins(self):
        data = self.real()
        self.assertFalse(findings(data, "HW-030", "FAIL"))
        pi = next(p for p in data["parts"]["part"] if p["id"] == "PI5")
        self.assertTrue(pi["ai_host"])
        data["netlist"]["connection"].append({"id": "C722x", "from": "PI5:USB2A", "to": "PICO:USB", "net": "USB", "domain": "USB",
                                              "joint": "usb", "variant": "both", "observed_by": [5]})
        self.fails(data, "HW-030")  # the original Pi-hosted probe

    def test_regression_296_both_relays_on_one_module_fail(self):
        d = baseline()
        for p in d["paths"]["path"]:
            p["series_elements"][2]["part"] = "REL"; p["series_elements"][2]["pins"] = []
        fs = findings(d, "HW-020", "FAIL")
        self.assertTrue(any("same module" in f.message for f in fs))

    def test_regression_296_unkeyed_connector_on_a_cut_signal_fails(self):
        d = baseline(); conn(d, "C13")["joint"] = "dupont"
        self.fails(d, "HW-020")

    def test_regression_296_real_cuts_are_on_separate_keyed_modules(self):
        data = self.real()
        self.assertFalse(findings(data, "HW-020", "FAIL"))
        k = {e["id"]: e for e in data["paths"]["path"][0]["series_elements"]}
        self.assertNotEqual(k["K1"]["part"], k["K2"]["part"])

    def test_regression_296_10k_pulldown_does_not_overcome_e9(self):
        d = baseline(); add_conn(d, "C99", "node:Q1.C", "MCU:GP2", pulldown_ohm=10000.0, safe_state="low")
        self.fails(d, "HW-032")

    def test_regression_296_4k7_pulldown_overcomes_e9(self):
        d = baseline(); add_conn(d, "C99", "node:Q1.C", "MCU:GP2", pulldown_ohm=4700.0, safe_state="low")
        self.ok(d, "HW-032")

    def test_regression_296_safe_low_input_without_a_pulldown_warns(self):
        d = baseline(); add_conn(d, "C99", "node:Q1.C", "MCU:GP2", safe_state="low")
        self.warns(d, "HW-032")

    def test_regression_296_real_sense_pulldowns_are_4k7(self):
        data = self.real()
        pd = {c["id"]: c["pulldown_ohm"] for c in data["netlist"]["connection"] if "pulldown_ohm" in c}
        self.assertEqual({k: pd[k] for k in ("C324", "C325", "C326")}, {"C324": 4700, "C325": 4700, "C326": 4700})
        self.assertFalse(findings(data, "HW-032", "FAIL"))

    def _variant_b(self, claim, latch=None):
        d = baseline()
        el = d["paths"]["path"][1]["series_elements"][1]
        el["cut_sources"] = ["WD:2"]; el["cut_claim"] = claim
        if latch:
            el["latch"] = latch
        return d

    def test_regression_296_wdo_to_mr_read_as_a_held_cut_fails(self):
        self.fails(self._variant_b("held"), "HW-033")

    def test_regression_296_pulse_cut_without_a_latch_fails(self):
        self.fails(self._variant_b("pulse-latched"), "HW-033")

    def test_regression_296_pulse_cut_with_a_latch_passes(self):
        self.ok(self._variant_b("pulse-latched", "RP2350 latches on VBUS loss"), "HW-033")

    def test_regression_296_real_variant_b_k2_names_its_latch(self):
        data = self.real()
        k2 = [e for p in data["paths"]["path"] if p["variant"] == "B" for e in p["series_elements"] if e["id"] == "K2"]
        self.assertEqual(len(k2), 2)
        self.assertTrue(all(e["cut_claim"] == "pulse-latched" and e["latch"] for e in k2))
        k2[0]["cut_claim"] = "held"
        self.fails(data, "HW-033")

    def test_regression_296_fault_table_covers_the_review_rows(self):
        data = self.real()
        ids = {f["id"] for f in data["paths"]["fault"]}
        self.assertTrue({f"A{i}" for i in range(31, 40)} <= ids and {"B12", "B13"} <= ids)
        by = {f["id"]: f for f in data["paths"]["fault"]}
        self.assertIn("pulse", by["B1"]["effect"])
        self.assertNotIn("disabled", by["B6"]["effect"].split("Only")[0])


if __name__ == "__main__":
    unittest.main()
