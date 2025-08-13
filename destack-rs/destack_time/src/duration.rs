//! Duration in signed 64-bit nanosecond precision.
//! We provide the `Duration` type and some conversions.

use std::convert::TryFrom;
use std::fmt;
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};
use std::str::FromStr;

use crate::parse::TimeParseError;

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// Duration in signed 64-bit nanosecond precision.
///
/// Range: ±292,277 years.
pub struct Duration(pub i64);

impl Duration {
    /// Create Duration from seconds.
    pub fn from_seconds(seconds: i64) -> Self {
        Duration(seconds.saturating_mul(1_000_000_000))
    }

    /// Create Duration from milliseconds.
    pub fn from_millis(millis: i64) -> Self {
        Duration(millis.saturating_mul(1_000_000))
    }

    /// Create Duration from microseconds.
    pub fn from_micros(micros: i64) -> Self {
        Duration(micros.saturating_mul(1_000))
    }

    /// Create Duration from nanoseconds.
    pub fn from_nanos(nanos: i64) -> Self {
        Duration(nanos)
    }

    /// Total nanoseconds contained.
    pub fn as_nanos(self) -> i64 {
        self.0
    }

    /// Format as ISO 8601 duration (days, hours, minutes, seconds with optional fractional seconds).
    pub fn to_iso(&self) -> String {
        if self.0 == 0 {
            return "PT0S".to_string();
        }

        let mut nanos = self.0 as i128;
        let is_negative = nanos < 0;
        if is_negative {
            nanos = -nanos;
        }

        let mut result = String::new();
        if is_negative {
            result.push('-');
        }
        result.push('P');

        // extract days
        const DAY_NS: i128 = 86_400 * 1_000_000_000;
        const HOUR_NS: i128 = 3_600 * 1_000_000_000;
        const MINUTE_NS: i128 = 60 * 1_000_000_000;
        const SECOND_NS: i128 = 1_000_000_000;

        let days = nanos / DAY_NS;
        let mut remainder = nanos % DAY_NS;

        // add days if any
        if days > 0 {
            result.push_str(&days.to_string());
            result.push('D');
        }

        // extract time components
        if remainder > 0 {
            // add time separator
            result.push('T');

            // extract hours
            let hours = remainder / HOUR_NS;
            remainder %= HOUR_NS;
            if hours > 0 {
                result.push_str(&hours.to_string());
                result.push('H');
            }

            // extract minutes
            let minutes = remainder / MINUTE_NS;
            remainder %= MINUTE_NS;
            if minutes > 0 {
                result.push_str(&minutes.to_string());
                result.push('M');
            }

            // extract seconds and fractional seconds
            let seconds = remainder / SECOND_NS;
            let fractional_ns = remainder % SECOND_NS;
            if seconds > 0 || fractional_ns > 0 {
                result.push_str(&seconds.to_string());

                // add fractional seconds if any
                if fractional_ns > 0 {
                    // format fractional seconds without trailing zeros
                    let mut fractional = format!("{fractional_ns:09}");
                    while fractional.ends_with('0') {
                        fractional.pop();
                    }
                    result.push('.');
                    result.push_str(&fractional);
                }
                result.push('S');
            }
        }

        result
    }

