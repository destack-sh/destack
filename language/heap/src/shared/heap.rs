use std::collections::BTreeSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use destack_mir::ReferenceMap;
use serde::{Deserialize, Serialize};

use super::{
    SharedAllocator, SharedGcPacer, SharedGcPhase, SharedGcWorker, SharedHeapLimits,
    SharedHeapSpace, SharedHeapSpaceImage, SharedHeapUsage, SharedRawSpace, SharedRawSpaceImage,
};
use crate::{
    AllocationLayout, AllocationShape, Allocator, AllocatorImage, GcPacer, GcPressure, GcProgress,
    GcState, GcStats, HeapError, HeapOptions, HeapResult, PageId, PageRun, Payload,
    RawAllocationShape, SharedHeapReference, SharedRawPointer, apply_byte_delta,
};

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
    /// The retained shared heap image state.
    state: Arc<SharedHeapImageState>,
}

/// One retained shared heap image state.
#[derive(Debug)]
struct SharedHeapImageState {
    /// The allocator backing every captured page.
    allocator: Arc<Allocator>,
    /// The captured shared heap options.
    options: HeapOptions,
    /// The frozen shared heap space.
    heap: SharedHeapSpaceImage,
    /// The frozen shared raw space.
    raw: SharedRawSpaceImage,
    /// The shared page runs owned by this image.
    page_runs: Box<[PageRun]>,
}

/// One serialized shared heap snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SharedHeapSnapshot {
    /// The serialized allocator pages reachable from this shared heap image.
    allocator: AllocatorImage,
    /// The captured shared heap options.
    options: HeapOptions,
    /// The frozen shared heap space.
    heap: SharedHeapSpaceImage,
    /// The frozen shared raw space.
    raw: SharedRawSpaceImage,
}

impl Drop for SharedHeapImageState {
    fn drop(&mut self) {
        let _ = self.allocator.release_page_runs(&self.page_runs);
    }
}

impl SharedHeapImage {
    /// Create one frozen shared heap image.
    pub(crate) fn new(
        allocator: Arc<Allocator>,
        options: HeapOptions,
        heap: SharedHeapSpaceImage,
        raw: SharedRawSpaceImage,
    ) -> HeapResult<Self> {
        let mut page_runs = heap.page_runs();
        page_runs.extend(raw.page_runs());

        let state = SharedHeapImageState {
            allocator,
            options,
            heap,
            raw,
            page_runs: page_runs.into_boxed_slice(),
        };

        Ok(Self {
            state: Arc::new(state),
        })
    }

    /// Build one shared heap image from one serialized snapshot.
    pub fn from_snapshot(snapshot: &SharedHeapSnapshot) -> HeapResult<Self> {
        let allocator = Arc::new(Allocator::try_new(
            snapshot.allocator.page_bytes as usize,
            snapshot.allocator.chunk_bytes as usize,
        )?);

        Self::from_snapshot_with_allocator(snapshot, allocator)
    }

    /// Build one shared heap image from one serialized snapshot and allocator.
    pub fn from_snapshot_with_allocator(
        snapshot: &SharedHeapSnapshot,
        allocator: Arc<Allocator>,
    ) -> HeapResult<Self> {
        allocator.restore_image_pages(&snapshot.allocator)?;
        let mut page_runs = snapshot.heap.page_runs();
        page_runs.extend(snapshot.raw.page_runs());
        allocator.restore_page_run_refs(&page_runs)?;

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
        &self.state.allocator
    }

    /// Return the captured shared heap options.
    pub fn options(&self) -> &HeapOptions {
        &self.state.options
    }

    /// Return the frozen shared heap space.
    pub fn heap(&self) -> &SharedHeapSpaceImage {
        &self.state.heap
    }

    /// Return the frozen shared raw space.
    pub fn raw(&self) -> &SharedRawSpaceImage {
        &self.state.raw
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
            raw: SharedRawSpace::with_options(allocator.clone(), &options)?,
            allocator,
            options,
            collection_requested: AtomicBool::new(false),
            gc_pacer: SharedGcPacer::default(),
            limits,
        };

