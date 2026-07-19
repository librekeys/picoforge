//! Low-level FIDO2 operations implementing the CTAP2 PIN/UV auth protocol,
//! credential management, and firmware-specific vendor commands.
//!
//! The [`FidoOperations`] trait is implemented on [`HidTransport`] and provides
//! the building blocks used by the high-level functions in [`super`].

use cbc::cipher::{Block, BlockModeDecrypt, BlockModeEncrypt, KeyIvInit, block_padding::NoPadding};

use ring::{agreement, digest, hmac};
use serde_cbor_2::{Value, from_slice, to_vec};
use std::collections::BTreeMap;

use crate::error::PFError;
use crate::hal::fido::constants::*;
use crate::hal::transport::fido::{CTAPHID_CBOR, HidTransport};

/// Returned by [`HidTransport::credential_management_enumerate_rps`]. Each entry
/// represents one RP stored on the authenticator.
#[derive(Debug, Clone)]
pub struct EnumerateRpResponse {
    pub rp: Value,
    pub rp_id_hash: Vec<u8>,
    #[allow(dead_code)]
    pub total_rps: Option<usize>,
}

/// Response from enumerating a credential via credential management.
///
/// Returned by [`HidTransport::credential_management_enumerate_credentials`].
/// Each entry represents one credential (public key) registered under an RP.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct EnumerateCredentialResponse {
    pub user: Value,
    pub credential_id: Value,
    pub public_key: Value,
    #[allow(dead_code)]
    pub total_credentials: Option<usize>,
}

/// Low-level CTAP2 operations implemented on the FIDO HID transport.
///
/// Each method encodes the appropriate CBOR map, sends it via
/// [`HidTransport::send_cbor`], and parses the response. PIN operations
/// follow the ECDH + AES-256-CBC key-agreement flow defined in
/// CTAP2 §11.5.4.
pub trait FidoOperations {
    /// Send a vendor-prototype config sub-command (pico-fido specific).
    fn send_vendor_config(
        &self,
        pin_token: &[u8],
        vendor_cmd: VendorConfigCommand,
        param: Value,
    ) -> Result<(), PFError>;
    /// Retrieve the enterprise attestation CSR from the authenticator.
    fn get_enterprise_attestation_csr(&self) -> Result<Vec<u8>, PFError>;
    /// Send an `authenticatorConfig` sub-command.
    fn send_config(
        &self,
        sub_cmd: ConfigSubCommand,
        pin_token: &[u8],
        sub_params: Option<Value>,
    ) -> Result<Vec<u8>, PFError>;
    /// Enable enterprise attestation via config sub-command.
    fn send_config_enable_ea(&self, pin_token: &[u8]) -> Result<(), PFError>;
    /// Set the minimum PIN length via config sub-command.
    fn send_config_set_min_pin_length(
        &self,
        pin_token: &[u8],
        new_min_pin_length: u8,
    ) -> Result<(), PFError>;
    /// Retrieve the authenticator's ECDH P-256 public key for PIN token exchange.
    fn get_key_agreement(&self) -> Result<Value, PFError>;
    /// Derive a PIN token from the user-supplied PIN.
    fn get_pin_token(&self, pin: &str) -> Result<Vec<u8>, PFError>;
    /// Derive a PIN token scoped to specific permissions (e.g. credential management).
    fn get_pin_token_with_permission(
        &self,
        pin: &str,
        permissions: PinUvAuthTokenPermissions,
        rp_id: Option<String>,
    ) -> Result<Vec<u8>, PFError>;
    /// Set a new PIN on the authenticator.
    fn set_pin(&self, new_pin: &str) -> Result<(), PFError>;
    /// Change an existing PIN on the authenticator.
    fn change_pin(&self, current_pin: &str, new_pin: &str) -> Result<(), PFError>;
    /// Compute a pinUvAuthToken signature for an `authenticatorConfig` sub-command.
    fn sign_config_command(
        &self,
        pin_token: &[u8],
        sub_cmd: u8,
        sub_params_bytes: &[u8],
    ) -> Vec<u8>;
    /// Encode an ECDH public key as a COSE_Key map (used in PIN exchanges).
    fn encode_cose_key(&self, x: &[u8], y: &[u8]) -> Vec<u8>;
    /// Build the CBOR map for a `clientPin` sub-command.
    fn encode_client_pin_params(
        &self,
        sub_cmd: ClientPinSubCommand,
        cose_key_bytes: &[u8],
        pin_hash_enc: &[u8],
        permissions: Option<u8>,
        rp_id: Option<String>,
    ) -> Vec<u8>;
    /// Enumerate all relying parties stored on the authenticator.
    fn credential_management_enumerate_rps(
        &self,
        pin: &str,
    ) -> Result<Vec<EnumerateRpResponse>, PFError>;
    /// Enumerate all credentials for a given relying party.
    fn credential_management_enumerate_credentials(
        &self,
        pin: &str,
        rp_id_hash: &[u8],
    ) -> Result<Vec<EnumerateCredentialResponse>, PFError>;
    /// Delete a credential from the authenticator.
    fn credential_management_delete_credential(
        &self,
        pin: &str,
        credential_id_map: Value,
    ) -> Result<(), PFError>;
    /// Read RS-Key configuration via the 0x41 CONFIG_READ vendor command.
    fn rs_key_config_read(&self, target: u8) -> Result<Vec<u8>, PFError>;
    /// Write RS-Key configuration via the 0x41 CONFIG_WRITE vendor command.
    fn rs_key_config_write(&self, pin_token: &[u8], target: u8, blob: &[u8])
    -> Result<(), PFError>;
    /// Compute a pinUvAuthToken signature for a credential management sub-command.
    fn sign_credential_mgmt_command(
        &self,
        pin_token: &[u8],
        sub_cmd: u8,
        sub_params_bytes: Option<&[u8]>,
    ) -> Vec<u8>;
}

impl FidoOperations for HidTransport {
    /// determines which CBOR key (0x02/0x03/0x04) is used.
    fn send_vendor_config(
        &self,
        pin_token: &[u8],
        vendor_cmd: VendorConfigCommand,
        param: Value,
    ) -> Result<(), PFError> {
        log::debug!("Sending vendor config command: {}...", vendor_cmd);

        // Build subCommandParams (Key 0x02)
        // This map contains:
        // 0x01: vendorCommandId (u64)
        // 0x02/0x03/0x04: param
        let mut sub_params_inner = BTreeMap::new();
        sub_params_inner.insert(Value::Integer(0x01), Value::Integer(vendor_cmd as i128));

        match param {
            Value::Bytes(_) => {
                sub_params_inner.insert(Value::Integer(0x02), param.clone());
            }
            Value::Integer(_) => {
                sub_params_inner.insert(Value::Integer(0x03), param.clone());
            }
            Value::Text(_) => {
                sub_params_inner.insert(Value::Integer(0x04), param.clone());
            }
            _ => return Err(PFError::Io("Unsupported parameter type".into())),
        }

        let sub_params = Value::Map(sub_params_inner);
        let sub_params_bytes = to_vec(&sub_params).map_err(|e| PFError::Io(e.to_string()))?;

        // Calculate PIN Auth
        let pin_auth = self.sign_config_command(
            pin_token,
            ConfigSubCommand::VendorPrototype as u8,
            &sub_params_bytes,
        );

        // Build full authenticatorConfig map
        let mut config_map = BTreeMap::new();
        config_map.insert(
            Value::Integer(ConfigParam::SubCommand as i128),
            Value::Integer(ConfigSubCommand::VendorPrototype as i128),
        );
        config_map.insert(
            Value::Integer(ConfigParam::SubCommandParams as i128),
            sub_params,
        );
        config_map.insert(
            Value::Integer(ConfigParam::PinUvAuthProtocol as i128),
            Value::Integer(1),
        );
        config_map.insert(
            Value::Integer(ConfigParam::PinUvAuthParam as i128),
            Value::Bytes(pin_auth),
        );

        let config_payload_cbor =
            to_vec(&Value::Map(config_map)).map_err(|e| PFError::Io(e.to_string()))?;

        // Encapsulate for CTAP
        let mut payload = vec![CtapCommand::Config as u8];
        payload.extend(config_payload_cbor);

        log::debug!("Sending config command...");
        self.send_cbor(CTAPHID_CBOR, &payload).map_err(|e| {
            log::error!("Failed to send FIDO config: {}", e);
            PFError::Device(format!("FIDO config failed: {}", e))
        })?;

        Ok(())
    }

