use snafu::{ResultExt, Snafu};
use time::{Date, Month, OffsetDateTime, PrimitiveDateTime, Time};

use crate::error::SpanTraceWrapper;

pub fn time_from_cp24time2a(bytes: &[u8]) -> Result<PrimitiveDateTime, TimeConversionError> {
	let ms = u16::from_be_bytes([bytes[1], bytes[0]]);
	let min = bytes[2] & 0b0011_1111;
	let sec = (ms / 1000) as u8;
	let ms = ms % 1000;
	let now = OffsetDateTime::now_local().context(LocalTimezoneUnknown)?;
	Ok(PrimitiveDateTime::new(
		now.date(),
		now.time()
			.replace_millisecond(ms)
			.context(InvalidMillisecond)?
			.replace_second(sec)
			.context(InvalidSecond)?
			.replace_minute(min)
			.context(InvalidMinute)?,
	))
}

pub fn time_from_cp56time2a(bytes: &[u8]) -> Result<PrimitiveDateTime, TimeConversionError> {
	let ms = u16::from_be_bytes([bytes[1], bytes[0]]);
	let min = bytes[2] & 0b0011_1111;
	let hour = bytes[3] & 0b0001_1111;
	let day = bytes[4] & 0b0001_1111;
	let month = bytes[5] & 0b0000_1111;
	let year: i32 = (bytes[6] & 0b0111_1111).into();

	// TODO: Handle summer time and IV
	let _summer_time = bytes[3] & 0b1000_0000 != 0;
	let _iv = bytes[2] & 0b1000_0000 != 0;

	let sec = (ms / 1000) as u8;
	let ms = ms % 1000;

	let date =
		Date::from_calendar_date(2000 + year, Month::try_from(month).context(InvalidMonth)?, day)
			.context(InvalidDate)?;

	let time = Time::from_hms_milli(hour, min, sec, ms).context(InvalidTime)?;

	Ok(PrimitiveDateTime::new(date, time))
}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub), context(suffix(false)))]
pub enum TimeConversionError {
	#[snafu(display("Invalid millisecond"))]
	InvalidMillisecond {
		source: time::error::ComponentRange,
		#[snafu(implicit)]
		context: SpanTraceWrapper,
	},
	#[snafu(display("Invalid second"))]
	InvalidSecond {
		source: time::error::ComponentRange,
		#[snafu(implicit)]
		context: SpanTraceWrapper,
	},
	#[snafu(display("Invalid minute"))]
	InvalidMinute {
		source: time::error::ComponentRange,
		#[snafu(implicit)]
		context: SpanTraceWrapper,
	},
	#[snafu(display("Invalid hour"))]
	InvalidHour {
		source: time::error::ComponentRange,
		#[snafu(implicit)]
		context: SpanTraceWrapper,
	},
	#[snafu(display("Invalid day"))]
	InvalidDay {
		source: time::error::ComponentRange,
		#[snafu(implicit)]
		context: SpanTraceWrapper,
	},
	#[snafu(display("Invalid month"))]
	InvalidMonth {
		source: time::error::ComponentRange,
		#[snafu(implicit)]
		context: SpanTraceWrapper,
	},
	#[snafu(display("Invalid year"))]
	InvalidYear {
		source: time::error::ComponentRange,
		#[snafu(implicit)]
		context: SpanTraceWrapper,
	},
	#[snafu(display("Invalid date"))]
	InvalidDate {
		source: time::error::ComponentRange,
		#[snafu(implicit)]
		context: SpanTraceWrapper,
	},
	#[snafu(display("Invalid time"))]
	InvalidTime {
		source: time::error::ComponentRange,
		#[snafu(implicit)]
		context: SpanTraceWrapper,
	},
	#[snafu(display("Local timezone unknown"))]
	LocalTimezoneUnknown {
		source: time::error::IndeterminateOffset,
		#[snafu(implicit)]
		context: SpanTraceWrapper,
	},
}
