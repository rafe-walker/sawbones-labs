#!/usr/bin/env python3
"""Tests for ci/no_build_artifacts.sh (#208, #246). Run: python3 -I -m unittest discover -s ci -p 'test_*.py'"""
import os
import subprocess
import tempfile
import unittest
from pathlib import Path

GUARD = Path(__file__).with_name("no_build_artifacts.sh").resolve()


def guard_passes(*paths):
    with tempfile.TemporaryDirectory() as d:
        env = {**os.environ, "GIT_CONFIG_GLOBAL": os.devnull, "GIT_CONFIG_NOSYSTEM": "1"}
        subprocess.run(["git", "init", "-q", d], check=True, env=env)
        for p in paths:
            f = Path(d, p)
            f.parent.mkdir(parents=True, exist_ok=True)
            f.write_bytes(b"x")
        subprocess.run(["git", "-C", d, "add", "-f", "."], check=True, env=env)
        r = subprocess.run(["bash", str(GUARD)], cwd=d, capture_output=True, env=env)
        return r.returncode == 0


class NoBuildArtifacts(unittest.TestCase):
    def test_regression_246_coverage_profiles_are_build_output(self):
        self.assertFalse(guard_passes("default_1_0_8420.profraw"))
        self.assertFalse(guard_passes("crates/x/merged.profdata"))

    def test_regression_208_python_caches_are_build_output(self):
        self.assertFalse(guard_passes("ci/__pycache__/x.cpython-312.pyc"))
        self.assertFalse(guard_passes("ci/x.pyc"))

    def test_rust_build_output_is_refused(self):
        for p in ("target/debug/x", "crates/a/target/x", "x.o", "libx.rlib", "x.rmeta"):
            with self.subTest(p=p):
                self.assertFalse(guard_passes(p))

    def test_ordinary_source_passes(self):
        self.assertTrue(guard_passes("crates/a/src/lib.rs", "ci/check.py", "docs/targets.md", "README.md"))


if __name__ == "__main__":
    unittest.main()
