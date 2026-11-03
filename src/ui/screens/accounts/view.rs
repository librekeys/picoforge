//! Accounts (OATH) screen rendering.

use crate::ui::components::button::PFIconButton;
use crate::ui::components::card::Card;
use crate::ui::components::page_view::PageView;
use crate::ui::models::device::oath;
use crate::ui::screens::accounts::view_model::AccountsViewModel;
use gpui::*;
use gpui_component::button::{Button, ButtonCustomVariant, ButtonVariants};
use gpui_component::{ActiveTheme, Disableable, Icon, StyledExt, Theme, h_flex, v_flex};

/// Split a numeric code into two halves for readability ("123 456").
fn format_code(code: &str) -> String {
    if code.len() >= 6 {
        let mid = code.len() / 2;
        crate::tr!("{} {}", &code[..mid], &code[mid..])
    } else {
        code.to_string()
    }
}

fn seconds_left(now: u64, period: u32) -> u32 {
    let p = period.max(1) as u64;
    (p - (now % p)) as u32
}

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

impl AccountsViewModel {
    fn render_account_row(
        &self,
        acc: &oath::Account,
        now: u64,
        can_rename: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = cx.theme();
        let issuer = acc.issuer.clone().unwrap_or_default();
        let account = acc.account.clone();
        let id = acc.id.clone();
        let period = acc.period;

        let (primary, secondary) = if issuer.is_empty() {
            (account.clone(), String::new())
        } else {
            (issuer, account)
        };

        let identity = v_flex()
            .gap_0p5()
            .child(div().font_medium().child(primary))
            .child(
                div()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child(secondary),
            );

        let acc_for_rename = acc.clone();
        let rename_btn = can_rename.then(|| {
            Button::new(SharedString::from(crate::tr!("ren-{id}", id = id)))
                .icon(Icon::default().path("icons/tag.svg"))
                .ghost()
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.open_rename_dialog(&acc_for_rename, window, cx);
                }))
        });
        let acc_for_delete = acc.clone();
        let delete_btn = Button::new(SharedString::from(crate::tr!("del-{id}", id = id)))
            .icon(Icon::default().path("icons/trash-2.svg"))
            .ghost()
            .on_click(cx.listener(move |this, _, window, cx| {
                this.open_delete_dialog(&acc_for_delete, window, cx);
            }));

        let right = match &acc.state {
            oath::CodeState::Code { value, period: p } => {
                let code = value.clone();
                let rem = seconds_left(now, *p);
                let copy_code = code.clone();
                h_flex()
                    .gap_3()
                    .items_center()
                    .child(
                        div()
                            .font_family("Mono")
                            .text_xl()
                            .text_color(theme.foreground)
                            .child(format_code(&code)),
                    )
                    .child(
                        div()
                            .w_8()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child(crate::tr!("{rem}s", rem = rem)),
                    )
                    .child(
                        Button::new(SharedString::from(crate::tr!("copy-{id}", id = id)))
                            .icon(Icon::default().path("icons/copy.svg"))
                            .ghost()
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.copy_code(copy_code.clone(), cx);
                            })),
                    )
                    .children(rename_btn)
                    .child(delete_btn)
            }
            oath::CodeState::Hotp => h_flex()
                .gap_3()
                .items_center()
                .child(
                    Button::new(SharedString::from(crate::tr!("calc-{id}", id = id)))
                        .label(crate::tr!("Generate"))
                        .outline()
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.calculate(id.clone(), period, cx);
                        })),
                )
                .children(rename_btn)
                .child(delete_btn),
            oath::CodeState::Touch => h_flex()
                .gap_3()
                .items_center()
                .child(
                    Button::new(SharedString::from(crate::tr!("touch-{id}", id = id)))
                        .label(crate::tr!("Touch to reveal"))
                        .outline()
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.calculate(id.clone(), period, cx);
                        })),
                )
                .children(rename_btn)
                .child(delete_btn),
        };

        h_flex()
            .justify_between()
            .items_center()
            .p_3()
            .border_1()
            .border_color(theme.border)
            .rounded_lg()
            .child(identity)
            .child(right)
            .into_any_element()
    }
}

