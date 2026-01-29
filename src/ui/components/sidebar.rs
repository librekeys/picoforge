use crate::device::types::{DeviceMethod, FullDeviceStatus};
use crate::ui::colors;
use gpui::*;
use gpui_component::{
    ActiveTheme, Icon, IconName, Side,
    button::{Button, ButtonVariants},
    h_flex,
    sidebar::*,
    v_flex,
};
use std::rc::Rc;

#[derive(Clone, Copy, PartialEq)]
pub enum ActiveView {
    Home,
    Passkeys,
    Configuration,
    Security,
    Logs,
    About,
}

pub struct AppSidebar<V: 'static> {
    active_view: ActiveView,
    width: Pixels,
    collapsed: bool,
    device_status: Option<FullDeviceStatus>,
    device_error: Option<String>,
    on_select: Option<Rc<dyn Fn(&mut V, ActiveView, &mut Window, &mut Context<V>)>>,
    on_refresh: Option<Rc<dyn Fn(&mut V, &mut Window, &mut Context<V>)>>,
}

impl<V: 'static> AppSidebar<V> {
    pub fn new(
        active_view: ActiveView,
        width: Pixels,
        collapsed: bool,
        device_status: Option<FullDeviceStatus>,
        device_error: Option<String>,
    ) -> Self {
        Self {
            active_view,
            width,
            collapsed,
            device_status,
            device_error,
            on_select: None,
            on_refresh: None,
        }
    }

    pub fn on_select(
        mut self,
        handler: impl Fn(&mut V, ActiveView, &mut Window, &mut Context<V>) + 'static,
    ) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }

    pub fn on_refresh(
        mut self,
        handler: impl Fn(&mut V, &mut Window, &mut Context<V>) + 'static,
    ) -> Self {
        self.on_refresh = Some(Rc::new(handler));
        self
    }

    pub fn render(self, cx: &mut Context<V>) -> impl IntoElement {
        let width = self.width;
        let collapsed = self.collapsed;
        let device_status = self.device_status.clone();
        let device_error = self.device_error.clone();

        let border_color = cx.theme().sidebar_border;
        let muted_foreground = cx.theme().muted_foreground;

        v_flex()
            .h_full()
            .bg(rgb(colors::zinc::ZINC900))
            .w(width)
            .child({
                let header = h_flex()
                    .w_full()
                    .items_center()
                    .bg(rgb(colors::zinc::ZINC900))
                    .border_r_1()
                    .border_color(border_color)
                    .pt_4();

                let current_width = width;
                let t = ((current_width - px(48.)) / (px(255.) - px(48.))).clamp(0.0, 1.0);

                // Animate padding-left from 8px (collapsed) to 16px (expanded)
                let padding_left = px(8.) + (px(16.) - px(8.)) * t;

                let header = header.justify_start().pl(padding_left);

                let width_icon_start = px(120.);
                let t_icon =
                    ((current_width - px(48.)) / (width_icon_start - px(48.))).clamp(0.0, 1.0);
                let icon_size = px(32.) + (px(48.) - px(32.)) * t_icon;

                let width_text_start = px(200.);
                let text_opacity: f32 = if current_width > width_text_start {
                    ((current_width - width_text_start) / (px(255.) - width_text_start))
                        .clamp(0.0, 1.0)
                } else {
                    0.0
                };

                header
                    .child(
                        img("appIcons/in.suyogtandel.picoforge.svg")
                            .w(icon_size)
                            .h(icon_size)
                            .flex_shrink_0(),
                    )
                    .children(if width > px(60.) {
                        Some(
                            div()
                                .ml_2()
                                .opacity(text_opacity)
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
                    .collapsed(width < px(120.))
                    .collapsible(false)
                    .h_auto()
                    .w_full()
                    .flex_grow()
                    .bg(rgb(colors::zinc::ZINC900))
                    .child(
                        SidebarGroup::new("Menu").child(
                            SidebarMenu::new()
                                .child(self.menu_item(
                                    cx,
                                    "Home",
                                    "icons/house.svg",
                                    ActiveView::Home,
                                ))
                                .child(self.menu_item(
                                    cx,
                                    "Passkeys",
                                    "icons/key-round.svg",
                                    ActiveView::Passkeys,
                                ))
                                .child(self.menu_item(
                                    cx,
                                    "Configuration",
                                    "icons/settings.svg",
                                    ActiveView::Configuration,
                                ))
                                .child(self.menu_item(
                                    cx,
                                    "Security",
                                    "icons/shield-check.svg",
                                    ActiveView::Security,
                                ))
                                .child(self.menu_item(
                                    cx,
                                    "Logs",
                                    "icons/scroll-text.svg",
                                    ActiveView::Logs,
                                ))
                                .child(self.menu_item_icon_name(
                                    cx,
                                    "About",
                                    IconName::Info,
                                    ActiveView::About,
                                )),
                        ),
                    ),
            )
            .child(
                v_flex()
                    .w_full()
                    .bg(rgb(0x111113))
                    .border_r_1()
                    .border_color(border_color)
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
                                    .on_click(cx.listener({
                                        let on_refresh = self.on_refresh.clone();
                                        move |view, _, window, cx| {
                                            if let Some(f) = &on_refresh {
                                                f(view, window, cx)
                                            }
                                        }
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
                                            .text_color(muted_foreground)
                                            .child("Device Status"),
                                    )
                                    .child({
                                        let (text, color_bg, color_text) =
                                            if let Some(status) = &device_status {
                                                if status.method == DeviceMethod::Fido {
                                                    ("Online - Fido", rgb(0xf59e0b), rgb(0xffffff))
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
                                            .child(Icon::default().path("icons/refresh-cw.svg"))
                                            .child("Refresh"),
                                    )
                                    .on_click(cx.listener({
                                        let on_refresh = self.on_refresh.clone();
                                        move |view, _, window, cx| {
                                            if let Some(f) = &on_refresh {
                                                f(view, window, cx)
                                            }
                                        }
                                    })),
                            )
                    }),
            )
    }

    fn menu_item(
        &self,
        cx: &mut Context<V>,
        label: &'static str,
        icon_path: &'static str,
        view: ActiveView,
    ) -> SidebarMenuItem {
        let on_select = self.on_select.clone();
        SidebarMenuItem::new(label)
            .icon(Icon::default().path(icon_path))
            .active(self.active_view == view)
            .on_click(cx.listener(move |element, _, window, cx| {
                if let Some(f) = &on_select {
                    f(element, view, window, cx);
                }
            }))
    }

    fn menu_item_icon_name(
        &self,
        cx: &mut Context<V>,
        label: &'static str,
        icon: IconName,
        view: ActiveView,
    ) -> SidebarMenuItem {
        let on_select = self.on_select.clone();
        SidebarMenuItem::new(label)
            .icon(icon)
            .active(self.active_view == view)
            .on_click(cx.listener(move |element, _, window, cx| {
                if let Some(f) = &on_select {
                    f(element, view, window, cx);
                }
            }))
    }
}
