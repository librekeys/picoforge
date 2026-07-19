//! Shared types for device communication.
//!
//! Organized into three groups:
//! - Application-level types: device info, config, and status used across both protocols
//! - Rescue (PC/SC) types: LED and USB applet configuration read/written over PC/SC
//! - FIDO2 types: credential and authenticator info from CTAP2

#![allow(unused)]

use serde::{Deserialize, Serialize};
use std::fmt;

// ── Application-level types ─────────────────────────────────────────────────

/// Internal application state holding device info for the current session.
struct PForgeState {
    device_info: DeviceInfo,
}

/// Basic device identity and flash usage reported by the firmware.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DeviceInfo {
    pub serial: String,
    pub flash_used: Option<u32>,
    pub flash_total: Option<u32>,
    pub firmware_version: String,
}

/// Full device configuration (USB descriptors, LED, touch, crypto options).
#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub vid: String,
    pub pid: String,
    pub product_name: String,
    /// GPIO pin the status LED is connected to. `None` = no phy override, i.e.
    /// the firmware's build-time default (which the device doesn't report back).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub led_gpio: Option<u8>,
    /// Global LED brightness cap. `None` = no phy override (firmware default).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub led_brightness: Option<u8>,
    /// Touch-button press timeout in seconds. `None` = no phy override; the
    /// firmware then uses its built-in default (30 s), and `0` means the same.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub touch_timeout: Option<u8>,
    /// LED driver type identifier (e.g. PWM direct vs external driver).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub led_driver: Option<u8>,
    pub led_dimmable: bool,
    pub power_cycle_on_reset: bool,
    /// When set, the LED stays on (not pulsed) for touch/processing states.
    pub led_steady: bool,
    pub enable_secp256k1: bool,
    /// Bitmask of raw (unwrapped) curve identifiers supported by the firmware.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_curves_mask: Option<u32>,
    /// The order in which LED colours are sequenced during status transitions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub led_order: Option<u8>,
    /// Bitmask of USB interface endpoints that are enabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled_usb_itf: Option<u8>,
    /// Number of individual LEDs on the device.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub led_num: Option<u8>,
}

/// Partial config update; `None` fields are left unchanged on the device.
#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AppConfigInput {
    pub vid: Option<String>,
    pub pid: Option<String>,
    pub product_name: Option<String>,
    pub led_gpio: Option<u8>,
    pub led_brightness: Option<u8>,
    pub touch_timeout: Option<u8>,
    pub led_driver: Option<u8>,
    pub led_dimmable: Option<bool>,
    pub power_cycle_on_reset: Option<bool>,
    pub led_steady: Option<bool>,
    pub enable_secp256k1: Option<bool>,
    pub raw_curves_mask: Option<u32>,
    pub led_order: Option<u8>,
    pub enabled_usb_itf: Option<u8>,
    pub led_num: Option<u8>,
}

/// Aggregated snapshot of device info, config, and security state.
#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FullDeviceStatus {
    /// Basic device identity and flash usage.
    pub info: DeviceInfo,
    /// Full device configuration (USB descriptors, LED, touch, crypto).
    pub config: AppConfig,
    /// Whether secure boot is enabled on the device.
    pub secure_boot: bool,
    /// Whether the device's secure configuration is locked (read-only until reset).
    pub secure_lock: bool,
    /// Protocol channel used for the last successful communication.
    pub method: DeviceMethod,
    /// Detected firmware variant.
    pub firmware_type: FirmwareType,
}

/// Protocol channel used to communicate with the device.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum DeviceMethod {
    /// Communication over FIDO HID (CTAPHID / CTAP2).
    #[serde(rename = "FIDO")]
    Fido,
    /// Communication over PC/SC rescue channel (ISO 7816-4 APDU).
    Rescue,
}

/// Recognized firmware variants. Gates UI features, connection methods, and
/// compatibility checks throughout the application.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub enum FirmwareType {
    /// Pol Henarejos' pico-fido / pico-keys-sdk based firmware.
    PicoFido,
    /// TheMaxMur's RS-Key firmware (SDK 5.x+).
    RSKey,
    /// LibreKeys LK-ONE (pico-fido fork, same AAGUID as pico-fido).
    LkOne,
    /// Unrecognised or undetected firmware.
    #[default]
    Unknown,
}

