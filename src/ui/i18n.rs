//! Runtime internationalisation (i18n) for the PicoForge UI.
//!
//! # Design
//!
//! Every user-facing string is *keyed by its English source text*. A catalog
//! is a flat `JSON` object mapping that English text to a translation, so a
//! missing entry transparently falls back to English. This keeps the source
//! readable (no synthetic keys to invent or look up) and lets localisation
//! roll out incrementally.
//!
//! Locales live in `locales/<code>.json` and are embedded at compile time via
//! [`include_str!`]. The six catalogs correspond to the six official languages
//! of the United Nations: English, Chinese, French, Spanish, Russian and Arabic.
//!
//! # Usage
//!
//! ```ignore
//! crate::tr!("Device Information")                       // plain lookup
//! crate::tr!("Error: {e}", e = err)                      // named placeholder
//! crate::tr!("{} credentials stored", creds_len)         // positional placeholder
//! ```
//!
//! The [`tr!`](crate::tr) macro returns an owned [`String`], which converts into
//! GPUI's `SharedString`/element types wherever a label is expected.
//!
//! Placeholders follow Rust's `format!` syntax (`{}`, `{name}`, `{name:spec}`),
//! so existing format strings keep working once wrapped in `tr!`.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

use directories::ProjectDirs;

/// A human language supported by the UI.
///
/// The set deliberately matches the six official languages of the United
/// Nations.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Locale {
    /// English (`en`).
    English,
    /// Simplified Chinese (`zh-CN`).
    Chinese,
    /// French (`fr`).
    French,
    /// Spanish (`es`).
    Spanish,
    /// Russian (`ru`).
    Russian,
    /// Arabic (`ar`).
    Arabic,
}

impl Locale {
    /// Every supported locale, in the order shown in the language picker.
    pub const ALL: [Locale; 6] = [
        Locale::English,
        Locale::Chinese,
        Locale::French,
        Locale::Spanish,
        Locale::Russian,
        Locale::Arabic,
    ];

    /// The stable code persisted to disk and used for catalog file names.
    pub fn code(self) -> &'static str {
        match self {
            Locale::English => "en",
            Locale::Chinese => "zh-CN",
            Locale::French => "fr",
            Locale::Spanish => "es",
            Locale::Russian => "ru",
            Locale::Arabic => "ar",
        }
    }

    /// The English name of the language.
    pub fn english_name(self) -> &'static str {
        match self {
            Locale::English => "English",
            Locale::Chinese => "Chinese",
            Locale::French => "French",
            Locale::Spanish => "Spanish",
            Locale::Russian => "Russian",
            Locale::Arabic => "Arabic",
        }
    }

    /// The language's endonym — its name written in the language itself.
    pub fn native_name(self) -> &'static str {
        match self {
            Locale::English => "English",
            Locale::Chinese => "简体中文",
            Locale::French => "Français",
            Locale::Spanish => "Español",
            Locale::Russian => "Русский",
            Locale::Arabic => "العربية",
        }
    }

    /// The language's flag emoji, used as a compact picker glyph.
    pub fn flag(self) -> &'static str {
        match self {
            Locale::English => "🇬🇧",
            Locale::Chinese => "🇨🇳",
            Locale::French => "🇫🇷",
            Locale::Spanish => "🇪🇸",
            Locale::Russian => "🇷🇺",
            Locale::Arabic => "🇸🇦",
        }
    }

    /// Resolve a locale from a code, tolerating case and region variants
    /// (e.g. `zh_CN`, `zh-Hans`, `en-US`).
    pub fn from_code(code: &str) -> Option<Locale> {
        let base = code
            .split(['-', '_', '.', '@'])
            .next()
            .unwrap_or(code)
            .to_ascii_lowercase();
        match base.as_str() {
            "en" => Some(Locale::English),
            "zh" => Some(Locale::Chinese),
            "fr" => Some(Locale::French),
            "es" => Some(Locale::Spanish),
            "ru" => Some(Locale::Russian),
            "ar" => Some(Locale::Arabic),
            _ => None,
        }
    }

    /// Whether the language is written right-to-left.
    pub fn is_rtl(self) -> bool {
        matches!(self, Locale::Arabic)
    }

    fn index(self) -> usize {
        Self::ALL.iter().position(|l| *l == self).unwrap_or(0)
    }
}

/// The locale currently in use, as an index into [`Locale::ALL`].
static CURRENT: AtomicUsize = AtomicUsize::new(0);

/// Compile-time embedded translation catalogs, parsed lazily.
static CATALOGS: OnceLock<HashMap<Locale, HashMap<String, String>>> = OnceLock::new();

