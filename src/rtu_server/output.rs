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
	},
	types_id::TypeId,
};

/// Maximum number of information objects per ASDU (7-bit count field).
pub(super) const MAX_OBJECTS_PER_ASDU: usize = 127;

fn sort_and_push_interrogation_chunks<T>(
	out: &mut Vec<Asdu>,
	ca: u16,
	objs: &mut [(u32, T)],
	type_id: TypeId,
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
			cot: Cot::InterrogationGeneral,
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

/// General-interrogation data ASDUs for one common address. Buckets must match
/// [`PointValue`] variants (Type IDs 1–40 monitoring range).
#[allow(clippy::too_many_lines)]
#[must_use]
pub(super) fn interrogation_data_asdus(
	ca: u16,
	model: &HashMap<PointAddress, PointValue>,
) -> Vec<Asdu> {
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
		if addr.common_address != ca {
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

	sort_and_push_interrogation_chunks(&mut out, ca, &mut m_sp_na, TypeId::M_SP_NA_1);
	sort_and_push_interrogation_chunks(&mut out, ca, &mut m_sp_ta, TypeId::M_SP_TA_1);
	sort_and_push_interrogation_chunks(&mut out, ca, &mut m_dp_na, TypeId::M_DP_NA_1);
	sort_and_push_interrogation_chunks(&mut out, ca, &mut m_dp_ta, TypeId::M_DP_TA_1);
	sort_and_push_interrogation_chunks(&mut out, ca, &mut m_st_na, TypeId::M_ST_NA_1);
	sort_and_push_interrogation_chunks(&mut out, ca, &mut m_st_ta, TypeId::M_ST_TA_1);
	sort_and_push_interrogation_chunks(&mut out, ca, &mut m_bo_na, TypeId::M_BO_NA_1);
	sort_and_push_interrogation_chunks(&mut out, ca, &mut m_me_na, TypeId::M_ME_NA_1);
	sort_and_push_interrogation_chunks(&mut out, ca, &mut m_me_ta, TypeId::M_ME_TA_1);
	sort_and_push_interrogation_chunks(&mut out, ca, &mut m_me_nb, TypeId::M_ME_NB_1);
	sort_and_push_interrogation_chunks(&mut out, ca, &mut m_me_tb, TypeId::M_ME_TB_1);
	sort_and_push_interrogation_chunks(&mut out, ca, &mut m_me_nc, TypeId::M_ME_NC_1);
	sort_and_push_interrogation_chunks(&mut out, ca, &mut m_me_tc, TypeId::M_ME_TC_1);
	sort_and_push_interrogation_chunks(&mut out, ca, &mut m_it_na, TypeId::M_IT_NA_1);
	sort_and_push_interrogation_chunks(&mut out, ca, &mut m_ep_ta, TypeId::M_EP_TA_1);
	sort_and_push_interrogation_chunks(&mut out, ca, &mut m_ep_tb, TypeId::M_EP_TB_1);
	sort_and_push_interrogation_chunks(&mut out, ca, &mut m_ep_tc, TypeId::M_EP_TC_1);
	sort_and_push_interrogation_chunks(&mut out, ca, &mut m_ps_na, TypeId::M_PS_NA_1);
	sort_and_push_interrogation_chunks(&mut out, ca, &mut m_me_nd, TypeId::M_ME_ND_1);
	sort_and_push_interrogation_chunks(&mut out, ca, &mut m_sp_tb, TypeId::M_SP_TB_1);
	sort_and_push_interrogation_chunks(&mut out, ca, &mut m_dp_tb, TypeId::M_DP_TB_1);
	sort_and_push_interrogation_chunks(&mut out, ca, &mut m_st_tb, TypeId::M_ST_TB_1);
	sort_and_push_interrogation_chunks(&mut out, ca, &mut m_bo_tb, TypeId::M_BO_TB_1);
	sort_and_push_interrogation_chunks(&mut out, ca, &mut m_me_td, TypeId::M_ME_TD_1);
	sort_and_push_interrogation_chunks(&mut out, ca, &mut m_me_te, TypeId::M_ME_TE_1);
	sort_and_push_interrogation_chunks(&mut out, ca, &mut m_me_tf, TypeId::M_ME_TF_1);
	sort_and_push_interrogation_chunks(&mut out, ca, &mut m_it_tb, TypeId::M_IT_TB_1);
	sort_and_push_interrogation_chunks(&mut out, ca, &mut m_ep_td, TypeId::M_EP_TD_1);
	sort_and_push_interrogation_chunks(&mut out, ca, &mut m_ep_te, TypeId::M_EP_TE_1);
	sort_and_push_interrogation_chunks(&mut out, ca, &mut m_ep_tf, TypeId::M_EP_TF_1);

	out
}
