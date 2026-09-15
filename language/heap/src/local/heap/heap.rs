use std::sync::Arc;

use crate::TraceView;
use destack_mir::TraceMap;

use crate::local::storage::HeapStorage;
use crate::{
    Allocation, AllocationPlan, DropPlan, DropReference, GcAdvance, GcDrop, GcPacer, GcState,
    GcStats, HeapError, HeapLimits, HeapOptions, HeapReference, HeapResult, Payload, Release,
    RootSlot, SharedHeapReference, SmallAllocationPlan,
};
use destack_memory::{MemoryMap, MemoryRange};

/// One live heap over one shared memory.
#[derive(Debug)]
pub struct Heap {
    /// The configured heap options.
    pub(super) options: HeapOptions,
    /// The derived collector pacing targets.
    pub(super) gc_pacer: GcPacer,
    /// Whether one collection is requested for the next safepoint.
    pub(super) is_gc_requested: bool,
    /// The physical local heap storage.
    pub(crate) storage: HeapStorage,
    /// Exact hard limits for this heap.
    pub(super) limits: HeapLimits,
}

impl Heap {
    /// Set the constant object range, which tracing skips.
    pub fn set_constant_range(&mut self, range: MemoryRange) {
        self.storage.constant = range;
    }

    /// Create one heap over one explicit memory, limits, and options.
    pub fn new(
        memory: Arc<MemoryMap>,
        limits: HeapLimits,
        options: HeapOptions,
    ) -> HeapResult<Self> {
        options.validate_local()?;

        let mut heap = Self {
            storage: HeapStorage::new(memory, &options)?,
            options,
            gc_pacer: GcPacer::default(),
            is_gc_requested: false,
            limits,
        };

        heap.gc_pacer
            .set_live_bytes(heap.options.gc, heap.heap_allocated_bytes());
        heap.refresh_gc_request();

        Ok(heap)
    }

    /// Return the heap options.
    pub fn options(&self) -> &HeapOptions {
        &self.options
    }

    /// Return the configured heap limits.
    pub fn limits(&self) -> HeapLimits {
        self.limits
    }

    /// Replace the heap hard limits.
    pub fn set_limits(&mut self, limits: HeapLimits) -> HeapResult<()> {
        self.limits = limits;
        self.check_limits()
    }

    /// Check the configured heap hard limits against current usage.
    pub fn check_limits(&self) -> HeapResult<()> {
        self.check_retained_byte_delta(0)
    }

    /// Return the currently live heap references.
    pub fn live_references(&self) -> HeapResult<Vec<HeapReference>> {
        self.storage.live_references()
    }

    /// Return the current collector state.
    pub fn gc_state(&self) -> &GcState {
        self.storage.gc_state()
    }

    /// Start one incremental local-to-shared edge scan.
    pub fn start_shared_edge_scan(&mut self) {
        self.storage.start_shared_edge_scan();
    }

    /// Return whether the current local-to-shared edge scan is drained.
    pub fn shared_edge_scan_idle(&self) -> bool {
        self.storage.shared_edge_scan_idle()
    }

    /// Finish the current local-to-shared edge scan.
    pub fn finish_shared_edge_scan(&mut self) {
        self.storage.finish_shared_edge_scan();
    }

    /// Trace bounded local-to-shared edges into the provided root buffer.
    pub fn trace_shared_roots(
        &mut self,
        roots: &mut Vec<SharedHeapReference>,
        budget_bytes: usize,
        trace_view: TraceView<'_>,
    ) -> HeapResult<usize> {
        self.storage
            .trace_shared_roots(roots, budget_bytes, trace_view)
    }

