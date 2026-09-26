"""Generate `locales/<code>.json` from the `tr_*.py` dictionaries and validate
that every translated key actually exists in the source (typos would silently
fall back to English)."""

import importlib
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, "..", ".."))
sys.path.insert(0, HERE)

LANGS = {
    "zh-CN": "tr_zh",
    "fr": "tr_fr",
    "es": "tr_es",
    "ru": "tr_ru",
    "ar": "tr_ar",
}

keys = set(json.load(open(os.path.join(HERE, "keys.json"), encoding="utf-8")))
os.makedirs(os.path.join(ROOT, "locales"), exist_ok=True)

problems = 0
for code, module_name in LANGS.items():
    mod = importlib.import_module(module_name)
    table = mod.T
    unknown = sorted(k for k in table if k not in keys)
    if unknown:
        problems += len(unknown)
        print(f"[{code}] {len(unknown)} key(s) not found in source:")
        for k in unknown:
            print("   -", repr(k))
    out = os.path.join(ROOT, "locales", f"{code}.json")
    with open(out, "w", encoding="utf-8") as fh:
        json.dump(dict(sorted(table.items())), fh, ensure_ascii=False, indent=2)
        fh.write("\n")
    print(f"[{code}] wrote {len(table)} entries -> {out}")

# English is the fallback (key == value), so its catalog can stay empty.
with open(os.path.join(ROOT, "locales", "en.json"), "w", encoding="utf-8") as fh:
    fh.write("{}\n")

print("problems:", problems)
