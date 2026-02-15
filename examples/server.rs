//! Example server for IEC 60870-5-104

use std::{net::SocketAddr, time::Duration};

use iec104::{
	asdu::Asdu,
	config::ServerConfig,
	cot::Cot,
	error::Error,
	server::{ConnectionId, Server, ServerCallback},
	types::{
		GenericObject, InformationObjects, MSpNa1,
		information_elements::{Siq, Spi},
	},
	types_id::TypeId,
};
use snafu::{ResultExt as _, Whatever};
use tokio::{
	signal::unix::{SignalKind, signal},
	time::Instant,
};
use tracing_error::ErrorLayer;
use tracing_subscriber::{EnvFilter, Layer, layer::SubscriberExt, util::SubscriberInitExt};

#[snafu::report]
#[tokio::main]
async fn main() -> Result<(), Whatever> {
	let filter = EnvFilter::from("info");
	let layer = tracing_subscriber::fmt::layer().with_filter(filter);
	tracing_subscriber::registry()
		.with(layer)
		//needed to get the tracing_error working
		.with(ErrorLayer::default().with_filter(EnvFilter::from("debug")))
		.init();

	let server = Server::start(ServerConfig::default(), MyCallback {})
		.await
		.whatever_context("Failed to start server")?;

	let mut s1 = signal(SignalKind::interrupt()).whatever_context("Failed to create signal")?;
	let mut s2 = signal(SignalKind::terminate()).whatever_context("Failed to create signal")?;
	let period = tokio::time::sleep(Duration::from_secs(1));
	tokio::pin!(period);

	loop {
		tokio::select! {
				_ = s1.recv() => {tracing::info!("SIGINT"); break;},
				_ = s2.recv() => {tracing::info!("SIGTERM"); break;},
				_ = &mut period => {
					tracing::info!("Period");
					let _ = server.broadcast_asdu(make_asdu()).await.inspect_err(|e| tracing::error!("Error broadcasting ASDU: {e:?}"));
					period.as_mut().reset(Instant::now() + Duration::from_secs(1));
			}
		}
	}

	Ok(())
}

/// Make an ASDU with two single-point information objects
fn make_asdu() -> Asdu {
	Asdu {
		type_id: TypeId::M_SP_NA_1,
		cot: Cot::SpontaneousData,
		originator_address: 1,
		address_field: 47,
		sequence: false,
		test: false,
		positive: false,
		information_objects: InformationObjects::MSpNa1(vec![
			GenericObject {
				address: 1,
				object: MSpNa1 {
					siq: Siq { iv: false, nt: false, sb: false, bl: false, spi: Spi::On },
				},
			},
			GenericObject {
				address: 3,
				object: MSpNa1 {
					siq: Siq { iv: false, nt: false, sb: false, bl: false, spi: Spi::Off },
				},
			},
		]),
	}
}

/// Sample callback for the server
struct MyCallback {}

#[async_trait::async_trait]
impl ServerCallback for MyCallback {
	// Required callbacks
	async fn on_new_objects(&self, asdu: Asdu, id: ConnectionId, address: SocketAddr) {
		tracing::trace!("Connection {id:?} from {address} received objects: {asdu:?}");
	}
	// Optional callbacks
	async fn on_new_connection(&self, address: SocketAddr) {
		tracing::debug!("New connection from {address}");
	}
	async fn on_connection_started(&self, id: ConnectionId, address: SocketAddr) {
		tracing::debug!("Connection {id:?} from {address} started");
	}
	async fn on_connection_stopped(&self, id: ConnectionId, address: SocketAddr) {
		tracing::debug!("Connection {id:?} from {address} stopped");
	}
	async fn on_error(&self, error: &Error) {
		tracing::debug!("Error: {error}");
	}
}
