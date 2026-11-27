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
scan can still be fooled. Python tests are parsed with `ast` (a fail-closed allow-list,
see python_scan) and must be methods of a TestCase in ci/test_*.py; with `--py-tests
FILE`, made by ci/py_test_list.py from a real `unittest discover`, a Python ref must
also be recorded there as passed.

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
import unittest

REQ_DIR = "design/requirements"
NAMES = "design/names.toml"
NAMES_DOC = "docs/NAMES.md"
STATUS_FILE = "design/STATUS.md"
MARK_START = "<!-- status:start (generated: python3 ci/conformance.py --bless) -->"
MARK_END = "<!-- status:end -->"

BUILT, PARTIAL, NOT_BUILT, DRIFTED, WAIVED = "built", "partial", "not built", "drifted", "waived"
ORDER = [BUILT, PARTIAL, NOT_BUILT, DRIFTED, WAIVED]

REQ_KEYS = {"id", "rev", "level", "title", "source", "anchor", "source_sha", "tracks", "acceptance", "waiver", "hws"}
REQ_REQUIRED = {"id", "rev", "title", "source", "anchor", "source_sha", "tracks", "acceptance"}
ACC_KEYS = {"id", "kind", "ref", "pending", "note"}
ACC_REQUIRED = {"id", "kind", "ref"}
KINDS = {"test", "file"}
LEVELS = {"MUST", "SHOULD", "MAY"}
HWS_ID_RE = re.compile(r"^HWS-[A-Z]+-[1-9][0-9]*$")
HWS_GLOB = "standards/hws"
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
    indent = len(lines[start]) - len(lines[start].lstrip())
    bullet = bool(_BULLET.match(lines[start]))
    end = i
    j = end + 1
    while j < len(lines):
        line = lines[j]
        if not line.strip():  # a blank line belongs to the item only if deeper-indented text follows
            k = j
            while k < len(lines) and not lines[k].strip():
                k += 1
            if bullet and k < len(lines) and len(lines[k]) - len(lines[k].lstrip()) > indent:
                end, j = k, k + 1
                continue
            break
        deeper = len(line) - len(line.lstrip()) > indent
        if bullet and deeper:  # continuation lines and nested sub-bullets are part of the item
            end, j = j, j + 1
        elif not _BULLET.match(line) and not line.lstrip().startswith("|"):
            end, j = j, j + 1  # the rest of a paragraph, or a lazy continuation line of a bullet
        else:
            break
    return "\n".join(lines[start:end + 1])


def unit_hash(unit):
    norm = " ".join(unit.split())
    return hashlib.sha256(norm.encode("utf-8")).hexdigest()[:16]


# ------------------------------------------------------------ test discovery
#
# Two sources of truth for Rust tests:
#   * cargo's own list (--rust-tests FILE, made by ci/rust_test_list.py from the
#     libtest binaries `cargo test --workspace` runs, each run with --list, with and
#     without --ignored). A test that does not compile, is cfg'd out, sits in a file
#     no `mod` reaches or in an uninvoked macro is simply not in that list.
#   * a source scan. It must also agree: a cargo-listed test only counts if a
#     `#[test] fn` with exactly that module path is written in a file that `mod`
#     declarations really reach from the binary's root (inline mods included). A
#     test binary, build.rs or proc macro in a hostile PR can still print or add
#     what it likes, so against a hostile PR the guarantee is this scan plus review.
#   * with no cargo list (a local run) the scan alone is the labelled fallback.
# Python tests are parsed with `ast`.

GLOB_CHARS = "*?["
_ATTRS = r"((?:#\s*\[[^\]]*\]\s*)*)"
_VIS = r"(?:pub\s*(?:\(\s*[^)]*\))?\s+)?"
_FN = re.compile(_ATTRS + _VIS + r"(?:async\s+|const\s+|unsafe\s+)*fn\s+(\w+)")
_MODDECL = re.compile(_ATTRS + _VIS + r"(?<![\w#])mod\s+(?:r#)?(\w+)\s*([;{])")
_MOD_TOKEN = re.compile(r"(?<![\w#])mod\b")
_INCLUDE = re.compile(r"(?<!\w)include\s*!")
_MACRO = re.compile(r"macro_rules\s*!\s*\w+\s*[{(\[]")
_TEST_ATTR = re.compile(r"#\s*\[\s*(?:\w+::)*test\b")
_IGNORE_ATTR = re.compile(r"#\s*\[\s*ignore\b|cfg_attr\s*\([^\]]*\bignore\b")
_PATH_ATTR = re.compile(r"#\s*\[\s*path\b")
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


_MACRO_CALL = re.compile(r"(?<![\w])(?!macro_rules\b)[A-Za-z_][\w:]*\s*!\s*[{(\[]")
_PATH_ANY = re.compile(r"\bpath\s*=")  # `#[path = ..]` and `#[cfg_attr(.., path = ..)]`


def _match_close(s, i):
    """Index just past the bracket that closes the one at s[i] (blanked code)."""
    pairs = {"(": ")", "[": "]", "{": "}"}
    stack, n = [], len(s)
    while i < n:
        c = s[i]
        if c in pairs:
            stack.append(pairs[c])
        elif stack and c == stack[-1]:
            stack.pop()
            if not stack:
                return i + 1
        i += 1
    return n


