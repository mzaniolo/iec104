use snafu::{OptionExt as _, ResultExt as _};
use tracing::instrument;

use crate::types::{
	FromBytes, NotEnoughBytes, ParseError, ParseTimeTag, SizedSlice, ToBytes,
	information_elements::*,
	quality_descriptors::{Qdp, Qds},
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
	fn from_bytes(bytes: &[u8]) -> Result<Self, Box<ParseError>> {
		let siq = Siq::from_byte(*bytes.first().context(NotEnoughBytes)?);
		Ok(Self { siq })
	}
}

impl ToBytes for MSpNa1 {
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), Box<ParseError>> {
		buffer.push(self.siq.to_byte());
		Ok(())
	}
}

/// Single-point with time tag
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct MSpTa1 {
	/// Single-point with quality descriptor
	pub siq: Siq,
	/// Time tag
	pub time: Cp24Time2a,
}

impl FromBytes for MSpTa1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, Box<ParseError>> {
		let siq = Siq::from_byte(*bytes.first().context(NotEnoughBytes)?);
		let time = Cp24Time2a::from_bytes(
			bytes.get(1..4).context(NotEnoughBytes)?.try_into().context(SizedSlice)?,
		)
		.context(ParseTimeTag)?;
		Ok(Self { siq, time })
	}
}

impl ToBytes for MSpTa1 {
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), Box<ParseError>> {
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
	fn from_bytes(bytes: &[u8]) -> Result<Self, Box<ParseError>> {
		let diq = Diq::from_byte(*bytes.first().context(NotEnoughBytes)?);
		Ok(Self { diq })
	}
}

impl ToBytes for MDpNa1 {
	#[instrument]
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), Box<ParseError>> {
		buffer.push(self.diq.to_byte());
		Ok(())
	}
}

/// Double point information with CP24Time2a time tag
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct MDpTa1 {
	/// Double point information with quality descriptor
	pub diq: Diq,
	/// Time tag
	pub time: Cp24Time2a,
}

impl FromBytes for MDpTa1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, Box<ParseError>> {
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
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), Box<ParseError>> {
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
	fn from_bytes(bytes: &[u8]) -> Result<Self, Box<ParseError>> {
		let vti = Vti::from_byte(*bytes.first_chunk::<2>().context(NotEnoughBytes)?);
		Ok(Self { vti })
	}
}

impl ToBytes for MStNa1 {
	#[instrument]
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), Box<ParseError>> {
		buffer.extend_from_slice(&self.vti.to_bytes());
		Ok(())
	}
}

/// Step position information with time tag
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct MStTa1 {
	/// Value with transient state indication
	pub vti: Vti,
	/// Time tag
	pub time: Cp24Time2a,
}

impl FromBytes for MStTa1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, Box<ParseError>> {
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
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), Box<ParseError>> {
		buffer.extend_from_slice(&self.vti.to_bytes());
		buffer.extend_from_slice(&self.time.to_bytes());
		Ok(())
	}
}

/// Bitstring of 32 bit
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct MBoNa1 {
	/// Bit string of 32 bits
	pub bsi: Bsi,
	/// Quality descriptor
	pub qds: Qds,
}

impl FromBytes for MBoNa1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, Box<ParseError>> {
		let bsi = Bsi::from_byte(*bytes.first_chunk::<4>().context(NotEnoughBytes)?);
		let qds = Qds::from_byte(*bytes.get(4).context(NotEnoughBytes)?);
		Ok(Self { bsi, qds })
	}
}

impl ToBytes for MBoNa1 {
	#[instrument]
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), Box<ParseError>> {
		buffer.extend_from_slice(&self.bsi.to_bytes());
		buffer.push(self.qds.to_byte());
		Ok(())
	}
}

/// Measured value, normalized value
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct MMeNa1 {
	/// Normalized value
	pub nva: Nva,
	/// Quality descriptor
	pub qds: Qds,
}

impl FromBytes for MMeNa1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, Box<ParseError>> {
		let nva = Nva::from_bytes(*bytes.first_chunk::<2>().context(NotEnoughBytes)?);
		let qds = Qds::from_byte(*bytes.get(2).context(NotEnoughBytes)?);
		Ok(Self { nva, qds })
	}
}

impl ToBytes for MMeNa1 {
	#[instrument]
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), Box<ParseError>> {
		buffer.extend_from_slice(&self.nva.to_bytes());
		buffer.push(self.qds.to_byte());
		Ok(())
	}
}

/// Measured value, normalized value with time tag
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct MMeTa1 {
	/// Normalized value
	pub nva: Nva,
	/// Quality descriptor
	pub qds: Qds,
	/// Time tag
	pub time: Cp24Time2a,
}

