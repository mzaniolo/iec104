//! TCP + STARTDT integration tests for [`iec104::rtu_server`] (see
//! [`iec104::rtu_server::start_rtu_server_ephemeral`]).
#![allow(clippy::expect_used)]

mod support;

use std::sync::Arc;

use iec104::{
	cot::Cot,
	rtu_server::{
		MapCommandsToSameIoaMonitoring, PointAddress, PointValue, RejectAllCommands,
		RtuInitialPoint, RtuSystemHandlers,
	},
	types::{
		InformationObjects, MItNa1, MMeNa1,
		commands::{Frz, Qoi, Qrp, Rqt},
		information_elements::Spi,
		quality_descriptors::Qds,
		time::Cp56Time2a,
	},
	types_id::TypeId,
};
use support::{
	WAIT, c_ci_activation, c_cs_activation, c_ic_activation, c_rp_activation, c_sc_execute_on,
	drain_until_m_ei, m_sp_off, recv_until, spawn_rtu_and_client,
};

#[tokio::test]
async fn rtu_sends_end_of_initialization_after_startdt() {
	let ca = 47_u16;
	let initial = vec![RtuInitialPoint::new(
		PointAddress::new(ca, 10),
		PointValue::MMeNa1(MMeNa1 { nva: 0, qds: Qds::default() }),
	)];
	let (_rtu, _client, mut rx) =
		spawn_rtu_and_client(initial, Arc::new(RejectAllCommands), RtuSystemHandlers::default())
			.await;
	let mei = drain_until_m_ei(&mut rx).await;
	assert_eq!(mei.type_id, TypeId::M_EI_NA_1);
	assert_eq!(mei.address_field, ca);
	assert_eq!(mei.cot, Cot::Initiated);
}

#[tokio::test]
async fn global_interrogation_activation_confirmation_data_termination() {
	let ca = 11_u16;
	let initial = vec![
		RtuInitialPoint::new(
			PointAddress::new(ca, 100),
			PointValue::MMeNa1(MMeNa1 { nva: 10, qds: Qds::default() }),
		),
		RtuInitialPoint::new(
			PointAddress::new(ca, 200),
			PointValue::MMeNa1(MMeNa1 { nva: 20, qds: Qds::default() }),
		),
	];
	let (_rtu, client, mut rx) =
		spawn_rtu_and_client(initial, Arc::new(RejectAllCommands), RtuSystemHandlers::default())
			.await;
	drain_until_m_ei(&mut rx).await;

	client.send_asdu(c_ic_activation(ca, 0, Qoi::Global)).await.expect("send C_IC");

	let actcon = recv_until(&mut rx, WAIT, |a| {
		a.type_id == TypeId::C_IC_NA_1 && a.cot == Cot::ActivationConfirmation && !a.negative
	})
	.await;
	assert_eq!(actcon.address_field, ca);

	let data = recv_until(&mut rx, WAIT, |a| {
		a.type_id == TypeId::M_ME_NA_1 && a.cot == Cot::InterrogationGeneral
	})
	.await;
	if let InformationObjects::MMeNa1(objs) = &data.information_objects {
		assert_eq!(objs.len(), 2, "global GI should return both M_ME_NA_1 points");
	} else {
		panic!("expected M_ME_NA_1 interrogation data, got {:?}", data.type_id);
	}

	let term = recv_until(&mut rx, WAIT, |a| {
		a.type_id == TypeId::C_IC_NA_1 && a.cot == Cot::ActivationTermination && !a.negative
	})
	.await;
	assert_eq!(term.address_field, ca);
}

