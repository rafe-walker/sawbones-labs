"""Tests for ci/conformance.py (the design-conformance X-ray).

Run: python3 -I -m unittest discover -s ci -p 'test_*.py' -v
Every failure mode has a regression_ test that builds a throwaway tree in a
temp dir, so the checker is exercised exactly as CI runs it. The last class
runs it on the real tree.
"""
import json
import os
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
        for ref in ("it_starts_tripped", "crates/sup/tests/t.rs::it_starts_tripped", "crates/sup::it_starts_tripped"):
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
        self.assertEqual(len(run_blocks), 4)
        for b in run_blocks:
            self.assertNotIn("github.event.pull_request", b)



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
            t = self.tree(recs=[self.rec(ref)], bless=False)
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
        for ref, why in cases:
            t = self.tree(recs=[self.rec(ref)], bless=False)
            lst = cargo_list(os.path.join(t.d, "l.json"), tests)
            rc, out = t.run("--bless", "--rust-tests", lst)
            self.assertNotEqual(rc, 0, ref)
            self.assertIn(why, out, ref)
            # The same rules hold for the source fallback.
            if why != "ignored test":
                self.assertNotEqual(self.tree(recs=[self.rec(ref)], bless=False).run("--bless")[0], 0, ref)
        t = self.tree(recs=[self.rec("sup::regression_9_a*")], bless=False)
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
        if not shutil.which("cargo"):
            self.skipTest("needs cargo")
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
                                env=dict(os.environ, CARGO_TARGET_DIR=os.path.join(t.d, "tgt")))
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

    def test_regression_271_python_tests_are_what_unittest_runs(self):
        src = (
            "import unittest\n"
            "def test_module_level(): pass\n"
            "S = 'def test_in_string(self): pass'\n"
            "if False:\n"
            "    class T0(unittest.TestCase):\n"
            "        def test_under_if_false(self): pass\n"
            "class T(unittest.TestCase):\n"
            "    def test_real(self): pass\n"
            "    async def test_async_real(self): pass\n"
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
            "# def test_comment(self): pass\n"
        )
        self.assertEqual(c.python_tests(src), {"test_real", "test_async_real", "test_inherited_class_member"})
        self.assertEqual(c.python_tests("def broken(:\n"), set())

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
