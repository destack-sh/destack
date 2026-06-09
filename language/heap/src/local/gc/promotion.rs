use destack_mir::{TraceMap, TraceTable};

use crate::allocator::Slot;
use crate::local::gc::{DirtyCard, DirtyExtent};
use crate::local::storage::{HeapExtent, HeapPageMapEntry, HeapPlace, HeapStorage, LargeBlockId};
use crate::{
    HeapError, HeapReference, HeapResult, ReferenceRange, RootSlot, visit_heap_root_slots,
};

/// Young-to-mature forwarding built for one promotion pass.
#[derive(Debug, Default)]
struct ForwardingTable {
    /// Forwarded range blocks keyed by young range index.
    ranges: Vec<RangeForwarding>,
    /// Forwarded span slots keyed by span and slot index.
    slots: Vec<SlotForwarding>,
}

impl ForwardingTable {
    /// Record one promoted young range.
    fn push_range(&mut self, range_index: usize, target: HeapReference) {
        self.ranges.push(RangeForwarding {
            range_index,
            target,
        });
    }

    /// Record one promoted young span slot.
    fn push_slot(&mut self, slot: Slot, target: HeapReference) {
        self.slots.push(SlotForwarding { slot, target });
    }

    /// Return the promoted reference for one young range.
    fn range(&self, range_index: usize) -> Option<HeapReference> {
        self.ranges
            .binary_search_by_key(&range_index, |forwarding| forwarding.range_index)
            .ok()
            .map(|index| self.ranges[index].target)
    }

    /// Return the promoted reference for one young span slot.
    fn slot(&self, slot: Slot) -> Option<HeapReference> {
        self.slots
            .binary_search_by_key(&slot_key(slot), |forwarding| slot_key(forwarding.slot))
            .ok()
            .map(|index| self.slots[index].target)
    }
}

/// One promoted young range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RangeForwarding {
    /// The young range index.
    range_index: usize,
    /// The mature reference for the copied payload.
    target: HeapReference,
}

/// One promoted young fixed-size slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SlotForwarding {
    /// The young span slot.
    slot: Slot,
    /// The mature reference for the copied payload.
    target: HeapReference,
}

/// Dirty span card selected for rewriting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DirtySpanRewrite {
    /// The selected dirty card.
    card: DirtyCard,
    /// The span byte offset within heap storage.
    first_offset: usize,
    /// The span size class in bytes.
    size_class: usize,
    /// The number of slots in the span.
    slot_count: usize,
}

/// Dirty large-block card selected for rewriting.
#[derive(Debug, Clone, PartialEq, Eq)]
struct DirtyLargeRewrite {
    /// The selected dirty card.
    card: DirtyCard,
    /// The block byte offset within heap storage.
    first_offset: usize,
    /// The block byte length.
    byte_len: usize,
    /// The trace map used to scan the block.
    trace_map: TraceMap,
}

/// Return the sortable key for one young span slot.
fn slot_key(slot: Slot) -> (usize, usize) {
    (slot.span_index(), slot.slot_index())
}

impl HeapStorage {
    /// Relocate eligible young survivors and rewrite their visible references.
    pub(super) fn relocate_young_survivors<E>(
        &mut self,
        roots: &mut impl FnMut(&mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>) -> Result<(), E>,
        trace_table: &TraceTable,
    ) -> Result<(), E>
    where
        E: From<HeapError>,
    {
        let mut forwarding = ForwardingTable::default();

        // copy eligible survivors before publishing forwarded references
        self.promote_young_range_survivors(&mut forwarding, trace_table)?;
        self.promote_young_span_survivors(&mut forwarding, trace_table)?;

        // rewrite every visible local reference before the mutator resumes
        self.rewrite_promoted_references(roots, &forwarding, trace_table)?;

        Ok(())
    }

