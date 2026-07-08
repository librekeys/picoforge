//! Rescue applet implementation for pico-fido and RS-Key firmware.
//!
//! ```text
//! rescue/
//! ├── mod.rs       — high-level rescue operations (read/write config, reboot, LED, management)
//! └── constants.rs — ISO 7816-4 constants, rescue instructions, PHY tags, vendor applets
//! ```
//!
//! # What is the Rescue Applet?
//!
//! The Rescue applet is a low-level firmware recovery and hardware configuration
//! interface that operates independently of the FIDO2/CTAP2 stack. It provides
//! direct access to device hardware settings, flash memory, and security features
//! through a proprietary APDU-based protocol.
//!
//! Both [pico-fido](https://github.com/polhenarejos/pico-fido) (C) and
//! [RS-Key](https://github.com/TheMaxMur/RS-Key) (Rust) firmware implement
//! this applet with the same AID and command set.
//!
//! # Why is Rescue Mode Needed?
//!
//! FIDO2 devices expose a standardized interface (CTAP2) that abstracts away
//! hardware details. However, there are scenarios where direct hardware access
//! is required:
//!
//! - **Firmware recovery**: When FIDO mode is unresponsive or corrupted
//! - **Hardware configuration**: Changing USB VID/PID, LED settings, touch timeout
//!   without requiring FIDO PIN authentication
//! - **Secure boot management**: Enabling/disabling secure boot, reading OTP status
//! - **Device provisioning**: Uploading attestation certificates, setting serial numbers
//! - **Firmware updates**: Rebooting into bootloader (BOOTSEL) mode for flashing
//!
//! The Rescue applet runs on the CCID (smart card) USB interface, which is always
//! available even when FIDO functionality is disabled or misconfigured.
//!
//! # Communication Protocol: PC/SC
//!
//! Unlike FIDO2 which uses USB HID (CTAPHID), the Rescue applet communicates via
//! **PC/SC** (Personal Computer/Smart Card) — the standard protocol for interacting
//! with smart card readers and ICCs (Integrated Circuit Cards).
//!
//! ```text
//! Host Application
//!       │
//!       ▼
//!  pcsc-lite daemon (pcscd)         ← Linux/macOS daemon
//!       │
//!       ▼
//!  USB CCID Class Driver             ← Smart card reader driver
//!       │
//!       ▼
//!  Device CCID Interface             ← Composite USB device
//!       │
//!       ▼
//!  Rescue Applet (APDU commands)     ← Firmware
//! ```
//!
//! ## PC/SC Architecture
//!
//! The PC/SC specification defines a standard API for communicating with smart
//! cards. In our case, the RP2040/RP2350 device emulates a CCID-compliant smart
//! card reader with an embedded ICC.
//!
//! Key concepts:
//! - **Context**: A connection to the PC/SC daemon (establishes resource manager)
//! - **Reader**: A physical or virtual smart card reader (our device appears as one)
//! - **Card**: A connection to a specific card in a reader
//! - **APDU**: Application Protocol Data Unit — the command/response format
//!
//! ## APDU Command Structure
//!
//! ```text
//! ┌─────┬─────┬─────┬─────┬─────┬─────────────┐
//! │ CLA │ INS │  P1 │  P2 │ Lc  │    Data     │
//! └─────┴─────┴─────┴─────┴─────┴─────────────┘
//!   1B    1B    1B    1B   0-1B   0-255 bytes
//! ```
//!
//! - **CLA** (0x80 for Rescue): Command class — proprietary extension
//! - **INS**: Instruction code (e.g., 0x1E for READ, 0x1C for WRITE)
//! - **P1/P2**: Parameters (sub-command selectors)
//! - **Lc**: Length of data field
//! - **Data**: Command payload
//!
//! Response ends with Status Words (SW1 SW2):
//! - `0x90 0x00`: Success
//! - `0x6A 0x82`: File/application not found
//! - `0x69 0x82`: Security status not satisfied
//!
//! # Data Flow
//!
//! ```text
//!  io::read_device_details()
//!       │
//!       ▼
//!  rescue::read_device_details()     ← this file
//!       │
//!       ▼
//!  connect_and_select()              ← PC/SC connection + applet selection
//!       │
//!       ▼
//!  card.transmit(apdu)               ← ISO 7816-4 APDU exchange
//!       │
//!       ▼
//!  PC/SC (CCID USB interface)
//! ```
//!
//! ## Applet Selection
//!
//! Every session begins with applet selection:
//!
//! ```text
//! APDU: 00 A4 04 04 08 A0 58 3F C1 9B 7E 4F 21
//!       ── ── ── ── ── ─────────────────────────
//!      CLA INS P1  P2 Len AID (Rescue Applet)
//! ```
//!
//! The SELECT response contains device identity:
//! - Byte 0: MCU type (1=RP2350, 2=ESP32-S3, etc.)
//! - Byte 1: Product type (2=FIDO)
//! - Byte 2: SDK version major
//! - Byte 3: SDK version minor
//! - Bytes 4-11: Serial number (8 bytes)
//!
//! # Module Structure
//!
//! [`constants`](crate::hal::rescue::constants) defines all protocol constants shared between pico-fido and RS-Key:
//! - ISO 7816-4 command bytes (CLA, INS, P1, P2, SW)
//! - Rescue instruction codes and parameters
//! - PHY configuration tags and bitflags
//! - Vendor applet AIDs and instructions (LED, Management)
//!
//! This module contains the public functions called from [`io`](crate::hal::io):
//! - `read_device_details()`: Reads full device status via Rescue
//! - `write_config()`: Writes PHY configuration (VID/PID, LED, curves, etc.)
//! - `reboot_device()`: Reboots device (normal or BOOTSEL mode)
//! - `enable_secure_boot()`: Enables secure boot (WIP)
//! - `read_led_config()` / `write_led_status()`: LED color configuration (RS-Key)
//! - `read_management_config()` / `write_management_config()`: USB interface config (RS-Key)
//!
//! # Firmware Differences
//!
//! | Feature | pico-fido | RS-Key |
//! |---------|-----------|--------|
//! | Language | C | Rust |
//! | Rescue AID | `A0 58 3F C1 9B 7E 4F 21` | Same |
//! | Secure Boot | `INS_SECURE` (0x1D) | `INS_OTP_LOCK` (0x1B) — irreversible |
//! | LED Applet | Not available | Available (AID: `F0 00 00 00 01`) |
//! | Management | Not available | Available (Yubico-compatible) |
//! | Anti-rollback | Not available | Available (OTP fuses) |
//!
//! # References
//!
//! - [pico-fido Rescue](https://github.com/polhenarejos/pico-fido/blob/main/src/rescue.c)
//! - [RS-Key Rescue](https://github.com/TheMaxMur/RS-Key/blob/main/crates/rsk-rescue/src/lib.rs)
//! - [PC/SC Specification](https://pcsc1groupwg.readthedocs.io/)
//! - [ISO 7816-4](https://www.iso.org/standard/74873.html)
//! - [CCID Specification](https://www.usb.org/document-library/class-specification-12-chip-smart-card-interface)

