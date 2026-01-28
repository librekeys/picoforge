use crate::device::types::{DeviceMethod, FullDeviceStatus};
use crate::ui::{
    colors,
    views::{
        about::AboutView, config::ConfigView, home::HomeView, logs::LogsView,
        passkeys::PasskeysView, security::SecurityView,
    },
};
use gpui::*;
use gpui_component::{
    ActiveTheme, Icon, IconName, Side, TitleBar,
    button::{Button, ButtonVariants},
    h_flex,
    scroll::ScrollableElement,
    sidebar::*,
    v_flex,
};

#[derive(Clone, Copy, PartialEq)]
enum ActiveView {
    Home,
    Passkeys,
    Configuration,
    Security,
    Logs,
    About,
}

pub struct ApplicationRoot {
    active_view: ActiveView,
    collapsed: bool,
    device_status: Option<FullDeviceStatus>,
    device_loading: bool,
    device_error: Option<String>,
}

impl ApplicationRoot {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let mut this = Self {
            active_view: ActiveView::Home,
            collapsed: false,
            device_status: None,
            device_loading: false,
            device_error: None,
        };
        this.refresh_device_status(cx);
        this
    }

    fn refresh_device_status(&mut self, cx: &mut Context<Self>) {
        if self.device_loading {
            return;
        }

        self.device_loading = true;
        self.device_error = None;
        cx.notify();

        // TODO: Enable async refresh once WeakView/handle type is resolved
        self.device_loading = false;
    }
}

impl Render for ApplicationRoot {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let device_status = self.device_status.clone();

        let device_error = self.device_error.clone();
        let collapsed = self.collapsed;

