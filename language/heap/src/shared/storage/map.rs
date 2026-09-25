use super::{HeapExtent, HeapPlace, HeapState, HeapStorage, LargeBlockId, PageOwner};
use crate::{SharedHeapReference, Slot};
use tspp_memory::MemoryRange;

impl HeapStorage {
    /// Return the owner of one logical page.
    fn page_owner(&self, store: &HeapState, page_index: usize) -> Option<PageOwner> {
        store.page_table.get(page_index).copied()
    }

    /// Record the owner of every page in one logical page span.
    pub(super) fn map_page_span(
        &self,
        store: &mut HeapState,
        page_span: &MemoryRange,
        mut owner: impl FnMut(usize) -> PageOwner,
    ) {
        let first_page_index = page_span.offset / self.page_size_bytes();

        let page_count = page_span.byte_len / self.page_size_bytes();
        for logical_page_index in 0..page_count {
            let page_index = first_page_index + logical_page_index;

            store.page_table.set(page_index, owner(logical_page_index));
        }
    }

    /// Clear the owner of every page in one logical page span.
    pub(crate) fn unmap_page_span(&self, store: &mut HeapState, page_span: &MemoryRange) {
        let first_page_index = page_span.offset / self.page_size_bytes();

        let page_count = page_span.byte_len / self.page_size_bytes();
        for logical_page_index in 0..page_count {
            let page_index = first_page_index + logical_page_index;

            store.page_table.clear(page_index);
        }
    }

    /// Return whether one reference addresses constant storage, which tracing skips.
    pub(crate) fn is_constant(&self, reference: SharedHeapReference) -> bool {
        let offset = reference.offset();

        self.constant.byte_len != 0
            && offset >= self.constant.offset
            && offset < self.constant.offset + self.constant.byte_len
    }

    /// Return the resolved extent for one live shared heap reference.
    pub(crate) fn resolve_extent(&self, reference: SharedHeapReference) -> Option<HeapExtent> {
        let page_size_bytes = self.page_size_bytes();
        let page_index = reference.offset() / page_size_bytes;
        let page_offset = reference.offset() % page_size_bytes;
        let store = self.state.read();
        let owner = self.page_owner(&store, page_index)?;

        match owner {
            PageOwner::SmallSpan {
                span_index,
                logical_page_index,
            } => self.resolve_small_extent(&store, span_index, logical_page_index, page_offset),
            PageOwner::LargeBlock {
                block_id,
                logical_page_index,
            } => self.resolve_large_extent(&store, block_id, logical_page_index, page_offset),
        }
    }

    /// Return the resolved small-span extent for one live shared heap reference.
    fn resolve_small_extent(
        &self,
        store: &HeapState,
        span_index: usize,
        logical_page_index: usize,
        page_offset: usize,
    ) -> Option<HeapExtent> {
        let span = store.small.spans.get(span_index)?;
        let logical_byte_offset = logical_page_index * self.page_size_bytes() + page_offset;
        let slot_index = logical_byte_offset / span.class.size_class();
        let slot_offset = logical_byte_offset % span.class.size_class();

        // reject empty or free slots
        if slot_index >= span.slot_count || !span.contains_slot(slot_index) {
            return None;
        }

        let byte_len = span.class.size_class();
        if slot_offset >= byte_len {
            return None;
        }

        let slot_base_offset = slot_index * span.class.size_class();
        let base_offset = span.first_offset + slot_base_offset;
        let slot = Slot::new(span_index, slot_index).ok()?;

        Some(HeapExtent {
            place: HeapPlace::SmallSlot(slot),
            base: SharedHeapReference::new(base_offset),
            byte_offset: slot_offset,
            byte_len,
        })
    }

    /// Return the resolved large-block extent for one live shared heap reference.
    fn resolve_large_extent(
        &self,
        store: &HeapState,
        block_id: LargeBlockId,
        logical_page_index: usize,
        page_offset: usize,
    ) -> Option<HeapExtent> {
        let block = store.large.blocks.get(block_id.index().ok()?)?.as_ref()?;
        let block = block.read();

        let logical_byte_offset = logical_page_index * self.page_size_bytes() + page_offset;
        if block.byte_len == 0 {
            if logical_byte_offset != 0 {
                return None;
            }
        } else if logical_byte_offset >= block.byte_len {
            return None;
        }

        Some(HeapExtent {
            place: HeapPlace::LargeBlock(block_id),
            base: SharedHeapReference::new(block.first_offset),
            byte_offset: logical_byte_offset,
            byte_len: block.byte_len,
        })
    }
}
