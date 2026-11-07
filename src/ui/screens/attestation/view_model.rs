//! View model for the Attestation screen — org (enterprise) attestation key +
//! certificate chain provisioning.

use crate::ui::app::AppModels;
use crate::ui::components::applet_gate::AppletGate;
use crate::ui::components::dialog;
use crate::ui::components::dialog::StatusContent;
use crate::ui::models::device::{AttStatus, DeviceEvent, DeviceRepo, FirmwareType};
use gpui_kit::component::WindowExt;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::component::input::InputState;
use gpui_kit::*;

pub struct AttestationViewModel {
    pub(super) device: Entity<DeviceRepo>,
    pub(super) status: Option<AttStatus>,
    pub(super) loading: bool,
    _task: Option<Task<()>>,
}

pub enum AttestationEvent {
    Notification(String),
}

impl EventEmitter<AttestationEvent> for AttestationViewModel {}

impl AttestationViewModel {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>, models: &AppModels) -> Self {
        let device = models.device.clone();
        cx.subscribe(&device, |this: &mut Self, _, _: &DeviceEvent, cx| {
            if this.device.read(cx).device_changed {
                this.status = None;
            }
            this.load(cx);
            cx.notify();
        })
        .detach();
        let mut this = Self {
            device,
            status: None,
            loading: false,
            _task: None,
        };
        this.load(cx);
        this
    }

    pub(super) fn gate(&self, cx: &App) -> AppletGate {
        let repo = self.device.read(cx);
        match &repo.status {
            None => AppletGate::Unsupported,
            Some(s) if s.firmware_type != FirmwareType::RSKey => AppletGate::Unsupported,
            Some(_) => AppletGate::Ready,
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
                .spawn(async { DeviceRepo::att_status_blocking() })
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

    fn pin_input(window: &mut Window, cx: &mut Context<Self>) -> Entity<InputState> {
        cx.new(|cx| {
            InputState::new(window, cx)
                .masked(true)
                .placeholder("FIDO PIN — leave blank to touch instead")
        })
    }

    // ── Import (key file → chain file → PIN → run) ───────────────────────────

    pub(super) fn open_import(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let handle = window.window_handle();
        let key_recv = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Select attestation P-256 private key (PEM/DER)".into()),
        });
        let view = cx.entity().downgrade();
        self._task = Some(cx.spawn(async move |_, cx| {
            let Ok(Ok(Some(kp))) = key_recv.await else {
                return;
            };
            let Some(key_path) = kp.into_iter().next() else {
                return;
            };
            let Ok(key_bytes) = std::fs::read(&key_path) else {
                let _ = view.update(cx, |_, cx| {
                    cx.emit(AttestationEvent::Notification("Could not read the key file".into()))
                });
                return;
            };
            // Now the chain file.
            let chain_recv = cx.update_window(handle, |_, _window, cx| {
                cx.prompt_for_paths(PathPromptOptions {
                    files: true,
                    directories: false,
                    multiple: false,
                    prompt: Some("Select certificate chain, leaf first (PEM/DER)".into()),
                })
            });
            let Ok(chain_recv) = chain_recv else { return };
            let Ok(Ok(Some(cp))) = chain_recv.await else {
                return;
            };
            let Some(chain_path) = cp.into_iter().next() else {
                return;
            };
            let Ok(chain_bytes) = std::fs::read(&chain_path) else {
                let _ = view.update(cx, |_, cx| {
                    cx.emit(AttestationEvent::Notification("Could not read the chain file".into()))
                });
                return;
            };
            let _ = cx.update_window(handle, |_, window, cx| {
                let _ = view.update(cx, |this, cx| {
                    this.open_pin_dialog(
                        "Import Org Attestation",
                        "Installs the org attestation key and chain (P-256). Requires the FIDO PIN, or a touch if none is set.",
                        move |pin, this, window, cx| {
                            let status = dialog::open_status_dialog("Importing Attestation", window, cx);
                            let (kb, cb) = (key_bytes.clone(), chain_bytes.clone());
                            this.run_unit(
                                move || DeviceRepo::att_import_blocking(pin, kb, cb),
                                "Org attestation installed.",
                                status,
                                cx,
                            );
                        },
                        window,
                        cx,
                    );
                });
            });
        }));
    }

    // ── Clear ────────────────────────────────────────────────────────────────

    pub(super) fn open_clear(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.open_pin_dialog(
            "Remove Org Attestation",
            "Removes the org attestation and reverts to the self-signed device certificate. Requires the FIDO PIN, or a touch if none is set.",
            |pin, this, window, cx| {
                let status = dialog::open_status_dialog("Removing Attestation", window, cx);
                this.run_unit(
                    move || DeviceRepo::att_clear_blocking(pin),
                    "Org attestation removed.",
                    status,
                    cx,
                );
            },
            window,
            cx,
        );
    }

    /// An optional-PIN dialog that invokes `on_submit(pin, this, window, cx)`.
    fn open_pin_dialog(
        &mut self,
        title: &'static str,
        body: &'static str,
        on_submit: impl Fn(Option<String>, &mut Self, &mut Window, &mut Context<Self>) + 'static,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let pin = Self::pin_input(window, cx);
        let view = cx.entity().downgrade();
        let on_submit = std::rc::Rc::new(on_submit);
        let submit = {
            let pin = pin.clone();
            std::rc::Rc::new(move |window: &mut Window, cx: &mut App| {
                let p = pin.read(cx).text().to_string();
                let p = (!p.is_empty()).then_some(p);
                window.close_dialog(cx);
                let on_submit = on_submit.clone();
                let _ = view.update(cx, |this, cx| on_submit(p, this, window, cx));
            })
        };
        window.open_dialog(cx, move |dialog, _w, _| {
            let pin = pin.clone();
            let ok = submit.clone();
            let btn = submit.clone();
            dialog
                .title(title)
                .child(body)
                .child(
                    gpui_kit::component::v_flex()
                        .gap_2()
                        .pb_2()
                        .child("FIDO PIN")
                        .child(gpui_kit::component::input::Input::new(&pin)),
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
                            .primary()
                            .label("Run")
                            .on_click(move |_, window, cx| s(window, cx)),
                    ])
                })
        });
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
            d.set_loading("Working… touch the device (BOOTSEL).", cx)
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
}
