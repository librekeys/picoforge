use crate::device::io;
use crate::device::types::{FingerprintStatus, FingerprintTemplate};
use crate::ui::components::{
    button::{PFButton, PFIconButton},
    card::Card,
    dialog,
    dialog::{ConfirmContent, PinPromptContent, StatusContent, TextPromptContent},
    page_view::PageView,
    tag::Tag,
};
use crate::ui::rootview::ApplicationRoot;
use crate::ui::types::DeviceConnectionState;
use gpui::*;
use gpui_component::button::ButtonVariant;
use gpui_component::{
    ActiveTheme, Icon, StyledExt, Theme, WindowExt, badge::Badge, h_flex, v_flex,
};

pub struct SecurityView {
    root: WeakEntity<ApplicationRoot>,
    fingerprint_status: Option<FingerprintStatus>,
    cached_pin: Option<String>,
    loading: bool,
    _task: Option<Task<()>>,
}

impl SecurityView {
    pub fn new(
        _window: &mut Window,
        _cx: &mut Context<Self>,
        root: WeakEntity<ApplicationRoot>,
    ) -> Self {
        Self {
            root,
            fingerprint_status: None,
            cached_pin: None,
            loading: false,
            _task: None,
        }
    }

    pub fn refresh_status(&mut self, pin: Option<String>, cx: &mut Context<Self>) {
        if self.loading {
            return;
        }

        self.loading = true;
        cx.notify();

        let entity = cx.entity().downgrade();
        self._task = Some(cx.spawn(async move |_, cx| {
            let pin_for_bg = pin.clone();
            let result = cx
                .background_executor()
                .spawn(async move { io::get_fingerprint_status(pin_for_bg) })
                .await;

            let _ = entity.update(cx, |this, cx| {
                this.loading = false;
                match result {
                    Ok(status) => {
                        this.cached_pin = pin;
                        this.fingerprint_status = Some(status);
                    }
                    Err(err) => {
                        log::error!("Failed to refresh fingerprint status: {}", err);
                        this.cached_pin = None;
                        this.fingerprint_status = None;
                    }
                }
                cx.notify();
            });
        }));
    }

    fn lock_session(&mut self, cx: &mut Context<Self>) {
        self.cached_pin = None;
        if let Some(status) = &mut self.fingerprint_status {
            status.templates_loaded = false;
            status.templates.clear();
        }
        cx.notify();
    }

    fn open_unlock_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.loading {
            return;
        }

