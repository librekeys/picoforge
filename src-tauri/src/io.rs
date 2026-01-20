//! Tauri Commands to interact with the pico-fido firmware via rescue and fido protocols.
use crate::{error::PFError, fido, rescue, types::*};

#[tauri::command]
pub fn read_device_details() -> Result<FullDeviceStatus, PFError> {
	match rescue::read_device_details() {
		Ok(status) => Ok(status),
		Err(e) => {
			log::warn!("Rescue method failed: {}. Falling back to FIDO...", e);
			fido::read_device_details()
		}
	}
}

#[tauri::command]
pub fn write_config(config: AppConfigInput) -> Result<String, PFError> {
	rescue::write_config(config)
}

#[tauri::command]
pub fn enable_secure_boot(lock: bool) -> Result<String, PFError> {
	rescue::enable_secure_boot(lock)
}

#[tauri::command]
pub(crate) fn get_fido_info() -> Result<FidoDeviceInfo, String> {
	fido::get_fido_info()
}

#[tauri::command]
pub(crate) fn change_fido_pin(
	current_pin: Option<String>,
	new_pin: String,
) -> Result<String, String> {
	fido::change_fido_pin(current_pin, new_pin)
}

/// UNSTABLE!
#[tauri::command]
pub(crate) fn set_min_pin_length(
	current_pin: String,
	min_pin_length: u8,
) -> Result<String, String> {
	fido::set_min_pin_length(current_pin, min_pin_length)
}

#[tauri::command]
pub fn reboot(to_bootsel: bool) -> Result<String, PFError> {
	rescue::reboot_device(to_bootsel)
}

#[tauri::command]
pub async fn get_credentials(pin: String) -> Result<Vec<StoredCredential>, String> {
	tauri::async_runtime::spawn_blocking(move || fido::get_credentials(pin))
		.await
		.map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn delete_credential(pin: String, credential_id: String) -> Result<String, String> {
	tauri::async_runtime::spawn_blocking(move || fido::delete_credential(pin, credential_id))
		.await
		.map_err(|e| e.to_string())?
}
