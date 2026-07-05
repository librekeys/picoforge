use crate::hal::types::{FidoDeviceInfo, FullDeviceStatus, LedStatusConfig, ManagementAppConfig};
use gpui::*;

#[allow(dead_code)]
pub enum DeviceEvent {
    Updated,
}

impl EventEmitter<DeviceEvent> for DeviceRepo {}

pub struct DeviceRepo {
    pub status: Option<FullDeviceStatus>,
    pub fido_info: Option<FidoDeviceInfo>,
    pub led_status: Option<LedStatusConfig>,
    pub management_apps: Option<ManagementAppConfig>,
    pub error: Option<String>,
    pub loading: bool,
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
        }
    }

    // --- State lifecycle ---

    pub fn begin_load(&mut self) {
        self.loading = true;
        self.error = None;
    }

    pub fn end_load(&mut self) {
        self.loading = false;
    }

    pub fn is_loading(&self) -> bool {
        self.loading
    }

    // --- Field setters ---

    pub fn set_status(&mut self, status: FullDeviceStatus) -> bool {
        let changed = self
            .status
            .as_ref()
            .map(|s| s.info.serial != status.info.serial)
            .unwrap_or(true);
        self.status = Some(status);
        changed
    }

    pub fn set_fido_info(&mut self, fido: Option<FidoDeviceInfo>) {
        self.fido_info = fido;
    }

    pub fn set_auxiliary_data(
        &mut self,
        led: Option<LedStatusConfig>,
        mgmt: Option<ManagementAppConfig>,
    ) {
        self.led_status = led;
        self.management_apps = mgmt;
    }

    pub fn clear_auxiliary_data(&mut self) {
        self.led_status = None;
        self.management_apps = None;
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
