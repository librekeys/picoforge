//! Device I/O layer bridging rescue (pcsc) and FIDO2 protocols.
//!
//! High-level entry points for reading/writing device configuration,
//! managing credentials, and controlling LED/boot behavior.
//!
//! Functions are grouped by the protocol they use:
//! - Functions that use both rescue and FIDO (fallback/dispatch logic)
//! - Functions that communicate exclusively over the rescue (PC/SC) channel
//! - Functions that communicate exclusively over the FIDO2 channel

#![allow(unused)]

use crate::{error::PFError, hal::fido, hal::rescue, hal::types::*};

// ── Shared: functions that use both rescue and FIDO ─────────────────────────

/// Read full device status. Tries rescue first, falls back to FIDO on failure.
pub fn read_device_details() -> Result<FullDeviceStatus, PFError> {
    match rescue::read_device_details() {
        Ok(status) => Ok(status),
        Err(e) => {
            log::warn!("Rescue method failed: {}. Falling back to FIDO...", e);
            fido::read_device_details()
        }
    }
}

/// Write app config. Dispatches to rescue or FIDO based on `method`.
pub fn write_config(
    config: AppConfigInput,
    method: DeviceMethod,
    pin: Option<String>,
) -> Result<String, PFError> {
    if method == DeviceMethod::Fido {
        fido::write_config(config, pin)
    } else {
        rescue::write_config(config)
    }
}

// ── Rescue protocol (PC/SC) ─────────────────────────────────────────────────

/// Lock or unlock secure boot via rescue.
pub fn enable_secure_boot(lock: bool) -> Result<String, PFError> {
    rescue::enable_secure_boot(lock)
}

/// Reboot the device. Pass `true` to enter BOOTSEL mode.
pub fn reboot(to_bootsel: bool) -> Result<String, PFError> {
    rescue::reboot_device(to_bootsel)
}

/// Read current LED status config via rescue.
pub fn read_led_config() -> Result<LedStatusConfig, PFError> {
    rescue::read_led_config()
}

/// Write LED status (on/off, color, brightness, steady/blinking).
pub fn write_led_status(
    status: u8,
    color: u8,
    brightness: u8,
    steady: bool,
) -> Result<String, PFError> {
    rescue::write_led_status(status, color, brightness, steady)
}

/// Read management app config via rescue.
pub fn read_management_config() -> Result<ManagementAppConfig, PFError> {
    rescue::read_management_config()
}

/// Write management app enabled-mask via rescue.
pub fn write_management_config(enabled_mask: u16) -> Result<String, PFError> {
    rescue::write_management_config(enabled_mask)
}

// ── FIDO2 protocol ──────────────────────────────────────────────────────────

/// Query basic FIDO device info (AAGUID, version, etc.).
pub(crate) fn get_fido_info() -> Result<FidoDeviceInfo, String> {
    fido::get_fido_info()
}

/// Change the FIDO user PIN.
pub(crate) fn change_fido_pin(
    current_pin: Option<String>,
    new_pin: String,
) -> Result<String, String> {
    fido::change_fido_pin(current_pin, new_pin)
}

/// Set the minimum PIN length requirement.
pub(crate) fn set_min_pin_length(
    current_pin: String,
    min_pin_length: u8,
) -> Result<String, String> {
    fido::set_min_pin_length(current_pin, min_pin_length)
}

/// List stored credentials for the given PIN.
pub fn get_credentials(pin: String) -> Result<Vec<StoredCredential>, String> {
    fido::get_credentials(pin)
}

/// Delete a single credential by its ID.
pub fn delete_credential(pin: String, credential_id: String) -> Result<String, String> {
    fido::delete_credential(pin, credential_id)
}

/// Factory-reset the device, wiping all credentials and settings.
pub fn reset_device() -> Result<String, String> {
    fido::reset_device()
}

/// Enable enterprise attestation for the device.
pub fn enable_enterprise_attestation(pin: String) -> Result<String, String> {
    fido::enable_enterprise_attestation(pin)
}

/// Retrieve the enterprise attestation CSR.
pub fn get_enterprise_attestation_csr() -> Result<String, String> {
    fido::get_enterprise_attestation_csr()
}

/// Upload a signed enterprise attestation certificate.
pub fn upload_enterprise_attestation_cert(
    pin: String,
    cert_path: String,
) -> Result<String, String> {
    fido::upload_enterprise_attestation_cert(pin, cert_path)
}
