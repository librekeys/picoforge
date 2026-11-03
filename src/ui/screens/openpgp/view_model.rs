//! View model for the OpenPGP screen — card status, PIN management, per-key
//! touch policy, on-device key generation, cardholder editing, and reset.

use crate::error::PFError;
use crate::ui::app::AppModels;
use crate::ui::components::applet_gate::AppletGate;
use crate::ui::components::dialog;
use crate::ui::components::dialog::StatusContent;
use crate::ui::components::form::{select_state, selected_key};
use crate::ui::models::device::{DeviceEvent, DeviceRepo, USB_CAP_OPENPGP, openpgp};
use gpui::*;
use gpui_component::WindowExt;
use gpui_component::button::ButtonVariants;
use openpgp::PgpSlot;

const OPT_TOUCH: &[(&str, u8)] = &[("Off", 0), ("On", 1)];
/// OpenPGP sex (DO 5F35): the byte is an ISO-5218-style ASCII digit.
const OPT_SEX: &[(&str, u8)] = &[("Not announced", 0x39), ("Male", 0x31), ("Female", 0x32)];

pub struct OpenPgpViewModel {
    pub(super) device: Entity<DeviceRepo>,
    pub(super) info: Option<openpgp::PgpInfo>,
    pub(super) loaded: bool,
    pub(super) loading: bool,
    _task: Option<Task<()>>,
}

pub enum OpenPgpEvent {
    Notification(String),
}

impl EventEmitter<OpenPgpEvent> for OpenPgpViewModel {}

