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
  python3 -I ci/conformance.py closure         closure gate for a PR (env: PR_TITLE, PR_BODY, REPO)

Exit status is non-zero on: a malformed record, an unknown or duplicate ID, a
record naming a test or file that does not exist, a drifted record, a stale
STATUS.md, or (closure) a closing keyword for an issue whose requirements are
not all built or waived.

Stdlib only (Python 3.11+). The closure gate reads PR text only to extract
issue numbers with a strict regex; it never runs or interpolates that text.
"""
import fnmatch
import hashlib
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


class Problem(Exception):
    pass


# ---------------------------------------------------------------- file helpers


def read(root, rel):
    with open(os.path.join(root, rel), encoding="utf-8") as fh:
        return fh.read()


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

_TOKEN = re.compile(r'//[^\n]*|/\*.*?\*/|r(#*)".*?"\1|"(?:\\.|[^"\\])*"|\'(?:\\.[^\']*|[^\\\'])\'', re.DOTALL)
_FN = re.compile(r"((?:#\[[^\]]*\]\s*)*)(?:pub(?:\([^)]*\))?\s+)?(?:async\s+|const\s+|unsafe\s+)*fn\s+(\w+)")
_TEST_ATTR = re.compile(r"#\[\s*(?:\w+::)*test\b")
_IGNORE_ATTR = re.compile(r"#\[\s*ignore\b")
_PY_DEF = re.compile(r"^\s*(?:async\s+)?def\s+(test_\w+)\s*\(", re.MULTILINE)


def _blank(m):
    s = m.group(0)
    if s.startswith("/"):
        return re.sub(r"[^\n]", " ", s)
    return re.sub(r"[^\n]", "_", s)  # string contents never count as code


def rust_tests(src):
    """Names of the live `#[test]` fns in `src` (comments and strings ignored)
    and the names of those marked `#[ignore]`."""
    skeleton = _TOKEN.sub(_blank, src)
    live, ignored = set(), set()
    for m in _FN.finditer(skeleton):
        # Attribute text with strings blanked is fine for test/ignore detection.
        attrs = m.group(1)
        if not _TEST_ATTR.search(attrs):
            continue
        (ignored if _IGNORE_ATTR.search(attrs) else live).add(m.group(2))
    return live, ignored


def python_tests(src):
    return set(_PY_DEF.findall(src))


class TestIndex:
    """Which test names exist where. Built once per run."""

    def __init__(self, root):
        self.root = root
        self.by_file = {}      # rel path -> set of live test names
        self.ignored = {}      # rel path -> set of ignored test names
        for rel in walk(root, ("crates", "firmware", "plugins", "ci", "fuzz"), (".rs", ".py")):
            try:
                src = read(root, rel)
            except (OSError, UnicodeDecodeError):
                continue
            if rel.endswith(".rs"):
                live, ign = rust_tests(src)
            else:
                live, ign = python_tests(src), set()
            if live:
                self.by_file[rel] = live
            if ign:
                self.ignored[rel] = ign

    def scope(self, head):
        """Files a ref's path part selects: a file, a crate name, or None for all."""
        if head is None:
            return list(self.by_file)
        if head.endswith((".rs", ".py")) or "/" in head:
            prefix = head.rstrip("/")
            return [f for f in self.by_file if f == prefix or f.startswith(prefix + "/")]
        crate = f"crates/{head}/"
        return [f for f in self.by_file if f.startswith(crate)]

    def resolve(self, ref):
        """Return (matching names, problem or None) for `path::pattern`,
        `crate::pattern` or a bare `pattern` (a name or an fnmatch glob)."""
        head, sep, pattern = ref.rpartition("::")
        head = head or None
        if not pattern.strip() or pattern != pattern.strip() or " " in pattern:
            return [], f"'{ref}' is not a test name or glob"
        files = self.scope(head)
        found = sorted({n for f in files for n in self.by_file[f] if fnmatch.fnmatchcase(n, pattern)})
        if found:
            return found, None
        ign = sorted({n for f in (self.scope(head) or []) for n in self.ignored.get(f, ()) if fnmatch.fnmatchcase(n, pattern)})
        if not ign:
            for f, names in self.ignored.items():
                if head is None or f in self.scope(head) or f.startswith(f"crates/{head}/"):
                    ign += [n for n in names if fnmatch.fnmatchcase(n, pattern)]
        if ign:
            return [], f"'{ref}' only matches ignored tests"
        return [], f"no test matches '{ref}'"


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
    for c in data.get("component", []):
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
    if "level" in rec and rec["level"] not in LEVELS:
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
        if not (isinstance(w, dict) and set(w) <= {"reason", "ref"} and isinstance(w.get("reason"), str) and w["reason"].strip()):
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
        seen.add(a["id"])
        if a["kind"] not in KINDS:
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
    files = sorted(f for f in os.listdir(d) if f.endswith(".toml")) if os.path.isdir(d) else []
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
            shape = check_shape(rec, where, comp)
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
        src = read(root, rec["source"])
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
            path = a["ref"]
            ok = not os.path.isabs(path) and ".." not in path.split("/") and os.path.exists(os.path.join(root, path))
            problem = None if ok else f"file '{path}' does not exist"
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


