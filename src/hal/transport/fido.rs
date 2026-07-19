//! USB HID transport for CTAP2/FIDO2 communication.
//!
//! # What is HID?
//!
//! USB HID (Human Interface Device) is a standard USB device class for input
//! devices like keyboards, mice, and gamepads. HID devices communicate through
//! *reports* — fixed-size packets sent/received on USB endpoints. The OS
//! auto-detects HID devices without requiring custom drivers, making it ideal
//! for FIDO2 security keys that need to work across platforms.
//!
//! # What is CTAPHID?
//!
//! CTAPHID is the [CTAP2] transport binding for USB HID. It layers the CTAP2
//! protocol on top of HID reports, allowing FIDO2 authenticators to
//! communicate with hosts through the standard HID driver stack. The
//! specification is defined in [CTAP2 §11.2](https://fidoalliance.org/specs/fido-v2.3-ps-20260226/fido-client-to-authenticator-protocol-v2.3-ps-20260226.html#usb-human-interface-device-hid).
//!
//! # Framing protocol
//!
//! CTAPHID uses 64-byte HID reports. Messages that exceed 64 bytes are split
//! across multiple packets:
//!
//! ```text
//! Init Packet (64 bytes):
//!   CID(4) | CMD(1) | BCNT_HI(1) | BCNT_LO(1) | payload[..57]
//!
//! Continuation Packets:
//!   CID(4) | SEQ(1) | payload[..59]
//! ```
//!
//! - **CID** (Channel ID): 4-byte identifier negotiated via `CTAPHID_INIT`.
//!   Multiplexes multiple logical channels on one HID device.
//! - **CMD**: Command byte (e.g., `0x90` for CBOR, `0x86` for INIT).
//! - **BCNT**: 16-bit big-endian payload length.
//! - **SEQ**: Sequence number for continuation packets (starts at 0).
//!
//! # Channel initialization
//!
//! Before any CTAP2 command can be sent, the host must negotiate a Channel ID:
//!
//! 1. Host sends `CTAPHID_INIT` to the broadcast CID (`0xFFFFFFFF`) with a
//!    random 8-byte nonce.
//! 2. Device responds with the same nonce and a newly allocated CID.
//! 3. All subsequent communication uses this CID.
//!
//! This allows multiple CTAP2 sessions to coexist on one device (e.g., two
//! browsers open simultaneously).
//!
//! # Cryptographic operations
//!
//! PIN operations require ECDH key agreement and AES-256-CBC encryption:
//!
//! ```text
//! 1. Host → Device: GetKeyAgreement (returns device's P-256 public key)
//! 2. Host generates ephemeral P-256 key pair
//! 3. Host computes ECDH shared secret → SHA-256(shared_secret)
//! 4. PIN hash encrypted with AES-256-CBC (key = shared_secret, IV = 0)
//! 5. Token decrypted with same key
//! ```
//!
//! The shared secret is derived as `SHA-256(ECDH_x_coordinate)`.
//!
//! # Firmware compatibility
//!
//! Both [pico-fido] and [RS-Key] implement CTAPHID. This module handles:
//! - Standard CTAP2 commands (GetInfo, MakeCredential, GetAssertion, etc.)
//! - Pico-fido vendor commands (`0xC1`, `0xC2`) for hardware config
//! - RS-Key vendor command (`0x41`) for seed backup and attestation
//!
//! # File structure
//!
//! - [`HidTransport`] — main transport struct; opens HID device, negotiates
//!   CID, sends/receives CBOR payloads
//! - [`EnumerateRpResponse`](crate::hal::fido::ops::EnumerateRpResponse),
//!   [`EnumerateCredentialResponse`](crate::hal::fido::ops::EnumerateCredentialResponse) — response
//!   types for credential management enumeration
//! - PIN methods (`get_pin_token`, `set_pin`, `change_pin`) implement the
//!   full ECDH + AES-CBC flow per CTAP2 §11.5.4
//! - Vendor methods (`send_vendor_config`, `get_enterprise_attestation_csr`)
//!   handle pico-fido/RS-Key specific extensions
//!
//! [CTAP2]: https://fidoalliance.org/specs/fido-v2.3-ps-20260226/fido-client-to-authenticator-protocol-v2.3-ps-20260226.html
//! [pico-fido]: https://github.com/polhenarejos/pico-fido
//! [RS-Key]: https://github.com/TheMaxMur/RS-Key

use rand::RngExt;
use std::time::Duration;

use crate::error::PFError;

/// Size of a single USB HID report in bytes (CTAP2 §11.2 mandates 64-byte reports).
const HID_REPORT_SIZE: usize = 64;

