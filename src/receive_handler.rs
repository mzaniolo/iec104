use std::{
	collections::VecDeque,
	pin::Pin,
	sync::{
		Arc,
		atomic::{AtomicBool, Ordering},
	},
	time::Duration,
};

use lazy_static::lazy_static;
use snafu::{OptionExt as _, ResultExt as _, whatever};
use tokio::{
	io::{AsyncRead, AsyncReadExt as _, AsyncWrite, AsyncWriteExt as _, ReadHalf, WriteHalf},
	select,
	sync::{mpsc, oneshot},
	time::Instant,
};
use tracing::instrument;

use crate::{
	Connection, STOP_DT_ACT_FRAME, STOP_DT_CON_FRAME, TEST_FR_ACT_FRAME, TEST_FR_CON_FRAME,
	apdu::{APUD_MAX_LENGTH, Apdu, Frame, IFrame, SFrame, TELEGRAM_HEADER, UFrame},
	asdu::Asdu,
	config::ProtocolConfig,
	error::Error,
};

lazy_static! {
	static ref TIMER_UNSET: Duration = Duration::from_secs(2_600_000);
}

/// Checks if a received sequence number is valid and removes acknowledged
/// entries from the buffer. Kept as a free function (not a method) so it
/// doesn't require `ReceiveHandler`'s generic `C`.
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
			whatever!("Received frame with sequence number that is not in the unacknowledged list");
		}
	}

	//TODO: Fix it
	whatever!("Received frame with invalid sequence number");
}

