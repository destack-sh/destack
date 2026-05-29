use std::sync::Arc;

use destack_mir::{TraceMap, TraceTable};

use crate::allocator::Allocator;
use crate::local::raw::RawSpace;
use crate::local::space::HeapSpace;
use crate::{
    AllocationPlan, AllocationShape, AllocationSite, GcPacer, GcPressure, GcProgress, GcState,
    GcStats, HeapAllocationError, HeapError, HeapLimits, HeapOptions, HeapReference, HeapResult,
    Payload, RawAllocationShape, RawPointer, RootSlot, SharedHeapReference, SmallAllocationSite,
};

/// One live heap over one shared allocator.
#[derive(Debug)]
pub struct Heap {
    /// The configured heap options.
    pub(super) options: HeapOptions,
    /// The derived collector pacing targets.
    pub(super) gc_pacer: GcPacer,
    /// The pending pacing or explicit collection request.
    pub(super) gc_request: Option<GcRequest>,
    /// The heap local allocation space.
    pub(super) heap: HeapSpace,
    /// The raw local allocation space.
    pub(crate) raw: RawSpace,
    /// Exact hard limits for this heap.
    pub(super) limits: HeapLimits,
}

impl Heap {
    /// Create one heap over one explicit allocator, limits, and options.
    pub fn with_allocator_limits_and_options(
        allocator: Arc<Allocator>,
        limits: HeapLimits,
        options: HeapOptions,
    ) -> HeapResult<Self> {
        options.validate_local()?;
        options.validate_allocator(&allocator)?;

        Self::build_with_options(allocator, limits, options)
    }

