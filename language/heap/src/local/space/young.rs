use serde::{Deserialize, Serialize};

use crate::HeapResult;
use crate::allocator::{Allocator, Bitmap, PageRun, PageRunCache};

/// One live byte range in young space.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct YoungRange {
    /// The first byte offset inside young space.
    pub(crate) first_offset: usize,
    /// The logical byte length for this allocation.
    pub(crate) byte_len: usize,
}

/// One frozen young-space root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct YoungImage {
    /// The generation number for this young space.
    generation: u32,
    /// The configured byte capacity for the young space.
    capacity_bytes: usize,
    /// The fixed page width for young space.
    page_bytes: usize,
    /// The bump-allocation cursor inside the logical young byte space.
    next_offset: usize,
    /// The required alignment for young allocation bases.
    allocation_alignment_bytes: usize,
    /// The allocator pages backing this young space.
    pages: PageRun,
    /// The captured young-space ranges.
    ranges: Box<[YoungRange]>,
    /// The live young-space boundary bits.
    live: Bitmap,
    /// The exact local-reference bits across young space.
    local_reference_bits: Bitmap,
    /// The exact shared-reference bits across young space.
    shared_reference_bits: Bitmap,
}

/// One live heap young space.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct YoungSpace {
    /// The generation number for this young space.
    pub(crate) generation: u32,
    /// The configured byte capacity for the young space.
    pub(crate) capacity_bytes: usize,
    /// The fixed page width for young space.
    pub(crate) page_bytes: usize,
    /// The bump-allocation cursor inside the logical young byte space.
    pub(crate) next_offset: usize,
    /// The required alignment for young allocation bases.
    pub(crate) allocation_alignment_bytes: usize,
    /// The allocator pages backing this young space.
    pub(crate) pages: PageRun,
    /// The live young-space ranges.
    pub(crate) ranges: Vec<YoungRange>,
    /// The live young-space boundary bits.
    pub(crate) live: Bitmap,
    /// The marked young-space boundary bits.
    pub(crate) marked: Bitmap,
    /// The exact local-reference bits across young space.
    pub(crate) local_reference_bits: Bitmap,
    /// The exact shared-reference bits across young space.
    pub(crate) shared_reference_bits: Bitmap,
}

impl YoungSpace {
    /// Create one empty young space with its full page run.
    pub(crate) fn new(
        allocator: &Allocator,
        capacity_bytes: usize,
        page_bytes: usize,
        allocation_alignment_bytes: usize,
        cache: &mut PageRunCache,
    ) -> HeapResult<Self> {
        let reference_bit_capacity = capacity_bytes.div_ceil(std::mem::size_of::<usize>());

        Ok(Self {
            generation: 0,
            capacity_bytes,
            page_bytes,
            next_offset: allocation_alignment_bytes,
            allocation_alignment_bytes,
            pages: cache.allocate_pages(allocator, capacity_bytes)?,
            ranges: Vec::new(),
            live: Bitmap::with_capacity(0),
            marked: Bitmap::with_capacity(0),
            local_reference_bits: Bitmap::with_capacity(reference_bit_capacity),
            shared_reference_bits: Bitmap::with_capacity(reference_bit_capacity),
        })
    }

    /// Return the allocated young-space byte prefix.
    pub(crate) fn used_bytes(&self) -> usize {
        self.next_offset - self.allocation_alignment_bytes
    }

    /// Return whether young space currently holds no allocations.
    pub(crate) fn is_empty(&self) -> bool {
        self.ranges.is_empty()
    }

    /// Return whether young space has crossed one occupancy threshold.
    pub(crate) fn should_collect(&self, trigger_percent: u32) -> bool {
        if self.capacity_bytes == 0 {
            return false;
        }

        let used_bytes = self.used_bytes() as u128;
        let capacity_bytes = self.capacity_bytes as u128;
        let trigger_percent = u128::from(trigger_percent);

        used_bytes * 100 >= capacity_bytes * trigger_percent
    }
}

impl YoungImage {
    /// Create one frozen young-space root.
    pub(crate) fn new(
        generation: u32,
        capacity_bytes: usize,
        page_bytes: usize,
        next_offset: usize,
        allocation_alignment_bytes: usize,
        pages: PageRun,
        ranges: Box<[YoungRange]>,
        live: Bitmap,
        local_reference_bits: Bitmap,
        shared_reference_bits: Bitmap,
    ) -> Self {
        Self {
            generation,
            capacity_bytes,
            page_bytes,
            next_offset,
            allocation_alignment_bytes,
            pages,
            ranges,
            live,
            local_reference_bits,
            shared_reference_bits,
        }
    }

    /// Return the generation number.
    pub(crate) const fn generation(&self) -> u32 {
        self.generation
    }

    /// Return the young-space byte capacity.
    pub(crate) const fn capacity_bytes(&self) -> usize {
        self.capacity_bytes
    }

    /// Return the young page width.
    pub(crate) const fn page_bytes(&self) -> usize {
        self.page_bytes
    }

    /// Return the next bump offset.
    pub(crate) const fn next_offset(&self) -> usize {
        self.next_offset
    }

    /// Return the required young-space allocation alignment.
    pub(crate) const fn allocation_alignment_bytes(&self) -> usize {
        self.allocation_alignment_bytes
    }

    /// Return the allocator pages for this young root.
    pub(crate) fn pages(&self) -> &PageRun {
        &self.pages
    }

    /// Return the young-space ranges.
    pub(crate) fn ranges(&self) -> &[YoungRange] {
        &self.ranges
    }

    /// Return the live young-space boundary bits.
    pub(crate) fn live(&self) -> &Bitmap {
        &self.live
    }

    /// Return the local-reference bits.
    pub(crate) fn local_reference_bits(&self) -> &Bitmap {
        &self.local_reference_bits
    }

    /// Return the shared-reference bits.
    pub(crate) fn shared_reference_bits(&self) -> &Bitmap {
        &self.shared_reference_bits
    }
}
