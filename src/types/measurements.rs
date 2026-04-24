use snafu::{OptionExt as _, ResultExt as _};
use tracing::instrument;

use crate::types::{
	FromBytes, NotEnoughBytes, ParseError, ParseTimeTag, SizedSlice, ToBytes,
	information_elements::*,
	quality_descriptors::{Qdp, Qds, SeqQd},
	time::{Cp16Time2a, Cp24Time2a, Cp56Time2a},
};

/// Single-point
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct MSpNa1 {
	/// Single-point with quality descriptor
	pub siq: Siq,
}

impl FromBytes for MSpNa1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, ParseError> {
		let siq = Siq::from_byte(*bytes.first().context(NotEnoughBytes)?);
		Ok(Self { siq })
	}
}

impl ToBytes for MSpNa1 {
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), ParseError> {
		buffer.push(self.siq.to_byte());
		Ok(())
	}
}

/// Single-point with time tag
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct MSpTa1 {
	/// Single-point with quality descriptor
	pub siq: Siq,
	/// Time tag
	pub time: Cp24Time2a,
}

impl FromBytes for MSpTa1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, ParseError> {
		let siq = Siq::from_byte(*bytes.first().context(NotEnoughBytes)?);
		let time = Cp24Time2a::from_bytes(
			bytes.get(1..4).context(NotEnoughBytes)?.try_into().context(SizedSlice)?,
		)
		.context(ParseTimeTag)?;
		Ok(Self { siq, time })
	}
}

impl ToBytes for MSpTa1 {
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), ParseError> {
		buffer.push(self.siq.to_byte());
		buffer.extend_from_slice(&self.time.to_bytes());
		Ok(())
	}
}

/// Double-point
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct MDpNa1 {
	/// Double-point with quality descriptor
	pub diq: Diq,
}

impl FromBytes for MDpNa1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, ParseError> {
		let diq = Diq::from_byte(*bytes.first().context(NotEnoughBytes)?);
		Ok(Self { diq })
	}
}

impl ToBytes for MDpNa1 {
	#[instrument]
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), ParseError> {
		buffer.push(self.diq.to_byte());
		Ok(())
	}
}

/// Double point information with CP24Time2a time tag
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct MDpTa1 {
	/// Double point information with quality descriptor
	pub diq: Diq,
	/// Time tag
	pub time: Cp24Time2a,
}

impl FromBytes for MDpTa1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, ParseError> {
		let diq = Diq::from_byte(*bytes.first().context(NotEnoughBytes)?);
		let time = Cp24Time2a::from_bytes(
			bytes.get(1..4).context(NotEnoughBytes)?.try_into().context(SizedSlice)?,
		)
		.context(ParseTimeTag)?;
		Ok(Self { diq, time })
	}
}

impl ToBytes for MDpTa1 {
	#[instrument]
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), ParseError> {
		buffer.push(self.diq.to_byte());
		buffer.extend_from_slice(&self.time.to_bytes());
		Ok(())
	}
}

/// Step position information
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct MStNa1 {
	/// Value with transient state indication
	pub vti: Vti,
}

impl FromBytes for MStNa1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, ParseError> {
		let vti = Vti::from_byte(*bytes.first_chunk::<2>().context(NotEnoughBytes)?);
		Ok(Self { vti })
	}
}

impl ToBytes for MStNa1 {
	#[instrument]
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), ParseError> {
		buffer.extend_from_slice(&self.vti.to_bytes());
		Ok(())
	}
}

/// Step position information with time tag
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct MStTa1 {
	/// Value with transient state indication
	pub vti: Vti,
	/// Time tag
	pub time: Cp24Time2a,
}

impl FromBytes for MStTa1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, ParseError> {
		let vti = Vti::from_byte(*bytes.first_chunk::<2>().context(NotEnoughBytes)?);
		let time = Cp24Time2a::from_bytes(
			bytes.get(2..5).context(NotEnoughBytes)?.try_into().context(SizedSlice)?,
		)
		.context(ParseTimeTag)?;
		Ok(Self { vti, time })
	}
}

