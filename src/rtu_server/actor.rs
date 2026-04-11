use std::{
	collections::{HashMap, HashSet},
	net::SocketAddr,
	sync::Arc,
};

use super::{
	command_handler::RtuCommandHandler,
	commands,
	error::{InterrogationError, SetPointError},
	model::{PointAddress, PointValue, RtuInitialPoint},
	output::{
		end_of_initialization_asdu, interrogation_data_asdus, spontaneous_asdu,
		station_common_address,
	},
	system_command_handler::{RtuSystemHandlers, SystemCommandContext, is_system_command_cot},
};
use crate::{
	asdu::Asdu,
	cot::Cot,
	server::{ConnectionId, Server, ServerCallback, error::ServerError},
	types::{CIcNa1, GenericObject, InformationObjects, commands::Qoi, time::Cp56Time2a},
	types_id::TypeId,
};

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
		interrogation_group: Option<u8>,
		reply: tokio::sync::oneshot::Sender<Result<(), RegisterPointError>>,
	},
	Unregister {
		address: PointAddress,
		reply: tokio::sync::oneshot::Sender<Result<(), PointAddress>>,
	},
	RegisterPoints {
		points: Vec<RtuInitialPoint>,
		reply: tokio::sync::oneshot::Sender<Result<(), RegisterPointsError>>,
	},
	UnregisterAll {
		reply: tokio::sync::oneshot::Sender<usize>,
	},
	/// TCP + STARTDT completed; send [`TypeId::M_EI_NA_1`] to this peer.
	ConnectionStarted {
		connection_id: ConnectionId,
	},
}

/// Failure for a bulk register (actor-internal; mapped to
/// [`super::error::RtuHandleError`]).
#[derive(Debug)]
pub(crate) enum RegisterPointsError {
	/// At least one [`PointAddress`] appears more than once in the batch.
	DuplicateInInput,
	AlreadyInModel {
		address: PointAddress,
	},
	InvalidInterrogationGroup {
		group: u8,
	},
}

/// Failure for a single [`ActorMsg::Register`].
#[derive(Debug)]
pub(crate) enum RegisterPointError {
	AlreadyRegistered(PointAddress),
	InvalidInterrogationGroup { group: u8 },
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

	async fn on_connection_started(&self, connection_id: ConnectionId, _address: SocketAddr) {
		let _ = self.tx.send(ActorMsg::ConnectionStarted { connection_id });
	}
}

pub(super) async fn run_actor(
	mut rx: tokio::sync::mpsc::UnboundedReceiver<ActorMsg>,
	server: Server,
	mut model: HashMap<PointAddress, PointValue>,
	mut interrogation_groups: HashMap<PointAddress, u8>,
	command_handler: Arc<dyn RtuCommandHandler>,
	system_handlers: RtuSystemHandlers,
) {
	let mut last_master_clock: Option<Cp56Time2a> = None;
	while let Some(msg) = rx.recv().await {
		match msg {
			ActorMsg::SetPoint { address, value, reply } => {
				let res = handle_set_point(&mut model, &server, address, value).await;
				let _ = reply.send(res);
			}
			ActorMsg::Register { address, initial, interrogation_group, reply } => {
				let res = if let Some(g) = interrogation_group {
					if !(1..=16).contains(&g) {
						Err(RegisterPointError::InvalidInterrogationGroup { group: g })
					} else {
						register_one_point(
							&mut model,
							&mut interrogation_groups,
							address,
							initial,
							Some(g),
						)
					}
				} else {
					register_one_point(
						&mut model,
						&mut interrogation_groups,
						address,
						initial,
						None,
					)
				};
				let _ = reply.send(res);
			}
			ActorMsg::Unregister { address, reply } => {
				let res = match model.remove(&address) {
					Some(_) => {
						interrogation_groups.remove(&address);
						Ok(())
					}
					None => Err(address),
				};
				let _ = reply.send(res);
			}
			ActorMsg::RegisterPoints { points, reply } => {
				let res = try_register_points(&mut model, &mut interrogation_groups, points);
				let _ = reply.send(res);
			}
			ActorMsg::UnregisterAll { reply } => {
				let n = model.len();
				model.clear();
				interrogation_groups.clear();
				let _ = reply.send(n);
			}
			ActorMsg::IngressAsdu { asdu, connection_id, peer } => {
				handle_ingress_asdu(
					&mut model,
					&interrogation_groups,
					&server,
					&command_handler,
					&system_handlers,
					&mut last_master_clock,
					asdu,
					connection_id,
					peer,
				)
				.await;
			}
			ActorMsg::ConnectionStarted { connection_id } => {
				send_end_of_initialization(&model, &server, connection_id).await;
			}
		}
	}
	tracing::warn!("RTU actor channel closed; stopping model loop");
}

fn register_one_point(
	model: &mut HashMap<PointAddress, PointValue>,
	interrogation_groups: &mut HashMap<PointAddress, u8>,
	address: PointAddress,
	initial: PointValue,
	interrogation_group: Option<u8>,
) -> Result<(), RegisterPointError> {
	use std::collections::hash_map::Entry;
	match model.entry(address) {
		Entry::Vacant(e) => {
			e.insert(initial);
			if let Some(g) = interrogation_group {
				interrogation_groups.insert(address, g);
			}
			Ok(())
		}
		Entry::Occupied(e) => Err(RegisterPointError::AlreadyRegistered(*e.key())),
	}
}