def _attached(code, pos):
    """Can the declaration that starts at `pos` have lost attributes or a visibility
    to an odd spelling? True when what precedes it is a statement end, a block edge,
    the start of the file or an inner attribute `#![..]`; anything else (a stray
    `]`, `)` or `pub`) means an outer attribute may sit out of the scanner's view."""
    head = code[:pos].rstrip()
    if not head or head[-1] in "{};":
        return True
    if head[-1] == "]":
        depth, i = 0, len(head) - 1
        while i >= 0:
            depth += head[i] == "]"
            depth -= head[i] == "["
            if depth == 0:
                break
            i -= 1
        return head[:i].rstrip().endswith("!") and head[:i].rstrip()[:-1].rstrip().endswith("#")
    return False


def scan_rust(src):
    """What a source scan sees in one Rust file, with nested comments, strings,
    `macro_rules!` bodies and modules or fns behind a never-true cfg removed:
      tests:   [(inline module path, fn name, ignored)]
      mods:    [(inline module path, name, attrs)] for out-of-line `mod name;`.
      tainted: None, or why this file cannot be trusted (see below).
      macro_hazard: None, or where a `macro_rules!` body (read with only comments and
        strings blanked) contains `mod` or `include!`: such a macro can declare the live
        module behind the declaration the scanner follows, here or in any file that
        uses it, so TestIndex refuses every Rust credit in the workspace.
    Module names a scope declares more than once (a cfg'd-out `mod b;` next to a
    live `mod b {}`, inline or not), or declares with a path attribute (even inside
    `cfg_attr`), are ambiguous without evaluating cfg: all their declarations, and
    the tests inside them, are dropped. `mod x;` inside a macro invocation is not
    followed. FAIL CLOSED: a `mod` keyword the declaration pattern did not parse
    (an odd spelling, a macro-built item) or an `include!` taints the file, and a
    tainted file credits no test and has its `mod` children ignored."""
    code = blank_rust(src)
    hazard = None
    for m in list(_MACRO.finditer(code)):
        code_open = m.end() - 1
        end = _match_close(code, code_open)
        body = code[m.start():end]
        bad = _MOD_TOKEN.search(body) or _INCLUDE.search(body)
        if bad and hazard is None:
            line = code.count("\n", 0, m.start() + bad.start()) + 1
            hazard = f"line {line}: a `macro_rules!` body contains `{bad.group(0).strip()}`"
        code = _blank_span(code, m.start(), end)
    decls = list(_MODDECL.finditer(code))
    kw_positions = {code.rfind("mod", m.start(), m.start(2)) for m in decls}
    unparsed = [t for t in _MOD_TOKEN.finditer(code) if t.start() not in kw_positions]
    taint = None
    stray = [m for m in decls if not _attached(code, m.start())]
    if stray:
        line = code.count("\n", 0, stray[0].start()) + 1
        taint = f"line {line}: a `mod` whose attributes or visibility the scanner may not see"
    elif unparsed:
        line = code.count("\n", 0, unparsed[0].start()) + 1
        taint = f"line {line}: a `mod` the scanner could not parse as a declaration"
    elif _INCLUDE.search(code):
        line = code.count("\n", 0, _INCLUDE.search(code).start()) + 1
        taint = f"line {line}: `include!` pulls in text the scanner cannot see"
    if taint:
        return {"tests": [], "mods": [], "tainted": taint, "macro_hazard": hazard}
    spans0 = sorted((m.start(), _match_brace(code, m.end() - 1), m.group(2)) for m in decls if m.group(3) == "{")

    def scope_at(p, spans):
        return [name for a, b, name in spans if a < p < b]  # strict: an inline module is declared in its parent's scope

    seen, refused = {}, set()
    for m in decls:
        key = (tuple(scope_at(m.start(), spans0)), m.group(2))
        seen[key] = seen.get(key, 0) + 1
        if seen[key] > 1 or _PATH_ANY.search(m.group(1)):
            refused.add(key)
    macro_spans = [(m.start(), _match_close(code, m.end() - 1)) for m in _MACRO_CALL.finditer(code)]
    for m in decls:
        if m.group(3) == "{" and _NEVER_CFG.search(m.group(1)):
            code = _blank_span(code, m.start(), _match_brace(code, m.end() - 1))

    def inline_ok(path):
        return not any((tuple(path[:k]), path[k]) in refused for k in range(len(path)))

    tests = []
    for m in _FN.finditer(code):
        attrs = m.group(1)
        if _TEST_ATTR.search(attrs) and not _NEVER_CFG.search(attrs):
            path = scope_at(m.start(), spans0)
            if inline_ok(path):
                tests.append((path, m.group(2), bool(_IGNORE_ATTR.search(attrs))))
    mods = []
    for m in decls:
        if m.group(3) != ";" or _NEVER_CFG.search(m.group(1)) or any(a <= m.start() < b for a, b in macro_spans):
            continue
        scope = scope_at(m.start(), spans0)
        if inline_ok(scope) and (tuple(scope), m.group(2)) not in refused:
            mods.append((scope, m.group(2), m.group(1)))
    return {"tests": tests, "mods": mods, "tainted": None, "macro_hazard": hazard}