        shared
            .gc_pacer
            .set_live_bytes(&shared.options, shared.heap_allocated_bytes());
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

    /// Return the exact retained shared allocator-page bytes.
    pub fn retained_bytes(&self) -> u64 {
        self.heap.retained_bytes() + self.raw.retained_bytes()
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
        self.gc_pacer
            .snapshot(&self.options, self.heap_allocated_bytes())
    }

    /// Return and consume the base shared collection budget for one world step.
    fn take_base_collection_budget_bytes(&self, worker_count: usize) -> usize {
        self.gc_pacer
            .base_budget_bytes(&self.options, self.heap_allocated_bytes(), worker_count)
    }

    /// Return and consume one bounded shared collection budget in bytes for the current world step.
    pub fn take_collection_budget_bytes(&self, worker_count: usize) -> usize {
        self.gc_pacer.take_collection_budget_bytes(
            &self.options,
            self.heap_allocated_bytes(),
            worker_count,
        )
    }

    /// Return and consume one bounded shared assist budget in bytes for one allocator step.
    pub fn take_assist_budget_bytes(&self) -> usize {
        if self.gc_phase() == SharedGcPhase::Idle {
            return 0;
        }

        let gc_pacer = self.gc_pacer();
        let budget_bytes = gc_pacer.base_budget_bytes(self.options.gc, 1);

        self.gc_pacer.take_assist_budget_bytes(budget_bytes)
    }

    /// Return and consume the local-to-shared edge scan budget for one worker.
    pub fn take_edge_scan_work_bytes(&self) -> usize {
        self.take_base_collection_budget_bytes(1)
    }

    /// Return the current shared heap collector phase.
    #[inline(always)]
    pub fn gc_phase(&self) -> SharedGcPhase {
        self.heap.gc_phase()
    }

    /// Return the exact retained shared raw allocator-page bytes.
    pub fn raw_retained_bytes(&self) -> u64 {
        self.raw.retained_bytes()
    }

    /// Return the projected retained-byte delta for one shared raw allocation.
    pub fn raw_alloc_retained_byte_delta(&self, shape: RawAllocationShape) -> i64 {
        self.raw.alloc_retained_byte_delta(shape)
    }

    /// Return the projected retained-byte delta for one shared raw replacement.
    pub fn raw_replace_retained_byte_delta(
        &self,
        pointer: SharedRawPointer,
        next_byte_len: usize,
    ) -> HeapResult<i64> {
        self.raw.replace_retained_byte_delta(pointer, next_byte_len)
    }

    /// Allocate one shared raw allocation.
    pub fn allocate_raw(
        &self,
        shape: RawAllocationShape,
        allocation: Payload<'_>,
    ) -> HeapResult<SharedRawPointer> {
        self.check_raw_retained_byte_delta(self.raw.alloc_retained_byte_delta(shape))?;

        self.raw.allocate(shape, allocation)
    }