    /// Promote reachable young range blocks.
    fn promote_young_range_survivors(
        &mut self,
        forwarding: &mut ForwardingTable,
        trace_table: &TraceTable,
    ) -> HeapResult<()> {
        let mut start = 0usize;

        while let Some(range_index) = self.young.live.first_set_from(start) {
            start = range_index + 1;

            // only marked survivors can be promoted
            if !self.young.marked.contains(range_index) {
                continue;
            }

            let Some(range) = self.young.range(range_index) else {
                return Err(HeapError::Internal {
                    context: "marked young range missing during promotion",
                });
            };
            let source = HeapReference::new(range.first_offset);

            // pinned young objects keep their stable address
            if self.collector.pins.contains(source) {
                continue;
            }

            let trace_map = self.young_range_trace_map(range.first_offset)?;
            let target_place =
                self.allocate_promoted_payload(range.byte_len, &trace_map, range.first_offset)?;
            let target = self.base_reference(target_place)?;

            // publish forwarding after the copied payload is tracked
            self.publish_promoted_payload(
                target,
                target_place,
                range.byte_len,
                &trace_map,
                trace_table,
            )?;
            forwarding.push_range(range_index, target);
            self.retire_promoted_young_range(range_index, source, range.byte_len);
        }

        Ok(())
    }

    /// Promote reachable fixed-size young span slots.
    fn promote_young_span_survivors(
        &mut self,
        forwarding: &mut ForwardingTable,
        trace_table: &TraceTable,
    ) -> HeapResult<()> {
        self.flush_young_cursor();

        for span_index in 0..self.young.spans.len() {
            let Some(span) = self.young.span(span_index).cloned() else {
                return Err(HeapError::internal("missing span"));
            };
            let reserved_slots = self
                .young
                .span_reserved_slot_count(span_index)
                .unwrap_or_else(|| span.slot_count());

            for slot_index in 0..reserved_slots {
                let slot = Slot::new(span_index, slot_index)?;
                let Some(bits) = self.young.span_bits_mut(span_index) else {
                    return Err(HeapError::internal("missing span"));
                };

                // only marked occupied slots can be promoted
                if bits.freed.contains(slot_index) || !bits.marked.contains(slot_index) {
                    continue;
                }

                let source = HeapReference::new(span.slot_offset(slot_index));

                // pinned young objects keep their stable address
                if self.collector.pins.contains(source) {
                    continue;
                }

                let trace_map =
                    self.trace_map_for_place(HeapPlace::YoungSlot(slot), trace_table)?;
                let target_place = self.allocate_promoted_payload(
                    span.class.size_class,
                    &trace_map,
                    source.offset(),
                )?;
                let target = self.base_reference(target_place)?;

                self.publish_promoted_payload(
                    target,
                    target_place,
                    span.class.size_class,
                    &trace_map,
                    trace_table,
                )?;
                forwarding.push_slot(slot, target);
                self.retire_promoted_young_slot(slot, source, span.class.size_class)?;
            }
        }

        Ok(())
    }

    /// Publish remembered metadata for one promoted mature payload.
    fn publish_promoted_payload(
        &mut self,
        reference: HeapReference,
        storage: HeapPlace,
        byte_len: usize,
        trace_map: &TraceMap,
        trace_table: &TraceTable,
    ) -> HeapResult<()> {
        if trace_map.has_shared_reference() {
            self.collector.track_shared_edge_root(reference);
        }

        self.record_write_barrier(
            reference,
            HeapExtent {
                storage,
                base: reference,
                byte_offset: 0,
                byte_len,
            },
            0,
            byte_len,
            trace_table,
        )
    }

    /// Retire one promoted young range source.
    fn retire_promoted_young_range(
        &mut self,
        range_index: usize,
        reference: HeapReference,
        byte_len: usize,
    ) {
        self.young.live.clear(range_index);
        self.young.marked.clear(range_index);
        self.collector.remove_shared_edge_root(reference);
        self.record_young_free(byte_len);
    }