    /// Send CTAP_VENDOR_EA (0x04) with GenerateCsr sub-command and return the raw DER bytes.
    ///
    /// This calls the pico-fido enterprise attestation vendor command to generate a
    /// Certificate Signing Request (CSR) for the device's attestation key.
    fn get_enterprise_attestation_csr(&self) -> Result<Vec<u8>, PFError> {
        log::debug!("Requesting Enterprise Attestation CSR (CTAP_VENDOR_EA)...");

        let mut req = BTreeMap::new();
        req.insert(
            Value::Integer(1),
            Value::Integer(EnterpriseAttestationSubCommand::GenerateCsr as i128),
        );

        let cbor = to_vec(&Value::Map(req)).map_err(|e| PFError::Io(e.to_string()))?;
        let mut payload = vec![VendorCommand::EnterpriseAttestation as u8];
        payload.extend(cbor);

        let response = self
            .send_cbor(CTAP_VENDOR_CBOR_CMD, &payload)
            .map_err(|e| PFError::Device(format!("CSR request failed: {}", e)))?;

        if response.is_empty() {
            return Err(PFError::Device(
                "Empty response to CSR request from device".into(),
            ));
        }

        // The device returns the DER-encoded CSR. Try to unwrap from CBOR if present.
        match from_slice::<Value>(&response) {
            Ok(Value::Bytes(b)) => {
                log::debug!("CSR received as CBOR byte string ({} bytes)", b.len());
                Ok(b)
            }
            Ok(Value::Map(m)) => {
                // Prefer key 0x01 (common for first response field)
                if let Some(Value::Bytes(b)) = m.get(&Value::Integer(1)) {
                    log::debug!("CSR found at map key 0x01 ({} bytes)", b.len());
                    return Ok(b.clone());
                }
                // Fall back to the first bytes value in the map
                for v in m.values() {
                    if let Value::Bytes(b) = v {
                        log::debug!("CSR found in map value ({} bytes)", b.len());
                        return Ok(b.clone());
                    }
                }
                log::warn!(
                    "CSR response was a CBOR map but contained no byte values; returning raw bytes"
                );
                Ok(response)
            }
            _ => {
                // Assume the response is raw DER bytes (not CBOR-wrapped)
                log::debug!(
                    "CSR response treated as raw DER bytes ({} bytes)",
                    response.len()
                );
                Ok(response)
            }
        }
    }

    /// Send authenticatorConfig command.
    ///
    /// This bypasses the ctap-hid-fido2 library which has a bug where it sends
    /// CBOR map keys out of order (0x01, 0x03, 0x04, 0x02) instead of the required
    /// ascending order (0x01, 0x02, 0x03, 0x04). The pico-fido firmware strictly
    /// enforces canonical CBOR ordering per CTAP2 spec.
    ///
    /// Builds the authenticatorConfig CBOR map with keys in ascending order, signs
    /// it with the PIN token, and sends it as a CTAP Config command.
    fn send_config(
        &self,
        sub_cmd: ConfigSubCommand,
        pin_token: &[u8],
        sub_params: Option<Value>,
    ) -> Result<Vec<u8>, PFError> {
        let mut sub_params_bytes: Vec<u8> = Vec::new();

        if let Some(ref params) = sub_params {
            sub_params_bytes = to_vec(&params).map_err(|e| PFError::Io(e.to_string()))?;
        }

        // Calculate PIN Auth
        let pin_auth = self.sign_config_command(pin_token, sub_cmd as u8, &sub_params_bytes);

        // Build full authenticatorConfig map with keys in ASCENDING ORDER
        let mut config_map = BTreeMap::new();
        config_map.insert(
            Value::Integer(ConfigParam::SubCommand as i128), // 0x01
            Value::Integer(sub_cmd as i128),
        );
        if let Some(params) = sub_params {
            config_map.insert(
                Value::Integer(ConfigParam::SubCommandParams as i128), // 0x02
                params,
            );
        }
        config_map.insert(
            Value::Integer(ConfigParam::PinUvAuthProtocol as i128), // 0x03
            Value::Integer(1),                                      // PIN protocol version 1
        );
        config_map.insert(
            Value::Integer(ConfigParam::PinUvAuthParam as i128), // 0x04
            Value::Bytes(pin_auth),
        );

        let config_payload_cbor =
            to_vec(&Value::Map(config_map)).map_err(|e| PFError::Io(e.to_string()))?;

        // Prepend CTAP command byte
        let mut payload = vec![CtapCommand::Config as u8];
        payload.extend(config_payload_cbor);

        self.send_cbor(CTAPHID_CBOR, &payload)
    }

    /// Send authenticatorConfig command to enable Enterprise attestation.
    ///
    /// Calls the EnableEnterpriseAttestation sub-command (0x01) via [`send_config`](HidTransport::send_config).
    /// Enterprise attestation allows RPs to receive a per-device attestation certificate
    /// during MakeCredential, enabling enterprise device identification.
    fn send_config_enable_ea(&self, pin_token: &[u8]) -> Result<(), PFError> {
        log::debug!("Sending Enterprise Attestation enable config command...");
        match self.send_config(
            ConfigSubCommand::EnableEnterpriseAttestation,
            pin_token,
            None,
        ) {
            Ok(_) => {
                log::info!("Successfully enable Enterprise Attestation");
                Ok(())
            }
            Err(e) => {
                let error_string = e.to_string();
                log::error!("Failed to enable Enterprise Attestation: {}", error_string);
                Err(PFError::Device(format!(
                    "EnableEnterpriseAttestation failed: {}",
                    e
                )))
            }
        }
    }

    /// Send authenticatorConfig command to set minimum PIN length.
    ///
    /// Calls the SetMinPinLength sub-command (0x03) via [`send_config`](HidTransport::send_config).
    /// The minimum PIN length can only be increased; attempting to decrease it returns
    /// `PIN_POLICY_VIOLATION` (0x37). A device reset is required to lower the minimum.
    fn send_config_set_min_pin_length(
        &self,
        pin_token: &[u8],
        new_min_pin_length: u8,
    ) -> Result<(), PFError> {
        log::debug!(
            "Sending setMinPINLength config command (new length: {})...",
            new_min_pin_length
        );

        // Build subCommandParams (Key 0x02): { 0x01: newMinPINLength }
        let mut sub_params_map = BTreeMap::new();
        sub_params_map.insert(
            Value::Integer(ConfigSubCommandParam::NewMinPinLength as i128),
            Value::Integer(new_min_pin_length as i128),
        );
        let sub_params = Value::Map(sub_params_map);
        match self.send_config(
            ConfigSubCommand::SetMinPinLength,
            pin_token,
            Some(sub_params),
        ) {
            Ok(_) => {
                log::info!(
                    "Successfully set minimum PIN length to {}",
                    new_min_pin_length
                );
                Ok(())
            }
            Err(e) => {
                let error_string = e.to_string();
                log::error!("Failed to send setMinPINLength config: {}", error_string);

                // Check for PIN policy violation (0x37) - cannot decrease min PIN length
                if error_string.contains("0x37") {
                    return Err(PFError::Device(
                        "Cannot decrease minimum PIN length. The FIDO2 security policy only allows increasing the minimum PIN length, not decreasing it. A device reset is required to lower the minimum.".into()
                    ));
                }

                Err(PFError::Device(format!("setMinPINLength failed: {}", e)))
            }
        }
    }

    /// Request the authenticator's P-256 ECDH public key for PIN protocol v1.
    ///
    /// Sends a `getClientPin` command with `getKeyAgreement` sub-command (0x02).
    /// The returned COSE Key contains the authenticator's ephemeral public key
    /// (x and y coordinates) used for ECDH key agreement in PIN operations.
    fn get_key_agreement(&self) -> Result<Value, PFError> {
        let mut map = BTreeMap::new();
        map.insert(
            Value::Integer(ClientPinParam::PinUvAuthProtocol as i128),
            Value::Integer(1),
        );
        map.insert(
            Value::Integer(ClientPinParam::SubCommand as i128),
            Value::Integer(ClientPinSubCommand::GetKeyAgreement as i128),
        );

        let mut payload = vec![CtapCommand::ClientPin as u8];
        payload.extend(to_vec(&Value::Map(map)).map_err(|e| PFError::Io(e.to_string()))?);

        log::debug!("Sending GetKeyAgreement command...");
        let response = self.send_cbor(CTAPHID_CBOR, &payload)?;
        let val: Value = from_slice(&response).map_err(|e| PFError::Io(e.to_string()))?;

        if let Value::Map(m) = val {
            log::debug!("GetKeyAgreement response: {:?}", m);
            m.get(&Value::Integer(
                ClientPinResponseParam::KeyAgreement as i128,
            ))
            .cloned()
            .ok_or_else(|| PFError::Device("KeyAgreement not found in response".into()))
        } else {
            Err(PFError::Device(
                "Unexpected response for GetKeyAgreement".into(),
            ))
        }
    }

