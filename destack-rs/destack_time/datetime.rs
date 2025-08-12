use std::fmt;
use std::ops::{Add, Sub};
use std::str::FromStr;
use std::time::SystemTime;

use crate::parser::{
    ParseError, parse_hh_mm_ss, parse_tz_offset, parse_us_maybe, split_time_and_tz,
};
use crate::{Date, Duration};

/// DateTime in signed 64-bit microsecond precision since epoch (UTC)
/// range: ±292,277 years
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DateTime(pub i64);

impl DateTime {
    /// Get the current datetime in UTC
    pub fn now() -> Self {
        Self(
            SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_else(|_| panic!("SystemTime before UNIX_EPOCH"))
                .as_micros() as i64,
        )
    }

    /// Format as ISO 8601 datetime (YYYY-MM-DDTHH:MM:SS.ffffffZ).
    pub fn to_iso(&self) -> String {
        // microseconds since epoch (UTC)
        // compute days, seconds within day, and micros remainder carefully with floor division
        let micros = self.0;
        let days: i64;
        let seconds: i64;
        if micros >= 0 {
            days = (micros / 1_000_000) / 86_400;
            seconds = (micros / 1_000_000) - days * 86_400;
        } else {
            // floor division behavior for negatives
            days = ((micros - 1) / 1_000_000 - 86_400 + 1) / 86_400;
            seconds = (micros / 1_000_000) - days * 86_400;
        }
        let us = (micros - (days * 86_400 + seconds) * 1_000_000).abs();
        let date = Date(days);
        let h = seconds / 3600;
        let m = (seconds % 3600) / 60;
        let s = seconds % 60;
        format!("{date}T{h:02}:{m:02}:{s:02}.{us:06}Z")
    }

    /// Parse ISO 8601 datetime format (YYYY-MM-DDTHH:MM:SS.ffffffZ or with offset).
    pub fn from_iso(s: &str) -> Result<Self, ParseError> {
        // split date and time
        if s.len() < 20 {
            return Err(ParseError::TooShort);
        }
        let (date_part, rest) = s.split_at(10);
        if &rest[0..1] != "T" {
            return Err(ParseError::MissingT);
        }
        let (time_part, tz_part) = split_time_and_tz(&s[11..])?;
        // parse date
        let date: Date = date_part.parse::<Date>()?;
        // parse time with 6 fractional digits
        let (hh, mm, ss, us) = Self::_parse_time_us(time_part)?;
        let mut total_ns = (date.0 as i128) * 86_400 * 1_000_000_000i128
            + (hh as i128) * 3_600 * 1_000_000_000
            + (mm as i128) * 60 * 1_000_000_000
            + (ss as i128) * 1_000_000_000
            + (us as i128) * 1_000; // micro to nano
        // handle tz
        let offset_ns = parse_tz_offset(tz_part)?;
        total_ns -= offset_ns; // convert to UTC
        let micros = total_ns / 1_000;
        Ok(DateTime(micros as i64))
    }

    #[inline]
    /// Parse the time part of a datetime string into (hh, mm, ss, us)
    fn _parse_time_us(dt_str: &str) -> Result<(i64, i64, i64, i64), ParseError> {
        let dt_bytes = dt_str.as_bytes();
        let (hh, mm, ss) = parse_hh_mm_ss(dt_bytes)?;
        let us = if dt_bytes.len() > 8 {
            let tail = &dt_bytes[8..];
            match parse_us_maybe(tail) {
                Ok(Some(v)) => v,
                Ok(None) => 0,
                Err(e) => return Err(e),
            }
        } else {
            0
        };
        Ok((hh, mm, ss, us))
    }
}

impl Add<Duration> for DateTime {
    fn add(self, rhs: Duration) -> DateTime {
        DateTime(self.0 + rhs.0 / 1_000)
    }
    type Output = DateTime;
}

impl Sub<Duration> for DateTime {
    type Output = DateTime;

    fn sub(self, rhs: Duration) -> DateTime {
        DateTime(self.0 - rhs.0 / 1_000)
    }
}

impl Sub<DateTime> for DateTime {
    type Output = Duration;

    fn sub(self, rhs: DateTime) -> Duration {
        // convert microsecond difference to nanoseconds
        Duration((self.0 - rhs.0).saturating_mul(1_000))
    }
}

//
// Conversions
//

impl From<i64> for DateTime {
    fn from(value: i64) -> Self {
        DateTime(value)
    }
}

impl From<DateTime> for i64 {
    fn from(value: DateTime) -> Self {
        value.0
    }
}

impl From<std::time::SystemTime> for DateTime {
    fn from(value: std::time::SystemTime) -> Self {
        let dur = match value.duration_since(std::time::UNIX_EPOCH) {
            Ok(d) => d,
            Err(e) => e.duration(),
        };
        let micros = dur.as_micros() as i64;
        if value >= std::time::UNIX_EPOCH {
            DateTime(micros)
        } else {
            DateTime(-micros)
        }
    }
}

impl From<DateTime> for std::time::SystemTime {
    fn from(value: DateTime) -> Self {
        if value.0 >= 0 {
            std::time::UNIX_EPOCH + std::time::Duration::from_micros(value.0 as u64)
        } else {
            std::time::UNIX_EPOCH - std::time::Duration::from_micros((-value.0) as u64)
        }
    }
}

impl fmt::Display for DateTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_iso())
    }
}

impl FromStr for DateTime {
    type Err = ParseError;

    /// Parse a datetime string in the format YYYY-MM-DDTHH:MM:SS.ffffffZ.
    ///  (or with explicit offset ±HH:MM to convert to UTC).
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_iso(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// Parse and format a datetime string in UTC.
    fn test_parse_format_datetime_utc() {
        let datetime_str = "2025-08-12T15:34:56.123456Z";
        let datetime: DateTime = datetime_str.parse().unwrap();
        let parsed_datetime = datetime.to_string();
        assert!(parsed_datetime.starts_with("2025-08-12T15:34:56.123456"));
        assert!(parsed_datetime.ends_with('Z'));
    }

    #[test]
    /// Parse a datetime string with an offset.
    fn test_parse_datetime_with_offset() {
        let datetime_str = "2025-08-12T17:34:56.123456+02:00"; // equals 15:34:56.123456Z
        let datetime: DateTime = datetime_str.parse().unwrap();
        assert!(datetime.to_string().contains("15:34:56.123456"));
    }

    #[test]
    /// Add and subtract durations from a datetime.
    fn test_add_sub_datetime() {
        let datetime = DateTime::now();
        let later = datetime + Duration::from_millis(1_500);
        let diff = later - datetime;
        assert!(diff.as_nanos() >= 1_500_000_000);
        let back = later - Duration::from_millis(1_500);
        assert_eq!(back.0, datetime.0);
    }

    #[test]
    /// Roundtrip a datetime from and to a SystemTime.
    fn test_system_time_datetime() {
        let system_time = std::time::SystemTime::now();
        let datetime: DateTime = system_time.into();
        let system_time2: std::time::SystemTime = datetime.into();
        // allow some drift but they should be close (microsecond precision)
        let delta = system_time2
            .duration_since(system_time)
            .unwrap_or_else(|e| e.duration());
        assert!(delta.as_millis() < 10);
    }
}
