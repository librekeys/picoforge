//! Fapico2 firmware implementation (eddieoz/fapico2 — pico-fido lineage with its own AAGUID).
//!
//! The [`Fapico2Firmware`] struct stores a parsed version; capability gating is
//! static, like [`crate::hal::firmwares::RSKeyFirmware`], because fapico2 speaks
//! the RS-Key `0x41` vendor dialect rather than pico-fido's versioned
//! vendor-prototype paths.
//! Each answer cites the firmware source that backs it (sibling checkout
//! `pico/fapico2`):
//!
//! - `supports_rs_key_vendor_command` — **true, and this is the whole point of
//!   the profile**: fapico2 answers the RS-Key `0x41` CTAPHID vendor channel.
//!   `apps/fido/src/vendor41.rs` (fapico2) implements every sub-command of that
//!   dialect — its module header calls it "the RS-Key `0x41` vendor channel
//!   (PicoForge vendor framing **(C)) ... every one of its fourteen
//!   sub-commands", and `Subcommand::Mse` (`vendor41.rs:561`) is the same
//!   `RSKEY_VENDOR_MSE (1)` PicoForge probes on the RS-Key profile.
//! - `supports_fido_config_write` — true: `CONFIG_WRITE` (`0x0C`) and
//!   `CONFIG_READ` (`0x0D`) are live on that channel (`vendor41.rs` header:
//!   "`CONFIG_READ` (`0x0D`) ... and the written decision for every one of its
//!   fourteen sub-commands"; US-1751's probe run answered CONFIG_READ on-board).
//! - `supports_rescue_channel` — true: fapico2 ships a rescue applet whose
//!   SELECT answers `data[2] = RSKEY_SDK_MAJOR = 8`
//!   (`apps/rescue/src/lib.rs`, pinned by `tests/protocol.rs`). That byte is a
//!   capability statement ("RS-Key SDK ≥ 8"), not an identity claim — see the
//!   merge-authority note in [`crate::hal::io::read_device_details`] and
//!   [`crate::hal::transport::pcsc`]: PC/SC cannot name fapico2 alone, and the
//!   FIDO AAGUID stays authoritative when both interfaces are paired.
//! - `supports_legacy_fido_hardware_config` — **false**: fapico2 has no legacy
//!   pico-fido `vendorPrototype` 0xFF hardware-config path. US-1751's probe on
//!   a stock board answered the `0x41` CONFIG_READ, never the legacy CONFIG_PHY_*
//!   framing; and the legacy probe (`probe_legacy_vendor_support`) is a
//!   pico-fido-only thing fapico2 does not implement.
//!
//! ## Divergences — do not re-derive (epic PF-FAPICO2)
//!
//! - **D1 — MSE is P-256-only.** RS-Key's MSE handshake can optionally carry an
//!   ML-KEM-768 post-quantum half (`{2: mlkem_ek}`); fapico2 never emits one.
//!   Its `0x41` dialect and seed-backup crypto are P-256 end to end
//!   (`apps/fido/src/vendor41.rs`: `Subcommand::Mse` at 561, and the backup/attest
//!   key material is all "P-256" — lines 3684-3914). PicoForge's `mse_handshake`
//!   (`fido/mod.rs:1684-1734`) only ever sends P-256, so **no change is needed**
//!   — this note exists so nobody "fixes" the profile into an ML-KEM half and
//!   breaks compatibility.

use crate::hal::common::FirmwareVersion;
use crate::hal::firmwares::FirmwareTrait;
use crate::hal::types::FirmwareType;

#[derive(Debug, Clone)]
pub struct Fapico2Firmware {
    version: FirmwareVersion,
}

impl Fapico2Firmware {
    pub fn new(version: FirmwareVersion) -> Self {
        Self { version }
    }
}

impl FirmwareTrait for Fapico2Firmware {
    fn firmware_type(&self) -> FirmwareType {
        FirmwareType::Fapico2
    }

    fn version(&self) -> &FirmwareVersion {
        &self.version
    }

    /// No legacy pico-fido `vendorPrototype` 0xFF hardware-config path: fapico2
    /// answers config reads over the `0x41` vendor channel instead (US-1751
    /// probe evidence; the legacy CONFIG_PHY_* framing is pico-fido-only).
    fn supports_legacy_fido_hardware_config(&self) -> bool {
        false
    }

    /// FIDO config writes go over CTAPHID `0x41` CONFIG_WRITE, which fapico2
    /// implements (`apps/fido/src/vendor41.rs`). The CTAP firmware version from
    /// GET_INFO is a release version (e.g. 7.6), not an SDK level, so support
    /// is static rather than version-gated — same shape as the RS-Key answer.
    fn supports_fido_config_write(&self) -> bool {
        true
    }

    /// The whole point of this profile: fapico2 answers the RS-Key `0x41`
    /// vendor channel (`apps/fido/src/vendor41.rs`, all fourteen sub-commands).
    fn supports_rs_key_vendor_command(&self) -> bool {
        true
    }

    /// The rescue (PC/SC) applet is shipped and its SELECT answers with the
    /// RS-Key SDK capability byte `data[2] = 8` (`apps/rescue/src/lib.rs`,
    /// `RSKEY_SDK_MAJOR`).
    fn supports_rescue_channel(&self) -> bool {
        true
    }
}
