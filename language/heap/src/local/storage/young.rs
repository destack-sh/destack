use std::mem;

use serde::{Deserialize, Serialize};

use crate::allocator::{Allocator, Bitmap, PageSpan, PageSpanCache};

use crate::{AllocationUsage, HeapReference, HeapResult, SmallSpanClass};

/// One live heap young space.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct YoungSpace {
    /// The configured byte capacity for the young space.
    pub(crate) capacity_bytes: usize,
    /// The fixed page width for young space.
    pub(crate) page_size_bytes: usize,
    /// The bump-block cursor inside the logical young byte space.
    pub(crate) next_offset: usize,
    /// The first byte offset after the materialized young space prefix.
    pub(crate) mapped_until: usize,
    /// The required alignment for young block bases.
    pub(crate) allocation_alignment_bytes: usize,

    /// The allocator pages backing this young space.
    pub(crate) pages: PageSpan,
    /// The variable-size young range blocks.
    pub(crate) ranges: Vec<YoungRange>,

    /// The live young space range bits.
    pub(crate) live: Bitmap,
    /// The marked young space range bits.
    pub(crate) marked: Bitmap,
    /// The exact local-reference bits across young space.
    pub(crate) local_reference_bits: Bitmap,
    /// The exact shared-reference bits across young space.
    pub(crate) shared_reference_bits: Bitmap,

    /// The fixed-size young spans.
    pub(crate) spans: Vec<YoungSpan>,
    /// The fixed-size young span bits.
    pub(crate) span_bits: Vec<YoungSpanBits>,
    /// The reusable span for each exact small allocation cache index.
    pub(crate) span_cache: Vec<Option<usize>>,
    /// The active fixed-size young cursor.
    pub(crate) cursor: Option<YoungCursor>,
    /// The owning span for each young space page.
    pub(crate) page_spans: Vec<Option<usize>>,
    /// The uncommitted usage for variable-size young ranges.
    pub(crate) pending_range_usage: AllocationUsage,
}

impl YoungSpace {
    /// Create one empty young space with its full page span.
    pub(crate) fn new(
        allocator: &Allocator,
        capacity_bytes: usize,
        page_size_bytes: usize,
        allocation_alignment_bytes: usize,
        cache: &mut PageSpanCache,
    ) -> HeapResult<Self> {
        let reference_bit_capacity = capacity_bytes.div_ceil(std::mem::size_of::<usize>());
        let page_count = capacity_bytes.div_ceil(page_size_bytes);

        Ok(Self {
            capacity_bytes,
            page_size_bytes,
            next_offset: allocation_alignment_bytes,
            mapped_until: 0,
            allocation_alignment_bytes,
            pages: cache.allocate_pages(allocator, capacity_bytes)?,
            ranges: Vec::new(),
            live: Bitmap::with_capacity(0),
            marked: Bitmap::with_capacity(0),
            local_reference_bits: Bitmap::with_capacity(reference_bit_capacity),
            shared_reference_bits: Bitmap::with_capacity(reference_bit_capacity),
            spans: Vec::new(),
            span_bits: Vec::new(),
            span_cache: Vec::new(),
            cursor: None,
            page_spans: vec![None; page_count],
            pending_range_usage: AllocationUsage::default(),
        })
    }

    /// Return the allocated young space byte prefix.
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
    pub(crate) fn range_by_offset(&self, first_offset: usize) -> Option<IndexedYoungRange> {
        let range_index = self.range_index_at_offset(first_offset)?;
        let range = self.range(range_index)?;

        if range.first_offset != first_offset {
            return None;
        }

        Some(IndexedYoungRange {
            index: range_index,
            range,
        })
    }

    /// Return one live young range containing one byte offset.
    pub(crate) fn range_at_offset(&self, byte_offset: usize) -> Option<IndexedYoungRange> {
        let range_index = self.range_index_at_offset(byte_offset)?;
        let range = self.range(range_index)?;

        if byte_offset >= range.first_offset + range.byte_len {
            return None;
        }

        Some(IndexedYoungRange {
            index: range_index,
            range,
        })
    }

    /// Return one young range record containing one byte offset.
    pub(crate) fn range_record_at_offset(&self, byte_offset: usize) -> Option<IndexedYoungRange> {
        let range_index = self.range_index_at_offset(byte_offset)?;
        let range = self.range_record(range_index)?;

        if byte_offset >= range.first_offset + range.byte_len {
            return None;
        }

        Some(IndexedYoungRange {
            index: range_index,
            range,
        })
    }

    /// Return every captured young space range and its live range bits.
    pub(crate) fn image_ranges(&self) -> (Box<[YoungRange]>, Bitmap) {
        (self.ranges.clone().into_boxed_slice(), self.live.clone())
    }

    /// Return one fixed-size young span by index.
    pub(crate) fn span(&self, span_index: usize) -> Option<&YoungSpan> {
        self.spans.get(span_index)
    }

