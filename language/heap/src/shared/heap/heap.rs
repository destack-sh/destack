use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use serde::{Deserialize, Serialize};
use tspp_memory::{MemoryMap, MemoryRange};
use tspp_mir::TraceMap;

use super::limits::SharedHeapLimits;
use super::usage::SharedHeapUsage;
use crate::shared::gc::{Pacer, SharedMarkWorker};
use crate::shared::storage::{AllocationCache, HeapStorage, HeapStorageImage};
use crate::{
    AccountingRegion, Allocation, AllocationPlan, DropPlan, DropReference, GcAdvance, GcCollector,
    GcDrop, GcPacer, GcPhase, GcState, GcStats, HeapAllocationError, HeapError, HeapGcStateError,
    HeapResult, Payload, Release, SharedHeapOptions, SharedHeapReference, SmallAllocationClass,
    TraceView, apply_byte_delta,
};

/// One live shared heap.
#[derive(Debug)]
pub struct SharedHeap {
    /// The configured shared heap options.
    pub(crate) options: SharedHeapOptions,

    /// The traced shared heap storage.
    pub(crate) storage: HeapStorage,
    /// The exact hard limits for this shared heap.
    pub(crate) limits: SharedHeapLimits,

    /// Whether one shared collection has been requested by pressure or explicitly.
    collection_requested: AtomicBool,
    /// Shared collector pacing state.
    gc_pacer: Pacer,
}

impl SharedHeap {
    /// Set the constant object range, which tracing skips.
    pub fn set_constant_range(&mut self, range: MemoryRange) {
        self.storage.constant = range;
    }
}

/// One frozen shared heap metadata image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SharedHeapImage {
    /// The retained shared heap image state.
    state: Arc<ImageState>,
}

/// One retained shared heap metadata state.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
struct ImageState {
    /// The captured shared heap options.
    options: SharedHeapOptions,
    /// The frozen shared heap storage metadata.
    storage: HeapStorageImage,
}

impl SharedHeapImage {
    /// Create one frozen shared heap metadata image.
    pub(crate) fn new(options: SharedHeapOptions, storage: HeapStorageImage) -> Self {
        let state = ImageState { options, storage };

        Self {
            state: Arc::new(state),
        }
    }

    /// Return the captured shared heap options.
    pub fn options(&self) -> &SharedHeapOptions {
        &self.state.options
    }

    /// Return the shared heap storage metadata.
    pub(crate) fn storage(&self) -> &HeapStorageImage {
        &self.state.storage
    }

    /// Return the captured collector state.
    pub fn gc_state(&self) -> &GcState {
        self.storage().gc_state()
    }

    /// Return the retained shared heap page count.
    pub fn page_count(&self) -> usize {
        self.storage().page_count()
    }

    /// Return the allocated shared heap bytes.
    pub fn allocated_bytes(&self) -> u64 {
        self.storage().allocated_bytes()
    }
}

impl SharedHeap {
    /// Return whether one address lies in this heap's storage.
    pub fn contains(&self, reference: SharedHeapReference) -> bool {
        self.storage.resolve_extent(reference).is_some()
    }

    /// Return the memory map backing this shared heap.
    pub fn memory(&self) -> &Arc<MemoryMap> {
        &self.storage.memory
    }

    /// Create one shared heap over one explicit memory, limits, and options.
    pub fn new(
        memory: Arc<MemoryMap>,
        limits: SharedHeapLimits,
        options: SharedHeapOptions,
    ) -> HeapResult<Self> {
        options.validate()?;

        let shared = Self {
            storage: HeapStorage::new(memory, &options)?,
            options,
            collection_requested: AtomicBool::new(false),
            gc_pacer: Pacer::default(),
            limits,
        };

        shared
            .gc_pacer
            .set_live_bytes(&shared.options, shared.heap_allocated_bytes());
        shared.refresh_gc_request();

        Ok(shared)
    }

    /// Return the configured shared page size.
    pub fn page_size_bytes(&self) -> usize {
        self.storage.page_size_bytes()
    }

    /// Return the configured shared heap options.
    pub fn options(&self) -> &SharedHeapOptions {
        &self.options
    }

