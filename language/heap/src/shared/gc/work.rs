use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};

use parking_lot::Mutex;

use crate::SharedHeapReference;
use crate::allocator::Bitmap;

/// One queued unit of shared mark work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SharedTraceWork {
    /// One shared small span with pending slots to trace.
    SmallSpan(usize),
    /// One shared large reference to trace directly.
    Reference(SharedHeapReference),
}

/// One pending shared small-span mark state.
#[derive(Debug)]
pub(crate) struct SharedSmallSpanWork {
    /// The number of slots in this span.
    slot_count: usize,
    /// The pending slots whose references still need tracing.
    pending_slots: Mutex<Bitmap>,
    /// The queue gate for this span.
    is_queued: AtomicU8,
}

impl SharedSmallSpanWork {
    /// Create one empty small-span work state.
    pub(crate) fn new(slot_count: usize) -> Self {
        Self {
            slot_count,
            pending_slots: Mutex::new(Bitmap::with_capacity(slot_count)),
            is_queued: AtomicU8::new(0),
        }
    }

    /// Return whether this state matches the given slot count.
    pub(crate) fn matches_slot_count(&self, slot_count: usize) -> bool {
        self.slot_count == slot_count
    }

    /// Queue one pending slot and return whether the span needs one queue entry.
    pub(crate) fn queue_slot(&self, slot_index: usize) -> bool {
        let mut pending_slots = self.pending_slots.lock();
        pending_slots.set(slot_index);
        drop(pending_slots);

        self.is_queued
            .compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }

    /// Drain the currently pending slots.
    pub(crate) fn drain_slots(&self) -> Vec<usize> {
        let mut pending_slots = self.pending_slots.lock();
        let mut slots = Vec::with_capacity(pending_slots.count_ones());

        // current batch
        pending_slots.visit_set_ranges(|start, len| {
            for slot_index in start..start + len {
                slots.push(slot_index);
            }
        });
        pending_slots.clear_all();

        slots
    }

    /// Return whether more slots arrived while the caller was tracing this span.
    pub(crate) fn has_more_slots(&self) -> bool {
        let pending_slots = self.pending_slots.lock();

        if pending_slots.count_ones() == 0 {
            self.is_queued.store(0, Ordering::Release);

            return false;
        }

        true
    }
}

/// One table of per-span small-span work state.
pub(crate) type SharedSmallSpanWorkTable = Vec<Arc<SharedSmallSpanWork>>;
