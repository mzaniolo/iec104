//! Optional **convenience** mappings — not specified by IEC 60870-5-104.
//!
//! [`MapCommandsToSameIoaMonitoring`] assumes each command IOA maps to a
//! **monitoring point at the same IOA** with the matching measurement type
//! (e.g. `C_SC_NA_1` → `M_SP_NA_1`). That pattern appears in some simulators
//! and simple devices, but real substations often use different IOA layouts and
//! side-effects; prefer a custom [`super::command_handler::RtuCommandHandler`]
//! for production.

use super::{
	command_handler::{CommandContext, CommandHandling, RtuCommandHandler},
	model::{PointAddress, PointValue},
};
use crate::{
	types::{
		CScNa1, CScTa1, CSeNa1, CSeNb1, CSeNc1, CSeTa1, CSeTb1, CSeTc1, GenericObject,
		InformationObjects, MMeNa1, MMeNb1, MMeNc1, MSpNa1, information_elements::SelectExecute,
	},
	types_id::TypeId,
};

/// Same-IOA command → monitoring-point convention (see module docs).
#[derive(Debug, Default, Clone, Copy)]
pub struct MapCommandsToSameIoaMonitoring;

#[async_trait::async_trait]
impl RtuCommandHandler for MapCommandsToSameIoaMonitoring {
	async fn handle_command(&self, ctx: CommandContext<'_>) -> CommandHandling {
		match ctx.asdu.type_id {
			TypeId::C_SC_NA_1 => plan_c_sc_na_1(ctx),
			TypeId::C_SC_TA_1 => plan_c_sc_ta_1(ctx),
			TypeId::C_SE_NA_1 => plan_c_se_na_1(ctx),
			TypeId::C_SE_NB_1 => plan_c_se_nb_1(ctx),
			TypeId::C_SE_NC_1 => plan_c_se_nc_1(ctx),
			TypeId::C_SE_TA_1 => plan_c_se_ta_1(ctx),
			TypeId::C_SE_TB_1 => plan_c_se_tb_1(ctx),
			TypeId::C_SE_TC_1 => plan_c_se_tc_1(ctx),
			_ => echo_reject(ctx),
		}
	}
}

fn plan_c_sc_na_1(ctx: CommandContext<'_>) -> CommandHandling {
	let InformationObjects::CScNa1(objs) = &ctx.asdu.information_objects else {
		return echo_reject(ctx);
	};
	let ca = ctx.asdu.address_field;
	let mut negative = false;
	let mut mirrors: Vec<GenericObject<CScNa1>> = Vec::with_capacity(objs.len());
	let mut updates: Vec<(PointAddress, MSpNa1)> = Vec::new();

	for go in objs {
		let addr = PointAddress::new(ca, go.address);
		let cmd = &go.object;
		mirrors.push(GenericObject { address: go.address, object: cmd.clone() });

		match ctx.model.get(&addr) {
			Some(PointValue::MSpNa1(msp)) => {
				if cmd.sco.se == SelectExecute::Execute {
					let mut m = msp.clone();
					m.siq.spi = cmd.sco.scs;
					updates.push((addr, m));
				}
			}
			_ => negative = true,
		}
	}

	CommandHandling {
		negative,
		reply_type_id: TypeId::C_SC_NA_1,
		reply_information_objects: InformationObjects::CScNa1(mirrors),
		apply_updates: updates.into_iter().map(|(a, m)| (a, PointValue::MSpNa1(m))).collect(),
	}
}

fn plan_c_sc_ta_1(ctx: CommandContext<'_>) -> CommandHandling {
	let InformationObjects::CScTa1(objs) = &ctx.asdu.information_objects else {
		return echo_reject(ctx);
	};
	let ca = ctx.asdu.address_field;
	let mut negative = false;
	let mut mirrors: Vec<GenericObject<CScTa1>> = Vec::with_capacity(objs.len());
	let mut updates: Vec<(PointAddress, MSpNa1)> = Vec::new();

	for go in objs {
		let addr = PointAddress::new(ca, go.address);
		let cmd = &go.object;
		mirrors.push(GenericObject { address: go.address, object: cmd.clone() });

		match ctx.model.get(&addr) {
			Some(PointValue::MSpNa1(msp)) => {
				if cmd.sco.se == SelectExecute::Execute {
					let mut m = msp.clone();
					m.siq.spi = cmd.sco.scs;
					updates.push((addr, m));
				}
			}
			_ => negative = true,
		}
	}

	CommandHandling {
		negative,
		reply_type_id: TypeId::C_SC_TA_1,
		reply_information_objects: InformationObjects::CScTa1(mirrors),
		apply_updates: updates.into_iter().map(|(a, m)| (a, PointValue::MSpNa1(m))).collect(),
	}
}

