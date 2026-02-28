use crate::device::io;
use crate::ui::components::sidebar::AppSidebar;
use crate::ui::types::{ActiveView, GlobalDeviceState};
use crate::ui::views::{
    about::AboutView, config::ConfigView, home::HomeView, logs::LogsView, passkeys::PasskeysEvent,
    passkeys::PasskeysView, security::SecurityView,
};

use gpui::prelude::*;
use gpui::*;
use gpui_component::Root;
use gpui_component::{
    ActiveTheme, Icon, TitleBar, WindowExt, h_flex, scroll::ScrollableElement, v_flex,
};

gpui::actions!(picoforge, [ToggleSidebar]);

pub struct ApplicationRoot {
    active_view: ActiveView,
    is_sidebar_collapsed: bool,
    sidebar_toggle_hovered: bool,
    state: GlobalDeviceState,
    device_loading: bool,
    // Umm, why did my past self do this? This does not belong here.
    sidebar_width: Pixels,
    config_view: Option<Entity<ConfigView>>,
    passkeys_view: Option<Entity<PasskeysView>>,
    logs_view: Option<Entity<LogsView>>,
    focus_handle: FocusHandle,
}

impl ApplicationRoot {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let mut this = Self {
            active_view: ActiveView::Home,
            is_sidebar_collapsed: false,
            sidebar_toggle_hovered: false,
            state: GlobalDeviceState::new(),
            device_loading: false,
            sidebar_width: px(255.),
            config_view: None,
            passkeys_view: None,
            logs_view: None,
            focus_handle: cx.focus_handle(),
        };
        this.refresh_device_status(None, cx);
        this
    }

    pub fn focus_handle(&self) -> FocusHandle {
        self.focus_handle.clone()
    }

    fn refresh_device_status(&mut self, window: Option<&mut Window>, cx: &mut Context<Self>) {
        if self.device_loading {
            return;
        }

        self.device_loading = true;
        self.state.error = None;
        cx.notify();

        match io::read_device_details() {
            Ok(status) => {
                self.state.device_status = Some(status.clone());
                self.state.error = None;

                match io::get_fido_info() {
                    Ok(fido) => {
                        self.state.fido_info = Some(fido);
                    }
                    Err(e) => {
                        log::error!("FIDO Info fetch failed: {}", e);
                        self.state.fido_info = None;
                    }
                }

                if let Some(config_view) = &self.config_view {
                    if let Some(window) = window {
                        config_view.update(cx, |view, cx| {
                            view.update_device_status(Some(status.clone()), window, cx);
                        });
                    }
                }

                if let Some(passkeys_view) = &self.passkeys_view {
                    let fido = self.state.fido_info.clone();
                    passkeys_view.update(cx, |view, cx| {
                        view.update_device_status(Some(status.clone()), fido, cx);
                    });
                }
            }
            Err(e) => {
                self.state.device_status = None;
                self.state.error = Some(format!("{}", e));
                self.state.fido_info = None;
            }
        }
        self.device_loading = false;
        cx.notify();
    }

    pub fn toggle_sidebar(&mut self, cx: &mut Context<Self>) {
        self.is_sidebar_collapsed = !self.is_sidebar_collapsed;
        cx.notify();
    }
}

impl Render for ApplicationRoot {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let window_width = window.bounds().size.width;
        let is_window_wide = window_width > px(800.0);
        let is_sidebar_collapsed = self.is_sidebar_collapsed || !is_window_wide;

        let target_width = if is_sidebar_collapsed {
            px(48.)
        } else {
            px(255.)
        };

        if (self.sidebar_width - target_width).abs() > px(0.1) {
            self.sidebar_width = self.sidebar_width + (target_width - self.sidebar_width) * 0.2;
            window.request_animation_frame();
        } else {
            self.sidebar_width = target_width;
        }

        let dialog_layer = Root::render_dialog_layer(window, &mut **cx);
        let sheet_layer = Root::render_sheet_layer(window, &mut **cx);

        let title_bar = TitleBar::new().bg(cx.theme().title_bar).child(
            h_flex()
                .w_full()
                .justify_between()
                .bg(cx.theme().title_bar)
                .items_center()
                .cursor(gpui::CursorStyle::OpenHand),
        );

