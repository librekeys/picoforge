use crate::device::io;
use crate::device::types::{TotpEntry, TotpStatus};
use crate::ui::components::{
    button::{PFButton, PFIconButton},
    card::Card,
    dialog,
    dialog::{ChangePinContent, ConfirmContent, PinPromptContent, SetPinContent, TextPromptContent},
    page_view::PageView,
    tag::Tag,
};
use crate::ui::rootview::ApplicationRoot;
use gpui::*;
use gpui_component::button::ButtonVariant;
use gpui_component::{ActiveTheme, Icon, StyledExt, Theme, h_flex, v_flex};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct TotpView {
    root: WeakEntity<ApplicationRoot>,
    status: Option<TotpStatus>,
    cached_password: Option<String>,
    loading: bool,
    last_error: Option<String>,
    _task: Option<Task<()>>,
}

impl TotpView {
    pub fn new(
        _window: &mut Window,
        _cx: &mut Context<Self>,
        root: WeakEntity<ApplicationRoot>,
    ) -> Self {
        Self {
            root,
            status: None,
            cached_password: None,
            loading: false,
            last_error: None,
            _task: None,
        }
    }

    pub fn refresh_status(&mut self, cx: &mut Context<Self>) {
        if self.loading {
            return;
        }
        self.loading = true;
        self.last_error = None;
        cx.notify();

        let entity = cx.entity().downgrade();
        let cached_password = self.cached_password.clone();
        self._task = Some(cx.spawn(async move |_, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { io::get_totp_status(cached_password) })
                .await;
            let _ = entity.update(cx, |this, cx| {
                this.loading = false;
                match result {
                    Ok(status) => {
                        this.status = Some(status);
                        this.last_error = None;
                    }
                    Err(err) => {
                        log::error!("Failed to load TOTP status: {}", err);
                        this.cached_password = None;
                        this.status = None;
                        this.last_error = Some(err);
                    }
                }
                cx.notify();
            });
        }));
    }

    fn open_import_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.loading {
            return;
        }
        let view_handle = cx.entity().downgrade();
        dialog::open_text_prompt(
            "Import TOTP Account",
            "Paste a standard otpauth://totp/... URI. PicoForge currently stores standard 30-second TOTP accounts.",
            "otpauth://totp/...",
            "Import",
            None,
            window,
            cx,
            move |uri, dialog_handle, cx| {
                let _ = view_handle.update(cx, |this, cx| {
                    this.import_totp(uri, dialog_handle, cx);
                });
            },
        );
    }

    fn import_totp(
        &mut self,
        uri: String,
        dialog_handle: WeakEntity<TextPromptContent>,
        cx: &mut Context<Self>,
    ) {
        if self.loading {
            return;
        }
        self.loading = true;
        cx.notify();

        let entity = cx.entity().downgrade();
        let password = self.cached_password.clone();
        self._task = Some(cx.spawn(async move |_, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { io::import_totp_uri(uri, password) })
                .await;

            let _ = entity.update(cx, |this, cx| {
                this.loading = false;
                match result {
                    Ok(message) => {
                        let _ = dialog_handle.update(cx, |dialog, cx| {
                            dialog.set_success(message, cx);
                        });
                        this.refresh_status(cx);
                    }
                    Err(err) => {
                        log::error!("Failed to import TOTP account: {}", err);
                        let _ = dialog_handle.update(cx, |dialog, cx| {
                            dialog.set_error(format!("Import failed: {}", err), cx);
                        });
                        cx.notify();
                    }
                }
            });
        }));
    }

    fn open_rename_dialog(&mut self, entry: TotpEntry, window: &mut Window, cx: &mut Context<Self>) {
        let view_handle = cx.entity().downgrade();
        dialog::open_text_prompt(
            "Rename TOTP Account",
            "Provide a new label for this TOTP account.",
            "New account name",
            "Rename",
            Some(entry.name.clone()),
            window,
            cx,
            move |new_name, dialog_handle, cx| {
                let _ = view_handle.update(cx, |this, cx| {
                    this.rename_totp(entry.name.clone(), new_name, dialog_handle, cx);
                });
            },
        );
    }

    fn rename_totp(
        &mut self,
        old_name: String,
        new_name: String,
        dialog_handle: WeakEntity<TextPromptContent>,
        cx: &mut Context<Self>,
    ) {
        if self.loading {
            return;
        }
        self.loading = true;
        cx.notify();

        let entity = cx.entity().downgrade();
        let password = self.cached_password.clone();
        self._task = Some(cx.spawn(async move |_, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { io::rename_totp(old_name, new_name, password) })
                .await;

            let _ = entity.update(cx, |this, cx| {
                this.loading = false;
                match result {
                    Ok(message) => {
                        let _ = dialog_handle.update(cx, |dialog, cx| {
                            dialog.set_success(message, cx);
                        });
                        this.refresh_status(cx);
                    }
                    Err(err) => {
                        log::error!("Failed to rename TOTP account: {}", err);
                        let _ = dialog_handle.update(cx, |dialog, cx| {
                            dialog.set_error(format!("Rename failed: {}", err), cx);
                        });
                        cx.notify();
                    }
                }
            });
        }));
    }

    fn open_delete_dialog(&mut self, entry: TotpEntry, window: &mut Window, cx: &mut Context<Self>) {
        let label = entry.name.clone();
        let view_handle = cx.entity().downgrade();
        dialog::open_confirm(
            "Delete TOTP Account",
            format!("Delete TOTP account {}?", label),
            "Delete",
            ButtonVariant::Danger,
            window,
            cx,
            move |dialog_handle, cx| {
                let _ = view_handle.update(cx, |this, cx| {
                    this.delete_totp(entry.name.clone(), dialog_handle, cx);
                });
            },
        );
    }

    fn delete_totp(
        &mut self,
        name: String,
        dialog_handle: WeakEntity<ConfirmContent>,
        cx: &mut Context<Self>,
    ) {
        if self.loading {
            return;
        }
        self.loading = true;
        cx.notify();

        let entity = cx.entity().downgrade();
        let password = self.cached_password.clone();
        self._task = Some(cx.spawn(async move |_, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { io::delete_totp(name, password) })
                .await;

            let _ = entity.update(cx, |this, cx| {
                this.loading = false;
                match result {
                    Ok(message) => {
                        let _ = dialog_handle.update(cx, |dialog, cx| {
                            dialog.set_success(message, cx);
                        });
                        this.refresh_status(cx);
                    }
                    Err(err) => {
                        log::error!("Failed to delete TOTP account: {}", err);
                        let _ = dialog_handle.update(cx, |dialog, cx| {
                            dialog.set_error(format!("Delete failed: {}", err), cx);
                        });
                        cx.notify();
                    }
                }
            });
        }));
    }

    fn lock_store(&mut self, cx: &mut Context<Self>) {
        self.cached_password = None;
        self.refresh_status(cx);
    }

    fn open_set_password_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let view_handle = cx.entity().downgrade();
        dialog::open_setup_pin(window, cx, move |new_password, dialog_handle, cx| {
            let _ = view_handle.update(cx, |this, cx| {
                this.set_store_password(new_password, dialog_handle, cx);
            });
        });
    }

    fn set_store_password(
        &mut self,
        new_password: String,
        dialog_handle: WeakEntity<SetPinContent>,
        cx: &mut Context<Self>,
    ) {
        if self.loading {
            return;
        }
        self.loading = true;
        cx.notify();

        let entity = cx.entity().downgrade();
        let new_password_for_bg = new_password.clone();
        self._task = Some(cx.spawn(async move |_, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { io::set_totp_password(None, new_password_for_bg) })
                .await;

            let _ = entity.update(cx, |this, cx| {
                this.loading = false;
                match result {
                    Ok(message) => {
                        this.cached_password = Some(new_password);
                        let _ = dialog_handle.update(cx, |dialog, cx| {
                            dialog.set_success(message, cx);
                        });
                        this.refresh_status(cx);
                    }
                    Err(err) => {
                        log::error!("Failed to set TOTP password: {}", err);
                        let _ = dialog_handle.update(cx, |dialog, cx| {
                            dialog.set_error(format!("Failed to set password: {}", err), cx);
                        });
                        cx.notify();
                    }
                }
            });
        }));
    }

    fn open_change_password_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let view_handle = cx.entity().downgrade();
        dialog::open_change_pin(window, cx, move |current_password, new_password, dialog_handle, cx| {
            let _ = view_handle.update(cx, |this, cx| {
                this.change_store_password(current_password, new_password, dialog_handle, cx);
            });
        });
    }

    fn change_store_password(
        &mut self,
        current_password: String,
        new_password: String,
        dialog_handle: WeakEntity<ChangePinContent>,
        cx: &mut Context<Self>,
    ) {
        if self.loading {
            return;
        }
        self.loading = true;
        cx.notify();

        let entity = cx.entity().downgrade();
        let new_password_for_bg = new_password.clone();
        self._task = Some(cx.spawn(async move |_, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    io::set_totp_password(Some(current_password), new_password_for_bg)
                })
                .await;

            let _ = entity.update(cx, |this, cx| {
                this.loading = false;
                match result {
                    Ok(message) => {
                        this.cached_password = Some(new_password);
                        let _ = dialog_handle.update(cx, |dialog, cx| {
                            dialog.set_success(message, cx);
                        });
                        this.refresh_status(cx);
                    }
                    Err(err) => {
                        log::error!("Failed to change TOTP password: {}", err);
                        let _ = dialog_handle.update(cx, |dialog, cx| {
                            dialog.set_error(format!("Failed to change password: {}", err), cx);
                        });
                        cx.notify();
                    }
                }
            });
        }));
    }

    fn open_unlock_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.loading {
            return;
        }

        let view_handle = cx.entity().downgrade();
        dialog::open_pin_prompt(
            "Unlock TOTP Store",
            "Enter the OATH password to unlock and calculate TOTP codes.",
            "Enter OATH password",
            "Unlock",
            window,
            cx,
            move |password, dialog_handle, cx| {
                let _ = view_handle.update(cx, |this, cx| {
                    this.unlock_store_with_password(password, dialog_handle, cx);
                });
            },
        );
    }

    fn unlock_store_with_password(
        &mut self,
        password: String,
        dialog_handle: WeakEntity<PinPromptContent>,
        cx: &mut Context<Self>,
    ) {
        if self.loading {
            return;
        }
        self.loading = true;
        self.last_error = None;
        cx.notify();

        let entity = cx.entity().downgrade();
        self._task = Some(cx.spawn(async move |_, cx| {
            let password_for_bg = password.clone();
            let result = cx
                .background_executor()
                .spawn(async move { io::get_totp_status(Some(password_for_bg)) })
                .await;

            let _ = entity.update(cx, |this, cx| {
                this.loading = false;
                match result {
                    Ok(status) => {
                        this.cached_password = Some(password);
                        this.status = Some(status);
                        this.last_error = None;
                        let _ = dialog_handle.update(cx, |dialog, cx| {
                            dialog.set_success("TOTP store unlocked.".to_string(), cx);
                        });
                    }
                    Err(err) => {
                        log::error!("Failed to unlock TOTP storage: {}", err);
                        this.cached_password = None;
                        let _ = dialog_handle.update(cx, |dialog, cx| {
                            dialog.set_error(format!("Failed to unlock: {}", err), cx);
                        });
                    }
                }
                cx.notify();
            });
        }));
    }

    fn render_no_device(&self, theme: &Theme) -> AnyElement {
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
                    .child("Connect your key to manage TOTP accounts."),
            )
            .into_any_element()
    }

    fn render_error(&self, theme: &Theme) -> AnyElement {
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
                    .child(self.last_error.clone().unwrap_or_else(|| "Unable to read TOTP storage.".into())),
            )
            .into_any_element()
    }

    fn seconds_remaining(&self) -> u64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        30 - (now % 30)
    }

    fn render_entry_row(&self, entry: TotpEntry, cx: &mut Context<Self>) -> AnyElement {
        let title = entry
            .account_name
            .clone()
            .unwrap_or_else(|| entry.name.clone());
        let subtitle = entry
            .issuer
            .clone()
            .unwrap_or_else(|| entry.algorithm.clone());
        let code = entry.current_code.clone().unwrap_or_else(|| {
            if entry.requires_touch {
                "Touch required".into()
            } else {
                "Unavailable".into()
            }
        });

        let rename_listener = {
            let entry = entry.clone();
            cx.listener(move |this, _, window, cx| {
                this.open_rename_dialog(entry.clone(), window, cx);
            })
        };
        let delete_listener = {
            let entry = entry.clone();
            cx.listener(move |this, _, window, cx| {
                this.open_delete_dialog(entry.clone(), window, cx);
            })
        };

        div()
            .w_full()
            .border_1()
            .border_color(cx.theme().border)
            .rounded_lg()
            .px_4()
            .py_3()
            .child(
                h_flex()
                    .justify_between()
                    .items_center()
                    .gap_4()
                    .child(
                        h_flex()
                            .items_center()
                            .gap_4()
                            .child(
                                v_flex()
                                    .gap_1()
                                    .child(div().font_semibold().child(title))
                                    .child(
                                        h_flex()
                                            .gap_2()
                                            .items_center()
                                            .child(div().text_sm().text_color(cx.theme().muted_foreground).child(subtitle))
                                            .child(Tag::new(format!("{} digits", entry.digits)))
                                            .children(if entry.requires_touch {
                                                Some(Tag::new("Touch").into_any_element())
                                            } else {
                                                None
                                            }),
                                    ),
                            )
                            .child(
                                div()
                                    .text_xl()
                                    .font_weight(FontWeight::EXTRA_BOLD)
                                    .child(code),
                            ),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(
                                PFButton::new("Rename")
                                    .id(format!("totp-rename-{}", entry.name))
                                    .small()
                                    .on_click(rename_listener),
                            )
                            .child(
                                PFButton::new("Delete")
                                    .id(format!("totp-delete-{}", entry.name))
                                    .small()
                                    .on_click(delete_listener),
                            ),
                    ),
            )
            .into_any_element()
    }
}

