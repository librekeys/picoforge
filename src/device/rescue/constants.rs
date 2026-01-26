//! Constants, enums, bitflags and data structures for Rescue Application for pico-fido firmware.
#![allow(unused)]

// use serde::{Deserialize, Serialize};
// use std::fmt;

// --- 1. ISO 7816-4 Standard Constants ---

/// Class Byte (CLA)
pub const APDU_CLA_ISO: u8 = 0x00; // Standard ISO commands
pub const APDU_CLA_PROPRIETARY: u8 = 0x80; // Custom/Rescue commands

/// Instruction (INS) for Selection
pub const APDU_INS_SELECT: u8 = 0xA4;

/// Selection Parameters (P1, P2)
pub const APDU_P1_SELECT_BY_DF_NAME: u8 = 0x04;
pub const APDU_P2_RETURN_FCI: u8 = 0x04; // Return File Control Info

/// Status Words (SW1 SW2)
pub const SW_SUCCESS: [u8; 2] = [0x90, 0x00];

// --- 2. Rescue Applet Constants ---

// The Rescue Application ID (AID) from src/rescue.c
pub const RESCUE_AID: &[u8] = &[0xA0, 0x58, 0x3F, 0xC1, 0x9B, 0x7E, 0x4F, 0x21];

// APDU Instructions
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RescueInstruction {
	KeyDevSign = 0x10,
	Write = 0x1C,
	Secure = 0x1D,
	Read = 0x1E,
	Reboot = 0x1F,
}

/// P1 Parameters for RescueInstruction::Read (0x1E)
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadParam {
	PhyConfig = 0x01,
	FlashInfo = 0x02,
	SecureBootStatus = 0x03,
}

/// P1 Parameters for WRITE (0x1C)
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteParam {
	PhyConfig = 0x01,
}

/// P1 Parameters for RescueInstruction::KeyDevSign (0x10)
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignParam {
	SignData = 0x01,
	GetPublicKey = 0x02,
	UploadCert = 0x03,
}

/// P1 Parameters for RescueInstruction::Reboot (0x1F)
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RebootParam {
	Normal = 0x00,
	Bootsel = 0x01,
}

/// P2 Parameters for SECURE (0x1D)
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SecureLockParam {
	#[default]
	Unlock = 0x00,
	Lock = 0x01,
}

/// Default P2 value when not used
pub const P2_UNUSED: u8 = 0x00;

// --- 3. PHY Configuration Tags & Flags ---

// PHY Tags from src/fs/phy.h
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhyTag {
	VidPid = 0x00,
	LedGpio = 0x04,
	LedBrightness = 0x05,
	Opts = 0x06,
	PresenceTimeout = 0x08, // Previously TAG_UP_BTN
	UsbProduct = 0x09,
	Curves = 0x0A,
	LedDriver = 0x0C,
}

impl PhyTag {
	/// Helper to convert raw u8 from device back to Enum
	pub fn from_u8(val: u8) -> Option<Self> {
		match val {
			0x00 => Some(Self::VidPid),
			0x04 => Some(Self::LedGpio),
			0x05 => Some(Self::LedBrightness),
			0x06 => Some(Self::Opts),
			0x08 => Some(Self::PresenceTimeout),
			0x09 => Some(Self::UsbProduct),
			0x0A => Some(Self::Curves),
			0x0C => Some(Self::LedDriver),
			_ => None,
		}
	}
}

bitflags::bitflags! {
	/// Configuration options for TAG_OPTS (Tag 0x06)
	pub struct RescueOptions: u16 {
		const LED_DIMMABLE = 0x02;
		const DISABLE_POWER_RESET = 0x04;
		const LED_STEADY = 0x08;
	}
}

bitflags::bitflags! {
	/// Enabled curves for TAG_CURVES (Tag 0x0A)
	pub struct RescueCurves: u32 {
		const SECP256K1 = 0x08;
	}
}
