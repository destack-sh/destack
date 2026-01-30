use super::GcOptions;

/// GC pacing targets derived from live heap size.
#[derive(Debug, Clone, Copy)]
pub struct GcPacer {
    /// Estimated live heap size in bytes.
    pub live_bytes: u64,
    /// Target heap size in bytes for the next cycle.
    pub heap_goal_bytes: u64,
    /// Trigger point for starting a cycle.
    pub trigger_bytes: u64,
}

impl Default for GcPacer {
    fn default() -> Self {
        Self {
            live_bytes: 0,
            heap_goal_bytes: 0,
            trigger_bytes: 0,
        }
    }
}

impl GcPacer {
    /// Update pacing targets from options and live heap size.
    pub fn update(&mut self, options: GcOptions, live_bytes: u64) {
        let growth = live_bytes.saturating_mul(options.heap_growth_percent);
        let growth = growth / 100;
        let mut goal = live_bytes
            .saturating_add(growth)
            .max(options.min_heap_bytes);

        if let Some(max) = options.max_heap_bytes {
            goal = goal.min(max);
        }

        let trigger = goal.saturating_mul(options.trigger_ratio_percent) / 100;

        self.live_bytes = live_bytes;
        self.heap_goal_bytes = goal;
        self.trigger_bytes = trigger.max(options.min_heap_bytes);
    }

    /// Check whether the heap size should trigger a new GC cycle.
    pub fn should_start(&self, heap_bytes: u64) -> bool {
        heap_bytes >= self.trigger_bytes && self.heap_goal_bytes > 0
    }
}
