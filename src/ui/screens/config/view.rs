use crate::hal::rescue::constants::{
    LedColor, LedStatus, USB_CAP_FIDO2, USB_CAP_OATH, USB_CAP_OPENPGP, USB_CAP_OTP, USB_CAP_PIV,
    USB_CAP_U2F,
};
use crate::hal::types::DeviceMethod;
use crate::ui::components::{card::Card, page_view::PageView};
use crate::ui::screens::config::view_model::ConfigView;
use gpui::*;
use gpui_component::button::{ButtonCustomVariant, ButtonVariants};
use gpui_component::{
    ActiveTheme, Disableable, Icon, Theme, button::Button, input::Input, select::Select,
    slider::Slider, switch::Switch, v_flex,
};

impl ConfigView {
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
                v_flex().gap_2().child("LED GPIO Pin").child(
                    Input::new(&self.led_gpio_input)
                        .bg(rgb(0x222225))
                        .disabled(hardware_config_disabled),
                ),
            )
            .child(
                v_flex().gap_2().child("LED Driver").child(
                    Select::new(&self.led_driver_select)
                        .w_full()
                        .bg(rgb(0x222225))
                        .disabled(is_fido),
                ),
            )
            .child(div().h_px().bg(theme.border))
            .child(
                v_flex().gap_2().child("Brightness (0-15)").child(
                    gpui_component::h_flex()
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
                gpui_component::h_flex()
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
                gpui_component::h_flex()
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
        is_fido: bool,
        hardware_config_disabled: bool,
    ) -> impl IntoElement {
        let power_cycle_listener = cx.listener(|this, checked, _, cx| {
            this.power_cycle = *checked;
            cx.notify();
        });

        let secp_listener = cx.listener(|this, checked, _, cx| {
            this.enable_secp256k1 = *checked;
            cx.notify();
        });

        let theme = cx.theme();

        let content = v_flex()
            .gap_4()
            .child(
                gpui_component::h_flex()
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
            )
            .child(
                gpui_component::h_flex()
                    .items_center()
                    .justify_between()
                    .child(
                        v_flex().gap_0p5().child("Enable Secp256k1").child(
                            div()
                                .text_sm()
                                .text_color(theme.muted_foreground)
                                .child("Does not work on Android!"),
                        ),
                    )
                    .child(
                        Switch::new("enable-secp")
                            .checked(self.enable_secp256k1)
                            .disabled(is_fido)
                            .on_click(secp_listener),
                    ),
            );

        Card::new()
            .title("Device Options")
            .description("Toggle advanced features")
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
            gpui_component::h_flex()
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
                gpui_component::h_flex()
                    .items_center()
                    .justify_between()
                    .child(div().w_24().child(status.label()))
                    .child(
                        gpui_component::h_flex()
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
            gpui_component::h_flex().justify_end().child(
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
                    gpui_component::h_flex()
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
            gpui_component::h_flex().justify_end().child(
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
                gpui_component::h_flex()
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

impl Render for ConfigView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let has_device = self.device.read(cx).status.is_some();

        if !has_device {
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
        let supports_legacy_fido_config = status
            .as_ref()
            .map(ConfigView::status_supports_legacy_fido_config)
            .unwrap_or(false);
        let hardware_config_disabled = is_fido && !supports_legacy_fido_config;

        let led_card = self
            .render_led_card(cx, is_fido, hardware_config_disabled)
            .into_any_element();
        let options_card = self
            .render_options_card(cx, is_fido, hardware_config_disabled)
            .into_any_element();

        let identity_card = self
            .render_identity_card(cx.theme(), is_fido, hardware_config_disabled)
            .into_any_element();
        let touch_card = self
            .render_touch_card(cx.theme(), is_fido)
            .into_any_element();

        let is_wide = window.bounds().size.width > px(1100.0);
        let columns = if is_wide { 2 } else { 1 };

        let is_rskey = status.as_ref().map(|s| &s.firmware_type)
            == Some(&crate::hal::types::FirmwareType::RSKey);

        let mut grid_children = vec![identity_card, led_card, touch_card, options_card];

        if is_rskey {
            let rskey_led = self.render_rskey_led_card(cx, is_fido).into_any_element();
            let rskey_apps = self.render_rskey_apps_card(cx, is_fido).into_any_element();
            let rskey_usb_itf = self
                .render_rskey_usb_itf_card(cx, is_fido)
                .into_any_element();
            grid_children.push(rskey_led);
            grid_children.push(rskey_apps);
            grid_children.push(rskey_usb_itf);
        }

        let theme = cx.theme();

        PageView::build(
            "Configuration",
            "Customize device settings and behavior.",
            v_flex()
                .gap_6()
                .child(
                    div()
                        .grid()
                        .grid_cols(columns)
                        .gap_6()
                        .children(grid_children),
                )
                .child(
                    gpui_component::h_flex().justify_end().pt_4().child(
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
                ),
            theme,
        )
        .into_any_element()
    }
}
