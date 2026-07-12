use snafu::ResultExt as _;
use tokio::net::TcpStream;
use tokio_native_tls::{
	TlsAcceptor, TlsConnector,
	native_tls::{Certificate, Identity},
};

use super::{TlsClientConnector, TlsServerAcceptor, client_auth_paths};
use crate::{
	config::{TlsClientConfig, TlsServerConfig},
	error::Error,
};

pub(crate) type NativeTlsStream = tokio_native_tls::TlsStream<TcpStream>;

pub(crate) struct NativeClientConnector(TlsConnector);

impl TlsClientConnector for NativeClientConnector {
	type Stream = NativeTlsStream;

	async fn connect(self, host: &str, stream: TcpStream) -> Result<Self::Stream, Error> {
		self.0.connect(host, stream).await.whatever_context("Error connecting")
	}
}

#[derive(Clone)]
pub(crate) struct NativeServerAcceptor(TlsAcceptor);

impl TlsServerAcceptor for NativeServerAcceptor {
	type Stream = NativeTlsStream;

	async fn accept(self, stream: TcpStream) -> Result<Self::Stream, Error> {
		self.0.accept(stream).await.whatever_context("Error doing the TLS handshake")
	}
}

pub(crate) fn build_client_connector(
	tls: &TlsClientConfig,
) -> Result<NativeClientConnector, Error> {
	let root_cert: Option<Certificate> = tls
		.server_certificate
		.as_ref()
		.map(std::fs::read)
		.transpose()
		.whatever_context("Failed to read server certificate")?
		.map(|cert_data| Certificate::from_pem(cert_data.as_slice()))
		.transpose()
		.whatever_context("Invalid server certificate")?;

	let identity: Option<Identity> =
		match client_auth_paths(&tls.client_key, &tls.client_certificate)? {
			Some((client_cert, client_key)) => Some(
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
			None => None,
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
	Ok(NativeClientConnector(TlsConnector::from(connector)))
}

pub(crate) fn build_server_acceptor(tls: &TlsServerConfig) -> Result<NativeServerAcceptor, Error> {
	let identity = Identity::from_pkcs8(
		std::fs::read(&tls.server_certificate)
			.whatever_context("Failed to read server certificate")?
			.as_slice(),
		std::fs::read(&tls.server_key).whatever_context("Failed to read server key")?.as_slice(),
	)
	.whatever_context("Error creating server TLS identity")?;
	let acceptor = tokio_native_tls::native_tls::TlsAcceptor::new(identity)
		.map(TlsAcceptor::from)
		.whatever_context("Error creating TLS acceptor")?;
	Ok(NativeServerAcceptor(acceptor))
}
