//! PIV applet client — YubiKey-PIV-compatible over CCID.
//!
//! Covers the management surface a config GUI needs: slot/PIN/mgmt status
//! (GET METADATA + GET DATA), PIN/PUK/management-key management, key GENERATE,
//! key import payload construction and PEM/DER parsing, certificate import/export,
//! and factory reset. Raw private-key crypto (sign/decrypt/ECDH) is out of scope;
//! those operations are normally provided by SSH, age, or PKCS#11 clients.
//!
//! Byte layout follows the RS-Key rsk-piv wire spec; the management-gated ops
//! (GENERATE, PUT DATA, SET MGM, SET PIN RETRIES) require a GENERAL AUTHENTICATE
//! mutual-auth first — SELECT resets the session, so auth and the gated op MUST
//! run on the same open [`CcidSession`].

#![allow(dead_code)]

use crate::error::PFError;
use crate::hal::apdu::{Apdu, CLA_CHAIN, CLA_ISO, tlv};
use crate::hal::transport::ccid::CcidSession;
use cbc::cipher::{Block, BlockModeDecrypt, BlockModeEncrypt, KeyIvInit};
use ring::rand::{SecureRandom, SystemRandom};

/// Full PIV AID.
pub const PIV_AID: &[u8] = &[
    0xA0, 0x00, 0x00, 0x03, 0x08, 0x00, 0x00, 0x10, 0x00, 0x01, 0x00,
];

// Instructions.
const INS_VERIFY: u8 = 0x20;
const INS_CHANGE_REF: u8 = 0x24;
const INS_RESET_RETRY: u8 = 0x2C;
const INS_GENERATE: u8 = 0x47;
const INS_GENERAL_AUTH: u8 = 0x87;
const INS_GET_DATA: u8 = 0xCB;
const INS_PUT_DATA: u8 = 0xDB;
const INS_MOVE: u8 = 0xF6;
const INS_GET_METADATA: u8 = 0xF7;
const INS_GET_SERIAL: u8 = 0xF8;
const INS_ATTEST: u8 = 0xF9;
const INS_IMPORT: u8 = 0xFE;
const INS_SET_RETRIES: u8 = 0xFA;
const INS_RESET: u8 = 0xFB;
const INS_GET_VERSION: u8 = 0xFD;
const INS_SET_MGM: u8 = 0xFF;

// Slots / references.
pub const REF_PIN: u8 = 0x80;
pub const REF_PUK: u8 = 0x81;
pub const SLOT_9A: u8 = 0x9A;
pub const SLOT_9C: u8 = 0x9C;
pub const SLOT_9D: u8 = 0x9D;
pub const SLOT_9E: u8 = 0x9E;
pub const SLOT_MGM: u8 = 0x9B;
/// The four primary key slots.
pub const PRIMARY_SLOTS: [u8; 4] = [SLOT_9A, SLOT_9C, SLOT_9D, SLOT_9E];

// Algorithm ids (NON-contiguous RSA ids — a common encoder bug).
pub const ALGO_3DES: u8 = 0x03;
pub const ALGO_RSA3072: u8 = 0x05;
pub const ALGO_RSA1024: u8 = 0x06;
pub const ALGO_RSA2048: u8 = 0x07;
pub const ALGO_AES128: u8 = 0x08;
pub const ALGO_AES192: u8 = 0x0A;
pub const ALGO_AES256: u8 = 0x0C;
pub const ALGO_ECCP256: u8 = 0x11;
pub const ALGO_ECCP384: u8 = 0x14;
pub const ALGO_RSA4096: u8 = 0x16;
pub const ALGO_ED25519: u8 = 0xE0;
pub const ALGO_X25519: u8 = 0xE1;

/// Key algorithms offered in the generate wizard (RSA-1024 omitted — weak and
/// refused under the firmware's fips profile).
pub const GENERATE_ALGOS: &[u8] = &[
    ALGO_ECCP256,
    ALGO_ECCP384,
    ALGO_ED25519,
    ALGO_X25519,
    ALGO_RSA2048,
    ALGO_RSA3072,
    ALGO_RSA4096,
];

// PIN / touch policy + origin.
pub const PIN_POLICY_DEFAULT: u8 = 0;
pub const PIN_POLICY_NEVER: u8 = 1;
pub const PIN_POLICY_ONCE: u8 = 2;
pub const PIN_POLICY_ALWAYS: u8 = 3;
pub const TOUCH_POLICY_DEFAULT: u8 = 0;
pub const TOUCH_POLICY_NEVER: u8 = 1;
pub const TOUCH_POLICY_ALWAYS: u8 = 2;
pub const TOUCH_POLICY_CACHED: u8 = 3;
pub const ORIGIN_GENERATED: u8 = 0x01;
pub const ORIGIN_IMPORTED: u8 = 0x02;

// General-auth TLV tags.
const TAG_DYN_AUTH: u32 = 0x7C;
const TAG_WITNESS: u32 = 0x80;
const TAG_CHALLENGE: u32 = 0x81;
const TAG_RESPONSE: u32 = 0x82;

// Object / template tags.
const TAG_DATA_PATH: u32 = 0x5C;
const TAG_DATA_OBJECT: u32 = 0x53;
const TAG_GEN_TEMPLATE: u32 = 0xAC;
const TAG_GEN_ALGO: u32 = 0x80;
const TAG_PIN_POLICY: u32 = 0x0AA;
const TAG_TOUCH_POLICY: u32 = 0x0AB;

/// The YubiKey factory default 24-byte management key, typed AES-192.
pub const DEFAULT_MGM_KEY: [u8; 24] = [
    1, 2, 3, 4, 5, 6, 7, 8, 1, 2, 3, 4, 5, 6, 7, 8, 1, 2, 3, 4, 5, 6, 7, 8,
];
pub const DEFAULT_PIN: &str = "123456";
pub const DEFAULT_PUK: &str = "12345678";

