use std::borrow::Cow;
use std::cell::Cell;
use std::mem::size_of;
use std::sync::Arc;

use destack_mir::LayoutId;

use super::{
    GcState, ManagedImage, ManagedLargeAllocation, ManagedLargeAllocationId, ManagedSpan,
    ManagedYoungId, ReferenceMap, ReferenceMapId, ReferenceMapTable, YoungSpace,
};
use crate::alloc::{
    ChunkPayload, PageArena, PageId, SizeClassTable, projected_vec_capacity,
    vec_capacity_bytes_delta,
};
use crate::heap::{
    HeapCaptureError, HeapLayoutOptions, ManagedSpaceUsage, validate_managed_reference_bytes,
};
use crate::value::ManagedReference;

/// The first non-null managed reference id.
pub(crate) const FIRST_ALLOCATED_REFERENCE_ID: u64 = 1;

/// The first non-null managed large-allocation id.
const FIRST_ALLOCATED_LARGE_ALLOCATION_ID: u64 = 1;

/// The survivor age at which one young allocation promotes to mature storage.
pub(crate) const YOUNG_PROMOTION_AGE: u8 = 2;

/// One stable span slot location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) struct ManagedSpanSlot {
    /// The containing span index.
    span_index: u32,
    /// The slot index inside the span.
    slot_index: u32,
}

impl ManagedSpanSlot {
    /// Create one span slot location.
    pub(crate) const fn new(span_index: usize, slot_index: usize) -> Self {
        Self {
            span_index: span_index as u32,
            slot_index: slot_index as u32,
        }
    }

    /// Return the containing span index.
    pub(crate) const fn span_index(self) -> usize {
        self.span_index as usize
    }

    /// Return the slot index inside the span.
    pub(crate) const fn slot_index(self) -> usize {
        self.slot_index as usize
    }
}

/// One stable managed allocation location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) enum ManagedLocation {
    /// One vacant directory slot.
    Vacant,
    /// One young-space allocation stored in the bump arena.
    Young(ManagedYoungId),
    /// One small-space allocation stored in one span slot.
    Small(ManagedSpanSlot),
    /// One large-space allocation stored in one managed large allocation.
    Large(ManagedLargeAllocationId),
}

/// One live managed handle entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) struct ManagedHandle {
    /// The storage location for this managed allocation.
    pub(crate) location: ManagedLocation,
    /// The logical byte length for this allocation.
    pub(crate) byte_len: usize,
    /// The nominal type id for this allocation, if any.
    pub(crate) type_id: Option<u32>,
    /// The dense live-handle index for this allocation.
    pub(crate) live_index: u32,
    /// Reserved per-handle flags.
    pub(crate) flags: u32,
}

/// One managed handle-table entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) enum ManagedHandleEntry {
    /// One free handle-table entry linked into the intrusive free list.
    Free { next_free: u64 },
    /// One live managed allocation.
    Live(ManagedHandle),
}

/// One managed small-allocation space.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SmallSpace {
    /// The configured size-class table.
    pub(crate) size_classes: SizeClassTable,
    /// The configured span width.
    pub(crate) span_bytes: usize,
    /// The live managed spans.
    pub(crate) spans: Vec<ManagedSpan>,
    /// The reusable non-full spans per size class.
    pub(crate) available_spans: Vec<Vec<usize>>,
    /// The vacant span slots available for reuse.
    pub(crate) free_span_ids: Vec<usize>,
}

/// One managed large-allocation space.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LargeSpace {
    /// The configured page width for large allocations.
    pub(crate) page_bytes: usize,
    /// The live managed large allocations.
    pub(crate) large_allocations: Vec<ManagedLargeAllocation>,
    /// The free managed large-allocation ids available for reuse.
    pub(crate) free_large_allocation_ids: Vec<u64>,
    /// The next managed large-allocation id to allocate.
    pub(crate) next_unused_large_allocation_id: u64,
}

/// One retained-byte snapshot for the managed young space.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct YoungRetainedSnapshot {
    /// The local-page vector capacity.
    pages_capacity: usize,
    /// The local-page count.
    page_count: usize,
    /// The young-allocation metadata capacity.
    allocations_capacity: usize,
    /// The free-id capacity.
    free_ids_capacity: usize,
}

impl YoungRetainedSnapshot {
    /// Capture one retained-byte snapshot for the given young space.
    fn capture(space: &YoungSpace) -> Self {
        Self {
            pages_capacity: space.pages_capacity(),
            page_count: space.page_count(),
            allocations_capacity: space.allocations_capacity(),
            free_ids_capacity: space.free_ids_capacity(),
        }
    }

    /// Return the retained-byte delta relative to the current young space.
    fn retained_delta(self, space: &YoungSpace) -> i64 {
        capacity_bytes_delta::<PageId>(self.pages_capacity, space.pages_capacity())
            + capacity_bytes_delta::<super::YoungAllocation>(
                self.allocations_capacity,
                space.allocations_capacity(),
            )
            + capacity_bytes_delta::<u32>(self.free_ids_capacity, space.free_ids_capacity())
    }
}

/// One live managed allocation space.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagedSpace {
    /// The encoded byte width for managed references inside traced payloads.
    pub(crate) managed_reference_bytes: u8,
    /// The interned managed reference maps.
    pub(crate) reference_map_table: ReferenceMapTable,
    /// The managed young-allocation space.
    pub(crate) young: YoungSpace,
    /// The previous young generation during an active collection.
    pub(crate) young_from: Option<YoungSpace>,
    /// The managed small-allocation space.
    pub(crate) small: SmallSpace,
    /// The managed large-allocation space.
    pub(crate) large: LargeSpace,
    /// The local page arena for large-allocation backing.
    pub(crate) page_arena: PageArena,
    /// Dense managed handle metadata keyed by reference id minus one.
    pub(crate) handles: Vec<ManagedHandleEntry>,
    /// Dense live managed reference ids.
    pub(crate) live_handles: Vec<u64>,
    /// The free managed handle id at the head of the intrusive free list.
    pub(crate) free_handle_head: u64,
    /// The next managed reference id to allocate.
    pub(crate) next_unused_id: u64,
    /// The number of allocated managed references.
    pub(crate) allocated_count: usize,
    /// The number of allocated managed bytes.
    pub(crate) allocated_bytes: u64,
    /// The exact retained managed bytes.
    pub(crate) retained_bytes: Cell<u64>,
    /// Whether the retained-byte cache needs one exact recomputation.
    pub(crate) retained_bytes_dirty: Cell<bool>,
    /// The live GC state.
    pub(crate) gc_state: GcState,
    /// The pending mark queue.
    pub(crate) mark_queue: Vec<ManagedReference>,
    /// Reused outgoing-reference scratch space for tracing.
    pub(crate) trace_scratch: Vec<ManagedReference>,
    /// Dirty small spans that may still point into the young generation.
    pub(crate) dirty_spans: Vec<usize>,
    /// Dirty large allocations that may still point into the young generation.
    pub(crate) dirty_large_allocations: Vec<ManagedLargeAllocationId>,
    /// The rebuilt dirty small-span queue for the active collection.
    pub(crate) dirty_next_spans: Vec<usize>,
    /// The rebuilt dirty large-allocation queue for the active collection.
    pub(crate) dirty_next_large_allocations: Vec<ManagedLargeAllocationId>,
}

impl Default for ManagedSpace {
    fn default() -> Self {
        Self::new()
    }
}

impl ManagedSpace {
    /// Create one managed space with the default layout options.
    pub fn new() -> Self {
        Self::with_layout(&HeapLayoutOptions::default())
    }

    /// Create one managed space with explicit layout options.
    pub fn with_layout(options: &HeapLayoutOptions) -> Self {
        validate_managed_reference_bytes(options.managed_reference_bytes);

        let space = Self {
            managed_reference_bytes: options.managed_reference_bytes,
            reference_map_table: ReferenceMapTable::new(),
            young: YoungSpace::with_capacity(options.managed_young_bytes, options.page_bytes),
            young_from: None,
            small: SmallSpace {
                size_classes: options.size_classes.clone(),
                span_bytes: options.managed_small_bytes,
                spans: Vec::new(),
                available_spans: vec![Vec::new(); options.size_classes.classes.len()],
                free_span_ids: Vec::new(),
            },
            large: LargeSpace {
                page_bytes: options.page_bytes,
                large_allocations: Vec::new(),
                free_large_allocation_ids: Vec::new(),
                next_unused_large_allocation_id: FIRST_ALLOCATED_LARGE_ALLOCATION_ID,
            },
            page_arena: PageArena::with_page_bytes(options.page_bytes),
            handles: Vec::new(),
            live_handles: Vec::new(),
            free_handle_head: 0,
            next_unused_id: FIRST_ALLOCATED_REFERENCE_ID,
            allocated_count: 0,
            allocated_bytes: 0,
            retained_bytes: Cell::new(0),
            retained_bytes_dirty: Cell::new(false),
            gc_state: GcState::default(),
            mark_queue: Vec::new(),
            trace_scratch: Vec::new(),
            dirty_spans: Vec::new(),
            dirty_large_allocations: Vec::new(),
            dirty_next_spans: Vec::new(),
            dirty_next_large_allocations: Vec::new(),
        };

        // exact retained bytes
        space.recompute_retained_bytes();

        space
    }