#[tokio::test]
async fn group_interrogation_returns_only_group_points() {
	let ca = 12_u16;
	let initial = vec![
		RtuInitialPoint::new(
			PointAddress::new(ca, 10),
			PointValue::MMeNa1(MMeNa1 { nva: 1, qds: Qds::default() }),
		),
		RtuInitialPoint::with_interrogation_group(
			PointAddress::new(ca, 20),
			PointValue::MMeNa1(MMeNa1 { nva: 2, qds: Qds::default() }),
			2,
		)
		.expect("group 2"),
	];
	let (_rtu, client, mut rx) =
		spawn_rtu_and_client(initial, Arc::new(RejectAllCommands), RtuSystemHandlers::default())
			.await;
	drain_until_m_ei(&mut rx).await;

	client.send_asdu(c_ic_activation(ca, 0, Qoi::Group2)).await.expect("send C_IC group2");

	let _actcon = recv_until(&mut rx, WAIT, |a| {
		a.type_id == TypeId::C_IC_NA_1 && a.cot == Cot::ActivationConfirmation && !a.negative
	})
	.await;

	let data = recv_until(&mut rx, WAIT, |a| {
		a.type_id == TypeId::M_ME_NA_1 && a.cot == Cot::InterrogationGroup2
	})
	.await;
	if let InformationObjects::MMeNa1(objs) = &data.information_objects {
		assert_eq!(objs.len(), 1);
		assert_eq!(objs[0].address, 20);
	} else {
		panic!("expected M_ME_NA_1 group 2 data");
	}

	let _term = recv_until(&mut rx, WAIT, |a| {
		a.type_id == TypeId::C_IC_NA_1 && a.cot == Cot::ActivationTermination
	})
	.await;
}

#[tokio::test]
async fn unsupported_interrogation_qoi_negative_confirmation_only() {
	let ca = 13_u16;
	let initial = vec![RtuInitialPoint::new(
		PointAddress::new(ca, 1),
		PointValue::MMeNa1(MMeNa1 { nva: 0, qds: Qds::default() }),
	)];
	let (_rtu, client, mut rx) =
		spawn_rtu_and_client(initial, Arc::new(RejectAllCommands), RtuSystemHandlers::default())
			.await;
	drain_until_m_ei(&mut rx).await;

	client.send_asdu(c_ic_activation(ca, 0, Qoi::Other(19))).await.expect("send C_IC");

	let actcon = recv_until(&mut rx, WAIT, |a| {
		a.type_id == TypeId::C_IC_NA_1 && a.cot == Cot::ActivationConfirmation && a.negative
	})
	.await;
	assert_eq!(actcon.address_field, ca);

	let no_more = tokio::time::timeout(std::time::Duration::from_millis(700), rx.recv()).await;
	assert!(
		no_more.is_err(),
		"expected no further ASDUs (no ACTTERM / no interrogation data) after negative QOI"
	);
}

#[tokio::test]
async fn clock_sync_positive_activation_confirmation() {
	let ca = 14_u16;
	let initial = vec![RtuInitialPoint::new(
		PointAddress::new(ca, 1),
		PointValue::MMeNa1(MMeNa1 { nva: 0, qds: Qds::default() }),
	)];
	let (_rtu, client, mut rx) =
		spawn_rtu_and_client(initial, Arc::new(RejectAllCommands), RtuSystemHandlers::default())
			.await;
	drain_until_m_ei(&mut rx).await;

	let time = Cp56Time2a {
		ms: 0,
		iv: false,
		min: 30,
		summer_time: false,
		hour: 12,
		weekday: 3,
		day: 10,
		month: 4,
		year: 26,
	};
	client.send_asdu(c_cs_activation(ca, 0, time.clone())).await.expect("send C_CS");

	let reply = recv_until(&mut rx, WAIT, |a| {
		a.type_id == TypeId::C_CS_NA_1 && a.cot == Cot::ActivationConfirmation && !a.negative
	})
	.await;
	if let InformationObjects::CCsNa1(objs) = &reply.information_objects {
		assert_eq!(objs.len(), 1);
		assert_eq!(objs[0].object.time, time);
	} else {
		panic!("expected CCsNa1 echo");
	}
}

