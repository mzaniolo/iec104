use std::fmt;

pub use super::point_value::PointValue;

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
