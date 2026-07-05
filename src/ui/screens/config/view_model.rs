use crate::hal::types::{AppConfigInput, DeviceMethod};
use crate::hal::{fido, io};
use crate::ui::app::AppModels;
use crate::ui::components::dialog::PinPromptContent;
use crate::ui::components::{dialog, dialog::StatusContent};
use crate::ui::models::device::{DeviceEvent, DeviceRepo};
use gpui::*;
use gpui_component::input::InputState;
use gpui_component::select::{SelectItem, SelectState};
use gpui_component::slider::SliderState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsbIdentityPreset {
    Custom,
    Generic,
    LibreKeys,
    PicoHsm,
    PicoFido,
    PicoOpenPgp,
    Pico,
    SoloKeys,
    NitroHsm,
    NitroFido2,
    NitroStart,
    NitroPro,
    NitroKey3,
    YubiKey5,
    YubiKeyNeo,
    YubiHsm2,
    Gnuk,
    GnuPg,
}

impl UsbIdentityPreset {
    pub fn details(&self) -> (SharedString, Option<&'static str>, Option<&'static str>) {
        match self {
            Self::Custom => ("Custom (Manual Entry)".into(), None, None),
            Self::Generic => ("Generic (FEFF:FCFD)".into(), Some("FEFF"), Some("FCFD")),
            Self::LibreKeys => (
                "LibreKeys One (1D50:619B)".into(),
                Some("1D50"),
                Some("619B"),
            ),
            Self::PicoHsm => (
                "Pico Keys HSM (2E8A:10FD)".into(),
                Some("2E8A"),
                Some("10FD"),
            ),
            Self::PicoFido => (
                "Pico Keys Fido (2E8A:10FE)".into(),
                Some("2E8A"),
                Some("10FE"),
            ),
            Self::PicoOpenPgp => (
                "Pico Keys OpenPGP (2E8A:10FF)".into(),
                Some("2E8A"),
                Some("10FF"),
            ),
            Self::Pico => ("Pico (2E8A:0003)".into(), Some("2E8A"), Some("0003")),
            Self::SoloKeys => ("SoloKeys (0483:A2CA)".into(), Some("0483"), Some("A2CA")),
            Self::NitroHsm => ("NitroHSM (20A0:4230)".into(), Some("20A0"), Some("4230")),
            Self::NitroFido2 => ("NitroFIDO2 (20A0:42D4)".into(), Some("20A0"), Some("42D4")),
            Self::NitroStart => ("NitroStart (20A0:4211)".into(), Some("20A0"), Some("4211")),
            Self::NitroPro => ("NitroPro (20A0:4108)".into(), Some("20A0"), Some("4108")),
            Self::NitroKey3 => ("Nitrokey 3 (20A0:42B2)".into(), Some("20A0"), Some("42B2")),
            Self::YubiKey5 => ("YubiKey 5 (1050:0407)".into(), Some("1050"), Some("0407")),
            Self::YubiKeyNeo => ("YubiKey Neo (1050:0116)".into(), Some("1050"), Some("0116")),
            Self::YubiHsm2 => ("YubiHSM 2 (1050:0030)".into(), Some("1050"), Some("0030")),
            Self::Gnuk => ("Gnuk Token (234B:0000)".into(), Some("234B"), Some("0000")),
            Self::GnuPg => ("GnuPG (234B:0000)".into(), Some("234B"), Some("0000")),
        }
    }

