use crate::ui::app::Destination;
use crate::ui::components::button::PFIconButton;
use crate::ui::models::device::{DeviceMethod, DeviceRepo, FirmwareType};
use gpui::*;
use gpui_component::{
    ActiveTheme, Icon, IconName, Side,
    button::{Button, ButtonVariants},
    h_flex,
    sidebar::*,
    v_flex,
};

/// Events emitted by [`AppSidebar`] to its subscribers.
pub enum SidebarEvent {
    /// Navigate to the given destination screen.
    Navigate(Destination),
    /// Re-poll device hardware.
    RefreshDevice,
}

impl EventEmitter<SidebarEvent> for AppSidebar {}

/// Self-contained navigation sidebar. Owns its own collapse state, width animation,
/// and toggle hover state. The toggle button is rendered separately in
/// [`ApplicationRoot`](crate::ui::app::ApplicationRoot) as the last child of `main-area`
/// so it paints on top of the content area.
pub struct AppSidebar {
    pub(crate) collapsed: bool,
    pub(crate) toggle_hovered: bool,
    current_width: Pixels,
    active_destination: Destination,
    device: Entity<DeviceRepo>,
}

impl AppSidebar {
    pub fn new(active_destination: Destination, device: Entity<DeviceRepo>) -> Self {
        Self {
            collapsed: false,
            toggle_hovered: false,
            current_width: px(255.),
            active_destination,
            device,
        }
    }

    pub fn current_width(&self) -> Pixels {
        self.current_width
    }

    pub fn collapsed(&self) -> bool {
        self.collapsed
    }

    pub fn toggle_hovered(&self) -> bool {
        self.toggle_hovered
    }

    /// Update which nav item is highlighted.
    pub fn set_active_destination(&mut self, dest: Destination) {
        self.active_destination = dest;
    }

    fn menu_item(
        &self,
        cx: &mut Context<Self>,
        label: &'static str,
        icon_path: &'static str,
        dest: Destination,
    ) -> SidebarMenuItem {
        SidebarMenuItem::new(label)
            .icon(Icon::default().path(icon_path))
            .active(self.active_destination == dest)
            .on_click(cx.listener(move |_, _, _, cx| {
                cx.emit(SidebarEvent::Navigate(dest));
            }))
    }

    fn menu_item_icon_name(
        &self,
        cx: &mut Context<Self>,
        label: &'static str,
        icon: IconName,
        dest: Destination,
    ) -> SidebarMenuItem {
        SidebarMenuItem::new(label)
            .icon(icon)
            .active(self.active_destination == dest)
            .on_click(cx.listener(move |_, _, _, cx| {
                cx.emit(SidebarEvent::Navigate(dest));
            }))
    }
}