def rust_tests(src):
    """(live, ignored) fn names found by a source scan (module paths dropped)."""
    info = scan_rust(src)
    return ({n for _, n, ign in info["tests"] if not ign}, {n for _, n, ign in info["tests"] if ign})


_SKIP_DECORATORS = {"skip", "skipIf", "skipUnless", "expectedFailure"}
_TESTCASE_NAMES = {"TestCase", "IsolatedAsyncioTestCase"}
_MODULE_HOOKS = {"load_tests", "setUpModule", "tearDownModule"}
# Anything a TestCase class already has (run, setUp, setUpClass, asyncSetUp, __init__,
# __new__, __init_subclass__, __call__, debug, ...) must not be redefined in a counted class.
_TC_RESERVED = (set(dir(unittest.IsolatedAsyncioTestCase)) | set(dir(object))) - {"__doc__"}
_SAFE_CALLS = {"pathlib.Path", "re.compile", "importlib.util.spec_from_file_location", "importlib.util.module_from_spec",
               "sys.path.insert", "sys.path.append", "unittest.skip", "unittest.skipIf", "unittest.skipUnless"}
_SAFE_CALL_PREFIX = ("os.path.",)
_PATH_METHODS = {"with_name", "with_suffix", "resolve", "absolute", "joinpath"}
_PLAIN_DECORATORS = {"classmethod", "staticmethod", "property"}


def _has_yield(fn):
    """Does this function's own body (not a nested def, lambda or class) yield?"""
    stack = list(fn.body)
    while stack:
        n = stack.pop()
        if isinstance(n, (ast.Yield, ast.YieldFrom)):
            return True
        if not isinstance(n, (ast.FunctionDef, ast.AsyncFunctionDef, ast.Lambda, ast.ClassDef)):
            stack.extend(ast.iter_child_nodes(n))
    return False


class _Refuse(Exception):
    def __init__(self, node, why):
        super().__init__(why)
        self.node, self.why = node, why


def _dotted(node):
    parts = []
    while isinstance(node, ast.Attribute):
        parts.append(node.attr)
        node = node.value
    if isinstance(node, ast.Name):
        return ".".join([node.id] + parts[::-1])
    return None