    /// Restore one managed space from one immutable image.
    pub fn from_image(image: &ManagedImage) -> Self {
        validate_managed_reference_bytes(image.managed_reference_bytes);

        let mut space = Self {
            managed_reference_bytes: image.managed_reference_bytes,
            reference_map_table: ReferenceMapTable::from_maps(image.reference_maps.clone()),
            young: YoungSpace::with_capacity(image.young_bytes, image.page_bytes),
            young_from: None,
            small: SmallSpace {
                size_classes: image.size_classes.clone(),
                span_bytes: image.small_bytes,
                spans: image
                    .spans
                    .iter()
                    .cloned()
                    .map(ManagedSpan::from_image)
                    .collect(),
                available_spans: vec![Vec::new(); image.size_classes.classes.len()],
                free_span_ids: Vec::new(),
            },
            large: LargeSpace {
                page_bytes: image.page_bytes,
                large_allocations: image
                    .large_allocations
                    .iter()
                    .map(ManagedLargeAllocation::from_large_allocation_image)
                    .collect(),
                free_large_allocation_ids: image
                    .free_large_allocation_ids
                    .iter()
                    .copied()
                    .collect(),
                next_unused_large_allocation_id: image.next_unused_large_allocation_id,
            },
            page_arena: PageArena::with_page_bytes(image.page_bytes),
            handles: image.handles.iter().copied().collect(),
            live_handles: Vec::new(),
            free_handle_head: image.free_handle_head,
            next_unused_id: image.next_unused_id,
            allocated_count: image.allocated_count,
            allocated_bytes: image.allocated_bytes,
            retained_bytes: Cell::new(0),
            retained_bytes_dirty: Cell::new(false),
            gc_state: image.gc_state.clone(),
            mark_queue: Vec::new(),
            trace_scratch: Vec::new(),
            dirty_spans: Vec::new(),
            dirty_large_allocations: Vec::new(),
            dirty_next_spans: Vec::new(),
            dirty_next_large_allocations: Vec::new(),
        };

        // rebuild span directories
        space.rebuild_span_directories();
        space.rebuild_live_handles();

        // exact retained bytes
        space.recompute_retained_bytes();

        space
    }

    /// Return the number of live managed allocations.
    pub fn allocation_count(&self) -> usize {
        self.allocated_count
    }

    /// Return the logical live managed payload bytes.
    pub fn allocated_bytes(&self) -> u64 {
        self.allocated_bytes
    }

    /// Return the exact active managed allocator bytes.
    pub fn active_bytes(&self) -> u64 {
        self.refresh_retained_bytes();
        self.retained_bytes.get()
    }

    /// Return the exact mapped managed page-arena bytes.
    pub fn mapped_bytes(&self) -> u64 {
        self.page_arena.mapped_bytes() as u64
    }

    /// Return the exact borrowed managed image bytes.
    pub fn borrowed_bytes(&self) -> u64 {
        let mut borrowed_bytes = 0usize;

        for span in &self.small.spans {
            borrowed_bytes += span.borrowed_bytes(self.page_arena.page_bytes());
        }

        for large_allocation in &self.large.large_allocations {
            borrowed_bytes += large_allocation.borrowed_bytes();
        }

        borrowed_bytes as u64
    }

    /// Return the current GC state.
    pub fn gc_state(&self) -> &GcState {
        &self.gc_state
    }

    /// Return the exact live usage for this managed space.
    pub fn usage(&self) -> ManagedSpaceUsage {
        ManagedSpaceUsage {
            allocation_count: self.allocated_count,
            allocated_bytes: self.allocated_bytes,
            active_bytes: self.active_bytes(),
            mapped_bytes: self.mapped_bytes(),
            borrowed_bytes: self.borrowed_bytes(),
        }
    }

    /// Capture one immutable managed-space image.
    pub fn image(&mut self, base: Option<&ManagedImage>) -> Result<ManagedImage, HeapCaptureError> {
        // reject active collection work
        if self.gc_state.phase != super::GcPhase::Idle {
            return Err(HeapCaptureError::GcActive);
        }

        // reject active pins
        if self.active_pins() > 0 {
            return Err(HeapCaptureError::PinnedManagedReferences);
        }

        // drain the young space before capturing durable storage
        self.drain_young_to_mature();

        // capture spans
        let spans = self
            .small
            .spans
            .iter_mut()
            .enumerate()
            .map(|(index, span)| {
                let _ = base.and_then(|image| image.spans.get(index));
                span.image(&mut self.page_arena)
            })
            .collect::<Vec<_>>();

        // capture large allocations
        let large_allocations = self
            .large
            .large_allocations
            .iter_mut()
            .enumerate()
            .map(|(index, large_allocation)| {
                let _ = base.and_then(|image| image.large_allocations.get(index));
                large_allocation.large_allocation_image(&mut self.page_arena)
            })
            .collect::<Vec<_>>();

        Ok(ManagedImage {
            size_classes: self.small.size_classes.clone(),
            managed_reference_bytes: self.managed_reference_bytes,
            young_bytes: self.young.capacity_bytes(),
            small_bytes: self.small.span_bytes,
            page_bytes: self.large.page_bytes,
            spans,
            large_allocations,
            handles: Arc::from(self.handles.as_slice()),
            free_handle_head: self.free_handle_head,
            free_large_allocation_ids: Arc::from(self.large.free_large_allocation_ids.as_slice()),
            next_unused_id: self.next_unused_id,
            next_unused_large_allocation_id: self.large.next_unused_large_allocation_id,
            allocated_count: self.allocated_count,
            allocated_bytes: self.allocated_bytes,
            reference_maps: self.reference_map_table.snapshot(),
            gc_state: self.gc_state.clone(),
        })
    }

    /// Return the currently allocated managed references.
    pub fn allocated_references(&self) -> Vec<ManagedReference> {
        self.live_handles
            .iter()
            .copied()
            .map(ManagedReference::new)
            .collect()
    }

    // allocate one young managed payload if space is available
    fn allocate_young_bytes(
        &mut self,
        bytes: &[u8],
        trace_id: ReferenceMapId,
        layout_id: Option<LayoutId>,
    ) -> Option<ManagedYoungId> {
        if bytes.len() > self.young.capacity_bytes() {
            return None;
        }

        self.young
            .allocate(bytes, trace_id, layout_id, &mut self.page_arena)
    }

    // allocate one zeroed young managed payload if space is available
    fn allocate_zeroed_young_bytes(
        &mut self,
        byte_len: usize,
        trace_id: ReferenceMapId,
        layout_id: Option<LayoutId>,
    ) -> Option<ManagedYoungId> {
        if byte_len > self.young.capacity_bytes() {
            return None;
        }

        self.young
            .allocate_zeroed(byte_len, trace_id, layout_id, &mut self.page_arena)
    }

    /// Allocate one managed byte allocation.
    pub(crate) fn allocate_bytes(
        &mut self,
        bytes: &[u8],
        reference_map: ReferenceMap,
        layout_id: Option<LayoutId>,
    ) -> ManagedReference {
        let (reference_map_id, reference_map_delta) =
            self.reference_map_table.intern(reference_map);
        self.allocate_bytes_with_trace_id(
            bytes,
            reference_map_id,
            layout_id,
            None,
            reference_map_delta,
        )
    }

    /// Allocate one managed byte allocation and stamp one nominal type id.
    pub(crate) fn allocate_bytes_typed(
        &mut self,
        bytes: &[u8],
        reference_map: ReferenceMap,
        layout_id: Option<LayoutId>,
        type_id: u32,
    ) -> ManagedReference {
        let (reference_map_id, reference_map_delta) =
            self.reference_map_table.intern(reference_map);
        self.allocate_bytes_with_trace_id(
            bytes,
            reference_map_id,
            layout_id,
            Some(type_id),
            reference_map_delta,
        )
    }

    /// Allocate one managed byte allocation using one borrowed reference map.
    pub(crate) fn allocate_bytes_borrowed(
        &mut self,
        bytes: &[u8],
        reference_map: &ReferenceMap,
        layout_id: Option<LayoutId>,
    ) -> ManagedReference {
        let (reference_map_id, reference_map_delta) =
            self.reference_map_table.intern_borrowed(reference_map);
        self.allocate_bytes_with_trace_id(
            bytes,
            reference_map_id,
            layout_id,
            None,
            reference_map_delta,
        )
    }

    /// Allocate one borrowed managed byte allocation and stamp one nominal type id.
    pub(crate) fn allocate_bytes_borrowed_typed(
        &mut self,
        bytes: &[u8],
        reference_map: &ReferenceMap,
        layout_id: Option<LayoutId>,
        type_id: u32,
    ) -> ManagedReference {
        let (reference_map_id, reference_map_delta) =
            self.reference_map_table.intern_borrowed(reference_map);
        self.allocate_bytes_with_trace_id(
            bytes,
            reference_map_id,
            layout_id,
            Some(type_id),
            reference_map_delta,
        )
    }

    /// Allocate one repeated-offset managed byte allocation using borrowed offsets.
    pub(crate) fn allocate_bytes_repeated_reference_offsets(
        &mut self,
        bytes: &[u8],
        count: u32,
        element_size: u32,
        offsets: &[u32],
        layout_id: Option<LayoutId>,
    ) -> ManagedReference {
        let (reference_map_id, reference_map_delta) = self
            .reference_map_table
            .intern_repeated_reference_offsets(count, element_size, offsets);
        self.allocate_bytes_with_trace_id(
            bytes,
            reference_map_id,
            layout_id,
            None,
            reference_map_delta,
        )
    }

