//! Example multi-thread server for IEC 60870-5-104

use iec104::{
	apdu::{Frame, IFrame},
	asdu::Asdu,
	cot::Cot,
	error::Error,
	multi_thread::{
		base_connection::{ConnectionCallbacks, ConnectionEvent},
		client::ClientConfig,
		server::{Server, ServerCallbacks},
	},
	types::{
		GenericObject, InformationObjects, MMeNa1, MMeNb1, MMeNc1, commands::Qoi,
		information_elements::SelectExecute,
	},
	types_id::TypeId,
};
use snafu::ResultExt;
use std::sync::{Arc, Mutex};
use tokio::signal::unix::{SignalKind, signal};
use tracing_subscriber::{Layer, layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Clone, PartialEq)]
enum DataPointValue {
	Normalized(u16),
	Scaled(u16),
	Float(f32),
}

#[derive(Clone)]
struct DataPoint {
	read_address: Option<u32>,
	write_address: Option<u32>,
	value: DataPointValue,
}

struct SampleConnectionCallback {
	data: Arc<Mutex<Vec<DataPoint>>>,
	name: String,
}

struct SampleServerCallback {
	// Only short float points in this sample
	data: Arc<Mutex<Vec<DataPoint>>>,
	name: String,
}

#[async_trait::async_trait]
impl ConnectionCallbacks for SampleConnectionCallback {
	async fn on_connection_event(&self, event: ConnectionEvent) -> Result<(), Error> {
		match event {
			ConnectionEvent::Opened => {
				tracing::info!("[ Connection {} ] Opened.", self.name);
			}
			ConnectionEvent::Closed => {
				tracing::info!("[ Connection {} ] Closed.", self.name);
			}
			ConnectionEvent::STARTDTCONReceived => {
				tracing::info!("[ Connection {} ] Received START_DT_CON.", self.name);
			}
			ConnectionEvent::STOPDTCONReceived => {
				tracing::info!("[ Connection {} ] Received STOP_DT_CON.", self.name);
			}
		}
		return Ok(());
	}
	async fn on_finish_receive_once(&self) -> Result<(), Error> {
		tracing::debug!("[ Connection {} ] Received some telegrams.", self.name);
		return Ok(());
	}
	async fn on_receive_i_frame(&self, iframe: IFrame) -> Result<Vec<Frame>, Error> {
		tracing::debug!("[ Connection {} ] Received I frame: {:?}", self.name, iframe);
		match &iframe.asdu.information_objects {
			InformationObjects::CCsNa1(generic_objects) => {
				// Clock synchronization
				for obj in generic_objects {
					// Real clock synchronization not implemented yet!
					tracing::info!(
						"[ Connection {} ] Clock synchronization {:?}",
						self.name,
						obj.object.time
					);
				}
				let mut response = iframe.clone();
				response.asdu.cot = Cot::ActivationConfirmation;
				response.asdu.positive = false;
				return Ok(vec![Frame::I(response)]);
			}
			InformationObjects::CIcNa1(generic_objects) => {
				// Interrogation
				let mut responses = Vec::new();
				for obj in generic_objects {
					tracing::info!(
						"[ Connection {} ] Interrogation {:?}",
						self.name,
						obj.object.qoi
					);
					if obj.object.qoi == Qoi::Global {
						match self.data.lock() {
							Ok(guard) => {
								for point in guard.iter() {
									if let Some(addr) = point.read_address {
										responses.push(generate_measure_frame(
											&point.value,
											addr,
											Cot::InterrogationGeneral,
										));
									}
								}
							}
							Err(e) => {
								return Err(snafu::FromString::without_source(format!(
									"Lock error {}",
									e
								)));
							}
						}
					} else {
						let mut response = iframe.clone();
						response.asdu.cot = Cot::ActivationConfirmation;
						response.asdu.positive = true;
						return Ok(vec![Frame::I(response)]);
					}
				}
				let mut response = iframe.clone();
				response.asdu.cot = Cot::ActivationConfirmation;
				response.asdu.positive = false;
				responses.insert(0, Frame::I(response));
				// TODO: Merge multiple data in one frame if possible to accelate.
				return Ok(responses);
			}
			InformationObjects::CSeNa1(generic_objects) => {
				let mut succ = true;
				for obj in generic_objects {
					tracing::info!(
						"[ Connection {} ] Set normalized #{} to {:?}",
						self.name,
						obj.address,
						obj.object.nva
					);
					match self.data.lock() {
						Ok(mut guard) => {
							if let Some(pos) = guard
								.iter()
								.position(|point| point.write_address == Some(obj.address))
							{
								match &mut guard[pos].value {
									DataPointValue::Normalized(v) => {
										if obj.object.qos.se == SelectExecute::Execute {
											*v = obj.object.nva;
										}
									}
									_ => {
										succ = false;
										break;
									}
								}
							} else {
								let mut response = iframe.clone();
								response.asdu.cot = Cot::UnknownObjectAddress;
								response.asdu.positive = true;
								return Ok(vec![Frame::I(response)]);
							}
						}
						Err(e) => {
							return Err(snafu::FromString::without_source(format!(
								"Lock error {}",
								e
							)));
						}
					}
				}
				let mut response = iframe.clone();
				response.asdu.cot = Cot::ActivationConfirmation;
				response.asdu.positive = !succ;
				return Ok(vec![Frame::I(response)]);
			}
			InformationObjects::CSeNb1(generic_objects) => {
				let mut succ = true;
				for obj in generic_objects {
					tracing::info!(
						"[ Connection {} ] Set scaled #{} to {:?}",
						self.name,
						obj.address,
						obj.object.sva
					);
					match self.data.lock() {
						Ok(mut guard) => {
							if let Some(pos) = guard
								.iter()
								.position(|point| point.write_address == Some(obj.address))
							{
								match &mut guard[pos].value {
									DataPointValue::Scaled(v) => {
										if obj.object.qos.se == SelectExecute::Execute {
											*v = obj.object.sva;
										}
									}
									_ => {
										succ = false;
										break;
									}
								}
							} else {
								let mut response = iframe.clone();
								response.asdu.cot = Cot::UnknownObjectAddress;
								response.asdu.positive = true;
								return Ok(vec![Frame::I(response)]);
							}
						}
						Err(e) => {
							return Err(snafu::FromString::without_source(format!(
								"Lock error {}",
								e
							)));
						}
					}
				}
				let mut response = iframe.clone();
				response.asdu.cot = Cot::ActivationConfirmation;
				response.asdu.positive = !succ;
				return Ok(vec![Frame::I(response)]);
			}
			InformationObjects::CSeNc1(generic_objects) => {
				let mut succ = true;
				for obj in generic_objects {
					tracing::info!(
						"[ Connection {} ] Set short float #{} to {:?}",
						self.name,
						obj.address,
						obj.object.value
					);
					match self.data.lock() {
						Ok(mut guard) => {
							if let Some(pos) = guard
								.iter()
								.position(|point| point.write_address == Some(obj.address))
							{
								match &mut guard[pos].value {
									DataPointValue::Float(v) => {
										if obj.object.qos.se == SelectExecute::Execute {
											*v = obj.object.value;
										}
									}
									_ => {
										succ = false;
										break;
									}
								}
							} else {
								let mut response = iframe.clone();
								response.asdu.cot = Cot::UnknownObjectAddress;
								response.asdu.positive = true;
								return Ok(vec![Frame::I(response)]);
							}
						}
						Err(e) => {
							return Err(snafu::FromString::without_source(format!(
								"Lock error {}",
								e
							)));
						}
					}
				}
				let mut response = iframe.clone();
				response.asdu.cot = Cot::ActivationConfirmation;
				response.asdu.positive = !succ;
				return Ok(vec![Frame::I(response)]);
			}
			_ => {
				// Other types not supported yet...
				let mut response = iframe.clone();
				response.asdu.cot = Cot::UnknownType;
				response.asdu.positive = true;
				return Ok(vec![Frame::I(response)]);
			}
		}
	}
	async fn on_error(&self, e: Error) {
		tracing::error!("[ Connection {} ] {}", self.name, e);
	}
}