/// A key slot's metadata (from GET METADATA), or `None` when the slot is empty.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlotMeta {
    pub algo: u8,
    pub pin_policy: u8,
    pub touch_policy: u8,
    pub origin: u8,
}

/// Status of one primary key slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlotStatus {
    pub slot: u8,
    pub meta: Option<SlotMeta>,
    pub has_cert: bool,
}

/// Retry / default status of a PIN or PUK.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RefStatus {
    pub is_default: bool,
    pub total: u8,
    pub left: u8,
}

/// Aggregated PIV card status shown on the screen.
#[derive(Debug, Clone)]
pub struct PivInfo {
    pub version: [u8; 3],
    pub serial: u32,
    pub pin: Option<RefStatus>,
    pub puk: Option<RefStatus>,
    pub mgm_algo: u8,
    pub mgm_default: bool,
    /// The management key is PIN-protected (ykman `--protect`): it isn't typed as
    /// hex; it's fetched from the PRINTED object after a PIN VERIFY.
    pub mgm_protected: bool,
    pub slots: Vec<SlotStatus>,
}

/// Human label for a key algorithm.
pub fn algo_label(algo: u8) -> &'static str {
    match algo {
        ALGO_RSA1024 => "RSA-1024",
        ALGO_RSA2048 => "RSA-2048",
        ALGO_RSA3072 => "RSA-3072",
        ALGO_RSA4096 => "RSA-4096",
        ALGO_ECCP256 => "ECC P-256",
        ALGO_ECCP384 => "ECC P-384",
        ALGO_ED25519 => "Ed25519",
        ALGO_X25519 => "X25519",
        ALGO_3DES => "3DES",
        ALGO_AES128 => "AES-128",
        ALGO_AES192 => "AES-192",
        ALGO_AES256 => "AES-256",
        _ => "Unknown",
    }
}

pub fn slot_label(slot: u8) -> &'static str {
    match slot {
        SLOT_9A => "Authentication (9A)",
        SLOT_9C => "Signature (9C)",
        SLOT_9D => "Key Management (9D)",
        SLOT_9E => "Card Authentication (9E)",
        _ => "Slot",
    }
}

/// Object id (`5F C1 xx`) of a slot's certificate.
pub fn cert_object_id(slot: u8) -> [u8; 3] {
    let low = match slot {
        SLOT_9A => 0x05,
        SLOT_9C => 0x0A,
        SLOT_9D => 0x0B,
        SLOT_9E => 0x01,
        s if (0x82..=0x95).contains(&s) => 0x0D + (s - 0x82), // retired R1..R20 → 5FC10D..5FC120
        _ => 0x05,
    };
    [0x5F, 0xC1, low]
}

// ── SELECT + open ───────────────────────────────────────────────────────────

pub fn open() -> Result<CcidSession, PFError> {
    CcidSession::open(PIV_AID)
}

// ── Simple reads (no auth) ──────────────────────────────────────────────────

pub fn get_version(session: &CcidSession) -> Result<[u8; 3], PFError> {
    let r = session.transceive_full(&Apdu::read(CLA_ISO, INS_GET_VERSION, 0, 0, &[]))?;
    let mut v = [0u8; 3];
    v[..r.len().min(3)].copy_from_slice(&r[..r.len().min(3)]);
    Ok(v)
}

pub fn get_serial(session: &CcidSession) -> Result<u32, PFError> {
    let r = session.transceive_full(&Apdu::read(CLA_ISO, INS_GET_SERIAL, 0, 0, &[]))?;
    if r.len() < 4 {
        return Ok(0);
    }
    Ok(u32::from_be_bytes([r[0], r[1], r[2], r[3]]))
}

/// Raw GET METADATA response for a slot/reference (P1=0, P2=slot).
fn get_metadata(session: &CcidSession, slot: u8) -> Result<Vec<u8>, PFError> {
    session.transceive_full(&Apdu::read(CLA_ISO, INS_GET_METADATA, 0, slot, &[]))
}

pub fn parse_ref_status(resp: &[u8]) -> Option<RefStatus> {
    let is_default = tlv::find(resp, 0x05)
        .map(|v| v.first() == Some(&1))
        .unwrap_or(false);
    let retry = tlv::find(resp, 0x06)?;
    if retry.len() < 2 {
        return None;
    }
    Some(RefStatus {
        is_default,
        total: retry[0],
        left: retry[1],
    })
}

pub fn parse_slot_meta(resp: &[u8]) -> Option<SlotMeta> {
    let algo = *tlv::find(resp, 0x01)?.first()?;
    let policy = tlv::find(resp, 0x02)?;
    let origin = tlv::find(resp, 0x03)
        .and_then(|v| v.first().copied())
        .unwrap_or(0);
    Some(SlotMeta {
        algo,
        pin_policy: *policy.first().unwrap_or(&0),
        touch_policy: *policy.get(1).unwrap_or(&0),
        origin,
    })
}

/// Read a data object (certificate/CHUID/…) — returns the unwrapped `53` value.
fn get_data(session: &CcidSession, object_id: &[u8]) -> Result<Vec<u8>, PFError> {
    let mut body = Vec::new();
    tlv::write(&mut body, TAG_DATA_PATH, object_id);
    let resp = session.transceive_full(&Apdu::read(CLA_ISO, INS_GET_DATA, 0x3F, 0xFF, &body))?;
    Ok(tlv::find(&resp, TAG_DATA_OBJECT)
        .map(|v| v.to_vec())
        .unwrap_or(resp))
}

/// Extract the bare DER certificate from a slot's data object (`53{70 …}`).
pub fn cert_der(object: &[u8]) -> Option<Vec<u8>> {
    tlv::find(object, 0x70).map(|v| v.to_vec())
}

