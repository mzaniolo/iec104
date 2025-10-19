use std::sync::{Arc, atomic::AtomicBool};

use atomic_enum::atomic_enum;
use snafu::{ResultExt as _, whatever};
use tokio::{
	io::{ReadHalf, WriteHalf},
	net::TcpStream,
	sync::mpsc,
};
use tokio_native_tls::{
	TlsConnector,
	native_tls::{Certificate, Identity},
};
use tracing::instrument;

use crate::{
	apdu::Frame,
	asdu::Asdu,
	config::{LinkConfig, TlsConfig},
	error::Error,
	link::{Connection, OnNewObjects, START_DT_ACT_FRAME, receive_handler::ReceiveHandler},
};

#[atomic_enum]
#[derive(PartialEq)]
pub enum ConnectionHandlerState {
	WaitingForStart,
	Starting,
	Started,
	Reconnecting,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionHandlerCommand {
	Start,
	Stop,
	Test,
	Asdu(Asdu),
}

pub struct ConnectionHandler {
	read_connection: ReadHalf<Connection>,
	write_connection: WriteHalf<Connection>,
	callback: Arc<dyn OnNewObjects + Send + Sync>,
	config: LinkConfig,
	state: Arc<AtomicConnectionHandlerState>,
	rx: mpsc::Receiver<ConnectionHandlerCommand>,
	out_buffer_full: Arc<AtomicBool>,
}

impl ConnectionHandler {
	pub async fn new(
		callback: Arc<dyn OnNewObjects + Send + Sync>,
		config: LinkConfig,
		rx: mpsc::Receiver<ConnectionHandlerCommand>,
		out_buffer_full: Arc<AtomicBool>,
	) -> Result<Self, Error> {
		if config.server {
			let connection =
				Self::make_server(&config).await.whatever_context("Error making server")?;
			let (read_connection, write_connection) = tokio::io::split(connection);
			Ok(Self {
				callback,
				config,
				state: Arc::new(AtomicConnectionHandlerState::new(
					ConnectionHandlerState::WaitingForStart,
				)),
				read_connection,
				write_connection,
				rx,
				out_buffer_full,
			})
		} else {
			let connection =
				Self::make_connection(&config).await.whatever_context("Error making connection")?;
			let (read_connection, write_connection) = tokio::io::split(connection);
			Ok(Self {
				callback,
				config,
				state: Arc::new(AtomicConnectionHandlerState::new(
					ConnectionHandlerState::WaitingForStart,
				)),
				read_connection,
				write_connection,
				rx,
				out_buffer_full,
			})
		}
	}

	pub fn get_state(&self) -> Arc<AtomicConnectionHandlerState> {
		self.state.clone()
	}

	#[instrument(level = "debug", skip_all)]
	pub async fn run(&mut self) -> Result<(), Error> {
		loop {
			match self.state.load(std::sync::atomic::Ordering::Relaxed) {
				ConnectionHandlerState::WaitingForStart => {
					tracing::debug!("Waiting for start");
					if let Some(cmd) = self.rx.recv().await {
						match cmd {
							ConnectionHandlerCommand::Start => {
								self.state.store(
									ConnectionHandlerState::Starting,
									std::sync::atomic::Ordering::Relaxed,
								);
							}
							_ => {
								tracing::error!("Received unexpected command: {cmd:?}");
							}
						}
					} else {
						tracing::error!("Error receiving command. Aborting...");
						whatever!("Error receiving command.");
					}
				}
				ConnectionHandlerState::Starting => {
					tracing::debug!("Starting");
					if !self.config.server
						&& let Err(e) = self.send_start_dt().await
					{
						tracing::error!("Error sending startDT: {e}. Reconnecting");
						self.state.store(
							ConnectionHandlerState::Reconnecting,
							std::sync::atomic::Ordering::Relaxed,
						);
						continue;
					}

					tracing::debug!("StartDT activation confirmed");
					self.state.store(
						ConnectionHandlerState::Started,
						std::sync::atomic::Ordering::Relaxed,
					);
				}
				ConnectionHandlerState::Started => {
					if let Err(e) = ReceiveHandler::new(
						&mut self.read_connection,
						&mut self.write_connection,
						self.callback.clone(),
						self.config.clone(),
						&mut self.rx,
						self.out_buffer_full.clone(),
					)
					.receive_task()
					.await
					{
						tracing::error!("Error receiving task: {e}. Reconnecting");
						self.state.store(
							ConnectionHandlerState::Reconnecting,
							std::sync::atomic::Ordering::Relaxed,
						);
						continue;
					}
					tracing::debug!("Received a stop. Going back to waiting for start");
					self.state.store(
						ConnectionHandlerState::WaitingForStart,
						std::sync::atomic::Ordering::Relaxed,
					);
				}
				ConnectionHandlerState::Reconnecting => {
					tracing::debug!("Reconnecting");
					if self.config.server {
						let Ok(connection) = Self::make_server(&self.config).await else {
							tracing::error!("Error making server");
							tokio::time::sleep(self.config.protocol.t0).await;
							continue;
						};
						(self.read_connection, self.write_connection) =
							tokio::io::split(connection);
						self.state.store(
							ConnectionHandlerState::Starting,
							std::sync::atomic::Ordering::Relaxed,
						);
						continue;
					} else {
						let Ok(connection) = Self::make_connection(&self.config).await else {
							tracing::error!("Error making connection");
							tokio::time::sleep(self.config.protocol.t0).await;
							continue;
						};
						(self.read_connection, self.write_connection) =
							tokio::io::split(connection);
						self.state.store(
							ConnectionHandlerState::Starting,
							std::sync::atomic::Ordering::Relaxed,
						);
					}
				}
			}
		}
	}

