use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::allocator::{Allocator, Bitmap, PageRun, PageRunCache};

use crate::{HeapReference, HeapResult, SmallSpanClass};

/// One live heap young space.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct YoungSpace {
    /// The configured byte capacity for the young space.
    pub(crate) capacity_bytes: usize,
    /// The fixed page width for young space.
    pub(crate) page_bytes: usize,
    /// The bump-allocation cursor inside the logical young byte space.
    pub(crate) next_offset: usize,
    /// The first byte offset after the materialized young-space prefix.
    pub(crate) mapped_until: usize,
    /// The required alignment for young allocation bases.
    pub(crate) allocation_alignment_bytes: usize,

    /// The allocator pages backing this young space.
    pub(crate) pages: PageRun,
    /// The variable-size young range allocations.
    pub(crate) ranges: Vec<YoungRange>,

    /// The live young-space range bits.
    pub(crate) live: Bitmap,
    /// The marked young-space range bits.
    pub(crate) marked: Bitmap,
    /// The exact local-reference bits across young space.
    pub(crate) local_reference_bits: Bitmap,
    /// The exact shared-reference bits across young space.
    pub(crate) shared_reference_bits: Bitmap,

    /// The fixed-size young runs.
    pub(crate) runs: Vec<YoungRun>,
    /// The fixed-size young run bits.
    pub(crate) run_bits: Vec<YoungRunBits>,
    /// The active run for each exact small-span class.
    pub(crate) run_buckets: BTreeMap<SmallSpanClass, usize>,
    /// The active fixed-size young run cursor.
    pub(crate) run_cursor: YoungRunCursor,
    /// The owning run for each young-space page.
    pub(crate) page_runs: Vec<Option<usize>>,
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
        let page_count = capacity_bytes.div_ceil(page_bytes);

        Ok(Self {
            capacity_bytes,
            page_bytes,
            next_offset: allocation_alignment_bytes,
            mapped_until: 0,
            allocation_alignment_bytes,
            pages: cache.allocate_pages(allocator, capacity_bytes)?,
            ranges: Vec::new(),
            live: Bitmap::with_capacity(0),
            marked: Bitmap::with_capacity(0),
            local_reference_bits: Bitmap::with_capacity(reference_bit_capacity),
            shared_reference_bits: Bitmap::with_capacity(reference_bit_capacity),
            runs: Vec::new(),
            run_bits: Vec::new(),
            run_buckets: BTreeMap::new(),
            run_cursor: YoungRunCursor::inactive(),
            page_runs: vec![None; page_count],
        })
    }

    /// Return the allocated young-space byte prefix.
    pub(crate) fn used_bytes(&self) -> usize {
        self.next_offset - self.allocation_alignment_bytes
    }

    /// Insert one live young range and return its range index.
    #[inline(always)]
    pub(crate) fn push_range(&mut self, first_offset: usize, byte_len: usize) -> usize {
        let range_index = self.ranges.len();
        let range_count = range_index + 1;

        self.live.ensure_capacity(range_count);
        self.marked.ensure_capacity(range_count);
        self.ranges.push(YoungRange {
            first_offset,
            byte_len,
        });
        self.live.set_in_bounds(range_index);
        self.marked.clear_in_bounds(range_index);

        range_index
    }

    /// Return one live young range by range index.
    pub(crate) fn range(&self, range_index: usize) -> Option<YoungRange> {
        if !self.live.contains(range_index) {
            return None;
        }

        self.ranges.get(range_index).copied()
    }

    /// Return one young range record by range index.
    pub(crate) fn range_record(&self, range_index: usize) -> Option<YoungRange> {
        self.ranges.get(range_index).copied()
    }

    /// Return one young range by object base offset.
    pub(crate) fn range_by_offset(&self, first_offset: usize) -> Option<(usize, YoungRange)> {
        let range_index = self.range_index_at_offset(first_offset)?;
        let range = self.range(range_index)?;

        if range.first_offset != first_offset {
            return None;
        }

        Some((range_index, range))
    }

    /// Return one live young range containing one byte offset.
    pub(crate) fn range_at_offset(&self, byte_offset: usize) -> Option<(usize, YoungRange)> {
        let range_index = self.range_index_at_offset(byte_offset)?;
        let range = self.range(range_index)?;

        if byte_offset >= range.first_offset + range.byte_len {
            return None;
        }

        Some((range_index, range))
    }

    /// Return one young range record containing one byte offset.
    pub(crate) fn range_record_at_offset(&self, byte_offset: usize) -> Option<(usize, YoungRange)> {
        let range_index = self.range_index_at_offset(byte_offset)?;
        let range = self.range_record(range_index)?;

        if byte_offset >= range.first_offset + range.byte_len {
            return None;
        }

        Some((range_index, range))
    }

    /// Return every captured young-space range and its live range bits.
    pub(crate) fn image_ranges(&self) -> (Box<[YoungRange]>, Bitmap) {
        (self.ranges.clone().into_boxed_slice(), self.live.clone())
    }

    /// Return one fixed-size young run by index.
    pub(crate) fn run(&self, run_index: usize) -> Option<&YoungRun> {
        self.runs.get(run_index)
    }

    /// Return one fixed-size young run mark bitmap by index.
    pub(crate) fn run_bits(&self, run_index: usize) -> Option<&YoungRunBits> {
        self.run_bits.get(run_index)
    }

    /// Return one fixed-size young run mark bitmap mutably by index.
    pub(crate) fn run_bits_mut(&mut self, run_index: usize) -> Option<&mut YoungRunBits> {
        self.run_bits.get_mut(run_index)
    }

    /// Return the exact next byte offset for one run.
    #[inline(always)]
    pub(crate) fn run_next_offset(&self, run_index: usize) -> Option<usize> {
        if self.run_cursor.is_active() && self.run_cursor.run_index == run_index {
            return Some(self.run_cursor.next_offset);
        }

        Some(self.run(run_index)?.next_offset)
    }

    /// Return the exact reserved slot count for one run.
    #[inline(always)]
    pub(crate) fn run_reserved_slot_count(&self, run_index: usize) -> Option<usize> {
        let run = self.run(run_index)?;
        let next_offset = self.run_next_offset(run_index)?;

        Some(run.reserved_slot_count_with(next_offset))
    }

    /// Flush the active young run cursor into run metadata.
    #[inline(always)]
    pub(crate) fn flush_run_cursor(&mut self) {
        if !self.run_cursor.is_active() {
            return;
        }

        let run_index = self.run_cursor.run_index;
        if let Some(run) = self.runs.get_mut(run_index) {
            run.next_offset = self.run_cursor.next_offset;
        }
    }

    /// Return runs with the active cursor written into the clone.
    pub(crate) fn cloned_runs(&self) -> Vec<YoungRun> {
        let mut runs = self.runs.clone();

        if self.run_cursor.is_active()
            && let Some(run) = runs.get_mut(self.run_cursor.run_index)
        {
            run.next_offset = self.run_cursor.next_offset;
        }

        runs
    }

    /// Install one active young run cursor.
    #[inline(always)]
    pub(crate) fn activate_run_cursor(
        &mut self,
        minimum_byte_len: usize,
        class: SmallSpanClass,
        run_index: usize,
    ) -> Option<()> {
        if self.run_cursor.is_active() && self.run_cursor.run_index == run_index {
            return Some(());
        }

        self.flush_run_cursor();

        let run = self.run(run_index)?;
        self.run_cursor = YoungRunCursor {
            minimum_byte_len,
            class,
            run_index,
            next_offset: run.next_offset,
            end_offset: run.end_offset,
        };

        Some(())
    }

    /// Return the last young range index whose base is at or before one offset.
    fn range_index_at_offset(&self, byte_offset: usize) -> Option<usize> {
        let end = self
            .ranges
            .partition_point(|range| range.first_offset <= byte_offset);

        end.checked_sub(1)
    }
}