/// Full card status. All reads here are unauthenticated.
pub fn read_info(session: &CcidSession) -> Result<PivInfo, PFError> {
    let version = get_version(session).unwrap_or([0; 3]);
    let serial = get_serial(session).unwrap_or(0);
    let pin = get_metadata(session, REF_PIN)
        .ok()
        .and_then(|r| parse_ref_status(&r));
    let puk = get_metadata(session, REF_PUK)
        .ok()
        .and_then(|r| parse_ref_status(&r));

    let (mgm_algo, mgm_default) = match get_metadata(session, SLOT_MGM) {
        Ok(r) => (
            tlv::find(&r, 0x01)
                .and_then(|v| v.first().copied())
                .unwrap_or(ALGO_AES192),
            tlv::find(&r, 0x05)
                .map(|v| v.first() == Some(&1))
                .unwrap_or(false),
        ),
        Err(_) => (ALGO_AES192, false),
    };

    let mut slots = Vec::new();
    for &slot in &PRIMARY_SLOTS {
        let meta = get_metadata(session, slot)
            .ok()
            .and_then(|r| parse_slot_meta(&r));
        let has_cert = get_data(session, &cert_object_id(slot))
            .ok()
            .and_then(|o| cert_der(&o))
            .map(|d| !d.is_empty())
            .unwrap_or(false);
        slots.push(SlotStatus {
            slot,
            meta,
            has_cert,
        });
    }

    let mgm_protected = mgm_is_protected(session);

    Ok(PivInfo {
        version,
        serial,
        pin,
        puk,
        mgm_algo,
        mgm_default,
        mgm_protected,
        slots,
    })
}

// ── PIN-protected management key (ykman --protect) ───────────────────────────

/// ADMIN DATA (PivmanData) object — carries the --protect flag.
const OBJ_ADMIN_DATA: [u8; 3] = [0x5F, 0xFF, 0x00];
/// PRINTED object — holds the PIN-protected management key.
const OBJ_PRINTED: [u8; 3] = [0x5F, 0xC1, 0x09];
const PIVMAN_TAG: u32 = 0x80;
const PIVMAN_FLAGS_TAG: u32 = 0x81;
const PIVMAN_FLAG_MGM_PROTECTED: u8 = 0x02;
const PROTECTED_OUTER_TAG: u32 = 0x88;
const PROTECTED_MGM_TAG: u32 = 0x89;

/// Whether the management key is PIN-protected. Reads ADMIN DATA (`5FFF00`,
/// `80 { 81 <flags> }`) and tests the mgm-protected flag bit; a missing/malformed
/// object reads as not protected (fail-open to the hex path).
fn mgm_is_protected(session: &CcidSession) -> bool {
    let Ok(obj) = get_data(session, &OBJ_ADMIN_DATA) else {
        return false;
    };
    let inner = tlv::find(&obj, 0x53).unwrap_or(&obj);
    let Some(pivman) = tlv::find(inner, PIVMAN_TAG) else {
        return false;
    };
    tlv::find(pivman, PIVMAN_FLAGS_TAG)
        .and_then(|f| f.first().copied())
        .map(|f| f & PIVMAN_FLAG_MGM_PROTECTED != 0)
        .unwrap_or(false)
}

/// Fetch the PIN-protected management key: VERIFY the PIN, then GET DATA PRINTED
/// (`53 { 88 { 89 <key> } }`). Only valid on a `--protect`'d card.
pub fn read_protected_mgm(session: &CcidSession, pin: &str) -> Result<Vec<u8>, PFError> {
    verify_pin(session, pin)?;
    let obj = get_data(session, &OBJ_PRINTED)?;
    parse_protected_mgm(&obj)
        .ok_or_else(|| PFError::Device("No PIN-protected management key on this card".into()))
}

/// Parse the PRINTED object `53 { 88 { 89 <key> } }` to the raw key bytes.
fn parse_protected_mgm(obj: &[u8]) -> Option<Vec<u8>> {
    let inner = tlv::find(obj, 0x53).unwrap_or(obj);
    let protected = tlv::find(inner, PROTECTED_OUTER_TAG)?;
    let key = tlv::find(protected, PROTECTED_MGM_TAG)?;
    matches!(key.len(), 16 | 24 | 32).then(|| key.to_vec())
}

/// The AES management-key algorithm id for a key of `len` bytes.
pub fn mgm_algo_for_len(len: usize) -> u8 {
    match len {
        16 => ALGO_AES128,
        32 => ALGO_AES256,
        _ => ALGO_AES192,
    }
}

/// Export a slot's certificate DER (empty error if none).
pub fn export_cert(session: &CcidSession, slot: u8) -> Result<Vec<u8>, PFError> {
    let obj = get_data(session, &cert_object_id(slot))?;
    cert_der(&obj)
        .filter(|d| !d.is_empty())
        .ok_or_else(|| PFError::Device("No certificate in this slot".into()))
}

// ── PIN / PUK management (no session auth, burns retries) ────────────────────

/// PIN/PUK padded to 8 bytes with `0xFF`.
fn pad8(s: &[u8]) -> [u8; 8] {
    let mut p = [0xFFu8; 8];
    let n = s.len().min(8);
    p[..n].copy_from_slice(&s[..n]);
    p
}

pub fn verify_pin(session: &CcidSession, pin: &str) -> Result<(), PFError> {
    let block = pad8(pin.as_bytes());
    session.transceive_full(&Apdu::write(CLA_ISO, INS_VERIFY, 0x00, REF_PIN, &block))?;
    Ok(())
}

/// The 16-byte CHANGE REFERENCE / RESET RETRY body: current then new secret,
/// each the standard 8-byte PIV block (`0xFF`-padded). The firmware stores every
/// verifier over its 8-byte-padded form and splits the command at 8, so the
/// current block must be a full 8 — an unpadded one mis-verifies and burns a
/// retry even when the secret is correct.
fn change_ref_body(current: &str, new: &str) -> Vec<u8> {
    let mut body = pad8(current.as_bytes()).to_vec();
    body.extend_from_slice(&pad8(new.as_bytes()));
    body
}