impl ToBytes for MStTa1 {
	#[instrument]
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), ParseError> {
		buffer.extend_from_slice(&self.vti.to_bytes());
		buffer.extend_from_slice(&self.time.to_bytes());
		Ok(())
	}
}

/// Bitstring of 32 bit
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct MBoNa1 {
	/// Bit string of 32 bits
	pub bsi: u32,
	/// Quality descriptor
	pub qds: Qds,
}

impl FromBytes for MBoNa1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, ParseError> {
		let bsi = u32::from_le_bytes(*bytes.first_chunk::<4>().context(NotEnoughBytes)?);
		let qds = Qds::from_byte(*bytes.get(4).context(NotEnoughBytes)?);
		Ok(Self { bsi, qds })
	}
}

impl ToBytes for MBoNa1 {
	#[instrument]
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), ParseError> {
		buffer.extend_from_slice(&self.bsi.to_le_bytes());
		buffer.push(self.qds.to_byte());
		Ok(())
	}
}

/// Measured value, normalized value
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct MMeNa1 {
	/// Normalized value
	pub nva: i16,
	/// Quality descriptor
	pub qds: Qds,
}

impl FromBytes for MMeNa1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, ParseError> {
		let nva = i16::from_le_bytes(*bytes.first_chunk::<2>().context(NotEnoughBytes)?);
		let qds = Qds::from_byte(*bytes.get(2).context(NotEnoughBytes)?);
		Ok(Self { nva, qds })
	}
}

impl ToBytes for MMeNa1 {
	#[instrument]
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), ParseError> {
		buffer.extend_from_slice(&self.nva.to_le_bytes());
		buffer.push(self.qds.to_byte());
		Ok(())
	}
}

/// Measured value, normalized value with time tag
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct MMeTa1 {
	/// Normalized value
	pub nva: i16,
	/// Quality descriptor
	pub qds: Qds,
	/// Time tag
	pub time: Cp24Time2a,
}

impl FromBytes for MMeTa1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, ParseError> {
		let nva = i16::from_le_bytes(*bytes.first_chunk::<2>().context(NotEnoughBytes)?);
		let qds = Qds::from_byte(*bytes.get(2).context(NotEnoughBytes)?);
		let time = Cp24Time2a::from_bytes(
			bytes.get(3..6).context(NotEnoughBytes)?.try_into().context(SizedSlice)?,
		)
		.context(ParseTimeTag)?;
		Ok(Self { nva, qds, time })
	}
}

impl ToBytes for MMeTa1 {
	#[instrument]
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), ParseError> {
		buffer.extend_from_slice(&self.nva.to_le_bytes());
		buffer.push(self.qds.to_byte());
		buffer.extend_from_slice(&self.time.to_bytes());
		Ok(())
	}
}

/// Measured value, scaled value
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct MMeNb1 {
	/// Scaled value
	pub sva: i16,
	/// Quality descriptor
	pub qds: Qds,
}

impl FromBytes for MMeNb1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, ParseError> {
		let sva = i16::from_le_bytes(*bytes.first_chunk::<2>().context(NotEnoughBytes)?);
		let qds = Qds::from_byte(*bytes.get(2).context(NotEnoughBytes)?);
		Ok(Self { sva, qds })
	}
}

impl ToBytes for MMeNb1 {
	#[instrument]
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), ParseError> {
		buffer.extend_from_slice(&self.sva.to_le_bytes());
		buffer.push(self.qds.to_byte());
		Ok(())
	}
}

/// Measured value, scaled value with time tag
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct MMeTb1 {
	/// Scaled value
	pub sva: i16,
	/// Quality descriptor
	pub qds: Qds,
	/// Time tag
	pub time: Cp24Time2a,
}

impl FromBytes for MMeTb1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, ParseError> {
		let sva = i16::from_le_bytes(*bytes.first_chunk::<2>().context(NotEnoughBytes)?);
		let qds = Qds::from_byte(*bytes.get(2).context(NotEnoughBytes)?);
		let time = Cp24Time2a::from_bytes(
			bytes.get(3..6).context(NotEnoughBytes)?.try_into().context(SizedSlice)?,
		)
		.context(ParseTimeTag)?;
		Ok(Self { sva, qds, time })
	}
}

