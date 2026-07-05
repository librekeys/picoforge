use crate::hal::io;
use crate::hal::types::{DeviceMethod, FirmwareType};
use crate::ui::models::device::DeviceRepo;
use crate::ui::screens::{
    about::AboutViewModel, config::ConfigView, home::HomeViewModel, passkeys::PasskeysView,
    security::SecurityViewModel,
};
use gpui::*;

gpui::actions!(picoforge, [ToggleSidebar]);

pub struct AppModels {
    pub device: Entity<DeviceRepo>,
}

pub struct ViewModelStore {
    pub home: Option<Entity<HomeViewModel>>,
    pub about: Option<Entity<AboutViewModel>>,
    pub security: Option<Entity<SecurityViewModel>>,
    pub passkeys: Option<Entity<PasskeysView>>,
    pub config: Option<Entity<ConfigView>>,
}

impl ViewModelStore {
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

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ActiveView {
    Home,
    Passkeys,
    Configuration,
    Security,
    About,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LayoutState {
    pub active_view: ActiveView,
    pub is_sidebar_collapsed: bool,
    pub sidebar_toggle_hovered: bool,
    pub sidebar_width: Pixels,
}

impl LayoutState {
    pub fn new() -> Self {
        Self {
            active_view: ActiveView::Home,
            is_sidebar_collapsed: false,
            sidebar_toggle_hovered: false,
            sidebar_width: px(255.),
        }
    }
}

pub struct ApplicationRoot {
    pub models: AppModels,
    pub view_state: LayoutState,
    pub views_store: ViewModelStore,
    pub focus_handle: FocusHandle,
}

impl ApplicationRoot {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let device = cx.new(|_| DeviceRepo::new());

        let mut this = Self {
            models: AppModels { device },
            view_state: LayoutState::new(),
            views_store: ViewModelStore::new(),
            focus_handle: cx.focus_handle(),
        };

        this.refresh_device_status(None, cx);
        this
    }

    pub fn focus_handle(&self) -> FocusHandle {
        self.focus_handle.clone()
    }

    pub(crate) fn refresh_device_status(
        &mut self,
        window: Option<&mut Window>,
        cx: &mut Context<Self>,
    ) {
        if self.models.device.read(cx).is_loading() {
            return;
        }

        self.models.device.update(cx, |repo, _| repo.begin_load());
        cx.notify();

        match io::read_device_details() {
            Ok(status) => {
                let device_changed = self
                    .models
                    .device
                    .update(cx, |repo, _| repo.set_status(status.clone()));

                let firmware_type = status.firmware_type;
                let method = status.method;

                if device_changed {
                    self.views_store.passkeys = None;
                } else if let Some(passkeys_view) = &self.views_store.passkeys {
                    passkeys_view.update(cx, |view, cx| {
                        view.refresh_if_unlocked(cx);
                    });
                }

                match io::get_fido_info() {
                    Ok(fido) => {
                        self.models
                            .device
                            .update(cx, |repo, _| repo.set_fido_info(Some(fido)));
                    }
                    Err(e) => {
                        log::error!("FIDO Info fetch failed: {}", e);
                        self.models
                            .device
                            .update(cx, |repo, _| repo.set_fido_info(None));
                    }
                }

                if firmware_type == FirmwareType::RSKey && method == DeviceMethod::Rescue {
                    let led = io::read_led_config().ok();
                    let mgmt = io::read_management_config().ok();
                    self.models
                        .device
                        .update(cx, |repo, _| repo.set_auxiliary_data(led, mgmt));
                } else {
                    self.models
                        .device
                        .update(cx, |repo, _| repo.clear_auxiliary_data());
                }

                if let Some(config_view) = &self.views_store.config
                    && let Some(window) = window
                {
                    config_view.update(cx, |view, cx| {
                        view.sync_from_device(window, cx);
                    });
                }
            }
            Err(e) => {
                self.models
                    .device
                    .update(cx, |repo, _| repo.set_error(format!("{}", e)));
                self.views_store.passkeys = None;
            }
        }

        self.models.device.update(cx, |repo, _| repo.end_load());
        cx.notify();
    }

    pub fn toggle_sidebar(&mut self, cx: &mut Context<Self>) {
        self.view_state.is_sidebar_collapsed = !self.view_state.is_sidebar_collapsed;
        cx.notify();
    }
}
