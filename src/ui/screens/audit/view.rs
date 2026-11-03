//! Audit screen rendering.

use crate::ui::components::button::PFButton;
use crate::ui::components::card::Card;
use crate::ui::components::page_view::PageView;
use crate::ui::models::device::audit;
use crate::ui::screens::audit::view_model::AuditViewModel;
use gpui::*;
use gpui_component::{ActiveTheme, Icon, StyledExt, Theme, h_flex, v_flex};

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

fn mono(theme: &Theme, s: String) -> AnyElement {
    div()
        .font_family(crate::tr!("monospace"))
        .text_xs()
        .text_color(theme.muted_foreground)
        .child(s)
        .into_any_element()
}

fn short_hex(bytes: &[u8; 32]) -> String {
    let h = hex::encode(bytes);
    crate::tr!("{}…{}", &h[..8], &h[h.len() - 8..])
}

impl AuditViewModel {
    fn entry_row(entry: &audit::AuditEntry, theme: &Theme) -> AnyElement {
        h_flex()
            .gap_3()
            .py_1()
            .text_sm()
            .child(
                div()
                    .w(px(56.))
                    .text_color(theme.muted_foreground)
                    .child(entry.seq.to_string()),
            )
            .child(
                div()
                    .w(px(72.))
                    .text_color(theme.muted_foreground)
                    .child(crate::tr!("{:.1}s", entry.uptime_s())),
            )
            .child(div().w(px(160.)).font_medium().child(entry.event_label()))
            .child(
                div()
                    .w(px(40.))
                    .text_color(theme.muted_foreground)
                    .child(entry.aux.to_string()),
            )
            .child(
                div()
                    .flex_1()
                    .font_family(crate::tr!("monospace"))
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .child(entry.detail_hex()),
            )
            .into_any_element()
    }

    fn journal_body(&self, theme: &Theme) -> AnyElement {
        let Some(j) = &self.journal else {
            return div()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(crate::tr!(
                    "Read the journal to view the security-event log."
                ))
                .into_any_element();
        };

        let header = h_flex()
            .gap_3()
            .pb_1()
            .text_xs()
            .font_semibold()
            .text_color(theme.muted_foreground)
            .child(div().w(px(56.)).child(crate::tr!("seq")))
            .child(div().w(px(72.)).child(crate::tr!("uptime")))
            .child(div().w(px(160.)).child(crate::tr!("event")))
            .child(div().w(px(40.)).child(crate::tr!("aux")))
            .child(div().flex_1().child(crate::tr!("detail")));

        let mut rows = vec![header.into_any_element()];
        for e in &j.entries {
            rows.push(Self::entry_row(e, theme));
        }
        if j.entries.is_empty() {
            rows.push(
                div()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child(crate::tr!("No live entries in the window."))
                    .into_any_element(),
            );
        }

        v_flex()
            .gap_2()
            .child(
                v_flex()
                    .gap_1()
                    .child(div().text_sm().child(crate::tr!(
                        "Window [{}, {}) — {} entries, {} folded into the epoch",
                        j.start,
                        j.seq_next,
                        j.entries.len(),
                        j.start,
                    )))
                    .child(mono(theme, crate::tr!("epoch  {}", short_hex(&j.epoch))))
                    .child(mono(
                        theme,
                        crate::tr!("head   {}  (chain OK)", short_hex(&j.head)),
                    )),
            )
            .child(div().h(px(1.)).bg(theme.border))
            .child(v_flex().gap_0p5().children(rows))
            .into_any_element()
    }

    fn verify_body(&self, theme: &Theme) -> AnyElement {
        let Some(v) = &self.verification else {
            return div()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(crate::tr!("Verify a signed checkpoint to prove the journal is authentic and the device genuine."))
                .into_any_element();
        };

        let (label, color) = if v.authentic() {
            (crate::tr!("Authentic ✓"), theme.green)
        } else {
            (crate::tr!("Not trusted ✗"), theme.danger)
        };

        let kv = |k: &str, val: String| {
            v_flex()
                .gap_0p5()
                .child(
                    div()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child(k.to_string()),
                )
                .child(
                    div()
                        .font_family(crate::tr!("monospace"))
                        .text_xs()
                        .child(val),
                )
        };

        let expected_line = match v.expected_match {
            Some(true) => Some((
                crate::tr!("Pinned key"),
                crate::tr!("matches ✓").to_string(),
            )),
            Some(false) => Some((
                crate::tr!("Pinned key"),
                crate::tr!("MISMATCH ✗").to_string(),
            )),
            None => None,
        };

        let mut col = v_flex()
            .gap_3()
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(div().w(px(10.)).h(px(10.)).rounded_full().bg(color))
                    .child(div().font_semibold().text_color(color).child(label)),
            )
            .child(div().text_sm().child(crate::tr!(
                "Signature {} · chain head {} · checkpoint over seq {}",
                if v.signature_ok {
                    crate::tr!("OK")
                } else {
                    crate::tr!("INVALID")
                },
                if v.head_matches {
                    crate::tr!("bound")
                } else {
                    crate::tr!("MISMATCH")
                },
                v.seq_signed,
            )))
            .child(kv(crate::tr!("Attestation key"), v.pubkey_hex.clone()))
            .child(kv(
                crate::tr!("Fingerprint (pin later with Expected key)"),
                v.fingerprint.clone(),
            ));
        if let Some((k, val)) = expected_line {
            col = col.child(kv(k, val));
        }
        col.into_any_element()
    }
}

