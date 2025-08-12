use std::ops::{Add, Sub};

/// Date in signed 64-bit day precision since epoch (UTC).
/// Range: ±2.525x10^16 days.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Date(pub i64);

impl Date {
    /// Get the current Date.
    pub fn now() -> Self {
        Self(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("SystemTime before UNIX_EPOCH")
                .as_secs() as i64
                / 86400, // convert seconds to days
        )
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
}