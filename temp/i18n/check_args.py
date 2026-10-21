import os
import re

# Find crate::tr!("...") invocations whose literal has a {...} placeholder but
# which appear to have no argument (no comma before the closing paren).
pat = re.compile(r'crate::tr!\("((?:[^"\\]|\\.)*)"')
suspects = []
for root, _, files in os.walk("src/ui"):
    for fn in files:
        if not fn.endswith(".rs") or fn == "i18n.rs":
            continue
        path = os.path.join(root, fn)
        src = open(path, encoding="utf-8").read()
        for m in pat.finditer(src):
            lit = m.group(1)
            if "{" not in lit:
                continue
            # look at what follows the literal
            tail = src[m.end() : m.end() + 1]
            # skip whitespace/newlines to next non-space
            j = m.end()
            while j < len(src) and src[j] in " \t\r\n":
                j += 1
            if src[j : j + 1] != ",":
                suspects.append((path, lit, src[max(0, m.start() - 30) : j + 10].replace("\n", " ")))
for s in suspects:
    print(s[0])
    print("   lit:", repr(s[1]))
print("suspects:", len(suspects))
