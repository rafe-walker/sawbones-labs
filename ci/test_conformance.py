"""Tests for ci/conformance.py (the design-conformance X-ray).

Run: python3 -I -m unittest discover -s ci -p 'test_*.py' -v
Every failure mode has a regression_ test that builds a throwaway tree in a
temp dir, so the checker is exercised exactly as CI runs it. The last class
runs it on the real tree.
"""
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, HERE)
import conformance as c  # noqa: E402

DESIGN = "# Design\n\nThe latch starts tripped.\n\n- A second statement.\n"
NAMES = """
[[component]]
key = "cortex"
name = "C.O.R.T.E.X."
meaning = "supervisor"
crates = ["sup"]
"""
RUST = """
#[test]
fn it_starts_tripped() {}

#[test]
fn regression_9_a() {}

#[test]
#[ignore]
fn skipped_one() {}

fn helper_not_a_test() {}

// #[test]
// fn only_in_a_comment() {}
"""


def record(**kw):
    r = {
        "id": "SAW-CORTEX-LATCH-1", "rev": 1, "title": "t", "source": "docs/D.md",
        "anchor": "The latch starts tripped", "sha": None, "tracks": "[5]",
        "acc": [('a1', 'test', 'sup::it_starts_tripped', None)], "extra": "",
    }
    r.update(kw)
    return r


def toml_of(recs):
    out = []
    for r in recs:
        acc = []
        for i, k, ref, pend in r["acc"]:
            acc.append(f'  {{ id = "{i}", kind = "{k}", ref = "{ref}"' + (f", pending = {pend}" if pend else "") + " },")
        out.append(f'[[requirement]]\nid = "{r["id"]}"\nrev = {r["rev"]}\ntitle = "{r["title"]}"\nsource = "{r["source"]}"\n'
                   f'anchor = "{r["anchor"]}"\nsource_sha = "{r["sha"]}"\ntracks = {r["tracks"]}\n{r["extra"]}\nacceptance = [\n' + "\n".join(acc) + "\n]\n")
    return "\n".join(out)


class Tree:
    """A throwaway repo tree."""

    def __init__(self, recs=None, design=DESIGN, names=NAMES, rust=RUST, extra=None, bless=True):
        self.d = tempfile.mkdtemp()
        self.write("docs/D.md", design)
        self.write("docs/NAMES.md", "C.O.R.T.E.X. is the supervisor.\n")
        self.write("design/names.toml", names)
        self.write("crates/sup/Cargo.toml", '[package]\nname = "sup"\n')
        self.write("crates/sup/tests/t.rs", rust)
        for path, text in (extra or {}).items():
            self.write(path, text)
        recs = recs if recs is not None else [record()]
        for r in recs:
            if r["sha"] is None:
                unit = c.source_unit(design, r["anchor"])
                r["sha"] = c.unit_hash(unit)
        self.write("design/requirements/cortex.toml", toml_of(recs))
        if bless:
            self.run("--bless")

    def write(self, rel, text):
        p = os.path.join(self.d, rel)
        os.makedirs(os.path.dirname(p), exist_ok=True)
        with open(p, "w") as fh:
            fh.write(text)

    def run(self, *args, env=None):
        e = dict(os.environ, **(env or {}))
        r = subprocess.run([sys.executable, "-I", os.path.join(HERE, "conformance.py"), "--root", self.d, *args],
                           capture_output=True, text=True, env=e)
        return r.returncode, r.stdout + r.stderr

    def close(self):
        shutil.rmtree(self.d, ignore_errors=True)


class Base(unittest.TestCase):
    def tree(self, **kw):
        t = Tree(**kw)
        self.addCleanup(t.close)
        return t


class Statuses(Base):
    def status_of(self, t, rid="SAW-CORTEX-LATCH-1"):
        rows, errors = c.run_check(t.d)
        return {r["rec"]["id"]: r["status"] for r in rows}[rid], errors

    def test_built_when_every_check_resolves(self):
        s, e = self.status_of(self.tree())
        self.assertEqual((s, e), (c.BUILT, []))

    def test_partial_when_some_are_pending(self):
        t = self.tree(recs=[record(acc=[("a1", "test", "sup::it_starts_tripped", None), ("a2", "test", "not written yet", 5)])])
        self.assertEqual(self.status_of(t)[0], c.PARTIAL)
        self.assertEqual(t.run()[0], 0)

    def test_not_built_is_allowed(self):
        t = self.tree(recs=[record(acc=[("a1", "test", "not written yet", 5)])])
        self.assertEqual(self.status_of(t)[0], c.NOT_BUILT)
        self.assertEqual(t.run()[0], 0)

    def test_pending_check_that_now_exists_counts_as_built(self):
        t = self.tree(recs=[record(acc=[("a1", "test", "sup::it_starts_tripped", 5)])])
        self.assertEqual(self.status_of(t)[0], c.BUILT)

    def test_waived_is_shown_and_does_not_fail(self):
        t = self.tree(recs=[record(acc=[("a1", "test", "gone", None)], extra='waiver = { reason = "owner dropped it" }')])
        self.assertEqual(self.status_of(t)[0], c.WAIVED)

    def test_glob_matches_and_an_empty_glob_does_not(self):
        t = self.tree(recs=[record(acc=[("a1", "test", "sup::regression_9_*", None)])])
        self.assertEqual(self.status_of(t)[0], c.BUILT)
        t = self.tree(recs=[record(acc=[("a1", "test", "sup::regression_77_*", None)])])
        self.assertNotEqual(t.run()[0], 0)

    def test_bare_name_and_path_forms_resolve(self):
        for ref in ("it_starts_tripped", "crates/sup/tests/t.rs::it_starts_tripped", "sup::it_starts_tripped"):
            t = self.tree(recs=[record(acc=[("a1", "test", ref, None)])])
            self.assertEqual(self.status_of(t)[0], c.BUILT, ref)

    def test_file_kind(self):
        t = self.tree(recs=[record(acc=[("a1", "file", "docs/NAMES.md", None)])])
        self.assertEqual(self.status_of(t)[0], c.BUILT)
        t = self.tree(recs=[record(acc=[("a1", "file", "docs/nope.md", None)])])
        self.assertNotEqual(t.run()[0], 0)


class FailureModes(Base):
    def test_regression_unknown_id_in_an_impl_tag_fails(self):
        t = self.tree(extra={"crates/sup/src/lib.rs": "// [impl->SAW-CORTEX-NOPE-1]\n"})
        rc, out = t.run()
        self.assertNotEqual(rc, 0)
        self.assertIn("unknown id SAW-CORTEX-NOPE-1", out)

    def test_known_impl_tag_passes_and_an_old_rev_tag_drifts(self):
        t = self.tree(extra={"crates/sup/src/lib.rs": "// [impl->SAW-CORTEX-LATCH-1]\n// [impl->SAW-CORTEX-LATCH-1@1]\n"})
        self.assertEqual(t.run()[0], 0)
        t.write("crates/sup/src/lib.rs", "// [impl->SAW-CORTEX-LATCH-1@2]\n")
        rc, out = t.run()
        self.assertNotEqual(rc, 0)
        self.assertIn("drifted", out)

    def test_regression_tags_outside_comments_are_not_claims(self):
        t = self.tree(extra={"crates/sup/src/lib.rs": 'let s = "[impl->SAW-CORTEX-NOPE-1]";\n'})
        self.assertEqual(t.run()[0], 0)

    def test_regression_duplicate_id_fails(self):
        t = self.tree(recs=[record(), record()], bless=False)
        rc, out = t.run("--bless")
        self.assertNotEqual(rc, 0)
        self.assertIn("duplicate id", out)

    def test_regression_missing_test_fails(self):
        t = self.tree(recs=[record(acc=[("a1", "test", "sup::no_such_test", None)])], bless=False)
        rc, out = t.run("--bless")
        self.assertNotEqual(rc, 0)
        self.assertIn("no test matches", out)

    def test_regression_ignored_commented_and_helper_fns_do_not_count(self):
        for name in ("skipped_one", "only_in_a_comment", "helper_not_a_test"):
            t = self.tree(recs=[record(acc=[("a1", "test", f"sup::{name}", None)])], bless=False)
            rc, out = t.run("--bless")
            self.assertNotEqual(rc, 0, name)

    def test_regression_wrong_crate_or_path_does_not_resolve(self):
        for ref in ("other::it_starts_tripped", "crates/nope/t.rs::it_starts_tripped"):
            t = self.tree(recs=[record(acc=[("a1", "test", ref, None)])], bless=False)
            self.assertNotEqual(t.run("--bless")[0], 0, ref)

    def test_regression_drifted_when_the_design_prose_changes(self):
        t = self.tree()
        self.assertEqual(t.run()[0], 0)
        t.write("docs/D.md", DESIGN.replace("starts tripped", "starts clear"))
        rc, out = t.run()
        self.assertNotEqual(rc, 0)
        self.assertIn("drifted", out)
        rows, _ = c.run_check(t.d)
        self.assertEqual(rows[0]["status"], c.DRIFTED)

    def test_regression_drifted_when_the_anchor_paragraph_is_removed(self):
        t = self.tree()
        t.write("docs/D.md", "# Design\n\nnothing here\n")
        rc, out = t.run()
        self.assertNotEqual(rc, 0)
        self.assertIn("anchor not found", out)

    def test_drift_outranks_a_waiver(self):
        t = self.tree(recs=[record(extra='waiver = { reason = "x" }')])
        t.write("docs/D.md", DESIGN.replace("tripped", "loose"))
        rows, _ = c.run_check(t.d)
        self.assertEqual(rows[0]["status"], c.DRIFTED)

    def test_regression_malformed_records_fail(self):
        bad = [
            record(id="CORTEX-1"),
            record(id="SAW-NERVE-X-1"),
            record(rev=0),
            record(tracks="[]", acc=[("a1", "test", "sup::it_starts_tripped", 5)]),
            record(acc=[]),
            record(acc=[("a1", "magic", "x", None)]),
            record(acc=[("a1", "test", "sup::it_starts_tripped", 99)]),
            record(extra="surprise = 1"),
            record(extra='waiver = { reason = "" }'),
            record(acc=[("a1", "test", "sup::it_starts_tripped", None), ("a1", "test", "sup::regression_9_a", None)]),
        ]
        for r in bad:
            t = self.tree(recs=[r], bless=False)
            rc, out = t.run("--bless")
            self.assertNotEqual(rc, 0, r)

    def test_regression_malformed_toml_and_unknown_component_fail(self):
        t = self.tree()
        t.write("design/requirements/cortex.toml", "[[requirement\n")
        self.assertNotEqual(t.run()[0], 0)
        t = self.tree()
        with open(os.path.join(t.d, "design/requirements/cortex.toml")) as fh:
            t.write("design/requirements/ghost.toml", fh.read())
        rc, out = t.run()
        self.assertNotEqual(rc, 0)
        self.assertIn("not a component", out)

    def test_regression_names_must_match_crates_and_docs(self):
        t = self.tree(names=NAMES.replace('["sup"]', '["no-such-crate"]'))
        self.assertIn("not in crates/", t.run()[1])
        t = self.tree()
        t.write("docs/NAMES.md", "empty\n")
        self.assertIn("does not mention", t.run()[1])

    def test_regression_ambiguous_anchor_fails(self):
        d = "# D\n\nsame words\n\nsame words again\n"
        t = self.tree(design=d, recs=[record(anchor="same words", sha="0" * 16)], bless=False)
        rc, out = t.run("--bless")
        self.assertNotEqual(rc, 0)
        self.assertIn("matches 2 lines", out)


