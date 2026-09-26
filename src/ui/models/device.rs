//! Sole gateway from the UI layer to the HAL.
//!
//! [`DeviceRepo`] is the **only** entity that imports from `crate::hal`.
//! Views and ViewModels must never import `crate::hal` directly — they
//! interact with the hardware exclusively through `DeviceRepo` methods
//! and the types re-exported below.
//!
//! # Architecture
//!
//! - **Blocking static methods** (`*_blocking`) wrap HAL I/O calls for use
//!   inside background tasks spawned by ViewModels.
//! - **`refresh()`** performs a full polling cycle and emits
//!   [`DeviceEvent::Updated`].
//! - **`apply_fresh_state()`** lets ViewModels push post-write HAL results
//!   back into the repo so subscribers get the event.

use crate::hal::firmwares::AnyFirmware;
use crate::hal::io;
use crate::hal::types;
use gpui::*;
use std::time::Duration;

/// How often the hot-plug watcher samples device presence. Only a *change*
/// triggers a refresh, so this is a detection-latency knob, not a poll cost.
const HOTPLUG_POLL_MS: u64 = 1000;

pub use crate::hal::applets::oath;
pub use crate::hal::applets::openpgp;
pub use crate::hal::applets::otp;
pub use crate::hal::applets::piv;
pub use crate::hal::applets::{OathFeatures, OpenPgpFeatures, OtpFeatures, PivFeatures};
pub use crate::hal::fido::AttStatus;
pub use crate::hal::fido::audit;
pub use crate::hal::fido::backup;
pub use crate::hal::io::MgmAuth;
pub use crate::hal::offboard::OffboardReport;
pub use crate::hal::rescue::constants::{
    LedColor, LedStatus, USB_CAP_FIDO2, USB_CAP_OATH, USB_CAP_OPENPGP, USB_CAP_OTP, USB_CAP_PIV,
    USB_CAP_U2F,
};
pub use types::{
    AppConfigInput, DeviceMethod, FidoDeviceInfo, FirmwareType, FullDeviceStatus, LedStatusConfig,
    StoredCredential,
};

/// CCID (smart-card) USB interface bit in `AppConfig.enabled_usb_itf`.
const USB_ITF_CCID: u8 = 0x01;

// ── Events ──────────────────────────────────────────────────────────────────

/// Events emitted by [`DeviceRepo`] to notify subscribers of state changes.
pub enum DeviceEvent {
    /// Device details were refreshed.
    Updated,
}

impl EventEmitter<DeviceEvent> for DeviceRepo {}

// ── Snapshot returned by post-write state refresh ───────────────────────────

/// Snapshot of device state produced by a blocking HAL read.
#[derive(Clone)]
pub struct FreshDeviceState {
    pub status: types::FullDeviceStatus,
    pub led_status: Option<types::LedStatusConfig>,
    pub management_apps: Option<types::ManagementAppConfig>,
}

// ── DeviceRepo ──────────────────────────────────────────────────────────────

pub struct DeviceRepo {
    pub status: Option<types::FullDeviceStatus>,
    pub fido_info: Option<types::FidoDeviceInfo>,
    pub led_status: Option<types::LedStatusConfig>,
    pub management_apps: Option<types::ManagementAppConfig>,
    pub error: Option<String>,
    pub loading: bool,
    pub device_changed: bool,
    /// Handle to the hot-plug watcher task; dropped (cancelled) with the repo.
    hotplug_watch: Option<Task<()>>,
}

impl DeviceRepo {
    /// Create a new device repo in the disconnected state.
    pub fn new() -> Self {
        Self {
            status: None,
            fido_info: None,
            led_status: None,
            management_apps: None,
            error: None,
            loading: false,
            device_changed: false,
            hotplug_watch: None,
        }
    }

    // ── HAL static methods (blocking — call from background executor) ──────

    pub fn firmware_supports_legacy_fido_config(
        fw_type: &types::FirmwareType,
        version: &str,
    ) -> bool {
        AnyFirmware::new(fw_type.clone(), version).supports_legacy_fido_hardware_config()
    }

    // ── OATH (Accounts) blocking wrappers ─────────────────────────────────

