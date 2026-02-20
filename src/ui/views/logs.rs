use crate::logging::LOG_BUFFER;
use crate::ui::components::page_view::PageView;
use gpui::*;
use gpui_component::{
    ActiveTheme, Icon, button::Button, h_flex, scroll::ScrollableElement, v_flex,
};

pub struct LogsView {
    logs: Vec<String>,
}

impl LogsView {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let view_weak = cx.entity().downgrade();
        let mut cx_async = cx.to_async();

        cx.spawn(async move |_, _| {
            loop {
                cx_async
                    .background_executor()
                    .timer(std::time::Duration::from_millis(250))
                    .await;

                if let Some(view) = view_weak.upgrade() {
                    view.update(&mut cx_async, |view, cx| {
                        view.sync_logs();
                        cx.notify();
                    })
                    .ok();
                } else {
                    break;
                }
            }
        })
        .detach();

        Self { logs: Vec::new() }
    }

    fn sync_logs(&mut self) {
        if let Some(buffer) = LOG_BUFFER.get() {
            if let Ok(logs) = buffer.lock() {
                self.logs = logs.clone();
            }
        }
    }

    fn clear(&mut self, cx: &mut Context<Self>) {
        if let Some(buffer) = LOG_BUFFER.get() {
            if let Ok(mut logs) = buffer.lock() {
                logs.clear();
            }
        }
        self.logs.clear();
        cx.notify();
    }
}

impl Render for LogsView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let clear_logs_listener = cx.listener(|this, _, _, cx| {
            this.clear(cx);
        });

        let copy_logs_listener = cx.listener(|this, _, _, cx| {
            let all_logs = this.logs.join("\n");
            log::info!("Copying {} bytes of logs", all_logs.len());
            cx.write_to_clipboard(ClipboardItem::new_string(all_logs));
        });

        let theme = cx.theme();

        PageView::build(
            "System Logs",
            "Real-time device communication and application events.",
            v_flex()
                .gap_4()
                .h_full()
                .child(
                    div()
                        .flex_1()
                        .flex()
                        .flex_col()
                        .min_h(px(500.0))
                        .bg(gpui::black())
                        .border_1()
                        .border_color(theme.border)
                        .rounded(theme.radius)
                        .child(if self.logs.is_empty() {
                            div()
                                .flex_1()
                                .flex()
                                .flex_col()
                                .items_center()
                                .justify_center()
                                .text_color(theme.muted_foreground)
                                .child(
                                    Icon::default()
                                        .path("icons/terminal.svg")
                                        .size_12()
                                        .text_color(theme.muted_foreground)
                                        .opacity(0.5),
                                )
                                .child(div().mt_4().child("No events recorded yet."))
                                .into_any_element()
                        } else {
                            div()
                                .overflow_y_scrollbar()
                                .flex_1()
                                .h(px(500.0))
                                .child(
                                    div()
                                        .overflow_y_scrollbar()
                                        .max_h(px(500.0))
                                        .p_4()
                                        .font_family("Mono")
                                        .text_sm()
                                        .child(v_flex().gap_neg_4().children(
                                            self.logs.iter().map(|log| {
                                                // TODO: Convert these values to constants in colors.rs
                                                let color = if log.contains("ERROR") {
                                                    rgb(0xef4444)
                                                } else if log.contains("WARN") {
                                                    rgb(0xfde047)
                                                } else if log.contains("INFO") {
                                                    rgb(0x4ade80)
                                                } else {
                                                    theme.foreground.to_rgb()
                                                };

                                                div()
                                                    .text_color(color)
                                                    .child(log.clone())
                                                    .cursor_text()
                                            }),
                                        )),
                                )
                                .into_any_element()
                        }),
                )
                .child(
                    h_flex()
                        .justify_end()
                        .gap_2()
                        .child(
                            // NOTE: This does not work on linux as of now(tested on NIXOS-26_GNOME-49_WAYLAND)
                            Button::new("copy_logs")
                                .label("Copy Logs")
                                .on_click(copy_logs_listener),
                        )
                        .child(
                            Button::new("clear_logs")
                                .label("Clear Logs")
                                .on_click(clear_logs_listener),
                        ),
                ),
            theme,
        )
    }
}
