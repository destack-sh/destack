use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::{
    SharedGcPhase, SharedHeapLimits, SharedHeapUsage, SharedManagedSpace, SharedManagedSpaceImage,
    SharedRawSpace, SharedRawSpaceImage,
};
use crate::{
    Arena, GcPacer, GcState, GcStats, HeapError, HeapOptions, HeapResult, HeapScan, HeapSpace,
    LayoutId, PageId, SharedManagedReference, SharedRawPointer, apply_byte_delta, sum_bytes,
};

/// One live world-shared heap.
#[derive(Debug)]
pub struct SharedHeap {
    /// The shared arena for both shared heap spaces.
    pub(crate) arena: Arc<Arena>,
    /// The configured shared heap options.
    pub(crate) options: HeapOptions,
    /// The derived collector pacing targets.
    pub(crate) gc_pacer: GcPacer,
    /// Whether one shared cycle has been requested by pressure or explicitly.
    gc_requested: bool,
    /// The traced shared managed space.
    pub(crate) managed: SharedManagedSpace,
    /// The explicit shared raw space.
    pub(crate) raw: SharedRawSpace,
    /// The exact hard limits for this shared heap.
    pub(crate) limits: SharedHeapLimits,
}

/// One frozen shared heap.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SharedHeapImage {
    /// The captured shared heap options.
    pub options: HeapOptions,
    /// The frozen shared managed space.
    pub managed: SharedManagedSpaceImage,
    /// The frozen shared raw space.
    pub raw: SharedRawSpaceImage,
}

impl SharedHeapImage {
    /// Return every arena page reachable from this shared-heap image.
    pub fn page_ids(&self) -> Vec<PageId> {
        let mut pages = self.managed.page_ids();
        pages.extend(self.raw.page_ids());

        pages
    }
}

impl Default for SharedHeap {
    fn default() -> Self {
        Self::new()
    }
}

impl SharedHeap {
    /// Create a new empty shared heap.
    pub fn new() -> Self {
        Self::with_limits_and_options(SharedHeapLimits::default(), HeapOptions::shared())
    }

    /// Create a new empty shared heap with explicit options.
    pub fn with_options(options: HeapOptions) -> Self {
        Self::with_arena_limits_and_options(
            Arc::new(Arena::new()),
            SharedHeapLimits::default(),
            options,
        )
    }

    /// Create a new empty shared heap over one shared arena.
    pub fn with_arena(arena: Arc<Arena>) -> Self {
        Self::with_arena_and_options(arena, HeapOptions::shared())
    }

    /// Create a new empty shared heap with explicit limits and options.
    pub fn with_limits_and_options(limits: SharedHeapLimits, options: HeapOptions) -> Self {
        Self::with_arena_limits_and_options(Arc::new(Arena::new()), limits, options)
    }

    /// Create a new empty shared heap over one shared arena and limit set.
    pub fn with_arena_and_limits(arena: Arc<Arena>, limits: SharedHeapLimits) -> Self {
        Self::with_arena_limits_and_options(arena, limits, HeapOptions::shared())
    }

    /// Create a new empty shared heap over one shared arena and explicit options.
    pub fn with_arena_and_options(arena: Arc<Arena>, options: HeapOptions) -> Self {
        Self::with_arena_limits_and_options(arena, SharedHeapLimits::default(), options)
    }

    /// Create a new empty shared heap over one shared arena, limits, and options.
    pub fn with_arena_limits_and_options(
        arena: Arc<Arena>,
        limits: SharedHeapLimits,
        options: HeapOptions,
    ) -> Self {
        let mut shared = Self {
            managed: SharedManagedSpace::with_options(arena.clone(), &options),
            raw: SharedRawSpace::with_arena(arena.clone()),
            arena,
            options,
            gc_pacer: GcPacer::default(),
            gc_requested: false,
            limits,
        };

        shared.refresh_gc_request();

        shared
    }

    /// Return the configured shared page width.
    pub fn page_bytes(&self) -> usize {
        self.arena.page_bytes()
    }

    /// Return the exact live usage for this shared heap.
    pub fn usage(&self) -> HeapResult<SharedHeapUsage> {
        Ok(SharedHeapUsage {
            managed: self.managed.usage()?,
            raw: self.raw.usage()?,
        })
    }

    /// Return the exact active shared heap bytes.
    pub fn active_bytes(&self) -> u64 {
        self.managed.active_bytes() + self.raw.active_bytes()
    }