    /// Create one heap from one checked shared allocator, limits, and options.
    fn build_with_options(
        allocator: Arc<Allocator>,
        limits: HeapLimits,
        options: HeapOptions,
    ) -> HeapResult<Self> {
        let mut heap = Self {
            heap: HeapSpace::build_with_options(allocator.clone(), &options)?,
            raw: RawSpace::with_options(allocator.clone(), &options)?,
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

    /// Return the shared page allocator.
    pub(crate) fn allocator(&self) -> &Arc<Allocator> {
        self.heap.allocator()
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
        self.check_retained_byte_delta(0, 0)
    }

    /// Return the currently live heap references.
    pub fn live_references(&self) -> HeapResult<Vec<HeapReference>> {
        self.heap.live_references()
    }

    /// Return whether one heap reference currently refers to young space.
    #[cfg(test)]
    pub(crate) fn is_young(&self, reference: HeapReference) -> bool {
        self.heap.is_young(reference)
    }

    /// Return the current collector state.
    pub fn gc_state(&self) -> &GcState {
        self.heap.gc_state()
    }

    /// Start one incremental local-to-shared edge scan.
    pub fn start_shared_edge_scan(&mut self) {
        self.heap.start_shared_edge_scan();
    }

    /// Return whether the current local-to-shared edge scan is drained.
    pub fn shared_edge_scan_idle(&self) -> bool {
        self.heap.shared_edge_scan_idle()
    }

    /// Finish the current local-to-shared edge scan.
    pub fn finish_shared_edge_scan(&mut self) {
        self.heap.finish_shared_edge_scan();
    }

    /// Scan bounded local-to-shared edge work into the provided root buffer.
    pub fn scan_shared_references(
        &mut self,
        roots: &mut Vec<SharedHeapReference>,
        budget_bytes: usize,
        trace_table: &TraceTable,
    ) -> HeapResult<usize> {
        self.heap
            .scan_shared_references(roots, budget_bytes, trace_table)
    }

    /// Stabilize one heap reference in mature space.
    pub fn stabilize(&mut self, reference: HeapReference) -> HeapResult<HeapReference> {
        self.heap.stabilize(reference)
    }

    /// Pin one heap reference against movement.
    pub fn pin(&mut self, reference: HeapReference) -> HeapResult<HeapReference> {
        self.heap.pin(reference)
    }

    /// Release one heap pin.
    pub fn unpin(&mut self, reference: HeapReference) -> HeapResult<()> {
        self.heap.unpin(reference)
    }

    /// Free one heap allocation immediately.
    pub fn free(&mut self, reference: HeapReference) -> HeapResult<()> {
        self.heap.free(reference)
    }

    /// Return the current derived collector pacing targets.
    pub fn gc_pacer(&self) -> GcPacer {
        self.gc_pacer
    }

    /// Perform one minor heap collection over mutable roots.
    pub fn collect_minor<E>(
        &mut self,
        roots: &mut impl FnMut(&mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>) -> Result<(), E>,
        trace_table: &TraceTable,
    ) -> Result<GcStats, E>
    where
        E: From<HeapError>,
    {
        self.gc_pacer
            .begin_cycle(self.options.gc, self.heap_allocated_bytes());

        let stats = self.heap.collect_minor(roots, trace_table)?;
        self.on_after_gc_cycle(stats);

        Ok(stats)
    }

    /// Perform one full heap collection over mutable roots.
    pub fn collect_full<E>(
        &mut self,
        roots: &mut impl FnMut(&mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>) -> Result<(), E>,
        trace_table: &TraceTable,
    ) -> Result<GcStats, E>
    where
        E: From<HeapError>,
    {
        self.gc_pacer
            .begin_cycle(self.options.gc, self.heap_allocated_bytes());

        let stats = self.heap.collect_full(roots, trace_table)?;
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

        if !self.heap.major_gc_active() && !self.heap.young_gc_active() && self.gc_request.is_none()
        {
            return 0;
        }

        self.ensure_pacer_cycle();

        self.gc_pacer.budget_bytes(self.options.gc, 1)
    }

    /// Run one local collection step within one byte budget.
    pub fn collect_step<E>(
        &mut self,
        roots: &mut impl FnMut(&mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>) -> Result<(), E>,
        budget_bytes: usize,
        trace_table: &TraceTable,
    ) -> Result<GcProgress, E>
    where
        E: From<HeapError>,
    {
        if budget_bytes == 0 {
            return Ok(GcProgress::Idle);
        }

        self.refresh_gc_request();

        // service young GC work
        if self.heap.young_gc_active() {
            let progress = self.heap.step_young_gc(roots, budget_bytes, trace_table)?;
            if let Some(stats) = progress.completed_stats() {
                self.on_after_gc_cycle(stats);
            }

            return Ok(progress);
        }

        // service major GC work
        if self.heap.major_gc_active() {
            let progress = self.heap.step_major_gc(roots, budget_bytes, trace_table)?;
            if let Some(stats) = progress.completed_stats() {
                self.on_after_gc_cycle(stats);
            }

            return Ok(progress);
        }

        // check if there is an active GC request
        let Some(gc_request) = self.gc_request.take() else {
            return Ok(GcProgress::Idle);
        };

        self.ensure_pacer_cycle();

        // full cycles run as bounded mark and sweep work
        if gc_request == GcRequest::Full {
            self.heap.start_major_gc(roots)?;
            let progress = self.heap.step_major_gc(roots, budget_bytes, trace_table)?;

            if let Some(stats) = progress.completed_stats() {
                self.on_after_gc_cycle(stats);
            } else {
                self.gc_request = Some(GcRequest::Full);
            }

            Ok(progress)
        }
        // nursery cycles move objects, so drain them at one safepoint
        else {
            self.heap.start_young_gc()?;
            let progress = self.heap.step_young_gc(roots, budget_bytes, trace_table)?;

            if let Some(stats) = progress.completed_stats() {
                self.on_after_gc_cycle(stats);
            } else {
                self.gc_request = Some(GcRequest::Minor);
            }

            Ok(progress)
        }
    }

    /// Allocate one zeroed dynamic managed heap allocation.
    #[inline(always)]
    pub fn allocate_dynamic_zeroed(
        &mut self,
        shape: AllocationShape<'_>,
    ) -> HeapResult<HeapReference> {
        let layout = self.heap.allocation_plan(shape);

        self.allocate_dynamic_payload(&layout, Payload::Zeroed)
    }

    /// Allocate one uninitialized dynamic managed heap allocation.
    #[inline(always)]
    pub fn allocate_dynamic_uninit(
        &mut self,
        shape: AllocationShape<'_>,
    ) -> HeapResult<HeapReference> {
        let layout = self.heap.allocation_plan(shape);

        self.allocate_dynamic_payload(&layout, Payload::Uninit)
    }

    /// Allocate one byte-initialized dynamic managed heap allocation.
    #[inline(always)]
    pub fn allocate_dynamic_bytes(
        &mut self,
        shape: AllocationShape<'_>,
        bytes: &[u8],
    ) -> HeapResult<HeapReference> {
        let layout = self.heap.allocation_plan(shape);

        self.allocate_dynamic_payload(&layout, Payload::Bytes(bytes))
    }

    /// Allocate one allocator-planned dynamic payload.
    #[inline(always)]
    pub(crate) fn allocate_dynamic_payload(
        &mut self,
        layout: &AllocationPlan<'_>,
        allocation: Payload<'_>,
    ) -> HeapResult<HeapReference> {
        if layout.is_empty() {
            return Err(HeapError::invalid_allocation(HeapAllocationError::ZeroSize));
        }

        if let Some(actual) = allocation.byte_len()
            && actual != layout.byte_len
        {
            return Err(HeapError::invalid_allocation(
                HeapAllocationError::ByteLengthMismatch {
                    expected: layout.byte_len,
                    actual,
                },
            ));
        }

        if let Some(reference) = self.heap.reserve_young_payload(layout, allocation)? {
            return Ok(reference);
        }

        let retained_byte_delta = self.heap.retained_byte_delta(layout)?;

        // check the projected heap retained-byte delta first
        self.check_retained_byte_delta(retained_byte_delta, 0)?;

        // then allocate from mature space
        let has_initialized_bytes = allocation.byte_len().is_some();
        let reference =
            self.heap
                .allocate_mature_layout(layout, allocation, has_initialized_bytes)?;
        self.accrue_assist_debt(layout.byte_len);
        self.refresh_gc_request();

        Ok(reference)
    }

    /// Reserve one zeroed noscan small payload from the active young run.
    #[inline(always)]
    pub fn reserve_small_noscan_zeroed(
        &mut self,
        allocation: SmallAllocationSite,
    ) -> Option<HeapReference> {
        self.reserve_small_noscan(allocation)
    }

    /// Reserve one uninitialized noscan small payload from the active young run.
    #[inline(always)]
    pub fn reserve_small_noscan_uninit(
        &mut self,
        allocation: SmallAllocationSite,
    ) -> Option<HeapReference> {
        self.reserve_small_noscan(allocation)
    }

    /// Reserve one zeroed scanned small payload from the active young run.
    #[inline(always)]
    pub fn reserve_small_scan_zeroed(
        &mut self,
        allocation: SmallAllocationSite,
    ) -> Option<HeapReference> {
        self.reserve_small_scan(allocation)
    }

    /// Reserve one uninitialized scanned small payload from the active young run.
    #[inline(always)]
    pub fn reserve_small_scan_uninit(
        &mut self,
        allocation: SmallAllocationSite,
    ) -> Option<HeapReference> {
        self.reserve_small_scan(allocation)
    }

    /// Reserve one zeroed small payload that may point into shared heap.
    #[inline(always)]
    pub fn reserve_small_shared_edge_zeroed(
        &mut self,
        allocation: SmallAllocationSite,
    ) -> Option<HeapReference> {
        self.reserve_small_shared_edge(allocation)
    }

    /// Reserve one uninitialized small payload that may point into shared heap.
    #[inline(always)]
    pub fn reserve_small_shared_edge_uninit(
        &mut self,
        allocation: SmallAllocationSite,
    ) -> Option<HeapReference> {
        self.reserve_small_shared_edge(allocation)
    }

    /// Reserve one no-scan small payload.
    #[inline(always)]
    fn reserve_small_noscan(&mut self, allocation: SmallAllocationSite) -> Option<HeapReference> {
        self.heap
            .reserve_young_noscan_run_cursor(allocation.byte_len, allocation.span_class())
    }

    /// Reserve one scanned small payload.
    #[inline(always)]
    fn reserve_small_scan(&mut self, allocation: SmallAllocationSite) -> Option<HeapReference> {
        if self.heap.major_gc_active() {
            return None;
        }

        self.heap
            .reserve_young_run_cursor(allocation.byte_len, allocation.span_class())
    }

    /// Reserve one small payload that may point into shared heap.
    #[inline(always)]
    fn reserve_small_shared_edge(
        &mut self,
        allocation: SmallAllocationSite,
    ) -> Option<HeapReference> {
        if self.heap.major_gc_active() {
            return None;
        }

        let reference = self
            .heap
            .reserve_young_run_cursor(allocation.byte_len, allocation.span_class())?;
        self.heap.collector.track_shared_edge_root(reference);

        Some(reference)
    }

    /// Allocate one zeroed payload from one allocation site.
    #[cold]
    #[inline(never)]
    pub fn allocate_zeroed(
        &mut self,
        allocation: AllocationSite,
        trace_map: &TraceMap,
    ) -> HeapResult<HeapReference> {
        let layout = allocation.plan(trace_map);

        self.allocate_dynamic_payload(&layout, Payload::Zeroed)
    }

    /// Allocate one uninitialized payload from one allocation site.
    #[cold]
    #[inline(never)]
    pub fn allocate_uninit(
        &mut self,
        allocation: AllocationSite,
        trace_map: &TraceMap,
    ) -> HeapResult<HeapReference> {
        let layout = allocation.plan(trace_map);

        self.allocate_dynamic_payload(&layout, Payload::Uninit)
    }

    /// Resolve one allocation shape to one compiled allocation site.
    #[inline(always)]
    pub fn allocation_site(&self, shape: AllocationShape<'_>) -> AllocationSite {
        self.heap.allocation_plan(shape).site()
    }

    /// Resolve one allocation shape against this heap.
    #[inline(always)]
    pub fn allocation_plan<'a>(&self, shape: AllocationShape<'a>) -> AllocationPlan<'a> {
        self.heap.allocation_plan(shape)
    }

