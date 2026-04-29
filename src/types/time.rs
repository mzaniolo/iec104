//! CPXXTime2a time types.
//!
//! Note: conversions for Daylight Saving Time ( DST ) are **NOT** supported
//! yet! The `summer_time` flag on [`Cp56Time2a`] is preserved as an
//! informational field but does not influence time conversions.

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
}

#[cfg(feature = "chrono")]
impl<Tz: chrono::TimeZone> From<&chrono::DateTime<Tz>> for Cp24Time2a {
	fn from(dt: &chrono::DateTime<Tz>) -> Self {
		use chrono::Timelike;

		let mut ms = (dt.timestamp_subsec_millis() + dt.second() * 1000) as u16;
		// Workaround for leap seconds.
		if ms > 59999 {
			ms = 59999;
		}
		Self { ms, iv: false, min: dt.minute() as u8 }
	}
}

#[cfg(feature = "chrono")]
impl TryFrom<Cp24Time2a> for chrono::NaiveTime {
	type Error = ParseTimeError;

	/// The hour is not encoded in [`Cp24Time2a`] and is taken from the current
	/// local time at the moment of conversion.
	fn try_from(val: Cp24Time2a) -> Result<Self, Self::Error> {
		use chrono::Timelike;

		if val.iv {
			return InvalidError.fail();
		}
		let seconds = (val.ms / 1000) as u8;
		let ms = val.ms % 1000;
		let hour = chrono::Local::now().hour();
		chrono::NaiveTime::from_hms_milli_opt(
			hour,
			u32::from(val.min),
			u32::from(seconds),
			u32::from(ms),
		)
		.ok_or_else(|| MinutesError.build())
	}
}

#[cfg(feature = "time")]
impl From<&time::OffsetDateTime> for Cp24Time2a {
	fn from(dt: &time::OffsetDateTime) -> Self {
		let mut ms = dt.millisecond() + u16::from(dt.second()) * 1000;
		// Workaround for leap seconds.
		if ms > 59999 {
			ms = 59999;
		}
		Self { ms, min: dt.minute(), iv: false }
	}
}

#[cfg(feature = "time")]
impl TryFrom<Cp24Time2a> for time::Time {
	type Error = ParseTimeError;

	/// The hour is not encoded in [`Cp24Time2a`] and is taken from the current
	/// UTC time at the moment of conversion.
	fn try_from(val: Cp24Time2a) -> Result<Self, Self::Error> {
		if val.iv {
			return InvalidError.fail();
		}
		let seconds = (val.ms / 1000) as u8;
		let ms = val.ms % 1000;
		let hour = time::OffsetDateTime::now_utc().hour();
		time::Time::from_hms_milli(hour, val.min, seconds, ms).map_err(|_| MinutesError.build())
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
}

impl TryFrom<&tokio::time::Duration> for Cp16Time2a {
	type Error = ParseTimeError;

	fn try_from(duration: &tokio::time::Duration) -> Result<Self, Self::Error> {
		let ms = duration.as_millis();
		if ms > 59999 {
			return MillisecondsError.fail();
		}
		Ok(Self { ms: ms as u16 })
	}
}

impl From<Cp16Time2a> for tokio::time::Duration {
	fn from(val: Cp16Time2a) -> Self {
		tokio::time::Duration::from_millis(u64::from(val.ms))
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
	/// Summer time (DST) flag — informational only, does not affect conversions
	pub summer_time: bool,
	/// Hours (0-23)
	pub hour: u8,
	/// Weekday
	pub weekday: u8,
	/// Day of month (1-31)
	pub day: u8,
	/// Month (1-12)
	pub month: u8,
	/// Year (0-99, relative to 2000)
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

	/// Convert to a timezone-aware datetime using the provided timezone or UTC
	/// offset.
	///
	/// The `tz` argument accepts any [`chrono::TimeZone`] (e.g.
	/// `chrono::Utc`, `chrono::Local`, or a `chrono_tz` zone) when the
	/// `chrono` feature is enabled, and [`time::UtcOffset`] (e.g.
	/// `time::UtcOffset::UTC`) when the `time` feature is enabled.
	///
	/// Returns an error if the [`Cp56Time2a::iv`] flag is set or if the
	/// stored fields do not form a valid calendar date/time.
	///
	/// # Example — chrono
	/// ```ignore
	/// let dt = cp56.to_datetime(chrono::Utc)?;
	/// let dt = cp56.to_datetime(chrono::Local)?;
	/// ```
	///
	/// # Example — time
	/// ```ignore
	/// let dt = cp56.to_datetime(time::UtcOffset::UTC)?;
	/// let dt = cp56.to_datetime(time::UtcOffset::from_hms(1, 0, 0)?)?;
	/// ```
	pub fn to_datetime<Tz: IecTimezone>(&self, tz: Tz) -> Result<Tz::DateTime, ParseTimeError> {
		if self.iv {
			return InvalidError.fail();
		}
		let seconds = (self.ms / 1000) as u8;
		let ms = self.ms % 1000;
		tz.make_datetime(
			2000 + i32::from(self.year),
			self.month,
			self.day,
			self.hour,
			self.min,
			seconds,
			ms,
		)
	}
}

#[cfg(feature = "chrono")]
impl<Tz: chrono::TimeZone> TryFrom<&chrono::DateTime<Tz>> for Cp56Time2a {
	type Error = ParseTimeError;

