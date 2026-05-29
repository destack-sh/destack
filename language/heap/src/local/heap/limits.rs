use serde::{Deserialize, Serialize};

use crate::{AccountingRegion, HeapError, HeapResult, apply_byte_delta};

/// Hard limits for one live heap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct HeapLimits {
    /// Optional hard limit for total live heap bytes.
    pub max_bytes: Option<u64>,
    /// Optional hard limit for retained heap bytes.
    pub retained_bytes: Option<u64>,
}

impl HeapLimits {
    /// Check exact heap retained bytes against these limits.
    pub fn check(&self, heap_bytes: u64) -> HeapResult<()> {
        // check heap limit
        if let Some(max_bytes) = self.retained_bytes
            && heap_bytes > max_bytes
        {
            return Err(HeapError::LimitExceeded {
                region: AccountingRegion::Heap,
                used_bytes: heap_bytes,
                max_bytes,
            });
        }

        // check total heap limit
        if let Some(max_bytes) = self.max_bytes
            && heap_bytes > max_bytes
        {
            return Err(HeapError::LimitExceeded {
                region: AccountingRegion::Total,
                used_bytes: heap_bytes,
                max_bytes,
            });
        }

        Ok(())
    }

    /// Check exact heap retained bytes after one requested retained-byte delta.
    pub fn check_retained_byte_delta(
        &self,
        heap_bytes: u64,
        heap_retained_byte_delta: i64,
    ) -> HeapResult<()> {
        let heap_bytes = apply_byte_delta(heap_bytes, heap_retained_byte_delta);

        self.check(heap_bytes)
    }
}
