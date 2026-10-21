//! OpenPGP screen rendering.

use crate::ui::components::button::PFButton;
use crate::ui::components::card::Card;
use crate::ui::components::page_view::PageView;
use crate::ui::models::device::openpgp;
use crate::ui::screens::openpgp::view_model::OpenPgpViewModel;
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

fn kv(label: &str, value: String, theme: &Theme) -> impl IntoElement {
    v_flex()
        .gap_1()
        .child(
            div()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(label.to_string()),
        )
        .child(div().text_sm().font_medium().child(value))
}

/// Group a fingerprint hex string into 4-char blocks for readability.
fn group_fp(fp: &str) -> String {
    fp.to_uppercase()
        .as_bytes()
        .chunks(4)
        .map(|c| String::from_utf8_lossy(c).to_string())
        .collect::<Vec<_>>()
        .join(" ")
}

impl OpenPgpViewModel {
    fn render_key_row(&self, k: openpgp::PgpKey, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let slot = k.slot;
        let d = self.loading;
        let has_fp = k.fingerprint.chars().any(|c| c != '0');
        let status = if k.present {
            crate::tr!(
                "{} · touch {}",
                k.algo,
                if k.touch {
                    crate::tr!("on")
                } else {
                    crate::tr!("off")
                }
            )
        } else {
            crate::tr!("Empty").to_string()
        };

        let mut col = v_flex()
            .gap_0p5()
            .child(div().font_medium().child(slot.label()))
            .child(
                div()
                    .text_sm()
                    .text_color(if k.present {
                        theme.foreground
                    } else {
                        theme.muted_foreground
                    })
                    .child(status),
            );
        if has_fp {
            col = col.child(
                div()
                    .text_xs()
                    .font_family(crate::tr!("monospace"))
                    .text_color(theme.muted_foreground)
                    .child(group_fp(&k.fingerprint)),
            );
        }

        v_flex()
            .gap_3()
            .p_4()
            .border_1()
            .border_color(theme.border)
            .rounded_lg()
            .child(col)
            .child(
                h_flex()
                    .gap_2()
                    .flex_wrap()
                    .child(
                        PFButton::new(if k.present {
                            crate::tr!("Regenerate")
                        } else {
                            crate::tr!("Generate")
                        })
                        .id(crate::tr!("gen-{}", slot.label()))
                        .with_colors(rgb(0x222225), rgb(0x2a2a2d), rgb(0x333336))
                        .disabled(d)
                        .on_click(cx.listener(
                            move |this, _, window, cx| {
                                this.open_generate(slot, window, cx);
                            },
                        )),
                    )
                    .child(
                        PFButton::new(crate::tr!("Touch policy"))
                            .id(crate::tr!("touch-{}", slot.label()))
                            .disabled(d)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.open_touch(slot, window, cx);
                            })),
                    ),
            )
            .into_any_element()
    }

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
}