	#[instrument(level = "debug")]
	async fn make_server(config: &LinkConfig) -> Result<Connection, Error> {
		let listener = tokio::net::TcpListener::bind(format!("{}:{}", config.address, config.port))
			.await
			.whatever_context("Error binding to address")?;
		tracing::info!("TCP server listening to {}:{}", config.address, config.port);

		let (stream, _addr) = listener.accept().await.whatever_context("Error connecting")?;
		tracing::info!("Client connecté");

		Ok(if let Some(ref tls) = config.tls {
			let connector = Self::make_tls_connector(tls)?;
			Connection::Tls(Box::new(
				connector
					.connect(&config.address, stream)
					.await
					.whatever_context("Error connecting TLS ")?,
			))
		} else {
			Connection::Tcp(stream)
		})
	}

	#[instrument(level = "debug")]
	async fn make_connection(config: &LinkConfig) -> Result<Connection, Error> {
		let stream = tokio::time::timeout(
			config.protocol.t0,
			TcpStream::connect(format!("{}:{}", config.address, config.port)),
		)
		.await
		.whatever_context("Connection timeout")?
		.whatever_context("Error connecting")?;

		Ok(if let Some(ref tls) = config.tls {
			let connector = Self::make_tls_connector(tls)?;
			Connection::Tls(Box::new(
				connector
					.connect(&config.address, stream)
					.await
					.whatever_context("Error connecting")?,
			))
		} else {
			Connection::Tcp(stream)
		})
	}

	#[instrument(level = "debug")]
	fn make_tls_connector(tls: &TlsConfig) -> Result<TlsConnector, Error> {
		let root_cert: Option<Certificate> = tls
			.server_certificate
			.as_ref()
			.map(std::fs::read)
			.transpose()
			.whatever_context("Failed to read server certificate")?
			.map(|cert_data| Certificate::from_pem(cert_data.as_slice()))
			.transpose()
			.whatever_context("Invalid server certificate")?;

		let identity: Option<Identity> = match (&tls.client_key, &tls.client_certificate) {
			(Some(client_key), Some(client_cert)) => Some(
				Identity::from_pkcs8(
					std::fs::read(client_cert)
						.whatever_context("Failed to read client certificate")?
						.as_slice(),
					std::fs::read(client_key)
						.whatever_context("Failed to read client key")?
						.as_slice(),
				)
				.whatever_context("Could not create client identity")?,
			),
			(None, None) => None,
			_ => whatever!("Both client key *and* certificate must be specified"),
		};

		let mut connector = tokio_native_tls::native_tls::TlsConnector::builder();

		if let Some(root_cert) = root_cert {
			connector.add_root_certificate(root_cert);
		}

		if let Some(identity) = identity {
			connector.identity(identity);
		}

		connector.danger_accept_invalid_certs(tls.danger_disable_tls_verify);

		let connector = connector.build().whatever_context("Error building TLS connector")?;
		Ok(TlsConnector::from(connector))
	}

	#[instrument(level = "debug", skip_all)]
	pub async fn send_start_dt(&mut self) -> Result<(), Error> {
		let mut buffer = [0; 255];
		ReceiveHandler::send_frame(&mut self.write_connection, &START_DT_ACT_FRAME)
			.await
			.whatever_context("Error sending startDT activation")?;

		let apdu = tokio::time::timeout(
			self.config.protocol.t1,
			ReceiveHandler::receive_apdu(&mut self.read_connection, &mut buffer),
		)
		.await
		.whatever_context("Timeout waiting for startDT activation")?;

		let apdu = apdu.whatever_context("Error receiving APDU")?;
		if let Frame::U(u) = apdu.frame
			&& !u.start_dt_confirmation
		{
			whatever!("StartDT activation not confirmed");
			//TODO: Do I need to check the rest?
		}

		Ok(())
	}
}
