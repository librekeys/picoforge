//! View model for the home screen — tracks device connection state and polling.

use crate::ui::app::AppModels;
use crate::ui::models::device::{DeviceEvent, DeviceRepo};
use gpui::*;

/// Application state and device-detection polling for the home screen.
pub struct HomeViewModel {
    pub device: Entity<DeviceRepo>,
}

impl HomeViewModel {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>, models: &AppModels) -> Self {
        let device = models.device.clone();
        cx.subscribe(&device, |_, _, _: &DeviceEvent, cx| cx.notify())
            .detach();
        Self { device }
    }

    /// Map an RS-Key USB `bcdDevice` build counter to a release tag.
    ///
    /// RS-Key's `bcdDevice` is a **monotonic build counter** (bumped on every
    /// behaviour change), not a BCD-encoded version number — there is no
    /// mathematical conversion to semver.  This table provides the known
    /// mapping for released versions.  The data comes from the RS-Key
    /// CHANGELOG (<https://github.com/TheMaxMur/RS-Key/blob/main/CHANGELOG.md>)
    /// and the project's git tags.
    ///
    /// When RS-Key ships a new release, add its `bcdDevice` value(s) here.
    /// Unknown values fall back to a bare hex display in the caller.
    pub fn rs_key_version_from_bcd(bcd: u16) -> Option<&'static str> {
        // Keep sorted for readability; matches are exact.
        let (tag, _bcd) = match bcd {
            // v0.4.4 — challenge-response fixes, OTP frame protocol, touch gate
            0x0859..=0x085B => (crate::tr!("v0.4.4"), bcd),
            // v0.4.3 — CTAP 2.1 text pass, 28th security audit
            0x0857 | 0x0858 => (crate::tr!("v0.4.3"), bcd),
            // v0.4.2 — fingerprint-free credential IDs, makeCredUvNotRqd
            0x0851..=0x0855 => (crate::tr!("v0.4.2"), bcd),
            // v0.4.1 — ykman interop fixes, OATH CALCULATE ALL, CCID ATR
            0x084A..=0x0850 => (crate::tr!("v0.4.1"), bcd),
            // v0.4.0 — USB identity, audit journal, security fixes
            0x083D | 0x0847 | 0x0848 | 0x0849 => (crate::tr!("v0.4.0"), bcd),
            _ => return None,
        };
        Some(tag)
    }
}
