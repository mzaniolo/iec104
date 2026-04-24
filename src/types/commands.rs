use snafu::{OptionExt as _, ResultExt as _};
use tracing::instrument;

use crate::types::{
	FromBytes, NotEnoughBytes, ParseError, ParseTimeTag, SizedSlice, ToBytes,
	information_elements::{Dpi, SelectExecute, Spi},
	quality_descriptors::Qos,
	time::{Cp16Time2a, Cp56Time2a},
};

/// Command qualifier
#[derive(Debug, Clone, Eq, PartialEq, Default, Copy)]
#[repr(u8)]
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
	#[must_use]
	pub const fn from_byte(byte: u8) -> Self {
		match byte {
			0 => Qu::Unspecified,
			1 => Qu::ShortPulse,
			2 => Qu::LongPulse,
			3 => Qu::Persistent,
			_ => Qu::Other(byte),
		}
	}

	#[must_use]
	pub const fn to_byte(self) -> u8 {
		match self {
			Qu::Unspecified => 0,
			Qu::ShortPulse => 1,
			Qu::LongPulse => 2,
			Qu::Persistent => 3,
			Qu::Other(byte) => byte,
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
	#[must_use]
	pub const fn from_byte(byte: u8) -> Self {
		let se = SelectExecute::from_bool(byte & 0b1000_0000 != 0);
		let qu = Qu::from_byte((byte & 0b0111_1100) >> 2);
		let scs = Spi::from_byte(byte & 0b0000_0001);
		Sco { se, qu, scs }
	}

	#[must_use]
	pub const fn to_byte(&self) -> u8 {
		let mut byte: u8 = 0;
		byte |= (self.se as u8) << 7;
		byte |= self.qu.to_byte() << 2;
		byte |= self.scs as u8;
		byte
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
	#[must_use]
	pub const fn from_byte(byte: u8) -> Self {
		let se = SelectExecute::from_bool(byte & 0b1000_0000 != 0);
		let qu = Qu::from_byte((byte & 0b0111_1100) >> 2);
		let dcs = Dpi::from_byte(byte & 0b0000_0011);
		Dco { se, qu, dcs }
	}

	#[must_use]
	pub const fn to_byte(&self) -> u8 {
		let mut byte: u8 = 0;
		byte |= (self.se as u8) << 7;
		byte |= self.qu.to_byte() << 2;
		byte |= self.dcs as u8;
		byte
	}
}

/// Status of regulating step
#[derive(Debug, Clone, Eq, PartialEq, Default, Copy)]
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
	#[must_use]
	pub const fn from_byte(byte: u8) -> Self {
		match byte {
			0 => Rcs::None,
			1 => Rcs::Decrement,
			2 => Rcs::Increment,
			_ => Rcs::Invalid,
		}
	}

	#[must_use]
	pub const fn to_byte(self) -> u8 {
		match self {
			Rcs::None => 0,
			Rcs::Decrement => 1,
			Rcs::Increment => 2,
			Rcs::Invalid => 3,
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
	#[must_use]
	pub const fn from_byte(byte: u8) -> Self {
		let se = SelectExecute::from_bool(byte & 0b1000_0000 != 0);
		let qu = Qu::from_byte((byte & 0b0111_1100) >> 2);
		let rcs = Rcs::from_byte(byte & 0b0000_0011);
		Rco { se, qu, rcs }
	}

	#[must_use]
	pub const fn to_byte(&self) -> u8 {
		let mut byte: u8 = 0;
		byte |= (self.se as u8) << 7;
		byte |= self.qu.to_byte() << 2;
		byte |= self.rcs as u8;
		byte
	}
}

/// Qualifier of interrogation
#[derive(Debug, Clone, Eq, PartialEq, Default, Copy)]
#[repr(u8)]
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
	#[must_use]
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

	#[must_use]
	pub const fn to_byte(self) -> u8 {
		match self {
			Qoi::Unused => 0,
			Qoi::Global => 20,
			Qoi::Group1 => 21,
			Qoi::Group2 => 22,
			Qoi::Group3 => 23,
			Qoi::Group4 => 24,
			Qoi::Group5 => 25,
			Qoi::Group6 => 26,
			Qoi::Group7 => 27,
			Qoi::Group8 => 28,
			Qoi::Group9 => 29,
			Qoi::Group10 => 30,
			Qoi::Group11 => 31,
			Qoi::Group12 => 32,
			Qoi::Group13 => 33,
			Qoi::Group14 => 34,
			Qoi::Group15 => 35,
			Qoi::Group16 => 36,
			Qoi::Other(byte) => byte,
		}
	}
}

/// Freeze/reset qualifier of counter interrogation commands
#[derive(Debug, Clone, Eq, PartialEq, Default, Copy)]
#[repr(u8)]
pub enum Frz {
	#[default]
	/// Read
	Read = 0,
	/// Freeze
	Freeze = 1,
	/// Freeze and reset
	FreezeAndReset = 2,
	/// Reset
	Reset = 3,
}

impl Frz {
	#[must_use]
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
#[derive(Debug, Clone, Eq, PartialEq, Default, Copy)]
#[repr(u8)]
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
	#[must_use]
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

	#[must_use]
	pub const fn to_byte(self) -> u8 {
		match self {
			Rqt::None => 0,
			Rqt::ReqCo1 => 1,
			Rqt::ReqCo2 => 2,
			Rqt::ReqCo3 => 3,
			Rqt::ReqCo4 => 4,
			Rqt::ReqCoGen => 5,
			Rqt::Other(byte) => byte,
		}
	}
}

/// Qualifier of reset process
#[derive(Debug, Clone, Eq, PartialEq, Default, Copy)]
#[repr(u8)]
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
	#[must_use]
	pub const fn from_byte(byte: u8) -> Self {
		match byte {
			0 => Qrp::Unused,
			1 => Qrp::General,
			2 => Qrp::TtEvents,
			_ => Qrp::Other(byte),
		}
	}

	#[must_use]
	pub const fn to_byte(self) -> u8 {
		match self {
			Qrp::Unused => 0,
			Qrp::General => 1,
			Qrp::TtEvents => 2,
			Qrp::Other(byte) => byte,
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
	fn from_bytes(bytes: &[u8]) -> Result<Self, ParseError> {
		let sco = Sco::from_byte(*bytes.first().context(NotEnoughBytes)?);
		Ok(Self { sco })
	}
}

impl ToBytes for CScNa1 {
	#[instrument]
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), ParseError> {
		buffer.push(self.sco.to_byte());
		Ok(())
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
	fn from_bytes(bytes: &[u8]) -> Result<Self, ParseError> {
		let dco = Dco::from_byte(*bytes.first().context(NotEnoughBytes)?);
		Ok(Self { dco })
	}
}

impl ToBytes for CdcNa1 {
	#[instrument]
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), ParseError> {
		buffer.push(self.dco.to_byte());
		Ok(())
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
	fn from_bytes(bytes: &[u8]) -> Result<Self, ParseError> {
		let rco = Rco::from_byte(*bytes.first().context(NotEnoughBytes)?);
		Ok(Self { rco })
	}
}

impl ToBytes for CrcNa1 {
	#[instrument]
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), ParseError> {
		buffer.push(self.rco.to_byte());
		Ok(())
	}
}

/// Set-point command, normalized value
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct CSeNa1 {
	/// Normalized value
	pub nva: i16,
	/// Qualifier of set point command
	pub qos: Qos,
}

impl FromBytes for CSeNa1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, ParseError> {
		let nva = i16::from_le_bytes(*bytes.first_chunk::<2>().context(NotEnoughBytes)?);
		let qos = Qos::from_byte(*bytes.get(2).context(NotEnoughBytes)?);
		Ok(Self { nva, qos })
	}
}

impl ToBytes for CSeNa1 {
	#[instrument]
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), ParseError> {
		buffer.extend_from_slice(&self.nva.to_le_bytes());
		buffer.push(self.qos.to_byte());
		Ok(())
	}
}

