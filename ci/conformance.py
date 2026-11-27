#!/usr/bin/env python3
"""Design conformance ("the X-ray"): does the code match the blueprint?

Requirement records live in design/requirements/<component>.toml. Each record
pins the design paragraph it came from (by hash) and names the checks that
prove it. This script computes, never trusts, each record's status:

  built      every acceptance check resolves to a test or file that exists
  partial    some resolve; the rest are `pending` on a tracking issue
  not built  none resolves (allowed, and it does not fail the run)
  drifted    the pinned source paragraph changed (or vanished) without the
             record being updated, or a `[impl->ID@rev]` tag names an old rev
  waived     the record carries a waiver with a reason (shown, never hidden)

Usage (run from the repository root, or pass --root):
  python3 -I ci/conformance.py                 check; fail if anything is wrong or STATUS.md is stale
  python3 -I ci/conformance.py --bless         also rewrite design/STATUS.md
  python3 -I ci/conformance.py hash FILE ANCHOR  print the hash to pin for a source paragraph
  python3 -I ci/conformance.py closure         closure gate for a PR (env: PR_TITLE, PR_BODY, REPO; --commits FILE
                                               holds `git log --format=%B BASE..HEAD`)

Rust tests are resolved against cargo's own list: pass `--rust-tests FILE`, made by
ci/rust_test_list.py. A test that does not compile, is cfg'd out, sits in a file no
`mod` reaches or in an uninvoked macro is not in that list, so it cannot count.
Without the flag the checker falls back to a pure-source scan and warns, because that
scan can still be fooled. Python tests are parsed with `ast` and must be methods of a
TestCase in ci/test_*.py that unittest discovery really runs.

Exit status is non-zero on: a malformed record, an unknown or duplicate ID, a
record naming a test or file that does not exist, a drifted record, a stale
STATUS.md, or (closure) a closing keyword for an issue whose requirements are
not all built or waived.

Stdlib only (Python 3.11+). The closure gate reads PR text only to extract
issue numbers with a strict regex; it never runs or interpolates that text.
"""
import ast
import fnmatch
import hashlib
import json
import os
import re
import sys
import tomllib

REQ_DIR = "design/requirements"
NAMES = "design/names.toml"
NAMES_DOC = "docs/NAMES.md"
STATUS_FILE = "design/STATUS.md"
MARK_START = "<!-- status:start (generated: python3 ci/conformance.py --bless) -->"
MARK_END = "<!-- status:end -->"

BUILT, PARTIAL, NOT_BUILT, DRIFTED, WAIVED = "built", "partial", "not built", "drifted", "waived"
ORDER = [BUILT, PARTIAL, NOT_BUILT, DRIFTED, WAIVED]

REQ_KEYS = {"id", "rev", "level", "title", "source", "anchor", "source_sha", "tracks", "acceptance", "waiver"}
REQ_REQUIRED = {"id", "rev", "title", "source", "anchor", "source_sha", "tracks", "acceptance"}
ACC_KEYS = {"id", "kind", "ref", "pending", "note"}
ACC_REQUIRED = {"id", "kind", "ref"}
KINDS = {"test", "file"}
LEVELS = {"MUST", "SHOULD", "MAY"}
ID_RE = re.compile(r"^SAW-([A-Z]+)-([A-Z0-9]+)-([1-9][0-9]*)$")
TAG_RE = re.compile(r"\[impl->(SAW-[A-Z]+-[A-Z0-9]+-[0-9]+)(?:@([0-9]+))?\]")
SCAN_DIRS = ("crates", "firmware", "plugins", "ci", "fuzz")
SCAN_SUFFIX = (".rs", ".py", ".sh")
# Files that mention tags as examples or fixtures, not as claims.
TAG_SKIP = {"ci/conformance.py", "ci/test_conformance.py"}
SKIP_DIRS = {"target", ".git", "node_modules"}

CLOSING = r"(?:close[sd]?|fix(?:e[sd])?|resolve[sd]?)"
REQ_FLAGS = re.IGNORECASE | re.ASCII  # ASCII: Arabic-Indic digits must not parse as an issue number


class Problem(Exception):
    pass


# ---------------------------------------------------------------- file helpers


def read(root, rel):
    with open(os.path.join(root, rel), encoding="utf-8") as fh:
        return fh.read()


def repo_file(root, rel):
    """Absolute path of `rel` if it is a regular file inside the repository
    (after normalising `..` and following symlinks), else None."""
    if not isinstance(rel, str) or not rel or "\0" in rel or os.path.isabs(rel):
        return None
    norm = os.path.normpath(rel)
    if norm == "." or norm == ".." or norm.startswith(".." + os.sep):
        return None
    real = os.path.realpath(os.path.join(root, norm))
    top = os.path.realpath(root)
    if not (real == top or real.startswith(top + os.sep)):
        return None
    return real if os.path.isfile(real) else None


