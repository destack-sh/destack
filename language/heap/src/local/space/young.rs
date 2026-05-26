use serde::{Deserialize, Serialize};

use crate::allocator::{Allocator, Bitmap, PageRun, PageRunCache, SpanSlot};
use crate::{HeapReference, HeapResult, SmallSpanClass};

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
    /// The first byte offset after the materialized young-space prefix.
    pub(crate) mapped_until: usize,
    /// The required alignment for young allocation bases.
    pub(crate) allocation_alignment_bytes: usize,
    /// The allocator pages backing this young space.
    pub(crate) pages: PageRun,
    /// The object-start bits across young space.
    pub(crate) starts: Bitmap,
    /// The logical byte length for each object-start index.
    pub(crate) byte_lens: Box<[u32]>,
    /// The live young-space object-start bits.
    pub(crate) live: Bitmap,
    /// The marked young-space object-start bits.
    pub(crate) marked: Bitmap,
    /// The promoted reference for each copied young-space allocation.
    pub(crate) forwarded: Box<[usize]>,
    /// The exact local-reference bits across young space.
    pub(crate) local_reference_bits: Bitmap,
    /// The exact shared-reference bits across young space.
    pub(crate) shared_reference_bits: Bitmap,
    /// The fixed-size no-scan young runs.
    pub(crate) runs: Vec<YoungRun>,
    /// The fixed-size no-scan young run bits.
    pub(crate) run_bits: Vec<YoungRunBits>,
    /// The promoted references for fixed-size no-scan young run slots.
    pub(crate) run_forwarded: Vec<Box<[usize]>>,
    /// The active run for each small no-scan bucket.
    pub(crate) run_buckets: Vec<Option<usize>>,
    /// The active fixed-size no-scan young run cursor.
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
        small_bucket_count: usize,
        cache: &mut PageRunCache,
    ) -> HeapResult<Self> {
        let reference_bit_capacity = capacity_bytes.div_ceil(std::mem::size_of::<usize>());
        let start_bit_capacity = capacity_bytes.div_ceil(allocation_alignment_bytes.max(1));
        let page_count = capacity_bytes.div_ceil(page_bytes);

        Ok(Self {
            generation: 0,
            capacity_bytes,
            page_bytes,
            next_offset: allocation_alignment_bytes,
            mapped_until: 0,
            allocation_alignment_bytes,
            pages: cache.allocate_pages(allocator, capacity_bytes)?,
            starts: Bitmap::with_capacity(start_bit_capacity),
            byte_lens: vec![0; start_bit_capacity].into_boxed_slice(),
            live: Bitmap::with_capacity(start_bit_capacity),
            marked: Bitmap::with_capacity(start_bit_capacity),
            forwarded: vec![0; start_bit_capacity].into_boxed_slice(),
            local_reference_bits: Bitmap::with_capacity(reference_bit_capacity),
            shared_reference_bits: Bitmap::with_capacity(reference_bit_capacity),
            runs: Vec::new(),
            run_bits: Vec::new(),
            run_forwarded: Vec::new(),
            run_buckets: vec![None; small_bucket_count],
            run_cursor: YoungRunCursor::inactive(),
            page_runs: vec![None; page_count],
        })
    }

    /// Return the allocated young-space byte prefix.
    pub(crate) fn used_bytes(&self) -> usize {
        self.next_offset - self.allocation_alignment_bytes
    }

    /// Return whether young space currently holds no allocations.
    pub(crate) fn is_empty(&self) -> bool {
        self.live.count_ones() == 0
    }

    /// Return the young-space start-bit index for one byte offset.
    pub(crate) fn start_index(&self, first_offset: usize) -> usize {
        first_offset / self.allocation_alignment_bytes
    }

    /// Return the byte offset for one young-space start-bit index.
    pub(crate) fn start_offset(&self, start_index: usize) -> usize {
        start_index * self.allocation_alignment_bytes
    }

    /// Return one young range by object-start index.
    pub(crate) fn range(&self, start_index: usize) -> Option<YoungRange> {
        if !self.starts.contains(start_index) || !self.live.contains(start_index) {
            return None;
        }

        let first_offset = self.start_offset(start_index);
        let byte_len = self.byte_lens[start_index] as usize;

        Some(YoungRange {
            first_offset,
            byte_len,
        })
    }

    /// Return one young range by object base offset.
    pub(crate) fn range_by_offset(&self, first_offset: usize) -> Option<(usize, YoungRange)> {
        if !first_offset.is_multiple_of(self.allocation_alignment_bytes) {
            return None;
        }

        let start_index = self.start_index(first_offset);
        let range = self.range(start_index)?;

        Some((start_index, range))
    }

    /// Return every captured young-space range and its live range bits.
    pub(crate) fn image_ranges(&self) -> (Box<[YoungRange]>, Bitmap) {
        let mut ranges = Vec::new();
        let mut live = Bitmap::with_capacity(self.starts.count_ones());
        let mut start = 0;

        while let Some(start_index) = self.starts.first_set_from(start) {
            let first_offset = self.start_offset(start_index);
            let byte_len = self.byte_lens[start_index] as usize;
            let range_index = ranges.len();

            ranges.push(YoungRange {
                first_offset,
                byte_len,
            });

            if self.live.contains(start_index) {
                live.set(range_index);
            }

            start = start_index + 1;
        }

        (ranges.into_boxed_slice(), live)
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

    /// Return the promoted reference for one young-space allocation.
    #[inline(always)]
    pub(crate) fn forwarded_range(&self, start_index: usize) -> Option<HeapReference> {
        let reference = HeapReference::from_bits(*self.forwarded.get(start_index)?);

        if reference.is_null() {
            return None;
        }

        Some(reference)
    }

    /// Record the promoted reference for one young-space allocation.
    #[inline(always)]
    pub(crate) fn forward_range(&mut self, start_index: usize, reference: HeapReference) {
        self.forwarded[start_index] = reference.bits();
    }

    /// Return the promoted reference for one fixed-size young run slot.
    #[inline(always)]
    pub(crate) fn forwarded_run_slot(&self, slot: SpanSlot) -> Option<HeapReference> {
        let run = self.run_forwarded.get(slot.span_index())?;
        let reference = HeapReference::from_bits(*run.get(slot.slot_index())?);

        if reference.is_null() {
            return None;
        }

        Some(reference)
    }

    /// Record the promoted reference for one fixed-size young run slot.
    #[inline(always)]
    pub(crate) fn forward_run_slot(&mut self, slot: SpanSlot, reference: HeapReference) {
        self.run_forwarded[slot.span_index()][slot.slot_index()] = reference.bits();
    }

    /// Clear all transient forwarding references.
    pub(crate) fn clear_forwarding(&mut self) {
        self.forwarded.fill(0);

        for run in &mut self.run_forwarded {
            run.fill(0);
        }
    }

    /// Return empty forwarding tables for fixed-size young run slots.
    pub(crate) fn empty_run_forwarding(runs: &[YoungRun]) -> Vec<Box<[usize]>> {
        runs.iter()
            .map(|run| vec![0; run.slot_count].into_boxed_slice())
            .collect()
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
        size_class: usize,
        run_index: usize,
    ) -> Option<()> {
        if self.run_cursor.is_active() && self.run_cursor.run_index == run_index {
            return Some(());
        }

        self.flush_run_cursor();

        let run = self.run(run_index)?;
        self.run_cursor = YoungRunCursor {
            minimum_byte_len,
            size_class,
            run_index,
            next_offset: run.next_offset,
            end_offset: run.end_offset,
        };

        Some(())
    }
}

