//! FIDO2 / CTAP2 protocol implementation for pico-fido and RS-Key firmware.
//!
//! ```text
//! fido/
//! ├── mod.rs       — high-level FIDO2 operations (info, PIN, credentials, config)
//! ├── constants.rs — CTAP2 command codes, CBOR map keys, COSE algorithms, bitflags
//! └── ops.rs       — FidoOperations trait impl (CTAPHID framing, PIN, credential mgmt)
//! ```
//!
//! # Architecture
//!
//! Communication flows top-down:
//!
//! ```text
//!  io::read_device_details()
//!       │
//!       ▼
//!  fido::read_device_details()     ← this file
//!       │
//!       ▼
//!  HidTransport::open()            ← transport/fido.rs
//!       │
//!       ▼
//!  USB HID (CTAPHID protocol)
//! ```
//!
//! [`constants`] is imported by both `mod.rs` and `ops.rs` and should be the
//! single source of truth for every CTAP2-defined byte value. If you need to
//! add a new command, sub-command, or CBOR key, put it there.
//!
//! [`ops`] implements the [`FidoOperations`] trait on [`HidTransport`],
//! owning the raw byte-level exchange: channel ID negotiation, packet
//! framing (init + continuation packets), PIN token acquisition, ECDH key
//! agreement, and CBOR serialization.
//!
//! This module contains the public functions called from [`super::io`].
//! Each function opens an [`HidTransport`], performs the CTAP2 operation,
//! and parses the CBOR response into the structs defined in [`super::types`].
//!
//! # Vendor extensions
//!
//! Pico-fido firmware exposes vendor-specific CTAP commands (`0xC1`, `0xC2`)
//! for hardware configuration (VID/PID, LED, memory stats). These are handled
//! through [`send_vendor_config`](ops::FidoOperations::send_vendor_config) and the
//! [`VendorConfigCommand`] enum in constants. Legacy firmware (≤7.2) uses a
//! different physical-options encoding; see `AnyFirmware::supports_legacy_fido_hardware_config`.
//!
//! # Adding a new FIDO2 operation
//!
//! 1. Add any new command/sub-command enums to [`constants`].
//! 2. Implement the CBOR encoding and transport call in [`ops`] (if it
//!    requires new framing or PIN token logic).
//! 3. Add the high-level function in this file, following the pattern:
//!    open transport → build CBOR payload → send → parse response → return.
//! 4. Expose it through [`super::io`].

pub mod audit;
pub mod backup;
pub mod constants;
pub mod ops;
use crate::hal::transport::fido::{CTAPHID_CBOR, HidTransport};

use crate::{
    error::PFError,
    hal::{
        firmwares::AnyFirmware,
        types::{
            AppConfig, AppConfigInput, DeviceInfo, DeviceMethod, FidoDeviceInfo, FirmwareType,
            FullDeviceStatus, LKONE_AAGUID, LedStatusConfig, PICOFIDO_AAGUID, RSKEY_AAGUID,
            StoredCredential,
        },
    },
};
use base64::{Engine as _, engine::general_purpose};
use constants::*;
use ops::FidoOperations;

use serde_cbor_2::{Value, from_slice, to_vec};
use std::collections::BTreeMap;

const LEGACY_PHY_OPT_DIMMABLE: u16 = 0x02;
const LEGACY_PHY_OPT_DISABLE_POWER_RESET: u16 = 0x04;
const LEGACY_PHY_OPT_LED_STEADY: u16 = 0x08;

// PHY tag constants for RS-Key FIDO config (mirrors rescue PhyTag)
const RSKEY_PHY_TAG_VIDPID: u8 = 0x00;
const RSKEY_PHY_TAG_LED_GPIO: u8 = 0x04;
const RSKEY_PHY_TAG_LED_BRIGHTNESS: u8 = 0x05;
const RSKEY_PHY_TAG_OPTS: u8 = 0x06;
const RSKEY_PHY_TAG_PRESENCE_TIMEOUT: u8 = 0x08;
const RSKEY_PHY_TAG_USB_PRODUCT: u8 = 0x09;
const RSKEY_PHY_TAG_CURVES: u8 = 0x0A;
const RSKEY_PHY_TAG_ENABLED_USB_ITF: u8 = 0x0B;
const RSKEY_PHY_TAG_LED_DRIVER: u8 = 0x0C;
const RSKEY_PHY_TAG_LED_ORDER: u8 = 0x0D;
const RSKEY_PHY_TAG_LED_NUM: u8 = 0x0E;
const RSKEY_PHY_TAG_USB_MANUFACTURER: u8 = 0x0F;

/// The boot-resolved *effective* phy values a device reports in CONFIG_READ key 2
/// (build defaults or overrides) — used to show real numbers instead of a bare
/// "firmware default". All `None` on firmware that predates the field / a headless
/// build.
#[derive(Default, Clone, Copy)]
pub struct EffectivePhy {
    pub led_gpio: Option<u8>,
    pub led_driver: Option<u8>,
    pub touch_timeout: Option<u8>,
}

const RSKEY_OPT_DIMMABLE: u16 = 0x02;
const RSKEY_OPT_DISABLE_POWER_RESET: u16 = 0x04;
const RSKEY_OPT_LED_STEADY: u16 = 0x08;

// Fido functions that require pin:

pub(crate) fn get_fido_info() -> Result<FidoDeviceInfo, String> {
    log::info!("Reading FIDO device info via custom GetInfo...");

    let transport =
        HidTransport::open().map_err(|e| format!("Could not open HID transport: {}", e))?;

    let info_payload = [CtapCommand::GetInfo as u8];
    let info_response = transport
        .send_cbor(CTAPHID_CBOR, &info_payload)
        .map_err(|e| format!("GetInfo CTAP command failed: {}", e))?;

    let info_value: Value =
        from_slice(&info_response).map_err(|e| format!("Failed to parse GetInfo CBOR: {}", e))?;

    parse_fido_get_info(&info_value)
}

fn format_firmware_version(raw: i128) -> String {
    if raw > 0xFFFF {
        format!(
            "{}.{}.{}",
            (raw >> 16) & 0xFF,
            (raw >> 8) & 0xFF,
            raw & 0xFF
        )
    } else {
        format!("{}.{}", (raw >> 8) & 0xFF, raw & 0xFF)
    }
}

fn parse_fido_get_info(info_value: &Value) -> Result<FidoDeviceInfo, String> {
    let map = match info_value {
        Value::Map(m) => m,
        _ => return Err("GetInfo response is not a CBOR map".into()),
    };

    let mut versions = Vec::new();
    let mut extensions = Vec::new();
    let mut aaguid = String::from("Unknown");
    let mut options = std::collections::HashMap::new();
    let mut max_msg_size: i128 = 0;
    let mut pin_protocols = Vec::new();
    let mut remaining_discoverable_credentials: Option<i128> = None;
    let mut min_pin_length: i128 = 0;
    let mut firmware_version_raw: i128 = 0;
    let mut vendor_config_commands = Vec::new();
    let mut certifications = std::collections::HashMap::new();
    let mut max_credential_count_in_list = None;
    let mut max_credential_id_length = None;
    let mut algorithms = Vec::new();
    let mut max_serialized_large_blob_array = None;
    let mut force_pin_change = None;
    let mut max_cred_blob_length = None;

    for (key, val) in map {
        let key_num = match key {
            Value::Integer(n) => *n,
            _ => continue,
        };

        match key_num {
            // 0x01: versions (array of strings)
            0x01 => {
                if let Value::Array(arr) = val {
                    for entry in arr {
                        if let Value::Text(version_str) = entry {
                            versions.push(version_str.clone());
                        }
                    }
                    log::info!("Device versions (0x01): {:?}", versions);
                }
            }
            // 0x02: extensions (array of strings)
            0x02 => {
                if let Value::Array(arr) = val {
                    for entry in arr {
                        if let Value::Text(ext_str) = entry {
                            extensions.push(ext_str.clone());
                        }
                    }
                    log::info!("Device extensions (0x02): {:?}", extensions);
                }
            }
            // 0x03: aaguid (byte string)
            0x03 => {
                if let Value::Bytes(g) = val {
                    aaguid = hex::encode_upper(g);
                    log::info!("Device aaguid (0x03): {}", aaguid);
                }
            }
            // 0x04: options (map of string -> bool)
            0x04 => {
                if let Value::Map(opts_map) = val {
                    for (option_key, option_value) in opts_map {
                        if let (Value::Text(name), Value::Bool(enabled)) =
                            (option_key, option_value)
                        {
                            options.insert(name.clone(), *enabled);
                        }
                    }
                    log::info!("Device options (0x04): {:?}", options);
                }
            }
            // 0x05: maxMsgSize
            0x05 => {
                if let Value::Integer(raw_size) = val {
                    max_msg_size = *raw_size;
                    log::info!("Device maxMsgSize (0x05): {}", max_msg_size);
                }
            }
            // 0x06: pinUvAuthProtocols (array of unsigned)
            0x06 => {
                if let Value::Array(arr) = val {
                    for entry in arr {
                        if let Value::Integer(protocol) = entry {
                            pin_protocols.push(*protocol as u32);
                        }
                    }
                    log::info!("Device pinUvAuthProtocols (0x06): {:?}", pin_protocols);
                }
            }
            // 0x07: maxCredentialCountInList
            0x07 => {
                if let Value::Integer(count) = val {
                    max_credential_count_in_list = Some(*count);
                    log::info!(
                        "Device maxCredentialCountInList (0x07): {}",
                        max_credential_count_in_list.unwrap()
                    );
                }
            }
            // 0x08: maxCredentialIdLength
            0x08 => {
                if let Value::Integer(max_len) = val {
                    max_credential_id_length = Some(*max_len);
                    log::info!(
                        "Device maxCredentialIdLength (0x08): {}",
                        max_credential_id_length.unwrap()
                    );
                }
            }
            // 0x0A: algorithms
            0x0A => {
                if let Value::Array(arr) = val {
                    for alg_entry in arr {
                        if let Value::Map(alg_map) = alg_entry
                            && let Some(Value::Integer(alg_id)) =
                                alg_map.get(&Value::Text("alg".into()))
                        {
                            if let Some(alg) = CoseAlgorithm::from_i128(*alg_id) {
                                algorithms.push(alg.to_string());
                            } else {
                                algorithms.push(format!("Unknown ({})", alg_id));
                            }
                        }
                    }
                    log::info!("Device algorithms (0x0A): {:?}", algorithms);
                }
            }
            // 0x0B: maxSerializedLargeBlobArray
            0x0B => {
                if let Value::Integer(blob_max) = val {
                    max_serialized_large_blob_array = Some(*blob_max);
                    log::info!(
                        "Device maxSerializedLargeBlobArray (0x0B): {}",
                        max_serialized_large_blob_array.unwrap()
                    );
                }
            }
            // 0x0C: forcePinChange
            0x0C => {
                if let Value::Bool(force) = val {
                    force_pin_change = Some(*force);
                    log::info!(
                        "Device forcePinChange (0x0C): {}",
                        force_pin_change.unwrap()
                    );
                }
            }
            // 0x0D: minPINLength
            0x0D => {
                if let Value::Integer(min_len) = val {
                    min_pin_length = *min_len;
                    log::info!("Device minPINLength (0x0D): {}", min_pin_length);
                }
            }
            // 0x0E: firmwareVersion
            0x0E => {
                if let Value::Integer(fw_ver) = val {
                    firmware_version_raw = *fw_ver;
                    log::info!("Device firmwareVersion (0x0E): {}", firmware_version_raw);
                }
            }
            // 0x0F: maxCredBlobLength
            0x0F => {
                if let Value::Integer(cred_blob) = val {
                    max_cred_blob_length = Some(*cred_blob);
                    log::info!(
                        "Device maxCredBlobLength (0x0F): {}",
                        max_cred_blob_length.unwrap()
                    );
                }
            }
            // Some firmware versions used 0x13 here. Pico-FIDO 7.6 reports
            // vendorPrototypeConfigCommands at 0x15.
            0x13 => {
                parse_get_info_extension_list(val, &mut vendor_config_commands, &mut certifications)
            }
            // 0x14: remainingDiscoverableCredentials
            0x14 => {
                if let Value::Integer(remaining) = val {
                    remaining_discoverable_credentials = Some(*remaining);
                    log::info!(
                        "Device remainingDiscoverableCredentials (0x14): {}",
                        remaining_discoverable_credentials.unwrap()
                    );
                }
            }
            // Pico-FIDO 7.6 uses 0x15 for vendorPrototypeConfigCommands.
            0x15 => {
                parse_get_info_extension_list(val, &mut vendor_config_commands, &mut certifications)
            }
            // 0x1B/0x1C are Pico-FIDO PIN policy extensions.
            0x1B | 0x1C => {
                log::trace!("GetInfo Pico-FIDO extension key 0x{:02X} skipped", key_num);
            }
            // All other known keys (0x10-0x12, 0x16) - silently skip
            0x10..=0x12 | 0x16 => {
                log::trace!("GetInfo key 0x{:02X} skipped", key_num);
            }
            // Unknown keys
            _ => {
                log::debug!("GetInfo: unknown key 0x{:02X}: {:?}", key_num, val);
            }
        }
    }

    let firmware_version = format_firmware_version(firmware_version_raw);

    log::info!(
        "FIDO GetInfo parsed: {} versions, {} extensions, AAGUID={}, FW={}",
        versions.len(),
        extensions.len(),
        aaguid,
        firmware_version
    );

    Ok(FidoDeviceInfo {
        versions,
        extensions,
        aaguid,
        options,
        max_msg_size,
        pin_protocols,
        remaining_discoverable_credentials,
        min_pin_length,
        firmware_version,
        vendor_config_commands,
        certifications,
        max_credential_count_in_list,
        max_credential_id_length,
        algorithms,
        max_serialized_large_blob_array,
        force_pin_change,
        max_cred_blob_length,
    })
}

