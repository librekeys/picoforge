// src/views/about.rs
use crate::ui::components::{card::Card, page_view::PageView, tag::Tag};
use gpui::*;
use gpui_component::{Icon, StyledExt, Theme, button::Button, h_flex, v_flex};

pub struct AboutView;

impl AboutView {
    pub fn build(theme: &Theme) -> impl IntoElement {
        PageView::build(
            "About",
            "Information about the application and its development.",
            div()
                .w_full()
                .flex()
                .justify_center()
                .child(
                    div()
                        .w_full()
                        .max_w(px(1200.0))
                        .child(
                            Card::new().child(
                                v_flex()
                                    .items_center()
                                    .gap_4()
                                    .py_8()
                                    .text_center()
                                    .child(
                                        img("appIcons/in.suyogtandel.picoforge.svg")
                                            .w(px(256.0))
                                            .h(px(256.0)),
                                    )
                                    .child(
                                        div()
                                            .text_2xl()
                                            .font_bold()
                                            .text_color(theme.foreground)
                                            .child("PicoForge"),
                                    )
                                    .child(Tag::new("v0.4.0"))
                                    .child(
                                        div()
                                            .text_color(theme.muted_foreground)
                                            .max_w(px(450.0))
                                            .child(
                                                "An open source commissioning tool for Pico FIDO security keys. Developed with Rust and GPUI.",
                                            ),
                                    )
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(theme.muted_foreground)
                                            .gap_1()
                                            .pt_4()
                                            .border_t_1()
                                            .border_color(theme.border)
                                            .border_t_1()
                                            .border_color(theme.border)
                                            .w(px(320.0))
                                            .child(
                                                h_flex()
                                                    .justify_between()
                                                    .items_start()
                                                    .child("Code By:")
                                                    .child(
                                                        v_flex()
                                                            .font_medium()
                                                            .text_color(theme.foreground)
                                                            .items_end()
                                                            .child("Suyog Tandel")
                                                            .child("Fabrice Bellamy")
                                                            .child("Jetcookies"),
                                                    ),
                                            )
                                            .child(
                                                h_flex()
                                                    .justify_between()
                                                    .items_center()
                                                    .pt_2()
                                                    .mt_2()
                                                    .child(h_flex().items_center().gap_1().child("Copyright:"))
                                                    .child(
                                                        div()
                                                            .font_medium()
                                                            .text_color(theme.foreground)
                                                            .child("© 2026 Suyog Tandel"),
                                                    ),
                                            ),
                                    )
                                    .child(
                                        h_flex()
                                            .gap_4()
                                            .pt_6()
                                            .child(
                                                Button::new("github_btn")
                                                    .outline()
                                                    .bg(rgb(0x222225))
                                                    .child(
                                                        h_flex()
                                                            .gap_2()
                                                            .child(
                                                                Icon::default()
                                                                    .path("icons/github.svg")
                                                                    .size_4(),
                                                            )
                                                            .child("GitHub"),
                                                    )
                                                    .on_click(|_, _, cx| {
                                                        cx.open_url("https://github.com/librekeys/picoforge")
                                                    }),
                                            ),
                                    ),
                            ),
                        ),
                ),
            theme,
        )
    }
}
