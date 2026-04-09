//! Build monitoring ASDUs (spontaneous + interrogation data) from the RTU point
//! model.

use std::collections::HashMap;

use super::model::{PointAddress, PointValue};
use crate::{
	asdu::Asdu,
	cot::Cot,
	types::{
		FromBytes, GenericObject, InformationObjects, MBoNa1, MDpNa1, MMeNa1, MMeNb1, MMeNc1,
		MSpNa1, MStNa1, ToBytes,
	},
	types_id::TypeId,
};

/// Maximum number of information objects per ASDU (7-bit count field).
pub(super) const MAX_OBJECTS_PER_ASDU: usize = 127;

fn sort_points_by_ioa<T>(v: &mut [(u32, T)]) {
	v.sort_by_key(|(ioa, _)| *ioa);
}

fn push_interrogation_chunks<T: Clone + FromBytes + ToBytes + Default>(
	out: &mut Vec<Asdu>,
	ca: u16,
	objs: &[(u32, T)],
	type_id: TypeId,
	wrap: impl Fn(Vec<GenericObject<T>>) -> InformationObjects,
) {
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
			information_objects: wrap(chunk_objs),
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
	let ca = address.common_address;
	let ioa = address.information_object_address;
	match value {
		PointValue::MSpNa1(m) => Asdu {
			type_id: TypeId::M_SP_NA_1,
			cot: Cot::SpontaneousData,
			originator_address: 0,
			address_field: ca,
			sequence: false,
			test: false,
			negative: false,
			information_objects: InformationObjects::MSpNa1(vec![GenericObject {
				address: ioa,
				object: m.clone(),
			}]),
		},
		PointValue::MDpNa1(m) => Asdu {
			type_id: TypeId::M_DP_NA_1,
			cot: Cot::SpontaneousData,
			originator_address: 0,
			address_field: ca,
			sequence: false,
			test: false,
			negative: false,
			information_objects: InformationObjects::MDpNa1(vec![GenericObject {
				address: ioa,
				object: m.clone(),
			}]),
		},
		PointValue::MStNa1(m) => Asdu {
			type_id: TypeId::M_ST_NA_1,
			cot: Cot::SpontaneousData,
			originator_address: 0,
			address_field: ca,
			sequence: false,
			test: false,
			negative: false,
			information_objects: InformationObjects::MStNa1(vec![GenericObject {
				address: ioa,
				object: m.clone(),
			}]),
		},
		PointValue::MBoNa1(m) => Asdu {
			type_id: TypeId::M_BO_NA_1,
			cot: Cot::SpontaneousData,
			originator_address: 0,
			address_field: ca,
			sequence: false,
			test: false,
			negative: false,
			information_objects: InformationObjects::MBoNa1(vec![GenericObject {
				address: ioa,
				object: m.clone(),
			}]),
		},
		PointValue::MMeNa1(m) => Asdu {
			type_id: TypeId::M_ME_NA_1,
			cot: Cot::SpontaneousData,
			originator_address: 0,
			address_field: ca,
			sequence: false,
			test: false,
			negative: false,
			information_objects: InformationObjects::MMeNa1(vec![GenericObject {
				address: ioa,
				object: m.clone(),
			}]),
		},
		PointValue::MMeNb1(m) => Asdu {
			type_id: TypeId::M_ME_NB_1,
			cot: Cot::SpontaneousData,
			originator_address: 0,
			address_field: ca,
			sequence: false,
			test: false,
			negative: false,
			information_objects: InformationObjects::MMeNb1(vec![GenericObject {
				address: ioa,
				object: m.clone(),
			}]),
		},
		PointValue::MMeNc1(m) => Asdu {
			type_id: TypeId::M_ME_NC_1,
			cot: Cot::SpontaneousData,
			originator_address: 0,
			address_field: ca,
			sequence: false,
			test: false,
			negative: false,
			information_objects: InformationObjects::MMeNc1(vec![GenericObject {
				address: ioa,
				object: m.clone(),
			}]),
		},
	}
}

pub(super) fn interrogation_data_asdus(
	ca: u16,
	model: &HashMap<PointAddress, PointValue>,
) -> Vec<Asdu> {
	let mut out = Vec::new();

	let mut sp: Vec<(u32, MSpNa1)> = Vec::new();
	let mut dp: Vec<(u32, MDpNa1)> = Vec::new();
	let mut st: Vec<(u32, MStNa1)> = Vec::new();
	let mut bo: Vec<(u32, MBoNa1)> = Vec::new();
	let mut mna: Vec<(u32, MMeNa1)> = Vec::new();
	let mut mnb: Vec<(u32, MMeNb1)> = Vec::new();
	let mut mnc: Vec<(u32, MMeNc1)> = Vec::new();
	for (addr, v) in model.iter() {
		if addr.common_address != ca {
			continue;
		}
		let ioa = addr.information_object_address;
		match v {
			PointValue::MSpNa1(m) => sp.push((ioa, m.clone())),
			PointValue::MDpNa1(m) => dp.push((ioa, m.clone())),
			PointValue::MStNa1(m) => st.push((ioa, m.clone())),
			PointValue::MBoNa1(m) => bo.push((ioa, m.clone())),
			PointValue::MMeNa1(m) => mna.push((ioa, m.clone())),
			PointValue::MMeNb1(m) => mnb.push((ioa, m.clone())),
			PointValue::MMeNc1(m) => mnc.push((ioa, m.clone())),
		}
	}

	sort_points_by_ioa(&mut sp);
	sort_points_by_ioa(&mut dp);
	sort_points_by_ioa(&mut st);
	sort_points_by_ioa(&mut bo);
	sort_points_by_ioa(&mut mna);
	sort_points_by_ioa(&mut mnb);
	sort_points_by_ioa(&mut mnc);

	push_interrogation_chunks(&mut out, ca, &sp, TypeId::M_SP_NA_1, InformationObjects::MSpNa1);
	push_interrogation_chunks(&mut out, ca, &dp, TypeId::M_DP_NA_1, InformationObjects::MDpNa1);
	push_interrogation_chunks(&mut out, ca, &st, TypeId::M_ST_NA_1, InformationObjects::MStNa1);
	push_interrogation_chunks(&mut out, ca, &bo, TypeId::M_BO_NA_1, InformationObjects::MBoNa1);
	push_interrogation_chunks(&mut out, ca, &mna, TypeId::M_ME_NA_1, InformationObjects::MMeNa1);
	push_interrogation_chunks(&mut out, ca, &mnb, TypeId::M_ME_NB_1, InformationObjects::MMeNb1);
	push_interrogation_chunks(&mut out, ca, &mnc, TypeId::M_ME_NC_1, InformationObjects::MMeNc1);

	out
}
