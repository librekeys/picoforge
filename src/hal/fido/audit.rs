//! Tamper-evident audit journal — parsing, hash-chain folding, and checkpoint
//! signature verification.
//!
//! The firmware keeps a flash ring of 20-byte security events, hash-chained from
//! an "epoch" accumulator that absorbs evicted history. A checkpoint is the chain
//! head signed with an ECDSA P-256 key derived from the device's OTP DEVK, over a
//! host-chosen challenge — so a caller can prove the log is authentic and that it
//! is talking to the enrolled device. These are pure functions (no transport), so
//! they are host-tested; the CBOR field extraction and I/O live in the parent.

use ring::{digest, signature};

/// Bytes per journal entry on the wire.
pub const ENTRY_LEN: usize = 20;

/// Domain-separation tag prefixing the signed checkpoint message.
const CKPT_TAG: &[u8] = b"RSK-AUDIT-CKPT-v1";

/// `EV_RESET` — the factory-reset event (offboard receipts require it present).
pub const EVT_RESET: u8 = 0x04;

/// One decoded journal entry. The wire record is 20 bytes; the parser reads the
/// sequence, uptime, event, auxiliary byte, and eight-byte detail field from
/// bytes `0..18` and currently ignores the final two bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditEntry {
    pub seq: u32,
    pub uptime_ms: u32,
    pub event: u8,
    pub aux: u8,
    pub detail: [u8; 8],
}

impl AuditEntry {
    pub fn event_label(&self) -> String {
        match event_name(self.event) {
            Some(name) => name.to_string(),
            None => format!("0x{:02x}", self.event),
        }
    }
    pub fn uptime_s(&self) -> f64 {
        self.uptime_ms as f64 / 1000.0
    }
    pub fn detail_hex(&self) -> String {
        hex::encode(self.detail)
    }
}

/// A journal window plus its locally recomputed chain head.
#[derive(Debug, Clone)]
pub struct AuditJournal {
    /// First live sequence number (`start` older entries are folded into epoch).
    pub start: u32,
    /// One past the last sequence number.
    pub seq_next: u32,
    pub epoch: [u8; 32],
    /// Chain head folded over the exported window (`fold_chain`).
    pub head: [u8; 32],
    pub entries: Vec<AuditEntry>,
}

/// Result of `audit verify`: the journal plus the checkpoint outcome. Also
/// serves the identity check (`inventory verify`) via `expected_match`.
#[derive(Debug, Clone)]
pub struct AuditVerification {
    pub journal: AuditJournal,
    /// The DEVK checkpoint signature verified against the returned public key.
    pub signature_ok: bool,
    /// The signed head equals the locally folded head (no read/sign race).
    pub head_matches: bool,
    pub pubkey_hex: String,
    pub fingerprint: String,
    pub seq_signed: u32,
    /// The signed chain head (hex) and DER signature (hex) — for offboard receipts.
    pub signed_head_hex: String,
    pub signature_hex: String,
    /// `Some(true/false)` when an expected key was supplied, else `None`.
    pub expected_match: Option<bool>,
}

impl AuditVerification {
    /// Whether the journal is authentic: signature + head bind, and any pinned
    /// key matches.
    pub fn authentic(&self) -> bool {
        self.signature_ok && self.head_matches && self.expected_match != Some(false)
    }
}

/// Human-readable name for an event id, or `None` for unknown ids.
pub fn event_name(event: u8) -> Option<&'static str> {
    Some(match event {
        0x01 => "BOOT",
        0x02 => "MAKE_CREDENTIAL",
        0x03 => "GET_ASSERTION",
        0x04 => "RESET",
        0x05 => "PIN_SET",
        0x06 => "PIN_CHANGE",
        0x07 => "PIN_LOCKOUT",
        0x08 => "CFG_MIN_PIN",
        0x09 => "CFG_ENTERPRISE_ATT",
        0x0A => "LOCK_ENGAGE",
        0x0B => "LOCK_RELEASE",
        0x0C => "BACKUP_EXPORT",
        0x0D => "BACKUP_LOAD",
        0x0E => "BACKUP_FINALIZE",
        0x0F => "U2F_REGISTER",
        0x10 => "U2F_AUTH",
        0x11 => "CHECKPOINT",
        0x12 => "ATT_IMPORT",
        0x13 => "ATT_CLEAR",
        0x14 => "CFG_ALWAYS_UV",
        0x15 => "CONFIG_WRITE",
        _ => return None,
    })
}

/// Parse a concatenation of 20-byte entries. Trailing bytes shorter than an
/// entry are ignored.
pub fn parse_entries(bytes: &[u8]) -> Vec<AuditEntry> {
    bytes
        .as_chunks::<ENTRY_LEN>()
        .0
        .iter()
        .map(|e| AuditEntry {
            seq: u32::from_le_bytes([e[0], e[1], e[2], e[3]]),
            uptime_ms: u32::from_le_bytes([e[4], e[5], e[6], e[7]]),
            event: e[8],
            aux: e[9],
            detail: e[10..18].try_into().unwrap(),
        })
        .collect()
}