#[async_trait::async_trait]
impl ServerCallbacks for SampleServerCallback {
	async fn on_connection_requested(
		&self,
		address: std::net::SocketAddr,
	) -> Option<Arc<dyn ConnectionCallbacks + Send + Sync>> {
		tracing::info!("Incoming connection from {}", address.to_string());
		// Return None to reject connection.
		// Accept all connections.
		return Some(Arc::new(SampleConnectionCallback {
			name: address.to_string(),
			data: self.data.clone(),
		}));
	}
	async fn on_error(&self, e: Error) {
		tracing::error!("[ Server {} ] {}", self.name, e);
	}
}

fn generate_measure_frame(value: &DataPointValue, addr: u32, cot: Cot) -> Frame {
	let (information_objects, type_id) = match value {
		DataPointValue::Normalized(nva) => {
			let information_objects = InformationObjects::MMeNa1(vec![GenericObject {
				address: addr,
				object: MMeNa1 { nva: *nva, qds: Default::default() },
			}]);
			(information_objects, TypeId::M_ME_NA_1)
		}
		DataPointValue::Scaled(sva) => {
			let information_objects = InformationObjects::MMeNb1(vec![GenericObject {
				address: addr,
				object: MMeNb1 { sva: *sva, qds: Default::default() },
			}]);
			(information_objects, TypeId::M_ME_NB_1)
		}
		DataPointValue::Float(v) => {
			let information_objects = InformationObjects::MMeNc1(vec![GenericObject {
				address: addr,
				object: MMeNc1 { value: *v, qds: Default::default() },
			}]);
			(information_objects, TypeId::M_ME_NC_1)
		}
	};
	let asdu = Asdu {
		type_id,
		information_objects,
		originator_address: 0,
		address_field: 1,
		sequence: false,
		test: false,
		cot,
		positive: false,
	};
	// `send_sequence_number` and `receive_sequence_number` will be automatically filled when sending.
	let frame = Frame::I(IFrame { send_sequence_number: 0, receive_sequence_number: 0, asdu });
	return frame;
}

