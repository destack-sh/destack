use std::sync::Arc;
use std::sync::atomic::Ordering;

use crate::shared::gc::{SharedGcPhase, SharedGcWorker, SharedTraceWork};
use crate::shared::space::{
    SharedHeapPlace, SharedHeapSpace, checked_place_offset, checked_slot_offset,
};
use crate::{
    GcKind, GcProgress, GcStats, HeapError, HeapResult, SharedHeapReference, slot_reference_map,
    visit_shared_references_in_reader, visit_shared_references_in_reader_range,
};

impl SharedHeapSpace {
    /// Start one shared heap mark phase over the given roots.
    pub(crate) fn start_mark(
        &self,
        roots: impl IntoIterator<Item = SharedHeapReference>,
    ) -> HeapResult<()> {
        // lifecycle
        let _lifecycle = self.gc.lock_lifecycle();

        // phase
        if self.gc.phase() != SharedGcPhase::Idle {
            return Err(HeapError::SharedCollectionActive);
        }

        // cycle state
        self.clear_mark_bits();
        self.gc.trace_queue.clear();
        self.gc.open_mark_publication();
        self.gc.sweep_cursor.store(0, Ordering::Release);
        self.gc.mark_publishers.store(0, Ordering::Release);
        self.gc.mark_inflight.store(0, Ordering::Release);
        self.gc.freed_allocations.store(0, Ordering::Release);
        self.gc.freed_bytes.store(0, Ordering::Release);
        self.gc.set_phase(SharedGcPhase::Mark);

        // begin with roots
        self.queue_unmarked_references(None, roots)?;

        Ok(())
    }

    /// Perform one full shared heap collection over the given roots.
    pub fn collect_full(
        &self,
        roots: impl IntoIterator<Item = SharedHeapReference>,
    ) -> HeapResult<GcStats> {
        self.start_mark(roots)?;

        // concurrent mark
        while !self.mark_idle() {
            self.mark_step(None, [], usize::MAX)?;
        }

        // incremental sweep
        if !self.try_start_sweep()? {
            return Err(HeapError::SharedCollectionActive);
        }
        loop {
            if let Some(stats) = self.sweep_step(usize::MAX)?.completed_stats() {
                return Ok(stats);
            }
        }
    }

    /// Perform bounded shared mark work.
    pub(crate) fn mark_step(
        &self,
        worker: Option<&SharedGcWorker>,
        roots: impl IntoIterator<Item = SharedHeapReference>,
        step_budget: usize,
    ) -> HeapResult<()> {
        // phase
        if self.gc.phase() != SharedGcPhase::Mark {
            return Err(HeapError::SharedCollectionNotMarking);
        }

        // newly discovered roots
        self.queue_unmarked_references(worker, roots)?;

        // mark queue
        let mut work_done = 0usize;
        let batch_capacity = self.trace_batch_capacity();
        while work_done < step_budget {
            // reserve before popping so termination sees in-flight batches
            let batch_len = (step_budget - work_done).min(batch_capacity);
            self.gc.mark_inflight.fetch_add(batch_len, Ordering::AcqRel);

            // claim one batch from local, global, or stolen work
            let batch = self.gc.trace_queue.pop_batch(worker, batch_len);
            if batch.is_empty() {
                self.gc.mark_inflight.fetch_sub(batch_len, Ordering::AcqRel);
                break;
            }

            // release unused reservation when fewer items were available
            let unused = batch_len - batch.len();
            if unused > 0 {
                self.gc.mark_inflight.fetch_sub(unused, Ordering::AcqRel);
            }

            // trace claimed work without collector-side allocation
            let trace_result = self.trace_batch(worker, &batch, step_budget - work_done);
            let (traced_work, processed_items) = match trace_result {
                Ok(result) => result,
                Err(error) => {
                    self.gc
                        .mark_inflight
                        .fetch_sub(batch.len(), Ordering::AcqRel);

                    return Err(error);
                }
            };

            // return work that did not fit this step budget
            for work in batch[processed_items..].iter().copied() {
                self.gc.trace_queue.push(worker, work);
            }

            // publish requeued work before dropping the in-flight count
            self.gc
                .mark_inflight
                .fetch_sub(batch.len(), Ordering::AcqRel);
            work_done += traced_work.max(1);
        }

        Ok(())
    }

