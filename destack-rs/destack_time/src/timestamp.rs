//! Timestamp in unsigned 64-bit nanosecond precision since epoch (UTC).
//! Provide the `Timestamp` type and some conversions.

use std::convert::TryFrom;
use std::fmt;
use std::ops::{Add, Sub};
use std::str::FromStr;
use std::time::SystemTime;

use crate::parse::parse_hh_mm_ss;
use crate::{
    Date, Duration,
    format::write_hms_ns,
    parse::{TimeParseError, parse_tz_offset, split_time_and_tz},
};

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// Timestamp in unsigned 64-bit nanosecond precision since epoch (UTC).
///
/// Range: [0, 2^64-1].
pub struct Timestamp(pub u64);

impl Timestamp {
    /// Get the current Timestamp (nanoseconds since epoch).
    pub fn now() -> Self {
        let now = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_else(|_| panic!("SystemTime before UNIX_EPOCH"));
        let ns = now.as_nanos();
        Self(ns as u64)
    }

    #[inline]
    fn parse_time_ns(time_part: &str) -> Result<(u64, u64, u64, u64), TimeParseError> {
        let tb = time_part.as_bytes();
        let (hh, mm, ss) = parse_hh_mm_ss(tb)?;
        let ns = if tb.len() > 8 {
            let tail = &tb[8..];
            match crate::parse::parse_ns9_maybe(tail) {
                Ok(Some(v)) => v,
                Ok(None) => 0,
                Err(e) => return Err(e),
            }
        } else {
            0
        };
        Ok((hh as u64, mm as u64, ss as u64, ns))
    }
}

//
// Operators
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
    type Error = TimeParseError;

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
        let date = Date(days);
        let sod = secs % 86_400;
        let h = sod / 3600;
        let m = (sod % 3600) / 60;
        let s = sod % 60;
        write!(f, "{date}T")?;
        write_hms_ns(f, h, m, s, ns)?;
        f.write_str("Z")
    }
}

impl FromStr for Timestamp {
    type Err = TimeParseError;

    // Parse YYYY-MM-DDTHH:MM:SS.fffffffffZ or with explicit offset ±HH:MM to convert to UTC.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() < 20 {
            return Err(TimeParseError::TooShort);
        }
        let (date_part, rest) = s.split_at(10);
        if &rest[0..1] != "T" {
            return Err(TimeParseError::MissingT);
        }
        let (time_part, tz_part) = split_time_and_tz(&s[11..])?;
        let date: Date = date_part.parse::<Date>()?;
        // parse time with 9 fractional digits
        let (hh, mm, ss, ns) = Self::parse_time_ns(time_part)?;
        let total_ns: u128 = (date.0 as i128).max(0) as u128 * 86_400 * 1_000_000_000
            + (hh as u128) * 3_600 * 1_000_000_000
            + (mm as u128) * 60 * 1_000_000_000
            + (ss as u128) * 1_000_000_000
            + ns as u128;
        if date.0 < 0 {
            return Err(TimeParseError::BeforeEpoch);
        }
        let offset_ns = parse_tz_offset(tz_part)?;
        let adj = (total_ns as i128) - offset_ns;
        if adj < 0 {
            return Err(TimeParseError::BeforeEpoch);
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
