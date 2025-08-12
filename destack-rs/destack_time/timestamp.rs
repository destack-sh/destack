use std::convert::TryFrom;
use std::ops::{Add, Sub};

use crate::Duration;

/// Timestamp in unsigned 64-bit nanosecond precision.
/// Range: 00:00:00.000_000_000 to 23:59:59.999_999_999.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Timestamp(pub u64);

impl Timestamp {
    const DAY_NANOS: u128 = 24 * 60 * 60 * 1_000_000_000;

    /// Get the current Timestamp.
    pub fn now() -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("SystemTimestamp before UNIX_EPOCH");
        // get nanoseconds since midnight today
        let nanos_since_midnight = now.as_nanos() % Self::DAY_NANOS;
        Self(nanos_since_midnight as u64)
    }
}

//
// Duration operators
//

impl Add<Duration> for Timestamp {
    type Output = Timestamp;

    fn add(self, rhs: Duration) -> Timestamp {
        let total_nanos = self.0 as i64 + rhs.0;
        let nanos_in_day = total_nanos.rem_euclid(Self::DAY_NANOS as i64);
        Timestamp(nanos_in_day as u64)
    }
}

impl Sub<Duration> for Timestamp {
    type Output = Timestamp;

    fn sub(self, rhs: Duration) -> Timestamp {
        let total_nanos = self.0 as i64 - rhs.0;
        let nanos_in_day = total_nanos.rem_euclid(Self::DAY_NANOS as i64);
        Timestamp(nanos_in_day as u64)
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
    type Error = &'static str;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        if (value as u128) >= Self::DAY_NANOS {
            return Err("timestamp nanoseconds exceed a day");
        }
        Ok(Timestamp(value))
    }
}

impl TryFrom<(u32, u32, u32)> for Timestamp {
    type Error = &'static str;

    fn try_from(hms: (u32, u32, u32)) -> Result<Self, Self::Error> {
        let (h, m, s) = hms;
        if h >= 24 || m >= 60 || s >= 60 {
            return Err("invalid timestamp components");
        }
        let nanos = (h as u128) * 3_600_000_000_000
            + (m as u128) * 60_000_000_000
            + (s as u128) * 1_000_000_000;
        Ok(Timestamp(nanos as u64))
    }
}

impl TryFrom<(u32, u32, u32, u32)> for Timestamp {
    type Error = &'static str;

    fn try_from(hmsn: (u32, u32, u32, u32)) -> Result<Self, Self::Error> {
        let (h, m, s, n) = hmsn;
        if h >= 24 || m >= 60 || s >= 60 || n >= 1_000_000_000 {
            return Err("invalid timestamp components");
        }
        let nanos = (h as u128) * 3_600_000_000_000
            + (m as u128) * 60_000_000_000
            + (s as u128) * 1_000_000_000
            + (n as u128);
        Ok(Timestamp(nanos as u64))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_sub_and_diff() {
        let t = Timestamp::try_from((12, 0, 0)).unwrap();
        let t2 = t + Duration::from_seconds(1);
        assert_eq!(t2 - t, Duration::from_seconds(1));
        let wrapped = Timestamp::try_from((23, 59, 59, 900_000_000)).unwrap() + Duration::from_millis(200);
        let expected = Timestamp::try_from((0, 0, 0, 100_000_000)).unwrap();
        assert_eq!(wrapped, expected);
    }
}
