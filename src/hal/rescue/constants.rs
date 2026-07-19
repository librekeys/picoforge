//! Rescue and vendor applet constants for pico-fido and RS-Key firmware.
//!
//! This file defines constants, enums, bitflags, and data structures used by the
//! Rescue applet and vendor-specific applets (LED, Management) across both
//! [pico-fido](https://github.com/polhenarejos/pico-fido) and
//! [RS-Key](https://github.com/TheMaxMur/RS-Key) firmware.
//!
//! # Organization
//!
//! - **ISO 7816-4 Standard Constants**: APDU command structure constants (CLA, INS, P1, P2, SW)
//! - **Rescue Applet Constants**: Commands and parameters for the Rescue applet
//! - **PHY Configuration Tags & Flags**: Hardware configuration tags and option bitflags
//! - **Vendor/LED Applet**: RS-Key specific LED control commands
//! - **Management Applet**: Yubico-compatible management interface for configuration
//!
//! # Rescue Applet
//!
//! The Rescue applet (AID: `A0 58 3F C1 9B 7E 4F 21`) provides low-level device access
//! for firmware recovery and hardware configuration. It uses ISO 7816-4 APDU commands
//! with a proprietary CLA byte (0x80).
//!
//! This applet is implemented in both firmware variants:
//! - **pico-fido**: C implementation in `pico-keys-sdk/src/rescue.c`
//! - **RS-Key**: Rust reimplementation in `crates/rsk-rescue/src/lib.rs`
//!
//! Key operations:
//! - `KeyDevSign` (0x10): Cryptographic operations (sign, get public key, upload cert)
//! - `Write` (0x1C): Write hardware configuration (PHY tags), set RTC time
//! - `Read` (0x1E): Read hardware configuration, flash info, secure boot status, time
//! - `Reboot` (0x1F): Reboot device (normal or bootloader mode)
//!
//! # PHY Configuration Tags
//!
//! PHY tags define hardware-specific parameters stored in the device's flash memory.
//! These tags control USB identifiers, LED behavior, cryptographic curves, and
//! interface enablement.
//!
//! PHY configuration is shared between pico-fido and RS-Key, with RS-Key adding
//! additional tags like `LedOrder` for RGB LED support.
//!
//! References:
//! - [pico-fido](https://github.com/polhenarejos/pico-fido) `src/fs/phy.h`
//! - [RS-Key](https://github.com/TheMaxMur/RS-Key) `crates/rsk-rescue/src/phy.rs`
//!
//! # Vendor Applets
//!
//! - **LED Applet** (AID: `F0 00 00 00 01`): RS-Key specific LED color control
//! - **Management Applet** (Yubico-compatible): Read/write device configuration
//!
//! These applets are only available in RS-Key firmware, not in pico-fido.
#![allow(unused)]

// use serde::{Deserialize, Serialize};
// use std::fmt;

// --- 1. ISO 7816-4 Standard Constants ---

/// ISO 7816-4 Class Byte (CLA) for standard commands.
///
/// CLA byte indicates the class of the command. Value 0x00 indicates standard
/// ISO commands that follow the specification exactly.
pub const APDU_CLA_ISO: u8 = 0x00;

/// ISO 7816-4 Class Byte (CLA) for proprietary commands.
///
/// CLA byte 0x80 indicates proprietary commands that extend beyond the
/// standard ISO 7816-4 specification. Used by the Rescue applet.
pub const APDU_CLA_PROPRIETARY: u8 = 0x80;

/// ISO 7816-4 SELECT instruction (INS) byte.
///
/// The SELECT command (INS 0xA4) is used to select an application or file
/// on the device by its Application Identifier (AID).
pub const APDU_INS_SELECT: u8 = 0xA4;

/// SELECT P1 parameter: Select by DF name (Application Identifier).
///
/// When P1 = 0x04, the command selects an application using its AID
/// (Application Identifier) in the data field.
pub const APDU_P1_SELECT_BY_DF_NAME: u8 = 0x04;

/// SELECT P2 parameter: Return File Control Information (FCI).
///
/// When P2 = 0x04, the device returns FCI template containing
/// application metadata (AID, label, etc.) after selection.
pub const APDU_P2_RETURN_FCI: u8 = 0x04;