class _PyScan:
    """A fail-closed reading of one Python test module. The module is counted only if
    everything it runs at import time is on a short allow-list; see `python_scan`."""

    def __init__(self, src, tree):
        self.src, self.tree = src, tree
        self.imports, self.reasons = {}, []
        self.skip_aliases, self.specs, self.mods = set(), set(), set()
        self.file_names, self.path_names = {"__file__"}, set()

    def stmt(self, node):
        seg = (ast.get_source_segment(self.src, node) or "").strip().splitlines()
        text = seg[0] if seg else type(node).__name__
        return (node.lineno, text[:70])

    def refuse(self, node, why):
        raise _Refuse(node, why)

    def full(self, node):
        d = _dotted(node)
        if d is None:
            return None
        head, _, rest = d.partition(".")
        base = self.imports.get(head)
        if base is None:
            return None
        return base + ("." + rest if rest else "")

    def safe_call(self, node):
        if any(k.arg is None for k in node.keywords):
            return False
        if not all(self.safe(a) for a in node.args) or not all(self.safe(k.value) for k in node.keywords):
            return False
        full = self.full(node.func)
        if full is not None and (full in _SAFE_CALLS or full.startswith(_SAFE_CALL_PREFIX)):
            return True
        f = node.func
        return isinstance(f, ast.Attribute) and f.attr in _PATH_METHODS and self.is_path(f.value)

    def is_path(self, node):
        if isinstance(node, ast.Name):
            return node.id in self.path_names
        return isinstance(node, ast.Call) and self.safe_call(node) and (
            self.full(node.func) == "pathlib.Path" or (isinstance(node.func, ast.Attribute) and node.func.attr in _PATH_METHODS))

    def safe(self, node):
        """An expression that only builds constants, names, attributes, containers and
        calls on the allow-list, so evaluating it cannot run PR-chosen code."""
        if node is None or isinstance(node, (ast.Constant, ast.Name)):
            return True
        if isinstance(node, ast.Attribute):
            return self.safe(node.value)
        if isinstance(node, (ast.Tuple, ast.List, ast.Set)):
            return all(self.safe(e) for e in node.elts)
        if isinstance(node, ast.Dict):
            return all(k is not None and self.safe(k) for k in node.keys) and all(self.safe(v) for v in node.values)
        if isinstance(node, ast.BinOp):
            return self.safe(node.left) and self.safe(node.right)
        if isinstance(node, ast.UnaryOp):
            return self.safe(node.operand)
        if isinstance(node, ast.Subscript):
            return self.safe(node.value) and self.safe(node.slice)
        if isinstance(node, ast.Slice):
            return all(self.safe(x) for x in (node.lower, node.upper, node.step))
        if isinstance(node, ast.JoinedStr):
            return all(self.safe(v) for v in node.values)
        if isinstance(node, ast.FormattedValue):
            return self.safe(node.value) and self.safe(node.format_spec)
        if isinstance(node, ast.Call):
            return self.safe_call(node)
        return False

    def need_safe(self, node, what):
        if not self.safe(node):
            self.refuse(node, f"{what} is not a constant, name, attribute or allow-listed call, so it may run PR-chosen code at import")

    def check_signature(self, fn):
        a = fn.args
        for d in a.defaults + [d for d in a.kw_defaults if d is not None]:
            self.need_safe(d, "a default argument")
        for arg in a.posonlyargs + a.args + a.kwonlyargs + [x for x in (a.vararg, a.kwarg) if x]:
            if arg.annotation is not None:
                self.need_safe(arg.annotation, "an annotation")
        if fn.returns is not None:
            self.need_safe(fn.returns, "an annotation")

    def is_skip(self, d):
        if isinstance(d, ast.Name):
            return d.id in self.skip_aliases
        if isinstance(d, ast.Call):
            return self.safe_call(d) and self.full(d.func) in ("unittest.skip", "unittest.skipIf", "unittest.skipUnless")
        return self.full(d) == "unittest.expectedFailure"

    def is_testcase_base(self, b, local):
        if isinstance(b, ast.Name):
            return b.id in self.imports and self.imports[b.id] in {"unittest." + n for n in _TESTCASE_NAMES} or b.id in local
        return self.full(b) in {"unittest." + n for n in _TESTCASE_NAMES}

    def module_level(self):
        body = self.tree.body
        binds = {}
        for node in body:
            names = []
            if isinstance(node, ast.Import):
                names = [(a.asname or a.name.split(".")[0], "import") for a in node.names]
            elif isinstance(node, ast.ImportFrom):
                if any(a.name == "*" for a in node.names):
                    self.refuse(node, "`import *` can rebind anything")
                names = [(a.asname or a.name, "import") for a in node.names]
            elif isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef)):
                names = [(node.name, "def")]
            elif isinstance(node, (ast.Assign, ast.AnnAssign)):
                for t in node.targets if isinstance(node, ast.Assign) else [node.target]:
                    if not isinstance(t, ast.Name):
                        self.refuse(node, "an assignment to an attribute, item or tuple can rebind a test or unittest itself")
                    names.append((t.id, "assign"))
            for n, kind in names:
                if n in _MODULE_HOOKS:
                    self.refuse(node, f"`{n}` changes what discovery runs")
                binds.setdefault(n, []).append((kind, node))
        for n, lst in binds.items():
            if any(k == "import" for k, _ in lst) and len(lst) > 1:
                self.refuse(lst[-1][1], f"`{n}` is imported and then bound again")
        self.binds = binds
        for node in body:
            self.top_statement(node)

    def top_statement(self, node):
        if isinstance(node, ast.Import):
            for a in node.names:
                self.imports[a.asname or a.name.split(".")[0]] = a.name if a.asname else a.name.split(".")[0]
        elif isinstance(node, ast.ImportFrom):
            if node.level:
                for a in node.names:
                    self.imports.pop(a.asname or a.name, None)
                return
            for a in node.names:
                self.imports[a.asname or a.name] = f"{node.module}.{a.name}"
        elif isinstance(node, ast.Expr):
            self.top_expr(node)
        elif isinstance(node, (ast.Assign, ast.AnnAssign)):
            value = node.value
            if value is None:
                return
            self.need_safe(value, "an assignment value")
            targets = node.targets if isinstance(node, ast.Assign) else [node.target]
            names = {t.id for t in targets}
            if self.mentions_file(value):
                self.file_names |= names
            if self.is_path(value):
                self.path_names |= names
            if isinstance(value, ast.Call):
                full = self.full(value.func)
                if full in ("unittest.skip", "unittest.skipIf", "unittest.skipUnless"):
                    self.skip_aliases |= names
                elif full == "importlib.util.spec_from_file_location":
                    self.check_spec(value)
                    self.specs |= names
                elif full == "importlib.util.module_from_spec" and len(value.args) == 1 and isinstance(value.args[0], ast.Name) \
                        and value.args[0].id in self.specs:
                    self.mods |= names
                elif full == "importlib.util.module_from_spec":
                    self.refuse(node, "module_from_spec of something that is not a spec from this file's directory")
        elif isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
            if node.decorator_list:
                self.refuse(node, "a decorator on a module-level function runs code at import")
            self.check_signature(node)
        elif isinstance(node, ast.ClassDef):
            self.check_class_header(node)
        elif isinstance(node, ast.If) and self.is_main_guard(node):
            pass
        else:
            self.refuse(node, f"a module-level `{type(node).__name__}` statement runs code at import")

    def mentions_file(self, node):
        return any(isinstance(n, ast.Name) and n.id in self.file_names for n in ast.walk(node))

    def check_spec(self, call):
        if len(call.args) < 2 or not self.mentions_file(call.args[1]):
            self.refuse(call, "a module spec must be built from a path derived from __file__ (a sibling in ci/)")
        for n in ast.walk(call.args[1]):
            if isinstance(n, ast.Constant) and isinstance(n.value, str) and (".." in n.value or n.value.startswith("/")):
                self.refuse(call, "a module spec path must stay inside ci/")

    def top_expr(self, node):
        v = node.value
        if isinstance(v, ast.Constant) and isinstance(v.value, str):
            return
        if isinstance(v, ast.Call):
            full = self.full(v.func)
            if full in ("sys.path.insert", "sys.path.append") and self.safe(v):
                return
            f = v.func
            if (isinstance(f, ast.Attribute) and f.attr == "exec_module" and isinstance(f.value, ast.Attribute) and f.value.attr == "loader"
                    and isinstance(f.value.value, ast.Name) and f.value.value.id in self.specs and len(v.args) == 1 and not v.keywords
                    and isinstance(v.args[0], ast.Name) and v.args[0].id in self.mods):
                return
        self.refuse(node, "a module-level call runs code at import")

    @staticmethod
    def is_main_guard(node):
        t = node.test
        return (isinstance(t, ast.Compare) and isinstance(t.left, ast.Name) and t.left.id == "__name__" and len(t.ops) == 1
                and isinstance(t.ops[0], ast.Eq) and len(t.comparators) == 1 and isinstance(t.comparators[0], ast.Constant)
                and t.comparators[0].value == "__main__" and not node.orelse and len(node.body) == 1
                and isinstance(node.body[0], ast.Expr) and isinstance(node.body[0].value, ast.Call)
                and _dotted(node.body[0].value.func) == "unittest.main" and not node.body[0].value.args
                and not node.body[0].value.keywords)

    def check_class_header(self, node):
        if node.keywords:
            self.refuse(node, "class keywords (a metaclass) run code at import")
        for b in node.bases:
            self.need_safe(b, "a base class")
        for d in node.decorator_list:
            if not self.is_skip(d):
                self.refuse(node, "a class decorator runs code at import")
        for item in node.body:
            if isinstance(item, (ast.FunctionDef, ast.AsyncFunctionDef)):
                for d in item.decorator_list:
                    plain = isinstance(d, ast.Name) and d.id in _PLAIN_DECORATORS and d.id not in self.binds
                    if not (plain or self.is_skip(d)):
                        self.refuse(item, "a method decorator runs code at import")
                self.check_signature(item)
            elif isinstance(item, ast.Assign):
                if not all(isinstance(t, ast.Name) for t in item.targets):
                    self.refuse(item, "a class-body assignment to an attribute or tuple")
                self.need_safe(item.value, "a class-body assignment value")
            elif isinstance(item, ast.AnnAssign):
                if not isinstance(item.target, ast.Name):
                    self.refuse(item, "a class-body assignment to an attribute")
                self.need_safe(item.value, "a class-body assignment value")
            elif isinstance(item, ast.Delete):
                if not all(isinstance(t, ast.Name) for t in item.targets):
                    self.refuse(item, "a class-body delete of an attribute")
            elif isinstance(item, ast.Pass) or (isinstance(item, ast.Expr) and isinstance(item.value, ast.Constant)):
                pass
            else:
                self.refuse(item, f"a class-body `{type(item).__name__}` statement runs code at import")

    def classes(self):
        local, skipped, bad, found, asyncs = set(), set(), set(), set(), set()
        async_full = "unittest.IsolatedAsyncioTestCase"
        for node in self.tree.body:
            if not isinstance(node, ast.ClassDef) or not any(self.is_testcase_base(b, local) for b in node.bases):
                continue
            local.add(node.name)
            if any((isinstance(b, ast.Name) and (b.id in asyncs or self.imports.get(b.id) == async_full)) or self.full(b) == async_full
                   for b in node.bases):
                asyncs.add(node.name)
            if any(not self.is_testcase_base(b, local) and not (isinstance(b, ast.Name) and b.id in local) for b in node.bases):
                self.reasons.append(self.stmt(node) + ("a base class the scanner cannot vouch for",))
                bad.add(node.name)
                continue
            if any(isinstance(b, ast.Name) and b.id in bad for b in node.bases):
                bad.add(node.name)
                self.reasons.append(self.stmt(node) + ("derives from a class that is not counted",))
                continue
            if node.decorator_list or any(isinstance(b, ast.Name) and b.id in skipped for b in node.bases):
                skipped.add(node.name)
                continue
            if self.binds[node.name][-1][1] is not node:
                continue  # a later binding of the same name replaced this class
            defs, flagged, reserved = {}, False, None
            for item in node.body:
                if isinstance(item, (ast.FunctionDef, ast.AsyncFunctionDef)):
                    if item.name in _TC_RESERVED:
                        reserved = reserved or item
                    defs[item.name] = not item.decorator_list
                    if item.name.startswith("test_") and defs[item.name]:
                        if _has_yield(item):
                            defs[item.name] = False
                            self.reasons.append(self.stmt(item) + ("a test body that yields only builds a generator; it never runs",))
                        elif isinstance(item, ast.AsyncFunctionDef) and node.name not in asyncs:
                            defs[item.name] = False
                            self.reasons.append(self.stmt(item) + ("an `async def` test outside IsolatedAsyncioTestCase only builds a "
                                                                  "coroutine; it never runs",))
                elif isinstance(item, (ast.Assign, ast.AnnAssign)):
                    for t in item.targets if isinstance(item, ast.Assign) else [item.target]:
                        if t.id == "__unittest_skip__" and isinstance(item.value, ast.Constant) and item.value.value:
                            flagged = True
                        elif t.id in _TC_RESERVED:
                            reserved = reserved or item
                        defs[t.id] = False
                elif isinstance(item, ast.Delete):
                    for t in item.targets:
                        defs.pop(t.id, None)
            if reserved is not None:
                bad.add(node.name)
                name = getattr(reserved, "name", None) or "an attribute"
                self.reasons.append(self.stmt(reserved) + (f"class {node.name} redefines `{name}`, which TestCase already has (run, setUp, "
                                                          "setUpClass, __init__, ... change what runs)",))
                continue
            if flagged:
                skipped.add(node.name)
                continue
            found |= {n for n, ok in defs.items() if ok and n.startswith("test_")}
        return found


