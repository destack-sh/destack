use std::sync::atomic::{AtomicU8, AtomicU64, AtomicUsize, Ordering};

use destack_memory::MemoryRange;
use destack_mir::TraceMap;
use destack_serde::Reflect;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use crate::{
    Bitmap, HeapReference, ReferenceRange, SharedHeapReference, SmallSpanClass,
    visit_static_reference_offsets,
};

/// The number of bits in one atomic bitmap word.
const ATOMIC_BITMAP_WORD_BITS: usize = u64::BITS as usize;

/// One live shared heap span.
#[derive(Debug)]
pub(crate) struct SmallSpan {
    /// The first byte offset inside shared heap storage.
    pub(crate) first_offset: usize,
    /// The homogeneous payload class for this span.
    pub(crate) class: SmallSpanClass,
    /// The number of slots in this span.
    pub(crate) slot_count: usize,
    /// The number of occupied slots in this span.
    pub(crate) occupied_count: AtomicUsize,
    /// The number of occupied slots represented as one dense span.
    dense_len: AtomicUsize,
    /// The next likely free slot search cursor.
    pub(crate) free_cursor: AtomicUsize,
    /// The slots claimed by worker-local caches.
    reserved: AtomicBitmap,
    /// The occupied slots in this span.
    pub(crate) occupied: AtomicBitmap,
    /// The free slots that must be cleared before zeroed reuse.
    needs_zero: AtomicBitmap,
    /// The exact local-reference bits for each occupied slot.
    pub(crate) local_reference_bits: AtomicBitmap,
    /// The exact shared-reference bits for each occupied slot.
    pub(crate) shared_reference_bits: AtomicBitmap,
    /// The marked slots in this span.
    pub(crate) marked: AtomicBitmap,
    /// The slots a borrow or heap storage retained, whose release waits for the collector.
    pub(crate) retained: AtomicBitmap,
    /// The released slots whose values moved out, freed by the collector without their drop plan.
    pub(crate) empty: AtomicBitmap,
    /// The marked slots whose payloads have already been scanned.
    scanned: AtomicBitmap,
    /// The next marked slot candidate to scan.
    scan_cursor: AtomicUsize,
    /// The mark epoch represented by the collector bitmaps.
    mark_epoch: AtomicU64,
    /// The lazy mark reset lock.
    mark_reset: Mutex<()>,
    /// The block list this span belongs to.
    pub(crate) list: AtomicSpanList,
    /// The memory pages for this span.
    pub(crate) pages: MemoryRange,
}

impl SmallSpan {
    /// Create one live shared small span.
    pub(crate) fn new(
        first_offset: usize,
        class: SmallSpanClass,
        slot_count: usize,
        pages: MemoryRange,
        list: SpanList,
    ) -> Self {
        let scan_word_count = class.size_class().div_ceil(std::mem::size_of::<usize>());

        Self {
            first_offset,
            class,
            slot_count,
            occupied_count: AtomicUsize::new(0),
            dense_len: AtomicUsize::new(0),
            free_cursor: AtomicUsize::new(0),
            reserved: AtomicBitmap::with_capacity(slot_count),
            occupied: AtomicBitmap::with_capacity(slot_count),
            needs_zero: AtomicBitmap::with_capacity(slot_count),
            local_reference_bits: AtomicBitmap::with_capacity(slot_count * scan_word_count),
            shared_reference_bits: AtomicBitmap::with_capacity(slot_count * scan_word_count),
            marked: AtomicBitmap::with_capacity(slot_count),
            retained: AtomicBitmap::with_capacity(slot_count),
            empty: AtomicBitmap::with_capacity(slot_count),
            scanned: AtomicBitmap::with_capacity(slot_count),
            scan_cursor: AtomicUsize::new(0),
            mark_epoch: AtomicU64::new(0),
            mark_reset: Mutex::new(()),
            list: AtomicSpanList::new(list),
            pages,
        }
    }

