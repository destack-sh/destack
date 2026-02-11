use destack_workspace::GcOptions;

const DEFAULT_MIN_HEAP_BYTES: u64 = 8 * 1024 * 1024;
const DEFAULT_TRIGGER_RATIO_PERCENT: u64 = 75;

/// GC pacing targets derived from live heap size.
#[derive(Debug, Clone, Copy, Default)]
pub struct GcPacer {
    /// Estimated live heap size in bytes.
    pub live_bytes: u64,
    /// Target heap size in bytes for the next cycle.
    pub heap_goal_bytes: u64,
    /// Trigger point for starting a cycle.
    pub trigger_bytes: u64,
}

impl GcPacer {
    /// Update pacing targets from options and live heap size.
    pub fn update(&mut self, options: &GcOptions, live_bytes: u64) {
        // derive the minimum heap floor
        let min_heap_bytes = options.heap_initial_bytes.unwrap_or(DEFAULT_MIN_HEAP_BYTES);

        // derive the next heap goal from configured growth
        let growth_percent = u64::from(options.heap_growth_percent);
        let growth = live_bytes.saturating_mul(growth_percent);
        let growth = growth / 100;
        let mut goal = live_bytes.saturating_add(growth).max(min_heap_bytes);

        // clamp to optional soft cap
        if let Some(max) = options.heap_soft_limit_bytes {
            goal = goal.min(max);
        }

        // compute trigger threshold from the current goal
        let trigger = goal.saturating_mul(DEFAULT_TRIGGER_RATIO_PERCENT) / 100;

        self.live_bytes = live_bytes;
        self.heap_goal_bytes = goal;
        self.trigger_bytes = trigger.max(min_heap_bytes);
    }

    /// Check whether the heap size should trigger a new GC cycle.
    pub fn should_start(&self, heap_bytes: u64) -> bool {
        heap_bytes >= self.trigger_bytes && self.heap_goal_bytes > 0
    }
}