impl ToBytes for MMeTb1 {
	#[instrument]
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), ParseError> {
		buffer.extend_from_slice(&self.sva.to_le_bytes());
		buffer.push(self.qds.to_byte());
		buffer.extend_from_slice(&self.time.to_bytes());
		Ok(())
	}
}

/// Measured value, short floating point number
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MMeNc1 {
	/// Short floating point
	pub value: f32,
	/// Quality descriptor
	pub qds: Qds,
}

impl FromBytes for MMeNc1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, ParseError> {
		let value = f32::from_le_bytes(*bytes.first_chunk::<4>().context(NotEnoughBytes)?);
		let qds = Qds::from_byte(*bytes.get(4).context(NotEnoughBytes)?);
		Ok(Self { value, qds })
	}
}

impl ToBytes for MMeNc1 {
	#[instrument]
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), ParseError> {
		buffer.extend_from_slice(&self.value.to_le_bytes());
		buffer.push(self.qds.to_byte());
		Ok(())
	}
}

/// Measured value, short floating point number with time tag
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MMeTc1 {
	/// Short floating point
	pub value: f32,
	/// Quality descriptor
	pub qds: Qds,
	/// Time tag
	pub time: Cp24Time2a,
}

impl FromBytes for MMeTc1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, ParseError> {
		let value = f32::from_le_bytes(*bytes.first_chunk::<4>().context(NotEnoughBytes)?);
		let qds = Qds::from_byte(*bytes.get(4).context(NotEnoughBytes)?);
		let time = Cp24Time2a::from_bytes(
			bytes.get(5..8).context(NotEnoughBytes)?.try_into().context(SizedSlice)?,
		)
		.context(ParseTimeTag)?;
		Ok(Self { value, qds, time })
	}
}

impl ToBytes for MMeTc1 {
	#[instrument]
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), ParseError> {
		buffer.extend_from_slice(&self.value.to_le_bytes());
		buffer.push(self.qds.to_byte());
		buffer.extend_from_slice(&self.time.to_bytes());
		Ok(())
	}
}

/// Integrated totals
///
/// Per IEC 60870-5-101 §7.2.6.9: 32-bit signed counter reading followed by a
/// sequence-quality descriptor (SeqQd) carrying sequence number, `CY`, `CA`
/// and `IV` bits.
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct MItNa1 {
	/// Binary counter reading (signed 32-bit integer)
	pub bcr: i32,
	/// Sequence-quality descriptor
	pub sqd: SeqQd,
}

impl FromBytes for MItNa1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, ParseError> {
		let bcr = i32::from_le_bytes(*bytes.first_chunk::<4>().context(NotEnoughBytes)?);
		let sqd = SeqQd::from_byte(*bytes.get(4).context(NotEnoughBytes)?);
		Ok(Self { bcr, sqd })
	}
}

impl ToBytes for MItNa1 {
	#[instrument]
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), ParseError> {
		buffer.extend_from_slice(&self.bcr.to_le_bytes());
		buffer.push(self.sqd.to_byte());
		Ok(())
	}
}

/// Event of protection equipment with time tag
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct MEpTa1 {
	/// Single event of protection equipment
	pub sep: Sep,
	/// Elapsed time
	pub elapsed: Cp16Time2a,
	/// Time tag
	pub time: Cp24Time2a,
}

impl FromBytes for MEpTa1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, ParseError> {
		let sep = Sep::from_byte(*bytes.first().context(NotEnoughBytes)?);
		let elapsed = Cp16Time2a::from_bytes(
			bytes.get(1..3).context(NotEnoughBytes)?.try_into().context(SizedSlice)?,
		)
		.context(ParseTimeTag)?;
		let time = Cp24Time2a::from_bytes(
			bytes.get(3..6).context(NotEnoughBytes)?.try_into().context(SizedSlice)?,
		)
		.context(ParseTimeTag)?;
		Ok(Self { sep, elapsed, time })
	}
}

impl ToBytes for MEpTa1 {
	#[instrument]
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), ParseError> {
		buffer.push(self.sep.to_byte());
		buffer.extend_from_slice(&self.elapsed.to_bytes());
		buffer.extend_from_slice(&self.time.to_bytes());
		Ok(())
	}
}

