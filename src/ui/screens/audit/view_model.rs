//! View model for the Audit screen — export and verify the device's
//! tamper-evident security journal.

use crate::ui::DialogSubmit;
use crate::ui::app::AppModels;
use crate::ui::components::applet_gate::AppletGate;
use crate::ui::components::dialog;
use crate::ui::components::dialog::StatusContent;
use crate::ui::models::device::{DeviceEvent, DeviceRepo, audit};
use gpui_kit::component::WindowExt;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::component::input::InputState;
use gpui_kit::*;

pub struct AuditViewModel {
    pub(super) device: Entity<DeviceRepo>,
    pub(super) journal: Option<audit::AuditJournal>,
    pub(super) verification: Option<audit::AuditVerification>,
    /// Whether journalling is currently on. `None` until the status is read (the
    /// query is ungated, so it loads automatically). Journalling is opt-in.
    pub(super) enabled: Option<bool>,
    pub(super) loading: bool,
    _task: Option<Task<()>>,
}

impl AuditViewModel {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>, models: &AppModels) -> Self {
        let device = models.device.clone();
        cx.subscribe(&device, |this: &mut Self, _, _: &DeviceEvent, cx| {
            if this.device.read(cx).device_changed {
                this.journal = None;
                this.verification = None;
                this.enabled = None;
            }
            this.refresh_status(cx);
            cx.notify();
        })
        .detach();
        let mut this = Self {
            device,
            journal: None,
            verification: None,
            enabled: None,
            loading: false,
            _task: None,
        };
        this.refresh_status(cx);
        this
    }

    /// Load whether journalling is on (ungated — no PIN, no touch).
    pub(super) fn refresh_status(&mut self, cx: &mut Context<Self>) {
        if self.loading || self.gate(cx) != AppletGate::Ready {
            return;
        }
        let weak = cx.entity().downgrade();
        self._task = Some(cx.spawn(async move |_, cx| {
            let res = cx
                .background_executor()
                .spawn(async { DeviceRepo::audit_status_blocking() })
                .await;
            let _ = weak.update(cx, |this, cx| {
                this.enabled = res.ok();
                cx.notify();
            });
        }));
    }

    // ── Enable / disable journalling (PIN + touch) ──────────────────────────

    pub(super) fn open_toggle(
        &mut self,
        enable: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let pin = Self::pin_input(window, cx);
        let view = cx.entity().downgrade();
        let submit = {
            let pin = pin.clone();
            std::rc::Rc::new(move |window: &mut Window, cx: &mut App| {
                let p = pin.read(cx).text().to_string();
                let p = (!p.is_empty()).then_some(p);
                window.close_dialog(cx);
                let status = dialog::open_status_dialog(
                    if enable {
                        "Enabling Journalling"
                    } else {
                        "Disabling Journalling"
                    },
                    window,
                    cx,
                );
                let _ = view.update(cx, |this, cx| this.run_toggle(enable, p, status, cx));
            })
        };
        let (title, body) = if enable {
            (
                "Enable Audit Journalling",
                "Turns the tamper-evident journal ON — security events are then recorded to the key's flash. Requires the FIDO PIN (or a touch if none is set) plus a touch to confirm.",
            )
        } else {
            (
                "Disable Audit Journalling",
                "Turns the journal OFF — no further events are recorded. Requires the FIDO PIN (or a touch if none is set) plus a touch to confirm.",
            )
        };
        Self::open_gate_dialog(title, body, pin, None, submit, window, cx);
    }

    fn run_toggle(
        &mut self,
        enable: bool,
        pin: Option<String>,
        status: WeakEntity<StatusContent>,
        cx: &mut Context<Self>,
    ) {
        if self.loading {
            return;
        }
        self.loading = true;
        let _ = status.update(cx, |d, cx| {
            d.set_loading("Touch the device (BOOTSEL) to confirm.", cx)
        });
        cx.notify();
        let weak = cx.entity().downgrade();
        self._task = Some(cx.spawn(async move |_, cx| {
            let res = cx
                .background_executor()
                .spawn(async move { DeviceRepo::audit_set_enabled_blocking(enable, pin) })
                .await;
            let _ = weak.update(cx, |this, cx| {
                this.loading = false;
                match res {
                    Ok(on) => {
                        this.enabled = Some(on);
                        let _ = status.update(cx, |d, cx| {
                            d.set_success(
                                format!("Journalling {}.", if on { "enabled" } else { "disabled" }),
                                cx,
                            )
                        });
                    }
                    Err(e) => {
                        let _ = status.update(cx, |d, cx| d.set_error(e, cx));
                    }
                }
                cx.notify();
            });
        }));
    }

    pub(super) fn gate(&self, cx: &App) -> AppletGate {
        self.device.read(cx).rskey_vendor_gate()
    }

    /// Masked, optional PIN input (blank = authorise by touch).
    fn pin_input(window: &mut Window, cx: &mut Context<Self>) -> Entity<InputState> {
        cx.new(|cx| {
            InputState::new(window, cx)
                .masked(true)
                .placeholder("FIDO PIN — leave blank to touch instead")
        })
    }

    // ── Read journal ────────────────────────────────────────────────────────

    pub(super) fn open_read(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let pin = Self::pin_input(window, cx);
        let view = cx.entity().downgrade();
        let submit = {
            let pin = pin.clone();
            std::rc::Rc::new(move |window: &mut Window, cx: &mut App| {
                let p = pin.read(cx).text().to_string();
                let p = (!p.is_empty()).then_some(p);
                window.close_dialog(cx);
                let status = dialog::open_status_dialog("Reading Journal", window, cx);
                let _ = view.update(cx, |this, cx| this.run_read(p, status, cx));
            })
        };
        Self::open_gate_dialog(
            "Read Audit Journal",
            "Exports the security journal. Requires the FIDO PIN, or a touch if no PIN is set.",
            pin,
            None,
            submit,
            window,
            cx,
        );
    }

    fn run_read(
        &mut self,
        pin: Option<String>,
        status: WeakEntity<StatusContent>,
        cx: &mut Context<Self>,
    ) {
        if self.loading {
            return;
        }
        self.loading = true;
        let _ = status.update(cx, |d, cx| {
            d.set_loading("Reading… touch the device (BOOTSEL) if it blinks.", cx)
        });
        cx.notify();
        let weak = cx.entity().downgrade();
        self._task = Some(cx.spawn(async move |_, cx| {
            let res = cx
                .background_executor()
                .spawn(async move { DeviceRepo::audit_log_blocking(pin) })
                .await;
            let _ = weak.update(cx, |this, cx| {
                this.loading = false;
                match res {
                    Ok(journal) => {
                        let n = journal.entries.len();
                        this.journal = Some(journal);
                        this.verification = None;
                        let _ = status.update(cx, |d, cx| {
                            d.set_success(format!("Journal read — {n} entries."), cx)
                        });
                    }
                    Err(e) => {
                        let _ = status.update(cx, |d, cx| d.set_error(e, cx));
                    }
                }
                cx.notify();
            });
        }));
    }

    // ── Verify checkpoint ───────────────────────────────────────────────────

    pub(super) fn open_verify(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let pin = Self::pin_input(window, cx);
        let expect = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Expected key: 16-hex fingerprint or full pubkey (optional)")
        });
        let view = cx.entity().downgrade();
        let submit = {
            let pin = pin.clone();
            let expect = expect.clone();
            std::rc::Rc::new(move |window: &mut Window, cx: &mut App| {
                let p = pin.read(cx).text().to_string();
                let p = (!p.is_empty()).then_some(p);
                let e = expect.read(cx).text().to_string();
                let e = (!e.trim().is_empty()).then_some(e);
                window.close_dialog(cx);
                let status = dialog::open_status_dialog("Verifying Checkpoint", window, cx);
                let _ = view.update(cx, |this, cx| this.run_verify(p, e, status, cx));
            })
        };
        Self::open_gate_dialog(
            "Verify Audit Checkpoint",
            "Exports the journal and checks a fresh DEVK-signed checkpoint over it — proving the log is authentic and the device genuine.",
            pin,
            Some(("Expected key (optional)", expect)),
            submit,
            window,
            cx,
        );
    }

    fn run_verify(
        &mut self,
        pin: Option<String>,
        expect: Option<String>,
        status: WeakEntity<StatusContent>,
        cx: &mut Context<Self>,
    ) {
        if self.loading {
            return;
        }
        self.loading = true;
        let _ = status.update(cx, |d, cx| {
            d.set_loading("Signing checkpoint… touch the device (BOOTSEL).", cx)
        });
        cx.notify();
        let weak = cx.entity().downgrade();
        self._task = Some(cx.spawn(async move |_, cx| {
            let res = cx
                .background_executor()
                .spawn(async move { DeviceRepo::audit_verify_blocking(pin, expect) })
                .await;
            let _ = weak.update(cx, |this, cx| {
                this.loading = false;
                match res {
                    Ok(v) => {
                        let msg = if v.authentic() {
                            "Journal authentic — signature and chain verified.".to_string()
                        } else if !v.signature_ok {
                            "SIGNATURE INVALID — do not trust this journal.".to_string()
                        } else if !v.head_matches {
                            "Head mismatch — the journal changed mid-read (possible tamper)."
                                .to_string()
                        } else {
                            "Attestation key MISMATCH — not the enrolled device.".to_string()
                        };
                        this.journal = Some(v.journal.clone());
                        this.verification = Some(v);
                        let _ = status.update(cx, |d, cx| d.set_success(msg, cx));
                    }
                    Err(e) => {
                        let _ = status.update(cx, |d, cx| d.set_error(e, cx));
                    }
                }
                cx.notify();
            });
        }));
    }

    /// A dialog with an optional-PIN field and an optional second field.
    fn open_gate_dialog(
        title: &'static str,
        body: &'static str,
        pin: Entity<InputState>,
        extra: Option<(&'static str, Entity<InputState>)>,
        submit: DialogSubmit,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.open_dialog(cx, move |dialog, _w, _| {
            let pin = pin.clone();
            let extra = extra.clone();
            let ok = submit.clone();
            let btn = submit.clone();
            let mut fields = gpui_kit::component::v_flex()
                .gap_3()
                .pb_2()
                .child("FIDO PIN")
                .child(gpui_kit::component::input::Input::new(&pin));
            if let Some((label, input)) = &extra {
                fields = fields
                    .child(label.to_string())
                    .child(gpui_kit::component::input::Input::new(input));
            }
            dialog
                .title(title)
                .child(body)
                .child(fields)
                .on_ok(move |_, window, cx| {
                    ok(window, cx);
                    false
                })
                .footer({
                    let s = btn.clone();
                    gpui_kit::component::dialog::DialogFooter::new().children(vec![
                        gpui_kit::component::button::Button::new("cancel")
                            .label("Cancel")
                            .on_click(|_, window, cx| window.close_dialog(cx)),
                        gpui_kit::component::button::Button::new("run")
                            .primary()
                            .label("Run")
                            .on_click(move |_, window, cx| s(window, cx)),
                    ])
                })
        });
    }
}