impl Render for AccountsViewModel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let title: &str = crate::tr!("Accounts");
        let subtitle: &str = crate::tr!("One-time password accounts (OATH).");

        if let Some((heading, body)) = self.gate(cx).message() {
            let theme = cx.theme();
            return PageView::build(title, subtitle, empty_state(heading, body, theme), theme)
                .into_any_element();
        }

        if self.needs_password && !self.loaded {
            let unlock = Button::new("unlock-oath")
                .icon(Icon::default().path("icons/lock-open.svg"))
                .label(crate::tr!("Unlock"))
                .primary()
                .on_click(cx.listener(|this, _, window, cx| this.open_unlock_dialog(window, cx)));
            let theme = cx.theme();
            let card = Card::new()
                .title(crate::tr!("Accounts"))
                .description(crate::tr!("Password-protected"))
                .icon(Icon::default().path("icons/key.svg"))
                .child(
                    v_flex()
                        .items_center()
                        .justify_center()
                        .gap_3()
                        .py_6()
                        .child(
                            div()
                                .font_semibold()
                                .child(crate::tr!("Accounts are password-protected")),
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_color(theme.muted_foreground)
                                .child(crate::tr!("Enter the OATH password to view your codes.")),
                        )
                        .child(unlock),
                );
            return PageView::build(title, subtitle, card, theme).into_any_element();
        }

        // Build rows first (mutable cx), then the chrome.
        let accounts = self.accounts.clone();
        let now = self.now;
        let can_rename = self
            .device
            .read(cx)
            .oath_features()
            .map(|f| f.rename)
            .unwrap_or(false);
        let mut rows = Vec::with_capacity(accounts.len());
        for acc in &accounts {
            rows.push(self.render_account_row(acc, now, can_rename, cx));
        }

        let password_label = if self.password_is_set() {
            crate::tr!("Change password")
        } else {
            crate::tr!("Set password")
        };
        let theme = cx.theme();
        let password_btn =
            PFIconButton::new(Icon::default().path("icons/lock.svg"), password_label)
                .with_colors(rgb(0x222225), rgb(0x2a2a2d), rgb(0x333336))
                .on_click(cx.listener(|this, _, window, cx| this.open_password_dialog(window, cx)));
        let refresh_btn = Button::new("refresh-oath")
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
        let add_btn = Button::new("add-account")
            .icon(Icon::default().path("icons/plus.svg"))
            .label(crate::tr!("Add account"))
            .primary()
            .disabled(self.loading)
            .on_click(cx.listener(|this, _, window, cx| this.open_add_dialog(window, cx)));
        let reset_btn = Button::new("reset-oath")
            .label(crate::tr!("Reset OATH applet"))
            .danger()
            .disabled(self.loading)
            .on_click(cx.listener(|this, _, window, cx| this.open_reset_dialog(window, cx)));

        let toolbar = h_flex()
            .gap_2()
            .child(password_btn)
            .child(refresh_btn)
            .child(add_btn);
        let list = if rows.is_empty() {
            empty_state(
                crate::tr!("No accounts yet"),
                crate::tr!("Add an account from an otpauth:// URI or a base32 secret.").into(),
                theme,
            )
        } else {
            v_flex().gap_2().children(rows).into_any_element()
        };

        let accounts_card = Card::new()
            .title(crate::tr!("Accounts"))
            .description(crate::tr!("{} stored", accounts.len()))
            .icon(Icon::default().path("icons/users-round.svg"))
            .header_right(toolbar)
            .child(list);
        let reset_card = Card::new()
            .title(crate::tr!("Reset"))
            .description(crate::tr!("Erase all accounts and the OATH password"))
            .icon(Icon::default().path("icons/trash.svg"))
            .child(
                h_flex()
                    .items_center()
                    .justify_between()
                    .child(
                        v_flex()
                            .gap_1()
                            .child(div().font_medium().child(crate::tr!("Reset OATH applet")))
                            .child(div().text_sm().text_color(theme.muted_foreground).child(
                                crate::tr!(
                                    "Deletes every account and the password. Cannot be undone."
                                ),
                            )),
                    )
                    .child(reset_btn),
            );

        let content = v_flex().gap_6().child(accounts_card).child(reset_card);
        PageView::build(title, subtitle, content, theme).into_any_element()
    }
}
