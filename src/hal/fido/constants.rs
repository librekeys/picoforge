//! CTAP2 / FIDO2 protocol constants for pico-fido and RS-Key firmware.
//!
//! This file is the single source of truth for every byte value, error code,
//! and CBOR map key used by this codebase. Values are organized into three
//! categories:
//!
//! 1. **CTAP2 standard** — defined by the [FIDO CTAP2 spec], used by any
//!    CTAP2-compliant authenticator.
//! 2. **Pico-fido vendor extensions** — custom commands/IDs for the
//!    [pico-fido] firmware (vendor commands `0xC1`/`0xC2`, 64-bit config IDs).
//! 3. **RS-Key extensions** — additions specific to [RS-Key] firmware
//!    (rescue applet AIDs, phy record tags, CTAPHID `0x41` vendor command).
//!
//! All enums use `#[repr(u8)]` or `#[repr(u64)]` so their numeric values
//! match the wire format exactly.
//!
//! # Reference
//!
//! - CTAP2 values: [CTAP2 v2.3 spec §8.1](https://fidoalliance.org/specs/fido-v2.3-ps-20260226/fido-client-to-authenticator-protocol-v2.3-ps-20260226.html)
//! - Pico-fido vendor commands: [pico-fido source](https://github.com/polhenarejos/pico-fido)
//! - RS-Key protocol: [RS-Key Host Protocol Docs](https://themaxmur.github.io/RS-Key/develop/protocol.html)
//!
//! [FIDO CTAP2 spec]: https://fidoalliance.org/specs/fido-v2.3-ps-20260226/fido-client-to-authenticator-protocol-v2.3-ps-20260226.html
//! [pico-fido]: https://github.com/polhenarejos/pico-fido
//! [RS-Key]: https://github.com/TheMaxMur/RS-Key
#![allow(unused)]

use std::fmt;

// ══════════════════════════════════════════════════════════════════════════════
// CTAP2 STANDARD — FIDO Alliance specification §8.1
// ══════════════════════════════════════════════════════════════════════════════

// ── CTAP2 command codes (§8.1) ──────────────────────────────────────────────

/// CTAP2 CBOR command codes (CTAP2 spec §8.1).
///
/// These are the opcodes sent as the first byte of a `CTAPHID_CBOR` payload.
/// The authenticator dispatches to the corresponding handler based on this byte.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CtapCommand {
    /// Create a new credential (§11.5.1).
    MakeCredential = 0x01,
    /// Generate an authentication assertion (§11.5.2).
    GetAssertion = 0x02,
    /// Return authenticator metadata (§11.5.3).
    GetInfo = 0x04,
    /// PIN/UV token management (§11.5.4).
    ClientPin = 0x06,
    /// Factory-reset all credentials and PIN (§11.5.5).
    Reset = 0x07,
    /// Get the next assertion when multiple credentials match (§11.5.6).
    GetNextAssertion = 0x08,
    /// Credential management operations (§11.5.8).
    CredentialMgmt = 0x0A,
    /// Put the authenticator into a discoverable state (§11.5.7).
    Selection = 0x0B,
    /// Read/write large blob storage (§11.5.9).
    LargeBlobs = 0x0C,
    /// Authenticator configuration (enterprise attestation, min PIN, etc.) (§11.5.10).
    Config = 0x0D,
}

/// CTAP1/U2F command codes (legacy protocol, U2F Raw Messages spec).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum U2fCommand {
    /// U2F Register command.
    Register = 0x01,
    /// U2F Authenticate command.
    Authenticate = 0x02,
    /// U2F Version inquiry.
    Version = 0x03,
}

// ── CBOR map key enums (§11.5.x) ───────────────────────────────────────────
//
// Each CTAP2 command encodes its parameters as a CBOR map with integer keys.
// These enums map human-readable names to the wire-format key bytes.

/// CBOR map keys for `authenticatorClientPIN` sub-commands (§11.5.4).
///
/// The `subCommand` field selects which PIN operation to perform.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientPinSubCommand {
    /// Get remaining PIN attempts.
    GetPinRetries = 0x01,
    /// Get ECDH key agreement public key.
    GetKeyAgreement = 0x02,
    /// Set a new PIN (first-time setup).
    SetPin = 0x03,
    /// Change an existing PIN.
    ChangePin = 0x04,
    /// Get a PIN token for permission-gated operations.
    GetPinToken = 0x05,
    /// Get a UV auth token using biometric/other UV (§11.5.4.1).
    GetPinUvAuthTokenUsingUvWithPermissions = 0x06,
    /// Get remaining UV attempts.
    GetUvRetries = 0x07,
    /// Get a PIN auth token with specific permissions.
    GetPinUvAuthTokenUsingPinWithPermissions = 0x09,
}

/// CBOR map keys for `authenticatorMakeCredential` (§11.5.1).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MakeCredentialParam {
    /// SHA-256 hash of the client data (required).
    ClientDataHash = 0x01,
    /// Relying party object `{ id, name, icon }` (required).
    Rp = 0x02,
    /// User object `{ id, name, displayName, icon }` (required).
    User = 0x03,
    /// Supported credential algorithms, preferred-first (required).
    PubKeyCredParams = 0x04,
    /// Credentials to exclude (prevents duplication).
    ExcludeList = 0x05,
    /// Extension inputs.
    Extensions = 0x06,
    /// Options like `rk`, `up`, `uv`.
    Options = 0x07,
    /// HMAC from PIN/UV token for user verification.
    PinUvAuthParam = 0x08,
    /// PIN/UV protocol version (currently 1).
    PinUvAuthProtocol = 0x09,
    /// Enterprise attestation mode (0=off, 1=permissive, 2=strict).
    EnterpriseAttestation = 0x0A,
}

/// CBOR map keys for `authenticatorGetAssertion` (§11.5.2).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GetAssertionParam {
    /// Relying party identifier (required).
    RpId = 0x01,
    /// SHA-256 hash of the client data (required).
    ClientDataHash = 0x02,
    /// Allowed credentials; if present, only these may be used.
    AllowList = 0x03,
    /// Extension inputs.
    Extensions = 0x04,
    /// Options like `up`, `uv`, `pin`.
    Options = 0x05,
    /// HMAC from PIN/UV token.
    PinUvAuthParam = 0x06,
    /// PIN/UV protocol version.
    PinUvAuthProtocol = 0x07,
}

/// CBOR map keys for `authenticatorClientPIN` request body (§11.5.4).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientPinParam {
    /// PIN/UV protocol version (must be 1).
    PinUvAuthProtocol = 0x01,
    /// Sub-command to execute (see [`ClientPinSubCommand`]).
    SubCommand = 0x02,
    /// Platform's ECDH public key (COSE_Key).
    KeyAgreement = 0x03,
    /// HMAC of the encrypted PIN or client data.
    PinUvAuthParam = 0x04,
    /// AES-256-CBC encrypted new PIN.
    NewPinEnc = 0x05,
    /// AES-256-CBC encrypted first 16 bytes of PIN hash.
    PinHashEnc = 0x06,
    /// Permission bits for `getPinUvAuthTokenUsingPinWithPermissions`.
    Permissions = 0x09,
    /// RP ID scope for the requested permissions.
    PermissionsRpId = 0x0A,
}

