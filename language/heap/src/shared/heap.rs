use std::collections::BTreeSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use destack_mir::ReferenceMap;
use serde::{Deserialize, Serialize};

use super::{
    SharedAllocator, SharedGcPhase, SharedGcWorker, SharedHeapLimits, SharedHeapSpace,
    SharedHeapSpaceImage, SharedHeapUsage, SharedRawSpace, SharedRawSpaceImage,
};
use crate::{
    AllocationLayout, Allocator, AllocatorImage, GcPacer, GcProgress, GcState, GcStats, HeapError,
    HeapOptions, HeapResult, PageId, PageView, Payload, SharedHeapReference, SharedRawPointer,
    apply_byte_delta,
};

/// Shared collector pacing state.
#[derive(Debug, Default)]
struct SharedGcPacer {
    /// Pending shared collector assist debt in allocated bytes.
    assist_debt_bytes: AtomicUsize,
}

impl SharedGcPacer {
    /// Add pending collector assist debt without losing concurrent updates.
    fn add_assist_debt(&self, byte_len: usize) {
        loop {
            let pending = self.assist_debt_bytes.load(Ordering::Acquire);
            let next = pending.saturating_add(byte_len);

            if self
                .assist_debt_bytes
                .compare_exchange(pending, next, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                return;
            }
        }
    }

    /// Consume pending collector assist debt as collector steps.
    fn take_assist_work(&self, work_bytes: usize, bytes_per_step: usize) -> usize {
        loop {
            let pending = self.assist_debt_bytes.load(Ordering::Acquire);
            let consumed = pending.min(work_bytes);
            let remaining = pending - consumed;

            if self
                .assist_debt_bytes
                .compare_exchange(pending, remaining, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                return consumed.div_ceil(bytes_per_step.max(1));
            }
        }
    }

    /// Clear pending collector assist debt.
    fn clear_assist_debt(&self) {
        self.assist_debt_bytes.store(0, Ordering::Release);
    }

    /// Return pending collector assist debt in bytes.
    fn assist_debt_bytes(&self) -> usize {
        self.assist_debt_bytes.load(Ordering::Acquire)
    }
}

/// One live world-shared heap.
#[derive(Debug)]
pub struct SharedHeap {
    /// The shared allocator for both shared heap spaces.
    pub(crate) allocator: Arc<Allocator>,
    /// The configured shared heap options.
    pub(crate) options: HeapOptions,

    /// The traced shared heap space.
    pub(crate) heap: SharedHeapSpace,
    /// The explicit shared raw space.
    pub(crate) raw: SharedRawSpace,
    /// The exact hard limits for this shared heap.
    pub(crate) limits: SharedHeapLimits,

    /// Whether one shared collection has been requested by pressure or explicitly.
    collection_requested: AtomicBool,
    /// Shared collector pacing state.
    gc_pacer: SharedGcPacer,
}

/// One frozen shared heap.
#[derive(Debug, Clone)]
pub struct SharedHeapImage {
    /// The retained shared heap root.
    root: Arc<SharedHeapImageRoot>,
}

/// One retained shared heap image root.
#[derive(Debug)]
struct SharedHeapImageRoot {
    /// The allocator backing every captured page.
    allocator: Arc<Allocator>,
    /// The captured shared heap options.
    options: HeapOptions,
    /// The frozen shared heap space.
    heap: SharedHeapSpaceImage,
    /// The frozen shared raw space.
    raw: SharedRawSpaceImage,
    /// The retained shared page views owned by this image.
    retained_page_views: Box<[PageView]>,
}

/// One serialized shared heap snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SharedHeapSnapshot {
    /// The serialized allocator pages reachable from this shared root.
    allocator: AllocatorImage,
    /// The captured shared heap options.
    options: HeapOptions,
    /// The frozen shared heap space.
    heap: SharedHeapSpaceImage,
    /// The frozen shared raw space.
    raw: SharedRawSpaceImage,
}

impl Drop for SharedHeapImageRoot {
    fn drop(&mut self) {
        let _ = self.allocator.release_page_views(&self.retained_page_views);
    }
}

