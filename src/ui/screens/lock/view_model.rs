//! View model for the Lock screen — at-rest soft-lock of the FIDO seed.

use crate::ui::DialogSubmit;
use crate::ui::app::AppModels;
use crate::ui::components::applet_gate::AppletGate;
use crate::ui::components::dialog;
use crate::ui::components::dialog::StatusContent;
use crate::ui::models::device::{DeviceEvent, DeviceRepo, backup};
use gpui_kit::component::WindowExt;
use gpui_kit::component::button::{ButtonVariant, ButtonVariants};
use gpui_kit::component::input::InputState;
use gpui_kit::*;

pub struct LockViewModel {
    pub(super) device: Entity<DeviceRepo>,
    pub(super) status: Option<backup::BackupStatus>,
    /// The lock-key phrase produced by `enable`, held until cleared.
    pub(super) lock_key: Option<String>,
    pub(super) loading: bool,
    _task: Option<Task<()>>,
}

impl LockViewModel {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>, models: &AppModels) -> Self {
        let device = models.device.clone();
        cx.subscribe(&device, |this: &mut Self, _, _: &DeviceEvent, cx| {
            if this.device.read(cx).device_changed {
                this.status = None;
                this.lock_key = None;
            }
            this.load(cx);
            cx.notify();
        })
        .detach();
        let mut this = Self {
            device,
            status: None,
            lock_key: None,
            loading: false,
            _task: None,
        };
        this.load(cx);
        this
    }

    pub(super) fn gate(&self, cx: &App) -> AppletGate {
        self.device.read(cx).rskey_vendor_gate()
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
                .spawn(async { DeviceRepo::backup_status_blocking() })
                .await;
            let _ = weak.update(cx, |this, cx| {
                this.loading = false;
                if let Ok(s) = res {
                    this.status = Some(s);
                }
                cx.notify();
            });
        }));
    }

    pub(super) fn refresh(&mut self, cx: &mut Context<Self>) {
        self.load(cx);
    }

    pub(super) fn clear_lock_key(&mut self, cx: &mut Context<Self>) {
        self.lock_key = None;
        cx.notify();
    }

    fn pin_input(window: &mut Window, cx: &mut Context<Self>) -> Entity<InputState> {
        cx.new(|cx| {
            InputState::new(window, cx)
                .masked(true)
                .placeholder("FIDO PIN (required)")
        })
    }

    fn phrase_input(window: &mut Window, cx: &mut Context<Self>) -> Entity<InputState> {
        cx.new(|cx| InputState::new(window, cx).placeholder("24-word lock key"))
    }

    // ── Enable ──────────────────────────────────────────────────────────────

    pub(super) fn open_enable(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let pin = Self::pin_input(window, cx);
        let view = cx.entity().downgrade();
        let submit = {
            let pin = pin.clone();
            std::rc::Rc::new(move |window: &mut Window, cx: &mut App| {
                let p = pin.read(cx).text().to_string();
                if p.is_empty() {
                    return;
                }
                window.close_dialog(cx);
                let status = dialog::open_status_dialog("Engaging Lock", window, cx);
                let _ = view.update(cx, |this, cx| this.run_enable(p, status, cx));
            })
        };
        Self::dialog(
            "Engage At-Rest Lock",
            "Wraps the FIDO seed under a fresh lock key and erases the plaintext. After this, EVERY power-cycle needs an unlock before any FIDO login works. Losing the lock key means the only recovery is a factory reset, which destroys this identity. A FIDO PIN is required; touch to confirm.",
            None,
            pin,
            ("Engage lock", ButtonVariant::Danger),
            submit,
            window,
            cx,
        );
    }

    fn run_enable(
        &mut self,
        pin: String,
        status: WeakEntity<StatusContent>,
        cx: &mut Context<Self>,
    ) {
        if self.loading {
            return;
        }
        self.loading = true;
        let _ = status.update(cx, |d, cx| {
            d.set_loading("Engaging… touch the device (BOOTSEL).", cx)
        });
        cx.notify();
        let weak = cx.entity().downgrade();
        self._task = Some(cx.spawn(async move |_, cx| {
            let res = cx
                .background_executor()
                .spawn(async move { DeviceRepo::lock_enable_blocking(pin) })
                .await;
            let _ = weak.update(cx, |this, cx| {
                this.loading = false;
                match res {
                    Ok(phrase) => {
                        this.lock_key = Some(phrase);
                        this.load(cx);
                        let _ = status.update(cx, |d, cx| {
                            d.set_success(
                                "Locked — write down the lock key shown below; you need it every power-cycle.".into(),
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

    // ── Unlock ──────────────────────────────────────────────────────────────

    pub(super) fn open_unlock(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let phrase = Self::phrase_input(window, cx);
        let view = cx.entity().downgrade();
        let submit = {
            let phrase = phrase.clone();
            std::rc::Rc::new(move |window: &mut Window, cx: &mut App| {
                let m = phrase.read(cx).text().to_string();
                if m.trim().is_empty() {
                    return;
                }
                window.close_dialog(cx);
                let status = dialog::open_status_dialog("Unlocking", window, cx);
                let _ = view.update(cx, |this, cx| {
                    this.run_unit(
                        move || DeviceRepo::lock_unlock_blocking(m),
                        "Unlocked — FIDO works until power-off.",
                        status,
                        cx,
                    );
                });
            })
        };
        Self::dialog_phrase_only(
            "Unlock Seed",
            "Loads the seed into RAM for this power cycle using the lock key.",
            phrase,
            ("Unlock", ButtonVariant::Primary),
            submit,
            window,
            cx,
        );
    }

    // ── Disable ─────────────────────────────────────────────────────────────

    pub(super) fn open_disable(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let pin = Self::pin_input(window, cx);
        let phrase = Self::phrase_input(window, cx);
        let view = cx.entity().downgrade();
        let submit = {
            let pin = pin.clone();
            let phrase = phrase.clone();
            std::rc::Rc::new(move |window: &mut Window, cx: &mut App| {
                let p = pin.read(cx).text().to_string();
                let m = phrase.read(cx).text().to_string();
                if p.is_empty() || m.trim().is_empty() {
                    return;
                }
                window.close_dialog(cx);
                let status = dialog::open_status_dialog("Disabling Lock", window, cx);
                let _ = view.update(cx, |this, cx| {
                    this.run_unit(
                        move || DeviceRepo::lock_disable_blocking(p, m),
                        "Lock disabled — plaintext seed restored.",
                        status,
                        cx,
                    );
                });
            })
        };
        Self::dialog(
            "Disable At-Rest Lock",
            "Restores the plaintext seed so FIDO works without an unlock. Needs the lock key and the FIDO PIN; touch to confirm.",
            Some(("Lock key (24 words)", phrase)),
            pin,
            ("Disable lock", ButtonVariant::Primary),
            submit,
            window,
            cx,
        );
    }

    fn run_unit(
        &mut self,
        op: impl FnOnce() -> Result<(), String> + Send + 'static,
        ok_msg: &'static str,
        status: WeakEntity<StatusContent>,
        cx: &mut Context<Self>,
    ) {
        if self.loading {
            return;
        }
        self.loading = true;
        let _ = status.update(cx, |d, cx| {
            d.set_loading("Working… touch the device (BOOTSEL) if it blinks.", cx)
        });
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
                        let _ = status.update(cx, |d, cx| d.set_error(e, cx));
                    }
                }
                cx.notify();
            });
        }));
    }

    /// Dialog with an optional text field above a required PIN field.
    #[allow(clippy::too_many_arguments)]
    fn dialog(
        title: &'static str,
        body: &'static str,
        extra: Option<(&'static str, Entity<InputState>)>,
        pin: Entity<InputState>,
        action: (&'static str, ButtonVariant),
        submit: DialogSubmit,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.open_dialog(cx, move |dialog, _w, _| {
            let pin = pin.clone();
            let extra = extra.clone();
            let ok = submit.clone();
            let btn = submit.clone();
            let (action_label, action_variant) = action;
            let mut fields = gpui_kit::component::v_flex().gap_3().pb_2();
            if let Some((label, input)) = &extra {
                fields = fields
                    .child(label.to_string())
                    .child(gpui_kit::component::input::Input::new(input));
            }
            fields = fields
                .child("FIDO PIN")
                .child(gpui_kit::component::input::Input::new(&pin));
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
                        gpui_kit::component::button::Button::new("go")
                            .with_variant(action_variant)
                            .label(action_label)
                            .on_click(move |_, window, cx| s(window, cx)),
                    ])
                })
        });
    }

    /// Dialog with a single text field (unlock — no PIN).
    fn dialog_phrase_only(
        title: &'static str,
        body: &'static str,
        phrase: Entity<InputState>,
        action: (&'static str, ButtonVariant),
        submit: DialogSubmit,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.open_dialog(cx, move |dialog, _w, _| {
            let phrase = phrase.clone();
            let ok = submit.clone();
            let btn = submit.clone();
            let (action_label, action_variant) = action;
            dialog
                .title(title)
                .child(body)
                .child(
                    gpui_kit::component::v_flex()
                        .gap_2()
                        .pb_2()
                        .child("Lock key (24 words)")
                        .child(gpui_kit::component::input::Input::new(&phrase)),
                )
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
                        gpui_kit::component::button::Button::new("go")
                            .with_variant(action_variant)
                            .label(action_label)
                            .on_click(move |_, window, cx| s(window, cx)),
                    ])
                })
        });
    }
}
