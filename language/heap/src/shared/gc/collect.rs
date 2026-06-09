use std::sync::atomic::Ordering;

use destack_mir::TraceTable;

use crate::shared::gc::{GcPhase, GcWorker, MarkWork};
use crate::shared::storage::{HeapPlace, HeapStorage, small_slot_offset};
use crate::{
    GcStats, HeapConfigurationError, HeapError, HeapGcStateError, HeapResult, ReferenceInput,
    ReferenceRange, SharedHeapReference, SizeClassTableError, visit_references,
};

impl HeapStorage {
    /// Start one shared heap mark phase over the given roots.
    pub(crate) fn start_mark(&self, roots: &[SharedHeapReference]) -> HeapResult<()> {
        // lifecycle
        let _lifecycle = self.gc.lock_lifecycle();

        // phase
        if self.gc.phase() != GcPhase::Idle {
            return Err(HeapError::gc_state(HeapGcStateError::SharedGcActive));
        }

        // cycle state
        self.gc.advance_mark_epoch();
        self.gc.trace_queue.clear();
        self.gc.open_mark_publication();
        self.gc.reset_sweep();
        self.gc.mark_publishers.store(0, Ordering::Release);
        self.gc.mark_inflight.store(0, Ordering::Release);
        self.gc.set_phase(GcPhase::Mark);

        // begin with roots
        self.mark_roots(None, roots)?;

        Ok(())
    }

    /// Perform one full shared heap collection over the given roots.
    pub(crate) fn collect_full(
        &self,
        roots: &[SharedHeapReference],
        trace_table: &TraceTable,
    ) -> HeapResult<GcStats> {
        // mark phase
        self.start_mark(roots)?;

        // concurrent mark
        while !self.mark_idle() {
            self.step_mark(None, &[], usize::MAX, trace_table)?;
        }

        // sweep phase
        if !self.start_sweep_when_drained()? {
            return Err(HeapError::gc_state(HeapGcStateError::SharedGcActive));
        }

        // drain sweep work synchronously
        loop {
            if let Some(stats) = self.step_sweep(usize::MAX)?.completed_stats() {
                return Ok(stats);
            }
        }
    }

    /// Perform bounded shared mark work.
    pub(crate) fn step_mark(
        &self,
        worker: Option<&GcWorker>,
        roots: &[SharedHeapReference],
        budget_bytes: usize,
        trace_table: &TraceTable,
    ) -> HeapResult<()> {
        // phase
        if self.gc.phase() != GcPhase::Mark {
            return Err(HeapError::gc_state(HeapGcStateError::SharedGcNotMarking));
        }

        // newly discovered roots
        self.mark_roots(worker, roots)?;

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
            let traced = match trace_result {
                Ok(result) => result,
                Err(error) => {
                    self.gc
                        .mark_inflight
                        .fetch_sub(batch.len(), Ordering::AcqRel);

                    return Err(error);
                }
            };

            // return work that did not fit this step budget
            for work in batch[traced.work_count..].iter().copied() {
                self.gc.trace_queue.push(worker, work);
            }

            // publish requeued work before dropping the in-flight count
            self.gc
                .mark_inflight
                .fetch_sub(batch.len(), Ordering::AcqRel);

            marked_bytes += traced.byte_len;
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
        worker: Option<&GcWorker>,
        batch: &[MarkWork],
        budget_bytes: usize,
        trace_table: &TraceTable,
    ) -> HeapResult<TracedBatch> {
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
                MarkWork::Large {
                    reference,
                    start: range_start,
                } => {
                    marked_bytes +=
                        self.trace_large_range(worker, reference, range_start, trace_table)?;
                    start += 1;
                }

                // adjacent small-span items can share one scan
                MarkWork::SmallSpan(span_index) => {
                    let mut end = start + 1;

                    // skip duplicate work for the same span
                    while end < batch.len() {
                        if batch[end] != MarkWork::SmallSpan(span_index) {
                            break;
                        }

                        end += 1;
                    }

                    marked_bytes += self.trace_small_span(
                        worker,
                        span_index,
                        budget_bytes - marked_bytes,
                        trace_table,
                    )?;
                    start = end;
                }
            }
        }

        Ok(TracedBatch {
            byte_len: marked_bytes,
            work_count: start,
        })
    }

    /// Return whether concurrent mark is currently drained.
    pub(crate) fn mark_idle(&self) -> bool {
        self.gc.phase() == GcPhase::Mark && self.gc.mark_drained()
    }

    /// Trace one page-sized range from one shared large block.
    fn trace_large_range(
        &self,
        worker: Option<&GcWorker>,
        reference: SharedHeapReference,
        start: usize,
        trace_table: &TraceTable,
    ) -> HeapResult<usize> {
        // resolve and verify the large block
        let Some(extent) = self.resolve_extent(reference) else {
            return Err(HeapError::invalid_shared_heap_reference(reference));
        };
        let HeapPlace::LargeBlock(_) = extent.storage else {
            return Err(HeapError::invalid_shared_heap_reference(reference));
        };

        // skip empty ranges and noscan payloads
        let trace_map = self.trace_map_for_place_ref(extent.storage, trace_table)?;
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

        // payload scan
        let base_address = self.mapping.base_address() + extent.base.offset();
        visit_references::<SharedHeapReference>(
            &trace_map,
            ReferenceInput::mapped(base_address),
            ReferenceRange::bytes(start, range_len),
            &mut |reference| {
                if !reference.is_null() {
                    self.mark_reference(worker, reference)?;
                }

                Ok(())
            },
        )?;

        // continue this large block on a later step
        let next_start = start + range_len;

        if next_start < extent.byte_len {
            self.gc.trace_queue.push(
                worker,
                MarkWork::Large {
                    reference,
                    start: next_start,
                },
            );
        }

        Ok(range_len)
    }

    /// Trace one shared small-span work item.
    fn trace_small_span(
        &self,
        worker: Option<&GcWorker>,
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
            let slot_bytes = span.class.size_class.max(1);
            let mark_epoch = self.gc.mark_epoch();
            let Some(slot_index) = span.claim_next_marked_slot(mark_epoch) else {
                return Ok(scanned_bytes);
            };
            let slot_offset = small_slot_offset(span.class.size_class, slot_index);
            let trace_map = span.trace_map(slot_index, trace_table)?;
            scanned_bytes += slot_bytes;

            // noscan slots cost one claimed unit only
            if !trace_map.has_shared_reference() {
                continue;
            }

            // payload scan
            let base_address = self.mapping.base_address() + span.first_offset + slot_offset;
            visit_references::<SharedHeapReference>(
                &trace_map,
                ReferenceInput::mapped(base_address),
                ReferenceRange::All,
                &mut |reference| {
                    if !reference.is_null() {
                        self.mark_reference(worker, reference)?;
                    }

                    Ok(())
                },
            )?;
        }

        // keep this span queued when the step budget runs out
        self.gc
            .trace_queue
            .push(worker, MarkWork::SmallSpan(span_index));

        Ok(scanned_bytes)
    }
}

/// Result from tracing one shared mark batch.
struct TracedBatch {
    /// The traced payload byte length.
    byte_len: usize,
    /// The number of consumed work items.
    work_count: usize,
}
