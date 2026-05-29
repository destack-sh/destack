use destack_mir::TraceTable;

use crate::shared::gc::{SharedGcPhase, SharedGcWorker, SharedTraceWork};
use crate::shared::space::{SharedHeapSpace, SharedHeapStorage};
use crate::{
    HeapError, HeapResult, ReferenceInput, ReferenceRange, SharedHeapReference, scan_references,
};

impl SharedHeapSpace {
    /// Record one shared heap write barrier before one byte store.
    pub(crate) fn write_shared_barrier_bytes(
        &self,
        reference: SharedHeapReference,
        byte_offset: usize,
        bytes: &[u8],
        trace_table: &TraceTable,
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
        let Some(extent) = self.resolve_extent(reference) else {
            return Err(HeapError::invalid_shared_heap_reference(reference));
        };
        let trace_map = self.trace_map_for_place(extent.storage, trace_table)?;
        let base_address = self.mapping.base_address() + extent.base.offset();
        scan_references::<SharedHeapReference>(
            &trace_map,
            ReferenceInput::mapped(base_address),
            ReferenceRange::bytes(byte_offset, bytes.len()),
            &mut reference_buffer,
        )?;

        // inserted bytes
        scan_references::<SharedHeapReference>(
            &trace_map,
            ReferenceInput::bytes(byte_offset, bytes),
            ReferenceRange::bytes(byte_offset, bytes.len()),
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
        // idle block needs no publication work
        let phase = self.gc.phase();
        if phase == SharedGcPhase::Idle {
            return Ok(());
        }

        // concurrent publication
        let Some(_publication) = self.gc.begin_mark_publication() else {
            if phase == SharedGcPhase::Sweep {
                let Some(extent) = self.resolve_extent(reference) else {
                    return Err(HeapError::invalid_shared_heap_reference(reference));
                };
                self.mark_place(extent.storage)?;
            }

            return Ok(());
        };

        let Some(extent) = self.resolve_extent(reference) else {
            return Err(HeapError::invalid_shared_heap_reference(reference));
        };

        // new blocks without initial shared edges can stay black
        if !has_initial_edges {
            self.mark_place(extent.storage)?;

            return Ok(());
        }

        // queue the new block so the active cycle traces its initial payload
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
        // resolve the reference to its physical storage
        let Some(extent) = self.resolve_extent(reference) else {
            return Err(HeapError::invalid_shared_heap_reference(reference));
        };

        // small references mark their span slot for later scanning
        if let SharedHeapStorage::SmallSlot(slot) = extent.storage {
            let should_queue = {
                let store = self.state.read();
                let Some(span) = store.small.spans.get(slot.span_index()).cloned() else {
                    return Err(HeapError::internal("missing span"));
                };

                let mark_epoch = self.gc.mark_epoch();
                if span.is_marked(slot.slot_index(), mark_epoch) {
                    return Ok(());
                }

                span.mark_slot(slot.slot_index(), mark_epoch)
            };

            if should_queue {
                self.gc
                    .trace_queue
                    .push(worker, SharedTraceWork::SmallSpan(slot.span_index()));
            }

            return Ok(());
        }

        // large references scan in page-sized chunks
        if !self.mark_place(extent.storage)? {
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

    /// Mark one shared heap storage and return whether this was the first mark.
    pub(super) fn mark_place(&self, storage: SharedHeapStorage) -> HeapResult<bool> {
        let store = self.state.read();
        let mark_epoch = self.gc.mark_epoch();

        // mark by physical shared heap storage
        match storage {
            SharedHeapStorage::SmallSlot(slot) => {
                let Some(span) = store.small.spans.get(slot.span_index()).cloned() else {
                    return Err(HeapError::internal("missing span"));
                };

                return Ok(span.mark_slot(slot.slot_index(), mark_epoch));
            }
            SharedHeapStorage::LargeBlock(block_id) => {
                let Some(block) = store.large.blocks.get(block_id.index()?).cloned() else {
                    return Err(HeapError::internal("missing large block"));
                };
                let mut block = block.write();
                if block.mark_epoch == mark_epoch {
                    return Ok(false);
                }

                block.mark_epoch = mark_epoch;
            }
        }

        Ok(true)
    }
}