impl fmt::Display for FirmwareType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PicoFido => write!(f, "pico-fido"),
            Self::RSKey => write!(f, "RS-Key"),
            Self::LkOne => write!(f, "LK-ONE"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

// ── Rescue (PC/SC) types ────────────────────────────────────────────────────

/// LED status configuration read from the Vendor/LED applet.
/// `statuses` is a fixed array of `(color, brightness)` pairs indexed by
/// device status: Idle, Processing, Touch, Boot.
#[derive(Serialize, Debug, Default, Clone, PartialEq)]
pub struct LedStatusConfig {
    /// Whether the LED stays on steady (true) or pulses (false).
    pub steady: bool,
    /// Fixed array of `(color, brightness)` pairs indexed by device status:
    /// Idle, Processing, Touch, Boot.
    pub statuses: [(u8, u8); 4],
}

/// USB application endpoint bitmasks from the Management applet.
/// `usb_supported` lists applets the firmware can run;
/// `usb_enabled` lists those active on next boot.
#[derive(Serialize, Debug, Default, Clone, PartialEq)]
pub struct ManagementAppConfig {
    pub usb_supported: u16,
    pub usb_enabled: u16,
}

// ── FIDO2 types ─────────────────────────────────────────────────────────────

/// Authenticator metadata from CTAP2 GetInfo.
#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FidoDeviceInfo {
    /// Supported CTAP versions reported by the authenticator.
    pub versions: Vec<String>,
    /// Supported CTAP extensions.
    pub extensions: Vec<String>,
    /// Authenticator Attestation GUID (hex-encoded, uppercase, no dashes).
    pub aaguid: String,
    /// Authenticator options map from `authenticatorGetInfo`.
    pub options: std::collections::HashMap<String, bool>,
    pub max_msg_size: i128,
    /// PIN/UV protocol versions supported.
    pub pin_protocols: Vec<u32>,
    pub remaining_discoverable_credentials: Option<i128>,
    pub min_pin_length: i128,
    /// Firmware version as reported by the authenticator (may differ from the HAL-parsed version).
    pub firmware_version: String,
    /// Supported vendor config commands (human-readable names), parsed from CTAP GetInfo.
    pub vendor_config_commands: Vec<String>,
    /// Device certifications when firmware exposes them separately from vendor commands.
    pub certifications: std::collections::HashMap<String, bool>,
    pub max_credential_count_in_list: Option<i128>,
    pub max_credential_id_length: Option<i128>,
    /// List of supported COSE algorithm display names.
    pub algorithms: Vec<String>,
    pub max_serialized_large_blob_array: Option<i128>,
    pub force_pin_change: Option<bool>,
    pub max_cred_blob_length: Option<i128>,
}

/// A single FIDO2 credential stored on the device.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredCredential {
    pub rp_id: String,
    pub rp_name: String,
    pub user_name: String,
    pub user_display_name: String,
    pub user_id: String,
    pub credential_id: String,
}

// ── Constants ───────────────────────────────────────────────────────────────

/// Re-export curve bitflags for use by UI components.
pub use crate::hal::rescue::constants::RescueCurves;

/// AAGUID assigned to RS-Key hardware.
pub const RSKEY_AAGUID: &str = "2479C7BF6B3056839EC80E8171A918B7";
/// AAGUID assigned to Pico-Fido hardware.
pub const PICOFIDO_AAGUID: &str = "89FB94B706C936739B7E30526D968145";
/// AAGUID assigned to LibreKeys LK-ONE hardware (same as pico-fido fork).
pub const LKONE_AAGUID: &str = "89FB94B706C936739B7E30526D968145";
/// LibreKeys USB VID:PID allocated by OpenMoko.
pub const LKONE_VID: u16 = 0x1D50;
pub const LKONE_PID: u16 = 0x619B;
