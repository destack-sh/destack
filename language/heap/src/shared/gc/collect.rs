use std::sync::Arc;
use std::sync::atomic::Ordering;

use crate::shared::gc::{SharedGcPhase, SharedGcWorker, SharedTraceWork};
use crate::shared::space::{
    SharedHeapPlace, SharedHeapSpace, checked_place_offset, checked_slot_offset,
};
use crate::{
    AccountingRegion, GcKind, GcStats, HeapError, HeapResult, SharedHeapReference,
    slot_reference_map, visit_shared_references_in_reader, visit_shared_references_in_reader_range,
};

/// The maximum references to drain from the shared trace queue at once.
const SHARED_MARK_BATCH_LEN: usize = 64;

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

        if !self.try_start_sweep()? {
            return Err(HeapError::SharedCollectionActive);
        }

        // incremental sweep
        loop {
            if let Some(stats) = self.sweep_step(usize::MAX)? {
                return Ok(stats);
            }
        }
    }

    /// Perform bounded shared mark work.
    pub(crate) fn mark_step(
        &self,
        worker: Option<&SharedGcWorker>,
        roots: impl IntoIterator<Item = SharedHeapReference>,
        work_items: usize,
    ) -> HeapResult<()> {
        // phase
        if self.gc.phase() != SharedGcPhase::Mark {
            return Err(HeapError::SharedCollectionNotMarking);
        }

        // newly discovered roots
        self.queue_unmarked_references(worker, roots)?;

        // mark queue
        let mut work_done = 0usize;
        while work_done < work_items {
            let batch_len = (work_items - work_done).min(SHARED_MARK_BATCH_LEN);
            let mut batch = self.gc.trace_queue.pop_batch(worker, batch_len);
            if batch.is_empty() {
                break;
            }

            // trace nearby allocations together
            self.sort_trace_batch(&mut batch)?;

            self.gc
                .mark_inflight
                .fetch_add(batch.len(), Ordering::AcqRel);

            let trace_result = self.trace_batch(worker, &batch);
            self.gc
                .mark_inflight
                .fetch_sub(batch.len(), Ordering::AcqRel);
            trace_result?;
            work_done += batch.len();
        }

        Ok(())
    }

    /// Trace one sorted shared mark batch.
    fn trace_batch(
        &self,
        worker: Option<&SharedGcWorker>,
        batch: &[SharedTraceWork],
    ) -> HeapResult<()> {
        let mut start = 0usize;

        // trace small spans together and fall back to large references otherwise
        while start < batch.len() {
            let work = batch[start];

            if let SharedTraceWork::Large {
                reference,
                start: range_start,
            } = work
            {
                self.trace_large_range(worker, reference, range_start)?;
                start += 1;

                continue;
            }

            let SharedTraceWork::SmallSpan(span_index) = work else {
                return Err(HeapError::InvariantOverflow {
                    context: "shared trace work",
                });
            };

            let mut end = start + 1;

            while end < batch.len() {
                if batch[end] != SharedTraceWork::SmallSpan(span_index) {
                    break;
                }

                end += 1;
            }

            self.trace_small_span_work(worker, span_index)?;
            start = end;
        }

        Ok(())
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
    pub(crate) fn sweep_step(&self, work_items: usize) -> HeapResult<Option<GcStats>> {
        let references = self.gc.sweep_references.lock().clone();
        let reference_len = references.len();

        let mut work_done = 0usize;
        let mut released_references = Vec::new();

        // sweep cursor
        while work_done < work_items {
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
            let freed_allocations = self.gc.freed_allocations.fetch_add(1, Ordering::AcqRel);
            if freed_allocations == usize::MAX {
                return Err(HeapError::InvariantOverflow {
                    context: "shared gc freed allocation count",
                });
            }

            let freed_bytes = self
                .gc
                .freed_bytes
                .fetch_add(released_bytes, Ordering::AcqRel);
            if freed_bytes > u64::MAX - released_bytes {
                return Err(HeapError::InvariantOverflow {
                    context: "shared gc freed bytes",
                });
            }
        }

        // end of sweep
        if self.gc.sweep_cursor.load(Ordering::Acquire) >= reference_len {
            return self.finish_collection().map(Some);
        }

        Ok(None)
    }

    /// Trace one page-sized range from one shared large allocation.
    fn trace_large_range(
        &self,
        worker: Option<&SharedGcWorker>,
        reference: SharedHeapReference,
        start: usize,
    ) -> HeapResult<()> {
        let Some(location) = self.resolve_location(reference) else {
            return Err(HeapError::InvalidSharedHeapReference { reference });
        };
        let SharedHeapPlace::Large(_) = location.place else {
            return Err(HeapError::InvalidSharedHeapReference { reference });
        };

        let scan = self.scan(reference)?;
        if !scan.has_shared_reference() {
            return Ok(());
        }
        if start >= location.byte_len {
            return Ok(());
        }
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

        let next_start = start
            .checked_add(range_len)
            .ok_or(HeapError::TraceOffsetOverflow {
                start,
                width: range_len,
            })?;

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
    ) -> HeapResult<()> {
        let (span, span_bytes) = {
            let store = self.state.read();
            let Some(span) = store.small.spans.get(span_index).cloned() else {
                return Err(HeapError::MissingSpan { span_index });
            };

            (span, store.small.span_bytes)
        };

        // keep draining until this span really goes idle
        loop {
            let scan_slots = {
                let mut span = span.write();
                let mut slot_indices = Vec::new();

                // marked minus scanned
                span.marked.visit_set_ranges(|start, len| {
                    for slot_index in start..start + len {
                        if !span.scanned.contains(slot_index) {
                            slot_indices.push(slot_index);
                        }
                    }
                });

                if slot_indices.is_empty() {
                    span.is_queued_for_scan = false;
                    return Ok(());
                }

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
                    scan_slots.push((slot_offset, span.pages.clone(), reference_map));
                }

                scan_slots
            };

            let mut edge_buffer = Vec::new();

            // shared small-span scan
            for (slot_offset, pages, reference_map) in scan_slots {
                if !reference_map.has_shared_reference() {
                    continue;
                }
                // payload scan
                let trace_result = visit_shared_references_in_reader(
                    &reference_map,
                    |start, buffer| {
                        let read_offset = checked_place_offset(slot_offset, start, span_bytes)?;

                        self.allocator.fill_bytes_from(&pages, read_offset, buffer)
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
    }

    /// Sort one shared mark batch by allocation locality before tracing it.
    fn sort_trace_batch(&self, batch: &mut [SharedTraceWork]) -> HeapResult<()> {
        let mut keyed_batch = Vec::with_capacity(batch.len());

        // gather stable allocation keys once before sorting
        for &work in &*batch {
            let key = self.trace_order_key(work)?;
            keyed_batch.push((key, work));
        }

        // sort small spans and large allocations into a more contiguous walk
        keyed_batch.sort_unstable_by_key(|(key, _)| *key);

        // write the reordered batch back in place
        for (slot, (_, reference)) in batch.iter_mut().zip(keyed_batch) {
            *slot = reference;
        }

        Ok(())
    }

    /// Return one stable sort key for tracing this queued work.
    fn trace_order_key(&self, work: SharedTraceWork) -> HeapResult<u64> {
        match work {
            SharedTraceWork::SmallSpan(span_index) => Ok(span_index as u64),
            SharedTraceWork::Large { reference, .. } => {
                let Some(location) = self.resolve_location(reference) else {
                    return Err(HeapError::InvalidSharedHeapReference { reference });
                };
                let SharedHeapPlace::Large(allocation_id) = location.place else {
                    return Err(HeapError::InvalidSharedHeapReference { reference });
                };
                let large_base = 1_u64 << 63;

                Ok(large_base | allocation_id.id())
            }
        }
    }

    /// Free one shared heap reference.
    fn free_reference(&self, reference: SharedHeapReference) -> HeapResult<u64> {
        let Some(location) = self.resolve_location(reference) else {
            return Err(HeapError::InvalidSharedHeapReference { reference });
        };
        let released_bytes = location.byte_len as u64;
        let mut store = self.state.write();

        // usage
        store
            .usage
            .check_free(released_bytes, AccountingRegion::SharedHeap)?;

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
                let pages = {
                    let mut allocation = allocation.write();
                    if !allocation.is_live {
                        return Err(HeapError::MissingLargeAllocation {
                            allocation_id: allocation_id.id(),
                        });
                    }

                    let pages = allocation.pages.clone();
                    allocation.retire();

                    pages
                };

                store
                    .large
                    .free_large_allocation_ids
                    .push(allocation_id.id());
                self.unmap_page_view(&mut store, &pages)?;
                store
                    .page_run_cache
                    .release_page_view(&self.allocator, pages)?;
            }
        }

        // reference release
        store
            .usage
            .free(released_bytes, AccountingRegion::SharedHeap)?;

        Ok(released_bytes)
    }

    /// Finish one completed shared collection cycle.
    fn finish_collection(&self) -> HeapResult<GcStats> {
        // lifecycle
        let _lifecycle = self.gc.lock_lifecycle();
        let mut store = self.state.write();
        let active_bytes = self.live_mapped_bytes(&store);

        // cycle stats
        let stats = GcStats {
            freed_allocations: self.gc.freed_allocations.load(Ordering::Acquire),
            live_allocations: store.usage.allocation_count(),
            freed_bytes: self.gc.freed_bytes.load(Ordering::Acquire),
            allocated_bytes: store.usage.allocated_bytes(),
            active_bytes,
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
        store.gc.record_cycle(GcKind::Full, stats)?;

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
                let local_start = start.saturating_sub(byte_offset);
                let Some(local_end) = local_start.checked_add(buffer.len()) else {
                    return Err(HeapError::TraceOffsetOverflow {
                        start: local_start,
                        width: buffer.len(),
                    });
                };
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

            let Some(location) = self.resolve_location(reference) else {
                return Err(HeapError::InvalidSharedHeapReference { reference });
            };

            if self.is_marked_place(location.place)? {
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
        // nulls
        for reference in references {
            if reference.is_null() {
                continue;
            }

            if self.resolve_location(reference).is_none() {
                return Err(HeapError::InvalidSharedHeapReference { reference });
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