/// ISO 7816-4 status word for successful command execution.
///
/// SW1=0x90, SW2=0x00 indicates the command completed successfully
/// with no errors.
pub const SW_SUCCESS: [u8; 2] = [0x90, 0x00];

// --- 2. Rescue Applet Constants ---

/// Rescue Applet Application Identifier (AID).
///
/// The Rescue applet is selected using this AID. The applet provides low-level
/// device access for firmware recovery and hardware configuration.
///
/// This AID is shared between pico-fido and RS-Key firmware:
/// - **pico-fido**: Defined in `src/rescue.c` (C implementation)
/// - **RS-Key**: Defined in `crates/rsk-rescue/src/lib.rs` (Rust reimplementation)
///
/// Byte sequence: `A0 58 3F C1 9B 7E 4F 21`
pub const RESCUE_AID: &[u8] = &[0xA0, 0x58, 0x3F, 0xC1, 0x9B, 0x7E, 0x4F, 0x21];

/// Rescue applet instruction codes.
///
/// These instructions define the operations supported by the Rescue applet.
/// Each instruction has specific P1/P2 parameters and data field requirements.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RescueInstruction {
    /// Cryptographic operations: sign data, get public key, upload certificate.
    ///
    /// P1 parameter determines the operation type (SignData, GetPublicKey, UploadCert).
    /// P2 is typically 0x00. Data field contains operation-specific payload.
    KeyDevSign = 0x10,

    /// Write hardware configuration to flash memory.
    ///
    /// P1 parameter determines which PHY tag to write (e.g., PhyConfig for 0x01).
    /// Data field contains the tag value to write.
    Write = 0x1C,

    /// Lock or unlock device access (pico-fido only, not RS-Key).
    ///
    /// P2 parameter determines lock state (0x00=Unlock, 0x01=Lock).
    /// When locked, PHY configuration commands are rejected.
    ///
    /// **Note**: This instruction is only available on pico-fido firmware
    /// (RP2350/ESP32). RS-Key uses `OtpLock = 0x1B` instead.
    Secure = 0x1D,

    /// Read hardware configuration from flash memory.
    ///
    /// P1 parameter determines which PHY tag to read (e.g., PhyConfig for 0x01).
    /// Response contains the tag value.
    Read = 0x1E,

    /// Reboot the device.
    ///
    /// P2 parameter determines reboot mode (0x00=Normal, 0x01=Bootsel).
    /// Normal reboot restarts the firmware; bootsel enters bootloader mode.
    Reboot = 0x1F,
}

/// P1 parameters for `RescueInstruction::Read` (0x1E).
///
/// These parameters determine which hardware configuration to read.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadParam {
    /// Read full PHY configuration (VID/PID, LED settings, curves, etc.).
    PhyConfig = 0x01,

    /// Read flash memory information (size, used, free).
    FlashInfo = 0x02,

    /// Read secure boot status and verification result.
    SecureBootStatus = 0x03,
}

/// P1 parameters for `RescueInstruction::Write` (0x1C).
///
/// These parameters determine which hardware configuration to write.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteParam {
    /// Write full PHY configuration (VID/PID, LED settings, curves, etc.).
    PhyConfig = 0x01,
}

/// P1 parameters for `RescueInstruction::KeyDevSign` (0x10).
///
/// These parameters determine the cryptographic operation to perform.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignParam {
    /// Sign data using the device's attestation key.
    ///
    /// Data field contains the data to sign. Response contains the signature.
    SignData = 0x01,

    /// Get the device's attestation public key.
    ///
    /// No data field required. Response contains the COSE-encoded public key.
    GetPublicKey = 0x02,

    /// Upload a certificate for the attestation key.
    ///
    /// Data field contains the DER-encoded certificate. Device stores it
    /// for later retrieval during attestation.
    UploadCert = 0x03,
}

/// P1 parameters for `RescueInstruction::Reboot` (0x1F).
///
/// These parameters determine the reboot mode.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RebootParam {
    /// Normal reboot: restart the current firmware.
    Normal = 0x00,

    /// Bootsel reboot: enter RP2040 bootloader for firmware update.
    ///
    /// This mode allows flashing new firmware via USB mass storage.
    Bootsel = 0x01,
}

