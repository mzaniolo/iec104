use std::{str::FromStr, sync::{atomic, Arc}};

use snafu::ResultExt;

use crate::{apdu, config, error::Error};

use super::base_connection;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ClientConfig {
	/// The address of the server.
	pub address: String,
	/// The port of the server.
	pub port: u16,
	/// The protocol configuration.
	#[serde(default)]
	pub protocol: config::ProtocolConfig,
	/// Whether to use TCP_NODELAY option or not.
	pub tcp_nodelay: bool,
	/// Whether to use SO_KEEPALIVE option or not.
	pub so_keepalive: bool,
	// The TLS configuration is not implemented yet!
}

/// Multi-thread version of IEC60870-5-104 client. Implements Sync + Send + Clone.
pub struct Client {
	counter: Arc<atomic::AtomicUsize>,
    config:ClientConfig,
	connection: base_connection::Iec104Connection,
}

impl Default for ClientConfig {
	fn default() -> Self {
		Self {
			address: "127.0.0.1".to_string(),
			port: 2404,
			protocol: config::ProtocolConfig::default(),
			tcp_nodelay: true,
            so_keepalive: true,
		}
	}
}

impl core::fmt::Debug for Client{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.connection.is_closed(){
            write!(f,"Client ( Inactive ), remote address: {}:{}",self.config.address,self.config.port)
        }else{
            write!(f,"Client ( Active ), remote address: {}:{}",self.config.address,self.config.port)
        }
    }
}

impl Clone for Client {
	fn clone(&self) -> Self {
		self.counter.fetch_add(1, atomic::Ordering::Release);
		return Self { counter: self.counter.clone(),config:self.config.clone(), connection: self.connection.clone() };
	}
}

impl Drop for Client {
	fn drop(&mut self) {
		if self.counter.fetch_sub(1, atomic::Ordering::Release) != 1 {
			return;
		}
		self.connection.close();
	}
}

impl Client{
	/// Create a new IEC60870-5-104 client.
    pub async fn new(config:ClientConfig,callbacks:Arc<dyn base_connection::ConnectionCallbacks + Send + Sync>)->Result<Self,Error>{
        let socket_address = std::net::SocketAddr::new(std::net::IpAddr::from_str(&config.address).whatever_context("Invalid address")?, config.port);
        let socket = tokio::net::TcpSocket::new_v4().whatever_context("Error creating tcp socket")?;
        if config.tcp_nodelay{
            socket.set_nodelay(true).whatever_context("Error setting tcp socket option")?;
        }
        if config.so_keepalive{
            socket.set_keepalive(true).whatever_context("Error setting tcp socket option")?;
        }
        let stream=tokio::time::timeout(config.protocol.t0,socket.connect(socket_address)).await.whatever_context("Connection timeout")?.whatever_context("Error connecting")?;
        let connection=base_connection::Iec104Connection::new(stream, base_connection::ConnectionType::Client, config.protocol.clone(), callbacks).await?;
        return Ok(Self{ counter: Arc::new(atomic::AtomicUsize::new(1)), config, connection });
    }
	/// Gracefully close the connection.
	pub async fn close(&self)->Result<(), Error>{
		return self.connection.stop().await;
	}
	/// Force close the connection.
	pub fn force_shutdown(&self){
		self.connection.close();
	}
	/// Send a frame, waiting until it is fully written to TCP socket or timeout.
	pub async fn send(&self,frame: apdu::Frame)->Result<(), Error>{
		self.connection.send(frame).await?;
		return Ok(());
	}
	/// Send a frame, waiting until reomte confirmation is received or timeout.
	/// Useful to ensure remote device has received control command.
	/// Returns `true` if the frame was positive confirmed, otherwise `false`.
	/// If `allow_negative` is `false` and remote sends a negative confirmation, an error is returned.
	pub async fn send_and_wait_confirm(&self,frame: apdu::Frame,allow_negative: bool)->Result<bool, Error>{
		return self.connection.send_and_wait_confirm(frame,allow_negative).await;
	}
	pub fn is_closed(&self)->bool{
		return self.connection.is_closed();
	}
	pub fn status(&self)->base_connection::ConnectionStatus{
		return self.connection.status();
	}
}