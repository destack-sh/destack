use destack_mir::LayoutId;

use super::{
    Allocation, AllocationId, ManagedLocation, ManagedReferenceRecord, ManagedSpace,
    ManagedYoungId, ReferenceMap, ReferenceMapId, Span, SpanSlot, StoredLayoutId, YoungAllocation,
};
use crate::Bitmap;
use crate::alloc::{CardSet, PageMap};
use crate::value::ManagedReference;

impl ManagedSpace {
    /// Return the projected active-byte reservation for one managed allocation.
    pub fn allocate_bytes_active_reservation(
        &self,
        byte_len: usize,
        _reference_map: &ReferenceMap,
        _layout_id: Option<LayoutId>,
    ) -> i64 {
        byte_len as i64
    }

    /// Allocate one managed byte allocation.
    pub fn allocate_bytes(
        &mut self,
        bytes: &[u8],
        reference_map: ReferenceMap,
        layout_id: Option<LayoutId>,
    ) -> ManagedReference {
        let (trace_id, _) = self.reference_map_table.intern(reference_map);
        let reference_id = self.allocate_reference_id();
        let reference = ManagedReference::new(reference_id);
        let location = if let Some(young_id) = self.allocate_young(bytes, trace_id, layout_id) {
            ManagedLocation::Young(young_id)
        } else if let Some(slot) = self.allocate_small_slot(bytes, trace_id, layout_id) {
            ManagedLocation::Small(slot)
        } else {
            let pages = self.arena.allocate_bytes(bytes);
            let allocation_id =
                self.allocate_allocation_slot(bytes.len(), pages, trace_id, layout_id);

            ManagedLocation::Large(allocation_id)
        };

        self.store_reference(
            reference_id,
            ManagedReferenceRecord::new(location, bytes.len(), None),
        );
        self.allocated_count = self.allocated_count.saturating_add(1);
        self.allocated_bytes = self.allocated_bytes.saturating_add(bytes.len() as u64);

        reference
    }

    /// Allocate one zeroed managed byte allocation.
    pub fn allocate_zeroed(
        &mut self,
        byte_len: usize,
        reference_map: ReferenceMap,
        layout_id: Option<LayoutId>,
    ) -> ManagedReference {
        self.allocate_bytes(&vec![0; byte_len], reference_map, layout_id)
    }

    /// Allocate one reference id from the intrusive free list or the unused tail.
    fn allocate_reference_id(&mut self) -> u64 {
        if let Some(reference_id) = self.free_reference_ids.pop() {
            reference_id
        } else {
            let reference_id = self.next_unused_reference_id;
            self.next_unused_reference_id = self.next_unused_reference_id.saturating_add(1);

            reference_id
        }
    }

    /// Store one live reference entry by reference id.
    fn store_reference(&mut self, reference_id: u64, record: ManagedReferenceRecord) {
        let index = reference_id.saturating_sub(1) as usize;
        if index == self.references.len() {
            self.references.push(record);
        } else if let Some(entry) = self.references.get_mut(index) {
            *entry = record;
        }
    }

    /// Allocate one allocation slot in large space and return its stable id.
    pub(super) fn allocate_allocation_slot(
        &mut self,
        len: usize,
        pages: PageMap,
        trace_id: ReferenceMapId,
        layout_id: Option<LayoutId>,
    ) -> AllocationId {
        let allocation_id = if let Some(allocation_id) = self.large.free_allocation_ids.pop() {
            allocation_id
        } else {
            let allocation_id = self.large.next_unused_allocation_id;
            self.large.next_unused_allocation_id =
                self.large.next_unused_allocation_id.saturating_add(1);
            allocation_id
        };
        let allocation = Allocation {
            is_allocated: true,
            len,
            pages,
            trace_id,
            layout_id: StoredLayoutId::from_option(layout_id),
            marked: false,
            pin_count: 0,
            dirty_cards: CardSet::with_len(len),
            is_dirty_queued: false,
        };
        let index = allocation_id.saturating_sub(1) as usize;

        if index == self.large.allocations.len() {
            self.large.allocations.push(allocation);
        } else if let Some(existing) = self.large.allocations.get_mut(index) {
            *existing = allocation;
        }

        AllocationId::new(allocation_id)
    }

