//! Example link for IEC 60870-5-104

use std::time::Duration;

use async_trait::async_trait;
//use snafu::{ResultExt as _, Whatever, whatever};
use futures::FutureExt;
use futures::future::pending;
use iec104::{
	asdu::Asdu,
	config::LinkConfig,
	//	types::{
	//		commands::Rcs,
	//		information_elements::{Dpi, Spi},
	//	},
	//	link::{Link, OnNewObjects, errors::LinkError},
	link::{Link, OnNewObjects},
};
use snafu::{ResultExt as _, Whatever};
#[cfg(windows)]
use tokio::signal;
#[cfg(unix)]
use tokio::signal::unix::{SignalKind, signal};
use tokio::time::Instant;
use tracing_error::ErrorLayer;
use tracing_subscriber::{
	Layer as _, filter::EnvFilter, layer::SubscriberExt as _, util::SubscriberInitExt as _,
};

#[tokio::main]
async fn main() -> Result<(), Whatever> {
	let filter = EnvFilter::from("debug");
	let layer = tracing_subscriber::fmt::layer().with_filter(filter);
	tracing_subscriber::registry()
		.with(layer)
		//needed to get the tracing_error working
		.with(ErrorLayer::default().with_filter(EnvFilter::from("debug")))
		.init();

	let my_config = LinkConfig { server: true, ..Default::default() };

	let mut link = Link::new(my_config, MyCallback);

	link.connect().await.whatever_context("Failed to connect")?;
	link.start_receiving().await.whatever_context("Failed to start receiving")?;

	// Préparation des signaux de manière portable
	#[cfg(unix)]
	let mut sigint = signal(SignalKind::interrupt()).whatever_context("Failed to create signal")?;
	#[cfg(unix)]
	let mut sigterm = signal(SignalKind::terminate()).whatever_context("Failed to create signal")?;
	#[cfg(windows)]
	let ctrl_c = signal::ctrl_c();

	// Dummy futures pour la portabilité
	#[cfg(unix)]
	let mut ctrl_c_fut = pending::<()>().boxed();
	#[cfg(windows)]
	let mut sigint_fut = pending::<()>().boxed();
	#[cfg(windows)]
	let mut sigterm_fut = pending::<()>().boxed();

	#[cfg(unix)]
	let mut sigint_fut = sigint.recv().boxed();
	#[cfg(unix)]
	let mut sigterm_fut = sigterm.recv().boxed();
	#[cfg(windows)]
	let mut ctrl_c_fut = Box::pin(ctrl_c);

	let period = tokio::time::sleep(Duration::from_secs(1));
	tokio::pin!(period);

	let stop = tokio::time::sleep(Duration::from_secs(500));
	tokio::pin!(stop);

	let restart = tokio::time::sleep(Duration::from_secs(500));
	tokio::pin!(restart);

	loop {
		tokio::select! {
			_ = &mut sigint_fut => {
				tracing::info!("SIGINT/Ctrl+C");
				break;
			},
			_ = &mut sigterm_fut => {
				tracing::info!("SIGTERM");
				break;
			},
			_ = &mut ctrl_c_fut => {
				tracing::info!("Ctrl+C");
				break;
			},
			_ = &mut period => {
				tracing::info!("Period");
				period.as_mut().reset(Instant::now() + Duration::from_secs(10));
			},
			_ = &mut stop => {
				tracing::info!("Stopping");
				link.stop_receiving().await.whatever_context("Failed to stop receiving")?;
				stop.as_mut().reset(Instant::now() + Duration::from_secs(3600));
			}
			_ = &mut restart => {
				tracing::info!("Restarting");
				link.start_receiving().await.whatever_context("Failed to start receiving")?;
				restart.as_mut().reset(Instant::now() + Duration::from_secs(3615));
			}
		}
	}

	tracing::info!("Disconnecting");

	Ok(())
}

/// Callback for the link
struct MyCallback;

#[async_trait]
impl OnNewObjects for MyCallback {
	async fn on_new_objects(&self, _asdu: Asdu) {
		tracing::info!("Received objects: {_asdu:?}");
	}
}
/*
/// Check the error to see if it is a critical error
fn check_error(r: Result<(), LinkError>) -> Result<(), Whatever> {
	if let Err(e) = r {
		match e {
			LinkError::NoWriteChannel { .. } => {
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
 */