/// Sends an APDU frame. Free function so callers don't need `ReceiveHandler`'s
/// generic `C`.
#[instrument(level = "debug", skip_all)]
pub(crate) async fn send_frame<W: AsyncWrite + Unpin>(
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

/// Receives an APDU. Free function so callers don't need `ReceiveHandler`'s
/// generic `C`.
#[instrument(level = "debug", skip_all)]
pub(crate) async fn receive_apdu<R: AsyncRead + Unpin>(
	connection: &mut R,
	buffer: &mut [u8; 255],
) -> Result<Apdu, Error> {
	connection.read_exact(&mut buffer[0..2]).await.whatever_context("Error receiving data")?;
	if buffer[0] != TELEGRAM_HEADER {
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

/// Reason enqueueing an ASDU was rejected by the receive-handler task.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendAsduQueueError {
	/// [`ProtocolConfig::max_pending_outgoing_asdu`](crate::config::ProtocolConfig::max_pending_outgoing_asdu)
	/// non-zero limit reached (waiting for `k` window / peer acks).
	PendingBufferFull,
}

/// Payload for [`ReceiveHandlerCommand::Asdu`]'s `reply` oneshot.
pub type SendAsduCommandAck = Result<(), SendAsduQueueError>;

#[derive(Debug)]
pub enum ReceiveHandlerCommand {
	Start,
	Stop,
	Test,
	Asdu { asdu: Asdu, reply: oneshot::Sender<SendAsduCommandAck> },
}

#[async_trait::async_trait]
pub trait ReceiveHandlerCallback: Send + Sync {
	async fn on_new_objects(&self, asdu: Asdu);
}

pub struct ReceiveHandler<'a, C: ReceiveHandlerCallback> {
	read_connection: &'a mut ReadHalf<Connection>,
	write_connection: &'a mut WriteHalf<Connection>,
	callback: Arc<C>,
	config: ProtocolConfig,
	rx: &'a mut mpsc::Receiver<ReceiveHandlerCommand>,
	out_buffer_full: Arc<AtomicBool>,
	t1_u: Pin<Box<tokio::time::Sleep>>,
	t1_i: Pin<Box<tokio::time::Sleep>>,
	t2: Pin<Box<tokio::time::Sleep>>,
	t3: Pin<Box<tokio::time::Sleep>>,
	unacknowledged_seq_num: VecDeque<(u16, Instant)>,
	/// ASDUs from [`ReceiveHandlerCommand::Asdu`] waiting for `k` window space.
	pending_outgoing_asdu: VecDeque<Asdu>,
	sent_counter: u16,
	received_counter: u16,
	unacknowledged_rcv_frames: u16,
	outstanding_test_fr_con_messages: u16,
}

impl<'a, C: ReceiveHandlerCallback> ReceiveHandler<'a, C> {
	pub fn new(
		read_connection: &'a mut ReadHalf<Connection>,
		write_connection: &'a mut WriteHalf<Connection>,
		callback: Arc<C>,
		config: ProtocolConfig,
		rx: &'a mut mpsc::Receiver<ReceiveHandlerCommand>,
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
			unacknowledged_seq_num: VecDeque::with_capacity(config.k as usize),
			pending_outgoing_asdu: config
				.max_pending_outgoing_asdu_limit()
				.map_or_else(VecDeque::new, |cap| VecDeque::with_capacity(cap.min(4096))),
			sent_counter: 0,
			received_counter: 0,
			unacknowledged_rcv_frames: 0,
			outstanding_test_fr_con_messages: 0,
			config,
		}
	}

	/// Tries to parse an APDU from the reading buffer. If the buffer don't have
	/// the full APDU bytes Ok(None) is returned
	#[instrument(level = "debug")]
	fn try_parse_apdu(reading_buffer: &mut VecDeque<u8>) -> Result<Option<Apdu>, Error> {
		if reading_buffer.front().is_none_or(|&b| b != TELEGRAM_HEADER) {
			whatever!("Invalid header");
		}
		let length = reading_buffer.get(1).whatever_context("Error getting length")?;
		if *length > APUD_MAX_LENGTH {
			whatever!("Invalid length: {length}");
		}
		if reading_buffer.len() >= (length + 2) as usize {
			let apdu_bytes: Vec<u8> = reading_buffer.drain(0..(length + 2) as usize).collect();
			Apdu::from_bytes(&apdu_bytes).map(Some)
		} else {
			Ok(None)
		}
	}

	#[instrument(level = "debug", skip_all)]
	pub async fn receive_task(mut self) -> Result<(), Error> {
		self.t3.as_mut().reset(Instant::now() + self.config.t3);

		let mut reading_buffer: VecDeque<u8> = VecDeque::with_capacity(512);
		let mut buffer = [0; 255];

		loop {
			select! {
				res = self.read_connection.read(&mut buffer)=>{
					let len = res.whatever_context("Error receiving data")?;
					if len == 0 {
						whatever!("Connection closed");
					}
					reading_buffer.extend(buffer[0..len].iter());
					let mut reset_t3 = false;

					while reading_buffer.len() > 2 && let Some(apdu) = Self::try_parse_apdu(&mut reading_buffer).whatever_context("Error parsing APDU")? {
						match apdu.frame {
							Frame::I(i) => {
								self.handle_receive_i_frame(&i)?;
								self.flush_pending_outgoing().await.whatever_context(
									"Error sending queued ASDUs after I-frame ack",
								)?;
								let new_t2_instant = Instant::now() + self.config.t2;
								if new_t2_instant < self.t2.deadline() {
									self.t2.as_mut().reset(new_t2_instant);
								}
								self.callback.on_new_objects(i.asdu).await;
							}
							Frame::S(s) => {
								self.handle_receive_s_frame(&s)?;
								self.flush_pending_outgoing().await.whatever_context(
									"Error sending queued ASDUs after S-frame ack",
								)?;
							}
							Frame::U(u) => {
								let should_stop = self.handle_receive_u_frame(&u).await?;
								if should_stop {
									return Ok(());
								}
							}
						}
						reset_t3 = true;
					}

					if reset_t3{
						self.t3.as_mut().reset(Instant::now() + self.config.t3);
					}
				}
				Some(cmd) = self.rx.recv() => {
					match cmd {
						ReceiveHandlerCommand::Asdu { asdu, reply } => {
							let ack = self.enqueue_incoming_asdu(asdu).await?;
							let _ = reply.send(ack);
						}
						ReceiveHandlerCommand::Stop => {
							send_frame(&mut self.write_connection, &STOP_DT_ACT_FRAME).await.whatever_context("Error sending stopDT activation")?;
							self.confirm_all_messages().await.whatever_context("Error confirming all messages")?;
							self.t1_u.as_mut().reset(Instant::now() + self.config.t1);
						},
						ReceiveHandlerCommand::Test => {
							self.send_test_frame().await.whatever_context("Error sending test frame")?;
						},
						_ => {
							tracing::error!("Received unexpected command: {cmd:?}");
						}
					}
				}
				_ = &mut self.t3 => {
					tracing::debug!("t3 timeout. Sending test frame");
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
			if self.unacknowledged_rcv_frames > self.config.w {
				tracing::debug!(
					"Received more than w frames without acknowledgement. Sending S frame"
				);
				self.confirm_all_messages().await?;
			}
		}
	}

	/// Flush, enqueue `asdu`, flush again. Returns `Ok(Err(PendingBufferFull))`
	/// when the pending queue is at
	/// [`ProtocolConfig::max_pending_outgoing_asdu`](crate::config::ProtocolConfig::max_pending_outgoing_asdu);
	/// returns `Err(Error)` on wire/protocol failure.
	async fn enqueue_incoming_asdu(&mut self, asdu: Asdu) -> Result<SendAsduCommandAck, Error> {
		self.flush_pending_outgoing().await.whatever_context("Error sending queued ASDUs")?;

		if let Some(limit) = self.config.max_pending_outgoing_asdu_limit()
			&& self.pending_outgoing_asdu.len() >= limit
		{
			return Ok(Err(SendAsduQueueError::PendingBufferFull));
		}

		self.pending_outgoing_asdu.push_back(asdu);
		self.flush_pending_outgoing().await.whatever_context("Error sending queued ASDUs")?;

		Ok(Ok(()))
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
		if unacknowledged_seq_num.len() >= k as usize {
			whatever!("internal error: I-format send requested with full k window");
		}

		let frame = Frame::I(IFrame {
			send_sequence_number: *sent_counter,
			receive_sequence_number: received_counter,
			asdu,
		});

		send_frame(write_connection, &frame).await.whatever_context("Error sending command")?;

		// The modulo is to avoid overflow
		*sent_counter = (*sent_counter + 1) % 32768;
		unacknowledged_seq_num.push_back((*sent_counter, Instant::now()));

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

		check_sequence_acknowledge(
			&mut self.unacknowledged_seq_num,
			i.receive_sequence_number,
			self.sent_counter,
		)
		.whatever_context("Error checking sequence acknowledge")?;

		self.out_buffer_full
			.store(self.unacknowledged_seq_num.len() >= self.config.k as usize, Ordering::Relaxed);

		self.t1_i.as_mut().reset(
			self.unacknowledged_seq_num
				.front()
				.map_or(Instant::now() + *TIMER_UNSET, |(_, time)| *time + self.config.t1),
		);

		// The modulo is to avoid overflow
		self.received_counter = (self.received_counter + 1) % 32768;
		self.unacknowledged_rcv_frames += 1;

		Ok(())
	}

	#[instrument(level = "debug", skip_all)]
	fn handle_receive_s_frame(&mut self, s: &SFrame) -> Result<(), Error> {
		tracing::debug!("Received S frame: {s:?}");
		check_sequence_acknowledge(
			&mut self.unacknowledged_seq_num,
			s.receive_sequence_number,
			self.sent_counter,
		)
		.whatever_context("Error checking sequence acknowledge")?;

		self.out_buffer_full
			.store(self.unacknowledged_seq_num.len() >= self.config.k as usize, Ordering::Relaxed);

		self.t1_i.as_mut().reset(
			self.unacknowledged_seq_num
				.front()
				.map_or(Instant::now() + *TIMER_UNSET, |(_, time)| *time + self.config.t1),
		);
		Ok(())
	}

	/// Sends as many queued [`Asdu`]s as the `k` window allows.
	///
	/// Each [`Self::send_one_asdu`] adds an entry to `unacknowledged_seq_num`,
	/// so the `len < k` check becomes false once the window is full even if
	/// the deque still has items.
	#[instrument(level = "debug", skip_all)]
	async fn flush_pending_outgoing(&mut self) -> Result<(), Error> {
		while self.unacknowledged_seq_num.len() < self.config.k as usize
			&& let Some(asdu) = self.pending_outgoing_asdu.pop_front()
		{
			self.send_one_asdu(asdu).await?;
		}
		Ok(())
	}

	async fn send_one_asdu(&mut self, asdu: Asdu) -> Result<(), Error> {
		Self::handle_send_asdu(
			asdu,
			&mut self.sent_counter,
			self.received_counter,
			self.write_connection,
			&mut self.unacknowledged_seq_num,
			self.config.k,
			&mut self.unacknowledged_rcv_frames,
		)
		.await?;
		self.out_buffer_full
			.store(self.unacknowledged_seq_num.len() >= self.config.k as usize, Ordering::Relaxed);
		self.t1_i.as_mut().reset(
			self.unacknowledged_seq_num
				.front()
				.map_or(Instant::now() + *TIMER_UNSET, |(_, time)| *time + self.config.t1),
		);
		Ok(())
	}

	#[instrument(level = "debug", skip_all)]
	async fn handle_receive_u_frame(&mut self, u: &UFrame) -> Result<bool, Error> {
		tracing::debug!("Received U frame: {u:?}");
		// Match exactly one flag — per IEC 60870-5-104 §5.3 a U-frame
		// carries exactly one of these six commands. Treating any
		// non-matched variant as TestFR-CON (the previous behavior)
		// silently cleared the t1_u timer on stray frames.
		match (
			u.test_fr_activation,
			u.test_fr_confirmation,
			u.start_dt_activation,
			u.start_dt_confirmation,
			u.stop_dt_activation,
			u.stop_dt_confirmation,
		) {
			(true, false, false, false, false, false) => {
				send_frame(&mut self.write_connection, &TEST_FR_CON_FRAME)
					.await
					.whatever_context("Error sending testFR confirmation")?;
				Ok(false)
			}
			(false, true, false, false, false, false) => {
				// Valid TestFR-CON: clear the t1_u timer.
				self.outstanding_test_fr_con_messages = 0;
				self.t1_u.as_mut().reset(Instant::now() + *TIMER_UNSET);
				Ok(false)
			}
			(false, false, true, false, false, false) => {
				// In-session StartDT-ACT is a protocol error per §5.3:
				// data transfer is already started, so re-confirming would
				// mask a peer fault. Close the connection instead.
				whatever!("Received StartDT activation while data transfer is already started")
			}
			(false, false, false, true, false, false) => {
				// StartDT-CON arriving outside `send_start_dt` is unexpected
				// for an established session.
				whatever!("Received unsolicited StartDT confirmation")
			}
			(false, false, false, false, true, false) => {
				send_frame(&mut self.write_connection, &STOP_DT_CON_FRAME)
					.await
					.whatever_context("Error sending stopDT confirmation")?;
				Ok(true)
			}
			(false, false, false, false, false, true) => {
				tracing::debug!("StopDT confirmation");
				Ok(true)
			}
			_ => whatever!("Invalid U-frame: zero or multiple flags set: {u:?}"),
		}
	}

	#[instrument(level = "debug", skip_all)]
	async fn send_test_frame(&mut self) -> Result<(), Error> {
		if self.outstanding_test_fr_con_messages > 2 {
			whatever!(
				"Outstanding test frame confirmation messages is greater than 2. Closing connection"
			);
		}
		send_frame(&mut self.write_connection, &TEST_FR_ACT_FRAME)
			.await
			.whatever_context("Error sending test frame")?;
		self.outstanding_test_fr_con_messages += 1;
		self.t3.as_mut().reset(Instant::now() + self.config.t3);
		self.t1_u.as_mut().reset(Instant::now() + self.config.t1);
		Ok(())
	}

	#[instrument(level = "debug", skip_all)]
	async fn confirm_all_messages(&mut self) -> Result<(), Error> {
		send_frame(
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
		assert!(check_sequence_acknowledge(&mut k_buffer, 100, send_count).is_ok());

		// Invalid: seq_no doesn't match send_count
		assert!(check_sequence_acknowledge(&mut k_buffer, 101, send_count).is_err());
	}

	#[test]
	fn test_single_value_buffer() {
		let now = Instant::now();
		let k_buffer = VecDeque::from([(100, now)]);

		// Valid: seq_no matches the single value in buffer
		assert!(check_sequence_acknowledge(&mut k_buffer.clone(), 100, 101).is_ok());

		// Invalid: seq_no doesn't match
		assert!(check_sequence_acknowledge(&mut k_buffer.clone(), 101, 101).is_err());
	}

	#[test]
	fn test_normal_range_no_overflow() {
		let now = Instant::now();
		let k_buffer = VecDeque::from([(100, now), (101, now), (102, now)]);

		// Valid: seq_no within range
		assert!(check_sequence_acknowledge(&mut k_buffer.clone(), 101, 103).is_ok());
		assert!(check_sequence_acknowledge(&mut k_buffer.clone(), 102, 103).is_ok());

		// Invalid: seq_no outside range
		assert!(check_sequence_acknowledge(&mut k_buffer.clone(), 98, 103).is_err());
		assert!(check_sequence_acknowledge(&mut k_buffer.clone(), 103, 103).is_err());
	}

	#[test]
	fn test_overflow_scenario() {
		let now = Instant::now();
		let k_buffer = VecDeque::from([(32766, now), (32767, now), (0, now), (1, now)]);

		// Valid: seq_no in overflow range
		assert!(check_sequence_acknowledge(&mut k_buffer.clone(), 32767, 2).is_ok());
		assert!(check_sequence_acknowledge(&mut k_buffer.clone(), 0, 2).is_ok());
		assert!(check_sequence_acknowledge(&mut k_buffer.clone(), 1, 2).is_ok());

		// Invalid: seq_no outside overflow range
		assert!(check_sequence_acknowledge(&mut k_buffer.clone(), 32764, 2).is_err());
		assert!(check_sequence_acknowledge(&mut k_buffer.clone(), 2, 2).is_err());
	}

	#[test]
	fn test_oldest_valid_sequence_number() {
		let now = Instant::now();
		let mut k_buffer = VecDeque::from([(100, now), (101, now)]);

		// Valid: seq_no equals oldest_valid_seq_no (99)
		assert!(check_sequence_acknowledge(&mut k_buffer, 99, 102).is_ok());

		// Test with wraparound
		let mut k_buffer_wrap = VecDeque::from([(0, now), (1, now)]);

		// Valid: seq_no equals oldest_valid_seq_no (32767)
		assert!(check_sequence_acknowledge(&mut k_buffer_wrap, 32767, 2).is_ok());
	}

	#[test]
	fn test_buffer_cleanup() {
		let now = Instant::now();
		let mut k_buffer = VecDeque::from([(100, now), (101, now), (102, now)]);

		// Confirm sequence number 101
		assert!(check_sequence_acknowledge(&mut k_buffer, 101, 103).is_ok());

		// Buffer should now only contain 102
		assert_eq!(k_buffer, vec![(102, now)]);
	}

	#[test]
	fn test_multiple_cleanup() {
		let now = Instant::now();
		let mut k_buffer = VecDeque::from([(100, now), (101, now), (102, now), (103, now)]);

		// Confirm sequence number 102 (should remove 100, 101, 102)
		assert!(check_sequence_acknowledge(&mut k_buffer, 102, 104).is_ok());

		// Buffer should now only contain 103
		assert_eq!(k_buffer, vec![(103, now)]);
	}

	#[test]
	fn test_overflow_cleanup() {
		let now = Instant::now();
		let mut k_buffer = VecDeque::from([(32766, now), (32767, now), (0, now), (1, now)]);

		// Confirm sequence number 32767
		assert!(check_sequence_acknowledge(&mut k_buffer, 32767, 2).is_ok());

		// Buffer should now contain 0, 1
		assert_eq!(k_buffer, vec![(0, now), (1, now)]);
	}

	#[test]
	fn test_invalid_scenarios() {
		let now = Instant::now();
		let k_buffer = VecDeque::from([(100, now), (101, now)]);

		// Invalid: seq_no too low
		assert!(check_sequence_acknowledge(&mut k_buffer.clone(), 98, 102).is_err());

		// Invalid: seq_no too high
		assert!(check_sequence_acknowledge(&mut k_buffer.clone(), 103, 102).is_err());

		// Invalid: seq_no in middle but not in buffer
		assert!(check_sequence_acknowledge(&mut k_buffer.clone(), 105, 102).is_err());
	}

	#[test]
	fn test_edge_cases() {
		let now = Instant::now();
		let mut k_buffer = VecDeque::from([(0, now)]);

		// Edge case: sequence number 0
		assert!(check_sequence_acknowledge(&mut k_buffer, 0, 1).is_ok());

		// Edge case: sequence number 32767
		let mut k_buffer_max = VecDeque::from([(32767, now)]);
		assert!(check_sequence_acknowledge(&mut k_buffer_max, 32767, 0).is_ok());
	}

	#[test]
	fn test_complex_overflow_scenario() {
		let now = Instant::now();
		let k_buffer =
			VecDeque::from([(32765, now), (32766, now), (32767, now), (0, now), (1, now)]);

		// Test various sequence numbers
		assert!(check_sequence_acknowledge(&mut k_buffer.clone(), 32764, 2).is_ok()); // Last acknowledged
		assert!(check_sequence_acknowledge(&mut k_buffer.clone(), 32765, 2).is_ok()); // Oldest
		assert!(check_sequence_acknowledge(&mut k_buffer.clone(), 32767, 2).is_ok()); // Pre-overflow
		assert!(check_sequence_acknowledge(&mut k_buffer.clone(), 0, 2).is_ok()); // Overflow point
		assert!(check_sequence_acknowledge(&mut k_buffer.clone(), 1, 2).is_ok()); // Post-overflow

		// Invalid cases
		assert!(check_sequence_acknowledge(&mut k_buffer.clone(), 32763, 2).is_err()); // Too old
		assert!(check_sequence_acknowledge(&mut k_buffer.clone(), 2, 2).is_err()); // Too new
	}

	#[test]
	fn test_cleanup_function() {
		let now = Instant::now();
		let mut k_buffer =
			VecDeque::from([(100, now), (101, now), (102, now), (103, now), (104, now)]);

		// Cleanup up to sequence number 102
		assert!(check_sequence_acknowledge(&mut k_buffer, 102, 103).is_ok());

		// Should only have 103, 104 left
		assert_eq!(k_buffer, vec![(103, now), (104, now)]);
	}

	#[test]
	fn test_cleanup_with_overflow() {
		let now = Instant::now();
		let mut k_buffer =
			VecDeque::from([(32766, now), (32767, now), (0, now), (1, now), (2, now)]);

		// Cleanup up to sequence number 0
		assert!(check_sequence_acknowledge(&mut k_buffer, 0, 1).is_ok());

		// Should only have 1, 2 left
		assert_eq!(k_buffer, vec![(1, now), (2, now)]);
	}

	#[test]
	fn test_cleanup_empty_buffer() {
		let mut k_buffer = VecDeque::new();
		assert!(check_sequence_acknowledge(&mut k_buffer, 100, 101).is_err());
		assert_eq!(k_buffer, vec![]);
	}

	#[test]
	fn test_cleanup_no_match() {
		let now = Instant::now();
		let mut k_buffer = VecDeque::from([(100, now), (101, now), (102, now)]);

		// Try to cleanup with sequence number that doesn't exist
		assert!(check_sequence_acknowledge(&mut k_buffer, 98, 103).is_err());

		// Buffer should remain unchanged
		assert_eq!(k_buffer, vec![(100, now), (101, now), (102, now)]);
	}
}
