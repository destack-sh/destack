use std::convert::TryFrom;
use std::error::Error as StdError;
use std::fmt;
use std::ops::{Add, Sub};
use std::str::FromStr;

use crate::Duration;
use crate::parser;

/// Time in unsigned 64-bit nanosecond precision since midnight
/// range: 00:00:00.000_000_000 to 23:59:59.999_999_999
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Time(pub u64);

/// errors for parsing or constructing `Time`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeError {
    TooShort,
    MissingColon,
    OutOfRange,
    FractionMustBe9,
    InvalidFraction,
    Trailing,
}

impl fmt::Display for TimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = match self {
            TimeError::TooShort => "too short",
            TimeError::MissingColon => "missing colon",
            TimeError::OutOfRange => "time out of range",
            TimeError::FractionMustBe9 => "fraction must be 9 digits",
            TimeError::InvalidFraction => "invalid fraction",
            TimeError::Trailing => "invalid trailing characters",
        };
        f.write_str(msg)
    }
}

impl StdError for TimeError {}

impl Time {
    /// number of nanoseconds in a day
    const DAY_NANOS: u128 = 24 * 60 * 60 * 1_000_000_000;

    /// Get the current time (UTC) as nanoseconds since midnight
    pub fn now() -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("SystemTime before UNIX_EPOCH");
        // get nanoseconds since midnight today
        let nanos_since_midnight = now.as_nanos() % Self::DAY_NANOS;
        Self(nanos_since_midnight as u64)
    }

    #[inline]
    /// Parse the fractional part of a time string.
    fn parse_fraction_ns_9(b: &[u8], idx: usize) -> Result<(u64, usize), TimeError> {
        if idx >= b.len() || b[idx] != b'.' {
            return Err(TimeError::Trailing);
        }
        let start = idx + 1;
        let ns = crate::parser::parse_ns_digits_exact(&b[start..]).map_err(|e| match e {
            parser::ParseError::FractionDigitsMustBe9 => TimeError::FractionMustBe9,
            parser::ParseError::InvalidTime | parser::ParseError::InvalidNumber => {
                TimeError::InvalidFraction
            }
            _ => TimeError::InvalidFraction,
        })?;
        Ok((ns, b.len()))
    }

    #[inline]
    /// Parse a time string in the format HH:MM:SS[.NNNNNNNNN].
    fn parse_hms_ns(b: &[u8]) -> Result<u64, TimeError> {
        if b.len() < 8 {
            return Err(TimeError::TooShort);
        }
        let h = match crate::parser::parse_two_digits(&b[0..2]) {
            Ok(v) => v as u64,
            Err(_) => return Err(TimeError::TooShort),
        };
        if b.get(2) != Some(&b':') {
            return Err(TimeError::MissingColon);
        }
        let m = match crate::parser::parse_two_digits(&b[3..5]) {
            Ok(v) => v as u64,
            Err(_) => return Err(TimeError::TooShort),
        };
        if b.get(5) != Some(&b':') {
            return Err(TimeError::MissingColon);
        }
        let s = match crate::parser::parse_two_digits(&b[6..8]) {
            Ok(v) => v as u64,
            Err(_) => return Err(TimeError::TooShort),
        };
        if h >= 24 || m >= 60 || s >= 60 {
            return Err(TimeError::OutOfRange);
        }
        let mut ns: u64 = 0;
        if 8 < b.len() {
            let (frac, _end) = Self::parse_fraction_ns_9(b, 8)?;
            ns = frac;
        }
        let nanos = h * 3_600_000_000_000 + m * 60_000_000_000 + s * 1_000_000_000 + ns;
        Ok(nanos)
    }
}

//
// Oerators
//

impl Add<Duration> for Time {
    type Output = Time;

    fn add(self, rhs: Duration) -> Time {
        let total_nanos = self.0 as i64 + rhs.0;
        let nanos_in_day = total_nanos.rem_euclid(Self::DAY_NANOS as i64);
        Time(nanos_in_day as u64)
    }
}

impl Sub<Duration> for Time {
    type Output = Time;

