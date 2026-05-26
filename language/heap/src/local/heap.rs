use std::sync::Arc;

use destack_mir::TraceMap;

use crate::allocator::Allocator;
use crate::local::raw::RawSpace;
use crate::local::space::HeapSpace;
use crate::{
    AllocationPlan, AllocationShape, GcPacer, GcPressure, GcProgress, GcState, GcStats, HeapError,
    HeapLimits, HeapOptions, HeapReference, HeapResult, Payload, RawAllocationShape, RawPointer,
    RootSet, SharedHeapReference,
};

/// One pending local GC request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum GcRequest {
    /// Run one minor cycle.
    Minor,
    /// Run one full cycle.
    Full,
}

/// One live heap over one shared allocator.
#[derive(Debug)]
pub struct Heap {
    /// The shared page allocator for every local byte payload.
    pub(super) allocator: Arc<Allocator>,
    /// The configured heap options.
    pub(super) options: HeapOptions,
    /// The derived collector pacing targets.
    pub(super) gc_pacer: GcPacer,
    /// The young-space occupancy that starts a minor collection request.
    pub(super) young_trigger_bytes: usize,
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
            allocator,
            options,
            gc_pacer: GcPacer::default(),
            young_trigger_bytes: 0,
            gc_request: None,
            limits,
        };

        heap.young_trigger_bytes = heap.young_trigger_bytes();
        heap.gc_pacer
            .set_live_bytes(heap.options.gc, heap.heap_allocated_bytes());
        heap.refresh_gc_request();

        Ok(heap)
    }

    /// Return the shared page allocator.
    pub(crate) fn allocator(&self) -> &Arc<Allocator> {
        &self.allocator
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
    ) -> HeapResult<usize> {
        self.heap.scan_shared_references(roots, budget_bytes)
    }

    /// Stabilize one heap reference in mature space.
    pub fn stabilize_heap(&mut self, reference: HeapReference) -> HeapResult<HeapReference> {
        self.heap.stabilize(reference)
    }

    /// Pin one heap reference against movement.
    pub fn pin_heap(&mut self, reference: HeapReference) -> HeapResult<HeapReference> {
        self.heap.pin(reference)
    }

    /// Release one heap pin.
    pub fn unpin_heap(&mut self, reference: HeapReference) -> HeapResult<()> {
        self.heap.unpin(reference)
    }

    /// Free one heap allocation immediately.
    pub fn free_heap(&mut self, reference: HeapReference) -> HeapResult<()> {
        self.heap.free(reference)
    }

    /// Return the current derived collector pacing targets.
    pub fn gc_pacer(&self) -> GcPacer {
        self.gc_pacer
    }

    /// Perform one minor heap collection over mutable roots.
    pub fn collect_minor<R>(&mut self, roots: &mut R) -> Result<GcStats, R::Error>
    where
        R: RootSet,
    {
        self.gc_pacer
            .begin_cycle(self.options.gc, self.heap_allocated_bytes());

        let stats = self.heap.collect_minor(roots)?;
        self.on_after_gc_cycle(stats);

        Ok(stats)
    }

    /// Perform one full heap collection over mutable roots.
    pub fn collect_full<R>(&mut self, roots: &mut R) -> Result<GcStats, R::Error>
    where
        R: RootSet,
    {
        self.gc_pacer
            .begin_cycle(self.options.gc, self.heap_allocated_bytes());

        let stats = self.heap.collect_full(roots)?;
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

        let budget_bytes = self.gc_pacer.budget_bytes(self.options.gc, 1);

        match self.gc_request {
            Some(GcRequest::Minor | GcRequest::Full) => {
                budget_bytes.max(self.heap.young.used_bytes())
            }
            None => budget_bytes,
        }
    }

    /// Run one local collection step within one byte budget.
    pub fn collect_step<R>(
        &mut self,
        roots: &mut R,
        budget_bytes: usize,
    ) -> Result<GcProgress, R::Error>
    where
        R: RootSet,
    {
        if budget_bytes == 0 {
            return Ok(GcProgress::Idle);
        }

        self.refresh_gc_request();

        // service major GC work
        if self.heap.major_gc_active() {
            let progress = self.heap.step_major_gc(roots, budget_bytes)?;
            if let Some(stats) = progress.completed_stats() {
                self.on_after_gc_cycle(stats);
            }

            return Ok(progress);
        }

        // check if there is an active GC request
        let Some(gc_request) = self.gc_request.take() else {
            return Ok(GcProgress::Idle);
        };

        self.gc_pacer
            .begin_cycle(self.options.gc, self.heap_allocated_bytes());

        // full cycles first drain young space
        if gc_request == GcRequest::Full {
            let mut remaining_bytes = budget_bytes;

            // run one bounded nursery quantum
            if !self.heap.young.is_empty() {
                let young_bytes = self.heap.young.used_bytes();
                if remaining_bytes < young_bytes {
                    self.gc_request = Some(GcRequest::Full);

                    return Ok(GcProgress::Idle);
                }

                let _minor = self.heap.collect_minor(roots)?;
                remaining_bytes -= young_bytes;
            }

            // begin / service major
            self.heap.start_major_gc(roots)?;
            if remaining_bytes == 0 {
                self.gc_request = Some(GcRequest::Full);

                Ok(GcProgress::Idle)
            } else {
                let progress = self.heap.step_major_gc(roots, remaining_bytes)?;

                if let Some(stats) = progress.completed_stats() {
                    self.on_after_gc_cycle(stats);
                } else {
                    self.gc_request = Some(GcRequest::Full);
                }

                Ok(progress)
            }
        }
        // minor cycles keep the steady-state path short
        else {
            let young_bytes = self.heap.young.used_bytes();
            if budget_bytes < young_bytes {
                self.gc_request = Some(GcRequest::Minor);
                Ok(GcProgress::Idle)
            } else {
                let stats = self.collect_minor(roots)?;
                Ok(GcProgress::Complete(stats))
            }
        }
    }

    /// Allocate one managed heap allocation.
    #[inline(always)]
    pub fn allocate(
        &mut self,
        layout: &AllocationPlan<'_>,
        allocation: Payload<'_>,
    ) -> HeapResult<HeapReference> {
        if self.heap.layout_fits_young(layout) {
            return self.heap.allocate(layout, allocation);
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

    /// Allocate one byte-initialized managed heap allocation.
    #[inline(always)]
    pub fn allocate_bytes(
        &mut self,
        layout: &AllocationPlan<'_>,
        bytes: &[u8],
    ) -> HeapResult<HeapReference> {
        if layout.is_empty() {
            return Err(HeapError::ZeroSizeAllocation);
        }

        if bytes.len() != layout.byte_len {
            return Err(HeapError::InvalidAllocationBytes {
                expected: layout.byte_len,
                actual: bytes.len(),
            });
        }

        if let Some(reference) = self.heap.try_allocate_young_bytes(layout, bytes)? {
            return Ok(reference);
        }

        let retained_byte_delta = self.heap.retained_byte_delta(layout)?;

        // check the projected heap retained-byte delta first
        self.check_retained_byte_delta(retained_byte_delta, 0)?;

        // then allocate from mature space
        let reference = self
            .heap
            .allocate_mature_layout(layout, Payload::Bytes(bytes), true)?;
        self.accrue_assist_debt(layout.byte_len);
        self.refresh_gc_request();

        Ok(reference)
    }

    /// Allocate one zeroed managed heap allocation.
    #[inline(always)]
    pub fn allocate_zeroed(&mut self, layout: &AllocationPlan<'_>) -> HeapResult<HeapReference> {
        if layout.is_empty() {
            return Err(HeapError::ZeroSizeAllocation);
        }

        // no-scan small allocations use the young run cursor directly
        if layout.is_noscan
            && let Some(small) = layout.class.small()
            && let Some(reference) = self.heap.reserve_young_run_cursor(small.slot_bytes())
        {
            return Ok(reference);
        }

        self.allocate_zeroed_refill(layout)
    }

    /// Reserve one zeroed no-scan allocation from the active young run.
    #[inline(always)]
    pub fn reserve_young(&mut self, slot_bytes: usize) -> Option<HeapReference> {
        self.heap.reserve_young_run_cursor(slot_bytes)
    }

    /// Refill zeroed allocation state or allocate from mature space.
    #[cold]
    #[inline(never)]
    fn allocate_zeroed_refill(&mut self, layout: &AllocationPlan<'_>) -> HeapResult<HeapReference> {
        // refill the young cursor or allocate through mature space
        if let Some(reference) = self.heap.try_allocate_young_zeroed(layout)? {
            return Ok(reference);
        }

        let retained_byte_delta = self.heap.retained_byte_delta(layout)?;

        // check the projected heap retained-byte delta first
        self.check_retained_byte_delta(retained_byte_delta, 0)?;

        // then allocate from mature space
        let reference = self
            .heap
            .allocate_mature_layout(layout, Payload::Zeroed, false)?;
        self.accrue_assist_debt(layout.byte_len);
        self.refresh_gc_request();

        Ok(reference)
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
    pub fn scan(&self, reference: HeapReference) -> HeapResult<TraceMap> {
        self.heap.scan(reference)
    }

    /// Record one heap write barrier over one byte range.
    pub fn write_barrier(
        &mut self,
        reference: HeapReference,
        start: usize,
        byte_len: usize,
    ) -> HeapResult<()> {
        self.heap.write_barrier(reference, start, byte_len)
    }

    /// Return old and new shared edges for one heap store before it writes.
    pub fn shared_write_barrier_bytes(
        &self,
        reference: HeapReference,
        start: usize,
        bytes: &[u8],
    ) -> HeapResult<Vec<SharedHeapReference>> {
        self.heap
            .shared_write_barrier_bytes(reference, start, bytes)
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
    ) -> HeapResult<*mut u8> {
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
            GcPressure::Cycle => self.request_gc(GcRequest::Minor),
            GcPressure::Full => self.request_gc(GcRequest::Full),
        }

        // nursery occupancy requests the cheap stop-the-world scavenge
        if self.heap.young.used_bytes() >= self.young_trigger_bytes {
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
