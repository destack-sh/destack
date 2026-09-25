use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

/// Nanosecond duration on the shared World timeline.
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    Reflect,
)]
pub struct Nanos(u64);

impl Nanos {
    /// Create a nanosecond value.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Return the raw nanosecond value.
    pub const fn get(self) -> u64 {
        self.0
    }

    /// Return the saturating sum of two nanosecond values.
    pub const fn saturating_add(self, other: Self) -> Self {
        Self(self.0.saturating_add(other.0))
    }

    /// Return the saturating difference of two nanosecond values.
    pub const fn saturating_sub(self, other: Self) -> Self {
        Self(self.0.saturating_sub(other.0))
    }
}

impl From<u64> for Nanos {
    fn from(value: u64) -> Self {
        Self::new(value)
    }
}
