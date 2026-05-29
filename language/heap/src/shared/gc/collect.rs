use std::sync::atomic::Ordering;

use destack_mir::TraceTable;

use crate::shared::gc::{SharedGcPhase, SharedGcWorker, SharedTraceWork};
use crate::shared::space::{SharedHeapSpace, SharedHeapStorage, small_slot_offset};
use crate::{
    GcStats, HeapConfigurationError, HeapError, HeapGcStateError, HeapResult, ReferenceInput,
    ReferenceRange, SharedHeapReference, SizeClassTableError, scan_references,
};

impl SharedHeapSpace {
    /// Start one shared heap mark phase over the given roots.
    pub(crate) fn start_mark(&self, roots: &[SharedHeapReference]) -> HeapResult<()> {
        // lifecycle
        let _lifecycle = self.gc.lock_lifecycle();

        // phase
        if self.gc.phase() != SharedGcPhase::Idle {
            return Err(HeapError::gc_state(HeapGcStateError::SharedGcActive));
        }

        // cycle state
        self.gc.advance_mark_epoch();
        self.gc.trace_queue.clear();
        self.gc.open_mark_publication();
        self.gc.reset_sweep();
        self.gc.mark_publishers.store(0, Ordering::Release);
        self.gc.mark_inflight.store(0, Ordering::Release);
        self.gc.set_phase(SharedGcPhase::Mark);

        // begin with roots
        self.queue_unmarked_references(None, roots)?;

        Ok(())
    }

