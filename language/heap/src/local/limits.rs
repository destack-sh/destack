use serde::{Deserialize, Serialize};

use crate::{HeapError, HeapResult, HeapSpace, apply_byte_delta, sum_bytes};

/// Hard limits for one live heap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct HeapLimits {
    /// Optional hard limit for total live heap bytes.
    pub max_bytes: Option<u64>,
    /// Hard limits for the managed space.
    pub managed: ManagedLimits,
    /// Hard limits for the raw space.
    pub raw: RawLimits,
}

/// Hard limits for one live managed space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ManagedLimits {
    /// Optional hard limit for active managed heap bytes.
    pub max_bytes: Option<u64>,
}

/// Hard limits for one live raw space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct RawLimits {
    /// Optional hard limit for active raw heap bytes.
    pub max_bytes: Option<u64>,
}

impl HeapLimits {
    /// Check exact managed and raw active bytes against these limits.
    pub fn check(&self, managed_bytes: u64, raw_bytes: u64) -> HeapResult<()> {
        // check managed heap limit
        if let Some(max_bytes) = self.managed.max_bytes
            && managed_bytes > max_bytes
        {
            return Err(HeapError::LimitExceeded {
                space: HeapSpace::Managed,
                used_bytes: managed_bytes,
                max_bytes,
            });
        }

        // check raw heap limit
        if let Some(max_bytes) = self.raw.max_bytes
            && raw_bytes > max_bytes
        {
            return Err(HeapError::LimitExceeded {
                space: HeapSpace::Raw,
                used_bytes: raw_bytes,
                max_bytes,
            });
        }

        // check total heap limit
        let total_bytes = sum_bytes(managed_bytes, raw_bytes)?;
        if let Some(max_bytes) = self.max_bytes
            && total_bytes > max_bytes
        {
            return Err(HeapError::LimitExceeded {
                space: HeapSpace::Total,
                used_bytes: total_bytes,
                max_bytes,
            });
        }

        Ok(())
    }

    /// Check exact managed and raw active bytes after one requested mapped-byte delta.
    pub fn check_mapped_delta(
        &self,
        managed_bytes: u64,
        raw_bytes: u64,
        managed_mapped_delta: i64,
        raw_mapped_delta: i64,
    ) -> HeapResult<()> {
        let managed_bytes = apply_byte_delta(managed_bytes, managed_mapped_delta)?;
        let raw_bytes = apply_byte_delta(raw_bytes, raw_mapped_delta)?;

        self.check(managed_bytes, raw_bytes)
    }
}