impl Render for AppSidebar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_window_wide = window.bounds().size.width > px(800.0);
        let is_collapsed = self.collapsed || !is_window_wide;

        let target_width = if is_collapsed { px(48.) } else { px(255.) };

        if (self.current_width - target_width).abs() > px(0.1) {
            self.current_width = self.current_width + (target_width - self.current_width) * 0.2;
            window.request_animation_frame();
        } else {
            self.current_width = target_width;
        }

        let state = self.device.read(cx);
        let status_owned = state.status.clone();
        let error_owned = state.error.clone();

        let sidebar_bg = cx.theme().sidebar;
        let sidebar_fg = cx.theme().sidebar_foreground;
        let border_color = cx.theme().sidebar_border;
        let muted_foreground = cx.theme().muted_foreground;

        let sidebar_width = self.current_width;
        let collapsed = is_collapsed;

        // ── Header ────────────────────────────────────────────────────
        let header = {
            let t = ((sidebar_width - px(48.)) / (px(255.) - px(48.))).clamp(0.0, 1.0);
            let padding_left = px(8.) + (px(16.) - px(8.)) * t;

            let width_icon_start = px(120.);
            let t_icon = ((sidebar_width - px(48.)) / (width_icon_start - px(48.))).clamp(0.0, 1.0);
            let icon_size = px(32.) + (px(48.) - px(32.)) * t_icon;

            let width_text_start = px(200.);
            let text_opacity: f32 = if sidebar_width > width_text_start {
                ((sidebar_width - width_text_start) / (px(255.) - width_text_start)).clamp(0.0, 1.0)
            } else {
                0.0
            };

            h_flex()
                .w_full()
                .items_center()
                .bg(sidebar_bg)
                .pt_4()
                .justify_start()
                .pl(padding_left)
                .child(
                    img("appIcons/in.suyogtandel.picoforge.svg")
                        .w(icon_size)
                        .h(icon_size)
                        .flex_shrink_0(),
                )
                .children(if sidebar_width > px(60.) {
                    Some(
                        div()
                            .ml_2()
                            .opacity(text_opacity)
                            .child("PicoForge")
                            .font_weight(gpui::FontWeight::EXTRA_BOLD)
                            .text_color(sidebar_fg),
                    )
                } else {
                    None
                })
        };

        // ── Navigation items (gpui-component Sidebar) ────────────────
        let nav_sidebar = Sidebar::new(Side::Left)
            .collapsed(sidebar_width < px(120.))
            .collapsible(false)
            .h_auto()
            .w_full()
            .flex_grow()
            .bg(sidebar_bg)
            .border_color(gpui::transparent_white())
            .child(
                SidebarGroup::new("Menu").child(
                    SidebarMenu::new()
                        .child(self.menu_item(cx, "Home", "icons/house.svg", Destination::Home))
                        .child(self.menu_item(
                            cx,
                            "Passkeys",
                            "icons/key-round.svg",
                            Destination::Passkeys,
                        ))
                        .child(self.menu_item(
                            cx,
                            "Configuration",
                            "icons/settings.svg",
                            Destination::Configuration,
                        ))
                        .child(self.menu_item(
                            cx,
                            "Security",
                            "icons/shield-check.svg",
                            Destination::Security,
                        ))
                        .child(self.menu_item_icon_name(
                            cx,
                            "About",
                            IconName::Info,
                            Destination::About,
                        )),
                ),
            );

        // ── Footer (device status + refresh) ─────────────────────────
        let footer = v_flex()
            .w_full()
            .bg(rgb(0x111113))
            .mt_1()
            .border_t_1()
            .border_color(border_color)
            .p_2()
            .gap_3()
            .child(if collapsed {
                v_flex()
                    .items_center()
                    .justify_center()
                    .gap_2()
                    .child(
                        Button::new("refresh-btn-collapsed")
                            .ghost()
                            .child(Icon::default().path("icons/refresh-cw.svg"))
                            .on_click(cx.listener(|_, _, _, cx| {
                                cx.emit(SidebarEvent::RefreshDevice);
                            }))
                            .w_full(),
                    )
                    .child(div().w(px(8.)).h(px(8.)).rounded_full().bg(
                        if let Some(s) = &status_owned {
                            if s.method == DeviceMethod::Fido {
                                rgb(0xf59e0b)
                            } else {
                                rgb(0x22c55e)
                            }
                        } else if error_owned.is_some() {
                            rgb(0xf59e0b)
                        } else {
                            rgb(0xef4444)
                        },
                    ))
            } else {
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
                                let (text, color_bg, color_text) = if let Some(s) = &status_owned {
                                    let is_rskey = s.firmware_type == FirmwareType::RSKey;
                                    let fw_label = if is_rskey { "RS-Key" } else { "Pico-FIDO" };
                                    if s.method == DeviceMethod::Fido {
                                        (
                                            format!("Online - FIDO ({})", fw_label),
                                            rgb(0xf59e0b),
                                            rgb(0xffffff),
                                        )
                                    } else {
                                        (
                                            format!("Online - {}", fw_label),
                                            rgb(0x16a34a),
                                            rgb(0xffffff),
                                        )
                                    }
                                } else if error_owned.is_some() {
                                    ("Error".to_string(), rgb(0xd97706), rgb(0xffffff))
                                } else {
                                    ("Offline".to_string(), rgb(0xef4444), rgb(0xffffff))
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
                        PFIconButton::new(Icon::default().path("icons/refresh-cw.svg"), "Refresh")
                            .on_click(cx.listener(|_, _, _, cx| {
                                cx.emit(SidebarEvent::RefreshDevice);
                            })),
                    )
            });

        // ── Sidebar content column (header + nav + footer) ──────────
        let sidebar_column = v_flex()
            .h_full()
            .bg(sidebar_bg)
            .border_r_1()
            .border_color(border_color)
            .w(sidebar_width)
            .child(header)
            .child(nav_sidebar)
            .child(footer);

        div()
            .id("sidebar-section")
            .h_full()
            .w(sidebar_width)
            .flex_shrink_0()
            .child(sidebar_column)
    }
}