use crate::error::PFError;
use crate::hal::transport::pcsc::PcscTransport;
use crate::hal::{rescue::constants::*, types::*};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

use std::io::Cursor;

/// APDU-level rescue operations implemented on the PC/SC transport.
///
/// Each method builds the appropriate ISO 7816-4 command APDU, transmits
/// it via [`PcscTransport::transmit`], and parses the response. Multiple
/// applets are available (Rescue, LED, Management) depending on firmware.
pub trait RescueOperations {
    /// Read full device status (info, config, security flags) via the Rescue applet.
    fn read_device_details(&self) -> Result<FullDeviceStatus, PFError>;
    /// Write PHY configuration (VID/PID, LED, curves, etc.) via the Rescue applet.
    fn write_config(&self, config: AppConfigInput) -> Result<String, PFError>;
    /// Reboot the device — either normally or into BOOTSEL (firmware-update) mode.
    fn reboot_device(&self, to_bootsel: bool) -> Result<String, PFError>;
    /// Enable or lock secure boot on the device (WIP / firmware-specific).
    fn enable_secure_boot(&self, lock: bool) -> Result<String, PFError>;
    /// Read LED status configuration from the vendor LED applet (RS-Key only).
    fn read_led_config(&self) -> Result<LedStatusConfig, PFError>;
    /// Write a single LED status (color + brightness) via the LED applet (RS-Key only).
    fn write_led_status(
        &self,
        status: u8,
        color: u8,
        brightness: u8,
        steady: bool,
    ) -> Result<String, PFError>;
    /// Read USB interface configuration from the Management applet (RS-Key only).
    fn read_management_config(&self) -> Result<ManagementAppConfig, PFError>;
    /// Write USB interface enable mask to the Management applet (RS-Key only).
    fn write_management_config(&self, enabled_mask: u16) -> Result<String, PFError>;
}

