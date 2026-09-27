"""Apply the glossary/overrides in ``terms.py`` to an existing ``auto.json``.

Kept separate from ``autofill.py`` so terminology can be tuned without
re-querying the translation endpoint.
"""

import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import terms  # noqa: E402

path = os.path.join(HERE, "auto.json")
auto = json.load(open(path, encoding="utf-8"))

for lang, table in auto.items():
    overrides = terms.OVERRIDES.get(lang, {})
    glossary = terms.GLOSSARY.get(lang, [])
    for key, value in list(table.items()):
        if key in terms.KEEP_AS_IS:
            table[key] = key
            continue
        for src, dst in glossary:
            value = value.replace(src, dst)
        table[key] = overrides.get(key, value)

with open(path, "w", encoding="utf-8") as fh:
    json.dump(auto, fh, ensure_ascii=False, indent=2, sort_keys=True)
    fh.write("\n")
print("post-processed", path)
