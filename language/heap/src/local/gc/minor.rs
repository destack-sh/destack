use crate::TraceView;

use crate::local::gc::{DirtyCard, DirtyExtent, GC_METADATA_STEP_BYTES, Phase, charge_bitmap_skip};
use crate::local::storage::{GcStats, HeapPlace, HeapStorage, LargeBlockId};
use crate::{
    AllocationUsage, DropCursor, DropReference, GcAdvance, GcCollector, GcDrop, GcPhase, HeapError,
    HeapGcStateError, HeapOperationSource, HeapReference, HeapResult, ReferenceInput,
    ReferenceRange, RootSlot, Slot, TraceQueue,
};
impl HeapStorage {
    /// Perform one young-generation collection over mutable heap roots.
    pub(crate) fn collect_minor<E>(
        &mut self,
        roots: &mut impl FnMut(&mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>) -> Result<(), E>,
        trace_view: TraceView<'_>,
        drop: &mut impl FnMut(GcDrop) -> Result<(), E>,
    ) -> Result<GcStats, E>
    where
        E: From<HeapError>,
    {
        self.start_young_gc()?;

        self.drain_young_gc(roots, usize::MAX, trace_view, drop)
    }

    /// Drain the active local young collection and return its completed stats.
    pub(crate) fn drain_young_gc<E>(
        &mut self,
        roots: &mut impl FnMut(&mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>) -> Result<(), E>,
        budget_bytes: usize,
        trace_view: TraceView<'_>,
        drop: &mut impl FnMut(GcDrop) -> Result<(), E>,
    ) -> Result<GcStats, E>
    where
        E: From<HeapError>,
    {
        // drain the cycle in bounded steps
        loop {
            match self.step_young_gc(roots, budget_bytes, trace_view)? {
                GcAdvance::Completed(cycle) => return Ok(cycle.stats),
                GcAdvance::Stepped(_) => continue,
                GcAdvance::Drop(request) => {
                    drop(request)?;
                    self.collector.complete_drop(request.reference)?;
                }
                GcAdvance::Started(_) => {
                    return Err(HeapError::internal(
                        "local young collection returned a start event during drain",
                    )
                    .into());
                }
                GcAdvance::Idle => {
                    return Err(
                        HeapError::internal("local young collection made no progress").into(),
                    );
                }
            }
        }
    }

    /// Return whether one local young collection is active.
    pub(crate) fn minor_gc_active(&self) -> bool {
        self.collector.minor_phase != Phase::Idle
    }

    /// Start one local young collection.
    pub(crate) fn start_young_gc(&mut self) -> HeapResult<()> {
        // reject overlapping collection work
        if self.collector.is_collecting() {
            return Err(HeapError::gc_state(HeapGcStateError::LocalGcActive));
        }

        // publish active span accounting before tracing
        self.flush_young_cursor();

        // reset minor cycle cursors
        self.clear_young_mark_bits();
        self.collector.minor_queue.clear();
        self.collector.minor_phase = Phase::Mark;
        self.collector.minor_resume_phase = Phase::Idle;
        self.collector.young_reclaim_range_cursor = 0;
        self.collector.young_reclaim_span_cursor = 0;
        self.collector.young_reclaim_slot_cursor = 0;
        self.collector.young_dirty_extent_cursor = 0;
        self.collector.young_dirty_card_cursor = 0;
        self.collector.dirty_rescan_needed = false;
        self.collector.young_freed_allocations = 0;
        self.collector.young_freed_bytes = 0;

        Ok(())
    }

    /// Drain one active local young collection at a safepoint.
    pub(crate) fn step_young_gc<E>(
        &mut self,
        roots: &mut impl FnMut(&mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>) -> Result<(), E>,
        budget_bytes: usize,
        trace_view: TraceView<'_>,
    ) -> Result<GcAdvance, E>
    where
        E: From<HeapError>,
    {
        // no active work
        if budget_bytes == 0 || self.collector.minor_phase == Phase::Idle {
            return Ok(GcAdvance::Idle);
        }

        match self.collector.minor_phase {
            Phase::Idle => Ok(GcAdvance::Idle),
            Phase::Mark => self.step_young_gc_mark(roots, budget_bytes, trace_view),
            Phase::Drop => self.step_young_gc_reclaim(roots, budget_bytes, trace_view),
            Phase::Sweep => self.step_young_gc_sweep(roots, budget_bytes, trace_view),
        }
    }

