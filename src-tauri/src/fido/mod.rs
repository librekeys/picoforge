#![allow(unused)]

pub mod constants;
pub mod hid;

use crate::{
	error::PFError,
	types::{
		AppConfig, AppConfigInput, DeviceInfo, FidoDeviceInfo, FullDeviceStatus, StoredCredential,
	},
};
use constants::*;
use ctap_hid_fido2::{
	Cfg, FidoKeyHidFactory,
	fidokey::make_credential::{MakeCredentialArgs, MakeCredentialArgsBuilder},
	public_key_credential_descriptor::PublicKeyCredentialDescriptor,
	public_key_credential_user_entity::PublicKeyCredentialUserEntity,
};
use hid::*;
use rand::Rng;
use serde_cbor_2::{Value, from_slice, to_vec};
use std::collections::{BTreeMap, HashMap};

// Fido functions that require pin: ( Uses ctap_hid_fido2 crate)

pub(crate) fn get_fido_info() -> Result<FidoDeviceInfo, String> {
	let cfg = Cfg::init();

	let device = FidoKeyHidFactory::create(&cfg)
		.map_err(|_| "Could not connect to FIDO device. Is it plugged in?".to_string())?;

	let info = device
		.get_info()
		.map_err(|e| format!("Error reading device info: {:?}", e))?;

	let options_map: HashMap<String, bool> = info.options.into_iter().collect();

	Ok(FidoDeviceInfo {
		versions: info.versions,
		extensions: info.extensions,
		aaguid: hex::encode_upper(info.aaguid),
		options: options_map,
		max_msg_size: info.max_msg_size,
		pin_protocols: info.pin_uv_auth_protocols,
		min_pin_length: info.min_pin_length,
		firmware_version: format!(
			"{}.{}",
			(info.firmware_version >> 8) & 0xFF,
			info.firmware_version & 0xFF
		),
	})
}

pub(crate) fn change_fido_pin(
	current_pin: Option<String>,
	new_pin: String,
) -> Result<String, String> {
	let cfg = Cfg::init();
	let device = FidoKeyHidFactory::create(&cfg)
		.map_err(|e| format!("Failed to connect to FIDO device: {:?}", e))?;

	match current_pin {
		Some(old) => {
			device
				.change_pin(&old, &new_pin)
				.map_err(|e| format!("Failed to change PIN: {:?}", e))?;
			Ok("PIN Changed Successfully".into())
		}
		None => {
			device
				.set_new_pin(&new_pin)
				.map_err(|e| format!("Failed to set PIN: {:?}", e))?;
			Ok("PIN Set Successfully".into())
		}
	}
}

pub(crate) fn set_min_pin_length(
	current_pin: String,
	min_pin_length: u8,
) -> Result<String, String> {
	log::info!("Starting set_min_pin_length (custom implementation)...");

	// 1. Obtain PIN token using the library handle
	let pin_token = {
		let cfg = Cfg::init();
		let device = FidoKeyHidFactory::create(&cfg)
			.map_err(|e| format!("Could not connect to FIDO device: {:?}", e))?;

		use ctap_hid_fido2::fidokey::pin::Permission;
		// Obtain a token with AuthenticatorConfiguration permission (CTAP 2.1)
		match device.get_pinuv_auth_token_with_permission(
			&current_pin,
			Permission::AuthenticatorConfiguration,
		) {
			Ok(token) => {
				log::debug!("Successfully obtained PIN token with ACFG permission.");
				token.key
			}
			Err(e) => {
				log::error!("Failed to get PIN token with ACFG permission: {:?}", e);
				return Err(format!("Failed to obtain PIN token: {:?}", e));
			}
		}
		// Library handle 'device' is dropped here, closing the HID session.
	};

	// 2. Open custom HidTransport and send command using the token because ctap-hid-fido2 has a bug where it sends CBOR map keys out of order (0x01, 0x03, 0x04, 0x02) instead of the required ascending order (0x01, 0x02, 0x03, 0x04). The pico-fido firmware strictly requires ascending order.
	let transport =
		HidTransport::open().map_err(|e| format!("Could not open HID transport: {}", e))?;

	transport
		.send_config_set_min_pin_length(&pin_token, min_pin_length)
		.map_err(|e| format!("Failed to set minimum PIN length: {}", e))?;

	Ok(format!(
		"Minimum PIN length successfully set to {}",
		min_pin_length
	))
}