/// P2 parameters for `RescueInstruction::Secure` (0x1D).
///
/// These parameters determine the lock state.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SecureLockParam {
    /// Unlock the device: allow PHY configuration commands.
    #[default]
    Unlock = 0x00,

    /// Lock the device: reject PHY configuration commands.
    ///
    /// When locked, Write and some Read operations are rejected
    /// to prevent unauthorized configuration changes.
    Lock = 0x01,
}

/// Default P2 value when not used in APDU commands.
///
/// Some commands don't use the P2 parameter. This constant provides
/// a consistent value (0x00) for such cases.
pub const P2_UNUSED: u8 = 0x00;

// --- 3. PHY Configuration Tags & Flags ---

/// PHY configuration tag identifiers.
///
/// These tags define the hardware parameters stored in the device's flash memory.
/// Each tag has a specific format and purpose for configuring the device.
///
/// Tags are used with `RescueInstruction::Read` and `RescueInstruction::Write`
/// commands to access hardware configuration.
///
/// PHY configuration is shared between pico-fido and RS-Key, with RS-Key adding
/// additional tags like `LedOrder` for RGB LED support and `LedNum` for
/// multi-LED count.
///
/// References:
/// - [pico-fido](https://github.com/polhenarejos/pico-fido) `src/fs/phy.h`
/// - [RS-Key](https://github.com/TheMaxMur/RS-Key) `crates/rsk-rescue/src/phy.rs`
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhyTag {
    /// USB Vendor ID and Product ID.
    ///
    /// Data format: `[VID_LSB, VID_MSB, PID_LSB, PID_MSB]` (4 bytes).
    /// Used to identify the device to the host system.
    VidPid = 0x00,

    /// LED GPIO pin configuration.
    ///
    /// Data format: `[GPIO_PIN]` (1 byte) or `[R_PIN, G_PIN, B_PIN]` (3 bytes).
    /// Defines which GPIO pins control the status LED(s).
    LedGpio = 0x04,

    /// LED brightness level.
    ///
    /// Data format: `[BRIGHTNESS]` (1 byte, 0-255).
    /// Controls the default brightness of the status LED.
    LedBrightness = 0x05,

    /// Device configuration options (bitflags).
    ///
    /// Data format: `[OPTIONS_LSB, OPTIONS_MSB]` (2 bytes).
    /// See `RescueOptions` bitflags for individual option bits.
    Opts = 0x06,

    /// Touch presence timeout.
    ///
    /// Data format: `[TIMEOUT_SECONDS]` (1 byte); `0`/absent = firmware default.
    /// How long the device waits for a user touch before giving up.
    PresenceTimeout = 0x08,

    /// USB product string.
    ///
    /// Data format: UTF-8 string bytes.
    /// The product name displayed to the host system.
    UsbProduct = 0x09,

    /// Enabled cryptographic curves (bitflags).
    ///
    /// Data format: `[CURVES_LSB, CURVES_MSB, CURVES_3, CURVES_MSB]` (4 bytes).
    /// See `RescueCurves` bitflags for supported curves.
    Curves = 0x0A,

    /// Enabled USB interfaces (bitflags).
    ///
    /// Data format: `[INTERFACES]` (1 byte).
    /// See `UsbInterfaces` bitflags for available interfaces.
    EnabledUsbItf = 0x0B,

    /// LED driver type.
    ///
    /// Data format: `[DRIVER_TYPE]` (1 byte).
    /// Specifies the LED driver hardware (e.g., PWM, I2C, etc.).
    LedDriver = 0x0C,

    /// LED color order for RGB LEDs.
    ///
    /// Data format: `[ORDER]` (1 byte).
    /// RS-Key specific tag for configuring LED color channel order.
    LedOrder = 0x0D,

    /// Number of LEDs on the device (RS-Key extension).
    ///
    /// Data format: `[COUNT]` (1 byte).
    /// RS-Key specific tag specifying how many individual LEDs
    /// are present (e.g., 1 for single, 3 for RGB).
    LedNum = 0x0E,
}

