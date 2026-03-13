use destack_heap::GcStats;
use destack_workspace::HeapOptions;

use super::GcPacer;

/// Runtime GC controller for pacing and trigger policy.
#[derive(Debug, Clone, Default)]
pub struct Gc {
    /// Heap pacing options for collection policy.
    pub options: HeapOptions,
    /// Pacing targets derived from recent heap state.
    pub pacer: GcPacer,
}

impl Gc {
    /// Configure GC pacing options.
    pub fn configure(&mut self, options: HeapOptions) {
        self.options = options;
    }

    /// Check whether a collection cycle should start for this heap size.
    pub fn should_collect(&mut self, heap_bytes: u64) -> bool {
        // refresh pacing targets from the latest heap size
        self.pacer.update(&self.options, heap_bytes);

        // trigger when heap has crossed the current threshold
        self.pacer.should_start(heap_bytes)
    }

    /// Record one completed collection cycle.
    pub fn on_cycle_complete(&mut self, stats: GcStats) {
        // seed next cycle targets from observed live bytes
        self.pacer.update(&self.options, stats.live_bytes);
    }
}
