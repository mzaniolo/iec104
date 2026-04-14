//! Build monitoring ASDUs (spontaneous + interrogation data) from the RTU point
//! model.

use std::collections::HashMap;

use super::model::{PointAddress, PointValue};
use crate::{
	asdu::Asdu,
	cot::Cot,
	types::{
		FromBytes, GenericObject, InformationObjects, MBoNa1, MBoTb1, MDpNa1, MDpTa1, MDpTb1,
		MEiNa1, MEpTa1, MEpTb1, MEpTc1, MEpTd1, MEpTe1, MEpTf1, MItNa1, MItTb1, MMeNa1, MMeNb1,
		MMeNc1, MMeNd1, MMeTa1, MMeTb1, MMeTc1, MMeTd1, MMeTe1, MMeTf1, MPsNa1, MSpNa1, MSpTa1,
		MSpTb1, MStNa1, MStTa1, MStTb1, ToBytes,
		commands::{Qoi, Rqt},
	},
	types_id::TypeId,
};

/// Maximum number of information objects per ASDU (7-bit count field).
pub(super) const MAX_OBJECTS_PER_ASDU: usize = 127;
/// Global common address for global interrogation.
const GLOBAL_COMMON_ADDRESS: u16 = 0xFFFF;

/// [`Cot`] for monitoring ASDUs answering `C_IC_NA_1` with this QOI (global or
/// group 1–16). [`None`] for [`Qoi::Unused`], [`Qoi::Other`], etc.
#[must_use]
pub(super) const fn interrogation_data_cot(qoi: Qoi) -> Option<Cot> {
	match qoi {
		Qoi::Global => Some(Cot::InterrogationGeneral),
		Qoi::Group1 => Some(Cot::InterrogationGroup1),
		Qoi::Group2 => Some(Cot::InterrogationGroup2),
		Qoi::Group3 => Some(Cot::InterrogationGroup3),
		Qoi::Group4 => Some(Cot::InterrogationGroup4),
		Qoi::Group5 => Some(Cot::InterrogationGroup5),
		Qoi::Group6 => Some(Cot::InterrogationGroup6),
		Qoi::Group7 => Some(Cot::InterrogationGroup7),
		Qoi::Group8 => Some(Cot::InterrogationGroup8),
		Qoi::Group9 => Some(Cot::InterrogationGroup9),
		Qoi::Group10 => Some(Cot::InterrogationGroup10),
		Qoi::Group11 => Some(Cot::InterrogationGroup11),
		Qoi::Group12 => Some(Cot::InterrogationGroup12),
		Qoi::Group13 => Some(Cot::InterrogationGroup13),
		Qoi::Group14 => Some(Cot::InterrogationGroup14),
		Qoi::Group15 => Some(Cot::InterrogationGroup15),
		Qoi::Group16 => Some(Cot::InterrogationGroup16),
		Qoi::Unused | Qoi::Other(_) => None,
	}
}

#[must_use]
fn point_included_in_interrogation(
	addr: PointAddress,
	qoi: Qoi,
	interrogation_groups: &HashMap<PointAddress, u8>,
) -> bool {
	match qoi {
		Qoi::Global => true,
		Qoi::Group1
		| Qoi::Group2
		| Qoi::Group3
		| Qoi::Group4
		| Qoi::Group5
		| Qoi::Group6
		| Qoi::Group7
		| Qoi::Group8
		| Qoi::Group9
		| Qoi::Group10
		| Qoi::Group11
		| Qoi::Group12
		| Qoi::Group13
		| Qoi::Group14
		| Qoi::Group15
		| Qoi::Group16 => {
			let expected = qoi.to_byte().saturating_sub(20);
			interrogation_groups.get(&addr).is_some_and(|g| *g == expected)
		}
		Qoi::Unused | Qoi::Other(_) => false,
	}
}

fn sort_and_push_interrogation_chunks<T>(
	out: &mut Vec<Asdu>,
	ca: u16,
	objs: &mut [(u32, T)],
	type_id: TypeId,
	cot: Cot,
) where
	T: Clone + FromBytes + ToBytes + Default,
	InformationObjects: From<Vec<GenericObject<T>>>,
{
	objs.sort_by_key(|(ioa, _)| *ioa);
	for chunk in objs.chunks(MAX_OBJECTS_PER_ASDU) {
		let chunk_objs: Vec<_> = chunk
			.iter()
			.map(|(ioa, m)| GenericObject { address: *ioa, object: m.clone() })
			.collect();
		out.push(Asdu {
			type_id,
			cot,
			originator_address: 0,
			address_field: ca,
			sequence: false,
			test: false,
			negative: false,
			information_objects: chunk_objs.into(),
		});
	}
}