/// Packed start events of protection equipment with time tag
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct MEpTb1 {
	/// Start events of protection equipment
	pub start_ep: StartEp,
	/// Quality descriptor of protection equipment
	pub qdp: Qdp,
	///  Relay duration time
	pub relay_duration: Cp16Time2a,
	/// Time tag
	pub time: Cp24Time2a,
}

impl FromBytes for MEpTb1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, ParseError> {
		let start_ep = StartEp::from_byte(*bytes.first().context(NotEnoughBytes)?);
		let qdp = Qdp::from_byte(*bytes.get(1).context(NotEnoughBytes)?);
		let relay_duration = Cp16Time2a::from_bytes(
			bytes.get(2..4).context(NotEnoughBytes)?.try_into().context(SizedSlice)?,
		)
		.context(ParseTimeTag)?;
		let time = Cp24Time2a::from_bytes(
			bytes.get(4..7).context(NotEnoughBytes)?.try_into().context(SizedSlice)?,
		)
		.context(ParseTimeTag)?;
		Ok(Self { start_ep, qdp, relay_duration, time })
	}
}

impl ToBytes for MEpTb1 {
	#[instrument]
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), ParseError> {
		buffer.push(self.start_ep.to_byte());
		buffer.push(self.qdp.to_byte());
		buffer.extend_from_slice(&self.relay_duration.to_bytes());
		buffer.extend_from_slice(&self.time.to_bytes());
		Ok(())
	}
}

/// Packed output circuit information of protection equipment with time tag
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct MEpTc1 {
	/// Output circuit information
	pub oci: Oci,
	/// Quality descriptor of protection equipment
	pub qdp: Qdp,
	/// Relay operation time
	pub relay_op_time: Cp16Time2a,
	/// Time tag
	pub time: Cp24Time2a,
}

impl FromBytes for MEpTc1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, ParseError> {
		let oci = Oci::from_byte(*bytes.first().context(NotEnoughBytes)?);
		let qdp = Qdp::from_byte(*bytes.get(1).context(NotEnoughBytes)?);
		let relay_op_time = Cp16Time2a::from_bytes(
			bytes.get(2..4).context(NotEnoughBytes)?.try_into().context(SizedSlice)?,
		)
		.context(ParseTimeTag)?;
		let time = Cp24Time2a::from_bytes(
			bytes.get(4..7).context(NotEnoughBytes)?.try_into().context(SizedSlice)?,
		)
		.context(ParseTimeTag)?;
		Ok(Self { oci, qdp, relay_op_time, time })
	}
}

impl ToBytes for MEpTc1 {
	#[instrument]
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), ParseError> {
		buffer.push(self.oci.to_byte());
		buffer.push(self.qdp.to_byte());
		buffer.extend_from_slice(&self.relay_op_time.to_bytes());
		buffer.extend_from_slice(&self.time.to_bytes());
		Ok(())
	}
}

/// Packed single point information with status change detection
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct MPsNa1 {
	/// Bit string of 32 bits
	pub bsi: u32,
	/// Quality descriptor
	pub qds: Qds,
}

impl FromBytes for MPsNa1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, ParseError> {
		let bsi = u32::from_le_bytes(*bytes.first_chunk::<4>().context(NotEnoughBytes)?);
		let qds = Qds::from_byte(*bytes.get(4).context(NotEnoughBytes)?);
		Ok(Self { bsi, qds })
	}
}

impl ToBytes for MPsNa1 {
	#[instrument]
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), ParseError> {
		buffer.extend_from_slice(&self.bsi.to_le_bytes());
		buffer.push(self.qds.to_byte());
		Ok(())
	}
}

/// Measured value, normalized value without quality descriptor
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct MMeNd1 {
	/// Normalized value
	pub nva: i16,
}

impl FromBytes for MMeNd1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, ParseError> {
		let nva = i16::from_le_bytes(*bytes.first_chunk::<2>().context(NotEnoughBytes)?);
		Ok(Self { nva })
	}
}

impl ToBytes for MMeNd1 {
	#[instrument]
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), ParseError> {
		buffer.extend_from_slice(&self.nva.to_le_bytes());
		Ok(())
	}
}

