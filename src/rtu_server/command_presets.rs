//! Optional **convenience** mappings — not specified by IEC 60870-5-104.
//!
//! [`MapCommandsToSameIoaMonitoring`] assumes each command IOA maps to a
//! **monitoring point at the same IOA** with the matching measurement type
//! (e.g. `C_SC_NA_1` → `M_SP_NA_1`, `C_DC_NA_1` → `M_DP_NA_1`, `C_RC_NA_1` →
//! `M_ST_NA_1`, `C_BO_NA_1` → `M_BO_NA_1`, set-point types → `M_ME_*`). That
//! pattern appears in some simulators and simple devices, but real substations
//! often use different IOA layouts and side-effects; prefer a custom
//! [`super::command_handler::RtuCommandHandler`] for production.

use std::collections::HashMap;

use super::{
	command_handler::{CommandContext, CommandHandling, RtuCommandHandler},
	model::{PointAddress, PointValue},
};
use crate::{
	cot::Cot,
	types::{
		GenericObject, InformationObjects, commands::Rcs, information_elements::SelectExecute,
	},
	types_id::TypeId,
};

/// Build mirrors, walk IOAs, and assemble [`CommandHandling`] for same-IOA
/// command → monitoring mapping.
///
/// Pass loop-bound names (must match what you use after `=>`): `addr`, `cmd`,
/// `pv` (`&PointValue`), `negative`, `apply_updates`.
macro_rules! same_ioa_plan {
	(
		$ctx:expr,
		$ios:ident,
		$reply_tid:expr,
		$wrap:ident,
		$addr:ident,
		$cmd:ident,
		$pv:ident,
		$neg:ident,
		$updates:ident =>
		$($body:tt)*
	) => {{
		let ctx = $ctx;
		let InformationObjects::$ios(objs) = &ctx.asdu.information_objects else {
			return echo_reject(ctx);
		};
		let ca = ctx.asdu.address_field;
		let mut $neg = false;
		let mut mirrors = Vec::with_capacity(objs.len());
		let mut $updates = Vec::new();
		for go in objs {
			let $addr = PointAddress::new(ca, go.address);
			let $cmd = &go.object;
			mirrors.push(GenericObject {
				address: go.address,
				object: $cmd.clone(),
			});
			match ctx.model.get(&$addr) {
				Some($pv) => {
					$($body)*
				}
				None => $neg = true,
			}
		}
		CommandHandling {
			negative: $neg,
			reply_type_id: $reply_tid,
			reply_information_objects: InformationObjects::$wrap(mirrors),
			apply_updates: $updates,
		}
	}};
}

/// Same-IOA command → monitoring-point convention (see module docs).
#[derive(Debug, Default, Clone, Copy)]
pub struct MapCommandsToSameIoaMonitoring;

#[allow(clippy::too_many_lines)]
#[must_use]
fn all_command_ioas_in_model(
	ca: u16,
	model: &HashMap<PointAddress, PointValue>,
	mut ioas: impl Iterator<Item = u32>,
) -> bool {
	ioas.all(|ioa| model.contains_key(&PointAddress::new(ca, ioa)))
}