    pub fn oath_password_required_blocking() -> Result<bool, crate::error::PFError> {
        io::oath_password_required()
    }

    pub fn oath_list_accounts_blocking(
        password: Option<String>,
    ) -> Result<Vec<oath::Account>, crate::error::PFError> {
        io::oath_list_accounts(password)
    }

    pub fn oath_add_blocking(
        password: Option<String>,
        cred: oath::NewCredential,
    ) -> Result<(), crate::error::PFError> {
        io::oath_add(password, cred)
    }

    pub fn oath_delete_blocking(
        password: Option<String>,
        id: String,
    ) -> Result<(), crate::error::PFError> {
        io::oath_delete(password, id)
    }

    pub fn oath_rename_blocking(
        password: Option<String>,
        old_id: String,
        new_id: String,
    ) -> Result<(), crate::error::PFError> {
        io::oath_rename(password, old_id, new_id)
    }

    pub fn oath_calculate_blocking(
        password: Option<String>,
        id: String,
        period: u32,
    ) -> Result<String, crate::error::PFError> {
        io::oath_calculate(password, id, period)
    }

    pub fn oath_set_password_blocking(
        current: Option<String>,
        new_password: Option<String>,
    ) -> Result<(), crate::error::PFError> {
        io::oath_set_password(current, new_password)
    }

    pub fn oath_reset_blocking() -> Result<(), crate::error::PFError> {
        io::oath_reset()
    }

    // ── OTP (Slots) blocking wrappers ─────────────────────────────────────

    pub fn otp_read_info_blocking() -> Result<[otp::SlotInfo; 4], crate::error::PFError> {
        io::otp_read_info()
    }

    pub fn otp_program_chalresp_blocking(
        slot: u8,
        secret: Vec<u8>,
        touch: bool,
        new_acc: [u8; 6],
        current_acc: [u8; 6],
    ) -> Result<(), crate::error::PFError> {
        io::otp_program_chalresp(slot, secret, touch, new_acc, current_acc)
    }

    pub fn otp_program_hotp_blocking(
        slot: u8,
        secret: Vec<u8>,
        digits8: bool,
        append_cr: bool,
        new_acc: [u8; 6],
        current_acc: [u8; 6],
    ) -> Result<(), crate::error::PFError> {
        io::otp_program_hotp(slot, secret, digits8, append_cr, new_acc, current_acc)
    }

