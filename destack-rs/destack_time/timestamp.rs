use std::convert::TryFrom;
use std::error::Error as StdError;
use std::fmt;
use std::ops::{Add, Sub};
use std::str::FromStr;

use crate::Duration;

/// Timestamp in unsigned 64-bit nanosecond precision since epoch (UTC)
/// range: [0, u64::MAX]
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Timestamp(pub u64);

/// errors for parsing `Timestamp`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimestampParseError {
    TooShort,
    MissingT,
    MissingTimezone,
    InvalidTime,
    InvalidDate,
    InvalidOffset,
    TimeOutOfRange,
    FractionDigitsMustBe9,
    BeforeEpoch,
}

impl fmt::Display for TimestampParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = match self {
            TimestampParseError::TooShort => "too short",
            TimestampParseError::MissingT => "missing 'T'",
            TimestampParseError::MissingTimezone => "missing timezone",
            TimestampParseError::InvalidTime => "invalid time",
            TimestampParseError::InvalidDate => "invalid date",
            TimestampParseError::InvalidOffset => "invalid offset",
            TimestampParseError::TimeOutOfRange => "time out of range",
            TimestampParseError::FractionDigitsMustBe9 => "nanoseconds must be 9 digits",
            TimestampParseError::BeforeEpoch => "timestamp before epoch",
        };
        f.write_str(msg)
    }
}

impl StdError for TimestampParseError {}

impl Timestamp {
    /// Get the current Timestamp (nanoseconds since epoch).
    pub fn now() -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("SystemTimestamp before UNIX_EPOCH");
        let ns = now.as_nanos();
        Self(ns as u64)
    }

    #[inline]
    /// Split the time and timezone parts of a timestamp string.
    fn split_time_and_tz(ts_str: &str) -> Result<(&str, &str), TimestampParseError> {
        if let Some(zpos) = ts_str.rfind('Z') {
            Ok((&ts_str[..zpos], &ts_str[zpos..]))
        } else if let Some(pos) = ts_str.rfind(|c| c == '+' || c == '-') {
            Ok((&ts_str[..pos], &ts_str[pos..]))
        } else {
            Err(TimestampParseError::MissingTimezone)
        }
    }

    #[inline]
    fn parse_two_digits(dt_bytes: &[u8], idx: usize) -> Result<u64, TimestampParseError> {
        if idx + 1 >= dt_bytes.len()
            || !dt_bytes[idx].is_ascii_digit()
            || !dt_bytes[idx + 1].is_ascii_digit()
        {
            return Err(TimestampParseError::InvalidTime);
        }
        Ok(((dt_bytes[idx] - b'0') as u64) * 10 + (dt_bytes[idx + 1] - b'0') as u64)
    }

    #[inline]
    fn parse_time_ns(time_part: &str) -> Result<(u64, u64, u64, u64), TimestampParseError> {
        let tb = time_part.as_bytes();
        if tb.len() < 8 {
            return Err(TimestampParseError::InvalidTime);
        }
        // hh
        let hh = Self::parse_two_digits(tb, 0)?;
        if tb.get(2) != Some(&b':') {
            return Err(TimestampParseError::InvalidTime);
        }
        // mm
        let mm = Self::parse_two_digits(tb, 3)?;
        if tb.get(5) != Some(&b':') {
            return Err(TimestampParseError::InvalidTime);
        }
        // ss
        let ss = Self::parse_two_digits(tb, 6)?;
        if hh >= 24 || mm >= 60 || ss >= 60 {
            return Err(TimestampParseError::TimeOutOfRange);
        }
        // ns
        let mut ns: u64 = 0;
        if tb.len() > 8 {
            if tb.get(8) != Some(&b'.') {
                return Err(TimestampParseError::InvalidTime);
            }
            // nanoseconds must be 9 digits
            if tb.len() != 18 {
                return Err(TimestampParseError::FractionDigitsMustBe9);
            }
            // parse nanoseconds
            for &ch in &tb[9..18] {
                if !ch.is_ascii_digit() {
                    return Err(TimestampParseError::InvalidTime);
                }
                ns = ns * 10 + (ch - b'0') as u64;
            }
        }
        Ok((hh, mm, ss, ns))
    }

    #[inline]
    fn parse_offset_ns(tz_part: &str) -> Result<i128, TimestampParseError> {
        // Z = no offset
        if tz_part == "Z" {
            return Ok(0);
        }
        // offset is ±HH:MM
        let tz_bytes = tz_part.as_bytes();
        if tz_bytes.len() != 6
            || (tz_bytes[0] != b'+' && tz_bytes[0] != b'-')
            || tz_bytes[3] != b':'
        {
            return Err(TimestampParseError::InvalidOffset);
        }
        // parse HH and MM
        let sign = if tz_bytes[0] == b'-' { -1i128 } else { 1 };
        let hh = ((tz_bytes[1] - b'0') as i128) * 10 + (tz_bytes[2] - b'0') as i128;
        let mm = ((tz_bytes[4] - b'0') as i128) * 10 + (tz_bytes[5] - b'0') as i128;
        Ok(sign * (hh * 3_600 + mm * 60) * 1_000_000_000)
    }
}

