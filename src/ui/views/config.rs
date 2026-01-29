use crate::ui::components::page_view::PageView;
use gpui::*;
use gpui_component::{
    ActiveTheme, Disableable, Icon, StyledExt, Theme,
    button::Button,
    input::{Input, InputState},
    select::{Select, SelectItem, SelectState},
    slider::{Slider, SliderState},
    switch::Switch,
    v_flex,
};

#[derive(Clone, PartialEq)]
struct VendorItem {
    value: SharedString,
    label: SharedString,
}

impl SelectItem for VendorItem {
    type Value = SharedString;

    fn title(&self) -> SharedString {
        self.label.clone()
    }

    fn value(&self) -> &Self::Value {
        &self.value
    }
}

#[derive(Clone, PartialEq)]
struct DriverItem {
    value: u8,
    label: SharedString,
}

impl SelectItem for DriverItem {
    type Value = u8;

    fn title(&self) -> SharedString {
        self.label.clone()
    }

    fn value(&self) -> &Self::Value {
        &self.value
    }
}

pub struct ConfigView {
    vendor_select: Entity<SelectState<Vec<VendorItem>>>,
    vid_input: Entity<InputState>,
    pid_input: Entity<InputState>,
    product_name_input: Entity<InputState>,
    led_gpio_input: Entity<InputState>,
    led_driver_select: Entity<SelectState<Vec<DriverItem>>>,
    led_brightness_slider: Entity<SliderState>,
    led_dimmable: bool,
    led_steady: bool,
    touch_timeout_input: Entity<InputState>,
    power_cycle: bool,
    enable_secp256k1: bool,
    loading: bool,
}

impl ConfigView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let vendors = vec![
            VendorItem {
                value: "custom".into(),
                label: "Custom".into(),
            },
            VendorItem {
                value: "solokeys".into(),
                label: "SoloKeys".into(),
            },
            VendorItem {
                value: "google".into(),
                label: "Google".into(),
            },
            VendorItem {
                value: "yubico".into(),
                label: "Yubico".into(),
            },
        ];

        let drivers = vec![
            DriverItem {
                value: 0,
                label: "WS2812".into(),
            },
            DriverItem {
                value: 1,
                label: "SK6812".into(),
            },
            DriverItem {
                value: 2,
                label: "APA102".into(),
            },
        ];

        let vendor_select = cx.new(|cx| {
            SelectState::new(
                vendors,
                Some(gpui_component::IndexPath::default()),
                window,
                cx,
            )
        });

        let vid_input = cx.new(|cx| InputState::new(window, cx).default_value("CAFE"));
        let pid_input = cx.new(|cx| InputState::new(window, cx).default_value("4242"));
        let product_name_input = cx.new(|cx| InputState::new(window, cx).default_value("My Key"));

        let led_gpio_input = cx.new(|cx| InputState::new(window, cx).default_value("25"));
        let led_driver_select = cx.new(|cx| {
            SelectState::new(
                drivers,
                Some(gpui_component::IndexPath::default()),
                window,
                cx,
            )
        });

        let led_brightness_slider = cx.new(|_| {
            SliderState::new()
                .min(0.0)
                .max(15.0)
                .step(1.0)
                .default_value(8.0)
        });

        let touch_timeout_input = cx.new(|cx| InputState::new(window, cx).default_value("10"));

        Self {
            vendor_select,
            vid_input,
            pid_input,
            product_name_input,
            led_gpio_input,
            led_driver_select,
            led_brightness_slider,
            led_dimmable: true,
            led_steady: false,
            touch_timeout_input,
            power_cycle: false,
            enable_secp256k1: true,
            loading: false,
        }
    }

    fn render_identity_card(&self, theme: &Theme) -> impl IntoElement {
        let content = v_flex()
            .gap_4()
            .child(
                v_flex()
                    .gap_2()
                    .child("Vendor Preset")
                    .child(Select::new(&self.vendor_select).w_full()),
            )
            .child(
                div()
                    .grid()
                    .grid_cols(2)
                    .gap_4()
                    .child(
                        v_flex()
                            .gap_2()
                            .child("Vendor ID (HEX)")
                            .child(Input::new(&self.vid_input).font_family("Mono")),
                    )
                    .child(
                        v_flex()
                            .gap_2()
                            .child("Product ID (HEX)")
                            .child(Input::new(&self.pid_input).font_family("Mono")),
                    ),
            )
            .child(div().h_px().bg(theme.border))
            .child(
                v_flex()
                    .gap_2()
                    .child("Product Name")
                    .child(Input::new(&self.product_name_input)),
            );

        Self::config_card(
            "Identity",
            "USB Identification settings",
            Icon::default().path("icons/tag.svg"),
            content,
            theme,
        )
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

        // Access theme after creating listeners (which requires mutable borrow of cx)
        let theme = cx.theme();

        // Read slider value (requires immutable borrow of cx)
        let brightness = self.led_brightness_slider.read(cx).value().start() as i32;

        let content = v_flex()
            .gap_4()
            .child(
                v_flex()
                    .gap_2()
                    .child("LED GPIO Pin")
                    .child(Input::new(&self.led_gpio_input)),
            )
            .child(
                v_flex()
                    .gap_2()
                    .child("LED Driver")
                    .child(Select::new(&self.led_driver_select).w_full()),
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

        Self::config_card(
            "LED Settings",
            "Adjust visual feedback behavior",
            Icon::default().path("icons/microchip.svg"),
            content,
            theme,
        )
    }

    fn render_touch_card(&self, theme: &Theme) -> impl IntoElement {
        let content = v_flex().gap_4().child(
            v_flex()
                .gap_2()
                .child("Touch Timeout (seconds)")
                .child(Input::new(&self.touch_timeout_input)),
        );

        Self::config_card(
            "Touch & Timing",
            "Configure interaction timeouts",
            Icon::default().path("icons/settings.svg"),
            content,
            theme,
        )
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

        Self::config_card(
            "Device Options",
            "Toggle advanced features",
            Icon::default().path("icons/settings.svg"),
            content,
            &theme,
        )
    }

    fn config_card(
        title: &str,
        description: &str,
        icon: Icon,
        content: impl IntoElement,
        theme: &Theme,
    ) -> impl IntoElement {
        div()
            .w_full()
            .bg(rgb(0x18181b)) // Using the same bg as home card
            .border_1()
            .border_color(theme.border)
            .rounded_xl()
            .p_6()
            .child(
                v_flex()
                    .gap_6()
                    .child(
                        v_flex()
                            .gap_1()
                            .child(
                                gpui_component::h_flex()
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
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(theme.muted_foreground)
                                    .child(description.to_string()),
                            ),
                    )
                    .child(content),
            )
    }
}

impl Render for ConfigView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // I need to call mutable methods first.
        let led_card = self.render_led_card(cx).into_any_element();
        let options_card = self.render_options_card(cx).into_any_element();

        // Then get theme and render rest
        let theme = cx.theme();

        let identity_card = self.render_identity_card(theme).into_any_element();
        let touch_card = self.render_touch_card(theme).into_any_element();

        PageView::build(
            "Configuration",
            "Customize device settings and behavior.",
            v_flex()
                .gap_6()
                .child(
                    div()
                        .grid()
                        .grid_cols(2)
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
                            .on_click(|_, _, _| {
                                println!("Save clicked");
                            }),
                    ),
                ),
            &theme,
        )
    }
}
