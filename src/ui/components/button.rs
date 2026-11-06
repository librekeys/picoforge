//! Reusable styled button and icon-button components.

#![allow(unused)]

use crate::ui::colors;
use gpui_kit::Styled;
use gpui_kit::component::{
    ActiveTheme, Disableable, Icon, Sizable, Size,
    button::{Button, ButtonCustomVariant, ButtonVariants},
    h_flex,
};
use gpui_kit::prelude::*;
use gpui_kit::*;

/// PicoForge's original solid button palette, independent of Kit's translucent
/// custom-variant defaults. Kit still owns focus, activation and loading behavior.
#[derive(Clone, Copy)]
pub(crate) struct ButtonColors {
    background: Hsla,
    hover: Hsla,
    active: Hsla,
    foreground: Hsla,
    border: Hsla,
}

impl ButtonColors {
    pub(crate) fn new(cx: &App) -> Self {
        Self {
            background: cx.theme().transparent,
            hover: cx.theme().transparent,
            active: cx.theme().transparent,
            foreground: cx.theme().foreground,
            border: cx.theme().transparent,
        }
    }

    pub(crate) fn color(mut self, color: Hsla) -> Self {
        self.background = color;
        self
    }
    pub(crate) fn hover(mut self, color: Hsla) -> Self {
        self.hover = color;
        self
    }
    pub(crate) fn active(mut self, color: Hsla) -> Self {
        self.active = color;
        self
    }
    pub(crate) fn foreground(mut self, color: Hsla) -> Self {
        self.foreground = color;
        self
    }
    pub(crate) fn border(mut self, color: Hsla) -> Self {
        self.border = color;
        self
    }

    pub(crate) fn apply(self, button: Button, disabled: bool, cx: &App) -> Button {
        // Kit uses custom.color as the border in every pointer state. Supply the
        // border there and restore the independent solid background explicitly.
        // Disabled colors match the previous component's 15% background opacity.
        let background = if disabled {
            self.background.opacity(0.15)
        } else {
            self.background
        };
        let border = if disabled { background } else { self.border };
        button
            .with_variant(gpui_kit::component::button::ButtonVariant::Custom(
                ButtonCustomVariant::new(cx)
                    .color(border)
                    .hover(self.hover)
                    .active(self.active)
                    .foreground(self.foreground),
            ))
            .bg(background)
            .border_1()
            .border_color(border)
            .disabled(disabled)
    }
}

type ClickHandler = Option<Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>>;

/// A stateless text button wrapper
#[derive(IntoElement)]
pub struct PFButton {
    id: SharedString,
    text: SharedString,
    on_click: ClickHandler,
    bg_color_start: Rgba,
    bg_color_hover: Rgba,
    bg_color_active: Rgba,
    width_full: bool,
    centered: bool,
    disabled: bool,
    small: bool,
    loading: bool,
    text_color: Option<Rgba>,
}

impl PFButton {
    pub fn new(text: impl Into<SharedString>) -> Self {
        let text = text.into();
        Self {
            id: text.clone(),
            text,
            on_click: None,
            bg_color_start: rgb(0x1b1b1d),
            bg_color_hover: rgb(0x232325),
            bg_color_active: rgb(colors::zinc::ZINC700),
            width_full: false,
            centered: false,
            disabled: false,
            small: false,
            loading: false,
            text_color: None,
        }
    }

    pub fn id(mut self, id: impl Into<SharedString>) -> Self {
        self.id = id.into();
        self
    }

    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }

    /// Allows overriding the default colors
    pub fn with_colors(mut self, start: Rgba, hover: Rgba, active: Rgba) -> Self {
        self.bg_color_start = start;
        self.bg_color_hover = hover;
        self.bg_color_active = active;
        self
    }

    pub fn with_text_color(mut self, color: Rgba) -> Self {
        self.text_color = Some(color);
        self
    }

    pub fn section_header(mut self) -> Self {
        self.width_full = true;
        self.centered = false;
        self
    }

    pub fn w_full(mut self) -> Self {
        self.width_full = true;
        self
    }

    pub fn centered(mut self) -> Self {
        self.centered = true;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn small(mut self) -> Self {
        self.small = true;
        self
    }

    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }
}

impl RenderOnce for PFButton {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let text = self.text;

        let mut btn = Button::new(self.id)
            .map(|button| {
                ButtonColors::new(cx)
                    .color(self.bg_color_start.into())
                    .hover(self.bg_color_hover.into())
                    .active(self.bg_color_active.into())
                    .border(cx.theme().border)
                    .apply(button, self.disabled, cx)
            })
            .border_t_1();

        if self.width_full {
            btn = btn.w_full();
        }

        if self.disabled {
            btn = btn.disabled(true);
        }

        if self.loading {
            btn = btn.loading(true);
        }

        if self.small {
            btn = btn.with_size(Size::Small);
        }

        let content = if self.centered {
            h_flex().justify_center().child(text)
        } else {
            h_flex().child(text)
        };

        let content = if let Some(color) = self.text_color {
            content.text_color(color)
        } else {
            content
        };

        if let Some(handler) = self.on_click {
            btn = btn.on_click(move |e, w, c| handler(e, w, c));
        }

        btn.child(content)
    }
}

/// A stateless Icon + Text button wrapper
#[derive(IntoElement)]
pub struct PFIconButton {
    icon: Icon,
    text: SharedString,
    on_click: ClickHandler,
    bg_color_start: Rgba,
    bg_color_hover: Rgba,
    bg_color_active: Rgba,
    disabled: bool,
    small: bool,
    width_full: bool,
    loading: bool,
    text_color: Option<Rgba>,
}

impl PFIconButton {
    pub fn new(icon: impl Into<Icon>, text: impl Into<SharedString>) -> Self {
        Self {
            icon: icon.into(),
            text: text.into(),
            on_click: None,
            bg_color_start: rgb(0x1b1b1d),
            bg_color_hover: rgb(0x232325),
            bg_color_active: rgb(colors::zinc::ZINC700),
            disabled: false,
            small: false,
            width_full: false,
            loading: false,
            text_color: None,
        }
    }

    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }

    pub fn with_colors(mut self, start: Rgba, hover: Rgba, active: Rgba) -> Self {
        self.bg_color_start = start;
        self.bg_color_hover = hover;
        self.bg_color_active = active;
        self
    }

    pub fn with_text_color(mut self, color: Rgba) -> Self {
        self.text_color = Some(color);
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn small(mut self) -> Self {
        self.small = true;
        self
    }

    pub fn w_full(mut self) -> Self {
        self.width_full = true;
        self
    }

    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }
}

impl RenderOnce for PFIconButton {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let text = self.text;
        let icon = self.icon;

        let mut btn = Button::new("pf-icon-btn")
            .map(|button| {
                ButtonColors::new(cx)
                    .color(self.bg_color_start.into())
                    .hover(self.bg_color_hover.into())
                    .active(self.bg_color_active.into())
                    .border(cx.theme().border)
                    .apply(button, self.disabled, cx)
            })
            .border_t_2();

        if self.width_full {
            btn = btn.w_full();
        }

        if self.disabled {
            btn = btn.disabled(true);
        }

        if self.loading {
            btn = btn.loading(true);
        }

        if self.small {
            btn = btn.with_size(Size::Small);
        }

        if let Some(handler) = self.on_click {
            btn = btn.on_click(move |e, w, c| handler(e, w, c));
        }

        let content = h_flex().gap_2().justify_center().child(icon).child(text);
        let content = if let Some(color) = self.text_color {
            content.text_color(color)
        } else {
            content
        };

        btn.child(content)
    }
}
