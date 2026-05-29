use destack_mir::TraceTable;

use crate::local::gc::SharedEdgeWork;
use crate::local::space::{HeapPlace, HeapSpace};
use crate::{
    HeapError, HeapOperationSource, HeapReference, HeapResult, ReferenceInput, ReferenceRange,
    SharedHeapReference, scan_references,
};

impl HeapSpace {
    /// Start one incremental local-to-shared edge scan.
    pub(crate) fn start_shared_edge_scan(&mut self) {
        self.collector.start_shared_edge_scan();
    }

    /// Return whether the current local-to-shared edge scan is fully drained.
    pub(crate) fn shared_edge_scan_idle(&self) -> bool {
        self.collector.shared_edge_scan_idle()
    }

    /// Finish the current local-to-shared edge scan.
    pub(crate) fn finish_shared_edge_scan(&mut self) {
        self.collector.finish_shared_edge_scan();
    }

    /// Scan bounded local-to-shared edge work into the provided root buffer.
    pub(crate) fn scan_shared_references(
        &mut self,
        roots: &mut Vec<SharedHeapReference>,
        budget_bytes: usize,
        trace_table: &TraceTable,
    ) -> HeapResult<usize> {
        // inactive scan
        if !self.collector.is_scanning_shared_edges || budget_bytes == 0 {
            return Ok(0);
        }

        let mut scanned_bytes = 0usize;

        // drain queued rescans first
        while scanned_bytes < budget_bytes {
            let Some(work) = self.collector.pop_shared_edge_work() else {
                break;
            };

            scanned_bytes += self.trace_shared_edge_work(work, roots, trace_table)?;
        }

        // then continue the tracked shared-edge walk
        while scanned_bytes < budget_bytes {
            let Some(reference) = self.collector.next_shared_edge_root() else {
                break;
            };

            scanned_bytes += self.trace_shared_edge_work(
                SharedEdgeWork::Reference(reference),
                roots,
                trace_table,
            )?;
        }

        Ok(scanned_bytes)
    }

    /// Queue one local reference for one later shared-edge rescan.
    pub(crate) fn queue_shared_reference(
        &mut self,
        reference: HeapReference,
        trace_table: &TraceTable,
    ) -> HeapResult<()> {
        // only live shared-reference carriers need rescanning
        if !self.collector.is_scanning_shared_edges
            || !self.reference_has_shared_roots(reference, trace_table)?
        {
            return Ok(());
        }

        self.collector.queue_shared_edge_root(reference);

        Ok(())
    }

    /// Queue one shared-edge root when a scan is active.
    pub(crate) fn queue_shared_edge_root(&mut self, reference: HeapReference) {
        if self.collector.is_scanning_shared_edges {
            self.collector.queue_shared_edge_root(reference);
        }
    }

    /// Return whether one live local reference may contain shared heap roots.
    pub(crate) fn reference_has_shared_roots(
        &self,
        reference: HeapReference,
        trace_table: &TraceTable,
    ) -> HeapResult<bool> {
        // freed references cannot publish shared roots
        let Some(region) = self.resolve_region(reference) else {
            return Ok(false);
        };

        // layout metadata decides whether scanning is needed
        let trace_map = self.trace_map_for_place(region.place, trace_table)?;

        Ok(trace_map.has_shared_reference())
    }

    /// Trace shared heap roots from one queued edge work item.
    fn trace_shared_edge_work(
        &mut self,
        work: SharedEdgeWork,
        roots: &mut Vec<SharedHeapReference>,
        trace_table: &TraceTable,
    ) -> HeapResult<usize> {
        match work {
            SharedEdgeWork::Reference(reference) => {
                self.trace_shared_edges(reference, roots, trace_table)
            }
            SharedEdgeWork::LargeRange { reference, start } => {
                self.trace_large_shared_edges(reference, start, roots, trace_table)
            }
        }
    }

    /// Trace shared heap roots from one heap reference.
    fn trace_shared_edges(
        &mut self,
        reference: HeapReference,
        roots: &mut Vec<SharedHeapReference>,
        trace_table: &TraceTable,
    ) -> HeapResult<usize> {
        // freed references contribute no work
        let Some(region) = self.resolve_region(reference) else {
            return Ok(0);
        };

        // load exact shared-reference layout
        let trace_map = self
            .trace_map_for_place(region.place, trace_table)
            .map_err(|error| {
                HeapError::scan_failed(HeapOperationSource::Reference(reference), error)
            })?;

        // noscan payloads still consume their byte budget
        if !trace_map.has_shared_reference() {
            return Ok(region.byte_len);
        }

        // large references are sliced to keep shared-root scans bounded
        if matches!(region.place, HeapPlace::Large(_)) {
            return self.trace_large_shared_edges(reference, 0, roots, trace_table);
        }

        // scan mapped heap memory directly
        let base_address = self.mapping.base_address() + region.base.offset();
        let mut reference_buffer = Vec::new();
        let result = scan_references::<SharedHeapReference>(
            &trace_map,
            ReferenceInput::mapped(base_address),
            ReferenceRange::All,
            &mut reference_buffer,
        );

        if let Err(error) = result {
            return Err(HeapError::scan_failed(
                HeapOperationSource::Reference(reference),
                error,
            ));
        }

        // publish non-null shared roots
        reference_buffer.retain(|reference| !reference.is_null());
        roots.extend(reference_buffer);

        Ok(region.byte_len)
    }

    /// Trace one page-sized range of shared roots from one large heap reference.
    fn trace_large_shared_edges(
        &mut self,
        reference: HeapReference,
        start: usize,
        roots: &mut Vec<SharedHeapReference>,
        trace_table: &TraceTable,
    ) -> HeapResult<usize> {
        // freed references contribute no work
        let Some(region) = self.resolve_region(reference) else {
            return Ok(0);
        };
        let HeapPlace::Large(_) = region.place else {
            return Err(HeapError::Internal {
                context: "large shared-edge work resolved to non-large allocation",
            });
        };

        // load exact shared-reference layout
        let trace_map = self
            .trace_map_for_place(region.place, trace_table)
            .map_err(|error| {
                HeapError::scan_failed(HeapOperationSource::Reference(reference), error)
            })?;

        // empty or noscan ranges need no continuation
        if start >= region.byte_len || !trace_map.has_shared_reference() {
            return Ok(0);
        }

        // scan at most one allocator page
        let range_len = self
            .allocator()
            .page_size_bytes()
            .min(region.byte_len - start);
        let base_address = self.mapping.base_address() + region.base.offset();
        let mut reference_buffer = Vec::new();
        let result = scan_references::<SharedHeapReference>(
            &trace_map,
            ReferenceInput::mapped(base_address),
            ReferenceRange::bytes(start, range_len),
            &mut reference_buffer,
        );

        if let Err(error) = result {
            return Err(HeapError::scan_failed(
                HeapOperationSource::Reference(reference),
                error,
            ));
        }

        // publish non-null shared roots
        reference_buffer.retain(|reference| !reference.is_null());
        roots.extend(reference_buffer);

        // continue this large allocation on a later step
        let next_start = start + range_len;
        if next_start < region.byte_len {
            self.collector
                .shared_edge_queue
                .push(SharedEdgeWork::LargeRange {
                    reference,
                    start: next_start,
                });
        }

        Ok(range_len)
    }
}
