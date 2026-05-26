use std::collections::{BTreeMap, BTreeSet};

use crate::local::gc::PinSet;
use crate::local::space::LargeAllocationId;
use crate::{HeapReference, TraceQueue, TraceReference};

/// Active local collector state.
#[derive(Debug, Default)]
pub(crate) struct LocalGcState {
    /// The reusable minor collector trace queue.
    pub(crate) minor_queue: HeapTraceQueue,
    /// The current local major collection phase.
    pub(crate) major_phase: LocalGcPhase,
    /// The persistent trace queue for an active local major cycle.
    pub(crate) major_queue: LocalTraceQueue,
    /// The stable sweep reference snapshot for an active local major cycle.
    pub(crate) major_sweep_references: Vec<HeapReference>,
    /// The next sweep snapshot index to visit.
    pub(crate) major_sweep_cursor: usize,
    /// The number of allocations freed by the active local major cycle.
    pub(crate) major_freed_allocations: usize,
    /// The number of bytes freed by the active local major cycle.
    pub(crate) major_freed_bytes: u64,
    /// Whether a heap collection is currently running.
    pub(crate) is_collecting: bool,
    /// The scoped heap pins that keep stable addresses and block branch boundaries.
    pub(crate) pins: PinSet,
    /// Mature spans queued for dirty-card scanning.
    pub(crate) dirty_spans: Vec<usize>,
    /// Mature large allocations queued for dirty-card scanning.
    pub(crate) dirty_large_allocations: Vec<LargeAllocationId>,
    /// Live local references whose layouts may contain shared heap references.
    pub(crate) shared_edge_roots: Vec<HeapReference>,
    /// Reverse index into tracked shared-edge roots.
    pub(crate) shared_edge_index: BTreeMap<HeapReference, usize>,
    /// Whether one local-to-shared edge scan is currently active.
    pub(crate) is_scanning_shared_edges: bool,
    /// The next dense reference slot to scan for shared edges.
    pub(crate) shared_edge_cursor: usize,
    /// The pending local-to-shared edge work.
    pub(crate) shared_edge_queue: SharedEdgeQueue,
    /// Queue membership for pending shared-edge rescans.
    pub(crate) shared_edge_pending: BTreeSet<HeapReference>,
}

impl LocalGcState {
    /// Clear every tracked shared-edge root.
    pub(crate) fn clear_shared_edge_roots(&mut self) {
        self.shared_edge_roots.clear();
        self.shared_edge_index.clear();
    }

    /// Start one tracked shared-edge scan.
    pub(crate) fn start_shared_edge_scan(&mut self) {
        self.is_scanning_shared_edges = true;
        self.shared_edge_cursor = 0;
        self.clear_shared_edge_work();
    }

    /// Return whether the active shared-edge scan is drained.
    pub(crate) fn shared_edge_scan_idle(&self) -> bool {
        !self.is_scanning_shared_edges
            || (self.shared_edge_cursor >= self.shared_edge_roots.len()
                && self.shared_edge_queue.is_empty())
    }

    /// Finish one tracked shared-edge scan.
    pub(crate) fn finish_shared_edge_scan(&mut self) {
        self.is_scanning_shared_edges = false;
        self.shared_edge_cursor = 0;
        self.clear_shared_edge_work();
        self.compact_shared_edge_roots();
    }

    /// Record one live reference whose layout may contain shared edges.
    pub(crate) fn track_shared_edge_root(&mut self, reference: HeapReference) {
        // duplicate root
        if self.shared_edge_index.contains_key(&reference) {
            return;
        }

        // dense slot
        let tracked_index = self.shared_edge_roots.len();
        self.shared_edge_roots.push(reference);
        self.shared_edge_index.insert(reference, tracked_index);
    }

