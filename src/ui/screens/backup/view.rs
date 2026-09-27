//! Backup screen rendering.

use crate::ui::components::button::PFButton;
use crate::ui::components::card::Card;
use crate::ui::components::page_view::PageView;
use crate::ui::screens::backup::view_model::BackupViewModel;
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

impl BackupViewModel {
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

    fn exported_card(&self, phrase: &str, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let copy = {
            let p = phrase.to_string();
            Button::new("bk-copy")
                .label(crate::tr!("Copy"))
                .ghost()
                .on_click(cx.listener(move |_, _, _, cx| {
                    cx.write_to_clipboard(ClipboardItem::new_string(p.clone()));
                }))
        };
        let clear = Button::new("bk-clear")
            .label(crate::tr!("Clear from screen"))
            .ghost()
            .on_click(cx.listener(|this, _, _, cx| this.clear_exported(cx)));

        Card::new()
            .title(crate::tr!("Recovery phrase"))
            .description(crate::tr!("Shown once — write it down now, then seal the window"))
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
                            .child(crate::tr!("Anyone with this phrase can clone your FIDO identity. Store it offline; never paste it into a website.")),
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

impl Render for BackupViewModel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let title: &str = crate::tr!("Backup");
        let subtitle: &str = crate::tr!("Wallet-style FIDO seed backup and restore.");

        if let Some((heading, body)) = self.gate(cx).message() {
            let theme = cx.theme();
            return PageView::build(title, subtitle, empty_state(heading, body, theme), theme)
                .into_any_element();
        }

        let status = self.status;
        let exported = self.exported.clone();

        let exported_card = exported.map(|p| self.exported_card(&p, cx));
        let theme = cx.theme();

        let refresh_btn = Button::new("bk-refresh")
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
        let export_btn = Button::new("bk-export")
            .label(crate::tr!("Export seed"))
            .danger()
            .disabled(self.loading)
            .on_click(cx.listener(|this, _, window, cx| this.open_export(window, cx)));
        let seal_btn = PFButton::new(crate::tr!("Seal window"))
            .id("bk-seal")
            .with_colors(rgb(0x222225), rgb(0x2a2a2d), rgb(0x333336))
            .disabled(self.loading)
            .on_click(cx.listener(|this, _, window, cx| this.open_finalize(window, cx)));
        let restore_btn = Button::new("bk-restore")
            .label(crate::tr!("Restore seed"))
            .danger()
            .disabled(self.loading)
            .on_click(cx.listener(|this, _, window, cx| this.open_restore(window, cx)));

        let status_card = {
            let body = match status {
                Some(s) => {
                    let yn = |b: bool| {
                        if b {
                            crate::tr!("yes")
                        } else {
                            crate::tr!("no")
                        }
                    };
                    let export_state = if s.sealed {
                        crate::tr!("sealed (export refused until a factory reset)")
                    } else if s.has_seed {
                        crate::tr!("open — seed can be exported once")
                    } else {
                        crate::tr!("no seed present")
                    };
                    v_flex()
                        .gap_2()
                        .child(
                            div()
                                .text_sm()
                                .child(crate::tr!("Seed present: {}", yn(s.has_seed))),
                        )
                        .child(div().text_sm().child(crate::tr!(
                            "Export window: {export_state}",
                            export_state = export_state
                        )))
                        .into_any_element()
                }
                None => div()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child(crate::tr!("Reading backup state…"))
                    .into_any_element(),
            };
            Card::new()
                .title(crate::tr!("Backup status"))
                .description(crate::tr!(
                    "Whether a seed is present and the export window is open"
                ))
                .icon(Icon::default().path("icons/cpu.svg"))
                .header_right(refresh_btn)
                .child(body)
        };

        let export_card = Card::new()
            .title(crate::tr!("Export"))
            .description(crate::tr!(
                "Reveal the seed as a 24-word phrase, then seal the window"
            ))
            .icon(Icon::default().path("icons/lock-open.svg"))
            .child(
                v_flex()
                    .gap_2()
                    .child(self.action_row(
                        crate::tr!("Export seed"),
                        crate::tr!("Show the recovery phrase (offline; PIN or touch)"),
                        export_btn,
                        theme,
                    ))
                    .child(self.action_row(
                        crate::tr!("Seal export window"),
                        crate::tr!("Refuse further exports until a factory reset"),
                        seal_btn,
                        theme,
                    )),
            );

        let restore_card = Card::new()
            .title(crate::tr!("Restore"))
            .description(crate::tr!("Install a seed from a 24-word phrase"))
            .icon(Icon::default().path("icons/lock.svg"))
            .child(self.action_row(
                crate::tr!("Restore seed"),
                crate::tr!("Replace the FIDO identity from a recovery phrase"),
                restore_btn,
                theme,
            ));

        let content = v_flex()
            .gap_6()
            .child(status_card)
            .children(exported_card)
            .child(export_card)
            .child(restore_card);

        PageView::build(title, subtitle, content, theme).into_any_element()
    }
}
