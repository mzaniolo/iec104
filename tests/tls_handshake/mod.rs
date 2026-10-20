//! Regression tests for how the server handles the TLS handshake.
//!
//! The handshake used to be awaited directly on the accept loop, so a peer
//! that connected and then stalled blocked every other client (and every
//! pending send command), while a peer that sent garbage made the loop
//! return an error and stop the server for good.

use std::{net::SocketAddr, path::PathBuf, time::Duration};

use async_trait::async_trait;
use iec104::{
	asdu::Asdu,
	client::Client,
	config::{ClientConfig, ProtocolConfig, ServerConfig, TlsClientConfig, TlsServerConfig},
	cot::Cot,
	server::{ConnectionId, Server, ServerCallback},
	types::{GenericObject, InformationObjects},
	types_id::TypeId,
};
use rcgen::{CertificateParams, KeyPair, PKCS_RSA_SHA256};
use tokio::{
	io::{AsyncReadExt as _, AsyncWriteExt as _},
	net::TcpStream,
	sync::mpsc,
	time::timeout,
};

use super::{AsduCollector, WAIT, m_sp_off, recv_until};

/// An IEC 104 StartDT-ACT frame, which is not a valid TLS ClientHello.
const START_DT_ACT_BYTES: [u8; 6] = [0x68, 0x04, 0x07, 0x00, 0x00, 0x00];

/// Paths to a generated self-signed certificate and its key.
struct TlsPaths {
	certificate: PathBuf,
	key: PathBuf,
}

/// Writes a fresh self-signed certificate for `127.0.0.1` to a temporary
/// directory unique to this test.
fn generate_server_cert(test_name: &str) -> TlsPaths {
	// RSA PKCS#8, not the ECDSA key from `generate_simple_self_signed`.
	// `native_tls` on macOS imports the PEM through Security.framework, which
	// rejects P-256 PKCS#8 ("Unknown format in import").
	let signing_key = KeyPair::generate_for(&PKCS_RSA_SHA256).expect("generate key");
	let cert = CertificateParams::new(vec!["127.0.0.1".to_owned()])
		.expect("certificate parameters")
		.self_signed(&signing_key)
		.expect("self-sign certificate");

	let dir = std::env::temp_dir().join(format!("iec104-tls-{}-{test_name}", std::process::id()));
	std::fs::create_dir_all(&dir).expect("create certificate directory");

	let certificate = dir.join("cert.pem");
	let key = dir.join("key.pem");
	std::fs::write(&certificate, cert.pem()).expect("write certificate");
	std::fs::write(&key, signing_key.serialize_pem()).expect("write key");

	TlsPaths { certificate, key }
}

/// Reports connection lifecycle transitions to the test.
struct ConnectionEvents {
	connected: mpsc::UnboundedSender<SocketAddr>,
	started: mpsc::UnboundedSender<ConnectionId>,
	stopped: mpsc::UnboundedSender<ConnectionId>,
}

#[async_trait]
impl ServerCallback for ConnectionEvents {
	async fn on_new_objects(&self, _asdu: Asdu, _id: ConnectionId, _addr: SocketAddr) {}
	async fn on_new_connection(&self, address: SocketAddr) {
		let _ = self.connected.send(address);
	}
	async fn on_connection_started(&self, id: ConnectionId, _addr: SocketAddr) {
		let _ = self.started.send(id);
	}
	async fn on_connection_stopped(&self, id: ConnectionId, _addr: SocketAddr) {
		let _ = self.stopped.send(id);
	}
}

/// Starts a TLS server on an ephemeral port with the given handshake timeout.
async fn start_tls_server(
	test_name: &str,
	handshake_timeout: Duration,
) -> (
	Server,
	SocketAddr,
	mpsc::UnboundedReceiver<SocketAddr>,
	mpsc::UnboundedReceiver<ConnectionId>,
	mpsc::UnboundedReceiver<ConnectionId>,
) {
	let tls = generate_server_cert(test_name);
	let cfg = ServerConfig {
		address: "127.0.0.1".to_owned(),
		port: 0,
		protocol: ProtocolConfig {
			t1: Duration::from_secs(8),
			t3: Duration::from_secs(8),
			..ProtocolConfig::default()
		},
		tls: Some(TlsServerConfig { server_certificate: tls.certificate, server_key: tls.key }),
		tls_handshake_timeout: handshake_timeout,
	};

	let (connected_tx, connected_rx) = mpsc::unbounded_channel();
	let (started_tx, started_rx) = mpsc::unbounded_channel();
	let (stopped_tx, stopped_rx) = mpsc::unbounded_channel();
	let (server, addr) = Server::start_with_listen_addr(
		cfg,
		ConnectionEvents { connected: connected_tx, started: started_tx, stopped: stopped_tx },
	)
	.await
	.expect("start server");
	(server, addr, connected_rx, started_rx, stopped_rx)
}

