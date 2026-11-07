//! Embedded asset loader for SVG icons used throughout the application.
//!
//! Uses `rust-embed` for application SVGs under `static/`, then falls back to
//! Kit's bundled icons. Application artwork takes precedence on shared paths.

use gpui_kit::*;
use rust_embed::RustEmbed;
use std::borrow::Cow;

/// Application SVG assets with a fallback to GPUI Kit's component icons.
#[derive(RustEmbed)]
#[folder = "./static"]
#[include = "**/*.svg"]
pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if path.is_empty() {
            return Ok(None);
        }

        if let Some(asset) = Self::get(path) {
            return Ok(Some(asset.data));
        }
        gpui_kit::assets::Assets.load(path)
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut paths: Vec<SharedString> = Self::iter()
            .filter_map(|p| p.starts_with(path).then(|| p.into()))
            .collect();
        paths.extend(gpui_kit::assets::Assets.list(path)?);
        paths.sort();
        paths.dedup();
        Ok(paths)
    }
}

#[cfg(test)]
mod tests {
    use super::Assets;
    use gpui_kit::AssetSource;

    #[test]
    fn resolves_application_and_framework_icons() {
        let source = Assets;
        let path = "icons/cpu.svg";
        assert_eq!(
            source.load(path).unwrap().unwrap(),
            Assets::get(path).unwrap().data
        );
        let framework_paths = gpui_kit::assets::Assets.list("").unwrap();
        assert!(!framework_paths.is_empty());
        for path in framework_paths {
            assert!(source.load(&path).unwrap().is_some(), "missing {path}");
        }
        assert!(source.load("missing-icon.svg").is_err());
        assert!(source.load("").unwrap().is_none());
        let paths = source.list("").unwrap();
        assert!(paths.windows(2).all(|pair| pair[0] < pair[1]));
    }
}
