//! Pluggable handling of standard **system** ASDUs (test, read, clock sync,
//! counter interrogation, reset process). Process commands (`C_SC_*`, `C_SE_*`)
//! stay on [`super::command_handler::RtuCommandHandler`].
//!
//! Compose [`RtuTestSystemHandler`], [`RtuReadSystemHandler`],
//! [`RtuClockSyncSystemHandler`], [`RtuCounterInterrogationHandler`], and
//! [`RtuResetProcessSystemHandler`] in [`RtuSystemHandlers`]. Swap individual
//! [`Arc`] fields for custom behaviour; use [`RtuSystemHandlers::default`] for
//! library defaults. The actor routes ingress ASDUs by type ID and calls the
//! matching handler.
//!
//! Default behaviour:
//! - **`C_TS_NA_1` / `C_TS_TA_1`**: positive activation confirmation echoing
//!   the test ASDU.
//! - **`C_RD_NA_1`**: positive confirmation + one monitoring ASDU per IOA when
//!   all addresses exist for the ASDU common address; otherwise negative
//!   confirmation (echo).
//! - **`C_CS_NA_1`**: positive confirmation echoing the time; stores the time
//!   in [`SystemCommandContext::last_master_clock`] for application use.
//! - **`C_CI_NA_1`**: activation confirmation → `M_IT_*` counter data (from
//!   in-memory integrated-total points and optional counter groups) →
//!   activation termination; supports [`Frz`](crate::types::commands::Frz)
//!   reset / freeze-and-reset on read addresses. Replace via
//!   [`RtuCounterInterrogationHandler`].
//! - **`C_RP_NA_1`**: positive activation confirmation echoing the reset
//!   command (what “reset” does in your plant is application-defined; override
//!   the handler to clear state, reconnect, etc.).

use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use super::{
	commands::send_command_confirmation,
	model::{PointAddress, PointValue},
	output::{
		counter_interrogation_data, counter_interrogation_data_cot, monitoring_asdu_requested,
	},
};
use crate::{
	asdu::Asdu,
	cot::Cot,
	server::{ConnectionId, Server, error::ServerError},
	types::{
		GenericObject, InformationObjects,
		commands::{CCiNa1, Frz, Rqt},
		time::Cp56Time2a,
	},
	types_id::TypeId,
};

/// [`Cot::Activation`] or [`Cot::Request`], as used for system commands in this
/// RTU.
#[must_use]
pub(super) const fn is_system_command_cot(cot: Cot) -> bool {
	matches!(cot, Cot::Activation | Cot::Request)
}

/// Context for system ASDU handlers: immutable model view plus optional clock
/// storage.
#[derive(Debug)]
pub struct SystemCommandContext<'a> {
	pub connection_id: ConnectionId,
	pub peer: SocketAddr,
	pub asdu: &'a Asdu,
	pub model: &'a HashMap<PointAddress, PointValue>,
	/// Last time accepted from a successful `C_CS_NA_1` (default handler
	/// overwrites on each accepted clock sync).
	pub last_master_clock: &'a mut Option<Cp56Time2a>,
}

/// Context for [`RtuCounterInterrogationHandler`]: mutable process image and
/// counter-group map (`C_CI_NA_1` RQT 1–4).
#[derive(Debug)]
pub struct CounterInterrogationContext<'a> {
	pub connection_id: ConnectionId,
	pub peer: SocketAddr,
	pub asdu: &'a Asdu,
	pub model: &'a mut HashMap<PointAddress, PointValue>,
	pub counter_groups: &'a HashMap<PointAddress, u8>,
}

/// `C_TS_NA_1` and `C_TS_TA_1` when [`Cot`] is [`Cot::Activation`] or
/// [`Cot::Request`].
#[async_trait::async_trait]
pub trait RtuTestSystemHandler: Send + Sync {
	async fn handle_test(
		&self,
		ctx: &mut SystemCommandContext<'_>,
		server: &Server,
	) -> Result<(), ServerError>;
}

/// `C_RD_NA_1` when [`Cot`] is [`Cot::Activation`] or [`Cot::Request`].
#[async_trait::async_trait]
pub trait RtuReadSystemHandler: Send + Sync {
	async fn handle_read(
		&self,
		ctx: &mut SystemCommandContext<'_>,
		server: &Server,
	) -> Result<(), ServerError>;
}

/// `C_CS_NA_1` when [`Cot`] is [`Cot::Activation`] or [`Cot::Request`].
#[async_trait::async_trait]
pub trait RtuClockSyncSystemHandler: Send + Sync {
	async fn handle_clock_sync(
		&self,
		ctx: &mut SystemCommandContext<'_>,
		server: &Server,
	) -> Result<(), ServerError>;
}

/// `C_CI_NA_1` when [`Cot`] is [`Cot::Activation`] or [`Cot::Request`].
#[async_trait::async_trait]
pub trait RtuCounterInterrogationHandler: Send + Sync {
	async fn handle_counter_interrogation(
		&self,
		ctx: &mut CounterInterrogationContext<'_>,
		server: &Server,
	) -> Result<(), ServerError>;
}