/// Monitoring ASDU with [`Cot::Request`] (typical response to `C_RD_NA_1`).
#[must_use]
pub(super) fn monitoring_asdu_requested(address: PointAddress, value: &PointValue) -> Asdu {
	let mut a = spontaneous_asdu(address, value);
	a.cot = Cot::Request;
	a
}

pub(super) fn spontaneous_asdu(address: PointAddress, value: &PointValue) -> Asdu {
	let (type_id, information_objects) =
		value.single_object_information_objects(address.information_object_address);
	Asdu {
		type_id,
		cot: Cot::SpontaneousData,
		originator_address: 0,
		address_field: address.common_address,
		sequence: false,
		test: false,
		negative: false,
		information_objects,
	}
}

/// [`TypeId::M_EI_NA_1`] (end of initialization) after local startup — not a
/// process point.
///
/// Uses [`Cot::Initiated`], IOA `0`, and default [`MEiNa1`]
/// (`Coi::LocalPowerOn`). `common_address` is the ASDU common address field
/// (station); use [`station_common_address`] when deriving it from
/// the point model.
#[must_use]
pub(super) fn end_of_initialization_asdu(common_address: u16) -> Asdu {
	Asdu {
		type_id: TypeId::M_EI_NA_1,
		cot: Cot::Initiated,
		originator_address: 0,
		address_field: common_address,
		sequence: false,
		test: false,
		negative: false,
		information_objects: InformationObjects::MEiNa1(vec![GenericObject {
			address: 0,
			object: MEiNa1::default(),
		}]),
	}
}

/// Smallest [`PointAddress::common_address`] among points, or `0` if the model
/// is empty.
#[must_use]
pub(super) fn station_common_address(model: &HashMap<PointAddress, PointValue>) -> u16 {
	model.keys().map(|a| a.common_address).min().unwrap_or(0)
}

