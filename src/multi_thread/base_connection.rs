use std::{
	collections::VecDeque,
	sync::{Arc, Mutex, atomic},
};

use snafu::ResultExt;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::{apdu, cot, error::Error};

const CHANNEL_BUFFER_SIZE: usize = 1024;
const VEC_BUFFER_SIZE: usize = 1024;

// client::TEST_FR_CON_FRAME etc. are private, so I have to copy them as a workaround.
lazy_static::lazy_static! {
	static ref TEST_FR_CON_FRAME: apdu::Frame =
		apdu::Frame::U(apdu::UFrame { test_fr_confirmation: true, ..Default::default() });
	static ref START_DT_CON_FRAME: apdu::Frame =
		apdu::Frame::U(apdu::UFrame { start_dt_confirmation: true, ..Default::default() });
	static ref STOP_DT_CON_FRAME: apdu::Frame =
		apdu::Frame::U(apdu::UFrame { stop_dt_confirmation: true, ..Default::default() });
	static ref TEST_FR_ACT_FRAME: apdu::Frame =
		apdu::Frame::U(apdu::UFrame { test_fr_activation: true, ..Default::default() });
	static ref START_DT_ACT_FRAME: apdu::Frame =
		apdu::Frame::U(apdu::UFrame { start_dt_activation: true, ..Default::default() });
	static ref STOP_DT_ACT_FRAME: apdu::Frame =
		apdu::Frame::U(apdu::UFrame { stop_dt_activation: true, ..Default::default() });
}

#[async_trait::async_trait]
pub trait ConnectionCallbacks {
	async fn on_connection_event(&self, event: ConnectionEvent) -> Result<(), Error>;
	async fn on_finish_receive_once(&self) -> Result<(), Error>;
	async fn on_receive_i_frame(&self, iframe: apdu::IFrame) -> Result<Vec<apdu::Frame>, Error>;
	async fn on_error(&self, e: Error);
}

#[derive(Clone, Copy)]
pub(crate) enum ConnectionType {
	// TCP Client ( Master )
	Client,
	// TCP Server ( Slave )
	Server,
}

#[derive(Clone, Copy, Debug)]
pub enum ConnectionStatus {
	Idle,
	Inactive,
	Active,
	WaitingForSTARTDTCON,
	WaitingForSTOPDTCON,
}

#[derive(Clone, Copy, Debug)]
pub enum ConnectionEvent {
	Opened,
	Closed,
	STARTDTCONReceived,
	STOPDTCONReceived,
}

struct SentIFrame {
	sent_time: tokio::time::Instant,
	send_sn: u16,
}

#[derive(Clone)]
struct IFrameWatingConfirm {
	frame: apdu::IFrame,
	send_id: u16,
}

