//! Yubico OTP applet client for configurable slots exposed by the firmware
//! profile (currently four on both PicoFido and RS-Key profiles).
//!
//! Speaks the standard YubiKey slot protocol over CCID (single INS `0x01`,
//! P1 selects the operation). Slot config records are the classic 52-byte packed
//! frame with a trailing CRC-16 (residual `0xF0B8`). The device stores the
//! secrets; the host builds the frame and reads status.
//!
//! Frame builders cover challenge-response (HMAC-SHA1), OATH-HOTP,
//! static-password, and Yubico-OTP programming. The corresponding write
//! operations are wired through the HAL and Slots screen; status, delete, and
//! swap operations are also available. Firmware capability profiles determine
//! which controls the UI exposes.

// Keep protocol helpers available even when an individual helper has no direct UI call site.
#![allow(dead_code)]

use crate::error::PFError;
use crate::hal::apdu::{Apdu, CLA_ISO, tlv};
use crate::hal::transport::ccid::CcidSession;
use ring::rand::{SecureRandom, SystemRandom};

/// Yubico OTP applet AID.
pub const OTP_AID: &[u8] = &[0xA0, 0x00, 0x00, 0x05, 0x27, 0x20, 0x01];

const INS_OTP: u8 = 0x01;

// Slot-command P1 codes.
const P1_CONFIG_SLOT1: u8 = 0x01;
const P1_CONFIG_SLOT2: u8 = 0x03;
const P1_SWAP: u8 = 0x06;
const P1_STATUS_EXT: u8 = 0x14;
const P1_CHAL_HMAC_SLOT1: u8 = 0x30;
const P1_CHAL_HMAC_SLOT2: u8 = 0x38;

/// HMAC-SHA1 challenge-response frame size. The slot takes a fixed 64-byte
/// challenge; the firmware rejects a shorter one with `6700`.
const CHALLENGE_FRAME: usize = 64;

// 52-byte config frame offsets.
const OFF_UID: usize = 16;
const OFF_AES_KEY: usize = 22;
const OFF_ACC_CODE: usize = 38;
const OFF_FIXED_SIZE: usize = 44;
const OFF_EXT_FLAGS: usize = 45;
const OFF_TKT_FLAGS: usize = 46;
const OFF_CFG_FLAGS: usize = 47;
const CONFIG_SIZE: usize = 52;
const ACC_CODE_SIZE: usize = 6;
const SECRET_LEN: usize = 20;

// Ticket (TKT) flags.
const TKT_OATH_HOTP: u8 = 0x40;
const TKT_CHAL_RESP: u8 = 0x40;
const TKT_APPEND_CR: u8 = 0x20;

// Config (CFG) flags.
const CFG_SHORT_TICKET: u8 = 0x02;
const CFG_OATH_HOTP8: u8 = 0x02;
const CFG_HMAC_LT64: u8 = 0x04;
const CFG_CHAL_BTN_TRIG: u8 = 0x08;
const CFG_STATIC_TICKET: u8 = 0x20;
const CFG_CHAL_YUBICO: u8 = 0x20;
const CFG_CHAL_HMAC: u8 = 0x22;

/// What a programmed slot holds (best-effort classification from its flags).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotType {
    Empty,
    YubicoOtp,
    StaticPassword,
    OathHotp,
    ChallengeResponse,
}

impl SlotType {
    pub fn label(self) -> &'static str {
        match self {
            Self::Empty => "Empty",
            Self::YubicoOtp => "Yubico OTP",
            Self::StaticPassword => "Static password",
            Self::OathHotp => "OATH-HOTP",
            Self::ChallengeResponse => "Challenge-response",
        }
    }
}

/// Status of one slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlotInfo {
    /// 1-based slot number (1/2 classic, 3/4 = RS-Key extension).
    pub slot: u8,
    pub kind: SlotType,
    pub touch: bool,
}

impl SlotInfo {
    fn empty(slot: u8) -> Self {
        Self {
            slot,
            kind: SlotType::Empty,
            touch: false,
        }
    }
    pub fn configured(&self) -> bool {
        self.kind != SlotType::Empty
    }
}

fn classify(tkt: u8, cfg: u8) -> SlotType {
    if tkt & TKT_CHAL_RESP != 0 {
        if cfg & CFG_CHAL_YUBICO != 0 {
            SlotType::ChallengeResponse
        } else {
            SlotType::OathHotp
        }
    } else if cfg & (CFG_STATIC_TICKET | CFG_SHORT_TICKET) != 0 {
        SlotType::StaticPassword
    } else {
        SlotType::YubicoOtp
    }
}

