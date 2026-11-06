#![allow(dead_code)]

//! Firmware-type abstraction layer.
//!
//! This module provides the [`FirmwareTrait`] trait and two concrete
//! implementations ([`PicoFidoFirmware`], [`RSKeyFirmware`]) that
//! encapsulate per-firmware capability gating. The trait methods are
//! checked throughout [`crate::hal::io`] to select the correct command
//! path for each operation.
//!
//! ## Detection
//!
//! Firmware type is determined at transport-scan time via AAGUID lookup
//! in [`AnyFirmware::detect_by_aaguid`]. The AAGUID constants live in
//! [`crate::hal::types`]. LK-ONE shares pico-fido's AAGUID and is
//! treated as a pico-fido variant.
//!
//! ## Trait methods
//!
//! | Method | What it gates |
//! |---|---|
//! | `supports_legacy_fido_hardware_config` | Whether the device accepts legacy `vendorPrototype` 0xFF commands for hardware config (pico-fido ≤ 7.2, or RS-Key which uses a separate `0x41` path). |
//! | `supports_fido_config_write` | Whether `authenticatorConfig` + `vendorPrototype` writes can be used for config. |
//! | `supports_rs_key_vendor_command` | Whether RS-Key-specific vendor commands (0x05 etc.) are available. |
//! | `supports_rescue_channel` | Whether the PC/SC rescue channel is accessible. |

pub mod applets;
pub mod picofido;
pub mod rskey;

pub use picofido::*;
pub use rskey::*;

use crate::hal::common::FirmwareVersion;
use crate::hal::types::*;

/// Dispatch enum wrapping a concrete firmware implementation.
///
/// Most callers interact through the [`FirmwareTrait`] methods rather
/// than matching on variants directly.
#[derive(Debug, Clone)]
pub enum AnyFirmware {
    PicoFido(PicoFidoFirmware),
    RSKey(RSKeyFirmware),
}

/// Capability gating per firmware variant.
///
/// Each method queries a static or version-derived capability flag.
/// The concrete implementations in [`PicoFidoFirmware`] and
/// [`RSKeyFirmware`] encode the known compatibility boundaries for
/// each firmware.
pub trait FirmwareTrait {
    fn firmware_type(&self) -> FirmwareType;
    fn version(&self) -> &FirmwareVersion;
    fn major_minor(&self) -> (u16, u16) {
        (self.version().major, self.version().minor)
    }
    fn version_str(&self) -> &str {
        &self.version().raw
    }

    /// Whether the firmware accepts legacy FIDO hardware-config commands.
    fn supports_legacy_fido_hardware_config(&self) -> bool;
    /// Whether the firmware accepts FIDO config writes.
    fn supports_fido_config_write(&self) -> bool;
    /// Whether RS-Key-specific vendor commands are available.
    fn supports_rs_key_vendor_command(&self) -> bool;
    /// Whether the PC/SC rescue channel can be activated.
    fn supports_rescue_channel(&self) -> bool;
}

impl AnyFirmware {
    /// Detect firmware type from the authenticator's AAGUID hex string.
    pub fn detect_by_aaguid(aaguid: &str) -> FirmwareType {
        if aaguid == crate::hal::types::RSKEY_AAGUID {
            FirmwareType::RSKey
        } else if aaguid == crate::hal::types::PICOFIDO_AAGUID
            || aaguid == crate::hal::types::LKONE_AAGUID
        {
            FirmwareType::PicoFido
        } else if aaguid == crate::hal::types::FAPICO2_AAGUID {
            FirmwareType::Fapico2
        } else {
            FirmwareType::Unknown
        }
    }

    /// Construct an `AnyFirmware` from a known firmware type and version string.
    ///
    /// LkOne and Unknown are treated as pico-fido variants.
    pub fn new(firmware_variant: FirmwareType, version: &str) -> Self {
        let firmware_version = FirmwareVersion::parse(version).unwrap_or_default();
        match firmware_variant {
            FirmwareType::PicoFido => Self::PicoFido(PicoFidoFirmware::new(firmware_version)),
            FirmwareType::RSKey => Self::RSKey(RSKeyFirmware::new(firmware_version)),
            // Provisional: US-1753 (EPIC PF-FAPICO2) replaces this arm with a
            // real Fapico2Firmware dispatch once firmwares/fapico2.rs exists.
            FirmwareType::Fapico2 => Self::PicoFido(PicoFidoFirmware::new(firmware_version)),
            FirmwareType::LkOne | FirmwareType::Unknown => {
                Self::PicoFido(PicoFidoFirmware::new(firmware_version))
            }
        }
    }