fn parse_get_info_extension_list(
    val: &Value,
    vendor_config_commands: &mut Vec<String>,
    certifications: &mut std::collections::HashMap<String, bool>,
) {
    match val {
        Value::Array(arr) => {
            for v in arr {
                if let Value::Integer(n) = v {
                    let cmd_id = *n as u64;
                    let cmd_name = VendorConfigCommand::from_u64(cmd_id)
                        .map(|c| format!("{}", c))
                        .unwrap_or_else(|| format!("0x{:016X}", cmd_id));
                    if !vendor_config_commands.contains(&cmd_name) {
                        vendor_config_commands.push(cmd_name);
                    }
                }
            }
            log::info!(
                "Device supports {} vendor config commands: {:?}",
                vendor_config_commands.len(),
                vendor_config_commands
            );
        }
        Value::Map(cert_map) => {
            for (k, v) in cert_map {
                if let (Value::Text(name), Value::Bool(enabled)) = (k, v) {
                    let display_name = FidoCertification::from_str(name)
                        .map(|c| format!("{}", c))
                        .unwrap_or_else(|| name.clone());
                    certifications.insert(display_name, *enabled);
                }
            }
            log::info!("Device certifications: {:?}", certifications);
        }
        _ => {
            log::trace!("Unsupported GetInfo extension list shape: {:?}", val);
        }
    }
}

pub(crate) fn change_fido_pin(
    current_pin: Option<String>,
    new_pin: String,
) -> Result<String, String> {
    log::info!("Starting change_fido_pin (custom implementation)...");

    let transport =
        HidTransport::open().map_err(|e| format!("Could not open HID transport: {}", e))?;

    match current_pin {
        Some(old) => {
            transport
                .change_pin(&old, &new_pin)
                .map_err(|e| e.to_string())?;
            Ok("PIN Changed Successfully".into())
        }
        Option::None => {
            transport.set_pin(&new_pin).map_err(|e| e.to_string())?;
            Ok("PIN Set Successfully".into())
        }
    }
}

pub(crate) fn set_min_pin_length(
    current_pin: String,
    min_pin_length: u8,
) -> Result<String, String> {
    log::info!("Starting set_min_pin_length (custom implementation)...");

    // 1. Open custom HidTransport
    let transport =
        HidTransport::open().map_err(|e| format!("Could not open HID transport: {}", e))?;

    // 2. Obtain PIN token using the custom implementation
    let pin_token = transport
        .get_pin_token_with_permission(
            &current_pin,
            PinUvAuthTokenPermissions::AUTHENTICATOR_CONFIG,
            None,
        )
        .map_err(|e| {
            let err_str = e.to_string();
            log::error!("Failed to get PIN token with ACFG permission: {}", err_str);
            if err_str.contains("0x2B") {
                return "The device does not support FIDO 2.1 advanced configuration (Error 0x2B). Ensure your device firmware is up to date and supports this feature.".to_string();
            }
            format!("Failed to obtain PIN token: {}", err_str)
        })?;

    // 3. Send command using the token because ctap-hid-fido2 has a bug where it sends CBOR map keys out of order (0x01, 0x03, 0x04, 0x02) instead of the required ascending order (0x01, 0x02, 0x03, 0x04). The pico-fido firmware strictly requires ascending order.

    transport
        .send_config_set_min_pin_length(&pin_token, min_pin_length)
        .map_err(|e| format!("Failed to set minimum PIN length: {}", e))?;

    Ok(format!(
        "Minimum PIN length successfully set to {}",
        min_pin_length
    ))
}

pub(crate) fn get_credentials(pin: String) -> Result<Vec<StoredCredential>, String> {
    log::info!("Listing FIDO credentials via custom implementation...");

    let transport =
        HidTransport::open().map_err(|e| format!("Could not open HID transport: {}", e))?;

    let rps = transport
        .credential_management_enumerate_rps(&pin)
        .map_err(|e| format!("Failed to enumerate Relying Parties: {}", e))?;

    let mut all_credentials = Vec::new();

    for rp_res in rps {
        let rp_id = if let Value::Map(m) = &rp_res.rp {
            match m.get(&Value::Text("id".into())) {
                Some(Value::Text(s)) => s.clone(),
                _ => "Unknown".to_string(),
            }
        } else {
            "Unknown".to_string()
        };

        let rp_name = if let Value::Map(m) = &rp_res.rp {
            match m.get(&Value::Text("name".into())) {
                Some(Value::Text(s)) => s.clone(),
                _ => rp_id.clone(),
            }
        } else {
            rp_id.clone()
        };

        log::debug!("Enumerating credentials for RP: {}", rp_id);

        let creds = transport
            .credential_management_enumerate_credentials(&pin, &rp_res.rp_id_hash)
            .map_err(|e| format!("Failed to enumerate credentials for RP {}: {}", rp_id, e))?;

        for cred in creds {
            let mut stored_cred = StoredCredential {
                credential_id: "".to_string(),
                rp_id: rp_id.clone(),
                rp_name: rp_name.clone(),
                user_name: "".to_string(),
                user_display_name: "".to_string(),
                user_id: "".to_string(),
            };

            // Parse User Map
            if let Value::Map(m) = &cred.user {
                if let Some(Value::Text(s)) = m.get(&Value::Text("name".into())) {
                    stored_cred.user_name = s.clone();
                }
                if let Some(Value::Text(s)) = m.get(&Value::Text("displayName".into())) {
                    stored_cred.user_display_name = s.clone();
                }
                if let Some(Value::Bytes(b)) = m.get(&Value::Text("id".into())) {
                    stored_cred.user_id = hex::encode(b);
                }
            }

            // Parse Credential ID Descriptor
            if let Value::Map(m) = &cred.credential_id
                && let Some(Value::Bytes(b)) = m.get(&Value::Text("id".into()))
            {
                stored_cred.credential_id = hex::encode(b);
            }

            all_credentials.push(stored_cred);
        }
    }

    Ok(all_credentials)
}

pub(crate) fn delete_credential(pin: String, credential_id_hex: String) -> Result<String, String> {
    log::info!("Deleting FIDO credential via custom implementation...");

    let transport =
        HidTransport::open().map_err(|e| format!("Could not open HID transport: {}", e))?;

    let cred_id_bytes = hex::decode(&credential_id_hex)
        .map_err(|_| "Invalid Credential ID Hex string".to_string())?;

    // Create PublicKeyCredentialDescriptor map: { "type": "public-key", "id": <bytes> }
    let mut descriptor = BTreeMap::new();
    descriptor.insert(Value::Text("type".into()), Value::Text("public-key".into()));
    descriptor.insert(Value::Text("id".into()), Value::Bytes(cred_id_bytes));

    transport
        .credential_management_delete_credential(&pin, Value::Map(descriptor))
        .map_err(|e| format!("Failed to delete credential: {}", e))?;

    Ok("Credential deleted successfully".into())
}

pub(crate) fn reset_device() -> Result<String, String> {
    log::info!("Starting FIDO authenticatorReset...");

    let transport =
        HidTransport::open().map_err(|e| format!("Could not open HID transport: {}", e))?;

    transport.reset().map_err(|e| {
        let error_text = e.to_string();
        if error_text.contains("0x30") {
            return "Reset not allowed. The device must be unplugged and re-plugged within 10 seconds before sending the reset command.".to_string();
        }
        if error_text.contains("0x27") {
            return "Reset declined. Touch was not confirmed on the device.".to_string();
        }
        format!("Reset failed: {}", error_text)
    })?;

    Ok("Device has been factory reset. All credentials and PIN have been erased.".to_string())
}

// Custom Fido functions ( works only with pico-fido firmware )

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct ManagementInfo {
    pub serial: Option<String>,
    pub firmware_version: Option<String>,
    pub usb_supported: Option<u16>,
    pub usb_enabled: Option<u16>,
    pub config_locked: Option<bool>,
}

pub(crate) fn read_rskey_management_info(
    transport: &HidTransport,
) -> Result<ManagementInfo, PFError> {
    // Enabled-apps info is NOT readable over the 0x41 CONFIG_READ path — the
    // firmware exposes only PHY/LED there and rejects DEV_CONF. Both RS-Key and
    // pico-fido answer the CTAPHID vendor command 0xC2 (logical READ_CONFIG)
    // with the Management DeviceInfo TLV, so read it there directly.
    read_management_info(transport)
        .ok_or_else(|| PFError::Device("Failed to read management info over FIDO".to_string()))
}

/// Read full device status (info, config, security flags) over the FIDO HID transport.
///
/// Opens a CTAPHID channel, performs `GetInfo`, reads hardware config
/// via vendor or standard config commands, and returns everything as
/// a [`FullDeviceStatus`].
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

    let fido_info = read_device_info(&transport)?;

    log::info!(
        "Device identified: AAGUID={}, FW={}",
        fido_info.aaguid,
        fido_info.firmware_version
    );

    let firmware_type = if fido_info.aaguid == RSKEY_AAGUID {
        FirmwareType::RSKey
    } else if fido_info.aaguid == PICOFIDO_AAGUID || fido_info.aaguid == LKONE_AAGUID {
        FirmwareType::PicoFido
    } else {
        FirmwareType::Unknown
    };
    let has_legacy_vendor =
        firmware_type == FirmwareType::PicoFido && probe_legacy_vendor_support(&transport);
    let firmware = AnyFirmware::new_with_legacy(
        firmware_type.clone(),
        &fido_info.firmware_version,
        has_legacy_vendor,
    );
    let supports_legacy_hardware_config = firmware.supports_legacy_fido_hardware_config();
    let management = read_management_info(&transport);
    let config = AppConfig {
        vid: format!("{:04X}", transport.vid),
        pid: format!("{:04X}", transport.pid),
        product_name: transport.product_name.clone(),
        // The effective iManufacturer string, so the field mirrors Product Name;
        // a phy 0x0F override (read below) then wins if one is set.
        manufacturer_name: transport.manufacturer.clone().unwrap_or_default(),
        ..Default::default()
    };
    let config = if firmware_type == FirmwareType::RSKey {
        // RS-Key uses 0x41 CONFIG_READ via CTAPHID_CBOR — not the
        // legacy 0xC1 vendor command. Always attempt it; pre-v0.3.1
        // firmware gracefully returns the config unchanged with a log.
        read_rskey_physical_config(&transport, config)
    } else if supports_legacy_hardware_config {
        read_legacy_physical_config(&transport, config)
    } else {
        config
    };
    let mem_stats = if supports_legacy_hardware_config {
        read_legacy_memory_stats(&transport).unwrap_or_else(|e| {
            log::info!("Legacy FIDO memory stats unavailable: {}", e);
            None
        })
    } else {
        None
    };

    log::info!("Successfully read all device details.");

    let firmware_version = if fido_info.firmware_version != "0.0" {
        fido_info.firmware_version
    } else {
        management
            .as_ref()
            .and_then(|info| info.firmware_version.clone())
            .unwrap_or_else(|| "Unknown".to_string())
    };

    Ok(FullDeviceStatus {
        info: DeviceInfo {
            serial: management
                .and_then(|info| info.serial)
                .unwrap_or_else(|| "Unknown".to_string()),
            flash_used: mem_stats.map(|(used, _)| used / 1024),
            flash_total: mem_stats.map(|(_, total)| total / 1024),
            firmware_version,
            bcd_device: Some(transport.release_number),
            manufacturer: transport.manufacturer.clone(),
            // Object count / chip size come from the Rescue FlashInfo, not FIDO.
            flash_files: None,
            flash_chip_size: None,
        },
        config,
        secure_boot: false,
        secure_lock: false,
        method: DeviceMethod::Fido,
        firmware_type: firmware.firmware_type(),
    })
}