def walk(root, dirs, suffixes):
    for top in dirs:
        base = os.path.join(root, top)
        for dirpath, dirnames, filenames in os.walk(base):
            dirnames[:] = sorted(d for d in dirnames if d not in SKIP_DIRS)
            for name in sorted(filenames):
                if name.endswith(suffixes):
                    full = os.path.join(dirpath, name)
                    yield os.path.relpath(full, root).replace(os.sep, "/")


# ------------------------------------------------------------- source hashing

_BULLET = re.compile(r"^\s*(?:[-*+]|\d+[.)])\s")


def source_unit(text, anchor):
    """The one table row, list item or paragraph that contains `anchor`."""
    lines = text.splitlines()
    hits = [i for i, l in enumerate(lines) if anchor in l]
    if not hits:
        raise Problem("anchor not found")
    if len(hits) > 1:
        raise Problem(f"anchor matches {len(hits)} lines; make it unique")
    i = hits[0]
    if lines[i].lstrip().startswith("|"):
        return lines[i]
    start = i
    while start > 0 and not _BULLET.match(lines[start]) and lines[start - 1].strip() \
            and not lines[start - 1].lstrip().startswith("|"):
        start -= 1
    end = i
    while end + 1 < len(lines) and lines[end + 1].strip() and not _BULLET.match(lines[end + 1]) \
            and not lines[end + 1].lstrip().startswith("|"):
        end += 1
    return "\n".join(lines[start:end + 1])


def unit_hash(unit):
    norm = " ".join(unit.split())
    return hashlib.sha256(norm.encode("utf-8")).hexdigest()[:16]


# ------------------------------------------------------------ test discovery
#
# Two sources of truth for Rust tests:
#   * cargo's own list (--rust-tests FILE, made by ci/rust_test_list.py from
#     `cargo test --workspace --all-targets` binaries run with --list, with and
#     without --ignored). A test that does not compile, is cfg'd out, sits in an
#     unreachable file or an uninvoked macro is simply not in that list.
#   * a pure-source scan, used for local runs without cargo (labelled as a
#     fallback, it can still be fooled) and to attribute cargo's tests to files.
# Python tests are parsed with `ast`.

GLOB_CHARS = "*?["
_FN = re.compile(r"((?:#\[[^\]]*\]\s*)*)(?:pub(?:\([^)]*\))?\s+)?(?:async\s+|const\s+|unsafe\s+)*fn\s+(\w+)")
_MOD = re.compile(r"((?:#\[[^\]]*\]\s*)*)(?:pub(?:\([^)]*\))?\s+)?mod\s+\w+\s*\{")
_MACRO = re.compile(r"macro_rules\s*!\s*\w+\s*[{(\[]")
_TEST_ATTR = re.compile(r"#\[\s*(?:\w+::)*test\b")
_IGNORE_ATTR = re.compile(r"#\[\s*ignore\b|cfg_attr\s*\([^\]]*\bignore\b")
_NEVER_CFG = re.compile(r"cfg\s*\(\s*(?:any\s*\(\s*\)|false|not\s*\(\s*(?:all\s*\(\s*\)|true)\s*\))\s*\)")
_CHAR = re.compile(r"'(?:\\.[^']*|[^\\'])'")
_RAW = re.compile(r'b?r(#*)"')


def blank_rust(src):
    """`src` with comments (block comments nest) and string/char contents
    replaced by spaces/underscores, same length, so regexes see only code."""
    out = list(src)
    n, i = len(src), 0

    def fill(a, b, ch):
        for k in range(a, b):
            if out[k] != "\n":
                out[k] = ch

    while i < n:
        c = src[i]
        if src.startswith("//", i):
            j = src.find("\n", i)
            j = n if j == -1 else j
            fill(i, j, " ")
            i = j
        elif src.startswith("/*", i):
            depth, j = 1, i + 2
            while j < n and depth:
                if src.startswith("/*", j):
                    depth, j = depth + 1, j + 2
                elif src.startswith("*/", j):
                    depth, j = depth - 1, j + 2
                else:
                    j += 1
            fill(i, j, " ")
            i = j
        elif c in "rb" and (i == 0 or not (src[i - 1].isalnum() or src[i - 1] == "_")) and _RAW.match(src, i):
            m = _RAW.match(src, i)
            end = src.find('"' + m.group(1), m.end())
            end = n if end == -1 else end + 1 + len(m.group(1))
            fill(m.end(), max(m.end(), end - 1 - len(m.group(1))), "_")
            i = end
        elif c == '"':
            j = i + 1
            while j < n and src[j] != '"':
                j += 2 if src[j] == "\\" else 1
            fill(i + 1, min(j, n), "_")
            i = j + 1
        elif c == "'":
            m = _CHAR.match(src, i)
            if m:
                fill(i + 1, m.end() - 1, "_")
                i = m.end()
            else:
                i += 1
        else:
            i += 1
    return "".join(out)


