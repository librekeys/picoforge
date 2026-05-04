use crate::{
    device::types::{TotpEntry, TotpStatus},
    error::PFError,
};
use pcsc::{Card, Context, Protocols, Scope, ShareMode};
use rand::random;
use ring::{hmac, pbkdf2};
use std::collections::HashMap;
use std::num::NonZeroU32;
use std::time::{SystemTime, UNIX_EPOCH};

const OATH_AID: &[u8] = &[0xA0, 0x00, 0x00, 0x05, 0x27, 0x21, 0x01];

const TAG_NAME: u8 = 0x71;
const TAG_NAME_LIST: u8 = 0x72;
const TAG_KEY: u8 = 0x73;
const TAG_CHALLENGE: u8 = 0x74;
const TAG_RESPONSE: u8 = 0x75;
const TAG_T_RESPONSE: u8 = 0x76;
const TAG_NO_RESPONSE: u8 = 0x77;
const TAG_T_VERSION: u8 = 0x79;
const TAG_ALGO: u8 = 0x7B;
const TAG_TOUCH_RESPONSE: u8 = 0x7C;
const TAG_PIN_COUNTER: u8 = 0x82;

const INS_PUT: u8 = 0x01;
const INS_DELETE: u8 = 0x02;
const INS_SET_CODE: u8 = 0x03;
const INS_RENAME: u8 = 0x05;
const INS_LIST: u8 = 0xA1;
const INS_VALIDATE: u8 = 0xA3;
const INS_CALCULATE_ALL: u8 = 0xA4;

const OATH_TYPE_MASK: u8 = 0xF0;
const OATH_TYPE_TOTP: u8 = 0x20;
const ALG_MASK: u8 = 0x0F;
const ALG_SHA1: u8 = 0x01;
const ALG_SHA256: u8 = 0x02;
const ALG_SHA512: u8 = 0x03;

#[derive(Default, Clone)]
struct OathSelectInfo {
    version: Option<String>,
    serial: Option<String>,
    salt: Option<Vec<u8>>,
    challenge: Option<Vec<u8>>,
    pin_retries: Option<u8>,
}

#[derive(Clone)]
struct ParsedUri {
    label: String,
    issuer: Option<String>,
    secret: Vec<u8>,
    algorithm: u8,
    digits: u8,
}

fn connect_and_select() -> Result<(Card, OathSelectInfo), PFError> {
    let ctx = Context::establish(Scope::User)?;
    let mut readers_buf = [0; 2048];
    let mut readers = ctx.list_readers(&mut readers_buf)?;
    let reader = readers.next().ok_or(PFError::NoDevice)?;
    let card = ctx.connect(reader, ShareMode::Shared, Protocols::ANY)?;

    let mut apdu = vec![0x00, 0xA4, 0x04, 0x00, OATH_AID.len() as u8];
    apdu.extend_from_slice(OATH_AID);

    let mut rx_buf = [0; 512];
    let rx = card.transmit(&apdu, &mut rx_buf)?;
    if rx.len() < 2 {
        return Err(PFError::Device("Short OATH select response".into()));
    }
    let (sw1, sw2) = (rx[rx.len() - 2], rx[rx.len() - 1]);
    if (sw1, sw2) != (0x90, 0x00) {
        return Err(PFError::Device(format!(
            "OATH applet not available (SW={:02X}{:02X})",
            sw1, sw2
        )));
    }

    let mut info = OathSelectInfo::default();
    for (tag, value) in parse_tlvs(&rx[..rx.len() - 2])? {
        match tag {
            TAG_T_VERSION if value.len() == 3 => {
                info.version = Some(format!("{}.{}.{}", value[0], value[1], value[2]));
            }
            TAG_NAME => {
                info.serial = Some(String::from_utf8_lossy(value).into_owned());
                info.salt = Some(value.to_vec());
            }
            TAG_CHALLENGE => {
                info.challenge = Some(value.to_vec());
            }
            TAG_PIN_COUNTER if value.len() == 1 => {
                info.pin_retries = Some(value[0]);
            }
            TAG_ALGO | _ => {}
        }
    }

    Ok((card, info))
}

fn tlv(tag: u8, value: &[u8]) -> Result<Vec<u8>, PFError> {
    if value.len() > u8::MAX as usize {
        return Err(PFError::Io("OATH TLV payload too large".into()));
    }
    let mut out = Vec::with_capacity(value.len() + 2);
    out.push(tag);
    out.push(value.len() as u8);
    out.extend_from_slice(value);
    Ok(out)
}