fn read_device_info(transport: &HidTransport) -> Result<FidoDeviceInfo, PFError> {
    log::debug!("Sending GetInfo command (0x04)...");
    let info_payload = [CtapCommand::GetInfo as u8];
    let info_res = transport
        .send_cbor(CTAPHID_CBOR, &info_payload[..])
        .map_err(|e| {
            log::error!("GetInfo CTAP command failed: {}", e);
            PFError::Device(format!("GetInfo failed: {}", e))
        })?;

    log::debug!("GetInfo response received ({} bytes)", info_res.len());

    let info_val: Value = from_slice(&info_res).map_err(|e| {
        log::error!("Failed to parse GetInfo CBOR: {}", e);
        PFError::Io(e.to_string())
    })?;

    parse_fido_get_info(&info_val).map_err(PFError::Io)
}

fn read_management_info(transport: &HidTransport) -> Option<ManagementInfo> {
    // pico-fido v7.6 src/fido/cbor.c handles HID cmd 0xC2 as raw
    // man_get_config() TLV bytes, not as CTAP CBOR with a status byte.
    match transport.send_raw(CTAP_VENDOR_CONFIG_CMD, &[]) {
        Ok(raw) => {
            if raw.len() == 1 {
                log::info!(
                    "FIDO management config is not available (CTAP error 0x{:02X})",
                    raw[0]
                );
                None
            } else {
                match parse_management_info(&raw) {
                    Ok(info) => Some(info),
                    Err(e) => {
                        log::warn!("Failed to parse FIDO management config: {}", e);
                        None
                    }
                }
            }
        }
        Err(e) => {
            log::info!("FIDO management config is not available: {}", e);
            None
        }
    }
}

fn parse_management_info(raw: &[u8]) -> Result<ManagementInfo, String> {
    let data = if raw.first().map(|len| *len as usize) == Some(raw.len().saturating_sub(1)) {
        &raw[1..]
    } else {
        raw
    };

    let mut info = ManagementInfo::default();
    let mut i = 0;
    while i < data.len() {
        if i + 2 > data.len() {
            return Err("truncated management tag header".to_string());
        }

        let tag = data[i];
        let len = data[i + 1] as usize;
        i += 2;

        if i + len > data.len() {
            return Err(format!("truncated management tag 0x{:02X}", tag));
        }

        let field_data = &data[i..i + len];
        match tag {
            0x01 => info.usb_supported = parse_management_u16(field_data),
            0x02 if field_data.len() == 4 => {
                // TAG_SERIAL is the 8-digit Yubico decimal (already serial4-masked
                // by the firmware), big-endian — render it as ykman does, not hex.
                let n = u32::from_be_bytes([
                    field_data[0],
                    field_data[1],
                    field_data[2],
                    field_data[3],
                ]);
                info.serial = Some(n.to_string());
            }
            0x03 => info.usb_enabled = parse_management_u16(field_data),
            0x05 if field_data.len() >= 2 => {
                info.firmware_version = Some(format!("{}.{}", field_data[0], field_data[1]));
            }
            0x0A => {
                if let Some(locked) = field_data.first() {
                    info.config_locked = Some(*locked != 0);
                }
            }
            _ => {}
        }

        i += len;
    }

    Ok(info)
}

fn parse_management_u16(val: &[u8]) -> Option<u16> {
    match val {
        [single] => Some(*single as u16),
        [hi, lo] => Some(u16::from_be_bytes([*hi, *lo])),
        _ => None,
    }
}

fn read_legacy_memory_stats(transport: &HidTransport) -> Result<Option<(u32, u32)>, PFError> {
    let mut mem_req = BTreeMap::new();
    mem_req.insert(
        Value::Integer(1),
        Value::Integer(MemorySubCommand::GetStats as i128),
    );

    let mem_cbor = to_vec(&Value::Map(mem_req)).map_err(|e| PFError::Io(e.to_string()))?;
    let mut mem_payload = vec![VendorCommand::Memory as u8];
    mem_payload.extend(mem_cbor);

    let mem_res = transport.send_cbor(CTAP_VENDOR_CBOR_CMD, &mem_payload)?;
    if mem_res.is_empty() {
        return Ok(None);
    }

    let mem_map: BTreeMap<i128, i128> =
        from_slice(&mem_res).map_err(|e| PFError::Io(e.to_string()))?;
    let used = mem_map
        .get(&(MemoryResponseKey::UsedSpace as i128))
        .copied()
        .unwrap_or(0) as u32;
    let total = mem_map
        .get(&(MemoryResponseKey::TotalSpace as i128))
        .copied()
        .unwrap_or(0) as u32;

    Ok(Some((used, total)))
}

/// Probe whether the device supports the legacy VendorPrototype 0xFF handler
/// (the PicoForge CONFIG_PHY_* command set). Used to distinguish LK-ONE (and
/// old pico-fido ≤ v7.2) from pico-fido v7.4+ which removed this handler.
///
/// Sends a single PhysicalOptions read CBOR command — a cheap round-trip.
fn probe_legacy_vendor_support(transport: &HidTransport) -> bool {
    let mut params = BTreeMap::new();
    params.insert(
        Value::Integer(1),
        Value::Integer(PhysicalOptionsSubCommand::GetOptions as i128),
    );
    let Ok(phy_cbor) = to_vec(&Value::Map(params)) else {
        return false;
    };
    let mut phy_payload = vec![VendorCommand::PhysicalOptions as u8];
    phy_payload.extend(phy_cbor);
    match transport.send_cbor(CTAP_VENDOR_CBOR_CMD, &phy_payload) {
        Ok(resp) => from_slice::<Value>(&resp).is_ok(),
        Err(_) => false,
    }
}

fn read_legacy_physical_config(transport: &HidTransport, mut config: AppConfig) -> AppConfig {
    let mut phy_params = BTreeMap::new();
    phy_params.insert(
        Value::Integer(1),
        Value::Integer(PhysicalOptionsSubCommand::GetOptions as i128),
    );

    let Ok(phy_cbor) = to_vec(&Value::Map(phy_params)) else {
        return config;
    };

    let mut phy_payload = vec![VendorCommand::PhysicalOptions as u8];
    phy_payload.extend(phy_cbor);

    let Ok(phy_res) = transport.send_cbor(CTAP_VENDOR_CBOR_CMD, &phy_payload) else {
        return config;
    };

    let Ok(Value::Map(m)) = from_slice::<Value>(&phy_res) else {
        return config;
    };

    if let Some(Value::Integer(opts_raw)) = m.get(&Value::Integer(1)) {
        let opts = *opts_raw as u16;
        config.led_dimmable = opts & LEGACY_PHY_OPT_DIMMABLE != 0;
        config.power_cycle_on_reset = opts & LEGACY_PHY_OPT_DISABLE_POWER_RESET == 0;
        config.led_steady = opts & LEGACY_PHY_OPT_LED_STEADY != 0;
    }

    config
}

/// Read PHY configuration from an RS-Key via CTAPHID 0x41 CONFIG_READ.
///
/// Falls back to returning the unchanged config if the command is not
/// supported by the device (pre-v0.3.1 firmware) or the transport errors.
fn read_rskey_physical_config(transport: &HidTransport, mut config: AppConfig) -> AppConfig {
    // `rs_key_config_read` unwraps the CBOR `{1: blob}` and returns the raw
    // `EF_PHY` TLV record — a bare `TAG LEN VALUE` sequence, no length prefix.
    let Ok((data, effective)) = transport.rs_key_config_read(RSKEY_CFG_TARGET_PHY) else {
        log::info!("RS-Key FIDO config read unavailable (pre-v0.3.1 firmware or transport error)");
        return config;
    };
    config.effective_led_gpio = effective.led_gpio;
    config.effective_led_driver = effective.led_driver;
    config.effective_touch_timeout = effective.touch_timeout;

    let data = &data[..];
    let mut i = 0;
    while i + 1 < data.len() {
        if i + 2 > data.len() {
            break;
        }
        let tag_byte = data[i];
        let len = data[i + 1] as usize;
        i += 2;
        if i + len > data.len() {
            break;
        }
        let field_data = &data[i..i + len];

        match tag_byte {
            RSKEY_PHY_TAG_VIDPID if field_data.len() == 4 => {
                config.vid = format!("{:04X}", u16::from_be_bytes([field_data[0], field_data[1]]));
                config.pid = format!("{:04X}", u16::from_be_bytes([field_data[2], field_data[3]]));
            }
            RSKEY_PHY_TAG_LED_GPIO if !field_data.is_empty() => {
                config.led_gpio = Some(field_data[0]);
            }
            RSKEY_PHY_TAG_LED_BRIGHTNESS if !field_data.is_empty() => {
                config.led_brightness = Some(field_data[0]);
            }
            RSKEY_PHY_TAG_PRESENCE_TIMEOUT if !field_data.is_empty() => {
                config.touch_timeout = Some(field_data[0]);
            }
            RSKEY_PHY_TAG_USB_PRODUCT => {
                let product_str = std::str::from_utf8(field_data)
                    .unwrap_or("")
                    .trim_matches(char::from(0));
                config.product_name = product_str.to_string();
            }
            RSKEY_PHY_TAG_USB_MANUFACTURER => {
                let mfr = std::str::from_utf8(field_data)
                    .unwrap_or("")
                    .trim_matches(char::from(0));
                config.manufacturer_name = mfr.to_string();
            }
            RSKEY_PHY_TAG_OPTS if field_data.len() >= 2 => {
                let opts = u16::from_be_bytes([field_data[0], field_data[1]]);
                config.led_dimmable = opts & RSKEY_OPT_DIMMABLE != 0;
                config.power_cycle_on_reset = opts & RSKEY_OPT_DISABLE_POWER_RESET == 0;
                config.led_steady = opts & RSKEY_OPT_LED_STEADY != 0;
            }
            RSKEY_PHY_TAG_CURVES if field_data.len() == 4 => {
                let mask = u32::from_be_bytes([
                    field_data[0],
                    field_data[1],
                    field_data[2],
                    field_data[3],
                ]);
                config.raw_curves_mask = Some(mask);
                config.enable_secp256k1 = mask & 0x08 != 0; // SECP256K1, mirrors the CCID read
            }
            RSKEY_PHY_TAG_LED_DRIVER if !field_data.is_empty() => {
                config.led_driver = Some(field_data[0]);
            }
            RSKEY_PHY_TAG_LED_ORDER if !field_data.is_empty() => {
                config.led_order = Some(field_data[0]);
            }
            RSKEY_PHY_TAG_LED_NUM if !field_data.is_empty() => {
                config.led_num = Some(field_data[0]);
            }
            RSKEY_PHY_TAG_ENABLED_USB_ITF if !field_data.is_empty() => {
                config.enabled_usb_itf = Some(field_data[0]);
            }
            _ => {}
        }
        i += len;
    }

    config
}

