use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use destack_mir::{Layout, LayoutId, LayoutTable, ReferenceMap};
use serde::{Deserialize, Serialize};

use super::constants::{
    MAX_SHARED_EDGE_SCAN_BUDGET, MAX_SHARED_MARK_BUDGET, MAX_SHARED_SWEEP_BUDGET,
    MIN_SHARED_EDGE_SCAN_BUDGET, MIN_SHARED_MARK_BUDGET, MIN_SHARED_SWEEP_BUDGET,
    SHARED_ASSIST_BUDGET, SHARED_EDGE_SCAN_BUDGET_PER_WORKER, SHARED_MARK_BUDGET_PER_WORKER,
    SHARED_MARK_PAGES_PER_BUDGET, SHARED_SWEEP_BUDGET_PER_WORKER, SHARED_SWEEP_PAGES_PER_BUDGET,
};
use super::{
    SharedGcPhase, SharedHeapLimits, SharedHeapSpace, SharedHeapSpaceImage, SharedHeapUsage,
    SharedRawSpace, SharedRawSpaceImage,
};
use crate::{
    Allocator, GcPacer, GcState, GcStats, HeapError, HeapOptions, HeapResult, PageId, Payload,
    SharedHeapReference, SharedRawPointer, apply_byte_delta,
};

/// One live world-shared heap.
#[derive(Debug)]
pub struct SharedHeap {
    /// The shared allocator for both shared heap spaces.
    pub(crate) allocator: Arc<Allocator>,
    /// The configured shared heap options.
    pub(crate) options: HeapOptions,

    /// The traced shared heap space.
    pub(crate) heap: SharedHeapSpace,
    /// The explicit shared raw space.
    pub(crate) raw: SharedRawSpace,
    /// The exact hard limits for this shared heap.
    pub(crate) limits: SharedHeapLimits,

    /// Whether one shared collection has been requested by pressure or explicitly.
    collection_requested: AtomicBool,
    /// Pending shared assist debt derived from recent heap allocation pressure.
    assist_debt: AtomicUsize,
}

/// One frozen shared heap.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedHeapImage {
    /// The captured shared heap options.
    pub options: HeapOptions,
    /// The frozen shared heap space.
    pub heap: SharedHeapSpaceImage,
    /// The frozen shared raw space.
    pub raw: SharedRawSpaceImage,
}

impl SharedHeapImage {
    /// Return every allocator page reachable from this shared-heap image.
    pub fn page_ids(&self) -> Vec<PageId> {
        let mut pages = self.heap.page_ids();
        pages.extend(self.raw.page_ids());

        pages
    }
}

impl SharedHeap {
    /// Create one shared heap over one explicit allocator, layout table, limits, and options.
    pub fn with_allocator_limits_layouts_and_options(
        allocator: Arc<Allocator>,
        layouts: Arc<LayoutTable>,
        limits: SharedHeapLimits,
        options: HeapOptions,
    ) -> HeapResult<Self> {
        options.validate_shared()?;
        options.validate_allocator(&allocator)?;

        let shared = Self {
            heap: SharedHeapSpace::with_layouts_and_options(allocator.clone(), layouts, &options)?,
            raw: SharedRawSpace::with_allocator(allocator.clone()),
            allocator,
            options,
            collection_requested: AtomicBool::new(false),
            assist_debt: AtomicUsize::new(0),
            limits,
        };

        shared.refresh_gc_request();

        Ok(shared)
    }

    /// Return the configured shared page size.
    pub fn page_bytes(&self) -> usize {
        self.allocator.page_bytes()
    }

    /// Return the configured shared heap options.
    pub fn options(&self) -> &HeapOptions {
        &self.options
    }

    /// Return the configured shared heap limits.
    pub fn limits(&self) -> SharedHeapLimits {
        self.limits
    }

    /// Return the exact live usage for this shared heap.
    pub fn usage(&self) -> SharedHeapUsage {
        SharedHeapUsage {
            heap: self.heap.usage(),
            raw: self.raw.usage(),
        }
    }

