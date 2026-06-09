use crate::local::gc::PinSet;
use crate::local::storage::LargeBlockId;
use crate::{HeapReference, TraceQueue, TraceReference};

/// The budget charged for one metadata-only GC step.
pub(crate) const GC_METADATA_STEP_BYTES: usize = 1;

/// The number of metadata bits skipped by one bitmap word scan.
pub(crate) const GC_METADATA_WORD_BITS: usize = u64::BITS as usize;

/// Active local collector state.
#[derive(Debug, Default)]
pub(crate) struct CollectorState {
    /// The scoped heap pins that keep stable addresses and block branch boundaries.
    pub(crate) pins: PinSet,
    /// Mature extents queued for dirty-card scanning.
    pub(crate) dirty_extents: Vec<DirtyExtent>,

    /// The current minor collection phase.
    pub(crate) minor_phase: Phase,
    /// The next young range start bit to sweep.
    pub(crate) young_sweep_range_cursor: usize,
    /// The next young span index to sweep.
    pub(crate) young_sweep_span_cursor: usize,
    /// The next slot inside the current young span to sweep.
    pub(crate) young_sweep_slot_cursor: usize,
    /// The next dirty mature extent queued for young marking.
    pub(crate) young_dirty_extent_cursor: usize,
    /// The next dirty card inside the current mature extent.
    pub(crate) young_dirty_card_cursor: usize,
    /// The number of blocks freed by the active young cycle.
    pub(crate) young_freed_allocations: usize,
    /// The number of bytes freed by the active young cycle.
    pub(crate) young_freed_bytes: u64,

    /// The reusable minor collector trace queue.
    pub(crate) minor_queue: TraceQueue<HeapReference>,
    /// The current major collection phase.
    pub(crate) major_phase: Phase,
    /// The persistent trace queue for an active major cycle.
    pub(crate) major_queue: TraceQueue<MarkWork>,
    /// The active local mark epoch.
    pub(crate) mark_epoch: u64,
    /// The active major sweep cursor.
    pub(crate) major_sweep: MajorSweepCursor,
    /// The number of blocks freed by the active local major cycle.
    pub(crate) major_freed_allocations: usize,
    /// The number of bytes freed by the active local major cycle.
    pub(crate) major_freed_bytes: u64,

    /// Live local references whose layouts may contain shared heap references.
    pub(crate) shared_edge_roots: Vec<HeapReference>,
    /// Whether one local-to-shared edge scan is currently active.
    pub(crate) is_scanning_shared_edges: bool,
    /// The next dense reference slot to scan for shared edges.
    pub(crate) shared_edge_cursor: usize,
    /// The pending local-to-shared edge work.
    pub(crate) shared_edge_queue: TraceQueue<EdgeWork>,
    /// Queue membership for pending shared-edge rescans.
    pub(crate) shared_edge_pending: Vec<HeapReference>,
}

/// One mature extent queued for dirty-card scanning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DirtyExtent {
    /// One mature small span.
    Span(usize),
    /// One mature large block.
    Large(LargeBlockId),
}

impl CollectorState {
    /// Return whether a local collection is currently running.
    pub(crate) fn is_collecting(&self) -> bool {
        self.minor_phase != Phase::Idle || self.major_phase != Phase::Idle
    }

    /// Clear every tracked shared-edge root.
    pub(crate) fn clear_shared_edge_roots(&mut self) {
        self.shared_edge_roots.clear();
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
    }

    /// Record one live reference whose layout may contain shared edges.
    pub(crate) fn track_shared_edge_root(&mut self, reference: HeapReference) {
        self.shared_edge_roots.push(reference);
    }

    /// Remove one live reference from the tracked shared-edge set.
    pub(crate) fn remove_shared_edge_root(&mut self, reference: HeapReference) {
        // absent roots are already removed
        let Some(tracked_index) = self
            .shared_edge_roots
            .iter()
            .position(|root| *root == reference)
        else {
            return;
        };

        self.remove_shared_edge_pending(reference);

        let last_index = self.shared_edge_roots.len() - 1;
        self.shared_edge_roots.swap_remove(tracked_index);

        // keep active scans from skipping the root moved into a visited slot
        if self.is_scanning_shared_edges
            && tracked_index < self.shared_edge_cursor
            && tracked_index < last_index
        {
            self.shared_edge_cursor -= 1;
        }
    }