fn plan_c_se_na_1(ctx: CommandContext<'_>) -> CommandHandling {
	let InformationObjects::CSeNa1(objs) = &ctx.asdu.information_objects else {
		return echo_reject(ctx);
	};
	let ca = ctx.asdu.address_field;
	let mut negative = false;
	let mut mirrors: Vec<GenericObject<CSeNa1>> = Vec::with_capacity(objs.len());
	let mut updates: Vec<(PointAddress, MMeNa1)> = Vec::new();

	for go in objs {
		let addr = PointAddress::new(ca, go.address);
		let cmd = &go.object;
		mirrors.push(GenericObject { address: go.address, object: cmd.clone() });

		match ctx.model.get(&addr) {
			Some(PointValue::MMeNa1(mm)) => {
				if cmd.qos.se == SelectExecute::Execute {
					let mut m = mm.clone();
					m.nva = cmd.nva;
					updates.push((addr, m));
				}
			}
			_ => negative = true,
		}
	}

	CommandHandling {
		negative,
		reply_type_id: TypeId::C_SE_NA_1,
		reply_information_objects: InformationObjects::CSeNa1(mirrors),
		apply_updates: updates.into_iter().map(|(a, m)| (a, PointValue::MMeNa1(m))).collect(),
	}
}

fn plan_c_se_ta_1(ctx: CommandContext<'_>) -> CommandHandling {
	let InformationObjects::CSeTa1(objs) = &ctx.asdu.information_objects else {
		return echo_reject(ctx);
	};
	let ca = ctx.asdu.address_field;
	let mut negative = false;
	let mut mirrors: Vec<GenericObject<CSeTa1>> = Vec::with_capacity(objs.len());
	let mut updates: Vec<(PointAddress, MMeNa1)> = Vec::new();

	for go in objs {
		let addr = PointAddress::new(ca, go.address);
		let cmd = &go.object;
		mirrors.push(GenericObject { address: go.address, object: cmd.clone() });

		match ctx.model.get(&addr) {
			Some(PointValue::MMeNa1(mm)) => {
				if cmd.qos.se == SelectExecute::Execute {
					let mut m = mm.clone();
					m.nva = cmd.nva;
					updates.push((addr, m));
				}
			}
			_ => negative = true,
		}
	}

	CommandHandling {
		negative,
		reply_type_id: TypeId::C_SE_TA_1,
		reply_information_objects: InformationObjects::CSeTa1(mirrors),
		apply_updates: updates.into_iter().map(|(a, m)| (a, PointValue::MMeNa1(m))).collect(),
	}
}

fn plan_c_se_nb_1(ctx: CommandContext<'_>) -> CommandHandling {
	let InformationObjects::CSeNb1(objs) = &ctx.asdu.information_objects else {
		return echo_reject(ctx);
	};
	let ca = ctx.asdu.address_field;
	let mut negative = false;
	let mut mirrors: Vec<GenericObject<CSeNb1>> = Vec::with_capacity(objs.len());
	let mut updates: Vec<(PointAddress, MMeNb1)> = Vec::new();

	for go in objs {
		let addr = PointAddress::new(ca, go.address);
		let cmd = &go.object;
		mirrors.push(GenericObject { address: go.address, object: cmd.clone() });

		match ctx.model.get(&addr) {
			Some(PointValue::MMeNb1(mm)) => {
				if cmd.qos.se == SelectExecute::Execute {
					let mut m = mm.clone();
					m.sva = cmd.sva;
					updates.push((addr, m));
				}
			}
			_ => negative = true,
		}
	}

	CommandHandling {
		negative,
		reply_type_id: TypeId::C_SE_NB_1,
		reply_information_objects: InformationObjects::CSeNb1(mirrors),
		apply_updates: updates.into_iter().map(|(a, m)| (a, PointValue::MMeNb1(m))).collect(),
	}
}