    /// Return the current shared managed collector state.
    pub fn gc_state(&self) -> &GcState {
        self.managed.gc_state()
    }

    /// Return the current derived collector pacing targets.
    pub fn gc_pacer(&self) -> GcPacer {
        let mut gc_pacer = self.gc_pacer;
        gc_pacer.update(self.options.gc, self.managed_allocated_bytes());

        gc_pacer
    }

    /// Return the current shared managed collector phase.
    pub fn gc_phase(&self) -> SharedGcPhase {
        self.managed.gc_phase()
    }

    /// Return whether shared pacing wants to start a new cycle.
    pub fn should_start_cycle(&self) -> bool {
        self.gc_phase() == SharedGcPhase::Idle && self.gc_requested
    }

    /// Return the exact mapped shared heap bytes.
    pub fn mapped_bytes(&self) -> u64 {
        self.managed.mapped_bytes() + self.raw.mapped_bytes()
    }

    /// Return the exact borrowed shared heap bytes.
    pub fn borrowed_bytes(&self) -> HeapResult<u64> {
        sum_bytes(self.managed.borrowed_bytes()?, self.raw.borrowed_bytes()?)
    }

    /// Return the exact active shared raw-space bytes.
    pub fn raw_active_bytes(&self) -> u64 {
        self.raw.active_bytes()
    }

    /// Return the projected mapped-byte delta for one shared raw allocation.
    pub fn raw_alloc_mapped_delta(&self, byte_len: usize) -> i64 {
        self.raw.alloc_mapped_delta(byte_len)
    }

    /// Return the projected mapped-byte delta for one shared raw replacement.
    pub fn raw_replace_mapped_delta(
        &self,
        pointer: SharedRawPointer,
        next_byte_len: usize,
    ) -> HeapResult<i64> {
        self.raw.replace_mapped_delta(pointer, next_byte_len)
    }

    /// Allocate one shared raw byte allocation.
    pub fn allocate_raw_bytes(&mut self, bytes: &[u8]) -> HeapResult<SharedRawPointer> {
        self.check_raw_mapped_delta(self.raw.alloc_mapped_delta(bytes.len()))?;

        self.raw.allocate_bytes(bytes)
    }

    /// Replace one shared raw allocation payload.
    pub fn replace_raw_bytes(&mut self, pointer: SharedRawPointer, bytes: &[u8]) -> HeapResult<()> {
        self.check_raw_mapped_delta(self.raw.replace_mapped_delta(pointer, bytes.len())?)?;

        self.raw.replace_bytes(pointer, bytes)
    }

    /// Return the bytes for one shared raw allocation.
    pub fn read_raw_bytes(&self, pointer: SharedRawPointer) -> HeapResult<Vec<u8>> {
        self.raw.read_bytes(pointer)
    }

    /// Allocate one shared managed byte allocation.
    pub fn allocate_managed_bytes(
        &mut self,
        bytes: &[u8],
        scan: impl Into<HeapScan>,
        layout_id: Option<LayoutId>,
    ) -> HeapResult<SharedManagedReference> {
        let path = self.managed.allocation_path(bytes.len());
        self.check_managed_mapped_delta(path.mapped_delta())?;

        let reference = self.managed.place_bytes(bytes, scan, layout_id, path)?;
        self.refresh_gc_request();

        Ok(reference)
    }

    /// Allocate one zeroed shared managed byte allocation.
    pub fn allocate_managed_zeroed(
        &mut self,
        byte_len: usize,
        scan: impl Into<HeapScan>,
        layout_id: Option<LayoutId>,
    ) -> HeapResult<SharedManagedReference> {
        let path = self.managed.allocation_path(byte_len);
        self.check_managed_mapped_delta(path.mapped_delta())?;

        let reference = self.managed.place_zeroed(byte_len, scan, layout_id, path)?;
        self.refresh_gc_request();

        Ok(reference)
    }

    /// Return whether one shared managed reference currently refers to one live entry.
    pub fn is_managed_live(&self, reference: SharedManagedReference) -> bool {
        self.managed.is_live(reference)
    }

    /// Return the remaining byte length for one shared managed reference.
    pub fn managed_byte_len(&self, reference: SharedManagedReference) -> HeapResult<usize> {
        self.managed.byte_len(reference)
    }

    /// Return the bytes for one shared managed reference.
    pub fn read_managed_bytes(&self, reference: SharedManagedReference) -> HeapResult<Vec<u8>> {
        self.managed.read_bytes(reference)
    }