impl SharedHeapImage {
    /// Create one frozen shared heap root.
    pub(crate) fn new(
        allocator: Arc<Allocator>,
        options: HeapOptions,
        heap: SharedHeapSpaceImage,
        raw: SharedRawSpaceImage,
    ) -> HeapResult<Self> {
        let page_views = heap
            .page_views()
            .into_iter()
            .chain(raw.page_views())
            .collect::<Vec<_>>();
        let retained_page_views = allocator.retain_page_views(page_views)?.into_boxed_slice();
        let root = SharedHeapImageRoot {
            allocator,
            options,
            heap,
            raw,
            retained_page_views,
        };

        Ok(Self {
            root: Arc::new(root),
        })
    }

    /// Build one shared heap image from one serialized snapshot.
    pub fn from_snapshot(snapshot: &SharedHeapSnapshot) -> HeapResult<Self> {
        let allocator = Arc::new(Allocator::from_image(&snapshot.allocator)?);

        Self::new(
            allocator,
            snapshot.options.clone(),
            snapshot.heap.clone(),
            snapshot.raw.clone(),
        )
    }

    /// Flatten this image into one serialized snapshot.
    pub fn snapshot(&self) -> HeapResult<SharedHeapSnapshot> {
        let pages = self.page_ids();

        Ok(SharedHeapSnapshot {
            allocator: self.allocator().image_pages_from_ids(&pages)?,
            options: self.options().clone(),
            heap: self.heap().clone(),
            raw: self.raw().clone(),
        })
    }

    /// Return the allocator backing every captured page.
    pub fn allocator(&self) -> &Arc<Allocator> {
        &self.root.allocator
    }

    /// Return the captured shared heap options.
    pub fn options(&self) -> &HeapOptions {
        &self.root.options
    }

    /// Return the frozen shared heap space.
    pub fn heap(&self) -> &SharedHeapSpaceImage {
        &self.root.heap
    }

    /// Return the frozen shared raw space.
    pub fn raw(&self) -> &SharedRawSpaceImage {
        &self.root.raw
    }

    /// Return every allocator page reachable from this shared-heap image.
    pub fn page_ids(&self) -> Vec<PageId> {
        let mut pages = self.heap().page_ids();
        pages.extend(self.raw().page_ids());

        pages
            .into_iter()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }
}

impl SharedHeapSnapshot {
    /// Return every allocator page captured by this shared heap snapshot.
    pub fn page_ids(&self) -> Vec<PageId> {
        self.allocator.pages.iter().map(|page| page.id).collect()
    }
}

impl SharedHeap {
    /// Create one shared heap over one explicit allocator, limits, and options.
    pub fn with_allocator_limits_and_options(
        allocator: Arc<Allocator>,
        limits: SharedHeapLimits,
        options: HeapOptions,
    ) -> HeapResult<Self> {
        options.validate_shared()?;
        options.validate_allocator(&allocator)?;

        let shared = Self {
            heap: SharedHeapSpace::with_options(allocator.clone(), &options)?,
            raw: SharedRawSpace::with_allocator(allocator.clone()),
            allocator,
            options,
            collection_requested: AtomicBool::new(false),
            gc_pacer: SharedGcPacer::default(),
            limits,
        };

        shared.refresh_gc_request();

        Ok(shared)
    }

    /// Return the configured shared page size.
    pub fn page_bytes(&self) -> usize {
        self.allocator.page_bytes()
    }

    /// Return the configured shared heap options.
    pub fn options(&self) -> &HeapOptions {
        &self.options
    }

    /// Return the configured shared heap limits.
    pub fn limits(&self) -> SharedHeapLimits {
        self.limits
    }

    /// Return the exact live usage for this shared heap.
    pub fn usage(&self) -> SharedHeapUsage {
        SharedHeapUsage {
            heap: self.heap.usage(),
            raw: self.raw.usage(),
        }
    }

    /// Return the exact active shared heap bytes.
    pub fn active_bytes(&self) -> u64 {
        self.heap.active_bytes() + self.raw.active_bytes()
    }

    /// Return the current shared heap collector state.
    pub fn gc_state(&self) -> GcState {
        self.heap.gc_state()
    }

    /// Return whether the active shared mark phase is currently drained.
    pub fn mark_idle(&self) -> bool {
        self.heap.mark_idle()
    }

