use super::{HeapDomain, HeapError, HeapResult};

/// Exact live allocation totals.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct AllocationTotals {
    /// The number of live allocations.
    allocation_count: usize,
    /// The number of live allocated bytes.
    allocated_bytes: u64,
}

impl AllocationTotals {
    /// Create exact live allocation totals.
    pub(crate) const fn new(allocation_count: usize, allocated_bytes: u64) -> Self {
        Self {
            allocation_count,
            allocated_bytes,
        }
    }

    /// Return the number of live allocations.
    pub(crate) const fn allocation_count(self) -> usize {
        self.allocation_count
    }

    /// Return the number of live allocated bytes.
    pub(crate) const fn allocated_bytes(self) -> u64 {
        self.allocated_bytes
    }

    /// Charge one allocation into these totals.
    pub(crate) fn allocate(&mut self, byte_len: usize, domain: HeapDomain) -> HeapResult<()> {
        let added_bytes = byte_len as u64;
        let allocation_count = self
            .allocation_count
            .checked_add(1)
            .ok_or_else(|| domain.overflow_error(*self, added_bytes))?;
        let allocated_bytes = self
            .allocated_bytes
            .checked_add(added_bytes)
            .ok_or_else(|| domain.overflow_error(*self, added_bytes))?;

        self.allocation_count = allocation_count;
        self.allocated_bytes = allocated_bytes;

        Ok(())
    }

    /// Replace one allocation byte count inside these totals.
    pub(crate) fn resize(
        &mut self,
        previous_len: usize,
        next_len: usize,
        domain: HeapDomain,
    ) -> HeapResult<()> {
        let previous_len = previous_len as u64;
        let next_len = next_len as u64;

        self.check_free(previous_len, domain)?;
        self.allocated_bytes = self
            .allocated_bytes
            .checked_sub(previous_len)
            .ok_or_else(|| domain.invalid_error(*self, previous_len))?
            .checked_add(next_len)
            .ok_or_else(|| domain.overflow_error(*self, next_len))?;

        Ok(())
    }

    /// Check whether these totals can release one allocation.
    pub(crate) fn check_free(&self, freed_bytes: u64, domain: HeapDomain) -> HeapResult<()> {
        if self.allocation_count == 0 || self.allocated_bytes < freed_bytes {
            return Err(domain.invalid_error(*self, freed_bytes));
        }

        Ok(())
    }

    /// Release one allocation from these totals.
    pub(crate) fn free(&mut self, freed_bytes: u64, domain: HeapDomain) -> HeapResult<()> {
        self.check_free(freed_bytes, domain)?;
        self.allocation_count -= 1;
        self.allocated_bytes -= freed_bytes;

        Ok(())
    }
}

impl HeapDomain {
    /// Build one invalid-usage error for this allocation domain.
    fn invalid_error(self, totals: AllocationTotals, freed_bytes: u64) -> HeapError {
        match self {
            Self::Managed => HeapError::InvalidUsage {
                domain: self,
                allocated_count: totals.allocation_count,
                allocated_bytes: totals.allocated_bytes,
                freed_bytes,
            },
            Self::Raw => HeapError::InvalidUsage {
                domain: self,
                allocated_count: totals.allocation_count,
                allocated_bytes: totals.allocated_bytes,
                freed_bytes,
            },
            Self::Shared => HeapError::InvalidUsage {
                domain: self,
                allocated_count: totals.allocation_count,
                allocated_bytes: totals.allocated_bytes,
                freed_bytes,
            },
            Self::Total => unreachable!("allocation totals do not track the total domain"),
        }
    }

    /// Build one overflow error for this allocation domain.
    fn overflow_error(self, totals: AllocationTotals, added_bytes: u64) -> HeapError {
        match self {
            Self::Managed => HeapError::UsageOverflow {
                domain: self,
                allocated_count: totals.allocation_count,
                allocated_bytes: totals.allocated_bytes,
                added_bytes,
            },
            Self::Raw => HeapError::UsageOverflow {
                domain: self,
                allocated_count: totals.allocation_count,
                allocated_bytes: totals.allocated_bytes,
                added_bytes,
            },
            Self::Shared => HeapError::UsageOverflow {
                domain: self,
                allocated_count: totals.allocation_count,
                allocated_bytes: totals.allocated_bytes,
                added_bytes,
            },
            Self::Total => unreachable!("allocation totals do not track the total domain"),
        }
    }
}

/// Return the exact sum of two byte counts.
pub(crate) fn sum_bytes(left: u64, right: u64) -> HeapResult<u64> {
    left.checked_add(right)
        .ok_or(HeapError::ByteCountOverflow { left, right })
}

/// Apply one signed byte delta to one current byte count.
pub(crate) fn apply_byte_delta(current: u64, delta: i64) -> HeapResult<u64> {
    if delta >= 0 {
        return current
            .checked_add(delta as u64)
            .ok_or(HeapError::InvalidByteDelta { current, delta });
    }

    current
        .checked_sub(delta.unsigned_abs())
        .ok_or(HeapError::InvalidByteDelta { current, delta })
}
