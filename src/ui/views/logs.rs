use crate::ui::components::page_view::PageView;
use gpui::*;
use gpui_component::{
    ActiveTheme, Icon, StyledExt, button::Button, h_flex, scroll::ScrollableElement, v_flex,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LogType {
    Success,
    Error,
    Warning,
    Info,
}

#[derive(Clone, Debug)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: LogType,
    pub message: String,
}

pub struct LogsView {
    logs: Vec<LogEntry>,
}

impl LogsView {
    pub fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Self {
            logs: vec![
                LogEntry {
                    timestamp: "16:03:42".to_string(), // Placeholder
                    level: LogType::Info,
                    message: "Application started.".to_string(),
                },
                LogEntry {
                    timestamp: "16:03:42".to_string(),
                    level: LogType::Info,
                    message: "Attempting to connect to device...".to_string(),
                },
                LogEntry {
                    timestamp: "16:03:43".to_string(),
                    level: LogType::Success,
                    message: "Device Connected! Serial: 4490838745737CC0, FW: v7.2".to_string(),
                },
            ],
        }
    }

    fn clear(&mut self, cx: &mut Context<Self>) {
        self.logs.clear();
        cx.notify();
    }
}

impl Render for LogsView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let clear_listener = cx.listener(|this, _, _, cx| {
            this.clear(cx);
        });

        let theme = cx.theme();

        PageView::build(
            "System Logs",
            "Real-time device communication and application events.",
            v_flex()
                .gap_4()
                .h_full()
                .child(
                    h_flex().justify_end().child(
                        Button::new("clear_logs")
                            .label("Clear Logs")
                            .on_click(clear_listener),
                    ),
                )
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
                                .child(div().p_4().font_family("Mono").text_sm().child({
                                    let theme = theme.clone(); // Clone for closure
                                    v_flex().gap_1().children(self.logs.iter().map(move |log| {
                                        div()
                                            .flex()
                                            .gap_3()
                                            .child(
                                                div()
                                                    .text_color(theme.muted_foreground)
                                                    .flex_shrink_0()
                                                    .child(format!("[{}]", log.timestamp)),
                                            )
                                            .child(
                                                div()
                                                    .whitespace_normal()
                                                    .text_color(match log.level {
                                                        LogType::Success => gpui::green(),
                                                        LogType::Error => gpui::red(),
                                                        LogType::Warning => gpui::yellow(),
                                                        LogType::Info => gpui::white(),
                                                    })
                                                    .child(format!(
                                                        "{} {}",
                                                        match log.level {
                                                            LogType::Success => "➜",
                                                            LogType::Error => "✖",
                                                            LogType::Warning => "⚠",
                                                            LogType::Info => "",
                                                        },
                                                        log.message
                                                    )),
                                            )
                                    }))
                                }))
                                .into_any_element()
                        }),
                ),
            theme,
        )
    }
}
