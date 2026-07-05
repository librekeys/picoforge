use crate::ui::app::{ActiveView, ApplicationRoot, ToggleSidebar};
use crate::ui::components::sidebar::AppSidebar;
use crate::ui::screens::{
    about::AboutViewModel, config::ConfigView, home::HomeViewModel, passkeys::PasskeysEvent,
    passkeys::PasskeysView, security::SecurityViewModel,
};
use gpui::prelude::*;
use gpui::*;
use gpui_component::Root;
use gpui_component::{
    ActiveTheme, Icon, TitleBar, WindowExt, h_flex, scroll::ScrollableElement, v_flex,
};

impl Render for ApplicationRoot {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let window_width = window.bounds().size.width;
        let is_window_wide = window_width > px(800.0);
        let is_sidebar_collapsed = self.view_state.is_sidebar_collapsed || !is_window_wide;

        let target_width = if is_sidebar_collapsed {
            px(48.)
        } else {
            px(255.)
        };

        if (self.view_state.sidebar_width - target_width).abs() > px(0.1) {
            self.view_state.sidebar_width = self.view_state.sidebar_width
                + (target_width - self.view_state.sidebar_width) * 0.2;
            window.request_animation_frame();
        } else {
            self.view_state.sidebar_width = target_width;
        }

        let dialog_layer = Root::render_dialog_layer(window, cx);
        let sheet_layer = Root::render_sheet_layer(window, cx);

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
            .child(match self.view_state.active_view {
                ActiveView::Home => {
                    let view = self.views_store.home.get_or_insert_with(|| {
                        cx.new(|cx| HomeViewModel::new(window, cx, &self.models))
                    });
                    view.clone().into_any_element()
                }
                ActiveView::Passkeys => {
                    let view = self.views_store.passkeys.get_or_insert_with(|| {
                        let view = cx.new(|cx| PasskeysView::new(window, cx, &self.models));
                        cx.subscribe_in(
                            &view,
                            window,
                            |_, _, event: &PasskeysEvent, window, cx| match event {
                                PasskeysEvent::Notification(msg) => {
                                    window.push_notification(msg.to_string(), cx);
                                }
                            },
                        )
                        .detach();
                        view
                    });
                    view.clone().into_any_element()
                }
                ActiveView::Configuration => {
                    let view = self.views_store.config.get_or_insert_with(|| {
                        cx.new(|cx| ConfigView::new(window, cx, &self.models))
                    });
                    view.clone().into_any_element()
                }
                ActiveView::Security => {
                    let view = self.views_store.security.get_or_insert_with(|| {
                        cx.new(|cx| SecurityViewModel::new(window, cx, &self.models))
                    });
                    view.clone().into_any_element()
                }
                ActiveView::About => {
                    let view = self.views_store.about.get_or_insert_with(|| {
                        cx.new(|cx| AboutViewModel::new(window, cx, &self.models))
                    });
                    view.clone().into_any_element()
                }
            });

        let sidebar = AppSidebar::new(
            self.view_state.active_view,
            self.view_state.sidebar_width,
            is_sidebar_collapsed,
            &self.models,
        )
        .on_select(|this: &mut Self, view, _, _| {
            this.view_state.active_view = view;
        })
        .on_refresh(|this, window, cx| {
            this.refresh_device_status(Some(window), cx);
        });

        let sidebar_bg = cx.theme().sidebar;
        let border_color = cx.theme().sidebar_border;
        let sidebar_fg = cx.theme().sidebar_foreground;
        let is_toggle_visible = !is_sidebar_collapsed || self.view_state.sidebar_toggle_hovered;
        let sidebar_width = self.view_state.sidebar_width;
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
                this.view_state.sidebar_toggle_hovered = *hovered;
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
                        this.view_state.is_sidebar_collapsed =
                            !this.view_state.is_sidebar_collapsed;
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
