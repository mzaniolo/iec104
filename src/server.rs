use std::{collections::HashMap, net::SocketAddr, sync::Arc, time::Duration};

use crate::{
	Connection, asdu::Asdu, config::ServerConfig, error::Error,
	receive_handler::ReceiveHandlerCallback, server::connection_handler::ConnectionHandler,
};

mod connection_handler;
pub mod error;

use snafu::{ResultExt as _, whatever};
use tokio::{
	net::{TcpListener, TcpStream},
	sync::{mpsc, oneshot},
};

#[cfg(any(feature = "native_tls", feature = "rustls"))]
use crate::tls::TlsServerAcceptor;

/// How long the accept loop pauses after an accept failure that is not tied
/// to a single connection attempt, so it does not spin while the listener
/// stays readable.
const ACCEPT_BACKOFF: Duration = Duration::from_millis(100);

/// Whether an accept error is specific to the connection attempt that just
/// failed.
///
/// Those are safe to retry immediately: the peer went away and the next
/// accept will make progress. Anything else (running out of file descriptors
/// or buffers, for instance) leaves the listener readable, so retrying
/// without a pause would spin the loop at full speed.
fn is_per_connection_accept_error(error: &std::io::Error) -> bool {
	matches!(
		error.kind(),
		std::io::ErrorKind::ConnectionAborted
			| std::io::ErrorKind::ConnectionReset
			| std::io::ErrorKind::Interrupted
	)
}

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

/// A freshly accepted socket whose transport is not negotiated yet.
///
/// Building this is synchronous so the accept loop never awaits on a peer.
/// The handshake itself happens in the connection's own task through
/// [`Self::establish`].
pub(crate) enum PendingConnection {
	/// Plain TCP; nothing left to negotiate.
	Tcp(TcpStream),
	/// TLS; the handshake still has to run.
	#[cfg(any(feature = "native_tls", feature = "rustls"))]
	Tls {
		/// The accepted socket.
		socket: TcpStream,
		/// The acceptor to run the handshake with.
		acceptor: crate::tls::TlsAcceptor,
	},
}

impl PendingConnection {
	/// Negotiates the transport, bounded by `handshake_timeout` so a peer that
	/// connects but never completes the TLS handshake cannot hold the
	/// connection slot forever.
	#[cfg_attr(
		not(any(feature = "native_tls", feature = "rustls")),
		expect(clippy::unused_async, reason = "the TLS branch is compiled out")
	)]
	pub(crate) async fn establish(
		self,
		#[cfg_attr(
			not(any(feature = "native_tls", feature = "rustls")),
			expect(unused_variables, reason = "the TLS branch is compiled out")
		)]
		handshake_timeout: Duration,
	) -> Result<Connection, Error> {
		match self {
			Self::Tcp(socket) => Ok(Connection::Tcp(socket)),
			#[cfg(any(feature = "native_tls", feature = "rustls"))]
			Self::Tls { socket, acceptor } => {
				let stream = tokio::time::timeout(handshake_timeout, acceptor.accept(socket))
					.await
					.whatever_context("Timeout doing the TLS handshake")?
					.whatever_context("Error doing the TLS handshake")?;
				Ok(Connection::Tls(Box::new(stream)))
			}
		}
	}
}

#[async_trait::async_trait]
pub trait ServerCallback {
	async fn on_new_objects(&self, asdu: Asdu, id: ConnectionId, address: SocketAddr);
	/// Called once the transport is up: immediately for plain TCP, and only
	/// after the TLS handshake succeeds when TLS is enabled.
	///
	/// Runs on the connection's own task, so a slow callback delays only that
	/// connection. A peer that never completes the handshake does not trigger
	/// this callback.
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
	async fn on_new_connection(&self, address: SocketAddr) {
		self.callback.on_new_connection(address).await;
	}
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
	#[cfg(any(feature = "native_tls", feature = "rustls"))]
	acceptor: Option<crate::tls::TlsAcceptor>,
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

		#[cfg(not(any(feature = "native_tls", feature = "rustls")))]
		if config.tls.is_some() {
			whatever!("TLS support is disabled; enable the `native_tls` or `rustls` Cargo feature");
		}

		#[cfg(any(feature = "native_tls", feature = "rustls"))]
		let acceptor = match &config.tls {
			None => None,
			Some(tls) => Some(crate::tls::build_server_acceptor(tls)?),
		};
		Ok(Self {
			callback,
			config,
			connections: HashMap::new(),
			rx,
			#[cfg(any(feature = "native_tls", feature = "rustls"))]
			acceptor,
			listener,
		})
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
					// A failure here is about this connection attempt or a
					// transient resource shortage, never a reason to take the
					// whole server down.
					let (socket, address) = match accept_result {
						Ok(accepted) => accepted,
						Err(e) => {
							tracing::warn!("Error accepting a new connection: {e}");
							if !is_per_connection_accept_error(&e) {
								tokio::time::sleep(ACCEPT_BACKOFF).await;
							}
							continue;
						}
					};
					// The TLS handshake is deliberately not awaited here; it
					// runs in the connection's own task so a stalled peer
					// cannot block accepting, reaping or send commands.
					let pending = {
						#[cfg(any(feature = "native_tls", feature = "rustls"))]
						{
							match &self.acceptor {
								Some(acceptor) => PendingConnection::Tls {
									socket,
									acceptor: acceptor.clone(),
								},
								None => PendingConnection::Tcp(socket),
							}
						}
						#[cfg(not(any(feature = "native_tls", feature = "rustls")))]
						{
							PendingConnection::Tcp(socket)
						}
					};
					// TODO: Do want to have a max number of connections?

					// Gets the next available connection id
					while self.connections.contains_key(&next_id) {
						next_id = next_id.next();
					}

					let id = next_id;
					next_id = next_id.next();
					let callback = InnerServerCallback { id, address, callback: self.callback.clone() };
					let connection_handler = ConnectionHandler::start(id, address, pending, self.config.clone(), callback, closed_tx.clone());
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
