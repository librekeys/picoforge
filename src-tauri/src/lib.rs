use byteorder::{BigEndian, WriteBytesExt, ReadBytesExt};
use pcsc::{Context, Protocols, Scope, ShareMode};
use serde::{Deserialize, Serialize};
use std::io::Cursor;
// use tauri::State;

// --- Constants ---

// The Rescue Application ID (AID) from src/rescue.c
const RESCUE_AID: &[u8] = &[0xA0, 0x58, 0x3F, 0xC1, 0x9B, 0x7E, 0x4F, 0x21];

// APDU Instructions
const INS_WRITE: u8 = 0x1C;
const INS_SECURE: u8 = 0x1D;
const INS_READ: u8 = 0x1E;

// PHY Tags from src/fs/phy.h
const TAG_VIDPID: u8 = 0x00;
const TAG_LED_GPIO: u8 = 0x04;
const TAG_LED_BRIGHTNESS: u8 = 0x05;
const TAG_OPTS: u8 = 0x06;
const TAG_UP_BTN: u8 = 0x08; // Presence Button Timeout
const TAG_USB_PRODUCT: u8 = 0x09;
const TAG_CURVES: u8 = 0x0A;
const TAG_LED_DRIVER: u8 = 0x0C;

// Bitmasks for TAG_OPTS
const OPT_LED_DIMMABLE: u16 = 0x02;            
const OPT_DISABLE_POWER_RESET: u16 = 0x04;     
const OPT_LED_STEADY: u16 = 0x08;              

// Bitmasks for TAG_CURVES
const CURVE_SECP256K1: u32 = 0x08;

// --- Data Structures ---

#[derive(Serialize)]
struct DeviceInfo {
    serial: String,
    flash_used: u32,
    flash_total: u32,
    firmware_version: String,
}

#[derive(Serialize, Deserialize, Debug, Default)]
struct AppConfig {
    vid: String,
    pid: String,
    product_name: String,
    led_gpio: u8,
    led_brightness: u8,
    touch_timeout: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    led_driver: Option<u8>,
    // New Options
    led_dimmable: bool,
    power_cycle_on_reset: bool,
    led_steady: bool,
    enable_secp256k1: bool,
}

// Partial config for writing only changed values
#[derive(Deserialize, Debug)]
struct AppConfigInput {
    vid: Option<String>,
    pid: Option<String>,
    product_name: Option<String>,
    led_gpio: Option<u8>,
    led_brightness: Option<u8>,
    touch_timeout: Option<u8>,
    led_driver: Option<u8>,
    led_dimmable: Option<bool>,
    power_cycle_on_reset: Option<bool>,
    led_steady: Option<bool>,
    enable_secp256k1: Option<bool>,
}

#[derive(Serialize)]
struct FullDeviceStatus {
    info: DeviceInfo,
    config: AppConfig,
    secure_boot: bool,
    secure_lock: bool,
}

