//! pico-fido / pico-keys-sdk firmware implementation.
//!
//! The [`PicoFidoFirmware`] struct stores a parsed version and an optional
//! legacy-vendor flag. The `supports_legacy_fido_hardware_config` method
//! returns `true` when the firmware is ≤ 7.2 or when the legacy probe
//! succeeded – gating the old `vendorPrototype` 0xFF command path.

use crate::hal::common::FirmwareVersion;
use crate::hal::firmwares::FirmwareTrait;
use crate::hal::types::FirmwareType;

/// Firmware implementation for pico-fido / pico-keys-sdk devices.
///
/// Version-gates the legacy vendor-prototype hardware config path:
/// - Versions ≤ 7.2 (major < 7, or 7.x where x ≤ 2) support the legacy path.
/// - Versions ≥ 7.3 require the rescue channel or the new-style config.
/// - An explicit `has_legacy_vendor` probe result can override the version check.
#[derive(Debug, Clone)]
pub struct PicoFidoFirmware {
    version: FirmwareVersion,
    /// Whether the device responded positively to the legacy
    /// VendorPrototype 0xFF probe (PicoForge CONFIG_PHY_* commands).
    has_legacy_vendor: bool,
}

impl PicoFidoFirmware {
    /// Create a new pico-fido firmware state with no legacy vendor flag.
    pub fn new(version: FirmwareVersion) -> Self {
        Self {
            version,
            has_legacy_vendor: false,
        }
    }

    pub fn with_legacy_vendor(mut self, legacy: bool) -> Self {
        self.has_legacy_vendor = legacy;
        self
    }
}

impl FirmwareTrait for PicoFidoFirmware {
    fn firmware_type(&self) -> FirmwareType {
        FirmwareType::PicoFido
    }

    fn version(&self) -> &FirmwareVersion {
        &self.version
    }

    fn supports_legacy_fido_hardware_config(&self) -> bool {
        self.has_legacy_vendor
            || self.version.major < 7
            || (self.version.major == 7 && self.version.minor <= 2)
    }

    fn supports_fido_config_write(&self) -> bool {
        self.has_legacy_vendor || self.version.major >= 7
    }

    fn supports_rs_key_vendor_command(&self) -> bool {
        false
    }

    fn supports_rescue_channel(&self) -> bool {
        true
    }
}
