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
	pub fn from_bytes(bytes: &[u8; 3]) -> Result<Self, ParseTimeError> {
		let ms = u16::from_be_bytes([bytes[1], bytes[0]]);
		let min = bytes[2] & 0b0011_1111;
		let iv = bytes[2] & 0b1000_0000 != 0;
		if ms > 59999 {
			return MillisecondsError.fail();
		}
		if min > 59 {
			return MinutesError.fail();
		}
		Ok(Self { ms, min, iv })
	}

	#[instrument]
	pub fn to_bytes(self) -> [u8; 3] {
		let mut bytes: [u8; 3] = [0, 0, 0];
		let ms_bytes = self.ms.to_le_bytes();
		bytes[0] = ms_bytes[0];
		bytes[1] = ms_bytes[1];
		bytes[2] = (self.min & 0b0011_1111) | (u8::from(self.iv) << 7);
		bytes
	}

	#[cfg(feature = "with-chrono")]
	#[instrument]
	pub fn from_chrono<Tz: chrono::TimeZone>(dt: chrono::DateTime<Tz>) -> Self {
		use chrono::Timelike;

		let mut ms = (dt.timestamp_subsec_millis() + dt.second() * 1000) as u16;
		// Workaround for leap seconds.
		if ms > 59999 {
			ms = 59999;
		}
		let iv = false;
		let min = dt.minute() as u8;
		return Self { ms, iv, min };
	}

	#[cfg(feature = "with-chrono")]
	#[instrument]
	pub fn to_chrono_local(&self) -> Result<chrono::DateTime<chrono::Local>, ParseTimeError> {
		use chrono::Timelike;

		let seconds = self.ms / 1000;
		let ms = self.ms % 1000;
		let t = chrono::Local::now()
			.with_minute(u32::from(self.min))
			.ok_or_else(|| MinutesError.build())?
			.with_second(seconds.into())
			.ok_or_else(|| SecondsError.build())?
			.with_nanosecond(u32::from(ms) * 1_000_000)
			.ok_or_else(|| NanosecondsError.build())?;
		return Ok(t);
	}

	#[cfg(feature = "with-chrono")]
	#[instrument]
	pub fn to_chrono_utc(&self) -> Result<chrono::DateTime<chrono::Utc>, ParseTimeError> {
		use chrono::Timelike;

		let seconds = self.ms / 1000;
		let ms = self.ms % 1000;
		let t = chrono::Utc::now()
			.with_minute(u32::from(self.min))
			.ok_or_else(|| MinutesError.build())?
			.with_second(seconds.into())
			.ok_or_else(|| SecondsError.build())?
			.with_nanosecond(u32::from(ms) * 1_000_000)
			.ok_or_else(|| NanosecondsError.build())?;
		return Ok(t);
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
	pub fn from_bytes(bytes: [u8; 2]) -> Result<Self, ParseTimeError> {
		let ms = u16::from_le_bytes(bytes);
		if ms > 59999 {
			return MillisecondsError.fail();
		}
		Ok(Self { ms })
	}

	#[instrument]
	pub fn to_bytes(self) -> [u8; 2] {
		self.ms.to_le_bytes()
	}

	#[instrument]
	pub fn to_duration(&self) -> tokio::time::Duration {
		tokio::time::Duration::from_millis(self.ms as u64)
	}

	#[instrument]
	pub fn from_duration(duration: &tokio::time::Duration) -> Result<Self, ParseTimeError> {
		let ms = duration.as_millis();
		if ms > 59999 {
			return MillisecondsError.fail();
		}
		Ok(Self { ms: ms as u16 })
	}
}

