//! COSE (CBOR Object Signing and Encryption) algorithm, curve, and key-parameter
//! constants used in CTAP2 credential creation and authentication responses.

#![allow(dead_code)]

use std::fmt;

/// COSE algorithm identifiers as defined in the IANA COSE Algorithms registry.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoseAlgorithm {
    /// ECDSA w/ SHA-256 on P-256 (NIST P-256 / secp256r1).
    ES256 = -7,
    /// EdDSA (Edwards-curve Digital Signature Algorithm).
    EdDSA = -8,
    /// ECDSA w/ SHA-256 on P-256 (parallel-sphere variant).
    ESP256 = -9,
    /// Ed25519 signature algorithm (EdDSA on Curve25519).
    Ed25519 = -19,
    /// ECDH-ES key agreement w/ HKDF-256.
    EcdhEsHkdf256 = -25,
    /// ECDSA w/ SHA-384 on P-384.
    ES384 = -35,
    /// ECDSA w/ SHA-512 on P-521.
    ES512 = -36,
    /// ECDSA w/ SHA-256 on secp256k1 (Koblitz curve).
    ES256K = -47,
    /// ECDSA w/ SHA-384 on P-384 (parallel-sphere variant).
    ESP384 = -51,
    /// ECDSA w/ SHA-512 on P-521 (parallel-sphere variant).
    ESP512 = -52,
    /// Ed448 signature algorithm.
    Ed448 = -53,
    /// RSASSA-PKCS1-v1_5 w/ SHA-256.
    RS256 = -257,
    /// RSASSA-PKCS1-v1_5 w/ SHA-384.
    RS384 = -258,
    /// RSASSA-PKCS1-v1_5 w/ SHA-512.
    RS512 = -259,
    /// BLS (Boneh–Lynn–Shacham) signature w/ BLS12-381 (curve B).
    ESB256 = -265,
    /// BLS signature w/ BLS12-381 (curve B, larger subgroup).
    ESB384 = -267,
    /// BLS signature w/ BLS12-381 (curve B, full size).
    ESB512 = -268,
    /// ML-DSA-44 (CRYSTALS-Dilithium, NIST Level 2).
    MLDSA44 = -48,
    /// ML-DSA-65 (CRYSTALS-Dilithium, NIST Level 3).
    MLDSA65 = -49,
    /// ML-DSA-87 (CRYSTALS-Dilithium, NIST Level 5).
    MLDSA87 = -50,
}

impl CoseAlgorithm {
    /// Decode a COSE algorithm identifier from an `i128` value as seen in
    /// CTAP2 `authenticatorGetInfo` or credential public-key data.
    pub fn from_i128(val: i128) -> Option<Self> {
        match val as i32 {
            -7 => Some(Self::ES256),
            -8 => Some(Self::EdDSA),
            -9 => Some(Self::ESP256),
            -19 => Some(Self::Ed25519),
            -25 => Some(Self::EcdhEsHkdf256),
            -35 => Some(Self::ES384),
            -36 => Some(Self::ES512),
            -47 => Some(Self::ES256K),
            -51 => Some(Self::ESP384),
            -52 => Some(Self::ESP512),
            -53 => Some(Self::Ed448),
            -257 => Some(Self::RS256),
            -258 => Some(Self::RS384),
            -259 => Some(Self::RS512),
            -265 => Some(Self::ESB256),
            -267 => Some(Self::ESB384),
            -268 => Some(Self::ESB512),
            -48 => Some(Self::MLDSA44),
            -49 => Some(Self::MLDSA65),
            -50 => Some(Self::MLDSA87),
            _ => None,
        }
    }
}

impl fmt::Display for CoseAlgorithm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ES256 => write!(f, "ES256"),
            Self::EdDSA => write!(f, "EdDSA"),
            Self::ESP256 => write!(f, "ESP256"),
            Self::Ed25519 => write!(f, "Ed25519"),
            Self::EcdhEsHkdf256 => write!(f, "ECDH-ES-HKDF-256"),
            Self::ES384 => write!(f, "ES384"),
            Self::ES512 => write!(f, "ES512"),
            Self::ES256K => write!(f, "ES256K"),
            Self::ESP384 => write!(f, "ESP384"),
            Self::ESP512 => write!(f, "ESP512"),
            Self::Ed448 => write!(f, "Ed448"),
            Self::RS256 => write!(f, "RS256"),
            Self::RS384 => write!(f, "RS384"),
            Self::RS512 => write!(f, "RS512"),
            Self::ESB256 => write!(f, "ESB256"),
            Self::ESB384 => write!(f, "ESB384"),
            Self::ESB512 => write!(f, "ESB512"),
            Self::MLDSA44 => write!(f, "ML-DSA-44"),
            Self::MLDSA65 => write!(f, "ML-DSA-65"),
            Self::MLDSA87 => write!(f, "ML-DSA-87"),
        }
    }
}

/// COSE elliptic curve identifiers from the IANA COSE Elliptic Curves registry.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoseCurve {
    /// NIST P-256 (secp256r1).
    P256 = 1,
    /// NIST P-384 (secp384r1).
    P384 = 2,
    /// NIST P-521 (secp521r1).
    P521 = 3,
    /// X25519 key-exchange curve.
    X25519 = 4,
    /// X448 key-exchange curve.
    X448 = 5,
    /// Ed25519 signing curve.
    Ed25519 = 6,
    /// Ed448 signing curve.
    Ed448 = 7,
    /// secp256k1 (Koblitz curve, used by ES256K).
    P256K1 = 8,
    /// Barreto–Naehrig BN256 curve (pairing-friendly).
    BP256R1 = 9,
    /// Barreto–Naehrig BN384 curve (pairing-friendly).
    BP384R1 = 10,
    /// Barreto–Naehrig BN512 curve (pairing-friendly).
    BP512R1 = 11,
}

/// COSE key-parameter labels from RFC 8152 §7.1 / IANA.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoseKeyParam {
    /// Key type (kty).
    Kty = 1,
    /// Key identifier (kid).
    Kid = 2,
    /// Algorithm (alg).
    Alg = 3,
    /// Key operations (key_ops).
    KeyOps = 4,
    /// Base initialization vector (Base IV).
    BaseIV = 5,
    /// Curve / subgroup (crv).
    Crv = -1,
    /// X coordinate.
    X = -2,
    /// Y coordinate.
    Y = -3,
    /// Private key (d).
    D = -4,
}
