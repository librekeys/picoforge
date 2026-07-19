use crate::ui::components::{card::Card, page_view::PageView};
use crate::ui::models::device::{
    DeviceMethod, FirmwareType, LedColor, LedStatus, USB_CAP_FIDO2, USB_CAP_OATH, USB_CAP_OPENPGP,
    USB_CAP_OTP, USB_CAP_PIV, USB_CAP_U2F,
};
use crate::ui::screens::config::view_model::ConfigViewModel;
use gpui::*;
use gpui_component::{button::*, input::*, select::*, slider::*, switch::*, *};

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
                v_flex().gap_2().child("Vendor Preset").child(
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
                        v_flex().gap_2().child("Vendor ID (HEX)").child(
                            Input::new(&self.vid_input)
                                .font_family("Mono")
                                .bg(rgb(0x222225))
                                .disabled(hardware_config_disabled || !self.is_custom_vendor),
                        ),
                    )
                    .child(
                        v_flex().gap_2().child("Product ID (HEX)").child(
                            Input::new(&self.pid_input)
                                .font_family("Mono")
                                .bg(rgb(0x222225))
                                .disabled(hardware_config_disabled || !self.is_custom_vendor),
                        ),
                    ),
            )
            .child(div().h_px().bg(theme.border))
            .child(
                v_flex().gap_2().child("Product Name").child(
                    Input::new(&self.product_name_input)
                        .bg(rgb(0x222225))
                        .disabled(is_fido),
                ),
            );

        Card::new()
            .title("Identity")
            .description("USB Identification settings")
            .icon(Icon::default().path("icons/tag.svg"))
            .child(content)
    }

    fn render_led_card(
        &mut self,
        cx: &mut Context<Self>,
        is_fido: bool,
        hardware_config_disabled: bool,
    ) -> impl IntoElement {
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

        let content = v_flex()
            .gap_4()
            .child(
                h_flex()
                    .gap_4()
                    .flex_wrap()
                    .child(
                        v_flex().gap_2().flex_1().child("LED GPIO Pin").child(
                            Input::new(&self.led_gpio_input)
                                .bg(rgb(0x222225))
                                .disabled(hardware_config_disabled),
                        ),
                    )
                    .child(
                        v_flex().gap_2().flex_1().child("LED Driver").child(
                            Select::new(&self.led_driver_select)
                                .w_full()
                                .bg(rgb(0x222225))
                                .disabled(is_fido),
                        ),
                    ),
            )
            .child(div().h_px().bg(theme.border))
            .child(
                v_flex().gap_2().child("Brightness (0-15)").child(
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
                                .child(format!("Level {}", brightness)),
                        ),
                ),
            )
            .child(
                h_flex()
                    .items_center()
                    .justify_between()
                    .child(
                        v_flex().gap_0p5().child("LED Dimmable").child(
                            div()
                                .text_sm()
                                .text_color(theme.muted_foreground)
                                .child("Allow brightness adjustment"),
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
                        v_flex().gap_0p5().child("LED Steady Mode").child(
                            div()
                                .text_sm()
                                .text_color(theme.muted_foreground)
                                .child("Keep LED on constantly"),
                        ),
                    )
                    .child(
                        Switch::new("led-steady")
                            .checked(self.led_steady)
                            .disabled(hardware_config_disabled)
                            .on_click(steady_listener),
                    ),
            );

        Card::new()
            .title("LED Settings")
            .description("Adjust visual feedback behavior")
            .icon(Icon::default().path("icons/microchip.svg"))
            .child(content)
    }

    fn render_touch_card(&self, _theme: &Theme, is_fido: bool) -> impl IntoElement {
        let content = v_flex().gap_4().child(
            v_flex().gap_2().child("Touch Timeout (seconds)").child(
                Input::new(&self.touch_timeout_input)
                    .bg(rgb(0x222225))
                    .disabled(is_fido),
            ),
        );

        Card::new()
            .title("Touch & Timing")
            .description("Configure interaction timeouts")
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
                    v_flex().gap_0p5().child("Power Cycle on Reset").child(
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child("Restart device on reset"),
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
            .title("Device Options")
            .description("Toggle advanced features")
            .icon(Icon::default().path("icons/settings.svg"))
            .child(content)
    }

    fn render_curves_card(&mut self, cx: &mut Context<Self>, is_fido: bool) -> impl IntoElement {
        let theme = cx.theme();
        let mut rows = v_flex().gap_4();

        let curves = [
            ("curve-p256", "P-256 (secp256r1)", self.curve_p256),
            ("curve-p384", "P-384 (secp384r1)", self.curve_p384),
            ("curve-p521", "P-521 (secp521r1)", self.curve_p521),
            ("curve-k1", "secp256k1 (Bitcoin)", self.curve_secp256k1),
            ("curve-bp256", "Brainpool 256r1", self.curve_bp256),
            ("curve-bp384", "Brainpool 384r1", self.curve_bp384),
            ("curve-bp512", "Brainpool 512r1", self.curve_bp512),
            ("curve-ed25519", "Ed25519", self.curve_ed25519),
            ("curve-ed448", "Ed448", self.curve_ed448),
            ("curve-x25519", "X25519", self.curve_x25519),
            ("curve-x448", "X448", self.curve_x448),
        ];

        for (id, label, checked) in curves {
            let toggle_listener = cx.listener(move |this, checked, _, cx| {
                match id {
                    "curve-p256" => this.curve_p256 = *checked,
                    "curve-p384" => this.curve_p384 = *checked,
                    "curve-p521" => this.curve_p521 = *checked,
                    "curve-k1" => this.curve_secp256k1 = *checked,
                    "curve-bp256" => this.curve_bp256 = *checked,
                    "curve-bp384" => this.curve_bp384 = *checked,
                    "curve-bp512" => this.curve_bp512 = *checked,
                    "curve-ed25519" => this.curve_ed25519 = *checked,
                    "curve-ed448" => this.curve_ed448 = *checked,
                    "curve-x25519" => this.curve_x25519 = *checked,
                    "curve-x448" => this.curve_x448 = *checked,
                    _ => {}
                }
                cx.notify();
            });

            rows = rows.child(
                h_flex()
                    .items_center()
                    .justify_between()
                    .child(
                        v_flex().gap_0p5().child(label).child(
                            div()
                                .text_sm()
                                .text_color(theme.muted_foreground)
                                .child("Cryptographic curve"),
                        ),
                    )
                    .child(
                        Switch::new(id)
                            .checked(checked)
                            .disabled(is_fido)
                            .on_click(toggle_listener),
                    ),
            );
        }

        Card::new()
            .title("Supported Curves")
            .description("Enable or disable cryptographic curves for RS-Key")
            .icon(Icon::default().path("icons/shield.svg"))
            .child(rows)
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
                    v_flex().gap_0p5().child("Global Steady Mode").child(
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child("Keep status LEDs on constantly"),
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
                let mut b = this.led_status_brightness[i];
                b = b.saturating_sub(1);
                this.led_status_brightness[i] = b;
                cx.notify();
            });

            let inc_bright_listener = cx.listener(move |this, _, _, cx| {
                let mut b = this.led_status_brightness[i];
                if b < 15 {
                    b += 1;
                }
                this.led_status_brightness[i] = b;
                cx.notify();
            });

            let color_name = LedColor::from_u8(color_val)
                .map(|c| c.label())
                .unwrap_or("Unknown");

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
                                Button::new(gpui::SharedString::from(format!("color-btn-{}", i)))
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
                                Button::new(gpui::SharedString::from(format!("bdec-btn-{}", i)))
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
                                Button::new(gpui::SharedString::from(format!("binc-btn-{}", i)))
                                    .child("+")
                                    .custom(
                                        ButtonCustomVariant::new(cx)
                                            .color(rgb(0x1b1b1d).into())
                                            .hover(rgb(0x232325).into())
                                            .active(rgb(0x3f3f46).into())
                                            .border(theme.border),
                                    )
                                    .disabled(is_fido || brightness_val >= 15)
                                    .on_click(inc_bright_listener),
                            ),
                    ),
            );
        }

        rows = rows.child(div().h_px().bg(theme.border));
        rows = rows.child(
            h_flex().justify_end().child(
                Button::new("apply-rskey-leds")
                    .child("Save LED Status")
                    .custom(
                        ButtonCustomVariant::new(cx)
                            .color(rgb(0xe3e3e6).into())
                            .hover(rgb(0xcfcfd1).into())
                            .active(rgb(0xe3e3e6).into())
                            .foreground(rgb(0x4b4b4e).into()),
                    )
                    .disabled(is_fido || self.loading)
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.apply_rskey_led_settings(window, cx);
                    })),
            ),
        );

        Card::new()
            .title("Status LED Colors")
            .description("Configure LED colors and brightness per device state")
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
            ("FIDO2", USB_CAP_FIDO2),
            ("OATH", USB_CAP_OATH),
            ("PIV", USB_CAP_PIV),
            ("OpenPGP", USB_CAP_OPENPGP),
            ("U2F", USB_CAP_U2F),
            ("OTP", USB_CAP_OTP),
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
                                    "Supported"
                                } else {
                                    "Not Supported by Firmware"
                                },
                            ),
                        ))
                        .child(
                            Switch::new(gpui::SharedString::from(format!("app-toggle-{}", cap)))
                                .checked(is_enabled)
                                .disabled(is_fido || !is_supported)
                                .on_click(toggle_listener),
                        ),
                );
        }

        rows = rows.child(div().h_px().bg(theme.border));
        rows = rows.child(
            h_flex().justify_end().child(
                Button::new("apply-rskey-apps")
                    .child("Save USB Applications")
                    .custom(
                        ButtonCustomVariant::new(cx)
                            .color(rgb(0xe3e3e6).into())
                            .hover(rgb(0xcfcfd1).into())
                            .active(rgb(0xe3e3e6).into())
                            .foreground(rgb(0x4b4b4e).into()),
                    )
                    .disabled(is_fido || self.loading)
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.apply_rskey_apps_settings(window, cx);
                    })),
            ),
        );

        Card::new()
            .title("USB Applications")
            .description("Enable or disable specific USB features")
            .icon(Icon::default().path("icons/microchip.svg"))
            .child(rows)
    }

    fn render_rskey_usb_itf_card(
        &mut self,
        cx: &mut Context<Self>,
        is_fido: bool,
    ) -> impl IntoElement {
        let theme = cx.theme();
        let mut rows = v_flex().gap_4();

        let interfaces = [
            ("CCID (Smart Card)", 0x01u8),
            ("WCID (WebUSB)", 0x02u8),
            ("HID (FIDO)", 0x04u8),
            ("KB (Keyboard)", 0x08u8),
            ("LWIP", 0x10u8),
        ];

        let current_mask = self.enabled_usb_itf.unwrap_or(0x1F);

        for (name, bit) in interfaces {
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
                                .child(if is_ccid {
                                    "Required for Rescue Applet"
                                } else {
                                    "USB Endpoint"
                                }),
                        ),
                    )
                    .child(
                        Switch::new(gpui::SharedString::from(format!("usb-itf-toggle-{}", bit)))
                            .checked(is_enabled || is_ccid)
                            .disabled(is_fido || is_ccid)
                            .on_click(toggle_listener),
                    ),
            );
        }

        Card::new()
            .title("Hardware Endpoints")
            .description("Toggle low-level USB interfaces")
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
                "Configuration",
                "Customize device settings and behavior.",
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
                            .child("No Device Connected"),
                    ),
                theme,
            )
            .into_any_element();
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
            .render_led_card(cx, is_fido_no_rskey, hardware_config_disabled)
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

        let mut inner = v_flex()
            .gap_6()
            .child(identity_card)
            .child(led_card)
            .child(touch_card)
            .child(options_card);

        if is_rskey {
            inner = inner
                .child(self.render_curves_card(cx, false))
                .child(self.render_rskey_led_card(cx, false))
                .child(self.render_rskey_apps_card(cx, false))
                .child(self.render_rskey_usb_itf_card(cx, false));
        }

        inner = inner.child(
            h_flex().justify_end().pt_4().child(
                Button::new("apply-changes")
                    .icon(Icon::default().path("icons/save.svg"))
                    .child("Apply Changes")
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
            "Configuration",
            "Customize device settings and behavior.",
            inner,
            theme,
        )
        .into_any_element()
    }
}
