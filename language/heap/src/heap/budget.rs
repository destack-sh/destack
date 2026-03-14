use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt;

use super::{
    HeapLimitError, HeapLimits, HeapUsage, ManagedLimits, MemoryUsage, RawLimits, SharedSpaceUsage,
};

/// Hard limits for world-shared memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SharedLimits {
    /// Optional hard limit for retained shared-memory bytes.
    pub max_bytes: Option<u64>,
}

impl SharedLimits {
    /// Check exact retained shared-memory bytes against these limits.
    pub fn check(&self, retained_bytes: u64) -> Result<(), SharedLimitError> {
        if let Some(max_bytes) = self.max_bytes
            && retained_bytes > max_bytes
        {
            return Err(SharedLimitError {
                used_bytes: retained_bytes,
                max_bytes,
            });
        }

        Ok(())
    }

    /// Check retained shared-memory bytes after one signed retained-byte delta.
    pub fn check_delta(
        &self,
        retained_bytes: u64,
        retained_delta: i64,
    ) -> Result<(), SharedLimitError> {
        self.check(apply_delta(retained_bytes, retained_delta))
    }
}

/// Combined memory limits across local heap and shared memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct MemoryLimits {
    /// The local heap limits.
    pub heap: HeapLimits,
    /// The shared-memory limits.
    pub shared: SharedLimits,
}

/// One shared-memory hard-limit violation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SharedLimitError {
    /// The exact retained shared-memory bytes.
    pub used_bytes: u64,
    /// The configured hard limit in bytes.
    pub max_bytes: u64,
}

impl fmt::Display for SharedLimitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "shared memory limit exceeded: using {} bytes with limit {}",
            self.used_bytes, self.max_bytes,
        )
    }
}

impl Error for SharedLimitError {}

/// One live admission budget for local heap operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeapBudget {
    /// The configured heap limits.
    limits: HeapLimits,
    /// The managed-space budget.
    pub managed: ManagedBudget,
    /// The raw-space budget.
    pub raw: RawBudget,
}

/// One live admission budget for local managed-space operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ManagedBudget {
    /// The configured managed-space limits.
    pub limits: ManagedLimits,
    /// The current retained managed bytes.
    pub retained_bytes: u64,
}

/// One live admission budget for local raw-space operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawBudget {
    /// The configured raw-space limits.
    pub limits: RawLimits,
    /// The current retained raw bytes.
    pub retained_bytes: u64,
}

impl HeapBudget {
    /// Create one heap budget from explicit retained-byte counts.
    pub fn new(limits: HeapLimits, managed_retained_bytes: u64, raw_retained_bytes: u64) -> Self {
        Self {
            limits,
            managed: ManagedBudget {
                limits: limits.managed,
                retained_bytes: managed_retained_bytes,
            },
            raw: RawBudget {
                limits: limits.raw,
                retained_bytes: raw_retained_bytes,
            },
        }
    }

    /// Create one heap budget from one usage snapshot.
    pub fn from_usage(limits: HeapLimits, usage: HeapUsage) -> Self {
        Self::new(
            limits,
            usage.managed.retained_bytes,
            usage.raw.retained_bytes,
        )
    }

    /// Return the configured heap limits.
    pub fn limits(&self) -> HeapLimits {
        self.limits
    }

    /// Check one retained-byte delta without mutating this budget.
    pub fn check_delta(&self, managed_delta: i64, raw_delta: i64) -> Result<(), HeapLimitError> {
        self.limits.check_delta(
            self.managed.retained_bytes,
            self.raw.retained_bytes,
            managed_delta,
            raw_delta,
        )
    }

    /// Apply one retained-byte delta after one committed mutation.
    pub fn apply_delta(&mut self, managed_delta: i64, raw_delta: i64) {
        self.managed.retained_bytes = apply_delta(self.managed.retained_bytes, managed_delta);
        self.raw.retained_bytes = apply_delta(self.raw.retained_bytes, raw_delta);
    }
}

/// One live admission budget for world-shared memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SharedBudget {
    /// The configured shared-memory limits.
    limits: SharedLimits,
    /// The current retained shared-memory bytes.
    retained_bytes: u64,
}

impl SharedBudget {
    /// Create one shared-memory budget from explicit retained bytes.
    pub fn new(limits: SharedLimits, retained_bytes: u64) -> Self {
        Self {
            limits,
            retained_bytes,
        }
    }

    /// Create one shared-memory budget from one usage snapshot.
    pub fn from_usage(limits: SharedLimits, usage: SharedSpaceUsage) -> Self {
        Self::new(limits, usage.retained_bytes)
    }

    /// Return the configured shared-memory limits.
    pub fn limits(&self) -> SharedLimits {
        self.limits
    }

    /// Check one retained-byte delta without mutating this budget.
    pub fn check_retained_delta(&self, retained_delta: i64) -> Result<(), SharedLimitError> {
        self.limits.check_delta(self.retained_bytes, retained_delta)
    }

    /// Apply one retained-byte delta after one committed mutation.
    pub fn apply_retained_delta(&mut self, retained_delta: i64) {
        self.retained_bytes = apply_delta(self.retained_bytes, retained_delta);
    }
}

/// One combined live admission budget across local and shared memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryBudget {
    /// The local heap budget.
    pub heap: HeapBudget,
    /// The shared-memory budget.
    pub shared: SharedBudget,
}

impl MemoryBudget {
    /// Create one combined memory budget.
    pub fn new(heap: HeapBudget, shared: SharedBudget) -> Self {
        Self { heap, shared }
    }

    /// Create one combined memory budget from one usage snapshot.
    pub fn from_usage(limits: MemoryLimits, usage: MemoryUsage) -> Self {
        Self {
            heap: HeapBudget::from_usage(limits.heap, usage.heap),
            shared: SharedBudget::from_usage(limits.shared, usage.shared),
        }
    }
}

/// Apply one signed retained-byte delta to one current byte count.
fn apply_delta(current: u64, delta: i64) -> u64 {
    if delta >= 0 {
        current.saturating_add(delta as u64)
    } else {
        current.saturating_sub(delta.unsigned_abs())
    }
}
