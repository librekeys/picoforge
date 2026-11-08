//! Wallet-style FIDO seed backup — the crypto half.
//!
//! The device exports its 32-byte master seed once, encrypted over an ephemeral
//! ECDH channel; restore re-seals a seed under the new device's key. Here live
//! the transport-independent pieces: the HKDF channel-key derivation, the
//! ChaCha20-Poly1305 seal/open, and the BIP-39 mnemonic rendering — all host-
//! tested. The ECDH handshake and vendor I/O live in the parent module.
//!
//! Only the classical P-256 channel is implemented; the firmware falls back to
//! it when the host offers no ML-KEM encapsulation key, so this stays
//! interoperable (just not post-quantum hybrid).

use ring::aead;
use ring::hkdf;
use std::str::FromStr;

/// Soft-lock / backup state reported by the vendor STATE subcommand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackupStatus {
    pub sealed: bool,
    pub has_seed: bool,
    pub locked: bool,
    pub unlocked: bool,
}

struct Len32;
impl hkdf::KeyType for Len32 {
    fn len(&self) -> usize {
        32
    }
}

/// Derive the 32-byte channel key: `HKDF-SHA256(salt="", ikm=z, info=aad)`.
/// `aad` is the device's uncompressed P-256 point (`0x04 ‖ x ‖ y`).
pub fn derive_channel_key(z: &[u8], aad: &[u8]) -> [u8; 32] {
    let prk = hkdf::Salt::new(hkdf::HKDF_SHA256, b"").extract(z);
    let info: [&[u8]; 1] = [aad];
    let okm = prk.expand(&info, Len32).expect("hkdf expand");
    let mut key = [0u8; 32];
    okm.fill(&mut key).expect("hkdf fill");
    key
}

/// Decrypt `nonce(12) ‖ ciphertext‖tag` bound to `aad`; returns the plaintext.
pub fn chacha_open(key: &[u8; 32], nonce_and_ct: &[u8], aad: &[u8]) -> Result<Vec<u8>, String> {
    if nonce_and_ct.len() < 12 + 16 {
        return Err("ciphertext too short".into());
    }
    let (nonce_b, ct) = nonce_and_ct.split_at(12);
    let ubk =
        aead::UnboundKey::new(&aead::CHACHA20_POLY1305, key).map_err(|_| "bad key".to_string())?;
    let lk = aead::LessSafeKey::new(ubk);
    let nonce = aead::Nonce::assume_unique_for_key(nonce_b.try_into().unwrap());
    let mut buf = ct.to_vec();
    let pt = lk
        .open_in_place(nonce, aead::Aad::from(aad), &mut buf)
        .map_err(|_| "decryption failed (wrong channel / tampered)".to_string())?;
    Ok(pt.to_vec())
}

/// Encrypt `plaintext` under `nonce`+`aad`; returns `nonce(12) ‖ ciphertext‖tag`.
pub fn chacha_seal(
    key: &[u8; 32],
    nonce_b: &[u8; 12],
    plaintext: &[u8],
    aad: &[u8],
) -> Result<Vec<u8>, String> {
    let ubk =
        aead::UnboundKey::new(&aead::CHACHA20_POLY1305, key).map_err(|_| "bad key".to_string())?;
    let lk = aead::LessSafeKey::new(ubk);
    let nonce = aead::Nonce::assume_unique_for_key(*nonce_b);
    let mut buf = plaintext.to_vec();
    lk.seal_in_place_append_tag(nonce, aead::Aad::from(aad), &mut buf)
        .map_err(|_| "encryption failed".to_string())?;
    let mut blob = nonce_b.to_vec();
    blob.extend(buf);
    Ok(blob)
}

/// Render a 32-byte seed as a 24-word BIP-39 phrase.
pub fn seed_to_mnemonic(seed: &[u8; 32]) -> Result<String, String> {
    bip39::Mnemonic::from_entropy(seed)
        .map(|m| m.to_string())
        .map_err(|e| e.to_string())
}

/// Parse a 24-word BIP-39 phrase back to its 32-byte seed.
pub fn mnemonic_to_seed(phrase: &str) -> Result<[u8; 32], String> {
    let m = bip39::Mnemonic::from_str(phrase.trim())
        .map_err(|e| format!("invalid BIP-39 phrase: {e}"))?;
    let (entropy, len) = m.to_entropy_array();
    if len != 32 {
        return Err(format!("phrase encodes {len} bytes, expected 32"));
    }
    Ok(entropy[..32].try_into().unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channel_key_is_deterministic_and_32_bytes() {
        let z = [3u8; 32];
        let aad = [0x04u8; 65];
        let k1 = derive_channel_key(&z, &aad);
        assert_eq!(k1, derive_channel_key(&z, &aad));
        // A different AAD must change the key.
        let mut aad2 = aad;
        aad2[1] ^= 1;
        assert_ne!(k1, derive_channel_key(&z, &aad2));
    }

    #[test]
    fn chacha_roundtrip_and_aad_binding() {
        let key = [7u8; 32];
        let nonce = [9u8; 12];
        let aad = b"\x04aad-point";
        let seed = [0x42u8; 32];
        let blob = chacha_seal(&key, &nonce, &seed, aad).unwrap();
        assert_eq!(&blob[..12], &nonce);
        assert_eq!(chacha_open(&key, &blob, aad).unwrap(), seed);
        // Wrong AAD must fail closed.
        assert!(chacha_open(&key, &blob, b"other").is_err());
    }

    #[test]
    fn mnemonic_roundtrip_is_24_words() {
        let seed: [u8; 32] = std::array::from_fn(|i| i as u8);
        let phrase = seed_to_mnemonic(&seed).unwrap();
        assert_eq!(phrase.split_whitespace().count(), 24);
        assert_eq!(mnemonic_to_seed(&phrase).unwrap(), seed);
        assert!(mnemonic_to_seed("not a valid phrase").is_err());
    }
}