/// Select cancel: positive deactivation confirmation if every command IOA
/// exists for the ASDU common address; no process-image updates.
fn same_ioa_deactivation_echo(ctx: CommandContext<'_>) -> CommandHandling {
	use crate::types::InformationObjects as IO;
	let ca = ctx.asdu.address_field;

	let (negative, reply_ios) = match (ctx.asdu.type_id, &ctx.asdu.information_objects) {
		(TypeId::C_SC_NA_1, IO::CScNa1(objs)) => (
			!all_command_ioas_in_model(ca, ctx.model, objs.iter().map(|g| g.address)),
			IO::CScNa1(objs.clone()),
		),
		(TypeId::C_SC_TA_1, IO::CScTa1(objs)) => (
			!all_command_ioas_in_model(ca, ctx.model, objs.iter().map(|g| g.address)),
			IO::CScTa1(objs.clone()),
		),
		(TypeId::C_DC_NA_1, IO::CdcNa1(objs)) => (
			!all_command_ioas_in_model(ca, ctx.model, objs.iter().map(|g| g.address)),
			IO::CdcNa1(objs.clone()),
		),
		(TypeId::C_DC_TA_1, IO::CdcTa1(objs)) => (
			!all_command_ioas_in_model(ca, ctx.model, objs.iter().map(|g| g.address)),
			IO::CdcTa1(objs.clone()),
		),
		(TypeId::C_RC_NA_1, IO::CrcNa1(objs)) => (
			!all_command_ioas_in_model(ca, ctx.model, objs.iter().map(|g| g.address)),
			IO::CrcNa1(objs.clone()),
		),
		(TypeId::C_RC_TA_1, IO::CrcTa1(objs)) => (
			!all_command_ioas_in_model(ca, ctx.model, objs.iter().map(|g| g.address)),
			IO::CrcTa1(objs.clone()),
		),
		(TypeId::C_SE_NA_1, IO::CSeNa1(objs)) => (
			!all_command_ioas_in_model(ca, ctx.model, objs.iter().map(|g| g.address)),
			IO::CSeNa1(objs.clone()),
		),
		(TypeId::C_SE_TA_1, IO::CSeTa1(objs)) => (
			!all_command_ioas_in_model(ca, ctx.model, objs.iter().map(|g| g.address)),
			IO::CSeTa1(objs.clone()),
		),
		(TypeId::C_SE_NB_1, IO::CSeNb1(objs)) => (
			!all_command_ioas_in_model(ca, ctx.model, objs.iter().map(|g| g.address)),
			IO::CSeNb1(objs.clone()),
		),
		(TypeId::C_SE_TB_1, IO::CSeTb1(objs)) => (
			!all_command_ioas_in_model(ca, ctx.model, objs.iter().map(|g| g.address)),
			IO::CSeTb1(objs.clone()),
		),
		(TypeId::C_SE_NC_1, IO::CSeNc1(objs)) => (
			!all_command_ioas_in_model(ca, ctx.model, objs.iter().map(|g| g.address)),
			IO::CSeNc1(objs.clone()),
		),
		(TypeId::C_SE_TC_1, IO::CSeTc1(objs)) => (
			!all_command_ioas_in_model(ca, ctx.model, objs.iter().map(|g| g.address)),
			IO::CSeTc1(objs.clone()),
		),
		(TypeId::C_BO_NA_1, IO::CBoNa1(objs)) => (
			!all_command_ioas_in_model(ca, ctx.model, objs.iter().map(|g| g.address)),
			IO::CBoNa1(objs.clone()),
		),
		(TypeId::C_BO_TA_1, IO::CBoTa1(objs)) => (
			!all_command_ioas_in_model(ca, ctx.model, objs.iter().map(|g| g.address)),
			IO::CBoTa1(objs.clone()),
		),
		_ => return echo_reject(ctx),
	};
	CommandHandling {
		negative,
		reply_type_id: ctx.asdu.type_id,
		reply_information_objects: reply_ios,
		apply_updates: Vec::new(),
	}
}

