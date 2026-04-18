use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::{
    SharedGcPhase, SharedHeapLimits, SharedHeapUsage, SharedManagedSpace, SharedManagedSpaceImage,
    SharedRawSpace, SharedRawSpaceImage,
};
use crate::core::sum_bytes;
use crate::{
    Arena, EdgeMap, GcState, GcStats, HeapResult, LayoutId, PageId, SharedManagedReference,
    SharedRawPointer,
};

/// One live world-shared heap.
#[derive(Debug)]
pub struct SharedHeap {
    /// The shared arena for both shared heap spaces.
    pub(crate) arena: Arc<Arena>,
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
        Self::with_arena_and_limits(Arc::new(Arena::new()), SharedHeapLimits::default())
    }

    /// Create a new empty shared heap over one shared arena.
    pub fn with_arena(arena: Arc<Arena>) -> Self {
        Self::with_arena_and_limits(arena, SharedHeapLimits::default())
    }

    /// Create a new empty shared heap over one shared arena and limit set.
    pub fn with_arena_and_limits(arena: Arc<Arena>, limits: SharedHeapLimits) -> Self {
        Self {
            managed: SharedManagedSpace::with_arena(arena.clone()),
            raw: SharedRawSpace::with_arena(arena.clone()),
            arena,
            limits,
        }
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

    /// Return the current shared managed collector phase.
    pub fn phase(&self) -> SharedGcPhase {
        self.managed.phase()
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
        edge_map: EdgeMap,
        layout_id: Option<LayoutId>,
    ) -> HeapResult<SharedManagedReference> {
        let mapped_delta = self.managed.alloc_mapped_delta(bytes.len());
        self.check_managed_mapped_delta(mapped_delta)?;

        self.managed.allocate_bytes(bytes, edge_map, layout_id)
    }

    /// Allocate one zeroed shared managed byte allocation.
    pub fn allocate_managed_zeroed(
        &mut self,
        byte_len: usize,
        edge_map: EdgeMap,
        layout_id: Option<LayoutId>,
    ) -> HeapResult<SharedManagedReference> {
        let mapped_delta = self.managed.alloc_mapped_delta(byte_len);
        self.check_managed_mapped_delta(mapped_delta)?;

        self.managed.allocate_zeroed(byte_len, edge_map, layout_id)
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

    /// Return the traced edge map for one shared managed reference.
    pub fn edge_map(&self, reference: SharedManagedReference) -> HeapResult<&EdgeMap> {
        self.managed.edge_map(reference)
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

    /// Perform one full shared managed collection over explicit roots.
    pub fn collect(
        &mut self,
        roots: impl IntoIterator<Item = SharedManagedReference>,
    ) -> HeapResult<GcStats> {
        self.managed.collect(roots)
    }

    /// Start one shared managed collection over explicit roots.
    pub fn start_collection(
        &mut self,
        roots: impl IntoIterator<Item = SharedManagedReference>,
    ) -> HeapResult<()> {
        self.managed.start_collection(roots)
    }

    /// Perform bounded shared managed collector work.
    pub fn collect_step(&mut self, work_budget: usize) -> HeapResult<Option<GcStats>> {
        self.managed.collect_step(work_budget)
    }

    /// Return whether shared concurrent mark is currently drained.
    pub fn is_mark_idle(&self) -> bool {
        self.managed.is_mark_idle()
    }

    /// Finish shared marking after the final root handshake.
    pub fn finish_mark(
        &mut self,
        roots: impl IntoIterator<Item = SharedManagedReference>,
    ) -> HeapResult<()> {
        self.managed.finish_mark(roots)
    }

    /// Fork this shared heap over the same shared arena.
    pub fn fork(&self) -> HeapResult<Self> {
        Ok(Self {
            arena: self.arena.clone(),
            managed: self.managed.fork()?,
            raw: self.raw.fork()?,
            limits: self.limits,
        })
    }

    /// Create one shared heap from one frozen shared heap image.
    pub fn from_image_with_arena(arena: Arc<Arena>, image: &SharedHeapImage) -> HeapResult<Self> {
        Self::from_image_with_arena_and_limits(arena, image, SharedHeapLimits::default())
    }

    /// Create one shared heap from one frozen shared heap image and explicit limits.
    pub fn from_image_with_arena_and_limits(
        arena: Arc<Arena>,
        image: &SharedHeapImage,
        limits: SharedHeapLimits,
    ) -> HeapResult<Self> {
        let shared = Self {
            managed: SharedManagedSpace::from_image_with_arena(arena.clone(), &image.managed)?,
            raw: SharedRawSpace::from_image_with_arena(arena.clone(), &image.raw)?,
            arena,
            limits,
        };

        shared.check_limits()?;

        Ok(shared)
    }

    /// Check the current shared heap usage against the configured limits.
    fn check_limits(&self) -> HeapResult<()> {
        self.limits.managed.check(self.managed.active_bytes())?;
        self.limits.raw.check(self.raw.active_bytes())?;

        Ok(())
    }

    /// Check one projected mapped-byte delta against shared managed limits.
    fn check_managed_mapped_delta(&self, mapped_delta: i64) -> HeapResult<()> {
        self.limits
            .managed
            .check_mapped_delta(self.managed.active_bytes(), mapped_delta)
    }

    /// Check one projected mapped-byte delta against shared raw limits.
    fn check_raw_mapped_delta(&self, mapped_delta: i64) -> HeapResult<()> {
        self.limits
            .raw
            .check_mapped_delta(self.raw.active_bytes(), mapped_delta)
    }

    /// Return one frozen shared heap image.
    pub fn image(&self) -> SharedHeapImage {
        SharedHeapImage {
            managed: self.managed.image(),
            raw: self.raw.image(),
        }
    }
}