/// Slot 1-based number → the `(P1, P2)` addressing a config/delete command uses.
/// Slot 1 = `01/0`, slot 2 = `03/0`, slots 3/4 = `01` with a P2 offset.
fn config_p1p2(slot: u8) -> (u8, u8) {
    match slot {
        2 => (P1_CONFIG_SLOT2, 0),
        3 => (P1_CONFIG_SLOT1, 2),
        4 => (P1_CONFIG_SLOT1, 3),
        _ => (P1_CONFIG_SLOT1, 0),
    }
}

/// Slot → `(P1, P2)` for an HMAC challenge-response command.
fn chal_p1p2(slot: u8) -> (u8, u8) {
    match slot {
        2 => (P1_CHAL_HMAC_SLOT2, 0),
        3 => (P1_CHAL_HMAC_SLOT1, 2),
        4 => (P1_CHAL_HMAC_SLOT1, 3),
        _ => (P1_CHAL_HMAC_SLOT1, 0),
    }
}

/// CRC-16 (X.25, reflected poly 0x8408, init 0xFFFF). A valid config CRCs the
/// whole 52 bytes to the residual 0xF0B8.
fn crc16(data: &[u8]) -> u16 {
    let mut crc = 0xFFFFu16;
    for &b in data {
        crc ^= b as u16;
        for _ in 0..8 {
            let lsb = crc & 1;
            crc >>= 1;
            if lsb != 0 {
                crc ^= 0x8408;
            }
        }
    }
    crc
}

#[allow(clippy::too_many_arguments)]
fn build_config(
    fixed: &[u8; 16],
    uid: &[u8; 6],
    key: &[u8; 16],
    acc: &[u8; 6],
    fixed_size: u8,
    tkt: u8,
    cfg: u8,
) -> [u8; CONFIG_SIZE] {
    let mut c = [0u8; CONFIG_SIZE];
    c[..16].copy_from_slice(fixed);
    c[OFF_UID..OFF_UID + 6].copy_from_slice(uid);
    c[OFF_AES_KEY..OFF_AES_KEY + 16].copy_from_slice(key);
    c[OFF_ACC_CODE..OFF_ACC_CODE + 6].copy_from_slice(acc);
    c[OFF_FIXED_SIZE] = fixed_size;
    c[OFF_TKT_FLAGS] = tkt;
    c[OFF_CFG_FLAGS] = cfg;
    // Stored CRC = ~crc16(first 50 bytes), little-endian (Yubico convention).
    let crc = !crc16(&c[..CONFIG_SIZE - 2]);
    c[CONFIG_SIZE - 2..].copy_from_slice(&crc.to_le_bytes());
    c
}

/// Split a secret (≤20 bytes) into the AES-key + UID fields the way ykman does.
fn key_uid(secret: &[u8]) -> ([u8; 16], [u8; 6]) {
    let mut key = [0u8; 16];
    let mut uid = [0u8; 6];
    let n = secret.len().min(SECRET_LEN);
    let kn = n.min(16);
    key[..kn].copy_from_slice(&secret[..kn]);
    if n > 16 {
        uid[..n - 16].copy_from_slice(&secret[16..n]);
    }
    (key, uid)
}

/// Build an HMAC-SHA1 challenge-response config (variable-length challenges).
pub fn build_chalresp(secret: &[u8], touch: bool, new_acc: &[u8; 6]) -> [u8; CONFIG_SIZE] {
    let (key, uid) = key_uid(secret);
    let mut cfg = CFG_CHAL_HMAC | CFG_HMAC_LT64;
    if touch {
        cfg |= CFG_CHAL_BTN_TRIG;
    }
    build_config(&[0u8; 16], &uid, &key, new_acc, 0, TKT_CHAL_RESP, cfg)
}

/// Build an OATH-HOTP config (6 or 8 digits, optional trailing CR).
pub fn build_hotp(
    secret: &[u8],
    digits8: bool,
    append_cr: bool,
    new_acc: &[u8; 6],
) -> [u8; CONFIG_SIZE] {
    let (key, uid) = key_uid(secret);
    let mut tkt = TKT_OATH_HOTP;
    if append_cr {
        tkt |= TKT_APPEND_CR;
    }
    let cfg = if digits8 { CFG_OATH_HOTP8 } else { 0 };
    build_config(&[0u8; 16], &uid, &key, new_acc, 0, tkt, cfg)
}

