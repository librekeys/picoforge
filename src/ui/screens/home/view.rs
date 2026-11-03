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

    /// RS-Key impersonates a YubiKey's CTAP `firmwareVersion` (e.g. 5.7.4), so
    /// its real build id is the USB bcdDevice; prefer that when we have it.
    fn firmware_version_label(status: &FullDeviceStatus) -> String {
        if status.firmware_type == FirmwareType::RSKey
            && let Some(bcd) = status.info.bcd_device
        {
            if let Some(ver) = Self::rs_key_version_from_bcd(bcd) {
                crate::tr!("RS-Key {} (build 0x{:04X})", ver, bcd)
            } else {
                crate::tr!("RS-Key build 0x{:04X}", bcd)
            }
        } else {
            crate::tr!("v{}", status.info.firmware_version)
        }
    }

    /// Human-readable flash chip size. RP2350 boards are whole-MB (2/4/16 MB).
    fn format_flash_size(bytes: u32) -> String {
        const MB: u32 = 1024 * 1024;
        if bytes >= MB && bytes.is_multiple_of(MB) {
            crate::tr!("{} MB", bytes / MB)
        } else if bytes >= 1024 && bytes.is_multiple_of(1024) {
            crate::tr!("{} KB", bytes / 1024)
        } else {
            crate::tr!("{} B", bytes)
        }
    }

    fn render_device_info(status: &FullDeviceStatus, theme: &Theme) -> impl IntoElement {
        let info = &status.info;
        let config = &status.config;
        // RS-Key's rescue FlashInfo is the KV filesystem (credentials & config),
        // not the whole chip — label it honestly and surface objects + chip size.
        let is_rskey = status.firmware_type == FirmwareType::RSKey;
        let flash_label = if is_rskey {
            crate::tr!("Storage (credentials & config)")
        } else {
            crate::tr!("Flash Memory")
        };

        Card::new()
            .title(crate::tr!("Device Information"))
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
                                crate::tr!("Serial Number"),
                                info.serial.clone(),
                                theme,
                                true,
                            ))
                            .child(Self::render_kv(
                                crate::tr!("Firmware Version"),
                                Self::firmware_version_label(status),
                                theme,
                                true,
                            ))
                            .child(Self::render_kv(
                                crate::tr!("Firmware Type"),
                                status.firmware_type.to_string(),
                                theme,
                                false,
                            ))
                            .child(Self::render_kv(
                                crate::tr!("VID:PID"),
                                crate::tr!("{}:{}", config.vid, config.pid),
                                theme,
                                true,
                            ))
                            .child(Self::render_kv(
                                crate::tr!("Manufacturer"),
                                info.manufacturer
                                    .clone()
                                    .unwrap_or_else(|| crate::tr!("Unknown").to_string()),
                                theme,
                                false,
                            ))
                            .child(Self::render_kv(
                                crate::tr!("Product Name"),
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
                                        div().text_color(theme.muted_foreground).child(flash_label),
                                    )
                                    .child(div().text_color(theme.foreground).child(
                                        if let (Some(used), Some(total)) =
                                            (info.flash_used, info.flash_total)
                                        {
                                            crate::tr!("{:.0} / {:.0} KB", used, total)
                                        } else {
                                            crate::tr!("Not Available").to_string()
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
                            )
                            .when_some(info.flash_files.filter(|_| is_rskey), |this, nfiles| {
                                this.child(
                                    h_flex()
                                        .justify_between()
                                        .text_sm()
                                        .child(
                                            div()
                                                .text_color(theme.muted_foreground)
                                                .child(crate::tr!("Stored objects")),
                                        )
                                        .child(
                                            div()
                                                .text_color(theme.foreground)
                                                .child(nfiles.to_string()),
                                        ),
                                )
                            })
                            .when_some(info.flash_chip_size.filter(|_| is_rskey), |this, chip| {
                                this.child(
                                    h_flex()
                                        .justify_between()
                                        .text_sm()
                                        .child(
                                            div()
                                                .text_color(theme.muted_foreground)
                                                .child(crate::tr!("Flash chip")),
                                        )
                                        .child(
                                            div()
                                                .text_color(theme.foreground)
                                                .child(Self::format_flash_size(chip)),
                                        ),
                                )
                            }),
                    ),
            )
    }

    fn render_fido_info(fido: Option<&FidoDeviceInfo>, theme: &Theme) -> impl IntoElement {
        Card::new()
            .title(crate::tr!("FIDO2 Information"))
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
                                    .child(crate::tr!("FIDO Versions")),
                            )
                            .child(div().text_color(theme.foreground).child(
                                if fido.versions.is_empty() {
                                    crate::tr!("N/A").to_string()
                                } else {
                                    fido.versions.join(crate::tr!(" · "))
                                },
                            )),
                    )
                    .child(div().h_px().bg(theme.border))
                    .child(
                        h_flex()
                            .justify_between()
                            .items_center()
                            .child(
                                div()
                                    .text_color(theme.muted_foreground)
                                    .child(crate::tr!("PIN Set")),
                            )
                            .child({
                                let pin_set =
                                    fido.options.get("clientPin").copied().unwrap_or(false);
                                Tag::new(if pin_set {
                                    crate::tr!("Set")
                                } else {
                                    crate::tr!("Not Set")
                                })
                                .active(pin_set)
                            }),
                    )
                    .child(
                        h_flex()
                            .justify_between()
                            .items_center()
                            .child(
                                div()
                                    .text_color(theme.muted_foreground)
                                    .child(crate::tr!("Resident Keys")),
                            )
                            .child({
                                let resident_keys_supported =
                                    fido.options.get(crate::tr!("rk")).copied().unwrap_or(false);
                                Tag::new(if resident_keys_supported {
                                    crate::tr!("Supported")
                                } else {
                                    crate::tr!("Not Supported")
                                })
                                .active(resident_keys_supported)
                            }),
                    )
                    .child(
                        h_flex()
                            .justify_between()
                            .items_center()
                            .child(
                                div()
                                    .text_color(theme.muted_foreground)
                                    .child(crate::tr!("Min PIN Length")),
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
                                    .child(crate::tr!("Enterprise attestation")),
                            )
                            .child(div().font_medium().text_color(theme.foreground).child({
                                let enterprise_attestation_set =
                                    fido.options.get(crate::tr!("ep")).copied().unwrap_or(false);
                                Tag::new(if enterprise_attestation_set {
                                    crate::tr!("Set")
                                } else {
                                    crate::tr!("Not Set")
                                })
                                .active(enterprise_attestation_set)
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
                                        .child(crate::tr!("Remaining Credentials")),
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
                    .child(crate::tr!("FIDO information not available"))
                    .into_any_element()
            })
    }

    fn render_led_config(status: &FullDeviceStatus, theme: &Theme) -> impl IntoElement {
        let config = &status.config;
        let has_fido_config =
            status.firmware_type == FirmwareType::RSKey || status.method != DeviceMethod::Fido;
        Card::new()
            .title(crate::tr!("LED Configuration"))
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
                            .child(crate::tr!(
                                "Information is not available in Fido only communication mode."
                            )),
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
                                    .child(crate::tr!("LED GPIO Pin")),
                            )
                            .child(
                                config
                                    .led_gpio
                                    .map(|g| crate::tr!("GPIO {}", g))
                                    .or_else(|| {
                                        config
                                            .effective_led_gpio
                                            .map(|g| crate::tr!("GPIO {g} (default)", g = g))
                                    })
                                    .unwrap_or_else(|| crate::tr!("Firmware default").into()),
                            ),
                    )
                    .child(
                        h_flex()
                            .justify_between()
                            .child(
                                div()
                                    .text_color(theme.muted_foreground)
                                    .child(crate::tr!("LED Brightness")),
                            )
                            .child(
                                config
                                    .led_brightness
                                    .map(|b| b.to_string())
                                    .unwrap_or_else(|| crate::tr!("Firmware default").into()),
                            ),
                    )
                    .child(
                        h_flex()
                            .justify_between()
                            .child(
                                div()
                                    .text_color(theme.muted_foreground)
                                    .child(crate::tr!("Presence Touch Timeout")),
                            )
                            .child(
                                config
                                    .touch_timeout
                                    .map(|t| crate::tr!("{}s", t))
                                    .or_else(|| {
                                        config
                                            .effective_touch_timeout
                                            .map(|t| crate::tr!("{t}s (default)", t = t))
                                    })
                                    .unwrap_or_else(|| crate::tr!("Firmware default").into()),
                            ),
                    )
                    .child(
                        h_flex()
                            .justify_between()
                            .child(
                                div()
                                    .text_color(theme.muted_foreground)
                                    .child(crate::tr!("LED Dimmable")),
                            )
                            .child(
                                Tag::new(if config.led_dimmable {
                                    crate::tr!("Yes")
                                } else {
                                    crate::tr!("No")
                                })
                                .active(config.led_dimmable),
                            ),
                    )
                    .child(
                        h_flex()
                            .justify_between()
                            .child(
                                div()
                                    .text_color(theme.muted_foreground)
                                    .child(crate::tr!("LED Steady Mode")),
                            )
                            .child(
                                Tag::new(if config.led_steady {
                                    crate::tr!("On")
                                } else {
                                    crate::tr!("Off")
                                })
                                .active(config.led_steady),
                            ),
                    )
                    .into_any_element()
            })
    }

    fn render_security_status(status: &FullDeviceStatus, theme: &Theme) -> impl IntoElement {
        Card::new()
            .title(crate::tr!("Security Status"))
            .icon(Icon::default().path("icons/shield-check.svg"))
            .child(
                v_flex()
                    .gap_3()
                    .text_sm()
                    .child(
                        h_flex()
                            .justify_between()
                            .items_center()
                            .child(
                                div()
                                    .text_color(theme.muted_foreground)
                                    .child(crate::tr!("Boot Mode")),
                            )
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
                                            crate::tr!("Secure Boot")
                                        } else {
                                            crate::tr!("Development")
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
                                    .child(crate::tr!("Debug Interface")),
                            )
                            .child(div().font_medium().text_color(theme.foreground).child(
                                if status.secure_lock {
                                    crate::tr!("Read-out Locked")
                                } else {
                                    crate::tr!("Debug Enabled")
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
                                    .child(crate::tr!("Secure Lock")),
                            )
                            .child(
                                Tag::new(if status.secure_lock {
                                    crate::tr!("Acknowledged")
                                } else {
                                    crate::tr!("Pending")
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
            crate::tr!("Device Overview"),
            crate::tr!("Quick view of your device status and specifications."),
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
                            .child(crate::tr!("No Device Connected")),
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
