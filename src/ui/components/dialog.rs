use gpui::*;
use gpui_component::{
    ActiveTheme, Disableable, Sizable, WindowExt,
    button::{Button, ButtonVariant, ButtonVariants},
    h_flex,
    input::{Input, InputEvent, InputState},
    v_flex,
};

#[derive(Clone)]
enum DialogPhase {
    Input,
    Loading,
    Success(String),
    Error(String),
}

pub struct PinPromptContent {
    phase: DialogPhase,
    title: SharedString,
    description: SharedString,
    confirm_label: SharedString,
    pin_input: Entity<InputState>,
    on_confirm: std::rc::Rc<dyn Fn(String, WeakEntity<PinPromptContent>, &mut App)>,
    _subscription: Subscription,
}

impl PinPromptContent {
    fn set_loading(&mut self, cx: &mut Context<Self>) {
        self.phase = DialogPhase::Loading;
        cx.notify();
    }

    pub fn set_success(&mut self, msg: String, cx: &mut Context<Self>) {
        self.phase = DialogPhase::Success(msg);
        cx.notify();
    }

    pub fn set_error(&mut self, msg: String, cx: &mut Context<Self>) {
        self.phase = DialogPhase::Error(msg);
        cx.notify();
    }

    fn trigger_confirm(&mut self, cx: &mut Context<Self>) {
        if matches!(self.phase, DialogPhase::Loading | DialogPhase::Success(_)) {
            return;
        }
        let pin = self.pin_input.read(cx).text().to_string();
        if !pin.is_empty() {
            let handle = cx.entity().downgrade();
            self.set_loading(cx);
            (self.on_confirm)(pin, handle, cx);
        }
    }
}

impl Render for PinPromptContent {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let phase = self.phase.clone();

        match &phase {
            DialogPhase::Success(msg) => v_flex()
                .gap_4()
                .child(
                    h_flex()
                        .gap_2()
                        .items_center()
                        .child(
                            gpui_component::Icon::new(gpui_component::IconName::CircleCheck)
                                .text_color(cx.theme().green)
                                .with_size(gpui_component::Size::Large),
                        )
                        .child(self.title.clone()),
                )
                .child(msg.clone())
                .child(
                    h_flex().justify_end().child(
                        Button::new("done")
                            .primary()
                            .label("Done")
                            .on_click(|_, window, cx| {
                                window.close_dialog(cx);
                            }),
                    ),
                )
                .into_any_element(),

            DialogPhase::Loading => v_flex()
                .gap_4()
                .child(self.description.clone())
                .child(Input::new(&self.pin_input).disabled(true))
                .child(
                    h_flex()
                        .justify_end()
                        .gap_2()
                        .child(Button::new("cancel").label("Cancel").disabled(true))
                        .child(
                            Button::new("confirm")
                                .primary()
                                .label("Loading...")
                                .loading(true),
                        ),
                )
                .into_any_element(),

            DialogPhase::Error(err_msg) => {
                let pin_input = self.pin_input.clone();
                let confirm_label = self.confirm_label.clone();
                let on_confirm = self.on_confirm.clone();
                let handle = cx.entity().downgrade();

                v_flex()
                    .gap_4()
                    .child(self.description.clone())
                    .child(
                        div()
                            .px_3()
                            .py_2()
                            .rounded_md()
                            .bg(cx.theme().danger.opacity(0.1))
                            .text_color(cx.theme().danger)
                            .text_sm()
                            .child(err_msg.clone()),
                    )
                    .child(Input::new(&pin_input))
                    .child(
                        h_flex()
                            .justify_end()
                            .gap_2()
                            .child(Button::new("cancel").label("Cancel").on_click(
                                |_, window, cx| {
                                    window.close_dialog(cx);
                                },
                            ))
                            .child(
                                Button::new("confirm")
                                    .primary()
                                    .label(confirm_label)
                                    .on_click(move |_, _, cx| {
                                        let pin = pin_input.read(cx).text().to_string();
                                        if !pin.is_empty() {
                                            if let Some(h) = handle.upgrade() {
                                                h.update(cx, |this, cx| this.set_loading(cx));
                                            }
                                            on_confirm(pin, handle.clone(), cx);
                                        }
                                    }),
                            ),
                    )
                    .into_any_element()
            }

            DialogPhase::Input => {
                let pin_input = self.pin_input.clone();
                let confirm_label = self.confirm_label.clone();
                let on_confirm = self.on_confirm.clone();
                let handle = cx.entity().downgrade();

                v_flex()
                    .gap_4()
                    .child(self.description.clone())
                    .child(Input::new(&pin_input))
                    .child(
                        h_flex()
                            .justify_end()
                            .gap_2()
                            .child(Button::new("cancel").label("Cancel").on_click(
                                |_, window, cx| {
                                    window.close_dialog(cx);
                                },
                            ))
                            .child(
                                Button::new("confirm")
                                    .primary()
                                    .label(confirm_label)
                                    .on_click(move |_, _, cx| {
                                        let pin = pin_input.read(cx).text().to_string();
                                        if !pin.is_empty() {
                                            if let Some(h) = handle.upgrade() {
                                                h.update(cx, |this, cx| this.set_loading(cx));
                                            }
                                            on_confirm(pin, handle.clone(), cx);
                                        }
                                    }),
                            ),
                    )
                    .into_any_element()
            }
        }
    }
}