/// Fold the epoch accumulator over the window: `h = SHA256(h || entry)` per entry.
pub fn fold_chain(epoch: &[u8; 32], entries: &[u8]) -> [u8; 32] {
    let mut h = *epoch;
    for chunk in entries.chunks(ENTRY_LEN) {
        let mut buf = [0u8; 32 + ENTRY_LEN];
        buf[..32].copy_from_slice(&h);
        buf[32..32 + chunk.len()].copy_from_slice(chunk);
        let d = digest::digest(&digest::SHA256, &buf[..32 + chunk.len()]);
        h.copy_from_slice(d.as_ref());
    }
    h
}

/// The 16-hex-char attestation-key fingerprint (`sha256(pubkey)[..8]`).
pub fn fingerprint(pubkey: &[u8]) -> String {
    let d = digest::digest(&digest::SHA256, pubkey);
    hex::encode(&d.as_ref()[..8])
}

/// Verify the DEVK checkpoint signature over `head ‖ seq ‖ challenge`.
pub fn verify_checkpoint(
    head: &[u8],
    seq: u32,
    sig: &[u8],
    pubkey: &[u8],
    challenge: &[u8],
) -> bool {
    let mut msg = Vec::with_capacity(CKPT_TAG.len() + head.len() + 4 + challenge.len());
    msg.extend_from_slice(CKPT_TAG);
    msg.extend_from_slice(head);
    msg.extend_from_slice(&seq.to_le_bytes());
    msg.extend_from_slice(challenge);
    let vk = signature::UnparsedPublicKey::new(&signature::ECDSA_P256_SHA256_ASN1, pubkey);
    vk.verify(&msg, sig).is_ok()
}

/// Assemble a journal window, checking its length matches `[start, seq_next)`.
pub fn build_journal(
    start: u32,
    seq_next: u32,
    epoch: [u8; 32],
    entries_bytes: &[u8],
) -> Result<AuditJournal, String> {
    let expected = (seq_next.saturating_sub(start) as usize) * ENTRY_LEN;
    if !entries_bytes.len().is_multiple_of(ENTRY_LEN) || entries_bytes.len() != expected {
        return Err("export length does not match the window — corrupt journal?".into());
    }
    Ok(AuditJournal {
        start,
        seq_next,
        epoch,
        head: fold_chain(&epoch, entries_bytes),
        entries: parse_entries(entries_bytes),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ring::rand::SystemRandom;
    use ring::signature::{ECDSA_P256_SHA256_ASN1_SIGNING, EcdsaKeyPair, KeyPair};

    fn entry(seq: u32, event: u8) -> Vec<u8> {
        let mut e = vec![0u8; ENTRY_LEN];
        e[0..4].copy_from_slice(&seq.to_le_bytes());
        e[8] = event;
        e
    }

    #[test]
    fn parses_and_labels_entries() {
        let mut bytes = entry(5, 0x01);
        bytes.extend(entry(6, 0xAB)); // unknown id
        let parsed = parse_entries(&bytes);
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].seq, 5);
        assert_eq!(parsed[0].event_label(), "BOOT");
        assert_eq!(parsed[1].event_label(), "0xab");
    }

    #[test]
    fn fold_is_deterministic_and_chained() {
        let epoch = [7u8; 32];
        let mut w = entry(1, 0x01);
        w.extend(entry(2, 0x03));
        let h1 = fold_chain(&epoch, &w);
        // Folding one more entry must change the head (chain property).
        let mut w2 = w.clone();
        w2.extend(entry(3, 0x04));
        assert_ne!(h1, fold_chain(&epoch, &w2));
        // Same input → same output.
        assert_eq!(h1, fold_chain(&epoch, &w));
    }

    #[test]
    fn build_journal_rejects_length_mismatch() {
        assert!(build_journal(0, 3, [0u8; 32], &entry(0, 0x01)).is_err());
        let mut w = entry(0, 0x01);
        w.extend(entry(1, 0x01));
        assert!(build_journal(0, 2, [0u8; 32], &w).is_ok());
    }

    #[test]
    fn checkpoint_roundtrip_verifies_and_rejects_tamper() {
        let rng = SystemRandom::new();
        let pkcs8 = EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, &rng).unwrap();
        let kp = EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, pkcs8.as_ref(), &rng)
            .unwrap();
        let pubkey = kp.public_key().as_ref().to_vec();

        let head = [0x11u8; 32];
        let seq = 42u32;
        let challenge = [0x22u8; 16];
        let mut msg = Vec::new();
        msg.extend_from_slice(CKPT_TAG);
        msg.extend_from_slice(&head);
        msg.extend_from_slice(&seq.to_le_bytes());
        msg.extend_from_slice(&challenge);
        let sig = kp.sign(&rng, &msg).unwrap();

        assert!(verify_checkpoint(
            &head,
            seq,
            sig.as_ref(),
            &pubkey,
            &challenge
        ));
        // A different challenge must fail (freshness) and a bad key too.
        assert!(!verify_checkpoint(
            &head,
            seq,
            sig.as_ref(),
            &pubkey,
            &[0x23u8; 16]
        ));
        assert!(!verify_checkpoint(
            &head,
            seq + 1,
            sig.as_ref(),
            &pubkey,
            &challenge
        ));
    }

    #[test]
    fn fingerprint_is_16_hex_chars() {
        assert_eq!(fingerprint(&[0x04u8; 65]).len(), 16);
    }
}
