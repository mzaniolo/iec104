use std::{
	fmt::Debug,
	pin::Pin,
	sync::{Arc, atomic::AtomicBool},
	time::Duration,
};

use async_trait::async_trait;
use lazy_static::lazy_static;
use snafu::{OptionExt as _, ResultExt, whatever};
use tokio::{
	io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadHalf, WriteHalf},
	net::TcpStream,
	select,
	sync::mpsc,
	task::JoinHandle,
	time::Instant,
};
use tokio_native_tls::{
	TlsConnector, TlsStream,
	native_tls::{Certificate, Identity},
};

use crate::{
	apdu::{APUD_MAX_LENGTH, Apdu, Frame, IFrame, SFrame, TELEGRAN_HEADER, UFrame},
	asdu::Asdu,
	config::{ClientConfig, TlsClientConfig},
	error::Error,
	types::InformationObject,
};

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
	static ref TIMER_UNSET: Duration = Duration::from_secs(2_600_000);
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
	async fn on_new_objects(&self, objects: InformationObject);
}

pub struct Client {
	config: ClientConfig,
	callback: Arc<dyn OnNewObjects + Send + Sync>,
	receive_task: Option<JoinHandle<Result<(), Error>>>,
	read_connection: Option<ReadHalf<Connection>>,
	write_connection: Option<WriteHalf<Connection>>,
	write_tx: Option<mpsc::Sender<Asdu>>,
	out_buffer_full: Arc<AtomicBool>,
}

impl Client {
	pub async fn new(
		config: ClientConfig,
		callback: impl OnNewObjects + Send + Sync + 'static,
	) -> Result<Self, Error> {
		let connection =
			Self::make_connection(&config).await.whatever_context("Error making connection")?;
		let (read_connection, write_connection) = tokio::io::split(connection);
		Ok(Self {
			config,
			read_connection: Some(read_connection),
			write_connection: Some(write_connection),
			callback: Arc::new(callback),
			receive_task: None,
			write_tx: None,
			out_buffer_full: Arc::new(AtomicBool::new(false)),
		})
	}

	async fn make_connection(config: &ClientConfig) -> Result<Connection, Error> {
		let stream = TcpStream::connect(format!("{}:{}", config.address, config.port))
			.await
			.whatever_context("Error connecting")?;
		Ok(if let Some(ref tls) = config.tls {
			let connector = Self::make_tls_connector(tls)?;
			Connection::Tls(
				connector
					.connect(&config.address, stream)
					.await
					.whatever_context("Error connecting")?,
			)
		} else {
			Connection::Tcp(stream)
		})
	}

	// TODO: Remove this
	#[allow(clippy::result_large_err)]
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

	async fn send_frame(
		write_connection: &mut WriteHalf<Connection>,
		frame: &Frame,
	) -> Result<(), Error> {
		write_connection
			.write_all(
				&frame
					.to_apdu_bytes()
					.whatever_context("Error converting frame to APDU and encoding")?,
			)
			.await
			.whatever_context("Error sending data")?;
		Ok(())
	}

	pub async fn start_receiving(&mut self) -> Result<(), Error> {
		if self.receive_task.is_some() {
			whatever!("Receive task already running");
		}

		let (mut read_connection, mut write_connection) =
			if let (Some(read_connection), Some(write_connection)) =
				(self.read_connection.take(), self.write_connection.take())
			{
				(read_connection, write_connection)
			} else {
				tracing::debug!("No connection to send or receive data. Creating a new one");
				let connection = Self::make_connection(&self.config)
					.await
					.whatever_context("Error making connection")?;
				tokio::io::split(connection)
			};

		let mut buffer = [0; 255];
		Self::send_frame(&mut write_connection, &START_DT_ACT_FRAME)
			.await
			.whatever_context("Error sending startDT activation")?;

		let apdu = tokio::time::timeout(
			self.config.protocol.t1,
			Self::receive_apdu(&mut read_connection, &mut buffer),
		)
		.await
		.whatever_context("Timeout waiting for startDT activation")?;

		let apdu = apdu.whatever_context("Error receiving APDU")?;
		if let Frame::U(u) = apdu.frame {
			if !u.start_dt_confirmation {
				whatever!("StartDT activation not confirmed");
			}
			//TODO: Do I need to check the rest?
		}

		let (tx, rx) = mpsc::channel(1024);
		self.write_tx = Some(tx);
		let callback = self.callback.clone();
		let config = self.config.clone();
		let out_buffer_full = self.out_buffer_full.clone();

		self.receive_task = Some(tokio::spawn(async move {
			Self::receive_task(
				read_connection,
				write_connection,
				callback,
				config,
				rx,
				out_buffer_full,
			)
			.await
			.inspect_err(|e| tracing::error!("Error in receiving the task: {e:?}"))
		}));

		Ok(())
	}

