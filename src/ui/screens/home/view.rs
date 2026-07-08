use crate::ui::components::{card::Card, page_view::PageView, tag::Tag};
use crate::ui::models::device::{DeviceMethod, FidoDeviceInfo, FirmwareType, FullDeviceStatus};
use crate::ui::screens::home::view_model::HomeViewModel;
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_component::{ActiveTheme, StyledExt};
use gpui_component::{Icon, IconName, Theme, h_flex, progress::Progress, v_flex};

impl HomeViewModel {
    fn render_kv(
        label: &str,
        value: impl IntoElement,
        theme: &Theme,
        font_mono: bool,
    ) -> impl IntoElement {
        v_flex()
            .gap_1()
            .child(
                div()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child(label.to_string()),
            )
            .child(
                div()
                    .text_sm()
                    .font_weight(if font_mono {
                        FontWeight::NORMAL
                    } else {
                        FontWeight::MEDIUM
                    })
                    .font_family(if font_mono { "Mono" } else { "Sans" })
                    .text_color(theme.foreground)
                    .child(value),
            )
    }

    fn render_device_info(status: &FullDeviceStatus, theme: &Theme) -> impl IntoElement {
        let info = &status.info;
        let config = &status.config;

        Card::new()
            .title("Device Information")
            .icon(Icon::default().path("icons/cpu.svg"))
            .child(
                v_flex()
                    .gap_6()
                    .child(
                        div()
                            .grid()
                            .grid_cols(2)
                            .gap_4()
                            .child(Self::render_kv(
                                "Serial Number",
                                info.serial.clone(),
                                theme,
                                true,
                            ))
                            .child(Self::render_kv(
                                "Firmware Version",
                                format!("v{}", info.firmware_version),
                                theme,
                                true,
                            ))
                            .child(Self::render_kv(
                                "Firmware Type",
                                status.firmware_type.to_string(),
                                theme,
                                false,
                            ))
                            .child(Self::render_kv(
                                "VID:PID",
                                format!("{}:{}", config.vid, config.pid),
                                theme,
                                true,
                            ))
                            .child(Self::render_kv(
                                "Product Name",
                                config.product_name.clone(),
                                theme,
                                false,
                            )),
                    )
                    .child(div().h_px().bg(theme.border))
                    .child(
                        v_flex()
                            .gap_2()
                            .child(
                                h_flex()
                                    .justify_between()
                                    .text_sm()
                                    .child(
                                        div()
                                            .text_color(theme.muted_foreground)
                                            .child("Flash Memory"),
                                    )
                                    .child(div().text_color(theme.foreground).child(
                                        if let (Some(used), Some(total)) =
                                            (info.flash_used, info.flash_total)
                                        {
                                            format!("{:.0} / {:.0} KB", used, total)
                                        } else {
                                            "Not Available".to_string()
                                        },
                                    )),
                            )
                            .when(
                                info.flash_used.is_some() && info.flash_total.is_some(),
                                |this| {
                                    let used = info.flash_used.unwrap();
                                    let total = info.flash_total.unwrap();
                                    let flash_percent = (used as f32 / total as f32) * 100.0;
                                    this.child(Progress::new().value(flash_percent))
                                },
                            ),
                    ),
            )
    }