	/// Note: Daylight Saving Time (DST) is **NOT** considered. The
	/// `summer_time` field is always set to `false`.
	///
	/// Returns [`ParseTimeError::Years`] if the year is outside 2000–2099.
	fn try_from(dt: &chrono::DateTime<Tz>) -> Result<Self, Self::Error> {
		use chrono::{Datelike, Timelike};

		if dt.year() < 2000 || dt.year() > 2099 {
			return YearsError.fail();
		}
		let mut ms = (dt.timestamp_subsec_millis() + dt.second() * 1000) as u16;
		// Workaround for leap seconds.
		if ms > 59999 {
			ms = 59999;
		}
		Ok(Self {
			ms,
			iv: false,
			min: dt.minute() as u8,
			summer_time: false,
			hour: dt.hour() as u8,
			weekday: dt.weekday().num_days_from_sunday() as u8,
			day: dt.day() as u8,
			month: dt.month() as u8,
			year: (dt.year() - 2000) as u8,
		})
	}
}

#[cfg(feature = "chrono")]
impl TryFrom<Cp56Time2a> for chrono::NaiveDateTime {
	type Error = ParseTimeError;

	fn try_from(val: Cp56Time2a) -> Result<Self, Self::Error> {
		if val.iv {
			return InvalidError.fail();
		}
		let seconds = (val.ms / 1000) as u8;
		let ms = val.ms % 1000;
		chrono::NaiveDate::from_ymd_opt(
			2000 + i32::from(val.year),
			u32::from(val.month),
			u32::from(val.day),
		)
		.ok_or_else(|| DaysError.build())?
		.and_hms_milli_opt(
			u32::from(val.hour),
			u32::from(val.min),
			u32::from(seconds),
			u32::from(ms),
		)
		.ok_or_else(|| HoursError.build())
	}
}

#[cfg(feature = "time")]
impl TryFrom<&time::OffsetDateTime> for Cp56Time2a {
	type Error = ParseTimeError;

	/// Note: Daylight Saving Time (DST) is **NOT** considered. The
	/// `summer_time` field is always set to `false`.
	///
	/// Returns [`ParseTimeError::Years`] if the year is outside 2000–2099.
	fn try_from(dt: &time::OffsetDateTime) -> Result<Self, Self::Error> {
		if dt.year() < 2000 || dt.year() > 2099 {
			return YearsError.fail();
		}
		let mut ms = dt.millisecond() + u16::from(dt.second()) * 1000;
		// Workaround for leap seconds.
		if ms > 59999 {
			ms = 59999;
		}
		Ok(Self {
			ms,
			iv: false,
			min: dt.minute(),
			summer_time: false,
			hour: dt.hour(),
			weekday: dt.weekday().number_days_from_sunday(),
			day: dt.day(),
			month: u8::from(dt.month()),
			year: (dt.year() - 2000) as u8,
		})
	}
}

#[cfg(feature = "time")]
impl TryFrom<Cp56Time2a> for time::PrimitiveDateTime {
	type Error = ParseTimeError;

	fn try_from(val: Cp56Time2a) -> Result<Self, Self::Error> {
		if val.iv {
			return InvalidError.fail();
		}
		let seconds = (val.ms / 1000) as u8;
		let ms = val.ms % 1000;
		let month = time::Month::try_from(val.month).map_err(|_| MonthsError.build())?;
		let date = time::Date::from_calendar_date(2000 + i32::from(val.year), month, val.day)
			.map_err(|_| DaysError.build())?;
		let t = time::Time::from_hms_milli(val.hour, val.min, seconds, ms)
			.map_err(|_| HoursError.build())?;
		Ok(time::PrimitiveDateTime::new(date, t))
	}
}

mod sealed {
	/// Sealing trait — prevents external implementations of
	/// [`super::IecTimezone`].
	pub trait Sealed {}

