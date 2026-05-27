use super::{
    SharedHeapLocation, SharedHeapPageMapEntry, SharedHeapPlace, SharedHeapSpace, SharedHeapState,
    SharedLargeAllocationId,
};
use crate::SharedHeapReference;
use crate::allocator::{PageRun, SpanSlot};

impl SharedHeapSpace {
    /// Return the page-map entry for one logical page.
    fn page_entry(
        &self,
        store: &SharedHeapState,
        page_index: usize,
    ) -> Option<SharedHeapPageMapEntry> {
        store.page_map.get(page_index).copied().flatten()
    }

    /// Record one page-map entry for every page in one logical page run.
    pub(super) fn map_page_run(
        &self,
        store: &mut SharedHeapState,
        first_offset: usize,
        page_run: &PageRun,
        mut entry: impl FnMut(usize) -> SharedHeapPageMapEntry,
    ) {
        let first_page_index = first_offset / self.allocator.page_bytes();

        for logical_page_index in 0..page_run.len() {
            let page_index = first_page_index + logical_page_index;

            // grow the sparse logical page map to the touched page
            if store.page_map.len() <= page_index {
                store.page_map.resize(page_index + 1, None);
            }

            store.page_map[page_index] = Some(entry(logical_page_index));
        }
    }

    /// Clear every page-map entry for one logical page run.
    pub(crate) fn unmap_page_run(
        &self,
        store: &mut SharedHeapState,
        first_offset: usize,
        page_run: &PageRun,
    ) {
        let first_page_index = first_offset / self.allocator.page_bytes();

        for logical_page_index in 0..page_run.len() {
            let page_index = first_page_index + logical_page_index;

            // sparse trailing pages may never have been mapped
            if let Some(entry) = store.page_map.get_mut(page_index) {
                *entry = None;
            }
        }
    }

    /// Return the resolved location for one live shared heap reference.
    pub(crate) fn resolve_location(
        &self,
        reference: SharedHeapReference,
    ) -> Option<SharedHeapLocation> {
        let page_bytes = self.allocator.page_bytes();
        let page_index = reference.offset() / page_bytes;
        let page_offset = reference.offset() % page_bytes;
        let store = self.state.read();
        let entry = self.page_entry(&store, page_index)?;

        match entry {
            SharedHeapPageMapEntry::Small {
                span_index,
                logical_page_index,
            } => self.resolve_small_location(&store, span_index, logical_page_index, page_offset),
            SharedHeapPageMapEntry::Large {
                allocation_id,
                logical_page_index,
            } => {
                self.resolve_large_location(&store, allocation_id, logical_page_index, page_offset)
            }
        }
    }

    /// Return the resolved small-span location for one live shared heap reference.
    fn resolve_small_location(
        &self,
        store: &SharedHeapState,
        span_index: usize,
        logical_page_index: usize,
        page_offset: usize,
    ) -> Option<SharedHeapLocation> {
        let span = store.small.spans.get(span_index)?.clone();
        let logical_byte_offset = logical_page_index * self.allocator.page_bytes() + page_offset;
        let slot_index = logical_byte_offset / span.class.size_class;
        let slot_offset = logical_byte_offset % span.class.size_class;

        // reject empty or free slots
        if slot_index >= span.slot_count || !span.contains_slot(slot_index) {
            return None;
        }

        let byte_len = span.class.size_class;
        if slot_offset >= byte_len {
            return None;
        }

        let slot_base_offset = slot_index * span.class.size_class;
        let base_offset = span.first_offset + slot_base_offset;
        let slot = SpanSlot::new(span_index, slot_index).ok()?;

        Some(SharedHeapLocation {
            place: SharedHeapPlace::Small(slot),
            base: SharedHeapReference::new(base_offset),
            byte_offset: slot_offset,
            byte_len,
        })
    }

    /// Return the resolved large-allocation location for one live shared heap reference.
    fn resolve_large_location(
        &self,
        store: &SharedHeapState,
        allocation_id: SharedLargeAllocationId,
        logical_page_index: usize,
        page_offset: usize,
    ) -> Option<SharedHeapLocation> {
        let allocation = store
            .large
            .allocations
            .get(allocation_id.index().ok()?)?
            .clone();
        let allocation = allocation.read();
        if !allocation.is_live {
            return None;
        }

        let logical_byte_offset = logical_page_index * self.allocator.page_bytes() + page_offset;
        if allocation.byte_len == 0 {
            if logical_byte_offset != 0 {
                return None;
            }
        } else if logical_byte_offset >= allocation.byte_len {
            return None;
        }

        Some(SharedHeapLocation {
            place: SharedHeapPlace::Large(allocation_id),
            base: SharedHeapReference::new(allocation.first_offset),
            byte_offset: logical_byte_offset,
            byte_len: allocation.byte_len,
        })
    }
}
