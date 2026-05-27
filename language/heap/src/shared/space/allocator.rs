use std::sync::Arc;

use super::{SharedSmallSpan, SpanList};
use crate::allocator::{SizeClassTable, SpanSlot};
use crate::{AllocationPlan, SharedHeapReference, SmallAllocationPlan, SmallSpanClass};

/// One mutator-local shared allocation cache.
#[derive(Debug)]
pub struct SharedAllocationCache {
    /// The mutator-local dense allocation runs.
    pub(super) runs: Vec<SmallRun>,
    /// The mutator-local small allocation buckets.
    pub(super) small: Vec<SmallBucket>,
}

// SAFETY: runtime worker ownership keeps one shared cache on one worker at a time
unsafe impl Send for SharedAllocationCache {}

/// One mutator-local small allocation bucket.
#[derive(Debug)]
pub(super) struct SmallBucket {
    /// The homogeneous payload class allocated by this bucket.
    pub(super) class: SmallSpanClass,
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

/// One mutator-local dense shared small allocation run.
#[derive(Debug, Clone, Copy)]
pub(super) struct SmallRun {
    /// The next byte offset allocated from this run.
    pub(super) next_offset: usize,
    /// The byte offset after this run.
    pub(super) end_offset: usize,
}

/// One slot reserved from a shared small allocation bucket.
#[derive(Debug, Clone, Copy)]
pub(super) struct SmallSlot {
    /// The slot index inside the span.
    pub(super) slot_index: usize,
    /// Whether the slot belongs to the dense run.
    pub(super) is_dense: bool,
}

/// One shared small allocation from a mutator-local bucket.
#[derive(Debug, Clone, Copy)]
pub(super) struct SmallAllocation {
    /// The allocated small slot.
    pub(super) slot: SpanSlot,
    /// The base shared heap reference for the slot.
    pub(super) reference: SharedHeapReference,
    /// Whether the bucket still owns usable slots.
    pub(super) keep_bucket: bool,
}

impl SharedAllocationCache {
    /// Create one empty mutator-local shared allocation cache.
    pub(super) fn new(size_classes: SizeClassTable, span_bytes: usize, page_bytes: usize) -> Self {
        let bucket_count = SmallAllocationPlan::bucket_count(&size_classes);
        let mut small = Vec::with_capacity(bucket_count);
        let mut runs = Vec::with_capacity(bucket_count);

        // create scan and noscan buckets for every size class
        for class_index in 0..size_classes.classes.len() {
            let size_class = size_classes.classes[class_index];

            for is_noscan in [false, true] {
                let class = SmallSpanClass {
                    size_class: size_class.bytes,
                    span_bytes: size_class
                        .span_bytes(page_bytes, span_bytes)
                        .max(span_bytes),
                    trace_id: None,
                    is_noscan,
                };

                small.push(SmallBucket {
                    class,
                    span_index: 0,
                    span: None,
                    first_offset: 0,
                    slot_count: 0,
                    next_slot: 0,
                });
                runs.push(SmallRun {
                    next_offset: 0,
                    end_offset: 0,
                });
            }
        }

        Self { runs, small }
    }

    /// Return the byte charge for acquiring an allocation run.
    #[inline(always)]
    pub(crate) fn plan_run_charge_bytes(&self, layout: &AllocationPlan<'_>) -> usize {
        // small allocations acquire one span at a time
        if let Some(small) = layout.class.small() {
            return self.small[small.bucket_index].class.span_bytes;
        }

        layout.byte_len
    }

    /// Try to allocate one zeroed payload from the active run for one small class.
    #[inline(always)]
    pub(crate) fn try_allocate_zeroed(
        &mut self,
        small: SmallAllocationPlan,
    ) -> Option<SharedHeapReference> {
        let bucket = self.small.get(small.bucket_index())?;
        if bucket.class != small.class {
            return None;
        }

        // SAFETY: the bucket class was checked against this small allocation plan
        unsafe { self.reserve_zeroed_run_slot_unchecked(small.bucket_index(), small.slot_bytes()) }
    }

