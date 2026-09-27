//! Offboard screen rendering.

use crate::ui::components::card::Card;
use crate::ui::components::page_view::PageView;
use crate::ui::screens::offboard::view_model::OffboardViewModel;
use gpui::*;
use gpui_component::button::{Button, ButtonVariants};
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

impl OffboardViewModel {
    fn report_card(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let Some(r) = &self.report else {
            return div().into_any_element();
        };

        let mut rows = Vec::new();
        for s in &r.steps {
            let (mark, color) = if s.ok {
                ("✓", theme.green)
            } else {
                ("✗", theme.danger)
            };
            rows.push(
                h_flex()
                    .gap_3()
                    .py_1()
                    .items_center()
                    .child(div().w(px(16.)).text_color(color).child(mark))
                    .child(div().w(px(120.)).font_medium().child(s.name.clone()))
                    .child(
                        div()
                            .flex_1()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child(s.detail.clone()),
                    )
                    .into_any_element(),
            );
        }

        let (verdict, vcolor) = if !r.all_ok() {
            (crate::tr!("Finished with failures"), theme.danger)
        } else if r.signed {
            (crate::tr!("All wiped · receipt signed"), theme.green)
        } else {
            (
                crate::tr!("All wiped · receipt unsigned"),
                theme.muted_foreground,
            )
        };

        let save_btn = Button::new("off-save")
            .label(crate::tr!("Save receipt (JSON)"))
            .outline()
            .on_click(cx.listener(|this, _, window, cx| this.save_receipt(window, cx)));

        let mut col = v_flex()
            .gap_3()
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(div().w(px(10.)).h(px(10.)).rounded_full().bg(vcolor))
                    .child(div().font_semibold().text_color(vcolor).child(verdict)),
            )
            .child(v_flex().gap_0p5().children(rows));
        if let Some(fp) = &r.fingerprint {
            col =
                col.child(
                    v_flex()
                        .gap_0p5()
                        .child(div().text_xs().text_color(theme.muted_foreground).child(
                            crate::tr!("Attestation fingerprint (match against inventory)"),
                        ))
                        .child(
                            div()
                                .font_family(crate::tr!("monospace"))
                                .text_xs()
                                .child(fp.clone()),
                        ),
                );
        }

        Card::new()
            .title(crate::tr!("Offboard receipt"))
            .description(crate::tr!(
                "Per-applet wipe results and the signed checkpoint"
            ))
            .icon(Icon::default().path("icons/scroll-text.svg"))
            .header_right(save_btn)
            .child(col)
            .into_any_element()
    }
}

impl Render for OffboardViewModel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let title: &str = crate::tr!("Offboard");
        let subtitle: &str = crate::tr!("Guided full-device wipe with a signed receipt.");

        if let Some((heading, body)) = self.gate(cx).message() {
            let theme = cx.theme();
            return PageView::build(title, subtitle, empty_state(heading, body, theme), theme)
                .into_any_element();
        }

        let serial = self.serial(cx);
        let report_card = self.report.as_ref().map(|_| self.report_card(cx));

        let offboard_btn = Button::new("off-run")
            .label(crate::tr!("Offboard device"))
            .danger()
            .disabled(self.loading)
            .on_click(cx.listener(|this, _, window, cx| this.open_confirm(window, cx)));

        let theme = cx.theme();

        let warn_card = Card::new()
            .title(crate::tr!("Decommission"))
            .description(crate::tr!("Wipe every applet and finish with a cryptographic receipt"))
            .icon(Icon::default().path("icons/trash-2.svg"))
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
                            .child(crate::tr!("Erases OTP, OATH, PIV, OpenPGP, the FIDO seed, passkeys, PINs, and org attestation. Irreversible. Needs the CCID interface and several touches.")),
                    )
                    .child(
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
                                    .child(div().font_medium().child(crate::tr!("Offboard {serial}", serial = serial)))
                                    .child(div().text_sm().text_color(theme.muted_foreground).child(crate::tr!("Wipe all applets, then sign a receipt"))),
                            )
                            .child(offboard_btn),
                    ),
            );

        let content = v_flex().gap_6().child(warn_card).children(report_card);
        PageView::build(title, subtitle, content, theme).into_any_element()
    }
}
