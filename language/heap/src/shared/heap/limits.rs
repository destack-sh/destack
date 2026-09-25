use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

use crate::{AccountingRegion, HeapError, HeapResult, apply_byte_delta};

/// Hard limits for one live shared heap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, Reflect)]
pub struct SharedHeapLimits {
    /// Optional hard limit for total retained shared-heap bytes.
    pub max_bytes: Option<u64>,
    /// Optional hard limit for retained shared heap bytes.
    pub retained_bytes: Option<u64>,
}

impl SharedHeapLimits {
    /// Check exact retained shared heap bytes against these limits.
    pub fn check(&self, retained_bytes: u64) -> HeapResult<()> {
        if let Some(max_bytes) = self.retained_bytes
            && retained_bytes > max_bytes
        {
            return Err(HeapError::LimitExceeded {
                region: AccountingRegion::SharedHeap,
                used_bytes: retained_bytes,
                max_bytes,
            });
        }

        if let Some(max_bytes) = self.max_bytes
            && retained_bytes > max_bytes
        {
            return Err(HeapError::LimitExceeded {
                region: AccountingRegion::Total,
                used_bytes: retained_bytes,
                max_bytes,
            });
        }

        Ok(())
    }

    /// Check retained shared heap bytes after one requested retained-byte delta.
    pub fn check_retained_byte_delta(
        &self,
        retained_bytes: u64,
        retained_byte_delta: i64,
    ) -> HeapResult<()> {
        self.check(apply_byte_delta(retained_bytes, retained_byte_delta))
    }
}
