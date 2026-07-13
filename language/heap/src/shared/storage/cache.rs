use std::sync::Arc;

use super::{SmallSpan, SpanList};
use crate::{AllocationUsage, SharedHeapReference, Slot, SmallSpanClass};

/// One mutator-local shared allocation cache.
#[derive(Debug)]
pub struct AllocationCache {
    /// The mutator-local small allocation caches.
    pub(super) small: Vec<SmallSizeClassCache>,
}

/// One mutator-local block cache for one small size class.
#[derive(Debug)]
pub(super) struct SmallSizeClassCache {
    /// The homogeneous payload class allocated by this cache.
    pub(super) class: Option<SmallSpanClass>,
    /// The active allocation cursor.
    pub(super) cursor: SpanCursor,
    /// The shared span table index.
    pub(super) span_index: u32,
    /// The shared span itself.
    pub(super) span: Option<Arc<SmallSpan>>,
    /// The first byte offset inside shared heap storage.
    pub(super) first_offset: usize,
    /// The number of slots in this span.
    pub(super) slot_count: usize,
}

/// One mutator-local shared small span cursor.
#[derive(Debug, Clone, Copy)]
pub(super) enum SpanCursor {
    /// Dense bump allocation over one fresh span.
    Dense {
        /// The next byte offset allocated from this cursor.
        next_offset: usize,
        /// The next byte offset already published into usage accounting.
        accounted_offset: usize,
        /// The byte offset after this cursor.
        end_offset: usize,
    },
    /// Bitmap allocation over one reused span.
    Sparse {
        /// The next never-tried slot for this cursor.
        next_slot: usize,
    },
}

const _: () = assert!(std::mem::size_of::<SpanCursor>() == 4 * std::mem::size_of::<usize>());

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
    pub(super) should_keep_cache: bool,
}

impl AllocationCache {
    /// Create one empty mutator-local shared allocation cache.
    pub(super) fn new() -> Self {
        Self { small: Vec::new() }
    }

    /// Return one small allocation cache entry, growing the table when needed.
    #[inline(always)]
    pub(super) fn ensure_small(&mut self, cache_index: usize) -> &mut SmallSizeClassCache {
        while self.small.len() <= cache_index {
            self.small.push(SmallSizeClassCache::inactive());
        }

        &mut self.small[cache_index]
    }

    /// Return whether this cache owns one unflushed shared heap reference.
    pub fn contains_heap_reference(&self, reference: SharedHeapReference) -> bool {
        let offset = reference.offset();

        // dense spans are private to the owning mutator until flushed
        for size_class_cache in &self.small {
            if size_class_cache.cursor.contains_pending_offset(offset) {
                return true;
            }
        }

        false
    }
}

impl SpanCursor {
    /// Create one dense cursor.
    pub(super) const fn dense(next_offset: usize, end_offset: usize) -> Self {
        Self::Dense {
            next_offset,
            accounted_offset: next_offset,
            end_offset,
        }
    }

    /// Create one sparse cursor.
    pub(super) const fn sparse(next_slot: usize) -> Self {
        Self::Sparse { next_slot }
    }

    /// Return whether this cursor owns one uncommitted offset.
    pub(super) fn contains_pending_offset(self, offset: usize) -> bool {
        match self {
            Self::Dense {
                next_offset,
                accounted_offset,
                ..
            } => offset >= accounted_offset && offset < next_offset,
            Self::Sparse { .. } => false,
        }
    }