    fn sub(self, rhs: Duration) -> Time {
        let total_nanos = self.0 as i64 - rhs.0;
        let nanos_in_day = total_nanos.rem_euclid(Self::DAY_NANOS as i64);
        Time(nanos_in_day as u64)
    }
}

impl Sub<Time> for Time {
    type Output = Duration;

    fn sub(self, rhs: Time) -> Duration {
        Duration(self.0 as i64 - rhs.0 as i64)
    }
}

//
// Conversions
//

impl From<Time> for u64 {
    fn from(value: Time) -> Self {
        value.0
    }
}

impl fmt::Display for Time {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut rem = self.0;
        let h = rem / 3_600_000_000_000;
        rem -= h * 3_600_000_000_000;
        let m = rem / 60_000_000_000;
        rem -= m * 60_000_000_000;
        let s = rem / 1_000_000_000;
        let ns = rem - s * 1_000_000_000;
        write!(f, "{:02}:{:02}:{:02}.{:09}", h, m, s, ns)
    }
}

impl FromStr for Time {
    type Err = TimeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let b = s.as_bytes();
        let nanos = Self::parse_hms_ns(b)?;
        Ok(Time(nanos))
    }
}

// tests at bottom
impl TryFrom<u64> for Time {
    type Error = &'static str;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        if (value as u128) >= Self::DAY_NANOS {
            return Err("time nanoseconds exceed a day");
        }
        Ok(Time(value))
    }
}

impl TryFrom<(u32, u32, u32)> for Time {
    type Error = &'static str;

    fn try_from(hms: (u32, u32, u32)) -> Result<Self, Self::Error> {
        let (h, m, s) = hms;
        if h >= 24 || m >= 60 || s >= 60 {
            return Err("invalid time components");
        }
        let nanos = (h as u128) * 3_600_000_000_000
            + (m as u128) * 60_000_000_000
            + (s as u128) * 1_000_000_000;
        Ok(Time(nanos as u64))
    }
}

impl TryFrom<(u32, u32, u32, u32)> for Time {
    type Error = &'static str;

    fn try_from(hmsn: (u32, u32, u32, u32)) -> Result<Self, Self::Error> {
        let (h, m, s, n) = hmsn;
        if h >= 24 || m >= 60 || s >= 60 || n >= 1_000_000_000 {
            return Err("invalid time components");
        }
        let nanos = (h as u128) * 3_600_000_000_000
            + (m as u128) * 60_000_000_000
            + (s as u128) * 1_000_000_000
            + (n as u128);
        Ok(Time(nanos as u64))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quickcheck_macros::quickcheck;

    #[test]
    fn test_format_parse_time() {
        let time = Time::try_from((15, 34, 56, 123_456_789)).unwrap();
        let time_str = time.to_string();
        assert_eq!(time_str, "15:34:56.123456789");
        let parsed_time: Time = time_str.parse().unwrap();
        assert_eq!(parsed_time, time);
    }

    #[quickcheck]
    fn test_time_roundtrip(n: u64) -> bool {
        let day = 24u64 * 60 * 60 * 1_000_000_000;
        let time = Time(n % day);
        let time_str = time.to_string();
        let parsed_time: Time = time_str.parse().unwrap();
        parsed_time == time
    }

    #[test]
    fn test_add_sub_duration_wraps() {
        let time = Time::try_from((23, 59, 59, 900_000_000)).unwrap();
        let later = time + Duration::from_millis(200);
        // wraps to next day at 00:00:00.100
        let expected = Time::try_from((0, 0, 0, 100_000_000)).unwrap();
        assert_eq!(later, expected);
        let earlier = expected - Duration::from_millis(200);
        assert_eq!(earlier, time);
    }

    #[test]
    fn test_sub_time_gives_duration() {
        let time1 = Time::try_from((1, 0, 0)).unwrap();
        let time2 = Time::try_from((0, 30, 0)).unwrap();
        let duration = time1 - time2;
        assert_eq!(duration.as_nanos(), 30 * 60 * 1_000_000_000);
    }
}
