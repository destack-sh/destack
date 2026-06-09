use destack_mir::{TraceMap, TraceTable};

use super::{HeapStorage, LargeBlockId, Phase};
use crate::local::gc::DirtyExtent;
use crate::{HeapError, HeapResult, overlaps_heap_range, overlaps_shared_range};

impl HeapStorage {
    /// Rebuild the mature remembered set conservatively.
    pub(crate) fn rebuild_remembered_set(&mut self, trace_table: &TraceTable) -> HeapResult<()> {
        self.clear_remembered_set();

        // conservatively dirty every mature span slot with heap references
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

                let trace_map = self.small_slot_trace_map(span_index, slot_index, trace_table)?;
                if !trace_map.has_reference() {
                    continue;
                }

                self.mark_span_slot_dirty(span_index, slot_index, 0, size_class, &trace_map)?;
            }
        }

        // conservatively dirty every mature large block with heap references
        for block_index in 0..self.large.blocks.len() {
            let block_id = LargeBlockId::new(block_index as u64 + 1);
            let Some(block) = self.large_block(block_id) else {
                continue;
            };
            if !block.trace_map.has_reference() {
                continue;
            }

            self.mark_large_block_dirty(block_id, 0, block.byte_len)?;
        }

        Ok(())
    }

    /// Clear every mature remembered-set entry.
    fn clear_remembered_set(&mut self) {
        self.collector.dirty_extents.clear();
        self.collector.young_dirty_extent_cursor = 0;
        self.collector.young_dirty_card_cursor = 0;

        for span_index in 0..self.small.spans.len() {
            let Some(span) = self.small.spans.get_mut(span_index) else {
                continue;
            };

            span.dirty_cards.clear();
            span.is_dirty_queued = false;
        }

        for block_index in 0..self.large.blocks.len() {
            let Some(block) = self.large.blocks.get_mut(block_index) else {
                continue;
            };

            block.dirty_cards.clear();
            block.is_dirty_queued = false;
        }
    }

    /// Remember one mature heap span write if it may touch references.
    pub(crate) fn mark_span_slot_dirty(
        &mut self,
        span_index: usize,
        slot_index: usize,
        byte_offset: usize,
        byte_len: usize,
        trace_map: &TraceMap,
    ) -> HeapResult<()> {
        // skip writes that cannot touch local references
        let is_overlapping = overlaps_heap_range(trace_map, byte_offset, byte_len);
        if !is_overlapping {
            return Ok(());
        }

        self.remember_span_slot_write(span_index, slot_index, byte_offset, byte_len)
    }

    /// Remember one overlapping mature heap span write.
    pub(crate) fn remember_span_slot_write(
        &mut self,
        span_index: usize,
        slot_index: usize,
        byte_offset: usize,
        byte_len: usize,
    ) -> HeapResult<()> {
        let Some(span) = self.span(span_index) else {
            return Err(HeapError::internal("missing span"));
        };
        if !span.occupied.contains(slot_index) {
            return Err(HeapError::internal("missing small slot"));
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
            self.collector
                .dirty_extents
                .push(DirtyExtent::Span(span_index));
        }

        self.request_minor_dirty_rescan(should_queue);

        Ok(())
    }

    /// Remember one mature heap large-block write if it may touch references.
    pub(crate) fn mark_large_block_dirty(
        &mut self,
        block_id: LargeBlockId,
        byte_offset: usize,
        byte_len: usize,
    ) -> HeapResult<()> {
        let Some(block) = self.large_block(block_id) else {
            return Err(HeapError::internal("missing large block"));
        };
        let is_overlapping = overlaps_heap_range(&block.trace_map, byte_offset, byte_len);
        if !is_overlapping {
            return Ok(());
        }

        let mut should_queue = false;

        // mark the overlapping card range on the owning block
        if let Some(block) = self.large_block_mut(block_id) {
            block.dirty_cards.mark_range(byte_offset, byte_len);

            if !block.is_dirty_queued {
                block.is_dirty_queued = true;
                should_queue = true;
            }
        }

        // queue the owning block once for the next minor collection
        if should_queue {
            self.collector
                .dirty_extents
                .push(DirtyExtent::Large(block_id));
        }

        self.request_minor_dirty_rescan(should_queue);

        Ok(())
    }

    /// Request one remembered-set rescan from the active minor cycle.
    fn request_minor_dirty_rescan(&mut self, is_newly_queued: bool) {
        // inactive cycles scan everything at their next start
        if self.collector.minor_phase == Phase::Idle {
            return;
        }

        // writes to already queued extents may land behind the scan cursor
        if !is_newly_queued {
            self.collector.dirty_rescan_needed = true;
        }

        // sweeping resumes marking until the remembered set is quiet
        if self.collector.minor_phase == Phase::Sweep {
            self.collector.minor_phase = Phase::Mark;
        }
    }

    /// Return whether one local write range may overlap shared heap roots.
    pub(crate) fn overlaps_shared_roots(
        &self,
        trace_map: &TraceMap,
        byte_offset: usize,
        byte_len: usize,
    ) -> bool {
        overlaps_shared_range(trace_map, byte_offset, byte_len)
    }
}
