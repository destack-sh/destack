use std::sync::atomic::Ordering;

use crate::shared::gc::{SharedGcPhase, SharedTraceWork};
use crate::shared::managed::space::{checked_slot_offset, checked_storage_offset};
use crate::shared::managed::{SharedManagedLocation, SharedManagedSpace};
use crate::{
    GcKind, GcStats, HeapError, HeapResult, HeapSpace, SharedManagedReference,
    visit_shared_references_in_reader, visit_shared_references_in_reader_range,
};

/// The maximum references to drain from the shared trace queue at once.
const SHARED_MARK_BATCH_LEN: usize = 64;

impl SharedManagedSpace {
    /// Start one shared managed mark phase over explicit roots.
    pub(crate) fn start_mark(
        &self,
        roots: impl IntoIterator<Item = SharedManagedReference>,
    ) -> HeapResult<()> {
        // lifecycle
        let _lifecycle = self.gc.lock_lifecycle();

        // phase
        if self.gc.phase() != SharedGcPhase::Idle {
            return Err(HeapError::SharedCollectionActive);
        }

        // cycle state
        self.gc.marks.start_cycle();
        self.gc.trace_queue.clear();
        self.gc.clear_small_span_work();
        self.gc.open_mark_publication();
        self.gc.sweep_cursor.store(0, Ordering::Release);
        self.gc.mark_publishers.store(0, Ordering::Release);
        self.gc.mark_inflight.store(0, Ordering::Release);
        self.gc.freed_allocations.store(0, Ordering::Release);
        self.gc.freed_bytes.store(0, Ordering::Release);
        self.gc.set_phase(SharedGcPhase::Mark);

        // explicit roots
        self.queue_unmarked_references(roots)?;

        Ok(())
    }

    /// Perform one full shared managed collection over explicit roots.
    pub fn collect_full(
        &self,
        roots: impl IntoIterator<Item = SharedManagedReference>,
    ) -> HeapResult<GcStats> {
        self.start_mark(roots)?;

        // concurrent mark
        while !self.mark_idle() {
            self.mark_step([], usize::MAX)?;
        }

        while !self.try_start_sweep()? {}

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
        roots: impl IntoIterator<Item = SharedManagedReference>,
        work_items: usize,
    ) -> HeapResult<()> {
        // phase
        if self.gc.phase() != SharedGcPhase::Mark {
            return Err(HeapError::SharedCollectionNotMarking);
        }

        // newly discovered roots
        self.queue_unmarked_references(roots)?;

        let mut work_done = 0usize;

        // mark queue
        while work_done < work_items {
            let batch_len = (work_items - work_done).min(SHARED_MARK_BATCH_LEN);
            let mut batch = self.gc.trace_queue.pop_batch(batch_len);

            if batch.is_empty() {
                break;
            }

            // trace nearby storage together
            self.sort_trace_batch(&mut batch)?;

            self.gc
                .mark_inflight
                .fetch_add(batch.len(), Ordering::AcqRel);

            let trace_result = self.trace_batch(&batch);
            self.gc
                .mark_inflight
                .fetch_sub(batch.len(), Ordering::AcqRel);
            trace_result?;
            work_done += batch.len();
        }

        Ok(())
    }

