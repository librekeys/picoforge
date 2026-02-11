#![allow(unused)]

use gpui::*;
use gpui_component::{ActiveTheme, Icon, Theme, h_flex, v_flex};

#[derive(IntoElement)]
pub struct Card {
    title: Option<SharedString>,
    description: Option<SharedString>,
    icon: Option<Icon>,
    header_right: Option<AnyElement>,
    children: Vec<AnyElement>,
}

impl Card {
    pub fn new() -> Self {
        Self {
            title: None,
            description: None,
            icon: None,
            header_right: None,
            children: Vec::new(),
        }
    }

    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn header_right(mut self, element: impl IntoElement) -> Self {
        self.header_right = Some(element.into_any_element());
        self
    }
}

impl ParentElement for Card {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Card {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme: &Theme = cx.theme();

        let has_header = self.title.is_some()
            || self.icon.is_some()
            || self.description.is_some()
            || self.header_right.is_some();

        let header = if has_header {
            let mut left_side = v_flex().gap_1();

            let mut icon_title_row = h_flex().items_center().gap_2();
            if let Some(icon) = self.icon {
                icon_title_row =
                    icon_title_row.child(Icon::new(icon).size_5().text_color(theme.foreground));
            }
            if let Some(title) = self.title {
                icon_title_row = icon_title_row.child(
                    div()
                        .font_weight(FontWeight::BOLD)
                        .text_color(theme.foreground)
                        .child(title),
                );
            }

            left_side = left_side.child(icon_title_row);

            if let Some(desc) = self.description {
                left_side = left_side.child(
                    div()
                        .text_sm()
                        .text_color(theme.muted_foreground)
                        .child(desc),
                );
            }

            let mut header_row = h_flex().items_center().justify_between().child(left_side);

            if let Some(right) = self.header_right {
                header_row = header_row.child(right);
            }

            Some(header_row.into_any_element())
        } else {
            None
        };

        div()
            .w_full()
            .bg(rgb(0x18181b))
            .border_1()
            .border_color(theme.border)
            .rounded_xl()
            .p_6()
            .child(v_flex().gap_6().children(header).children(self.children))
    }
}
