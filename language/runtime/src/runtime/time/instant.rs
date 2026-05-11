use serde::{Deserialize, Serialize};

use super::Nanos;

/// Absolute instant on the shared world timeline.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct Instant(Nanos);

impl Instant {
    /// Create a world instant from one raw nanosecond value.
    pub const fn new(value: u64) -> Self {
        Self(Nanos::new(value))
    }

    /// Create a world instant from one typed nanosecond value.
    pub const fn from_nanos(value: Nanos) -> Self {
        Self(value)
    }

    /// Return the raw nanosecond value.
    pub const fn get(self) -> u64 {
        self.0.get()
    }

    /// Return the wrapped nanosecond value.
    pub const fn nanos(self) -> Nanos {
        self.0
    }

    /// Return the saturating duration between two instants.
    pub const fn saturating_sub(self, other: Self) -> Nanos {
        self.0.saturating_sub(other.0)
    }

    /// Return a new instant advanced by one duration.
    pub const fn saturating_add(self, duration: Nanos) -> Self {
        Self(self.0.saturating_add(duration))
    }
}

impl From<u64> for Instant {
    fn from(value: u64) -> Self {
        Self::new(value)
    }
}
