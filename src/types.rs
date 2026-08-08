pub mod commands;
pub mod information_elements;
pub mod measurements;
pub mod parameters;
pub mod quality_descriptors;
pub mod time;

pub use commands::{
	CBoNa1, CBoTa1, CCdNa1, CCiNa1, CCsNa1, CIcNa1, CRdNa1, CRpNa1, CScNa1, CScTa1, CSeNa1, CSeNb1,
	CSeNc1, CSeTa1, CSeTb1, CSeTc1, CTsNa1, CTsTa1, CdcNa1, CdcTa1, CrcNa1, CrcTa1,
};
pub use measurements::{
	MBoNa1, MBoTb1, MDpNa1, MDpTa1, MDpTb1, MEiNa1, MEpTa1, MEpTb1, MEpTc1, MEpTd1, MEpTe1, MEpTf1,
	MItNa1, MItTb1, MMeNa1, MMeNb1, MMeNc1, MMeNd1, MMeTa1, MMeTb1, MMeTc1, MMeTd1, MMeTe1, MMeTf1,
	MPsNa1, MSpNa1, MSpTa1, MSpTb1, MStNa1, MStTa1, MStTb1,
};
pub use parameters::{PAcNa1, PMeNa1, PMeNb1, PMeNc1};
use snafu::Snafu;
use tracing::instrument;

use crate::{error::SpanTraceWrapper, types::time::ParseTimeError};

pub trait FromBytes: Sized {
	fn from_bytes(bytes: &[u8]) -> Result<Self, ParseError>;
}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub), context(suffix(false)))]
pub enum ParseError {
	#[snafu(display("Time conversion error"))]
	ParseTimeTag {
		source: ParseTimeError,
		#[snafu(implicit)]
		context: Box<SpanTraceWrapper>,
	},
	#[snafu(display("Invalid type"))]
	InvalidType {
		#[snafu(implicit)]
		context: Box<SpanTraceWrapper>,
	},
	#[snafu(display("Not implemented yet"))]
	NotImplemented {
		#[snafu(implicit)]
		context: Box<SpanTraceWrapper>,
	},
	#[snafu(display("Not enough bytes"))]
	NotEnoughBytes {
		#[snafu(implicit)]
		context: Box<SpanTraceWrapper>,
	},
	#[snafu(display("Failed to convert to sized slice"))]
	SizedSlice {
		source: std::array::TryFromSliceError,
		#[snafu(implicit)]
		context: Box<SpanTraceWrapper>,
	},
}

pub trait ToBytes {
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), ParseError>;
}

/// Raw object for custom ASDUs.
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct RawObject {
	pub raw: Vec<u8>,
}

impl FromBytes for RawObject {
	#[instrument]
	fn from_bytes(bytes: &[u8]) -> Result<Self, ParseError> {
		let raw = bytes.to_vec();
		Ok(Self { raw })
	}
}

impl ToBytes for RawObject {
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), ParseError> {
		buffer.extend_from_slice(&self.raw);
		Ok(())
	}
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct GenericObject<T: FromBytes + ToBytes + Default> {
	pub address: u32,
	pub object: T,
}

mod information_objects;

pub(crate) use information_objects::ADDRESS_SIZE;
pub use information_objects::InformationObjects;