impl RescueOperations for PcscTransport {
    /// Reads comprehensive device details including identity, flash usage, secure boot status, and PHY configuration.
    ///
    /// Performs three sequential APDU operations after applet selection:
    /// 1. SELECT response is parsed for MCU type, firmware version, and serial number
    /// 2. `READ(FlashInfo)` — reads flash usage statistics (free, used, total)
    /// 3. `READ(SecureBootStatus)` — reads secure boot enable/lock state
    /// 4. `READ(PhyConfig)` — reads TLV-encoded hardware configuration (VID/PID, LED, curves, etc.)
    ///
    /// # Returns
    /// A `FullDeviceStatus` struct containing device info, parsed PHY config, and secure boot state.
    ///
    /// # Errors
    /// - `PFError::Device` if the SELECT response is malformed or any READ command fails
    /// - `PFError::NoDevice` if no reader is available
    fn read_device_details(&self) -> Result<FullDeviceStatus, PFError> {
        log::info!("Reading full device details");
        let select_resp = &self.select_resp;
        let fw_type = &self.firmware_type;

        log::info!("Select Response: {:?}", select_resp);

        // FIX: Relax the length check.
        // Minimum valid response is 4 bytes data + 2 bytes SW = 6 bytes.
        if select_resp.len() < 6 {
            log::error!("Invalid select response length: {}", select_resp.len());
            return Err(PFError::Device("Invalid select response".into()));
        }

        let version_major = select_resp[2];
        let version_minor = select_resp[3];

        // FIX: Handle missing Serial Number safely
        // If the firmware sends 14 bytes, we have a serial. If it sends 6, we don't.
        let serial_str = if select_resp.len() >= 14 {
            hex::encode_upper(&select_resp[4..12])
        } else {
            log::warn!(
                "Device did not return a Serial Number (Firmware mismatch?). Using placeholder."
            );
            "00000000".to_string()
        };

        log::info!("Device Version: {}.{}", version_major, version_minor);
        log::info!("Device Serial: {}", serial_str);

        // 2. Read Flash Info
        let mut rx_buf = [0; 256];
        let rx_flash = self.transmit(
            &[
                APDU_CLA_PROPRIETARY,
                RescueInstruction::Read as u8,
                ReadParam::FlashInfo as u8,
                P2_UNUSED,
                0x00, // Le
            ],
            &mut rx_buf,
        )?;

        if !rx_flash.ends_with(&SW_SUCCESS) {
            return Err(PFError::Device("Failed to read flash".into()));
        }

        let mut rdr = Cursor::new(&rx_flash[..rx_flash.len() - 2]);
        let _free = rdr.read_u32::<BigEndian>().unwrap_or(0);
        let used = rdr.read_u32::<BigEndian>().unwrap_or(0);
        let total = rdr.read_u32::<BigEndian>().unwrap_or(0);

        // NOTE: captured but currently unused variables
        let _nfiles = rdr.read_u32::<BigEndian>().unwrap_or(0);
        let _chip_size = rdr.read_u32::<BigEndian>().unwrap_or(0);

        // --- Read Secure Boot Status ---
        let rx_secure = self.transmit(
            &[
                APDU_CLA_PROPRIETARY,
                RescueInstruction::Read as u8,
                ReadParam::SecureBootStatus as u8,
                P2_UNUSED,
                0x00,
            ],
            &mut rx_buf,
        )?;

        let (sb_enabled, sb_locked) = if rx_secure.ends_with(&[0x90, 0x00]) && rx_secure.len() >= 4
        {
            (rx_secure[0] != 0, rx_secure[1] != 0)
        } else {
            (false, false)
        }; // --- Read PHY Config ---
        let rx_phy = self.transmit(
            &[
                APDU_CLA_PROPRIETARY,
                RescueInstruction::Read as u8,
                ReadParam::PhyConfig as u8,
                0x01,
                0x00,
            ],
            &mut rx_buf,
        )?;

        if !rx_phy.ends_with(&[0x90, 0x00]) {
            return Err(PFError::Device("Failed to read config".into()));
        }

        // Parse TLV
        let mut config = AppConfig::default();
        let data = &rx_phy[..rx_phy.len() - 2];
        let mut i = 0;
        while i < data.len() {
            if i + 2 > data.len() {
                break;
            }
            let tag_byte = data[i];
            let len = data[i + 1] as usize;
            i += 2;
            if i + len > data.len() {
                break;
            }
            let val = &data[i..i + len];

            if let Some(tag) = PhyTag::from_u8(tag_byte) {
                match tag {
                    PhyTag::VidPid => {
                        if val.len() == 4 {
                            let vid = u16::from_be_bytes([val[0], val[1]]);
                            let pid = u16::from_be_bytes([val[2], val[3]]);
                            config.vid = format!("{:04X}", vid);
                            config.pid = format!("{:04X}", pid);
                        }
                    }
                    PhyTag::LedGpio => {
                        if !val.is_empty() {
                            config.led_gpio = val[0];
                        }
                    }
                    PhyTag::LedBrightness => {
                        if !val.is_empty() {
                            config.led_brightness = val[0];
                        }
                    }
                    PhyTag::PresenceTimeout => {
                        if !val.is_empty() {
                            config.touch_timeout = val[0];
                        }
                    }
                    PhyTag::UsbProduct => {
                        let s = std::str::from_utf8(val)
                            .unwrap_or("")
                            .trim_matches(char::from(0));
                        config.product_name = s.to_string();
                    }
                    PhyTag::Opts => {
                        if val.len() >= 2 {
                            let opts_val = u16::from_be_bytes([val[0], val[1]]);
                            let opts = RescueOptions::from_bits_truncate(opts_val);

                            config.led_dimmable = opts.contains(RescueOptions::LED_DIMMABLE);
                            config.power_cycle_on_reset =
                                !opts.contains(RescueOptions::DISABLE_POWER_RESET);
                            config.led_steady = opts.contains(RescueOptions::LED_STEADY);
                        }
                    }
                    PhyTag::Curves => {
                        if val.len() == 4 {
                            let curves_val = u32::from_be_bytes([val[0], val[1], val[2], val[3]]);
                            config.raw_curves_mask = Some(curves_val);
                            let curves = RescueCurves::from_bits_truncate(curves_val);
                            config.enable_secp256k1 = curves.contains(RescueCurves::SECP256K1);
                        }
                    }
                    PhyTag::LedDriver => {
                        if !val.is_empty() {
                            config.led_driver = Some(val[0]);
                        }
                    }
                    PhyTag::LedOrder => {
                        if !val.is_empty() {
                            config.led_order = Some(val[0]);
                        }
                    }
                    PhyTag::LedNum => {
                        if !val.is_empty() {
                            config.led_num = Some(val[0]);
                        }
                    }
                    PhyTag::EnabledUsbItf => {
                        if !val.is_empty() {
                            config.enabled_usb_itf = Some(val[0]);
                        }
                    }
                }
            }
            i += len;
        }

        log::info!(
            "Successfully read device details - Serial: {}, Firmware: {}.{}",
            serial_str,
            version_major,
            version_minor
        );

        Ok(FullDeviceStatus {
            info: DeviceInfo {
                serial: serial_str,
                flash_used: Some(used / 1024),
                flash_total: Some(total / 1024),
                firmware_version: format!("{}.{}", version_major, version_minor),
            },
            config,
            secure_boot: sb_enabled,
            secure_lock: sb_locked,
            method: DeviceMethod::Rescue,
            firmware_type: fw_type.clone(),
        })
    }

