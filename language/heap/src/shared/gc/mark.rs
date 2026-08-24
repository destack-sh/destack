use crate::TraceView;

use crate::shared::gc::{MarkWork, SharedMarkWorker};
use crate::shared::storage::{HeapPlace, HeapStorage};
use crate::{GcPhase, HeapError, HeapResult, ReferenceInput, ReferenceRange, SharedHeapReference};

impl HeapStorage {
    /// Record one shared heap write barrier before one byte store.
    pub(crate) fn write_shared_barrier_bytes(
        &self,
        reference: SharedHeapReference,
        byte_offset: usize,
        bytes: &[u8],
        trace_view: TraceView<'_>,
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
        let Some(extent) = self.resolve_extent(reference) else {
            return Err(HeapError::invalid_shared_heap_reference(reference));
        };

        // scan references overwritten by this store
        let base_address = self.memory.base_address() + extent.base.offset();
        self.visit_references::<SharedHeapReference>(
            extent.place,
            trace_view,
            ReferenceInput::mapped(base_address),
            ReferenceRange::bytes(byte_offset, bytes.len()),
            &mut |reference| {
                if !reference.is_nullish() {
                    self.mark_reference(None, reference)?;
                }

                Ok(())
            },
        )?;

        // scan references inserted by this store
        self.visit_references::<SharedHeapReference>(
            extent.place,
            trace_view,
            ReferenceInput::bytes(byte_offset, bytes),
            ReferenceRange::bytes(byte_offset, bytes.len()),
            &mut |reference| {
                if !reference.is_nullish() {
                    self.mark_reference(None, reference)?;
                }

                Ok(())
            },
        )?;

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
        if phase == GcPhase::Idle {
            return Ok(());
        }

        // trace initialized edges while mark publication remains open
        let Some(_publication) = self.gc.begin_mark_publication() else {
            let Some(extent) = self.resolve_extent(reference) else {
                return Err(HeapError::invalid_shared_heap_reference(reference));
            };
            self.mark_place(extent.place)?;

            return Ok(());
        };

        let Some(extent) = self.resolve_extent(reference) else {
            return Err(HeapError::invalid_shared_heap_reference(reference));
        };

        // new blocks without initial shared edges can stay black
        if !has_initial_edges {
            self.mark_place(extent.place)?;

            return Ok(());
        }

        // queue the new block so the active cycle traces its initial payload
        self.mark_reference(None, reference)
    }

    /// Queue explicit roots that are not already marked in this cycle.
    pub(super) fn mark_roots(
        &self,
        worker: Option<&SharedMarkWorker>,
        references: &[SharedHeapReference],
    ) -> HeapResult<()> {
        // seed explicit roots
        for reference in references {
            if reference.is_nullish() {
                continue;
            }

            self.mark_reference(worker, *reference)?;
        }

        Ok(())
    }

    /// Mark one shared reference and queue trace work when needed.
    pub(crate) fn mark_reference(
        &self,
        worker: Option<&SharedMarkWorker>,
        reference: SharedHeapReference,
    ) -> HeapResult<()> {
        // nullish and constant references are never shared mark work
        if reference.is_nullish() || self.is_constant(reference) {
            return Ok(());
        }

        // resolve the reference to physical storage
        let Some(extent) = self.resolve_extent(reference) else {
            return Err(HeapError::invalid_shared_heap_reference(reference));
        };

        // skip references already marked in this cycle
        if !self.mark_place(extent.place)? {
            return Ok(());
        }

        // queue trace work for the newly marked storage
        match extent.place {
            HeapPlace::SmallSlot(slot) => {
                self.gc
                    .trace_queue
                    .push(worker, MarkWork::SmallSpan(slot.span_index()));
            }
            HeapPlace::LargeBlock(_) => {
                self.gc.trace_queue.push(
                    worker,
                    MarkWork::Large {
                        reference,
                        start: 0,
                    },
                );
            }
        }

        Ok(())
    }

    /// Mark one shared heap place and return whether this was the first mark.
    pub(super) fn mark_place(&self, place: HeapPlace) -> HeapResult<bool> {
        // load shared heap state for physical mark bits
        let store = self.state.read();
        let mark_epoch = self.gc.mark_epoch();

        // mark by physical shared heap place
        match place {
            HeapPlace::SmallSlot(slot) => {
                let Some(span) = store.small.spans.get(slot.span_index()).cloned() else {
                    return Err(HeapError::internal("missing span"));
                };

                return Ok(span.mark_slot(slot.slot_index(), mark_epoch));
            }
            HeapPlace::LargeBlock(block_id) => {
                let Some(block) = store
                    .large
                    .blocks
                    .get(block_id.index()?)
                    .and_then(Option::as_ref)
                    .cloned()
                else {
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
