use snafu::Snafu;

use super::model::PointAddress;
use crate::{server::error::ServerError, types_id::TypeId};

/// Logical failure updating a point (model or broadcast).
#[derive(Debug, Snafu)]
#[snafu(visibility(pub), context(suffix(false)))]
pub enum SetPointError {
	/// No point registered at this address.
	#[snafu(display("unknown point: {address}"))]
	UnknownPoint { address: PointAddress },
	/// The new value variant does not match the registered point type.
	#[snafu(display("type mismatch for point {address}: expected {expected:?}, got {got:?}"))]
	TypeMismatch { address: PointAddress, expected: TypeId, got: TypeId },
	/// Spontaneous ASDU could not be sent to all connections.
	#[snafu(display("failed to broadcast spontaneous ASDU"))]
	BroadcastFailed { source: ServerError },
}

/// Error returned by
/// [`super::RtuServerHandle`](crate::rtu_server::RtuServerHandle) when the
/// actor is gone, a point already exists, or a [`SetPointError`] occurs.
#[derive(Debug, Snafu)]
#[snafu(visibility(pub), context(suffix(false)))]
pub enum RtuHandleError {
	/// The RTU actor task exited or the channel was closed.
	#[snafu(display("RTU server is no longer running"))]
	Disconnected,
	/// No response from the actor (task panic or drop).
	#[snafu(display("RTU server actor stopped before responding"))]
	ActorStopped,
	/// [`super::RtuServerHandle::register_point`](crate::rtu_server::RtuServerHandle::register_point)
	/// was called for an existing address.
	#[snafu(display("Point already registered: {address}"))]
	AlreadyRegistered { address: PointAddress },
	/// [`super::RtuServerHandle::set_point`](crate::rtu_server::RtuServerHandle::set_point) failed.
	#[snafu(display("{source}"))]
	SetPoint { source: SetPointError },
}

impl From<SetPointError> for RtuHandleError {
	fn from(source: SetPointError) -> Self {
		Self::SetPoint { source }
	}
}

/// Failure while handling a general interrogation on the wire (internal; logged
/// in ingress handler).
#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub(crate) enum InterrogationError {
	/// ASDU is not `C_IC_NA_1` activation with a supported qualifier.
	#[snafu(display("not handled as interrogation by RTU server"))]
	Skipped,
	#[snafu(display("failed to send interrogation activation confirmation: {source}"))]
	SendConfirmation { source: ServerError },
	#[snafu(display("failed to send interrogation data ASDU: {source}"))]
	SendData { source: ServerError },
}
