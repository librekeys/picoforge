import json
import os
import re

TR = re.compile(r'crate::tr!\("((?:[^"\\]|\\.)*)"')

keys = {}


def decode(s):
    out = []
    i = 0
    while i < len(s):
        c = s[i]
        if c == "\\" and i + 1 < len(s):
            e = s[i + 1]
            mapping = {"n": "\n", "t": "\t", "r": "\r", "0": "\0", "\\": "\\", '"': '"', "'": "'"}
            if e in mapping:
                out.append(mapping[e])
                i += 2
                continue
            if e == "x":
                out.append(chr(int(s[i + 2 : i + 4], 16)))
                i += 4
                continue
            if e == "u":
                close = s.find("}", i + 2)
                out.append(chr(int(s[i + 3 : close], 16)))
                i = close + 1
                continue
            out.append(e)
            i += 2
            continue
        out.append(c)
        i += 1
    return "".join(out)


for root, _, files in os.walk("src/ui"):
    for fn in files:
        if not fn.endswith(".rs") or fn == "i18n.rs":
            continue
        path = os.path.join(root, fn)
        src = open(path, encoding="utf-8").read()
        for m in TR.finditer(src):
            val = decode(m.group(1))
            keys[val] = keys.get(val, 0) + 1

with open("temp/i18n/keys.json", "w", encoding="utf-8") as fh:
    json.dump(sorted(keys), fh, ensure_ascii=False, indent=1)

print("unique keys:", len(keys))
# technical-looking keys (all ASCII, no spaces) that likely need no translation
tech = [k for k in keys if re.fullmatch(r"[A-Za-z0-9 .\-:/()]+", k) and " " not in k]
print("no-space keys:", len(tech))