    /// Return the exact active shared heap bytes.
    pub fn active_bytes(&self) -> u64 {
        self.heap.active_bytes() + self.raw.active_bytes()
    }

    /// Return the current shared heap collector state.
    pub fn gc_state(&self) -> GcState {
        self.heap.gc_state()
    }

    /// Return whether the active shared mark phase is currently drained.
    pub fn mark_idle(&self) -> bool {
        self.heap.mark_idle()
    }

    /// Return the current derived collector pacing targets.
    pub fn gc_pacer(&self) -> GcPacer {
        let mut gc_pacer = GcPacer::default();
        gc_pacer.update(self.options.gc, self.heap_allocated_bytes());

        gc_pacer
    }

    /// Return the bounded shared collection budget for one world step.
    fn base_collection_budget(&self, worker_count: usize) -> usize {
        // heap pressure
        let worker_count = worker_count.max(1);
        let page_bytes = self.page_bytes().max(1);
        let heap_pages = (self.heap_allocated_bytes() as usize).div_ceil(page_bytes);

        // sweep gets a larger budget because it is pure reclamation work
        if self.gc_phase() == SharedGcPhase::Sweep {
            let heap_budget = heap_pages.div_ceil(SHARED_SWEEP_PAGES_PER_BUDGET).max(1);
            let sweep_work = worker_count
                .saturating_mul(SHARED_SWEEP_BUDGET_PER_WORKER)
                .saturating_add(heap_budget);

            return sweep_work.clamp(MIN_SHARED_SWEEP_BUDGET, MAX_SHARED_SWEEP_BUDGET);
        }

        // mark budget
        let heap_budget = heap_pages.div_ceil(SHARED_MARK_PAGES_PER_BUDGET).max(1);
        let mark_work = worker_count
            .saturating_mul(SHARED_MARK_BUDGET_PER_WORKER)
            .saturating_add(heap_budget);

        mark_work.clamp(MIN_SHARED_MARK_BUDGET, MAX_SHARED_MARK_BUDGET)
    }

    /// Return and consume one bounded shared collection budget for the current world step.
    pub fn take_collection_budget(&self, worker_count: usize) -> usize {
        // world budget
        let base_budget = self.base_collection_budget(worker_count);

        // pending assist debt
        let assist_work = self
            .assist_debt
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |pending| {
                let consumed = pending.min(base_budget);
                Some(pending - consumed)
            })
            .map(|pending| pending.min(base_budget))
            .unwrap_or(0);