/// Single-point information with CP56Time2a time tag
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct MSpTb1 {
	/// Single-point with quality descriptor
	pub siq: Siq,
	/// Time tag
	pub time: Cp56Time2a,
}

impl FromBytes for MSpTb1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, ParseError> {
		let siq = Siq::from_byte(*bytes.first().context(NotEnoughBytes)?);
		let time = Cp56Time2a::from_bytes(
			bytes.get(1..8).context(NotEnoughBytes)?.try_into().context(SizedSlice)?,
		)
		.context(ParseTimeTag)?;
		Ok(Self { siq, time })
	}
}

impl ToBytes for MSpTb1 {
	#[instrument]
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), ParseError> {
		buffer.push(self.siq.to_byte());
		buffer.extend_from_slice(&self.time.to_bytes());
		Ok(())
	}
}

/// Double-point information with CP56Time2a time tag
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct MDpTb1 {
	/// Double point information with quality descriptor
	pub diq: Diq,
	/// Time tag
	pub time: Cp56Time2a,
}

impl FromBytes for MDpTb1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, ParseError> {
		let diq = Diq::from_byte(*bytes.first().context(NotEnoughBytes)?);
		let time = Cp56Time2a::from_bytes(
			bytes.get(1..8).context(NotEnoughBytes)?.try_into().context(SizedSlice)?,
		)
		.context(ParseTimeTag)?;
		Ok(Self { diq, time })
	}
}

impl ToBytes for MDpTb1 {
	#[instrument]
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), ParseError> {
		buffer.push(self.diq.to_byte());
		buffer.extend_from_slice(&self.time.to_bytes());
		Ok(())
	}
}
/// Step position information with CP56Time2a time tag
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct MStTb1 {
	/// Value with transient state indication
	pub vti: Vti,
	/// Time tag
	pub time: Cp56Time2a,
}

impl FromBytes for MStTb1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, ParseError> {
		let vti = Vti::from_byte(*bytes.first_chunk::<2>().context(NotEnoughBytes)?);
		let time = Cp56Time2a::from_bytes(
			bytes.get(2..9).context(NotEnoughBytes)?.try_into().context(SizedSlice)?,
		)
		.context(ParseTimeTag)?;
		Ok(Self { vti, time })
	}
}

impl ToBytes for MStTb1 {
	#[instrument]
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), ParseError> {
		buffer.extend_from_slice(&self.vti.to_bytes());
		buffer.extend_from_slice(&self.time.to_bytes());
		Ok(())
	}
}

/// Bitstring of 32 bit with CP56Time2a time tag
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct MBoTb1 {
	/// Bit string of 32 bits
	pub bsi: u32,
	/// Quality descriptor
	pub qds: Qds,
	/// Time tag
	pub time: Cp56Time2a,
}

impl FromBytes for MBoTb1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, ParseError> {
		let bsi = u32::from_le_bytes(*bytes.first_chunk::<4>().context(NotEnoughBytes)?);
		let qds = Qds::from_byte(*bytes.get(4).context(NotEnoughBytes)?);
		let time = Cp56Time2a::from_bytes(
			bytes.get(5..12).context(NotEnoughBytes)?.try_into().context(SizedSlice)?,
		)
		.context(ParseTimeTag)?;
		Ok(Self { bsi, qds, time })
	}
}

impl ToBytes for MBoTb1 {
	#[instrument]
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), ParseError> {
		buffer.extend_from_slice(&self.bsi.to_le_bytes());
		buffer.push(self.qds.to_byte());
		buffer.extend_from_slice(&self.time.to_bytes());
		Ok(())
	}
}

/// Measured value, normalized value with CP56Time2a time tag
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct MMeTd1 {
	/// Normalized value
	pub nva: i16,
	/// Quality descriptor
	pub qds: Qds,
	/// Time tag
	pub time: Cp56Time2a,
}

impl FromBytes for MMeTd1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, ParseError> {
		let nva = i16::from_le_bytes(*bytes.first_chunk::<2>().context(NotEnoughBytes)?);
		let qds = Qds::from_byte(*bytes.get(2).context(NotEnoughBytes)?);
		let time = Cp56Time2a::from_bytes(
			bytes.get(3..10).context(NotEnoughBytes)?.try_into().context(SizedSlice)?,
		)
		.context(ParseTimeTag)?;
		Ok(Self { nva, qds, time })
	}
}

