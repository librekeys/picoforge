use crate::hal::common::FirmwareVersion;
use crate::hal::firmwares::FirmwareTrait;
use crate::hal::types::FirmwareType;

#[derive(Debug, Clone)]
pub struct RSKeyFirmware {
    version: FirmwareVersion,
}

impl RSKeyFirmware {
    pub fn new(version: FirmwareVersion) -> Self {
        Self { version }
    }
}

impl FirmwareTrait for RSKeyFirmware {
    fn firmware_type(&self) -> FirmwareType {
        FirmwareType::RSKey
    }

    fn version(&self) -> &FirmwareVersion {
        &self.version
    }

    fn supports_legacy_fido_hardware_config(&self) -> bool {
        false
    }

    fn supports_rs_key_vendor_command(&self) -> bool {
        self.version.is_at_least(0, 1)
    }

    fn supports_rescue_channel(&self) -> bool {
        true
    }
}