    /// Return the configured shared heap limits.
    pub fn limits(&self) -> SharedHeapLimits {
        self.limits
    }

    /// Return the exact live usage for this shared heap.
    pub fn usage(&self) -> SharedHeapUsage {
        self.storage.usage()
    }

    /// Return the exact retained shared memory-page bytes.
    pub fn retained_bytes(&self) -> u64 {
        self.storage.retained_bytes()
    }

    /// Return the current shared heap collector state.
    pub fn gc_state(&self) -> GcState {
        self.storage.gc_state()
    }

    /// Return whether the active shared mark phase is currently drained.
    pub fn mark_idle(&self) -> bool {
        self.storage.mark_idle()
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

    /// Return and consume one bounded shared assist budget in bytes for one memory step.
    pub fn take_assist_budget_bytes(&self) -> usize {
        if self.gc_phase() == GcPhase::Idle {
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
    pub fn gc_phase(&self) -> GcPhase {
        self.storage.gc_phase()
    }

    /// Create one mutator-local shared allocation cache.
    pub fn allocation_cache(&self) -> AllocationCache {
        self.storage.allocation_cache()
    }

    /// Publish and retire every mutator-local shared allocation cache.
    pub fn flush_allocation_cache(&self, cache: &mut AllocationCache) {
        self.storage.flush_allocation_cache(cache);
    }

    /// Allocate one payload from one allocation plan.
    #[inline(always)]
    pub(crate) fn allocate_payload(
        &self,
        worker: &SharedMarkWorker,
        cache: &mut AllocationCache,
        layout: &Allocation<'_>,
        block: Payload<'_>,
        trace_view: TraceView<'_>,
    ) -> HeapResult<SharedHeapReference> {
        // reject invalid blocks
        if layout.is_empty() {
            return Err(HeapError::invalid_allocation(HeapAllocationError::ZeroSize));
        }
        if let Some(actual) = block.byte_len()
            && actual != layout.byte_len
        {
            return Err(HeapError::invalid_allocation(
                HeapAllocationError::ByteLengthMismatch {
                    expected: layout.byte_len,
                    actual,
                },
            ));
        }

        let should_keep_worker_cache = self.gc_phase() == GcPhase::Idle;

        // check heap pressure first
        let retained_byte_delta = self.storage.retained_byte_delta(cache, layout)?;
        self.check_heap_retained_byte_delta(retained_byte_delta)?;

        let pressure_bytes = retained_byte_delta.max(0) as usize;

        // assist before increasing retained heap pressure
        self.assist_allocation(worker, pressure_bytes, trace_view)?;

        // allocate from the shared heap storage
        let reference = self
            .storage
            .allocate(cache, layout, block, should_keep_worker_cache)?;
        if should_keep_worker_cache {
            self.accrue_assist_debt(pressure_bytes);
        }
        self.refresh_gc_request();

        Ok(reference)
    }

    /// Reserve one shared small payload from the worker-local cache.
    #[inline(always)]
    pub fn reserve_small_from_cache(
        &self,
        cache: &mut AllocationCache,
        small: SmallAllocationClass,
    ) -> Option<SharedHeapReference> {
        if self.gc_phase() != GcPhase::Idle {
            return None;
        }

        self.storage.reserve_small_from_cache(cache, small)
    }

    /// Zero one byte range in a live shared heap allocation.
    #[inline(always)]
    pub fn zero(&self, reference: SharedHeapReference, byte_len: usize) -> HeapResult<()> {
        self.storage.zero(reference, byte_len)
    }

    /// Allocate one zeroed payload from one allocation plan.
    #[cold]
    #[inline(never)]
    pub fn allocate_zeroed(
        &self,
        worker: &SharedMarkWorker,
        cache: &mut AllocationCache,
        plan: AllocationPlan,
        trace_map: &TraceMap,
        trace_view: TraceView<'_>,
    ) -> HeapResult<SharedHeapReference> {
        let layout = plan.allocation(trace_map);

        self.allocate_payload(worker, cache, &layout, Payload::Zeroed, trace_view)
    }

    /// Allocate one uninitialized payload from one allocation plan.
    #[cold]
    #[inline(never)]
    pub fn allocate_uninit(
        &self,
        worker: &SharedMarkWorker,
        cache: &mut AllocationCache,
        plan: AllocationPlan,
        trace_map: &TraceMap,
        trace_view: TraceView<'_>,
    ) -> HeapResult<SharedHeapReference> {
        let layout = plan.allocation(trace_map);

        self.allocate_payload(worker, cache, &layout, Payload::Uninit, trace_view)
    }

    /// Allocate one byte-initialized payload from one allocation plan.
    #[cold]
    #[inline(never)]
    pub fn allocate_bytes(
        &self,
        worker: &SharedMarkWorker,
        cache: &mut AllocationCache,
        plan: AllocationPlan,
        trace_map: &TraceMap,
        bytes: &[u8],
        trace_view: TraceView<'_>,
    ) -> HeapResult<SharedHeapReference> {
        let layout = plan.allocation(trace_map);

        self.allocate_payload(worker, cache, &layout, Payload::Bytes(bytes), trace_view)
    }

    /// Return whether one shared heap reference currently refers to one live block.
    pub fn is_heap_live(&self, reference: SharedHeapReference) -> bool {
        self.storage.is_live(reference)
    }

    /// Return the drop plan for one live shared allocation base.
    pub fn drop_plan(
        &self,
        cache: &mut AllocationCache,
        reference: SharedHeapReference,
    ) -> HeapResult<Option<DropPlan>> {
        self.storage.flush_reference_cache(cache, reference);

        self.storage.drop_plan(reference)
    }

    /// Release one uniquely owned shared block.
    pub fn release(
        &self,
        cache: &mut AllocationCache,
        reference: SharedHeapReference,
    ) -> HeapResult<Release> {
        self.storage.flush_reference_cache(cache, reference);
        if self.storage.is_retained(reference)? {
            return Ok(Release::Retained);
        }

        // destroy the values before freeing the storage
        if let Some(plan) = self.storage.drop_plan(reference)? {
            let byte_len = self.storage.byte_len(reference)?;

            return Ok(Release::Destroy { plan, byte_len });
        }
        self.storage.free(reference)?;

        Ok(Release::Freed)
    }

    /// Free one uniquely owned shared block holding no live values.
    pub fn free(
        &self,
        cache: &mut AllocationCache,
        reference: SharedHeapReference,
    ) -> HeapResult<()> {
        self.storage.flush_reference_cache(cache, reference);
        if self.storage.is_retained(reference)? {
            return self.storage.mark_empty(reference);
        }

        self.storage.free(reference).map(|_| ())
    }

    /// Return the base native address for direct shared heap access.
    #[inline(always)]
    pub fn heap_base_address(&self) -> usize {
        self.storage.base_address()
    }

    /// Return the scan metadata for one shared heap reference.
    pub fn trace_map(
        &self,
        reference: SharedHeapReference,
        trace_view: TraceView<'_>,
    ) -> HeapResult<TraceMap> {
        self.storage.trace_map(reference, trace_view)
    }

    /// Record one shared heap write barrier before one byte store.
    pub fn write_barrier_bytes(
        &self,
        reference: SharedHeapReference,
        start: usize,
        bytes: &[u8],
        trace_view: TraceView<'_>,
    ) -> HeapResult<()> {
        self.storage
            .write_barrier_bytes(reference, start, bytes, trace_view)
    }

    /// Record one shared heap write barrier after one completed byte store.
    pub fn write_barrier(
        &self,
        reference: SharedHeapReference,
        start: usize,
        byte_len: usize,
        trace_view: TraceView<'_>,
    ) -> HeapResult<()> {
        self.storage
            .write_barrier(reference, start, byte_len, trace_view)
    }

    /// Request one shared collection cycle at the next world step.
    pub fn request_gc(&self) {
        self.collection_requested.store(true, Ordering::Release);
    }

    /// Return whether one collection is requested for the next safepoint.
    pub fn is_gc_requested(&self) -> bool {
        self.collection_requested.load(Ordering::Acquire)
    }

    /// Start one requested or pressure-driven shared collection cycle.
    pub fn start_gc(&self) -> HeapResult<bool> {
        // active cycle
        if self.gc_phase() != GcPhase::Idle {
            return Ok(false);
        }

        // no pending request
        if !self.collection_requested.load(Ordering::Acquire) {
            return Ok(false);
        }

        // cycle start
        self.gc_pacer
            .begin_cycle(&self.options, self.heap_allocated_bytes());
        self.storage.start_mark(&[])?;
        self.collection_requested.store(false, Ordering::Release);

        Ok(true)
    }

    /// Perform one full shared heap collection over explicit roots.
    ///
    /// Call this at a safepoint after publishing every mutator allocation cache.
    pub fn collect_full<E>(
        &self,
        roots: &[SharedHeapReference],
        trace_view: TraceView<'_>,
        drop: &mut impl FnMut(GcDrop) -> Result<(), E>,
    ) -> Result<GcStats, E>
    where
        E: From<HeapError>,
    {
        self.gc_pacer
            .begin_cycle(&self.options, self.heap_allocated_bytes());

        let stats = self.storage.collect_full(roots, trace_view, drop)?;
        self.record_gc_cycle(stats);

        Ok(stats)
    }

    /// Run one shared collection step with one explicit byte budget.
    pub fn step_collection(
        &self,
        roots: &[SharedHeapReference],
        roots_complete: bool,
        budget_bytes: usize,
        trace_view: TraceView<'_>,
    ) -> HeapResult<GcAdvance> {
        self.step_collection_for_worker(None, roots, roots_complete, budget_bytes, trace_view)
    }

    /// Register one shared mark worker.
    pub fn register_mark_worker(&self) -> SharedMarkWorker {
        self.storage.gc.trace_queue.register_worker()
    }

    /// Run one shared collection step for one worker with one explicit byte budget.
    pub fn step_collection_for_worker(
        &self,
        worker: Option<&SharedMarkWorker>,
        roots: &[SharedHeapReference],
        roots_complete: bool,
        budget_bytes: usize,
        trace_view: TraceView<'_>,
    ) -> HeapResult<GcAdvance> {
        // empty budget
        if budget_bytes == 0 {
            return Ok(GcAdvance::Idle);
        }

        // idle
        if self.gc_phase() == GcPhase::Idle {
            return Ok(GcAdvance::Idle);
        }

        // concurrent mark
        if self.gc_phase() == GcPhase::Mark {
            let work_bytes = self
                .storage
                .step_mark(worker, roots, budget_bytes, trace_view)?;

            // termination check
            if roots_complete {
                self.storage.start_drop_when_drained()?;
            }

            return Ok(GcAdvance::stepped(
                GcCollector::Shared,
                GcPhase::Mark,
                budget_bytes,
                work_bytes,
            ));
        }

        // incremental post-mark work
        let progress = match self.gc_phase() {
            GcPhase::Drop => {
                let progress = self.storage.step_drop(budget_bytes, worker.is_some())?;
                let drop_bytes = progress.work_bytes();

                // continue directly into sweep when Drop completed within this budget
                if self.gc_phase() == GcPhase::Sweep && drop_bytes < budget_bytes {
                    self.storage
                        .step_sweep(budget_bytes - drop_bytes)?
                        .with_prior_work(drop_bytes)
                } else {
                    progress
                }
            }
            GcPhase::Sweep => self.storage.step_sweep(budget_bytes)?,
            GcPhase::Idle | GcPhase::PublishRoots | GcPhase::ScanEdges | GcPhase::Mark => {
                return Err(HeapError::gc_state(HeapGcStateError::SharedGcActive));
            }
        };
        if let Some(stats) = progress.completed_stats() {
            self.record_gc_cycle(stats);
        }

        Ok(progress)
    }

    /// Complete the currently claimed shared value.
    pub fn complete_drop(&self, reference: DropReference) -> HeapResult<()> {
        self.storage.complete_drop(reference)
    }

    /// Fork this shared heap over the same shared memory.
    ///
    /// Call this only from a safepoint where shared heap mutators are stopped.
    pub fn fork(&self, memory: Arc<MemoryMap>) -> HeapResult<Self> {
        Ok(Self {
            options: self.options.clone(),
            collection_requested: AtomicBool::new(
                self.collection_requested.load(Ordering::Acquire),
            ),
            gc_pacer: self.gc_pacer.fork(),
            storage: self.storage.fork(memory)?,
            limits: self.limits,
        })
    }

    /// Restore one shared heap image over its captured world memory and explicit hard limits.
    pub fn from_image_with_limits(
        image: &SharedHeapImage,
        memory: Arc<MemoryMap>,
        limits: SharedHeapLimits,
    ) -> HeapResult<Self> {
        let options = image.options().clone();

        options.validate()?;

        let shared = Self {
            storage: HeapStorage::from_image(memory.clone(), image.storage())?,
            options,
            collection_requested: AtomicBool::new(false),
            gc_pacer: Pacer::default(),
            limits,
        };

        shared
            .gc_pacer
            .set_live_bytes(&shared.options, shared.heap_allocated_bytes());
        shared.refresh_gc_request();
        shared.check_limits()?;

        Ok(shared)
    }

    /// Check the current shared heap usage against the configured limits.
    fn check_limits(&self) -> HeapResult<()> {
        // total limit
        if let Some(max_bytes) = self.limits.max_bytes {
            let retained_bytes = self.retained_bytes();
            if retained_bytes > max_bytes {
                return Err(HeapError::LimitExceeded {
                    region: AccountingRegion::Total,
                    used_bytes: retained_bytes,
                    max_bytes,
                });
            }
        }

        self.limits.check(self.storage.retained_bytes())
    }

    /// Check one projected retained-byte delta against shared heap limits.
    fn check_heap_retained_byte_delta(&self, retained_byte_delta: i64) -> HeapResult<()> {
        // total limit
        if let Some(max_bytes) = self.limits.max_bytes {
            let retained_bytes = apply_byte_delta(self.retained_bytes(), retained_byte_delta);
            if retained_bytes > max_bytes {
                return Err(HeapError::LimitExceeded {
                    region: AccountingRegion::Total,
                    used_bytes: retained_bytes,
                    max_bytes,
                });
            }
        }

        self.limits
            .check_retained_byte_delta(self.storage.retained_bytes(), retained_byte_delta)
    }

    /// Capture one frozen shared heap metadata image.
    pub fn image(&self) -> SharedHeapImage {
        let heap = self.storage.image();

        SharedHeapImage::new(self.options.clone(), heap)
    }

    /// Return the number of live shared heap blocks.
    pub fn heap_allocation_count(&self) -> usize {
        self.storage.allocation_count()
    }

    /// Return the number of allocated shared heap bytes.
    pub fn heap_allocated_bytes(&self) -> u64 {
        self.storage.allocated_bytes()
    }

    /// Refresh the pending shared cycle request from current heap pressure.
    fn refresh_gc_request(&self) {
        // request a cycle under pacer pressure
        if self.gc_pacer().is_pressured(self.heap_allocated_bytes()) {
            self.collection_requested.store(true, Ordering::Release);
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

    /// Accrue shared assist debt from one heap block.
    fn accrue_assist_debt(&self, allocated_bytes: usize) {
        // empty block
        if allocated_bytes == 0 {
            return;
        }

        let gc_pacer = self.gc_pacer();
        let heap_bytes = self.heap_allocated_bytes();
        if heap_bytes < gc_pacer.trigger_bytes && self.gc_phase() == GcPhase::Idle {
            return;
        }

        self.gc_pacer
            .charge_allocation(&self.options, heap_bytes, allocated_bytes);
    }

    /// Run shared collector work proportional to one block.
    fn assist_allocation(
        &self,
        worker: &SharedMarkWorker,
        allocated_bytes: usize,
        trace_view: TraceView<'_>,
    ) -> HeapResult<()> {
        if allocated_bytes == 0 || self.gc_phase() == GcPhase::Idle {
            return Ok(());
        }

        // charge this block into the active mark-assist debt
        self.gc_pacer.charge_allocation(
            &self.options,
            self.heap_allocated_bytes(),
            allocated_bytes,
        );
        let budget_bytes = self.take_assist_budget_bytes();
        if budget_bytes == 0 {
            return Ok(());
        }

        self.step_collection_for_worker(Some(worker), &[], false, budget_bytes, trace_view)?;

        Ok(())
    }
}
