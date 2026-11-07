//! RS-Key firmware implementation.
//!
//! RS-Key reports its SDK version (e.g. 5.7) via CTAP `GetInfo`, which
//! does not directly map to the RS-Key release version. Because of this,
//! capability gating uses static values and runtime probes rather than
//! version checks. RS-Key does not use the legacy `authenticatorConfig` /
//! `vendorPrototype` hardware-config path; its FIDO configuration uses the
//! separate CTAPHID `0x41` CONFIG_READ/CONFIG_WRITE commands. It also supports
//! the Rescue channel.

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

    /// RS-Key does not accept PicoForge's legacy hardware-config command path.
    fn supports_legacy_fido_hardware_config(&self) -> bool {
        false
    }

    /// RS-Key uses CTAPHID `0x41` CONFIG_WRITE. The CTAP firmware version from
    /// GET_INFO reports the SDK version (for example, 5.7), which does not map
    /// to the RS-Key release version. Runtime support is checked through a
    /// CONFIG_READ probe in `write_rskey_config()`.
    fn supports_fido_config_write(&self) -> bool {
        true
    }

    fn supports_rs_key_vendor_command(&self) -> bool {
        true
    }

    fn supports_rescue_channel(&self) -> bool {
        true
    }
}