    /// Allocate one raw allocation.
    pub fn allocate_raw(
        &mut self,
        shape: RawAllocationShape,
        allocation: Payload<'_>,
    ) -> HeapResult<RawPointer> {
        let retained_byte_delta = self.raw.alloc_retained_byte_delta(shape);

        // check the projected raw retained-byte delta first
        self.check_retained_byte_delta(0, retained_byte_delta)?;

        // then allocate through raw space
        self.raw.allocate(shape, allocation)
    }

    /// Return whether one heap reference currently refers to one live allocation.
    pub fn is_heap_live(&self, reference: HeapReference) -> bool {
        self.heap.is_live(reference)
    }

    /// Return the base native address for direct managed heap access.
    #[inline(always)]
    pub fn heap_base_address(&self) -> usize {
        self.heap.base_address()
    }

    /// Return the base native address for direct raw heap access.
    #[inline(always)]
    pub fn raw_base_address(&self) -> usize {
        self.raw.base_address()
    }

    /// Return the heap scan metadata for one heap allocation.
    pub fn scan(&self, reference: HeapReference, trace_table: &TraceTable) -> HeapResult<TraceMap> {
        self.heap.scan(reference, trace_table)
    }

    /// Record one heap write barrier over one byte range.
    pub fn write_barrier(
        &mut self,
        reference: HeapReference,
        start: usize,
        byte_len: usize,
        trace_table: &TraceTable,
    ) -> HeapResult<()> {
        self.heap
            .write_barrier(reference, start, byte_len, trace_table)
    }

