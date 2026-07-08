//! Device discovery and transport abstraction.
//!
//! Two physical transports coexist:
//!
//! * **FIDO HID** ([`fido::HidTransport`]) — the primary CTAP2 / CTAPHID channel
//!   over USB HID. Used for normal operations (credential management, PIN,
//!   authentication). Supports both pico-fido and RS-Key firmwares.
//! * **Rescue PC/SC** ([`pcsc::PcscTransport`]) — an ISO 7816-4 APDU channel over
//!   a PC/SC smart-card reader. Used when the device is in rescue/bootloader mode
//!   or when FIDO commands are blocked (e.g. firmware version ≥ 7.4 on pico-fido).
//!
//! The [`DeviceHandle::discover`] method tries PC/SC first and falls back to
//! FIDO HID. The PC/SC rescue channel provides richer device details (serial,
//! flash stats, secure boot) and does not require PIN authentication for
//! configuration writes.

use std::fmt;

use crate::error::PFError;
use crate::hal::types::FirmwareType;

pub mod fido;
use fido::HidTransport;

pub mod pcsc;
use pcsc::PcscTransport;

/// A connected device handle over either the FIDO or rescue transport.
pub enum DeviceHandle {
    /// Connected via CTAPHID (USB HID).
    Fido(HidTransport),
    /// Connected via PC/SC (ISO 7816-4 APDU, rescue/bootloader mode).
    Rescue(PcscTransport),
}

impl fmt::Debug for DeviceHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Fido(t) => f.debug_tuple("Fido").field(t).finish(),
            Self::Rescue(t) => f.debug_tuple("Rescue").field(&t.firmware_type).finish(),
        }
    }
}

/// Opaque device identity presented to consumers after discovery.
///
/// vid/pid are populated only for the FIDO HID path; the rescue path
/// reports (0, 0) since PC/SC does not expose USB identifiers.
#[derive(Debug)]
#[allow(dead_code)]
pub struct DeviceIdentity {
    /// USB Vendor ID (0 for rescue/PC/SC).
    pub vid: u16,
    /// USB Product ID (0 for rescue/PC/SC).
    pub pid: u16,
    /// Human-readable product name.
    pub product_name: String,
    /// Detected firmware type (unknown for FIDO HID until GetInfo is called).
    pub firmware_type: FirmwareType,
}

impl DeviceHandle {
    /// Return the firmware type for a rescue handle, or `Unknown` for FIDO.
    pub fn firmware_type(&self) -> FirmwareType {
        match self {
            Self::Fido(_) => FirmwareType::Unknown,
            Self::Rescue(t) => t.firmware_type.clone(),
        }
    }

    /// Extract the inner FIDO transport, consuming the handle.
    #[allow(dead_code)]
    pub fn into_fido(self) -> Option<HidTransport> {
        match self {
            Self::Fido(t) => Some(t),
            _ => None,
        }
    }

    /// Try to discover a device via Rescue PC/SC first, falling back to FIDO HID.
    #[allow(dead_code)]
    pub fn discover() -> Result<(Self, DeviceIdentity), PFError> {
        match Self::try_rescue() {
            Ok(Some((handle, identity))) => {
                log::info!("Device discovered via Rescue PC/SC transport");
                return Ok((handle, identity));
            }
            Ok(None) => log::info!("No Rescue PC/SC device found"),
            Err(e) => log::warn!("Rescue PC/SC discovery error: {}", e),
        }

        match Self::try_fido() {
            Ok(Some((handle, identity))) => {
                log::info!("Device discovered via FIDO HID transport");
                return Ok((handle, identity));
            }
            Ok(None) => log::info!("No FIDO HID device found"),
            Err(e) => log::warn!("FIDO HID discovery error: {}", e),
        }

        Err(PFError::NoDevice)
    }

    /// Try to connect via FIDO HID transport.
    pub fn try_fido() -> Result<Option<(Self, DeviceIdentity)>, PFError> {
        let transport = HidTransport::open()?;
        let identity = DeviceIdentity {
            vid: transport.vid,
            pid: transport.pid,
            product_name: transport.product_name.clone(),
            firmware_type: FirmwareType::Unknown,
        };
        Ok(Some((Self::Fido(transport), identity)))
    }

    /// Try to connect via Rescue PC/SC transport.
    pub fn try_rescue() -> Result<Option<(Self, DeviceIdentity)>, PFError> {
        match PcscTransport::open() {
            Ok(transport) => {
                let identity = DeviceIdentity {
                    vid: 0,
                    pid: 0,
                    product_name: "Rescue Device".into(),
                    firmware_type: transport.firmware_type.clone(),
                };
                Ok(Some((Self::Rescue(transport), identity)))
            }
            Err(PFError::NoDevice) => Ok(None),
            Err(e) => Err(e),
        }
    }
}