/// FIDO Alliance HID Usage Page identifier.
///
/// Devices advertising this usage page in their HID descriptor are identified
/// as FIDO authenticators by the operating system's HID enumeration.
const HID_USAGE_PAGE_FIDO: u16 = 0xF1D0;

/// Broadcast Channel ID used for the initial CTAPHID_INIT handshake.
///
/// The host sends an INIT command to this CID to request a unique Channel ID
/// from the authenticator. All subsequent communication uses the negotiated CID.
const CTAPHID_CID_BROADCAST: u32 = 0xFFFFFFFF;

/// CTAPHID INIT command byte (0x86).
///
/// Initiates channel negotiation. The host sends a random 8-byte nonce; the
/// device responds with the same nonce and a newly allocated Channel ID.
const CTAPHID_INIT: u8 = 0x86;

/// CTAPHID CBOR command byte (0x90).
///
/// Wraps a CTAP2 CBOR-encoded command or response payload. The payload is
/// fragmented across one init packet and zero or more continuation packets.
pub const CTAPHID_CBOR: u8 = 0x90;

/// CTAPHID ERROR response byte (0xBF).
///
/// Indicates the authenticator encountered an error processing the command.
/// The next byte contains the CTAP2 error code.
const CTAPHID_ERROR: u8 = 0xBF;

/// CTAPHID KEEPALIVE status byte (0xBB).
///
/// Sent by the authenticator while processing a long-running operation (e.g.,
/// MakeCredential with user interaction). The host must continue reading
/// until it receives the final CBOR or ERROR response.
const CTAPHID_KEEPALIVE: u8 = 0xBB;

/// Default timeout in milliseconds for draining stale HID packets.
const HID_READ_TIMEOUT_MS: i32 = 10;

/// Timeout in milliseconds for reading the CTAPHID_INIT response during channel negotiation.
const HID_INIT_READ_TIMEOUT_MS: i32 = 100;

/// Timeout in milliseconds for reading a single HID response packet (excluding keepalives).
const HID_RESP_READ_TIMEOUT_MS: i32 = 2000;

/// Timeout in milliseconds for reading CTAPHID continuation packets.
const HID_CONT_READ_TIMEOUT_MS: i32 = 500;

/// Maximum total time in milliseconds allowed for a complete CBOR command/response exchange.
const HID_TOTAL_TIMEOUT_MS: i32 = 5000;

/// USB HID transport for CTAP2/FIDO2 communication.
///
/// Wraps a `hidapi::HidDevice` and manages the CTAPHID framing layer:
/// channel negotiation (INIT), multi-packet CBOR send/receive, keepalive
/// handling, and all higher-level CTAP2 operations (PIN, credential management,
/// vendor commands).
///
/// Created via [`HidTransport::open`], which scans for a device with the FIDO
/// HID Usage Page (0xF1D0) and performs the INIT handshake to obtain a Channel ID.
#[derive(Debug)]
pub struct HidTransport {
    device: hidapi::HidDevice,
    cid: u32,
    pub vid: u16,
    pub pid: u16,
    pub product_name: String,
}

impl HidTransport {
    /// Open the first available FIDO HID device and negotiate a Channel ID.
    ///
    /// Scans for a device with HID Usage Page `0xF1D0`, opens it, and performs
    /// the CTAPHID_INIT handshake. Returns an error if no device is found or
    /// the INIT handshake times out.
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

    /// Cheap, non-intrusive presence fingerprint of the attached FIDO HID device.
    ///
    /// Returns `vid:pid:serial` (serial may be empty) for the first device with
    /// the FIDO usage page, or `None` when none is present. It only *enumerates*
    /// USB descriptors — it does not open the device or run `CTAPHID_INIT`, so it
    /// is safe to poll on a timer even while another handle holds the device open.
    /// A change in the returned value signals a plug / unplug / swap.
    pub fn fingerprint() -> Option<String> {
        let api = hidapi::HidApi::new().ok()?;
        let info = api
            .device_list()
            .find(|d| d.usage_page() == HID_USAGE_PAGE_FIDO)?;
        Some(format!(
            "{:04x}:{:04x}:{}",
            info.vendor_id(),
            info.product_id(),
            info.serial_number().unwrap_or("")
        ))
    }

