use rand::Rng;
use serde_cbor_2::{Value, to_vec};
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use crate::error::PFError;
use crate::fido::constants::*;

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

pub struct HidTransport {
	device: hidapi::HidDevice,
	cid: u32,
	pub vid: u16,
	pub pid: u16,
	pub product_name: String,
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
				e
			)));
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
					e
				)));
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

		// 1. Read First Packet (Loop to handle Keepalives)
		loop {
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
				buf[5]
			)));
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
		sub_params_inner.insert(
			Value::Integer(0x01),
			Value::Integer(vendor_cmd.to_u64() as i128),
		);

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

		// Send via HID
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

		// Send via HID
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
		use ring::hmac;
		let hmac_key = hmac::Key::new(hmac::HMAC_SHA256, pin_token);
		let sig = hmac::sign(&hmac_key, &message);
		sig.as_ref()[0..16].to_vec()
	}
}
