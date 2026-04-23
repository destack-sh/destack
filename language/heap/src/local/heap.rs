use std::sync::Arc;

use destack_mir::{LayoutId, LayoutTable, ReferenceMap};

use crate::allocator::Allocator;
use crate::local::raw::RawSpace;
use crate::local::space::HeapSpace;
use crate::{
    GcPacer, GcState, GcStats, HeapLimits, HeapOptions, HeapReference, HeapResult, Payload,
    RawPointer, SharedHeapReference,
};

/// One pending local GC request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum GcRequest {
    /// Run one minor cycle.
    Minor,
    /// Run one full cycle.
    Full,
}

/// One live local heap rooted in one shared allocator.
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
    /// Create one heap over one explicit allocator, layout table, limits, and options.
    pub fn with_allocator_limits_layouts_and_options(
        allocator: Arc<Allocator>,
        layouts: Arc<LayoutTable>,
        limits: HeapLimits,
        options: HeapOptions,
    ) -> HeapResult<Self> {
        options.validate_local()?;
        options.validate_allocator(&allocator)?;

        Self::build_with_options(allocator, layouts, limits, options)
    }

    /// Create one heap from one checked shared allocator, limits, and options.
    fn build_with_options(
        allocator: Arc<Allocator>,
        layouts: Arc<LayoutTable>,
        limits: HeapLimits,
        options: HeapOptions,
    ) -> HeapResult<Self> {
        let mut heap = Self {
            heap: HeapSpace::build_with_options(allocator.clone(), layouts, &options)?,
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
        self.check_mapped_byte_delta(0, 0)
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
        work_items: usize,
    ) -> HeapResult<usize> {
        self.heap.scan_shared_edge_step(roots, work_items)
    }

    /// Stabilize one local heap reference in mature storage.
    pub fn stabilize_heap(&mut self, reference: HeapReference) -> HeapResult<HeapReference> {
        self.heap.stabilize(reference)
    }

    /// Pin one local heap reference against movement.
    pub fn pin_heap(&mut self, reference: HeapReference) -> HeapResult<HeapReference> {
        self.heap.pin(reference)
    }

    /// Release one local heap pin.
    pub fn unpin_heap(&mut self, reference: HeapReference) -> HeapResult<()> {
        self.heap.unpin(reference)
    }

    /// Return the current derived collector pacing targets.
    pub fn gc_pacer(&self) -> GcPacer {
        let mut gc_pacer = self.gc_pacer;
        gc_pacer.update(self.options.gc, self.heap_allocated_bytes());

        gc_pacer
    }

    /// Perform one minor heap collection over explicit roots.
    pub fn collect_minor(&mut self, roots: &mut [HeapReference]) -> HeapResult<GcStats> {
        let stats = self.heap.collect_minor(roots)?;
        self.on_after_gc_cycle(stats);

        Ok(stats)
    }

    /// Perform one full heap collection over explicit roots.
    pub fn collect_full(&mut self, roots: &mut [HeapReference]) -> HeapResult<GcStats> {
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
    pub fn gc_step(&mut self, roots: &mut [HeapReference]) -> HeapResult<Option<GcStats>> {
        let Some(gc_request) = self.gc_request.take() else {
            return Ok(None);
        };

        // full cycles compact mature space and clear all young debt
        if gc_request == GcRequest::Full {
            let stats = self.collect_full(roots)?;

            return Ok(Some(stats));
        }

        // minor cycles keep the steady-state path short
        let stats = self.collect_minor(roots)?;

        Ok(Some(stats))
    }

    /// Allocate one managed heap entry.
    pub fn allocate(
        &mut self,
        layout_id: LayoutId,
        allocation: Payload<'_>,
    ) -> HeapResult<HeapReference> {
        let mapped_byte_delta = self.heap.mapped_byte_delta(layout_id)?;

        // check the projected heap mapped-byte delta first
        self.check_mapped_byte_delta(mapped_byte_delta, 0)?;

        // then allocate through heap space
        let reference = self.heap.allocate(layout_id, allocation)?;
        self.refresh_gc_request();

        Ok(reference)
    }

    /// Allocate one managed slice backing entry.
    pub fn allocate_slice(
        &mut self,
        element_layout_id: LayoutId,
        length: usize,
        allocation: Payload<'_>,
    ) -> HeapResult<HeapReference> {
        let mapped_byte_delta = self
            .heap
            .slice_mapped_byte_delta(element_layout_id, length)?;

        // check the projected heap mapped-byte delta first
        self.check_mapped_byte_delta(mapped_byte_delta, 0)?;

        // then allocate through heap space
        let reference = self
            .heap
            .allocate_slice(element_layout_id, length, allocation)?;
        self.refresh_gc_request();

        Ok(reference)
    }

    /// Allocate one raw entry.
    pub fn allocate_raw(
        &mut self,
        byte_len: usize,
        allocation: Payload<'_>,
    ) -> HeapResult<RawPointer> {
        let mapped_byte_delta = self.raw.alloc_mapped_byte_delta(byte_len);

        // check the projected raw mapped-byte delta first
        self.check_mapped_byte_delta(0, mapped_byte_delta)?;

        // then allocate through raw space
        self.raw.allocate(byte_len, allocation)
    }

    /// Register one managed layout and return its stable id.
    pub fn register_layout(&mut self, layout: destack_mir::Layout) -> LayoutId {
        self.heap.register_layout(layout)
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
        let mapped_byte_delta = self
            .heap
            .write_mapped_byte_delta(reference, start, bytes.len())?;

        self.check_mapped_byte_delta(mapped_byte_delta, 0)?;

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
        // check the projected replacement mapped-byte delta next
        self.check_raw_replace_mapped_byte_delta(pointer, bytes.len())?;

        // then replace the raw payload
        self.raw.replace_bytes(pointer, bytes)
    }

    /// Overwrite one raw byte range.
    pub fn set_raw_bytes(
        &mut self,
        pointer: RawPointer,
        start: usize,
        bytes: &[u8],
    ) -> HeapResult<()> {
        let mapped_byte_delta = self
            .raw
            .write_mapped_byte_delta(pointer, start, bytes.len())?;
        self.check_mapped_byte_delta(0, mapped_byte_delta)?;
        self.raw.set_bytes(pointer, start, bytes)
    }

    /// Overwrite one raw byte.
    pub fn set_raw_byte(&mut self, pointer: RawPointer, index: usize, byte: u8) -> HeapResult<()> {
        self.set_raw_bytes(pointer, index, &[byte])
    }

    /// Free one raw allocation.
    pub fn free_raw(&mut self, pointer: RawPointer) -> HeapResult<bool> {
        self.raw.free(pointer)
    }

    /// Check the projected mapped-byte delta for one raw replacement.
    fn check_raw_replace_mapped_byte_delta(
        &self,
        pointer: RawPointer,
        next_len: usize,
    ) -> HeapResult<()> {
        let mapped_byte_delta = self.raw.replace_mapped_byte_delta(pointer, next_len)?;

        self.check_mapped_byte_delta(0, mapped_byte_delta)
    }

    /// Check heap limits after one requested mapped-byte delta.
    fn check_mapped_byte_delta(
        &self,
        heap_mapped_byte_delta: i64,
        raw_mapped_byte_delta: i64,
    ) -> HeapResult<()> {
        self.limits.check_mapped_byte_delta(
            self.heap.active_bytes(),
            self.raw.active_bytes(),
            heap_mapped_byte_delta,
            raw_mapped_byte_delta,
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
        self.refresh_gc_request();
    }
}
