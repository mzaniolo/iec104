//! Integration tests for IEC 60870-5-104 protocol-behavior compliance.
//!
//! Each test exercises a specific spec requirement that was previously
//! violated; see the audit notes in the commit that introduces this file.
#![allow(clippy::expect_used)]

use std::{future::Future, net::SocketAddr, sync::Arc, time::Duration};

use async_trait::async_trait;
use iec104::{
	asdu::Asdu,
	client::{Client, ClientCallback},
	config::{ClientConfig, ProtocolConfig, ServerConfig},
	cot::Cot,
	rtu_server::{
		PointAddress, PointValue, RejectAllCommands, RtuInitialPoint, RtuSystemHandlers,
		command_handler_from_fn,
	},
	server::{ConnectionId, Server, ServerCallback},
	types::{
		MMeNa1,
		information_elements::{SelectExecute, Spi},
		quality_descriptors::Qds,
	},
};
use tokio::{
	io::{AsyncReadExt, AsyncWriteExt},
	net::TcpStream,
	sync::mpsc,
	time::timeout,
};

use super::{WAIT, drain_until_m_ei, m_sp_off};

// ---------------------------------------------------------------------------
// 2.1: client::send_command_* must emit Cot::Activation, not Cot::Request,
// and must use the originator_address from the protocol config.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn send_command_sp_uses_activation_cot_and_configured_originator() {
	let ca = 21_u16;
	let ioa = 7_u32;
	let initial =
		vec![RtuInitialPoint::new(PointAddress::new(ca, ioa), PointValue::MSpNa1(m_sp_off()))];

	// Capture the COT and originator_address of the first command ASDU
	// the RTU server's command handler sees.
	let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel::<(Cot, u8)>();
	let handler = command_handler_from_fn(move |ctx| {
		let _ = cmd_tx.send((ctx.asdu.cot, ctx.asdu.originator_address));
		// Echo + accept so the test framework completes cleanly.
		iec104::rtu_server::CommandHandling {
			negative: false,
			reply_type_id: ctx.asdu.type_id,
			reply_information_objects: ctx.asdu.information_objects.clone(),
			apply_updates: Vec::new(),
		}
	});

	let (rtu, listen) = iec104::rtu_server::start_rtu_server_ephemeral(
		ServerConfig::default(),
		initial,
		handler,
		RtuSystemHandlers::default(),
	)
	.await
	.expect("start_rtu_server_ephemeral");

	// Client with a non-default originator_address so we can verify it's
	// actually read from the config (the bug hard-coded 0).
	let protocol = ProtocolConfig {
		originator_address: 42,
		t1: Duration::from_secs(8),
		t3: Duration::from_secs(8),
		..ProtocolConfig::default()
	};
	let cfg =
		ClientConfig { address: listen.ip().to_string(), port: listen.port(), protocol, tls: None };

	let (asdu_tx, mut asdu_rx) = mpsc::unbounded_channel();
	let mut client = Client::new(cfg, super::AsduCollector { tx: asdu_tx });
	client.connect().await.expect("connect");
	client.start_receiving().await.expect("start_receiving");

	// Wait until the connection is in Started state (RTU sends M_EI on
	// transition); only then is send_command_sp allowed.
	drain_until_m_ei(&mut asdu_rx).await;

	client
		.send_command_sp(ca, ioa, Spi::On, None, Some(SelectExecute::Execute), None)
		.await
		.expect("send_command_sp");

	let (cot, oa) = timeout(WAIT, cmd_rx.recv()).await.expect("timeout").expect("channel closed");
	assert_eq!(cot, Cot::Activation, "commands must use Cot::Activation per IEC 101 Table 14");
	assert_eq!(oa, 42, "originator_address must come from ProtocolConfig, not hard-coded 0");

	drop(rtu);
}

// ---------------------------------------------------------------------------
// 2.2: server must answer TestFR-ACT in any state, including before
// StartDT-ACT has been received.
// ---------------------------------------------------------------------------

const START_DT_ACT_BYTES: [u8; 6] = [0x68, 0x04, 0x07, 0x00, 0x00, 0x00];
const START_DT_CON_BYTES: [u8; 6] = [0x68, 0x04, 0x0B, 0x00, 0x00, 0x00];
const TEST_FR_ACT_BYTES: [u8; 6] = [0x68, 0x04, 0x43, 0x00, 0x00, 0x00];
const TEST_FR_CON_BYTES: [u8; 6] = [0x68, 0x04, 0x83, 0x00, 0x00, 0x00];

