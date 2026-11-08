//! High-level device I/O dispatching across FIDO and Rescue protocols.
//!
//! Each public function here selects the appropriate protocol path based
//! on the detected firmware type or an explicit [`DeviceMethod`] parameter.
//! Some functions (e.g. `read_device_details`) try Rescue (PC/SC) first,
//! then FIDO, and merge results to produce a complete status snapshot.

use crate::{
    error::PFError,
    hal::{
        applets::oath, applets::openpgp, applets::otp, applets::piv, fido, rescue,
        transport::DeviceHandle, transport::ccid::CcidSession, types::*,
    },
};

/// Read full device status by merging FIDO and Rescue data where available.
///
/// Tries the PC/SC Rescue channel first, then also attempts FIDO HID. When both
/// succeed, the function combines fields from both results (for example,
/// serial and flash data from Rescue with the AAGUID from FIDO).
pub fn read_device_details() -> Result<FullDeviceStatus, PFError> {
    let mut fido_status: Option<FullDeviceStatus> = None;
    let mut rescue_status: Option<FullDeviceStatus> = None;
    let mut rescue_fw_type: Option<FirmwareType> = None;

    // Discover via Rescue/PC/SC transport (preferred for richer details)
    match DeviceHandle::try_rescue() {
        Ok(Some((handle, _identity))) => {
            rescue_fw_type = Some(handle.firmware_type());
            match rescue::read_device_details() {
                Ok(status) => {
                    log::info!("Rescue device details read successfully");
                    rescue_status = Some(status);
                }
                Err(e) => log::warn!("Rescue read_device_details failed: {}", e),
            }
        }
        Ok(None) => log::info!("No Rescue PC/SC device found"),
        Err(e) => log::warn!("Rescue PC/SC discovery error: {}", e),
    }

    // Discover via FIDO/HID transport (fallback for FIDO-only details)
    match DeviceHandle::try_fido() {
        Ok(Some((_handle, _identity))) => match fido::read_device_details() {
            Ok(status) => {
                log::info!("FIDO device details read successfully");
                fido_status = Some(status);
            }
            Err(e) => log::warn!("FIDO read_device_details failed: {}", e),
        },
        Ok(None) => log::info!("No FIDO HID device found"),
        Err(e) => log::warn!("FIDO HID discovery error: {}", e),
    }

    match (fido_status, rescue_status) {
        (Some(fido), Some(rescue)) => {
            log::info!("Merging FIDO and Rescue device details");
            Ok(FullDeviceStatus {
                info: DeviceInfo {
                    serial: rescue.info.serial,
                    flash_used: rescue.info.flash_used,
                    flash_total: rescue.info.flash_total,
                    firmware_version: fido.info.firmware_version,
                    // The USB bcdDevice and iManufacturer come from the FIDO
                    // transport; the Rescue PC-SC channel has no USB descriptor.
                    bcd_device: fido.info.bcd_device,
                    manufacturer: fido.info.manufacturer,
                    // Object count / chip size come from the Rescue FlashInfo.
                    flash_files: rescue.info.flash_files,
                    flash_chip_size: rescue.info.flash_chip_size,
                },
                config: AppConfig {
                    vid: if !rescue.config.vid.is_empty() {
                        rescue.config.vid
                    } else {
                        fido.config.vid
                    },
                    pid: if !rescue.config.pid.is_empty() {
                        rescue.config.pid
                    } else {
                        fido.config.pid
                    },
                    led_gpio: rescue.config.led_gpio.or(fido.config.led_gpio),
                    led_brightness: rescue.config.led_brightness.or(fido.config.led_brightness),
                    led_dimmable: rescue.config.led_dimmable,
                    power_cycle_on_reset: rescue.config.power_cycle_on_reset,
                    led_steady: rescue.config.led_steady,
                    enable_secp256k1: rescue.config.enable_secp256k1,
                    led_driver: rescue.config.led_driver.or_else(|| {
                        if fido.config.led_driver.is_some() {
                            fido.config.led_driver
                        } else {
                            None
                        }
                    }),
                    // Prefer the phy record's product override; fall back to the
                    // FIDO transport's USB product string when it has none.
                    product_name: if !rescue.config.product_name.is_empty() {
                        rescue.config.product_name
                    } else {
                        fido.config.product_name
                    },
                    manufacturer_name: if !rescue.config.manufacturer_name.is_empty() {
                        rescue.config.manufacturer_name
                    } else {
                        fido.config.manufacturer_name
                    },
                    touch_timeout: rescue.config.touch_timeout.or(fido.config.touch_timeout),
                    raw_curves_mask: rescue.config.raw_curves_mask,
                    led_order: rescue.config.led_order,
                    enabled_usb_itf: rescue.config.enabled_usb_itf,
                    led_num: rescue.config.led_num,
                    // Effective values are only reported over FIDO (CONFIG_READ
                    // key 2); the rescue phy read has no equivalent.
                    effective_led_gpio: fido.config.effective_led_gpio,
                    effective_led_driver: fido.config.effective_led_driver,
                    effective_touch_timeout: fido.config.effective_touch_timeout,
                },
                secure_boot: rescue.secure_boot,
                secure_lock: rescue.secure_lock,
                method: DeviceMethod::Rescue,
                firmware_type: fido.firmware_type,
            })
        }
        (Some(fido), None) => {
            log::info!("Using FIDO-only device details");
            Ok(FullDeviceStatus {
                firmware_type: fido.firmware_type,
                ..fido
            })
        }
        (None, Some(rescue)) => {
            log::info!("Using Rescue-only device details");
            let ft = rescue_fw_type.filter(|_| rescue.firmware_type == FirmwareType::Unknown);
            Ok(FullDeviceStatus {
                firmware_type: ft.unwrap_or(rescue.firmware_type),
                ..rescue
            })
        }
        (None, None) => {
            log::error!("Failed to read device details via both FIDO and Rescue");
            Err(PFError::NoDevice)
        }
    }
}

