use snafu::ResultExt as _;
use time::PrimitiveDateTime;
use tracing::instrument;

use crate::types::{
	FromBytes, ParseError, ParseTimeTag,
	information_elements::{Bsi, Dpi, Nva, R32, SelectExecute, Spi, Sva},
	quality_descriptors::Qos,
	time::time_from_cp56time2a,
};

/// Command qualifier
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub enum Qu {
	#[default]
	/// Unspecified
	Unspecified,
	/// Short pulse
	ShortPulse,
	/// Long pulse
	LongPulse,
	/// Persistent
	Persistent,
	/// Other (custom)
	Other(u8),
}

impl Qu {
	pub const fn from_byte(byte: u8) -> Self {
		match byte {
			0 => Qu::Unspecified,
			1 => Qu::ShortPulse,
			2 => Qu::LongPulse,
			3 => Qu::Persistent,
			_ => Qu::Other(byte),
		}
	}
}

/// Single command
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct Sco {
	/// Select/execute
	pub se: SelectExecute,
	/// Qualifier of command
	pub qu: Qu,
	/// Single command state information
	pub scs: Spi,
}

impl Sco {
	pub const fn from_byte(byte: u8) -> Self {
		let se = SelectExecute::from_bool(byte & 0b1000_0000 != 0);
		let qu = Qu::from_byte(byte & 0b0111_1100 >> 2);
		let scs = Spi::from_byte(byte & 0b0000_0001);
		Sco { se, qu, scs }
	}
}

/// Double command
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct Dco {
	/// Select/execute
	pub se: SelectExecute,
	/// Qualifier of command
	pub qu: Qu,
	/// Double command state information
	pub dcs: Dpi,
}

impl Dco {
	pub const fn from_byte(byte: u8) -> Self {
		let se = SelectExecute::from_bool(byte & 0b1000_0000 != 0);
		let qu = Qu::from_byte(byte & 0b0111_1100 >> 2);
		let dcs = Dpi::from_byte(byte & 0b0000_0011);
		Dco { se, qu, dcs }
	}
}

/// Status of regulating step
#[derive(Debug, Clone, Eq, PartialEq, Default)]
#[repr(u8)]
pub enum Rcs {
	#[default]
	/// Not allowed
	None = 0,
	/// Decrement
	Decrement = 1,
	/// Increment
	Increment = 2,
	/// Not allowed
	Invalid = 3,
}

impl Rcs {
	pub const fn from_byte(byte: u8) -> Self {
		match byte {
			0 => Rcs::None,
			1 => Rcs::Decrement,
			2 => Rcs::Increment,
			_ => Rcs::Invalid,
		}
	}
}

/// Regulating step command
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct Rco {
	/// Select/execute
	pub se: SelectExecute,
	/// Qualifier of command
	pub qu: Qu,
	/// Status of regulating step
	pub rcs: Rcs,
}

impl Rco {
	pub const fn from_byte(byte: u8) -> Self {
		let se = SelectExecute::from_bool(byte & 0b1000_0000 != 0);
		let qu = Qu::from_byte(byte & 0b0111_1100 >> 2);
		let rcs = Rcs::from_byte(byte & 0b0000_0011);
		Rco { se, qu, rcs }
	}
}

/// Qualifier of interrogation
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub enum Qoi {
	#[default]
	/// Unused
	Unused,
	/// Global interrogation
	Global,
	/// Group 1 interrogation
	Group1,
	/// Group 2 interrogation
	Group2,
	/// Group 3 interrogation
	Group3,
	/// Group 4 interrogation
	Group4,
	/// Group 5 interrogation
	Group5,
	/// Group 6 interrogation
	Group6,
	/// Group 7 interrogation
	Group7,
	/// Group 8 interrogation
	Group8,
	/// Group 9 interrogation
	Group9,
	/// Group 10 interrogation
	Group10,
	/// Group 11 interrogation
	Group11,
	/// Group 12 interrogation
	Group12,
	/// Group 13 interrogation
	Group13,
	/// Group 14 interrogation
	Group14,
	/// Group 15 interrogation
	Group15,
	/// Group 16 interrogation
	Group16,
	/// Other (custom)
	Other(u8),
}