/// Set-point command, scaled value
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct CSeNb1 {
	/// Scaled value
	pub sva: i16,
	/// Qualifier of set point command
	pub qos: Qos,
}

impl FromBytes for CSeNb1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, ParseError> {
		let sva = i16::from_le_bytes(*bytes.first_chunk::<2>().context(NotEnoughBytes)?);
		let qos = Qos::from_byte(*bytes.get(2).context(NotEnoughBytes)?);
		Ok(Self { sva, qos })
	}
}

impl ToBytes for CSeNb1 {
	#[instrument]
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), ParseError> {
		buffer.extend_from_slice(&self.sva.to_le_bytes());
		buffer.push(self.qos.to_byte());
		Ok(())
	}
}

/// Set-point command, short floating point number
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CSeNc1 {
	/// Short floating point
	pub value: f32,
	/// Qualifier of set point command
	pub qos: Qos,
}

impl FromBytes for CSeNc1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, ParseError> {
		let value = f32::from_le_bytes(*bytes.first_chunk::<4>().context(NotEnoughBytes)?);
		let qos = Qos::from_byte(*bytes.get(4).context(NotEnoughBytes)?);
		Ok(Self { value, qos })
	}
}

impl ToBytes for CSeNc1 {
	#[instrument]
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), ParseError> {
		buffer.extend_from_slice(&self.value.to_le_bytes());
		buffer.push(self.qos.to_byte());
		Ok(())
	}
}