    /// Reserve one zeroed allocation from trusted instruction fields.
    ///
    /// The caller must pass a bucket index and slot byte width from the same resolved small allocation.
    ///
    /// # Safety
    ///
    /// `bucket_index` must identify the bucket that owns `slot_bytes`.
    #[inline(always)]
    pub(crate) unsafe fn reserve_zeroed_run_slot_unchecked(
        &mut self,
        bucket_index: usize,
        slot_bytes: usize,
    ) -> Option<SharedHeapReference> {
        // SAFETY: caller resolved both values from the same small allocation plan
        let run = unsafe { self.runs.get_unchecked_mut(bucket_index) };
        // SAFETY: caller resolved both values from the same small allocation plan
        let bucket = unsafe { self.small.get_unchecked_mut(bucket_index) };
        let reference = run.reserve_reference(slot_bytes)?;

        // publish the bump before returning to the mutator
        bucket.publish_run(run);
        if !bucket.has_available_slot(run) {
            bucket.finish(run);
            bucket.clear(run);
        }

        Some(reference)
    }
}

impl SmallRun {
    /// Reserve one reference from this dense run.
    #[inline(always)]
    pub(super) fn reserve_reference(&mut self, size_class: usize) -> Option<SharedHeapReference> {
        // run exhausted
        if size_class > self.end_offset - self.next_offset {
            return None;
        }

        // bump the dense run
        let reference = SharedHeapReference::new(self.next_offset);
        self.next_offset += size_class;

        Some(reference)
    }

    /// Reserve one slot index from this dense run.
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

    /// Return the current dense-run slot index.
    #[inline(always)]
    pub(super) fn next_slot(&self, first_offset: usize, size_class: usize) -> usize {
        // inactive run
        if self.end_offset == 0 {
            return 0;
        }

        (self.next_offset - first_offset) / size_class
    }

    /// Return whether this dense run still has one full slot.
    #[inline(always)]
    pub(super) fn has_available_slot(&self, size_class: usize) -> bool {
        size_class <= self.end_offset - self.next_offset
    }

    /// Install one dense run.
    pub(super) fn install(&mut self, next_offset: usize, end_offset: usize) {
        self.next_offset = next_offset;
        self.end_offset = end_offset;
    }

    /// Clear this dense run.
    pub(super) fn clear(&mut self) {
        self.next_offset = 0;
        self.end_offset = 0;
    }
}

impl SmallBucket {
    /// Reserve one slot from this allocator.
    #[inline(always)]
    pub(super) fn reserve_slot(&mut self, run: &mut SmallRun) -> Option<SmallSlot> {
        // dense run path
        if run.end_offset > 0 {
            return run.reserve_slot(self.first_offset, self.class.size_class);
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

        // final fallback to the span free bitmap
        self.span
            .as_ref()?
            .reserve_slot()
            .map(|slot_index| SmallSlot {
                slot_index,
                is_dense: false,
            })
    }

    /// Return whether this bucket can still allocate locally.
    #[inline(always)]
    pub(super) fn has_available_slot(&self, run: &SmallRun) -> bool {
        // dense run capacity
        if run.end_offset > 0 {
            return run.has_available_slot(self.class.size_class);
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

    /// Publish allocated run slots from this bucket.
    #[inline(always)]
    pub(super) fn publish_run(&self, run: &SmallRun) {
        // non-dense buckets have no run state to publish
        if run.end_offset == 0 {
            return;
        }

        // publish every dense slot reserved so far
        if let Some(span) = &self.span {
            span.publish_dense_len(run.next_slot(self.first_offset, self.class.size_class));
        }
    }

    /// Finish this bucket after its last usable slot.
    #[inline(always)]
    pub(super) fn finish(&self, run: &SmallRun) {
        // publish pending dense slots before list transition
        self.publish_run(run);

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

    /// Return whether this bucket currently owns a span.
    #[inline(always)]
    pub(super) fn is_active(&self) -> bool {
        self.span.is_some()
    }

    /// Install one shared small span into this bucket.
    pub(super) fn install(
        &mut self,
        run: &mut SmallRun,
        span_index: usize,
        span: Arc<SharedSmallSpan>,
        use_dense_run: bool,
    ) {
        // initialize bucket metadata from the span
        let next_slot = span.first_free_slot();

        self.span_index = span_index as u32;
        self.class = span.class;
        self.span = Some(span.clone());
        self.first_offset = span.first_offset;
        self.slot_count = span.slot_count;
        self.next_slot = next_slot;

        // dense runs cover newly mapped spans
        let end_offset = if use_dense_run {
            span.first_offset + span.class.size_class * span.slot_count
        } else {
            0
        };
        let next_offset = span.first_offset + span.class.size_class * next_slot;

        // publish run bounds to the allocator hot path
        run.install(next_offset, end_offset);
    }

    /// Clear the current shared small span from this bucket.
    pub(super) fn clear(&mut self, run: &mut SmallRun) {
        // clear bucket metadata
        self.span = None;
        self.span_index = 0;
        self.first_offset = 0;
        self.slot_count = 0;
        self.next_slot = 0;

        // clear paired dense run
        run.clear();
    }

    /// Return the span slot for one trusted slot index.
    #[inline(always)]
    pub(super) fn span_slot(&self, slot_index: usize) -> SpanSlot {
        SpanSlot::from_raw(self.span_index, slot_index as u32)
    }
}
