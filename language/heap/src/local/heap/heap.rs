use std::sync::Arc;

use crate::TraceView;
use destack_mir::TraceMap;

use crate::local::storage::HeapStorage;
use crate::{
    Allocation, AllocationPlan, DropPlan, DropReference, GcAdvance, GcDrop, GcPacer, GcPressure,
    GcState, GcStats, HeapError, HeapLimits, HeapOptions, HeapReference, HeapResult, Payload,
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
    /// The pending pacing or explicit collection request.
    pub(super) gc_request: Option<GcRequest>,
    /// The physical local heap storage.
    pub(crate) storage: HeapStorage,
    /// Exact hard limits for this heap.
    pub(super) limits: HeapLimits,
}

impl Heap {
    /// Set the immortal object range, which tracing skips.
    pub fn set_immortal_range(&mut self, range: MemoryRange) {
        self.storage.immortal = range;
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
            gc_request: None,
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

    /// Return whether one heap reference currently refers to young space.
    #[cfg(test)]
    pub(crate) fn is_young(&self, reference: HeapReference) -> bool {
        self.storage.is_young(reference)
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

    /// Stabilize one heap reference in mature space.
    pub fn stabilize(&mut self, reference: HeapReference) -> HeapResult<HeapReference> {
        self.storage.stabilize(reference)
    }

    /// Pin one heap reference against movement.
    pub fn pin(&mut self, reference: HeapReference) -> HeapResult<HeapReference> {
        self.storage.pin(reference)
    }

    /// Release one heap pin.
    pub fn unpin(&mut self, reference: HeapReference) -> HeapResult<()> {
        self.storage.unpin(reference)
    }

    /// Free one heap block immediately.
    pub fn free(&mut self, reference: HeapReference) -> HeapResult<()> {
        self.storage.free(reference)
    }

    /// Return the current derived collector pacing targets.
    pub fn gc_pacer(&self) -> GcPacer {
        self.gc_pacer
    }

    /// Perform one minor heap collection over mutable roots.
    pub fn collect_minor<E>(
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

        let stats = self.storage.collect_minor(roots, trace_view, drop)?;
        self.on_after_gc_cycle(stats);

        Ok(stats)
    }

    /// Perform one full heap collection over mutable roots.
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

    /// Request one minor collection at the next safepoint.
    pub fn request_minor_gc(&mut self) {
        self.request_gc(GcRequest::Minor);
    }

    /// Request one full collection at the next safepoint.
    pub fn request_full_gc(&mut self) {
        self.request_gc(GcRequest::Full);
    }

    /// Return and consume one local collection byte budget.
    pub fn take_collection_budget_bytes(&mut self) -> usize {
        self.refresh_gc_request();

        if !self.storage.major_gc_active()
            && !self.storage.minor_gc_active()
            && self.gc_request.is_none()
        {
            return 0;
        }

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
        if budget_bytes == 0 {
            return Ok(GcAdvance::Idle);
        }

        self.refresh_gc_request();

        // service young GC work
        if self.storage.minor_gc_active() {
            let progress = self
                .storage
                .step_young_gc(roots, budget_bytes, trace_view)?;
            if let Some(stats) = progress.completed_stats() {
                self.on_after_gc_cycle(stats);
            }

            return Ok(progress);
        }

        // service major GC work
        if self.storage.major_gc_active() {
            let progress = self
                .storage
                .step_major_gc(roots, budget_bytes, trace_view)?;
            if let Some(stats) = progress.completed_stats() {
                self.on_after_gc_cycle(stats);
            }

            return Ok(progress);
        }

        // check if there is an active GC request
        let Some(gc_request) = self.gc_request.take() else {
            return Ok(GcAdvance::Idle);
        };

        self.ensure_pacer_cycle();

        // full cycles run as bounded mark and sweep work
        if gc_request == GcRequest::Full {
            self.storage.start_major_gc(roots)?;
            let progress = self
                .storage
                .step_major_gc(roots, budget_bytes, trace_view)?
                .with_start();

            if let Some(stats) = progress.completed_stats() {
                self.on_after_gc_cycle(stats);
            } else {
                self.gc_request = Some(GcRequest::Full);
            }

            Ok(progress)
        }
        // nursery cycles move objects, so drain them at one safepoint
        else {
            self.storage.start_young_gc()?;
            let progress = self
                .storage
                .step_young_gc(roots, budget_bytes, trace_view)?
                .with_start();

            if let Some(stats) = progress.completed_stats() {
                self.on_after_gc_cycle(stats);
            } else {
                self.gc_request = Some(GcRequest::Minor);
            }

            Ok(progress)
        }
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

    /// Reserve one no-scan small payload while no local cycle is active.
    #[inline(always)]
    pub fn reserve_small_noscan(&mut self, site: SmallAllocationPlan) -> Option<HeapReference> {
        // active cycles publish every new block through the slow path
        if self.storage.collector.is_collecting() {
            return None;
        }

        self.storage
            .reserve_young_noscan_cursor(site.byte_len(), site.span_class())
    }

    /// Reserve one scanned small payload while no local cycle is active.
    #[inline(always)]
    pub fn reserve_small_scan(&mut self, site: SmallAllocationPlan) -> Option<HeapReference> {
        // active cycles publish every new block through the slow path
        if self.storage.collector.is_collecting() {
            return None;
        }

        self.storage
            .reserve_young_cursor(site.byte_len(), site.span_class())
    }

    /// Reserve one small payload that may point into shared heap while no local cycle is active.
    #[inline(always)]
    pub fn reserve_small_shared_edge(
        &mut self,
        plan: SmallAllocationPlan,
    ) -> Option<HeapReference> {
        // active cycles publish every new block through the slow path
        if self.storage.collector.is_collecting() {
            return None;
        }

        let reference = self
            .storage
            .reserve_young_cursor(plan.byte_len(), plan.span_class())?;
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
    pub fn is_heap_live(&self, reference: HeapReference) -> bool {
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
        let heap_bytes = self.heap_allocated_bytes();

        // translate pacer pressure into local cycle policy
        match self.gc_pacer.pressure(heap_bytes) {
            GcPressure::Idle => {}
            GcPressure::Cycle => self.request_gc(GcRequest::Full),
            GcPressure::Full => self.request_gc(GcRequest::Full),
        }

        // recycle the nursery once occupancy crosses the trigger
        let young_size_bytes = self.storage.young.used_bytes();
        if young_size_bytes > 0 && young_size_bytes >= self.young_trigger_bytes() {
            self.request_gc(GcRequest::Minor);
        }
    }

    /// Merge one pending request into the current request state.
    fn request_gc(&mut self, request: GcRequest) {
        if self.gc_request == Some(GcRequest::Full) || request == GcRequest::Full {
            self.gc_request = Some(GcRequest::Full);

            return;
        }

        self.gc_request = Some(GcRequest::Minor);
    }

    /// Return the young space byte occupancy that starts minor collection.
    pub(super) fn young_trigger_bytes(&self) -> usize {
        let capacity_bytes = self.storage.young.capacity_bytes;
        let trigger_percent = self.options.gc.trigger_percent as usize;

        capacity_bytes * trigger_percent / 100
    }

    /// Record one completed local collection cycle in the pacer.
    fn on_after_gc_cycle(&mut self, stats: GcStats) {
        self.gc_pacer.record_cycle(self.options.gc, stats);
        self.gc_request = None;
        self.refresh_gc_request();
    }

    /// Ensure the local pacer has one active cycle budget.
    fn ensure_pacer_cycle(&mut self) {
        if self.storage.major_gc_active() || self.storage.minor_gc_active() {
            return;
        }

        if self.gc_pacer.remaining_work_bytes != 0 {
            return;
        }

        self.gc_pacer
            .begin_cycle(self.options.gc, self.heap_allocated_bytes());
    }

    /// Accrue local collector work from heap block pressure.
    fn accrue_assist_debt(&mut self, allocated_bytes: usize) {
        let heap_bytes = self.heap_allocated_bytes();
        if heap_bytes < self.gc_pacer.trigger_bytes && !self.storage.major_gc_active() {
            return;
        }

        self.gc_pacer
            .charge_allocation(self.options.gc, allocated_bytes);
    }
}

/// One pending local GC request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum GcRequest {
    /// Run one young mark-and-sweep cycle.
    Minor,
    /// Run one full mark-and-sweep cycle.
    Full,
}
