// use crate::device::types::*;
use gpui::*;
use gpui_component::StyledExt;
use gpui_component::{Icon, IconName, Theme, badge::Badge, h_flex, progress::Progress, v_flex};

// These will be replaced/added in types.rs
struct DeviceInfo {
    serial: String,
    firmware_version: String,
    flash_used: f32,
    flash_total: f32,
}

struct DeviceConfig {
    vid: u16,
    pid: u16,
    product_name: String,
    led_gpio: u8,
    led_brightness: u8,
    touch_timeout: u8,
    led_dimmable: bool,
    led_steady: bool,
}

struct FidoInfo {
    versions: Vec<String>,
    client_pin: bool,
    min_pin_length: u8,
    resident_keys: bool,
    aaguid: String,
}

struct DeviceSecurity {
    secure_boot: bool,
    secure_lock: bool,
    confirmed: bool,
}

struct DeviceState {
    connected: bool,
    method: String,
    info: DeviceInfo,
    config: DeviceConfig,
    fido_info: Option<FidoInfo>,
    security: DeviceSecurity,
}

pub struct HomeView;

impl HomeView {
    pub fn build(theme: &Theme) -> impl IntoElement {
        // Mock Data, I will replace this with fetching of actual data later, kinda bored rn.
        let device = DeviceState {
            connected: true,
            method: "HID".to_string(),
            info: DeviceInfo {
                serial: "A1B2C3D4E5".to_string(),
                firmware_version: "1.2.0".to_string(),
                flash_used: 128.0,
                flash_total: 2048.0,
            },
            config: DeviceConfig {
                vid: 0x0000,
                pid: 0x0000,
                product_name: "Pico FIDO Key".to_string(),
                led_gpio: 25,
                led_brightness: 128,
                touch_timeout: 10,
                led_dimmable: true,
                led_steady: false,
            },
            fido_info: Some(FidoInfo {
                versions: vec!["FIDO2_1".to_string(), "U2F_V2".to_string()],
                client_pin: true,
                min_pin_length: 4,
                resident_keys: true,
                aaguid: "00000000-0000-0000-0000-000000000000".to_string(),
            }),
            security: DeviceSecurity {
                secure_boot: true,
                secure_lock: false,
                confirmed: true,
            },
        };

        div()
            .size_full()
            .bg(theme.background)
            .flex()
            .flex_col()
            .items_center()
            .child(
                div().w_full().max_w(px(1200.0)).px_10().py_5().child(
                    v_flex()
                        .gap_8()
                        .child(
                            v_flex()
                                .child(
                                    div()
                                        .text_3xl()
                                        .font_extrabold()
                                        .text_color(theme.foreground)
                                        .child("Device Overview"),
                                )
                                .child(
                                    div().text_sm().text_color(theme.muted_foreground).child(
                                        "Quick view of your device status and specifications.",
                                    ),
                                ),
                        )
                        // Content Section
                        .child(if !device.connected {
                            // No Device Status Placeholder
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
                                )
                                .into_any_element()
                        } else {
                            // 4 Card Grid
                            div()
                                .grid()
                                .grid_cols(2)
                                .gap_6()
                                .child(Self::render_device_info(&device, theme))
                                .child(Self::render_fido_info(&device, theme))
                                .child(Self::render_led_config(&device, theme))
                                .child(Self::render_security_status(&device, theme))
                                .into_any_element()
                        }),
                ),
            )
    }

    fn home_card(
        title: &str,
        icon: Icon,
        content: impl IntoElement,
        theme: &Theme,
    ) -> impl IntoElement {
        div()
            .w_full()
            // TODO: REPLACE with a constant or modify default theme
            .bg(rgb(0x18181b))
            .border_1()
            .border_color(theme.border)
            .rounded_xl()
            .p_6()
            .child(
                v_flex()
                    .gap_6()
                    .child(
                        h_flex()
                            .items_center()
                            .gap_2()
                            .child(Icon::new(icon).size_5().text_color(theme.foreground))
                            .child(
                                div()
                                    .font_bold()
                                    .text_color(theme.foreground)
                                    .child(title.to_string()),
                            ),
                    )
                    .child(content),
            )
    }

    // --- Helper for Key-Value pairs ---
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

    fn render_device_info(device: &DeviceState, theme: &Theme) -> impl IntoElement {
        let flash_percent = (device.info.flash_used / device.info.flash_total) * 100.0;

        Self::home_card(
            "Device Information",
            Icon::default().path("icons/cpu.svg"),
            v_flex()
                .gap_6()
                .child(
                    div()
                        .grid()
                        .grid_cols(2)
                        .gap_4()
                        .child(Self::render_kv(
                            "Serial Number",
                            device.info.serial.clone(),
                            theme,
                            true,
                        ))
                        .child(Self::render_kv(
                            "Firmware Version",
                            format!("v{}", device.info.firmware_version),
                            theme,
                            true,
                        ))
                        .child(Self::render_kv(
                            "VID:PID",
                            format!("{:04x}:{:04x}", device.config.vid, device.config.pid),
                            theme,
                            true,
                        ))
                        .child(Self::render_kv(
                            "Product Name",
                            device.config.product_name.clone(),
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
                                .child(div().text_color(theme.foreground).child(format!(
                                    "{:.0} / {:.0} KB",
                                    device.info.flash_used, device.info.flash_total
                                ))),
                        )
                        .child(Progress::new().value(flash_percent)),
                ),
            theme,
        )
    }

    fn render_fido_info(device: &DeviceState, theme: &Theme) -> impl IntoElement {
        Self::home_card(
            "FIDO2 Information",
            Icon::default().path("icons/shield.svg"),
            if let Some(fido) = &device.fido_info {
                v_flex()
                    .gap_6()
                    .child(
                        div()
                            .grid()
                            .grid_cols(2)
                            .gap_4()
                            .child(Self::render_kv(
                                "FIDO Version",
                                fido.versions.first().cloned().unwrap_or("N/A".into()),
                                theme,
                                false,
                            ))
                            .child(Self::render_kv(
                                "PIN Set",
                                if fido.client_pin { "Yes" } else { "No" },
                                theme,
                                false,
                            ))
                            .child(Self::render_kv(
                                "Min PIN Length",
                                fido.min_pin_length.to_string(),
                                theme,
                                false,
                            ))
                            .child(Self::render_kv(
                                "Resident Keys",
                                if fido.resident_keys {
                                    "Supported"
                                } else {
                                    "Not Supported"
                                },
                                theme,
                                false,
                            )),
                    )
                    .child(div().h_px().bg(theme.border))
                    .child(Self::render_kv("AAGUID", fido.aaguid.clone(), theme, true))
                    .into_any_element()
            } else {
                div()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child("FIDO information not available")
                    .into_any_element()
            },
            theme,
        )
    }

    fn render_led_config(device: &DeviceState, theme: &Theme) -> impl IntoElement {
        Self::home_card(
            "LED Configuration",
            Icon::default().path("icons/microchip.svg"),
            if device.method == "FIDO" {
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
                            .child(format!("GPIO {}", device.config.led_gpio)),
                    )
                    .child(
                        h_flex()
                            .justify_between()
                            .child(
                                div()
                                    .text_color(theme.muted_foreground)
                                    .child("LED Brightness"),
                            )
                            .child(device.config.led_brightness.to_string()),
                    )
                    .child(
                        h_flex()
                            .justify_between()
                            .child(
                                div()
                                    .text_color(theme.muted_foreground)
                                    .child("Presence Touch Timeout"),
                            )
                            .child(format!("{}s", device.config.touch_timeout)),
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
                                Badge::new()
                                    .child(if device.config.led_dimmable {
                                        "Yes"
                                    } else {
                                        "No"
                                    })
                                    .color(if device.config.led_dimmable {
                                        theme.primary
                                    } else {
                                        theme.secondary
                                    }),
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
                                Badge::new()
                                    .child(if device.config.led_steady {
                                        "On"
                                    } else {
                                        "Off"
                                    })
                                    .color(if device.config.led_steady {
                                        theme.primary
                                    } else {
                                        theme.secondary
                                    }),
                            ),
                    )
                    .into_any_element()
            },
            theme,
        )
    }

    fn render_security_status(device: &DeviceState, theme: &Theme) -> impl IntoElement {
        Self::home_card(
            "Security Status",
            Icon::default().path("icons/shield-check.svg"),
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
                                .child(if device.security.secure_boot {
                                    Icon::default()
                                        .path("icons/lock.svg")
                                        .size_3p5()
                                        .text_color(gpui::green())
                                } else {
                                    Icon::default()
                                        .path("icons/lock-open.svg")
                                        .size_3p5()
                                        .text_color(gpui::yellow())
                                })
                                .child(
                                    Badge::new()
                                        .child(if device.security.secure_boot {
                                            "Secure Boot"
                                        } else {
                                            "Development"
                                        })
                                        .color(if device.security.secure_boot {
                                            theme.primary
                                        } else {
                                            theme.secondary
                                        }),
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
                            if device.security.secure_lock {
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
                            Badge::new()
                                .child(if device.security.confirmed {
                                    "Acknowledged"
                                } else {
                                    "Pending"
                                })
                                .color(if device.security.confirmed {
                                    gpui::red()
                                } else {
                                    theme.secondary
                                }),
                        ),
                ),
            theme,
        )
    }
}