    /// Parse ISO 8601 duration format.
    pub fn from_iso(s: &str) -> Result<Self, crate::parse::TimeParseError> {
        if s.is_empty() {
            return Err(TimeParseError::TooShort);
        }
        let bytes = s.as_bytes();
        let mut idx = 0;
        let mut negative = false;

        // optional negative sign
        if bytes[idx] == b'-' {
            negative = true;
            idx += 1;
            if idx >= bytes.len() {
                return Err(TimeParseError::InvalidFormat);
            }
        }

        // mandatory 'P'
        if bytes.get(idx) != Some(&b'P') {
            return Err(TimeParseError::InvalidFormat);
        }
        idx += 1;

        // parse components
        let mut days: i64 = 0;
        let mut hours: i64 = 0;
        let mut minutes: i64 = 0;
        let mut seconds: i64 = 0;
        let mut nanos: i64 = 0;
        let mut is_in_time = false;

        // parse each component
        while idx < bytes.len() {
            if bytes[idx] == b'T' {
                is_in_time = true;
                idx += 1;
                continue;
            }

            // parse an integer (or integer.fraction if seconds)
            let start = idx;
            while idx < bytes.len() && bytes[idx].is_ascii_digit() {
                idx += 1;
            }
            if start == idx {
                return Err(TimeParseError::InvalidFormat);
            }
            let num: i64 = s[start..idx]
                .parse()
                .map_err(|_| TimeParseError::InvalidFormat)?;

            if idx >= bytes.len() {
                return Err(TimeParseError::InvalidFormat);
            }
            let unit = bytes[idx];
            idx += 1;

            match unit {
                b'D' if !is_in_time => days = num,
                b'H' if is_in_time => hours = num,
                b'M' if is_in_time => minutes = num,
                b'S' if is_in_time => seconds = num,
                b'.' if is_in_time => {
                    // previous number is the integral seconds component
                    seconds = num;

                    // fraction then must end with 'S'
                    let frac_start = idx;
                    let mut frac_end = frac_start;
                    while frac_end < bytes.len() && bytes[frac_end].is_ascii_digit() {
                        frac_end += 1;
                    }
                    if frac_start == frac_end || frac_end >= bytes.len() || bytes[frac_end] != b'S'
                    {
                        return Err(TimeParseError::InvalidFormat);
                    }
                    let mut frac = &s[frac_start..frac_end];
                    let len = frac.len();
                    let mut ns: i64 = 0;

                    // take up to 9 digits, pad with zeros to the right
                    if len > 9 {
                        frac = &frac[..9];
                    }
                    for ch in frac.as_bytes() {
                        ns = ns * 10 + (*ch - b'0') as i64;
                    }
                    for _ in 0..(9 - frac.len()) {
                        ns *= 10;
                    }
                    nanos = ns;
                    idx = frac_end + 1; // skip 'S'
                }
                _ => return Err(TimeParseError::InvalidFormat),
            }
        }

        // compute total nanoseconds
        let mut total: i128 = 0;
        total += (days as i128) * 86_400 * 1_000_000_000i128;
        total += (hours as i128) * 3_600 * 1_000_000_000i128;
        total += (minutes as i128) * 60 * 1_000_000_000i128;
        total += (seconds as i128) * 1_000_000_000i128;
        total += nanos as i128;
        if negative {
            total = -total;
        }
        if total < i64::MIN as i128 || total > i64::MAX as i128 {
            return Err(TimeParseError::Overflow);
        }
        Ok(Duration(total as i64))
    }
}

//
// Operators
//

impl Add<Duration> for Duration {
    type Output = Duration;

    fn add(self, rhs: Duration) -> Duration {
        Duration(self.0 + rhs.0)
    }
}

impl AddAssign for Duration {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}

impl Sub<Duration> for Duration {
    type Output = Duration;

    fn sub(self, rhs: Duration) -> Duration {
        Duration(self.0 - rhs.0)
    }
}

impl SubAssign for Duration {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0;
    }
}

impl Neg for Duration {
    type Output = Duration;

    fn neg(self) -> Self::Output {
        Duration(-self.0)
    }
}

impl Add<i64> for Duration {
    type Output = Duration;

    fn add(self, rhs: i64) -> Duration {
        Duration(self.0 + rhs)
    }
}

impl Sub<i64> for Duration {
    type Output = Duration;

    fn sub(self, rhs: i64) -> Duration {
        Duration(self.0 - rhs)
    }
}

impl Mul<i64> for Duration {
    type Output = Duration;

    fn mul(self, rhs: i64) -> Duration {
        Duration(self.0 * rhs)
    }
}

impl MulAssign<i64> for Duration {
    fn mul_assign(&mut self, rhs: i64) {
        self.0 *= rhs;
    }
}

impl Div<i64> for Duration {
    type Output = Duration;

    fn div(self, rhs: i64) -> Duration {
        Duration(self.0 / rhs)
    }
}

impl DivAssign<i64> for Duration {
    fn div_assign(&mut self, rhs: i64) {
        self.0 /= rhs;
    }
}

//
// Conversions
//

impl From<i64> for Duration {
    fn from(value: i64) -> Self {
        Duration(value)
    }
}

impl From<Duration> for i64 {
    fn from(value: Duration) -> Self {
        value.0
    }
}

impl TryFrom<std::time::Duration> for Duration {
    type Error = crate::parse::TimeParseError;

    fn try_from(value: std::time::Duration) -> Result<Self, Self::Error> {
        let nanos_u128 = value.as_nanos();
        if nanos_u128 > i64::MAX as u128 {
            return Err(crate::parse::TimeParseError::Overflow);
        }
        Ok(Duration(nanos_u128 as i64))
    }
}

