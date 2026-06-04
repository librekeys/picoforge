pub mod constants;
pub mod hid;

use crate::{
    device::types::{
        AppConfig, AppConfigInput, DeviceInfo, DeviceMethod, FidoDeviceInfo, FullDeviceStatus,
        StoredCredential,
    },
    error::PFError,
};
use base64::{Engine as _, engine::general_purpose};
use constants::*;
use hid::*;
use serde_cbor_2::{Value, from_slice, to_vec};
use std::collections::BTreeMap;

const LEGACY_PHY_OPT_DIMMABLE: u16 = 0x02;
const LEGACY_PHY_OPT_DISABLE_POWER_RESET: u16 = 0x04;
const LEGACY_PHY_OPT_LED_STEADY: u16 = 0x08;

// Fido functions that require pin:

pub(crate) fn get_fido_info() -> Result<FidoDeviceInfo, String> {
    log::info!("Reading FIDO device info via custom GetInfo...");

    let transport =
        HidTransport::open().map_err(|e| format!("Could not open HID transport: {}", e))?;

    let info_payload = [CtapCommand::GetInfo as u8];
    let info_res = transport
        .send_cbor(CTAPHID_CBOR, &info_payload)
        .map_err(|e| format!("GetInfo CTAP command failed: {}", e))?;

    let info_val: Value =
        from_slice(&info_res).map_err(|e| format!("Failed to parse GetInfo CBOR: {}", e))?;

    parse_fido_get_info(&info_val)
}

