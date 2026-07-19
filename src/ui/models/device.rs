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

use crate::hal::firmwares::AnyFirmware;
use crate::hal::io;
use crate::hal::types;
use gpui::*;
use std::time::Duration;

/// How often the hot-plug watcher samples device presence. Only a *change*
/// triggers a refresh, so this is a detection-latency knob, not a poll cost.
const HOTPLUG_POLL_MS: u64 = 1000;

pub use crate::hal::rescue::constants::{
    LedColor, LedStatus, USB_CAP_FIDO2, USB_CAP_OATH, USB_CAP_OPENPGP, USB_CAP_OTP, USB_CAP_PIV,
    USB_CAP_U2F,
};
pub use types::{
    AppConfigInput, DeviceMethod, FidoDeviceInfo, FirmwareType, FullDeviceStatus, LedStatusConfig,
    StoredCredential,
};

// ── Events ──────────────────────────────────────────────────────────────────

/// Events emitted by [`DeviceRepo`] to notify subscribers of state changes.
pub enum DeviceEvent {
    /// Device details were refreshed.
    Updated,
}

impl EventEmitter<DeviceEvent> for DeviceRepo {}

// ── Snapshot returned by post-write state refresh ───────────────────────────

/// Snapshot of device state produced by a blocking HAL read.
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
    /// Handle to the hot-plug watcher task; dropped (cancelled) with the repo.
    hotplug_watch: Option<Task<()>>,
}

impl DeviceRepo {
    /// Create a new device repo in the disconnected state.
    pub fn new() -> Self {
        Self {
            status: None,
            fido_info: None,
            led_status: None,
            management_apps: None,
            error: None,
            loading: false,
            device_changed: false,
            hotplug_watch: None,
        }
    }

    // ── HAL static methods (blocking — call from background executor) ──────

    pub fn firmware_supports_legacy_fido_config(
        fw_type: &types::FirmwareType,
        version: &str,
    ) -> bool {
        AnyFirmware::new(fw_type.clone(), version).supports_legacy_fido_hardware_config()
    }

    pub fn read_device_state_blocking() -> Result<FreshDeviceState, crate::error::PFError> {
        let status = io::read_device_details()?;
        let (led_status, management_apps) = if status.firmware_type == types::FirmwareType::RSKey {
            (
                io::read_led_config(status.method.clone()).ok(),
                io::read_management_config(status.method.clone()).ok(),
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

    pub fn write_led_config_blocking(
        method: DeviceMethod,
        config: LedStatusConfig,
        pin: Option<String>,
    ) -> Result<String, crate::error::PFError> {
        io::write_led_config(method, config, pin)
    }

    pub fn write_management_config_blocking(
        method: DeviceMethod,
        enabled_mask: u16,
        pin: Option<String>,
    ) -> Result<String, crate::error::PFError> {
        io::write_management_config(method, enabled_mask, pin)
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
        crate::hal::transport::fido::HidTransport::open().is_ok()
    }

    /// Cheap, non-intrusive presence fingerprint of the attached FIDO device
    /// (`vid:pid:serial`, or `None` when absent). Enumerates only — does not
    /// open the device — so it is safe to poll from the hot-plug watcher.
    pub fn device_fingerprint_blocking() -> Option<String> {
        crate::hal::transport::fido::HidTransport::fingerprint()
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

    /// Start the hot-plug watcher: a background timer that samples the device
    /// fingerprint and, whenever it changes (plug / unplug / swap), triggers a
    /// [`refresh`](Self::refresh) so every screen reflects the current key with
    /// no manual Refresh. Idempotent — a second call is a no-op. The task is
    /// owned by the repo and cancelled when it is dropped.
    pub fn start_hotplug_watch(&mut self, cx: &mut Context<Self>) {
        if self.hotplug_watch.is_some() {
            return;
        }
        let weak = cx.entity().downgrade();
        self.hotplug_watch = Some(cx.spawn(async move |_, cx| {
            // Seed with the fingerprint the initial refresh already reflects so
            // the first tick doesn't re-read an already-loaded device.
            let mut last = cx
                .background_executor()
                .spawn(async { Self::device_fingerprint_blocking() })
                .await;
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(HOTPLUG_POLL_MS))
                    .await;
                let current = cx
                    .background_executor()
                    .spawn(async { Self::device_fingerprint_blocking() })
                    .await;
                if current == last {
                    continue;
                }
                // Re-read on the main thread. Skip while a refresh/write is in
                // flight and retry next tick (don't commit `last`, or we'd drop
                // the change). Break when the repo — and thus the app — is gone.
                let refreshed = weak.update(cx, |repo, cx| {
                    if repo.loading {
                        false
                    } else {
                        repo.refresh(cx);
                        true
                    }
                });
                match refreshed {
                    Ok(true) => last = current,
                    Ok(false) => {}
                    Err(_) => break,
                }
            }
        }));
    }

    /// Initiate a device-details refresh (async, emits [`DeviceEvent::Updated`] on completion).
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

                if status.firmware_type == types::FirmwareType::RSKey {
                    self.led_status = io::read_led_config(status.method.clone()).ok();
                    self.management_apps = io::read_management_config(status.method.clone()).ok();
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

    /// Mark the repo as loading.
    pub fn begin_load(&mut self) {
        self.loading = true;
        self.error = None;
    }

    /// Mark the repo as finished loading.
    pub fn end_load(&mut self) {
        self.loading = false;
    }

    /// Set an error state on the repo.
    pub fn set_error(&mut self, error: String) {
        self.status = None;
        self.fido_info = None;
        self.led_status = None;
        self.management_apps = None;
        self.loading = false;
        self.error = Some(error);
    }
}