    /// Restore one live shared small span from an image.
    pub(crate) fn from_image(
        first_offset: usize,
        class: SmallSpanClass,
        slot_count: usize,
        occupied: &Bitmap,
        local_reference_bits: &Bitmap,
        shared_reference_bits: &Bitmap,
        retained: &Bitmap,
        empty: &Bitmap,
        pages: MemoryRange,
        list: SpanList,
    ) -> Self {
        let occupied_count = occupied.count_ones();
        let free_cursor = occupied.first_clear_from(0).unwrap_or(slot_count);

        Self {
            first_offset,
            class,
            slot_count,
            occupied_count: AtomicUsize::new(occupied_count),
            dense_len: AtomicUsize::new(0),
            free_cursor: AtomicUsize::new(free_cursor),
            reserved: AtomicBitmap::from_bitmap(occupied),
            occupied: AtomicBitmap::from_bitmap(occupied),
            needs_zero: AtomicBitmap::free_slots_from_occupied(occupied, slot_count),
            local_reference_bits: AtomicBitmap::from_bitmap(local_reference_bits),
            shared_reference_bits: AtomicBitmap::from_bitmap(shared_reference_bits),
            marked: AtomicBitmap::with_capacity(slot_count),
            retained: AtomicBitmap::from_bitmap(retained),
            empty: AtomicBitmap::from_bitmap(empty),
            scanned: AtomicBitmap::with_capacity(slot_count),
            scan_cursor: AtomicUsize::new(0),
            mark_epoch: AtomicU64::new(0),
            mark_reset: Mutex::new(()),
            list: AtomicSpanList::new(list),
            pages,
        }
    }

    /// Return the number of occupied slots.
    pub(crate) fn occupied_count(&self) -> usize {
        let dense_len = self.dense_len.load(Ordering::Acquire);
        let materialized_count = self.occupied_count.load(Ordering::Acquire);

        dense_len + materialized_count
    }

    /// Return whether one slot is occupied.
    pub(crate) fn contains_slot(&self, slot_index: usize) -> bool {
        if slot_index < self.dense_len.load(Ordering::Acquire) {
            return true;
        }

        self.occupied.contains(slot_index)
    }

    /// Reset the free slot cursor from the current reserved bitmap.
    pub(crate) fn reset_free_cursor(&self) {
        self.free_cursor
            .store(self.next_free_slot(0), Ordering::Release);
    }

    /// Reset the free slot cursor to the first slot.
    pub(crate) fn reset_free_cursor_to_start(&self) {
        self.free_cursor.store(0, Ordering::Release);
    }

    /// Return the current page span.
    pub(crate) fn pages(&self) -> MemoryRange {
        self.pages
    }

    /// Return the mapped byte length.
    pub(crate) fn mapped_byte_len(&self) -> usize {
        self.pages.byte_len
    }

    /// Return the first currently reusable slot.
    pub(crate) fn first_free_slot(&self) -> usize {
        self.next_free_slot(0)
    }

    /// Reserve one free slot if available.
    pub(crate) fn reserve_slot(&self) -> Option<usize> {
        let mut slot_index = self.free_cursor.load(Ordering::Acquire);

        // claim the first available unpublished slot
        while slot_index < self.slot_count {
            if self.reserved.try_set(slot_index) {
                self.free_cursor
                    .store(self.next_free_slot(slot_index + 1), Ordering::Release);

                return Some(slot_index);
            }

            slot_index += 1;
        }

        // mark the cursor exhausted
        self.free_cursor.store(self.slot_count, Ordering::Release);

        None
    }

    /// Reserve one known slot.
    pub(crate) fn reserve_slot_at(&self, slot_index: usize) -> bool {
        if slot_index >= self.slot_count {
            return false;
        }
        if slot_index < self.dense_len.load(Ordering::Acquire) {
            return false;
        }

        self.reserved.try_set(slot_index)
    }

    /// Publish initialized dense-span slots.
    #[inline(always)]
    pub(crate) fn publish_dense_len(&self, dense_len: usize) {
        // make initialized bytes visible through the dense span
        self.dense_len.store(dense_len, Ordering::Release);
    }

    /// Publish one initialized reserved slot.
    pub(crate) fn publish_slot(&self, slot_index: usize) {
        // keep reusable-slot discovery consistent after dense spans
        self.reserved.set(slot_index);

        // make initialized bytes visible to scanners
        if self.occupied.try_set(slot_index) {
            self.occupied_count.fetch_add(1, Ordering::AcqRel);
        }

        // clear stale cycle state from any prior occupant
        self.marked.clear(slot_index);
        self.scanned.clear(slot_index);

        // advance the memory cursor past this slot
        self.free_cursor
            .store(self.next_free_slot(slot_index + 1), Ordering::Release);
    }

