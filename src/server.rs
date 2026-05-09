use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use crate::{
	Connection, asdu::Asdu, config::ServerConfig, error::Error,
	receive_handler::ReceiveHandlerCallback, server::connection_handler::ConnectionHandler,
};

mod connection_handler;
pub mod error;

use snafu::{ResultExt as _, whatever};
use tokio::{
	net::TcpListener,
	sync::{mpsc, oneshot},
};
#[cfg(feature = "native-tls")]
use tokio_native_tls::{TlsAcceptor, native_tls::Identity};

/// Identifies a connection so it can be removed from the list when it
/// terminates.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ConnectionId(u64);

impl ConnectionId {
	const fn new() -> Self {
		Self(0)
	}
	const fn next(self) -> Self {
		Self(self.0.wrapping_add(1))
	}
}

#[async_trait::async_trait]
pub trait ServerCallback {
	async fn on_new_objects(&self, asdu: Asdu, id: ConnectionId, address: SocketAddr);
	async fn on_new_connection(&self, address: SocketAddr) {
		tracing::debug!("New connection from {address}");
	}
	async fn on_connection_started(&self, id: ConnectionId, address: SocketAddr) {
		tracing::debug!("Connection {id:?} from {address} started");
	}
	async fn on_connection_stopped(&self, id: ConnectionId, address: SocketAddr) {
		tracing::debug!("Connection {id:?} from {address} stopped");
	}
	async fn on_error(&self, error: &Error) {
		tracing::debug!("Error: {error}");
	}
}

struct InnerServerCallback<C: ServerCallback + Send + Sync> {
	id: ConnectionId,
	address: SocketAddr,
	callback: Arc<C>,
}

#[async_trait::async_trait]
impl<C: ServerCallback + Send + Sync> ReceiveHandlerCallback for InnerServerCallback<C> {
	async fn on_new_objects(&self, asdu: Asdu) {
		self.callback.on_new_objects(asdu, self.id, self.address).await;
	}
}

impl<C: ServerCallback + Send + Sync> InnerServerCallback<C> {
	async fn on_connection_started(&self, id: ConnectionId, address: SocketAddr) {
		self.callback.on_connection_started(id, address).await;
	}
	async fn on_error(&self, error: &Error) {
		self.callback.on_error(error).await;
	}
}

/// Commands for the server to send to the inner server
#[derive(Debug)]
pub enum ServerCommand {
	SingleConnection {
		id: ConnectionId,
		asdu: Asdu,
		tx: oneshot::Sender<Result<(), error::ServerError>>,
	},
	Broadcast {
		asdu: Asdu,
		tx: oneshot::Sender<Result<(), error::ServerError>>,
	},
}

#[derive(Debug, Clone)]
pub struct Server {
	tx: mpsc::Sender<ServerCommand>,
}

struct InnerServer<C: ServerCallback + Send + Sync + 'static> {
	callback: Arc<C>,
	config: ServerConfig,
	connections: HashMap<ConnectionId, ConnectionHandler>,
	rx: mpsc::Receiver<ServerCommand>,
	#[cfg(feature = "native-tls")]
	acceptor: Option<TlsAcceptor>,
	listener: TcpListener,
}

impl Server {
	pub async fn start<C: ServerCallback + Send + Sync + 'static>(
		config: ServerConfig,
		callback: C,
	) -> Result<Self, Error> {
		let (tx, rx) = mpsc::channel(1024);
		let inner_server = InnerServer::connect(config, Arc::new(callback), rx).await?;
		tokio::spawn(async move {
			let _ = inner_server
				.start()
				.await
				.inspect_err(|e| tracing::error!("Error in running server: {e}"));
		});
		Ok(Self { tx })
	}

	/// Like [`Self::start`], but also returns the socket address the TCP
	/// listener bound to (for example after setting
	/// [`crate::config::ServerConfig::port`] to `0` for an ephemeral port).
	pub async fn start_with_listen_addr<C: ServerCallback + Send + Sync + 'static>(
		config: ServerConfig,
		callback: C,
	) -> Result<(Self, SocketAddr), Error> {
		let (tx, rx) = mpsc::channel(1024);
		let inner_server = InnerServer::connect(config, Arc::new(callback), rx).await?;
		let listen_addr = inner_server
			.listen_local_addr()
			.with_whatever_context(|e| format!("Unable to read listener local address: {e}"))?;
		tokio::spawn(async move {
			let _ = inner_server
				.start()
				.await
				.inspect_err(|e| tracing::error!("Error in running server: {e}"));
		});
		Ok((Self { tx }, listen_addr))
	}

	pub async fn send_asdu(&self, id: ConnectionId, asdu: Asdu) -> Result<(), error::ServerError> {
		let (tx, rx) = oneshot::channel();
		self.tx
			.send(ServerCommand::SingleConnection { id, asdu, tx })
			.await
			.context(error::SendCommand)?;
		rx.await.context(error::ReceiveResponse)?
	}
	pub async fn broadcast_asdu(&self, asdu: Asdu) -> Result<(), error::ServerError> {
		let (tx, rx) = oneshot::channel();
		self.tx.send(ServerCommand::Broadcast { asdu, tx }).await.context(error::SendCommand)?;
		rx.await.context(error::ReceiveResponse)?
	}
}

