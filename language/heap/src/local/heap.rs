use std::sync::Arc;

use destack_mir::ReferenceMap;

use crate::allocator::Allocator;
use crate::local::raw::RawSpace;
use crate::local::space::HeapSpace;
use crate::{
    AllocationLayout, GcPacer, GcProgress, GcState, GcStats, HeapLimits, HeapOptions,
    HeapReference, HeapResult, Payload, RawPointer, RootSlots, SharedHeapReference,
};

/// One pending local GC request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum GcRequest {
    /// Run one minor cycle.
    Minor,
    /// Run one full cycle.
    Full,
}

/// One live heap rooted in one shared allocator.
#[derive(Debug)]
pub struct Heap {
    /// The shared page allocator for every local byte payload.
    pub(super) allocator: Arc<Allocator>,
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
            allocator,
            options,
            gc_pacer: GcPacer::default(),
            gc_request: None,
            limits,
        };

        heap.refresh_gc_request();

        Ok(heap)
    }

    /// Return the shared page allocator.
    pub(crate) fn allocator(&self) -> &Arc<Allocator> {
        &self.allocator
    }

    /// Return the heap options.
    pub(crate) fn options(&self) -> &HeapOptions {
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
    pub fn scan_shared_edge_step(
        &mut self,
        roots: &mut Vec<SharedHeapReference>,
        step_budget: usize,
    ) -> HeapResult<usize> {
        self.heap.scan_shared_edge_step(roots, step_budget)
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

    /// Return the current derived collector pacing targets.
    pub fn gc_pacer(&self) -> GcPacer {
        let mut gc_pacer = self.gc_pacer;
        gc_pacer.update(self.options.gc, self.heap_allocated_bytes());

        gc_pacer
    }

    /// Perform one minor heap collection over mutable roots.
    pub fn collect_minor<R>(&mut self, roots: &mut R) -> Result<GcStats, R::Error>
    where
        R: RootSlots,
    {
        let stats = self.heap.collect_minor(roots)?;
        self.on_after_gc_cycle(stats);

        Ok(stats)
    }

    /// Perform one full heap collection over mutable roots.
    pub fn collect_full<R>(&mut self, roots: &mut R) -> Result<GcStats, R::Error>
    where
        R: RootSlots,
    {
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

    /// Run one pacing-driven local collection step at one safepoint.
    pub fn gc_step<R>(&mut self, roots: &mut R) -> Result<GcProgress, R::Error>
    where
        R: RootSlots,
    {
        // service major GC work
        if self.heap.major_gc_active() {
            let step_budget = self.take_major_gc_step_budget();
            let progress = self.heap.step_major_gc(roots, step_budget)?;
            if let Some(stats) = progress.completed_stats() {
                self.on_after_gc_cycle(stats);
            }

            return Ok(progress);
        }

        // check if there is an active GC request
        let Some(gc_request) = self.gc_request.take() else {
            return Ok(GcProgress::Idle);
        };

        // full cycles first clear young debt, then continue as incremental major work
        if gc_request == GcRequest::Full {
            // service minor first
            let _minor = self.heap.collect_minor(roots)?;

            // begin / service major
            self.heap.start_major_gc(roots)?;
            let step_budget = self.take_major_gc_step_budget();
            let progress = self.heap.step_major_gc(roots, step_budget)?;

            if let Some(stats) = progress.completed_stats() {
                self.on_after_gc_cycle(stats);
            } else {
                self.gc_request = Some(GcRequest::Full);
            }

            Ok(progress)
        }
        // minor cycles keep the steady-state path short
        else {
            let stats = self.collect_minor(roots)?;
            Ok(GcProgress::Complete(stats))
        }
    }

    /// Return the local major collection work budget for one safepoint.
    fn take_major_gc_step_budget(&mut self) -> usize {
        // scale with current heap size and worst-case span density
        let page_bytes = self.allocator.page_bytes().max(1);
        let heap_pages = (self.heap_allocated_bytes() as usize).div_ceil(page_bytes);
        let span_pages = self.options.small_span_pages();
        let heap_spans = heap_pages.div_ceil(span_pages).max(1);
        let span_slots = self.options.minimum_small_span_slots();
        let base_budget = heap_spans.max(span_slots);
        let assist_steps = self
            .gc_pacer
            .take_assist_work(base_budget * page_bytes, page_bytes);

        base_budget + assist_steps
    }

    /// Allocate one managed heap allocation.
    pub fn allocate(
        &mut self,
        layout: AllocationLayout<'_>,
        allocation: Payload<'_>,
    ) -> HeapResult<HeapReference> {
        let retained_byte_delta = self.heap.retained_byte_delta(layout)?;

        // check the projected heap retained-byte delta first
        self.check_retained_byte_delta(retained_byte_delta, 0)?;

        // then allocate through heap space
        let reference = self.heap.allocate(layout, allocation)?;
        self.accrue_assist_debt(layout.byte_len);
        self.refresh_gc_request();

        Ok(reference)
    }

    /// Allocate one raw allocation.
    pub fn allocate_raw(
        &mut self,
        byte_len: usize,
        allocation: Payload<'_>,
    ) -> HeapResult<RawPointer> {
        let retained_byte_delta = self.raw.alloc_retained_byte_delta(byte_len);

        // check the projected raw retained-byte delta first
        self.check_retained_byte_delta(0, retained_byte_delta)?;

        // then allocate through raw space
        self.raw.allocate(byte_len, allocation)
    }

    /// Return whether one heap reference currently refers to one live allocation.
    pub fn is_heap_live(&self, reference: HeapReference) -> bool {
        self.heap.is_live(reference)
    }

    /// Return the bytes for one heap allocation as one owned vector.
    pub fn read_heap_bytes(&self, reference: HeapReference) -> HeapResult<Vec<u8>> {
        self.heap.read_bytes(reference)
    }

    /// Return the remaining byte length for one heap allocation.
    pub fn heap_byte_len(&self, reference: HeapReference) -> HeapResult<usize> {
        self.heap.byte_len(reference)
    }

    /// Fill one caller-provided buffer from one heap allocation at one offset.
    pub fn read_heap_bytes_into(
        &self,
        reference: HeapReference,
        start: usize,
        target: &mut [u8],
    ) -> HeapResult<()> {
        self.heap.read_bytes_into(reference, start, target)
    }

    /// Return one checked address for a managed heap byte range.
    pub fn heap_address(
        &self,
        reference: HeapReference,
        start: usize,
        byte_len: usize,
    ) -> HeapResult<*mut u8> {
        self.heap.address(reference, start, byte_len)
    }

    /// Return one checked writable address for a managed heap byte range.
    pub fn heap_address_mut(
        &mut self,
        reference: HeapReference,
        start: usize,
        byte_len: usize,
    ) -> HeapResult<*mut u8> {
        self.heap.address_mut(reference, start, byte_len)
    }

    /// Return the heap scan metadata for one heap allocation.
    pub fn scan(&self, reference: HeapReference) -> HeapResult<ReferenceMap> {
        self.heap.scan(reference)
    }

    /// Overwrite one heap byte range.
    pub fn write_heap_bytes(
        &mut self,
        reference: HeapReference,
        start: usize,
        bytes: &[u8],
    ) -> HeapResult<()> {
        self.heap.write_bytes(reference, start, bytes)
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

    /// Return one raw byte by offset.
    pub fn raw_byte_at(&self, pointer: RawPointer, index: usize) -> Option<u8> {
        self.raw.byte_at(pointer, index)
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
    pub fn free_raw(&mut self, pointer: RawPointer) -> HeapResult<bool> {
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

    /// Refresh the collector pacing targets from current heap state.
    fn refresh_gc_pacer(&mut self) {
        self.gc_pacer
            .update(self.options.gc, self.heap_allocated_bytes());
    }

    /// Refresh the pending collection request from current heap pressure.
    pub(crate) fn refresh_gc_request(&mut self) {
        self.refresh_gc_pacer();
        let heap_bytes = self.heap_allocated_bytes();

        // goal crossings force one full cycle
        if self.gc_pacer.should_collect_full(heap_bytes) {
            self.request_gc(GcRequest::Full);

            return;
        }

        // trigger crossings request one young cycle
        if self.gc_pacer.should_start(heap_bytes) {
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

    /// Record one completed local collection cycle in the pacer.
    fn on_after_gc_cycle(&mut self, stats: GcStats) {
        self.gc_pacer.update(self.options.gc, stats.allocated_bytes);
        self.gc_request = None;
        self.gc_pacer.clear_assist_debt();
        self.refresh_gc_request();
    }

    /// Accrue local collector work from heap allocation pressure.
    fn accrue_assist_debt(&mut self, allocated_bytes: usize) {
        self.refresh_gc_pacer();
        let heap_bytes = self.heap_allocated_bytes();
        if heap_bytes < self.gc_pacer.trigger_bytes && !self.heap.major_gc_active() {
            return;
        }

        self.gc_pacer.add_assist_debt(allocated_bytes);
    }
}