	pub async fn send_asdu(&mut self, asdu: Asdu) -> Result<(), Error> {
		if self.out_buffer_full.load(std::sync::atomic::Ordering::Relaxed) {
			whatever!("Output buffer is full. Waiting for it to be cleared");
		}

		if let Some(tx) = &mut self.write_tx {
			tx.send(asdu).await.whatever_context("Error sending command")?;
		} else {
			whatever!("No write connection. Start receiving first");
		}
		Ok(())
	}

	async fn receive_apdu(
		connection: &mut ReadHalf<Connection>,
		buffer: &mut [u8; 255],
	) -> Result<Apdu, Error> {
		connection.read(&mut buffer[0..2]).await.whatever_context("Error receiving data")?;
		if buffer[0] != TELEGRAN_HEADER {
			whatever!("Invalid starter byte: {:02x}{:02x}", buffer[0], buffer[1]);
		}
		let length = buffer[1] as usize;
		if length > APUD_MAX_LENGTH as usize {
			whatever!("Invalid length: {}", length);
		}
		connection
			.read_exact(&mut buffer[2..length + 2])
			.await
			.whatever_context("Error receiving data")?;
		Apdu::from_bytes(&buffer[0..length + 2]).whatever_context("Error decoding APDU")
	}

	async fn receive_task(
		mut read_connection: ReadHalf<Connection>,
		mut write_connection: WriteHalf<Connection>,
		callback: Arc<dyn OnNewObjects + Send + Sync>,
		config: ClientConfig,
		mut rx: mpsc::Receiver<Asdu>,
		out_buffer_full: Arc<AtomicBool>,
	) -> Result<(), Error> {
		// The t1 only start counting after we sent something. A month should be enough
		let t1_u = tokio::time::sleep(*TIMER_UNSET);
		let t1_i = tokio::time::sleep(*TIMER_UNSET);
		let t2 = tokio::time::sleep(*TIMER_UNSET);
		let t3 = tokio::time::sleep(config.protocol.t3);
		tokio::pin!(t1_u);
		tokio::pin!(t1_i);
		tokio::pin!(t2);
		tokio::pin!(t3);

		let mut buffer = [0; 255];
		let mut sent_counter: u16 = 0;
		let mut received_counter: u16 = 0;
		let mut unacknowledged_rcv_frames: u16 = 0;
		let mut outstanding_test_fr_con_messages: u16 = 0;
		let mut unacknowledged_seq_num: Vec<(u16, Instant)> =
			Vec::with_capacity(config.protocol.k as usize);

		loop {
			select! {
				apdu = Self::receive_apdu(&mut read_connection,	&mut buffer) => {
					if let Ok(apdu) = apdu {
						match apdu.frame {
							Frame::I(i) => {
								Self::handle_receive_i_frame(&i, &mut received_counter, &mut sent_counter, &mut unacknowledged_seq_num, &out_buffer_full, &mut t1_i, &config, &mut unacknowledged_rcv_frames)?;

								// TODO: Should I spawn a task for it?
								callback.on_new_objects(i.asdu.information_objects).await;
							}
							Frame::S(s) => {
								tracing::debug!("Received S frame: {s:?}");
								Self::check_sequence_acknowledge(&mut unacknowledged_seq_num, s.receive_sequence_number, sent_counter).whatever_context("Error checking sequence acknowledge")?;

								out_buffer_full.store(unacknowledged_seq_num.len() >= config.protocol.k as usize, std::sync::atomic::Ordering::Relaxed);

								if !unacknowledged_seq_num.is_empty() {
									t1_i.as_mut().reset(unacknowledged_seq_num.first().whatever_context("Unacknowledged sequence number is empty")?.1 + config.protocol.t1);
								}

							}
							Frame::U(u) => {
								Self::handle_receive_u_frame(&u, &mut write_connection, &mut outstanding_test_fr_con_messages, &mut t1_u).await?;
							}
						}
						t3.as_mut().reset(Instant::now() + config.protocol.t3);
					} else {
						whatever!("Error receiving APDU");
					}
				}
				Some(cmd) = rx.recv() => {
					Self::handle_send_asdu(cmd, &mut sent_counter, received_counter, &mut write_connection, &mut unacknowledged_seq_num, config.protocol.k, &mut unacknowledged_rcv_frames).await.whatever_context("Error sending command")?;

					out_buffer_full.store(unacknowledged_seq_num.len() >= config.protocol.k as usize, std::sync::atomic::Ordering::Relaxed);

					t2.as_mut().reset(unacknowledged_seq_num.first().whatever_context("Unacknowledged sequence number is empty")?.1 + config.protocol.t2);
				}
				_ = &mut t3 => {
					tracing::debug!("t3 timeout. Sending test frame");
					if outstanding_test_fr_con_messages > 2 {
						whatever!("Outstanding test frame confirmation messages is greater than 2. Closing connection");
					}
					Self::send_frame(&mut write_connection, &TEST_FR_ACT_FRAME).await.whatever_context("Error sending test frame")?;
					outstanding_test_fr_con_messages += 1;
					t3.as_mut().reset(Instant::now() + config.protocol.t3);
					t1_u.as_mut().reset(Instant::now() + config.protocol.t1);

				}
				_ = &mut t2 => {
					tracing::debug!("t2 timeout. Sending S frame");
					Self::send_frame(&mut write_connection, &Frame::S(SFrame{receive_sequence_number: received_counter})).await.whatever_context("Error sending S frame")?;
					unacknowledged_rcv_frames = 0;
					t2.as_mut().reset(Instant::now() + *TIMER_UNSET);
				}
				_ = &mut t1_u => {
					whatever!("t1 for u frames timeout");
				}
				_ = &mut t1_i => {
					whatever!("t1 for i frames timeout");
				}
			}
			if unacknowledged_rcv_frames > config.protocol.w {
				tracing::debug!(
					"Received more than w frames without acknowledgement. Sending S frame"
				);
				Self::send_frame(
					&mut write_connection,
					&Frame::S(SFrame { receive_sequence_number: received_counter }),
				)
				.await
				.whatever_context("Error sending S frame")?;
				unacknowledged_rcv_frames = 0;
				t2.as_mut().reset(Instant::now() + config.protocol.t2);
			}
		}
	}