impl FromBytes for MMeTa1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, Box<ParseError>> {
		let nva = Nva::from_bytes(*bytes.first_chunk::<2>().context(NotEnoughBytes)?);
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
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), Box<ParseError>> {
		buffer.extend_from_slice(&self.nva.to_bytes());
		buffer.push(self.qds.to_byte());
		buffer.extend_from_slice(&self.time.to_bytes());
		Ok(())
	}
}

/// Measured value, scaled value
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct MMeNb1 {
	/// Scaled value
	pub sva: Sva,
	/// Quality descriptor
	pub qds: Qds,
}

impl FromBytes for MMeNb1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, Box<ParseError>> {
		let sva = Sva::from_bytes(*bytes.first_chunk::<2>().context(NotEnoughBytes)?);
		let qds = Qds::from_byte(*bytes.get(2).context(NotEnoughBytes)?);
		Ok(Self { sva, qds })
	}
}

impl ToBytes for MMeNb1 {
	#[instrument]
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), Box<ParseError>> {
		buffer.extend_from_slice(&self.sva.to_bytes());
		buffer.push(self.qds.to_byte());
		Ok(())
	}
}

/// Measured value, scaled value with time tag
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct MMeTb1 {
	/// Scaled value
	pub sva: Sva,
	/// Quality descriptor
	pub qds: Qds,
	/// Time tag
	pub time: Cp24Time2a,
}

impl FromBytes for MMeTb1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, Box<ParseError>> {
		let sva = Sva::from_bytes(*bytes.first_chunk::<2>().context(NotEnoughBytes)?);
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
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), Box<ParseError>> {
		buffer.extend_from_slice(&self.sva.to_bytes());
		buffer.push(self.qds.to_byte());
		buffer.extend_from_slice(&self.time.to_bytes());
		Ok(())
	}
}

/// Measured value, short floating point number
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MMeNc1 {
	/// Short floating point
	pub r32: R32,
	/// Quality descriptor
	pub qds: Qds,
}

impl FromBytes for MMeNc1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, Box<ParseError>> {
		#[allow(clippy::unwrap_used)]
		let r32 = R32::from_bytes(*bytes.first_chunk::<4>().context(NotEnoughBytes)?);
		let qds = Qds::from_byte(*bytes.get(4).context(NotEnoughBytes)?);
		Ok(Self { r32, qds })
	}
}

impl ToBytes for MMeNc1 {
	#[instrument]
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), Box<ParseError>> {
		buffer.extend_from_slice(&self.r32.to_bytes());
		buffer.push(self.qds.to_byte());
		Ok(())
	}
}

/// Measured value, short floating point number with time tag
#[derive(Debug, Clone, PartialEq)]
pub struct MMeTc1 {
	/// Short floating point
	pub r32: R32,
	/// Quality descriptor
	pub qds: Qds,
	/// Time tag
	pub time: Cp24Time2a,
}

impl FromBytes for MMeTc1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, Box<ParseError>> {
		let r32 = R32::from_bytes(*bytes.first_chunk::<4>().context(NotEnoughBytes)?);
		let qds = Qds::from_byte(*bytes.get(4).context(NotEnoughBytes)?);
		let time = Cp24Time2a::from_bytes(
			bytes.get(5..8).context(NotEnoughBytes)?.try_into().context(SizedSlice)?,
		)
		.context(ParseTimeTag)?;
		Ok(Self { r32, qds, time })
	}
}

impl ToBytes for MMeTc1 {
	#[instrument]
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), Box<ParseError>> {
		buffer.extend_from_slice(&self.r32.to_bytes());
		buffer.push(self.qds.to_byte());
		buffer.extend_from_slice(&self.time.to_bytes());
		Ok(())
	}
}

/// Integrated totals
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct MItNa1 {
	/// Binary counter reading
	pub bcr: Bcr,
	/// Quality descriptor
	pub qds: Qds,
}

impl FromBytes for MItNa1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, Box<ParseError>> {
		let bcr = Bcr::from_byte(*bytes.first_chunk::<4>().context(NotEnoughBytes)?);
		let qds = Qds::from_byte(*bytes.get(4).context(NotEnoughBytes)?);
		Ok(Self { bcr, qds })
	}
}

impl ToBytes for MItNa1 {
	#[instrument]
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), Box<ParseError>> {
		buffer.extend_from_slice(&self.bcr.to_bytes());
		buffer.push(self.qds.to_byte());
		Ok(())
	}
}

/// Event of protection equipment with time tag
#[derive(Debug, Clone, Eq, PartialEq)]
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
	fn from_bytes(bytes: &[u8]) -> Result<Self, Box<ParseError>> {
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
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), Box<ParseError>> {
		buffer.push(self.sep.to_byte());
		buffer.extend_from_slice(&self.elapsed.to_bytes());
		buffer.extend_from_slice(&self.time.to_bytes());
		Ok(())
	}
}