    /// Return old and new shared edges for one heap store before it writes.
    pub fn shared_write_barrier_bytes(
        &self,
        reference: HeapReference,
        start: usize,
        bytes: &[u8],
        trace_table: &TraceTable,
    ) -> HeapResult<Vec<SharedHeapReference>> {
        self.heap
            .shared_write_barrier_bytes(reference, start, bytes, trace_table)
    }

    /// Return the bytes for one raw allocation as one owned vector.
    pub fn read_raw_bytes(&self, pointer: RawPointer) -> HeapResult<Vec<u8>> {
        self.raw.read_bytes(pointer)
    }

    /// Return the remaining byte length for one raw allocation.
    pub fn raw_byte_len(&self, pointer: RawPointer) -> HeapResult<usize> {
        self.raw.byte_len(pointer)
    }

    /// Fill one caller-provided buffer from one raw allocation at one offset.
    pub fn read_raw_bytes_into(
        &self,
        pointer: RawPointer,
        start: usize,
        target: &mut [u8],
    ) -> HeapResult<()> {
        self.raw.read_bytes_into(pointer, start, target)
    }

    /// Return one checked address for a raw byte range.
    pub fn raw_address(
        &self,
        pointer: RawPointer,
        start: usize,
        byte_len: usize,
    ) -> HeapResult<*const u8> {
        self.raw.address(pointer, start, byte_len)
    }