impl ToBytes for MMeTd1 {
	#[instrument]
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), ParseError> {
		buffer.extend_from_slice(&self.nva.to_le_bytes());
		buffer.push(self.qds.to_byte());
		buffer.extend_from_slice(&self.time.to_bytes());
		Ok(())
	}
}

/// Measured value, scaled value with CP56Time2a time tag
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct MMeTe1 {
	/// Scaled value
	pub sva: i16,
	/// Quality descriptor
	pub qds: Qds,
	/// Time tag
	pub time: Cp56Time2a,
}

impl FromBytes for MMeTe1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, ParseError> {
		let sva = i16::from_le_bytes(*bytes.first_chunk::<2>().context(NotEnoughBytes)?);
		let qds = Qds::from_byte(*bytes.get(2).context(NotEnoughBytes)?);
		let time = Cp56Time2a::from_bytes(
			bytes.get(3..10).context(NotEnoughBytes)?.try_into().context(SizedSlice)?,
		)
		.context(ParseTimeTag)?;
		Ok(Self { sva, qds, time })
	}
}

impl ToBytes for MMeTe1 {
	#[instrument]
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), ParseError> {
		buffer.extend_from_slice(&self.sva.to_le_bytes());
		buffer.push(self.qds.to_byte());
		buffer.extend_from_slice(&self.time.to_bytes());
		Ok(())
	}
}

/// Measured value, short floating point number with CP56Time2a time tag
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MMeTf1 {
	/// Short floating point
	pub value: f32,
	/// Quality descriptor
	pub qds: Qds,
	/// Time tag
	pub time: Cp56Time2a,
}

impl FromBytes for MMeTf1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, ParseError> {
		let value = f32::from_le_bytes(*bytes.first_chunk::<4>().context(NotEnoughBytes)?);
		let qds = Qds::from_byte(*bytes.get(4).context(NotEnoughBytes)?);
		let time = Cp56Time2a::from_bytes(
			bytes.get(5..12).context(NotEnoughBytes)?.try_into().context(SizedSlice)?,
		)
		.context(ParseTimeTag)?;
		Ok(Self { value, qds, time })
	}
}

impl ToBytes for MMeTf1 {
	#[instrument]
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), ParseError> {
		buffer.extend_from_slice(&self.value.to_le_bytes());
		buffer.push(self.qds.to_byte());
		buffer.extend_from_slice(&self.time.to_bytes());
		Ok(())
	}
}
/// Integrated totals with CP56Time2a time tag
///
/// Per IEC 60870-5-101 §7.2.6.9, same as [`MItNa1`] plus a 7-byte CP56Time2a
/// tag.
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct MItTb1 {
	/// Binary counter reading (signed 32-bit integer)
	pub bcr: i32,
	/// Sequence-quality descriptor
	pub sqd: SeqQd,
	/// Time tag
	pub time: Cp56Time2a,
}

impl FromBytes for MItTb1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, ParseError> {
		let bcr = i32::from_le_bytes(*bytes.first_chunk::<4>().context(NotEnoughBytes)?);
		let sqd = SeqQd::from_byte(*bytes.get(4).context(NotEnoughBytes)?);
		let time = Cp56Time2a::from_bytes(
			bytes.get(5..12).context(NotEnoughBytes)?.try_into().context(SizedSlice)?,
		)
		.context(ParseTimeTag)?;
		Ok(Self { bcr, sqd, time })
	}
}

impl ToBytes for MItTb1 {
	#[instrument]
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), ParseError> {
		buffer.extend_from_slice(&self.bcr.to_le_bytes());
		buffer.push(self.sqd.to_byte());
		buffer.extend_from_slice(&self.time.to_bytes());
		Ok(())
	}
}

/// Event of protection equipment with CP56Time2a time tag
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct MEpTd1 {
	/// Single event of protection equipment
	pub sep: Sep,
	/// Elapsed time
	pub elapsed: Cp16Time2a,
	/// Time tag
	pub time: Cp56Time2a,
}

