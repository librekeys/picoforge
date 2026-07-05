use crate::hal::io;
use crate::hal::types::StoredCredential;
use crate::ui::app::AppModels;
use crate::ui::components::dialog;
use crate::ui::components::dialog::{
    ChangePinContent, ConfirmContent, PinPromptContent, SetPinContent, StatusContent,
};
use crate::ui::models::device::{DeviceEvent, DeviceRepo};
use gpui::*;
use gpui_component::button::ButtonVariants;
use gpui_component::{ActiveTheme, StyledExt, WindowExt};

pub struct PasskeysView {
    pub(super) device: Entity<DeviceRepo>,
    pub(super) credentials: Vec<StoredCredential>,
    pub(super) unlocked: bool,
    cached_pin: Option<String>,
    pub(super) loading: bool,
    pub(super) csr_loading: bool,
    pub(super) csr_pem: Option<String>,
    pub(super) show_csr: bool,
    pub(super) _task: Option<Task<()>>,
}

pub enum PasskeysEvent {
    Notification(String),
}

impl EventEmitter<PasskeysEvent> for PasskeysView {}

impl PasskeysView {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>, models: &AppModels) -> Self {
        let device = models.device.clone();
        cx.subscribe(&device, |_, _, _: &DeviceEvent, cx| cx.notify())
            .detach();
        Self {
            device,
            credentials: Vec::new(),
            unlocked: false,
            cached_pin: None,
            loading: false,
            csr_loading: false,
            csr_pem: None,
            show_csr: false,
            _task: None,
        }
    }

    pub(super) fn unlock_storage(
        &mut self,
        pin: String,
        dialog_handle: WeakEntity<PinPromptContent>,
        cx: &mut Context<Self>,
    ) {
        if self.loading {
            return;
        }
        self.loading = true;
        cx.notify();

        log::info!("Unlocking FIDO storage...");
        let entity = cx.entity().downgrade();

        self._task = Some(cx.spawn(async move |_, cx| {
            let pin_for_bg = pin.clone();
            let result = cx
                .background_executor()
                .spawn(async move { io::get_credentials(pin_for_bg) })
                .await;

            let _ = entity.update(cx, |this, cx| {
                this.loading = false;
                match result {
                    Ok(creds) => {
                        log::info!("Storage unlocked. {} credentials found.", creds.len());
                        this.unlocked = true;
                        this.cached_pin = Some(pin);
                        this.credentials = creds;
                        let _ = dialog_handle.update(cx, |d, cx| {
                            d.set_success("Storage unlocked successfully.".to_string(), cx);
                        });
                    }
                    Err(e) => {
                        log::error!("Failed to unlock storage: {}", e);
                        let _ = dialog_handle.update(cx, |d, cx| {
                            d.set_error(format!("Failed to unlock: {}", e), cx);
                        });
                    }
                }
                cx.notify();
            });
        }));
    }

    pub(super) fn lock_storage(&mut self, cx: &mut Context<Self>) {
        self.unlocked = false;
        self.cached_pin = None;
        self.credentials.clear();
        cx.notify();
    }

    pub(super) fn execute_delete(
        &mut self,
        credential_id: String,
        pin: String,
        dialog_handle: WeakEntity<ConfirmContent>,
        cx: &mut Context<Self>,
    ) {
        if self.loading {
            return;
        }
        self.loading = true;
        cx.notify();

        log::info!("Deleting credential...");
        let entity = cx.entity().downgrade();

        self._task = Some(cx.spawn(async move |_, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { io::delete_credential(pin, credential_id) })
                .await;

            let _ = entity.update(cx, |this, cx| match result {
                Ok(_) => {
                    log::info!("Credential deleted successfully.");
                    let _ = dialog_handle.update(cx, |d, cx| {
                        d.set_success("Credential deleted successfully.".to_string(), cx);
                    });
                    this.sync_fido_state(None, cx);
                }
                Err(e) => {
                    log::error!("Error deleting credential: {}", e);
                    this.loading = false;
                    let _ = dialog_handle.update(cx, |d, cx| {
                        d.set_error(format!("Error deleting: {}", e), cx);
                    });
                    cx.notify();
                }
            });
        }));
    }

    fn refresh_credentials(&mut self, pin: String, cx: &mut Context<Self>) {
        let entity = cx.entity().downgrade();
        self._task = Some(cx.spawn(async move |_, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { io::get_credentials(pin) })
                .await;

            let _ = entity.update(cx, |this, cx| {
                this.loading = false;
                if let Ok(creds) = result {
                    this.credentials = creds;
                }
                cx.notify();
            });
        }));
    }

    pub fn refresh_if_unlocked(&mut self, cx: &mut Context<Self>) {
        if !self.unlocked || self.loading {
            return;
        }
        let Some(pin) = self.cached_pin.clone() else {
            return;
        };
        self.loading = true;
        cx.notify();
        self.refresh_credentials(pin, cx);
    }

    fn sync_fido_state(&mut self, new_pin: Option<String>, cx: &mut Context<Self>) {
        if let Ok(info) = io::get_fido_info() {
            self.device.update(cx, |repo, _| {
                repo.fido_info = Some(info);
            });
        }

        if let Some(pin) = new_pin {
            self.cached_pin = Some(pin);
        }

        if self.unlocked
            && let Some(pin) = self.cached_pin.clone()
        {
            self.refresh_credentials(pin, cx);
            return;
        }
        self.loading = false;
        cx.notify();
    }

    pub(super) fn open_unlock_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let view_handle = cx.entity().downgrade();

        dialog::open_pin_prompt(
            "Unlock Storage",
            "Enter your device PIN to view saved passkeys",
            None,
            "Unlock",
            window,
            cx,
            move |pin, dialog_handle, cx| {
                let _ = view_handle.update(cx, |this, cx| {
                    this.unlock_storage(pin, dialog_handle, cx);
                });
            },
        );
    }

    fn open_delete_dialog(
        &mut self,
        cred: &StoredCredential,
        pin: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let cred_id = cred.credential_id.clone();
        let pin_str = pin.clone();
        let name = cred.rp_id.clone();
        let view_handle = cx.entity().downgrade();

        dialog::open_confirm(
            "Delete Passkey",
            format!("Are you sure you want to delete the passkey for {}?", name),
            "Delete",
            gpui_component::button::ButtonVariant::Danger,
            window,
            cx,
            move |dialog_handle, _, cx| {
                let _ = view_handle.update(cx, |this, cx| {
                    this.execute_delete(cred_id.clone(), pin_str.clone(), dialog_handle, cx);
                });
            },
        );
    }

    pub(super) fn open_change_pin_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let view_handle = cx.entity().downgrade();

        dialog::open_change_pin(window, cx, move |current, new, dialog_handle, cx| {
            let _ = view_handle.update(cx, |this, cx| {
                this.change_pin(current, new, dialog_handle, cx);
            });
        });
    }

    pub(super) fn open_setup_pin_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let view_handle = cx.entity().downgrade();

        dialog::open_setup_pin(window, cx, move |new_pin, dialog_handle, cx| {
            let _ = view_handle.update(cx, |this, cx| {
                this.setup_pin(new_pin, dialog_handle, cx);
            });
        });
    }

    fn setup_pin(
        &mut self,
        new: String,
        dialog_handle: WeakEntity<SetPinContent>,
        cx: &mut Context<Self>,
    ) {
        if self.loading {
            return;
        }
        self.loading = true;
        cx.notify();

        log::info!("Setting up FIDO PIN...");
        let entity = cx.entity().downgrade();

        self._task = Some(cx.spawn(async move |_, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { io::change_fido_pin(None, new) })
                .await;

            let _ = entity.update(cx, |this, cx| match result {
                Ok(msg) => {
                    log::info!("PIN configured: {}", msg);
                    let _ = dialog_handle.update(cx, |d, cx| {
                        d.set_success("PIN configured successfully.".to_string(), cx);
                    });
                    this.sync_fido_state(None, cx);
                }
                Err(e) => {
                    log::error!("PIN setup failed: {}", e);
                    this.loading = false;
                    let _ = dialog_handle.update(cx, |d, cx| {
                        d.set_error(format!("Error: {}", e), cx);
                    });
                    cx.notify();
                }
            });
        }));
    }

    pub(super) fn open_min_pin_length_dialog(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let current_min = self
            .device
            .read(cx)
            .fido_info
            .as_ref()
            .map(|f| f.min_pin_length)
            .unwrap_or(4);

        let slider = cx.new(|_| {
            gpui_component::slider::SliderState::new()
                .min(4.0)
                .max(63.0)
                .step(1.0)
                .default_value(current_min as f32)
        });

        let current_pin = cx.new(|cx| {
            gpui_component::input::InputState::new(window, cx)
                .placeholder("Enter current PIN")
                .masked(true)
        });
        let new_pin = cx.new(|cx| {
            gpui_component::input::InputState::new(window, cx)
                .placeholder("Enter new PIN")
                .masked(true)
        });
        let confirm_pin = cx.new(|cx| {
            gpui_component::input::InputState::new(window, cx)
                .placeholder("Confirm new PIN")
                .masked(true)
        });

        let label_view = cx.new(|_cx| SliderLabel {
            slider: slider.clone(),
        });

        let view_handle = cx.entity().downgrade();

        let submit = {
            let current_pin2 = current_pin.clone();
            let new_pin2 = new_pin.clone();
            let confirm_pin2 = confirm_pin.clone();
            let slider2 = slider.clone();
            let view2 = view_handle.clone();
            std::rc::Rc::new(move |window: &mut Window, cx: &mut App| {
                let current_val = current_pin2.read(cx).text().to_string();
                let new_val = new_pin2.read(cx).text().to_string();
                let confirm_val = confirm_pin2.read(cx).text().to_string();
                let min_len = slider2.read(cx).value().start() as u8;

                if current_val.is_empty() {
                    return;
                }

                if !new_val.is_empty() {
                    if new_val != confirm_val {
                        let _ = view2.update(cx, |_, cx| {
                            cx.emit(PasskeysEvent::Notification("PINs do not match".to_string()));
                        });
                        return;
                    }
                    if new_val.len() < min_len as usize {
                        let _ = view2.update(cx, |_, cx| {
                            cx.emit(PasskeysEvent::Notification(format!(
                                "PIN must be at least {} characters",
                                min_len
                            )));
                        });
                        return;
                    }
                }
                window.close_dialog(cx);
                let status_handle =
                    dialog::open_status_dialog("Update Minimum PIN Length", window, cx);
                let _ = view2.update(cx, |this, cx| {
                    this.update_min_length(current_val, min_len, new_val, status_handle, cx);
                });
            })
        };

        window.open_dialog(cx, move |dialog, window, _| {
            let current = current_pin.clone();
            let new = new_pin.clone();
            let confirm = confirm_pin.clone();
            let slider_handle = slider.clone();
            let submit_for_ok = submit.clone();
            let submit_for_btn = submit.clone();
            let _ = window;

            dialog
                .title("Update Minimum PIN Length")
                .child(
                    "Set the minimum allowed PIN length (4-63 characters) and enter a new PIN that meets this requirement.",
                )
                .child(
                    gpui_component::v_flex()
                        .gap_4()
                        .pb_4()
                        .child(
                             gpui_component::v_flex()
                                .gap_2()
                                .child(label_view.clone())
                                .child(gpui_component::slider::Slider::new(&slider_handle))
                        )
                        .child("Current PIN")
                        .child(gpui_component::input::Input::new(&current))
                        .child(
                             gpui_component::v_flex()
                                .gap_2()
                                .child(format!("New PIN (min {} chars)", current_min))
                                .child(gpui_component::input::Input::new(&new))
                        )
                        .child("Confirm New PIN")
                        .child(gpui_component::input::Input::new(&confirm)),
                )
                .on_ok(move |_, window, cx| {
                    submit_for_ok(window, cx);
                    false
                })
                .footer(move |_, _window, _cx, _| {
                    let s = submit_for_btn.clone();
                    vec![
                        gpui_component::button::Button::new("cancel")
                            .label("Cancel")
                            .on_click(|_, window, cx| window.close_dialog(cx)),
                        gpui_component::button::Button::new("update")
                            .primary()
                            .label("Update")
                            .on_click(move |_, window, cx| {
                                s(window, cx);
                            }),
                    ]
                })
        });
    }

    fn change_pin(
        &mut self,
        current: String,
        new: String,
        dialog_handle: WeakEntity<ChangePinContent>,
        cx: &mut Context<Self>,
    ) {
        if self.loading {
            return;
        }
        self.loading = true;
        cx.notify();

        log::info!("Changing FIDO PIN...");
        let entity = cx.entity().downgrade();
        let new_for_sync = new.clone();

        self._task = Some(cx.spawn(async move |_, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { io::change_fido_pin(Some(current), new) })
                .await;

            let _ = entity.update(cx, |this, cx| match result {
                Ok(msg) => {
                    log::info!("PIN changed: {}", msg);
                    let _ = dialog_handle.update(cx, |d, cx| {
                        d.set_success("PIN changed successfully.".to_string(), cx);
                    });
                    this.sync_fido_state(Some(new_for_sync), cx);
                }
                Err(e) => {
                    log::error!("PIN change failed: {}", e);
                    this.loading = false;
                    let _ = dialog_handle.update(cx, |d, cx| {
                        d.set_error(format!("Error: {}", e), cx);
                    });
                    cx.notify();
                }
            });
        }));
    }

    fn update_min_length(
        &mut self,
        current: String,
        min_len: u8,
        new_pin: String,
        status_handle: WeakEntity<StatusContent>,
        cx: &mut Context<Self>,
    ) {
        if self.loading {
            return;
        }
        self.loading = true;
        cx.notify();
        log::info!("Updating minimum PIN length to {}...", min_len);
        let entity = cx.entity().downgrade();

        self._task = Some(cx.spawn(async move |_, cx| {
            let current_for_bg = current.clone();
            let res_len = cx
                .background_executor()
                .spawn(async move { io::set_min_pin_length(current_for_bg, min_len) })
                .await;

            if let Err(e) = res_len {
                log::error!("Failed to set minimum PIN length: {}", e);
                let _ = entity.update(cx, |this, cx| {
                    this.loading = false;
                    let _ = status_handle.update(cx, |s, cx| {
                        s.set_error(format!("Failed to set length: {}", e), cx);
                    });
                    cx.notify();
                });
                return;
            }

            if !new_pin.is_empty() {
                let new_pin_for_sync = new_pin.clone();
                let res_pin = cx
                    .background_executor()
                    .spawn(async move { io::change_fido_pin(Some(current), new_pin) })
                    .await;
                let _ = entity.update(cx, |this, cx| match res_pin {
                    Ok(_) => {
                        log::info!("Minimum length and PIN updated successfully.");
                        let _ = status_handle.update(cx, |s, cx| {
                            s.set_success("Minimum length and PIN updated.".to_string(), cx);
                        });
                        this.sync_fido_state(Some(new_pin_for_sync), cx);
                    }
                    Err(e) => {
                        log::error!("Length set, but PIN change failed: {}", e);
                        this.loading = false;
                        let _ = status_handle.update(cx, |s, cx| {
                            s.set_error(format!("Length set, but PIN change failed: {}", e), cx);
                        });
                        cx.notify();
                    }
                });
            } else {
                let _ = entity.update(cx, |this, cx| {
                    log::info!("Minimum PIN length updated to {}.", min_len);
                    let _ = status_handle.update(cx, |s, cx| {
                        s.set_success(format!("Minimum length updated to {}.", min_len), cx);
                    });
                    this.sync_fido_state(None, cx);
                });
            }
        }));
    }

    pub(super) fn request_csr(
        &mut self,
        status_handle: WeakEntity<StatusContent>,
        cx: &mut Context<Self>,
    ) {
        if self.loading {
            return;
        }
        self.loading = true;
        self.csr_loading = true;
        cx.notify();

        log::info!("Request Attestation CSR...");
        let entity = cx.entity().downgrade();

        self._task = Some(cx.spawn(async move |_, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { io::get_enterprise_attestation_csr() })
                .await;

            let _ = entity.update(cx, |this, cx| {
                this.loading = false;
                this.csr_loading = false;
                match result {
                    Ok(pem) => {
                        log::info!("CSR retrieved successfully ({} bytes).", pem.len());
                        this.csr_pem = Some(pem);
                        let _ = status_handle.update(cx, |s, cx| {
                            s.set_success(
                                "CSR retrieved from device. Click \"View CSR\" to inspect or save it.".to_string(),
                                cx,
                            );
                        });
                    }
                    Err(e) => {
                        log::error!("Failed to retrieve CSR: {}", e);
                        let _ = status_handle.update(cx, |s, cx| {
                            s.set_error(format!("Failed to retrieve CSR: {}", e), cx);
                        });
                    }
                }
                cx.notify();
            });
        }));
    }

    fn execute_upload_cert(
        &mut self,
        pin: String,
        cert_path: String,
        dialog_handle: WeakEntity<PinPromptContent>,
        cx: &mut Context<Self>,
    ) {
        if self.loading {
            return;
        }
        self.loading = true;
        cx.notify();

        log::info!(
            "Uploading enterprise attestation certificate from: {}",
            cert_path
        );
        let entity = cx.entity().downgrade();

        self._task = Some(cx.spawn(async move |_, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { io::upload_enterprise_attestation_cert(pin, cert_path) })
                .await;

            let _ = entity.update(cx, |this, cx| {
                this.loading = false;
                match result {
                    Ok(msg) => {
                        log::info!("{}", msg);
                        let _ = dialog_handle.update(cx, |d, cx| {
                            d.set_success(msg, cx);
                        });
                    }
                    Err(e) => {
                        log::error!("Certificate upload failed: {}", e);
                        let _ = dialog_handle.update(cx, |d, cx| {
                            d.set_error(format!("Upload failed: {}", e), cx);
                        });
                    }
                }
                cx.notify();
            });
        }));
    }

    pub(super) fn open_enable_ea_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let view_handle = cx.entity().downgrade();

        dialog::open_pin_prompt(
            "Enable Enterprise Attestation",
            "Enter your device PIN to enable enterprise attestation",
            Some("This operation is irreversible"),
            "Enable",
            window,
            cx,
            move |pin, dialog_handle, cx| {
                let _ = view_handle.update(cx, |this, cx| {
                    this.enable_ea(pin, dialog_handle, cx);
                });
            },
        );
    }

    fn enable_ea(
        &mut self,
        pin: String,
        dialog_handle: WeakEntity<PinPromptContent>,
        cx: &mut Context<Self>,
    ) {
        if self.loading {
            return;
        }
        self.loading = true;
        cx.notify();

        log::info!("Enabling enterprise attestation...");
        let entity = cx.entity().downgrade();

        self._task = Some(cx.spawn(async move |_, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { io::enable_enterprise_attestation(pin) })
                .await;

            let _ = entity.update(cx, |this, cx| match result {
                Ok(msg) => {
                    log::info!("{}", msg);
                    let _ = dialog_handle.update(cx, |d, cx| {
                        d.set_success(msg, cx);
                    });
                    this.sync_fido_state(None, cx);
                }
                Err(e) => {
                    log::error!("Failed to enable EA: {}", e);
                    this.loading = false;
                    let _ = dialog_handle.update(cx, |d, cx| {
                        d.set_error(format!("Error: {}", e), cx);
                    });
                    cx.notify();
                }
            });
        }));
    }

    pub(super) fn open_upload_cert_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let window_handle = window.window_handle();
        let entity = cx.entity().downgrade();

        let receiver = cx.prompt_for_paths(gpui::PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Select Certificate File (PEM or DER)".into()),
        });

        self._task = Some(cx.spawn(async move |_, cx| {
            let Ok(Ok(Some(paths))) = receiver.await else {
                return;
            };
            let Some(first) = paths.into_iter().next() else {
                return;
            };
            let cert_path = first.to_string_lossy().to_string();

            let _ = cx.update_window(window_handle, |_, window, cx| {
                dialog::open_pin_prompt(
                    "Upload Certificate",
                    "Enter your device PIN to upload the certificate to the device",
                    None,
                    "Upload",
                    window,
                    cx,
                    move |pin, dialog_handle, cx| {
                        let _ = entity.update(cx, |this, cx| {
                            this.execute_upload_cert(pin, cert_path.clone(), dialog_handle, cx);
                        });
                    },
                );
            });
        }));
    }

    pub(super) fn open_reset_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let view_handle = cx.entity().downgrade();

        dialog::open_confirm(
            "Factory Reset Device",
            "Are you sure you want to completely erase your device? This will permanently delete ALL passkeys, credentials, and your PIN. This action cannot be undone.".to_string(),
            "Reset Device",
            gpui_component::button::ButtonVariant::Danger,
            window,
            cx,
            move |_dialog_handle, window, cx| {
                window.close_dialog(cx);
                let _ = view_handle.update(cx, |this, cx| {
                    this.execute_reset(window, cx);
                });
            },
        );
    }

    fn execute_reset(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.loading {
            return;
        }
        self.loading = true;

        let status_handle = dialog::open_status_dialog("Resetting Device...", window, cx);
        let entity = cx.entity().downgrade();

        let _ = status_handle.update(cx, |d, cx| {
            d.set_loading(
                "Unplug your security key, then plug it back in within 10 seconds.",
                cx,
            );
        });

        self._task = Some(cx.spawn(async move |_, cx| {
            let reconnected = cx
                .background_executor()
                .spawn(async move {
                    let start = std::time::Instant::now();
                    while start.elapsed().as_secs() < 15 {
                        std::thread::sleep(std::time::Duration::from_millis(200));
                        if crate::hal::fido::hid::HidTransport::open().is_err() {
                            break;
                        }
                    }

                    while start.elapsed().as_secs() < 15 {
                        std::thread::sleep(std::time::Duration::from_millis(500));
                        if crate::hal::fido::hid::HidTransport::open().is_ok() {
                            return true;
                        }
                    }
                    false
                })
                .await;

            if !reconnected {
                let _ = entity.update(cx, |this, cx| {
                    this.loading = false;
                    let _ = status_handle.update(cx, |d, cx| {
                        d.set_error(
                            "Timeout waiting for device reconnection. Reset canceled.".to_string(),
                            cx,
                        );
                    });
                    cx.notify();
                });
                return;
            }

            let _ = status_handle.update(cx, |d, cx| {
                d.set_loading("Touch your security key now to confirm the reset...", cx);
            });

            let result = cx
                .background_executor()
                .spawn(async move { io::reset_device() })
                .await;

            let _ = entity.update(cx, |this, cx| match result {
                Ok(msg) => {
                    log::info!("Device Reset: {}", msg);
                    this.lock_storage(cx);
                    let _ = status_handle.update(cx, |d, cx| {
                        d.set_success(msg, cx);
                    });
                    cx.emit(PasskeysEvent::Notification(
                        "Device reset successfully".into(),
                    ));
                    this.lock_storage(cx);
                    this.sync_fido_state(None, cx);
                }
                Err(e) => {
                    log::error!("Error resetting device: {}", e);
                    this.loading = false;
                    let _ = status_handle.update(cx, |d, cx| {
                        d.set_error(format!("Reset failed: {}", e), cx);
                    });
                    cx.notify();
                }
            });
        }));
    }

    pub(super) fn open_ask_delete_pin(
        &mut self,
        cred: StoredCredential,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(pin) = &self.cached_pin {
            self.open_delete_dialog(&cred, pin.clone(), window, cx);
        } else {
            window.push_notification("Session expired, please unlock again.", cx);
            self.lock_storage(cx);
        }
    }

    pub(super) fn open_credential_details(
        &mut self,
        cred: &StoredCredential,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let title = if !cred.rp_name.is_empty() {
            cred.rp_name.clone()
        } else if !cred.rp_id.is_empty() {
            cred.rp_id.clone()
        } else {
            "Passkey Details".to_string()
        };
        let rp_id = cred.rp_id.clone();
        let user_name = cred.user_name.clone();
        let display_name = if cred.user_display_name.is_empty() {
            "N/A".to_string()
        } else {
            cred.user_display_name.clone()
        };
        let user_id = cred.user_id.clone();
        let credential_id = cred.credential_id.clone();

        window.open_sheet_at(
            gpui_component::Placement::Bottom,
            cx,
            move |sheet, _, cx| {
                let theme = cx.theme();

                let header_row = gpui_component::h_flex()
                    .gap_3()
                    .p_4()
                    .bg(theme.muted.opacity(0.3))
                    .border_1()
                    .border_color(theme.border)
                    .rounded_lg()
                    .child(
                        div()
                            .size_12()
                            .rounded_full()
                            .bg(theme.primary.opacity(0.1))
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(
                                gpui_component::Icon::default()
                                    .path("icons/key-round.svg")
                                    .text_color(theme.primary)
                                    .size_6(),
                            ),
                    )
                    .child(
                        gpui_component::v_flex()
                            .child(div().font_semibold().child(rp_id.clone()))
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(theme.muted_foreground)
                                    .font_family("monospace")
                                    .child(user_name.clone()),
                            ),
                    );

                let separator = div().w_full().h(px(1.)).bg(theme.border);

                let detail_field = |label: &str, value: String, mono: bool| {
                    let value_el = if mono {
                        div()
                            .text_xs()
                            .font_family("monospace")
                            .bg(theme.muted)
                            .p_2()
                            .rounded_md()
                            .overflow_hidden()
                            .child(value)
                            .into_any_element()
                    } else {
                        div()
                            .text_sm()
                            .font_medium()
                            .child(value)
                            .into_any_element()
                    };
                    gpui_component::v_flex()
                        .gap_1()
                        .child(
                            div()
                                .text_sm()
                                .font_medium()
                                .text_color(theme.muted_foreground)
                                .child(label.to_string()),
                        )
                        .child(value_el)
                };

                let description = gpui_component::h_flex()
                    .gap_1()
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child("Credential details for user"),
                    )
                    .child(
                        div()
                            .text_sm()
                            .font_semibold()
                            .text_color(theme.foreground)
                            .child(user_name.clone()),
                    );

                sheet
                    .title(
                        div().w_full().child(
                            gpui_component::v_flex()
                                .mx_auto()
                                .max_w(px(512.))
                                .px_4()
                                .gap_0p5()
                                .child(div().text_2xl().font_bold().child(title.clone()))
                                .child(description),
                        ),
                    )
                    .size(px(500.))
                    .resizable(false)
                    .margin_top(px(0.))
                    .child(
                        div().mx_auto().max_w(px(512.)).w_full().px_4().child(
                            gpui_component::v_flex()
                                .gap_4()
                                .child(header_row)
                                .child(separator)
                                .child(detail_field("Display Name", display_name.clone(), false))
                                .child(detail_field("User ID (Hex)", user_id.clone(), true))
                                .child(detail_field(
                                    "Credential ID (Hex)",
                                    credential_id.clone(),
                                    true,
                                )),
                        ),
                    )
            },
        );
    }
}

struct SliderLabel {
    slider: Entity<gpui_component::slider::SliderState>,
}

impl Render for SliderLabel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let val = self.slider.read(cx).value().start() as u8;
        format!("Minimum PIN Length ({})", val)
    }
}
