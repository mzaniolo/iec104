use tracing::instrument;

use crate::types::{
	FromBytes, ParseError, ToBytes,
	information_elements::{Lpc, Nva, R32, Sva},
};

/// Kind of parameter of measured value
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub enum Kpa {
	#[default]
	/// Unused
	Unused,
	/// Threshold
	Thresh,
	/// Filter
	Filter,
	/// Low limit
	LoLimit,
	/// High limit
	HiLimit,
	/// Other (custom)
	Other(u8),
}

impl Kpa {
	#[must_use]
	pub const fn from_byte(byte: u8) -> Self {
		match byte {
			0 => Kpa::Unused,
			1 => Kpa::Thresh,
			2 => Kpa::Filter,
			3 => Kpa::LoLimit,
			4 => Kpa::HiLimit,
			_ => Kpa::Other(byte),
		}
	}
	#[must_use]
	pub const fn to_byte(&self) -> u8 {
		match self {
			Kpa::Unused => 0,
			Kpa::Thresh => 1,
			Kpa::Filter => 2,
			Kpa::LoLimit => 3,
			Kpa::HiLimit => 4,
			Kpa::Other(byte) => *byte,
		}
	}
}

/// Qualifier of parameter of measured value
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct Qpm {
	/// Kind of parameter of measured value
	pub kpa: Kpa,
	/// POP
	pub pop: bool,
	/// Local parameter change
	pub lpc: Lpc,
}

impl Qpm {
	#[must_use]
	pub const fn from_byte(byte: u8) -> Self {
		let kpa = Kpa::from_byte(byte >> 5);
		let pop = byte & 0b0100_0000 != 0;
		let lpc = Lpc::from_bool(byte & 0b1000_0000 != 0);
		Self { kpa, pop, lpc }
	}
	#[must_use]
	pub const fn to_byte(&self) -> u8 {
		let mut byte: u8 = 0;
		byte |= self.kpa.to_byte() << 5;
		byte |= (self.pop as u8) << 6;
		byte |= (self.lpc as u8) << 7;
		byte
	}
}

/// Qualifier of parameter activation
#[derive(Debug, Clone, Eq, PartialEq, Default, Copy)]
#[repr(u8)]
pub enum Qpa {
	#[default]
	/// Unused
	Unused,
	/// General
	General,
	/// Object
	Object,
	/// Transmission
	Transmission,
	/// Other (custom)
	Other(u8),
}

impl Qpa {
	#[must_use]
	pub const fn from_byte(byte: u8) -> Self {
		match byte {
			0 => Qpa::Unused,
			1 => Qpa::General,
			2 => Qpa::Object,
			3 => Qpa::Transmission,
			_ => Qpa::Other(byte),
		}
	}
	#[must_use]
	pub const fn to_byte(self) -> u8 {
		match self {
			Qpa::Unused => 0,
			Qpa::General => 1,
			Qpa::Object => 2,
			Qpa::Transmission => 3,
			Qpa::Other(byte) => byte,
		}
	}
}

/// Parameter of measured value, normalized value
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct PMeNa1 {
	/// Normalized value
	pub nva: Nva,
	/// Qualifier of parameter of measured value
	pub qpm: Qpm,
}

impl FromBytes for PMeNa1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, Box<ParseError>> {
		let nva = Nva::from_bytes(&bytes[0..2]);
		let qpm = Qpm::from_byte(bytes[2]);
		Ok(Self { nva, qpm })
	}
}

impl ToBytes for PMeNa1 {
	#[instrument]
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), Box<ParseError>> {
		buffer.extend_from_slice(&self.nva.to_bytes());
		buffer.push(self.qpm.to_byte());
		Ok(())
	}
}

/// Parameter of scaled value, measured value
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct PMeNb1 {
	/// Scaled value
	pub sva: Sva,
	/// Qualifier of parameter of measured value
	pub qpm: Qpm,
}

impl FromBytes for PMeNb1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, Box<ParseError>> {
		let sva = Sva::from_bytes(&bytes[0..2]);
		let qpm = Qpm::from_byte(bytes[2]);
		Ok(Self { sva, qpm })
	}
}

impl ToBytes for PMeNb1 {
	#[instrument]
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), Box<ParseError>> {
		buffer.extend_from_slice(&self.sva.to_bytes());
		buffer.push(self.qpm.to_byte());
		Ok(())
	}
}

/// Parameter of short floating point value, measured value
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct PMeNc1 {
	/// Short floating point value
	pub r32: R32,
	/// Qualifier of parameter of measured value
	pub qpm: Qpm,
}

impl FromBytes for PMeNc1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, Box<ParseError>> {
		let r32 = R32::from_bytes(&bytes[0..4]);
		let qpm = Qpm::from_byte(bytes[4]);
		Ok(Self { r32, qpm })
	}
}

impl ToBytes for PMeNc1 {
	#[instrument]
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), Box<ParseError>> {
		buffer.extend_from_slice(&self.r32.to_bytes());
		buffer.push(self.qpm.to_byte());
		Ok(())
	}
}

/// Parameter activation
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct PAcNa1 {
	/// Qualifier of parameter activation
	pub qpa: Qpa,
}

impl FromBytes for PAcNa1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, Box<ParseError>> {
		let qpa = Qpa::from_byte(bytes[0]);
		Ok(Self { qpa })
	}
}

impl ToBytes for PAcNa1 {
	#[instrument]
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), Box<ParseError>> {
		buffer.push(self.qpa.to_byte());
		Ok(())
	}
}
