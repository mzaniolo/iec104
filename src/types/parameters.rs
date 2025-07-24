use crate::types::{
	FromBytes, ParseError,
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
	pub const fn from_byte(byte: u8) -> Self {
		let kpa = Kpa::from_byte(byte >> 5);
		let pop = byte & 0b0100_0000 != 0;
		let lpc = Lpc::from_bool(byte & 0b1000_0000 != 0);
		Self { kpa, pop, lpc }
	}
}

/// Qualifier of parameter activation
#[derive(Debug, Clone, Eq, PartialEq, Default)]
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
	pub const fn from_byte(byte: u8) -> Self {
		match byte {
			0 => Qpa::Unused,
			1 => Qpa::General,
			2 => Qpa::Object,
			3 => Qpa::Transmission,
			_ => Qpa::Other(byte),
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
	fn from_bytes(bytes: &[u8]) -> Result<Self, Box<ParseError>> {
		let nva = Nva::from_bytes(&bytes[0..2]);
		let qpm = Qpm::from_byte(bytes[2]);
		Ok(Self { nva, qpm })
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
	fn from_bytes(bytes: &[u8]) -> Result<Self, Box<ParseError>> {
		let sva = Sva::from_bytes(&bytes[0..2]);
		let qpm = Qpm::from_byte(bytes[2]);
		Ok(Self { sva, qpm })
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
	fn from_bytes(bytes: &[u8]) -> Result<Self, Box<ParseError>> {
		let r32 = R32::from_bytes(&bytes[0..4]);
		let qpm = Qpm::from_byte(bytes[4]);
		Ok(Self { r32, qpm })
	}
}

/// Parameter activation
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct PAcNa1 {
	/// Qualifier of parameter activation
	pub qpa: Qpa,
}

impl FromBytes for PAcNa1 {
	fn from_bytes(bytes: &[u8]) -> Result<Self, Box<ParseError>> {
		let qpa = Qpa::from_byte(bytes[0]);
		Ok(Self { qpa })
	}
}