/// CBOR map keys for `authenticatorClientPIN` response body (§11.5.4).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientPinResponseParam {
    /// Authenticator's ECDH public key (COSE_Key).
    KeyAgreement = 0x01,
    /// Encrypted PIN/UV auth token.
    PinToken = 0x02,
    /// Remaining PIN attempts.
    PinRetries = 0x03,
    /// Continuation message for large payloads.
    NextMsg = 0x04,
    /// Remaining UV attempts.
    UvRetries = 0x05,
}

/// CBOR map keys for `authenticatorConfig` (§11.5.10).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigParam {
    /// Config sub-command (see [`ConfigSubCommand`]).
    SubCommand = 0x01,
    /// Sub-command parameters (CBOR map).
    SubCommandParams = 0x02,
    /// PIN/UV protocol version.
    PinUvAuthProtocol = 0x03,
    /// HMAC of the sub-command parameters.
    PinUvAuthParam = 0x04,
}

/// Sub-commands for `authenticatorConfig` (§11.5.10).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigSubCommand {
    /// Enable enterprise attestation for this authenticator.
    EnableEnterpriseAttestation = 0x01,
    /// Toggle "always UV" policy (requires re-setting PIN).
    ToggleAlwaysUv = 0x02,
    /// Set the minimum PIN length requirement.
    SetMinPinLength = 0x03,
    /// Vendor-defined prototype config command (pico-fido/RS-Key extension).
    VendorPrototype = 0xFF,
}

/// Sub-commands for `authenticatorCredentialManagement` (§11.5.8).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredentialMgmtSubCommand {
    /// Get total credential/RP counts and remaining space.
    GetCredsMetadata = 0x01,
    /// Begin enumerating Relying Parties.
    EnumerateRpsBegin = 0x02,
    /// Get the next RP in the enumeration.
    EnumerateRpsGetNextRp = 0x03,
    /// Begin enumerating credentials for a given RP.
    EnumerateCredentialsBegin = 0x04,
    /// Get the next credential in the enumeration.
    EnumerateCredentialsGetNextCredential = 0x05,
    /// Delete a stored credential.
    DeleteCredential = 0x06,
    /// Update user information for a credential.
    UpdateUserInformation = 0x07,
}

/// CBOR map keys for `authenticatorCredentialManagement` requests (§11.5.8).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredentialMgmtParam {
    /// Sub-command to execute (see [`CredentialMgmtSubCommand`]).
    SubCommand = 0x01,
    /// Sub-command parameters (CBOR map).
    SubCommandParams = 0x02,
    /// PIN/UV protocol version.
    PinUvAuthProtocol = 0x03,
    /// HMAC for authentication.
    PinUvAuthParam = 0x04,
}

/// CBOR map keys for `authenticatorCredentialManagement` responses (§11.5.8).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredentialMgmtResponseParam {
    /// Relying party object.
    Rp = 0x03,
    /// SHA-256 hash of the RP ID.
    RpIdHash = 0x04,
    /// Total number of RPs stored.
    TotalRps = 0x05,
    /// User object.
    User = 0x06,
    /// Credential descriptor.
    CredentialId = 0x07,
    /// Credential public key (COSE_Key).
    PublicKey = 0x08,
    /// Total credentials for the current RP.
    TotalCredentials = 0x09,
}

/// Sub-command parameters for `authenticatorConfig` (§11.5.10).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigSubCommandParam {
    /// New minimum PIN length.
    NewMinPinLength = 0x01,
    /// RP IDs allowed to read the minimum PIN length.
    MinPinLengthRPIDs = 0x02,
    /// Force PIN change on next use.
    ForceChangePin = 0x03,
}

/// Control byte for U2F Authenticate (check-only vs. enforce presence).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthenticateControl {
    /// Require user presence test.
    EnforceUserPresence = 0x03,
    /// Check if key handle is valid (no user interaction).
    CheckOnly = 0x07,
}

// ── Bitflags (§11.3.2, §11.5.x) ────────────────────────────────────────────

/// Permission bits for `getPinUvAuthTokenUsingPinWithPermissions` (§11.5.4.1).
///
/// The platform requests specific permissions when obtaining a PIN/UV token.
/// The authenticator gates access to sensitive operations behind these flags.
bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PinUvAuthTokenPermissions: u8 {
        /// Permission to create credentials (MakeCredential).
        const MAKE_CREDENTIAL = 0x01;
        /// Permission to generate assertions (GetAssertion).
        const GET_ASSERTION = 0x02;
        /// Permission to enumerate/delete credentials.
        const CREDENTIAL_MANAGEMENT = 0x04;
        /// Permission for biometric enrollment.
        const BIO_ENROLLMENT = 0x08;
        /// Permission to write to large blob storage.
        const LARGE_BLOB_WRITE = 0x10;
        /// Permission to modify authenticator config (enterprise attestation, min PIN).
        const AUTHENTICATOR_CONFIG = 0x20;
        /// Read-only credential management (no delete/update).
        const PER_CREDENTIAL_MGMT_READONLY = 0x40;
    }
}

/// Flags byte in CTAP2 response messages (§11.3.2).
///
/// Indicates the authenticator's state after processing a command.
bitflags::bitflags! {
    pub struct AuthenticatorFlags: u8 {
        /// User presence was tested and confirmed.
        const USER_PRESENT = 0x01;
        /// User verification (biometric or PIN) was performed.
        const USER_VERIFIED = 0x04;
        /// Response includes attested credential data.
        const ATTESTED_CREDENTIAL_DATA = 0x40;
        /// Response includes extension output data.
        const EXTENSION_DATA = 0x80;
    }
}

/// Options that can be passed in `MakeCredential` or `GetAssertion` (§11.5.1/2).
bitflags::bitflags! {
    pub struct AuthenticatorOptions: u8 {
        /// Request enterprise attestation (MakeCredential only).
        const ENTERPRISE_ATTESTATION = 0x01;
        /// Require user verification (PIN or biometric).
        const USER_VERIFICATION = 0x02;
    }
}

// ── COSE key types (RFC 8152) ───────────────────────────────────────────────

/// COSE algorithm identifiers (IANA COSE Algorithms registry).
///
/// Used in `pubKeyCredParams` to specify which signature algorithms
/// the platform supports. The authenticator picks the first match.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoseAlgorithm {
    /// ECDSA with P-256 and SHA-256 (most common for WebAuthn).
    ES256 = -7,
    /// EdDSA with Ed25519.
    EdDSA = -8,
    /// ECDSA with P-256 (alternate ID, same as ES256).
    ESP256 = -9,
    /// EdDSA with Ed25519 (alternate ID).
    Ed25519 = -19,
    /// ECDH-ES with HKDF-256 key agreement.
    EcdhEsHkdf256 = -25,
    /// ECDSA with P-384 and SHA-384.
    ES384 = -35,
    /// ECDSA with P-521 and SHA-512.
    ES512 = -36,
    /// ECDSA with secp256k1 and SHA-256 (Bitcoin curve).
    ES256K = -47,
    /// ECDSA with P-384 (alternate ID).
    ESP384 = -51,
    /// ECDSA with P-521 (alternate ID).
    ESP512 = -52,
    /// EdDSA with Ed448.
    Ed448 = -53,
    /// RSASSA-PKCS1-v1_5 with SHA-256.
    RS256 = -257,
    /// RSASSA-PKCS1-v1_5 with SHA-384.
    RS384 = -258,
    /// RSASSA-PKCS1-v1_5 with SHA-512.
    RS512 = -259,
    /// ECDSA with brainpool256r1 and SHA-256.
    ESB256 = -265,
    /// ECDSA with brainpool384r1 and SHA-384.
    ESB384 = -267,
    /// ECDSA with brainpool512r1 and SHA-512.
    ESB512 = -268,
    /// ML-DSA-44 (FIPS 204, Level 2) — post-quantum signing.
    ///
    /// RS-Key specific. Uses COSE key type AKP (7) instead of EC2/OKP.
    MLDSA44 = -48,
    /// ML-DSA-65 (FIPS 204, Level 3) — declared in getInfo but may be
    /// unsupported for credential creation.
    MLDSA65 = -49,
    /// ML-DSA-87 (FIPS 204, Level 5) — declared in getInfo but may be
    /// unsupported for credential creation.
    MLDSA87 = -50,
}