    pub fn from_vid_pid(vid: &str, pid: &str) -> Self {
        let vid = vid.to_uppercase();
        let pid = pid.to_uppercase();

        match (vid.as_str(), pid.as_str()) {
            ("FEFF", "FCFD") => Self::Generic,
            ("1D50", "619B") => Self::LibreKeys,
            ("2E8A", "10FD") => Self::PicoHsm,
            ("2E8A", "10FE") => Self::PicoFido,
            ("2E8A", "10FF") => Self::PicoOpenPgp,
            ("2E8A", "0003") => Self::Pico,
            ("0483", "A2CA") => Self::SoloKeys,
            ("20A0", "4230") => Self::NitroHsm,
            ("20A0", "42D4") => Self::NitroFido2,
            ("20A0", "4211") => Self::NitroStart,
            ("20A0", "4108") => Self::NitroPro,
            ("20A0", "42B2") => Self::NitroKey3,
            ("1050", "0407") => Self::YubiKey5,
            ("1050", "0116") => Self::YubiKeyNeo,
            ("1050", "0030") => Self::YubiHsm2,
            ("234B", "0000") => Self::Gnuk,
            _ => Self::Custom,
        }
    }

    pub fn all() -> &'static [Self] {
        &[
            Self::Custom,
            Self::Generic,
            Self::LibreKeys,
            Self::PicoHsm,
            Self::PicoFido,
            Self::PicoOpenPgp,
            Self::Pico,
            Self::SoloKeys,
            Self::NitroHsm,
            Self::NitroFido2,
            Self::NitroStart,
            Self::NitroPro,
            Self::NitroKey3,
            Self::YubiKey5,
            Self::YubiKeyNeo,
            Self::YubiHsm2,
            Self::Gnuk,
            Self::GnuPg,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedDriverType {
    PicoGpio = 1,
    PimoroniRgb = 2,
    Ws2812Neopixel = 3,
    Esp32Neopixel = 5,
}

impl LedDriverType {
    pub fn label(&self) -> SharedString {
        match self {
            Self::PicoGpio => "Pico (Standard GPIO)".into(),
            Self::PimoroniRgb => "Pimoroni (RGB)".into(),
            Self::Ws2812Neopixel => "WS2812 (Neopixel)".into(),
            Self::Esp32Neopixel => "ESP32 Neopixel".into(),
        }
    }

    pub fn value(&self) -> u8 {
        *self as u8
    }

    pub fn all() -> &'static [Self] {
        &[
            Self::PicoGpio,
            Self::PimoroniRgb,
            Self::Ws2812Neopixel,
            Self::Esp32Neopixel,
        ]
    }
}

#[derive(Clone, PartialEq)]
pub(super) struct VendorSelectOption {
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
pub(super) struct DriverSelectOption {
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

pub(super) enum StatusDialogHandle {
    Pin(WeakEntity<PinPromptContent>),
    Status(WeakEntity<StatusContent>),
}

pub struct ConfigView {
    pub(super) device: Entity<DeviceRepo>,
    pub(super) vendor_select: Entity<SelectState<Vec<VendorSelectOption>>>,
    pub(super) vid_input: Entity<InputState>,
    pub(super) pid_input: Entity<InputState>,
    pub(super) product_name_input: Entity<InputState>,
    pub(super) led_gpio_input: Entity<InputState>,
    pub(super) led_driver_select: Entity<SelectState<Vec<DriverSelectOption>>>,
    pub(super) led_brightness_slider: Entity<SliderState>,
    pub(super) led_dimmable: bool,
    pub(super) led_steady: bool,
    pub(super) touch_timeout_input: Entity<InputState>,
    pub(super) power_cycle: bool,
    pub(super) enable_secp256k1: bool,
    pub(super) loading: bool,
    pub(super) is_custom_vendor: bool,

    // RS-Key specific state
    pub(super) led_status_steady: bool,
    pub(super) led_status_colors: [u8; 4],
    pub(super) led_status_brightness: [u8; 4],
    pub(super) usb_apps_supported: u16,
    pub(super) usb_apps_enabled: u16,
    pub(super) enabled_usb_itf: Option<u8>,

    pub(super) _task: Option<Task<()>>,
}

impl ConfigView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>, models: &AppModels) -> Self {
        let device = models.device.clone();
        cx.subscribe(&device, |_, _, _: &DeviceEvent, cx| cx.notify())
            .detach();

        let device_read = device.read(cx);
        let config = device_read.status.as_ref().map(|s| &s.config);

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

