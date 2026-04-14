use super::{Allocation, AllocationId, RawLocation, RawPointerRecord, RawSpace, Span, SpanSlot};
use crate::alloc::PageMap;
use crate::value::RawPointer;

impl RawSpace {
    /// Return the projected active-byte reservation for one raw allocation.
    pub fn allocate_bytes_active_reservation(&self, byte_len: usize) -> i64 {
        byte_len as i64
    }

    /// Allocate one raw byte allocation.
    pub fn allocate_bytes(&mut self, bytes: &[u8]) -> RawPointer {
        // allocate the stable pointer id first
        let pointer_id = self.allocate_pointer_id();
        let pointer = RawPointer::new(pointer_id);
        let location = self.allocate_location(bytes);

        // then install the live pointer record
        self.store_pointer(
            pointer_id,
            RawPointerRecord {
                location,
                byte_len: bytes.len(),
            },
        );

        // charge the live raw allocation counters
        self.allocated_count = self.allocated_count.saturating_add(1);
        self.allocated_bytes = self.allocated_bytes.saturating_add(bytes.len() as u64);

        pointer
    }

    /// Allocate one raw pointer id from the intrusive free list or the unused tail.
    fn allocate_pointer_id(&mut self) -> u64 {
        if let Some(pointer_id) = self.free_pointer_ids.pop() {
            pointer_id
        } else {
            let pointer_id = self.next_unused_pointer_id;
            self.next_unused_pointer_id = self.next_unused_pointer_id.saturating_add(1);

            pointer_id
        }
    }

    /// Store one live raw pointer record by pointer id.
    fn store_pointer(&mut self, pointer_id: u64, record: RawPointerRecord) {
        let index = pointer_id.saturating_sub(1) as usize;

        // append at the unused tail when possible
        if index == self.pointers.len() {
            self.pointers.push(record);
        }
        // otherwise overwrite one recycled slot
        else if let Some(entry) = self.pointers.get_mut(index) {
            *entry = record;
        }
    }

    /// Allocate one raw storage location for the given payload.
    fn allocate_location(&mut self, bytes: &[u8]) -> RawLocation {
        // prefer one small slot first
        if let Some(slot) = self.allocate_small_slot(bytes) {
            return RawLocation::Small(slot);
        }

        // otherwise allocate one dedicated large-space allocation
        let pages = self.arena.allocate_bytes(bytes);
        let allocation_id = self.allocate_allocation_slot(bytes.len(), pages);

        RawLocation::Large(allocation_id)
    }

    /// Allocate one raw allocation slot in large space and return its stable id.
    pub(super) fn allocate_allocation_slot(&mut self, len: usize, pages: PageMap) -> AllocationId {
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
        };
        let index = allocation_id.saturating_sub(1) as usize;

        // append at the unused tail when possible
        if index == self.large.allocations.len() {
            self.large.allocations.push(allocation);
        }
        // otherwise overwrite one recycled slot
        else if let Some(existing) = self.large.allocations.get_mut(index) {
            *existing = allocation;
        }

        AllocationId::new(allocation_id)
    }

    /// Allocate one raw small slot if the payload fits one configured class.
    pub(super) fn allocate_small_slot(&mut self, bytes: &[u8]) -> Option<SpanSlot> {
        // resolve the matching size class first
        let class_index = self.small.size_classes.class_index_for(bytes.len())?;
        let size_class = self.small.size_classes.classes[class_index].bytes;
        let span_index = self.allocate_small_span(class_index, size_class);
        let span = self.small.spans.get_mut(span_index)?;
        let slot_index = span.next_free_slot;
        let slot_offset = span.size_class.saturating_mul(slot_index);
        let is_written = self.arena.set_bytes(&mut span.pages, slot_offset, bytes);

        debug_assert!(is_written);

        // mark the slot as live inside its span
        span.occupied.set(slot_index);
        span.lengths[slot_index] = bytes.len() as u16;
        span.occupied_count = span.occupied_count.saturating_add(1);
        span.next_free_slot = span
            .occupied
            .first_clear_from(slot_index)
            .unwrap_or(span.slot_count);

        // requeue the span if it still has capacity
        if span.occupied_count < span.slot_count {
            self.small.available_spans[class_index].push(span_index);
        }

        Some(SpanSlot::new(span_index, slot_index))
    }

    /// Allocate or reuse one non-full raw span for the given size class.
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
            lengths: vec![0; slot_count].into_boxed_slice(),
            occupied: crate::Bitmap::with_capacity(slot_count),
            pages: self.arena.allocate_zeroed(self.small.span_bytes),
        };
        let span_index = self.small.spans.len();

        self.small.spans.push(span);

        span_index
    }

    /// Release one raw small slot without releasing its stable pointer id.
    pub(super) fn release_small_slot(&mut self, slot: SpanSlot) {
        let Some(span) = self.span_mut(slot.span_index()) else {
            return;
        };
        let slot_index = slot.slot_index();
        let was_full = span.occupied_count == span.slot_count;
        let size_class = span.size_class;

        if !span.occupied.contains(slot_index) {
            return;
        }

        span.occupied.clear(slot_index);
        span.lengths[slot_index] = 0;
        span.occupied_count = span.occupied_count.saturating_sub(1);
        span.next_free_slot = span.next_free_slot.min(slot_index);

        let should_requeue = was_full && span.occupied_count < span.slot_count;

        if should_requeue
            && let Some(class_index) = self.small.size_classes.class_index_for(size_class)
        {
            self.small.available_spans[class_index].push(slot.span_index());
        }
    }
}
