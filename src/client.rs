use std::{
	fmt::Debug,
	pin::Pin,
	sync::{Arc, atomic::AtomicBool},
};

use async_trait::async_trait;
use lazy_static::lazy_static;
use snafu::{ResultExt, whatever};
use tokio::{
	io::{AsyncRead, AsyncWrite},
	net::TcpStream,
	sync::mpsc,
	task::JoinHandle,
};
use tokio_native_tls::TlsStream;
use tracing::instrument;

use crate::{
	apdu::{Frame, UFrame},
	asdu::Asdu,
	client::{
		connection_handler::{ConnectionHandler, ConnectionHandlerState},
		errors::ClientError,
	},
	config::ClientConfig,
	error::Error,
};

mod connection_handler;
pub mod errors;
mod receive_handler;

use connection_handler::{AtomicConnectionHandlerState, ConnectionHandlerCommand};

lazy_static! {
	static ref TEST_FR_CON_FRAME: Frame =
		Frame::U(UFrame { test_fr_confirmation: true, ..Default::default() });
	static ref START_DT_CON_FRAME: Frame =
		Frame::U(UFrame { start_dt_confirmation: true, ..Default::default() });
	static ref STOP_DT_CON_FRAME: Frame =
		Frame::U(UFrame { stop_dt_confirmation: true, ..Default::default() });
	static ref TEST_FR_ACT_FRAME: Frame =
		Frame::U(UFrame { test_fr_activation: true, ..Default::default() });
	static ref START_DT_ACT_FRAME: Frame =
		Frame::U(UFrame { start_dt_activation: true, ..Default::default() });
	static ref STOP_DT_ACT_FRAME: Frame =
		Frame::U(UFrame { stop_dt_activation: true, ..Default::default() });
}

#[derive(Debug)]
enum Connection {
	Tcp(TcpStream),
	Tls(TlsStream<TcpStream>),
}

impl AsyncRead for Connection {
	fn poll_read(
		self: Pin<&mut Self>,
		cx: &mut std::task::Context<'_>,
		buf: &mut tokio::io::ReadBuf<'_>,
	) -> std::task::Poll<std::io::Result<()>> {
		match self.get_mut() {
			Connection::Tcp(stream) => Pin::new(stream).poll_read(cx, buf),
			Connection::Tls(stream) => Pin::new(stream).poll_read(cx, buf),
		}
	}
}

impl AsyncWrite for Connection {
	fn poll_write(
		self: Pin<&mut Self>,
		cx: &mut std::task::Context<'_>,
		buf: &[u8],
	) -> std::task::Poll<Result<usize, std::io::Error>> {
		match self.get_mut() {
			Connection::Tcp(stream) => Pin::new(stream).poll_write(cx, buf),
			Connection::Tls(stream) => Pin::new(stream).poll_write(cx, buf),
		}
	}

	fn poll_flush(
		self: Pin<&mut Self>,
		cx: &mut std::task::Context<'_>,
	) -> std::task::Poll<Result<(), std::io::Error>> {
		match self.get_mut() {
			Connection::Tcp(stream) => Pin::new(stream).poll_flush(cx),
			Connection::Tls(stream) => Pin::new(stream).poll_flush(cx),
		}
	}

	fn poll_shutdown(
		self: Pin<&mut Self>,
		cx: &mut std::task::Context<'_>,
	) -> std::task::Poll<Result<(), std::io::Error>> {
		match self.get_mut() {
			Connection::Tcp(stream) => Pin::new(stream).poll_shutdown(cx),
			Connection::Tls(stream) => Pin::new(stream).poll_shutdown(cx),
		}
	}
}

#[async_trait]
pub trait OnNewObjects {
	async fn on_new_objects(&self, asdu: Asdu);
}

pub struct Client {
	config: ClientConfig,
	callback: Arc<dyn OnNewObjects + Send + Sync>,
	receive_task: Option<JoinHandle<Result<(), Error>>>,
	write_tx: Option<mpsc::Sender<ConnectionHandlerCommand>>,
	out_buffer_full: Arc<AtomicBool>,
	connection_handler_state: Option<Arc<AtomicConnectionHandlerState>>,
}

impl Client {
	#[must_use]
	pub fn new(config: ClientConfig, callback: impl OnNewObjects + Send + Sync + 'static) -> Self {
		Self {
			config,
			callback: Arc::new(callback),
			receive_task: None,
			write_tx: None,
			out_buffer_full: Arc::new(AtomicBool::new(false)),
			connection_handler_state: None,
		}
	}

	#[instrument(level = "debug")]
	pub async fn connect(&mut self) -> Result<(), Error> {
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
			tx.send(ConnectionHandlerCommand::Asdu(asdu)).await.context(errors::SendCommand)?;
		} else {
			return errors::NoWriteChannel.fail();
		}
		Ok(())
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
			tx.send(ConnectionHandlerCommand::Start).await.context(errors::SendCommand)?;
		} else {
			return errors::NoWriteChannel.fail();
		}
		Ok(())
	}

	#[instrument(level = "debug")]
	pub async fn stop_receiving(&mut self) -> Result<(), ClientError> {
		self.check_connection_started()?;

		if let Some(tx) = &self.write_tx {
			tx.send(ConnectionHandlerCommand::Stop).await.context(errors::SendCommand)?;
		} else {
			return errors::NoWriteChannel.fail();
		}
		Ok(())
	}

	#[instrument(level = "debug")]
	pub async fn send_test_frame(&mut self) -> Result<(), ClientError> {
		self.check_connection_started()?;

		if let Some(tx) = &self.write_tx {
			tx.send(ConnectionHandlerCommand::Test).await.context(errors::SendCommand)?;
		} else {
			return errors::NoWriteChannel.fail();
		}
		Ok(())
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

impl Debug for Client {
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