/// Change PIN (`ref`=REF_PIN) or PUK (`ref`=REF_PUK).
pub fn change_ref(
    session: &CcidSession,
    reference: u8,
    old: &str,
    new: &str,
) -> Result<(), PFError> {
    let body = change_ref_body(old, new);
    session.transceive_full(&Apdu::write(
        CLA_ISO,
        INS_CHANGE_REF,
        0x00,
        reference,
        &body,
    ))?;
    Ok(())
}

/// Unblock the PIN using the PUK (RESET RETRY COUNTER).
pub fn unblock_pin(session: &CcidSession, puk: &str, new_pin: &str) -> Result<(), PFError> {
    let body = change_ref_body(puk, new_pin);
    session.transceive_full(&Apdu::write(CLA_ISO, INS_RESET_RETRY, 0x00, REF_PIN, &body))?;
    Ok(())
}

// ── Management mutual-auth (0x87) ───────────────────────────────────────────

/// One-block AES ECB via CBC with a zero IV (E/D of a single block under CBC and
/// IV=0 is exactly ECB). Reuses the `aes`/`cbc` crates already in the tree.
fn aes_ecb(key: &[u8], block: &mut [u8; 16], encrypt: bool) -> Result<(), PFError> {
    let iv = [0u8; 16];
    macro_rules! run {
        ($cipher:ty) => {{
            let mut b = Block::<$cipher>::try_from(&block[..])
                .map_err(|_| PFError::Device("AES block size".into()))?;
            if encrypt {
                cbc::Encryptor::<$cipher>::new_from_slices(key, &iv)
                    .map_err(|_| PFError::Device("Bad management key".into()))?
                    .encrypt_block(&mut b);
            } else {
                cbc::Decryptor::<$cipher>::new_from_slices(key, &iv)
                    .map_err(|_| PFError::Device("Bad management key".into()))?
                    .decrypt_block(&mut b);
            }
            block.copy_from_slice(b.as_slice());
        }};
    }
    match key.len() {
        16 => run!(aes::Aes128),
        24 => run!(aes::Aes192),
        32 => run!(aes::Aes256),
        _ => {
            return Err(PFError::Device(
                "Management key must be AES (16/24/32 bytes)".into(),
            ));
        }
    }
    Ok(())
}

/// Authenticate the management key on this session (witness mutual-auth). Must
/// precede any mgmt-gated op on the same open session. AES keys only (3DES is
/// not implemented — the default key is AES-192).
pub fn authenticate_mgm(session: &CcidSession, key: &[u8], algo: u8) -> Result<(), PFError> {
    if algo == ALGO_3DES {
        // A 3DES 9B key returns an 8-byte witness this AES-only path can't
        // process; fail with the real reason, not a generic "Bad witness".
        return Err(PFError::Device(
            "3DES management keys are not supported — set an AES management key first".into(),
        ));
    }
    // Step 1: request the encrypted witness.
    let mut req1 = Vec::new();
    tlv::write(&mut req1, TAG_DYN_AUTH, &{
        let mut inner = Vec::new();
        tlv::write(&mut inner, TAG_WITNESS, &[]);
        inner
    });
    let r1 = session.transceive_full(&Apdu::read(
        CLA_ISO,
        INS_GENERAL_AUTH,
        algo,
        SLOT_MGM,
        &req1,
    ))?;
    let outer =
        tlv::find(&r1, TAG_DYN_AUTH).ok_or_else(|| PFError::Device("No 7C in auth".into()))?;
    let enc_witness = tlv::find(outer, TAG_WITNESS)
        .filter(|w| w.len() == 16)
        .ok_or_else(|| PFError::Device("Bad witness".into()))?;
    let mut witness = [0u8; 16];
    witness.copy_from_slice(enc_witness);
    aes_ecb(key, &mut witness, false)?; // decrypt → R

    // Step 2: return the decrypted witness + our own challenge.
    let mut challenge = [0u8; 16];
    SystemRandom::new()
        .fill(&mut challenge)
        .map_err(|_| PFError::Device("RNG failure".into()))?;
    let mut inner = Vec::new();
    tlv::write(&mut inner, TAG_WITNESS, &witness);
    tlv::write(&mut inner, TAG_CHALLENGE, &challenge);
    let mut req2 = Vec::new();
    tlv::write(&mut req2, TAG_DYN_AUTH, &inner);
    let r2 = session.transceive_full(&Apdu::read(
        CLA_ISO,
        INS_GENERAL_AUTH,
        algo,
        SLOT_MGM,
        &req2,
    ))?;

    // Verify the card's response encrypts our challenge (mutual auth).
    let outer2 =
        tlv::find(&r2, TAG_DYN_AUTH).ok_or_else(|| PFError::Device("No 7C in auth-2".into()))?;
    let resp = tlv::find(outer2, TAG_RESPONSE)
        .filter(|r| r.len() == 16)
        .ok_or_else(|| PFError::Device("Bad auth response".into()))?;
    let mut expect = challenge;
    aes_ecb(key, &mut expect, true)?;
    if expect.as_slice() != resp {
        return Err(PFError::Device(
            "Management-key authentication failed".into(),
        ));
    }
    Ok(())
}

// ── Management-gated ops (require prior authenticate_mgm on same session) ─────

/// Build the GENERATE command template `AC{80 01 algo [AA 01 pp][AB 01 tp]}`.
pub fn generate_template(algo: u8, pin_policy: u8, touch_policy: u8) -> Vec<u8> {
    let mut inner = Vec::new();
    tlv::write(&mut inner, TAG_GEN_ALGO, &[algo]);
    if pin_policy != PIN_POLICY_DEFAULT {
        tlv::write(&mut inner, TAG_PIN_POLICY, &[pin_policy]);
    }
    if touch_policy != TOUCH_POLICY_DEFAULT {
        tlv::write(&mut inner, TAG_TOUCH_POLICY, &[touch_policy]);
    }
    let mut out = Vec::new();
    tlv::write(&mut out, TAG_GEN_TEMPLATE, &inner);
    out
}

