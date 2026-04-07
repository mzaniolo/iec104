//! Build monitoring ASDUs (spontaneous + interrogation data) from the RTU point
//! model.

use std::collections::HashMap;

use super::model::{PointAddress, PointValue};
use crate::{
	asdu::Asdu,
	cot::Cot,
	types::{GenericObject, InformationObjects, MMeNa1, MMeNb1, MMeNc1, MSpNa1},
	types_id::TypeId,
};

/// Maximum number of information objects per ASDU (7-bit count field).
pub(super) const MAX_OBJECTS_PER_ASDU: usize = 127;

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
			PointValue::MMeNa1(m) => mna.push((ioa, m.clone())),
			PointValue::MMeNb1(m) => mnb.push((ioa, m.clone())),
			PointValue::MMeNc1(m) => mnc.push((ioa, m.clone())),
		}
	}

	sp.sort_by_key(|(ioa, _)| *ioa);
	for chunk in sp.chunks(MAX_OBJECTS_PER_ASDU) {
		let objs: Vec<GenericObject<MSpNa1>> = chunk
			.iter()
			.map(|(ioa, m)| GenericObject { address: *ioa, object: m.clone() })
			.collect();
		out.push(Asdu {
			type_id: TypeId::M_SP_NA_1,
			cot: Cot::InterrogationGeneral,
			originator_address: 0,
			address_field: ca,
			sequence: false,
			test: false,
			negative: false,
			information_objects: InformationObjects::MSpNa1(objs),
		});
	}

	mna.sort_by_key(|(ioa, _)| *ioa);
	for chunk in mna.chunks(MAX_OBJECTS_PER_ASDU) {
		let objs: Vec<GenericObject<MMeNa1>> = chunk
			.iter()
			.map(|(ioa, m)| GenericObject { address: *ioa, object: m.clone() })
			.collect();
		out.push(Asdu {
			type_id: TypeId::M_ME_NA_1,
			cot: Cot::InterrogationGeneral,
			originator_address: 0,
			address_field: ca,
			sequence: false,
			test: false,
			negative: false,
			information_objects: InformationObjects::MMeNa1(objs),
		});
	}

	mnb.sort_by_key(|(ioa, _)| *ioa);
	for chunk in mnb.chunks(MAX_OBJECTS_PER_ASDU) {
		let objs: Vec<GenericObject<MMeNb1>> = chunk
			.iter()
			.map(|(ioa, m)| GenericObject { address: *ioa, object: m.clone() })
			.collect();
		out.push(Asdu {
			type_id: TypeId::M_ME_NB_1,
			cot: Cot::InterrogationGeneral,
			originator_address: 0,
			address_field: ca,
			sequence: false,
			test: false,
			negative: false,
			information_objects: InformationObjects::MMeNb1(objs),
		});
	}

	mnc.sort_by_key(|(ioa, _)| *ioa);
	for chunk in mnc.chunks(MAX_OBJECTS_PER_ASDU) {
		let objs: Vec<GenericObject<MMeNc1>> = chunk
			.iter()
			.map(|(ioa, m)| GenericObject { address: *ioa, object: m.clone() })
			.collect();
		out.push(Asdu {
			type_id: TypeId::M_ME_NC_1,
			cot: Cot::InterrogationGeneral,
			originator_address: 0,
			address_field: ca,
			sequence: false,
			test: false,
			negative: false,
			information_objects: InformationObjects::MMeNc1(objs),
		});
	}

	out
}
