//! Example multi-thread client for IEC 60870-5-104

use iec104::{
    apdu::{Frame, IFrame},
    asdu::Asdu,
    cot::Cot,
    error::Error,
    multi_thread::{
        base_connection::{ConnectionCallbacks, ConnectionEvent},
        client::{Client, ClientConfig},
    },
    types::{
        CIcNa1, CSeNc1, GenericObject, InformationObjects, commands::Qoi,
        information_elements::SelectExecute, quality_descriptors::Qos,
    },
    types_id::TypeId,
};
use snafu::ResultExt;
use std::sync::Arc;
use tokio::signal::unix::{SignalKind, signal};
use tracing_subscriber::{Layer, layer::SubscriberExt, util::SubscriberInitExt};

struct SampleClientCallback {
    name: String,
}

#[async_trait::async_trait]
impl ConnectionCallbacks for SampleClientCallback {
    async fn on_connection_event(&self, event: ConnectionEvent) -> Result<(), Error> {
        match event {
            ConnectionEvent::Opened => {
                tracing::info!("Connection \"{}\" opened.", self.name);
            }
            ConnectionEvent::Closed => {
                tracing::info!("Connection \"{}\" closed.", self.name);
            }
            ConnectionEvent::STARTDTCONReceived => {
                tracing::info!("Connection \"{}\" received START_DT_CON.", self.name);
            }
            ConnectionEvent::STOPDTCONReceived => {
                tracing::info!("Connection \"{}\" received STOP_DT_CON.", self.name);
            }
        }
        return Ok(());
    }
    async fn on_finish_receive_once(&self) -> Result<(), Error> {
        tracing::debug!(
            "Connection \"{}\" has just received some telegrams.",
            self.name
        );
        return Ok(());
    }
    async fn on_receive_i_frame(&self, iframe: IFrame) -> Result<Vec<Frame>, Error> {
        tracing::debug!(
            "Connection \"{}\" received I frame: {:?}",
            self.name,
            iframe
        );
        match iframe.asdu.information_objects {
            InformationObjects::MMeNa1(generic_objects) => {
                if iframe.asdu.type_id == TypeId::M_ME_NA_1 {
                    for obj in generic_objects {
                        tracing::info!("Measured normalized #{} = {}", obj.address, obj.object.nva);
                    }
                } else {
                    snafu::whatever!(
                        "Type mismatch, {:?} expected, got {:?}",
                        TypeId::M_ME_NA_1,
                        iframe.asdu.type_id
                    );
                }
            }
            InformationObjects::MMeTa1(generic_objects) => {
                if iframe.asdu.type_id == TypeId::M_ME_TA_1 {
                    for obj in generic_objects {
                        tracing::info!(
                            "Measured normalized #{} = {}, with time tag {:?}",
                            obj.address,
                            obj.object.nva,
                            obj.object.time
                        );
                    }
                } else {
                    snafu::whatever!(
                        "Type mismatch, {:?} expected, got {:?}",
                        TypeId::M_ME_TA_1,
                        iframe.asdu.type_id
                    );
                }
            }
            InformationObjects::MMeTd1(generic_objects) => {
                if iframe.asdu.type_id == TypeId::M_ME_TD_1 {
                    for obj in generic_objects {
                        tracing::info!(
                            "Measured normalized #{} = {}, with time tag {:?}",
                            obj.address,
                            obj.object.nva,
                            obj.object.time
                        );
                    }
                } else {
                    snafu::whatever!(
                        "Type mismatch, {:?} expected, got {:?}",
                        TypeId::M_ME_TD_1,
                        iframe.asdu.type_id
                    );
                }
            }
            InformationObjects::MMeNb1(generic_objects) => {
                if iframe.asdu.type_id == TypeId::M_ME_NB_1 {
                    for obj in generic_objects {
                        tracing::info!("Measured scaled #{} = {}", obj.address, obj.object.sva);
                    }
                } else {
                    snafu::whatever!(
                        "Type mismatch, {:?} expected, got {:?}",
                        TypeId::M_ME_NB_1,
                        iframe.asdu.type_id
                    );
                }
            }
            InformationObjects::MMeTb1(generic_objects) => {
                if iframe.asdu.type_id == TypeId::M_ME_TB_1 {
                    for obj in generic_objects {
                        tracing::info!(
                            "Measured scaled #{} = {}, with time tag {:?}",
                            obj.address,
                            obj.object.sva,
                            obj.object.time
                        );
                    }
                } else {
                    snafu::whatever!(
                        "Type mismatch, {:?} expected, got {:?}",
                        TypeId::M_ME_TB_1,
                        iframe.asdu.type_id
                    );
                }
            }
            InformationObjects::MMeTe1(generic_objects) => {
                if iframe.asdu.type_id == TypeId::M_ME_TE_1 {
                    for obj in generic_objects {
                        tracing::info!(
                            "Measured scaled #{} = {}, with time tag {:?}",
                            obj.address,
                            obj.object.sva,
                            obj.object.time
                        );
                    }
                } else {
                    snafu::whatever!(
                        "Type mismatch, {:?} expected, got {:?}",
                        TypeId::M_ME_TE_1,
                        iframe.asdu.type_id
                    );
                }
            }
            InformationObjects::MMeNc1(generic_objects) => {
                if iframe.asdu.type_id == TypeId::M_ME_NC_1 {
                    for obj in generic_objects {
                        tracing::info!(
                            "Measured short float #{} = {}",
                            obj.address,
                            obj.object.value
                        );
                    }
                } else {
                    snafu::whatever!(
                        "Type mismatch, {:?} expected, got {:?}",
                        TypeId::M_ME_NC_1,
                        iframe.asdu.type_id
                    );
                }
            }
            InformationObjects::MMeTc1(generic_objects) => {
                if iframe.asdu.type_id == TypeId::M_ME_TC_1 {
                    for obj in generic_objects {
                        tracing::info!(
                            "Measured short float #{} = {}, with time tag {:?}",
                            obj.address,
                            obj.object.value,
                            obj.object.time
                        );
                    }
                } else {
                    snafu::whatever!(
                        "Type mismatch, {:?} expected, got {:?}",
                        TypeId::M_ME_TC_1,
                        iframe.asdu.type_id
                    );
                }
            }
            InformationObjects::MMeTf1(generic_objects) => {
                if iframe.asdu.type_id == TypeId::M_ME_TF_1 {
                    for obj in generic_objects {
                        tracing::info!(
                            "Measured short float #{} = {}, with time tag {:?}",
                            obj.address,
                            obj.object.value,
                            obj.object.time
                        );
                    }
                } else {
                    snafu::whatever!(
                        "Type mismatch, {:?} expected, got {:?}",
                        TypeId::M_ME_TF_1,
                        iframe.asdu.type_id
                    );
                }
            }
            _ => {
                // Other types ...
            }
        }
        return Ok(Vec::new());
    }
    async fn on_error(&self, e: Error) {
        tracing::error!("{}", e);
    }
}