pub fn open_pin_prompt(
    title: &str,
    description: &str,
    confirm_label: &str,
    window: &mut Window,
    cx: &mut App,
    on_confirm: impl Fn(String, WeakEntity<PinPromptContent>, &mut App) + 'static,
) {
    let title_str = SharedString::from(title.to_string());
    let description = SharedString::from(description.to_string());
    let confirm_label = SharedString::from(confirm_label.to_string());

    let pin_input = cx.new(|cx| {
        InputState::new(window, cx)
            .placeholder("Enter FIDO PIN")
            .masked(true)
    });

    let dialog_title = title_str.clone();
    let pin_for_sub = pin_input.clone();

    let content = cx.new(|cx| {
        let sub = cx.subscribe(&pin_for_sub, |this: &mut PinPromptContent, _, event, cx| {
            if matches!(event, InputEvent::PressEnter { .. }) {
                this.trigger_confirm(cx);
            }
        });

        PinPromptContent {
            phase: DialogPhase::Input,
            title: title_str,
            description,
            confirm_label,
            pin_input: pin_for_sub,
            on_confirm: std::rc::Rc::new(on_confirm),
            _subscription: sub,
        }
    });

    window.open_dialog(cx, move |dialog, _, _| {
        dialog
            .title(dialog_title.clone())
            .child(content.clone())
            .overlay_closable(false)
            .close_button(false)
    });
}

pub struct ConfirmContent {
    phase: DialogPhase,
    title: SharedString,
    message: String,
    ok_label: SharedString,
    ok_variant: ButtonVariant,
    on_ok: std::rc::Rc<dyn Fn(WeakEntity<ConfirmContent>, &mut App)>,
}

impl ConfirmContent {
    fn set_loading(&mut self, cx: &mut Context<Self>) {
        self.phase = DialogPhase::Loading;
        cx.notify();
    }

    pub fn set_success(&mut self, msg: String, cx: &mut Context<Self>) {
        self.phase = DialogPhase::Success(msg);
        cx.notify();
    }

    pub fn set_error(&mut self, msg: String, cx: &mut Context<Self>) {
        self.phase = DialogPhase::Error(msg);
        cx.notify();
    }
}

impl Render for ConfirmContent {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let phase = self.phase.clone();