    /// Return the mark queue batch capacity for this space.
    fn trace_batch_capacity(&self) -> usize {
        let state = self.state.read();
        let min_slot_bytes = state
            .small
            .size_classes
            .min_small_allocation_bytes()
            .unwrap_or(state.small.span_bytes);

        (state.small.span_bytes / min_slot_bytes).max(1)
    }

    /// Trace one shared mark batch.
    fn trace_batch(
        &self,
        worker: Option<&SharedGcWorker>,
        batch: &[SharedTraceWork],
        step_budget: usize,
    ) -> HeapResult<(usize, usize)> {
        let mut start = 0usize;
        let mut work_done = 0usize;

        // trace small spans together and fall back to large references otherwise
        while start < batch.len() {
            // stop before consuming another queued item
            if work_done >= step_budget {
                break;
            }

            match batch[start] {
                // large allocations are already page-sliced
                SharedTraceWork::Large {
                    reference,
                    start: range_start,
                } => {
                    self.trace_large_range(worker, reference, range_start)?;
                    start += 1;
                    work_done += 1;
                }

                // adjacent small-span items can share one scan
                SharedTraceWork::SmallSpan(span_index) => {
                    let mut end = start + 1;

                    // skip duplicate work for the same span
                    while end < batch.len() {
                        if batch[end] != SharedTraceWork::SmallSpan(span_index) {
                            break;
                        }

                        end += 1;
                    }

                    let span_work =
                        self.trace_small_span_work(worker, span_index, step_budget - work_done)?;
                    work_done += span_work.max(1);
                    start = end;
                }
            }
        }

        Ok((work_done, start))
    }

    /// Return whether concurrent mark is currently drained.
    pub(crate) fn mark_idle(&self) -> bool {
        self.gc.phase() == SharedGcPhase::Mark && self.gc.mark_drained()
    }

    /// Transition from concurrent mark into sweeping.
    pub(crate) fn try_start_sweep(&self) -> HeapResult<bool> {
        // lifecycle
        let _lifecycle = self.gc.lock_lifecycle();

        // phase
        if self.gc.phase() != SharedGcPhase::Mark {
            return Err(HeapError::SharedCollectionNotMarking);
        }

        // termination
        self.gc.close_mark_publication();
        if !self.gc.mark_drained() {
            self.gc.open_mark_publication();

            return Ok(false);
        }

        // sweep snapshot
        let references = self.live_references()?;
        *self.gc.sweep_references.lock() = Arc::from(references);

        // sweep cursor
        self.gc.sweep_cursor.store(0, Ordering::Release);
        self.gc.set_phase(SharedGcPhase::Sweep);

        Ok(true)
    }

    /// Perform bounded shared sweep work.
    pub(crate) fn sweep_step(&self, step_budget: usize) -> HeapResult<GcProgress> {
        let references = self.gc.sweep_references.lock().clone();
        let reference_len = references.len();

        let mut work_done = 0usize;
        let mut released_references = Vec::new();

        // sweep cursor
        while work_done < step_budget {
            let index = self.gc.sweep_cursor.fetch_add(1, Ordering::AcqRel);
            if index >= reference_len {
                break;
            }

            work_done += 1;

            let Some(reference) = references.get(index).copied() else {
                continue;
            };

            let Some(location) = self.resolve_location(reference) else {
                continue;
            };

            if self.is_marked_place(location.place)? {
                continue;
            }

            released_references.push(reference);
        }

        // reclaimed allocation
        for reference in released_references {
            let released_bytes = self.free_reference(reference)?;

            // freed totals
            self.gc.freed_allocations.fetch_add(1, Ordering::AcqRel);
            self.gc
                .freed_bytes
                .fetch_add(released_bytes, Ordering::AcqRel);
        }

        // end of sweep
        if self.gc.sweep_cursor.load(Ordering::Acquire) >= reference_len {
            return self.finish_collection().map(GcProgress::Complete);
        }

        Ok(GcProgress::Active)
    }