	#[cfg(feature = "chrono")]
	impl Sealed for chrono::Utc {}
	#[cfg(feature = "chrono")]
	impl Sealed for chrono::Local {}
	#[cfg(feature = "chrono")]
	impl Sealed for chrono::FixedOffset {}

	#[cfg(feature = "time")]
	impl Sealed for time::UtcOffset {}
}

#[allow(clippy::too_many_arguments)]
/// Timezone or UTC-offset type accepted by [`Cp56Time2a::to_datetime`].
///
/// Implemented for:
/// - [`chrono::Utc`], [`chrono::Local`], and [`chrono::FixedOffset`] when the
///   `chrono` feature is enabled.
/// - [`time::UtcOffset`] when the `time` feature is enabled.
///
/// This trait is sealed — it cannot be implemented outside this crate.
pub trait IecTimezone: sealed::Sealed {
	/// The resulting datetime type produced by the conversion.
	type DateTime;

	/// Build a timezone-aware datetime from raw IEC 104 field values.
	///
	/// `year` is the absolute year (e.g. `2024`), `ms` is the sub-second
	/// milliseconds (0–999).
	fn make_datetime(
		&self,
		year: i32,
		month: u8,
		day: u8,
		hour: u8,
		min: u8,
		second: u8,
		ms: u16,
	) -> Result<Self::DateTime, ParseTimeError>;
}

/// Shared implementation body for all concrete chrono timezone types.
#[cfg(feature = "chrono")]
fn chrono_make_datetime<Tz: chrono::TimeZone>(
	tz: &Tz,
	year: i32,
	month: u8,
	day: u8,
	hour: u8,
	min: u8,
	second: u8,
	ms: u16,
) -> Result<chrono::DateTime<Tz>, ParseTimeError> {
	use chrono::Timelike;

	tz.with_ymd_and_hms(
		year,
		u32::from(month),
		u32::from(day),
		u32::from(hour),
		u32::from(min),
		u32::from(second),
	)
	.earliest()
	.ok_or_else(|| DaysError.build())?
	.with_nanosecond(u32::from(ms) * 1_000_000)
	.ok_or_else(|| NanosecondsError.build())
}

#[cfg(feature = "chrono")]
impl IecTimezone for chrono::Utc {
	type DateTime = chrono::DateTime<chrono::Utc>;

	fn make_datetime(
		&self,
		year: i32,
		month: u8,
		day: u8,
		hour: u8,
		min: u8,
		second: u8,
		ms: u16,
	) -> Result<Self::DateTime, ParseTimeError> {
		chrono_make_datetime(self, year, month, day, hour, min, second, ms)
	}
}

#[cfg(feature = "chrono")]
impl IecTimezone for chrono::Local {
	type DateTime = chrono::DateTime<chrono::Local>;

	fn make_datetime(
		&self,
		year: i32,
		month: u8,
		day: u8,
		hour: u8,
		min: u8,
		second: u8,
		ms: u16,
	) -> Result<Self::DateTime, ParseTimeError> {
		chrono_make_datetime(self, year, month, day, hour, min, second, ms)
	}
}

#[cfg(feature = "chrono")]
impl IecTimezone for chrono::FixedOffset {
	type DateTime = chrono::DateTime<chrono::FixedOffset>;

	fn make_datetime(
		&self,
		year: i32,
		month: u8,
		day: u8,
		hour: u8,
		min: u8,
		second: u8,
		ms: u16,
	) -> Result<Self::DateTime, ParseTimeError> {
		chrono_make_datetime(self, year, month, day, hour, min, second, ms)
	}
}

#[cfg(feature = "time")]
impl IecTimezone for time::UtcOffset {
	type DateTime = time::OffsetDateTime;

	fn make_datetime(
		&self,
		year: i32,
		month: u8,
		day: u8,
		hour: u8,
		min: u8,
		second: u8,
		ms: u16,
	) -> Result<Self::DateTime, ParseTimeError> {
		let month = time::Month::try_from(month).map_err(|_| MonthsError.build())?;
		let date =
			time::Date::from_calendar_date(year, month, day).map_err(|_| DaysError.build())?;
		let t =
			time::Time::from_hms_milli(hour, min, second, ms).map_err(|_| HoursError.build())?;
		Ok(time::PrimitiveDateTime::new(date, t).assume_offset(*self))
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
	#[snafu(display("Year out of range (must be 2000–2099)"))]
	Years {
		#[snafu(implicit)]
		context: Box<SpanTraceWrapper>,
	},
	#[snafu(display("Invalid flag detected"))]
	Invalid {
		#[snafu(implicit)]
		context: Box<SpanTraceWrapper>,
	},
}
