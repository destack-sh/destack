//! Date in signed 64-bit day precision since epoch (UTC).
//! We provide the `Date` type and some conversions.

use std::fmt;
use std::ops::{Add, Sub};
use std::str::FromStr;
use std::time::SystemTime;

use crate::parser::TimeParseError;

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// Date in signed 64-bit day precision since epoch (UTC).
///
/// Range: ±2.5e16 days.
pub struct Date(pub i64);

impl Date {
    /// Get the current date (UTC)
    pub fn now() -> Self {
        Self(
            SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_else(|_| panic!("SystemTime before UNIX_EPOCH"))
                .as_secs() as i64
                / 86400, // convert seconds to days
        )
    }

    /// Format as ISO 8601 date (YYYY-MM-DD).
    pub fn to_iso(&self) -> String {
        let days_since_epoch = self.0 + 719468;
        let era = if days_since_epoch >= 0 {
            days_since_epoch / 146097
        } else {
            (days_since_epoch - 146096) / 146097
        };
        let day_of_era = days_since_epoch - era * 146097;
        let year_of_era =
            (day_of_era - day_of_era / 1460 + day_of_era / 36524 - day_of_era / 146096) / 365;
        let year = year_of_era + era * 400;
        let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
        let month_plus = (5 * day_of_year + 2) / 153;
        let day = day_of_year - (153 * month_plus + 2) / 5 + 1;
        let month = month_plus + if month_plus < 10 { 3 } else { -9 };
        let adjusted_year = year + (month <= 2) as i64;
        format!("{adjusted_year:04}-{month:02}-{day:02}")
    }

    /// Parse ISO 8601 date format (YYYY-MM-DD).
    pub fn from_iso(s: &str) -> Result<Self, TimeParseError> {
        let bytes = s.as_bytes();
        if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
            return Err(TimeParseError::InvalidFormat);
        }
        let year = Self::_parse_i32_digits(&bytes[0..4])? as i64;
        let month = Self::_parse_i32_digits(&bytes[5..7])? as i64;
        let day = Self::_parse_i32_digits(&bytes[8..10])? as i64;
        if month < 1 || month > 12 {
            return Err(TimeParseError::InvalidFormat);
        }
        if day < 1 || day > 31 {
            return Err(TimeParseError::InvalidFormat);
        }
        let adjusted_year = year - (month <= 2) as i64;
        let adjusted_month = month + if month > 2 { -3 } else { 9 };
        let era = if adjusted_year >= 0 {
            adjusted_year / 400
        } else {
            (adjusted_year - 399) / 400
        };
        let year_of_era = adjusted_year - era * 400;
        let day_of_year = (153 * adjusted_month + 2) / 5 + day - 1;
        let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
        let days_since_epoch = era * 146097 + day_of_era - 719468;
        Ok(Date(days_since_epoch))
    }

    #[inline]
    fn _parse_i32_digits(bytes: &[u8]) -> Result<i32, TimeParseError> {
        let mut v: i32 = 0;
        for &ch in bytes {
            if !ch.is_ascii_digit() {
                return Err(TimeParseError::InvalidNumber);
            }
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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_iso())
    }
}

impl FromStr for Date {
    type Err = TimeParseError;

    /// Parse a date string in the format YYYY-MM-DD.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_iso(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quickcheck_macros::quickcheck;

    #[test]
    fn test_convert_date() {
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
    fn test_parse_date() {
        let d: Date = "2025-08-12".parse().unwrap();
        assert_eq!(d.to_string(), "2025-08-12");
        let rt: Date = d.to_string().parse().unwrap();
        assert_eq!(rt, d);
    }

    #[quickcheck]
    fn test_date_roundtrip(days: i64) -> bool {
        let d = Date(days);
        if days < -100_000_000 || days > 100_000_000 {
            return true;
        }
        let s = d.to_string();
        let back: Date = s.parse().unwrap();
        back == d
    }
}
