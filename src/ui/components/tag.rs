//! Simple tag/badge component for displaying status or labels.

use gpui_kit::component::ActiveTheme;
use gpui_kit::*;

/// A small badge showing a status or category label.
#[derive(IntoElement)]
pub struct Tag {
    label: SharedString,
    active: bool,
}

impl Tag {
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            active: false,
        }
    }

    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }
}

impl RenderOnce for Tag {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let (bg, text) = if self.active {
            (cx.theme().primary, cx.theme().primary_foreground)
        } else {
            (cx.theme().muted, cx.theme().foreground)
        };

        div()
            .px_2p5()
            .py_0p5()
            .rounded_xl()
            .text_xs()
            .font_weight(FontWeight::MEDIUM)
            .bg(bg)
            .text_color(text)
            .child(self.label)
    }
}