impl<C: ServerCallback + Send + Sync + 'static> InnerServer<C> {
	async fn connect(
		config: ServerConfig,
		callback: Arc<C>,
		rx: mpsc::Receiver<ServerCommand>,
	) -> Result<Self, Error> {
		// Reject spec-violating k/w combinations (IEC 60870-5-104 §5.2)
		// before binding the socket so misconfiguration fails fast.
		config.protocol.validate().whatever_context("Invalid protocol configuration")?;

		let bind_addr = format!("{}:{}", config.address, config.port);
		let listener = TcpListener::bind(&bind_addr)
			.await
			.with_whatever_context(|_| format!("Unable to bind to address '{bind_addr}'"))?;
		#[cfg(feature = "native-tls")]
		{
			let acceptor = config
				.tls
				.as_ref()
				.map(|tls| {
					let identity = Identity::from_pkcs8(
						std::fs::read(tls.server_certificate.clone())
							.whatever_context::<_, Error>("Failed to read server certificate")?
							.as_slice(),
						std::fs::read(tls.server_key.clone())
							.whatever_context::<_, Error>("Failed to read server key")?
							.as_slice(),
					)
					.whatever_context("Error creating server TLS identity")?;
					tokio_native_tls::native_tls::TlsAcceptor::new(identity)
						.map(TlsAcceptor::from)
						.whatever_context("Error crating TLS acceptor")
				})
				.transpose()?;
			Ok(Self { callback, config, connections: HashMap::new(), rx, acceptor, listener })
		}
		#[cfg(not(feature = "native-tls"))]
		Ok(Self { callback, config, connections: HashMap::new(), rx, listener })
	}

	fn listen_local_addr(&self) -> std::io::Result<SocketAddr> {
		self.listener.local_addr()
	}

	async fn start(mut self) -> Result<(), Error> {
		let (closed_tx, mut closed_rx) = mpsc::unbounded_channel::<ConnectionId>();
		let mut next_id = ConnectionId::new();

		loop {
			tokio::select! {
				accept_result = self.listener.accept() => {
					let (socket, address) =
						accept_result.whatever_context("Error accepting a new connection")?;
					#[cfg(feature = "native-tls")]
					let connection = if let Some(ref acceptor) = self.acceptor {
						Connection::Tls(
							acceptor
								.accept(socket)
								.await
								.whatever_context("Error doing the TLS handshake")?,
						)
					} else {
						Connection::Tcp(socket)
					};
					#[cfg(not(feature = "native-tls"))]
					let connection = Connection::Tcp(socket);
					// TODO: Do we want to accept or decline the connection based on the callback response?
					self.callback.on_new_connection(address).await;

					// TODO: Do want to have a max number of connections?

					// Gets the next available connection id
					while self.connections.contains_key(&next_id) {
						next_id = next_id.next();
					}

					let id = next_id;
					next_id = next_id.next();
					let callback = InnerServerCallback { id, address, callback: self.callback.clone() };
					let connection_handler = ConnectionHandler::start(id, address, connection, self.config.clone(), callback, closed_tx.clone());
					self.connections.insert(id, connection_handler);
				}
				closed_id = closed_rx.recv() => {
					if let Some(id) = closed_id {
						let connection_handler = self.connections.remove(&id);
						match connection_handler {
							Some(connection_handler) => {
								self.callback.on_connection_stopped(id, connection_handler.address).await;
							}
							None => {
								tracing::error!("Connection {id:?} not found");
							}
						}
					}
				}
				command = self.rx.recv() => {
					let Some(command) = command else {
						tracing::error!("Error receiving command. Aborting...");
						whatever!("");
					};
					match command {
						ServerCommand::SingleConnection { id, asdu, tx } => {
							let connection_handler = self.connections.get(&id);
							let res = match connection_handler {
								Some(connection_handler) => {
									connection_handler.send_asdu(asdu).await
								}
								None => {
									error::ConnectionNotFound { id }.fail()
								}
							};
							let _ = tx.send(res).inspect_err(|e| tracing::error!("Error sending response to single connection command: {e:?}"));
						},
						ServerCommand::Broadcast { asdu, tx } => {
							let mut res = Ok(());
							for connection_handler in self.connections.values().filter(|c| c.is_started()) {
								if let Err(e) = connection_handler.send_asdu(asdu.clone()).await {
									res = Err(e);
									break;
								}
							}
							let _ = tx.send(res).inspect_err(|e| tracing::error!("Error sending response to broadcast command: {e:?}"));
						},
					}
				}
			}
		}
	}
}