pub(crate) fn get_credentials(pin: String) -> Result<Vec<StoredCredential>, String> {
	let cfg = Cfg::init();
	let device = FidoKeyHidFactory::create(&cfg)
		.map_err(|e| format!("Failed to connect to FIDO device: {:?}", e))?;

	let rps = match device.credential_management_enumerate_rps(Some(&pin)) {
		Ok(rps) => rps,
		Err(e) => {
			// CTAP2_ERR_NO_CREDENTIALS (0x2E) means no credentials exist - return empty list
			let err_str = format!("{:?}", e);
			if err_str.contains("0x2E") || err_str.contains("NO_CREDENTIALS") {
				log::info!("No credentials stored on device (CTAP2_ERR_NO_CREDENTIALS)");
				return Ok(Vec::new());
			}
			return Err(format!("Failed to enumerate Relying Parties: {:?}", e));
		}
	};

	let mut all_credentials = Vec::new();

	for rp in rps {
		let creds = device
			.credential_management_enumerate_credentials(Some(&pin), &rp.rpid_hash)
			.map_err(|e| {
				format!(
					"Failed to enumerate credentials for RP {}: {:?}",
					rp.public_key_credential_rp_entity.id, e
				)
			})?;

		for cred in creds {
			all_credentials.push(StoredCredential {
				credential_id: hex::encode(&cred.public_key_credential_descriptor.id),
				rp_id: rp.public_key_credential_rp_entity.id.clone(),
				rp_name: rp.public_key_credential_rp_entity.name.clone(),
				user_name: cred.public_key_credential_user_entity.name.clone(),
				user_display_name: cred.public_key_credential_user_entity.display_name.clone(),
				user_id: hex::encode(&cred.public_key_credential_user_entity.id).clone(),
			});
		}
	}

	Ok(all_credentials)
}

pub(crate) fn delete_credential(pin: String, credential_id_hex: String) -> Result<String, String> {
	let cfg = Cfg::init();
	let device = FidoKeyHidFactory::create(&cfg)
		.map_err(|e| format!("Failed to connect to FIDO device: {:?}", e))?;

	let cred_id_bytes = hex::decode(&credential_id_hex)
		.map_err(|_| "Invalid Credential ID Hex string".to_string())?;

	let descriptor = PublicKeyCredentialDescriptor {
		ctype: "public-key".to_string(),
		id: cred_id_bytes,
	};

	device
		.credential_management_delete_credential(Some(&pin), descriptor)
		.map_err(|e| format!("Failed to delete credential: {:?}", e))?;

	Ok("Credential deleted successfully".into())
}

// Custom Fido functions ( works only with pico-fido firmware )

