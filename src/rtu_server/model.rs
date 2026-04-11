use std::{collections::HashMap, fmt};

pub use super::point_value::PointValue;

/// Point values plus interrogation-group assignments for
/// [`super::RtuServer::start`](crate::rtu_server::RtuServer::start).
#[derive(Debug)]
pub struct RtuInitialMaps {
	pub points: HashMap<PointAddress, PointValue>,
	pub interrogation_groups: HashMap<PointAddress, u8>,
}

/// Initial point for [`super::RtuServer::start`] /
/// [`super::RtuServerHandle::register_points`].
///
/// Interrogation group is IEC 60870-5-101 §7.3.6.22: values **1–16** assign the
/// point to that group for group interrogation (`C_IC_NA_1` QOI 21–36).
/// [`None`] means the point is included in **global** interrogation only (QOI
/// 20), not in group-specific interrogations.
#[derive(Debug, Clone, PartialEq)]
pub struct RtuInitialPoint {
	pub address: PointAddress,
	pub value: PointValue,
	pub interrogation_group: Option<u8>,
}

impl RtuInitialPoint {
	#[must_use]
	pub const fn new(address: PointAddress, value: PointValue) -> Self {
		Self { address, value, interrogation_group: None }
	}

	/// `interrogation_group` must be in **1..=16** or this returns [`None`].
	#[must_use]
	pub fn with_interrogation_group(
		address: PointAddress,
		value: PointValue,
		interrogation_group: u8,
	) -> Option<Self> {
		(1..=16).contains(&interrogation_group).then_some(Self {
			address,
			value,
			interrogation_group: Some(interrogation_group),
		})
	}
}

impl From<(PointAddress, PointValue)> for RtuInitialPoint {
	fn from((address, value): (PointAddress, PointValue)) -> Self {
		Self { address, value, interrogation_group: None }
	}
}

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