    /// Writes PHY configuration to the device via the Rescue Applet's WRITE command.
    ///
    /// Constructs a TLV (Tag-Length-Value) blob from the provided `AppConfigInput` fields and sends
    /// it as a single APDU: `80 1C 01 00 [Lc] [TLV Data]`. Supported tags include:
    /// - `0x00`: VID:PID (4 bytes, big-endian)
    /// - `0x04`: LED GPIO pin
    /// - `0x05`: LED brightness
    /// - `0x08`: Touch/presence timeout
    /// - `0x06`: Options bitmask (LED_DIMMABLE, DISABLE_POWER_RESET, LED_STEADY)
    /// - `0x07`: Elliptic curves bitmask (SECP256K1, etc.)
    /// - `0x0C`: LED driver selection
    /// - `0x09`: USB product name (null-terminated)
    /// - `0x0D`: LED order (RS-Key extension)
    /// - `0x0B`: Enabled USB interfaces (CCID bit is always forced on for safety)
    ///
    /// # Returns
    /// A success message string on `SW 9000`.
    ///
    /// # Errors
    /// - `PFError::Io` if VID/PID are not valid hex strings
    /// - `PFError::Device` if the WRITE APDU fails or returns a non-success status
    /// - `PFError::Io` if the product name exceeds 32 bytes
    fn write_config(&self, config: AppConfigInput) -> Result<String, PFError> {
        log::info!("Writing configuration to device");
        log::debug!("Config input: {:?}", config);

        // 1. Construct TLV Blob
        let mut tlv = Vec::new();

        // VID:PID (Tag 0x00)
        if let (Some(vid_str), Some(pid_str)) = (&config.vid, &config.pid) {
            let vid =
                u16::from_str_radix(vid_str, 16).map_err(|_| PFError::Io("Invalid VID".into()))?;
            let pid =
                u16::from_str_radix(pid_str, 16).map_err(|_| PFError::Io("Invalid PID".into()))?;

            tlv.push(PhyTag::VidPid as u8);
            tlv.push(0x04);
            tlv.write_u16::<BigEndian>(vid).unwrap();
            tlv.write_u16::<BigEndian>(pid).unwrap();
        }

        // LED GPIO (Tag 0x04)
        if let Some(val) = config.led_gpio {
            tlv.push(PhyTag::LedGpio as u8);
            tlv.push(0x01);
            tlv.push(val);
        }

        // LED Brightness (Tag 0x05)
        if let Some(val) = config.led_brightness {
            tlv.push(PhyTag::LedBrightness as u8);
            tlv.push(0x01);
            tlv.push(val);
        }

        // Touch Timeout (Tag 0x08)
        if let Some(val) = config.touch_timeout {
            tlv.push(PhyTag::PresenceTimeout as u8);
            tlv.push(0x01);
            tlv.push(val);
        }

        // Options
        if let (Some(dim), Some(cycle), Some(steady)) = (
            config.led_dimmable,
            config.power_cycle_on_reset,
            config.led_steady,
        ) {
            let mut opts = RescueOptions::empty();
            if dim {
                opts.insert(RescueOptions::LED_DIMMABLE);
            }
            if !cycle {
                opts.insert(RescueOptions::DISABLE_POWER_RESET);
            }
            if steady {
                opts.insert(RescueOptions::LED_STEADY);
            }

            tlv.push(PhyTag::Opts as u8);
            tlv.push(0x02);
            tlv.write_u16::<BigEndian>(opts.bits()).unwrap();
        }

        // Curves
        if config.enable_secp256k1.is_some() || config.raw_curves_mask.is_some() {
            let mut mask = config.raw_curves_mask.unwrap_or(0);
            if let Some(enabled) = config.enable_secp256k1 {
                if enabled {
                    mask |= RescueCurves::SECP256K1.bits();
                } else {
                    mask &= !RescueCurves::SECP256K1.bits();
                }
            }
            tlv.push(PhyTag::Curves as u8);
            tlv.push(0x04);
            tlv.write_u32::<BigEndian>(mask).unwrap();
        }

        // LED Driver (Tag 0x0C)
        if let Some(val) = config.led_driver {
            tlv.push(PhyTag::LedDriver as u8);
            tlv.push(0x01);
            tlv.push(val);
        }

        // Product Name (Tag 0x09)
        if let Some(name) = config.product_name.filter(|n| !n.is_empty()) {
            let name_bytes = name.as_bytes();
            let len = name_bytes.len() + 1;
            if len > 32 {
                return Err(PFError::Io("Product name too long".into()));
            }

            tlv.push(PhyTag::UsbProduct as u8);
            tlv.push(len as u8);
            tlv.extend_from_slice(name_bytes);
            tlv.push(0x00);
        }

        // LED Order (Tag 0x0D) — RS-Key extension, silently preserved
        if let Some(val) = config.led_order {
            tlv.push(PhyTag::LedOrder as u8);
            tlv.push(0x01);
            tlv.push(val);
        }

        // Enabled USB Interfaces (Tag 0x0B)
        if let Some(val) = config.enabled_usb_itf {
            tlv.push(PhyTag::EnabledUsbItf as u8);
            tlv.push(0x01);
            // SAFETY: Never write a mask without CCID, otherwise Rescue applet is unreachable.
            tlv.push(val | UsbInterfaces::CCID.bits());
        }

        // 2. Connect and Send
        if tlv.is_empty() {
            log::warn!("No configuration changes to apply");
            return Ok("No changes to apply".into());
        }

        log::debug!("TLV payload size: {} bytes", tlv.len());

        // APDU: 80 1C 01 00 [Lc] [Data]
        let mut apdu = vec![
            APDU_CLA_PROPRIETARY,
            RescueInstruction::Write as u8,
            WriteParam::PhyConfig as u8,
            P2_UNUSED,
            tlv.len() as u8, // Lc
        ];
        apdu.extend_from_slice(&tlv);

        let mut rx_buf = [0; 256];
        let rx = self.transmit(&apdu, &mut rx_buf)?;

        if rx.ends_with(&[0x90, 0x00]) {
            log::info!("Configuration applied successfully");
            Ok("Configuration Applied Successfully".into())
        } else {
            log::error!("Configuration write failed: {:02X?}", rx);
            Err(PFError::Device(format!("Write failed: {:02X?}", rx)))
        }
    }

