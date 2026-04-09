//! Dispatch incoming command ASDUs to
//! [`super::command_handler::RtuCommandHandler`], apply returned model updates,
//! send activation confirmation.

use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use super::{
	command_handler::{CommandContext, RtuCommandHandler},
	model::{PointAddress, PointValue},
	output::spontaneous_asdu,
};
use crate::{
	asdu::Asdu,
	cot::Cot,
	server::{ConnectionId, Server, error::ServerError},
	types_id::TypeId,
};

#[must_use]
const fn is_command_cot(cot: Cot) -> bool {
	matches!(cot, Cot::Request | Cot::Activation)
}

/// Process commands (`C_SC_*`, `C_DC_*`, `C_RC_*`, `C_SE_*`, `C_BO_*`) with
/// [`Cot::Request`] or [`Cot::Activation`].
#[must_use]
pub(super) const fn is_process_command(asdu: &Asdu) -> bool {
	is_command_cot(asdu.cot) && is_supported_command_type(asdu.type_id)
}

#[must_use]
pub(super) const fn is_supported_command_type(type_id: TypeId) -> bool {
	matches!(
		type_id,
		TypeId::C_SC_NA_1
			| TypeId::C_DC_NA_1
			| TypeId::C_RC_NA_1
			| TypeId::C_SE_NA_1
			| TypeId::C_SE_NB_1
			| TypeId::C_SE_NC_1
			| TypeId::C_BO_NA_1
			| TypeId::C_SC_TA_1
			| TypeId::C_DC_TA_1
			| TypeId::C_RC_TA_1
			| TypeId::C_SE_TA_1
			| TypeId::C_SE_TB_1
			| TypeId::C_SE_TC_1
			| TypeId::C_BO_TA_1
	)
}

const fn command_confirmation_asdu(
	incoming: &Asdu,
	type_id: TypeId,
	information_objects: crate::types::InformationObjects,
	negative: bool,
) -> Asdu {
	Asdu {
		type_id,
		cot: Cot::ActivationConfirmation,
		originator_address: incoming.originator_address,
		address_field: incoming.address_field,
		sequence: incoming.sequence,
		test: incoming.test,
		negative,
		information_objects,
	}
}

pub(super) async fn send_activation_confirmation(
	server: &Server,
	connection_id: ConnectionId,
	incoming: &Asdu,
	type_id: TypeId,
	objs: crate::types::InformationObjects,
	negative: bool,
) -> Result<(), ServerError> {
	let reply = command_confirmation_asdu(incoming, type_id, objs, negative);
	server.send_asdu(connection_id, reply).await
}

fn can_apply_update(
	model: &HashMap<PointAddress, PointValue>,
	addr: PointAddress,
	val: &PointValue,
) -> bool {
	model.get(&addr).is_some_and(|ex| ex.type_id() == val.type_id())
}

async fn apply_model_updates(
	model: &mut HashMap<PointAddress, PointValue>,
	server: &Server,
	updates: &[(PointAddress, PointValue)],
) -> Result<(), ServerError> {
	for (addr, val) in updates {
		if !can_apply_update(model, *addr, val) {
			tracing::warn!(
				%addr,
				"command apply_updates: unknown point or type mismatch; skipping this entry"
			);
			continue;
		}
		model.insert(*addr, val.clone());
		server.broadcast_asdu(spontaneous_asdu(*addr, val)).await?;
	}
	Ok(())
}

/// Handle a process command ASDU. Call only when [`is_process_command`] is
/// true.
pub(super) async fn handle_process_command(
	model: &mut HashMap<PointAddress, PointValue>,
	server: &Server,
	asdu: &Asdu,
	connection_id: ConnectionId,
	peer: SocketAddr,
	handler: &Arc<dyn RtuCommandHandler>,
) -> Result<(), ServerError> {
	let handling = {
		let ctx = CommandContext { connection_id, peer, asdu, model: &*model };
		handler.handle_command(ctx).await
	};

	if !handling.negative {
		apply_model_updates(model, server, &handling.apply_updates).await?;
	}

	send_activation_confirmation(
		server,
		connection_id,
		asdu,
		handling.reply_type_id,
		handling.reply_information_objects,
		handling.negative,
	)
	.await?;
	Ok(())
}