struct NoopCallback;

#[async_trait]
impl ServerCallback for NoopCallback {
	async fn on_new_objects(&self, _asdu: Asdu, _id: ConnectionId, _addr: SocketAddr) {}
}

async fn start_bare_server() -> (Server, SocketAddr) {
	let cfg = ServerConfig {
		address: "127.0.0.1".to_owned(),
		port: 0,
		protocol: ProtocolConfig::default(),
		tls: None,
	};
	Server::start_with_listen_addr(cfg, NoopCallback).await.expect("start server")
}

async fn read_apdu(stream: &mut TcpStream) -> Vec<u8> {
	let mut header = [0_u8; 2];
	timeout(WAIT, stream.read_exact(&mut header))
		.await
		.expect("read header timeout")
		.expect("read header");
	assert_eq!(header[0], 0x68, "expected APDU start byte");
	let len = header[1] as usize;
	let mut rest = vec![0_u8; len];
	timeout(WAIT, stream.read_exact(&mut rest))
		.await
		.expect("read body timeout")
		.expect("read body");
	let mut full = Vec::with_capacity(2 + len);
	full.extend_from_slice(&header);
	full.extend_from_slice(&rest);
	full
}

#[tokio::test]
async fn server_answers_test_fr_act_before_start_dt() {
	let (_server, addr) = start_bare_server().await;
	let mut sock = TcpStream::connect(addr).await.expect("tcp connect");

	// Send TestFR-ACT before any StartDT — the server was previously
	// silent here and would have closed on T1 timeout.
	sock.write_all(&TEST_FR_ACT_BYTES).await.expect("write test_fr_act");
	let reply = read_apdu(&mut sock).await;
	assert_eq!(reply, TEST_FR_CON_BYTES, "server must reply with TestFR-CON in WaitingStartDT");

	// Then a normal StartDT-ACT must still work.
	sock.write_all(&START_DT_ACT_BYTES).await.expect("write start_dt_act");
	let reply = read_apdu(&mut sock).await;
	assert_eq!(reply, START_DT_CON_BYTES, "StartDT-ACT must still confirm after a TestFR exchange");
}

#[tokio::test]
async fn server_closes_on_invalid_frame_in_waiting_start_dt() {
	let (_server, addr) = start_bare_server().await;
	let mut sock = TcpStream::connect(addr).await.expect("tcp connect");

	// A StopDT-ACT before StartDT is invalid per §5.3 — server should
	// drop the connection rather than silently waiting for T1.
	let stop_dt_act = [0x68, 0x04, 0x13, 0x00, 0x00, 0x00];
	sock.write_all(&stop_dt_act).await.expect("write stop_dt_act");

	let mut buf = [0_u8; 1];
	let res = timeout(Duration::from_secs(2), sock.read(&mut buf)).await.expect("io timeout");
	match res {
		Ok(0) => {} // EOF — server closed
		Ok(n) => panic!("expected server to close, got {n} bytes"),
		Err(e) => panic!("unexpected io error: {e}"),
	}
}

// ---------------------------------------------------------------------------
// 2.3: client::send_start_dt must error when the server sends anything
// other than a U-frame with start_dt_confirmation.
// ---------------------------------------------------------------------------

/// Run a fake server on an ephemeral port. The provided closure receives the
/// accepted [`TcpStream`] and decides what to write back. Returns the bound
/// address.
async fn fake_server<F, Fut>(handler: F) -> SocketAddr
where
	F: FnOnce(TcpStream) -> Fut + Send + 'static,
	Fut: Future<Output = ()> + Send + 'static,
{
	let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("bind");
	let addr = listener.local_addr().expect("local_addr");
	tokio::spawn(async move {
		if let Ok((sock, _)) = listener.accept().await {
			handler(sock).await;
		}
	});
	addr
}

struct DiscardingCallback;
#[async_trait]
impl ClientCallback for DiscardingCallback {
	async fn on_new_objects(&self, _asdu: Asdu) {}
}