/// Build a PHY TLV blob from `AppConfigInput` for RS-Key CONFIG_WRITE.
///
/// The TLV format matches the Rescue PHY record and is sent as-is
/// to the RS-Key 0x41 CONFIG_WRITE handler. Errors if a field can't fit
/// the record (e.g. an over-long product name).
fn build_rskey_phy_tlv(config: &AppConfigInput) -> Result<Vec<u8>, PFError> {
    let mut tlv = Vec::new();

    if let (Some(vid_str), Some(pid_str)) = (&config.vid, &config.pid)
        && let (Ok(vid), Ok(pid)) = (
            u16::from_str_radix(vid_str, 16),
            u16::from_str_radix(pid_str, 16),
        )
    {
        tlv.push(RSKEY_PHY_TAG_VIDPID);
        tlv.push(0x04);
        tlv.extend_from_slice(&vid.to_be_bytes());
        tlv.extend_from_slice(&pid.to_be_bytes());
    }

    if let Some(val) = config.led_gpio {
        tlv.push(RSKEY_PHY_TAG_LED_GPIO);
        tlv.push(0x01);
        tlv.push(val);
    }

    if let Some(val) = config.led_brightness {
        tlv.push(RSKEY_PHY_TAG_LED_BRIGHTNESS);
        tlv.push(0x01);
        tlv.push(val);
    }

    if let (Some(dim), Some(cycle), Some(steady)) = (
        config.led_dimmable,
        config.power_cycle_on_reset,
        config.led_steady,
    ) {
        let mut opts = 0u16;
        if dim {
            opts |= RSKEY_OPT_DIMMABLE;
        }
        if !cycle {
            opts |= RSKEY_OPT_DISABLE_POWER_RESET;
        }
        if steady {
            opts |= RSKEY_OPT_LED_STEADY;
        }
        tlv.push(RSKEY_PHY_TAG_OPTS);
        tlv.push(0x02);
        tlv.extend_from_slice(&opts.to_be_bytes());
    }

    if let Some(val) = config.touch_timeout {
        tlv.push(RSKEY_PHY_TAG_PRESENCE_TIMEOUT);
        tlv.push(0x01);
        tlv.push(val);
    }

    if let Some(name) = config.product_name.as_deref().filter(|n| !n.is_empty()) {
        let bytes = name.as_bytes();
        // Firmware accepts a product record of 1..=33 bytes (name + trailing NUL),
        // so the name must be <= 32 bytes — reject rather than emit a wrapped length.
        if bytes.len() + 1 > 33 {
            return Err(PFError::Device(
                "Product name too long (max 32 bytes).".into(),
            ));
        }
        tlv.push(RSKEY_PHY_TAG_USB_PRODUCT);
        tlv.push((bytes.len() + 1) as u8);
        tlv.extend_from_slice(bytes);
        tlv.push(0x00);
    }

    if let Some(mfr) = config
        .manufacturer_name
        .as_deref()
        .filter(|n| !n.is_empty())
    {
        let bytes = mfr.as_bytes();
        if bytes.len() + 1 > 33 {
            return Err(PFError::Device(
                "Manufacturer name too long (max 32 bytes).".into(),
            ));
        }
        tlv.push(RSKEY_PHY_TAG_USB_MANUFACTURER);
        tlv.push((bytes.len() + 1) as u8);
        tlv.extend_from_slice(bytes);
        tlv.push(0x00);
    }

    if config.enable_secp256k1.is_some() || config.raw_curves_mask.is_some() {
        let mut mask = config.raw_curves_mask.unwrap_or(0);
        if let Some(enabled) = config.enable_secp256k1 {
            if enabled {
                mask |= 0x08; // SECP256K1
            } else {
                mask &= !0x08u32;
            }
        }
        tlv.push(RSKEY_PHY_TAG_CURVES);
        tlv.push(0x04);
        tlv.extend_from_slice(&mask.to_be_bytes());
    }

    if let Some(val) = config.led_driver {
        tlv.push(RSKEY_PHY_TAG_LED_DRIVER);
        tlv.push(0x01);
        tlv.push(val);
    }

    if let Some(val) = config.led_order {
        tlv.push(RSKEY_PHY_TAG_LED_ORDER);
        tlv.push(0x01);
        tlv.push(val);
    }

    if let Some(val) = config.enabled_usb_itf {
        tlv.push(RSKEY_PHY_TAG_ENABLED_USB_ITF);
        tlv.push(0x01);
        tlv.push(val);
    }

    if let Some(val) = config.led_num {
        tlv.push(RSKEY_PHY_TAG_LED_NUM);
        tlv.push(0x01);
        tlv.push(val);
    }

    Ok(tlv)
}

/// Write PHY config to an RS-Key via CTAPHID 0x41 CONFIG_WRITE.
fn write_rskey_config(
    transport: &HidTransport,
    config: &AppConfigInput,
    pin: &str,
) -> Result<String, PFError> {
    let tlv = build_rskey_phy_tlv(config)?;
    if tlv.is_empty() {
        return Ok("No RS-Key configuration changes were needed.".to_string());
    }

    // Probe: CONFIG_READ (0x41 subcommand 0x0D) is ungated and, on success,
    // confirms the device supports the 0x41 CONFIG_WRITE/CONFIG_READ commands
    // (RS-Key v0.3.1+). Pre-v0.3.1 firmware rejects it with a CTAP error, which
    // surfaces here as `Err`. A supported device may legitimately return an
    // empty PHY blob, so success alone is the signal — never the blob length.
    transport
        .rs_key_config_read(RSKEY_CFG_TARGET_PHY)
        .map_err(|_| {
            PFError::Device(
                "This RS-Key firmware does not support FIDO configuration. \
                 Please use Rescue mode (CCID/PCSC) or update to RS-Key v0.3.1+."
                    .into(),
            )
        })?;

    let pin_token = transport
        .get_pin_token_with_permission(pin, PinUvAuthTokenPermissions::AUTHENTICATOR_CONFIG, None)
        .or_else(|e| {
            log::warn!(
                "Failed to get PIN token with ACFG permission: {}. Falling back.",
                e
            );
            transport.get_pin_token(pin)
        })?;

    transport.rs_key_config_write(&pin_token, RSKEY_CFG_TARGET_PHY, &tlv)?;

    // RS-Key v0.3.x (bcd 0x083A+) warm-reboots and re-enumerates itself after a
    // PHY write, unless the user disabled power-cycle-on-reset (OPT_DISABLE_
    // POWER_RESET), in which case a manual re-plug is still required.
    let msg = if config.power_cycle_on_reset == Some(false) {
        "Configuration updated successfully! Unplug and re-plug the device to apply changes."
    } else {
        "Configuration updated successfully! The device is re-enumerating to apply the changes."
    };
    Ok(msg.to_string())
}

/// Write device configuration over the FIDO HID transport.
///
/// Applies the partial [`AppConfigInput`] to the authenticator using
/// the appropriate vendor or standard config command path for the
/// detected firmware type. Requires the PIN for config write operations.
pub fn write_config(config: AppConfigInput, pin: Option<String>) -> Result<String, PFError> {
    log::info!("Starting FIDO write_config...");

    if is_empty_config_input(&config) {
        return Ok("No FIDO-only hardware configuration changes were needed.".to_string());
    }

    let transport = HidTransport::open().map_err(|e| {
        log::error!("Failed to open HID transport: {}", e);
        PFError::Device(format!("Could not open HID transport: {}", e))
    })?;
    let fido_info = read_device_info(&transport)?;
    let firmware_type = if fido_info.aaguid == RSKEY_AAGUID {
        FirmwareType::RSKey
    } else if fido_info.aaguid == PICOFIDO_AAGUID || fido_info.aaguid == LKONE_AAGUID {
        FirmwareType::PicoFido
    } else {
        FirmwareType::Unknown
    };
    let has_legacy_vendor =
        firmware_type == FirmwareType::PicoFido && probe_legacy_vendor_support(&transport);
    let firmware = AnyFirmware::new_with_legacy(
        firmware_type.clone(),
        &fido_info.firmware_version,
        has_legacy_vendor,
    );

    validate_fido_config_changes(&config, &firmware)?;

    let pin_val = pin.as_deref().ok_or_else(|| {
        log::error!("write_config called without any security PIN provided");
        PFError::Device(
            "A security PIN is required to change hardware configuration over FIDO.".into(),
        )
    })?;

    match firmware_type {
        FirmwareType::RSKey => write_rskey_config(&transport, &config, pin_val),
        FirmwareType::PicoFido if firmware.supports_fido_config_write() => {
            write_legacy_hardware_config(&transport, &config, pin_val)
        }
        _ => {
            log::error!(
                "write_config called on unsupported firmware (pico-fido requires rescue mode)"
            );
            Err(PFError::Device(
                "Hardware configuration over FIDO is not supported on this device. \
                 Use rescue mode (CCID/PCSC) instead."
                    .into(),
            ))
        }
    }
}

fn is_empty_config_input(config: &AppConfigInput) -> bool {
    config.vid.is_none()
        && config.pid.is_none()
        && config.product_name.is_none()
        && config.manufacturer_name.is_none()
        && config.led_gpio.is_none()
        && config.led_brightness.is_none()
        && config.touch_timeout.is_none()
        && config.led_driver.is_none()
        && config.led_dimmable.is_none()
        && config.power_cycle_on_reset.is_none()
        && config.led_steady.is_none()
        && config.enable_secp256k1.is_none()
        // RS-Key-only PHY fields — `build_rskey_phy_tlv` writes these, so a
        // change to any of them alone must not be treated as a no-op.
        && config.raw_curves_mask.is_none()
        && config.led_order.is_none()
        && config.enabled_usb_itf.is_none()
        && config.led_num.is_none()
}

fn validate_fido_config_changes(
    config: &AppConfigInput,
    firmware: &AnyFirmware,
) -> Result<(), PFError> {
    let allow_write = firmware.supports_legacy_fido_hardware_config()
        || firmware.supports_rs_key_vendor_command();
    if !allow_write {
        if config.vid.is_some()
            || config.pid.is_some()
            || config.product_name.is_some()
            || config.manufacturer_name.is_some()
            || config.led_gpio.is_some()
            || config.led_brightness.is_some()
            || config.touch_timeout.is_some()
            || config.led_driver.is_some()
            || config.led_dimmable.is_some()
            || config.power_cycle_on_reset.is_some()
            || config.led_steady.is_some()
            || config.enable_secp256k1.is_some()
        {
            return Err(PFError::Device(
                "This firmware does not support hardware configuration over FIDO. \
                 Use rescue mode for hardware changes."
                    .into(),
            ));
        }
        return Ok(());
    }

    // RS-Key: 0x41 CONFIG_WRITE supports the full PHY TLV — no field restrictions.
    Ok(())
}

