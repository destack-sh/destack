use destack_mir::{TraceMap, TraceTable};

use crate::allocator::SpanSlot;
use crate::local::space::{
    DirtyRegion, HeapPageMapEntry, HeapPlace, HeapRegion, HeapSpace, LargeAllocationId, YoungPlace,
};
use crate::{
    HeapError, HeapReference, HeapResult, ReferenceRange, RootSlot, visit_heap_root_slots,
};

/// Young-to-mature forwarding built for one promotion pass.
#[derive(Debug, Default)]
struct ForwardingTable {
    /// Forwarded range allocations keyed by young range index.
    ranges: Vec<RangeForwarding>,
    /// Forwarded run slots keyed by span and slot index.
    run_slots: Vec<RunForwarding>,
}

impl ForwardingTable {
    /// Record one promoted young range.
    fn push_range(&mut self, range_index: usize, target: HeapReference) {
        self.ranges.push(RangeForwarding {
            range_index,
            target,
        });
    }

    /// Record one promoted young run slot.
    fn push_run_slot(&mut self, slot: SpanSlot, target: HeapReference) {
        self.run_slots.push(RunForwarding { slot, target });
    }

    /// Return the promoted reference for one young range.
    fn range(&self, range_index: usize) -> Option<HeapReference> {
        self.ranges
            .binary_search_by_key(&range_index, |forwarding| forwarding.range_index)
            .ok()
            .map(|index| self.ranges[index].target)
    }

