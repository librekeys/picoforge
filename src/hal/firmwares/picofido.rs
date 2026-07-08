use crate::hal::common::FirmwareVersion;
use crate::hal::firmwares::FirmwareTrait;
use crate::hal::types::FirmwareType;

#[derive(Debug, Clone)]
pub struct PicoFidoFirmware {
    version: FirmwareVersion,
    /// Whether the device responded positively to the legacy
    /// VendorPrototype 0xFF probe (PicoForge CONFIG_PHY_* commands).
    has_legacy_vendor: bool,
}

impl PicoFidoFirmware {
    pub fn new(version: FirmwareVersion) -> Self {
        Self {
            version,
            has_legacy_vendor: false,
        }
    }

    pub fn with_legacy_vendor(mut self, legacy: bool) -> Self {
        self.has_legacy_vendor = legacy;
        self
    }
}

impl FirmwareTrait for PicoFidoFirmware {
    fn firmware_type(&self) -> FirmwareType {
        FirmwareType::PicoFido
    }

    fn version(&self) -> &FirmwareVersion {
        &self.version
    }

    fn supports_legacy_fido_hardware_config(&self) -> bool {
        self.has_legacy_vendor
            || self.version.major < 7
            || (self.version.major == 7 && self.version.minor <= 2)
    }

    fn supports_fido_config_write(&self) -> bool {
        self.has_legacy_vendor || self.version.major >= 7
    }

    fn supports_rs_key_vendor_command(&self) -> bool {
        false
    }

    fn supports_rescue_channel(&self) -> bool {
        true
    }
}