fn write_legacy_hardware_config(
    transport: &HidTransport,
    config: &AppConfigInput,
    pin: &str,
) -> Result<String, PFError> {
    let get_fresh_token = || -> Result<Vec<u8>, PFError> {
        transport
            .get_pin_token_with_permission(
                pin,
                PinUvAuthTokenPermissions::AUTHENTICATOR_CONFIG,
                None,
            )
            .or_else(|e| {
                log::warn!(
                    "Failed to get PIN token with ACFG permission (Error: {:?}). Falling back to standard token.",
                    e
                );
                transport.get_pin_token(pin)
            })
    };

    if let (Some(vid_str), Some(pid_str)) = (&config.vid, &config.pid) {
        let vid = u16::from_str_radix(vid_str, 16).map_err(|e| PFError::Io(e.to_string()))?;
        let pid = u16::from_str_radix(pid_str, 16).map_err(|e| PFError::Io(e.to_string()))?;
        let vidpid = ((vid as u32) << 16) | (pid as u32);
        transport.send_vendor_config(
            &get_fresh_token()?,
            VendorConfigCommand::PhysicalVidPid,
            Value::Integer(vidpid as i128),
        )?;
    }

    if let Some(gpio) = config.led_gpio {
        transport.send_vendor_config(
            &get_fresh_token()?,
            VendorConfigCommand::PhysicalLedGpio,
            Value::Integer(gpio as i128),
        )?;
    }

    if let Some(brightness) = config.led_brightness {
        transport.send_vendor_config(
            &get_fresh_token()?,
            VendorConfigCommand::PhysicalLedBrightness,
            Value::Integer(brightness as i128),
        )?;
    }

    if config.led_dimmable.is_some()
        || config.power_cycle_on_reset.is_some()
        || config.led_steady.is_some()
    {
        let current_config = read_legacy_physical_config(transport, AppConfig::default());
        let mut opts = 0u16;
        if config.led_dimmable.unwrap_or(current_config.led_dimmable) {
            opts |= LEGACY_PHY_OPT_DIMMABLE;
        }
        if !config
            .power_cycle_on_reset
            .unwrap_or(current_config.power_cycle_on_reset)
        {
            opts |= LEGACY_PHY_OPT_DISABLE_POWER_RESET;
        }
        if config.led_steady.unwrap_or(current_config.led_steady) {
            opts |= LEGACY_PHY_OPT_LED_STEADY;
        }

        transport.send_vendor_config(
            &get_fresh_token()?,
            VendorConfigCommand::PhysicalOptions,
            Value::Integer(opts as i128),
        )?;
    }

    if config.touch_timeout.is_some()
        || config.led_driver.is_some()
        || config.enable_secp256k1.is_some()
    {
        log::warn!(
            "Legacy hardware config does not support touch_timeout, led_driver, or enable_secp256k1 \
             fields. These were silently ignored. If your device supports them, file a feature request."
        );
    }

    Ok("Configuration updated successfully! Unplug and re-plug the device to apply VID/PID changes.".to_string())
}

/// Parse raw bytes from a certificate file into DER format.
/// Accepts both PEM (ASCII-armored base64) and raw DER (binary) input.
fn parse_cert_bytes(data: Vec<u8>) -> Result<Vec<u8>, String> {
    if data.starts_with(b"-----") {
        let text = String::from_utf8(data)
            .map_err(|e| format!("Certificate file is not valid UTF-8: {}", e))?;
        let b64: String = text
            .lines()
            .map(str::trim)
            .filter(|l| !l.starts_with("-----") && !l.is_empty())
            .collect();
        general_purpose::STANDARD
            .decode(&b64)
            .map_err(|e| format!("Failed to decode PEM base64: {}", e))
    } else {
        Ok(data)
    }
}

/// Upload a certificate to the device's enterprise attestation slot.
///
/// Sends CTAP_CONFIG_EA_UPLOAD (0x66f2a674c29a8dcf / subcommand 0xFF) via
/// authenticatorConfig VendorPrototype. Accepts PEM or DER certificate files.
pub(crate) fn upload_enterprise_attestation_cert(
    pin: String,
    cert_path: String,
) -> Result<String, String> {
    log::info!("Reading certificate from: {}", cert_path);

    let raw = std::fs::read(&cert_path)
        .map_err(|e| format!("Cannot read certificate file \"{}\": {}", cert_path, e))?;

    let cert_der = parse_cert_bytes(raw)?;
    log::info!("Certificate parsed. Uploading to device...");

    let transport =
        HidTransport::open().map_err(|e| format!("Could not open HID transport: {}", e))?;

    let pin_token = transport
        .get_pin_token_with_permission(
            &pin,
            PinUvAuthTokenPermissions::AUTHENTICATOR_CONFIG,
            None,
        )
        .map_err(|e| {
            let error_text = e.to_string();
            if error_text.contains("0x2B") {
                return "Device does not support enterprise attestation (0x2B). Ensure firmware is up to date.".to_string();
            }
            format!("Failed to obtain PIN token: {}", error_text)
        })?;

    transport
        .send_vendor_config(
            &pin_token,
            VendorConfigCommand::EnterpriseAttestationUpload,
            Value::Bytes(cert_der),
        )
        .map_err(|e| format!("Failed to upload certificate: {}", e))?;

    log::info!("Enterprise attestation certificate uploaded successfully.");
    Ok("Enterprise attestation certificate uploaded successfully.".to_string())
}

pub(crate) fn enable_enterprise_attestation(pin: String) -> Result<String, String> {
    log::info!("Enabling enterprise attestation...");

    let transport =
        HidTransport::open().map_err(|e| format!("Could not open HID transport: {}", e))?;

    let pin_token = transport
        .get_pin_token_with_permission(
            &pin,
            PinUvAuthTokenPermissions::AUTHENTICATOR_CONFIG,
            None,
        )
        .map_err(|e| {
            let error_text = e.to_string();
            log::error!("Failed to get PIN token: {}", error_text);
            if error_text.contains("0x2B") {
                return "Device does not support enterprise attestation (0x2B). Ensure firmware is up to date.".to_string();
            }
            format!("Failed to obtain PIN token: {}", error_text)
        })?;

    transport
        .send_config_enable_ea(&pin_token)
        .map_err(|e| format!("Failed to enable enterprise attestation: {}", e))?;

    Ok("Enterprise attestation enabled successfully.".into())
}

/// Request a Certificate Signing Request (CSR) from the device.
pub(crate) fn get_enterprise_attestation_csr() -> Result<String, String> {
    log::info!("Requesting Attestation CSR from device...");

    let transport =
        HidTransport::open().map_err(|e| format!("Could not open HID transport: {}", e))?;

    let csr_der = transport
        .get_enterprise_attestation_csr()
        .map_err(|e| format!("Failed to retrieve CSR: {}", e))?;

    log::info!(
        "CSR retrieved ({} bytes). Converting to PEM...",
        csr_der.len()
    );

    // Base64-encode the DER bytes and wrap in PEM
    let b64 = general_purpose::STANDARD.encode(&csr_der);
    let wrapped: String = b64
        .as_bytes()
        .chunks(64)
        .map(|c| std::str::from_utf8(c).unwrap_or(""))
        .collect::<Vec<&str>>()
        .join("\n");

    let pem = format!(
        "-----BEGIN CERTIFICATE REQUEST-----\n{}\n-----END CERTIFICATE REQUEST-----\n",
        wrapped
    );

    Ok(pem)
}

// ── RS-Key audit journal (0x41 vendor AUDIT_READ / AUDIT_CHECKPOINT) ─────

/// Map a non-zero CTAP status from a gated vendor command to a message.
fn vendor_error(status: u8, op: &str) -> String {
    match status {
        0x36 => "device requires a PIN — enter it".to_string(),
        0x27 => {
            "denied — no touch within the window (press the button when the LED blinks)".to_string()
        }
        0x30 => format!("{op}: operation not allowed (already sealed, or no OTP DEVK provisioned)"),
        0x3D => "device is not locked".to_string(),
        other => format!("{op} failed: status 0x{other:02x}"),
    }
}

/// Require a successful vendor response and unwrap its CBOR map.
fn vendor_map(status: u8, map: Option<Value>, op: &str) -> Result<BTreeMap<Value, Value>, String> {
    if status != 0 {
        return Err(vendor_error(status, op));
    }
    match map {
        Some(Value::Map(m)) => Ok(m),
        _ => Err(format!("{op}: response is not a CBOR map")),
    }
}

fn m_int(m: &BTreeMap<Value, Value>, k: i128) -> Option<i128> {
    match m.get(&Value::Integer(k)) {
        Some(Value::Integer(n)) => Some(*n),
        _ => None,
    }
}

fn m_bytes(m: &BTreeMap<Value, Value>, k: i128) -> Option<Vec<u8>> {
    match m.get(&Value::Integer(k)) {
        Some(Value::Bytes(b)) => Some(b.clone()),
        _ => None,
    }
}

/// Coerce a CBOR field to bool (accepts a CBOR bool or a non-zero integer).
fn m_bool(m: &BTreeMap<Value, Value>, k: i128) -> bool {
    match m.get(&Value::Integer(k)) {
        Some(Value::Bool(b)) => *b,
        Some(Value::Integer(n)) => *n != 0,
        _ => false,
    }
}

/// Read AUDIT_READ into a journal window (gated by PIN, or a touch if `pin` is None).
fn read_journal(
    transport: &HidTransport,
    pin: Option<&str>,
) -> Result<audit::AuditJournal, String> {
    let (status, map) = transport
        .rs_key_vendor(RSKEY_VENDOR_AUDIT_READ, None, pin)
        .map_err(|e| e.to_string())?;
    let m = vendor_map(status, map, "audit read")?;
    let start = m_int(&m, 1).ok_or("audit read: missing start")? as u32;
    let seq_next = m_int(&m, 2).ok_or("audit read: missing seq_next")? as u32;
    let epoch: [u8; 32] = m_bytes(&m, 3)
        .ok_or("audit read: missing epoch")?
        .try_into()
        .map_err(|_| "audit read: epoch is not 32 bytes")?;
    let entries = m_bytes(&m, 4).ok_or("audit read: missing entries")?;
    audit::build_journal(start, seq_next, epoch, &entries)
}

/// Export the audit journal (PIN or touch gated).
pub(crate) fn audit_log(pin: Option<String>) -> Result<audit::AuditJournal, String> {
    let transport =
        HidTransport::open().map_err(|e| format!("Could not open HID transport: {}", e))?;
    read_journal(&transport, pin.as_deref())
}

/// Export the journal, then verify a fresh DEVK-signed checkpoint over it.
/// `expect_key` (16-hex fingerprint or full SEC1 pubkey hex) pins the identity.
pub(crate) fn audit_verify(
    pin: Option<String>,
    expect_key: Option<String>,
) -> Result<audit::AuditVerification, String> {
    let transport =
        HidTransport::open().map_err(|e| format!("Could not open HID transport: {}", e))?;
    let journal = read_journal(&transport, pin.as_deref())?;

    let mut challenge = [0u8; 16];
    ring::rand::SecureRandom::fill(&ring::rand::SystemRandom::new(), &mut challenge)
        .map_err(|_| "RNG failure".to_string())?;

    let mut params = BTreeMap::new();
    params.insert(Value::Integer(1), Value::Bytes(challenge.to_vec()));
    let (status, map) = transport
        .rs_key_vendor(
            RSKEY_VENDOR_AUDIT_CHECKPOINT,
            Some(Value::Map(params)),
            pin.as_deref(),
        )
        .map_err(|e| e.to_string())?;
    let m = vendor_map(status, map, "checkpoint")?;

    let head_signed = m_bytes(&m, 1).ok_or("checkpoint: missing head")?;
    let seq_signed = m_int(&m, 2).ok_or("checkpoint: missing seq")? as u32;
    let sig = m_bytes(&m, 3).ok_or("checkpoint: missing signature")?;
    let pubkey = m_bytes(&m, 4).ok_or("checkpoint: missing public key")?;

    let signature_ok =
        audit::verify_checkpoint(&head_signed, seq_signed, &sig, &pubkey, &challenge);
    let head_matches = head_signed == journal.head;
    let fingerprint = audit::fingerprint(&pubkey);
    let pubkey_hex = hex::encode(&pubkey);
    let expected_match = expect_key.map(|k| {
        let k = k.trim().to_lowercase();
        !k.is_empty() && (k == pubkey_hex || k == fingerprint)
    });

    Ok(audit::AuditVerification {
        journal,
        signature_ok,
        head_matches,
        pubkey_hex,
        fingerprint,
        seq_signed,
        signed_head_hex: hex::encode(&head_signed),
        signature_hex: hex::encode(&sig),
        expected_match,
    })
}

/// Whether the audit journal is currently on (ungated status query, no touch).
pub(crate) fn audit_status() -> Result<bool, String> {
    let transport =
        HidTransport::open().map_err(|e| format!("Could not open HID transport: {}", e))?;
    let mut params = BTreeMap::new();
    params.insert(Value::Integer(1), Value::Integer(2)); // target 2 = read-only status
    let (status, map) = transport
        .rs_key_vendor(RSKEY_VENDOR_AUDIT_CONFIG, Some(Value::Map(params)), None)
        .map_err(|e| e.to_string())?;
    let m = vendor_map(status, map, "audit status")?;
    Ok(m_bool(&m, 1))
}

