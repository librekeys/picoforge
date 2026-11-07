//! Hardware abstraction layer for FIDO HID, Rescue APDUs, and smart-card applets.
//!
//! - [`types`] contains data shared across the HAL.
//! - [`common`] contains COSE, version, and LED helpers.
//! - [`apdu`] provides APDU, status-word, and BER-TLV types.
//! - [`transport`] implements FIDO HID, Rescue PC/SC, and persistent CCID sessions.
//! - [`applets`] implements OATH, OTP, PIV, and OpenPGP command sets over CCID.
//! - [`fido`] implements CTAP2 and PicoForge vendor operations, including audit
//!   and backup helpers.
//! - [`rescue`] implements Rescue applet operations; [`offboard`] coordinates the
//!   RS-Key full-device offboarding flow.
//! - [`firmwares`] defines per-firmware capability checks, while [`io`] exposes
//!   higher-level operations consumed by the UI.
//!
//! Device-detail reads can combine Rescue and FIDO HID results. Applet clients
//! use persistent CCID sessions so authentication state survives across commands
//! that require it.

pub mod apdu;
pub mod applets;
pub mod common;
pub mod fido;
pub mod firmwares;
pub mod io;
pub mod offboard;
pub mod rescue;
pub mod transport;
pub mod types;