/// Generate a key in `slot`. Returns the raw `7F49` public-key response.
pub fn generate(
    session: &CcidSession,
    slot: u8,
    algo: u8,
    pin_policy: u8,
    touch_policy: u8,
) -> Result<Vec<u8>, PFError> {
    let body = generate_template(algo, pin_policy, touch_policy);
    session.transceive_full(&Apdu::read(CLA_ISO, INS_GENERATE, 0x00, slot, &body))
}

/// Wrap a DER certificate into a PIV data object (`70 <der> 71 01 00 FE 00`).
pub fn wrap_cert_object(der: &[u8]) -> Vec<u8> {
    let mut inner = Vec::new();
    tlv::write(&mut inner, 0x70, der);
    tlv::write(&mut inner, 0x71, &[0x00]);
    tlv::write(&mut inner, 0xFE, &[]);
    inner
}

/// PUT DATA (mgmt-gated). Uses command chaining for large objects.
pub fn put_data(session: &CcidSession, object_id: &[u8], data: &[u8]) -> Result<(), PFError> {
    let mut body = Vec::new();
    tlv::write(&mut body, TAG_DATA_PATH, object_id);
    tlv::write(&mut body, TAG_DATA_OBJECT, data);
    let apdu = Apdu {
        cla: CLA_ISO,
        ins: INS_PUT_DATA,
        p1: 0x3F,
        p2: 0xFF,
        data: body,
        le: None,
    };
    session.send_chained(&apdu)?;
    Ok(())
}

/// Import a certificate into a slot (mgmt-gated).
pub fn import_cert(session: &CcidSession, slot: u8, der: &[u8]) -> Result<(), PFError> {
    put_data(session, &cert_object_id(slot), &wrap_cert_object(der))
}

/// Delete a slot's certificate (mgmt-gated) — an empty data object.
pub fn delete_cert(session: &CcidSession, slot: u8) -> Result<(), PFError> {
    put_data(session, &cert_object_id(slot), &[])
}

/// A random management key of the length for `algo` (AES-128/192/256).
pub fn random_key(algo: u8) -> Result<Vec<u8>, PFError> {
    let len = match algo {
        ALGO_AES128 => 16,
        ALGO_AES256 => 32,
        _ => 24,
    };
    let mut k = vec![0u8; len];
    SystemRandom::new()
        .fill(&mut k)
        .map_err(|_| PFError::Device("RNG failure".into()))?;
    Ok(k)
}

/// Set a new management key (mgmt-gated). `touch` sets touch-always.
pub fn set_mgm(session: &CcidSession, algo: u8, key: &[u8], touch: bool) -> Result<(), PFError> {
    let mut body = vec![algo, SLOT_MGM, key.len() as u8];
    body.extend_from_slice(key);
    let p2 = if touch { 0xFE } else { 0xFF };
    session.transceive_full(&Apdu::write(CLA_ISO, INS_SET_MGM, 0xFF, p2, &body))?;
    Ok(())
}

/// Set PIN/PUK retry counts (requires mgmt AND PIN; resets PIN/PUK to defaults).
pub fn set_retries(session: &CcidSession, pin_tries: u8, puk_tries: u8) -> Result<(), PFError> {
    session.transceive_full(&Apdu::read(
        CLA_ISO,
        INS_SET_RETRIES,
        pin_tries,
        puk_tries,
        &[],
    ))?;
    Ok(())
}

/// Attest a slot's (generated) key — returns the bare DER attestation cert.
pub fn attest(session: &CcidSession, slot: u8) -> Result<Vec<u8>, PFError> {
    session.transceive_full(&Apdu::read(CLA_ISO, INS_ATTEST, slot, 0, &[]))
}

/// Move a key from `src` to `dst` slot (mgmt-gated).
pub fn move_key(session: &CcidSession, src: u8, dst: u8) -> Result<(), PFError> {
    session.transceive_full(&Apdu::write(CLA_ISO, INS_MOVE, dst, src, &[]))?;
    Ok(())
}

/// Delete a slot's key entirely (mgmt-gated) — MOVE with the delete sentinel.
pub fn delete_key(session: &CcidSession, slot: u8) -> Result<(), PFError> {
    session.transceive_full(&Apdu::write(CLA_ISO, INS_MOVE, 0xFF, slot, &[]))?;
    Ok(())
}

/// Import pre-built key-material TLVs into a slot (mgmt-gated). `algo` is P1.
pub fn import_key(
    session: &CcidSession,
    slot: u8,
    algo: u8,
    material: &[u8],
) -> Result<(), PFError> {
    let apdu = Apdu {
        cla: CLA_ISO,
        ins: INS_IMPORT,
        p1: algo,
        p2: slot,
        data: material.to_vec(),
        le: None,
    };
    session.send_chained(&apdu)?;
    Ok(())
}

// ── PEM/DER private-key parsing (for IMPORT) ────────────────────────────────

const OID_RSA: &[u8] = &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x01, 0x01];
const OID_EC: &[u8] = &[0x2A, 0x86, 0x48, 0xCE, 0x3D, 0x02, 0x01];
const OID_P256: &[u8] = &[0x2A, 0x86, 0x48, 0xCE, 0x3D, 0x03, 0x01, 0x07];
const OID_P384: &[u8] = &[0x2B, 0x81, 0x04, 0x00, 0x22];
const OID_ED25519: &[u8] = &[0x2B, 0x65, 0x70];
const OID_X25519: &[u8] = &[0x2B, 0x65, 0x6E];

