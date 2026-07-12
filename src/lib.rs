#![allow(missing_docs, clippy::missing_docs_in_private_items)]

use std::pin::Pin;

use lazy_static::lazy_static;
use tokio::{
	io::{AsyncRead, AsyncWrite},
	net::TcpStream,
};

use crate::apdu::{Frame, UFrame};

pub mod apdu;
pub mod asdu;
pub mod client;
pub mod config;
pub mod cot;
pub mod error;
mod receive_handler;
pub mod rtu_server;
pub mod server;
mod tls;
pub mod types;
pub mod types_id;

lazy_static! {
	static ref TEST_FR_CON_FRAME: Frame =
		Frame::U(UFrame { test_fr_confirmation: true, ..Default::default() });
	static ref START_DT_CON_FRAME: Frame =
		Frame::U(UFrame { start_dt_confirmation: true, ..Default::default() });
	static ref STOP_DT_CON_FRAME: Frame =
		Frame::U(UFrame { stop_dt_confirmation: true, ..Default::default() });
	static ref TEST_FR_ACT_FRAME: Frame =
		Frame::U(UFrame { test_fr_activation: true, ..Default::default() });
	static ref START_DT_ACT_FRAME: Frame =
		Frame::U(UFrame { start_dt_activation: true, ..Default::default() });
	static ref STOP_DT_ACT_FRAME: Frame =
		Frame::U(UFrame { stop_dt_activation: true, ..Default::default() });
}

#[derive(Debug)]
enum Connection {
	Tcp(TcpStream),
	#[cfg(any(feature = "native_tls", feature = "rustls"))]
	Tls(Box<tls::TlsStream>),
}

impl AsyncRead for Connection {
	fn poll_read(
		self: Pin<&mut Self>,
		cx: &mut std::task::Context<'_>,
		buf: &mut tokio::io::ReadBuf<'_>,
	) -> std::task::Poll<std::io::Result<()>> {
		match self.get_mut() {
			Connection::Tcp(stream) => Pin::new(stream).poll_read(cx, buf),
			#[cfg(any(feature = "native_tls", feature = "rustls"))]
			Connection::Tls(stream) => Pin::new(stream.as_mut()).poll_read(cx, buf),
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
			#[cfg(any(feature = "native_tls", feature = "rustls"))]
			Connection::Tls(stream) => Pin::new(stream.as_mut()).poll_write(cx, buf),
		}
	}

	fn poll_flush(
		self: Pin<&mut Self>,
		cx: &mut std::task::Context<'_>,
	) -> std::task::Poll<Result<(), std::io::Error>> {
		match self.get_mut() {
			Connection::Tcp(stream) => Pin::new(stream).poll_flush(cx),
			#[cfg(any(feature = "native_tls", feature = "rustls"))]
			Connection::Tls(stream) => Pin::new(stream.as_mut()).poll_flush(cx),
		}
	}

	fn poll_shutdown(
		self: Pin<&mut Self>,
		cx: &mut std::task::Context<'_>,
	) -> std::task::Poll<Result<(), std::io::Error>> {
		match self.get_mut() {
			Connection::Tcp(stream) => Pin::new(stream).poll_shutdown(cx),
			#[cfg(any(feature = "native_tls", feature = "rustls"))]
			Connection::Tls(stream) => Pin::new(stream.as_mut()).poll_shutdown(cx),
		}
	}
}