    /// Release one occupied slot.
    pub(crate) fn release_slot(&self, slot_index: usize) -> bool {
        self.materialize_dense_cursor();

        // reject double frees and stale releases
        if !self.occupied.try_clear(slot_index) {
            return false;
        }

        // clear all per slot metadata
        self.reserved.clear(slot_index);
        self.needs_zero.set(slot_index);
        self.marked.clear(slot_index);
        self.scanned.clear(slot_index);
        self.retained.clear(slot_index);
        self.empty.clear(slot_index);
        self.clear_reference_bits(slot_index);

        // publish the newly reusable slot
        self.occupied_count.fetch_sub(1, Ordering::AcqRel);
        self.free_cursor.fetch_min(slot_index, Ordering::AcqRel);

        true
    }

    /// Write exact reference bits for one occupied slot.
    pub(crate) fn write_reference_bits(&self, slot_index: usize, trace_map: &TraceMap) {
        debug_assert!(!trace_map.has_variant_reference());

        if self.class.trace_id().is_some() {
            return;
        }

        if !trace_map.has_heap_reference() {
            return;
        }

        // encode both edge classes into side bitmaps
        self.write_local_reference_offsets(slot_index, trace_map);
        self.write_shared_reference_offsets(slot_index, trace_map);
    }

    /// Return whether one reserved slot must be cleared before zeroed reuse.
    pub(crate) fn take_needs_zero(&self, slot_index: usize) -> bool {
        self.needs_zero.try_clear(slot_index)
    }

    /// Mark one reserved slot as fully initialized.
    pub(crate) fn clear_needs_zero(&self, slot_index: usize) {
        self.needs_zero.clear(slot_index);
    }

    /// Return whether one slot is marked in one cycle.
    pub(crate) fn is_marked(&self, slot_index: usize, epoch: u64) -> bool {
        if self.mark_epoch.load(Ordering::Acquire) != epoch {
            return false;
        }

        self.marked.contains(slot_index)
    }

    /// Mark one slot for one cycle and return whether it was newly marked.
    pub(crate) fn mark_slot(&self, slot_index: usize, epoch: u64) -> bool {
        self.ensure_mark_epoch(epoch);

        let is_new = self.marked.try_set(slot_index);
        if is_new {
            self.scan_cursor.fetch_min(slot_index, Ordering::AcqRel);
        }

        is_new
    }

    /// Claim one marked slot for scanning in one cycle.
    pub(crate) fn claim_marked_slot(&self, slot_index: usize, epoch: u64) -> bool {
        if self.mark_epoch.load(Ordering::Acquire) != epoch {
            return false;
        }

        if !self.marked.contains(slot_index) {
            return false;
        }

        self.scanned.try_set(slot_index)
    }

    /// Claim the next marked slot for scanning in one cycle.
    pub(crate) fn claim_next_marked_slot(&self, epoch: u64) -> Option<usize> {
        if self.mark_epoch.load(Ordering::Acquire) != epoch {
            return None;
        }

        loop {
            let start = self.scan_cursor.load(Ordering::Acquire);
            let slot_index = self.marked.first_set_from(start)?;
            let next_slot_index = slot_index + 1;
            if self
                .scan_cursor
                .compare_exchange(start, next_slot_index, Ordering::AcqRel, Ordering::Acquire)
                .is_err()
            {
                continue;
            }

            if self.claim_marked_slot(slot_index, epoch) {
                return Some(slot_index);
            }
        }
    }

    /// Ensure collector bitmaps represent one mark epoch.
    fn ensure_mark_epoch(&self, epoch: u64) {
        if self.mark_epoch.load(Ordering::Acquire) == epoch {
            return;
        }

        let _reset = self.mark_reset.lock();
        if self.mark_epoch.load(Ordering::Acquire) == epoch {
            return;
        }

        self.marked.clear_all();
        self.scanned.clear_all();
        self.scan_cursor.store(0, Ordering::Release);
        self.mark_epoch.store(epoch, Ordering::Release);
    }

    /// Return the occupied bitmap as an image bitmap.
    pub(crate) fn occupied_snapshot(&self) -> Bitmap {
        let mut occupied = self.occupied.snapshot();
        let dense_len = self.dense_len.load(Ordering::Acquire);

        // dense-span slots are live but not represented in the occupied bitmap
        for slot_index in 0..dense_len {
            occupied.set(slot_index);
        }

        occupied
    }