/// Bitstring 32 bit command
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct CBoNa1 {
	/// Bit string of 32 bits
	pub bsi: u32,
}

impl FromBytes for CBoNa1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, ParseError> {
		let bsi = u32::from_le_bytes(*bytes.first_chunk::<4>().context(NotEnoughBytes)?);
		Ok(Self { bsi })
	}
}

impl ToBytes for CBoNa1 {
	#[instrument]
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), ParseError> {
		buffer.extend_from_slice(&self.bsi.to_le_bytes());
		Ok(())
	}
}

/// Single command with time tag
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct CScTa1 {
	/// Single command
	pub sco: Sco,
	/// Time tag
	pub time: Cp56Time2a,
}

impl FromBytes for CScTa1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, ParseError> {
		let sco = Sco::from_byte(*bytes.first().context(NotEnoughBytes)?);
		let time = Cp56Time2a::from_bytes(
			bytes.get(1..8).context(NotEnoughBytes)?.try_into().context(SizedSlice)?,
		)
		.context(ParseTimeTag)?;
		Ok(Self { sco, time })
	}
}

impl ToBytes for CScTa1 {
	#[instrument]
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), ParseError> {
		buffer.push(self.sco.to_byte());
		buffer.extend_from_slice(&self.time.to_bytes());
		Ok(())
	}
}

/// Double command with time tag
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct CdcTa1 {
	/// Double command
	pub dco: Dco,
	/// Time tag
	pub time: Cp56Time2a,
}

impl FromBytes for CdcTa1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, ParseError> {
		let dco = Dco::from_byte(*bytes.first().context(NotEnoughBytes)?);
		let time = Cp56Time2a::from_bytes(
			bytes.get(1..8).context(NotEnoughBytes)?.try_into().context(SizedSlice)?,
		)
		.context(ParseTimeTag)?;
		Ok(Self { dco, time })
	}
}

impl ToBytes for CdcTa1 {
	#[instrument]
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), ParseError> {
		buffer.push(self.dco.to_byte());
		buffer.extend_from_slice(&self.time.to_bytes());
		Ok(())
	}
}

/// Regulating step command with time tag
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct CrcTa1 {
	/// Regulating step command
	pub rco: Rco,
	/// Time tag
	pub time: Cp56Time2a,
}

impl FromBytes for CrcTa1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, ParseError> {
		let rco = Rco::from_byte(*bytes.first().context(NotEnoughBytes)?);
		let time = Cp56Time2a::from_bytes(
			bytes.get(1..8).context(NotEnoughBytes)?.try_into().context(SizedSlice)?,
		)
		.context(ParseTimeTag)?;
		Ok(Self { rco, time })
	}
}

impl ToBytes for CrcTa1 {
	#[instrument]
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), ParseError> {
		buffer.push(self.rco.to_byte());
		buffer.extend_from_slice(&self.time.to_bytes());
		Ok(())
	}
}