/// Interrogation data ASDUs for one common address and QOI (global or group
/// 1–16). Buckets must match [`PointValue`] variants (Type IDs 1–40 monitoring
/// range).
///
/// If `qoi` has no interrogation COT ([`interrogation_data_cot`] is [`None`],
/// e.g. [`Qoi::Unused`] or [`Qoi::Other`]), returns an empty [`Vec`] — no
/// panic.
#[allow(clippy::too_many_lines)]
#[must_use]
pub(super) fn interrogation_data_asdus(
	ca: u16,
	model: &HashMap<PointAddress, PointValue>,
	qoi: Qoi,
	interrogation_groups: &HashMap<PointAddress, u8>,
) -> Vec<Asdu> {
	let Some(data_cot) = interrogation_data_cot(qoi) else {
		return Vec::new();
	};
	let mut out = Vec::new();

	let mut m_sp_na: Vec<(u32, MSpNa1)> = Vec::new();
	let mut m_sp_ta: Vec<(u32, MSpTa1)> = Vec::new();
	let mut m_dp_na: Vec<(u32, MDpNa1)> = Vec::new();
	let mut m_dp_ta: Vec<(u32, MDpTa1)> = Vec::new();
	let mut m_st_na: Vec<(u32, MStNa1)> = Vec::new();
	let mut m_st_ta: Vec<(u32, MStTa1)> = Vec::new();
	let mut m_bo_na: Vec<(u32, MBoNa1)> = Vec::new();
	let mut m_me_na: Vec<(u32, MMeNa1)> = Vec::new();
	let mut m_me_ta: Vec<(u32, MMeTa1)> = Vec::new();
	let mut m_me_nb: Vec<(u32, MMeNb1)> = Vec::new();
	let mut m_me_tb: Vec<(u32, MMeTb1)> = Vec::new();
	let mut m_me_nc: Vec<(u32, MMeNc1)> = Vec::new();
	let mut m_me_tc: Vec<(u32, MMeTc1)> = Vec::new();
	let mut m_it_na: Vec<(u32, MItNa1)> = Vec::new();
	let mut m_ep_ta: Vec<(u32, MEpTa1)> = Vec::new();
	let mut m_ep_tb: Vec<(u32, MEpTb1)> = Vec::new();
	let mut m_ep_tc: Vec<(u32, MEpTc1)> = Vec::new();
	let mut m_ps_na: Vec<(u32, MPsNa1)> = Vec::new();
	let mut m_me_nd: Vec<(u32, MMeNd1)> = Vec::new();
	let mut m_sp_tb: Vec<(u32, MSpTb1)> = Vec::new();
	let mut m_dp_tb: Vec<(u32, MDpTb1)> = Vec::new();
	let mut m_st_tb: Vec<(u32, MStTb1)> = Vec::new();
	let mut m_bo_tb: Vec<(u32, MBoTb1)> = Vec::new();
	let mut m_me_td: Vec<(u32, MMeTd1)> = Vec::new();
	let mut m_me_te: Vec<(u32, MMeTe1)> = Vec::new();
	let mut m_me_tf: Vec<(u32, MMeTf1)> = Vec::new();
	let mut m_it_tb: Vec<(u32, MItTb1)> = Vec::new();
	let mut m_ep_td: Vec<(u32, MEpTd1)> = Vec::new();
	let mut m_ep_te: Vec<(u32, MEpTe1)> = Vec::new();
	let mut m_ep_tf: Vec<(u32, MEpTf1)> = Vec::new();

	for (addr, v) in model.iter() {
		if ca != GLOBAL_COMMON_ADDRESS && addr.common_address != ca {
			continue;
		}
		if !point_included_in_interrogation(*addr, qoi, interrogation_groups) {
			continue;
		}
		let ioa = addr.information_object_address;
		match v {
			PointValue::MSpNa1(m) => m_sp_na.push((ioa, m.clone())),
			PointValue::MSpTa1(m) => m_sp_ta.push((ioa, m.clone())),
			PointValue::MDpNa1(m) => m_dp_na.push((ioa, m.clone())),
			PointValue::MDpTa1(m) => m_dp_ta.push((ioa, m.clone())),
			PointValue::MStNa1(m) => m_st_na.push((ioa, m.clone())),
			PointValue::MStTa1(m) => m_st_ta.push((ioa, m.clone())),
			PointValue::MBoNa1(m) => m_bo_na.push((ioa, m.clone())),
			PointValue::MMeNa1(m) => m_me_na.push((ioa, m.clone())),
			PointValue::MMeTa1(m) => m_me_ta.push((ioa, m.clone())),
			PointValue::MMeNb1(m) => m_me_nb.push((ioa, m.clone())),
			PointValue::MMeTb1(m) => m_me_tb.push((ioa, m.clone())),
			PointValue::MMeNc1(m) => m_me_nc.push((ioa, m.clone())),
			PointValue::MMeTc1(m) => m_me_tc.push((ioa, m.clone())),
			PointValue::MItNa1(m) => m_it_na.push((ioa, m.clone())),
			PointValue::MEpTa1(m) => m_ep_ta.push((ioa, m.clone())),
			PointValue::MEpTb1(m) => m_ep_tb.push((ioa, m.clone())),
			PointValue::MEpTc1(m) => m_ep_tc.push((ioa, m.clone())),
			PointValue::MPsNa1(m) => m_ps_na.push((ioa, m.clone())),
			PointValue::MMeNd1(m) => m_me_nd.push((ioa, m.clone())),
			PointValue::MSpTb1(m) => m_sp_tb.push((ioa, m.clone())),
			PointValue::MDpTb1(m) => m_dp_tb.push((ioa, m.clone())),
			PointValue::MStTb1(m) => m_st_tb.push((ioa, m.clone())),
			PointValue::MBoTb1(m) => m_bo_tb.push((ioa, m.clone())),
			PointValue::MMeTd1(m) => m_me_td.push((ioa, m.clone())),
			PointValue::MMeTe1(m) => m_me_te.push((ioa, m.clone())),
			PointValue::MMeTf1(m) => m_me_tf.push((ioa, m.clone())),
			PointValue::MItTb1(m) => m_it_tb.push((ioa, m.clone())),
			PointValue::MEpTd1(m) => m_ep_td.push((ioa, m.clone())),
			PointValue::MEpTe1(m) => m_ep_te.push((ioa, m.clone())),
			PointValue::MEpTf1(m) => m_ep_tf.push((ioa, m.clone())),
		}
	}

	sort_and_push_interrogation_chunks(&mut out, ca, &mut m_sp_na, TypeId::M_SP_NA_1, data_cot);
	sort_and_push_interrogation_chunks(&mut out, ca, &mut m_sp_ta, TypeId::M_SP_TA_1, data_cot);
	sort_and_push_interrogation_chunks(&mut out, ca, &mut m_dp_na, TypeId::M_DP_NA_1, data_cot);
	sort_and_push_interrogation_chunks(&mut out, ca, &mut m_dp_ta, TypeId::M_DP_TA_1, data_cot);
	sort_and_push_interrogation_chunks(&mut out, ca, &mut m_st_na, TypeId::M_ST_NA_1, data_cot);
	sort_and_push_interrogation_chunks(&mut out, ca, &mut m_st_ta, TypeId::M_ST_TA_1, data_cot);
	sort_and_push_interrogation_chunks(&mut out, ca, &mut m_bo_na, TypeId::M_BO_NA_1, data_cot);
	sort_and_push_interrogation_chunks(&mut out, ca, &mut m_me_na, TypeId::M_ME_NA_1, data_cot);
	sort_and_push_interrogation_chunks(&mut out, ca, &mut m_me_ta, TypeId::M_ME_TA_1, data_cot);
	sort_and_push_interrogation_chunks(&mut out, ca, &mut m_me_nb, TypeId::M_ME_NB_1, data_cot);
	sort_and_push_interrogation_chunks(&mut out, ca, &mut m_me_tb, TypeId::M_ME_TB_1, data_cot);
	sort_and_push_interrogation_chunks(&mut out, ca, &mut m_me_nc, TypeId::M_ME_NC_1, data_cot);
	sort_and_push_interrogation_chunks(&mut out, ca, &mut m_me_tc, TypeId::M_ME_TC_1, data_cot);
	sort_and_push_interrogation_chunks(&mut out, ca, &mut m_it_na, TypeId::M_IT_NA_1, data_cot);
	sort_and_push_interrogation_chunks(&mut out, ca, &mut m_ep_ta, TypeId::M_EP_TA_1, data_cot);
	sort_and_push_interrogation_chunks(&mut out, ca, &mut m_ep_tb, TypeId::M_EP_TB_1, data_cot);
	sort_and_push_interrogation_chunks(&mut out, ca, &mut m_ep_tc, TypeId::M_EP_TC_1, data_cot);
	sort_and_push_interrogation_chunks(&mut out, ca, &mut m_ps_na, TypeId::M_PS_NA_1, data_cot);
	sort_and_push_interrogation_chunks(&mut out, ca, &mut m_me_nd, TypeId::M_ME_ND_1, data_cot);
	sort_and_push_interrogation_chunks(&mut out, ca, &mut m_sp_tb, TypeId::M_SP_TB_1, data_cot);
	sort_and_push_interrogation_chunks(&mut out, ca, &mut m_dp_tb, TypeId::M_DP_TB_1, data_cot);
	sort_and_push_interrogation_chunks(&mut out, ca, &mut m_st_tb, TypeId::M_ST_TB_1, data_cot);
	sort_and_push_interrogation_chunks(&mut out, ca, &mut m_bo_tb, TypeId::M_BO_TB_1, data_cot);
	sort_and_push_interrogation_chunks(&mut out, ca, &mut m_me_td, TypeId::M_ME_TD_1, data_cot);
	sort_and_push_interrogation_chunks(&mut out, ca, &mut m_me_te, TypeId::M_ME_TE_1, data_cot);
	sort_and_push_interrogation_chunks(&mut out, ca, &mut m_me_tf, TypeId::M_ME_TF_1, data_cot);
	sort_and_push_interrogation_chunks(&mut out, ca, &mut m_it_tb, TypeId::M_IT_TB_1, data_cot);
	sort_and_push_interrogation_chunks(&mut out, ca, &mut m_ep_td, TypeId::M_EP_TD_1, data_cot);
	sort_and_push_interrogation_chunks(&mut out, ca, &mut m_ep_te, TypeId::M_EP_TE_1, data_cot);
	sort_and_push_interrogation_chunks(&mut out, ca, &mut m_ep_tf, TypeId::M_EP_TF_1, data_cot);

	out
}

