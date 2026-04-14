use serde::{Deserialize, Serialize};

use std::sync::Arc;

use super::{
    Allocation, AllocationImage, GcState, ManagedReferenceRecord, ManagedSpace, ReferenceMap,
    ReferenceMapId, ReferenceMapTable, Span, SpanImage, StoredLayoutId, YoungImage, YoungSpace,
};
use crate::Bitmap;
use crate::alloc::{Arena, CardSet, SizeClassTable};
use crate::heap::{HeapCaptureError, HeapLayoutError, check_managed_reference_bytes};

/// One frozen managed-space root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ManagedSpaceImage {
    /// The configured size-class table.
    size_classes: SizeClassTable,
    /// The encoded byte width for managed references inside traced payloads.
    managed_reference_bytes: u8,
    /// The configured young-space byte width.
    young_bytes: usize,
    /// The configured small-space span width.
    small_bytes: usize,
    /// The configured local page width.
    page_bytes: usize,
    /// The captured branchable nursery root.
    young: YoungImage,
    /// The captured managed spans.
    spans: Box<[SpanImage]>,
    /// The captured managed allocations in large space.
    allocations: Box<[AllocationImage]>,
    /// Dense managed reference metadata keyed by reference id minus one.
    references: Box<[ManagedReferenceRecord]>,
    /// The next managed reference id to allocate.
    next_unused_reference_id: u64,
    /// The next managed allocation id to allocate in large space.
    next_unused_allocation_id: u64,
    /// The number of allocated managed references.
    allocated_count: usize,
    /// The number of allocated managed bytes.
    allocated_bytes: u64,
    /// The captured reference maps.
    reference_maps: Box<[ReferenceMap]>,
    /// The captured GC state.
    gc_state: GcState,
}

/// One serialized managed-space snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ManagedSpaceSnapshot {
    /// The configured size-class table.
    pub(crate) size_classes: SizeClassTable,
    /// The encoded byte width for managed references inside traced payloads.
    pub(crate) managed_reference_bytes: u8,
    /// The configured young-space byte width.
    pub(crate) young_bytes: usize,
    /// The configured small-space span width.
    pub(crate) small_bytes: usize,
    /// The configured local page width.
    pub(crate) page_bytes: usize,
    /// The serialized branchable nursery root.
    pub(crate) young: super::YoungSnapshot,
    /// The serialized managed spans.
    pub(crate) spans: Box<[SpanImage]>,
    /// The serialized managed allocations in large space.
    pub(crate) allocations: Box<[AllocationImage]>,
    /// Dense managed reference metadata keyed by reference id minus one.
    pub(crate) references: Box<[ManagedReferenceRecord]>,
    /// The next managed reference id to allocate.
    pub(crate) next_unused_reference_id: u64,
    /// The next managed allocation id to allocate in large space.
    pub(crate) next_unused_allocation_id: u64,
    /// The number of allocated managed references.
    pub(crate) allocated_count: usize,
    /// The number of allocated managed bytes.
    pub(crate) allocated_bytes: u64,
    /// The serialized reference maps.
    pub(crate) reference_maps: Box<[ReferenceMap]>,
    /// The captured GC state.
    pub(crate) gc_state: GcState,
}

#[allow(clippy::too_many_arguments)]
impl ManagedSpaceImage {
    /// Create one frozen managed-space root.
    pub(crate) fn new(
        size_classes: SizeClassTable,
        managed_reference_bytes: u8,
        young_bytes: usize,
        small_bytes: usize,
        page_bytes: usize,
        young: YoungImage,
        spans: Box<[SpanImage]>,
        allocations: Box<[AllocationImage]>,
        references: Box<[ManagedReferenceRecord]>,
        next_unused_reference_id: u64,
        next_unused_allocation_id: u64,
        allocated_count: usize,
        allocated_bytes: u64,
        reference_maps: Box<[ReferenceMap]>,
        gc_state: GcState,
    ) -> Self {
        Self {
            size_classes,
            managed_reference_bytes,
            young_bytes,
            small_bytes,
            page_bytes,
            young,
            spans,
            allocations,
            references,
            next_unused_reference_id,
            next_unused_allocation_id,
            allocated_count,
            allocated_bytes,
            reference_maps,
            gc_state,
        }
    }

