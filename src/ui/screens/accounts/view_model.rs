//! View model for the Accounts (OATH) screen — TOTP/HOTP credential
//! management over the CCID OATH applet.

use crate::error::PFError;
use crate::ui::app::AppModels;
use crate::ui::components::applet_gate::AppletGate;
use crate::ui::components::dialog;
use crate::ui::components::dialog::{ConfirmContent, PinPromptContent};
use crate::ui::components::form::{LabeledU8, select_state, selected_key};
use crate::ui::models::device::{DeviceEvent, DeviceRepo, USB_CAP_OATH, oath};
use gpui::*;
use gpui_component::WindowExt;
use gpui_component::button::ButtonVariants;
use gpui_component::select::SelectState;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

// Add-form dropdown options (label, key). The key is the wire value where one
// exists (algorithm byte, period seconds, digit count) or a 0/1 flag otherwise.
const OPT_TYPE: &[(&str, u8)] = &[("Time-based (TOTP)", 0), ("Counter-based (HOTP)", 1)];
const OPT_ALGO: &[(&str, u8)] = &[("SHA-1", 1), ("SHA-256", 2), ("SHA-512", 3)];
const OPT_PERIOD: &[(&str, u8)] = &[
    ("20 seconds", 20),
    ("30 seconds", 30),
    ("45 seconds", 45),
    ("60 seconds", 60),
];
const OPT_DIGITS: &[(&str, u8)] = &[("6 digits", 6), ("8 digits", 8)];
const OPT_TOUCH: &[(&str, u8)] = &[("Not required", 0), ("Touch required", 1)];

pub fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Accounts screen state, code refresh, and OATH operations.
pub struct AccountsViewModel {
    pub(super) device: Entity<DeviceRepo>,
    pub(super) accounts: Vec<oath::Account>,
    pub(super) loaded: bool,
    pub(super) needs_password: bool,
    password: Option<String>,
    pub(super) loading: bool,
    /// Current unix time, ticked once a second to drive the countdown + refresh.
    pub(super) now: u64,
    last_window: u64,
    _task: Option<Task<()>>,
    _ticker: Option<Task<()>>,
}

/// UI-level notifications surfaced by the parent as window toasts.
pub enum AccountsEvent {
    Notification(String),
}

impl EventEmitter<AccountsEvent> for AccountsViewModel {}

impl AccountsViewModel {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>, models: &AppModels) -> Self {
        let device = models.device.clone();
        cx.subscribe(&device, |this: &mut Self, _, _: &DeviceEvent, cx| {
            this.on_device_event(cx);
        })
        .detach();