    fn render_fido_info(fido: Option<&FidoDeviceInfo>, theme: &Theme) -> impl IntoElement {
        Card::new()
            .title("FIDO2 Information")
            .icon(Icon::default().path("icons/shield.svg"))
            .child(if let Some(fido) = fido {
                v_flex()
                    .gap_3()
                    .text_sm()
                    .child(
                        h_flex()
                            .justify_between()
                            .items_center()
                            .flex_wrap()
                            .gap_1()
                            .child(div().text_color(theme.muted_foreground).child("AAGUID"))
                            .child(
                                div()
                                    .font_family("Mono")
                                    .text_color(theme.foreground)
                                    .child(fido.aaguid.clone()),
                            ),
                    )
                    .child(
                        h_flex()
                            .justify_between()
                            .items_center()
                            .flex_wrap()
                            .gap_1()
                            .child(
                                div()
                                    .text_color(theme.muted_foreground)
                                    .child("FIDO Versions"),
                            )
                            .child(div().text_color(theme.foreground).child(
                                if fido.versions.is_empty() {
                                    "N/A".to_string()
                                } else {
                                    fido.versions.join(" · ")
                                },
                            )),
                    )
                    .child(div().h_px().bg(theme.border))
                    .child(
                        h_flex()
                            .justify_between()
                            .items_center()
                            .child(div().text_color(theme.muted_foreground).child("PIN Set"))
                            .child({
                                let pin_set =
                                    fido.options.get("clientPin").copied().unwrap_or(false);
                                Tag::new(if pin_set { "Set" } else { "Not Set" }).active(pin_set)
                            }),
                    )
                    .child(
                        h_flex()
                            .justify_between()
                            .items_center()
                            .child(
                                div()
                                    .text_color(theme.muted_foreground)
                                    .child("Resident Keys"),
                            )
                            .child({
                                let rk = fido.options.get("rk").copied().unwrap_or(false);
                                Tag::new(if rk { "Supported" } else { "Not Supported" }).active(rk)
                            }),
                    )
                    .child(
                        h_flex()
                            .justify_between()
                            .items_center()
                            .child(
                                div()
                                    .text_color(theme.muted_foreground)
                                    .child("Min PIN Length"),
                            )
                            .child(
                                div()
                                    .font_medium()
                                    .text_color(theme.foreground)
                                    .child(fido.min_pin_length.to_string()),
                            ),
                    )
                    .child(
                        h_flex()
                            .justify_between()
                            .items_center()
                            .child(
                                div()
                                    .text_color(theme.muted_foreground)
                                    .child("Enterprise attestation"),
                            )
                            .child(div().font_medium().text_color(theme.foreground).child({
                                let ep_set = fido.options.get("ep").copied().unwrap_or(false);
                                Tag::new(if ep_set { "Set" } else { "Not Set" }).active(ep_set)
                            })),
                    )
                    .when(fido.remaining_discoverable_credentials.is_some(), |this| {
                        this.child(
                            h_flex()
                                .justify_between()
                                .items_center()
                                .child(
                                    div()
                                        .text_color(theme.muted_foreground)
                                        .child("Remaining Credentials"),
                                )
                                .child(
                                    div().font_medium().text_color(theme.foreground).child(
                                        fido.remaining_discoverable_credentials
                                            .unwrap_or(0)
                                            .to_string(),
                                    ),
                                ),
                        )
                    })
                    .into_any_element()
            } else {
                div()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child("FIDO information not available")
                    .into_any_element()
            })
    }

    fn render_led_config(status: &FullDeviceStatus, theme: &Theme) -> impl IntoElement {
        let config = &status.config;
        let has_fido_config =
            status.firmware_type == FirmwareType::RSKey || status.method != DeviceMethod::Fido;
        Card::new()
            .title("LED Configuration")
            .icon(Icon::default().path("icons/microchip.svg"))
            .child(if !has_fido_config {
                v_flex()
                    .items_center()
                    .justify_center()
                    .py_4()
                    .gap_2()
                    .child(
                        Icon::new(IconName::TriangleAlert)
                            .size_8()
                            .text_color(gpui::yellow()),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child("Information is not available in Fido only communication mode."),
                    )
                    .into_any_element()
            } else {
                v_flex()
                    .gap_3()
                    .text_sm()
                    .child(
                        h_flex()
                            .justify_between()
                            .child(
                                div()
                                    .text_color(theme.muted_foreground)
                                    .child("LED GPIO Pin"),
                            )
                            .child(format!("GPIO {}", config.led_gpio)),
                    )
                    .child(
                        h_flex()
                            .justify_between()
                            .child(
                                div()
                                    .text_color(theme.muted_foreground)
                                    .child("LED Brightness"),
                            )
                            .child(config.led_brightness.to_string()),
                    )
                    .child(
                        h_flex()
                            .justify_between()
                            .child(
                                div()
                                    .text_color(theme.muted_foreground)
                                    .child("Presence Touch Timeout"),
                            )
                            .child(format!("{}s", config.touch_timeout)),
                    )
                    .child(
                        h_flex()
                            .justify_between()
                            .child(
                                div()
                                    .text_color(theme.muted_foreground)
                                    .child("LED Dimmable"),
                            )
                            .child(
                                Tag::new(if config.led_dimmable { "Yes" } else { "No" })
                                    .active(config.led_dimmable),
                            ),
                    )
                    .child(
                        h_flex()
                            .justify_between()
                            .child(
                                div()
                                    .text_color(theme.muted_foreground)
                                    .child("LED Steady Mode"),
                            )
                            .child(
                                Tag::new(if config.led_steady { "On" } else { "Off" })
                                    .active(config.led_steady),
                            ),
                    )
                    .into_any_element()
            })
    }

