use std::error::Error as StdError;
use std::fmt;
use std::ops::{Add, Sub};
use std::str::FromStr;

use crate::Duration;

/// DateTime in signed 64-bit microsecond precision since epoch (UTC)
/// range: ±292,277 years
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DateTime(pub i64);

/// errors for parsing `DateTime`
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DateTimeParseError {
    TooShort,
    MissingT,
    MissingTimezone,
    InvalidTime,
    InvalidDate(crate::date::DateParseError),
    InvalidOffset,
    TimeOutOfRange,
    FractionDigitsMustBe6,
}

impl fmt::Display for DateTimeParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DateTimeParseError::TooShort => f.write_str("too short"),
            DateTimeParseError::MissingT => f.write_str("missing 'T'"),
            DateTimeParseError::MissingTimezone => f.write_str("missing timezone"),
            DateTimeParseError::InvalidTime => f.write_str("invalid time"),
            DateTimeParseError::InvalidDate(_) => f.write_str("invalid date"),
            DateTimeParseError::InvalidOffset => f.write_str("invalid offset"),
            DateTimeParseError::TimeOutOfRange => f.write_str("time out of range"),
            DateTimeParseError::FractionDigitsMustBe6 => {
                f.write_str("microseconds must be 6 digits")
            }
        }
    }
}

impl StdError for DateTimeParseError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            DateTimeParseError::InvalidDate(e) => Some(e),
            _ => None,
        }
    }
}

impl DateTime {
    /// Get the current datetime in UTC
    pub fn now() -> Self {
        Self(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("SystemTime before UNIX_EPOCH")
                .as_micros() as i64,
        )
    }

    #[inline]
    /// Split the time and timezone parts of a datetime string.
    fn split_time_and_tz(dt_str: &str) -> Result<(&str, &str), DateTimeParseError> {
        if let Some(zpos) = dt_str.rfind('Z') {
            Ok((&dt_str[..zpos], &dt_str[zpos..]))
        } else if let Some(pos) = dt_str.rfind(|c| c == '+' || c == '-') {
            Ok((&dt_str[..pos], &dt_str[pos..]))
        } else {
            Err(DateTimeParseError::MissingTimezone)
        }
    }

    #[inline]
    /// Parse two digits from a byte slice at the given index.
    fn parse_two_digits(dt_bytes: &[u8], idx: usize) -> Result<i64, DateTimeParseError> {
        if idx + 1 >= dt_bytes.len()
            || !dt_bytes[idx].is_ascii_digit()
            || !dt_bytes[idx + 1].is_ascii_digit()
        {
            return Err(DateTimeParseError::InvalidTime);
        }
        Ok(((dt_bytes[idx] - b'0') as i64) * 10 + (dt_bytes[idx + 1] - b'0') as i64)
    }

    #[inline]
    /// Parse the time part of a datetime string.
    fn parse_time_us(dt_str: &str) -> Result<(i64, i64, i64, i64), DateTimeParseError> {
        let dt_bytes = dt_str.as_bytes();
        if dt_bytes.len() < 8 {
            return Err(DateTimeParseError::InvalidTime);
        }
        // hh
        let hh = Self::parse_two_digits(dt_bytes, 0)?;
        if dt_bytes.get(2) != Some(&b':') {
            return Err(DateTimeParseError::InvalidTime);
        }
        // mm
        let mm = Self::parse_two_digits(dt_bytes, 3)?;
        if dt_bytes.get(5) != Some(&b':') {
            return Err(DateTimeParseError::InvalidTime);
        }
        // ss
        let ss = Self::parse_two_digits(dt_bytes, 6)?;
        if hh >= 24 || mm >= 60 || ss >= 60 {
            return Err(DateTimeParseError::TimeOutOfRange);
        }
        
        // us
        let mut us: i64 = 0;
        if dt_bytes.len() > 8 {
            if dt_bytes.get(8) != Some(&b'.') {
                return Err(DateTimeParseError::InvalidTime);
            }
            // microseconds must be 6 digits
            if dt_bytes.len() != 15 {
                return Err(DateTimeParseError::FractionDigitsMustBe6);
            }
            // parse microseconds
            for &ch in &dt_bytes[9..15] {
                if !ch.is_ascii_digit() {
                    return Err(DateTimeParseError::InvalidTime);
                }
                us = us * 10 + (ch - b'0') as i64;
            }
        }
        Ok((hh, mm, ss, us))
    }