    /// Return one fixed-size young span mark bitmap by index.
    pub(crate) fn span_bits(&self, span_index: usize) -> Option<&YoungSpanBits> {
        self.span_bits.get(span_index)
    }

    /// Return one fixed-size young span mark bitmap mutably by index.
    pub(crate) fn span_bits_mut(&mut self, span_index: usize) -> Option<&mut YoungSpanBits> {
        self.span_bits.get_mut(span_index)
    }

    /// Return the exact next byte offset for one span.
    #[inline(always)]
    pub(crate) fn span_next_offset(&self, span_index: usize) -> Option<usize> {
        if let Some(cursor) = self.cursor
            && cursor.span_index == span_index
        {
            return Some(cursor.next_offset);
        }

        Some(self.span(span_index)?.next_offset)
    }

    /// Return the exact reserved slot count for one span.
    #[inline(always)]
    pub(crate) fn span_reserved_slot_count(&self, span_index: usize) -> Option<usize> {
        let span = self.span(span_index)?;
        let next_offset = self.span_next_offset(span_index)?;

        Some(span.reserved_slot_count_with(next_offset))
    }

    /// Return the uncommitted usage held by young allocation cursors.
    #[inline(always)]
    pub(crate) fn pending_usage(&self) -> AllocationUsage {
        let mut usage = self.pending_range_usage;
        let cursor_usage = self
            .cursor
            .map(|cursor| cursor.pending_usage())
            .unwrap_or_default();

        usage.allocate_many(
            cursor_usage.allocation_count(),
            cursor_usage.allocated_bytes(),
        );

        usage
    }

    /// Record one uncommitted variable-size young range.
    #[inline(always)]
    pub(crate) fn record_range_usage(&mut self, byte_len: usize) {
        self.pending_range_usage.allocate(byte_len);
    }

    /// Flush active young usage into exact heap accounting.
    #[inline(always)]
    pub(crate) fn flush_usage(&mut self) -> AllocationUsage {
        let mut usage = mem::take(&mut self.pending_range_usage);

        if let Some(cursor) = &mut self.cursor {
            let cursor_usage = cursor.flush_usage();
            let span_index = cursor.span_index;
            if let Some(span) = self.spans.get_mut(span_index) {
                span.next_offset = cursor.next_offset;
            }

            usage.allocate_many(
                cursor_usage.allocation_count(),
                cursor_usage.allocated_bytes(),
            );
        }

        usage
    }

    /// Return spans with the active cursor written into the clone.
    pub(crate) fn cloned_spans(&self) -> Vec<YoungSpan> {
        let mut spans = self.spans.clone();

        if let Some(cursor) = self.cursor
            && let Some(span) = spans.get_mut(cursor.span_index)
        {
            span.next_offset = cursor.next_offset;
        }

        spans
    }

    /// Install one active young cursor.
    #[inline(always)]
    pub(crate) fn activate_cursor(
        &mut self,
        byte_len: usize,
        class: SmallSpanClass,
        span_index: usize,
    ) -> Option<()> {
        if let Some(cursor) = self.cursor
            && cursor.span_index == span_index
        {
            return cursor.matches(class, byte_len).then_some(());
        }

        let span = self.span(span_index)?;
        if span.byte_len != byte_len || span.class != class {
            return None;
        }

        self.cursor = Some(YoungCursor {
            byte_len,
            class,
            span_index,
            next_offset: span.next_offset,
            accounted_offset: span.next_offset,
            end_offset: span.end_offset,
        });

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

/// One live fixed-size span in young space.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct YoungSpan {
    /// The first byte offset inside young space.
    pub(crate) first_offset: usize,
    /// The exact logical payload byte length for each slot.
    pub(crate) byte_len: usize,
    /// The next byte offset allocated from this span.
    pub(crate) next_offset: usize,
    /// The byte offset after this span.
    pub(crate) end_offset: usize,
    /// The homogeneous payload class for this span.
    pub(crate) class: SmallSpanClass,
}

impl YoungSpan {
    /// Return the homogeneous payload class for this span.
    pub(crate) const fn class(&self) -> SmallSpanClass {
        self.class
    }

    /// Return the exact logical payload byte length for each slot.
    pub(crate) const fn byte_len(&self) -> usize {
        self.byte_len
    }

    /// Return the total byte length of this span.
    pub(crate) const fn span_size_bytes(&self) -> usize {
        self.end_offset - self.first_offset
    }

