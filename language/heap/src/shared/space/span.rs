use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU64, AtomicUsize, Ordering};

use destack_mir::ReferenceMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::allocator::{Bitmap, PageRun};
use crate::{
    SmallSpanClass, local_reference_offsets, shared_reference_offsets, slot_reference_map,
};

/// The number of bits in one atomic bitmap word.
const ATOMIC_BITMAP_WORD_BITS: usize = u64::BITS as usize;

/// One frozen shared heap small-span image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SharedHeapSmallSpanImage {
    /// The first byte offset inside shared heap space.
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
    /// The allocator pages for this span.
    pub pages: PageRun,
}

/// One live shared heap span.
#[derive(Debug)]
pub(crate) struct SharedSmallSpan {
    /// The first byte offset inside shared heap space.
    pub(crate) first_offset: usize,
    /// The homogeneous payload class for this span.
    pub(crate) class: SmallSpanClass,
    /// The number of slots in this span.
    pub(crate) slot_count: usize,
    /// The number of occupied slots in this span.
    pub(crate) occupied_count: AtomicUsize,
    /// The number of occupied slots represented as one dense run.
    dense_len: AtomicUsize,
    /// The next likely free slot search cursor.
    pub(crate) free_cursor: AtomicUsize,
    /// The slots claimed by worker-local allocators.
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
    /// The marked slots whose payloads have already been scanned.
    pub(crate) scanned: AtomicBitmap,
    /// Whether this span already has one queued scan work item.
    pub(crate) is_queued_for_scan: AtomicBool,
    /// The allocation list this span belongs to.
    pub(crate) list: AtomicSpanList,
    /// The allocator pages for this span.
    pub(crate) pages: RwLock<PageRun>,
}

/// One shared small-span allocation list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SpanList {
    /// The central partial list.
    Central,
    /// One worker-local cache.
    Worker,
    /// No allocation list because the span has no free slots.
    Full,
    /// No allocation list because the span has no mapped pages.
    Released,
}