        let led_dimmable = config.map(|c| c.led_dimmable).unwrap_or(true);
        let led_steady = config.map(|c| c.led_steady).unwrap_or(false);
        let power_cycle = config.map(|c| c.power_cycle_on_reset).unwrap_or(false);
        let enable_secp256k1 = config.map(|c| c.enable_secp256k1).unwrap_or(true);
        let enabled_usb_itf = config.and_then(|c| c.enabled_usb_itf);
        let current_driver_val = config.and_then(|c| c.led_driver).unwrap_or(0);

        let mut led_status_steady = false;
        let mut led_status_colors = [0; 4];
        let mut led_status_brightness = [0; 4];
        if let Some(led) = &device_read.led_status {
            led_status_steady = led.steady;
            for i in 0..4 {
                led_status_colors[i] = led.statuses[i].0;
                led_status_brightness[i] = led.statuses[i].1;
            }
        }

        let mut usb_apps_supported = 0;
        let mut usb_apps_enabled = 0;
        if let Some(apps) = &device_read.management_apps {
            usb_apps_supported = apps.usb_supported;
            usb_apps_enabled = apps.usb_enabled;
        }

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
            device,
            vendor_select,
            vid_input,
            pid_input,
            product_name_input,
            led_gpio_input,
            led_driver_select,
            led_brightness_slider,
            led_dimmable,
            led_steady,
            touch_timeout_input,
            power_cycle,
            enable_secp256k1,
            loading: false,
            is_custom_vendor,
            led_status_steady,
            led_status_colors,
            led_status_brightness,
            usb_apps_supported,
            usb_apps_enabled,
            enabled_usb_itf,
            _task: None,
        }
    }

