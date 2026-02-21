use gpui::*;

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
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let (bg, text) = if self.active {
            (rgb(0xffffff), rgb(0x000000))
        } else {
            (rgb(0x27272a), rgb(0xffffff))
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
