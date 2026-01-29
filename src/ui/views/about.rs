// src/views/about.rs
use gpui::*;
use gpui_component::{Icon, StyledExt, Theme, badge::Badge, button::Button, h_flex, v_flex};

pub struct AboutView;

impl AboutView {
    pub fn build(theme: &Theme) -> impl IntoElement {
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
                                        .child("About"),
                                )
                                .child(
                                    div().text_sm().text_color(theme.muted_foreground).child(
                                        "Information about the application and its development.",
                                    ),
                                ),
                        )
                        .child(
                            div()
                                .w_full()
                                .flex()
                                .justify_center()
                                .child(
                                    div()
                                        .max_w(px(1200.0))
                                        .w_full()
                                        .bg(rgb(0x18181b))
                                        .border_1()
                                        .border_color(theme.border)
                                        .rounded_xl()
                                        .p_6()
                                        .child(
                                            v_flex()
                                                .items_center()
                                                .justify_center()
                                                .gap_4()
                                                .py_8()
                                                .text_center()
                                                .child(
                                                    img("appIcons/in.suyogtandel.picoforge.svg")
                                                        .w(px(256.0))
                                                        .h(px(256.0))
                                                )
                                                .child(
                                                    div()
                                                        .text_2xl()
                                                        .font_bold()
                                                        .text_color(theme.foreground)
                                                        .child("PicoForge"),
                                                )
                                                .child(
                                                    Badge::new()
                                                        .child("v0.4.0")
                                                        .color(theme.secondary),
                                                )
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
                                                                .child("Code By:")
                                                                .child(
                                                                    div()
                                                                        .font_medium()
                                                                        .text_color(theme.foreground)
                                                                        .child("Suyog Tandel"),
                                                                ),
                                                        )
                                                        .child(
                                                            h_flex()
                                                                .justify_between()
                                                                .items_center()
                                                                .pt_2()
                                                                .mt_2()
                                                                .child(
                                                                    h_flex()
                                                                        .items_center()
                                                                        .gap_1()
                                                                        .child("Copyright:"),
                                                                )
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
                                                                .child(
                                                                    h_flex()
                                                                        .gap_2()
                                                                        .child(
                                                                             Icon::default().path("icons/github.svg").size_4()
                                                                        )
                                                                        .child("GitHub")
                                                                )
                                                                .on_click(|_, _, cx| {
                                                                    cx.open_url("https://github.com/librekeys/picoforge")
                                                                }),
                                                        ),
                                                ),
                                        ),
                                ),
                        ),
                ),
            )
    }
}
