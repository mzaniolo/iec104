//! Example client for IEC 60870-5-104

use std::time::Duration;

use async_trait::async_trait;
use iec104::{
	asdu::Asdu,
	client::{Client, OnNewObjects},
	config::ClientConfig,
	cot::Cot,
	types::{
		CdcNa1, GenericObject, InformationObject,
		commands::{Dco, Qu},
		information_elements::{Dpi, SelectExecute},
	},
	types_id::TypeId,
};
use snafu::{ResultExt as _, Whatever};
use tokio::{
	signal::unix::{SignalKind, signal},
	time::Instant,
};
use tracing_subscriber::{
	Layer as _, filter::EnvFilter, layer::SubscriberExt as _, util::SubscriberInitExt as _,
};

#[tokio::main]
async fn main() -> Result<(), Whatever> {
	let filter = EnvFilter::from("debug");
	let layer = tracing_subscriber::fmt::layer().with_filter(filter);
	tracing_subscriber::registry().with(layer).init();
	let mut client = Client::new(ClientConfig::default(), MyCallback)
		.await
		.whatever_context("Failed to create client")?;
	client.start_receiving().await.whatever_context("Failed to start receiving")?;

	let mut s1 = signal(SignalKind::interrupt()).whatever_context("Failed to create signal")?;
	let mut s2 = signal(SignalKind::terminate()).whatever_context("Failed to create signal")?;

	let period = tokio::time::sleep(Duration::from_secs(1));
	tokio::pin!(period);

	let asdu = Asdu {
		type_id: TypeId::C_DC_NA_1,
		cot: Cot::Request,
		originator_address: 1,
		address_field: 47,
		sequence: false,
		test: false,
		positive: false,
		information_objects: InformationObject::CdcNa1(vec![GenericObject {
			address: 13,
			object: CdcNa1 {
				dco: Dco { se: SelectExecute::Select, qu: Qu::Persistent, dcs: Dpi::On },
			},
		}]),
	};

	loop {
		tokio::select! {
			_ = s1.recv() => {tracing::info!("SIGINT"); break;},
			_ = s2.recv() => {tracing::info!("SIGTERM"); break;},
			_ = &mut period => {
				tracing::info!("Period");
				client.send_asdu(asdu.clone()).await.whatever_context("Error sending test frame")?;
				 period.as_mut().reset(Instant::now() + Duration::from_secs(1));
			},
		}
	}

	tracing::info!("Disconnecting");

	Ok(())
}

/// Callback for the client
struct MyCallback;

#[async_trait]
impl OnNewObjects for MyCallback {
	async fn on_new_objects(&self, _objects: InformationObject) {
		// tracing::info!("Received objects: {objects:?}");
	}
}