        let content_area = v_flex()
            .track_focus(&self.focus_handle)
            .key_context("ApplicationRoot")
            .on_action(cx.listener(|this, _: &ToggleSidebar, _, cx| {
                this.toggle_sidebar(cx);
            }))
            .min_h(px(0.))
            .min_w(px(0.))
            .overflow_y_scrollbar()
            .flex_grow()
            .bg(cx.theme().background)
            .child(match self.active_view {
                ActiveView::Home => {
                    HomeView::build(&self.state, cx.theme(), window.bounds().size.width)
                        .into_any_element()
                }
                ActiveView::Passkeys => {
                    let view = self.passkeys_view.get_or_insert_with(|| {
                        let view = cx.new(|cx| {
                            PasskeysView::new(
                                window,
                                cx,
                                self.state.device_status.clone(),
                                self.state.fido_info.clone(),
                            )
                        });
                        cx.subscribe_in(
                            &view,
                            window,
                            |_, _, event: &PasskeysEvent, window, cx| match event {
                                PasskeysEvent::Notification(msg) => {
                                    window.push_notification(msg.to_string(), cx);
                                }
                                PasskeysEvent::CloseDialog => {
                                    window.close_dialog(cx);
                                }
                            },
                        )
                        .detach();
                        view
                    });
                    view.clone().into_any_element()
                }
                ActiveView::Configuration => {
                    let view = self.config_view.get_or_insert_with(|| {
                        cx.new(|cx| ConfigView::new(window, cx, self.state.device_status.clone()))
                    });
                    view.clone().into_any_element()
                }
                ActiveView::Security => SecurityView::build(cx).into_any_element(),
                ActiveView::Logs => {
                    let view = self
                        .logs_view
                        .get_or_insert_with(|| cx.new(|cx| LogsView::new(window, cx)));
                    view.clone().into_any_element()
                }
                ActiveView::About => AboutView::build(cx.theme()).into_any_element(),
            });

        let sidebar = AppSidebar::new(
            self.active_view,
            self.sidebar_width,
            is_sidebar_collapsed,
            self.state.clone(),
        )
        .on_select(|this: &mut Self, view, _, _| {
            this.active_view = view;
        })
        .on_refresh(|this, window, cx| {
            this.refresh_device_status(Some(window), cx);
        });

        // Toggle button absolutely positioned at the sidebar's right edge.
        // It fades in on hover when the sidebar is collapsed.
        let sidebar_bg = cx.theme().sidebar;
        let border_color = cx.theme().sidebar_border;
        let sidebar_fg = cx.theme().sidebar_foreground;
        let is_toggle_visible = !is_sidebar_collapsed || self.sidebar_toggle_hovered;
        let sidebar_width = self.sidebar_width;
        let toggle_icon = if is_sidebar_collapsed {
            "icons/chevron-right.svg"
        } else {
            "icons/chevron-left.svg"
        };
        let toggle_tooltip = if is_sidebar_collapsed {
            "Expand"
        } else {
            "Collapse"
        };
        let toggle_btn = div()
            .id("sidebar-toggle-zone")
            .absolute()
            .left(sidebar_width - px(14.))
            .top_0()
            .bottom_0()
            .w(px(28.))
            .flex()
            .items_center()
            .justify_center()
            .on_hover(cx.listener(|this, hovered, _, cx| {
                this.sidebar_toggle_hovered = *hovered;
                cx.notify();
            }))
            .child(
                div()
                    .id("sidebar-toggle-btn")
                    .w(px(24.))
                    .h(px(24.))
                    .rounded_full()
                    .bg(sidebar_bg)
                    .border_1()
                    .border_color(border_color)
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor(gpui::CursorStyle::PointingHand)
                    .opacity(if is_toggle_visible { 1.0 } else { 0.0 })
                    .tooltip(move |window, cx| {
                        gpui_component::tooltip::Tooltip::new(toggle_tooltip)
                            .action(&ToggleSidebar, None)
                            .build(window, cx)
                    })
                    .on_click(cx.listener(|this, _, _, _| {
                        this.is_sidebar_collapsed = !this.is_sidebar_collapsed;
                    }))
                    .child(Icon::default().path(toggle_icon).text_color(sidebar_fg)),
            );

        #[cfg(target_os = "macos")]
        let content_column = content_area;
        #[cfg(not(target_os = "macos"))]
        let content_column = v_flex().size_full().child(title_bar).child(content_area);

        let main_area = h_flex()
            .id("main-area")
            .relative()
            .items_start()
            .map(|this| {
                if cfg!(target_os = "macos") {
                    this.flex_1().min_h(px(0.))
                } else {
                    this.size_full()
                }
            })
            .child(
                div()
                    .h_full()
                    .w(sidebar_width)
                    .flex_shrink_0()
                    .child(sidebar.render(cx)),
            )
            .child(content_column.h_full().flex_1().w_0())
            .child(toggle_btn);

        #[cfg(target_os = "macos")]
        let body = v_flex().size_full().child(title_bar).child(main_area);

        #[cfg(not(target_os = "macos"))]
        let body = main_area;

        div()
            .id("application-root")
            .size_full()
            .overflow_hidden()
            .child(body)
            .children(dialog_layer)
            .children(sheet_layer)
    }
}
