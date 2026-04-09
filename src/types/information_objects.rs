//! [`InformationObjects`]: `TypeId` ↔ wire-type mapping for parsed ASDU payloads.
//!
//! RTU-specific [`crate::rtu_server::PointValue`] lives in the RTU layer; keep
//! its `TypeId` / payload pairs aligned with the matching rows here when you
//! extend either side.

use snafu::OptionExt;
use tracing::instrument;

use super::{
	CBoNa1, CBoTa1, CCdNa1, CCiNa1, CCsNa1, CIcNa1, CRdNa1, CRpNa1, CScNa1, CScTa1, CSeNa1, CSeNb1,
	CSeNc1, CSeTa1, CSeTb1, CSeTc1, CTsNa1, CTsTa1, CdcNa1, CdcTa1, CrcNa1, CrcTa1, FromBytes,
	GenericObject, MBoNa1, MBoTb1, MDpNa1, MDpTa1, MDpTb1, MEiNa1, MEpTa1, MEpTb1, MEpTc1, MEpTd1,
	MEpTe1, MEpTf1, MItNa1, MItTb1, MMeNa1, MMeNb1, MMeNc1, MMeNd1, MMeTa1, MMeTb1, MMeTc1, MMeTd1,
	MMeTe1, MMeTf1, MPsNa1, MSpNa1, MSpTa1, MSpTb1, MStNa1, MStTa1, MStTb1, NotEnoughBytes,
	NotImplemented, PAcNa1, PMeNa1, PMeNb1, PMeNc1, ParseError, RawObject, ToBytes,
};
use crate::types_id::TypeId;

const ADDRESS_SIZE: usize = 3;

