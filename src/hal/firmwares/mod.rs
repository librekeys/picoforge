#![allow(dead_code)]
pub mod picofido;
pub mod rskey;

pub use picofido::*;
pub use rskey::*;

use crate::hal::common::FirmwareVersion;
use crate::hal::types::FirmwareType;

#[derive(Debug, Clone)]
pub enum AnyFirmware {
    PicoFido(PicoFidoFirmware),
    RSKey(RSKeyFirmware),
}

pub trait FirmwareTrait {
    fn firmware_type(&self) -> FirmwareType;
    fn version(&self) -> &FirmwareVersion;
    fn major_minor(&self) -> (u16, u16) {
        (self.version().major, self.version().minor)
    }
    fn version_str(&self) -> &str {
        &self.version().raw
    }

    fn supports_legacy_fido_hardware_config(&self) -> bool;
    fn supports_rs_key_vendor_command(&self) -> bool;
    fn supports_rescue_channel(&self) -> bool;
}

impl AnyFirmware {
    pub fn detect_by_aaguid(aaguid: &str) -> FirmwareType {
        if aaguid == crate::hal::types::RSKEY_AAGUID {
            FirmwareType::RSKey
        } else if aaguid == crate::hal::types::PICOFIDO_AAGUID {
            FirmwareType::PicoFido
        } else {
            FirmwareType::Unknown
        }
    }

    pub fn new(fw_type: FirmwareType, version: &str) -> Self {
        let ver = FirmwareVersion::parse(version).unwrap_or_default();
        match fw_type {
            FirmwareType::PicoFido => Self::PicoFido(PicoFidoFirmware::new(ver)),
            FirmwareType::RSKey => Self::RSKey(RSKeyFirmware::new(ver)),
            FirmwareType::Unknown => Self::PicoFido(PicoFidoFirmware::new(ver)),
        }
    }

    pub fn version(&self) -> &FirmwareVersion {
        match self {
            Self::PicoFido(fw) => fw.version(),
            Self::RSKey(fw) => fw.version(),
        }
    }

    pub fn firmware_type(&self) -> FirmwareType {
        match self {
            Self::PicoFido(_) => FirmwareType::PicoFido,
            Self::RSKey(_) => FirmwareType::RSKey,
        }
    }

    pub fn supports_legacy_fido_hardware_config(&self) -> bool {
        match self {
            Self::PicoFido(fw) => fw.supports_legacy_fido_hardware_config(),
            Self::RSKey(fw) => fw.supports_legacy_fido_hardware_config(),
        }
    }

    pub fn supports_new_fido_hardware_config(&self) -> bool {
        match self {
            Self::PicoFido(fw) => !fw.supports_legacy_fido_hardware_config(),
            Self::RSKey(_) => false,
        }
    }

    pub fn supports_rs_key_vendor_command(&self) -> bool {
        match self {
            Self::PicoFido(_) => false,
            Self::RSKey(fw) => fw.supports_rs_key_vendor_command(),
        }
    }

    pub fn supports_rescue_channel(&self) -> bool {
        match self {
            Self::PicoFido(_) => true,
            Self::RSKey(_) => true,
        }
    }
}