def _match_brace(s, i):
    """Index just past the `}` matching the `{` at s[i] (s is blanked code)."""
    depth, n = 0, len(s)
    while i < n:
        depth += s[i] == "{"
        depth -= s[i] == "}"
        i += 1
        if depth == 0:
            break
    return i


def _blank_span(s, a, b):
    return s[:a] + re.sub(r"[^\n]", " ", s[a:b]) + s[b:]


def rust_tests(src):
    """(live, ignored): names of `#[test]` fns found by a source scan, with
    nested comments, strings, `macro_rules!` bodies and modules or fns behind a
    never-true cfg removed. A fallback only: cargo's list is the authority."""
    code = blank_rust(src)
    for m in list(_MACRO.finditer(code)):
        code_open = m.end() - 1
        if code[code_open] == "{":
            code = _blank_span(code, m.start(), _match_brace(code, code_open))
        else:
            close = {"(": ")", "[": "]"}[code[code_open]]
            end = code.find(close, code_open)
            code = _blank_span(code, m.start(), len(code) if end == -1 else end + 1)
    for m in list(_MOD.finditer(code)):
        if _NEVER_CFG.search(m.group(1)):
            code = _blank_span(code, m.start(), _match_brace(code, m.end() - 1))
    live, ignored = set(), set()
    for m in _FN.finditer(code):
        attrs = m.group(1)
        if not _TEST_ATTR.search(attrs) or _NEVER_CFG.search(attrs):
            continue
        (ignored if _IGNORE_ATTR.search(attrs) else live).add(m.group(2))
    return live, ignored


_SKIP_DECORATORS = {"skip", "skipIf", "skipUnless", "expectedFailure"}


def _decorator_name(d):
    if isinstance(d, ast.Call):
        d = d.func
    if isinstance(d, ast.Attribute):
        return d.attr
    if isinstance(d, ast.Name):
        return d.id
    return ""


def _base_name(b):
    return b.attr if isinstance(b, ast.Attribute) else b.id if isinstance(b, ast.Name) else ""


def python_tests(src):
    """Names of `test_*` methods that `unittest` discovery would really run:
    defined directly in the body of a module-level TestCase subclass, not skipped
    or expected to fail, and not nested in `if`, strings or comments. A module-level
    `def test_x` is never collected by unittest, so it does not count."""
    try:
        tree = ast.parse(src)
    except (SyntaxError, ValueError):
        return set()
    testcase = {"TestCase", "IsolatedAsyncioTestCase"}
    found = set()
    for node in tree.body:
        if not isinstance(node, ast.ClassDef):
            continue
        if not any(_base_name(b) in testcase for b in node.bases):
            continue
        testcase.add(node.name)  # later classes may derive from this one
        for item in node.body:
            if isinstance(item, (ast.FunctionDef, ast.AsyncFunctionDef)) and item.name.startswith("test_"):
                if not any(_decorator_name(d) in _SKIP_DECORATORS for d in item.decorator_list):
                    found.add(item.name)
    return found


def pkg_dir(root, rel):
    """The directory (relative to root) of the nearest Cargo.toml above `rel`."""
    d = os.path.dirname(rel)
    while d:
        if os.path.isfile(os.path.join(root, d, "Cargo.toml")):
            return d
        d = os.path.dirname(d)
    return ""


