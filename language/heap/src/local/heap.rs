use std::sync::Arc;

use crate::allocator::Allocator;
use crate::local::raw::RawSpace;
use crate::local::space::HeapSpace;
use crate::{
    GcPacer, GcStats, GcSummary, HeapLimits, HeapOptions, HeapReference, HeapResult, LayoutId,
    RawPointer, SharedHeapReference, TracePlan,
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
    /// Create one heap with the default limits and options.
    pub fn new() -> HeapResult<Self> {
        Self::with_limits_and_options(HeapLimits::default(), HeapOptions::local())
    }

    /// Create one heap with explicit limits and options.
    pub fn with_limits_and_options(limits: HeapLimits, options: HeapOptions) -> HeapResult<Self> {
        let allocator = Arc::new(Allocator::try_new(
            options.page_bytes,
            options.allocator_segment_bytes,
        )?);

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
        self.check_mapped_delta(0, 0)
    }

    /// Return the currently live heap references.
    pub fn live_references(&self) -> HeapResult<Vec<HeapReference>> {
        self.heap.live_references()
    }

    /// Return the current collector state.
    pub fn gc_state(&self) -> &GcSummary {
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
        self.note_gc_cycle(stats);

        Ok(stats)
    }

    /// Perform one full heap collection over explicit roots.
    pub fn collect_full(&mut self, roots: &mut [HeapReference]) -> HeapResult<GcStats> {
        let stats = self.heap.collect_full(roots)?;
        self.note_gc_cycle(stats);

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

    /// Allocate one heap byte allocation.
    pub fn allocate_heap_bytes(
        &mut self,
        bytes: &[u8],
        scan: impl Into<TracePlan>,
        layout_id: Option<LayoutId>,
    ) -> HeapResult<HeapReference> {
        let scan = scan.into();
        let path = self.heap.allocation_path(bytes.len());

        // check the projected heap mapped-byte delta first
        self.check_mapped_delta(path.mapped_delta(), 0)?;

        // then allocate through heap space
        let reference = self.heap.place_bytes(bytes, scan, layout_id, path)?;
        self.refresh_gc_request();

        Ok(reference)
    }

    /// Allocate one raw byte allocation.
    pub fn allocate_raw_bytes(&mut self, bytes: &[u8]) -> HeapResult<RawPointer> {
        let path = self.raw.allocation_path(bytes.len());

        // check the projected raw mapped-byte delta first
        self.check_mapped_delta(0, path.mapped_delta())?;

        // then allocate through raw space
        self.raw.place_bytes(bytes, path)
    }

    /// Allocate one zeroed heap byte allocation.
    pub fn allocate_heap_zeroed(
        &mut self,
        byte_len: usize,
        scan: impl Into<TracePlan>,
        layout_id: Option<LayoutId>,
    ) -> HeapResult<HeapReference> {
        let scan = scan.into();
        let path = self.heap.allocation_path(byte_len);

        // check the projected heap mapped-byte delta first
        self.check_mapped_delta(path.mapped_delta(), 0)?;

        // then allocate through heap space
        let reference = self.heap.place_zeroed(byte_len, scan, layout_id, path)?;
        self.refresh_gc_request();

        Ok(reference)
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

    /// Return the storage layout id for one heap allocation.
    pub fn heap_layout_id(&self, reference: HeapReference) -> HeapResult<Option<LayoutId>> {
        self.heap.layout_id(reference)
    }

    /// Return the heap scan metadata for one heap allocation.
    pub fn scan(&self, reference: HeapReference) -> HeapResult<&TracePlan> {
        self.heap.scan(reference)
    }

    /// Overwrite one heap byte range.
    pub fn write_heap_bytes(
        &mut self,
        reference: HeapReference,
        start: usize,
        bytes: &[u8],
    ) -> HeapResult<()> {
        let mapped_delta = self
            .heap
            .write_mapped_delta(reference, start, bytes.len())?;

        self.check_mapped_delta(mapped_delta, 0)?;

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

    /// Allocate one zeroed raw byte allocation.
    pub fn allocate_raw_zeroed(&mut self, byte_len: usize) -> HeapResult<RawPointer> {
        let path = self.raw.allocation_path(byte_len);

        // check the projected raw mapped-byte delta first
        self.check_mapped_delta(0, path.mapped_delta())?;

        // then allocate through raw space
        self.raw.place_zeroed(byte_len, path)
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
        self.check_raw_replace_mapped_delta(pointer, bytes.len())?;

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
        let mapped_delta = self.raw.write_mapped_delta(pointer, start, bytes.len())?;
        self.check_mapped_delta(0, mapped_delta)?;
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
    fn check_raw_replace_mapped_delta(
        &self,
        pointer: RawPointer,
        next_len: usize,
    ) -> HeapResult<()> {
        let mapped_delta = self.raw.replace_mapped_delta(pointer, next_len)?;

        self.check_mapped_delta(0, mapped_delta)
    }

    /// Check heap limits after one requested mapped-byte delta.
    fn check_mapped_delta(&self, heap_mapped_delta: i64, raw_mapped_delta: i64) -> HeapResult<()> {
        self.limits.check_mapped_delta(
            self.heap.active_bytes(),
            self.raw.active_bytes(),
            heap_mapped_delta,
            raw_mapped_delta,
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
    fn note_gc_cycle(&mut self, stats: GcStats) {
        self.gc_pacer.update(self.options.gc, stats.allocated_bytes);
        self.gc_request = None;
        self.refresh_gc_request();
    }
}