#[tokio::test]
async fn reset_process_positive_activation_confirmation() {
	let ca = 15_u16;
	let initial = vec![RtuInitialPoint::new(
		PointAddress::new(ca, 1),
		PointValue::MMeNa1(MMeNa1 { nva: 0, qds: Qds::default() }),
	)];
	let (_rtu, client, mut rx) =
		spawn_rtu_and_client(initial, Arc::new(RejectAllCommands), RtuSystemHandlers::default())
			.await;
	drain_until_m_ei(&mut rx).await;

	client.send_asdu(c_rp_activation(ca, 0, Qrp::General)).await.expect("send C_RP");

	let reply = recv_until(&mut rx, WAIT, |a| {
		a.type_id == TypeId::C_RP_NA_1 && a.cot == Cot::ActivationConfirmation && !a.negative
	})
	.await;
	assert_eq!(reply.address_field, ca);
}

#[tokio::test]
async fn counter_interrogation_general_returns_m_it_and_termination() {
	let ca = 16_u16;
	let initial = vec![RtuInitialPoint::new(
		PointAddress::new(ca, 50),
		PointValue::MItNa1(MItNa1 { bcr: 99, qds: Qds::default() }),
	)];
	let (_rtu, client, mut rx) =
		spawn_rtu_and_client(initial, Arc::new(RejectAllCommands), RtuSystemHandlers::default())
			.await;
	drain_until_m_ei(&mut rx).await;

	client.send_asdu(c_ci_activation(ca, 0, Rqt::ReqCoGen, Frz::Read)).await.expect("send C_CI");

	let _actcon = recv_until(&mut rx, WAIT, |a| {
		a.type_id == TypeId::C_CI_NA_1 && a.cot == Cot::ActivationConfirmation && !a.negative
	})
	.await;

	let data = recv_until(&mut rx, WAIT, |a| {
		a.type_id == TypeId::M_IT_NA_1 && a.cot == Cot::CounterInterrogationGeneral
	})
	.await;
	if let InformationObjects::MItNa1(objs) = &data.information_objects {
		assert_eq!(objs.len(), 1);
		assert_eq!(objs[0].address, 50);
		assert_eq!(objs[0].object.bcr, 99);
	} else {
		panic!("expected M_IT_NA_1 counter interrogation data");
	}

	let term = recv_until(&mut rx, WAIT, |a| {
		a.type_id == TypeId::C_CI_NA_1 && a.cot == Cot::ActivationTermination && !a.negative
	})
	.await;
	assert_eq!(term.address_field, ca);
}

#[tokio::test]
async fn single_command_updates_monitoring_then_confirmation() {
	let ca = 17_u16;
	let ioa = 5_u32;
	let initial =
		vec![RtuInitialPoint::new(PointAddress::new(ca, ioa), PointValue::MSpNa1(m_sp_off()))];
	let (_rtu, client, mut rx) = spawn_rtu_and_client(
		initial,
		Arc::new(MapCommandsToSameIoaMonitoring),
		RtuSystemHandlers::default(),
	)
	.await;
	drain_until_m_ei(&mut rx).await;

	client.send_asdu(c_sc_execute_on(ca, ioa, Spi::On)).await.expect("send C_SC");

	let spontaneous = recv_until(&mut rx, WAIT, |a| {
		a.type_id == TypeId::M_SP_NA_1 && a.cot == Cot::SpontaneousData && a.address_field == ca
	})
	.await;
	if let InformationObjects::MSpNa1(objs) = &spontaneous.information_objects {
		assert_eq!(objs[0].address, ioa);
		assert_eq!(objs[0].object.siq.spi, Spi::On);
	} else {
		panic!("expected spontaneous M_SP_NA_1");
	}

	let actcon = recv_until(&mut rx, WAIT, |a| {
		a.type_id == TypeId::C_SC_NA_1 && a.cot == Cot::ActivationConfirmation && !a.negative
	})
	.await;
	assert_eq!(actcon.address_field, ca);
}
