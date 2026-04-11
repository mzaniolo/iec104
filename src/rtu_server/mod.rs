//! High-level in-memory RTU-style server: typed points, Tokio message passing
//! into one actor task, built on [`crate::server::Server`] and
//! [`crate::server::ServerCallback`].
//!
//! Monitoring points cover IEC 60870-5-104 process information in Type ID range
//! 1–40 (see [`crate::types_id::TypeId`]: `M_SP_*`, `M_DP_*`, `M_ST_*`,
//! `M_BO_*`, `M_ME_*`, `M_IT_*`, `M_EP_*`, `M_PS_NA_1`, `M_ME_ND_1`, etc.).
//! Interrogation: `C_IC_NA_1` with QOI **20** (global) returns all points for
//! the ASDU common address with [`crate::cot::Cot::InterrogationGeneral`]; QOI
//! **21–36** (groups 1–16) returns only points registered with that group via
//! [`RtuInitialPoint::with_interrogation_group`] or
//! [`RtuServerHandle::register_point_with_interrogation_group`], with matching
//! [`crate::cot::Cot::InterrogationGroup1`] …
//! [`crate::cot::Cot::InterrogationGroup16`]. Unsupported custom QOI
//! ([`crate::types::commands::Qoi::Other`]) yields a negative activation
//! confirmation only (no data, no termination). Sequence: activation →
//! confirmation → interrogation data → activation termination when data is
//! supported.
//!
//! **Commands** (`Cot::Request` / `Cot::Activation` for supported `C_SC_*`,
//! `C_DC_*`, `C_RC_*`, `C_SE_*`, `C_BO_*`): the standard defines the telegram,
//! not what it must do in your plant. You supply an [`RtuCommandHandler`] that
//! returns activation confirmation (echo / negative) and optional
//! [`CommandHandling::apply_updates`]. For a test-style “command IOA = monitor
//! IOA” mapping, see [`MapCommandsToSameIoaMonitoring`].
//!
//! After each TCP connection completes **STARTDT**, the RTU sends
//! [`crate::types_id::TypeId::M_EI_NA_1`] (end of initialization) on that link
//! before other monitoring traffic (common address from the point model, or `0`
//! if empty).
//!
//! **System ASDUs** (`C_TS_*`, `C_RD_NA_1`, `C_CS_NA_1`, `C_CI_NA_1`, …): build
//! [`RtuSystemHandlers`] (defaults or custom [`Arc`] per
//! [`RtuTestSystemHandler`], [`RtuReadSystemHandler`],
//! [`RtuClockSyncSystemHandler`], [`RtuCounterInterrogationHandler`]).
//! [`RtuServer::start`] uses [`RtuSystemHandlers::default`]; use
//! [`RtuServer::start_with_system_handlers`] to customize.
//!
//! The point set can be changed at runtime with
//! [`RtuServerHandle::register_point`],
//! [`RtuServerHandle::register_point_with_interrogation_group`],
//! [`RtuServerHandle::register_points`], [`RtuServerHandle::unregister_point`],
//! and [`RtuServerHandle::unregister_all`] (no spontaneous ASDU on
//! register/unregister; clients see changes on the next interrogation
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

use std::{
	collections::{HashMap, hash_map::Entry},
	sync::Arc,
};

pub use command_handler::{
	CommandContext, CommandHandling, RejectAllCommands, RtuCommandHandler, command_handler_from_fn,
};
pub use command_presets::MapCommandsToSameIoaMonitoring;
pub use error::{RtuHandleError, SetPointError};
pub use model::{PointAddress, PointValue, RtuInitialMaps, RtuInitialPoint};
use snafu::whatever;
pub use system_command_handler::{
	DefaultRtuClockSyncSystemHandler, DefaultRtuCounterInterrogationHandler,
	DefaultRtuReadSystemHandler, DefaultRtuTestSystemHandler, RtuClockSyncSystemHandler,
	RtuCounterInterrogationHandler, RtuReadSystemHandler, RtuSystemHandlers, RtuTestSystemHandler,
	SystemCommandContext,
};

use crate::{config::ServerConfig, error::Error};

