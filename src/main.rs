// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use gpui::*;
use gpui_component::Root;
use gpui_component::{Theme, ThemeMode};
use ui::rootview::ApplicationRoot;

mod device;
pub mod logging;
mod ui;

fn main() {
    logging::logger_init();
    let app = Application::new().with_assets(ui::assets::Assets);

    app.run(move |cx| {
        gpui_component::init(cx);
        Theme::change(ThemeMode::Dark, None, cx);

        cx.activate(true);

        let mut window_size = size(px(1280.0), px(720.0));

        // Basically, make sure that the window is max to max 85 percent size of the actual
        // monitor/display, so the window does not get too big on small monitors.
        if let Some(display) = cx.primary_display() {
            let display_size = display.bounds().size;

            window_size.width = window_size.width.min(display_size.width * 0.85);
            window_size.height = window_size.height.min(display_size.height * 0.85);
        }

        let window_bounds = Bounds::centered(None, window_size, cx);

        cx.spawn(async move |cx| {
            let window_options = WindowOptions {
                app_id: Some("in.suyogtandel.picoforge".into()),

                window_bounds: Some(WindowBounds::Windowed(window_bounds)),

                titlebar: Some(TitlebarOptions {
                    title: Some("PicoForge".into()),
                    appears_transparent: true,
                    // TODO: This option needs to be tested and adjusted on macos
                    traffic_light_position: Some(gpui::point(px(12.0), px(12.0))),
                }),

                #[cfg(any(target_os = "linux", target_os = "freebsd"))]
                window_background: gpui::WindowBackgroundAppearance::Transparent,
                #[cfg(any(target_os = "linux", target_os = "freebsd"))]
                window_decorations: Some(gpui::WindowDecorations::Client),

                window_min_size: Some(gpui::Size {
                    width: px(450.),
                    height: px(200.),
                }),
                kind: WindowKind::Normal,
                ..Default::default()
            };

            cx.open_window(window_options, |window, cx| {
                let view = cx.new(|cx| ApplicationRoot::new(cx));
                cx.new(|cx| Root::new(view, window, cx))
            })?;

            Ok::<_, anyhow::Error>(())
        })
        .detach();
    });
}
