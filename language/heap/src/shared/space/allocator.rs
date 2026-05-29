use std::sync::Arc;

use super::{SharedSmallSpan, SpanList};
use crate::allocator::Slot;
use crate::{AllocationUsage, SharedHeapReference, SmallAllocationPlan, SmallSpanClass};

/// One mutator-local shared allocation cache.
#[derive(Debug)]
pub struct SharedAllocationCache {
    /// The mutator-local small allocation caches.
    pub(super) small: Vec<SmallSizeClassCache>,
}

// SAFETY: runtime worker ownership keeps one shared cache on one worker at a time
unsafe impl Send for SharedAllocationCache {}

/// One mutator-local block cache for one small size class.
#[derive(Debug)]
pub(super) struct SmallSizeClassCache {
    /// The homogeneous payload class allocated by this cache.
    pub(super) class: SmallSpanClass,
    /// The dense allocation cursor for fresh spans.
    pub(super) cursor: SpanCursor,
    /// The shared span table index.
    pub(super) span_index: u32,
    /// The shared span itself.
    pub(super) span: Option<Arc<SharedSmallSpan>>,
    /// The first byte offset inside shared heap space.
    pub(super) first_offset: usize,
    /// The number of slots in this span.
    pub(super) slot_count: usize,
    /// The next never-tried slot for this cache.
    pub(super) next_slot: usize,
}

/// One mutator-local dense shared small block cursor.
#[derive(Debug, Clone, Copy)]
pub(super) struct SpanCursor {
    /// The next byte offset allocated from this cursor.
    pub(super) next_offset: usize,
    /// The next byte offset already published into usage accounting.
    pub(super) accounted_offset: usize,
    /// The byte offset after this cursor.
    pub(super) end_offset: usize,
}

/// One slot reserved from a shared small allocation cache.
#[derive(Debug, Clone, Copy)]
pub(super) struct SmallSlot {
    /// The slot index inside the span.
    pub(super) slot_index: usize,
    /// Whether the slot belongs to the dense cursor.
    pub(super) is_dense: bool,
}

/// One shared small reservation from a mutator-local cache.
#[derive(Debug, Clone, Copy)]
pub(super) struct ReservedSlot {
    /// The allocated small slot.
    pub(super) slot: Slot,
    /// Whether the block came from a dense worker cursor.
    pub(super) is_dense: bool,
    /// Whether the cache still owns usable slots.
    pub(super) keep_cache: bool,
}

impl SharedAllocationCache {
    /// Create one empty mutator-local shared allocation cache.
    pub(super) fn new() -> Self {
        Self { small: Vec::new() }
    }

    /// Ensure the exact small allocation cache entry exists.
    #[inline(always)]
    pub(super) fn ensure_small(&mut self, small: SmallAllocationPlan) {
        let cache_index = small.cache_index();

        while self.small.len() <= cache_index {
            self.small.push(SmallSizeClassCache::inactive());
        }

        if self.small[cache_index].class == SmallSpanClass::EMPTY {
            self.small[cache_index].class = small.class;
        }

        debug_assert_eq!(self.small[cache_index].class, small.class);
    }

    /// Return whether this cache owns one unflushed shared heap reference.
    pub fn contains_heap_reference(&self, reference: SharedHeapReference) -> bool {
        let offset = reference.offset();

        // dense spans are private to the owning mutator until flushed
        for size_class_cache in &self.small {
            if size_class_cache.cursor.end_offset == 0 {
                continue;
            }

            if offset >= size_class_cache.cursor.accounted_offset
                && offset < size_class_cache.cursor.next_offset
            {
                return true;
            }
        }

        false
    }
}

impl SpanCursor {
    /// Return an inactive dense cursor.
    pub(super) const fn inactive() -> Self {
        Self {
            next_offset: 0,
            accounted_offset: 0,
            end_offset: 0,
        }
    }

    /// Reserve one reference from this dense cursor.
    #[inline(always)]
    pub(super) fn reserve_reference(&mut self, size_class: usize) -> Option<SharedHeapReference> {
        // cursor exhausted
        if size_class > self.end_offset - self.next_offset {
            return None;
        }

        // bump the dense cursor
        let reference = SharedHeapReference::new(self.next_offset);
        self.next_offset += size_class;

        Some(reference)
    }

    /// Reserve one slot index from this dense cursor.
    #[inline(always)]
    pub(super) fn reserve_slot(
        &mut self,
        first_offset: usize,
        size_class: usize,
    ) -> Option<SmallSlot> {
        // reserve bytes first, then project back to the slot index
        let reference = self.reserve_reference(size_class)?;
        let slot_index = (reference.offset() - first_offset) / size_class;

        Some(SmallSlot {
            slot_index,
            is_dense: true,
        })
    }

    /// Return the current dense-cursor slot index.
    #[inline(always)]
    pub(super) fn next_slot(&self, first_offset: usize, size_class: usize) -> usize {
        // inactive cursor
        if self.end_offset == 0 {
            return 0;
        }

        (self.next_offset - first_offset) / size_class
    }

    /// Return whether this dense cursor still has one full slot.
    #[inline(always)]
    pub(super) fn has_available_slot(&self, size_class: usize) -> bool {
        size_class <= self.end_offset - self.next_offset
    }

