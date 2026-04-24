use snafu::OptionExt as _;
use tracing::instrument;

use crate::types::{FromBytes, NotEnoughBytes, ParseError, ToBytes, information_elements::Lpc};

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
		// Per IEC 60870-5-101 §7.2.6.24:
		//   bits 0-5 = KPA (6 bits)
		//   bit 6    = LPC
		//   bit 7    = POP
		let kpa = Kpa::from_byte(byte & 0b0011_1111);
		let lpc = Lpc::from_bool(byte & 0b0100_0000 != 0);
		let pop = byte & 0b1000_0000 != 0;
		Self { kpa, pop, lpc }
	}
	#[must_use]
	pub const fn to_byte(&self) -> u8 {
		let mut byte: u8 = 0;
		byte |= self.kpa.to_byte() & 0b0011_1111;
		byte |= (self.lpc as u8) << 6;
		byte |= (self.pop as u8) << 7;
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
	pub nva: i16,
	/// Qualifier of parameter of measured value
	pub qpm: Qpm,
}

impl FromBytes for PMeNa1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, ParseError> {
		let nva = i16::from_le_bytes(*bytes.first_chunk::<2>().context(NotEnoughBytes)?);
		let qpm = Qpm::from_byte(*bytes.get(2).context(NotEnoughBytes)?);
		Ok(Self { nva, qpm })
	}
}

impl ToBytes for PMeNa1 {
	#[instrument]
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), ParseError> {
		buffer.extend_from_slice(&self.nva.to_le_bytes());
		buffer.push(self.qpm.to_byte());
		Ok(())
	}
}

/// Parameter of scaled value, measured value
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct PMeNb1 {
	/// Scaled value
	pub sva: i16,
	/// Qualifier of parameter of measured value
	pub qpm: Qpm,
}

impl FromBytes for PMeNb1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, ParseError> {
		let sva = i16::from_le_bytes(*bytes.first_chunk::<2>().context(NotEnoughBytes)?);
		let qpm = Qpm::from_byte(*bytes.get(2).context(NotEnoughBytes)?);
		Ok(Self { sva, qpm })
	}
}

impl ToBytes for PMeNb1 {
	#[instrument]
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), ParseError> {
		buffer.extend_from_slice(&self.sva.to_le_bytes());
		buffer.push(self.qpm.to_byte());
		Ok(())
	}
}

/// Parameter of short floating point value, measured value
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PMeNc1 {
	/// Short floating point value
	pub value: f32,
	/// Qualifier of parameter of measured value
	pub qpm: Qpm,
}

impl FromBytes for PMeNc1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, ParseError> {
		let value = f32::from_le_bytes(*bytes.first_chunk::<4>().context(NotEnoughBytes)?);
		let qpm = Qpm::from_byte(*bytes.get(4).context(NotEnoughBytes)?);
		Ok(Self { value, qpm })
	}
}

impl ToBytes for PMeNc1 {
	#[instrument]
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), ParseError> {
		buffer.extend_from_slice(&self.value.to_le_bytes());
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
	fn from_bytes(bytes: &[u8]) -> Result<Self, ParseError> {
		let qpa = Qpa::from_byte(*bytes.first().context(NotEnoughBytes)?);
		Ok(Self { qpa })
	}
}

impl ToBytes for PAcNa1 {
	#[instrument]
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), ParseError> {
		buffer.push(self.qpa.to_byte());
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn qpm_round_trip(byte: u8) -> Qpm {
		let q = Qpm::from_byte(byte);
		assert_eq!(q.to_byte(), byte, "wire byte must match after round-trip");
		q
	}

	#[test]
	fn qpm_kpa_uses_lower_six_bits() {
		// KPA = HiLimit (4) in bits 0-5, no LPC, no POP -> 0x04
		let q = qpm_round_trip(0b0000_0100);
		assert_eq!(q.kpa, Kpa::HiLimit);
		assert!(!q.pop);
		assert_eq!(q.lpc, Lpc::NoChange);
	}

	#[test]
	fn qpm_lpc_is_bit_6() {
		// LPC = Changed (bit 6), KPA = Unused, POP off -> 0b0100_0000 (0x40)
		let q = qpm_round_trip(0b0100_0000);
		assert_eq!(q.lpc, Lpc::Changed);
		assert_eq!(q.kpa, Kpa::Unused);
		assert!(!q.pop);
	}

	#[test]
	fn qpm_pop_is_bit_7() {
		// POP set (bit 7), KPA Unused, LPC NoChange -> 0b1000_0000 (0x80)
		let q = qpm_round_trip(0b1000_0000);
		assert!(q.pop);
		assert_eq!(q.lpc, Lpc::NoChange);
		assert_eq!(q.kpa, Kpa::Unused);
	}

	#[test]
	fn qpm_all_set() {
		// KPA=Other(63), LPC, POP all set -> 0xFF
		let q = qpm_round_trip(0xFF);
		assert_eq!(q.kpa, Kpa::Other(63));
		assert_eq!(q.lpc, Lpc::Changed);
		assert!(q.pop);
	}

	#[test]
	fn qpm_kpa_filter_round_trip() {
		// KPA = Filter (2)
		let q = qpm_round_trip(0b0000_0010);
		assert_eq!(q.kpa, Kpa::Filter);
	}
}