fn build_rtu_initial_maps(
	initial_points: impl IntoIterator<Item = impl Into<RtuInitialPoint>>,
) -> Result<RtuInitialMaps, Error> {
	let mut points = HashMap::new();
	let mut interrogation_groups = HashMap::new();
	for p in initial_points.into_iter().map(Into::into) {
		let RtuInitialPoint { address, value, interrogation_group } = p;
		if let Some(g) = interrogation_group
			&& !(1..=16).contains(&g)
		{
			whatever!(
				"invalid interrogation group {g} for RTU point CA {} IOA {}",
				address.common_address,
				address.information_object_address
			);
		}
		match points.entry(address) {
			Entry::Vacant(e) => {
				e.insert(value);
				if let Some(g) = interrogation_group {
					interrogation_groups.insert(address, g);
				}
			}
			Entry::Occupied(_) => {
				whatever!("duplicate RTU point address in start(): {address}");
			}
		}
	}
	Ok(RtuInitialMaps { points, interrogation_groups })
}

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
		initial_points: impl IntoIterator<Item = impl Into<RtuInitialPoint>>,
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
		initial_points: impl IntoIterator<Item = impl Into<RtuInitialPoint>>,
		command_handler: Arc<dyn RtuCommandHandler>,
		system_handlers: RtuSystemHandlers,
	) -> Result<RtuServerHandle, Error> {
		let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
		let ingress = actor::NetworkIngress::new(tx.clone());
		let server = crate::server::Server::start(config, ingress).await?;
		let RtuInitialMaps { points, interrogation_groups } =
			build_rtu_initial_maps(initial_points)?;
		let server_for_actor = server.clone();
		tokio::spawn(actor::run_actor(
			rx,
			server_for_actor,
			points,
			interrogation_groups,
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
			.send(actor::ActorMsg::Register {
				address,
				initial,
				interrogation_group: None,
				reply: reply_tx,
			})
			.map_err(|_| RtuHandleError::Disconnected)?;
		match reply_rx.await {
			Err(_) => Err(RtuHandleError::ActorStopped),
			Ok(Ok(())) => Ok(()),
			Ok(Err(actor::RegisterPointError::AlreadyRegistered(address))) => {
				Err(RtuHandleError::AlreadyRegistered { address })
			}
			Ok(Err(actor::RegisterPointError::InvalidInterrogationGroup { group })) => {
				Err(RtuHandleError::InvalidInterrogationGroup { group })
			}
		}
	}

	/// Like [`Self::register_point`], but assigns an IEC interrogation group
	/// **1..=16** so the point is included in group interrogation (`C_IC_NA_1`
	/// QOI 21–36) as well as in global interrogation (QOI 20).
	pub async fn register_point_with_interrogation_group(
		&self,
		address: PointAddress,
		initial: PointValue,
		interrogation_group: u8,
	) -> Result<(), RtuHandleError> {
		if !(1..=16).contains(&interrogation_group) {
			return Err(RtuHandleError::InvalidInterrogationGroup { group: interrogation_group });
		}
		let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
		self.tx
			.send(actor::ActorMsg::Register {
				address,
				initial,
				interrogation_group: Some(interrogation_group),
				reply: reply_tx,
			})
			.map_err(|_| RtuHandleError::Disconnected)?;
		match reply_rx.await {
			Err(_) => Err(RtuHandleError::ActorStopped),
			Ok(Ok(())) => Ok(()),
			Ok(Err(actor::RegisterPointError::AlreadyRegistered(address))) => {
				Err(RtuHandleError::AlreadyRegistered { address })
			}
			Ok(Err(actor::RegisterPointError::InvalidInterrogationGroup { group })) => {
				Err(RtuHandleError::InvalidInterrogationGroup { group })
			}
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
		points: impl IntoIterator<Item = impl Into<RtuInitialPoint>>,
	) -> Result<(), RtuHandleError> {
		let points: Vec<RtuInitialPoint> = points.into_iter().map(Into::into).collect();
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
			Ok(Err(actor::RegisterPointsError::InvalidInterrogationGroup { group })) => {
				Err(RtuHandleError::InvalidInterrogationGroup { group })
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