pub fn read_device_details() -> Result<FullDeviceStatus, PFError> {
	log::info!("Starting FIDO device details read...");

	let transport = HidTransport::open().map_err(|e| {
		if matches!(e, PFError::NoDevice) {
			PFError::NoDevice
		} else {
			log::error!("Failed to open HID transport: {}", e);
			PFError::Device(e.to_string())
		}
	})?;

	// --- 1. Get Info ---
	log::debug!("Sending GetInfo command (0x04)...");
	let info_payload = [CtapCommand::GetInfo as u8];
	let info_res = transport
		.send_cbor(CTAPHID_CBOR, &info_payload)
		.map_err(|e| {
			log::error!("GetInfo CTAP command failed: {}", e);
			PFError::Device(format!("GetInfo failed: {}", e))
		})?;

	log::debug!("GetInfo response received ({} bytes)", info_res.len());

	let info_val: Value = from_slice(&info_res).map_err(|e| {
		log::error!("Failed to parse GetInfo CBOR: {}", e);
		PFError::Io(e.to_string())
	})?;

	// NOTE: Key 0x03 is AAGUID, not the unique device Serial.
	let aaguid_str = if let Value::Map(m) = &info_val {
		m.get(&Value::Integer(0x03))
			.and_then(|v| {
				if let Value::Bytes(b) = v {
					Some(hex::encode_upper(b))
				} else {
					None
				}
			})
			.unwrap_or_else(|| {
				log::warn!("AAGUID not found in GetInfo response");
				"Unknown".into()
			})
	} else {
		"Unknown".into()
	};

	let fw_version = if let Value::Map(m) = &info_val {
		m.get(&Value::Integer(0x0E))
			.and_then(|v| {
				if let Value::Integer(i) = v {
					Some(format!("{}.{}", (i >> 8) & 0xFF, i & 0xFF))
				} else {
					None
				}
			})
			.unwrap_or_else(|| {
				log::warn!("Firmware version not found in GetInfo response");
				"Unknown".into()
			})
	} else {
		"Unknown".into()
	};

	log::info!(
		"Device identified: AAGUID={}, FW={}",
		aaguid_str,
		fw_version
	);

	// --- 2. Get Memory Stats ---
	log::debug!("Preparing Memory Stats vendor command...");

	// FIX: The CBOR map should only contain the arguments ({1: 1}), not the command category.
	let mut mem_req = BTreeMap::new();
	mem_req.insert(
		Value::Integer(1), // Sub-command key (usually 1)
		Value::Integer(MemorySubCommand::GetStats as i128),
	);

	let mem_cbor = to_vec(&Value::Map(mem_req)).map_err(|e| {
		log::error!("Failed to encode Memory Stats CBOR: {}", e);
		PFError::Io(format!("CBOR encode error: {}", e))
	})?;

	// FIX: Prepend the Vendor Command ID (0x06 for Memory) to the payload
	// The firmware expects: [VendorCmdByte] [CBOR Map]
	let mut mem_payload = vec![VendorCommand::Memory as u8];
	mem_payload.extend(mem_cbor);

	log::debug!("Sending Memory Stats command...");
	let mem_res = transport
		.send_cbor(CTAP_VENDOR_CBOR_CMD, &mem_payload)
		.unwrap_or_else(|e| {
			log::warn!("Failed to fetch memory stats (Vendor Cmd): {}", e);
			Vec::new()
		});

	let mem_map: BTreeMap<i128, i128> = if !mem_res.is_empty() {
		from_slice(&mem_res).unwrap_or_else(|e| {
			log::error!("Failed to parse Memory Stats CBOR response: {}", e);
			BTreeMap::new()
		})
	} else {
		BTreeMap::new()
	};

	let used = mem_map
		.get(&(MemoryResponseKey::UsedSpace as i128))
		.cloned()
		.unwrap_or(0) as u32;
	let total = mem_map
		.get(&(MemoryResponseKey::TotalSpace as i128))
		.cloned()
		.unwrap_or(0) as u32;

	log::debug!(
		"Memory Stats: Used={}KB, Total={}KB",
		used / 1024,
		total / 1024
	);

	// --- 3. Get Physical Config ---
	log::debug!("Preparing Physical Config vendor command...");

	// FIX: Only arguments in CBOR map
	let mut phy_params = BTreeMap::new();
	phy_params.insert(
		Value::Integer(1), // Sub-command key
		Value::Integer(PhysicalOptionsSubCommand::GetOptions as i128),
	);

	// Note: The previous code nested this inside another map with key 2.
	// Based on cbor_vendor.c, we usually just send the sub-command params directly
	// or wrapped depending on the specific vendor command logic.
	// For 'PhysicalOptions', looking at cbor_vendor.c, it expects a map where key 1 is subcommand.
	// So the map we built above `phy_params` ( {1: GetOptions} ) is correct as the top-level CBOR.

	let phy_cbor = to_vec(&Value::Map(phy_params)).map_err(|e| {
		log::error!("Failed to encode Physical Config CBOR: {}", e);
		PFError::Io(format!("CBOR encode error: {}", e))
	})?;

	// FIX: Prepend Vendor Command ID (0x05 for PhysicalOptions)
	let mut phy_payload = vec![VendorCommand::PhysicalOptions as u8];
	phy_payload.extend(phy_cbor);

	log::debug!("Sending Physical Config command...");
	let phy_res = transport
		.send_cbor(CTAP_VENDOR_CBOR_CMD, &phy_payload)
		.unwrap_or_else(|e| {
			log::warn!("Failed to fetch physical config (Vendor Cmd): {}", e);
			Vec::new()
		});

	let mut config = AppConfig {
		vid: format!("{:04X}", transport.vid),
		pid: format!("{:04X}", transport.pid),
		product_name: transport.product_name.clone(),
		..Default::default()
	};

	if let Ok(Value::Map(m)) = from_slice(&phy_res) {
		log::debug!("Parsed Physical Config map successfully");
		// These keys might need adjustment based on exact firmware response structure
		// usually they are integer keys in CBOR, but if your firmware returns text keys:
		if let Some(Value::Integer(v)) = m.get(&Value::Text("gpio".into())) {
			config.led_gpio = *v as u8;
		}
		if let Some(Value::Integer(v)) = m.get(&Value::Text("brightness".into())) {
			config.led_brightness = *v as u8;
		}
	} else if !phy_res.is_empty() {
		log::warn!("Physical config response was not a valid CBOR map or empty");
	}

	log::info!("Successfully read all device details.");

	Ok(FullDeviceStatus {
		info: DeviceInfo {
			serial: "?".to_string(), // Serial number is not available through fido. Previous code was using AAGUID as serial but it is too long to display in place of serial it is already displayed somewhere else.
			flash_used: used / 1024,
			flash_total: total / 1024,
			firmware_version: fw_version,
		},
		config,
		secure_boot: false,
		secure_lock: false,
		method: "FIDO".to_string(),
	})
}

