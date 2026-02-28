use tauri_winres;

// Configures windows application resource.( fix for app icon and launching app as admin)
#[cfg(windows)]
fn main() {
    let mut res = tauri_winres::WindowsResource::new();
    res.set_icon("static/appIcons/icon.ico");
    res.compile().unwrap();
}

#[cfg(unix)]
fn main() {}