fn der_len(d: &[u8], p: &mut usize) -> Option<usize> {
    let first = *d.get(*p)?;
    *p += 1;
    if first < 0x80 {
        return Some(first as usize);
    }
    let n = (first & 0x7F) as usize;
    if n == 0 || n > 4 {
        return None;
    }
    let mut len = 0usize;
    for _ in 0..n {
        len = (len << 8) | *d.get(*p)? as usize;
        *p += 1;
    }
    Some(len)
}

/// Read one DER TLV (single-byte tags only — sufficient for key structures).
fn der_tlv<'a>(d: &'a [u8], p: &mut usize) -> Option<(u8, &'a [u8])> {
    let tag = *d.get(*p)?;
    *p += 1;
    let len = der_len(d, p)?;
    let end = p.checked_add(len)?;
    if end > d.len() {
        return None;
    }
    let v = &d[*p..end];
    *p = end;
    Some((tag, v))
}

/// Strip the leading sign byte from a DER INTEGER's magnitude.
fn int_bytes(v: &[u8]) -> &[u8] {
    if v.first() == Some(&0) && v.len() > 1 {
        &v[1..]
    } else {
        v
    }
}

/// Left-pad a scalar to a fixed field size.
fn pad_left(v: &[u8], n: usize) -> Vec<u8> {
    let v = int_bytes(v);
    if v.len() >= n {
        return v[v.len() - n..].to_vec();
    }
    let mut out = vec![0u8; n];
    out[n - v.len()..].copy_from_slice(v);
    out
}

/// Decode a PEM or DER private key into `(algo_id, IMPORT command data)`.
pub fn parse_private_key(input: &[u8]) -> Result<(u8, Vec<u8>), String> {
    let (der, label) = strip_pem(input)?;
    match label.as_deref() {
        Some("RSA") => parse_rsa(&der),
        Some("EC") => parse_ec(&der, None),
        _ => parse_pkcs8(&der),
    }
}

fn strip_pem(input: &[u8]) -> Result<(Vec<u8>, Option<String>), String> {
    let text = std::str::from_utf8(input).unwrap_or("");
    let Some(begin) = text.find("-----BEGIN ") else {
        return Ok((input.to_vec(), None));
    };
    let after = &text[begin + 11..];
    let label_end = after.find("-----").ok_or("Malformed PEM header")?;
    let label = &after[..label_end];
    let body = &after[label_end + 5..];
    let end = body.find("-----END").ok_or("Malformed PEM footer")?;
    let b64: String = body[..end].chars().filter(|c| !c.is_whitespace()).collect();
    use base64::Engine;
    let der = base64::engine::general_purpose::STANDARD
        .decode(b64.as_bytes())
        .map_err(|_| "Invalid base64 in PEM")?;
    let kind = if label.contains("RSA") {
        "RSA"
    } else if label.contains("EC") {
        "EC"
    } else {
        "PKCS8"
    };
    Ok((der, Some(kind.to_string())))
}

fn parse_pkcs8(der: &[u8]) -> Result<(u8, Vec<u8>), String> {
    let mut p = 0;
    let (_, seq) = der_tlv(der, &mut p).ok_or("Not a DER SEQUENCE")?;
    let mut q = 0;
    der_tlv(seq, &mut q).ok_or("PKCS8 version")?; // version
    let (_, alg) = der_tlv(seq, &mut q).ok_or("PKCS8 algorithm")?;
    let (_, priv_octets) = der_tlv(seq, &mut q).ok_or("PKCS8 privateKey")?;

    let mut a = 0;
    let (_, oid) = der_tlv(alg, &mut a).ok_or("PKCS8 OID")?;
    let params = der_tlv(alg, &mut a).map(|(_, v)| v);

    if oid == OID_RSA {
        parse_rsa(priv_octets)
    } else if oid == OID_EC {
        parse_ec(priv_octets, params)
    } else if oid == OID_ED25519 || oid == OID_X25519 {
        // CurvePrivateKey ::= OCTET STRING wrapping the 32-byte seed/scalar.
        let mut s = 0;
        let (_, seed) = der_tlv(priv_octets, &mut s).ok_or("Curve25519 key")?;
        if seed.len() != 32 {
            return Err("Curve25519 key must be 32 bytes".into());
        }
        let (algo, tag) = if oid == OID_ED25519 {
            (ALGO_ED25519, 0x07u32)
        } else {
            (ALGO_X25519, 0x08u32)
        };
        let mut data = Vec::new();
        tlv::write(&mut data, tag, seed);
        Ok((algo, data))
    } else {
        Err("Unsupported key algorithm".into())
    }
}

fn parse_rsa(der: &[u8]) -> Result<(u8, Vec<u8>), String> {
    let mut p = 0;
    let (_, seq) = der_tlv(der, &mut p).ok_or("RSA SEQUENCE")?;
    let mut q = 0;
    let mut ints: Vec<&[u8]> = Vec::new();
    while ints.len() < 6 {
        match der_tlv(seq, &mut q) {
            Some((0x02, v)) => ints.push(int_bytes(v)),
            _ => break,
        }
    }
    if ints.len() < 6 {
        return Err("Not an RSA private key (need n,e,d,p,q)".into());
    }
    // ints = [version, n, e, d, p, q]
    let algo = match ints[1].len() {
        128 => ALGO_RSA1024,
        256 => ALGO_RSA2048,
        384 => ALGO_RSA3072,
        512 => ALGO_RSA4096,
        _ => return Err("Unsupported RSA key size".into()),
    };
    let mut data = Vec::new();
    tlv::write(&mut data, 0x01, ints[4]); // prime P
    tlv::write(&mut data, 0x02, ints[5]); // prime Q
    Ok((algo, data))
}

