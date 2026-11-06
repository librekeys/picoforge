//! View model for the about screen — version info and firmware compatibility.

use crate::ui::app::AppModels;
use gpui_kit::*;

/// Application metadata and firmware compatibility information.
pub struct AboutViewModel;

impl AboutViewModel {
    pub fn new(_window: &mut Window, _cx: &mut Context<Self>, _models: &AppModels) -> Self {
        Self
    }
}