fn catalogs() -> &'static HashMap<Locale, HashMap<String, String>> {
    CATALOGS.get_or_init(|| {
        let mut map = HashMap::new();
        map.insert(
            Locale::Chinese,
            parse_catalog(include_str!("../../locales/zh-CN.json")),
        );
        map.insert(
            Locale::French,
            parse_catalog(include_str!("../../locales/fr.json")),
        );
        map.insert(
            Locale::Spanish,
            parse_catalog(include_str!("../../locales/es.json")),
        );
        map.insert(
            Locale::Russian,
            parse_catalog(include_str!("../../locales/ru.json")),
        );
        map.insert(
            Locale::Arabic,
            parse_catalog(include_str!("../../locales/ar.json")),
        );
        map
    })
}

fn parse_catalog(raw: &str) -> HashMap<String, String> {
    serde_json::from_str(raw).unwrap_or_default()
}

/// The locale currently in use.
pub fn current() -> Locale {
    let idx = CURRENT.load(Ordering::Relaxed).min(Locale::ALL.len() - 1);
    Locale::ALL[idx]
}

/// Switch the active locale and persist it to disk.
pub fn set_locale(locale: Locale) {
    CURRENT.store(locale.index(), Ordering::Relaxed);
    persist(locale);
}

/// Cache of resolved `(locale, key) -> leaked translation` pairs. Leaking is
/// bounded (locales × keys) and lets [`translate`] hand out `&'static str`, which
/// coerces into every `&str`/`SharedString` slot the UI uses.
static CACHE: OnceLock<Mutex<HashMap<(Locale, String), &'static str>>> = OnceLock::new();

/// Look up an English key, falling back to the key itself when untranslated.
pub fn translate(key: &str) -> &'static str {
    let locale = current();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(hit) = cache.lock().unwrap().get(&(locale, key.to_string())) {
        return hit;
    }

    let resolved = catalogs()
        .get(&locale)
        .and_then(|catalog| catalog.get(key))
        .cloned()
        .unwrap_or_else(|| key.to_string());
    let leaked: &'static str = Box::leak(resolved.into_boxed_str());
    cache
        .lock()
        .unwrap()
        .insert((locale, key.to_string()), leaked);
    leaked
}

/// Look up `key` and substitute `format!`-style placeholders.
///
/// Pairs with an empty name fill `{}` positionally, in order; named pairs fill
/// `{name}`. Unknown placeholders are left untouched.
pub fn translate_args(key: &str, args: &[(String, String)]) -> String {
    let template = translate(key);
    substitute(template, args)
}

fn substitute(template: &str, args: &[(String, String)]) -> String {
    let mut out = String::with_capacity(template.len());
    let mut positional = 0usize;
    let mut chars = template.char_indices().peekable();

    while let Some((idx, ch)) = chars.next() {
        if ch != '{' {
            out.push(ch);
            continue;
        }
        // Find the matching closing brace.
        let Some(close_rel) = template[idx + 1..].find('}') else {
            out.push(ch);
            continue;
        };
        let close = idx + 1 + close_rel;
        let inner = &template[idx + 1..close];
        let (name, spec) = match inner.split_once(':') {
            Some((n, s)) => (n, Some(s)),
            None => (inner, None),
        };

        let replaced = if name.is_empty() {
            let value = args.get(positional).map(|(_, v)| v);
            positional += 1;
            value.map(|v| apply_spec(v, spec))
        } else {
            args.iter()
                .find(|(n, _)| n == name)
                .map(|(_, v)| apply_spec(v, spec))
        };

        match replaced {
            Some(text) => out.push_str(&text),
            None => out.push_str(&template[idx..=close]),
        }
        // Skip the characters already consumed inside the braces.
        while let Some((next, _)) = chars.peek() {
            if *next <= close {
                chars.next();
            } else {
                break;
            }
        }
    }
    out
}

/// Apply the small subset of `format!` specs used across the UI to a value that
/// has already been stringified. Unsupported specs pass the value through.
fn apply_spec(value: &str, spec: Option<&str>) -> String {
    let Some(spec) = spec else {
        return value.to_string();
    };
    if spec == "?" {
        return value.to_string();
    }

    // Floating-point precision, e.g. `{:.1}`.
    if let Some(prec) = spec.strip_prefix('.') {
        if let Ok(prec) = prec.parse::<usize>() {
            if let Ok(num) = value.trim().parse::<f64>() {
                return format!("{:.prec$}", num, prec = prec);
            }
        }
        return value.to_string();
    }

    // Zero-padded integer, e.g. `{:04X}`, `{:02x}`, `{:02}`.
    let zero_padded = spec.starts_with('0');
    let rest = spec.strip_prefix('0').unwrap_or(spec);
    let (digits, ty) = match rest.chars().last() {
        Some(c) if c.is_ascii_alphabetic() => (&rest[..rest.len() - c.len_utf8()], c),
        _ => (rest, 'd'),
    };
    let width = digits.parse::<usize>().unwrap_or(0);

    match ty {
        'x' | 'X' => {
            if let Ok(num) = value.trim().parse::<u64>() {
                return if ty == 'x' {
                    format!("{:0width$x}", num, width = width)
                } else {
                    format!("{:0width$X}", num, width = width)
                };
            }
        }
        'd' if zero_padded && width > 0 => {
            if let Ok(num) = value.trim().parse::<i64>() {
                return format!("{:0width$}", num, width = width);
            }
        }
        _ => {}
    }

    value.to_string()
}

