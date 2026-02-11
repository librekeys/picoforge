use crate::device::io;
use crate::device::types::{AppConfigInput, FullDeviceStatus};
use crate::ui::components::{card::Card, page_view::PageView};
use crate::ui::ui_types::{LedDriverType, UsbIdentityPreset};
use gpui::*;
use gpui_component::button::{ButtonCustomVariant, ButtonVariants};
use gpui_component::{
    ActiveTheme, Disableable, Icon, Theme,
    button::Button,
    input::{Input, InputState},
    select::{Select, SelectItem, SelectState},
    slider::{Slider, SliderState},
    switch::Switch,
    v_flex,
};

#[derive(Clone, PartialEq)]
struct VendorSelectOption {
    preset: UsbIdentityPreset,
    label: SharedString,
}

impl SelectItem for VendorSelectOption {
    type Value = UsbIdentityPreset;

    fn title(&self) -> SharedString {
        self.label.clone()
    }

    fn value(&self) -> &Self::Value {
        &self.preset
    }
}

#[derive(Clone, PartialEq)]
struct DriverSelectOption {
    driver_type: LedDriverType,
    label: SharedString,
}

impl SelectItem for DriverSelectOption {
    type Value = LedDriverType;

    fn title(&self) -> SharedString {
        self.label.clone()
    }

    fn value(&self) -> &Self::Value {
        &self.driver_type
    }
}

pub struct ConfigView {
    vendor_select: Entity<SelectState<Vec<VendorSelectOption>>>,
    vid_input: Entity<InputState>,
    pid_input: Entity<InputState>,
    product_name_input: Entity<InputState>,
    led_gpio_input: Entity<InputState>,
    led_driver_select: Entity<SelectState<Vec<DriverSelectOption>>>,
    led_brightness_slider: Entity<SliderState>,
    led_dimmable: bool,
    led_steady: bool,
    touch_timeout_input: Entity<InputState>,
    power_cycle: bool,
    enable_secp256k1: bool,
    loading: bool,
    device_status: Option<FullDeviceStatus>,
    is_custom_vendor: bool,
    _task: Option<Task<()>>,
}