#[allow(dead_code)]
/// Enable or lock secure boot on the device (Rescue-only operation).
pub fn enable_secure_boot(lock: bool) -> Result<String, PFError> {
    rescue::enable_secure_boot(lock)
}

#[allow(dead_code)]
/// Reboot the device (normal or BOOTSEL mode) via the Rescue channel.
pub fn reboot(to_bootsel: bool) -> Result<String, PFError> {
    rescue::reboot_device(to_bootsel)
}

/// Write device configuration, selecting FIDO or Rescue path by method.
///
/// The FIDO path requires a PIN; the Rescue path does not.
pub fn write_config(
    config: AppConfigInput,
    method: DeviceMethod,
    pin: Option<String>,
) -> Result<String, PFError> {
    if method == DeviceMethod::Fido {
        fido::write_config(config, pin)
    } else {
        rescue::write_config(config)
    }
}

/// Read the LED status configuration via the specified transport method.
pub fn read_led_config(method: DeviceMethod) -> Result<LedStatusConfig, PFError> {
    match method {
        DeviceMethod::Fido => {
            let transport = crate::hal::transport::fido::HidTransport::open()?;
            fido::read_rskey_led_config(&transport)
        }
        DeviceMethod::Rescue => rescue::read_led_config(),
    }
}

/// Write LED status configuration (all four status slots) via the specified transport.
pub fn write_led_config(
    method: DeviceMethod,
    config: LedStatusConfig,
    pin: Option<String>,
) -> Result<String, PFError> {
    match method {
        DeviceMethod::Fido => {
            let pin = pin.ok_or_else(|| {
                PFError::Device("PIN is required for FIDO LED config write".into())
            })?;
            let transport = crate::hal::transport::fido::HidTransport::open()?;
            fido::write_rskey_led_config(&transport, &config, &pin)
        }
        DeviceMethod::Rescue => {
            for i in 0..4 {
                let (color, brightness) = config.statuses[i];
                rescue::write_led_status(i as u8, color, brightness, config.steady)?;
            }
            Ok("LED configuration applied successfully.".to_string())
        }
    }
}

