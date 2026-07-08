//! High-level device I/O dispatching across FIDO and Rescue protocols.
//!
//! Each public function here selects the appropriate protocol path based
//! on the detected firmware type or an explicit [`DeviceMethod`] parameter.
//! Some functions (e.g. `read_device_details`) try Rescue (PC/SC) first,
//! then FIDO, and merge results to produce a complete status snapshot.

use crate::{
    error::PFError,
    hal::{fido, rescue, transport::DeviceHandle, types::*},
};

/// Read full device status by merging FIDO and Rescue data where available.
///
/// Tries the FIDO HID transport first, then falls back to the PC/SC
/// rescue channel. When both succeed, fields from the more detailed
/// source are used (e.g. serial/flash from Rescue, AAGUID from FIDO).
pub fn read_device_details() -> Result<FullDeviceStatus, PFError> {
    let mut fido_status: Option<FullDeviceStatus> = None;
    let mut rescue_status: Option<FullDeviceStatus> = None;
    let mut rescue_fw_type: Option<FirmwareType> = None;

    // Discover via Rescue/PC/SC transport (preferred for richer details)
    match DeviceHandle::try_rescue() {
        Ok(Some((handle, _identity))) => {
            rescue_fw_type = Some(handle.firmware_type());
            match rescue::read_device_details() {
                Ok(status) => {
                    log::info!("Rescue device details read successfully");
                    rescue_status = Some(status);
                }
                Err(e) => log::warn!("Rescue read_device_details failed: {}", e),
            }
        }
        Ok(None) => log::info!("No Rescue PC/SC device found"),
        Err(e) => log::warn!("Rescue PC/SC discovery error: {}", e),
    }

    // Discover via FIDO/HID transport (fallback for FIDO-only details)
    match DeviceHandle::try_fido() {
        Ok(Some((_handle, _identity))) => match fido::read_device_details() {
            Ok(status) => {
                log::info!("FIDO device details read successfully");
                fido_status = Some(status);
            }
            Err(e) => log::warn!("FIDO read_device_details failed: {}", e),
        },
        Ok(None) => log::info!("No FIDO HID device found"),
        Err(e) => log::warn!("FIDO HID discovery error: {}", e),
    }

    match (fido_status, rescue_status) {
        (Some(fido), Some(rescue)) => {
            log::info!("Merging FIDO and Rescue device details");
            Ok(FullDeviceStatus {
                info: DeviceInfo {
                    serial: rescue.info.serial,
                    flash_used: rescue.info.flash_used,
                    flash_total: rescue.info.flash_total,
                    firmware_version: fido.info.firmware_version,
                },
                config: AppConfig {
                    vid: if !rescue.config.vid.is_empty() {
                        rescue.config.vid
                    } else {
                        fido.config.vid
                    },
                    pid: if !rescue.config.pid.is_empty() {
                        rescue.config.pid
                    } else {
                        fido.config.pid
                    },
                    led_gpio: rescue.config.led_gpio,
                    led_brightness: rescue.config.led_brightness,
                    led_dimmable: rescue.config.led_dimmable,
                    power_cycle_on_reset: rescue.config.power_cycle_on_reset,
                    led_steady: rescue.config.led_steady,
                    enable_secp256k1: rescue.config.enable_secp256k1,
                    led_driver: rescue.config.led_driver.or_else(|| {
                        if fido.config.led_driver.is_some() {
                            fido.config.led_driver
                        } else {
                            None
                        }
                    }),
                    product_name: rescue.config.product_name,
                    touch_timeout: rescue.config.touch_timeout,
                    raw_curves_mask: rescue.config.raw_curves_mask,
                    led_order: rescue.config.led_order,
                    enabled_usb_itf: rescue.config.enabled_usb_itf,
                    led_num: rescue.config.led_num,
                },
                secure_boot: rescue.secure_boot,
                secure_lock: rescue.secure_lock,
                method: DeviceMethod::Rescue,
                firmware_type: fido.firmware_type,
            })
        }
        (Some(fido), None) => {
            log::info!("Using FIDO-only device details");
            Ok(FullDeviceStatus {
                firmware_type: fido.firmware_type,
                ..fido
            })
        }
        (None, Some(rescue)) => {
            log::info!("Using Rescue-only device details");
            let ft = rescue_fw_type.and_then(|ft| {
                if rescue.firmware_type == FirmwareType::Unknown {
                    Some(ft)
                } else {
                    None
                }
            });
            Ok(FullDeviceStatus {
                firmware_type: ft.unwrap_or(rescue.firmware_type),
                ..rescue
            })
        }
        (None, None) => {
            log::error!("Failed to read device details via both FIDO and Rescue");
            Err(PFError::NoDevice)
        }
    }
}

#[allow(dead_code)]
/// Enable or lock secure boot on the device (Rescue-only operation).
pub fn enable_secure_boot(lock: bool) -> Result<String, PFError> {
    rescue::enable_secure_boot(lock)
}