impl PhyTag {
    /// Convert a raw u8 value to a PhyTag enum variant.
    ///
    /// Returns `None` if the value doesn't match any known tag.
    /// Used when parsing device responses that contain raw tag bytes.
    pub fn from_u8(tag_value: u8) -> Option<Self> {
        match tag_value {
            0x00 => Some(Self::VidPid),
            0x04 => Some(Self::LedGpio),
            0x05 => Some(Self::LedBrightness),
            0x06 => Some(Self::Opts),
            0x08 => Some(Self::PresenceTimeout),
            0x09 => Some(Self::UsbProduct),
            0x0A => Some(Self::Curves),
            0x0B => Some(Self::EnabledUsbItf),
            0x0C => Some(Self::LedDriver),
            0x0D => Some(Self::LedOrder),
            0x0E => Some(Self::LedNum),
            _ => None,
        }
    }
}

/// Device configuration options bitflags.
///
/// These flags control device behavior and capabilities. They are stored
/// in the PHY configuration under tag `0x06` (Opts).
///
/// This bitflags type is shared between pico-fido and RS-Key firmware.
///
/// References:
/// - [pico-fido](https://github.com/polhenarejos/pico-fido) `src/fs/phy.h`
/// - [RS-Key](https://github.com/TheMaxMur/RS-Key) `crates/rsk-rescue/src/phy.rs`
bitflags::bitflags! {
    pub struct RescueOptions: u16 {
        /// Windows Compatible ID (WCID) support.
        ///
        /// When set, the device advertises WCID descriptors for
        /// automatic driver installation on Windows without
        /// requiring a custom .inf file.
        const WCID = 0x01;

        /// LED supports dimming (PWM control).
        ///
        /// When set, the LED brightness can be adjusted. When clear,
        /// the LED is only on/off.
        const LED_DIMMABLE = 0x02;

        /// Disable power-on reset detection.
        ///
        /// When set, the device doesn't reset on power-on events.
        /// Useful for devices with unstable power supply.
        const DISABLE_POWER_RESET = 0x04;

        /// LED stays steady (no blinking).
        ///
        /// When set, the LED remains solid when active.
        /// When clear, the LED blinks to indicate activity.
        const LED_STEADY = 0x08;
    }
}

/// Enabled cryptographic curves bitflags.
///
/// These flags define which elliptic curves are available for
/// cryptographic operations (ECDH, ECDSA, etc.).
///
/// This bitflags type is shared between pico-fido and RS-Key firmware.
///
/// References:
/// - [pico-fido](https://github.com/polhenarejos/pico-fido) `src/fs/phy.h`
/// - [RS-Key](https://github.com/TheMaxMur/RS-Key) `crates/rsk-rescue/src/phy.rs`
bitflags::bitflags! {
    pub struct RescueCurves: u32 {
        /// SECP256R1 curve (NIST P-256).
        const SECP256R1 = 0x01;
        /// SECP384R1 curve (NIST P-384).
        const SECP384R1 = 0x02;
        /// SECP521R1 curve (NIST P-521).
        const SECP521R1 = 0x04;
        /// SECP256K1 curve (Bitcoin/Ethereum).
        ///
        /// Used by cryptocurrency wallets and some FIDO2 implementations.
        /// Curve OID: 1.3.132.0.10
        const SECP256K1 = 0x08;
        /// BrainpoolP256R1 curve.
        const BP256R1 = 0x10;
        /// BrainpoolP384R1 curve.
        const BP384R1 = 0x20;
        /// BrainpoolP512R1 curve.
        const BP512R1 = 0x40;
        /// Ed25519 curve.
        const ED25519 = 0x80;
        /// Ed448 curve.
        const ED448 = 0x100;
        /// Curve25519 (X25519 key exchange).
        const CURVE25519 = 0x200;
        /// Curve448 (X448 key exchange).
        const CURVE448 = 0x400;
    }
}