#[derive(Clone)]
pub(crate) struct Iec104Connection {
	// The connection type ( client / server ).
	connection_type: ConnectionType,
	// The protocol configuration.
	protocol: crate::config::ProtocolConfig,
	// The stop signal watch tx & rx.
	stop_watch_tx: tokio::sync::watch::Sender<bool>,
	stop_watch_rx: tokio::sync::watch::Receiver<bool>,
	// The APDU id to handle feedbacks.
	id_counter: Arc<atomic::AtomicU16>,
	// The channel sender to send APDU.
	send_mpsc_tx: tokio::sync::mpsc::Sender<(u16, bool, apdu::Frame)>,
	// The channel sender to feedback send result.
	send_result_broadcast_tx: tokio::sync::broadcast::Sender<(u16, Result<(), String>)>,
	// The channel sender to feedback confirm info.
	send_confirm_broadcast_tx: tokio::sync::broadcast::Sender<(u16, cot::Cot, bool)>,
	// The status change watch tx & rx.
	connection_status_watch_tx: tokio::sync::watch::Sender<ConnectionStatus>,
	connection_status_watch_rx: tokio::sync::watch::Receiver<ConnectionStatus>,
	// The sent counter.
	sent_counter: Arc<atomic::AtomicU16>,
	// The received counter.
	received_counter: Arc<atomic::AtomicU16>,
	// The number of received but not yet unconfirmed I frames.
	unconfirmed_received_i_frames: Arc<atomic::AtomicU16>,
	// The time of creation.
	create_time: tokio::time::Instant,
	// The time of first received but not yet unconfirmed I frame (in milliseconds since creation, negative means no unconfirmed I frames).
	first_unconfirmed_i_frame_ms: Arc<atomic::AtomicI64>,
	// The time of coming t3 timeout (in milliseconds since creation).
	t3_timeout_ms: Arc<atomic::AtomicU64>,
	// The time of coming U frame timeout (in milliseconds since creation, negative means no U frame timeout).
	u_frame_timeout_ms: Arc<atomic::AtomicI64>,
	// The number of unconfirmed test messages.
	unconfirmed_test_messages: Arc<atomic::AtomicU16>,
	// The callbacks.
	callbacks: Arc<dyn ConnectionCallbacks + Send + Sync>,
	// The I frames sent but not yet confirmed.
	sent_i_frames: Arc<Mutex<VecDeque<SentIFrame>>>,
	// The I frames waiting confirmation.
	i_frames_waiting_confirm: Arc<Mutex<Vec<IFrameWatingConfirm>>>,
}

