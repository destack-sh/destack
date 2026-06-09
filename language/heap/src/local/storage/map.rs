use super::{HeapExtent, HeapPageMapEntry, HeapPlace, HeapStorage, LargeBlockId};
use crate::HeapReference;
use crate::allocator::{PageSpan, Slot};

impl HeapStorage {
    /// Return the page-map entry for one logical page.
    pub(crate) fn page_entry(&self, page_index: usize) -> Option<HeapPageMapEntry> {
        self.page_map.get(page_index).copied().flatten()
    }

    /// Record one page-map entry for every page in one logical page span.
    pub(crate) fn map_page_span(
        &mut self,
        first_offset: usize,
        page_span: &PageSpan,
        mut entry: impl FnMut(usize) -> HeapPageMapEntry,
    ) {
        let first_page_index = first_offset / self.allocator.page_size_bytes();

        for logical_page_index in 0..page_span.len() {
            let page_index = first_page_index + logical_page_index;

            // grow the sparse logical page map to the touched page
            if self.page_map.len() <= page_index {
                self.page_map.resize(page_index + 1, None);
            }

            self.page_map[page_index] = Some(entry(logical_page_index));
        }
    }

    /// Clear every page-map entry for one logical page span.
    pub(crate) fn unmap_page_span(&mut self, first_offset: usize, page_span: &PageSpan) {
        let first_page_index = first_offset / self.allocator.page_size_bytes();

        for logical_page_index in 0..page_span.len() {
            let page_index = first_page_index + logical_page_index;

            // sparse trailing pages may never have been mapped
            if let Some(entry) = self.page_map.get_mut(page_index) {
                *entry = None;
            }
        }
    }

    /// Return the resolved extent for one live heap reference.
    pub(crate) fn resolve_extent(&self, reference: HeapReference) -> Option<HeapExtent> {
        let page_size_bytes = self.allocator.page_size_bytes();
        let page_index = reference.offset() / page_size_bytes;
        let page_offset = reference.offset() % page_size_bytes;
        let entry = self.page_entry(page_index)?;

        match entry {
            HeapPageMapEntry::Young { logical_page_index } => {
                self.resolve_young_extent(logical_page_index, page_offset)
            }
            HeapPageMapEntry::MatureSpan {
                span_index,
                logical_page_index,
            } => self.resolve_small_extent(reference, span_index, logical_page_index, page_offset),
            HeapPageMapEntry::LargeBlock {
                block_id,
                logical_page_index,
            } => self.resolve_large_extent(reference, block_id, logical_page_index, page_offset),
        }
    }

    /// Return the resolved young space extent for one live heap reference.
    fn resolve_young_extent(
        &self,
        logical_page_index: usize,
        page_offset: usize,
    ) -> Option<HeapExtent> {
        let logical_byte_offset = logical_page_index * self.young.page_size_bytes + page_offset;

        if let Some(span_index) = self
            .young
            .page_spans
            .get(logical_page_index)
            .copied()
            .flatten()
        {
            return self.resolve_young_span_extent(span_index, logical_byte_offset);
        }

        let range = self.young.range_at_offset(logical_byte_offset)?;
        let block_offset = range.range.first_offset;

        let byte_offset = logical_byte_offset - block_offset;

        Some(HeapExtent {
            storage: HeapPlace::YoungRange {
                first_offset: block_offset,
            },
            base: HeapReference::new(block_offset),
            byte_offset,
            byte_len: range.range.byte_len,
        })
    }

    /// Return the resolved fixed-size young extent for one live heap reference.
    fn resolve_young_span_extent(
        &self,
        span_index: usize,
        logical_byte_offset: usize,
    ) -> Option<HeapExtent> {
        let span = self.young.span(span_index)?;
        let span_offset = logical_byte_offset.checked_sub(span.first_offset)?;
        let slot_index = span_offset / span.class.size_class;
        let slot_offset = span_offset % span.class.size_class;
        let bits = self.young.span_bits(span_index)?;
        if slot_index >= self.young.span_reserved_slot_count(span_index)?
            || bits.freed.contains(slot_index)
        {
            return None;
        }

        let base_offset = span.slot_offset(slot_index);
        let slot = Slot::new(span_index, slot_index).ok()?;

        Some(HeapExtent {
            storage: HeapPlace::YoungSlot(slot),
            base: HeapReference::new(base_offset),
            byte_offset: slot_offset,
            byte_len: span.byte_len(),
        })
    }

    /// Return the resolved small-span extent for one live heap reference.
    fn resolve_small_extent(
        &self,
        reference: HeapReference,
        span_index: usize,
        logical_page_index: usize,
        page_offset: usize,
    ) -> Option<HeapExtent> {
        let span = self.span(span_index)?;
        let logical_byte_offset =
            logical_page_index * self.allocator.page_size_bytes() + page_offset;
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
        let slot = Slot::new(span_index, slot_index).ok()?;

        debug_assert_eq!(reference.offset(), base_offset + slot_offset);

        Some(HeapExtent {
            storage: HeapPlace::MatureSlot(slot),
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
        let logical_byte_offset =
            logical_page_index * self.allocator.page_size_bytes() + page_offset;
        if block.byte_len == 0 {
            if logical_byte_offset != 0 {
                return None;
            }
        } else if logical_byte_offset >= block.byte_len {
            return None;
        }

        debug_assert_eq!(reference.offset(), block.first_offset + logical_byte_offset);

        Some(HeapExtent {
            storage: HeapPlace::LargeBlock(block_id),
            base: HeapReference::new(block.first_offset),
            byte_offset: logical_byte_offset,
            byte_len: block.byte_len,
        })
    }
}