        match &phase {
            DialogPhase::Success(msg) => v_flex()
                .gap_4()
                .child(
                    h_flex()
                        .gap_2()
                        .items_center()
                        .child(
                            gpui_component::Icon::new(gpui_component::IconName::CircleCheck)
                                .text_color(cx.theme().green)
                                .with_size(gpui_component::Size::Large),
                        )
                        .child(self.title.clone()),
                )
                .child(msg.clone())
                .child(
                    h_flex().justify_end().child(
                        Button::new("done")
                            .primary()
                            .label("Done")
                            .on_click(|_, window, cx| {
                                window.close_dialog(cx);
                            }),
                    ),
                )
                .into_any_element(),

            DialogPhase::Loading => v_flex()
                .gap_4()
                .child(self.message.clone())
                .child(
                    h_flex()
                        .justify_end()
                        .gap_2()
                        .child(Button::new("cancel").label("Cancel").disabled(true))
                        .child(
                            Button::new("ok")
                                .with_variant(self.ok_variant)
                                .label("Loading...")
                                .loading(true),
                        ),
                )
                .into_any_element(),

            DialogPhase::Error(err_msg) => {
                let ok_label = self.ok_label.clone();
                let ok_variant = self.ok_variant;
                let on_ok = self.on_ok.clone();
                let handle = cx.entity().downgrade();

                v_flex()
                    .gap_4()
                    .child(self.message.clone())
                    .child(
                        div()
                            .px_3()
                            .py_2()
                            .rounded_md()
                            .bg(cx.theme().danger.opacity(0.1))
                            .text_color(cx.theme().danger)
                            .text_sm()
                            .child(err_msg.clone()),
                    )
                    .child(
                        h_flex()
                            .justify_end()
                            .gap_2()
                            .child(Button::new("cancel").label("Cancel").on_click(
                                |_, window, cx| {
                                    window.close_dialog(cx);
                                },
                            ))
                            .child(
                                Button::new("ok")
                                    .with_variant(ok_variant)
                                    .label(ok_label)
                                    .on_click(move |_, _, cx| {
                                        if let Some(h) = handle.upgrade() {
                                            h.update(cx, |this, cx| this.set_loading(cx));
                                        }
                                        on_ok(handle.clone(), cx);
                                    }),
                            ),
                    )
                    .into_any_element()
            }

            DialogPhase::Input => {
                let ok_label = self.ok_label.clone();
                let ok_variant = self.ok_variant;
                let on_ok = self.on_ok.clone();
                let handle = cx.entity().downgrade();

                v_flex()
                    .gap_4()
                    .child(self.message.clone())
                    .child(
                        h_flex()
                            .justify_end()
                            .gap_2()
                            .child(Button::new("cancel").label("Cancel").on_click(
                                |_, window, cx| {
                                    window.close_dialog(cx);
                                },
                            ))
                            .child(
                                Button::new("ok")
                                    .with_variant(ok_variant)
                                    .label(ok_label)
                                    .on_click(move |_, _, cx| {
                                        if let Some(h) = handle.upgrade() {
                                            h.update(cx, |this, cx| this.set_loading(cx));
                                        }
                                        on_ok(handle.clone(), cx);
                                    }),
                            ),
                    )
                    .into_any_element()
            }
        }
    }
}

pub fn open_confirm(
    title: &str,
    message: String,
    ok_label: &str,
    ok_variant: ButtonVariant,
    window: &mut Window,
    cx: &mut App,
    on_ok: impl Fn(WeakEntity<ConfirmContent>, &mut App) + 'static,
) {
    let title_str = SharedString::from(title.to_string());
    let dialog_title = title_str.clone();

    let content = cx.new(|_cx| ConfirmContent {
        phase: DialogPhase::Input,
        title: title_str,
        message,
        ok_label: SharedString::from(ok_label.to_string()),
        ok_variant,
        on_ok: std::rc::Rc::new(on_ok),
    });

    window.open_dialog(cx, move |dialog, _, _| {
        dialog
            .title(dialog_title.clone())
            .child(content.clone())
            .overlay_closable(false)
            .close_button(false)
    });
}