impl FromBytes for MEpTd1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, ParseError> {
		let sep = Sep::from_byte(*bytes.first().context(NotEnoughBytes)?);
		let elapsed = Cp16Time2a::from_bytes(
			bytes.get(1..3).context(NotEnoughBytes)?.try_into().context(SizedSlice)?,
		)
		.context(ParseTimeTag)?;
		let time = Cp56Time2a::from_bytes(
			bytes.get(3..10).context(NotEnoughBytes)?.try_into().context(SizedSlice)?,
		)
		.context(ParseTimeTag)?;
		Ok(Self { sep, elapsed, time })
	}
}

impl ToBytes for MEpTd1 {
	#[instrument]
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), ParseError> {
		buffer.push(self.sep.to_byte());
		buffer.extend_from_slice(&self.elapsed.to_bytes());
		buffer.extend_from_slice(&self.time.to_bytes());
		Ok(())
	}
}

/// Packed start events of protection equipment with CP56Time2a time tag
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct MEpTe1 {
	/// Start events of protection equipment
	pub start_ep: StartEp,
	/// Quality descriptor of protection equipment
	pub qdp: Qdp,
	/// Relay duration time
	pub relay_duration: Cp16Time2a,
	/// Time tag
	pub time: Cp56Time2a,
}

impl FromBytes for MEpTe1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, ParseError> {
		let start_ep = StartEp::from_byte(*bytes.first().context(NotEnoughBytes)?);
		let qdp = Qdp::from_byte(*bytes.get(1).context(NotEnoughBytes)?);
		let relay_duration = Cp16Time2a::from_bytes(
			bytes.get(2..4).context(NotEnoughBytes)?.try_into().context(SizedSlice)?,
		)
		.context(ParseTimeTag)?;
		let time = Cp56Time2a::from_bytes(
			bytes.get(4..11).context(NotEnoughBytes)?.try_into().context(SizedSlice)?,
		)
		.context(ParseTimeTag)?;
		Ok(Self { start_ep, qdp, relay_duration, time })
	}
}

impl ToBytes for MEpTe1 {
	#[instrument]
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), ParseError> {
		buffer.push(self.start_ep.to_byte());
		buffer.push(self.qdp.to_byte());
		buffer.extend_from_slice(&self.relay_duration.to_bytes());
		buffer.extend_from_slice(&self.time.to_bytes());
		Ok(())
	}
}

/// Packed output circuit information of protection equipment with CP56Time2a
/// time tag
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct MEpTf1 {
	/// Output circuit information
	pub oci: Oci,
	/// Quality descriptor of protection equipment
	pub qdp: Qdp,
	/// Relay operation time
	pub relay_op_time: Cp16Time2a,
	/// Time tag
	pub time: Cp56Time2a,
}

impl FromBytes for MEpTf1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, ParseError> {
		let oci = Oci::from_byte(*bytes.first().context(NotEnoughBytes)?);
		let qdp = Qdp::from_byte(*bytes.get(1).context(NotEnoughBytes)?);
		let relay_op_time = Cp16Time2a::from_bytes(
			bytes.get(2..4).context(NotEnoughBytes)?.try_into().context(SizedSlice)?,
		)
		.context(ParseTimeTag)?;
		let time = Cp56Time2a::from_bytes(
			bytes.get(4..11).context(NotEnoughBytes)?.try_into().context(SizedSlice)?,
		)
		.context(ParseTimeTag)?;
		Ok(Self { oci, qdp, relay_op_time, time })
	}
}

impl ToBytes for MEpTf1 {
	#[instrument]
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), ParseError> {
		buffer.push(self.oci.to_byte());
		buffer.push(self.qdp.to_byte());
		buffer.extend_from_slice(&self.relay_op_time.to_bytes());
		buffer.extend_from_slice(&self.time.to_bytes());
		Ok(())
	}
}

/// End of initialization
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct MEiNa1 {
	/// Local parameter change
	pub lpc: Lpc,
	/// Cause of initialization
	pub coi: Coi,
}

impl FromBytes for MEiNa1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, ParseError> {
		let byte = *bytes.first().context(NotEnoughBytes)?;
		let lpc = Lpc::from_bool(byte & 0b1000_0000 != 0);
		let coi = Coi::from_byte(byte & 0b0111_1111);
		Ok(Self { lpc, coi })
	}
}