/// Read USB interface configuration from the Management applet.
pub fn read_management_config(method: DeviceMethod) -> Result<ManagementAppConfig, PFError> {
    match method {
        DeviceMethod::Fido => {
            let transport = crate::hal::transport::fido::HidTransport::open()?;
            let info = fido::read_rskey_management_info(&transport)?;
            // Absent USB_ENABLED → all supported apps enabled (firmware default),
            // not all-disabled, so a device without the field doesn't false-gate.
            let usb_supported = info.usb_supported.unwrap_or(0);
            Ok(ManagementAppConfig {
                usb_supported,
                usb_enabled: info.usb_enabled.unwrap_or(usb_supported),
            })
        }
        DeviceMethod::Rescue => rescue::read_management_config(),
    }
}

/// Write the USB interface enable mask via the specified transport.
pub fn write_management_config(
    method: DeviceMethod,
    enabled_mask: u16,
    pin: Option<String>,
) -> Result<String, PFError> {
    match method {
        DeviceMethod::Fido => {
            let pin = pin.ok_or_else(|| {
                PFError::Device("PIN is required for FIDO management config write".into())
            })?;
            let transport = crate::hal::transport::fido::HidTransport::open()?;
            fido::write_rskey_dev_config(&transport, enabled_mask, &pin)
        }
        DeviceMethod::Rescue => rescue::write_management_config(enabled_mask),
    }
}

/// Apply every changed configuration domain in one PIN/touch ceremony, so the
/// Configuration screen has a single Save. Order is load-bearing: the LED-status
/// and USB-applications writes run first (no reboot), then the phy record LAST —
/// an RS-Key phy write warm-reboots and re-enumerates the device.
pub fn write_all_config(
    method: DeviceMethod,
    phy: Option<AppConfigInput>,
    led: Option<LedStatusConfig>,
    apps: Option<u16>,
    pin: Option<String>,
) -> Result<String, PFError> {
    let mut done = Vec::new();
    if let Some(led) = led {
        write_led_config(method.clone(), led, pin.clone())?;
        done.push("LED colours");
    }
    if let Some(mask) = apps {
        write_management_config(method.clone(), mask, pin.clone())?;
        done.push("USB applications");
    }
    if let Some(phy) = phy {
        write_config(phy, method, pin)?;
        done.push("device settings");
    }
    if done.is_empty() {
        return Ok("No changes to apply.".to_string());
    }
    Ok(format!("Applied {}.", done.join(", ")))
}

/// Retrieve the FIDO authenticator metadata (GetInfo) as [`FidoDeviceInfo`].
pub(crate) fn get_fido_info() -> Result<FidoDeviceInfo, String> {
    fido::get_fido_info()
}

/// Change the FIDO PIN from `current_pin` to `new_pin`.
pub(crate) fn change_fido_pin(
    current_pin: Option<String>,
    new_pin: String,
) -> Result<String, String> {
    fido::change_fido_pin(current_pin, new_pin)
}

/// Set a new minimum PIN length on the authenticator.
pub(crate) fn set_min_pin_length(
    current_pin: String,
    min_pin_length: u8,
) -> Result<String, String> {
    fido::set_min_pin_length(current_pin, min_pin_length)
}

/// Enumerate all credentials stored on the authenticator.
pub fn get_credentials(pin: String) -> Result<Vec<StoredCredential>, String> {
    fido::get_credentials(pin)
}

/// Delete a credential from the authenticator by credential ID.
pub fn delete_credential(pin: String, credential_id: String) -> Result<String, String> {
    fido::delete_credential(pin, credential_id)
}

/// Perform a factory reset on the authenticator.
pub fn reset_device() -> Result<String, String> {
    fido::reset_device()
}

/// Enable enterprise attestation on the authenticator.
pub fn enable_enterprise_attestation(pin: String) -> Result<String, String> {
    fido::enable_enterprise_attestation(pin)
}