    /// Obtain an encrypted PIN token using the standard getPinToken flow.
    ///
    /// Implements the full CTAP2 §11.5.4 PIN token acquisition:
    /// 1. Fetches the authenticator's key agreement public key.
    /// 2. Generates an ephemeral P-256 key pair on the platform.
    /// 3. Performs ECDH and derives `SHA-256(shared_secret)`.
    /// 4. Encrypts the first 16 bytes of `SHA-256(pin)` with AES-256-CBC.
    /// 5. Sends getPinToken (sub-command 0x05) and decrypts the response token.
    fn get_pin_token(&self, pin: &str) -> Result<Vec<u8>, PFError> {
        log::info!("Starting custom get_pin_token (Subcommand 0x05)...");

        // 1. Get Authenticator Key Agreement
        let auth_key_agreement = self.get_key_agreement()?;

        // 2. Generate Platform Key Pair (P-256)
        let system_rng = ring::rand::SystemRandom::new();
        let platform_private_key =
            agreement::EphemeralPrivateKey::generate(&agreement::ECDH_P256, &system_rng)
                .map_err(|_| PFError::Device("Failed to generate platform ephemeral key".into()))?;
        let platform_public_key_bytes = platform_private_key
            .compute_public_key()
            .map_err(|_| PFError::Device("Failed to compute platform public key".into()))?;

        // 3. Extract Authenticator Public Key (X and Y coordinates)
        let (auth_point_x, auth_point_y) = if let Value::Map(m) = &auth_key_agreement {
            let x = match m.get(&Value::Integer(-2)) {
                Some(Value::Bytes(b)) => b,
                _ => return Err(PFError::Device("Invalid KeyAgreement X coordinate".into())),
            };
            let y = match m.get(&Value::Integer(-3)) {
                Some(Value::Bytes(b)) => b,
                _ => return Err(PFError::Device("Invalid KeyAgreement Y coordinate".into())),
            };
            (x, y)
        } else {
            return Err(PFError::Device("Invalid KeyAgreement format".into()));
        };

        let mut auth_pub_key_bytes = vec![0x04];
        auth_pub_key_bytes.extend(auth_point_x);
        auth_pub_key_bytes.extend(auth_point_y);

        let auth_unparsed_pub_key =
            agreement::UnparsedPublicKey::new(&agreement::ECDH_P256, auth_pub_key_bytes);

        // 4. Perform ECDH to get Shared Secret
        let shared_secret =
            agreement::agree_ephemeral(platform_private_key, &auth_unparsed_pub_key, |material| {
                let mut hasher = digest::Context::new(&digest::SHA256);
                hasher.update(material);
                Ok(hasher.finish()) as Result<digest::Digest, ring::error::Unspecified>
            })
            .map_err(|_| PFError::Device("ECDH shared secret computation failed".into()))?
            .map_err(|_| PFError::Device("Inner ECDH shared secret computation failed".into()))?;

        // 5. Encrypt PIN Hash
        let pin_hash = digest::digest(&digest::SHA256, pin.as_bytes());
        let pin_hash_16 = &pin_hash.as_ref()[0..16];

        let iv = [0u8; 16];
        let mut block = Block::<aes::Aes256>::try_from(pin_hash_16).unwrap();

        let shared_secret_bytes = shared_secret.as_ref();
        let mut encryptor =
            cbc::Encryptor::<aes::Aes256>::new_from_slices(shared_secret_bytes, &iv).unwrap();
        encryptor.encrypt_block(&mut block);
        let pin_hash_enc = block.to_vec();

        // 6. Send getPinToken command (Subcommand 0x05)

        // 7. Send getPinToken command (Subcommand 0x05)
        let cose_key_bytes = self.encode_cose_key(
            &platform_public_key_bytes.as_ref()[1..33],
            &platform_public_key_bytes.as_ref()[33..65],
        );

        let payload_cbor = self.encode_client_pin_params(
            ClientPinSubCommand::GetPinToken,
            &cose_key_bytes,
            &pin_hash_enc,
            None,
            None,
        );

        let mut payload = vec![CtapCommand::ClientPin as u8];
        payload.extend(payload_cbor);

        log::debug!("Sending getPinToken command...");
        let response = self.send_cbor(CTAPHID_CBOR, &payload)?;
        let val: Value = from_slice(&response).map_err(|e| PFError::Io(e.to_string()))?;

        if let Value::Map(m) = val {
            log::debug!("getPinToken response: {:?}", m);
            match m.get(&Value::Integer(ClientPinResponseParam::PinToken as i128)) {
                Some(Value::Bytes(token_enc)) => {
                    // Decrypt the PIN token using shared secret (AES-256-CBC, IV=0)
                    let mut token_buf = token_enc.clone();
                    let decrypted =
                        cbc::Decryptor::<aes::Aes256>::new_from_slices(shared_secret_bytes, &iv)
                            .map_err(|_| PFError::Device("Failed to create decryptor".into()))?
                            .decrypt_padded::<NoPadding>(&mut token_buf)
                            .map_err(|_| PFError::Device("Failed to decrypt PIN token".into()))?;
                    log::info!("Successfully obtained and decrypted PIN token (Subcommand 0x05).");
                    Ok(decrypted.to_vec())
                }
                _ => Err(PFError::Device("pinToken not found in response".into())),
            }
        } else {
            Err(PFError::Device("Unexpected response format".into()))
        }
    }

    /// Obtain a PIN token with specific permissions and optional RP ID scope.
    ///
    /// Like [`get_pin_token`](HidTransport::get_pin_token) but uses the
    /// `getPinUvAuthTokenUsingPinWithPermissions` sub-command (0x09). This allows
    /// requesting only the permissions needed (e.g., `CREDENTIAL_MANAGEMENT` for
    /// enumeration/deletion), following the principle of least privilege.
    fn get_pin_token_with_permission(
        &self,
        pin: &str,
        permissions: PinUvAuthTokenPermissions,
        rp_id: Option<String>,
    ) -> Result<Vec<u8>, PFError> {
        log::info!(
            "Starting custom get_pin_token_with_permission (Subcommand 0x09, permissions: {:?})...",
            permissions
        );

        // 1. Get Authenticator Key Agreement
        let auth_key_agreement = self.get_key_agreement()?;

        // 2. Generate Platform Key Pair (P-256)
        let system_rng = ring::rand::SystemRandom::new();
        let platform_private_key =
            agreement::EphemeralPrivateKey::generate(&agreement::ECDH_P256, &system_rng)
                .map_err(|_| PFError::Device("Failed to generate platform ephemeral key".into()))?;
        let platform_public_key_bytes = platform_private_key
            .compute_public_key()
            .map_err(|_| PFError::Device("Failed to compute platform public key".into()))?;

        // 3. Extract Authenticator Public Key (X and Y coordinates)
        let (auth_point_x, auth_point_y) = if let Value::Map(m) = &auth_key_agreement {
            let x = match m.get(&Value::Integer(-2)) {
                Some(Value::Bytes(b)) => b,
                _ => return Err(PFError::Device("Invalid KeyAgreement X coordinate".into())),
            };
            let y = match m.get(&Value::Integer(-3)) {
                Some(Value::Bytes(b)) => b,
                _ => return Err(PFError::Device("Invalid KeyAgreement Y coordinate".into())),
            };
            (x, y)
        } else {
            return Err(PFError::Device("Invalid KeyAgreement format".into()));
        };

        let mut auth_pub_key_bytes = vec![0x04];
        auth_pub_key_bytes.extend(auth_point_x);
        auth_pub_key_bytes.extend(auth_point_y);

        let auth_unparsed_pub_key =
            agreement::UnparsedPublicKey::new(&agreement::ECDH_P256, auth_pub_key_bytes);

        // 4. Perform ECDH to get Shared Secret
        let shared_secret =
            agreement::agree_ephemeral(platform_private_key, &auth_unparsed_pub_key, |material| {
                let mut hasher = digest::Context::new(&digest::SHA256);
                hasher.update(material);
                Ok(hasher.finish()) as Result<digest::Digest, ring::error::Unspecified>
            })
            .map_err(|_| PFError::Device("ECDH shared secret computation failed".into()))?
            .map_err(|_| PFError::Device("Inner ECDH shared secret computation failed".into()))?;

        // 5. Encrypt PIN Hash
        let pin_hash = digest::digest(&digest::SHA256, pin.as_bytes());
        let pin_hash_16 = &pin_hash.as_ref()[0..16];

        let iv = [0u8; 16];
        let mut block = Block::<aes::Aes256>::try_from(pin_hash_16).unwrap();

        let shared_secret_bytes = shared_secret.as_ref();
        let mut encryptor =
            cbc::Encryptor::<aes::Aes256>::new_from_slices(shared_secret_bytes, &iv).unwrap();
        encryptor.encrypt_block(&mut block);
        let pin_hash_enc = block.to_vec();

        // 6. Send getPinUvAuthTokenUsingPinWithPermissions command (Subcommand 0x09)

        // 7. Send getPinUvAuthTokenUsingPinWithPermissions command (Subcommand 0x09)

        let mut payload = vec![CtapCommand::ClientPin as u8];
        let cose_key_bytes = self.encode_cose_key(
            &platform_public_key_bytes.as_ref()[1..33],
            &platform_public_key_bytes.as_ref()[33..65],
        );

        log::trace!(
            "Encrypted PIN hash (first 4 bytes): {:?}",
            &pin_hash_enc[..4]
        );
        let payload_cbor = self.encode_client_pin_params(
            ClientPinSubCommand::GetPinUvAuthTokenUsingPinWithPermissions,
            &cose_key_bytes,
            &pin_hash_enc,
            Some(permissions.bits()),
            rp_id,
        );
        payload.extend(payload_cbor);

        log::debug!("Sending getPinUvAuthTokenUsingPinWithPermissions command...");
        let response = self.send_cbor(CTAPHID_CBOR, &payload)?;
        log::debug!(
            "getPinUvAuthTokenUsingPinWithPermissions response: {:?}",
            response
        );
        let val: Value = from_slice(&response).map_err(|e| PFError::Io(e.to_string()))?;

        if let Value::Map(m) = val {
            log::debug!("getPinUvAuthTokenUsingPinWithPermissions response: {:?}", m);
            match m.get(&Value::Integer(ClientPinResponseParam::PinToken as i128)) {
                Some(Value::Bytes(token_enc)) => {
                    // Decrypt the PIN token using shared secret (AES-256-CBC, IV=0)
                    let mut token_buf = token_enc.clone();
                    let decrypted =
                        cbc::Decryptor::<aes::Aes256>::new_from_slices(shared_secret_bytes, &iv)
                            .map_err(|_| PFError::Device("Failed to create decryptor".into()))?
                            .decrypt_padded::<NoPadding>(&mut token_buf)
                            .map_err(|_| PFError::Device("Failed to decrypt PIN token".into()))?;
                    log::info!("Successfully obtained and decrypted PIN token (Subcommand 0x09).");
                    Ok(decrypted.to_vec())
                }
                _ => Err(PFError::Device(
                    "pinUvAuthToken not found in response".into(),
                )),
            }
        } else {
            Err(PFError::Device("Unexpected response format".into()))
        }
    }

