//! RS-Key firmware implementation.
//!
//! RS-Key reports its SDK version (e.g. 5.7) via CTAP `GetInfo`, which
//! does not directly map to the RS-Key release version. Because of this,
//! capability gating uses static values and runtime probes rather than
//! version checks. RS-Key supports both `legacy_fido_hardware_config`
//! (via the `0x41` CONFIG_READ/CONFIG_WRITE path) and the rescue channel.

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

    /// RS-Key reports firmware 5.x (< 7) per the SDK version scheme.
    /// Per the protocol integration notes, this version range triggers
    /// PicoForge's legacy hardware-config path (authenticatorConfig +
    /// vendorPrototype) which RS-Key supports for writes, and for reads
    /// it tries the 0x41 CONFIG_READ path instead.
    fn supports_legacy_fido_hardware_config(&self) -> bool {
        false
    }

    /// RS-Key supports FIDO config write via CTAPHID 0x41 CONFIG_WRITE
    /// on v0.3.1+. The CTAP firmware version from GET_INFO reports the SDK
    /// version (e.g., 5.7) which does not map to the RS-Key release version,
    /// so we cannot version-gate here. Actual support is determined via a
    /// runtime CONFIG_READ probe in write_rskey_config().
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
