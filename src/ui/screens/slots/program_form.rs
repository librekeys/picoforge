//! The slot-programming dialog — a reactive form whose fields change with the
//! selected credential type, mirroring Yubico Authenticator's OTP editor.
//!
//! Each type shows only its own inputs and options: challenge-response has a
//! secret + touch; OATH-HOTP a secret + digits + append; static a password;
//! Yubico OTP the public/private/secret triple. A subscription to the type
//! dropdown re-renders the form on change. The slot access code is optional and
//! shown for every type.

use crate::error::PFError;
use crate::ui::components::dialog;
use crate::ui::components::form::{LabeledU8, select_state, selected_key};
use crate::ui::models::device::{DeviceRepo, otp};
use crate::ui::screens::slots::view_model::{SlotsEvent, SlotsViewModel};
use gpui::*;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::input::{Input, InputState};
use gpui_component::select::{Select, SelectEvent, SelectState};
use gpui_component::{WindowExt, h_flex, v_flex};

const OPT_TYPE: &[(&str, u8)] = &[
    ("Challenge-response", 0),
    ("OATH-HOTP", 1),
    ("Static password", 2),
    ("Yubico OTP", 3),
];
const OPT_TOUCH: &[(&str, u8)] = &[("Not required", 0), ("Touch required", 1)];
const OPT_DIGITS: &[(&str, u8)] = &[("6 digits", 6), ("8 digits", 8)];
const OPT_APPEND: &[(&str, u8)] = &[("No", 0), ("Append Enter", 1)];

/// Parse an optional hex access code into a fixed 6-byte code (zeros if empty).
pub(super) fn parse_acc(hex_str: &str) -> Option<[u8; 6]> {
    let t = hex_str.trim();
    if t.is_empty() {
        return Some([0u8; 6]);
    }
    let bytes = hex::decode(t).ok()?;
    if bytes.len() > 6 {
        return None;
    }
    let mut acc = [0u8; 6];
    acc[..bytes.len()].copy_from_slice(&bytes);
    Some(acc)
}

/// A reactive slot-programming form embedded in the dialog. Owns every input and
/// dropdown; re-renders when the credential type changes.
pub struct ProgramSlotForm {
    slot: u8,
    view: WeakEntity<SlotsViewModel>,
    type_sel: Entity<SelectState<Vec<LabeledU8>>>,
    touch_sel: Entity<SelectState<Vec<LabeledU8>>>,
    digits_sel: Entity<SelectState<Vec<LabeledU8>>>,
    append_sel: Entity<SelectState<Vec<LabeledU8>>>,
    secret: Entity<InputState>,
    password: Entity<InputState>,
    yk_public: Entity<InputState>,
    yk_private: Entity<InputState>,
    yk_key: Entity<InputState>,
    new_acc: Entity<InputState>,
    cur_acc: Entity<InputState>,
    _sub: Subscription,
}

