use std::fmt::{self, Display, Formatter};

/// One allocation accounting region.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountingRegion {
    /// One heap space.
    Heap,
    /// One local raw space.
    Raw,
    /// One shared heap space.
    SharedHeap,
    /// One shared raw space.
    SharedRaw,
}

impl AccountingRegion {
    /// Return the usage subject for this region.
    pub(crate) fn subject(self) -> &'static str {
        match self {
            Self::Heap => "heap",
            Self::Raw => "local raw heap",
            Self::SharedHeap => "shared heap",
            Self::SharedRaw => "shared raw heap",
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
    #[inline(always)]
    pub(crate) fn allocate(&mut self, byte_len: usize) {
        self.allocation_count += 1;
        self.allocated_bytes += byte_len as u64;
    }

    /// Replace one allocation byte count inside this usage.
    pub(crate) fn resize(&mut self, previous_len: usize, next_len: usize) {
        let previous_len = previous_len as u64;
        let next_len = next_len as u64;

        self.check_free(previous_len);
        self.allocated_bytes = self.allocated_bytes - previous_len + next_len;
    }

    /// Check whether this usage can release one allocation.
    pub(crate) fn check_free(&self, freed_bytes: u64) {
        debug_assert!(self.allocation_count > 0);
        debug_assert!(self.allocated_bytes >= freed_bytes);
    }

    /// Release one allocation from this usage.
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
