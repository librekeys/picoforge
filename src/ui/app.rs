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
    about::AboutViewModel, accounts::AccountsEvent, accounts::AccountsViewModel,
    attestation::AttestationEvent, attestation::AttestationViewModel, audit::AuditViewModel,
    backup::BackupViewModel, config::ConfigViewModel, home::HomeViewModel, lock::LockViewModel,
    offboard::OffboardEvent, offboard::OffboardViewModel, openpgp::OpenPgpEvent,
    openpgp::OpenPgpViewModel, passkeys::PasskeysEvent, passkeys::PasskeysViewModel, piv::PivEvent,
    piv::PivViewModel, security::SecurityViewModel, slots::SlotsEvent, slots::SlotsViewModel,
};
use gpui_kit::component::{
    ActiveTheme, Icon, TitleBar, WindowExt, h_flex, scroll::ScrollableElement, v_flex,
};
use gpui_kit::prelude::*;
use gpui_kit::*;

gpui_kit::actions!(picoforge, [ToggleSidebar]);

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
    pub accounts: Option<Entity<AccountsViewModel>>,
    pub slots: Option<Entity<SlotsViewModel>>,
    pub piv: Option<Entity<PivViewModel>>,
    pub openpgp: Option<Entity<OpenPgpViewModel>>,
    pub audit: Option<Entity<AuditViewModel>>,
    pub backup: Option<Entity<BackupViewModel>>,
    pub lock: Option<Entity<LockViewModel>>,
    pub attestation: Option<Entity<AttestationViewModel>>,
    pub offboard: Option<Entity<OffboardViewModel>>,
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
            accounts: None,
            slots: None,
            piv: None,
            openpgp: None,
            audit: None,
            backup: None,
            lock: None,
            attestation: None,
            offboard: None,
            config: None,
        }
    }
}

/// Which screen is currently displayed in the content area.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Destination {
    Home,
    Passkeys,
    Accounts,
    Slots,
    Piv,
    OpenPgp,
    Audit,
    Backup,
    Lock,
    Attestation,
    Offboard,
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
    device_picker_open: bool,
    seen_inventory_revision: u64,
}

impl ApplicationRoot {
    /// Creates the root and initializes the shared device and sidebar models.
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let device = cx.new(|_| DeviceRepo::new());
        let sidebar = cx.new(|_| AppSidebar::new(Destination::Home, device.clone()));