    fn render_security_status(status: &FullDeviceStatus, theme: &Theme) -> impl IntoElement {
        Card::new()
            .title("Security Status")
            .icon(Icon::default().path("icons/shield-check.svg"))
            .child(
                v_flex()
                    .gap_3()
                    .text_sm()
                    .child(
                        h_flex()
                            .justify_between()
                            .items_center()
                            .child(div().text_color(theme.muted_foreground).child("Boot Mode"))
                            .child(
                                h_flex()
                                    .gap_2()
                                    .items_center()
                                    .child(if status.secure_boot {
                                        Icon::default()
                                            .path("icons/lock.svg")
                                            .size_3p5()
                                            .text_color(gpui::green())
                                    } else {
                                        Icon::default()
                                            .path("icons/lock-open.svg")
                                            .size_3p5()
                                            .text_color(rgb(0xfe9a00))
                                    })
                                    .child(
                                        Tag::new(if status.secure_boot {
                                            "Secure Boot"
                                        } else {
                                            "Development"
                                        })
                                        .active(status.secure_boot),
                                    ),
                            ),
                    )
                    .child(
                        h_flex()
                            .justify_between()
                            .items_center()
                            .child(
                                div()
                                    .text_color(theme.muted_foreground)
                                    .child("Debug Interface"),
                            )
                            .child(div().font_medium().text_color(theme.foreground).child(
                                if status.secure_lock {
                                    "Read-out Locked"
                                } else {
                                    "Debug Enabled"
                                },
                            )),
                    )
                    .child(
                        h_flex()
                            .justify_between()
                            .items_center()
                            .child(
                                div()
                                    .text_color(theme.muted_foreground)
                                    .child("Secure Lock"),
                            )
                            .child(
                                Tag::new(if status.secure_lock {
                                    "Acknowledged"
                                } else {
                                    "Pending"
                                })
                                .active(status.secure_lock),
                            ),
                    ),
            )
    }
}

impl Render for HomeViewModel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let device = self.device.read(cx);
        let connected = device.status.is_some();
        let is_wide = window.bounds().size.width > px(1100.0);
        let columns = if is_wide { 2 } else { 1 };

        PageView::build(
            "Device Overview",
            "Quick view of your device status and specifications.",
            if !connected {
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .h_64()
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded_xl()
                    .child(
                        div()
                            .text_color(cx.theme().muted_foreground)
                            .child("No Device Connected"),
                    )
                    .into_any_element()
            } else {
                let status = device.status.as_ref().unwrap();
                div()
                    .grid()
                    .grid_cols(columns)
                    .gap_6()
                    .child(Self::render_device_info(status, cx.theme()))
                    .child(Self::render_fido_info(
                        device.fido_info.as_ref(),
                        cx.theme(),
                    ))
                    .child(Self::render_led_config(status, cx.theme()))
                    .child(Self::render_security_status(status, cx.theme()))
                    .into_any_element()
            },
            cx.theme(),
        )
    }
}