class TestIndex:
    """Which tests exist, are live and are ignored. Built once per run.

    `cargo` is the parsed --rust-tests JSON, or None for the source fallback."""

    def __init__(self, root, cargo=None):
        self.root = root
        self.cargo = cargo
        self.decl = {}       # rel .rs path -> live names found by the source scan
        self.decl_ign = {}   # rel .rs path -> ignored names found by the source scan
        self.entries = []    # {"lang", "package", "dir", "src", "name", "leaf", "ignored"}
        for rel in walk(root, ("crates", "firmware", "plugins", "fuzz"), (".rs",)):
            try:
                live, ign = rust_tests(read(root, rel))
            except (OSError, UnicodeDecodeError):
                continue
            if live:
                self.decl[rel] = live
            if ign:
                self.decl_ign[rel] = ign
        if cargo is None:
            for rel, names in self.decl.items():
                self._add("rust", self._fallback_package(rel), pkg_dir(root, rel), rel, names, False)
            for rel, names in self.decl_ign.items():
                self._add("rust", self._fallback_package(rel), pkg_dir(root, rel), rel, names, True)
        else:
            for t in cargo["tests"]:
                self._add("rust", t["package"], pkg_dir(root, t["src"]), t["src"], [t["name"]], bool(t["ignored"]))
        # Only files `unittest discover -s ci -p 'test_*.py'` collects.
        for rel in walk(root, ("ci",), (".py",)):
            base = os.path.basename(rel)
            if "/" in rel[len("ci/"):] or not (base.startswith("test_") and base.endswith(".py")):
                continue
            try:
                names = python_tests(read(root, rel))
            except (OSError, UnicodeDecodeError):
                continue
            self._add("python", None, "ci", rel, names, False)

    @staticmethod
    def _fallback_package(rel):
        parts = rel.split("/")
        return parts[1] if len(parts) > 2 and parts[0] in ("crates", "firmware", "plugins", "fuzz") else parts[0]

    def _add(self, lang, package, d, src, names, ignored):
        for n in names:
            self.entries.append({"lang": lang, "package": package, "dir": d, "src": src, "name": n,
                                 "leaf": n.rsplit("::", 1)[-1], "ignored": ignored})

    def _in_file(self, e, f):
        """Is entry `e` declared in file `f`?"""
        if e["src"] == f:
            return True
        if e["lang"] != "rust" or self.cargo is None:
            return False
        # Cargo names a test binary by its root file; attribute it to the file
        # in the same package whose source declares a test of that name.
        if pkg_dir(self.root, f) != e["dir"]:
            return False
        names = self.decl.get(f, set()) | self.decl_ign.get(f, set())
        return e["leaf"] in names

    def resolve(self, ref):
        """Return (matching test names, problem or None) for `path::pattern`,
        `crate::pattern` or a bare `pattern`."""
        head, _, pattern = ref.rpartition("::")
        head = head or None
        if not pattern or pattern != pattern.strip() or re.search(r"\s", pattern):
            return [], f"'{ref}' is not a test name or glob"
        if pattern[0] in GLOB_CHARS:
            return [], f"'{ref}': a glob must start with a literal prefix (a bare '*' proves nothing)"
        is_glob = any(ch in pattern for ch in GLOB_CHARS)
        if head is None and is_glob:
            return [], f"'{ref}': a glob must be qualified with a crate or file (`crate::{pattern}`)"

        def selected(e):
            if head is None:
                return True
            if head.endswith(".py"):
                return e["lang"] == "python" and self._in_file(e, head)
            if head.endswith(".rs"):
                return e["lang"] == "rust" and self._in_file(e, head)
            if "/" in head:  # a directory
                d = head.rstrip("/")
                return e["src"] == d or e["src"].startswith(d + "/")
            return e["lang"] == "rust" and e["package"] == head

        hits = [e for e in self.entries if selected(e) and (fnmatch.fnmatchcase(e["leaf"], pattern) or fnmatch.fnmatchcase(e["name"], pattern))]
        live = [e for e in hits if not e["ignored"]]
        ign = [e for e in hits if e["ignored"]]
        if head is None and len({(e["lang"], e["package"], e["leaf"]) for e in hits}) > 1:
            return [], f"'{ref}' is not unique (matches {len(hits)} tests); qualify it with a crate or file"
        if is_glob and ign:
            return [], f"'{ref}': the glob also matches the ignored test '{ign[0]['name']}'"
        if live:
            return sorted({e["name"] for e in live}), None
        if ign:
            return [], f"'{ref}' only matches ignored tests"
        how = "cargo's list of runnable tests" if self.cargo is not None else "the source scan"
        return [], f"no test matches '{ref}' in {how}"


def load_cargo_list(path):
    """Parse the JSON made by ci/rust_test_list.py. Raises Problem if malformed."""
    try:
        with open(path, encoding="utf-8") as fh:
            data = json.load(fh)
    except (OSError, ValueError) as e:
        raise Problem(f"--rust-tests {path} cannot be read: {e}")
    tests = data.get("tests") if isinstance(data, dict) else None
    ok = isinstance(tests, list) and tests and all(
        isinstance(t, dict) and isinstance(t.get("package"), str) and isinstance(t.get("src"), str)
        and isinstance(t.get("name"), str) and isinstance(t.get("ignored"), bool) for t in tests)
    if not ok:
        raise Problem(f"--rust-tests {path} is not a non-empty list of tests from ci/rust_test_list.py")
    return data


# ---------------------------------------------------------------- loading


def load_toml(root, rel):
    try:
        return tomllib.loads(read(root, rel))
    except FileNotFoundError:
        raise Problem(f"{rel} is missing")
    except (tomllib.TOMLDecodeError, UnicodeDecodeError) as e:
        raise Problem(f"{rel} does not parse: {e}")


