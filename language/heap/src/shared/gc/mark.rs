use std::sync::atomic::Ordering;

use crate::shared::gc::{SharedGcPhase, SharedGcWorker, SharedTraceWork};
use crate::shared::space::{SharedHeapPlace, SharedHeapSpace};
use crate::{
    HeapError, HeapResult, SharedHeapReference, scan_shared_references_in_bytes_range,
    scan_shared_references_in_range,
};

impl SharedHeapSpace {
    /// Record one shared heap write barrier before one byte store.
    pub(crate) fn write_shared_barrier_bytes(
        &self,
        reference: SharedHeapReference,
        byte_offset: usize,
        bytes: &[u8],
    ) -> HeapResult<()> {
        // inactive collector
        let Some(_publication) = self.gc.begin_mark_publication() else {
            return Ok(());
        };

        // empty stores cannot publish references
        if bytes.is_empty() {
            return Ok(());
        }

        // overwritten bytes
        let mut reference_buffer = Vec::new();
        let Some(location) = self.resolve_location(reference) else {
            return Err(HeapError::InvalidSharedHeapReference { reference });
        };
        let trace_map = self.trace_map_for_place(location.place)?;
        let base_address = self.mapping.base_address() + location.base.offset();
        scan_shared_references_in_range(
            &trace_map,
            byte_offset,
            bytes.len(),
            base_address,
            &mut reference_buffer,
        )?;

        // inserted bytes
        scan_shared_references_in_bytes_range(
            &trace_map,
            byte_offset,
            bytes.len(),
            bytes,
            &mut reference_buffer,
        )?;
        reference_buffer.retain(|reference| !reference.is_null());

        // published references
        self.queue_references(None, reference_buffer)?;

        Ok(())
    }

    /// Publish one newly allocated shared heap reference into the active cycle.
    pub(crate) fn publish_shared_allocation(
        &self,
        reference: SharedHeapReference,
        has_initial_edges: bool,
    ) -> HeapResult<()> {
        // idle allocation needs no publication work
        let phase = self.gc.phase();
        if phase == SharedGcPhase::Idle {
            return Ok(());
        }

        // concurrent publication
        let Some(_publication) = self.gc.begin_mark_publication() else {
            if phase == SharedGcPhase::Sweep {
                let Some(location) = self.resolve_location(reference) else {
                    return Err(HeapError::InvalidSharedHeapReference { reference });
                };
                self.mark_place(location.place)?;
            }

            return Ok(());
        };

        let Some(location) = self.resolve_location(reference) else {
            return Err(HeapError::InvalidSharedHeapReference { reference });
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
    pub(super) fn queue_unmarked_references(
        &self,
        worker: Option<&SharedGcWorker>,
        references: &[SharedHeapReference],
    ) -> HeapResult<()> {
        // seed explicit roots
        for reference in references {
            if reference.is_null() {
                continue;
            }

            self.queue_reference_work(worker, *reference)?;
        }

        Ok(())
    }

    /// Queue one shared reference for later trace work.
    pub(super) fn queue_reference(
        &self,
        worker: Option<&SharedGcWorker>,
        reference: SharedHeapReference,
    ) -> HeapResult<()> {
        self.queue_references(worker, [reference])
    }

    /// Queue shared references for later trace work.
    pub(crate) fn queue_references(
        &self,
        worker: Option<&SharedGcWorker>,
        references: impl IntoIterator<Item = SharedHeapReference>,
    ) -> HeapResult<()> {
        // queue every non-null shared reference
        for reference in references {
            if reference.is_null() {
                continue;
            }

            self.queue_reference_work(worker, reference)?;
        }

        Ok(())
    }

    /// Queue one shared reference as span work or direct reference work.
    pub(super) fn queue_reference_work(
        &self,
        worker: Option<&SharedGcWorker>,
        reference: SharedHeapReference,
    ) -> HeapResult<()> {
        // resolve the reference to its physical place
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

                if span.marked.contains(slot.slot_index()) {
                    return Ok(());
                }

                span.marked.set(slot.slot_index());

                !span.is_queued_for_scan.swap(true, Ordering::AcqRel)
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
    pub(super) fn clear_mark_bits(&self) {
        let store = self.state.read();

        // small-span marks
        for span in &store.small.spans {
            span.clear_marks();
        }

        // large-allocation marks
        for allocation in &store.large.allocations {
            allocation.write().is_marked = false;
        }
    }

    /// Return whether one shared heap place is marked in the active cycle.
    pub(super) fn is_marked_place(&self, place: SharedHeapPlace) -> HeapResult<bool> {
        let store = self.state.read();

        // dispatch by physical shared heap place
        match place {
            SharedHeapPlace::Small(slot) => {
                let Some(span) = store.small.spans.get(slot.span_index()).cloned() else {
                    return Err(HeapError::MissingSpan {
                        span_index: slot.span_index(),
                    });
                };

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
    pub(super) fn mark_place(&self, place: SharedHeapPlace) -> HeapResult<bool> {
        // skip already marked places
        if self.is_marked_place(place)? {
            return Ok(false);
        }

        let store = self.state.read();

        // mark by physical shared heap place
        match place {
            SharedHeapPlace::Small(slot) => {
                let Some(span) = store.small.spans.get(slot.span_index()).cloned() else {
                    return Err(HeapError::MissingSpan {
                        span_index: slot.span_index(),
                    });
                };
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
