use crate::ui::components::{card::Card, page_view::PageView};
use crate::ui::models::device::{
    DeviceMethod, FirmwareType, LedColor, LedStatus, USB_CAP_FIDO2, USB_CAP_OATH, USB_CAP_OPENPGP,
    USB_CAP_OTP, USB_CAP_PIV, USB_CAP_U2F,
};
use crate::ui::screens::config::view_model::ConfigViewModel;
use gpui::*;
use gpui_component::{button::*, input::*, select::*, slider::*, switch::*, *};

/// Per-status LED brightness is a full u8 on the device (0-255, 0 = off). The
/// +/- steppers move in coarse steps (15 divides 255 evenly) so the whole range
/// stays reachable without hundreds of clicks.
const LED_BRIGHTNESS_MAX: u8 = 255;
const LED_BRIGHTNESS_STEP: u8 = 15;

impl ConfigViewModel {
    fn render_identity_card(
        &self,
        theme: &Theme,
        is_fido: bool,
        hardware_config_disabled: bool,
    ) -> impl IntoElement {
        let content = v_flex()
            .gap_4()
            .child(
                v_flex().gap_2().child(crate::tr!("Vendor Preset")).child(
                    Select::new(&self.vendor_select)
                        .bg(rgb(0x222225))
                        .w_full()
                        .disabled(hardware_config_disabled),
                ),
            )
            .child(
                div()
                    .grid()
                    .grid_cols(2)
                    .gap_4()
                    .child(
                        v_flex().gap_2().child(crate::tr!("Vendor ID (HEX)")).child(
                            Input::new(&self.vid_input)
                                .font_family("Mono")
                                .bg(rgb(0x222225))
                                .disabled(hardware_config_disabled || !self.is_custom_vendor),
                        ),
                    )
                    .child(
                        v_flex()
                            .gap_2()
                            .child(crate::tr!("Product ID (HEX)"))
                            .child(
                                Input::new(&self.pid_input)
                                    .font_family("Mono")
                                    .bg(rgb(0x222225))
                                    .disabled(hardware_config_disabled || !self.is_custom_vendor),
                            ),
                    ),
            )
            .child(div().h_px().bg(theme.border))
            .child(
                div()
                    .grid()
                    .grid_cols(2)
                    .gap_4()
                    .child(
                        v_flex().gap_2().child(crate::tr!("Product Name")).child(
                            Input::new(&self.product_name_input)
                                .bg(rgb(0x222225))
                                .disabled(is_fido),
                        ),
                    )
                    .child(
                        v_flex().gap_2().child(crate::tr!("Manufacturer")).child(
                            Input::new(&self.manufacturer_input)
                                .bg(rgb(0x222225))
                                .disabled(is_fido),
                        ),
                    ),
            );

        Card::new()
            .title(crate::tr!("Identity"))
            .description(crate::tr!("USB Identification settings"))
            .icon(Icon::default().path("icons/tag.svg"))
            .child(content)
    }