    /// Reboots the device, optionally entering BOOTSEL (mass storage) mode for firmware updates.
    ///
    /// Sends a REBOOT APDU: `80 1B [P1] 00 00` where:
    /// - `P1 = 0x00` (`RebootParam::Normal`): Reboots into normal FIDO mode
    /// - `P1 = 0x01` (`RebootParam::Bootsel`): Reboots into BOOTSEL/UF2 bootloader mode
    ///
    /// # Arguments
    /// * `to_bootsel` - If `true`, device enters UF2 bootloader mode for firmware flashing.
    ///   If `false`, device performs a normal reboot into FIDO mode.
    ///
    /// # Returns
    /// A confirmation string if the reboot command was accepted.
    ///
    /// # Errors
    /// - `PFError::Device` if the APDU fails or returns a non-success status
    #[allow(dead_code)]
    fn reboot_device(&self, to_bootsel: bool) -> Result<String, PFError> {
        let param = if to_bootsel {
            RebootParam::Bootsel
        } else {
            RebootParam::Normal
        };

        let apdu = [
            APDU_CLA_PROPRIETARY,
            RescueInstruction::Reboot as u8,
            param as u8,
            P2_UNUSED,
            0x00,
        ];

        let mut rx_buf = [0; 256];
        let rx = self.transmit(&apdu, &mut rx_buf)?;

        if rx.ends_with(&SW_SUCCESS) {
            Ok("Reboot command sent".into())
        } else {
            Err(PFError::Device(format!("Reboot failed: {:02X?}", rx)))
        }
    }

