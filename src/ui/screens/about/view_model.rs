use crate::ui::app::AppModels;
use gpui::*;

pub struct AboutViewModel;

impl AboutViewModel {
    pub fn new(_window: &mut Window, _cx: &mut Context<Self>, _models: &AppModels) -> Self {
        Self
    }
}