fn try_register_points(
	model: &mut HashMap<PointAddress, PointValue>,
	interrogation_groups: &mut HashMap<PointAddress, u8>,
	points: Vec<RtuInitialPoint>,
) -> Result<(), RegisterPointsError> {
	if points.is_empty() {
		return Ok(());
	}
	let mut seen = HashSet::with_capacity(points.len());
	for p in &points {
		if let Some(g) = p.interrogation_group
			&& !(1..=16).contains(&g)
		{
			return Err(RegisterPointsError::InvalidInterrogationGroup { group: g });
		}
		if !seen.insert(p.address) {
			return Err(RegisterPointsError::DuplicateInInput);
		}
		if model.contains_key(&p.address) {
			return Err(RegisterPointsError::AlreadyInModel { address: p.address });
		}
	}
	for p in points {
		model.insert(p.address, p.value);
		if let Some(g) = p.interrogation_group {
			interrogation_groups.insert(p.address, g);
		}
	}
	Ok(())
}

async fn send_end_of_initialization(
	model: &HashMap<PointAddress, PointValue>,
	server: &Server,
	connection_id: ConnectionId,
) {
	let ca = station_common_address(model);
	let asdu = end_of_initialization_asdu(ca);
	if let Err(e) = server.send_asdu(connection_id, asdu).await {
		tracing::warn!(
			error = ?e,
			?connection_id,
			"failed to send M_EI_NA_1 (end of initialization)"
		);
	}
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

async fn dispatch_rtu_system_handler(
	handlers: &RtuSystemHandlers,
	type_id: TypeId,
	ctx: &mut SystemCommandContext<'_>,
	server: &Server,
) -> Result<(), ServerError> {
	match type_id {
		TypeId::C_TS_NA_1 | TypeId::C_TS_TA_1 => handlers.test.handle_test(ctx, server).await,
		TypeId::C_RD_NA_1 => handlers.read.handle_read(ctx, server).await,
		TypeId::C_CS_NA_1 => handlers.clock_sync.handle_clock_sync(ctx, server).await,
		TypeId::C_CI_NA_1 => {
			handlers.counter_interrogation.handle_counter_interrogation(ctx, server).await
		}
		TypeId::C_RP_NA_1 => handlers.reset_process.handle_reset_process(ctx, server).await,
		_ => Ok(()),
	}
}

#[allow(clippy::too_many_arguments)]
async fn handle_ingress_asdu(
	model: &mut HashMap<PointAddress, PointValue>,
	interrogation_groups: &HashMap<PointAddress, u8>,
	server: &Server,
	command_handler: &Arc<dyn RtuCommandHandler>,
	system_handlers: &RtuSystemHandlers,
	last_master_clock: &mut Option<Cp56Time2a>,
	asdu: Asdu,
	connection_id: ConnectionId,
	peer: SocketAddr,
) {
	let sys_cot = is_system_command_cot(asdu.cot);

	match asdu.type_id {
		_ if commands::is_process_command(&asdu) => {
			if let Err(e) = commands::handle_process_command(
				model,
				server,
				&asdu,
				connection_id,
				peer,
				command_handler,
			)
			.await
			{
				tracing::error!(error = ?e, ?peer, "process command handling");
			}
		}
		tid @ (TypeId::C_TS_NA_1
		| TypeId::C_TS_TA_1
		| TypeId::C_RD_NA_1
		| TypeId::C_CS_NA_1
		| TypeId::C_CI_NA_1
		| TypeId::C_RP_NA_1)
			if sys_cot =>
		{
			let mut ctx =
				SystemCommandContext { connection_id, peer, asdu: &asdu, model, last_master_clock };
			if let Err(e) =
				dispatch_rtu_system_handler(system_handlers, tid, &mut ctx, server).await
			{
				tracing::error!(error = ?e, ?peer, type_id = ?tid, "system command handling");
			}
		}
		TypeId::C_IC_NA_1 => {
			match handle_interrogation(model, interrogation_groups, server, &asdu, connection_id)
				.await
			{
				Ok(()) => {}
				Err(InterrogationError::Skipped) => {
					tracing::trace!(?peer, type_id = ?asdu.type_id, "C_IC_NA_1 skipped");
				}
				Err(e) => tracing::error!(error = %e, ?peer, "interrogation handling"),
			}
		}
		_ => {
			tracing::trace!(?peer, type_id = ?asdu.type_id, "ingress ASDU (no handler)");
		}
	}
}

async fn handle_interrogation(
	model: &HashMap<PointAddress, PointValue>,
	interrogation_groups: &HashMap<PointAddress, u8>,
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

	if matches!(qoi, Qoi::Other(_)) {
		let actcon = Asdu {
			type_id: TypeId::C_IC_NA_1,
			cot: Cot::ActivationConfirmation,
			originator_address: asdu.originator_address,
			address_field: asdu.address_field,
			sequence: asdu.sequence,
			test: asdu.test,
			negative: true,
			information_objects: InformationObjects::CIcNa1(vec![GenericObject {
				address: go.address,
				object: CIcNa1 { qoi },
			}]),
		};
		server
			.send_asdu(connection_id, actcon)
			.await
			.map_err(|source| InterrogationError::SendConfirmation { source })?;
		return Ok(());
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
	for data_asdu in interrogation_data_asdus(ca, model, qoi, interrogation_groups) {
		server
			.send_asdu(connection_id, data_asdu)
			.await
			.map_err(|source| InterrogationError::SendData { source })?;
	}

	let actterm = Asdu {
		type_id: TypeId::C_IC_NA_1,
		cot: Cot::ActivationTermination,
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
		.send_asdu(connection_id, actterm)
		.await
		.map_err(|source| InterrogationError::SendTermination { source })?;
	Ok(())
}