/// Build a static-password config: the given HID scancodes are typed verbatim.
pub fn build_static(scancodes: &[u8], append_cr: bool, new_acc: &[u8; 6]) -> [u8; CONFIG_SIZE] {
    let mut buf = [0u8; 38];
    let n = scancodes.len().min(38);
    buf[..n].copy_from_slice(&scancodes[..n]);
    let mut fixed = [0u8; 16];
    fixed.copy_from_slice(&buf[..16]);
    let mut uid = [0u8; 6];
    uid.copy_from_slice(&buf[16..22]);
    let mut key = [0u8; 16];
    key.copy_from_slice(&buf[22..38]);
    let mut tkt = 0;
    if append_cr {
        tkt |= TKT_APPEND_CR;
    }
    build_config(&fixed, &uid, &key, new_acc, n as u8, tkt, CFG_STATIC_TICKET)
}

/// Build a Yubico-OTP config (public id ‖ private id ‖ AES key).
pub fn build_yubico_otp(
    public_id: &[u8],
    private_id: &[u8; 6],
    key: &[u8; 16],
    append_cr: bool,
    new_acc: &[u8; 6],
) -> [u8; CONFIG_SIZE] {
    let mut fixed = [0u8; 16];
    let n = public_id.len().min(16);
    fixed[..n].copy_from_slice(&public_id[..n]);
    let mut tkt = 0;
    if append_cr {
        tkt |= TKT_APPEND_CR;
    }
    build_config(&fixed, private_id, key, new_acc, n as u8, tkt, 0)
}

const MODHEX: &[u8; 16] = b"cbdefghijklnrtuv";

/// Decode a modhex string (Yubico's keyboard-safe hex) into bytes.
pub fn modhex_decode(s: &str) -> Option<Vec<u8>> {
    let s = s.trim();
    if s.is_empty() || !s.len().is_multiple_of(2) {
        return None;
    }
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(s.len() / 2);
    for pair in bytes.chunks(2) {
        let hi = MODHEX
            .iter()
            .position(|&c| c == pair[0].to_ascii_lowercase())? as u8;
        let lo = MODHEX
            .iter()
            .position(|&c| c == pair[1].to_ascii_lowercase())? as u8;
        out.push((hi << 4) | lo);
    }
    Some(out)
}

/// Encode bytes as modhex.
pub fn modhex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        s.push(MODHEX[(b >> 4) as usize] as char);
        s.push(MODHEX[(b & 0x0F) as usize] as char);
    }
    s
}

/// Map an ASCII character to its US-keyboard HID scancode (bit 0x80 = shift),
/// the byte format YubiKey static-password slots type verbatim.
fn scancode(ch: char) -> Option<u8> {
    const SHIFT: u8 = 0x80;
    Some(match ch {
        'a'..='z' => 0x04 + (ch as u8 - b'a'),
        'A'..='Z' => SHIFT | (0x04 + (ch as u8 - b'A')),
        '1'..='9' => 0x1E + (ch as u8 - b'1'),
        '0' => 0x27,
        ' ' => 0x2C,
        '-' => 0x2D,
        '=' => 0x2E,
        '[' => 0x2F,
        ']' => 0x30,
        '\\' => 0x31,
        ';' => 0x33,
        '\'' => 0x34,
        '`' => 0x35,
        ',' => 0x36,
        '.' => 0x37,
        '/' => 0x38,
        '!' => SHIFT | 0x1E,
        '@' => SHIFT | 0x1F,
        '#' => SHIFT | 0x20,
        '$' => SHIFT | 0x21,
        '%' => SHIFT | 0x22,
        '^' => SHIFT | 0x23,
        '&' => SHIFT | 0x24,
        '*' => SHIFT | 0x25,
        '(' => SHIFT | 0x26,
        ')' => SHIFT | 0x27,
        '_' => SHIFT | 0x2D,
        '+' => SHIFT | 0x2E,
        '{' => SHIFT | 0x2F,
        '}' => SHIFT | 0x30,
        '|' => SHIFT | 0x31,
        ':' => SHIFT | 0x33,
        '"' => SHIFT | 0x34,
        '~' => SHIFT | 0x35,
        '<' => SHIFT | 0x36,
        '>' => SHIFT | 0x37,
        '?' => SHIFT | 0x38,
        _ => return None,
    })
}

