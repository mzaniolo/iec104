//! Shared helpers for RTU server integration tests.
#![allow(
	missing_docs,
	missing_debug_implementations,
	clippy::expect_used,
	clippy::must_use_candidate,
	clippy::missing_const_for_fn
)]

use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use iec104::{
	asdu::Asdu,
	client::{Client, ClientCallback},
	config::{ClientConfig, ServerConfig},
	cot::Cot,
	rtu_server::{
		RtuCommandHandler, RtuInitialPoint, RtuServerHandle, RtuSystemHandlers,
		start_rtu_server_ephemeral,
	},
	types::{
		CCiNa1, CCsNa1, CIcNa1, CRpNa1, CScNa1, GenericObject, InformationObjects, MSpNa1,
		commands::{Frz, Qoi, Qrp, Qu, Rqt, Sco},
		information_elements::{SelectExecute, Siq, Spi},
		time::Cp56Time2a,
	},
	types_id::TypeId,
};
use tokio::sync::mpsc;

/// Upper bound for waiting on protocol reactions in these tests.
pub const WAIT: Duration = Duration::from_secs(8);

pub struct AsduCollector {
	pub tx: mpsc::UnboundedSender<Asdu>,
}

#[async_trait]
impl ClientCallback for AsduCollector {
	async fn on_new_objects(&self, asdu: Asdu) {
		let _ = self.tx.send(asdu);
	}
}

pub fn client_cfg_for(listen: std::net::SocketAddr) -> ClientConfig {
	let mut protocol = ClientConfig::default().protocol.clone();
	protocol.t1 = Duration::from_secs(8);
	protocol.t3 = Duration::from_secs(8);
	ClientConfig { address: listen.ip().to_string(), port: listen.port(), protocol, tls: None }
}

pub async fn spawn_rtu_and_client(
	initial: Vec<RtuInitialPoint>,
	command_handler: Arc<dyn RtuCommandHandler>,
	system_handlers: RtuSystemHandlers,
) -> (RtuServerHandle, Client<AsduCollector>, mpsc::UnboundedReceiver<Asdu>) {
	let (rtu, listen) = start_rtu_server_ephemeral(
		ServerConfig::default(),
		initial,
		command_handler,
		system_handlers,
	)
	.await
	.expect("start_rtu_server_ephemeral");
	let (tx, rx) = mpsc::unbounded_channel();
	let mut client = Client::new(client_cfg_for(listen), AsduCollector { tx });
	client.connect().await.expect("client.connect");
	client.start_receiving().await.expect("client.start_receiving");
	(rtu, client, rx)
}

pub async fn recv_until(
	rx: &mut mpsc::UnboundedReceiver<Asdu>,
	timeout: Duration,
	mut pred: impl FnMut(&Asdu) -> bool,
) -> Asdu {
	tokio::time::timeout(timeout, async {
		loop {
			let asdu = rx.recv().await.expect("ASDU channel closed");
			if pred(&asdu) {
				break asdu;
			}
		}
	})
	.await
	.expect("recv_until timed out")
}

pub async fn drain_until_m_ei(rx: &mut mpsc::UnboundedReceiver<Asdu>) -> Asdu {
	tokio::time::timeout(WAIT, async {
		loop {
			let asdu = rx.recv().await.expect("ASDU channel closed");
			if asdu.type_id == TypeId::M_EI_NA_1 {
				break asdu;
			}
		}
	})
	.await
	.expect("timeout waiting for M_EI_NA_1")
}

pub fn c_ic_activation(ca: u16, qoi_ioa: u32, qoi: Qoi) -> Asdu {
	Asdu {
		type_id: TypeId::C_IC_NA_1,
		cot: Cot::Activation,
		originator_address: 0,
		address_field: ca,
		sequence: false,
		test: false,
		negative: false,
		information_objects: InformationObjects::CIcNa1(vec![GenericObject {
			address: qoi_ioa,
			object: CIcNa1 { qoi },
		}]),
	}
}

pub fn c_cs_activation(ca: u16, ioa: u32, time: Cp56Time2a) -> Asdu {
	Asdu {
		type_id: TypeId::C_CS_NA_1,
		cot: Cot::Activation,
		originator_address: 0,
		address_field: ca,
		sequence: false,
		test: false,
		negative: false,
		information_objects: InformationObjects::CCsNa1(vec![GenericObject {
			address: ioa,
			object: CCsNa1 { time },
		}]),
	}
}

pub fn c_rp_activation(ca: u16, ioa: u32, qrp: Qrp) -> Asdu {
	Asdu {
		type_id: TypeId::C_RP_NA_1,
		cot: Cot::Activation,
		originator_address: 0,
		address_field: ca,
		sequence: false,
		test: false,
		negative: false,
		information_objects: InformationObjects::CRpNa1(vec![GenericObject {
			address: ioa,
			object: CRpNa1 { qrp },
		}]),
	}
}

pub fn c_ci_activation(ca: u16, ioa: u32, rqt: Rqt, frz: Frz) -> Asdu {
	Asdu {
		type_id: TypeId::C_CI_NA_1,
		cot: Cot::Activation,
		originator_address: 0,
		address_field: ca,
		sequence: false,
		test: false,
		negative: false,
		information_objects: InformationObjects::CCiNa1(vec![GenericObject {
			address: ioa,
			object: CCiNa1 { rqt, frz },
		}]),
	}
}

pub fn c_sc_execute_on(ca: u16, ioa: u32, spi: Spi) -> Asdu {
	Asdu {
		type_id: TypeId::C_SC_NA_1,
		cot: Cot::Activation,
		originator_address: 0,
		address_field: ca,
		sequence: false,
		test: false,
		negative: false,
		information_objects: InformationObjects::CScNa1(vec![GenericObject {
			address: ioa,
			object: CScNa1 {
				sco: Sco { se: SelectExecute::Execute, qu: Qu::Unspecified, scs: spi },
			},
		}]),
	}
}

pub fn m_sp_off() -> MSpNa1 {
	MSpNa1 { siq: Siq { iv: false, nt: false, sb: false, bl: false, spi: Spi::Off } }
}