def python_scan(src):
    """(test method names, reasons) for what `unittest` discovery would really run.

    A module is counted only if everything it runs at import time is on a short
    allow-list: imports, docstrings, classes and functions without decorators (apart
    from unittest skips), assignments whose right-hand side is built from constants,
    names, attributes, containers and calls of `os.path.*`, `pathlib.Path`,
    `re.compile`, `importlib.util.spec_from_file_location` / `module_from_spec`
    (for a file next to the test) and `spec.loader.exec_module(module)`, `sys.path`
    inserts, and the exact `if __name__ == "__main__": unittest.main()`. Anything else
    (a `raise`, `if`, `try`, any other call, `load_tests` or `setUpModule` in any form,
    an imported name bound again, a class with keywords) means the module is not
    counted. A class counts only if it derives from `unittest.TestCase` and defines
    none of TestCase's own attributes (`run`, `setUp`, `setUpClass`, ...). Its `test_*`
    methods count when they carry no decorator, do not `yield`, are not `async` outside an
    `IsolatedAsyncioTestCase`, the last definition of a name wins,
    and skips (decorators, `__unittest_skip__`, inherited) exclude them.
    `reasons` lists (line, statement, why) for every module or class left out."""
    try:
        tree = ast.parse(src)
    except (SyntaxError, ValueError) as e:
        return set(), [(getattr(e, "lineno", 1) or 1, "", f"the file does not parse: {e.__class__.__name__}")]
    scan = _PyScan(src, tree)
    try:
        scan.module_level()
    except _Refuse as r:
        return set(), [scan.stmt(r.node) + (r.why,)]
    found = scan.classes()
    return found, scan.reasons


