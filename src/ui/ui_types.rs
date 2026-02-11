use gpui::SharedString;

use crate::device::types::{FidoDeviceInfo, FullDeviceStatus};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ActiveView {
    Home,
    Passkeys,
    Configuration,
    Security,
    Logs,
    About,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GlobalDeviceState {
    pub device_status: Option<FullDeviceStatus>,
    pub fido_info: Option<FidoDeviceInfo>,
    pub error: Option<String>,
}

impl GlobalDeviceState {
    pub fn new() -> Self {
        Self {
            device_status: None,
            fido_info: None,
            error: None,
        }
    }
}

// config view:

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsbIdentityPreset {
    Custom,
    Generic,
    PicoHsm,
    PicoFido,
    PicoOpenPgp,
    Pico,
    SoloKeys,
    NitroHsm,
    NitroFido2,
    NitroStart,
    NitroPro,
    NitroKey3,
    YubiKey5,
    YubiKeyNeo,
    YubiHsm2,
    Gnuk,
    GnuPg,
}

impl UsbIdentityPreset {
    pub fn details(&self) -> (SharedString, Option<&'static str>, Option<&'static str>) {
        match self {
            Self::Custom => ("Custom (Manual Entry)".into(), None, None),
            Self::Generic => ("Generic (FEFF:FCFD)".into(), Some("FEFF"), Some("FCFD")),
            Self::PicoHsm => (
                "Pico Keys HSM (2E8A:10FD)".into(),
                Some("2E8A"),
                Some("10FD"),
            ),
            Self::PicoFido => (
                "Pico Keys Fido (2E8A:10FE)".into(),
                Some("2E8A"),
                Some("10FE"),
            ),
            Self::PicoOpenPgp => (
                "Pico Keys OpenPGP (2E8A:10FF)".into(),
                Some("2E8A"),
                Some("10FF"),
            ),
            Self::Pico => ("Pico (2E8A:0003)".into(), Some("2E8A"), Some("0003")),
            Self::SoloKeys => ("SoloKeys (0483:A2CA)".into(), Some("0483"), Some("A2CA")),
            Self::NitroHsm => ("NitroHSM (20A0:4230)".into(), Some("20A0"), Some("4230")),
            Self::NitroFido2 => ("NitroFIDO2 (20A0:42D4)".into(), Some("20A0"), Some("42D4")),
            Self::NitroStart => ("NitroStart (20A0:4211)".into(), Some("20A0"), Some("4211")),
            Self::NitroPro => ("NitroPro (20A0:4108)".into(), Some("20A0"), Some("4108")),
            Self::NitroKey3 => ("Nitrokey 3 (20A0:42B2)".into(), Some("20A0"), Some("42B2")),
            Self::YubiKey5 => ("YubiKey 5 (1050:0407)".into(), Some("1050"), Some("0407")),
            Self::YubiKeyNeo => ("YubiKey Neo (1050:0116)".into(), Some("1050"), Some("0116")),
            Self::YubiHsm2 => ("YubiHSM 2 (1050:0030)".into(), Some("1050"), Some("0030")),
            Self::Gnuk => ("Gnuk Token (234B:0000)".into(), Some("234B"), Some("0000")),
            Self::GnuPg => ("GnuPG (234B:0000)".into(), Some("234B"), Some("0000")),
        }
    }

    /// Helper to find a preset by VID/PID string matching
    pub fn from_vid_pid(vid: &str, pid: &str) -> Self {
        let vid = vid.to_uppercase();
        let pid = pid.to_uppercase();

        match (vid.as_str(), pid.as_str()) {
            ("FEFF", "FCFD") => Self::Generic,
            ("2E8A", "10FD") => Self::PicoHsm,
            ("2E8A", "10FE") => Self::PicoFido,
            ("2E8A", "10FF") => Self::PicoOpenPgp,
            ("2E8A", "0003") => Self::Pico,
            ("0483", "A2CA") => Self::SoloKeys,
            ("20A0", "4230") => Self::NitroHsm,
            ("20A0", "42D4") => Self::NitroFido2,
            ("20A0", "4211") => Self::NitroStart,
            ("20A0", "4108") => Self::NitroPro,
            ("20A0", "42B2") => Self::NitroKey3,
            ("1050", "0407") => Self::YubiKey5,
            ("1050", "0116") => Self::YubiKeyNeo,
            ("1050", "0030") => Self::YubiHsm2,
            ("234B", "0000") => Self::Gnuk,
            _ => Self::Custom,
        }
    }

    pub fn all() -> &'static [Self] {
        &[
            Self::Custom,
            Self::Generic,
            Self::PicoHsm,
            Self::PicoFido,
            Self::PicoOpenPgp,
            Self::Pico,
            Self::SoloKeys,
            Self::NitroHsm,
            Self::NitroFido2,
            Self::NitroStart,
            Self::NitroPro,
            Self::NitroKey3,
            Self::YubiKey5,
            Self::YubiKeyNeo,
            Self::YubiHsm2,
            Self::Gnuk,
            Self::GnuPg,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedDriverType {
    PicoGpio = 1,
    PimoroniRgb = 2,
    Ws2812Neopixel = 3,
    Esp32Neopixel = 5,
}

impl LedDriverType {
    pub fn label(&self) -> SharedString {
        match self {
            Self::PicoGpio => "Pico (Standard GPIO)".into(),
            Self::PimoroniRgb => "Pimoroni (RGB)".into(),
            Self::Ws2812Neopixel => "WS2812 (Neopixel)".into(),
            Self::Esp32Neopixel => "ESP32 Neopixel".into(),
        }
    }

    /// Returns the u8 value expected by the firmware configuration
    pub fn value(&self) -> u8 {
        *self as u8
    }

    pub fn all() -> &'static [Self] {
        &[
            Self::PicoGpio,
            Self::PimoroniRgb,
            Self::Ws2812Neopixel,
            Self::Esp32Neopixel,
        ]
    }
}
