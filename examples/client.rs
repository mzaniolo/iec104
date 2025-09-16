//! Example client for IEC 60870-5-104

use std::time::Duration;

use async_trait::async_trait;
use iec104::{
	asdu::Asdu,
	client::{Client, OnNewObjects, errors::ClientError},
	config::ClientConfig,
	types::{
		commands::Rcs,
		information_elements::{Dpi, Spi},
	},
};
use snafu::{ResultExt as _, Whatever, whatever};
use tokio::{
	signal::unix::{SignalKind, signal},
	time::Instant,
};
use tracing_error::ErrorLayer;
use tracing_subscriber::{
	Layer as _, filter::EnvFilter, layer::SubscriberExt as _, util::SubscriberInitExt as _,
};

#[tokio::main]
async fn main() -> Result<(), Whatever> {
	let filter = EnvFilter::from("info");
	let layer = tracing_subscriber::fmt::layer().with_filter(filter);
	tracing_subscriber::registry()
		.with(layer)
		//needed to get the tracing_error working
		.with(ErrorLayer::default().with_filter(EnvFilter::from("debug")))
		.init();

	let mut client = Client::new(ClientConfig::default(), MyCallback);
	client.connect().await.whatever_context("Failed to connect")?;
	client.start_receiving().await.whatever_context("Failed to start receiving")?;

	let mut s1 = signal(SignalKind::interrupt()).whatever_context("Failed to create signal")?;
	let mut s2 = signal(SignalKind::terminate()).whatever_context("Failed to create signal")?;

	let period = tokio::time::sleep(Duration::from_secs(1));
	tokio::pin!(period);

	let stop = tokio::time::sleep(Duration::from_secs(5));
	tokio::pin!(stop);

	let restart = tokio::time::sleep(Duration::from_secs(15));
	tokio::pin!(restart);

	loop {
		tokio::select! {
			_ = s1.recv() => {tracing::info!("SIGINT"); break;},
			_ = s2.recv() => {tracing::info!("SIGTERM"); break;},
			_ = &mut period => {
				tracing::info!("Period");
				check_error(client.send_command_rc(47, 13, Rcs::Increment, None, None, None).await)?;
				check_error(client.send_command_sp(47, 14, Spi::On, None, None, None).await)?;
				check_error(client.send_command_dp(47, 15, Dpi::On, None, None, None).await)?;
				check_error(client.send_command_bs(47, 16, 1, None).await)?;


				period.as_mut().reset(Instant::now() + Duration::from_secs(1));
			},
			_ = &mut stop => {
				tracing::info!("Stopping");
				client.stop_receiving().await.whatever_context("Failed to stop receiving")?;
				stop.as_mut().reset(Instant::now() + Duration::from_secs(3600));
			}
			_ = &mut restart => {
				tracing::info!("Restarting");
				client.start_receiving().await.whatever_context("Failed to start receiving")?;
				restart.as_mut().reset(Instant::now() + Duration::from_secs(3615));
			}
		}
	}

	tracing::info!("Disconnecting");

	Ok(())
}

/// Callback for the client
struct MyCallback;

#[async_trait]
impl OnNewObjects for MyCallback {
	async fn on_new_objects(&self, _asdu: Asdu) {
		// tracing::info!("Received objects: {objects:?}");
	}
}

/// Check the error to see if it is a critical error
fn check_error(r: Result<(), ClientError>) -> Result<(), Whatever> {
	if let Err(e) = r {
		match e {
			ClientError::NoWriteChannel { .. } => {
				whatever!("There is no channel to send commands");
			}
			_ => {
				tracing::error!("Error sending ASDU: {e}");
				Ok(())
			}
		}
	} else {
		Ok(())
	}
}