impl ConfigView {
    pub fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
        device_status: Option<FullDeviceStatus>,
    ) -> Self {
        let config = device_status.as_ref().map(|s| &s.config);

        let vendors: Vec<VendorSelectOption> = UsbIdentityPreset::all()
            .iter()
            .map(|preset| {
                let (label, _, _) = preset.details();
                VendorSelectOption {
                    preset: *preset,
                    label,
                }
            })
            .collect();

        let drivers: Vec<DriverSelectOption> = LedDriverType::all()
            .iter()
            .map(|driver| DriverSelectOption {
                driver_type: *driver,
                label: driver.label(),
            })
            .collect();

        let current_vid: SharedString = config
            .map(|c| c.vid.clone().into())
            .unwrap_or_else(|| "CAFE".into());
        let current_pid: SharedString = config
            .map(|c| c.pid.clone().into())
            .unwrap_or_else(|| "4242".into());
        let current_product_name: SharedString = config
            .map(|c| c.product_name.clone().into())
            .unwrap_or_else(|| "My Key".into());
        let current_led_gpio: SharedString = config
            .map(|c| c.led_gpio.to_string().into())
            .unwrap_or_else(|| "25".into());
        let current_touch_timeout: SharedString = config
            .map(|c| c.touch_timeout.to_string().into())
            .unwrap_or_else(|| "10".into());
        let current_brightness = config.map(|c| c.led_brightness as f32).unwrap_or(8.0);

        let initial_preset = UsbIdentityPreset::from_vid_pid(&current_vid, &current_pid);
        let is_custom_vendor = initial_preset == UsbIdentityPreset::Custom;

        let initial_vendor_idx = UsbIdentityPreset::all()
            .iter()
            .position(|p| *p == initial_preset)
            .unwrap_or(0);

        let vendor_select = cx.new(|cx| {
            SelectState::new(
                vendors,
                Some(gpui_component::IndexPath::default().row(initial_vendor_idx)),
                window,
                cx,
            )
        });

        let vid_input = cx.new(|cx| InputState::new(window, cx).default_value(current_vid.clone()));
        let pid_input = cx.new(|cx| InputState::new(window, cx).default_value(current_pid.clone()));
        let product_name_input =
            cx.new(|cx| InputState::new(window, cx).default_value(current_product_name.clone()));

        let led_gpio_input =
            cx.new(|cx| InputState::new(window, cx).default_value(current_led_gpio.clone()));

        let current_driver_val = config.and_then(|c| c.led_driver).unwrap_or(0);
        let initial_driver_idx = LedDriverType::all()
            .iter()
            .position(|d| d.value() == current_driver_val)
            .unwrap_or(0);

        let led_driver_select = cx.new(|cx| {
            SelectState::new(
                drivers,
                Some(gpui_component::IndexPath::default().row(initial_driver_idx)),
                window,
                cx,
            )
        });

        cx.subscribe_in(
            &vendor_select,
            window,
            |this: &mut Self, _, event, window, cx| {
                if let gpui_component::select::SelectEvent::Confirm(Some(preset)) = event {
                    let (_, vid_opt, pid_opt) = preset.details();

                    if let (Some(vid), Some(pid)) = (vid_opt, pid_opt) {
                        this.is_custom_vendor = false;
                        this.vid_input
                            .update(cx, |input, cx| input.set_value(vid, window, cx));
                        this.pid_input
                            .update(cx, |input, cx| input.set_value(pid, window, cx));
                    } else {
                        this.is_custom_vendor = true;
                    }
                    cx.notify();
                }
            },
        )
        .detach();

        let led_brightness_slider = cx.new(|_| {
            SliderState::new()
                .min(0.0)
                .max(15.0)
                .step(1.0)
                .default_value(current_brightness)
        });

        let touch_timeout_input =
            cx.new(|cx| InputState::new(window, cx).default_value(current_touch_timeout.clone()));

        Self {
            vendor_select,
            vid_input,
            pid_input,
            product_name_input,
            led_gpio_input,
            led_driver_select,
            led_brightness_slider,
            led_dimmable: config.map(|c| c.led_dimmable).unwrap_or(true),
            led_steady: config.map(|c| c.led_steady).unwrap_or(false),
            touch_timeout_input,
            power_cycle: config.map(|c| c.power_cycle_on_reset).unwrap_or(false),
            enable_secp256k1: config.map(|c| c.enable_secp256k1).unwrap_or(true),
            loading: false,
            device_status: device_status.clone(),
            is_custom_vendor,
            _task: None,
        }
    }

    fn apply_changes(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let status = if let Some(s) = &self.device_status {
            s
        } else {
            return;
        };

        let current_config = &status.config;
        let mut changes = AppConfigInput {
            vid: None,
            pid: None,
            product_name: None,
            led_gpio: None,
            led_brightness: None,
            touch_timeout: None,
            led_driver: None,
            led_dimmable: None,
            power_cycle_on_reset: None,
            led_steady: None,
            enable_secp256k1: None,
        };

        let vid = self.vid_input.read(cx).text().to_string();
        if vid != current_config.vid {
            changes.vid = Some(vid);
        }

        let pid = self.pid_input.read(cx).text().to_string();
        if pid != current_config.pid {
            changes.pid = Some(pid);
        }

        let product_name = self.product_name_input.read(cx).text().to_string();
        if product_name != current_config.product_name {
            changes.product_name = Some(product_name);
        }

        let led_gpio_str = self.led_gpio_input.read(cx).text().to_string();
        if let Ok(val) = led_gpio_str.parse::<u8>() {
            if val != current_config.led_gpio {
                changes.led_gpio = Some(val);
            }
        }

        let driver_idx = self.led_driver_select.read(cx).selected_index(cx);
        if let Some(idx) = driver_idx {
            if let Some(driver) = LedDriverType::all().get(idx.row) {
                let val = driver.value();
                if Some(val) != current_config.led_driver {
                    changes.led_driver = Some(val);
                }
            }
        }

        let brightness = self.led_brightness_slider.read(cx).value().start() as u8;
        if brightness != current_config.led_brightness {
            changes.led_brightness = Some(brightness);
        }

        let touch_timeout_str = self.touch_timeout_input.read(cx).text().to_string();
        if let Ok(val) = touch_timeout_str.parse::<u8>() {
            if val != current_config.touch_timeout {
                changes.touch_timeout = Some(val);
            }
        }

        if (self.led_dimmable != current_config.led_dimmable)
            || (self.led_steady != current_config.led_steady)
            || (self.power_cycle != current_config.power_cycle_on_reset)
        {
            changes.led_dimmable = Some(self.led_dimmable);
            changes.led_steady = Some(self.led_steady);
            changes.power_cycle_on_reset = Some(self.power_cycle);
        }

        if self.enable_secp256k1 != current_config.enable_secp256k1 {
            changes.enable_secp256k1 = Some(self.enable_secp256k1);
        }

        let has_changes = changes.vid.is_some()
            || changes.pid.is_some()
            || changes.product_name.is_some()
            || changes.led_gpio.is_some()
            || changes.led_brightness.is_some()
            || changes.touch_timeout.is_some()
            || changes.led_driver.is_some()
            || changes.led_dimmable.is_some()
            || changes.power_cycle_on_reset.is_some()
            || changes.led_steady.is_some()
            || changes.enable_secp256k1.is_some();

        if !has_changes {
            log::info!("No changes detected");
            return;
        }

        self.loading = true;
        cx.notify();

        let entity = cx.entity().downgrade();
        let method = status.method.clone();

        self._task = Some(cx.spawn(async move |_, cx| {
            let result = io::write_config(changes, method, None).await;

            let _ = entity.update(cx, |this, cx| {
                this.loading = false;

                match result {
                    Ok(msg) => {
                        log::info!("Success: {}", msg);

                        if let Ok(new_status) = io::read_device_details() {
                            log::info!(
                                "Refreshed device status. LED Steady: {}",
                                new_status.config.led_steady
                            );

                            let config = &new_status.config;

                            this.led_dimmable = config.led_dimmable;
                            this.led_steady = config.led_steady;
                            this.power_cycle = config.power_cycle_on_reset;
                            this.enable_secp256k1 = config.enable_secp256k1;

                            this.device_status = Some(new_status);
                            cx.notify();
                        }
                    }
                    Err(e) => {
                        log::error!("Error saving config: {}", e);
                    }
                }

                cx.notify();
            });
        }));
    }

    pub(crate) fn update_device_status(
        &mut self,
        status: Option<FullDeviceStatus>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.device_status == status {
            return;
        }
        self.device_status = status.clone();
        let config = status.as_ref().map(|s| &s.config);

        let vid = config
            .map(|c| c.vid.clone())
            .unwrap_or_else(|| "CAFE".into());
        self.vid_input
            .update(cx, |input, cx| input.set_value(vid, window, cx));

        let pid = config
            .map(|c| c.pid.clone())
            .unwrap_or_else(|| "4242".into());
        self.pid_input
            .update(cx, |input, cx| input.set_value(pid, window, cx));

        let product = config
            .map(|c| c.product_name.clone())
            .unwrap_or_else(|| "My Key".into());
        self.product_name_input
            .update(cx, |input, cx| input.set_value(product, window, cx));

        let gpio = config
            .map(|c| c.led_gpio.to_string())
            .unwrap_or_else(|| "25".into());
        self.led_gpio_input
            .update(cx, |input, cx| input.set_value(gpio, window, cx));

        let timeout = config
            .map(|c| c.touch_timeout.to_string())
            .unwrap_or_else(|| "10".into());
        self.touch_timeout_input
            .update(cx, |input, cx| input.set_value(timeout, window, cx));

        self.led_dimmable = config.map(|c| c.led_dimmable).unwrap_or(true);
        self.led_steady = config.map(|c| c.led_steady).unwrap_or(false);
        self.power_cycle = config.map(|c| c.power_cycle_on_reset).unwrap_or(false);
        self.enable_secp256k1 = config.map(|c| c.enable_secp256k1).unwrap_or(true);

        let brightness = config.map(|c| c.led_brightness as f32).unwrap_or(8.0);
        self.led_brightness_slider
            .update(cx, |slider, cx| slider.set_value(brightness, window, cx));

        cx.notify();
    }

    fn render_identity_card(&self, theme: &Theme) -> impl IntoElement {
        let content = v_flex()
            .gap_4()
            .child(
                v_flex()
                    .gap_2()
                    .child("Vendor Preset")
                    .child(Select::new(&self.vendor_select).bg(rgb(0x222225)).w_full()),
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
                                .disabled(!self.is_custom_vendor),
                        ),
                    )
                    .child(
                        v_flex().gap_2().child("Product ID (HEX)").child(
                            Input::new(&self.pid_input)
                                .font_family("Mono")
                                .disabled(!self.is_custom_vendor),
                        ),
                    ),
            )
            .child(div().h_px().bg(theme.border))
            .child(
                v_flex()
                    .gap_2()
                    .child("Product Name")
                    .child(Input::new(&self.product_name_input).bg(rgb(0x222225))),
            );

        Card::new()
            .title("Identity")
            .description("USB Identification settings")
            .icon(Icon::default().path("icons/tag.svg"))
            .child(content)
    }

    fn render_led_card(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
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
                v_flex()
                    .gap_2()
                    .child("LED GPIO Pin")
                    .child(Input::new(&self.led_gpio_input).bg(rgb(0x222225))),
            )
            .child(
                v_flex().gap_2().child("LED Driver").child(
                    Select::new(&self.led_driver_select)
                        .w_full()
                        .bg(rgb(0x222225)),
                ),
            )
            .child(div().h_px().bg(theme.border))
            .child(
                v_flex().gap_2().child("Brightness (0-15)").child(
                    gpui_component::h_flex()
                        .items_center()
                        .gap_4()
                        .child(Slider::new(&self.led_brightness_slider).flex_1())
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
                            .on_click(steady_listener),
                    ),
            );

        Card::new()
            .title("LED Settings")
            .description("Adjust visual feedback behavior")
            .icon(Icon::default().path("icons/microchip.svg"))
            .child(content)
    }

    fn render_touch_card(&self, _theme: &Theme) -> impl IntoElement {
        let content = v_flex().gap_4().child(
            v_flex()
                .gap_2()
                .child("Touch Timeout (seconds)")
                .child(Input::new(&self.touch_timeout_input).bg(rgb(0x222225))),
        );

        Card::new()
            .title("Touch & Timing")
            .description("Configure interaction timeouts")
            .icon(Icon::default().path("icons/settings.svg"))
            .child(content)
    }

    fn render_options_card(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
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
                            .on_click(secp_listener),
                    ),
            );

        Card::new()
            .title("Device Options")
            .description("Toggle advanced features")
            .icon(Icon::default().path("icons/settings.svg"))
            .child(content)
    }
}

impl Render for ConfigView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        if self.device_status.is_none() {
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
                    .child(div().text_color(theme.muted_foreground).child("No Content")),
                theme,
            )
            .into_any_element();
        }

        let led_card = self.render_led_card(cx).into_any_element();
        let options_card = self.render_options_card(cx).into_any_element();

        let theme = cx.theme();

        let identity_card = self.render_identity_card(theme).into_any_element();
        let touch_card = self.render_touch_card(theme).into_any_element();

        let is_wide = window.bounds().size.width > px(1100.0);
        let columns = if is_wide { 2 } else { 1 };

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
                        .child(identity_card)
                        .child(led_card)
                        .child(touch_card)
                        .child(options_card),
                )
                .child(
                    gpui_component::h_flex().justify_end().child(
                        Button::new("apply-changes")
                            .icon(Icon::default().path("icons/save.svg"))
                            .child("Apply Changes")
                            .disabled(self.loading)
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
            &theme,
        )
        .into_any_element()
    }
}
