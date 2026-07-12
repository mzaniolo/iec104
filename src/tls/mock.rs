use std::pin::Pin;

use tokio::{
	io::{AsyncRead, AsyncWrite, ReadBuf},
	net::TcpStream,
};

use super::{TlsClientConnector, TlsServerAcceptor};
use crate::error::Error;

#[derive(Debug)]
pub(crate) struct MockStream(pub TcpStream);

impl AsyncRead for MockStream {
	fn poll_read(
		self: Pin<&mut Self>,
		cx: &mut std::task::Context<'_>,
		buf: &mut ReadBuf<'_>,
	) -> std::task::Poll<std::io::Result<()>> {
		Pin::new(&mut self.get_mut().0).poll_read(cx, buf)
	}
}

impl AsyncWrite for MockStream {
	fn poll_write(
		self: Pin<&mut Self>,
		cx: &mut std::task::Context<'_>,
		buf: &[u8],
	) -> std::task::Poll<Result<usize, std::io::Error>> {
		Pin::new(&mut self.get_mut().0).poll_write(cx, buf)
	}

	fn poll_flush(
		self: Pin<&mut Self>,
		cx: &mut std::task::Context<'_>,
	) -> std::task::Poll<Result<(), std::io::Error>> {
		Pin::new(&mut self.get_mut().0).poll_flush(cx)
	}

	fn poll_shutdown(
		self: Pin<&mut Self>,
		cx: &mut std::task::Context<'_>,
	) -> std::task::Poll<Result<(), std::io::Error>> {
		Pin::new(&mut self.get_mut().0).poll_shutdown(cx)
	}
}

pub(crate) struct MockClientConnector;

impl TlsClientConnector for MockClientConnector {
	type Stream = MockStream;

	async fn connect(self, _host: &str, stream: TcpStream) -> Result<Self::Stream, Error> {
		Ok(MockStream(stream))
	}
}

#[derive(Clone)]
pub(crate) struct MockServerAcceptor;

impl TlsServerAcceptor for MockServerAcceptor {
	type Stream = MockStream;

	async fn accept(self, stream: TcpStream) -> Result<Self::Stream, Error> {
		Ok(MockStream(stream))
	}
}
