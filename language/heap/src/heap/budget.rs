use super::{Heap, HeapLimits, HeapUsage, ManagedLimits, RawLimits};
use crate::HeapResult;

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

    /// Check one mapped-byte delta without mutating this budget.
    pub fn check_mapped_delta(
        &self,
        managed_mapped_delta: i64,
        raw_mapped_delta: i64,
    ) -> HeapResult<()> {
        self.limits.check_mapped_delta(
            self.managed.active_bytes,
            self.raw.active_bytes,
            managed_mapped_delta,
            raw_mapped_delta,
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
    pub fn set_limits(&mut self, limits: HeapLimits) -> HeapResult<()> {
        self.limits = limits;
        self.check_limits()
    }

    /// Check the configured heap hard limits against current usage.
    pub fn check_limits(&self) -> HeapResult<()> {
        self.budget().check_mapped_delta(0, 0)
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