/// Retrieve the enterprise attestation CSR from the authenticator.
pub fn get_enterprise_attestation_csr() -> Result<String, String> {
    fido::get_enterprise_attestation_csr()
}

/// Upload an X.509 certificate for enterprise attestation.
pub fn upload_enterprise_attestation_cert(
    pin: String,
    cert_path: String,
) -> Result<String, String> {
    fido::upload_enterprise_attestation_cert(pin, cert_path)
}

// ── Audit journal ─────────────────────────────────────────────────────────────

/// Export the tamper-evident audit journal (PIN or touch gated).
pub fn audit_log(pin: Option<String>) -> Result<fido::audit::AuditJournal, String> {
    fido::audit_log(pin)
}

/// Export + verify a DEVK-signed checkpoint; `expect_key` pins the identity.
pub fn audit_verify(
    pin: Option<String>,
    expect_key: Option<String>,
) -> Result<fido::audit::AuditVerification, String> {
    fido::audit_verify(pin, expect_key)
}

/// Whether the audit journal is on (ungated, no touch).
pub fn audit_status() -> Result<bool, String> {
    fido::audit_status()
}

/// Turn the audit journal on/off (PIN + touch); returns the resulting state.
pub fn audit_set_enabled(on: bool, pin: Option<String>) -> Result<bool, String> {
    fido::audit_set_enabled(on, pin)
}

// ── Seed backup ───────────────────────────────────────────────────────────────

/// Read `{sealed, has_seed, locked, unlocked}`.
pub fn backup_status() -> Result<fido::backup::BackupStatus, String> {
    fido::backup_status()
}

/// Seal the one-time export window (touch).
pub fn backup_finalize() -> Result<(), String> {
    fido::backup_finalize()
}

/// Export the master seed as a 24-word BIP-39 phrase.
pub fn backup_export(pin: Option<String>) -> Result<String, String> {
    fido::backup_export(pin)
}

/// Restore a seed from a 24-word BIP-39 phrase.
pub fn backup_restore(pin: Option<String>, mnemonic: String) -> Result<(), String> {
    fido::backup_restore(pin, mnemonic)
}

// ── At-rest soft lock ─────────────────────────────────────────────────────────

/// Engage the lock; returns the lock key as a 24-word phrase.
pub fn lock_enable(pin: String) -> Result<String, String> {
    fido::lock_enable(pin)
}

/// Unlock the seed for this power cycle (BIP-39 lock key).
///
/// `pin` is optional: RS-Key never needs it for UNLOCK; a fapico2 board with
/// a PIN set does (divergence D8) and the retry only fires when the device
/// answers "PIN required".
pub fn lock_unlock(pin: Option<String>, mnemonic: String) -> Result<(), String> {
    fido::lock_unlock(pin, mnemonic)
}

/// Disable the lock (unlock + restore plaintext).
pub fn lock_disable(pin: String, mnemonic: String) -> Result<(), String> {
    fido::lock_disable(pin, mnemonic)
}

// ── Org attestation ───────────────────────────────────────────────────────────

/// Read `{installed, chain_hash}`.
pub fn att_status() -> Result<fido::AttStatus, String> {
    fido::att_status()
}

/// Remove the org attestation.
pub fn att_clear(pin: Option<String>) -> Result<(), String> {
    fido::att_clear(pin)
}

/// Install an org attestation P-256 key + cert chain.
pub fn att_import(
    pin: Option<String>,
    key_file: Vec<u8>,
    chain_file: Vec<u8>,
) -> Result<(), String> {
    fido::att_import(pin, key_file, chain_file)
}

// ── Offboard (guided full wipe + signed receipt) ──────────────────────────────