pub struct ChangePinContent {
    phase: DialogPhase,
    current_pin: Entity<InputState>,
    new_pin: Entity<InputState>,
    confirm_pin: Entity<InputState>,
    on_confirm: std::rc::Rc<dyn Fn(String, String, WeakEntity<ChangePinContent>, &mut App)>,
    _subscriptions: Vec<Subscription>,
}

impl ChangePinContent {
    fn set_loading(&mut self, cx: &mut Context<Self>) {
        self.phase = DialogPhase::Loading;
        cx.notify();
    }

    pub fn set_success(&mut self, msg: String, cx: &mut Context<Self>) {
        self.phase = DialogPhase::Success(msg);
        cx.notify();
    }

    pub fn set_error(&mut self, msg: String, cx: &mut Context<Self>) {
        self.phase = DialogPhase::Error(msg);
        cx.notify();
    }

    fn trigger_confirm(&mut self, cx: &mut Context<Self>) {
        if matches!(self.phase, DialogPhase::Loading | DialogPhase::Success(_)) {
            return;
        }

        let current_val = self.current_pin.read(cx).text().to_string();
        let new_val = self.new_pin.read(cx).text().to_string();
        let confirm_val = self.confirm_pin.read(cx).text().to_string();

        if current_val.is_empty() {
            return;
        }

        if new_val != confirm_val {
            self.set_error("PINs do not match".to_string(), cx);
            return;
        }

        if new_val.len() < 4 {
            self.set_error("PIN must be at least 4 characters".to_string(), cx);
            return;
        }

        let handle = cx.entity().downgrade();
        self.set_loading(cx);
        (self.on_confirm)(current_val, new_val, handle, cx);
    }
}

impl Render for ChangePinContent {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let phase = self.phase.clone();