impl CoseAlgorithm {
    /// Convert a raw i128 (from CBOR) to a [`CoseAlgorithm`].
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

/// COSE elliptic curve identifiers (RFC 8152 §13.1.1).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoseCurve {
    /// NIST P-256 (secp256r1, prime256v1).
    P256 = 1,
    /// NIST P-384 (secp384r1).
    P384 = 2,
    /// NIST P-521 (secp521r1).
    P521 = 3,
    /// X25519 for key agreement.
    X25519 = 4,
    /// X448 for key agreement.
    X448 = 5,
    /// Ed25519 for signing.
    Ed25519 = 6,
    /// Ed448 for signing.
    Ed448 = 7,
    /// secp256k1 (Bitcoin/Ethereum curve).
    P256K1 = 8,
    /// BrainpoolP256R1.
    BP256R1 = 9,
    /// BrainpoolP384R1.
    BP384R1 = 10,
    /// BrainpoolP512R1.
    BP512R1 = 11,
}

/// COSE key parameter identifiers (RFC 8152 §7.1).
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoseKeyParam {
    /// Key type (OKP, EC2, RSA, etc.).
    Kty = 1,
    /// Key identifier.
    Kid = 2,
    /// Algorithm identifier.
    Alg = 3,
    /// Key operations (sign, verify, encrypt, etc.).
    KeyOps = 4,
    /// Base IV for symmetric operations.
    BaseIV = 5,
    /// Elliptic curve identifier.
    Crv = -1,
    /// X coordinate (EC2) or public key bytes (OKP).
    X = -2,
    /// Y coordinate (EC2).
    Y = -3,
    /// Private key (EC2 or OKP).
    D = -4,
}

// ── CTAP2 errors (§8.2) ────────────────────────────────────────────────────

/// CTAP2 error codes (§8.2).
///
/// Returned as the first byte of a `CTAPHID_CBOR` response when the
/// status code is non-zero. Negative status indicates an error.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ctap2Error {
    /// Operation completed successfully.
    Success = 0x00,
    /// CBOR value has an unexpected type.
    CborUnexpectedType = 0x11,
    /// CBOR structure is malformed.
    InvalidCbor = 0x12,
    /// A required parameter is missing.
    MissingParameter = 0x14,
    /// A limit has been exceeded (e.g., too many credentials).
    LimitExceeded = 0x15,
    /// Internal fingerprint database is full.
    FpDatabaseFull = 0x17,
    /// Large blob storage is full.
    LargeBlobStorageFull = 0x18,
    /// Credential already exists (exclusion list match).
    CredentialExcluded = 0x19,
    /// Operation is still processing.
    Processing = 0x21,
    /// Credential ID is invalid or not found.
    InvalidCredential = 0x22,
    /// User action (touch) is pending.
    UserActionPending = 0x23,
    /// Another operation is in progress.
    OperationPending = 0x24,
    /// No more operations to process.
    NoOperations = 0x25,
    /// Algorithm not supported by the authenticator.
    UnsupportedAlgorithm = 0x26,
    /// Operation was denied (user declined or policy).
    OperationDenied = 0x27,
    /// Key store is full.
    KeyStoreFull = 0x28,
    /// Option not recognized or not supported.
    UnsupportedOption = 0x2B,
    /// Option value is invalid.
    InvalidOption = 0x2C,
    /// Keepalive was cancelled.
    KeepaliveCancel = 0x2D,
    /// No matching credentials found.
    NoCredentials = 0x2E,
    /// User action timed out.
    UserActionTimeout = 0x2F,
    /// Operation not allowed (e.g., reset not within power cycle).
    NotAllowed = 0x30,
    /// PIN is invalid.
    PinInvalid = 0x31,
    /// PIN is blocked (too many failed attempts).
    PinBlocked = 0x32,
    /// PIN authentication token is invalid.
    PinAuthInvalid = 0x33,
    /// PIN authentication is blocked.
    PinAuthBlocked = 0x34,
    /// PIN has not been set.
    PinNotSet = 0x35,
    /// PIN/UV auth token required but not provided.
    PuatRequired = 0x36,
    /// PIN policy violation (e.g., min length not met).
    PinPolicyViolation = 0x37,
    /// Request payload is too large.
    RequestTooLarge = 0x39,
    /// Action timed out.
    ActionTimeout = 0x3A,
    /// User presence (touch) required.
    UpRequired = 0x3B,
    /// User verification is blocked.
    UvBlocked = 0x3C,
    /// Cryptographic integrity check failed.
    IntegrityFailure = 0x3D,
    /// Sub-command not recognized.
    InvalidSubcommand = 0x3E,
    /// User verification is invalid.
    UvInvalid = 0x3F,
    /// Requested permission not authorized.
    UnauthorizedPermission = 0x40,
}

// ══════════════════════════════════════════════════════════════════════════════
// PICO-FIDO VENDOR EXTENSIONS
// ══════════════════════════════════════════════════════════════════════════════
//
// The following types are NOT part of the CTAP2 standard. They are custom
// extensions used by the pico-fido firmware (and RS-Key, which shares the
// same vendor command surface).

/// Pico-fido vendor CBOR sub-commands.
///
/// Sent as the first byte of the CBOR payload inside a
/// `CTAP_VENDOR_CBOR_CMD` (0xC1) message.
///
/// # Version history
///
/// - **All versions**: Backup(0x01), MSE(0x02), Unlock(0x03), EA(0x04)
/// - **≤ v7.2**: PhysicalOptions(0x05), Memory(0x06) — removed in later
///   releases. PicoForge keeps them for legacy device support.
/// - **Current**: AdminPin(0x08) added.
///
/// RS-Key uses a different vendor command scheme (CTAPHID 0x41 with
/// 64-bit sub-command IDs) — this enum does NOT apply to RS-Key.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VendorCommand {
    /// Encrypted backup / restore operations.
    Backup = 0x01,
    /// Manage security environment (key agreement).
    ManageSecurityEnvironment = 0x02,
    /// Unlock a locked device.
    Unlock = 0x03,
    /// Enterprise attestation CSR generation.
    EnterpriseAttestation = 0x04,
    /// Physical options (LED, power, etc.) — legacy TLV encoding.
    ///
    /// **Legacy** (pico-fido ≤ v7.2 only). Removed in current firmware.
    PhysicalOptions = 0x05,
    /// Flash memory statistics (free/used/total).
    ///
    /// **Legacy** (pico-fido ≤ v7.2 only). Removed in current firmware.
    Memory = 0x06,
}

