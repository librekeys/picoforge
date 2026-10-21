//! Lock screen rendering.

use crate::ui::components::button::PFButton;
use crate::ui::components::card::Card;
use crate::ui::components::page_view::PageView;
use crate::ui::screens::lock::view_model::LockViewModel;
use gpui::*;
use gpui_component::button::{Button, ButtonCustomVariant, ButtonVariants};
use gpui_component::{ActiveTheme, Disableable, Icon, StyledExt, Theme, h_flex, v_flex};

fn empty_state(heading: &str, body: String, theme: &Theme) -> AnyElement {
    v_flex()
        .items_center()
        .justify_center()
        .h_64()
        .gap_2()
        .border_1()
        .border_color(theme.border)
        .rounded_xl()
        .child(div().font_semibold().child(heading.to_string()))
        .child(
            div()
                .text_sm()
                .max_w(px(380.))
                .text_color(theme.muted_foreground)
                .child(body),
        )
        .into_any_element()
}

impl LockViewModel {
    fn action_row(
        &self,
        title: &'static str,
        subtitle: &'static str,
        btn: impl IntoElement,
        theme: &Theme,
    ) -> impl IntoElement {
        h_flex()
            .items_center()
            .justify_between()
            .p_4()
            .border_1()
            .border_color(theme.border)
            .rounded_lg()
            .child(
                v_flex()
                    .gap_0p5()
                    .child(div().font_medium().child(title))
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child(subtitle),
                    ),
            )
            .child(btn)
    }

    fn lock_key_card(&self, phrase: &str, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let copy = {
            let p = phrase.to_string();
            PFButton::new(crate::tr!("Copy"))
                .id("lk-copy")
                .on_click(cx.listener(move |_, _, _, cx| {
                    cx.write_to_clipboard(ClipboardItem::new_string(p.clone()));
                }))
        };
        let clear = PFButton::new(crate::tr!("Clear from screen"))
            .id("lk-clear")
            .on_click(cx.listener(|this, _, _, cx| this.clear_lock_key(cx)));

        Card::new()
            .title(crate::tr!("Lock key"))
            .description(crate::tr!("Shown once — you need it to unlock after every power-cycle"))
            .icon(Icon::default().path("icons/key-round.svg"))
            .header_right(h_flex().gap_2().child(copy).child(clear))
            .child(
                v_flex()
                    .gap_3()
                    .child(
                        div()
                            .p_3()
                            .rounded_md()
                            .bg(rgb(0x18181b))
                            .text_color(rgb(0xf59e0b))
                            .text_sm()
                            .child(crate::tr!("Lose this and the only recovery is a FIDO factory reset, which destroys this identity. Store it offline.")),
                    )
                    .child(
                        div()
                            .p_4()
                            .rounded_lg()
                            .border_1()
                            .border_color(theme.border)
                            .font_family(crate::tr!("monospace"))
                            .text_sm()
                            .child(phrase.to_string()),
                    ),
            )
            .into_any_element()
    }
}

impl Render for LockViewModel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let title: &str = crate::tr!("Lock");
        let subtitle: &str = crate::tr!("At-rest soft-lock of the FIDO seed.");

        if let Some((heading, body)) = self.gate(cx).message() {
            let theme = cx.theme();
            return PageView::build(title, subtitle, empty_state(heading, body, theme), theme)
                .into_any_element();
        }

        let status = self.status;
        let locked = status.map(|s| s.locked).unwrap_or(false);
        let lock_key = self.lock_key.clone();
        let lock_key_card = lock_key.map(|p| self.lock_key_card(&p, cx));
        let theme = cx.theme();

        let refresh_btn = Button::new("lk-refresh")
            .icon(Icon::default().path("icons/refresh-cw.svg"))
            .custom(
                ButtonCustomVariant::new(cx)
                    .color(rgb(0x1b1b1d).into())
                    .hover(rgb(0x232325).into())
                    .active(rgb(0x3f3f46).into())
                    .border(theme.border),
            )
            .disabled(self.loading)
            .on_click(cx.listener(|this, _, _, cx| this.refresh(cx)));
        let enable_btn = Button::new("lk-enable")
            .label(crate::tr!("Engage lock"))
            .danger()
            .disabled(self.loading || locked)
            .on_click(cx.listener(|this, _, window, cx| this.open_enable(window, cx)));
        let unlock_btn = PFButton::new(crate::tr!("Unlock"))
            .id("lk-unlock")
            .with_colors(rgb(0x222225), rgb(0x2a2a2d), rgb(0x333336))
            .disabled(self.loading || !locked)
            .on_click(cx.listener(|this, _, window, cx| this.open_unlock(window, cx)));
        let disable_btn = PFButton::new(crate::tr!("Disable lock"))
            .id("lk-disable")
            .with_colors(rgb(0x222225), rgb(0x2a2a2d), rgb(0x333336))
            .disabled(self.loading || !locked)
            .on_click(cx.listener(|this, _, window, cx| this.open_disable(window, cx)));

        let status_card = {
            let body = match status {
                Some(s) => {
                    let state = if s.locked {
                        if s.unlocked {
                            crate::tr!("locked, unlocked for this power-cycle")
                        } else {
                            crate::tr!("locked — unlock before any FIDO login")
                        }
                    } else {
                        crate::tr!("not locked (plaintext seed)")
                    };
                    let (dot, color) = if s.locked && !s.unlocked {
                        ("●", theme.danger)
                    } else if s.locked {
                        ("●", theme.green)
                    } else {
                        ("●", theme.muted_foreground)
                    };
                    v_flex()
                        .gap_2()
                        .child(
                            h_flex()
                                .gap_2()
                                .items_center()
                                .child(div().text_color(color).child(dot))
                                .child(
                                    div()
                                        .text_sm()
                                        .child(crate::tr!("State: {state}", state = state)),
                                ),
                        )
                        .child(div().text_sm().text_color(theme.muted_foreground).child(
                            crate::tr!(
                                "Seed present: {}",
                                if s.has_seed {
                                    crate::tr!("yes")
                                } else {
                                    crate::tr!("no")
                                }
                            ),
                        ))
                        .into_any_element()
                }
                None => div()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child(crate::tr!("Reading lock state…"))
                    .into_any_element(),
            };
            Card::new()
                .title(crate::tr!("Lock status"))
                .description(crate::tr!("Whether the seed is wrapped at rest"))
                .icon(Icon::default().path("icons/lock.svg"))
                .header_right(refresh_btn)
                .child(body)
        };

        let actions_card = Card::new()
            .title(crate::tr!("Actions"))
            .description(crate::tr!("Engage, unlock, or disable the at-rest lock"))
            .icon(Icon::default().path("icons/lock-open.svg"))
            .child(
                v_flex()
                    .gap_2()
                    .child(self.action_row(
                        crate::tr!("Engage lock"),
                        crate::tr!("Wrap the seed and reveal a new lock key (PIN + touch)"),
                        enable_btn,
                        theme,
                    ))
                    .child(self.action_row(
                        crate::tr!("Unlock"),
                        crate::tr!("Load the seed for this power-cycle (lock key)"),
                        unlock_btn,
                        theme,
                    ))
                    .child(self.action_row(
                        crate::tr!("Disable lock"),
                        crate::tr!("Restore the plaintext seed (lock key + PIN)"),
                        disable_btn,
                        theme,
                    )),
            );

        let content = v_flex()
            .gap_6()
            .child(status_card)
            .children(lock_key_card)
            .child(actions_card);

        PageView::build(title, subtitle, content, theme).into_any_element()
    }
}