/// Wipe every applet, factory-reset FIDO, clear any org attestation, then sign
/// an audit checkpoint over the post-wipe journal. Each step is best-effort and
/// recorded; the receipt is signed only if the checkpoint verifies and contains
/// the RESET event. PIN-free by design (block-then-reset / touch-gated paths).
pub fn offboard(serial: String) -> Result<crate::hal::offboard::OffboardReport, String> {
    use crate::hal::offboard::{OffboardReport, OffboardStep};

    let mut steps: Vec<OffboardStep> = Vec::new();
    let step = |name: &str, res: Result<(), String>| OffboardStep {
        name: name.to_string(),
        ok: res.is_ok(),
        detail: res.err().unwrap_or_else(|| "ok".to_string()),
    };

    // CCID applets first (each opens its own connection).
    {
        let mut ok = true;
        let mut detail = String::from("ok");
        for slot in 1..=4u8 {
            if let Err(e) = otp_delete(slot, [0u8; 6]) {
                ok = false;
                detail = format!("slot {slot}: {e}");
                break;
            }
        }
        steps.push(OffboardStep {
            name: "otp".into(),
            ok,
            detail,
        });
    }
    steps.push(step("oath", oath_reset().map_err(|e| e.to_string())));
    steps.push(step("piv", piv_reset().map_err(|e| e.to_string())));
    steps.push(step("openpgp", openpgp_reset().map_err(|e| e.to_string())));

    // FIDO factory reset (touch) — must precede the checkpoint so RESET is logged.
    steps.push(step("fido_reset", reset_device().map(|_| ())));

    // Org attestation — clear only if one is installed.
    match att_status() {
        Ok(s) if s.installed => steps.push(step("org_attestation", att_clear(None))),
        Ok(_) => steps.push(OffboardStep {
            name: "org_attestation".into(),
            ok: true,
            detail: "none".into(),
        }),
        Err(e) => steps.push(OffboardStep {
            name: "org_attestation".into(),
            ok: true,
            detail: format!("status unavailable: {e}"),
        }),
    }

    // Signed receipt: checkpoint over the post-wipe journal (must hold RESET).
    let (signed, fingerprint, signed_head, signature, pubkey) = match audit_verify(None, None) {
        Ok(v) => {
            let has_reset = v
                .journal
                .entries
                .iter()
                .any(|e| e.event == fido::audit::EVT_RESET);
            if v.signature_ok && v.head_matches && has_reset {
                (
                    true,
                    Some(v.fingerprint),
                    Some(v.signed_head_hex),
                    Some(v.signature_hex),
                    Some(v.pubkey_hex),
                )
            } else {
                (false, None, None, None, None)
            }
        }
        Err(_) => (false, None, None, None, None),
    };

    Ok(OffboardReport {
        serial,
        steps,
        signed,
        fingerprint,
        signed_head,
        signature,
        pubkey,
    })
}

// ── OATH (Accounts) ─────────────────────────────────────────────────────────
//
// Each operation is one open→(validate)→op unit: SELECT resets the applet's
// security state, so the VALIDATE and the operation it authorises must share
// the same freshly-opened session.

/// Open the OATH applet and unlock it with `password` when a code is set.
fn oath_unlock(password: Option<&str>) -> Result<CcidSession, PFError> {
    let (session, info) = oath::open()?;
    if info.password_set() {
        let pw = password.ok_or_else(|| {
            PFError::Device("This device's OATH accounts are password-protected.".into())
        })?;
        let key = oath::derive_access_key(pw, &info.device_id);
        let challenge = info.challenge.clone().unwrap_or_default();
        oath::validate(&session, &key, &challenge)?;
    }
    Ok(session)
}

/// Whether the OATH applet has an access code set (needs a password to read).
pub fn oath_password_required() -> Result<bool, PFError> {
    let (_, info) = oath::open()?;
    Ok(info.password_set())
}

/// List every account with its current code (one CALCULATE ALL round-trip).
pub fn oath_list_accounts(password: Option<String>) -> Result<Vec<oath::Account>, PFError> {
    oath::calculate_all(&oath_unlock(password.as_deref())?)
}

/// Compute a single account's code (for HOTP or non-30 s / touch credentials).
pub fn oath_calculate(
    password: Option<String>,
    id: String,
    period: u32,
) -> Result<String, PFError> {
    oath::calculate(&oath_unlock(password.as_deref())?, &id, period)
}

