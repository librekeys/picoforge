//! View model for the Slots (OTP) screen — the configurable YubiKey slots.

use crate::error::PFError;
use crate::ui::app::AppModels;
use crate::ui::components::applet_gate::AppletGate;
use crate::ui::components::dialog;
use crate::ui::components::dialog::StatusContent;
use crate::ui::models::device::{DeviceEvent, DeviceRepo, USB_CAP_OTP, otp};
use gpui::*;
use gpui_component::WindowExt;
use gpui_component::button::ButtonVariants;

/// Slots screen state and OTP slot operations.
pub struct SlotsViewModel {
    pub(super) device: Entity<DeviceRepo>,
    pub(super) slots: Vec<otp::SlotInfo>,
    pub(super) loaded: bool,
    pub(super) loading: bool,
    _task: Option<Task<()>>,
}

pub enum SlotsEvent {
    Notification(String),
}

impl EventEmitter<SlotsEvent> for SlotsViewModel {}

/// Read the delete/swap dialog's access-code input, or emit a validation toast
/// and return `None`. An empty field is a valid all-zero code (unprotected slot).
fn current_acc(
    input: &Entity<gpui_component::input::InputState>,
    view: &WeakEntity<SlotsViewModel>,
    cx: &mut App,
) -> Option<[u8; 6]> {
    match super::program_form::parse_acc(&input.read(cx).text().to_string()) {
        Some(acc) => Some(acc),
        None => {
            let _ = view.update(cx, |_, cx| {
                cx.emit(SlotsEvent::Notification(
                    crate::tr!("Access code must be empty or up to 12 hex chars (6 bytes)").into(),
                ));
            });
            None
        }
    }
}

