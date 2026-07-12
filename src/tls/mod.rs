#[cfg(all(feature = "native_tls", feature = "rustls"))]
compile_error!("Enable at most one of `native_tls` or `rustls`.");

#[cfg(feature = "native_tls")]
mod native;
#[cfg(feature = "rustls")]
mod rustls_impl;

#[cfg(test)]
#[cfg(any(feature = "native_tls", feature = "rustls"))]
mod mock;

#[cfg(any(feature = "native_tls", feature = "rustls"))]
use std::{
	fmt::Debug,
	path::{Path, PathBuf},
	pin::Pin,
	task::{Context, Poll},
};

#[cfg(any(feature = "native_tls", feature = "rustls"))]
use snafu::whatever;
#[cfg(any(feature = "native_tls", feature = "rustls"))]
use tokio::{
	io::{AsyncRead, AsyncWrite, ReadBuf},
	net::TcpStream,
};

#[cfg(any(feature = "native_tls", feature = "rustls"))]
use crate::{
	config::{TlsClientConfig, TlsServerConfig},
	error::Error,
};

#[cfg(any(feature = "native_tls", feature = "rustls"))]
pub trait TlsClientConnector: Send {
	type Stream: AsyncRead + AsyncWrite + Unpin + Send + Debug;

	async fn connect(self, host: &str, stream: TcpStream) -> Result<Self::Stream, Error>;
}

#[cfg(any(feature = "native_tls", feature = "rustls"))]
pub trait TlsServerAcceptor: Send + Clone {
	type Stream: AsyncRead + AsyncWrite + Unpin + Send + Debug;

	async fn accept(self, stream: TcpStream) -> Result<Self::Stream, Error>;
}

#[cfg(any(feature = "native_tls", feature = "rustls"))]
pub(crate) fn client_auth_paths<'a>(
	key: &'a Option<PathBuf>,
	cert: &'a Option<PathBuf>,
) -> Result<Option<(&'a Path, &'a Path)>, Error> {
	match (key, cert) {
		(Some(client_key), Some(client_cert)) => {
			Ok(Some((client_cert.as_path(), client_key.as_path())))
		}
		(None, None) => Ok(None),
		_ => whatever!("Both client key *and* certificate must be specified"),
	}
}

#[cfg(any(feature = "native_tls", feature = "rustls"))]
#[derive(Debug)]
pub enum TlsStream {
	#[cfg(feature = "native_tls")]
	Native(native::NativeTlsStream),
	#[cfg(feature = "rustls")]
	Rustls(rustls_impl::RustlsTlsStream),
}

#[cfg(any(feature = "native_tls", feature = "rustls"))]
impl AsyncRead for TlsStream {
	fn poll_read(
		self: Pin<&mut Self>,
		cx: &mut Context<'_>,
		buf: &mut ReadBuf<'_>,
	) -> Poll<std::io::Result<()>> {
		match self.get_mut() {
			#[cfg(feature = "native_tls")]
			Self::Native(stream) => Pin::new(stream).poll_read(cx, buf),
			#[cfg(feature = "rustls")]
			Self::Rustls(stream) => Pin::new(stream).poll_read(cx, buf),
		}
	}
}

#[cfg(any(feature = "native_tls", feature = "rustls"))]
impl AsyncWrite for TlsStream {
	fn poll_write(
		self: Pin<&mut Self>,
		cx: &mut Context<'_>,
		buf: &[u8],
	) -> Poll<Result<usize, std::io::Error>> {
		match self.get_mut() {
			#[cfg(feature = "native_tls")]
			Self::Native(stream) => Pin::new(stream).poll_write(cx, buf),
			#[cfg(feature = "rustls")]
			Self::Rustls(stream) => Pin::new(stream).poll_write(cx, buf),
		}
	}

	fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), std::io::Error>> {
		match self.get_mut() {
			#[cfg(feature = "native_tls")]
			Self::Native(stream) => Pin::new(stream).poll_flush(cx),
			#[cfg(feature = "rustls")]
			Self::Rustls(stream) => Pin::new(stream).poll_flush(cx),
		}
	}

	fn poll_shutdown(
		self: Pin<&mut Self>,
		cx: &mut Context<'_>,
	) -> Poll<Result<(), std::io::Error>> {
		match self.get_mut() {
			#[cfg(feature = "native_tls")]
			Self::Native(stream) => Pin::new(stream).poll_shutdown(cx),
			#[cfg(feature = "rustls")]
			Self::Rustls(stream) => Pin::new(stream).poll_shutdown(cx),
		}
	}
}

#[cfg(any(feature = "native_tls", feature = "rustls"))]
pub enum TlsConnector {
	#[cfg(feature = "native_tls")]
	Native(native::NativeClientConnector),
	#[cfg(feature = "rustls")]
	Rustls(rustls_impl::RustlsClientConnector),
}

