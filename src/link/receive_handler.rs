use std::{
	collections::VecDeque,
	pin::Pin,
	sync::{Arc, atomic::AtomicBool},
	time::Duration,
};

use lazy_static::lazy_static;
use snafu::{OptionExt as _, ResultExt as _, whatever};
use tokio::{
	io::{AsyncRead, AsyncReadExt as _, AsyncWrite, AsyncWriteExt as _, ReadHalf, WriteHalf},
	select,
	sync::mpsc,
	time::Instant,
};
use tracing::instrument;

use crate::{
	apdu::{APUD_MAX_LENGTH, Apdu, Frame, IFrame, SFrame, TELEGRAN_HEADER, UFrame},
	asdu::Asdu,
	link::{
		Connection, OnNewObjects, START_DT_CON_FRAME, STOP_DT_ACT_FRAME, STOP_DT_CON_FRAME,
		TEST_FR_ACT_FRAME, TEST_FR_CON_FRAME, connection_handler::ConnectionHandlerCommand,
	},
	config::LinkConfig,
	error::Error,
};

lazy_static! {
	static ref TIMER_UNSET: Duration = Duration::from_secs(2_600_000);
}

pub struct ReceiveHandler<'a> {
	read_connection: &'a mut ReadHalf<Connection>,
	write_connection: &'a mut WriteHalf<Connection>,
	callback: Arc<dyn OnNewObjects + Send + Sync>,
	config: LinkConfig,
	rx: &'a mut mpsc::Receiver<ConnectionHandlerCommand>,
	out_buffer_full: Arc<AtomicBool>,
	t1_u: Pin<Box<tokio::time::Sleep>>,
	t1_i: Pin<Box<tokio::time::Sleep>>,
	t2: Pin<Box<tokio::time::Sleep>>,
	t3: Pin<Box<tokio::time::Sleep>>,
	unacknowledged_seq_num: VecDeque<(u16, Instant)>,
	sent_counter: u16,
	received_counter: u16,
	unacknowledged_rcv_frames: u16,
	outstanding_test_fr_con_messages: u16,
}

impl<'a> ReceiveHandler<'a> {
	pub fn new(
		read_connection: &'a mut ReadHalf<Connection>,
		write_connection: &'a mut WriteHalf<Connection>,
		callback: Arc<dyn OnNewObjects + Send + Sync>,
		config: LinkConfig,
		rx: &'a mut mpsc::Receiver<ConnectionHandlerCommand>,
		out_buffer_full: Arc<AtomicBool>,
	) -> Self {
		Self {
			read_connection,
			write_connection,
			callback,
			rx,
			out_buffer_full,
			t1_u: Box::pin(tokio::time::sleep(*TIMER_UNSET)),
			t1_i: Box::pin(tokio::time::sleep(*TIMER_UNSET)),
			t2: Box::pin(tokio::time::sleep(*TIMER_UNSET)),
			t3: Box::pin(tokio::time::sleep(*TIMER_UNSET)),
			unacknowledged_seq_num: VecDeque::with_capacity(config.protocol.k as usize),
			sent_counter: 0,
			received_counter: 0,
			unacknowledged_rcv_frames: 0,
			outstanding_test_fr_con_messages: 0,
			config,
		}
	}

