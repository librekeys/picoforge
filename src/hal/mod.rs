//! Device communication layer for pico-forge.
//!
//! ```text
//! device/
//! ├── mod.rs      — module root, re-exports submodules
//! ├── io.rs       — high-level entry points (both protocols)
//! ├── rescue.rs   — rescue / PC/SC protocol implementation
//! ├── fido.rs     — FIDO2 / CTAP2 protocol implementation
//! └── types.rs    — shared structs, enums, and constants
//! ```
//!
//! # Overview
//!
//! The `device` module is the only place that talks to the hardware token.
//! Everything above it (UI state, gpui-component views) depends on the
//! public functions exported here; nothing below it should know about the
//! communication details.
//!
//! Two protocols are used:
//!
//! - **Rescue (PC/SC)** — low-level APDU channel for firmware-level
//!   configuration: secure boot, LED status, USB applet management, and
//!   device reboot.  Implemented in [`rescue`].
//!
//! - **FIDO2 (CTAP2)** — standard authenticator protocol for credential
//!   management, PIN operations, and enterprise attestation.
//!   Implemented in [`fido`].
//!
//! [`io`] sits on top of both and exposes a single function per device
//! operation.  Some functions dispatch to one protocol or the other based
//! on a [`types::DeviceMethod`] flag; others try rescue first and fall back
//! to FIDO on failure.
//!
//! # Adding a new device operation
//!
//! 1. Add any new structs/enums to [`types`].
//! 2. Implement the raw protocol call in [`rescue`] or [`fido`].
//! 3. Expose a high-level wrapper in [`io`] that picks the right protocol
//!    and converts errors to the caller's expected type.
//! 4. Wire the wrapper into a gpui-component view or action handler.

pub mod fido;
pub mod io;
pub mod rescue;
pub mod types;