    /// Retire one promoted fixed-size young span source.
    fn retire_promoted_young_slot(
        &mut self,
        slot: Slot,
        reference: HeapReference,
        byte_len: usize,
    ) -> HeapResult<()> {
        let Some(bits) = self.young.span_bits_mut(slot.span_index()) else {
            return Err(HeapError::internal("missing span"));
        };

        bits.freed.set(slot.slot_index());
        bits.marked.clear(slot.slot_index());
        self.collector.remove_shared_edge_root(reference);
        self.record_young_free(byte_len);

        Ok(())
    }

    /// Rewrite roots and live payloads through completed forwarding metadata.
    fn rewrite_promoted_references<E>(
        &mut self,
        roots: &mut impl FnMut(&mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>) -> Result<(), E>,
        forwarding: &ForwardingTable,
        trace_table: &TraceTable,
    ) -> Result<(), E>
    where
        E: From<HeapError>,
    {
        if forwarding.ranges.is_empty() && forwarding.slots.is_empty() {
            return Ok(());
        }

        // external roots first
        self.rewrite_root_references(roots, forwarding)?;

        // payloads after roots, while forwarding is still live
        self.rewrite_young_payload_references(forwarding)?;
        self.rewrite_remembered_mature_references(forwarding, trace_table)?;

        Ok(())
    }