def load_names(root, errors):
    comps = {}
    try:
        data = load_toml(root, NAMES)
    except Problem as e:
        errors.append(str(e))
        return comps
    extra = set(data) - {"component"}
    if extra:
        errors.append(f"{NAMES}: unknown top-level keys {sorted(extra)}")
    items = data.get("component", [])
    if not isinstance(items, list):
        errors.append(f"{NAMES}: component must be a list of tables")
        return comps
    for c in items:
        if not isinstance(c, dict):
            errors.append(f"{NAMES}: a component is not a table")
            continue
        key = c.get("key")
        if not isinstance(key, str) or not re.fullmatch(r"[a-z]+", key):
            errors.append(f"{NAMES}: a component has a bad key {key!r}")
            continue
        if key in comps:
            errors.append(f"{NAMES}: duplicate component key '{key}'")
            continue
        for f in ("name", "meaning"):
            if not isinstance(c.get(f), str) or not c[f].strip():
                errors.append(f"{NAMES}: component '{key}' needs a non-empty {f}")
        for f in ("crates", "crates_pending"):
            if f in c and not (isinstance(c[f], list) and all(isinstance(x, str) for x in c[f])):
                errors.append(f"{NAMES}: component '{key}' {f} must be a list of strings")
        comps[key] = c
        for crate in c.get("crates", []) if isinstance(c.get("crates"), list) else []:
            if not os.path.isdir(os.path.join(root, "crates", crate)):
                errors.append(f"{NAMES}: component '{key}' names crate '{crate}' which is not in crates/")
    if os.path.exists(os.path.join(root, NAMES_DOC)):
        doc = read(root, NAMES_DOC)
        for key, c in comps.items():
            if isinstance(c.get("name"), str) and c["name"] not in doc:
                errors.append(f"{NAMES_DOC} does not mention '{c['name']}' ({key})")
    else:
        errors.append(f"{NAMES_DOC} is missing")
    return comps


def check_shape(rec, where, comp):
    """Schema errors for one record (a dict). Returns a list of messages."""
    errs = []
    keys = set(rec)
    if keys - REQ_KEYS:
        errs.append(f"{where}: unknown fields {sorted(keys - REQ_KEYS)}")
    if REQ_REQUIRED - keys:
        errs.append(f"{where}: missing fields {sorted(REQ_REQUIRED - keys)}")
    rid = rec.get("id")
    if not isinstance(rid, str) or not ID_RE.match(rid):
        errs.append(f"{where}: id {rid!r} is not SAW-<COMPONENT>-<AREA>-<n>")
    elif ID_RE.match(rid).group(1).lower() != comp:
        errs.append(f"{where}: id {rid} does not belong in the '{comp}' file")
    rev = rec.get("rev")
    if not isinstance(rev, int) or isinstance(rev, bool) or rev < 1:
        errs.append(f"{where}: rev must be an integer >= 1")
    if "level" in rec and not (isinstance(rec["level"], str) and rec["level"] in LEVELS):
        errs.append(f"{where}: level must be one of {sorted(LEVELS)}")
    for f in ("title", "source", "anchor", "source_sha"):
        if f in rec and (not isinstance(rec[f], str) or not rec[f].strip()):
            errs.append(f"{where}: {f} must be a non-empty string")
    tracks = rec.get("tracks")
    if not (isinstance(tracks, list) and all(isinstance(t, int) and not isinstance(t, bool) and t > 0 for t in tracks)):
        errs.append(f"{where}: tracks must be a list of issue numbers (empty only for a built requirement)")
        tracks = []
    if "waiver" in rec:
        w = rec["waiver"]
        if not (isinstance(w, dict) and set(w) <= {"reason", "ref"} and isinstance(w.get("reason"), str) and w["reason"].strip()
                and isinstance(w.get("ref", ""), str)):
            errs.append(f"{where}: waiver must be a table with a non-empty reason (and optional ref)")
    acc = rec.get("acceptance")
    if not (isinstance(acc, list) and acc and all(isinstance(a, dict) for a in acc)):
        errs.append(f"{where}: acceptance must be a non-empty list of tables")
        return errs
    seen = set()
    for a in acc:
        aw = f"{where} acceptance {a.get('id', '?')}"
        if set(a) - ACC_KEYS:
            errs.append(f"{aw}: unknown fields {sorted(set(a) - ACC_KEYS)}")
        if ACC_REQUIRED - set(a):
            errs.append(f"{aw}: missing fields {sorted(ACC_REQUIRED - set(a))}")
            continue
        if not isinstance(a["id"], str) or not a["id"].strip() or a["id"] in seen:
            errs.append(f"{aw}: id must be a unique non-empty string")
        else:
            seen.add(a["id"])
        if not (isinstance(a["kind"], str) and a["kind"] in KINDS):
            errs.append(f"{aw}: kind must be one of {sorted(KINDS)}")
        if not isinstance(a["ref"], str) or not a["ref"].strip():
            errs.append(f"{aw}: ref must be a non-empty string")
        if "pending" in a:
            p = a["pending"]
            if not isinstance(p, int) or isinstance(p, bool) or p <= 0:
                errs.append(f"{aw}: pending must be an issue number")
            elif p not in tracks:
                errs.append(f"{aw}: pending #{p} is not in tracks {tracks}")
    return errs