impl Qoi {
	pub const fn from_byte(byte: u8) -> Self {
		match byte {
			0 => Qoi::Unused,
			20 => Qoi::Global,
			21 => Qoi::Group1,
			22 => Qoi::Group2,
			23 => Qoi::Group3,
			24 => Qoi::Group4,
			25 => Qoi::Group5,
			26 => Qoi::Group6,
			27 => Qoi::Group7,
			28 => Qoi::Group8,
			29 => Qoi::Group9,
			30 => Qoi::Group10,
			31 => Qoi::Group11,
			32 => Qoi::Group12,
			33 => Qoi::Group13,
			34 => Qoi::Group14,
			35 => Qoi::Group15,
			36 => Qoi::Group16,
			_ => Qoi::Other(byte),
		}
	}
}

/// Freeze/reset qualifier of counter interrogation commands
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub enum Frz {
	#[default]
	/// Read
	Read,
	/// Freeze
	Freeze,
	/// Freeze and reset
	FreezeAndReset,
	/// Reset
	Reset,
}

impl Frz {
	pub const fn from_byte(byte: u8) -> Self {
		match byte {
			0 => Frz::Read,
			1 => Frz::Freeze,
			2 => Frz::FreezeAndReset,
			_ => Frz::Reset,
		}
	}
}

/// Request qualifier of counter interrogation commands
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub enum Rqt {
	#[default]
	/// No counter read
	None,
	/// Group 1 counter interrogation
	ReqCo1,
	/// Group 2 counter interrogation
	ReqCo2,
	/// Group 3 counter interrogation
	ReqCo3,
	/// Group 4 counter interrogation
	ReqCo4,
	/// General counter interrogation
	ReqCoGen,
	/// Other (custom)
	Other(u8),
}

impl Rqt {
	pub const fn from_byte(byte: u8) -> Self {
		match byte {
			0 => Rqt::None,
			1 => Rqt::ReqCo1,
			2 => Rqt::ReqCo2,
			3 => Rqt::ReqCo3,
			4 => Rqt::ReqCo4,
			5 => Rqt::ReqCoGen,
			_ => Rqt::Other(byte),
		}
	}
}

/// Qualifier of reset process
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub enum Qrp {
	#[default]
	/// Unused
	Unused,
	/// General process reset
	General,
	/// Reset pending events with time tag
	TtEvents,
	/// Other (custom)
	Other(u8),
}

impl Qrp {
	pub const fn from_byte(byte: u8) -> Self {
		match byte {
			0 => Qrp::Unused,
			1 => Qrp::General,
			2 => Qrp::TtEvents,
			_ => Qrp::Other(byte),
		}
	}
}

/// Single command
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct CScNa1 {
	/// Single command
	pub sco: Sco,
}

impl FromBytes for CScNa1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, Box<ParseError>> {
		let sco = Sco::from_byte(bytes[0]);
		Ok(Self { sco })
	}
}

/// Double command
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct CdcNa1 {
	/// Double command
	pub dco: Dco,
}

impl FromBytes for CdcNa1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, Box<ParseError>> {
		let dco = Dco::from_byte(bytes[0]);
		Ok(Self { dco })
	}
}

/// Regulating step command
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct CrcNa1 {
	/// Regulating step command
	pub rco: Rco,
}

impl FromBytes for CrcNa1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, Box<ParseError>> {
		let rco = Rco::from_byte(bytes[0]);
		Ok(Self { rco })
	}
}

/// Set-point command, normalized value
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct CSeNa1 {
	/// Normalized value
	pub nva: Nva,
	/// Qualifier of set point command
	pub qos: Qos,
}