    /// Return one checked writable address for a raw byte range.
    pub fn raw_address_mut(
        &mut self,
        pointer: RawPointer,
        start: usize,
        byte_len: usize,
    ) -> HeapResult<*mut u8> {
        self.raw.address_mut(pointer, start, byte_len)
    }

    /// Replace the bytes for one raw allocation.
    pub fn replace_raw_bytes(
        &mut self,
        pointer: RawPointer,
        bytes: &[u8],
    ) -> HeapResult<RawPointer> {
        // check the projected replacement retained-byte delta next
        self.check_raw_replace_retained_byte_delta(pointer, bytes.len())?;

        // then replace the raw payload
        self.raw.replace_bytes(pointer, bytes)
    }

    /// Overwrite one raw byte range.
    pub fn write_raw_bytes(
        &mut self,
        pointer: RawPointer,
        start: usize,
        bytes: &[u8],
    ) -> HeapResult<()> {
        self.raw.write_bytes(pointer, start, bytes)
    }

    /// Overwrite one raw byte.
    pub fn write_raw_byte(
        &mut self,
        pointer: RawPointer,
        index: usize,
        byte: u8,
    ) -> HeapResult<()> {
        self.write_raw_bytes(pointer, index, &[byte])
    }

    /// Free one raw allocation.
    pub fn free_raw(&mut self, pointer: RawPointer) -> HeapResult<()> {
        self.raw.free(pointer)
    }

    /// Check the projected retained-byte delta for one raw replacement.
    fn check_raw_replace_retained_byte_delta(
        &self,
        pointer: RawPointer,
        next_len: usize,
    ) -> HeapResult<()> {
        let retained_byte_delta = self.raw.replace_retained_byte_delta(pointer, next_len)?;

        self.check_retained_byte_delta(0, retained_byte_delta)
    }

    /// Check heap limits after one requested retained-byte delta.
    fn check_retained_byte_delta(
        &self,
        heap_retained_byte_delta: i64,
        raw_retained_byte_delta: i64,
    ) -> HeapResult<()> {
        self.limits.check_retained_byte_delta(
            self.heap.retained_bytes(),
            self.raw.retained_bytes(),
            heap_retained_byte_delta,
            raw_retained_byte_delta,
        )
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

        // recycle the nursery only when it fits the hard local quantum
        let young_size_bytes = self.heap.young.used_bytes();
        let nursery_fits_quantum =
            young_size_bytes > 0 && young_size_bytes <= self.options.gc.minimum_work_bytes;
        if young_size_bytes >= self.young_trigger_bytes() && nursery_fits_quantum {
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

    /// Return the young-space byte occupancy that starts minor collection.
    pub(super) fn young_trigger_bytes(&self) -> usize {
        let capacity_bytes = self.heap.young.capacity_bytes;
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
        if self.heap.major_gc_active() || self.heap.young_gc_active() {
            return;
        }

        if self.gc_pacer.remaining_work_bytes != 0 {
            return;
        }

        self.gc_pacer
            .begin_cycle(self.options.gc, self.heap_allocated_bytes());
    }

    /// Accrue local collector work from heap allocation pressure.
    fn accrue_assist_debt(&mut self, allocated_bytes: usize) {
        let heap_bytes = self.heap_allocated_bytes();
        if heap_bytes < self.gc_pacer.trigger_bytes && !self.heap.major_gc_active() {
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
