#!/usr/bin/env python3
"""Unit tests for ci/check_typosquat.py. Run: python3 ci/test_check_typosquat.py"""
import importlib.util
import unittest
from pathlib import Path

spec = importlib.util.spec_from_file_location("ct", Path(__file__).with_name("check_typosquat.py"))
ct = importlib.util.module_from_spec(spec)
spec.loader.exec_module(ct)

POPULAR = {"proc-macro2", "serde", "tokio", "rand", "syn"}
EXISTING = {"zeroize", "curve25519-dalek"}


def flagged(*added):
    return {n for n, _ in ct.find_matches(set(added), EXISTING, POPULAR)}


class Typosquat(unittest.TestCase):
    def test_proc_macro1_is_flagged(self):
        self.assertEqual(flagged("proc-macro1"), {"proc-macro1"})

    def test_underscore_variant_flagged(self):
        self.assertEqual(flagged("proc_macro2"), {"proc_macro2"})

    def test_distance_two_long_name(self):
        self.assertEqual(flagged("curve25519-dalec"), {"curve25519-dalec"})
        self.assertEqual(flagged("zeroise"), {"zeroise"})

    def test_short_names_only_distance_one(self):
        self.assertEqual(flagged("serd"), {"serd"})
        self.assertEqual(flagged("sya"), {"sya"})
        self.assertEqual(flagged("sxx"), set())

    def test_clean_names_pass(self):
        self.assertEqual(flagged("blake3", "bytemuck", "bytes-utils"), set())

    def test_popular_crate_itself_passes(self):
        self.assertEqual(flagged("tokio", "rand"), set())


if __name__ == "__main__":
    unittest.main()