    /// Allocate one managed byte allocation from one interned trace id.
    fn allocate_bytes_with_trace_id(
        &mut self,
        bytes: &[u8],
        reference_map_id: ReferenceMapId,
        layout_id: Option<LayoutId>,
        type_id: Option<u32>,
        reference_map_delta: i64,
    ) -> ManagedReference {
        let max_small_bytes = self.small.size_classes.max_small_allocation_bytes();
        let retained_was_clean = !self.retained_bytes_dirty.get();
        let old_handle_capacity = self.handles.capacity();
        let old_live_handle_capacity = self.live_handles.capacity();

        // use the young fast path for small managed allocations first
        let (location, storage_retained_delta) = if bytes.len() <= max_small_bytes {
            let young_snapshot = YoungRetainedSnapshot::capture(&self.young);
            let page_arena_retained = self.page_arena.retained_bytes() as i64;

            if let Some(young_id) = self.allocate_young_bytes(bytes, reference_map_id, layout_id) {
                (
                    ManagedLocation::Young(young_id),
                    young_snapshot.retained_delta(&self.young)
                        + (self.page_arena.retained_bytes() as i64 - page_arena_retained),
                )
            } else {
                let class_index = self
                    .small
                    .size_classes
                    .class_index_for(bytes.len())
                    .expect("small managed payloads must fit one size class");

                let (slot, retained_delta) =
                    self.allocate_span_slot(class_index, bytes, reference_map_id, layout_id);
                (ManagedLocation::Small(slot), retained_delta)
            }
        } else {
            let (large_allocation_id, retained_delta) =
                self.allocate_large_allocation(bytes, reference_map_id, layout_id);
            (ManagedLocation::Large(large_allocation_id), retained_delta)
        };

        // publish stable id
        let handle = self.allocate_location(location, bytes.len(), type_id);
        self.allocated_count += 1;
        self.allocated_bytes = self.allocated_bytes.saturating_add(bytes.len() as u64);

        if retained_was_clean {
            let retained_delta = reference_map_delta
                + storage_retained_delta
                + capacity_bytes_delta::<ManagedHandleEntry>(
                    old_handle_capacity,
                    self.handles.capacity(),
                )
                + capacity_bytes_delta::<u64>(
                    old_live_handle_capacity,
                    self.live_handles.capacity(),
                );
            self.apply_retained_bytes_delta(retained_delta);
        } else {
            self.mark_retained_bytes_dirty();
        }

        handle
    }

    /// Allocate one zeroed managed byte allocation.
    pub(crate) fn allocate_zeroed(
        &mut self,
        byte_len: usize,
        reference_map: ReferenceMap,
        layout_id: Option<LayoutId>,
    ) -> ManagedReference {
        let (reference_map_id, reference_map_delta) =
            self.reference_map_table.intern(reference_map);
        self.allocate_zeroed_with_trace_id(
            byte_len,
            reference_map_id,
            layout_id,
            None,
            reference_map_delta,
        )
    }

    /// Allocate one zeroed managed byte allocation using one borrowed reference map.
    pub(crate) fn allocate_zeroed_borrowed(
        &mut self,
        byte_len: usize,
        reference_map: &ReferenceMap,
        layout_id: Option<LayoutId>,
    ) -> ManagedReference {
        let (reference_map_id, reference_map_delta) =
            self.reference_map_table.intern_borrowed(reference_map);
        self.allocate_zeroed_with_trace_id(
            byte_len,
            reference_map_id,
            layout_id,
            None,
            reference_map_delta,
        )
    }

    /// Allocate one zeroed borrowed managed byte allocation and stamp one nominal type id.
    pub(crate) fn allocate_zeroed_borrowed_typed(
        &mut self,
        byte_len: usize,
        reference_map: &ReferenceMap,
        layout_id: Option<LayoutId>,
        type_id: u32,
    ) -> ManagedReference {
        let (reference_map_id, reference_map_delta) =
            self.reference_map_table.intern_borrowed(reference_map);
        self.allocate_zeroed_with_trace_id(
            byte_len,
            reference_map_id,
            layout_id,
            Some(type_id),
            reference_map_delta,
        )
    }

    /// Allocate one zeroed repeated-offset managed byte allocation using borrowed offsets.
    pub(crate) fn allocate_zeroed_repeated_reference_offsets(
        &mut self,
        byte_len: usize,
        count: u32,
        element_size: u32,
        offsets: &[u32],
        layout_id: Option<LayoutId>,
    ) -> ManagedReference {
        let (reference_map_id, reference_map_delta) = self
            .reference_map_table
            .intern_repeated_reference_offsets(count, element_size, offsets);
        self.allocate_zeroed_with_trace_id(
            byte_len,
            reference_map_id,
            layout_id,
            None,
            reference_map_delta,
        )
    }

    /// Allocate one zeroed managed byte allocation from one interned trace id.
    fn allocate_zeroed_with_trace_id(
        &mut self,
        byte_len: usize,
        reference_map_id: ReferenceMapId,
        layout_id: Option<LayoutId>,
        type_id: Option<u32>,
        reference_map_delta: i64,
    ) -> ManagedReference {
        let max_small_bytes = self.small.size_classes.max_small_allocation_bytes();
        let retained_was_clean = !self.retained_bytes_dirty.get();
        let old_handle_capacity = self.handles.capacity();
        let old_live_handle_capacity = self.live_handles.capacity();

        // choose young, small, or large storage without staging one full temporary zero buffer
        let (location, storage_retained_delta) = if byte_len <= max_small_bytes {
            let young_snapshot = YoungRetainedSnapshot::capture(&self.young);
            let page_arena_retained = self.page_arena.retained_bytes() as i64;

            if let Some(young_id) =
                self.allocate_zeroed_young_bytes(byte_len, reference_map_id, layout_id)
            {
                (
                    ManagedLocation::Young(young_id),
                    young_snapshot.retained_delta(&self.young)
                        + (self.page_arena.retained_bytes() as i64 - page_arena_retained),
                )
            } else {
                let class_index = self
                    .small
                    .size_classes
                    .class_index_for(byte_len)
                    .expect("small managed payloads must fit one size class");

                let (slot, retained_delta) = self.allocate_zeroed_span_slot(
                    class_index,
                    byte_len,
                    reference_map_id,
                    layout_id,
                );
                (ManagedLocation::Small(slot), retained_delta)
            }
        } else {
            let (large_allocation_id, retained_delta) =
                self.allocate_zeroed_large_allocation(byte_len, reference_map_id, layout_id);
            (ManagedLocation::Large(large_allocation_id), retained_delta)
        };

        // publish stable id
        let handle = self.allocate_location(location, byte_len, type_id);
        self.allocated_count += 1;
        self.allocated_bytes = self.allocated_bytes.saturating_add(byte_len as u64);

        if retained_was_clean {
            let retained_delta = reference_map_delta
                + storage_retained_delta
                + capacity_bytes_delta::<ManagedHandleEntry>(
                    old_handle_capacity,
                    self.handles.capacity(),
                )
                + capacity_bytes_delta::<u64>(
                    old_live_handle_capacity,
                    self.live_handles.capacity(),
                );
            self.apply_retained_bytes_delta(retained_delta);
        } else {
            self.mark_retained_bytes_dirty();
        }

        handle
    }

    /// Free one managed allocation by handle.
    pub(crate) fn free(&mut self, handle: ManagedReference) -> bool {
        let Some(location) = self.location(handle) else {
            return false;
        };

        let Some(old_len) = self.byte_len(ManagedReference::new(handle.id())) else {
            return false;
        };

        // free span or large-allocation storage
        match location {
            ManagedLocation::Vacant => return false,
            ManagedLocation::Young(young_id) => {
                if !self.free_young(young_id) {
                    return false;
                }
            }
            ManagedLocation::Small(slot) => {
                let span_index = slot.span_index();
                let Some(size_class) = self
                    .small
                    .spans
                    .get(span_index)
                    .map(ManagedSpan::size_class)
                else {
                    return false;
                };
                let Some(class_index) = self.class_index_for_size_class(size_class) else {
                    return false;
                };
                let Some(span) = self.small.spans.get_mut(span_index) else {
                    return false;
                };

                if !span.free_slot(&mut self.page_arena, slot.slot_index()) {
                    return false;
                }

                if span.is_empty() {
                    span.release(&mut self.page_arena);
                    self.small.spans[span_index] = ManagedSpan::vacant();
                    self.small.free_span_ids.push(span_index);
                } else if span.has_free_slot() {
                    self.small.available_spans[class_index].push(span_index);
                }
            }
            ManagedLocation::Large(large_allocation_id) => {
                if large_allocation_id.id() == 0 {
                    return false;
                }

                let Some(large_allocation) = self
                    .large
                    .large_allocations
                    .get_mut((large_allocation_id.id() - 1) as usize)
                else {
                    return false;
                };

                large_allocation.free(&mut self.page_arena);
                self.large
                    .free_large_allocation_ids
                    .push(large_allocation_id.id());
            }
        }

        // release the managed handle entry
        let handle_id = handle.id();
        self.remove_live_handle(handle_id);
        let next_free = self.free_handle_head;
        let Some(entry) = self.handle_entry_mut(handle_id) else {
            return false;
        };
        *entry = ManagedHandleEntry::Free { next_free };
        self.free_handle_head = handle_id;
        self.allocated_count = self.allocated_count.saturating_sub(1);
        self.allocated_bytes = self.allocated_bytes.saturating_sub(old_len as u64);
        self.mark_retained_bytes_dirty();

        true
    }

    /// Report whether one managed reference is currently allocated.
    pub fn is_allocated(&self, handle: ManagedReference) -> bool {
        let Some(location) = self.location(handle) else {
            return false;
        };

        match location {
            ManagedLocation::Vacant => false,
            ManagedLocation::Young(young_id) => self.is_young_allocated(young_id),
            ManagedLocation::Small(slot) => self
                .small
                .spans
                .get(slot.span_index())
                .map(|span| span.is_occupied(slot.slot_index()))
                .unwrap_or(false),
            ManagedLocation::Large(large_allocation_id) => self
                .large_allocation(large_allocation_id)
                .map(ManagedLargeAllocation::is_allocated)
                .unwrap_or(false),
        }
    }

    /// Return the logical byte length for one managed allocation.
    pub fn byte_len(&self, handle: ManagedReference) -> Option<usize> {
        let base = ManagedReference::new(handle.id());
        let offset = handle.byte_offset();
        let handle = self.handle(base)?;

        handle.byte_len.checked_sub(offset)
    }

