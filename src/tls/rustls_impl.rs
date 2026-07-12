use std::{
	fs::File,
	io::BufReader,
	pin::Pin,
	sync::Arc,
	task::{Context, Poll},
};

use rustls::{
	ClientConfig, RootCertStore, ServerConfig,
	client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier},
	crypto::ring::default_provider,
	pki_types::{CertificateDer, PrivateKeyDer, ServerName, UnixTime},
};
use rustls_pemfile::{certs, private_key};
use snafu::{ResultExt as _, whatever};
use tokio::{
	io::{AsyncRead, AsyncWrite, ReadBuf},
	net::TcpStream,
};
use tokio_rustls::{TlsAcceptor, TlsConnector};

use super::{TlsClientConnector, TlsServerAcceptor, client_auth_paths};
use crate::{
	config::{TlsClientConfig, TlsServerConfig},
	error::Error,
};

#[derive(Debug)]
pub(crate) enum RustlsTlsStream {
	Client(tokio_rustls::client::TlsStream<TcpStream>),
	Server(tokio_rustls::server::TlsStream<TcpStream>),
}

impl AsyncRead for RustlsTlsStream {
	fn poll_read(
		self: Pin<&mut Self>,
		cx: &mut Context<'_>,
		buf: &mut ReadBuf<'_>,
	) -> Poll<std::io::Result<()>> {
		match self.get_mut() {
			Self::Client(stream) => Pin::new(stream).poll_read(cx, buf),
			Self::Server(stream) => Pin::new(stream).poll_read(cx, buf),
		}
	}
}

impl AsyncWrite for RustlsTlsStream {
	fn poll_write(
		self: Pin<&mut Self>,
		cx: &mut Context<'_>,
		buf: &[u8],
	) -> Poll<Result<usize, std::io::Error>> {
		match self.get_mut() {
			Self::Client(stream) => Pin::new(stream).poll_write(cx, buf),
			Self::Server(stream) => Pin::new(stream).poll_write(cx, buf),
		}
	}

	fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), std::io::Error>> {
		match self.get_mut() {
			Self::Client(stream) => Pin::new(stream).poll_flush(cx),
			Self::Server(stream) => Pin::new(stream).poll_flush(cx),
		}
	}

	fn poll_shutdown(
		self: Pin<&mut Self>,
		cx: &mut Context<'_>,
	) -> Poll<Result<(), std::io::Error>> {
		match self.get_mut() {
			Self::Client(stream) => Pin::new(stream).poll_shutdown(cx),
			Self::Server(stream) => Pin::new(stream).poll_shutdown(cx),
		}
	}
}

pub(crate) struct RustlsClientConnector(TlsConnector);

impl TlsClientConnector for RustlsClientConnector {
	type Stream = RustlsTlsStream;

	async fn connect(self, host: &str, stream: TcpStream) -> Result<Self::Stream, Error> {
		let server_name =
			ServerName::try_from(host).whatever_context("Invalid server name for TLS")?.to_owned();
		let stream =
			self.0.connect(server_name, stream).await.whatever_context("Error connecting")?;
		Ok(RustlsTlsStream::Client(stream))
	}
}

#[derive(Clone)]
pub(crate) struct RustlsServerAcceptor(TlsAcceptor);

impl TlsServerAcceptor for RustlsServerAcceptor {
	type Stream = RustlsTlsStream;

	async fn accept(self, stream: TcpStream) -> Result<Self::Stream, Error> {
		let stream =
			self.0.accept(stream).await.whatever_context("Error doing the TLS handshake")?;
		Ok(RustlsTlsStream::Server(stream))
	}
}

#[derive(Debug)]
struct NoCertificateVerification;

