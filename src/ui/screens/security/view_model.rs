use crate::ui::app::AppModels;
use gpui::*;

pub struct SecurityViewModel;

impl SecurityViewModel {
    pub fn new(_window: &mut Window, _cx: &mut Context<Self>, _models: &AppModels) -> Self {
        Self
    }
}