    fn render_led_card(
        &mut self,
        cx: &mut Context<Self>,
        is_fido: bool,
        is_rskey: bool,
        hardware_config_disabled: bool,
    ) -> impl IntoElement {
        // GPIO pin + driver are the LED hardware topology — always shown.
        let mut content = v_flex().gap_4().child(
            div()
                .grid()
                .grid_cols(2)
                .gap_4()
                .child(
                    v_flex().gap_2().child(crate::tr!("LED GPIO Pin")).child(
                        Input::new(&self.led_gpio_input)
                            .bg(rgb(0x222225))
                            .disabled(hardware_config_disabled),
                    ),
                )
                .child(
                    v_flex().gap_2().child(crate::tr!("LED Driver")).child(
                        Select::new(&self.led_driver_select)
                            .w_full()
                            .bg(rgb(0x222225))
                            .disabled(is_fido),
                    ),
                ),
        );

        // Colour order is an RS-Key extension (phy tag 0x0D); pico-fido ignores
        // it, so only surface it for RS-Key. Fixes red/green swap on GRB panels.
        if is_rskey {
            content = content.child(
                v_flex()
                    .gap_2()
                    .child(crate::tr!("LED Colour Order"))
                    .child(
                        Select::new(&self.led_order_select)
                            .w_full()
                            .bg(rgb(0x222225))
                            .disabled(hardware_config_disabled),
                    ),
            );
        }

        // Global brightness / dimmable / steady live in the phy record. On RS-Key
        // the per-status EF_LED_CONF (Status LED Colors card) overrides them at
        // boot, so showing them here too would be duplicate, dead controls.
        if !is_rskey {
            let dim_listener = cx.listener(|this, checked, _, cx| {
                this.led_dimmable = *checked;
                cx.notify();
            });
            let steady_listener = cx.listener(|this, checked, _, cx| {
                this.led_steady = *checked;
                cx.notify();
            });
            let theme = cx.theme();
            let brightness = self.led_brightness_slider.read(cx).value().start() as i32;

            content = content
                .child(div().h_px().bg(theme.border))
                .child(
                    v_flex()
                        .gap_2()
                        .child(crate::tr!("Brightness (0-15)"))
                        .child(
                            h_flex()
                                .items_center()
                                .gap_4()
                                .child(
                                    Slider::new(&self.led_brightness_slider)
                                        .flex_1()
                                        .disabled(hardware_config_disabled),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(theme.muted_foreground)
                                        .child(crate::tr!("Level {}", brightness)),
                                ),
                        ),
                )
                .child(
                    h_flex()
                        .items_center()
                        .justify_between()
                        .child(
                            v_flex().gap_0p5().child(crate::tr!("LED Dimmable")).child(
                                div()
                                    .text_sm()
                                    .text_color(theme.muted_foreground)
                                    .child(crate::tr!("Allow brightness adjustment")),
                            ),
                        )
                        .child(
                            Switch::new("led-dimmable")
                                .checked(self.led_dimmable)
                                .disabled(hardware_config_disabled)
                                .on_click(dim_listener),
                        ),
                )
                .child(
                    h_flex()
                        .items_center()
                        .justify_between()
                        .child(
                            v_flex()
                                .gap_0p5()
                                .child(crate::tr!("LED Steady Mode"))
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(theme.muted_foreground)
                                        .child(crate::tr!("Keep LED on constantly")),
                                ),
                        )
                        .child(
                            Switch::new("led-steady")
                                .checked(self.led_steady)
                                .disabled(hardware_config_disabled)
                                .on_click(steady_listener),
                        ),
                );
        }

