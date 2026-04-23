use serde::{Deserialize, Serialize};

use crate::{AccountingRegion, HeapError, HeapResult, apply_byte_delta};

/// Hard limits for one live heap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct HeapLimits {
    /// Optional hard limit for total live heap bytes.
    pub max_bytes: Option<u64>,
    /// Hard limits for the heap space.
    pub heap: HeapSpaceLimits,
    /// Hard limits for the raw space.
    pub raw: RawLimits,
}

/// Hard limits for one live heap space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct HeapSpaceLimits {
    /// Optional hard limit for active heap bytes.
    pub max_bytes: Option<u64>,
}

/// Hard limits for one live raw space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct RawLimits {
    /// Optional hard limit for active raw heap bytes.
    pub max_bytes: Option<u64>,
}

impl HeapLimits {
    /// Check exact heap and raw active bytes against these limits.
    pub fn check(&self, heap_bytes: u64, raw_bytes: u64) -> HeapResult<()> {
        // check heap limit
        if let Some(max_bytes) = self.heap.max_bytes
            && heap_bytes > max_bytes
        {
            return Err(HeapError::LimitExceeded {
                region: AccountingRegion::Heap,
                used_bytes: heap_bytes,
                max_bytes,
            });
        }

        // check raw heap limit
        if let Some(max_bytes) = self.raw.max_bytes
            && raw_bytes > max_bytes
        {
            return Err(HeapError::LimitExceeded {
                region: AccountingRegion::Raw,
                used_bytes: raw_bytes,
                max_bytes,
            });
        }

        // check total heap limit
        let total_bytes =
            heap_bytes
                .checked_add(raw_bytes)
                .ok_or(HeapError::InvariantOverflow {
                    context: "heap limit accounting",
                })?;
        if let Some(max_bytes) = self.max_bytes
            && total_bytes > max_bytes
        {
            return Err(HeapError::TotalLimitExceeded {
                used_bytes: total_bytes,
                max_bytes,
            });
        }

        Ok(())
    }

    /// Check exact heap and raw active bytes after one requested mapped-byte delta.
    pub fn check_mapped_delta(
        &self,
        heap_bytes: u64,
        raw_bytes: u64,
        heap_mapped_delta: i64,
        raw_mapped_delta: i64,
    ) -> HeapResult<()> {
        let heap_bytes = apply_byte_delta(heap_bytes, heap_mapped_delta)?;
        let raw_bytes = apply_byte_delta(raw_bytes, raw_mapped_delta)?;

        self.check(heap_bytes, raw_bytes)
    }
}