fn clone_point_data_read_addresses(
	data: &Arc<Mutex<Vec<DataPoint>>>,
) -> Result<Vec<Option<u32>>, Error> {
	match data.lock() {
		Ok(guard) => {
			return Ok(guard.iter().map(|point| point.read_address).collect());
		}
		Err(e) => {
			return Err(snafu::FromString::without_source(format!("Lock error {}", e)));
		}
	}
}

fn clone_point_data_values(
	data: &Arc<Mutex<Vec<DataPoint>>>,
) -> Result<Vec<DataPointValue>, Error> {
	match data.lock() {
		Ok(guard) => {
			return Ok(guard.iter().map(|point| point.value.clone()).collect());
		}
		Err(e) => {
			return Err(snafu::FromString::without_source(format!("Lock error {}", e)));
		}
	}
}

async fn spontaneous_scan_thread(
	server: Server,
	data: &Arc<Mutex<Vec<DataPoint>>>,
) -> Result<(), Error> {
	let addresses = clone_point_data_read_addresses(&data)?;
	let mut old_data = clone_point_data_values(&data)?;
	loop {
		// Interval 100 ms.
		tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
		let new_data = clone_point_data_values(&data)?;
		let mut responses = Vec::new();
		for i in 0..new_data.len() {
			if let Some(addr) = addresses[i] {
				if old_data[i] != new_data[i] {
					responses.push(generate_measure_frame(
						&new_data[i],
						addr,
						Cot::SpontaneousData,
					));
				}
			}
		}
		// TODO: Merge multiple data in one frame if possible to accelate.
		for frame in responses.into_iter() {
			server.broadcast(frame, true).await?;
		}
		old_data.splice(0..old_data.len(), new_data);
	}
}

#[tokio::main]
async fn main() -> Result<(), snafu::Whatever> {
	// Switch to `debug` to see more details.
	let filter = tracing_subscriber::EnvFilter::from("info");
	let layer = tracing_subscriber::fmt::layer().with_filter(filter);
	tracing_subscriber::registry()
		.with(layer)
		//needed to get the tracing_error working
		.with(
			tracing_error::ErrorLayer::default()
				.with_filter(tracing_subscriber::EnvFilter::from("debug")),
		)
		.init();
	let mut config = ClientConfig::default();
	// Listen on all IPv4 addresses.
	config.address = "0.0.0.0".to_string();
	// Sample data points.
	let points = vec![DataPoint {
		read_address: Some(600),
		write_address: Some(6000),
		value: DataPointValue::Float(0.0),
	}];
	let data = Arc::new(Mutex::new(points));
	let callbacks = Arc::new(SampleServerCallback { name: "test".to_string(), data: data.clone() });
	let server =
		Server::new(config, callbacks).await.whatever_context("Failed to create server")?;
	let mut s1 = signal(SignalKind::interrupt()).whatever_context("Failed to create signal")?;
	let mut s2 = signal(SignalKind::terminate()).whatever_context("Failed to create signal")?;
	loop {
		tokio::select! {
			res = spontaneous_scan_thread(server.clone(),&data)=>{
				res.whatever_context("Error in spontaneous scan thread")?;
			}
			_ = s1.recv() => {tracing::info!("SIGINT"); break;},
			_ = s2.recv() => {tracing::info!("SIGTERM"); break;},
		}
	}
	tracing::info!("Stopping");
	match server.close().await {
		Ok(_) => {}
		Err(e) => {
			tracing::warn!("Failed to gracefully stop");
			tracing::info!("Force shutdown");
			server.force_shutdown();
			return Err(snafu::FromString::with_source(
				e.into(),
				"Failed to gracefully close server".to_string(),
			));
		}
	}
	return Ok(());
}