    /// Set a new PIN on the authenticator (sub-command 0x03).
    ///
    /// Implements the full CTAP2 setPin flow:
    /// 1. Performs ECDH key agreement to derive the shared secret.
    /// 2. Encrypts the new PIN (padded to 64 bytes) with AES-256-CBC.
    /// 3. Computes `HMAC-SHA-256(shared_secret, newPinEnc)[0..16]` as pinUvAuthParam.
    /// 4. Sends the SetPin command with the platform's public key, encrypted PIN, and HMAC.
    ///
    /// The PIN must be 4–63 characters. Fails with `PIN_POLICY_VIOLATION` (0x37) if
    /// the PIN is too short.
    fn set_pin(&self, new_pin: &str) -> Result<(), PFError> {
        log::info!("Starting custom set_pin (Subcommand 0x03)...");

        if new_pin.len() < 4 {
            return Err(PFError::Device("PIN must be at least 4 characters".into()));
        }
        if new_pin.len() > 63 {
            return Err(PFError::Device(
                "PIN must be less than 64 characters".into(),
            ));
        }

        // 1. Get Authenticator Key Agreement
        let auth_key_agreement = self.get_key_agreement()?;

        // 2. Generate Platform Key Pair (P-256)
        let system_rng = ring::rand::SystemRandom::new();
        let platform_private_key =
            agreement::EphemeralPrivateKey::generate(&agreement::ECDH_P256, &system_rng)
                .map_err(|_| PFError::Device("Failed to generate platform ephemeral key".into()))?;
        let platform_public_key_bytes = platform_private_key
            .compute_public_key()
            .map_err(|_| PFError::Device("Failed to compute platform public key".into()))?;

        // 3. Extract Authenticator Public Key
        let (auth_point_x, auth_point_y) = if let Value::Map(m) = &auth_key_agreement {
            let x = match m.get(&Value::Integer(-2)) {
                Some(Value::Bytes(b)) => b,
                _ => return Err(PFError::Device("Invalid KeyAgreement X coordinate".into())),
            };
            let y = match m.get(&Value::Integer(-3)) {
                Some(Value::Bytes(b)) => b,
                _ => return Err(PFError::Device("Invalid KeyAgreement Y coordinate".into())),
            };
            (x, y)
        } else {
            return Err(PFError::Device("Invalid KeyAgreement format".into()));
        };

        let mut auth_pub_key_bytes = vec![0x04];
        auth_pub_key_bytes.extend(auth_point_x);
        auth_pub_key_bytes.extend(auth_point_y);

        let auth_unparsed_pub_key =
            agreement::UnparsedPublicKey::new(&agreement::ECDH_P256, auth_pub_key_bytes);

        // 4. Perform ECDH to get Shared Secret
        let shared_secret =
            agreement::agree_ephemeral(platform_private_key, &auth_unparsed_pub_key, |material| {
                let mut hasher = digest::Context::new(&digest::SHA256);
                hasher.update(material);
                Ok(hasher.finish()) as Result<digest::Digest, ring::error::Unspecified>
            })
            .map_err(|_| PFError::Device("ECDH shared secret computation failed".into()))?
            .map_err(|_| PFError::Device("Inner ECDH shared secret computation failed".into()))?;

        let shared_secret_bytes = shared_secret.as_ref();

        // 5. Encrypt newPinEnc
        let mut padded_new_pin = [0u8; 64];
        let bytes = new_pin.as_bytes();
        padded_new_pin[..bytes.len()].copy_from_slice(bytes);

        let iv = [0u8; 16];
        let mut new_pin_enc = Vec::new();
        let mut encryptor =
            cbc::Encryptor::<aes::Aes256>::new_from_slices(shared_secret_bytes, &iv).unwrap();
        for chunk in padded_new_pin.chunks_exact(16) {
            let mut block = Block::<aes::Aes256>::try_from(chunk).unwrap();
            encryptor.encrypt_block(&mut block);
            new_pin_enc.extend_from_slice(&block);
        }

        // 6. Calculate pinUvAuthParam: HMAC-SHA-256(shared_secret, newPinEnc)[0..16]
        let hmac_key = hmac::Key::new(hmac::HMAC_SHA256, shared_secret_bytes);
        let pin_uv_auth_param = hmac::sign(&hmac_key, &new_pin_enc).as_ref()[0..16].to_vec();

        // 7. Send SetPin command
        let cose_key_bytes = self.encode_cose_key(
            &platform_public_key_bytes.as_ref()[1..33],
            &platform_public_key_bytes.as_ref()[33..65],
        );

        let mut payload_cbor = vec![0xA5]; // Map(5)
        payload_cbor
            .extend(to_vec(&Value::Integer(ClientPinParam::PinUvAuthProtocol as i128)).unwrap());
        payload_cbor.extend(to_vec(&Value::Integer(1)).unwrap());
        payload_cbor.extend(to_vec(&Value::Integer(ClientPinParam::SubCommand as i128)).unwrap());
        payload_cbor.extend(to_vec(&Value::Integer(ClientPinSubCommand::SetPin as i128)).unwrap());
        payload_cbor.extend(to_vec(&Value::Integer(ClientPinParam::KeyAgreement as i128)).unwrap());
        payload_cbor.extend(cose_key_bytes);
        payload_cbor
            .extend(to_vec(&Value::Integer(ClientPinParam::PinUvAuthParam as i128)).unwrap());
        payload_cbor.extend(to_vec(&Value::Bytes(pin_uv_auth_param)).unwrap());
        payload_cbor.extend(to_vec(&Value::Integer(ClientPinParam::NewPinEnc as i128)).unwrap());
        payload_cbor.extend(to_vec(&Value::Bytes(new_pin_enc)).unwrap());

        let mut payload = vec![CtapCommand::ClientPin as u8];
        payload.extend(payload_cbor);

        log::debug!("Sending setPin command...");
        match self.send_cbor(CTAPHID_CBOR, &payload) {
            Ok(_) => {
                log::info!("Successfully set new PIN.");
                Ok(())
            }
            Err(e) => {
                let error_string = e.to_string();
                log::error!("Failed to send setPin config: {}", error_string);
                if error_string.contains("0x37") {
                    return Err(PFError::Device(
                        "New PIN violates policy (e.g. too short).".into(),
                    ));
                }
                Err(PFError::Device(format!("setPin failed: {}", e)))
            }
        }
    }

