use std::convert::TryFrom;
use std::error::Error as StdError;
use std::fmt;
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};
use std::str::FromStr;

/// Duration in signed 64-bit nanosecond precision
/// range: ±292.277 years
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Duration(pub i64);

/// errors for parsing and converting `Duration`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DurationError {
    Empty,
    Invalid,
    MissingP,
    ExpectedNumber,
    InvalidUnitOrPosition,
    InvalidFraction,
    Overflow,
    NegativeToStd,
}

impl fmt::Display for DurationError {
    /// Format: "empty duration", "invalid duration", "missing 'P'", "expected number", "invalid unit or position", "invalid fractional seconds", "duration overflow", "negative duration cannot convert to std::time::Duration"
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = match self {
            DurationError::Empty => "empty duration",
            DurationError::Invalid => "invalid duration",
            DurationError::MissingP => "missing 'P'",
            DurationError::ExpectedNumber => "expected number",
            DurationError::InvalidUnitOrPosition => "invalid unit or position",
            DurationError::InvalidFraction => "invalid fractional seconds",
            DurationError::Overflow => "duration overflow",
            DurationError::NegativeToStd => "negative duration cannot convert to std::time::Duration",
        };
        f.write_str(msg)
    }
}

impl StdError for DurationError {}

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
        let mut nanos = self.0;
        let neg = nanos < 0;
        if neg {
            nanos = -nanos;
        }
        let mut s = String::new();
        if neg {
            s.push('-');
        }
        s.push('P');
        let mut rem = nanos;
        let day_ns: i64 = 86_400 * 1_000_000_000;
        let hour_ns: i64 = 3_600 * 1_000_000_000;
        let minute_ns: i64 = 60 * 1_000_000_000;
        let days = rem / day_ns;
        rem -= days * day_ns;
        if days != 0 {
            s.push_str(&days.to_string());
            s.push('D');
        }
        if rem != 0 {
            s.push('T');
            let hours = rem / hour_ns;
            rem -= hours * hour_ns;
            if hours != 0 {
                s.push_str(&hours.to_string());
                s.push('H');
            }
            let minutes = rem / minute_ns;
            rem -= minutes * minute_ns;
            if minutes != 0 {
                s.push_str(&minutes.to_string());
                s.push('M');
            }
            let secs = rem / 1_000_000_000;
            let ns = (rem % 1_000_000_000) as i64;
            if secs != 0 || ns != 0 {
                s.push_str(&secs.to_string());
                if ns != 0 {
                    // fractional seconds up to 9 digits without trailing zeros
                    let mut frac = format!("{:09}", ns);
                    while frac.ends_with('0') {
                        frac.pop();
                    }
                    s.push('.');
                    s.push_str(&frac);
                }
                s.push('S');
            }
        }
        s
    }
}

//
// Duration operators
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

//
// Scalar operators
//

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
    type Error = DurationError;

    fn try_from(value: std::time::Duration) -> Result<Self, Self::Error> {
        let nanos_u128 = value.as_nanos();
        if nanos_u128 > i64::MAX as u128 {
            return Err(DurationError::Overflow);
        }
        Ok(Duration(nanos_u128 as i64))
    }
}

impl TryFrom<Duration> for std::time::Duration {
    type Error = DurationError;

    fn try_from(value: Duration) -> Result<Self, Self::Error> {
        if value.0 < 0 {
            return Err(DurationError::NegativeToStd);
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
    type Err = DurationError;

    /// Accepts ISO 8601 duration formats like: P3DT4H, PT1.234567890S, -PT2S, P0D, PT0S.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(DurationError::Empty);
        }
        let bytes = s.as_bytes();
        let mut idx = 0;
        let mut negative = false;
        if bytes[idx] == b'-' {
            negative = true;
            idx += 1;
            if idx >= bytes.len() {
                return Err(DurationError::Invalid);
            }
        }
        if bytes.get(idx) != Some(&b'P') {
            return Err(DurationError::MissingP);
        }
        idx += 1;

        let mut days: i64 = 0;
        let mut hours: i64 = 0;
        let mut minutes: i64 = 0;
        let mut seconds: i64 = 0;
        let mut nanos: i64 = 0;
        let mut in_time = false;

        while idx < bytes.len() {
            if bytes[idx] == b'T' {
                in_time = true;
                idx += 1;
                continue;
            }
            // parse an integer (or integer.fraction if seconds)
            let start = idx;
            while idx < bytes.len() && bytes[idx].is_ascii_digit() {
                idx += 1;
            }
            if start == idx {
                return Err(DurationError::ExpectedNumber);
            }
            let num: i64 = s[start..idx].parse().map_err(|_| DurationError::Invalid)?;
            if idx >= bytes.len() {
                return Err(DurationError::Invalid);
            }
            let unit = bytes[idx];
            idx += 1;
            match unit {
                b'D' if !in_time => days = num,
                b'H' if in_time => hours = num,
                b'M' if in_time => minutes = num,
                b'S' if in_time => seconds = num,
                b'.' if in_time => {
                    // previous number is the integral seconds component
                    seconds = num;
                    // fraction then must end with 'S'
                    // read fraction digits up to 9
                    let frac_start = idx;
                    let mut frac_end = frac_start;
                    while frac_end < bytes.len() && bytes[frac_end].is_ascii_digit() {
                        frac_end += 1;
                    }
                    if frac_start == frac_end || frac_end >= bytes.len() || bytes[frac_end] != b'S' {
                        return Err(DurationError::InvalidFraction);
                    }
                    let mut frac = &s[frac_start..frac_end];
                    let len = frac.len();
                    let mut ns_val: i64 = 0;
                    // take up to 9 digits, pad with zeros to the right
                    if len <= 9 {
                        // parse and scale
                        for ch in frac.as_bytes() {
                            ns_val = ns_val * 10 + (*ch - b'0') as i64;
                        }
                        for _ in 0..(9 - len) {
                            ns_val *= 10;
                        }
                    } else {
                        // truncate beyond 9
                        frac = &frac[..9];
                        for ch in frac.as_bytes() {
                            ns_val = ns_val * 10 + (*ch - b'0') as i64;
                        }
                    }
                    nanos = ns_val;
                    idx = frac_end + 1; // skip 'S'
                }
                _ => return Err(DurationError::InvalidUnitOrPosition),
            }
        }

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
            return Err(DurationError::Overflow);
        }
        Ok(Duration(total as i64))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quickcheck_macros::quickcheck;

    #[test]
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
    fn test_parse_duration() {
        let d: Duration = "PT1.23456789S".parse().unwrap();
        assert_eq!(d.as_nanos(), 1_234_567_890);
        assert_eq!(d.to_string(), "PT1.23456789S");

        let d2: Duration = "P3DT4H".parse().unwrap();
        assert_eq!(d2.to_string(), "P3DT4H");
        let d3: Duration = "-PT2S".parse().unwrap();
        assert_eq!(d3.as_nanos(), -2_000_000_000);
        assert_eq!(d3.to_string(), "-PT2S");
    }

    #[quickcheck]
    fn test_duration_roundtrip(n: i64) -> bool {
        let d = Duration::from_nanos(n);
        let s = d.to_string();
        let back: Duration = s.parse().unwrap();
        back == d
    }
}
