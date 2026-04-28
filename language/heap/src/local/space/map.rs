use super::{HeapLocation, HeapPageMapEntry, HeapPlace, HeapSpace, LargeAllocationId};
use crate::HeapReference;
use crate::allocator::{PageRun, SpanSlot};

impl HeapSpace {
    /// Return the page-map entry for one logical page.
    pub(crate) fn page_entry(&self, page_index: usize) -> Option<HeapPageMapEntry> {
        self.page_map.get(page_index).copied().flatten()
    }

    /// Record one page-map entry for every page in one logical page run.
    pub(crate) fn map_page_run(
        &mut self,
        first_offset: usize,
        page_run: &PageRun,
        mut entry: impl FnMut(usize) -> HeapPageMapEntry,
    ) {
        let first_page_index = first_offset / self.allocator.page_bytes();

        for logical_page_index in 0..page_run.len() {
            let page_index = first_page_index + logical_page_index;

            // grow the sparse logical page map to the touched page
            if self.page_map.len() <= page_index {
                self.page_map.resize(page_index + 1, None);
            }

            self.page_map[page_index] = Some(entry(logical_page_index));
        }
    }

    /// Clear every page-map entry for one logical page run.
    pub(crate) fn unmap_page_run(&mut self, first_offset: usize, page_run: &PageRun) {
        let first_page_index = first_offset / self.allocator.page_bytes();

        for logical_page_index in 0..page_run.len() {
            let page_index = first_page_index + logical_page_index;

            // sparse trailing pages may never have been mapped
            if let Some(entry) = self.page_map.get_mut(page_index) {
                *entry = None;
            }
        }
    }

    /// Return the resolved location for one live heap reference.
    pub(crate) fn resolve_location(&self, reference: HeapReference) -> Option<HeapLocation> {
        let page_bytes = self.allocator.page_bytes();
        let page_index = reference.offset() / page_bytes;
        let page_offset = reference.offset() % page_bytes;
        let entry = self.page_entry(page_index)?;

        match entry {
            HeapPageMapEntry::Young { logical_page_index } => {
                self.resolve_young_location(logical_page_index, page_offset)
            }
            HeapPageMapEntry::Small {
                span_index,
                logical_page_index,
            } => {
                self.resolve_small_location(reference, span_index, logical_page_index, page_offset)
            }
            HeapPageMapEntry::Large {
                allocation_id,
                logical_page_index,
            } => self.resolve_large_location(
                reference,
                allocation_id,
                logical_page_index,
                page_offset,
            ),
        }
    }

    /// Return the resolved young-space location for one live heap reference.
    fn resolve_young_location(
        &self,
        logical_page_index: usize,
        page_offset: usize,
    ) -> Option<HeapLocation> {
        let logical_byte_offset = logical_page_index * self.young.page_bytes + page_offset;
        let range_end = self
            .young
            .ranges
            .partition_point(|range| range.first_offset <= logical_byte_offset);

        // young ranges are bump ordered, so address resolution is predecessor lookup
        if range_end == 0 {
            return None;
        }

        let range_index = range_end - 1;
        let allocation = self.young.ranges.get(range_index)?;

        // retired young ranges stay addressable only until the next reset
        if !self.young.live.contains(range_index) {
            return None;
        }

        let allocation_offset = allocation.first_offset;
        let allocation_limit = allocation_offset + allocation.byte_len;
        if logical_byte_offset >= allocation_limit {
            return None;
        }

        let byte_offset = logical_byte_offset - allocation_offset;

        Some(HeapLocation {
            place: HeapPlace::Young {
                first_offset: allocation_offset,
            },
            base: HeapReference::new(allocation_offset),
            byte_offset,
            byte_len: allocation.byte_len,
        })
    }

    /// Return the resolved small-span location for one live heap reference.
    fn resolve_small_location(
        &self,
        reference: HeapReference,
        span_index: usize,
        logical_page_index: usize,
        page_offset: usize,
    ) -> Option<HeapLocation> {
        let span = self.span(span_index)?;
        let logical_byte_offset = logical_page_index * self.allocator.page_bytes() + page_offset;
        let slot_index = logical_byte_offset / span.class.size_class;
        let slot_offset = logical_byte_offset % span.class.size_class;
        if slot_index >= span.slot_count || !span.occupied.contains(slot_index) {
            return None;
        }

        let byte_len = span.class.size_class;
        if slot_offset >= byte_len {
            return None;
        }

        let slot_base_offset = slot_index * span.class.size_class;
        let base_offset = span.first_offset + slot_base_offset;
        let slot = SpanSlot::new(span_index, slot_index).ok()?;

        debug_assert_eq!(reference.offset(), base_offset + slot_offset);

        Some(HeapLocation {
            place: HeapPlace::Small(slot),
            base: HeapReference::new(base_offset),
            byte_offset: slot_offset,
            byte_len,
        })
    }

    /// Return the resolved large-allocation location for one live heap reference.
    fn resolve_large_location(
        &self,
        reference: HeapReference,
        allocation_id: LargeAllocationId,
        logical_page_index: usize,
        page_offset: usize,
    ) -> Option<HeapLocation> {
        let allocation = self.large_allocation(allocation_id)?;
        let logical_byte_offset = logical_page_index * self.allocator.page_bytes() + page_offset;
        if allocation.len == 0 {
            if logical_byte_offset != 0 {
                return None;
            }
        } else if logical_byte_offset >= allocation.len {
            return None;
        }

        debug_assert_eq!(
            reference.offset(),
            allocation.first_offset + logical_byte_offset
        );

        Some(HeapLocation {
            place: HeapPlace::Large(allocation_id),
            base: HeapReference::new(allocation.first_offset),
            byte_offset: logical_byte_offset,
            byte_len: allocation.len,
        })
    }
}