    /// Return the current derived collector pacing targets.
    pub fn gc_pacer(&self) -> GcPacer {
        let mut gc_pacer = GcPacer::default();
        gc_pacer.update(self.options.gc, self.heap_allocated_bytes());
        gc_pacer.assist_debt_bytes = self.gc_pacer.assist_debt_bytes() as u64;

        gc_pacer
    }

    /// Return the bounded shared collection budget for one world step.
    fn base_collection_budget(&self, worker_count: usize) -> usize {
        let worker_count = worker_count.max(1);
        let span_pages = self.options.small_span_pages();
        let heap_pages = self.heap_pages();
        let heap_spans = heap_pages.div_ceil(span_pages).max(1);
        let span_slots = self.options.minimum_small_span_slots();
        let worker_work = worker_count.saturating_mul(span_slots);

        // sweep can spend the whole span budget on reclamation
        if self.gc_phase() == SharedGcPhase::Sweep {
            return worker_work.saturating_add(heap_spans);
        }

        worker_work.saturating_add(heap_spans)
    }

    /// Return and consume one bounded shared collection budget for the current world step.
    pub fn take_collection_budget(&self, worker_count: usize) -> usize {
        // world budget
        let base_budget = self.base_collection_budget(worker_count);

        // pending assist debt
        let page_bytes = self.page_bytes().max(1);
        let assist_steps = self
            .gc_pacer
            .take_assist_work(base_budget.saturating_mul(page_bytes), page_bytes);

        base_budget.saturating_add(assist_steps)
    }

    /// Return and consume one bounded shared assist budget for one allocator step.
    pub fn take_assist_budget(&self) -> usize {
        if self.gc_phase() == SharedGcPhase::Idle {
            return 0;
        }

        let assist_budget = self.options.minimum_small_span_slots();

        // bounded assist slice
        let page_bytes = self.page_bytes().max(1);

        self.gc_pacer
            .take_assist_work(assist_budget.saturating_mul(page_bytes), page_bytes)
    }

    /// Return the bounded local-to-shared edge scan budget for one worker step.
    pub fn edge_scan_budget(&self, worker_count: usize) -> usize {
        let worker_count = worker_count.max(1);

        worker_count.saturating_mul(self.options.minimum_small_span_slots())
    }

    /// Return the current shared heap size in allocator pages.
    fn heap_pages(&self) -> usize {
        let page_bytes = self.page_bytes().max(1);

        (self.heap_allocated_bytes() as usize).div_ceil(page_bytes)
    }

    /// Return the current shared heap collector phase.
    pub fn gc_phase(&self) -> SharedGcPhase {
        self.heap.gc_phase()
    }

    /// Return the exact mapped shared heap bytes.
    pub fn mapped_bytes(&self) -> u64 {
        self.heap.mapped_bytes() + self.raw.mapped_bytes()
    }

    /// Return the exact borrowed shared heap bytes.
    pub fn borrowed_bytes(&self) -> u64 {
        self.heap.borrowed_bytes() + self.raw.borrowed_bytes()
    }

    /// Return the exact active shared raw-space bytes.
    pub fn raw_active_bytes(&self) -> u64 {
        self.raw.active_bytes()
    }

    /// Return the projected mapped-byte delta for one shared raw allocation.
    pub fn raw_alloc_mapped_byte_delta(&self, byte_len: usize) -> i64 {
        self.raw.alloc_mapped_byte_delta(byte_len)
    }

    /// Return the projected mapped-byte delta for one shared raw replacement.
    pub fn raw_replace_mapped_byte_delta(
        &self,
        pointer: SharedRawPointer,
        next_byte_len: usize,
    ) -> HeapResult<i64> {
        self.raw.replace_mapped_byte_delta(pointer, next_byte_len)
    }

    /// Allocate one shared raw allocation.
    pub fn allocate_raw(
        &self,
        byte_len: usize,
        allocation: Payload<'_>,
    ) -> HeapResult<SharedRawPointer> {
        self.check_raw_mapped_byte_delta(self.raw.alloc_mapped_byte_delta(byte_len))?;

        self.raw.allocate(byte_len, allocation)
    }