fn derive_access_key(salt: &[u8], password: &str) -> [u8; 16] {
    let mut key = [0u8; 16];
    pbkdf2::derive(
        pbkdf2::PBKDF2_HMAC_SHA1,
        NonZeroU32::new(1000).unwrap(),
        salt,
        password.as_bytes(),
        &mut key,
    );
    key
}

fn hmac_sha1(key: &[u8], message: &[u8]) -> Vec<u8> {
    let key = hmac::Key::new(hmac::HMAC_SHA1_FOR_LEGACY_USE_ONLY, key);
    hmac::sign(&key, message).as_ref().to_vec()
}

fn unlock_if_needed(
    card: &Card,
    info: &OathSelectInfo,
    password: Option<&str>,
) -> Result<(), PFError> {
    let Some(challenge) = info.challenge.as_ref() else {
        return Ok(());
    };
    let password = password
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| PFError::Device("OATH storage is locked. Enter the OATH password.".into()))?;
    let salt = info
        .salt
        .as_deref()
        .ok_or_else(|| PFError::Io("Missing OATH salt in select response".into()))?;

    let key = derive_access_key(salt, password);
    let response = hmac_sha1(&key, challenge);
    let verify_challenge: [u8; 8] = random();
    let expected = hmac_sha1(&key, &verify_challenge);

    let mut data = tlv(TAG_RESPONSE, &response)?;
    data.extend_from_slice(&tlv(TAG_CHALLENGE, &verify_challenge)?);

    let resp = transmit(card, INS_VALIDATE, 0x00, 0x00, &data)?;
    let response_tlv = parse_tlvs(&resp)?
        .into_iter()
        .find(|(tag, _)| *tag == TAG_RESPONSE)
        .ok_or_else(|| PFError::Io("Missing OATH validation response".into()))?;
    if response_tlv.1 != expected.as_slice() {
        return Err(PFError::Device(
            "OATH validation response did not match the expected proof".into(),
        ));
    }
    Ok(())
}

fn transmit(card: &Card, ins: u8, p1: u8, p2: u8, data: &[u8]) -> Result<Vec<u8>, PFError> {
    if data.len() > u8::MAX as usize {
        return Err(PFError::Io("APDU payload too large".into()));
    }

    let mut apdu = vec![0x00, ins, p1, p2, data.len() as u8];
    apdu.extend_from_slice(data);

    let mut rx_buf = [0; 2048];
    let rx = card.transmit(&apdu, &mut rx_buf)?;
    if rx.len() < 2 {
        return Err(PFError::Device(format!("Short APDU response for INS 0x{ins:02X}")));
    }

    let (sw1, sw2) = (rx[rx.len() - 2], rx[rx.len() - 1]);
    match (sw1, sw2) {
        (0x90, 0x00) => Ok(rx[..rx.len() - 2].to_vec()),
        (0x69, 0x82) => Err(PFError::Device(
            "OATH storage is locked or protected; protected OATH stores are not supported yet"
                .into(),
        )),
        (0x6A, 0x80) => Err(PFError::Device("Invalid OATH data".into())),
        (0x6A, 0x84) => Err(PFError::Device("OATH storage is full".into())),
        (0x63, sw2) => Err(PFError::Device(format!(
            "OATH verification failed (retries remaining: {})",
            sw2 & 0x0F
        ))),
        _ => Err(PFError::Device(format!(
            "OATH APDU failed (INS=0x{ins:02X}, SW={sw1:02X}{sw2:02X})"
        ))),
    }
}

fn parse_tlvs(data: &[u8]) -> Result<Vec<(u8, &[u8])>, PFError> {
    let mut items = Vec::new();
    let mut i = 0usize;
    while i < data.len() {
        if i + 2 > data.len() {
            return Err(PFError::Io("Malformed OATH TLV".into()));
        }
        let tag = data[i];
        let len = data[i + 1] as usize;
        i += 2;
        if i + len > data.len() {
            return Err(PFError::Io("Malformed OATH TLV length".into()));
        }
        items.push((tag, &data[i..i + len]));
        i += len;
    }
    Ok(items)
}

fn parse_name_parts(name: &str) -> (Option<String>, Option<String>) {
    if let Some((issuer, account)) = name.split_once(':') {
        let issuer = issuer.trim();
        let account = account.trim();
        let issuer = if issuer.is_empty() { None } else { Some(issuer.to_string()) };
        let account = if account.is_empty() { None } else { Some(account.to_string()) };
        (issuer, account)
    } else {
        (None, Some(name.to_string()))
    }
}