impl SlotsViewModel {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>, models: &AppModels) -> Self {
        let device = models.device.clone();
        cx.subscribe(&device, |this: &mut Self, _, _: &DeviceEvent, cx| {
            this.on_device_event(cx);
        })
        .detach();
        let mut this = Self {
            device,
            slots: Vec::new(),
            loaded: false,
            loading: false,
            _task: None,
        };
        this.load(cx);
        this
    }

    fn on_device_event(&mut self, cx: &mut Context<Self>) {
        if self.device.read(cx).device_changed {
            self.slots.clear();
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
        match repo.otp_features() {
            None => AppletGate::Unsupported,
            Some(_) if !repo.ccid_on() => AppletGate::CcidOff,
            Some(_) if !repo.applet_enabled(USB_CAP_OTP) => AppletGate::Disabled(crate::tr!("OTP")),
            Some(_) => AppletGate::Ready,
        }
    }

    /// Number of slots the firmware exposes (2 classic, 4 for RS-Key).
    pub(super) fn slot_count(&self, cx: &App) -> u8 {
        self.device
            .read(cx)
            .otp_features()
            .map(|f| f.slots)
            .unwrap_or(2)
    }

    pub(super) fn slot_info(&self, slot: u8) -> otp::SlotInfo {
        self.slots
            .iter()
            .find(|s| s.slot == slot)
            .copied()
            .unwrap_or(otp::SlotInfo {
                slot,
                kind: otp::SlotType::Empty,
                touch: false,
            })
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
                .spawn(async { DeviceRepo::otp_read_info_blocking() })
                .await;
            let _ = weak.update(cx, |this, cx| this.apply(res, cx));
        }));
    }

    fn apply(&mut self, res: Result<[otp::SlotInfo; 4], PFError>, cx: &mut Context<Self>) {
        self.loading = false;
        match res {
            Ok(slots) => {
                self.slots = slots.to_vec();
                self.loaded = true;
            }
            Err(e) => {
                log::warn!("{}", crate::tr!("OTP status read failed: {e}", e = e));
                cx.emit(SlotsEvent::Notification(crate::tr!("Slots: {e}", e = e)));
            }
        }
        cx.notify();
    }

    pub(super) fn refresh(&mut self, cx: &mut Context<Self>) {
        self.load(cx);
    }

    pub(super) fn open_swap_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let acc_input = cx.new(|cx| {
            gpui_component::input::InputState::new(window, cx)
                .placeholder(crate::tr!("Access code (hex, if a slot is protected)"))
        });
        let view = cx.entity().downgrade();
        let submit = {
            let acc_input = acc_input.clone();
            let view = view.clone();
            std::rc::Rc::new(move |window: &mut Window, cx: &mut App| {
                let Some(acc) = current_acc(&acc_input, &view, cx) else {
                    return;
                };
                window.close_dialog(cx);
                let status = dialog::open_status_dialog(crate::tr!("Swap Slots"), window, cx);
                let _ = view.update(cx, |this, cx| this.execute_swap(acc, status, cx));
            })
        };
        window.open_dialog(cx, move |dialog, _window, _| {
            let acc_input = acc_input.clone();
            let submit_ok = submit.clone();
            let submit_btn = submit.clone();
            dialog
                .title(crate::tr!("Swap Slots"))
                .child(crate::tr!("Swap the contents of slot 1 and slot 2?"))
                .child(
                    gpui_component::v_flex()
                        .gap_2()
                        .pb_2()
                        .child(crate::tr!("Access code (hex, leave empty if unprotected)"))
                        .child(gpui_component::input::Input::new(&acc_input)),
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
                        gpui_component::button::Button::new(crate::tr!("swap"))
                            .primary()
                            .label(crate::tr!("Swap"))
                            .on_click(move |_, window, cx| submit(window, cx)),
                    ]
                })
        });
    }

    fn execute_swap(
        &mut self,
        acc: [u8; 6],
        status: WeakEntity<StatusContent>,
        cx: &mut Context<Self>,
    ) {
        self.run_op(
            cx,
            move |_| DeviceRepo::otp_swap_blocking(acc),
            crate::tr!("Slots swapped."),
            move |cx, msg, ok| {
                let _ = status.update(cx, |d, cx| {
                    if ok {
                        d.set_success(msg, cx)
                    } else {
                        d.set_error(msg, cx)
                    }
                });
            },
        );
    }

    pub(super) fn open_delete_dialog(
        &mut self,
        slot: u8,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let acc_input = cx.new(|cx| {
            gpui_component::input::InputState::new(window, cx)
                .placeholder(crate::tr!("Access code (hex, if the slot is protected)"))
        });
        let view = cx.entity().downgrade();
        let submit = {
            let acc_input = acc_input.clone();
            let view = view.clone();
            std::rc::Rc::new(move |window: &mut Window, cx: &mut App| {
                let Some(acc) = current_acc(&acc_input, &view, cx) else {
                    return;
                };
                window.close_dialog(cx);
                let status = dialog::open_status_dialog(crate::tr!("Delete Slot"), window, cx);
                let _ = view.update(cx, |this, cx| this.execute_delete(slot, acc, status, cx));
            })
        };
        window.open_dialog(cx, move |dialog, _window, _| {
            let acc_input = acc_input.clone();
            let submit_ok = submit.clone();
            let submit_btn = submit.clone();
            dialog
                .title(crate::tr!("Delete Slot {slot}", slot = slot))
                .child(crate::tr!(
                    "Erase the configuration in slot {slot}? This cannot be undone.",
                    slot = slot
                ))
                .child(
                    gpui_component::v_flex()
                        .gap_2()
                        .pb_2()
                        .child(crate::tr!("Access code (hex, leave empty if unprotected)"))
                        .child(gpui_component::input::Input::new(&acc_input)),
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
                        gpui_component::button::Button::new(crate::tr!("delete"))
                            .danger()
                            .label(crate::tr!("Delete"))
                            .on_click(move |_, window, cx| submit(window, cx)),
                    ]
                })
        });
    }

    fn execute_delete(
        &mut self,
        slot: u8,
        acc: [u8; 6],
        status: WeakEntity<StatusContent>,
        cx: &mut Context<Self>,
    ) {
        self.run_op(
            cx,
            move |_| DeviceRepo::otp_delete_blocking(slot, acc),
            crate::tr!("Slot deleted."),
            move |cx, msg, ok| {
                let _ = status.update(cx, |d, cx| {
                    if ok {
                        d.set_success(msg, cx)
                    } else {
                        d.set_error(msg, cx)
                    }
                });
            },
        );
    }

    pub(super) fn open_program_dialog(
        &mut self,
        slot: u8,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        super::program_form::open(slot, window, cx);
    }

    pub(super) fn execute_program(
        &mut self,
        op: impl FnOnce() -> Result<(), PFError> + Send + 'static,
        ok_msg: String,
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
                        let _ = status.update(cx, |d, cx| d.set_success(ok_msg, cx));
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

    pub(super) fn open_test_dialog(
        &mut self,
        slot: u8,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let challenge = cx.new(|cx| {
            gpui_component::input::InputState::new(window, cx)
                .placeholder(crate::tr!("Challenge (hex)"))
        });
        let view = cx.entity().downgrade();
        let submit = {
            let challenge = challenge.clone();
            let view = view.clone();
            std::rc::Rc::new(move |window: &mut Window, cx: &mut App| {
                let chal = match hex::decode(challenge.read(cx).text().to_string().trim()) {
                    Ok(b) if !b.is_empty() => b,
                    _ => {
                        let _ = view.update(cx, |_, cx| {
                            cx.emit(SlotsEvent::Notification(
                                crate::tr!("Enter a valid hex challenge").into(),
                            ));
                        });
                        return;
                    }
                };
                window.close_dialog(cx);
                let status =
                    dialog::open_status_dialog(crate::tr!("Challenge-Response"), window, cx);
                let _ = view.update(cx, |this, cx| this.execute_test(slot, chal, status, cx));
            })
        };
        window.open_dialog(cx, move |dialog, _window, _| {
            let challenge = challenge.clone();
            let submit_ok = submit.clone();
            let submit_btn = submit.clone();
            dialog
                .title(crate::tr!("Test Slot {slot}", slot = slot))
                .child(crate::tr!(
                    "Send a challenge; the slot answers with its HMAC-SHA1 response."
                ))
                .child(
                    gpui_component::v_flex()
                        .gap_2()
                        .pb_2()
                        .child(crate::tr!("Challenge (hex)"))
                        .child(gpui_component::input::Input::new(&challenge)),
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
                        gpui_component::button::Button::new(crate::tr!("run"))
                            .primary()
                            .label(crate::tr!("Run"))
                            .on_click(move |_, window, cx| submit(window, cx)),
                    ]
                })
        });
    }

    fn execute_test(
        &mut self,
        slot: u8,
        challenge: Vec<u8>,
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
            let res = cx
                .background_executor()
                .spawn(async move { DeviceRepo::otp_calculate_blocking(slot, challenge) })
                .await;
            let _ = weak.update(cx, |this, cx| {
                this.loading = false;
                match res {
                    Ok(resp) => {
                        let _ = status.update(cx, |d, cx| {
                            d.set_success(crate::tr!("Response: {}", hex::encode(resp)), cx)
                        });
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

    /// Shared plumbing: run a blocking OTP op off-thread, report via `finish`,
    /// and reload slot status on success.
    fn run_op(
        &mut self,
        cx: &mut Context<Self>,
        op: impl FnOnce(()) -> Result<(), PFError> + Send + 'static,
        ok_msg: &'static str,
        finish: impl FnOnce(&mut Context<Self>, String, bool) + 'static,
    ) {
        if self.loading {
            return;
        }
        self.loading = true;
        cx.notify();
        let weak = cx.entity().downgrade();
        self._task = Some(cx.spawn(async move |_, cx| {
            let res = cx.background_executor().spawn(async move { op(()) }).await;
            let _ = weak.update(cx, |this, cx| {
                this.loading = false;
                match res {
                    Ok(_) => {
                        finish(cx, ok_msg.to_string(), true);
                        this.load(cx);
                    }
                    Err(e) => finish(cx, crate::tr!("{e}", e = e), false),
                }
                cx.notify();
            });
        }));
    }
}
