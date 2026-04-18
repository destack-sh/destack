use serde::{Deserialize, Serialize};

use crate::core::apply_byte_delta;
use crate::{HeapDomain, HeapError, HeapResult};

/// One live admission budget for world-shared raw space.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SharedRawBudget {
    /// The configured shared raw-space limits.
    limits: SharedRawLimits,
    /// The current active shared raw-space bytes.
    active_bytes: u64,
}

impl SharedRawBudget {
    /// Create one shared raw-space budget from explicit active bytes.
    pub fn new(limits: SharedRawLimits, active_bytes: u64) -> Self {
        Self {
            limits,
            active_bytes,
        }
    }

    /// Check one mapped-byte delta without mutating this budget.
    pub fn check_mapped_delta(&self, mapped_delta: i64) -> HeapResult<()> {
        self.limits
            .check_mapped_delta(self.active_bytes, mapped_delta)
    }
}

/// Hard limits for world-shared raw space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SharedRawLimits {
    /// Optional hard limit for active shared raw-space bytes.
    pub max_bytes: Option<u64>,
}

impl SharedRawLimits {
    /// Check exact active shared raw-space bytes against these limits.
    pub fn check(&self, active_bytes: u64) -> HeapResult<()> {
        if let Some(max_bytes) = self.max_bytes
            && active_bytes > max_bytes
        {
            return Err(HeapError::LimitExceeded {
                domain: HeapDomain::Shared,
                used_bytes: active_bytes,
                max_bytes,
            });
        }

        Ok(())
    }

    /// Check active shared raw-space bytes after one requested mapped-byte delta.
    pub fn check_mapped_delta(&self, active_bytes: u64, mapped_delta: i64) -> HeapResult<()> {
        self.check(apply_byte_delta(active_bytes, mapped_delta)?)
    }
}

/// Hard limits for one live shared managed space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SharedManagedLimits {
    /// Optional hard limit for active shared managed-space bytes.
    pub max_bytes: Option<u64>,
}

impl SharedManagedLimits {
    /// Check exact active shared managed-space bytes against these limits.
    pub fn check(&self, active_bytes: u64) -> HeapResult<()> {
        if let Some(max_bytes) = self.max_bytes
            && active_bytes > max_bytes
        {
            return Err(HeapError::LimitExceeded {
                domain: HeapDomain::Shared,
                used_bytes: active_bytes,
                max_bytes,
            });
        }

        Ok(())
    }

    /// Check active shared managed-space bytes after one requested mapped-byte delta.
    pub fn check_mapped_delta(&self, active_bytes: u64, mapped_delta: i64) -> HeapResult<()> {
        self.check(apply_byte_delta(active_bytes, mapped_delta)?)
    }
}

/// Hard limits for one live shared heap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SharedHeapLimits {
    /// The managed-space limits.
    pub managed: SharedManagedLimits,
    /// The raw-space limits.
    pub raw: SharedRawLimits,
}
