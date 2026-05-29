use std::fmt::{self, Display, Formatter};

/// One block accounting region.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountingRegion {
    /// All heap regions together.
    Total,
    /// One heap space.
    Heap,
    /// One shared heap space.
    SharedHeap,
}

impl AccountingRegion {
    /// Return the usage subject for this region.
    pub(crate) fn subject(self) -> &'static str {
        match self {
            Self::Total => "total heap",
            Self::Heap => "heap",
            Self::SharedHeap => "shared heap",
        }
    }
}

impl Display for AccountingRegion {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::Total => "total",
            Self::Heap => "heap",
            Self::SharedHeap => "shared heap",
        };

        write!(formatter, "{label}")
    }
}

/// Exact block accounting for one region.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct AllocationUsage {
    /// The number of live blocks.
    allocation_count: usize,
    /// The number of live allocated bytes.
    allocated_bytes: u64,
}

impl AllocationUsage {
    /// Create exact block accounting.
    pub(crate) const fn new(allocation_count: usize, allocated_bytes: u64) -> Self {
        Self {
            allocation_count,
            allocated_bytes,
        }
    }

    /// Return the number of live blocks.
    pub(crate) const fn allocation_count(self) -> usize {
        self.allocation_count
    }

    /// Return the number of live allocated bytes.
    pub(crate) const fn allocated_bytes(self) -> u64 {
        self.allocated_bytes
    }

    /// Charge one block into this usage.
    #[inline(always)]
    pub(crate) fn allocate(&mut self, byte_len: usize) {
        self.allocation_count += 1;
        self.allocated_bytes += byte_len as u64;
    }

    /// Charge multiple blocks into this usage.
    #[inline(always)]
    pub(crate) fn allocate_many(&mut self, allocation_count: usize, allocated_bytes: u64) {
        self.allocation_count += allocation_count;
        self.allocated_bytes += allocated_bytes;
    }

    /// Check whether this usage can release one block.
    pub(crate) fn check_free(&self, freed_bytes: u64) {
        debug_assert!(self.allocation_count > 0);
        debug_assert!(self.allocated_bytes >= freed_bytes);
    }

    /// Release one block from this usage.
    pub(crate) fn free(&mut self, freed_bytes: u64) {
        self.check_free(freed_bytes);
        self.allocation_count -= 1;
        self.allocated_bytes -= freed_bytes;
    }
}

/// Apply one signed byte delta to one current byte count.
pub(crate) fn apply_byte_delta(current: u64, delta: i64) -> u64 {
    if delta >= 0 {
        return current + delta as u64;
    }

    let delta = delta.unsigned_abs();
    debug_assert!(current >= delta);

    current - delta
}