/// Add or overwrite a credential.
pub fn oath_add(password: Option<String>, cred: oath::NewCredential) -> Result<(), PFError> {
    oath::put(&oath_unlock(password.as_deref())?, &cred)
}

/// Delete a credential by id.
pub fn oath_delete(password: Option<String>, id: String) -> Result<(), PFError> {
    oath::delete(&oath_unlock(password.as_deref())?, &id)
}

/// Rename a credential (change its issuer/account; YubiKey 5.3+ / RS-Key).
pub fn oath_rename(
    password: Option<String>,
    old_id: String,
    new_id: String,
) -> Result<(), PFError> {
    oath::rename(&oath_unlock(password.as_deref())?, &old_id, &new_id)
}

/// Set, change, or clear the applet access code. `new_password = None` clears it.
pub fn oath_set_password(
    current: Option<String>,
    new_password: Option<String>,
) -> Result<(), PFError> {
    let session = oath_unlock(current.as_deref())?;
    match new_password {
        Some(pw) if !pw.is_empty() => {
            let info = oath::parse_select(&session.select_resp);
            let key = oath::derive_access_key(&pw, &info.device_id);
            oath::set_code(&session, &key)
        }
        _ => oath::clear_code(&session),
    }
}

/// Factory-reset the OATH applet (wipes all accounts and the access code).
pub fn oath_reset() -> Result<(), PFError> {
    let (session, _) = oath::open()?;
    oath::reset(&session)
}

// ── OTP (Slots) ─────────────────────────────────────────────────────────────

/// Read per-slot status (all four slots).
pub fn otp_read_info() -> Result<[otp::SlotInfo; 4], PFError> {
    otp::read_info(&otp::open()?)
}

/// Program a slot for HMAC-SHA1 challenge-response.
pub fn otp_program_chalresp(
    slot: u8,
    secret: Vec<u8>,
    touch: bool,
    new_acc: [u8; 6],
    current_acc: [u8; 6],
) -> Result<(), PFError> {
    otp::configure(
        &otp::open()?,
        slot,
        &otp::build_chalresp(&secret, touch, &new_acc),
        &current_acc,
    )
}

/// Program a slot for OATH-HOTP.
pub fn otp_program_hotp(
    slot: u8,
    secret: Vec<u8>,
    digits8: bool,
    append_cr: bool,
    new_acc: [u8; 6],
    current_acc: [u8; 6],
) -> Result<(), PFError> {
    otp::configure(
        &otp::open()?,
        slot,
        &otp::build_hotp(&secret, digits8, append_cr, &new_acc),
        &current_acc,
    )
}

/// Program a slot for a static password (ASCII typed as HID scancodes).
pub fn otp_program_static(
    slot: u8,
    scancodes: Vec<u8>,
    append_cr: bool,
    new_acc: [u8; 6],
    current_acc: [u8; 6],
) -> Result<(), PFError> {
    otp::configure(
        &otp::open()?,
        slot,
        &otp::build_static(&scancodes, append_cr, &new_acc),
        &current_acc,
    )
}

/// Program a slot for Yubico OTP (public id ‖ private id ‖ AES key).
#[allow(clippy::too_many_arguments)]
pub fn otp_program_yubico(
    slot: u8,
    public_id: Vec<u8>,
    private_id: [u8; 6],
    key: [u8; 16],
    append_cr: bool,
    new_acc: [u8; 6],
    current_acc: [u8; 6],
) -> Result<(), PFError> {
    otp::configure(
        &otp::open()?,
        slot,
        &otp::build_yubico_otp(&public_id, &private_id, &key, append_cr, &new_acc),
        &current_acc,
    )
}

/// Delete a slot, presenting `current_acc` (all-zero for an unprotected slot).
pub fn otp_delete(slot: u8, current_acc: [u8; 6]) -> Result<(), PFError> {
    otp::delete_slot(&otp::open()?, slot, &current_acc)
}

