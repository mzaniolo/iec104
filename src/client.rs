use std::{
	fmt::Debug,
	sync::{Arc, atomic::AtomicBool},
};

use async_trait::async_trait;
use snafu::{ResultExt, whatever};
use tokio::{sync::mpsc, task::JoinHandle};
use tracing::instrument;

use crate::{
	Connection,
	asdu::Asdu,
	client::{
		connection_handler::{ConnectionHandler, ConnectionHandlerState},
		errors::ClientError,
	},
	config::ClientConfig,
	cot::Cot,
	error::Error,
	receive_handler::ReceiveHandlerCallback,
	types::{
		CBoNa1, CBoTa1, CScNa1, CScTa1, CdcNa1, CdcTa1, CrcNa1, CrcTa1, GenericObject,
		InformationObjects,
		commands::{Dco, Qu, Rco, Rcs, Sco},
		information_elements::{Dpi, SelectExecute, Spi},
		time::Cp56Time2a,
	},
	types_id::TypeId,
};

mod connection_handler;
pub mod errors;

use connection_handler::AtomicConnectionHandlerState;

use crate::receive_handler::{ReceiveHandlerCommand, SendAsduQueueError};

#[async_trait]
pub trait ClientCallback {
	async fn on_new_objects(&self, asdu: Asdu);

	async fn on_connection_started(&self) {
		tracing::debug!("Connection started");
	}
	async fn on_connection_stopped(&self) {
		tracing::debug!("Connection stopped");
	}
	async fn on_error(&self, error: Error) {
		tracing::debug!("Error: {error}");
	}
	async fn on_reconnecting(&self) {
		tracing::debug!("Reconnecting");
	}
}

#[derive(Debug)]
struct InnerClientCallback<C: ClientCallback + Send + Sync> {
	callback: C,
}

#[async_trait::async_trait]
impl<C: ClientCallback + Send + Sync> ReceiveHandlerCallback for InnerClientCallback<C> {
	async fn on_new_objects(&self, asdu: Asdu) {
		self.callback.on_new_objects(asdu).await;
	}
}

impl<C: ClientCallback + Send + Sync> InnerClientCallback<C> {
	async fn on_connection_started(&self) {
		self.callback.on_connection_started().await;
	}
	async fn on_connection_stopped(&self) {
		self.callback.on_connection_stopped().await;
	}
	async fn on_error(&self, error: Error) {
		self.callback.on_error(error).await;
	}
	async fn on_reconnecting(&self) {
		self.callback.on_reconnecting().await;
	}
}

pub struct Client<C: ClientCallback + Send + Sync + 'static> {
	config: ClientConfig,
	callback: Arc<InnerClientCallback<C>>,
	receive_task: Option<JoinHandle<Result<(), Error>>>,
	write_tx: Option<mpsc::Sender<ReceiveHandlerCommand>>,
	out_buffer_full: Arc<AtomicBool>,
	connection_handler_state: Option<Arc<AtomicConnectionHandlerState>>,
}

