use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt;

use super::{
    Heap, HeapLimitError, HeapLimits, HeapUsage, ManagedLimits, MemoryUsage, RawLimits,
    SharedSpaceUsage,
};

/// Combined memory limits across local heap and shared memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct MemoryLimits {
    /// The local heap limits.
    pub heap: HeapLimits,
    /// The shared-memory limits.
    pub shared: SharedLimits,
}

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
    /// The current active managed bytes.
    pub active_bytes: u64,
}

/// One live admission budget for local raw-space operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawBudget {
    /// The configured raw-space limits.
    pub limits: RawLimits,
    /// The current active raw bytes.
    pub active_bytes: u64,
}

impl HeapBudget {
    /// Create one heap budget from explicit active-byte counts.
    pub fn new(limits: HeapLimits, managed_active_bytes: u64, raw_active_bytes: u64) -> Self {
        Self {
            limits,
            managed: ManagedBudget {
                limits: limits.managed,
                active_bytes: managed_active_bytes,
            },
            raw: RawBudget {
                limits: limits.raw,
                active_bytes: raw_active_bytes,
            },
        }
    }

    /// Create one heap budget from one usage snapshot.
    pub fn from_usage(limits: HeapLimits, usage: HeapUsage) -> Self {
        Self::new(limits, usage.managed.active_bytes, usage.raw.active_bytes)
    }

    /// Return the configured heap limits.
    pub fn limits(&self) -> HeapLimits {
        self.limits
    }

    /// Check one active-byte reservation without mutating this budget.
    pub fn check_active_reservation(
        &self,
        managed_reservation: i64,
        raw_reservation: i64,
    ) -> Result<(), HeapLimitError> {
        self.limits.check_active_reservation(
            self.managed.active_bytes,
            self.raw.active_bytes,
            managed_reservation,
            raw_reservation,
        )
    }

    /// Refresh this budget after one committed mutation.
    pub fn refresh(&mut self, managed_active_bytes: u64, raw_active_bytes: u64) {
        self.managed.active_bytes = managed_active_bytes;
        self.raw.active_bytes = raw_active_bytes;
    }
}

impl Heap {
    /// Replace the heap hard limits.
    pub fn set_limits(&mut self, limits: HeapLimits) -> Result<(), HeapLimitError> {
        self.limits = limits;
        self.check_limits()
    }

    /// Check the configured heap hard limits against current usage.
    pub fn check_limits(&self) -> Result<(), HeapLimitError> {
        self.budget().check_active_reservation(0, 0)
    }

    /// Return the current live heap budget.
    pub fn budget(&self) -> HeapBudget {
        HeapBudget::new(
            self.limits,
            self.managed.active_bytes(),
            self.raw.active_bytes(),
        )
    }
}

/// One live admission budget for world-shared memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SharedBudget {
    /// The configured shared-memory limits.
    limits: SharedLimits,
    /// The current active shared-memory bytes.
    active_bytes: u64,
}

impl SharedBudget {
    /// Create one shared-memory budget from explicit active bytes.
    pub fn new(limits: SharedLimits, active_bytes: u64) -> Self {
        Self {
            limits,
            active_bytes,
        }
    }

    /// Create one shared-memory budget from one usage snapshot.
    pub fn from_usage(limits: SharedLimits, usage: SharedSpaceUsage) -> Self {
        Self::new(limits, usage.active_bytes)
    }

    /// Return the configured shared-memory limits.
    pub fn limits(&self) -> SharedLimits {
        self.limits
    }

    /// Check one active-byte reservation without mutating this budget.
    pub fn check_active_reservation(
        &self,
        active_reservation: i64,
    ) -> Result<(), SharedLimitError> {
        self.limits
            .check_active_reservation(self.active_bytes, active_reservation)
    }

    /// Refresh this budget after one committed mutation.
    pub fn refresh(&mut self, active_bytes: u64) {
        self.active_bytes = active_bytes;
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

/// Apply one signed reservation to one current byte count.
fn apply_reservation(current: u64, reservation: i64) -> u64 {
    if reservation >= 0 {
        current.saturating_add(reservation as u64)
    } else {
        current.saturating_sub(reservation.unsigned_abs())
    }
}

/// Hard limits for world-shared memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SharedLimits {
    /// Optional hard limit for active shared-memory bytes.
    pub max_bytes: Option<u64>,
}

impl SharedLimits {
    /// Check exact active shared-memory bytes against these limits.
    pub fn check(&self, active_bytes: u64) -> Result<(), SharedLimitError> {
        if let Some(max_bytes) = self.max_bytes
            && active_bytes > max_bytes
        {
            return Err(SharedLimitError {
                used_bytes: active_bytes,
                max_bytes,
            });
        }

        Ok(())
    }

    /// Check active shared-memory bytes after one requested reservation.
    pub fn check_active_reservation(
        &self,
        active_bytes: u64,
        active_reservation: i64,
    ) -> Result<(), SharedLimitError> {
        self.check(apply_reservation(active_bytes, active_reservation))
    }
}

/// One shared-memory hard-limit violation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SharedLimitError {
    /// The exact active shared-memory bytes.
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