/// Built counter-interrogation monitoring ASDUs and the model addresses that
/// contributed (for freeze/reset after the read).
#[derive(Debug, Default)]
pub(super) struct CounterInterrogationData {
	pub asdus: Vec<Asdu>,
	pub read_addresses: Vec<PointAddress>,
}

/// [`Cot`] for `M_IT_*` ASDUs answering `C_CI_NA_1` with this RQT.
#[must_use]
pub(super) const fn counter_interrogation_data_cot(rqt: Rqt) -> Option<Cot> {
	match rqt {
		Rqt::ReqCoGen => Some(Cot::CounterInterrogationGeneral),
		Rqt::ReqCo1 => Some(Cot::CounterInterrogationGroup1),
		Rqt::ReqCo2 => Some(Cot::CounterInterrogationGroup2),
		Rqt::ReqCo3 => Some(Cot::CounterInterrogationGroup3),
		Rqt::ReqCo4 => Some(Cot::CounterInterrogationGroup4),
		Rqt::None | Rqt::Other(_) => None,
	}
}

#[must_use]
fn point_included_in_counter_interrogation(
	addr: PointAddress,
	rqt: Rqt,
	counter_groups: &HashMap<PointAddress, u8>,
) -> bool {
	match rqt {
		Rqt::ReqCoGen => true,
		Rqt::ReqCo1 => counter_groups.get(&addr).is_some_and(|g| *g == 1),
		Rqt::ReqCo2 => counter_groups.get(&addr).is_some_and(|g| *g == 2),
		Rqt::ReqCo3 => counter_groups.get(&addr).is_some_and(|g| *g == 3),
		Rqt::ReqCo4 => counter_groups.get(&addr).is_some_and(|g| *g == 4),
		Rqt::None | Rqt::Other(_) => false,
	}
}

