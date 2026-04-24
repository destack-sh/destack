use serde::{Deserialize, Serialize};

use crate::HeapResult;
use crate::allocator::{Allocator, Bitmap, PageRunCache, PageView};

/// One heap young-allocation identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct HeapYoungId {
    /// The young-space generation that owns this allocation.
    generation: u32,
    /// The zero-based allocation index inside that generation.
    index: u32,
}

impl HeapYoungId {
    /// Create one heap young-allocation identifier.
    pub(crate) const fn new(generation: u32, index: u32) -> Self {
        Self { generation, index }
    }

    /// Return the owning young-space generation.
    pub(crate) const fn generation(self) -> u32 {
        self.generation
    }

    /// Return the zero-based young-allocation index.
    pub(crate) const fn index(self) -> u32 {
        self.index
    }
}

/// One live young-allocation record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct YoungAllocation {
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
    pages: PageView,
    /// The captured young-allocation records.
    allocations: Box<[YoungAllocation]>,
    /// The live young-allocation bits.
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
    pub(crate) pages: PageView,
    /// The live young-allocation records.
    pub(crate) allocations: Vec<YoungAllocation>,
    /// The live young-allocation bits.
    pub(crate) live: Bitmap,
    /// The marked young-allocation bits.
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
            next_offset: 0,
            allocation_alignment_bytes,
            pages: cache.allocate_zeroed(allocator, capacity_bytes)?,
            allocations: Vec::new(),
            live: Bitmap::with_capacity(0),
            marked: Bitmap::with_capacity(0),
            local_reference_bits: Bitmap::with_capacity(reference_bit_capacity),
            shared_reference_bits: Bitmap::with_capacity(reference_bit_capacity),
        })
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
        pages: PageView,
        allocations: Box<[YoungAllocation]>,
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
            allocations,
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

    /// Return the required young-allocation alignment.
    pub(crate) const fn allocation_alignment_bytes(&self) -> usize {
        self.allocation_alignment_bytes
    }

    /// Return the allocator pages for this young root.
    pub(crate) fn pages(&self) -> &PageView {
        &self.pages
    }

    /// Return the allocation table.
    pub(crate) fn allocations(&self) -> &[YoungAllocation] {
        &self.allocations
    }

    /// Return the live young-allocation bits.
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