fn parse_ec(der: &[u8], pkcs8_params: Option<&[u8]>) -> Result<(u8, Vec<u8>), String> {
    let mut p = 0;
    let (_, seq) = der_tlv(der, &mut p).ok_or("EC SEQUENCE")?;
    let mut q = 0;
    der_tlv(seq, &mut q).ok_or("EC version")?; // version
    let (_, scalar) = der_tlv(seq, &mut q).ok_or("EC privateKey")?;

    // Curve OID: from PKCS8 algorithm params, else the [0] tagged field in SEC1.
    let curve = pkcs8_params
        .and_then(|pp| {
            let mut cp = 0;
            der_tlv(pp, &mut cp).map(|(_, v)| v)
        })
        .or_else(|| {
            let mut r = q;
            while let Some((tag, v)) = der_tlv(seq, &mut r) {
                if tag == 0xA0 {
                    let mut cp = 0;
                    return der_tlv(v, &mut cp).map(|(_, o)| o);
                }
            }
            None
        });
    let (algo, field) = match curve {
        Some(c) if c == OID_P256 => (ALGO_ECCP256, 32),
        Some(c) if c == OID_P384 => (ALGO_ECCP384, 48),
        _ => return Err("Unsupported EC curve (only P-256/P-384)".into()),
    };
    let mut data = Vec::new();
    tlv::write(&mut data, 0x06, &pad_left(scalar, field));
    Ok((algo, data))
}

// ── Reset (block both, then factory reset) ──────────────────────────────────

/// Upper bound on deliberate bad guesses when blocking a reference for reset.
/// SET PIN RETRIES accepts up to 255, so a fixed-10 loop failed to block a card
/// whose retry counter was raised above 10 (then RESET returns 6A80).
const PIN_BLOCK_MAX_TRIES: usize = 256;

/// Factory-reset the PIV applet. The device only permits this once both PIN and
/// PUK are blocked, so this blocks them with deliberate bad guesses first.
pub fn reset(session: &CcidSession) -> Result<(), PFError> {
    // Block PIN: wrong VERIFY until 0x6983.
    for _ in 0..PIN_BLOCK_MAX_TRIES {
        let bad = pad8(b"00000000");
        match session.transceive(&Apdu::write(CLA_ISO, INS_VERIFY, 0x00, REF_PIN, &bad)) {
            Ok((_, sw)) if sw.0 == 0x6983 => break,
            Ok(_) => continue,
            Err(e) => return Err(e),
        }
    }
    // Block PUK: wrong RESET RETRY until 0x6983.
    for _ in 0..PIN_BLOCK_MAX_TRIES {
        let mut bad = pad8(b"00000000").to_vec();
        bad.extend_from_slice(&pad8(b"00000000"));
        match session.transceive(&Apdu::write(CLA_ISO, INS_RESET_RETRY, 0x00, REF_PIN, &bad)) {
            Ok((_, sw)) if sw.0 == 0x6983 => break,
            Ok(_) => continue,
            Err(e) => return Err(e),
        }
    }
    session.transceive_full(&Apdu::write(CLA_ISO, INS_RESET, 0x00, 0x00, &[]))?;
    Ok(())
}