    /// Trace one page-sized range from one shared large allocation.
    fn trace_large_range(
        &self,
        worker: Option<&SharedGcWorker>,
        reference: SharedHeapReference,
        start: usize,
    ) -> HeapResult<()> {
        // resolve and verify the large allocation
        let Some(location) = self.resolve_location(reference) else {
            return Err(HeapError::InvalidSharedHeapReference { reference });
        };
        let SharedHeapPlace::Large(_) = location.place else {
            return Err(HeapError::InvalidSharedHeapReference { reference });
        };

        // skip empty ranges and noscan payloads
        let scan = self.scan(reference)?;
        if !scan.has_shared_reference() {
            return Ok(());
        }
        if start >= location.byte_len {
            return Ok(());
        }

        // scan at most one allocator page
        let range_len = self.allocator.page_bytes().min(location.byte_len - start);

        let mut edge_buffer = Vec::new();

        // payload scan
        let trace_result = visit_shared_references_in_reader_range(
            &scan,
            start,
            range_len,
            |start, buffer| self.read_bytes_into(reference, start, buffer),
            |reference: SharedHeapReference| {
                if !reference.is_null() {
                    edge_buffer.push(reference);
                }
            },
        );

        trace_result?;

        // discovered edges
        self.queue_references(worker, edge_buffer)?;

        // continue this large allocation on a later step
        let next_start = start + range_len;

        if next_start < location.byte_len {
            self.gc.trace_queue.push(
                worker,
                SharedTraceWork::Large {
                    reference,
                    start: next_start,
                },
            );
        }

        Ok(())
    }

    /// Trace one shared small-span work item under one span read.
    fn trace_small_span_work(
        &self,
        worker: Option<&SharedGcWorker>,
        span_index: usize,
        step_budget: usize,
    ) -> HeapResult<usize> {
        let mut work_done = 0usize;

        // load the span handle once, then scan outside the space lock
        let span = {
            let store = self.state.read();
            let Some(span) = store.small.spans.get(span_index).cloned() else {
                return Err(HeapError::MissingSpan { span_index });
            };

            span
        };

        // keep draining until this span really goes idle
        while work_done < step_budget {
            let scan_slots = {
                let mut span = span.write();
                let mut slot_indices = Vec::new();
                let remaining_work = step_budget - work_done;

                // marked minus scanned
                span.marked.visit_set_ranges(|start, len| {
                    for slot_index in start..start + len {
                        // stop when the step budget is claimed
                        if slot_indices.len() == remaining_work {
                            return;
                        }

                        // skip slots already claimed by another worker
                        if !span.scanned.contains(slot_index) {
                            slot_indices.push(slot_index);
                        }
                    }
                });

                if slot_indices.is_empty() {
                    span.is_queued_for_scan = false;
                    return Ok(work_done);
                }

                // claim bounded marked slots before releasing the span lock
                let mut scan_slots = Vec::with_capacity(slot_indices.len());

                for slot_index in slot_indices {
                    span.scanned.set(slot_index);

                    let slot_offset = checked_slot_offset(span.class.size_class, slot_index)?;
                    let reference_map = slot_reference_map(
                        &span.local_reference_bits,
                        &span.shared_reference_bits,
                        slot_index,
                        span.class.size_class,
                        span.class.size_class,
                    );
                    scan_slots.push((
                        span.first_offset,
                        slot_offset,
                        span.pages.len(),
                        reference_map,
                    ));
                }

                scan_slots
            };

            work_done += scan_slots.len();
            let mut edge_buffer = Vec::new();

            // shared small-span scan
            for (span_offset, slot_offset, page_count, reference_map) in scan_slots {
                // noscan slots cost one claimed unit only
                if !reference_map.has_shared_reference() {
                    continue;
                }

                // payload scan
                let span_bytes = page_count * self.allocator.page_bytes();
                let trace_result = visit_shared_references_in_reader(
                    &reference_map,
                    |start, buffer| {
                        let read_offset = checked_place_offset(slot_offset, start, span_bytes)?;
                        let store = self.state.read();

                        store.mapping.read(span_offset + read_offset, buffer)
                    },
                    |reference: SharedHeapReference| {
                        if !reference.is_null() {
                            edge_buffer.push(reference);
                        }
                    },
                );

                trace_result?;
            }

            // discovered edges
            self.queue_references(worker, edge_buffer)?;
        }

        // keep this span queued when the step budget runs out
        self.gc
            .trace_queue
            .push(worker, SharedTraceWork::SmallSpan(span_index));

        Ok(work_done)
    }