    /// Remove one live reference from the tracked shared-edge set.
    pub(crate) fn remove_shared_edge_root(&mut self, reference: HeapReference) {
        // absent roots are already removed
        let Some(tracked_index) = self.shared_edge_index.remove(&reference) else {
            return;
        };

        self.shared_edge_pending.remove(&reference);

        // active scans need stable cursor ordering
        if self.is_scanning_shared_edges {
            self.shared_edge_roots[tracked_index] = HeapReference::NULL;

            return;
        }

        let Some(moved_reference) = self.shared_edge_roots.pop() else {
            return;
        };

        // removing the last root needs no swap
        if tracked_index == self.shared_edge_roots.len() {
            return;
        }

        // backfill the removed slot
        self.shared_edge_roots[tracked_index] = moved_reference;
        if !moved_reference.is_null() {
            self.shared_edge_index
                .insert(moved_reference, tracked_index);
        }
    }

    /// Compact removed roots after one active shared-edge scan.
    pub(crate) fn compact_shared_edge_roots(&mut self) {
        // remove active-scan tombstones
        self.shared_edge_roots
            .retain(|reference| !reference.is_null());
        self.shared_edge_index.clear();

        // rebuild dense reverse index
        for (index, reference) in self.shared_edge_roots.iter().copied().enumerate() {
            self.shared_edge_index.insert(reference, index);
        }
    }

    /// Queue one live reference for one later shared-edge rescan.
    pub(crate) fn queue_shared_edge_root(&mut self, reference: HeapReference) {
        // duplicate rescan
        if !self.shared_edge_pending.insert(reference) {
            return;
        }

        self.shared_edge_queue
            .push(SharedEdgeWork::Reference(reference));
    }

    /// Clear pending shared-edge rescan work.
    pub(crate) fn clear_shared_edge_work(&mut self) {
        self.shared_edge_queue.clear();
        self.shared_edge_pending.clear();
    }

    /// Pop one queued shared-edge work item.
    pub(crate) fn pop_shared_edge_work(&mut self) -> Option<SharedEdgeWork> {
        // remove queue membership with the queue entry
        let work = self.shared_edge_queue.pop()?;
        let SharedEdgeWork::Reference(reference) = work else {
            return Some(work);
        };
        self.shared_edge_pending.remove(&reference);

        Some(work)
    }

    /// Return the next tracked local reference that may contain shared edges.
    pub(crate) fn next_shared_edge_root(&mut self) -> Option<HeapReference> {
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
}

/// Collector queue for heap references.
pub(crate) type HeapTraceQueue = TraceQueue<HeapReference>;

/// Collector queue for local major mark work.
pub(crate) type LocalTraceQueue = TraceQueue<LocalTraceWork>;

/// Collector queue for local-to-shared edge work.
pub(crate) type SharedEdgeQueue = TraceQueue<SharedEdgeWork>;

/// The current local major collection phase.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LocalGcPhase {
    /// No major collection is active.
    #[default]
    Idle,
    /// The major collector is marking reachable allocations.
    Mark,
    /// The major collector is reclaiming unreachable allocations.
    Sweep,
}

/// One queued unit of local major mark work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LocalTraceWork {
    /// One heap allocation to scan.
    Reference(HeapReference),
    /// One range of one large heap allocation to scan.
    LargeRange {
        /// The heap allocation reference.
        reference: HeapReference,
        /// The range start in bytes.
        start: usize,
    },
}

impl TraceReference for LocalTraceWork {
    /// Report whether this trace work points at null.
    fn is_null(self) -> bool {
        match self {
            Self::Reference(reference) | Self::LargeRange { reference, .. } => reference.is_null(),
        }
    }
}

/// One queued unit of local-to-shared edge scan work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SharedEdgeWork {
    /// One local heap allocation to scan.
    Reference(HeapReference),
    /// One range of one large local heap allocation to scan.
    LargeRange {
        /// The local heap allocation reference.
        reference: HeapReference,
        /// The range start in bytes.
        start: usize,
    },
}

impl TraceReference for SharedEdgeWork {
    /// Report whether this edge work points at null.
    fn is_null(self) -> bool {
        match self {
            Self::Reference(reference) | Self::LargeRange { reference, .. } => reference.is_null(),
        }
    }
}