	// check if received sequence number is valid and remove the acknowledged ones
	fn check_sequence_acknowledge(
		unacknowledged_seq_num: &mut Vec<(u16, Instant)>,
		frame_rss: u16,
		sent_counter: u16,
	) -> Result<(), Error> {
		let mut is_valid = false;

		if let (Some(newest_seq_num), Some(oldest_seq_num)) =
			(unacknowledged_seq_num.last(), unacknowledged_seq_num.first())
		{
			// Two cases are required to reflect sequence number overflow
			if oldest_seq_num.0 <= newest_seq_num.0 {
				if frame_rss >= oldest_seq_num.0 && frame_rss <= newest_seq_num.0 {
					is_valid = true;
				}
			} else {
				// overflow case
				if frame_rss >= oldest_seq_num.0 || frame_rss <= newest_seq_num.0 {
					is_valid = true;
				}
			}

			// check if confirmed message was already removed from list
			let oldest_valid_seq_num =
				if oldest_seq_num.0 == 0 { 32767 } else { (oldest_seq_num.0 - 1) % 32768 };

			if oldest_valid_seq_num == frame_rss {
				return Ok(());
			}
		} else {
			// If we cant get the first and last members it means that the vector is empty
			if frame_rss == sent_counter {
				return Ok(());
			}
		}

		if is_valid {
			let i = unacknowledged_seq_num.iter().position(|(seq, _)| *seq == frame_rss);
			if let Some(i) = i {
				unacknowledged_seq_num.drain(0..=i);
				return Ok(());
			} else {
				whatever!(
					"Received frame with sequence number that is not in the unacknowledged list"
				);
			}
		}

		//TODO: Fix it
		whatever!("Received frame with invalid sequence number");
	}