    /// Construct an `AnyFirmware` with an explicit legacy-vendor flag for pico-fido.
    ///
    /// The flag is only meaningful for `FirmwareType::PicoFido`; other types
    /// ignore it.
    pub fn new_with_legacy(
        firmware_variant: FirmwareType,
        version: &str,
        has_legacy_vendor: bool,
    ) -> Self {
        let firmware_version = FirmwareVersion::parse(version).unwrap_or_default();
        match firmware_variant {
            FirmwareType::PicoFido => Self::PicoFido(
                PicoFidoFirmware::new(firmware_version).with_legacy_vendor(has_legacy_vendor),
            ),
            FirmwareType::RSKey => Self::RSKey(RSKeyFirmware::new(firmware_version)),
            // Provisional: US-1753 (EPIC PF-FAPICO2) replaces this arm with a
            // real Fapico2Firmware dispatch once firmwares/fapico2.rs exists.
            FirmwareType::Fapico2 => Self::PicoFido(PicoFidoFirmware::new(firmware_version)),
            FirmwareType::LkOne | FirmwareType::Unknown => {
                Self::PicoFido(PicoFidoFirmware::new(firmware_version))
            }
        }
    }

    /// Delegate to the inner firmware's version.
    pub fn version(&self) -> &FirmwareVersion {
        match self {
            Self::PicoFido(fw) => fw.version(),
            Self::RSKey(fw) => fw.version(),
        }
    }

    /// Return the concrete [`FirmwareType`] of the inner firmware.
    pub fn firmware_type(&self) -> FirmwareType {
        match self {
            Self::PicoFido(_) => FirmwareType::PicoFido,
            Self::RSKey(_) => FirmwareType::RSKey,
        }
    }

    /// Whether the inner firmware supports legacy FIDO hardware-config commands.
    pub fn supports_legacy_fido_hardware_config(&self) -> bool {
        match self {
            Self::PicoFido(fw) => fw.supports_legacy_fido_hardware_config(),
            Self::RSKey(fw) => fw.supports_legacy_fido_hardware_config(),
        }
    }

    /// Whether the inner firmware supports FIDO config writes.
    pub fn supports_fido_config_write(&self) -> bool {
        match self {
            Self::PicoFido(fw) => fw.supports_fido_config_write(),
            Self::RSKey(fw) => fw.supports_fido_config_write(),
        }
    }

    /// Whether the inner firmware supports the new-style (post-v7.2) FIDO hardware config path.
    ///
    /// This is the logical negation of `supports_legacy_fido_hardware_config` for
    /// pico-fido, and always `false` for RS-Key.
    pub fn supports_new_fido_hardware_config(&self) -> bool {
        match self {
            Self::PicoFido(fw) => !fw.supports_legacy_fido_hardware_config(),
            Self::RSKey(_) => false,
        }
    }

    /// Whether RS-Key-specific vendor commands are available on the inner firmware.
    pub fn supports_rs_key_vendor_command(&self) -> bool {
        match self {
            Self::PicoFido(_) => false,
            Self::RSKey(fw) => fw.supports_rs_key_vendor_command(),
        }
    }

    /// Whether the PC/SC rescue channel can be used with the inner firmware.
    pub fn supports_rescue_channel(&self) -> bool {
        match self {
            Self::PicoFido(_) => true,
            Self::RSKey(_) => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_by_aaguid_fapico2() {
        // US-1750 (EPIC PF-FAPICO2): pin the AAGUID gate for a stock fapico2
        // getInfo. The classification must land on Fapico2 — i.e.
        // detect_by_aaguid must not fall to the pico-fido profile and must
        // not report Unknown. The detector only knows RS-Key, pico-fido and
        // LK-ONE AAGUIDs today, so this RED test fails: the stock fapico2
        // AAGUID falls through to Unknown (and via AnyFirmware::new, to the
        // pico-fido dispatch path).
        let detected = AnyFirmware::detect_by_aaguid("66617069636F32000000000000000002");
        assert_ne!(detected, FirmwareType::PicoFido);
        assert_ne!(detected, FirmwareType::Unknown);
    }
}