impl ServerCertVerifier for NoCertificateVerification {
	fn verify_server_cert(
		&self,
		_end_entity: &CertificateDer<'_>,
		_intermediates: &[CertificateDer<'_>],
		_server_name: &ServerName<'_>,
		_ocsp_response: &[u8],
		_now: UnixTime,
	) -> Result<ServerCertVerified, rustls::Error> {
		Ok(ServerCertVerified::assertion())
	}

	fn verify_tls12_signature(
		&self,
		_message: &[u8],
		_cert: &CertificateDer<'_>,
		_dss: &rustls::DigitallySignedStruct,
	) -> Result<HandshakeSignatureValid, rustls::Error> {
		Ok(HandshakeSignatureValid::assertion())
	}

	fn verify_tls13_signature(
		&self,
		_message: &[u8],
		_cert: &CertificateDer<'_>,
		_dss: &rustls::DigitallySignedStruct,
	) -> Result<HandshakeSignatureValid, rustls::Error> {
		Ok(HandshakeSignatureValid::assertion())
	}

	fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
		default_provider().signature_verification_algorithms.supported_schemes()
	}
}

fn load_certs(path: &std::path::Path) -> Result<Vec<CertificateDer<'static>>, Error> {
	let cert_data = std::fs::read(path).whatever_context("Failed to read certificate file")?;
	let mut reader = cert_data.as_slice();
	certs(&mut reader).collect::<Result<Vec<_>, _>>().whatever_context("Invalid certificate PEM")
}

fn load_private_key(path: &std::path::Path) -> Result<PrivateKeyDer<'static>, Error> {
	let key_file = File::open(path).whatever_context("Failed to read key file")?;
	match private_key(&mut BufReader::new(key_file)).whatever_context("Invalid key PEM")? {
		Some(key) => Ok(key),
		None => whatever!("No private key found in PEM file"),
	}
}

fn load_root_cert_store(tls: &TlsClientConfig) -> Result<RootCertStore, Error> {
	let mut root_cert_store = RootCertStore::empty();
	if let Some(ref path) = tls.server_certificate {
		for cert in load_certs(path)? {
			root_cert_store.add(cert).whatever_context("Invalid server certificate")?;
		}
	} else {
		let native_certs = rustls_native_certs::load_native_certs();
		if let Some(err) = native_certs.errors.first() {
			tracing::warn!("Failed to load some native root certificates: {err}");
		}
		for cert in native_certs.certs {
			let _ = root_cert_store.add(cert);
		}
	}
	Ok(root_cert_store)
}

pub(crate) fn build_client_connector(
	tls: &TlsClientConfig,
) -> Result<RustlsClientConnector, Error> {
	let _ = default_provider().install_default();

	let client_certs_and_key = match client_auth_paths(&tls.client_key, &tls.client_certificate)? {
		Some((client_cert, client_key)) => {
			Some((load_certs(client_cert)?, load_private_key(client_key)?))
		}
		None => None,
	};

	let config = if tls.danger_disable_tls_verify {
		let builder = ClientConfig::builder()
			.dangerous()
			.with_custom_certificate_verifier(Arc::new(NoCertificateVerification));
		match client_certs_and_key {
			Some((certs, key)) => builder
				.with_client_auth_cert(certs, key)
				.whatever_context("Error building TLS connector")?,
			None => builder.with_no_client_auth(),
		}
	} else {
		let root_cert_store = load_root_cert_store(tls)?;
		let builder = ClientConfig::builder().with_root_certificates(root_cert_store);
		match client_certs_and_key {
			Some((certs, key)) => builder
				.with_client_auth_cert(certs, key)
				.whatever_context("Error building TLS connector")?,
			None => builder.with_no_client_auth(),
		}
	};

	Ok(RustlsClientConnector(TlsConnector::from(Arc::new(config))))
}

pub(crate) fn build_server_acceptor(tls: &TlsServerConfig) -> Result<RustlsServerAcceptor, Error> {
	let _ = default_provider().install_default();

	let certs = load_certs(&tls.server_certificate)?;
	let key = load_private_key(&tls.server_key)?;
	let config = ServerConfig::builder()
		.with_no_client_auth()
		.with_single_cert(certs, key)
		.whatever_context("Error creating TLS acceptor")?;
	Ok(RustlsServerAcceptor(TlsAcceptor::from(Arc::new(config))))
}
