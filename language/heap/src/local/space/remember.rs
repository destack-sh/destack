use destack_mir::ReferenceMap;

use super::{HeapSpace, LargeAllocationId};
use crate::{HeapError, HeapResult, overlaps_heap_range, overlaps_shared_range};

impl HeapSpace {
    /// Rebuild the mature remembered set conservatively.
    pub(crate) fn rebuild_remembered_set(&mut self) -> HeapResult<()> {
        self.dirty_spans.clear();
        self.dirty_large_allocations.clear();

        // conservatively dirty every mature span slot with heap edges
        for span_index in 0..self.small.spans.len() {
            let Some(span) = self.span(span_index) else {
                continue;
            };
            let occupied = span.occupied.clone();
            let size_class = span.class.size_class;

            for slot_index in 0..span.slot_count {
                if !occupied.contains(slot_index) {
                    continue;
                }

                let reference_map = self.small_slot_reference_map(span_index, slot_index)?;
                if !reference_map.has_reference() {
                    continue;
                }

                self.mark_span_slot_dirty(span_index, slot_index, 0, size_class)?;
            }
        }

        // conservatively dirty every mature large allocation with heap edges
        for allocation_index in 0..self.large.allocations.len() {
            let allocation_id = LargeAllocationId::new(allocation_index as u64 + 1);
            let Some(allocation) = self.large_allocation(allocation_id) else {
                continue;
            };
            if !allocation.reference_map.has_reference() {
                continue;
            }

            self.mark_large_allocation_dirty(allocation_id, 0, allocation.len)?;
        }

        Ok(())
    }

    /// Remember one mature heap span write if it may touch references.
    pub(crate) fn mark_span_slot_dirty(
        &mut self,
        span_index: usize,
        slot_index: usize,
        byte_offset: usize,
        byte_len: usize,
    ) -> HeapResult<()> {
        let Some(span) = self.span(span_index) else {
            return Err(HeapError::MissingSpan { span_index });
        };
        if !span.occupied.contains(slot_index) {
            return Err(HeapError::MissingSmallSlot {
                span_index,
                slot_index,
            });
        }

        let reference_map = self.small_slot_reference_map(span_index, slot_index)?;
        let is_overlapping = overlaps_heap_range(&reference_map, byte_offset, byte_len)?;
        if !is_overlapping {
            return Ok(());
        }

        let slot_offset = span.class.size_class * slot_index;
        let mut should_queue = false;

        // mark the overlapping card range on the owning span
        if let Some(span) = self.span_mut(span_index) {
            let dirty_start = slot_offset + byte_offset;
            span.dirty_cards.mark_range(dirty_start, byte_len);
            if !span.is_dirty_queued {
                span.is_dirty_queued = true;
                should_queue = true;
            }
        }

        // queue the owning span once for the next minor collection
        if should_queue {
            self.dirty_spans.push(span_index);
        }

        Ok(())
    }

    /// Remember one mature heap large-allocation write if it may touch references.
    pub(crate) fn mark_large_allocation_dirty(
        &mut self,
        allocation_id: LargeAllocationId,
        byte_offset: usize,
        byte_len: usize,
    ) -> HeapResult<()> {
        let Some(allocation) = self.large_allocation(allocation_id) else {
            return Err(HeapError::MissingLargeAllocation {
                allocation_id: allocation_id.id(),
            });
        };
        let is_overlapping = overlaps_heap_range(&allocation.reference_map, byte_offset, byte_len)?;
        if !is_overlapping {
            return Ok(());
        }

        let mut should_queue = false;

        // mark the overlapping card range on the owning allocation
        if let Some(allocation) = self.large_allocation_mut(allocation_id) {
            allocation.dirty_cards.mark_range(byte_offset, byte_len);

            if !allocation.is_dirty_queued {
                allocation.is_dirty_queued = true;
                should_queue = true;
            }
        }

        // queue the owning allocation once for the next minor collection
        if should_queue {
            self.dirty_large_allocations.push(allocation_id);
        }

        Ok(())
    }

    /// Return whether one local write range may overlap shared heap roots.
    pub(crate) fn overlaps_shared_roots(
        &self,
        reference_map: &ReferenceMap,
        byte_offset: usize,
        byte_len: usize,
    ) -> HeapResult<bool> {
        overlaps_shared_range(reference_map, byte_offset, byte_len)
    }
}
