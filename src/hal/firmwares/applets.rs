//! Per-firmware applet feature profiles.
//!
//! Extends the `firmwares/` capability seam to the CCID applets. A firmware
//! answers "does my applet support feature X?" through [`AppletProfile`], and
//! the applet screens read the answer to show or disable controls. Adding a new
//! firmware means one `impl AppletProfile` here — the shared wire ops in
//! [`crate::hal::applets`], the transport, and the UI stay untouched.

use crate::hal::applets::{OathFeatures, OpenPgpFeatures, OtpFeatures, PivFeatures};
use crate::hal::firmwares::{AnyFirmware, Fapico2Firmware, PicoFidoFirmware, RSKeyFirmware};

/// Feature answers per applet. `None` means the firmware exposes no such applet
/// (or it is not yet characterised) → the screen renders an honest empty state
/// rather than offering controls the wire can't back.
pub trait AppletProfile {
    fn oath(&self) -> Option<OathFeatures> {
        None
    }
    fn otp(&self) -> Option<OtpFeatures> {
        None
    }
    fn piv(&self) -> Option<PivFeatures> {
        None
    }
    fn openpgp(&self) -> Option<OpenPgpFeatures> {
        None
    }
}

impl AppletProfile for RSKeyFirmware {
    fn oath(&self) -> Option<OathFeatures> {
        // From the RS-Key rsk-oath wire audit: full Yubico-Authenticator parity.
        Some(OathFeatures {
            rename: true,
            touch: true,
            sha512: true,
            password: true,
        })
    }

    fn otp(&self) -> Option<OtpFeatures> {
        // rsk-otp implements all four slot types over four slots (3/4 = extension).
        Some(OtpFeatures {
            slots: 4,
            chalresp: true,
            hotp: true,
            static_pw: true,
            yubiotp: true,
            swap: true,
        })
    }

    fn piv(&self) -> Option<PivFeatures> {
        Some(PivFeatures {
            generate: true,
            import_cert: true,
            attestation: true,
            retired_slots: true,
        })
    }

    fn openpgp(&self) -> Option<OpenPgpFeatures> {
        // rsk-openpgp implements OpenPGP Card 3.4 with RSA + the full EC set
        // (Weierstrass / Ed25519 / X25519), per-key touch, and reset code.
        Some(OpenPgpFeatures {
            generate: true,
            touch: true,
            reset_code: true,
            ecc: true,
        })
    }
}

impl AppletProfile for PicoFidoFirmware {
    fn oath(&self) -> Option<OathFeatures> {
        Some(OathFeatures {
            rename: true,
            touch: true,
            sha512: true,
            password: true,
        })
    }

    fn otp(&self) -> Option<OtpFeatures> {
        Some(OtpFeatures {
            slots: 4,
            chalresp: true,
            hotp: true,
            static_pw: true,
            yubiotp: true,
            swap: true,
        })
    }
}

impl AppletProfile for Fapico2Firmware {
    fn oath(&self) -> Option<OathFeatures> {
        Some(OathFeatures {
            // fapico2 apps/oath/src/oath.rs:59 implements `INS_RENAME (0x05)`.
            rename: true,
            // fapico2's only OATH presence tag is `PRESENCE_TAG_RESET` for
            // RESET (apps/oath/src/oath.rs:69-73, US-921/US-903) — there is no
            // per-credential touch property, unlike RS-Key/pico-fido.
            touch: false,
            // fapico2 apps/oath/src/oath.rs:52 implements `ALG_SHA512 (0x03)`.
            sha512: true,
            // fapico2 apps/oath/src/oath.rs:41-42,208 implement the
            // access-code (TAG_PASSWORD/TAG_NEW_PASSWORD) validation latch.
            password: true,
        })
    }

    fn otp(&self) -> Option<OtpFeatures> {
        Some(OtpFeatures {
            // fapico2 apps/oath/src/otp.rs:27-28 configures exactly two slots
            // (`SLOT_CONFIGURE 0x01`, `SLOT_CONFIGURE_SLOT2 0x03`) — no 3/4
            // extension like RS-Key's four-slot build.
            slots: 2,
            // fapico2 apps/oath/src/otp.rs:40-43 answers HMAC-SHA1 and AES
            // challenge-response (`CALC_HMAC_SLOT1/2`, `CALC_AES_SLOT1/2`).
            chalresp: true,
            // fapico2 apps/oath/src/otp.rs:2143,2156 expose `OathHotp` as a
            // client slot type.
            hotp: true,
            // fapico2 apps/oath/src/otp.rs:113-127 implement static-password
            // slot programming ("the reference client's `build_static`").
            static_pw: true,
            // fapico2 apps/oath/src/otp.rs:138-139 carry the Yubico OTP
            // config fields (CFG_AES_KEY, CFG_UID).
            yubiotp: true,
            // fapico2 apps/oath/src/otp.rs:31 implements `SLOT_SWAP (0x06)`.
            swap: true,
        })
    }

    fn piv(&self) -> Option<PivFeatures> {
        // fapico2 ships a PIV applet, but PicoForge's PIV screen is
        // generate/import centric and the applet implements neither: no
        // GENERATE INS exists in apps/piv/src/lib.rs:42-53, cert IMPORT is
        // `INS 0xFE` — "an INS this applet does not implement"
        // (apps/piv/src/lib.rs:358-360), and "admin/attestation function
        // files have no equivalent" (apps/piv/src/lib.rs:838). Returning
        // `Some` would render Generate/Import buttons the wire refuses, so
        // the honest answer is the empty state (same as pico-fido).
        None
    }

    fn openpgp(&self) -> Option<OpenPgpFeatures> {
        // fapico2 vendors the `opcard` OpenPGP card 3.4 implementation
        // unmodified apart from review patches (apps/openpgp/src/lib.rs:1-20,
        // US-331+; US-912 factory-default gate, US-914 presence seam), which
        // supports on-card key generation, per-key touch (the
        // PRESENCE_TAG_PSO_SIGN/PSO_DECIPHER/INT_AUTH tags), the resetting
        // code, and the RSA + full EC algorithm set.
        Some(OpenPgpFeatures {
            generate: true,
            touch: true,
            reset_code: true,
            ecc: true,
        })
    }
}

impl AnyFirmware {
    /// OATH feature profile for the inner firmware.
    pub fn oath_features(&self) -> Option<OathFeatures> {
        match self {
            Self::PicoFido(fw) => fw.oath(),
            Self::RSKey(fw) => fw.oath(),
            Self::Fapico2(fw) => fw.oath(),
        }
    }

    /// OTP feature profile for the inner firmware.
    pub fn otp_features(&self) -> Option<OtpFeatures> {
        match self {
            Self::PicoFido(fw) => fw.otp(),
            Self::RSKey(fw) => fw.otp(),
            Self::Fapico2(fw) => fw.otp(),
        }
    }

    /// PIV feature profile for the inner firmware.
    pub fn piv_features(&self) -> Option<PivFeatures> {
        match self {
            Self::PicoFido(fw) => fw.piv(),
            Self::RSKey(fw) => fw.piv(),
            Self::Fapico2(fw) => fw.piv(),
        }
    }

    /// OpenPGP feature profile for the inner firmware.
    pub fn openpgp_features(&self) -> Option<OpenPgpFeatures> {
        match self {
            Self::PicoFido(fw) => fw.openpgp(),
            Self::RSKey(fw) => fw.openpgp(),
            Self::Fapico2(fw) => fw.openpgp(),
        }
    }
}
