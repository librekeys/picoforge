//! Shared COSE algorithm/curve/key-parameter definitions, firmware-version
//! parsing, and the RS-Key LED status-block codec.

pub mod cose;
pub mod led;
pub mod version;

pub use led::parse_led_block;
pub use version::FirmwareVersion;
