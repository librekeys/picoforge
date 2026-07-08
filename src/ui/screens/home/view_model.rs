//! View model for the home screen — tracks device connection state and polling.

use crate::ui::app::AppModels;
use crate::ui::models::device::{DeviceEvent, DeviceRepo};
use gpui::*;

/// Application state and device-detection polling for the home screen.
pub struct HomeViewModel {
    pub device: Entity<DeviceRepo>,
}

impl HomeViewModel {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>, models: &AppModels) -> Self {
        let device = models.device.clone();
        cx.subscribe(&device, |_, _, _: &DeviceEvent, cx| cx.notify())
            .detach();
        Self { device }
    }
}