    /// Free one shared heap reference.
    fn free_reference(&self, reference: SharedHeapReference) -> HeapResult<u64> {
        let Some(location) = self.resolve_location(reference) else {
            return Err(HeapError::InvalidSharedHeapReference { reference });
        };
        let released_bytes = location.byte_len as u64;
        let mut store = self.state.write();

        // location release
        match location.place {
            SharedHeapPlace::Small(slot) => {
                self.release_small_slot(&mut store, slot)?;
            }
            SharedHeapPlace::Large(allocation_id) => {
                let Some(allocation) = store.large.allocations.get(allocation_id.index()?).cloned()
                else {
                    return Err(HeapError::MissingLargeAllocation {
                        allocation_id: allocation_id.id(),
                    });
                };
                let (first_offset, pages) = {
                    let mut allocation = allocation.write();
                    if !allocation.is_live {
                        return Err(HeapError::MissingLargeAllocation {
                            allocation_id: allocation_id.id(),
                        });
                    }

                    let first_offset = allocation.first_offset;
                    let pages = allocation.pages;
                    allocation.retire();

                    (first_offset, pages)
                };

                store
                    .large
                    .free_large_allocation_ids
                    .push(allocation_id.id());
                self.unmap_page_run(&mut store, first_offset, &pages);
                store
                    .page_run_cache
                    .release_page_run(&self.allocator, pages)?;
            }
        }

        // reference release
        self.usage.free(released_bytes);

        Ok(released_bytes)
    }

    /// Finish one completed shared collection cycle.
    fn finish_collection(&self) -> HeapResult<GcStats> {
        // lifecycle
        let _lifecycle = self.gc.lock_lifecycle();
        let mut store = self.state.write();
        let retained_bytes = self.live_retained_bytes(&store);

        // cycle stats
        let stats = GcStats {
            freed_allocations: self.gc.freed_allocations.load(Ordering::Acquire),
            live_allocations: self.usage.allocation_count(),
            freed_bytes: self.gc.freed_bytes.load(Ordering::Acquire),
            allocated_bytes: self.usage.allocated_bytes(),
            retained_bytes,
        };

        // cycle reset
        self.gc.close_mark_publication();
        self.gc.sweep_cursor.store(0, Ordering::Release);
        self.gc.mark_publishers.store(0, Ordering::Release);
        self.gc.mark_inflight.store(0, Ordering::Release);
        self.gc.freed_allocations.store(0, Ordering::Release);
        self.gc.freed_bytes.store(0, Ordering::Release);
        *self.gc.sweep_references.lock() = Arc::from([]);
        self.gc.trace_queue.clear();

        // cycle summary
        store.gc.record_cycle(GcKind::Full, stats);

        // idle publication
        self.gc.set_phase(SharedGcPhase::Idle);
        self.gc.open_mark_publication();

        Ok(stats)
    }

