use crate::{
    error::PFError,
    hal::{fido, rescue, types::*},
};

pub fn read_device_details() -> Result<FullDeviceStatus, PFError> {
    let mut fido_status: Option<FullDeviceStatus> = None;
    let mut rescue_status: Option<FullDeviceStatus> = None;

    match fido::read_device_details() {
        Ok(status) => {
            log::info!("FIDO device details read successfully");
            fido_status = Some(status);
        }
        Err(e) => log::warn!("FIDO read_device_details failed: {}", e),
    }

    match rescue::read_device_details() {
        Ok(status) => {
            log::info!("Rescue device details read successfully");
            rescue_status = Some(status);
        }
        Err(e) => log::warn!("Rescue read_device_details failed: {}", e),
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
                method: DeviceMethod::Fido,
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
            Ok(rescue)
        }
        (None, None) => {
            log::error!("Failed to read device details via both FIDO and Rescue");
            Err(PFError::NoDevice)
        }
    }
}

#[allow(dead_code)]
pub fn enable_secure_boot(lock: bool) -> Result<String, PFError> {
    rescue::enable_secure_boot(lock)
}

#[allow(dead_code)]
pub fn reboot(to_bootsel: bool) -> Result<String, PFError> {
    rescue::reboot_device(to_bootsel)
}

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

pub fn read_led_config() -> Result<LedStatusConfig, PFError> {
    rescue::read_led_config()
}

pub fn write_led_status(
    status: u8,
    color: u8,
    brightness: u8,
    steady: bool,
) -> Result<String, PFError> {
    rescue::write_led_status(status, color, brightness, steady)
}

pub fn read_management_config() -> Result<ManagementAppConfig, PFError> {
    rescue::read_management_config()
}

pub fn write_management_config(enabled_mask: u16) -> Result<String, PFError> {
    rescue::write_management_config(enabled_mask)
}

pub(crate) fn get_fido_info() -> Result<FidoDeviceInfo, String> {
    fido::get_fido_info()
}

pub(crate) fn change_fido_pin(
    current_pin: Option<String>,
    new_pin: String,
) -> Result<String, String> {
    fido::change_fido_pin(current_pin, new_pin)
}

pub(crate) fn set_min_pin_length(
    current_pin: String,
    min_pin_length: u8,
) -> Result<String, String> {
    fido::set_min_pin_length(current_pin, min_pin_length)
}

pub fn get_credentials(pin: String) -> Result<Vec<StoredCredential>, String> {
    fido::get_credentials(pin)
}

pub fn delete_credential(pin: String, credential_id: String) -> Result<String, String> {
    fido::delete_credential(pin, credential_id)
}

pub fn reset_device() -> Result<String, String> {
    fido::reset_device()
}

pub fn enable_enterprise_attestation(pin: String) -> Result<String, String> {
    fido::enable_enterprise_attestation(pin)
}

pub fn get_enterprise_attestation_csr() -> Result<String, String> {
    fido::get_enterprise_attestation_csr()
}

pub fn upload_enterprise_attestation_cert(
    pin: String,
    cert_path: String,
) -> Result<String, String> {
    fido::upload_enterprise_attestation_cert(pin, cert_path)
}
