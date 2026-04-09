//! High-level in-memory RTU-style server: typed points, Tokio message passing
//! into one actor task, built on [`crate::server::Server`] and
//! [`crate::server::ServerCallback`].
//!
//! Monitoring types: `M_SP_NA_1`, `M_DP_NA_1`, `M_ST_NA_1`, `M_BO_NA_1`,
//! `M_ME_NA_1`, `M_ME_NB_1`, `M_ME_NC_1`.
//! General interrogation: `C_IC_NA_1` (activation → confirmation +
//! interrogation data on the same connection).
//!
//! **Commands** (`Cot::Request` / `Cot::Activation` for supported `C_SC_*`,
//! `C_DC_*`, `C_RC_*`, `C_SE_*`, `C_BO_*`): the standard defines the telegram,
//! not what it must do in your plant. You supply an [`RtuCommandHandler`] that
//! returns activation confirmation (echo / negative) and optional
//! [`CommandHandling::apply_updates`]. For a test-style “command IOA = monitor
//! IOA” mapping, see [`MapCommandsToSameIoaMonitoring`].
//!
//! **System ASDUs** (`C_TS_*`, `C_RD_NA_1`, `C_CS_NA_1`, `C_CI_NA_1`, …): build
//! [`RtuSystemHandlers`] (defaults or custom [`Arc`] per
//! [`RtuTestSystemHandler`], [`RtuReadSystemHandler`],
//! [`RtuClockSyncSystemHandler`], [`RtuCounterInterrogationHandler`]).
//! [`RtuServer::start`] uses [`RtuSystemHandlers::default`]; use
//! [`RtuServer::start_with_system_handlers`] to customize.
//!
//! The point set can be changed at runtime with
//! [`RtuServerHandle::register_point`], [`RtuServerHandle::register_points`],
//! [`RtuServerHandle::unregister_point`], and
//! [`RtuServerHandle::unregister_all`] (no spontaneous ASDU on
//! register/unregister; clients see changes on the next general interrogation
//! or after updates to remaining points).

mod actor;
mod command_handler;
mod command_presets;
mod commands;
mod error;
mod model;
mod output;
mod point_value;
mod system_command_handler;

use std::{collections::HashMap, sync::Arc};

pub use command_handler::{
	CommandContext, CommandHandling, RejectAllCommands, RtuCommandHandler, command_handler_from_fn,
};
pub use command_presets::MapCommandsToSameIoaMonitoring;
pub use error::{RtuHandleError, SetPointError};
pub use model::{PointAddress, PointValue};
pub use system_command_handler::{
	DefaultRtuClockSyncSystemHandler, DefaultRtuCounterInterrogationHandler,
	DefaultRtuReadSystemHandler, DefaultRtuTestSystemHandler, RtuClockSyncSystemHandler,
	RtuCounterInterrogationHandler, RtuReadSystemHandler, RtuSystemHandlers, RtuTestSystemHandler,
	SystemCommandContext,
};

use crate::{config::ServerConfig, error::Error};

/// Starts the low-level [`crate::server::Server`] and a single actor that owns
/// the point model.
///
/// Returns only a [`RtuServerHandle`]; the [`crate::server::Server`] handle is
/// kept inside the actor for outbound ASDUs.
#[derive(Debug)]
pub struct RtuServer(());

impl RtuServer {
	/// Bind and accept connections, spawn the model actor, and return a
	/// [`RtuServerHandle`].
	///
	/// Uses [`RtuSystemHandlers::default`] for test, read, clock sync, and
	/// counter interrogation ASDUs. Override via
	/// [`Self::start_with_system_handlers`].
	pub async fn start(
		config: ServerConfig,
		initial_points: impl IntoIterator<Item = (PointAddress, PointValue)>,
		command_handler: Arc<dyn RtuCommandHandler>,
	) -> Result<RtuServerHandle, Error> {
		Self::start_with_system_handlers(
			config,
			initial_points,
			command_handler,
			RtuSystemHandlers::default(),
		)
		.await
	}

	/// Like [`Self::start`], but supplies custom [`RtuSystemHandlers`] (e.g.
	/// replace one [`Arc`] field or use [`RtuSystemHandlers::with_read`]).
	pub async fn start_with_system_handlers(
		config: ServerConfig,
		initial_points: impl IntoIterator<Item = (PointAddress, PointValue)>,
		command_handler: Arc<dyn RtuCommandHandler>,
		system_handlers: RtuSystemHandlers,
	) -> Result<RtuServerHandle, Error> {
		let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
		let ingress = actor::NetworkIngress::new(tx.clone());
		let server = crate::server::Server::start(config, ingress).await?;
		let model: HashMap<PointAddress, PointValue> = initial_points.into_iter().collect();
		let server_for_actor = server.clone();
		tokio::spawn(actor::run_actor(
			rx,
			server_for_actor,
			model,
			command_handler,
			system_handlers,
		));
		Ok(RtuServerHandle { tx })
	}
}

