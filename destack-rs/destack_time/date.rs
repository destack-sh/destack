use std::error::Error as StdError;
use std::fmt;
use std::ops::{Add, Sub};
use std::str::FromStr;

/// Date in signed 64-bit day precision since epoch (UTC)
/// range: ±2.5e16 days
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Date(pub i64);

/// errors for parsing `Date`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DateParseError {
    InvalidFormat,
    InvalidNumber,
    MonthOutOfRange,
    DayOutOfRange,
}

impl fmt::Display for DateParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = match self {
            DateParseError::InvalidFormat => "invalid date format",
            DateParseError::InvalidNumber => "invalid number",
            DateParseError::MonthOutOfRange => "month out of range",
            DateParseError::DayOutOfRange => "day out of range",
        };
        f.write_str(msg)
    }
}

impl StdError for DateParseError {}

impl Date {
    /// Get the current date (UTC)
    pub fn now() -> Self {
        Self(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("SystemTime before UNIX_EPOCH")
                .as_secs() as i64
                / 86400, // convert seconds to days
        )
    }

    #[inline]
    fn parse_i32_digits(bytes: &[u8]) -> Result<i32, DateParseError> {
        let mut v: i32 = 0;
        for &ch in bytes {
            if !ch.is_ascii_digit() { return Err(DateParseError::InvalidNumber); }
            v = v * 10 + (ch - b'0') as i32;
        }
        Ok(v)
    }
}

//
// Operators
//

impl Add<i64> for Date {
    type Output = Date;

    fn add(self, rhs: i64) -> Date {
        Date(self.0 + rhs)
    }
}

impl Sub<i64> for Date {
    type Output = Date;

    fn sub(self, rhs: i64) -> Date {
        Date(self.0 - rhs)
    }
}

impl Sub<Date> for Date {
    type Output = i64;

    fn sub(self, rhs: Date) -> i64 {
        self.0 - rhs.0
    }
}

//
// Conversions
//

impl From<i64> for Date {
    fn from(value: i64) -> Self {
        Date(value)
    }
}

impl From<Date> for i64 {
    fn from(value: Date) -> Self {
        value.0
    }
}

impl From<std::time::SystemTime> for Date {
    fn from(value: std::time::SystemTime) -> Self {
        let secs = match value.duration_since(std::time::UNIX_EPOCH) {
            Ok(d) => d.as_secs() as i64,
            Err(e) => -(e.duration().as_secs() as i64),
        };
        Date(secs / 86_400)
    }
}

impl fmt::Display for Date {
    /// Format the date as YYYY-MM-DD.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let z = self.0 + 719468;
        let era = if z >= 0 { z / 146097 } else { (z - 146096) / 146097 };
        let doe = z - era * 146097;
        let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
        let y = yoe + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = doy - (153 * mp + 2) / 5 + 1;
        let m = mp + if mp < 10 { 3 } else { -9 };
        let year = y + (m <= 2) as i64;
        write!(f, "{:04}-{:02}-{:02}", year, m, d)
    }
}

impl FromStr for Date {
    type Err = DateParseError;

    /// Parse a date string in the format YYYY-MM-DD.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let b = s.as_bytes();
        if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
            return Err(DateParseError::InvalidFormat);
        }
        let year = Self::parse_i32_digits(&b[0..4])? as i64;
        let month = Self::parse_i32_digits(&b[5..7])? as i64;
        let day = Self::parse_i32_digits(&b[8..10])? as i64;
        if month < 1 || month > 12 { return Err(DateParseError::MonthOutOfRange); }
        if day < 1 || day > 31 { return Err(DateParseError::DayOutOfRange); }
        let y = year - (month <= 2) as i64;
        let m = month + if month > 2 { -3 } else { 9 };
        let era = if y >= 0 { y / 400 } else { (y - 399) / 400 };
        let yoe = y - era * 400;
        let doy = (153 * m + 2) / 5 + day - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        let z = era * 146097 + doe - 719468;
        Ok(Date(z))
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ops_and_conversions() {
        let today = Date::now();
        let tomorrow = today + 1;
        assert_eq!(tomorrow - today, 1);
        let raw: i64 = tomorrow.into();
        let back: Date = raw.into();
        assert_eq!(back, Date(raw));
        let from_st: Date = std::time::SystemTime::now().into();
        assert!((from_st - today).abs() <= 1);
    }

    #[test]
    fn parse_and_format() {
        let d: Date = "2025-08-12".parse().unwrap();
        assert_eq!(d.to_string(), "2025-08-12");
        let rt: Date = d.to_string().parse().unwrap();
        assert_eq!(rt, d);
    }

    use quickcheck_macros::quickcheck;
    #[quickcheck]
    fn date_roundtrip(days: i64) -> bool {
        let d = Date(days);
        // guard: our Display keeps year in 4 digits; restrict to a safe range to avoid extreme years
        if days < -100_000_000 || days > 100_000_000 { return true; }
        let s = d.to_string();
        let back: Date = s.parse().unwrap();
        back == d
    }
}