    /// Mark reachable young blocks and remembered mature writes.
    fn step_young_gc_mark<E>(
        &mut self,
        roots: &mut impl FnMut(&mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>) -> Result<(), E>,
        budget_bytes: usize,
        trace_view: TraceView<'_>,
    ) -> Result<GcAdvance, E>
    where
        E: From<HeapError>,
    {
        // seed roots before each mark step
        self.seed_young_roots(roots)?;

        // scan remembered mature writes first
        let dirty_bytes = self.step_dirty_young_reference_scan(budget_bytes, trace_view)?;

        // use remaining budget for young graph tracing
        let marked_bytes = if dirty_bytes < budget_bytes {
            self.trace_young_references(budget_bytes - dirty_bytes, trace_view)?
        } else {
            0
        };

        // repass the remembered set when writes dirtied already scanned extents
        if self.collector.dirty_rescan_needed && self.young_dirty_references_drained() {
            self.collector.dirty_rescan_needed = false;
            self.collector.young_dirty_extent_cursor = 0;
            self.collector.young_dirty_card_cursor = 0;

            return Ok(GcAdvance::stepped(
                GcCollector::LocalMinor,
                GcPhase::Mark,
                budget_bytes,
                dirty_bytes + marked_bytes,
            ));
        }

        // use remaining safepoint budget before returning
        if self.young_dirty_references_drained()
            && self.collector.minor_queue.is_empty()
            && dirty_bytes + marked_bytes < budget_bytes
        {
            let phase = self.resume_young_reclamation();
            let remaining_bytes = budget_bytes - dirty_bytes - marked_bytes;
            return match phase {
                Phase::Drop => self
                    .step_young_gc_reclaim(roots, remaining_bytes, trace_view)
                    .map(|advance| advance.with_prior_work(dirty_bytes + marked_bytes)),
                Phase::Sweep => self
                    .step_young_gc_sweep(roots, remaining_bytes, trace_view)
                    .map(|advance| advance.with_prior_work(dirty_bytes + marked_bytes)),
                Phase::Idle | Phase::Mark => {
                    Err(HeapError::internal("invalid minor reclamation phase").into())
                }
            };
        }

        // advance to Drop for the next safepoint
        if self.young_dirty_references_drained() && self.collector.minor_queue.is_empty() {
            self.resume_young_reclamation();
        }

        Ok(GcAdvance::stepped(
            GcCollector::LocalMinor,
            GcPhase::Mark,
            budget_bytes,
            dirty_bytes + marked_bytes,
        ))
    }

    /// Start young Drop from the beginning.
    fn start_young_gc_drop(&mut self) {
        self.collector.young_reclaim_range_cursor = 0;
        self.collector.young_reclaim_span_cursor = 0;
        self.collector.young_reclaim_slot_cursor = 0;
        self.collector.minor_phase = Phase::Drop;
    }

    /// Resume the interrupted young reclamation phase or begin Drop.
    fn resume_young_reclamation(&mut self) -> Phase {
        let phase = self.collector.minor_resume_phase;
        self.collector.minor_resume_phase = Phase::Idle;

        match phase {
            Phase::Drop | Phase::Sweep => {
                self.collector.minor_phase = phase;
            }
            Phase::Idle => self.start_young_gc_drop(),
            Phase::Mark => {
                self.collector.minor_phase = Phase::Mark;
            }
        }

        self.collector.minor_phase
    }

    /// Start young sweep from the beginning.
    fn start_young_gc_sweep(&mut self) {
        self.collector.young_reclaim_range_cursor = 0;
        self.collector.young_reclaim_span_cursor = 0;
        self.collector.young_reclaim_slot_cursor = 0;
        self.collector.minor_phase = Phase::Sweep;
    }

    /// Advance young Drop and spend any remaining budget in sweep.
    fn step_young_gc_reclaim<E>(
        &mut self,
        roots: &mut impl FnMut(&mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>) -> Result<(), E>,
        budget_bytes: usize,
        trace_view: TraceView<'_>,
    ) -> Result<GcAdvance, E>
    where
        E: From<HeapError>,
    {
        let advance = self.step_young_gc_drop(budget_bytes)?;
        let drop_bytes = advance.work_bytes();

        // continue directly into sweep when Drop completed within this budget
        if self.collector.minor_phase == Phase::Sweep && drop_bytes < budget_bytes {
            let remaining_bytes = budget_bytes - drop_bytes;

            return self
                .step_young_gc_sweep(roots, remaining_bytes, trace_view)
                .map(|advance| advance.with_prior_work(drop_bytes));
        }

        Ok(advance)
    }

