use std::fmt;

use crate::{
	types::{MBoNa1, MDpNa1, MMeNa1, MMeNb1, MMeNc1, MSpNa1, MStNa1},
	types_id::TypeId,
};

/// ASDU common address and information object address (IOA) for one logical
/// point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PointAddress {
	/// Station / ASDU address field (common address).
	pub common_address: u16,
	/// Information object address within the station.
	pub information_object_address: u32,
}

impl PointAddress {
	/// Builds a key for the in-memory model.
	#[must_use]
	pub const fn new(common_address: u16, information_object_address: u32) -> Self {
		Self { common_address, information_object_address }
	}
}

impl fmt::Display for PointAddress {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "CA {} IOA {}", self.common_address, self.information_object_address)
	}
}

/// In-memory value for one monitored point.
#[derive(Debug, Clone, PartialEq)]
pub enum PointValue {
	/// Single-point information without time tag (`M_SP_NA_1`).
	MSpNa1(MSpNa1),
	/// Double-point information (`M_DP_NA_1`) — pairs with `C_DC_NA_1` /
	/// `C_DC_TA_1`.
	MDpNa1(MDpNa1),
	/// Step position (`M_ST_NA_1`) — pairs with `C_RC_NA_1` / `C_RC_TA_1`.
	MStNa1(MStNa1),
	/// 32-bit bitstring (`M_BO_NA_1`) — pairs with `C_BO_NA_1` / `C_BO_TA_1`.
	MBoNa1(MBoNa1),
	/// Measured value, normalized (`M_ME_NA_1`) — pairs with set-point
	/// `C_SE_NA_1` / `C_SE_TA_1`.
	MMeNa1(MMeNa1),
	/// Measured value, scaled (`M_ME_NB_1`) — pairs with `C_SE_NB_1` /
	/// `C_SE_TB_1`.
	MMeNb1(MMeNb1),
	/// Measured value, short float (`M_ME_NC_1`) — pairs with `C_SE_NC_1` /
	/// `C_SE_TC_1`.
	MMeNc1(MMeNc1),
}

impl PointValue {
	#[must_use]
	pub const fn type_id(&self) -> TypeId {
		match self {
			Self::MSpNa1(_) => TypeId::M_SP_NA_1,
			Self::MDpNa1(_) => TypeId::M_DP_NA_1,
			Self::MStNa1(_) => TypeId::M_ST_NA_1,
			Self::MBoNa1(_) => TypeId::M_BO_NA_1,
			Self::MMeNa1(_) => TypeId::M_ME_NA_1,
			Self::MMeNb1(_) => TypeId::M_ME_NB_1,
			Self::MMeNc1(_) => TypeId::M_ME_NC_1,
		}
	}
}