/// Build the form and open the programming dialog for `slot`.
pub(super) fn open(slot: u8, window: &mut Window, cx: &mut Context<SlotsViewModel>) {
    let type_sel = select_state(window, cx, OPT_TYPE, 0);
    let touch_sel = select_state(window, cx, OPT_TOUCH, 0);
    let digits_sel = select_state(window, cx, OPT_DIGITS, 0);
    let append_sel = select_state(window, cx, OPT_APPEND, 0);
    let input = |ph: &str, window: &mut Window, cx: &mut Context<SlotsViewModel>| {
        let ph = ph.to_string();
        cx.new(|cx| InputState::new(window, cx).placeholder(ph))
    };
    let secret = input(crate::tr!("Secret key (hex)"), window, cx);
    let password = input(crate::tr!("Password to type (ASCII)"), window, cx);
    let yk_public = input(crate::tr!("Public ID (modhex)"), window, cx);
    let yk_private = input(crate::tr!("Private ID (hex)"), window, cx);
    let yk_key = input(crate::tr!("Secret key (hex)"), window, cx);
    let new_acc = input(crate::tr!("Set an access code (hex, optional)"), window, cx);
    let cur_acc = input(crate::tr!("Current access code, if protected"), window, cx);
    let view = cx.entity().downgrade();

    let form = cx.new(|cx| {
        let sub = cx.subscribe(
            &type_sel,
            |_this: &mut ProgramSlotForm, _, _e: &SelectEvent<Vec<LabeledU8>>, cx| cx.notify(),
        );
        ProgramSlotForm {
            slot,
            view,
            type_sel,
            touch_sel,
            digits_sel,
            append_sel,
            secret,
            password,
            yk_public,
            yk_private,
            yk_key,
            new_acc,
            cur_acc,
            _sub: sub,
        }
    });

    window.open_dialog(cx, move |dialog, _w, _| {
        let body = form.clone();
        let footer_form = form.clone();
        dialog
            .title(crate::tr!("Program Slot {slot}", slot = slot))
            .child(body)
            .footer(move |_, _w, _c, _| {
                let f = footer_form.clone();
                vec![
                    Button::new(crate::tr!("cancel"))
                        .label(crate::tr!("Cancel"))
                        .on_click(|_, window, cx| window.close_dialog(cx)),
                    Button::new(crate::tr!("program"))
                        .primary()
                        .label(crate::tr!("Program"))
                        .on_click(move |_, window, cx| {
                            let f = f.clone();
                            f.update(cx, |f, cx| f.submit(window, cx));
                        }),
                ]
            })
    });
}

impl ProgramSlotForm {
    fn notify(&self, cx: &mut Context<Self>, msg: &str) {
        let _ = self.view.update(cx, |_, cx| {
            cx.emit(SlotsEvent::Notification(msg.to_string()))
        });
    }

    /// Close the form dialog, show a status dialog, and run the program op.
    fn dispatch(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
        op: impl FnOnce() -> Result<(), PFError> + Send + 'static,
        msg: String,
    ) {
        window.close_dialog(cx);
        let status = dialog::open_status_dialog(crate::tr!("Programming Slot"), window, cx);
        let _ = self
            .view
            .update(cx, |vm, cx| vm.execute_program(op, msg, status, cx));
    }