        match &phase {
            DialogPhase::Success(msg) => v_flex()
                .gap_4()
                .child(
                    h_flex()
                        .gap_2()
                        .items_center()
                        .child(
                            gpui_component::Icon::new(gpui_component::IconName::CircleCheck)
                                .text_color(cx.theme().green)
                                .with_size(gpui_component::Size::Large),
                        )
                        .child("Change PIN"),
                )
                .child(msg.clone())
                .child(
                    h_flex().justify_end().child(
                        Button::new("done")
                            .primary()
                            .label("Done")
                            .on_click(|_, window, cx| {
                                window.close_dialog(cx);
                            }),
                    ),
                )
                .into_any_element(),

            DialogPhase::Loading => v_flex()
                .gap_4()
                .child("Enter your current PIN and choose a new one.")
                .child(
                    v_flex()
                        .gap_4()
                        .child("Current PIN")
                        .child(Input::new(&self.current_pin).disabled(true))
                        .child("New PIN")
                        .child(Input::new(&self.new_pin).disabled(true))
                        .child("Confirm New PIN")
                        .child(Input::new(&self.confirm_pin).disabled(true)),
                )
                .child(
                    h_flex()
                        .justify_end()
                        .gap_2()
                        .child(Button::new("cancel").label("Cancel").disabled(true))
                        .child(
                            Button::new("confirm")
                                .primary()
                                .label("Changing PIN...")
                                .loading(true),
                        ),
                )
                .into_any_element(),

            DialogPhase::Error(err_msg) => {
                let current = self.current_pin.clone();
                let new = self.new_pin.clone();
                let confirm = self.confirm_pin.clone();
                let on_confirm = self.on_confirm.clone();
                let handle = cx.entity().downgrade();

                v_flex()
                    .gap_4()
                    .child("Enter your current PIN and choose a new one.")
                    .child(
                        div()
                            .px_3()
                            .py_2()
                            .rounded_md()
                            .bg(cx.theme().danger.opacity(0.1))
                            .text_color(cx.theme().danger)
                            .text_sm()
                            .child(err_msg.clone()),
                    )
                    .child(
                        v_flex()
                            .gap_4()
                            .child("Current PIN")
                            .child(Input::new(&current))
                            .child("New PIN")
                            .child(Input::new(&new))
                            .child("Confirm New PIN")
                            .child(Input::new(&confirm)),
                    )
                    .child(
                        h_flex()
                            .justify_end()
                            .gap_2()
                            .child(
                                Button::new("cancel")
                                    .label("Cancel")
                                    .on_click(|_, window, cx| window.close_dialog(cx)),
                            )
                            .child(Button::new("confirm").primary().label("Confirm").on_click(
                                move |_, _, cx| {
                                    let current_val = current.read(cx).text().to_string();
                                    let new_val = new.read(cx).text().to_string();
                                    let confirm_val = confirm.read(cx).text().to_string();

                                    if current_val.is_empty() {
                                        return;
                                    }

                                    if new_val != confirm_val {
                                        if let Some(h) = handle.upgrade() {
                                            h.update(cx, |this, cx| {
                                                this.set_error("PINs do not match".to_string(), cx);
                                            });
                                        }
                                        return;
                                    }

                                    if new_val.len() < 4 {
                                        if let Some(h) = handle.upgrade() {
                                            h.update(cx, |this, cx| {
                                                this.set_error(
                                                    "PIN must be at least 4 characters".to_string(),
                                                    cx,
                                                );
                                            });
                                        }
                                        return;
                                    }

                                    if let Some(h) = handle.upgrade() {
                                        h.update(cx, |this, cx| this.set_loading(cx));
                                    }
                                    on_confirm(current_val, new_val, handle.clone(), cx);
                                },
                            )),
                    )
                    .into_any_element()
            }

            DialogPhase::Input => {
                let current = self.current_pin.clone();
                let new = self.new_pin.clone();
                let confirm = self.confirm_pin.clone();
                let on_confirm = self.on_confirm.clone();
                let handle = cx.entity().downgrade();

                v_flex()
                    .gap_4()
                    .child("Enter your current PIN and choose a new one.")
                    .child(
                        v_flex()
                            .gap_4()
                            .child("Current PIN")
                            .child(Input::new(&current))
                            .child("New PIN")
                            .child(Input::new(&new))
                            .child("Confirm New PIN")
                            .child(Input::new(&confirm)),
                    )
                    .child(
                        h_flex()
                            .justify_end()
                            .gap_2()
                            .child(
                                Button::new("cancel")
                                    .label("Cancel")
                                    .on_click(|_, window, cx| window.close_dialog(cx)),
                            )
                            .child(Button::new("confirm").primary().label("Confirm").on_click(
                                move |_, _, cx| {
                                    let current_val = current.read(cx).text().to_string();
                                    let new_val = new.read(cx).text().to_string();
                                    let confirm_val = confirm.read(cx).text().to_string();

                                    if current_val.is_empty() {
                                        return;
                                    }

                                    if new_val != confirm_val {
                                        if let Some(h) = handle.upgrade() {
                                            h.update(cx, |this, cx| {
                                                this.set_error("PINs do not match".to_string(), cx);
                                            });
                                        }
                                        return;
                                    }

                                    if new_val.len() < 4 {
                                        if let Some(h) = handle.upgrade() {
                                            h.update(cx, |this, cx| {
                                                this.set_error(
                                                    "PIN must be at least 4 characters".to_string(),
                                                    cx,
                                                );
                                            });
                                        }
                                        return;
                                    }

                                    if let Some(h) = handle.upgrade() {
                                        h.update(cx, |this, cx| this.set_loading(cx));
                                    }
                                    on_confirm(current_val, new_val, handle.clone(), cx);
                                },
                            )),
                    )
                    .into_any_element()
            }
        }
    }
}