    /// Return the local-reference bitmap as an image bitmap.
    pub(crate) fn local_reference_snapshot(&self) -> Bitmap {
        self.local_reference_bits.snapshot()
    }

    /// Return the shared-reference bitmap as an image bitmap.
    pub(crate) fn shared_reference_snapshot(&self) -> Bitmap {
        self.shared_reference_bits.snapshot()
    }

    /// Return the retained bitmap as an image bitmap.
    pub(crate) fn retained_snapshot(&self) -> Bitmap {
        self.retained.snapshot()
    }

    /// Return the empty bitmap as an image bitmap.
    pub(crate) fn empty_snapshot(&self) -> Bitmap {
        self.empty.snapshot()
    }

    /// Return the next free slot from a start offset.
    fn next_free_slot(&self, start: usize) -> usize {
        let start = start.max(self.dense_len.load(Ordering::Acquire));

        self.reserved
            .first_clear_from(start)
            .unwrap_or(self.slot_count)
    }

    /// Move dense-span slots into the occupied bitmap.
    fn materialize_dense_cursor(&self) {
        let dense_len = self.dense_len.swap(0, Ordering::AcqRel);
        if dense_len == 0 {
            return;
        }

        // record each dense-span slot in the bitmap form
        let mut materialized_count = 0usize;
        for slot_index in 0..dense_len {
            if self.reserved.try_set(slot_index) {
                self.occupied.set(slot_index);
                materialized_count += 1;
            }
        }

        // preserve usage accounting after the representation switch
        self.occupied_count
            .fetch_add(materialized_count, Ordering::AcqRel);
        self.free_cursor
            .store(self.next_free_slot(dense_len), Ordering::Release);
    }

    /// Clear exact reference bits for one slot.
    fn clear_reference_bits(&self, slot_index: usize) {
        let bit_len = self
            .class
            .size_class()
            .div_ceil(std::mem::size_of::<usize>());
        let bit_start = slot_index * bit_len;

        // clear both edge classes over the slot payload width
        self.local_reference_bits.clear_range(bit_start, bit_len);
        self.shared_reference_bits.clear_range(bit_start, bit_len);
    }

    /// Write local reference offsets into exact slot bits.
    fn write_local_reference_offsets(&self, slot_index: usize, trace_map: &TraceMap) {
        visit_static_reference_offsets::<HeapReference>(
            trace_map,
            ReferenceRange::All,
            &mut |offset| self.set_local_reference_bit(slot_index, offset),
        );
    }

    /// Write shared reference offsets into exact slot bits.
    fn write_shared_reference_offsets(&self, slot_index: usize, trace_map: &TraceMap) {
        visit_static_reference_offsets::<SharedHeapReference>(
            trace_map,
            ReferenceRange::All,
            &mut |offset| self.set_shared_reference_bit(slot_index, offset),
        );
    }

    /// Set one local-reference bit inside one slot.
    fn set_local_reference_bit(&self, slot_index: usize, byte_offset: usize) {
        let bit_index = self.reference_bit_index(slot_index, byte_offset);

        self.local_reference_bits.set(bit_index);
    }

    /// Set one shared-reference bit inside one slot.
    fn set_shared_reference_bit(&self, slot_index: usize, byte_offset: usize) {
        let bit_index = self.reference_bit_index(slot_index, byte_offset);

        self.shared_reference_bits.set(bit_index);
    }

    /// Return one reference bit index inside this span.
    fn reference_bit_index(&self, slot_index: usize, byte_offset: usize) -> usize {
        let word_bytes = std::mem::size_of::<usize>();
        let bit_len = self.class.size_class().div_ceil(word_bytes);

        slot_index * bit_len + byte_offset / word_bytes
    }
}

/// One frozen shared heap small-span image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub(crate) struct SmallSpanImage {
    /// The first byte offset inside shared heap storage.
    pub first_offset: usize,
    /// The homogeneous payload class for this span.
    pub class: SmallSpanClass,
    /// The number of slots in this span.
    pub slot_count: usize,
    /// The occupied slots in this span.
    pub occupied: Bitmap,
    /// The exact local-reference bits for each occupied slot.
    pub local_reference_bits: Bitmap,
    /// The exact shared-reference bits for each occupied slot.
    pub shared_reference_bits: Bitmap,
    /// The slots a borrow or heap storage retained.
    pub retained: Bitmap,
    /// The released slots whose values moved out.
    pub empty: Bitmap,
}

