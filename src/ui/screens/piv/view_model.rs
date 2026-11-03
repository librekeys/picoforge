//! View model for the PIV screen — slot status, key generation, PIN/PUK
//! management, certificate export, and factory reset.

use crate::error::PFError;
use crate::ui::app::AppModels;
use crate::ui::components::applet_gate::AppletGate;
use crate::ui::components::dialog;
use crate::ui::components::dialog::StatusContent;
use crate::ui::components::form::{LabeledU8, select_state, selected_key};
use crate::ui::models::device::{DeviceEvent, DeviceRepo, MgmAuth, USB_CAP_PIV, piv};
use gpui::*;
use gpui_component::WindowExt;
use gpui_component::button::ButtonVariants;
use gpui_component::select::SelectState;

const OPT_ALGO: &[(&str, u8)] = &[
    ("ECC P-256", 0x11),
    ("ECC P-384", 0x14),
    ("Ed25519", 0xE0),
    ("X25519", 0xE1),
    ("RSA-2048", 0x07),
    ("RSA-3072", 0x05),
    ("RSA-4096", 0x16),
];
const OPT_PIN_POLICY: &[(&str, u8)] = &[("Default", 0), ("Never", 1), ("Once", 2), ("Always", 3)];
const OPT_TOUCH_POLICY: &[(&str, u8)] =
    &[("Default", 0), ("Never", 1), ("Always", 2), ("Cached", 3)];
const OPT_TRIES: &[(&str, u8)] = &[("3", 3), ("5", 5), ("8", 8), ("10", 10)];
const OPT_MGM_ALGO: &[(&str, u8)] = &[("AES-192", 0x0A), ("AES-128", 0x08), ("AES-256", 0x0C)];
const OPT_MGM_TOUCH: &[(&str, u8)] = &[("Not required", 0), ("Touch required", 1)];
const OPT_SLOTS: &[(&str, u8)] = &[
    ("Authentication (9A)", 0x9A),
    ("Signature (9C)", 0x9C),
    ("Key Management (9D)", 0x9D),
    ("Card Authentication (9E)", 0x9E),
];

fn default_mgm_hex() -> String {
    hex::encode(piv::DEFAULT_MGM_KEY)
}

fn parse_mgm(hex_str: &str) -> Option<Vec<u8>> {
    let b = hex::decode(hex_str.trim()).ok()?;
    matches!(b.len(), 16 | 24 | 32).then_some(b)
}

/// Whether a management-key byte length matches its AES algorithm id.
fn piv_key_len_ok(algo: u8, len: usize) -> bool {
    matches!((algo, len), (0x08, 16) | (0x0A, 24) | (0x0C, 32))
}

/// Resolve a management-auth dialog input to an [`MgmAuth`], or emit a validation
/// toast and return `None`. On a `--protect`'d card the field holds the PIN; else
/// a hex management key. `algo` is the card's read-back management-key algorithm.
fn resolve_mgm_auth(
    input: &Entity<gpui_component::input::InputState>,
    protected: bool,
    algo: u8,
    view: &WeakEntity<PivViewModel>,
    cx: &mut App,
) -> Option<MgmAuth> {
    let text = input.read(cx).text().to_string();
    if protected {
        let pin = text.trim().to_string();
        if pin.is_empty() {
            let _ = view.update(cx, |_, cx| {
                cx.emit(PivEvent::Notification(
                    crate::tr!("Enter the PIN to unlock the management key").into(),
                ));
            });
            return None;
        }
        return Some(MgmAuth::Pin(pin));
    }
    match parse_mgm(&text) {
        Some(key) => Some(MgmAuth::Key { key, algo }),
        None => {
            let _ = view.update(cx, |_, cx| {
                cx.emit(PivEvent::Notification(
                    crate::tr!("Management key must be 16/24/32-byte hex").into(),
                ));
            });
            None
        }
    }
}

pub struct PivViewModel {
    pub(super) device: Entity<DeviceRepo>,
    pub(super) info: Option<piv::PivInfo>,
    pub(super) loaded: bool,
    pub(super) loading: bool,
    _task: Option<Task<()>>,
}

pub enum PivEvent {
    Notification(String),
}