/// Packed start events of protection equipment with time tag
#[derive(Debug, Clone, Eq, PartialEq)]
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
	fn from_bytes(bytes: &[u8]) -> Result<Self, Box<ParseError>> {
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
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), Box<ParseError>> {
		buffer.push(self.start_ep.to_byte());
		buffer.push(self.qdp.to_byte());
		buffer.extend_from_slice(&self.relay_duration.to_bytes());
		buffer.extend_from_slice(&self.time.to_bytes());
		Ok(())
	}
}

/// Packed output circuit information of protection equipment with time tag
#[derive(Debug, Clone, Eq, PartialEq)]
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
	fn from_bytes(bytes: &[u8]) -> Result<Self, Box<ParseError>> {
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
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), Box<ParseError>> {
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
	pub bsi: Bsi,
	/// Quality descriptor
	pub qds: Qds,
}

impl FromBytes for MPsNa1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, Box<ParseError>> {
		let bsi = Bsi::from_byte(*bytes.first_chunk::<4>().context(NotEnoughBytes)?);
		let qds = Qds::from_byte(*bytes.get(4).context(NotEnoughBytes)?);
		Ok(Self { bsi, qds })
	}
}

impl ToBytes for MPsNa1 {
	#[instrument]
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), Box<ParseError>> {
		buffer.extend_from_slice(&self.bsi.to_bytes());
		buffer.push(self.qds.to_byte());
		Ok(())
	}
}

/// Measured value, normalized value without quality descriptor
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct MMeNd1 {
	/// Normalized value
	pub nva: Nva,
}

impl FromBytes for MMeNd1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, Box<ParseError>> {
		let nva = Nva::from_bytes(*bytes.first_chunk::<2>().context(NotEnoughBytes)?);
		Ok(Self { nva })
	}
}

impl ToBytes for MMeNd1 {
	#[instrument]
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), Box<ParseError>> {
		buffer.extend_from_slice(&self.nva.to_bytes());
		Ok(())
	}
}

/// Single-point information with CP56Time2a time tag
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct MSpTb1 {
	/// Single-point with quality descriptor
	pub siq: Siq,
	/// Time tag
	pub time: Cp56Time2a,
}

impl FromBytes for MSpTb1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, Box<ParseError>> {
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
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), Box<ParseError>> {
		buffer.push(self.siq.to_byte());
		buffer.extend_from_slice(&self.time.to_bytes());
		Ok(())
	}
}

/// Double-point information with CP56Time2a time tag
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct MDpTb1 {
	/// Double point information with quality descriptor
	pub diq: Diq,
	/// Time tag
	pub time: Cp56Time2a,
}

impl FromBytes for MDpTb1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, Box<ParseError>> {
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
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), Box<ParseError>> {
		buffer.push(self.diq.to_byte());
		buffer.extend_from_slice(&self.time.to_bytes());
		Ok(())
	}
}
/// Step position information with CP56Time2a time tag
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct MStTb1 {
	/// Value with transient state indication
	pub vti: Vti,
	/// Time tag
	pub time: Cp56Time2a,
}

impl FromBytes for MStTb1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, Box<ParseError>> {
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
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), Box<ParseError>> {
		buffer.extend_from_slice(&self.vti.to_bytes());
		buffer.extend_from_slice(&self.time.to_bytes());
		Ok(())
	}
}

/// Bitstring of 32 bit with CP56Time2a time tag
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct MBoTb1 {
	/// Bit string of 32 bits
	pub bsi: Bsi,
	/// Quality descriptor
	pub qds: Qds,
	/// Time tag
	pub time: Cp56Time2a,
}

impl FromBytes for MBoTb1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, Box<ParseError>> {
		let bsi = Bsi::from_byte(*bytes.first_chunk::<4>().context(NotEnoughBytes)?);
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
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), Box<ParseError>> {
		buffer.extend_from_slice(&self.bsi.to_bytes());
		buffer.push(self.qds.to_byte());
		buffer.extend_from_slice(&self.time.to_bytes());
		Ok(())
	}
}

/// Measured value, normalized value with CP56Time2a time tag
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct MMeTd1 {
	/// Normalized value
	pub nva: Nva,
	/// Quality descriptor
	pub qds: Qds,
	/// Time tag
	pub time: Cp56Time2a,
}

impl FromBytes for MMeTd1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, Box<ParseError>> {
		let nva = Nva::from_bytes(*bytes.first_chunk::<2>().context(NotEnoughBytes)?);
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
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), Box<ParseError>> {
		buffer.extend_from_slice(&self.nva.to_bytes());
		buffer.push(self.qds.to_byte());
		buffer.extend_from_slice(&self.time.to_bytes());
		Ok(())
	}
}

