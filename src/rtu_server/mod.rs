//! High-level in-memory RTU-style server: typed points, Tokio message passing
//! into one actor task, built on [`crate::server::Server`] and
//! [`crate::server::ServerCallback`].
//!
//! v1 supports a small monitoring set (`M_SP_NA_1`, `M_ME_NC_1`) and general
//! interrogation activation (`C_IC_NA_1` → activation confirmation +
//! interrogation data on the same connection).

mod actor;
mod error;
mod model;

use std::collections::HashMap;

pub use error::{RtuHandleError, SetPointError};
pub use model::{PointAddress, PointValue};

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
	pub async fn start(
		config: ServerConfig,
		initial_points: impl IntoIterator<Item = (PointAddress, PointValue)>,
	) -> Result<RtuServerHandle, Error> {
		let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
		let ingress = actor::NetworkIngress::new(tx.clone());
		let server = crate::server::Server::start(config, ingress).await?;
		let model: HashMap<PointAddress, PointValue> = initial_points.into_iter().collect();
		let server_for_actor = server.clone();
		tokio::spawn(actor::run_actor(rx, server_for_actor, model));
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
}