def load_records(root, comps, errors):
    """Return the shape-valid records, each with a `_component` and `_file`."""
    recs, ids = [], {}
    d = os.path.join(root, REQ_DIR)
    entries = sorted(os.listdir(d)) if os.path.isdir(d) else []
    files = [f for f in entries if f.endswith(".toml") and os.path.isfile(os.path.join(d, f))]
    for f in entries:
        if f not in files:
            errors.append(f"{REQ_DIR}/{f}: only .toml record files belong in {REQ_DIR}/ (remove it or rename it)")
    if not files:
        errors.append(f"{REQ_DIR}/ has no records")
    for f in files:
        rel = f"{REQ_DIR}/{f}"
        comp = f[:-5]
        if comp not in comps:
            errors.append(f"{rel}: '{comp}' is not a component in {NAMES}")
        try:
            data = load_toml(root, rel)
        except Problem as e:
            errors.append(str(e))
            continue
        if set(data) - {"requirement"}:
            errors.append(f"{rel}: unknown top-level keys {sorted(set(data) - {'requirement'})}")
        items = data.get("requirement", [])
        if not isinstance(items, list) or not items:
            errors.append(f"{rel}: no [[requirement]] records")
            continue
        for n, rec in enumerate(items, 1):
            where = f"{rel} #{n} ({rec.get('id', '?') if isinstance(rec, dict) else '?'})"
            if not isinstance(rec, dict):
                errors.append(f"{where}: not a table")
                continue
            try:
                shape = check_shape(rec, where, comp)
            except Exception as e:  # a malformed record must be an error, never a traceback
                shape = [f"{where}: malformed record ({type(e).__name__}: {e})"]
            rid = rec.get("id")
            if isinstance(rid, str):
                if rid in ids:
                    errors.append(f"{where}: duplicate id (also in {ids[rid]})")
                    continue
                ids[rid] = rel
            if shape:
                errors.extend(shape)
                continue
            rec["_component"], rec["_file"] = comp, rel
            recs.append(rec)
    return recs


# ---------------------------------------------------------------- evaluation


def find_tags(root):
    """{id: [(path, line, rev or None)]} for every `[impl->ID]` tag in code."""
    tags = {}
    for rel in walk(root, SCAN_DIRS, SCAN_SUFFIX):
        if rel in TAG_SKIP:
            continue
        try:
            text = read(root, rel)
        except (OSError, UnicodeDecodeError):
            continue
        for i, line in enumerate(text.splitlines(), 1):
            stripped = line.lstrip()
            if not stripped.startswith(("//", "#", "///", "//!")):
                continue
            for m in TAG_RE.finditer(line):
                tags.setdefault(m.group(1), []).append((rel, i, int(m.group(2)) if m.group(2) else None))
    return tags


def evaluate(root, rec, index, errors):
    """Compute (status, detail) for one record; append hard errors to `errors`."""
    rid = rec["id"]
    # Source paragraph: is the design the record was written from still there?
    drift = []
    try:
        src_path = repo_file(root, rec["source"])
        if src_path is None:
            raise OSError("not a file inside the repository")
        with open(src_path, encoding="utf-8") as fh:
            src = fh.read()
        try:
            unit = source_unit(src, rec["anchor"])
            if unit_hash(unit) != rec["source_sha"]:
                drift.append(
                    f"the design paragraph in {rec['source']} (anchor '{rec['anchor']}') changed since the record was written. "
                    f"Re-read it, then either revert the design change, or update the record in {rec['_file']} "
                    f"(title and acceptance, bump rev, set source_sha = \"{unit_hash(unit)}\")")
        except Problem as e:
            drift.append(
                f"{rec['source']}: {e} for anchor '{rec['anchor']}'. Restore the paragraph, or point the record's anchor in "
                f"{rec['_file']} at its new wording and re-pin it with `python3 ci/conformance.py hash {rec['source']} '<anchor>'`")
    except (OSError, UnicodeDecodeError):
        drift.append(f"source file {rec['source']} is missing. Restore it, or update `source` in {rec['_file']}")

    resolved, pending, missing = [], [], []
    for a in rec["acceptance"]:
        if a["kind"] == "file":
            ok = repo_file(root, a["ref"]) is not None
            problem = None if ok else f"'{a['ref']}' is not a regular file inside the repository"
        else:
            found, problem = index.resolve(a["ref"])
            ok = bool(found)
        if ok:
            resolved.append(a["id"])
        elif "pending" in a:
            pending.append(f"{a['ref']} (#{a['pending']})")
        else:
            missing.append(f"{a['id']}: {problem}")
    if "waiver" not in rec:  # a waiver records that the requirement was dropped, so its checks may be gone
        for m in missing:
            errors.append(
                f"{rid}: acceptance {m}. Write the test, correct the ref in {rec['_file']}, "
                f"or mark the item `pending = <issue>` (open an issue first and add it to tracks)")

    if drift:
        status = DRIFTED
        for d in drift:
            errors.append(f"{rid}: drifted: {d}")
    elif "waiver" in rec:
        status = WAIVED
    elif resolved and not pending and not missing:
        status = BUILT
    elif resolved:
        status = PARTIAL
    else:
        status = NOT_BUILT
    needs_issue = status in (PARTIAL, NOT_BUILT) and not rec["tracks"]
    if needs_issue:
        errors.append(f"{rid}: {status} but tracked by no issue. Open an issue for it and add the number to tracks in {rec['_file']}")
    return status, {"resolved": resolved, "pending": pending, "drift": drift, "needs_issue": needs_issue}