def python_tests(src):
    return python_scan(src)[0]


def harness_false_srcs(root, pkg):
    """Repo-relative source paths of the targets that a package manifest declares
    with `harness = false` (lib, bin, test, example, bench), with the default path
    cargo would use when the manifest gives none. Their `--list` output is the
    target's own, so no test can count from them."""
    try:
        with open(os.path.join(root, pkg, "Cargo.toml"), "rb") as fh:
            manifest = tomllib.load(fh)
    except (OSError, ValueError):
        return set()
    out = set()

    def add(path):
        out.add(os.path.normpath(os.path.join(pkg, path)).replace(os.sep, "/"))

    lib = manifest.get("lib", {})
    if isinstance(lib, dict) and lib.get("harness") is False:
        add(lib.get("path", "src/lib.rs"))
    for kind, folder in (("bin", "src/bin"), ("test", "tests"), ("example", "examples"), ("bench", "benches")):
        for t in manifest.get(kind, []) if isinstance(manifest.get(kind, []), list) else []:
            if not (isinstance(t, dict) and t.get("harness") is False):
                continue
            name = t.get("name") if isinstance(t.get("name"), str) else ""
            if isinstance(t.get("path"), str):
                add(t["path"])
            else:
                for cand in (f"{folder}/{name}.rs", f"{folder}/{name}/main.rs") + (("src/main.rs",) if kind == "bin" else ()):
                    add(cand)
    return out


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

    def __init__(self, root, cargo=None, pylist=None):
        self.root = root
        self.cargo = cargo
        self.pylist = pylist
        self.refusals = []  # (file, line, statement, why): what was left out and why
        self.macro_hazard = None
        self._scans = {}
        for rel in walk(root, ("crates", "firmware", "plugins", "fuzz"), (".rs",)):
            info = self._scan(rel)
            if info and info["macro_hazard"]:
                self.macro_hazard = f"{rel}:{info['macro_hazard']}"
                self.refusals.append((rel, 0, "", info["macro_hazard"] + "; every Rust test is refused workspace-wide"))
                break
        self.entries = []   # {"lang", "package", "src", "files", "name", "leaf", "ignored"}
        if self.macro_hazard:
            pass  # no Rust test is credited; see resolve()
        elif cargo is None:
            for rel in walk(root, ("crates", "firmware", "plugins", "fuzz"), (".rs",)):
                info = self._scan(rel)
                if info and info["tainted"]:
                    self.refusals.append((rel, 0, "", info["tainted"]))
                for _, name, ign in (info["tests"] if info else []):
                    self._add("rust", self._fallback_package(rel), rel, {rel}, name, name, ign)
        else:
            reach = {}
            no_harness = {}
            for t in cargo["tests"]:
                d = pkg_dir(root, t["src"])
                if d not in no_harness:
                    no_harness[d] = harness_false_srcs(root, d)
                if t["src"] in no_harness[d]:
                    continue  # a harness = false target: the list entry is not libtest's
                if t["src"] not in reach:
                    reach[t["src"]] = self._reach(t["src"])
                # Cargo's list says what compiles and runs. The source must also declare
                # a test with exactly this module path in a file that `mod` really
                # reaches from the binary's root; a source #[ignore] wins over the list.
                files = {f for f, names in reach[t["src"]].items() if t["name"] in names}
                if len(files) == 1:  # the same full name reached from two files is ambiguous: refused
                    ign = bool(t["ignored"]) or any(reach[t["src"]][f][t["name"]] for f in files)
                    self._add("rust", t["package"], t["src"], files, t["name"], t["name"].rsplit("::", 1)[-1], ign)
        # Only files `unittest discover -s ci -p 'test_*.py'` collects.
        for rel in walk(root, ("ci",), (".py",)):
            base = os.path.basename(rel)
            if "/" in rel[len("ci/"):] or not (base.startswith("test_") and base.endswith(".py")):
                continue
            try:
                names, reasons = python_scan(read(root, rel))
            except (OSError, UnicodeDecodeError):
                continue
            for line, text, why in reasons:
                self.refusals.append((rel, line, text, why))
            for n in names:
                if pylist is not None:
                    outcomes = pylist.get((rel, n))
                    if not outcomes or outcomes != {"passed"}:
                        got = ", ".join(sorted(outcomes)) if outcomes else "absent"
                        self.refusals.append((rel, 0, n, f"the test run did not record it as passed ({got})"))
                        continue
                self._add("python", None, rel, {rel}, n, n, False)

    def _scan(self, rel):
        if rel not in self._scans:
            try:
                self._scans[rel] = scan_rust(read(self.root, rel))
            except (OSError, UnicodeDecodeError):
                self._scans[rel] = None
        return self._scans[rel]

    def _reach(self, root_rel):
        """{file: {full test name: ignored}} for the files `mod` declarations reach
        from the crate root `root_rel`. A test's module path must equal its file's
        module path plus the inline modules around it; `#[path]` and never-true cfg
        declarations are not followed."""
        out = {}

        def visit(rel, modpath, child_dir):
            if rel in out:
                return
            info = self._scan(rel)
            if info is None:
                return
            out[rel] = {}
            if info["tainted"]:
                self.refusals.append((rel, 0, "", info["tainted"]))
                return  # nothing in this file or below it is credited
            for inline, name, ign in info["tests"]:
                out[rel]["::".join(modpath + inline + [name])] = ign
            for inline, name, attrs in info["mods"]:
                if _PATH_ATTR.search(attrs):
                    continue
                base = [p for p in child_dir + inline if p]
                for cand, kid_dir in ((base + [name + ".rs"], base + [name]), (base + [name, "mod.rs"], base + [name])):
                    path = "/".join(cand)
                    if os.path.isfile(os.path.join(self.root, path)):
                        visit(path, modpath + inline + [name], kid_dir)
                        break

        visit(root_rel, [], [p for p in os.path.dirname(root_rel).split("/") if p])
        return out

    @staticmethod
    def _fallback_package(rel):
        parts = rel.split("/")
        return parts[1] if len(parts) > 2 and parts[0] in ("crates", "firmware", "plugins", "fuzz") else parts[0]

    def _add(self, lang, package, src, files, name, leaf, ignored):
        self.entries.append({"lang": lang, "package": package, "src": src, "files": files, "name": name,
                             "leaf": leaf, "ignored": ignored})

    def resolve(self, ref):
        """Return (matching test names, problem or None) for `path/file.rs::pattern`,
        `crate::pattern` or a bare `pattern`. A pattern may carry a module path
        (`module::name`); a glob needs a literal prefix and a crate or file."""
        head, sep, pattern = ref.partition("::")  # `crate::module::name` is crate + a module path
        if not sep:
            head, pattern = None, ref
        elif not head:
            head = None
        if not pattern or pattern != pattern.strip() or re.search(r"\s", pattern):
            return [], f"'{ref}' is not a test name or glob"
        if head is not None and "/" in head and not head.endswith((".rs", ".py")):
            return [], f"'{ref}': name a crate or a file (`path/file.rs::name`), not a directory"
        if pattern[0] in GLOB_CHARS:
            return [], f"'{ref}': a glob must start with a literal prefix (a bare '*' proves nothing)"
        is_glob = any(ch in pattern for ch in GLOB_CHARS)
        if head is None and is_glob:
            return [], f"'{ref}': a glob must be qualified with a crate or file (`crate::{pattern}`)"

        def selected(e):
            if head is None:
                return True
            if head.endswith(".py"):
                return e["lang"] == "python" and head in e["files"]
            if head.endswith(".rs"):
                return e["lang"] == "rust" and head in e["files"]
            return e["lang"] == "rust" and e["package"] == head

        hits = [e for e in self.entries if selected(e) and (fnmatch.fnmatchcase(e["leaf"], pattern) or fnmatch.fnmatchcase(e["name"], pattern))]
        live = [e for e in hits if not e["ignored"]]
        ign = [e for e in hits if e["ignored"]]
        distinct = {(e["lang"], e["package"], e["src"], e["name"]) for e in hits}
        if not is_glob and len(distinct) > 1:
            return [], f"'{ref}' matches {len(distinct)} different tests; use a longer name (`module::name`) or qualify it with a crate or file"
        if is_glob and ign:
            return [], f"'{ref}': the glob also matches the ignored test '{ign[0]['name']}'"
        if live:
            return sorted({e["name"] for e in live}), None
        if ign:
            return [], f"'{ref}' only matches ignored tests"
        how = "cargo's list of runnable tests" if self.cargo is not None else "the source scan"
        if head is not None and head.endswith((".rs", ".py")):
            why = [r for r in self.refusals if r[0] == head]
            if not why and head.endswith(".rs"):  # a file below a refused one is never reached: name the refusal above it
                why = [r for r in self.refusals if r[0].endswith(".rs") and pkg_dir(self.root, r[0]) == pkg_dir(self.root, head)]
        elif head is not None:
            why = [r for r in self.refusals if r[0].endswith(".rs") and self._fallback_package(r[0]) == head]
        else:
            why = []
        note = ""
        if self.macro_hazard and not (head or "").endswith(".py"):
            note = f"; every Rust test is refused: {self.macro_hazard}"
        elif why:
            f, line, text, reason = why[0]
            note = f"; left out: {f}" + (f":{line}" if line else "") + (f" `{text}`" if text else "") + f" ({reason})"
        return [], f"no test matches '{ref}' in {how}{note}"


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


