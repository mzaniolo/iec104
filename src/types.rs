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
use snafu::{OptionExt, Snafu};
use tracing::instrument;

use crate::{error::SpanTraceWrapper, types::time::ParseTimeError, types_id::TypeId};

const ADDRESS_SIZE: usize = 3;

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

#[derive(Debug, Clone, PartialEq, Default)]
pub struct GenericObject<T: FromBytes + ToBytes + Default> {
	pub address: u32,
	pub object: T,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InformationObjects {
	MSpNa1(Vec<GenericObject<MSpNa1>>),
	MSpTa1(Vec<GenericObject<MSpTa1>>),
	MDpNa1(Vec<GenericObject<MDpNa1>>),
	MDpTa1(Vec<GenericObject<MDpTa1>>),
	MStNa1(Vec<GenericObject<MStNa1>>),
	MStTa1(Vec<GenericObject<MStTa1>>),
	MBoNa1(Vec<GenericObject<MBoNa1>>),
	MMeNa1(Vec<GenericObject<MMeNa1>>),
	MMeTa1(Vec<GenericObject<MMeTa1>>),
	MMeNb1(Vec<GenericObject<MMeNb1>>),
	MMeTb1(Vec<GenericObject<MMeTb1>>),
	MMeNc1(Vec<GenericObject<MMeNc1>>),
	MMeTc1(Vec<GenericObject<MMeTc1>>),
	MItNa1(Vec<GenericObject<MItNa1>>),
	MEpTa1(Vec<GenericObject<MEpTa1>>),
	MEpTb1(Vec<GenericObject<MEpTb1>>),
	MEpTc1(Vec<GenericObject<MEpTc1>>),
	MPsNa1(Vec<GenericObject<MPsNa1>>),
	MMeNd1(Vec<GenericObject<MMeNd1>>),
	MSpTb1(Vec<GenericObject<MSpTb1>>),
	MDpTb1(Vec<GenericObject<MDpTb1>>),
	MStTb1(Vec<GenericObject<MStTb1>>),
	MBoTb1(Vec<GenericObject<MBoTb1>>),
	MMeTd1(Vec<GenericObject<MMeTd1>>),
	MMeTe1(Vec<GenericObject<MMeTe1>>),
	MMeTf1(Vec<GenericObject<MMeTf1>>),
	MItTb1(Vec<GenericObject<MItTb1>>),
	MEpTd1(Vec<GenericObject<MEpTd1>>),
	MEpTe1(Vec<GenericObject<MEpTe1>>),
	MEpTf1(Vec<GenericObject<MEpTf1>>),
	MEiNa1(Vec<GenericObject<MEiNa1>>),
	CScNa1(Vec<GenericObject<CScNa1>>),
	CdcNa1(Vec<GenericObject<CdcNa1>>),
	CrcNa1(Vec<GenericObject<CrcNa1>>),
	CSeNa1(Vec<GenericObject<CSeNa1>>),
	CSeNb1(Vec<GenericObject<CSeNb1>>),
	CSeNc1(Vec<GenericObject<CSeNc1>>),
	CBoNa1(Vec<GenericObject<CBoNa1>>),
	CScTa1(Vec<GenericObject<CScTa1>>),
	CdcTa1(Vec<GenericObject<CdcTa1>>),
	CrcTa1(Vec<GenericObject<CrcTa1>>),
	CSeTa1(Vec<GenericObject<CSeTa1>>),
	CSeTb1(Vec<GenericObject<CSeTb1>>),
	CSeTc1(Vec<GenericObject<CSeTc1>>),
	CBoTa1(Vec<GenericObject<CBoTa1>>),
	CIcNa1(Vec<GenericObject<CIcNa1>>),
	CCiNa1(Vec<GenericObject<CCiNa1>>),
	CRdNa1(Vec<GenericObject<CRdNa1>>),
	CCsNa1(Vec<GenericObject<CCsNa1>>),
	CTsNa1(Vec<GenericObject<CTsNa1>>),
	CRpNa1(Vec<GenericObject<CRpNa1>>),
	CCdNa1(Vec<GenericObject<CCdNa1>>),
	CTsTa1(Vec<GenericObject<CTsTa1>>),
	PMeNa1(Vec<GenericObject<PMeNa1>>),
	PMeNb1(Vec<GenericObject<PMeNb1>>),
	PMeNc1(Vec<GenericObject<PMeNc1>>),
	PAcNa1(Vec<GenericObject<PAcNa1>>),
}

impl InformationObjects {
	#[instrument]
	fn build_objects<T: FromBytes + ToBytes + Default>(
		type_id: TypeId,
		sequence: bool,
		num_objs: u8,
		bytes: &[u8],
	) -> Result<Vec<GenericObject<T>>, ParseError> {
		let object_size = type_id.size();
		tracing::trace!(
			"Building information objects. Object size: {object_size}. Bytes: {:?}",
			bytes
		);

		if sequence {
			let mut objs = Vec::<GenericObject<T>>::with_capacity(num_objs as usize);
			let (first_chunk, other_chunks) =
				bytes.split_at_checked(object_size + ADDRESS_SIZE).context(NotEnoughBytes)?;

			// This won't panic because we checked that the first chunk is at least
			// ADDRESS_SIZE bytes long
			let first_addr =
				u32::from_le_bytes([first_chunk[0], first_chunk[1], first_chunk[2], 0]);
			let first_obj = T::from_bytes(&first_chunk[ADDRESS_SIZE..])?;
			objs.push(GenericObject { address: first_addr, object: first_obj });
			let other_chunks = other_chunks.chunks_exact(object_size);
			// TODO: Do we really need to make sure of this here?
			if !other_chunks.remainder().is_empty() {
				return NotEnoughBytes.fail();
			}

			let other_objs = other_chunks
				.enumerate()
				.map(|(i, chunk)| {
					tracing::trace!("Building object: {:?}", chunk);
					// If it's a sequence we only get the address of the first object. So the first
					// object has object_size + 3 bytes for the address. The subsequent chunks only
					// have the object_size.
					// Since the i starts at 0, we need to add 1 to the address.
					let address = first_addr + (i as u32) + 1;
					let object = T::from_bytes(&chunk[ADDRESS_SIZE..])?;
					Ok(GenericObject { address, object })
				})
				.collect::<Result<Vec<_>, ParseError>>()?;
			objs.extend(other_objs);
			Ok::<_, ParseError>(objs)
		} else {
			// If it's not a sequence we get the address of each object in the first 3
			// bytes.
			Ok(bytes[0..]
				.chunks(object_size + 3)
				.map(|chunk| {
					tracing::trace!("Building object: {:?}", chunk);
					let address = u32::from_be_bytes([0, chunk[2], chunk[1], chunk[0]]);
					let object = T::from_bytes(&chunk[3..])?;
					Ok(GenericObject { address, object })
				})
				.collect::<Result<Vec<_>, ParseError>>()?)
		}
	}