fn plan_c_se_tb_1(ctx: CommandContext<'_>) -> CommandHandling {
	let InformationObjects::CSeTb1(objs) = &ctx.asdu.information_objects else {
		return echo_reject(ctx);
	};
	let ca = ctx.asdu.address_field;
	let mut negative = false;
	let mut mirrors: Vec<GenericObject<CSeTb1>> = Vec::with_capacity(objs.len());
	let mut updates: Vec<(PointAddress, MMeNb1)> = Vec::new();

	for go in objs {
		let addr = PointAddress::new(ca, go.address);
		let cmd = &go.object;
		mirrors.push(GenericObject { address: go.address, object: cmd.clone() });

		match ctx.model.get(&addr) {
			Some(PointValue::MMeNb1(mm)) => {
				if cmd.qos.se == SelectExecute::Execute {
					let mut m = mm.clone();
					m.sva = cmd.sva;
					updates.push((addr, m));
				}
			}
			_ => negative = true,
		}
	}

	CommandHandling {
		negative,
		reply_type_id: TypeId::C_SE_TB_1,
		reply_information_objects: InformationObjects::CSeTb1(mirrors),
		apply_updates: updates.into_iter().map(|(a, m)| (a, PointValue::MMeNb1(m))).collect(),
	}
}

fn plan_c_se_nc_1(ctx: CommandContext<'_>) -> CommandHandling {
	let InformationObjects::CSeNc1(objs) = &ctx.asdu.information_objects else {
		return echo_reject(ctx);
	};
	let ca = ctx.asdu.address_field;
	let mut negative = false;
	let mut mirrors: Vec<GenericObject<CSeNc1>> = Vec::with_capacity(objs.len());
	let mut updates: Vec<(PointAddress, MMeNc1)> = Vec::new();

	for go in objs {
		let addr = PointAddress::new(ca, go.address);
		let cmd = &go.object;
		mirrors.push(GenericObject { address: go.address, object: cmd.clone() });

		match ctx.model.get(&addr) {
			Some(PointValue::MMeNc1(mm)) => {
				if cmd.qos.se == SelectExecute::Execute {
					let mut m = mm.clone();
					m.value = cmd.value;
					updates.push((addr, m));
				}
			}
			_ => negative = true,
		}
	}

	CommandHandling {
		negative,
		reply_type_id: TypeId::C_SE_NC_1,
		reply_information_objects: InformationObjects::CSeNc1(mirrors),
		apply_updates: updates.into_iter().map(|(a, m)| (a, PointValue::MMeNc1(m))).collect(),
	}
}

fn plan_c_se_tc_1(ctx: CommandContext<'_>) -> CommandHandling {
	let InformationObjects::CSeTc1(objs) = &ctx.asdu.information_objects else {
		return echo_reject(ctx);
	};
	let ca = ctx.asdu.address_field;
	let mut negative = false;
	let mut mirrors: Vec<GenericObject<CSeTc1>> = Vec::with_capacity(objs.len());
	let mut updates: Vec<(PointAddress, MMeNc1)> = Vec::new();

	for go in objs {
		let addr = PointAddress::new(ca, go.address);
		let cmd = &go.object;
		mirrors.push(GenericObject { address: go.address, object: cmd.clone() });

		match ctx.model.get(&addr) {
			Some(PointValue::MMeNc1(mm)) => {
				if cmd.qos.se == SelectExecute::Execute {
					let mut m = mm.clone();
					m.value = cmd.value;
					updates.push((addr, m));
				}
			}
			_ => negative = true,
		}
	}

	CommandHandling {
		negative,
		reply_type_id: TypeId::C_SE_TC_1,
		reply_information_objects: InformationObjects::CSeTc1(mirrors),
		apply_updates: updates.into_iter().map(|(a, m)| (a, PointValue::MMeNc1(m))).collect(),
	}
}

fn echo_reject(ctx: CommandContext<'_>) -> CommandHandling {
	CommandHandling {
		negative: true,
		reply_type_id: ctx.asdu.type_id,
		reply_information_objects: ctx.asdu.information_objects.clone(),
		apply_updates: Vec::new(),
	}
}