/// Convert an ASCII password to HID scancodes (≤38 chars), or `None` if it has
/// an unmappable character or is too long.
pub fn ascii_to_scancodes(s: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(s.len());
    for ch in s.chars() {
        out.push(scancode(ch)?);
    }
    if out.is_empty() || out.len() > 38 {
        return None;
    }
    Some(out)
}

/// A 20-byte cryptographically-random secret.
pub fn random_secret() -> Result<[u8; SECRET_LEN], PFError> {
    let mut s = [0u8; SECRET_LEN];
    SystemRandom::new()
        .fill(&mut s)
        .map_err(|_| PFError::Device("RNG failure".into()))?;
    Ok(s)
}

/// (public id, private id, AES key) for a Yubico OTP slot.
pub type YubicoKeyMaterial = ([u8; 6], [u8; 6], [u8; 16]);

/// Random (public id, private id, AES key) for a self-generated Yubico OTP slot.
pub fn random_yubico() -> Result<YubicoKeyMaterial, PFError> {
    let mut buf = [0u8; 28];
    SystemRandom::new()
        .fill(&mut buf)
        .map_err(|_| PFError::Device("RNG failure".into()))?;
    let mut public_id = [0u8; 6];
    let mut private_id = [0u8; 6];
    let mut key = [0u8; 16];
    public_id.copy_from_slice(&buf[..6]);
    private_id.copy_from_slice(&buf[6..12]);
    key.copy_from_slice(&buf[12..28]);
    Ok((public_id, private_id, key))
}

/// Open the OTP applet (SELECT).
pub fn open() -> Result<CcidSession, PFError> {
    CcidSession::open(OTP_AID)
}

/// Read per-slot status via EXTENDED STATUS (`0x14`). Always returns four
/// entries (slots 1-4); absent slots are `SlotType::Empty`.
pub fn read_info(session: &CcidSession) -> Result<[SlotInfo; 4], PFError> {
    let resp = session.transceive_full(&Apdu::read(CLA_ISO, INS_OTP, P1_STATUS_EXT, 0, &[]))?;
    let mut slots = [
        SlotInfo::empty(1),
        SlotInfo::empty(2),
        SlotInfo::empty(3),
        SlotInfo::empty(4),
    ];
    for (tag, value) in tlv::TlvIter::new(&resp) {
        if (0xB0..=0xB3).contains(&tag)
            && let Some(flags) = tlv::find(value, 0xA0)
            && flags.len() >= 2
        {
            let (tkt, cfg) = (flags[0], flags[1]);
            let idx = (tag - 0xB0) as usize;
            slots[idx] = SlotInfo {
                slot: idx as u8 + 1,
                kind: classify(tkt, cfg),
                touch: cfg & CFG_CHAL_BTN_TRIG != 0,
            };
        }
    }
    Ok(slots)
}

/// Write a slot config. `current_acc` is the slot's existing access code (zeros
/// when it is unprotected); a protected slot with the wrong code returns `6982`.
pub fn configure(
    session: &CcidSession,
    slot: u8,
    config: &[u8; CONFIG_SIZE],
    current_acc: &[u8; 6],
) -> Result<(), PFError> {
    let (p1, p2) = config_p1p2(slot);
    let mut body = config.to_vec();
    body.extend_from_slice(current_acc);
    session.transceive_full(&Apdu::read(CLA_ISO, INS_OTP, p1, p2, &body))?;
    Ok(())
}

/// Delete (zap) a slot — an all-zero config write.
pub fn delete_slot(session: &CcidSession, slot: u8, current_acc: &[u8; 6]) -> Result<(), PFError> {
    let (p1, p2) = config_p1p2(slot);
    let mut body = vec![0u8; CONFIG_SIZE];
    body.extend_from_slice(current_acc);
    session.transceive_full(&Apdu::read(CLA_ISO, INS_OTP, p1, p2, &body))?;
    Ok(())
}

/// Swap the contents of slots 1 and 2. An empty body swaps unprotected slots;
/// a `[0, 0, acc…]` body presents an access code.
pub fn swap(session: &CcidSession, current_acc: &[u8; 6]) -> Result<(), PFError> {
    let body = if current_acc.iter().all(|&b| b == 0) {
        Vec::new()
    } else {
        let mut b = vec![0u8, 0u8];
        b.extend_from_slice(current_acc);
        b
    };
    session.transceive_full(&Apdu::read(CLA_ISO, INS_OTP, P1_SWAP, 0, &body))?;
    Ok(())
}