    /// Install one dense cursor.
    pub(super) fn install(&mut self, next_offset: usize, end_offset: usize) {
        self.next_offset = next_offset;
        self.accounted_offset = next_offset;
        self.end_offset = end_offset;
    }

    /// Return the uncommitted usage held by this cursor.
    #[inline(always)]
    pub(super) fn pending_usage(&self, size_class: usize) -> AllocationUsage {
        if self.end_offset == 0 {
            return AllocationUsage::default();
        }

        let allocated_bytes = self.next_offset - self.accounted_offset;
        let allocation_count = allocated_bytes / size_class;

        AllocationUsage::new(allocation_count, allocated_bytes as u64)
    }

    /// Flush uncommitted cursor usage.
    #[inline(always)]
    pub(super) fn flush_usage(&mut self, size_class: usize) -> AllocationUsage {
        let usage = self.pending_usage(size_class);
        self.accounted_offset = self.next_offset;

        usage
    }

    /// Clear this dense cursor.
    pub(super) fn clear(&mut self) {
        self.next_offset = 0;
        self.accounted_offset = 0;
        self.end_offset = 0;
    }
}

impl SmallSizeClassCache {
    /// Return an inactive small allocation cache.
    pub(super) const fn inactive() -> Self {
        Self {
            class: SmallSpanClass::EMPTY,
            cursor: SpanCursor::inactive(),
            span_index: 0,
            span: None,
            first_offset: 0,
            slot_count: 0,
            next_slot: 0,
        }
    }

    /// Reserve one slot from this allocator.
    #[inline(always)]
    pub(super) fn reserve_slot(&mut self) -> Option<SmallSlot> {
        // dense cursor path
        if self.cursor.end_offset > 0 {
            return self
                .cursor
                .reserve_slot(self.first_offset, self.class.size_class);
        }

        // first pass through never-tried slots
        while self.next_slot < self.slot_count {
            let slot_index = self.next_slot;
            self.next_slot += 1;

            // reused spans claim bitmap slots
            let span = self.span.as_ref()?;
            if span.reserve_slot_at(slot_index) {
                return Some(SmallSlot {
                    slot_index,
                    is_dense: false,
                });
            }
        }

        // then use the span free bitmap
        self.span
            .as_ref()?
            .reserve_slot()
            .map(|slot_index| SmallSlot {
                slot_index,
                is_dense: false,
            })
    }

    /// Return whether this cache can still allocate locally.
    #[inline(always)]
    pub(super) fn has_available_slot(&self) -> bool {
        // dense cursor capacity
        if self.cursor.end_offset > 0 {
            return self.cursor.has_available_slot(self.class.size_class);
        }

        // never-tried slots remain
        if self.next_slot < self.slot_count {
            return true;
        }

        // span bitmap may still contain reusable slots
        self.span
            .as_ref()
            .is_some_and(|span| span.occupied_count() < self.slot_count)
    }

    /// Publish allocated cursor slots from this cache.
    #[inline(always)]
    pub(super) fn publish_cursor(&self) {
        // non-dense caches have no cursor state to publish
        if self.cursor.end_offset == 0 {
            return;
        }

        // publish every dense slot reserved so far
        if let Some(span) = &self.span {
            span.publish_dense_len(
                self.cursor
                    .next_slot(self.first_offset, self.class.size_class),
            );
        }
    }

    /// Finish this cache after its last usable slot.
    #[inline(always)]
    pub(super) fn finish(&self) {
        // publish pending dense slots before list transition
        self.publish_cursor();

        // no reusable worker-local slots remain
        if let Some(span) = &self.span {
            span.list.store(SpanList::Full);
        }
    }

    /// Return the heap reference for one slot.
    #[inline(always)]
    pub(super) fn reference_for_slot(&self, slot_index: usize) -> SharedHeapReference {
        let mapping_offset = self.first_offset + self.class.size_class * slot_index;

        SharedHeapReference::new(mapping_offset)
    }

    /// Return whether this cache currently owns a span.
    #[inline(always)]
    pub(super) fn is_active(&self) -> bool {
        self.span.is_some()
    }

    /// Install one shared small span into this cache.
    pub(super) fn install(
        &mut self,
        span_index: usize,
        span: Arc<SharedSmallSpan>,
        use_dense_cursor: bool,
    ) {
        // initialize cache metadata from the span
        let next_slot = span.first_free_slot();

        self.span_index = span_index as u32;
        self.class = span.class;
        self.span = Some(span.clone());
        self.first_offset = span.first_offset;
        self.slot_count = span.slot_count;
        self.next_slot = next_slot;

        // dense spans cover newly mapped spans
        let end_offset = if use_dense_cursor {
            span.first_offset + span.class.size_class * span.slot_count
        } else {
            0
        };
        let next_offset = span.first_offset + span.class.size_class * next_slot;

        // publish cursor bounds to the allocator hot path
        self.cursor.install(next_offset, end_offset);
    }

    /// Clear the current shared small span from this cache.
    pub(super) fn clear(&mut self) {
        // clear cache metadata
        self.span = None;
        self.span_index = 0;
        self.first_offset = 0;
        self.slot_count = 0;
        self.next_slot = 0;

        // clear paired dense cursor
        self.cursor.clear();
    }

    /// Return the span slot for one trusted slot index.
    #[inline(always)]
    pub(super) fn span_slot(&self, slot_index: usize) -> Slot {
        Slot::from_raw(self.span_index, slot_index as u32)
    }
}
