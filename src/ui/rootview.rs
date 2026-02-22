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
    ActiveTheme, IconName, TitleBar, WindowExt,
    button::{Button, ButtonVariants},
    h_flex,
    scroll::ScrollableElement,
    v_flex,
};

pub struct ApplicationRoot {
    active_view: ActiveView,
    is_sidebar_collapsed: bool,
    state: GlobalDeviceState,
    device_loading: bool,
    // Umm, why did my past self do this? This does not belong here.
    sidebar_width: Pixels,
    config_view: Option<Entity<ConfigView>>,
    passkeys_view: Option<Entity<PasskeysView>>,
    logs_view: Option<Entity<LogsView>>,
}

impl ApplicationRoot {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let mut this = Self {
            active_view: ActiveView::Home,
            is_sidebar_collapsed: false,
            state: GlobalDeviceState::new(),
            device_loading: false,
            sidebar_width: px(255.),
            config_view: None,
            passkeys_view: None,
            logs_view: None,
        };
        this.refresh_device_status(None, cx);
        this
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

        let dialog_layer = Root::render_dialog_layer(window, cx);
        let sheet_layer = Root::render_sheet_layer(window, cx);

        let title_bar = TitleBar::new().bg(cx.theme().title_bar).child(
            h_flex()
                .w_full()
                .justify_between()
                .bg(cx.theme().title_bar)
                .items_center()
                .cursor(gpui::CursorStyle::OpenHand)
                .child(
                    Button::new("sidebar_toggle")
                        .ghost()
                        .icon(IconName::PanelLeft)
                        .on_click(cx.listener(|this, _, _, _| {
                            this.is_sidebar_collapsed = !this.is_sidebar_collapsed;
                        }))
                        .tooltip("Toggle Sidebar"),
                ),
        );

        let content_area = v_flex()
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

        #[cfg(target_os = "macos")]
        let body = v_flex().size_full().child(title_bar).child(
            h_flex()
                .size_full()
                .child(sidebar.render(cx))
                .child(content_area),
        );

        #[cfg(not(target_os = "macos"))]
        let body = h_flex()
            .size_full()
            .child(sidebar.render(cx))
            .child(v_flex().size_full().child(title_bar).child(content_area));

        div()
            .size_full()
            .overflow_hidden()
            .child(body)
            .children(dialog_layer)
            .children(sheet_layer)
    }
}
