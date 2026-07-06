#![allow(dead_code)]
use std::fmt;

use crate::error::PFError;
use crate::hal::fido::hid::HidTransport;
use crate::hal::types::FirmwareType;

pub enum DeviceHandle {
    Fido(HidTransport),
    Rescue(pcsc::Card, FirmwareType),
}

impl fmt::Debug for DeviceHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Fido(t) => f.debug_tuple("Fido").field(t).finish(),
            Self::Rescue(_, ft) => f.debug_tuple("Rescue").field(ft).finish(),
        }
    }
}

#[derive(Debug)]
pub struct DeviceIdentity {
    pub vid: u16,
    pub pid: u16,
    pub product_name: String,
    pub firmware_type: FirmwareType,
}

impl DeviceHandle {
    pub fn discover() -> Result<(Self, DeviceIdentity), PFError> {
        match Self::try_fido() {
            Ok(Some((handle, identity))) => {
                log::info!("Device discovered via FIDO HID transport");
                return Ok((handle, identity));
            }
            Ok(None) => log::info!("No FIDO HID device found"),
            Err(e) => log::warn!("FIDO HID discovery error: {}", e),
        }

        match Self::try_rescue() {
            Ok(Some((handle, identity))) => {
                log::info!("Device discovered via Rescue PC/SC transport");
                return Ok((handle, identity));
            }
            Ok(None) => log::info!("No Rescue PC/SC device found"),
            Err(e) => log::warn!("Rescue PC/SC discovery error: {}", e),
        }

        Err(PFError::NoDevice)
    }

    fn try_fido() -> Result<Option<(Self, DeviceIdentity)>, PFError> {
        let transport = HidTransport::open()?;
        let identity = DeviceIdentity {
            vid: transport.vid,
            pid: transport.pid,
            product_name: transport.product_name.clone(),
            firmware_type: FirmwareType::Unknown,
        };
        Ok(Some((Self::Fido(transport), identity)))
    }

    fn try_rescue() -> Result<Option<(Self, DeviceIdentity)>, PFError> {
        let ctx = pcsc::Context::establish(pcsc::Scope::User).map_err(PFError::Pcsc)?;
        let mut readers_buf = [0; 2048];
        let mut readers = ctx.list_readers(&mut readers_buf).map_err(PFError::Pcsc)?;
        let reader = match readers.next() {
            Some(r) => r,
            None => return Ok(None),
        };
        let reader_name = reader.to_string_lossy();
        let fw_type = if reader_name.contains("RS-Key") || reader_name.contains("RSK") {
            FirmwareType::RSKey
        } else {
            FirmwareType::Unknown
        };
        let card = ctx
            .connect(reader, pcsc::ShareMode::Shared, pcsc::Protocols::ANY)
            .map_err(PFError::Pcsc)?;
        let identity = DeviceIdentity {
            vid: 0,
            pid: 0,
            product_name: reader_name.to_string(),
            firmware_type: fw_type.clone(),
        };
        Ok(Some((Self::Rescue(card, fw_type), identity)))
    }
}