    /// Enables or disables secure boot on the device. **UNSTABLE — work in progress.**
    ///
    /// Sends a SECURE APDU: `80 1D 00 [LockBool] 00` where:
    /// - `LockBool = 0x01`: Enable and lock secure boot (irreversible on some firmware)
    /// - `LockBool = 0x00`: Disable secure boot
    ///
    /// Uses pico-fido instruction `INS_SECURE` (0x1D). RS-Key uses `INS_OTP_LOCK` (0x1B)
    /// for OTP fuse locking, which is a different operation.
    ///
    /// # Arguments
    /// * `lock` - If `true`, enables secure boot with lock (may be irreversible).
    ///
    /// # Returns
    /// A confirmation string if the secure boot command was accepted.
    ///
    /// # Errors
    /// - `PFError::Device` if the APDU fails or returns a non-success status
    ///
    /// # Warning
    /// This function is unstable and may change. Locking secure boot can permanently
    /// prevent firmware downgrades. Use with caution.
    #[allow(dead_code)]
    fn enable_secure_boot(&self, lock: bool) -> Result<String, PFError> {
        // APDU: 80 1D [KeyIndex] [LockBool] 00
        // KeyIndex = 0 (Default), LockBool = 1 if true
        let lock_byte = if lock { 0x01 } else { 0x00 };

        let apdu = [
            APDU_CLA_PROPRIETARY,
            RescueInstruction::Secure as u8,
            0x00, // Boot Key Index (0 = Default)
            lock_byte as u8,
            0x00,
        ];

        let mut rx_buf = [0; 256];
        let rx = self.transmit(&apdu, &mut rx_buf)?;

        if rx.ends_with(&[0x90, 0x00]) {
            Ok("Secure Boot Enabled".into())
        } else {
            Err(PFError::Device(format!("Secure Boot failed: {:02X?}", rx)))
        }
    }

