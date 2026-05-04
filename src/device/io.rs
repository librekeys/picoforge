//! Tauri Commands to interact with the pico-fido firmware via rescue and fido protocols.
#![allow(unused)]

use crate::{device::fido, device::oath, device::rescue, device::types::*, error::PFError};

pub fn read_device_details() -> Result<FullDeviceStatus, PFError> {
    match rescue::read_device_details() {
        Ok(status) => Ok(status),
        Err(e) => {
            log::warn!("Rescue method failed: {}. Falling back to FIDO...", e);
            fido::read_device_details()
        }
    }
}

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

pub fn enable_secure_boot(lock: bool) -> Result<String, PFError> {
    rescue::enable_secure_boot(lock)
}

pub(crate) fn get_fido_info() -> Result<FidoDeviceInfo, String> {
    fido::get_fido_info()
}

pub(crate) fn change_fido_pin(
    current_pin: Option<String>,
    new_pin: String,
) -> Result<String, String> {
    fido::change_fido_pin(current_pin, new_pin)
}

pub(crate) fn set_min_pin_length(
    current_pin: String,
    min_pin_length: u8,
) -> Result<String, String> {
    fido::set_min_pin_length(current_pin, min_pin_length)
}

pub(crate) fn get_fingerprint_status(pin: Option<String>) -> Result<FingerprintStatus, String> {
    fido::get_fingerprint_status(pin)
}

pub(crate) fn enroll_fingerprint(
    pin: String,
    timeout_ms: Option<u16>,
) -> Result<FingerprintEnrollResult, String> {
    fido::enroll_fingerprint(pin, timeout_ms)
}

pub(crate) fn rename_fingerprint(
    pin: String,
    template_id: String,
    friendly_name: String,
) -> Result<String, String> {
    fido::rename_fingerprint(pin, template_id, friendly_name)
}

pub(crate) fn remove_fingerprint(pin: String, template_id: String) -> Result<String, String> {
    fido::remove_fingerprint(pin, template_id)
}

pub(crate) fn get_totp_status(password: Option<String>) -> Result<TotpStatus, String> {
    oath::get_totp_status(password).map_err(|e| e.to_string())
}

pub(crate) fn import_totp_uri(uri: String, password: Option<String>) -> Result<String, String> {
    oath::import_totp_uri(uri, password).map_err(|e| e.to_string())
}

pub(crate) fn set_totp_password(
    current_password: Option<String>,
    new_password: String,
) -> Result<String, String> {
    oath::set_totp_password(current_password, new_password).map_err(|e| e.to_string())
}

pub(crate) fn rename_totp(
    old_name: String,
    new_name: String,
    password: Option<String>,
) -> Result<String, String> {
    oath::rename_totp(old_name, new_name, password).map_err(|e| e.to_string())
}

pub(crate) fn delete_totp(name: String, password: Option<String>) -> Result<String, String> {
    oath::delete_totp(name, password).map_err(|e| e.to_string())
}

pub fn reboot(to_bootsel: bool) -> Result<String, PFError> {
    rescue::reboot_device(to_bootsel)
}

pub fn get_credentials(pin: String) -> Result<Vec<StoredCredential>, String> {
    fido::get_credentials(pin)
}

pub fn delete_credential(pin: String, credential_id: String) -> Result<String, String> {
    fido::delete_credential(pin, credential_id)
}
