use std::fmt;

use crate::{
	types::{MMeNc1, MSpNa1},
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

/// In-memory value for one monitored point (minimal v1 set).
#[derive(Debug, Clone, PartialEq)]
pub enum PointValue {
	/// Single-point information without time tag (`M_SP_NA_1`).
	MSpNa1(MSpNa1),
	/// Measured value, short float (`M_ME_NC_1`).
	MMeNc1(MMeNc1),
}

impl PointValue {
	#[must_use]
	pub const fn type_id(&self) -> TypeId {
		match self {
			Self::MSpNa1(_) => TypeId::M_SP_NA_1,
			Self::MMeNc1(_) => TypeId::M_ME_NC_1,
		}
	}
}