    /// Change the authenticator PIN (sub-command 0x04).
    ///
    /// Implements the full CTAP2 changePin flow:
    /// 1. Performs ECDH key agreement to derive the shared secret.
    /// 2. Encrypts `SHA-256(current_pin)[0..16]` with AES-256-CBC (pinHashEnc).
    /// 3. Encrypts the new PIN (padded to 64 bytes) with AES-256-CBC (newPinEnc).
    /// 4. Computes `HMAC-SHA-256(shared_secret, newPinEnc || pinHashEnc)[0..16]`.
    /// 5. Sends the ChangePin command.
    ///
    /// Returns `CTAP2_ERR_PIN_AUTH_INVALID` (0x31) if the current PIN is wrong,
    /// `CTAP2_ERR_PIN_BLOCKED` (0x32) if the PIN is blocked, or
    /// `CTAP2_ERR_PIN_POLICY_VIOLATION` (0x37) if the new PIN violates policy.
    fn change_pin(&self, current_pin: &str, new_pin: &str) -> Result<(), PFError> {
        log::info!("Starting custom change_pin (Subcommand 0x04)...");

        if new_pin.len() < 4 {
            return Err(PFError::Device("PIN must be at least 4 characters".into()));
        }
        if new_pin.len() > 63 {
            return Err(PFError::Device(
                "PIN must be less than 64 characters".into(),
            ));
        }

        // 1. Get Authenticator Key Agreement
        let auth_key_agreement = self.get_key_agreement()?;

        // 2. Generate Platform Key Pair (P-256)
        let system_rng = ring::rand::SystemRandom::new();
        let platform_private_key =
            agreement::EphemeralPrivateKey::generate(&agreement::ECDH_P256, &system_rng)
                .map_err(|_| PFError::Device("Failed to generate platform ephemeral key".into()))?;
        let platform_public_key_bytes = platform_private_key
            .compute_public_key()
            .map_err(|_| PFError::Device("Failed to compute platform public key".into()))?;

        // 3. Extract Authenticator Public Key
        let (auth_point_x, auth_point_y) = if let Value::Map(m) = &auth_key_agreement {
            let x = match m.get(&Value::Integer(-2)) {
                Some(Value::Bytes(b)) => b,
                _ => return Err(PFError::Device("Invalid KeyAgreement X coordinate".into())),
            };
            let y = match m.get(&Value::Integer(-3)) {
                Some(Value::Bytes(b)) => b,
                _ => return Err(PFError::Device("Invalid KeyAgreement Y coordinate".into())),
            };
            (x, y)
        } else {
            return Err(PFError::Device("Invalid KeyAgreement format".into()));
        };

        let mut auth_pub_key_bytes = vec![0x04];
        auth_pub_key_bytes.extend(auth_point_x);
        auth_pub_key_bytes.extend(auth_point_y);

        let auth_unparsed_pub_key =
            agreement::UnparsedPublicKey::new(&agreement::ECDH_P256, auth_pub_key_bytes);

        // 4. Perform ECDH to get Shared Secret
        let shared_secret =
            agreement::agree_ephemeral(platform_private_key, &auth_unparsed_pub_key, |material| {
                let mut hasher = digest::Context::new(&digest::SHA256);
                hasher.update(material);
                Ok(hasher.finish()) as Result<digest::Digest, ring::error::Unspecified>
            })
            .map_err(|_| PFError::Device("ECDH shared secret computation failed".into()))?
            .map_err(|_| PFError::Device("Inner ECDH shared secret computation failed".into()))?;

        let shared_secret_bytes = shared_secret.as_ref();

        // 5. Encrypt current_pin hash
        let pin_hash = digest::digest(&digest::SHA256, current_pin.as_bytes());
        let pin_hash_16 = &pin_hash.as_ref()[0..16];
        let iv = [0u8; 16];
        let mut block = Block::<aes::Aes256>::try_from(pin_hash_16).unwrap();
        cbc::Encryptor::<aes::Aes256>::new_from_slices(shared_secret_bytes, &iv)
            .unwrap()
            .encrypt_block(&mut block);
        let pin_hash_enc = block.to_vec();

        // 6. Encrypt newPinEnc
        let mut padded_new_pin = [0u8; 64];
        let bytes = new_pin.as_bytes();
        padded_new_pin[..bytes.len()].copy_from_slice(bytes);

        let mut new_pin_enc = Vec::new();
        let mut encryptor =
            cbc::Encryptor::<aes::Aes256>::new_from_slices(shared_secret_bytes, &iv).unwrap();
        for chunk in padded_new_pin.chunks_exact(16) {
            let mut block = Block::<aes::Aes256>::try_from(chunk).unwrap();
            encryptor.encrypt_block(&mut block);
            new_pin_enc.extend_from_slice(&block);
        }

        // 7. Calculate pinUvAuthParam: HMAC-SHA-256(shared_secret, newPinEnc || pinHashEnc)[0..16]
        let mut hmac_msg = Vec::new();
        hmac_msg.extend_from_slice(&new_pin_enc);
        hmac_msg.extend_from_slice(&pin_hash_enc);

        let hmac_key = hmac::Key::new(hmac::HMAC_SHA256, shared_secret_bytes);
        let pin_uv_auth_param = hmac::sign(&hmac_key, &hmac_msg).as_ref()[0..16].to_vec();

        // 8. Send ChangePin command
        let cose_key_bytes = self.encode_cose_key(
            &platform_public_key_bytes.as_ref()[1..33],
            &platform_public_key_bytes.as_ref()[33..65],
        );

        let mut payload_cbor = vec![0xA6]; // Map(6)
        payload_cbor
            .extend(to_vec(&Value::Integer(ClientPinParam::PinUvAuthProtocol as i128)).unwrap());
        payload_cbor.extend(to_vec(&Value::Integer(1)).unwrap());
        payload_cbor.extend(to_vec(&Value::Integer(ClientPinParam::SubCommand as i128)).unwrap());
        payload_cbor
            .extend(to_vec(&Value::Integer(ClientPinSubCommand::ChangePin as i128)).unwrap());
        payload_cbor.extend(to_vec(&Value::Integer(ClientPinParam::KeyAgreement as i128)).unwrap());
        payload_cbor.extend(cose_key_bytes);
        payload_cbor
            .extend(to_vec(&Value::Integer(ClientPinParam::PinUvAuthParam as i128)).unwrap());
        payload_cbor.extend(to_vec(&Value::Bytes(pin_uv_auth_param)).unwrap());
        payload_cbor.extend(to_vec(&Value::Integer(ClientPinParam::NewPinEnc as i128)).unwrap());
        payload_cbor.extend(to_vec(&Value::Bytes(new_pin_enc)).unwrap());
        payload_cbor.extend(to_vec(&Value::Integer(ClientPinParam::PinHashEnc as i128)).unwrap());
        payload_cbor.extend(to_vec(&Value::Bytes(pin_hash_enc)).unwrap());

        let mut payload = vec![CtapCommand::ClientPin as u8];
        payload.extend(payload_cbor);

        log::debug!("Sending changePin command...");
        match self.send_cbor(CTAPHID_CBOR, &payload) {
            Ok(_) => {
                log::info!("Successfully changed PIN.");
                Ok(())
            }
            Err(e) => {
                let error_string = e.to_string();
                log::error!("Failed to send changePin config: {}", error_string);
                if error_string.contains("0x31") {
                    return Err(PFError::Device("Invalid current PIN (0x31). Please check that you entered the correct PIN.".into()));
                }
                if error_string.contains("0x32") {
                    return Err(PFError::Device(
                        "PIN blocked (0x32). Device reset may be required.".into(),
                    ));
                }
                if error_string.contains("0x37") {
                    return Err(PFError::Device(
                        "New PIN violates policy (e.g. too short).".into(),
                    ));
                }
                Err(PFError::Device(format!("changePin failed: {}", e)))
            }
        }
    }

    /// Sign an authenticatorConfig command using HMAC-SHA-256.
    ///
    /// Computes `HMAC-SHA-256(pin_token, 0x0d || subCommand || subCommandParams)[0..16]`
    /// per the CTAP2 authenticatorConfig signing specification. The 0x0d byte
    /// identifies the Config command category.
    fn sign_config_command(
        &self,
        pin_token: &[u8],
        sub_cmd: u8,
        sub_params_bytes: &[u8],
    ) -> Vec<u8> {
        // Build HMAC message for signing
        // According to FIDO 2.1: authenticate(pinUvAuthToken, 32×0xff || 0x0d || uint8(subCommand) || subCommandParams)
        let mut message = vec![0xff; 32];
        message.push(CtapCommand::Config as u8);
        message.push(sub_cmd);
        message.extend(sub_params_bytes);

        // Sign using provided PIN token
        let hmac_key = hmac::Key::new(hmac::HMAC_SHA256, pin_token);
        let sig = hmac::sign(&hmac_key, &message);
        sig.as_ref()[0..16].to_vec()
    }

