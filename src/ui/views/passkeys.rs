use crate::device::io;
use crate::device::types::{FidoDeviceInfo, FullDeviceStatus, StoredCredential};
use crate::ui::components::{
    button::{PFButton, PFIconButton},
    card::Card,
    dialog,
    dialog::{ChangePinContent, ConfirmContent, PinPromptContent},
    page_view::PageView,
};
use gpui::*;
use gpui_component::button::{Button, ButtonVariant, ButtonVariants};
use gpui_component::{
    ActiveTheme, Icon, Placement, Sizable, StyledExt, Theme, WindowExt,
    badge::Badge,
    h_flex,
    input::{Input, InputState},
    slider::{Slider, SliderState},
    v_flex,
};

struct SliderLabel {
    slider: Entity<SliderState>,
}

impl Render for SliderLabel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let val = self.slider.read(cx).value().start() as u8;
        format!("Minimum PIN Length ({})", val)
    }
}

pub struct PasskeysView {
    device_status: Option<FullDeviceStatus>,
    fido_info: Option<FidoDeviceInfo>,
    credentials: Vec<StoredCredential>,
    unlocked: bool,
    cached_pin: Option<String>,
    loading: bool,

    _task: Option<Task<()>>,
}

pub enum PasskeysEvent {
    Notification(String),
    CloseDialog,
}

impl EventEmitter<PasskeysEvent> for PasskeysView {}

impl PasskeysView {
    pub fn new(
        _window: &mut Window,
        _cx: &mut Context<Self>,
        device_status: Option<FullDeviceStatus>,
        fido_info: Option<FidoDeviceInfo>,
    ) -> Self {
        Self {
            device_status,
            fido_info,
            credentials: Vec::new(),
            unlocked: false,
            cached_pin: None,
            loading: false,
            _task: None,
        }
    }

    pub fn update_device_status(
        &mut self,
        status: Option<FullDeviceStatus>,
        fido_info: Option<FidoDeviceInfo>,
        cx: &mut Context<Self>,
    ) {
        if self.device_status == status && self.fido_info == fido_info {
            return;
        }
        self.device_status = status;
        self.fido_info = fido_info;
        cx.notify();
    }

