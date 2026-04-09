//! RTU in-memory point value: NA monitoring payloads the simulator stores per
//! IOA.
//!
//! This is separate from [`crate::types::InformationObjects`]; rows here must
//! stay aligned with the corresponding `TypeId` / wire types in the core layer.

use crate::{
	types::{MBoNa1, MDpNa1, MMeNa1, MMeNb1, MMeNc1, MSpNa1, MStNa1},
	types_id::TypeId,
};

macro_rules! define_rtu_point_value {
	(
		$(
			$type_id:ident => $variant:ident($payload:ty) ;
		)*
	) => {
		#[derive(Debug, Clone, PartialEq)]
		pub enum PointValue {
			$( $variant($payload), )*
		}

		impl PointValue {
			#[must_use]
			pub const fn type_id(&self) -> TypeId {
				match self {
					$( Self::$variant(_) => TypeId::$type_id, )*
				}
			}
		}

		#[cfg(test)]
		mod type_id_tests {
			use crate::types_id::TypeId;

			use super::PointValue;

			#[test]
			fn point_value_type_ids_match_mapping() {
				$(
					assert_eq!(
						PointValue::$variant(Default::default()).type_id(),
						TypeId::$type_id,
					);
				)*
			}
		}
	};
}

define_rtu_point_value! {
	M_SP_NA_1 => MSpNa1(MSpNa1) ;
	M_DP_NA_1 => MDpNa1(MDpNa1) ;
	M_ST_NA_1 => MStNa1(MStNa1) ;
	M_BO_NA_1 => MBoNa1(MBoNa1) ;
	M_ME_NA_1 => MMeNa1(MMeNa1) ;
	M_ME_NB_1 => MMeNb1(MMeNb1) ;
	M_ME_NC_1 => MMeNc1(MMeNc1) ;
}