    /// Encode an uncompressed P-256 public key as a COSE_Key map.
    ///
    /// Returns CBOR bytes for a map with keys: kty(1)=EC2(2), alg(3)=ES256(-7),
    /// crv(-1)=P-256(1), x(-2), y(-3). Used in PIN key agreement payloads.
    fn encode_cose_key(&self, x: &[u8], y: &[u8]) -> Vec<u8> {
        let mut bytes = vec![0xA5]; // Map(5)
        bytes.extend(to_vec(&Value::Integer(1)).unwrap());
        bytes.extend(to_vec(&Value::Integer(2)).unwrap());
        bytes.extend(to_vec(&Value::Integer(3)).unwrap());
        bytes.extend(to_vec(&Value::Integer(-7)).unwrap());
        bytes.extend(to_vec(&Value::Integer(-1)).unwrap());
        bytes.extend(to_vec(&Value::Integer(1)).unwrap());
        bytes.extend(to_vec(&Value::Integer(-2)).unwrap());
        bytes.extend(to_vec(&Value::Bytes(x.to_vec())).unwrap());
        bytes.extend(to_vec(&Value::Integer(-3)).unwrap());
        bytes.extend(to_vec(&Value::Bytes(y.to_vec())).unwrap());
        bytes
    }

    /// Build a CBOR map for ClientPin sub-command parameters.
    ///
    /// Constructs the parameter map with `pinProtocol`, `subCommand`, `keyAgreement`,
    /// and `pinHashEnc`. Optionally includes `permissions` and `rpId` when the
    /// `getPinUvAuthTokenUsingPinWithPermissions` sub-command is used.
    fn encode_client_pin_params(
        &self,
        sub_cmd: ClientPinSubCommand,
        cose_key_bytes: &[u8],
        pin_hash_enc: &[u8],
        permissions: Option<u8>,
        rp_id: Option<String>,
    ) -> Vec<u8> {
        let mut count = 4;
        if permissions.is_some() {
            count += 1;
        }
        if rp_id.is_some() {
            count += 1;
        }
        let mut bytes = vec![0xA0 | (count as u8)];
        bytes.extend(to_vec(&Value::Integer(ClientPinParam::PinUvAuthProtocol as i128)).unwrap());
        bytes.extend(to_vec(&Value::Integer(1)).unwrap());
        bytes.extend(to_vec(&Value::Integer(ClientPinParam::SubCommand as i128)).unwrap());
        bytes.extend(to_vec(&Value::Integer(sub_cmd as i128)).unwrap());
        bytes.extend(to_vec(&Value::Integer(ClientPinParam::KeyAgreement as i128)).unwrap());
        bytes.extend(cose_key_bytes);
        bytes.extend(to_vec(&Value::Integer(ClientPinParam::PinHashEnc as i128)).unwrap());
        bytes.extend(to_vec(&Value::Bytes(pin_hash_enc.to_vec())).unwrap());
        if let Some(p) = permissions {
            bytes.extend(to_vec(&Value::Integer(ClientPinParam::Permissions as i128)).unwrap());
            bytes.extend(to_vec(&Value::Integer(p as i128)).unwrap());
        }
        if let Some(rp) = rp_id {
            bytes.extend(to_vec(&Value::Integer(ClientPinParam::PermissionsRpId as i128)).unwrap());
            bytes.extend(to_vec(&Value::Text(rp)).unwrap());
        }
        bytes
    }

    /// Enumerate all Relying Parties stored on the authenticator.
    ///
    /// Performs the CTAP2 credential management enumeration flow:
    /// 1. Obtains a PIN token with `CREDENTIAL_MANAGEMENT` permission.
    /// 2. Sends `EnumerateRpsBegin` (sub-command 0x02) to get the first RP.
    /// 3. Iterates with `EnumerateRpsGetNextRp` (sub-command 0x03) until all RPs are returned.
    ///
    /// Returns an empty vector if no credentials exist on the device.
    fn credential_management_enumerate_rps(
        &self,
        pin: &str,
    ) -> Result<Vec<EnumerateRpResponse>, PFError> {
        log::info!("Starting custom credential_management_enumerate_rps...");

        // 1. Get PIN token with CREDENTIAL_MANAGEMENT permission
        let pin_token = self.get_pin_token_with_permission(
            pin,
            PinUvAuthTokenPermissions::CREDENTIAL_MANAGEMENT,
            None,
        )?;

        let mut all_rps = Vec::new();

        // 2. EnumerateRpsBegin (Subcommand 0x02)
        // let sub_params = BTreeMap::new();
        // let sub_params_bytes = to_vec(&Value::Map(sub_params.clone())).unwrap();

        let pin_auth = self.sign_credential_mgmt_command(
            &pin_token,
            CredentialMgmtSubCommand::EnumerateRpsBegin as u8,
            None, // sub_params_bytes
        );

        let mut mgmt_map = BTreeMap::new();
        mgmt_map.insert(
            Value::Integer(CredentialMgmtParam::SubCommand as i128),
            Value::Integer(CredentialMgmtSubCommand::EnumerateRpsBegin as i128),
        );
        mgmt_map.insert(
            Value::Integer(CredentialMgmtParam::PinUvAuthProtocol as i128),
            Value::Integer(1),
        );
        mgmt_map.insert(
            Value::Integer(CredentialMgmtParam::PinUvAuthParam as i128),
            Value::Bytes(pin_auth),
        );

        let mut payload = vec![CtapCommand::CredentialMgmt as u8];
        payload.extend(to_vec(&Value::Map(mgmt_map)).map_err(|e| PFError::Io(e.to_string()))?);

        let response = match self.send_cbor(CTAPHID_CBOR, &payload) {
            Ok(r) => r,
            Err(e) => {
                if e.to_string().contains("0x2E") {
                    log::info!("No credentials found on device (0x2E)");
                    return Ok(Vec::new());
                }
                return Err(e);
            }
        };

        let val: Value = from_slice(&response).map_err(|e| PFError::Io(e.to_string()))?;
        let mut total_rps = None;

        if let Value::Map(m) = &val {
            let rp = m
                .get(&Value::Integer(CredentialMgmtResponseParam::Rp as i128))
                .cloned()
                .ok_or_else(|| {
                    PFError::Device("RP not found in EnumerateRpsBegin response".into())
                })?;
            let rp_id_hash = match m.get(&Value::Integer(
                CredentialMgmtResponseParam::RpIdHash as i128,
            )) {
                Some(Value::Bytes(b)) => b.clone(),
                _ => {
                    return Err(PFError::Device(
                        "RpIdHash not found in EnumerateRpsBegin response".into(),
                    ));
                }
            };
            if let Some(Value::Integer(t)) = m.get(&Value::Integer(
                CredentialMgmtResponseParam::TotalRps as i128,
            )) {
                total_rps = Some(*t as usize);
            }

            all_rps.push(EnumerateRpResponse {
                rp,
                rp_id_hash,
                total_rps,
            });
        }

        // 3. EnumerateRpsGetNextRp (Subcommand 0x03)
        let num_to_fetch = total_rps.unwrap_or(1);
        while all_rps.len() < num_to_fetch {
            let mut mgmt_map = BTreeMap::new();
            mgmt_map.insert(
                Value::Integer(CredentialMgmtParam::SubCommand as i128),
                Value::Integer(CredentialMgmtSubCommand::EnumerateRpsGetNextRp as i128),
            );

            let mut payload = vec![CtapCommand::CredentialMgmt as u8];
            payload.extend(to_vec(&Value::Map(mgmt_map)).map_err(|e| PFError::Io(e.to_string()))?);

            match self.send_cbor(CTAPHID_CBOR, &payload) {
                Ok(rsp) => {
                    let val: Value = from_slice(&rsp).map_err(|e| PFError::Io(e.to_string()))?;
                    if let Value::Map(m) = val {
                        let rp = m
                            .get(&Value::Integer(CredentialMgmtResponseParam::Rp as i128))
                            .cloned()
                            .ok_or_else(|| {
                                PFError::Device(
                                    "RP not found in EnumerateRpsGetNextRp response".into(),
                                )
                            })?;
                        let rp_id_hash = match m.get(&Value::Integer(
                            CredentialMgmtResponseParam::RpIdHash as i128,
                        )) {
                            Some(Value::Bytes(b)) => b.clone(),
                            _ => {
                                return Err(PFError::Device(
                                    "RpIdHash not found in EnumerateRpsGetNextRp response".into(),
                                ));
                            }
                        };
                        all_rps.push(EnumerateRpResponse {
                            rp,
                            rp_id_hash,
                            total_rps,
                        });
                    }
                }
                Err(e) => {
                    if e.to_string().contains("0x2E") {
                        break;
                    }
                    return Err(e);
                }
            }
        }

        Ok(all_rps)
    }