/// Pico-fido vendor config command IDs (64-bit).
///
/// These are sent via `authenticatorConfig` → `VendorPrototype` (0xFF)
/// sub-command. Each ID identifies a specific hardware configuration
/// operation (LED, VID/PID, encryption, etc.).
///
/// RS-Key uses the same command IDs for compatibility.
#[repr(u64)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VendorConfigCommand {
    /// Enable authenticated encryption for secure communication.
    AuthEncryptionEnable = 0x03e43f56b34285e2,
    /// Disable authenticated encryption.
    AuthEncryptionDisable = 0x1831a40f04a25ed9,
    /// Upload enterprise attestation certificate.
    EnterpriseAttestationUpload = 0x66f2a674c29a8dcf,
    /// Configure PIN complexity policy.
    PinComplexityPolicy = 0x6c07d70fe96c3897,
    /// Set USB Vendor ID and Product ID.
    PhysicalVidPid = 0x6fcb19b0cbe3acfa,
    /// Set LED brightness level.
    PhysicalLedBrightness = 0x76a85945985d02fd,
    /// Set LED GPIO pin assignment.
    PhysicalLedGpio = 0x7b392a394de9f948,
    /// Physical options bitmask (dimmable, power-reset, steady LED).
    PhysicalOptions = 0x269f3b09eceb805f,
}

impl VendorConfigCommand {
    /// Convert a raw 64-bit value to a [`VendorConfigCommand`].
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

/// Certification identifiers reported by pico-fido/RS-Key firmware.
///
/// These share the same 64-bit IDs as [`VendorConfigCommand`] but are
/// used in the `GetInfo` certifications map to indicate which features
/// the device has been certified for.
#[repr(u64)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FidoCertification {
    /// Authenticated encryption enabled and certified.
    AuthEncryption = 0x03E43F56B34285E2,
    /// Authenticated encryption locked (cannot be disabled).
    AuthEncryptionLock = 0x1831A40F04A25ED9,
    /// Enterprise attestation certified.
    EnterpriseAttestation = 0x66F2A674C29A8DCF,
    /// PIN complexity policy enforced.
    PinComplexity = 0x6C07D70FE96C3897,
    /// Physical VID/PID configuration certified.
    PhysicalVidPid = 0x6FCB19B0CBE3ACFA,
    /// LED brightness control certified.
    LedBrightness = 0x76A85945985D02FD,
    /// LED GPIO assignment certified.
    LedGpio = 0x7B392A394DE9F948,
    /// Physical options (dimmable, power-reset, steady) certified.
    PhysicalOptions = 0x269F3B09ECEB805F,
}

impl FidoCertification {
    /// Convert a raw 64-bit value to a [`FidoCertification`].
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

    /// Parse a hex string (with or without `0x` prefix) to a [`FidoCertification`].
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

// ── Vendor sub-commands ─────────────────────────────────────────────────────

/// CBOR map keys for pico-fido vendor prototype commands.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VendorParam {
    /// Vendor command identifier (64-bit).
    VendorCommand = 0x01,
    /// Nested vendor sub-parameters.
    VendorSubParams = 0x02,
    /// PIN/UV protocol version.
    PinUvAuthProtocol = 0x03,
    /// HMAC for authentication.
    PinUvAuthParam = 0x04,
}

/// Sub-parameter keys inside vendor prototype payloads.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VendorSubParam {
    /// Raw vendor parameter value.
    VendorParam = 0x01,
    /// COSE-encoded public key.
    CoseKey = 0x02,
    /// Integer vendor parameter.
    VendorParamInt = 0x03,
    /// Text vendor parameter.
    VendorParamText = 0x04,
}

/// Backup sub-commands (encrypted backup/restore).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackupSubCommand {
    /// Export an encrypted backup blob.
    GetEncryptedBackup = 0x01,
    /// Import and restore an encrypted backup.
    RestoreEncryptedBackup = 0x02,
}

/// Manage Security Environment sub-commands.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MseSubCommand {
    /// Perform ECDH key agreement for secure channel setup.
    KeyAgreement = 0x01,
}

/// Enterprise attestation sub-commands.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnterpriseAttestationSubCommand {
    /// Generate a Certificate Signing Request.
    GenerateCsr = 0x01,
}

/// Physical options sub-commands (legacy vendor command 0x05).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhysicalOptionsSubCommand {
    /// Read the current physical options bitmask.
    GetOptions = 0x01,
}

/// Memory sub-commands (vendor command 0x06).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemorySubCommand {
    /// Get flash memory usage statistics.
    GetStats = 0x01,
}

/// Response keys for `Memory::GetStats`.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryResponseKey {
    /// Free space in bytes.
    FreeSpace = 0x01,
    /// Used space in bytes.
    UsedSpace = 0x02,
    /// Total flash capacity in bytes.
    TotalSpace = 0x03,
    /// Number of stored files/credentials.
    NumFiles = 0x04,
    /// Raw flash chip size.
    FlashSize = 0x05,
}

// ══════════════════════════════════════════════════════════════════════════════
// RS-KEY SPECIFIC EXTENSIONS
// ══════════════════════════════════════════════════════════════════════════════
//
// The following constants are specific to RS-Key firmware. Some overlap with
// pico-fido (RS-Key is a rust rewrite of pico-fido), but the Rescue applet AIDs and CTAPHID 0x41
// vendor command are RS-Key additions.

/// Vendor CBOR command opcode (pico-fido/RS-Key extension).
///
/// Sent as the first byte of a `CTAPHID_CBOR` payload to invoke
/// vendor-specific CBOR-encoded commands.
pub const CTAP_VENDOR_CBOR_CMD: u8 = 0xC1;

/// Vendor config command opcode (pico-fido/RS-Key extension).
///
/// Sent as the first byte of a `CTAPHID_CBOR` payload for
/// `authenticatorConfig` vendor prototype commands.
pub const CTAP_VENDOR_CONFIG_CMD: u8 = 0xC2;

/// RS-Key CTAPHID vendor command (0x41).
///
/// Carries CBOR-encoded sub-commands for seed backup, attestation,
/// and audit operations. This is RS-Key specific and not part of pico-fido.
///
/// See [RS-Key protocol §9](https://themaxmur.github.io/RS-Key/develop/) for details.
pub const RSKEY_CTAPHID_VENDOR_CMD: u8 = 0x41;

// ══════════════════════════════════════════════════════════════════════════════
// SHARED PROTOCOL CONSTANTS
// ══════════════════════════════════════════════════════════════════════════════