    /// Build one frozen managed-space root from one serialized snapshot.
    pub(crate) fn from_snapshot(snapshot: &ManagedSpaceSnapshot) -> Self {
        Self {
            size_classes: snapshot.size_classes.clone(),
            managed_reference_bytes: snapshot.managed_reference_bytes,
            young_bytes: snapshot.young_bytes,
            small_bytes: snapshot.small_bytes,
            page_bytes: snapshot.page_bytes,
            young: YoungImage::from_snapshot(&snapshot.young),
            spans: snapshot.spans.clone(),
            allocations: snapshot.allocations.clone(),
            references: snapshot.references.clone(),
            next_unused_reference_id: snapshot.next_unused_reference_id,
            next_unused_allocation_id: snapshot.next_unused_allocation_id,
            allocated_count: snapshot.allocated_count,
            allocated_bytes: snapshot.allocated_bytes,
            reference_maps: snapshot.reference_maps.clone(),
            gc_state: snapshot.gc_state.clone(),
        }
    }

    /// Flatten one frozen managed-space root into one serialized snapshot.
    pub(crate) fn snapshot(&self) -> ManagedSpaceSnapshot {
        ManagedSpaceSnapshot {
            size_classes: self.size_classes.clone(),
            managed_reference_bytes: self.managed_reference_bytes,
            young_bytes: self.young_bytes,
            small_bytes: self.small_bytes,
            page_bytes: self.page_bytes,
            young: self.young.snapshot(),
            spans: self.spans.clone(),
            allocations: self.allocations.clone(),
            references: self.references.clone(),
            next_unused_reference_id: self.next_unused_reference_id,
            next_unused_allocation_id: self.next_unused_allocation_id,
            allocated_count: self.allocated_count,
            allocated_bytes: self.allocated_bytes,
            reference_maps: self.reference_maps.clone(),
            gc_state: self.gc_state.clone(),
        }
    }

    /// Return the configured size-class table.
    pub(crate) fn size_classes(&self) -> &SizeClassTable {
        &self.size_classes
    }

    /// Return the branchable nursery root.
    pub(crate) fn young(&self) -> &YoungImage {
        &self.young
    }

    /// Return the encoded byte width for managed references.
    pub(crate) const fn managed_reference_bytes(&self) -> u8 {
        self.managed_reference_bytes
    }

    /// Return the configured small-space span width.
    pub(crate) const fn small_bytes(&self) -> usize {
        self.small_bytes
    }

    /// Return the configured local page width.
    pub(crate) const fn page_bytes(&self) -> usize {
        self.page_bytes
    }

    /// Return the captured managed spans.
    pub(crate) fn spans(&self) -> &[SpanImage] {
        &self.spans
    }

    /// Return the captured managed allocations in large space.
    pub(crate) fn allocations(&self) -> &[AllocationImage] {
        &self.allocations
    }

    /// Return the captured managed reference table.
    pub(crate) fn references(&self) -> &[ManagedReferenceRecord] {
        &self.references
    }

    /// Return the next managed reference id.
    pub(crate) const fn next_unused_reference_id(&self) -> u64 {
        self.next_unused_reference_id
    }

    /// Return the next managed allocation id in large space.
    pub(crate) const fn next_unused_allocation_id(&self) -> u64 {
        self.next_unused_allocation_id
    }

    /// Return the allocated managed reference count.
    pub(crate) const fn allocated_count(&self) -> usize {
        self.allocated_count
    }

    /// Return the allocated managed bytes.
    pub(crate) const fn allocated_bytes(&self) -> u64 {
        self.allocated_bytes
    }

    /// Return the captured reference maps.
    pub(crate) fn reference_maps(&self) -> &[ReferenceMap] {
        &self.reference_maps
    }

    /// Return the captured collector state.
    pub(crate) fn gc_state(&self) -> &GcState {
        &self.gc_state
    }
}

impl ManagedSpace {
    /// Restore one managed space from one frozen managed-space root.
    pub(crate) fn from_image(
        arena: Arc<Arena>,
        image: &ManagedSpaceImage,
    ) -> Result<Self, HeapLayoutError> {
        check_managed_reference_bytes(image.managed_reference_bytes())?;

        Ok(Self::restore_from_image(arena, image))
    }

    /// Restore one managed space from one checked frozen managed-space root.
    fn restore_from_image(arena: Arc<Arena>, image: &ManagedSpaceImage) -> Self {
        // retain the shared backing first
        Self::retain_image_pages(&arena, image);

        // rebuild the dense metadata tables
        let references = Self::restore_reference_table(image);
        let free_reference_ids = Self::free_reference_ids(&references);
        let reference_map_table = Self::restore_reference_map_table(image);

        // rebuild each live managed storage partition
        let young = Self::restore_young_space(image);
        let small = Self::restore_small_space(image);
        let large = Self::restore_large_space(image);

        // rebuild the live root over the shared arena
        Self {
            arena,
            managed_reference_bytes: image.managed_reference_bytes(),
            reference_map_table,
            young,
            small,
            large,
            references,
            free_reference_ids,
            next_unused_reference_id: image.next_unused_reference_id(),
            allocated_count: image.allocated_count(),
            allocated_bytes: image.allocated_bytes(),
            gc_state: image.gc_state().clone(),
        }
    }