/// A TLS client config for `addr`; verification is off because the test
/// certificate is self-signed and the subject is not what is under test.
fn tls_client_cfg(addr: SocketAddr) -> ClientConfig {
	ClientConfig {
		address: addr.ip().to_string(),
		port: addr.port(),
		protocol: ProtocolConfig {
			t1: Duration::from_secs(8),
			t3: Duration::from_secs(8),
			..ProtocolConfig::default()
		},
		tls: Some(TlsClientConfig {
			client_key: None,
			client_certificate: None,
			server_certificate: None,
			danger_disable_tls_verify: true,
		}),
	}
}

/// Connects a TLS client, brings it to the started state and returns its
/// connection id plus the channel its ASDUs arrive on.
async fn connect_started_client(
	addr: SocketAddr,
	connected_rx: &mut mpsc::UnboundedReceiver<SocketAddr>,
	started_rx: &mut mpsc::UnboundedReceiver<ConnectionId>,
) -> (Client<AsduCollector>, ConnectionId, mpsc::UnboundedReceiver<Asdu>) {
	let (asdu_tx, asdu_rx) = mpsc::unbounded_channel();
	let mut client = Client::new(tls_client_cfg(addr), AsduCollector { tx: asdu_tx });
	timeout(WAIT, client.connect())
		.await
		.expect("client.connect timed out")
		.expect("client.connect");
	client.start_receiving().await.expect("client.start_receiving");

	// The transport callback fires once the handshake succeeds, before StartDT.
	timeout(WAIT, connected_rx.recv())
		.await
		.expect("timed out waiting for on_new_connection")
		.expect("connection event channel closed");
	let id = timeout(WAIT, started_rx.recv())
		.await
		.expect("timed out waiting for the connection to start")
		.expect("connection event channel closed");
	(client, id, asdu_rx)
}

/// A spontaneous single-point ASDU for round-trip checks.
fn spontaneous_sp(ca: u16, ioa: u32) -> Asdu {
	Asdu {
		type_id: TypeId::M_SP_NA_1,
		cot: Cot::SpontaneousData,
		originator_address: 0,
		address_field: ca,
		sequence: false,
		test: false,
		negative: false,
		information_objects: InformationObjects::MSpNa1(vec![GenericObject {
			address: ioa,
			object: m_sp_off(),
		}]),
	}
}

#[tokio::test]
async fn stalled_tls_handshake_does_not_block_other_clients() {
	// A generous handshake timeout: the point is that the stalled peer is
	// still mid-handshake while the rest of the test runs.
	let (server, addr, mut connected_rx, mut started_rx, _stopped_rx) =
		start_tls_server("stall-blocks", Duration::from_secs(30)).await;

	// Connect without ever sending a ClientHello.
	let _stalled = TcpStream::connect(addr).await.expect("tcp connect");

	// A well-behaved client must still get all the way to the started state.
	let (_client, id, mut asdu_rx) =
		connect_started_client(addr, &mut connected_rx, &mut started_rx).await;

	// And the server loop must still be servicing send commands, which it
	// could not do while it awaited a handshake inline.
	timeout(WAIT, server.send_asdu(id, spontaneous_sp(11, 101)))
		.await
		.expect("send_asdu timed out")
		.expect("send_asdu");
	let asdu = recv_until(&mut asdu_rx, WAIT, |asdu| asdu.type_id == TypeId::M_SP_NA_1).await;
	assert_eq!(asdu.address_field, 11, "the ASDU must reach the client unchanged");
}

#[tokio::test]
async fn stalled_tls_handshake_is_reaped_after_timeout() {
	let (_server, addr, mut connected_rx, _started_rx, mut stopped_rx) =
		start_tls_server("stall-reaped", Duration::from_millis(200)).await;

	let mut stalled = TcpStream::connect(addr).await.expect("tcp connect");

	// The server gives up on the handshake and drops the connection.
	let read = timeout(Duration::from_secs(2), stalled.read(&mut [0_u8; 1]))
		.await
		.expect("server did not close the stalled connection")
		.expect("read");
	assert_eq!(read, 0, "a connection that never handshakes must be closed");

	// And it is reaped from the server's connection list, without ever being
	// reported as a new connection: the handshake never completed.
	timeout(WAIT, stopped_rx.recv())
		.await
		.expect("timed out waiting for on_connection_stopped")
		.expect("connection event channel closed");
	assert!(
		connected_rx.try_recv().is_err(),
		"a failed handshake must not call on_new_connection"
	);
}

#[tokio::test]
async fn garbage_on_tls_port_does_not_kill_the_server() {
	let (_server, addr, mut connected_rx, mut started_rx, _stopped_rx) =
		start_tls_server("garbage", Duration::from_secs(30)).await;

	// Plain IEC 104 bytes on a TLS port fail the handshake; this used to
	// propagate out of the accept loop and stop the server permanently.
	let mut garbage = TcpStream::connect(addr).await.expect("tcp connect");
	garbage.write_all(&START_DT_ACT_BYTES).await.expect("write garbage");
	let _ = timeout(Duration::from_secs(2), garbage.read(&mut [0_u8; 64])).await;
	drop(garbage);

	// The server must still accept and serve new clients.
	let (_client, _id, _asdu_rx) =
		connect_started_client(addr, &mut connected_rx, &mut started_rx).await;
}