impl Render for OpenPgpViewModel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let title: &str = crate::tr!("OpenPGP");
        let subtitle: &str = crate::tr!("OpenPGP smart card — keys, PINs, and cardholder.");

        if let Some((heading, body)) = self.gate(cx).message() {
            let theme = cx.theme();
            return PageView::build(title, subtitle, empty_state(heading, body, theme), theme)
                .into_any_element();
        }

        let info = self.info.clone();
        let keys = info.as_ref().map(|i| i.keys.clone()).unwrap_or_default();

        let mut key_rows = Vec::new();
        for k in keys {
            key_rows.push(self.render_key_row(k, cx));
        }
        let theme = cx.theme();

        let refresh_btn = Button::new("pgp-refresh")
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
        let change_user_btn = PFButton::new(crate::tr!("Change User PIN"))
            .id("pgp-change-user")
            .with_colors(rgb(0x222225), rgb(0x2a2a2d), rgb(0x333336))
            .on_click(cx.listener(|this, _, window, cx| this.open_change_user_pin(window, cx)));
        let change_admin_btn = PFButton::new(crate::tr!("Change Admin PIN"))
            .id("pgp-change-admin")
            .with_colors(rgb(0x222225), rgb(0x2a2a2d), rgb(0x333336))
            .on_click(cx.listener(|this, _, window, cx| this.open_change_admin_pin(window, cx)));
        let unblock_code_btn = PFButton::new(crate::tr!("Reset code"))
            .id("pgp-unblock-code")
            .with_colors(rgb(0x222225), rgb(0x2a2a2d), rgb(0x333336))
            .on_click(cx.listener(|this, _, window, cx| this.open_unblock_with_code(window, cx)));
        let unblock_admin_btn = PFButton::new(crate::tr!("Admin PIN"))
            .id("pgp-unblock-admin")
            .with_colors(rgb(0x222225), rgb(0x2a2a2d), rgb(0x333336))
            .on_click(cx.listener(|this, _, window, cx| this.open_unblock_with_admin(window, cx)));
        let reset_code_btn = PFButton::new(crate::tr!("Set reset code"))
            .id("pgp-set-rc")
            .with_colors(rgb(0x222225), rgb(0x2a2a2d), rgb(0x333336))
            .on_click(cx.listener(|this, _, window, cx| this.open_set_reset_code(window, cx)));
        let cardholder_btn = PFButton::new(crate::tr!("Edit"))
            .id("pgp-cardholder")
            .with_colors(rgb(0x222225), rgb(0x2a2a2d), rgb(0x333336))
            .on_click(cx.listener(|this, _, window, cx| this.open_cardholder(window, cx)));
        let reset_btn = Button::new("pgp-reset")
            .label(crate::tr!("Reset OpenPGP applet"))
            .danger()
            .disabled(self.loading)
            .on_click(cx.listener(|this, _, window, cx| this.open_reset_dialog(window, cx)));

        let info_card = {
            let body = match &info {
                Some(i) => {
                    let serial = if i.serial != 0 {
                        i.serial.to_string()
                    } else {
                        "—".into()
                    };
                    let field = |s: &str| {
                        if s.is_empty() {
                            "—".to_string()
                        } else {
                            s.to_string()
                        }
                    };
                    div()
                        .grid()
                        .grid_cols(2)
                        .gap_4()
                        .child(kv(
                            crate::tr!("Version"),
                            crate::tr!("{}.{}.{}", i.version[0], i.version[1], i.version[2]),
                            theme,
                        ))
                        .child(kv(crate::tr!("Serial"), serial, theme))
                        .child(kv(crate::tr!("Name"), field(&i.name), theme))
                        .child(kv(crate::tr!("Login"), field(&i.login), theme))
                        .child(kv(crate::tr!("URL"), field(&i.url), theme))
                        .child(kv(
                            crate::tr!("PIN tries (user / reset / admin)"),
                            crate::tr!("{} / {} / {}", i.pw1_retries, i.rc_retries, i.pw3_retries),
                            theme,
                        ))
                        .into_any_element()
                }
                None => div()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child(crate::tr!("Reading card…"))
                    .into_any_element(),
            };
            Card::new()
                .title(crate::tr!("Card information"))
                .description(crate::tr!("OpenPGP card status"))
                .icon(Icon::default().path("icons/cpu.svg"))
                .header_right(refresh_btn)
                .child(body)
        };

        let keys_card = Card::new()
            .title(crate::tr!("Keys"))
            .description(crate::tr!("Signature / Encryption / Authentication slots"))
            .icon(Icon::default().path("icons/key.svg"))
            .child(v_flex().gap_2().children(key_rows));

        let pin_card = Card::new()
            .title(crate::tr!("PINs"))
            .description(crate::tr!("User PIN (PW1) and admin PIN (PW3)"))
            .icon(Icon::default().path("icons/lock.svg"))
            .child(
                v_flex()
                    .gap_2()
                    .child(self.action_row(
                        crate::tr!("User PIN"),
                        crate::tr!("Change the user PIN (PW1)"),
                        change_user_btn,
                        theme,
                    ))
                    .child(self.action_row(
                        crate::tr!("Admin PIN"),
                        crate::tr!("Change the admin PIN (PW3)"),
                        change_admin_btn,
                        theme,
                    ))
                    .child(self.action_row(
                        crate::tr!("Unblock user PIN"),
                        crate::tr!("Reset a blocked user PIN with the reset code"),
                        unblock_code_btn,
                        theme,
                    ))
                    .child(self.action_row(
                        crate::tr!("Unblock via admin"),
                        crate::tr!("Reset a blocked user PIN with the admin PIN"),
                        unblock_admin_btn,
                        theme,
                    ))
                    .child(self.action_row(
                        crate::tr!("Reset code"),
                        crate::tr!("Set or clear the reset code (needs the admin PIN)"),
                        reset_code_btn,
                        theme,
                    )),
            );

        let cardholder_card = Card::new()
            .title(crate::tr!("Cardholder"))
            .description(crate::tr!("Name, login, and URL stored on the card"))
            .icon(Icon::default().path("icons/user.svg"))
            .child(self.action_row(
                crate::tr!("Cardholder details"),
                crate::tr!("Edit the cardholder name, login, and URL"),
                cardholder_btn,
                theme,
            ));

        let reset_card = Card::new()
            .title(crate::tr!("Reset"))
            .description(crate::tr!("Erase all OpenPGP keys and data"))
            .icon(Icon::default().path("icons/trash.svg"))
            .child(self.action_row(
                crate::tr!("Factory reset OpenPGP"),
                crate::tr!("Blocks both PINs then wipes everything. Cannot be undone."),
                reset_btn,
                theme,
            ));

        let content = v_flex()
            .gap_6()
            .child(info_card)
            .child(keys_card)
            .child(pin_card)
            .child(cardholder_card)
            .child(reset_card);

        PageView::build(title, subtitle, content, theme).into_any_element()
    }
}