pub fn open_change_pin(
    window: &mut Window,
    cx: &mut App,
    on_confirm: impl Fn(String, String, WeakEntity<ChangePinContent>, &mut App) + 'static,
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

    let confirm_for_sub = confirm_pin.clone();

    let content = cx.new(|cx| {
        let sub = cx.subscribe(
            &confirm_for_sub,
            |this: &mut ChangePinContent, _, event, cx| {
                if matches!(event, InputEvent::PressEnter { .. }) {
                    this.trigger_confirm(cx);
                }
            },
        );

        ChangePinContent {
            phase: DialogPhase::Input,
            current_pin,
            new_pin,
            confirm_pin: confirm_for_sub,
            on_confirm: std::rc::Rc::new(on_confirm),
            _subscriptions: vec![sub],
        }
    });

    window.open_dialog(cx, move |dialog, _, _| {
        dialog
            .title("Change PIN")
            .child(content.clone())
            .overlay_closable(false)
            .close_button(false)
    });
}

pub struct StatusContent {
    phase: DialogPhase,
    title: SharedString,
}

impl StatusContent {
    pub fn set_success(&mut self, msg: String, cx: &mut Context<Self>) {
        self.phase = DialogPhase::Success(msg);
        cx.notify();
    }

    pub fn set_error(&mut self, msg: String, cx: &mut Context<Self>) {
        self.phase = DialogPhase::Error(msg);
        cx.notify();
    }
}

impl Render for StatusContent {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let phase = self.phase.clone();

        match &phase {
            DialogPhase::Success(msg) => v_flex()
                .gap_4()
                .child(
                    h_flex()
                        .gap_2()
                        .items_center()
                        .child(
                            gpui_component::Icon::new(gpui_component::IconName::CircleCheck)
                                .text_color(cx.theme().green)
                                .with_size(gpui_component::Size::Large),
                        )
                        .child(self.title.clone()),
                )
                .child(msg.clone())
                .child(
                    h_flex().justify_end().child(
                        Button::new("done")
                            .primary()
                            .label("Done")
                            .on_click(|_, window, cx| {
                                window.close_dialog(cx);
                            }),
                    ),
                )
                .into_any_element(),

            DialogPhase::Error(err_msg) => {
                v_flex()
                    .gap_4()
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(
                                gpui_component::Icon::new(gpui_component::IconName::CircleX)
                                    .text_color(cx.theme().danger)
                                    .with_size(gpui_component::Size::Large),
                            )
                            .child(self.title.clone()),
                    )
                    .child(
                        div()
                            .px_3()
                            .py_2()
                            .rounded_md()
                            .bg(cx.theme().danger.opacity(0.1))
                            .text_color(cx.theme().danger)
                            .text_sm()
                            .child(err_msg.clone()),
                    )
                    .child(
                        h_flex()
                            .justify_end()
                            .child(Button::new("close").label("Close").on_click(
                                |_, window, cx| {
                                    window.close_dialog(cx);
                                },
                            )),
                    )
                    .into_any_element()
            }

            _ => v_flex()
                .gap_4()
                .items_center()
                .child("Applying configuration...")
                .child(
                    Button::new("loading")
                        .primary()
                        .label("Applying...")
                        .loading(true),
                )
                .into_any_element(),
        }
    }
}

pub fn open_status_dialog(
    title: &str,
    window: &mut Window,
    cx: &mut App,
) -> WeakEntity<StatusContent> {
    let title_str = SharedString::from(title.to_string());
    let dialog_title = title_str.clone();

    let content = cx.new(|_cx| StatusContent {
        phase: DialogPhase::Loading,
        title: title_str,
    });

    let handle = content.downgrade();

    window.open_dialog(cx, move |dialog, _, _| {
        dialog
            .title(dialog_title.clone())
            .child(content.clone())
            .overlay_closable(false)
            .close_button(false)
    });

    handle
}