        h_flex()
            .size_full()
            .child(
                v_flex()
                    .h_full()
                    .bg(rgb(colors::zinc::ZINC900))
                    .w(if self.collapsed { px(48.) } else { px(255.) })
                    .child({
                        let header = h_flex()
                            .w_full()
                            .items_center()
                            .bg(rgb(colors::zinc::ZINC900))
                            .border_r_1()
                            .border_color(cx.theme().sidebar_border)
                            .pt_4();

                        let header = if self.collapsed {
                            header.justify_center()
                        } else {
                            header.justify_start().pl_4()
                        };

                        header
                            .child(
                                img("appIcons/in.suyogtandel.picoforge.svg")
                                    .w(if self.collapsed { px(32.) } else { px(48.) })
                                    .h(if self.collapsed { px(32.) } else { px(48.) }),
                            )
                            .children(if !self.collapsed {
                                Some(
                                    div()
                                        .ml_2()
                                        .child("PicoForge")
                                        .font_weight(gpui::FontWeight::EXTRA_BOLD)
                                        .text_color(rgb(colors::zinc::ZINC100)),
                                )
                            } else {
                                None
                            })
                    })
                    .child(
                        Sidebar::new(Side::Left)
                            .collapsed(self.collapsed)
                            .collapsible(true)
                            .h_auto()
                            .w_full()
                            .flex_grow()
                            .bg(rgb(colors::zinc::ZINC900))
                            .child(
                                SidebarGroup::new("Menu").child(
                                    SidebarMenu::new()
                                        .child(
                                            SidebarMenuItem::new("Home")
                                                .icon(Icon::default().path("icons/house.svg"))
                                                .active(self.active_view == ActiveView::Home)
                                                .on_click(cx.listener(|this, _, _, _| {
                                                    this.active_view = ActiveView::Home;
                                                })),
                                        )
                                        .child(
                                            SidebarMenuItem::new("Passkeys")
                                                .icon(Icon::default().path("icons/key-round.svg"))
                                                .active(self.active_view == ActiveView::Passkeys)
                                                .on_click(cx.listener(|this, _, _, _| {
                                                    this.active_view = ActiveView::Passkeys;
                                                })),
                                        )
                                        .child(
                                            SidebarMenuItem::new("Configuration")
                                                .icon(Icon::default().path("icons/settings.svg"))
                                                .active(
                                                    self.active_view == ActiveView::Configuration,
                                                )
                                                .on_click(cx.listener(|this, _, _, _| {
                                                    this.active_view = ActiveView::Configuration;
                                                })),
                                        )
                                        .child(
                                            SidebarMenuItem::new("Security")
                                                .icon(
                                                    Icon::default().path("icons/shield-check.svg"),
                                                )
                                                .active(self.active_view == ActiveView::Security)
                                                .on_click(cx.listener(|this, _, _, _| {
                                                    this.active_view = ActiveView::Security;
                                                })),
                                        )
                                        .child(
                                            SidebarMenuItem::new("Logs")
                                                .icon(Icon::default().path("icons/scroll-text.svg"))
                                                .active(self.active_view == ActiveView::Logs)
                                                .on_click(cx.listener(|this, _, _, _| {
                                                    this.active_view = ActiveView::Logs;
                                                })),
                                        )
                                        .child(
                                            SidebarMenuItem::new("About")
                                                .icon(IconName::Info)
                                                .active(self.active_view == ActiveView::About)
                                                .on_click(cx.listener(|this, _, _, _| {
                                                    this.active_view = ActiveView::About;
                                                })),
                                        ),
                                ),
                            ),
                    )
                    .child(
                        v_flex()
                            .w_full()
                            .bg(rgb(0x111113))
                            .border_r_1()
                            .border_color(cx.theme().sidebar_border)
                            .p_2()
                            .gap_3()
                            .child(if collapsed {
                                // Collapsed View
                                v_flex()
                                    .items_center()
                                    .justify_center()
                                    .gap_2()
                                    .child(
                                        Button::new("refresh-btn-collapsed")
                                            .ghost()
                                            .child(Icon::default().path("icons/refresh-cw.svg"))
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.refresh_device_status(cx);
                                            })),
                                    )
                                    .child(div().w(px(8.)).h(px(8.)).rounded_full().bg(
                                        if let Some(status) = &device_status {
                                            if status.method == DeviceMethod::Fido {
                                                rgb(0xf59e0b)
                                            } else {
                                                rgb(0x22c55e)
                                            }
                                        } else if device_error.is_some() {
                                            rgb(0xf59e0b)
                                        } else {
                                            rgb(0xef4444)
                                        },
                                    ))
                            } else {
                                // Expanded View
                                v_flex()
                                    .gap_3()
                                    .child(
                                        h_flex()
                                            .items_center()
                                            .justify_between()
                                            .child(
                                                div()
                                                    .text_size(px(12.))
                                                    .font_weight(gpui::FontWeight::MEDIUM)
                                                    .text_color(cx.theme().muted_foreground)
                                                    .child("Device Status"),
                                            )
                                            .child({
                                                let (text, color_bg, color_text) =
                                                    if let Some(status) = &device_status {
                                                        if status.method == DeviceMethod::Fido {
                                                            (
                                                                "Online - Fido",
                                                                rgb(0xf59e0b),
                                                                rgb(0xffffff),
                                                            )
                                                        } else {
                                                            ("Online", rgb(0x16a34a), rgb(0xffffff))
                                                        }
                                                    } else if device_error.is_some() {
                                                        ("Error", rgb(0xd97706), rgb(0xffffff))
                                                    } else {
                                                        ("Offline", rgb(0xef4444), rgb(0xffffff))
                                                    };

                                                div()
                                                    .px(px(6.))
                                                    .h(px(20.))
                                                    .flex()
                                                    .items_center()
                                                    .rounded(px(10.))
                                                    .bg(color_bg)
                                                    .child(
                                                        div()
                                                            .text_size(px(10.))
                                                            .font_weight(gpui::FontWeight::BOLD)
                                                            .text_color(color_text)
                                                            .child(text),
                                                    )
                                            }),
                                    )
                                    .child(
                                        Button::new("refresh-btn")
                                            .outline()
                                            .w_full()
                                            .child(
                                                h_flex()
                                                    .gap_2()
                                                    .justify_center()
                                                    .child(
                                                        Icon::default()
                                                            .path("icons/refresh-cw.svg"),
                                                    )
                                                    .child("Refresh"),
                                            )
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.refresh_device_status(cx);
                                            })),
                                    )
                            }),
                    ),
            )
            .child(
                v_flex()
                    .size_full()
                    .child(
                        TitleBar::new().bg(rgba(colors::zinc::ZINC950)).child(
                            h_flex()
                                .w_full()
                                .justify_between()
                                .bg(rgba(colors::zinc::ZINC950))
                                .items_center()
                                .cursor(gpui::CursorStyle::OpenHand)
                                .child(
                                    Button::new("sidebar_toggle")
                                        .ghost()
                                        .icon(IconName::PanelLeft)
                                        .on_click(cx.listener(|this, _, _, _| {
                                            this.collapsed = !this.collapsed;
                                        }))
                                        .tooltip("Toggle Sidebar"),
                                ),
                        ),
                    )
                    .child(
                        v_flex()
                            .min_h(px(0.))
                            .min_w(px(0.))
                            .overflow_y_scrollbar()
                            .flex_grow()
                            .bg(cx.theme().background)
                            .child(match self.active_view {
                                ActiveView::Home => HomeView::build(cx.theme()).into_any_element(),
                                ActiveView::Passkeys => PasskeysView::build().into_any_element(),
                                ActiveView::Configuration => ConfigView::build().into_any_element(),
                                ActiveView::Security => SecurityView::build().into_any_element(),
                                ActiveView::Logs => LogsView::build().into_any_element(),
                                ActiveView::About => AboutView::build().into_any_element(),
                            }),
                    ),
            )
    }
}