/// Size of the relying party identifier hash (SHA-256).
pub const CTAP_APPID_SIZE: usize = 32;
/// Size of the challenge hash (SHA-256).
pub const CTAP_CHAL_SIZE: usize = 32;
/// Size of an EC public key coordinate (P-256).
pub const CTAP_EC_KEY_SIZE: usize = 32;
/// Size of an uncompressed EC public key point (0x04 + X + Y).
pub const CTAP_EC_POINT_SIZE: usize = 65;
/// Maximum key handle size stored on the device.
pub const CTAP_MAX_KH_SIZE: usize = 128;
/// Default key handle length for credential serialization.
pub const KEY_HANDLE_LEN: usize = 64;
/// Maximum EC signature size (DER-encoded).
pub const CTAP_MAX_EC_SIG_SIZE: usize = 72;
/// Size of the transaction counter field.
pub const CTAP_CTR_SIZE: usize = 4;

/// Maximum number of PIN entry attempts before lockout.
pub const MAX_PIN_RETRIES: u8 = 8;
/// Maximum credentials returned in a single `GetAssertion` response.
pub const MAX_CREDENTIAL_COUNT_IN_LIST: usize = 16;
/// Maximum credential ID length in bytes.
pub const MAX_CRED_ID_LENGTH: usize = 1024;
/// Maximum number of discoverable (resident) credentials.
pub const MAX_RESIDENT_CREDENTIALS: usize = 256;
/// Maximum length of a credential blob extension.
pub const MAX_CREDBLOB_LENGTH: usize = 128;
/// Maximum CTAP2 message size in bytes.
pub const MAX_MSG_SIZE: usize = 1024;
/// Maximum fragment size (message size minus CTAPHID header).
pub const MAX_FRAGMENT_LENGTH: usize = MAX_MSG_SIZE - 64;
/// Maximum large blob array size in bytes.
pub const MAX_LARGE_BLOB_SIZE: usize = 2048;

/// Default AAGUID for pico-fido firmware.
///
/// Used to identify the authenticator model. Compare against
/// [`super::super::types::PICOFIDO_AAGUID`] or
/// [`super::super::types::RSKEY_AAGUID`] to determine firmware type.
pub const AAGUID: [u8; 16] = [
    0x89, 0xFB, 0x94, 0xB7, 0x06, 0xC9, 0x36, 0x73, 0x9B, 0x7E, 0x30, 0x52, 0x6D, 0x96, 0x81, 0x45,
];

