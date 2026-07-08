//! Application-wide error types.
//!
//! `PFError` is a single enum covering the four failure modes
//! encountered during device discovery, communication, and I/O.
//! Each variant carries enough context to render a user-facing message
//! and to serialize through the UI layer.

/// Custom error types for PicoForge operations.
#[derive(Debug, thiserror::Error)]
pub enum PFError {
    /// No compatible FIDO device could be detected on any transport.
    #[error("No device found")]
    NoDevice,
    /// Wrapped error from the PC/SC smart card subsystem.
    #[error("PCSC Error: {0}")]
    Pcsc(#[from] pcsc::Error),
    /// An I/O or encoding/decoding failure (hex, CBOR, transport framing).
    #[error("IO/Hex Error: {0}")]
    Io(String),
    /// A device-level error returned by the firmware or transport layer.
    #[error("Device Error: {0}")]
    Device(String),
}

impl serde::Serialize for PFError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("PFError", 2)?;
        match self {
            PFError::NoDevice => {
                state.serialize_field("type", "NoDevice")?;
                state.serialize_field("message", "No device found")?;
            }
            PFError::Pcsc(err) => {
                state.serialize_field("type", "Pcsc")?;
                state.serialize_field("message", &err.to_string())?;
            }
            PFError::Io(msg) => {
                state.serialize_field("type", "Io")?;
                state.serialize_field("message", msg)?;
            }
            PFError::Device(msg) => {
                state.serialize_field("type", "Device")?;
                state.serialize_field("message", msg)?;
            }
        }
        state.end()
    }
}

// pub type Result<T> = std::result::Result<T, PFError>;
