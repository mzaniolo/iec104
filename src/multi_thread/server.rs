use std::{
	ops::DerefMut,
	sync::{Arc, Mutex, atomic},
};

use snafu::ResultExt;

use crate::{apdu, error::Error};

use super::{base_connection, client};

const CLEAN_UP_SECONDS: u64 = 60;

#[async_trait::async_trait]
pub trait ServerCallbacks {
	async fn on_connection_requested(
		&self,
		address: std::net::SocketAddr,
	) -> Option<Arc<dyn base_connection::ConnectionCallbacks + Send + Sync>>;
	async fn on_error(&self, e: Error);
}

pub struct Server {
	counter: Arc<atomic::AtomicUsize>,
	// Although it is a little strange, server configurations are the same as client configurations.
	config: client::ClientConfig,
	//callbacks: Arc<dyn ServerCallbacks + Send + Sync>,
	stop_watch_tx: tokio::sync::watch::Sender<bool>,
	stop_watch_rx: tokio::sync::watch::Receiver<bool>,
	connections: Arc<Mutex<Vec<base_connection::Iec104Connection>>>,
}

impl core::fmt::Debug for Server {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		if *self.stop_watch_rx.borrow() {
			write!(
				f,
				"Client ( Inactive ), listen address: {}:{}",
				self.config.address, self.config.port
			)
		} else {
			write!(
				f,
				"Client ( Active ), listen address: {}:{}",
				self.config.address, self.config.port
			)
		}
	}
}

impl Clone for Server {
	fn clone(&self) -> Self {
		self.counter.fetch_add(1, atomic::Ordering::Release);
		return Self {
			counter: self.counter.clone(),
			config: self.config.clone(),
			stop_watch_tx: self.stop_watch_tx.clone(),
			stop_watch_rx: self.stop_watch_rx.clone(),
			connections: self.connections.clone(),
		};
	}
}

impl Drop for Server {
	fn drop(&mut self) {
		if self.counter.fetch_sub(1, atomic::Ordering::Release) != 1 {
			return;
		}
		self.force_shutdown();
	}
}

impl Server {
	pub async fn new(
		config: client::ClientConfig,
		callbacks: Arc<dyn ServerCallbacks + Send + Sync>,
	) -> Result<Self, Error> {
		let listener = tokio::net::TcpListener::bind(format!("{}:{}", config.address, config.port))
			.await
			.whatever_context("Bind port error")?;
		let (stop_watch_tx, stop_watch_rx) = tokio::sync::watch::channel(false);
		let server = Self {
			counter: Arc::new(atomic::AtomicUsize::new(1)),
			config,
			stop_watch_tx,
			stop_watch_rx,
			connections: Arc::new(Mutex::new(Vec::new())),
		};
		tokio::spawn(server.clone().wait_connection_thread(listener, callbacks.clone()));
		tokio::spawn(server.clone().clean_up_thread(callbacks));
		return Ok(server);
	}
	/// Gracefully close the server.
	pub async fn close(&self) -> Result<(), Error> {
		self.stop_watch_tx.send_replace(true);
		match self.connections.lock() {
			Ok(guard) => {
				for connection in guard.iter() {
					if let base_connection::ConnectionStatus::Active = connection.status(){
						connection.stop().await?;
					}else{
						connection.close();
					}
				}
				return Ok(());
			}
			Err(e) => {
				snafu::whatever!("Lock error {}", e);
			}
		}
	}
	/// Force close the server.
	pub fn force_shutdown(&self) {
		self.stop_watch_tx.send_replace(true);
		// May panic!
		// But maybe panic is better than error, since restarting program is often easier than error handling.
		for con in self.connections.lock().unwrap().iter() {
			con.close();
		}
	}
	/// Send a frame, waiting until it is fully written to all connected TCP sockets or timeout.
	/// Returns the number of connections that are broadcasted.
	/// If `allow_empty` is `false` and there are no TCP sockets connected, an error is returned.
	pub async fn broadcast(&self, frame: apdu::Frame, allow_empty: bool) -> Result<usize, Error> {
		let mut join_set = tokio::task::JoinSet::new();
		match self.connections.lock() {
			Ok(guard) => {
				for connection in guard.iter() {
					if let base_connection::ConnectionStatus::Active = connection.status() {
						join_set.spawn(connection.clone().send_owned(frame.clone()));
					}
				}
			}
			Err(e) => {
				snafu::whatever!("Lock error {}", e);
			}
		}
		let len = join_set.len();
		if len <= 0 {
			if allow_empty {
				return Ok(0);
			} else {
				snafu::whatever!("No connections");
			}
		}
		while let Some(res) = join_set.join_next().await {
			match res {
				Ok(result) => {
					result?;
				}
				Err(err) if err.is_panic() => std::panic::resume_unwind(err.into_panic()),
				Err(err) => {
					return Err(snafu::FromString::with_source(
						err.into(),
						"Error joining tasks".to_string(),
					));
				}
			}
		}
		return Ok(len);
	}
	pub fn is_running(&self) -> bool {
		return !*self.stop_watch_rx.borrow();
	}
	async fn wait_connection_thread(
		mut self,
		listener: tokio::net::TcpListener,
		callbacks: Arc<dyn ServerCallbacks + Send + Sync>,
	) {
		loop {
			tokio::select! {
				res = self.stop_watch_rx.changed()=>{
					match res.whatever_context("Channel error"){
						Ok(_)=>{
							if *self.stop_watch_rx.borrow_and_update(){
								break;
							}
						}
						Err(e)=>{
							self.stop_watch_tx.send_replace(true);
							callbacks.on_error(e).await;
							break;
						}
					}
				}
				res = listener.accept() =>{
					match res.whatever_context("Error waiting connection"){
					//match res{
						Ok((stream,addr))=>{
							let mut err=None;
							match callbacks.on_connection_requested(addr.clone()).await{
								Some(connection_callbacks) => {
									match base_connection::Iec104Connection::new(stream,base_connection::ConnectionType::Server,self.config.protocol.clone(),connection_callbacks).await{
										Ok(connection) => {
											match self.connections.lock(){
												Ok(mut guard)=>{
													guard.push(connection);
												}
												Err(e) => {
													err=Some(snafu::FromString::without_source(format!("Lock error {}", e)));
												}
											}
										}
										Err(e) => {
											err=Some(e);
										}
									}
									//
								}
								None => {
									// Reject connection
								}
							}
							if let Some(e)=err{
								callbacks.on_error(e).await;
								break;
							}
						}
						Err(e)=>{
							callbacks.on_error(e).await;
							break;
						}
					}
				}
			}
		}
		self.stop_watch_tx.send_replace(true);
	}
	async fn clean_up_thread(mut self, callbacks: Arc<dyn ServerCallbacks + Send + Sync>) {
		let interval = tokio::time::Duration::from_secs(CLEAN_UP_SECONDS);
		loop {
			tokio::select! {
				_=tokio::time::sleep(interval)=>{}
				res=self.stop_watch_rx.changed()=>{
					match res {
						Ok(_) => {
							if *self.stop_watch_rx.borrow_and_update() {
								break;
							}
						}
						Err(_) => {
							break;
						}
					}
				}
			}
			let err_str = match self.connections.lock() {
				Ok(mut guard) => {
					guard.deref_mut().retain(|connection| !connection.is_closed());
					None
				}
				Err(e) => Some(format!("Lock error {}", e)),
			};
			if let Some(s) = err_str {
				callbacks.on_error(snafu::FromString::without_source(s)).await;
				break;
			}
		}
		self.stop_watch_tx.send_replace(true);
	}
}