/// `C_RP_NA_1` when [`Cot`] is [`Cot::Activation`] or [`Cot::Request`].
#[async_trait::async_trait]
pub trait RtuResetProcessSystemHandler: Send + Sync {
	async fn handle_reset_process(
		&self,
		ctx: &mut SystemCommandContext<'_>,
		server: &Server,
	) -> Result<(), ServerError>;
}

/// Composes the five system ASDU groups. Replace individual [`Arc`] fields to
/// override only that behaviour; use [`Default`] for library defaults on the
/// rest.
#[derive(Clone)]
pub struct RtuSystemHandlers {
	pub test: Arc<dyn RtuTestSystemHandler>,
	pub read: Arc<dyn RtuReadSystemHandler>,
	pub clock_sync: Arc<dyn RtuClockSyncSystemHandler>,
	pub counter_interrogation: Arc<dyn RtuCounterInterrogationHandler>,
	pub reset_process: Arc<dyn RtuResetProcessSystemHandler>,
}

impl std::fmt::Debug for RtuSystemHandlers {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("RtuSystemHandlers").finish_non_exhaustive()
	}
}

impl Default for RtuSystemHandlers {
	fn default() -> Self {
		Self {
			test: Arc::new(DefaultRtuTestSystemHandler),
			read: Arc::new(DefaultRtuReadSystemHandler),
			clock_sync: Arc::new(DefaultRtuClockSyncSystemHandler),
			counter_interrogation: Arc::new(DefaultRtuCounterInterrogationHandler),
			reset_process: Arc::new(DefaultRtuResetProcessSystemHandler),
		}
	}
}

impl RtuSystemHandlers {
	#[must_use]
	pub fn with_test(mut self, test: Arc<dyn RtuTestSystemHandler>) -> Self {
		self.test = test;
		self
	}

	#[must_use]
	pub fn with_read(mut self, read: Arc<dyn RtuReadSystemHandler>) -> Self {
		self.read = read;
		self
	}

	#[must_use]
	pub fn with_clock_sync(mut self, clock_sync: Arc<dyn RtuClockSyncSystemHandler>) -> Self {
		self.clock_sync = clock_sync;
		self
	}

	#[must_use]
	pub fn with_counter_interrogation(
		mut self,
		counter_interrogation: Arc<dyn RtuCounterInterrogationHandler>,
	) -> Self {
		self.counter_interrogation = counter_interrogation;
		self
	}

	#[must_use]
	pub fn with_reset_process(
		mut self,
		reset_process: Arc<dyn RtuResetProcessSystemHandler>,
	) -> Self {
		self.reset_process = reset_process;
		self
	}
}

// --- Default implementations per trait ---

#[derive(Debug, Default, Clone, Copy)]
pub struct DefaultRtuTestSystemHandler;

#[async_trait::async_trait]
impl RtuTestSystemHandler for DefaultRtuTestSystemHandler {
	async fn handle_test(
		&self,
		ctx: &mut SystemCommandContext<'_>,
		server: &Server,
	) -> Result<(), ServerError> {
		let (type_id, information_objects) = match ctx.asdu.type_id {
			TypeId::C_TS_NA_1 => {
				let InformationObjects::CTsNa1(objs) = &ctx.asdu.information_objects else {
					return Ok(());
				};
				if objs.is_empty() {
					return Ok(());
				}
				(TypeId::C_TS_NA_1, InformationObjects::CTsNa1(objs.clone()))
			}
			TypeId::C_TS_TA_1 => {
				let InformationObjects::CTsTa1(objs) = &ctx.asdu.information_objects else {
					return Ok(());
				};
				if objs.is_empty() {
					return Ok(());
				}
				(TypeId::C_TS_TA_1, InformationObjects::CTsTa1(objs.clone()))
			}
			_ => return Ok(()),
		};
		send_command_confirmation(
			server,
			ctx.connection_id,
			ctx.asdu,
			type_id,
			information_objects,
			false,
		)
		.await?;
		Ok(())
	}
}

#[derive(Debug, Default, Clone, Copy)]
pub struct DefaultRtuReadSystemHandler;

#[async_trait::async_trait]
impl RtuReadSystemHandler for DefaultRtuReadSystemHandler {
	async fn handle_read(
		&self,
		ctx: &mut SystemCommandContext<'_>,
		server: &Server,
	) -> Result<(), ServerError> {
		let InformationObjects::CRdNa1(objs) = &ctx.asdu.information_objects else {
			return Ok(());
		};
		if objs.is_empty() {
			return Ok(());
		}

		let ca = ctx.asdu.address_field;
		let mut all_ok = true;
		for go in objs {
			let addr = PointAddress::new(ca, go.address);
			if !ctx.model.contains_key(&addr) {
				all_ok = false;
				break;
			}
		}

		let echo = InformationObjects::CRdNa1(objs.clone());
		send_command_confirmation(
			server,
			ctx.connection_id,
			ctx.asdu,
			TypeId::C_RD_NA_1,
			echo,
			!all_ok,
		)
		.await?;

		if !all_ok {
			return Ok(());
		}

		for go in objs {
			let addr = PointAddress::new(ca, go.address);
			let Some(pv) = ctx.model.get(&addr) else {
				continue;
			};
			let data = monitoring_asdu_requested(addr, pv);
			server.send_asdu(ctx.connection_id, data).await?;
		}
		Ok(())
	}
}

