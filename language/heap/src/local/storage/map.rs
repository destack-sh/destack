use super::{HeapExtent, HeapPlace, HeapStorage, LargeBlockId, PageOwner};
use crate::{HeapReference, Slot};
use tspp_memory::MemoryRange;

impl HeapStorage {
    /// Return the owner of one logical page.
    pub(crate) fn page_owner(&self, page_index: usize) -> Option<PageOwner> {
        self.page_table.get(page_index).copied()
    }

    /// Record the owner of every page in one logical page span.
    pub(crate) fn map_page_span(
        &mut self,
        page_span: &MemoryRange,
        mut owner: impl FnMut(usize) -> PageOwner,
    ) {
        let first_page_index = page_span.offset / self.page_size_bytes();

        let page_count = page_span.byte_len / self.page_size_bytes();
        for logical_page_index in 0..page_count {
            let page_index = first_page_index + logical_page_index;

            self.page_table.set(page_index, owner(logical_page_index));
        }
    }

    /// Clear the owner of every page in one logical page span.
    pub(crate) fn unmap_page_span(&mut self, page_span: &MemoryRange) {
        let first_page_index = page_span.offset / self.page_size_bytes();

        let page_count = page_span.byte_len / self.page_size_bytes();
        for logical_page_index in 0..page_count {
            let page_index = first_page_index + logical_page_index;

            self.page_table.clear(page_index);
        }
    }

    /// Return whether one reference addresses constant storage, which tracing skips.
    pub(crate) fn is_constant(&self, reference: HeapReference) -> bool {
        let offset = reference.offset();

        self.constant.byte_len != 0
            && offset >= self.constant.offset
            && offset < self.constant.offset + self.constant.byte_len
    }

    /// Return the resolved extent for one live heap reference.
    pub(crate) fn resolve_extent(&self, reference: HeapReference) -> Option<HeapExtent> {
        let page_size_bytes = self.page_size_bytes();
        let page_index = reference.offset() / page_size_bytes;
        let page_offset = reference.offset() % page_size_bytes;
        let owner = self.page_owner(page_index)?;

        match owner {
            PageOwner::Span {
                span_index,
                logical_page_index,
            } => self.resolve_slot_extent(reference, span_index, logical_page_index, page_offset),
            PageOwner::LargeBlock {
                block_id,
                logical_page_index,
            } => self.resolve_large_extent(reference, block_id, logical_page_index, page_offset),
        }
    }

    /// Return the resolved span slot extent for one live heap reference.
    fn resolve_slot_extent(
        &self,
        reference: HeapReference,
        span_index: usize,
        logical_page_index: usize,
        page_offset: usize,
    ) -> Option<HeapExtent> {
        let span = self.span(span_index)?;
        let logical_byte_offset = logical_page_index * self.page_size_bytes() + page_offset;
        let slot_index = logical_byte_offset / span.class.size_class();
        let slot_offset = logical_byte_offset % span.class.size_class();
        if slot_index >= span.slot_count || !span.occupied.contains(slot_index) {
            return None;
        }

        let byte_len = span.class.size_class();
        if slot_offset >= byte_len {
            return None;
        }

        let base_offset = span.slot_offset(slot_index);
        let slot = Slot::new(span_index, slot_index).ok()?;

        debug_assert_eq!(reference.offset(), base_offset + slot_offset);

        Some(HeapExtent {
            place: HeapPlace::Slot(slot),
            base: HeapReference::new(base_offset),
            byte_offset: slot_offset,
            byte_len,
        })
    }

    /// Return the resolved large-block extent for one live heap reference.
    fn resolve_large_extent(
        &self,
        reference: HeapReference,
        block_id: LargeBlockId,
        logical_page_index: usize,
        page_offset: usize,
    ) -> Option<HeapExtent> {
        let block = self.large_block(block_id)?;
        let logical_byte_offset = logical_page_index * self.page_size_bytes() + page_offset;
        if block.byte_len == 0 {
            if logical_byte_offset != 0 {
                return None;
            }
        } else if logical_byte_offset >= block.byte_len {
            return None;
        }

        debug_assert_eq!(reference.offset(), block.first_offset + logical_byte_offset);

        Some(HeapExtent {
            place: HeapPlace::LargeBlock(block_id),
            base: HeapReference::new(block.first_offset),
            byte_offset: logical_byte_offset,
            byte_len: block.byte_len,
        })
    }
}
