use crate::TraceView;
use destack_mir::TraceMap;

use crate::local::gc::{
    GC_METADATA_STEP_BYTES, MajorReclaimCursor, MarkWork, Phase, charge_bitmap_skip,
};
use crate::local::storage::{GcStats, HeapExtent, HeapPlace, HeapStorage};
use crate::{
    DropCursor, DropPlan, DropReference, GcAdvance, GcCollector, GcDrop, GcPhase, HeapError,
    HeapGcStateError, HeapOperationSource, HeapReference, HeapResult, ReferenceInput,
    ReferenceRange, RootSlot, scan_references,
};

impl HeapStorage {
    /// Return whether one local major collection is active.
    pub(crate) fn major_gc_active(&self) -> bool {
        self.collector.major_phase != Phase::Idle
    }

    /// Publish one block to an active local major cycle.
    pub(crate) fn publish_major_allocation(
        &mut self,
        reference: HeapReference,
        place: HeapPlace,
        trace_map: &TraceMap,
    ) -> HeapResult<()> {
        // inactive collector
        if self.collector.major_phase == Phase::Idle {
            return Ok(());
        }

        self.mark_place(place)?;

        self.enqueue_payload_references(reference, place, 0, usize::MAX, trace_map)
    }

    /// Queue local references written into one active local major cycle.
    pub(crate) fn write_major_barrier(
        &mut self,
        extent: HeapExtent,
        byte_offset: usize,
        byte_len: usize,
        trace_view: TraceView<'_>,
    ) -> HeapResult<()> {
        // inactive collector
        if self.collector.major_phase == Phase::Idle {
            return Ok(());
        }

        // scan the written range into the reusable scratch
        let scan_len = byte_len.min(self.resolve_byte_len(extent.place)? - byte_offset);
        let mut scratch = std::mem::take(&mut self.collector.local_reference_scratch);
        let base_address = self.memory.base_address() + extent.base.offset();
        self.visit_references::<HeapReference>(
            extent.place,
            trace_view,
            ReferenceInput::mapped(base_address),
            ReferenceRange::bytes(byte_offset, scan_len),
            &mut |reference| {
                scratch.push(reference);

                Ok(())
            },
        )?;

        // enqueue the discovered references
        for reference in scratch.drain(..) {
            self.enqueue_major_reference(reference)?;
        }
        self.collector.local_reference_scratch = scratch;

        Ok(())
    }

    /// Queue local references from one heap payload range into the active major cycle.
    fn enqueue_payload_references(
        &mut self,
        reference: HeapReference,
        place: HeapPlace,
        byte_offset: usize,
        byte_len: usize,
        trace_map: &TraceMap,
    ) -> HeapResult<()> {
        if !trace_map.has_local_reference() {
            return Ok(());
        }

        // resolve the scan window inside the block payload
        let place_byte_len = self.resolve_byte_len(place)?;
        let scan_len = byte_len.min(place_byte_len - byte_offset);

        // scan local reference slots into the reusable scratch
        let mut scratch = std::mem::take(&mut self.collector.local_reference_scratch);
        let base_address = self.memory.base_address() + reference.offset();
        let result = scan_references::<HeapReference>(
            trace_map,
            ReferenceInput::mapped(base_address),
            ReferenceRange::bytes(byte_offset, scan_len),
            &mut scratch,
        );

        if let Err(error) = result {
            return Err(HeapError::scan_failed(
                HeapOperationSource::Reference(reference),
                error,
            ));
        }

        // enqueue the discovered references
        for reference in scratch.drain(..) {
            self.enqueue_major_reference(reference)?;
        }
        self.collector.local_reference_scratch = scratch;

        Ok(())
    }

    /// Perform one full heap collection over mutable heap roots.
    pub(crate) fn collect_full<E>(
        &mut self,
        roots: &mut impl FnMut(&mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>) -> Result<(), E>,
        trace_view: TraceView<'_>,
        drop: &mut impl FnMut(GcDrop) -> Result<(), E>,
    ) -> Result<GcStats, E>
    where
        E: From<HeapError>,
    {
        // collect the nursery first so full collection sees mature references
        let _minor = self.collect_minor(roots, trace_view, drop)?;

        self.start_major_gc(roots)?;

        self.drain_major_gc(roots, usize::MAX, trace_view, drop)
    }