        base_budget.saturating_add(assist_work)
    }

    /// Return and consume one bounded shared assist budget for one mutator step.
    pub fn take_assist_budget(&self) -> usize {
        // bounded assist slice
        self.assist_debt
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |pending| {
                let consumed = pending.min(SHARED_ASSIST_BUDGET);
                Some(pending - consumed)
            })
            .map(|pending| pending.min(SHARED_ASSIST_BUDGET))
            .unwrap_or(0)
    }

    /// Return the bounded local-to-shared edge scan budget for one worker step.
    pub fn edge_scan_budget(&self, worker_count: usize) -> usize {
        // worker-scaled budget
        let worker_count = worker_count.max(1);
        let edge_budget = worker_count.saturating_mul(SHARED_EDGE_SCAN_BUDGET_PER_WORKER);

        edge_budget.clamp(MIN_SHARED_EDGE_SCAN_BUDGET, MAX_SHARED_EDGE_SCAN_BUDGET)
    }

    /// Return the current shared heap collector phase.
    pub fn gc_phase(&self) -> SharedGcPhase {
        self.heap.gc_phase()
    }

    /// Return the exact mapped shared heap bytes.
    pub fn mapped_bytes(&self) -> u64 {
        self.heap.mapped_bytes() + self.raw.mapped_bytes()
    }

    /// Return the exact borrowed shared heap bytes.
    pub fn borrowed_bytes(&self) -> u64 {
        self.heap
            .borrowed_bytes()
            .checked_add(self.raw.borrowed_bytes())
            .unwrap_or_else(|| panic!("shared heap usage overflow: borrowed bytes"))
    }

    /// Return the exact active shared raw-space bytes.
    pub fn raw_active_bytes(&self) -> u64 {
        self.raw.active_bytes()
    }

    /// Return the projected mapped-byte delta for one shared raw allocation.
    pub fn raw_alloc_mapped_byte_delta(&self, byte_len: usize) -> i64 {
        self.raw.alloc_mapped_byte_delta(byte_len)
    }

    /// Return the projected mapped-byte delta for one shared raw replacement.
    pub fn raw_replace_mapped_byte_delta(
        &self,
        pointer: SharedRawPointer,
        next_byte_len: usize,
    ) -> HeapResult<i64> {
        self.raw.replace_mapped_byte_delta(pointer, next_byte_len)
    }

    /// Allocate one shared raw entry.
    pub fn allocate_raw(
        &self,
        byte_len: usize,
        allocation: Payload<'_>,
    ) -> HeapResult<SharedRawPointer> {
        self.check_raw_mapped_byte_delta(self.raw.alloc_mapped_byte_delta(byte_len))?;

        self.raw.allocate(byte_len, allocation)
    }

    /// Replace one shared raw allocation payload.
    pub fn replace_raw_bytes(
        &self,
        pointer: SharedRawPointer,
        bytes: &[u8],
    ) -> HeapResult<SharedRawPointer> {
        self.check_raw_mapped_byte_delta(
            self.raw.replace_mapped_byte_delta(pointer, bytes.len())?,
        )?;

        self.raw.replace_bytes(pointer, bytes)
    }

    /// Return the bytes for one shared raw allocation.
    pub fn read_raw_bytes(&self, pointer: SharedRawPointer) -> HeapResult<Vec<u8>> {
        self.raw.read_bytes(pointer)
    }

    /// Allocate one shared managed heap entry.
    pub fn allocate(
        &self,
        layout_id: LayoutId,
        allocation: Payload<'_>,
    ) -> HeapResult<SharedHeapReference> {
        let byte_len = self.heap.layout_byte_len(layout_id)?;

        // projected growth
        let mapped_byte_delta = self.heap.mapped_byte_delta(layout_id)?;
        self.check_heap_mapped_byte_delta(mapped_byte_delta)?;

        // allocation and pacing
        let reference = self.heap.allocate(layout_id, allocation)?;
        self.accrue_assist_debt(byte_len);
        self.refresh_gc_request();

        Ok(reference)
    }

    /// Register one shared managed layout and return its stable id.
    pub fn register_layout(&self, layout: Layout) -> LayoutId {
        self.heap.register_layout(layout)
    }

    /// Return whether one shared heap reference currently refers to one live entry.
    pub fn is_heap_live(&self, reference: SharedHeapReference) -> bool {
        self.heap.is_live(reference)
    }

    /// Return the remaining byte length for one shared heap reference.
    pub fn heap_byte_len(&self, reference: SharedHeapReference) -> HeapResult<usize> {
        self.heap.byte_len(reference)
    }

    /// Return the bytes for one shared heap reference.
    pub fn read_heap_bytes(&self, reference: SharedHeapReference) -> HeapResult<Vec<u8>> {
        self.heap.read_bytes(reference)
    }

    /// Fill one caller-provided buffer from one shared heap entry at one offset.
    pub fn read_heap_bytes_into(
        &self,
        reference: SharedHeapReference,
        start: usize,
        target: &mut [u8],
    ) -> HeapResult<()> {
        self.heap.read_bytes_into(reference, start, target)
    }

    /// Return the scan metadata for one shared heap reference.
    pub fn scan(&self, reference: SharedHeapReference) -> HeapResult<ReferenceMap> {
        self.heap.scan(reference)
    }

    /// Overwrite one shared heap byte range.
    pub fn write_heap_bytes(
        &self,
        reference: SharedHeapReference,
        start: usize,
        bytes: &[u8],
    ) -> HeapResult<()> {
        self.heap.write_bytes(reference, start, bytes)
    }

    /// Record one shared heap write barrier over one byte range.
    pub fn write_barrier(
        &self,
        reference: SharedHeapReference,
        start: usize,
        byte_len: usize,
    ) -> HeapResult<()> {
        self.heap.write_barrier(reference, start, byte_len)
    }

    /// Record one shared heap write barrier from one caller-provided byte slice.
    pub fn write_barrier_bytes(
        &self,
        reference: SharedHeapReference,
        start: usize,
        bytes: &[u8],
    ) -> HeapResult<()> {
        self.heap
            .write_shared_barrier_bytes(reference, start, bytes)
    }

    /// Publish one exact shared heap reference after one completed store.
    pub fn publish_edge(&self, reference: SharedHeapReference) -> HeapResult<()> {
        self.heap.publish_edge(reference)
    }

    /// Request one shared collection cycle at the next world step.
    pub fn request_gc(&self) {
        self.collection_requested.store(true, Ordering::Release);
    }

    /// Start one requested or pressure-driven shared collection cycle.
    pub fn start_gc(&self) -> HeapResult<bool> {
        // active cycle
        if self.gc_phase() != SharedGcPhase::Idle {
            return Ok(false);
        }

        // no pending request
        if !self.collection_requested.load(Ordering::Acquire) {
            return Ok(false);
        }

        // cycle start
        self.heap.start_mark([])?;
        self.collection_requested.store(false, Ordering::Release);

        Ok(true)
    }

    /// Perform one full shared heap collection over explicit roots.
    pub fn collect_full(
        &self,
        roots: impl IntoIterator<Item = SharedHeapReference>,
    ) -> HeapResult<GcStats> {
        let stats = self.heap.collect_full(roots)?;
        self.record_gc_cycle(stats);

        Ok(stats)
    }

    /// Run one shared collection step with one explicit work budget.
    pub fn gc_step(
        &self,
        roots: &[SharedHeapReference],
        roots_complete: bool,
        work_items: usize,
    ) -> HeapResult<Option<GcStats>> {
        // empty budget
        if work_items == 0 {
            return Ok(None);
        }

        // idle
        if self.gc_phase() == SharedGcPhase::Idle {
            return Ok(None);
        }

        // concurrent mark
        if self.gc_phase() == SharedGcPhase::Mark {
            self.heap.mark_step(roots.iter().copied(), work_items)?;

            // termination check
            if roots_complete {
                self.heap.try_start_sweep()?;
            }

            return Ok(None);
        }

        // incremental sweep
        let stats = self.heap.sweep_step(work_items)?;
        if let Some(stats) = stats {
            self.record_gc_cycle(stats);

            return Ok(Some(stats));
        }

        Ok(None)
    }

    /// Fork this shared heap over the same shared allocator.
    pub fn fork(&self) -> HeapResult<Self> {
        Ok(Self {
            allocator: self.allocator.clone(),
            options: self.options.clone(),
            collection_requested: AtomicBool::new(
                self.collection_requested.load(Ordering::Acquire),
            ),
            assist_debt: AtomicUsize::new(0),
            heap: self.heap.fork()?,
            raw: self.raw.fork()?,
            limits: self.limits,
        })
    }

    /// Create one shared heap from one frozen shared heap image, limits, and options.
    pub fn from_image_with_allocator_limits_and_options(
        allocator: Arc<Allocator>,
        image: &SharedHeapImage,
        limits: SharedHeapLimits,
        options: HeapOptions,
    ) -> HeapResult<Self> {
        options.validate_shared()?;
        options.validate_allocator(&allocator)?;

        let shared = Self {
            heap: SharedHeapSpace::from_image_with_allocator(allocator.clone(), &image.heap)?,
            raw: SharedRawSpace::from_image_with_allocator(allocator.clone(), &image.raw)?,
            allocator,
            options,
            collection_requested: AtomicBool::new(false),
            assist_debt: AtomicUsize::new(0),
            limits,
        };

        shared.refresh_gc_request();
        shared.check_limits()?;

        Ok(shared)
    }

    /// Check the current shared heap usage against the configured limits.
    fn check_limits(&self) -> HeapResult<()> {
        // total limit
        if let Some(max_bytes) = self.limits.max_bytes {
            let active_bytes = self.active_bytes();
            if active_bytes > max_bytes {
                return Err(HeapError::TotalLimitExceeded {
                    used_bytes: active_bytes,
                    max_bytes,
                });
            }
        }

        // per-space limits
        self.limits.heap.check(self.heap.active_bytes())?;
        self.limits.raw.check(self.raw.active_bytes())?;

        Ok(())
    }

    /// Check one projected mapped-byte delta against shared heap limits.
    fn check_heap_mapped_byte_delta(&self, mapped_byte_delta: i64) -> HeapResult<()> {
        // total limit
        if let Some(max_bytes) = self.limits.max_bytes {
            let active_bytes = apply_byte_delta(self.active_bytes(), mapped_byte_delta)?;
            if active_bytes > max_bytes {
                return Err(HeapError::TotalLimitExceeded {
                    used_bytes: active_bytes,
                    max_bytes,
                });
            }
        }

        // heap limit
        self.limits
            .heap
            .check_mapped_byte_delta(self.heap.active_bytes(), mapped_byte_delta)
    }

    /// Check one projected mapped-byte delta against shared raw limits.
    fn check_raw_mapped_byte_delta(&self, mapped_byte_delta: i64) -> HeapResult<()> {
        // total limit
        if let Some(max_bytes) = self.limits.max_bytes {
            let active_bytes = apply_byte_delta(self.active_bytes(), mapped_byte_delta)?;
            if active_bytes > max_bytes {
                return Err(HeapError::TotalLimitExceeded {
                    used_bytes: active_bytes,
                    max_bytes,
                });
            }
        }

        // raw limit
        self.limits
            .raw
            .check_mapped_byte_delta(self.raw.active_bytes(), mapped_byte_delta)
    }

    /// Return one frozen shared heap image.
    pub fn image(&self) -> SharedHeapImage {
        SharedHeapImage {
            options: self.options.clone(),
            heap: self.heap.image(),
            raw: self.raw.image(),
        }
    }

    /// Return the number of live shared heap allocations.
    pub fn heap_allocation_count(&self) -> usize {
        self.heap.allocation_count()
    }

    /// Return the number of allocated shared heap bytes.
    pub fn heap_allocated_bytes(&self) -> u64 {
        self.heap.allocated_bytes()
    }

    /// Refresh the pending shared cycle request from current heap pressure.
    fn refresh_gc_request(&self) {
        // trigger crossing
        if self.gc_pacer().should_start(self.heap_allocated_bytes()) {
            self.collection_requested.store(true, Ordering::Release);
        }
    }

    /// Record one completed shared collection cycle in the pacer.
    fn record_gc_cycle(&self, _stats: GcStats) {
        // clear the completed cycle state
        self.collection_requested.store(false, Ordering::Release);
        self.assist_debt.store(0, Ordering::Release);

        // re-evaluate current pressure
        self.refresh_gc_request();
    }

    /// Accrue shared assist debt from one heap allocation.
    fn accrue_assist_debt(&self, allocated_bytes: usize) {
        // empty allocation
        if allocated_bytes == 0 {
            return;
        }

        // convert bytes into page-scaled work debt
        let page_bytes = self.page_bytes().max(1);
        let work_items = allocated_bytes.div_ceil(page_bytes).max(1);

        // saturating debt
        let _ = self
            .assist_debt
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |pending| {
                Some(pending.saturating_add(work_items))
            });
    }
}