    /// Return the bytes for one managed allocation.
    pub fn bytes(&self, handle: ManagedReference) -> Option<Cow<'_, [u8]>> {
        let base = ManagedReference::new(handle.id());
        let offset = handle.byte_offset();
        let handle = self.handle(base)?;
        let location = self.location(base)?;

        let bytes = match location {
            ManagedLocation::Vacant => return None,
            ManagedLocation::Young(young_id) => Cow::Borrowed(self.young_bytes(young_id)?),
            ManagedLocation::Small(slot) => self.small.spans.get(slot.span_index())?.bytes(
                &self.page_arena,
                slot.slot_index(),
                handle.byte_len,
            )?,
            ManagedLocation::Large(large_allocation_id) => self
                .large_allocation(large_allocation_id)?
                .bytes(&self.page_arena),
        };

        match bytes {
            Cow::Borrowed(bytes) => Some(Cow::Borrowed(bytes.get(offset..)?)),
            Cow::Owned(bytes) => Some(Cow::Owned(bytes.get(offset..)?.to_vec())),
        }
    }

    /// Return one owned copy of the bytes for one managed allocation.
    pub fn bytes_to_vec(&self, handle: ManagedReference) -> Option<Vec<u8>> {
        Some(self.bytes(handle)?.into_owned())
    }

    /// Return one byte by offset within one managed allocation.
    pub fn byte_at(&self, handle: ManagedReference, index: usize) -> Option<u8> {
        self.bytes(handle)?.as_ref().get(index).copied()
    }

    /// Set one byte inside one managed allocation.
    pub(crate) fn set_byte(&mut self, handle: ManagedReference, index: usize, byte: u8) -> bool {
        let base = ManagedReference::new(handle.id());
        let offset = handle.byte_offset().saturating_add(index);
        let Some(byte_len) = self.handle(base).map(|handle| handle.byte_len) else {
            return false;
        };
        let Some(location) = self.location(base) else {
            return false;
        };

        match location {
            ManagedLocation::Vacant => false,
            ManagedLocation::Young(young_id) => self.set_young_byte(young_id, offset, byte),
            ManagedLocation::Small(slot) => {
                let updated = self
                    .small
                    .spans
                    .get_mut(slot.span_index())
                    .map(|span| {
                        span.set_byte(
                            &mut self.page_arena,
                            slot.slot_index(),
                            byte_len,
                            offset,
                            byte,
                        )
                    })
                    .unwrap_or(false);

                if updated {
                    self.mark_mature_allocation_range_if_may_touch_managed_edges(base, offset, 1);
                }

                updated
            }
            ManagedLocation::Large(large_allocation_id) => {
                let updated = if large_allocation_id.id() == 0 {
                    false
                } else {
                    self.large
                        .large_allocations
                        .get_mut((large_allocation_id.id() - 1) as usize)
                        .map(|large_allocation| {
                            large_allocation.set(&mut self.page_arena, offset, byte)
                        })
                        .unwrap_or(false)
                };

                if updated {
                    self.mark_mature_allocation_range_if_may_touch_managed_edges(base, offset, 1);
                }

                updated
            }
        }
    }

    /// Return the peak active-byte reservation for writing one managed byte window.
    pub(crate) fn write_active_reservation(
        &self,
        handle: ManagedReference,
        start: usize,
        len: usize,
    ) -> i64 {
        let base = ManagedReference::new(handle.id());
        let offset = handle.byte_offset().saturating_add(start);
        let Some(location) = self.location(base) else {
            return 0;
        };

        match location {
            ManagedLocation::Vacant => 0,
            ManagedLocation::Young(_) => 0,
            ManagedLocation::Small(slot) => self
                .small
                .spans
                .get(slot.span_index())
                .map(|span| {
                    let (page_count, active_reservation) =
                        span.write_active_reservation(self.page_arena.page_bytes());
                    self.page_arena
                        .allocate_pages_active_reservation(page_count)
                        + active_reservation
                })
                .unwrap_or(0),
            ManagedLocation::Large(large_allocation_id) => self
                .large_allocation(large_allocation_id)
                .map(|large_allocation| {
                    let (page_count, active_reservation) =
                        large_allocation.write_active_reservation(offset, len);
                    self.page_arena
                        .allocate_pages_active_reservation(page_count)
                        + active_reservation
                })
                .unwrap_or(0),
        }
    }

    /// Set one byte slice inside one managed allocation.
    pub(crate) fn set_bytes(
        &mut self,
        handle: ManagedReference,
        start: usize,
        bytes: &[u8],
    ) -> bool {
        let base = ManagedReference::new(handle.id());
        let offset = handle.byte_offset().saturating_add(start);
        let Some(byte_len) = self.handle(base).map(|handle| handle.byte_len) else {
            return false;
        };
        let Some(location) = self.location(base) else {
            return false;
        };

        match location {
            ManagedLocation::Vacant => false,
            ManagedLocation::Young(young_id) => self.set_young_bytes(young_id, offset, bytes),
            ManagedLocation::Small(slot) => {
                let updated = self
                    .small
                    .spans
                    .get_mut(slot.span_index())
                    .map(|span| {
                        span.set_bytes(
                            &mut self.page_arena,
                            slot.slot_index(),
                            byte_len,
                            offset,
                            bytes,
                        )
                    })
                    .unwrap_or(false);

                if updated {
                    self.mark_mature_allocation_range_if_may_touch_managed_edges(
                        base,
                        offset,
                        bytes.len(),
                    );
                }

                updated
            }
            ManagedLocation::Large(large_allocation_id) => {
                let updated = if large_allocation_id.id() == 0 {
                    false
                } else {
                    self.large
                        .large_allocations
                        .get_mut((large_allocation_id.id() - 1) as usize)
                        .map(|large_allocation| {
                            large_allocation.set_bytes(&mut self.page_arena, offset, bytes)
                        })
                        .unwrap_or(false)
                };

                if updated {
                    self.mark_mature_allocation_range_if_may_touch_managed_edges(
                        base,
                        offset,
                        bytes.len(),
                    );
                }

                updated
            }
        }
    }

    /// Return the reference map for one managed allocation.
    pub fn reference_map(&self, handle: ManagedReference) -> Option<&ReferenceMap> {
        let base = ManagedReference::new(handle.id());
        let location = self.location(base)?;
        let map_id = match location {
            ManagedLocation::Vacant => return None,
            ManagedLocation::Young(young_id) => self.young_trace_id(young_id)?,
            ManagedLocation::Small(slot) => self
                .small
                .spans
                .get(slot.span_index())?
                .trace_id(slot.slot_index())?,
            ManagedLocation::Large(large_allocation_id) => {
                self.large_allocation(large_allocation_id)?.trace_id()
            }
        };

        self.reference_map_table.get(map_id)
    }

    /// Return the layout id for one managed allocation.
    pub fn layout_id(&self, handle: ManagedReference) -> Option<LayoutId> {
        let base = ManagedReference::new(handle.id());
        let location = self.location(base)?;

        match location {
            ManagedLocation::Vacant => None,
            ManagedLocation::Young(young_id) => self.young_layout_id(young_id),
            ManagedLocation::Small(slot) => self
                .small
                .spans
                .get(slot.span_index())?
                .layout_id(slot.slot_index()),
            ManagedLocation::Large(large_allocation_id) => {
                self.large_allocation(large_allocation_id)?.layout_id()
            }
        }
    }

    /// Return the nominal type id for one managed allocation.
    pub fn type_id(&self, handle: ManagedReference) -> Option<u32> {
        let base = ManagedReference::new(handle.id());
        let handle = self.handle(base)?;
        handle.type_id
    }

    /// Set the layout id for one managed allocation.
    #[cfg(test)]
    pub(crate) fn set_layout_id(&mut self, handle: ManagedReference, layout_id: LayoutId) -> bool {
        let base = ManagedReference::new(handle.id());
        let Some(location) = self.location(base) else {
            return false;
        };

        let updated = match location {
            ManagedLocation::Vacant => false,
            ManagedLocation::Young(young_id) => self.set_young_layout_id(young_id, layout_id),
            ManagedLocation::Small(slot) => self
                .small
                .spans
                .get_mut(slot.span_index())
                .map(|span| span.set_layout_id(&mut self.page_arena, slot.slot_index(), layout_id))
                .unwrap_or(false),
            ManagedLocation::Large(large_allocation_id) => self
                .large_allocation_mut(large_allocation_id)
                .map(|large_allocation| {
                    large_allocation.set_layout_id(layout_id);
                    true
                })
                .unwrap_or(false),
        };

        if updated {
            self.mark_retained_bytes_dirty();
        }

        updated
    }

    /// Set the nominal type id for one managed allocation.
    pub(crate) fn set_type_id(&mut self, handle: ManagedReference, type_id: u32) -> bool {
        let base = ManagedReference::new(handle.id());
        let Some(entry) = self.handle_mut(base) else {
            return false;
        };

        entry.type_id = Some(type_id);
        true
    }

    /// Pin one managed allocation for raw exposure.
    #[cfg(test)]
    pub(crate) fn pin(&mut self, handle: ManagedReference) -> bool {
        let base = ManagedReference::new(handle.id());
        let Some(location) = self.location(base) else {
            return false;
        };

        let updated = match location {
            ManagedLocation::Vacant => false,
            ManagedLocation::Young(_) => {
                let promoted = self.drain_young_handle_to_mature(base);
                let Some(location) = promoted.and_then(|handle| self.location(handle)) else {
                    return false;
                };

                match location {
                    ManagedLocation::Vacant => false,
                    ManagedLocation::Young(_) => false,
                    ManagedLocation::Small(slot) => self
                        .small
                        .spans
                        .get_mut(slot.span_index())
                        .map(|span| span.pin(slot.slot_index()))
                        .unwrap_or(false),
                    ManagedLocation::Large(large_allocation_id) => self
                        .large_allocation_mut(large_allocation_id)
                        .map(ManagedLargeAllocation::pin)
                        .unwrap_or(false),
                }
            }
            ManagedLocation::Small(slot) => self
                .small
                .spans
                .get_mut(slot.span_index())
                .map(|span| span.pin(slot.slot_index()))
                .unwrap_or(false),
            ManagedLocation::Large(large_allocation_id) => self
                .large_allocation_mut(large_allocation_id)
                .map(ManagedLargeAllocation::pin)
                .unwrap_or(false),
        };

        if updated {
            self.mark_retained_bytes_dirty();
        }

        updated
    }

    /// Release one managed allocation pin.
    #[cfg(test)]
    pub(crate) fn unpin(&mut self, handle: ManagedReference) -> bool {
        let Some(location) = self.location(ManagedReference::new(handle.id())) else {
            return false;
        };

        let updated = match location {
            ManagedLocation::Vacant => false,
            ManagedLocation::Young(_) => false,
            ManagedLocation::Small(slot) => self
                .small
                .spans
                .get_mut(slot.span_index())
                .map(|span| span.unpin(slot.slot_index()))
                .unwrap_or(false),
            ManagedLocation::Large(large_allocation_id) => self
                .large_allocation_mut(large_allocation_id)
                .map(ManagedLargeAllocation::unpin)
                .unwrap_or(false),
        };

        if updated {
            self.mark_retained_bytes_dirty();
        }

        updated
    }

    // handle table helpers

    fn allocate_location(
        &mut self,
        location: ManagedLocation,
        byte_len: usize,
        type_id: Option<u32>,
    ) -> ManagedReference {
        let live_index = self.live_handles.len() as u32;
        let handle = ManagedHandle {
            location,
            byte_len,
            type_id,
            live_index,
            flags: 0,
        };

        if self.free_handle_head != 0 {
            let id = self.free_handle_head;
            let next_free = match self.handle_entry(id) {
                Some(ManagedHandleEntry::Free { next_free }) => *next_free,
                Some(ManagedHandleEntry::Live(_)) => {
                    unreachable!("free handle head must point at one free handle entry")
                }
                None => unreachable!("free handle head must remain addressable"),
            };

            self.free_handle_head = next_free;

            let Some(entry) = self.handle_entry_mut(id) else {
                unreachable!("free handle head must remain mutable");
            };
            *entry = ManagedHandleEntry::Live(handle);
            self.live_handles.push(id);

            return ManagedReference::new(id);
        }

        let id = self.next_unused_id;
        self.next_unused_id = self.next_unused_id.saturating_add(1);
        self.handles.push(ManagedHandleEntry::Live(handle));
        self.live_handles.push(id);

        ManagedReference::new(id)
    }

    pub(crate) fn location(&self, handle: ManagedReference) -> Option<ManagedLocation> {
        Some(self.handle(handle)?.location)
    }

    pub(crate) fn handle(&self, handle: ManagedReference) -> Option<&ManagedHandle> {
        match self.handle_entry(handle.id())? {
            ManagedHandleEntry::Live(handle) => Some(handle),
            ManagedHandleEntry::Free { .. } => None,
        }
    }

    pub(crate) fn handle_mut(&mut self, handle: ManagedReference) -> Option<&mut ManagedHandle> {
        match self.handle_entry_mut(handle.id())? {
            ManagedHandleEntry::Live(handle) => Some(handle),
            ManagedHandleEntry::Free { .. } => None,
        }
    }

    pub(crate) fn handle_entry(&self, id: u64) -> Option<&ManagedHandleEntry> {
        if id == 0 {
            return None;
        }

        self.handles.get((id - 1) as usize)
    }

    pub(crate) fn handle_entry_mut(&mut self, id: u64) -> Option<&mut ManagedHandleEntry> {
        if id == 0 {
            return None;
        }

        self.handles.get_mut((id - 1) as usize)
    }

    pub(crate) fn is_young_allocated(&self, young_id: ManagedYoungId) -> bool {
        self.young.is_allocated(young_id)
            || self
                .young_from
                .as_ref()
                .map(|space| space.is_allocated(young_id))
                .unwrap_or(false)
    }

    pub(crate) fn young_byte_len(&self, young_id: ManagedYoungId) -> Option<usize> {
        self.young
            .byte_len(young_id)
            .or_else(|| self.young_from.as_ref()?.byte_len(young_id))
    }

    pub(crate) fn young_bytes(&self, young_id: ManagedYoungId) -> Option<&[u8]> {
        self.young
            .bytes(young_id, &self.page_arena)
            .or_else(|| self.young_from.as_ref()?.bytes(young_id, &self.page_arena))
    }

    pub(crate) fn young_trace_id(&self, young_id: ManagedYoungId) -> Option<ReferenceMapId> {
        self.young
            .trace_id(young_id)
            .or_else(|| self.young_from.as_ref()?.trace_id(young_id))
    }

    pub(crate) fn young_layout_id(&self, young_id: ManagedYoungId) -> Option<LayoutId> {
        self.young
            .layout_id(young_id)
            .or_else(|| self.young_from.as_ref()?.layout_id(young_id))
    }

    pub(crate) fn young_age(&self, young_id: ManagedYoungId) -> Option<u8> {
        self.young
            .age(young_id)
            .or_else(|| self.young_from.as_ref()?.age(young_id))
    }

    #[cfg(test)]
    pub(crate) fn set_young_layout_id(
        &mut self,
        young_id: ManagedYoungId,
        layout_id: LayoutId,
    ) -> bool {
        if self.young.set_layout_id(young_id, layout_id) {
            return true;
        }

        self.young_from
            .as_mut()
            .map(|space| space.set_layout_id(young_id, layout_id))
            .unwrap_or(false)
    }

    pub(crate) fn set_young_byte(
        &mut self,
        young_id: ManagedYoungId,
        offset: usize,
        byte: u8,
    ) -> bool {
        if self
            .young
            .set_byte(young_id, offset, byte, &mut self.page_arena)
        {
            return true;
        }

        self.young_from
            .as_mut()
            .map(|space| space.set_byte(young_id, offset, byte, &mut self.page_arena))
            .unwrap_or(false)
    }

    pub(crate) fn set_young_bytes(
        &mut self,
        young_id: ManagedYoungId,
        offset: usize,
        bytes: &[u8],
    ) -> bool {
        if self
            .young
            .set_bytes(young_id, offset, bytes, &mut self.page_arena)
        {
            return true;
        }

        self.young_from
            .as_mut()
            .map(|space| space.set_bytes(young_id, offset, bytes, &mut self.page_arena))
            .unwrap_or(false)
    }

    pub(crate) fn free_young(&mut self, young_id: ManagedYoungId) -> bool {
        if self.young.free(young_id) {
            return true;
        }

        self.young_from
            .as_mut()
            .map(|space| space.free(young_id))
            .unwrap_or(false)
    }

    pub(crate) fn mark_mature_allocation_dirty(&mut self, handle: ManagedReference) {
        let base = ManagedReference::new(handle.id());
        let Some(location) = self.location(base) else {
            return;
        };

        let Some(byte_len) = self.handle(base).map(|handle| handle.byte_len) else {
            return;
        };

        match location {
            ManagedLocation::Small(slot) => {
                let Some(span) = self.small.spans.get_mut(slot.span_index()) else {
                    return;
                };

                let marked_any = span.mark_dirty_slot_range(slot.slot_index(), 0, byte_len);
                if marked_any || span.has_dirty_cards() {
                    self.enqueue_dirty_span(slot.span_index());
                }
            }
            ManagedLocation::Large(large_allocation_id) => {
                let Some(large_allocation) = self.large_allocation_mut(large_allocation_id) else {
                    return;
                };

                let marked_any = large_allocation.mark_dirty_range(0, byte_len);
                if marked_any || large_allocation.has_dirty_cards() {
                    self.enqueue_dirty_large_allocation(large_allocation_id);
                }
            }
            ManagedLocation::Vacant | ManagedLocation::Young(_) => {}
        }
    }

    fn mark_mature_allocation_range_if_may_touch_managed_edges(
        &mut self,
        handle: ManagedReference,
        start: usize,
        len: usize,
    ) {
        let Some(reference_map) = self.reference_map(handle) else {
            return;
        };

        if !reference_map.touches_managed_range(start, len, self.managed_reference_bytes) {
            return;
        }

        let base = ManagedReference::new(handle.id());
        let Some(location) = self.location(base) else {
            return;
        };

        match location {
            ManagedLocation::Small(slot) => {
                let Some(span) = self.small.spans.get_mut(slot.span_index()) else {
                    return;
                };

                let marked_any = span.mark_dirty_slot_range(slot.slot_index(), start, len);
                if marked_any || span.has_dirty_cards() {
                    self.enqueue_dirty_span(slot.span_index());
                }
            }
            ManagedLocation::Large(large_allocation_id) => {
                let Some(large_allocation) = self.large_allocation_mut(large_allocation_id) else {
                    return;
                };

                let marked_any = large_allocation.mark_dirty_range(start, len);
                if marked_any || large_allocation.has_dirty_cards() {
                    self.enqueue_dirty_large_allocation(large_allocation_id);
                }
            }
            ManagedLocation::Vacant | ManagedLocation::Young(_) => {}
        }
    }

    fn allocate_span_slot(
        &mut self,
        class_index: usize,
        bytes: &[u8],
        trace_id: ReferenceMapId,
        layout_id: Option<LayoutId>,
    ) -> (ManagedSpanSlot, i64) {
        let page_bytes = self.page_arena.page_bytes();

        // try one reusable span first
        while let Some(span_index) = self.small.available_spans[class_index].pop() {
            let Some(span) = self.small.spans.get_mut(span_index) else {
                continue;
            };

            if !span.has_free_slot() {
                continue;
            }

            let Some(slot_index) = span.first_free_slot() else {
                continue;
            };

            let retained_before = span.active_bytes(page_bytes) as i64;
            let page_arena_retained = self.page_arena.retained_bytes() as i64;
            if span.allocate_slot(&mut self.page_arena, slot_index, bytes, trace_id, layout_id) {
                let retained_after = span.active_bytes(page_bytes) as i64;
                let mut retained_delta = retained_after - retained_before
                    + (self.page_arena.retained_bytes() as i64 - page_arena_retained);

                if span.has_free_slot() {
                    let queue_capacity = self.small.available_spans[class_index].capacity();
                    self.small.available_spans[class_index].push(span_index);
                    retained_delta += capacity_bytes_delta::<usize>(
                        queue_capacity,
                        self.small.available_spans[class_index].capacity(),
                    );
                }

                return (ManagedSpanSlot::new(span_index, slot_index), retained_delta);
            }
        }

        // otherwise allocate a fresh span
        let size_class = self.small.size_classes.classes[class_index].bytes;
        let spans_capacity = self.small.spans.capacity();
        let queue_capacity = self.small.available_spans[class_index].capacity();
        let page_arena_retained = self.page_arena.retained_bytes() as i64;
        let mut span = ManagedSpan::new(size_class, self.small.span_bytes, &mut self.page_arena);
        let slot_index = span
            .first_free_slot()
            .unwrap_or_else(|| panic!("fresh managed span must have one free slot"));
        let allocated =
            span.allocate_slot(&mut self.page_arena, slot_index, bytes, trace_id, layout_id);
        debug_assert!(allocated, "fresh managed span must accept its first slot");
        let span_retained = span.active_bytes(page_bytes) as i64;

        let span_index = if let Some(index) = self.small.free_span_ids.pop() {
            self.small.spans[index] = span;
            index
        } else {
            self.small.spans.push(span);
            self.small.spans.len() - 1
        };

        if self.small.spans[span_index].has_free_slot() {
            self.small.available_spans[class_index].push(span_index);
        }

        let retained_delta = span_retained
            + capacity_bytes_delta::<ManagedSpan>(spans_capacity, self.small.spans.capacity())
            + capacity_bytes_delta::<usize>(
                queue_capacity,
                self.small.available_spans[class_index].capacity(),
            )
            + (self.page_arena.retained_bytes() as i64 - page_arena_retained);

        (ManagedSpanSlot::new(span_index, slot_index), retained_delta)
    }

    fn allocate_zeroed_span_slot(
        &mut self,
        class_index: usize,
        byte_len: usize,
        trace_id: ReferenceMapId,
        layout_id: Option<LayoutId>,
    ) -> (ManagedSpanSlot, i64) {
        let page_bytes = self.page_arena.page_bytes();

        // try one reusable span first
        while let Some(span_index) = self.small.available_spans[class_index].pop() {
            let Some(span) = self.small.spans.get_mut(span_index) else {
                continue;
            };

            if !span.has_free_slot() {
                continue;
            }

            let Some(slot_index) = span.first_free_slot() else {
                continue;
            };

            let retained_before = span.active_bytes(page_bytes) as i64;
            let page_arena_retained = self.page_arena.retained_bytes() as i64;
            if span.allocate_zeroed_slot(
                &mut self.page_arena,
                slot_index,
                byte_len,
                trace_id,
                layout_id,
            ) {
                let retained_after = span.active_bytes(page_bytes) as i64;
                let mut retained_delta = retained_after - retained_before
                    + (self.page_arena.retained_bytes() as i64 - page_arena_retained);

                if span.has_free_slot() {
                    let queue_capacity = self.small.available_spans[class_index].capacity();
                    self.small.available_spans[class_index].push(span_index);
                    retained_delta += capacity_bytes_delta::<usize>(
                        queue_capacity,
                        self.small.available_spans[class_index].capacity(),
                    );
                }

                return (ManagedSpanSlot::new(span_index, slot_index), retained_delta);
            }
        }

        // otherwise allocate a fresh span
        let size_class = self.small.size_classes.classes[class_index].bytes;
        let spans_capacity = self.small.spans.capacity();
        let queue_capacity = self.small.available_spans[class_index].capacity();
        let page_arena_retained = self.page_arena.retained_bytes() as i64;
        let mut span = ManagedSpan::new(size_class, self.small.span_bytes, &mut self.page_arena);
        let slot_index = span
            .first_free_slot()
            .unwrap_or_else(|| panic!("fresh managed span must have one free slot"));
        let allocated = span.allocate_zeroed_slot(
            &mut self.page_arena,
            slot_index,
            byte_len,
            trace_id,
            layout_id,
        );
        debug_assert!(allocated, "fresh managed span must accept its first slot");
        let span_retained = span.active_bytes(page_bytes) as i64;

        let span_index = if let Some(index) = self.small.free_span_ids.pop() {
            self.small.spans[index] = span;
            index
        } else {
            self.small.spans.push(span);
            self.small.spans.len() - 1
        };

        if self.small.spans[span_index].has_free_slot() {
            self.small.available_spans[class_index].push(span_index);
        }

        let retained_delta = span_retained
            + capacity_bytes_delta::<ManagedSpan>(spans_capacity, self.small.spans.capacity())
            + capacity_bytes_delta::<usize>(
                queue_capacity,
                self.small.available_spans[class_index].capacity(),
            )
            + (self.page_arena.retained_bytes() as i64 - page_arena_retained);

        (ManagedSpanSlot::new(span_index, slot_index), retained_delta)
    }

    fn allocate_large_allocation(
        &mut self,
        bytes: &[u8],
        trace_id: ReferenceMapId,
        layout_id: Option<LayoutId>,
    ) -> (ManagedLargeAllocationId, i64) {
        if let Some(id) = self.large.free_large_allocation_ids.pop() {
            let large_allocation_id = ManagedLargeAllocationId::new(id);
            let page_bytes = self.large.page_bytes;
            let page_arena_retained = self.page_arena.retained_bytes() as i64;
            let retained_before = self
                .large
                .large_allocations
                .get((large_allocation_id.id() - 1) as usize)
                .map(ManagedLargeAllocation::active_bytes)
                .unwrap_or(0) as i64;
            if let Some(large_allocation) = self
                .large
                .large_allocations
                .get_mut((large_allocation_id.id() - 1) as usize)
            {
                large_allocation.replace(
                    bytes,
                    page_bytes,
                    trace_id,
                    layout_id,
                    &mut self.page_arena,
                );
            }

            let retained_after = self
                .large
                .large_allocations
                .get((large_allocation_id.id() - 1) as usize)
                .map(ManagedLargeAllocation::active_bytes)
                .unwrap_or(0) as i64;
            let retained_delta = retained_after - retained_before
                + (self.page_arena.retained_bytes() as i64 - page_arena_retained);

            return (large_allocation_id, retained_delta);
        }

        let large_allocation_id =
            ManagedLargeAllocationId::new(self.large.next_unused_large_allocation_id);
        self.large.next_unused_large_allocation_id =
            self.large.next_unused_large_allocation_id.saturating_add(1);
        let large_capacity = self.large.large_allocations.capacity();
        let page_arena_retained = self.page_arena.retained_bytes() as i64;
        self.large
            .large_allocations
            .push(ManagedLargeAllocation::new(
                bytes,
                self.large.page_bytes,
                trace_id,
                layout_id,
                &mut self.page_arena,
            ));
        let retained_delta = self
            .large
            .large_allocations
            .last()
            .map(ManagedLargeAllocation::active_bytes)
            .unwrap_or(0) as i64
            + capacity_bytes_delta::<ManagedLargeAllocation>(
                large_capacity,
                self.large.large_allocations.capacity(),
            )
            + (self.page_arena.retained_bytes() as i64 - page_arena_retained);

        (large_allocation_id, retained_delta)
    }

    fn allocate_zeroed_large_allocation(
        &mut self,
        byte_len: usize,
        trace_id: ReferenceMapId,
        layout_id: Option<LayoutId>,
    ) -> (ManagedLargeAllocationId, i64) {
        if let Some(id) = self.large.free_large_allocation_ids.pop() {
            let large_allocation_id = ManagedLargeAllocationId::new(id);
            let page_bytes = self.large.page_bytes;
            let page_arena_retained = self.page_arena.retained_bytes() as i64;
            let retained_before = self
                .large
                .large_allocations
                .get((large_allocation_id.id() - 1) as usize)
                .map(ManagedLargeAllocation::active_bytes)
                .unwrap_or(0) as i64;
            if let Some(large_allocation) = self
                .large
                .large_allocations
                .get_mut((large_allocation_id.id() - 1) as usize)
            {
                large_allocation.replace_zeroed(
                    byte_len,
                    page_bytes,
                    trace_id,
                    layout_id,
                    &mut self.page_arena,
                );
            }

            let retained_after = self
                .large
                .large_allocations
                .get((large_allocation_id.id() - 1) as usize)
                .map(ManagedLargeAllocation::active_bytes)
                .unwrap_or(0) as i64;
            let retained_delta = retained_after - retained_before
                + (self.page_arena.retained_bytes() as i64 - page_arena_retained);

            return (large_allocation_id, retained_delta);
        }

        let large_allocation_id =
            ManagedLargeAllocationId::new(self.large.next_unused_large_allocation_id);
        self.large.next_unused_large_allocation_id =
            self.large.next_unused_large_allocation_id.saturating_add(1);
        let large_capacity = self.large.large_allocations.capacity();
        let page_arena_retained = self.page_arena.retained_bytes() as i64;
        self.large
            .large_allocations
            .push(ManagedLargeAllocation::new_zeroed(
                byte_len,
                self.large.page_bytes,
                trace_id,
                layout_id,
                &mut self.page_arena,
            ));
        let retained_delta = self
            .large
            .large_allocations
            .last()
            .map(ManagedLargeAllocation::active_bytes)
            .unwrap_or(0) as i64
            + capacity_bytes_delta::<ManagedLargeAllocation>(
                large_capacity,
                self.large.large_allocations.capacity(),
            )
            + (self.page_arena.retained_bytes() as i64 - page_arena_retained);

        (large_allocation_id, retained_delta)
    }

    pub(crate) fn large_allocation(
        &self,
        large_allocation_id: ManagedLargeAllocationId,
    ) -> Option<&ManagedLargeAllocation> {
        if large_allocation_id.id() == 0 {
            return None;
        }

        self.large
            .large_allocations
            .get((large_allocation_id.id() - 1) as usize)
    }

    pub(crate) fn large_allocation_mut(
        &mut self,
        large_allocation_id: ManagedLargeAllocationId,
    ) -> Option<&mut ManagedLargeAllocation> {
        if large_allocation_id.id() == 0 {
            return None;
        }

        self.large
            .large_allocations
            .get_mut((large_allocation_id.id() - 1) as usize)
    }

    fn class_index_for_size_class(&self, size_class: usize) -> Option<usize> {
        self.small
            .size_classes
            .classes
            .iter()
            .position(|class| class.bytes == size_class)
    }

    pub(crate) fn allocate_mature_location(
        &mut self,
        bytes: &[u8],
        trace_id: ReferenceMapId,
        layout_id: Option<LayoutId>,
    ) -> ManagedLocation {
        if let Some(class_index) = self.small.size_classes.class_index_for(bytes.len()) {
            let (slot, _) = self.allocate_span_slot(class_index, bytes, trace_id, layout_id);
            ManagedLocation::Small(slot)
        } else {
            let (large_allocation_id, _) =
                self.allocate_large_allocation(bytes, trace_id, layout_id);
            ManagedLocation::Large(large_allocation_id)
        }
    }

    pub(crate) fn drain_young_handle_to_mature(
        &mut self,
        handle: ManagedReference,
    ) -> Option<ManagedReference> {
        let base = ManagedReference::new(handle.id());
        let ManagedLocation::Young(young_id) = self.location(base)? else {
            return Some(base);
        };
        let trace_id = self.young_trace_id(young_id)?;
        let layout_id = self.young_layout_id(young_id);
        let bytes = self.young_bytes(young_id)?.to_vec();
        let location = self.allocate_mature_location(&bytes, trace_id, layout_id);

        let _ = self.free_young(young_id);

        let entry = self.handle_mut(base)?;
        entry.location = location;

        Some(base)
    }

    pub(crate) fn drain_young_to_mature(&mut self) {
        if self.young.is_empty() {
            self.young.reset(&mut self.page_arena);
            return;
        }

        let mut drained_any = false;
        let live_handles = self.live_handles.clone();

        // drain live young handles before durable capture
        for id in live_handles {
            let handle = ManagedReference::new(id);

            if !matches!(self.location(handle), Some(ManagedLocation::Young(_))) {
                continue;
            }

            if self.drain_young_handle_to_mature(handle).is_some() {
                drained_any = true;
            }
        }

        if self.young.is_empty() {
            self.young.reset(&mut self.page_arena);
        }

        if drained_any {
            self.mark_retained_bytes_dirty();
        }
    }

    pub(crate) fn free_storage(&mut self, location: ManagedLocation) {
        match location {
            ManagedLocation::Vacant => {}
            ManagedLocation::Young(young_id) => {
                let _ = self.free_young(young_id);
            }
            ManagedLocation::Small(slot) => {
                let span_index = slot.span_index();
                let Some(size_class) = self
                    .small
                    .spans
                    .get(span_index)
                    .map(ManagedSpan::size_class)
                else {
                    return;
                };
                let Some(class_index) = self.class_index_for_size_class(size_class) else {
                    return;
                };
                let Some(span) = self.small.spans.get_mut(span_index) else {
                    return;
                };

                if !span.free_slot(&mut self.page_arena, slot.slot_index()) {
                    return;
                }

                if span.is_empty() {
                    span.release(&mut self.page_arena);
                    self.small.spans[span_index] = ManagedSpan::vacant();
                    self.small.free_span_ids.push(span_index);
                } else if span.has_free_slot() {
                    self.small.available_spans[class_index].push(span_index);
                }
            }
            ManagedLocation::Large(large_allocation_id) => {
                if large_allocation_id.id() == 0 {
                    return;
                }

                let Some(large_allocation) = self
                    .large
                    .large_allocations
                    .get_mut((large_allocation_id.id() - 1) as usize)
                else {
                    return;
                };

                large_allocation.free(&mut self.page_arena);
                self.large
                    .free_large_allocation_ids
                    .push(large_allocation_id.id());
            }
        }
    }

    fn active_pins(&self) -> usize {
        let span_pins = self
            .small
            .spans
            .iter()
            .map(ManagedSpan::active_pins)
            .sum::<usize>();
        let large_allocation_pins = self
            .large
            .large_allocations
            .iter()
            .map(ManagedLargeAllocation::active_pins)
            .sum::<usize>();

        span_pins.saturating_add(large_allocation_pins)
    }

    fn rebuild_span_directories(&mut self) {
        self.small.available_spans = vec![Vec::new(); self.small.size_classes.classes.len()];
        self.small.free_span_ids.clear();

        // rebuild reusable span directories
        for (index, span) in self.small.spans.iter().enumerate() {
            if span.is_vacant() {
                self.small.free_span_ids.push(index);
                continue;
            }

            if !span.has_free_slot() {
                continue;
            }

            if let Some(class_index) = self.class_index_for_size_class(span.size_class()) {
                self.small.available_spans[class_index].push(index);
            }
        }
    }

    fn rebuild_live_handles(&mut self) {
        self.live_handles.clear();

        for (index, entry) in self.handles.iter_mut().enumerate() {
            let ManagedHandleEntry::Live(handle) = entry else {
                continue;
            };

            handle.live_index = self.live_handles.len() as u32;
            self.live_handles
                .push(index as u64 + FIRST_ALLOCATED_REFERENCE_ID);
        }
    }

    pub(crate) fn remove_live_handle(&mut self, handle_id: u64) {
        let Some(live_index) = self
            .handle(ManagedReference::new(handle_id))
            .map(|handle| handle.live_index as usize)
        else {
            return;
        };

        let removed = self.live_handles.swap_remove(live_index);
        debug_assert_eq!(removed, handle_id, "live handle index must match handle id");

        if live_index >= self.live_handles.len() {
            return;
        }

        let moved_handle_id = self.live_handles[live_index];
        let Some(moved_handle) = self.handle_mut(ManagedReference::new(moved_handle_id)) else {
            return;
        };
        moved_handle.live_index = live_index as u32;
    }

    /// Return the active-byte reservation for allocating one managed payload.
    pub(crate) fn allocate_bytes_active_reservation(
        &self,
        byte_len: usize,
        reference_map: &ReferenceMap,
        layout_id: Option<LayoutId>,
    ) -> i64 {
        let reference_map_delta = self
            .reference_map_table
            .intern_borrowed_delta(reference_map);
        let storage_delta =
            self.allocate_storage_active_reservation(byte_len, reference_map, layout_id);

        reference_map_delta + storage_delta + self.allocate_handle_active_reservation()
    }

    /// Return the active-byte reservation for allocating one repeated-offset managed payload.
    pub(crate) fn allocate_repeated_offsets_active_reservation(
        &self,
        byte_len: usize,
        count: u32,
        element_size: u32,
        offsets: &[u32],
        layout_id: Option<LayoutId>,
    ) -> i64 {
        let reference_map_delta = self
            .reference_map_table
            .intern_repeated_reference_offsets_delta(count, element_size, offsets);
        let reference_map = ReferenceMap::RepeatedReferenceOffsets {
            count,
            element_size,
            offsets: offsets.to_vec(),
        };
        let storage_delta =
            self.allocate_storage_active_reservation(byte_len, &reference_map, layout_id);

        reference_map_delta + storage_delta + self.allocate_handle_active_reservation()
    }

    /// Return the active-byte reservation for one managed storage allocation.
    fn allocate_storage_active_reservation(
        &self,
        byte_len: usize,
        reference_map: &ReferenceMap,
        layout_id: Option<LayoutId>,
    ) -> i64 {
        let max_small_bytes = self.small.size_classes.max_small_allocation_bytes();

        if byte_len <= max_small_bytes {
            if let Some((new_page_count, young_delta)) =
                self.young.allocate_retained_delta(byte_len)
            {
                let page_arena_delta = self
                    .page_arena
                    .allocate_pages_active_reservation(new_page_count);

                return young_delta + page_arena_delta;
            }

            let class_index = self
                .small
                .size_classes
                .class_index_for(byte_len)
                .expect("small managed payloads must fit one size class");

            return self.allocate_span_slot_active_reservation(
                class_index,
                reference_map,
                layout_id,
            );
        }

        self.allocate_large_allocation_active_reservation(byte_len)
    }

    /// Return the active-byte reservation for one managed handle publication.
    fn allocate_handle_active_reservation(&self) -> i64 {
        let live_capacity =
            projected_vec_capacity::<u64>(self.live_handles.len(), self.live_handles.capacity(), 1);
        let live_delta =
            vec_capacity_bytes_delta::<u64>(self.live_handles.capacity(), live_capacity);

        if self.free_handle_head != 0 {
            return live_delta;
        }

        let handle_capacity = projected_vec_capacity::<ManagedHandleEntry>(
            self.handles.len(),
            self.handles.capacity(),
            1,
        );

        live_delta
            + vec_capacity_bytes_delta::<ManagedHandleEntry>(
                self.handles.capacity(),
                handle_capacity,
            )
    }

    /// Return the active-byte reservation for allocating one managed span slot.
    fn allocate_span_slot_active_reservation(
        &self,
        class_index: usize,
        reference_map: &ReferenceMap,
        layout_id: Option<LayoutId>,
    ) -> i64 {
        let page_bytes = self.page_arena.page_bytes();
        let trace_id = self.reference_map_table.projected_id(reference_map);

        for &span_index in self.small.available_spans[class_index].iter().rev() {
            let Some(span) = self.small.spans.get(span_index) else {
                continue;
            };

            if !span.has_free_slot() {
                continue;
            }

            let span_delta = span.allocate_active_reservation(page_bytes, trace_id, layout_id);
            let page_count = self.small.span_bytes.div_ceil(page_bytes).max(1);
            let page_arena_delta = if span_delta != 0 {
                self.page_arena
                    .allocate_pages_active_reservation(page_count)
            } else {
                0
            };

            return span_delta + page_arena_delta;
        }

        let size_class = self.small.size_classes.classes[class_index].bytes;
        let span_delta =
            ManagedSpan::active_bytes_for_new(size_class, self.small.span_bytes, page_bytes) as i64;
        let spans_delta = if self.small.free_span_ids.is_empty() {
            let capacity = projected_vec_capacity::<ManagedSpan>(
                self.small.spans.len(),
                self.small.spans.capacity(),
                1,
            );
            vec_capacity_bytes_delta::<ManagedSpan>(self.small.spans.capacity(), capacity)
        } else {
            0
        };
        let slot_count = (self.small.span_bytes / size_class).max(1);
        let queue_delta = if slot_count > 1 {
            let queue = &self.small.available_spans[class_index];
            let capacity = projected_vec_capacity::<usize>(queue.len(), queue.capacity(), 1);
            vec_capacity_bytes_delta::<usize>(queue.capacity(), capacity)
        } else {
            0
        };
        let page_count = self.small.span_bytes.div_ceil(page_bytes).max(1);
        let page_arena_delta = self
            .page_arena
            .allocate_pages_active_reservation(page_count);

        span_delta + spans_delta + queue_delta + page_arena_delta
    }

    /// Return the active-byte reservation for allocating one managed large allocation.
    fn allocate_large_allocation_active_reservation(&self, byte_len: usize) -> i64 {
        let page_bytes = self.large.page_bytes;
        let page_count = byte_len.div_ceil(page_bytes).max((byte_len > 0) as usize);
        let page_arena_delta = self
            .page_arena
            .allocate_pages_active_reservation(page_count);
        let payload_retained = ChunkPayload::active_bytes_for_len(byte_len, page_bytes);

        if let Some(id) = self.large.free_large_allocation_ids.last().copied() {
            let retained_before = self
                .large
                .large_allocations
                .get((id - 1) as usize)
                .map(ManagedLargeAllocation::active_bytes)
                .unwrap_or(0);

            return payload_retained as i64 - retained_before as i64 + page_arena_delta;
        }

        let large_capacity = projected_vec_capacity::<ManagedLargeAllocation>(
            self.large.large_allocations.len(),
            self.large.large_allocations.capacity(),
            1,
        );
        let large_delta = vec_capacity_bytes_delta::<ManagedLargeAllocation>(
            self.large.large_allocations.capacity(),
            large_capacity,
        );

        payload_retained as i64 + large_delta + page_arena_delta
    }

    pub(crate) fn enqueue_dirty_span(&mut self, span_index: usize) {
        let retained_was_clean = !self.retained_bytes_dirty.get();
        let Some(span) = self.small.spans.get_mut(span_index) else {
            return;
        };
        if span.is_dirty_queued() || !span.has_dirty_cards() {
            return;
        }

        span.set_dirty_queued(true);

        if self.gc_state.phase == super::GcPhase::Idle {
            let queue_capacity = self.dirty_spans.capacity();
            self.dirty_spans.push(span_index);
            if retained_was_clean {
                self.apply_retained_bytes_delta(capacity_bytes_delta::<usize>(
                    queue_capacity,
                    self.dirty_spans.capacity(),
                ));
            } else {
                self.mark_retained_bytes_dirty();
            }
        } else {
            let queue_capacity = self.dirty_next_spans.capacity();
            self.dirty_next_spans.push(span_index);
            if retained_was_clean {
                self.apply_retained_bytes_delta(capacity_bytes_delta::<usize>(
                    queue_capacity,
                    self.dirty_next_spans.capacity(),
                ));
            } else {
                self.mark_retained_bytes_dirty();
            }
        }
    }

    pub(crate) fn enqueue_dirty_large_allocation(
        &mut self,
        large_allocation_id: ManagedLargeAllocationId,
    ) {
        let retained_was_clean = !self.retained_bytes_dirty.get();
        let Some(large_allocation) = self.large_allocation_mut(large_allocation_id) else {
            return;
        };
        if large_allocation.is_dirty_queued() || !large_allocation.has_dirty_cards() {
            return;
        }

        large_allocation.set_dirty_queued(true);

        if self.gc_state.phase == super::GcPhase::Idle {
            let queue_capacity = self.dirty_large_allocations.capacity();
            self.dirty_large_allocations.push(large_allocation_id);
            if retained_was_clean {
                self.apply_retained_bytes_delta(capacity_bytes_delta::<ManagedLargeAllocationId>(
                    queue_capacity,
                    self.dirty_large_allocations.capacity(),
                ));
            } else {
                self.mark_retained_bytes_dirty();
            }
        } else {
            let queue_capacity = self.dirty_next_large_allocations.capacity();
            self.dirty_next_large_allocations.push(large_allocation_id);
            if retained_was_clean {
                self.apply_retained_bytes_delta(capacity_bytes_delta::<ManagedLargeAllocationId>(
                    queue_capacity,
                    self.dirty_next_large_allocations.capacity(),
                ));
            } else {
                self.mark_retained_bytes_dirty();
            }
        }
    }

    pub(crate) fn mark_retained_bytes_dirty(&self) {
        self.retained_bytes_dirty.set(true);
    }

    pub(crate) fn apply_retained_bytes_delta(&self, delta: i64) {
        let retained_bytes = self.retained_bytes.get();
        let retained_bytes = if delta >= 0 {
            retained_bytes.saturating_add(delta as u64)
        } else {
            retained_bytes.saturating_sub(delta.unsigned_abs())
        };

        self.retained_bytes.set(retained_bytes);
        self.retained_bytes_dirty.set(false);

        self.debug_assert_retained_bytes();
    }

    pub(crate) fn refresh_retained_bytes(&self) {
        if !self.retained_bytes_dirty.get() {
            return;
        }

        self.retained_bytes.set(self.exact_retained_bytes());
        self.retained_bytes_dirty.set(false);
    }

    pub(crate) fn recompute_retained_bytes(&self) {
        self.retained_bytes_dirty.set(true);
        self.refresh_retained_bytes();
    }

    /// Return the exact retained bytes implied by the current live state.
    fn exact_retained_bytes(&self) -> u64 {
        let mut retained_bytes = 0usize;

        // core directories
        retained_bytes += self.small.spans.capacity() * size_of::<ManagedSpan>();
        retained_bytes += self.small.size_classes.retained_bytes();
        retained_bytes += self.small.available_spans.capacity() * size_of::<Vec<usize>>();
        retained_bytes += self.small.free_span_ids.capacity() * size_of::<usize>();
        retained_bytes +=
            self.large.large_allocations.capacity() * size_of::<ManagedLargeAllocation>();
        retained_bytes += self.large.free_large_allocation_ids.capacity() * size_of::<u64>();
        retained_bytes += self.page_arena.retained_bytes();
        retained_bytes += self.handles.capacity() * size_of::<ManagedHandleEntry>();
        retained_bytes += self.live_handles.capacity() * size_of::<u64>();
        retained_bytes += self.mark_queue.capacity() * size_of::<ManagedReference>();
        retained_bytes += self.dirty_spans.capacity() * size_of::<usize>();
        retained_bytes +=
            self.dirty_large_allocations.capacity() * size_of::<ManagedLargeAllocationId>();
        retained_bytes += self.dirty_next_spans.capacity() * size_of::<usize>();
        retained_bytes +=
            self.dirty_next_large_allocations.capacity() * size_of::<ManagedLargeAllocationId>();
        retained_bytes += self.young.retained_bytes();

        if let Some(from) = &self.young_from {
            retained_bytes += from.retained_bytes();
        }

        // span vectors
        for queue in &self.small.available_spans {
            retained_bytes += queue.capacity() * size_of::<usize>();
        }

        // leaf backing
        for span in &self.small.spans {
            retained_bytes += span.active_bytes(self.page_arena.page_bytes());
        }

        for large_allocation in &self.large.large_allocations {
            retained_bytes += large_allocation.active_bytes();
        }

        retained_bytes += self.reference_map_table.retained_bytes();

        retained_bytes as u64
    }

    /// Assert that the incremental retained-byte cache matches exact state.
    fn debug_assert_retained_bytes(&self) {
        #[cfg(debug_assertions)]
        {
            debug_assert!(
                !self.retained_bytes_dirty.get(),
                "retained-byte audit requires one clean managed-space cache"
            );

            let retained_bytes = self.exact_retained_bytes();
            debug_assert_eq!(
                self.retained_bytes.get(),
                retained_bytes,
                "managed-space retained bytes must match exact recomputation"
            );
        }
    }
}

/// Return the retained-byte delta implied by one vector-capacity change.
fn capacity_bytes_delta<T>(old_capacity: usize, new_capacity: usize) -> i64 {
    let old_bytes = old_capacity.saturating_mul(size_of::<T>()) as i64;
    let new_bytes = new_capacity.saturating_mul(size_of::<T>()) as i64;

    new_bytes - old_bytes
}
