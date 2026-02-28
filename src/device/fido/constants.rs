//! Constants, enums, bitflags and data structures for FIDO2 protocol for pico-fido firmware.
#![allow(unused)]

use std::fmt;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CtapCommand {
    MakeCredential = 0x01,
    GetAssertion = 0x02,
    GetInfo = 0x04,
    ClientPin = 0x06,
    Reset = 0x07,
    GetNextAssertion = 0x08,
    CredentialMgmt = 0x0A,
    Selection = 0x0B,
    LargeBlobs = 0x0C,
    Config = 0x0D,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum U2fCommand {
    Register = 0x01,
    Authenticate = 0x02,
    Version = 0x03,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VendorCommand {
    Backup = 0x01,
    ManageSecurityEnvironment = 0x02,
    Unlock = 0x03,
    EnterpriseAttestation = 0x04,
    PhysicalOptions = 0x05,
    Memory = 0x06,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthenticateControl {
    EnforceUserPresence = 0x03,
    CheckOnly = 0x07,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientPinSubCommand {
    GetPinRetries = 0x01,
    GetKeyAgreement = 0x02,
    SetPin = 0x03,
    ChangePin = 0x04,
    GetPinToken = 0x05,
    GetPinUvAuthTokenUsingUvWithPermissions = 0x06,
    GetUvRetries = 0x07,
    GetPinUvAuthTokenUsingPinWithPermissions = 0x08,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MakeCredentialParam {
    ClientDataHash = 0x01,
    Rp = 0x02,
    User = 0x03,
    PubKeyCredParams = 0x04,
    ExcludeList = 0x05,
    Extensions = 0x06,
    Options = 0x07,
    PinUvAuthParam = 0x08,
    PinUvAuthProtocol = 0x09,
    EnterpriseAttestation = 0x0A,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GetAssertionParam {
    RpId = 0x01,
    ClientDataHash = 0x02,
    AllowList = 0x03,
    Extensions = 0x04,
    Options = 0x05,
    PinUvAuthParam = 0x06,
    PinUvAuthProtocol = 0x07,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientPinParam {
    PinUvAuthProtocol = 0x01,
    SubCommand = 0x02,
    KeyAgreement = 0x03,
    PinUvAuthParam = 0x04,
    NewPinEnc = 0x05,
    PinHashEnc = 0x06,
    Permissions = 0x09,
    PermissionsRpId = 0x0A,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigParam {
    SubCommand = 0x01,
    SubCommandParams = 0x02,
    PinUvAuthProtocol = 0x03,
    PinUvAuthParam = 0x04,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigSubCommand {
    EnableEnterpriseAttestation = 0x01,
    ToggleAlwaysUv = 0x02,
    SetMinPinLength = 0x03,
    VendorPrototype = 0xFF,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VendorParam {
    VendorCommand = 0x01,
    VendorSubParams = 0x02,
    PinUvAuthProtocol = 0x03,
    PinUvAuthParam = 0x04,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VendorSubParam {
    VendorParam = 0x01,
    CoseKey = 0x02,
    VendorParamInt = 0x03,
    VendorParamText = 0x04,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigSubCommandParam {
    NewMinPinLength = 0x01,
    MinPinLengthRPIDs = 0x02,
    ForceChangePin = 0x03,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VendorConfigCommand {
    AuthEncryptionEnable,
    AuthEncryptionDisable,
    EnterpriseAttestationUpload,
    PinComplexityPolicy,
    PhysicalVidPid,
    PhysicalLedBrightness,
    PhysicalLedGpio,
    PhysicalOptions,
}

impl VendorConfigCommand {
    pub fn to_u64(self) -> u64 {
        match self {
            Self::AuthEncryptionEnable => 0x03e43f56b34285e2,
            Self::AuthEncryptionDisable => 0x1831a40f04a25ed9,
            Self::EnterpriseAttestationUpload => 0x66f2a674c29a8dcf,
            Self::PinComplexityPolicy => 0x6c07d70fe96c3897,
            Self::PhysicalVidPid => 0x6fcb19b0cbe3acfa,
            Self::PhysicalLedBrightness => 0x76a85945985d02fd,
            Self::PhysicalLedGpio => 0x7b392a394de9f948,
            Self::PhysicalOptions => 0x269f3b09eceb805f,
        }
    }

    pub fn from_u64(val: u64) -> Option<Self> {
        match val {
            0x03e43f56b34285e2 => Some(Self::AuthEncryptionEnable),
            0x1831a40f04a25ed9 => Some(Self::AuthEncryptionDisable),
            0x66f2a674c29a8dcf => Some(Self::EnterpriseAttestationUpload),
            0x6c07d70fe96c3897 => Some(Self::PinComplexityPolicy),
            0x6fcb19b0cbe3acfa => Some(Self::PhysicalVidPid),
            0x76a85945985d02fd => Some(Self::PhysicalLedBrightness),
            0x7b392a394de9f948 => Some(Self::PhysicalLedGpio),
            0x269f3b09eceb805f => Some(Self::PhysicalOptions),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FidoCertification {
    AuthEncryption,
    AuthEncryptionLock,
    EnterpriseAttestation,
    PinComplexity,
    PhysicalVidPid,
    LedBrightness,
    LedGpio,
    PhysicalOptions,
}

impl FidoCertification {
    pub fn from_u64(val: u64) -> Option<Self> {
        match val {
            0x03E43F56B34285E2 => Some(Self::AuthEncryption),
            0x1831A40F04A25ED9 => Some(Self::AuthEncryptionLock),
            0x66F2A674C29A8DCF => Some(Self::EnterpriseAttestation),
            0x6C07D70FE96C3897 => Some(Self::PinComplexity),
            0x6FCB19B0CBE3ACFA => Some(Self::PhysicalVidPid),
            0x76A85945985D02FD => Some(Self::LedBrightness),
            0x7B392A394DE9F948 => Some(Self::LedGpio),
            0x269F3B09ECEB805F => Some(Self::PhysicalOptions),
            _ => None,
        }
    }

    pub fn from_str(val: &str) -> Option<Self> {
        let val = val.strip_prefix("0x").unwrap_or(val);
        u64::from_str_radix(val, 16).ok().and_then(Self::from_u64)
    }
}

impl fmt::Display for FidoCertification {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AuthEncryption => write!(f, "Auth Encryption"),
            Self::AuthEncryptionLock => write!(f, "Auth Encryption (Lock)"),
            Self::EnterpriseAttestation => write!(f, "Enterprise Attestation"),
            Self::PinComplexity => write!(f, "PIN Complexity"),
            Self::PhysicalVidPid => write!(f, "Physical VID/PID"),
            Self::LedBrightness => write!(f, "LED Brightness"),
            Self::LedGpio => write!(f, "LED GPIO"),
            Self::PhysicalOptions => write!(f, "Physical Options"),
        }
    }
}

impl fmt::Display for VendorConfigCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AuthEncryptionEnable => write!(f, "AuthEncryptionEnable"),
            Self::AuthEncryptionDisable => write!(f, "AuthEncryptionDisable"),
            Self::EnterpriseAttestationUpload => write!(f, "EnterpriseAttestationUpload"),
            Self::PinComplexityPolicy => write!(f, "PinComplexityPolicy"),
            Self::PhysicalVidPid => write!(f, "PhysicalVidPid"),
            Self::PhysicalLedBrightness => write!(f, "PhysicalLedBrightness"),
            Self::PhysicalLedGpio => write!(f, "PhysicalLedGpio"),
            Self::PhysicalOptions => write!(f, "PhysicalOptions"),
        }
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackupSubCommand {
    GetEncryptedBackup = 0x01,
    RestoreEncryptedBackup = 0x02,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MseSubCommand {
    KeyAgreement = 0x01,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnterpriseAttestationSubCommand {
    GenerateCsr = 0x01,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhysicalOptionsSubCommand {
    GetOptions = 0x01,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemorySubCommand {
    GetStats = 0x01,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryResponseKey {
    FreeSpace = 0x01,
    UsedSpace = 0x02,
    TotalSpace = 0x03,
    NumFiles = 0x04,
    FlashSize = 0x05,
}

bitflags::bitflags! {
    pub struct PinUvAuthTokenPermissions: u8 {
        const MAKE_CREDENTIAL = 0x01;
        const GET_ASSERTION = 0x02;
        const CREDENTIAL_MANAGEMENT = 0x04;
        const BIO_ENROLLMENT = 0x08;
        const LARGE_BLOB_WRITE = 0x10;
        const AUTHENTICATOR_CONFIG = 0x20;
        const PER_CREDENTIAL_MGMT_READONLY = 0x40;
    }
}

bitflags::bitflags! {
    pub struct AuthenticatorFlags: u8 {
        const USER_PRESENT = 0x01;
        const USER_VERIFIED = 0x04;
        const ATTESTED_CREDENTIAL_DATA = 0x40;
        const EXTENSION_DATA = 0x80;
    }
}

bitflags::bitflags! {
    pub struct AuthenticatorOptions: u8 {
        const ENTERPRISE_ATTESTATION = 0x01;
        const USER_VERIFICATION = 0x02;
    }
}

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

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ctap2Error {
    Success = 0x00,
    CborUnexpectedType = 0x11,
    InvalidCbor = 0x12,
    MissingParameter = 0x14,
    LimitExceeded = 0x15,
    FpDatabaseFull = 0x17,
    LargeBlobStorageFull = 0x18,
    CredentialExcluded = 0x19,
    Processing = 0x21,
    InvalidCredential = 0x22,
    UserActionPending = 0x23,
    OperationPending = 0x24,
    NoOperations = 0x25,
    UnsupportedAlgorithm = 0x26,
    OperationDenied = 0x27,
    KeyStoreFull = 0x28,
    UnsupportedOption = 0x2B,
    InvalidOption = 0x2C,
    KeepaliveCancel = 0x2D,
    NoCredentials = 0x2E,
    UserActionTimeout = 0x2F,
    NotAllowed = 0x30,
    PinInvalid = 0x31,
    PinBlocked = 0x32,
    PinAuthInvalid = 0x33,
    PinAuthBlocked = 0x34,
    PinNotSet = 0x35,
    PuatRequired = 0x36,
    PinPolicyViolation = 0x37,
    RequestTooLarge = 0x39,
    ActionTimeout = 0x3A,
    UpRequired = 0x3B,
    UvBlocked = 0x3C,
    IntegrityFailure = 0x3D,
    InvalidSubcommand = 0x3E,
    UvInvalid = 0x3F,
    UnauthorizedPermission = 0x40,
}

pub const CTAP_VENDOR_CBOR_CMD: u8 = 0xC1;
pub const CTAP_VENDOR_CONFIG_CMD: u8 = 0xC2;

pub const CTAP_APPID_SIZE: usize = 32;
pub const CTAP_CHAL_SIZE: usize = 32;
pub const CTAP_EC_KEY_SIZE: usize = 32;
pub const CTAP_EC_POINT_SIZE: usize = 65;
pub const CTAP_MAX_KH_SIZE: usize = 128;
pub const KEY_HANDLE_LEN: usize = 64;
pub const CTAP_MAX_EC_SIG_SIZE: usize = 72;
pub const CTAP_CTR_SIZE: usize = 4;

pub const MAX_PIN_RETRIES: u8 = 8;
pub const MAX_CREDENTIAL_COUNT_IN_LIST: usize = 16;
pub const MAX_CRED_ID_LENGTH: usize = 1024;
pub const MAX_RESIDENT_CREDENTIALS: usize = 256;
pub const MAX_CREDBLOB_LENGTH: usize = 128;
pub const MAX_MSG_SIZE: usize = 1024;
pub const MAX_FRAGMENT_LENGTH: usize = MAX_MSG_SIZE - 64;
pub const MAX_LARGE_BLOB_SIZE: usize = 2048;

pub const AAGUID: [u8; 16] = [
    0x89, 0xFB, 0x94, 0xB7, 0x06, 0xC9, 0x36, 0x73, 0x9B, 0x7E, 0x30, 0x52, 0x6D, 0x96, 0x81, 0x45,
];
