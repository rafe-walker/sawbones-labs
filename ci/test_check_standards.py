#!/usr/bin/env python3
"""Tests for ci/check_standards.py. Run: python3 -I -m unittest discover -s ci -p 'test_*.py'"""
import hashlib
import importlib.util
import subprocess
import tempfile
import unittest
from pathlib import Path

spec = importlib.util.spec_from_file_location("check_standards", Path(__file__).with_name("check_standards.py"))
cs = importlib.util.module_from_spec(spec)
spec.loader.exec_module(cs)

REC = '''[[requirement]]
id = "HWS-ABC-1"
area = "ABC"
level = "MUST"
title = "t"
text = "x"
rationale = "r"
borrows = ["b"]
verify = ["test"]
version_added = "0.1"
'''


def sha(b: bytes) -> str:
    return hashlib.sha256(b).hexdigest()


class Tree:
    def __init__(self, tmp):
        self.root = Path(tmp)
        self.std = self.root / "standards" / "hws"
        (self.std / "0.1").mkdir(parents=True)
        self.write("0.1/requirements.toml", REC)
        self.std.joinpath("CHANGELOG.md").write_text("# c\n\n## 0.1\n\nfirst\n")

    def write(self, rel, text):
        p = self.std / rel
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_text(text)

    def errors(self, base=None):
        return cs.check_all(self.root, base)

    def release(self, v="0.1"):
        folder = self.std / v
        lines = [f"{h}  {n}" for n, h in sorted(cs.manifest(folder).items())]
        (folder / "RELEASED").write_text("\n".join(lines) + "\n")

    def git(self, *a):
        subprocess.run(["git", "-C", str(self.root), "-c", "user.name=t", "-c", "user.email=t@example.com",
                        "-c", "commit.gpgsign=false", *a], check=True, capture_output=True)


class RecordRules(unittest.TestCase):
    def check(self, mutate, expect):
        with tempfile.TemporaryDirectory() as tmp:
            t = Tree(tmp)
            t.write("0.1/requirements.toml", mutate(REC))
            errs = t.errors()
            self.assertTrue(any(expect in e for e in errs), (expect, errs))

    def test_valid_tree_is_clean(self):
        with tempfile.TemporaryDirectory() as tmp:
            self.assertEqual(Tree(tmp).errors(), [])

    def test_bad_toml_fails(self):
        self.check(lambda s: s + "\n[[requirement\n", "cannot parse")

    def test_duplicate_id_fails(self):
        self.check(lambda s: s + "\n" + s, "duplicate id")

    def test_malformed_id_fails(self):
        self.check(lambda s: s.replace("HWS-ABC-1", "HWS-AB-01"), "id must look like")

    def test_area_must_match_id(self):
        self.check(lambda s: s.replace('area = "ABC"', 'area = "XYZ"'), "does not match the id")

    def test_level_must_be_must_or_should(self):
        self.check(lambda s: s.replace("MUST", "MAY"), "level must be MUST or SHOULD")

    def test_level_is_case_sensitive(self):
        self.check(lambda s: s.replace("MUST", "must"), "level must be MUST or SHOULD")

    def test_verify_values_are_limited(self):
        self.check(lambda s: s.replace('["test"]', '["vibes"]'), "verify must be")

    def test_verify_cannot_be_empty(self):
        self.check(lambda s: s.replace('["test"]', "[]"), "verify must be")

    def test_status_field_is_rejected(self):
        self.check(lambda s: s + 'status = "met"\n', "unknown keys")

    def test_missing_key_fails(self):
        self.check(lambda s: s.replace('rationale = "r"\n', ""), "missing keys")

    def test_version_added_must_exist(self):
        self.check(lambda s: s.replace('version_added = "0.1"', 'version_added = "9.9"'), "not a version folder")

    def test_missing_changelog_section_fails(self):
        with tempfile.TemporaryDirectory() as tmp:
            t = Tree(tmp)
            t.std.joinpath("CHANGELOG.md").write_text("# c\n")
            self.assertTrue(any("no '## 0.1' section" in e for e in t.errors()))

    def test_real_repo_standards_pass(self):
        root = Path(__file__).resolve().parent.parent
        self.assertEqual(cs.check_all(root), [])