def run_check(root, cargo=None):
    """Return (rows, errors). rows are dicts ready for the table. `cargo` is the
    parsed --rust-tests list; None means the labelled pure-source fallback."""
    errors = []
    comps = load_names(root, errors)
    recs = load_records(root, comps, errors)
    index = TestIndex(root, cargo)
    tags = find_tags(root)
    known = {r["id"]: r for r in recs}
    rows = []
    for rec in recs:
        status, detail = evaluate(root, rec, index, errors)
        for path, line, rev in tags.get(rec["id"], []):
            if rev is not None and rev != rec["rev"]:
                status = DRIFTED
                detail["drift"].append(f"{path}:{line} tags rev {rev}, the record is rev {rec['rev']}")
                errors.append(f"{rec['id']}: drifted: {path}:{line} tags rev {rev}, the record is rev {rec['rev']}. "
                              f"Update the tag if the code was brought up to the new rev, or the record's rev if the meaning changed")
        rows.append({"rec": rec, "status": status, "detail": detail, "tags": len(tags.get(rec["id"], []))})
    for tid, where in sorted(tags.items()):
        if tid not in known:
            path, line, _ = where[0]
            errors.append(f"unknown id {tid} in an impl tag at {path}:{line}. Add a record with that id under {REQ_DIR}/, or fix the tag")
    return rows, errors


# ------------------------------------------------------------------ report


def counts(rows):
    c = {s: 0 for s in ORDER}
    for r in rows:
        c[r["status"]] += 1
    return c


def cell(text):
    return str(text).replace("|", "\\|").replace("\n", " ")


def render_table(rows):
    c = counts(rows)
    total = len(rows)
    needs = [r for r in rows if r["detail"]["needs_issue"]]
    drifted = [r for r in rows if r["status"] == DRIFTED]
    lines = [
        "### Needs an issue",
        "",
        *([f"- `{r['rec']['id']}` ({r['status']}): open an issue and add it to `tracks` in `{r['rec']['_file']}`." for r in needs] or ["None."]),
        "",
        "### Drifted",
        "",
        *([f"- `{r['rec']['id']}`: update the record in `{r['rec']['_file']}` or revert the design paragraph in `{r['rec']['source']}`." for r in drifted] or ["None."]),
        "",
        "### All requirements",
        "",
        f"**{total} requirements:** " + ", ".join(f"{c[s]} {s}" for s in ORDER if c[s] or s in (BUILT, PARTIAL, NOT_BUILT)) + ".",
        "",
        "| ID | Requirement | Status | Tracking | Still to prove |",
        "| --- | --- | --- | --- | --- |",
    ]
    for r in sorted(rows, key=lambda r: (r["rec"]["_component"], r["rec"]["id"])):
        rec, d = r["rec"], r["detail"]
        todo = list(d["pending"])
        if r["status"] == DRIFTED:
            todo = ["design changed: " + "; ".join(d["drift"])]
        elif r["status"] == WAIVED:
            todo = ["waived: " + rec["waiver"]["reason"]]
        elif r["status"] == NOT_BUILT and not todo:
            todo = ["nothing proves it yet"]
        tracks = ", ".join(f"#{t}" for t in rec["tracks"])
        lines.append(f"| `{rec['id']}` | {cell(rec['title'])} | {r['status']} | {tracks} | {cell(', '.join(todo)) or '-'} |")
    return "\n".join(lines)


STATUS_HEAD = """# Design conformance status (the X-ray)

Generated by `ci/conformance.py` from the records in `design/requirements/`. Status is computed from the code, never typed in: *built* means every acceptance check resolves to a test or file that exists, *partial* means some do and the rest wait on a tracking issue, *not built* means none do, *drifted* means the design paragraph changed without its record, *waived* means a recorded waiver. How it works is in [CONTRIBUTING.md](../CONTRIBUTING.md#design-conformance-the-x-ray); component names are in [docs/NAMES.md](../docs/NAMES.md).

"""
STATUS_TAIL = """
A test fails the build if this table is stale. Regenerate it with `python3 ci/conformance.py --bless`.
"""


