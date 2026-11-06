//! Lock screen rendering.

use crate::ui::components::button::PFButton;
use crate::ui::components::card::Card;
use crate::ui::components::page_view::PageView;
use crate::ui::screens::lock::view_model::LockViewModel;
use gpui_kit::component::button::{Button, ButtonCustomVariant, ButtonVariants};
use gpui_kit::component::{ActiveTheme, Disableable, Icon, StyledExt, Theme, h_flex, v_flex};
use gpui_kit::*;

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
            PFButton::new("Copy")
                .id("lk-copy")
                .on_click(cx.listener(move |_, _, _, cx| {
                    cx.write_to_clipboard(ClipboardItem::new_string(p.clone()));
                }))
        };
        let clear = PFButton::new("Clear from screen")
            .id("lk-clear")
            .on_click(cx.listener(|this, _, _, cx| this.clear_lock_key(cx)));

        Card::new()
            .title("Lock key")
            .description("Shown once — you need it to unlock after every power-cycle")
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
                            .child("Lose this and the only recovery is a FIDO factory reset, which destroys this identity. Store it offline."),
                    )
                    .child(
                        div()
                            .p_4()
                            .rounded_lg()
                            .border_1()
                            .border_color(theme.border)
                            .font_family("monospace")
                            .text_sm()
                            .child(phrase.to_string()),
                    ),
            )
            .into_any_element()
    }
}

impl Render for LockViewModel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        const TITLE: &str = "Lock";
        const SUBTITLE: &str = "At-rest soft-lock of the FIDO seed.";

        if let Some((heading, body)) = self.gate(cx).message() {
            let theme = cx.theme();
            return PageView::build(TITLE, SUBTITLE, empty_state(heading, body, theme), theme)
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
                    .active(rgb(0x3f3f46).into()),
            )
            .border_color(theme.border)
            .disabled(self.loading)
            .on_click(cx.listener(|this, _, _, cx| this.refresh(cx)));
        let enable_btn = Button::new("lk-enable")
            .label("Engage lock")
            .danger()
            .disabled(self.loading || locked)
            .on_click(cx.listener(|this, _, window, cx| this.open_enable(window, cx)));
        let unlock_btn = PFButton::new("Unlock")
            .id("lk-unlock")
            .with_colors(rgb(0x222225), rgb(0x2a2a2d), rgb(0x333336))
            .disabled(self.loading || !locked)
            .on_click(cx.listener(|this, _, window, cx| this.open_unlock(window, cx)));
        let disable_btn = PFButton::new("Disable lock")
            .id("lk-disable")
            .with_colors(rgb(0x222225), rgb(0x2a2a2d), rgb(0x333336))
            .disabled(self.loading || !locked)
            .on_click(cx.listener(|this, _, window, cx| this.open_disable(window, cx)));

        let status_card = {
            let body =
                match status {
                    Some(s) => {
                        let state = if s.locked {
                            if s.unlocked {
                                "locked, unlocked for this power-cycle"
                            } else {
                                "locked — unlock before any FIDO login"
                            }
                        } else {
                            "not locked (plaintext seed)"
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
                                    .child(div().text_sm().child(format!("State: {state}"))),
                            )
                            .child(div().text_sm().text_color(theme.muted_foreground).child(
                                format!("Seed present: {}", if s.has_seed { "yes" } else { "no" }),
                            ))
                            .into_any_element()
                    }
                    None => div()
                        .text_sm()
                        .text_color(theme.muted_foreground)
                        .child("Reading lock state…")
                        .into_any_element(),
                };
            Card::new()
                .title("Lock status")
                .description("Whether the seed is wrapped at rest")
                .icon(Icon::default().path("icons/lock.svg"))
                .header_right(refresh_btn)
                .child(body)
        };

        let actions_card = Card::new()
            .title("Actions")
            .description("Engage, unlock, or disable the at-rest lock")
            .icon(Icon::default().path("icons/lock-open.svg"))
            .child(
                v_flex()
                    .gap_2()
                    .child(self.action_row(
                        "Engage lock",
                        "Wrap the seed and reveal a new lock key (PIN + touch)",
                        enable_btn,
                        theme,
                    ))
                    .child(self.action_row(
                        "Unlock",
                        "Load the seed for this power-cycle (lock key)",
                        unlock_btn,
                        theme,
                    ))
                    .child(self.action_row(
                        "Disable lock",
                        "Restore the plaintext seed (lock key + PIN)",
                        disable_btn,
                        theme,
                    )),
            );

        let content = v_flex()
            .gap_6()
            .child(status_card)
            .children(lock_key_card)
            .child(actions_card);

        PageView::build(TITLE, SUBTITLE, content, theme).into_any_element()
    }
}