        Card::new()
            .title(crate::tr!("LED Settings"))
            .description(crate::tr!("Adjust visual feedback behavior"))
            .icon(Icon::default().path("icons/microchip.svg"))
            .child(content)
    }

    fn render_touch_card(&self, _theme: &Theme, is_fido: bool) -> impl IntoElement {
        let content = v_flex().gap_4().child(
            v_flex()
                .gap_2()
                .child(crate::tr!("Touch Timeout (seconds)"))
                .child(
                    Input::new(&self.touch_timeout_input)
                        .bg(rgb(0x222225))
                        .disabled(is_fido),
                ),
        );

        Card::new()
            .title(crate::tr!("Touch & Timing"))
            .description(crate::tr!("Configure interaction timeouts"))
            .icon(Icon::default().path("icons/settings.svg"))
            .child(content)
    }

    fn render_options_card(
        &mut self,
        cx: &mut Context<Self>,
        hardware_config_disabled: bool,
    ) -> impl IntoElement {
        let power_cycle_listener = cx.listener(|this, checked, _, cx| {
            this.power_cycle = *checked;
            cx.notify();
        });

        let theme = cx.theme();

        let content = v_flex().gap_4().child(
            h_flex()
                .items_center()
                .justify_between()
                .child(
                    v_flex()
                        .gap_0p5()
                        .child(crate::tr!("Power Cycle on Reset"))
                        .child(
                            div()
                                .text_sm()
                                .text_color(theme.muted_foreground)
                                .child(crate::tr!("Restart device on reset")),
                        ),
                )
                .child(
                    Switch::new("power-cycle")
                        .checked(self.power_cycle)
                        .disabled(hardware_config_disabled)
                        .on_click(power_cycle_listener),
                ),
        );

        Card::new()
            .title(crate::tr!("Device Options"))
            .description(crate::tr!("Toggle advanced features"))
            .icon(Icon::default().path("icons/settings.svg"))
            .child(content)
    }

    fn render_rskey_led_card(&mut self, cx: &mut Context<Self>, is_fido: bool) -> impl IntoElement {
        let theme = cx.theme();
        let mut rows = v_flex().gap_4();

        let steady_listener = cx.listener(|this, checked, _, cx| {
            this.led_status_steady = *checked;
            cx.notify();
        });

        rows = rows.child(
            h_flex()
                .items_center()
                .justify_between()
                .child(
                    v_flex()
                        .gap_0p5()
                        .child(crate::tr!("Global Steady Mode"))
                        .child(
                            div()
                                .text_sm()
                                .text_color(theme.muted_foreground)
                                .child(crate::tr!("Keep status LEDs on constantly")),
                        ),
                )
                .child(
                    Switch::new("rskey-led-steady")
                        .checked(self.led_status_steady)
                        .disabled(is_fido)
                        .on_click(steady_listener),
                ),
        );

        rows = rows.child(div().h_px().bg(theme.border));

        for (i, status) in LedStatus::all().iter().enumerate() {
            let color_val = self.led_status_colors[i];
            let brightness_val = self.led_status_brightness[i];

            let cycle_color_listener = cx.listener(move |this, _, _, cx| {
                let mut c = this.led_status_colors[i];
                c = (c + 1) % LedColor::all().len() as u8;
                this.led_status_colors[i] = c;
                cx.notify();
            });

            let dec_bright_listener = cx.listener(move |this, _, _, cx| {
                let b = this.led_status_brightness[i];
                this.led_status_brightness[i] = b.saturating_sub(LED_BRIGHTNESS_STEP);
                cx.notify();
            });

            let inc_bright_listener = cx.listener(move |this, _, _, cx| {
                let b = this.led_status_brightness[i];
                this.led_status_brightness[i] = b.saturating_add(LED_BRIGHTNESS_STEP);
                cx.notify();
            });

            let color_name = LedColor::from_u8(color_val)
                .map(|c| c.label())
                .unwrap_or(crate::tr!("Unknown"));

            rows = rows.child(
                h_flex()
                    .items_center()
                    .justify_between()
                    .child(div().w_24().child(status.label()))
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(
                                Button::new(gpui::SharedString::from(crate::tr!(
                                    "color-btn-{}",
                                    i
                                )))
                                .child(color_name)
                                .custom(
                                    ButtonCustomVariant::new(cx)
                                        .color(rgb(0x27272a).into())
                                        .hover(rgb(0x3f3f46).into())
                                        .active(rgb(0x52525b).into())
                                        .border(theme.border),
                                )
                                .disabled(is_fido)
                                .on_click(cycle_color_listener),
                            )
                            .child(div().w_4())
                            .child(
                                Button::new(gpui::SharedString::from(crate::tr!("bdec-btn-{}", i)))
                                    .child("-")
                                    .custom(
                                        ButtonCustomVariant::new(cx)
                                            .color(rgb(0x1b1b1d).into())
                                            .hover(rgb(0x232325).into())
                                            .active(rgb(0x3f3f46).into())
                                            .border(theme.border),
                                    )
                                    .disabled(is_fido || brightness_val == 0)
                                    .on_click(dec_bright_listener),
                            )
                            .child(
                                div()
                                    .w_8()
                                    .flex()
                                    .justify_center()
                                    .child(brightness_val.to_string()),
                            )
                            .child(
                                Button::new(gpui::SharedString::from(crate::tr!("binc-btn-{}", i)))
                                    .child("+")
                                    .custom(
                                        ButtonCustomVariant::new(cx)
                                            .color(rgb(0x1b1b1d).into())
                                            .hover(rgb(0x232325).into())
                                            .active(rgb(0x3f3f46).into())
                                            .border(theme.border),
                                    )
                                    .disabled(is_fido || brightness_val == LED_BRIGHTNESS_MAX)
                                    .on_click(inc_bright_listener),
                            ),
                    ),
            );
        }

        Card::new()
            .title(crate::tr!("Status LED Colors"))
            .description(crate::tr!(
                "Configure LED colors and brightness per device state"
            ))
            .icon(Icon::default().path("icons/palette.svg"))
            .child(rows)
    }

    fn render_rskey_apps_card(
        &mut self,
        cx: &mut Context<Self>,
        is_fido: bool,
    ) -> impl IntoElement {
        let theme = cx.theme();
        let mut rows = v_flex().gap_4();

        let apps = [
            (crate::tr!("FIDO2"), USB_CAP_FIDO2),
            (crate::tr!("OATH"), USB_CAP_OATH),
            (crate::tr!("PIV"), USB_CAP_PIV),
            (crate::tr!("OpenPGP"), USB_CAP_OPENPGP),
            (crate::tr!("U2F"), USB_CAP_U2F),
            (crate::tr!("OTP"), USB_CAP_OTP),
        ];

        for (name, cap) in apps {
            let is_supported = (self.usb_apps_supported & cap) != 0;
            let is_enabled = (self.usb_apps_enabled & cap) != 0;

            let toggle_listener = cx.listener(move |this, checked, _, cx| {
                if *checked {
                    this.usb_apps_enabled |= cap;
                } else {
                    this.usb_apps_enabled &= !cap;
                }
                cx.notify();
            });

            rows =
                rows.child(
                    h_flex()
                        .items_center()
                        .justify_between()
                        .child(v_flex().gap_0p5().child(name).child(
                            div().text_sm().text_color(theme.muted_foreground).child(
                                if is_supported {
                                    crate::tr!("Supported")
                                } else {
                                    crate::tr!("Not Supported by Firmware")
                                },
                            ),
                        ))
                        .child(
                            Switch::new(gpui::SharedString::from(crate::tr!("app-toggle-{}", cap)))
                                .checked(is_enabled)
                                .disabled(is_fido || !is_supported)
                                .on_click(toggle_listener),
                        ),
                );
        }

        Card::new()
            .title(crate::tr!("USB Applications"))
            .description(crate::tr!("Enable or disable specific USB features"))
            .icon(Icon::default().path("icons/microchip.svg"))
            .child(rows)
    }

    fn render_rskey_usb_itf_card(
        &mut self,
        cx: &mut Context<Self>,
        is_fido: bool,
    ) -> impl IntoElement {
        let theme = cx.theme();
        // Only the interfaces the firmware actually instantiates (USB_ITF_SUPPORTED
        // = CCID | HID | KB). WCID (WebUSB) and LWIP are pico-fido concepts RS-Key
        // never builds, so toggling them would be a no-op — don't offer them.
        let mut rows = v_flex().gap_4().child(
            div()
                .text_sm()
                .text_color(rgb(0xf59e0b))
                .w_full()
                .max_w(px(800.0))
                .child(crate::tr!("Advanced. HID off disables all FIDO2/U2F; CCID off disables every smart-card app (and the rescue applet). The firmware always keeps one of them, so you can't lock yourself out here.")),
        );

        let interfaces = [
            (
                crate::tr!("CCID (Smart Card)"),
                0x01u8,
                crate::tr!("Required for the rescue applet and all smart-card apps"),
            ),
            (
                crate::tr!("HID (FIDO)"),
                0x04u8,
                crate::tr!("FIDO/CTAP transport — off disables all FIDO2 and U2F"),
            ),
            (
                crate::tr!("KB (Keyboard)"),
                0x08u8,
                crate::tr!("OTP keyboard — Yubico OTP and static-password typing"),
            ),
        ];

        let current_mask = self.enabled_usb_itf.unwrap_or(0x1F);

        for (name, bit, desc) in interfaces {
            let is_enabled = (current_mask & bit) != 0;
            let is_ccid = bit == 0x01;

            let toggle_listener = cx.listener(move |this, checked, _, cx| {
                let mut mask = this.enabled_usb_itf.unwrap_or(0x1F);
                if *checked {
                    mask |= bit;
                } else {
                    mask &= !bit;
                }

                if bit == 0x01 {
                    mask |= 0x01;
                }

                this.enabled_usb_itf = Some(mask);
                cx.notify();
            });

            rows = rows.child(
                h_flex()
                    .items_center()
                    .justify_between()
                    .child(
                        v_flex().gap_0p5().child(name).child(
                            div()
                                .text_sm()
                                .text_color(theme.muted_foreground)
                                .w_full()
                                .max_w(px(600.0))
                                .child(desc),
                        ),
                    )
                    .child(
                        Switch::new(gpui::SharedString::from(crate::tr!(
                            "usb-itf-toggle-{}",
                            bit
                        )))
                        .checked(is_enabled || is_ccid)
                        .disabled(is_fido || is_ccid)
                        .on_click(toggle_listener),
                    ),
            );
        }

        Card::new()
            .title(crate::tr!("Hardware Endpoints"))
            .description(crate::tr!("Toggle low-level USB interfaces"))
            .icon(Icon::default().path("icons/cpu.svg"))
            .child(rows)
    }
}

