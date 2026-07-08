//! Build script for embedding application resources.
//!
//! On Windows, embeds the application icon into the PE binary so that
//! the `.exe` and taskbar show the correct icon. On Unix this is a no-op.

#[cfg(windows)]
#[allow(clippy::single_component_path_imports)]
use tauri_winres;

/// Embed the application icon into the Windows PE binary.
#[cfg(windows)]
fn main() {
    let mut res = tauri_winres::WindowsResource::new();
    res.set_icon("static/appIcons/icon.ico");
    res.compile().unwrap();
}

#[cfg(unix)]
fn main() {}
