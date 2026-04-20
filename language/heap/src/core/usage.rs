use std::fmt::{self, Display, Formatter};

use super::{HeapError, HeapResult};

/// One heap space.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeapSpace {
    /// One local managed space.
    Managed,
    /// One local raw space.
    Raw,
    /// One shared managed space.
    SharedManaged,
    /// One shared raw space.
    SharedRaw,
    /// Combined heap memory.
    Total,
}

impl HeapSpace {
    /// Return the heap usage subject for this space.
    pub(crate) fn usage_subject(self) -> &'static str {
        match self {
            Self::Managed => "local managed heap",
            Self::Raw => "local raw heap",
            Self::SharedManaged => "shared managed heap",
            Self::SharedRaw => "shared raw heap",
            Self::Total => "total heap",
        }
    }
}

impl Display for HeapSpace {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::Managed => "managed",
            Self::Raw => "raw",
            Self::SharedManaged => "shared managed",
            Self::SharedRaw => "shared raw",
            Self::Total => "total",
        };

        write!(formatter, "{label}")
    }
}

/// Exact allocation accounting for one heap space.
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
    pub(crate) fn allocate(&mut self, byte_len: usize, space: HeapSpace) -> HeapResult<()> {
        let added_bytes = byte_len as u64;
        let allocation_count = self
            .allocation_count
            .checked_add(1)
            .ok_or_else(|| space.overflow_error(*self, added_bytes))?;
        let allocated_bytes = self
            .allocated_bytes
            .checked_add(added_bytes)
            .ok_or_else(|| space.overflow_error(*self, added_bytes))?;

        self.allocation_count = allocation_count;
        self.allocated_bytes = allocated_bytes;

        Ok(())
    }

    /// Replace one allocation byte count inside this usage.
    pub(crate) fn resize(
        &mut self,
        previous_len: usize,
        next_len: usize,
        space: HeapSpace,
    ) -> HeapResult<()> {
        let previous_len = previous_len as u64;
        let next_len = next_len as u64;

        self.check_free(previous_len, space)?;
        self.allocated_bytes = self
            .allocated_bytes
            .checked_sub(previous_len)
            .ok_or_else(|| space.invalid_error(*self, previous_len))?
            .checked_add(next_len)
            .ok_or_else(|| space.overflow_error(*self, next_len))?;

        Ok(())
    }

    /// Check whether this usage can release one allocation.
    pub(crate) fn check_free(&self, freed_bytes: u64, space: HeapSpace) -> HeapResult<()> {
        if self.allocation_count == 0 || self.allocated_bytes < freed_bytes {
            return Err(space.invalid_error(*self, freed_bytes));
        }

        Ok(())
    }

    /// Release one allocation from this usage.
    pub(crate) fn free(&mut self, freed_bytes: u64, space: HeapSpace) -> HeapResult<()> {
        self.check_free(freed_bytes, space)?;
        self.allocation_count -= 1;
        self.allocated_bytes -= freed_bytes;

        Ok(())
    }
}

impl HeapSpace {
    /// Build one invalid-usage error for this allocation space.
    fn invalid_error(self, usage: AllocationUsage, freed_bytes: u64) -> HeapError {
        match self {
            Self::Managed | Self::Raw | Self::SharedManaged | Self::SharedRaw => {
                HeapError::InvalidUsage {
                    space: self,
                    allocated_count: usage.allocation_count,
                    allocated_bytes: usage.allocated_bytes,
                    freed_bytes,
                }
            }
            Self::Total => unreachable!("allocation accounting does not track the total space"),
        }
    }

    /// Build one overflow error for this allocation space.
    fn overflow_error(self, _usage: AllocationUsage, _added_bytes: u64) -> HeapError {
        match self {
            Self::Managed => HeapError::InvariantOverflow {
                context: "managed allocation accounting",
            },
            Self::Raw => HeapError::InvariantOverflow {
                context: "raw allocation accounting",
            },
            Self::SharedManaged => HeapError::InvariantOverflow {
                context: "shared managed allocation accounting",
            },
            Self::SharedRaw => HeapError::InvariantOverflow {
                context: "shared raw allocation accounting",
            },
            Self::Total => unreachable!("allocation accounting does not track the total space"),
        }
    }
}

/// Return the exact sum of two byte counts.
pub(crate) fn sum_bytes(left: u64, right: u64) -> HeapResult<u64> {
    left.checked_add(right).ok_or(HeapError::InvariantOverflow {
        context: "byte sum",
    })
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