fn config_path() -> Option<PathBuf> {
    ProjectDirs::from("in", "suyogtandel", "picoforge").map(|dirs| dirs.config_dir().join("locale"))
}

fn persist(locale: Locale) {
    if let Some(path) = config_path() {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(path, locale.code());
    }
}

/// Load the persisted locale (if any) and the OS language, then activate the
/// best match. Call once at startup, before the first window opens.
pub fn init() {
    let saved = config_path()
        .and_then(|path| std::fs::read_to_string(path).ok())
        .and_then(|code| Locale::from_code(code.trim()));

    let system = ["LANG", "LC_ALL", "LC_MESSAGES"]
        .iter()
        .find_map(|var| std::env::var(var).ok())
        .and_then(|value| Locale::from_code(&value));

    if let Some(locale) = saved.or(system) {
        CURRENT.store(locale.index(), Ordering::Relaxed);
    }
}

/// Push a `(name, value)` pair while expanding a [`tr!`](crate::tr) invocation.
#[macro_export]
#[doc(hidden)]
macro_rules! __tr_push {
    ($v:ident) => {};
    ($v:ident, $name:ident = $val:expr) => {
        $v.push((::std::stringify!($name).to_string(), $val.to_string()));
    };
    ($v:ident, $name:ident = $val:expr,) => {
        $v.push((::std::stringify!($name).to_string(), $val.to_string()));
    };
    ($v:ident, $name:ident = $val:expr, $($rest:tt)+) => {
        $v.push((::std::stringify!($name).to_string(), $val.to_string()));
        $crate::__tr_push!($v, $($rest)+);
    };
    ($v:ident, $val:expr) => {
        $v.push((::std::string::String::new(), $val.to_string()));
    };
    ($v:ident, $val:expr,) => {
        $v.push((::std::string::String::new(), $val.to_string()));
    };
    ($v:ident, $val:expr, $($rest:tt)+) => {
        $v.push((::std::string::String::new(), $val.to_string()));
        $crate::__tr_push!($v, $($rest)+);
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locale_codes_round_trip() {
        for locale in Locale::ALL {
            assert_eq!(Locale::from_code(locale.code()), Some(locale));
        }
        assert_eq!(Locale::from_code("zh_CN"), Some(Locale::Chinese));
        assert_eq!(Locale::from_code("en-US"), Some(Locale::English));
        assert_eq!(Locale::from_code("xx"), None);
    }

    #[test]
    fn substitutes_positional_and_named() {
        let args = vec![
            (String::new(), "first".to_string()),
            ("name".to_string(), "Ada".to_string()),
        ];
        assert_eq!(substitute("{} met {name}", &args), "first met Ada");
    }

    #[test]
    fn applies_hex_and_float_specs() {
        assert_eq!(apply_spec("255", Some("04X")), "00FF");
        assert_eq!(apply_spec("7", Some("02x")), "07");
        assert_eq!(apply_spec("1.239", Some(".1")), "1.2");
        assert_eq!(apply_spec("raw", None), "raw");
    }

    #[test]
    fn chinese_catalog_resolves() {
        let previous = current();
        set_locale(Locale::Chinese);
        assert_eq!(translate("Device"), "设备");
        assert_eq!(translate("Not a real key"), "Not a real key");
        set_locale(previous);
    }
}
///
/// With no arguments the key is returned translated verbatim. With arguments it
/// also substitutes `format!`-style placeholders:
///
/// ```ignore
/// crate::tr!("Apply Changes");
/// crate::tr!("Error: {e}", e = err);
/// crate::tr!("{} credentials stored", count);
/// ```
#[macro_export]
macro_rules! tr {
    ($key:literal) => {
        $crate::ui::i18n::translate($key)
    };
    ($key:literal, $($args:tt)+) => {{
        let mut __tr_args: ::std::vec::Vec<(::std::string::String, ::std::string::String)> =
            ::std::vec::Vec::new();
        $crate::__tr_push!(__tr_args, $($args)+);
        $crate::ui::i18n::translate_args($key, &__tr_args)
    }};
}
