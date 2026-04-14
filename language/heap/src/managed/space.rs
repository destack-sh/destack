use std::sync::Arc;

use destack_mir::LayoutId;

use super::{
    Allocation, AllocationId, GcState, ManagedLocation, ManagedReferenceRecord, ManagedYoungId,
    ReferenceMapId, ReferenceMapTable, Span, StoredLayoutId, YoungAllocation, YoungSpace,
};
use crate::alloc::{Arena, SizeClassTable};
use crate::heap::{HeapLayout, HeapLayoutError, ManagedSpaceUsage, check_managed_reference_bytes};
use crate::value::ManagedReference;

/// The first non-null managed reference id.
pub(crate) const FIRST_ALLOCATED_REFERENCE_ID: u64 = 1;

/// The first non-null managed allocation id in large space.
const FIRST_ALLOCATED_ALLOCATION_ID: u64 = 1;

/// One managed small-allocation space.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SmallSpace {
    /// The configured size-class table.
    pub(crate) size_classes: SizeClassTable,
    /// The configured span width.
    pub(crate) span_bytes: usize,
    /// The live managed spans.
    pub(crate) spans: Vec<Span>,
    /// The reusable non-full spans per size class.
    pub(crate) available_spans: Vec<Vec<usize>>,
}

/// One managed large space.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LargeSpace {
    /// The configured page width for allocations in large space.
    pub(crate) page_bytes: usize,
    /// The live managed allocations.
    pub(crate) allocations: Vec<Allocation>,
    /// The free managed allocation ids available for reuse.
    pub(crate) free_allocation_ids: Vec<u64>,
    /// The next managed allocation id to allocate.
    pub(crate) next_unused_allocation_id: u64,
}

/// One live managed allocation space rooted in one arena.
#[derive(Debug, Clone)]
pub struct ManagedSpace {
    /// The shared page arena for every managed payload.
    pub(super) arena: Arc<Arena>,

    /// The managed young-allocation space.
    pub(crate) young: YoungSpace,
    /// The managed small-allocation space.
    pub(crate) small: SmallSpace,
    /// The managed large space.
    pub(crate) large: LargeSpace,

    /// The encoded byte width for managed references inside traced payloads.
    pub(crate) managed_reference_bytes: u8,
    /// The interned managed reference maps.
    pub(crate) reference_map_table: ReferenceMapTable,
    /// Dense managed reference metadata keyed by reference id minus one.
    pub(crate) references: Vec<ManagedReferenceRecord>,
    /// The free managed reference ids available for reuse.
    pub(crate) free_reference_ids: Vec<u64>,
    /// The next managed reference id to allocate.
    pub(crate) next_unused_reference_id: u64,
    /// The number of allocated managed references.
    pub(crate) allocated_count: usize,
    /// The number of allocated managed bytes.
    pub(crate) allocated_bytes: u64,
    /// The live GC state.
    pub(crate) gc_state: GcState,
}

impl Default for ManagedSpace {
    fn default() -> Self {
        Self::new()
    }
}

impl ManagedSpace {
    /// Create one managed space with the default layout.
    pub fn new() -> Self {
        let options = HeapLayout::default();
        let arena = Arc::new(Arena::with_page_bytes(options.page_bytes));

        Self::build_with_layout(arena, &options)
    }

    /// Create one managed space with explicit layout.
    pub fn with_layout(arena: Arc<Arena>, options: &HeapLayout) -> Result<Self, HeapLayoutError> {
        check_managed_reference_bytes(options.managed_reference_bytes)?;

        Ok(Self::build_with_layout(arena, options))
    }

    /// Create one managed space from one checked layout.
    pub(crate) fn build_with_layout(arena: Arc<Arena>, options: &HeapLayout) -> Self {
        // reserve one fixed nursery up front
        let young = YoungSpace::new(&arena, options.managed_young_bytes, options.page_bytes);

        // build the live root over the shared arena
        Self {
            arena,
            managed_reference_bytes: options.managed_reference_bytes,
            reference_map_table: ReferenceMapTable::new(),
            young,
            small: SmallSpace {
                size_classes: options.size_classes.clone(),
                span_bytes: options.managed_small_bytes,
                spans: Vec::new(),
                available_spans: vec![Vec::new(); options.size_classes.classes.len()],
            },
            large: LargeSpace {
                page_bytes: options.page_bytes,
                allocations: Vec::new(),
                free_allocation_ids: Vec::new(),
                next_unused_allocation_id: FIRST_ALLOCATED_ALLOCATION_ID,
            },
            references: Vec::new(),
            free_reference_ids: Vec::new(),
            next_unused_reference_id: FIRST_ALLOCATED_REFERENCE_ID,
            allocated_count: 0,
            allocated_bytes: 0,
            gc_state: GcState::default(),
        }
    }

    /// Return the shared page arena.
    pub fn arena(&self) -> &Arc<Arena> {
        &self.arena
    }

    /// Return the exact retained managed bytes.
    pub fn active_bytes(&self) -> u64 {
        self.allocated_bytes
    }

    /// Return the exact mapped managed page bytes.
    pub fn mapped_bytes(&self) -> u64 {
        0
    }

    /// Return the exact borrowed managed image bytes.
    pub fn borrowed_bytes(&self) -> u64 {
        self.allocated_bytes
    }

