//! Slots (OTP) screen rendering.

use crate::ui::components::button::PFButton;
use crate::ui::components::card::Card;
use crate::ui::components::page_view::PageView;
use crate::ui::models::device::otp;
use crate::ui::screens::slots::view_model::SlotsViewModel;
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

impl SlotsViewModel {
    fn render_slot_card(&self, slot: u8, cx: &mut Context<Self>) -> AnyElement {
        let info = self.slot_info(slot);
        let configured = info.configured();
        let theme = cx.theme();

        let status_text = if configured {
            let mut s = info.kind.label().to_string();
            if info.touch {
                s.push_str(crate::tr!(" · touch"));
            }
            s
        } else {
            crate::tr!("Empty").to_string()
        };

        let program_btn = PFButton::new(if configured {
            crate::tr!("Reprogram")
        } else {
            crate::tr!("Program")
        })
        .id(crate::tr!("prog-{slot}", slot = slot))
        .with_colors(rgb(0x222225), rgb(0x2a2a2d), rgb(0x333336))
        .disabled(self.loading)
        .on_click(cx.listener(move |this, _, window, cx| {
            this.open_program_dialog(slot, window, cx);
        }));
        let test_btn = (info.kind == otp::SlotType::ChallengeResponse).then(|| {
            Button::new(SharedString::from(crate::tr!("test-{slot}", slot = slot)))
                .label(crate::tr!("Test"))
                .ghost()
                .disabled(self.loading)
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.open_test_dialog(slot, window, cx);
                }))
        });
        let delete_btn = configured.then(|| {
            Button::new(SharedString::from(crate::tr!("del-{slot}", slot = slot)))
                .icon(Icon::default().path("icons/trash-2.svg"))
                .ghost()
                .disabled(self.loading)
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.open_delete_dialog(slot, window, cx);
                }))
        });

        h_flex()
            .justify_between()
            .items_center()
            .p_4()
            .border_1()
            .border_color(theme.border)
            .rounded_lg()
            .child(
                v_flex()
                    .gap_0p5()
                    .child(
                        div()
                            .font_medium()
                            .child(crate::tr!("Slot {slot}", slot = slot)),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(if configured {
                                theme.foreground
                            } else {
                                theme.muted_foreground
                            })
                            .child(status_text),
                    ),
            )
            .child(
                h_flex()
                    .gap_2()
                    .child(program_btn)
                    .children(test_btn)
                    .children(delete_btn),
            )
            .into_any_element()
    }
}

impl Render for SlotsViewModel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let title: &str = crate::tr!("Slots");
        let subtitle: &str = crate::tr!("Configurable OTP slots (Yubico OTP protocol).");

        if let Some((heading, body)) = self.gate(cx).message() {
            let theme = cx.theme();
            return PageView::build(title, subtitle, empty_state(heading, body, theme), theme)
                .into_any_element();
        }

        let count = self.slot_count(cx);
        let mut cards = Vec::with_capacity(count as usize);
        for slot in 1..=count {
            cards.push(self.render_slot_card(slot, cx));
        }

        let theme = cx.theme();
        let swap_btn = PFButton::new(crate::tr!("Swap 1 ↔ 2"))
            .id("swap-slots")
            .with_colors(rgb(0x222225), rgb(0x2a2a2d), rgb(0x333336))
            .disabled(self.loading)
            .on_click(cx.listener(|this, _, window, cx| this.open_swap_dialog(window, cx)));
        let refresh_btn = Button::new("refresh-slots")
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
        let toolbar = h_flex().gap_2().child(swap_btn).child(refresh_btn);

        let slots_card = Card::new()
            .title(crate::tr!("Slots"))
            .description(crate::tr!("{count} configurable slots", count = count))
            .icon(Icon::default().path("icons/touch-app.svg"))
            .header_right(toolbar)
            .child(v_flex().gap_2().children(cards));

        let content = v_flex().gap_6().child(slots_card);
        PageView::build(title, subtitle, content, theme).into_any_element()
    }
}
