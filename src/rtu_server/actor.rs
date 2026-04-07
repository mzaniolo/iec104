use std::{collections::HashMap, net::SocketAddr};

use super::{
	error::{InterrogationError, SetPointError},
	model::{PointAddress, PointValue},
};
use crate::{
	asdu::Asdu,
	cot::Cot,
	server::{ConnectionId, Server, ServerCallback},
	types::{CIcNa1, GenericObject, InformationObjects, MMeNc1, MSpNa1, commands::Qoi},
	types_id::TypeId,
};

/// Maximum number of information objects per ASDU (7-bit count field).
const MAX_OBJECTS_PER_ASDU: usize = 127;

pub(crate) enum ActorMsg {
	IngressAsdu {
		asdu: Asdu,
		connection_id: ConnectionId,
		peer: SocketAddr,
	},
	SetPoint {
		address: PointAddress,
		value: PointValue,
		reply: tokio::sync::oneshot::Sender<Result<(), SetPointError>>,
	},
	Register {
		address: PointAddress,
		initial: PointValue,
		reply: tokio::sync::oneshot::Sender<Result<(), PointAddress>>,
	},
}

#[derive(Clone)]
pub(super) struct NetworkIngress {
	tx: tokio::sync::mpsc::UnboundedSender<ActorMsg>,
}

impl NetworkIngress {
	pub(super) const fn new(tx: tokio::sync::mpsc::UnboundedSender<ActorMsg>) -> Self {
		Self { tx }
	}
}

#[async_trait::async_trait]
impl ServerCallback for NetworkIngress {
	async fn on_new_objects(&self, asdu: Asdu, connection_id: ConnectionId, peer: SocketAddr) {
		let _ = self.tx.send(ActorMsg::IngressAsdu { asdu, connection_id, peer });
	}
}

pub(super) async fn run_actor(
	mut rx: tokio::sync::mpsc::UnboundedReceiver<ActorMsg>,
	server: Server,
	mut model: HashMap<PointAddress, PointValue>,
) {
	while let Some(msg) = rx.recv().await {
		match msg {
			ActorMsg::SetPoint { address, value, reply } => {
				let res = handle_set_point(&mut model, &server, address, value).await;
				let _ = reply.send(res);
			}
			ActorMsg::Register { address, initial, reply } => {
				use std::collections::hash_map::Entry;
				let res = match model.entry(address) {
					Entry::Vacant(e) => {
						e.insert(initial);
						Ok(())
					}
					Entry::Occupied(e) => Err(*e.key()),
				};
				let _ = reply.send(res);
			}
			ActorMsg::IngressAsdu { asdu, connection_id, peer } => {
				handle_ingress_asdu(&model, &server, asdu, connection_id, peer).await;
			}
		}
	}
	tracing::warn!("RTU actor channel closed; stopping model loop");
}

async fn handle_set_point(
	model: &mut HashMap<PointAddress, PointValue>,
	server: &Server,
	address: PointAddress,
	value: PointValue,
) -> Result<(), SetPointError> {
	let existing = model.get(&address).ok_or(SetPointError::UnknownPoint { address })?;
	let expected = existing.type_id();
	let got = value.type_id();
	if expected != got {
		return Err(SetPointError::TypeMismatch { address, expected, got });
	}
	model.insert(address, value.clone());
	let asdu = spontaneous_asdu(address, &value);
	server
		.broadcast_asdu(asdu)
		.await
		.map_err(|source| SetPointError::BroadcastFailed { source })?;
	Ok(())
}

async fn handle_ingress_asdu(
	model: &HashMap<PointAddress, PointValue>,
	server: &Server,
	asdu: Asdu,
	connection_id: ConnectionId,
	peer: SocketAddr,
) {
	if asdu.type_id == TypeId::C_IC_NA_1 {
		match handle_interrogation(model, server, &asdu, connection_id).await {
			Ok(()) => return,
			Err(InterrogationError::Skipped) => {}
			Err(e) => tracing::error!(error = %e, ?peer, "interrogation handling"),
		}
	}
	tracing::trace!(?peer, type_id = ?asdu.type_id, "ingress ASDU (no handler)");
}

/// `Ok(())` if this ASDU was handled as interrogation;
/// [`InterrogationError::Skipped`] if not applicable.
async fn handle_interrogation(
	model: &HashMap<PointAddress, PointValue>,
	server: &Server,
	asdu: &Asdu,
	connection_id: ConnectionId,
) -> Result<(), InterrogationError> {
	let InformationObjects::CIcNa1(objs) = &asdu.information_objects else {
		return Err(InterrogationError::Skipped);
	};
	let Some(go) = objs.first() else {
		return Err(InterrogationError::Skipped);
	};
	let qoi = go.object.qoi;
	if matches!(qoi, Qoi::Unused) {
		return Err(InterrogationError::Skipped);
	}
	if asdu.cot != Cot::Activation {
		return Err(InterrogationError::Skipped);
	}

	let actcon = Asdu {
		type_id: TypeId::C_IC_NA_1,
		cot: Cot::ActivationConfirmation,
		originator_address: asdu.originator_address,
		address_field: asdu.address_field,
		sequence: asdu.sequence,
		test: asdu.test,
		negative: false,
		information_objects: InformationObjects::CIcNa1(vec![GenericObject {
			address: go.address,
			object: CIcNa1 { qoi },
		}]),
	};
	server
		.send_asdu(connection_id, actcon)
		.await
		.map_err(|source| InterrogationError::SendConfirmation { source })?;

	let ca = asdu.address_field;
	for data_asdu in interrogation_data_asdus(ca, model) {
		server
			.send_asdu(connection_id, data_asdu)
			.await
			.map_err(|source| InterrogationError::SendData { source })?;
	}
	Ok(())
}

fn spontaneous_asdu(address: PointAddress, value: &PointValue) -> Asdu {
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

fn interrogation_data_asdus(ca: u16, model: &HashMap<PointAddress, PointValue>) -> Vec<Asdu> {
	let mut out = Vec::new();

	let mut sp: Vec<(u32, MSpNa1)> = Vec::new();
	let mut mv: Vec<(u32, MMeNc1)> = Vec::new();
	for (addr, v) in model.iter() {
		if addr.common_address != ca {
			continue;
		}
		let ioa = addr.information_object_address;
		match v {
			PointValue::MSpNa1(m) => sp.push((ioa, m.clone())),
			PointValue::MMeNc1(m) => mv.push((ioa, m.clone())),
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

	mv.sort_by_key(|(ioa, _)| *ioa);
	for chunk in mv.chunks(MAX_OBJECTS_PER_ASDU) {
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
