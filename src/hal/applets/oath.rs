//! YKOATH (Yubico OATH) applet client — TOTP/HOTP account management.
//!
//! Speaks the standard Yubico OATH wire protocol over a
//! [`CcidSession`], so it is
//! firmware-agnostic (RS-Key emulates the same AID and command set). The device
//! holds the secrets and computes the codes; the host only frames commands,
//! parses responses, and — for password-protected devices — derives the access
//! key (PBKDF2-HMAC-SHA1) and proves knowledge of it (HMAC-SHA1).

// The Accounts screen uses the account-list and rename operations; inline tests
// cover protocol helpers that do not have a direct screen call site.
#![allow(dead_code)]

use crate::error::PFError;
use crate::hal::apdu::{Apdu, CLA_ISO, tlv};
use crate::hal::transport::ccid::CcidSession;
use ring::rand::{SecureRandom, SystemRandom};
use ring::{hmac, pbkdf2};
use std::num::NonZeroU32;
use std::time::{SystemTime, UNIX_EPOCH};

/// Standard Yubico OATH applet AID.
pub const OATH_AID: &[u8] = &[0xA0, 0x00, 0x00, 0x05, 0x27, 0x21, 0x01];

// Instructions.
const INS_PUT: u8 = 0x01;
const INS_DELETE: u8 = 0x02;
const INS_SET_CODE: u8 = 0x03;
const INS_RESET: u8 = 0x04;
const INS_RENAME: u8 = 0x05;
const INS_LIST: u8 = 0xA1;
const INS_CALCULATE: u8 = 0xA2;
const INS_VALIDATE: u8 = 0xA3;
const INS_CALCULATE_ALL: u8 = 0xA4;

// Data-object tags.
const TAG_NAME: u32 = 0x71;
const TAG_NAME_LIST: u32 = 0x72;
const TAG_KEY: u32 = 0x73;
const TAG_CHALLENGE: u32 = 0x74;
const TAG_RESPONSE_FULL: u32 = 0x75;
const TAG_RESPONSE_TRUNC: u32 = 0x76;
const TAG_NO_RESPONSE: u32 = 0x77;
const TAG_PROPERTY: u32 = 0x78;
const TAG_VERSION: u32 = 0x79;
const TAG_IMF: u32 = 0x7A;
const TAG_TOUCH_RESPONSE: u32 = 0x7C;

const PROP_TOUCH: u8 = 0x02;
const DEFAULT_PERIOD: u32 = 30;
const ACCESS_KEY_LEN: usize = 16;
const PBKDF2_ITERS: u32 = 1000;
const YKOATH_MIN_KEY_LEN: usize = 14;

/// OATH credential kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OathType {
    Totp,
    Hotp,
}

impl OathType {
    fn wire(self) -> u8 {
        match self {
            Self::Hotp => 0x10,
            Self::Totp => 0x20,
        }
    }
    fn from_wire(b: u8) -> Self {
        if b & 0xF0 == 0x10 {
            Self::Hotp
        } else {
            Self::Totp
        }
    }
}

/// HMAC hash used by a credential.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HashAlgo {
    Sha1,
    Sha256,
    Sha512,
}

impl HashAlgo {
    fn wire(self) -> u8 {
        match self {
            Self::Sha1 => 0x01,
            Self::Sha256 => 0x02,
            Self::Sha512 => 0x03,
        }
    }
    fn from_wire(b: u8) -> Self {
        match b & 0x0F {
            0x02 => Self::Sha256,
            0x03 => Self::Sha512,
            _ => Self::Sha1,
        }
    }
    /// Label used in `otpauth://` URIs.
    pub fn label(self) -> &'static str {
        match self {
            Self::Sha1 => "SHA1",
            Self::Sha256 => "SHA256",
            Self::Sha512 => "SHA512",
        }
    }
}

/// The current code state for one credential after CALCULATE ALL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodeState {
    /// A computed TOTP code, valid for `period` seconds from its window start.
    Code { value: String, period: u32 },
    /// HOTP — counter-based; compute on demand with [`calculate`].
    Hotp,
    /// Requires a physical touch before the device will compute the code.
    Touch,
}