#[allow(clippy::too_many_lines)]
#[async_trait::async_trait]
impl RtuCommandHandler for MapCommandsToSameIoaMonitoring {
	async fn handle_command(&self, ctx: CommandContext<'_>) -> CommandHandling {
		if matches!(ctx.asdu.cot, Cot::Deactivation) {
			return same_ioa_deactivation_echo(ctx);
		}
		match ctx.asdu.type_id {
			TypeId::C_SC_NA_1 => {
				same_ioa_plan!(ctx, CScNa1, TypeId::C_SC_NA_1, CScNa1, addr, cmd, pv, negative, apply_updates =>
					match pv {
						PointValue::MSpNa1(msp) => {
							if cmd.sco.se == SelectExecute::Execute {
								let mut m = msp.clone();
								m.siq.spi = cmd.sco.scs;
								apply_updates.push((addr, PointValue::MSpNa1(m)));
							}
						}
						_ => negative = true,
					}
				)
			}
			TypeId::C_SC_TA_1 => {
				same_ioa_plan!(ctx, CScTa1, TypeId::C_SC_TA_1, CScTa1, addr, cmd, pv, negative, apply_updates =>
					match pv {
						PointValue::MSpNa1(msp) => {
							if cmd.sco.se == SelectExecute::Execute {
								let mut m = msp.clone();
								m.siq.spi = cmd.sco.scs;
								apply_updates.push((addr, PointValue::MSpNa1(m)));
							}
						}
						_ => negative = true,
					}
				)
			}
			TypeId::C_DC_NA_1 => {
				same_ioa_plan!(ctx, CdcNa1, TypeId::C_DC_NA_1, CdcNa1, addr, cmd, pv, negative, apply_updates =>
					match pv {
						PointValue::MDpNa1(mdp) => {
							if cmd.dco.se == SelectExecute::Execute {
								let mut m = mdp.clone();
								m.diq.dpi = cmd.dco.dcs;
								apply_updates.push((addr, PointValue::MDpNa1(m)));
							}
						}
						_ => negative = true,
					}
				)
			}
			TypeId::C_DC_TA_1 => {
				same_ioa_plan!(ctx, CdcTa1, TypeId::C_DC_TA_1, CdcTa1, addr, cmd, pv, negative, apply_updates =>
					match pv {
						PointValue::MDpNa1(mdp) => {
							if cmd.dco.se == SelectExecute::Execute {
								let mut m = mdp.clone();
								m.diq.dpi = cmd.dco.dcs;
								apply_updates.push((addr, PointValue::MDpNa1(m)));
							}
						}
						_ => negative = true,
					}
				)
			}
			TypeId::C_RC_NA_1 => {
				same_ioa_plan!(ctx, CrcNa1, TypeId::C_RC_NA_1, CrcNa1, addr, cmd, pv, negative, apply_updates =>
					match pv {
						PointValue::MStNa1(st) => {
							if cmd.rco.se == SelectExecute::Execute {
								match cmd.rco.rcs {
									Rcs::Increment => {
										let mut m = st.clone();
										m.vti.value = m.vti.value.saturating_add(1).min(127);
										apply_updates.push((addr, PointValue::MStNa1(m)));
									}
									Rcs::Decrement => {
										let mut m = st.clone();
										m.vti.value = m.vti.value.saturating_sub(1);
										apply_updates.push((addr, PointValue::MStNa1(m)));
									}
									Rcs::None | Rcs::Invalid => negative = true,
								}
							}
						}
						_ => negative = true,
					}
				)
			}
			TypeId::C_RC_TA_1 => {
				same_ioa_plan!(ctx, CrcTa1, TypeId::C_RC_TA_1, CrcTa1, addr, cmd, pv, negative, apply_updates =>
					match pv {
						PointValue::MStNa1(st) => {
							if cmd.rco.se == SelectExecute::Execute {
								match cmd.rco.rcs {
									Rcs::Increment => {
										let mut m = st.clone();
										m.vti.value = m.vti.value.saturating_add(1).min(127);
										apply_updates.push((addr, PointValue::MStNa1(m)));
									}
									Rcs::Decrement => {
										let mut m = st.clone();
										m.vti.value = m.vti.value.saturating_sub(1);
										apply_updates.push((addr, PointValue::MStNa1(m)));
									}
									Rcs::None | Rcs::Invalid => negative = true,
								}
							}
						}
						_ => negative = true,
					}
				)
			}
			TypeId::C_SE_NA_1 => {
				same_ioa_plan!(ctx, CSeNa1, TypeId::C_SE_NA_1, CSeNa1, addr, cmd, pv, negative, apply_updates =>
					match pv {
						PointValue::MMeNa1(mm) => {
							if cmd.qos.se == SelectExecute::Execute {
								let mut m = mm.clone();
								m.nva = cmd.nva;
								apply_updates.push((addr, PointValue::MMeNa1(m)));
							}
						}
						_ => negative = true,
					}
				)
			}
			TypeId::C_SE_TA_1 => {
				same_ioa_plan!(ctx, CSeTa1, TypeId::C_SE_TA_1, CSeTa1, addr, cmd, pv, negative, apply_updates =>
					match pv {
						PointValue::MMeNa1(mm) => {
							if cmd.qos.se == SelectExecute::Execute {
								let mut m = mm.clone();
								m.nva = cmd.nva;
								apply_updates.push((addr, PointValue::MMeNa1(m)));
							}
						}
						_ => negative = true,
					}
				)
			}
			TypeId::C_SE_NB_1 => {
				same_ioa_plan!(ctx, CSeNb1, TypeId::C_SE_NB_1, CSeNb1, addr, cmd, pv, negative, apply_updates =>
					match pv {
						PointValue::MMeNb1(mm) => {
							if cmd.qos.se == SelectExecute::Execute {
								let mut m = mm.clone();
								m.sva = cmd.sva;
								apply_updates.push((addr, PointValue::MMeNb1(m)));
							}
						}
						_ => negative = true,
					}
				)
			}
			TypeId::C_SE_TB_1 => {
				same_ioa_plan!(ctx, CSeTb1, TypeId::C_SE_TB_1, CSeTb1, addr, cmd, pv, negative, apply_updates =>
					match pv {
						PointValue::MMeNb1(mm) => {
							if cmd.qos.se == SelectExecute::Execute {
								let mut m = mm.clone();
								m.sva = cmd.sva;
								apply_updates.push((addr, PointValue::MMeNb1(m)));
							}
						}
						_ => negative = true,
					}
				)
			}
			TypeId::C_SE_NC_1 => {
				same_ioa_plan!(ctx, CSeNc1, TypeId::C_SE_NC_1, CSeNc1, addr, cmd, pv, negative, apply_updates =>
					match pv {
						PointValue::MMeNc1(mm) => {
							if cmd.qos.se == SelectExecute::Execute {
								let mut m = mm.clone();
								m.value = cmd.value;
								apply_updates.push((addr, PointValue::MMeNc1(m)));
							}
						}
						_ => negative = true,
					}
				)
			}
			TypeId::C_SE_TC_1 => {
				same_ioa_plan!(ctx, CSeTc1, TypeId::C_SE_TC_1, CSeTc1, addr, cmd, pv, negative, apply_updates =>
					match pv {
						PointValue::MMeNc1(mm) => {
							if cmd.qos.se == SelectExecute::Execute {
								let mut m = mm.clone();
								m.value = cmd.value;
								apply_updates.push((addr, PointValue::MMeNc1(m)));
							}
						}
						_ => negative = true,
					}
				)
			}
			TypeId::C_BO_NA_1 => {
				same_ioa_plan!(ctx, CBoNa1, TypeId::C_BO_NA_1, CBoNa1, addr, cmd, pv, negative, apply_updates =>
					match pv {
						PointValue::MBoNa1(m) => {
							let mut x = m.clone();
							x.bsi = cmd.bsi;
							apply_updates.push((addr, PointValue::MBoNa1(x)));
						}
						_ => negative = true,
					}
				)
			}
			TypeId::C_BO_TA_1 => {
				same_ioa_plan!(ctx, CBoTa1, TypeId::C_BO_TA_1, CBoTa1, addr, cmd, pv, negative, apply_updates =>
					match pv {
						PointValue::MBoNa1(m) => {
							let mut x = m.clone();
							x.bsi = cmd.bsi;
							apply_updates.push((addr, PointValue::MBoNa1(x)));
						}
						_ => negative = true,
					}
				)
			}
			_ => echo_reject(ctx),
		}
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