fn algorithm_name(algorithm: u8) -> &'static str {
    match algorithm & ALG_MASK {
        ALG_SHA1 => "SHA1",
        ALG_SHA256 => "SHA256",
        ALG_SHA512 => "SHA512",
        _ => "Unknown",
    }
}

fn pow10(digits: u8) -> u32 {
    10u32.saturating_pow(digits as u32)
}

fn current_challenge(period: u32) -> [u8; 8] {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let counter = now / period as u64;
    counter.to_be_bytes()
}

fn parse_list_entries(data: &[u8]) -> Result<Vec<TotpEntry>, PFError> {
    let mut entries = Vec::new();
    for (tag, value) in parse_tlvs(data)? {
        if tag != TAG_NAME_LIST || value.len() < 2 {
            continue;
        }
        let key_type = value[0];
        let props = *value.last().unwrap_or(&0);
        if (key_type & OATH_TYPE_MASK) != OATH_TYPE_TOTP {
            continue;
        }
        if value.len() < 3 {
            continue;
        }
        let name_bytes = &value[1..value.len() - 1];
        let name = String::from_utf8_lossy(name_bytes).into_owned();
        let (issuer, account_name) = parse_name_parts(&name);
        entries.push(TotpEntry {
            name,
            issuer,
            account_name,
            algorithm: algorithm_name(key_type).to_string(),
            digits: 6,
            period: 30,
            current_code: None,
            requires_touch: (props & 0x01) != 0,
        });
    }
    Ok(entries)
}

fn parse_code_map(data: &[u8]) -> Result<HashMap<String, (u8, String)>, PFError> {
    let mut map = HashMap::new();
    let mut i = 0usize;
    while i < data.len() {
        if i + 2 > data.len() {
            return Err(PFError::Io("Malformed OATH calculateAll response".into()));
        }
        if data[i] != TAG_NAME {
            return Err(PFError::Io("Expected OATH name tag in calculateAll response".into()));
        }
        let name_len = data[i + 1] as usize;
        i += 2;
        if i + name_len > data.len() {
            return Err(PFError::Io("Malformed OATH name length".into()));
        }
        let name = String::from_utf8_lossy(&data[i..i + name_len]).into_owned();
        i += name_len;
        if i + 2 > data.len() {
            return Err(PFError::Io("Missing OATH response tag".into()));
        }
        let tag = data[i];
        let len = data[i + 1] as usize;
        i += 2;
        if i + len > data.len() {
            return Err(PFError::Io("Malformed OATH response length".into()));
        }
        let value = &data[i..i + len];
        i += len;
        match tag {
            TAG_T_RESPONSE if len >= 5 => {
                let digits = value[0];
                let raw = u32::from_be_bytes([value[1], value[2], value[3], value[4]]);
                let code = raw % pow10(digits);
                map.insert(name, (digits, format!("{:0width$}", code, width = digits as usize)));
            }
            TAG_TOUCH_RESPONSE | TAG_NO_RESPONSE => {
                if len >= 1 {
                    map.insert(name, (value[0], String::new()));
                }
            }
            _ => {}
        }
    }
    Ok(map)
}

