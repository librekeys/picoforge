// use tauri::State;

use std::sync::Mutex;
use crate::fido::PicoState;

mod types;
mod fido;
mod logging;
mod rescue;
mod io;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    logging::logger_init();
    log::info!("Initialisng PicoForge...");

    tauri::Builder::default()
        .manage(PicoState(Mutex::new(None)))
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            io::refresh_device_status,
            rescue::read_device_details,
            rescue::get_device_info,
            rescue::write_config,
            fido::get_fido_info,
            fido::change_fido_pin,
            fido::set_min_pin_length,
            fido::discover_fido_device,
            fido::connect_pico_vendor,
            fido::get_fido_memory_stats,
            fido::list_fido_credentials,
            fido::set_fido_led_brightness,
            fido::update_fido_vid_pid,
            rescue::enable_secure_boot
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