    /// Replace one shared raw allocation payload.
    pub fn replace_raw_bytes(
        &self,
        pointer: SharedRawPointer,
        bytes: &[u8],
    ) -> HeapResult<SharedRawPointer> {
        self.check_raw_mapped_byte_delta(
            self.raw.replace_mapped_byte_delta(pointer, bytes.len())?,
        )?;

        self.raw.replace_bytes(pointer, bytes)
    }

    /// Return the bytes for one shared raw allocation.
    pub fn read_raw_bytes(&self, pointer: SharedRawPointer) -> HeapResult<Vec<u8>> {
        self.raw.read_bytes(pointer)
    }

    /// Fill one caller-provided buffer from one shared raw allocation at one offset.
    pub fn read_raw_bytes_into(
        &self,
        pointer: SharedRawPointer,
        start: usize,
        target: &mut [u8],
    ) -> HeapResult<()> {
        self.raw.read_bytes_into(pointer, start, target)
    }

    /// Return the remaining byte length for one shared raw allocation.
    pub fn raw_byte_len(&self, pointer: SharedRawPointer) -> HeapResult<usize> {
        self.raw.byte_len(pointer)
    }

    /// Overwrite one shared raw byte range.
    pub fn write_raw_bytes(
        &self,
        pointer: SharedRawPointer,
        start: usize,
        bytes: &[u8],
    ) -> HeapResult<()> {
        self.raw.write_bytes(pointer, start, bytes)
    }

    /// Create one shared heap allocator front end.
    pub fn allocator(&self) -> SharedAllocator {
        self.heap.allocator()
    }

    /// Allocate one shared managed heap allocation.
    pub fn allocate(
        &self,
        allocator: &mut SharedAllocator,
        layout: AllocationLayout<'_>,
        allocation: Payload<'_>,
    ) -> HeapResult<SharedHeapReference> {
        let mapped_byte_delta = self.heap.mapped_byte_delta(allocator, layout)?;
        self.check_heap_mapped_byte_delta(mapped_byte_delta)?;

        let reference = self.heap.allocate(allocator, layout, allocation)?;
        self.accrue_assist_debt(layout.byte_len);
        self.refresh_gc_request();

        Ok(reference)
    }

    /// Return whether one shared heap reference currently refers to one live allocation.
    pub fn is_heap_live(&self, reference: SharedHeapReference) -> bool {
        self.heap.is_live(reference)
    }

    /// Return the remaining byte length for one shared heap reference.
    pub fn heap_byte_len(&self, reference: SharedHeapReference) -> HeapResult<usize> {
        self.heap.byte_len(reference)
    }

    /// Return the bytes for one shared heap reference.
    pub fn read_heap_bytes(&self, reference: SharedHeapReference) -> HeapResult<Vec<u8>> {
        self.heap.read_bytes(reference)
    }

    /// Fill one caller-provided buffer from one shared heap allocation at one offset.
    pub fn read_heap_bytes_into(
        &self,
        reference: SharedHeapReference,
        start: usize,
        target: &mut [u8],
    ) -> HeapResult<()> {
        self.heap.read_bytes_into(reference, start, target)
    }

    /// Return the scan metadata for one shared heap reference.
    pub fn scan(&self, reference: SharedHeapReference) -> HeapResult<ReferenceMap> {
        self.heap.scan(reference)
    }

    /// Overwrite one shared heap byte range.
    pub fn write_heap_bytes(
        &self,
        reference: SharedHeapReference,
        start: usize,
        bytes: &[u8],
    ) -> HeapResult<()> {
        self.heap.write_bytes(reference, start, bytes)
    }

    /// Record one shared heap write barrier over one byte range.
    pub fn write_barrier(
        &self,
        reference: SharedHeapReference,
        start: usize,
        byte_len: usize,
    ) -> HeapResult<()> {
        self.heap.write_barrier(reference, start, byte_len)
    }

    /// Record one shared heap write barrier from one caller-provided byte slice.
    pub fn write_barrier_bytes(
        &self,
        reference: SharedHeapReference,
        start: usize,
        bytes: &[u8],
    ) -> HeapResult<()> {
        self.heap
            .write_shared_barrier_bytes(reference, start, bytes)
    }