    #[inline]
    /// Parse the timezone part of a datetime string.
    fn parse_offset_ns(tz_part: &str) -> Result<i128, DateTimeParseError> {
        // Z = no offset
        if tz_part == "Z" {
            return Ok(0);
        }
        // offset is ±HH:MM
        let tz_bytes = tz_part.as_bytes();
        if tz_bytes.len() != 6 || (tz_bytes[0] != b'+' && tz_bytes[0] != b'-') || tz_bytes[3] != b':' {
            return Err(DateTimeParseError::InvalidOffset);
        }
        // parse HH and MM
        let sign = if tz_bytes[0] == b'-' { -1i128 } else { 1 };
        let hh = ((tz_bytes[1] - b'0') as i128) * 10 + (tz_bytes[2] - b'0') as i128;
        let mm = ((tz_bytes[4] - b'0') as i128) * 10 + (tz_bytes[5] - b'0') as i128;
        Ok(sign * (hh * 3_600 + mm * 60) * 1_000_000_000)
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
    /// Format: YYYY-MM-DDTHH:MM:SS.ffffffZ.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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
        let date = crate::Date(days);
        let h = seconds / 3600;
        let m = (seconds % 3600) / 60;
        let s = seconds % 60;
        write!(f, "{}T{:02}:{:02}:{:02}.{:06}Z", date, h, m, s, us)
    }
}

impl FromStr for DateTime {
    type Err = DateTimeParseError;

    /// Parse a datetime string in the format YYYY-MM-DDTHH:MM:SS.ffffffZ.
    ///  (or with explicit offset ±HH:MM to convert to UTC).
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // split date and time
        if s.len() < 20 {
            return Err(DateTimeParseError::TooShort);
        }
        let (date_part, rest) = s.split_at(10);
        if &rest[0..1] != "T" {
            return Err(DateTimeParseError::MissingT);
        }
        let (time_part, tz_part) = Self::split_time_and_tz(&s[11..])?;
        // parse date
        let date: crate::Date = date_part.parse().map_err(DateTimeParseError::InvalidDate)?;
        // parse time with 6 fractional digits
        let (hh, mm, ss, us) = Self::parse_time_us(time_part)?;
        let mut total_ns = (date.0 as i128) * 86_400 * 1_000_000_000i128
            + (hh as i128) * 3_600 * 1_000_000_000
            + (mm as i128) * 60 * 1_000_000_000
            + (ss as i128) * 1_000_000_000
            + (us as i128) * 1_000; // micro to nano
        // handle tz
        let offset_ns = Self::parse_offset_ns(tz_part)?;
        total_ns -= offset_ns; // convert to UTC
        let micros = total_ns / 1_000;
        Ok(DateTime(micros as i64))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// Parse and format a datetime string in UTC.
    fn test_parse_format_datetime_utc() {
        let s = "2025-08-12T15:34:56.123456Z";
        let dt: DateTime = s.parse().unwrap();
        // Format back (Display) yields same shape (microseconds)
        let out = dt.to_string();
        // We only guarantee roundtrip via FromStr/Display pair for microseconds
        assert!(out.starts_with("2025-08-12T15:34:56.123456"));
        assert!(out.ends_with('Z'));
    }

    #[test]
    /// Parse a datetime string with an offset.
    fn test_parse_with_offset() {
        let s = "2025-08-12T17:34:56.123456+02:00"; // equals 15:34:56.123456Z
        let dt: DateTime = s.parse().unwrap();
        assert!(dt.to_string().contains("15:34:56.123456"));
    }

    #[test]
    /// Add and subtract durations from a datetime.
    fn test_add_sub_and_diff() {
        let dt = DateTime::now();
        let later = dt + Duration::from_millis(1_500);
        let diff = later - dt;
        assert!(diff.as_nanos() >= 1_500_000_000);
        let back = later - Duration::from_millis(1_500);
        assert_eq!(back.0, dt.0);
    }

    #[test]
    /// Roundtrip a datetime from and to a SystemTime.
    fn test_system_time_roundtrip() {
        let st = std::time::SystemTime::now();
        let dt: DateTime = st.into();
        let st2: std::time::SystemTime = dt.into();
        // allow some drift but they should be close (microsecond precision)
        let delta = st2.duration_since(st).unwrap_or_else(|e| e.duration());
        assert!(delta.as_millis() < 10);
    }
}