    /// Fill one caller-provided buffer from one shared managed entry at one offset.
    pub fn read_managed_bytes_into(
        &self,
        reference: SharedManagedReference,
        start: usize,
        target: &mut [u8],
    ) -> HeapResult<()> {
        self.managed.read_bytes_into(reference, start, target)
    }

    /// Return the scan metadata for one shared managed reference.
    pub fn scan(&self, reference: SharedManagedReference) -> HeapResult<&HeapScan> {
        self.managed.scan(reference)
    }

    /// Return the storage layout id for one shared managed reference.
    pub fn managed_layout_id(
        &self,
        reference: SharedManagedReference,
    ) -> HeapResult<Option<LayoutId>> {
        self.managed.layout_id(reference)
    }

    /// Set the storage layout id for one shared managed reference.
    pub fn set_managed_layout_id(
        &mut self,
        reference: SharedManagedReference,
        layout_id: LayoutId,
    ) -> HeapResult<()> {
        self.managed.set_layout_id(reference, layout_id)
    }

    /// Overwrite one shared managed byte range.
    pub fn write_managed_bytes(
        &mut self,
        reference: SharedManagedReference,
        start: usize,
        bytes: &[u8],
    ) -> HeapResult<()> {
        let mapped_delta = self
            .managed
            .write_mapped_delta(reference, start, bytes.len())?;
        self.check_managed_mapped_delta(mapped_delta)?;

        self.managed.write_bytes(reference, start, bytes)
    }

    /// Record one shared managed write barrier over one byte range.
    pub fn write_barrier(
        &mut self,
        reference: SharedManagedReference,
        start: usize,
        byte_len: usize,
    ) -> HeapResult<()> {
        self.managed.write_barrier(reference, start, byte_len)
    }

    /// Request one shared collection cycle at the next world step.
    pub fn request_gc(&mut self) {
        self.gc_requested = true;
    }

    /// Perform one full shared managed collection over explicit roots.
    pub fn collect_full(
        &mut self,
        roots: impl IntoIterator<Item = SharedManagedReference>,
    ) -> HeapResult<GcStats> {
        let stats = self.managed.collect_full(roots)?;
        self.note_gc_cycle(stats);

        Ok(stats)
    }

    /// Run one pacing-driven shared collection step with one explicit work budget.
    pub fn gc_step(
        &mut self,
        roots: &[SharedManagedReference],
        roots_complete: bool,
        work_items: usize,
    ) -> HeapResult<Option<GcStats>> {
        if work_items == 0 {
            return Ok(None);
        }

        // idle
        let was_idle = self.gc_phase() == SharedGcPhase::Idle;
        if was_idle {
            if !self.gc_requested {
                return Ok(None);
            }

            self.gc_requested = false;
            self.managed.start_mark(roots.iter().copied())?;
        }

        // concurrent mark
        if self.gc_phase() == SharedGcPhase::Mark {
            let mark_roots = if was_idle { &[][..] } else { roots };
            self.managed
                .mark_step(mark_roots.iter().copied(), work_items)?;

            if self.managed.mark_idle() && roots_complete {
                self.managed.start_sweep()?;
            }

            return Ok(None);
        }

        // incremental sweep
        let stats = self.managed.sweep_step(work_items)?;
        if let Some(stats) = stats {
            self.note_gc_cycle(stats);

            return Ok(Some(stats));
        }

        Ok(None)
    }

    /// Fork this shared heap over the same shared arena.
    pub fn fork(&self) -> HeapResult<Self> {
        Ok(Self {
            arena: self.arena.clone(),
            options: self.options.clone(),
            gc_pacer: self.gc_pacer,
            gc_requested: self.gc_requested,
            managed: self.managed.fork()?,
            raw: self.raw.fork()?,
            limits: self.limits,
        })
    }

    /// Create one shared heap from one frozen shared heap image.
    pub fn from_image_with_arena(arena: Arc<Arena>, image: &SharedHeapImage) -> HeapResult<Self> {
        Self::from_image_with_arena_limits_and_options(
            arena,
            image,
            SharedHeapLimits::default(),
            image.options.clone(),
        )
    }

    /// Create one shared heap from one frozen shared heap image and explicit limits.
    pub fn from_image_with_arena_and_limits(
        arena: Arc<Arena>,
        image: &SharedHeapImage,
        limits: SharedHeapLimits,
    ) -> HeapResult<Self> {
        Self::from_image_with_arena_limits_and_options(arena, image, limits, image.options.clone())
    }

