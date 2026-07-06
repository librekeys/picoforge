use crate::hal::common::FirmwareVersion;
use crate::hal::firmwares::FirmwareTrait;
use crate::hal::types::FirmwareType;

#[derive(Debug, Clone)]
pub struct PicoFidoFirmware {
    version: FirmwareVersion,
}

impl PicoFidoFirmware {
    pub fn new(version: FirmwareVersion) -> Self {
        Self { version }
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
        self.version.major < 7 || (self.version.major == 7 && self.version.minor <= 2)
    }

    fn supports_rs_key_vendor_command(&self) -> bool {
        false
    }

    fn supports_rescue_channel(&self) -> bool {
        true
    }
}
