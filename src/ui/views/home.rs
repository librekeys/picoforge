use crate::device::types::DeviceMethod;
use crate::ui::components::{card::Card, page_view::PageView};
use crate::ui::ui_types::GlobalDeviceState;
use gpui::*;
use gpui_component::StyledExt;
use gpui_component::{Icon, IconName, Theme, badge::Badge, h_flex, progress::Progress, v_flex};

pub struct HomeView;

impl HomeView {
    pub fn build(
        state: &GlobalDeviceState,
        theme: &Theme,
        window_width: Pixels,
    ) -> impl IntoElement {
        let connected = state.device_status.is_some();
        let is_wide = window_width > px(1100.0);
        let columns = if is_wide { 2 } else { 1 };

        PageView::build(
            "Device Overview",
            "Quick view of your device status and specifications.",
            if !connected {
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
                // Card Grid
                div()
                    .grid()
                    .grid_cols(columns)
                    .gap_6()
                    .child(Self::render_device_info(state, theme))
                    .child(Self::render_fido_info(state, theme))
                    .child(Self::render_led_config(state, theme))
                    .child(Self::render_security_status(state, theme))
                    .into_any_element()
            },
            theme,
        )
    }

    // Helper for Key-Value pairs
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

    fn render_device_info(state: &GlobalDeviceState, theme: &Theme) -> impl IntoElement {
        let status = state.device_status.as_ref().unwrap();
        let info = &status.info;
        let config = &status.config;

        let flash_percent = (info.flash_used as f32 / info.flash_total as f32) * 100.0;

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
                                    .child(div().text_color(theme.foreground).child(format!(
                                        "{:.0} / {:.0} KB",
                                        info.flash_used, info.flash_total
                                    ))),
                            )
                            .child(Progress::new().value(flash_percent)),
                    ),
            )
    }

    fn render_fido_info(state: &GlobalDeviceState, theme: &Theme) -> impl IntoElement {
        Card::new()
            .title("FIDO2 Information")
            .icon(Icon::default().path("icons/shield.svg"))
            .child(if let Some(fido) = &state.fido_info {
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
                                if fido.options.get("clientPin").copied().unwrap_or(false) {
                                    "Yes"
                                } else {
                                    "No"
                                },
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
                                if fido.options.get("rk").copied().unwrap_or(false) {
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
            })
    }

    fn render_led_config(state: &GlobalDeviceState, theme: &Theme) -> impl IntoElement {
        let status = state.device_status.as_ref().unwrap();
        let config = &status.config;
        Card::new()
            .title("LED Configuration")
            .icon(Icon::default().path("icons/microchip.svg"))
            .child(if status.method == DeviceMethod::Fido {
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
                                Badge::new()
                                    .child(if config.led_dimmable { "Yes" } else { "No" })
                                    .color(if config.led_dimmable {
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
                                    .child(if config.led_steady { "On" } else { "Off" })
                                    .color(if config.led_steady {
                                        theme.primary
                                    } else {
                                        theme.secondary
                                    }),
                            ),
                    )
                    .into_any_element()
            })
    }

    fn render_security_status(state: &GlobalDeviceState, theme: &Theme) -> impl IntoElement {
        let status = state.device_status.as_ref().unwrap();
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
                                        Badge::new()
                                            .child(if status.secure_boot {
                                                "Secure Boot"
                                            } else {
                                                "Development"
                                            })
                                            .color(if status.secure_boot {
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
                                Badge::new()
                                    .child(if status.secure_lock {
                                        "Acknowledged"
                                    } else {
                                        "Pending"
                                    })
                                    .color(if status.secure_lock {
                                        gpui::red()
                                    } else {
                                        theme.secondary
                                    }),
                            ),
                    ),
            )
    }
}