fn percent_decode(input: &str) -> Result<String, PFError> {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0usize;
    while i < bytes.len() {
        match bytes[i] {
            b'%' => {
                if i + 2 >= bytes.len() {
                    return Err(PFError::Io("Invalid percent-encoding in otpauth URI".into()));
                }
                let hi = (bytes[i + 1] as char)
                    .to_digit(16)
                    .ok_or_else(|| PFError::Io("Invalid percent-encoding in otpauth URI".into()))?;
                let lo = (bytes[i + 2] as char)
                    .to_digit(16)
                    .ok_or_else(|| PFError::Io("Invalid percent-encoding in otpauth URI".into()))?;
                out.push(((hi << 4) | lo) as u8);
                i += 3;
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8(out).map_err(|_| PFError::Io("Invalid UTF-8 in otpauth URI".into()))
}

fn decode_base32(secret: &str) -> Result<Vec<u8>, PFError> {
    let mut bits = 0u32;
    let mut bit_len = 0u8;
    let mut out = Vec::new();
    for ch in secret.chars() {
        let ch = ch.to_ascii_uppercase();
        let value = match ch {
            'A'..='Z' => ch as u8 - b'A',
            '2'..='7' => 26 + (ch as u8 - b'2'),
            '=' | ' ' => continue,
            _ => return Err(PFError::Io("TOTP secret is not valid Base32".into())),
        } as u32;
        bits = (bits << 5) | value;
        bit_len += 5;
        while bit_len >= 8 {
            bit_len -= 8;
            out.push(((bits >> bit_len) & 0xFF) as u8);
        }
    }
    if out.is_empty() {
        return Err(PFError::Io("TOTP secret is empty".into()));
    }
    Ok(out)
}

fn parse_otpauth_uri(uri: &str) -> Result<ParsedUri, PFError> {
    let trimmed = uri.trim();
    let rest = trimmed
        .strip_prefix("otpauth://")
        .ok_or_else(|| PFError::Io("Expected an otpauth:// URI".into()))?;
    let (kind, remainder) = rest
        .split_once('/')
        .ok_or_else(|| PFError::Io("Invalid otpauth URI".into()))?;
    if !kind.eq_ignore_ascii_case("totp") {
        return Err(PFError::Io("Only TOTP otpauth URIs are supported".into()));
    }
    let (label_raw, query_raw) = remainder
        .split_once('?')
        .ok_or_else(|| PFError::Io("otpauth URI is missing query parameters".into()))?;
    let label = percent_decode(label_raw)?.trim_matches('/').trim().to_string();
    if label.is_empty() {
        return Err(PFError::Io("otpauth URI label is empty".into()));
    }

    let mut params = HashMap::new();
    for pair in query_raw.split('&') {
        if pair.is_empty() {
            continue;
        }
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        params.insert(percent_decode(key)?.to_lowercase(), percent_decode(value)?);
    }

    let secret = params
        .get("secret")
        .ok_or_else(|| PFError::Io("otpauth URI is missing secret".into()))?;
    let secret = decode_base32(secret)?;

    let algorithm = match params
        .get("algorithm")
        .map(|s| s.trim().to_ascii_uppercase())
        .unwrap_or_else(|| "SHA1".into())
        .as_str()
    {
        "SHA1" => ALG_SHA1,
        "SHA256" => ALG_SHA256,
        "SHA512" => ALG_SHA512,
        other => return Err(PFError::Io(format!("Unsupported TOTP algorithm: {other}"))),
    };

    let digits = params
        .get("digits")
        .map(|s| s.parse::<u8>())
        .transpose()
        .map_err(|_| PFError::Io("Invalid TOTP digits".into()))?
        .unwrap_or(6);
    if digits != 6 && digits != 8 {
        return Err(PFError::Io("Only 6-digit and 8-digit TOTP codes are supported".into()));
    }

    let period = params
        .get("period")
        .map(|s| s.parse::<u32>())
        .transpose()
        .map_err(|_| PFError::Io("Invalid TOTP period".into()))?
        .unwrap_or(30);
    if period != 30 {
        return Err(PFError::Io(
            "Only 30-second TOTP periods are supported right now".into(),
        ));
    }

    let issuer = params
        .get("issuer")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let (label_issuer, _) = parse_name_parts(&label);
    let issuer = issuer.or(label_issuer);

    Ok(ParsedUri {
        label,
        issuer,
        secret,
        algorithm,
        digits,
    })
}

pub fn get_totp_status(password: Option<String>) -> Result<TotpStatus, PFError> {
    let (card, info) = connect_and_select()?;
    let password = password.as_deref();

    let list_resp = match transmit(&card, INS_LIST, 0x00, 0x00, &[0x01]) {
        Ok(resp) => resp,
        Err(PFError::Device(msg)) if msg.contains("locked or protected") => {
            if password.is_none() {
                return Ok(TotpStatus {
                    supported: true,
                    version: info.version,
                    serial: info.serial,
                    protected: true,
                    pin_retries: info.pin_retries,
                    entries: Vec::new(),
                });
            }
            unlock_if_needed(&card, &info, password)?;
            transmit(&card, INS_LIST, 0x00, 0x00, &[0x01])?
        }
        Err(err) => return Err(err),
    };

    let mut entries = parse_list_entries(&list_resp)?;
    let challenge = current_challenge(30);
    let calc_resp = match transmit(
        &card,
        INS_CALCULATE_ALL,
        0x00,
        0x01,
        &[
            TAG_CHALLENGE,
            8,
            challenge[0],
            challenge[1],
            challenge[2],
            challenge[3],
            challenge[4],
            challenge[5],
            challenge[6],
            challenge[7],
        ],
    ) {
        Ok(resp) => resp,
        Err(PFError::Device(msg)) if msg.contains("locked or protected") && password.is_some() => {
            unlock_if_needed(&card, &info, password)?;
            transmit(
                &card,
                INS_CALCULATE_ALL,
                0x00,
                0x01,
                &[
                    TAG_CHALLENGE,
                    8,
                    challenge[0],
                    challenge[1],
                    challenge[2],
                    challenge[3],
                    challenge[4],
                    challenge[5],
                    challenge[6],
                    challenge[7],
                ],
            )?
        }
        Err(PFError::Device(msg)) if msg.contains("locked or protected") => {
            return Ok(TotpStatus {
                supported: true,
                version: info.version,
                serial: info.serial,
                protected: true,
                pin_retries: info.pin_retries,
                entries: Vec::new(),
            });
        }
        Err(err) => return Err(err),
    };
    let codes = parse_code_map(&calc_resp)?;

    for entry in &mut entries {
        if let Some((digits, code)) = codes.get(&entry.name) {
            entry.digits = *digits;
            if !code.is_empty() {
                entry.current_code = Some(code.clone());
            }
        }
    }

    Ok(TotpStatus {
        supported: true,
        version: info.version,
        serial: info.serial,
        protected: false,
        pin_retries: info.pin_retries,
        entries,
    })
}

pub fn import_totp_uri(uri: String, password: Option<String>) -> Result<String, PFError> {
    let parsed = parse_otpauth_uri(&uri)?;
    let (card, info) = connect_and_select()?;
    unlock_if_needed(&card, &info, password.as_deref())?;

    let mut data = Vec::with_capacity(parsed.label.len() + parsed.secret.len() + 8);
    data.push(TAG_NAME);
    data.push(parsed.label.len() as u8);
    data.extend_from_slice(parsed.label.as_bytes());
    data.push(TAG_KEY);
    data.push((parsed.secret.len() + 2) as u8);
    data.push(OATH_TYPE_TOTP | parsed.algorithm);
    data.push(parsed.digits);
    data.extend_from_slice(&parsed.secret);

    transmit(&card, INS_PUT, 0x00, 0x00, &data)?;

    let label = if let Some(issuer) = parsed.issuer {
        format!("{} ({})", parsed.label, issuer)
    } else {
        parsed.label
    };
    Ok(format!("Stored TOTP account {}", label))
}

pub fn set_totp_password(current_password: Option<String>, new_password: String) -> Result<String, PFError> {
    let new_password = new_password.trim().to_string();
    if new_password.is_empty() {
        return Err(PFError::Io("OATH password cannot be empty".into()));
    }

    let (card, info) = connect_and_select()?;
    unlock_if_needed(&card, &info, current_password.as_deref())?;

    let salt = info
        .salt
        .as_deref()
        .ok_or_else(|| PFError::Io("Missing OATH salt in select response".into()))?;
    let key = derive_access_key(salt, &new_password);
    let mut oath_key = Vec::with_capacity(1 + key.len());
    oath_key.push(ALG_SHA1);
    oath_key.extend_from_slice(&key);

    let verify_challenge: [u8; 8] = random();
    let verify_response = hmac_sha1(&key, &verify_challenge);

    let mut data = tlv(TAG_KEY, &oath_key)?;
    data.extend_from_slice(&tlv(TAG_CHALLENGE, &verify_challenge)?);
    data.extend_from_slice(&tlv(TAG_RESPONSE, &verify_response)?);

    transmit(&card, INS_SET_CODE, 0x00, 0x00, &data)?;

    Ok("TOTP password updated".into())
}

pub fn rename_totp(
    old_name: String,
    new_name: String,
    password: Option<String>,
) -> Result<String, PFError> {
    let new_name = new_name.trim().to_string();
    if new_name.is_empty() {
        return Err(PFError::Io("New TOTP name cannot be empty".into()));
    }
    if old_name == new_name {
        return Err(PFError::Io("New TOTP name must be different".into()));
    }

    let (card, info) = connect_and_select()?;
    unlock_if_needed(&card, &info, password.as_deref())?;
    let mut data = Vec::with_capacity(old_name.len() + new_name.len() + 4);
    data.push(TAG_NAME);
    data.push(old_name.len() as u8);
    data.extend_from_slice(old_name.as_bytes());
    data.push(TAG_NAME);
    data.push(new_name.len() as u8);
    data.extend_from_slice(new_name.as_bytes());
    transmit(&card, INS_RENAME, 0x00, 0x00, &data)?;
    Ok("TOTP account renamed".into())
}

pub fn delete_totp(name: String, password: Option<String>) -> Result<String, PFError> {
    let (card, info) = connect_and_select()?;
    unlock_if_needed(&card, &info, password.as_deref())?;
    let mut data = Vec::with_capacity(name.len() + 2);
    data.push(TAG_NAME);
    data.push(name.len() as u8);
    data.extend_from_slice(name.as_bytes());
    transmit(&card, INS_DELETE, 0x00, 0x00, &data)?;
    Ok("TOTP account removed".into())
}
