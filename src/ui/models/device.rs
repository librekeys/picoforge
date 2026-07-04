use crate::hal::types::{FidoDeviceInfo, FullDeviceStatus, LedStatusConfig, ManagementAppConfig};
use gpui::*;

// Repo Events:
pub enum DeviceEvent {
    Updated,
}

impl EventEmitter<DeviceEvent> for DeviceRepo {}

// Repo Definition:
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
}