//
// Duration operators
//

impl Add<Duration> for Timestamp {
    type Output = Timestamp;

    fn add(self, rhs: Duration) -> Timestamp {
        if rhs.0 < 0 {
            let sub = (-rhs.0) as u64;
            Timestamp(self.0.saturating_sub(sub))
        } else {
            Timestamp(self.0.saturating_add(rhs.0 as u64))
        }
    }
}

impl Sub<Duration> for Timestamp {
    type Output = Timestamp;

    fn sub(self, rhs: Duration) -> Timestamp {
        if rhs.0 < 0 {
            Timestamp(self.0.saturating_add((-rhs.0) as u64))
        } else {
            Timestamp(self.0.saturating_sub(rhs.0 as u64))
        }
    }
}

impl Sub<Timestamp> for Timestamp {
    type Output = Duration;

    fn sub(self, rhs: Timestamp) -> Duration {
        Duration(self.0 as i64 - rhs.0 as i64)
    }
}

//
// Conversions
//

impl From<Timestamp> for u64 {
    fn from(value: Timestamp) -> Self {
        value.0
    }
}

impl TryFrom<u64> for Timestamp {
    type Error = TimestampParseError;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        Ok(Timestamp(value))
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // seconds and nanoseconds since epoch
        let secs = self.0 / 1_000_000_000;
        let ns = self.0 % 1_000_000_000;
        // derive date from days since epoch
        let days = (secs / 86_400) as i64;
        let date = crate::Date(days);
        let sod = secs % 86_400;
        let h = sod / 3600;
        let m = (sod % 3600) / 60;
        let s = sod % 60;
        write!(f, "{}T{:02}:{:02}:{:02}.{:09}Z", date, h, m, s, ns)
    }
}

impl FromStr for Timestamp {
    type Err = TimestampParseError;

    // Accept: YYYY-MM-DDTHH:MM:SS.fffffffffZ or with explicit offset ±HH:MM to convert to UTC
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() < 20 {
            return Err(TimestampParseError::TooShort);
        }
        let (date_part, rest) = s.split_at(10);
        if &rest[0..1] != "T" {
            return Err(TimestampParseError::MissingT);
        }
        let (time_part, tz_part) = Self::split_time_and_tz(&s[11..])?;
        let date: crate::Date = date_part
            .parse()
            .map_err(|_| TimestampParseError::InvalidDate)?;
        // parse time with 9 fractional digits
        let (hh, mm, ss, ns) = Self::parse_time_ns(time_part)?;
        let total_ns: u128 = (date.0 as i128).max(0) as u128 * 86_400 * 1_000_000_000
            + (hh as u128) * 3_600 * 1_000_000_000
            + (mm as u128) * 60 * 1_000_000_000
            + (ss as u128) * 1_000_000_000
            + ns as u128;
        if date.0 < 0 {
            return Err(TimestampParseError::BeforeEpoch);
        }
        let offset_ns = Self::parse_offset_ns(tz_part)?;
        let adj = (total_ns as i128) - offset_ns;
        if adj < 0 {
            return Err(TimestampParseError::BeforeEpoch);
        }
        Ok(Timestamp(adj as u64))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_sub_and_diff() {
        let t = Timestamp::now();
        let t2 = t + Duration::from_seconds(1);
        assert_eq!(t2 - t, Duration::from_seconds(1));
    }

    #[test]
    fn parse_and_format() {
        let s = "2025-08-12T15:34:56.123456789Z";
        let ts: Timestamp = s.parse().unwrap();
        assert_eq!(ts.to_string(), s);
    }
}