/// A parsed OATH account (identity + current code state).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
    /// Raw credential id as stored on the device (the wire `0x71` name).
    pub id: String,
    pub issuer: Option<String>,
    pub account: String,
    pub oath_type: OathType,
    pub period: u32,
    pub state: CodeState,
}

/// A new credential to enroll via [`put`].
#[derive(Debug, Clone)]
pub struct NewCredential {
    pub issuer: Option<String>,
    pub account: String,
    pub secret: Vec<u8>,
    pub oath_type: OathType,
    pub algorithm: HashAlgo,
    pub digits: u8,
    pub period: u32,
    pub counter: u32,
    pub touch: bool,
}

/// Applet identity parsed from the SELECT response.
#[derive(Debug, Clone)]
pub struct OathInfo {
    pub version: [u8; 3],
    /// PBKDF2 salt (device id) used to derive the access key.
    pub device_id: Vec<u8>,
    /// SELECT challenge — present only when an access code is set.
    pub challenge: Option<Vec<u8>>,
}

impl OathInfo {
    pub fn password_set(&self) -> bool {
        self.challenge.is_some()
    }
}

/// Parse the OATH SELECT response.
pub fn parse_select(resp: &[u8]) -> OathInfo {
    let mut version = [0u8; 3];
    if let Some(v) = tlv::find(resp, TAG_VERSION) {
        for (i, b) in v.iter().take(3).enumerate() {
            version[i] = *b;
        }
    }
    OathInfo {
        version,
        device_id: tlv::find(resp, TAG_NAME).unwrap_or(&[]).to_vec(),
        challenge: tlv::find(resp, TAG_CHALLENGE).map(|c| c.to_vec()),
    }
}

/// Open an OATH session (SELECT the applet).
pub fn open() -> Result<(CcidSession, OathInfo), PFError> {
    let session = CcidSession::open(OATH_AID)?;
    let info = parse_select(&session.select_resp);
    Ok((session, info))
}

/// Derive the 16-byte access key from a password and the device id (salt).
pub fn derive_access_key(password: &str, device_id: &[u8]) -> [u8; ACCESS_KEY_LEN] {
    let mut key = [0u8; ACCESS_KEY_LEN];
    pbkdf2::derive(
        pbkdf2::PBKDF2_HMAC_SHA1,
        NonZeroU32::new(PBKDF2_ITERS).unwrap(),
        device_id,
        password.as_bytes(),
        &mut key,
    );
    key
}

fn hmac_sha1(key: &[u8], msg: &[u8]) -> Vec<u8> {
    let k = hmac::Key::new(hmac::HMAC_SHA1_FOR_LEGACY_USE_ONLY, key);
    hmac::sign(&k, msg).as_ref().to_vec()
}

fn random8() -> Result<[u8; 8], PFError> {
    let mut c = [0u8; 8];
    SystemRandom::new()
        .fill(&mut c)
        .map_err(|_| PFError::Device("RNG failure".into()))?;
    Ok(c)
}

/// Unlock a password-protected applet with the derived access key and the
/// SELECT challenge, using YKOATH mutual authentication.
pub fn validate(session: &CcidSession, key: &[u8], select_challenge: &[u8]) -> Result<(), PFError> {
    let response = hmac_sha1(key, select_challenge);
    let host_challenge = random8()?;
    let mut data = Vec::new();
    tlv::write(&mut data, TAG_RESPONSE_FULL, &response);
    tlv::write(&mut data, TAG_CHALLENGE, &host_challenge);
    session.transceive_full(&Apdu::read(CLA_ISO, INS_VALIDATE, 0, 0, &data))?;
    Ok(())
}