class Versions(unittest.TestCase):
    def test_deleting_an_id_in_a_later_version_fails(self):
        with tempfile.TemporaryDirectory() as tmp:
            t = Tree(tmp)
            t.write("0.2/requirements.toml", REC.replace("HWS-ABC-1", "HWS-ABC-2"))
            t.std.joinpath("CHANGELOG.md").write_text("## 0.2\n\n## 0.1\n")
            self.assertTrue(any("is gone" in e for e in t.errors()))

    def test_withdrawn_id_stays_present(self):
        with tempfile.TemporaryDirectory() as tmp:
            t = Tree(tmp)
            t.write("0.2/requirements.toml", REC + 'withdrawn_in = "0.2"\n')
            t.std.joinpath("CHANGELOG.md").write_text("## 0.2\n\n## 0.1\n")
            self.assertEqual(t.errors(), [])


class Released(unittest.TestCase):
    def test_released_folder_matches_its_manifest(self):
        with tempfile.TemporaryDirectory() as tmp:
            t = Tree(tmp)
            t.release()
            self.assertEqual(t.errors(), [])

    def test_editing_a_released_file_fails(self):
        with tempfile.TemporaryDirectory() as tmp:
            t = Tree(tmp)
            t.release()
            t.write("0.1/requirements.toml", REC.replace('title = "t"', 'title = "edited"'))
            self.assertTrue(any("was modified" in e for e in t.errors()))

    def test_adding_a_file_to_a_released_folder_fails(self):
        with tempfile.TemporaryDirectory() as tmp:
            t = Tree(tmp)
            t.release()
            t.write("0.1/extra.md", "x")
            self.assertTrue(any("not in the RELEASED manifest" in e for e in t.errors()))

    def test_deleting_a_released_file_fails(self):
        with tempfile.TemporaryDirectory() as tmp:
            t = Tree(tmp)
            t.write("0.1/README.md", "x")
            t.release()
            (t.std / "0.1" / "README.md").unlink()
            self.assertTrue(any("is missing" in e for e in t.errors()))

    def test_bad_manifest_line_fails(self):
        with tempfile.TemporaryDirectory() as tmp:
            t = Tree(tmp)
            t.write("0.1/RELEASED", "not a manifest\n")
            self.assertTrue(any("each line must be" in e for e in t.errors()))

    def test_draft_folder_may_change(self):
        with tempfile.TemporaryDirectory() as tmp:
            t = Tree(tmp)
            t.write("0.1/requirements.toml", REC.replace('title = "t"', 'title = "edited"'))
            self.assertEqual(t.errors(), [])


class GitBase(unittest.TestCase):
    def setup_repo(self, tmp, released):
        t = Tree(tmp)
        t.git("init", "-q", "-b", "main")
        if released:
            t.release()
        t.git("add", "-A")
        t.git("commit", "-q", "-m", "base")
        t.git("checkout", "-q", "-b", "pr")
        return t

    def test_changing_a_folder_released_at_base_fails_even_with_a_new_manifest(self):
        with tempfile.TemporaryDirectory() as tmp:
            t = self.setup_repo(tmp, released=True)
            t.write("0.1/requirements.toml", REC.replace('title = "t"', 'title = "edited"'))
            t.release()
            t.git("add", "-A")
            t.git("commit", "-q", "-m", "edit")
            errs = t.errors("main")
            self.assertTrue(any("must not change" in e for e in errs), errs)

    def test_removing_the_released_marker_fails(self):
        with tempfile.TemporaryDirectory() as tmp:
            t = self.setup_repo(tmp, released=True)
            (t.std / "0.1" / "RELEASED").unlink()
            t.git("add", "-A")
            t.git("commit", "-q", "-m", "unrelease")
            self.assertTrue(any("must not change" in e for e in t.errors("main")))

    def test_releasing_a_draft_is_allowed(self):
        with tempfile.TemporaryDirectory() as tmp:
            t = self.setup_repo(tmp, released=False)
            t.release()
            t.git("add", "-A")
            t.git("commit", "-q", "-m", "release")
            self.assertEqual(t.errors("main"), [])

    def test_a_new_version_beside_a_released_one_is_allowed(self):
        with tempfile.TemporaryDirectory() as tmp:
            t = self.setup_repo(tmp, released=True)
            t.write("0.2/requirements.toml", REC + 'withdrawn_in = "0.2"\n')
            t.std.joinpath("CHANGELOG.md").write_text("## 0.2\n\n## 0.1\n")
            t.git("add", "-A")
            t.git("commit", "-q", "-m", "next")
            self.assertEqual(t.errors("main"), [])

    def test_unresolvable_base_is_an_error(self):
        with tempfile.TemporaryDirectory() as tmp:
            t = self.setup_repo(tmp, released=False)
            self.assertTrue(any("does not resolve" in e for e in t.errors("nope")))


if __name__ == "__main__":
    unittest.main()
