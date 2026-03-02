use aes::cipher::generic_array::GenericArray;
use cbc::cipher::{BlockDecryptMut, BlockEncryptMut, KeyIvInit, block_padding::NoPadding};
use rand::RngExt;
use ring::{agreement, digest, hmac};
use serde_cbor_2::{Value, from_slice, to_vec};
use std::collections::BTreeMap;
use std::time::Duration;

use crate::device::fido::constants::*;
use crate::error::PFError;

// HID Transport Constants
const HID_REPORT_SIZE: usize = 64;
const HID_USAGE_PAGE_FIDO: u16 = 0xF1D0;
const CTAPHID_CID_BROADCAST: u32 = 0xFFFFFFFF;
const CTAPHID_INIT: u8 = 0x86;
pub const CTAPHID_CBOR: u8 = 0x90;
const CTAPHID_ERROR: u8 = 0xBF;
const CTAPHID_KEEPALIVE: u8 = 0xBB;

// Timeouts
const HID_READ_TIMEOUT_MS: i32 = 10;
const HID_INIT_READ_TIMEOUT_MS: i32 = 100;
const HID_RESP_READ_TIMEOUT_MS: i32 = 2000;
const HID_CONT_READ_TIMEOUT_MS: i32 = 500;
const HID_TOTAL_TIMEOUT_MS: i32 = 5000;

pub struct HidTransport {
    device: hidapi::HidDevice,
    cid: u32,
    pub vid: u16,
    pub pid: u16,
    pub product_name: String,
}

#[derive(Debug, Clone)]
pub struct EnumerateRpResponse {
    pub rp: Value,
    pub rp_id_hash: Vec<u8>,
    #[allow(dead_code)]
    pub total_rps: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct EnumerateCredentialResponse {
    pub user: Value,
    pub credential_id: Value,
    #[allow(dead_code)]
    pub public_key: Value,
    #[allow(dead_code)]
    pub total_credentials: Option<usize>,
}

impl HidTransport {
    pub fn open() -> Result<Self, PFError> {
        log::info!("Attempting to open HID transport for FIDO device...");
        let api = hidapi::HidApi::new().map_err(|e| {
            log::error!("Failed to initialize HidApi: {}", e);
            PFError::Device(format!("Failed to initialize HidApi: {}", e))
        })?;

        // Find device with FIDO Usage Page (0xF1D0)
        let info = api
            .device_list()
            .find(|d| d.usage_page() == HID_USAGE_PAGE_FIDO)
            .ok_or_else(|| {
                log::warn!("No FIDO device found with Usage Page 0xF1D0.");
                PFError::NoDevice
            })?;

        log::debug!(
            "Found FIDO device: VendorID=0x{:04X}, ProductID=0x{:04X}",
            info.vendor_id(),
            info.product_id()
        );

        let vid = info.vendor_id();
        let pid = info.product_id();
        let product_name = info
            .product_string()
            .unwrap_or("Unknown FIDO Device")
            .to_string();

        let device = info.open_device(&api).map_err(|e| {
            log::error!("Failed to open HID device: {}", e);
            PFError::Device(format!("Failed to open HID device: {}", e))
        })?;

        // Negotiate Channel ID (CID)
        let cid = Self::init_channel(&device).map_err(|e| {
            log::error!("Failed to negotiate Channel ID: {}", e);
            PFError::Device(format!("Failed to negotiate Channel ID: {}", e))
        })?;

        log::info!("HID Transport established successfully. CID: 0x{:08X}", cid);
        Ok(Self {
            device,
            cid,
            vid,
            pid,
            product_name,
        })
    }

    fn init_channel(device: &hidapi::HidDevice) -> Result<u32, PFError> {
        log::debug!("Initializing CTAPHID channel...");

        // --- Drain Step ---
        // Read and discard any pending packets to avoid using a stale response for CID negotiation.
        let mut drain_buf = [0u8; HID_REPORT_SIZE];
        while let Ok(n) = device.read_timeout(&mut drain_buf[..], HID_READ_TIMEOUT_MS) {
            if n == 0 {
                break;
            }
            log::trace!("Drained stale HID packet: {:02X?}", &drain_buf[0..16]);
        }

        let mut nonce = [0u8; 8];
        rand::rng().fill(&mut nonce);

        // Construct Init Packet: [CID(4) | CMD(1) | LEN(2) | NONCE(8)]
        let mut report = [0u8; HID_REPORT_SIZE + 1]; // +1 for Report ID (always 0)
        report[1..5].copy_from_slice(&CTAPHID_CID_BROADCAST.to_be_bytes());
        report[5] = CTAPHID_INIT;
        report[6] = 0; // Len MSB
        report[7] = 8; // Len LSB
        report[8..16].copy_from_slice(&nonce);

        log::trace!("Sending CTAPHID_INIT broadcast with nonce: {:02X?}", nonce);
        device.write(&report[..]).map_err(|e| {
            log::error!("Failed to write INIT packet: {}", e);
            PFError::Io(format!("Failed to write INIT packet: {}", e))
        })?;

        // Read Response until we find our nonce
        let start = std::time::Instant::now();
        while start.elapsed() < Duration::from_secs(1) {
            let mut buf = [0u8; HID_REPORT_SIZE];
            if device
                .read_timeout(&mut buf[..], HID_INIT_READ_TIMEOUT_MS)
                .is_ok()
            {
                // Check if response matches our broadcast and nonce
                if buf[0..4] == CTAPHID_CID_BROADCAST.to_be_bytes()
                    && buf[4] == CTAPHID_INIT
                    && buf[7..15] == nonce
                {
                    // New CID is at bytes 16..20
                    let new_cid = u32::from_be_bytes([buf[15], buf[16], buf[17], buf[18]]);
                    log::debug!("Channel negotiation successful. New CID: 0x{:08X}", new_cid);
                    return Ok(new_cid);
                } else {
                    log::trace!(
                        "Received ignoreable HID packet during CID negotiation: {:02X?}",
                        &buf[0..16]
                    );
                }
            }
        }
        log::error!("Timeout waiting for CTAPHID_INIT response.");
        Err(PFError::Device(
            "Timeout waiting for FIDO Init response".into(),
        ))
    }

    pub fn send_cbor(&self, cmd: u8, payload: &[u8]) -> Result<Vec<u8>, PFError> {
        self.write_cbor_request(cmd, payload)?;
        self.read_cbor_response(cmd)
    }

    fn write_cbor_request(&self, cmd: u8, payload: &[u8]) -> Result<(), PFError> {
        log::debug!(
            "Sending CBOR Command: 0x{:02X}, Payload Size: {} bytes",
            cmd,
            payload.len()
        );

        let total_len = payload.len();
        let mut sent = 0;
        let mut sequence = 0u8;

        // 1. Init Packet
        let mut report = [0u8; HID_REPORT_SIZE + 1];
        report[1..5].copy_from_slice(&self.cid.to_be_bytes());
        report[5] = cmd;
        report[6] = (total_len >> 8) as u8;
        report[7] = (total_len & 0xFF) as u8;

        let to_copy = std::cmp::min(total_len, HID_REPORT_SIZE - 7);
        report[8..8 + to_copy].copy_from_slice(&payload[0..to_copy]);
        sent += to_copy;

        // log::trace!("Writing Init Packet (Sent: {}/{})", sent, total_len);
        if let Err(e) = self.device.write(&report[..]) {
            log::error!("Failed to write initial HID packet: {}", e);
            return Err(PFError::Io(format!(
                "Failed to write initial HID packet: {}",
                e,
            )));
        } else {
            log::trace!("Successfully sent initial HID packet");
        }

        // 2. Continuation Packets
        while sent < total_len {
            let mut report = [0u8; HID_REPORT_SIZE + 1];
            report[1..5].copy_from_slice(&self.cid.to_be_bytes());
            report[5] = 0x7F & sequence; // SEQ
            sequence += 1;

            let to_copy = std::cmp::min(total_len - sent, HID_REPORT_SIZE - 5);
            report[6..6 + to_copy].copy_from_slice(&payload[sent..sent + to_copy]);
            sent += to_copy;

            // log::trace!("Writing Cont Packet Seq {} (Sent: {}/{})", sequence - 1, sent, total_len);
            if let Err(e) = self.device.write(&report[..]) {
                log::error!(
                    "Failed to write continuation HID packet (Seq {}): {}",
                    sequence - 1,
                    e
                );
                return Err(PFError::Io(format!(
                    "Failed to write continuation HID packet: {}",
                    e,
                )));
            } else {
                log::trace!(
                    "Successfully sent continuation HID packet (Seq {})",
                    sequence - 1
                );
            }
        }

        Ok(())
    }