    /// Negotiate a CTAPHID Channel ID via CTAPHID_INIT.
    ///
    /// Sends an INIT command to the broadcast CID (`0xFFFFFFFF`) with a random
    /// 8-byte nonce, then reads the response to extract the allocated CID.
    /// Drains any stale packets before the handshake to avoid confusion.
    fn init_channel(device: &hidapi::HidDevice) -> Result<u32, PFError> {
        log::debug!("Initializing CTAPHID channel...");

        let mut stale_packet_buffer = [0u8; HID_REPORT_SIZE];
        while let Ok(n) = device.read_timeout(&mut stale_packet_buffer[..], HID_READ_TIMEOUT_MS) {
            if n == 0 {
                break;
            }
            log::trace!(
                "Drained stale HID packet: {:02X?}",
                &stale_packet_buffer[0..16]
            );
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
            let mut init_buf = [0u8; HID_REPORT_SIZE];
            if device
                .read_timeout(&mut init_buf[..], HID_INIT_READ_TIMEOUT_MS)
                .is_ok()
            {
                // Check if response matches our broadcast and nonce
                if init_buf[0..4] == CTAPHID_CID_BROADCAST.to_be_bytes()
                    && init_buf[4] == CTAPHID_INIT
                    && init_buf[7..15] == nonce
                {
                    // New CID is at bytes 16..20
                    let new_cid = u32::from_be_bytes([
                        init_buf[15],
                        init_buf[16],
                        init_buf[17],
                        init_buf[18],
                    ]);
                    log::debug!("Channel negotiation successful. New CID: 0x{:08X}", new_cid);
                    return Ok(new_cid);
                } else {
                    log::trace!(
                        "Received ignoreable HID packet during CID negotiation: {:02X?}",
                        &init_buf[0..16]
                    );
                }
            }
        }
        log::error!("Timeout waiting for CTAPHID_INIT response.");
        Err(PFError::Device(
            "Timeout waiting for FIDO Init response".into(),
        ))
    }

    /// Send a CTAP2 CBOR command and wait for the response using the default timeout.
    ///
    /// Convenience wrapper around [`send_cbor_with_timeout`](HidTransport::send_cbor_with_timeout).
    pub fn send_cbor(&self, cmd: u8, payload: &[u8]) -> Result<Vec<u8>, PFError> {
        self.send_cbor_with_timeout(cmd, payload, HID_TOTAL_TIMEOUT_MS)
    }

    /// Send a CTAP2 CBOR command and wait for the response with a custom timeout.
    ///
    /// Fragments `payload` into CTAPHID init + continuation packets, then reads
    /// and reassembles the response. The `timeout_ms` parameter overrides the
    /// default for the read phase (useful for operations that require user interaction).
    pub fn send_cbor_with_timeout(
        &self,
        cmd: u8,
        payload: &[u8],
        timeout_ms: i32,
    ) -> Result<Vec<u8>, PFError> {
        self.write_cbor_request(cmd, payload)?;
        self.read_cbor_response(cmd, timeout_ms)
    }

    /// Send a CTAP2 CBOR command and return the raw HID response without status-byte parsing.
    ///
    /// Unlike [`send_cbor`](HidTransport::send_cbor), this does not check the CTAP status byte
    /// or strip it from the response. Useful for vendor commands that return non-standard payloads.
    pub fn send_raw(&self, cmd: u8, payload: &[u8]) -> Result<Vec<u8>, PFError> {
        self.write_cbor_request(cmd, payload)?;
        self.read_hid_response(cmd, HID_TOTAL_TIMEOUT_MS)
    }

    /// Send the CTAP authenticatorReset command (0x07).
    ///
    /// Resets the authenticator to its factory state: all credentials, PINs,
    /// and configuration are erased. Uses a 30-second timeout to allow for
    /// any required user interaction (e.g., touch confirmation).
    pub fn reset(&self) -> Result<(), PFError> {
        log::info!("Sending CTAP authenticatorReset (0x07)...");
        self.write_cbor_request(CTAPHID_CBOR, &[0x07])?;
        self.read_cbor_response(CTAPHID_CBOR, 30_000)?;
        Ok(())
    }

    /// Fragment and write a CTAPHID request to the device.
    ///
    /// Encodes the command byte and payload into a CTAPHID init packet followed
    /// by zero or more continuation packets, then writes each 65-byte HID report
    /// (1 byte Report ID + 64 bytes payload) to the device.
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