    pub(super) fn write_config_to_device(
        &mut self,
        changes: AppConfigInput,
        method: crate::hal::types::DeviceMethod,
        pin: Option<String>,
        dialog_handle: StatusDialogHandle,
        cx: &mut Context<Self>,
    ) {
        let expected_serial = self
            .device
            .read(cx)
            .status
            .as_ref()
            .map(|s| s.info.serial.clone());

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

                                this.device.update(cx, |repo, _| {
                                    repo.status = Some(new_status);
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

                        if method == DeviceMethod::Fido && err_msg.contains("0x3E") {
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

    pub(super) fn apply_changes(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let device = self.device.read(cx);
        let Some(status) = &device.status else { return };

        let current_vid = status.config.vid.clone();
        let current_pid = status.config.pid.clone();
        let current_product_name = status.config.product_name.clone();
        let current_led_gpio = status.config.led_gpio;
        let current_led_driver = status.config.led_driver;
        let current_led_brightness = status.config.led_brightness;
        let current_touch_timeout = status.config.touch_timeout;
        let current_led_dimmable = status.config.led_dimmable;
        let current_led_steady = status.config.led_steady;
        let current_power_cycle = status.config.power_cycle_on_reset;
        let current_enable_secp256k1 = status.config.enable_secp256k1;
        let current_enabled_usb_itf = status.config.enabled_usb_itf;
        let raw_curves_mask = status.config.raw_curves_mask;
        let led_order = status.config.led_order;
        let method = status.method.clone();

        let mut has_changes = false;

        let vid = self.vid_input.read(cx).text().to_string();
        if vid != current_vid {
            has_changes = true;
        }

        let pid = self.pid_input.read(cx).text().to_string();
        if pid != current_pid {
            has_changes = true;
        }

        let product_name = self.product_name_input.read(cx).text().to_string();
        if product_name != current_product_name {
            has_changes = true;
        }

        let mut final_led_gpio = current_led_gpio;
        let led_gpio_str = self.led_gpio_input.read(cx).text().to_string();
        if let Ok(val) = led_gpio_str.parse::<u8>() {
            if val != current_led_gpio {
                has_changes = true;
            }
            final_led_gpio = val;
        }

        let mut final_led_driver = current_led_driver;
        let driver_idx = self.led_driver_select.read(cx).selected_index(cx);
        if let Some(idx) = driver_idx
            && let Some(driver) = LedDriverType::all().get(idx.row)
        {
            let val = driver.value();
            let current_val = current_led_driver.unwrap_or(1);
            if val != current_val {
                has_changes = true;
            }
            final_led_driver = Some(val);
        }

        let brightness = self.led_brightness_slider.read(cx).value().start() as u8;
        if brightness != current_led_brightness {
            has_changes = true;
        }

        let mut final_touch_timeout = current_touch_timeout;
        let touch_timeout_str = self.touch_timeout_input.read(cx).text().to_string();
        if let Ok(val) = touch_timeout_str.parse::<u8>() {
            if val != current_touch_timeout {
                has_changes = true;
            }
            final_touch_timeout = val;
        }

        if (self.led_dimmable != current_led_dimmable)
            || (self.led_steady != current_led_steady)
            || (self.power_cycle != current_power_cycle)
        {
            has_changes = true;
        }

        if self.enable_secp256k1 != current_enable_secp256k1 {
            has_changes = true;
        }

        let mut final_enabled_usb_itf = current_enabled_usb_itf;
        if self.enabled_usb_itf != current_enabled_usb_itf {
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
            raw_curves_mask,
            led_order,
            enabled_usb_itf: final_enabled_usb_itf,
        };

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

    pub(super) fn status_supports_legacy_fido_config(
        status: &crate::hal::types::FullDeviceStatus,
    ) -> bool {
        status.method == DeviceMethod::Fido
            && fido::firmware_supports_legacy_fido_hardware_config(&status.info.firmware_version)
    }

    pub fn sync_from_device(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let device = self.device.read(cx);
        let config = device.status.as_ref().map(|s| &s.config);

        let vid = config
            .map(|c| c.vid.clone())
            .unwrap_or_else(|| "CAFE".into());
        let pid = config
            .map(|c| c.pid.clone())
            .unwrap_or_else(|| "4242".into());
        let product = config
            .map(|c| c.product_name.clone())
            .unwrap_or_else(|| "My Key".into());
        let gpio = config
            .map(|c| c.led_gpio.to_string())
            .unwrap_or_else(|| "25".into());
        let timeout = config
            .map(|c| c.touch_timeout.to_string())
            .unwrap_or_else(|| "10".into());

        self.led_dimmable = config.map(|c| c.led_dimmable).unwrap_or(true);
        self.led_steady = config.map(|c| c.led_steady).unwrap_or(false);
        self.power_cycle = config.map(|c| c.power_cycle_on_reset).unwrap_or(false);
        self.enable_secp256k1 = config.map(|c| c.enable_secp256k1).unwrap_or(true);

        let brightness = config.map(|c| c.led_brightness as f32).unwrap_or(8.0);

        let new_driver_val = config.and_then(|c| c.led_driver).unwrap_or(1);

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

        let preset = UsbIdentityPreset::from_vid_pid(&vid, &pid);
        self.is_custom_vendor = preset == UsbIdentityPreset::Custom;
        let preset_idx = UsbIdentityPreset::all()
            .iter()
            .position(|p| *p == preset)
            .unwrap_or(0);
        self.vendor_select.update(cx, |select, cx| {
            select.set_selected_index(
                Some(gpui_component::IndexPath::default().row(preset_idx)),
                window,
                cx,
            );
        });

        self.vid_input
            .update(cx, |input, cx| input.set_value(vid, window, cx));
        self.pid_input
            .update(cx, |input, cx| input.set_value(pid, window, cx));
        self.product_name_input
            .update(cx, |input, cx| input.set_value(product, window, cx));
        self.led_gpio_input
            .update(cx, |input, cx| input.set_value(gpio, window, cx));
        self.touch_timeout_input
            .update(cx, |input, cx| input.set_value(timeout, window, cx));
        self.led_brightness_slider
            .update(cx, |slider, cx| slider.set_value(brightness, window, cx));

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

        cx.notify();
    }

    pub(super) fn apply_rskey_led_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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

    pub(super) fn apply_rskey_apps_settings(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
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
}