    pub fn otp_program_static_blocking(
        slot: u8,
        scancodes: Vec<u8>,
        append_cr: bool,
        new_acc: [u8; 6],
        current_acc: [u8; 6],
    ) -> Result<(), crate::error::PFError> {
        io::otp_program_static(slot, scancodes, append_cr, new_acc, current_acc)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn otp_program_yubico_blocking(
        slot: u8,
        public_id: Vec<u8>,
        private_id: [u8; 6],
        key: [u8; 16],
        append_cr: bool,
        new_acc: [u8; 6],
        current_acc: [u8; 6],
    ) -> Result<(), crate::error::PFError> {
        io::otp_program_yubico(
            slot,
            public_id,
            private_id,
            key,
            append_cr,
            new_acc,
            current_acc,
        )
    }

    pub fn otp_delete_blocking(
        slot: u8,
        current_acc: [u8; 6],
    ) -> Result<(), crate::error::PFError> {
        io::otp_delete(slot, current_acc)
    }

    pub fn otp_swap_blocking(current_acc: [u8; 6]) -> Result<(), crate::error::PFError> {
        io::otp_swap(current_acc)
    }

    pub fn otp_calculate_blocking(
        slot: u8,
        challenge: Vec<u8>,
    ) -> Result<Vec<u8>, crate::error::PFError> {
        io::otp_calculate(slot, challenge)
    }

    // ── PIV blocking wrappers ─────────────────────────────────────────────

    pub fn piv_read_info_blocking() -> Result<piv::PivInfo, crate::error::PFError> {
        io::piv_read_info()
    }

    pub fn piv_change_pin_blocking(old: String, new: String) -> Result<(), crate::error::PFError> {
        io::piv_change_pin(old, new)
    }

    pub fn piv_change_puk_blocking(old: String, new: String) -> Result<(), crate::error::PFError> {
        io::piv_change_puk(old, new)
    }

    pub fn piv_unblock_pin_blocking(
        puk: String,
        new_pin: String,
    ) -> Result<(), crate::error::PFError> {
        io::piv_unblock_pin(puk, new_pin)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn piv_generate_blocking(
        slot: u8,
        algo: u8,
        pin_policy: u8,
        touch_policy: u8,
        auth: MgmAuth,
    ) -> Result<Vec<u8>, crate::error::PFError> {
        io::piv_generate(slot, algo, pin_policy, touch_policy, auth)
    }

    pub fn piv_export_cert_blocking(slot: u8) -> Result<Vec<u8>, crate::error::PFError> {
        io::piv_export_cert(slot)
    }

    pub fn piv_import_cert_blocking(
        slot: u8,
        der: Vec<u8>,
        auth: MgmAuth,
    ) -> Result<(), crate::error::PFError> {
        io::piv_import_cert(slot, der, auth)
    }

    pub fn piv_attest_blocking(slot: u8) -> Result<Vec<u8>, crate::error::PFError> {
        io::piv_attest(slot)
    }

    pub fn piv_move_key_blocking(
        src: u8,
        dst: u8,
        auth: MgmAuth,
    ) -> Result<(), crate::error::PFError> {
        io::piv_move_key(src, dst, auth)
    }

    pub fn piv_delete_key_blocking(slot: u8, auth: MgmAuth) -> Result<(), crate::error::PFError> {
        io::piv_delete_key(slot, auth)
    }

    pub fn piv_import_key_blocking(
        slot: u8,
        key_file: Vec<u8>,
        auth: MgmAuth,
    ) -> Result<(), crate::error::PFError> {
        io::piv_import_key(slot, key_file, auth)
    }

    pub fn piv_delete_cert_blocking(slot: u8, auth: MgmAuth) -> Result<(), crate::error::PFError> {
        io::piv_delete_cert(slot, auth)
    }

    pub fn piv_set_mgm_blocking(
        current: MgmAuth,
        new_algo: u8,
        new_key: Vec<u8>,
        touch: bool,
    ) -> Result<(), crate::error::PFError> {
        io::piv_set_mgm(current, new_algo, new_key, touch)
    }

    pub fn piv_set_retries_blocking(
        auth: MgmAuth,
        pin: String,
        pin_tries: u8,
        puk_tries: u8,
    ) -> Result<(), crate::error::PFError> {
        io::piv_set_retries(auth, pin, pin_tries, puk_tries)
    }

    pub fn piv_reset_blocking() -> Result<(), crate::error::PFError> {
        io::piv_reset()
    }

    // ── OpenPGP blocking wrappers ─────────────────────────────────────────

    pub fn openpgp_read_info_blocking() -> Result<openpgp::PgpInfo, crate::error::PFError> {
        io::openpgp_read_info()
    }

    pub fn openpgp_change_user_pin_blocking(
        old: String,
        new: String,
    ) -> Result<(), crate::error::PFError> {
        io::openpgp_change_user_pin(old, new)
    }

    pub fn openpgp_change_admin_pin_blocking(
        old: String,
        new: String,
    ) -> Result<(), crate::error::PFError> {
        io::openpgp_change_admin_pin(old, new)
    }

    pub fn openpgp_unblock_with_code_blocking(
        rc: String,
        new_pin: String,
    ) -> Result<(), crate::error::PFError> {
        io::openpgp_unblock_with_code(rc, new_pin)
    }

    pub fn openpgp_unblock_with_admin_blocking(
        admin: String,
        new_pin: String,
    ) -> Result<(), crate::error::PFError> {
        io::openpgp_unblock_with_admin(admin, new_pin)
    }

    pub fn openpgp_set_reset_code_blocking(
        admin: String,
        new_rc: String,
    ) -> Result<(), crate::error::PFError> {
        io::openpgp_set_reset_code(admin, new_rc)
    }

    pub fn openpgp_set_cardholder_blocking(
        admin: String,
        name: String,
        login: String,
        url: String,
        lang: String,
        sex: u8,
    ) -> Result<(), crate::error::PFError> {
        io::openpgp_set_cardholder(admin, name, login, url, lang, sex)
    }

    pub fn openpgp_set_touch_blocking(
        admin: String,
        slot: openpgp::PgpSlot,
        on: bool,
    ) -> Result<(), crate::error::PFError> {
        io::openpgp_set_touch(admin, slot, on)
    }

    pub fn openpgp_generate_blocking(
        admin: String,
        slot: openpgp::PgpSlot,
        choice: u8,
    ) -> Result<(), crate::error::PFError> {
        io::openpgp_generate(admin, slot, choice)
    }

    pub fn openpgp_reset_blocking() -> Result<(), crate::error::PFError> {
        io::openpgp_reset()
    }

    // ── Applet gating (read already-held device state) ────────────────────

    /// OATH feature profile of the connected firmware, if it exposes the applet.
    pub fn oath_features(&self) -> Option<OathFeatures> {
        let status = self.status.as_ref()?;
        AnyFirmware::new(status.firmware_type.clone(), &status.info.firmware_version)
            .oath_features()
    }

    /// OTP feature profile of the connected firmware, if it exposes the applet.
    pub fn otp_features(&self) -> Option<OtpFeatures> {
        let status = self.status.as_ref()?;
        AnyFirmware::new(status.firmware_type.clone(), &status.info.firmware_version).otp_features()
    }

    /// PIV feature profile of the connected firmware, if it exposes the applet.
    pub fn piv_features(&self) -> Option<PivFeatures> {
        let status = self.status.as_ref()?;
        AnyFirmware::new(status.firmware_type.clone(), &status.info.firmware_version).piv_features()
    }

    /// OpenPGP feature profile of the connected firmware, if it exposes the applet.
    pub fn openpgp_features(&self) -> Option<OpenPgpFeatures> {
        let status = self.status.as_ref()?;
        AnyFirmware::new(status.firmware_type.clone(), &status.info.firmware_version)
            .openpgp_features()
    }

    /// Whether an applet capability bit is enabled in USB Applications. Lenient
    /// when the mask is unknown — the SELECT then gives the authoritative answer.
    pub fn applet_enabled(&self, cap: u16) -> bool {
        self.management_apps
            .as_ref()
            .map(|m| m.usb_enabled & cap != 0)
            .unwrap_or(true)
    }

    /// Whether the CCID/smart-card USB interface is on (lenient when unknown).
    pub fn ccid_on(&self) -> bool {
        self.status
            .as_ref()
            .and_then(|s| s.config.enabled_usb_itf)
            .map(|m| m & USB_ITF_CCID != 0)
            .unwrap_or(true)
    }

    pub fn read_device_state_blocking() -> Result<FreshDeviceState, crate::error::PFError> {
        let status = io::read_device_details()?;
        let (led_status, management_apps) = if status.firmware_type == types::FirmwareType::RSKey {
            (
                io::read_led_config(status.method.clone()).ok(),
                io::read_management_config(status.method.clone()).ok(),
            )
        } else {
            (None, None)
        };
        Ok(FreshDeviceState {
            status,
            led_status,
            management_apps,
        })
    }

    pub fn write_all_config_blocking(
        method: DeviceMethod,
        phy: Option<types::AppConfigInput>,
        led: Option<LedStatusConfig>,
        apps: Option<u16>,
        pin: Option<String>,
    ) -> Result<String, crate::error::PFError> {
        io::write_all_config(method, phy, led, apps, pin)
    }

    pub fn get_fido_info_blocking() -> Result<types::FidoDeviceInfo, String> {
        io::get_fido_info()
    }

    pub fn get_credentials_blocking(pin: String) -> Result<Vec<types::StoredCredential>, String> {
        io::get_credentials(pin)
    }

    pub fn delete_credential_blocking(
        pin: String,
        credential_id: String,
    ) -> Result<String, String> {
        io::delete_credential(pin, credential_id)
    }

    pub fn change_fido_pin_blocking(
        current: Option<String>,
        new: String,
    ) -> Result<String, String> {
        io::change_fido_pin(current, new)
    }

    pub fn set_min_pin_length_blocking(pin: String, min_len: u8) -> Result<String, String> {
        io::set_min_pin_length(pin, min_len)
    }

    pub fn get_enterprise_attestation_csr_blocking() -> Result<String, String> {
        io::get_enterprise_attestation_csr()
    }

    pub fn upload_enterprise_attestation_cert_blocking(
        pin: String,
        cert_path: String,
    ) -> Result<String, String> {
        io::upload_enterprise_attestation_cert(pin, cert_path)
    }

    pub fn enable_enterprise_attestation_blocking(pin: String) -> Result<String, String> {
        io::enable_enterprise_attestation(pin)
    }

    pub fn reset_device_blocking() -> Result<String, String> {
        io::reset_device()
    }

    // ── Audit journal blocking wrappers ───────────────────────────────────

    pub fn audit_log_blocking(pin: Option<String>) -> Result<audit::AuditJournal, String> {
        io::audit_log(pin)
    }

    pub fn audit_verify_blocking(
        pin: Option<String>,
        expect_key: Option<String>,
    ) -> Result<audit::AuditVerification, String> {
        io::audit_verify(pin, expect_key)
    }

    pub fn audit_status_blocking() -> Result<bool, String> {
        io::audit_status()
    }

    pub fn audit_set_enabled_blocking(on: bool, pin: Option<String>) -> Result<bool, String> {
        io::audit_set_enabled(on, pin)
    }

    // ── Seed backup blocking wrappers ─────────────────────────────────────

    pub fn backup_status_blocking() -> Result<backup::BackupStatus, String> {
        io::backup_status()
    }

    pub fn backup_finalize_blocking() -> Result<(), String> {
        io::backup_finalize()
    }

    pub fn backup_export_blocking(pin: Option<String>) -> Result<String, String> {
        io::backup_export(pin)
    }

    pub fn backup_restore_blocking(pin: Option<String>, mnemonic: String) -> Result<(), String> {
        io::backup_restore(pin, mnemonic)
    }

    // ── At-rest soft lock blocking wrappers ───────────────────────────────

    pub fn lock_enable_blocking(pin: String) -> Result<String, String> {
        io::lock_enable(pin)
    }

    pub fn lock_unlock_blocking(mnemonic: String) -> Result<(), String> {
        io::lock_unlock(mnemonic)
    }

    pub fn lock_disable_blocking(pin: String, mnemonic: String) -> Result<(), String> {
        io::lock_disable(pin, mnemonic)
    }

    // ── Org attestation blocking wrappers ─────────────────────────────────

    pub fn att_status_blocking() -> Result<AttStatus, String> {
        io::att_status()
    }

    pub fn att_clear_blocking(pin: Option<String>) -> Result<(), String> {
        io::att_clear(pin)
    }

    pub fn att_import_blocking(
        pin: Option<String>,
        key_file: Vec<u8>,
        chain_file: Vec<u8>,
    ) -> Result<(), String> {
        io::att_import(pin, key_file, chain_file)
    }

    // ── Offboard blocking wrapper ─────────────────────────────────────────

    pub fn offboard_blocking(serial: String) -> Result<OffboardReport, String> {
        io::offboard(serial)
    }

    pub fn read_device_serial_blocking() -> Option<String> {
        io::read_device_details().ok().map(|s| s.info.serial)
    }

    pub fn check_hid_available_blocking() -> bool {
        crate::hal::transport::fido::HidTransport::open().is_ok()
    }

    /// Cheap, non-intrusive presence fingerprint of the attached FIDO device
    /// (`vid:pid:serial`, or `None` when absent). Enumerates only — does not
    /// open the device — so it is safe to poll from the hot-plug watcher.
    pub fn device_fingerprint_blocking() -> Option<String> {
        crate::hal::transport::fido::HidTransport::fingerprint()
    }

    // ── State mutation (called from ViewModel after background work) ───────

    /// Push a freshly-read [`FreshDeviceState`] into the repo and emit
    /// [`DeviceEvent::Updated`]. Also updates `device_changed` if the
    /// serial number differs from the previous value.
    pub fn apply_fresh_state(&mut self, state: FreshDeviceState, cx: &mut Context<Self>) {
        let old_serial = self.status.as_ref().map(|s| s.info.serial.clone());
        self.device_changed = old_serial
            .as_ref()
            .map(|s| *s != state.status.info.serial)
            .unwrap_or(true);
        self.status = Some(state.status);
        self.led_status = state.led_status;
        self.management_apps = state.management_apps;
        self.fido_info = Self::get_fido_info_blocking().ok();
        cx.emit(DeviceEvent::Updated);
        cx.notify();
    }

    /// Re-read FIDO info from the device and emit [`DeviceEvent::Updated`].
    /// ViewModels should call this instead of manually setting `repo.fido_info`.
    pub fn update_fido_info(&mut self, cx: &mut Context<Self>) {
        self.fido_info = Self::get_fido_info_blocking().ok();
        cx.emit(DeviceEvent::Updated);
        cx.notify();
    }

    // ── Polling cycle ──────────────────────────────────────────────────────

    /// Start the hot-plug watcher: a background timer that samples the device
    /// fingerprint and, whenever it changes (plug / unplug / swap), triggers a
    /// [`refresh`](Self::refresh) so every screen reflects the current key with
    /// no manual Refresh. Idempotent — a second call is a no-op. The task is
    /// owned by the repo and cancelled when it is dropped.
    pub fn start_hotplug_watch(&mut self, cx: &mut Context<Self>) {
        if self.hotplug_watch.is_some() {
            return;
        }
        let weak = cx.entity().downgrade();
        self.hotplug_watch = Some(cx.spawn(async move |_, cx| {
            // Seed with the fingerprint the initial refresh already reflects so
            // the first tick doesn't re-read an already-loaded device.
            let mut last = cx
                .background_executor()
                .spawn(async { Self::device_fingerprint_blocking() })
                .await;
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(HOTPLUG_POLL_MS))
                    .await;
                let current = cx
                    .background_executor()
                    .spawn(async { Self::device_fingerprint_blocking() })
                    .await;
                if current == last {
                    continue;
                }
                // Re-read on the main thread. Skip while a refresh/write is in
                // flight and retry next tick (don't commit `last`, or we'd drop
                // the change). Break when the repo — and thus the app — is gone.
                let refreshed = weak.update(cx, |repo, cx| {
                    if repo.loading {
                        false
                    } else {
                        repo.refresh(cx);
                        true
                    }
                });
                match refreshed {
                    Ok(true) => last = current,
                    Ok(false) => {}
                    Err(_) => break,
                }
            }
        }));
    }

