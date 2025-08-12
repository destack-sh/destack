use std::convert::TryFrom;
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

/// Duration in signed 64-bit nanosecond precision.
/// Range: ±292.277 years.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Duration(pub i64);

impl Duration {
    /// Create a Duration from seconds.
    pub fn from_seconds(seconds: i64) -> Self {
        Duration(seconds.saturating_mul(1_000_000_000))
    }

    /// Create a Duration from milliseconds.
    pub fn from_millis(millis: i64) -> Self {
        Duration(millis.saturating_mul(1_000_000))
    }

    /// Create a Duration from microseconds.
    pub fn from_micros(micros: i64) -> Self {
        Duration(micros.saturating_mul(1_000))
    }

    /// Create a Duration from nanoseconds.
    pub fn from_nanos(nanos: i64) -> Self {
        Duration(nanos)
    }

    /// Total nanoseconds contained in this Duration.
    pub fn as_nanos(self) -> i64 {
        self.0
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
    type Error = &'static str;

    fn try_from(value: std::time::Duration) -> Result<Self, Self::Error> {
        let nanos_u128 = value.as_nanos();
        if nanos_u128 > i64::MAX as u128 {
            return Err("duration overflow converting to i64 nanoseconds");
        }
        Ok(Duration(nanos_u128 as i64))
    }
}

impl TryFrom<Duration> for std::time::Duration {
    type Error = &'static str;

    fn try_from(value: Duration) -> Result<Self, Self::Error> {
        if value.0 < 0 {
            return Err("negative duration cannot convert to std::time::Duration");
        }
        Ok(std::time::Duration::from_nanos(value.0 as u64))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ops_and_conversions() {
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
}
