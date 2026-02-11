use super::{GcPhase, GcStats};

/// GC state tracked across collection cycles.
#[derive(Debug, Clone)]
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
