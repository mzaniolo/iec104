use snafu::Snafu;
use tracing::instrument;

use crate::error::SpanTraceWrapper;

/// CP24Time2a time type
#[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
pub struct Cp24Time2a {
	/// Milliseconds (0-59999)
	pub ms: u16,
	/// Minutes (0-59)
	pub min: u8,
	/// Invalid flag
	pub iv: bool,
}

impl Cp24Time2a {
	#[instrument]
	pub fn from_bytes(bytes: &[u8]) -> Result<Self, ParseTimeError> {
		let ms = u16::from_be_bytes([bytes[1], bytes[0]]);
		let min = bytes[2] & 0b0011_1111;
		let iv = bytes[2] & 0b1000_0000 != 0;
		if ms > 59999 {
			return MillisecondsError.fail()?;
		}
		if min > 59 {
			return MinutesError.fail()?;
		}
		Ok(Self { ms, min, iv })
	}
}

/// CP16Time2a time type
#[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
pub struct Cp16Time2a {
	/// Milliseconds (0-59999)
	pub ms: u16,
}

impl Cp16Time2a {
	#[instrument]
	pub fn from_bytes(bytes: &[u8]) -> Result<Self, ParseTimeError> {
		let ms = u16::from_be_bytes([bytes[1], bytes[0]]);
		if ms > 59999 {
			return MillisecondsError.fail()?;
		}
		Ok(Self { ms })
	}
}

/// CP56Time2a time type
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Cp56Time2a {
	/// Milliseconds (0-59999)
	pub ms: u16,
	/// Invalid flag
	pub iv: bool,
	/// Minutes (0-59)
	pub min: u8,
	/// Summer time flag
	pub summer_time: bool,
	/// Hours (0-23)
	pub hour: u8,
	/// Weekday
	pub weekday: u8,
	/// Day of month (1-31)
	pub day: u8,
	/// Month (1-12)
	pub month: u8,
	/// Year (0-99)
	pub year: u8,
}

impl Cp56Time2a {
	#[instrument]
	pub fn from_bytes(bytes: &[u8]) -> Result<Self, ParseTimeError> {
		let ms = u16::from_be_bytes([bytes[1], bytes[0]]);
		let iv = bytes[2] & 0b1000_0000 != 0;
		let min = bytes[2] & 0b0011_1111;
		let summer_time = bytes[3] & 0b1000_0000 != 0;
		let hour = bytes[3] & 0b0001_1111;
		let weekday = (bytes[4] & 0b1110_0000) >> 5;
		let day = bytes[4] & 0b0001_1111;
		let month = bytes[5] & 0b0000_1111;
		let year = bytes[6] & 0b0111_1111;

		if ms > 59999 {
			return MillisecondsError.fail()?;
		}
		if min > 59 {
			return MinutesError.fail()?;
		}
		if hour > 23 {
			return HoursError.fail()?;
		}
		if day > 31 {
			return DaysError.fail()?;
		}
		if month > 12 {
			return MonthsError.fail()?;
		}
		if year > 99 {
			return YearsError.fail()?;
		}
		Ok(Self { ms, iv, min, summer_time, hour, weekday, day, month, year })
	}
}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub), context(suffix(Error)))]
pub enum ParseTimeError {
	#[snafu(display("Milliseconds out of range"))]
	Milliseconds {
		#[snafu(implicit)]
		context: SpanTraceWrapper,
	},
	#[snafu(display("Minutes out of range"))]
	Minutes {
		#[snafu(implicit)]
		context: SpanTraceWrapper,
	},
	#[snafu(display("Hours out of range"))]
	Hours {
		#[snafu(implicit)]
		context: SpanTraceWrapper,
	},
	#[snafu(display("Days out of range"))]
	Days {
		#[snafu(implicit)]
		context: SpanTraceWrapper,
	},
	#[snafu(display("Months out of range"))]
	Months {
		#[snafu(implicit)]
		context: SpanTraceWrapper,
	},
	#[snafu(display("Years out of range"))]
	Years {
		#[snafu(implicit)]
		context: SpanTraceWrapper,
	},
}