/// CTAP 2.1 GetInfo response map keys (§11.5.3).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ctap2GetInfoKey {
    Versions = 0x01,
    Extensions = 0x02,
    Aaguid = 0x03,
    Options = 0x04,
    MaxMsgSize = 0x05,
    PinUvAuthProtocols = 0x06,
    MaxCredentialCountInList = 0x07,
    MaxCredentialIdLength = 0x08,
    RemainingDiscoverableCredentials = 0x14,
    FirmwareVersion = 0x0E,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── CTAP2 command codes ──────────────────────────────────────────────────
    // Reference: pico-fido src/fido/ctap.h: #define CTAP_MAKE_CREDENTIAL 0x01
    //                                               CTAP_GET_ASSERTION   0x02
    //                                               CTAP_GET_INFO        0x04
    //                                               CTAP_CLIENT_PIN      0x06
    //                                               CTAP_RESET           0x07
    //                                               CTAP_GET_NEXT_ASSERTION 0x08
    //                                               CTAP_CREDENTIAL_MGMT 0x0A
    //                                               CTAP_SELECTION       0x0B
    //                                               CTAP_LARGE_BLOBS     0x0C
    //                                               CTAP_CONFIG          0x0D

    #[test]
    fn test_ctap_command_values_match_firmware() {
        assert_eq!(CtapCommand::MakeCredential as u8, 0x01);
        assert_eq!(CtapCommand::GetAssertion as u8, 0x02);
        assert_eq!(CtapCommand::GetInfo as u8, 0x04);
        assert_eq!(CtapCommand::ClientPin as u8, 0x06);
        assert_eq!(CtapCommand::Reset as u8, 0x07);
        assert_eq!(CtapCommand::GetNextAssertion as u8, 0x08);
        assert_eq!(CtapCommand::CredentialMgmt as u8, 0x0A);
        assert_eq!(CtapCommand::Selection as u8, 0x0B);
        assert_eq!(CtapCommand::LargeBlobs as u8, 0x0C);
        assert_eq!(CtapCommand::Config as u8, 0x0D);
    }

    // ── U2F command codes ────────────────────────────────────────────────────
    // Reference: pico-fido src/fido/ctap.h: #define CTAP_REGISTER    0x01
    //                                               CTAP_AUTHENTICATE 0x02
    //                                               CTAP_VERSION      0x03

    #[test]
    fn test_u2f_command_values_match_firmware() {
        assert_eq!(U2fCommand::Register as u8, 0x01);
        assert_eq!(U2fCommand::Authenticate as u8, 0x02);
        assert_eq!(U2fCommand::Version as u8, 0x03);
    }

    // ── AuthenticateControl ──────────────────────────────────────────────────
    // Reference: pico-fido src/fido/ctap.h: #define CTAP_AUTH_ENFORCE    0x03
    //                                               CTAP_AUTH_CHECK_ONLY 0x07

    #[test]
    fn test_authenticate_control_values_match_firmware() {
        assert_eq!(AuthenticateControl::EnforceUserPresence as u8, 0x03);
        assert_eq!(AuthenticateControl::CheckOnly as u8, 0x07);
    }

    // ── Client PIN sub-commands ──────────────────────────────────────────────
    // Reference: CTAP 2.1 §11.5.4

    #[test]
    fn test_client_pin_sub_command_values_match_spec() {
        assert_eq!(ClientPinSubCommand::GetPinRetries as u8, 0x01);
        assert_eq!(ClientPinSubCommand::GetKeyAgreement as u8, 0x02);
        assert_eq!(ClientPinSubCommand::SetPin as u8, 0x03);
        assert_eq!(ClientPinSubCommand::ChangePin as u8, 0x04);
        assert_eq!(ClientPinSubCommand::GetPinToken as u8, 0x05);
        assert_eq!(
            ClientPinSubCommand::GetPinUvAuthTokenUsingUvWithPermissions as u8,
            0x06
        );
        assert_eq!(ClientPinSubCommand::GetUvRetries as u8, 0x07);
        assert_eq!(
            ClientPinSubCommand::GetPinUvAuthTokenUsingPinWithPermissions as u8,
            0x09
        );
    }

    // ── Config sub-commands ──────────────────────────────────────────────────
    // Reference: CTAP 2.1 §11.5.10

    #[test]
    fn test_config_sub_command_values_match_spec() {
        assert_eq!(ConfigSubCommand::EnableEnterpriseAttestation as u8, 0x01);
        assert_eq!(ConfigSubCommand::ToggleAlwaysUv as u8, 0x02);
        assert_eq!(ConfigSubCommand::SetMinPinLength as u8, 0x03);
        assert_eq!(ConfigSubCommand::VendorPrototype as u8, 0xFF);
    }

    // ── AuthenticatorFlags ───────────────────────────────────────────────────
    // Reference: pico-fido src/fido/fido.h:
    //   #define FIDO2_AUT_FLAG_UP 0x1
    //   #define FIDO2_AUT_FLAG_UV 0x4
    //   #define FIDO2_AUT_FLAG_AT 0x40
    //   #define FIDO2_AUT_FLAG_ED 0x80

    #[test]
    fn test_authenticator_flags_values_match_firmware() {
        assert_eq!(AuthenticatorFlags::USER_PRESENT.bits(), 0x01);
        assert_eq!(AuthenticatorFlags::USER_VERIFIED.bits(), 0x04);
        assert_eq!(AuthenticatorFlags::ATTESTED_CREDENTIAL_DATA.bits(), 0x40);
        assert_eq!(AuthenticatorFlags::EXTENSION_DATA.bits(), 0x80);
    }

    #[test]
    fn test_authenticator_flags_combine_correctly() {
        let up_uv = AuthenticatorFlags::USER_PRESENT | AuthenticatorFlags::USER_VERIFIED;
        assert_eq!(up_uv.bits(), 0x05);
        let full = up_uv
            | AuthenticatorFlags::ATTESTED_CREDENTIAL_DATA
            | AuthenticatorFlags::EXTENSION_DATA;
        assert_eq!(full.bits(), 0xC5);
    }

    // ── AuthenticatorOptions ─────────────────────────────────────────────────
    // Reference: pico-fido src/fido/fido.h:
    //   #define FIDO2_OPT_EA   0x01
    //   #define FIDO2_OPT_AUV  0x02

    #[test]
    fn test_authenticator_options_values_match_firmware() {
        assert_eq!(AuthenticatorOptions::ENTERPRISE_ATTESTATION.bits(), 0x01);
        assert_eq!(AuthenticatorOptions::USER_VERIFICATION.bits(), 0x02);
    }

    // ── PinUvAuthTokenPermissions ────────────────────────────────────────────
    // Reference: pico-fido src/fido/ctap.h:
    //   #define CTAP_PERMISSION_MC    0x01
    //   #define CTAP_PERMISSION_GA    0x02
    //   #define CTAP_PERMISSION_CM    0x04
    //   #define CTAP_PERMISSION_BE    0x08
    //   #define CTAP_PERMISSION_LBW   0x10
    //   #define CTAP_PERMISSION_ACFG  0x20
    //   #define CTAP_PERMISSION_PCMR  0x40

    #[test]
    fn test_pin_uv_auth_token_permissions_values_match_firmware() {
        assert_eq!(PinUvAuthTokenPermissions::MAKE_CREDENTIAL.bits(), 0x01);
        assert_eq!(PinUvAuthTokenPermissions::GET_ASSERTION.bits(), 0x02);
        assert_eq!(
            PinUvAuthTokenPermissions::CREDENTIAL_MANAGEMENT.bits(),
            0x04
        );
        assert_eq!(PinUvAuthTokenPermissions::BIO_ENROLLMENT.bits(), 0x08);
        assert_eq!(PinUvAuthTokenPermissions::LARGE_BLOB_WRITE.bits(), 0x10);
        assert_eq!(PinUvAuthTokenPermissions::AUTHENTICATOR_CONFIG.bits(), 0x20);
        assert_eq!(
            PinUvAuthTokenPermissions::PER_CREDENTIAL_MGMT_READONLY.bits(),
            0x40
        );
    }

    #[test]
    fn test_pin_uv_auth_token_permissions_combine() {
        let mc_ga =
            PinUvAuthTokenPermissions::MAKE_CREDENTIAL | PinUvAuthTokenPermissions::GET_ASSERTION;
        assert_eq!(mc_ga.bits(), 0x03);
        let all = mc_ga
            | PinUvAuthTokenPermissions::CREDENTIAL_MANAGEMENT
            | PinUvAuthTokenPermissions::AUTHENTICATOR_CONFIG;
        assert_eq!(all.bits(), 0x27);
    }

    // ── COSE Algorithms ──────────────────────────────────────────────────────
    // Reference: pico-fido src/fido/fido.h: #define FIDO2_ALG_* defines

    #[test]
    fn test_cose_algorithm_values_match_firmware() {
        assert_eq!(CoseAlgorithm::ES256 as i32, -7);
        assert_eq!(CoseAlgorithm::EdDSA as i32, -8);
        assert_eq!(CoseAlgorithm::ESP256 as i32, -9);
        assert_eq!(CoseAlgorithm::Ed25519 as i32, -19);
        assert_eq!(CoseAlgorithm::EcdhEsHkdf256 as i32, -25);
        assert_eq!(CoseAlgorithm::ES384 as i32, -35);
        assert_eq!(CoseAlgorithm::ES512 as i32, -36);
        assert_eq!(CoseAlgorithm::ES256K as i32, -47);
        assert_eq!(CoseAlgorithm::ESP384 as i32, -51);
        assert_eq!(CoseAlgorithm::ESP512 as i32, -52);
        assert_eq!(CoseAlgorithm::Ed448 as i32, -53);
        assert_eq!(CoseAlgorithm::RS256 as i32, -257);
        assert_eq!(CoseAlgorithm::RS384 as i32, -258);
        assert_eq!(CoseAlgorithm::RS512 as i32, -259);
        assert_eq!(CoseAlgorithm::ESB256 as i32, -265);
        assert_eq!(CoseAlgorithm::ESB384 as i32, -267);
        assert_eq!(CoseAlgorithm::ESB512 as i32, -268);
    }

    #[test]
    fn test_cose_algorithm_from_i128_roundtrip() {
        let test_cases = [
            -7, -8, -9, -19, -25, -35, -36, -47, -51, -52, -53, -257, -258, -259, -265, -267, -268,
            -48, -49, -50,
        ];
        for val in test_cases {
            let alg = CoseAlgorithm::from_i128(val as i128)
                .unwrap_or_else(|| panic!("from_i128({}) failed", val));
            assert_eq!(alg as i32, val);
        }
    }

    #[test]
    fn test_cose_algorithm_unknown_returns_none() {
        assert!(CoseAlgorithm::from_i128(0).is_none());
        assert!(CoseAlgorithm::from_i128(1).is_none());
        assert!(CoseAlgorithm::from_i128(-1).is_none());
        assert!(CoseAlgorithm::from_i128(-100).is_none());
        assert!(CoseAlgorithm::from_i128(-300).is_none());
    }

    #[test]
    fn test_cose_algorithm_display() {
        assert_eq!(CoseAlgorithm::ES256.to_string(), "ES256");
        assert_eq!(CoseAlgorithm::EdDSA.to_string(), "EdDSA");
        assert_eq!(CoseAlgorithm::MLDSA44.to_string(), "ML-DSA-44");
    }

    // ── COSE Curves ──────────────────────────────────────────────────────────
    // Reference: pico-fido src/fido/fido.h:
    //   #define FIDO2_CURVE_P256     1    #define FIDO2_CURVE_BP256R1  9
    //   #define FIDO2_CURVE_P384     2    #define FIDO2_CURVE_BP384R1 10
    //   #define FIDO2_CURVE_P521     3    #define FIDO2_CURVE_BP512R1 11
    //   #define FIDO2_CURVE_X25519   4
    //   #define FIDO2_CURVE_X448     5
    //   #define FIDO2_CURVE_ED25519  6
    //   #define FIDO2_CURVE_ED448    7
    //   #define FIDO2_CURVE_P256K1   8

    #[test]
    fn test_cose_curve_values_match_firmware() {
        assert_eq!(CoseCurve::P256 as u8, 1);
        assert_eq!(CoseCurve::P384 as u8, 2);
        assert_eq!(CoseCurve::P521 as u8, 3);
        assert_eq!(CoseCurve::X25519 as u8, 4);
        assert_eq!(CoseCurve::X448 as u8, 5);
        assert_eq!(CoseCurve::Ed25519 as u8, 6);
        assert_eq!(CoseCurve::Ed448 as u8, 7);
        assert_eq!(CoseCurve::P256K1 as u8, 8);
        assert_eq!(CoseCurve::BP256R1 as u8, 9);
        assert_eq!(CoseCurve::BP384R1 as u8, 10);
        assert_eq!(CoseCurve::BP512R1 as u8, 11);
    }

    // ── CTAP2 error codes ────────────────────────────────────────────────────
    // Reference: pico-fido src/fido/ctap.h: #define CTAP2_ERR_* defines

    #[test]
    fn test_ctap2_error_values_match_firmware() {
        assert_eq!(Ctap2Error::Success as u8, 0x00);
        assert_eq!(Ctap2Error::CborUnexpectedType as u8, 0x11);
        assert_eq!(Ctap2Error::InvalidCbor as u8, 0x12);
        assert_eq!(Ctap2Error::MissingParameter as u8, 0x14);
        assert_eq!(Ctap2Error::LimitExceeded as u8, 0x15);
        assert_eq!(Ctap2Error::FpDatabaseFull as u8, 0x17);
        assert_eq!(Ctap2Error::LargeBlobStorageFull as u8, 0x18);
        assert_eq!(Ctap2Error::CredentialExcluded as u8, 0x19);
        assert_eq!(Ctap2Error::Processing as u8, 0x21);
        assert_eq!(Ctap2Error::InvalidCredential as u8, 0x22);
        assert_eq!(Ctap2Error::UserActionPending as u8, 0x23);
        assert_eq!(Ctap2Error::OperationPending as u8, 0x24);
        assert_eq!(Ctap2Error::NoOperations as u8, 0x25);
        assert_eq!(Ctap2Error::UnsupportedAlgorithm as u8, 0x26);
        assert_eq!(Ctap2Error::OperationDenied as u8, 0x27);
        assert_eq!(Ctap2Error::KeyStoreFull as u8, 0x28);
        assert_eq!(Ctap2Error::UnsupportedOption as u8, 0x2B);
        assert_eq!(Ctap2Error::InvalidOption as u8, 0x2C);
        assert_eq!(Ctap2Error::KeepaliveCancel as u8, 0x2D);
        assert_eq!(Ctap2Error::NoCredentials as u8, 0x2E);
        assert_eq!(Ctap2Error::UserActionTimeout as u8, 0x2F);
        assert_eq!(Ctap2Error::NotAllowed as u8, 0x30);
        assert_eq!(Ctap2Error::PinInvalid as u8, 0x31);
        assert_eq!(Ctap2Error::PinBlocked as u8, 0x32);
        assert_eq!(Ctap2Error::PinAuthInvalid as u8, 0x33);
        assert_eq!(Ctap2Error::PinAuthBlocked as u8, 0x34);
        assert_eq!(Ctap2Error::PinNotSet as u8, 0x35);
        assert_eq!(Ctap2Error::PuatRequired as u8, 0x36);
        assert_eq!(Ctap2Error::PinPolicyViolation as u8, 0x37);
        assert_eq!(Ctap2Error::RequestTooLarge as u8, 0x39);
        assert_eq!(Ctap2Error::ActionTimeout as u8, 0x3A);
        assert_eq!(Ctap2Error::UpRequired as u8, 0x3B);
        assert_eq!(Ctap2Error::UvBlocked as u8, 0x3C);
        assert_eq!(Ctap2Error::IntegrityFailure as u8, 0x3D);
        assert_eq!(Ctap2Error::InvalidSubcommand as u8, 0x3E);
        assert_eq!(Ctap2Error::UvInvalid as u8, 0x3F);
        assert_eq!(Ctap2Error::UnauthorizedPermission as u8, 0x40);
    }

    // ── VendorCommand codes ──────────────────────────────────────────────────
    // Reference: pico-fido src/fido/ctap.h:
    //   #define CTAP_VENDOR_BACKUP     0x01
    //   #define CTAP_VENDOR_MSE        0x02
    //   #define CTAP_VENDOR_UNLOCK     0x03
    //   #define CTAP_VENDOR_EA         0x04
    //   #define CTAP_VENDOR_ADMIN_PIN  0x08
    // Note: PhysicalOptions(0x05) and Memory(0x06) are legacy (<=v7.2) and
    // were removed in later firmware releases.

    #[test]
    fn test_vendor_command_values_match_firmware() {
        assert_eq!(VendorCommand::Backup as u8, 0x01);
        assert_eq!(VendorCommand::ManageSecurityEnvironment as u8, 0x02);
        assert_eq!(VendorCommand::Unlock as u8, 0x03);
        assert_eq!(VendorCommand::EnterpriseAttestation as u8, 0x04);
        // PhysicalOptions(0x05) and Memory(0x06) are legacy <=v7.2
        assert_eq!(VendorCommand::PhysicalOptions as u8, 0x05);
        assert_eq!(VendorCommand::Memory as u8, 0x06);
    }

    // ── RS-Key vendor command ────────────────────────────────────────────────
    // Reference: RS-Key protocol docs §9

    #[test]
    fn test_rskey_vendor_cmd_value() {
        assert_eq!(RSKEY_CTAPHID_VENDOR_CMD, 0x41);
    }

    // ── Shared protocol constants ────────────────────────────────────────────
    // Reference: pico-fido src/fido/fido.h, src/fido/ctap.h

    #[test]
    fn test_size_constants_match_firmware() {
        assert_eq!(CTAP_APPID_SIZE, 32);
        assert_eq!(CTAP_CHAL_SIZE, 32);
        assert_eq!(CTAP_EC_KEY_SIZE, 32);
        assert_eq!(CTAP_EC_POINT_SIZE, 65);
        assert_eq!(CTAP_MAX_KH_SIZE, 128);
        assert_eq!(KEY_HANDLE_LEN, 64);
        assert_eq!(CTAP_MAX_EC_SIG_SIZE, 72);
        assert_eq!(CTAP_CTR_SIZE, 4);
        assert_eq!(MAX_PIN_RETRIES, 8);
        assert_eq!(MAX_CREDENTIAL_COUNT_IN_LIST, 16);
        assert_eq!(MAX_CRED_ID_LENGTH, 1024);
        assert_eq!(MAX_RESIDENT_CREDENTIALS, 256);
        assert_eq!(MAX_CREDBLOB_LENGTH, 128);
        assert_eq!(MAX_MSG_SIZE, 1024);
        assert_eq!(MAX_FRAGMENT_LENGTH, 960);
        assert_eq!(MAX_LARGE_BLOB_SIZE, 2048);
    }

    // ── Vendor config command IDs ────────────────────────────────────────────
    // Reference: pico-fido src/fido/ctap.h (for auth/enable/disable/EA/PIN)
    //            RS-Key protocol docs §11 (for physical config commands)
    //
    // NOTE: The auth encryption and PIN policy IDs in PicoForge do NOT match the
    // current pico-fido ctap.h values. The ctap.h values are documented below
    // for reference but the PicoForge values may target an older firmware version.
    //
    // Firmware ctap.h values:
    //   AuthEncryptionEnable:   0x00043f56b34285e2
    //   AuthEncryptionDisable:  0x0001a40f04a25ed9
    //   EnterpriseAttestationUpload: 0x0002a674c29a8dcf
    //   PinComplexityPolicy:    0x0007d70fe96c3897
    //
    // PicoForge values (verified against RS-Key protocol for physical ones):

    #[test]
    fn test_vendor_config_command_from_u64() {
        assert_eq!(
            VendorConfigCommand::from_u64(0x03e43f56b34285e2),
            Some(VendorConfigCommand::AuthEncryptionEnable)
        );
        assert_eq!(
            VendorConfigCommand::from_u64(0x1831a40f04a25ed9),
            Some(VendorConfigCommand::AuthEncryptionDisable)
        );
        assert_eq!(
            VendorConfigCommand::from_u64(0x66f2a674c29a8dcf),
            Some(VendorConfigCommand::EnterpriseAttestationUpload)
        );
        assert_eq!(
            VendorConfigCommand::from_u64(0x6c07d70fe96c3897),
            Some(VendorConfigCommand::PinComplexityPolicy)
        );
    }

    #[test]
    fn test_physical_vendor_config_ids_match_rskey_protocol() {
        // These 4 values are verified against RS-Key protocol docs §11
        assert_eq!(
            VendorConfigCommand::from_u64(0x6fcb19b0cbe3acfa),
            Some(VendorConfigCommand::PhysicalVidPid)
        );
        assert_eq!(
            VendorConfigCommand::from_u64(0x7b392a394de9f948),
            Some(VendorConfigCommand::PhysicalLedGpio)
        );
        assert_eq!(
            VendorConfigCommand::from_u64(0x76a85945985d02fd),
            Some(VendorConfigCommand::PhysicalLedBrightness)
        );
        assert_eq!(
            VendorConfigCommand::from_u64(0x269f3b09eceb805f),
            Some(VendorConfigCommand::PhysicalOptions)
        );
    }

    #[test]
    fn test_vendor_config_command_unknown_returns_none() {
        assert!(VendorConfigCommand::from_u64(0).is_none());
        assert!(VendorConfigCommand::from_u64(0xDEADBEEF).is_none());
    }

    #[test]
    fn test_vendor_config_command_display() {
        assert_eq!(
            VendorConfigCommand::AuthEncryptionEnable.to_string(),
            "AuthEncryptionEnable"
        );
        assert_eq!(
            VendorConfigCommand::PhysicalVidPid.to_string(),
            "PhysicalVidPid"
        );
    }

    // ── FidoCertification ────────────────────────────────────────────────────

    #[test]
    fn test_fido_certification_from_str() {
        assert_eq!(
            FidoCertification::from_str("0x03E43F56B34285E2"),
            Some(FidoCertification::AuthEncryption)
        );
        assert_eq!(
            FidoCertification::from_str("03E43F56B34285E2"),
            Some(FidoCertification::AuthEncryption)
        );
        assert_eq!(
            FidoCertification::from_str("0x6FCB19B0CBE3ACFA"),
            Some(FidoCertification::PhysicalVidPid)
        );
        assert!(FidoCertification::from_str("unknown").is_none());
    }

    #[test]
    fn test_fido_certification_display() {
        assert_eq!(
            FidoCertification::AuthEncryption.to_string(),
            "Auth Encryption"
        );
        assert_eq!(
            FidoCertification::PhysicalVidPid.to_string(),
            "Physical VID/PID"
        );
        assert_eq!(
            FidoCertification::PinComplexity.to_string(),
            "PIN Complexity"
        );
    }

    // ── Credential management sub-commands ────────────────────────────────────

    #[test]
    fn test_credential_mgmt_sub_command_values() {
        assert_eq!(CredentialMgmtSubCommand::GetCredsMetadata as u8, 0x01);
        assert_eq!(CredentialMgmtSubCommand::EnumerateRpsBegin as u8, 0x02);
        assert_eq!(CredentialMgmtSubCommand::EnumerateRpsGetNextRp as u8, 0x03);
        assert_eq!(
            CredentialMgmtSubCommand::EnumerateCredentialsBegin as u8,
            0x04
        );
        assert_eq!(
            CredentialMgmtSubCommand::EnumerateCredentialsGetNextCredential as u8,
            0x05
        );
        assert_eq!(CredentialMgmtSubCommand::DeleteCredential as u8, 0x06);
        assert_eq!(CredentialMgmtSubCommand::UpdateUserInformation as u8, 0x07);
    }

    // ── AAGUID ───────────────────────────────────────────────────────────────

    #[test]
    fn test_aaguid_is_pico_fido_default() {
        let expected: [u8; 16] = [
            0x89, 0xFB, 0x94, 0xB7, 0x06, 0xC9, 0x36, 0x73, 0x9B, 0x7E, 0x30, 0x52, 0x6D, 0x96,
            0x81, 0x45,
        ];
        assert_eq!(AAGUID, expected);
        assert_eq!(AAGUID.len(), 16);
    }

    // ── Vendor command constants ─────────────────────────────────────────────

    #[test]
    fn test_vendor_cbor_and_config_cmds() {
        assert_eq!(CTAP_VENDOR_CBOR_CMD, 0xC1);
        assert_eq!(CTAP_VENDOR_CONFIG_CMD, 0xC2);
    }

    // ── GetInfo response key constants ───────────────────────────────────────
    // These are the CTAP 2.1 GetInfo response keys

    #[test]
    fn test_get_info_response_keys() {
        assert_eq!(Ctap2GetInfoKey::Versions as u8, 0x01);
        assert_eq!(Ctap2GetInfoKey::Extensions as u8, 0x02);
        assert_eq!(Ctap2GetInfoKey::Aaguid as u8, 0x03);
        assert_eq!(Ctap2GetInfoKey::Options as u8, 0x04);
        assert_eq!(Ctap2GetInfoKey::MaxMsgSize as u8, 0x05);
        assert_eq!(Ctap2GetInfoKey::PinUvAuthProtocols as u8, 0x06);
        assert_eq!(Ctap2GetInfoKey::MaxCredentialCountInList as u8, 0x07);
        assert_eq!(Ctap2GetInfoKey::MaxCredentialIdLength as u8, 0x08);
        assert_eq!(Ctap2GetInfoKey::FirmwareVersion as u8, 0x0E);
        assert_eq!(
            Ctap2GetInfoKey::RemainingDiscoverableCredentials as u8,
            0x14
        );
    }

    // ── MakeCredential param keys ────────────────────────────────────────────

    #[test]
    fn test_make_credential_param_keys() {
        assert_eq!(MakeCredentialParam::ClientDataHash as u8, 0x01);
        assert_eq!(MakeCredentialParam::Rp as u8, 0x02);
        assert_eq!(MakeCredentialParam::User as u8, 0x03);
        assert_eq!(MakeCredentialParam::PubKeyCredParams as u8, 0x04);
        assert_eq!(MakeCredentialParam::ExcludeList as u8, 0x05);
        assert_eq!(MakeCredentialParam::EnterpriseAttestation as u8, 0x0A);
    }
}