	#[instrument(level = "debug", skip_all)]
	pub async fn send_frame<W: AsyncWrite + Unpin>(
		write_connection: &mut W,
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

	#[instrument(level = "debug", skip_all)]
	pub async fn receive_apdu<R: AsyncRead + Unpin>(
		connection: &mut R,
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

	#[instrument(level = "debug", skip_all)]
	pub async fn receive_task(mut self) -> Result<(), Error> {
		self.t3.as_mut().reset(Instant::now() + self.config.protocol.t3);

		let mut buffer = [0; 255];

		loop {
			select! {
				apdu = Self::receive_apdu(&mut self.read_connection,	&mut buffer) => {
					if let Ok(apdu) = apdu {
						match apdu.frame {
							Frame::I(i) => {
								self.handle_receive_i_frame(&i)?;
								self.callback.on_new_objects(i.asdu).await;
							}
							Frame::S(s) => {
								self.handle_receive_s_frame(&s)?;
							}
							Frame::U(u) => {
								let should_stop = self.handle_receive_u_frame(&u).await?;
								if should_stop {
									return Ok(());
								}
							}
						}
						self.t3.as_mut().reset(Instant::now() + self.config.protocol.t3);
					} else {
						whatever!("Error receiving APDU");
					}
				}
				Some(cmd) = self.rx.recv() => {
					match cmd {
						ConnectionHandlerCommand::Asdu(asdu) => {
							Self::handle_send_asdu(asdu, &mut self.sent_counter, self.received_counter, self.write_connection, &mut self.unacknowledged_seq_num, self.config.protocol.k, &mut self.unacknowledged_rcv_frames).await.whatever_context("Error sending command")?;
							self.out_buffer_full.store(self.unacknowledged_seq_num.len() >= self.config.protocol.k as usize, std::sync::atomic::Ordering::Relaxed);
							self.t2.as_mut().reset(self.unacknowledged_seq_num.front().whatever_context("Unacknowledged sequence number is empty")?.1 + self.config.protocol.t2);
						}
						ConnectionHandlerCommand::Stop => {
							Self::send_frame(&mut self.write_connection, &STOP_DT_ACT_FRAME).await.whatever_context("Error sending stopDT activation")?;
							self.confirm_all_messages().await.whatever_context("Error confirming all messages")?;
							self.t1_u.as_mut().reset(Instant::now() + self.config.protocol.t1);
						},
						ConnectionHandlerCommand::Test => {
							self.send_test_frame().await.whatever_context("Error sending test frame")?;
						},
						_ => {
							tracing::error!("Received unexpected command: {cmd:?}");
						}
					}
				}
				_ = &mut self.t3 => {
					tracing::debug!("t3 timeout. FAKE Sending test frame");
					self.send_test_frame().await.whatever_context("Error sending test frame for t3 timeout")?;
				}
				_ = &mut self.t2 => {
					tracing::debug!("t2 timeout. Sending S frame");
					self.confirm_all_messages().await.whatever_context("Error confirming all messages")?;
				}
				_ = &mut self.t1_u => {
					whatever!("t1 for u frames timeout");
				}
				_ = &mut self.t1_i => {
					whatever!("t1 for i frames timeout");
				}
			}
			if self.unacknowledged_rcv_frames > self.config.protocol.w {
				tracing::debug!(
					"Received more than w frames without acknowledgement. Sending S frame"
				);
				Self::send_frame(
					&mut self.write_connection,
					&Frame::S(SFrame { receive_sequence_number: self.received_counter }),
				)
				.await
				.whatever_context("Error sending S frame")?;
				self.unacknowledged_rcv_frames = 0;
				self.t2.as_mut().reset(Instant::now() + self.config.protocol.t2);
			}
		}
	}

	#[instrument(level = "debug")]
	async fn handle_send_asdu(
		asdu: Asdu,
		sent_counter: &mut u16,
		received_counter: u16,
		write_connection: &mut WriteHalf<Connection>,
		unacknowledged_seq_num: &mut VecDeque<(u16, Instant)>,
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
			unacknowledged_seq_num.push_back((*sent_counter, Instant::now()));
		} else {
			whatever!("Unacknowledged sequence number is full. Closing connection");
		}

		*unacknowledged_rcv_frames = 0;

		Ok(())
	}

	#[instrument(level = "debug", skip_all)]
	fn handle_receive_i_frame(&mut self, i: &IFrame) -> Result<(), Error> {
		tracing::debug!("Received I frame: {i:?}");
		if i.send_sequence_number != self.received_counter {
			whatever!(
				"Received I frame with wrong sequence number. Expected: {}, Received: {}",
				self.received_counter,
				i.send_sequence_number
			);
		}

		Self::check_sequence_acknowledge(
			&mut self.unacknowledged_seq_num,
			i.receive_sequence_number,
			self.sent_counter,
		)
		.whatever_context("Error checking sequence acknowledge")?;

		self.out_buffer_full.store(
			self.unacknowledged_seq_num.len() >= self.config.protocol.k as usize,
			std::sync::atomic::Ordering::Relaxed,
		);

		if !self.unacknowledged_seq_num.is_empty() {
			self.t1_i.as_mut().reset(
				self.unacknowledged_seq_num
					.front()
					.whatever_context("Unacknowledged sequence number is empty")?
					.1 + self.config.protocol.t1,
			);
		}

		// The modulo is to avoid overflow
		self.received_counter = (self.received_counter + 1) % 32768;
		self.unacknowledged_rcv_frames += 1;

		Ok(())
	}

	#[instrument(level = "debug", skip_all)]
	fn handle_receive_s_frame(&mut self, s: &SFrame) -> Result<(), Error> {
		tracing::debug!("Received S frame: {s:?}");
		Self::check_sequence_acknowledge(
			&mut self.unacknowledged_seq_num,
			s.receive_sequence_number,
			self.sent_counter,
		)
		.whatever_context("Error checking sequence acknowledge")?;

		self.out_buffer_full.store(
			self.unacknowledged_seq_num.len() >= self.config.protocol.k as usize,
			std::sync::atomic::Ordering::Relaxed,
		);

		if !self.unacknowledged_seq_num.is_empty() {
			self.t1_i.as_mut().reset(
				self.unacknowledged_seq_num
					.front()
					.whatever_context("Unacknowledged sequence number is empty")?
					.1 + self.config.protocol.t1,
			);
		}
		Ok(())
	}

	#[instrument(level = "debug", skip_all)]
	async fn handle_receive_u_frame(&mut self, u: &UFrame) -> Result<bool, Error> {
		tracing::debug!("Received U frame: {u:?}");
		if u.test_fr_activation {
			Self::send_frame(&mut self.write_connection, &TEST_FR_CON_FRAME)
				.await
				.whatever_context("Error sending test frame")?;
		} else if u.start_dt_activation {
			tracing::debug!("StartDT activation");
			if self.config.server {
				tracing::debug!("Server mode: sending StartDT confirmation");
				Self::send_frame(&mut self.write_connection, &START_DT_CON_FRAME)
					.await
					.whatever_context("Error sending startDT confirmation")?;
			} else {
				tracing::debug!("Client mode: received unexpected StartDT activation");
				whatever!("Received unexpected StartDT activation");
			}
		} else if u.start_dt_confirmation {
			tracing::debug!("StartDT confirmation");


		} else if u.stop_dt_activation {
			Self::send_frame(&mut self.write_connection, &STOP_DT_CON_FRAME)
				.await
				.whatever_context("Error sending test frame")?;
			return Ok(true);
		} else if u.stop_dt_confirmation {
			tracing::debug!("StopDT confirmation");
			return Ok(true);
		} else {
			//This is a confirmation frame. Lets unset the t1_u timer
			self.outstanding_test_fr_con_messages = 0;
			self.t1_u.as_mut().reset(Instant::now() + *TIMER_UNSET);
		}

		Ok(false)
	}

	#[instrument(level = "debug", skip_all)]
	async fn send_test_frame(&mut self) -> Result<(), Error> {
		if self.outstanding_test_fr_con_messages > 2 {
			whatever!(
				"Outstanding test frame confirmation messages is greater than 2. Closing connection"
			);
		}
		Self::send_frame(&mut self.write_connection, &TEST_FR_ACT_FRAME)
			.await
			.whatever_context("Error sending test frame")?;
		self.outstanding_test_fr_con_messages += 1;
		self.t3.as_mut().reset(Instant::now() + self.config.protocol.t3);
		self.t1_u.as_mut().reset(Instant::now() + self.config.protocol.t1);
		Ok(())
	}

	// check if received sequence number is valid and remove the acknowledged ones
	#[instrument(level = "debug")]
	fn check_sequence_acknowledge(
		unacknowledged_seq_num: &mut VecDeque<(u16, Instant)>,
		frame_rss: u16,
		sent_counter: u16,
	) -> Result<(), Error> {
		let mut is_valid = false;

		if let (Some(newest_seq_num), Some(oldest_seq_num)) =
			(unacknowledged_seq_num.back(), unacknowledged_seq_num.front())
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

	#[instrument(level = "debug", skip_all)]
	async fn confirm_all_messages(&mut self) -> Result<(), Error> {
		Self::send_frame(
			&mut self.write_connection,
			&Frame::S(SFrame { receive_sequence_number: self.received_counter }),
		)
		.await
		.whatever_context("Error sending S frame")?;
		self.unacknowledged_rcv_frames = 0;
		self.t2.as_mut().reset(Instant::now() + *TIMER_UNSET);
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_empty_buffer_valid_sequence() {
		let mut k_buffer = VecDeque::new();
		let send_count = 100;

		// Valid: seq_no matches send_count when buffer is empty
		assert!(ReceiveHandler::check_sequence_acknowledge(&mut k_buffer, 100, send_count).is_ok());

		// Invalid: seq_no doesn't match send_count
		assert!(
			ReceiveHandler::check_sequence_acknowledge(&mut k_buffer, 101, send_count).is_err()
		);
	}

	#[test]
	fn test_single_value_buffer() {
		let now = Instant::now();
		let k_buffer = VecDeque::from([(100, now)]);

		// Valid: seq_no matches the single value in buffer
		assert!(
			ReceiveHandler::check_sequence_acknowledge(&mut k_buffer.clone(), 100, 101).is_ok()
		);

		// Invalid: seq_no doesn't match
		assert!(
			ReceiveHandler::check_sequence_acknowledge(&mut k_buffer.clone(), 101, 101).is_err()
		);
	}

	#[test]
	fn test_normal_range_no_overflow() {
		let now = Instant::now();
		let k_buffer = VecDeque::from([(100, now), (101, now), (102, now)]);

		// Valid: seq_no within range
		assert!(
			ReceiveHandler::check_sequence_acknowledge(&mut k_buffer.clone(), 101, 103).is_ok()
		);
		assert!(
			ReceiveHandler::check_sequence_acknowledge(&mut k_buffer.clone(), 102, 103).is_ok()
		);

		// Invalid: seq_no outside range
		assert!(
			ReceiveHandler::check_sequence_acknowledge(&mut k_buffer.clone(), 98, 103).is_err()
		);
		assert!(
			ReceiveHandler::check_sequence_acknowledge(&mut k_buffer.clone(), 103, 103).is_err()
		);
	}

	#[test]
	fn test_overflow_scenario() {
		let now = Instant::now();
		let k_buffer = VecDeque::from([(32766, now), (32767, now), (0, now), (1, now)]);

		// Valid: seq_no in overflow range
		assert!(
			ReceiveHandler::check_sequence_acknowledge(&mut k_buffer.clone(), 32767, 2).is_ok()
		);
		assert!(ReceiveHandler::check_sequence_acknowledge(&mut k_buffer.clone(), 0, 2).is_ok());
		assert!(ReceiveHandler::check_sequence_acknowledge(&mut k_buffer.clone(), 1, 2).is_ok());

		// Invalid: seq_no outside overflow range
		assert!(
			ReceiveHandler::check_sequence_acknowledge(&mut k_buffer.clone(), 32764, 2).is_err()
		);
		assert!(ReceiveHandler::check_sequence_acknowledge(&mut k_buffer.clone(), 2, 2).is_err());
	}

	#[test]
	fn test_oldest_valid_sequence_number() {
		let now = Instant::now();
		let mut k_buffer = VecDeque::from([(100, now), (101, now)]);

		// Valid: seq_no equals oldest_valid_seq_no (99)
		assert!(ReceiveHandler::check_sequence_acknowledge(&mut k_buffer, 99, 102).is_ok());

		// Test with wraparound
		let mut k_buffer_wrap = VecDeque::from([(0, now), (1, now)]);

		// Valid: seq_no equals oldest_valid_seq_no (32767)
		assert!(ReceiveHandler::check_sequence_acknowledge(&mut k_buffer_wrap, 32767, 2).is_ok());
	}

	#[test]
	fn test_buffer_cleanup() {
		let now = Instant::now();
		let mut k_buffer = VecDeque::from([(100, now), (101, now), (102, now)]);

		// Confirm sequence number 101
		assert!(ReceiveHandler::check_sequence_acknowledge(&mut k_buffer, 101, 103).is_ok());

		// Buffer should now only contain 102
		assert_eq!(k_buffer, vec![(102, now)]);
	}

	#[test]
	fn test_multiple_cleanup() {
		let now = Instant::now();
		let mut k_buffer = VecDeque::from([(100, now), (101, now), (102, now), (103, now)]);

		// Confirm sequence number 102 (should remove 100, 101, 102)
		assert!(ReceiveHandler::check_sequence_acknowledge(&mut k_buffer, 102, 104).is_ok());

		// Buffer should now only contain 103
		assert_eq!(k_buffer, vec![(103, now)]);
	}

	#[test]
	fn test_overflow_cleanup() {
		let now = Instant::now();
		let mut k_buffer = VecDeque::from([(32766, now), (32767, now), (0, now), (1, now)]);

		// Confirm sequence number 32767
		assert!(ReceiveHandler::check_sequence_acknowledge(&mut k_buffer, 32767, 2).is_ok());

		// Buffer should now contain 0, 1
		assert_eq!(k_buffer, vec![(0, now), (1, now)]);
	}

	#[test]
	fn test_invalid_scenarios() {
		let now = Instant::now();
		let k_buffer = VecDeque::from([(100, now), (101, now)]);

		// Invalid: seq_no too low
		assert!(
			ReceiveHandler::check_sequence_acknowledge(&mut k_buffer.clone(), 98, 102).is_err()
		);

		// Invalid: seq_no too high
		assert!(
			ReceiveHandler::check_sequence_acknowledge(&mut k_buffer.clone(), 103, 102).is_err()
		);

		// Invalid: seq_no in middle but not in buffer
		assert!(
			ReceiveHandler::check_sequence_acknowledge(&mut k_buffer.clone(), 105, 102).is_err()
		);
	}

	#[test]
	fn test_edge_cases() {
		let now = Instant::now();
		let mut k_buffer = VecDeque::from([(0, now)]);

		// Edge case: sequence number 0
		assert!(ReceiveHandler::check_sequence_acknowledge(&mut k_buffer, 0, 1).is_ok());

		// Edge case: sequence number 32767
		let mut k_buffer_max = VecDeque::from([(32767, now)]);
		assert!(ReceiveHandler::check_sequence_acknowledge(&mut k_buffer_max, 32767, 0).is_ok());
	}

	#[test]
	fn test_complex_overflow_scenario() {
		let now = Instant::now();
		let k_buffer =
			VecDeque::from([(32765, now), (32766, now), (32767, now), (0, now), (1, now)]);

		// Test various sequence numbers
		assert!(
			ReceiveHandler::check_sequence_acknowledge(&mut k_buffer.clone(), 32764, 2).is_ok()
		); // Last acknowledged
		assert!(
			ReceiveHandler::check_sequence_acknowledge(&mut k_buffer.clone(), 32765, 2).is_ok()
		); // Oldest
		assert!(
			ReceiveHandler::check_sequence_acknowledge(&mut k_buffer.clone(), 32767, 2).is_ok()
		); // Pre-overflow
		assert!(ReceiveHandler::check_sequence_acknowledge(&mut k_buffer.clone(), 0, 2).is_ok()); // Overflow point
		assert!(ReceiveHandler::check_sequence_acknowledge(&mut k_buffer.clone(), 1, 2).is_ok()); // Post-overflow

		// Invalid cases
		assert!(
			ReceiveHandler::check_sequence_acknowledge(&mut k_buffer.clone(), 32763, 2).is_err()
		); // Too old
		assert!(ReceiveHandler::check_sequence_acknowledge(&mut k_buffer.clone(), 2, 2).is_err()); // Too new
	}

	#[test]
	fn test_cleanup_function() {
		let now = Instant::now();
		let mut k_buffer =
			VecDeque::from([(100, now), (101, now), (102, now), (103, now), (104, now)]);

		// Cleanup up to sequence number 102
		assert!(ReceiveHandler::check_sequence_acknowledge(&mut k_buffer, 102, 103).is_ok());

		// Should only have 103, 104 left
		assert_eq!(k_buffer, vec![(103, now), (104, now)]);
	}

	#[test]
	fn test_cleanup_with_overflow() {
		let now = Instant::now();
		let mut k_buffer =
			VecDeque::from([(32766, now), (32767, now), (0, now), (1, now), (2, now)]);

		// Cleanup up to sequence number 0
		assert!(ReceiveHandler::check_sequence_acknowledge(&mut k_buffer, 0, 1).is_ok());

		// Should only have 1, 2 left
		assert_eq!(k_buffer, vec![(1, now), (2, now)]);
	}

	#[test]
	fn test_cleanup_empty_buffer() {
		let mut k_buffer = VecDeque::new();
		assert!(ReceiveHandler::check_sequence_acknowledge(&mut k_buffer, 100, 101).is_err());
		assert_eq!(k_buffer, vec![]);
	}

	#[test]
	fn test_cleanup_no_match() {
		let now = Instant::now();
		let mut k_buffer = VecDeque::from([(100, now), (101, now), (102, now)]);

		// Try to cleanup with sequence number that doesn't exist
		assert!(ReceiveHandler::check_sequence_acknowledge(&mut k_buffer, 98, 103).is_err());

		// Buffer should remain unchanged
		assert_eq!(k_buffer, vec![(100, now), (101, now), (102, now)]);
	}
}