    // --- Vendor/LED Applet (RS-Key) ---

    /// Reads the customized LED status configurations from the Vendor/LED applet.
    ///
    /// Communicates with the `F0 00 00 00 01` applet to retrieve a 9-byte configuration block
    /// that dictates the color and brightness for each device state (idle, processing, touch, boot),
    /// as well as the global 'steady' toggle flag.
    fn read_led_config(&self) -> Result<LedStatusConfig, PFError> {
        log::info!("Reading LED status config from Vendor/LED applet");

        let apdu = [
            APDU_CLA_ISO,
            VendorLedInstruction::GetLed as u8,
            0x00,
            0x00,
            0x00,
        ];
        let mut rx_buf = [0; 256];
        let rx = self.transmit(&apdu, &mut rx_buf)?;

        if !rx.ends_with(&SW_SUCCESS) || rx.len() < 11 {
            return Err(PFError::Device("Failed to read LED config".into()));
        }

        let data = &rx[..rx.len() - 2];
        if data.len() < 9 {
            return Err(PFError::Device("LED config response too short".into()));
        }

        let steady = data[0] != 0;
        let mut statuses = [(0u8, 0u8); 4];
        for s in 0..4 {
            statuses[s] = (data[1 + 2 * s], data[2 + 2 * s]);
        }

        log::info!("LED config: steady={}, statuses={:?}", steady, statuses);
        Ok(LedStatusConfig { steady, statuses })
    }

    /// Applies an individual LED status update to the Vendor/LED applet.
    ///
    /// Constructs the APDU payload combining the targeted status index, color code, and global
    /// steady flag into `P2`, with the brightness value in `P1`. The update is persisted to flash
    /// and applied immediately.
    fn write_led_status(
        &self,
        status: u8,
        color: u8,
        brightness: u8,
        steady: bool,
    ) -> Result<String, PFError> {
        log::info!(
            "Setting LED: status={}, color={}, brightness={}, steady={}",
            status,
            color,
            brightness,
            steady
        );
        // Assuming transport is already connected to LED applet via open_with_aid

        let steady_bit: u8 = if steady { 0x08 } else { 0x00 };
        let p2 = (color & 0x07) | steady_bit | ((status & 0x03) << 4);

        let apdu = [
            APDU_CLA_ISO,
            VendorLedInstruction::SetLed as u8,
            brightness,
            p2,
        ];
        let mut rx_buf = [0; 256];
        let rx = self.transmit(&apdu, &mut rx_buf)?;

        if rx.ends_with(&SW_SUCCESS) {
            Ok("LED status updated".into())
        } else {
            Err(PFError::Device(format!("SET LED failed: {:02X?}", rx)))
        }
    }