    /// Return one frozen managed-space root.
    pub(crate) fn image(&self) -> Result<ManagedSpaceImage, HeapCaptureError> {
        // capture the live managed storage directly
        let young = self.capture_young_image();
        let spans = self.capture_span_images();
        let allocations = self.capture_allocation_images();
        let references = self.capture_reference_table();
        let reference_maps = self.capture_reference_maps();

        // freeze the current managed root
        Ok(ManagedSpaceImage::new(
            self.small.size_classes.clone(),
            self.managed_reference_bytes,
            self.young.capacity_bytes,
            self.small.span_bytes,
            self.large.page_bytes,
            young,
            spans,
            allocations,
            references,
            self.next_unused_reference_id,
            self.large.next_unused_allocation_id,
            self.allocated_count,
            self.allocated_bytes,
            reference_maps,
            self.gc_state.clone(),
        ))
    }

    /// Retain every arena page reachable from one frozen managed-space root.
    fn retain_image_pages(arena: &Arc<Arena>, image: &ManagedSpaceImage) {
        // retain every captured span root
        for span in image.spans() {
            arena.retain_pages(&span.pages);
        }

        // retain every captured large allocation root
        for allocation in image.allocations() {
            arena.retain_pages(&allocation.pages);
        }

        // retain the nursery root last
        arena.retain_pages(image.young().pages());
    }

    /// Rebuild the dense managed reference table from one frozen image.
    fn restore_reference_table(image: &ManagedSpaceImage) -> Vec<ManagedReferenceRecord> {
        image.references().to_vec()
    }

    /// Rebuild the interned reference maps from one frozen image.
    fn restore_reference_map_table(image: &ManagedSpaceImage) -> ReferenceMapTable {
        ReferenceMapTable::from_maps(image.reference_maps().to_vec())
    }

    /// Return the reusable managed reference ids from one frozen table.
    fn free_reference_ids(references: &[ManagedReferenceRecord]) -> Vec<u64> {
        references
            .iter()
            .enumerate()
            .filter_map(|(index, record)| record.is_vacant().then_some(index as u64 + 1))
            .collect()
    }

    /// Restore the managed young-allocation space from one frozen image.
    fn restore_young_space(image: &ManagedSpaceImage) -> YoungSpace {
        YoungSpace {
            generation: image.young().generation(),
            capacity_bytes: image.young().capacity_bytes(),
            page_bytes: image.young().page_bytes(),
            next_offset: image.young().next_offset(),
            pages: image.young().pages().clone(),
            allocations: image.young().allocations().to_vec().into_boxed_slice(),
            free_ids: image.young().free_ids().to_vec().into_boxed_slice(),
        }
    }

    /// Restore the managed small-allocation space from one frozen image.
    fn restore_small_space(image: &ManagedSpaceImage) -> super::SmallSpace {
        // restore the captured span roots first
        let spans = image
            .spans()
            .iter()
            .map(Self::restore_span)
            .collect::<Vec<_>>();

        let mut small = super::SmallSpace {
            size_classes: image.size_classes().clone(),
            span_bytes: image.small_bytes(),
            spans,
            available_spans: vec![Vec::new(); image.size_classes().classes.len()],
        };

        // rebuild the derived span occupancy state
        Self::restore_available_spans(&mut small);

        small
    }

    /// Rebuild the derived reusable-span state for one restored small space.
    fn restore_available_spans(small: &mut super::SmallSpace) {
        for (span_index, span) in small.spans.iter_mut().enumerate() {
            // rebuild the derived per-span occupancy counters
            span.occupied_count = span.occupied.count_ones();
            span.next_free_slot = span.occupied.first_clear_from(0).unwrap_or(span.slot_count);

            // requeue every non-full span under its size class
            if span.occupied_count >= span.slot_count {
                continue;
            }

            let Some(class_index) = small.size_classes.class_index_for(span.size_class) else {
                continue;
            };

            small.available_spans[class_index].push(span_index);
        }
    }