        // Keep screens in sync and prompt when the supported-key inventory changes.
        cx.subscribe_in(
            &device,
            window,
            |this: &mut Self,
             _device: &Entity<DeviceRepo>,
             _event: &DeviceEvent,
             window: &mut Window,
             cx: &mut Context<Self>| {
                if this.models.device.read(cx).device_changed {
                    this.views_store.passkeys = None;
                }

                let (revision, key_count) = {
                    let repo = this.models.device.read(cx);
                    (repo.inventory_revision, repo.connected_keys.len())
                };
                if revision != this.seen_inventory_revision {
                    this.seen_inventory_revision = revision;
                    if this.device_picker_open && key_count < 2 {
                        this.device_picker_open = false;
                        window.close_dialog(cx);
                    } else if key_count > 1 {
                        this.open_device_picker(window, cx);
                    }
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

        Self {
            models: AppModels {
                device: device.clone(),
            },
            active_destination: Destination::Home,
            views_store: ViewModelStore::new(),
            sidebar,
            focus_handle: cx.focus_handle(),
            device_picker_open: false,
            seen_inventory_revision: 0,
        }
    }

    fn open_device_picker(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.device_picker_open || window.has_active_dialog(cx) {
            return;
        }

        let (keys, selected_serial) = {
            let repo = self.models.device.read(cx);
            (
                repo.connected_keys.clone(),
                repo.selected_key_serial.clone(),
            )
        };
        if keys.len() < 2 {
            return;
        }

        self.device_picker_open = true;
        let app = cx.entity();
        let can_cancel = selected_serial.is_some();
        window.open_dialog(cx, move |dialog, _window, _cx| {
            let choices = keys.iter().enumerate().map(|(index, key)| {
                let serial = key.serial.clone();
                let firmware_name = match &key.firmware_type {
                    crate::hal::types::FirmwareType::RSKey => "RS-Key",
                    crate::hal::types::FirmwareType::Fapico2 => "fapico2",
                    _ => "PicoFido",
                };
                let current = selected_serial.as_deref() == Some(key.serial.as_str());
                let label = if current {
                    format!("{firmware_name} · {} (current)", key.serial)
                } else {
                    format!("{firmware_name} · {}", key.serial)
                };
                let app = app.clone();
                gpui_kit::component::button::Button::new(format!("choose-key-{index}"))
                    .label(label)
                    .on_click(move |_, window, cx| {
                        app.update(cx, |this, cx| {
                            this.device_picker_open = false;
                            this.models
                                .device
                                .update(cx, |repo, cx| repo.select_key(serial.clone(), cx));
                        });
                        window.close_dialog(cx);
                    })
            });
            let choices = v_flex()
                .gap_2()
                .max_h(px(360.))
                .overflow_y_scrollbar()
                .children(choices);
            let body = v_flex()
                .gap_2()
                .child("PicoForge found more than one supported key. Choose which key to use.")
                .child(choices);

            let dialog = dialog
                .title("Choose a security key")
                .child(body)
                .close_button(false)
                .overlay_closable(false)
                .keyboard(false);
            if can_cancel {
                let app = app.clone();
                dialog.footer(
                    gpui_kit::component::dialog::DialogFooter::new().children(vec![
                        gpui_kit::component::button::Button::new("keep-current-key")
                            .label("Cancel")
                            .on_click(move |_, window, cx| {
                                app.update(cx, |this, _| this.device_picker_open = false);
                                window.close_dialog(cx);
                            }),
                    ]),
                )
            } else {
                dialog
            }
        });
    }

    pub fn focus_handle(&self) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ApplicationRoot {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let connected_key_count = self.models.device.read(cx).connected_keys.len();
        let device_switcher = if connected_key_count > 1 {
            gpui_kit::component::button::Button::new("switch-security-key")
                .label(format!("{connected_key_count} security keys"))
                .on_click(cx.listener(|this, _, window, cx| {
                    this.open_device_picker(window, cx);
                }))
                .into_any_element()
        } else {
            div().into_any_element()
        };
        let title_bar = TitleBar::new().bg(cx.theme().title_bar).child(
            h_flex()
                .w_full()
                .justify_between()
                .bg(cx.theme().title_bar)
                .items_center()
                .cursor(gpui_kit::CursorStyle::OpenHand)
                .child(div().flex_grow(1.0))
                .child(device_switcher),
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
            .flex_grow(1.0)
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
                Destination::Accounts => {
                    let view = self.views_store.accounts.get_or_insert_with(|| {
                        let view = cx.new(|cx| AccountsViewModel::new(window, cx, &self.models));
                        cx.subscribe_in(
                            &view,
                            window,
                            |_, _, event: &AccountsEvent, window, cx| match event {
                                AccountsEvent::Notification(msg) => {
                                    window.push_notification(msg.to_string(), cx);
                                }
                            },
                        )
                        .detach();
                        view
                    });
                    view.clone().into_any_element()
                }
                Destination::Slots => {
                    let view = self.views_store.slots.get_or_insert_with(|| {
                        let view = cx.new(|cx| SlotsViewModel::new(window, cx, &self.models));
                        cx.subscribe_in(&view, window, |_, _, event: &SlotsEvent, window, cx| {
                            match event {
                                SlotsEvent::Notification(msg) => {
                                    window.push_notification(msg.to_string(), cx);
                                }
                            }
                        })
                        .detach();
                        view
                    });
                    view.clone().into_any_element()
                }
                Destination::Piv => {
                    let view = self.views_store.piv.get_or_insert_with(|| {
                        let view = cx.new(|cx| PivViewModel::new(window, cx, &self.models));
                        cx.subscribe_in(&view, window, |_, _, event: &PivEvent, window, cx| {
                            match event {
                                PivEvent::Notification(msg) => {
                                    window.push_notification(msg.to_string(), cx);
                                }
                            }
                        })
                        .detach();
                        view
                    });
                    view.clone().into_any_element()
                }
                Destination::OpenPgp => {
                    let view = self.views_store.openpgp.get_or_insert_with(|| {
                        let view = cx.new(|cx| OpenPgpViewModel::new(window, cx, &self.models));
                        cx.subscribe_in(&view, window, |_, _, event: &OpenPgpEvent, window, cx| {
                            match event {
                                OpenPgpEvent::Notification(msg) => {
                                    window.push_notification(msg.to_string(), cx);
                                }
                            }
                        })
                        .detach();
                        view
                    });
                    view.clone().into_any_element()
                }
                Destination::Audit => {
                    let view = self.views_store.audit.get_or_insert_with(|| {
                        cx.new(|cx| AuditViewModel::new(window, cx, &self.models))
                    });
                    view.clone().into_any_element()
                }
                Destination::Backup => {
                    let view = self.views_store.backup.get_or_insert_with(|| {
                        cx.new(|cx| BackupViewModel::new(window, cx, &self.models))
                    });
                    view.clone().into_any_element()
                }
                Destination::Lock => {
                    let view = self.views_store.lock.get_or_insert_with(|| {
                        cx.new(|cx| LockViewModel::new(window, cx, &self.models))
                    });
                    view.clone().into_any_element()
                }
                Destination::Attestation => {
                    let view = self.views_store.attestation.get_or_insert_with(|| {
                        let view = cx.new(|cx| AttestationViewModel::new(window, cx, &self.models));
                        cx.subscribe_in(
                            &view,
                            window,
                            |_, _, event: &AttestationEvent, window, cx| match event {
                                AttestationEvent::Notification(msg) => {
                                    window.push_notification(msg.to_string(), cx);
                                }
                            },
                        )
                        .detach();
                        view
                    });
                    view.clone().into_any_element()
                }
                Destination::Offboard => {
                    let view = self.views_store.offboard.get_or_insert_with(|| {
                        let view = cx.new(|cx| OffboardViewModel::new(window, cx, &self.models));
                        cx.subscribe_in(
                            &view,
                            window,
                            |_, _, event: &OffboardEvent, window, cx| match event {
                                OffboardEvent::Notification(msg) => {
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
                    .cursor(gpui_kit::CursorStyle::PointingHand)
                    .opacity(if is_toggle_visible { 1.0 } else { 0.0 })
                    .tooltip(move |window, cx| {
                        gpui_kit::component::tooltip::Tooltip::new(toggle_tooltip)
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
    }
}
