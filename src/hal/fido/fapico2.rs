//! Fapico2-only support code — **owned by the fapico2 author, not a PicoForge
//! maintainer concern** (PR #131 ownership agreement).
//!
//! Everything here is additive: no RS-Key or pico-fido code path, string, or
//! comment was changed to introduce this module. The pieces that must stay
//! wire-compatible with RS-Key live in the parent module and are reused
//! unmodified — as a child module this file can call the parent's private
//! helpers directly ([`super::build_rskey_phy_tlv`],
//! [`super::read_rskey_physical_config`], [`super::vendor_error`]).
//!
//! What is fapico2-specific (EPIC PF-FAPICO2 divergences):
//!
//! - **0x30 wording (D3/D7)** — the parent [`super::vendor_error`] keeps
//!   RS-Key's original generic denial for every op. fapico2 answers 0x30 for
//!   different reasons: LOAD refuses while resident credentials exist
//!   (reset-first gate, US-1737 — the incoming seed may not even have an
//!   installed counterpart to differ from), and EXPORT refuses both when no
//!   master seed is installed and when the export window is sealed. The
//!   wording here names those reasons; it is only reached for a device
//!   confirmed (by AAGUID, lazily, on the error path) to be fapico2.
//! - **Config write refusal set** — fapico2's `apply_phy_record`
//!   (`apps/fido/src/vendor41.rs` ~2049) rejects the whole PHY blob with
//!   `UnsupportedOption` for the tags it has no `PhyConfig` field for:
//!   Curves, PresenceTimeout, LedDriver, LedOrder, LedNum; it also refuses an
//!   all-zero USB interface mask outright. RS-Key accepts all of these, so
//!   the pre-send validation here is fapico2-only and duplicates the
//!   device-side policy on purpose: a fapico2 firmware update that starts
//!   accepting a field degrades gracefully to a device-side error, never a
//!   corrupt write.
//! - **Post-write behaviour** — fapico2 persists a config write durably and
//!   does **not** re-enumerate itself the way RS-Key v0.3.x does; USB identity
//!   (VID/PID, product, manufacturer) is applied at boot. The success message
//!   therefore asks for an unplug/re-plug instead of RS-Key's
//!   "re-enumerating" wording.

use super::constants::{RSKEY_CFG_TARGET_PHY, PinUvAuthTokenPermissions};
use super::ops::FidoOperations;
use super::{build_rskey_phy_tlv, read_device_info, read_rskey_physical_config, vendor_error};
use crate::error::PFError;
use crate::hal::transport::fido::HidTransport;
use crate::hal::types::{AppConfig, AppConfigInput, FAPICO2_AAGUID};
use serde_cbor_2::Value;
use std::collections::BTreeMap;

// ── Error wording (D3/D7) ───────────────────────────────────────────────────

/// Map a non-zero CTAP status to a message, fapico2 flavour.
///
/// Only the 0x30 arm differs from the parent's table (see the module header);
/// every other status keeps RS-Key's original wording verbatim via
/// [`vendor_error`].
fn vendor_error_fapico2(status: u8, op: &str) -> String {
    if status != 0x30 {
        return vendor_error(status, op);
    }
    // Call sites label ops with host-side hedges ("export (already sealed?)");
    // strip the parenthetical so the user never sees an internal guess.
    let label = op.split(" (").next().unwrap_or(op);
    match label {
        "restore" => format!(
            "{label}: operation not allowed — the board holds resident credentials; \
             reset the device first, then restore"
        ),
        "export" => format!(
            "{label}: operation not allowed — the one-time export window is sealed, \
             or the board has no master seed to export"
        ),
        _ => vendor_error(status, op),
    }
}

/// Resolve the fapico2 0x30 wording only when the connected device really is
/// a fapico2; every other case (and any detection failure) falls back to the
/// parent's original RS-Key wording.
///
/// Detection is lazy by design: a GET_INFO roundtrip happens only when a 0x30
/// has already been refused — never on the success path, and never for the
/// ops whose 0x30 meaning both firmwares share.
pub(super) fn backup_error(status: u8, op: &str, transport: &HidTransport) -> String {
    if status == 0x30
        && let Ok(info) = read_device_info(transport)
        && info.aaguid == FAPICO2_AAGUID
    {
        return vendor_error_fapico2(status, op);
    }
    vendor_error(status, op)
}

