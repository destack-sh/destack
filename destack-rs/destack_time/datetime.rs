use std::ops::{Add, Sub};

use crate::Duration;

/// DateTime in signed 64-bit microsecond precision since epoch (UTC).
/// Range: ±292,277 years.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DateTime(pub i64);

impl DateTime {
    /// Get the current DateTime.
    pub fn now() -> Self {
        Self(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("SystemTime before UNIX_EPOCH")
                .as_micros() as i64,
        )
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_sub_and_diff() {
        let dt = DateTime::now();
        let later = dt + Duration::from_millis(1_500);
        let diff = later - dt;
        assert!(diff.as_nanos() >= 1_500_000_000);
        let back = later - Duration::from_millis(1_500);
        assert_eq!(back.0, dt.0);
    }

    #[test]
    fn systemtime_roundtrip() {
        let st = std::time::SystemTime::now();
        let dt: DateTime = st.into();
        let st2: std::time::SystemTime = dt.into();
        // allow some drift but they should be close (microsecond precision)
        let delta = st2.duration_since(st).unwrap_or_else(|e| e.duration());
        assert!(delta.as_millis() < 10);
    }
}