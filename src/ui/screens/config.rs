use crate::hal::rescue::constants::{
    LedColor, LedStatus, USB_CAP_FIDO2, USB_CAP_OATH, USB_CAP_OPENPGP, USB_CAP_OTP, USB_CAP_PIV,
    USB_CAP_U2F,
};
use crate::hal::types::{AppConfigInput, DeviceMethod};
use crate::hal::{fido, io};
use crate::ui::components::dialog::PinPromptContent;
use crate::ui::components::{card::Card, dialog, dialog::StatusContent, page_view::PageView};
use crate::ui::rootview::ApplicationRoot;
use crate::ui::types::{DeviceConnectionState, LedDriverType, UsbIdentityPreset};
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

enum StatusDialogHandle {
    Pin(WeakEntity<PinPromptContent>),
    Status(WeakEntity<StatusContent>),
}

pub struct ConfigView {
    root: WeakEntity<ApplicationRoot>,
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
    is_custom_vendor: bool,

    // RS-Key specific state
    led_status_steady: bool,
    led_status_colors: [u8; 4],
    led_status_brightness: [u8; 4],
    usb_apps_supported: u16,
    usb_apps_enabled: u16,
    enabled_usb_itf: Option<u8>,

    _task: Option<Task<()>>,
}

impl ConfigView {
    pub fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
        root: WeakEntity<ApplicationRoot>,
        device: DeviceConnectionState,
    ) -> Self {
        let config = device.status.as_ref().map(|s| &s.config);

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

        let mut led_status_steady = false;
        let mut led_status_colors = [0; 4];
        let mut led_status_brightness = [0; 4];
        if let Some(led) = &device.led_status {
            led_status_steady = led.steady;
            for i in 0..4 {
                led_status_colors[i] = led.statuses[i].0;
                led_status_brightness[i] = led.statuses[i].1;
            }
        }

        let mut usb_apps_supported = 0;
        let mut usb_apps_enabled = 0;
        if let Some(apps) = &device.management_apps {
            usb_apps_supported = apps.usb_supported;
            usb_apps_enabled = apps.usb_enabled;
        }

        Self {
            root,
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
            is_custom_vendor,
            led_status_steady,
            led_status_colors,
            led_status_brightness,
            usb_apps_supported,
            usb_apps_enabled,
            enabled_usb_itf: config.and_then(|c| c.enabled_usb_itf),
            _task: None,
        }
    }

    fn write_config_to_device(
        &mut self,
        changes: AppConfigInput,
        method: crate::hal::types::DeviceMethod,
        pin: Option<String>,
        dialog_handle: StatusDialogHandle,
        cx: &mut Context<Self>,
    ) {
        let expected_serial = self.root.upgrade().and_then(|r| {
            r.read(cx)
                .device
                .status
                .as_ref()
                .map(|s| s.info.serial.clone())
        });

        self.loading = true;
        cx.notify();

        let entity = cx.entity().downgrade();
        let method_clone = method.clone();

        self._task = Some(cx.spawn(async move |_, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { io::write_config(changes, method_clone, pin) })
                .await;

            let new_status_result = if result.is_ok() {
                Some(
                    cx.background_executor()
                        .spawn(async move { io::read_device_details() })
                        .await,
                )
            } else {
                None
            };

            let _ = entity.update(cx, |this, cx| {
                this.loading = false;

                match result {
                    Ok(msg) => {
                        log::info!("Success: {}", msg);

                        if let Some(Ok(new_status)) = new_status_result {
                            let serial_matches = expected_serial.as_deref()
                                                        == Some(new_status.info.serial.as_str());

                            if serial_matches {
                                log::info!(
                                    "Refreshed device status. LED Steady: {}",
                                    new_status.config.led_steady
                                );

                                let config = &new_status.config;
                                this.led_dimmable = config.led_dimmable;
                                this.led_steady = config.led_steady;
                                this.power_cycle = config.power_cycle_on_reset;
                                this.enable_secp256k1 = config.enable_secp256k1;

                                let _ = this.root.update(cx, |root, cx| {
                                    root.device.status = Some(new_status);
                                    cx.notify();
                                });
                            } else {
                                log::warn!("Device changed during config write, discarding stale status");
                            }
                        }

                        match &dialog_handle {
                            StatusDialogHandle::Pin(dh) => {
                                let _ = dh.update(cx, |d, cx| {
                                    d.set_success(
                                        "Configuration applied successfully.".to_string(),
                                        cx,
                                    );
                                });
                            }
                            StatusDialogHandle::Status(dh) => {
                                let _ = dh.update(cx, |d, cx| {
                                    d.set_success(
                                        "Configuration applied successfully.".to_string(),
                                        cx,
                                    );
                                });
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("Error saving config: {}", e);

                        let mut err_msg = format!("Failed to apply configuration: {}", e);

                        // Special case for FIDO 0x3E error (Invalid Subcommand)
                        // This happens when the firmware is too old to support config over FIDO
                        if method == DeviceMethod::Fido && err_msg.contains("0x3E")
                        {
                            err_msg = "The device firmware does not support being configured in fido only communication mode. \nHave a look at the troubleshooting guide to fix this".to_string();
                        }

                        match &dialog_handle {
                            StatusDialogHandle::Pin(dh) => {
                                let _ = dh.update(cx, |d, cx| {
                                    d.set_error(err_msg, cx);
                                });
                            }
                            StatusDialogHandle::Status(dh) => {
                                let _ = dh.update(cx, |d, cx| {
                                    d.set_error(err_msg, cx);
                                });
                            }
                        }
                    }
                }

                cx.notify();
            });
        }));
    }

    fn open_pin_dialog(
        &mut self,
        changes: AppConfigInput,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let view_handle = cx.entity().downgrade();

        dialog::open_pin_prompt(
            "Authentication Required",
            "Enter your device PIN to apply changes.",
            None,
            "Confirm",
            window,
            cx,
            move |pin, dialog_handle, cx| {
                let _ = view_handle.update(cx, |this, cx| {
                    this.write_config_to_device(
                        changes.clone(),
                        DeviceMethod::Fido,
                        Some(pin),
                        StatusDialogHandle::Pin(dialog_handle),
                        cx,
                    );
                });
            },
        );
    }

    fn apply_changes(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(root) = self.root.upgrade() else {
            return;
        };
        let device = root.read(cx).device.clone();
        let Some(status) = &device.status else { return };

        let current_config = &status.config;
        let mut has_changes = false;

        let vid = self.vid_input.read(cx).text().to_string();
        if vid != current_config.vid {
            has_changes = true;
        }

        let pid = self.pid_input.read(cx).text().to_string();
        if pid != current_config.pid {
            has_changes = true;
        }

        let product_name = self.product_name_input.read(cx).text().to_string();
        if product_name != current_config.product_name {
            has_changes = true;
        }

        let mut final_led_gpio = current_config.led_gpio;
        let led_gpio_str = self.led_gpio_input.read(cx).text().to_string();
        if let Ok(val) = led_gpio_str.parse::<u8>() {
            if val != current_config.led_gpio {
                has_changes = true;
            }
            final_led_gpio = val;
        }

        let mut final_led_driver = current_config.led_driver;
        let driver_idx = self.led_driver_select.read(cx).selected_index(cx);
        if let Some(idx) = driver_idx
            && let Some(driver) = LedDriverType::all().get(idx.row)
        {
            let val = driver.value();
            let current_val = current_config.led_driver.unwrap_or(1);
            if val != current_val {
                has_changes = true;
            }
            final_led_driver = Some(val);
        }

        let brightness = self.led_brightness_slider.read(cx).value().start() as u8;
        if brightness != current_config.led_brightness {
            has_changes = true;
        }

        let mut final_touch_timeout = current_config.touch_timeout;
        let touch_timeout_str = self.touch_timeout_input.read(cx).text().to_string();
        if let Ok(val) = touch_timeout_str.parse::<u8>() {
            if val != current_config.touch_timeout {
                has_changes = true;
            }
            final_touch_timeout = val;
        }

        if (self.led_dimmable != current_config.led_dimmable)
            || (self.led_steady != current_config.led_steady)
            || (self.power_cycle != current_config.power_cycle_on_reset)
        {
            has_changes = true;
        }

        if self.enable_secp256k1 != current_config.enable_secp256k1 {
            has_changes = true;
        }

        let mut final_enabled_usb_itf = current_config.enabled_usb_itf;
        if self.enabled_usb_itf != current_config.enabled_usb_itf {
            has_changes = true;
            final_enabled_usb_itf = self.enabled_usb_itf;
        }

        if !has_changes {
            log::info!("No changes detected");
            return;
        }

        let changes = AppConfigInput {
            vid: Some(vid),
            pid: Some(pid),
            product_name: Some(product_name),
            led_gpio: Some(final_led_gpio),
            led_brightness: Some(brightness),
            touch_timeout: Some(final_touch_timeout),
            led_driver: final_led_driver,
            led_dimmable: Some(self.led_dimmable),
            power_cycle_on_reset: Some(self.power_cycle),
            led_steady: Some(self.led_steady),
            enable_secp256k1: Some(self.enable_secp256k1),
            raw_curves_mask: current_config.raw_curves_mask,
            led_order: current_config.led_order,
            enabled_usb_itf: final_enabled_usb_itf,
        };

        let method = status.method.clone();

        if method == DeviceMethod::Fido {
            if Self::status_supports_legacy_fido_config(status) {
                self.open_pin_dialog(changes, window, cx);
            } else {
                let handle =
                    dialog::open_status_dialog("Configuration Requires Rescue Mode", window, cx);
                self.write_config_to_device(
                    changes,
                    method,
                    None,
                    StatusDialogHandle::Status(handle),
                    cx,
                );
            }
        } else {
            let handle = dialog::open_status_dialog("Applying Configuration", window, cx);
            self.write_config_to_device(
                changes,
                method,
                None,
                StatusDialogHandle::Status(handle),
                cx,
            );
        }
    }

    fn status_supports_legacy_fido_config(status: &crate::hal::types::FullDeviceStatus) -> bool {
        status.method == DeviceMethod::Fido
            && fido::firmware_supports_legacy_fido_hardware_config(&status.info.firmware_version)
    }

    pub fn sync_from_device(
        &mut self,
        device: &DeviceConnectionState,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let config = device.status.as_ref().map(|s| &s.config);

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

        let new_driver_val = config.and_then(|c| c.led_driver).unwrap_or(1);
        let new_driver_idx = LedDriverType::all()
            .iter()
            .position(|d| d.value() == new_driver_val)
            .unwrap_or(0);
        self.led_driver_select.update(cx, |select, cx| {
            select.set_selected_index(
                Some(gpui_component::IndexPath::default().row(new_driver_idx)),
                window,
                cx,
            );
        });

        if let Some(led) = &device.led_status {
            self.led_status_steady = led.steady;
            for i in 0..4 {
                self.led_status_colors[i] = led.statuses[i].0;
                self.led_status_brightness[i] = led.statuses[i].1;
            }
        }

        if let Some(apps) = &device.management_apps {
            self.usb_apps_supported = apps.usb_supported;
            self.usb_apps_enabled = apps.usb_enabled;
        }

        self.enabled_usb_itf = config.and_then(|c| c.enabled_usb_itf);

        cx.notify();
    }

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

    /// Renders the RS-Key-specific LED configuration card.
    ///
    /// This dynamic panel iterates through the device's LED operating statuses (Idle, Processing,
    /// Touch, Boot) and provides interactive widgets to customize the active color and brightness
    /// level for each. Only displayed when an RS-Key firmware is detected.
    fn render_rskey_led_card(&mut self, cx: &mut Context<Self>, is_fido: bool) -> impl IntoElement {
        let theme = cx.theme();
        let mut rows = v_flex().gap_4();

        // Steady switch
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

        // Create rows for each status
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

        // Add a save button for LED status
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

    fn apply_rskey_led_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let steady = self.led_status_steady;
        let colors = self.led_status_colors;
        let brightnesses = self.led_status_brightness;

        self.loading = true;
        let handle = dialog::open_status_dialog("Applying LED Configuration...", window, cx);
        let entity = cx.entity().downgrade();

        self._task = Some(cx.spawn(async move |_, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    for i in 0..4 {
                        io::write_led_status(i as u8, colors[i], brightnesses[i], steady)?;
                    }
                    Ok::<_, crate::error::PFError>(())
                })
                .await;

            let _ = entity.update(cx, |this, cx| {
                this.loading = false;
                match result {
                    Ok(_) => {
                        let _ = handle.update(cx, |d, cx| {
                            d.set_success(
                                "LED configuration applied successfully.".to_string(),
                                cx,
                            );
                        });
                    }
                    Err(e) => {
                        let _ = handle.update(cx, |d, cx| {
                            d.set_error(format!("Failed to apply LED config: {}", e), cx);
                        });
                    }
                }
                cx.notify();
            });
        }));
    }

    /// Renders the RS-Key-specific USB Applications management card.
    ///
    /// Provides toggles to enable or disable USB endpoints such as U2F, OATH, PIV, and OpenPGP.
    /// Safely computes the bitmasks and writes to the Management applet. Gated by hardware support.
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

    fn apply_rskey_apps_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let mask = self.usb_apps_enabled;

        self.loading = true;
        let handle = dialog::open_status_dialog("Applying USB Applications...", window, cx);
        let entity = cx.entity().downgrade();

        self._task = Some(cx.spawn(async move |_, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { io::write_management_config(mask) })
                .await;

            let _ = entity.update(cx, |this, cx| {
                this.loading = false;
                match result {
                    Ok(_) => {
                        let _ = handle.update(cx, |d, cx| {
                            d.set_success(
                                "USB applications updated successfully. Please re-plug the device."
                                    .to_string(),
                                cx,
                            );
                        });
                    }
                    Err(e) => {
                        let _ = handle.update(cx, |d, cx| {
                            d.set_error(format!("Failed to apply USB applications: {}", e), cx);
                        });
                    }
                }
                cx.notify();
            });
        }));
    }

    fn render_rskey_usb_itf_card(
        &mut self,
        cx: &mut Context<Self>,
        is_fido: bool,
    ) -> impl IntoElement {
        let theme = cx.theme();
        let mut rows = v_flex().gap_4();

        // 0x01: CCID, 0x02: WCID, 0x04: HID, 0x08: KB, 0x10: LWIP
        let interfaces = [
            ("CCID (Smart Card)", 0x01u8),
            ("WCID (WebUSB)", 0x02u8),
            ("HID (FIDO)", 0x04u8),
            ("KB (Keyboard)", 0x08u8),
            ("LWIP", 0x10u8),
        ];

        let current_mask = self.enabled_usb_itf.unwrap_or(0x1F); // Default to all on if missing

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
                    // Force CCID on to prevent bricking
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
                            .checked(is_enabled || is_ccid) // CCID always looks checked
                            .disabled(is_fido || is_ccid) // Disable toggling CCID entirely!
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
        let has_device = self
            .root
            .upgrade()
            .map(|r| r.read(cx).device.status.is_some())
            .unwrap_or(false);

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

        let status = self
            .root
            .upgrade()
            .and_then(|r| r.read(cx).device.status.clone());
        let is_fido = status.as_ref().map(|s| s.method.clone()) == Some(DeviceMethod::Fido);
        let supports_legacy_fido_config = status
            .as_ref()
            .map(Self::status_supports_legacy_fido_config)
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