    /// Record one shared heap write barrier after one completed store.
    pub(crate) fn write_shared_barrier(
        &self,
        reference: SharedHeapReference,
        byte_offset: usize,
        byte_len: usize,
    ) -> HeapResult<()> {
        // publication
        let Some(_publication) = self.gc.begin_mark_publication() else {
            return Ok(());
        };

        let mut edge_buffer = Vec::new();
        let scan = self.scan(reference)?;

        // written range
        let trace_result = visit_shared_references_in_reader_range(
            &scan,
            byte_offset,
            byte_len,
            |start, buffer| self.read_bytes_into(reference, start, buffer),
            |reference: SharedHeapReference| {
                if !reference.is_null() {
                    edge_buffer.push(reference);
                }
            },
        );

        trace_result?;

        // published edges
        self.queue_references(None, edge_buffer)?;

        Ok(())
    }

    /// Record one shared heap write barrier from one caller-provided byte slice.
    pub(crate) fn write_shared_barrier_bytes(
        &self,
        reference: SharedHeapReference,
        byte_offset: usize,
        bytes: &[u8],
    ) -> HeapResult<()> {
        // publication
        let Some(_publication) = self.gc.begin_mark_publication() else {
            return Ok(());
        };
        if bytes.is_empty() {
            return Ok(());
        }

        // written bytes
        let mut edge_buffer = Vec::new();
        let scan = self.scan(reference)?;
        let trace_result = visit_shared_references_in_reader_range(
            &scan,
            byte_offset,
            bytes.len(),
            |start, buffer| {
                let local_start = start - byte_offset;
                let local_end = local_start + buffer.len();
                let Some(window) = bytes.get(local_start..local_end) else {
                    return Err(HeapError::TruncatedReferenceReaderWindow {
                        start: local_start,
                        width: buffer.len(),
                    });
                };

                buffer.copy_from_slice(window);
                Ok(())
            },
            |reference: SharedHeapReference| {
                if !reference.is_null() {
                    edge_buffer.push(reference);
                }
            },
        );

        trace_result?;

        // published edges
        self.queue_references(None, edge_buffer)?;

        Ok(())
    }

    /// Publish one exact shared heap reference after one completed store.
    pub(crate) fn publish_shared_edge(&self, reference: SharedHeapReference) -> HeapResult<()> {
        // publication
        let Some(_publication) = self.gc.begin_mark_publication() else {
            return Ok(());
        };

        if reference.is_null() {
            return Ok(());
        }

        self.queue_reference(None, reference)
    }

    /// Publish one newly allocated shared heap reference into the active cycle.
    pub(crate) fn publish_shared_allocation(
        &self,
        reference: SharedHeapReference,
        has_initial_edges: bool,
    ) -> HeapResult<()> {
        let Some(location) = self.resolve_location(reference) else {
            return Err(HeapError::InvalidSharedHeapReference { reference });
        };

        // sweeping allocations stay live in the active cycle
        if self.gc.phase() == SharedGcPhase::Sweep {
            self.mark_place(location.place)?;

            return Ok(());
        }

        // publication
        let Some(_publication) = self.gc.begin_mark_publication() else {
            if self.gc.phase() == SharedGcPhase::Sweep {
                self.mark_place(location.place)?;
            }

            return Ok(());
        };

        // new allocations without initial shared edges can stay black
        if !has_initial_edges {
            self.mark_place(location.place)?;

            return Ok(());
        }

        // queue the new allocation so the active cycle traces its initial payload
        self.queue_reference(None, reference)
    }

    /// Queue explicit roots that are not already marked in this cycle.
    fn queue_unmarked_references(
        &self,
        worker: Option<&SharedGcWorker>,
        references: impl IntoIterator<Item = SharedHeapReference>,
    ) -> HeapResult<()> {
        for reference in references {
            if reference.is_null() {
                continue;
            }

            self.queue_reference_work(worker, reference)?;
        }

        Ok(())
    }

    /// Queue one shared reference for later trace work.
    fn queue_reference(
        &self,
        worker: Option<&SharedGcWorker>,
        reference: SharedHeapReference,
    ) -> HeapResult<()> {
        self.queue_references(worker, [reference])
    }

