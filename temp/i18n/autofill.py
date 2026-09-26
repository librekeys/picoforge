"""Machine-fill the remaining catalog entries.

Reads the unique source keys produced by ``collect_keys.py``, subtracts the
keys already hand-translated in ``tr_*.py`` and a small set of *functional*
keys that must stay byte-identical (env vars, FIDO option names, PEM markers,
error codes), then queries the public Google translate endpoint for each
missing key in every target language.

``format!`` placeholders are masked before the request and restored after, so
Google cannot rewrite ``{slot:02x}`` into ``{槽：02x}``.

Results are written to ``auto.json`` (resume-friendly) and merged by
``gen_locales.py``.
"""

import concurrent.futures as futures
import importlib
import json
import os
import re
import sys
import time
import urllib.parse
import urllib.request

import terms

HERE = os.path.dirname(os.path.abspath(__file__))
LANGS = ["zh-CN", "fr", "es", "ru", "ar"]

# Keys whose tr! result is consumed by code (map lookup, env var, PEM parsing)
# and therefore must not be localised.
EXCLUDE = {
    "HOME",
    "rk",
    "ep",
    "otpauth://",
    "0x27",
    "0x3E",
    "-----BEGIN CERTIFICATE-----",
    "-----BEGIN CERTIFICATE-----\n",
    "-----END",
    "-----END CERTIFICATE-----\n",
}

PROXY = os.environ.get("I18N_PROXY", "http://192.168.1.127:1080")
OPENER = urllib.request.build_opener(
    urllib.request.ProxyHandler({"http": PROXY, "https": PROXY})
)

PLACEHOLDER = re.compile(r"\{[^{}]*\}")


def mask(text):
    """Hide format placeholders and protected terms behind @@n@@ tokens."""
    parts = []

    def token(value):
        parts.append(value)
        return f"@@{len(parts) - 1}@@"

    text = PLACEHOLDER.sub(lambda m: token(m.group(0)), text)
    for term in terms.PROTECT:
        text = text.replace(term, token(term))
    return text, parts


def unmask(text, parts):
    for i, original in enumerate(parts):
        text = text.replace(f"@@{i}@@", original)
    return text


def translate(text, lang, attempts=4):
    masked, parts = mask(text)
    query = urllib.parse.urlencode(
        {"client": "dict-chrome-ex", "sl": "en", "tl": lang, "dt": "t", "q": masked}
    )
    url = "https://clients5.google.com/translate_a/single?" + query
    last = None
    for attempt in range(attempts):
        try:
            with OPENER.open(url, timeout=25) as resp:
                data = json.load(resp)
            joined = "".join(seg[0] for seg in data[0] if seg and seg[0])
            if joined.strip():
                result = unmask(joined, parts)
                if text in terms.KEEP_AS_IS:
                    return text
                for src, dst in terms.GLOSSARY.get(lang, []):
                    result = result.replace(src, dst)
                return terms.OVERRIDES.get(lang, {}).get(text, result)
            last = "empty"
        except Exception as exc:  # noqa: BLE001 - network flakiness
            last = repr(exc)
        time.sleep(1.5 * (attempt + 1))
    raise RuntimeError(f"{lang}: {text!r} -> {last}")


def existing_keys():
    keys = set()
    for module in ["tr_zh", "tr_fr", "tr_es", "tr_ru", "tr_ar"]:
        sys.path.insert(0, HERE)
        keys |= set(importlib.import_module(module).T)
    return keys


def main():
    source_keys = json.load(open(os.path.join(HERE, "keys.json"), encoding="utf-8"))
    todo = [k for k in source_keys if k not in existing_keys() and k not in EXCLUDE]

    path = os.path.join(HERE, "auto.json")
    auto = json.load(open(path, encoding="utf-8")) if os.path.exists(path) else {}

    jobs = []
    for lang in LANGS:
        table = auto.setdefault(lang, {})
        for key in todo:
            if key not in table:
                jobs.append((lang, key))

    print(f"source keys: {len(source_keys)}; missing: {len(todo)}; jobs: {len(jobs)}")

    def work(job):
        lang, key = job
        return lang, key, translate(key, lang)

    done = 0
    with futures.ThreadPoolExecutor(max_workers=8) as pool:
        for lang, key, value in pool.map(work, jobs):
            auto[lang][key] = value
            done += 1
            if done % 50 == 0:
                print(f"  {done}/{len(jobs)}")
                with open(path, "w", encoding="utf-8") as fh:
                    json.dump(auto, fh, ensure_ascii=False, indent=2, sort_keys=True)
                    fh.write("\n")

    with open(path, "w", encoding="utf-8") as fh:
        json.dump(auto, fh, ensure_ascii=False, indent=2, sort_keys=True)
        fh.write("\n")
    print(f"wrote {path}")


if __name__ == "__main__":
    main()