/// Measured value, normalized value command with time tag
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct CSeTa1 {
	/// Normalized value
	pub nva: i16,
	/// Qualifier of set point command
	pub qos: Qos,
	/// Time tag
	pub time: Cp56Time2a,
}

impl FromBytes for CSeTa1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, ParseError> {
		let nva = i16::from_le_bytes(*bytes.first_chunk::<2>().context(NotEnoughBytes)?);
		let qos = Qos::from_byte(*bytes.get(2).context(NotEnoughBytes)?);
		let time = Cp56Time2a::from_bytes(
			bytes.get(3..10).context(NotEnoughBytes)?.try_into().context(SizedSlice)?,
		)
		.context(ParseTimeTag)?;
		Ok(Self { nva, qos, time })
	}
}

impl ToBytes for CSeTa1 {
	#[instrument]
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), ParseError> {
		buffer.extend_from_slice(&self.nva.to_le_bytes());
		buffer.push(self.qos.to_byte());
		buffer.extend_from_slice(&self.time.to_bytes());
		Ok(())
	}
}

/// Measured value, scaled value command with time tag
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct CSeTb1 {
	/// Scaled value
	pub sva: i16,
	/// Qualifier of set point command
	pub qos: Qos,
	/// Time tag
	pub time: Cp56Time2a,
}

impl FromBytes for CSeTb1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, ParseError> {
		let sva = i16::from_le_bytes(*bytes.first_chunk::<2>().context(NotEnoughBytes)?);
		let qos = Qos::from_byte(*bytes.get(2).context(NotEnoughBytes)?);
		let time = Cp56Time2a::from_bytes(
			bytes.get(3..10).context(NotEnoughBytes)?.try_into().context(SizedSlice)?,
		)
		.context(ParseTimeTag)?;
		Ok(Self { sva, qos, time })
	}
}

impl ToBytes for CSeTb1 {
	#[instrument]
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), ParseError> {
		buffer.extend_from_slice(&self.sva.to_le_bytes());
		buffer.push(self.qos.to_byte());
		buffer.extend_from_slice(&self.time.to_bytes());
		Ok(())
	}
}

/// Measured value, short floating point number command with time tag
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CSeTc1 {
	/// Short floating point
	pub value: f32,
	/// Qualifier of set point command
	pub qos: Qos,
	/// Time tag
	pub time: Cp56Time2a,
}

impl FromBytes for CSeTc1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, ParseError> {
		let value = f32::from_le_bytes(*bytes.first_chunk::<4>().context(NotEnoughBytes)?);
		let qos = Qos::from_byte(*bytes.get(4).context(NotEnoughBytes)?);
		let time = Cp56Time2a::from_bytes(
			bytes.get(5..12).context(NotEnoughBytes)?.try_into().context(SizedSlice)?,
		)
		.context(ParseTimeTag)?;
		Ok(Self { value, qos, time })
	}
}

impl ToBytes for CSeTc1 {
	#[instrument]
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), ParseError> {
		buffer.extend_from_slice(&self.value.to_le_bytes());
		buffer.push(self.qos.to_byte());
		buffer.extend_from_slice(&self.time.to_bytes());
		Ok(())
	}
}

/// Bitstring of 32 bit command with time tag
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct CBoTa1 {
	/// Bit string of 32 bits
	pub bsi: u32,
	/// Time tag
	pub time: Cp56Time2a,
}

impl FromBytes for CBoTa1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, ParseError> {
		let (bsi_bytes, time_bytes) = bytes.split_at(4);
		let bsi = u32::from_le_bytes(*bsi_bytes.first_chunk::<4>().context(NotEnoughBytes)?);
		let time = Cp56Time2a::from_bytes(time_bytes.try_into().context(SizedSlice)?)
			.context(ParseTimeTag)?;
		Ok(Self { bsi, time })
	}
}

impl ToBytes for CBoTa1 {
	#[instrument]
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), ParseError> {
		buffer.extend_from_slice(&self.bsi.to_le_bytes());
		buffer.extend_from_slice(&self.time.to_bytes());
		Ok(())
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
	fn from_bytes(bytes: &[u8]) -> Result<Self, ParseError> {
		let qoi = Qoi::from_byte(*bytes.first().context(NotEnoughBytes)?);
		Ok(Self { qoi })
	}
}

