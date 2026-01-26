/// Custom error types for Pico Forge application.
#[derive(Debug, thiserror::Error)]
pub enum PFError {
	#[error("No device found")]
	NoDevice,
	#[error("PCSC Error: {0}")]
	Pcsc(#[from] pcsc::Error),
	#[error("IO/Hex Error: {0}")]
	Io(String),
	#[error("Device Error: {0}")]
	Device(String),
}

// Allow error to be serialized to string for Tauri
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