        let view_handle = cx.entity().downgrade();
        dialog::open_pin_prompt(
            "Unlock Fingerprint Management",
            "Enter your device PIN to enumerate fingerprints stored on the key.",
            "Load",
            window,
            cx,
            move |pin, dialog_handle, cx| {
                let _ = view_handle.update(cx, |this, cx| {
                    this.unlock_with_pin(pin, dialog_handle, cx);
                });
            },
        );
    }

    fn unlock_with_pin(
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

        let entity = cx.entity().downgrade();
        self._task = Some(cx.spawn(async move |_, cx| {
            let pin_for_bg = pin.clone();
            let result = cx
                .background_executor()
                .spawn(async move { io::get_fingerprint_status(Some(pin_for_bg)) })
                .await;

            let _ = entity.update(cx, |this, cx| {
                this.loading = false;
                match result {
                    Ok(status) => {
                        this.cached_pin = Some(pin);
                        this.fingerprint_status = Some(status);
                        let _ = dialog_handle.update(cx, |d, cx| {
                            d.set_success("Fingerprint list refreshed.".to_string(), cx);
                        });
                    }
                    Err(err) => {
                        log::error!("Failed to unlock fingerprint management: {}", err);
                        let _ = dialog_handle.update(cx, |d, cx| {
                            d.set_error(format!("Failed to load fingerprints: {}", err), cx);
                        });
                    }
                }
                cx.notify();
            });
        }));
    }

    fn open_enroll_flow(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(pin) = self.cached_pin.clone() else {
            window.push_notification("Unlock fingerprint management first.", cx);
            return;
        };

        if self.loading {
            return;
        }

        let status_handle = dialog::open_status_dialog("Enroll Fingerprint", window, cx);
        self.enroll_with_pin(pin, status_handle, cx);
    }

    fn enroll_with_pin(
        &mut self,
        pin: String,
        status_handle: WeakEntity<StatusContent>,
        cx: &mut Context<Self>,
    ) {
        if self.loading {
            return;
        }

        self.loading = true;
        cx.notify();

        let entity = cx.entity().downgrade();
        self._task = Some(cx.spawn(async move |_, cx| {
            let pin_for_bg = pin.clone();
            let result = cx
                .background_executor()
                .spawn(async move { io::enroll_fingerprint(pin_for_bg, Some(20_000)) })
                .await;

            let _ = entity.update(cx, |this, cx| {
                this.loading = false;
                match result {
                    Ok(result) => {
                        this.cached_pin = Some(pin.clone());
                        let _ = status_handle.update(cx, |status, cx| {
                            status.set_success(
                                format!(
                                    "Fingerprint enrolled as template {}. Touch the sensor twice during capture.",
                                    result.template_id
                                ),
                                cx,
                            );
                        });
                        this.refresh_status(Some(pin), cx);
                    }
                    Err(err) => {
                        log::error!("Fingerprint enrollment failed: {}", err);
                        let _ = status_handle.update(cx, |status, cx| {
                            status.set_error(format!("Enrollment failed: {}", err), cx);
                        });
                        cx.notify();
                    }
                }
            });
        }));
    }

    fn open_rename_dialog(
        &mut self,
        template: FingerprintTemplate,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(pin) = self.cached_pin.clone() else {
            window.push_notification("Unlock fingerprint management first.", cx);
            return;
        };

        let template_id = template.template_id.clone();
        let current_name = template.friendly_name.clone();
        let view_handle = cx.entity().downgrade();

        dialog::open_text_prompt(
            "Rename Fingerprint",
            "Provide a friendly name for this fingerprint template.",
            "Friendly name",
            "Save",
            current_name,
            window,
            cx,
            move |friendly_name, dialog_handle, cx| {
                let _ = view_handle.update(cx, |this, cx| {
                    this.rename_fingerprint(
                        pin.clone(),
                        template_id.clone(),
                        friendly_name,
                        dialog_handle,
                        cx,
                    );
                });
            },
        );
    }

    fn rename_fingerprint(
        &mut self,
        pin: String,
        template_id: String,
        friendly_name: String,
        dialog_handle: WeakEntity<TextPromptContent>,
        cx: &mut Context<Self>,
    ) {
        if self.loading {
            return;
        }

        self.loading = true;
        cx.notify();

        let entity = cx.entity().downgrade();
        self._task = Some(cx.spawn(async move |_, cx| {
            let pin_for_bg = pin.clone();
            let template_id_for_bg = template_id.clone();
            let friendly_name_for_bg = friendly_name.clone();
            let result = cx.background_executor().spawn(async move {
                io::rename_fingerprint(pin_for_bg, template_id_for_bg, friendly_name_for_bg)
            });

            let result = result.await;

            let _ = entity.update(cx, |this, cx| {
                this.loading = false;
                match result {
                    Ok(_) => {
                        this.cached_pin = Some(pin.clone());
                        let _ = dialog_handle.update(cx, |dialog, cx| {
                            dialog.set_success("Fingerprint name updated.".to_string(), cx);
                        });
                        this.refresh_status(Some(pin), cx);
                    }
                    Err(err) => {
                        log::error!("Failed to rename fingerprint: {}", err);
                        let _ = dialog_handle.update(cx, |dialog, cx| {
                            dialog.set_error(format!("Rename failed: {}", err), cx);
                        });
                        cx.notify();
                    }
                }
            });
        }));
    }

    fn open_delete_dialog(
        &mut self,
        template: FingerprintTemplate,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(pin) = self.cached_pin.clone() else {
            window.push_notification("Unlock fingerprint management first.", cx);
            return;
        };

        let template_id = template.template_id.clone();
        let label = template
            .friendly_name
            .clone()
            .unwrap_or_else(|| template_id.clone());
        let view_handle = cx.entity().downgrade();

        dialog::open_confirm(
            "Delete Fingerprint",
            format!("Delete fingerprint template {}?", label),
            "Delete",
            ButtonVariant::Danger,
            window,
            cx,
            move |dialog_handle, cx| {
                let _ = view_handle.update(cx, |this, cx| {
                    this.delete_fingerprint(pin.clone(), template_id.clone(), dialog_handle, cx);
                });
            },
        );
    }

    fn delete_fingerprint(
        &mut self,
        pin: String,
        template_id: String,
        dialog_handle: WeakEntity<ConfirmContent>,
        cx: &mut Context<Self>,
    ) {
        if self.loading {
            return;
        }

        self.loading = true;
        cx.notify();

        let entity = cx.entity().downgrade();
        self._task = Some(cx.spawn(async move |_, cx| {
            let pin_for_bg = pin.clone();
            let template_id_for_bg = template_id.clone();
            let result = cx
                .background_executor()
                .spawn(async move { io::remove_fingerprint(pin_for_bg, template_id_for_bg) })
                .await;

            let _ = entity.update(cx, |this, cx| {
                this.loading = false;
                match result {
                    Ok(_) => {
                        this.cached_pin = Some(pin.clone());
                        let _ = dialog_handle.update(cx, |dialog, cx| {
                            dialog.set_success("Fingerprint removed successfully.".to_string(), cx);
                        });
                        this.refresh_status(Some(pin), cx);
                    }
                    Err(err) => {
                        log::error!("Failed to delete fingerprint: {}", err);
                        let _ = dialog_handle.update(cx, |dialog, cx| {
                            dialog.set_error(format!("Delete failed: {}", err), cx);
                        });
                        cx.notify();
                    }
                }
            });
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
                    .child("Connect your pico-key to manage biometric security."),
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
                    .child("Biometric management is not supported on this device."),
            )
            .into_any_element()
    }

    fn render_fingerprint_card(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let status = self.fingerprint_status.clone();
        let unlocked = self.cached_pin.is_some()
            && status
                .as_ref()
                .map(|s| s.templates_loaded)
                .unwrap_or(false);
        let muted_fg = cx.theme().muted_foreground;

        let header_right = if unlocked {
            Badge::new()
                .child(
                    h_flex()
                        .gap_1()
                        .items_center()
                        .child(Icon::default().path("icons/lock-open.svg").size_3p5())
                        .child("Unlocked"),
                )
                .color(gpui::green())
                .into_any_element()
        } else {
            Tag::new("PIN required").into_any_element()
        };

        let sensor_chips = if let Some(sensor) = status.as_ref().and_then(|s| s.sensor.clone()) {
            h_flex()
                .gap_2()
                .flex_wrap()
                .child(Tag::new(sensor.modality))
                .child(Tag::new(format!("{} sensor", sensor.fingerprint_kind)))
                .child(Tag::new(format!(
                    "{} samples / enroll",
                    sensor.max_capture_samples_required_for_enroll
                )))
                .child(Tag::new(format!(
                    "{} byte names",
                    sensor.max_template_friendly_name
                )))
                .into_any_element()
        } else {
            div()
                .text_sm()
                .text_color(muted_fg)
                .child("Sensor information unavailable.")
                .into_any_element()
        };

        let body = if !unlocked {
            self.render_locked_state(cx).into_any_element()
        } else {
            self.render_unlocked_state(cx).into_any_element()
        };

        Card::new()
            .title("Fingerprints")
            .description("Enumerate, enroll, rename, and delete fingerprints stored on the key.")
            .icon(Icon::default().path("icons/shield.svg"))
            .header_right(header_right)
            .child(v_flex().gap_4().child(sensor_chips).child(body))
    }

    fn render_locked_state(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let muted = cx.theme().muted;
        let muted_fg = cx.theme().muted_foreground;
        let pin_set = self
            .root
            .upgrade()
            .and_then(|r| r.read(cx).device.fido_info.clone())
            .and_then(|info| info.options.get("clientPin").copied())
            .unwrap_or(false);

        if !pin_set {
            return v_flex()
                .items_center()
                .justify_center()
                .gap_3()
                .py_3()
                .child(
                    div().rounded_full().bg(muted).p_4().child(
                        Icon::default()
                            .path("icons/key.svg")
                            .size_12()
                            .text_color(muted_fg),
                    ),
                )
                .child(div().text_lg().font_semibold().child("PIN Required"))
                .child(
                    div()
                        .text_color(muted_fg)
                        .text_sm()
                        .child("Set a FIDO PIN in the Passkeys view before managing fingerprints."),
                )
                .into_any_element();
        }

        let listener = cx.listener(|this, _, window, cx| {
            this.open_unlock_dialog(window, cx);
        });

        v_flex()
            .items_center()
            .justify_center()
            .gap_3()
            .py_3()
            .child(
                div().rounded_full().bg(muted).p_4().child(
                    Icon::default()
                        .path("icons/lock.svg")
                        .size_12()
                        .text_color(muted_fg),
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
                    .text_color(muted_fg)
                    .text_sm()
                    .child("Unlock with your device PIN to view and manage enrolled fingerprints."),
            )
            .child(
                PFIconButton::new(Icon::default().path("icons/lock-open.svg"), "Unlock Fingerprints")
                    .on_click(listener)
                    .with_colors(rgb(0xe4e4e7), rgb(0xd0d0d3), rgb(0xe4e4e7))
                    .with_text_color(rgb(0x18181b))
                    .loading(self.loading),
            )
            .into_any_element()
    }

    fn render_unlocked_state(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let templates = self
            .fingerprint_status
            .as_ref()
            .map(|status| status.templates.clone())
            .unwrap_or_default();
        let border = cx.theme().border;
        let muted = cx.theme().muted;
        let muted_fg = cx.theme().muted_foreground;
        let count = templates.len();
        let lock_listener = cx.listener(|this, _, _, cx| {
            this.lock_session(cx);
        });
        let enroll_listener = cx.listener(|this, _, window, cx| {
            this.open_enroll_flow(window, cx);
        });

        let mut rows = Vec::new();
        for template in templates {
            rows.push(self.render_template_card(template, cx).into_any_element());
        }

        let content = if rows.is_empty() {
            v_flex()
                .items_center()
                .justify_center()
                .py_12()
                .border_1()
                .border_color(border)
                .rounded_xl()
                .gap_4()
                .child(
                    div().rounded_full().bg(muted).p_4().child(
                        Icon::default()
                            .path("icons/shield.svg")
                            .size_8()
                            .text_color(muted_fg),
                    ),
                )
                .child(div().text_lg().font_semibold().child("No Fingerprints Enrolled"))
                .child(
                    div()
                        .text_sm()
                        .text_color(muted_fg)
                        .child("Enroll a fingerprint to use biometric verification on this key."),
                )
                .into_any_element()
        } else {
            v_flex().gap_4().children(rows).into_any_element()
        };

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
                                div()
                                    .text_sm()
                                    .text_color(muted_fg)
                                    .child(format!("{} fingerprint template(s) loaded", count)),
                            ),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                PFIconButton::new(
                                    Icon::default().path("icons/plus.svg").size_3p5(),
                                    "Enroll",
                                )
                                .small()
                                .loading(self.loading)
                                .on_click(enroll_listener),
                            )
                            .child(
                                PFIconButton::new(
                                    Icon::default().path("icons/lock.svg").size_3p5(),
                                    "Lock",
                                )
                                .small()
                                .disabled(self.loading)
                                .on_click(lock_listener),
                            ),
                    ),
            )
            .child(content)
    }

    fn render_template_card(
        &self,
        template: FingerprintTemplate,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let border = cx.theme().border;
        let muted_fg = cx.theme().muted_foreground;
        let rename_template = template.clone();
        let delete_template = template.clone();

        div()
            .w_full()
            .border_1()
            .border_color(border)
            .rounded_xl()
            .p_4()
            .child(
                v_flex()
                    .gap_4()
                    .child(
                        h_flex()
                            .justify_between()
                            .items_start()
                            .child(
                                v_flex()
                                    .gap_1()
                                    .child(
                                        div()
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .child(
                                                template
                                                    .friendly_name
                                                    .clone()
                                                    .unwrap_or_else(|| "Unnamed fingerprint".to_string()),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(muted_fg)
                                            .font_family("Mono")
                                            .child(template.template_id.clone()),
                                    ),
                            )
                            .child(
                                h_flex()
                                    .gap_2()
                                    .child(
                                        PFButton::new("Rename")
                                            .small()
                                            .disabled(self.loading)
                                            .with_colors(
                                                rgb(0x222225),
                                                rgb(0x2a2a2d),
                                                rgb(0x333336),
                                            )
                                            .on_click(cx.listener(move |this, _, window, cx| {
                                                this.open_rename_dialog(
                                                    rename_template.clone(),
                                                    window,
                                                    cx,
                                                );
                                            })),
                                    )
                                    .child(
                                        PFButton::new("Delete")
                                            .small()
                                            .disabled(self.loading)
                                            .with_colors(
                                                rgb(0x7f1d1d),
                                                rgb(0x991b1b),
                                                rgb(0xb91c1c),
                                            )
                                            .with_text_color(rgb(0xfef2f2))
                                            .on_click(cx.listener(move |this, _, window, cx| {
                                                this.open_delete_dialog(
                                                    delete_template.clone(),
                                                    window,
                                                    cx,
                                                );
                                            })),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(muted_fg)
                            .child("Rename or delete this template. Enrollment requires two captures on the sensor."),
                    ),
            )
    }

    fn render_secure_boot_card(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let destructive_red = rgb(0xef4444);
        let destructive_border = rgba(0xef44444d);
        let destructive_bg_muted = rgba(0xef44441a);

        div()
            .w_full()
            .border_1()
            .border_color(destructive_border)
            .bg(theme.secondary)
            .rounded_xl()
            .overflow_hidden()
            .child(
                div()
                    .p_6()
                    .child(div().text_lg().font_bold().child("Secure Boot")),
            )
            .child(
                v_flex()
                    .px_6()
                    .pb_6()
                    .gap_6()
                    .child(
                        div()
                            .p_4()
                            .border_1()
                            .border_color(destructive_border)
                            .rounded_md()
                            .bg(destructive_bg_muted)
                            .child(
                                v_flex()
                                    .gap_2()
                                    .child(
                                        h_flex()
                                            .gap_2()
                                            .items_center()
                                            .child(
                                                Icon::default()
                                                    .path("icons/triangle-alert.svg")
                                                    .text_color(destructive_red),
                                            )
                                            .child(
                                                div()
                                                    .font_bold()
                                                    .text_color(destructive_red)
                                                    .child("Feature Unstable"),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(destructive_red)
                                            .child("This feature is currently disabled for safety."),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child("Permanently lock this device to the current firmware vendor."),
                    ),
            )
    }
}

impl Render for SecurityView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let device = self
            .root
            .upgrade()
            .map(|r| r.read(cx).device.clone())
            .unwrap_or_else(DeviceConnectionState::new);

        if device.status.is_none() {
            let theme = cx.theme();
            return PageView::build(
                "Security",
                "Manage FIDO2 security controls and biometric enrollments.",
                self.render_no_device(theme).into_any_element(),
                theme,
            )
            .into_any_element();
        }

        let bio_supported = device
            .fido_info
            .as_ref()
            .and_then(|info| info.options.get("bioEnroll").copied())
            .unwrap_or(false);

        if !bio_supported {
            let theme = cx.theme();
            return PageView::build(
                "Security",
                "Manage FIDO2 security controls and biometric enrollments.",
                self.render_not_supported(theme).into_any_element(),
                theme,
            )
            .into_any_element();
        }

        let is_wide = window.bounds().size.width > px(1100.0);
        let columns = if is_wide { 2 } else { 1 };

        PageView::build(
            "Security",
            "Manage FIDO2 security controls and biometric enrollments.",
            div()
                .grid()
                .grid_cols(columns)
                .gap_6()
                .child(self.render_fingerprint_card(cx))
                .child(self.render_secure_boot_card(cx)),
            cx.theme(),
        )
        .into_any_element()
    }
}