impl ToBytes for CIcNa1 {
	#[instrument]
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), ParseError> {
		buffer.push(self.qoi.to_byte());
		Ok(())
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
	fn from_bytes(bytes: &[u8]) -> Result<Self, ParseError> {
		let byte = *bytes.first().context(NotEnoughBytes)?;
		let rqt = Rqt::from_byte(byte & 0b0011_1111);
		let frz = Frz::from_byte(byte >> 6);
		Ok(Self { rqt, frz })
	}
}

impl ToBytes for CCiNa1 {
	#[instrument]
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), ParseError> {
		let mut byte: u8 = 0;
		byte |= self.rqt.to_byte() & 0b0011_1111;
		byte |= (self.frz as u8) << 6;
		buffer.push(byte);
		Ok(())
	}
}

/// Read command
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct CRdNa1 {
	// No data fields for read command
}

impl FromBytes for CRdNa1 {
	#[instrument]
	fn from_bytes(_: &[u8]) -> Result<Self, ParseError> {
		Ok(Self {})
	}
}

impl ToBytes for CRdNa1 {
	#[instrument]
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), ParseError> {
		Ok(())
	}
}

/// Clock synchronization command
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct CCsNa1 {
	/// Time tag
	pub time: Cp56Time2a,
}

impl FromBytes for CCsNa1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, ParseError> {
		let time = Cp56Time2a::from_bytes(bytes.first_chunk::<7>().context(NotEnoughBytes)?)
			.context(ParseTimeTag)?;
		Ok(Self { time })
	}
}

impl ToBytes for CCsNa1 {
	#[instrument]
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), ParseError> {
		buffer.extend_from_slice(&self.time.to_bytes());
		Ok(())
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
	fn from_bytes(bytes: &[u8]) -> Result<Self, ParseError> {
		let tsc = u16::from_le_bytes(*bytes.first_chunk::<2>().context(NotEnoughBytes)?);
		Ok(Self { tsc })
	}
}

impl ToBytes for CTsNa1 {
	#[instrument]
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), ParseError> {
		buffer.extend_from_slice(&self.tsc.to_le_bytes());
		Ok(())
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
	fn from_bytes(bytes: &[u8]) -> Result<Self, ParseError> {
		let qrp = Qrp::from_byte(*bytes.first().context(NotEnoughBytes)?);
		Ok(Self { qrp })
	}
}

impl ToBytes for CRpNa1 {
	#[instrument]
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), ParseError> {
		buffer.push(self.qrp.to_byte());
		Ok(())
	}
}

/// Delay acquisition command
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct CCdNa1 {
	/// Delay
	pub delay: Cp16Time2a,
}

impl FromBytes for CCdNa1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, ParseError> {
		let delay = Cp16Time2a::from_bytes(*bytes.first_chunk::<2>().context(NotEnoughBytes)?)
			.context(ParseTimeTag)?;
		Ok(Self { delay })
	}
}

impl ToBytes for CCdNa1 {
	#[instrument]
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), ParseError> {
		buffer.extend_from_slice(&self.delay.to_bytes());
		Ok(())
	}
}

/// Test command with time tag
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct CTsTa1 {
	/// Test value
	pub tsc: u16,
	/// Time tag
	pub time: Cp56Time2a,
}

impl FromBytes for CTsTa1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, ParseError> {
		let (tsc_bytes, time_bytes) = bytes.split_at(2);
		let tsc = u16::from_le_bytes(tsc_bytes.try_into().context(SizedSlice)?);
		let time = Cp56Time2a::from_bytes(time_bytes.try_into().context(SizedSlice)?)
			.context(ParseTimeTag)?;
		Ok(Self { tsc, time })
	}
}

