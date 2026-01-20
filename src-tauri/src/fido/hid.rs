#![allow(unused)]

use anyhow::{Result, anyhow};
use rand::Rng;
use std::time::Duration;

// HID Transport Constants
const HID_REPORT_SIZE: usize = 64;
const HID_USAGE_PAGE_FIDO: u16 = 0xF1D0;
const CTAPHID_CID_BROADCAST: u32 = 0xFFFFFFFF;
const CTAPHID_INIT: u8 = 0x86;
pub const CTAPHID_CBOR: u8 = 0x90;
const CTAPHID_ERROR: u8 = 0xBF;
const CTAPHID_KEEPALIVE: u8 = 0xBB;

pub struct HidTransport {
	device: hidapi::HidDevice,
	cid: u32,
	pub vid: u16,
	pub pid: u16,
	pub product_name: String,
}

impl HidTransport {
	pub fn open() -> Result<Self> {
		log::info!("Attempting to open HID transport for FIDO device...");
		let api = hidapi::HidApi::new().map_err(|e| {
			log::error!("Failed to initialize HidApi: {}", e);
			e
		})?;

		// Find device with FIDO Usage Page (0xF1D0)
		let info = api
			.device_list()
			.find(|d| d.usage_page() == HID_USAGE_PAGE_FIDO)
			.ok_or_else(|| {
				log::warn!("No FIDO device found with Usage Page 0xF1D0.");
				anyhow!("No FIDO device found. Is it plugged in?")
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
			e
		})?;

		// Negotiate Channel ID (CID)
		let cid = Self::init_channel(&device).map_err(|e| {
			log::error!("Failed to negotiate Channel ID: {}", e);
			e
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

	fn init_channel(device: &hidapi::HidDevice) -> Result<u32> {
		log::debug!("Initializing CTAPHID channel...");
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
		device.write(&report).map_err(|e| {
			log::error!("Failed to write INIT packet: {}", e);
			e
		})?;

		// Read Response until we find our nonce
		let start = std::time::Instant::now();
		while start.elapsed() < Duration::from_secs(1) {
			let mut buf = [0u8; HID_REPORT_SIZE];
			if device.read_timeout(&mut buf, 100).is_ok() {
				// Check if response matches our broadcast and nonce
				if buf[0..4] == CTAPHID_CID_BROADCAST.to_be_bytes()
					&& buf[4] == CTAPHID_INIT
					&& &buf[7..15] == &nonce
				{
					// New CID is at bytes 16..20
					let new_cid = u32::from_be_bytes([buf[15], buf[16], buf[17], buf[18]]);
					log::debug!("Channel negotiation successful. New CID: 0x{:08X}", new_cid);
					return Ok(new_cid);
				}
			}
		}
		log::error!("Timeout waiting for CTAPHID_INIT response.");
		Err(anyhow!("Timeout waiting for FIDO Init response"))
	}

	pub fn send_cbor(&self, cmd: u8, payload: &[u8]) -> Result<Vec<u8>> {
		log::debug!(
			"Sending CBOR Command: 0x{:02X}, Payload Size: {} bytes",
			cmd,
			payload.len()
		);

		// --- Transmit ---
		let mut sequence = 0u8;
		let total_len = payload.len();
		let mut sent = 0;

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
		if let Err(e) = self.device.write(&report) {
			log::error!("Failed to write initial HID packet: {}", e);
			return Err(e.into());
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
			if let Err(e) = self.device.write(&report) {
				log::error!(
					"Failed to write continuation HID packet (Seq {}): {}",
					sequence - 1,
					e
				);
				return Err(e.into());
			}
		}

		// --- Receive ---
		let mut response_data = Vec::new();
		let mut expected_len = 0;
		let mut read_len = 0;
		let mut last_seq = 0;

		log::debug!("Waiting for response...");

		// Read First Packet (Loop to handle Keepalives)
		let mut buf = [0u8; HID_REPORT_SIZE];

		let mut buf = [0u8; HID_REPORT_SIZE];
		loop {
			if let Err(e) = self.device.read_timeout(&mut buf, 2000) {
				log::error!("Timeout reading response packet: {}", e);
				return Err(e.into());
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
			return Err(anyhow!("Device returned CTAP Error: 0x{:02X}", buf[5]));
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
			return Err(anyhow!(
				"Unexpected command response: 0x{:02X} (Expected 0x{:02X})",
				buf[4],
				cmd
			));
		}

		// 2. Read Continuation Packets
		while read_len < expected_len {
			if let Err(e) = self.device.read_timeout(&mut buf, 500) {
				log::error!("Timeout reading continuation packet: {}", e);
				return Err(e.into());
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
				return Err(anyhow!("Sequence mismatch"));
			}
			last_seq += 1;

			let in_pkt = std::cmp::min(expected_len - read_len, HID_REPORT_SIZE - 5);
			response_data.extend_from_slice(&buf[5..5 + in_pkt]);
			read_len += in_pkt;
		}

		// 3. Check CTAP Status Byte (First byte of payload)
		if response_data.is_empty() {
			log::error!("Device sent empty payload response.");
			return Err(anyhow!("Empty response"));
		}
		let status = response_data[0];
		if status != 0x00 {
			log::error!("FIDO Operation returned failure status: 0x{:02X}", status);
			return Err(anyhow!(
				"FIDO Operation Failed with Status: 0x{:02X}",
				status
			));
		}

		log::debug!(
			"Command 0x{:02X} successful. Response payload len: {}",
			cmd,
			response_data.len() - 1
		);
		// Return payload without status byte
		Ok(response_data[1..].to_vec())
	}
}