    /// Enumerate all credentials registered under a specific Relying Party.
    ///
    /// Given an `rp_id_hash` (SHA-256 of the RP's ID), performs:
    /// 1. Obtains a PIN token with `CREDENTIAL_MANAGEMENT` permission.
    /// 2. Sends `EnumerateCredentialsBegin` (sub-command 0x04) with the RP ID hash.
    /// 3. Iterates with `EnumerateCredentialsGetNextCredential` (sub-command 0x05).
    ///
    /// Returns user info, credential ID, and public key for each credential.
    fn credential_management_enumerate_credentials(
        &self,
        pin: &str,
        rp_id_hash: &[u8],
    ) -> Result<Vec<EnumerateCredentialResponse>, PFError> {
        log::info!("Starting custom credential_management_enumerate_credentials...");

        // 1. Get PIN token with CREDENTIAL_MANAGEMENT permission
        let pin_token = self.get_pin_token_with_permission(
            pin,
            PinUvAuthTokenPermissions::CREDENTIAL_MANAGEMENT,
            None,
        )?;

        let mut all_creds = Vec::new();

        // 2. EnumerateCredentialsBegin (Subcommand 0x04)
        let mut sub_params = BTreeMap::new();
        sub_params.insert(
            Value::Integer(0x01), // rpIdHash
            Value::Bytes(rp_id_hash.to_vec()),
        );
        let sub_params_bytes = to_vec(&Value::Map(sub_params.clone())).unwrap();

        let pin_auth = self.sign_credential_mgmt_command(
            &pin_token,
            CredentialMgmtSubCommand::EnumerateCredentialsBegin as u8,
            Some(&sub_params_bytes),
        );

        let mut mgmt_map = BTreeMap::new();
        mgmt_map.insert(
            Value::Integer(CredentialMgmtParam::SubCommand as i128),
            Value::Integer(CredentialMgmtSubCommand::EnumerateCredentialsBegin as i128),
        );
        mgmt_map.insert(
            Value::Integer(CredentialMgmtParam::SubCommandParams as i128),
            Value::Map(sub_params),
        );
        mgmt_map.insert(
            Value::Integer(CredentialMgmtParam::PinUvAuthProtocol as i128),
            Value::Integer(1),
        );
        mgmt_map.insert(
            Value::Integer(CredentialMgmtParam::PinUvAuthParam as i128),
            Value::Bytes(pin_auth),
        );

        let mut payload = vec![CtapCommand::CredentialMgmt as u8];
        payload.extend(to_vec(&Value::Map(mgmt_map)).map_err(|e| PFError::Io(e.to_string()))?);

        let response = match self.send_cbor(CTAPHID_CBOR, &payload) {
            Ok(r) => r,
            Err(e) => {
                if e.to_string().contains("0x2E") {
                    return Ok(Vec::new());
                }
                return Err(e);
            }
        };

        let val: Value = from_slice(&response).map_err(|e| PFError::Io(e.to_string()))?;
        let mut total_creds = None;

        if let Value::Map(m) = &val {
            let user = m
                .get(&Value::Integer(CredentialMgmtResponseParam::User as i128))
                .cloned()
                .ok_or_else(|| {
                    PFError::Device("User not found in EnumerateCredentialsBegin response".into())
                })?;
            let credential_id = m
                .get(&Value::Integer(
                    CredentialMgmtResponseParam::CredentialId as i128,
                ))
                .cloned()
                .ok_or_else(|| {
                    PFError::Device(
                        "CredentialId not found in EnumerateCredentialsBegin response".into(),
                    )
                })?;
            let public_key = m
                .get(&Value::Integer(
                    CredentialMgmtResponseParam::PublicKey as i128,
                ))
                .cloned()
                .ok_or_else(|| {
                    PFError::Device(
                        "PublicKey not found in EnumerateCredentialsBegin response".into(),
                    )
                })?;
            if let Some(Value::Integer(t)) = m.get(&Value::Integer(
                CredentialMgmtResponseParam::TotalCredentials as i128,
            )) {
                total_creds = Some(*t as usize);
            }

            all_creds.push(EnumerateCredentialResponse {
                user,
                credential_id,
                public_key,
                total_credentials: total_creds,
            });
        }

        // 3. EnumerateCredentialsGetNextCredential (Subcommand 0x05)
        let num_to_fetch = total_creds.unwrap_or(1);
        while all_creds.len() < num_to_fetch {
            let mut mgmt_map = BTreeMap::new();
            mgmt_map.insert(
                Value::Integer(CredentialMgmtParam::SubCommand as i128),
                Value::Integer(
                    CredentialMgmtSubCommand::EnumerateCredentialsGetNextCredential as i128,
                ),
            );

            let mut payload = vec![CtapCommand::CredentialMgmt as u8];
            payload.extend(to_vec(&Value::Map(mgmt_map)).map_err(|e| PFError::Io(e.to_string()))?);

            match self.send_cbor(CTAPHID_CBOR, &payload) {
                Ok(rsp) => {
                    let val: Value = from_slice(&rsp).map_err(|e| PFError::Io(e.to_string()))?;
                    if let Value::Map(m) = val {
                        let user = m
                            .get(&Value::Integer(CredentialMgmtResponseParam::User as i128))
                            .cloned()
                            .ok_or_else(|| {
                                PFError::Device(
                                    "User not found in EnumerateCredentialsGetNextCredential response"
                                        .into(),
                                )
                            })?;
                        let credential_id = m
                            .get(&Value::Integer(CredentialMgmtResponseParam::CredentialId as i128))
                            .cloned()
                            .ok_or_else(|| {
                                PFError::Device(
                                    "CredentialId not found in EnumerateCredentialsGetNextCredential response"
                                        .into(),
                                )
                            })?;
                        let public_key = m
                            .get(&Value::Integer(CredentialMgmtResponseParam::PublicKey as i128))
                            .cloned()
                            .ok_or_else(|| {
                                PFError::Device(
                                    "PublicKey not found in EnumerateCredentialsGetNextCredential response"
                                        .into(),
                                )
                            })?;

                        all_creds.push(EnumerateCredentialResponse {
                            user,
                            credential_id,
                            public_key,
                            total_credentials: total_creds,
                        });
                    }
                }
                Err(e) => {
                    if e.to_string().contains("0x2E") {
                        break;
                    }
                    return Err(e);
                }
            }
        }

        Ok(all_creds)
    }

    /// Delete a specific credential from the authenticator.
    ///
    /// Obtains a PIN token with `CREDENTIAL_MANAGEMENT` permission, then sends
    /// the `DeleteCredential` command (sub-command 0x06) with the credential ID
    /// descriptor map. The `credential_id_map` must be a CBOR map with key 0x02
    /// containing the credential ID.
    fn credential_management_delete_credential(
        &self,
        pin: &str,
        credential_id_map: Value,
    ) -> Result<(), PFError> {
        log::info!("Starting custom credential_management_delete_credential...");

        // 1. Get PIN token with CREDENTIAL_MANAGEMENT permission
        let pin_token = self.get_pin_token_with_permission(
            pin,
            PinUvAuthTokenPermissions::CREDENTIAL_MANAGEMENT,
            None,
        )?;

        // 2. DeleteCredential (Subcommand 0x06)
        let mut sub_params = BTreeMap::new();
        sub_params.insert(
            Value::Integer(0x02), // credentialId descriptor map
            credential_id_map,
        );
        let sub_params_bytes = to_vec(&Value::Map(sub_params.clone())).unwrap();

        let pin_auth = self.sign_credential_mgmt_command(
            &pin_token,
            CredentialMgmtSubCommand::DeleteCredential as u8,
            Some(&sub_params_bytes),
        );

        let mut mgmt_map = BTreeMap::new();
        mgmt_map.insert(
            Value::Integer(CredentialMgmtParam::SubCommand as i128),
            Value::Integer(CredentialMgmtSubCommand::DeleteCredential as i128),
        );
        mgmt_map.insert(
            Value::Integer(CredentialMgmtParam::SubCommandParams as i128),
            Value::Map(sub_params),
        );
        mgmt_map.insert(
            Value::Integer(CredentialMgmtParam::PinUvAuthProtocol as i128),
            Value::Integer(1),
        );
        mgmt_map.insert(
            Value::Integer(CredentialMgmtParam::PinUvAuthParam as i128),
            Value::Bytes(pin_auth),
        );

        let mut payload = vec![CtapCommand::CredentialMgmt as u8];
        payload.extend(to_vec(&Value::Map(mgmt_map)).map_err(|e| PFError::Io(e.to_string()))?);

        self.send_cbor(CTAPHID_CBOR, &payload)?;

        Ok(())
    }

