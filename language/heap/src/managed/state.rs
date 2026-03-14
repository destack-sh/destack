use serde::{Deserialize, Serialize};

/// Phase of the garbage collector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GcPhase {
    /// GC is idle.
    Idle,
    /// GC is marking reachable objects.
    Mark,
    /// GC is draining remaining work and finalizing the mark phase.
    MarkTermination,
    /// GC is sweeping unreachable objects.
    Sweep,
}

impl GcPhase {
    /// Report whether the GC is currently marking.
    pub fn is_marking(self) -> bool {
        matches!(self, Self::Mark | Self::MarkTermination)
    }
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
    /// Number of live bytes after the collection.
    pub live_bytes: u64,
    /// Total heap bytes after the collection.
    pub heap_bytes: u64,
}

/// GC state tracked across collection cycles.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GcState {
    /// Number of completed GC cycles.
    pub cycles: u64,
    /// Current GC phase.
    pub phase: GcPhase,
    /// Stats from the last completed cycle.
    pub last_stats: Option<GcStats>,
}

impl Default for GcState {
    fn default() -> Self {
        Self {
            cycles: 0,
            phase: GcPhase::Idle,
            last_stats: None,
        }
    }
}

impl GcState {
    /// Begin a new GC cycle.
    pub fn begin_cycle(&mut self) {
        self.cycles = self.cycles.saturating_add(1);
        self.phase = GcPhase::Mark;
    }

    /// Finish a GC cycle and record its stats.
    pub fn finish_cycle(&mut self, stats: GcStats) {
        self.phase = GcPhase::Idle;
        self.last_stats = Some(stats);
    }
}