def status_doc(table):
    return STATUS_HEAD + MARK_START + "\n\n" + table + "\n\n" + MARK_END + "\n" + STATUS_TAIL


def check_status_file(root, rows, bless):
    """The whole of design/STATUS.md is generated, so the whole file is compared."""
    want = status_doc(render_table(rows))
    path = os.path.join(root, STATUS_FILE)
    if bless:
        os.makedirs(os.path.dirname(path), exist_ok=True)
        with open(path, "w", encoding="utf-8") as fh:
            fh.write(want)
        return []
    try:
        current = read(root, STATUS_FILE)
    except FileNotFoundError:
        current = None
    except UnicodeDecodeError:
        current = ""
    if current != want:
        return [f"{STATUS_FILE} is stale or edited by hand: run `python3 ci/conformance.py --bless`"]
    return []


# ------------------------------------------------------------------ closure


def referenced_issues(text, repo):
    """Issue numbers of `repo` closed by `text` (same strict parse as
    regression_guard.py: a closing keyword, then #N, repo#N or an issue URL)."""
    repo = repo.lower()
    found = set()
    ref = r"(?:(?P<repo>[\w.-]+/[\w.-]+))?#(?P<n>\d+)\b"
    url = r"https://github\.com/(?P<urepo>[\w.-]+/[\w.-]+)/issues/(?P<un>\d+)\b"
    pattern = re.compile(rf"(?<![\w-]){CLOSING}\b:?\s+(?:{ref}|{url})", REQ_FLAGS)
    for m in pattern.finditer(text or ""):
        if m.group("n"):
            if m.group("repo") is None or m.group("repo").lower() == repo:
                found.add(int(m.group("n")))
        elif m.group("urepo").lower() == repo:
            found.add(int(m.group("un")))
    return sorted(found)


def closure_problems(rows, closed):
    out = []
    for n in closed:
        for r in rows:
            if n in r["rec"]["tracks"] and r["status"] not in (BUILT, WAIVED):
                out.append(f"This PR closes #{n}, but {r['rec']['id']} ({r['rec']['title']}) tracks #{n} and is {r['status']}.")
    return out


# --------------------------------------------------------------------- main


def main(argv):
    args = list(argv)
    root = os.getcwd()
    if "--root" in args:
        i = args.index("--root")
        root = args[i + 1]
        del args[i:i + 2]
    bless = "--bless" in args
    args = [a for a in args if a != "--bless"]
    opts = {}
    for flag in ("--rust-tests", "--commits"):
        if flag in args:
            i = args.index(flag)
            if i + 1 >= len(args):
                print(f"{flag} needs a file", file=sys.stderr)
                return 2
            opts[flag] = args[i + 1]
            del args[i:i + 2]
    cmd = args[0] if args else "check"

    if cmd == "hash":
        if len(args) != 3:
            print("usage: conformance.py hash FILE ANCHOR", file=sys.stderr)
            return 2
        try:
            print(unit_hash(source_unit(read(root, args[1]), args[2])))
        except (Problem, OSError) as e:
            print(f"error: {e}", file=sys.stderr)
            return 1
        return 0
    if cmd not in ("check", "closure") or len(args) > 1:
        print(__doc__)
        return 2

    cargo, errors = None, []
    if "--rust-tests" in opts:
        try:
            cargo = load_cargo_list(opts["--rust-tests"])
        except Problem as e:
            print(f"::error::{e}")
            return 1
    else:
        print("::warning::No --rust-tests list: using the pure-source fallback, which can still count tests that "
              "never compile or run. CI passes cargo's own list (ci/rust_test_list.py).")
    rows, errors = run_check(root, cargo)
    if cmd == "check":
        errors += check_status_file(root, rows, bless)
    else:
        repo = os.environ.get("REPO") or "rafe-walker/sawbones-labs"
        texts = [os.environ.get("PR_TITLE", ""), os.environ.get("PR_BODY", "")]
        if "--commits" in opts:  # GitHub also closes issues from commit messages that reach the base branch
            try:
                with open(opts["--commits"], encoding="utf-8", errors="replace") as fh:
                    texts.append(fh.read())
            except OSError as e:
                print(f"::error::--commits {opts['--commits']} cannot be read: {e}")
                return 1
        closed = referenced_issues("\n".join(texts), repo)
        print(f"Closing keywords name: {', '.join('#' + str(n) for n in closed) or 'no issues'}.")
        errors += closure_problems(rows, closed)

    c = counts(rows)
    print("Design conformance: " + ", ".join(f"{c[s]} {s}" for s in ORDER) + f" (of {len(rows)}).")
    for e in errors:
        print(f"::error::{e}")
    if errors:
        print("The design and the code must agree, or the record must say so. See CONTRIBUTING.md, 'Design conformance'.")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