    /// Trace one sorted shared mark batch.
    fn trace_batch(&self, batch: &[SharedTraceWork]) -> HeapResult<()> {
        let mut start = 0usize;

        // trace small spans together and fall back to large references otherwise
        while start < batch.len() {
            let work = batch[start];

            if let SharedTraceWork::Reference(reference) = work {
                self.trace_reference(reference)?;
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

            self.trace_small_span_work(span_index)?;
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

        // sweep cursor
        self.gc.sweep_cursor.store(0, Ordering::Release);
        self.gc.set_phase(SharedGcPhase::Sweep);

        Ok(true)
    }

    /// Perform bounded shared sweep work.
    pub(crate) fn sweep_step(&self, work_items: usize) -> HeapResult<Option<GcStats>> {
        let store = self.store.read();
        let reference_len = store.references.len();
        drop(store);

        let mut work_done = 0usize;
        let mut released_references = Vec::new();

        // reference table
        while work_done < work_items {
            let index = self.gc.sweep_cursor.fetch_add(1, Ordering::AcqRel);
            if index >= reference_len {
                break;
            }

            work_done += 1;

            let Some(record) = self.store.read().references.get(index).copied() else {
                continue;
            };

            let reference_id = index.checked_add(1).ok_or(HeapError::InvariantOverflow {
                context: "shared managed reference id",
            })?;
            let reference_id = u32::try_from(reference_id).map_err(|_| {
                HeapError::InvalidSharedManagedReferenceId {
                    id: reference_id as u64,
                }
            })?;
            let reference = SharedManagedReference::new(reference_id);

            if record.is_vacant() || self.gc.marks.contains(reference) {
                continue;
            }

            released_references.push(reference_id);
        }

        // reclaimed references
        for reference_id in released_references {
            let released_bytes = self.free_reference(reference_id)?;

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

    /// Mark one shared managed reference and queue its children.
    fn trace_reference(&self, reference: SharedManagedReference) -> HeapResult<()> {
        let Some(record) = self.reference_entry(reference) else {
            return Err(HeapError::InvalidSharedManagedReference { reference });
        };

        // live entry
        if record.is_vacant() {
            return Err(HeapError::InvalidSharedManagedReference { reference });
        }

        // mark bit
        if !self.gc.marks.mark(reference) {
            return Ok(());
        }

        let mut edge_buffer = Vec::new();
        let mut first_reader_error = None;
        let scan = self.scan(reference)?;

        // payload scan
        let trace_result = visit_shared_references_in_reader(
            &scan,
            SharedManagedReference::BYTE_LEN,
            |start, buffer| match self.read_bytes_into(reference, start, buffer) {
                Ok(()) => true,
                Err(error) => {
                    first_reader_error.get_or_insert(error);
                    false
                }
            },
            |reference: SharedManagedReference| {
                if !reference.is_null() {
                    edge_buffer.push(reference);
                }
            },
        );

        // payload errors
        if let Err(error) = trace_result {
            if let Some(error) = first_reader_error {
                return Err(error);
            }

            return Err(error);
        }

        // discovered edges
        self.queue_references(edge_buffer)?;

        Ok(())
    }

    /// Trace one shared small-span work item under one span read.
    fn trace_small_span_work(&self, span_index: usize) -> HeapResult<()> {
        let Some(span_work) = self.gc.small_span_work(span_index) else {
            return Ok(());
        };

        // keep draining until this span really goes idle
        loop {
            let pending_slots = span_work.drain_slots();

            if pending_slots.is_empty() {
                if !span_work.has_more_slots() {
                    return Ok(());
                }

                continue;
            }

            let mut edge_buffer = Vec::new();

            {
                let store = self.store.read();
                let Some(span) = store.small.spans.get(span_index).cloned() else {
                    return Err(HeapError::MissingSpan { span_index });
                };
                let span = span.read();
                let span_bytes = store.small.span_bytes;

                // shared small-span scan
                for slot_index in pending_slots {
                    let reference_id = span
                        .reference_ids
                        .get(slot_index)
                        .copied()
                        .flatten()
                        .ok_or(HeapError::MissingSmallSlot {
                            span_index,
                            slot_index,
                        })?;
                    let reference = SharedManagedReference::new(reference_id);

                    if !self.gc.marks.mark(reference) {
                        continue;
                    }

                    let shape_id = span.shape_ids.get(slot_index).copied().flatten().ok_or(
                        HeapError::MissingSmallSlot {
                            span_index,
                            slot_index,
                        },
                    )?;
                    let shape =
                        store
                            .shape_table
                            .shape(shape_id)
                            .ok_or(HeapError::InvalidShapeId {
                                index: shape_id.index(),
                            })?;

                    if !shape.scan.has_shared_reference() {
                        continue;
                    }

                    let slot_offset = checked_slot_offset(span.size_class, slot_index)?;
                    let mut first_reader_error = None;

                    // payload scan
                    let trace_result = visit_shared_references_in_reader(
                        &shape.scan,
                        SharedManagedReference::BYTE_LEN,
                        |start, buffer| {
                            let read_offset =
                                match checked_storage_offset(slot_offset, start, span_bytes) {
                                    Ok(offset) => offset,
                                    Err(error) => {
                                        first_reader_error.get_or_insert(error);

                                        return false;
                                    }
                                };

                            match self.arena.fill_bytes_from(&span.pages, read_offset, buffer) {
                                Ok(()) => true,
                                Err(error) => {
                                    first_reader_error.get_or_insert(error);
                                    false
                                }
                            }
                        },
                        |reference: SharedManagedReference| {
                            if !reference.is_null() {
                                edge_buffer.push(reference);
                            }
                        },
                    );

                    // payload errors
                    if let Err(error) = trace_result {
                        if let Some(error) = first_reader_error {
                            return Err(error);
                        }

                        return Err(error);
                    }
                }
            }

            // discovered edges
            self.queue_references(edge_buffer)?;

            if !span_work.has_more_slots() {
                return Ok(());
            }
        }
    }

    /// Sort one shared mark batch by storage locality before tracing it.
    fn sort_trace_batch(&self, batch: &mut [SharedTraceWork]) -> HeapResult<()> {
        let mut keyed_batch = Vec::with_capacity(batch.len());

        // gather stable storage keys once before sorting
        for &work in &*batch {
            let key = self.trace_order_key(work)?;
            keyed_batch.push((key, work));
        }

        // sort small spans and large entries into a more contiguous walk
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
            SharedTraceWork::Reference(reference) => {
                let Some(record) = self.reference_entry(reference) else {
                    return Err(HeapError::InvalidSharedManagedReference { reference });
                };
                let Some(location) = record.location() else {
                    return Err(HeapError::InvalidSharedManagedReference { reference });
                };
                let SharedManagedLocation::Large(entry_id) = location else {
                    return Err(HeapError::InvalidSharedManagedReference { reference });
                };
                let large_base = 1_u64 << 63;

                Ok(large_base | entry_id.id())
            }
        }
    }

    /// Free one shared managed reference by stable reference id.
    fn free_reference(&self, reference_id: u32) -> HeapResult<u64> {
        let reference = SharedManagedReference::new(reference_id);
        let Some(record) = self.reference_entry(reference) else {
            return Err(HeapError::InvalidSharedManagedReference { reference });
        };
        let Some(location) = record.location() else {
            return Err(HeapError::InvalidSharedManagedReference { reference });
        };
        let released_bytes = record.byte_len() as u64;
        let mut store = self.store.write();

        // usage
        store
            .usage
            .check_free(released_bytes, HeapSpace::SharedManaged)?;

        // location release
        match location {
            SharedManagedLocation::Small(slot) => {
                self.release_small_slot(&mut store, slot)?;
            }
            SharedManagedLocation::Large(entry_id) => {
                let Some(entry) = store.large.entries.get(entry_id.index()?).cloned() else {
                    return Err(HeapError::MissingLargeEntry {
                        entry_id: entry_id.id(),
                    });
                };
                let pages = {
                    let mut entry = entry.write();
                    if !entry.is_live {
                        return Err(HeapError::MissingLargeEntry {
                            entry_id: entry_id.id(),
                        });
                    }

                    let pages = entry.pages;
                    entry.retire();

                    pages
                };

                store.large.free_large_entry_ids.push(entry_id.id());
                store.page_run_cache.release_page_view(&self.arena, pages)?;
            }
        }

        // reference release
        store.usage.free(released_bytes, HeapSpace::SharedManaged)?;
        self.retire_reference(&mut store, reference_id)?;

        Ok(released_bytes)
    }

    /// Finish one completed shared collection cycle.
    fn finish_collection(&self) -> HeapResult<GcStats> {
        // lifecycle
        let _lifecycle = self.gc.lock_lifecycle();
        let mut store = self.store.write();
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
        self.gc.trace_queue.clear();
        self.gc.clear_small_span_work();

        // cycle summary
        store.gc_state.record_cycle(GcKind::Full, stats)?;

        // idle publication
        self.gc.set_phase(SharedGcPhase::Idle);
        self.gc.open_mark_publication();

        Ok(stats)
    }

    /// Record one shared managed write barrier after one completed store.
    pub(crate) fn write_shared_barrier(
        &self,
        reference: SharedManagedReference,
        byte_offset: usize,
        byte_len: usize,
    ) -> HeapResult<()> {
        // publication
        let Some(_publication) = self.gc.begin_mark_publication() else {
            return Ok(());
        };

        let mut edge_buffer = Vec::new();
        let mut first_reader_error = None;
        let scan = self.scan(reference)?;

        // written range
        let trace_result = visit_shared_references_in_reader_range(
            &scan,
            byte_offset,
            byte_len,
            SharedManagedReference::BYTE_LEN,
            |start, buffer| match self.read_bytes_into(reference, start, buffer) {
                Ok(()) => true,
                Err(error) => {
                    first_reader_error.get_or_insert(error);
                    false
                }
            },
            |reference: SharedManagedReference| {
                if !reference.is_null() {
                    edge_buffer.push(reference);
                }
            },
        );

        // reader errors
        if let Err(error) = trace_result {
            if let Some(error) = first_reader_error {
                return Err(error);
            }

            return Err(error);
        }

        // published edges
        self.queue_references(edge_buffer)?;

        Ok(())
    }

    /// Record one shared managed write barrier from one caller-provided byte slice.
    pub(crate) fn write_shared_barrier_bytes(
        &self,
        reference: SharedManagedReference,
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
            SharedManagedReference::BYTE_LEN,
            |start, buffer| {
                let local_start = start.saturating_sub(byte_offset);
                let Some(local_end) = local_start.checked_add(buffer.len()) else {
                    return false;
                };
                let Some(window) = bytes.get(local_start..local_end) else {
                    return false;
                };

                buffer.copy_from_slice(window);
                true
            },
            |reference: SharedManagedReference| {
                if !reference.is_null() {
                    edge_buffer.push(reference);
                }
            },
        );

        // reader errors
        if let Err(error) = trace_result {
            return Err(error);
        }

        // published edges
        self.queue_references(edge_buffer)?;

        Ok(())
    }

    /// Publish one exact shared managed reference after one completed store.
    pub(crate) fn publish_shared_edge(&self, reference: SharedManagedReference) -> HeapResult<()> {
        // publication
        let Some(_publication) = self.gc.begin_mark_publication() else {
            return Ok(());
        };

        if reference.is_null() {
            return Ok(());
        }

        self.queue_reference(reference)
    }

    /// Publish one newly allocated shared managed reference into the active cycle.
    pub(crate) fn publish_shared_allocation(
        &self,
        reference: SharedManagedReference,
        has_initial_edges: bool,
    ) -> HeapResult<()> {
        // sweeping allocations stay live in the active cycle
        if self.gc.phase() == SharedGcPhase::Sweep {
            self.gc.marks.mark(reference);

            return Ok(());
        }

        // publication
        let Some(_publication) = self.gc.begin_mark_publication() else {
            if self.gc.phase() == SharedGcPhase::Sweep {
                self.gc.marks.mark(reference);
            }

            return Ok(());
        };

        // new allocations without initial shared edges can stay black
        if !has_initial_edges {
            self.gc.marks.mark(reference);

            return Ok(());
        }

        // queue the new object so the active cycle traces its initial payload
        self.queue_reference(reference)
    }

    /// Queue explicit roots that are not already marked in this cycle.
    fn queue_unmarked_references(
        &self,
        references: impl IntoIterator<Item = SharedManagedReference>,
    ) -> HeapResult<()> {
        for reference in references {
            if reference.is_null() {
                continue;
            }

            let Some(_reference_index) = reference.id().checked_sub(1).map(|id| id as usize) else {
                return Err(HeapError::InvalidSharedManagedReference { reference });
            };

            if self.gc.marks.contains(reference) {
                continue;
            }

            self.queue_reference_work(reference)?;
        }

        Ok(())
    }

    /// Queue one shared reference for later trace work.
    fn queue_reference(&self, reference: SharedManagedReference) -> HeapResult<()> {
        self.queue_references([reference])
    }

    /// Queue shared references for later trace work.
    fn queue_references(
        &self,
        references: impl IntoIterator<Item = SharedManagedReference>,
    ) -> HeapResult<()> {
        // nulls
        for reference in references {
            if reference.is_null() {
                continue;
            }

            // stable id
            let Some(_reference_index) = reference.id().checked_sub(1).map(|id| id as usize) else {
                return Err(HeapError::InvalidSharedManagedReference { reference });
            };

            self.queue_reference_work(reference)?;
        }

        Ok(())
    }

    /// Queue one shared reference as span work or direct reference work.
    fn queue_reference_work(&self, reference: SharedManagedReference) -> HeapResult<()> {
        let Some(record) = self.reference_entry(reference) else {
            return Err(HeapError::InvalidSharedManagedReference { reference });
        };
        let Some(location) = record.location() else {
            return Err(HeapError::InvalidSharedManagedReference { reference });
        };

        // small references queue one span work item with pending slots
        if let SharedManagedLocation::Small(slot) = location {
            let slot_count = {
                let store = self.store.read();
                let Some(span) = store.small.spans.get(slot.span_index()).cloned() else {
                    return Err(HeapError::MissingSpan {
                        span_index: slot.span_index(),
                    });
                };
                let span = span.read();

                span.slot_count
            };
            let span_work = self
                .gc
                .ensure_small_span_work(slot.span_index(), slot_count);

            if span_work.queue_slot(slot.slot_index()) {
                self.gc
                    .trace_queue
                    .push(SharedTraceWork::SmallSpan(slot.span_index()));
            }

            return Ok(());
        }

        // large references stay as direct trace work
        self.gc
            .trace_queue
            .push(SharedTraceWork::Reference(reference));

        Ok(())
    }
}
