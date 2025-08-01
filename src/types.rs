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
	fn from_bytes(bytes: &[u8]) -> Result<Self, Box<ParseError>>;
}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub), context(suffix(false)))]
pub enum ParseError {
	#[snafu(display("Time conversion error"))]
	ParseTimeTag {
		source: ParseTimeError,
		#[snafu(implicit)]
		context: SpanTraceWrapper,
	},
	#[snafu(display("Invalid type"))]
	InvalidType {
		#[snafu(implicit)]
		context: SpanTraceWrapper,
	},
	#[snafu(display("Not implemented yet"))]
	NotImplemented {
		#[snafu(implicit)]
		context: SpanTraceWrapper,
	},
	#[snafu(display("Not enough bytes"))]
	NotEnoughBytes {
		#[snafu(implicit)]
		context: SpanTraceWrapper,
	},
}

pub trait ToBytes {
	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), Box<ParseError>>;
}

#[derive(Debug, Clone, PartialEq)]
pub struct GenericObject<T: FromBytes + ToBytes> {
	pub address: u32,
	pub object: T,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InformationObject {
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

impl InformationObject {
	#[instrument]
	fn build_objects<T: FromBytes + ToBytes>(
		type_id: TypeId,
		sequence: bool,
		num_objs: u8,
		bytes: &[u8],
	) -> Result<Vec<GenericObject<T>>, Box<ParseError>> {
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
				return NotEnoughBytes.fail()?;
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
				.collect::<Result<Vec<_>, Box<ParseError>>>()?;
			objs.extend(other_objs);
			Ok::<_, Box<ParseError>>(objs)
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
				.collect::<Result<Vec<_>, Box<ParseError>>>()?)
		}
	}