/// Pad a variable-length challenge into the fixed 64-byte frame the slot
/// expects. A shorter challenge is filled with a byte that differs from its
/// last byte, because the firmware recovers the message by trimming trailing
/// bytes equal to the frame's final byte — so the pad must not match, or the
/// tail of the challenge would be trimmed away.
fn pad_challenge(challenge: &[u8]) -> Result<[u8; CHALLENGE_FRAME], PFError> {
    if challenge.is_empty() || challenge.len() > CHALLENGE_FRAME {
        return Err(PFError::Device(format!(
            "Challenge must be 1..={CHALLENGE_FRAME} bytes"
        )));
    }
    let mut frame = [0u8; CHALLENGE_FRAME];
    frame[..challenge.len()].copy_from_slice(challenge);
    if challenge.len() < CHALLENGE_FRAME {
        let pad = if *challenge.last().unwrap() == 0x7F {
            0x00
        } else {
            0x7F
        };
        frame[challenge.len()..].fill(pad);
    }
    Ok(frame)
}

/// Run an HMAC-SHA1 challenge-response against a slot (returns the 20-byte MAC).
pub fn calculate_hmac(
    session: &CcidSession,
    slot: u8,
    challenge: &[u8],
) -> Result<Vec<u8>, PFError> {
    let frame = pad_challenge(challenge)?;
    let (p1, p2) = chal_p1p2(slot);
    session.transceive_full(&Apdu::read(CLA_ISO, INS_OTP, p1, p2, &frame))
}

#[cfg(test)]
mod tests {
    use super::*;

    const NO_ACC: [u8; 6] = [0; 6];

    #[test]
    fn config_crc_residual_is_valid() {
        // Every builder must produce a frame that CRCs to the X.25 residual.
        assert_eq!(crc16(&build_chalresp(&[0x11; 20], false, &NO_ACC)), 0xF0B8);
        assert_eq!(crc16(&build_hotp(&[0xAB; 20], true, true, &NO_ACC)), 0xF0B8);
        assert_eq!(
            crc16(&build_static(&[0x04, 0x05, 0x06], false, &NO_ACC)),
            0xF0B8
        );
        assert_eq!(
            crc16(&build_yubico_otp(
                &[1; 6], &[2; 6], &[3; 16], false, &NO_ACC
            )),
            0xF0B8
        );
    }

    #[test]
    fn chalresp_layout() {
        let mut secret = [0u8; 20];
        for (i, b) in secret.iter_mut().enumerate() {
            *b = i as u8;
        }
        let c = build_chalresp(&secret, true, &NO_ACC);
        assert_eq!(c[OFF_TKT_FLAGS], 0x40); // TKT_CHAL_RESP
        assert_eq!(c[OFF_CFG_FLAGS], 0x22 | 0x04 | 0x08); // HMAC | LT64 | BTN_TRIG
        assert_eq!(&c[OFF_AES_KEY..OFF_AES_KEY + 16], &secret[..16]);
        assert_eq!(&c[OFF_UID..OFF_UID + 4], &secret[16..20]);
        assert_eq!(&c[OFF_UID + 4..OFF_UID + 6], &[0, 0]);
    }

    #[test]
    fn hotp_layout() {
        let c6 = build_hotp(&[0xAB; 20], false, false, &NO_ACC);
        assert_eq!(c6[OFF_TKT_FLAGS], 0x40); // TKT_OATH_HOTP
        assert_eq!(c6[OFF_CFG_FLAGS], 0x00); // 6 digits
        let c8 = build_hotp(&[0xAB; 20], true, true, &NO_ACC);
        assert_eq!(c8[OFF_TKT_FLAGS], 0x40 | 0x20); // + APPEND_CR
        assert_eq!(c8[OFF_CFG_FLAGS], 0x02); // OATH_HOTP8
    }

