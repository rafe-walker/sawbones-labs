"""Tests for ci/regression_guard.py (regression_199_*; issue #199).

Run: python3 -m unittest discover -s ci -p 'test_*.py' -v
Each end-to-end case builds a throwaway git repo, so the guard is exercised
exactly as CI runs it.
"""
import json
import os
import subprocess
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import regression_guard as g  # noqa: E402

GOOD_TEST = """
#[test]
fn regression_5_thing() {
    assert_eq!(1, 1);
}
"""
BASE_FILES = {
    "src/lib.rs": "pub fn a() {}\n",
    "tests/existing.rs": "#[test]\nfn keeps_working() { assert!(true); }\n#[test]\nfn another() { assert!(true); }\n",
    "docs/REGRESSIONS.md": "| Issue | x | y |\n| --- | --- | --- |\n",
    ".github/workflows/ci.yml": "name: ci\njobs:\n  build:\n    steps:\n      - name: compile\n        run: cargo build\n      - name: test\n        run: cargo test\n  lint:\n    steps:\n      - run: cargo clippy\n",
    "ci/regression_guard.py": "# guard\n",
    ".github/workflows/regression-guard.yml": "name: guard\n",
}
LEDGER5 = "| #5 | broke | `regression_5_thing` |\n"


def git(cwd, *args):
    subprocess.run(["git", "-c", "user.name=t", "-c", "user.email=t@example.com", "-c", "commit.gpgsign=false", *args],
                   cwd=cwd, check=True, capture_output=True)


def guard(change, title="Fixes #5", body="", labels=(), bugs="5"):
    """Apply `change(dir)` on top of BASE_FILES and run the guard; return (rc, output)."""
    with tempfile.TemporaryDirectory() as d:
        git(d, "init", "-q", "-b", "main")
        for path, text in BASE_FILES.items():
            os.makedirs(os.path.dirname(os.path.join(d, path)) or d, exist_ok=True)
            with open(os.path.join(d, path), "w") as fh:
                fh.write(text)
        git(d, "add", "-A")
        git(d, "commit", "-q", "-m", "base")
        git(d, "checkout", "-q", "-b", "pr")
        change(d)
        git(d, "add", "-A")
        git(d, "commit", "-q", "--allow-empty", "-m", "pr")
        env = dict(os.environ, PR_TITLE=title, PR_BODY=body, PR_LABELS=json.dumps(list(labels)),
                   BASE_REF="main", FAKE_BUG_ISSUES=bugs, REPO="rafe-walker/sawbones-labs")
        r = subprocess.run([sys.executable, "-I", os.path.join(HERE, "regression_guard.py")], cwd=d, env=env,
                           capture_output=True, text=True)
        return r.returncode, r.stdout + r.stderr


def write(path, text, append=False):
    def f(d):
        p = os.path.join(d, path)
        os.makedirs(os.path.dirname(p), exist_ok=True)
        with open(p, "a" if append else "w") as fh:
            fh.write(text)
    return f


def both(*fs):
    return lambda d: [f(d) for f in fs]


def good_fix(extra=None):
    fs = [write("tests/new.rs", GOOD_TEST), write("docs/REGRESSIONS.md", LEDGER5, append=True)]
    return both(*fs, *(extra or []))


class Keywords(unittest.TestCase):
    def test_regression_199_every_closing_keyword(self):
        for kw in ["close", "closes", "closed", "fix", "fixes", "fixed", "resolve", "resolves", "resolved",
                   "Fixes:", "CLOSED:", "rEsOlVeD", "fix:"]:
            sep = " " if kw.endswith(":") else " "
            with self.subTest(kw=kw):
                self.assertEqual(g.referenced_issues(f"{kw}{sep}#7", "o/r"), [7])

    def test_regression_199_repo_qualified_and_url_forms(self):
        self.assertEqual(g.referenced_issues("Fixes rafe-walker/sawbones-labs#9", "rafe-walker/sawbones-labs"), [9])
        self.assertEqual(g.referenced_issues("fixes https://github.com/rafe-walker/sawbones-labs/issues/11", "rafe-walker/sawbones-labs"), [11])
        self.assertEqual(g.referenced_issues("Fixes other/repo#9", "rafe-walker/sawbones-labs"), [])
        self.assertEqual(g.referenced_issues("prefixes #4 and Part of #3", "o/r"), [])

    def test_regression_199_keyword_variants_are_gated_end_to_end(self):
        for title in ["Fix #5", "Closed #5", "resolves rafe-walker/sawbones-labs#5"]:
            with self.subTest(title=title):
                rc, out = guard(write("src/other.rs", "pub fn b() {}\n"), title=title)
                self.assertEqual(rc, 1, out)
                self.assertIn("#5 is a bug", out)


