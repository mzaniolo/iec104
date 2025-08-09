use std::fmt;

use snafu::Snafu;
use tracing_error::SpanTrace;

use crate::asdu::AsduError;

#[derive(Debug)]
pub struct SpanTraceWrapper(SpanTrace);

impl snafu::GenerateImplicitData for Box<SpanTraceWrapper> {
	fn generate() -> Self {
		Box::new(SpanTraceWrapper(SpanTrace::capture()))
	}
}

impl fmt::Display for SpanTraceWrapper {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		if self.0.status() == tracing_error::SpanTraceStatus::CAPTURED {
			write!(f, "\nAt:\n")?;
			self.0.fmt(f)?;
		}
		Ok(())
	}
}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub), context(suffix(false)))]
pub enum Error {
	#[snafu(display("Invalid data{context}"))]
	ApduTooShort {
		#[snafu(implicit)]
		context: Box<SpanTraceWrapper>,
	},

	#[snafu(display("Invalid telegram header{context}"))]
	InvalidTelegramHeader {
		#[snafu(implicit)]
		context: Box<SpanTraceWrapper>,
	},

	#[snafu(display("Invalid length{context}"))]
	InvalidLength {
		#[snafu(implicit)]
		context: Box<SpanTraceWrapper>,
	},

	#[snafu(display("Invalid control fields{context}"))]
	InvalidControlFields {
		#[snafu(implicit)]
		context: Box<SpanTraceWrapper>,
	},

	#[snafu(display("Invalid control field{context}"))]
	InvalidControlField {
		#[snafu(implicit)]
		context: Box<SpanTraceWrapper>,
	},

	#[snafu(display("Invalid S-frame control fields{context}"))]
	InvalidSFrameControlFields {
		#[snafu(implicit)]
		context: Box<SpanTraceWrapper>,
	},

	#[snafu(display("Invalid U-frame control fields{context}"))]
	InvalidUFrameControlFields {
		#[snafu(implicit)]
		context: Box<SpanTraceWrapper>,
	},

	#[snafu(display("Invalid I-frame control fields{context}"))]
	InvalidIFrameControlFields {
		#[snafu(implicit)]
		context: Box<SpanTraceWrapper>,
	},

	#[snafu(display("Invalid ASDU{context}"))]
	InvalidAsdu {
		source: AsduError,
		#[snafu(implicit)]
		context: Box<SpanTraceWrapper>,
	},
	#[snafu(display("Failed to convert to sized slice"))]
	SizedSlice {
		source: std::array::TryFromSliceError,
		#[snafu(implicit)]
		context: Box<SpanTraceWrapper>,
	},
	#[snafu(display("Not enough bytes"))]
	NotEnoughBytes {
		#[snafu(implicit)]
		context: Box<SpanTraceWrapper>,
	},

	#[snafu(whatever, display("{message}{context}\n{source:?}"))]
	Whatever {
		message: String,
		#[snafu(source(from(Box<dyn std::error::Error + Send + Sync>, Some)))]
		source: Option<Box<dyn std::error::Error + Send + Sync>>,
		#[snafu(implicit)]
		context: Box<SpanTraceWrapper>,
	},
}