#[tokio::test]
async fn client_rejects_non_u_frame_as_start_dt_response() {
	// Fake server: read whatever the client sends, then reply with an
	// S-frame instead of a U-frame.
	let addr = fake_server(|mut sock| async move {
		let mut buf = [0_u8; 6];
		let _ = sock.read_exact(&mut buf).await;
		// S-frame: 0x68 0x04 0x01 0x00 RSN_LO RSN_HI
		let s_frame = [0x68, 0x04, 0x01, 0x00, 0x00, 0x00];
		let _ = sock.write_all(&s_frame).await;
		// Keep socket alive long enough for client to read it.
		tokio::time::sleep(Duration::from_millis(500)).await;
	})
	.await;

	let protocol = ProtocolConfig { t1: Duration::from_secs(2), ..ProtocolConfig::default() };
	let cfg =
		ClientConfig { address: addr.ip().to_string(), port: addr.port(), protocol, tls: None };

	let mut client = Client::new(cfg, DiscardingCallback);
	client.connect().await.expect("connect");
	// start_receiving sends StartDT-ACT and waits for the confirmation.
	// The fake server replies with an S-frame, which must be rejected.
	client.start_receiving().await.expect("queue start command");

	// Wait briefly for the connection handler to process the S-frame and
	// transition to Reconnecting.
	tokio::time::sleep(Duration::from_millis(300)).await;
	// We don't assert on the state directly (no public accessor); the
	// behavior we want is that send_command on this client now fails,
	// because the connection is not in `Started`.
	let res = client.send_command_sp(1, 1, Spi::On, None, Some(SelectExecute::Execute), None).await;
	assert!(res.is_err(), "client must not reach Started after a non-U StartDT response");
}

// ---------------------------------------------------------------------------
// 2.5: in-session StartDT-ACT is a protocol error (server should not
// re-confirm; client/peer must close the connection).
// ---------------------------------------------------------------------------

#[tokio::test]
async fn in_session_start_dt_act_closes_the_connection() {
	let ca = 31_u16;
	let initial = vec![RtuInitialPoint::new(
		PointAddress::new(ca, 1),
		PointValue::MMeNa1(MMeNa1 { nva: 0, qds: Qds::default() }),
	)];

	// Stand up an RTU server (no built-in client; we'll talk raw TCP).
	let (rtu, listen) = iec104::rtu_server::start_rtu_server_ephemeral(
		ServerConfig::default(),
		initial,
		Arc::new(RejectAllCommands),
		RtuSystemHandlers::default(),
	)
	.await
	.expect("start_rtu_server_ephemeral");

	let mut sock = TcpStream::connect(listen).await.expect("tcp connect");
	sock.write_all(&START_DT_ACT_BYTES).await.expect("write start_dt_act");
	let _ = read_apdu(&mut sock).await; // consume START_DT_CON

	// Send a second StartDT-ACT while already in data-transfer state —
	// invalid per §5.3, server must drop the connection.
	sock.write_all(&START_DT_ACT_BYTES).await.expect("write 2nd start_dt_act");

	// Drain until EOF (the server may emit M_EI and other ASDUs first).
	let mut buf = [0_u8; 256];
	let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
	let closed = loop {
		if tokio::time::Instant::now() >= deadline {
			break false;
		}
		match timeout(Duration::from_millis(500), sock.read(&mut buf)).await {
			Ok(Ok(0)) => break true,
			Ok(Ok(_)) => continue,
			Ok(Err(_)) => break true,
			Err(_) => continue,
		}
	};
	assert!(closed, "server must close connection on stray StartDT-ACT");

	drop(rtu);
}

// ---------------------------------------------------------------------------
// 4: protocol-config validation must reject spec-violating k/w combinations
// at Server::start and Client::connect (IEC 60870-5-104 §5.2: w ≤ ⌊2k/3⌋).
// ---------------------------------------------------------------------------

#[tokio::test]
async fn server_start_rejects_invalid_k_w_ratio() {
	let cfg = ServerConfig {
		address: "127.0.0.1".to_owned(),
		port: 0,
		// k=12 → max_w = 8; w=9 violates the spec.
		protocol: ProtocolConfig { k: 12, w: 9, ..ProtocolConfig::default() },
		tls: None,
	};
	let res = Server::start_with_listen_addr(cfg, NoopCallback).await;
	assert!(res.is_err(), "Server::start must refuse w > 2k/3");
}

#[tokio::test]
async fn client_connect_rejects_invalid_k_w_ratio() {
	let cfg = ClientConfig {
		address: "127.0.0.1".to_owned(),
		port: 1,
		protocol: ProtocolConfig { k: 12, w: 9, ..ProtocolConfig::default() },
		tls: None,
	};
	let mut client = Client::new(cfg, DiscardingCallback);
	let res = client.connect().await;
	assert!(res.is_err(), "Client::connect must refuse w > 2k/3");
}