/// Set (or replace) the applet access code. The device stores the key and
/// re-locks on the next SELECT.
pub fn set_code(session: &CcidSession, key: &[u8]) -> Result<(), PFError> {
    let challenge = random8()?;
    let response = hmac_sha1(key, &challenge);
    let mut key_tlv = vec![HashAlgo::Sha1.wire()];
    key_tlv.extend_from_slice(key);
    let mut data = Vec::new();
    tlv::write(&mut data, TAG_KEY, &key_tlv);
    tlv::write(&mut data, TAG_CHALLENGE, &challenge);
    tlv::write(&mut data, TAG_RESPONSE_FULL, &response);
    session.transceive_full(&Apdu::write(CLA_ISO, INS_SET_CODE, 0, 0, &data))?;
    Ok(())
}

/// Remove the access code (an empty key TLV).
pub fn clear_code(session: &CcidSession) -> Result<(), PFError> {
    let mut data = Vec::new();
    tlv::write(&mut data, TAG_KEY, &[]);
    session.transceive_full(&Apdu::write(CLA_ISO, INS_SET_CODE, 0, 0, &data))?;
    Ok(())
}

fn normalize_ykoath_secret(secret: &[u8]) -> Vec<u8> {
    let mut normalized = secret.to_vec();
    if normalized.len() < YKOATH_MIN_KEY_LEN {
        normalized.resize(YKOATH_MIN_KEY_LEN, 0);
    }
    normalized
}

/// Add or overwrite a credential.
pub fn put(session: &CcidSession, cred: &NewCredential) -> Result<(), PFError> {
    let id = build_cred_id(
        cred.issuer.as_deref(),
        &cred.account,
        cred.oath_type,
        cred.period,
    );
    let mut key_tlv = vec![cred.oath_type.wire() | cred.algorithm.wire(), cred.digits];
    key_tlv.extend_from_slice(&normalize_ykoath_secret(&cred.secret));

    let mut data = Vec::new();
    tlv::write(&mut data, TAG_NAME, id.as_bytes());
    tlv::write(&mut data, TAG_KEY, &key_tlv);
    if cred.touch {
        // Yubico quirk: the property object is a bare value with no length octet.
        data.push(TAG_PROPERTY as u8);
        data.push(PROP_TOUCH);
    }
    if cred.oath_type == OathType::Hotp && cred.counter != 0 {
        tlv::write(&mut data, TAG_IMF, &cred.counter.to_be_bytes());
    }
    session.transceive_full(&Apdu::write(CLA_ISO, INS_PUT, 0, 0, &data))?;
    Ok(())
}

/// Delete a credential by its raw id.
pub fn delete(session: &CcidSession, id: &str) -> Result<(), PFError> {
    let mut data = Vec::new();
    tlv::write(&mut data, TAG_NAME, id.as_bytes());
    session.transceive_full(&Apdu::write(CLA_ISO, INS_DELETE, 0, 0, &data))?;
    Ok(())
}

/// Rename a credential (YubiKey 5.3+ / RS-Key).
pub fn rename(session: &CcidSession, old_id: &str, new_id: &str) -> Result<(), PFError> {
    let mut data = Vec::new();
    tlv::write(&mut data, TAG_NAME, old_id.as_bytes());
    tlv::write(&mut data, TAG_NAME, new_id.as_bytes());
    session.transceive_full(&Apdu::write(CLA_ISO, INS_RENAME, 0, 0, &data))?;
    Ok(())
}

/// Factory-reset the applet (destroys all credentials and the access code).
pub fn reset(session: &CcidSession) -> Result<(), PFError> {
    session.transceive_full(&Apdu::write(CLA_ISO, INS_RESET, 0xDE, 0xAD, &[]))?;
    Ok(())
}

/// Compute one credential's code for a given period (used for HOTP and for
/// TOTP credentials whose period is not 30 s).
pub fn calculate(session: &CcidSession, id: &str, period: u32) -> Result<String, PFError> {
    let counter = time_counter(period.max(1));
    let mut data = Vec::new();
    tlv::write(&mut data, TAG_NAME, id.as_bytes());
    tlv::write(&mut data, TAG_CHALLENGE, &counter.to_be_bytes());
    // P2 = 1 → truncated response.
    let resp = session.transceive_full(&Apdu::read(CLA_ISO, INS_CALCULATE, 0, 1, &data))?;
    let (_, value) = tlv::TlvIter::new(&resp)
        .next()
        .ok_or_else(|| PFError::Device("Empty CALCULATE response".into()))?;
    format_response(value)
}