    /// Return the promoted reference for one young run slot.
    fn run_slot(&self, slot: SpanSlot) -> Option<HeapReference> {
        self.run_slots
            .binary_search_by_key(&slot_key(slot), |forwarding| slot_key(forwarding.slot))
            .ok()
            .map(|index| self.run_slots[index].target)
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
struct RunForwarding {
    /// The young run slot.
    slot: SpanSlot,
    /// The mature reference for the copied payload.
    target: HeapReference,
}

/// Return the sortable key for one young run slot.
fn slot_key(slot: SpanSlot) -> (usize, usize) {
    (slot.span_index(), slot.slot_index())
}

impl HeapSpace {
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
        self.promote_young_run_survivors(&mut forwarding, trace_table)?;

        // rewrite every visible local reference before the mutator resumes
        self.rewrite_promoted_references(roots, &forwarding, trace_table)?;

        Ok(())
    }

    /// Promote reachable young range allocations.
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
            let bytes = self
                .mapping
                .read_bytes(range.first_offset, range.byte_len)?;
            let target_place =
                self.allocate_promoted_payload(range.byte_len, &trace_map, &bytes)?;
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

    /// Promote reachable fixed-size young run slots.
    fn promote_young_run_survivors(
        &mut self,
        forwarding: &mut ForwardingTable,
        trace_table: &TraceTable,
    ) -> HeapResult<()> {
        self.flush_young_run_cursor();

        for run_index in 0..self.young.runs.len() {
            let Some(run) = self.young.run(run_index).cloned() else {
                return Err(HeapError::internal("missing span"));
            };
            let reserved_slots = self
                .young
                .run_reserved_slot_count(run_index)
                .unwrap_or_else(|| run.slot_count());

            for slot_index in 0..reserved_slots {
                let slot = SpanSlot::new(run_index, slot_index)?;
                let Some(bits) = self.young.run_bits_mut(run_index) else {
                    return Err(HeapError::internal("missing span"));
                };

                // only marked occupied slots can be promoted
                if bits.freed.contains(slot_index) || !bits.marked.contains(slot_index) {
                    continue;
                }

                let source = HeapReference::new(run.slot_offset(slot_index));

                // pinned young objects keep their stable address
                if self.collector.pins.contains(source) {
                    continue;
                }

                let bytes = self
                    .mapping
                    .read_bytes(source.offset(), run.class.size_class)?;
                let trace_map = self
                    .trace_map_for_place(HeapPlace::Young(YoungPlace::Slot(slot)), trace_table)?;
                let target_place =
                    self.allocate_promoted_payload(run.class.size_class, &trace_map, &bytes)?;
                let target = self.base_reference(target_place)?;

                self.publish_promoted_payload(
                    target,
                    target_place,
                    run.class.size_class,
                    &trace_map,
                    trace_table,
                )?;
                forwarding.push_run_slot(slot, target);
                self.retire_promoted_young_slot(slot, source, run.class.size_class)?;
            }
        }

        Ok(())
    }

    /// Publish remembered metadata for one promoted mature payload.
    fn publish_promoted_payload(
        &mut self,
        reference: HeapReference,
        place: HeapPlace,
        byte_len: usize,
        trace_map: &TraceMap,
        trace_table: &TraceTable,
    ) -> HeapResult<()> {
        if trace_map.has_shared_reference() {
            self.collector.track_shared_edge_root(reference);
        }

        self.record_write_barrier(
            reference,
            HeapRegion {
                place,
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

    /// Retire one promoted fixed-size young run source.
    fn retire_promoted_young_slot(
        &mut self,
        slot: SpanSlot,
        reference: HeapReference,
        byte_len: usize,
    ) -> HeapResult<()> {
        let Some(bits) = self.young.run_bits_mut(slot.span_index()) else {
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
        if forwarding.ranges.is_empty() && forwarding.run_slots.is_empty() {
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
        let dirty_region_count = self.collector.dirty_regions.len();

        for dirty_index in 0..dirty_region_count {
            match self.collector.dirty_regions[dirty_index] {
                DirtyRegion::Span(span_index) => {
                    self.rewrite_dirty_span_references(span_index, forwarding, trace_table)?;
                }
                DirtyRegion::Large(allocation_id) => {
                    self.rewrite_dirty_large_references(allocation_id, forwarding)?;
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
            let Some(span) = self.span(span_index) else {
                return Err(HeapError::internal("missing span"));
            };
            let Some((card_index, card_start, card_len)) =
                span.dirty_cards.next_dirty_card_from(card_cursor)
            else {
                self.finish_rewritten_dirty_span(span_index)?;

                return Ok(());
            };

            let has_young_reference = self.rewrite_dirty_span_card(
                span_index,
                card_start,
                card_len,
                forwarding,
                trace_table,
            )?;
            self.finish_rewritten_dirty_span_card(span_index, card_index, has_young_reference)?;
            card_cursor = card_index + 1;
        }
    }

    /// Rewrite one dirty card on one mature span.
    fn rewrite_dirty_span_card(
        &mut self,
        span_index: usize,
        card_start: usize,
        card_len: usize,
        forwarding: &ForwardingTable,
        trace_table: &TraceTable,
    ) -> HeapResult<bool> {
        let Some(span) = self.span(span_index) else {
            return Err(HeapError::internal("missing span"));
        };
        let card_end = card_start + card_len;
        let first_offset = span.first_offset;
        let size_class = span.class.size_class;
        let slot_count = span.slot_count;
        let first_slot = card_start / size_class;
        let last_slot = (card_end - 1) / size_class;
        let end_slot = (last_slot + 1).min(slot_count);
        let mut has_young_reference = false;

        for slot_index in first_slot..end_slot {
            let Some(span) = self.span(span_index) else {
                return Err(HeapError::internal("missing span"));
            };
            if !span.occupied.contains(slot_index) {
                continue;
            }

            let trace_map = self.small_slot_trace_map(span_index, slot_index, trace_table)?;
            if !trace_map.has_local_reference() {
                continue;
            }

            let slot_start = size_class * slot_index;
            let slot_end = slot_start + size_class;
            let overlap_start = card_start.max(slot_start);
            let overlap_end = card_end.min(slot_end);
            if overlap_start >= overlap_end {
                continue;
            }

            let offset = first_offset + slot_start;
            let local_start = overlap_start - slot_start;
            let local_len = overlap_end - overlap_start;
            has_young_reference |= self.rewrite_payload_reference_range(
                offset,
                size_class,
                &trace_map,
                local_start,
                local_len,
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

    /// Rewrite every dirty card on one mature large allocation.
    fn rewrite_dirty_large_references(
        &mut self,
        allocation_id: LargeAllocationId,
        forwarding: &ForwardingTable,
    ) -> HeapResult<()> {
        let mut card_cursor = 0usize;

        loop {
            let Some(allocation) = self.large_allocation(allocation_id) else {
                return Err(HeapError::internal("missing large allocation"));
            };
            let Some((card_index, card_start, card_len)) =
                allocation.dirty_cards.next_dirty_card_from(card_cursor)
            else {
                self.finish_rewritten_dirty_large(allocation_id)?;

                return Ok(());
            };

            let has_young_reference =
                self.rewrite_dirty_large_card(allocation_id, card_start, card_len, forwarding)?;
            self.finish_rewritten_dirty_large_card(allocation_id, card_index, has_young_reference)?;
            card_cursor = card_index + 1;
        }
    }

    /// Rewrite one dirty card on one mature large allocation.
    fn rewrite_dirty_large_card(
        &mut self,
        allocation_id: LargeAllocationId,
        card_start: usize,
        card_len: usize,
        forwarding: &ForwardingTable,
    ) -> HeapResult<bool> {
        let Some(allocation) = self.large_allocation(allocation_id) else {
            return Err(HeapError::internal("missing large allocation"));
        };
        let first_offset = allocation.first_offset;
        let byte_len = allocation.byte_len;
        let trace_map = allocation.trace_map.clone();

        self.rewrite_payload_reference_range(
            first_offset,
            byte_len,
            &trace_map,
            card_start,
            card_len,
            forwarding,
        )
    }

    /// Finish one rewritten dirty large-allocation card.
    fn finish_rewritten_dirty_large_card(
        &mut self,
        allocation_id: LargeAllocationId,
        card_index: usize,
        has_young_reference: bool,
    ) -> HeapResult<()> {
        let Some(allocation) = self.large_allocation_mut(allocation_id) else {
            return Err(HeapError::internal("missing large allocation"));
        };

        if !has_young_reference {
            allocation.dirty_cards.clear_card(card_index);
        }
        if allocation.dirty_cards.is_empty() {
            allocation.is_dirty_queued = false;
        }

        Ok(())
    }

    /// Finish one rewritten dirty large allocation with no remaining cards to visit.
    fn finish_rewritten_dirty_large(&mut self, allocation_id: LargeAllocationId) -> HeapResult<()> {
        let Some(allocation) = self.large_allocation_mut(allocation_id) else {
            return Err(HeapError::internal("missing large allocation"));
        };

        allocation.is_dirty_queued = !allocation.dirty_cards.is_empty();

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

        let mut did_rewrite = false;
        let mut bytes = self.mapping.read_bytes(offset, byte_len)?;
        visit_heap_root_slots(
            trace_map,
            0,
            &mut bytes,
            ReferenceRange::All,
            &mut |mut slot| {
                let Some(reference) = slot.load_heap_reference()? else {
                    return Ok(());
                };

                if let Some(next_reference) = self.forwarded_reference(reference, forwarding)? {
                    slot.store_heap_reference(next_reference)?;
                    did_rewrite = true;
                }

                Ok(())
            },
        )?;
        if did_rewrite {
            self.mapping.write_bytes(offset, &bytes)?;
        }

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
        let mut did_rewrite = false;
        let mut bytes = self.mapping.read_bytes(window_offset, window_len)?;

        visit_heap_root_slots(
            trace_map,
            window_start,
            &mut bytes,
            ReferenceRange::bytes(range_start, range_len),
            &mut |mut slot| {
                let Some(reference) = slot.load_heap_reference()? else {
                    return Ok(());
                };
                let reference = if let Some(next_reference) =
                    self.forwarded_reference(reference, forwarding)?
                {
                    slot.store_heap_reference(next_reference)?;
                    did_rewrite = true;

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

        if did_rewrite {
            self.mapping.write_bytes(window_offset, &bytes)?;
        }

        Ok(has_young_reference)
    }

    /// Return whether one reference points into live young space.
    fn reference_is_young(&self, reference: HeapReference) -> HeapResult<bool> {
        if reference.is_null() {
            return Ok(false);
        }

        let Some(region) = self.resolve_region(reference) else {
            return Err(HeapError::invalid_heap_reference(reference));
        };

        Ok(matches!(
            region.place,
            HeapPlace::Young(YoungPlace::Range { .. }) | HeapPlace::Young(YoungPlace::Slot(_))
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

        if let Some(run_index) = self
            .young
            .page_runs
            .get(logical_page_index)
            .copied()
            .flatten()
        {
            return self.forwarded_young_run_reference(
                run_index,
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
        let Some((range_index, range)) = self.young.range_record_at_offset(logical_byte_offset)
        else {
            return Ok(None);
        };
        let allocation_offset = range.first_offset;

        let Some(target) = forwarding.range(range_index) else {
            return Ok(None);
        };
        let byte_offset = reference.offset() - allocation_offset;

        Ok(Some(target.add_bytes(byte_offset)))
    }

    /// Return the promoted reference for one young run slot reference.
    fn forwarded_young_run_reference(
        &self,
        run_index: usize,
        logical_byte_offset: usize,
        reference: HeapReference,
        forwarding: &ForwardingTable,
    ) -> HeapResult<Option<HeapReference>> {
        let Some(run) = self.young.run(run_index) else {
            return Err(HeapError::internal("missing span"));
        };
        let Some(run_offset) = logical_byte_offset.checked_sub(run.first_offset) else {
            return Ok(None);
        };
        let slot_index = run_offset / run.class.size_class;
        let slot_offset = run_offset % run.class.size_class;
        let slot = SpanSlot::new(run_index, slot_index)?;
        let Some(target) = forwarding.run_slot(slot) else {
            return Ok(None);
        };

        debug_assert_eq!(
            reference.offset(),
            run.slot_offset(slot_index) + slot_offset
        );

        Ok(Some(target.add_bytes(slot_offset)))
    }
}
