//! Root application wiring — owns shared state, navigation, and view-model lifecycles.
//!
//! [`ApplicationRoot`] is the top-level GPUI component. It triggers an initial
//! HAL poll via [`DeviceRepo::refresh`], routes between screens based on
//! `active_destination`, and renders the sidebar toggle button as the last child
//! of `main-area` (so it paints on top of the content column). Sidebar collapse/width
//! state and toggle hover state are owned by [`AppSidebar`].

use crate::ui::components::sidebar::{AppSidebar, SidebarEvent};
use crate::ui::models::device::{DeviceEvent, DeviceRepo};
use crate::ui::screens::{
    about::AboutViewModel, config::ConfigViewModel, home::HomeViewModel, passkeys::PasskeysEvent,
    passkeys::PasskeysViewModel, security::SecurityViewModel,
};
use gpui::prelude::*;
use gpui::*;
use gpui_component::Root;
use gpui_component::{
    ActiveTheme, Icon, TitleBar, WindowExt, h_flex, scroll::ScrollableElement, v_flex,
};

gpui::actions!(picoforge, [ToggleSidebar]);

/// Shared reactive models accessible to every screen view-model.
pub struct AppModels {
    pub device: Entity<DeviceRepo>,
}

/// Lazy-initialisation registry for screen view-models. Each field is `None`
/// until its screen is navigated to, then created via `get_or_insert_with`.
pub struct ViewModelStore {
    pub home: Option<Entity<HomeViewModel>>,
    pub about: Option<Entity<AboutViewModel>>,
    pub security: Option<Entity<SecurityViewModel>>,
    pub passkeys: Option<Entity<PasskeysViewModel>>,
    pub config: Option<Entity<ConfigViewModel>>,
}

impl ViewModelStore {
    /// Create an empty view-model store.
    pub fn new() -> Self {
        Self {
            home: None,
            about: None,
            security: None,
            passkeys: None,
            config: None,
        }
    }
}

/// Which screen is currently displayed in the content area.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Destination {
    Home,
    Passkeys,
    Configuration,
    Security,
    About,
}

/// Top-level GPUI component — owns models, navigation, and wires sidebar + content routing.
pub struct ApplicationRoot {
    pub models: AppModels,
    pub active_destination: Destination,
    pub views_store: ViewModelStore,
    pub sidebar: Entity<AppSidebar>,
    pub focus_handle: FocusHandle,
}

impl ApplicationRoot {
    /// Creates the root, initialises `DeviceRepo`, sidebar, and triggers an immediate device poll.
    pub fn new(cx: &mut Context<Self>) -> Self {
        let device = cx.new(|_| DeviceRepo::new());
        let sidebar = cx.new(|_| AppSidebar::new(Destination::Home, device.clone()));

        // Re-subscribe on device changes
        cx.subscribe(
            &device,
            |this: &mut Self,
             _device: Entity<DeviceRepo>,
             _event: &DeviceEvent,
             cx: &mut Context<Self>| {
                if this.models.device.read(cx).device_changed {
                    this.views_store.passkeys = None;
                }
                cx.notify();
            },
        )
        .detach();

        // Subscribe to sidebar navigation events
        cx.subscribe(
            &sidebar,
            |this: &mut Self,
             _sidebar: Entity<AppSidebar>,
             event: &SidebarEvent,
             cx: &mut Context<Self>| {
                match event {
                    SidebarEvent::Navigate(dest) => {
                        this.active_destination = *dest;
                        this.sidebar.update(cx, |s, cx| {
                            s.set_active_destination(*dest);
                            cx.notify();
                        });
                        cx.notify();
                    }
                    SidebarEvent::RefreshDevice => {
                        this.models.device.update(cx, |repo, cx| repo.refresh(cx));
                    }
                }
            },
        )
        .detach();

        let this = Self {
            models: AppModels {
                device: device.clone(),
            },
            active_destination: Destination::Home,
            views_store: ViewModelStore::new(),
            sidebar,
            focus_handle: cx.focus_handle(),
        };

        device.update(cx, |repo, cx| {
            repo.refresh(cx);
            repo.start_hotplug_watch(cx);
        });
        this
    }

    pub fn focus_handle(&self) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ApplicationRoot {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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
                this.sidebar.update(cx, |s, cx| {
                    s.collapsed = !s.collapsed;
                    cx.notify();
                });
            }))
            .min_h(px(0.))
            .min_w(px(0.))
            .overflow_y_scrollbar()
            .flex_grow()
            .bg(cx.theme().background)
            .child(match self.active_destination {
                Destination::Home => {
                    let view = self.views_store.home.get_or_insert_with(|| {
                        cx.new(|cx| HomeViewModel::new(window, cx, &self.models))
                    });
                    view.clone().into_any_element()
                }
                Destination::Passkeys => {
                    let view = self.views_store.passkeys.get_or_insert_with(|| {
                        let view = cx.new(|cx| PasskeysViewModel::new(window, cx, &self.models));
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
                Destination::Configuration => {
                    let view = self.views_store.config.get_or_insert_with(|| {
                        cx.new(|cx| ConfigViewModel::new(window, cx, &self.models))
                    });
                    view.clone().into_any_element()
                }
                Destination::Security => {
                    let view = self.views_store.security.get_or_insert_with(|| {
                        cx.new(|cx| SecurityViewModel::new(window, cx, &self.models))
                    });
                    view.clone().into_any_element()
                }
                Destination::About => {
                    let view = self.views_store.about.get_or_insert_with(|| {
                        cx.new(|cx| AboutViewModel::new(window, cx, &self.models))
                    });
                    view.clone().into_any_element()
                }
            });

        #[cfg(target_os = "macos")]
        let content_column = content_area;
        #[cfg(not(target_os = "macos"))]
        let content_column = v_flex().size_full().child(title_bar).child(content_area);

        let sidebar_state = self.sidebar.read(cx);
        let sidebar_width = sidebar_state.current_width();
        let is_window_wide = window.bounds().size.width > px(800.0);
        let collapsed = sidebar_state.collapsed() || !is_window_wide;
        let toggle_hovered = sidebar_state.toggle_hovered();

        let is_toggle_visible = !collapsed || toggle_hovered;
        let toggle_icon = if collapsed {
            "icons/chevron-right.svg"
        } else {
            "icons/chevron-left.svg"
        };
        let toggle_tooltip = if collapsed { "Expand" } else { "Collapse" };

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
            .on_hover(cx.listener(|this: &mut Self, hovered, _window, cx| {
                this.sidebar.update(cx, |s, cx| {
                    s.toggle_hovered = *hovered;
                    cx.notify();
                });
            }))
            .child(
                div()
                    .id("sidebar-toggle-btn")
                    .w(px(24.))
                    .h(px(24.))
                    .rounded_full()
                    .bg(cx.theme().sidebar)
                    .border_1()
                    .border_color(cx.theme().sidebar_border)
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
                    .on_click(cx.listener(|this: &mut Self, _, _, cx| {
                        this.sidebar.update(cx, |s, cx| {
                            s.collapsed = !s.collapsed;
                            cx.notify();
                        });
                    }))
                    .child(
                        Icon::default()
                            .path(toggle_icon)
                            .text_color(cx.theme().sidebar_foreground),
                    ),
            );

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
            .child(self.sidebar.clone().into_any_element())
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