class Feedback(Base):
    def test_regression_a_hand_typed_status_is_refused(self):
        t = self.tree(recs=[record(extra='status = "built"')], bless=False)
        rc, out = t.run("--bless")
        self.assertNotEqual(rc, 0)
        self.assertIn("unknown fields", out)

    def test_regression_268_a_waived_record_does_not_fail_on_missing_checks(self):
        t = self.tree(recs=[record(acc=[("a1", "test", "gone", None)], extra='waiver = { reason = "owner dropped it" }')], bless=False)
        rc, out = t.run("--bless")
        self.assertEqual(rc, 0, out)
        self.assertIn("1 waived", out)

    def test_regression_findings_say_what_to_update(self):
        t = self.tree(recs=[record(anchor="The latch")])
        t.write("docs/D.md", DESIGN.replace("tripped", "clear"))
        out = t.run()[1]
        self.assertIn("source_sha =", out)
        self.assertIn("design/requirements/cortex.toml", out)
        self.assertIn("revert the design change", out)
        t.write("docs/D.md", "# Design\n\nnothing\n")
        self.assertIn("hash docs/D.md", t.run()[1])
        t = self.tree(recs=[record(acc=[("a1", "test", "sup::no_such_test", None)])], bless=False)
        self.assertIn("pending = <issue>", t.run("--bless")[1])
        t = self.tree(extra={"crates/sup/src/lib.rs": "// [impl->SAW-CORTEX-NOPE-1]\n"})
        self.assertIn("Add a record with that id", t.run()[1])

    def test_regression_a_requirement_with_no_issue_needs_one(self):
        t = self.tree(recs=[record(tracks="[]", acc=[("a1", "test", "later", None)], extra='waiver = { reason = "x" }')])
        self.assertEqual(t.run()[0], 0)  # waived: nothing to track
        t = self.tree(recs=[record(tracks="[]", acc=[("a1", "test", "sup::it_starts_tripped", None), ("a2", "test", "later", 7)])], bless=False)
        rc, out = t.run("--bless")
        self.assertNotEqual(rc, 0)  # pending #7 is not in tracks: malformed
        t = self.tree(recs=[record(tracks="[]")])
        self.assertEqual(t.run()[0], 0)  # built needs no issue
        with open(os.path.join(t.d, "design/requirements/cortex.toml")) as fh:
            text = fh.read()
        t.write("crates/sup/tests/t.rs", "")
        self.assertNotEqual(t.run()[0], 0)

    def test_status_lists_needs_an_issue_and_drifted_at_the_top(self):
        # A record whose only check is missing is not built; give it no issue and no pending item.
        t = self.tree(recs=[record(tracks="[]", acc=[("a1", "test", "sup::it_starts_tripped", None)])])
        t.write("docs/D.md", DESIGN.replace("starts tripped", "starts clear"))
        t.run("--bless")
        with open(os.path.join(t.d, "design/STATUS.md")) as fh:
            text = fh.read()
        self.assertLess(text.index("### Drifted"), text.index("### All requirements"))
        self.assertLess(text.index("### Needs an issue"), text.index("### Drifted"))
        drifted = text[text.index("### Drifted"):text.index("### All requirements")]
        self.assertIn("SAW-CORTEX-LATCH-1", drifted)
        # A partial record with no tracking issue is listed under "Needs an issue".
        t = self.tree(recs=[record(tracks="[]", acc=[("a1", "test", "sup::it_starts_tripped", None), ("a2", "test", "later", None)])], bless=False)
        t.run("--bless")
        with open(os.path.join(t.d, "design/STATUS.md")) as fh:
            text = fh.read()
        needs = text[text.index("### Needs an issue"):text.index("### Drifted")]
        self.assertIn("SAW-CORTEX-LATCH-1", needs)
        self.assertIn("tracked by no issue", t.run()[1])

    def test_the_workflow_judges_from_the_base_branch_and_holds_no_privilege(self):
        with open(os.path.join(ROOT, ".github/workflows/conformance.yml")) as fh:
            wf = fh.read()
        self.assertEqual(wf.count('git show "${BASE_REF}:ci/conformance.py"'), 2)
        self.assertIn("--rust-tests", wf)
        self.assertIn("permissions:\n  contents: read", wf)
        self.assertNotIn("pull_request_target", wf)
        self.assertNotIn("secrets.", wf)
        uses = [l for l in wf.splitlines() if "uses:" in l]
        self.assertTrue(uses)
        for l in uses:
            self.assertRegex(l, r"@[0-9a-f]{40} # v\d")
        # PR text goes through env vars only, never into a run script.
        import re
        run_blocks = re.findall(r"run: \|\n((?: {10}.*\n|\n)+)", wf)
        self.assertEqual(len(run_blocks), 8)
        for b in run_blocks:
            self.assertNotIn("github.event.pull_request", b)



def need_cargo(case):
    """Real-cargo probes skip locally without cargo but must fail in CI, so the
    checks cannot silently vanish there."""
    if not shutil.which("cargo"):
        if os.environ.get("CI"):
            case.fail("cargo is required in CI for the real-cargo probes")
        case.skipTest("needs cargo")


def cargo_list(path, tests):
    """Write a --rust-tests file. tests: (package, src, name[, ignored])."""
    rows = [{"package": t[0], "src": t[1], "target": "t", "name": t[2], "ignored": bool(t[3]) if len(t) > 3 else False} for t in tests]
    with open(path, "w") as fh:
        json.dump({"version": 1, "tests": rows}, fh)
    return path


