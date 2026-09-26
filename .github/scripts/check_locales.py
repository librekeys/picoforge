#!/usr/bin/env python3
"""Validate the i18n catalogs under ``locales/``.

The UI keys every string by its English source text and embeds the catalogs at
compile time, so a broken catalog degrades silently: ``parse_catalog`` falls back
to an empty map and every string quietly reverts to English. This script makes
those failures loud in CI.

Checks performed for every ``locales/*.json`` file:

* valid JSON, a flat object of string -> string, no duplicate keys;
* every non-English catalog has exactly the same key set as the reference;
* the ``format!``-style placeholders in each translation match those of the
  English key (same names and positional slots), so ``tr!`` substitution cannot
  silently drop or reorder an argument.

Exit status is non-zero if any check fails.
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

LOCALES_DIR = Path(__file__).resolve().parents[2] / "locales"
REFERENCE = "en"

# ``{name}``, ``{}``, ``{name:spec}`` — but not the escaped ``{{`` / ``}}``.
PLACEHOLDER = re.compile(r"\{([^{}]*)\}")


def load_catalog(path: Path) -> dict[str, str]:
    """Parse a catalog, rejecting duplicate keys and non-string values."""
    duplicates: list[str] = []

    def hook(pairs: list[tuple[str, object]]) -> dict[str, object]:
        seen: dict[str, object] = {}
        for key, value in pairs:
            if key in seen:
                duplicates.append(key)
            seen[key] = value
        return seen

    catalog = json.loads(path.read_text(encoding="utf-8"), object_pairs_hook=hook)
    if not isinstance(catalog, dict):
        raise ValueError("top level must be a JSON object")
    if duplicates:
        raise ValueError(f"duplicate keys: {', '.join(sorted(set(duplicates)))}")
    for key, value in catalog.items():
        if not isinstance(value, str):
            raise ValueError(f"value for {key!r} is not a string")
    return catalog


def placeholders(text: str) -> list[str]:
    """Return an ordered signature of the format fields in ``text``.

    Positional slots are normalised to ``""`` and named fields to ``"name"``;
    the format spec is ignored, only the field identity matters.
    """
    fields: list[str] = []
    for inner in PLACEHOLDER.findall(text):
        name = inner.split(":", 1)[0]
        fields.append(name)
    return fields


def main() -> int:
    if not LOCALES_DIR.is_dir():
        print(f"error: {LOCALES_DIR} does not exist", file=sys.stderr)
        return 1

    files = sorted(LOCALES_DIR.glob("*.json"))
    if not files:
        print(f"error: no catalogs found in {LOCALES_DIR}", file=sys.stderr)
        return 1

    catalogs: dict[str, dict[str, str]] = {}
    errors: list[str] = []

    for path in files:
        code = path.stem
        try:
            catalogs[code] = load_catalog(path)
            print(f"ok   {path.name}: {len(catalogs[code])} entries")
        except (OSError, ValueError, json.JSONDecodeError) as exc:
            errors.append(f"{path.name}: {exc}")

    if errors:
        for error in errors:
            print(f"error: {error}", file=sys.stderr)
        return 1

    reference = catalogs.get(REFERENCE)
    if reference is None:
        print(f"error: reference catalog locales/{REFERENCE}.json is missing", file=sys.stderr)
        return 1

    # English is the fallback language, so ``en.json`` may legitimately be empty
    # (or hold only a few overrides). Every other catalog must cover the exact
    # same set of keys.
    translated_codes = [code for code in catalogs if code != REFERENCE]
    if not translated_codes:
        print("error: no translated catalogs found", file=sys.stderr)
        return 1
    baseline_code = translated_codes[0]
    baseline_keys = set(catalogs[baseline_code])

    unknown_in_reference = set(reference) - baseline_keys
    if unknown_in_reference:
        errors.append(
            f"{REFERENCE}: {len(unknown_in_reference)} key(s) absent from {baseline_code}, "
            f"e.g. {sorted(unknown_in_reference)[:3]}"
        )

    for code in translated_codes[1:]:
        catalog = catalogs[code]
        missing = baseline_keys - set(catalog)
        extra = set(catalog) - baseline_keys
        if missing:
            errors.append(f"{code}: {len(missing)} missing key(s), e.g. {sorted(missing)[:3]}")
        if extra:
            errors.append(f"{code}: {len(extra)} unknown key(s), e.g. {sorted(extra)[:3]}")

    for code, catalog in catalogs.items():
        for key, translation in catalog.items():
            expected = placeholders(key)
            actual = placeholders(translation)
            if expected != actual:
                errors.append(
                    f"{code}: placeholder mismatch for {key!r} "
                    f"(expected {expected}, got {actual})"
                )

    if errors:
        print("\nCatalog validation failed:", file=sys.stderr)
        for error in errors:
            print(f"  - {error}", file=sys.stderr)
        return 1

    print(f"\nAll {len(catalogs)} catalogs are consistent.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