def run_check(root):
    """Return (rows, errors). rows are dicts ready for the table."""
    errors = []
    comps = load_names(root, errors)
    recs = load_records(root, comps, errors)
    index = TestIndex(root)
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


def with_table(doc, table):
    a, b = doc.find(MARK_START), doc.find(MARK_END)
    if a == -1 or b == -1 or b < a:
        raise Problem(f"{STATUS_FILE} is missing its status:start / status:end marks")
    return doc[:a + len(MARK_START)] + "\n\n" + table + "\n\n" + doc[b:]


STATUS_HEAD = """# Design conformance status (the X-ray)

Generated by `ci/conformance.py` from the records in `design/requirements/`. Status is computed from the code, never typed in: *built* means every acceptance check resolves to a test or file that exists, *partial* means some do and the rest wait on a tracking issue, *not built* means none do, *drifted* means the design paragraph changed without its record, *waived* means a recorded waiver. How it works is in [CONTRIBUTING.md](../CONTRIBUTING.md#design-conformance-the-x-ray); component names are in [docs/NAMES.md](../docs/NAMES.md).

"""
STATUS_TAIL = """
A test fails the build if this table is stale. Regenerate it with `python3 ci/conformance.py --bless`.
"""


def status_doc(table):
    return STATUS_HEAD + MARK_START + "\n\n" + table + "\n\n" + MARK_END + "\n" + STATUS_TAIL


def check_status_file(root, rows, bless):
    table = render_table(rows)
    path = os.path.join(root, STATUS_FILE)
    try:
        current = read(root, STATUS_FILE)
        want = with_table(current, table)
    except FileNotFoundError:
        current, want = None, status_doc(table)
    except Problem as e:
        return [str(e)]
    if bless:
        os.makedirs(os.path.dirname(path), exist_ok=True)
        with open(path, "w", encoding="utf-8") as fh:
            fh.write(want)
        return []
    if current != want:
        return [f"{STATUS_FILE} is stale: run `python3 ci/conformance.py --bless`"]
    return []


# ------------------------------------------------------------------ closure


def referenced_issues(text, repo):
    """Issue numbers of `repo` closed by `text` (same strict parse as
    regression_guard.py: a closing keyword, then #N, repo#N or an issue URL)."""
    repo = repo.lower()
    found = set()
    ref = r"(?:(?P<repo>[\w.-]+/[\w.-]+))?#(?P<n>\d+)\b"
    url = r"https://github\.com/(?P<urepo>[\w.-]+/[\w.-]+)/issues/(?P<un>\d+)\b"
    pattern = re.compile(rf"(?<![\w-]){CLOSING}\b:?\s+(?:{ref}|{url})", re.IGNORECASE)
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

    rows, errors = run_check(root)
    if cmd == "check":
        errors += check_status_file(root, rows, bless)
    else:
        repo = os.environ.get("REPO") or "rafe-walker/sawbones-labs"
        closed = referenced_issues(os.environ.get("PR_TITLE", "") + "\n" + os.environ.get("PR_BODY", ""), repo)
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