class Issue271(Base):
    """Holes the clean-room review of #252 found (issue #271)."""

    TRICKS = """
#[test]
fn real_test() {}

/* outer /* nested */
#[test]
fn in_nested_comment() {}
*/

#[cfg(any())]
#[test]
fn cfg_never_fn() {}

#[cfg(any())]
mod hidden {
    #[test]
    fn in_cfg_never_mod() {}
}

macro_rules! never_invoked {
    () => {
        #[test]
        fn in_uninvoked_macro() {}
    };
}

#[test]
#[cfg_attr(all(), ignore)]
fn cfg_attr_ignored() {}
"""

    def rec(self, ref):
        return record(acc=[("a1", "test", ref, None)])

    def test_regression_271_source_fallback_drops_comments_never_cfg_macros_and_cfg_attr_ignore(self):
        live, ign = c.rust_tests(self.TRICKS)
        self.assertEqual(live, {"real_test"})
        self.assertEqual(ign, {"cfg_attr_ignored"})
        live, _ = c.rust_tests('const S: &str = "/* #[test] fn in_str() {} */";\n#[test]\nfn after() {}\n')
        self.assertEqual(live, {"after"})

    def test_regression_271_a_test_missing_from_cargos_list_does_not_count(self):
        t = self.tree(recs=[self.rec("sup::it_starts_tripped")], bless=False)
        good = cargo_list(os.path.join(t.d, "list.json"), [("sup", "crates/sup/tests/t.rs", "it_starts_tripped")])
        self.assertEqual(t.run("--bless", "--rust-tests", good)[0], 0)
        empty = cargo_list(os.path.join(t.d, "empty.json"), [("sup", "crates/sup/tests/t.rs", "something_else")])
        rc, out = t.run("--rust-tests", empty)
        self.assertNotEqual(rc, 0)
        self.assertIn("cargo's list", out)

    def test_regression_271_an_ignored_test_in_cargos_list_does_not_count(self):
        t = self.tree(recs=[self.rec("sup::it_starts_tripped")], bless=False)
        lst = cargo_list(os.path.join(t.d, "l.json"), [("sup", "crates/sup/tests/t.rs", "it_starts_tripped", True)])
        rc, out = t.run("--bless", "--rust-tests", lst)
        self.assertNotEqual(rc, 0)
        self.assertIn("only matches ignored", out)

    def test_regression_271_cargo_list_keys_by_crate_and_path(self):
        lst_tests = [("sup", "crates/sup/tests/t.rs", "it_starts_tripped"), ("other", "crates/other/tests/o.rs", "it_starts_tripped")]
        for ref, ok in (("sup::it_starts_tripped", True), ("other::it_starts_tripped", True), ("nope::it_starts_tripped", False),
                        ("crates/sup/tests/t.rs::it_starts_tripped", True), ("crates/other/tests/t.rs::it_starts_tripped", False),
                        ("it_starts_tripped", False)):  # bare and not unique across crates
            t = self.tree(recs=[self.rec(ref)], bless=False,
                          extra={"crates/other/Cargo.toml": '[package]\nname = "other"\n', "crates/other/tests/o.rs": RUST})
            lst = cargo_list(os.path.join(t.d, "l.json"), lst_tests)
            rc, out = t.run("--bless", "--rust-tests", lst)
            self.assertEqual(rc == 0, ok, (ref, out))

    def test_regression_271_a_bare_name_that_is_unique_resolves(self):
        t = self.tree(recs=[self.rec("it_starts_tripped")], bless=False)
        lst = cargo_list(os.path.join(t.d, "l.json"), [("sup", "crates/sup/tests/t.rs", "it_starts_tripped")])
        self.assertEqual(t.run("--bless", "--rust-tests", lst)[0], 0)

    def test_regression_271_globs_need_a_literal_prefix_a_crate_and_no_ignored_match(self):
        tests = [("sup", "crates/sup/tests/t.rs", "regression_9_a"), ("sup", "crates/sup/tests/t.rs", "regression_9_b", True)]
        cases = (("sup::*", "literal prefix"), ("sup::?egression_9_a", "literal prefix"), ("sup::[r]egression_9_a", "literal prefix"),
                 ("regression_9_*", "qualified"), ("sup::regression_9_*", "ignored test"))
        fixture = RUST + "\n#[test]\n#[ignore]\nfn regression_9_b() {}\n"
        for ref, why in cases:
            t = self.tree(recs=[self.rec(ref)], bless=False, rust=fixture)
            lst = cargo_list(os.path.join(t.d, "l.json"), tests)
            rc, out = t.run("--bless", "--rust-tests", lst)
            self.assertNotEqual(rc, 0, ref)
            self.assertIn(why, out, ref)
            # The same rules hold for the source fallback.
            if why != "ignored test":
                self.assertNotEqual(self.tree(recs=[self.rec(ref)], bless=False, rust=fixture).run("--bless")[0], 0, ref)
        t = self.tree(recs=[self.rec("sup::regression_9_a*")], bless=False, rust=fixture)
        lst = cargo_list(os.path.join(t.d, "l.json"), tests[:1])
        self.assertEqual(t.run("--bless", "--rust-tests", lst)[0], 0)

    def test_regression_271_a_malformed_cargo_list_fails_cleanly(self):
        t = self.tree()
        for body in ("not json", "[]", '{"tests": []}', '{"tests": [{"package": 1}]}'):
            t.write("bad.json", body)
            rc, out = t.run("--rust-tests", os.path.join(t.d, "bad.json"))
            self.assertNotEqual(rc, 0, body)
            self.assertNotIn("Traceback", out)

    def test_regression_271_real_cargo_rejects_every_trick(self):
        need_cargo(self)
        refs = ["real_test", "in_nested_comment", "cfg_never_fn", "in_cfg_never_mod", "in_uninvoked_macro", "cfg_attr_ignored", "in_orphan_file"]
        t = self.tree(recs=[record(acc=[("a1", "test", "tiny::" + refs[0], None)])], bless=False)
        t.write("Cargo.toml", '[workspace]\nmembers = ["crates/*"]\nresolver = "2"\n')
        shutil.rmtree(os.path.join(t.d, "crates/sup"))
        t.write("design/names.toml", NAMES.replace('["sup"]', '["tiny"]'))
        t.write("crates/tiny/Cargo.toml", '[package]\nname = "tiny"\nversion = "0.0.0"\nedition = "2021"\n')
        t.write("crates/tiny/src/lib.rs", self.TRICKS)
        t.write("crates/tiny/src/orphan.rs", "#[test]\nfn in_orphan_file() {}\n")  # no `mod orphan;` reaches it
        lister = subprocess.run([sys.executable, "-I", os.path.join(HERE, "rust_test_list.py"), os.path.join(t.d, "l.json"),
                                 "--unlocked", "--root", t.d], capture_output=True, text=True,
                                env=dict(os.environ, CARGO_TARGET_DIR=os.path.join(t.d, "target")))
        self.assertEqual(lister.returncode, 0, lister.stdout + lister.stderr)
        with open(os.path.join(t.d, "l.json")) as fh:
            listed = {x["name"]: x["ignored"] for x in json.load(fh)["tests"]}
        self.assertEqual(listed.get("real_test"), False)
        self.assertEqual(listed.get("cfg_attr_ignored"), True)
        for hidden in ("in_nested_comment", "cfg_never_fn", "in_cfg_never_mod", "in_uninvoked_macro", "in_orphan_file"):
            self.assertNotIn(hidden, listed, hidden)
        lst = os.path.join(t.d, "l.json")
        for ref in refs:
            t.write("design/requirements/cortex.toml", toml_of([record(acc=[("a1", "test", "tiny::" + ref, None)], sha=c.unit_hash(c.source_unit(DESIGN, "The latch starts tripped")))]))
            rc, out = t.run("--bless", "--rust-tests", lst)
            self.assertEqual(rc == 0, ref == "real_test", (ref, out))

    def test_regression_271_a_dead_file_cannot_borrow_a_live_tests_name(self):
        lib = "#[test]\nfn shared() {}\nmod live;\nmod sub;\n"
        files = {"crates/tiny/Cargo.toml": '[package]\nname = "tiny"\n', "crates/tiny/src/lib.rs": lib,
                 "crates/tiny/src/dead.rs": "#[test]\nfn shared() {}\n", "crates/tiny/src/live.rs": "#[test]\nfn in_live_mod() {}\n",
                 "crates/tiny/src/sub/mod.rs": "#[test]\nfn in_sub_mod() {}\n"}
        tests = [("tiny", "crates/tiny/src/lib.rs", "shared"), ("tiny", "crates/tiny/src/lib.rs", "live::in_live_mod"),
                 ("tiny", "crates/tiny/src/lib.rs", "sub::in_sub_mod")]
        for ref, ok in (("crates/tiny/src/lib.rs::shared", True), ("crates/tiny/src/dead.rs::shared", False),
                        ("crates/tiny/src/live.rs::in_live_mod", True), ("crates/tiny/src/sub/mod.rs::in_sub_mod", True),
                        ("crates/tiny/src/dead.rs::in_live_mod", False), ("crates/tiny/src/live.rs::shared", False)):
            t = self.tree(recs=[self.rec(ref)], extra=files, bless=False)
            lst = cargo_list(os.path.join(t.d, "l.json"), tests)
            rc, out = t.run("--bless", "--rust-tests", lst)
            self.assertEqual(rc == 0, ok, (ref, out))

    def test_regression_271_a_non_glob_ref_must_match_exactly_one_test(self):
        src = "mod a {\n#[test]\nfn dup() {}\n}\nmod b {\n#[test]\nfn dup() {}\n}\n"
        files = {"crates/tiny/Cargo.toml": '[package]\nname = "tiny"\n', "crates/tiny/src/lib.rs": src}
        tests = [("tiny", "crates/tiny/src/lib.rs", "a::dup"), ("tiny", "crates/tiny/src/lib.rs", "b::dup")]
        for ref, ok in (("tiny::dup", False), ("crates/tiny/src/lib.rs::dup", False), ("tiny::a::dup", True), ("tiny::b::dup", True)):
            t = self.tree(recs=[self.rec(ref)], extra=files, bless=False)
            rc, out = t.run("--bless", "--rust-tests", cargo_list(os.path.join(t.d, "l.json"), tests))
            self.assertEqual(rc == 0, ok, (ref, out))
            if not ok:
                self.assertIn("different tests", out)

    def test_regression_271_a_forged_test_list_cannot_add_tests_or_clear_ignores(self):
        # A harness = false binary, a build.rs or a proc macro can print any `--list`.
        for name, ignored in (("ghost_test_not_in_source", False), ("skipped_one", False), ("only_in_a_comment", False)):
            t = self.tree(recs=[self.rec(f"sup::{name}")], bless=False)
            lst = cargo_list(os.path.join(t.d, "l.json"), [("sup", "crates/sup/tests/t.rs", name, ignored)])
            rc, out = t.run("--bless", "--rust-tests", lst)
            self.assertNotEqual(rc, 0, name)
        t = self.tree(recs=[self.rec("sup::it_starts_tripped")], bless=False)
        lst = cargo_list(os.path.join(t.d, "l.json"), [("sup", "crates/sup/tests/t.rs", "it_starts_tripped")])
        self.assertEqual(t.run("--bless", "--rust-tests", lst)[0], 0)

    def test_regression_271_examples_and_benches_are_not_listed(self):
        with open(os.path.join(HERE, "rust_test_list.py")) as fh:
            src = fh.read()
        cmd = [l for l in src.splitlines() if l.strip().startswith("cmd = [")][0]
        self.assertNotIn("--all-targets", cmd)  # ci.yml runs `cargo test --workspace --locked`
        need_cargo(self)
        t = self.tree(bless=False)
        shutil.rmtree(os.path.join(t.d, "crates/sup"))
        t.write("Cargo.toml", '[workspace]\nmembers = ["crates/*"]\nresolver = "2"\n')
        t.write("crates/tiny/Cargo.toml", '[package]\nname = "tiny"\nversion = "0.0.0"\nedition = "2021"\n')
        t.write("crates/tiny/src/lib.rs", "#[test]\nfn in_lib() {}\n")
        t.write("crates/tiny/examples/ex.rs", "fn main() {}\n#[test]\nfn in_example() {}\n")
        t.write("crates/tiny/benches/b.rs", "#[test]\nfn in_bench() {}\n")
        r = subprocess.run([sys.executable, "-I", os.path.join(HERE, "rust_test_list.py"), os.path.join(t.d, "l.json"), "--unlocked",
                            "--root", t.d], capture_output=True, text=True, env=dict(os.environ, CARGO_TARGET_DIR=os.path.join(t.d, "target")))
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
        with open(os.path.join(t.d, "l.json")) as fh:
            names = {x["name"] for x in json.load(fh)["tests"]}
        self.assertEqual(names, {"in_lib"})

    def test_regression_271_class_level_skips_are_honoured(self):
        src = ("import unittest\n"
               "@unittest.skip('x')\n"
               "class Off(unittest.TestCase):\n    def test_in_skipped_class(self): pass\n"
               "class Sub(Off):\n    def test_in_subclass_of_skipped_class(self): pass\n"
               "@unittest.skipIf(True, 'x')\n"
               "class Off2(unittest.TestCase):\n    def test_in_skipif_class(self): pass\n"
               "class On(unittest.TestCase):\n    def test_live(self): pass\n")
        self.assertEqual(c.python_tests(src), {"test_live"})

    def test_regression_271_the_unit_tests_run_after_the_judge(self):
        with open(os.path.join(ROOT, ".github/workflows/conformance.yml")) as fh:
            wf = fh.read()
        job = wf[wf.index("\n  conformance:"):wf.index("\n  closure-gate:")]
        self.assertLess(job.index("Check the records with the base branch copy"), job.index("Unit-test the checker"))

    def test_regression_271_a_directory_where_a_file_belongs_is_a_clean_error(self):
        for rel in ("design/STATUS.md", "docs/NAMES.md", "design/names.toml"):
            t = self.tree()
            os.remove(os.path.join(t.d, rel))
            os.makedirs(os.path.join(t.d, rel))
            rc, out = t.run()
            self.assertNotIn("Traceback", out, rel)
            self.assertNotEqual(rc, 0, rel)

    def scratch_tiny(self, lib, extra=None, manifest_extra=""):
        t = self.tree(bless=False)
        shutil.rmtree(os.path.join(t.d, "crates/sup"))
        t.write("design/names.toml", NAMES.replace('["sup"]', '["tiny"]'))
        t.write("Cargo.toml", '[workspace]\nmembers = ["crates/*"]\nresolver = "2"\n')
        t.write("crates/tiny/Cargo.toml", '[package]\nname = "tiny"\nversion = "0.0.0"\nedition = "2021"\n' + manifest_extra)
        t.write("crates/tiny/src/lib.rs", lib)
        for path, text in (extra or {}).items():
            t.write(path, text)
        r = subprocess.run([sys.executable, "-I", os.path.join(HERE, "rust_test_list.py"), os.path.join(t.d, "l.json"), "--unlocked",
                            "--root", t.d], capture_output=True, text=True, env=dict(os.environ, CARGO_TARGET_DIR=os.path.join(t.d, "target")))
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
        with open(os.path.join(t.d, "l.json")) as fh:
            return t, json.load(fh)

    def judge(self, t, ref):
        t.write("design/requirements/cortex.toml", toml_of([record(acc=[("a1", "test", ref, None)],
                sha=c.unit_hash(c.source_unit(DESIGN, "The latch starts tripped")))]))
        return t.run("--bless", "--rust-tests", os.path.join(t.d, "l.json"))

    def test_regression_271_a_dead_file_cannot_borrow_an_inline_modules_test_or_a_child_module(self):
        files = {
            "crates/tiny/Cargo.toml": '[package]\nname = "tiny"\n',
            "crates/tiny/src/lib.rs": "mod a {\n    #[test]\n    fn borrowed() {}\n    mod sub;\n}\nmod b;\n",
            "crates/tiny/src/a.rs": "#[test]\nfn borrowed() {}\n#[test]\nfn deep() {}\n",  # dead: `mod a` is inline
            "crates/tiny/src/a/sub.rs": "#[test]\nfn deep() {}\n",
            "crates/tiny/src/b.rs": "mod c;\n#[cfg(feature = \"never\")]\n#[test]\nfn deep2() {}\n",
            "crates/tiny/src/b/c.rs": "#[test]\nfn deep2() {}\n",
        }
        lib = "crates/tiny/src/lib.rs"
        tests = [("tiny", lib, "a::borrowed"), ("tiny", lib, "a::sub::deep"), ("tiny", lib, "b::c::deep2")]
        for ref, ok in ((f"{lib}::a::borrowed", True), ("crates/tiny/src/a.rs::borrowed", False), ("crates/tiny/src/a.rs::deep", False),
                        ("crates/tiny/src/a/sub.rs::deep", True), ("crates/tiny/src/b.rs::deep2", False),
                        ("crates/tiny/src/b/c.rs::deep2", True), ("crates/tiny/src/a.rs::a::borrowed", False)):
            t = self.tree(recs=[self.rec(ref)], extra=files, bless=False)
            rc, out = t.run("--bless", "--rust-tests", cargo_list(os.path.join(t.d, "l.json"), tests))
            self.assertEqual(rc == 0, ok, (ref, out))

    def test_regression_271_harness_false_targets_are_skipped(self):
        need_cargo(self)
        forge = ("fn main() {\n    let a: Vec<String> = std::env::args().collect();\n"
                 "    if a.iter().any(|x| x == \"--list\") && !a.iter().any(|x| x == \"--ignored\") {\n"
                 "        println!(\"ghost: test\");\n    }\n}\n#[test]\nfn ghost() {}\n")
        t, listed = self.scratch_tiny("#[test]\nfn real() {}\n", {"crates/tiny/tests/forge.rs": forge},
                                      '\n[[test]]\nname = "forge"\npath = "tests/forge.rs"\nharness = false\n')
        self.assertEqual({x["name"] for x in listed["tests"]}, {"real"})
        self.assertEqual(listed["skipped_harness_false"], ["tiny:forge"])
        self.assertNotEqual(self.judge(t, "tiny::ghost")[0], 0)
        self.assertEqual(self.judge(t, "tiny::real")[0], 0)

    def test_regression_271_harness_false_examples_and_benches_are_skipped_and_refused(self):
        need_cargo(self)
        forge = ("fn main() {\n    let a: Vec<String> = std::env::args().collect();\n"
                 "    if a.iter().any(|x| x == \"--list\") && !a.iter().any(|x| x == \"--ignored\") {\n"
                 "        println!(\"ghost: test\");\n    }\n}\n#[test]\nfn ghost() {}\n")
        extra = {"crates/tiny/examples/ex.rs": forge, "crates/tiny/benches/bn.rs": forge}
        manifest = ('\n[[example]]\nname = "ex"\npath = "examples/ex.rs"\nharness = false\ntest = true\n'
                    '\n[[bench]]\nname = "bn"\npath = "benches/bn.rs"\nharness = false\n')
        t, listed = self.scratch_tiny("#[test]\nfn real() {}\n", extra, manifest)
        self.assertEqual({x["name"] for x in listed["tests"]}, {"real"})
        # `cargo test` does not build benches, so only the example is ever seen by the lister.
        self.assertIn("tiny:ex", listed["skipped_harness_false"])
        # Even a forged list that includes the forged entries is refused by the judge,
        # which reads the manifests itself.
        for src in ("crates/tiny/examples/ex.rs", "crates/tiny/benches/bn.rs"):
            forged = cargo_list(os.path.join(t.d, "f.json"), [("tiny", src, "ghost")])
            t.write("design/requirements/cortex.toml", toml_of([record(acc=[("a1", "test", "tiny::ghost", None)],
                    sha=c.unit_hash(c.source_unit(DESIGN, "The latch starts tripped")))]))
            self.assertNotEqual(t.run("--bless", "--rust-tests", forged)[0], 0, src)
        self.assertEqual(c.harness_false_srcs(t.d, "crates/tiny"),
                         {"crates/tiny/examples/ex.rs", "crates/tiny/benches/bn.rs"})

    def test_regression_271_dead_or_ambiguous_module_declarations_earn_no_credit(self):
        lib = "crates/tiny/src/lib.rs"
        cases = {
            "macro-swallowed mod": ("macro_rules! m { ($($t:tt)*) => {}; }\nm! { mod ghost; }\n", {"crates/tiny/src/ghost.rs": "#[test]\nfn t() {}\n"}, "ghost::t", "crates/tiny/src/ghost.rs::t"),
            "duplicate mod": ("mod d;\nmod d;\n", {"crates/tiny/src/d.rs": "#[test]\nfn t() {}\n"}, "d::t", "crates/tiny/src/d.rs::t"),
            "path attribute": ("#[path = \"other.rs\"]\nmod p;\n", {"crates/tiny/src/p.rs": "#[test]\nfn t() {}\n", "crates/tiny/src/other.rs": "#[test]\nfn t() {}\n"}, "p::t", "crates/tiny/src/p.rs::t"),
            "cfg_attr path": ("#[cfg_attr(unix, path = \"other.rs\")]\nmod p;\n", {"crates/tiny/src/p.rs": "#[test]\nfn t() {}\n", "crates/tiny/src/other.rs": "#[test]\nfn t() {}\n"}, "p::t", "crates/tiny/src/p.rs::t"),
        }
        for name, (libsrc, files, full, ref) in cases.items():
            files = dict(files)
            files[lib] = libsrc
            t = self.tree(recs=[self.rec(ref)], extra=files, bless=False)
            rc, out = t.run("--bless", "--rust-tests", cargo_list(os.path.join(t.d, "l.json"), [("tiny", lib, full)]))
            self.assertNotEqual(rc, 0, (name, out))
        # The same full test name reached from two files is ambiguous: refused.
        files = {lib: "mod x;\nmod y;\n", "crates/tiny/src/x.rs": "#[test]\nfn t() {}\n", "crates/tiny/src/y.rs": "#[test]\nfn t() {}\n"}
        t = self.tree(recs=[self.rec("crates/tiny/src/x.rs::t")], extra=files, bless=False)
        self.assertEqual(t.run("--bless", "--rust-tests", cargo_list(os.path.join(t.d, "l.json"), [("tiny", lib, "x::t"), ("tiny", lib, "y::t")]))[0], 0)

    def test_regression_271_module_declarations_survive_spacing_raw_names_and_inline_scopes(self):
        # i: a space inside the visibility parens must not detach the attributes.
        i = c.scan_rust('#[path = "real_i.rs"]\npub (crate) mod i;\n')
        self.assertEqual((i["mods"], i["tainted"]), ([], None))
        # j: a raw identifier is the same module name as the plain one (duplicate: refused).
        j = c.scan_rust('#[cfg(feature = "never")]\nmod j;\n#[cfg(not(feature = "never"))]\n#[path = "real_j.rs"]\nmod r#j;\n')
        self.assertEqual(j["mods"], [])
        # k: #[path] on an inline module redirects its children: refuse the module and what is inside it.
        k = c.scan_rust('#[path = "elsewhere"]\nmod k {\n    mod kid;\n    #[test]\n    fn t() {}\n}\n')
        self.assertEqual((k["mods"], k["tests"]), ([], []))
        # l: a cfg'd-out `mod l;` next to a live inline `mod l {}` is a duplicate in the parent scope.
        l = c.scan_rust('#[cfg(feature = "never")]\nmod l;\n#[cfg(not(feature = "never"))]\nmod l {\n    #[test]\n    fn t() {}\n}\n')
        self.assertEqual((l["mods"], l["tests"]), ([], []))
        two = c.scan_rust("mod m {\n    #[test]\n    fn a() {}\n}\nmod m {\n    #[test]\n    fn b() {}\n}\n")
        self.assertEqual(two["tests"], [])
        # What stays counted: spacing and raw names are understood, and an inline module is keyed in its parent.
        ok = c.scan_rust("pub (crate) mod z;\nmod r#raw;\nmod ok {\n    #[test]\n    fn t() {}\n    mod deep;\n}\n")
        self.assertEqual([(m[0], m[1]) for m in ok["mods"]], [([], "z"), ([], "raw"), (["ok"], "deep")])
        self.assertEqual(ok["tests"], [(["ok"], "t", False)])
        sp = c.scan_rust("pub  ( in crate::x )  fn_not_here;\n#[test]\npub (crate) fn spaced_vis() {}\n")
        self.assertEqual([t[1] for t in sp["tests"]], ["spaced_vis"])

    def test_regression_271_a_file_with_an_unparsed_mod_or_an_include_credits_nothing(self):
        for name, src in (("mod without a space after the visibility", "pub(crate)mod q;\n#[test]\nfn t() {}\n"),
                          ("odd mod item", "mod 'x;\n#[test]\nfn t() {}\n"),
                          ("include!", 'include!("x.rs");\n#[test]\nfn t() {}\n'),
                          ("include! with spacing", 'include ! ("x.rs");\n#[test]\nfn t() {}\n')):
            info = c.scan_rust(src)
            self.assertTrue(info["tainted"], name)
            self.assertEqual((info["tests"], info["mods"]), ([], []), name)
        # A `mod` or `include!` inside a comment or a string is not code.
        for src in ('// mod x;\n// include!("a");\n#[test]\nfn t() {}\n', 'const S: &str = "mod x; include!(a)";\n#[test]\nfn t() {}\n'):
            info = c.scan_rust(src)
            self.assertEqual((info["tainted"], info["macro_hazard"], len(info["tests"])), (None, None, 1), src)
        # ...but one inside a macro_rules! body IS code that can declare a module: it is a hazard.
        for body in ("mod x;", "#[path = \"y.rs\"] mod x;", 'include!("a");', "mod $name { }"):
            info = c.scan_rust("macro_rules! m { ($name:ident) => { " + body + " } }\n#[test]\nfn t() {}\n")
            self.assertTrue(info["macro_hazard"], body)
        # A macro_rules! body without them is fine, as before.
        self.assertIsNone(c.scan_rust("macro_rules! ig { ($i:item) => { #[ignore] $i }; }\n#[test]\nfn t() {}\n")["macro_hazard"])
        # End to end: a tainted file and its `mod` children earn no credit and the error says why.
        lib = "crates/tiny/src/lib.rs"
        files = {lib: 'mod child;\ninclude!("x.rs");\n#[test]\nfn own() {}\n', "crates/tiny/src/child.rs": "#[test]\nfn kid() {}\n"}
        for ref, full in ((f"{lib}::own", "own"), ("crates/tiny/src/child.rs::kid", "child::kid")):
            t = self.tree(recs=[self.rec(ref)], extra=files, bless=False)
            rc, out = t.run("--bless", "--rust-tests", cargo_list(os.path.join(t.d, "l.json"), [("tiny", lib, full)]))
            self.assertNotEqual(rc, 0, ref)
            self.assertIn("include!", out, ref)

    def test_regression_271_a_macro_that_declares_a_module_refuses_every_rust_credit(self):
        lib = "crates/tiny/src/lib.rs"
        ws7 = ('// The scanner sees only the cfg\'d-out declaration; the live module comes from a macro.\n'
               '#[cfg(feature = "never")]\nmod m;\n'
               'macro_rules! live_m { () => { #[cfg(not(feature = "never"))] mod m { #[test] fn tm() {} } }; }\nlive_m!();\n'
               '#[cfg(feature = "never")]\nmod n;\n'
               'macro_rules! live_n { () => { #[cfg(not(feature = "never"))] #[path = "real_n.rs"] mod n; }; }\nlive_n!();\n'
               '#[test]\nfn plain() {}\n')
        files = {lib: ws7, "crates/tiny/src/m.rs": "#[test]\nfn tm() {}\n", "crates/tiny/src/n.rs": "#[test]\nfn tn() {}\n",
                 "crates/tiny/src/real_n.rs": "#[test]\nfn tn() {}\n"}
        tests = [("tiny", lib, "m::tm"), ("tiny", lib, "n::tn"), ("tiny", lib, "plain")]
        for ref in ("crates/tiny/src/m.rs::tm", "crates/tiny/src/n.rs::tn", "tiny::plain", f"{lib}::plain"):
            t = self.tree(recs=[self.rec(ref)], extra=files, bless=False)
            for args in (("--rust-tests", cargo_list(os.path.join(t.d, "l.json"), tests)), ()):
                rc, out = t.run("--bless", *args)
                self.assertNotEqual(rc, 0, (ref, args))
                self.assertIn("macro_rules!", out, (ref, args))
                self.assertIn("every Rust test is refused", out, (ref, args))
        # The hazard is workspace-wide: a macro in an unrelated crate refuses tests everywhere...
        t = self.tree(recs=[self.rec("sup::it_starts_tripped")], bless=False,
                      extra={"crates/other/src/lib.rs": "macro_rules! m { () => { mod x; }; }\n"})
        lst = cargo_list(os.path.join(t.d, "l.json"), [("sup", "crates/sup/tests/t.rs", "it_starts_tripped")])
        self.assertNotEqual(t.run("--bless", "--rust-tests", lst)[0], 0)
        # ...and Python refs are not affected by it.
        t.write("design/requirements/cortex.toml", toml_of([record(acc=[("a1", "test", "ci/test_conformance.py::test_x", None)],
                sha=c.unit_hash(c.source_unit(DESIGN, "The latch starts tripped")))]))
        t.write("ci/test_conformance.py", "import unittest\nclass T(unittest.TestCase):\n    def test_x(self): pass\n")
        self.assertEqual(t.run("--bless", "--rust-tests", lst)[0], 0)

    def test_regression_271_any_include_identifier_taints_the_file(self):
        for name, src in (("alias", 'use std::include as inc;\n#[cfg(feature = "never")]\nmod b;\ninc!("live_b.rs");\n#[test]\nfn t() {}\n'),
                          ("core path", 'use core::include;\ninclude!("x.rs");\n#[test]\nfn t() {}\n'),
                          ("raw identifier", 'r#include!("x.rs");\n#[test]\nfn t() {}\n'),
                          ("macro body", 'macro_rules! m { () => { use std::include as inc; }; }\n#[test]\nfn t() {}\n')):
            info = c.scan_rust(src)
            self.assertTrue(info["tainted"] or info["macro_hazard"], name)
            if name != "macro body":
                self.assertEqual((info["tests"], info["mods"]), ([], []), name)
        # `include_str!` and friends are different identifiers; comments and strings are not code.
        for src in ('const A: &str = include_str!("x.txt");\n#[test]\nfn t() {}\n', '// include\nconst S: &str = "include";\n#[test]\nfn t() {}\n'):
            info = c.scan_rust(src)
            self.assertEqual((info["tainted"], info["macro_hazard"], len(info["tests"])), (None, None, 1), src)

    def test_regression_271_the_hazard_scan_covers_every_rust_file_in_the_repo(self):
        lib = "crates/tiny/src/lib.rs"
        files = {lib: '#[cfg(feature = "never")]\nmod a;\nhelper::live_a!();\n#[test]\nfn plain() {}\n',
                 "crates/tiny/src/a.rs": "#[test]\nfn ta() {}\n",
                 "research/helper/src/lib.rs": "#[macro_export]\nmacro_rules! live_a { () => { #[cfg(not(feature = \"never\"))] mod a { #[test] fn ta() {} } }; }\n"}
        for ref, full in (("crates/tiny/src/a.rs::ta", "a::ta"), ("tiny::plain", "plain")):
            t = self.tree(recs=[self.rec(ref)], extra=files, bless=False)
            for args in (("--rust-tests", cargo_list(os.path.join(t.d, "l.json"), [("tiny", lib, full)])), ()):
                rc, out = t.run("--bless", *args)
                self.assertNotEqual(rc, 0, (ref, args))
                self.assertIn("research/helper/src/lib.rs", out, (ref, args))
        # Build output is not source: a `target/` directory is skipped.
        t = self.tree(recs=[self.rec("sup::it_starts_tripped")], bless=False,
                      extra={"target/debug/build/x/out.rs": "macro_rules! m { () => { mod x; }; }\n"})
        lst = cargo_list(os.path.join(t.d, "l.json"), [("sup", "crates/sup/tests/t.rs", "it_starts_tripped")])
        self.assertEqual(t.run("--bless", "--rust-tests", lst)[0], 0)

    def test_regression_271_a_proc_macro_crate_refuses_every_rust_credit(self):
        for key in ("proc-macro", "proc_macro"):
            t = self.tree(recs=[self.rec("sup::it_starts_tripped")], bless=False,
                          extra={"crates/pm/Cargo.toml": f'[package]\nname = "pm"\n[lib]\n{key} = true\n'})
            lst = cargo_list(os.path.join(t.d, "l.json"), [("sup", "crates/sup/tests/t.rs", "it_starts_tripped")])
            for args in (("--rust-tests", lst), ()):
                rc, out = t.run("--bless", *args)
                self.assertNotEqual(rc, 0, (key, args))
                self.assertIn("proc-macro", out)
        t = self.tree(recs=[self.rec("sup::it_starts_tripped")], bless=False,
                      extra={"crates/pm/Cargo.toml": '[package]\nname = "pm"\n[lib]\nproc-macro = false\n'})
        self.assertEqual(t.run("--bless", "--rust-tests", lst)[0], 0)

    def test_regression_271_the_lister_survives_a_proc_macro_test_binary_and_the_judge_refuses_it(self):
        need_cargo(self)
        pm = ("use proc_macro::TokenStream;\n#[proc_macro]\npub fn noop(_: TokenStream) -> TokenStream { TokenStream::new() }\n"
              "#[test]\nfn unit() {}\n")
        t, listed = self.scratch_tiny(pm, manifest_extra="\n[lib]\nproc-macro = true\n")
        self.assertEqual({x["name"] for x in listed["tests"]}, {"unit"})  # listed without crashing...
        self.assertNotEqual(self.judge(t, "tiny::unit")[0], 0)            # ...but not credited

    def test_regression_271_a_failed_lister_fails_both_gates_instead_of_skipping_them(self):
        with open(os.path.join(ROOT, ".github/workflows/conformance.yml")) as fh:
            wf = fh.read()
        for job, cond in (("conformance", "if: always()\n"), ("closure-gate", "if: always() && github.event_name == 'pull_request'\n")):
            start = wf.index(f"\n  {job}:")
            nxt = wf.find("\n  closure-gate:", start + 1) if job == "conformance" else len(wf)
            body = wf[start:nxt]
            self.assertIn("needs: [test-list, python-test-list]", body, job)
            self.assertIn(cond, body, job)
            # The first step checks both lister results, passed through env (never `${{ }}` inside `run:`).
            first = body[body.index("    steps:"):]
            self.assertLess(first.index("needs.test-list.result"), first.index("actions/checkout"), job)
            self.assertIn("needs.python-test-list.result", first, job)
            self.assertIn('!= "success"', first, job)
            self.assertIn("exit 1", first, job)
        for block in re.findall(r"run: \|\n((?: {10}.*\n|\n)+)", wf):
            self.assertNotIn("${{", block)

    def test_regression_271_generator_and_misplaced_async_tests_never_run(self):
        names, reasons = c.python_scan(
            "import unittest\nclass T(unittest.TestCase):\n    def test_gen(self):\n        yield\n        raise AssertionError\n"
            "    async def test_coro(self): raise AssertionError\n    def test_ok(self):\n        def inner():\n            yield 1\n"
            "class A(unittest.IsolatedAsyncioTestCase):\n    async def test_async_ok(self): pass\n    async def test_async_gen(self):\n        yield\n"
            "class B(A):\n    async def test_inherits_async(self): pass\n")
        self.assertEqual(names, {"test_ok", "test_async_ok", "test_inherits_async"})
        self.assertEqual({(line, text.split("(")[0]) for line, text, _ in reasons},
                         {(3, "def test_gen"), (6, "async def test_coro"), (12, "async def test_async_gen")})

    def test_regression_271_spaced_and_macro_added_ignores_are_ignores(self):
        need_cargo(self)
        for src in ("# [ignore]\n#[test]\nfn x() {}\n", "#[test]\n#  [ ignore ]\nfn x() {}\n", "# [test]\n# [ignore = \"r\"]\nfn x() {}\n"):
            self.assertEqual(c.rust_tests(src), (set(), {"x"}), src)
        self.assertEqual(c.rust_tests("#  [ test ]\nfn x() {}\n"), ({"x"}, set()))
        lib = ("# [ignore]\n#[test]\nfn spaced_ignored() {}\n"
               "macro_rules! ig { ($i:item) => { #[ignore] $i }; }\nig! {\n    #[test]\n    fn macro_ignored() {}\n}\n")
        t, listed = self.scratch_tiny(lib)
        ignored = {x["name"]: x["ignored"] for x in listed["tests"]}
        self.assertEqual(ignored, {"spaced_ignored": True, "macro_ignored": True})  # cargo knows the macro-added one
        self.assertNotEqual(self.judge(t, "tiny::spaced_ignored")[0], 0)
        self.assertNotEqual(self.judge(t, "tiny::macro_ignored")[0], 0)
        # A forged list that clears the flag cannot clear the source-visible ignore.
        forged = cargo_list(os.path.join(t.d, "f.json"), [("tiny", "crates/tiny/src/lib.rs", "spaced_ignored", False)])
        t.write("design/requirements/cortex.toml", toml_of([record(acc=[("a1", "test", "tiny::spaced_ignored", None)],
                sha=c.unit_hash(c.source_unit(DESIGN, "The latch starts tripped")))]))
        self.assertNotEqual(t.run("--bless", "--rust-tests", forged)[0], 0)

    def test_regression_271_sub_bullets_are_part_of_a_pinned_bullet(self):
        doc = "- Parent bullet that is pinned.\n  continuation line\n  - child one\n    - grandchild\n- Sibling.\n\nnext paragraph\n"
        base = c.unit_hash(c.source_unit(doc, "Parent bullet"))
        for edited in (doc.replace("  - child one\n", "  - child one\n  - Except in test mode, it may be skipped.\n"),
                       doc.replace("grandchild", "grandchild edited"), doc.replace("continuation line", "continuation line 2")):
            self.assertNotEqual(c.unit_hash(c.source_unit(edited, "Parent bullet")), base)
        self.assertEqual(c.unit_hash(c.source_unit(doc.replace("- Sibling.", "- Sibling, changed."), "Parent bullet")), base)
        self.assertEqual(c.unit_hash(c.source_unit(doc.replace("next paragraph", "other"), "Parent bullet")), base)
        # A pinned child is just the child.
        self.assertEqual(c.source_unit(doc, "child one"), "  - child one\n    - grandchild")
        # Blank lines inside a bullet keep deeper-indented text, and stop at shallower text.
        loose = "- Pinned.\n\n  more text under it\n\nafter\n"
        self.assertEqual(c.source_unit(loose, "Pinned"), "- Pinned.\n\n  more text under it")

    def test_regression_271_python_last_definition_wins_and_skips_cannot_hide(self):
        src = (
            "import unittest\nimport unittest as ut\nfrom unittest import TestCase as TC\n"
            "off = unittest.skip('x')\n"
            "class A(unittest.TestCase):\n"
            "    def test_redefined_live(self): pass\n"
            "    @unittest.skip('x')\n    def test_redefined_live(self): pass\n"   # last definition is skipped
            "    @unittest.skip('x')\n    def test_revived(self): pass\n"
            "    def test_revived(self): pass\n"                                   # last definition is live
            "    def test_deleted(self): pass\n    del test_deleted\n"
            "    def test_reassigned(self): pass\n    test_reassigned = None\n"
            "    @off\n    def test_alias_skipped(self): pass\n"
            "    def test_plain(self): pass\n"
            "class B(unittest.TestCase):\n    __unittest_skip__ = True\n    def test_in_flagged_class(self): pass\n"
            "class D(ut.TestCase):\n    def test_via_alias_import(self): pass\n"
            "class E(TC):\n    def test_via_from_import(self): pass\n"
            "class F(A):\n    def test_subclass_of_a_counted_class(self): pass\n"
        )
        self.assertEqual(c.python_tests(src), {"test_revived", "test_plain", "test_via_alias_import", "test_via_from_import",
                                               "test_subclass_of_a_counted_class"})
        # A decorator other than a unittest skip runs code at import: the module is not counted.
        self.assertEqual(c.python_tests("import unittest\n@some.decorator\nclass C(unittest.TestCase):\n    def test_x(self): pass\n"), set())
        self.assertEqual(c.python_tests("import unittest\nclass C(unittest.TestCase):\n    @some.decorator\n    def test_x(self): pass\n"), set())
        self.assertEqual(c.python_tests("import unittest\ndef load_tests(l, t, p): return t\nclass A(unittest.TestCase):\n    def test_x(self): pass\n"), set())
        # TestCase must come from unittest, not from anywhere named TestCase.
        self.assertEqual(c.python_tests("from other import TestCase\nclass A(TestCase):\n    def test_x(self): pass\n"), set())
        self.assertEqual(c.python_tests("class A(other.TestCase):\n    def test_x(self): pass\n"), set())

    HOSTILE_PY = {
        "p1 guard that is not the main guard": "import unittest\nclass T(unittest.TestCase):\n    def test_x(self): pass\n"
                                               "if __name__ != '__main__':\n    raise unittest.SkipTest('x')\n",
        "p2 side effect in sys.path args": "import sys\nimport unittest\nsys.path.insert(0, exec('raise unittest.SkipTest()'))\n"
                                           "class T(unittest.TestCase):\n    def test_x(self): pass\n",
        "p3 setattr on TestCase.run": "import unittest\n_ = setattr(unittest.TestCase, 'run', lambda self, result=None: None)\n"
                                      "class T(unittest.TestCase):\n    def test_x(self): pass\n",
        "p4 setattr replaces TestCase": "import unittest\n_ = setattr(unittest, 'TestCase', type('TestCase', (), {}))\n"
                                        "from unittest import TestCase\nclass T(TestCase):\n    def test_x(self): pass\n",
        "p5 run overridden": "import unittest\nclass T(unittest.TestCase):\n    def run(self, result=None):\n        return None\n"
                             "    def test_x(self): pass\n",
        "p6 setUp skips": "import unittest\nclass T(unittest.TestCase):\n    def setUp(self):\n        self.skipTest('x')\n"
                          "    def test_x(self): pass\n",
        "p7 setUpClass hides a skip": "import unittest\nclass T(unittest.TestCase):\n    @classmethod\n    def setUpClass(cls):\n"
                                      "        raise getattr(unittest, 'Skip' + 'Test')('x')\n    def test_x(self): pass\n",
        "p8 metaclass strips tests": "import unittest\nclass Strip(type(unittest.TestCase)):\n    pass\n"
                                     "class T(unittest.TestCase, metaclass=Strip):\n    def test_x(self): pass\n",
        "metaclass keyword alone": "import unittest\nclass T(unittest.TestCase, metaclass=type):\n    def test_x(self): pass\n",
        "main guard with arguments": "import unittest\nclass T(unittest.TestCase):\n    def test_x(self): pass\n"
                                     "if __name__ == '__main__':\n    unittest.main(verbosity=2)\n",
        "main guard with an else": "import unittest\nclass T(unittest.TestCase):\n    def test_x(self): pass\n"
                                   "if __name__ == '__main__':\n    unittest.main()\nelse:\n    pass\n",
        "main guard with two statements": "import unittest\nclass T(unittest.TestCase):\n    def test_x(self): pass\n"
                                          "if __name__ == '__main__':\n    unittest.main()\n    print('x')\n",
        "spec from a path outside ci": "import importlib.util\nimport unittest\nspec = importlib.util.spec_from_file_location('x', '/tmp/evil.py')\n"
                                       "m = importlib.util.module_from_spec(spec)\nspec.loader.exec_module(m)\n"
                                       "class T(unittest.TestCase):\n    def test_x(self): pass\n",
        "exec_module of something else": "import unittest\nimport evil\nevil.spec.loader.exec_module(evil)\n"
                                         "class T(unittest.TestCase):\n    def test_x(self): pass\n",
        "default argument runs code": "import unittest\ndef f(x=exec('pass')): pass\nclass T(unittest.TestCase):\n    def test_x(self): pass\n",
        "from import star": "import unittest\nfrom helpers import *\nclass T(unittest.TestCase):\n    def test_x(self): pass\n",
    }

    def test_regression_271_python_modules_are_fail_closed(self):
        for name, src in self.HOSTILE_PY.items():
            names, reasons = c.python_scan(src)
            self.assertEqual(names, set(), name)
            self.assertTrue(reasons and reasons[0][0] >= 1 and reasons[0][2], name)  # a line number and a reason
        ok = ("import os\nimport sys\nimport unittest\nfrom pathlib import Path\nimport importlib.util\n"
              "HERE = os.path.dirname(os.path.abspath(__file__))\nsys.path.insert(0, HERE)\nGUARD = Path(__file__).with_name('g.sh').resolve()\n"
              "spec = importlib.util.spec_from_file_location('g', Path(__file__).with_name('g.py'))\n"
              "g = importlib.util.module_from_spec(spec)\nspec.loader.exec_module(g)\n"
              "class T(unittest.TestCase):\n    def test_x(self): pass\n"
              "if __name__ == '__main__':\n    unittest.main()\n")
        self.assertEqual(c.python_scan(ok), ({"test_x"}, []))

    def test_regression_271_python_classes_that_change_what_runs_are_not_counted(self):
        for hook in ("run", "__call__", "debug", "setUp", "setUpClass", "asyncSetUp", "__init__", "__new__", "__init_subclass__", "tearDown"):
            src = (f"import unittest\nclass Bad(unittest.TestCase):\n    def {hook}(self, *a, **k): pass\n    def test_hidden(self): pass\n"
                   "class Sub(Bad):\n    def test_inherits_the_hook(self): pass\n"
                   "class Good(unittest.TestCase):\n    def test_fine(self): pass\n")
            names, reasons = c.python_scan(src)
            self.assertEqual(names, {"test_fine"}, hook)
            self.assertTrue(any(line == 3 and hook in text for line, text, _ in reasons), (hook, reasons))
        names, reasons = c.python_scan("import unittest\nclass T(unittest.TestCase):\n    run = None\n    def test_x(self): pass\n")
        self.assertEqual(names, set())
        self.assertEqual(reasons[0][:2], (3, "run = None"))

    def test_regression_271_the_real_ci_test_files_count(self):
        # The importlib idiom in these two files must not make the whole module uncountable.
        for rel in ("ci/test_check_standards.py", "ci/test_check_typosquat.py", "ci/test_no_build_artifacts.py",
                    "ci/test_regression_guard.py", "ci/test_conformance.py"):
            with open(os.path.join(ROOT, rel), encoding="utf-8") as fh:
                text = fh.read()
            names, reasons = c.python_scan(text)
            self.assertEqual((len(names), reasons), (len(re.findall(r"^    def test_\w+", text, re.M)), []), rel)

    def test_regression_271_the_runtime_list_must_agree_with_the_static_scan(self):
        body = ("import unittest\nclass T(unittest.TestCase):\n    def test_passes(self): pass\n"
                "    def test_fails(self): raise AssertionError('x')\n    def test_skips_at_runtime(self): self.skipTest('x')\n"
                "    def test_errors(self): raise RuntimeError('x')\n")
        hostile = self.HOSTILE_PY["p3 setattr on TestCase.run"]
        t = self.tree(recs=[self.rec("ci/test_live.py::test_passes")], bless=False,
                      extra={"ci/test_live.py": body})

        def listed(out):
            r = subprocess.run([sys.executable, "-I", os.path.join(HERE, "py_test_list.py"), out, "--root", t.d], capture_output=True, text=True)
            self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
            with open(out) as fh:
                return {(x["file"], x["name"]): x["outcome"] for x in json.load(fh)["tests"]}

        out = os.path.join(t.d, "py.json")
        rows = listed(out)
        self.assertEqual(rows[("ci/test_live.py", "test_passes")], "passed")
        self.assertEqual(rows[("ci/test_live.py", "test_fails")], "failed")
        self.assertEqual(rows[("ci/test_live.py", "test_skips_at_runtime")], "skipped")
        self.assertEqual(rows[("ci/test_live.py", "test_errors")], "errored")
        for ref, ok in (("ci/test_live.py::test_passes", True), ("ci/test_live.py::test_fails", False),
                        ("ci/test_live.py::test_skips_at_runtime", False), ("ci/test_live.py::test_errors", False)):
            t.write("design/requirements/cortex.toml", toml_of([record(acc=[("a1", "test", ref, None)],
                    sha=c.unit_hash(c.source_unit(DESIGN, "The latch starts tripped")))]))
            rc, text = t.run("--bless", "--py-tests", out)
            self.assertEqual(rc == 0, ok, (ref, text))
        # P3: one file replaces TestCase.run; the whole real run then reports no passes. The static
        # scan refuses that file anyway, but the other file's tests would still have counted without the list.
        t.write("ci/test_dead.py", hostile)
        dead_out = os.path.join(t.d, "py_dead.json")
        self.assertNotIn("passed", set(listed(dead_out).values()))
        t.write("design/requirements/cortex.toml", toml_of([record(acc=[("a1", "test", "ci/test_live.py::test_passes", None)],
                sha=c.unit_hash(c.source_unit(DESIGN, "The latch starts tripped")))]))
        rc, text = t.run("--bless", "--py-tests", dead_out)
        self.assertNotEqual(rc, 0, text)
        self.assertIn("did not record it as passed", text)
        self.assertEqual(t.run("--bless")[0], 0)  # the static scan alone is fooled: that is why the list exists
        # Without the list the static scan still counts what it can see, and says so.
        t.write("design/requirements/cortex.toml", toml_of([record(acc=[("a1", "test", "ci/test_live.py::test_fails", None)],
                sha=c.unit_hash(c.source_unit(DESIGN, "The latch starts tripped")))]))
        rc, text = t.run("--bless")
        self.assertEqual(rc, 0, text)
        self.assertIn("No --py-tests list", text)
        # A forged list cannot revive what the static scan refuses, and a malformed one is a clean error.
        forged = os.path.join(t.d, "forged.json")
        with open(forged, "w") as fh:
            json.dump({"version": 1, "tests": [{"id": "x", "file": "ci/test_dead.py", "class": "T", "name": "test_x", "outcome": "passed"}]}, fh)
        t.write("design/requirements/cortex.toml", toml_of([record(acc=[("a1", "test", "ci/test_dead.py::test_x", None)],
                sha=c.unit_hash(c.source_unit(DESIGN, "The latch starts tripped")))]))
        self.assertNotEqual(t.run("--bless", "--py-tests", forged)[0], 0)
        with open(forged, "w") as fh:
            fh.write('{"tests": "nope"}')
        rc, text = t.run("--bless", "--py-tests", forged)
        self.assertNotEqual(rc, 0)
        self.assertNotIn("Traceback", text)

    def test_regression_271_the_workflow_builds_and_uses_the_python_list(self):
        with open(os.path.join(ROOT, ".github/workflows/conformance.yml")) as fh:
            wf = fh.read()
        self.assertIn("py_test_list.py", wf)
        self.assertEqual(wf.count("--py-tests"), 2)  # the judge and the closure gate
        self.assertEqual(wf.count("--rust-tests"), 2)
        self.assertIn("py-tests.json", wf)
        # The PR's Python tests must not be able to write the Rust list: separate jobs, separate artifacts.
        rust_job = wf[wf.index("\n  test-list:"):wf.index("\n  python-test-list:")]
        py_job = wf[wf.index("\n  python-test-list:"):wf.index("\n  conformance:")]
        self.assertIn("rust-test-list", rust_job)
        self.assertNotIn("py_test_list", rust_job)
        self.assertNotIn("rust-tests.json", py_job)
        self.assertIn("py-test-list", py_job)
        self.assertIn("needs: [test-list, python-test-list]", wf)

    def test_regression_271_directory_form_refs_are_refused(self):
        for ref in ("crates/sup::it_starts_tripped", "crates/sup/tests::it_starts_tripped", "crates/sup/::it_starts_tripped"):
            t = self.tree(recs=[self.rec(ref)], bless=False)
            lst = cargo_list(os.path.join(t.d, "l.json"), [("sup", "crates/sup/tests/t.rs", "it_starts_tripped")])
            for args in ((), ("--rust-tests", lst)):
                rc, out = t.run("--bless", *args)
                self.assertNotEqual(rc, 0, (ref, args))
                self.assertIn("not a directory", out)

    def test_regression_271_python_tests_are_what_unittest_runs(self):
        src = (
            "import unittest\n"
            "def test_module_level(): pass\n"
            "S = 'def test_in_string(self): pass'\n"
            "class T(unittest.TestCase):\n"
            "    def test_real(self): pass\n"
            "    @unittest.skip('x')\n"
            "    def test_skipped(self): pass\n"
            "    @unittest.skipIf(True, 'x')\n"
            "    def test_skip_if(self): pass\n"
            "    @unittest.skipUnless(False, 'x')\n"
            "    def test_skip_unless(self): pass\n"
            "    @unittest.expectedFailure\n"
            "    def test_xfail(self): pass\n"
            "    def helper(self): pass\n"
            "class Sub(T):\n"
            "    def test_inherited_class_member(self): pass\n"
            "class NotATest:\n"
            "    def test_plain_class(self): pass\n"
            "class A(unittest.IsolatedAsyncioTestCase):\n"
            "    async def test_async_real(self): pass\n"
            "# def test_comment(self): pass\n"
        )
        self.assertEqual(c.python_tests(src), {"test_real", "test_async_real", "test_inherited_class_member"})
        self.assertEqual(c.python_tests("def broken(:\n"), set())

    def test_regression_271_python_modules_with_side_effects_are_not_counted(self):
        head = "import unittest\nclass T(unittest.TestCase):\n    def test_x(self): pass\n"
        for name, extra in (
            ("raise", "raise SystemExit(1)\n"),
            ("if block", "if False:\n    pass\n"),
            ("try", "try:\n    pass\nexcept Exception:\n    pass\n"),
            ("load_tests def", "def load_tests(l, t, p):\n    return t\n"),
            ("load_tests assign", "load_tests = None\n"),
            ("load_tests import", "from x import load_tests\n"),
            ("setUpModule", "def setUpModule():\n    raise unittest.SkipTest\n"),
            ("rebind TestCase", "unittest = None\n"),
            ("attribute assign", "T.test_x = None\n"),
            ("other call", "print('x')\n"),
        ):
            self.assertEqual(c.python_tests(head + extra), set(), name)
        shadow = "from unittest import TestCase\nclass TestCase: pass\nclass T(TestCase):\n    def test_x(self): pass\n"
        self.assertEqual(c.python_tests(shadow), set())
        ok = head + "HERE = 1\nimport sys\nsys.path.insert(0, '.')\nif __name__ == '__main__':\n    unittest.main()\n"
        self.assertEqual(c.python_tests(ok), {"test_x"})

    def test_regression_271_python_last_binding_and_decorators_decide(self):
        src = (
            "import unittest\n"
            "class T(unittest.TestCase):\n    def test_old(self): pass\n"
            "class T(unittest.TestCase):\n    def test_new(self): pass\n"
            "class D(unittest.TestCase):\n"
            "    def test_plain(self): pass\n"
            "    def test_twice(self): pass\n"
            "    test_twice = None\n"
            "class Skipped(unittest.TestCase):\n"
            "    @classmethod\n    def setUpClass(cls):\n        raise unittest.SkipTest('no')\n"
            "    def test_never(self): pass\n"
        )
        self.assertEqual(c.python_tests(src), {"test_new", "test_plain"})

    def test_regression_271_python_refs_resolve_only_in_ci_test_files(self):
        good = "import unittest\nclass T(unittest.TestCase):\n    def test_x(self): pass\n"
        for path, ok in (("ci/test_a.py", True), ("ci/helper.py", False), ("ci/sub/test_b.py", False), ("crates/x/test_c.py", False)):
            t = self.tree(recs=[record(acc=[("a1", "test", "x_never::test_x" if False else f"{path}::test_x", None)])], extra={path: good}, bless=False)
            self.assertEqual(t.run("--bless")[0] == 0, ok, path)

    def test_regression_271_file_kind_wants_a_regular_file_inside_the_repo(self):
        outside = tempfile.NamedTemporaryFile(delete=False)
        self.addCleanup(os.unlink, outside.name)
        for ref, ok in (("docs/NAMES.md", True), ("docs/../docs/NAMES.md", True), ("docs", False), ("design", False), (".", False),
                        ("../etc/passwd", False), ("/etc/passwd", False), ("docs/../../x", False), ("link-out", False), ("link-in", True)):
            t = self.tree(recs=[record(acc=[("a1", "file", ref, None)])], bless=False)
            os.symlink(outside.name, os.path.join(t.d, "link-out"))
            os.symlink(os.path.join(t.d, "docs/NAMES.md"), os.path.join(t.d, "link-in"))
            rc, out = t.run("--bless")
            self.assertEqual(rc == 0, ok, (ref, out))

    def test_regression_271_a_source_outside_the_repo_is_refused(self):
        t = self.tree(recs=[record(source="../outside.md", sha="0" * 16)], bless=False)
        rc, out = t.run("--bless")
        self.assertNotEqual(rc, 0)
        self.assertIn("drifted", out)

    def test_regression_271_wrong_field_types_are_clean_errors(self):
        base = {"id": '"SAW-CORTEX-LATCH-1"', "rev": "1", "title": '"t"', "source": '"docs/D.md"', "anchor": '"The latch"',
                "source_sha": '"0000000000000000"', "tracks": "[5]"}
        wrong = ["[]", "{}", "[[1]]", "1", "true", '"x"', "1.5", "{ a = 1 }"]
        acc_ok = '[{ id = "a1", kind = "test", ref = "sup::it_starts_tripped" }]'
        cases = []
        for key in list(base) + ["level", "waiver"]:
            for w in wrong:
                cases.append((key, w))
        for n, (key, w) in enumerate(cases):
            fields = dict(base, acceptance=acc_ok)
            fields[key] = w
            body = "[[requirement]]\n" + "\n".join(f"{k} = {v}" for k, v in fields.items()) + "\n"
            t = self.tree(bless=False) if n == 0 else t
            t.write("design/requirements/cortex.toml", body)
            rc, out = t.run("--bless")
            self.assertNotIn("Traceback", out, (key, w))
        for acc in ("[[1]]", '[{ id = [], kind = "test", ref = "x" }]', '[{ id = "a", kind = [], ref = "x" }]',
                    '[{ id = "a", kind = "test", ref = [] }]', '[{ id = "a", kind = "test", ref = "x", pending = [] }]',
                    '[{ id = "a", kind = "file", ref = {} }]', '"x"', "{}"):
            fields = dict(base, acceptance=acc)
            body = "[[requirement]]\n" + "\n".join(f"{k} = {v}" for k, v in fields.items()) + "\n"
            t.write("design/requirements/cortex.toml", body)
            rc, out = t.run("--bless")
            self.assertNotIn("Traceback", out, acc)
            self.assertNotEqual(rc, 0, acc)
        t.write("design/names.toml", "component = 5\n")
        rc, out = t.run()
        self.assertNotIn("Traceback", out)
        self.assertNotEqual(rc, 0)
        t.write("design/names.toml", "component = [1, 2]\n")
        rc, out = t.run()
        self.assertNotIn("Traceback", out)
        self.assertNotEqual(rc, 0)

    def test_regression_271_only_toml_files_belong_in_the_requirements_dir(self):
        for name in ("notes.md", "cortex.toml.bak", "sub"):
            t = self.tree()
            if name == "sub":
                os.makedirs(os.path.join(t.d, "design/requirements/sub"))
            else:
                t.write(f"design/requirements/{name}", "x")
            rc, out = t.run()
            self.assertNotEqual(rc, 0, name)
            self.assertIn("only .toml record files", out)

    def test_regression_271_the_whole_status_file_is_stale_checked(self):
        t = self.tree()
        p = os.path.join(t.d, "design/STATUS.md")
        with open(p) as fh:
            text = fh.read()
        for edited in (text.replace("# Design conformance status", "# Hand edited", 1), text + "\nextra\n", text.replace("A test fails", "A TEST fails")):
            with open(p, "w") as fh:
                fh.write(edited)
            rc, out = t.run()
            self.assertNotEqual(rc, 0)
            self.assertIn("stale", out)
        t.run("--bless")
        self.assertEqual(t.run()[0], 0)

    def test_regression_271_non_ascii_digits_do_not_close_an_issue(self):
        r = "rafe-walker/sawbones-labs"
        self.assertEqual(c.referenced_issues("Fixes #\u0665", r), [])  # Arabic-Indic digit five
        self.assertEqual(c.referenced_issues("Fixes #\uff15", r), [])  # fullwidth five
        self.assertEqual(c.referenced_issues("Fixes #5", r), [5])

    def test_regression_271_the_closure_gate_reads_commit_messages(self):
        t = self.tree(recs=[record(acc=[("a1", "test", "later", 5)])])
        t.write("commits.txt", "Tidy\n\nCloses #5\n")
        env = {"PR_BODY": "Part of #5", "PR_TITLE": "x", "REPO": "rafe-walker/sawbones-labs"}
        rc, out = t.run("closure", "--commits", os.path.join(t.d, "commits.txt"), env=env)
        self.assertNotEqual(rc, 0)
        self.assertIn("SAW-CORTEX-LATCH-1", out)
        t.write("commits.txt", "Tidy\n\nPart of #5\n")
        self.assertEqual(t.run("closure", "--commits", os.path.join(t.d, "commits.txt"), env=env)[0], 0)
        self.assertNotEqual(t.run("closure", "--commits", os.path.join(t.d, "missing.txt"), env=env)[0], 0)

    def test_hws_ids_are_validated_against_the_standard(self):
        std = '[[requirement]]\nid = "HWS-TXT-1"\narea = "TXT"\n'
        for ids, ok in (('["HWS-TXT-1"]', True), ('["HWS-TXT-9"]', False), ('[]', False), ('["TXT-1"]', False),
                        ('["HWS-TXT-1", "HWS-TXT-1"]', False), ('"HWS-TXT-1"', False), ('[1]', False)):
            t = self.tree(recs=[record(extra=f"hws = {ids}")], extra={"standards/hws/0.1/requirements.toml": std}, bless=False)
            rc, out = t.run("--bless")
            self.assertEqual(rc == 0, ok, (ids, out))
            self.assertNotIn("Traceback", out)
        t = self.tree(recs=[record(extra='hws = ["HWS-TXT-1"]')], bless=False)  # no standards directory at all
        self.assertNotEqual(t.run("--bless")[0], 0)

    def test_regression_271_the_judge_fails_closed_once_it_is_on_the_base_branch(self):
        with open(os.path.join(ROOT, ".github/workflows/conformance.yml")) as fh:
            wf = fh.read()
        self.assertNotIn("|| cp ci/", wf)  # no silent fallback on any `git show` failure
        self.assertEqual(wf.count('git cat-file -e "${BASE_REF}:ci/conformance.py"'), 2)
        self.assertIn('git cat-file -e "${BASE_REF}:ci/rust_test_list.py"', wf)
        self.assertEqual(wf.count("is not available"), 3)  # a missing base ref is an error, not a fallback
        self.assertIn("::warning::", wf)
        self.assertIn("git log --format=%B", wf)

    def test_regression_271_the_real_repo_text_check_is_partial_and_xray_tracks_an_issue(self):
        rows, errors = c.run_check(ROOT)
        status = {r["rec"]["id"]: r["status"] for r in rows}
        self.assertEqual(status["SAW-CORTEX-TEXT-1"], c.PARTIAL)
        for r in rows:
            if r["rec"]["id"].startswith("SAW-XRAY-"):
                self.assertNotIn(252, r["rec"]["tracks"], r["rec"]["id"])
                self.assertTrue(r["rec"]["tracks"])