/// Enabled USB interfaces bitflags.
///
/// These flags define which USB interfaces are active on the device.
/// Multiple interfaces can be enabled simultaneously.
///
/// This bitflags type is shared between pico-fido and RS-Key firmware.
///
/// References:
/// - [pico-fido](https://github.com/polhenarejos/pico-fido) `src/fs/phy.h`
/// - [RS-Key](https://github.com/TheMaxMur/RS-Key) `crates/rsk-rescue/src/phy.rs`
bitflags::bitflags! {
    pub struct UsbInterfaces: u8 {
        /// CCID interface (smart card reader).
        ///
        /// Implements USB CCID class for smart card operations.
        /// Used by some enterprise security solutions.
        const CCID = 0x01;

        /// WCID interface (Windows Compatible ID).
        ///
        /// Provides Windows-compatible device identification
        /// without requiring custom drivers.
        const WCID = 0x02;

        /// HID interface (FIDO2/CTAP2).
        ///
        /// Implements USB HID class for CTAP2/FIDO2 communication.
        /// This is the primary interface for security key operations.
        const HID = 0x04;

        /// Keyboard interface (HID keyboard).
        ///
        /// Emulates a USB keyboard for TOTP code entry or other
        /// keyboard-based interactions.
        const KB = 0x08;

        /// LWIP interface (TCP/IP stack).
        ///
        /// Enables the lightweight TCP/IP stack for network
        /// communication (if supported by hardware).
        const LWIP = 0x10;
    }
}

// --- 4. Vendor/LED Applet (RS-Key specific) ---

/// Vendor LED applet Application Identifier (AID).
///
/// This AID selects the RS-Key specific LED control applet.
/// The applet provides commands to control the device's RGB LED
/// for status indication and user feedback.
///
/// Byte sequence: `F0 00 00 00 01`
///
/// **Note**: This applet is only available in RS-Key firmware, not in pico-fido.
pub const VENDOR_LED_AID: &[u8] = &[0xF0, 0x00, 0x00, 0x00, 0x01];

/// Vendor LED applet instruction codes.
///
/// These instructions control the device's RGB LED.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VendorLedInstruction {
    /// Set LED color for a specific status indicator.
    ///
    /// P1: Status indicator (LedStatus).
    /// P2: LED color (LedColor).
    /// Data: None.
    SetLed = 0x10,

    /// Get current LED color for a status indicator.
    ///
    /// P1: Status indicator (LedStatus).
    /// P2: 0x00.
    /// Data: None.
    /// Response: LED color byte.
    GetLed = 0x11,
}

/// LED color definitions for the Vendor LED applet.
///
/// Each color corresponds to a specific RGB value that can be
/// displayed on the device's status LED.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedColor {
    /// LED off (no light).
    Off = 0,

    /// Red LED.
    Red = 1,

    /// Green LED.
    Green = 2,

    /// Blue LED.
    Blue = 3,

    /// Yellow LED (Red + Green).
    Yellow = 4,

    /// Magenta LED (Red + Blue).
    Magenta = 5,

    /// Cyan LED (Green + Blue).
    Cyan = 6,

    /// White LED (Red + Green + Blue).
    White = 7,
}

impl LedColor {
    /// Convert a raw u8 value to a LedColor enum variant.
    ///
    /// Returns `None` if the value doesn't match any known color.
    pub fn from_u8(color_value: u8) -> Option<Self> {
        match color_value {
            0 => Some(Self::Off),
            1 => Some(Self::Red),
            2 => Some(Self::Green),
            3 => Some(Self::Blue),
            4 => Some(Self::Yellow),
            5 => Some(Self::Magenta),
            6 => Some(Self::Cyan),
            7 => Some(Self::White),
            _ => None,
        }
    }

    /// Get a human-readable label for the color.
    ///
    /// Returns a static string like "Red", "Green", etc.
    /// Used for display in the UI.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::Red => "Red",
            Self::Green => "Green",
            Self::Blue => "Blue",
            Self::Yellow => "Yellow",
            Self::Magenta => "Magenta",
            Self::Cyan => "Cyan",
            Self::White => "White",
        }
    }

    /// Get all available LED colors.
    ///
    /// Returns a slice of all `LedColor` variants in order.
    /// Useful for populating UI dropdowns or color pickers.
    pub fn all() -> &'static [Self] {
        &[
            Self::Off,
            Self::Red,
            Self::Green,
            Self::Blue,
            Self::Yellow,
            Self::Magenta,
            Self::Cyan,
            Self::White,
        ]
    }
}

/// LED status indicator definitions.
///
/// These indicate the device state for which the LED color is configured.
/// Each status can have a different color for visual feedback.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedStatus {
    /// Idle state: device is waiting for user interaction.
    ///
    /// Typically shows a dim or pulsing light to indicate the device
    /// is ready but not actively processing.
    Idle = 0,

    /// Processing state: device is performing an operation.
    ///
    /// Shows a bright or blinking light to indicate the device
    /// is busy (e.g., during cryptographic operations).
    Processing = 1,

    /// Touch required: device is waiting for user touch.
    ///
    /// Shows a specific color pattern to prompt the user to
    /// touch the device's capacitive sensor.
    Touch = 2,

    /// Boot state: device is starting up.
    ///
    /// Shows a brief color pattern during the boot sequence
    /// to indicate successful initialization.
    Boot = 3,
}

