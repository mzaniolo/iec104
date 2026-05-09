use std::sync::{Arc, atomic::AtomicBool};

use atomic_enum::atomic_enum;
use snafu::{ResultExt as _, whatever};
use tokio::{
	io::{ReadHalf, WriteHalf},
	net::TcpStream,
	sync::mpsc,
};
#[cfg(feature = "native-tls")]
use tokio_native_tls::{
	TlsConnector,
	native_tls::{Certificate, Identity},
};
use tracing::instrument;

use crate::{
	START_DT_ACT_FRAME,
	apdu::Frame,
	client::{ClientCallback, Connection, InnerClientCallback},
	config::{ClientConfig, TlsClientConfig},
	error::Error,
	receive_handler::{ReceiveHandler, ReceiveHandlerCommand, receive_apdu, send_frame},
};

#[atomic_enum]
#[derive(PartialEq)]
pub enum ConnectionHandlerState {
	WaitingForStart,
	Starting,
	Started,
	Reconnecting,
}

pub struct ConnectionHandler<C: ClientCallback + Send + Sync + 'static> {
	read_connection: ReadHalf<Connection>,
	write_connection: WriteHalf<Connection>,
	callback: Arc<InnerClientCallback<C>>,
	config: ClientConfig,
	state: Arc<AtomicConnectionHandlerState>,
	rx: mpsc::Receiver<ReceiveHandlerCommand>,
	out_buffer_full: Arc<AtomicBool>,
}

impl<C: ClientCallback + Send + Sync + 'static> ConnectionHandler<C> {
	pub async fn new(
		callback: Arc<InnerClientCallback<C>>,
		config: ClientConfig,
		rx: mpsc::Receiver<ReceiveHandlerCommand>,
		out_buffer_full: Arc<AtomicBool>,
	) -> Result<Self, Error> {
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

	pub fn get_state(&self) -> Arc<AtomicConnectionHandlerState> {
		self.state.clone()
	}

	#[instrument(level = "debug", skip_all)]
	pub async fn run(&mut self) -> Result<(), Error> {
		loop {
			match self.state.load(std::sync::atomic::Ordering::Relaxed) {
				ConnectionHandlerState::WaitingForStart => {
					if let Some(cmd) = self.rx.recv().await {
						match cmd {
							ReceiveHandlerCommand::Start => {
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
					if let Err(e) = self.send_start_dt().await {
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
					self.callback.on_connection_started().await;
					if let Err(e) = ReceiveHandler::new(
						&mut self.read_connection,
						&mut self.write_connection,
						self.callback.clone(),
						self.config.protocol.clone(),
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
						self.callback.on_error(e).await;
						continue;
					}
					tracing::debug!("Received a stop. Going back to waiting for start");
					self.state.store(
						ConnectionHandlerState::WaitingForStart,
						std::sync::atomic::Ordering::Relaxed,
					);
					self.callback.on_connection_stopped().await;
				}
				ConnectionHandlerState::Reconnecting => {
					tracing::debug!("Reconnecting");
					self.callback.on_reconnecting().await;
					let Ok(connection) = Self::make_connection(&self.config).await else {
						tracing::error!("Error making connection");
						tokio::time::sleep(self.config.protocol.t0).await;
						continue;
					};
					(self.read_connection, self.write_connection) = tokio::io::split(connection);
					self.state.store(
						ConnectionHandlerState::Starting,
						std::sync::atomic::Ordering::Relaxed,
					);
				}
			}
		}
	}

	#[instrument(level = "debug")]
	async fn make_connection(config: &ClientConfig) -> Result<Connection, Error> {
		let stream = tokio::time::timeout(
			config.protocol.t0,
			TcpStream::connect(format!("{}:{}", config.address, config.port)),
		)
		.await
		.whatever_context("Connection timeout")?
		.whatever_context("Error connecting")?;

		#[cfg(feature = "native-tls")]
		return Ok(if let Some(ref tls) = config.tls {
			let connector = Self::make_tls_connector(tls)?;
			Connection::Tls(
				connector
					.connect(&config.address, stream)
					.await
					.whatever_context("Error connecting")?,
			)
		} else {
			Connection::Tcp(stream)
		});
		#[cfg(not(feature = "native-tls"))]
		if let Some(_) = config.tls {
			snafu::whatever!(
				"TLS features disabled! Recompile with \"native-tls\" feature on to enable them!"
			);
		} else {
			Ok(Connection::Tcp(stream))
		}
	}

	#[cfg(feature = "native-tls")]
	#[instrument(level = "debug")]
	fn make_tls_connector(tls: &TlsClientConfig) -> Result<TlsConnector, Error> {
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
		send_frame(&mut self.write_connection, &START_DT_ACT_FRAME)
			.await
			.whatever_context("Error sending startDT activation")?;

		let apdu = tokio::time::timeout(
			self.config.protocol.t1,
			receive_apdu(&mut self.read_connection, &mut buffer),
		)
		.await
		.whatever_context("Timeout waiting for startDT activation")?;

		let apdu = apdu.whatever_context("Error receiving APDU")?;
		// Per IEC 60870-5-104 §5.3, only a U-frame with the
		// startDT-confirmation flag set is a valid response. I/S frames
		// or any other U-frame variant is a protocol violation.
		match apdu.frame {
			Frame::U(u) if u.start_dt_confirmation => Ok(()),
			frame => whatever!("StartDT activation not confirmed; received {frame:?}"),
		}
	}
}
