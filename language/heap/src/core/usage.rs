use std::fmt::{self, Display, Formatter};

use super::{HeapError, HeapResult};

/// One allocation accounting region.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountingRegion {
    /// One local heap space.
    Heap,
    /// One local raw space.
    Raw,
    /// One shared heap space.
    SharedHeap,
    /// One shared raw space.
    SharedRaw,
    /// Combined heap memory.
    Total,
}

impl AccountingRegion {
    /// Return the usage subject for this region.
    pub(crate) fn subject(self) -> &'static str {
        match self {
            Self::Heap => "local heap",
            Self::Raw => "local raw heap",
            Self::SharedHeap => "shared heap",
            Self::SharedRaw => "shared raw heap",
            Self::Total => "total heap",
        }
    }
}

impl Display for AccountingRegion {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::Heap => "heap",
            Self::Raw => "raw",
            Self::SharedHeap => "shared heap",
            Self::SharedRaw => "shared raw",
            Self::Total => "total",
        };

        write!(formatter, "{label}")
    }
}

/// Exact allocation accounting for one region.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct AllocationUsage {
    /// The number of live allocations.
    allocation_count: usize,
    /// The number of live allocated bytes.
    allocated_bytes: u64,
}

impl AllocationUsage {
    /// Create exact allocation accounting.
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

    /// Charge one allocation into this usage.
    pub(crate) fn allocate(&mut self, byte_len: usize, region: AccountingRegion) -> HeapResult<()> {
        let added_bytes = byte_len as u64;
        let allocation_count = self
            .allocation_count
            .checked_add(1)
            .ok_or_else(|| region.overflow_error(*self, added_bytes))?;
        let allocated_bytes = self
            .allocated_bytes
            .checked_add(added_bytes)
            .ok_or_else(|| region.overflow_error(*self, added_bytes))?;

        self.allocation_count = allocation_count;
        self.allocated_bytes = allocated_bytes;

        Ok(())
    }

    /// Replace one allocation byte count inside this usage.
    pub(crate) fn resize(
        &mut self,
        previous_len: usize,
        next_len: usize,
        region: AccountingRegion,
    ) -> HeapResult<()> {
        let previous_len = previous_len as u64;
        let next_len = next_len as u64;

        self.check_free(previous_len, region)?;
        self.allocated_bytes = self
            .allocated_bytes
            .checked_sub(previous_len)
            .ok_or_else(|| region.invalid_error(*self, previous_len))?
            .checked_add(next_len)
            .ok_or_else(|| region.overflow_error(*self, next_len))?;

        Ok(())
    }

    /// Check whether this usage can release one allocation.
    pub(crate) fn check_free(&self, freed_bytes: u64, region: AccountingRegion) -> HeapResult<()> {
        if self.allocation_count == 0 || self.allocated_bytes < freed_bytes {
            return Err(region.invalid_error(*self, freed_bytes));
        }

        Ok(())
    }

    /// Release one allocation from this usage.
    pub(crate) fn free(&mut self, freed_bytes: u64, region: AccountingRegion) -> HeapResult<()> {
        self.check_free(freed_bytes, region)?;
        self.allocation_count -= 1;
        self.allocated_bytes -= freed_bytes;

        Ok(())
    }
}

impl AccountingRegion {
    /// Build one invalid-usage error for this allocation region.
    fn invalid_error(self, usage: AllocationUsage, freed_bytes: u64) -> HeapError {
        match self {
            Self::Heap | Self::Raw | Self::SharedHeap | Self::SharedRaw => {
                HeapError::InvalidUsage {
                    region: self,
                    allocated_count: usage.allocation_count,
                    allocated_bytes: usage.allocated_bytes,
                    freed_bytes,
                }
            }
            Self::Total => unreachable!("allocation accounting does not track the total space"),
        }
    }

    /// Build one overflow error for this allocation region.
    fn overflow_error(self, _usage: AllocationUsage, _added_bytes: u64) -> HeapError {
        match self {
            Self::Heap => HeapError::InvariantOverflow {
                context: "heap allocation accounting",
            },
            Self::Raw => HeapError::InvariantOverflow {
                context: "raw allocation accounting",
            },
            Self::SharedHeap => HeapError::InvariantOverflow {
                context: "shared heap allocation accounting",
            },
            Self::SharedRaw => HeapError::InvariantOverflow {
                context: "shared raw allocation accounting",
            },
            Self::Total => unreachable!("allocation accounting does not track the total space"),
        }
    }
}

/// Apply one signed byte delta to one current byte count.
pub(crate) fn apply_byte_delta(current: u64, delta: i64) -> HeapResult<u64> {
    if delta >= 0 {
        return current
            .checked_add(delta as u64)
            .ok_or(HeapError::InvariantOverflow {
                context: "byte delta",
            });
    }

    current
        .checked_sub(delta.unsigned_abs())
        .ok_or(HeapError::InvariantOverflow {
            context: "byte delta",
        })
}