    /// Return the number of slots in this span.
    pub(crate) const fn slot_count(&self) -> usize {
        self.span_size_bytes() / self.class.size_class
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

/// The active fixed-size young span.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct YoungCursor {
    /// The exact payload byte length allocated by this cursor.
    pub(crate) byte_len: usize,
    /// The fixed slot class allocated by this cursor.
    pub(crate) class: SmallSpanClass,
    /// The span index written back when this cursor changes.
    pub(crate) span_index: usize,
    /// The next byte offset allocated by this cursor.
    pub(crate) next_offset: usize,
    /// The next byte offset already published into usage accounting.
    pub(crate) accounted_offset: usize,
    /// The byte offset after this cursor.
    pub(crate) end_offset: usize,
}

impl YoungCursor {
    /// Return whether this cursor can allocate the requested byte length.
    #[inline(always)]
    pub(crate) fn matches(&self, class: SmallSpanClass, byte_len: usize) -> bool {
        self.class == class && self.byte_len == byte_len
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

    /// Return the uncommitted usage held by this cursor.
    #[inline(always)]
    pub(crate) fn pending_usage(&self) -> AllocationUsage {
        let allocated_bytes = self.next_offset - self.accounted_offset;
        let allocation_count = allocated_bytes / self.class.size_class;

        AllocationUsage::new(allocation_count, allocated_bytes as u64)
    }

    /// Flush uncommitted cursor usage.
    #[inline(always)]
    pub(crate) fn flush_usage(&mut self) -> AllocationUsage {
        let usage = self.pending_usage();
        self.accounted_offset = self.next_offset;

        usage
    }

    /// Reserve one reference when this cursor owns the requested size class.
    #[inline(always)]
    pub(crate) fn reserve_matching_reference(
        &mut self,
        byte_len: usize,
        class: SmallSpanClass,
    ) -> Option<HeapReference> {
        if !self.matches(class, byte_len) {
            return None;
        }

        self.reserve_reference()
    }

    /// Reserve one noscan reference when this cursor owns the requested slot width.
    #[inline(always)]
    pub(crate) fn reserve_noscan_reference(
        &mut self,
        byte_len: usize,
        class: SmallSpanClass,
    ) -> Option<HeapReference> {
        if !class.is_noscan || !self.matches(class, byte_len) {
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
    /// The logical byte length for this block.
    pub(crate) byte_len: usize,
}

/// One young range with its range index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct IndexedYoungRange {
    /// The young range index.
    pub(crate) index: usize,
    /// The young range record.
    pub(crate) range: YoungRange,
}

/// Mark bits for one fixed-size young span.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct YoungSpanBits {
    /// The live slots retired before the next young reset.
    pub(crate) freed: Bitmap,
    /// The marked slots in this span.
    pub(crate) marked: Bitmap,
}

/// One frozen young space image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct YoungImage {
    /// The configured byte capacity for the young space.
    capacity_bytes: usize,
    /// The fixed page width for young space.
    page_size_bytes: usize,
    /// The bump-block cursor inside the logical young byte space.
    next_offset: usize,
    /// The required alignment for young block bases.
    allocation_alignment_bytes: usize,
    /// The captured young space bytes.
    bytes: Box<[u8]>,
    /// The captured young space ranges.
    ranges: Box<[YoungRange]>,
    /// The captured young space fixed-size spans.
    spans: Box<[YoungSpan]>,
    /// The captured fixed-size young space span bits.
    span_bits: Box<[YoungSpanBits]>,
    /// The live young space boundary bits.
    live: Bitmap,
    /// The exact local-reference bits across young space.
    local_reference_bits: Bitmap,
    /// The exact shared-reference bits across young space.
    shared_reference_bits: Bitmap,
}

impl YoungImage {
    /// Create one frozen young space image.
    pub(crate) fn new(
        capacity_bytes: usize,
        page_size_bytes: usize,
        next_offset: usize,
        allocation_alignment_bytes: usize,
        bytes: Box<[u8]>,
        ranges: Box<[YoungRange]>,
        spans: Box<[YoungSpan]>,
        span_bits: Box<[YoungSpanBits]>,
        live: Bitmap,
        local_reference_bits: Bitmap,
        shared_reference_bits: Bitmap,
    ) -> Self {
        Self {
            capacity_bytes,
            page_size_bytes,
            next_offset,
            allocation_alignment_bytes,
            bytes,
            ranges,
            spans,
            span_bits,
            live,
            local_reference_bits,
            shared_reference_bits,
        }
    }

    /// Return the young space byte capacity.
    pub(crate) const fn capacity_bytes(&self) -> usize {
        self.capacity_bytes
    }

    /// Return the young page width.
    pub(crate) const fn page_size_bytes(&self) -> usize {
        self.page_size_bytes
    }

    /// Return the next bump offset.
    pub(crate) const fn next_offset(&self) -> usize {
        self.next_offset
    }

    /// Return the required young space block alignment.
    pub(crate) const fn allocation_alignment_bytes(&self) -> usize {
        self.allocation_alignment_bytes
    }

    /// Return the captured young space bytes.
    pub(crate) fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Return the young space ranges.
    pub(crate) fn ranges(&self) -> &[YoungRange] {
        &self.ranges
    }

    /// Return the young space fixed-size spans.
    pub(crate) fn spans(&self) -> &[YoungSpan] {
        &self.spans
    }

    /// Return the young space fixed-size span bits.
    pub(crate) fn span_bits(&self) -> &[YoungSpanBits] {
        &self.span_bits
    }

    /// Return the live young space boundary bits.
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