/// [`super::vendor_map`] with fapico2-aware 0x30 wording — the export call
/// site's replacement so the RS-Key `vendor_map` stays untouched.
pub(super) fn vendor_map_fapico2(
    transport: &HidTransport,
    status: u8,
    map: Option<Value>,
    op: &str,
) -> Result<BTreeMap<Value, Value>, String> {
    if status != 0 {
        return Err(backup_error(status, op, transport));
    }
    match map {
        Some(Value::Map(m)) => Ok(m),
        _ => Err(format!("{op}: response is not a CBOR map")),
    }
}

// ── FIDO config read/write (review r4505096097) ────────────────────────────

/// Read the PHY configuration over the fapico2 `0x41` CONFIG_READ dialect.
///
/// The dialect is byte-identical to RS-Key's (same sub-command, same target,
/// same TLV record stream; the response's optional effective-config key is
/// one fapico2 omits — the shared reader already treats it as optional), so
/// the parent's reader is reused unmodified.
pub(crate) fn read_config(transport: &HidTransport, config: AppConfig) -> AppConfig {
    read_rskey_physical_config(transport, config)
}

/// PHY fields fapico2's CONFIG_WRITE refuses (`UnsupportedOption`), with the
/// user-facing names used in the validation error.
const REFUSED_FIELDS: &[(&str, &str)] = &[
    ("touch_timeout", "touch timeout"),
    ("raw_curves_mask", "curve mask"),
    ("enable_secp256k1", "curve mask"),
    ("led_driver", "LED driver"),
    ("led_order", "LED order"),
    ("led_num", "LED count"),
];

/// Reject a change set the fapico2 device would refuse as a whole blob.
///
/// The device classifies the entire record stream and refuses it wholesale,
/// so a mixed save (one refused field + one supported field) would fail with
/// an opaque CTAP status and save nothing. Naming the blocking fields here
/// turns that into an actionable message.
fn validate_writable(config: &AppConfigInput) -> Result<(), PFError> {
    let mut blocked: Vec<&str> = Vec::new();
    for (field, label) in REFUSED_FIELDS {
        let set = match *field {
            "touch_timeout" => config.touch_timeout.is_some(),
            "raw_curves_mask" => config.raw_curves_mask.is_some(),
            "enable_secp256k1" => config.enable_secp256k1.is_some(),
            "led_driver" => config.led_driver.is_some(),
            "led_order" => config.led_order.is_some(),
            "led_num" => config.led_num.is_some(),
            _ => false,
        };
        if set && !blocked.contains(label) {
            blocked.push(label);
        }
    }
    // fapico2 refuses an all-zero interface mask unconditionally (it would
    // re-enumerate with no USB interfaces until BOOTSEL recovery).
    if config.enabled_usb_itf == Some(0) {
        blocked.push("an empty USB interface mask");
    }
    if blocked.is_empty() {
        return Ok(());
    }
    Err(PFError::Device(format!(
        "fapico2 does not support writing {} over FIDO. Remove {} to save the \
         remaining changes.",
        blocked.join(", "),
        if blocked.len() == 1 { "it" } else { "them" }
    )))
}