    /// Return the current GC state.
    pub fn gc_state(&self) -> &GcState {
        &self.gc_state
    }

    /// Return the number of live managed allocations.
    pub fn allocation_count(&self) -> usize {
        self.allocated_count
    }

    /// Return the number of live managed bytes.
    pub fn allocated_bytes(&self) -> u64 {
        self.allocated_bytes
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

    /// Return one live reference record by managed reference.
    pub(super) fn reference(&self, reference: ManagedReference) -> Option<&ManagedReferenceRecord> {
        // resolve the dense table slot first
        let index = reference.id().checked_sub(1)? as usize;
        let record = self.references.get(index)?;

        // skip vacant records
        (!record.is_vacant()).then_some(record)
    }

    /// Return one live reference record mutably by managed reference.
    pub(super) fn reference_mut(
        &mut self,
        reference: ManagedReference,
    ) -> Option<&mut ManagedReferenceRecord> {
        // resolve the dense table slot first
        let index = reference.id().checked_sub(1)? as usize;
        let record = self.references.get_mut(index)?;

        // skip vacant records
        (!record.is_vacant()).then_some(record)
    }

    /// Return one live allocation by id.
    pub(super) fn allocation(&self, allocation_id: AllocationId) -> Option<&Allocation> {
        // resolve the dense table slot first
        let index = allocation_id.id().checked_sub(1)? as usize;
        let allocation = self.large.allocations.get(index)?;

        // skip free allocations
        allocation.is_allocated.then_some(allocation)
    }

    /// Return one live allocation mutably by id.
    pub(super) fn allocation_mut(
        &mut self,
        allocation_id: AllocationId,
    ) -> Option<&mut Allocation> {
        // resolve the dense table slot first
        let index = allocation_id.id().checked_sub(1)? as usize;
        let allocation = self.large.allocations.get_mut(index)?;

        // skip free allocations
        allocation.is_allocated.then_some(allocation)
    }

    /// Return one live young allocation by id.
    pub(super) fn young_allocation(&self, young_id: ManagedYoungId) -> Option<&YoungAllocation> {
        // reject stale generation ids first
        self.check_young_generation(young_id)?;

        let allocation = self.young.allocations.get(young_id.index() as usize)?;

        // skip free nursery slots
        allocation.is_allocated.then_some(allocation)
    }

    /// Return one live young allocation mutably by id.
    pub(super) fn young_allocation_mut(
        &mut self,
        young_id: ManagedYoungId,
    ) -> Option<&mut YoungAllocation> {
        // reject stale generation ids first
        self.check_young_generation(young_id)?;

        let allocation = self.young.allocations.get_mut(young_id.index() as usize)?;

        // skip free nursery slots
        allocation.is_allocated.then_some(allocation)
    }

    /// Return one live managed span by index.
    pub(super) fn span(&self, span_index: usize) -> Option<&Span> {
        self.small.spans.get(span_index)
    }

    /// Return one live managed span mutably by index.
    pub(super) fn span_mut(&mut self, span_index: usize) -> Option<&mut Span> {
        self.small.spans.get_mut(span_index)
    }

    /// Return the layout id for one managed span slot.
    pub(super) fn span_layout_id(&self, span: &Span, slot_index: usize) -> Option<LayoutId> {
        span.layout_ids.get(slot_index)?.to_option()
    }

    /// Set the layout id for one managed span slot.
    pub(super) fn set_span_layout_id(
        span: &mut Span,
        slot_index: usize,
        layout_id: Option<LayoutId>,
    ) {
        if let Some(entry) = span.layout_ids.get_mut(slot_index) {
            *entry = StoredLayoutId::from_option(layout_id);
        }
    }

    /// Set the trace id for one managed span slot.
    pub(super) fn set_span_trace_id(span: &mut Span, slot_index: usize, trace_id: ReferenceMapId) {
        if let Some(entry) = span.trace_ids.get_mut(slot_index) {
            *entry = trace_id;
        }
    }

    /// Return the byte offset for one young allocation.
    pub(super) fn young_allocation_offset(&self, allocation: &YoungAllocation) -> usize {
        allocation.first_page as usize * self.young.page_bytes + allocation.first_offset as usize
    }

    /// Check that one young id belongs to the current nursery generation.
    fn check_young_generation(&self, young_id: ManagedYoungId) -> Option<()> {
        (young_id.generation() == self.young.generation).then_some(())
    }

    /// Return the reference map id for one managed location.
    pub(super) fn location_trace_id(&self, location: ManagedLocation) -> Option<ReferenceMapId> {
        // resolve the location-specific trace source
        match location {
            ManagedLocation::Vacant => None,
            ManagedLocation::Young(young_id) => Some(self.young_allocation(young_id)?.trace_id),
            ManagedLocation::Small(slot) => {
                let span = self.span(slot.span_index())?;
                let slot_index = slot.slot_index();

                span.trace_ids.get(slot_index).copied()
            }
            ManagedLocation::Large(allocation_id) => Some(self.allocation(allocation_id)?.trace_id),
        }
    }

    /// Append one value to one boxed u32 slice.
    pub(super) fn push_boxed_u32(&self, slice: &[u32], value: u32) -> Box<[u32]> {
        // append through one temporary vector
        let mut values = slice.to_vec();
        values.push(value);

        values.into_boxed_slice()
    }
}
