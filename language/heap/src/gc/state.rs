use serde::{Deserialize, Serialize};

use crate::{HeapError, HeapResult};

/// Scope of one garbage collection cycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GcKind {
    /// One full heap collection.
    Full,
    /// One young-generation collection.
    Minor,
}

/// Summary statistics for a garbage collection cycle.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GcStats {
    /// Number of allocations freed by the collection.
    pub freed_allocations: usize,
    /// Number of live allocations after the collection.
    pub live_allocations: usize,
    /// Number of bytes freed by the collection.
    pub freed_bytes: u64,
    /// Number of live allocated bytes after the collection.
    pub allocated_bytes: u64,
    /// Total active allocator bytes after the collection.
    pub active_bytes: u64,
}

/// One completed garbage collection cycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GcCycle {
    /// The scope of the completed cycle.
    pub kind: GcKind,
    /// The summary statistics for the completed cycle.
    pub stats: GcStats,
}

/// GC state tracked across collection cycles.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct GcState {
    /// Number of completed GC cycles.
    pub completed_cycles: u64,
    /// The last completed GC cycle.
    pub last_cycle: Option<GcCycle>,
}

impl GcState {
    /// Record one completed GC cycle.
    pub fn record_cycle(&mut self, kind: GcKind, stats: GcStats) -> HeapResult<()> {
        self.completed_cycles =
            self.completed_cycles
                .checked_add(1)
                .ok_or(HeapError::GcCycleCountOverflow {
                    completed_cycles: self.completed_cycles,
                })?;
        self.last_cycle = Some(GcCycle { kind, stats });

        Ok(())
    }
}