    /// Initiate a device-details refresh (async, emits [`DeviceEvent::Updated`] on completion).
    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        if self.loading {
            return;
        }

        self.begin_load();

        let old_serial = self.status.as_ref().map(|s| s.info.serial.clone());

        match io::read_device_details() {
            Ok(status) => {
                self.device_changed = old_serial
                    .as_ref()
                    .map(|s| *s != status.info.serial)
                    .unwrap_or(true);
                self.status = Some(status.clone());

                match io::get_fido_info() {
                    Ok(fido) => self.fido_info = Some(fido),
                    Err(e) => {
                        log::error!("{}", crate::tr!("FIDO Info fetch failed: {}", e));
                        self.fido_info = None;
                    }
                }

                if status.firmware_type == types::FirmwareType::RSKey {
                    self.led_status = io::read_led_config(status.method.clone()).ok();
                    self.management_apps = io::read_management_config(status.method.clone()).ok();
                } else {
                    self.led_status = None;
                    self.management_apps = None;
                }
            }
            Err(e) => {
                self.set_error(crate::tr!("{}", e));
                self.device_changed = false;
            }
        }

        self.end_load();
        cx.emit(DeviceEvent::Updated);
        cx.notify();
    }

    // ── State lifecycle helpers ────────────────────────────────────────────

    /// Mark the repo as loading.
    pub fn begin_load(&mut self) {
        self.loading = true;
        self.error = None;
    }

    /// Mark the repo as finished loading.
    pub fn end_load(&mut self) {
        self.loading = false;
    }

    /// Set an error state on the repo.
    pub fn set_error(&mut self, error: String) {
        self.status = None;
        self.fido_info = None;
        self.led_status = None;
        self.management_apps = None;
        self.loading = false;
        self.error = Some(error);
    }
}