	#[instrument(skip_all)]
	fn serialize_objects<T: FromBytes + ToBytes>(
		objects: &[GenericObject<T>],
		buffer: &mut Vec<u8>,
	) -> Result<(), Box<ParseError>> {
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
	) -> Result<Self, Box<ParseError>> {
		Ok(match type_id {
			TypeId::M_SP_NA_1 => InformationObject::MSpNa1(Self::build_objects::<MSpNa1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::M_SP_TA_1 => InformationObject::MSpTa1(Self::build_objects::<MSpTa1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::M_DP_NA_1 => InformationObject::MDpNa1(Self::build_objects::<MDpNa1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::M_DP_TA_1 => InformationObject::MDpTa1(Self::build_objects::<MDpTa1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::M_ST_NA_1 => InformationObject::MStNa1(Self::build_objects::<MStNa1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::M_ST_TA_1 => InformationObject::MStTa1(Self::build_objects::<MStTa1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::M_BO_NA_1 => InformationObject::MBoNa1(Self::build_objects::<MBoNa1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::M_ME_NA_1 => InformationObject::MMeNa1(Self::build_objects::<MMeNa1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::M_ME_TA_1 => InformationObject::MMeTa1(Self::build_objects::<MMeTa1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::M_ME_NB_1 => InformationObject::MMeNb1(Self::build_objects::<MMeNb1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::M_ME_TB_1 => InformationObject::MMeTb1(Self::build_objects::<MMeTb1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::M_ME_NC_1 => InformationObject::MMeNc1(Self::build_objects::<MMeNc1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::M_ME_TC_1 => InformationObject::MMeTc1(Self::build_objects::<MMeTc1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::M_IT_NA_1 => InformationObject::MItNa1(Self::build_objects::<MItNa1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::M_EP_TA_1 => InformationObject::MEpTa1(Self::build_objects::<MEpTa1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::M_EP_TB_1 => InformationObject::MEpTb1(Self::build_objects::<MEpTb1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::M_EP_TC_1 => InformationObject::MEpTc1(Self::build_objects::<MEpTc1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::M_PS_NA_1 => InformationObject::MPsNa1(Self::build_objects::<MPsNa1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::M_ME_ND_1 => InformationObject::MMeNd1(Self::build_objects::<MMeNd1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::M_SP_TB_1 => InformationObject::MSpTb1(Self::build_objects::<MSpTb1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::M_DP_TB_1 => InformationObject::MDpTb1(Self::build_objects::<MDpTb1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::M_ST_TB_1 => InformationObject::MStTb1(Self::build_objects::<MStTb1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::M_BO_TB_1 => InformationObject::MBoTb1(Self::build_objects::<MBoTb1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::M_ME_TD_1 => InformationObject::MMeTd1(Self::build_objects::<MMeTd1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::M_ME_TE_1 => InformationObject::MMeTe1(Self::build_objects::<MMeTe1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::M_ME_TF_1 => InformationObject::MMeTf1(Self::build_objects::<MMeTf1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::M_IT_TB_1 => InformationObject::MItTb1(Self::build_objects::<MItTb1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::M_EP_TD_1 => InformationObject::MEpTd1(Self::build_objects::<MEpTd1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::M_EP_TE_1 => InformationObject::MEpTe1(Self::build_objects::<MEpTe1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::M_EP_TF_1 => InformationObject::MEpTf1(Self::build_objects::<MEpTf1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::M_EI_NA_1 => InformationObject::MEiNa1(Self::build_objects::<MEiNa1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::C_SC_NA_1 => InformationObject::CScNa1(Self::build_objects::<CScNa1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::C_DC_NA_1 => InformationObject::CdcNa1(Self::build_objects::<CdcNa1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::C_RC_NA_1 => InformationObject::CrcNa1(Self::build_objects::<CrcNa1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::C_SE_NA_1 => InformationObject::CSeNa1(Self::build_objects::<CSeNa1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::C_SE_NB_1 => InformationObject::CSeNb1(Self::build_objects::<CSeNb1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::C_SE_NC_1 => InformationObject::CSeNc1(Self::build_objects::<CSeNc1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::C_BO_NA_1 => InformationObject::CBoNa1(Self::build_objects::<CBoNa1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::C_SC_TA_1 => InformationObject::CScTa1(Self::build_objects::<CScTa1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::C_DC_TA_1 => InformationObject::CdcTa1(Self::build_objects::<CdcTa1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::C_RC_TA_1 => InformationObject::CrcTa1(Self::build_objects::<CrcTa1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::C_SE_TA_1 => InformationObject::CSeTa1(Self::build_objects::<CSeTa1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::C_SE_TB_1 => InformationObject::CSeTb1(Self::build_objects::<CSeTb1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::C_SE_TC_1 => InformationObject::CSeTc1(Self::build_objects::<CSeTc1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::C_BO_TA_1 => InformationObject::CBoTa1(Self::build_objects::<CBoTa1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::C_IC_NA_1 => InformationObject::CIcNa1(Self::build_objects::<CIcNa1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::C_CI_NA_1 => InformationObject::CCiNa1(Self::build_objects::<CCiNa1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::C_RD_NA_1 => InformationObject::CRdNa1(Self::build_objects::<CRdNa1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::C_CS_NA_1 => InformationObject::CCsNa1(Self::build_objects::<CCsNa1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::C_TS_NA_1 => InformationObject::CTsNa1(Self::build_objects::<CTsNa1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::C_RP_NA_1 => InformationObject::CRpNa1(Self::build_objects::<CRpNa1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::C_CD_NA_1 => InformationObject::CCdNa1(Self::build_objects::<CCdNa1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::C_TS_TA_1 => InformationObject::CTsTa1(Self::build_objects::<CTsTa1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::P_ME_NA_1 => InformationObject::PMeNa1(Self::build_objects::<PMeNa1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::P_ME_NB_1 => InformationObject::PMeNb1(Self::build_objects::<PMeNb1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::P_ME_NC_1 => InformationObject::PMeNc1(Self::build_objects::<PMeNc1>(
				type_id, sequence, num_objs, bytes,
			)?),
			TypeId::P_AC_NA_1 => InformationObject::PAcNa1(Self::build_objects::<PAcNa1>(
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
			InformationObject::MSpNa1(objs) => objs.len(),
			InformationObject::MSpTa1(objs) => objs.len(),
			InformationObject::MDpNa1(objs) => objs.len(),
			InformationObject::MDpTa1(objs) => objs.len(),
			InformationObject::MStNa1(objs) => objs.len(),
			InformationObject::MStTa1(objs) => objs.len(),
			InformationObject::MBoNa1(objs) => objs.len(),
			InformationObject::MMeNa1(objs) => objs.len(),
			InformationObject::MMeTa1(objs) => objs.len(),
			InformationObject::MMeNb1(objs) => objs.len(),
			InformationObject::MMeTb1(objs) => objs.len(),
			InformationObject::MMeNc1(objs) => objs.len(),
			InformationObject::MMeTc1(objs) => objs.len(),
			InformationObject::MItNa1(objs) => objs.len(),
			InformationObject::MEpTa1(objs) => objs.len(),
			InformationObject::MEpTb1(objs) => objs.len(),
			InformationObject::MEpTc1(objs) => objs.len(),
			InformationObject::MPsNa1(objs) => objs.len(),
			InformationObject::MMeNd1(objs) => objs.len(),
			InformationObject::MSpTb1(objs) => objs.len(),
			InformationObject::MDpTb1(objs) => objs.len(),
			InformationObject::MStTb1(objs) => objs.len(),
			InformationObject::MBoTb1(objs) => objs.len(),
			InformationObject::MMeTd1(objs) => objs.len(),
			InformationObject::MMeTe1(objs) => objs.len(),
			InformationObject::MMeTf1(objs) => objs.len(),
			InformationObject::MItTb1(objs) => objs.len(),
			InformationObject::MEpTd1(objs) => objs.len(),
			InformationObject::MEpTe1(objs) => objs.len(),
			InformationObject::MEpTf1(objs) => objs.len(),
			InformationObject::MEiNa1(objs) => objs.len(),
			InformationObject::CScNa1(objs) => objs.len(),
			InformationObject::CdcNa1(objs) => objs.len(),
			InformationObject::CrcNa1(objs) => objs.len(),
			InformationObject::CSeNa1(objs) => objs.len(),
			InformationObject::CSeNb1(objs) => objs.len(),
			InformationObject::CSeNc1(objs) => objs.len(),
			InformationObject::CBoNa1(objs) => objs.len(),
			InformationObject::CScTa1(objs) => objs.len(),
			InformationObject::CdcTa1(objs) => objs.len(),
			InformationObject::CrcTa1(objs) => objs.len(),
			InformationObject::CSeTa1(objs) => objs.len(),
			InformationObject::CSeTb1(objs) => objs.len(),
			InformationObject::CSeTc1(objs) => objs.len(),
			InformationObject::CBoTa1(objs) => objs.len(),
			InformationObject::CIcNa1(objs) => objs.len(),
			InformationObject::CCiNa1(objs) => objs.len(),
			InformationObject::CRdNa1(objs) => objs.len(),
			InformationObject::CCsNa1(objs) => objs.len(),
			InformationObject::CTsNa1(objs) => objs.len(),
			InformationObject::CRpNa1(objs) => objs.len(),
			InformationObject::CCdNa1(objs) => objs.len(),
			InformationObject::CTsTa1(objs) => objs.len(),
			InformationObject::PMeNa1(objs) => objs.len(),
			InformationObject::PMeNb1(objs) => objs.len(),
			InformationObject::PMeNc1(objs) => objs.len(),
			InformationObject::PAcNa1(objs) => objs.len(),
		}
	}

	#[must_use]
	pub const fn is_empty(&self) -> bool {
		match self {
			InformationObject::MSpNa1(objs) => objs.is_empty(),
			InformationObject::MSpTa1(objs) => objs.is_empty(),
			InformationObject::MDpNa1(objs) => objs.is_empty(),
			InformationObject::MDpTa1(objs) => objs.is_empty(),
			InformationObject::MStNa1(objs) => objs.is_empty(),
			InformationObject::MStTa1(objs) => objs.is_empty(),
			InformationObject::MBoNa1(objs) => objs.is_empty(),
			InformationObject::MMeNa1(objs) => objs.is_empty(),
			InformationObject::MMeTa1(objs) => objs.is_empty(),
			InformationObject::MMeNb1(objs) => objs.is_empty(),
			InformationObject::MMeTb1(objs) => objs.is_empty(),
			InformationObject::MMeNc1(objs) => objs.is_empty(),
			InformationObject::MMeTc1(objs) => objs.is_empty(),
			InformationObject::MItNa1(objs) => objs.is_empty(),
			InformationObject::MEpTa1(objs) => objs.is_empty(),
			InformationObject::MEpTb1(objs) => objs.is_empty(),
			InformationObject::MEpTc1(objs) => objs.is_empty(),
			InformationObject::MPsNa1(objs) => objs.is_empty(),
			InformationObject::MMeNd1(objs) => objs.is_empty(),
			InformationObject::MSpTb1(objs) => objs.is_empty(),
			InformationObject::MDpTb1(objs) => objs.is_empty(),
			InformationObject::MStTb1(objs) => objs.is_empty(),
			InformationObject::MBoTb1(objs) => objs.is_empty(),
			InformationObject::MMeTd1(objs) => objs.is_empty(),
			InformationObject::MMeTe1(objs) => objs.is_empty(),
			InformationObject::MMeTf1(objs) => objs.is_empty(),
			InformationObject::MItTb1(objs) => objs.is_empty(),
			InformationObject::MEpTd1(objs) => objs.is_empty(),
			InformationObject::MEpTe1(objs) => objs.is_empty(),
			InformationObject::MEpTf1(objs) => objs.is_empty(),
			InformationObject::MEiNa1(objs) => objs.is_empty(),
			InformationObject::CScNa1(objs) => objs.is_empty(),
			InformationObject::CdcNa1(objs) => objs.is_empty(),
			InformationObject::CrcNa1(objs) => objs.is_empty(),
			InformationObject::CSeNa1(objs) => objs.is_empty(),
			InformationObject::CSeNb1(objs) => objs.is_empty(),
			InformationObject::CSeNc1(objs) => objs.is_empty(),
			InformationObject::CBoNa1(objs) => objs.is_empty(),
			InformationObject::CScTa1(objs) => objs.is_empty(),
			InformationObject::CdcTa1(objs) => objs.is_empty(),
			InformationObject::CrcTa1(objs) => objs.is_empty(),
			InformationObject::CSeTa1(objs) => objs.is_empty(),
			InformationObject::CSeTb1(objs) => objs.is_empty(),
			InformationObject::CSeTc1(objs) => objs.is_empty(),
			InformationObject::CBoTa1(objs) => objs.is_empty(),
			InformationObject::CIcNa1(objs) => objs.is_empty(),
			InformationObject::CCiNa1(objs) => objs.is_empty(),
			InformationObject::CRdNa1(objs) => objs.is_empty(),
			InformationObject::CCsNa1(objs) => objs.is_empty(),
			InformationObject::CTsNa1(objs) => objs.is_empty(),
			InformationObject::CRpNa1(objs) => objs.is_empty(),
			InformationObject::CCdNa1(objs) => objs.is_empty(),
			InformationObject::CTsTa1(objs) => objs.is_empty(),
			InformationObject::PMeNa1(objs) => objs.is_empty(),
			InformationObject::PMeNb1(objs) => objs.is_empty(),
			InformationObject::PMeNc1(objs) => objs.is_empty(),
			InformationObject::PAcNa1(objs) => objs.is_empty(),
		}
	}

	pub fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), Box<ParseError>> {
		match self {
			InformationObject::MSpNa1(objs) => Self::serialize_objects(objs, buffer),
			InformationObject::MSpTa1(objs) => Self::serialize_objects(objs, buffer),
			InformationObject::MDpNa1(objs) => Self::serialize_objects(objs, buffer),
			InformationObject::MDpTa1(objs) => Self::serialize_objects(objs, buffer),
			InformationObject::MStNa1(objs) => Self::serialize_objects(objs, buffer),
			InformationObject::MStTa1(objs) => Self::serialize_objects(objs, buffer),
			InformationObject::MBoNa1(objs) => Self::serialize_objects(objs, buffer),
			InformationObject::MMeNa1(objs) => Self::serialize_objects(objs, buffer),
			InformationObject::MMeTa1(objs) => Self::serialize_objects(objs, buffer),
			InformationObject::MMeNb1(objs) => Self::serialize_objects(objs, buffer),
			InformationObject::MMeTb1(objs) => Self::serialize_objects(objs, buffer),
			InformationObject::MMeNc1(objs) => Self::serialize_objects(objs, buffer),
			InformationObject::MMeTc1(objs) => Self::serialize_objects(objs, buffer),
			InformationObject::MItNa1(objs) => Self::serialize_objects(objs, buffer),
			InformationObject::MEpTa1(objs) => Self::serialize_objects(objs, buffer),
			InformationObject::MEpTb1(objs) => Self::serialize_objects(objs, buffer),
			InformationObject::MEpTc1(objs) => Self::serialize_objects(objs, buffer),
			InformationObject::MPsNa1(objs) => Self::serialize_objects(objs, buffer),
			InformationObject::MMeNd1(objs) => Self::serialize_objects(objs, buffer),
			InformationObject::MSpTb1(objs) => Self::serialize_objects(objs, buffer),
			InformationObject::MDpTb1(objs) => Self::serialize_objects(objs, buffer),
			InformationObject::MStTb1(objs) => Self::serialize_objects(objs, buffer),
			InformationObject::MBoTb1(objs) => Self::serialize_objects(objs, buffer),
			InformationObject::MMeTd1(objs) => Self::serialize_objects(objs, buffer),
			InformationObject::MMeTe1(objs) => Self::serialize_objects(objs, buffer),
			InformationObject::MMeTf1(objs) => Self::serialize_objects(objs, buffer),
			InformationObject::MItTb1(objs) => Self::serialize_objects(objs, buffer),
			InformationObject::MEpTd1(objs) => Self::serialize_objects(objs, buffer),
			InformationObject::MEpTe1(objs) => Self::serialize_objects(objs, buffer),
			InformationObject::MEpTf1(objs) => Self::serialize_objects(objs, buffer),
			InformationObject::MEiNa1(objs) => Self::serialize_objects(objs, buffer),
			InformationObject::CScNa1(objs) => Self::serialize_objects(objs, buffer),
			InformationObject::CdcNa1(objs) => Self::serialize_objects(objs, buffer),
			InformationObject::CrcNa1(objs) => Self::serialize_objects(objs, buffer),
			InformationObject::CSeNa1(objs) => Self::serialize_objects(objs, buffer),
			InformationObject::CSeNb1(objs) => Self::serialize_objects(objs, buffer),
			InformationObject::CSeNc1(objs) => Self::serialize_objects(objs, buffer),
			InformationObject::CBoNa1(objs) => Self::serialize_objects(objs, buffer),
			InformationObject::CScTa1(objs) => Self::serialize_objects(objs, buffer),
			InformationObject::CdcTa1(objs) => Self::serialize_objects(objs, buffer),
			InformationObject::CrcTa1(objs) => Self::serialize_objects(objs, buffer),
			InformationObject::CSeTa1(objs) => Self::serialize_objects(objs, buffer),
			InformationObject::CSeTb1(objs) => Self::serialize_objects(objs, buffer),
			InformationObject::CSeTc1(objs) => Self::serialize_objects(objs, buffer),
			InformationObject::CBoTa1(objs) => Self::serialize_objects(objs, buffer),
			InformationObject::CIcNa1(objs) => Self::serialize_objects(objs, buffer),
			InformationObject::CCiNa1(objs) => Self::serialize_objects(objs, buffer),
			InformationObject::CRdNa1(objs) => Self::serialize_objects(objs, buffer),
			InformationObject::CCsNa1(objs) => Self::serialize_objects(objs, buffer),
			InformationObject::CTsNa1(objs) => Self::serialize_objects(objs, buffer),
			InformationObject::CRpNa1(objs) => Self::serialize_objects(objs, buffer),
			InformationObject::CCdNa1(objs) => Self::serialize_objects(objs, buffer),
			InformationObject::CTsTa1(objs) => Self::serialize_objects(objs, buffer),
			InformationObject::PMeNa1(objs) => Self::serialize_objects(objs, buffer),
			InformationObject::PMeNb1(objs) => Self::serialize_objects(objs, buffer),
			InformationObject::PMeNc1(objs) => Self::serialize_objects(objs, buffer),
			InformationObject::PAcNa1(objs) => Self::serialize_objects(objs, buffer),
		}
	}
}