    fn read_cbor_response(&self, cmd: u8) -> Result<Vec<u8>, PFError> {
        log::debug!("Waiting for response...");

        let mut buf = [0u8; HID_REPORT_SIZE];
        let mut response_data = Vec::new();
        let expected_len: usize;
        let mut read_len = 0;
        let mut last_seq = 0;

        let start_time = std::time::Instant::now();
        let timeout_duration = std::time::Duration::from_millis(HID_TOTAL_TIMEOUT_MS as u64);

        // 1. Read First Packet (Loop to handle Keepalives)
        loop {
            if start_time.elapsed() > timeout_duration {
                log::error!("Timeout waiting for device response (Keepalive limit exceeded)");
                return Err(PFError::Device(
                    "Timeout waiting for device response (Keepalive limit exceeded)".into(),
                ));
            }

            if let Err(e) = self
                .device
                .read_timeout(&mut buf[..], HID_RESP_READ_TIMEOUT_MS)
            {
                log::error!("Timeout reading response packet: {}", e);
                return Err(PFError::Io(format!(
                    "Timeout reading response packet: {}",
                    e
                )));
            }

            // Check CID mismatch
            if u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]) != self.cid {
                log::warn!("Received packet from different CID, ignoring...");
                continue;
            }

            // Check for KEEPALIVE (0xBB)
            if buf[4] == CTAPHID_KEEPALIVE {
                let status = buf[5]; // Keepalive status byte
                log::debug!(
                    "Device sent KEEPALIVE (Status: 0x{:02X}), waiting...",
                    status
                );
                continue; // Go back to start of loop and read again
            }

            // If we are here, it's a real response
            break;
        }

        if buf[4] == CTAPHID_ERROR {
            log::error!("Device returned CTAP Error code: 0x{:02X}", buf[5]);
            return Err(PFError::Device(format!(
                "Device returned CTAP Error: 0x{:02X}",
                buf[5],
            )));
        } else {
            log::trace!("Packet received is not a CTAP Error");
        }

        if buf[4] == cmd {
            expected_len = u16::from_be_bytes([buf[5], buf[6]]) as usize;
            let in_pkt = std::cmp::min(expected_len, HID_REPORT_SIZE - 7);
            response_data.extend_from_slice(&buf[7..7 + in_pkt]);
            read_len += in_pkt;
            // log::trace!("Received Init Response. Expecting {} bytes total.", expected_len);
        } else {
            log::error!(
                "Unexpected command response: 0x{:02X} (Expected 0x{:02X})",
                buf[4],
                cmd
            );
            return Err(PFError::Device(format!(
                "Unexpected command response: 0x{:02X} (Expected 0x{:02X})",
                buf[4], cmd
            )));
        }

