use crate::device::types::FullDeviceStatus;
use crate::ui::components::sidebar::{ActiveView, AppSidebar};
use crate::ui::{
    colors,
    views::{
        about::AboutView, config::ConfigView, home::HomeView, logs::LogsView,
        passkeys::PasskeysView, security::SecurityView,
    },
};
use gpui::*;
use gpui_component::{
    ActiveTheme, IconName, TitleBar,
    button::{Button, ButtonVariants},
    h_flex,
    scroll::ScrollableElement,
    v_flex,
};

pub struct ApplicationRoot {
    active_view: ActiveView,
    collapsed: bool,
    device_status: Option<FullDeviceStatus>,
    device_loading: bool,
    device_error: Option<String>,
    sidebar_width: Pixels,
}

impl ApplicationRoot {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let mut this = Self {
            active_view: ActiveView::Home,
            collapsed: false,
            device_status: None,
            device_loading: false,
            device_error: None,
            sidebar_width: px(255.),
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
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let target_width = if self.collapsed { px(48.) } else { px(255.) };
        if (self.sidebar_width - target_width).abs() > px(0.1) {
            self.sidebar_width = self.sidebar_width + (target_width - self.sidebar_width) * 0.2;
            window.request_animation_frame();
        } else {
            self.sidebar_width = target_width;
        }

        div().size_full().overflow_hidden().child(
            h_flex()
                .size_full()
                .child(
                    AppSidebar::new(
                        self.active_view,
                        self.sidebar_width,
                        self.collapsed,
                        self.device_status.clone(),
                        self.device_error.clone(),
                    )
                    .on_select(|this: &mut Self, view, _, _| {
                        this.active_view = view;
                    })
                    .on_refresh(|this: &mut Self, _, cx| {
                        this.refresh_device_status(cx);
                    })
                    .render(cx),
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
                                    ActiveView::Home => {
                                        HomeView::build(cx.theme()).into_any_element()
                                    }
                                    ActiveView::Passkeys => {
                                        PasskeysView::build().into_any_element()
                                    }
                                    ActiveView::Configuration => {
                                        ConfigView::build().into_any_element()
                                    }
                                    ActiveView::Security => {
                                        SecurityView::build().into_any_element()
                                    }
                                    ActiveView::Logs => LogsView::build().into_any_element(),
                                    ActiveView::About => {
                                        AboutView::build(cx.theme()).into_any_element()
                                    }
                                }),
                        ),
                ),
        )
    }
}