def load_py_list(path):
    """Parse the JSON made by ci/py_test_list.py into {(file, test name): {outcomes}}.
    Raises Problem if malformed."""
    try:
        with open(path, encoding="utf-8") as fh:
            data = json.load(fh)
    except (OSError, ValueError) as e:
        raise Problem(f"--py-tests {path} cannot be read: {e}")
    tests = data.get("tests") if isinstance(data, dict) else None
    ok = isinstance(tests, list) and all(
        isinstance(t, dict) and isinstance(t.get("file"), (str, type(None))) and isinstance(t.get("name"), (str, type(None)))
        and isinstance(t.get("outcome"), str) for t in tests)
    if not ok:
        raise Problem(f"--py-tests {path} is not a list of test outcomes from ci/py_test_list.py")
    out = {}
    for t in tests:
        if t["file"] and t["name"]:
            out.setdefault((t["file"], t["name"]), set()).add(t["outcome"])
    return out


# ---------------------------------------------------------------- loading


def load_toml(root, rel):
    try:
        return tomllib.loads(read(root, rel))
    except FileNotFoundError:
        raise Problem(f"{rel} is missing")
    except OSError as e:
        raise Problem(f"{rel} cannot be read: {e}")
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
    doc = None
    if repo_file(root, NAMES_DOC):
        try:
            doc = read(root, NAMES_DOC)
        except (OSError, UnicodeDecodeError):
            doc = None
    if doc is not None:
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
    if "hws" in rec:
        h = rec["hws"]
        if not (isinstance(h, list) and h and all(isinstance(x, str) and HWS_ID_RE.match(x) for x in h) and len(set(h)) == len(h)):
            errs.append(f"{where}: hws must be a non-empty list of distinct HWS ids such as \"HWS-TXT-1\"")
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


