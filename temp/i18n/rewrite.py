"""Wrap user-facing English string literals in `crate::tr!(...)`.

Run from the repository root:

    python temp/i18n/rewrite.py            # dry run, prints statistics
    python temp/i18n/rewrite.py --write    # apply the rewrite in place

The script understands Rust string / raw-string / byte-string literals, skips
comments, converts `format!("...", args)` calls into `tr!("...", args)` (adding
inferred `name = name` arguments for inline captures) and leaves identifier-like
and path-like literals untouched.
"""

import os
import re
import sys

SKIP = {os.path.join("src", "ui", "i18n.rs")}
ROOTS = ["src/ui"]

ID_RE = re.compile(r"^[a-z0-9_-]+$")
HEX_RE = re.compile(r"^[0-9a-fA-F]+$")

# Literals we never translate even though they pass the generic filter.
DENY = {
    "PicoForge",
    "OFFBOARD",
    "Mono",
    "Sans",
    "clientPin",
    "AAGUID",
    "Q",
}


def translatable(value: str, is_byte: bool) -> bool:
    if is_byte or len(value) < 2:
        return False
    if value in DENY:
        return False
    if value.startswith("icons/") or value.startswith("appIcons/"):
        return False
    if value.startswith("http"):
        return False
    if ID_RE.match(value) and ("-" in value or "_" in value):
        return False
    if HEX_RE.match(value):
        return False
    if value.strip() == "":
        return False
    return True


def find_literals(src):
    """Return list of (start, end, value, is_byte, kind) for every literal."""
    lits = []
    comments = []
    i, n = 0, len(src)
    while i < n:
        c = src[i]
        # comments
        if c == "/" and i + 1 < n and src[i + 1] == "/":
            j = src.find("\n", i)
            j = n if j < 0 else j
            comments.append((i, j))
            i = j
            continue
        if c == "/" and i + 1 < n and src[i + 1] == "*":
            j = src.find("*/", i + 2)
            j = n if j < 0 else j + 2
            comments.append((i, j))
            i = j
            continue
        start = i
        j = i
        is_byte = False
        if src[j] == "b" and j + 1 < n and src[j + 1] in '"r':
            is_byte = True
            j += 1
        if j < n and src[j] == "r":
            k = j + 1
            hashes = 0
            while k < n and src[k] == "#":
                hashes += 1
                k += 1
            if k < n and src[k] == '"':
                term = '"' + "#" * hashes
                end = src.find(term, k + 1)
                end = n if end < 0 else end
                value = src[k + 1 : end]
                total = end + len(term)
                lits.append((start, total, value, is_byte, "raw"))
                i = total
                continue
        if j < n and src[j] == '"':
            k = j + 1
            buf = []
            closed = False
            while k < n:
                ch = src[k]
                if ch == "\\":
                    if k + 1 < n:
                        e = src[k + 1]
                        simple = {
                            "n": "\n",
                            "t": "\t",
                            "r": "\r",
                            "0": "\0",
                            "\\": "\\",
                            '"': '"',
                            "'": "'",
                        }
                        if e in simple:
                            buf.append(simple[e])
                            k += 2
                            continue
                        if e == "x" and k + 3 < n:
                            try:
                                buf.append(chr(int(src[k + 2 : k + 4], 16)))
                            except ValueError:
                                pass
                            k += 4
                            continue
                        if e == "u" and k + 2 < n and src[k + 2] == "{":
                            close = src.find("}", k + 3)
                            try:
                                buf.append(chr(int(src[k + 3 : close], 16)))
                            except ValueError:
                                pass
                            k = close + 1
                            continue
                        if e == "\n":
                            k += 2
                            while k < n and src[k] in " \t":
                                k += 1
                            continue
                        if e == "\r" and k + 2 < n and src[k + 2] == "\n":
                            k += 3
                            while k < n and src[k] in " \t":
                                k += 1
                            continue
                        buf.append(e)
                        k += 2
                        continue
                    k += 1
                    continue
                if ch == '"':
                    lits.append((start, k + 1, "".join(buf), is_byte, "normal"))
                    i = k + 1
                    closed = True
                    break
                buf.append(ch)
                k += 1
            if closed:
                continue
        i += 1
    return lits, comments


def masked_array(n, lits, comments):
    mask = bytearray(n)
    for start, end, *_ in lits:
        for x in range(start, end):
            mask[x] = 1
    for start, end in comments:
        for x in range(start, end):
            mask[x] = 1
    return mask


def find_matching_paren(src, mask, open_idx):
    depth = 0
    i = open_idx
    n = len(src)
    while i < n:
        if not mask[i]:
            c = src[i]
            if c == "(":
                depth += 1
            elif c == ")":
                depth -= 1
                if depth == 0:
                    return i
        i += 1
    return -1


PLACEHOLDER = re.compile(r"\{([^{}]*)\}")
MACRO_RE = re.compile(r"([A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z_][A-Za-z0-9_]*)*)!\s*\(")

# Macros whose first argument is a message literal. `format` is replaced
# outright by `tr!`; the others keep their identity and receive the translated
# string through a `"{}"` format slot.
LOG_MACROS = {
    "panic",
    "println",
    "print",
    "eprintln",
    "eprint",
    "log::error",
    "log::warn",
    "log::info",
    "log::debug",
    "log::trace",
}