/// Swap slots 1 and 2, presenting `current_acc` (all-zero for unprotected slots).
pub fn otp_swap(current_acc: [u8; 6]) -> Result<(), PFError> {
    otp::swap(&otp::open()?, &current_acc)
}

/// Run an HMAC-SHA1 challenge-response against a slot (returns the 20-byte MAC).
pub fn otp_calculate(slot: u8, challenge: Vec<u8>) -> Result<Vec<u8>, PFError> {
    otp::calculate_hmac(&otp::open()?, slot, &challenge)
}

// ── PIV ─────────────────────────────────────────────────────────────────────
//
// Management-gated ops (generate, import cert, set mgmt key, set retries) run
// GENERAL AUTHENTICATE and the op on the SAME session — SELECT resets auth.

/// How a management-gated op authenticates: a typed management key, or a PIN
/// that fetches the PIN-protected key on the same session (ykman `--protect`).
pub enum MgmAuth {
    Key { key: Vec<u8>, algo: u8 },
    Pin(String),
}

/// Open the PIV applet and authenticate the management key on that session.
fn piv_authed(auth: MgmAuth) -> Result<CcidSession, PFError> {
    let s = piv::open()?;
    match auth {
        MgmAuth::Key { key, algo } => piv::authenticate_mgm(&s, &key, algo)?,
        MgmAuth::Pin(pin) => {
            // --protect: fetch the random key by PIN, then authenticate — one
            // session, so the SELECT that would reset auth never intervenes.
            let key = piv::read_protected_mgm(&s, &pin)?;
            piv::authenticate_mgm(&s, &key, piv::mgm_algo_for_len(key.len()))?;
        }
    }
    Ok(s)
}

pub fn piv_read_info() -> Result<piv::PivInfo, PFError> {
    piv::read_info(&piv::open()?)
}

pub fn piv_change_pin(old: String, new: String) -> Result<(), PFError> {
    piv::change_ref(&piv::open()?, piv::REF_PIN, &old, &new)
}

pub fn piv_change_puk(old: String, new: String) -> Result<(), PFError> {
    piv::change_ref(&piv::open()?, piv::REF_PUK, &old, &new)
}

pub fn piv_unblock_pin(puk: String, new_pin: String) -> Result<(), PFError> {
    piv::unblock_pin(&piv::open()?, &puk, &new_pin)
}

pub fn piv_generate(
    slot: u8,
    algo: u8,
    pin_policy: u8,
    touch_policy: u8,
    auth: MgmAuth,
) -> Result<Vec<u8>, PFError> {
    let s = piv_authed(auth)?;
    piv::generate(&s, slot, algo, pin_policy, touch_policy)
}

pub fn piv_export_cert(slot: u8) -> Result<Vec<u8>, PFError> {
    piv::export_cert(&piv::open()?, slot)
}

pub fn piv_import_cert(slot: u8, der: Vec<u8>, auth: MgmAuth) -> Result<(), PFError> {
    let s = piv_authed(auth)?;
    piv::import_cert(&s, slot, &der)
}

pub fn piv_delete_cert(slot: u8, auth: MgmAuth) -> Result<(), PFError> {
    let s = piv_authed(auth)?;
    piv::delete_cert(&s, slot)
}

pub fn piv_set_mgm(
    current: MgmAuth,
    new_algo: u8,
    new_key: Vec<u8>,
    touch: bool,
) -> Result<(), PFError> {
    let s = piv_authed(current)?;
    piv::set_mgm(&s, new_algo, &new_key, touch)
}

pub fn piv_set_retries(
    auth: MgmAuth,
    pin: String,
    pin_tries: u8,
    puk_tries: u8,
) -> Result<(), PFError> {
    let s = piv_authed(auth)?;
    piv::verify_pin(&s, &pin)?;
    piv::set_retries(&s, pin_tries, puk_tries)
}

pub fn piv_reset() -> Result<(), PFError> {
    piv::reset(&piv::open()?)
}