fn parse_fido_get_info(info_val: &Value) -> Result<FidoDeviceInfo, String> {
    let map = match info_val {
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
                    for v in arr {
                        if let Value::Text(s) = v {
                            versions.push(s.clone());
                        }
                    }
                    log::info!("Device versions (0x01): {:?}", versions);
                }
            }
            // 0x02: extensions (array of strings)
            0x02 => {
                if let Value::Array(arr) = val {
                    for v in arr {
                        if let Value::Text(s) = v {
                            extensions.push(s.clone());
                        }
                    }
                    log::info!("Device extensions (0x02): {:?}", extensions);
                }
            }
            // 0x03: aaguid (byte string)
            0x03 => {
                if let Value::Bytes(b) = val {
                    aaguid = hex::encode_upper(b);
                    log::info!("Device aaguid (0x03): {}", aaguid);
                }
            }
            // 0x04: options (map of string -> bool)
            0x04 => {
                if let Value::Map(opts_map) = val {
                    for (k, v) in opts_map {
                        if let (Value::Text(name), Value::Bool(enabled)) = (k, v) {
                            options.insert(name.clone(), *enabled);
                        }
                    }
                    log::info!("Device options (0x04): {:?}", options);
                }
            }
            // 0x05: maxMsgSize
            0x05 => {
                if let Value::Integer(n) = val {
                    max_msg_size = *n;
                    log::info!("Device maxMsgSize (0x05): {}", max_msg_size);
                }
            }
            // 0x06: pinUvAuthProtocols (array of unsigned)
            0x06 => {
                if let Value::Array(arr) = val {
                    for v in arr {
                        if let Value::Integer(n) = v {
                            pin_protocols.push(*n as u32);
                        }
                    }
                    log::info!("Device pinUvAuthProtocols (0x06): {:?}", pin_protocols);
                }
            }
            // 0x07: maxCredentialCountInList
            0x07 => {
                if let Value::Integer(n) = val {
                    max_credential_count_in_list = Some(*n);
                    log::info!(
                        "Device maxCredentialCountInList (0x07): {}",
                        max_credential_count_in_list.unwrap()
                    );
                }
            }
            // 0x08: maxCredentialIdLength
            0x08 => {
                if let Value::Integer(n) = val {
                    max_credential_id_length = Some(*n);
                    log::info!(
                        "Device maxCredentialIdLength (0x08): {}",
                        max_credential_id_length.unwrap()
                    );
                }
            }
            // 0x0A: algorithms
            0x0A => {
                if let Value::Array(arr) = val {
                    for v in arr {
                        if let Value::Map(m) = v
                            && let Some(Value::Integer(alg_id)) = m.get(&Value::Text("alg".into()))
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
                if let Value::Integer(n) = val {
                    max_serialized_large_blob_array = Some(*n);
                    log::info!(
                        "Device maxSerializedLargeBlobArray (0x0B): {}",
                        max_serialized_large_blob_array.unwrap()
                    );
                }
            }
            // 0x0C: forcePinChange
            0x0C => {
                if let Value::Bool(b) = val {
                    force_pin_change = Some(*b);
                    log::info!(
                        "Device forcePinChange (0x0C): {}",
                        force_pin_change.unwrap()
                    );
                }
            }
            // 0x0D: minPINLength
            0x0D => {
                if let Value::Integer(n) = val {
                    min_pin_length = *n;
                    log::info!("Device minPINLength (0x0D): {}", min_pin_length);
                }
            }
            // 0x0E: firmwareVersion
            0x0E => {
                if let Value::Integer(n) = val {
                    firmware_version_raw = *n;
                    log::info!("Device firmwareVersion (0x0E): {}", firmware_version_raw);
                }
            }
            // 0x0F: maxCredBlobLength
            0x0F => {
                if let Value::Integer(n) = val {
                    max_cred_blob_length = Some(*n);
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
                if let Value::Integer(n) = val {
                    remaining_discoverable_credentials = Some(*n);
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

    let firmware_version = format!(
        "{}.{}",
        (firmware_version_raw >> 8) & 0xFF,
        firmware_version_raw & 0xFF
    );

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

pub(crate) fn firmware_supports_legacy_fido_hardware_config(version: &str) -> bool {
    let Some((major, minor)) = parse_firmware_version(version) else {
        return false;
    };

    major == 7 && minor <= 2
}

fn parse_firmware_version(version: &str) -> Option<(u16, u16)> {
    let mut parts = version.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    Some((major, minor))
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

// Custom Fido functions ( works only with pico-fido firmware )

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct ManagementInfo {
    serial: Option<String>,
    firmware_version: Option<String>,
    usb_supported: Option<u16>,
    usb_enabled: Option<u16>,
    config_locked: Option<bool>,
}

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

    let supports_legacy_hardware_config =
        firmware_supports_legacy_fido_hardware_config(&fido_info.firmware_version);
    let management = read_management_info(&transport);
    let config = AppConfig {
        vid: format!("{:04X}", transport.vid),
        pid: format!("{:04X}", transport.pid),
        product_name: transport.product_name.clone(),
        ..Default::default()
    };
    let config = if supports_legacy_hardware_config {
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
        },
        config,
        secure_boot: false,
        secure_lock: false,
        method: DeviceMethod::Fido,
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
        Ok(raw) => match parse_management_info(&raw) {
            Ok(info) => Some(info),
            Err(e) => {
                log::warn!("Failed to parse FIDO management config: {}", e);
                None
            }
        },
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

        let val = &data[i..i + len];
        match tag {
            0x01 => info.usb_supported = parse_management_u16(val),
            0x02 if val.len() == 4 => {
                info.serial = Some(hex::encode_upper(val));
            }
            0x03 => info.usb_enabled = parse_management_u16(val),
            0x05 if val.len() >= 2 => {
                info.firmware_version = Some(format!("{}.{}", val[0], val[1]));
            }
            0x0A => {
                if let Some(locked) = val.first() {
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
    let supports_legacy_hardware_config =
        firmware_supports_legacy_fido_hardware_config(&fido_info.firmware_version);

    validate_fido_config_changes(&config, supports_legacy_hardware_config)?;

    let pin_val = pin.as_deref().ok_or_else(|| {
        log::error!("write_config called without any security PIN provided");
        PFError::Device(
            "A security PIN is required to change legacy FIDO hardware configuration.".into(),
        )
    })?;

    write_legacy_hardware_config(&transport, &config, pin_val)
}

fn is_empty_config_input(config: &AppConfigInput) -> bool {
    config.vid.is_none()
        && config.pid.is_none()
        && config.product_name.is_none()
        && config.led_gpio.is_none()
        && config.led_brightness.is_none()
        && config.touch_timeout.is_none()
        && config.led_driver.is_none()
        && config.led_dimmable.is_none()
        && config.power_cycle_on_reset.is_none()
        && config.led_steady.is_none()
        && config.enable_secp256k1.is_none()
}

fn validate_fido_config_changes(
    config: &AppConfigInput,
    supports_legacy_hardware_config: bool,
) -> Result<(), PFError> {
    if !supports_legacy_hardware_config
        && (config.vid.is_some()
            || config.pid.is_some()
            || config.product_name.is_some()
            || config.led_gpio.is_some()
            || config.led_brightness.is_some()
            || config.touch_timeout.is_some()
            || config.led_driver.is_some()
            || config.led_dimmable.is_some()
            || config.power_cycle_on_reset.is_some()
            || config.led_steady.is_some()
            || config.enable_secp256k1.is_some())
    {
        return Err(PFError::Device(
            "Pico-FIDO 7.6 does not support hardware configuration over FIDO-only mode. Use rescue mode to change VID/PID, product name, LED, touch timeout, power/reset, or curve settings.".into(),
        ));
    }

    if supports_legacy_hardware_config {
        if config.product_name.is_some()
            || config.touch_timeout.is_some()
            || config.led_driver.is_some()
            || config.enable_secp256k1.is_some()
        {
            return Err(PFError::Device(
                "This firmware only supports VID/PID, LED GPIO, LED brightness, and basic LED/power options over FIDO. Use rescue mode for product name, touch timeout, LED driver, or curve settings.".into(),
            ));
        }

        if config.vid.is_some() != config.pid.is_some() {
            return Err(PFError::Device(
                "VID and PID must be changed together in FIDO mode.".into(),
            ));
        }
    }

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
        let mut opts = 0u16;
        if config.led_dimmable.unwrap_or(false) {
            opts |= LEGACY_PHY_OPT_DIMMABLE;
        }
        if !config.power_cycle_on_reset.unwrap_or(true) {
            opts |= LEGACY_PHY_OPT_DISABLE_POWER_RESET;
        }
        if config.led_steady.unwrap_or(false) {
            opts |= LEGACY_PHY_OPT_LED_STEADY;
        }

        transport.send_vendor_config(
            &get_fresh_token()?,
            VendorConfigCommand::PhysicalOptions,
            Value::Integer(opts as i128),
        )?;
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
    log::info!(
        "Certificate parsed ({} bytes). Uploading to device...",
        cert_der.len()
    );

    let transport =
        HidTransport::open().map_err(|e| format!("Could not open HID transport: {}", e))?;

    let pin_token = transport
        .get_pin_token_with_permission(
            &pin,
            PinUvAuthTokenPermissions::AUTHENTICATOR_CONFIG,
            None,
        )
        .map_err(|e| {
            let s = e.to_string();
            if s.contains("0x2B") {
                return "Device does not support enterprise attestation (0x2B). Ensure firmware is up to date.".to_string();
            }
            format!("Failed to obtain PIN token: {}", s)
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
            let s = e.to_string();
            log::error!("Failed to get PIN token: {}", s);
            if s.contains("0x2B") {
                return "Device does not support enterprise attestation (0x2B). Ensure firmware is up to date.".to_string();
            }
            format!("Failed to obtain PIN token: {}", s)
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
            led_gpio: None,
            led_brightness: None,
            touch_timeout: None,
            led_driver: None,
            led_dimmable: None,
            power_cycle_on_reset: None,
            led_steady: None,
            enable_secp256k1: None,
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
            0x02, 0x04, 0x12, 0x34, 0x56, 0x78, // TAG_SERIAL
            0x03, 0x01, 0x03, // TAG_USB_ENABLED
            0x05, 0x03, 0x07, 0x06, 0x00, // TAG_VERSION
            0x0A, 0x01, 0x01, // TAG_CONFIG_LOCK
        ];
        let mut raw = vec![tlv.len() as u8];
        raw.extend(tlv);

        let info = parse_management_info(&raw).unwrap();

        assert_eq!(info.usb_supported, Some(0x0223));
        assert_eq!(info.serial.as_deref(), Some("12345678"));
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
        assert!(firmware_supports_legacy_fido_hardware_config("7.0"));
        assert!(firmware_supports_legacy_fido_hardware_config("7.2"));
        assert!(!firmware_supports_legacy_fido_hardware_config("7.4"));
        assert!(!firmware_supports_legacy_fido_hardware_config("7.6"));
        assert!(!firmware_supports_legacy_fido_hardware_config("Unknown"));
    }

    #[test]
    fn test_validate_fido_config_changes_accepts_noop_without_legacy_support() {
        assert!(validate_fido_config_changes(&empty_config_input(), false).is_ok());
    }

    #[test]
    fn test_validate_fido_config_changes_rejects_hardware_update_without_legacy_support() {
        let mut config = empty_config_input();
        config.led_gpio = Some(25);

        let err = validate_fido_config_changes(&config, false)
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

        assert!(validate_fido_config_changes(&config, true).is_ok());
    }

    #[test]
    fn test_validate_fido_config_changes_rejects_legacy_unsupported_update() {
        let mut config = empty_config_input();
        config.product_name = Some("Pico Key".to_string());

        let err = validate_fido_config_changes(&config, true)
            .unwrap_err()
            .to_string();

        assert!(err.contains("only supports VID/PID"));
    }

    #[test]
    fn test_validate_fido_config_changes_requires_vid_pid_pair_for_legacy() {
        let mut config = empty_config_input();
        config.vid = Some("FEFF".to_string());

        let err = validate_fido_config_changes(&config, true)
            .unwrap_err()
            .to_string();

        assert!(err.contains("VID and PID"));
    }
}