/// Turn the audit journal on/off (PIN + touch); returns the resulting state.
/// Journalling is opt-in, so nothing is written to flash until it is enabled.
pub(crate) fn audit_set_enabled(on: bool, pin: Option<String>) -> Result<bool, String> {
    let transport =
        HidTransport::open().map_err(|e| format!("Could not open HID transport: {}", e))?;
    let mut params = BTreeMap::new();
    params.insert(Value::Integer(1), Value::Integer(if on { 1 } else { 0 }));
    let (status, map) = transport
        .rs_key_vendor(
            RSKEY_VENDOR_AUDIT_CONFIG,
            Some(Value::Map(params)),
            pin.as_deref(),
        )
        .map_err(|e| e.to_string())?;
    let m = vendor_map(status, map, "audit config")?;
    Ok(m_bool(&m, 1))
}

// ── RS-Key seed backup (0x41 vendor MSE / EXPORT / LOAD / FINALIZE / STATE) ──

/// Ephemeral ECDH (classical P-256) handshake → `(channel_key, aad)`.
///
/// Sends the host's P-256 public point as a COSE key; the device replies with
/// its point, and both sides derive the same ChaCha20-Poly1305 key. The device
/// falls back to this classical channel when no ML-KEM key is offered.
fn mse_handshake(transport: &HidTransport) -> Result<([u8; 32], Vec<u8>), String> {
    use ring::{agreement, rand::SystemRandom};

    let rng = SystemRandom::new();
    let priv_key = agreement::EphemeralPrivateKey::generate(&agreement::ECDH_P256, &rng)
        .map_err(|_| "ECDH keygen failed".to_string())?;
    let pub_bytes = priv_key
        .compute_public_key()
        .map_err(|_| "ECDH pubkey failed".to_string())?;
    let pk = pub_bytes.as_ref();
    if pk.len() != 65 {
        return Err("unexpected ECDH public key length".into());
    }

    // COSE_Key {1: EC2, 3: ECDH-ES+HKDF-256, -1: P-256, -2: x, -3: y}.
    let mut cose = BTreeMap::new();
    cose.insert(Value::Integer(1), Value::Integer(2));
    cose.insert(Value::Integer(3), Value::Integer(-25));
    cose.insert(Value::Integer(-1), Value::Integer(1));
    cose.insert(Value::Integer(-2), Value::Bytes(pk[1..33].to_vec()));
    cose.insert(Value::Integer(-3), Value::Bytes(pk[33..65].to_vec()));
    let mut subpara = BTreeMap::new();
    subpara.insert(Value::Integer(1), Value::Map(cose));

    let (status, map) = transport
        .rs_key_vendor(RSKEY_VENDOR_MSE, Some(Value::Map(subpara)), None)
        .map_err(|e| e.to_string())?;
    let m = vendor_map(status, map, "MSE")?;

    let dev = match m.get(&Value::Integer(1)) {
        Some(Value::Map(dm)) => dm,
        _ => return Err("MSE: no device key in response".into()),
    };
    let dx = m_bytes(dev, -2).ok_or("MSE: device key missing x")?;
    let dy = m_bytes(dev, -3).ok_or("MSE: device key missing y")?;

    let mut peer = Vec::with_capacity(65);
    peer.push(0x04);
    peer.extend_from_slice(&dx);
    peer.extend_from_slice(&dy);
    let aad = peer.clone(); // AAD = the device's uncompressed point.

    let peer_pub = agreement::UnparsedPublicKey::new(&agreement::ECDH_P256, &peer);
    let aad_kdf = aad.clone();
    let key = agreement::agree_ephemeral(priv_key, &peer_pub, |z| {
        backup::derive_channel_key(z, &aad_kdf)
    })
    .map_err(|_| "ECDH agreement failed".to_string())?;

    Ok((key, aad))
}

/// Read `{sealed, has_seed, locked, unlocked}` (ungated).
pub(crate) fn backup_status() -> Result<backup::BackupStatus, String> {
    let transport =
        HidTransport::open().map_err(|e| format!("Could not open HID transport: {}", e))?;
    let (status, map) = transport
        .rs_key_vendor(RSKEY_VENDOR_STATE, None, None)
        .map_err(|e| e.to_string())?;
    let m = vendor_map(status, map, "state")?;
    Ok(backup::BackupStatus {
        sealed: m_bool(&m, 1),
        has_seed: m_bool(&m, 2),
        locked: m_bool(&m, 3),
        unlocked: m_bool(&m, 4),
    })
}

/// Seal the one-time export window (touch-gated).
pub(crate) fn backup_finalize() -> Result<(), String> {
    let transport =
        HidTransport::open().map_err(|e| format!("Could not open HID transport: {}", e))?;
    let (status, _) = transport
        .rs_key_vendor(RSKEY_VENDOR_FINALIZE, None, None)
        .map_err(|e| e.to_string())?;
    if status != 0 {
        return Err(vendor_error(status, "finalize"));
    }
    Ok(())
}

/// Export the master seed as a 24-word BIP-39 phrase (PIN or touch gated).
pub(crate) fn backup_export(pin: Option<String>) -> Result<String, String> {
    let transport =
        HidTransport::open().map_err(|e| format!("Could not open HID transport: {}", e))?;
    let (key, aad) = mse_handshake(&transport)?;
    let (status, map) = transport
        .rs_key_vendor(RSKEY_VENDOR_EXPORT, None, pin.as_deref())
        .map_err(|e| e.to_string())?;
    let m = vendor_map(status, map, "export (already sealed?)")?;
    let enc = m_bytes(&m, 1).ok_or("export: missing sealed seed")?;
    let seed = backup::chacha_open(&key, &enc, &aad)?;
    let seed: [u8; 32] = seed
        .try_into()
        .map_err(|_| "exported seed is not 32 bytes".to_string())?;
    backup::seed_to_mnemonic(&seed)
}

/// Restore a seed from a 24-word BIP-39 phrase (PIN or touch gated).
pub(crate) fn backup_restore(pin: Option<String>, mnemonic: String) -> Result<(), String> {
    let seed = backup::mnemonic_to_seed(&mnemonic)?;
    let transport =
        HidTransport::open().map_err(|e| format!("Could not open HID transport: {}", e))?;
    let (key, aad) = mse_handshake(&transport)?;

    let mut nonce = [0u8; 12];
    ring::rand::SecureRandom::fill(&ring::rand::SystemRandom::new(), &mut nonce)
        .map_err(|_| "RNG failure".to_string())?;
    let blob = backup::chacha_seal(&key, &nonce, &seed, &aad)?;

    let mut params = BTreeMap::new();
    params.insert(Value::Integer(1), Value::Bytes(blob));
    let (status, _) = transport
        .rs_key_vendor(RSKEY_VENDOR_LOAD, Some(Value::Map(params)), pin.as_deref())
        .map_err(|e| e.to_string())?;
    if status != 0 {
        return Err(vendor_error(status, "restore"));
    }
    Ok(())
}

// ── RS-Key at-rest soft lock (0x41 UNLOCK + authConfig AUT_ENABLE/DISABLE) ──

/// MSE-wrap a 32-byte secret for the vendor channel: `nonce(12) ‖ ct‖tag`.
fn wrap_secret(transport: &HidTransport, secret: &[u8; 32]) -> Result<Vec<u8>, String> {
    let (key, aad) = mse_handshake(transport)?;
    let mut nonce = [0u8; 12];
    ring::rand::SecureRandom::fill(&ring::rand::SystemRandom::new(), &mut nonce)
        .map_err(|_| "RNG failure".to_string())?;
    backup::chacha_seal(&key, &nonce, secret, &aad)
}

/// Load the soft-locked seed into RAM for this power cycle (the lock key comes
/// as a BIP-39 phrase). Ungated over the 0x41 channel.
pub(crate) fn lock_unlock(mnemonic: String) -> Result<(), String> {
    let key = backup::mnemonic_to_seed(&mnemonic)?;
    let transport =
        HidTransport::open().map_err(|e| format!("Could not open HID transport: {}", e))?;
    let blob = wrap_secret(&transport, &key)?;
    let mut params = BTreeMap::new();
    params.insert(Value::Integer(1), Value::Bytes(blob));
    let (status, _) = transport
        .rs_key_vendor(RSKEY_VENDOR_UNLOCK, Some(Value::Map(params)), None)
        .map_err(|e| e.to_string())?;
    if status == 0x3D {
        return Err("device is not locked".into());
    }
    if status != 0 {
        return Err(vendor_error(status, "unlock (wrong key?)"));
    }
    Ok(())
}

/// Engage the lock: generate a random 32-byte key, wrap the seed under it, erase
/// the plaintext, and return the key as a 24-word phrase to record. PIN required.
pub(crate) fn lock_enable(pin: String) -> Result<String, String> {
    let mut key = [0u8; 32];
    ring::rand::SecureRandom::fill(&ring::rand::SystemRandom::new(), &mut key)
        .map_err(|_| "RNG failure".to_string())?;
    let transport =
        HidTransport::open().map_err(|e| format!("Could not open HID transport: {}", e))?;
    let blob = wrap_secret(&transport, &key)?;
    let token = transport
        .get_pin_token_with_permission(&pin, PinUvAuthTokenPermissions::AUTHENTICATOR_CONFIG, None)
        .map_err(|e| e.to_string())?;
    transport
        .authconfig_vendor(&token, RSKEY_AUT_ENABLE, Some(Value::Bytes(blob)))
        .map_err(|e| format!("engage lock failed: {e}"))?;
    backup::seed_to_mnemonic(&key)
}

/// Disable the lock: unlock with the phrase, then restore the plaintext seed.
/// PIN required.
pub(crate) fn lock_disable(pin: String, mnemonic: String) -> Result<(), String> {
    let key = backup::mnemonic_to_seed(&mnemonic)?;
    let transport =
        HidTransport::open().map_err(|e| format!("Could not open HID transport: {}", e))?;

    // Unlock first (loads the seed into RAM); tolerate "already unlocked".
    let blob = wrap_secret(&transport, &key)?;
    let mut params = BTreeMap::new();
    params.insert(Value::Integer(1), Value::Bytes(blob));
    let (status, _) = transport
        .rs_key_vendor(RSKEY_VENDOR_UNLOCK, Some(Value::Map(params)), None)
        .map_err(|e| e.to_string())?;
    if status != 0 && status != 0x3D {
        return Err(vendor_error(status, "unlock (wrong key?)"));
    }

    let token = transport
        .get_pin_token_with_permission(&pin, PinUvAuthTokenPermissions::AUTHENTICATOR_CONFIG, None)
        .map_err(|e| e.to_string())?;
    transport
        .authconfig_vendor(&token, RSKEY_AUT_DISABLE, None)
        .map_err(|e| format!("disable lock failed: {e}"))
}

// ── RS-Key org attestation (0x41 vendor ATT_STATE / ATT_CLEAR / ATT_IMPORT) ──

/// Org-attestation state.
#[derive(Debug, Clone)]
pub struct AttStatus {
    pub installed: bool,
    pub chain_hash: Option<String>,
}

/// Read `{installed, chain_hash}` (ungated).
pub(crate) fn att_status() -> Result<AttStatus, String> {
    let transport =
        HidTransport::open().map_err(|e| format!("Could not open HID transport: {}", e))?;
    let (status, map) = transport
        .rs_key_vendor(RSKEY_VENDOR_ATT_STATE, None, None)
        .map_err(|e| e.to_string())?;
    let m = vendor_map(status, map, "attestation status")?;
    let installed = m_bool(&m, 1);
    let chain_hash = if installed {
        m_bytes(&m, 2).map(hex::encode)
    } else {
        None
    };
    Ok(AttStatus {
        installed,
        chain_hash,
    })
}

/// Remove the org attestation (needs an MSE channel, then PIN/touch).
pub(crate) fn att_clear(pin: Option<String>) -> Result<(), String> {
    let transport =
        HidTransport::open().map_err(|e| format!("Could not open HID transport: {}", e))?;
    mse_handshake(&transport)?;
    let (status, _) = transport
        .rs_key_vendor(RSKEY_VENDOR_ATT_CLEAR, None, pin.as_deref())
        .map_err(|e| e.to_string())?;
    if status != 0 {
        return Err(vendor_error(status, "attestation clear"));
    }
    Ok(())
}