/// Attestation certificate DER for a generated key.
pub fn piv_attest(slot: u8) -> Result<Vec<u8>, PFError> {
    piv::attest(&piv::open()?, slot)
}

pub fn piv_move_key(src: u8, dst: u8, auth: MgmAuth) -> Result<(), PFError> {
    let s = piv_authed(auth)?;
    piv::move_key(&s, src, dst)
}

pub fn piv_delete_key(slot: u8, auth: MgmAuth) -> Result<(), PFError> {
    let s = piv_authed(auth)?;
    piv::delete_key(&s, slot)
}

/// Import a private key from PEM/DER (PKCS8 / PKCS1 / SEC1).
pub fn piv_import_key(slot: u8, key_file: Vec<u8>, auth: MgmAuth) -> Result<(), PFError> {
    let (algo, material) = piv::parse_private_key(&key_file).map_err(PFError::Device)?;
    let s = piv_authed(auth)?;
    piv::import_key(&s, slot, algo, &material)
}

// ── OpenPGP ───────────────────────────────────────────────────────────────────
//
// Admin-gated writes (cardholder, touch, reset code, generate) run VERIFY PW3
// and the op on the SAME session — SELECT clears the verification latch.

/// Open the OpenPGP applet and verify the admin PIN (PW3) on that session.
fn openpgp_admin(admin: &str) -> Result<CcidSession, PFError> {
    let s = openpgp::open()?;
    openpgp::verify_pin(&s, openpgp::PW3, admin)?;
    Ok(s)
}

pub fn openpgp_read_info() -> Result<openpgp::PgpInfo, PFError> {
    openpgp::read_info(&openpgp::open()?)
}

pub fn openpgp_change_user_pin(old: String, new: String) -> Result<(), PFError> {
    openpgp::change_pin(&openpgp::open()?, openpgp::PW1, &old, &new)
}

pub fn openpgp_change_admin_pin(old: String, new: String) -> Result<(), PFError> {
    openpgp::change_pin(&openpgp::open()?, openpgp::PW3, &old, &new)
}

/// Unblock the user PIN with the resetting code.
pub fn openpgp_unblock_with_code(rc: String, new_pin: String) -> Result<(), PFError> {
    openpgp::unblock_with_rc(&openpgp::open()?, &rc, &new_pin)
}

/// Unblock the user PIN with the admin PIN.
pub fn openpgp_unblock_with_admin(admin: String, new_pin: String) -> Result<(), PFError> {
    openpgp::unblock_with_admin(&openpgp_admin(&admin)?, &new_pin)
}

/// Set (or clear, if `new_rc` is empty) the resetting code.
pub fn openpgp_set_reset_code(admin: String, new_rc: String) -> Result<(), PFError> {
    openpgp::set_reset_code(&openpgp_admin(&admin)?, &new_rc)
}

pub fn openpgp_set_cardholder(
    admin: String,
    name: String,
    login: String,
    url: String,
    lang: String,
    sex: u8,
) -> Result<(), PFError> {
    openpgp::set_cardholder(&openpgp_admin(&admin)?, &name, &login, &url, &lang, sex)
}

pub fn openpgp_set_touch(admin: String, slot: openpgp::PgpSlot, on: bool) -> Result<(), PFError> {
    openpgp::set_touch(&openpgp_admin(&admin)?, slot, on)
}

/// Generate a key in a slot with the given algorithm choice (see `GENERATE_ALGOS`).
pub fn openpgp_generate(admin: String, slot: openpgp::PgpSlot, choice: u8) -> Result<(), PFError> {
    let attr = openpgp::algo_attr(slot, choice).ok_or_else(|| {
        PFError::Device(format!(
            "Algorithm not supported for the {} slot",
            slot.label()
        ))
    })?;
    let s = openpgp_admin(&admin)?;
    openpgp::generate(&s, slot, &attr).map(|_| ())
}

pub fn openpgp_reset() -> Result<(), PFError> {
    openpgp::reset(&openpgp::open()?)
}
