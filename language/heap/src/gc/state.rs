use super::{GcOptions, GcPacer, GcPhase, GcStats};

/// GC state tracked across collection cycles.
#[derive(Debug, Clone)]
pub struct GcState {
    /// Number of completed GC cycles.
    pub cycles: u64,
    /// Current GC phase.
    pub phase: GcPhase,
    /// Pacing targets based on live heap size.
    pub pacer: GcPacer,
    /// GC options for pacing and limits.
    pub options: GcOptions,
    /// Stats from the last completed cycle.
    pub last_stats: Option<GcStats>,
}

impl Default for GcState {
    fn default() -> Self {
        Self {
            cycles: 0,
            phase: GcPhase::Idle,
            pacer: GcPacer::default(),
            options: GcOptions::default(),
            last_stats: None,
        }
    }
}

impl GcState {
    /// Begin a new GC cycle and update pacing targets.
    pub fn begin_cycle(&mut self, live_bytes: u64) {
        self.cycles = self.cycles.saturating_add(1);
        self.phase = GcPhase::Mark;

        self.pacer.update(self.options, live_bytes);
    }

    /// Finish a GC cycle and record its stats.
    pub fn finish_cycle(&mut self, stats: GcStats) {
        self.phase = GcPhase::Idle;
        self.last_stats = Some(stats);
    }
}