/// One live fixed-size run in young space.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct YoungRun {
    /// The first byte offset inside young space.
    pub(crate) first_offset: usize,
    /// The next byte offset allocated from this run.
    pub(crate) next_offset: usize,
    /// The byte offset after this run.
    pub(crate) end_offset: usize,
    /// The homogeneous payload class for this run.
    pub(crate) class: SmallSpanClass,
}

impl YoungRun {
    /// Return the homogeneous payload class for this run.
    pub(crate) const fn class(&self) -> SmallSpanClass {
        self.class
    }

    /// Return the total byte length of this run.
    pub(crate) const fn span_bytes(&self) -> usize {
        self.end_offset - self.first_offset
    }

    /// Return the number of slots in this run.
    pub(crate) const fn slot_count(&self) -> usize {
        self.span_bytes() / self.class.size_class
    }

    /// Return the number of slots reserved through one next offset.
    pub(crate) const fn reserved_slot_count_with(&self, next_offset: usize) -> usize {
        (next_offset - self.first_offset) / self.class.size_class
    }

    /// Return the base byte offset for one slot.
    #[inline(always)]
    pub(crate) fn slot_offset(&self, slot_index: usize) -> usize {
        self.first_offset + slot_index * self.class.size_class
    }
}

/// The active fixed-size young run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct YoungRunCursor {
    /// The smallest payload byte length allocated by this cursor.
    pub(crate) minimum_byte_len: usize,
    /// The fixed slot class allocated by this cursor.
    pub(crate) class: SmallSpanClass,
    /// The run index written back when this cursor changes.
    pub(crate) run_index: usize,
    /// The next byte offset allocated by this cursor.
    pub(crate) next_offset: usize,
    /// The byte offset after this cursor.
    pub(crate) end_offset: usize,
}