#[derive(Debug, Default, Clone, Copy)]
pub struct DefaultRtuClockSyncSystemHandler;

#[async_trait::async_trait]
impl RtuClockSyncSystemHandler for DefaultRtuClockSyncSystemHandler {
	async fn handle_clock_sync(
		&self,
		ctx: &mut SystemCommandContext<'_>,
		server: &Server,
	) -> Result<(), ServerError> {
		let InformationObjects::CCsNa1(objs) = &ctx.asdu.information_objects else {
			return Ok(());
		};
		let Some(go) = objs.first() else {
			return Ok(());
		};

		*ctx.last_master_clock = Some(go.object.time.clone());

		send_command_confirmation(
			server,
			ctx.connection_id,
			ctx.asdu,
			TypeId::C_CS_NA_1,
			InformationObjects::CCsNa1(objs.clone()),
			false,
		)
		.await?;
		Ok(())
	}
}

#[derive(Debug, Default, Clone, Copy)]
pub struct DefaultRtuCounterInterrogationHandler;

fn apply_counter_freeze_qualifier(
	model: &mut HashMap<PointAddress, PointValue>,
	read_addresses: &[PointAddress],
	frz: Frz,
) {
	if matches!(frz, Frz::Read | Frz::Freeze) {
		return;
	}
	for addr in read_addresses {
		let Some(pv) = model.get_mut(addr) else {
			continue;
		};
		match (pv, frz) {
			(PointValue::MItNa1(m), Frz::Reset | Frz::FreezeAndReset) => {
				m.bcr = 0;
			}
			(PointValue::MItTb1(m), Frz::Reset | Frz::FreezeAndReset) => {
				m.bcr = 0;
			}
			_ => {}
		}
	}
}

#[async_trait::async_trait]
impl RtuCounterInterrogationHandler for DefaultRtuCounterInterrogationHandler {
	async fn handle_counter_interrogation(
		&self,
		ctx: &mut CounterInterrogationContext<'_>,
		server: &Server,
	) -> Result<(), ServerError> {
		let InformationObjects::CCiNa1(objs) = &ctx.asdu.information_objects else {
			return Ok(());
		};
		let Some(go) = objs.first() else {
			return Ok(());
		};
		let rqt = go.object.rqt;
		let frz = go.object.frz;
		let co_ioa = go.address;

		if !matches!(ctx.asdu.cot, Cot::Activation | Cot::Request) {
			return Ok(());
		}

		let echo_ci = move || {
			InformationObjects::CCiNa1(vec![GenericObject {
				address: co_ioa,
				object: CCiNa1 { rqt, frz },
			}])
		};

		if matches!(rqt, Rqt::None | Rqt::Other(_)) {
			send_command_confirmation(
				server,
				ctx.connection_id,
				ctx.asdu,
				TypeId::C_CI_NA_1,
				echo_ci(),
				true,
			)
			.await?;
			return Ok(());
		}

		if counter_interrogation_data_cot(rqt).is_none() {
			return Ok(());
		}

		send_command_confirmation(
			server,
			ctx.connection_id,
			ctx.asdu,
			TypeId::C_CI_NA_1,
			echo_ci(),
			false,
		)
		.await?;

		let ca = ctx.asdu.address_field;
		let counter_read = counter_interrogation_data(ca, ctx.model, rqt, ctx.counter_groups);
		for data_asdu in counter_read.asdus {
			server.send_asdu(ctx.connection_id, data_asdu).await?;
		}

		apply_counter_freeze_qualifier(ctx.model, &counter_read.read_addresses, frz);

		let actterm = Asdu {
			type_id: TypeId::C_CI_NA_1,
			cot: Cot::ActivationTermination,
			originator_address: ctx.asdu.originator_address,
			address_field: ctx.asdu.address_field,
			sequence: ctx.asdu.sequence,
			test: ctx.asdu.test,
			negative: false,
			information_objects: echo_ci(),
		};
		server.send_asdu(ctx.connection_id, actterm).await?;
		Ok(())
	}
}

#[derive(Debug, Default, Clone, Copy)]
pub struct DefaultRtuResetProcessSystemHandler;

#[async_trait::async_trait]
impl RtuResetProcessSystemHandler for DefaultRtuResetProcessSystemHandler {
	async fn handle_reset_process(
		&self,
		ctx: &mut SystemCommandContext<'_>,
		server: &Server,
	) -> Result<(), ServerError> {
		let InformationObjects::CRpNa1(objs) = &ctx.asdu.information_objects else {
			return Ok(());
		};
		if objs.is_empty() {
			return Ok(());
		}
		send_command_confirmation(
			server,
			ctx.connection_id,
			ctx.asdu,
			TypeId::C_RP_NA_1,
			InformationObjects::CRpNa1(objs.clone()),
			false,
		)
		.await?;
		Ok(())
	}
}