	#[instrument(skip_all)]
	fn serialize_objects<T: FromBytes + ToBytes + Default>(
		objects: &[GenericObject<T>],
		buffer: &mut Vec<u8>,
	) -> Result<(), ParseError> {
		//TODO: Handle sequence

		for obj in objects {
			let address = obj.address.to_le_bytes();
			buffer.push(address[0]);
			buffer.push(address[1]);
			buffer.push(address[2]);
			obj.object.to_bytes(buffer)?;
		}

		Ok(())
	}

	#[allow(clippy::too_many_lines)]
	pub fn from_bytes(
		type_id: TypeId,
		sequence: bool,
		num_objs: u8,
		bytes: &[u8],
	) -> Result<Self, ParseError> {
		Ok(match type_id {
			TypeId::M_SP_NA_1 => InformationObjects::MSpNa1(Self::build_objects::<MSpNa1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::M_SP_TA_1 => InformationObjects::MSpTa1(Self::build_objects::<MSpTa1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::M_DP_NA_1 => InformationObjects::MDpNa1(Self::build_objects::<MDpNa1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::M_DP_TA_1 => InformationObjects::MDpTa1(Self::build_objects::<MDpTa1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::M_ST_NA_1 => InformationObjects::MStNa1(Self::build_objects::<MStNa1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::M_ST_TA_1 => InformationObjects::MStTa1(Self::build_objects::<MStTa1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::M_BO_NA_1 => InformationObjects::MBoNa1(Self::build_objects::<MBoNa1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::M_ME_NA_1 => InformationObjects::MMeNa1(Self::build_objects::<MMeNa1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::M_ME_TA_1 => InformationObjects::MMeTa1(Self::build_objects::<MMeTa1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::M_ME_NB_1 => InformationObjects::MMeNb1(Self::build_objects::<MMeNb1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::M_ME_TB_1 => InformationObjects::MMeTb1(Self::build_objects::<MMeTb1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::M_ME_NC_1 => InformationObjects::MMeNc1(Self::build_objects::<MMeNc1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::M_ME_TC_1 => InformationObjects::MMeTc1(Self::build_objects::<MMeTc1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::M_IT_NA_1 => InformationObjects::MItNa1(Self::build_objects::<MItNa1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::M_EP_TA_1 => InformationObjects::MEpTa1(Self::build_objects::<MEpTa1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::M_EP_TB_1 => InformationObjects::MEpTb1(Self::build_objects::<MEpTb1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::M_EP_TC_1 => InformationObjects::MEpTc1(Self::build_objects::<MEpTc1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::M_PS_NA_1 => InformationObjects::MPsNa1(Self::build_objects::<MPsNa1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::M_ME_ND_1 => InformationObjects::MMeNd1(Self::build_objects::<MMeNd1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::M_SP_TB_1 => InformationObjects::MSpTb1(Self::build_objects::<MSpTb1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::M_DP_TB_1 => InformationObjects::MDpTb1(Self::build_objects::<MDpTb1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::M_ST_TB_1 => InformationObjects::MStTb1(Self::build_objects::<MStTb1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::M_BO_TB_1 => InformationObjects::MBoTb1(Self::build_objects::<MBoTb1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::M_ME_TD_1 => InformationObjects::MMeTd1(Self::build_objects::<MMeTd1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::M_ME_TE_1 => InformationObjects::MMeTe1(Self::build_objects::<MMeTe1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::M_ME_TF_1 => InformationObjects::MMeTf1(Self::build_objects::<MMeTf1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::M_IT_TB_1 => InformationObjects::MItTb1(Self::build_objects::<MItTb1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::M_EP_TD_1 => InformationObjects::MEpTd1(Self::build_objects::<MEpTd1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::M_EP_TE_1 => InformationObjects::MEpTe1(Self::build_objects::<MEpTe1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::M_EP_TF_1 => InformationObjects::MEpTf1(Self::build_objects::<MEpTf1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::M_EI_NA_1 => InformationObjects::MEiNa1(Self::build_objects::<MEiNa1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::C_SC_NA_1 => InformationObjects::CScNa1(Self::build_objects::<CScNa1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::C_DC_NA_1 => InformationObjects::CdcNa1(Self::build_objects::<CdcNa1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::C_RC_NA_1 => InformationObjects::CrcNa1(Self::build_objects::<CrcNa1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::C_SE_NA_1 => InformationObjects::CSeNa1(Self::build_objects::<CSeNa1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::C_SE_NB_1 => InformationObjects::CSeNb1(Self::build_objects::<CSeNb1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::C_SE_NC_1 => InformationObjects::CSeNc1(Self::build_objects::<CSeNc1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::C_BO_NA_1 => InformationObjects::CBoNa1(Self::build_objects::<CBoNa1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::C_SC_TA_1 => InformationObjects::CScTa1(Self::build_objects::<CScTa1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::C_DC_TA_1 => InformationObjects::CdcTa1(Self::build_objects::<CdcTa1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::C_RC_TA_1 => InformationObjects::CrcTa1(Self::build_objects::<CrcTa1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::C_SE_TA_1 => InformationObjects::CSeTa1(Self::build_objects::<CSeTa1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::C_SE_TB_1 => InformationObjects::CSeTb1(Self::build_objects::<CSeTb1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::C_SE_TC_1 => InformationObjects::CSeTc1(Self::build_objects::<CSeTc1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::C_BO_TA_1 => InformationObjects::CBoTa1(Self::build_objects::<CBoTa1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::C_IC_NA_1 => InformationObjects::CIcNa1(Self::build_objects::<CIcNa1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::C_CI_NA_1 => InformationObjects::CCiNa1(Self::build_objects::<CCiNa1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::C_RD_NA_1 => InformationObjects::CRdNa1(Self::build_objects::<CRdNa1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::C_CS_NA_1 => InformationObjects::CCsNa1(Self::build_objects::<CCsNa1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::C_TS_NA_1 => InformationObjects::CTsNa1(Self::build_objects::<CTsNa1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::C_RP_NA_1 => InformationObjects::CRpNa1(Self::build_objects::<CRpNa1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::C_CD_NA_1 => InformationObjects::CCdNa1(Self::build_objects::<CCdNa1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::C_TS_TA_1 => InformationObjects::CTsTa1(Self::build_objects::<CTsTa1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::P_ME_NA_1 => InformationObjects::PMeNa1(Self::build_objects::<PMeNa1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::P_ME_NB_1 => InformationObjects::PMeNb1(Self::build_objects::<PMeNb1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::P_ME_NC_1 => InformationObjects::PMeNc1(Self::build_objects::<PMeNc1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::P_AC_NA_1 => InformationObjects::PAcNa1(Self::build_objects::<PAcNa1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::F_FR_NA_1
			| TypeId::F_SR_NA_1
			| TypeId::F_SC_NA_1
			| TypeId::F_LS_NA_1
			| TypeId::F_FA_NA_1
			| TypeId::F_SG_NA_1
			| TypeId::F_DR_TA_1 => NotImplemented.fail()?,
			TypeId::Invalid => InvalidType.fail()?,
		})
	}

	#[must_use]
	pub const fn len(&self) -> usize {
		match self {
			InformationObjects::MSpNa1(objs) => objs.len(),
			InformationObjects::MSpTa1(objs) => objs.len(),
			InformationObjects::MDpNa1(objs) => objs.len(),
			InformationObjects::MDpTa1(objs) => objs.len(),
			InformationObjects::MStNa1(objs) => objs.len(),
			InformationObjects::MStTa1(objs) => objs.len(),
			InformationObjects::MBoNa1(objs) => objs.len(),
			InformationObjects::MMeNa1(objs) => objs.len(),
			InformationObjects::MMeTa1(objs) => objs.len(),
			InformationObjects::MMeNb1(objs) => objs.len(),
			InformationObjects::MMeTb1(objs) => objs.len(),
			InformationObjects::MMeNc1(objs) => objs.len(),
			InformationObjects::MMeTc1(objs) => objs.len(),
			InformationObjects::MItNa1(objs) => objs.len(),
			InformationObjects::MEpTa1(objs) => objs.len(),
			InformationObjects::MEpTb1(objs) => objs.len(),
			InformationObjects::MEpTc1(objs) => objs.len(),
			InformationObjects::MPsNa1(objs) => objs.len(),
			InformationObjects::MMeNd1(objs) => objs.len(),
			InformationObjects::MSpTb1(objs) => objs.len(),
			InformationObjects::MDpTb1(objs) => objs.len(),
			InformationObjects::MStTb1(objs) => objs.len(),
			InformationObjects::MBoTb1(objs) => objs.len(),
			InformationObjects::MMeTd1(objs) => objs.len(),
			InformationObjects::MMeTe1(objs) => objs.len(),
			InformationObjects::MMeTf1(objs) => objs.len(),
			InformationObjects::MItTb1(objs) => objs.len(),
			InformationObjects::MEpTd1(objs) => objs.len(),
			InformationObjects::MEpTe1(objs) => objs.len(),
			InformationObjects::MEpTf1(objs) => objs.len(),
			InformationObjects::MEiNa1(objs) => objs.len(),
			InformationObjects::CScNa1(objs) => objs.len(),
			InformationObjects::CdcNa1(objs) => objs.len(),
			InformationObjects::CrcNa1(objs) => objs.len(),
			InformationObjects::CSeNa1(objs) => objs.len(),
			InformationObjects::CSeNb1(objs) => objs.len(),
			InformationObjects::CSeNc1(objs) => objs.len(),
			InformationObjects::CBoNa1(objs) => objs.len(),
			InformationObjects::CScTa1(objs) => objs.len(),
			InformationObjects::CdcTa1(objs) => objs.len(),
			InformationObjects::CrcTa1(objs) => objs.len(),
			InformationObjects::CSeTa1(objs) => objs.len(),
			InformationObjects::CSeTb1(objs) => objs.len(),
			InformationObjects::CSeTc1(objs) => objs.len(),
			InformationObjects::CBoTa1(objs) => objs.len(),
			InformationObjects::CIcNa1(objs) => objs.len(),
			InformationObjects::CCiNa1(objs) => objs.len(),
			InformationObjects::CRdNa1(objs) => objs.len(),
			InformationObjects::CCsNa1(objs) => objs.len(),
			InformationObjects::CTsNa1(objs) => objs.len(),
			InformationObjects::CRpNa1(objs) => objs.len(),
			InformationObjects::CCdNa1(objs) => objs.len(),
			InformationObjects::CTsTa1(objs) => objs.len(),
			InformationObjects::PMeNa1(objs) => objs.len(),
			InformationObjects::PMeNb1(objs) => objs.len(),
			InformationObjects::PMeNc1(objs) => objs.len(),
			InformationObjects::PAcNa1(objs) => objs.len(),
		}
	}

	#[must_use]
	pub const fn is_empty(&self) -> bool {
		match self {
			InformationObjects::MSpNa1(objs) => objs.is_empty(),
			InformationObjects::MSpTa1(objs) => objs.is_empty(),
			InformationObjects::MDpNa1(objs) => objs.is_empty(),
			InformationObjects::MDpTa1(objs) => objs.is_empty(),
			InformationObjects::MStNa1(objs) => objs.is_empty(),
			InformationObjects::MStTa1(objs) => objs.is_empty(),
			InformationObjects::MBoNa1(objs) => objs.is_empty(),
			InformationObjects::MMeNa1(objs) => objs.is_empty(),
			InformationObjects::MMeTa1(objs) => objs.is_empty(),
			InformationObjects::MMeNb1(objs) => objs.is_empty(),
			InformationObjects::MMeTb1(objs) => objs.is_empty(),
			InformationObjects::MMeNc1(objs) => objs.is_empty(),
			InformationObjects::MMeTc1(objs) => objs.is_empty(),
			InformationObjects::MItNa1(objs) => objs.is_empty(),
			InformationObjects::MEpTa1(objs) => objs.is_empty(),
			InformationObjects::MEpTb1(objs) => objs.is_empty(),
			InformationObjects::MEpTc1(objs) => objs.is_empty(),
			InformationObjects::MPsNa1(objs) => objs.is_empty(),
			InformationObjects::MMeNd1(objs) => objs.is_empty(),
			InformationObjects::MSpTb1(objs) => objs.is_empty(),
			InformationObjects::MDpTb1(objs) => objs.is_empty(),
			InformationObjects::MStTb1(objs) => objs.is_empty(),
			InformationObjects::MBoTb1(objs) => objs.is_empty(),
			InformationObjects::MMeTd1(objs) => objs.is_empty(),
			InformationObjects::MMeTe1(objs) => objs.is_empty(),
			InformationObjects::MMeTf1(objs) => objs.is_empty(),
			InformationObjects::MItTb1(objs) => objs.is_empty(),
			InformationObjects::MEpTd1(objs) => objs.is_empty(),
			InformationObjects::MEpTe1(objs) => objs.is_empty(),
			InformationObjects::MEpTf1(objs) => objs.is_empty(),
			InformationObjects::MEiNa1(objs) => objs.is_empty(),
			InformationObjects::CScNa1(objs) => objs.is_empty(),
			InformationObjects::CdcNa1(objs) => objs.is_empty(),
			InformationObjects::CrcNa1(objs) => objs.is_empty(),
			InformationObjects::CSeNa1(objs) => objs.is_empty(),
			InformationObjects::CSeNb1(objs) => objs.is_empty(),
			InformationObjects::CSeNc1(objs) => objs.is_empty(),
			InformationObjects::CBoNa1(objs) => objs.is_empty(),
			InformationObjects::CScTa1(objs) => objs.is_empty(),
			InformationObjects::CdcTa1(objs) => objs.is_empty(),
			InformationObjects::CrcTa1(objs) => objs.is_empty(),
			InformationObjects::CSeTa1(objs) => objs.is_empty(),
			InformationObjects::CSeTb1(objs) => objs.is_empty(),
			InformationObjects::CSeTc1(objs) => objs.is_empty(),
			InformationObjects::CBoTa1(objs) => objs.is_empty(),
			InformationObjects::CIcNa1(objs) => objs.is_empty(),
			InformationObjects::CCiNa1(objs) => objs.is_empty(),
			InformationObjects::CRdNa1(objs) => objs.is_empty(),
			InformationObjects::CCsNa1(objs) => objs.is_empty(),
			InformationObjects::CTsNa1(objs) => objs.is_empty(),
			InformationObjects::CRpNa1(objs) => objs.is_empty(),
			InformationObjects::CCdNa1(objs) => objs.is_empty(),
			InformationObjects::CTsTa1(objs) => objs.is_empty(),
			InformationObjects::PMeNa1(objs) => objs.is_empty(),
			InformationObjects::PMeNb1(objs) => objs.is_empty(),
			InformationObjects::PMeNc1(objs) => objs.is_empty(),
			InformationObjects::PAcNa1(objs) => objs.is_empty(),
		}
	}

	pub fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), ParseError> {
		match self {
			InformationObjects::MSpNa1(objs) => Self::serialize_objects(objs, buffer),
			InformationObjects::MSpTa1(objs) => Self::serialize_objects(objs, buffer),
			InformationObjects::MDpNa1(objs) => Self::serialize_objects(objs, buffer),
			InformationObjects::MDpTa1(objs) => Self::serialize_objects(objs, buffer),
			InformationObjects::MStNa1(objs) => Self::serialize_objects(objs, buffer),
			InformationObjects::MStTa1(objs) => Self::serialize_objects(objs, buffer),
			InformationObjects::MBoNa1(objs) => Self::serialize_objects(objs, buffer),
			InformationObjects::MMeNa1(objs) => Self::serialize_objects(objs, buffer),
			InformationObjects::MMeTa1(objs) => Self::serialize_objects(objs, buffer),
			InformationObjects::MMeNb1(objs) => Self::serialize_objects(objs, buffer),
			InformationObjects::MMeTb1(objs) => Self::serialize_objects(objs, buffer),
			InformationObjects::MMeNc1(objs) => Self::serialize_objects(objs, buffer),
			InformationObjects::MMeTc1(objs) => Self::serialize_objects(objs, buffer),
			InformationObjects::MItNa1(objs) => Self::serialize_objects(objs, buffer),
			InformationObjects::MEpTa1(objs) => Self::serialize_objects(objs, buffer),
			InformationObjects::MEpTb1(objs) => Self::serialize_objects(objs, buffer),
			InformationObjects::MEpTc1(objs) => Self::serialize_objects(objs, buffer),
			InformationObjects::MPsNa1(objs) => Self::serialize_objects(objs, buffer),
			InformationObjects::MMeNd1(objs) => Self::serialize_objects(objs, buffer),
			InformationObjects::MSpTb1(objs) => Self::serialize_objects(objs, buffer),
			InformationObjects::MDpTb1(objs) => Self::serialize_objects(objs, buffer),
			InformationObjects::MStTb1(objs) => Self::serialize_objects(objs, buffer),
			InformationObjects::MBoTb1(objs) => Self::serialize_objects(objs, buffer),
			InformationObjects::MMeTd1(objs) => Self::serialize_objects(objs, buffer),
			InformationObjects::MMeTe1(objs) => Self::serialize_objects(objs, buffer),
			InformationObjects::MMeTf1(objs) => Self::serialize_objects(objs, buffer),
			InformationObjects::MItTb1(objs) => Self::serialize_objects(objs, buffer),
			InformationObjects::MEpTd1(objs) => Self::serialize_objects(objs, buffer),
			InformationObjects::MEpTe1(objs) => Self::serialize_objects(objs, buffer),
			InformationObjects::MEpTf1(objs) => Self::serialize_objects(objs, buffer),
			InformationObjects::MEiNa1(objs) => Self::serialize_objects(objs, buffer),
			InformationObjects::CScNa1(objs) => Self::serialize_objects(objs, buffer),
			InformationObjects::CdcNa1(objs) => Self::serialize_objects(objs, buffer),
			InformationObjects::CrcNa1(objs) => Self::serialize_objects(objs, buffer),
			InformationObjects::CSeNa1(objs) => Self::serialize_objects(objs, buffer),
			InformationObjects::CSeNb1(objs) => Self::serialize_objects(objs, buffer),
			InformationObjects::CSeNc1(objs) => Self::serialize_objects(objs, buffer),
			InformationObjects::CBoNa1(objs) => Self::serialize_objects(objs, buffer),
			InformationObjects::CScTa1(objs) => Self::serialize_objects(objs, buffer),
			InformationObjects::CdcTa1(objs) => Self::serialize_objects(objs, buffer),
			InformationObjects::CrcTa1(objs) => Self::serialize_objects(objs, buffer),
			InformationObjects::CSeTa1(objs) => Self::serialize_objects(objs, buffer),
			InformationObjects::CSeTb1(objs) => Self::serialize_objects(objs, buffer),
			InformationObjects::CSeTc1(objs) => Self::serialize_objects(objs, buffer),
			InformationObjects::CBoTa1(objs) => Self::serialize_objects(objs, buffer),
			InformationObjects::CIcNa1(objs) => Self::serialize_objects(objs, buffer),
			InformationObjects::CCiNa1(objs) => Self::serialize_objects(objs, buffer),
			InformationObjects::CRdNa1(objs) => Self::serialize_objects(objs, buffer),
			InformationObjects::CCsNa1(objs) => Self::serialize_objects(objs, buffer),
			InformationObjects::CTsNa1(objs) => Self::serialize_objects(objs, buffer),
			InformationObjects::CRpNa1(objs) => Self::serialize_objects(objs, buffer),
			InformationObjects::CCdNa1(objs) => Self::serialize_objects(objs, buffer),
			InformationObjects::CTsTa1(objs) => Self::serialize_objects(objs, buffer),
			InformationObjects::PMeNa1(objs) => Self::serialize_objects(objs, buffer),
			InformationObjects::PMeNb1(objs) => Self::serialize_objects(objs, buffer),
			InformationObjects::PMeNc1(objs) => Self::serialize_objects(objs, buffer),
			InformationObjects::PAcNa1(objs) => Self::serialize_objects(objs, buffer),
		}
	}
}