/// One shared small-span block list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SpanList {
    /// The central partial list.
    Central,
    /// One worker-local cache.
    Worker,
    /// No block list because the span has no free slots.
    Full,
}

/// One atomic span-list value.
#[derive(Debug)]
pub(crate) struct AtomicSpanList {
    /// The stored span-list byte.
    bits: AtomicU8,
}

impl AtomicSpanList {
    /// Create one atomic span list.
    pub(crate) const fn new(list: SpanList) -> Self {
        Self {
            bits: AtomicU8::new(list.bits()),
        }
    }

    /// Load the current span list.
    pub(crate) fn load(&self) -> SpanList {
        SpanList::from_bits(self.bits.load(Ordering::Acquire))
    }

    /// Store the current span list.
    pub(crate) fn store(&self, list: SpanList) {
        self.bits.store(list.bits(), Ordering::Release);
    }
}

impl SpanList {
    /// Return the stored span-list byte.
    const fn bits(self) -> u8 {
        match self {
            Self::Central => 0,
            Self::Worker => 1,
            Self::Full => 2,
        }
    }

    /// Decode one stored span-list byte.
    fn from_bits(bits: u8) -> Self {
        match bits {
            0 => Self::Central,
            1 => Self::Worker,
            2 => Self::Full,
            _ => unreachable!("invalid shared small-span list byte: {bits}"),
        }
    }
}

/// One atomic bitmap for shared span metadata.
#[derive(Debug)]
pub(crate) struct AtomicBitmap {
    /// The logical bit capacity.
    capacity: usize,
    /// The packed bitmap words.
    words: Vec<AtomicU64>,
}

impl AtomicBitmap {
    /// Create one empty atomic bitmap.
    fn with_capacity(capacity: usize) -> Self {
        let word_count = capacity.div_ceil(ATOMIC_BITMAP_WORD_BITS);
        let words = (0..word_count).map(|_| AtomicU64::new(0)).collect();

        Self { capacity, words }
    }

    /// Create one atomic bitmap from a bitmap snapshot.
    fn from_bitmap(bitmap: &Bitmap) -> Self {
        let words = bitmap
            .words()
            .iter()
            .map(|word| AtomicU64::new(*word))
            .collect();

        Self {
            capacity: bitmap.capacity(),
            words,
        }
    }

    /// Create one bitmap for every currently free slot.
    fn free_slots_from_occupied(occupied: &Bitmap, slot_count: usize) -> Self {
        let atomic = Self::with_capacity(slot_count);

        for slot_index in 0..slot_count {
            if !occupied.contains(slot_index) {
                atomic.set(slot_index);
            }
        }

        atomic
    }

    /// Return whether one bit is set.
    pub(crate) fn contains(&self, offset: usize) -> bool {
        if offset >= self.capacity {
            return false;
        }

        let word_index = offset / ATOMIC_BITMAP_WORD_BITS;
        let bit_offset = offset % ATOMIC_BITMAP_WORD_BITS;
        let mask = 1_u64 << bit_offset;

        self.words[word_index].load(Ordering::Acquire) & mask != 0
    }

    /// Set one bit.
    pub(crate) fn set(&self, offset: usize) {
        if offset >= self.capacity {
            return;
        }

        let word_index = offset / ATOMIC_BITMAP_WORD_BITS;
        let bit_offset = offset % ATOMIC_BITMAP_WORD_BITS;
        let mask = 1_u64 << bit_offset;

        self.words[word_index].fetch_or(mask, Ordering::AcqRel);
    }

    /// Try to set one clear bit.
    pub(crate) fn try_set(&self, offset: usize) -> bool {
        if offset >= self.capacity {
            return false;
        }

        let word_index = offset / ATOMIC_BITMAP_WORD_BITS;
        let bit_offset = offset % ATOMIC_BITMAP_WORD_BITS;
        let mask = 1_u64 << bit_offset;
        let previous = self.words[word_index].fetch_or(mask, Ordering::AcqRel);

        previous & mask == 0
    }

    /// Clear one bit.
    pub(crate) fn clear(&self, offset: usize) {
        if offset >= self.capacity {
            return;
        }

        let word_index = offset / ATOMIC_BITMAP_WORD_BITS;
        let bit_offset = offset % ATOMIC_BITMAP_WORD_BITS;
        let mask = !(1_u64 << bit_offset);

        self.words[word_index].fetch_and(mask, Ordering::AcqRel);
    }