impl FromBytes for CSeNa1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, Box<ParseError>> {
		let nva = Nva::from_bytes(&bytes[0..2]);
		let qos = Qos::from_byte(bytes[2]);
		Ok(Self { nva, qos })
	}
}

/// Set-point command, scaled value
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct CSeNb1 {
	/// Scaled value
	pub sva: Sva,
	/// Qualifier of set point command
	pub qos: Qos,
}

impl FromBytes for CSeNb1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, Box<ParseError>> {
		let sva = Sva::from_bytes(&bytes[0..2]);
		let qos = Qos::from_byte(bytes[2]);
		Ok(Self { sva, qos })
	}
}

/// Set-point command, short floating point number
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CSeNc1 {
	/// Short floating point
	pub r32: R32,
	/// Qualifier of set point command
	pub qos: Qos,
}

impl FromBytes for CSeNc1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, Box<ParseError>> {
		let r32 = R32::from_bytes(&bytes[0..4]);
		let qos = Qos::from_byte(bytes[4]);
		Ok(Self { r32, qos })
	}
}

/// Bitstring 32 bit command
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct CBoNa1 {
	/// Bit string of 32 bits
	pub bsi: Bsi,
}

impl FromBytes for CBoNa1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, Box<ParseError>> {
		let bsi = Bsi::from_byte(&bytes[0..4]);
		Ok(Self { bsi })
	}
}

/// Single command with time tag
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CScTa1 {
	/// Single command
	pub sco: Sco,
	/// Time tag
	pub time: PrimitiveDateTime,
}

impl FromBytes for CScTa1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, Box<ParseError>> {
		let sco = Sco::from_byte(bytes[0]);
		let time = time_from_cp56time2a(&bytes[1..8]).context(ParseTimeTag)?;
		Ok(Self { sco, time })
	}
}

/// Double command with time tag
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CdcTa1 {
	/// Double command
	pub dco: Dco,
	/// Time tag
	pub time: PrimitiveDateTime,
}

impl FromBytes for CdcTa1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, Box<ParseError>> {
		let dco = Dco::from_byte(bytes[0]);
		let time = time_from_cp56time2a(&bytes[1..8]).context(ParseTimeTag)?;
		Ok(Self { dco, time })
	}
}

/// Regulating step command with time tag
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CrcTa1 {
	/// Regulating step command
	pub rco: Rco,
	/// Time tag
	pub time: PrimitiveDateTime,
}

impl FromBytes for CrcTa1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, Box<ParseError>> {
		let rco = Rco::from_byte(bytes[0]);
		let time = time_from_cp56time2a(&bytes[1..8]).context(ParseTimeTag)?;
		Ok(Self { rco, time })
	}
}

/// Measured value, normalized value command with time tag
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CSeTa1 {
	/// Normalized value
	pub nva: Nva,
	/// Qualifier of set point command
	pub qos: Qos,
	/// Time tag
	pub time: PrimitiveDateTime,
}

impl FromBytes for CSeTa1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, Box<ParseError>> {
		let nva = Nva::from_bytes(&bytes[0..2]);
		let qos = Qos::from_byte(bytes[2]);
		let time = time_from_cp56time2a(&bytes[3..10]).context(ParseTimeTag)?;
		Ok(Self { nva, qos, time })
	}
}

/// Measured value, scaled value command with time tag
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CSeTb1 {
	/// Scaled value
	pub sva: Sva,
	/// Qualifier of set point command
	pub qos: Qos,
	/// Time tag
	pub time: PrimitiveDateTime,
}

impl FromBytes for CSeTb1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, Box<ParseError>> {
		let sva = Sva::from_bytes(&bytes[0..2]);
		let qos = Qos::from_byte(bytes[2]);
		let time = time_from_cp56time2a(&bytes[3..10]).context(ParseTimeTag)?;
		Ok(Self { sva, qos, time })
	}
}

