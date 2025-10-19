use snafu::Snafu;
use tokio::sync::mpsc;

use super::connection_handler::ConnectionHandlerCommand;
use crate::error::SpanTraceWrapper;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub), context(suffix(false)))]
pub enum LinkError {
	#[snafu(display("Client is not connected"))]
	NotConnected {
		#[snafu(implicit)]
		context: Box<SpanTraceWrapper>,
	},
	#[snafu(display("Client is reconnecting"))]
	Reconnecting {
		#[snafu(implicit)]
		context: Box<SpanTraceWrapper>,
	},
	#[snafu(display("Client is not receiving"))]
	NotReceiving {
		#[snafu(implicit)]
		context: Box<SpanTraceWrapper>,
	},
	#[snafu(display("Error sending command"))]
	SendCommand {
		source: mpsc::error::SendError<ConnectionHandlerCommand>,
		#[snafu(implicit)]
		context: Box<SpanTraceWrapper>,
	},
	#[snafu(display("Client is already started"))]
	AlreadyStarted {
		#[snafu(implicit)]
		context: Box<SpanTraceWrapper>,
	},
	#[snafu(display("Output buffer is full"))]
	OutputBufferFull {
		#[snafu(implicit)]
		context: Box<SpanTraceWrapper>,
	},
	#[snafu(display("There is no channel to send commands"))]
	NoWriteChannel {
		#[snafu(implicit)]
		context: Box<SpanTraceWrapper>,
	},
}