def load_hws_ids(root):
    """Every requirement id in standards/hws/*/requirements.toml (the Hardware
    Write Safety standard), so records can cite the clauses they implement."""
    ids = set()
    base = os.path.join(root, HWS_GLOB)
    for version in sorted(os.listdir(base)) if os.path.isdir(base) else []:
        rel = f"{HWS_GLOB}/{version}/requirements.toml"
        if os.path.isfile(os.path.join(root, rel)):
            try:
                data = load_toml(root, rel)
            except Problem:
                continue
            ids |= {r["id"] for r in data.get("requirement", []) if isinstance(r, dict) and isinstance(r.get("id"), str)}
    return ids


def run_check(root, cargo=None, pylist=None):
    """Return (rows, errors). rows are dicts ready for the table. `cargo` is the
    parsed --rust-tests list; None means the labelled pure-source fallback."""
    errors = []
    comps = load_names(root, errors)
    recs = load_records(root, comps, errors)
    index = TestIndex(root, cargo, pylist)
    tags = find_tags(root)
    known = {r["id"]: r for r in recs}
    rows = []
    hws_ids = load_hws_ids(root)
    for rec in recs:
        for h in rec.get("hws", []):
            if h not in hws_ids:
                errors.append(f"{rec['id']}: cites {h}, which is not in {HWS_GLOB}/*/requirements.toml. Fix the id in {rec['_file']} or add the clause to the standard")
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
    except (OSError, UnicodeDecodeError):
        current = None  # missing, a directory or unreadable: stale either way
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
    for flag in ("--rust-tests", "--py-tests", "--commits"):
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
    pylist = None
    if "--py-tests" in opts:
        try:
            pylist = load_py_list(opts["--py-tests"])
        except Problem as e:
            print(f"::error::{e}")
            return 1
    else:
        print("::warning::No --py-tests list: Python tests are counted from the static scan alone, which a test file "
              "that rewrites unittest at import can fool. CI passes the outcomes of a real run (ci/py_test_list.py).")
    rows, errors = run_check(root, cargo, pylist)
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