class StatusFile(Base):
    def test_regression_stale_status_file_fails_and_bless_fixes_it(self):
        t = self.tree()
        self.assertEqual(t.run()[0], 0)
        p = os.path.join(t.d, "design/STATUS.md")
        with open(p) as fh:
            text = fh.read().replace("| built |", "| not built |", 1)
        with open(p, "w") as fh:
            fh.write(text)
        rc, out = t.run()
        self.assertNotEqual(rc, 0)
        self.assertIn("stale", out)
        self.assertEqual(t.run("--bless")[0], 0)
        self.assertEqual(t.run()[0], 0)

    def test_status_changes_when_a_requirement_status_changes(self):
        t = self.tree()
        t.write("crates/sup/tests/t.rs", "")
        rc, _ = t.run("--bless")  # the test vanished: must fail loudly, not quietly update
        self.assertNotEqual(rc, 0)


class Closure(Base):
    def gate(self, t, body, title=""):
        return t.run("closure", env={"PR_BODY": body, "PR_TITLE": title, "REPO": "rafe-walker/sawbones-labs"})

    def test_regression_closing_an_issue_with_an_unbuilt_requirement_fails(self):
        t = self.tree(recs=[record(acc=[("a1", "test", "sup::it_starts_tripped", None), ("a2", "test", "later", 5)])])
        rc, out = self.gate(t, "Fixes #5")
        self.assertNotEqual(rc, 0)
        self.assertIn("SAW-CORTEX-LATCH-1", out)
        self.assertNotEqual(self.gate(t, "", title="closes #5")[0], 0)
        self.assertNotEqual(self.gate(t, "Resolved rafe-walker/sawbones-labs#5")[0], 0)

    def test_a_built_requirement_lets_the_issue_close(self):
        t = self.tree()
        self.assertEqual(self.gate(t, "Fixes #5")[0], 0)

    def test_an_unrelated_or_foreign_or_unclosing_reference_is_fine(self):
        t = self.tree(recs=[record(acc=[("a1", "test", "later", 5)])])
        for body in ("Fixes #6", "Part of #5", "See #5, also fixes other/repo#5", "not a closing word: prefixes #5", ""):
            self.assertEqual(self.gate(t, body)[0], 0, body)

    def test_waived_counts_as_done(self):
        t = self.tree(recs=[record(acc=[("a1", "test", "later", 5)], extra='waiver = { reason = "dropped" }')])
        self.assertEqual(self.gate(t, "Fixes #5")[0], 0)

    def test_pr_text_is_data_never_executed(self):
        t = self.tree()
        rc, out = self.gate(t, "Fixes #5 $(touch /tmp/pwned_by_pr_body) `touch /tmp/pwned_by_pr_body`")
        self.assertEqual(rc, 0)
        self.assertFalse(os.path.exists("/tmp/pwned_by_pr_body"))