impl ToBytes for MEiNa1 {
	#[instrument]
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), ParseError> {
		let mut byte: u8 = 0;
		byte |= (self.lpc as u8) << 7;
		byte |= self.coi.to_byte();
		buffer.push(byte);
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn round_trip<T: FromBytes + ToBytes + PartialEq + std::fmt::Debug>(bytes: &[u8]) -> T {
		let parsed = T::from_bytes(bytes).expect("parse");
		let mut out = Vec::new();
		parsed.to_bytes(&mut out).expect("serialize");
		assert_eq!(out, bytes, "wire bytes must match after round-trip");
		parsed
	}

	// ---- BCR (M_IT_NA_1 / M_IT_TB_1) ----

	#[test]
	fn m_it_na_1_negative_counter_round_trip() {
		// bcr = -1 (0xFFFF_FFFF LE), sqd = IV+CY+seq=5  -> 0b1010_0101 = 0xA5
		let bytes = [0xFF, 0xFF, 0xFF, 0xFF, 0xA5];
		let v: MItNa1 = round_trip(&bytes);
		assert_eq!(v.bcr, -1);
		assert_eq!(v.sqd.seq, 5);
		assert!(v.sqd.iv);
		assert!(v.sqd.cy);
		assert!(!v.sqd.ca);
	}

	#[test]
	fn m_it_na_1_max_signed_counter() {
		let bytes = [0xFF, 0xFF, 0xFF, 0x7F, 0x00];
		let v: MItNa1 = round_trip(&bytes);
		assert_eq!(v.bcr, i32::MAX);
	}

	#[test]
	fn m_it_na_1_min_signed_counter() {
		let bytes = [0x00, 0x00, 0x00, 0x80, 0x00];
		let v: MItNa1 = round_trip(&bytes);
		assert_eq!(v.bcr, i32::MIN);
	}

	#[test]
	fn m_it_tb_1_serializes_little_endian() {
		// Pre-fix this would have been big-endian on serialize.
		let v = MItTb1 { bcr: 0x1234_5678, sqd: SeqQd::default(), time: Cp56Time2a::default() };
		let mut out = Vec::new();
		v.to_bytes(&mut out).unwrap();
		assert_eq!(&out[0..4], &[0x78, 0x56, 0x34, 0x12], "BCR must be little-endian");
	}

	#[test]
	fn m_it_tb_1_round_trip() {
		// counter 0x0102_0304, sqd seq=10, time = all zeros
		let bytes = [0x04, 0x03, 0x02, 0x01, 0x0A, 0, 0, 0, 0, 0, 0, 0];
		let v: MItTb1 = round_trip(&bytes);
		assert_eq!(v.bcr, 0x0102_0304);
		assert_eq!(v.sqd.seq, 10);
	}

	// ---- NVA (signed normalized) ----

	#[test]
	fn m_me_na_1_negative_normalized() {
		// nva = -32768 (0x8000 LE = 0x00 0x80), qds = 0
		let bytes = [0x00, 0x80, 0x00];
		let v: MMeNa1 = round_trip(&bytes);
		assert_eq!(v.nva, i16::MIN);
	}

	#[test]
	fn m_me_na_1_max_normalized() {
		let bytes = [0xFF, 0x7F, 0x00];
		let v: MMeNa1 = round_trip(&bytes);
		assert_eq!(v.nva, i16::MAX);
	}

	// ---- SVA (signed scaled) ----

	#[test]
	fn m_me_nb_1_negative_scaled() {
		let bytes = [0x9C, 0xFF, 0x00]; // -100 LE = 0xFF9C
		let v: MMeNb1 = round_trip(&bytes);
		assert_eq!(v.sva, -100);
	}

	#[test]
	fn m_me_nb_1_round_trip_with_quality() {
		// sva = 1234, qds = IV (0x80)
		let bytes = [0xD2, 0x04, 0x80];
		let v: MMeNb1 = round_trip(&bytes);
		assert_eq!(v.sva, 1234);
		assert!(v.qds.iv);
	}
}