    /// Replace one shared raw allocation payload.
    pub fn replace_raw_bytes(
        &self,
        pointer: SharedRawPointer,
        bytes: &[u8],
    ) -> HeapResult<SharedRawPointer> {
        self.check_raw_retained_byte_delta(
            self.raw.replace_retained_byte_delta(pointer, bytes.len())?,
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

    /// Return one checked address for a shared raw byte range.
    pub fn raw_address(
        &self,
        pointer: SharedRawPointer,
        start: usize,
        byte_len: usize,
    ) -> HeapResult<*mut u8> {
        self.raw.address(pointer, start, byte_len)
    }

    /// Return one checked mutable address for a shared raw byte range.
    pub fn raw_address_mut(
        &self,
        pointer: SharedRawPointer,
        start: usize,
        byte_len: usize,
    ) -> HeapResult<*mut u8> {
        self.raw.address_mut(pointer, start, byte_len)
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

    /// Free one shared raw allocation.
    pub fn free_raw(&self, pointer: SharedRawPointer) -> HeapResult<()> {
        self.raw.free(pointer)
    }

    /// Create one worker-local shared heap allocator.
    pub fn allocator(&self) -> SharedAllocator {
        self.heap.allocator()
    }

    /// Publish and retire every worker-local shared heap run.
    pub fn flush_allocator(&self, allocator: &mut SharedAllocator) {
        self.heap.flush_allocator(allocator);
    }

    /// Allocate one shared managed heap allocation.
    #[inline(always)]
    pub fn allocate(
        &self,
        worker: &SharedGcWorker,
        allocator: &mut SharedAllocator,
        layout: &AllocationLayout<'_>,
        allocation: Payload<'_>,
    ) -> HeapResult<SharedHeapReference> {
        self.allocate_payload(worker, allocator, layout, allocation)
    }

    /// Allocate one byte-initialized shared managed heap allocation.
    #[inline(always)]
    pub fn allocate_bytes(
        &self,
        worker: &SharedGcWorker,
        allocator: &mut SharedAllocator,
        layout: &AllocationLayout<'_>,
        bytes: &[u8],
    ) -> HeapResult<SharedHeapReference> {
        self.allocate_payload(worker, allocator, layout, Payload::Bytes(bytes))
    }

    /// Allocate one zeroed shared managed heap allocation.
    #[inline(always)]
    pub fn allocate_zeroed(
        &self,
        worker: &SharedGcWorker,
        allocator: &mut SharedAllocator,
        layout: &AllocationLayout<'_>,
    ) -> HeapResult<SharedHeapReference> {
        if layout.is_empty() {
            return Err(HeapError::ZeroSizeAllocation);
        }

        let is_active_collection = self.gc_phase() != SharedGcPhase::Idle;

        // worker-local runs are the ordinary allocation path
        if !is_active_collection
            && layout.is_noscan
            && let Some(small) = layout.class.small()
            && let Some(reference) = allocator.reserve_small_zeroed(small)
        {
            return Ok(reference);
        }

        self.allocate_zeroed_refill(worker, allocator, layout)
    }

    /// Refill zeroed allocation state or allocate from published space.
    #[cold]
    #[inline(never)]
    fn allocate_zeroed_refill(
        &self,
        worker: &SharedGcWorker,
        allocator: &mut SharedAllocator,
        layout: &AllocationLayout<'_>,
    ) -> HeapResult<SharedHeapReference> {
        self.allocate_payload(worker, allocator, layout, Payload::Zeroed)
    }

    /// Allocate one shared managed payload.
    #[inline(always)]
    fn allocate_payload(
        &self,
        worker: &SharedGcWorker,
        allocator: &mut SharedAllocator,
        layout: &AllocationLayout<'_>,
        payload: Payload<'_>,
    ) -> HeapResult<SharedHeapReference> {
        if layout.is_empty() {
            return Err(HeapError::ZeroSizeAllocation);
        }

        if let Some(actual) = payload.byte_len()
            && actual != layout.byte_len
        {
            return Err(HeapError::InvalidAllocationBytes {
                expected: layout.byte_len,
                actual,
            });
        }

        let is_active_collection = self.gc_phase() != SharedGcPhase::Idle;

        // worker-local runs are the ordinary allocation path
        if !is_active_collection
            && let Some(reference) = self
                .heap
                .try_allocate_worker_small(allocator, layout, payload)?
        {
            return Ok(reference);
        }

        // active marking needs immediately published allocations
        if is_active_collection {
            self.flush_allocator(allocator);
        }

        let retained_byte_delta = self.heap.retained_byte_delta(allocator, layout)?;
        self.check_heap_retained_byte_delta(retained_byte_delta)?;

        let pressure_bytes = allocator.layout_run_charge_bytes(layout);

        // mark assist before acquiring another shared allocation run
        self.assist_allocation(worker, pressure_bytes)?;

        let reference = self
            .heap
            .allocate(allocator, layout, payload, !is_active_collection)?;
        if !is_active_collection {
            self.accrue_assist_debt(pressure_bytes);
        }
        self.refresh_gc_request();

        Ok(reference)
    }

    /// Resolve one allocation shape against this shared heap.
    #[inline(always)]
    pub fn allocation_layout<'a>(&self, shape: AllocationShape<'a>) -> AllocationLayout<'a> {
        self.heap.allocation_layout(shape)
    }

    /// Return whether one shared heap reference currently refers to one live allocation.
    pub fn is_heap_live(&self, reference: SharedHeapReference) -> bool {
        self.heap.is_live(reference)
    }

    /// Free one shared heap allocation immediately.
    pub fn free_heap(&self, reference: SharedHeapReference) -> HeapResult<()> {
        self.heap.free(reference).map(|_| ())
    }

    /// Return the base native address for direct shared heap access.
    #[inline(always)]
    pub fn heap_base_address(&self) -> usize {
        self.heap.base_address()
    }

    /// Return the base native address for direct shared raw heap access.
    #[inline(always)]
    pub fn raw_base_address(&self) -> usize {
        self.raw.base_address()
    }

    /// Return the scan metadata for one shared heap reference.
    pub fn scan(&self, reference: SharedHeapReference) -> HeapResult<ReferenceMap> {
        self.heap.scan(reference)
    }

    /// Record one shared heap write barrier before one byte store.
    pub fn write_barrier_bytes(
        &self,
        reference: SharedHeapReference,
        start: usize,
        bytes: &[u8],
    ) -> HeapResult<()> {
        self.heap.write_barrier_bytes(reference, start, bytes)
    }

    /// Record one shared heap write barrier after one completed byte store.
    pub fn write_barrier(
        &self,
        reference: SharedHeapReference,
        start: usize,
        byte_len: usize,
    ) -> HeapResult<()> {
        self.heap.write_barrier(reference, start, byte_len)
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
        self.gc_pacer
            .begin_cycle(&self.options, self.heap_allocated_bytes());
        self.heap.start_mark(&[])?;
        self.collection_requested.store(false, Ordering::Release);

        Ok(true)
    }

    /// Perform one full shared heap collection over explicit roots.
    pub fn collect_full(&self, roots: &[SharedHeapReference]) -> HeapResult<GcStats> {
        self.gc_pacer
            .begin_cycle(&self.options, self.heap_allocated_bytes());

        let stats = self.heap.collect_full(roots)?;
        self.record_gc_cycle(stats);

        Ok(stats)
    }

    /// Run one shared collection step with one explicit byte budget.
    pub fn collect_step(
        &self,
        roots: &[SharedHeapReference],
        roots_complete: bool,
        budget_bytes: usize,
    ) -> HeapResult<GcProgress> {
        self.collect_step_for_worker(None, roots, roots_complete, budget_bytes)
    }

    /// Register one shared GC worker.
    pub fn register_collector_worker(&self) -> SharedGcWorker {
        self.heap.gc.trace_queue.register_worker()
    }

    /// Run one shared collection step for one worker with one explicit byte budget.
    pub fn collect_step_for_worker(
        &self,
        worker: Option<&SharedGcWorker>,
        roots: &[SharedHeapReference],
        roots_complete: bool,
        budget_bytes: usize,
    ) -> HeapResult<GcProgress> {
        // empty budget
        if budget_bytes == 0 {
            return Ok(GcProgress::Idle);
        }

        // idle
        if self.gc_phase() == SharedGcPhase::Idle {
            return Ok(GcProgress::Idle);
        }

        // concurrent mark
        if self.gc_phase() == SharedGcPhase::Mark {
            self.heap.mark_step(worker, roots, budget_bytes)?;

            // termination check
            if roots_complete {
                self.heap.try_start_sweep()?;
            }

            return Ok(GcProgress::Active);
        }

        // incremental sweep
        let progress = self.heap.sweep_step(budget_bytes)?;
        if let Some(stats) = progress.completed_stats() {
            self.record_gc_cycle(stats);
        }

        Ok(progress)
    }

    /// Fork this shared heap over the same shared allocator.
    ///
    /// Call this only from a safepoint where shared heap mutators are stopped.
    pub fn fork(&self) -> HeapResult<Self> {
        Ok(Self {
            allocator: self.allocator.clone(),
            options: self.options.clone(),
            collection_requested: AtomicBool::new(
                self.collection_requested.load(Ordering::Acquire),
            ),
            gc_pacer: self.gc_pacer.fork(),
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

        shared
            .gc_pacer
            .set_live_bytes(&shared.options, shared.heap_allocated_bytes());
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

    /// Create one shared heap from one serialized snapshot, allocator, and explicit hard limits.
    pub fn from_snapshot_with_allocator(
        snapshot: &SharedHeapSnapshot,
        limits: SharedHeapLimits,
        allocator: Arc<Allocator>,
    ) -> HeapResult<Self> {
        let image = SharedHeapImage::from_snapshot_with_allocator(snapshot, allocator)?;

        Self::from_image_with_limits(&image, limits)
    }

    /// Check the current shared heap usage against the configured limits.
    fn check_limits(&self) -> HeapResult<()> {
        // total limit
        if let Some(max_bytes) = self.limits.max_bytes {
            let retained_bytes = self.retained_bytes();
            if retained_bytes > max_bytes {
                return Err(HeapError::TotalLimitExceeded {
                    used_bytes: retained_bytes,
                    max_bytes,
                });
            }
        }

        // per-space limits
        self.limits.heap.check(self.heap.retained_bytes())?;
        self.limits.raw.check(self.raw.retained_bytes())?;

        Ok(())
    }

    /// Check one projected retained-byte delta against shared heap limits.
    fn check_heap_retained_byte_delta(&self, retained_byte_delta: i64) -> HeapResult<()> {
        // total limit
        if let Some(max_bytes) = self.limits.max_bytes {
            let retained_bytes = apply_byte_delta(self.retained_bytes(), retained_byte_delta);
            if retained_bytes > max_bytes {
                return Err(HeapError::TotalLimitExceeded {
                    used_bytes: retained_bytes,
                    max_bytes,
                });
            }
        }

        // heap limit
        self.limits
            .heap
            .check_retained_byte_delta(self.heap.retained_bytes(), retained_byte_delta)
    }

    /// Check one projected retained-byte delta against shared raw limits.
    fn check_raw_retained_byte_delta(&self, retained_byte_delta: i64) -> HeapResult<()> {
        // total limit
        if let Some(max_bytes) = self.limits.max_bytes {
            let retained_bytes = apply_byte_delta(self.retained_bytes(), retained_byte_delta);
            if retained_bytes > max_bytes {
                return Err(HeapError::TotalLimitExceeded {
                    used_bytes: retained_bytes,
                    max_bytes,
                });
            }
        }

        // raw limit
        self.limits
            .raw
            .check_retained_byte_delta(self.raw.retained_bytes(), retained_byte_delta)
    }

    /// Return one frozen shared heap image.
    pub fn image(&self) -> HeapResult<SharedHeapImage> {
        let heap = self.heap.image()?;
        let raw = self.raw.image()?;

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
        // shared cycles are full-heap cycles
        match self.gc_pacer().pressure(self.heap_allocated_bytes()) {
            GcPressure::Idle => {}
            GcPressure::Cycle | GcPressure::Full => {
                self.collection_requested.store(true, Ordering::Release);
            }
        }
    }

    /// Record one completed shared collection cycle in the pacer.
    fn record_gc_cycle(&self, stats: GcStats) {
        // clear the completed cycle state
        self.collection_requested.store(false, Ordering::Release);
        self.gc_pacer.record_cycle(&self.options, stats);

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

        self.gc_pacer
            .charge_allocation(&self.options, heap_bytes, allocated_bytes);
    }

    /// Run shared collector work proportional to one allocation.
    fn assist_allocation(&self, worker: &SharedGcWorker, allocated_bytes: usize) -> HeapResult<()> {
        if allocated_bytes == 0 || self.gc_phase() == SharedGcPhase::Idle {
            return Ok(());
        }

        // charge this allocation into the active mark-assist debt
        self.gc_pacer.charge_allocation(
            &self.options,
            self.heap_allocated_bytes(),
            allocated_bytes,
        );
        let budget_bytes = self.take_assist_budget_bytes();
        if budget_bytes == 0 {
            return Ok(());
        }

        self.collect_step_for_worker(Some(worker), &[], false, budget_bytes)?;

        Ok(())
    }
}
