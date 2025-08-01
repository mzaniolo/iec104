//! Example client for IEC 60870-5-104

#![allow(clippy::expect_used, clippy::print_stdout)]

use iec_60870_5_104::{
	apdu::{Frame, UFrame},
	client::Client,
	config::ClientConfig,
};
use tracing_subscriber::{
	Layer as _, filter::EnvFilter, layer::SubscriberExt as _, util::SubscriberInitExt as _,
};

#[tokio::main]
async fn main() {
	let filter = EnvFilter::from("debug");
	let layer = tracing_subscriber::fmt::layer().with_filter(filter);
	tracing_subscriber::registry().with(layer).init();
	let mut client = Client::new(ClientConfig::default()).await.expect("Failed to create client");
	let frame = Frame::U(UFrame { start_dt_activation: true, ..Default::default() });
	client.send(frame).await.expect("Failed to send frame");
	let apdu = client.receive().await.expect("Failed to receive APDU");
	println!("Received APDU: {apdu:?}");
	loop {
		let apdu = client.receive().await.expect("Failed to receive APDU");
		println!("Received APDU: {apdu:?}");
	}
}