	async fn handle_send_asdu(
		asdu: Asdu,
		sent_counter: &mut u16,
		received_counter: u16,
		write_connection: &mut WriteHalf<Connection>,
		unacknowledged_seq_num: &mut Vec<(u16, Instant)>,
		k: u16,
		unacknowledged_rcv_frames: &mut u16,
	) -> Result<(), Error> {
		let frame = Frame::I(IFrame {
			send_sequence_number: *sent_counter,
			receive_sequence_number: received_counter,
			asdu,
		});

		Self::send_frame(write_connection, &frame)
			.await
			.whatever_context("Error sending command")?;

		// The modulo is to avoid overflow
		*sent_counter = (*sent_counter + 1) % 32768;

		if unacknowledged_seq_num.len() < k as usize {
			unacknowledged_seq_num.push((*sent_counter, Instant::now()));
		} else {
			whatever!("Unacknowledged sequence number is full. Closing connection");
		}

		*unacknowledged_rcv_frames = 0;

		Ok(())
	}

	#[allow(clippy::too_many_arguments)]
	fn handle_receive_i_frame(
		i: &IFrame,
		received_counter: &mut u16,
		sent_counter: &mut u16,
		unacknowledged_seq_num: &mut Vec<(u16, Instant)>,
		out_buffer_full: &AtomicBool,
		t1_i: &mut Pin<&mut tokio::time::Sleep>,
		config: &ClientConfig,
		unacknowledged_rcv_frames: &mut u16,
	) -> Result<(), Error> {
		tracing::debug!("Received I frame: {i:?}");
		if i.send_sequence_number != *received_counter {
			whatever!(
				"Received I frame with wrong sequence number. Expected: {received_counter}, Received: {}",
				i.send_sequence_number
			);
		}

		Self::check_sequence_acknowledge(
			unacknowledged_seq_num,
			i.receive_sequence_number,
			*sent_counter,
		)
		.whatever_context("Error checking sequence acknowledge")?;

		out_buffer_full.store(
			unacknowledged_seq_num.len() >= config.protocol.k as usize,
			std::sync::atomic::Ordering::Relaxed,
		);

		if !unacknowledged_seq_num.is_empty() {
			t1_i.as_mut().reset(
				unacknowledged_seq_num
					.first()
					.whatever_context("Unacknowledged sequence number is empty")?
					.1 + config.protocol.t1,
			);
		}

		// The modulo is to avoid overflow
		*received_counter = (*received_counter + 1) % 32768;
		*unacknowledged_rcv_frames += 1;

		Ok(())
	}

	async fn handle_receive_u_frame(
		u: &UFrame,
		write_connection: &mut WriteHalf<Connection>,
		outstanding_test_fr_con_messages: &mut u16,
		t1_u: &mut Pin<&mut tokio::time::Sleep>,
	) -> Result<(), Error> {
		tracing::debug!("Received U frame: {u:?}");
		if u.test_fr_activation {
			Self::send_frame(write_connection, &TEST_FR_CON_FRAME)
				.await
				.whatever_context("Error sending test frame")?;
		} else if u.start_dt_activation {
			tracing::debug!("StartDT activation");
			//TODO: We already stated. We shouldn't be receiving this frame
			//TOD: What else should we do here?
			Self::send_frame(write_connection, &START_DT_CON_FRAME)
				.await
				.whatever_context("Error sending test frame")?;
		} else if u.stop_dt_activation {
			//TOD: What else should we do here?
			Self::send_frame(write_connection, &STOP_DT_CON_FRAME)
				.await
				.whatever_context("Error sending test frame")?;
		} else {
			//This is a confirmation frame. Lets unset the t1_u timer
			*outstanding_test_fr_con_messages = 0;
			t1_u.as_mut().reset(Instant::now() + *TIMER_UNSET);
		}

		Ok(())
	}
}

