"""Two fixes after the bulk tr! rewrite:

1. Revert `crate::tr!("...")` embedded in `const OPT_*` option tables back to
   plain literals (const items cannot call the runtime `translate`). The labels
   are translated centrally in `components::form::select_state`.
2. Turn local `const TITLE/SUBTITLE: &str = crate::tr!(...)` into `let`.
"""

import os
import re

FILES = [
    "src/ui/screens/accounts/view_model.rs",
    "src/ui/screens/openpgp/view_model.rs",
    "src/ui/screens/piv/view_model.rs",
    "src/ui/screens/slots/program_form.rs",
]

TR_RE = re.compile(r'crate::tr!\("((?:[^"\\]|\\.)*)"\)')


def revert_const_blocks(src):
    out = []
    i = 0
    while True:
        m = re.search(r"const OPT_\w+[^=]*?=\s*&\[", src[i:])
        if not m:
            out.append(src[i:])
            break
        start = i + m.start()
        arr_open = i + m.end() - 1  # points at '['
        # find matching ']' respecting strings
        depth = 0
        j = arr_open
        while j < len(src):
            c = src[j]
            if c == '"':
                j += 1
                while j < len(src) and src[j] != '"':
                    if src[j] == "\\":
                        j += 1
                    j += 1
            elif c == "[":
                depth += 1
            elif c == "]":
                depth -= 1
                if depth == 0:
                    break
            j += 1
        # find terminating ';'
        semi = src.find(";", j)
        end = semi + 1 if semi >= 0 else j + 1
        out.append(src[i:start])
        block = src[start:end]
        out.append(TR_RE.sub(r'"\1"', block))
        i = end
    return "".join(out)


def main():
    for path in FILES:
        src = open(path, encoding="utf-8", newline="").read()
        new = revert_const_blocks(src)
        open(path, "w", encoding="utf-8", newline="").write(new)
        print(path, "OPT blocks reverted")

    for root, _, files in os.walk("src/ui"):
        for fn in files:
            if not fn.endswith(".rs"):
                continue
            path = os.path.join(root, fn)
            src = open(path, encoding="utf-8", newline="").read()
            new = src.replace("const TITLE: &str =", "let TITLE: &str =")
            new = new.replace("const SUBTITLE: &str =", "let SUBTITLE: &str =")
            if new != src:
                open(path, "w", encoding="utf-8", newline="").write(new)
                print(path, "TITLE/SUBTITLE -> let")


if __name__ == "__main__":
    main()
