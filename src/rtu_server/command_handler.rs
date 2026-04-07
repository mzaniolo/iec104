//! User-defined interpretation of incoming command ASDUs.
//!
//! The protocol defines command telegrams (e.g. single command, set-point
//! command) with a type identifier and information object address, but **not**
//! which process variables or outputs they must drive. That mapping is
//! **application / engineering** responsibility; compare IEC 60870-5-104
//! and typical RTU/SCADA integration where the same IOA may mean different
//! things per project.

use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use super::model::{PointAddress, PointValue};
use crate::{asdu::Asdu, server::ConnectionId, types::InformationObjects, types_id::TypeId};

/// Read-only view passed to [`RtuCommandHandler::handle_command`].
#[derive(Debug)]
pub struct CommandContext<'a> {
	/// Connection the ASDU arrived on (e.g. for per-master logic).
	pub connection_id: ConnectionId,
	/// Peer address (if needed for logging or policy).
	pub peer: SocketAddr,
	/// Full incoming ASDU (`type_id`, `address_field` = common address,
	/// `information_objects`, …).
	pub asdu: &'a Asdu,
	/// Current in-memory monitoring points (immutable for the duration of the
	/// callback).
	pub model: &'a HashMap<PointAddress, PointValue>,
}

/// Result of handling one command ASDU: activation confirmation payload and
/// optional model updates.
#[derive(Debug, Clone)]
pub struct CommandHandling {
	/// `true` = negative confirm (e.g. unknown command or interlock).
	pub negative: bool,
	/// Usually the same as the incoming command `type_id`.
	pub reply_type_id: TypeId,
	/// Objects mirrored in the activation confirmation (typically echo of the
	/// command).
	pub reply_information_objects: InformationObjects,
	/// When [`Self::negative`] is `false`, each pair is applied like
	/// [`super::RtuServerHandle::set_point`] (type must match the existing
	/// point); invalid entries are skipped with a warning.
	pub apply_updates: Vec<(PointAddress, PointValue)>,
}

#[async_trait::async_trait]
pub trait RtuCommandHandler: Send + Sync {
	/// Decide confirmation and any process-image updates for this command ASDU.
	async fn handle_command(&self, ctx: CommandContext<'_>) -> CommandHandling;
}

/// Always answers with **negative** activation confirmation and echoes the
/// command objects. Use when commands are not supported or until you plug in
/// real logic.
#[derive(Debug, Default, Clone, Copy)]
pub struct RejectAllCommands;

#[async_trait::async_trait]
impl RtuCommandHandler for RejectAllCommands {
	async fn handle_command(&self, ctx: CommandContext<'_>) -> CommandHandling {
		CommandHandling {
			negative: true,
			reply_type_id: ctx.asdu.type_id,
			reply_information_objects: ctx.asdu.information_objects.clone(),
			apply_updates: Vec::new(),
		}
	}
}

/// Wraps a closure as a handler (e.g. for tests or small binaries).
pub struct FnCommandHandler<F> {
	f: F,
}

impl<F> FnCommandHandler<F> {
	pub const fn new(f: F) -> Self {
		Self { f }
	}
}

#[async_trait::async_trait]
impl<F> RtuCommandHandler for FnCommandHandler<F>
where
	F: Fn(CommandContext<'_>) -> CommandHandling + Send + Sync,
{
	async fn handle_command(&self, ctx: CommandContext<'_>) -> CommandHandling {
		(self.f)(ctx)
	}
}

/// Build an [`Arc<dyn RtuCommandHandler>`] from a sync closure (no `.await`
/// inside the closure).
pub fn command_handler_from_fn(
	f: impl Fn(CommandContext<'_>) -> CommandHandling + Send + Sync + 'static,
) -> Arc<dyn RtuCommandHandler> {
	Arc::new(FnCommandHandler::new(f))
}