    /// Seed the active young trace queue from roots and pins.
    fn seed_young_roots<E>(
        &mut self,
        roots: &mut impl FnMut(&mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>) -> Result<(), E>,
    ) -> Result<(), E>
    where
        E: From<HeapError>,
    {
        let mut pending = std::mem::take(&mut self.collector.minor_queue);

        roots(&mut |slot: RootSlot<'_>| {
            let Some(reference) = slot.load_heap_reference()? else {
                return Ok(());
            };

            self.enqueue_young_reference(reference, &mut pending)
        })?;

        let pins = std::mem::take(&mut self.collector.pins);
        for reference in pins.references() {
            self.enqueue_young_reference(reference, &mut pending)?;
        }
        self.collector.pins = pins;

        self.collector.minor_queue = pending;

        Ok(())
    }

    /// Finish the active local young collection.
    fn finish_young_gc(&mut self) -> HeapResult<GcStats> {
        // recycle empty nursery metadata before publishing stats
        if self.young_usage.allocation_count() == 0 {
            self.recycle_young_space();
        }

        let stats = self.stats_after_collection(
            self.collector.young_freed_allocations,
            self.collector.young_freed_bytes,
        );
        self.gc.record_cycle(GcCollector::LocalMinor, stats);

        // reset active minor collection state
        self.collector.minor_phase = Phase::Idle;
        self.collector.minor_resume_phase = Phase::Idle;
        self.collector.minor_queue.clear();
        self.collector.young_reclaim_range_cursor = 0;
        self.collector.young_reclaim_span_cursor = 0;
        self.collector.young_reclaim_slot_cursor = 0;
        self.collector.young_dirty_extent_cursor = 0;
        self.collector.young_dirty_card_cursor = 0;
        self.collector.dirty_rescan_needed = false;

        // keep cards that still bridge mature objects to young objects
        self.compact_dirty_extents();
        self.collector.young_freed_allocations = 0;
        self.collector.young_freed_bytes = 0;

        Ok(stats)
    }

    /// Mark reachable young references within one byte budget.
    fn trace_young_references(
        &mut self,
        budget_bytes: usize,
        trace_view: TraceView<'_>,
    ) -> HeapResult<usize> {
        let mut marked_bytes = 0usize;
        let mut pending = std::mem::take(&mut self.collector.minor_queue);

        // drain bounded young trace work
        while marked_bytes < budget_bytes {
            let Some(reference) = pending.pop() else {
                break;
            };
            let Some(extent) = self.resolve_extent(reference) else {
                return Err(HeapError::invalid_heap_reference(reference));
            };

            // minor collection only traces young blocks
            if !matches!(
                extent.place,
                HeapPlace::YoungRange { .. } | HeapPlace::YoungSlot(_)
            ) {
                continue;
            }

            // scan every local reference in this payload into the reusable scratch
            let mut scratch = std::mem::take(&mut self.collector.local_reference_scratch);
            let base_address = self.memory.base_address() + extent.base.offset();
            let scan_result = self.visit_references::<HeapReference>(
                extent.place,
                trace_view,
                ReferenceInput::mapped(base_address),
                ReferenceRange::All,
                &mut |reference| {
                    scratch.push(reference);

                    Ok(())
                },
            );

            if let Err(error) = scan_result {
                return Err(HeapError::scan_failed(
                    HeapOperationSource::Reference(reference),
                    error,
                ));
            }

            // enqueue the discovered references
            for reference in scratch.drain(..) {
                self.enqueue_young_reference(reference, &mut pending)?;
            }
            self.collector.local_reference_scratch = scratch;

            marked_bytes += extent.byte_len.max(1);
        }

        self.collector.minor_queue = pending;

        Ok(marked_bytes)
    }

    /// Run Drop for unreachable young values within one byte budget.
    fn step_young_gc_drop(&mut self, budget_bytes: usize) -> HeapResult<GcAdvance> {
        // preserve the active value until its destructor completes
        if self.collector.is_drop_claimed() {
            return Ok(GcAdvance::Idle);
        }

        // retire one completed allocation cursor without reclaiming storage
        self.collector.retire_completed_drop();

        // continue one repeated allocation before scanning onward
        if let Some(drop) = self.collector.continue_drop(budget_bytes)? {
            return Ok(GcAdvance::Drop(drop));
        }

        let mut work_bytes = 0usize;

        // scan variable-size young ranges first
        if let Some(drop) =
            self.step_young_range_reclaim(Phase::Drop, budget_bytes, &mut work_bytes)?
        {
            return Ok(GcAdvance::Drop(drop));
        }

        // scan fixed-size young spans with remaining budget
        if let Some(drop) =
            self.step_young_span_reclaim(Phase::Drop, budget_bytes, &mut work_bytes)?
        {
            return Ok(GcAdvance::Drop(drop));
        }

        // begin sweep only after every unreachable value completed Drop
        if self.young_reclaim_drained() {
            self.start_young_gc_sweep();
        }

        Ok(GcAdvance::stepped(
            GcCollector::LocalMinor,
            GcPhase::Drop,
            budget_bytes,
            work_bytes,
        ))
    }

    /// Sweep unreachable young references within one byte budget.
    fn step_young_gc_sweep<E>(
        &mut self,
        roots: &mut impl FnMut(&mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>) -> Result<(), E>,
        budget_bytes: usize,
        trace_view: TraceView<'_>,
    ) -> Result<GcAdvance, E>
    where
        E: From<HeapError>,
    {
        let mut swept_bytes = 0usize;

        // sweep variable-size young ranges first
        self.step_young_range_reclaim(Phase::Sweep, budget_bytes, &mut swept_bytes)?;

        // sweep fixed-size young spans with remaining budget
        self.step_young_span_reclaim(Phase::Sweep, budget_bytes, &mut swept_bytes)?;

        // finish once every young allocation source is drained
        if self.young_reclaim_drained() {
            // young relocation must drain before the mutator resumes
            let promoted_bytes = self.relocate_young_survivors(roots, trace_view)?;

            return self
                .finish_young_gc()
                .map(|stats| {
                    GcAdvance::completed(
                        GcCollector::LocalMinor,
                        GcPhase::Promote,
                        budget_bytes,
                        swept_bytes + promoted_bytes,
                        stats,
                    )
                })
                .map_err(Into::into);
        }

        Ok(GcAdvance::stepped(
            GcCollector::LocalMinor,
            GcPhase::Sweep,
            budget_bytes,
            swept_bytes,
        ))
    }

    /// Process unreachable young range blocks for one reclamation phase.
    fn step_young_range_reclaim(
        &mut self,
        phase: Phase,
        budget_bytes: usize,
        work_bytes: &mut usize,
    ) -> HeapResult<Option<GcDrop>> {
        while *work_bytes < budget_bytes
            && self.collector.young_reclaim_range_cursor < self.young.ranges.len()
        {
            // skip clear bitmap words as charged metadata work
            let Some(block_index) = self
                .young
                .live
                .first_set_from(self.collector.young_reclaim_range_cursor)
            else {
                self.collector.young_reclaim_range_cursor = charge_bitmap_skip(
                    self.collector.young_reclaim_range_cursor,
                    self.young.ranges.len(),
                    budget_bytes,
                    work_bytes,
                );

                return Ok(None);
            };

            // charge skipped dead range bits
            self.collector.young_reclaim_range_cursor = charge_bitmap_skip(
                self.collector.young_reclaim_range_cursor,
                block_index,
                budget_bytes,
                work_bytes,
            );
            if *work_bytes >= budget_bytes {
                return Ok(None);
            }
            self.collector.young_reclaim_range_cursor = block_index + 1;

            // resolve the live range selected by the bitmap
            let Some(block) = self.young_range(block_index) else {
                return Err(HeapError::internal("live young range missing during sweep"));
            };
            let reference = HeapReference::new(block.first_offset);
            let byte_len = block.byte_len;

            // marked ranges survive this cycle
            if self.young.marked.contains(block_index) {
                *work_bytes += if phase == Phase::Drop {
                    GC_METADATA_STEP_BYTES
                } else {
                    byte_len.max(1)
                };

                continue;
            }

            // run Drop without reclaiming storage
            if phase == Phase::Drop {
                if let Some(drop) = block.drop {
                    let cursor = DropCursor::new(
                        GcCollector::LocalMinor,
                        DropReference::Local(reference),
                        byte_len,
                        drop,
                        *work_bytes,
                    )?;
                    let drop = self.collector.claim_drop(cursor, budget_bytes)?;

                    return Ok(Some(drop));
                }

                *work_bytes += GC_METADATA_STEP_BYTES;
            }
            // reclaim only after the complete Drop pass
            else if phase == Phase::Sweep {
                self.free(reference)
                    .map_err(|error| HeapError::free_failed(reference, error))?;
                self.collector.young_freed_allocations += 1;
                self.collector.young_freed_bytes += byte_len as u64;
                *work_bytes += byte_len.max(1);
            } else {
                return Err(HeapError::internal("invalid young range reclamation phase"));
            }
        }

        Ok(None)
    }

    /// Process unreachable fixed-size young spans for one reclamation phase.
    fn step_young_span_reclaim(
        &mut self,
        phase: Phase,
        budget_bytes: usize,
        work_bytes: &mut usize,
    ) -> HeapResult<Option<GcDrop>> {
        self.flush_young_cursor();

        // scan fixed-size young spans
        while *work_bytes < budget_bytes
            && self.collector.young_reclaim_span_cursor < self.young.spans.len()
        {
            // select the active span and reserved slot limit
            let span_index = self.collector.young_reclaim_span_cursor;
            let Some(span) = self.young.span(span_index).cloned() else {
                return Err(HeapError::internal("missing span"));
            };
            let Some(reserved_slots) = self.young.span_reserved_slot_count(span_index) else {
                return Err(HeapError::internal("missing young span reservation"));
            };

            // skip classes that cannot require a runtime callback during Drop
            if phase == Phase::Drop && span.class.drop_plan().is_none() {
                *work_bytes += GC_METADATA_STEP_BYTES;
                self.collector.young_reclaim_span_cursor += 1;
                self.collector.young_reclaim_slot_cursor = 0;

                continue;
            }

            // sweep slots within this span
            while *work_bytes < budget_bytes
                && self.collector.young_reclaim_slot_cursor < reserved_slots
            {
                let slot_index = self.collector.young_reclaim_slot_cursor;
                self.collector.young_reclaim_slot_cursor += 1;

                // already freed slots only charge metadata work
                let Some(bits) = self.young.span_bits(span_index) else {
                    return Err(HeapError::internal("missing span"));
                };
                if bits.freed.contains(slot_index) {
                    *work_bytes += GC_METADATA_STEP_BYTES;

                    continue;
                }

                // marked slots survive this cycle
                if bits.marked.contains(slot_index) {
                    *work_bytes += if phase == Phase::Drop {
                        GC_METADATA_STEP_BYTES
                    } else {
                        span.class.size_class().max(1)
                    };

                    continue;
                }

                // run Drop without reclaiming storage
                let reference = HeapReference::new(span.slot_offset(slot_index));
                if phase == Phase::Drop {
                    if let Some(drop) = span.class.drop_plan() {
                        let cursor = DropCursor::new(
                            GcCollector::LocalMinor,
                            DropReference::Local(reference),
                            span.class.size_class(),
                            drop,
                            *work_bytes,
                        )?;
                        let drop = self.collector.claim_drop(cursor, budget_bytes)?;

                        return Ok(Some(drop));
                    }

                    *work_bytes += GC_METADATA_STEP_BYTES;
                }
                // reclaim only after the complete Drop pass
                else if phase == Phase::Sweep {
                    let Some(bits) = self.young.span_bits_mut(span_index) else {
                        return Err(HeapError::internal("missing span"));
                    };
                    bits.freed.set(slot_index);
                    bits.marked.clear(slot_index);
                    self.collector.remove_shared_edge_root(reference);
                    self.record_young_free(span.class.size_class());
                    self.collector.young_freed_allocations += 1;
                    self.collector.young_freed_bytes += span.class.size_class() as u64;
                    *work_bytes += span.class.size_class().max(1);
                } else {
                    return Err(HeapError::internal("invalid young span reclamation phase"));
                }
            }

            // advance after all reserved slots drain
            if self.collector.young_reclaim_slot_cursor >= reserved_slots {
                self.collector.young_reclaim_span_cursor += 1;
                self.collector.young_reclaim_slot_cursor = 0;
            }
        }

        Ok(None)
    }

    /// Return whether the active young reclamation cursor is drained.
    fn young_reclaim_drained(&self) -> bool {
        self.collector.young_reclaim_range_cursor >= self.young.ranges.len()
            && self.collector.young_reclaim_span_cursor >= self.young.spans.len()
    }

    /// Recycle empty young metadata without changing the reserved page span.
    fn recycle_young_space(&mut self) {
        self.young.next_offset = self.young.pages.offset + self.young.allocation_alignment_bytes;
        self.young.ranges.clear();
        self.young.live.clear_all();
        self.young.marked.clear_all();
        self.young.local_reference_bits.clear_all();
        self.young.shared_reference_bits.clear_all();
        self.young.spans.clear();
        self.young.span_bits.clear();
        self.young.span_cache.fill(None);
        self.young.cursor = None;
        self.young.page_spans.fill(None);
        self.young.pending_range_usage = AllocationUsage::default();
    }

    /// Queue one young reference when it currently points into the young space.
    fn enqueue_young_reference(
        &mut self,
        reference: HeapReference,
        pending: &mut TraceQueue<HeapReference>,
    ) -> HeapResult<()> {
        // nullish and immortal references are not heap roots
        if reference.is_nullish() || self.is_immortal(reference) {
            return Ok(());
        }

        // only young references belong in the minor queue
        let Some(extent) = self.resolve_extent(reference) else {
            return Err(HeapError::invalid_heap_reference(reference));
        };

        if matches!(
            extent.place,
            HeapPlace::YoungRange { .. } | HeapPlace::YoungSlot(_)
        ) && self.mark_place(extent.place)?
        {
            pending.push(reference);
        }

        Ok(())
    }

    /// Scan remembered mature writes within one byte budget.
    fn step_dirty_young_reference_scan(
        &mut self,
        budget_bytes: usize,
        trace_view: TraceView<'_>,
    ) -> HeapResult<usize> {
        let mut scanned_bytes = 0usize;
        let mut pending = std::mem::take(&mut self.collector.minor_queue);

        self.step_dirty_extent_scan(budget_bytes, &mut scanned_bytes, &mut pending, trace_view)?;

        self.collector.minor_queue = pending;

        Ok(scanned_bytes)
    }

    /// Scan remembered mature extent writes within one byte budget.
    fn step_dirty_extent_scan(
        &mut self,
        budget_bytes: usize,
        scanned_bytes: &mut usize,
        pending: &mut TraceQueue<HeapReference>,
        trace_view: TraceView<'_>,
    ) -> HeapResult<()> {
        // scan queued dirty extents
        while *scanned_bytes < budget_bytes
            && self.collector.young_dirty_extent_cursor < self.collector.dirty_extents.len()
        {
            // select and scan the next dirty card
            let dirty_index = self.collector.young_dirty_extent_cursor;
            let extent = self.collector.dirty_extents[dirty_index];
            let card = self.scan_dirty_extent_card(extent, pending, trace_view)?;

            // finish extents that no longer have dirty cards
            let Some(card) = card else {
                self.finish_dirty_extent(extent)?;
                self.collector.young_dirty_extent_cursor += 1;
                self.collector.young_dirty_card_cursor = 0;
                *scanned_bytes += 1;

                continue;
            };

            // retain cards that still point into young space
            self.finish_dirty_extent_card(extent, card.card.index, card.has_young_reference)?;
            *scanned_bytes += card.card.byte_len.max(1);
        }

        Ok(())
    }

    /// Scan one dirty card from one mature extent.
    fn scan_dirty_extent_card(
        &mut self,
        extent: DirtyExtent,
        pending: &mut TraceQueue<HeapReference>,
        trace_view: TraceView<'_>,
    ) -> HeapResult<Option<DirtyCardScan>> {
        match extent {
            DirtyExtent::Span(span_index) => {
                self.scan_dirty_span_card(span_index, pending, trace_view)
            }
            DirtyExtent::Large(block_id) => {
                self.scan_dirty_large_card(block_id, pending, trace_view)
            }
        }
    }

    /// Return the next dirty card selected from one mature span.
    fn select_dirty_span_card(&self, span_index: usize) -> HeapResult<Option<DirtySpanCard>> {
        // resolve the mature span
        let Some(span) = self.span(span_index) else {
            return Err(HeapError::scan_failed(
                HeapOperationSource::Span(span_index),
                HeapError::internal("missing span"),
            ));
        };

        // select the next dirty card in this span
        let card_cursor = self.collector.young_dirty_card_cursor;
        let Some(card) = span.dirty_cards.find_dirty_card(card_cursor) else {
            return Ok(None);
        };

        Ok(Some(DirtySpanCard {
            card,
            size_class: span.class.size_class(),
            slot_count: span.slot_count,
            first_offset: span.first_offset,
        }))
    }

    /// Return whether one mature span slot is occupied.
    fn span_slot_occupied(&self, span_index: usize, slot_index: usize) -> HeapResult<bool> {
        let Some(span) = self.span(span_index) else {
            return Err(HeapError::scan_failed(
                HeapOperationSource::Span(span_index),
                HeapError::internal("missing span"),
            ));
        };

        Ok(span.occupied.contains(slot_index))
    }

    /// Scan one dirty card from one mature span.
    fn scan_dirty_span_card(
        &mut self,
        span_index: usize,
        pending: &mut TraceQueue<HeapReference>,
        trace_view: TraceView<'_>,
    ) -> HeapResult<Option<DirtyCardScan>> {
        let Some(card) = self.select_dirty_span_card(span_index)? else {
            return Ok(None);
        };
        let mut has_young_reference = false;

        // scan occupied slots overlapped by the dirty card
        for overlap in card.card.slot_overlaps(card.size_class, card.slot_count) {
            if !self.span_slot_occupied(span_index, overlap.slot_index)? {
                continue;
            }

            // scan young references in the dirty slice into the reusable scratch
            let mut scratch = std::mem::take(&mut self.collector.local_reference_scratch);
            let base_address = self.memory.base_address() + card.first_offset + overlap.slot_start;
            let slot = Slot::new(span_index, overlap.slot_index)?;
            self.visit_references::<HeapReference>(
                HeapPlace::MatureSlot(slot),
                trace_view,
                ReferenceInput::mapped(base_address),
                ReferenceRange::bytes(overlap.byte_start, overlap.byte_len),
                &mut |reference| {
                    scratch.push(reference);

                    Ok(())
                },
            )
            .map_err(|error| {
                HeapError::scan_failed(HeapOperationSource::Span(span_index), error)
            })?;

            // enqueue the discovered references
            for reference in scratch.drain(..) {
                has_young_reference |= self.reference_is_young(reference)?;
                self.enqueue_young_reference(reference, pending)?;
            }
            self.collector.local_reference_scratch = scratch;
        }

        Ok(Some(DirtyCardScan {
            card: card.card,
            has_young_reference,
        }))
    }

    /// Finish one scanned dirty extent card.
    fn finish_dirty_extent_card(
        &mut self,
        extent: DirtyExtent,
        card_index: usize,
        has_young_reference: bool,
    ) -> HeapResult<()> {
        match extent {
            DirtyExtent::Span(span_index) => {
                self.finish_dirty_span_card(span_index, card_index, has_young_reference)
            }
            DirtyExtent::Large(block_id) => {
                self.finish_dirty_large_card(block_id, card_index, has_young_reference)
            }
        }
    }

    /// Finish one scanned dirty span card.
    fn finish_dirty_span_card(
        &mut self,
        span_index: usize,
        card_index: usize,
        has_young_reference: bool,
    ) -> HeapResult<()> {
        // update card liveness
        let Some(span) = self.span_mut(span_index) else {
            return Err(HeapError::internal("missing span"));
        };

        if !has_young_reference {
            span.dirty_cards.clear_card(card_index);
        }
        let is_empty = span.dirty_cards.is_empty();
        if is_empty {
            span.is_dirty_queued = false;
        }

        // advance remembered-set cursors
        self.collector.young_dirty_card_cursor = card_index + 1;
        if is_empty {
            self.collector.young_dirty_extent_cursor += 1;
            self.collector.young_dirty_card_cursor = 0;
        }

        Ok(())
    }

    /// Scan one dirty card from one mature large block.
    fn scan_dirty_large_card(
        &mut self,
        block_id: LargeBlockId,
        pending: &mut TraceQueue<HeapReference>,
        trace_view: TraceView<'_>,
    ) -> HeapResult<Option<DirtyCardScan>> {
        let Some(card) = self.select_dirty_large_card(block_id)? else {
            return Ok(None);
        };
        let mut has_young_reference = false;

        // scan the dirty byte range into the reusable scratch
        let mut scratch = std::mem::take(&mut self.collector.local_reference_scratch);
        let base_address = self.memory.base_address() + card.first_offset;
        self.visit_references::<HeapReference>(
            HeapPlace::LargeBlock(block_id),
            trace_view,
            ReferenceInput::mapped(base_address),
            ReferenceRange::bytes(card.card.byte_start, card.card.byte_len),
            &mut |reference| {
                scratch.push(reference);

                Ok(())
            },
        )
        .map_err(|error| {
            HeapError::scan_failed(HeapOperationSource::LargeBlock(block_id.id()), error)
        })?;

        // enqueue the discovered references
        for reference in scratch.drain(..) {
            has_young_reference |= self.reference_is_young(reference)?;
            self.enqueue_young_reference(reference, pending)?;
        }
        self.collector.local_reference_scratch = scratch;

        Ok(Some(DirtyCardScan {
            card: card.card,
            has_young_reference,
        }))
    }

    /// Return the next dirty card selected from one mature large block.
    fn select_dirty_large_card(
        &self,
        block_id: LargeBlockId,
    ) -> HeapResult<Option<DirtyLargeCard>> {
        // resolve the large block
        let Some(block) = self.large_block(block_id) else {
            return Err(HeapError::scan_failed(
                HeapOperationSource::LargeBlock(block_id.id()),
                HeapError::internal("missing large block"),
            ));
        };

        // select the next dirty card in this block
        let card_cursor = self.collector.young_dirty_card_cursor;
        let Some(card) = block.dirty_cards.find_dirty_card(card_cursor) else {
            return Ok(None);
        };

        Ok(Some(DirtyLargeCard {
            card,
            first_offset: block.first_offset,
        }))
    }

    /// Finish one scanned dirty large-block card.
    fn finish_dirty_large_card(
        &mut self,
        block_id: LargeBlockId,
        card_index: usize,
        has_young_reference: bool,
    ) -> HeapResult<()> {
        // update card liveness
        let Some(block) = self.large_block_mut(block_id) else {
            return Err(HeapError::internal("missing large block"));
        };

        if !has_young_reference {
            block.dirty_cards.clear_card(card_index);
        }
        let is_empty = block.dirty_cards.is_empty();
        if is_empty {
            block.is_dirty_queued = false;
        }

        // advance remembered-set cursors
        self.collector.young_dirty_card_cursor = card_index + 1;
        if is_empty {
            self.collector.young_dirty_extent_cursor += 1;
            self.collector.young_dirty_card_cursor = 0;
        }

        Ok(())
    }

    /// Finish one queued dirty extent with no remaining dirty cards.
    fn finish_dirty_extent(&mut self, extent: DirtyExtent) -> HeapResult<()> {
        // clear queued state if the extent actually drained
        match extent {
            DirtyExtent::Span(span_index) => {
                let Some(span) = self.span_mut(span_index) else {
                    return Err(HeapError::internal("missing span"));
                };

                span.is_dirty_queued = !span.dirty_cards.is_empty();
            }
            DirtyExtent::Large(block_id) => {
                let Some(block) = self.large_block_mut(block_id) else {
                    return Err(HeapError::internal("missing large block"));
                };

                block.is_dirty_queued = !block.dirty_cards.is_empty();
            }
        }

        Ok(())
    }

    /// Keep only mature remembered-set entries that still have dirty cards.
    fn compact_dirty_extents(&mut self) {
        // retain extents that still contain dirty cards
        self.collector.dirty_extents.retain(|extent| match extent {
            DirtyExtent::Span(span_index) => self
                .small
                .spans
                .get(*span_index)
                .is_some_and(|span| span.is_dirty_queued),
            DirtyExtent::Large(block_id) => block_id
                .index()
                .ok()
                .and_then(|index| self.large.blocks.get(index))
                .and_then(Option::as_ref)
                .is_some_and(|block| block.is_dirty_queued),
        });
    }

    /// Return whether remembered young roots are fully scanned.
    fn young_dirty_references_drained(&self) -> bool {
        self.collector.young_dirty_extent_cursor >= self.collector.dirty_extents.len()
    }
}

/// Result from scanning one dirty remembered-set card.
struct DirtyCardScan {
    /// The scanned dirty card.
    card: DirtyCard,
    /// Whether the scanned card still contains young references.
    has_young_reference: bool,
}

/// Dirty card metadata selected from one mature span.
struct DirtySpanCard {
    /// The selected dirty card.
    card: DirtyCard,
    /// The span size class in bytes.
    size_class: usize,
    /// The number of slots in the span.
    slot_count: usize,
    /// The span byte offset within heap storage.
    first_offset: usize,
}

/// Dirty card metadata selected from one mature large block.
struct DirtyLargeCard {
    /// The selected dirty card.
    card: DirtyCard,
    /// The block byte offset within heap storage.
    first_offset: usize,
}
