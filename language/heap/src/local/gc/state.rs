use std::collections::{HashMap, HashSet};

use crate::local::gc::PinSet;
use crate::local::storage::LargeBlockId;
use crate::{
    DropCursor, DropReference, GcDrop, HeapError, HeapReference, HeapResult, TraceQueue,
    TraceReference,
};

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
    /// The minor reclamation phase interrupted by marking.
    pub(crate) minor_resume_phase: Phase,
    /// The next young range start bit to visit after marking.
    pub(crate) young_reclaim_range_cursor: usize,
    /// The next young span index to visit after marking.
    pub(crate) young_reclaim_span_cursor: usize,
    /// The next slot inside the current young span to visit after marking.
    pub(crate) young_reclaim_slot_cursor: usize,
    /// The next dirty mature extent queued for young marking.
    pub(crate) young_dirty_extent_cursor: usize,
    /// The next dirty card inside the current mature extent.
    pub(crate) young_dirty_card_cursor: usize,
    /// Whether a write dirtied an already queued extent during the active minor cycle.
    pub(crate) dirty_rescan_needed: bool,
    /// The number of blocks freed by the active young cycle.
    pub(crate) young_freed_allocations: usize,
    /// The number of bytes freed by the active young cycle.
    pub(crate) young_freed_bytes: u64,

    /// The reusable minor collector trace queue.
    pub(crate) minor_queue: TraceQueue<HeapReference>,
    /// The reusable scratch buffer for scanned local references.
    pub(crate) local_reference_scratch: Vec<HeapReference>,
    /// The current major collection phase.
    pub(crate) major_phase: Phase,
    /// The persistent trace queue for an active major cycle.
    pub(crate) major_queue: TraceQueue<MarkWork>,
    /// The active local mark epoch.
    pub(crate) mark_epoch: u64,
    /// The active major post-mark cursor.
    pub(crate) major_reclaim: MajorReclaimCursor,
    /// The number of blocks freed by the active local major cycle.
    pub(crate) major_freed_allocations: usize,
    /// The number of bytes freed by the active local major cycle.
    pub(crate) major_freed_bytes: u64,
    /// Incremental Drop progress for one unreachable allocation.
    pending_drop: Option<DropCursor>,

    /// Live local references whose layouts may contain shared heap references.
    pub(crate) shared_edge_roots: Vec<HeapReference>,
    /// The dense root slot for each tracked shared-edge root.
    shared_edge_root_slots: HashMap<HeapReference, usize>,
    /// Whether one local-to-shared edge scan is currently active.
    pub(crate) is_scanning_shared_edges: bool,
    /// The next dense reference slot to scan for shared edges.
    pub(crate) shared_edge_cursor: usize,
    /// The pending local-to-shared edge work.
    pub(crate) shared_edge_queue: TraceQueue<EdgeWork>,
    /// Queue membership for pending shared-edge rescans.
    shared_edge_pending: HashSet<HeapReference>,
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
    /// Claim the first value from one local allocation.
    pub(crate) fn claim_drop(
        &mut self,
        mut cursor: DropCursor,
        budget_bytes: usize,
    ) -> HeapResult<GcDrop> {
        // reject overlapping allocation cursors
        if self.pending_drop.is_some() {
            return Err(HeapError::internal("local Drop is already claimed"));
        }

        // publish the cursor only after its first claim succeeds
        let drop = cursor.claim(budget_bytes)?;
        self.pending_drop = Some(cursor);

        Ok(drop)
    }

    /// Claim the next value from the pending local allocation.
    pub(crate) fn continue_drop(&mut self, budget_bytes: usize) -> HeapResult<Option<GcDrop>> {
        let Some(cursor) = &mut self.pending_drop else {
            return Ok(None);
        };

        // wait for the runtime to complete the active value
        if cursor.is_claimed() {
            return Ok(None);
        }

        cursor.claim(budget_bytes).map(Some)
    }

    /// Return whether one local value is claimed for Drop.
    pub(crate) fn is_drop_claimed(&self) -> bool {
        self.pending_drop.is_some_and(|cursor| cursor.is_claimed())
    }

    /// Complete the currently claimed local value.
    pub(crate) fn complete_drop(&mut self, reference: DropReference) -> HeapResult<()> {
        let Some(cursor) = &mut self.pending_drop else {
            return Err(HeapError::internal("local Drop cursor is missing"));
        };
        cursor.complete(reference)?;

        Ok(())
    }

    /// Retire the allocation cursor whose values completed Drop.
    pub(crate) fn retire_completed_drop(&mut self) {
        let is_complete = self
            .pending_drop
            .is_some_and(|cursor| cursor.is_complete() && !cursor.is_claimed());
        if is_complete {
            self.pending_drop = None;
        }
    }

    /// Return whether a local collection is currently running.
    pub(crate) fn is_collecting(&self) -> bool {
        self.minor_phase != Phase::Idle || self.major_phase != Phase::Idle
    }

    /// Clear every tracked shared-edge root.
    pub(crate) fn clear_shared_edge_roots(&mut self) {
        self.shared_edge_roots.clear();
        self.shared_edge_root_slots.clear();
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
        self.shared_edge_root_slots
            .insert(reference, self.shared_edge_roots.len());
        self.shared_edge_roots.push(reference);
    }

    /// Remove one live reference from the tracked shared-edge set.
    pub(crate) fn remove_shared_edge_root(&mut self, reference: HeapReference) {
        // absent roots are already removed
        let Some(tracked_index) = self.shared_edge_root_slots.remove(&reference) else {
            return;
        };

        self.shared_edge_pending.remove(&reference);

        // move the last root into the freed slot
        let last_index = self.shared_edge_roots.len() - 1;
        self.shared_edge_roots.swap_remove(tracked_index);
        if tracked_index < last_index {
            let moved = self.shared_edge_roots[tracked_index];

            self.shared_edge_root_slots.insert(moved, tracked_index);
        }

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
        if !self.shared_edge_pending.insert(reference) {
            return;
        }

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
        self.shared_edge_pending.remove(&reference);

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
}