pub fn write_config(config: AppConfigInput, pin: Option<String>) -> Result<String, PFError> {
	log::info!("Starting FIDO write_config...");

	let pin_val = pin.as_deref().ok_or_else(|| {
		log::error!(
			"A security PIN is required to be set to change the configuration in fido mode"
		);
		PFError::Device(
			"A security PIN is required to be set to change the configuration in fido mode".into(),
		)
	})?;

	// 1. Obtain PIN token using the library handle
	let pin_token = {
		let cfg = Cfg::init();
		let device = FidoKeyHidFactory::create(&cfg)
			.map_err(|e| PFError::Device(format!("Could not connect to FIDO device: {:?}", e)))?;

		use ctap_hid_fido2::fidokey::pin::Permission;
		// Try to obtain a token with AuthenticatorConfiguration permission (CTAP 2.1)
		match device
			.get_pinuv_auth_token_with_permission(pin_val, Permission::AuthenticatorConfiguration)
		{
			Ok(token) => {
				log::debug!("Successfully obtained PIN token with ACFG permission.");
				token.key
			}
			Err(e) => {
				log::warn!(
					"Failed to get PIN token with ACFG permission (Error: {:?}). Falling back to standard token.",
					e
				);
				// Fallback to standard PIN token (Subcommand 0x05)
				let token = device.get_pin_token(pin_val).map_err(|e2| {
					log::error!("Failed to obtain even a standard PIN token: {:?}", e2);
					PFError::Device(format!("PIN token acquisition failed: {:?}", e2))
				})?;
				log::debug!("Successfully obtained standard PIN token (fallback).");
				token.key
			}
		}
		// Library handle 'device' is dropped here, closing the HID session.
	};

	// 2. Open custom HidTransport and send vendor commands using the token
	let transport = HidTransport::open().map_err(|e| {
		log::error!("Failed to open HID transport: {}", e);
		PFError::Device(format!("Could not open HID transport: {}", e))
	})?;

	// VID/PID config
	if let (Some(vid_str), Some(pid_str)) = (&config.vid, &config.pid) {
		let vid = u16::from_str_radix(vid_str, 16).map_err(|e| PFError::Io(e.to_string()))?;
		let pid = u16::from_str_radix(pid_str, 16).map_err(|e| PFError::Io(e.to_string()))?;
		let vidpid = ((vid as u32) << 16) | (pid as u32);
		transport.send_vendor_config(
			&pin_token,
			VendorConfigCommand::PhysicalVidPid,
			Value::Integer(vidpid as i128),
		)?;
	}

	// LED GPIO config
	if let Some(gpio) = config.led_gpio {
		transport.send_vendor_config(
			&pin_token,
			VendorConfigCommand::PhysicalLedGpio,
			Value::Integer(gpio as i128),
		)?;
	}

	// LED brightness config
	if let Some(brightness) = config.led_brightness {
		transport.send_vendor_config(
			&pin_token,
			VendorConfigCommand::PhysicalLedBrightness,
			Value::Integer(brightness as i128),
		)?;
	}

	// Options config
	let mut opts = 0u16;
	if config.led_dimmable.unwrap_or(false) {
		opts |= 0x02; // PHY_OPT_DIMM
	}
	if !config.power_cycle_on_reset.unwrap_or(true) {
		opts |= 0x04; // PHY_OPT_DISABLE_POWER_RESET
	}
	if config.led_steady.unwrap_or(false) {
		opts |= 0x08; // PHY_OPT_LED_STEADY
	}
	// Touch_timeout config
	if let Some(timeout) = config.touch_timeout {
		// In the firmware's phy_data, touch_timeout is often part of opts or separate.
		// Looking at the previous code, it was separate (0x08).
		// However, in vendor configuration, we usually send parameters individually.
		transport
			.send_vendor_config(
				&pin_token,
				VendorConfigCommand::PhysicalOptions, // Assuming there's a command for it or it's in opts
				Value::Integer(timeout as i128),      // Wait, let's check VendorConfigCommand again
			)
			.ok(); // If it fails, maybe it's not supported as a standalone vendor cmd
	}

	transport.send_vendor_config(
		&pin_token,
		VendorConfigCommand::PhysicalOptions,
		Value::Integer(opts as i128),
	)?;

	// ToDo : Product name configuration is not implemented in pico-fido firmware (cbor_config.c)?

	Ok(
		"Configuration updated successfully! Unplug and re-plug the device to apply VID/PID changes."
			.to_string(),
	)
}