        let mut this = Self {
            device,
            accounts: Vec::new(),
            loaded: false,
            needs_password: false,
            password: None,
            loading: false,
            now: now_unix(),
            last_window: 0,
            _task: None,
            _ticker: None,
        };
        this.start_ticker(cx);
        this.try_initial_load(cx);
        this
    }

    /// A device (un)plug: re-lock and try to (re)load for the new device.
    fn on_device_event(&mut self, cx: &mut Context<Self>) {
        if self.device.read(cx).device_changed {
            self.accounts.clear();
            self.loaded = false;
            self.needs_password = false;
            self.password = None;
        }
        self.try_initial_load(cx);
        cx.notify();
    }

    fn start_ticker(&mut self, cx: &mut Context<Self>) {
        self._ticker = Some(cx.spawn(async move |weak, cx| {
            loop {
                cx.background_executor().timer(Duration::from_secs(1)).await;
                let alive = weak.update(cx, |this, cx| {
                    this.now = now_unix();
                    if this.loaded && !this.loading && this.now / 30 != this.last_window {
                        this.reload(cx);
                    }
                    cx.notify();
                });
                if alive.is_err() {
                    break;
                }
            }
        }));
    }

    /// The gate deciding whether the screen can show accounts.
    pub(super) fn gate(&self, cx: &App) -> AppletGate {
        let repo = self.device.read(cx);
        if repo.status.is_none() {
            return AppletGate::Unsupported;
        }
        match repo.oath_features() {
            None => AppletGate::Unsupported,
            Some(_) if !repo.ccid_on() => AppletGate::CcidOff,
            Some(_) if !repo.applet_enabled(USB_CAP_OATH) => {
                AppletGate::Disabled(crate::tr!("OATH"))
            }
            Some(_) => AppletGate::Ready,
        }
    }

    fn try_initial_load(&mut self, cx: &mut Context<Self>) {
        if self.loaded || self.loading || self.gate(cx) != AppletGate::Ready {
            return;
        }
        self.loading = true;
        cx.notify();
        let weak = cx.entity().downgrade();
        self._task = Some(cx.spawn(async move |_, cx| {
            let required = cx
                .background_executor()
                .spawn(async { DeviceRepo::oath_password_required_blocking() })
                .await;
            match required {
                Ok(true) => {
                    let _ = weak.update(cx, |this, cx| {
                        this.needs_password = true;
                        this.loading = false;
                        cx.notify();
                    });
                }
                Ok(false) => {
                    let list = cx
                        .background_executor()
                        .spawn(async { DeviceRepo::oath_list_accounts_blocking(None) })
                        .await;
                    let _ = weak.update(cx, |this, cx| this.apply_load(list, None, cx));
                }
                Err(e) => {
                    let _ = weak.update(cx, |this, cx| {
                        this.loading = false;
                        log::warn!("{}", crate::tr!("OATH probe failed: {e}", e = e));
                        cx.notify();
                    });
                }
            }
        }));
    }

    fn apply_load(
        &mut self,
        result: Result<Vec<oath::Account>, PFError>,
        password: Option<String>,
        cx: &mut Context<Self>,
    ) {
        self.loading = false;
        match result {
            Ok(accounts) => {
                self.accounts = accounts;
                self.loaded = true;
                self.needs_password = false;
                self.last_window = self.now / 30;
                if password.is_some() {
                    self.password = password;
                }
            }
            Err(e) => {
                log::warn!("{}", crate::tr!("OATH load failed: {e}", e = e));
                cx.emit(AccountsEvent::Notification(crate::tr!(
                    "Accounts: {e}",
                    e = e
                )));
            }
        }
        cx.notify();
    }

    /// Re-fetch codes with the cached password (used by the ticker + after ops).
    fn reload(&mut self, cx: &mut Context<Self>) {
        if self.loading {
            return;
        }
        self.loading = true;
        self.last_window = self.now / 30;
        cx.notify();
        let pw = self.password.clone();
        let weak = cx.entity().downgrade();
        self._task = Some(cx.spawn(async move |_, cx| {
            let list = cx
                .background_executor()
                .spawn(async move { DeviceRepo::oath_list_accounts_blocking(pw) })
                .await;
            let _ = weak.update(cx, |this, cx| this.apply_load(list, None, cx));
        }));
    }

    pub(super) fn open_unlock_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let view = cx.entity().downgrade();
        dialog::open_pin_prompt(
            crate::tr!("Unlock Accounts"),
            crate::tr!("Enter the OATH password for this device."),
            crate::tr!("OATH password"),
            None,
            crate::tr!("Unlock"),
            window,
            cx,
            move |password, dialog_handle, cx| {
                let _ = view.update(cx, |this, cx| this.unlock(password, dialog_handle, cx));
            },
        );
    }

    fn unlock(
        &mut self,
        password: String,
        dh: WeakEntity<PinPromptContent>,
        cx: &mut Context<Self>,
    ) {
        if self.loading {
            return;
        }
        self.loading = true;
        cx.notify();
        let weak = cx.entity().downgrade();
        self._task = Some(cx.spawn(async move |_, cx| {
            let pw = password.clone();
            let list = cx
                .background_executor()
                .spawn(async move { DeviceRepo::oath_list_accounts_blocking(Some(pw)) })
                .await;
            let _ = weak.update(cx, |this, cx| {
                match &list {
                    Ok(_) => {
                        let _ = dh.update(cx, |d, cx| {
                            d.set_success(crate::tr!("Unlocked.").into(), cx)
                        });
                    }
                    Err(e) => {
                        let _ = dh.update(cx, |d, cx| d.set_error(crate::tr!("{e}", e = e), cx));
                    }
                }
                this.apply_load(list, Some(password), cx);
            });
        }));
    }

    pub(super) fn refresh(&mut self, cx: &mut Context<Self>) {
        self.reload(cx);
    }

    /// Whether the applet currently has an access code set.
    pub(super) fn password_is_set(&self) -> bool {
        self.needs_password || self.password.is_some()
    }

    pub(super) fn open_password_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let current = self.password.clone();
        let view = cx.entity().downgrade();
        dialog::open_pin_prompt(
            crate::tr!("OATH Password"),
            crate::tr!("Enter a new OATH password, or leave empty to remove password protection."),
            crate::tr!("New OATH password"),
            None,
            crate::tr!("Save"),
            window,
            cx,
            move |new_password, dialog_handle, cx| {
                let _ = view.update(cx, |this, cx| {
                    this.set_password(current.clone(), new_password, dialog_handle, cx)
                });
            },
        );
    }

    fn set_password(
        &mut self,
        current: Option<String>,
        new_password: String,
        dh: WeakEntity<PinPromptContent>,
        cx: &mut Context<Self>,
    ) {
        if self.loading {
            return;
        }
        self.loading = true;
        cx.notify();
        let new = (!new_password.trim().is_empty()).then(|| new_password.trim().to_string());
        let new_cache = new.clone();
        let weak = cx.entity().downgrade();
        self._task = Some(cx.spawn(async move |_, cx| {
            let res = cx
                .background_executor()
                .spawn(async move { DeviceRepo::oath_set_password_blocking(current, new) })
                .await;
            let _ = weak.update(cx, |this, cx| {
                this.loading = false;
                match res {
                    Ok(_) => {
                        this.password = new_cache;
                        let msg = if this.password.is_some() {
                            crate::tr!("Password saved.")
                        } else {
                            crate::tr!("Password removed.")
                        };
                        let _ = dh.update(cx, |d, cx| d.set_success(msg.into(), cx));
                        this.reload(cx);
                    }
                    Err(e) => {
                        let _ = dh.update(cx, |d, cx| d.set_error(crate::tr!("{e}", e = e), cx));
                    }
                }
                cx.notify();
            });
        }));
    }

    pub(super) fn copy_code(&self, code: String, cx: &mut Context<Self>) {
        cx.write_to_clipboard(ClipboardItem::new_string(code));
        cx.emit(AccountsEvent::Notification(
            crate::tr!("Code copied").into(),
        ));
    }

    /// Compute a single credential's code on demand (HOTP or touch-gated).
    pub(super) fn calculate(&mut self, id: String, period: u32, cx: &mut Context<Self>) {
        if self.loading {
            return;
        }
        self.loading = true;
        cx.notify();
        let pw = self.password.clone();
        let weak = cx.entity().downgrade();
        self._task = Some(cx.spawn(async move |_, cx| {
            let id_bg = id.clone();
            let res = cx
                .background_executor()
                .spawn(async move { DeviceRepo::oath_calculate_blocking(pw, id_bg, period) })
                .await;
            let _ = weak.update(cx, |this, cx| {
                this.loading = false;
                match res {
                    Ok(code) => {
                        if let Some(acc) = this.accounts.iter_mut().find(|a| a.id == id) {
                            acc.state = oath::CodeState::Code {
                                value: code,
                                period,
                            };
                        }
                    }
                    Err(e) => cx.emit(AccountsEvent::Notification(crate::tr!(
                        "Calculate: {e}",
                        e = e
                    ))),
                }
                cx.notify();
            });
        }));
    }

    pub(super) fn open_add_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let issuer = cx.new(|cx| {
            gpui_component::input::InputState::new(window, cx)
                .placeholder(crate::tr!("Issuer (e.g. GitHub)"))
        });
        let account = cx.new(|cx| {
            gpui_component::input::InputState::new(window, cx)
                .placeholder(crate::tr!("Account (e.g. you@example.com)"))
        });
        let secret = cx.new(|cx| {
            gpui_component::input::InputState::new(window, cx)
                .placeholder(crate::tr!("Base32 secret, or paste an otpauth:// URI"))
        });
        let type_sel = select_state(window, cx, OPT_TYPE, 0);
        let algo_sel = select_state(window, cx, OPT_ALGO, 0);
        let period_sel = select_state(window, cx, OPT_PERIOD, 1); // default 30 s
        let digits_sel = select_state(window, cx, OPT_DIGITS, 0);
        let touch_sel = select_state(window, cx, OPT_TOUCH, 0);

        let view = cx.entity().downgrade();
        let submit = {
            let issuer = issuer.clone();
            let account = account.clone();
            let secret = secret.clone();
            let type_sel = type_sel.clone();
            let algo_sel = algo_sel.clone();
            let period_sel = period_sel.clone();
            let digits_sel = digits_sel.clone();
            let touch_sel = touch_sel.clone();
            let view = view.clone();
            std::rc::Rc::new(move |window: &mut Window, cx: &mut App| {
                let secret_v = secret.read(cx).text().to_string().trim().to_string();
                if secret_v.is_empty() {
                    return;
                }
                let parsed = if secret_v.starts_with(crate::tr!("otpauth://")) {
                    // A pasted URI carries every field itself; the form is ignored.
                    oath::parse_otpauth(&secret_v)
                } else {
                    let account_v = account.read(cx).text().to_string().trim().to_string();
                    if account_v.is_empty() {
                        Err(crate::tr!("Enter an account name").to_string())
                    } else {
                        match oath::base32_decode(&secret_v).filter(|s| !s.is_empty()) {
                            None => Err(crate::tr!("Invalid base32 secret").to_string()),
                            Some(bytes) => {
                                let issuer_v =
                                    issuer.read(cx).text().to_string().trim().to_string();
                                let oath_type = if selected_key(&type_sel, OPT_TYPE, cx) == 1 {
                                    oath::OathType::Hotp
                                } else {
                                    oath::OathType::Totp
                                };
                                let algorithm = match selected_key(&algo_sel, OPT_ALGO, cx) {
                                    2 => oath::HashAlgo::Sha256,
                                    3 => oath::HashAlgo::Sha512,
                                    _ => oath::HashAlgo::Sha1,
                                };
                                Ok(oath::NewCredential {
                                    issuer: (!issuer_v.is_empty()).then_some(issuer_v),
                                    account: account_v,
                                    secret: bytes,
                                    oath_type,
                                    algorithm,
                                    digits: selected_key(&digits_sel, OPT_DIGITS, cx),
                                    period: selected_key(&period_sel, OPT_PERIOD, cx) as u32,
                                    counter: 0,
                                    touch: selected_key(&touch_sel, OPT_TOUCH, cx) == 1,
                                })
                            }
                        }
                    }
                };
                match parsed {
                    Ok(cred) => {
                        window.close_dialog(cx);
                        let status =
                            dialog::open_status_dialog(crate::tr!("Adding Account"), window, cx);
                        let _ = view.update(cx, |this, cx| this.execute_add(cred, status, cx));
                    }
                    Err(e) => {
                        let _ = view.update(cx, |_, cx| {
                            cx.emit(AccountsEvent::Notification(e));
                        });
                    }
                }
            })
        };

        window.open_dialog(cx, move |dialog, _window, _| {
            let issuer = issuer.clone();
            let account = account.clone();
            let secret = secret.clone();
            let type_sel = type_sel.clone();
            let algo_sel = algo_sel.clone();
            let period_sel = period_sel.clone();
            let digits_sel = digits_sel.clone();
            let touch_sel = touch_sel.clone();
            let submit_ok = submit.clone();
            let submit_btn = submit.clone();
            let field = |label: &str, sel: &Entity<SelectState<Vec<LabeledU8>>>| {
                gpui_component::v_flex()
                    .gap_1()
                    .flex_1()
                    .child(label.to_string())
                    .child(
                        gpui_component::select::Select::new(sel)
                            .w_full()
                            .bg(rgb(0x222225)),
                    )
            };
            dialog
                .title(crate::tr!("Add Account"))
                .child(crate::tr!(
                    "Fill in the details, or paste an otpauth:// URI into the secret field."
                ))
                .child(
                    gpui_component::v_flex()
                        .gap_3()
                        .pb_2()
                        .child(crate::tr!("Issuer"))
                        .child(gpui_component::input::Input::new(&issuer))
                        .child(crate::tr!("Account"))
                        .child(gpui_component::input::Input::new(&account))
                        .child(crate::tr!("Secret"))
                        .child(gpui_component::input::Input::new(&secret))
                        .child(
                            gpui_component::h_flex()
                                .gap_3()
                                .child(field(crate::tr!("Account type"), &type_sel))
                                .child(field(crate::tr!("Algorithm"), &algo_sel)),
                        )
                        .child(
                            gpui_component::h_flex()
                                .gap_3()
                                .child(field(crate::tr!("Period (time-based)"), &period_sel))
                                .child(field(crate::tr!("Digits"), &digits_sel)),
                        )
                        .child(field(crate::tr!("Touch"), &touch_sel)),
                )
                .on_ok(move |_, window, cx| {
                    submit_ok(window, cx);
                    false
                })
                .footer(move |_, _window, _cx, _| {
                    let submit = submit_btn.clone();
                    vec![
                        gpui_component::button::Button::new(crate::tr!("cancel"))
                            .label(crate::tr!("Cancel"))
                            .on_click(|_, window, cx| window.close_dialog(cx)),
                        gpui_component::button::Button::new("add")
                            .primary()
                            .label("Add")
                            .on_click(move |_, window, cx| submit(window, cx)),
                    ]
                })
        });
    }

    fn execute_add(
        &mut self,
        cred: oath::NewCredential,
        status: WeakEntity<dialog::StatusContent>,
        cx: &mut Context<Self>,
    ) {
        self.loading = true;
        cx.notify();
        let pw = self.password.clone();
        let weak = cx.entity().downgrade();
        self._task = Some(cx.spawn(async move |_, cx| {
            let res = cx
                .background_executor()
                .spawn(async move { DeviceRepo::oath_add_blocking(pw, cred) })
                .await;
            let _ = weak.update(cx, |this, cx| {
                this.loading = false;
                match res {
                    Ok(_) => {
                        let _ = status.update(cx, |d, cx| {
                            d.set_success(crate::tr!("Account added.").into(), cx)
                        });
                        this.reload(cx);
                    }
                    Err(e) => {
                        let _ =
                            status.update(cx, |d, cx| d.set_error(crate::tr!("{e}", e = e), cx));
                    }
                }
                cx.notify();
            });
        }));
    }

    pub(super) fn open_rename_dialog(
        &mut self,
        acc: &oath::Account,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let old_id = acc.id.clone();
        let oath_type = acc.oath_type;
        let period = acc.period;
        let issuer = cx.new(|cx| {
            gpui_component::input::InputState::new(window, cx)
                .placeholder(crate::tr!("Issuer"))
                .default_value(acc.issuer.clone().unwrap_or_default())
        });
        let account = cx.new(|cx| {
            gpui_component::input::InputState::new(window, cx)
                .placeholder(crate::tr!("Account"))
                .default_value(acc.account.clone())
        });

        let view = cx.entity().downgrade();
        let submit = {
            let issuer = issuer.clone();
            let account = account.clone();
            let view = view.clone();
            std::rc::Rc::new(move |window: &mut Window, cx: &mut App| {
                let new_issuer = issuer.read(cx).text().to_string();
                let new_account = account.read(cx).text().to_string();
                let acct = new_account.trim();
                if acct.is_empty() {
                    return;
                }
                let iss = new_issuer.trim();
                let new_id =
                    oath::build_cred_id((!iss.is_empty()).then_some(iss), acct, oath_type, period);
                window.close_dialog(cx);
                if new_id == old_id {
                    return;
                }
                let status = dialog::open_status_dialog(crate::tr!("Renaming Account"), window, cx);
                let old = old_id.clone();
                let _ = view.update(cx, |this, cx| this.execute_rename(old, new_id, status, cx));
            })
        };

        window.open_dialog(cx, move |dialog, _window, _| {
            let issuer = issuer.clone();
            let account = account.clone();
            let submit_ok = submit.clone();
            let submit_btn = submit.clone();
            dialog
                .title(crate::tr!("Rename Account"))
                .child(crate::tr!("Change the issuer and account name."))
                .child(
                    gpui_component::v_flex()
                        .gap_3()
                        .pb_2()
                        .child(crate::tr!("Issuer"))
                        .child(gpui_component::input::Input::new(&issuer))
                        .child(crate::tr!("Account"))
                        .child(gpui_component::input::Input::new(&account)),
                )
                .on_ok(move |_, window, cx| {
                    submit_ok(window, cx);
                    false
                })
                .footer(move |_, _window, _cx, _| {
                    let submit = submit_btn.clone();
                    vec![
                        gpui_component::button::Button::new(crate::tr!("cancel"))
                            .label(crate::tr!("Cancel"))
                            .on_click(|_, window, cx| window.close_dialog(cx)),
                        gpui_component::button::Button::new(crate::tr!("rename"))
                            .primary()
                            .label(crate::tr!("Rename"))
                            .on_click(move |_, window, cx| submit(window, cx)),
                    ]
                })
        });
    }

    fn execute_rename(
        &mut self,
        old_id: String,
        new_id: String,
        status: WeakEntity<dialog::StatusContent>,
        cx: &mut Context<Self>,
    ) {
        self.loading = true;
        cx.notify();
        let pw = self.password.clone();
        let weak = cx.entity().downgrade();
        self._task = Some(cx.spawn(async move |_, cx| {
            let res = cx
                .background_executor()
                .spawn(async move { DeviceRepo::oath_rename_blocking(pw, old_id, new_id) })
                .await;
            let _ = weak.update(cx, |this, cx| {
                this.loading = false;
                match res {
                    Ok(_) => {
                        let _ = status.update(cx, |d, cx| {
                            d.set_success(crate::tr!("Account renamed.").into(), cx)
                        });
                        this.reload(cx);
                    }
                    Err(e) => {
                        let _ =
                            status.update(cx, |d, cx| d.set_error(crate::tr!("{e}", e = e), cx));
                    }
                }
                cx.notify();
            });
        }));
    }

    pub(super) fn open_delete_dialog(
        &mut self,
        acc: &oath::Account,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let id = acc.id.clone();
        let label = acc.issuer.clone().unwrap_or_else(|| acc.account.clone());
        let view = cx.entity().downgrade();
        dialog::open_confirm(
            crate::tr!("Delete Account"),
            crate::tr!(
                "Delete the account \"{label}\"? This cannot be undone.",
                label = label
            ),
            crate::tr!("Delete"),
            gpui_component::button::ButtonVariant::Danger,
            window,
            cx,
            move |dialog_handle, _, cx| {
                let _ = view.update(cx, |this, cx| {
                    this.execute_delete(id.clone(), dialog_handle, cx)
                });
            },
        );
    }

    fn execute_delete(
        &mut self,
        id: String,
        dh: WeakEntity<ConfirmContent>,
        cx: &mut Context<Self>,
    ) {
        self.loading = true;
        cx.notify();
        let pw = self.password.clone();
        let weak = cx.entity().downgrade();
        self._task = Some(cx.spawn(async move |_, cx| {
            let res = cx
                .background_executor()
                .spawn(async move { DeviceRepo::oath_delete_blocking(pw, id) })
                .await;
            let _ = weak.update(cx, |this, cx| {
                this.loading = false;
                match res {
                    Ok(_) => {
                        let _ = dh.update(cx, |d, cx| {
                            d.set_success(crate::tr!("Account deleted.").into(), cx)
                        });
                        this.reload(cx);
                    }
                    Err(e) => {
                        let _ = dh.update(cx, |d, cx| d.set_error(crate::tr!("{e}", e = e), cx));
                    }
                }
                cx.notify();
            });
        }));
    }

    pub(super) fn open_reset_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let view = cx.entity().downgrade();
        dialog::open_confirm(
            crate::tr!("Reset OATH Applet"),
            crate::tr!("This permanently deletes ALL accounts and the OATH password. This cannot be undone.")
                .to_string(),
            crate::tr!("Reset"),
            gpui_component::button::ButtonVariant::Danger,
            window,
            cx,
            move |_dialog_handle, window, cx| {
                window.close_dialog(cx);
                let _ = view.update(cx, |this, cx| this.execute_reset(window, cx));
            },
        );
    }

    fn execute_reset(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.loading {
            return;
        }
        self.loading = true;
        cx.notify();
        let status = dialog::open_status_dialog(crate::tr!("Resetting OATH Applet"), window, cx);
        let weak = cx.entity().downgrade();
        self._task = Some(cx.spawn(async move |_, cx| {
            let res = cx
                .background_executor()
                .spawn(async { DeviceRepo::oath_reset_blocking() })
                .await;
            let _ = weak.update(cx, |this, cx| {
                this.loading = false;
                match res {
                    Ok(_) => {
                        let _ = status.update(cx, |d, cx| {
                            d.set_success(crate::tr!("OATH applet reset.").into(), cx)
                        });
                        this.accounts.clear();
                        this.loaded = false;
                        this.password = None;
                        this.try_initial_load(cx);
                    }
                    Err(e) => {
                        let _ =
                            status.update(cx, |d, cx| d.set_error(crate::tr!("{e}", e = e), cx));
                    }
                }
                cx.notify();
            });
        }));
    }
}
