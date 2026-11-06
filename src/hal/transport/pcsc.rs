//! PC/SC (Smart Card) transport for the rescue channel.
//!
//! Communicates with the device via ISO 7816-4 APDUs over a PC/SC
//! compatible smart-card reader. The device exposes a rescue applet
//! identified by [`RESCUE_AID`] when in rescue/bootloader mode.

use crate::error::PFError;
use crate::hal::{rescue::constants::*, types::FirmwareType};
use pcsc::{Context, Protocols, Scope, ShareMode};

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
    /// Scans for the first connected reader, sends the SELECT AID APDU,
    /// and determines the firmware type from the reader name or response data.
    pub fn open_with_aid(aid: &[u8]) -> Result<Self, PFError> {
        let ctx = Context::establish(Scope::User).map_err(|e| {
            log::error!("Failed to establish PCSC context: {}", e);
            PFError::Pcsc(e)
        })?;

        let mut readers_buf = [0; 2048];
        let mut readers = ctx.list_readers(&mut readers_buf)?;

        let reader = readers.next().ok_or_else(|| {
            log::info!("No Smart Card Reader found");
            PFError::NoDevice
        })?;

        let reader_name = reader.to_string_lossy();
        let mut fw_type = if reader_name.contains("RS-Key") || reader_name.contains("RSK") {
            FirmwareType::RSKey
        } else {
            FirmwareType::Unknown
        };

        let card = ctx.connect(reader, ShareMode::Shared, Protocols::ANY)?;

        let mut apdu = vec![
            APDU_CLA_ISO,
            APDU_INS_SELECT,
            APDU_P1_SELECT_BY_DF_NAME,
            APDU_P2_RETURN_FCI,
            aid.len() as u8,
        ];
        apdu.extend_from_slice(aid);

        let mut rx_buf = [0; 256];
        let rx = card.transmit(&apdu, &mut rx_buf)?;

        if !rx.ends_with(&[0x90, 0x00]) {
            log::error!("Rescue Applet not found on the device!");
            return Err(PFError::Device(
                "Rescue Applet not found on device. Is it in FIDO mode?".into(),
            ));
        }

        let data = rx.to_vec();

        if fw_type == FirmwareType::Unknown {
            if data.len() >= 4 && data[2] >= 8 {
                fw_type = FirmwareType::RSKey;
            } else {
                fw_type = FirmwareType::PicoFido;
            }
        }

        log::info!("Successfully connected to Rescue Applet");
        log::info!("Detected firmware type: {:?}", fw_type);

        Ok(Self {
            card,
            firmware_type: fw_type,
            select_resp: data,
        })
    }

    pub fn transmit<'a>(&self, apdu: &[u8], rx_buf: &'a mut [u8]) -> Result<&'a [u8], PFError> {
        self.card.transmit(apdu, rx_buf).map_err(PFError::Pcsc)
    }
}