/// Measured value, short floating point number command with time tag
#[derive(Debug, Clone, PartialEq)]
pub struct CSeTc1 {
	/// Short floating point
	pub r32: R32,
	/// Qualifier of set point command
	pub qos: Qos,
	/// Time tag
	pub time: PrimitiveDateTime,
}

impl FromBytes for CSeTc1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, Box<ParseError>> {
		let r32 = R32::from_bytes(&bytes[0..4]);
		let qos = Qos::from_byte(bytes[4]);
		let time = time_from_cp56time2a(&bytes[5..12]).context(ParseTimeTag)?;
		Ok(Self { r32, qos, time })
	}
}

/// Bitstring of 32 bit command with time tag
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CBoTa1 {
	/// Bit string of 32 bits
	pub bsi: Bsi,
	/// Time tag
	pub time: PrimitiveDateTime,
}

impl FromBytes for CBoTa1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, Box<ParseError>> {
		let bsi = Bsi::from_byte(&bytes[0..4]);
		let time = time_from_cp56time2a(&bytes[4..11]).context(ParseTimeTag)?;
		Ok(Self { bsi, time })
	}
}

/// Interrogation command
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct CIcNa1 {
	/// Qualifier of interrogation
	pub qoi: Qoi,
}

impl FromBytes for CIcNa1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, Box<ParseError>> {
		let qoi = Qoi::from_byte(bytes[0]);
		Ok(Self { qoi })
	}
}

/// Counter interrogation command
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct CCiNa1 {
	/// Request qualifier of counter interrogation commands
	pub rqt: Rqt,
	/// Freeze/reset qualifier of counter interrogation commands
	pub frz: Frz,
}

impl FromBytes for CCiNa1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, Box<ParseError>> {
		let rqt = Rqt::from_byte(bytes[0] & 0b0011_1111);
		let frz = Frz::from_byte(bytes[0] >> 6);
		Ok(Self { rqt, frz })
	}
}

/// Read command
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct CRdNa1 {
	// No data fields for read command
}

impl FromBytes for CRdNa1 {
	#[instrument]
	fn from_bytes(_: &[u8]) -> Result<Self, Box<ParseError>> {
		Ok(Self {})
	}
}

/// Clock synchronization command
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CCsNa1 {
	/// Time tag
	pub time: PrimitiveDateTime,
}

impl FromBytes for CCsNa1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, Box<ParseError>> {
		let time = time_from_cp56time2a(&bytes[0..7]).context(ParseTimeTag)?;
		Ok(Self { time })
	}
}

/// Test command
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct CTsNa1 {
	/// Test value. Pattern 0xAA55
	pub tsc: u16,
}

impl FromBytes for CTsNa1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, Box<ParseError>> {
		let tsc = u16::from_be_bytes([bytes[1], bytes[0]]);
		Ok(Self { tsc })
	}
}

/// Reset process command
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct CRpNa1 {
	/// Qualifier of reset process
	pub qrp: Qrp,
}

impl FromBytes for CRpNa1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, Box<ParseError>> {
		let qrp = Qrp::from_byte(bytes[0]);
		Ok(Self { qrp })
	}
}

/// Delay acquisition command
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct CCdNa1 {
	//TODO: Fix it, delay is not a u16
	/// Delay
	pub delay: u16,
}

impl FromBytes for CCdNa1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, Box<ParseError>> {
		let delay = u16::from_be_bytes([bytes[1], bytes[0]]);
		Ok(Self { delay })
	}
}

/// Test command with time tag
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CTsTa1 {
	/// Test value
	pub tsc: u16,
	/// Time tag
	pub time: PrimitiveDateTime,
}

impl FromBytes for CTsTa1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, Box<ParseError>> {
		let tsc = u16::from_be_bytes([bytes[1], bytes[0]]);
		let time = time_from_cp56time2a(&bytes[2..9]).context(ParseTimeTag)?;
		Ok(Self { tsc, time })
	}
}