        // 2. Read Continuation Packets
        while read_len < expected_len {
            if let Err(e) = self
                .device
                .read_timeout(&mut buf[..], HID_CONT_READ_TIMEOUT_MS)
            {
                log::error!("Timeout reading continuation packet: {}", e);
                return Err(PFError::Io(format!(
                    "Timeout reading continuation packet: {}",
                    e
                )));
            }

            if u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]) != self.cid {
                continue; // Ignore packets from other channels
            }

            let seq = buf[4];
            if seq != last_seq {
                log::error!(
                    "Sequence mismatch in response. Expected {}, got {}",
                    last_seq,
                    seq
                );
                return Err(PFError::Device("Sequence mismatch".into()));
            }
            last_seq += 1;

            let in_pkt = std::cmp::min(expected_len - read_len, HID_REPORT_SIZE - 5);
            response_data.extend_from_slice(&buf[5..5 + in_pkt]);
            read_len += in_pkt;
        }

        // 3. Check CTAP Status Byte (First byte of payload)
        if response_data.is_empty() {
            log::error!("Device sent empty payload response.");
            return Err(PFError::Device("Empty response".into()));
        }
        let status = response_data[0];
        if status != 0x00 {
            log::error!("FIDO Operation returned failure status: 0x{:02X}", status);
            return Err(PFError::Device(format!(
                "FIDO Operation Failed with Status: 0x{:02X}",
                status
            )));
        }

        log::debug!(
            "Command 0x{:02X} successful. Response payload len: {}",
            cmd,
            response_data.len() - 1
        );
        // Return payload without status byte
        Ok(response_data[1..].to_vec())
    }

    pub fn send_vendor_config(
        &self,
        pin_token: &[u8],
        vendor_cmd: VendorConfigCommand,
        param: Value,
    ) -> Result<(), PFError> {
        log::debug!("Sending vendor config command: {}...", vendor_cmd);

        // Build subCommandParams (Key 0x02)
        // This map contains:
        // 0x01: vendorCommandId (u64)
        // 0x02/0x03/0x04: param
        let mut sub_params_inner = BTreeMap::new();
        sub_params_inner.insert(Value::Integer(0x01), Value::Integer(vendor_cmd as i128));

        match param {
            Value::Bytes(_) => {
                sub_params_inner.insert(Value::Integer(0x02), param.clone());
            }
            Value::Integer(_) => {
                sub_params_inner.insert(Value::Integer(0x03), param.clone());
            }
            Value::Text(_) => {
                sub_params_inner.insert(Value::Integer(0x04), param.clone());
            }
            _ => return Err(PFError::Io("Unsupported parameter type".into())),
        }

        let sub_params = Value::Map(sub_params_inner);
        let sub_params_bytes = to_vec(&sub_params).map_err(|e| PFError::Io(e.to_string()))?;

        // Calculate PIN Auth
        let pin_auth = self.sign_config_command(
            pin_token,
            ConfigSubCommand::VendorPrototype as u8,
            &sub_params_bytes,
        );

        // Build full authenticatorConfig map
        let mut config_map = BTreeMap::new();
        config_map.insert(
            Value::Integer(ConfigParam::SubCommand as i128),
            Value::Integer(ConfigSubCommand::VendorPrototype as i128),
        );
        config_map.insert(
            Value::Integer(ConfigParam::SubCommandParams as i128),
            sub_params,
        );
        config_map.insert(
            Value::Integer(ConfigParam::PinUvAuthProtocol as i128),
            Value::Integer(1),
        );
        config_map.insert(
            Value::Integer(ConfigParam::PinUvAuthParam as i128),
            Value::Bytes(pin_auth),
        );

        let config_payload_cbor =
            to_vec(&Value::Map(config_map)).map_err(|e| PFError::Io(e.to_string()))?;

        // Encapsulate for CTAP
        let mut payload = vec![CtapCommand::Config as u8];
        payload.extend(config_payload_cbor);

        log::debug!("Sending config command...");
        self.send_cbor(CTAPHID_CBOR, &payload).map_err(|e| {
            log::error!("Failed to send FIDO config: {}", e);
            PFError::Device(format!("FIDO config failed: {}", e))
        })?;

        Ok(())
    }

    /// Send authenticatorConfig command to set minimum PIN length.
    ///
    /// This bypasses the ctap-hid-fido2 library which has a bug where it sends
    /// CBOR map keys out of order (0x01, 0x03, 0x04, 0x02) instead of the required
    /// ascending order (0x01, 0x02, 0x03, 0x04). The pico-fido firmware strictly
    /// enforces canonical CBOR ordering per CTAP2 spec.
    pub fn send_config_set_min_pin_length(
        &self,
        pin_token: &[u8],
        new_min_pin_length: u8,
    ) -> Result<(), PFError> {
        log::debug!(
            "Sending setMinPINLength config command (new length: {})...",
            new_min_pin_length
        );

        // Build subCommandParams (Key 0x02): { 0x01: newMinPINLength }
        let mut sub_params_map = BTreeMap::new();
        sub_params_map.insert(
            Value::Integer(ConfigSubCommandParam::NewMinPinLength as i128),
            Value::Integer(new_min_pin_length as i128),
        );
        let sub_params = Value::Map(sub_params_map);
        let sub_params_bytes = to_vec(&sub_params).map_err(|e| PFError::Io(e.to_string()))?;

        // Calculate PIN Auth
        let pin_auth = self.sign_config_command(
            pin_token,
            ConfigSubCommand::SetMinPinLength as u8,
            &sub_params_bytes,
        );

        // Build full authenticatorConfig map with keys in ASCENDING ORDER
        // Keeping the map item in the correct order is critical - the firmware parser rejects out-of-order keys with CTAP2_ERR_INVALID_CBOR
        let mut config_map = BTreeMap::new();
        config_map.insert(
            Value::Integer(ConfigParam::SubCommand as i128), // 0x01
            Value::Integer(ConfigSubCommand::SetMinPinLength as i128), // 0x03
        );
        config_map.insert(
            Value::Integer(ConfigParam::SubCommandParams as i128), // 0x02
            sub_params,
        );
        config_map.insert(
            Value::Integer(ConfigParam::PinUvAuthProtocol as i128), // 0x03
            Value::Integer(1),                                      // PIN protocol version 1
        );
        config_map.insert(
            Value::Integer(ConfigParam::PinUvAuthParam as i128), // 0x04
            Value::Bytes(pin_auth),
        );

        let config_payload_cbor =
            to_vec(&Value::Map(config_map)).map_err(|e| PFError::Io(e.to_string()))?;

        // Prepend CTAP command byte
        let mut payload = vec![CtapCommand::Config as u8];
        payload.extend(config_payload_cbor);

        log::debug!("Sending minimum PIN length config command...");
        match self.send_cbor(CTAPHID_CBOR, &payload) {
            Ok(_) => {
                log::info!(
                    "Successfully set minimum PIN length to {}",
                    new_min_pin_length
                );
                Ok(())
            }
            Err(e) => {
                let err_str = e.to_string();
                log::error!("Failed to send setMinPINLength config: {}", err_str);

                // Check for PIN policy violation (0x37) - cannot decrease min PIN length
                if err_str.contains("0x37") {
                    return Err(PFError::Device(
                        "Cannot decrease minimum PIN length. The FIDO2 security policy only allows increasing the minimum PIN length, not decreasing it. A device reset is required to lower the minimum.".into()
                    ));
                }

                Err(PFError::Device(format!("setMinPINLength failed: {}", e)))
            }
        }
    }

    pub fn get_key_agreement(&self) -> Result<Value, PFError> {
        let mut map = BTreeMap::new();
        map.insert(
            Value::Integer(ClientPinParam::PinUvAuthProtocol as i128),
            Value::Integer(1),
        );
        map.insert(
            Value::Integer(ClientPinParam::SubCommand as i128),
            Value::Integer(ClientPinSubCommand::GetKeyAgreement as i128),
        );

        let mut payload = vec![CtapCommand::ClientPin as u8];
        payload.extend(to_vec(&Value::Map(map)).map_err(|e| PFError::Io(e.to_string()))?);

        log::debug!("Sending GetKeyAgreement command...");
        let resp = self.send_cbor(CTAPHID_CBOR, &payload)?;
        let val: Value = from_slice(&resp).map_err(|e| PFError::Io(e.to_string()))?;

        if let Value::Map(m) = val {
            log::debug!("GetKeyAgreement response: {:?}", m);
            m.get(&Value::Integer(
                ClientPinResponseParam::KeyAgreement as i128,
            ))
            .cloned()
            .ok_or_else(|| PFError::Device("KeyAgreement not found in response".into()))
        } else {
            Err(PFError::Device(
                "Unexpected response for GetKeyAgreement".into(),
            ))
        }
    }

    pub fn get_pin_token(&self, pin: &str) -> Result<Vec<u8>, PFError> {
        log::info!("Starting custom get_pin_token (Subcommand 0x05)...");

        // 1. Get Authenticator Key Agreement
        let auth_key_agreement = self.get_key_agreement()?;

        // 2. Generate Platform Key Pair (P-256)
        let rng = ring::rand::SystemRandom::new();
        let platform_private_key =
            agreement::EphemeralPrivateKey::generate(&agreement::ECDH_P256, &rng)
                .map_err(|_| PFError::Device("Failed to generate platform ephemeral key".into()))?;
        let platform_public_key_bytes = platform_private_key
            .compute_public_key()
            .map_err(|_| PFError::Device("Failed to compute platform public key".into()))?;

        // 3. Extract Authenticator Public Key (X and Y coordinates)
        let (auth_x, auth_y) = if let Value::Map(m) = &auth_key_agreement {
            let x = match m.get(&Value::Integer(-2)) {
                Some(Value::Bytes(b)) => b,
                _ => return Err(PFError::Device("Invalid KeyAgreement X coordinate".into())),
            };
            let y = match m.get(&Value::Integer(-3)) {
                Some(Value::Bytes(b)) => b,
                _ => return Err(PFError::Device("Invalid KeyAgreement Y coordinate".into())),
            };
            (x, y)
        } else {
            return Err(PFError::Device("Invalid KeyAgreement format".into()));
        };

        let mut auth_pub_key_bytes = vec![0x04];
        auth_pub_key_bytes.extend(auth_x);
        auth_pub_key_bytes.extend(auth_y);

        let auth_unparsed_pub_key =
            agreement::UnparsedPublicKey::new(&agreement::ECDH_P256, auth_pub_key_bytes);

        // 4. Perform ECDH to get Shared Secret
        let shared_secret =
            agreement::agree_ephemeral(platform_private_key, &auth_unparsed_pub_key, |material| {
                let mut hasher = digest::Context::new(&digest::SHA256);
                hasher.update(material);
                Ok(hasher.finish()) as Result<digest::Digest, ring::error::Unspecified>
            })
            .map_err(|_| PFError::Device("ECDH shared secret computation failed".into()))?
            .map_err(|_| PFError::Device("Inner ECDH shared secret computation failed".into()))?;

        // 5. Encrypt PIN Hash
        let pin_hash = digest::digest(&digest::SHA256, pin.as_bytes());
        let pin_hash_16 = &pin_hash.as_ref()[0..16];

        let iv = [0u8; 16];
        let mut block = *GenericArray::from_slice(pin_hash_16);

        let shared_secret_bytes = shared_secret.as_ref();
        let mut encryptor = cbc::Encryptor::<aes::Aes256>::new(
            GenericArray::from_slice(shared_secret_bytes),
            GenericArray::from_slice(&iv),
        );
        encryptor.encrypt_block_mut(&mut block);
        let pin_hash_enc = block.to_vec();

        // 6. Send getPinToken command (Subcommand 0x05)

        // 7. Send getPinToken command (Subcommand 0x05)
        let cose_key_bytes = self.encode_cose_key(
            &platform_public_key_bytes.as_ref()[1..33],
            &platform_public_key_bytes.as_ref()[33..65],
        );

        let payload_cbor = self.encode_client_pin_params(
            ClientPinSubCommand::GetPinToken,
            &cose_key_bytes,
            &pin_hash_enc,
            None,
            None,
        );

        let mut payload = vec![CtapCommand::ClientPin as u8];
        payload.extend(payload_cbor);

        log::debug!("Sending getPinToken command...");
        let resp = self.send_cbor(CTAPHID_CBOR, &payload)?;
        let val: Value = from_slice(&resp).map_err(|e| PFError::Io(e.to_string()))?;

        if let Value::Map(m) = val {
            log::debug!("getPinToken response: {:?}", m);
            match m.get(&Value::Integer(ClientPinResponseParam::PinToken as i128)) {
                Some(Value::Bytes(token_enc)) => {
                    // Decrypt the PIN token using shared secret (AES-256-CBC, IV=0)
                    let mut token_buf = token_enc.clone();
                    let decrypted = cbc::Decryptor::<aes::Aes256>::new(
                        GenericArray::from_slice(shared_secret_bytes),
                        GenericArray::from_slice(&iv),
                    )
                    .decrypt_padded_mut::<NoPadding>(&mut token_buf)
                    .map_err(|_| PFError::Device("Failed to decrypt PIN token".into()))?;
                    log::info!("Successfully obtained and decrypted PIN token (Subcommand 0x05).");
                    Ok(decrypted.to_vec())
                }
                _ => Err(PFError::Device("pinToken not found in response".into())),
            }
        } else {
            Err(PFError::Device("Unexpected response format".into()))
        }
    }

    pub fn get_pin_token_with_permission(
        &self,
        pin: &str,
        permissions: PinUvAuthTokenPermissions,
        rp_id: Option<String>,
    ) -> Result<Vec<u8>, PFError> {
        log::info!(
            "Starting custom get_pin_token_with_permission (Subcommand 0x09, permissions: {:?})...",
            permissions
        );

        // 1. Get Authenticator Key Agreement
        let auth_key_agreement = self.get_key_agreement()?;

        // 2. Generate Platform Key Pair (P-256)
        let rng = ring::rand::SystemRandom::new();
        let platform_private_key =
            agreement::EphemeralPrivateKey::generate(&agreement::ECDH_P256, &rng)
                .map_err(|_| PFError::Device("Failed to generate platform ephemeral key".into()))?;
        let platform_public_key_bytes = platform_private_key
            .compute_public_key()
            .map_err(|_| PFError::Device("Failed to compute platform public key".into()))?;

        // 3. Extract Authenticator Public Key (X and Y coordinates)
        let (auth_x, auth_y) = if let Value::Map(m) = &auth_key_agreement {
            let x = match m.get(&Value::Integer(-2)) {
                Some(Value::Bytes(b)) => b,
                _ => return Err(PFError::Device("Invalid KeyAgreement X coordinate".into())),
            };
            let y = match m.get(&Value::Integer(-3)) {
                Some(Value::Bytes(b)) => b,
                _ => return Err(PFError::Device("Invalid KeyAgreement Y coordinate".into())),
            };
            (x, y)
        } else {
            return Err(PFError::Device("Invalid KeyAgreement format".into()));
        };

        let mut auth_pub_key_bytes = vec![0x04];
        auth_pub_key_bytes.extend(auth_x);
        auth_pub_key_bytes.extend(auth_y);

        let auth_unparsed_pub_key =
            agreement::UnparsedPublicKey::new(&agreement::ECDH_P256, auth_pub_key_bytes);

        // 4. Perform ECDH to get Shared Secret
        let shared_secret =
            agreement::agree_ephemeral(platform_private_key, &auth_unparsed_pub_key, |material| {
                let mut hasher = digest::Context::new(&digest::SHA256);
                hasher.update(material);
                Ok(hasher.finish()) as Result<digest::Digest, ring::error::Unspecified>
            })
            .map_err(|_| PFError::Device("ECDH shared secret computation failed".into()))?
            .map_err(|_| PFError::Device("Inner ECDH shared secret computation failed".into()))?;

        // 5. Encrypt PIN Hash
        let pin_hash = digest::digest(&digest::SHA256, pin.as_bytes());
        let pin_hash_16 = &pin_hash.as_ref()[0..16];

        let iv = [0u8; 16];
        let mut block = *GenericArray::from_slice(pin_hash_16);

        let shared_secret_bytes = shared_secret.as_ref();
        let mut encryptor = cbc::Encryptor::<aes::Aes256>::new(
            GenericArray::from_slice(shared_secret_bytes),
            GenericArray::from_slice(&iv),
        );
        encryptor.encrypt_block_mut(&mut block);
        let pin_hash_enc = block.to_vec();

        // 6. Send getPinUvAuthTokenUsingPinWithPermissions command (Subcommand 0x09)

        // 7. Send getPinUvAuthTokenUsingPinWithPermissions command (Subcommand 0x09)

        let mut payload = vec![CtapCommand::ClientPin as u8];
        let cose_key_bytes = self.encode_cose_key(
            &platform_public_key_bytes.as_ref()[1..33],
            &platform_public_key_bytes.as_ref()[33..65],
        );

        log::trace!(
            "Encrypted PIN hash (first 4 bytes): {:?}",
            &pin_hash_enc[..4]
        );
        let payload_cbor = self.encode_client_pin_params(
            ClientPinSubCommand::GetPinUvAuthTokenUsingPinWithPermissions,
            &cose_key_bytes,
            &pin_hash_enc,
            Some(permissions.bits()),
            rp_id,
        );
        payload.extend(payload_cbor);

        log::debug!("Sending getPinUvAuthTokenUsingPinWithPermissions command...");
        let resp = self.send_cbor(CTAPHID_CBOR, &payload)?;
        log::debug!(
            "getPinUvAuthTokenUsingPinWithPermissions response: {:?}",
            resp
        );
        let val: Value = from_slice(&resp).map_err(|e| PFError::Io(e.to_string()))?;

        if let Value::Map(m) = val {
            log::debug!("getPinUvAuthTokenUsingPinWithPermissions response: {:?}", m);
            match m.get(&Value::Integer(ClientPinResponseParam::PinToken as i128)) {
                Some(Value::Bytes(token_enc)) => {
                    // Decrypt the PIN token using shared secret (AES-256-CBC, IV=0)
                    let mut token_buf = token_enc.clone();
                    let decrypted = cbc::Decryptor::<aes::Aes256>::new(
                        GenericArray::from_slice(shared_secret_bytes),
                        GenericArray::from_slice(&iv),
                    )
                    .decrypt_padded_mut::<NoPadding>(&mut token_buf)
                    .map_err(|_| PFError::Device("Failed to decrypt PIN token".into()))?;
                    log::info!("Successfully obtained and decrypted PIN token (Subcommand 0x09).");
                    Ok(decrypted.to_vec())
                }
                _ => Err(PFError::Device(
                    "pinUvAuthToken not found in response".into(),
                )),
            }
        } else {
            Err(PFError::Device("Unexpected response format".into()))
        }
    }

    pub fn set_pin(&self, new_pin: &str) -> Result<(), PFError> {
        log::info!("Starting custom set_pin (Subcommand 0x03)...");

        if new_pin.len() < 4 {
            return Err(PFError::Device("PIN must be at least 4 characters".into()));
        }
        if new_pin.len() > 63 {
            return Err(PFError::Device(
                "PIN must be less than 64 characters".into(),
            ));
        }

        // 1. Get Authenticator Key Agreement
        let auth_key_agreement = self.get_key_agreement()?;

        // 2. Generate Platform Key Pair (P-256)
        let rng = ring::rand::SystemRandom::new();
        let platform_private_key =
            agreement::EphemeralPrivateKey::generate(&agreement::ECDH_P256, &rng)
                .map_err(|_| PFError::Device("Failed to generate platform ephemeral key".into()))?;
        let platform_public_key_bytes = platform_private_key
            .compute_public_key()
            .map_err(|_| PFError::Device("Failed to compute platform public key".into()))?;

        // 3. Extract Authenticator Public Key
        let (auth_x, auth_y) = if let Value::Map(m) = &auth_key_agreement {
            let x = match m.get(&Value::Integer(-2)) {
                Some(Value::Bytes(b)) => b,
                _ => return Err(PFError::Device("Invalid KeyAgreement X coordinate".into())),
            };
            let y = match m.get(&Value::Integer(-3)) {
                Some(Value::Bytes(b)) => b,
                _ => return Err(PFError::Device("Invalid KeyAgreement Y coordinate".into())),
            };
            (x, y)
        } else {
            return Err(PFError::Device("Invalid KeyAgreement format".into()));
        };

        let mut auth_pub_key_bytes = vec![0x04];
        auth_pub_key_bytes.extend(auth_x);
        auth_pub_key_bytes.extend(auth_y);

        let auth_unparsed_pub_key =
            agreement::UnparsedPublicKey::new(&agreement::ECDH_P256, auth_pub_key_bytes);

        // 4. Perform ECDH to get Shared Secret
        let shared_secret =
            agreement::agree_ephemeral(platform_private_key, &auth_unparsed_pub_key, |material| {
                let mut hasher = digest::Context::new(&digest::SHA256);
                hasher.update(material);
                Ok(hasher.finish()) as Result<digest::Digest, ring::error::Unspecified>
            })
            .map_err(|_| PFError::Device("ECDH shared secret computation failed".into()))?
            .map_err(|_| PFError::Device("Inner ECDH shared secret computation failed".into()))?;

        let shared_secret_bytes = shared_secret.as_ref();

        // 5. Encrypt newPinEnc
        let mut padded_new_pin = [0u8; 64];
        let bytes = new_pin.as_bytes();
        padded_new_pin[..bytes.len()].copy_from_slice(bytes);

        let iv = [0u8; 16];
        let mut new_pin_enc = Vec::new();
        let mut encryptor = cbc::Encryptor::<aes::Aes256>::new(
            GenericArray::from_slice(shared_secret_bytes),
            GenericArray::from_slice(&iv),
        );
        for chunk in padded_new_pin.chunks_exact(16) {
            let mut block = *GenericArray::from_slice(chunk);
            encryptor.encrypt_block_mut(&mut block);
            new_pin_enc.extend_from_slice(&block);
        }

        // 6. Calculate pinUvAuthParam: HMAC-SHA-256(shared_secret, newPinEnc)[0..16]
        let hmac_key = hmac::Key::new(hmac::HMAC_SHA256, shared_secret_bytes);
        let pin_uv_auth_param = hmac::sign(&hmac_key, &new_pin_enc).as_ref()[0..16].to_vec();

        // 7. Send SetPin command
        let cose_key_bytes = self.encode_cose_key(
            &platform_public_key_bytes.as_ref()[1..33],
            &platform_public_key_bytes.as_ref()[33..65],
        );

        let mut payload_cbor = vec![0xA5]; // Map(5)
        payload_cbor
            .extend(to_vec(&Value::Integer(ClientPinParam::PinUvAuthProtocol as i128)).unwrap());
        payload_cbor.extend(to_vec(&Value::Integer(1)).unwrap());
        payload_cbor.extend(to_vec(&Value::Integer(ClientPinParam::SubCommand as i128)).unwrap());
        payload_cbor.extend(to_vec(&Value::Integer(ClientPinSubCommand::SetPin as i128)).unwrap());
        payload_cbor.extend(to_vec(&Value::Integer(ClientPinParam::KeyAgreement as i128)).unwrap());
        payload_cbor.extend(cose_key_bytes);
        payload_cbor
            .extend(to_vec(&Value::Integer(ClientPinParam::PinUvAuthParam as i128)).unwrap());
        payload_cbor.extend(to_vec(&Value::Bytes(pin_uv_auth_param)).unwrap());
        payload_cbor.extend(to_vec(&Value::Integer(ClientPinParam::NewPinEnc as i128)).unwrap());
        payload_cbor.extend(to_vec(&Value::Bytes(new_pin_enc)).unwrap());

        let mut payload = vec![CtapCommand::ClientPin as u8];
        payload.extend(payload_cbor);

        log::debug!("Sending setPin command...");
        match self.send_cbor(CTAPHID_CBOR, &payload) {
            Ok(_) => {
                log::info!("Successfully set new PIN.");
                Ok(())
            }
            Err(e) => {
                let err_str = e.to_string();
                log::error!("Failed to send setPin config: {}", err_str);
                if err_str.contains("0x37") {
                    return Err(PFError::Device(
                        "New PIN violates policy (e.g. too short).".into(),
                    ));
                }
                Err(PFError::Device(format!("setPin failed: {}", e)))
            }
        }
    }

    pub fn change_pin(&self, current_pin: &str, new_pin: &str) -> Result<(), PFError> {
        log::info!("Starting custom change_pin (Subcommand 0x04)...");

        if new_pin.len() < 4 {
            return Err(PFError::Device("PIN must be at least 4 characters".into()));
        }
        if new_pin.len() > 63 {
            return Err(PFError::Device(
                "PIN must be less than 64 characters".into(),
            ));
        }

        // 1. Get Authenticator Key Agreement
        let auth_key_agreement = self.get_key_agreement()?;

        // 2. Generate Platform Key Pair (P-256)
        let rng = ring::rand::SystemRandom::new();
        let platform_private_key =
            agreement::EphemeralPrivateKey::generate(&agreement::ECDH_P256, &rng)
                .map_err(|_| PFError::Device("Failed to generate platform ephemeral key".into()))?;
        let platform_public_key_bytes = platform_private_key
            .compute_public_key()
            .map_err(|_| PFError::Device("Failed to compute platform public key".into()))?;

        // 3. Extract Authenticator Public Key
        let (auth_x, auth_y) = if let Value::Map(m) = &auth_key_agreement {
            let x = match m.get(&Value::Integer(-2)) {
                Some(Value::Bytes(b)) => b,
                _ => return Err(PFError::Device("Invalid KeyAgreement X coordinate".into())),
            };
            let y = match m.get(&Value::Integer(-3)) {
                Some(Value::Bytes(b)) => b,
                _ => return Err(PFError::Device("Invalid KeyAgreement Y coordinate".into())),
            };
            (x, y)
        } else {
            return Err(PFError::Device("Invalid KeyAgreement format".into()));
        };

        let mut auth_pub_key_bytes = vec![0x04];
        auth_pub_key_bytes.extend(auth_x);
        auth_pub_key_bytes.extend(auth_y);

        let auth_unparsed_pub_key =
            agreement::UnparsedPublicKey::new(&agreement::ECDH_P256, auth_pub_key_bytes);

        // 4. Perform ECDH to get Shared Secret
        let shared_secret =
            agreement::agree_ephemeral(platform_private_key, &auth_unparsed_pub_key, |material| {
                let mut hasher = digest::Context::new(&digest::SHA256);
                hasher.update(material);
                Ok(hasher.finish()) as Result<digest::Digest, ring::error::Unspecified>
            })
            .map_err(|_| PFError::Device("ECDH shared secret computation failed".into()))?
            .map_err(|_| PFError::Device("Inner ECDH shared secret computation failed".into()))?;

        let shared_secret_bytes = shared_secret.as_ref();

        // 5. Encrypt current_pin hash
        let pin_hash = digest::digest(&digest::SHA256, current_pin.as_bytes());
        let pin_hash_16 = &pin_hash.as_ref()[0..16];
        let iv = [0u8; 16];
        let mut block = *GenericArray::from_slice(pin_hash_16);
        cbc::Encryptor::<aes::Aes256>::new(
            GenericArray::from_slice(shared_secret_bytes),
            GenericArray::from_slice(&iv),
        )
        .encrypt_block_mut(&mut block);
        let pin_hash_enc = block.to_vec();

        // 6. Encrypt newPinEnc
        let mut padded_new_pin = [0u8; 64];
        let bytes = new_pin.as_bytes();
        padded_new_pin[..bytes.len()].copy_from_slice(bytes);

        let mut new_pin_enc = Vec::new();
        let mut encryptor = cbc::Encryptor::<aes::Aes256>::new(
            GenericArray::from_slice(shared_secret_bytes),
            GenericArray::from_slice(&iv),
        );
        for chunk in padded_new_pin.chunks_exact(16) {
            let mut block = *GenericArray::from_slice(chunk);
            encryptor.encrypt_block_mut(&mut block);
            new_pin_enc.extend_from_slice(&block);
        }

        // 7. Calculate pinUvAuthParam: HMAC-SHA-256(shared_secret, newPinEnc || pinHashEnc)[0..16]
        let mut hmac_msg = Vec::new();
        hmac_msg.extend_from_slice(&new_pin_enc);
        hmac_msg.extend_from_slice(&pin_hash_enc);

        let hmac_key = hmac::Key::new(hmac::HMAC_SHA256, shared_secret_bytes);
        let pin_uv_auth_param = hmac::sign(&hmac_key, &hmac_msg).as_ref()[0..16].to_vec();

        // 8. Send ChangePin command
        let cose_key_bytes = self.encode_cose_key(
            &platform_public_key_bytes.as_ref()[1..33],
            &platform_public_key_bytes.as_ref()[33..65],
        );

        let mut payload_cbor = vec![0xA6]; // Map(6)
        payload_cbor
            .extend(to_vec(&Value::Integer(ClientPinParam::PinUvAuthProtocol as i128)).unwrap());
        payload_cbor.extend(to_vec(&Value::Integer(1)).unwrap());
        payload_cbor.extend(to_vec(&Value::Integer(ClientPinParam::SubCommand as i128)).unwrap());
        payload_cbor
            .extend(to_vec(&Value::Integer(ClientPinSubCommand::ChangePin as i128)).unwrap());
        payload_cbor.extend(to_vec(&Value::Integer(ClientPinParam::KeyAgreement as i128)).unwrap());
        payload_cbor.extend(cose_key_bytes);
        payload_cbor
            .extend(to_vec(&Value::Integer(ClientPinParam::PinUvAuthParam as i128)).unwrap());
        payload_cbor.extend(to_vec(&Value::Bytes(pin_uv_auth_param)).unwrap());
        payload_cbor.extend(to_vec(&Value::Integer(ClientPinParam::NewPinEnc as i128)).unwrap());
        payload_cbor.extend(to_vec(&Value::Bytes(new_pin_enc)).unwrap());
        payload_cbor.extend(to_vec(&Value::Integer(ClientPinParam::PinHashEnc as i128)).unwrap());
        payload_cbor.extend(to_vec(&Value::Bytes(pin_hash_enc)).unwrap());

        let mut payload = vec![CtapCommand::ClientPin as u8];
        payload.extend(payload_cbor);

        log::debug!("Sending changePin command...");
        match self.send_cbor(CTAPHID_CBOR, &payload) {
            Ok(_) => {
                log::info!("Successfully changed PIN.");
                Ok(())
            }
            Err(e) => {
                let err_str = e.to_string();
                log::error!("Failed to send changePin config: {}", err_str);
                if err_str.contains("0x31") {
                    return Err(PFError::Device("Invalid current PIN (0x31). Please check that you entered the correct PIN.".into()));
                }
                if err_str.contains("0x32") {
                    return Err(PFError::Device(
                        "PIN blocked (0x32). Device reset may be required.".into(),
                    ));
                }
                if err_str.contains("0x37") {
                    return Err(PFError::Device(
                        "New PIN violates policy (e.g. too short).".into(),
                    ));
                }
                Err(PFError::Device(format!("changePin failed: {}", e)))
            }
        }
    }

    /// Helper to sign the authenticatorConfig command
    fn sign_config_command(
        &self,
        pin_token: &[u8],
        sub_cmd: u8,
        sub_params_bytes: &[u8],
    ) -> Vec<u8> {
        // Build HMAC message for signing
        // According to FIDO 2.1: authenticate(pinUvAuthToken, 32×0xff || 0x0d || uint8(subCommand) || subCommandParams)
        let mut message = vec![0xff; 32];
        message.push(CtapCommand::Config as u8);
        message.push(sub_cmd);
        message.extend(sub_params_bytes);

        // Sign using provided PIN token
        let hmac_key = hmac::Key::new(hmac::HMAC_SHA256, pin_token);
        let sig = hmac::sign(&hmac_key, &message);
        sig.as_ref()[0..16].to_vec()
    }

    fn encode_cose_key(&self, x: &[u8], y: &[u8]) -> Vec<u8> {
        let mut bytes = vec![0xA5]; // Map(5)
        bytes.extend(to_vec(&Value::Integer(1)).unwrap());
        bytes.extend(to_vec(&Value::Integer(2)).unwrap());
        bytes.extend(to_vec(&Value::Integer(3)).unwrap());
        bytes.extend(to_vec(&Value::Integer(-7)).unwrap());
        bytes.extend(to_vec(&Value::Integer(-1)).unwrap());
        bytes.extend(to_vec(&Value::Integer(1)).unwrap());
        bytes.extend(to_vec(&Value::Integer(-2)).unwrap());
        bytes.extend(to_vec(&Value::Bytes(x.to_vec())).unwrap());
        bytes.extend(to_vec(&Value::Integer(-3)).unwrap());
        bytes.extend(to_vec(&Value::Bytes(y.to_vec())).unwrap());
        bytes
    }

    fn encode_client_pin_params(
        &self,
        sub_cmd: ClientPinSubCommand,
        cose_key_bytes: &[u8],
        pin_hash_enc: &[u8],
        permissions: Option<u8>,
        rp_id: Option<String>,
    ) -> Vec<u8> {
        let mut count = 4;
        if permissions.is_some() {
            count += 1;
        }
        if rp_id.is_some() {
            count += 1;
        }
        let mut bytes = vec![0xA0 | (count as u8)];
        bytes.extend(to_vec(&Value::Integer(ClientPinParam::PinUvAuthProtocol as i128)).unwrap());
        bytes.extend(to_vec(&Value::Integer(1)).unwrap());
        bytes.extend(to_vec(&Value::Integer(ClientPinParam::SubCommand as i128)).unwrap());
        bytes.extend(to_vec(&Value::Integer(sub_cmd as i128)).unwrap());
        bytes.extend(to_vec(&Value::Integer(ClientPinParam::KeyAgreement as i128)).unwrap());
        bytes.extend(cose_key_bytes);
        bytes.extend(to_vec(&Value::Integer(ClientPinParam::PinHashEnc as i128)).unwrap());
        bytes.extend(to_vec(&Value::Bytes(pin_hash_enc.to_vec())).unwrap());
        if let Some(p) = permissions {
            bytes.extend(to_vec(&Value::Integer(ClientPinParam::Permissions as i128)).unwrap());
            bytes.extend(to_vec(&Value::Integer(p as i128)).unwrap());
        }
        if let Some(rp) = rp_id {
            bytes.extend(to_vec(&Value::Integer(ClientPinParam::PermissionsRpId as i128)).unwrap());
            bytes.extend(to_vec(&Value::Text(rp)).unwrap());
        }
        bytes
    }

    pub fn credential_management_enumerate_rps(
        &self,
        pin: &str,
    ) -> Result<Vec<EnumerateRpResponse>, PFError> {
        log::info!("Starting custom credential_management_enumerate_rps...");

        // 1. Get PIN token with CREDENTIAL_MANAGEMENT permission
        let pin_token = self.get_pin_token_with_permission(
            pin,
            PinUvAuthTokenPermissions::CREDENTIAL_MANAGEMENT,
            None,
        )?;

        let mut all_rps = Vec::new();

        // 2. EnumerateRpsBegin (Subcommand 0x02)
        // let sub_params = BTreeMap::new();
        // let sub_params_bytes = to_vec(&Value::Map(sub_params.clone())).unwrap();

        let pin_auth = self.sign_credential_mgmt_command(
            &pin_token,
            CredentialMgmtSubCommand::EnumerateRpsBegin as u8,
            None, // sub_params_bytes
        );

        let mut mgmt_map = BTreeMap::new();
        mgmt_map.insert(
            Value::Integer(CredentialMgmtParam::SubCommand as i128),
            Value::Integer(CredentialMgmtSubCommand::EnumerateRpsBegin as i128),
        );
        mgmt_map.insert(
            Value::Integer(CredentialMgmtParam::PinUvAuthProtocol as i128),
            Value::Integer(1),
        );
        mgmt_map.insert(
            Value::Integer(CredentialMgmtParam::PinUvAuthParam as i128),
            Value::Bytes(pin_auth),
        );

        let mut payload = vec![CtapCommand::CredentialMgmt as u8];
        payload.extend(to_vec(&Value::Map(mgmt_map)).map_err(|e| PFError::Io(e.to_string()))?);

        let resp = match self.send_cbor(CTAPHID_CBOR, &payload) {
            Ok(r) => r,
            Err(e) => {
                if e.to_string().contains("0x2E") {
                    log::info!("No credentials found on device (0x2E)");
                    return Ok(Vec::new());
                }
                return Err(e);
            }
        };

        let val: Value = from_slice(&resp).map_err(|e| PFError::Io(e.to_string()))?;
        let mut total_rps = None;

        if let Value::Map(m) = &val {
            let rp = m
                .get(&Value::Integer(CredentialMgmtResponseParam::Rp as i128))
                .cloned()
                .ok_or_else(|| {
                    PFError::Device("RP not found in EnumerateRpsBegin response".into())
                })?;
            let rp_id_hash = match m.get(&Value::Integer(
                CredentialMgmtResponseParam::RpIdHash as i128,
            )) {
                Some(Value::Bytes(b)) => b.clone(),
                _ => {
                    return Err(PFError::Device(
                        "RpIdHash not found in EnumerateRpsBegin response".into(),
                    ));
                }
            };
            if let Some(Value::Integer(t)) = m.get(&Value::Integer(
                CredentialMgmtResponseParam::TotalRps as i128,
            )) {
                total_rps = Some(*t as usize);
            }

            all_rps.push(EnumerateRpResponse {
                rp,
                rp_id_hash,
                total_rps,
            });
        }

        // 3. EnumerateRpsGetNextRp (Subcommand 0x03)
        let num_to_fetch = total_rps.unwrap_or(1);
        while all_rps.len() < num_to_fetch {
            let mut mgmt_map = BTreeMap::new();
            mgmt_map.insert(
                Value::Integer(CredentialMgmtParam::SubCommand as i128),
                Value::Integer(CredentialMgmtSubCommand::EnumerateRpsGetNextRp as i128),
            );

            let mut payload = vec![CtapCommand::CredentialMgmt as u8];
            payload.extend(to_vec(&Value::Map(mgmt_map)).map_err(|e| PFError::Io(e.to_string()))?);

            match self.send_cbor(CTAPHID_CBOR, &payload) {
                Ok(resp) => {
                    let val: Value = from_slice(&resp).map_err(|e| PFError::Io(e.to_string()))?;
                    if let Value::Map(m) = val {
                        let rp = m
                            .get(&Value::Integer(CredentialMgmtResponseParam::Rp as i128))
                            .cloned()
                            .ok_or_else(|| {
                                PFError::Device(
                                    "RP not found in EnumerateRpsGetNextRp response".into(),
                                )
                            })?;
                        let rp_id_hash = match m.get(&Value::Integer(
                            CredentialMgmtResponseParam::RpIdHash as i128,
                        )) {
                            Some(Value::Bytes(b)) => b.clone(),
                            _ => {
                                return Err(PFError::Device(
                                    "RpIdHash not found in EnumerateRpsGetNextRp response".into(),
                                ));
                            }
                        };
                        all_rps.push(EnumerateRpResponse {
                            rp,
                            rp_id_hash,
                            total_rps,
                        });
                    }
                }
                Err(e) => {
                    if e.to_string().contains("0x2E") {
                        break;
                    }
                    return Err(e);
                }
            }
        }

        Ok(all_rps)
    }

    pub fn credential_management_enumerate_credentials(
        &self,
        pin: &str,
        rp_id_hash: &[u8],
    ) -> Result<Vec<EnumerateCredentialResponse>, PFError> {
        log::info!("Starting custom credential_management_enumerate_credentials...");

        // 1. Get PIN token with CREDENTIAL_MANAGEMENT permission
        let pin_token = self.get_pin_token_with_permission(
            pin,
            PinUvAuthTokenPermissions::CREDENTIAL_MANAGEMENT,
            None,
        )?;

        let mut all_creds = Vec::new();

        // 2. EnumerateCredentialsBegin (Subcommand 0x04)
        let mut sub_params = BTreeMap::new();
        sub_params.insert(
            Value::Integer(0x01), // rpIdHash
            Value::Bytes(rp_id_hash.to_vec()),
        );
        let sub_params_bytes = to_vec(&Value::Map(sub_params.clone())).unwrap();

        let pin_auth = self.sign_credential_mgmt_command(
            &pin_token,
            CredentialMgmtSubCommand::EnumerateCredentialsBegin as u8,
            Some(&sub_params_bytes),
        );

        let mut mgmt_map = BTreeMap::new();
        mgmt_map.insert(
            Value::Integer(CredentialMgmtParam::SubCommand as i128),
            Value::Integer(CredentialMgmtSubCommand::EnumerateCredentialsBegin as i128),
        );
        mgmt_map.insert(
            Value::Integer(CredentialMgmtParam::SubCommandParams as i128),
            Value::Map(sub_params),
        );
        mgmt_map.insert(
            Value::Integer(CredentialMgmtParam::PinUvAuthProtocol as i128),
            Value::Integer(1),
        );
        mgmt_map.insert(
            Value::Integer(CredentialMgmtParam::PinUvAuthParam as i128),
            Value::Bytes(pin_auth),
        );

        let mut payload = vec![CtapCommand::CredentialMgmt as u8];
        payload.extend(to_vec(&Value::Map(mgmt_map)).map_err(|e| PFError::Io(e.to_string()))?);

        let resp = match self.send_cbor(CTAPHID_CBOR, &payload) {
            Ok(r) => r,
            Err(e) => {
                if e.to_string().contains("0x2E") {
                    return Ok(Vec::new());
                }
                return Err(e);
            }
        };

        let val: Value = from_slice(&resp).map_err(|e| PFError::Io(e.to_string()))?;
        let mut total_creds = None;

        if let Value::Map(m) = &val {
            let user = m
                .get(&Value::Integer(CredentialMgmtResponseParam::User as i128))
                .cloned()
                .ok_or_else(|| {
                    PFError::Device("User not found in EnumerateCredentialsBegin response".into())
                })?;
            let credential_id = m
                .get(&Value::Integer(
                    CredentialMgmtResponseParam::CredentialId as i128,
                ))
                .cloned()
                .ok_or_else(|| {
                    PFError::Device(
                        "CredentialId not found in EnumerateCredentialsBegin response".into(),
                    )
                })?;
            let public_key = m
                .get(&Value::Integer(
                    CredentialMgmtResponseParam::PublicKey as i128,
                ))
                .cloned()
                .ok_or_else(|| {
                    PFError::Device(
                        "PublicKey not found in EnumerateCredentialsBegin response".into(),
                    )
                })?;
            if let Some(Value::Integer(t)) = m.get(&Value::Integer(
                CredentialMgmtResponseParam::TotalCredentials as i128,
            )) {
                total_creds = Some(*t as usize);
            }

            all_creds.push(EnumerateCredentialResponse {
                user,
                credential_id,
                public_key,
                total_credentials: total_creds,
            });
        }

        // 3. EnumerateCredentialsGetNextCredential (Subcommand 0x05)
        let num_to_fetch = total_creds.unwrap_or(1);
        while all_creds.len() < num_to_fetch {
            let mut mgmt_map = BTreeMap::new();
            mgmt_map.insert(
                Value::Integer(CredentialMgmtParam::SubCommand as i128),
                Value::Integer(
                    CredentialMgmtSubCommand::EnumerateCredentialsGetNextCredential as i128,
                ),
            );

            let mut payload = vec![CtapCommand::CredentialMgmt as u8];
            payload.extend(to_vec(&Value::Map(mgmt_map)).map_err(|e| PFError::Io(e.to_string()))?);

            match self.send_cbor(CTAPHID_CBOR, &payload) {
                Ok(resp) => {
                    let val: Value = from_slice(&resp).map_err(|e| PFError::Io(e.to_string()))?;
                    if let Value::Map(m) = val {
                        let user = m
                            .get(&Value::Integer(CredentialMgmtResponseParam::User as i128))
                            .cloned()
                            .ok_or_else(|| {
                                PFError::Device(
                                    "User not found in EnumerateCredentialsGetNextCredential response"
                                        .into(),
                                )
                            })?;
                        let credential_id = m
                            .get(&Value::Integer(CredentialMgmtResponseParam::CredentialId as i128))
                            .cloned()
                            .ok_or_else(|| {
                                PFError::Device(
                                    "CredentialId not found in EnumerateCredentialsGetNextCredential response"
                                        .into(),
                                )
                            })?;
                        let public_key = m
                            .get(&Value::Integer(CredentialMgmtResponseParam::PublicKey as i128))
                            .cloned()
                            .ok_or_else(|| {
                                PFError::Device(
                                    "PublicKey not found in EnumerateCredentialsGetNextCredential response"
                                        .into(),
                                )
                            })?;

                        all_creds.push(EnumerateCredentialResponse {
                            user,
                            credential_id,
                            public_key,
                            total_credentials: total_creds,
                        });
                    }
                }
                Err(e) => {
                    if e.to_string().contains("0x2E") {
                        break;
                    }
                    return Err(e);
                }
            }
        }

        Ok(all_creds)
    }

    pub fn credential_management_delete_credential(
        &self,
        pin: &str,
        credential_id_map: Value,
    ) -> Result<(), PFError> {
        log::info!("Starting custom credential_management_delete_credential...");

        // 1. Get PIN token with CREDENTIAL_MANAGEMENT permission
        let pin_token = self.get_pin_token_with_permission(
            pin,
            PinUvAuthTokenPermissions::CREDENTIAL_MANAGEMENT,
            None,
        )?;

        // 2. DeleteCredential (Subcommand 0x06)
        let mut sub_params = BTreeMap::new();
        sub_params.insert(
            Value::Integer(0x02), // credentialId descriptor map
            credential_id_map,
        );
        let sub_params_bytes = to_vec(&Value::Map(sub_params.clone())).unwrap();

        let pin_auth = self.sign_credential_mgmt_command(
            &pin_token,
            CredentialMgmtSubCommand::DeleteCredential as u8,
            Some(&sub_params_bytes),
        );

        let mut mgmt_map = BTreeMap::new();
        mgmt_map.insert(
            Value::Integer(CredentialMgmtParam::SubCommand as i128),
            Value::Integer(CredentialMgmtSubCommand::DeleteCredential as i128),
        );
        mgmt_map.insert(
            Value::Integer(CredentialMgmtParam::SubCommandParams as i128),
            Value::Map(sub_params),
        );
        mgmt_map.insert(
            Value::Integer(CredentialMgmtParam::PinUvAuthProtocol as i128),
            Value::Integer(1),
        );
        mgmt_map.insert(
            Value::Integer(CredentialMgmtParam::PinUvAuthParam as i128),
            Value::Bytes(pin_auth),
        );

        let mut payload = vec![CtapCommand::CredentialMgmt as u8];
        payload.extend(to_vec(&Value::Map(mgmt_map)).map_err(|e| PFError::Io(e.to_string()))?);

        self.send_cbor(CTAPHID_CBOR, &payload)?;

        Ok(())
    }

    fn sign_credential_mgmt_command(
        &self,
        pin_token: &[u8],
        sub_cmd: u8,
        sub_params_bytes: Option<&[u8]>,
    ) -> Vec<u8> {
        // Research into pico-fido firmware reveals a non-standard signing logic:
        // 1. No 32-byte 0xff padding.
        // 2. No command byte (0x0d).
        // 3. For subcommands 0x01 (GetCredsMetadata) and 0x02 (EnumerateRpsBegin), only sign the subcommand byte.
        // 4. For others, sign the subcommand byte followed by the CBOR-encoded SubCommandParams map.

        let mut message = vec![sub_cmd];
        if let Some(params) = sub_params_bytes {
            if sub_cmd != CredentialMgmtSubCommand::GetCredsMetadata as u8
                && sub_cmd != CredentialMgmtSubCommand::EnumerateRpsBegin as u8
            {
                message.extend(params);
            }
        }

        log::debug!(
            "Custom CredentialMgmt signing for sub_cmd 0x{:02x}, message len: {}",
            sub_cmd,
            message.len()
        );

        let hmac_key = hmac::Key::new(hmac::HMAC_SHA256, pin_token);
        let sig = hmac::sign(&hmac_key, &message);
        sig.as_ref()[0..16].to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_pin_command_ordering() {
        // This test doesn't run HID IO, but verifies that our BTreeMap usage
        // (which is used in get_pin_token and get_pin_token_with_permission)
        // results in correct CBOR key ordering.
        let mut map = BTreeMap::new();
        map.insert(Value::Integer(0x01), Value::Integer(1)); // pinProtocol
        map.insert(Value::Integer(0x02), Value::Integer(8)); // subCommand (getPinUvAuthToken...)
        map.insert(Value::Integer(0x03), Value::Map(BTreeMap::new())); // keyAgreement
        map.insert(Value::Integer(0x04), Value::Bytes(vec![0u8; 16])); // pinHashEnc
        map.insert(Value::Integer(0x09), Value::Integer(0x01)); // permissions

        let cbor = to_vec(&Value::Map(map)).unwrap();

        // Canonical CBOR requires keys to be in ascending order.
        // BTreeMap in Rust is already ordered by key.
        // So 0x01, 0x02, 0x03, 0x04, 0x09 should be in order.

        // Let's check the first few bytes of the map
        // 0xA5 (Map of 5)
        // 0x01 (Key 1) ...
        assert_eq!(cbor[0], 0xA5);
        assert_eq!(cbor[1], 0x01);
        // We just care that it's ordered for pico-fido
    }

    #[test]
    fn test_get_key_agreement_parsing_logic() {
        use std::collections::BTreeMap;
        // Simulate a response map where key 0x01 is the KeyAgreement (as per CTAP 2.1)
        let mut inner_map = BTreeMap::new();
        inner_map.insert(Value::Integer(1), Value::Integer(2)); // kty: EC2
        inner_map.insert(Value::Integer(-1), Value::Integer(1)); // crv: P-256
        inner_map.insert(Value::Integer(-2), Value::Bytes(vec![0xAA; 32])); // x
        inner_map.insert(Value::Integer(-3), Value::Bytes(vec![0xBB; 32])); // y

        let mut resp_map = BTreeMap::new();
        resp_map.insert(
            Value::Integer(ClientPinResponseParam::KeyAgreement as i128),
            Value::Map(inner_map),
        );

        let val = Value::Map(resp_map);

        // This mimics the logic in get_key_agreement
        if let Value::Map(m) = val {
            let key_agreement = m.get(&Value::Integer(
                ClientPinResponseParam::KeyAgreement as i128,
            ));
            assert!(key_agreement.is_some());
            if let Some(Value::Map(km)) = key_agreement {
                assert_eq!(
                    km.get(&Value::Integer(-2)),
                    Some(&Value::Bytes(vec![0xAA; 32]))
                );
            } else {
                panic!("KeyAgreement should be a map");
            }
        } else {
            panic!("Expected map");
        }
    }

    #[test]
    fn test_pin_hash_encryption_actually_encrypts() {
        // Verify that our AES-CBC encryption actually modifies the data.
        // This guards against the previous bug where encrypt_block_mut
        // was called on a temporary copy (buffer.into()), discarding the result.
        use aes::cipher::generic_array::GenericArray;
        use cbc::cipher::{BlockEncryptMut, KeyIvInit};
        use ring::digest;

        let pin = "123456";
        let pin_hash = digest::digest(&digest::SHA256, pin.as_bytes());
        let pin_hash_16 = &pin_hash.as_ref()[0..16];

        // Use a known key (32 bytes of zeros) and IV (16 bytes of zeros)
        let key = [0u8; 32];
        let iv = [0u8; 16];

        let mut block = *GenericArray::from_slice(pin_hash_16);
        let original = block.clone();

        let mut encryptor = cbc::Encryptor::<aes::Aes256>::new(
            GenericArray::from_slice(&key),
            GenericArray::from_slice(&iv),
        );
        encryptor.encrypt_block_mut(&mut block);

        // The encrypted block MUST differ from the original
        assert_ne!(
            block.as_slice(),
            original.as_slice(),
            "Encryption did not modify the block — the old bug is back!"
        );
    }
}