    /// Drain the active local major collection and return its completed stats.
    pub(crate) fn drain_major_gc<E>(
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
            match self.step_major_gc(roots, budget_bytes, trace_view)? {
                GcAdvance::Completed(cycle) => return Ok(cycle.stats),
                GcAdvance::Stepped(_) => continue,
                GcAdvance::Drop(request) => {
                    drop(request)?;
                    self.collector.complete_drop(request.reference)?;
                }
                GcAdvance::Started(_) => {
                    return Err(HeapError::internal(
                        "local full collection returned a start event during drain",
                    )
                    .into());
                }
                GcAdvance::Idle => {
                    return Err(
                        HeapError::internal("local full collection made no progress").into(),
                    );
                }
            }
        }
    }

    /// Start one local major collection.
    pub(crate) fn start_major_gc<E>(
        &mut self,
        roots: &mut impl FnMut(&mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>) -> Result<(), E>,
    ) -> Result<(), E>
    where
        E: From<HeapError>,
    {
        // reject overlapping collection work
        if self.collector.is_collecting() {
            return Err(HeapError::gc_state(HeapGcStateError::LocalGcActive).into());
        }

        // publish active span accounting before tracing
        self.flush_young_cursor();

        // reset cycle state
        self.collector.mark_epoch += 1;
        self.clear_young_mark_bits();
        self.collector.major_queue.clear();
        self.collector.major_reclaim = MajorReclaimCursor::default();
        self.collector.major_freed_allocations = 0;
        self.collector.major_freed_bytes = 0;
        self.collector.major_phase = Phase::Mark;

        // seed initial roots
        self.seed_major_roots(roots)?;

        Ok(())
    }

    /// Perform bounded work for one active local major collection.
    pub(crate) fn step_major_gc<E>(
        &mut self,
        roots: &mut impl FnMut(&mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>) -> Result<(), E>,
        budget_bytes: usize,
        trace_view: TraceView<'_>,
    ) -> Result<GcAdvance, E>
    where
        E: From<HeapError>,
    {
        // no active work
        if budget_bytes == 0 || self.collector.major_phase == Phase::Idle {
            return Ok(GcAdvance::Idle);
        }

        // phase work
        match self.collector.major_phase {
            Phase::Idle => Ok(GcAdvance::Idle),
            Phase::Mark => {
                // roots may have changed between incremental steps
                self.seed_major_roots(roots)?;
                let marked_bytes = self.step_major_gc_mark(budget_bytes, trace_view)?;

                // switch to reclamation when mark work drains
                if self.collector.major_queue.is_empty() {
                    self.start_major_reclaim();
                    self.collector.major_phase = Phase::Drop;

                    // spend remaining budget dropping dead values
                    if marked_bytes < budget_bytes {
                        let remaining_bytes = budget_bytes - marked_bytes;
                        let advance = self.step_major_gc_reclaim(remaining_bytes)?;

                        return Ok(advance.with_prior_work(marked_bytes));
                    }
                }

                Ok(GcAdvance::stepped(
                    GcCollector::LocalMajor,
                    GcPhase::Mark,
                    budget_bytes,
                    marked_bytes,
                ))
            }
            Phase::Drop | Phase::Sweep => {
                let marked_bytes = self.step_major_gc_mark(budget_bytes, trace_view)?;
                if !self.collector.major_queue.is_empty() {
                    return Ok(GcAdvance::stepped(
                        GcCollector::LocalMajor,
                        GcPhase::Mark,
                        budget_bytes,
                        marked_bytes,
                    ));
                }
                if marked_bytes >= budget_bytes {
                    return Ok(GcAdvance::stepped(
                        GcCollector::LocalMajor,
                        GcPhase::Mark,
                        budget_bytes,
                        marked_bytes,
                    ));
                }

                let remaining_bytes = budget_bytes - marked_bytes;
                let advance = match self.collector.major_phase {
                    Phase::Drop => self.step_major_gc_reclaim(remaining_bytes)?,
                    Phase::Sweep => self.step_major_gc_sweep(remaining_bytes)?,
                    Phase::Idle | Phase::Mark => {
                        return Err(HeapError::internal("invalid major reclamation phase").into());
                    }
                };

                Ok(advance.with_prior_work(marked_bytes))
            }
        }
    }

    /// Seed the active major trace queue from explicit roots and pins.
    fn seed_major_roots<E>(
        &mut self,
        roots: &mut impl FnMut(&mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>) -> Result<(), E>,
    ) -> Result<(), E>
    where
        E: From<HeapError>,
    {
        // root slots
        roots(&mut |slot: RootSlot<'_>| {
            let Some(reference) = slot.load_heap_reference()? else {
                return Ok(());
            };

            self.enqueue_major_reference(reference)
        })?;

        // pins
        let pins = std::mem::take(&mut self.collector.pins);
        for reference in pins.references() {
            self.enqueue_major_reference(reference)?;
        }
        self.collector.pins = pins;

        Ok(())
    }

    /// Finish the active local major collection.
    fn finish_major_gc(&mut self) -> HeapResult<GcStats> {
        let stats = self.stats_after_collection(
            self.collector.major_freed_allocations,
            self.collector.major_freed_bytes,
        );

        // publish cycle statistics and reset collector state
        self.gc.record_cycle(GcCollector::LocalMajor, stats);
        self.collector.major_phase = Phase::Idle;
        self.collector.major_queue.clear();
        self.collector.major_reclaim = MajorReclaimCursor::default();
        self.collector.major_freed_allocations = 0;
        self.collector.major_freed_bytes = 0;

        Ok(stats)
    }

    /// Start one post-mark pass over the heap tables visible to the active major cycle.
    fn start_major_reclaim(&mut self) {
        // publish active span accounting before cursor bounds are captured
        self.flush_young_cursor();

        // capture reclamation limits for a stable bounded pass
        self.collector.major_reclaim = MajorReclaimCursor {
            small_span_limit: self.small.spans.len(),
            large_limit: self.large.blocks.len(),
            ..MajorReclaimCursor::default()
        };
    }

    /// Restart the post-mark cursor for sweep.
    fn start_major_sweep(&mut self) {
        let small_span_limit = self.collector.major_reclaim.small_span_limit;
        let large_limit = self.collector.major_reclaim.large_limit;
        self.collector.major_reclaim = MajorReclaimCursor {
            small_span_limit,
            large_limit,
            ..MajorReclaimCursor::default()
        };
        self.collector.major_phase = Phase::Sweep;
    }

    /// Advance major Drop and spend any remaining budget in sweep.
    fn step_major_gc_reclaim(&mut self, budget_bytes: usize) -> HeapResult<GcAdvance> {
        let advance = self.step_major_gc_drop(budget_bytes)?;
        let drop_bytes = advance.work_bytes();

        // continue directly into sweep when Drop completed within this budget
        if self.collector.major_phase == Phase::Sweep && drop_bytes < budget_bytes {
            let remaining_bytes = budget_bytes - drop_bytes;

            return self
                .step_major_gc_sweep(remaining_bytes)
                .map(|advance| advance.with_prior_work(drop_bytes));
        }

        Ok(advance)
    }

    /// Run Drop for unreachable values within one byte budget.
    fn step_major_gc_drop(&mut self, budget_bytes: usize) -> HeapResult<GcAdvance> {
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

        // scan young variable ranges first
        if let Some(drop) =
            self.step_major_young_range_reclaim(Phase::Drop, budget_bytes, &mut work_bytes)?
        {
            return Ok(GcAdvance::Drop(drop));
        }

        // scan young fixed spans with remaining budget
        if let Some(drop) =
            self.step_major_young_span_reclaim(Phase::Drop, budget_bytes, &mut work_bytes)?
        {
            return Ok(GcAdvance::Drop(drop));
        }

        // scan mature small spans with remaining budget
        if let Some(drop) =
            self.step_major_small_span_reclaim(Phase::Drop, budget_bytes, &mut work_bytes)?
        {
            return Ok(GcAdvance::Drop(drop));
        }

        // scan mature large blocks last
        if let Some(drop) =
            self.step_major_large_block_reclaim(Phase::Drop, budget_bytes, &mut work_bytes)?
        {
            return Ok(GcAdvance::Drop(drop));
        }

        // begin sweep only after every unreachable value completed Drop
        if self.major_reclaim_drained() {
            self.start_major_sweep();
        }

        Ok(GcAdvance::stepped(
            GcCollector::LocalMajor,
            GcPhase::Drop,
            budget_bytes,
            work_bytes,
        ))
    }

    /// Sweep unreachable references within one byte budget.
    fn step_major_gc_sweep(&mut self, budget_bytes: usize) -> HeapResult<GcAdvance> {
        let mut swept_bytes = 0usize;

        // sweep young variable ranges first
        self.step_major_young_range_reclaim(Phase::Sweep, budget_bytes, &mut swept_bytes)?;

        // sweep young fixed spans with remaining budget
        self.step_major_young_span_reclaim(Phase::Sweep, budget_bytes, &mut swept_bytes)?;

        // sweep mature small spans with remaining budget
        self.step_major_small_span_reclaim(Phase::Sweep, budget_bytes, &mut swept_bytes)?;

        // sweep mature large blocks last
        self.step_major_large_block_reclaim(Phase::Sweep, budget_bytes, &mut swept_bytes)?;

        // finish when all sweep cursors drain
        if self.major_reclaim_drained() {
            return self.finish_major_gc().map(|stats| {
                GcAdvance::completed(
                    GcCollector::LocalMajor,
                    GcPhase::Sweep,
                    budget_bytes,
                    swept_bytes,
                    stats,
                )
            });
        }

        Ok(GcAdvance::stepped(
            GcCollector::LocalMajor,
            GcPhase::Sweep,
            budget_bytes,
            swept_bytes,
        ))
    }

    /// Process unreachable young range blocks for one reclamation phase.
    fn step_major_young_range_reclaim(
        &mut self,
        phase: Phase,
        budget_bytes: usize,
        work_bytes: &mut usize,
    ) -> HeapResult<Option<GcDrop>> {
        while *work_bytes < budget_bytes
            && self.collector.major_reclaim.young_range_cursor < self.young.ranges.len()
        {
            // skip clear bitmap words as charged metadata work
            let Some(block_index) = self
                .young
                .live
                .first_set_from(self.collector.major_reclaim.young_range_cursor)
            else {
                self.collector.major_reclaim.young_range_cursor = charge_bitmap_skip(
                    self.collector.major_reclaim.young_range_cursor,
                    self.young.ranges.len(),
                    budget_bytes,
                    work_bytes,
                );

                return Ok(None);
            };

            // charge skipped dead range bits
            self.collector.major_reclaim.young_range_cursor = charge_bitmap_skip(
                self.collector.major_reclaim.young_range_cursor,
                block_index,
                budget_bytes,
                work_bytes,
            );
            if *work_bytes >= budget_bytes {
                return Ok(None);
            }
            self.collector.major_reclaim.young_range_cursor = block_index + 1;

            // resolve the live range selected by the bitmap
            let Some(block) = self.young_range(block_index) else {
                return Err(HeapError::internal(
                    "live young range missing during major sweep",
                ));
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
                        GcCollector::LocalMajor,
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
                self.free_swept_reference(reference, byte_len)?;
                *work_bytes += byte_len.max(1);
            } else {
                return Err(HeapError::internal("invalid major range reclamation phase"));
            }
        }

        Ok(None)
    }

    /// Process unreachable young span slots for one reclamation phase.
    fn step_major_young_span_reclaim(
        &mut self,
        phase: Phase,
        budget_bytes: usize,
        work_bytes: &mut usize,
    ) -> HeapResult<Option<GcDrop>> {
        while *work_bytes < budget_bytes
            && self.collector.major_reclaim.young_span_cursor < self.young.spans.len()
        {
            // select the active span and reserved slot limit
            let span_index = self.collector.major_reclaim.young_span_cursor;
            let span = self.major_young_span(span_index)?;

            // skip classes that cannot require a runtime callback during Drop
            if phase == Phase::Drop && span.drop.is_none() {
                *work_bytes += GC_METADATA_STEP_BYTES;
                self.collector.major_reclaim.young_span_cursor += 1;
                self.collector.major_reclaim.young_slot_cursor = 0;

                continue;
            }

            // sweep slots within this young span
            while *work_bytes < budget_bytes
                && self.collector.major_reclaim.young_slot_cursor < span.reserved_slots
            {
                let slot_index = self.collector.major_reclaim.young_slot_cursor;
                self.collector.major_reclaim.young_slot_cursor += 1;

                // already freed slots only charge metadata work
                let Some(bits) = self.young.span_bits_mut(span_index) else {
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
                        span.size_class.max(1)
                    };

                    continue;
                }

                // run Drop without reclaiming storage
                let reference =
                    HeapReference::new(span.first_offset + slot_index * span.size_class);
                if phase == Phase::Drop {
                    if let Some(drop) = span.drop {
                        let cursor = DropCursor::new(
                            GcCollector::LocalMajor,
                            DropReference::Local(reference),
                            span.size_class,
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
                    bits.freed.set(slot_index);
                    bits.marked.clear(slot_index);
                    self.collector.remove_shared_edge_root(reference);
                    self.record_young_free(span.size_class);
                    self.collector.major_freed_allocations += 1;
                    self.collector.major_freed_bytes += span.size_class as u64;
                    *work_bytes += span.size_class.max(1);
                } else {
                    return Err(HeapError::internal("invalid major span reclamation phase"));
                }
            }

            // advance after all reserved slots drain
            if self.collector.major_reclaim.young_slot_cursor >= span.reserved_slots {
                self.collector.major_reclaim.young_span_cursor += 1;
                self.collector.major_reclaim.young_slot_cursor = 0;
            }
        }

        Ok(None)
    }

    /// Return young span reclamation metadata without cloning span bitmaps.
    fn major_young_span(&self, span_index: usize) -> HeapResult<YoungSpanReclaim> {
        let Some(span) = self.young.span(span_index) else {
            return Err(HeapError::internal("missing span"));
        };

        let Some(reserved_slots) = self.young.span_reserved_slot_count(span_index) else {
            return Err(HeapError::internal("missing young span reservation"));
        };

        Ok(YoungSpanReclaim {
            first_offset: span.first_offset,
            size_class: span.class.size_class(),
            reserved_slots,
            drop: span.class.drop_plan(),
        })
    }

    /// Process unreachable small-span slots for one reclamation phase.
    fn step_major_small_span_reclaim(
        &mut self,
        phase: Phase,
        budget_bytes: usize,
        work_bytes: &mut usize,
    ) -> HeapResult<Option<GcDrop>> {
        while *work_bytes < budget_bytes
            && self.collector.major_reclaim.small_span_cursor
                < self.collector.major_reclaim.small_span_limit
        {
            // select mature span sweep metadata
            let span_index = self.collector.major_reclaim.small_span_cursor;
            let span = self.major_small_span(span_index)?;

            // skip classes that cannot require a runtime callback during Drop
            if phase == Phase::Drop && span.drop.is_none() {
                *work_bytes += GC_METADATA_STEP_BYTES;
                self.collector.major_reclaim.small_span_cursor += 1;
                self.collector.major_reclaim.small_slot_cursor = 0;

                continue;
            }

            // sweep slots within this mature span
            while *work_bytes < budget_bytes
                && self.collector.major_reclaim.small_slot_cursor < span.slot_count
            {
                let slot_index = self.collector.major_reclaim.small_slot_cursor;
                self.collector.major_reclaim.small_slot_cursor += 1;
                let slot = self.major_small_slot(span_index, slot_index)?;

                // empty slots only charge metadata work
                if !slot.is_occupied {
                    *work_bytes += GC_METADATA_STEP_BYTES;

                    continue;
                }

                // marked slots survive this cycle
                if slot.is_marked {
                    *work_bytes += if phase == Phase::Drop {
                        GC_METADATA_STEP_BYTES
                    } else {
                        span.size_class.max(1)
                    };

                    continue;
                }

                // unmarked slots are dead
                let reference =
                    HeapReference::new(span.first_offset + slot_index * span.size_class);
                if phase == Phase::Drop {
                    if let Some(drop) = span.drop {
                        let cursor = DropCursor::new(
                            GcCollector::LocalMajor,
                            DropReference::Local(reference),
                            span.size_class,
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
                    self.free_swept_reference(reference, span.size_class)?;
                    *work_bytes += span.size_class.max(1);
                } else {
                    return Err(HeapError::internal("invalid mature span reclamation phase"));
                }
            }

            // advance after the span drains
            if self.collector.major_reclaim.small_slot_cursor >= span.slot_count {
                self.collector.major_reclaim.small_span_cursor += 1;
                self.collector.major_reclaim.small_slot_cursor = 0;
            }
        }

        Ok(None)
    }

    /// Return mature small span reclamation metadata.
    fn major_small_span(&self, span_index: usize) -> HeapResult<SmallSpanReclaim> {
        let Some(span) = self.span(span_index) else {
            return Err(HeapError::internal("missing span"));
        };

        Ok(SmallSpanReclaim {
            first_offset: span.first_offset,
            size_class: span.class.size_class(),
            slot_count: span.slot_count,
            drop: span.class.drop_plan(),
        })
    }

    /// Return mature small slot reclamation state.
    fn major_small_slot(
        &self,
        span_index: usize,
        slot_index: usize,
    ) -> HeapResult<SmallSlotReclaim> {
        let Some(span) = self.span(span_index) else {
            return Err(HeapError::internal("missing span"));
        };

        Ok(SmallSlotReclaim {
            is_occupied: span.occupied.contains(slot_index),
            is_marked: span.mark_epoch == self.collector.mark_epoch
                && span.marked.contains(slot_index),
        })
    }

    /// Process unreachable large blocks for one reclamation phase.
    fn step_major_large_block_reclaim(
        &mut self,
        phase: Phase,
        budget_bytes: usize,
        work_bytes: &mut usize,
    ) -> HeapResult<Option<GcDrop>> {
        while *work_bytes < budget_bytes
            && self.collector.major_reclaim.large_cursor < self.collector.major_reclaim.large_limit
        {
            // select the next large block
            let block_index = self.collector.major_reclaim.large_cursor;
            self.collector.major_reclaim.large_cursor += 1;

            // inactive blocks only charge metadata work
            let Some(block) = self.large.blocks.get(block_index) else {
                return Err(HeapError::internal("missing large block"));
            };
            let Some(block) = block else {
                *work_bytes += GC_METADATA_STEP_BYTES;

                continue;
            };

            let reference = HeapReference::new(block.first_offset);
            let byte_len = block.byte_len;

            // marked blocks survive this cycle
            if block.mark_epoch == self.collector.mark_epoch {
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
                        GcCollector::LocalMajor,
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
                self.free_swept_reference(reference, byte_len)?;
                *work_bytes += byte_len.max(1);
            } else {
                return Err(HeapError::internal("invalid large block reclamation phase"));
            }
        }

        Ok(None)
    }

    /// Return whether the active major post-mark cursor is drained.
    fn major_reclaim_drained(&self) -> bool {
        self.collector.major_reclaim.young_range_cursor >= self.young.ranges.len()
            && self.collector.major_reclaim.young_span_cursor >= self.young.spans.len()
            && self.collector.major_reclaim.small_span_cursor
                >= self.collector.major_reclaim.small_span_limit
            && self.collector.major_reclaim.large_cursor >= self.collector.major_reclaim.large_limit
    }

    /// Free one unreachable block discovered by major sweep.
    fn free_swept_reference(
        &mut self,
        reference: HeapReference,
        byte_len: usize,
    ) -> HeapResult<()> {
        // free physical storage
        self.free(reference)
            .map_err(|error| HeapError::free_failed(reference, error))?;

        // record cycle accounting
        self.collector.major_freed_allocations += 1;
        self.collector.major_freed_bytes += byte_len as u64;

        Ok(())
    }

    /// Mark reachable heap references within one byte budget.
    fn step_major_gc_mark(
        &mut self,
        budget_bytes: usize,
        trace_view: TraceView<'_>,
    ) -> HeapResult<usize> {
        let mut marked_bytes = 0usize;

        // trace bounded mark work
        while marked_bytes < budget_bytes {
            // claim the next bounded mark item
            let Some(work) = self.collector.major_queue.pop() else {
                break;
            };

            match work {
                // large blocks are scanned page by page
                MarkWork::LargeRange { reference, start } => {
                    marked_bytes += self.trace_large_range(reference, start, trace_view)?;

                    continue;
                }

                // small and young blocks are scanned as one mark item
                MarkWork::Reference(reference) => {
                    marked_bytes += self.trace_reference(reference, trace_view)?;
                }
            }
        }

        Ok(marked_bytes)
    }

    /// Trace one local heap reference.
    fn trace_reference(
        &mut self,
        reference: HeapReference,
        trace_view: TraceView<'_>,
    ) -> HeapResult<usize> {
        // resolve the referenced block
        let Some(extent) = self.resolve_extent(reference) else {
            return Err(HeapError::invalid_heap_reference(reference));
        };

        // scan local references inside the payload into the reusable scratch
        let mut scratch = std::mem::take(&mut self.collector.local_reference_scratch);
        let base_address = self.memory.base_address() + extent.base.offset();
        let trace_result = self.visit_references::<HeapReference>(
            extent.place,
            trace_view,
            ReferenceInput::mapped(base_address),
            ReferenceRange::All,
            &mut |reference| {
                scratch.push(reference);

                Ok(())
            },
        );

        if let Err(error) = trace_result {
            return Err(HeapError::scan_failed(
                HeapOperationSource::Reference(reference),
                error,
            ));
        }

        // enqueue the discovered references
        for reference in scratch.drain(..) {
            self.enqueue_major_reference(reference)?;
        }
        self.collector.local_reference_scratch = scratch;

        Ok(extent.byte_len.max(1))
    }

    /// Trace one page-sized range from one local large block.
    fn trace_large_range(
        &mut self,
        reference: HeapReference,
        start: usize,
        trace_view: TraceView<'_>,
    ) -> HeapResult<usize> {
        // resolve and verify the large block
        let Some(extent) = self.resolve_extent(reference) else {
            return Err(HeapError::invalid_heap_reference(reference));
        };
        let HeapPlace::LargeBlock(_) = extent.place else {
            return Err(HeapError::invalid_heap_reference(reference));
        };

        // skip empty ranges and noscan payloads
        let has_local_reference = self
            .has_reference::<HeapReference>(extent.place, trace_view)
            .map_err(|error| {
                HeapError::scan_failed(HeapOperationSource::Reference(reference), error)
            })?;
        if !has_local_reference || start >= extent.byte_len {
            return Ok(0);
        }

        // scan at most one memory page into the reusable scratch
        let range_len = self.page_size_bytes().min(extent.byte_len - start);
        let mut scratch = std::mem::take(&mut self.collector.local_reference_scratch);
        let base_address = self.memory.base_address() + extent.base.offset();
        let trace_result = self.visit_references::<HeapReference>(
            extent.place,
            trace_view,
            ReferenceInput::mapped(base_address),
            ReferenceRange::bytes(start, range_len),
            &mut |reference| {
                scratch.push(reference);

                Ok(())
            },
        );

        if let Err(error) = trace_result {
            return Err(HeapError::scan_failed(
                HeapOperationSource::Reference(reference),
                error,
            ));
        }

        // enqueue the discovered references
        for reference in scratch.drain(..) {
            self.enqueue_major_reference(reference)?;
        }
        self.collector.local_reference_scratch = scratch;

        // continue this large block on a later step
        let next_start = start + range_len;
        if next_start < extent.byte_len {
            self.collector.major_queue.push(MarkWork::LargeRange {
                reference,
                start: next_start,
            });
        }

        Ok(range_len)
    }

    /// Queue one major collection reference after marking it.
    fn enqueue_major_reference(&mut self, reference: HeapReference) -> HeapResult<()> {
        // nullish references are not heap roots
        if reference.is_nullish() {
            return Ok(());
        }

        // mark the referenced block before queueing scan work
        let Some(extent) = self.resolve_extent(reference) else {
            return Err(HeapError::invalid_heap_reference(reference));
        };

        if self.mark_place(extent.place)? {
            // marked noscan blocks need no queued scan work
            // large blocks are sliced to keep major steps bounded
            if matches!(extent.place, HeapPlace::LargeBlock(_)) {
                self.collector.major_queue.push(MarkWork::LargeRange {
                    reference,
                    start: 0,
                });

                return Ok(());
            }

            // smaller blocks are one mark item
            self.collector
                .major_queue
                .push(MarkWork::Reference(reference));
        }

        Ok(())
    }

    /// Clear every young-space mark bit.
    pub(super) fn clear_young_mark_bits(&mut self) {
        self.young.marked.clear_all();
        for bits in &mut self.young.span_bits {
            bits.marked.clear_all();
        }
    }

    /// Return whether one heap place is marked in the active cycle.
    pub(super) fn is_marked_place(&self, place: HeapPlace) -> HeapResult<bool> {
        // dispatch by physical heap place
        match place {
            HeapPlace::YoungRange { first_offset } => {
                let Some(range) = self.young_range_by_offset(first_offset) else {
                    return Err(HeapError::internal("missing young range"));
                };

                Ok(self.young.marked.contains(range.index))
            }
            HeapPlace::YoungSlot(slot) => {
                let Some(bits) = self.young.span_bits(slot.span_index()) else {
                    return Err(HeapError::internal("missing span"));
                };

                Ok(bits.marked.contains(slot.slot_index()))
            }
            HeapPlace::MatureSlot(slot) => {
                let Some(span) = self.span(slot.span_index()) else {
                    return Err(HeapError::internal("missing span"));
                };

                Ok(span.mark_epoch == self.collector.mark_epoch
                    && span.marked.contains(slot.slot_index()))
            }
            HeapPlace::LargeBlock(block_id) => {
                let Some(block) = self.large_block(block_id) else {
                    return Err(HeapError::internal("missing large block"));
                };

                Ok(block.mark_epoch == self.collector.mark_epoch)
            }
        }
    }

    /// Mark one heap place and return whether this was the first mark.
    pub(crate) fn mark_place(&mut self, place: HeapPlace) -> HeapResult<bool> {
        // skip places already marked
        if self.is_marked_place(place)? {
            return Ok(false);
        }

        // mark by physical heap place
        match place {
            HeapPlace::YoungRange { first_offset } => {
                let Some(range) = self.young_range_by_offset(first_offset) else {
                    return Err(HeapError::internal("missing young range"));
                };

                self.young.marked.set(range.index);
            }
            HeapPlace::YoungSlot(slot) => {
                let Some(bits) = self.young.span_bits_mut(slot.span_index()) else {
                    return Err(HeapError::internal("missing span"));
                };

                bits.marked.set(slot.slot_index());
            }
            HeapPlace::MatureSlot(slot) => {
                let mark_epoch = self.collector.mark_epoch;
                let Some(span) = self.span_mut(slot.span_index()) else {
                    return Err(HeapError::internal("missing span"));
                };

                span.ensure_mark_epoch(mark_epoch);
                span.marked.set(slot.slot_index());
            }
            HeapPlace::LargeBlock(block_id) => {
                let mark_epoch = self.collector.mark_epoch;
                let Some(block) = self.large_block_mut(block_id) else {
                    return Err(HeapError::internal("missing large block"));
                };

                block.mark_epoch = mark_epoch;
            }
        }

        Ok(true)
    }

    /// Build one GC statistics snapshot after one collection pass.
    pub(super) fn stats_after_collection(
        &self,
        freed_allocations: usize,
        freed_bytes: u64,
    ) -> GcStats {
        GcStats {
            freed_allocations,
            live_allocations: self.usage.allocation_count(),
            freed_bytes,
            allocated_bytes: self.usage.allocated_bytes(),
            retained_bytes: self.retained_bytes(),
        }
    }
}

/// Young span metadata needed by major reclamation.
struct YoungSpanReclaim {
    /// The span byte offset within heap storage.
    first_offset: usize,
    /// The span size class in bytes.
    size_class: usize,
    /// The number of reserved slots to sweep.
    reserved_slots: usize,
    /// The drop plan for each managed slot.
    drop: Option<DropPlan>,
}

/// Mature small span metadata needed by major reclamation.
struct SmallSpanReclaim {
    /// The span byte offset within heap storage.
    first_offset: usize,
    /// The span size class in bytes.
    size_class: usize,
    /// The number of slots to sweep.
    slot_count: usize,
    /// The drop plan for each managed slot.
    drop: Option<DropPlan>,
}

/// Mature small slot state needed by major reclamation.
struct SmallSlotReclaim {
    /// Whether the slot is currently occupied.
    is_occupied: bool,
    /// Whether the slot is marked in the active major cycle.
    is_marked: bool,
}