impl Render for AuditViewModel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let title: &str = crate::tr!("Audit");
        let subtitle: &str = crate::tr!("Tamper-evident security journal.");

        if let Some((heading, body)) = self.gate(cx).message() {
            let theme = cx.theme();
            return PageView::build(title, subtitle, empty_state(heading, body, theme), theme)
                .into_any_element();
        }

        let read_btn = PFButton::new(crate::tr!("Read journal"))
            .id("audit-read")
            .with_colors(rgb(0x222225), rgb(0x2a2a2d), rgb(0x333336))
            .disabled(self.loading)
            .on_click(cx.listener(|this, _, window, cx| this.open_read(window, cx)));
        let verify_btn = PFButton::new(crate::tr!("Verify"))
            .id("audit-verify")
            .with_colors(rgb(0x222225), rgb(0x2a2a2d), rgb(0x333336))
            .disabled(self.loading)
            .on_click(cx.listener(|this, _, window, cx| this.open_verify(window, cx)));
        let toggle_btn = match self.enabled {
            Some(true) => Some(
                PFButton::new(crate::tr!("Disable"))
                    .id("audit-disable")
                    .with_colors(rgb(0x222225), rgb(0x2a2a2d), rgb(0x333336))
                    .disabled(self.loading)
                    .on_click(
                        cx.listener(|this, _, window, cx| this.open_toggle(false, window, cx)),
                    ),
            ),
            Some(false) => Some(
                PFButton::new(crate::tr!("Enable"))
                    .id("audit-enable")
                    .with_colors(rgb(0x222225), rgb(0x2a2a2d), rgb(0x333336))
                    .disabled(self.loading)
                    .on_click(
                        cx.listener(|this, _, window, cx| this.open_toggle(true, window, cx)),
                    ),
            ),
            None => None,
        };

        let theme = cx.theme();
        let journal_body = self.journal_body(theme);
        let verify_body = self.verify_body(theme);

        let (dot, status_text) = match self.enabled {
            Some(true) => (
                theme.green,
                crate::tr!("On — recording security events to the key's flash."),
            ),
            Some(false) => (
                theme.muted_foreground,
                crate::tr!("Off — journalling is opt-in; nothing is being recorded."),
            ),
            None => (theme.muted_foreground, crate::tr!("Reading status…")),
        };
        let status_body = h_flex()
            .gap_2()
            .items_center()
            .child(div().w(px(10.)).h(px(10.)).rounded_full().bg(dot))
            .child(div().text_sm().child(status_text.to_string()));
        let status_card = {
            let mut c = Card::new()
                .title(crate::tr!("Journalling"))
                .description(crate::tr!(
                    "Turn the tamper-evident journal on or off (PIN + touch)"
                ))
                .icon(Icon::default().path("icons/book-open.svg"));
            if let Some(btn) = toggle_btn {
                c = c.header_right(btn);
            }
            c.child(status_body)
        };

        let journal_card = Card::new()
            .title(crate::tr!("Audit journal"))
            .description(crate::tr!(
                "Hash-chained security events (boots, FIDO ops, PIN, config)"
            ))
            .icon(Icon::default().path("icons/scroll-text.svg"))
            .header_right(read_btn)
            .child(journal_body);

        let verify_card = Card::new()
            .title(crate::tr!("Checkpoint verification"))
            .description(crate::tr!(
                "DEVK-signed proof of authenticity and device identity"
            ))
            .icon(Icon::default().path("icons/shield-check.svg"))
            .header_right(verify_btn)
            .child(verify_body);

        let content = v_flex()
            .gap_6()
            .child(status_card)
            .child(journal_card)
            .child(verify_card);
        PageView::build(title, subtitle, content, theme).into_any_element()
    }
}
