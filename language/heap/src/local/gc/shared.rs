use crate::local::space::HeapSpace;
use crate::{
    HeapError, HeapReference, HeapResult, ScanSource, SharedHeapReference,
    scan_shared_references as scan_shared_heap_references,
};

impl HeapSpace {
    /// Start one incremental local-to-shared edge scan.
    pub(crate) fn start_shared_edge_scan(&mut self) {
        // reset scan cursors
        self.is_scanning_shared_edges = true;
        self.shared_edge_cursor = 0;
        self.shared_edge_queue.clear();
        self.shared_edge_pending.clear();
    }

    /// Return whether the current local-to-shared edge scan is fully drained.
    pub(crate) fn shared_edge_scan_idle(&self) -> bool {
        !self.is_scanning_shared_edges
            || (self.shared_edge_cursor >= self.shared_edge_roots.len()
                && self.shared_edge_queue.is_empty())
    }

    /// Finish the current local-to-shared edge scan.
    pub(crate) fn finish_shared_edge_scan(&mut self) {
        // close the active scan
        self.is_scanning_shared_edges = false;
        self.shared_edge_cursor = 0;
        self.shared_edge_queue.clear();
        self.shared_edge_pending.clear();

        // remove tombstones left by concurrent root removal
        self.compact_shared_edge_roots();
    }

    /// Scan bounded local-to-shared edge work into the provided root buffer.
    pub(crate) fn scan_shared_references(
        &mut self,
        roots: &mut Vec<SharedHeapReference>,
        budget_bytes: usize,
    ) -> HeapResult<usize> {
        // inactive scan
        if !self.is_scanning_shared_edges || budget_bytes == 0 {
            return Ok(0);
        }

        let mut scanned_bytes = 0usize;

        // drain queued rescans first
        while scanned_bytes < budget_bytes {
            let Some(reference) = self.shared_edge_queue.pop() else {
                break;
            };

            self.shared_edge_pending.remove(&reference);
            scanned_bytes += self.trace_shared_edges(reference, roots)?;
        }

        // then continue the tracked shared-edge walk
        while scanned_bytes < budget_bytes {
            let Some(reference) = self.next_shared_edge_root() else {
                break;
            };

            scanned_bytes += self.trace_shared_edges(reference, roots)?;
        }

        Ok(scanned_bytes)
    }

    /// Queue one local reference for one later shared-edge rescan.
    pub(crate) fn queue_shared_reference(&mut self, reference: HeapReference) -> HeapResult<()> {
        // only live shared-reference carriers need rescanning
        if !self.is_scanning_shared_edges || !self.reference_has_shared_roots(reference)? {
            return Ok(());
        }

        // avoid duplicate queued rescans
        if !self.shared_edge_pending.insert(reference) {
            return Ok(());
        }

        self.shared_edge_queue.push(reference);

        Ok(())
    }

    /// Return whether one live local reference may contain shared heap roots.
    pub(crate) fn reference_has_shared_roots(&self, reference: HeapReference) -> HeapResult<bool> {
        // freed references cannot publish shared roots
        let Some(location) = self.resolve_location(reference) else {
            return Ok(false);
        };

        // layout metadata decides whether scanning is needed
        let reference_map = self.reference_map_for_place(location.place)?;

        Ok(reference_map.has_shared_reference())
    }

    /// Return the next tracked local reference that may contain shared edges.
    fn next_shared_edge_root(&mut self) -> Option<HeapReference> {
        // skip tombstones left by removals during the active scan
        while self.shared_edge_cursor < self.shared_edge_roots.len() {
            let index = self.shared_edge_cursor;
            self.shared_edge_cursor += 1;

            let reference = self.shared_edge_roots.get(index).copied()?;
            if !reference.is_null() {
                return Some(reference);
            }
        }

        None
    }

    /// Trace shared heap roots from one heap reference.
    fn trace_shared_edges(
        &mut self,
        reference: HeapReference,
        roots: &mut Vec<SharedHeapReference>,
    ) -> HeapResult<usize> {
        // freed references contribute no work
        let Some(location) = self.resolve_location(reference) else {
            return Ok(0);
        };

        // load exact shared-reference layout
        let reference_map = self
            .reference_map_for_place(location.place)
            .map_err(|error| HeapError::HeapScanFailed {
                source: ScanSource::Reference(reference),
                error: Box::new(error),
            })?;

        // noscan payloads still consume their byte budget
        if !reference_map.has_shared_reference() {
            return Ok(location.byte_len);
        }

        // scan mapped heap memory directly
        let base_address = self.mapping.base_address() + location.base.offset();
        let mut reference_buffer = Vec::new();
        let result =
            scan_shared_heap_references(&reference_map, base_address, &mut reference_buffer);

        if let Err(error) = result {
            return Err(HeapError::HeapScanFailed {
                source: ScanSource::Reference(reference),
                error: Box::new(error),
            });
        }

        // publish non-null shared roots
        reference_buffer.retain(|reference| !reference.is_null());
        roots.extend(reference_buffer);

        Ok(location.byte_len)
    }
}