/// Concatenate the DER of every PEM certificate, or pass raw DER through.
fn certs_pem_to_der(input: &[u8]) -> Result<Vec<u8>, String> {
    let text = std::str::from_utf8(input).unwrap_or("");
    if !text.contains("-----BEGIN CERTIFICATE-----") {
        return if input.first() == Some(&0x30) {
            Ok(input.to_vec())
        } else {
            Err("chain is neither PEM nor DER".into())
        };
    }
    use base64::{Engine as _, engine::general_purpose};
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(b) = rest.find("-----BEGIN CERTIFICATE-----") {
        let after = &rest[b + 27..];
        let e = after.find("-----END").ok_or("truncated PEM certificate")?;
        let b64: String = after[..e].chars().filter(|c| !c.is_whitespace()).collect();
        let der = general_purpose::STANDARD
            .decode(b64.as_bytes())
            .map_err(|_| "bad base64 in certificate".to_string())?;
        out.extend(der);
        rest = &after[e..];
    }
    Ok(out)
}

/// Install an org attestation P-256 key (PEM/DER) + cert chain (PEM/DER).
pub(crate) fn att_import(
    pin: Option<String>,
    key_file: Vec<u8>,
    chain_file: Vec<u8>,
) -> Result<(), String> {
    use crate::hal::applets::piv;

    let (algo, material) = piv::parse_private_key(&key_file)?;
    if algo != piv::ALGO_ECCP256 {
        return Err("attestation key must be P-256".into());
    }
    // material = `06 20 <32-byte scalar>`; take the value.
    if material.len() < 2 + 32 {
        return Err("could not read the P-256 scalar".into());
    }
    let scalar: [u8; 32] = material[2..2 + 32]
        .try_into()
        .map_err(|_| "P-256 scalar is not 32 bytes".to_string())?;

    let chain = certs_pem_to_der(&chain_file)?;
    if chain.is_empty() || chain.len() > 2048 {
        return Err(format!(
            "cert chain must be 1..=2048 bytes (got {})",
            chain.len()
        ));
    }

    let transport =
        HidTransport::open().map_err(|e| format!("Could not open HID transport: {}", e))?;
    let blob = wrap_secret(&transport, &scalar)?;

    let mut params = BTreeMap::new();
    params.insert(Value::Integer(1), Value::Bytes(blob));
    params.insert(Value::Integer(2), Value::Bytes(chain));
    let (status, _) = transport
        .rs_key_vendor(
            RSKEY_VENDOR_ATT_IMPORT,
            Some(Value::Map(params)),
            pin.as_deref(),
        )
        .map_err(|e| e.to_string())?;
    if status != 0 {
        return Err(vendor_error(status, "attestation import"));
    }
    Ok(())
}

// ── RS-Key FIDO LED config (CONFIG_READ/WRITE target 0x02) ──────────────

/// RS-Key LED config block length: `[steady(1), (effect, color, brightness, speed) × 4]`
const RSKEY_LED_CONF_LEN: usize = 17;

/// Read the LED configuration from an RS-Key over FIDO.
///
/// Uses CTAPHID 0x41 CONFIG_READ (target 0x02) to retrieve the
/// 17-byte LED config block. Maps the device status fields
/// (effect, color, brightness, speed) into the compatible
/// [`LedStatusConfig`] type, keeping only color and brightness
/// for backward compatibility with the Rescue LED UI.
pub(crate) fn read_rskey_led_config(transport: &HidTransport) -> Result<LedStatusConfig, PFError> {
    // `rs_key_config_read` unwraps the CBOR `{1: blob}`, so `data` is the raw
    // `EF_LED_CONF` block. `parse_led_block` handles the 17/13/9-byte layouts.
    let (data, _) = transport.rs_key_config_read(RSKEY_CFG_TARGET_LED)?;
    let (steady, statuses) = crate::hal::common::parse_led_block(&data).ok_or_else(|| {
        PFError::Device(format!(
            "LED config response too short: {} bytes",
            data.len()
        ))
    })?;

    log::info!(
        "RS-Key FIDO LED config: steady={}, statuses={:?}",
        steady,
        statuses
    );
    Ok(LedStatusConfig { steady, statuses })
}

/// Write the full LED configuration to an RS-Key over FIDO.
///
/// Builds a 17-byte config block `[steady, (effect=0, color, brightness, speed=0) × 4]`
/// and sends it via CTAPHID 0x41 CONFIG_WRITE (target 0x02). The firmware applies
/// the new config live — no reboot required.
///
/// Requires a PIN token with `AUTHENTICATOR_CONFIG` permission.
pub(crate) fn write_rskey_led_config(
    transport: &HidTransport,
    config: &LedStatusConfig,
    pin: &str,
) -> Result<String, PFError> {
    // Read-modify-write: overwrite only steady/color/brightness so a WS2812 effect
    // + speed set out-of-band (e.g. `rsk led`) survives a colour change. Fall back
    // to a fresh solid block (effect/speed = 0) if the current one can't be read.
    let mut block = [0u8; RSKEY_LED_CONF_LEN];
    if let Ok((current, _)) = transport.rs_key_config_read(RSKEY_CFG_TARGET_LED)
        && current.len() >= RSKEY_LED_CONF_LEN
    {
        block.copy_from_slice(&current[..RSKEY_LED_CONF_LEN]);
    }
    block[0] = if config.steady { 0x01 } else { 0x00 };
    for (i, &(color, brightness)) in config.statuses.iter().enumerate() {
        // effect (1+4i) and speed (4+4i) are left untouched.
        block[2 + 4 * i] = color & 0x07;
        block[3 + 4 * i] = brightness;
    }

    let pin_token = transport.get_pin_token_with_permission(
        pin,
        PinUvAuthTokenPermissions::AUTHENTICATOR_CONFIG,
        None,
    )?;

    transport.rs_key_config_write(&pin_token, RSKEY_CFG_TARGET_LED, &block)?;

    Ok("LED configuration updated successfully.".to_string())
}

// ── RS-Key FIDO Management / DEV_CONF (CONFIG_WRITE target 0x00) ────────

/// MGMT TLV tag for USB enabled interfaces.
const FIDO_MGMT_TAG_USB_ENABLED: u8 = 0x03;