    fn submit(&self, window: &mut Window, cx: &mut Context<Self>) {
        let ty = selected_key(&self.type_sel, OPT_TYPE, cx);
        let append_cr = selected_key(&self.append_sel, OPT_APPEND, cx) == 1;
        let touch = selected_key(&self.touch_sel, OPT_TOUCH, cx) == 1;
        let digits8 = selected_key(&self.digits_sel, OPT_DIGITS, cx) == 8;

        let (new_a, cur_a) = match (
            parse_acc(&self.new_acc.read(cx).text().to_string()),
            parse_acc(&self.cur_acc.read(cx).text().to_string()),
        ) {
            (Some(n), Some(c)) => (n, c),
            _ => return self.notify(cx, crate::tr!("Access codes must be hex, ≤ 6 bytes")),
        };
        let slot = self.slot;
        let hex_secret = |s: &Entity<InputState>, cx: &mut Context<Self>| match hex::decode(
            s.read(cx).text().to_string().trim(),
        ) {
            Ok(b) if !b.is_empty() && b.len() <= 20 => Some(b),
            _ => None,
        };

        match ty {
            0 => {
                let Some(bytes) = hex_secret(&self.secret, cx) else {
                    return self.notify(cx, crate::tr!("Secret must be 1–20 bytes of hex"));
                };
                self.dispatch(
                    window,
                    cx,
                    move || {
                        DeviceRepo::otp_program_chalresp_blocking(slot, bytes, touch, new_a, cur_a)
                    },
                    crate::tr!("Challenge-response programmed.").into(),
                );
            }
            1 => {
                let Some(bytes) = hex_secret(&self.secret, cx) else {
                    return self.notify(cx, crate::tr!("Secret must be 1–20 bytes of hex"));
                };
                self.dispatch(
                    window,
                    cx,
                    move || {
                        DeviceRepo::otp_program_hotp_blocking(
                            slot, bytes, digits8, append_cr, new_a, cur_a,
                        )
                    },
                    crate::tr!("OATH-HOTP programmed.").into(),
                );
            }
            2 => {
                let scancodes =
                    match otp::ascii_to_scancodes(self.password.read(cx).text().to_string().trim())
                    {
                        Some(s) if !s.is_empty() => s,
                        _ => {
                            return self
                                .notify(cx, crate::tr!("Password must be ASCII, 1–38 characters"));
                        }
                    };
                self.dispatch(
                    window,
                    cx,
                    move || {
                        DeviceRepo::otp_program_static_blocking(
                            slot, scancodes, append_cr, new_a, cur_a,
                        )
                    },
                    crate::tr!("Static password programmed.").into(),
                );
            }
            _ => {
                let public =
                    match otp::modhex_decode(self.yk_public.read(cx).text().to_string().trim()) {
                        Some(p) if !p.is_empty() && p.len() <= 16 => p,
                        _ => {
                            return self.notify(
                                cx,
                                crate::tr!("Public ID must be modhex (≤ 16 bytes) — use Generate"),
                            );
                        }
                    };
                let private: [u8; 6] =
                    match hex::decode(self.yk_private.read(cx).text().to_string().trim()) {
                        Ok(b) if b.len() == 6 => b.try_into().unwrap(),
                        _ => {
                            return self.notify(
                                cx,
                                crate::tr!("Private ID must be 6 bytes of hex — use Generate"),
                            );
                        }
                    };
                let key: [u8; 16] =
                    match hex::decode(self.yk_key.read(cx).text().to_string().trim()) {
                        Ok(b) if b.len() == 16 => b.try_into().unwrap(),
                        _ => {
                            return self.notify(
                                cx,
                                crate::tr!("Secret key must be 16 bytes of hex — use Generate"),
                            );
                        }
                    };
                let msg = crate::tr!(
                    "Yubico OTP programmed. Register with a validation server:\nPublic ID: {}\nPrivate ID: {}\nKey: {}",
                    otp::modhex_encode(&public),
                    hex::encode(private),
                    hex::encode(key),
                );
                self.dispatch(
                    window,
                    cx,
                    move || {
                        DeviceRepo::otp_program_yubico_blocking(
                            slot, public, private, key, append_cr, new_a, cur_a,
                        )
                    },
                    msg,
                );
            }
        }
    }

    /// A labelled input with a Generate button that runs `on_gen`.
    fn input_with_generate(
        &self,
        label: &str,
        input: &Entity<InputState>,
        gen_id: &'static str,
        on_gen: impl Fn(&mut Window, &mut App) + 'static,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        v_flex()
            .gap_1()
            .child(label.to_string())
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(v_flex().flex_1().child(Input::new(input)))
                    .child(
                        Button::new(gen_id)
                            .label(crate::tr!("Generate"))
                            .outline()
                            .on_click(cx.listener(move |_, _, window, cx| on_gen(window, cx))),
                    ),
            )
            .into_any_element()
    }
}

fn labeled_select(label: &str, sel: &Entity<SelectState<Vec<LabeledU8>>>) -> AnyElement {
    v_flex()
        .gap_1()
        .flex_1()
        .child(label.to_string())
        .child(Select::new(sel).w_full().bg(rgb(0x222225)))
        .into_any_element()
}

fn labeled_input(label: &str, input: &Entity<InputState>) -> AnyElement {
    v_flex()
        .gap_1()
        .flex_1()
        .child(label.to_string())
        .child(Input::new(input))
        .into_any_element()
}

impl Render for ProgramSlotForm {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let ty = selected_key(&self.type_sel, OPT_TYPE, cx);