    /// Read a device-config record from an RS-Key via CTAPHID 0x41 CONFIG_READ.
    ///
    /// Sends `{1: 0x0D, 2: {1: target}}` CBOR payload to the RS-Key vendor
    /// command handler inside a CTAPHID_CBOR message. The firmware answers with
    /// a CBOR map `{1: blob}`; this unwraps key 1 and returns the raw record
    /// bytes. Ungated — no PIN needed.
    ///
    /// Targets: `RSKEY_CFG_TARGET_PHY` (0x01) and `RSKEY_CFG_TARGET_LED` (0x02).
    /// `DEV_CONF` (0x00) is write-only over FIDO — the firmware rejects it here
    /// (readable only via the CCID Management applet), so this returns an error.
    fn rs_key_config_read(&self, target: u8) -> Result<Vec<u8>, PFError> {
        let mut params = BTreeMap::new();
        params.insert(Value::Integer(1), Value::Integer(RSKEY_CONFIG_READ as i128));

        let mut target_map = BTreeMap::new();
        target_map.insert(Value::Integer(1), Value::Integer(target as i128));
        params.insert(Value::Integer(2), Value::Map(target_map));

        let inner = to_vec(&Value::Map(params)).map_err(|e| PFError::Io(e.to_string()))?;

        let mut full_payload = vec![RSKEY_CTAPHID_VENDOR_CMD];
        full_payload.extend(inner);
        let resp = self.send_cbor(CTAPHID_CBOR, &full_payload)?;

        // Response is CBOR `{1: blob(bstr)}` — unwrap key 1 to the raw record.
        match from_slice::<Value>(&resp) {
            Ok(Value::Map(m)) => match m.get(&Value::Integer(1)) {
                Some(Value::Bytes(b)) => Ok(b.clone()),
                _ => Err(PFError::Device(
                    "CONFIG_READ response missing blob (key 1)".into(),
                )),
            },
            _ => Err(PFError::Device(
                "CONFIG_READ response is not a CBOR map".into(),
            )),
        }
    }

    /// Write physical configuration to an RS-Key via CTAPHID 0x41 CONFIG_WRITE.
    ///
    /// Sends `{1: 0x0C, 2: {1: target, 2: blob}, 3: protocol, 4: mac}` CBOR
    /// to the RS-Key vendor command handler. Requires a PIN token obtained with
    /// `AUTHENTICATOR_CONFIG` permission.
    ///
    /// The MAC is computed as `HMAC-SHA256(pin_token, 0xFF*32 || 0x41 || 0x0C || cbor_params)[..16]`
    /// per the RS-Key protocol spec.
    fn rs_key_config_write(
        &self,
        pin_token: &[u8],
        target: u8,
        blob: &[u8],
    ) -> Result<(), PFError> {
        let mut params_map = BTreeMap::new();
        params_map.insert(Value::Integer(1), Value::Integer(target as i128));
        params_map.insert(Value::Integer(2), Value::Bytes(blob.to_vec()));
        let params = Value::Map(params_map);
        let params_bytes = to_vec(&params).map_err(|e| PFError::Io(e.to_string()))?;

        // MAC = HMAC-SHA256(pin_token, 0xFF*32 || vendor_cmd || sub_cmd || cbor_params)[..16]
        let mac = {
            let mut input = vec![0xFFu8; 32];
            input.push(RSKEY_CTAPHID_VENDOR_CMD);
            input.push(RSKEY_CONFIG_WRITE);
            input.extend(&params_bytes);
            let hmac_key = hmac::Key::new(hmac::HMAC_SHA256, pin_token);
            hmac::sign(&hmac_key, &input).as_ref()[..16].to_vec()
        };

        let mut outer = BTreeMap::new();
        outer.insert(
            Value::Integer(1),
            Value::Integer(RSKEY_CONFIG_WRITE as i128),
        );
        outer.insert(Value::Integer(2), params);
        outer.insert(Value::Integer(3), Value::Integer(1)); // PIN protocol v1
        outer.insert(Value::Integer(4), Value::Bytes(mac));

        let inner = to_vec(&Value::Map(outer)).map_err(|e| PFError::Io(e.to_string()))?;

        let mut full_payload = vec![RSKEY_CTAPHID_VENDOR_CMD];
        full_payload.extend(inner);
        // CONFIG_WRITE can involve flash erasure/write which takes
        // several seconds on RP2040 — use a generous timeout.
        const CONFIG_WRITE_TIMEOUT_MS: i32 = 30_000;
        self.send_cbor_with_timeout(CTAPHID_CBOR, &full_payload, CONFIG_WRITE_TIMEOUT_MS)
            .map(|_| ())
    }

    /// Sign a credential management command using HMAC-SHA-256.
    ///
    /// Uses pico-fido's non-standard signing scheme: for sub-commands 0x01
    /// (GetCredsMetadata) and 0x02 (EnumerateRpsBegin), only the sub-command
    /// byte is signed. For all others, the sub-command byte followed by the
    /// CBOR-encoded SubCommandParams is signed. Returns the first 16 bytes
    /// of the HMAC digest.
    fn sign_credential_mgmt_command(
        &self,
        pin_token: &[u8],
        sub_cmd: u8,
        sub_params_bytes: Option<&[u8]>,
    ) -> Vec<u8> {
        let mut message = vec![sub_cmd];
        if let Some(params) = sub_params_bytes
            && sub_cmd != CredentialMgmtSubCommand::GetCredsMetadata as u8
            && sub_cmd != CredentialMgmtSubCommand::EnumerateRpsBegin as u8
        {
            message.extend(params);
        }

        log::debug!(
            "Custom CredentialMgmt signing for sub_cmd 0x{:02x}, message len: {}",
            sub_cmd,
            message.len()
        );

        let hmac_key = hmac::Key::new(hmac::HMAC_SHA256, pin_token);
        let sig = hmac::sign(&hmac_key, &message);
        sig.as_ref()[0..16].to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_pin_command_ordering() {
        // This test doesn't run HID IO, but verifies that our BTreeMap usage
        // (which is used in get_pin_token and get_pin_token_with_permission)
        // results in correct CBOR key ordering.
        let mut map = BTreeMap::new();
        map.insert(Value::Integer(0x01), Value::Integer(1)); // pinProtocol
        map.insert(Value::Integer(0x02), Value::Integer(8)); // subCommand (getPinUvAuthToken...)
        map.insert(Value::Integer(0x03), Value::Map(BTreeMap::new())); // keyAgreement
        map.insert(Value::Integer(0x04), Value::Bytes(vec![0u8; 16])); // pinHashEnc
        map.insert(Value::Integer(0x09), Value::Integer(0x01)); // permissions

        let cbor = to_vec(&Value::Map(map)).unwrap();

        assert_eq!(cbor[0], 0xA5);
        assert_eq!(cbor[1], 0x01);
    }

    #[test]
    fn test_get_key_agreement_parsing_logic() {
        use std::collections::BTreeMap;
        // Simulate a response map where key 0x01 is the KeyAgreement (as per CTAP 2.1)
        let mut inner_map = BTreeMap::new();
        inner_map.insert(Value::Integer(1), Value::Integer(2)); // kty: EC2
        inner_map.insert(Value::Integer(-1), Value::Integer(1)); // crv: P-256
        inner_map.insert(Value::Integer(-2), Value::Bytes(vec![0xAA; 32])); // x
        inner_map.insert(Value::Integer(-3), Value::Bytes(vec![0xBB; 32])); // y

        let mut response_map = BTreeMap::new();
        response_map.insert(
            Value::Integer(ClientPinResponseParam::KeyAgreement as i128),
            Value::Map(inner_map),
        );

        let val = Value::Map(response_map);

        // This mimics the logic in get_key_agreement
        if let Value::Map(m) = val {
            let key_agreement = m.get(&Value::Integer(
                ClientPinResponseParam::KeyAgreement as i128,
            ));
            assert!(key_agreement.is_some());
            if let Some(Value::Map(km)) = key_agreement {
                assert_eq!(
                    km.get(&Value::Integer(-2)),
                    Some(&Value::Bytes(vec![0xAA; 32]))
                );
            } else {
                panic!("KeyAgreement should be a map");
            }
        } else {
            panic!("Expected map");
        }
    }

    #[test]
    fn test_pin_hash_encryption_actually_encrypts() {
        // Verify that our AES-CBC encryption actually modifies the data.
        // This guards against the previous bug where encrypt_block
        // was called on a temporary copy (buffer.into()), discarding the result.
        use cbc::cipher::BlockModeEncrypt;
        use ring::digest;

        let pin = "123456";
        let pin_hash = digest::digest(&digest::SHA256, pin.as_bytes());
        let pin_hash_16 = &pin_hash.as_ref()[0..16];

        // Use a known key (32 bytes of zeros) and IV (16 bytes of zeros)
        let key = [0u8; 32];
        let iv = [0u8; 16];

        let mut block = Block::<aes::Aes256>::try_from(pin_hash_16).unwrap();
        let original = block;

        let mut encryptor = cbc::Encryptor::<aes::Aes256>::new_from_slices(&key, &iv).unwrap();
        encryptor.encrypt_block(&mut block);

        // The encrypted block MUST differ from the original
        assert_ne!(
            block.as_slice(),
            original.as_slice(),
            "Encryption did not modify the block — the old bug is back!"
        );
    }
}