    /// Read a CTAPHID response and verify the CTAP status byte.
    ///
    /// Delegates to [`read_hid_response`](HidTransport::read_hid_response) for packet
    /// reassembly, then checks the first byte for a non-zero CTAP status code and
    /// strips it before returning the payload.
    fn read_cbor_response(&self, cmd: u8, timeout_ms: i32) -> Result<Vec<u8>, PFError> {
        let response_data = self.read_hid_response(cmd, timeout_ms)?;

        // Check CTAP Status Byte (First byte of payload)
        if response_data.is_empty() {
            log::error!("Device sent empty payload response.");
            return Err(PFError::Device("Empty response".into()));
        }
        let ctap_status_byte = response_data[0];
        if ctap_status_byte != 0x00 {
            log::error!(
                "FIDO Operation returned failure status: 0x{:02X}",
                ctap_status_byte
            );
            return Err(PFError::Device(format!(
                "FIDO Operation Failed with Status: 0x{:02X}",
                ctap_status_byte
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

    /// Read and reassemble a CTAPHID response from the device.
    ///
    /// Handles the full CTAPHID receive flow:
    /// 1. Reads the init packet while skipping KEEPALIVE and mismatched-CID packets.
    /// 2. Validates the command byte matches the expected response.
    /// 3. Reads continuation packets in sequence order until the full payload is received.
    /// 4. Enforces the `timeout_ms` deadline across the entire read.
    fn read_hid_response(&self, cmd: u8, timeout_ms: i32) -> Result<Vec<u8>, PFError> {
        log::debug!("Waiting for response...");

        let mut packet_buf = [0u8; HID_REPORT_SIZE];
        let mut response_data = Vec::new();
        let expected_len: usize;
        let mut read_len = 0;
        let mut last_seq = 0;

        let deadline_start = std::time::Instant::now();
        let timeout_duration = std::time::Duration::from_millis(timeout_ms as u64);

        // 1. Read First Packet (Keepalive Loop)
        loop {
            if deadline_start.elapsed() > timeout_duration {
                log::error!("Timeout waiting for device response (Keepalive limit exceeded)");
                return Err(PFError::Device(
                    "Timeout waiting for device response (Keepalive limit exceeded)".into(),
                ));
            }

            if let Err(e) = self
                .device
                .read_timeout(&mut packet_buf[..], HID_RESP_READ_TIMEOUT_MS)
            {
                log::error!("Timeout reading response packet: {}", e);
                return Err(PFError::Io(format!(
                    "Timeout reading response packet: {}",
                    e
                )));
            }

            // Check CID mismatch
            if u32::from_be_bytes([packet_buf[0], packet_buf[1], packet_buf[2], packet_buf[3]])
                != self.cid
            {
                log::warn!("Received packet from different CID, ignoring...");
                continue;
            }

            // Check for KEEPALIVE (0xBB)
            if packet_buf[4] == CTAPHID_KEEPALIVE {
                let keepalive_status = packet_buf[5];
                log::debug!(
                    "Device sent KEEPALIVE (Status: 0x{:02X}), waiting...",
                    keepalive_status
                );
                continue;
            }

            // If we are here, it's a real response
            break;
        }

        if packet_buf[4] == CTAPHID_ERROR {
            log::error!("Device returned CTAP Error code: 0x{:02X}", packet_buf[5]);
            return Err(PFError::Device(format!(
                "Device returned CTAP Error: 0x{:02X}",
                packet_buf[5],
            )));
        } else {
            log::trace!("Packet received is not a CTAP Error");
        }

        if packet_buf[4] == cmd {
            expected_len = u16::from_be_bytes([packet_buf[5], packet_buf[6]]) as usize;
            let in_pkt = std::cmp::min(expected_len, HID_REPORT_SIZE - 7);
            response_data.extend_from_slice(&packet_buf[7..7 + in_pkt]);
            read_len += in_pkt;
            // log::trace!("Received Init Response. Expecting {} bytes total.", expected_len);
        } else {
            log::error!(
                "Unexpected command response: 0x{:02X} (Expected 0x{:02X})",
                packet_buf[4],
                cmd
            );
            return Err(PFError::Device(format!(
                "Unexpected command response: 0x{:02X} (Expected 0x{:02X})",
                packet_buf[4], cmd
            )));
        }

        // 2. Read Continuation Packets
        while read_len < expected_len {
            if let Err(e) = self
                .device
                .read_timeout(&mut packet_buf[..], HID_CONT_READ_TIMEOUT_MS)
            {
                log::error!("Timeout reading continuation packet: {}", e);
                return Err(PFError::Io(format!(
                    "Timeout reading continuation packet: {}",
                    e
                )));
            }

            if u32::from_be_bytes([packet_buf[0], packet_buf[1], packet_buf[2], packet_buf[3]])
                != self.cid
            {
                continue; // Ignore packets from other channels
            }

            let seq = packet_buf[4];
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
            response_data.extend_from_slice(&packet_buf[5..5 + in_pkt]);
            read_len += in_pkt;
        }

        Ok(response_data)
    }
}