/// `M_IT_NA_1` / `M_IT_TB_1` ASDUs for one common address and counter RQT, plus
/// the addresses read (for [`Frz`](crate::types::commands::Frz) handling).
/// Returns empty when [`counter_interrogation_data_cot`]`(rqt)` is [`None`].
#[must_use]
pub(super) fn counter_interrogation_data(
	ca: u16,
	model: &HashMap<PointAddress, PointValue>,
	rqt: Rqt,
	counter_groups: &HashMap<PointAddress, u8>,
) -> CounterInterrogationData {
	let Some(data_cot) = counter_interrogation_data_cot(rqt) else {
		return CounterInterrogationData::default();
	};
	let mut read_addresses = Vec::new();
	let mut m_it_na: Vec<(u32, MItNa1)> = Vec::new();
	let mut m_it_tb: Vec<(u32, MItTb1)> = Vec::new();
	for (addr, v) in model.iter() {
		if ca != GLOBAL_COMMON_ADDRESS && addr.common_address != ca {
			continue;
		}
		if !v.is_counter_integration() {
			continue;
		}
		if !point_included_in_counter_interrogation(*addr, rqt, counter_groups) {
			continue;
		}
		read_addresses.push(*addr);
		let ioa = addr.information_object_address;
		match v {
			PointValue::MItNa1(m) => m_it_na.push((ioa, m.clone())),
			PointValue::MItTb1(m) => m_it_tb.push((ioa, m.clone())),
			_ => {}
		}
	}
	read_addresses.sort_by_key(|a| a.information_object_address);
	let mut asdus = Vec::new();
	sort_and_push_interrogation_chunks(&mut asdus, ca, &mut m_it_na, TypeId::M_IT_NA_1, data_cot);
	sort_and_push_interrogation_chunks(&mut asdus, ca, &mut m_it_tb, TypeId::M_IT_TB_1, data_cot);
	CounterInterrogationData { asdus, read_addresses }
}

#[cfg(test)]
mod interrogation_qoi_tests {
	use std::collections::HashMap;

	use super::{interrogation_data_asdus, interrogation_data_cot};
	use crate::{
		rtu_server::model::{PointAddress, PointValue},
		types::{MMeNa1, commands::Qoi, quality_descriptors::Qds},
	};

	#[test]
	fn cot_tracks_qoi() {
		assert_eq!(
			interrogation_data_cot(Qoi::Global),
			Some(crate::cot::Cot::InterrogationGeneral)
		);
		assert_eq!(interrogation_data_cot(Qoi::Group3), Some(crate::cot::Cot::InterrogationGroup3));
		assert_eq!(interrogation_data_cot(Qoi::Other(19)), None);
	}