    #[test]
    fn static_and_yubico_layout() {
        let s = build_static(&[0x04, 0x05, 0x06, 0x07], true, &NO_ACC);
        assert_eq!(s[OFF_CFG_FLAGS], CFG_STATIC_TICKET);
        assert_eq!(s[OFF_TKT_FLAGS], TKT_APPEND_CR);
        assert_eq!(s[OFF_FIXED_SIZE], 4);
        assert_eq!(&s[..4], &[0x04, 0x05, 0x06, 0x07]);

        let y = build_yubico_otp(&[0xAA; 6], &[0xBB; 6], &[0xCC; 16], false, &NO_ACC);
        assert_eq!(y[OFF_TKT_FLAGS], 0); // plain OTP
        assert_eq!(y[OFF_CFG_FLAGS], 0);
        assert_eq!(y[OFF_FIXED_SIZE], 6);
        assert_eq!(&y[..6], &[0xAA; 6]);
        assert_eq!(&y[OFF_UID..OFF_UID + 6], &[0xBB; 6]);
    }

    #[test]
    fn access_code_is_embedded() {
        let acc = [1u8, 2, 3, 4, 5, 6];
        let c = build_chalresp(&[0; 20], false, &acc);
        assert_eq!(&c[OFF_ACC_CODE..OFF_ACC_CODE + 6], &acc);
        assert_eq!(crc16(&c), 0xF0B8);
    }

    #[test]
    fn modhex_roundtrips() {
        assert_eq!(modhex_encode(&[0x2d, 0x34, 0x4f]), "dteffv");
        assert_eq!(modhex_decode("dteffv").unwrap(), vec![0x2d, 0x34, 0x4f]);
        assert!(modhex_decode("zzzz").is_none()); // not modhex letters
        assert!(modhex_decode("cvc").is_none()); // odd length
    }

    #[test]
    fn scancodes_map_us_keyboard() {
        assert_eq!(ascii_to_scancodes("a").unwrap(), vec![0x04]);
        assert_eq!(ascii_to_scancodes("A").unwrap(), vec![0x80 | 0x04]);
        assert_eq!(ascii_to_scancodes("1!").unwrap(), vec![0x1E, 0x80 | 0x1E]);
        assert!(ascii_to_scancodes("héllo").is_none()); // non-ASCII
        assert!(ascii_to_scancodes(&"x".repeat(39)).is_none()); // too long
    }

    #[test]
    fn classify_types() {
        assert_eq!(classify(0x40, 0x26), SlotType::ChallengeResponse);
        assert_eq!(classify(0x40, 0x00), SlotType::OathHotp);
        assert_eq!(classify(0x40, 0x02), SlotType::OathHotp);
        assert_eq!(classify(0x00, 0x20), SlotType::StaticPassword);
        assert_eq!(classify(0x00, 0x00), SlotType::YubicoOtp);
    }

    /// Mirror the firmware's LT64 trim (strip trailing bytes == the last frame
    /// byte) to prove the padded frame round-trips back to the challenge.
    fn firmware_trim(frame: &[u8; CHALLENGE_FRAME]) -> &[u8] {
        let last = frame[CHALLENGE_FRAME - 1];
        let mut n = CHALLENGE_FRAME;
        while n > 0 && frame[n - 1] == last {
            n -= 1;
        }
        &frame[..n]
    }

    #[test]
    fn hmac_challenge_pads_to_64_and_round_trips() {
        let chal = [1u8, 2, 3, 4, 5, 6, 7, 8];
        let frame = pad_challenge(&chal).unwrap();
        assert_eq!(&frame[..8], &chal);
        assert_eq!(frame[8], 0x7F);
        assert_eq!(frame[CHALLENGE_FRAME - 1], 0x7F);
        assert_eq!(firmware_trim(&frame), &chal);
    }

    #[test]
    fn hmac_challenge_ending_in_pad_byte_uses_alt_fill() {
        // Last byte == the default pad (0x7F) → fill with 0x00 so the trim stops
        // at the challenge boundary instead of eating the final 0x7F.
        let chal = [0xAAu8, 0x7F];
        let frame = pad_challenge(&chal).unwrap();
        assert_eq!(frame[2], 0x00);
        assert_eq!(firmware_trim(&frame), &chal);
    }

    #[test]
    fn hmac_challenge_length_bounds() {
        assert!(pad_challenge(&[]).is_err());
        assert!(pad_challenge(&[0u8; CHALLENGE_FRAME + 1]).is_err());
        assert!(pad_challenge(&[0u8; CHALLENGE_FRAME]).is_ok());
        assert_eq!(
            pad_challenge(&[9u8; CHALLENGE_FRAME]).unwrap().len(),
            CHALLENGE_FRAME
        );
    }
}