    /// Queue shared references for later trace work.
    fn queue_references(
        &self,
        worker: Option<&SharedGcWorker>,
        references: impl IntoIterator<Item = SharedHeapReference>,
    ) -> HeapResult<()> {
        for reference in references {
            if reference.is_null() {
                continue;
            }

            self.queue_reference_work(worker, reference)?;
        }

        Ok(())
    }

    /// Queue one shared reference as span work or direct reference work.
    fn queue_reference_work(
        &self,
        worker: Option<&SharedGcWorker>,
        reference: SharedHeapReference,
    ) -> HeapResult<()> {
        let Some(location) = self.resolve_location(reference) else {
            return Err(HeapError::InvalidSharedHeapReference { reference });
        };

        // small references mark their span slot for later scanning
        if let SharedHeapPlace::Small(slot) = location.place {
            let should_queue = {
                let store = self.state.read();
                let Some(span) = store.small.spans.get(slot.span_index()).cloned() else {
                    return Err(HeapError::MissingSpan {
                        span_index: slot.span_index(),
                    });
                };

                let mut span = span.write();
                if span.marked.contains(slot.slot_index()) {
                    return Ok(());
                }

                span.marked.set(slot.slot_index());

                if span.is_queued_for_scan {
                    false
                } else {
                    span.is_queued_for_scan = true;
                    true
                }
            };

            if should_queue {
                self.gc
                    .trace_queue
                    .push(worker, SharedTraceWork::SmallSpan(slot.span_index()));
            }

            return Ok(());
        }

        // large references scan in page-sized chunks
        if !self.mark_place(location.place)? {
            return Ok(());
        }
        if !self.scan(reference)?.has_shared_reference() {
            return Ok(());
        }

        self.gc.trace_queue.push(
            worker,
            SharedTraceWork::Large {
                reference,
                start: 0,
            },
        );

        Ok(())
    }

    /// Clear every collector mark bit in the live shared heap.
    fn clear_mark_bits(&self) {
        let store = self.state.read();

        for span in &store.small.spans {
            span.write().clear_marks();
        }

        for allocation in &store.large.allocations {
            allocation.write().is_marked = false;
        }
    }

    /// Return whether one shared heap place is marked in the active cycle.
    fn is_marked_place(&self, place: SharedHeapPlace) -> HeapResult<bool> {
        let store = self.state.read();

        match place {
            SharedHeapPlace::Small(slot) => {
                let Some(span) = store.small.spans.get(slot.span_index()).cloned() else {
                    return Err(HeapError::MissingSpan {
                        span_index: slot.span_index(),
                    });
                };
                let span = span.read();

                Ok(span.marked.contains(slot.slot_index()))
            }
            SharedHeapPlace::Large(allocation_id) => {
                let Some(allocation) = store.large.allocations.get(allocation_id.index()?).cloned()
                else {
                    return Err(HeapError::MissingLargeAllocation {
                        allocation_id: allocation_id.id(),
                    });
                };
                let allocation = allocation.read();

                Ok(allocation.is_marked)
            }
        }
    }

    /// Mark one shared heap place and return whether this was the first mark.
    fn mark_place(&self, place: SharedHeapPlace) -> HeapResult<bool> {
        if self.is_marked_place(place)? {
            return Ok(false);
        }

        let store = self.state.read();

        match place {
            SharedHeapPlace::Small(slot) => {
                let Some(span) = store.small.spans.get(slot.span_index()).cloned() else {
                    return Err(HeapError::MissingSpan {
                        span_index: slot.span_index(),
                    });
                };
                let mut span = span.write();

                span.marked.set(slot.slot_index());
            }
            SharedHeapPlace::Large(allocation_id) => {
                let Some(allocation) = store.large.allocations.get(allocation_id.index()?).cloned()
                else {
                    return Err(HeapError::MissingLargeAllocation {
                        allocation_id: allocation_id.id(),
                    });
                };
                let mut allocation = allocation.write();

                allocation.is_marked = true;
            }
        }

        Ok(true)
    }
}
