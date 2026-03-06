use serde::{Deserialize, Serialize};

/// Nanosecond value used by runtime time internals.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
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

/// Absolute instant on the shared world timeline.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct WorldInstant(Nanos);

impl WorldInstant {
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

impl From<u64> for WorldInstant {
    fn from(value: u64) -> Self {
        Self::new(value)
    }
}