/// Measured value, scaled value with CP56Time2a time tag
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct MMeTe1 {
	/// Scaled value
	pub sva: Sva,
	/// Quality descriptor
	pub qds: Qds,
	/// Time tag
	pub time: Cp56Time2a,
}

impl FromBytes for MMeTe1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, Box<ParseError>> {
		let sva = Sva::from_bytes(*bytes.first_chunk::<2>().context(NotEnoughBytes)?);
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
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), Box<ParseError>> {
		buffer.extend_from_slice(&self.sva.to_bytes());
		buffer.push(self.qds.to_byte());
		buffer.extend_from_slice(&self.time.to_bytes());
		Ok(())
	}
}

/// Measured value, short floating point number with CP56Time2a time tag
#[derive(Debug, Clone, PartialEq)]
pub struct MMeTf1 {
	/// Short floating point
	pub r32: R32,
	/// Quality descriptor
	pub qds: Qds,
	/// Time tag
	pub time: Cp56Time2a,
}

impl FromBytes for MMeTf1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, Box<ParseError>> {
		let r32 = R32::from_bytes(*bytes.first_chunk::<4>().context(NotEnoughBytes)?);
		let qds = Qds::from_byte(*bytes.get(4).context(NotEnoughBytes)?);
		let time = Cp56Time2a::from_bytes(
			bytes.get(5..12).context(NotEnoughBytes)?.try_into().context(SizedSlice)?,
		)
		.context(ParseTimeTag)?;
		Ok(Self { r32, qds, time })
	}
}

impl ToBytes for MMeTf1 {
	#[instrument]
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), Box<ParseError>> {
		buffer.extend_from_slice(&self.r32.to_bytes());
		buffer.push(self.qds.to_byte());
		buffer.extend_from_slice(&self.time.to_bytes());
		Ok(())
	}
}
/// Integrated totals with CP56Time2a time tag
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct MItTb1 {
	/// Binary counter reading
	pub bcr: Bcr,
	/// Quality descriptor
	pub qds: Qds,
	/// Time tag
	pub time: Cp56Time2a,
}

impl FromBytes for MItTb1 {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, Box<ParseError>> {
		let bcr = Bcr::from_byte(*bytes.first_chunk::<4>().context(NotEnoughBytes)?);
		let qds = Qds::from_byte(*bytes.get(4).context(NotEnoughBytes)?);
		let time = Cp56Time2a::from_bytes(
			bytes.get(5..12).context(NotEnoughBytes)?.try_into().context(SizedSlice)?,
		)
		.context(ParseTimeTag)?;
		Ok(Self { bcr, qds, time })
	}
}

impl ToBytes for MItTb1 {
	#[instrument]
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), Box<ParseError>> {
		buffer.extend_from_slice(&self.bcr.to_bytes());
		buffer.push(self.qds.to_byte());
		buffer.extend_from_slice(&self.time.to_bytes());
		Ok(())
	}
}

/// Event of protection equipment with CP56Time2a time tag
#[derive(Debug, Clone, Eq, PartialEq)]
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
	fn from_bytes(bytes: &[u8]) -> Result<Self, Box<ParseError>> {
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
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), Box<ParseError>> {
		buffer.push(self.sep.to_byte());
		buffer.extend_from_slice(&self.elapsed.to_bytes());
		buffer.extend_from_slice(&self.time.to_bytes());
		Ok(())
	}
}

/// Packed start events of protection equipment with CP56Time2a time tag
#[derive(Debug, Clone, Eq, PartialEq)]
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
	fn from_bytes(bytes: &[u8]) -> Result<Self, Box<ParseError>> {
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
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), Box<ParseError>> {
		buffer.push(self.start_ep.to_byte());
		buffer.push(self.qdp.to_byte());
		buffer.extend_from_slice(&self.relay_duration.to_bytes());
		buffer.extend_from_slice(&self.time.to_bytes());
		Ok(())
	}
}

/// Packed output circuit information of protection equipment with CP56Time2a
/// time tag
#[derive(Debug, Clone, Eq, PartialEq)]
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
	fn from_bytes(bytes: &[u8]) -> Result<Self, Box<ParseError>> {
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
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), Box<ParseError>> {
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
	fn from_bytes(bytes: &[u8]) -> Result<Self, Box<ParseError>> {
		let byte = *bytes.first().context(NotEnoughBytes)?;
		let lpc = Lpc::from_bool(byte & 0b1000_0000 != 0);
		let coi = Coi::from_byte(byte & 0b0111_1111);
		Ok(Self { lpc, coi })
	}
}

impl ToBytes for MEiNa1 {
	#[instrument]
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), Box<ParseError>> {
		let mut byte: u8 = 0;
		byte |= (self.lpc as u8) << 7;
		byte |= self.coi.to_byte();
		buffer.push(byte);
		Ok(())
	}
}
