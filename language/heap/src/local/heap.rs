use std::sync::Arc;

use crate::arena::Arena;
use crate::local::managed::ManagedSpace;
use crate::local::raw::RawSpace;
use crate::{
    GcPacer, GcState, GcStats, HeapLimits, HeapOptions, HeapResult, HeapScan, LayoutId,
    ManagedReference, RawPointer, SharedManagedReference,
};

/// One pending local GC request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum GcRequest {
    /// Run one minor cycle.
    Minor,
    /// Run one full cycle.
    Full,
}

/// One live local heap rooted in one shared arena.
#[derive(Debug)]
pub struct Heap {
    /// The shared page arena for every local byte payload.
    pub(super) arena: Arc<Arena>,
    /// The configured heap options.
    pub(super) options: HeapOptions,
    /// The derived collector pacing targets.
    pub(super) gc_pacer: GcPacer,
    /// The pending pacing or explicit collection request.
    pub(super) gc_request: Option<GcRequest>,
    /// The managed local allocation space.
    pub(super) managed: ManagedSpace,
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
        let arena = Arc::new(Arena::try_new(
            options.page_bytes,
            options.arena_segment_bytes,
        )?);

        options.validate()?;
        options.validate_arena(&arena)?;

        Self::build_with_options(arena, limits, options)
    }

    /// Create one heap from one checked shared arena, limits, and options.
    fn build_with_options(
        arena: Arc<Arena>,
        limits: HeapLimits,
        options: HeapOptions,
    ) -> HeapResult<Self> {
        let mut heap = Self {
            managed: ManagedSpace::build_with_options(arena.clone(), &options)?,
            raw: RawSpace::with_options(arena.clone(), &options)?,
            arena,
            options,
            gc_pacer: GcPacer::default(),
            gc_request: None,
            limits,
        };

        heap.refresh_gc_request();

        Ok(heap)
    }

    /// Return the shared page arena.
    pub(crate) fn arena(&self) -> &Arc<Arena> {
        &self.arena
    }

    /// Return the heap options.
    pub(crate) fn options(&self) -> &HeapOptions {
        &self.options
    }

    /// Return the encoded managed-reference byte width for this heap.
    pub fn managed_reference_bytes(&self) -> u8 {
        self.options.managed_reference_bytes
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

    /// Return the currently live managed references.
    pub fn live_references(&self) -> HeapResult<Vec<ManagedReference>> {
        self.managed.live_references()
    }

    /// Return the current collector state.
    pub fn gc_state(&self) -> &GcState {
        self.managed.gc_state()
    }

    /// Start one incremental local-to-shared root scan.
    pub fn start_shared_root_scan(&mut self) {
        self.managed.start_shared_root_scan();
    }

    /// Return whether the current local-to-shared root scan is drained.
    pub fn shared_root_scan_idle(&self) -> bool {
        self.managed.shared_root_scan_idle()
    }

    /// Finish the current local-to-shared root scan.
    pub fn finish_shared_root_scan(&mut self) {
        self.managed.finish_shared_root_scan();
    }

    /// Scan bounded local-to-shared root work into the provided root buffer.
    pub fn scan_shared_root_step(
        &mut self,
        roots: &mut Vec<SharedManagedReference>,
        work_items: usize,
    ) -> HeapResult<usize> {
        self.managed.scan_shared_root_step(roots, work_items)
    }

    /// Pin one local managed reference against movement.
    pub fn pin_managed(&mut self, reference: ManagedReference) -> HeapResult<()> {
        self.managed.pin(reference)
    }

    /// Release one local managed pin.
    pub fn unpin_managed(&mut self, reference: ManagedReference) -> HeapResult<()> {
        self.managed.unpin(reference)
    }

    /// Return the current derived collector pacing targets.
    pub fn gc_pacer(&self) -> GcPacer {
        let mut gc_pacer = self.gc_pacer;
        gc_pacer.update(self.options.gc, self.managed_allocated_bytes());

        gc_pacer
    }

    /// Perform one minor managed collection over explicit roots.
    pub fn collect_minor(
        &mut self,
        roots: impl IntoIterator<Item = ManagedReference>,
    ) -> HeapResult<GcStats> {
        let stats = self.managed.collect_minor(roots)?;
        self.note_gc_cycle(stats);

        Ok(stats)
    }

    /// Perform one full managed collection over explicit roots.
    pub fn collect_full(
        &mut self,
        roots: impl IntoIterator<Item = ManagedReference>,
    ) -> HeapResult<GcStats> {
        let stats = self.managed.collect_full(roots)?;
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
    pub fn gc_step(
        &mut self,
        roots: impl IntoIterator<Item = ManagedReference>,
    ) -> HeapResult<Option<GcStats>> {
        let Some(gc_request) = self.gc_request.take() else {
            return Ok(None);
        };

        let roots = roots.into_iter().collect::<Vec<_>>();

        // full cycles compact mature space and clear all young debt
        if gc_request == GcRequest::Full {
            let stats = self.collect_full(roots.iter().copied())?;

            return Ok(Some(stats));
        }

        // minor cycles keep the steady-state path short
        let stats = self.collect_minor(roots.iter().copied())?;

        Ok(Some(stats))
    }

    /// Allocate one managed byte allocation.
    pub fn allocate_managed_bytes(
        &mut self,
        bytes: &[u8],
        scan: impl Into<HeapScan>,
        layout_id: Option<LayoutId>,
    ) -> HeapResult<ManagedReference> {
        let scan = scan.into();
        let path = self.managed.allocation_path(bytes.len());

        // check the projected managed mapped-byte delta first
        self.check_mapped_delta(path.mapped_delta(), 0)?;

        // then allocate through managed space
        let reference = self.managed.place_bytes(bytes, scan, layout_id, path)?;
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

    /// Allocate one zeroed managed byte allocation.
    pub fn allocate_managed_zeroed(
        &mut self,
        byte_len: usize,
        scan: impl Into<HeapScan>,
        layout_id: Option<LayoutId>,
    ) -> HeapResult<ManagedReference> {
        let scan = scan.into();
        let path = self.managed.allocation_path(byte_len);

        // check the projected managed mapped-byte delta first
        self.check_mapped_delta(path.mapped_delta(), 0)?;

        // then allocate through managed space
        let reference = self.managed.place_zeroed(byte_len, scan, layout_id, path)?;
        self.refresh_gc_request();

        Ok(reference)
    }

    /// Return whether one managed reference currently refers to one live allocation.
    pub fn is_managed_live(&self, reference: ManagedReference) -> bool {
        self.managed.is_live(reference)
    }

    /// Return the bytes for one managed allocation as one owned vector.
    pub fn read_managed_bytes(&self, reference: ManagedReference) -> HeapResult<Vec<u8>> {
        self.managed.read_bytes(reference)
    }

    /// Return the remaining byte length for one managed allocation.
    pub fn managed_byte_len(&self, reference: ManagedReference) -> HeapResult<usize> {
        self.managed.byte_len(reference)
    }

    /// Fill one caller-provided buffer from one managed allocation at one offset.
    pub fn read_managed_bytes_into(
        &self,
        reference: ManagedReference,
        start: usize,
        target: &mut [u8],
    ) -> HeapResult<()> {
        self.managed.read_bytes_into(reference, start, target)
    }

    /// Return the storage layout id for one managed allocation.
    pub fn managed_layout_id(&self, reference: ManagedReference) -> HeapResult<Option<LayoutId>> {
        self.managed.layout_id(reference)
    }

    /// Return the managed scan metadata for one managed allocation.
    pub fn scan(&self, reference: ManagedReference) -> HeapResult<&HeapScan> {
        self.managed.scan(reference)
    }

    /// Overwrite one managed byte range.
    pub fn write_managed_bytes(
        &mut self,
        reference: ManagedReference,
        start: usize,
        bytes: &[u8],
    ) -> HeapResult<()> {
        let mapped_delta = self
            .managed
            .write_mapped_delta(reference, start, bytes.len())?;

        self.check_mapped_delta(mapped_delta, 0)?;

        self.managed.write_bytes(reference, start, bytes)
    }

    /// Record one managed write barrier over one byte range.
    pub fn write_barrier(
        &mut self,
        reference: ManagedReference,
        start: usize,
        byte_len: usize,
    ) -> HeapResult<()> {
        self.managed.write_barrier(reference, start, byte_len)
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
    pub fn replace_raw_bytes(&mut self, pointer: RawPointer, bytes: &[u8]) -> HeapResult<()> {
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
    fn check_mapped_delta(
        &self,
        managed_mapped_delta: i64,
        raw_mapped_delta: i64,
    ) -> HeapResult<()> {
        self.limits.check_mapped_delta(
            self.managed.active_bytes(),
            self.raw.active_bytes(),
            managed_mapped_delta,
            raw_mapped_delta,
        )
    }

    /// Refresh the collector pacing targets from current heap state.
    fn refresh_gc_pacer(&mut self) {
        self.gc_pacer
            .update(self.options.gc, self.managed_allocated_bytes());
    }

    /// Refresh the pending collection request from current managed pressure.
    pub(crate) fn refresh_gc_request(&mut self) {
        self.refresh_gc_pacer();
        let heap_bytes = self.managed_allocated_bytes();

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