    /// Perform one full shared heap collection over the given roots.
    pub fn collect_full(
        &self,
        roots: &[SharedHeapReference],
        trace_table: &TraceTable,
    ) -> HeapResult<GcStats> {
        // mark phase
        self.start_mark(roots)?;

        // concurrent mark
        while !self.mark_idle() {
            self.mark_step(None, &[], usize::MAX, trace_table)?;
        }

        // sweep phase
        if !self.try_start_sweep()? {
            return Err(HeapError::gc_state(HeapGcStateError::SharedGcActive));
        }

        // drain sweep work synchronously
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
        roots: &[SharedHeapReference],
        budget_bytes: usize,
        trace_table: &TraceTable,
    ) -> HeapResult<()> {
        // phase
        if self.gc.phase() != SharedGcPhase::Mark {
            return Err(HeapError::gc_state(HeapGcStateError::SharedGcNotMarking));
        }

        // newly discovered roots
        self.queue_unmarked_references(worker, roots)?;

        // mark queue
        let mut marked_bytes = 0usize;
        let batch_capacity = self.trace_batch_capacity()?;
        let mut batch = Vec::with_capacity(batch_capacity);
        while marked_bytes < budget_bytes {
            // reserve before popping so termination sees in-flight batches
            let batch_len = batch_capacity;
            self.gc.mark_inflight.fetch_add(batch_len, Ordering::AcqRel);

            // claim one batch from local, global, or stolen work
            self.gc.trace_queue.pop_batch(worker, batch_len, &mut batch);
            if batch.is_empty() {
                self.gc.mark_inflight.fetch_sub(batch_len, Ordering::AcqRel);
                break;
            }

            // release unused reservation when fewer items were available
            let unused = batch_len - batch.len();
            if unused > 0 {
                self.gc.mark_inflight.fetch_sub(unused, Ordering::AcqRel);
            }

            // trace claimed work
            let trace_result =
                self.trace_batch(worker, &batch, budget_bytes - marked_bytes, trace_table);
            let (traced_bytes, processed_items) = match trace_result {
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

            marked_bytes += traced_bytes;
        }

        Ok(())
    }

    /// Return the mark queue batch capacity for this space.
    fn trace_batch_capacity(&self) -> HeapResult<usize> {
        // derive a bounded batch from the smallest possible small block
        let state = self.state.read();
        let min_slot_bytes = state
            .small
            .size_classes
            .min_small_allocation_bytes()
            .ok_or(HeapError::configuration(
                HeapConfigurationError::InvalidSizeClassTable {
                    reason: SizeClassTableError::Empty,
                },
            ))?;

        Ok((state.small.span_size_bytes / min_slot_bytes).max(1))
    }

    /// Trace one shared mark batch.
    fn trace_batch(
        &self,
        worker: Option<&SharedGcWorker>,
        batch: &[SharedTraceWork],
        budget_bytes: usize,
        trace_table: &TraceTable,
    ) -> HeapResult<(usize, usize)> {
        let mut start = 0usize;
        let mut marked_bytes = 0usize;

        // trace small spans together, then trace large references directly
        while start < batch.len() {
            // stop before consuming another queued item
            if marked_bytes >= budget_bytes {
                break;
            }

            match batch[start] {
                // large blocks are already page-sliced
                SharedTraceWork::Large {
                    reference,
                    start: range_start,
                } => {
                    marked_bytes +=
                        self.trace_large_range(worker, reference, range_start, trace_table)?;
                    start += 1;
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

                    marked_bytes += self.trace_small_span_work(
                        worker,
                        span_index,
                        budget_bytes - marked_bytes,
                        trace_table,
                    )?;
                    start = end;
                }
            }
        }

        Ok((marked_bytes, start))
    }

    /// Return whether concurrent mark is currently drained.
    pub(crate) fn mark_idle(&self) -> bool {
        self.gc.phase() == SharedGcPhase::Mark && self.gc.mark_drained()
    }

    /// Trace one page-sized range from one shared large block.
    fn trace_large_range(
        &self,
        worker: Option<&SharedGcWorker>,
        reference: SharedHeapReference,
        start: usize,
        trace_table: &TraceTable,
    ) -> HeapResult<usize> {
        // resolve and verify the large block
        let Some(extent) = self.resolve_extent(reference) else {
            return Err(HeapError::invalid_shared_heap_reference(reference));
        };
        let SharedHeapStorage::LargeBlock(_) = extent.storage else {
            return Err(HeapError::invalid_shared_heap_reference(reference));
        };

        // skip empty ranges and noscan payloads
        let trace_map = self.trace_map_for_place(extent.storage, trace_table)?;
        if !trace_map.has_shared_reference() {
            return Ok(extent.byte_len);
        }

        // range already fully traced
        if start >= extent.byte_len {
            return Ok(0);
        }

        // scan at most one allocator page
        let range_len = self
            .allocator
            .page_size_bytes()
            .min(extent.byte_len - start);

        let mut reference_buffer = Vec::new();

        // payload scan
        let base_address = self.mapping.base_address() + extent.base.offset();
        scan_references::<SharedHeapReference>(
            &trace_map,
            ReferenceInput::mapped(base_address),
            ReferenceRange::bytes(start, range_len),
            &mut reference_buffer,
        )?;
        reference_buffer.retain(|reference| !reference.is_null());

        // discovered references
        self.queue_references(worker, reference_buffer)?;

        // continue this large block on a later step
        let next_start = start + range_len;

        if next_start < extent.byte_len {
            self.gc.trace_queue.push(
                worker,
                SharedTraceWork::Large {
                    reference,
                    start: next_start,
                },
            );
        }

        Ok(range_len)
    }

    /// Trace one shared small-span work item.
    fn trace_small_span_work(
        &self,
        worker: Option<&SharedGcWorker>,
        span_index: usize,
        budget_bytes: usize,
        trace_table: &TraceTable,
    ) -> HeapResult<usize> {
        let mut scanned_bytes = 0usize;

        // load the span handle once, then scan outside the space lock
        let span = {
            let store = self.state.read();
            let Some(span) = store.small.spans.get(span_index).cloned() else {
                return Err(HeapError::internal("missing span"));
            };

            span
        };
        // keep draining until this span really goes idle
        while scanned_bytes < budget_bytes {
            let scan_slots = {
                let remaining_bytes = budget_bytes - scanned_bytes;
                let slot_bytes = span.class.size_class.max(1);
                let mark_epoch = self.gc.mark_epoch();
                let max_slots = (remaining_bytes / slot_bytes).max(1);
                let slot_indices = span.claim_marked_slots(mark_epoch, max_slots);

                // no marked slots are currently available
                if slot_indices.is_empty() {
                    return Ok(scanned_bytes);
                }

                // claim bounded marked slots
                let mut scan_slots = Vec::with_capacity(slot_indices.len());

                for slot_index in slot_indices {
                    let slot_offset = small_slot_offset(span.class.size_class, slot_index);
                    let trace_map = span.trace_map(slot_index, trace_table)?;
                    scan_slots.push((
                        span.first_offset,
                        slot_offset,
                        span.class.size_class,
                        trace_map,
                    ));
                }

                scan_slots
            };

            // scan claimed slots
            let mut reference_buffer = Vec::new();
            for (span_offset, slot_offset, slot_bytes, trace_map) in scan_slots {
                scanned_bytes += slot_bytes.max(1);

                // noscan slots cost one claimed unit only
                if !trace_map.has_shared_reference() {
                    continue;
                }

                // payload scan
                let base_address = self.mapping.base_address() + span_offset + slot_offset;
                scan_references::<SharedHeapReference>(
                    &trace_map,
                    ReferenceInput::mapped(base_address),
                    ReferenceRange::All,
                    &mut reference_buffer,
                )?;
            }

            // discovered references
            reference_buffer.retain(|reference| !reference.is_null());
            self.queue_references(worker, reference_buffer)?;
        }

        // keep this span queued when the step budget runs out
        self.gc
            .trace_queue
            .push(worker, SharedTraceWork::SmallSpan(span_index));

        Ok(scanned_bytes)
    }
}
