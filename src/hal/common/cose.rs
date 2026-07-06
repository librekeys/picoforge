#![allow(dead_code)]
use std::fmt;

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoseAlgorithm {
    ES256 = -7,
    EdDSA = -8,
    ESP256 = -9,
    Ed25519 = -19,
    EcdhEsHkdf256 = -25,
    ES384 = -35,
    ES512 = -36,
    ES256K = -47,
    ESP384 = -51,
    ESP512 = -52,
    Ed448 = -53,
    RS256 = -257,
    RS384 = -258,
    RS512 = -259,
    ESB256 = -265,
    ESB384 = -267,
    ESB512 = -268,
    MLDSA44 = -48,
    MLDSA65 = -49,
    MLDSA87 = -50,
}

impl CoseAlgorithm {
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

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoseCurve {
    P256 = 1,
    P384 = 2,
    P521 = 3,
    X25519 = 4,
    X448 = 5,
    Ed25519 = 6,
    Ed448 = 7,
    P256K1 = 8,
    BP256R1 = 9,
    BP384R1 = 10,
    BP512R1 = 11,
}

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoseKeyParam {
    Kty = 1,
    Kid = 2,
    Alg = 3,
    KeyOps = 4,
    BaseIV = 5,
    Crv = -1,
    X = -2,
    Y = -3,
    D = -4,
}
