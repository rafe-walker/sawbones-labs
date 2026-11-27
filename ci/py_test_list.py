#!/usr/bin/env python3
"""Run `python -m unittest discover -s ci -p 'test_*.py'` and record what really ran.

Writes JSON for ci/conformance.py (`--py-tests`): one row per test with the outcome
passed, failed, errored, skipped, expected failure or unexpected success. A test
whose `run` was replaced, whose class was stripped by a metaclass or that never
reached the result object is simply not in the list, so the X-ray cannot count it.
Limit: the PR's own tests run here, so against a hostile PR the list can still be
forged (as cargo's list can by a build.rs); conformance.py also needs the test in a
static scan that fails closed.

Usage: python3 -I ci/py_test_list.py OUT.json [--root DIR]
Writes {"version": 1, "tests": [{"id", "file", "class", "name", "outcome"}]}. Exit status
is 0 whenever the list was written, even if tests failed (the unit-test step judges that).
Stdlib only.
"""
import json
import os
import sys
import unittest

BAD = {"failed", "errored"}


class Recorder(unittest.TestResult):
    def __init__(self, root):
        super().__init__()
        self.root = os.path.realpath(root)
        self.rows = {}

    def _row(self, test):
        key = test.id()
        if key not in self.rows:
            module = sys.modules.get(type(test).__module__)
            path = getattr(module, "__file__", None)
            rel = None
            if path:
                path = os.path.realpath(path)
                if path.startswith(self.root + os.sep):
                    rel = os.path.relpath(path, self.root).replace(os.sep, "/")
            self.rows[key] = {"id": key, "file": rel, "class": type(test).__name__,
                              "name": getattr(test, "_testMethodName", None), "outcome": "started"}
        return self.rows[key]

    def _mark(self, test, outcome):
        row = self._row(test)
        if row["outcome"] not in BAD:
            row["outcome"] = outcome

    def startTest(self, test):
        super().startTest(test)
        self._row(test)

    def addSuccess(self, test):
        super().addSuccess(test)
        if self._row(test)["outcome"] == "started":
            self._mark(test, "passed")

    def addFailure(self, test, err):
        super().addFailure(test, err)
        self._mark(test, "failed")

    def addError(self, test, err):
        super().addError(test, err)
        self._mark(test, "errored")

    def addSkip(self, test, reason):
        super().addSkip(test, reason)
        self._mark(test, "skipped")

    def addExpectedFailure(self, test, err):
        super().addExpectedFailure(test, err)
        self._mark(test, "expected failure")

    def addUnexpectedSuccess(self, test):
        super().addUnexpectedSuccess(test)
        self._mark(test, "unexpected success")

    def addSubTest(self, test, subtest, err):
        super().addSubTest(test, subtest, err)
        if err is not None:
            self._mark(test, "failed")


def main(argv):
    args = list(argv)
    root = os.getcwd()
    if "--root" in args:
        i = args.index("--root")
        root = args[i + 1]
        del args[i:i + 2]
    if len(args) != 1:
        print(__doc__)
        return 2
    root = os.path.realpath(root)
    ci = os.path.join(root, "ci")
    os.chdir(root)
    suite = unittest.TestLoader().discover(ci, pattern="test_*.py", top_level_dir=ci)
    result = Recorder(root)
    suite.run(result)
    rows = sorted(result.rows.values(), key=lambda r: r["id"])
    with open(args[0], "w", encoding="utf-8") as fh:
        json.dump({"version": 1, "tests": rows}, fh, indent=1)
        fh.write("\n")
    passed = sum(r["outcome"] == "passed" for r in rows)
    print(f"{passed} of {len(rows)} Python tests passed ({len(rows) - passed} did not).")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