    /// Create one shared heap from one frozen shared heap image and explicit options.
    pub fn from_image_with_arena_and_options(
        arena: Arc<Arena>,
        image: &SharedHeapImage,
        options: HeapOptions,
    ) -> HeapResult<Self> {
        Self::from_image_with_arena_limits_and_options(
            arena,
            image,
            SharedHeapLimits::default(),
            options,
        )
    }

    /// Create one shared heap from one frozen shared heap image, limits, and options.
    pub fn from_image_with_arena_limits_and_options(
        arena: Arc<Arena>,
        image: &SharedHeapImage,
        limits: SharedHeapLimits,
        options: HeapOptions,
    ) -> HeapResult<Self> {
        let mut shared = Self {
            managed: SharedManagedSpace::from_image_with_arena(arena.clone(), &image.managed)?,
            raw: SharedRawSpace::from_image_with_arena(arena.clone(), &image.raw)?,
            arena,
            options,
            gc_pacer: GcPacer::default(),
            gc_requested: false,
            limits,
        };

        shared.refresh_gc_request();
        shared.check_limits()?;

        Ok(shared)
    }

    /// Check the current shared heap usage against the configured limits.
    fn check_limits(&self) -> HeapResult<()> {
        if let Some(max_bytes) = self.limits.max_bytes {
            let active_bytes = self.active_bytes();
            if active_bytes > max_bytes {
                return Err(HeapError::LimitExceeded {
                    space: HeapSpace::Total,
                    used_bytes: active_bytes,
                    max_bytes,
                });
            }
        }

        self.limits.managed.check(self.managed.active_bytes())?;
        self.limits.raw.check(self.raw.active_bytes())?;

        Ok(())
    }

    /// Check one projected mapped-byte delta against shared managed limits.
    fn check_managed_mapped_delta(&self, mapped_delta: i64) -> HeapResult<()> {
        if let Some(max_bytes) = self.limits.max_bytes {
            let active_bytes = apply_byte_delta(self.active_bytes(), mapped_delta)?;
            if active_bytes > max_bytes {
                return Err(HeapError::LimitExceeded {
                    space: HeapSpace::Total,
                    used_bytes: active_bytes,
                    max_bytes,
                });
            }
        }

        self.limits
            .managed
            .check_mapped_delta(self.managed.active_bytes(), mapped_delta)
    }

    /// Check one projected mapped-byte delta against shared raw limits.
    fn check_raw_mapped_delta(&self, mapped_delta: i64) -> HeapResult<()> {
        if let Some(max_bytes) = self.limits.max_bytes {
            let active_bytes = apply_byte_delta(self.active_bytes(), mapped_delta)?;
            if active_bytes > max_bytes {
                return Err(HeapError::LimitExceeded {
                    space: HeapSpace::Total,
                    used_bytes: active_bytes,
                    max_bytes,
                });
            }
        }

        self.limits
            .raw
            .check_mapped_delta(self.raw.active_bytes(), mapped_delta)
    }

    /// Return one frozen shared heap image.
    pub fn image(&self) -> SharedHeapImage {
        SharedHeapImage {
            options: self.options.clone(),
            managed: self.managed.image(),
            raw: self.raw.image(),
        }
    }

    /// Return the number of live shared managed allocations.
    pub fn managed_allocation_count(&self) -> usize {
        self.managed.allocation_count()
    }

    /// Return the number of allocated shared managed bytes.
    pub fn managed_allocated_bytes(&self) -> u64 {
        self.managed.allocated_bytes()
    }

    /// Refresh the collector pacing targets from current shared managed state.
    fn refresh_gc_pacer(&mut self) {
        self.gc_pacer
            .update(self.options.gc, self.managed_allocated_bytes());
    }

    /// Refresh the pending shared cycle request from current managed pressure.
    fn refresh_gc_request(&mut self) {
        self.refresh_gc_pacer();

        if self.gc_pacer.should_start(self.managed_allocated_bytes()) {
            self.gc_requested = true;
        }
    }

    /// Record one completed shared collection cycle in the pacer.
    fn note_gc_cycle(&mut self, stats: GcStats) {
        self.gc_pacer.update(self.options.gc, stats.allocated_bytes);
        self.gc_requested = false;
        self.refresh_gc_request();
    }
}
