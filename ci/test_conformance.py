"""Tests for ci/conformance.py (the design-conformance X-ray).

Run: python3 -I -m unittest discover -s ci -p 'test_*.py' -v
Every failure mode has a regression_ test that builds a throwaway tree in a
temp dir, so the checker is exercised exactly as CI runs it. The last class
runs it on the real tree.
"""
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
            record(tracks="[]"),
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