/// List every credential and its current code in one CALCULATE ALL round-trip,
/// re-computing any non-30 s TOTP credential individually (matches ykman).
pub fn calculate_all(session: &CcidSession) -> Result<Vec<Account>, PFError> {
    let counter = time_counter(DEFAULT_PERIOD);
    let mut req = Vec::new();
    tlv::write(&mut req, TAG_CHALLENGE, &counter.to_be_bytes());
    let resp = session.transceive_oath(&Apdu::read(CLA_ISO, INS_CALCULATE_ALL, 0, 1, &req))?;

    let mut accounts = Vec::new();
    let mut pending: Option<(String, OathType)> = None;
    for (tag, value) in tlv::TlvIter::new(&resp) {
        if tag == TAG_NAME {
            let id = String::from_utf8_lossy(value).to_string();
            pending = Some((id, OathType::Totp));
            continue;
        }
        let Some((id, _)) = pending.take() else {
            continue;
        };
        let (issuer, account, period) = parse_cred_id(&id);
        let (oath_type, state) = match tag {
            TAG_NO_RESPONSE => (OathType::Hotp, CodeState::Hotp),
            TAG_TOUCH_RESPONSE => (OathType::Totp, CodeState::Touch),
            TAG_RESPONSE_TRUNC | TAG_RESPONSE_FULL => {
                let value_str = format_response(value)?;
                (
                    OathType::Totp,
                    CodeState::Code {
                        value: value_str,
                        period,
                    },
                )
            }
            _ => continue,
        };
        accounts.push(Account {
            id,
            issuer,
            account,
            oath_type,
            period,
            state,
        });
    }

    // Non-30 s TOTP codes computed above used the 30 s window — fix them up.
    for acc in accounts.iter_mut() {
        if acc.oath_type == OathType::Totp
            && acc.period != DEFAULT_PERIOD
            && matches!(acc.state, CodeState::Code { .. })
            && let Ok(code) = calculate(session, &acc.id, acc.period)
        {
            acc.state = CodeState::Code {
                value: code,
                period: acc.period,
            };
        }
    }
    Ok(accounts)
}