/// Write the USB application enabled-mask to an RS-Key over FIDO.
///
/// Builds a TLV blob (`tag 0x03, len 2, [enabled_be]`) matching the
/// CCID Management applet WRITE CONFIG format, and sends it via
/// CTAPHID 0x41 CONFIG_WRITE (target 0x00 = DEV_CONF). The firmware
/// persists the mask to `EF_DEV_CONF`; changes apply after a re-plug.
///
/// Requires a PIN token with `AUTHENTICATOR_CONFIG` permission.
pub(crate) fn write_rskey_dev_config(
    transport: &HidTransport,
    enabled_mask: u16,
    pin: &str,
) -> Result<String, PFError> {
    let tlv = [
        FIDO_MGMT_TAG_USB_ENABLED,
        0x02,
        (enabled_mask >> 8) as u8,
        (enabled_mask & 0xFF) as u8,
    ];

    let pin_token = transport.get_pin_token_with_permission(
        pin,
        PinUvAuthTokenPermissions::AUTHENTICATOR_CONFIG,
        None,
    )?;

    transport.rs_key_config_write(&pin_token, RSKEY_CFG_TARGET_DEV_CONF, &tlv)?;

    Ok("USB applications updated. Unplug and re-plug the device to apply changes.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_cbor_2::Value;
    use std::collections::BTreeMap;

    fn empty_config_input() -> AppConfigInput {
        AppConfigInput {
            vid: None,
            pid: None,
            product_name: None,
            manufacturer_name: None,
            led_gpio: None,
            led_brightness: None,
            touch_timeout: None,
            led_driver: None,
            led_dimmable: None,
            power_cycle_on_reset: None,
            led_steady: None,
            enable_secp256k1: None,
            raw_curves_mask: None,
            led_order: None,
            enabled_usb_itf: None,
            led_num: None,
        }
    }

    #[test]
    fn test_parse_get_info_pico_fido_76_vendor_commands_at_0x15() {
        let mut map = BTreeMap::new();
        map.insert(
            Value::Integer(0x01),
            Value::Array(vec![
                Value::Text("U2F_V2".into()),
                Value::Text("FIDO_2_1".into()),
                Value::Text("FIDO_2_2".into()),
            ]),
        );
        map.insert(Value::Integer(0x03), Value::Bytes(vec![0x89; 16]));
        map.insert(Value::Integer(0x05), Value::Integer(1024));
        map.insert(
            Value::Integer(0x06),
            Value::Array(vec![Value::Integer(1), Value::Integer(2)]),
        );
        map.insert(Value::Integer(0x0D), Value::Integer(4));
        map.insert(Value::Integer(0x0E), Value::Integer(0x0706));
        map.insert(
            Value::Integer(0x15),
            Value::Array(vec![
                Value::Integer(VendorConfigCommand::AuthEncryptionEnable as u64 as i128),
                Value::Integer(VendorConfigCommand::PhysicalVidPid as u64 as i128),
                Value::Integer(0x1234567890ABCDEF),
            ]),
        );

        let info = parse_fido_get_info(&Value::Map(map)).unwrap();

        assert_eq!(info.firmware_version, "7.6");
        assert_eq!(info.pin_protocols, vec![1, 2]);
        assert!(
            info.vendor_config_commands
                .contains(&"AuthEncryptionEnable".to_string())
        );
        assert!(
            info.vendor_config_commands
                .contains(&"PhysicalVidPid".to_string())
        );
        assert!(
            info.vendor_config_commands
                .contains(&"0x1234567890ABCDEF".to_string())
        );
        assert!(info.certifications.is_empty());
    }

    #[test]
    fn test_parse_get_info_all_keys() {
        let mut map = BTreeMap::new();
        map.insert(
            Value::Integer(0x01),
            Value::Array(vec![Value::Text("FIDO_2_1".into())]),
        );
        map.insert(
            Value::Integer(0x03),
            Value::Bytes(vec![0x01, 0x02, 0x03, 0x04]),
        );
        map.insert(Value::Integer(0x05), Value::Integer(1024));
        map.insert(Value::Integer(0x07), Value::Integer(16));
        map.insert(Value::Integer(0x08), Value::Integer(64));

        let mut alg_map = BTreeMap::new();
        alg_map.insert(Value::Text("alg".into()), Value::Integer(-7)); // ES256
        map.insert(
            Value::Integer(0x0A),
            Value::Array(vec![Value::Map(alg_map)]),
        );

        map.insert(Value::Integer(0x0B), Value::Integer(2048));
        map.insert(Value::Integer(0x0C), Value::Bool(true));
        map.insert(Value::Integer(0x0D), Value::Integer(4));
        map.insert(Value::Integer(0x0E), Value::Integer(0x0102)); // 1.2
        map.insert(Value::Integer(0x0F), Value::Integer(128));

        let info = parse_fido_get_info(&Value::Map(map)).unwrap();

        assert_eq!(info.versions, vec!["FIDO_2_1"]);
        assert_eq!(info.aaguid, "01020304");
        assert_eq!(info.max_msg_size, 1024);
        assert_eq!(info.max_credential_count_in_list, Some(16));
        assert_eq!(info.max_credential_id_length, Some(64));
        assert_eq!(info.algorithms, vec!["ES256"]);
        assert_eq!(info.max_serialized_large_blob_array, Some(2048));
        assert_eq!(info.force_pin_change, Some(true));
        assert_eq!(info.min_pin_length, 4);
        assert_eq!(info.firmware_version, "1.2");
        assert_eq!(info.max_cred_blob_length, Some(128));
    }

    #[test]
    fn test_parse_get_info_certification_map_still_supported() {
        let mut cert_map = BTreeMap::new();
        cert_map.insert(Value::Text("fido-v2".into()), Value::Bool(true));
        cert_map.insert(Value::Text("0x6C07D70FE96C3897".into()), Value::Bool(true));

        let mut map = BTreeMap::new();
        map.insert(Value::Integer(0x15), Value::Map(cert_map));

        let info = parse_fido_get_info(&Value::Map(map)).unwrap();

        assert!(info.vendor_config_commands.is_empty());
        assert_eq!(info.certifications.get("fido-v2"), Some(&true));
        assert_eq!(info.certifications.get("PIN Complexity"), Some(&true));
    }

    #[test]
    fn test_parse_management_info_length_prefixed_tlv() {
        let tlv = vec![
            0x01, 0x02, 0x02, 0x23, // TAG_USB_SUPPORTED
            0x02, 0x04, 0x02, 0x39, 0x2F, 0x25, // TAG_SERIAL (big-endian 0x02392F25)
            0x03, 0x01, 0x03, // TAG_USB_ENABLED
            0x05, 0x03, 0x07, 0x06, 0x00, // TAG_VERSION
            0x0A, 0x01, 0x01, // TAG_CONFIG_LOCK
        ];
        let mut raw = vec![tlv.len() as u8];
        raw.extend(tlv);

        let info = parse_management_info(&raw).unwrap();

        assert_eq!(info.usb_supported, Some(0x0223));
        // TAG_SERIAL renders as the big-endian decimal (0x02392F25 = 37302053).
        assert_eq!(info.serial.as_deref(), Some("37302053"));
        assert_eq!(info.usb_enabled, Some(0x0003));
        assert_eq!(info.firmware_version.as_deref(), Some("7.6"));
        assert_eq!(info.config_locked, Some(true));
    }

    #[test]
    fn test_parse_management_info_rejects_truncated_tag() {
        assert!(parse_management_info(&[0x01, 0x02, 0xAA]).is_err());
    }

    #[test]
    fn test_firmware_supports_legacy_fido_hardware_config() {
        let check = |v: &str| -> bool {
            let ver = match crate::hal::common::FirmwareVersion::parse(v) {
                Some(ver) => ver,
                None => return false,
            };
            ver.major < 7 || (ver.major == 7 && ver.minor <= 2)
        };
        assert!(check("6.6"));
        assert!(check("7.0"));
        assert!(check("7.2"));
        assert!(!check("7.4"));
        assert!(!check("7.6"));
        assert!(!check("Unknown"));
    }

    #[test]
    fn test_validate_fido_config_changes_accepts_noop_without_legacy_support() {
        let fw = AnyFirmware::new(FirmwareType::PicoFido, "7.6");
        assert!(validate_fido_config_changes(&empty_config_input(), &fw).is_ok());
    }

    #[test]
    fn test_validate_fido_config_changes_rejects_hardware_update_without_legacy_support() {
        let mut config = empty_config_input();
        config.led_gpio = Some(25);
        let fw = AnyFirmware::new(FirmwareType::PicoFido, "7.6");

        let err = validate_fido_config_changes(&config, &fw)
            .unwrap_err()
            .to_string();

        assert!(err.contains("rescue mode"));
    }

    #[test]
    fn test_validate_fido_config_changes_accepts_legacy_supported_update() {
        let mut config = empty_config_input();
        config.vid = Some("FEFF".to_string());
        config.pid = Some("FCFD".to_string());
        config.led_gpio = Some(25);
        config.led_brightness = Some(8);
        config.led_dimmable = Some(true);
        config.power_cycle_on_reset = Some(false);
        config.led_steady = Some(true);

        let fw = AnyFirmware::new_with_legacy(FirmwareType::PicoFido, "7.6", true);
        assert!(validate_fido_config_changes(&config, &fw).is_ok());
    }

    #[test]
    fn test_validate_fido_config_changes_accepts_all_common_fields_in_legacy_mode() {
        // With legacy vendor support, all fields are accepted — no LkOne-style
        // VID/PID-only restriction exists for the CONFIG_WRITE path.
        let mut config = empty_config_input();
        config.led_gpio = Some(25);
        config.product_name = Some("Pico Key".to_string());
        config.touch_timeout = Some(30);

        let fw = AnyFirmware::new_with_legacy(FirmwareType::PicoFido, "7.6", true);
        assert!(validate_fido_config_changes(&config, &fw).is_ok());
    }

    #[test]
    fn test_validate_fido_config_changes_accepts_rskey_all_fields() {
        // RS-Key accepts all fields via CONFIG_WRITE TLV.
        let mut config = empty_config_input();
        config.vid = Some("FEFF".to_string());
        config.power_cycle_on_reset = Some(false);
        config.led_steady = Some(true);

        let fw = AnyFirmware::new(FirmwareType::RSKey, "5.7");
        assert!(validate_fido_config_changes(&config, &fw).is_ok());
    }

    #[test]
    fn test_parse_get_info_rskey_style() {
        // RS-Key GetInfo has a different AAGUID, may include PQC algorithms,
        // and reports firmware at a different key.
        let mut map = BTreeMap::new();
        map.insert(
            Value::Integer(0x01),
            Value::Array(vec![
                Value::Text("U2F_V2".into()),
                Value::Text("FIDO_2_0".into()),
                Value::Text("FIDO_2_1".into()),
            ]),
        );
        // RS-Key AAGUID
        map.insert(
            Value::Integer(0x03),
            Value::Bytes(vec![
                0x24, 0x79, 0xC7, 0xBF, 0x6B, 0x30, 0x56, 0x83, 0x9E, 0xC8, 0x0E, 0x81, 0x71, 0xA9,
                0x18, 0xB7,
            ]),
        );
        map.insert(Value::Integer(0x05), Value::Integer(1200));
        map.insert(
            Value::Integer(0x06),
            Value::Array(vec![Value::Integer(1), Value::Integer(2)]),
        );
        map.insert(Value::Integer(0x0D), Value::Integer(4));

        // RS-Key firmware version (5.7.4 encoded as (5<<8)|7 = 0x0507)
        map.insert(Value::Integer(0x0E), Value::Integer(0x050704));

        // Algorithms list including PQC
        let es256 = BTreeMap::from([(Value::Text("alg".into()), Value::Integer(-7))]);
        let eddsa = BTreeMap::from([(Value::Text("alg".into()), Value::Integer(-8))]);
        let pqc = BTreeMap::from([(Value::Text("alg".into()), Value::Integer(-48))]);
        map.insert(
            Value::Integer(0x0A),
            Value::Array(vec![Value::Map(es256), Value::Map(eddsa), Value::Map(pqc)]),
        );

        // RS-Key options
        let mut opts = BTreeMap::new();
        opts.insert(Value::Text("rk".into()), Value::Bool(true));
        opts.insert(Value::Text("up".into()), Value::Bool(true));
        opts.insert(Value::Text("clientPin".into()), Value::Bool(false));
        opts.insert(Value::Text("credMgmt".into()), Value::Bool(true));
        opts.insert(Value::Text("authnrCfg".into()), Value::Bool(true));
        map.insert(Value::Integer(0x04), Value::Map(opts));

        let info = parse_fido_get_info(&Value::Map(map)).unwrap();

        assert_eq!(info.aaguid, "2479C7BF6B3056839EC80E8171A918B7");
        assert_eq!(info.firmware_version, "5.7.4");
        assert_eq!(info.versions, vec!["U2F_V2", "FIDO_2_0", "FIDO_2_1"]);
        assert_eq!(info.algorithms, vec!["ES256", "EdDSA", "ML-DSA-44"]);
        assert_eq!(info.min_pin_length, 4);
        assert!(info.options.get("rk") == Some(&true));
        assert!(info.options.get("credMgmt") == Some(&true));
        assert!(info.options.get("clientPin") == Some(&false));
    }

    #[test]
    fn test_parse_get_info_empty_returns_default() {
        let result = parse_fido_get_info(&Value::Map(BTreeMap::new()));
        assert!(result.is_ok());
        assert!(result.unwrap().versions.is_empty());
    }

    #[test]
    fn test_parse_get_info_skips_unknown_keys() {
        let mut map = BTreeMap::new();
        map.insert(
            Value::Integer(0x01),
            Value::Array(vec![Value::Text("FIDO_2_1".into())]),
        );
        map.insert(Value::Integer(0x03), Value::Bytes(vec![0x89; 16]));
        map.insert(Value::Integer(0x05), Value::Integer(1024));

        // Unknown keys should be silently skipped
        map.insert(Value::Integer(0x10), Value::Integer(999));
        map.insert(Value::Integer(0x11), Value::Integer(888));
        map.insert(Value::Integer(0x12), Value::Integer(777));

        let info = parse_fido_get_info(&Value::Map(map)).unwrap();
        assert_eq!(info.versions, vec!["FIDO_2_1"]);
        assert_eq!(info.max_msg_size, 1024);
    }

    #[test]
    fn test_parse_get_info_minimal_response() {
        let mut map = BTreeMap::new();
        map.insert(
            Value::Integer(0x01),
            Value::Array(vec![Value::Text("FIDO_2_0".into())]),
        );

        let info = parse_fido_get_info(&Value::Map(map)).unwrap();
        assert_eq!(info.versions, vec!["FIDO_2_0"]);
        assert_eq!(info.max_msg_size, 0);
    }

    #[test]
    fn test_parse_get_info_certification_map_unknown_id_becomes_hex() {
        let mut cert_map = BTreeMap::new();
        cert_map.insert(Value::Text("0xDEADBEEFCAFEBABE".into()), Value::Bool(true));

        let mut map = BTreeMap::new();
        map.insert(Value::Integer(0x15), Value::Map(cert_map));

        let info = parse_fido_get_info(&Value::Map(map)).unwrap();
        assert_eq!(info.certifications.get("0xDEADBEEFCAFEBABE"), Some(&true));
    }

    #[test]
    fn test_parse_get_info_management_info_version_fallback() {
        // When firmware version in GetInfo is 0.0, management info version
        // should be used instead - this is tested via the management info
        // parsing, but verify the GetInfo parser handles the edge case.
        let mut map = BTreeMap::new();
        map.insert(
            Value::Integer(0x01),
            Value::Array(vec![Value::Text("FIDO_2_1".into())]),
        );
        map.insert(Value::Integer(0x03), Value::Bytes(vec![0x89; 16]));
        // firmware_version = 0x0000 -> would become "0.0"
        map.insert(Value::Integer(0x0E), Value::Integer(0x0000));

        let info = parse_fido_get_info(&Value::Map(map)).unwrap();
        assert_eq!(info.firmware_version, "0.0");
    }

    #[test]
    fn test_is_empty_config_input_works() {
        assert!(is_empty_config_input(&empty_config_input()));
        let mut c = empty_config_input();
        c.led_gpio = Some(25);
        assert!(!is_empty_config_input(&c));
        let mut c = empty_config_input();
        c.vid = Some("FEFF".to_string());
        assert!(!is_empty_config_input(&c));
    }

    #[test]
    fn test_is_empty_config_input_counts_rskey_only_fields() {
        // A change to any RS-Key-only PHY field alone must not be a no-op,
        // otherwise write_config drops it before build_rskey_phy_tlv runs.
        for c in [
            {
                let mut c = empty_config_input();
                c.raw_curves_mask = Some(0x08);
                c
            },
            {
                let mut c = empty_config_input();
                c.led_order = Some(1);
                c
            },
            {
                let mut c = empty_config_input();
                c.enabled_usb_itf = Some(0x05);
                c
            },
            {
                let mut c = empty_config_input();
                c.led_num = Some(3);
                c
            },
        ] {
            assert!(!is_empty_config_input(&c));
        }
    }

    #[test]
    fn test_build_rskey_phy_tlv_rejects_overlong_product_name() {
        let mut c = empty_config_input();
        c.product_name = Some("x".repeat(32)); // 32 + NUL = 33, the firmware max
        assert!(build_rskey_phy_tlv(&c).is_ok());
        c.product_name = Some("x".repeat(33)); // 33 + NUL = 34, over the limit
        assert!(build_rskey_phy_tlv(&c).is_err());
    }

    #[test]
    fn test_build_rskey_phy_tlv_emits_rskey_only_tags() {
        let mut c = empty_config_input();
        c.led_num = Some(3);
        c.led_order = Some(1);
        let tlv = build_rskey_phy_tlv(&c).unwrap();
        // tag 0x0E len 0x01 val 0x03, and tag 0x0D len 0x01 val 0x01
        assert!(tlv.windows(3).any(|w| w == [0x0E, 0x01, 0x03]));
        assert!(tlv.windows(3).any(|w| w == [0x0D, 0x01, 0x01]));
    }
}
