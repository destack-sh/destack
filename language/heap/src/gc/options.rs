/// GC options for pacing and limits.
#[derive(Debug, Clone, Copy)]
pub struct GcOptions {
    /// Heap growth target as a percent (Go-style GOGC).
    pub heap_growth_percent: u64,
    /// Optional hard heap cap in bytes.
    pub max_heap_bytes: Option<u64>,
    /// Minimum heap size before triggering GC.
    pub min_heap_bytes: u64,
    /// Trigger ratio (percent of goal) to start a GC cycle.
    pub trigger_ratio_percent: u64,
    /// Number of marking workers to schedule.
    pub mark_workers: usize,
}

impl Default for GcOptions {
    fn default() -> Self {
        Self {
            heap_growth_percent: 100,
            max_heap_bytes: None,
            min_heap_bytes: 8 * 1024 * 1024,
            trigger_ratio_percent: 75,
            mark_workers: 1,
        }
    }
}