	#[test]
	fn group_interrogation_filters_points() {
		let ca = 1_u16;
		let a1 = PointAddress::new(ca, 10);
		let a2 = PointAddress::new(ca, 20);
		let v = PointValue::MMeNa1(MMeNa1 { nva: 0, qds: Qds::default() });
		let mut model = HashMap::new();
		model.insert(a1, v.clone());
		model.insert(a2, v);
		let mut groups = HashMap::new();
		groups.insert(a1, 2_u8);
		groups.insert(a2, 3_u8);

		let global = interrogation_data_asdus(ca, &model, Qoi::Global, &groups);
		assert_eq!(global.len(), 1);
		assert_eq!(global[0].cot, crate::cot::Cot::InterrogationGeneral);
		assert_eq!(global[0].information_objects.len(), 2);

		let g2 = interrogation_data_asdus(ca, &model, Qoi::Group2, &groups);
		assert_eq!(g2.len(), 1);
		assert_eq!(g2[0].cot, crate::cot::Cot::InterrogationGroup2);
		assert_eq!(g2[0].information_objects.len(), 1);

		let g3 = interrogation_data_asdus(ca, &model, Qoi::Group3, &groups);
		assert_eq!(g3.len(), 1);
		assert_eq!(g3[0].information_objects.len(), 1);

		let g1_empty = interrogation_data_asdus(ca, &model, Qoi::Group1, &groups);
		assert!(g1_empty.is_empty());
	}

	#[test]
	fn unsupported_qoi_returns_empty_asdus_without_panic() {
		let ca = 1_u16;
		let a1 = PointAddress::new(ca, 10);
		let v = PointValue::MMeNa1(MMeNa1 { nva: 0, qds: Qds::default() });
		let mut model = HashMap::new();
		model.insert(a1, v);
		let groups = HashMap::new();
		assert!(interrogation_data_asdus(ca, &model, Qoi::Unused, &groups).is_empty());
		assert!(interrogation_data_asdus(ca, &model, Qoi::Other(19), &groups).is_empty());
	}
}

#[cfg(test)]
mod counter_interrogation_rqt_tests {
	use std::collections::HashMap;

	use super::{counter_interrogation_data, counter_interrogation_data_cot};
	use crate::{
		cot::Cot,
		rtu_server::model::{PointAddress, PointValue},
		types::{MItNa1, commands::Rqt, quality_descriptors::Qds},
	};

	#[test]
	fn counter_cot_tracks_rqt() {
		assert_eq!(
			counter_interrogation_data_cot(Rqt::ReqCoGen),
			Some(Cot::CounterInterrogationGeneral)
		);
		assert_eq!(
			counter_interrogation_data_cot(Rqt::ReqCo2),
			Some(Cot::CounterInterrogationGroup2)
		);
		assert_eq!(counter_interrogation_data_cot(Rqt::None), None);
		assert_eq!(counter_interrogation_data_cot(Rqt::Other(0)), None);
	}

	#[test]
	fn counter_group_filters_counters() {
		let ca = 1_u16;
		let c1 = PointAddress::new(ca, 10);
		let c2 = PointAddress::new(ca, 20);
		let v = PointValue::MItNa1(MItNa1 { bcr: 1, qds: Qds::default() });
		let mut model = HashMap::new();
		model.insert(c1, v.clone());
		model.insert(c2, v);
		let mut counter_groups = HashMap::new();
		counter_groups.insert(c1, 1_u8);
		counter_groups.insert(c2, 2_u8);

		let general = counter_interrogation_data(ca, &model, Rqt::ReqCoGen, &counter_groups);
		assert_eq!(general.asdus.len(), 1);
		assert_eq!(general.asdus[0].cot, Cot::CounterInterrogationGeneral);
		assert_eq!(general.read_addresses.len(), 2);

		let g1 = counter_interrogation_data(ca, &model, Rqt::ReqCo1, &counter_groups);
		assert_eq!(g1.asdus.len(), 1);
		assert_eq!(g1.asdus[0].cot, Cot::CounterInterrogationGroup1);
		assert_eq!(g1.read_addresses, vec![c1]);

		let g3 = counter_interrogation_data(ca, &model, Rqt::ReqCo3, &counter_groups);
		assert!(g3.asdus.is_empty());
		assert!(g3.read_addresses.is_empty());
	}

	#[test]
	fn unsupported_rqt_returns_empty_without_panic() {
		let ca = 1_u16;
		let a = PointAddress::new(ca, 1);
		let mut model = HashMap::new();
		model.insert(a, PointValue::MItNa1(MItNa1 { bcr: 0, qds: Qds::default() }));
		let groups = HashMap::new();
		let d = counter_interrogation_data(ca, &model, Rqt::None, &groups);
		assert!(d.asdus.is_empty());
		assert!(d.read_addresses.is_empty());
	}
}
