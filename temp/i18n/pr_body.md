## Summary

Adds runtime internationalisation and ships catalogs for the six official languages of the United Nations: **English, 简体中文, Français, Español, Русский, العربية**.

## What changed

- **New `src/ui/i18n.rs`**
  - `Locale` enum (6 UN languages), `from_code`, endonyms, flags, RTL flag.
  - English-source-text-as-key catalogs embedded from `locales/*.json`; missing keys fall back to English so coverage can grow incrementally.
  - `tr!` macro with `format!`-style placeholders (`{}`, `{name}`, `{name:spec}`), e.g. `tr!("Error: {e}", e = err)`.
  - `translate()` returns `&'static str` (cached) so it drops into any `&str`/`SharedString`/element slot.
  - Locale persistence under `ProjectDirs` and startup detection from the environment.
- **All user-facing UI strings** across `src/ui` wrapped in `crate::tr!` (~1,260 call sites); `format!`/`log::*!`/`panic!` call sites are handled so placeholders still substitute.
- **`locales/*.json`** for all six languages (306 entries each: navigation, page chrome, buttons, common messages).
- **Sidebar language picker**: click-to-select dropdown with flags + endonyms and a checkmark on the active language.
- `temp/i18n/` tooling and a README *Localisation* section.

## Verification

- `cargo check`: 0 errors / 0 warnings
- `cargo test`: 136 passed (incl. 4 new i18n unit tests)
- `cargo fmt --check`: clean
- Manually launched and verified the Chinese and Arabic UIs render, and that the language dropdown opens and selects.

## Notes / follow-ups

- ~590 longer help/description strings still fall back to English; they can be added to `locales/<code>.json` incrementally.
- GPUI 0.2 has no RTL layout direction, so Arabic text renders correctly but is still laid out LTR (exposed via `Locale::is_rtl()`).
