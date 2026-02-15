use snafu::Snafu;
use tokio::sync::{mpsc, oneshot};

use crate::{
	error::SpanTraceWrapper,
	receive_handler::ReceiveHandlerCommand,
	server::{ConnectionId, ServerCommand},
};

#[derive(Debug, Snafu)]
#[snafu(visibility(pub), context(suffix(false)))]
pub enum ServerError {
	///Failed to start server
	Start {
		source: std::io::Error,
		#[snafu(implicit)]
		context: Box<SpanTraceWrapper>,
	},
	/// Failed to send command
	SendCommand {
		source: mpsc::error::SendError<ServerCommand>,
		#[snafu(implicit)]
		context: Box<SpanTraceWrapper>,
	},
	/// Failed to receive response
	ReceiveResponse {
		source: oneshot::error::RecvError,
		#[snafu(implicit)]
		context: Box<SpanTraceWrapper>,
	},
	/// Connection not found
	ConnectionNotFound {
		id: ConnectionId,
		#[snafu(implicit)]
		context: Box<SpanTraceWrapper>,
	},
	/// Failed to send connection command
	SendConnectionCommand {
		source: mpsc::error::SendError<ReceiveHandlerCommand>,
		#[snafu(implicit)]
		context: Box<SpanTraceWrapper>,
	},
}
