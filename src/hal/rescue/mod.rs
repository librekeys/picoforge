//! Application-level routines for interacting with the `Rescue Applet`.
//!
//! The Rescue Applet (`A0 58 3F C1 9B 7E 4F 21`) is used for out-of-band management of the Pico FIDO key,
//! allowing configuration of device parameters before FIDO provisioning occurs.
//!
//! This module delegates all APDU logic to `RescueOperations` implemented on `PcscTransport`.

pub mod constants;
pub mod ops;

use crate::error::PFError;
use crate::hal::transport::pcsc::PcscTransport;
use crate::hal::types::*;
use ops::RescueOperations;

/// Read full device status via the Rescue applet (PC/SC transport).
pub fn read_device_details() -> Result<FullDeviceStatus, PFError> {
    PcscTransport::open()?.read_device_details()
}

/// Write PHY configuration to the device via the Rescue applet.
pub fn write_config(config: AppConfigInput) -> Result<String, PFError> {
    PcscTransport::open()?.write_config(config)
}

/// Reboot the device (normal or BOOTSEL mode) via the Rescue applet.
pub fn reboot_device(to_bootsel: bool) -> Result<String, PFError> {
    PcscTransport::open()?.reboot_device(to_bootsel)
}

/// Enable or lock secure boot via the Rescue applet.
pub fn enable_secure_boot(lock: bool) -> Result<String, PFError> {
    PcscTransport::open()?.enable_secure_boot(lock)
}

/// Read LED status configuration from the vendor LED applet (RS-Key only).
pub fn read_led_config() -> Result<LedStatusConfig, PFError> {
    PcscTransport::open_with_aid(constants::VENDOR_LED_AID)?.read_led_config()
}

/// Write a single LED status (color + brightness) via the vendor LED applet (RS-Key only).
pub fn write_led_status(
    status: u8,
    color: u8,
    brightness: u8,
    steady: bool,
) -> Result<String, PFError> {
    PcscTransport::open_with_aid(constants::VENDOR_LED_AID)?
        .write_led_status(status, color, brightness, steady)
}

/// Read USB interface configuration from the Management applet (RS-Key only).
pub fn read_management_config() -> Result<ManagementAppConfig, PFError> {
    PcscTransport::open_with_aid(constants::MANAGEMENT_AID)?.read_management_config()
}

/// Write USB interface enable mask to the Management applet (RS-Key only).
pub fn write_management_config(enabled_mask: u16) -> Result<String, PFError> {
    PcscTransport::open_with_aid(constants::MANAGEMENT_AID)?.write_management_config(enabled_mask)
}
