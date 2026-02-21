use gpui::*;
use gpui_component::{
    WindowExt,
    button::{Button, ButtonVariant, ButtonVariants},
    dialog::DialogButtonProps,
    input::{Input, InputState},
    v_flex,
};

pub fn open_pin_prompt(
    title: &str,
    description: &str,
    confirm_label: &str,
    window: &mut Window,
    cx: &mut App,
    on_confirm: impl Fn(String, &mut Window, &mut App) + 'static,
) {
    let title = SharedString::from(title.to_string());
    let description = SharedString::from(description.to_string());
    let confirm_label = SharedString::from(confirm_label.to_string());

    let pin_input = cx.new(|cx| {
        InputState::new(window, cx)
            .placeholder("Enter FIDO PIN")
            .masked(true)
    });

    let on_confirm = std::rc::Rc::new(on_confirm);

    window.open_dialog(cx, move |dialog, _, _| {
        let pin_input_for_footer = pin_input.clone();
        let confirm_label = confirm_label.clone();
        let on_confirm = on_confirm.clone();

        dialog
            .title(title.clone())
            .child(
                v_flex()
                    .gap_4()
                    .pb_4()
                    .child(description.clone())
                    .child(Input::new(&pin_input)),
            )
            .footer(move |_, _, _, _| {
                let input = pin_input_for_footer.clone();
                let on_confirm = on_confirm.clone();

                vec![
                    Button::new("cancel")
                        .label("Cancel")
                        .on_click(|_, window, cx| {
                            window.close_dialog(cx);
                        }),
                    Button::new("confirm")
                        .primary()
                        .label(confirm_label.clone())
                        .on_click(move |_, window, cx| {
                            let pin = input.read(cx).text().to_string();
                            if !pin.is_empty() {
                                window.close_dialog(cx);
                                on_confirm(pin, window, cx);
                            }
                        }),
                ]
            })
    });
}

pub fn open_confirm(
    title: &str,
    message: String,
    ok_label: &str,
    ok_variant: ButtonVariant,
    window: &mut Window,
    cx: &mut App,
    on_ok: impl Fn(&mut Window, &mut App) + 'static,
) {
    let title = SharedString::from(title.to_string());
    let ok_label = SharedString::from(ok_label.to_string());
    let on_ok = std::rc::Rc::new(on_ok);

    window.open_dialog(cx, move |dialog, _, _| {
        let on_ok = on_ok.clone();

        dialog
            .confirm()
            .title(title.clone())
            .child(div().pb_4().child(message.clone()))
            .on_ok(move |_, window, cx| {
                on_ok(window, cx);
                false
            })
            .on_cancel(|_, _, _| true)
            .button_props(
                DialogButtonProps::default()
                    .ok_text(ok_label.clone())
                    .ok_variant(ok_variant),
            )
    });
}

pub fn open_change_pin(
    window: &mut Window,
    cx: &mut App,
    on_error: impl Fn(&str, &mut App) + 'static + Clone,
    on_confirm: impl Fn(String, String, &mut App) + 'static,
) {
    let current_pin = cx.new(|cx| {
        InputState::new(window, cx)
            .placeholder("Enter current PIN")
            .masked(true)
    });
    let new_pin = cx.new(|cx| {
        InputState::new(window, cx)
            .placeholder("Enter new PIN")
            .masked(true)
    });
    let confirm_pin = cx.new(|cx| {
        InputState::new(window, cx)
            .placeholder("Confirm new PIN")
            .masked(true)
    });

    let on_confirm = std::rc::Rc::new(on_confirm);

    window.open_dialog(cx, move |dialog, _, _| {
        let current = current_pin.clone();
        let new = new_pin.clone();
        let confirm = confirm_pin.clone();
        let on_error = on_error.clone();
        let on_confirm = on_confirm.clone();

        dialog
            .title("Change PIN")
            .child("Enter your current PIN and choose a new one.")
            .child(
                v_flex()
                    .gap_4()
                    .pb_4()
                    .child("Current PIN")
                    .child(Input::new(&current))
                    .child("New PIN")
                    .child(Input::new(&new))
                    .child("Confirm New PIN")
                    .child(Input::new(&confirm)),
            )
            .footer(move |_, _window, _cx, _| {
                let current = current.clone();
                let new = new.clone();
                let confirm = confirm.clone();
                let on_error = on_error.clone();
                let on_confirm = on_confirm.clone();

                vec![
                    Button::new("cancel")
                        .label("Cancel")
                        .on_click(|_, window, cx| window.close_dialog(cx)),
                    Button::new("confirm")
                        .primary()
                        .label("Confirm")
                        .on_click(move |_, _, cx| {
                            let current_val = current.read(cx).text().to_string();
                            let new_val = new.read(cx).text().to_string();
                            let confirm_val = confirm.read(cx).text().to_string();

                            if current_val.is_empty() {
                                return;
                            }

                            if new_val != confirm_val {
                                on_error("PINs do not match", cx);
                                return;
                            }

                            if new_val.len() < 4 {
                                on_error("PIN too short", cx);
                                return;
                            }

                            on_confirm(current_val, new_val, cx);
                        }),
                ]
            })
    });
}