impl OpenPgpViewModel {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>, models: &AppModels) -> Self {
        let device = models.device.clone();
        cx.subscribe(&device, |this: &mut Self, _, _: &DeviceEvent, cx| {
            this.on_device_event(cx);
        })
        .detach();
        let mut this = Self {
            device,
            info: None,
            loaded: false,
            loading: false,
            _task: None,
        };
        this.load(cx);
        this
    }

    fn on_device_event(&mut self, cx: &mut Context<Self>) {
        if self.device.read(cx).device_changed {
            self.info = None;
            self.loaded = false;
        }
        self.load(cx);
        cx.notify();
    }

    pub(super) fn gate(&self, cx: &App) -> AppletGate {
        let repo = self.device.read(cx);
        if repo.status.is_none() {
            return AppletGate::Unsupported;
        }
        match repo.openpgp_features() {
            None => AppletGate::Unsupported,
            Some(_) if !repo.ccid_on() => AppletGate::CcidOff,
            Some(_) if !repo.applet_enabled(USB_CAP_OPENPGP) => {
                AppletGate::Disabled(crate::tr!("OpenPGP"))
            }
            Some(_) => AppletGate::Ready,
        }
    }

    /// Whether the firmware advertises elliptic-curve keys (else RSA-only).
    fn ecc(&self, cx: &App) -> bool {
        self.device
            .read(cx)
            .openpgp_features()
            .map(|f| f.ecc)
            .unwrap_or(false)
    }

    fn load(&mut self, cx: &mut Context<Self>) {
        if self.loading || self.gate(cx) != AppletGate::Ready {
            return;
        }
        self.loading = true;
        cx.notify();
        let weak = cx.entity().downgrade();
        self._task = Some(cx.spawn(async move |_, cx| {
            let res = cx
                .background_executor()
                .spawn(async { DeviceRepo::openpgp_read_info_blocking() })
                .await;
            let _ = weak.update(cx, |this, cx| {
                this.loading = false;
                match res {
                    Ok(info) => {
                        this.info = Some(info);
                        this.loaded = true;
                    }
                    Err(e) => {
                        log::warn!("{}", crate::tr!("OpenPGP read failed: {e}", e = e));
                        cx.emit(OpenPgpEvent::Notification(crate::tr!(
                            "OpenPGP: {e}",
                            e = e
                        )));
                    }
                }
                cx.notify();
            });
        }));
    }

    pub(super) fn refresh(&mut self, cx: &mut Context<Self>) {
        self.load(cx);
    }

    /// Run a blocking op, report on `status`, and reload on success.
    fn run(
        &mut self,
        op: impl FnOnce() -> Result<(), PFError> + Send + 'static,
        ok_msg: &'static str,
        status: WeakEntity<StatusContent>,
        cx: &mut Context<Self>,
    ) {
        if self.loading {
            return;
        }
        self.loading = true;
        cx.notify();
        let weak = cx.entity().downgrade();
        self._task = Some(cx.spawn(async move |_, cx| {
            let res = cx.background_executor().spawn(async move { op() }).await;
            let _ = weak.update(cx, |this, cx| {
                this.loading = false;
                match res {
                    Ok(_) => {
                        let _ = status.update(cx, |d, cx| d.set_success(ok_msg.into(), cx));
                        this.load(cx);
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

    // ── PIN management ──────────────────────────────────────────────────────

    pub(super) fn open_change_user_pin(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.two_secret_dialog(
            crate::tr!("Change User PIN"),
            crate::tr!("Current PIN (PW1)"),
            crate::tr!("New PIN"),
            None,
            window,
            cx,
            DeviceRepo::openpgp_change_user_pin_blocking,
            crate::tr!("User PIN changed."),
        );
    }

    pub(super) fn open_change_admin_pin(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.two_secret_dialog(
            crate::tr!("Change Admin PIN"),
            crate::tr!("Current admin PIN (PW3)"),
            crate::tr!("New admin PIN"),
            None,
            window,
            cx,
            DeviceRepo::openpgp_change_admin_pin_blocking,
            crate::tr!("Admin PIN changed."),
        );
    }

    pub(super) fn open_unblock_with_code(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.two_secret_dialog(
            crate::tr!("Unblock with Reset Code"),
            crate::tr!("Reset code"),
            crate::tr!("New user PIN"),
            None,
            window,
            cx,
            DeviceRepo::openpgp_unblock_with_code_blocking,
            crate::tr!("User PIN unblocked."),
        );
    }

    pub(super) fn open_unblock_with_admin(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.two_secret_dialog(
            crate::tr!("Unblock with Admin PIN"),
            crate::tr!("Admin PIN (PW3)"),
            crate::tr!("New user PIN"),
            Some(openpgp::DEFAULT_PW3),
            window,
            cx,
            DeviceRepo::openpgp_unblock_with_admin_blocking,
            crate::tr!("User PIN unblocked."),
        );
    }

    pub(super) fn open_set_reset_code(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.two_secret_dialog(
            crate::tr!("Set Reset Code"),
            crate::tr!("Admin PIN (PW3)"),
            crate::tr!("New reset code"),
            Some(openpgp::DEFAULT_PW3),
            window,
            cx,
            DeviceRepo::openpgp_set_reset_code_blocking,
            crate::tr!("Reset code updated."),
        );
    }

    /// A two-masked-field dialog running a blocking `op(a, b)`. `prefill_a`
    /// seeds the first field (e.g. the default admin PIN) for convenience.
    #[allow(clippy::too_many_arguments)]
    fn two_secret_dialog(
        &mut self,
        title: &'static str,
        label_a: &'static str,
        label_b: &'static str,
        prefill_a: Option<&'static str>,
        window: &mut Window,
        cx: &mut Context<Self>,
        op: impl Fn(String, String) -> Result<(), PFError> + Send + Clone + 'static,
        ok_msg: &'static str,
    ) {
        let a = cx.new(|cx| {
            let st = gpui_component::input::InputState::new(window, cx).masked(true);
            match prefill_a {
                Some(v) => st.default_value(v),
                None => st,
            }
        });
        let b = cx.new(|cx| gpui_component::input::InputState::new(window, cx).masked(true));
        let view = cx.entity().downgrade();
        let submit = {
            let a = a.clone();
            let b = b.clone();
            let view = view.clone();
            std::rc::Rc::new(move |window: &mut Window, cx: &mut App| {
                let av = a.read(cx).text().to_string();
                let bv = b.read(cx).text().to_string();
                if av.is_empty() || bv.is_empty() {
                    return;
                }
                window.close_dialog(cx);
                let status = dialog::open_status_dialog(title, window, cx);
                let op = op.clone();
                let _ = view.update(cx, |this, cx| {
                    this.run(move || op(av, bv), ok_msg, status, cx);
                });
            })
        };
        window.open_dialog(cx, move |dialog, _w, _| {
            let a = a.clone();
            let b = b.clone();
            let ok = submit.clone();
            let btn = submit.clone();
            dialog
                .title(title)
                .child(
                    gpui_component::v_flex()
                        .gap_3()
                        .pb_2()
                        .child(label_a)
                        .child(gpui_component::input::Input::new(&a))
                        .child(label_b)
                        .child(gpui_component::input::Input::new(&b)),
                )
                .on_ok(move |_, window, cx| {
                    ok(window, cx);
                    false
                })
                .footer(move |_, _w, _c, _| {
                    let s = btn.clone();
                    vec![
                        gpui_component::button::Button::new(crate::tr!("cancel"))
                            .label(crate::tr!("Cancel"))
                            .on_click(|_, window, cx| window.close_dialog(cx)),
                        gpui_component::button::Button::new(crate::tr!("ok"))
                            .primary()
                            .label(crate::tr!("Save"))
                            .on_click(move |_, window, cx| s(window, cx)),
                    ]
                })
        });
    }

    // ── Generate ────────────────────────────────────────────────────────────

    pub(super) fn open_generate(
        &mut self,
        slot: PgpSlot,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let algos: &'static [(&str, u8)] = if self.ecc(cx) {
            openpgp::GENERATE_ALGOS
        } else {
            &openpgp::GENERATE_ALGOS[..3]
        };
        let algo_sel = select_state(window, cx, algos, 0);
        let admin = admin_input(window, cx);
        let view = cx.entity().downgrade();
        let submit = {
            let algo_sel = algo_sel.clone();
            let admin = admin.clone();
            let view = view.clone();
            std::rc::Rc::new(move |window: &mut Window, cx: &mut App| {
                let admin_pin = admin.read(cx).text().to_string();
                if admin_pin.is_empty() {
                    return;
                }
                let choice = selected_key(&algo_sel, algos, cx);
                window.close_dialog(cx);
                let status = dialog::open_status_dialog(crate::tr!("Generating Key"), window, cx);
                let _ = view.update(cx, |this, cx| {
                    this.run(
                        move || DeviceRepo::openpgp_generate_blocking(admin_pin, slot, choice),
                        crate::tr!(
                            "Key generated. Use GnuPG to set the fingerprint and publish the key."
                        ),
                        status,
                        cx,
                    );
                });
            })
        };
        window.open_dialog(cx, move |dialog, _w, _| {
            let algo_sel = algo_sel.clone();
            let admin = admin.clone();
            let ok = submit.clone();
            let btn = submit.clone();
            dialog
                .title(crate::tr!("Generate — {}", slot.label()))
                .child(crate::tr!("Generates a new key pair in this slot (overwrites any existing key). This can take several seconds for RSA."))
                .child(
                    gpui_component::v_flex()
                        .gap_3()
                        .pb_2()
                        .child(crate::tr!("Algorithm"))
                        .child(
                            gpui_component::select::Select::new(&algo_sel)
                                .w_full()
                                .bg(rgb(0x222225)),
                        )
                        .child(crate::tr!("Admin PIN (PW3)"))
                        .child(gpui_component::input::Input::new(&admin)),
                )
                .on_ok(move |_, window, cx| {
                    ok(window, cx);
                    false
                })
                .footer(move |_, _w, _c, _| {
                    let s = btn.clone();
                    vec![
                        gpui_component::button::Button::new(crate::tr!("cancel"))
                            .label(crate::tr!("Cancel"))
                            .on_click(|_, window, cx| window.close_dialog(cx)),
                        gpui_component::button::Button::new(crate::tr!("gen"))
                            .primary()
                            .label(crate::tr!("Generate"))
                            .on_click(move |_, window, cx| s(window, cx)),
                    ]
                })
        });
    }

    // ── Touch policy ──────────────────────────────────────────────────────────

    pub(super) fn open_touch(
        &mut self,
        slot: PgpSlot,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let current = self
            .info
            .as_ref()
            .and_then(|i| i.keys.iter().find(|k| k.slot == slot))
            .map(|k| k.touch)
            .unwrap_or(false);
        let touch_sel = select_state(window, cx, OPT_TOUCH, if current { 1 } else { 0 });
        let admin = admin_input(window, cx);
        let view = cx.entity().downgrade();
        let submit = {
            let touch_sel = touch_sel.clone();
            let admin = admin.clone();
            let view = view.clone();
            std::rc::Rc::new(move |window: &mut Window, cx: &mut App| {
                let admin_pin = admin.read(cx).text().to_string();
                if admin_pin.is_empty() {
                    return;
                }
                let on = selected_key(&touch_sel, OPT_TOUCH, cx) == 1;
                window.close_dialog(cx);
                let status =
                    dialog::open_status_dialog(crate::tr!("Updating Touch Policy"), window, cx);
                let _ = view.update(cx, |this, cx| {
                    this.run(
                        move || DeviceRepo::openpgp_set_touch_blocking(admin_pin, slot, on),
                        crate::tr!("Touch policy updated."),
                        status,
                        cx,
                    );
                });
            })
        };
        window.open_dialog(cx, move |dialog, _w, _| {
            let touch_sel = touch_sel.clone();
            let admin = admin.clone();
            let ok = submit.clone();
            let btn = submit.clone();
            dialog
                .title(crate::tr!("Touch — {}", slot.label()))
                .child(crate::tr!(
                    "When on, this key requires a physical touch for every operation."
                ))
                .child(
                    gpui_component::v_flex()
                        .gap_3()
                        .pb_2()
                        .child(crate::tr!("Touch requirement"))
                        .child(
                            gpui_component::select::Select::new(&touch_sel)
                                .w_full()
                                .bg(rgb(0x222225)),
                        )
                        .child(crate::tr!("Admin PIN (PW3)"))
                        .child(gpui_component::input::Input::new(&admin)),
                )
                .on_ok(move |_, window, cx| {
                    ok(window, cx);
                    false
                })
                .footer(move |_, _w, _c, _| {
                    let s = btn.clone();
                    vec![
                        gpui_component::button::Button::new(crate::tr!("cancel"))
                            .label(crate::tr!("Cancel"))
                            .on_click(|_, window, cx| window.close_dialog(cx)),
                        gpui_component::button::Button::new(crate::tr!("ok"))
                            .primary()
                            .label(crate::tr!("Save"))
                            .on_click(move |_, window, cx| s(window, cx)),
                    ]
                })
        });
    }

    // ── Cardholder ────────────────────────────────────────────────────────────

    pub(super) fn open_cardholder(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (cur_name, cur_login, cur_url, cur_lang, cur_sex) = self
            .info
            .as_ref()
            .map(|i| {
                (
                    i.name.clone(),
                    i.login.clone(),
                    i.url.clone(),
                    i.lang.clone(),
                    i.sex,
                )
            })
            .unwrap_or((
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                0x39,
            ));
        let name =
            cx.new(|cx| gpui_component::input::InputState::new(window, cx).default_value(cur_name));
        let login = cx
            .new(|cx| gpui_component::input::InputState::new(window, cx).default_value(cur_login));
        let url =
            cx.new(|cx| gpui_component::input::InputState::new(window, cx).default_value(cur_url));
        let lang =
            cx.new(|cx| gpui_component::input::InputState::new(window, cx).default_value(cur_lang));
        let sex_row = OPT_SEX.iter().position(|(_, k)| *k == cur_sex).unwrap_or(0);
        let sex = select_state(window, cx, OPT_SEX, sex_row);
        let admin = admin_input(window, cx);
        let view = cx.entity().downgrade();
        let submit = {
            let name = name.clone();
            let login = login.clone();
            let url = url.clone();
            let lang = lang.clone();
            let sex = sex.clone();
            let admin = admin.clone();
            let view = view.clone();
            std::rc::Rc::new(move |window: &mut Window, cx: &mut App| {
                let admin_pin = admin.read(cx).text().to_string();
                if admin_pin.is_empty() {
                    return;
                }
                let name_v = name.read(cx).text().to_string();
                let login_v = login.read(cx).text().to_string();
                let url_v = url.read(cx).text().to_string();
                let lang_v = lang.read(cx).text().to_string();
                let sex_v = selected_key(&sex, OPT_SEX, cx);
                window.close_dialog(cx);
                let status =
                    dialog::open_status_dialog(crate::tr!("Saving Cardholder"), window, cx);
                let _ = view.update(cx, |this, cx| {
                    this.run(
                        move || {
                            DeviceRepo::openpgp_set_cardholder_blocking(
                                admin_pin, name_v, login_v, url_v, lang_v, sex_v,
                            )
                        },
                        crate::tr!("Cardholder details saved."),
                        status,
                        cx,
                    );
                });
            })
        };
        window.open_dialog(cx, move |dialog, _w, _| {
            let name = name.clone();
            let login = login.clone();
            let url = url.clone();
            let lang = lang.clone();
            let sex = sex.clone();
            let admin = admin.clone();
            let ok = submit.clone();
            let btn = submit.clone();
            dialog
                .title(crate::tr!("Edit Cardholder"))
                .child(crate::tr!(
                    "Cardholder metadata stored on the card. Requires the admin PIN."
                ))
                .child(
                    gpui_component::v_flex()
                        .gap_3()
                        .pb_2()
                        .child(crate::tr!("Name"))
                        .child(gpui_component::input::Input::new(&name))
                        .child(crate::tr!("Login"))
                        .child(gpui_component::input::Input::new(&login))
                        .child(crate::tr!("URL"))
                        .child(gpui_component::input::Input::new(&url))
                        .child(crate::tr!("Language (ISO-639, e.g. en)"))
                        .child(gpui_component::input::Input::new(&lang))
                        .child(crate::tr!("Sex"))
                        .child(gpui_component::select::Select::new(&sex))
                        .child(crate::tr!("Admin PIN (PW3)"))
                        .child(gpui_component::input::Input::new(&admin)),
                )
                .on_ok(move |_, window, cx| {
                    ok(window, cx);
                    false
                })
                .footer(move |_, _w, _c, _| {
                    let s = btn.clone();
                    vec![
                        gpui_component::button::Button::new(crate::tr!("cancel"))
                            .label(crate::tr!("Cancel"))
                            .on_click(|_, window, cx| window.close_dialog(cx)),
                        gpui_component::button::Button::new(crate::tr!("ok"))
                            .primary()
                            .label(crate::tr!("Save"))
                            .on_click(move |_, window, cx| s(window, cx)),
                    ]
                })
        });
    }

    // ── Reset ─────────────────────────────────────────────────────────────────

    pub(super) fn open_reset_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let view = cx.entity().downgrade();
        dialog::open_confirm(
            crate::tr!("Reset OpenPGP Applet"),
            crate::tr!("This blocks both PINs, then factory-resets the OpenPGP applet — deleting ALL keys and restoring the default PINs (123456 / 12345678). This cannot be undone.").to_string(),
            crate::tr!("Reset"),
            gpui_component::button::ButtonVariant::Danger,
            window,
            cx,
            move |_dh, window, cx| {
                window.close_dialog(cx);
                let status = dialog::open_status_dialog(crate::tr!("Resetting OpenPGP"), window, cx);
                let _ = view.update(cx, |this, cx| {
                    this.run(DeviceRepo::openpgp_reset_blocking, crate::tr!("OpenPGP applet reset."), status, cx);
                });
            },
        );
    }
}

/// A masked input seeded with the default admin PIN for management dialogs.
fn admin_input(
    window: &mut Window,
    cx: &mut Context<OpenPgpViewModel>,
) -> Entity<gpui_component::input::InputState> {
    cx.new(|cx| {
        gpui_component::input::InputState::new(window, cx)
            .masked(true)
            .default_value(openpgp::DEFAULT_PW3)
    })
}