impl Iec104Connection {
	pub(crate) async fn new(
		stream: tokio::net::TcpStream,
		connection_type: ConnectionType,
		protocol: crate::config::ProtocolConfig,
		callbacks: Arc<dyn ConnectionCallbacks + Send + Sync>,
	) -> Result<Self, Error> {
		let (stop_watch_tx, stop_watch_rx) = tokio::sync::watch::channel(false);
		let (send_mpsc_tx, send_mpsc_rx) = tokio::sync::mpsc::channel(CHANNEL_BUFFER_SIZE);
		let (connection_status_watch_tx, connection_status_watch_rx) =
			tokio::sync::watch::channel(ConnectionStatus::Idle);
		let (send_result_broadcast_tx, _) = tokio::sync::broadcast::channel(CHANNEL_BUFFER_SIZE);
		let (send_confirm_broadcast_tx, _send_confirm_broadcast_rx) =
			tokio::sync::broadcast::channel(CHANNEL_BUFFER_SIZE);
		let (read_half, write_half) = stream.into_split();
		let mut connection = Iec104Connection {
			t3_timeout_ms: Arc::new(atomic::AtomicU64::new(protocol.t3.as_millis() as u64)),
			connection_type,
			protocol,
			stop_watch_tx,
			stop_watch_rx,
			id_counter: Arc::new(atomic::AtomicU16::new(0)),
			send_mpsc_tx,
			send_result_broadcast_tx,
			send_confirm_broadcast_tx,
			connection_status_watch_tx,
			connection_status_watch_rx,
			sent_counter: Arc::new(atomic::AtomicU16::new(0)),
			received_counter: Arc::new(atomic::AtomicU16::new(0)),
			unconfirmed_received_i_frames: Arc::new(atomic::AtomicU16::new(0)),
			create_time: tokio::time::Instant::now(),
			first_unconfirmed_i_frame_ms: Arc::new(atomic::AtomicI64::new(-1)),
			u_frame_timeout_ms: Arc::new(atomic::AtomicI64::new(-1)),
			unconfirmed_test_messages: Arc::new(atomic::AtomicU16::new(0)),
			callbacks,
			sent_i_frames: Arc::new(Mutex::new(VecDeque::new())),
			i_frames_waiting_confirm: Arc::new(Mutex::new(Vec::new())),
		};
		let read_thread_connection = connection.clone();
		let write_thread_connection = connection.clone();
		let timer_thread_connection = connection.clone();
		tokio::task::spawn(read_thread_connection.read_thread(read_half));
		tokio::task::spawn(write_thread_connection.write_thread(send_mpsc_rx, write_half));
		connection.callbacks.on_connection_event(ConnectionEvent::Opened).await?;
		if let ConnectionType::Client = connection.connection_type {
			connection.send(START_DT_ACT_FRAME.to_owned()).await?;
		}
		connection.wait_ready_or_timeout(false).await?;
		tokio::task::spawn(timer_thread_connection.timer_thread());
		return Ok(connection);
	}
	pub(crate) fn status(&self) -> ConnectionStatus {
		return *self.connection_status_watch_rx.borrow();
	}
	pub(crate) fn is_closed(&self) -> bool {
		match *self.connection_status_watch_rx.borrow() {
			ConnectionStatus::Inactive => {
				return true;
			}
			_ => {}
		}
		if *self.stop_watch_rx.borrow() {
			return true;
		}
		return false;
	}
	pub(crate) async fn stop(&self) -> Result<(), Error> {
		let frame = self.confirm_outstanding_messages();
		self.send(apdu::Frame::S(frame)).await?;
		self.send(STOP_DT_ACT_FRAME.to_owned()).await?;
		return Ok(());
	}
	pub(crate) fn close(&self) {
		self.stop_watch_tx.send_replace(true);
	}
	fn is_ready(&self, is_startdt: bool) -> Result<bool, Error> {
		if *self.stop_watch_rx.borrow() {
			snafu::whatever!("Connection closed");
		}
		match *self.connection_status_watch_rx.borrow() {
			ConnectionStatus::Inactive => {
				snafu::whatever!("Connection closed");
			}
			ConnectionStatus::Active => {
				return Ok(true);
			}
			ConnectionStatus::Idle => {
				return Ok(is_startdt);
			}
			_ => {
				return Ok(false);
			}
		}
	}
	async fn wait_ready(&mut self, is_startdt: bool) -> Result<(), Error> {
		self.connection_status_watch_rx.mark_unchanged();
		loop {
			if self.is_ready(is_startdt)? {
				return Ok(());
			}
			self.connection_status_watch_rx.changed().await.whatever_context("Channel error")?;
		}
	}
	async fn wait_ready_or_timeout(&mut self, is_startdt: bool) -> Result<(), Error> {
		if self.is_ready(is_startdt)? {
			return Ok(());
		}
		let t1 = self.protocol.t1.clone();
		tokio::select! {
			res=self.wait_ready(is_startdt) => {
				return res;
			}
			_=tokio::time::sleep(t1) => {
				snafu::whatever!("Wait ready timeout ( t1 = {} s)", t1.as_secs());
			}
		}
	}
	async fn wait_send_result(
		&self,
		id: u16,
		rx: &mut tokio::sync::broadcast::Receiver<(u16, Result<(), String>)>,
	) -> Result<(), String> {
		loop {
			match rx.recv().await {
				Ok((i, res)) => {
					if i == id {
						return res;
					}
				}
				Err(e) => {
					self.stop_watch_tx.send_replace(true);
					return Err(e.to_string());
				}
			}
		}
	}
	async fn wait_send_confirm(
		&self,
		id: u16,
		rx: &mut tokio::sync::broadcast::Receiver<(u16, cot::Cot, bool)>,
		allow_negative: bool,
	) -> Result<(cot::Cot, bool), String> {
		loop {
			match rx.recv().await {
				Ok((i, cot, neg)) => {
					if i == id {
						if !allow_negative {
							if neg {
								return Err("Nagative confirm".to_string());
							}
							match cot {
								cot::Cot::UnknownType => {
									return Err("Response: unknown type".to_string());
								}
								cot::Cot::UnknownCause => {
									return Err("Response: unknown cause".to_string());
								}
								cot::Cot::UnknownAsduAddress => {
									return Err("Response: unknown ASDU address".to_string());
								}
								cot::Cot::UnknownObjectAddress => {
									return Err("Response: unknown object address".to_string());
								}
								_ => {}
							}
						}
						return Ok((cot, !neg));
					}
				}
				Err(e) => {
					self.stop_watch_tx.send_replace(true);
					return Err(e.to_string());
				}
			}
		}
	}
	async fn send_base(&self, frame: apdu::Frame, need_confirm: bool) -> Result<u16, Error> {
		if !self.is_ready(frame == *START_DT_ACT_FRAME)? {
			self.stop_watch_tx.send_replace(true);
			snafu::whatever!("Connection closed");
		}
		let send_id = self.id_counter.fetch_add(1, atomic::Ordering::SeqCst);
		let mut send_result_broadcast_rx = self.send_result_broadcast_tx.subscribe();
		match self.send_mpsc_tx.send_timeout((send_id, need_confirm, frame), self.protocol.t2).await
		{
			Ok(_) => {
				tokio::select! {
					res=self.wait_send_result(send_id, &mut send_result_broadcast_rx)=>{
						if let Err(e)=res{
							snafu::whatever!("{}",e);
						}else{
							return Ok(send_id);
						}
					}
					_=tokio::time::sleep(self.protocol.t2) => {
						snafu::whatever!("Send timeout ( t2 = {} s)", self.protocol.t2.as_secs());
					}
				}
			}
			Err(_) => {
				self.stop_watch_tx.send_replace(true);
				snafu::whatever!("Send timeout ( t2 = {} s)", self.protocol.t2.as_secs());
			}
		}
	}
	pub(crate) async fn send(&self, frame: apdu::Frame) -> Result<u16, Error> {
		return self.send_base(frame, false).await;
	}
	pub(crate) async fn send_owned(self, frame: apdu::Frame) -> Result<u16, Error> {
		return self.send_base(frame, false).await;
	}
	pub(crate) async fn send_and_wait_confirm(
		&self,
		frame: apdu::Frame,
		allow_negative: bool,
	) -> Result<(cot::Cot, bool), Error> {
		let send_id = self.send_base(frame, true).await?;
		let mut send_confirm_broadcast_rx = self.send_confirm_broadcast_tx.subscribe();
		tokio::select! {
			res=self.wait_send_confirm(send_id, &mut send_confirm_broadcast_rx,allow_negative)=>{
				match res{
					Ok(result) => {
						return Ok(result);
					}
					Err(e) => {
						snafu::whatever!("{}",e);
					}
				}
			}
			_=tokio::time::sleep(self.protocol.t1) => {
				snafu::whatever!("Confirmation timeout ( t1 = {} s)", self.protocol.t1.as_secs());
			}
		}
	}
	pub(crate) fn load_sent_counter(&self) -> u16 {
		return self.sent_counter.load(atomic::Ordering::SeqCst) & 0x7fff;
	}
	pub(crate) fn fetch_add_sent_counter(&self) -> u16 {
		return self.sent_counter.fetch_add(1, atomic::Ordering::SeqCst) & 0x7fff;
	}
	pub(crate) fn load_received_counter(&self) -> u16 {
		return self.received_counter.load(atomic::Ordering::SeqCst) & 0x7fff;
	}
	pub(crate) fn fetch_add_received_counter(&self) -> u16 {
		return self.received_counter.fetch_add(1, atomic::Ordering::SeqCst) & 0x7fff;
	}
	fn check_incoming_sequence_number(&self, seq_no: u16) -> Result<(), Error> {
		let current_tx = self.load_sent_counter();
		match self.sent_i_frames.lock() {
			Ok(mut sent_i_frames) => {
				if sent_i_frames.is_empty() {
					if seq_no == current_tx {
						return Ok(());
					}
				} else {
					match sent_i_frames.iter().position(|m| m.send_sn == seq_no) {
						Some(ind) => {
							for _ in 0..ind + 1 {
								sent_i_frames.pop_front();
							}
							return Ok(());
						}
						None => {
							let oldest_seq_no = sent_i_frames.front().unwrap().send_sn;
							let oldest_valid_seq_no = if oldest_seq_no == 0 {
								0x7fff
							} else {
								(oldest_seq_no - 1) & 0x7fff
							};
							if oldest_valid_seq_no == seq_no {
								return Ok(());
							}
						}
					}
				}
			}
			Err(e) => {
				snafu::whatever!("Lock error {}", e);
			}
		}
		snafu::whatever!(
			"Received frame with wrong sequence number, expected: {}, received: {}",
			self.load_received_counter(),
			seq_no
		);
	}
	fn confirm_outstanding_messages(&self) -> apdu::SFrame {
		self.first_unconfirmed_i_frame_ms.store(-1, atomic::Ordering::SeqCst);
		self.unconfirmed_received_i_frames.store(0, atomic::Ordering::SeqCst);
		return apdu::SFrame { receive_sequence_number: 0 };
	}
	fn instant_to_ms_u64(&self, instant: &tokio::time::Instant) -> Result<u64, Error> {
		match instant.checked_duration_since(self.create_time) {
			Some(d) => {
				let ms = d.as_millis();
				if ms > u64::MAX as u128 {
					snafu::whatever!("Time conversion overflow");
				} else {
					return Ok(ms as u64);
				}
			}
			None => {
				snafu::whatever!("Time conversion error");
			}
		}
	}
	fn instant_to_ms_i64(&self, instant: &tokio::time::Instant) -> Result<i64, Error> {
		match instant.checked_duration_since(self.create_time) {
			Some(d) => {
				let ms = d.as_millis();
				if ms > i64::MAX as u128 {
					snafu::whatever!("Time conversion overflow");
				} else {
					return Ok(ms as i64);
				}
			}
			None => {
				snafu::whatever!("Time conversion error");
			}
		}
	}
	fn ms_u64_to_instant(&self, ms: u64) -> Result<tokio::time::Instant, Error> {
		match self.create_time.checked_add(tokio::time::Duration::from_millis(ms)) {
			Some(i) => {
				return Ok(i);
			}
			None => {
				snafu::whatever!("Time conversion error");
			}
		}
	}
	fn ms_i64_to_instant(&self, ms: i64) -> Result<Option<tokio::time::Instant>, Error> {
		if ms < 0 {
			return Ok(None);
		} else {
			match self.create_time.checked_add(tokio::time::Duration::from_millis(ms as u64)) {
				Some(i) => {
					return Ok(Some(i));
				}
				None => {
					snafu::whatever!("Time conversion error");
				}
			}
		}
	}
	fn check_timeouts(&self) -> Result<Vec<apdu::Frame>, Error> {
		let mut result = Vec::new();
		let current_time = tokio::time::Instant::now();
		let next_t3_timeout =
			self.ms_u64_to_instant(self.t3_timeout_ms.load(atomic::Ordering::SeqCst))?;
		if current_time > next_t3_timeout {
			if self.unconfirmed_test_messages.load(atomic::Ordering::SeqCst) > 2 {
				snafu::whatever!("TEST_FR_CON frame timeout");
			} else {
				result.push(TEST_FR_ACT_FRAME.to_owned());
				self.u_frame_timeout_ms.store(
					self.instant_to_ms_i64(&(current_time + self.protocol.t1))?,
					atomic::Ordering::SeqCst,
				);
				self.unconfirmed_test_messages.fetch_add(1, atomic::Ordering::SeqCst);
				self.t3_timeout_ms.store(
					self.instant_to_ms_u64(&(current_time + self.protocol.t3))?,
					atomic::Ordering::SeqCst,
				);
			}
		}
		if self.unconfirmed_received_i_frames.load(atomic::Ordering::SeqCst) > 0 {
			if let Some(first_confirmation_time) = self.ms_i64_to_instant(
				self.first_unconfirmed_i_frame_ms.load(atomic::Ordering::SeqCst),
			)? {
				if current_time > first_confirmation_time
					&& current_time - first_confirmation_time >= self.protocol.t2
				{
					result.push(apdu::Frame::S(self.confirm_outstanding_messages()));
				}
			}
		}
		if let Some(u_message_timeout) =
			self.ms_i64_to_instant(self.u_frame_timeout_ms.load(atomic::Ordering::SeqCst))?
		{
			if current_time > u_message_timeout {
				snafu::whatever!("U frame timeout ( t1 = {} s)", self.protocol.t1.as_secs());
			}
		}
		match self.sent_i_frames.lock() {
			Ok(sent_i_frames) => {
				if !sent_i_frames.is_empty() {
					if current_time - sent_i_frames.front().unwrap().sent_time >= self.protocol.t1 {
						snafu::whatever!(
							"I frame timeout ( t1 = {} s)",
							self.protocol.t1.as_secs()
						);
					}
				}
			}
			Err(e) => {
				snafu::whatever!("Lock error {}", e);
			}
		}
		return Ok(result);
	}
	async fn handle_message(&self, dat: Vec<u8>, cache: &mut Vec<u8>) -> Result<(), Error> {
		if cache.is_empty() {
			*cache = dat;
		} else {
			cache.extend_from_slice(&dat);
		}
		if cache.len() < 2 {
			return Ok(());
		}
		let mut telegrams = vec![];
		let mut last_start_i = usize::MAX;
		let mut i = 0;
		while i < cache.len() - 1 {
			if cache[i] == 104 {
				let len = cache[i + 1] as usize;
				if cache.len() < len + i + 2 {
					last_start_i = i;
					break;
				} else {
					telegrams.push(
						apdu::Apdu::from_bytes(&cache[i..i + len + 2])
							.whatever_context("APDU deserilize error")?,
					);
					i += len + 2;
					continue;
				}
			}
			i += 1;
		}
		if last_start_i == usize::MAX {
			cache.clear();
		} else if last_start_i > 0 {
			*cache = cache.split_off(last_start_i);
		}
		let mut responses = Vec::new();
		for telegram in telegrams {
			match telegram.frame {
				apdu::Frame::I(iframe) => {
					if self.first_unconfirmed_i_frame_ms.load(atomic::Ordering::SeqCst) < 0 {
						self.first_unconfirmed_i_frame_ms.store(
							self.instant_to_ms_i64(&tokio::time::Instant::now())?,
							atomic::Ordering::SeqCst,
						);
					}
					let expected_send_sequence_number = self.load_received_counter();
					if iframe.send_sequence_number != expected_send_sequence_number {
						snafu::whatever!(
							"Sequence number error, expected {}, got {}",
							self.load_received_counter(),
							expected_send_sequence_number
						);
					}
					let mut confirm_result = None;
					let cot = iframe.asdu.cot;
					if cot == cot::Cot::ActivationConfirmation
						|| cot == cot::Cot::UnknownType
						|| cot == cot::Cot::UnknownCause
						|| cot == cot::Cot::UnknownAsduAddress
						|| cot == cot::Cot::UnknownObjectAddress
					{
						match self.i_frames_waiting_confirm.lock() {
							Ok(mut i_frames_waiting_confirm) => {
								if let Some(ind) = i_frames_waiting_confirm.iter().position(|f| {
									f.frame.asdu.type_id == iframe.asdu.type_id
										&& f.frame.asdu.address_field == iframe.asdu.address_field
										&& f.frame.asdu.sequence == iframe.asdu.sequence
										&& f.frame.asdu.information_objects
											== iframe.asdu.information_objects
								}) {
									confirm_result = Some((
										i_frames_waiting_confirm[ind].send_id,
										cot,
										iframe.asdu.positive,
									));
									i_frames_waiting_confirm.remove(ind);
								}
							}
							Err(e) => {
								snafu::whatever!("Lock error {}", e);
							}
						}
					}
					self.check_incoming_sequence_number(iframe.receive_sequence_number)?;
					self.fetch_add_received_counter();
					self.unconfirmed_received_i_frames.fetch_add(1, atomic::Ordering::SeqCst);
					if let Some(to_send) = confirm_result {
						self.send_confirm_broadcast_tx
							.send(to_send)
							.whatever_context("Channel error")?;
					}
					let mut custom_responses = self.callbacks.on_receive_i_frame(iframe).await?;
					if !custom_responses.is_empty() {
						responses.append(&mut custom_responses);
					}
				}
				apdu::Frame::S(sframe) => {
					self.check_incoming_sequence_number(sframe.receive_sequence_number)?;
				}
				apdu::Frame::U(uframe) => {
					self.u_frame_timeout_ms.store(-1, atomic::Ordering::SeqCst);
					if uframe.test_fr_confirmation {
						self.unconfirmed_test_messages.store(0, atomic::Ordering::SeqCst);
					} else if uframe.test_fr_activation {
						self.send(TEST_FR_CON_FRAME.to_owned()).await?;
					} else if uframe.start_dt_confirmation {
						self.connection_status_watch_tx.send_replace(ConnectionStatus::Active);
						self.callbacks
							.on_connection_event(ConnectionEvent::STARTDTCONReceived)
							.await?;
					} else if uframe.start_dt_activation {
						self.connection_status_watch_tx.send_replace(ConnectionStatus::Active);
						self.send(START_DT_CON_FRAME.to_owned()).await?;
					} else if uframe.stop_dt_confirmation {
						self.connection_status_watch_tx.send_replace(ConnectionStatus::Inactive);
						self.stop_watch_tx.send_replace(true);
						self.callbacks
							.on_connection_event(ConnectionEvent::STOPDTCONReceived)
							.await?;
					} else if uframe.stop_dt_activation {
						self.send(STOP_DT_CON_FRAME.to_owned()).await?;
						self.connection_status_watch_tx.send_replace(ConnectionStatus::Inactive);
						self.stop_watch_tx.send_replace(true);
						self.callbacks
							.on_connection_event(ConnectionEvent::STOPDTCONReceived)
							.await?;
					}
				}
			}
		}
		if self.unconfirmed_received_i_frames.load(atomic::Ordering::SeqCst) >= self.protocol.w {
			responses.insert(0, apdu::Frame::S(self.confirm_outstanding_messages()));
		}
        for telegram in responses{
            self.send(telegram).await?;
        }
		if cache.len() > 0 && cache[0] != 104 {
			snafu::whatever!("Unexpected start byte: {:#02X}.", cache[0]);
		}
		return self.callbacks.on_finish_receive_once().await;
	}
	async fn read_thread(mut self, mut read_half: tokio::net::tcp::OwnedReadHalf) {
		let mut cache = Vec::new();
		let mut buffer = [0u8; VEC_BUFFER_SIZE];
		loop {
			tokio::select! {
				res = read_half.read(&mut buffer) =>{
					match res{
						Ok(len) => {
							if len > 0{
								if let Err(e)=self.handle_message(buffer[..len].to_vec(), &mut cache).await{
									self.callbacks.on_error(snafu::FromString::with_source(e.into(), "Handling received data failed.".to_string())).await;
									break;
								}
							}
						}
						Err(e) => {
							self.callbacks.on_error(snafu::FromString::with_source(e.into(), "TCP read error.".to_string())).await;
							break;
						}
					}
				}
				res=self.stop_watch_rx.changed()=>{
					if let Err(_)=res{
						self.stop_watch_tx.send_replace(true);
						break;
					}
					if *self.stop_watch_rx.borrow_and_update() {
						break;
					}
				}
			}
		}
		self.stop_watch_tx.send_replace(true);
		self.connection_status_watch_tx.send_replace(ConnectionStatus::Inactive);
	}
	async fn write_thread(
		mut self,
		mut send_mpsc_rx: tokio::sync::mpsc::Receiver<(u16, bool, apdu::Frame)>,
		mut write_half: tokio::net::tcp::OwnedWriteHalf,
	) {
		loop {
			tokio::select! {
				res = send_mpsc_rx.recv()=>{
					match res{
						Some((send_id,need_confirm,mut frame)) => {
							let is_start_dt= frame==*START_DT_ACT_FRAME;
							let is_stop_dt= frame==*STOP_DT_ACT_FRAME;
							match self.is_ready(is_start_dt){
								Ok(ready)=>{
									if ready {
										match &mut frame{
											apdu::Frame::I(iframe) => {
												iframe.send_sequence_number=self.fetch_add_sent_counter();
												iframe.receive_sequence_number=self.load_received_counter();
												if need_confirm{
													match self.i_frames_waiting_confirm.lock() {
														Ok(mut i_frames_waiting_confirm) => {
															i_frames_waiting_confirm.push(IFrameWatingConfirm { frame: iframe.clone(), send_id });
														}
														Err(e) => {
															self.stop_watch_tx.send_replace(true);
															let _=self.send_result_broadcast_tx.send((send_id,Err(format!("Lock error. {}",e))));
															break;
														}
													}
												}
												match self.sent_i_frames.lock(){
													Ok(mut sent_i_frames) => {
														sent_i_frames.push_back(SentIFrame {
															sent_time: tokio::time::Instant::now(),
															send_sn:if iframe.send_sequence_number>=0x7fff { 0 } else { iframe.send_sequence_number+1 },
														});
													}
													Err(e) => {
														self.stop_watch_tx.send_replace(true);
														let _=self.send_result_broadcast_tx.send((send_id,Err(format!("Lock error. {}",e))));
														break;
													}
												}
											}
											apdu::Frame::S(sframe) => {
												sframe.receive_sequence_number=self.load_received_counter();
											}
											apdu::Frame::U(_) => {},
										}
										match frame.to_apdu_bytes(){
											Ok(bytes) => {
												match write_half.write_all(&bytes).await{
													Ok(_) => {
														if is_start_dt{
															if let Err(e)=self.connection_status_watch_tx.send(ConnectionStatus::WaitingForSTARTDTCON){
																let _=self.send_result_broadcast_tx.send((send_id,Err(format!("Channel error. {}",e))));
																break;
															}
														}else if is_stop_dt{
															if let Err(e)=self.connection_status_watch_tx.send(ConnectionStatus::WaitingForSTOPDTCON){
																let _=self.send_result_broadcast_tx.send((send_id,Err(format!("Channel error. {}",e))));
																break;
															}
														}
														if let Err(e)=self.send_result_broadcast_tx.send((send_id,Ok(()))){
															let _=self.send_result_broadcast_tx.send((send_id,Err(format!("Channel error. {}",e))));
															break;
														}
													}
													Err(e) => {
														let _=self.send_result_broadcast_tx.send((send_id,Err(format!("TCP write error. {}",e))));
														break;
													}
												}
											}
											Err(e) => {
												let _=self.send_result_broadcast_tx.send((send_id,Err(format!("Serilize error. {}",e))));
												break;
											}
										}
									}else{
										let _=self.send_result_broadcast_tx.send((send_id,Err("Connection closed.".to_string())));
										break;
									}
								}
								Err(e) => {
									let _=self.send_result_broadcast_tx.send((send_id,Err(format!("Channel error. {}",e))));
									break;
								}
							}
						}
						None => {
							break;
						}
					}
				}
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
		}
		self.stop_watch_tx.send_replace(true);
	}
	async fn timer_thread(mut self) {
		loop {
			match self.check_timeouts() {
				Ok(frames) => {
					for frame in frames {
						if let Err(e) = self.send(frame).await {
							self.callbacks
								.on_error(snafu::FromString::with_source(
									e.into(),
									"Failed sending periodic telegram".to_string(),
								))
								.await;
							break;
						}
					}
				}
				Err(e) => {
					self.callbacks
						.on_error(snafu::FromString::with_source(
							e.into(),
							"Failed handling timer event".to_string(),
						))
						.await;
					break;
				}
			}
			tokio::select! {
				_=tokio::time::sleep(tokio::time::Duration::from_millis(500))=>{}
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
		}
		self.stop_watch_tx.send_replace(true);
	}
}