    fn unlock_storage(
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

    fn lock_storage(&mut self, cx: &mut Context<Self>) {
        self.unlocked = false;
        self.cached_pin = None;
        self.credentials.clear();
        cx.notify();
    }

    fn execute_delete(
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
            let pin_for_bg = pin.clone();
            let result = cx
                .background_executor()
                .spawn(async move { io::delete_credential(pin_for_bg, credential_id) })
                .await;

            let _ = entity.update(cx, |this, cx| match result {
                Ok(_) => {
                    log::info!("Credential deleted successfully.");
                    this.refresh_credentials(pin, cx);
                    let _ = dialog_handle.update(cx, |d, cx| {
                        d.set_success("Credential deleted successfully.".to_string(), cx);
                    });
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

    fn open_unlock_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let view_handle = cx.entity().downgrade();

        dialog::open_pin_prompt(
            "Unlock Storage",
            "Enter your device PIN to view saved passkeys",
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
            ButtonVariant::Danger,
            window,
            cx,
            move |dialog_handle, cx| {
                let _ = view_handle.update(cx, |this, cx| {
                    this.execute_delete(cred_id.clone(), pin_str.clone(), dialog_handle, cx);
                });
            },
        );
    }

    fn open_change_pin_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let view_handle = cx.entity().downgrade();

        dialog::open_change_pin(window, cx, move |current, new, dialog_handle, cx| {
            let _ = view_handle.update(cx, |this, cx| {
                this.change_pin(current, new, dialog_handle, cx);
            });
        });
    }

    fn open_min_pin_length_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let current_min = self
            .fido_info
            .as_ref()
            .map(|f| f.min_pin_length)
            .unwrap_or(4);

        let slider = cx.new(|_| {
            SliderState::new()
                .min(4.0)
                .max(63.0)
                .step(1.0)
                .default_value(current_min as f32)
        });

        let current_pin = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Enter current PIN")
                .masked(true)
        });
        let new_pin = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Enter new PIN")
                .masked(true)
        });
        let confirm_pin = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Confirm new PIN")
                .masked(true)
        });

        // Create the label view
        let label_view = cx.new(|_cx| SliderLabel {
            slider: slider.clone(),
        });

        let view_handle = cx.entity().downgrade();

        window.open_dialog(cx, move |dialog, _, _| {
            let view = view_handle.clone();
            let current = current_pin.clone();
            let new = new_pin.clone();
            let confirm = confirm_pin.clone();
            let slider_handle = slider.clone();

            dialog
                .title("Update Minimum PIN Length")
                .child(
                    "Set the minimum allowed PIN length (4-63 characters) and enter a new PIN that meets this requirement.",
                )
                .child(
                    v_flex()
                        .gap_4()
                        .pb_4()
                        .child(
                             v_flex()
                                .gap_2()
                                .child(label_view.clone())
                                .child(Slider::new(&slider_handle))
                        )
                        .child("Current PIN")
                        .child(Input::new(&current))
                        .child(
                             v_flex()
                                 .gap_2()
                                 .child(format!("New PIN (min {} chars)", current_min)) 
                                 .child(Input::new(&new))
                        )
                        .child("Confirm New PIN")
                        .child(Input::new(&confirm)),
                )
                .footer(move |_, _window, _cx, _| {
                    let view = view.clone();
                    let current = current.clone();
                    let new = new.clone();
                    let confirm = confirm.clone();
                    let slider = slider_handle.clone();

                    vec![
                        Button::new("cancel")
                            .label("Cancel")
                            .on_click(|_, window, cx| window.close_dialog(cx)),
                        Button::new("update")
                            .primary()
                            .label("Update")
                            .on_click(move |_, _, cx| {
                                let current_val = current.read(cx).text().to_string();
                                let new_val = new.read(cx).text().to_string();
                                let confirm_val = confirm.read(cx).text().to_string();
                                let min_len = slider.read(cx).value().start() as u8;

                                if current_val.is_empty() {
                                    return;
                                }

                                if !new_val.is_empty() {
                                    if new_val != confirm_val {
                                        let _ = view.update(cx, |_, cx| {
                                            cx.emit(PasskeysEvent::Notification("PINs do not match".to_string()));
                                        });
                                        return;
                                    }
                                    if new_val.len() < min_len as usize {
                                        let _ = view.update(cx, |_, cx| {
                                            cx.emit(PasskeysEvent::Notification(format!("PIN must be at least {} characters", min_len)));
                                        });
                                        return;
                                    }
                                }
                                let _ = view.update(cx, |this, cx| {
                                    this.update_min_length(current_val, min_len, new_val, cx);
                                });
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

        self._task = Some(cx.spawn(async move |_, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { io::change_fido_pin(Some(current), new) })
                .await;

            let _ = entity.update(cx, |this, cx| {
                this.loading = false;
                match result {
                    Ok(msg) => {
                        log::info!("PIN changed: {}", msg);
                        if let Ok(info) = io::get_fido_info() {
                            this.fido_info = Some(info);
                        }
                        let _ = dialog_handle.update(cx, |d, cx| {
                            d.set_success("PIN changed successfully.".to_string(), cx);
                        });
                    }
                    Err(e) => {
                        log::error!("PIN change failed: {}", e);
                        let _ = dialog_handle.update(cx, |d, cx| {
                            d.set_error(format!("Error: {}", e), cx);
                        });
                    }
                }
                cx.notify();
            });
        }));
    }

    fn update_min_length(
        &mut self,
        current: String,
        min_len: u8,
        new_pin: String,
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
            // 1. Set Min Length
            let current_for_bg = current.clone();
            let res_len = cx
                .background_executor()
                .spawn(async move { io::set_min_pin_length(current_for_bg, min_len) })
                .await;

            if let Err(e) = res_len {
                log::error!("Failed to set minimum PIN length: {}", e);
                let _ = entity.update(cx, |this, cx| {
                    this.loading = false;
                    cx.emit(PasskeysEvent::Notification(format!(
                        "Failed to set length: {}",
                        e
                    )));
                    cx.notify();
                });
                return;
            }

            if !new_pin.is_empty() {
                let res_pin = cx
                    .background_executor()
                    .spawn(async move { io::change_fido_pin(Some(current), new_pin) })
                    .await;
                let _ = entity.update(cx, |this, cx| {
                    this.loading = false;
                    match res_pin {
                        Ok(_) => {
                            log::info!("Minimum length and PIN updated successfully.");
                            cx.emit(PasskeysEvent::CloseDialog);
                            cx.emit(PasskeysEvent::Notification(
                                "Minimum length and PIN updated".to_string(),
                            ));
                            if let Ok(info) = io::get_fido_info() {
                                this.fido_info = Some(info);
                            }
                        }
                        Err(e) => {
                            log::error!("Length set, but PIN change failed: {}", e);
                            cx.emit(PasskeysEvent::Notification(format!(
                                "Length set, but PIN change failed: {}",
                                e
                            )));
                        }
                    }
                    cx.notify();
                });
            } else {
                let _ = entity.update(cx, |this, cx| {
                    this.loading = false;
                    log::info!("Minimum PIN length updated to {}.", min_len);
                    cx.emit(PasskeysEvent::CloseDialog);
                    cx.emit(PasskeysEvent::Notification(format!(
                        "Minimum length updated to {}",
                        min_len
                    )));
                    if let Ok(info) = io::get_fido_info() {
                        this.fido_info = Some(info);
                    }
                    cx.notify();
                });
            }
        }));
    }

    fn render_no_device(&self, theme: &Theme) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .justify_center()
            .h_64()
            .border_1()
            .border_color(theme.border)
            .rounded_xl()
            .child(
                div()
                    .text_color(theme.muted_foreground)
                    .child("Connect your pico-key to manage passkeys."),
            )
            .into_any_element()
    }

    fn render_not_supported(&self, theme: &Theme) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .justify_center()
            .h_64()
            .border_1()
            .border_color(theme.border)
            .rounded_xl()
            .child(
                div()
                    .text_color(theme.muted_foreground)
                    .child("FIDO Passkeys are not supported on this device."),
            )
            .into_any_element()
    }

    fn render_pin_management(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let status_row = self.render_pin_status_row(cx).into_any_element();
        let min_len_row = self.render_min_pin_length_row(cx).into_any_element();

        Card::new()
            .title("PIN Management")
            .icon(Icon::default().path("icons/key.svg"))
            .description("Configure FIDO2 PIN security")
            .child(v_flex().gap_4().child(status_row).child(min_len_row))
    }

    fn render_pin_status_row(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let pin_set = self
            .fido_info
            .as_ref()
            .and_then(|f| f.options.get("clientPin").copied())
            .unwrap_or(false);

        let listener = cx.listener(|this, _, window, cx| {
            this.open_change_pin_dialog(window, cx);
        });

        let theme = cx.theme();

        div()
            .flex()
            .items_center()
            .justify_between()
            .p_4()
            .border_1()
            .border_color(theme.border)
            .rounded_lg()
            .child(
                v_flex()
                    .child(div().font_medium().child("Current PIN Status"))
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child(if pin_set {
                                "PIN is set"
                            } else {
                                "No PIN configured"
                            }),
                    ),
            )
            .child(
                PFButton::new(if pin_set { "Change PIN" } else { "Set PIN" })
                    .id("change-pin-btn")
                    .with_colors(rgb(0x222225), rgb(0x2a2a2d), rgb(0x333336))
                    .on_click(listener),
            )
    }

    fn render_min_pin_length_row(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let min_len = self
            .fido_info
            .as_ref()
            .map(|f| f.min_pin_length)
            .unwrap_or(4);
        let pin_set = self
            .fido_info
            .as_ref()
            .and_then(|f| f.options.get("clientPin").copied())
            .unwrap_or(false);

        let theme = cx.theme();

        div()
            .flex()
            .items_center()
            .justify_between()
            .p_4()
            .border_1()
            .border_color(theme.border)
            .rounded_lg()
            .child(
                v_flex()
                    .child(div().font_medium().child("Minimum PIN Length"))
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child(format!("Current: {} characters", min_len)),
                    ),
            )
            .child(
                PFButton::new("Update Minimum Length")
                    .id("update-min-len-btn")
                    .with_colors(rgb(0x222225), rgb(0x2a2a2d), rgb(0x333336))
                    .disabled(!pin_set)
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.open_min_pin_length_dialog(window, cx);
                    })),
            )
    }

    fn render_stored_passkeys(&self, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.unlocked {
            self.render_locked_state(cx).into_any_element()
        } else {
            self.render_unlocked_state(cx).into_any_element()
        }
    }

    fn render_locked_state(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let listener = cx.listener(|this, _, window, cx| {
            this.open_unlock_dialog(window, cx);
        });
        let theme = cx.theme();

        Card::new()
            .title("Stored Passkeys")
            .icon(Icon::default().path("icons/key-round.svg"))
            .description("View and manage your resident credentials")
            .child(
                v_flex()
                    .items_center()
                    .justify_center()
                    .gap_3()
                    .py_3()
                    .child(
                        div().rounded_full().bg(theme.muted).p_4().child(
                            Icon::default()
                                .path("icons/shield.svg")
                                .size_12()
                                .text_color(theme.muted_foreground),
                        ),
                    )
                    .child(
                        div()
                            .text_lg()
                            .font_semibold()
                            .child("Authentication Required"),
                    )
                    .child(
                        div()
                            .text_color(theme.muted_foreground)
                            .text_sm()
                            .child("Unlock your device to view and manage passkeys."),
                    )
                    .child(
                        PFIconButton::new(
                            Icon::default().path("icons/lock-open.svg"),
                            "Unlock Storage",
                        )
                        .on_click(listener)
                        .with_colors(rgb(0xe4e4e7), rgb(0xd0d0d3), rgb(0xe4e4e7))
                        .with_text_color(rgb(0x18181b)),
                    ),
            )
    }

    fn render_unlocked_state(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let creds_len = self.credentials.len();
        let lock_listener = cx.listener(|this, _, _, cx| {
            this.lock_storage(cx);
        });

        let mut cards = Vec::new();
        for cred in &self.credentials {
            cards.push(self.render_credential_card(cred, cx).into_any_element());
        }

        let theme = cx.theme();

        Card::new()
            .title("Stored Passkeys")
            .icon(Icon::default().path("icons/key-round.svg"))
            .description("View and manage your resident credentials")
            .child(
                v_flex()
                    .gap_6()
                    .child(
                        h_flex()
                            .justify_between()
                            .items_center()
                            .child(
                                h_flex()
                                    .gap_4()
                                    .items_center()
                                    .child(
                                        Badge::new()
                                            .child(
                                                h_flex()
                                                    .gap_1()
                                                    .items_center()
                                                    .child(
                                                        Icon::default()
                                                            .path("icons/lock-open.svg")
                                                            .size_3p5(),
                                                    )
                                                    .child("Unlocked"),
                                            )
                                            .color(gpui::green()),
                                    )
                                    .child(div().w_px().h_4().bg(theme.border))
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(theme.muted_foreground)
                                            .child(format!("{} credentials stored", creds_len)),
                                    ),
                            )
                            .child(
                                PFIconButton::new(
                                    Icon::default().path("icons/lock.svg").size_3p5(),
                                    "Lock Storage",
                                )
                                .small()
                                .on_click(lock_listener),
                            ),
                    )
                    .child(if self.credentials.is_empty() {
                        self.render_empty_credentials_with_theme(theme)
                            .into_any_element()
                    } else {
                        div()
                            .grid()
                            .grid_cols(3)
                            .gap_4()
                            .children(cards)
                            .into_any_element()
                    }),
            )
    }

    fn render_empty_credentials_with_theme(&self, theme: &Theme) -> impl IntoElement {
        v_flex()
            .items_center()
            .justify_center()
            .py_12()
            .border_1()
            .border_color(theme.border)
            .rounded_xl()
            .gap_4()
            .child(
                div()
                    .rounded_full()
                    .bg(theme.muted)
                    .p_4()
                    .child(
                        Icon::default()
                            .path("icons/key-round.svg")
                            .size_8()
                            .text_color(theme.muted_foreground),
                    ),
            )
            .child(div().text_lg().font_semibold().child("No Passkeys Found"))
            .child(
                div()
                    .text_color(theme.muted_foreground)
                    .text_sm()
                    .text_center()
                    .max_w(px(384.0))
                    .child("This device doesn't have any resident credentials stored yet. Create passkeys on websites to see them here."),
            )
    }

    fn render_credential_card(
        &self,
        cred: &StoredCredential,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let cred_clone = cred.clone();
        let cred_for_click = cred.clone();

        let delete_listener = cx.listener(move |this, _, window, cx| {
            this.open_ask_delete_pin(cred_clone.clone(), window, cx);
        });

        let click_listener = cx.listener(move |this, _, window, cx| {
            this.open_credential_details(&cred_for_click, window, cx);
        });

        let theme = cx.theme();

        div()
            .id(SharedString::from(format!(
                "cred-card-{}",
                cred.credential_id
            )))
            .cursor_pointer()
            .on_click(click_listener)
            .border_1()
            .border_color(theme.border)
            .rounded_xl()
            .p_4()
            .hover(|s| s.bg(theme.accent).border_color(theme.primary))
            .child(
                h_flex()
                    .justify_between()
                    .items_center()
                    .child(
                        h_flex()
                            .gap_3()
                            .items_center()
                            .flex_1()
                            .min_w_0()
                            .child(
                                div()
                                    .size_10()
                                    .rounded_md()
                                    .bg(rgb(0x3b3b3e))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(
                                        Icon::default()
                                            .path("icons/key-round.svg")
                                            .text_color(theme.primary)
                                            .size_5(),
                                    ),
                            )
                            .child(
                                v_flex()
                                    .min_w_0()
                                    .overflow_hidden()
                                    .child(
                                        div()
                                            .font_semibold()
                                            .whitespace_nowrap()
                                            .overflow_hidden()
                                            .text_ellipsis()
                                            .child(if !cred.rp_name.is_empty() {
                                                cred.rp_name.clone()
                                            } else if !cred.rp_id.is_empty() {
                                                cred.rp_id.clone()
                                            } else {
                                                "Unknown Service".to_string()
                                            }),
                                    )
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(theme.muted_foreground)
                                            .whitespace_nowrap()
                                            .overflow_hidden()
                                            .text_ellipsis()
                                            .child(cred.user_name.clone()),
                                    ),
                            ),
                    )
                    .child(
                        Button::new("delete-cred-btn")
                            .ghost()
                            .small()
                            .child(
                                Icon::default()
                                    .path("icons/trash-2.svg")
                                    .size_4()
                                    .text_color(theme.muted_foreground),
                            )
                            .on_click(delete_listener),
                    ),
            )
    }

    fn open_credential_details(
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

        window.open_sheet_at(Placement::Bottom, cx, move |sheet, _, cx| {
            let theme = cx.theme();

            let header_row = h_flex()
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
                            Icon::default()
                                .path("icons/key-round.svg")
                                .text_color(theme.primary)
                                .size_6(),
                        ),
                )
                .child(
                    v_flex()
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
                let mut value_el = div().text_sm().font_medium().child(value.clone());
                if mono {
                    value_el = div()
                        .text_xs()
                        .font_family("monospace")
                        .bg(theme.muted)
                        .p_2()
                        .rounded_md()
                        .overflow_hidden()
                        .child(value);
                }
                v_flex()
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

            let description = h_flex()
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
                        v_flex()
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
                        v_flex()
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
        });
    }

    fn open_ask_delete_pin(
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
}

impl Render for PasskeysView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let device_connected = self.device_status.is_some();
        if !device_connected {
            let theme = cx.theme();
            return PageView::build(
                "Passkeys",
                "Manage your security PIN and the FIDO credentials (passkeys) stored on your device.",
                self.render_no_device(theme).into_any_element(),
                theme,
            )
            .into_any_element();
        }

        let has_fido = self
            .device_status
            .as_ref()
            .map(|s| s.method == crate::device::types::DeviceMethod::Fido)
            .unwrap_or(false)
            || self.fido_info.is_some();

        if !has_fido {
            let theme = cx.theme();
            return PageView::build(
                "Passkeys",
                "Manage your security PIN and the FIDO credentials (passkeys) stored on your device.",
                self.render_not_supported(theme).into_any_element(),
                theme,
            )
            .into_any_element();
        }

        let content = v_flex()
            .gap_6()
            .child(self.render_pin_management(cx))
            .child(self.render_stored_passkeys(cx));

        let theme = cx.theme();

        div()
            .size_full()
            .relative()
            .child(PageView::build(
                "Passkeys",
                "Manage your security PIN and the FIDO credentials (passkeys) stored on your device.",
                content.into_any_element(),
                theme,
            ))
            .into_any_element()
    }
}
