use serde::{Deserialize, Serialize};

use crate::{AccountingRegion, HeapError, HeapResult, apply_byte_delta};

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
    pub fn check_mapped_byte_delta(&self, mapped_byte_delta: i64) -> HeapResult<()> {
        self.limits
            .check_mapped_byte_delta(self.active_bytes, mapped_byte_delta)
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
                region: AccountingRegion::SharedRaw,
                used_bytes: active_bytes,
                max_bytes,
            });
        }

        Ok(())
    }

    /// Check active shared raw-space bytes after one requested mapped-byte delta.
    pub fn check_mapped_byte_delta(
        &self,
        active_bytes: u64,
        mapped_byte_delta: i64,
    ) -> HeapResult<()> {
        self.check(apply_byte_delta(active_bytes, mapped_byte_delta)?)
    }
}

/// Hard limits for one live shared heap space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SharedHeapSpaceLimits {
    /// Optional hard limit for active shared heap-space bytes.
    pub max_bytes: Option<u64>,
}

impl SharedHeapSpaceLimits {
    /// Check exact active shared heap-space bytes against these limits.
    pub fn check(&self, active_bytes: u64) -> HeapResult<()> {
        if let Some(max_bytes) = self.max_bytes
            && active_bytes > max_bytes
        {
            return Err(HeapError::LimitExceeded {
                region: AccountingRegion::SharedHeap,
                used_bytes: active_bytes,
                max_bytes,
            });
        }

        Ok(())
    }

    /// Check active shared heap-space bytes after one requested mapped-byte delta.
    pub fn check_mapped_byte_delta(
        &self,
        active_bytes: u64,
        mapped_byte_delta: i64,
    ) -> HeapResult<()> {
        self.check(apply_byte_delta(active_bytes, mapped_byte_delta)?)
    }
}

/// Hard limits for one live shared heap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SharedHeapLimits {
    /// Optional hard limit for total retained shared-heap bytes.
    pub max_bytes: Option<u64>,
    /// The heap-space limits.
    pub heap: SharedHeapSpaceLimits,
    /// The raw-space limits.
    pub raw: SharedRawLimits,
}
