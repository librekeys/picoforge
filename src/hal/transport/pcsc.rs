//! PC/SC (Smart Card) transport for the rescue channel.
//!
//! Communicates with the device via ISO 7816-4 APDUs over a PC/SC
//! compatible smart-card reader. The device exposes a rescue applet
//! identified by [`RESCUE_AID`] when in rescue/bootloader mode.

use crate::error::PFError;
use crate::hal::{rescue::constants::*, types::FirmwareType};
use pcsc::{Context, Protocols, Scope, ShareMode};
use std::{
    ffi::CString,
    sync::{OnceLock, RwLock},
};

const RESCUE_PRODUCT_FIDO: u8 = 2;
const RESCUE_IDENTITY_LEN: usize = 12;
const RESCUE_MCU_CODES: [u8; 5] = [0, 1, 2, 3, 4];

static ACTIVE_DEVICE_SERIAL: OnceLock<RwLock<Option<String>>> = OnceLock::new();

#[derive(Debug, Clone)]
pub(crate) struct PcscDeviceIdentity {
    pub serial: String,
    pub firmware_type: FirmwareType,
}

pub(crate) fn set_active_device_serial(serial: Option<String>) {
    let serial = serial.filter(|value| !value.is_empty() && value != "Unknown");
    *ACTIVE_DEVICE_SERIAL
        .get_or_init(|| RwLock::new(None))
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = serial;
}

pub(crate) fn active_device_serial() -> Option<String> {
    ACTIVE_DEVICE_SERIAL
        .get_or_init(|| RwLock::new(None))
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone()
}

/// Probe the firmware-specific Rescue identity response on an already-open
/// card. Ordinary PC/SC readers and cards do not implement this AID and payload.
pub(crate) fn probe_device_identity(
    card: &pcsc::Card,
) -> Result<Option<PcscDeviceIdentity>, String> {
    let response = select_aid(card, RESCUE_AID)?;
    if response.len() != RESCUE_IDENTITY_LEN + 2 || !response.ends_with(&SW_SUCCESS) {
        return Ok(None);
    }
    let data = &response[..RESCUE_IDENTITY_LEN];
    if !RESCUE_MCU_CODES.contains(&data[0])
        || data[1] != RESCUE_PRODUCT_FIDO
        || data[2] == 0
        || data[4..12].iter().all(|byte| *byte == 0)
    {
        return Ok(None);
    }

    let serial = u32::from_be_bytes([data[4] & 0x03, data[5], data[6], data[7]]).to_string();
    // The Yubico-compatible Management AID exists on RS-Key, while PicoFido
    // exposes its management response over CTAPHID instead.
    //
    // `data[2]` is a protocol-capability statement ("RS-Key SDK >= 8"), not an
    // identity claim: fapico2 ships an RS-Key-SDK-compatible rescue applet
    // (`RSKEY_SDK_MAJOR = 8`, pinned by its tests/protocol.rs) and answers it
    // too, but it is not RS-Key and exposes no Management AID. PC/SC alone
    // therefore cannot name fapico2; `io::read_device_details` reads the FIDO
    // AAGUID first and keeps it authoritative when both interfaces are paired.
    let firmware_type = if select_aid(card, MANAGEMENT_AID)?.ends_with(&SW_SUCCESS) {
        FirmwareType::RSKey
    } else {
        FirmwareType::PicoFido
    };
    Ok(Some(PcscDeviceIdentity {
        serial,
        firmware_type,
    }))
}

fn select_aid(card: &pcsc::Card, aid: &[u8]) -> Result<Vec<u8>, String> {
    let mut apdu = vec![
        APDU_CLA_ISO,
        APDU_INS_SELECT,
        APDU_P1_SELECT_BY_DF_NAME,
        APDU_P2_RETURN_FCI,
        aid.len() as u8,
    ];
    apdu.extend_from_slice(aid);
    let mut response = [0u8; 256];
    card.transmit(&apdu, &mut response)
        .map(|response| response.to_vec())
        .map_err(|error| error.to_string())
}

/// Return all readers, with PicoForge-compatible reader names tried first.
/// The name only affects ordering: firmware is validated through its Rescue
/// AID response before a reader is accepted.
pub(crate) fn ordered_readers(ctx: &Context) -> Result<Vec<CString>, PFError> {
    let mut readers = ctx.list_readers_owned()?;
    readers.sort_by_key(|reader| {
        let name = reader.to_string_lossy().to_ascii_lowercase();
        let rank = if name.contains("rs-key") || name.contains("rskey") || name.contains("rsk") {
            0
        } else if name.contains("pico-fido")
            || name.contains("picofido")
            || name.contains("librekeys")
        {
            1
        } else {
            2
        };
        (rank, name)
    });
    Ok(readers)
}

/// PC/SC transport wrapping a connected ISO 7816-4 smart card.
pub struct PcscTransport {
    /// The connected PC/SC card handle.
    pub card: pcsc::Card,
    /// Firmware type determined during the SELECT AID exchange.
    pub firmware_type: FirmwareType,
    /// Raw response bytes from the SELECT AID command.
    pub select_resp: Vec<u8>,
}

impl PcscTransport {
    /// Open the rescue channel using the default Rescue AID.
    pub fn open() -> Result<Self, PFError> {
        Self::open_with_aid(RESCUE_AID)
    }

    /// Open the rescue channel using a custom AID.
    ///
    /// Scans all readers, validates the firmware-specific Rescue identity, then
    /// selects the requested applet on the matching PicoForge key.
    pub fn open_with_aid(aid: &[u8]) -> Result<Self, PFError> {
        let ctx = Context::establish(Scope::User).map_err(|e| {
            log::error!("Failed to establish PCSC context: {}", e);
            PFError::Pcsc(e)
        })?;

        let readers = ordered_readers(&ctx)?;
        if readers.is_empty() {
            log::info!("No Smart Card Reader found");
            return Err(PFError::NoDevice);
        }
        let expected_serial = active_device_serial();
        let mut last_error = None;
        for reader in readers {
            let reader_name = reader.to_string_lossy();
            let card = match ctx.connect(reader.as_c_str(), ShareMode::Shared, Protocols::ANY) {
                Ok(card) => card,
                Err(error) => {
                    last_error = Some(error.to_string());
                    continue;
                }
            };
            let identity = match probe_device_identity(&card) {
                Ok(Some(identity)) => identity,
                Ok(None) => continue,
                Err(error) => {
                    log::debug!("Could not identify PC/SC reader {reader_name}: {error}");
                    last_error = Some(error);
                    continue;
                }
            };
            if expected_serial
                .as_deref()
                .is_some_and(|serial| serial != identity.serial)
            {
                continue;
            }

            let rx = match select_aid(&card, aid) {
                Ok(response) if response.ends_with(&SW_SUCCESS) => response,
                Ok(_) => continue,
                Err(error) => {
                    last_error = Some(error);
                    continue;
                }
            };
            log::info!(
                "Selected {:?} on PC/SC reader {reader_name}",
                identity.firmware_type
            );
            return Ok(Self {
                card,
                firmware_type: identity.firmware_type,
                select_resp: rx,
            });
        }

        let detail = last_error
            .map(|error| format!(" Last reader error: {error}."))
            .unwrap_or_default();
        Err(PFError::Device(format!(
            "No PicoFido or RS-Key reader exposes the requested applet.{detail}"
        )))
    }

    pub fn transmit<'a>(&self, apdu: &[u8], rx_buf: &'a mut [u8]) -> Result<&'a [u8], PFError> {
        self.card.transmit(apdu, rx_buf).map_err(PFError::Pcsc)
    }
}
