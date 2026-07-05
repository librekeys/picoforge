//! Sole gateway from the UI layer to the HAL.
//!
//! [`DeviceRepo`] is the **only** entity that imports from `crate::hal`.
//! Views and ViewModels must never import `crate::hal` directly — they
//! interact with the hardware exclusively through `DeviceRepo` methods
//! and the types re-exported below.
//!
//! # Architecture
//!
//! - **Blocking static methods** (`*_blocking`) wrap HAL I/O calls for use
//!   inside background tasks spawned by ViewModels.
//! - **`refresh()`** performs a full polling cycle and emits
//!   [`DeviceEvent::Updated`].
//! - **`apply_fresh_state()`** lets ViewModels push post-write HAL results
//!   back into the repo so subscribers get the event.

use crate::hal::io;
use crate::hal::types;
use gpui::*;

pub use crate::hal::rescue::constants::{
    LedColor, LedStatus, USB_CAP_FIDO2, USB_CAP_OATH, USB_CAP_OPENPGP, USB_CAP_OTP, USB_CAP_PIV,
    USB_CAP_U2F,
};
pub use types::{
    AppConfigInput, DeviceMethod, FidoDeviceInfo, FirmwareType, FullDeviceStatus, StoredCredential,
};

// ── Events ──────────────────────────────────────────────────────────────────

pub enum DeviceEvent {
    Updated,
}

impl EventEmitter<DeviceEvent> for DeviceRepo {}

// ── Snapshot returned by post-write state refresh ───────────────────────────

#[derive(Clone)]
pub struct FreshDeviceState {
    pub status: types::FullDeviceStatus,
    pub led_status: Option<types::LedStatusConfig>,
    pub management_apps: Option<types::ManagementAppConfig>,
}

// ── DeviceRepo ──────────────────────────────────────────────────────────────

pub struct DeviceRepo {
    pub status: Option<types::FullDeviceStatus>,
    pub fido_info: Option<types::FidoDeviceInfo>,
    pub led_status: Option<types::LedStatusConfig>,
    pub management_apps: Option<types::ManagementAppConfig>,
    pub error: Option<String>,
    pub loading: bool,
    pub device_changed: bool,
}

impl DeviceRepo {
    pub fn new() -> Self {
        Self {
            status: None,
            fido_info: None,
            led_status: None,
            management_apps: None,
            error: None,
            loading: false,
            device_changed: false,
        }
    }

    // ── HAL static methods (blocking — call from background executor) ──────

    pub fn firmware_supports_legacy_fido_config(version: &str) -> bool {
        crate::hal::fido::firmware_supports_legacy_fido_hardware_config(version)
    }

    pub fn read_device_state_blocking() -> Result<FreshDeviceState, crate::error::PFError> {
        let status = io::read_device_details()?;
        let (led_status, management_apps) = if status.firmware_type == types::FirmwareType::RSKey
            && status.method == types::DeviceMethod::Rescue
        {
            (
                io::read_led_config().ok(),
                io::read_management_config().ok(),
            )
        } else {
            (None, None)
        };
        Ok(FreshDeviceState {
            status,
            led_status,
            management_apps,
        })
    }

    pub fn write_config_blocking(
        config: types::AppConfigInput,
        method: types::DeviceMethod,
        pin: Option<String>,
    ) -> Result<String, crate::error::PFError> {
        io::write_config(config, method, pin)
    }

    pub fn write_led_status_blocking(
        status_idx: u8,
        color: u8,
        brightness: u8,
        steady: bool,
    ) -> Result<String, crate::error::PFError> {
        io::write_led_status(status_idx, color, brightness, steady)
    }

    pub fn write_management_config_blocking(
        enabled_mask: u16,
    ) -> Result<String, crate::error::PFError> {
        io::write_management_config(enabled_mask)
    }

    pub fn get_fido_info_blocking() -> Result<types::FidoDeviceInfo, String> {
        io::get_fido_info()
    }

    pub fn get_credentials_blocking(pin: String) -> Result<Vec<types::StoredCredential>, String> {
        io::get_credentials(pin)
    }

