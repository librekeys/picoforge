//! Attestation screen rendering.

use crate::ui::components::button::PFButton;
use crate::ui::components::card::Card;
use crate::ui::components::page_view::PageView;
use crate::ui::screens::attestation::view_model::AttestationViewModel;
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

impl AttestationViewModel {
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

impl Render for AttestationViewModel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let title: &str = crate::tr!("Attestation");
        let subtitle: &str = crate::tr!("Organisation (enterprise) attestation key and chain.");

        if let Some((heading, body)) = self.gate(cx).message() {
            let theme = cx.theme();
            return PageView::build(title, subtitle, empty_state(heading, body, theme), theme)
                .into_any_element();
        }

        let status = self.status.clone();
        let installed = status.as_ref().map(|s| s.installed).unwrap_or(false);
        let theme = cx.theme();

        let refresh_btn = Button::new("att-refresh")
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
        let import_btn = PFButton::new(if installed {
            crate::tr!("Replace")
        } else {
            crate::tr!("Import")
        })
        .id("att-import")
        .with_colors(rgb(0x222225), rgb(0x2a2a2d), rgb(0x333336))
        .disabled(self.loading)
        .on_click(cx.listener(|this, _, window, cx| this.open_import(window, cx)));
        let clear_btn = Button::new("att-clear")
            .label(crate::tr!("Remove"))
            .danger()
            .disabled(self.loading || !installed)
            .on_click(cx.listener(|this, _, window, cx| this.open_clear(window, cx)));

        let status_card = {
            let body = match &status {
                Some(s) => {
                    let mut col = v_flex().gap_2().child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(
                                div()
                                    .text_color(if s.installed {
                                        theme.green
                                    } else {
                                        theme.muted_foreground
                                    })
                                    .child("●"),
                            )
                            .child(div().text_sm().child(if s.installed {
                                crate::tr!("Org attestation installed")
                            } else {
                                crate::tr!("Not installed — self-signed device certificate in use")
                            })),
                    );
                    if let Some(h) = &s.chain_hash {
                        col = col.child(
                            v_flex()
                                .gap_0p5()
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(theme.muted_foreground)
                                        .child(crate::tr!("Chain hash")),
                                )
                                .child(
                                    div()
                                        .font_family(crate::tr!("monospace"))
                                        .text_xs()
                                        .child(h.clone()),
                                ),
                        );
                    }
                    col.into_any_element()
                }
                None => div()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child(crate::tr!("Reading attestation state…"))
                    .into_any_element(),
            };
            Card::new()
                .title(crate::tr!("Attestation status"))
                .description(crate::tr!(
                    "Whether an org attestation key + chain is installed"
                ))
                .icon(Icon::default().path("icons/building-2.svg"))
                .header_right(refresh_btn)
                .child(body)
        };

        let actions_card = Card::new()
            .title(crate::tr!("Manage"))
            .description(crate::tr!("Provision or remove the org attestation"))
            .icon(Icon::default().path("icons/shield-check.svg"))
            .child(
                v_flex()
                    .gap_2()
                    .child(self.action_row(
                        crate::tr!("Import key + chain"),
                        crate::tr!("P-256 key (PEM/DER) and certificate chain (PIN or touch)"),
                        import_btn,
                        theme,
                    ))
                    .child(self.action_row(
                        crate::tr!("Remove attestation"),
                        crate::tr!("Revert to the self-signed device certificate"),
                        clear_btn,
                        theme,
                    )),
            );

        let content = v_flex().gap_6().child(status_card).child(actions_card);
        PageView::build(title, subtitle, content, theme).into_any_element()
    }
}