    /// Publish one exact shared heap reference after one completed store.
    pub fn publish_edge(&self, reference: SharedHeapReference) -> HeapResult<()> {
        self.heap.publish_edge(reference)
    }

    /// Request one shared collection cycle at the next world step.
    pub fn request_gc(&self) {
        self.collection_requested.store(true, Ordering::Release);
    }

    /// Start one requested or pressure-driven shared collection cycle.
    pub fn start_gc(&self) -> HeapResult<bool> {
        // active cycle
        if self.gc_phase() != SharedGcPhase::Idle {
            return Ok(false);
        }

        // no pending request
        if !self.collection_requested.load(Ordering::Acquire) {
            return Ok(false);
        }

        // cycle start
        self.heap.start_mark([])?;
        self.collection_requested.store(false, Ordering::Release);

        Ok(true)
    }

    /// Perform one full shared heap collection over explicit roots.
    pub fn collect_full(
        &self,
        roots: impl IntoIterator<Item = SharedHeapReference>,
    ) -> HeapResult<GcStats> {
        let stats = self.heap.collect_full(roots)?;
        self.record_gc_cycle(stats);

        Ok(stats)
    }

    /// Run one shared collection step with one explicit work budget.
    pub fn gc_step(
        &self,
        roots: &[SharedHeapReference],
        roots_complete: bool,
        step_budget: usize,
    ) -> HeapResult<GcProgress> {
        self.gc_step_for_worker(None, roots, roots_complete, step_budget)
    }

    /// Return the shared GC worker handle for one runtime worker.
    pub fn gc_worker(&self, worker_index: usize) -> SharedGcWorker {
        self.heap.gc.trace_queue.worker(worker_index)
    }

    /// Run one shared collection step for one worker with one explicit work budget.
    pub fn gc_step_for_worker(
        &self,
        worker: Option<&SharedGcWorker>,
        roots: &[SharedHeapReference],
        roots_complete: bool,
        step_budget: usize,
    ) -> HeapResult<GcProgress> {
        // empty budget
        if step_budget == 0 {
            return Ok(GcProgress::Idle);
        }

        // idle
        if self.gc_phase() == SharedGcPhase::Idle {
            return Ok(GcProgress::Idle);
        }

        // concurrent mark
        if self.gc_phase() == SharedGcPhase::Mark {
            self.heap
                .mark_step(worker, roots.iter().copied(), step_budget)?;

            // termination check
            if roots_complete {
                self.heap.try_start_sweep()?;
            }

            return Ok(GcProgress::Active);
        }

        // incremental sweep
        let progress = self.heap.sweep_step(step_budget)?;
        if let Some(stats) = progress.completed_stats() {
            self.record_gc_cycle(stats);
        }

        Ok(progress)
    }

    /// Fork this shared heap over the same shared allocator.
    pub fn fork(&self) -> HeapResult<Self> {
        Ok(Self {
            allocator: self.allocator.clone(),
            options: self.options.clone(),
            collection_requested: AtomicBool::new(
                self.collection_requested.load(Ordering::Acquire),
            ),
            gc_pacer: SharedGcPacer::default(),
            heap: self.heap.fork()?,
            raw: self.raw.fork()?,
            limits: self.limits,
        })
    }

    /// Create one shared heap from one frozen shared heap image and explicit hard limits.
    pub fn from_image_with_limits(
        image: &SharedHeapImage,
        limits: SharedHeapLimits,
    ) -> HeapResult<Self> {
        let allocator = image.allocator().clone();
        let options = image.options().clone();

        options.validate_shared()?;
        options.validate_allocator(&allocator)?;

        let shared = Self {
            heap: SharedHeapSpace::from_image_with_allocator(allocator.clone(), image.heap())?,
            raw: SharedRawSpace::from_image_with_allocator(allocator.clone(), image.raw())?,
            allocator,
            options,
            collection_requested: AtomicBool::new(false),
            gc_pacer: SharedGcPacer::default(),
            limits,
        };

        shared.refresh_gc_request();
        shared.check_limits()?;

        Ok(shared)
    }