/// Seconds remaining in the current window for a period.
pub fn seconds_remaining(period: u32) -> u32 {
    let p = period.max(1);
    let now = unix_now();
    p - (now % p as u64) as u32
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn time_counter(period: u32) -> u64 {
    unix_now() / period.max(1) as u64
}

/// Format a CALCULATE/CALCULATE ALL response value into the numeric code.
/// Handles both truncated (`0x76`: `[digits, 4-byte int]`) and full
/// (`0x75`: `[digits, full HMAC]`, truncated host-side) forms.
fn format_response(value: &[u8]) -> Result<String, PFError> {
    let digits = *value
        .first()
        .ok_or_else(|| PFError::Device("Short code response".into()))?;
    let body = &value[1..];
    let num = if body.len() == 4 {
        u32::from_be_bytes([body[0], body[1], body[2], body[3]]) & 0x7FFF_FFFF
    } else if body.len() >= 20 {
        // Dynamic truncation of a full HMAC (RFC 4226).
        let offset = (body[body.len() - 1] & 0x0F) as usize;
        u32::from_be_bytes([
            body[offset],
            body[offset + 1],
            body[offset + 2],
            body[offset + 3],
        ]) & 0x7FFF_FFFF
    } else {
        return Err(PFError::Device("Malformed code response".into()));
    };
    let modulo = 10u32.pow(digits.min(9) as u32);
    Ok(format!("{:0width$}", num % modulo, width = digits as usize))
}

/// Build the Yubico credential id: `[<period>/]<issuer>:<account>`, with the
/// period prefix only for non-30 s TOTP credentials.
pub fn build_cred_id(
    issuer: Option<&str>,
    account: &str,
    oath_type: OathType,
    period: u32,
) -> String {
    let base = match issuer {
        Some(i) if !i.is_empty() => format!("{i}:{account}"),
        _ => account.to_string(),
    };
    if oath_type == OathType::Totp && period != DEFAULT_PERIOD && period != 0 {
        format!("{period}/{base}")
    } else {
        base
    }
}

/// Split a credential id back into `(issuer, account, period)`.
pub fn parse_cred_id(id: &str) -> (Option<String>, String, u32) {
    let (period, rest) = match id.split_once('/') {
        Some((p, r)) if p.parse::<u32>().is_ok() => (p.parse().unwrap(), r),
        _ => (DEFAULT_PERIOD, id),
    };
    match rest.split_once(':') {
        Some((issuer, account)) => (Some(issuer.to_string()), account.to_string(), period),
        None => (None, rest.to_string(), period),
    }
}

/// Parse an `otpauth://` URI into a [`NewCredential`].
pub fn parse_otpauth(uri: &str) -> Result<NewCredential, String> {
    let rest = uri
        .strip_prefix("otpauth://")
        .ok_or("Not an otpauth:// URI")?;
    let (type_part, after) = rest.split_once('/').ok_or("Missing credential type")?;
    let oath_type = match type_part.to_ascii_lowercase().as_str() {
        "totp" => OathType::Totp,
        "hotp" => OathType::Hotp,
        _ => return Err("Type must be totp or hotp".into()),
    };
    let (label, query) = match after.split_once('?') {
        Some((l, q)) => (l, q),
        None => (after, ""),
    };
    let label = url_decode(label);
    let (mut issuer, account) = match label.split_once(':') {
        Some((i, a)) => (Some(i.trim().to_string()), a.trim().to_string()),
        None => (None, label.trim().to_string()),
    };

    let mut secret_b32 = None;
    let mut algorithm = HashAlgo::Sha1;
    let mut digits = 6u8;
    let mut period = DEFAULT_PERIOD;
    let mut counter = 0u32;
    for pair in query.split('&').filter(|p| !p.is_empty()) {
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        let v = url_decode(v);
        match k.to_ascii_lowercase().as_str() {
            "secret" => secret_b32 = Some(v),
            "issuer" => {
                if !v.is_empty() {
                    issuer = Some(v);
                }
            }
            "algorithm" => {
                algorithm = match v.to_ascii_uppercase().as_str() {
                    "SHA256" => HashAlgo::Sha256,
                    "SHA512" => HashAlgo::Sha512,
                    _ => HashAlgo::Sha1,
                }
            }
            "digits" => digits = v.parse().unwrap_or(6),
            "period" => period = v.parse().unwrap_or(DEFAULT_PERIOD),
            "counter" => counter = v.parse().unwrap_or(0),
            _ => {}
        }
    }

    let secret =
        base32_decode(&secret_b32.ok_or("Missing secret")?).ok_or("Invalid base32 secret")?;
    if secret.is_empty() {
        return Err("Empty secret".into());
    }
    if account.is_empty() {
        return Err("Missing account name".into());
    }
    Ok(NewCredential {
        issuer,
        account,
        secret,
        oath_type,
        algorithm,
        digits: digits.clamp(6, 8),
        period,
        counter,
        touch: false,
    })
}

/// Decode RFC 4648 base32 (case-insensitive, padding and spaces ignored).
pub fn base32_decode(s: &str) -> Option<Vec<u8>> {
    const ALPHABET: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    let mut bits = 0u32;
    let mut nbits = 0u32;
    let mut out = Vec::new();
    for c in s.chars() {
        if c == '=' || c.is_whitespace() || c == '-' {
            continue;
        }
        let up = (c as u8).to_ascii_uppercase();
        let v = ALPHABET.iter().position(|&x| x == up)? as u32;
        bits = (bits << 5) | v;
        nbits += 5;
        if nbits >= 8 {
            nbits -= 8;
            out.push((bits >> nbits) as u8);
        }
    }
    Some(out)
}

/// Minimal percent-decoding (`+` → space, `%XX` → byte) for URI labels/params.
fn url_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len() => {
                let hi = (bytes[i + 1] as char).to_digit(16);
                let lo = (bytes[i + 2] as char).to_digit(16);
                if let (Some(h), Some(l)) = (hi, lo) {
                    out.push((h * 16 + l) as u8);
                    i += 3;
                } else {
                    out.push(bytes[i]);
                    i += 1;
                }
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).to_string()
}