    /// Rewrite mutable root slots through completed forwarding metadata.
    fn rewrite_root_references<E>(
        &self,
        roots: &mut impl FnMut(&mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>) -> Result<(), E>,
        forwarding: &ForwardingTable,
    ) -> Result<(), E>
    where
        E: From<HeapError>,
    {
        roots(&mut |mut slot: RootSlot<'_>| {
            let Some(reference) = slot.load_heap_reference()? else {
                return Ok(());
            };

            if let Some(next_reference) = self.forwarded_reference(reference, forwarding)? {
                slot.store_heap_reference(next_reference)?;
            }

            Ok(())
        })
    }

    /// Rewrite live young payload references through completed forwarding metadata.
    fn rewrite_young_payload_references(&mut self, forwarding: &ForwardingTable) -> HeapResult<()> {
        let mut start = 0usize;

        while let Some(range_index) = self.young.live.first_set_from(start) {
            start = range_index + 1;

            let Some(range) = self.young.range(range_index) else {
                return Err(HeapError::Internal {
                    context: "live young range missing during rewrite",
                });
            };

            // rewrite only payloads that may contain local references
            let trace_map = self.young_range_trace_map(range.first_offset)?;
            self.rewrite_payload_references(
                range.first_offset,
                range.byte_len,
                &trace_map,
                forwarding,
            )?;
        }

        Ok(())
    }

    /// Rewrite remembered mature references through completed forwarding metadata.
    fn rewrite_remembered_mature_references(
        &mut self,
        forwarding: &ForwardingTable,
        trace_table: &TraceTable,
    ) -> HeapResult<()> {
        let dirty_region_count = self.collector.dirty_extents.len();

        for dirty_index in 0..dirty_region_count {
            match self.collector.dirty_extents[dirty_index] {
                DirtyExtent::Span(span_index) => {
                    self.rewrite_dirty_span_references(span_index, forwarding, trace_table)?;
                }
                DirtyExtent::Large(block_id) => {
                    self.rewrite_dirty_large_references(block_id, forwarding)?;
                }
            }
        }

        Ok(())
    }

    /// Rewrite every dirty card on one mature span.
    fn rewrite_dirty_span_references(
        &mut self,
        span_index: usize,
        forwarding: &ForwardingTable,
        trace_table: &TraceTable,
    ) -> HeapResult<()> {
        let mut card_cursor = 0usize;

        loop {
            let Some(card) = self.select_rewrite_dirty_span_card(span_index, card_cursor)? else {
                self.finish_rewritten_dirty_span(span_index)?;

                return Ok(());
            };

            let has_young_reference =
                self.rewrite_dirty_span_card(span_index, card, forwarding, trace_table)?;
            self.finish_rewritten_dirty_span_card(
                span_index,
                card.card.index,
                has_young_reference,
            )?;
            card_cursor = card.card.index + 1;
        }
    }

    /// Select one dirty card to rewrite from one mature span.
    fn select_rewrite_dirty_span_card(
        &self,
        span_index: usize,
        card_cursor: usize,
    ) -> HeapResult<Option<DirtySpanRewrite>> {
        let Some(span) = self.span(span_index) else {
            return Err(HeapError::internal("missing span"));
        };

        let Some(card) = span.dirty_cards.find_dirty_card(card_cursor) else {
            return Ok(None);
        };

        Ok(Some(DirtySpanRewrite {
            card,
            first_offset: span.first_offset,
            size_class: span.class.size_class,
            slot_count: span.slot_count,
        }))
    }

    /// Rewrite one dirty card on one mature span.
    fn rewrite_dirty_span_card(
        &mut self,
        span_index: usize,
        card: DirtySpanRewrite,
        forwarding: &ForwardingTable,
        trace_table: &TraceTable,
    ) -> HeapResult<bool> {
        let mut has_young_reference = false;

        // rewrite occupied slots overlapped by the dirty card
        for overlap in card.card.slot_overlaps(card.size_class, card.slot_count) {
            let Some(span) = self.span(span_index) else {
                return Err(HeapError::internal("missing span"));
            };
            if !span.occupied.contains(overlap.slot_index) {
                continue;
            }

            // skip slots without local heap references
            let trace_map =
                self.small_slot_trace_map(span_index, overlap.slot_index, trace_table)?;
            if !trace_map.has_local_reference() {
                continue;
            }

            // rewrite forwarded references inside the dirty slice
            let offset = card.first_offset + overlap.slot_start;
            has_young_reference |= self.rewrite_payload_reference_range(
                offset,
                card.size_class,
                &trace_map,
                overlap.byte_start,
                overlap.byte_len,
                forwarding,
            )?;
        }

        Ok(has_young_reference)
    }

    /// Finish one rewritten dirty span card.
    fn finish_rewritten_dirty_span_card(
        &mut self,
        span_index: usize,
        card_index: usize,
        has_young_reference: bool,
    ) -> HeapResult<()> {
        let Some(span) = self.span_mut(span_index) else {
            return Err(HeapError::internal("missing span"));
        };

        if !has_young_reference {
            span.dirty_cards.clear_card(card_index);
        }
        if span.dirty_cards.is_empty() {
            span.is_dirty_queued = false;
        }

        Ok(())
    }

    /// Finish one rewritten dirty span with no remaining cards to visit.
    fn finish_rewritten_dirty_span(&mut self, span_index: usize) -> HeapResult<()> {
        let Some(span) = self.span_mut(span_index) else {
            return Err(HeapError::internal("missing span"));
        };

        span.is_dirty_queued = !span.dirty_cards.is_empty();

        Ok(())
    }

    /// Rewrite every dirty card on one mature large block.
    fn rewrite_dirty_large_references(
        &mut self,
        block_id: LargeBlockId,
        forwarding: &ForwardingTable,
    ) -> HeapResult<()> {
        let mut card_cursor = 0usize;

        loop {
            let Some(card) = self.select_rewrite_dirty_large_card(block_id, card_cursor)? else {
                self.finish_rewritten_dirty_large(block_id)?;

                return Ok(());
            };

            let has_young_reference = self.rewrite_dirty_large_card(&card, forwarding)?;
            self.finish_rewritten_dirty_large_card(block_id, card.card.index, has_young_reference)?;
            card_cursor = card.card.index + 1;
        }
    }

    /// Select one dirty card to rewrite from one mature large block.
    fn select_rewrite_dirty_large_card(
        &self,
        block_id: LargeBlockId,
        card_cursor: usize,
    ) -> HeapResult<Option<DirtyLargeRewrite>> {
        let Some(block) = self.large_block(block_id) else {
            return Err(HeapError::internal("missing large block"));
        };

        let Some(card) = block.dirty_cards.find_dirty_card(card_cursor) else {
            return Ok(None);
        };

        Ok(Some(DirtyLargeRewrite {
            card,
            first_offset: block.first_offset,
            byte_len: block.byte_len,
            trace_map: block.trace_map.clone(),
        }))
    }

    /// Rewrite one dirty card on one mature large block.
    fn rewrite_dirty_large_card(
        &mut self,
        card: &DirtyLargeRewrite,
        forwarding: &ForwardingTable,
    ) -> HeapResult<bool> {
        self.rewrite_payload_reference_range(
            card.first_offset,
            card.byte_len,
            &card.trace_map,
            card.card.byte_start,
            card.card.byte_len,
            forwarding,
        )
    }

    /// Finish one rewritten dirty large-block card.
    fn finish_rewritten_dirty_large_card(
        &mut self,
        block_id: LargeBlockId,
        card_index: usize,
        has_young_reference: bool,
    ) -> HeapResult<()> {
        let Some(block) = self.large_block_mut(block_id) else {
            return Err(HeapError::internal("missing large block"));
        };

        if !has_young_reference {
            block.dirty_cards.clear_card(card_index);
        }
        if block.dirty_cards.is_empty() {
            block.is_dirty_queued = false;
        }

        Ok(())
    }

    /// Finish one rewritten dirty large block with no remaining cards to visit.
    fn finish_rewritten_dirty_large(&mut self, block_id: LargeBlockId) -> HeapResult<()> {
        let Some(block) = self.large_block_mut(block_id) else {
            return Err(HeapError::internal("missing large block"));
        };

        block.is_dirty_queued = !block.dirty_cards.is_empty();

        Ok(())
    }

    /// Rewrite one mapped payload through completed forwarding metadata.
    fn rewrite_payload_references(
        &mut self,
        offset: usize,
        byte_len: usize,
        trace_map: &TraceMap,
        forwarding: &ForwardingTable,
    ) -> HeapResult<()> {
        if !trace_map.has_local_reference() {
            return Ok(());
        }

        // SAFETY: live payloads are materialized and the mutator is paused at this safepoint
        let bytes = unsafe { self.mapping.mapped_bytes_mut(offset, byte_len) };
        visit_heap_root_slots(trace_map, 0, bytes, ReferenceRange::All, &mut |mut slot| {
            let Some(reference) = slot.load_heap_reference()? else {
                return Ok(());
            };

            if let Some(next_reference) = self.forwarded_reference(reference, forwarding)? {
                slot.store_heap_reference(next_reference)?;
            }

            Ok(())
        })?;

        Ok(())
    }

    /// Rewrite one mapped payload byte range through completed forwarding metadata.
    fn rewrite_payload_reference_range(
        &mut self,
        offset: usize,
        byte_len: usize,
        trace_map: &TraceMap,
        range_start: usize,
        range_len: usize,
        forwarding: &ForwardingTable,
    ) -> HeapResult<bool> {
        if range_len == 0 || !trace_map.has_local_reference() {
            return Ok(false);
        }

        let range_end = (range_start + range_len).min(byte_len);
        let range_len = range_end.saturating_sub(range_start);
        if range_len == 0 {
            return Ok(false);
        }

        let (window_offset, window_start, window_len) = if trace_map.has_tagged_reference() {
            (offset, 0, byte_len)
        } else {
            let window_start = range_start.saturating_sub(HeapReference::BYTE_LEN - 1);
            let window_end = (range_end + HeapReference::BYTE_LEN - 1).min(byte_len);

            (
                offset + window_start,
                window_start,
                window_end - window_start,
            )
        };
        let mut has_young_reference = false;

        // SAFETY: live payloads are materialized and the mutator is paused at this safepoint
        let bytes = unsafe { self.mapping.mapped_bytes_mut(window_offset, window_len) };

        visit_heap_root_slots(
            trace_map,
            window_start,
            bytes,
            ReferenceRange::bytes(range_start, range_len),
            &mut |mut slot| {
                let Some(reference) = slot.load_heap_reference()? else {
                    return Ok(());
                };
                let reference = if let Some(next_reference) =
                    self.forwarded_reference(reference, forwarding)?
                {
                    slot.store_heap_reference(next_reference)?;

                    next_reference
                } else {
                    reference
                };

                if self.reference_is_young(reference)? {
                    has_young_reference = true;
                }

                Ok(())
            },
        )?;

        Ok(has_young_reference)
    }

    /// Return whether one reference points into live young space.
    pub(super) fn reference_is_young(&self, reference: HeapReference) -> HeapResult<bool> {
        if reference.is_null() {
            return Ok(false);
        }

        let Some(extent) = self.resolve_extent(reference) else {
            return Err(HeapError::invalid_heap_reference(reference));
        };

        Ok(matches!(
            extent.storage,
            HeapPlace::YoungRange { .. } | HeapPlace::YoungSlot(_)
        ))
    }

    /// Return the promoted reference for one young reference.
    fn forwarded_reference(
        &self,
        reference: HeapReference,
        forwarding: &ForwardingTable,
    ) -> HeapResult<Option<HeapReference>> {
        if reference.is_null() {
            return Ok(None);
        }

        let page_size_bytes = self.allocator().page_size_bytes();
        let page_index = reference.offset() / page_size_bytes;
        let page_offset = reference.offset() % page_size_bytes;
        let Some(HeapPageMapEntry::Young { logical_page_index }) = self.page_entry(page_index)
        else {
            return Ok(None);
        };
        let logical_byte_offset = logical_page_index * self.young.page_size_bytes + page_offset;

        if let Some(span_index) = self
            .young
            .page_spans
            .get(logical_page_index)
            .copied()
            .flatten()
        {
            return self.forwarded_young_span_reference(
                span_index,
                logical_byte_offset,
                reference,
                forwarding,
            );
        }

        self.forwarded_young_range_reference(logical_byte_offset, reference, forwarding)
    }

    /// Return the promoted reference for one young range reference.
    fn forwarded_young_range_reference(
        &self,
        logical_byte_offset: usize,
        reference: HeapReference,
        forwarding: &ForwardingTable,
    ) -> HeapResult<Option<HeapReference>> {
        let Some(range) = self.young.range_record_at_offset(logical_byte_offset) else {
            return Ok(None);
        };
        let block_offset = range.range.first_offset;

        let Some(target) = forwarding.range(range.index) else {
            return Ok(None);
        };
        let byte_offset = reference.offset() - block_offset;

        Ok(Some(target.add_bytes(byte_offset)))
    }

    /// Return the promoted reference for one young span slot reference.
    fn forwarded_young_span_reference(
        &self,
        span_index: usize,
        logical_byte_offset: usize,
        reference: HeapReference,
        forwarding: &ForwardingTable,
    ) -> HeapResult<Option<HeapReference>> {
        let Some(span) = self.young.span(span_index) else {
            return Err(HeapError::internal("missing span"));
        };
        let Some(span_offset) = logical_byte_offset.checked_sub(span.first_offset) else {
            return Ok(None);
        };
        let slot_index = span_offset / span.class.size_class;
        let slot_offset = span_offset % span.class.size_class;
        let slot = Slot::new(span_index, slot_index)?;
        let Some(target) = forwarding.slot(slot) else {
            return Ok(None);
        };

        debug_assert_eq!(
            reference.offset(),
            span.slot_offset(slot_index) + slot_offset
        );

        Ok(Some(target.add_bytes(slot_offset)))
    }
}
