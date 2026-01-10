use serde::{Deserialize, Serialize};

// --- Constants ---

// The Rescue Application ID (AID) from src/rescue.c
pub const RESCUE_AID: &[u8] = &[0xA0, 0x58, 0x3F, 0xC1, 0x9B, 0x7E, 0x4F, 0x21];

// APDU Instructions
pub const INS_WRITE: u8 = 0x1C;
pub const INS_SECURE: u8 = 0x1D;
pub const INS_READ: u8 = 0x1E;

// PHY Tags from src/fs/phy.h
pub const TAG_VIDPID: u8 = 0x00;
pub const TAG_LED_GPIO: u8 = 0x04;
pub const TAG_LED_BRIGHTNESS: u8 = 0x05;
pub const TAG_OPTS: u8 = 0x06;
pub const TAG_UP_BTN: u8 = 0x08; // Presence Button Timeout
pub const TAG_USB_PRODUCT: u8 = 0x09;
pub const TAG_CURVES: u8 = 0x0A;
pub const TAG_LED_DRIVER: u8 = 0x0C;

// Bitmasks for TAG_OPTS
pub const OPT_LED_DIMMABLE: u16 = 0x02;
pub const OPT_DISABLE_POWER_RESET: u16 = 0x04;
pub const OPT_LED_STEADY: u16 = 0x08;

// Bitmasks for TAG_CURVES
pub const CURVE_SECP256K1: u32 = 0x08;

// --- Data Structures ---

#[derive(Serialize)]
pub struct DeviceInfo {
    pub serial: String,
    pub flash_used: u32,
    pub flash_total: u32,
    pub firmware_version: String,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct AppConfig {
    pub vid: String,
    pub pid: String,
    pub product_name: String,
    pub led_gpio: u8,
    pub led_brightness: u8,
    pub touch_timeout: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub led_driver: Option<u8>,
    // New Options
    pub led_dimmable: bool,
    pub power_cycle_on_reset: bool,
    pub led_steady: bool,
    pub enable_secp256k1: bool,
}

#[derive(Deserialize, Debug)]
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

#[derive(Serialize)]
pub struct FullDeviceStatus {
    pub info: DeviceInfo,
    pub config: AppConfig,
    pub secure_boot: bool,
    pub secure_lock: bool,
}

// Fido stuff:

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FidoDeviceInfo {
    pub versions: Vec<String>,
    pub extensions: Vec<String>,
    pub aaguid: String,
    pub options: std::collections::HashMap<String, bool>,
    pub max_msg_size: i32,
    pub pin_protocols: Vec<u32>,
    // pub remaining_disc_creds: u32,
    pub min_pin_length: u32,
    pub firmware_version: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PicoMemoryStats {
    pub free: u64,
    pub used: u64,
    pub total: u64,
    pub files: u64,
    pub flash_size: u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PicoCredential {
    pub rp_id: String,
    pub user_id: String,
    pub user_name: String,
    pub user_display_name: String,
    pub credential_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PicoPinStatus {
    pub pin_set: bool,
    pub retries: u8,
}


// Error stuff:

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("PCSC Error: {0}")]
    Pcsc(#[from] pcsc::Error),
    #[error("IO/Hex Error: {0}")]
    Io(String),
    #[error("Device Error: {0}")]
    Device(String),
}

// Allow error to be serialized to string for Tauri
impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}