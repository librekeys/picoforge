//! View model for the Offboard screen — guided full-device wipe + signed receipt.

use crate::ui::app::AppModels;
use crate::ui::components::applet_gate::AppletGate;
use crate::ui::components::dialog;
use crate::ui::components::dialog::StatusContent;
use crate::ui::models::device::{DeviceEvent, DeviceRepo, FirmwareType, OffboardReport};
use gpui::*;
use gpui_component::WindowExt;
use gpui_component::button::ButtonVariants;
use gpui_component::input::InputState;

pub struct OffboardViewModel {
    pub(super) device: Entity<DeviceRepo>,
    pub(super) report: Option<OffboardReport>,
    pub(super) loading: bool,
    _task: Option<Task<()>>,
}

pub enum OffboardEvent {
    Notification(String),
}

impl EventEmitter<OffboardEvent> for OffboardViewModel {}

impl OffboardViewModel {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>, models: &AppModels) -> Self {
        let device = models.device.clone();
        cx.subscribe(&device, |this: &mut Self, _, _: &DeviceEvent, cx| {
            if this.device.read(cx).device_changed {
                this.report = None;
                cx.notify();
            }
        })
        .detach();
        Self {
            device,
            report: None,
            loading: false,
            _task: None,
        }
    }

    pub(super) fn gate(&self, cx: &App) -> AppletGate {
        let repo = self.device.read(cx);
        match &repo.status {
            None => AppletGate::Unsupported,
            Some(s) if s.firmware_type != FirmwareType::RSKey => AppletGate::Unsupported,
            Some(_) => AppletGate::Ready,
        }
    }

    pub(super) fn serial(&self, cx: &App) -> String {
        self.device
            .read(cx)
            .status
            .as_ref()
            .map(|s| s.info.serial.clone())
            .unwrap_or_default()
    }

    // ── Confirm + run ───────────────────────────────────────────────────────

    pub(super) fn open_confirm(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let serial = self.serial(cx);
        let confirm = cx.new(|cx| {
            InputState::new(window, cx).placeholder(crate::tr!("Type OFFBOARD to confirm"))
        });
        let view = cx.entity().downgrade();
        let submit = {
            let confirm = confirm.clone();
            let serial = serial.clone();
            std::rc::Rc::new(move |window: &mut Window, cx: &mut App| {
                if confirm.read(cx).text().to_string().trim() != "OFFBOARD" {
                    let _ = view.update(cx, |_, cx| {
                        cx.emit(OffboardEvent::Notification(
                            crate::tr!("Type OFFBOARD exactly to confirm").into(),
                        ))
                    });
                    return;
                }
                window.close_dialog(cx);
                let status = dialog::open_status_dialog(crate::tr!("Offboarding"), window, cx);
                let serial = serial.clone();
                let _ = view.update(cx, |this, cx| this.run_offboard(serial, status, cx));
            })
        };
        window.open_dialog(cx, move |dialog, _w, _| {
            let confirm = confirm.clone();
            let ok = submit.clone();
            let btn = submit.clone();
            let serial = serial.clone();
            dialog
                .title(crate::tr!("Offboard Device"))
                .child(crate::tr!(
                    "This ERASES everything on device {serial}: OTP slots, OATH, PIV, OpenPGP, the FIDO seed, passkeys, PINs, and org attestation — then writes a signed wipe receipt. It cannot be undone and needs several touches."
                , serial = serial))
                .child(
                    gpui_component::v_flex()
                        .gap_2()
                        .pb_2()
                        .child(crate::tr!("Confirmation"))
                        .child(gpui_component::input::Input::new(&confirm)),
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
                        gpui_component::button::Button::new(crate::tr!("go"))
                            .danger()
                            .label(crate::tr!("Offboard"))
                            .on_click(move |_, window, cx| s(window, cx)),
                    ]
                })
        });
    }

    fn run_offboard(
        &mut self,
        serial: String,
        status: WeakEntity<StatusContent>,
        cx: &mut Context<Self>,
    ) {
        if self.loading {
            return;
        }
        self.loading = true;
        let _ = status.update(cx, |d, cx| {
            d.set_loading(
                crate::tr!("Wiping… touch the device (BOOTSEL) when it blinks (several times)."),
                cx,
            )
        });
        cx.notify();
        let weak = cx.entity().downgrade();
        self._task = Some(cx.spawn(async move |_, cx| {
            let res = cx
                .background_executor()
                .spawn(async move { DeviceRepo::offboard_blocking(serial) })
                .await;
            let _ = weak.update(cx, |this, cx| {
                this.loading = false;
                match res {
                    Ok(report) => {
                        let msg = if report.all_ok() {
                            if report.signed {
                                crate::tr!("Offboarded — all applets wiped, receipt signed.").to_string()
                            } else {
                                crate::tr!("Offboarded — all applets wiped (receipt UNSIGNED: no OTP DEVK).")
                                    .to_string()
                            }
                        } else {
                            crate::tr!(
                                "Offboard finished WITH FAILURES: {}",
                                format!("{:?}", report.failures())
                            )
                        };
                        this.report = Some(report);
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

    // ── Save receipt ────────────────────────────────────────────────────────

    pub(super) fn save_receipt(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let Some(report) = self.report.clone() else {
            return;
        };
        let default_dir = std::env::var(crate::tr!("HOME"))
            .map(std::path::PathBuf::from)
            .unwrap_or_default();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let json = report.to_json(&iso_utc(now));
        let receiver = cx.prompt_for_new_path(
            &default_dir,
            Some(&crate::tr!("offboard-{}.json", report.serial)),
        );
        let view = cx.entity().downgrade();
        self._task = Some(cx.spawn(async move |_, cx| {
            let Ok(Ok(Some(path))) = receiver.await else {
                return;
            };
            let _ = view.update(cx, |_, cx| match std::fs::write(&path, json) {
                Ok(_) => cx.emit(OffboardEvent::Notification(crate::tr!(
                    "Receipt saved to {}",
                    path.display()
                ))),
                Err(e) => cx.emit(OffboardEvent::Notification(crate::tr!(
                    "Save failed: {e}",
                    e = e
                ))),
            });
        }));
    }
}

/// Format epoch seconds as an ISO-8601 UTC timestamp (civil date, Hinnant).
fn iso_utc(secs: u64) -> String {
    let days = (secs / 86400) as i64;
    let rem = secs % 86400;
    let (h, mi, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    crate::tr!(
        "{y:04}-{m:02}-{d:02}T{h:02}:{mi:02}:{s:02}Z",
        y = y,
        m = m,
        d = d,
        h = h,
        mi = mi,
        s = s
    )
}