macro_rules! define_information_objects {
	(
		$(
			$type_id:ident => $variant:ident($payload:ty) ;
		)*
	) => {
		#[derive(Debug, Clone, PartialEq)]
		pub enum InformationObjects {
			$( $variant(Vec<GenericObject<$payload>>), )*
			Raw(Vec<GenericObject<RawObject>>),
		}

		impl InformationObjects {
			#[instrument]
			fn build_objects<T: FromBytes + ToBytes + Default>(
				type_id: TypeId,
				sequence: bool,
				num_objs: u8,
				bytes: &[u8],
			) -> Result<Vec<GenericObject<T>>, ParseError> {
				if !type_id.is_standard() {
					tracing::trace!("Building RAW information objects. Bytes: {:?}", bytes);
					if bytes.len() < 3 {
						return NotEnoughBytes.fail();
					}
					let address = u32::from_be_bytes([0, bytes[2], bytes[1], bytes[0]]);
					let object = T::from_bytes(&bytes[3..])?;
					return Ok(vec![GenericObject { address, object }]);
				}
				let object_size = type_id.size();
				tracing::trace!(
					"Building information objects. Object size: {object_size}. Bytes: {:?}",
					bytes
				);

				if sequence {
					let mut objs = Vec::<GenericObject<T>>::with_capacity(num_objs as usize);
					let (first_chunk, other_chunks) = bytes
						.split_at_checked(object_size + ADDRESS_SIZE)
						.context(NotEnoughBytes)?;

					let first_addr =
						u32::from_le_bytes([first_chunk[0], first_chunk[1], first_chunk[2], 0]);
					let first_obj = T::from_bytes(&first_chunk[ADDRESS_SIZE..])?;
					objs.push(GenericObject { address: first_addr, object: first_obj });
					let other_chunks = other_chunks.chunks_exact(object_size);
					if !other_chunks.remainder().is_empty() {
						return NotEnoughBytes.fail();
					}

					let other_objs = other_chunks
						.enumerate()
						.map(|(i, chunk)| {
							tracing::trace!("Building object: {:?}", chunk);
							let address = first_addr + (i as u32) + 1;
							let object = T::from_bytes(chunk)?;
							Ok(GenericObject { address, object })
						})
						.collect::<Result<Vec<_>, ParseError>>()?;
					objs.extend(other_objs);
					Ok::<_, ParseError>(objs)
				} else {
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
			#[instrument(skip_all)]
			pub fn from_bytes(
				type_id: TypeId,
				sequence: bool,
				num_objs: u8,
				bytes: &[u8],
			) -> Result<Self, ParseError> {
				Ok(match type_id {
					$(
						TypeId::$type_id => Self::$variant(Self::build_objects::<$payload>(
							type_id, sequence, num_objs, bytes,
						)?),
					)*
					TypeId::F_FR_NA_1
					| TypeId::F_SR_NA_1
					| TypeId::F_SC_NA_1
					| TypeId::F_LS_NA_1
					| TypeId::F_FA_NA_1
					| TypeId::F_SG_NA_1
					| TypeId::F_DR_TA_1 => NotImplemented.fail()?,
					_ => Self::Raw(Self::build_objects::<RawObject>(
						type_id, sequence, num_objs, bytes,
					)?),
				})
			}

			#[must_use]
			pub const fn len(&self) -> usize {
				match self {
					$( Self::$variant(objs) => objs.len(), )*
					Self::Raw(objs) => objs.len(),
				}
			}

			#[must_use]
			pub const fn is_empty(&self) -> bool {
				match self {
					$( Self::$variant(objs) => objs.is_empty(), )*
					Self::Raw(objs) => objs.is_empty(),
				}
			}

			pub fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), ParseError> {
				match self {
					$( Self::$variant(objs) => Self::serialize_objects(objs, buffer), )*
					Self::Raw(objs) => Self::serialize_objects(objs, buffer),
				}
			}
		}

		$(
			impl From<Vec<GenericObject<$payload>>> for InformationObjects {
				fn from(objs: Vec<GenericObject<$payload>>) -> Self {
					Self::$variant(objs)
				}
			}
		)*

		impl From<Vec<GenericObject<RawObject>>> for InformationObjects {
			fn from(objs: Vec<GenericObject<RawObject>>) -> Self {
				Self::Raw(objs)
			}
		}
	};
}

define_information_objects! {
	M_SP_NA_1 => MSpNa1(MSpNa1) ;
	M_SP_TA_1 => MSpTa1(MSpTa1) ;
	M_DP_NA_1 => MDpNa1(MDpNa1) ;
	M_DP_TA_1 => MDpTa1(MDpTa1) ;
	M_ST_NA_1 => MStNa1(MStNa1) ;
	M_ST_TA_1 => MStTa1(MStTa1) ;
	M_BO_NA_1 => MBoNa1(MBoNa1) ;
	M_ME_NA_1 => MMeNa1(MMeNa1) ;
	M_ME_TA_1 => MMeTa1(MMeTa1) ;
	M_ME_NB_1 => MMeNb1(MMeNb1) ;
	M_ME_TB_1 => MMeTb1(MMeTb1) ;
	M_ME_NC_1 => MMeNc1(MMeNc1) ;
	M_ME_TC_1 => MMeTc1(MMeTc1) ;
	M_IT_NA_1 => MItNa1(MItNa1) ;
	M_EP_TA_1 => MEpTa1(MEpTa1) ;
	M_EP_TB_1 => MEpTb1(MEpTb1) ;
	M_EP_TC_1 => MEpTc1(MEpTc1) ;
	M_PS_NA_1 => MPsNa1(MPsNa1) ;
	M_ME_ND_1 => MMeNd1(MMeNd1) ;
	M_SP_TB_1 => MSpTb1(MSpTb1) ;
	M_DP_TB_1 => MDpTb1(MDpTb1) ;
	M_ST_TB_1 => MStTb1(MStTb1) ;
	M_BO_TB_1 => MBoTb1(MBoTb1) ;
	M_ME_TD_1 => MMeTd1(MMeTd1) ;
	M_ME_TE_1 => MMeTe1(MMeTe1) ;
	M_ME_TF_1 => MMeTf1(MMeTf1) ;
	M_IT_TB_1 => MItTb1(MItTb1) ;
	M_EP_TD_1 => MEpTd1(MEpTd1) ;
	M_EP_TE_1 => MEpTe1(MEpTe1) ;
	M_EP_TF_1 => MEpTf1(MEpTf1) ;
	M_EI_NA_1 => MEiNa1(MEiNa1) ;
	C_SC_NA_1 => CScNa1(CScNa1) ;
	C_DC_NA_1 => CdcNa1(CdcNa1) ;
	C_RC_NA_1 => CrcNa1(CrcNa1) ;
	C_SE_NA_1 => CSeNa1(CSeNa1) ;
	C_SE_NB_1 => CSeNb1(CSeNb1) ;
	C_SE_NC_1 => CSeNc1(CSeNc1) ;
	C_BO_NA_1 => CBoNa1(CBoNa1) ;
	C_SC_TA_1 => CScTa1(CScTa1) ;
	C_DC_TA_1 => CdcTa1(CdcTa1) ;
	C_RC_TA_1 => CrcTa1(CrcTa1) ;
	C_SE_TA_1 => CSeTa1(CSeTa1) ;
	C_SE_TB_1 => CSeTb1(CSeTb1) ;
	C_SE_TC_1 => CSeTc1(CSeTc1) ;
	C_BO_TA_1 => CBoTa1(CBoTa1) ;
	C_IC_NA_1 => CIcNa1(CIcNa1) ;
	C_CI_NA_1 => CCiNa1(CCiNa1) ;
	C_RD_NA_1 => CRdNa1(CRdNa1) ;
	C_CS_NA_1 => CCsNa1(CCsNa1) ;
	C_TS_NA_1 => CTsNa1(CTsNa1) ;
	C_RP_NA_1 => CRpNa1(CRpNa1) ;
	C_CD_NA_1 => CCdNa1(CCdNa1) ;
	C_TS_TA_1 => CTsTa1(CTsTa1) ;
	P_ME_NA_1 => PMeNa1(PMeNa1) ;
	P_ME_NB_1 => PMeNb1(PMeNb1) ;
	P_ME_NC_1 => PMeNc1(PMeNc1) ;
	P_AC_NA_1 => PAcNa1(PAcNa1) ;
}