def parse_placeholders(value):
    """Return (positional_count, [named names]) ignoring escaped braces."""
    names = []
    positional = 0
    i = 0
    while i < len(value):
        if value[i] == "{" and i + 1 < len(value) and value[i + 1] == "{":
            i += 2
            continue
        if value[i] == "}" and i + 1 < len(value) and value[i + 1] == "}":
            i += 2
            continue
        if value[i] == "{":
            close = value.find("}", i + 1)
            if close < 0:
                break
            inner = value[i + 1 : close]
            name = inner.split(":", 1)[0]
            if name == "":
                positional += 1
            else:
                names.append(name)
            i = close + 1
            continue
        i += 1
    return positional, names


def bracket_range(src, mask, open_idx, open_ch, close_ch):
    depth = 0
    i = open_idx
    n = len(src)
    while i < n:
        if not mask[i]:
            c = src[i]
            if c == open_ch:
                depth += 1
            elif c == close_ch:
                depth -= 1
                if depth == 0:
                    return i
        i += 1
    return -1


def protected_ranges(src, mask, lits):
    """Literals that must stay literals: inside `#[...]` attrs and `cfg!(...)`."""
    starts = set()
    for m in re.finditer(r"#\[", src):
        i = m.start()
        if mask[i]:
            continue
        end = bracket_range(src, mask, i + 1, "[", "]")
        if end < 0:
            continue
        for lit in lits:
            if i < lit[0] < end:
                starts.add(lit[0])
    for m in re.finditer(r"\bcfg!\s*\(", src):
        i = m.start()
        if mask[i]:
            continue
        open_idx = src.find("(", i)
        end = bracket_range(src, mask, open_idx, "(", ")")
        if end < 0:
            continue
        for lit in lits:
            if i < lit[0] < end:
                starts.add(lit[0])
    return starts


def process(src):
    lits, comments = find_literals(src)
    mask = masked_array(len(src), lits, comments)
    lit_by_start = {s: (s, e, v, b, k) for (s, e, v, b, k) in lits}

    protected = protected_ranges(src, mask, lits)

    edits = []  # (start, end, replacement)
    handled = set()
    skipped_macro_first = set()

    # --- Pass A: rewrite macro message literals ---
    for m in MACRO_RE.finditer(src):
        fs = m.start()
        name = m.group(1)
        if mask[fs] or name not in (LOG_MACROS | {"format"}):
            continue
        open_idx = src.find("(", fs)
        if open_idx < 0:
            continue
        # first literal must come right after '(' (whitespace only)
        first = None
        for lit in lits:
            if lit[0] <= open_idx:
                continue
            if src[open_idx + 1 : lit[0]].strip() != "":
                break
            first = lit
            break
        if first is None:
            continue
        skipped_macro_first.add(first[0])
        value, is_byte = first[2], first[3]
        if is_byte or not translatable(value, is_byte):
            continue
        close = find_matching_paren(src, mask, open_idx)
        if close < 0:
            continue
        _positional, names = parse_placeholders(value)
        # infer inline-capture args that are not already provided
        arg_region = src[first[1] : close]
        extra = []
        for ph in names:
            if not re.search(r"(?<![\w.])" + re.escape(ph) + r"\s*=", arg_region):
                extra.append(ph)
        insertion = "".join(", {} = {}".format(n, n) for n in extra)

        if name == "format":
            edits.append((fs, fs + len("format!"), "crate::tr!"))
            if insertion:
                edits.append((close, close, insertion))
        else:
            # keep the logging/printing macro; pass the translation through `{}`
            edits.append((first[0], first[1], '"{}", crate::tr!(' + src[first[0] : first[1]]))
            edits.append((close, close, insertion + ")"))
        handled.add(first[0])

    # --- Pass B: wrap remaining standalone literals ---
    for lit in lits:
        start, end, value, is_byte, kind = lit
        if start in handled or start in skipped_macro_first or start in protected:
            continue
        if not translatable(value, is_byte):
            continue
        edits.append((start, end, "crate::tr!(" + src[start:end] + ")"))

    edits.sort(key=lambda e: e[0])
    # verify no overlap
    out = []
    last = 0
    for start, end, rep in edits:
        if start < last:
            raise SystemExit("overlapping edit at {}".format(start))
        out.append(src[last:start])
        out.append(rep)
        last = end
    out.append(src[last:])
    return "".join(out)


def main():
    write = "--write" in sys.argv
    total = 0
    wrapped = 0
    per_file = {}
    for root in ROOTS:
        for dirpath, _, files in os.walk(root):
            for fn in files:
                if not fn.endswith(".rs"):
                    continue
                path = os.path.join(dirpath, fn)
                if path.replace("/", os.sep) in SKIP or path in SKIP:
                    continue
                src = open(path, encoding="utf-8", newline="").read()
                new = process(src)
                n = new.count("crate::tr!(") - src.count("crate::tr!(")
                if n:
                    per_file[path] = n
                    total += n
                if write and new != src:
                    open(path, "w", encoding="utf-8", newline="").write(new)
    for path in sorted(per_file):
        print("{:4d}  {}".format(per_file[path], path))
    print("total new tr! call sites:", total, "(write={})".format(write))


if __name__ == "__main__":
    main()