impl<C: ClientCallback + Send + Sync + 'static> Client<C> {
	#[must_use]
	pub fn new(config: ClientConfig, callback: C) -> Self {
		Self {
			config,
			callback: Arc::new(InnerClientCallback { callback }),
			receive_task: None,
			write_tx: None,
			out_buffer_full: Arc::new(AtomicBool::new(false)),
			connection_handler_state: None,
		}
	}

	#[instrument(level = "debug")]
	pub async fn connect(&mut self) -> Result<(), Error> {
		// Reject spec-violating k/w combinations (IEC 60870-5-104 §5.2)
		// before opening the TCP connection so misconfiguration fails fast.
		self.config.protocol.validate().whatever_context("Invalid protocol configuration")?;

		if self.receive_task.is_some() {
			whatever!("Receive task already running");
		}

		let (tx, rx) = mpsc::channel(1024);

		let callback = self.callback.clone();
		let config = self.config.clone();
		let out_buffer_full = self.out_buffer_full.clone();

		let mut connection_handler =
			ConnectionHandler::new(callback, config, rx, out_buffer_full).await?;

		self.connection_handler_state = Some(connection_handler.get_state());

		self.receive_task = Some(tokio::spawn(async move {
			connection_handler
				.run()
				.await
				.inspect_err(|e| tracing::error!("Error in running connection handler: {e}"))
		}));

		self.write_tx = Some(tx);

		Ok(())
	}

	#[instrument(level = "debug")]
	pub async fn send_asdu(&self, asdu: Asdu) -> Result<(), ClientError> {
		self.check_connection_started()?;

		if self.out_buffer_full.load(std::sync::atomic::Ordering::Relaxed) {
			return errors::OutputBufferFull.fail();
		}

		if let Some(tx) = &self.write_tx {
			let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
			tx.send(ReceiveHandlerCommand::Asdu { asdu, reply: reply_tx })
				.await
				.context(errors::SendCommand)?;
			match reply_rx.await.context(errors::ReceiveHandlerAsduAck)? {
				Ok(()) => Ok(()),
				Err(SendAsduQueueError::PendingBufferFull) => errors::OutputBufferFull.fail(),
			}
		} else {
			errors::NoWriteChannel.fail()
		}
	}

	#[instrument(level = "debug")]
	pub async fn start_receiving(&mut self) -> Result<(), ClientError> {
		self.check_connected()?;

		if let Some(state) = &self.connection_handler_state
			&& state.load(std::sync::atomic::Ordering::Relaxed)
				!= ConnectionHandlerState::WaitingForStart
		{
			return errors::AlreadyStarted.fail();
		}

		if let Some(tx) = &self.write_tx {
			tx.send(ReceiveHandlerCommand::Start).await.context(errors::SendCommand)?;
		} else {
			return errors::NoWriteChannel.fail();
		}
		Ok(())
	}

	#[instrument(level = "debug")]
	pub async fn stop_receiving(&mut self) -> Result<(), ClientError> {
		self.check_connection_started()?;

		if let Some(tx) = &self.write_tx {
			tx.send(ReceiveHandlerCommand::Stop).await.context(errors::SendCommand)?;
		} else {
			return errors::NoWriteChannel.fail();
		}
		Ok(())
	}

	#[instrument(level = "debug")]
	pub async fn send_test_frame(&mut self) -> Result<(), ClientError> {
		self.check_connection_started()?;

		if let Some(tx) = &self.write_tx {
			tx.send(ReceiveHandlerCommand::Test).await.context(errors::SendCommand)?;
		} else {
			return errors::NoWriteChannel.fail();
		}
		Ok(())
	}

	pub async fn send_command_sp(
		&self,
		common_address: u16,
		ioa: u32,
		value: Spi,
		timestamp: Option<Cp56Time2a>,
		select_execute: Option<SelectExecute>,
		qu: Option<Qu>,
	) -> Result<(), ClientError> {
		let sco = Sco {
			se: select_execute.unwrap_or(SelectExecute::Execute),
			qu: qu.unwrap_or(Qu::Unspecified),
			scs: value,
		};
		let (type_id, information_objects) = match timestamp {
			Some(timestamp) => (
				TypeId::C_SC_TA_1,
				InformationObjects::CScTa1(vec![GenericObject {
					address: ioa,
					object: CScTa1 { sco, time: timestamp },
				}]),
			),
			None => (
				TypeId::C_SC_NA_1,
				InformationObjects::CScNa1(vec![GenericObject {
					address: ioa,
					object: CScNa1 { sco },
				}]),
			),
		};

		self.send_asdu(Asdu {
			type_id,
			information_objects,
			originator_address: self.config.protocol.originator_address,
			address_field: common_address,
			sequence: false,
			test: false,
			cot: Cot::Activation,
			negative: false,
		})
		.await
	}

	pub async fn send_command_dp(
		&self,
		common_address: u16,
		ioa: u32,
		value: Dpi,
		timestamp: Option<Cp56Time2a>,
		select_execute: Option<SelectExecute>,
		qu: Option<Qu>,
	) -> Result<(), ClientError> {
		let dco = Dco {
			se: select_execute.unwrap_or(SelectExecute::Execute),
			qu: qu.unwrap_or(Qu::Unspecified),
			dcs: value,
		};
		let (type_id, information_objects) = match timestamp {
			Some(timestamp) => (
				TypeId::C_DC_TA_1,
				InformationObjects::CdcTa1(vec![GenericObject {
					address: ioa,
					object: CdcTa1 { dco, time: timestamp },
				}]),
			),
			None => (
				TypeId::C_DC_NA_1,
				InformationObjects::CdcNa1(vec![GenericObject {
					address: ioa,
					object: CdcNa1 { dco },
				}]),
			),
		};

		self.send_asdu(Asdu {
			type_id,
			information_objects,
			originator_address: self.config.protocol.originator_address,
			address_field: common_address,
			sequence: false,
			test: false,
			cot: Cot::Activation,
			negative: false,
		})
		.await
	}

	pub async fn send_command_rc(
		&self,
		common_address: u16,
		ioa: u32,
		value: Rcs,
		timestamp: Option<Cp56Time2a>,
		select_execute: Option<SelectExecute>,
		qu: Option<Qu>,
	) -> Result<(), ClientError> {
		let rco = Rco {
			se: select_execute.unwrap_or(SelectExecute::Execute),
			qu: qu.unwrap_or(Qu::Unspecified),
			rcs: value,
		};
		let (type_id, information_objects) = match timestamp {
			Some(timestamp) => (
				TypeId::C_RC_TA_1,
				InformationObjects::CrcTa1(vec![GenericObject {
					address: ioa,
					object: CrcTa1 { rco, time: timestamp },
				}]),
			),
			None => (
				TypeId::C_RC_NA_1,
				InformationObjects::CrcNa1(vec![GenericObject {
					address: ioa,
					object: CrcNa1 { rco },
				}]),
			),
		};

		self.send_asdu(Asdu {
			type_id,
			information_objects,
			originator_address: self.config.protocol.originator_address,
			address_field: common_address,
			sequence: false,
			test: false,
			cot: Cot::Activation,
			negative: false,
		})
		.await
	}

	pub async fn send_command_bs(
		&self,
		common_address: u16,
		ioa: u32,
		value: u32,
		timestamp: Option<Cp56Time2a>,
	) -> Result<(), ClientError> {
		let (type_id, information_objects) = match timestamp {
			Some(timestamp) => (
				TypeId::C_BO_TA_1,
				InformationObjects::CBoTa1(vec![GenericObject {
					address: ioa,
					object: CBoTa1 { bsi: value, time: timestamp },
				}]),
			),
			None => (
				TypeId::C_BO_NA_1,
				InformationObjects::CBoNa1(vec![GenericObject {
					address: ioa,
					object: CBoNa1 { bsi: value },
				}]),
			),
		};

		self.send_asdu(Asdu {
			type_id,
			information_objects,
			originator_address: self.config.protocol.originator_address,
			address_field: common_address,
			sequence: false,
			test: false,
			cot: Cot::Activation,
			negative: false,
		})
		.await
	}

	#[instrument(level = "debug")]
	fn check_connection_started(&self) -> Result<(), ClientError> {
		self.check_connected()?;

		if let Some(state) = &self.connection_handler_state
			&& state.load(std::sync::atomic::Ordering::Relaxed) != ConnectionHandlerState::Started
		{
			return errors::NotReceiving.fail();
		}

		Ok(())
	}

	#[instrument(level = "debug")]
	fn check_connected(&self) -> Result<(), ClientError> {
		if self.connection_handler_state.is_none() {
			return errors::NotConnected.fail();
		}
		if let Some(state) = &self.connection_handler_state
			&& state.load(std::sync::atomic::Ordering::Relaxed)
				== ConnectionHandlerState::Reconnecting
		{
			return errors::Reconnecting.fail();
		}

		Ok(())
	}
}

impl<C: ClientCallback + Send + Sync + 'static> Debug for Client<C> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"Client {{state: {:?} }}",
			self.connection_handler_state
				.as_ref()
				.map(|state| state.load(std::sync::atomic::Ordering::Relaxed))
		)
	}
}
