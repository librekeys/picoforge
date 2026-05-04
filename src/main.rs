// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::rc::Rc;

use gpui::*;
use gpui_component::Root;
use gpui_component::{Theme, ThemeMode, ThemeSet};
use ui::types::ActiveView;
use ui::rootview::ApplicationRoot;

mod device;
pub mod error;
pub mod logging;
mod tray;
mod ui;

fn build_main_window_root(
    initial_view: Option<ActiveView>,
    window: &mut Window,
    cx: &mut App,
) -> Entity<Root> {
    let view = cx.new(|cx| {
        let mut view = ApplicationRoot::new(cx);
        if let Some(active_view) = initial_view {
            view.layout.active_view = active_view;
        }
        view
    });
    window.focus(&view.read(cx).focus_handle());
    cx.new(|cx| Root::new(view, window, cx))
}

pub fn open_main_window_app(cx: &mut App, initial_view: Option<ActiveView>) -> anyhow::Result<()> {
    let mut window_size = size(px(1344.0), px(756.0));

    if let Some(display) = cx.primary_display() {
        let display_size = display.bounds().size;
        window_size.width = window_size.width.min(display_size.width * 0.85);
        window_size.height = window_size.height.min(display_size.height * 0.85);
    }

    let window_bounds = Bounds::centered(None, window_size, cx);
    let window_options = WindowOptions {
        app_id: Some("in.suyogtandel.picoforge".into()),
        window_bounds: Some(WindowBounds::Windowed(window_bounds)),
        titlebar: Some(TitlebarOptions {
            title: Some("PicoForge".into()),
            appears_transparent: true,
            traffic_light_position: Some(gpui::point(px(9.0), px(9.0))),
        }),
        #[cfg(any(target_os = "linux", target_os = "freebsd"))]
        window_background: gpui::WindowBackgroundAppearance::Transparent,
        #[cfg(any(target_os = "linux", target_os = "freebsd"))]
        window_decorations: Some(gpui::WindowDecorations::Client),
        window_min_size: Some(gpui::Size {
            width: px(450.),
            height: px(400.),
        }),
        kind: WindowKind::Normal,
        ..Default::default()
    };

    let window = cx.open_window(window_options, move |window, cx| {
        build_main_window_root(initial_view, window, cx)
    })?;

    window.update(cx, |_, window, cx| {
        window.on_window_should_close(cx, |_window, cx| {
            if crate::tray::should_close_to_tray() {
                cx.hide();
                false
            } else {
                true
            }
        });
    })?;

    Ok(())
}

pub fn open_main_window_async(
    cx: &mut AsyncApp,
    initial_view: Option<ActiveView>,
) -> anyhow::Result<()> {
    let window_options = WindowOptions {
        app_id: Some("in.suyogtandel.picoforge".into()),
        titlebar: Some(TitlebarOptions {
            title: Some("PicoForge".into()),
            appears_transparent: true,
            traffic_light_position: Some(gpui::point(px(9.0), px(9.0))),
        }),
        #[cfg(any(target_os = "linux", target_os = "freebsd"))]
        window_background: gpui::WindowBackgroundAppearance::Transparent,
        #[cfg(any(target_os = "linux", target_os = "freebsd"))]
        window_decorations: Some(gpui::WindowDecorations::Client),
        window_min_size: Some(gpui::Size {
            width: px(450.),
            height: px(400.),
        }),
        kind: WindowKind::Normal,
        ..Default::default()
    };

    let window = cx.open_window(window_options, move |window, cx| {
        build_main_window_root(initial_view, window, cx)
    })?;

    window.update(cx, |_, window, cx| {
        window.on_window_should_close(cx, |_window, cx| {
            if crate::tray::should_close_to_tray() {
                cx.hide();
                false
            } else {
                true
            }
        });
    })?;

    Ok(())
}

fn main() {
    logging::logger_init();
    tray::init_tray();
    let app = Application::new().with_assets(ui::assets::Assets);

    app.run(move |cx| {
        gpui_component::init(cx);
        Theme::change(ThemeMode::Dark, None, cx);

        // Register sidebar toggle keybinding
        cx.bind_keys([gpui::KeyBinding::new(
            "ctrl-shift-d",
            ui::rootview::ToggleSidebar,
            None,
        )]);

        let theme_json = include_str!("../themes/picoforge-zinc.json");
        if let Ok(theme_set) = serde_json::from_str::<ThemeSet>(theme_json) {
            for config in theme_set.themes {
                if config.mode == ThemeMode::Dark {
                    let config = Rc::new(config);
                    Theme::global_mut(cx).apply_config(&config);
                    break;
                }
            }
        }

        cx.activate(true);

        if let Err(err) = crate::open_main_window_app(cx, None) {
            log::error!("Failed to open main window: {}", err);
            cx.quit();
        }

        // Quit the application when the window is closed (specifically needed for macOS)
        #[cfg(target_os = "macos")]
        {
            cx.on_window_closed(|cx| cx.quit()).detach();
        }
    });
}
