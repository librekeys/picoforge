//! Firmware-agnostic CCID applet clients (Yubico / ISO wire protocol).
//!
//! Each submodule speaks one applet's command set over a
//! [`CcidSession`](crate::hal::transport::ccid::CcidSession). The wire protocol
//! is identical across firmwares that emulate the Yubico applets, so only
//! *feature gating* differs — and that lives in
//! [`crate::hal::firmwares::applets`], keyed off the descriptors below.
//!
//! Adding a new applet screen means: a submodule here (the ops), a `*Features`
//! descriptor, a method on `AppletProfile`, and per-firmware answers — the UI
//! and transport layers do not change.

pub mod oath;
pub mod openpgp;
pub mod otp;
pub mod piv;

/// OATH applet features a firmware exposes, read by the Accounts screen to
/// show or disable controls. Absence of a whole applet is expressed one level
/// up (`AppletProfile::oath` returning `None`), not here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OathFeatures {
    /// RENAME (`0x05`) — YubiKey 5.3+ / RS-Key.
    pub rename: bool,
    /// Require-touch property on credentials.
    pub touch: bool,
    /// SHA-512 credentials (accepted on the wire; some hosts hide it).
    pub sha512: bool,
    /// Access-code (password) protection.
    pub password: bool,
}

/// PIV applet features a firmware exposes, read by the PIV screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PivFeatures {
    /// On-device key generation.
    pub generate: bool,
    /// Certificate import (PUT DATA).
    pub import_cert: bool,
    /// Attestation of generated keys.
    pub attestation: bool,
    /// The 20 retired key slots (82..95).
    pub retired_slots: bool,
}

/// OpenPGP applet features a firmware exposes, read by the OpenPGP screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenPgpFeatures {
    /// On-device key generation.
    pub generate: bool,
    /// Per-key touch (UIF) policy.
    pub touch: bool,
    /// Resetting-code — unblock PW1 without the admin PIN.
    pub reset_code: bool,
    /// Elliptic-curve keys (ECDSA / EdDSA / ECDH) beyond RSA.
    pub ecc: bool,
}

/// OTP applet features a firmware exposes, read by the Slots screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OtpFeatures {
    /// Number of programmable slots exposed by this firmware's profile.
    pub slots: u8,
    /// HMAC-SHA1 challenge-response programming.
    pub chalresp: bool,
    /// OATH-HOTP programming.
    pub hotp: bool,
    /// Static-password programming.
    pub static_pw: bool,
    /// Yubico-OTP programming.
    pub yubiotp: bool,
    /// Swap slots 1↔2.
    pub swap: bool,
}