impl TryFrom<Duration> for std::time::Duration {
    type Error = crate::parse::TimeParseError;

    fn try_from(value: Duration) -> Result<Self, Self::Error> {
        if value.0 < 0 {
            return Err(crate::parse::TimeParseError::InvalidFormat);
        }
        Ok(std::time::Duration::from_nanos(value.0 as u64))
    }
}

impl fmt::Display for Duration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_iso())
    }
}

impl FromStr for Duration {
    type Err = crate::parse::TimeParseError;

    /// Accepts ISO 8601 duration formats like: P3DT4H, PT1.234567890S, -PT2S, P0D, PT0S.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_iso(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quickcheck_macros::quickcheck;

    #[test]
    /// Create durations and perform arithmetic operations.
    fn test_into_duration() {
        let mut d = Duration::from_seconds(1);
        assert_eq!(d.as_nanos(), 1_000_000_000);
        d += Duration::from_millis(500);
        assert_eq!(d.as_nanos(), 1_500_000_000);
        d -= Duration::from_micros(500_000);
        assert_eq!(d.as_nanos(), 1_000_000_000);
        d *= 2;
        assert_eq!(d.as_nanos(), 2_000_000_000);
        d /= 4;
        assert_eq!(d.as_nanos(), 500_000_000);
        let neg = -d;
        assert_eq!(neg.as_nanos(), -500_000_000);

        let std_dur = std::time::Duration::from_millis(250);
        let our = Duration::try_from(std_dur).unwrap();
        assert_eq!(our.as_nanos(), 250_000_000);
        let back = std::time::Duration::try_from(our).unwrap();
        assert_eq!(back.as_millis(), 250);
    }

    #[test]
    /// Parse various ISO 8601 duration formats.
    fn test_parse_duration() {
        // basic fractional seconds
        let d: Duration = "PT1.23456789S".parse().unwrap();
        assert_eq!(d.as_nanos(), 1_234_567_890);
        assert_eq!(d.to_string(), "PT1.23456789S");

        // days and hours combo
        let d2: Duration = "P3DT4H".parse().unwrap();
        assert_eq!(d2.to_string(), "P3DT4H");

        // negative duration
        let d3: Duration = "-PT2S".parse().unwrap();
        assert_eq!(d3.as_nanos(), -2_000_000_000);
        assert_eq!(d3.to_string(), "-PT2S");

        // zero duration edge case
        let zero: Duration = "PT0S".parse().unwrap();
        assert_eq!(zero.as_nanos(), 0);
        assert_eq!(zero.to_string(), "PT0S");

        // complex multi-component duration
        let complex: Duration = "P1DT2H3M4.567S".parse().unwrap();
        let expected_ns = 86_400_000_000_000 + 7_200_000_000_000 + 180_000_000_000 + 4_567_000_000;
        assert_eq!(complex.as_nanos(), expected_ns);

        // fractional seconds with trailing zeros should be preserved in parsing
        let trailing: Duration = "PT1.100S".parse().unwrap();
        assert_eq!(trailing.as_nanos(), 1_100_000_000);

        // very small fractional seconds
        let tiny: Duration = "PT0.000000001S".parse().unwrap();
        assert_eq!(tiny.as_nanos(), 1);

        // negative complex duration
        let neg_complex: Duration = "-P2DT1H30M45.123456789S".parse().unwrap();
        let neg_expected =
            -(2 * 86_400_000_000_000 + 3_600_000_000_000 + 1_800_000_000_000 + 45_123_456_789);
        assert_eq!(neg_complex.as_nanos(), neg_expected);

        // minutes only
        let mins_only: Duration = "PT42M".parse().unwrap();
        assert_eq!(mins_only.as_nanos(), 42 * 60_000_000_000);

        // hours only
        let hours_only: Duration = "PT7H".parse().unwrap();
        assert_eq!(hours_only.as_nanos(), 7 * 3_600_000_000_000);
    }

    #[quickcheck]
    /// Roundtrip duration parsing and formatting.
    fn test_duration_roundtrip(n: i64) -> bool {
        let duration = Duration::from_nanos(n);
        let duration_str = duration.to_string();
        let parsed_duration: Duration = duration_str.parse().unwrap();
        parsed_duration == duration
    }
}