impl ToBytes for CTsTa1 {
	#[instrument]
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), ParseError> {
		buffer.extend_from_slice(&self.tsc.to_le_bytes());
		buffer.extend_from_slice(&self.time.to_bytes());
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	// ---- Sco / Dco / Rco: QU lives in bits 2-6 ----
	//
	// Pre-fix bug: `byte & 0b0111_1100 >> 2` was `byte & 0b0001_1111` (bits
	// 0-4) due to operator precedence, mixing in SCS/DCS/RCS bits and
	// truncating QU's high bit.

	#[test]
	fn sco_qu_short_pulse_round_trip() {
		// S/E=Execute, QU=ShortPulse(1)<<2 = 0b0000_0100, SCS=On(1)
		let sco = Sco::from_byte(0b0000_0101);
		assert_eq!(sco.qu, Qu::ShortPulse);
		assert_eq!(sco.scs, Spi::On);
		assert_eq!(sco.se, SelectExecute::Execute);
		assert_eq!(sco.to_byte(), 0b0000_0101);
	}

	#[test]
	fn sco_qu_persistent_extracted_correctly() {
		// QU=Persistent(3)<<2 = 0b0000_1100, SCS=Off
		// Pre-fix this would have parsed QU as Qu::Other(0xC & 0x1F = 12).
		let sco = Sco::from_byte(0b0000_1100);
		assert_eq!(sco.qu, Qu::Persistent);
		assert_eq!(sco.scs, Spi::Off);
	}

	#[test]
	fn sco_qu_high_bit_preserved() {
		// QU=16 (Other) needs bit 6 set: 16<<2 = 0b0100_0000
		// Pre-fix: byte & 0b0001_1111 dropped bit 6 → wrong QU.
		let sco = Sco::from_byte(0b0100_0000);
		assert_eq!(sco.qu, Qu::Other(16));
	}

	#[test]
	fn sco_select_bit_independent_of_qu() {
		// S/E=Select(1), QU=Unspecified, SCS=On -> 0b1000_0001 = 0x81
		let sco = Sco::from_byte(0x81);
		assert_eq!(sco.se, SelectExecute::Select);
		assert_eq!(sco.qu, Qu::Unspecified);
		assert_eq!(sco.scs, Spi::On);
		assert_eq!(sco.to_byte(), 0x81);
	}

	#[test]
	fn dco_qu_long_pulse_round_trip() {
		// QU=LongPulse(2)<<2 = 0b0000_1000, DCS=On(2)
		let dco = Dco::from_byte(0b0000_1010);
		assert_eq!(dco.qu, Qu::LongPulse);
		assert_eq!(dco.dcs, Dpi::On);
		assert_eq!(dco.to_byte(), 0b0000_1010);
	}

	#[test]
	fn rco_qu_short_pulse_with_increment() {
		// QU=ShortPulse(1)<<2 = 0b0000_0100, RCS=Increment(2)
		let rco = Rco::from_byte(0b0000_0110);
		assert_eq!(rco.qu, Qu::ShortPulse);
		assert_eq!(rco.rcs, Rcs::Increment);
		assert_eq!(rco.to_byte(), 0b0000_0110);
	}

	// ---- Set-point commands: nva/sva are signed ----

	#[test]
	fn c_se_na_1_negative_setpoint_round_trip() {
		// nva = -1000 (LE: 0x18 0xFC), qos = QL=2, S/E=Execute -> 0x02
		let bytes = [0x18, 0xFC, 0x02];
		let parsed = CSeNa1::from_bytes(&bytes).unwrap();
		assert_eq!(parsed.nva, -1000);
		assert_eq!(parsed.qos.ql, 2);
		assert_eq!(parsed.qos.se, SelectExecute::Execute);
		let mut out = Vec::new();
		parsed.to_bytes(&mut out).unwrap();
		assert_eq!(out, bytes);
	}

	#[test]
	fn c_se_nb_1_negative_scaled_round_trip() {
		// sva = -32000 (LE: 0x00 0x83), qos = QL=0, S/E=Select -> 0x80
		let bytes = [0x00, 0x83, 0x80];
		let parsed = CSeNb1::from_bytes(&bytes).unwrap();
		assert_eq!(parsed.sva, -32000);
		assert_eq!(parsed.qos.se, SelectExecute::Select);
		let mut out = Vec::new();
		parsed.to_bytes(&mut out).unwrap();
		assert_eq!(out, bytes);
	}
}