/// One live fixed-size no-scan run in young space.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct YoungRun {
    /// The first byte offset inside young space.
    pub(crate) first_offset: usize,
    /// The next byte offset allocated from this run.
    pub(crate) next_offset: usize,
    /// The byte offset after this run.
    pub(crate) end_offset: usize,
    /// The byte length of each slot in this run.
    pub(crate) size_class: usize,
    /// The total byte length of this run.
    pub(crate) span_bytes: usize,
    /// The number of slots in this run.
    pub(crate) slot_count: usize,
}

impl YoungRun {
    /// Return the homogeneous payload class for this run.
    pub(crate) const fn class(&self) -> SmallSpanClass {
        SmallSpanClass {
            size_class: self.size_class,
            span_bytes: self.span_bytes,
            is_noscan: true,
        }
    }

    /// Return the number of slots reserved through one next offset.
    pub(crate) const fn reserved_slot_count_with(&self, next_offset: usize) -> usize {
        (next_offset - self.first_offset) / self.size_class
    }

    /// Return the base byte offset for one slot.
    #[inline(always)]
    pub(crate) fn slot_offset(&self, slot_index: usize) -> usize {
        self.first_offset + slot_index * self.size_class
    }
}

/// The active fixed-size no-scan young run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct YoungRunCursor {
    /// The smallest payload byte length allocated by this cursor.
    pub(crate) minimum_byte_len: usize,
    /// The fixed slot byte length allocated by this cursor.
    pub(crate) size_class: usize,
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
            size_class: 0,
            run_index: 0,
            next_offset: 0,
            end_offset: 0,
        }
    }

    /// Return whether this cursor currently owns a run.
    #[inline(always)]
    pub(crate) const fn is_active(&self) -> bool {
        self.size_class != 0
    }

    /// Return whether this cursor can allocate the requested byte length.
    #[inline(always)]
    pub(crate) const fn matches(&self, byte_len: usize) -> bool {
        self.minimum_byte_len <= byte_len && byte_len <= self.size_class
    }

    /// Reserve one reference from this cursor.
    #[inline(always)]
    pub(crate) fn reserve_reference(&mut self) -> Option<HeapReference> {
        if self.size_class > self.end_offset - self.next_offset {
            return None;
        }

        let reference = HeapReference::new(self.next_offset);
        self.next_offset += self.size_class;

        Some(reference)
    }

    /// Reserve one reference when this cursor owns the requested size class.
    #[inline(always)]
    pub(crate) fn reserve_matching_reference(
        &mut self,
        size_class: usize,
    ) -> Option<HeapReference> {
        if self.size_class != size_class {
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

/// Mark bits for one fixed-size no-scan young run.
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
        generation: u32,
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
            generation,
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
