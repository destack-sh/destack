use std::sync::atomic::Ordering;

use crate::shared::gc::{SharedGcPhase, SharedGcWorker, SharedTraceWork};
use crate::shared::space::{SharedHeapPlace, SharedHeapSpace, small_slot_offset};
use crate::{
    GcStats, HeapError, HeapResult, SharedHeapReference, scan_shared_references,
    scan_shared_references_in_range,
};

impl SharedHeapSpace {
    /// Start one shared heap mark phase over the given roots.
    pub(crate) fn start_mark(&self, roots: &[SharedHeapReference]) -> HeapResult<()> {
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
    pub fn collect_full(&self, roots: &[SharedHeapReference]) -> HeapResult<GcStats> {
        // mark phase
        self.start_mark(roots)?;

        // concurrent mark
        while !self.mark_idle() {
            self.mark_step(None, &[], usize::MAX)?;
        }

        // sweep phase
        if !self.try_start_sweep()? {
            return Err(HeapError::SharedCollectionActive);
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
    ) -> HeapResult<()> {
        // phase
        if self.gc.phase() != SharedGcPhase::Mark {
            return Err(HeapError::SharedCollectionNotMarking);
        }

        // newly discovered roots
        self.queue_unmarked_references(worker, roots)?;

        // mark queue
        let mut marked_bytes = 0usize;
        let batch_capacity = self.trace_batch_capacity();
        while marked_bytes < budget_bytes {
            // reserve before popping so termination sees in-flight batches
            let batch_len = batch_capacity;
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
            let trace_result = self.trace_batch(worker, &batch, budget_bytes - marked_bytes);
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
    fn trace_batch_capacity(&self) -> usize {
        // derive a bounded batch from the smallest possible small allocation
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
        budget_bytes: usize,
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
                // large allocations are already page-sliced
                SharedTraceWork::Large {
                    reference,
                    start: range_start,
                } => {
                    marked_bytes += self.trace_large_range(worker, reference, range_start)?;
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

    /// Trace one page-sized range from one shared large allocation.
    fn trace_large_range(
        &self,
        worker: Option<&SharedGcWorker>,
        reference: SharedHeapReference,
        start: usize,
    ) -> HeapResult<usize> {
        // resolve and verify the large allocation
        let Some(location) = self.resolve_location(reference) else {
            return Err(HeapError::InvalidSharedHeapReference { reference });
        };
        let SharedHeapPlace::Large(_) = location.place else {
            return Err(HeapError::InvalidSharedHeapReference { reference });
        };

        // skip empty ranges and noscan payloads
        let reference_map = self.reference_map_for_place(location.place)?;
        if !reference_map.has_shared_reference() {
            return Ok(location.byte_len);
        }

        // range already fully traced
        if start >= location.byte_len {
            return Ok(0);
        }

        // scan at most one allocator page
        let range_len = self.allocator.page_bytes().min(location.byte_len - start);

        let mut reference_buffer = Vec::new();

        // payload scan
        let base_address = self.mapping.base_address() + location.base.offset();
        scan_shared_references_in_range(
            &reference_map,
            start,
            range_len,
            base_address,
            &mut reference_buffer,
        )?;
        reference_buffer.retain(|reference| !reference.is_null());

        // discovered references
        self.queue_references(worker, reference_buffer)?;

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

        Ok(range_len)
    }

    /// Trace one shared small-span work item.
    fn trace_small_span_work(
        &self,
        worker: Option<&SharedGcWorker>,
        span_index: usize,
        budget_bytes: usize,
    ) -> HeapResult<usize> {
        let mut scanned_bytes = 0usize;

        // load the span handle once, then scan outside the space lock
        let span = {
            let store = self.state.read();
            let Some(span) = store.small.spans.get(span_index).cloned() else {
                return Err(HeapError::MissingSpan { span_index });
            };

            span
        };

        // keep draining until this span really goes idle
        while scanned_bytes < budget_bytes {
            let scan_slots = {
                let marked = span.marked.snapshot();
                let mut slot_indices = Vec::new();
                let remaining_bytes = budget_bytes - scanned_bytes;
                let slot_bytes = span.class.size_class.max(1);
                let mut claimed_bytes = 0usize;

                // claim marked slots that were not scanned yet
                for (start, len) in marked.set_ranges() {
                    for slot_index in start..start + len {
                        if !slot_indices.is_empty() && claimed_bytes >= remaining_bytes {
                            break;
                        }

                        if span.scanned.try_set(slot_index) {
                            slot_indices.push(slot_index);
                            claimed_bytes += slot_bytes;
                        }
                    }

                    if !slot_indices.is_empty() && claimed_bytes >= remaining_bytes {
                        break;
                    }
                }

                if slot_indices.is_empty() {
                    span.is_queued_for_scan.store(false, Ordering::Release);
                    return Ok(scanned_bytes);
                }

                // claim bounded marked slots
                let mut scan_slots = Vec::with_capacity(slot_indices.len());

                for slot_index in slot_indices {
                    let slot_offset = small_slot_offset(span.class.size_class, slot_index);
                    let reference_map = span.reference_map(slot_index);
                    scan_slots.push((
                        span.first_offset,
                        slot_offset,
                        span.class.size_class,
                        reference_map,
                    ));
                }

                scan_slots
            };

            // scan claimed slots
            let mut reference_buffer = Vec::new();
            for (span_offset, slot_offset, slot_bytes, reference_map) in scan_slots {
                scanned_bytes += slot_bytes.max(1);

                // noscan slots cost one claimed unit only
                if !reference_map.has_shared_reference() {
                    continue;
                }

                // payload scan
                let base_address = self.mapping.base_address() + span_offset + slot_offset;
                scan_shared_references(&reference_map, base_address, &mut reference_buffer)?;
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