// keep the chaining class referenced for the module's documented use.
const _: u8 = CLA_CHAIN;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cert_object_ids() {
        assert_eq!(cert_object_id(SLOT_9A), [0x5F, 0xC1, 0x05]);
        assert_eq!(cert_object_id(SLOT_9C), [0x5F, 0xC1, 0x0A]);
        assert_eq!(cert_object_id(SLOT_9D), [0x5F, 0xC1, 0x0B]);
        assert_eq!(cert_object_id(SLOT_9E), [0x5F, 0xC1, 0x01]);
        assert_eq!(cert_object_id(0x82), [0x5F, 0xC1, 0x0D]); // retired 1
    }

    #[test]
    fn parses_protected_mgm_key() {
        // 53 { 88 { 89 <24-byte key> } }
        let key = [0xABu8; 24];
        let mut inner = Vec::new();
        tlv::write(&mut inner, 0x89, &key);
        let mut protected = Vec::new();
        tlv::write(&mut protected, 0x88, &inner);
        let mut obj = Vec::new();
        tlv::write(&mut obj, 0x53, &protected);
        assert_eq!(parse_protected_mgm(&obj), Some(key.to_vec()));
        // Also accepts a bare (un-53-wrapped) body.
        assert_eq!(parse_protected_mgm(&protected), Some(key.to_vec()));
        // A 7-byte "key" is neither 16/24/32 → rejected.
        assert_eq!(
            parse_protected_mgm(&[0x88, 0x09, 0x89, 0x07, 0, 0, 0, 0, 0, 0, 0]),
            None
        );
        assert_eq!(mgm_algo_for_len(16), ALGO_AES128);
        assert_eq!(mgm_algo_for_len(24), ALGO_AES192);
        assert_eq!(mgm_algo_for_len(32), ALGO_AES256);
    }

    #[test]
    fn change_ref_body_is_two_padded_blocks() {
        // Standard 16-byte PIV CHANGE REFERENCE: current(8) ++ new(8), both
        // 0xFF-padded. The current block MUST be a full 8 — the firmware splits
        // the command at the stored length (always 8), so a 6-byte current would
        // mis-verify and burn a retry.
        let body = change_ref_body("123456", "87654321");
        assert_eq!(body.len(), 16);
        assert_eq!(
            &body[..8],
            &[0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0xFF, 0xFF]
        );
        assert_eq!(
            &body[8..],
            &[0x38, 0x37, 0x36, 0x35, 0x34, 0x33, 0x32, 0x31]
        );
    }

    #[test]
    fn generate_template_bytes() {
        // AC { 80 01 11 AA 01 03 AB 01 02 } for P-256, PIN-always, touch-always.
        let t = generate_template(ALGO_ECCP256, PIN_POLICY_ALWAYS, TOUCH_POLICY_ALWAYS);
        assert_eq!(
            t,
            vec![
                0xAC, 0x09, 0x80, 0x01, 0x11, 0xAA, 0x01, 0x03, 0xAB, 0x01, 0x02
            ]
        );
        // Default policies are omitted.
        let t2 = generate_template(ALGO_RSA2048, PIN_POLICY_DEFAULT, TOUCH_POLICY_DEFAULT);
        assert_eq!(t2, vec![0xAC, 0x03, 0x80, 0x01, 0x07]);
    }

    #[test]
    fn cert_wrap_and_unwrap() {
        let der = [0x30, 0x03, 0x01, 0x02, 0x03];
        let obj = wrap_cert_object(&der);
        // 70 05 <der> 71 01 00 FE 00
        assert_eq!(&obj[..2], &[0x70, 0x05]);
        assert_eq!(cert_der(&obj).unwrap(), der);
    }

    #[test]
    fn parses_metadata() {
        // Slot meta: 01 01 11, 02 02 03 02, 03 01 01
        let mut m = Vec::new();
        tlv::write(&mut m, 0x01, &[ALGO_ECCP256]);
        tlv::write(&mut m, 0x02, &[PIN_POLICY_ALWAYS, TOUCH_POLICY_ALWAYS]);
        tlv::write(&mut m, 0x03, &[ORIGIN_GENERATED]);
        let meta = parse_slot_meta(&m).unwrap();
        assert_eq!(meta.algo, ALGO_ECCP256);
        assert_eq!(meta.pin_policy, PIN_POLICY_ALWAYS);
        assert_eq!(meta.origin, ORIGIN_GENERATED);

        let mut r = Vec::new();
        tlv::write(&mut r, 0x05, &[0x00]);
        tlv::write(&mut r, 0x06, &[3, 2]);
        let rs = parse_ref_status(&r).unwrap();
        assert!(!rs.is_default);
        assert_eq!((rs.total, rs.left), (3, 2));
    }

    #[test]
    fn pin_padding() {
        assert_eq!(
            pad8(b"123456"),
            [0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0xFF, 0xFF]
        );
        assert_eq!(
            pad8(b"12345678"),
            [0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38]
        );
    }

    #[test]
    fn parse_ed25519_pkcs8_rfc8410() {
        // RFC 8410 §10.3 example Ed25519 private key (PEM decoded).
        let seed = [
            0xd4, 0xee, 0x72, 0xdb, 0xf9, 0x13, 0x58, 0x4a, 0xd5, 0xb6, 0xd8, 0xf1, 0xf7, 0x69,
            0xf8, 0xad, 0x3a, 0xfe, 0x7c, 0x28, 0xcb, 0xf1, 0xd4, 0xfb, 0xe0, 0x97, 0xa8, 0x8f,
            0x44, 0x75, 0x58, 0x42,
        ];
        let mut der = vec![
            0x30, 0x2e, 0x02, 0x01, 0x00, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x04, 0x22,
            0x04, 0x20,
        ];
        der.extend_from_slice(&seed);
        let (algo, data) = parse_pkcs8(&der).unwrap();
        assert_eq!(algo, ALGO_ED25519);
        // IMPORT data = 07 20 <seed>
        assert_eq!(data[0], 0x07);
        assert_eq!(data[1], 0x20);
        assert_eq!(&data[2..], &seed);
    }

    #[test]
    fn parse_rsa_pkcs1_extracts_primes() {
        let mut inner = Vec::new();
        tlv::write(&mut inner, 0x02, &[0x00]); // version
        tlv::write(&mut inner, 0x02, &[0x11; 128]); // n (128 B → RSA-1024)
        tlv::write(&mut inner, 0x02, &[0x01, 0x00, 0x01]); // e
        tlv::write(&mut inner, 0x02, &[0x11; 64]); // d
        tlv::write(&mut inner, 0x02, &[0x22; 64]); // p
        tlv::write(&mut inner, 0x02, &[0x33; 64]); // q
        let mut der = Vec::new();
        tlv::write(&mut der, 0x30, &inner);
        let (algo, data) = parse_rsa(&der).unwrap();
        assert_eq!(algo, ALGO_RSA1024);
        // 01 40 <p> 02 40 <q>
        assert_eq!(&data[..2], &[0x01, 0x40]);
        assert_eq!(&data[2..66], &[0x22; 64]);
        assert_eq!(&data[66..68], &[0x02, 0x40]);
        assert_eq!(&data[68..], &[0x33; 64]);
    }

    #[test]
    fn parse_ec_sec1_p256() {
        let mut curve = Vec::new();
        tlv::write(&mut curve, 0x06, OID_P256);
        let mut inner = Vec::new();
        tlv::write(&mut inner, 0x02, &[0x01]); // version
        tlv::write(&mut inner, 0x04, &[0xAB; 32]); // scalar
        tlv::write(&mut inner, 0xA0, &curve); // [0] curve params
        let mut der = Vec::new();
        tlv::write(&mut der, 0x30, &inner);
        let (algo, data) = parse_ec(&der, None).unwrap();
        assert_eq!(algo, ALGO_ECCP256);
        assert_eq!(data[0], 0x06); // scalar tag
        assert_eq!(data[1], 0x20);
        assert_eq!(&data[2..], &[0xAB; 32]);
    }

    #[test]
    fn aes_ecb_fips197_vector() {
        // FIPS-197 AES-128 ECB known-answer.
        let key: [u8; 16] = [
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
            0x0e, 0x0f,
        ];
        let mut block: [u8; 16] = [
            0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd,
            0xee, 0xff,
        ];
        aes_ecb(&key, &mut block, true).unwrap();
        assert_eq!(
            block,
            [
                0x69, 0xc4, 0xe0, 0xd8, 0x6a, 0x7b, 0x04, 0x30, 0xd8, 0xcd, 0xb7, 0x80, 0x70, 0xb4,
                0xc5, 0x5a
            ]
        );
        aes_ecb(&key, &mut block, false).unwrap();
        assert_eq!(block[0], 0x00);
        assert_eq!(block[15], 0xff);
    }
}
