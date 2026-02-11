use gpui::*;
use gpui_component::{StyledExt, Theme, v_flex};

pub struct PageView;

impl PageView {
    pub fn build(
        title: impl Into<SharedString>,
        subtitle: impl Into<SharedString>,
        content: impl IntoElement,
        theme: &Theme,
    ) -> impl IntoElement {
        div()
            .size_full()
            .bg(theme.background)
            .flex()
            .flex_col()
            .items_center()
            .child(
                div().w_full().max_w(px(1200.0)).px_10().py_5().child(
                    v_flex()
                        .gap_8()
                        .child(
                            v_flex()
                                .child(
                                    div()
                                        .text_3xl()
                                        .font_extrabold()
                                        .text_color(theme.foreground)
                                        .child(title.into()),
                                )
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(theme.muted_foreground)
                                        .child(subtitle.into()),
                                ),
                        )
                        .child(content),
                ),
            )
    }
}