impl EventEmitter<PivEvent> for PivViewModel {}

impl PivViewModel {
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
        match repo.piv_features() {
            None => AppletGate::Unsupported,
            Some(_) if !repo.ccid_on() => AppletGate::CcidOff,
            Some(_) if !repo.applet_enabled(USB_CAP_PIV) => AppletGate::Disabled(crate::tr!("PIV")),
            Some(_) => AppletGate::Ready,
        }
    }

    /// The stored management-key algorithm (default AES-192).
    fn mgm_algo(&self) -> u8 {
        self.info
            .as_ref()
            .map(|i| i.mgm_algo)
            .unwrap_or(piv::ALGO_AES192)
    }

    /// Whether this card's management key is PIN-protected (ykman `--protect`).
    fn mgm_protected(&self) -> bool {
        self.info.as_ref().map(|i| i.mgm_protected).unwrap_or(false)
    }

    /// The management-auth input for a dialog: a masked PIN field on a
    /// `--protect`'d card, else a hex management-key field defaulting to the key.
    fn mgm_input(
        &self,
        window: &mut Window,
        cx: &mut App,
    ) -> Entity<gpui_component::input::InputState> {
        if self.mgm_protected() {
            cx.new(|cx| gpui_component::input::InputState::new(window, cx).masked(true))
        } else {
            cx.new(|cx| {
                gpui_component::input::InputState::new(window, cx).default_value(default_mgm_hex())
            })
        }
    }

    /// The dialog label for the management-auth field.
    fn mgm_label(&self) -> &'static str {
        if self.mgm_protected() {
            crate::tr!("PIN (the management key is PIN-protected)")
        } else {
            crate::tr!("Management key (hex)")
        }
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
                .spawn(async { DeviceRepo::piv_read_info_blocking() })
                .await;
            let _ = weak.update(cx, |this, cx| {
                this.loading = false;
                match res {
                    Ok(info) => {
                        this.info = Some(info);
                        this.loaded = true;
                    }
                    Err(e) => {
                        log::warn!("{}", crate::tr!("PIV read failed: {e}", e = e));
                        cx.emit(PivEvent::Notification(crate::tr!("PIV: {e}", e = e)));
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

    // ── Generate ──────────────────────────────────────────────────────────

    pub(super) fn open_generate_dialog(
        &mut self,
        slot: u8,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let algo_sel = select_state(window, cx, OPT_ALGO, 0);
        let pin_sel = select_state(window, cx, OPT_PIN_POLICY, 0);
        let touch_sel = select_state(window, cx, OPT_TOUCH_POLICY, 0);
        let mgm = self.mgm_input(window, cx);
        let mgm_algo = self.mgm_algo();
        let protected = self.mgm_protected();
        let mgm_label = self.mgm_label();

        let view = cx.entity().downgrade();
        let submit = {
            let algo_sel = algo_sel.clone();
            let pin_sel = pin_sel.clone();
            let touch_sel = touch_sel.clone();
            let mgm = mgm.clone();
            let view = view.clone();
            std::rc::Rc::new(move |window: &mut Window, cx: &mut App| {
                let Some(auth) = resolve_mgm_auth(&mgm, protected, mgm_algo, &view, cx) else {
                    return;
                };
                let algo = selected_key(&algo_sel, OPT_ALGO, cx);
                let pin_pol = selected_key(&pin_sel, OPT_PIN_POLICY, cx);
                let touch_pol = selected_key(&touch_sel, OPT_TOUCH_POLICY, cx);
                window.close_dialog(cx);
                let status = dialog::open_status_dialog(crate::tr!("Generating Key"), window, cx);
                let _ = view.update(cx, |this, cx| {
                    this.run(
                        move || {
                            DeviceRepo::piv_generate_blocking(slot, algo, pin_pol, touch_pol, auth)
                                .map(|_| ())
                        },
                        crate::tr!("Key generated."),
                        status,
                        cx,
                    );
                });
            })
        };

        window.open_dialog(cx, move |dialog, _window, _| {
            let algo_sel = algo_sel.clone();
            let pin_sel = pin_sel.clone();
            let touch_sel = touch_sel.clone();
            let mgm = mgm.clone();
            let ok = submit.clone();
            let btn = submit.clone();
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
                .title(crate::tr!("Generate — {}", piv::slot_label(slot)))
                .child(crate::tr!(
                    "Generates a new key pair and a self-signed certificate in this slot."
                ))
                .child(
                    gpui_component::v_flex()
                        .gap_3()
                        .pb_2()
                        .child(field(crate::tr!("Algorithm"), &algo_sel))
                        .child(
                            gpui_component::h_flex()
                                .gap_3()
                                .child(field(crate::tr!("PIN policy"), &pin_sel))
                                .child(field(crate::tr!("Touch policy"), &touch_sel)),
                        )
                        .child(mgm_label)
                        .child(gpui_component::input::Input::new(&mgm)),
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

    // ── Export certificate (DER → PEM file) ─────────────────────────────────

    pub(super) fn open_export_cert(
        &mut self,
        slot: u8,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let default_dir = std::env::var(crate::tr!("HOME"))
            .map(std::path::PathBuf::from)
            .unwrap_or_default();
        let receiver = cx.prompt_for_new_path(
            &default_dir,
            Some(&crate::tr!("piv-{slot:02x}.pem", slot = slot)),
        );
        let view = cx.entity().downgrade();
        self._task = Some(cx.spawn(async move |_, cx| {
            let Ok(Ok(Some(path))) = receiver.await else {
                return;
            };
            let der = cx
                .background_executor()
                .spawn(async move { DeviceRepo::piv_export_cert_blocking(slot) })
                .await;
            let _ = view.update(cx, |_, cx| match der {
                Ok(der) => {
                    let pem = der_to_pem(&der);
                    match std::fs::write(&path, pem) {
                        Ok(_) => cx.emit(PivEvent::Notification(crate::tr!(
                            "Certificate saved to {}",
                            path.display()
                        ))),
                        Err(e) => cx.emit(PivEvent::Notification(crate::tr!(
                            "Save failed: {e}",
                            e = e
                        ))),
                    }
                }
                Err(e) => cx.emit(PivEvent::Notification(crate::tr!(
                    "Export failed: {e}",
                    e = e
                ))),
            });
        }));
    }

    // ── PIN / PUK ───────────────────────────────────────────────────────────

    pub(super) fn open_change_pin(
        &mut self,
        is_puk: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let title = if is_puk {
            crate::tr!("Change PUK")
        } else {
            crate::tr!("Change PIN")
        };
        self.two_secret_dialog(
            title,
            if is_puk {
                crate::tr!("Current PUK")
            } else {
                crate::tr!("Current PIN")
            },
            if is_puk {
                crate::tr!("New PUK")
            } else {
                crate::tr!("New PIN")
            },
            window,
            cx,
            move |cur, new| {
                if is_puk {
                    DeviceRepo::piv_change_puk_blocking(cur, new)
                } else {
                    DeviceRepo::piv_change_pin_blocking(cur, new)
                }
            },
            if is_puk {
                crate::tr!("PUK changed.")
            } else {
                crate::tr!("PIN changed.")
            },
        );
    }

    pub(super) fn open_unblock_pin(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.two_secret_dialog(
            crate::tr!("Unblock PIN"),
            crate::tr!("PUK"),
            crate::tr!("New PIN"),
            window,
            cx,
            DeviceRepo::piv_unblock_pin_blocking,
            crate::tr!("PIN unblocked."),
        );
    }

    /// A two-masked-field dialog (current/new or puk/new) running a blocking op.
    // TODO: refactor into parameter struct to remove this clippy escape
    #[allow(clippy::too_many_arguments)]
    fn two_secret_dialog(
        &mut self,
        title: &'static str,
        label_a: &'static str,
        label_b: &'static str,
        window: &mut Window,
        cx: &mut Context<Self>,
        op: impl Fn(String, String) -> Result<(), PFError> + Send + Clone + 'static,
        ok_msg: &'static str,
    ) {
        let a = cx.new(|cx| gpui_component::input::InputState::new(window, cx).masked(true));
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

    // ── Delete certificate (mgmt-gated) ─────────────────────────────────────

    pub(super) fn open_delete_cert(
        &mut self,
        slot: u8,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let mgm_algo = self.mgm_algo();
        let mgm = self.mgm_input(window, cx);
        let protected = self.mgm_protected();
        let mgm_label = self.mgm_label();
        let view = cx.entity().downgrade();
        let submit = {
            let mgm = mgm.clone();
            let view = view.clone();
            std::rc::Rc::new(move |window: &mut Window, cx: &mut App| {
                let Some(auth) = resolve_mgm_auth(&mgm, protected, mgm_algo, &view, cx) else {
                    return;
                };
                window.close_dialog(cx);
                let status =
                    dialog::open_status_dialog(crate::tr!("Deleting Certificate"), window, cx);
                let _ = view.update(cx, |this, cx| {
                    this.run(
                        move || DeviceRepo::piv_delete_cert_blocking(slot, auth),
                        crate::tr!("Certificate deleted."),
                        status,
                        cx,
                    );
                });
            })
        };
        window.open_dialog(cx, move |dialog, _w, _| {
            let mgm = mgm.clone();
            let ok = submit.clone();
            let btn = submit.clone();
            dialog
                .title(crate::tr!("Delete certificate — {}", piv::slot_label(slot)))
                .child(crate::tr!(
                    "Clears this slot's certificate (the key stays). Requires the management key."
                ))
                .child(
                    gpui_component::v_flex()
                        .gap_2()
                        .pb_2()
                        .child(mgm_label)
                        .child(gpui_component::input::Input::new(&mgm)),
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
                        gpui_component::button::Button::new(crate::tr!("del"))
                            .danger()
                            .label(crate::tr!("Delete"))
                            .on_click(move |_, window, cx| s(window, cx)),
                    ]
                })
        });
    }

    // ── Set PIN retries (mgmt + PIN) ─────────────────────────────────────────

    pub(super) fn open_set_retries(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let mgm_algo = self.mgm_algo();
        let mgm = self.mgm_input(window, cx);
        let protected = self.mgm_protected();
        let mgm_label = self.mgm_label();
        let pin = cx.new(|cx| gpui_component::input::InputState::new(window, cx).masked(true));
        let pin_tries = select_state(window, cx, OPT_TRIES, 0);
        let puk_tries = select_state(window, cx, OPT_TRIES, 0);
        let view = cx.entity().downgrade();
        let submit = {
            let mgm = mgm.clone();
            let pin = pin.clone();
            let pin_tries = pin_tries.clone();
            let puk_tries = puk_tries.clone();
            let view = view.clone();
            std::rc::Rc::new(move |window: &mut Window, cx: &mut App| {
                let Some(auth) = resolve_mgm_auth(&mgm, protected, mgm_algo, &view, cx) else {
                    return;
                };
                let pin_v = pin.read(cx).text().to_string();
                if pin_v.is_empty() {
                    return;
                }
                let pt = selected_key(&pin_tries, OPT_TRIES, cx);
                let ut = selected_key(&puk_tries, OPT_TRIES, cx);
                window.close_dialog(cx);
                let status = dialog::open_status_dialog(crate::tr!("Setting Retries"), window, cx);
                let _ = view.update(cx, |this, cx| {
                    this.run(
                        move || DeviceRepo::piv_set_retries_blocking(auth, pin_v, pt, ut),
                        crate::tr!("Retry counters updated (PIN/PUK reset to defaults)."),
                        status,
                        cx,
                    );
                });
            })
        };
        window.open_dialog(cx, move |dialog, _w, _| {
            let mgm = mgm.clone();
            let pin = pin.clone();
            let pin_tries = pin_tries.clone();
            let puk_tries = puk_tries.clone();
            let ok = submit.clone();
            let btn = submit.clone();
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
                .title(crate::tr!("Set PIN Retries"))
                .child(crate::tr!(
                    "Resets the PIN and PUK to their defaults and sets new retry limits."
                ))
                .child(
                    gpui_component::v_flex()
                        .gap_3()
                        .pb_2()
                        .child(
                            gpui_component::h_flex()
                                .gap_3()
                                .child(field(crate::tr!("PIN retries"), &pin_tries))
                                .child(field(crate::tr!("PUK retries"), &puk_tries)),
                        )
                        .child(crate::tr!("Current PIN"))
                        .child(gpui_component::input::Input::new(&pin))
                        .child(mgm_label)
                        .child(gpui_component::input::Input::new(&mgm)),
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
                            .label(crate::tr!("Apply"))
                            .on_click(move |_, window, cx| s(window, cx)),
                    ]
                })
        });
    }

    // ── Change management key (mgmt-gated) ───────────────────────────────────

    pub(super) fn open_change_mgm(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let cur_algo = self.mgm_algo();
        let protected = self.mgm_protected();
        let cur_label = if protected {
            crate::tr!("PIN (unlocks the current management key)")
        } else {
            crate::tr!("Current management key (hex)")
        };
        let cur = self.mgm_input(window, cx);
        let new = cx.new(|cx| gpui_component::input::InputState::new(window, cx));
        let algo_sel = select_state(window, cx, OPT_MGM_ALGO, 0);
        let touch_sel = select_state(window, cx, OPT_MGM_TOUCH, 0);
        let view = cx.entity().downgrade();

        let gen_key = {
            let new = new.clone();
            let algo_sel = algo_sel.clone();
            std::rc::Rc::new(move |window: &mut Window, cx: &mut App| {
                let algo = selected_key(&algo_sel, OPT_MGM_ALGO, cx);
                if let Ok(k) = piv::random_key(algo) {
                    new.update(cx, |st, cx| st.set_value(hex::encode(k), window, cx));
                }
            })
        };
        let submit = {
            let cur = cur.clone();
            let new = new.clone();
            let algo_sel = algo_sel.clone();
            let touch_sel = touch_sel.clone();
            let view = view.clone();
            std::rc::Rc::new(move |window: &mut Window, cx: &mut App| {
                let notify = |cx: &mut App, msg: &str| {
                    let _ =
                        view.update(cx, |_, cx| cx.emit(PivEvent::Notification(msg.to_string())));
                };
                let Some(current) = resolve_mgm_auth(&cur, protected, cur_algo, &view, cx) else {
                    return;
                };
                let new_algo = selected_key(&algo_sel, OPT_MGM_ALGO, cx);
                let new_key = match hex::decode(new.read(cx).text().to_string().trim()) {
                    Ok(k) if piv_key_len_ok(new_algo, k.len()) => k,
                    _ => {
                        return notify(
                            cx,
                            crate::tr!("New key length must match the algorithm (16/24/32 bytes)"),
                        );
                    }
                };
                let touch = selected_key(&touch_sel, OPT_MGM_TOUCH, cx) == 1;
                window.close_dialog(cx);
                let status =
                    dialog::open_status_dialog(crate::tr!("Changing Management Key"), window, cx);
                let _ = view.update(cx, |this, cx| {
                    this.run(
                        move || DeviceRepo::piv_set_mgm_blocking(current, new_algo, new_key, touch),
                        crate::tr!("Management key changed."),
                        status,
                        cx,
                    );
                });
            })
        };
        window.open_dialog(cx, move |dialog, _w, _| {
            let cur = cur.clone();
            let new = new.clone();
            let algo_sel = algo_sel.clone();
            let touch_sel = touch_sel.clone();
            let gen_key = gen_key.clone();
            let ok = submit.clone();
            let btn = submit.clone();
            let field = |label: &str, sel: &Entity<SelectState<Vec<LabeledU8>>>| {
                gpui_component::v_flex().gap_1().flex_1().child(label.to_string()).child(
                    gpui_component::select::Select::new(sel).w_full().bg(rgb(0x222225)),
                )
            };
            dialog
                .title(crate::tr!("Change Management Key"))
                .child(crate::tr!("Sets a new PIV management key. Store it safely — it is required for all key/cert changes."))
                .child(
                    gpui_component::v_flex()
                        .gap_3()
                        .pb_2()
                        .child(cur_label)
                        .child(gpui_component::input::Input::new(&cur))
                        .child(
                            gpui_component::h_flex()
                                .gap_3()
                                .child(field(crate::tr!("New algorithm"), &algo_sel))
                                .child(field(crate::tr!("Touch"), &touch_sel)),
                        )
                        .child(crate::tr!("New key (hex)"))
                        .child(
                            gpui_component::h_flex()
                                .gap_2()
                                .items_center()
                                .child(
                                    gpui_component::v_flex()
                                        .flex_1()
                                        .child(gpui_component::input::Input::new(&new)),
                                )
                                .child(
                                    gpui_component::button::Button::new("gen-mgm")
                                        .label(crate::tr!("Generate"))
                                        .outline()
                                        .on_click(move |_, window, cx| gen_key(window, cx)),
                                ),
                        ),
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
                            .label(crate::tr!("Change"))
                            .on_click(move |_, window, cx| s(window, cx)),
                    ]
                })
        });
    }

    // ── Import certificate / key (file → management-key dialog) ──────────────

    pub(super) fn open_import_cert(
        &mut self,
        slot: u8,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let handle = window.window_handle();
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some(crate::tr!("Select certificate (PEM or DER)").into()),
        });
        let view = cx.entity().downgrade();
        self._task = Some(cx.spawn(async move |_, cx| {
            let Ok(Ok(Some(paths))) = receiver.await else {
                return;
            };
            let Some(path) = paths.into_iter().next() else {
                return;
            };
            let Ok(bytes) = std::fs::read(&path) else {
                let _ = view.update(cx, |_, cx| {
                    cx.emit(PivEvent::Notification(
                        crate::tr!("Could not read file").into(),
                    ))
                });
                return;
            };
            let Some(der) = cert_pem_to_der(&bytes) else {
                let _ = view.update(cx, |_, cx| {
                    cx.emit(PivEvent::Notification(
                        crate::tr!("Not a valid PEM/DER certificate").into(),
                    ))
                });
                return;
            };
            let _ = cx.update_window(handle, |_, window, cx| {
                let _ = view.update(cx, |this, cx| {
                    this.open_mgm_import(slot, der, false, window, cx)
                });
            });
        }));
    }

    pub(super) fn open_import_key(
        &mut self,
        slot: u8,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let handle = window.window_handle();
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some(crate::tr!("Select private key (PEM or DER)").into()),
        });
        let view = cx.entity().downgrade();
        self._task = Some(cx.spawn(async move |_, cx| {
            let Ok(Ok(Some(paths))) = receiver.await else {
                return;
            };
            let Some(path) = paths.into_iter().next() else {
                return;
            };
            let Ok(bytes) = std::fs::read(&path) else {
                let _ = view.update(cx, |_, cx| {
                    cx.emit(PivEvent::Notification(
                        crate::tr!("Could not read file").into(),
                    ))
                });
                return;
            };
            let _ = cx.update_window(handle, |_, window, cx| {
                let _ = view.update(cx, |this, cx| {
                    this.open_mgm_import(slot, bytes, true, window, cx)
                });
            });
        }));
    }

    fn open_mgm_import(
        &mut self,
        slot: u8,
        file: Vec<u8>,
        is_key: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let mgm_algo = self.mgm_algo();
        let mgm = self.mgm_input(window, cx);
        let protected = self.mgm_protected();
        let mgm_label = self.mgm_label();
        let view = cx.entity().downgrade();
        let submit = {
            let mgm = mgm.clone();
            let view = view.clone();
            std::rc::Rc::new(move |window: &mut Window, cx: &mut App| {
                let Some(auth) = resolve_mgm_auth(&mgm, protected, mgm_algo, &view, cx) else {
                    return;
                };
                let file = file.clone();
                window.close_dialog(cx);
                let status = dialog::open_status_dialog(
                    if is_key {
                        crate::tr!("Importing Key")
                    } else {
                        crate::tr!("Importing Certificate")
                    },
                    window,
                    cx,
                );
                let _ = view.update(cx, |this, cx| {
                    if is_key {
                        this.run(
                            move || DeviceRepo::piv_import_key_blocking(slot, file, auth),
                            crate::tr!("Key imported."),
                            status,
                            cx,
                        );
                    } else {
                        this.run(
                            move || DeviceRepo::piv_import_cert_blocking(slot, file, auth),
                            crate::tr!("Certificate imported."),
                            status,
                            cx,
                        );
                    }
                });
            })
        };
        window.open_dialog(cx, move |dialog, _w, _| {
            let mgm = mgm.clone();
            let ok = submit.clone();
            let btn = submit.clone();
            dialog
                .title(if is_key {
                    crate::tr!("Import Key")
                } else {
                    crate::tr!("Import Certificate")
                })
                .child(crate::tr!(
                    "Enter the management key to authorise the import."
                ))
                .child(
                    gpui_component::v_flex()
                        .gap_2()
                        .pb_2()
                        .child(mgm_label)
                        .child(gpui_component::input::Input::new(&mgm)),
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
                            .label(crate::tr!("Import"))
                            .on_click(move |_, window, cx| s(window, cx)),
                    ]
                })
        });
    }

    // ── Attestation (export the attestation cert of a generated key) ─────────

    pub(super) fn open_attest(&mut self, slot: u8, _window: &mut Window, cx: &mut Context<Self>) {
        let default_dir = std::env::var(crate::tr!("HOME"))
            .map(std::path::PathBuf::from)
            .unwrap_or_default();
        let receiver = cx.prompt_for_new_path(
            &default_dir,
            Some(&crate::tr!("piv-{slot:02x}-attestation.pem", slot = slot)),
        );
        let view = cx.entity().downgrade();
        self._task = Some(cx.spawn(async move |_, cx| {
            let Ok(Ok(Some(path))) = receiver.await else {
                return;
            };
            let der = cx
                .background_executor()
                .spawn(async move { DeviceRepo::piv_attest_blocking(slot) })
                .await;
            let _ = view.update(cx, |_, cx| match der {
                Ok(der) => match std::fs::write(&path, der_to_pem(&der)) {
                    Ok(_) => cx.emit(PivEvent::Notification(crate::tr!(
                        "Attestation saved to {}",
                        path.display()
                    ))),
                    Err(e) => cx.emit(PivEvent::Notification(crate::tr!(
                        "Save failed: {e}",
                        e = e
                    ))),
                },
                Err(e) => cx.emit(PivEvent::Notification(crate::tr!(
                    "Attestation failed: {e}",
                    e = e
                ))),
            });
        }));
    }

    // ── Delete key (mgmt-gated) ──────────────────────────────────────────────

    pub(super) fn open_delete_key(
        &mut self,
        slot: u8,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let mgm_algo = self.mgm_algo();
        let mgm = self.mgm_input(window, cx);
        let protected = self.mgm_protected();
        let mgm_label = self.mgm_label();
        let view = cx.entity().downgrade();
        let submit = {
            let mgm = mgm.clone();
            let view = view.clone();
            std::rc::Rc::new(move |window: &mut Window, cx: &mut App| {
                let Some(auth) = resolve_mgm_auth(&mgm, protected, mgm_algo, &view, cx) else {
                    return;
                };
                window.close_dialog(cx);
                let status = dialog::open_status_dialog(crate::tr!("Deleting Key"), window, cx);
                let _ = view.update(cx, |this, cx| {
                    this.run(
                        move || DeviceRepo::piv_delete_key_blocking(slot, auth),
                        crate::tr!("Key deleted."),
                        status,
                        cx,
                    );
                });
            })
        };
        window.open_dialog(cx, move |dialog, _w, _| {
            let mgm = mgm.clone();
            let ok = submit.clone();
            let btn = submit.clone();
            dialog
                .title(crate::tr!("Delete key — {}", piv::slot_label(slot)))
                .child(crate::tr!("Permanently deletes this slot's key and certificate. Requires the management key."))
                .child(
                    gpui_component::v_flex()
                        .gap_2()
                        .pb_2()
                        .child(mgm_label)
                        .child(gpui_component::input::Input::new(&mgm)),
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
                        gpui_component::button::Button::new(crate::tr!("del"))
                            .danger()
                            .label(crate::tr!("Delete"))
                            .on_click(move |_, window, cx| s(window, cx)),
                    ]
                })
        });
    }

    // ── Move key (mgmt-gated) ────────────────────────────────────────────────

    pub(super) fn open_move_key(&mut self, src: u8, window: &mut Window, cx: &mut Context<Self>) {
        let mgm_algo = self.mgm_algo();
        let protected = self.mgm_protected();
        let mgm_label = self.mgm_label();
        let dst_sel = select_state(window, cx, OPT_SLOTS, 0);
        let mgm = self.mgm_input(window, cx);
        let view = cx.entity().downgrade();
        let submit = {
            let dst_sel = dst_sel.clone();
            let mgm = mgm.clone();
            let view = view.clone();
            std::rc::Rc::new(move |window: &mut Window, cx: &mut App| {
                let Some(auth) = resolve_mgm_auth(&mgm, protected, mgm_algo, &view, cx) else {
                    return;
                };
                let dst = selected_key(&dst_sel, OPT_SLOTS, cx);
                if dst == src {
                    let _ = view.update(cx, |_, cx| {
                        cx.emit(PivEvent::Notification(
                            crate::tr!("Choose a different destination slot").into(),
                        ));
                    });
                    return;
                }
                window.close_dialog(cx);
                let status = dialog::open_status_dialog(crate::tr!("Moving Key"), window, cx);
                let _ = view.update(cx, |this, cx| {
                    this.run(
                        move || DeviceRepo::piv_move_key_blocking(src, dst, auth),
                        crate::tr!("Key moved."),
                        status,
                        cx,
                    );
                });
            })
        };
        window.open_dialog(cx, move |dialog, _w, _| {
            let dst_sel = dst_sel.clone();
            let mgm = mgm.clone();
            let ok = submit.clone();
            let btn = submit.clone();
            dialog
                .title(crate::tr!("Move key from {}", piv::slot_label(src)))
                .child(crate::tr!(
                    "Moves the key + certificate to another slot (overwrites the destination)."
                ))
                .child(
                    gpui_component::v_flex()
                        .gap_3()
                        .pb_2()
                        .child(crate::tr!("Destination slot"))
                        .child(
                            gpui_component::select::Select::new(&dst_sel)
                                .w_full()
                                .bg(rgb(0x222225)),
                        )
                        .child(mgm_label)
                        .child(gpui_component::input::Input::new(&mgm)),
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
                        gpui_component::button::Button::new(crate::tr!("mv"))
                            .primary()
                            .label(crate::tr!("Move"))
                            .on_click(move |_, window, cx| s(window, cx)),
                    ]
                })
        });
    }

    // ── Reset ────────────────────────────────────────────────────────────────

    pub(super) fn open_reset_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let view = cx.entity().downgrade();
        dialog::open_confirm(
            crate::tr!("Reset PIV Applet"),
            crate::tr!("This blocks the PIN and PUK, then factory-resets PIV — deleting ALL keys and certificates and restoring the default PIN/PUK/management key. This cannot be undone.").to_string(),
            crate::tr!("Reset"),
            gpui_component::button::ButtonVariant::Danger,
            window,
            cx,
            move |_dh, window, cx| {
                window.close_dialog(cx);
                let status = dialog::open_status_dialog(crate::tr!("Resetting PIV"), window, cx);
                let _ = view.update(cx, |this, cx| {
                    this.run(DeviceRepo::piv_reset_blocking, crate::tr!("PIV applet reset."), status, cx);
                });
            },
        );
    }
}

/// Decode a certificate from PEM or accept raw DER.
fn cert_pem_to_der(input: &[u8]) -> Option<Vec<u8>> {
    let text = std::str::from_utf8(input).unwrap_or("");
    if let Some(begin) = text.find(crate::tr!("-----BEGIN CERTIFICATE-----")) {
        let body = &text[begin + 27..];
        let end = body.find(crate::tr!("-----END"))?;
        let b64: String = body[..end].chars().filter(|c| !c.is_whitespace()).collect();
        use base64::Engine;
        base64::engine::general_purpose::STANDARD
            .decode(b64.as_bytes())
            .ok()
    } else if input.first() == Some(&0x30) {
        Some(input.to_vec())
    } else {
        None
    }
}

/// Wrap a DER certificate as PEM.
fn der_to_pem(der: &[u8]) -> String {
    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD.encode(der);
    let mut out = String::from(crate::tr!("-----BEGIN CERTIFICATE-----\n"));
    for chunk in b64.as_bytes().chunks(64) {
        out.push_str(std::str::from_utf8(chunk).unwrap_or(""));
        out.push('\n');
    }
    out.push_str(crate::tr!("-----END CERTIFICATE-----\n"));
    out
}