impl YoungRunCursor {
    /// Return an inactive young run cursor.
    pub(crate) const fn inactive() -> Self {
        Self {
            minimum_byte_len: 0,
            class: SmallSpanClass::EMPTY,
            run_index: 0,
            next_offset: 0,
            end_offset: 0,
        }
    }

    /// Return whether this cursor currently owns a run.
    #[inline(always)]
    pub(crate) const fn is_active(&self) -> bool {
        self.class.size_class != 0
    }

    /// Return whether this cursor can allocate the requested byte length.
    #[inline(always)]
    pub(crate) fn matches(&self, class: SmallSpanClass, byte_len: usize) -> bool {
        self.class.size_class == class.size_class
            && self.class.span_bytes == class.span_bytes
            && self.class.trace_id == class.trace_id
            && self.class.is_noscan == class.is_noscan
            && self.minimum_byte_len <= byte_len
            && byte_len <= self.class.size_class
    }

    /// Reserve one reference from this cursor.
    #[inline(always)]
    pub(crate) fn reserve_reference(&mut self) -> Option<HeapReference> {
        if self.class.size_class > self.end_offset - self.next_offset {
            return None;
        }

        let reference = HeapReference::new(self.next_offset);
        self.next_offset += self.class.size_class;

        Some(reference)
    }

    /// Reserve one reference when this cursor owns the requested size class.
    #[inline(always)]
    pub(crate) fn reserve_matching_reference(
        &mut self,
        class: SmallSpanClass,
    ) -> Option<HeapReference> {
        if self.class != class {
            return None;
        }

        self.reserve_reference()
    }
}

/// One live byte range in young space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct YoungRange {
    /// The first byte offset inside young space.
    pub(crate) first_offset: usize,
    /// The logical byte length for this allocation.
    pub(crate) byte_len: usize,
}

/// Mark bits for one fixed-size young run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct YoungRunBits {
    /// The live slots retired before the next young reset.
    pub(crate) freed: Bitmap,
    /// The marked slots in this run.
    pub(crate) marked: Bitmap,
}

/// One frozen young-space image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct YoungImage {
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
    /// The captured young-space fixed-size runs.
    runs: Box<[YoungRun]>,
    /// The captured fixed-size young-space run bits.
    run_bits: Box<[YoungRunBits]>,
    /// The live young-space boundary bits.
    live: Bitmap,
    /// The exact local-reference bits across young space.
    local_reference_bits: Bitmap,
    /// The exact shared-reference bits across young space.
    shared_reference_bits: Bitmap,
}

impl YoungImage {
    /// Create one frozen young-space image.
    pub(crate) fn new(
        capacity_bytes: usize,
        page_bytes: usize,
        next_offset: usize,
        allocation_alignment_bytes: usize,
        pages: PageRun,
        ranges: Box<[YoungRange]>,
        runs: Box<[YoungRun]>,
        run_bits: Box<[YoungRunBits]>,
        live: Bitmap,
        local_reference_bits: Bitmap,
        shared_reference_bits: Bitmap,
    ) -> Self {
        Self {
            capacity_bytes,
            page_bytes,
            next_offset,
            allocation_alignment_bytes,
            pages,
            ranges,
            runs,
            run_bits,
            live,
            local_reference_bits,
            shared_reference_bits,
        }
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

    /// Return the allocator pages for this young-space image.
    pub(crate) fn pages(&self) -> &PageRun {
        &self.pages
    }

    /// Return the young-space ranges.
    pub(crate) fn ranges(&self) -> &[YoungRange] {
        &self.ranges
    }

    /// Return the young-space fixed-size runs.
    pub(crate) fn runs(&self) -> &[YoungRun] {
        &self.runs
    }

    /// Return the young-space fixed-size run bits.
    pub(crate) fn run_bits(&self) -> &[YoungRunBits] {
        &self.run_bits
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
