use std::pin::Pin;

use snafu::{ResultExt, whatever};
use tokio::{
	io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
	net::TcpStream,
};
use tokio_native_tls::{
	TlsConnector, TlsStream,
	native_tls::{Certificate, Identity},
};

use crate::{
	apdu::{APUD_MAX_LENGTH, Apdu, Frame, TELEGRAN_HEADER},
	config::{ClientConfig, TlsClientConfig},
	error::Error,
};

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

#[derive(Debug)]
pub struct Client {
	pub sent_counter: u16,
	pub received_counter: u16,
	pub last_acknowledged_counter: u16,
	connection: Connection,
}

impl Client {
	pub async fn new(config: ClientConfig) -> Result<Self, Error> {
		let stream = TcpStream::connect(format!("{}:{}", config.address, config.port))
			.await
			.whatever_context("Error connecting")?;
		let connection = if let Some(tls) = config.tls {
			let connector = Self::make_tls_connector(tls)?;
			Connection::Tls(
				connector
					.connect(&config.address, stream)
					.await
					.whatever_context("Error connecting")?,
			)
		} else {
			Connection::Tcp(stream)
		};

		Ok(Self { sent_counter: 0, received_counter: 0, last_acknowledged_counter: 0, connection })
	}

	// TODO: Remove this
	#[allow(clippy::result_large_err)]
	fn make_tls_connector(tls: TlsClientConfig) -> Result<TlsConnector, Error> {
		let root_cert: Option<Certificate> = tls
			.server_certificate
			.as_ref()
			.map(std::fs::read)
			.transpose()
			.whatever_context("Failed to read server certificate")?
			.map(|cert_data| Certificate::from_pem(cert_data.as_slice()))
			.transpose()
			.whatever_context("Invalid server certificate")?;

		let identity: Option<Identity> = match (tls.client_key, tls.client_certificate) {
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

	pub async fn send(&mut self, frame: Frame) -> Result<(), Error> {
		self.connection
			.write_all(
				&frame
					.to_apdu_bytes()
					.whatever_context("Error converting frame to APDU and encoding")?,
			)
			.await
			.whatever_context("Error sending data")?;
		Ok(())
	}

	pub async fn receive(&mut self) -> Result<Apdu, Error> {
		let mut buffer = [0; 255];
		self.connection
			.read_exact(&mut buffer[0..2])
			.await
			.whatever_context("Error receiving data")?;
		if buffer[0] != TELEGRAN_HEADER {
			whatever!("Invalid starter byte: {:02x}{:02x}", buffer[0], buffer[1]);
		}
		let length = buffer[1] as usize;
		if length > APUD_MAX_LENGTH as usize {
			whatever!("Invalid length: {}", length);
		}
		self.connection
			.read_exact(&mut buffer[2..length + 2])
			.await
			.whatever_context("Error receiving data")?;
		Apdu::from_bytes(&buffer[0..length + 2]).whatever_context("Error decoding APDU")
	}
}