    /// Release one uniquely owned block.
    pub fn release(&mut self, reference: HeapReference) -> HeapResult<Release> {
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

    /// Free one uniquely owned block holding no live values.
    pub fn free(&mut self, reference: HeapReference) -> HeapResult<()> {
        if self.storage.is_retained(reference)? {
            return self.storage.mark_empty(reference);
        }

        self.storage.free(reference)
    }

    /// Return the current derived collector pacing targets.
    pub fn gc_pacer(&self) -> GcPacer {
        self.gc_pacer
    }

    /// Perform one complete collection over mutable roots.
    pub fn collect_full<E>(
        &mut self,
        roots: &mut impl FnMut(&mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>) -> Result<(), E>,
        trace_view: TraceView<'_>,
        drop: &mut impl FnMut(GcDrop) -> Result<(), E>,
    ) -> Result<GcStats, E>
    where
        E: From<HeapError>,
    {
        self.gc_pacer
            .begin_cycle(self.options.gc, self.heap_allocated_bytes());

        let stats = self.storage.collect_full(roots, trace_view, drop)?;
        self.on_after_gc_cycle(stats);

        Ok(stats)
    }

    /// Request one collection at the next safepoint.
    pub fn request_gc(&mut self) {
        self.is_gc_requested = true;
    }

    /// Return whether one collection is requested for the next safepoint.
    pub fn is_gc_requested(&self) -> bool {
        self.is_gc_requested
    }

    /// Return and consume one local collection byte budget.
    pub fn take_collection_budget_bytes(&mut self) -> usize {
        // an idle heap owes no collector work
        self.refresh_gc_request();
        if !self.storage.collector.is_collecting() && !self.is_gc_requested {
            return 0;
        }

        // charge the budget against the active pacer cycle
        self.ensure_pacer_cycle();

        self.gc_pacer.budget_bytes(self.options.gc, 1)
    }

    /// Run one local collection step within one byte budget.
    pub fn step_collection<E>(
        &mut self,
        roots: &mut impl FnMut(&mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>) -> Result<(), E>,
        budget_bytes: usize,
        trace_view: TraceView<'_>,
    ) -> Result<GcAdvance, E>
    where
        E: From<HeapError>,
    {
        // an empty budget buys no work
        if budget_bytes == 0 {
            return Ok(GcAdvance::Idle);
        }

        // re-read the pacer before choosing what to service
        self.refresh_gc_request();

        // service active cycle work
        if self.storage.collector.is_collecting() {
            let progress = self.storage.step_gc(roots, budget_bytes, trace_view)?;
            if let Some(stats) = progress.completed_stats() {
                self.on_after_gc_cycle(stats);
            }

            return Ok(progress);
        }

        // stay idle until a cycle is requested
        if !self.is_gc_requested {
            return Ok(GcAdvance::Idle);
        }

        // start the requested cycle as bounded mark and sweep work
        self.ensure_pacer_cycle();
        self.storage.start_gc(roots)?;
        let progress = self
            .storage
            .step_gc(roots, budget_bytes, trace_view)?
            .with_start();

        // record a cycle that completed within this budget
        if let Some(stats) = progress.completed_stats() {
            self.on_after_gc_cycle(stats);
        }

        Ok(progress)
    }

    /// Complete the currently claimed local value.
    pub fn complete_drop(&mut self, reference: DropReference) -> HeapResult<()> {
        self.storage.collector.complete_drop(reference)
    }

    /// Allocate one payload from one allocation plan.
    #[inline(always)]
    pub(crate) fn allocate_payload(
        &mut self,
        layout: &Allocation<'_>,
        payload: Payload<'_>,
    ) -> HeapResult<HeapReference> {
        let retained_byte_delta = self.storage.retained_byte_delta(layout)?;

        // check the projected heap retained-byte delta first
        self.check_retained_byte_delta(retained_byte_delta)?;

        // allocate after limits are known
        let reference = self.storage.allocate(layout, payload)?;
        self.accrue_assist_debt(layout.byte_len);
        self.refresh_gc_request();

        Ok(reference)
    }

    /// Reserve one small payload from its class span while no local cycle is active.
    #[inline(always)]
    pub fn reserve_small(&mut self, plan: SmallAllocationPlan) -> Option<HeapReference> {
        // active cycles publish every new block through the slow path
        if self.storage.collector.is_collecting() {
            return None;
        }

        // claim one slot from the class span
        let slot = self.storage.reserve_slot(plan.small)?;

        Some(self.storage.slot_reference(slot))
    }

    /// Reserve one small payload that may point into shared heap while no local cycle is active.
    #[inline(always)]
    pub fn reserve_small_shared_edge(
        &mut self,
        plan: SmallAllocationPlan,
    ) -> Option<HeapReference> {
        let reference = self.reserve_small(plan)?;
        self.storage.collector.track_shared_edge_root(reference);

        Some(reference)
    }

    /// Zero one byte range in a live heap allocation.
    #[inline(always)]
    pub fn zero(&self, reference: HeapReference, byte_len: usize) -> HeapResult<()> {
        self.storage.zero(reference, byte_len)
    }

    /// Allocate one zeroed payload from one allocation plan.
    #[cold]
    #[inline(never)]
    pub fn allocate_zeroed(
        &mut self,
        plan: AllocationPlan,
        trace_map: &TraceMap,
    ) -> HeapResult<HeapReference> {
        let layout = plan.allocation(trace_map);

        self.allocate_payload(&layout, Payload::Zeroed)
    }

    /// Allocate one uninitialized payload from one allocation plan.
    #[cold]
    #[inline(never)]
    pub fn allocate_uninit(
        &mut self,
        plan: AllocationPlan,
        trace_map: &TraceMap,
    ) -> HeapResult<HeapReference> {
        let layout = plan.allocation(trace_map);

        self.allocate_payload(&layout, Payload::Uninit)
    }

    /// Allocate one byte-initialized payload from one allocation plan.
    #[cold]
    #[inline(never)]
    pub fn allocate_bytes(
        &mut self,
        plan: AllocationPlan,
        trace_map: &TraceMap,
        bytes: &[u8],
    ) -> HeapResult<HeapReference> {
        let layout = plan.allocation(trace_map);

        self.allocate_payload(&layout, Payload::Bytes(bytes))
    }

    /// Return whether one heap reference currently refers to one live block.
    pub fn is_live(&self, reference: HeapReference) -> bool {
        self.storage.is_live(reference)
    }

    /// Return the drop plan for one live allocation base.
    pub fn drop_plan(&self, reference: HeapReference) -> HeapResult<Option<DropPlan>> {
        self.storage.drop_plan(reference)
    }

    /// Return the base native address for direct heap access.
    #[inline(always)]
    pub fn heap_base_address(&self) -> usize {
        self.storage.base_address()
    }

    /// Return the heap scan metadata for one heap block.
    pub fn trace_map(
        &self,
        reference: HeapReference,
        trace_view: TraceView<'_>,
    ) -> HeapResult<TraceMap> {
        self.storage.trace_map(reference, trace_view)
    }

    /// Record one heap write barrier over one byte range.
    pub fn write_barrier(
        &mut self,
        reference: HeapReference,
        start: usize,
        byte_len: usize,
        trace_view: TraceView<'_>,
    ) -> HeapResult<()> {
        self.storage
            .write_barrier(reference, start, byte_len, trace_view)
    }

    /// Return old and new shared edges for one heap store before it writes.
    pub fn shared_write_barrier_bytes(
        &self,
        reference: HeapReference,
        start: usize,
        bytes: &[u8],
        trace_view: TraceView<'_>,
    ) -> HeapResult<Vec<SharedHeapReference>> {
        self.storage
            .shared_write_barrier_bytes(reference, start, bytes, trace_view)
    }

    /// Check heap limits after one requested retained-byte delta.
    fn check_retained_byte_delta(&self, heap_retained_byte_delta: i64) -> HeapResult<()> {
        self.limits
            .check_retained_byte_delta(self.storage.retained_bytes(), heap_retained_byte_delta)
    }

    /// Refresh the pending collection request from current heap state.
    pub(crate) fn refresh_gc_request(&mut self) {
        // request a cycle under pacer pressure
        if self.gc_pacer.is_pressured(self.heap_allocated_bytes()) {
            self.request_gc();
        }
    }

    /// Record one completed local collection cycle in the pacer.
    fn on_after_gc_cycle(&mut self, stats: GcStats) {
        self.gc_pacer.record_cycle(self.options.gc, stats);
        self.is_gc_requested = false;
        self.refresh_gc_request();
    }

    /// Ensure the local pacer has one active cycle budget.
    fn ensure_pacer_cycle(&mut self) {
        // an active cycle already holds a budget
        if self.storage.collector.is_collecting() {
            return;
        }

        // the current cycle still has work left
        if self.gc_pacer.remaining_work_bytes != 0 {
            return;
        }

        self.gc_pacer
            .begin_cycle(self.options.gc, self.heap_allocated_bytes());
    }

    /// Accrue local collector work from heap block pressure.
    fn accrue_assist_debt(&mut self, allocated_bytes: usize) {
        let heap_bytes = self.heap_allocated_bytes();
        if heap_bytes < self.gc_pacer.trigger_bytes && !self.storage.collector.is_collecting() {
            return;
        }

        self.gc_pacer
            .charge_allocation(self.options.gc, allocated_bytes);
    }
}
