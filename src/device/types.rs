#![allow(unused)]

use serde::{Deserialize, Serialize};

struct PForgeState {
    device_info: DeviceInfo,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DeviceInfo {
    pub serial: String,
    pub flash_used: Option<u32>,
    pub flash_total: Option<u32>,
    pub firmware_version: String,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub vid: String,
    pub pid: String,
    pub product_name: String,
    pub led_gpio: u8,
    pub led_brightness: u8,
    pub touch_timeout: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub led_driver: Option<u8>,
    pub led_dimmable: bool,
    pub power_cycle_on_reset: bool,
    pub led_steady: bool,
    pub enable_secp256k1: bool,
}

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
}

#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FullDeviceStatus {
    pub info: DeviceInfo,
    pub config: AppConfig,
    pub secure_boot: bool,
    pub secure_lock: bool,
    pub method: DeviceMethod,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum DeviceMethod {
    #[serde(rename = "FIDO")]
    Fido,
    Rescue,
}

// Fido stuff:

#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FidoDeviceInfo {
    pub versions: Vec<String>,
    pub extensions: Vec<String>,
    pub aaguid: String,
    pub options: std::collections::HashMap<String, bool>,
    pub max_msg_size: i128,
    pub pin_protocols: Vec<u32>,
    pub remaining_discoverable_credentials: Option<i128>,
    pub min_pin_length: i128,
    pub firmware_version: String,
    /// Supported vendor config commands (human-readable names), parsed from CTAP GetInfo key 0x13
    pub vendor_config_commands: Vec<String>,
    /// Device certifications, parsed from CTAP GetInfo key 0x15
    pub certifications: std::collections::HashMap<String, bool>,
    pub max_credential_count_in_list: Option<i128>,
    pub max_credential_id_length: Option<i128>,
    pub algorithms: Vec<String>,
    pub max_serialized_large_blob_array: Option<i128>,
    pub force_pin_change: Option<bool>,
    pub max_cred_blob_length: Option<i128>,
}

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