impl Render for ConfigViewModel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let has_device = self.device.read(cx).status.is_some();

        if !has_device {
            let theme = cx.theme();
            return PageView::build(
                crate::tr!("Configuration"),
                crate::tr!("Customize device settings and behavior."),
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .h_64()
                    .border_1()
                    .border_color(theme.border)
                    .rounded_xl()
                    .child(
                        div()
                            .text_color(theme.muted_foreground)
                            .child(crate::tr!("No Device Connected")),
                    ),
                theme,
            );
        }

        let device = self.device.read(cx);
        let status = device.status.clone();
        let is_fido = status.as_ref().map(|s| s.method.clone()) == Some(DeviceMethod::Fido);
        let is_rskey = status.as_ref().map(|s| &s.firmware_type) == Some(&FirmwareType::RSKey);

        let supports_legacy_fido_config = status
            .as_ref()
            .map(ConfigViewModel::status_supports_legacy_fido_config)
            .unwrap_or(false);

        let hardware_config_disabled = is_fido && !supports_legacy_fido_config && !is_rskey;

        // RS-Key supports full config read/write over FIDO via CONFIG_READ/CONFIG_WRITE.
        // Other firmwares (pico-fido) don't: product name, LED driver, curves, etc.
        let is_fido_no_rskey = is_fido && !is_rskey;

        let led_card = self
            .render_led_card(cx, is_fido_no_rskey, is_rskey, hardware_config_disabled)
            .into_any_element();
        let options_card = self
            .render_options_card(cx, hardware_config_disabled)
            .into_any_element();

        let identity_card = self
            .render_identity_card(cx.theme(), is_fido_no_rskey, hardware_config_disabled)
            .into_any_element();
        let touch_card = self
            .render_touch_card(cx.theme(), is_fido_no_rskey)
            .into_any_element();

        let mut inner = v_flex().gap_6().w_full().child(identity_card);

        // RS-Key: put the functional config (which apps + transports are on)
        // right after Identity, before appearance/misc, so the panel reads
        // top-down by importance rather than burying it under the LED cards.
        // No curves card: the firmware ignores the phy ENABLED_CURVES tag
        // (curve support is compile-time), so exposing it would only mislead.
        if is_rskey {
            inner = inner
                .child(self.render_rskey_apps_card(cx, false))
                .child(self.render_rskey_usb_itf_card(cx, false));
        }

        inner = inner.child(led_card);

        if is_rskey {
            inner = inner.child(self.render_rskey_led_card(cx, false));
        }

        inner = inner.child(touch_card).child(options_card);

        inner = inner.child(
            h_flex().justify_end().pt_4().child(
                Button::new("apply-changes")
                    .icon(Icon::default().path("icons/save.svg"))
                    .child(crate::tr!("Apply Changes"))
                    .disabled(self.loading || hardware_config_disabled)
                    .custom(
                        ButtonCustomVariant::new(cx)
                            .color(rgb(0xe3e3e6).into())
                            .hover(rgb(0xcfcfd1).into())
                            .active(rgb(0xe3e3e6).into())
                            .foreground(rgb(0x4b4b4e).into()),
                    )
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.apply_changes(window, cx);
                    })),
            ),
        );

        let theme = cx.theme();
        PageView::build(
            crate::tr!("Configuration"),
            crate::tr!("Customize device settings and behavior."),
            inner,
            theme,
        )
    }
}
