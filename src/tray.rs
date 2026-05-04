use crate::device::oath;
use once_cell::sync::OnceCell;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

pub static QUIT_REQUESTED: AtomicBool = AtomicBool::new(false);
static COMMAND_RECEIVER: OnceCell<Mutex<Receiver<TrayCommand>>> = OnceCell::new();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayCommand {
    ShowTotpWindow,
    Quit,
}

#[cfg(target_os = "linux")]
mod linux_impl {
    use super::*;
    use tinyfiledialogs::{message_box_ok, password_box, MessageBoxIcon};
    use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
    use tray_icon::{Icon, TrayIconBuilder};

    const ID_REFRESH: &str = "tray.refresh";
    const ID_UNLOCK: &str = "tray.unlock";
    const ID_LOCK: &str = "tray.lock";
    const ID_OPEN: &str = "tray.open";
    const ID_QUIT: &str = "tray.quit";

    #[derive(Default)]
    struct TrayTotpState {
        cached_password: Option<String>,
        account_ids: BTreeMap<String, String>,
    }

    pub fn start(sender: Sender<TrayCommand>) {
        thread::spawn(move || {
            if let Err(err) = gtk::init() {
                log::error!("Failed to initialize GTK for tray: {}", err);
                return;
            }

            let icon = match generate_tray_icon() {
                Ok(icon) => icon,
                Err(err) => {
                    log::error!("Failed to create tray icon: {}", err);
                    return;
                }
            };

            let mut state = TrayTotpState::default();
            let mut tray_state = refresh_totp_menu_state(&mut state);
            let menu = build_menu(&tray_state);
            let tray_icon = match TrayIconBuilder::new()
                .with_icon(icon)
                .with_tooltip("PicoForge")
                .with_menu(Box::new(menu.clone()))
                .build()
            {
                Ok(icon) => icon,
                Err(err) => {
                    log::error!("Failed to build tray icon: {}", err);
                    return;
                }
            };

            loop {
                while gtk::events_pending() {
                    gtk::main_iteration();
                }

                while let Ok(event) = MenuEvent::receiver().try_recv() {
                    handle_menu_event(&event.id.0, &sender, &mut state, &mut tray_state, &tray_icon);
                }

                thread::sleep(Duration::from_millis(120));
            }
        });
    }

    #[derive(Clone, Default)]
    struct TrayMenuState {
        protected: bool,
        entries: Vec<(String, String)>,
        error: Option<String>,
    }

    fn generate_tray_icon() -> Result<Icon, tray_icon::BadIcon> {
        let width = 16u32;
        let height = 16u32;
        let mut rgba = vec![0u8; (width * height * 4) as usize];
        for y in 0..height {
            for x in 0..width {
                let idx = ((y * width + x) * 4) as usize;
                let connector = (5..=10).contains(&x) && (1..=4).contains(&y);
                let connector_notch = (7..=8).contains(&x) && y == 4;
                let shell = (3..=12).contains(&x) && (5..=13).contains(&y);
                let shell_inner = (4..=11).contains(&x) && (6..=12).contains(&y);
                let usb_mark_h = (6..=9).contains(&x) && y == 8;
                let usb_mark_v = x == 7 && (7..=10).contains(&y);
                let usb_mark_tip = (6..=8).contains(&x) && y == 7;
                let usb_mark = usb_mark_h || usb_mark_v || usb_mark_tip;

                let (r, g, b, a) = if connector && !connector_notch {
                    (0xD8, 0xDE, 0xE7, 0xFF)
                } else if shell {
                    if shell_inner {
                        if usb_mark {
                            (0xD8, 0xDE, 0xE7, 0xFF)
                        } else {
                            (0x2C, 0x3E, 0x57, 0xFF)
                        }
                    } else {
                        (0x0F, 0x17, 0x23, 0xFF)
                    }
                } else {
                    (0x00, 0x00, 0x00, 0x00)
                };
                rgba[idx] = r;
                rgba[idx + 1] = g;
                rgba[idx + 2] = b;
                rgba[idx + 3] = a;
            }
        }
        Icon::from_rgba(rgba, width, height)
    }

    fn build_menu(state: &TrayMenuState) -> Menu {
        let menu = Menu::new();

        let header = MenuItem::with_id("tray.header", "PicoForge", false, None);
        let _ = menu.append(&header);
        let _ = menu.append(&PredefinedMenuItem::separator());

        let codes = Submenu::with_id("tray.codes", "TOTP Codes", true);
        for (menu_id, name) in &state.entries {
            let item = MenuItem::with_id(menu_id.clone(), name, true, None);
            let _ = codes.append(&item);
        }
        if state.entries.is_empty() {
            let label = if let Some(err) = &state.error {
                format!("Unavailable: {}", err)
            } else if state.protected {
                "Locked - unlock to access codes".to_string()
            } else {
                "No TOTP accounts found".to_string()
            };
            let empty = MenuItem::with_id("tray.codes.empty", label, false, None);
            let _ = codes.append(&empty);
        }
        let _ = menu.append(&codes);

        let refresh = MenuItem::with_id(ID_REFRESH, "Refresh TOTP List", true, None);
        let unlock = MenuItem::with_id(ID_UNLOCK, "Unlock TOTP Store", true, None);
        let lock = MenuItem::with_id(ID_LOCK, "Lock TOTP Store", true, None);
        lock.set_enabled(!state.protected);
        let open = MenuItem::with_id(ID_OPEN, "Open PicoForge", true, None);
        let quit = MenuItem::with_id(ID_QUIT, "Quit", true, None);

        let _ = menu.append(&refresh);
        let _ = menu.append(&unlock);
        let _ = menu.append(&lock);
        let _ = menu.append(&PredefinedMenuItem::separator());
        let _ = menu.append(&open);
        let _ = menu.append(&quit);
        menu
    }

