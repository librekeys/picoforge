//! Hardware abstraction layer — all device communication lives here.
//!
//! ```text
//! hal/
//! ├── mod.rs       — module root
//! ├── io.rs        — high-level entry points dispatching across protocols
//! ├── types.rs     — shared structs, enums, and constants
//! ├── common/      — COSE algorithm/curve enums and firmware-version parsing
//! │   ├── cose.rs
//! │   └── version.rs
//! ├── firmwares/   — per-firmware capability gating (PicoFido, RSKey)
//! │   ├── picofido.rs
//! │   └── rskey.rs
//! ├── transport/   — physical transport abstractions (HID, PC/SC)
//! │   ├── fido.rs  — CTAPHID framing over USB HID
//! │   └── pcsc.rs  — ISO 7816-4 APDU over PC/SC
//! ├── fido/        — FIDO2 / CTAP2 protocol implementation
//! │   ├── constants.rs — CTAP2 command codes, CBOR keys, vendor commands
//! │   └── ops.rs       — FidoOperations trait, PIN/credential management
//! └── rescue/      — Rescue applet protocol (PC/SC APDU)
//!     ├── constants.rs — ISO 7816-4 constants, PHY tags, vendor AIDs
//!     └── ops.rs       — RescueOperations trait
//! ```
//!
//! # Architecture
//!
//! [`types`] defines the data types shared across all submodules.
//! [`firmwares`] provides [`AnyFirmware`](crate::hal::firmwares::AnyFirmware) with per-firmware capability
//! checks (e.g. legacy vs new vendor commands).
//! [`transport`] discovers the device and returns a [`DeviceHandle`](crate::hal::transport::DeviceHandle)
//! wrapping either a FIDO HID or Rescue PC/SC connection.
//! [`fido`] and [`rescue`] implement the protocol-level operations.
//! [`io`] sits on top and exposes one function per device operation,
//! selecting the correct protocol path based on the detected firmware.

pub mod common;
pub mod fido;
pub mod firmwares;
pub mod io;
pub mod rescue;
pub mod transport;
pub mod types;