    pub fn delete_credential_blocking(
        pin: String,
        credential_id: String,
    ) -> Result<String, String> {
        io::delete_credential(pin, credential_id)
    }

    pub fn change_fido_pin_blocking(
        current: Option<String>,
        new: String,
    ) -> Result<String, String> {
        io::change_fido_pin(current, new)
    }

    pub fn set_min_pin_length_blocking(pin: String, min_len: u8) -> Result<String, String> {
        io::set_min_pin_length(pin, min_len)
    }

    pub fn get_enterprise_attestation_csr_blocking() -> Result<String, String> {
        io::get_enterprise_attestation_csr()
    }

    pub fn upload_enterprise_attestation_cert_blocking(
        pin: String,
        cert_path: String,
    ) -> Result<String, String> {
        io::upload_enterprise_attestation_cert(pin, cert_path)
    }

    pub fn enable_enterprise_attestation_blocking(pin: String) -> Result<String, String> {
        io::enable_enterprise_attestation(pin)
    }

    pub fn reset_device_blocking() -> Result<String, String> {
        io::reset_device()
    }

    pub fn read_device_serial_blocking() -> Option<String> {
        io::read_device_details().ok().map(|s| s.info.serial)
    }

    pub fn check_hid_available_blocking() -> bool {
        crate::hal::fido::hid::HidTransport::open().is_ok()
    }

    // ── State mutation (called from ViewModel after background work) ───────

    /// Push a freshly-read [`FreshDeviceState`] into the repo and emit
    /// [`DeviceEvent::Updated`]. Also updates `device_changed` if the
    /// serial number differs from the previous value.
    pub fn apply_fresh_state(&mut self, state: FreshDeviceState, cx: &mut Context<Self>) {
        let old_serial = self.status.as_ref().map(|s| s.info.serial.clone());
        self.device_changed = old_serial
            .as_ref()
            .map(|s| *s != state.status.info.serial)
            .unwrap_or(true);
        self.status = Some(state.status);
        self.led_status = state.led_status;
        self.management_apps = state.management_apps;
        self.fido_info = Self::get_fido_info_blocking().ok();
        cx.emit(DeviceEvent::Updated);
        cx.notify();
    }

    /// Re-read FIDO info from the device and emit [`DeviceEvent::Updated`].
    /// ViewModels should call this instead of manually setting `repo.fido_info`.
    pub fn update_fido_info(&mut self, cx: &mut Context<Self>) {
        self.fido_info = Self::get_fido_info_blocking().ok();
        cx.emit(DeviceEvent::Updated);
        cx.notify();
    }

    // ── Polling cycle ──────────────────────────────────────────────────────

    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        if self.loading {
            return;
        }

        self.begin_load();

        let old_serial = self.status.as_ref().map(|s| s.info.serial.clone());

        match io::read_device_details() {
            Ok(status) => {
                self.device_changed = old_serial
                    .as_ref()
                    .map(|s| *s != status.info.serial)
                    .unwrap_or(true);
                self.status = Some(status.clone());

                match io::get_fido_info() {
                    Ok(fido) => self.fido_info = Some(fido),
                    Err(e) => {
                        log::error!("FIDO Info fetch failed: {}", e);
                        self.fido_info = None;
                    }
                }

                if status.firmware_type == types::FirmwareType::RSKey
                    && status.method == types::DeviceMethod::Rescue
                {
                    self.led_status = io::read_led_config().ok();
                    self.management_apps = io::read_management_config().ok();
                } else {
                    self.led_status = None;
                    self.management_apps = None;
                }
            }
            Err(e) => {
                self.set_error(format!("{}", e));
                self.device_changed = false;
            }
        }

        self.end_load();
        cx.emit(DeviceEvent::Updated);
        cx.notify();
    }

    // ── State lifecycle helpers ────────────────────────────────────────────

    pub fn begin_load(&mut self) {
        self.loading = true;
        self.error = None;
    }

    pub fn end_load(&mut self) {
        self.loading = false;
    }

    pub fn set_error(&mut self, error: String) {
        self.status = None;
        self.fido_info = None;
        self.led_status = None;
        self.management_apps = None;
        self.loading = false;
        self.error = Some(error);
    }
}