#[cfg(any(feature = "native_tls", feature = "rustls"))]
impl TlsClientConnector for TlsConnector {
	type Stream = TlsStream;

	async fn connect(self, host: &str, stream: TcpStream) -> Result<Self::Stream, Error> {
		match self {
			#[cfg(feature = "native_tls")]
			Self::Native(connector) => {
				let stream = connector.connect(host, stream).await?;
				Ok(TlsStream::Native(stream))
			}
			#[cfg(feature = "rustls")]
			Self::Rustls(connector) => {
				let stream = connector.connect(host, stream).await?;
				Ok(TlsStream::Rustls(stream))
			}
		}
	}
}

#[cfg(any(feature = "native_tls", feature = "rustls"))]
#[derive(Clone)]
pub enum TlsAcceptor {
	#[cfg(feature = "native_tls")]
	Native(native::NativeServerAcceptor),
	#[cfg(feature = "rustls")]
	Rustls(rustls_impl::RustlsServerAcceptor),
}

#[cfg(any(feature = "native_tls", feature = "rustls"))]
impl TlsServerAcceptor for TlsAcceptor {
	type Stream = TlsStream;

	async fn accept(self, stream: TcpStream) -> Result<Self::Stream, Error> {
		match self {
			#[cfg(feature = "native_tls")]
			Self::Native(acceptor) => {
				let stream = acceptor.accept(stream).await?;
				Ok(TlsStream::Native(stream))
			}
			#[cfg(feature = "rustls")]
			Self::Rustls(acceptor) => {
				let stream = acceptor.accept(stream).await?;
				Ok(TlsStream::Rustls(stream))
			}
		}
	}
}

#[cfg(any(feature = "native_tls", feature = "rustls"))]
pub fn build_client_connector(tls: &TlsClientConfig) -> Result<TlsConnector, Error> {
	#[cfg(feature = "native_tls")]
	let connector = TlsConnector::Native(native::build_client_connector(tls)?);
	#[cfg(feature = "rustls")]
	let connector = TlsConnector::Rustls(rustls_impl::build_client_connector(tls)?);
	Ok(connector)
}

#[cfg(any(feature = "native_tls", feature = "rustls"))]
pub fn build_server_acceptor(tls: &TlsServerConfig) -> Result<TlsAcceptor, Error> {
	#[cfg(feature = "native_tls")]
	let acceptor = TlsAcceptor::Native(native::build_server_acceptor(tls)?);
	#[cfg(feature = "rustls")]
	let acceptor = TlsAcceptor::Rustls(rustls_impl::build_server_acceptor(tls)?);
	Ok(acceptor)
}

#[cfg(test)]
#[cfg(any(feature = "native_tls", feature = "rustls"))]
mod tests {
	use std::path::PathBuf;

	use tokio::{
		io::{AsyncReadExt as _, AsyncWriteExt as _},
		net::TcpStream,
	};

	use super::{TlsClientConnector, TlsServerAcceptor, client_auth_paths, mock};

	#[test]
	fn client_auth_paths_accepts_both_or_neither() {
		let key = PathBuf::from("/tmp/key.pem");
		let cert = PathBuf::from("/tmp/cert.pem");

		assert!(client_auth_paths(&None, &None).unwrap().is_none());
		assert_eq!(
			client_auth_paths(&Some(key.clone()), &Some(cert.clone()))
				.unwrap()
				.map(|(c, k)| (c.to_path_buf(), k.to_path_buf())),
			Some((cert, key))
		);
	}

	#[test]
	fn client_auth_paths_rejects_mismatched_pair() {
		let key = PathBuf::from("/tmp/key.pem");
		assert!(client_auth_paths(&Some(key), &None).is_err());
		assert!(client_auth_paths(&None, &Some(PathBuf::from("/tmp/cert.pem"))).is_err());
	}

	#[tokio::test]
	async fn mock_connector_and_acceptor_passthrough_tcp() {
		let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
		let addr = listener.local_addr().unwrap();

		let server = tokio::spawn(async move {
			let (stream, _) = listener.accept().await.unwrap();
			let mut stream = mock::MockServerAcceptor.accept(stream).await.unwrap().0;
			let mut buf = [0_u8; 4];
			stream.read_exact(&mut buf).await.unwrap();
			buf
		});

		let client_stream = TcpStream::connect(addr).await.unwrap();
		let mut client_stream =
			mock::MockClientConnector.connect("127.0.0.1", client_stream).await.unwrap().0;
		client_stream.write_all(b"ping").await.unwrap();

		assert_eq!(server.await.unwrap(), *b"ping");
	}
}
