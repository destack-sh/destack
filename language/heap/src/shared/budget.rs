use serde::{Deserialize, Serialize};

use crate::heap::apply_byte_delta;
use crate::{HeapBudget, HeapDomain, HeapError, HeapLimits, HeapResult};

use super::{MemoryContextUsage, SharedSpaceUsage};

/// Combined memory limits across local heap and shared space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct MemoryContextLimits {
    /// The local heap limits.
    pub heap: HeapLimits,
    /// The shared-space limits.
    pub shared: SharedLimits,
}

/// One live admission budget for world-shared space.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SharedBudget {
    /// The configured shared-space limits.
    limits: SharedLimits,
    /// The current active shared-space bytes.
    active_bytes: u64,
}

impl SharedBudget {
    /// Create one shared-space budget from explicit active bytes.
    pub fn new(limits: SharedLimits, active_bytes: u64) -> Self {
        Self {
            limits,
            active_bytes,
        }
    }

    /// Create one shared-space budget from one usage snapshot.
    pub fn from_usage(limits: SharedLimits, usage: SharedSpaceUsage) -> Self {
        Self::new(limits, usage.active_bytes)
    }

    /// Return the configured shared-space limits.
    pub fn limits(&self) -> SharedLimits {
        self.limits
    }

    /// Check one mapped-byte delta without mutating this budget.
    pub fn check_mapped_delta(&self, mapped_delta: i64) -> HeapResult<()> {
        self.limits
            .check_mapped_delta(self.active_bytes, mapped_delta)
    }

    /// Refresh this budget after one committed mutation.
    pub fn refresh(&mut self, active_bytes: u64) {
        self.active_bytes = active_bytes;
    }
}

/// One combined live admission budget across local and shared space.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryContextBudget {
    /// The local heap budget.
    pub heap: HeapBudget,
    /// The shared-space budget.
    pub shared: SharedBudget,
}

impl MemoryContextBudget {
    /// Create one combined memory budget.
    pub fn new(heap: HeapBudget, shared: SharedBudget) -> Self {
        Self { heap, shared }
    }

    /// Create one combined memory budget from one usage snapshot.
    pub fn from_usage(limits: MemoryContextLimits, usage: MemoryContextUsage) -> Self {
        Self {
            heap: HeapBudget::from_usage(limits.heap, usage.heap),
            shared: SharedBudget::from_usage(limits.shared, usage.shared),
        }
    }
}

/// Hard limits for world-shared space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SharedLimits {
    /// Optional hard limit for active shared-space bytes.
    pub max_bytes: Option<u64>,
}

impl SharedLimits {
    /// Check exact active shared-space bytes against these limits.
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

    /// Check active shared-space bytes after one requested mapped-byte delta.
    pub fn check_mapped_delta(&self, active_bytes: u64, mapped_delta: i64) -> HeapResult<()> {
        self.check(apply_byte_delta(active_bytes, mapped_delta)?)
    }
}
