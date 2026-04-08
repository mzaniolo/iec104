use std::{
	net::SocketAddr,
	sync::{Arc, atomic::AtomicBool},
};

use atomic_enum::atomic_enum;
use snafu::ResultExt as _;
use tokio::{
	io::{ReadHalf, WriteHalf},
	sync::{mpsc, oneshot},
};

use super::ConnectionId;
use crate::{
	Connection, START_DT_CON_FRAME,
	apdu::Frame,
	asdu::Asdu,
	config::ServerConfig,
	error::Error,
	receive_handler::{
		ReceiveHandler, ReceiveHandlerCommand, SendAsduQueueError, receive_apdu, send_frame,
	},
	server::{InnerServerCallback, ServerCallback, error},
};

#[atomic_enum]
#[derive(PartialEq)]
pub enum ConnectionState {
	WaitingStartDT,
	Started,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct ConnectionHandler {
	pub id: ConnectionId,
	pub address: SocketAddr,
	pub _task: tokio::task::JoinHandle<()>,
	pub tx: mpsc::Sender<ReceiveHandlerCommand>,
	pub state: Arc<AtomicConnectionState>,
}

struct InnerConnectionHandler<C: ServerCallback + Send + Sync + 'static> {
	read_connection: ReadHalf<Connection>,
	write_connection: WriteHalf<Connection>,
	callback: Arc<InnerServerCallback<C>>,
	config: ServerConfig,
	rx: mpsc::Receiver<ReceiveHandlerCommand>,
	id: ConnectionId,
	address: SocketAddr,
	state: Arc<AtomicConnectionState>,
}

impl ConnectionHandler {
	pub fn start<C: ServerCallback + Send + Sync + 'static>(
		id: ConnectionId,
		address: SocketAddr,
		connection: Connection,
		config: ServerConfig,
		callback: InnerServerCallback<C>,
		tx: mpsc::UnboundedSender<ConnectionId>,
	) -> Self {
		let (read_connection, write_connection) = tokio::io::split(connection);
		let (cmd_tx, rx) = mpsc::channel(1024);
		let state = Arc::new(AtomicConnectionState::new(ConnectionState::WaitingStartDT));
		let inner_connection_handler = InnerConnectionHandler {
			read_connection,
			write_connection,
			callback: Arc::new(callback),
			config,
			rx,
			id,
			address,
			state: state.clone(),
		};
		let task = tokio::spawn(async move {
			let _ = inner_connection_handler
				.start()
				.await
				.inspect_err(|e| tracing::error!("Error in running connection handler: {e}"));
			let _ = tx.send(id);
		});
		Self { id, address, _task: task, tx: cmd_tx, state }
	}
	pub async fn send_asdu(&self, asdu: Asdu) -> Result<(), error::ServerError> {
		let (reply_tx, reply_rx) = oneshot::channel();
		self.tx
			.send(ReceiveHandlerCommand::Asdu { asdu, reply: reply_tx })
			.await
			.context(error::SendConnectionCommand)?;
		match reply_rx.await.context(error::ReceiveHandlerAsduAck)? {
			Ok(()) => Ok(()),
			Err(SendAsduQueueError::PendingBufferFull) => {
				error::PendingOutgoingAsduBufferFull.fail()
			}
		}
	}
	pub fn is_started(&self) -> bool {
		self.state.load(std::sync::atomic::Ordering::Relaxed) == ConnectionState::Started
	}
}

impl<C: ServerCallback + Send + Sync + 'static> InnerConnectionHandler<C> {
	pub async fn start(mut self) -> Result<(), Error> {
		let mut buffer = [0; 255];
		let out_buffer_full = Arc::new(AtomicBool::new(false));

		loop {
			match self.state.load(std::sync::atomic::Ordering::Relaxed) {
				ConnectionState::WaitingStartDT => {
					let apdu = tokio::time::timeout(
						self.config.protocol.t1,
						receive_apdu(&mut self.read_connection, &mut buffer),
					)
					.await
					.whatever_context("Timeout waiting for startDT activation")?
					.whatever_context("Error receiving APDU")?;

					// TODO: How to detect a stuck client and terminated the connection?
					if let Frame::U(u) = apdu.frame
						&& u.start_dt_activation
					{
						send_frame(&mut self.write_connection, &START_DT_CON_FRAME)
							.await
							.whatever_context("Error sending startDT confirmation")?;
						self.state
							.store(ConnectionState::Started, std::sync::atomic::Ordering::Relaxed);
					}
				}
				ConnectionState::Started => {
					self.callback.on_connection_started(self.id, self.address).await;
					let rcv_handler = ReceiveHandler::new(
						&mut self.read_connection,
						&mut self.write_connection,
						self.callback.clone(),
						self.config.protocol.clone(),
						&mut self.rx,
						out_buffer_full.clone(),
					);
					if let Err(e) = rcv_handler.receive_task().await {
						self.callback.on_error(&e).await;
						return Err(e);
					}
					self.state.store(
						ConnectionState::WaitingStartDT,
						std::sync::atomic::Ordering::Relaxed,
					);
				}
			}
		}
	}
}