/// Parse a bare LIST response into credential ids + kinds (used when a code
/// isn't needed). `0x72` value = `[type|algo, name…]`.
pub fn parse_list(resp: &[u8]) -> Vec<(String, OathType, HashAlgo)> {
    let mut out = Vec::new();
    for (tag, value) in tlv::TlvIter::new(resp) {
        if tag == TAG_NAME_LIST && !value.is_empty() {
            let props = value[0];
            let name = String::from_utf8_lossy(&value[1..]).to_string();
            out.push((name, OathType::from_wire(props), HashAlgo::from_wire(props)));
        }
    }
    out
}

/// LIST every credential id on the device (chained via SEND REMAINING).
pub fn list(session: &CcidSession) -> Result<Vec<(String, OathType, HashAlgo)>, PFError> {
    let resp = session.transceive_oath(&Apdu::read(CLA_ISO, INS_LIST, 0, 0, &[]))?;
    Ok(parse_list(&resp))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base32_decodes_known_vectors() {
        assert_eq!(
            base32_decode("JBSWY3DPEHPK3PXP").unwrap(),
            b"Hello!\xde\xad\xbe\xef"
        );
        assert_eq!(base32_decode("").unwrap(), Vec::<u8>::new());
        // Lowercase, spaces and padding tolerated.
        assert_eq!(base32_decode("nb sw y3=dp").unwrap(), b"hello");
        assert!(base32_decode("0189").is_none()); // invalid symbols
    }

    #[test]
    fn short_ykoath_secret_is_zero_padded() {
        let secret: Vec<u8> = (0xA0..0xAA).collect();
        let normalized = normalize_ykoath_secret(&secret);

        assert_eq!(normalized.len(), 14);
        assert_eq!(&normalized[..10], secret.as_slice());
        assert_eq!(&normalized[10..], &[0u8; 4]);
    }

    #[test]
    fn ykoath_secret_padding_preserves_boundary_lengths() {
        let thirteen = vec![0x13; 13];
        assert_eq!(
            normalize_ykoath_secret(&thirteen),
            [thirteen, vec![0]].concat()
        );

        let fourteen: Vec<u8> = (0..14).collect();
        assert_eq!(normalize_ykoath_secret(&fourteen), fourteen);

        let longer: Vec<u8> = (0..16).collect();
        assert_eq!(normalize_ykoath_secret(&longer), longer);
    }

    #[test]
    fn short_base32_secret_is_normalized_without_changing_decode_errors() {
        let credential = parse_otpauth("otpauth://totp/example?secret=JBSWY3DPEHPK3PXP").unwrap();
        assert_eq!(credential.secret, b"Hello!\xde\xad\xbe\xef");
        assert_eq!(normalize_ykoath_secret(&credential.secret).len(), 14);

        assert_eq!(
            parse_otpauth("otpauth://totp/example?secret=0189").unwrap_err(),
            "Invalid base32 secret"
        );
        assert_eq!(
            parse_otpauth("otpauth://totp/example?secret=").unwrap_err(),
            "Empty secret"
        );
    }

    #[test]
    fn short_key_zero_padding_preserves_hmac_sha1_otp() {
        let short_key = b"1234567890";
        let normalized_key = normalize_ykoath_secret(short_key);
        let time_step = 42u64.to_be_bytes();

        let short_hmac = hmac_sha1(short_key, &time_step);
        let normalized_hmac = hmac_sha1(&normalized_key, &time_step);
        assert_eq!(short_hmac, normalized_hmac);

        let mut short_response = vec![6u8];
        short_response.extend_from_slice(&short_hmac);
        let mut normalized_response = vec![6u8];
        normalized_response.extend_from_slice(&normalized_hmac);
        assert_eq!(format_response(&short_response).unwrap(), "055299");
        assert_eq!(
            format_response(&short_response).unwrap(),
            format_response(&normalized_response).unwrap()
        );
    }

    #[test]
    fn cred_id_roundtrips() {
        assert_eq!(
            build_cred_id(Some("GitHub"), "alice", OathType::Totp, 30),
            "GitHub:alice"
        );
        assert_eq!(
            build_cred_id(Some("AWS"), "bob", OathType::Totp, 60),
            "60/AWS:bob"
        );
        assert_eq!(build_cred_id(None, "solo", OathType::Hotp, 30), "solo");

        assert_eq!(
            parse_cred_id("GitHub:alice"),
            (Some("GitHub".into()), "alice".into(), 30)
        );
        assert_eq!(
            parse_cred_id("60/AWS:bob"),
            (Some("AWS".into()), "bob".into(), 60)
        );
        assert_eq!(parse_cred_id("solo"), (None, "solo".into(), 30));
        // A slash that isn't a period prefix stays part of the account.
        assert_eq!(parse_cred_id("a/b:c"), (Some("a/b".into()), "c".into(), 30));
    }

    #[test]
    fn parses_otpauth_uri() {
        let c = parse_otpauth(
            "otpauth://totp/ACME%20Co:john@example.com?secret=JBSWY3DPEHPK3PXP&issuer=ACME%20Co&algorithm=SHA256&digits=8&period=60",
        )
        .unwrap();
        assert_eq!(c.issuer.as_deref(), Some("ACME Co"));
        assert_eq!(c.account, "john@example.com");
        assert_eq!(c.oath_type, OathType::Totp);
        assert_eq!(c.algorithm, HashAlgo::Sha256);
        assert_eq!(c.digits, 8);
        assert_eq!(c.period, 60);
        assert_eq!(c.secret, b"Hello!\xde\xad\xbe\xef");
    }

    #[test]
    fn otpauth_requires_secret_and_type() {
        assert!(parse_otpauth("otpauth://totp/x").is_err());
        assert!(parse_otpauth("https://totp/x?secret=AA").is_err());
        assert!(parse_otpauth("otpauth://sms/x?secret=AA").is_err());
    }

    #[test]
    fn truncated_response_formats_code() {
        // digits=6, truncated int 0x0000_04F0 = 1264 → "001264".
        let value = [6u8, 0x00, 0x00, 0x04, 0xF0];
        assert_eq!(format_response(&value).unwrap(), "001264");
    }

    #[test]
    fn full_hmac_response_dynamic_truncation() {
        // RFC 4226 §5.4 canonical HMAC → 6-digit code 872921.
        let hmac = [
            0x1f, 0x86, 0x98, 0x69, 0x0e, 0x02, 0xca, 0x16, 0x61, 0x85, 0x50, 0xef, 0x7f, 0x19,
            0xda, 0x8e, 0x94, 0x5b, 0x55, 0x5a,
        ];
        let mut value = vec![6u8];
        value.extend_from_slice(&hmac);
        assert_eq!(format_response(&value).unwrap(), "872921");
    }

    #[test]
    fn list_parses_type_and_algo() {
        let mut resp = Vec::new();
        // 0x72: [0x21, "A:b"] → TOTP + SHA1.
        tlv::write(&mut resp, TAG_NAME_LIST, &[0x21, b'A', b':', b'b']);
        tlv::write(&mut resp, TAG_NAME_LIST, &[0x12, b'h']); // HOTP + SHA256
        let list = parse_list(&resp);
        assert_eq!(list.len(), 2);
        assert_eq!(list[0], ("A:b".to_string(), OathType::Totp, HashAlgo::Sha1));
        assert_eq!(list[1], ("h".to_string(), OathType::Hotp, HashAlgo::Sha256));
    }
}