    /// Try to clear one set bit.
    pub(crate) fn try_clear(&self, offset: usize) -> bool {
        if offset >= self.capacity {
            return false;
        }

        let word_index = offset / ATOMIC_BITMAP_WORD_BITS;
        let bit_offset = offset % ATOMIC_BITMAP_WORD_BITS;
        let mask = 1_u64 << bit_offset;
        let previous = self.words[word_index].fetch_and(!mask, Ordering::AcqRel);

        previous & mask != 0
    }

    /// Clear all bits.
    fn clear_all(&self) {
        for word in &self.words {
            word.store(0, Ordering::Release);
        }
    }

    /// Clear every bit inside the given range.
    pub(crate) fn clear_range(&self, start: usize, len: usize) {
        if len == 0 || start >= self.capacity {
            return;
        }

        // mask whole words across the range
        let end = (start + len).min(self.capacity);
        let start_word_index = start / ATOMIC_BITMAP_WORD_BITS;
        let end_word_index = (end - 1) / ATOMIC_BITMAP_WORD_BITS;
        for word_index in start_word_index..=end_word_index {
            // keep bits outside the range inside boundary words
            let word_start = word_index * ATOMIC_BITMAP_WORD_BITS;
            let from_bit = start
                .saturating_sub(word_start)
                .min(ATOMIC_BITMAP_WORD_BITS);
            let until_bit = (end - word_start).min(ATOMIC_BITMAP_WORD_BITS);
            let range_mask = range_bit_mask(from_bit, until_bit);

            self.words[word_index].fetch_and(!range_mask, Ordering::AcqRel);
        }
    }

    /// Return the first clear bit from the given offset.
    pub(crate) fn first_clear_from(&self, start: usize) -> Option<usize> {
        if start >= self.capacity {
            return None;
        }

        // scan whole words for the first zero bit
        let mut word_index = start / ATOMIC_BITMAP_WORD_BITS;
        let bit_offset = start % ATOMIC_BITMAP_WORD_BITS;
        let mut word = self.words[word_index].load(Ordering::Acquire) | low_bit_mask(bit_offset);

        loop {
            // report the first zero bit inside this word
            let available_bits = !word;
            if available_bits != 0 {
                let first_bit = available_bits.trailing_zeros() as usize;
                let bit_index = word_index * ATOMIC_BITMAP_WORD_BITS + first_bit;

                return (bit_index < self.capacity).then_some(bit_index);
            }

            // advance to the next word
            word_index += 1;
            if word_index >= self.words.len() {
                return None;
            }

            word = self.words[word_index].load(Ordering::Acquire);
        }
    }

    /// Return the first set bit from the given offset.
    fn first_set_from(&self, start: usize) -> Option<usize> {
        if start >= self.capacity {
            return None;
        }

        let mut word_index = start / ATOMIC_BITMAP_WORD_BITS;
        let bit_offset = start % ATOMIC_BITMAP_WORD_BITS;
        let mut word = self.words[word_index].load(Ordering::Acquire) & !low_bit_mask(bit_offset);

        loop {
            if word != 0 {
                let first_bit = word.trailing_zeros() as usize;
                let bit_index = word_index * ATOMIC_BITMAP_WORD_BITS + first_bit;

                return (bit_index < self.capacity).then_some(bit_index);
            }

            word_index += 1;
            if word_index >= self.words.len() {
                return None;
            }

            word = self.words[word_index].load(Ordering::Acquire);
        }
    }

    /// Return a bitmap snapshot.
    pub(crate) fn snapshot(&self) -> Bitmap {
        let words = self.words.iter().map(|word| word.load(Ordering::Acquire));

        Bitmap::from_words(self.capacity, words)
    }
}

/// Return one mask covering the half-open bit range inside one word.
fn range_bit_mask(from_bit: usize, until_bit: usize) -> u64 {
    low_bit_mask(until_bit) & !low_bit_mask(from_bit)
}

/// Return one mask with every low bit below the offset set.
fn low_bit_mask(bit_offset: usize) -> u64 {
    if bit_offset >= ATOMIC_BITMAP_WORD_BITS {
        u64::MAX
    } else if bit_offset == 0 {
        0
    } else {
        (1_u64 << bit_offset) - 1
    }
}