    /// Restore one managed span from one frozen span root.
    fn restore_span(span: &SpanImage) -> Span {
        // rebuild the per-slot tracing table
        let trace_ids = span
            .trace_ids
            .iter()
            .copied()
            .map(|trace_id| ReferenceMapId::new(trace_id as usize))
            .collect::<Vec<_>>()
            .into_boxed_slice();

        // rebuild the live span around the captured page map
        Span {
            size_class: span.size_class,
            slot_count: span.slot_count,
            occupied_count: 0,
            next_free_slot: 0,
            occupied: span.occupied.clone(),
            trace_ids,
            layout_ids: span.layout_ids.clone(),
            pages: span.pages.clone(),
            marked: Bitmap::with_capacity(span.slot_count),
            pinned: Bitmap::with_capacity(span.slot_count),
            extra_pin_counts: Vec::new(),
            active_pins: 0,
            dirty_cards: CardSet::with_len(span.slot_count.saturating_mul(span.size_class)),
            is_dirty_queued: false,
        }
    }

    /// Restore the managed large space from one frozen image.
    fn restore_large_space(image: &ManagedSpaceImage) -> super::LargeSpace {
        // rebuild the captured allocation roots first
        let allocations = image
            .allocations()
            .iter()
            .map(Self::restore_allocation)
            .collect();

        // rebuild the reusable allocation ids from the frozen table
        let free_allocation_ids = Self::free_allocation_ids(image);

        super::LargeSpace {
            page_bytes: image.page_bytes(),
            allocations,
            free_allocation_ids,
            next_unused_allocation_id: image.next_unused_allocation_id(),
        }
    }

    /// Restore one managed allocation from one frozen allocation root.
    fn restore_allocation(allocation: &AllocationImage) -> Allocation {
        Allocation {
            is_allocated: allocation.is_allocated,
            len: allocation.len,
            pages: allocation.pages.clone(),
            trace_id: allocation.trace_id,
            layout_id: StoredLayoutId::from_option(allocation.layout_id),
            marked: false,
            pin_count: 0,
            dirty_cards: CardSet::with_len(allocation.len),
            is_dirty_queued: false,
        }
    }

    /// Return the reusable managed allocation ids from one frozen table.
    fn free_allocation_ids(image: &ManagedSpaceImage) -> Vec<u64> {
        image
            .allocations()
            .iter()
            .enumerate()
            .filter_map(|(index, allocation)| {
                (!allocation.is_allocated).then_some(index as u64 + 1)
            })
            .collect()
    }

    /// Capture the live managed young-space image.
    fn capture_young_image(&self) -> YoungImage {
        YoungImage::new(
            self.young.generation,
            self.young.capacity_bytes,
            self.young.page_bytes,
            self.young.next_offset,
            self.young.pages.clone(),
            self.young.allocations.clone(),
            self.young.free_ids.clone(),
        )
    }

    /// Capture the dense managed reference table.
    fn capture_reference_table(&self) -> Box<[ManagedReferenceRecord]> {
        self.references.clone().into_boxed_slice()
    }

    /// Capture the interned managed reference maps.
    fn capture_reference_maps(&self) -> Box<[ReferenceMap]> {
        self.reference_map_table.snapshot().into_boxed_slice()
    }

    /// Capture every live managed span image.
    fn capture_span_images(&self) -> Box<[SpanImage]> {
        self.small
            .spans
            .iter()
            .map(Self::capture_span_image)
            .collect::<Vec<_>>()
            .into_boxed_slice()
    }

    /// Capture one live managed span image.
    fn capture_span_image(span: &Span) -> SpanImage {
        SpanImage {
            size_class: span.size_class,
            slot_count: span.slot_count,
            occupied: span.occupied.clone(),
            trace_ids: span
                .trace_ids
                .iter()
                .map(|trace_id| trace_id.index() as u32)
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            layout_ids: span.layout_ids.clone(),
            pages: span.pages.clone(),
        }
    }

    /// Capture every live managed allocation image in large space.
    fn capture_allocation_images(&self) -> Box<[AllocationImage]> {
        self.large
            .allocations
            .iter()
            .map(Self::capture_allocation_image)
            .collect::<Vec<_>>()
            .into_boxed_slice()
    }

    /// Capture one live managed allocation image in large space.
    fn capture_allocation_image(allocation: &Allocation) -> AllocationImage {
        AllocationImage {
            is_allocated: allocation.is_allocated,
            len: allocation.len,
            pages: allocation.pages.clone(),
            trace_id: allocation.trace_id,
            layout_id: allocation.layout_id.to_option(),
        }
    }
}