/// The current local collection phase.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Phase {
    /// No collection is active.
    #[default]
    Idle,
    /// The collector is marking reachable blocks.
    Mark,
    /// The collector is running Drop for unreachable blocks.
    Drop,
    /// The collector is reclaiming unreachable blocks.
    Sweep,
}

/// Active local major post-mark heap cursor.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MajorReclaimCursor {
    /// The next young range start bit to visit.
    pub(crate) young_range_cursor: usize,
    /// The next young span index to visit.
    pub(crate) young_span_cursor: usize,
    /// The next young span slot index to visit.
    pub(crate) young_slot_cursor: usize,
    /// The next mature small span index to visit.
    pub(crate) small_span_cursor: usize,
    /// The next mature small span slot index to visit.
    pub(crate) small_slot_cursor: usize,
    /// The mature small span table length captured when reclamation started.
    pub(crate) small_span_limit: usize,
    /// The next mature large block index to visit.
    pub(crate) large_cursor: usize,
    /// The mature large block table length captured when reclamation started.
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

/// Charge bitmap metadata work and return the cursor reached.
pub(crate) fn charge_bitmap_skip(
    start: usize,
    end: usize,
    budget_bytes: usize,
    swept_bytes: &mut usize,
) -> usize {
    if start >= end {
        return end;
    }

    // charge one work unit per skipped bitmap word
    let skipped_bits = end - start;
    let skipped_words = skipped_bits.div_ceil(GC_METADATA_WORD_BITS);
    let remaining_budget = budget_bytes - *swept_bytes;
    if skipped_words <= remaining_budget {
        *swept_bytes += skipped_words;

        return end;
    }

    // stop at the bit reachable within the remaining budget
    *swept_bytes = budget_bytes;

    start + remaining_budget * GC_METADATA_WORD_BITS
}