impl SharedSmallSpan {
    /// Create one live shared small span.
    pub(crate) fn new(
        first_offset: usize,
        class: SmallSpanClass,
        slot_count: usize,
        pages: PageRun,
        list: SpanList,
    ) -> Self {
        let scan_word_count = class.size_class.div_ceil(std::mem::size_of::<usize>());

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
            scanned: AtomicBitmap::with_capacity(slot_count),
            is_queued_for_scan: AtomicBool::new(false),
            list: AtomicSpanList::new(list),
            pages: RwLock::new(pages),
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
        pages: PageRun,
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
            scanned: AtomicBitmap::with_capacity(slot_count),
            is_queued_for_scan: AtomicBool::new(false),
            list: AtomicSpanList::new(list),
            pages: RwLock::new(pages),
        }
    }

    /// Return the number of occupied slots.
    pub(crate) fn occupied_count(&self) -> usize {
        let dense_len = self.dense_len.load(Ordering::Relaxed);
        let materialized_count = self.occupied_count.load(Ordering::Acquire);

        dense_len + materialized_count
    }

    /// Return whether one slot is occupied.
    pub(crate) fn contains_slot(&self, slot_index: usize) -> bool {
        if slot_index < self.dense_len.load(Ordering::Relaxed) {
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

    /// Return the current page run.
    pub(crate) fn pages(&self) -> PageRun {
        *self.pages.read()
    }

    /// Return whether this span has no mapped pages.
    pub(crate) fn pages_empty(&self) -> bool {
        self.pages.read().is_empty()
    }

    /// Return the number of mapped pages.
    pub(crate) fn page_count(&self) -> usize {
        self.pages.read().len()
    }

    /// Return the first currently reusable slot.
    pub(crate) fn first_free_slot(&self) -> usize {
        self.next_free_slot(0)
    }

    /// Replace the current page run.
    pub(crate) fn set_pages(&self, pages: PageRun) {
        *self.pages.write() = pages;
    }

    /// Take the current page run.
    pub(crate) fn take_pages(&self) -> PageRun {
        std::mem::replace(&mut *self.pages.write(), PageRun::empty())
    }

    /// Try to reserve one free slot.
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
        if slot_index < self.dense_len.load(Ordering::Relaxed) {
            return false;
        }

        self.reserved.try_set(slot_index)
    }

    /// Publish initialized dense-run slots.
    #[inline(always)]
    pub(crate) fn publish_dense_len(&self, dense_len: usize) {
        // make initialized bytes visible through the dense run
        self.dense_len.store(dense_len, Ordering::Relaxed);
    }

    /// Publish one initialized reserved slot.
    pub(crate) fn publish_slot(&self, slot_index: usize) {
        // keep reusable-slot discovery consistent after dense runs
        self.reserved.set(slot_index);

        // make initialized bytes visible to scanners
        if self.occupied.try_set(slot_index) {
            self.occupied_count.fetch_add(1, Ordering::AcqRel);
        }

        // clear stale cycle state from any prior occupant
        self.marked.clear(slot_index);
        self.scanned.clear(slot_index);

        // advance the allocator cursor past this slot
        self.free_cursor
            .store(self.next_free_slot(slot_index + 1), Ordering::Release);
    }

    /// Release one occupied slot.
    pub(crate) fn release_slot(&self, slot_index: usize) -> bool {
        self.materialize_dense_run();

        // reject double frees and stale releases
        if !self.occupied.try_clear(slot_index) {
            return false;
        }

        // clear all per slot metadata
        self.reserved.clear(slot_index);
        self.needs_zero.set(slot_index);
        self.marked.clear(slot_index);
        self.scanned.clear(slot_index);
        self.clear_reference_bits(slot_index);

        // publish the newly reusable slot
        self.occupied_count.fetch_sub(1, Ordering::AcqRel);
        self.free_cursor.fetch_min(slot_index, Ordering::AcqRel);

        true
    }

    /// Write exact reference bits for one occupied slot.
    pub(crate) fn write_reference_bits(&self, slot_index: usize, reference_map: &ReferenceMap) {
        if !reference_map.has_reference() {
            return;
        }

        // replace stale metadata from a previous occupant
        self.clear_reference_bits(slot_index);

        // encode both edge classes into side bitmaps
        self.write_reference_offsets(slot_index, reference_map, true);
        self.write_reference_offsets(slot_index, reference_map, false);
    }

    /// Return whether one reserved slot must be cleared before zeroed reuse.
    pub(crate) fn take_needs_zero(&self, slot_index: usize) -> bool {
        self.needs_zero.try_clear(slot_index)
    }

    /// Mark one reserved slot as fully initialized.
    pub(crate) fn clear_needs_zero(&self, slot_index: usize) {
        self.needs_zero.clear(slot_index);
    }

    /// Return exact reference metadata for one occupied slot.
    pub(crate) fn reference_map(&self, slot_index: usize) -> ReferenceMap {
        // snapshot both edge classes consistently enough for tracing
        let local_reference_bits = self.local_reference_bits.snapshot();
        let shared_reference_bits = self.shared_reference_bits.snapshot();

        slot_reference_map(
            &local_reference_bits,
            &shared_reference_bits,
            slot_index,
            self.class.size_class,
            self.class.size_class,
        )
    }

    /// Clear every mark and scan bit in this span.
    pub(crate) fn clear_marks(&self) {
        // reset collector metadata for a new cycle
        self.marked.clear_all();
        self.scanned.clear_all();
        self.is_queued_for_scan.store(false, Ordering::Release);
    }

    /// Return the occupied bitmap as an image bitmap.
    pub(crate) fn occupied_snapshot(&self) -> Bitmap {
        let mut occupied = self.occupied.snapshot();
        let dense_len = self.dense_len.load(Ordering::Relaxed);

        // dense-run slots are live but not represented in the occupied bitmap
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

    /// Return the next free slot from a start offset.
    fn next_free_slot(&self, start: usize) -> usize {
        let start = start.max(self.dense_len.load(Ordering::Relaxed));

        self.reserved
            .first_clear_from(start)
            .unwrap_or(self.slot_count)
    }

    /// Move dense-run slots into the occupied bitmap.
    fn materialize_dense_run(&self) {
        let dense_len = self.dense_len.swap(0, Ordering::AcqRel);
        if dense_len == 0 {
            return;
        }

        // record each dense-run slot in the bitmap form
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
        let bit_len = self.class.size_class.div_ceil(std::mem::size_of::<usize>());
        let bit_start = slot_index * bit_len;

        // clear both edge classes over the slot payload width
        self.local_reference_bits.clear_range(bit_start, bit_len);
        self.shared_reference_bits.clear_range(bit_start, bit_len);
    }

    /// Write one reference kind into exact slot bits.
    fn write_reference_offsets(
        &self,
        slot_index: usize,
        reference_map: &ReferenceMap,
        is_local: bool,
    ) {
        let offsets = if is_local {
            local_reference_offsets(reference_map)
        } else {
            shared_reference_offsets(reference_map)
        };

        self.write_direct_reference_offsets(slot_index, &offsets, is_local);
    }

    /// Write direct reference offsets into exact slot bits.
    fn write_direct_reference_offsets(&self, slot_index: usize, offsets: &[u32], is_local: bool) {
        for offset in offsets {
            self.set_reference_bit(slot_index, *offset as usize, is_local);
        }
    }

    /// Set one reference bit inside one slot.
    fn set_reference_bit(&self, slot_index: usize, byte_offset: usize, is_local: bool) {
        let word_bytes = std::mem::size_of::<usize>();
        let bit_len = self.class.size_class.div_ceil(word_bytes);
        let bit_index = slot_index * bit_len + byte_offset / word_bytes;

        if is_local {
            self.local_reference_bits.set(bit_index);
        } else {
            self.shared_reference_bits.set(bit_index);
        }
    }
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
            Self::Released => 3,
        }
    }

    /// Decode one stored span-list byte.
    fn from_bits(bits: u8) -> Self {
        match bits {
            0 => Self::Central,
            1 => Self::Worker,
            2 => Self::Full,
            3 => Self::Released,
            _ => {
                debug_assert!(false, "invalid shared small-span list byte");

                Self::Released
            }
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
        let atomic = Self::with_capacity(bitmap.capacity());

        for bit_index in 0..atomic.capacity {
            if bitmap.contains(bit_index) {
                atomic.set(bit_index);
            }
        }

        atomic
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
    pub(crate) fn clear_all(&self) {
        for word in &self.words {
            word.store(0, Ordering::Release);
        }
    }

    /// Clear every bit inside the given range.
    pub(crate) fn clear_range(&self, start: usize, len: usize) {
        for bit_index in start..start + len {
            self.clear(bit_index);
        }
    }

    /// Return the first clear bit from the given offset.
    pub(crate) fn first_clear_from(&self, start: usize) -> Option<usize> {
        (start..self.capacity).find(|bit_index| !self.contains(*bit_index))
    }

    /// Return a bitmap snapshot.
    pub(crate) fn snapshot(&self) -> Bitmap {
        let mut bitmap = Bitmap::with_capacity(self.capacity);

        for bit_index in 0..self.capacity {
            if self.contains(bit_index) {
                bitmap.set(bit_index);
            }
        }

        bitmap
    }
}