impl Render for TotpView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (border, muted, muted_foreground) = {
            let theme = cx.theme();
            (theme.border, theme.muted, theme.muted_foreground)
        };
        let no_device = self
            .root
            .upgrade()
            .and_then(|root| root.read(cx).device.status.clone())
            .is_none();

        let content = if no_device {
            self.render_no_device(cx.theme())
        } else if self.status.is_none() && self.last_error.is_some() {
            self.render_error(cx.theme())
        } else {
            let status = self.status.clone().unwrap_or(TotpStatus {
                supported: true,
                version: None,
                serial: None,
                protected: false,
                pin_retries: None,
                entries: Vec::new(),
            });

            let refresh_listener = cx.listener(|this, _, _, cx| {
                this.refresh_status(cx);
            });
            let import_listener = cx.listener(|this, _, window, cx| {
                this.open_import_dialog(window, cx);
            });
            let unlock_card_listener = cx.listener(|this, _, window, cx| {
                this.open_unlock_dialog(window, cx);
            });
            let unlock_header_listener = cx.listener(|this, _, window, cx| {
                this.open_unlock_dialog(window, cx);
            });
            let set_password_listener = cx.listener(|this, _, window, cx| {
                this.open_set_password_dialog(window, cx);
            });
            let change_password_listener = cx.listener(|this, _, window, cx| {
                this.open_change_password_dialog(window, cx);
            });
            let lock_listener = cx.listener(|this, _, _, cx| {
                this.lock_store(cx);
            });
            let unlocked = self.cached_password.is_some();
            let store_locked = status.protected && !unlocked;

            let mut rows = Vec::new();
            for entry in status.entries.clone() {
                rows.push(self.render_entry_row(entry, cx));
            }

            let body = if status.protected {
                v_flex()
                    .gap_4()
                    .child(
                        div()
                            .p_4()
                            .rounded_lg()
                            .border_1()
                            .border_color(border)
                            .bg(muted)
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(muted_foreground)
                                    .child("This OATH store is protected. Unlock it with the OATH password to view and manage stored codes."),
                            ),
                    )
                    .child(
                        PFButton::new("Unlock TOTP Store")
                            .id("totp-unlock")
                            .small()
                            .on_click(unlock_card_listener),
                    )
                    .into_any_element()
            } else if rows.is_empty() {
                v_flex()
                    .items_center()
                    .justify_center()
                    .gap_3()
                    .py_10()
                    .border_1()
                    .border_color(border)
                    .rounded_xl()
                    .child(div().text_lg().font_semibold().child("No TOTP Accounts Stored"))
                    .child(
                        div()
                            .text_sm()
                            .text_color(muted_foreground)
                            .child("Import an otpauth URI to store a TOTP secret on the key."),
                    )
                    .into_any_element()
            } else {
                v_flex().gap_3().children(rows).into_any_element()
            };

            Card::new()
                .title("Authenticator")
                .description("Store and calculate standard OATH/TOTP accounts on the key.")
                .icon(Icon::default().path("icons/asterisk.svg"))
                .child(
                    v_flex()
                        .gap_5()
                        .child(
                            h_flex()
                                .justify_between()
                                .items_center()
                                .gap_3()
                                .child(
                                    h_flex()
                                        .gap_2()
                                        .flex_wrap()
                                        .child(Tag::new(format!("{} account(s)", status.entries.len())))
                                        .children(status.version.clone().map(Tag::new).map(|t| t.into_any_element()))
                                        .children(status.pin_retries.map(|v| Tag::new(format!("PIN retries: {}", v)).into_any_element()))
                                        .children(if unlocked {
                                            Some(Tag::new("Unlocked").active(true).into_any_element())
                                        } else {
                                            None
                                        })
                                        .children(if status.protected {
                                            Some(Tag::new("Password Protected").into_any_element())
                                        } else {
                                            None
                                        })
                                        .child(Tag::new(format!("Refresh in {}s", self.seconds_remaining()))),
                                )
                                .child(
                                    h_flex()
                                        .gap_2()
                                        .children(if unlocked {
                                            Some(
                                                PFButton::new("Lock")
                                                    .id("totp-lock")
                                                    .small()
                                                    .on_click(lock_listener)
                                                    .into_any_element(),
                                            )
                                        } else if status.protected {
                                            Some(
                                                PFButton::new("Unlock")
                                                    .id("totp-unlock-header")
                                                    .small()
                                                    .on_click(unlock_header_listener)
                                                    .into_any_element(),
                                            )
                                        } else {
                                            None
                                        })
                                        .children(if status.protected {
                                            Some(
                                                PFButton::new("Change Password")
                                                    .id("totp-change-password")
                                                    .small()
                                                    .on_click(change_password_listener)
                                                    .into_any_element(),
                                            )
                                        } else {
                                            Some(
                                                PFButton::new("Set Password")
                                                    .id("totp-set-password")
                                                    .small()
                                                    .on_click(set_password_listener)
                                                    .into_any_element(),
                                            )
                                        })
                                        .child(
                                            PFButton::new("Refresh Codes")
                                                .id("totp-refresh")
                                                .small()
                                                .on_click(refresh_listener)
                                                .loading(self.loading),
                                        )
                                        .child(
                                            PFIconButton::new(Icon::default().path("icons/plus.svg"), "Import URI")
                                                .id("totp-import")
                                                .on_click(import_listener)
                                                .disabled(store_locked)
                                                .loading(self.loading),
                                        ),
                                ),
                        )
                        .child(
                            div()
                                .p_3()
                                .rounded_lg()
                                .border_1()
                                .border_color(border)
                                .bg(muted)
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(muted_foreground)
                                        .child("Imported TOTP accounts are stored in the key's OATH applet. This first slice supports standard 30-second TOTP otpauth URIs."),
                                ),
                        )
                        .child(body),
                )
                .into_any_element()
        };

        PageView::build(
            "TOTP",
            "Manage OATH/TOTP accounts stored on the key.",
            content,
            cx.theme(),
        )
    }
}