/// `Clone` handle for updates from any task (e.g. NATS subscriber). Sends into
/// the actor via [`tokio::sync::mpsc::unbounded_channel`].
#[derive(Clone)]
pub struct RtuServerHandle {
	tx: tokio::sync::mpsc::UnboundedSender<actor::ActorMsg>,
}

impl std::fmt::Debug for RtuServerHandle {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str("RtuServerHandle")
	}
}

impl RtuServerHandle {
	/// Update an existing point and broadcast a spontaneous ASDU to all started
	/// connections.
	pub async fn set_point(
		&self,
		address: PointAddress,
		value: PointValue,
	) -> Result<(), RtuHandleError> {
		let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
		self.tx
			.send(actor::ActorMsg::SetPoint { address, value, reply: reply_tx })
			.map_err(|_| RtuHandleError::Disconnected)?;
		match reply_rx.await {
			Err(_) => Err(RtuHandleError::ActorStopped),
			Ok(r) => r.map_err(Into::into),
		}
	}

	/// Register a new point. Fails if the address is already present.
	///
	/// Does not broadcast; use [`Self::set_point`] to publish an initial value
	/// spontaneously, or rely on general interrogation. See also
	/// [`Self::unregister_point`]. For several points at once, see
	/// [`Self::register_points`].
	pub async fn register_point(
		&self,
		address: PointAddress,
		initial: PointValue,
	) -> Result<(), RtuHandleError> {
		let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
		self.tx
			.send(actor::ActorMsg::Register { address, initial, reply: reply_tx })
			.map_err(|_| RtuHandleError::Disconnected)?;
		match reply_rx.await {
			Err(_) => Err(RtuHandleError::ActorStopped),
			Ok(Ok(())) => Ok(()),
			Ok(Err(address)) => Err(RtuHandleError::AlreadyRegistered { address }),
		}
	}

	/// Remove a point from the model. Fails if the address is not present.
	///
	/// Like [`Self::register_point`], this does **not** send an ASDU; removed
	/// points simply stop appearing in the next general interrogation and no
	/// longer accept updates or command targets in the in-memory model.
	pub async fn unregister_point(&self, address: PointAddress) -> Result<(), RtuHandleError> {
		let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
		self.tx
			.send(actor::ActorMsg::Unregister { address, reply: reply_tx })
			.map_err(|_| RtuHandleError::Disconnected)?;
		match reply_rx.await {
			Err(_) => Err(RtuHandleError::ActorStopped),
			Ok(Ok(())) => Ok(()),
			Ok(Err(address)) => Err(RtuHandleError::NotRegistered { address }),
		}
	}

	/// Register several points in one actor turn. **All-or-nothing**: if any
	/// address is already in the model, or appears twice in `points`, nothing
	/// is inserted.
	///
	/// Does not broadcast. An empty iterator succeeds immediately.
	pub async fn register_points(
		&self,
		points: impl IntoIterator<Item = (PointAddress, PointValue)>,
	) -> Result<(), RtuHandleError> {
		let points: Vec<(PointAddress, PointValue)> = points.into_iter().collect();
		let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
		self.tx
			.send(actor::ActorMsg::RegisterPoints { points, reply: reply_tx })
			.map_err(|_| RtuHandleError::Disconnected)?;
		match reply_rx.await {
			Err(_) => Err(RtuHandleError::ActorStopped),
			Ok(Ok(())) => Ok(()),
			Ok(Err(actor::RegisterPointsError::DuplicateInInput)) => {
				Err(RtuHandleError::DuplicateAddressInInput)
			}
			Ok(Err(actor::RegisterPointsError::AlreadyInModel { address })) => {
				Err(RtuHandleError::AlreadyRegistered { address })
			}
		}
	}

	/// Remove every point from the model. Returns how many entries were
	/// removed.
	///
	/// Does not broadcast. [`Self::unregister_all`] on an empty model returns
	/// `Ok(0)`.
	pub async fn unregister_all(&self) -> Result<usize, RtuHandleError> {
		let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
		self.tx
			.send(actor::ActorMsg::UnregisterAll { reply: reply_tx })
			.map_err(|_| RtuHandleError::Disconnected)?;
		match reply_rx.await {
			Err(_) => Err(RtuHandleError::ActorStopped),
			Ok(n) => Ok(n),
		}
	}
}