class RealCheck(unittest.TestCase):
    def test_regression_199_real_test_and_ledger_pass(self):
        rc, out = guard(good_fix())
        self.assertEqual(rc, 0, out)

    def test_regression_199_test_named_in_src_cfg_test_passes(self):
        src = "pub fn a() {}\n#[cfg(test)]\nmod t {\n    #[test]\n    fn regression_5_inner() { assert!(true); }\n}\n"
        rc, out = guard(both(write("src/lib.rs", src), write("docs/REGRESSIONS.md", LEDGER5, append=True)))
        self.assertEqual(rc, 0, out)

    def test_regression_199_name_in_comment_does_not_count(self):
        for text in ["// regression_5 and #5\n", "/* regression_5 */\n"]:
            with self.subTest(text=text):
                rc, out = guard(both(write("src/lib.rs", text, append=True), write("tests/c.rs", text),
                                     write("docs/REGRESSIONS.md", LEDGER5, append=True)))
                self.assertEqual(rc, 1, out)
                self.assertIn("no real check", out)

    def test_regression_199_comment_inside_unrelated_test_does_not_count(self):
        t = "#[test]\nfn unrelated() {\n    // regression_5\n    assert!(true);\n}\n"
        rc, out = guard(both(write("tests/c.rs", t), write("docs/REGRESSIONS.md", LEDGER5, append=True)))
        self.assertEqual(rc, 1, out)

    def test_regression_199_mention_in_docs_does_not_count(self):
        rc, out = guard(both(write("docs/notes.md", "see #5 regression_5\n"),
                             write("docs/REGRESSIONS.md", LEDGER5, append=True)))
        self.assertEqual(rc, 1, out)

    def test_regression_199_ci_comment_does_not_count_but_a_step_does(self):
        base = BASE_FILES[".github/workflows/ci.yml"]
        rc, out = guard(both(write(".github/workflows/ci.yml", base + "# step for #5\n"),
                             write("docs/REGRESSIONS.md", LEDGER5, append=True)))
        self.assertEqual(rc, 1, out)
        step = base + "      - name: check for #5\n        run: ./ci/check5.sh\n"
        rc, out = guard(both(write(".github/workflows/ci.yml", step),
                             write("docs/REGRESSIONS.md", LEDGER5, append=True)))
        self.assertEqual(rc, 0, out)

    def test_regression_199_missing_ledger_row_fails(self):
        rc, out = guard(write("tests/new.rs", GOOD_TEST))
        self.assertEqual(rc, 1, out)
        self.assertIn("REGRESSIONS.md", out)

    def test_regression_199_non_bug_issue_is_skipped(self):
        rc, out = guard(write("src/other.rs", "pub fn b() {}\n"), bugs="")
        self.assertEqual(rc, 0, out)


class Removals(unittest.TestCase):
    def test_regression_199_deleted_test_fails_and_names_it(self):
        rc, out = guard(write("tests/existing.rs", "#[test]\nfn keeps_working() { assert!(true); }\n"), title="chore")
        self.assertEqual(rc, 1, out)
        self.assertIn("test function `another` removed (was in tests/existing.rs)", out)

    def test_regression_199_deleted_test_file_fails(self):
        rc, out = guard(lambda d: os.remove(os.path.join(d, "tests/existing.rs")), title="chore")
        self.assertEqual(rc, 1, out)
        self.assertIn("`keeps_working` removed", out)

    def test_regression_199_moved_test_is_not_a_deletion(self):
        def mv(d):
            os.remove(os.path.join(d, "tests/existing.rs"))
            write("tests/moved.rs", BASE_FILES["tests/existing.rs"])(d)
        rc, out = guard(mv, title="chore")
        self.assertEqual(rc, 0, out)

    def test_regression_199_added_ignore_fails(self):
        t = BASE_FILES["tests/existing.rs"].replace("#[test]\nfn another", "#[test]\n#[ignore]\nfn another")
        rc, out = guard(write("tests/existing.rs", t), title="chore")
        self.assertEqual(rc, 1, out)
        self.assertIn("`#[ignore]` added in tests/existing.rs", out)

    def test_regression_199_removed_ci_job_and_step_fail(self):
        ci = "name: ci\njobs:\n  build:\n    steps:\n      - name: compile\n        run: cargo build\n"
        rc, out = guard(write(".github/workflows/ci.yml", ci), title="chore")
        self.assertEqual(rc, 1, out)
        self.assertIn("CI job lint removed from .github/workflows/ci.yml", out)
        self.assertIn("step build: test", out)

    def test_regression_199_label_allows_removals_but_still_prints_them(self):
        rc, out = guard(write("tests/existing.rs", "#[test]\nfn keeps_working() { assert!(true); }\n"),
                        title="chore", labels=["guards-change"])
        self.assertEqual(rc, 0, out)
        self.assertIn("test function `another` removed", out)

    def test_regression_199_guard_self_edit_needs_label(self):
        for path in ["ci/regression_guard.py", ".github/workflows/regression-guard.yml"]:
            with self.subTest(path=path):
                rc, out = guard(write(path, "# edit\n", append=True), title="chore")
                self.assertEqual(rc, 1, out)
                self.assertIn("guards-change", out)
                rc, out = guard(write(path, "# edit\n", append=True), title="chore", labels=["guards-change"])
                self.assertEqual(rc, 0, out)

    def test_regression_199_editing_the_guard_cannot_satisfy_its_own_check(self):
        rc, out = guard(both(write("ci/regression_guard.py", "# #5 regression_5\nx = '#5'\n"),
                             write("docs/REGRESSIONS.md", LEDGER5, append=True)), labels=["guards-change"])
        self.assertEqual(rc, 1, out)
        self.assertIn("no real check", out)


class Parser(unittest.TestCase):
    def test_regression_199_braces_and_strings_do_not_confuse_the_parser(self):
        src = 'fn helper() { let s = "}{"; }\n#[test]\nfn a() { let c = \'}\'; /* } */ }\n#[tokio::test]\nasync fn b() {}\n'
        self.assertEqual(sorted(g.test_fn_names(src)), ["a", "b"])


if __name__ == "__main__":
    unittest.main()