// Custom Error type for easy returning to frontend
#[derive(Debug, thiserror::Error)]
enum AppError {
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

// --- Helper Functions ---

/// Connects to the first available reader and selects the Rescue Applet
fn connect_and_select() -> Result<(pcsc::Card, Vec<u8>), AppError> {
    let ctx = Context::establish(Scope::User)?;
    
    // List readers
    let mut readers_buf = [0; 2048];
    let mut readers = ctx.list_readers(&mut readers_buf)?;
    
    // Use the first reader found
    let reader = readers.next().ok_or_else(|| AppError::Device("No Smart Card Reader found.".into()))?;
    
    // Connect
    let card = ctx.connect(reader, ShareMode::Shared, Protocols::ANY)?;
    
    // Select Applet APDU: 00 A4 04 04 [Len] [AID]
    let mut apdu = vec![0x00, 0xA4, 0x04, 0x04, RESCUE_AID.len() as u8];
    apdu.extend_from_slice(RESCUE_AID);
    
    let mut rx_buf = [0; 256];
    let rx = card.transmit(&apdu, &mut rx_buf)?;
    
    // Check Success (0x90 0x00)
    if !rx.ends_with(&[0x90, 0x00]) {
        return Err(AppError::Device("Rescue Applet not found on device. Is it in FIDO mode?".into()));
    }

    Ok((card, rx.to_vec()))
}

// --- Tauri Commands ---

#[tauri::command]
fn read_device_details() -> Result<FullDeviceStatus, AppError> {
    let (card, select_resp) = connect_and_select()?;

    // 1. Parse Basic Info (Same as your get_device_info)
    if select_resp.len() < 14 {
        return Err(AppError::Device("Invalid select response".into()));
    }
    let version_major = select_resp[2];
    let version_minor = select_resp[3];
    let serial_str = hex::encode_upper(&select_resp[4..12]);

    // 2. Read Flash Info (APDU: 80 1E 02 00 00)
    let mut rx_buf = [0; 256];
    let rx_flash = card.transmit(&[0x80, INS_READ, 0x02, 0x00, 0x00], &mut rx_buf)?;
    if !rx_flash.ends_with(&[0x90, 0x00]) { return Err(AppError::Device("Failed to read flash".into())); }
    
    let mut rdr = Cursor::new(&rx_flash[..rx_flash.len()-2]);
    let _free = rdr.read_u32::<BigEndian>().unwrap_or(0);
    let used = rdr.read_u32::<BigEndian>().unwrap_or(0);
    let total = rdr.read_u32::<BigEndian>().unwrap_or(0);

    // 3. Read Secure Boot Status (APDU: 80 1E 03 00 00) -> [Enabled(1), Locked(1), Key(1)...]
    let rx_secure = card.transmit(&[0x80, INS_READ, 0x03, 0x00, 0x00], &mut rx_buf)?;
    let (sb_enabled, sb_locked) = if rx_secure.ends_with(&[0x90, 0x00]) && rx_secure.len() >= 4 {
        (rx_secure[0] != 0, rx_secure[1] != 0)
    } else {
        (false, false)
    };

    // 4. Read PHY Config (APDU: 80 1E 01 01 00) -> TLV Data
    let rx_phy = card.transmit(&[0x80, INS_READ, 0x01, 0x01, 0x00], &mut rx_buf)?;
    if !rx_phy.ends_with(&[0x90, 0x00]) { return Err(AppError::Device("Failed to read config".into())); }

    // Parse TLV
    let mut config = AppConfig::default();
    let data = &rx_phy[..rx_phy.len()-2];
    let mut i = 0;
    while i < data.len() {
        if i + 2 > data.len() { break; }
        let tag = data[i];
        let len = data[i+1] as usize;
        i += 2;
        if i + len > data.len() { break; }
        let val = &data[i..i+len];

        match tag {
            TAG_VIDPID => {
                if val.len() == 4 {
                    let vid = u16::from_be_bytes([val[0], val[1]]);
                    let pid = u16::from_be_bytes([val[2], val[3]]);
                    config.vid = format!("{:04X}", vid);
                    config.pid = format!("{:04X}", pid);
                }
            },
            TAG_LED_GPIO => if !val.is_empty() { config.led_gpio = val[0]; },
            TAG_LED_BRIGHTNESS => if !val.is_empty() { config.led_brightness = val[0]; },
            TAG_UP_BTN => if !val.is_empty() { config.touch_timeout = val[0]; },
            TAG_USB_PRODUCT => {
                // Remove null terminator if present
                let s = std::str::from_utf8(val).unwrap_or("").trim_matches(char::from(0));
                config.product_name = s.to_string();
            },
            TAG_OPTS => if val.len() >= 2 {
                let opts = u16::from_be_bytes([val[0], val[1]]);
                
                config.led_dimmable = (opts & OPT_LED_DIMMABLE) != 0;
                config.power_cycle_on_reset = (opts & OPT_DISABLE_POWER_RESET) == 0;
                config.led_steady = (opts & OPT_LED_STEADY) != 0;
            },
            TAG_CURVES => if val.len() >= 4 {
                let curves = u32::from_be_bytes([val[0], val[1], val[2], val[3]]);
                config.enable_secp256k1 = (curves & CURVE_SECP256K1) != 0;
            },
            TAG_LED_DRIVER => if !val.is_empty() { config.led_driver = Some(val[0]); },
            _ => {}
        }
        i += len;
    }

    Ok(FullDeviceStatus {
        info: DeviceInfo {
            serial: serial_str,
            flash_used: used / 1024,
            flash_total: total / 1024,
            firmware_version: format!("{}.{}", version_major, version_minor),
        },
        config,
        secure_boot: sb_enabled,
        secure_lock: sb_locked,
    })
}

#[tauri::command]
fn get_device_info() -> Result<DeviceInfo, AppError> {
    let (card, select_resp) = connect_and_select()?;
    
    // 1. Parse Version & Serial from Select Response (see src/rescue.c)
    // Response: [MCU, PROD, VER_MAJ, VER_MIN, SERIAL(8 bytes)..., 90, 00]
    if select_resp.len() < 14 {
        return Err(AppError::Device("Invalid response from device".into()));
    }
    
    let version_major = select_resp[2];
    let version_minor = select_resp[3];
    let serial_bytes = &select_resp[4..12];
    let serial_str = hex::encode_upper(serial_bytes);

    // 2. Read Flash Info
    // APDU: 80 1E 02 00 00 (Read Flash Info)
    let apdu_read = [0x80, INS_READ, 0x02, 0x00, 0x00];
    let mut rx_buf = [0; 256];
    let rx = card.transmit(&apdu_read, &mut rx_buf)?;
    
    if !rx.ends_with(&[0x90, 0x00]) {
        return Err(AppError::Device("Failed to read flash info".into()));
    }

    // Response: [Free(4), Used(4), Total(4), Files(4), Size(4), 90, 00]
    // We need 'Used' (index 4) and 'Total' (index 8)
    // Data is Big Endian
    let mut rdr = Cursor::new(&rx[..rx.len()-2]);
    let _free = rdr.read_u32::<BigEndian>().unwrap_or(0);
    let used = rdr.read_u32::<BigEndian>().unwrap_or(0);
    let total = rdr.read_u32::<BigEndian>().unwrap_or(0);

    Ok(DeviceInfo {
        serial: serial_str,
        flash_used: used / 1024, // Convert to KB
        flash_total: total / 1024,
        firmware_version: format!("{}.{}", version_major, version_minor),
    })
}

#[tauri::command]
fn write_config(config: AppConfigInput) -> Result<String, AppError> {
    // 1. Construct TLV Blob
    let mut tlv = Vec::new();

    // VID:PID (Tag 0x00)
    if let (Some(vid_str), Some(pid_str)) = (&config.vid, &config.pid) {
        let vid = u16::from_str_radix(vid_str, 16).map_err(|_| AppError::Io("Invalid VID".into()))?;
        let pid = u16::from_str_radix(pid_str, 16).map_err(|_| AppError::Io("Invalid PID".into()))?;
        
        tlv.push(TAG_VIDPID);
        tlv.push(0x04);
        tlv.write_u16::<BigEndian>(vid).unwrap();
        tlv.write_u16::<BigEndian>(pid).unwrap();
    }

    // LED GPIO (Tag 0x04)
    if let Some(val) = config.led_gpio {
        tlv.push(TAG_LED_GPIO);
        tlv.push(0x01);
        tlv.push(val);
    }

    // LED Brightness (Tag 0x05)
    if let Some(val) = config.led_brightness {
        tlv.push(TAG_LED_BRIGHTNESS);
        tlv.push(0x01);
        tlv.push(val);
    }

    // Touch Timeout (Tag 0x08)
    if let Some(val) = config.touch_timeout {
        tlv.push(TAG_UP_BTN);
        tlv.push(0x01);
        tlv.push(val);
    }

    // Options (Tag 0x06)
    if let (Some(dim), Some(cycle), Some(steady)) = (config.led_dimmable, config.power_cycle_on_reset, config.led_steady) {
        let mut opts: u16 = 0; // Changed from u8 to u16
        
        if dim { opts |= OPT_LED_DIMMABLE; }
        
        // Logic check: "Power Cycle On" means the DISABLE flag is NOT set.
        // If "Power Cycle On" is false, we SET the DISABLE flag.
        if !cycle { opts |= OPT_DISABLE_POWER_RESET; }
        
        if steady { opts |= OPT_LED_STEADY; }
        
        tlv.push(TAG_OPTS);
        tlv.push(0x02);
        tlv.write_u16::<BigEndian>(opts).unwrap();
    }

    // Curves (Tag 0x0A)
    if let Some(enabled) = config.enable_secp256k1 {
        let mut curves: u32 = 0; 
        if enabled { curves |= CURVE_SECP256K1; }

        tlv.push(TAG_CURVES);
        tlv.push(0x04); // Length is 4
        tlv.write_u32::<BigEndian>(curves).unwrap(); 
    }

    // LED Driver (Tag 0x0C)
    if let Some(val) = config.led_driver {
        tlv.push(TAG_LED_DRIVER);
        tlv.push(0x01); // Length 1
        tlv.push(val);
    }

    // Product Name (Tag 0x09)
    if let Some(name) = config.product_name {
        if !name.is_empty() {
            let name_bytes = name.as_bytes();
            let len = name_bytes.len() + 1; 
            if len > 32 { return Err(AppError::Io("Product name too long".into())); }
            
            tlv.push(TAG_USB_PRODUCT);
            tlv.push(len as u8);
            tlv.extend_from_slice(name_bytes);
            tlv.push(0x00); 
        }
    }

    // 2. Connect and Send
    if tlv.is_empty() {
        return Ok("No changes to apply".into());
    }

    let (card, _) = connect_and_select()?;

    // APDU: 80 1C 01 00 [Lc] [Data]
    let mut apdu = vec![0x80, INS_WRITE, 0x01, 0x00, tlv.len() as u8];
    apdu.extend_from_slice(&tlv);

    let mut rx_buf = [0; 256];
    let rx = card.transmit(&apdu, &mut rx_buf)?;

    if rx.ends_with(&[0x90, 0x00]) {
        Ok("Configuration Applied Successfully".into())
    } else {
        Err(AppError::Device(format!("Write failed: {:02X?}", rx)))
    }
}

#[tauri::command]
fn enable_secure_boot(lock: bool) -> Result<String, AppError> {
    let (card, _) = connect_and_select()?;

    // APDU: 80 1D [KeyIndex] [LockBool] 00
    // KeyIndex = 0 (Default), LockBool = 1 if true
    let lock_byte = if lock { 0x01 } else { 0x00 };
    let apdu = [0x80, INS_SECURE, 0x00, lock_byte, 0x00];

    let mut rx_buf = [0; 256];
    let rx = card.transmit(&apdu, &mut rx_buf)?;

    if rx.ends_with(&[0x90, 0x00]) {
        Ok("Secure Boot Enabled".into())
    } else {
        Err(AppError::Device(format!("Secure Boot failed: {:02X?}", rx)))
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            read_device_details,
            get_device_info,
            write_config,
            enable_secure_boot
            ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}