        // Fields specific to the selected credential type.
        let type_fields = match ty {
            0 => v_flex()
                .gap_3()
                .child(self.input_with_generate(
                    crate::tr!("Secret key (hex, ≤ 20 bytes)"),
                    &self.secret,
                    "gen-secret",
                    {
                        let secret = self.secret.clone();
                        move |window, cx| {
                            if let Ok(s) = otp::random_secret() {
                                secret
                                    .update(cx, |st, cx| st.set_value(hex::encode(s), window, cx));
                            }
                        }
                    },
                    cx,
                ))
                .child(labeled_select(crate::tr!("Touch"), &self.touch_sel))
                .into_any_element(),
            1 => v_flex()
                .gap_3()
                .child(self.input_with_generate(
                    crate::tr!("Secret key (hex, ≤ 20 bytes)"),
                    &self.secret,
                    "gen-secret",
                    {
                        let secret = self.secret.clone();
                        move |window, cx| {
                            if let Ok(s) = otp::random_secret() {
                                secret
                                    .update(cx, |st, cx| st.set_value(hex::encode(s), window, cx));
                            }
                        }
                    },
                    cx,
                ))
                .child(
                    h_flex()
                        .gap_3()
                        .child(labeled_select(crate::tr!("Digits"), &self.digits_sel))
                        .child(labeled_select(crate::tr!("Append Enter"), &self.append_sel)),
                )
                .into_any_element(),
            2 => v_flex()
                .gap_3()
                .child(self.input_with_generate(
                    crate::tr!("Password (ASCII, 1–38 characters)"),
                    &self.password,
                    "gen-password",
                    {
                        let password = self.password.clone();
                        move |window, cx| {
                            if let Ok(s) = otp::random_secret() {
                                password.update(cx, |st, cx| {
                                    st.set_value(otp::modhex_encode(&s[..8]), window, cx)
                                });
                            }
                        }
                    },
                    cx,
                ))
                .child(labeled_select(crate::tr!("Append Enter"), &self.append_sel))
                .into_any_element(),
            _ => v_flex()
                .gap_3()
                .child(labeled_input(
                    crate::tr!("Public ID (modhex)"),
                    &self.yk_public,
                ))
                .child(labeled_input(
                    crate::tr!("Private ID (hex, 6 bytes)"),
                    &self.yk_private,
                ))
                .child(labeled_input(
                    crate::tr!("Secret key (hex, 16 bytes)"),
                    &self.yk_key,
                ))
                .child(
                    h_flex()
                        .justify_between()
                        .items_center()
                        .child(labeled_select(crate::tr!("Append Enter"), &self.append_sel)),
                )
                .child(
                    Button::new("gen-yubico")
                        .label(crate::tr!("Generate keys"))
                        .outline()
                        .on_click(cx.listener({
                            let (pubf, privf, keyf) = (
                                self.yk_public.clone(),
                                self.yk_private.clone(),
                                self.yk_key.clone(),
                            );
                            move |_, _, window, cx| {
                                if let Ok((p, pv, k)) = otp::random_yubico() {
                                    pubf.update(cx, |st, cx| {
                                        st.set_value(otp::modhex_encode(&p), window, cx)
                                    });
                                    privf.update(cx, |st, cx| {
                                        st.set_value(hex::encode(pv), window, cx)
                                    });
                                    keyf.update(cx, |st, cx| {
                                        st.set_value(hex::encode(k), window, cx)
                                    });
                                }
                            }
                        })),
                )
                .into_any_element(),
        };

        v_flex()
            .gap_3()
            .pb_2()
            .child(labeled_select(
                crate::tr!("Credential type"),
                &self.type_sel,
            ))
            .child(type_fields)
            .child(
                v_flex()
                    .gap_1()
                    .pt_2()
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0x8b8b8f))
                            .child(crate::tr!("Slot access code (optional)")),
                    )
                    .child(
                        h_flex()
                            .gap_3()
                            .child(labeled_input(crate::tr!("Set new code"), &self.new_acc))
                            .child(labeled_input(crate::tr!("Current code"), &self.cur_acc)),
                    ),
            )
    }
}