    /// Reserve one reference from this dense cursor.
    #[inline(always)]
    pub(super) fn reserve_reference(&mut self, size_class: usize) -> Option<SharedHeapReference> {
        let Self::Dense {
            next_offset,
            end_offset,
            ..
        } = self
        else {
            return None;
        };

        // cursor exhausted
        if size_class > *end_offset - *next_offset {
            return None;
        }

        // bump the dense cursor
        let reference = SharedHeapReference::new(*next_offset);
        *next_offset += size_class;

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
    pub(super) fn dense_len(self, first_offset: usize, size_class: usize) -> Option<usize> {
        match self {
            Self::Dense { next_offset, .. } => Some((next_offset - first_offset) / size_class),
            Self::Sparse { .. } => None,
        }
    }

    /// Return whether this dense cursor still has one full slot.
    #[inline(always)]
    pub(super) fn has_available_slot(&self, size_class: usize) -> bool {
        match self {
            Self::Dense {
                next_offset,
                end_offset,
                ..
            } => size_class <= *end_offset - *next_offset,
            Self::Sparse { .. } => false,
        }
    }

    /// Return the uncommitted usage held by this cursor.
    #[inline(always)]
    pub(super) fn pending_usage(&self, class: SmallSpanClass) -> AllocationUsage {
        let Self::Dense {
            next_offset,
            accounted_offset,
            ..
        } = self
        else {
            return AllocationUsage::default();
        };

        let size_class = class.size_class();
        let allocated_bytes = *next_offset - *accounted_offset;
        let allocation_count = allocated_bytes / size_class;
        AllocationUsage::new(allocation_count, allocated_bytes as u64)
    }

    /// Flush uncommitted cursor usage.
    #[inline(always)]
    pub(super) fn flush_usage(&mut self, class: SmallSpanClass) -> AllocationUsage {
        let usage = self.pending_usage(class);
        if let Self::Dense {
            next_offset,
            accounted_offset,
            ..
        } = self
        {
            *accounted_offset = *next_offset;
        }

        usage
    }

    /// Return the next never-tried sparse slot.
    pub(super) fn take_sparse_slot(&mut self, slot_count: usize) -> Option<usize> {
        let Self::Sparse { next_slot } = self else {
            return None;
        };
        if *next_slot >= slot_count {
            return None;
        }

        let slot_index = *next_slot;
        *next_slot += 1;

        Some(slot_index)
    }

    /// Return whether this sparse cursor still has one never-tried slot.
    pub(super) fn has_sparse_slot(self, slot_count: usize) -> bool {
        matches!(self, Self::Sparse { next_slot } if next_slot < slot_count)
    }
}

impl SmallSizeClassCache {
    /// Return an inactive small allocation cache.
    pub(super) const fn inactive() -> Self {
        Self {
            class: None,
            cursor: SpanCursor::sparse(0),
            span_index: 0,
            span: None,
            first_offset: 0,
            slot_count: 0,
        }
    }

    /// Reserve one slot from this cache.
    #[inline(always)]
    pub(super) fn reserve_slot(&mut self) -> Option<SmallSlot> {
        // dense cursor path
        if matches!(self.cursor, SpanCursor::Dense { .. }) {
            let class = self.class()?;

            return self
                .cursor
                .reserve_slot(self.first_offset, class.size_class());
        }

        // first pass through never-tried slots
        while let Some(slot_index) = self.cursor.take_sparse_slot(self.slot_count) {
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
        if matches!(self.cursor, SpanCursor::Dense { .. }) {
            let Some(class) = self.class() else {
                return false;
            };

            return self.cursor.has_available_slot(class.size_class());
        }

        // never-tried slots remain
        if self.cursor.has_sparse_slot(self.slot_count) {
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
        let Some(class) = self.class() else {
            return;
        };
        let Some(dense_len) = self.cursor.dense_len(self.first_offset, class.size_class()) else {
            return;
        };
        let Some(span) = &self.span else {
            return;
        };

        span.publish_dense_len(dense_len);
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
    pub(super) fn reference_for_slot(&self, slot_index: usize) -> Option<SharedHeapReference> {
        let class = self.class()?;
        let reference_offset = self.first_offset + class.size_class() * slot_index;

        Some(SharedHeapReference::new(reference_offset))
    }

    /// Return whether this cache currently owns a span.
    #[inline(always)]
    pub(super) fn is_active(&self) -> bool {
        self.span.is_some()
    }

    /// Install one shared small span into this cache.
    pub(super) fn install(&mut self, span_index: usize, span: Arc<SmallSpan>, is_dense: bool) {
        // initialize cache metadata from the span
        let next_slot = span.first_free_slot();

        self.span_index = span_index as u32;
        self.class = Some(span.class);
        self.span = Some(span.clone());
        self.first_offset = span.first_offset;
        self.slot_count = span.slot_count;

        // dense spans cover newly mapped spans
        self.cursor = if is_dense {
            let next_offset = span.first_offset + span.class.size_class() * next_slot;
            let end_offset = span.first_offset + span.class.size_class() * span.slot_count;

            SpanCursor::dense(next_offset, end_offset)
        } else {
            SpanCursor::sparse(next_slot)
        };
    }

    /// Clear the current shared small span from this cache.
    pub(super) fn clear(&mut self) {
        // clear cache metadata
        self.class = None;
        self.span = None;
        self.span_index = 0;
        self.first_offset = 0;
        self.slot_count = 0;

        // clear the paired allocation cursor
        self.cursor = SpanCursor::sparse(0);
    }

    /// Return the span slot for one trusted slot index.
    #[inline(always)]
    pub(super) fn span_slot(&self, slot_index: usize) -> Slot {
        Slot::from_raw(self.span_index, slot_index as u32)
    }

    /// Return the small span class when this cache is initialized.
    #[inline(always)]
    pub(super) fn class(&self) -> Option<SmallSpanClass> {
        self.class
    }
}