    /// Queue one live reference for one later shared-edge rescan.
    pub(crate) fn queue_shared_edge_root(&mut self, reference: HeapReference) {
        // duplicate rescan
        if self.shared_edge_pending.contains(&reference) {
            return;
        }

        self.shared_edge_pending.push(reference);
        self.shared_edge_queue.push(EdgeWork::Reference(reference));
    }

    /// Clear pending shared-edge rescan work.
    pub(crate) fn clear_shared_edge_work(&mut self) {
        self.shared_edge_queue.clear();
        self.shared_edge_pending.clear();
    }

    /// Pop one queued shared-edge work item.
    pub(crate) fn pop_shared_edge_work(&mut self) -> Option<EdgeWork> {
        // remove queue membership with the queue entry
        let work = self.shared_edge_queue.pop()?;
        let EdgeWork::Reference(reference) = work else {
            return Some(work);
        };
        self.remove_shared_edge_pending(reference);

        Some(work)
    }

    /// Return the next tracked local reference that may contain shared edges.
    pub(crate) fn next_shared_edge_root(&mut self) -> Option<HeapReference> {
        if self.shared_edge_cursor >= self.shared_edge_roots.len() {
            return None;
        }

        let index = self.shared_edge_cursor;
        self.shared_edge_cursor += 1;

        self.shared_edge_roots.get(index).copied()
    }

    /// Remove one reference from the pending shared-edge set.
    fn remove_shared_edge_pending(&mut self, reference: HeapReference) {
        let Some(index) = self
            .shared_edge_pending
            .iter()
            .position(|pending| *pending == reference)
        else {
            return;
        };

        self.shared_edge_pending.swap_remove(index);
    }
}

/// The current local collection phase.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Phase {
    /// No collection is active.
    #[default]
    Idle,
    /// The collector is marking reachable blocks.
    Mark,
    /// The collector is reclaiming unreachable blocks.
    Sweep,
}

/// Active local major sweep cursor.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MajorSweepCursor {
    /// The next young range start bit to sweep.
    pub(crate) young_range_cursor: usize,
    /// The next young span index to sweep.
    pub(crate) young_span_cursor: usize,
    /// The next young span slot index to sweep.
    pub(crate) young_slot_cursor: usize,
    /// The next mature small span index to sweep.
    pub(crate) small_span_cursor: usize,
    /// The next mature small span slot index to sweep.
    pub(crate) small_slot_cursor: usize,
    /// The mature small span table length captured when sweep started.
    pub(crate) small_span_limit: usize,
    /// The next mature large block index to sweep.
    pub(crate) large_cursor: usize,
    /// The mature large block table length captured when sweep started.
    pub(crate) large_limit: usize,
}

/// One queued unit of local major mark work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MarkWork {
    /// One heap block to scan.
    Reference(HeapReference),
    /// One range of one large heap block to scan.
    LargeRange {
        /// The heap block reference.
        reference: HeapReference,
        /// The range start in bytes.
        start: usize,
    },
}

impl TraceReference for MarkWork {
    /// Report whether this trace work points at null.
    fn is_null(self) -> bool {
        match self {
            Self::Reference(reference) | Self::LargeRange { reference, .. } => reference.is_null(),
        }
    }
}

/// One queued unit of local-to-shared edge scan work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EdgeWork {
    /// One local heap block to scan.
    Reference(HeapReference),
    /// One range of one large local heap block to scan.
    LargeRange {
        /// The local heap block reference.
        reference: HeapReference,
        /// The range start in bytes.
        start: usize,
    },
}

impl TraceReference for EdgeWork {
    /// Report whether this edge work points at null.
    fn is_null(self) -> bool {
        match self {
            Self::Reference(reference) | Self::LargeRange { reference, .. } => reference.is_null(),
        }
    }
}