    /// Create one shared heap from one frozen shared heap image.
    pub fn from_image(image: &SharedHeapImage) -> HeapResult<Self> {
        Self::from_image_with_limits(image, SharedHeapLimits::default())
    }

    /// Create one shared heap from one serialized shared heap snapshot.
    pub fn from_snapshot_with_limits(
        snapshot: &SharedHeapSnapshot,
        limits: SharedHeapLimits,
    ) -> HeapResult<Self> {
        let image = SharedHeapImage::from_snapshot(snapshot)?;

        Self::from_image_with_limits(&image, limits)
    }

    /// Check the current shared heap usage against the configured limits.
    fn check_limits(&self) -> HeapResult<()> {
        // total limit
        if let Some(max_bytes) = self.limits.max_bytes {
            let active_bytes = self.active_bytes();
            if active_bytes > max_bytes {
                return Err(HeapError::TotalLimitExceeded {
                    used_bytes: active_bytes,
                    max_bytes,
                });
            }
        }

        // per-space limits
        self.limits.heap.check(self.heap.active_bytes())?;
        self.limits.raw.check(self.raw.active_bytes())?;

        Ok(())
    }

    /// Check one projected mapped-byte delta against shared heap limits.
    fn check_heap_mapped_byte_delta(&self, mapped_byte_delta: i64) -> HeapResult<()> {
        // total limit
        if let Some(max_bytes) = self.limits.max_bytes {
            let active_bytes = apply_byte_delta(self.active_bytes(), mapped_byte_delta)?;
            if active_bytes > max_bytes {
                return Err(HeapError::TotalLimitExceeded {
                    used_bytes: active_bytes,
                    max_bytes,
                });
            }
        }

        // heap limit
        self.limits
            .heap
            .check_mapped_byte_delta(self.heap.active_bytes(), mapped_byte_delta)
    }

    /// Check one projected mapped-byte delta against shared raw limits.
    fn check_raw_mapped_byte_delta(&self, mapped_byte_delta: i64) -> HeapResult<()> {
        // total limit
        if let Some(max_bytes) = self.limits.max_bytes {
            let active_bytes = apply_byte_delta(self.active_bytes(), mapped_byte_delta)?;
            if active_bytes > max_bytes {
                return Err(HeapError::TotalLimitExceeded {
                    used_bytes: active_bytes,
                    max_bytes,
                });
            }
        }

        // raw limit
        self.limits
            .raw
            .check_mapped_byte_delta(self.raw.active_bytes(), mapped_byte_delta)
    }

    /// Return one frozen shared heap image.
    pub fn image(&self) -> HeapResult<SharedHeapImage> {
        let heap = self.heap.image();
        let raw = self.raw.image();

        SharedHeapImage::new(self.allocator.clone(), self.options.clone(), heap, raw)
    }

    /// Return the number of live shared heap allocations.
    pub fn heap_allocation_count(&self) -> usize {
        self.heap.allocation_count()
    }

    /// Return the number of allocated shared heap bytes.
    pub fn heap_allocated_bytes(&self) -> u64 {
        self.heap.allocated_bytes()
    }

    /// Refresh the pending shared cycle request from current heap pressure.
    fn refresh_gc_request(&self) {
        // trigger crossing
        if self.gc_pacer().should_start(self.heap_allocated_bytes()) {
            self.collection_requested.store(true, Ordering::Release);
        }
    }

    /// Record one completed shared collection cycle in the pacer.
    fn record_gc_cycle(&self, _stats: GcStats) {
        // clear the completed cycle state
        self.collection_requested.store(false, Ordering::Release);
        self.gc_pacer.clear_assist_debt();

        // re-evaluate current pressure
        self.refresh_gc_request();
    }

    /// Accrue shared assist debt from one heap allocation.
    fn accrue_assist_debt(&self, allocated_bytes: usize) {
        // empty allocation
        if allocated_bytes == 0 {
            return;
        }

        let gc_pacer = self.gc_pacer();
        let heap_bytes = self.heap_allocated_bytes();
        if heap_bytes < gc_pacer.trigger_bytes && self.gc_phase() == SharedGcPhase::Idle {
            return;
        }

        self.gc_pacer.add_assist_debt(allocated_bytes);
    }
}
