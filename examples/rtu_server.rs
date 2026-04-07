//! Example high-level RTU server: in-memory points updated via
//! [`iec104::rtu_server::RtuServerHandle`].

use std::time::Duration;

use iec104::{
	config::ServerConfig,
	rtu_server::{PointAddress, PointValue, RtuServer},
	types::{
		MMeNc1, MSpNa1,
		information_elements::{Siq, Spi},
		quality_descriptors::Qds,
	},
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
		.with(ErrorLayer::default().with_filter(EnvFilter::from("debug")))
		.init();

	let rtu = RtuServer::start(
		ServerConfig::default(),
		[
			(
				PointAddress::new(47, 1),
				PointValue::MSpNa1(MSpNa1 {
					siq: Siq { iv: false, nt: false, sb: false, bl: false, spi: Spi::On },
				}),
			),
			(
				PointAddress::new(47, 3),
				PointValue::MSpNa1(MSpNa1 { siq: Siq { spi: Spi::Off, ..Default::default() } }),
			),
			(
				PointAddress::new(47, 11),
				PointValue::MMeNc1(MMeNc1 { value: 42.0, qds: Qds::default() }),
			),
		],
	)
	.await
	.whatever_context("Failed to start RTU server")?;

	let mut s1 = signal(SignalKind::interrupt()).whatever_context("Failed to create signal")?;
	let mut s2 = signal(SignalKind::terminate()).whatever_context("Failed to create signal")?;
	let period = tokio::time::sleep(Duration::from_secs(1));
	tokio::pin!(period);

	loop {
		tokio::select! {
			_ = s1.recv() => { tracing::info!("SIGINT"); break; }
			_ = s2.recv() => { tracing::info!("SIGTERM"); break; }
			_ = &mut period => {
				tracing::info!("tick: set IOA 1 off (see also spontaneous updates from your SCADA)");
				let res = rtu
					.clone()
					.set_point(
						PointAddress::new(47, 1),
						PointValue::MSpNa1(MSpNa1 {
							siq: Siq { iv: false, nt: false, sb: false, bl: false, spi: Spi::Off },
						}),
					)
					.await;
				if let Err(e) = res {
					tracing::error!("set_point: {e}");
				}
				period.as_mut().reset(Instant::now() + Duration::from_secs(1));
			}
		}
	}

	Ok(())
}