    fn refresh_totp_menu_state(state: &mut TrayTotpState) -> TrayMenuState {
        state.account_ids.clear();
        match oath::get_totp_status(state.cached_password.clone()) {
            Ok(status) => {
                let mut next = TrayMenuState {
                    protected: status.protected,
                    entries: Vec::new(),
                    error: None,
                };
                for (idx, entry) in status.entries.into_iter().enumerate() {
                    let menu_id = format!("tray.code.{}", idx);
                    state.account_ids.insert(menu_id.clone(), entry.name.clone());
                    next.entries.push((menu_id, entry.name));
                }
                next
            }
            Err(err) => {
                state.cached_password = None;
                TrayMenuState {
                    protected: false,
                    entries: Vec::new(),
                    error: Some(err.to_string()),
                }
            }
        }
    }

    fn prompt_and_unlock(state: &mut TrayTotpState) -> Option<TrayMenuState> {
        let password = password_box(
            "Unlock TOTP Store",
            "Enter the OATH password to unlock TOTP codes.",
        )?;

        match oath::get_totp_status(Some(password.clone())) {
            Ok(status) => {
                state.cached_password = Some(password);
                let mut next = TrayMenuState {
                    protected: status.protected,
                    entries: Vec::new(),
                    error: None,
                };
                state.account_ids.clear();
                for (idx, entry) in status.entries.into_iter().enumerate() {
                    let menu_id = format!("tray.code.{}", idx);
                    state.account_ids.insert(menu_id.clone(), entry.name.clone());
                    next.entries.push((menu_id, entry.name));
                }
                Some(next)
            }
            Err(err) => {
                state.cached_password = None;
                message_box_ok(
                    "PicoForge",
                    &format!("Failed to unlock TOTP store:\n{}", err),
                    MessageBoxIcon::Error,
                );
                None
            }
        }
    }

    fn copy_code_for_account(state: &mut TrayTotpState, account_name: &str) {
        let status = if state.cached_password.is_some() {
            oath::get_totp_status(state.cached_password.clone())
        } else {
            match prompt_and_unlock(state) {
                Some(_) => oath::get_totp_status(state.cached_password.clone()),
                None => return,
            }
        };

        let status = match status {
            Ok(status) => status,
            Err(err) => {
                state.cached_password = None;
                message_box_ok(
                    "PicoForge",
                    &format!("Failed to read TOTP code:\n{}", err),
                    MessageBoxIcon::Error,
                );
                return;
            }
        };

        let Some(entry) = status
            .entries
            .into_iter()
            .find(|entry| entry.name == account_name)
        else {
            message_box_ok(
                "PicoForge",
                "That TOTP account is no longer available on the key.",
                MessageBoxIcon::Warning,
            );
            return;
        };

        let Some(code) = entry.current_code else {
            message_box_ok(
                "PicoForge",
                "This TOTP account did not return a current code.",
                MessageBoxIcon::Warning,
            );
            return;
        };

        let Some(display) = gtk::gdk::Display::default() else {
            message_box_ok(
                "PicoForge",
                "Failed to access the GTK display for clipboard copy.",
                MessageBoxIcon::Error,
            );
            return;
        };

        let Some(clipboard) = gtk::Clipboard::default(&display) else {
            message_box_ok(
                "PicoForge",
                "Failed to access the system clipboard.",
                MessageBoxIcon::Error,
            );
            return;
        };

        clipboard.set_text(&code);
        clipboard.store();

        if clipboard.wait_for_text().as_deref() != Some(code.as_str()) {
            message_box_ok(
                "PicoForge",
                "Clipboard copy did not persist. Clipboard manager rejected the update.",
                MessageBoxIcon::Error,
            );
        }
    }

    fn handle_menu_event(
        id: &str,
        sender: &Sender<TrayCommand>,
        state: &mut TrayTotpState,
        tray_state: &mut TrayMenuState,
        tray_icon: &tray_icon::TrayIcon,
    ) {
        match id {
            ID_REFRESH => {
                *tray_state = refresh_totp_menu_state(state);
                tray_icon.set_menu(Some(Box::new(build_menu(tray_state))));
            }
            ID_UNLOCK => {
                if let Some(next) = prompt_and_unlock(state) {
                    *tray_state = next;
                    tray_icon.set_menu(Some(Box::new(build_menu(tray_state))));
                }
            }
            ID_LOCK => {
                state.cached_password = None;
                *tray_state = refresh_totp_menu_state(state);
                tray_icon.set_menu(Some(Box::new(build_menu(tray_state))));
            }
            ID_OPEN => {
                let _ = sender.send(TrayCommand::ShowTotpWindow);
            }
            ID_QUIT => {
                QUIT_REQUESTED.store(true, Ordering::SeqCst);
                let _ = sender.send(TrayCommand::Quit);
            }
            _ if id.starts_with("tray.code.") => {
                if let Some(account_name) = state.account_ids.get(id).cloned() {
                    copy_code_for_account(state, &account_name);
                    *tray_state = refresh_totp_menu_state(state);
                    tray_icon.set_menu(Some(Box::new(build_menu(tray_state))));
                }
            }
            _ => {}
        }
    }
}

pub fn init_tray() {
    let (tx, rx) = mpsc::channel();
    let _ = COMMAND_RECEIVER.set(Mutex::new(rx));
    #[cfg(target_os = "linux")]
    linux_impl::start(tx);
}

pub fn poll_command() -> Option<TrayCommand> {
    COMMAND_RECEIVER
        .get()
        .and_then(|rx| rx.lock().ok()?.try_recv().ok())
}

pub fn should_close_to_tray() -> bool {
    !QUIT_REQUESTED.load(Ordering::SeqCst)
}