impl LedStatus {
    /// Get a human-readable label for the status.
    ///
    /// Returns a static string like "Idle", "Processing", etc.
    /// Used for display in the UI.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Idle => "Idle",
            Self::Processing => "Processing",
            Self::Touch => "Touch",
            Self::Boot => "Boot",
        }
    }

    /// Get all available LED statuses.
    ///
    /// Returns a slice of all `LedStatus` variants in order.
    /// Useful for populating UI dropdowns or status lists.
    pub fn all() -> &'static [Self] {
        &[Self::Idle, Self::Processing, Self::Touch, Self::Boot]
    }
}

// --- 5. Management Applet (Yubico-compatible, RS-Key) ---

/// Management applet Application Identifier (AID).
///
/// This AID selects the Yubico-compatible management applet in RS-Key firmware.
/// The applet provides configuration read/write operations similar to Yubico's
/// management interface.
///
/// Byte sequence: `A0 00 00 05 27 47 11 17`
///
/// This applet is available in both pico-fido and RS-Key firmware.
pub const MANAGEMENT_AID: &[u8] = &[0xA0, 0x00, 0x00, 0x05, 0x27, 0x47, 0x11, 0x17];

/// Management applet instruction codes.
///
/// These instructions read and write device configuration using
/// a TLV (Tag-Length-Value) format.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManagementInstruction {
    /// Read device configuration.
    ///
    /// Returns a TLV-encoded map of device settings.
    /// Use `MGMT_TAG_*` constants to parse the response.
    ReadConfig = 0x1D,

    /// Write device configuration.
    ///
    /// Accepts a TLV-encoded map of settings to write.
    /// Use `MGMT_TAG_*` constants to build the request.
    WriteConfig = 0x1C,
}

/// TLV tag for USB support flag.
///
/// Value: `true` if the device supports USB, `false` otherwise.
pub const MGMT_TAG_USB_SUPPORTED: u8 = 0x01;

/// TLV tag for device serial number.
///
/// Value: 32-bit unsigned integer representing the device serial.
pub const MGMT_TAG_SERIAL: u8 = 0x02;

/// TLV tag for enabled USB interfaces.
///
/// Value: Bitmask of enabled USB interfaces (see `USB_CAP_*` constants).
pub const MGMT_TAG_USB_ENABLED: u8 = 0x03;

/// TLV tag for device form factor.
///
/// Value: Device form factor identifier (e.g., USB key, NFC, etc.).
pub const MGMT_TAG_FORM_FACTOR: u8 = 0x04;

/// TLV tag for firmware version.
///
/// Value: Firmware version number (major.minor.patch encoded as integer).
pub const MGMT_TAG_VERSION: u8 = 0x05;

/// TLV tag for device flags.
///
/// Value: Bitmask of device flags (e.g., FIPS compliance, etc.).
pub const MGMT_TAG_DEVICE_FLAGS: u8 = 0x08;

/// TLV tag for configuration lock state.
///
/// Value: `true` if configuration is locked, `false` otherwise.
/// When locked, `WriteConfig` commands are rejected.
pub const MGMT_TAG_CONFIG_LOCK: u8 = 0x0A;

/// USB capability: OTP (One-Time Password) interface.
pub const USB_CAP_OTP: u16 = 0x0001;

/// USB capability: U2F (Universal 2nd Factor) interface.
pub const USB_CAP_U2F: u16 = 0x0002;

/// USB capability: OpenPGP card interface.
pub const USB_CAP_OPENPGP: u16 = 0x0008;

/// USB capability: PIV (Personal Identity Verification) interface.
pub const USB_CAP_PIV: u16 = 0x0010;

/// USB capability: OATH (One-Time Auth) interface.
pub const USB_CAP_OATH: u16 = 0x0020;

/// USB capability: FIDO2/CTAP2 interface.
pub const USB_CAP_FIDO2: u16 = 0x0200;
