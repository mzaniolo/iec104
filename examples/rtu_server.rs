//! Example high-level RTU server: in-memory points updated via
//! [`iec104::rtu_server::RtuServerHandle`].
//!
//! Command ASDUs are handled by
//! [`iec104::rtu_server::MapCommandsToSameIoaMonitoring`] (optional
//! test preset). In real integrations, implement
//! [`iec104::rtu_server::RtuCommandHandler`] yourself.
//!
//! **Point model (common address 47)** — for `tests/test_client.py` / manual
//! SCADA checks:
//!
//! | IOA | Type | Notes |
//! |-----|------|--------|
//! | 1, 3 | `M_SP_NA_1` | `C_SC_NA_1` same IOA |
//! | 11 | `M_ME_NC_1` | `C_SE_NC_1` |
//! | 20, 21 | `M_ME_NA_1` / `M_ME_NB_1` | `C_SE_NA_1` / `C_SE_NB_1` |
//! | 30 | `M_IT_NA_1` | integrated total; **counter interrogation group 1** (`C_CI_NA_1` RQT 1) |
//! | 40 | `M_ME_NA_1` | **general interrogation group 2** (`C_IC_NA_1` QOI 22) |

#![allow(clippy::expect_used)]
use std::{sync::Arc, time::Duration};

use iec104::{
	config::ServerConfig,
	rtu_server::{
		MapCommandsToSameIoaMonitoring, PointAddress, PointValue, RtuInitialPoint, RtuServer,
	},
	types::{
		MItNa1, MMeNa1, MMeNb1, MMeNc1, MSpNa1,
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

	let initial_points = vec![
		(
			PointAddress::new(47, 1),
			PointValue::MSpNa1(MSpNa1 {
				siq: Siq { iv: false, nt: false, sb: false, bl: false, spi: Spi::On },
			}),
		)
			.into(),
		(
			PointAddress::new(47, 3),
			PointValue::MSpNa1(MSpNa1 { siq: Siq { spi: Spi::Off, ..Default::default() } }),
		)
			.into(),
		(
			PointAddress::new(47, 11),
			PointValue::MMeNc1(MMeNc1 { value: 42.0, qds: Qds::default() }),
		)
			.into(),
		// Normalized / scaled set-point targets (`C_SE_NA_1` / `C_SE_NB_1`)
		(PointAddress::new(47, 20), PointValue::MMeNa1(MMeNa1 { nva: 0, qds: Qds::default() }))
			.into(),
		(PointAddress::new(47, 21), PointValue::MMeNb1(MMeNb1 { sva: 0, qds: Qds::default() }))
			.into(),
		// Integrated total + counter interrogation group 1 (RQT 1)
		RtuInitialPoint::with_counter_interrogation_group(
			PointAddress::new(47, 30),
			PointValue::MItNa1(MItNa1 { bcr: 12_345, qds: Qds::default() }),
			1,
		)
		.expect("IOA 30: M_IT + counter group 1"),
		// Float measurement in general interrogation group 2 (QOI 22)
		RtuInitialPoint::with_interrogation_group(
			PointAddress::new(47, 40),
			PointValue::MMeNa1(MMeNa1 { nva: 100, qds: Qds::default() }),
			2,
		)
		.expect("IOA 40: group 2 interrogation"),
	];

	let rtu = RtuServer::start(
		ServerConfig::default(),
		initial_points,
		Arc::new(MapCommandsToSameIoaMonitoring),
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