async fn interrogation_thread(client: Client) -> Result<(), Error> {
    // Delay 1 s.
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    loop {
        let information_objects = InformationObjects::CIcNa1(vec![GenericObject {
            address: 0,
            object: CIcNa1 { qoi: Qoi::Global },
        }]);
        let asdu_data = Asdu {
            type_id: TypeId::C_IC_NA_1,
            information_objects,
            originator_address: 0,
            address_field: 1,
            sequence: false,
            test: false,
            cot: Cot::Activation,
            // I think this field should be called `negative`... `false` means positive!
            positive: false,
        };
        // `send_sequence_number` and `receive_sequence_number` will be automatically filled when sending.
        let frame = Frame::I(IFrame {
            send_sequence_number: 0,
            receive_sequence_number: 0,
            asdu: asdu_data,
        });
        client.send(frame).await?;
        // Interval 60 s.
        tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
    }
}

async fn set_point_thread(client: Client) -> Result<(), Error> {
    // Must match server side.
    const SET_POINT_ADDRESS: u32 = 6000;
    let mut value = 0.0f32;
    // Delay 2 s.
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    loop {
        value += 1.0f32;
        let information_objects = InformationObjects::CSeNc1(vec![GenericObject {
            address: SET_POINT_ADDRESS,
            object: CSeNc1 {
                value,
                qos: Qos {
                    se: SelectExecute::Execute,
                    ..Default::default()
                },
            },
        }]);
        let asdu_data = Asdu {
            type_id: TypeId::C_SE_NC_1,
            information_objects,
            originator_address: 0,
            address_field: 1,
            sequence: false,
            test: false,
            cot: Cot::Activation,
            positive: false,
        };
        // `send_sequence_number` and `receive_sequence_number` will be automatically filled when sending.
        let frame = Frame::I(IFrame {
            send_sequence_number: 0,
            receive_sequence_number: 0,
            asdu: asdu_data,
        });
        client.send_and_wait_confirm(frame, false).await?;
        tracing::info!(
            "Set short float #{} = {}",
            SET_POINT_ADDRESS,
            value
        );
        // Interval 1 s.
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
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
    let config=ClientConfig::default();
    let callbacks = Arc::new(SampleClientCallback {
        name: "test".to_string(),
    });
    let client = Client::new(config, callbacks)
        .await
        .whatever_context("Failed to create client")?;
    let mut s1 = signal(SignalKind::interrupt()).whatever_context("Failed to create signal")?;
    let mut s2 = signal(SignalKind::terminate()).whatever_context("Failed to create signal")?;
    loop {
        tokio::select! {
            res = interrogation_thread(client.clone())=>{
                res.whatever_context("Error in interrogation thread")?;
            }
            res = set_point_thread(client.clone())=>{
                res.whatever_context("Error in set point thread")?;
            }
            _ = s1.recv() => {tracing::info!("SIGINT"); break;},
            _ = s2.recv() => {tracing::info!("SIGTERM"); break;},
        }
    }
    tracing::info!("Stopping");
    match client.close().await {
        Ok(_) => {}
        Err(e) => {
            tracing::warn!("Failed to gracefully stop");
            tracing::info!("Force shutdown");
            client.force_shutdown();
            return Err(snafu::FromString::with_source(
                e.into(),
                "Failed to close client".to_string(),
            ));
        }
    }
    return Ok(());
}
