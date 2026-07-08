//! Firmware version parsing and comparison helpers.
//!
//! Firmware version strings follow a `major.minor[.patch]` format.
//! Two-part versions (e.g. `7.6`) are common; three-part versions
//! appear on newer firmware releases. The methods on [`FirmwareVersion`]
//! are used throughout the HAL to gate feature enablement based on
//! known firmware compatibility boundaries.

#![allow(dead_code)]

use std::fmt;

/// Parsed firmware version supporting semantic comparison.
///
/// Each version is split on `.` — the first component becomes `major`,
/// the second `minor`, and an optional third becomes `patch`. The raw
/// string is preserved for display purposes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirmwareVersion {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
    pub raw: String,
}

impl FirmwareVersion {
    /// Parse a version string in `major.minor` or `major.minor.patch` format.
    ///
    /// Returns `None` if the string has fewer than two components, or if
    /// any component is not a valid unsigned integer.
    pub fn parse(version: &str) -> Option<Self> {
        let parts: Vec<&str> = version.split('.').collect();
        let major = parts.first()?.parse().ok()?;
        let minor = parts.get(1)?.parse().ok()?;
        let patch = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);
        Some(Self {
            major,
            minor,
            patch,
            raw: version.to_string(),
        })
    }

    /// Returns `true` when `self >= (major, minor)`.
    ///
    /// Only major and minor are compared; patch is ignored so that
    /// two-part version strings (e.g. `7.2`) compare correctly with
    /// three-part ones (e.g. `7.2.1`).
    pub fn is_at_least(&self, major: u16, minor: u16) -> bool {
        self.major > major || (self.major == major && self.minor >= minor)
    }

    /// Returns `true` when `lo <= self <= hi`.
    ///
    /// The upper bound is inclusive. Calls [`is_at_least`](Self::is_at_least)
    /// for the lower bound and performs a symmetric comparison for the upper.
    pub fn is_between(&self, lo_major: u16, lo_minor: u16, hi_major: u16, hi_minor: u16) -> bool {
        self.is_at_least(lo_major, lo_minor)
            && (self.major < hi_major || (self.major == hi_major && self.minor <= hi_minor))
    }
}

impl fmt::Display for FirmwareVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.raw)
    }
}

impl Default for FirmwareVersion {
    fn default() -> Self {
        Self {
            major: 0,
            minor: 0,
            patch: 0,
            raw: "0.0".into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_two_part_version() {
        let v = FirmwareVersion::parse("7.6").unwrap();
        assert_eq!(v.major, 7);
        assert_eq!(v.minor, 6);
        assert_eq!(v.patch, 0);
        assert_eq!(v.raw, "7.6");
    }

    #[test]
    fn test_parse_three_part_version() {
        let v = FirmwareVersion::parse("5.7.4").unwrap();
        assert_eq!(v.major, 5);
        assert_eq!(v.minor, 7);
        assert_eq!(v.patch, 4);
        assert_eq!(v.raw, "5.7.4");
    }

    #[test]
    fn test_parse_single_part_fails() {
        assert!(FirmwareVersion::parse("7").is_none());
    }

    #[test]
    fn test_parse_non_numeric_fails() {
        assert!(FirmwareVersion::parse("a.b").is_none());
        assert!(FirmwareVersion::parse("7.x").is_none());
    }

    #[test]
    fn test_parse_empty_fails() {
        assert!(FirmwareVersion::parse("").is_none());
    }

    #[test]
    fn test_is_at_least_exact_match() {
        let v = FirmwareVersion::parse("7.2").unwrap();
        assert!(v.is_at_least(7, 2));
    }

    #[test]
    fn test_is_at_least_above() {
        let v = FirmwareVersion::parse("7.6").unwrap();
        assert!(v.is_at_least(7, 2));
        assert!(v.is_at_least(6, 0));
        assert!(v.is_at_least(7, 6));
    }

    #[test]
    fn test_is_at_least_below() {
        let v = FirmwareVersion::parse("7.0").unwrap();
        assert!(!v.is_at_least(7, 2));
        assert!(!v.is_at_least(8, 0));
    }

    #[test]
    fn test_is_between_inclusive_range() {
        let v = FirmwareVersion::parse("7.2").unwrap();
        assert!(v.is_between(6, 0, 8, 0));
        assert!(v.is_between(7, 0, 7, 2));
        assert!(v.is_between(7, 2, 7, 2));
    }

    #[test]
    fn test_is_between_outside_range() {
        let v = FirmwareVersion::parse("7.6").unwrap();
        assert!(!v.is_between(6, 0, 7, 2));
        assert!(!v.is_between(8, 0, 9, 0));
    }

    #[test]
    fn test_default_version() {
        let v = FirmwareVersion::default();
        assert_eq!(v.major, 0);
        assert_eq!(v.minor, 0);
        assert_eq!(v.patch, 0);
        assert_eq!(v.raw, "0.0");
    }

    #[test]
    fn test_display() {
        let v = FirmwareVersion::parse("7.6.1").unwrap();
        assert_eq!(v.to_string(), "7.6.1");
    }

    #[test]
    fn test_parse_with_patch_zero() {
        let v = FirmwareVersion::parse("7.6.0").unwrap();
        assert_eq!(v.major, 7);
        assert_eq!(v.minor, 6);
        assert_eq!(v.patch, 0);
    }

    #[test]
    fn test_legacy_fido_config_boundaries() {
        // <= 7.2 supports legacy FIDO hardware config
        assert!(FirmwareVersion::parse("7.2").unwrap().is_at_least(0, 0));
        assert!(
            !FirmwareVersion::parse("7.3")
                .unwrap()
                .is_between(0, 0, 7, 2)
        );
        assert!(
            FirmwareVersion::parse("7.2")
                .unwrap()
                .is_between(0, 0, 7, 2)
        );
        assert!(
            FirmwareVersion::parse("6.6")
                .unwrap()
                .is_between(0, 0, 7, 2)
        );
    }
}