impl Debug for Client {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "Client {{receiving: {} }}", self.read_connection.is_some())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_empty_buffer_valid_sequence() {
		let mut k_buffer = Vec::new();
		let send_count = 100;

		// Valid: seq_no matches send_count when buffer is empty
		assert!(Client::check_sequence_acknowledge(&mut k_buffer, 100, send_count).is_ok());

		// Invalid: seq_no doesn't match send_count
		assert!(Client::check_sequence_acknowledge(&mut k_buffer, 101, send_count).is_err());
	}

	#[test]
	fn test_single_value_buffer() {
		let now = Instant::now();
		let k_buffer = vec![(100, now)];

		// Valid: seq_no matches the single value in buffer
		assert!(Client::check_sequence_acknowledge(&mut k_buffer.clone(), 100, 101).is_ok());

		// Invalid: seq_no doesn't match
		assert!(Client::check_sequence_acknowledge(&mut k_buffer.clone(), 101, 101).is_err());
	}

	#[test]
	fn test_normal_range_no_overflow() {
		let now = Instant::now();
		let k_buffer = vec![(100, now), (101, now), (102, now)];

		// Valid: seq_no within range
		assert!(Client::check_sequence_acknowledge(&mut k_buffer.clone(), 101, 103).is_ok());
		assert!(Client::check_sequence_acknowledge(&mut k_buffer.clone(), 102, 103).is_ok());

		// Invalid: seq_no outside range
		assert!(Client::check_sequence_acknowledge(&mut k_buffer.clone(), 98, 103).is_err());
		assert!(Client::check_sequence_acknowledge(&mut k_buffer.clone(), 103, 103).is_err());
	}

	#[test]
	fn test_overflow_scenario() {
		let now = Instant::now();
		let k_buffer = vec![(32766, now), (32767, now), (0, now), (1, now)];

		// Valid: seq_no in overflow range
		assert!(Client::check_sequence_acknowledge(&mut k_buffer.clone(), 32767, 2).is_ok());
		assert!(Client::check_sequence_acknowledge(&mut k_buffer.clone(), 0, 2).is_ok());
		assert!(Client::check_sequence_acknowledge(&mut k_buffer.clone(), 1, 2).is_ok());

		// Invalid: seq_no outside overflow range
		assert!(Client::check_sequence_acknowledge(&mut k_buffer.clone(), 32764, 2).is_err());
		assert!(Client::check_sequence_acknowledge(&mut k_buffer.clone(), 2, 2).is_err());
	}

	#[test]
	fn test_oldest_valid_sequence_number() {
		let now = Instant::now();
		let mut k_buffer = vec![(100, now), (101, now)];

		// Valid: seq_no equals oldest_valid_seq_no (99)
		assert!(Client::check_sequence_acknowledge(&mut k_buffer, 99, 102).is_ok());

		// Test with wraparound
		let mut k_buffer_wrap = vec![(0, now), (1, now)];

		// Valid: seq_no equals oldest_valid_seq_no (32767)
		assert!(Client::check_sequence_acknowledge(&mut k_buffer_wrap, 32767, 2).is_ok());
	}

	#[test]
	fn test_buffer_cleanup() {
		let now = Instant::now();
		let mut k_buffer = vec![(100, now), (101, now), (102, now)];

		// Confirm sequence number 101
		assert!(Client::check_sequence_acknowledge(&mut k_buffer, 101, 103).is_ok());

		// Buffer should now only contain 102
		assert_eq!(k_buffer, vec![(102, now)]);
	}

	#[test]
	fn test_multiple_cleanup() {
		let now = Instant::now();
		let mut k_buffer = vec![(100, now), (101, now), (102, now), (103, now)];

		// Confirm sequence number 102 (should remove 100, 101, 102)
		assert!(Client::check_sequence_acknowledge(&mut k_buffer, 102, 104).is_ok());

		// Buffer should now only contain 103
		assert_eq!(k_buffer, vec![(103, now)]);
	}

	#[test]
	fn test_overflow_cleanup() {
		let now = Instant::now();
		let mut k_buffer = vec![(32766, now), (32767, now), (0, now), (1, now)];

		// Confirm sequence number 32767
		assert!(Client::check_sequence_acknowledge(&mut k_buffer, 32767, 2).is_ok());

		// Buffer should now contain 0, 1
		assert_eq!(k_buffer, vec![(0, now), (1, now)]);
	}

	#[test]
	fn test_invalid_scenarios() {
		let now = Instant::now();
		let k_buffer = vec![(100, now), (101, now)];

		// Invalid: seq_no too low
		assert!(Client::check_sequence_acknowledge(&mut k_buffer.clone(), 98, 102).is_err());

		// Invalid: seq_no too high
		assert!(Client::check_sequence_acknowledge(&mut k_buffer.clone(), 103, 102).is_err());

		// Invalid: seq_no in middle but not in buffer
		assert!(Client::check_sequence_acknowledge(&mut k_buffer.clone(), 105, 102).is_err());
	}

	#[test]
	fn test_edge_cases() {
		let now = Instant::now();
		let mut k_buffer = vec![(0, now)];

		// Edge case: sequence number 0
		assert!(Client::check_sequence_acknowledge(&mut k_buffer, 0, 1).is_ok());

		// Edge case: sequence number 32767
		let mut k_buffer_max = vec![(32767, now)];
		assert!(Client::check_sequence_acknowledge(&mut k_buffer_max, 32767, 0).is_ok());
	}

	#[test]
	fn test_complex_overflow_scenario() {
		let now = Instant::now();
		let k_buffer = vec![(32765, now), (32766, now), (32767, now), (0, now), (1, now)];

		// Test various sequence numbers
		assert!(Client::check_sequence_acknowledge(&mut k_buffer.clone(), 32764, 2).is_ok()); // Last acknowledged
		assert!(Client::check_sequence_acknowledge(&mut k_buffer.clone(), 32765, 2).is_ok()); // Oldest
		assert!(Client::check_sequence_acknowledge(&mut k_buffer.clone(), 32767, 2).is_ok()); // Pre-overflow
		assert!(Client::check_sequence_acknowledge(&mut k_buffer.clone(), 0, 2).is_ok()); // Overflow point
		assert!(Client::check_sequence_acknowledge(&mut k_buffer.clone(), 1, 2).is_ok()); // Post-overflow

		// Invalid cases
		assert!(Client::check_sequence_acknowledge(&mut k_buffer.clone(), 32763, 2).is_err()); // Too old
		assert!(Client::check_sequence_acknowledge(&mut k_buffer.clone(), 2, 2).is_err()); // Too new
	}

	#[test]
	fn test_cleanup_function() {
		let now = Instant::now();
		let mut k_buffer = vec![(100, now), (101, now), (102, now), (103, now), (104, now)];

		// Cleanup up to sequence number 102
		assert!(Client::check_sequence_acknowledge(&mut k_buffer, 102, 103).is_ok());

		// Should only have 103, 104 left
		assert_eq!(k_buffer, vec![(103, now), (104, now)]);
	}

	#[test]
	fn test_cleanup_with_overflow() {
		let now = Instant::now();
		let mut k_buffer = vec![(32766, now), (32767, now), (0, now), (1, now), (2, now)];

		// Cleanup up to sequence number 0
		assert!(Client::check_sequence_acknowledge(&mut k_buffer, 0, 1).is_ok());

		// Should only have 1, 2 left
		assert_eq!(k_buffer, vec![(1, now), (2, now)]);
	}

	#[test]
	fn test_cleanup_empty_buffer() {
		let mut k_buffer = Vec::new();
		assert!(Client::check_sequence_acknowledge(&mut k_buffer, 100, 101).is_err());
		assert_eq!(k_buffer, vec![]);
	}

	#[test]
	fn test_cleanup_no_match() {
		let now = Instant::now();
		let mut k_buffer = vec![(100, now), (101, now), (102, now)];

		// Try to cleanup with sequence number that doesn't exist
		assert!(Client::check_sequence_acknowledge(&mut k_buffer, 98, 103).is_err());

		// Buffer should remain unchanged
		assert_eq!(k_buffer, vec![(100, now), (101, now), (102, now)]);
	}
}