/// Write PHY config to a fapico2 via CTAPHID 0x41 CONFIG_WRITE.
///
/// Wire shape and reuse mirror the parent's `write_rskey_config` (same TLV
/// builder, same CONFIG_READ support probe, same ACFG token path); the
/// user-facing strings are fapico2's own because the post-write behaviour
/// differs (no self re-enumeration).
pub(crate) fn write_config(
    transport: &HidTransport,
    config: &AppConfigInput,
    pin: &str,
) -> Result<String, PFError> {
    validate_writable(config)?;

    let tlv = build_rskey_phy_tlv(config)?;
    if tlv.is_empty() {
        return Ok("No configuration changes were needed.".to_string());
    }

    // Support probe, identical to the RS-Key write path: CONFIG_READ is
    // ungated, so a success confirms the 0x41 CONFIG_WRITE/READ commands.
    transport
        .rs_key_config_read(RSKEY_CFG_TARGET_PHY)
        .map_err(|_| {
            PFError::Device(
                "This fapico2 firmware does not support FIDO configuration.".into(),
            )
        })?;

    let pin_token = transport
        .get_pin_token_with_permission(pin, PinUvAuthTokenPermissions::AUTHENTICATOR_CONFIG, None)
        .or_else(|e| {
            log::warn!(
                "Failed to get PIN token with ACFG permission: {}. Falling back.",
                e
            );
            transport.get_pin_token(pin)
        })?;

    transport.rs_key_config_write(&pin_token, RSKEY_CFG_TARGET_PHY, &tlv)?;

    // fapico2 persists the write immediately but does not re-enumerate; USB
    // identity fields are applied at boot, hence the unplug/re-plug ask.
    Ok("Configuration updated successfully! \
        Unplug and re-plug the device to apply changes."
        .to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── 0x30 wording (D3/D7) ────────────────────────────────────────────────

    #[test]
    fn restore_refusal_names_the_reset_requirement() {
        // D3: LOAD is reset-first — it refuses 0x30 while resident
        // credentials exist (the incoming seed may have no installed
        // counterpart to differ from, so the message must not assert one).
        let msg = vendor_error_fapico2(0x30, "restore");
        assert!(msg.contains("reset"), "message: {msg}");
        assert!(!msg.contains("already sealed"), "message: {msg}");
        assert!(!msg.contains("differs from the installed one"), "message: {msg}");
    }

    #[test]
    fn export_refusal_names_seedless_and_sealed() {
        // D7: a seedless board answers EXPORT with 0x30 — and so does a
        // sealed one (the one-time export window, permanently closed by
        // FINALIZE). Both reasons must be named, and NO remedy may be
        // prescribed: a sealed window cannot be reopened, so "install a
        // seed" would be misleading there.
        let msg = vendor_error_fapico2(0x30, "export (already sealed?)");
        assert!(msg.contains("no master seed"), "message: {msg}");
        assert!(msg.contains("sealed"), "message: {msg}");
        assert!(!msg.contains("install a seed"), "message: {msg}");
        // The host-side hedge must not leak into the user-facing text.
        assert!(!msg.contains("already sealed?"), "message: {msg}");
    }

    #[test]
    fn other_statuses_keep_the_parent_wording() {
        // Everything that is not a 0x30 keeps RS-Key's original strings.
        assert_eq!(
            vendor_error_fapico2(0x36, "restore"),
            vendor_error(0x36, "restore")
        );
        assert_eq!(
            vendor_error_fapico2(0x3D, "unlock"),
            vendor_error(0x3D, "unlock")
        );
    }

    // ── Pre-send validation ─────────────────────────────────────────────────

    fn input() -> AppConfigInput {
        serde_json::from_str("{}").expect("empty config input")
    }

    #[test]
    fn supported_fields_pass_validation() {
        let mut ok = input();
        ok.product_name = Some("My fapico2".into());
        ok.led_brightness = Some(64);
        ok.led_gpio = Some(7);
        ok.enabled_usb_itf = Some(0x07);
        assert!(validate_writable(&ok).is_ok());
    }

    #[test]
    fn refused_fields_are_named_individually() {
        let mut bad = input();
        bad.touch_timeout = Some(5);
        let msg = match validate_writable(&bad) {
            Err(PFError::Device(m)) => m,
            other => panic!("expected Device error, got {other:?}"),
        };
        assert!(msg.contains("touch timeout"), "message: {msg}");

        // A mixed save names the blocker AND says the rest won't save.
        bad.product_name = Some("Mixed save".into());
        let msg = match validate_writable(&bad) {
            Err(PFError::Device(m)) => m,
            other => panic!("expected Device error, got {other:?}"),
        };
        assert!(msg.contains("touch timeout"), "message: {msg}");
        assert!(msg.contains("remaining changes"), "message: {msg}");
    }

    #[test]
    fn curves_group_shares_one_label() {
        let mut bad = input();
        bad.enable_secp256k1 = Some(true);
        bad.raw_curves_mask = Some(0x08);
        let msg = match validate_writable(&bad) {
            Err(PFError::Device(m)) => m,
            other => panic!("expected Device error, got {other:?}"),
        };
        assert!(msg.contains("curve mask"), "message: {msg}");
        assert!(!msg.contains("curve mask, curve mask"), "message: {msg}");
    }

    #[test]
    fn zero_usb_interface_mask_is_refused() {
        let mut bad = input();
        bad.enabled_usb_itf = Some(0);
        assert!(validate_writable(&bad).is_err());
    }
}
