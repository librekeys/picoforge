//! View model for the security screen — secure boot and attestation state.

use crate::ui::app::AppModels;
use gpui::*;

/// Security-related state — stub for secure boot, attestation, and reset operations.
pub struct SecurityViewModel;

impl SecurityViewModel {
    pub fn new(_window: &mut Window, _cx: &mut Context<Self>, _models: &AppModels) -> Self {
        Self
    }
}