/// CP56Time2a time type
#[derive(Debug, Clone, Eq, PartialEq, Default)]
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
	pub fn from_bytes(bytes: &[u8; 7]) -> Result<Self, ParseTimeError> {
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
			return MillisecondsError.fail();
		}
		if min > 59 {
			return MinutesError.fail();
		}
		if hour > 23 {
			return HoursError.fail();
		}
		if day > 31 {
			return DaysError.fail();
		}
		if month > 12 {
			return MonthsError.fail();
		}
		if year > 99 {
			return YearsError.fail();
		}
		Ok(Self { ms, iv, min, summer_time, hour, weekday, day, month, year })
	}

	#[instrument]
	pub fn to_bytes(&self) -> [u8; 7] {
		let mut bytes: [u8; 7] = [0, 0, 0, 0, 0, 0, 0];
		let ms_bytes = self.ms.to_le_bytes();
		bytes[0] = ms_bytes[0];
		bytes[1] = ms_bytes[1];
		bytes[2] = (u8::from(self.iv) << 7) | (self.min & 0b0011_1111);
		bytes[3] = (u8::from(self.summer_time) << 7) | (self.hour & 0b0001_1111);
		bytes[4] = (self.weekday << 5) | (self.day & 0b0001_1111);
		bytes[5] = self.month & 0b0000_1111;
		bytes[6] = self.year & 0b0111_1111;
		bytes
	}

	#[cfg(feature = "with-chrono")]
	#[instrument]
	pub fn from_chrono_ignoring_dst<Tz: chrono::TimeZone>(
		dt: chrono::DateTime<Tz>,
	) -> Result<Self, ParseTimeError> {
		use chrono::{Datelike, Timelike};

		let mut ms = (dt.timestamp_subsec_millis() + dt.second() * 1000) as u16;
		// Workaround for leap seconds.
		if ms > 59999 {
			ms = 59999;
		}
		let iv = false;
		let min = dt.minute() as u8;
		let summer_time = false;
		let hour = dt.hour() as u8;
		let weekday = dt.weekday().num_days_from_sunday() as u8;
		let day = dt.day() as u8;
		let month = dt.month() as u8;
		let year = (dt.year() - 2000) as u8;
		return Ok(Self { ms, iv, min, summer_time, hour, weekday, day, month, year });
	}

	#[cfg(feature = "with-chrono")]
	#[instrument]
	pub fn to_chrono_local_ignoring_dst(
		&self,
	) -> Result<chrono::DateTime<chrono::Local>, ParseTimeError> {
		use chrono::{Datelike, Timelike};

		let seconds = self.ms / 1000;
		let ms = self.ms % 1000;
		let t = chrono::Local::now()
			.with_minute(u32::from(self.min))
			.ok_or_else(|| MillisecondsError.build())?
			.with_second(seconds.into())
			.ok_or_else(|| SecondsError.build())?
			.with_nanosecond(u32::from(ms) * 1_000_000)
			.ok_or_else(|| NanosecondsError.build())?
			.with_hour(u32::from(self.hour))
			.ok_or_else(|| HoursError.build())?
			.with_day(u32::from(self.day))
			.ok_or_else(|| DaysError.build())?
			.with_month(u32::from(self.month))
			.ok_or_else(|| MonthsError.build())?
			.with_year(2000 + i32::from(self.year))
			.ok_or_else(|| YearsError.build())?;
		return Ok(t);
	}

	#[cfg(feature = "with-chrono")]
	#[instrument]
	pub fn to_chrono_utc(&self) -> Result<chrono::DateTime<chrono::Utc>, ParseTimeError> {
		use chrono::{Datelike, Timelike};

		let seconds = self.ms / 1000;
		let ms = self.ms % 1000;
		let t = chrono::Utc::now()
			.with_minute(u32::from(self.min))
			.ok_or_else(|| MillisecondsError.build())?
			.with_second(seconds.into())
			.ok_or_else(|| SecondsError.build())?
			.with_nanosecond(u32::from(ms) * 1_000_000)
			.ok_or_else(|| NanosecondsError.build())?
			.with_hour(u32::from(self.hour))
			.ok_or_else(|| HoursError.build())?
			.with_day(u32::from(self.day))
			.ok_or_else(|| DaysError.build())?
			.with_month(u32::from(self.month))
			.ok_or_else(|| MonthsError.build())?
			.with_year(2000 + i32::from(self.year))
			.ok_or_else(|| YearsError.build())?;
		return Ok(t);
	}
}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub), context(suffix(Error)))]
pub enum ParseTimeError {
	#[snafu(display("Nanoseconds out of range"))]
	Nanoseconds {
		#[snafu(implicit)]
		context: Box<SpanTraceWrapper>,
	},
	#[snafu(display("Milliseconds out of range"))]
	Milliseconds {
		#[snafu(implicit)]
		context: Box<SpanTraceWrapper>,
	},
	#[snafu(display("Seconds out of range"))]
	Seconds {
		#[snafu(implicit)]
		context: Box<SpanTraceWrapper>,
	},
	#[snafu(display("Minutes out of range"))]
	Minutes {
		#[snafu(implicit)]
		context: Box<SpanTraceWrapper>,
	},
	#[snafu(display("Hours out of range"))]
	Hours {
		#[snafu(implicit)]
		context: Box<SpanTraceWrapper>,
	},
	#[snafu(display("Days out of range"))]
	Days {
		#[snafu(implicit)]
		context: Box<SpanTraceWrapper>,
	},
	#[snafu(display("Months out of range"))]
	Months {
		#[snafu(implicit)]
		context: Box<SpanTraceWrapper>,
	},
	#[snafu(display("Years out of range"))]
	Years {
		#[snafu(implicit)]
		context: Box<SpanTraceWrapper>,
	},
}