class Units(unittest.TestCase):
    def test_source_unit_is_a_row_a_bullet_or_a_paragraph(self):
        text = "| a | b |\n| c | d |\n\npara one\npara two\n\n- bullet one\n  more\n- bullet two\n"
        self.assertEqual(c.source_unit(text, "c | d"), "| c | d |")
        self.assertEqual(c.source_unit(text, "para two"), "para one\npara two")
        self.assertEqual(c.source_unit(text, "bullet one"), "- bullet one\n  more")

    def test_hash_ignores_whitespace_only(self):
        self.assertEqual(c.unit_hash("a  b\n c"), c.unit_hash("a b c"))
        self.assertNotEqual(c.unit_hash("a b c"), c.unit_hash("a b d"))

    def test_rust_tests_ignores_strings_and_comments(self):
        live, ign = c.rust_tests('#[test]\nfn a() { let _ = "#[test] fn b() {}"; }\n#[tokio::test]\nfn c() {}\n#[test]\n#[ignore = "x"]\nfn d() {}\n')
        self.assertEqual((live, ign), ({"a", "c"}, {"d"}))

    def test_closing_keyword_parse(self):
        r = "rafe-walker/sawbones-labs"
        self.assertEqual(c.referenced_issues("Fixes #1, closes: #2. Resolves https://github.com/rafe-walker/sawbones-labs/issues/3", r), [1, 2, 3])
        self.assertEqual(c.referenced_issues("prefixes #4 and fixes x/y#5", r), [])


class RealTree(unittest.TestCase):
    def test_the_repository_is_conformant_and_its_status_is_current(self):
        rows, errors = c.run_check(ROOT)
        self.assertEqual(errors + c.check_status_file(ROOT, rows, False), [])
        self.assertGreaterEqual(len(rows), 18)


if __name__ == "__main__":
    unittest.main()