    // --- Management Applet (RS-Key) ---

    /// Retrieves the device management configuration mapping from the Management applet.
    ///
    /// Reads the active state of various USB interfaces (U2F, OATH, PIV, OpenPGP, etc.) to
    /// determine which are supported by the hardware and which are currently enabled by the user.
    fn read_management_config(&self) -> Result<ManagementAppConfig, PFError> {
        log::info!("Reading management config from Management applet");

        let apdu = [
            APDU_CLA_ISO,
            ManagementInstruction::ReadConfig as u8,
            0x00,
            0x00,
            0x00,
        ];
        let mut rx_buf = [0; 256];
        let rx = self.transmit(&apdu, &mut rx_buf)?;

        if !rx.ends_with(&SW_SUCCESS) {
            return Err(PFError::Device("Failed to read management config".into()));
        }

        let data = &rx[..rx.len() - 2];
        if data.is_empty() {
            return Err(PFError::Device("Empty management config response".into()));
        }

        let overall_len = data[0] as usize;
        let tlv_data = if data.len() > 1 + overall_len {
            &data[1..1 + overall_len]
        } else {
            &data[1..]
        };

        let mut config = ManagementAppConfig::default();
        let mut i = 0;
        while i < tlv_data.len() {
            if i + 2 > tlv_data.len() {
                break;
            }
            let tag = tlv_data[i];
            let len = tlv_data[i + 1] as usize;
            i += 2;
            if i + len > tlv_data.len() {
                break;
            }
            let val = &tlv_data[i..i + len];
            match tag {
                MGMT_TAG_USB_SUPPORTED => {
                    if val.len() >= 2 {
                        config.usb_supported = u16::from_be_bytes([val[0], val[1]]);
                    }
                }
                MGMT_TAG_USB_ENABLED => {
                    if val.len() >= 2 {
                        config.usb_enabled = u16::from_be_bytes([val[0], val[1]]);
                    }
                }
                _ => {
                    log::trace!("Management TLV tag 0x{:02X} skipped", tag);
                }
            }
            i += len;
        }

        log::info!(
            "Management config: supported=0x{:04X}, enabled=0x{:04X}",
            config.usb_supported,
            config.usb_enabled
        );
        Ok(config)
    }

    /// Persists updated management endpoint configurations to the device.
    ///
    /// Overwrites the previously enabled interfaces with a new configuration bitmask.
    /// For the changes to fully apply across all composite USB endpoints, a subsequent
    /// device reboot or re-plug is required.
    fn write_management_config(&self, enabled_mask: u16) -> Result<String, PFError> {
        log::info!("Writing management config: enabled=0x{:04X}", enabled_mask);

        let inner = [
            MGMT_TAG_USB_ENABLED,
            0x02,
            (enabled_mask >> 8) as u8,
            (enabled_mask & 0xFF) as u8,
        ];

        let mut apdu = vec![
            APDU_CLA_ISO,
            ManagementInstruction::WriteConfig as u8,
            0x00,
            0x00,
            (inner.len() + 1) as u8,
            inner.len() as u8,
        ];
        apdu.extend_from_slice(&inner);

        let mut rx_buf = [0; 256];
        let rx = self.transmit(&apdu, &mut rx_buf)?;

        if rx.ends_with(&SW_SUCCESS) {
            Ok("USB applications updated".into())
        } else {
            Err(PFError::Device(format!(
                "Management write failed: {:02X?}",
                rx
            )))
        }
    }
}