    /// Allocate one young allocation if the nursery has room.
    fn allocate_young(
        &mut self,
        bytes: &[u8],
        trace_id: ReferenceMapId,
        layout_id: Option<LayoutId>,
    ) -> Option<ManagedYoungId> {
        let byte_len = bytes.len();
        let end_offset = self.young.next_offset.checked_add(byte_len)?;

        if end_offset > self.young.capacity_bytes {
            return None;
        }

        let allocation_id = if let Some(allocation_id) = self.young.free_ids.last().copied() {
            let mut free_ids = self.young.free_ids.to_vec();
            free_ids.pop();
            self.young.free_ids = free_ids.into_boxed_slice();
            allocation_id
        } else {
            self.young.allocations.len() as u32
        };

        let allocation = YoungAllocation {
            first_page: (self.young.next_offset / self.young.page_bytes) as u32,
            first_offset: (self.young.next_offset % self.young.page_bytes) as u32,
            byte_len,
            trace_id,
            layout_id: StoredLayoutId::from_option(layout_id),
            age: 0,
            is_allocated: true,
            marked: false,
        };

        let write_offset = self.young.next_offset;
        let is_written = self
            .arena
            .set_bytes(&mut self.young.pages, write_offset, bytes);
        debug_assert!(is_written);

        if allocation_id as usize == self.young.allocations.len() {
            let mut allocations = self.young.allocations.to_vec();
            allocations.push(allocation);
            self.young.allocations = allocations.into_boxed_slice();
        } else if let Some(entry) = self.young.allocations.get_mut(allocation_id as usize) {
            *entry = allocation;
        }

        self.young.next_offset = end_offset;

        Some(ManagedYoungId::new(self.young.generation, allocation_id))
    }

    /// Allocate one small managed slot if the payload fits one configured class.
    pub(super) fn allocate_small_slot(
        &mut self,
        bytes: &[u8],
        trace_id: ReferenceMapId,
        layout_id: Option<LayoutId>,
    ) -> Option<SpanSlot> {
        let class_index = self.small.size_classes.class_index_for(bytes.len())?;
        let size_class = self.small.size_classes.classes[class_index].bytes;
        let span_index = self.allocate_small_span(class_index, size_class);
        let span = self.small.spans.get_mut(span_index)?;
        let slot_index = span.next_free_slot;
        let slot_offset = span.size_class.saturating_mul(slot_index);
        let is_written = self.arena.set_bytes(&mut span.pages, slot_offset, bytes);

        debug_assert!(is_written);

        span.occupied.set(slot_index);
        span.occupied_count = span.occupied_count.saturating_add(1);
        span.next_free_slot = span
            .occupied
            .first_clear_from(slot_index)
            .unwrap_or(span.slot_count);
        Self::set_span_trace_id(span, slot_index, trace_id);
        Self::set_span_layout_id(span, slot_index, layout_id);

        if span.occupied_count < span.slot_count {
            self.small.available_spans[class_index].push(span_index);
        }

        Some(SpanSlot::new(span_index, slot_index))
    }

    /// Allocate or reuse one non-full managed span for the given size class.
    fn allocate_small_span(&mut self, class_index: usize, size_class: usize) -> usize {
        while let Some(span_index) = self.small.available_spans[class_index].pop() {
            let Some(span) = self.small.spans.get(span_index) else {
                continue;
            };

            if span.occupied_count < span.slot_count {
                return span_index;
            }
        }

        let slot_count = (self.small.span_bytes / size_class).max(1);
        let span = Span {
            size_class,
            slot_count,
            occupied_count: 0,
            next_free_slot: 0,
            occupied: Bitmap::with_capacity(slot_count),
            trace_ids: vec![ReferenceMapId::new(0); slot_count].into_boxed_slice(),
            layout_ids: vec![StoredLayoutId::from_option(None); slot_count].into_boxed_slice(),
            pages: self.arena.allocate_zeroed(self.small.span_bytes),
            marked: Bitmap::with_capacity(slot_count),
            pinned: Bitmap::with_capacity(slot_count),
            extra_pin_counts: Vec::new(),
            active_pins: 0,
            dirty_cards: CardSet::with_len(slot_count.saturating_mul(size_class)),
            is_dirty_queued: false,
        };
        let span_index = self.small.spans.len();

        self.small.spans.push(span);

        span_index
    }
}