#[allow(dead_code)]
/// Reboot the device (normal or BOOTSEL mode) via the Rescue channel.
pub fn reboot(to_bootsel: bool) -> Result<String, PFError> {
    rescue::reboot_device(to_bootsel)
}

/// Write device configuration, selecting FIDO or Rescue path by method.
///
/// The FIDO path requires a PIN; the Rescue path does not.
pub fn write_config(
    config: AppConfigInput,
    method: DeviceMethod,
    pin: Option<String>,
) -> Result<String, PFError> {
    if method == DeviceMethod::Fido {
        fido::write_config(config, pin)
    } else {
        rescue::write_config(config)
    }
}

/// Read the LED status configuration via the specified transport method.
pub fn read_led_config(method: DeviceMethod) -> Result<LedStatusConfig, PFError> {
    match method {
        DeviceMethod::Fido => {
            let transport = crate::hal::transport::fido::HidTransport::open()?;
            fido::read_rskey_led_config(&transport)
        }
        DeviceMethod::Rescue => rescue::read_led_config(),
    }
}

/// Write LED status configuration (all four status slots) via the specified transport.
pub fn write_led_config(
    method: DeviceMethod,
    config: LedStatusConfig,
    pin: Option<String>,
) -> Result<String, PFError> {
    match method {
        DeviceMethod::Fido => {
            let pin = pin.ok_or_else(|| {
                PFError::Device("PIN is required for FIDO LED config write".into())
            })?;
            let transport = crate::hal::transport::fido::HidTransport::open()?;
            fido::write_rskey_led_config(&transport, &config, &pin)
        }
        DeviceMethod::Rescue => {
            for i in 0..4 {
                let (color, brightness) = config.statuses[i];
                rescue::write_led_status(i as u8, color, brightness, config.steady)?;
            }
            Ok("LED configuration applied successfully.".to_string())
        }
    }
}

/// Read USB interface configuration from the Management applet.
pub fn read_management_config(method: DeviceMethod) -> Result<ManagementAppConfig, PFError> {
    match method {
        DeviceMethod::Fido => {
            let transport = crate::hal::transport::fido::HidTransport::open()?;
            let info = fido::read_rskey_management_info(&transport)?;
            Ok(ManagementAppConfig {
                usb_supported: info.usb_supported.unwrap_or(0),
                usb_enabled: info.usb_enabled.unwrap_or(0),
            })
        }
        DeviceMethod::Rescue => rescue::read_management_config(),
    }
}

/// Write the USB interface enable mask via the specified transport.
pub fn write_management_config(
    method: DeviceMethod,
    enabled_mask: u16,
    pin: Option<String>,
) -> Result<String, PFError> {
    match method {
        DeviceMethod::Fido => {
            let pin = pin.ok_or_else(|| {
                PFError::Device("PIN is required for FIDO management config write".into())
            })?;
            let transport = crate::hal::transport::fido::HidTransport::open()?;
            fido::write_rskey_dev_config(&transport, enabled_mask, &pin)
        }
        DeviceMethod::Rescue => rescue::write_management_config(enabled_mask),
    }
}

/// Retrieve the FIDO authenticator metadata (GetInfo) as [`FidoDeviceInfo`].
pub(crate) fn get_fido_info() -> Result<FidoDeviceInfo, String> {
    fido::get_fido_info()
}

/// Change the FIDO PIN from `current_pin` to `new_pin`.
pub(crate) fn change_fido_pin(
    current_pin: Option<String>,
    new_pin: String,
) -> Result<String, String> {
    fido::change_fido_pin(current_pin, new_pin)
}

/// Set a new minimum PIN length on the authenticator.
pub(crate) fn set_min_pin_length(
    current_pin: String,
    min_pin_length: u8,
) -> Result<String, String> {
    fido::set_min_pin_length(current_pin, min_pin_length)
}

/// Enumerate all credentials stored on the authenticator.
pub fn get_credentials(pin: String) -> Result<Vec<StoredCredential>, String> {
    fido::get_credentials(pin)
}

/// Delete a credential from the authenticator by credential ID.
pub fn delete_credential(pin: String, credential_id: String) -> Result<String, String> {
    fido::delete_credential(pin, credential_id)
}

/// Perform a factory reset on the authenticator.
pub fn reset_device() -> Result<String, String> {
    fido::reset_device()
}

/// Enable enterprise attestation on the authenticator.
pub fn enable_enterprise_attestation(pin: String) -> Result<String, String> {
    fido::enable_enterprise_attestation(pin)
}

/// Retrieve the enterprise attestation CSR from the authenticator.
pub fn get_enterprise_attestation_csr() -> Result<String, String> {
    fido::get_enterprise_attestation_csr()
}

/// Upload an X.509 certificate for enterprise attestation.
pub fn upload_enterprise_attestation_cert(
    pin: String,
    cert_path: String,
) -> Result<String, String> {
    fido::upload_enterprise_attestation_cert(pin, cert_path)
}
