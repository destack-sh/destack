/// The maximum assist work consumed by one shared mutator step.
pub(crate) const SHARED_ASSIST_BUDGET: usize = 256;

/// The heap pages represented by one shared mark budget unit.
pub(crate) const SHARED_MARK_PAGES_PER_BUDGET: usize = 16;

/// The heap pages represented by one shared sweep budget unit.
pub(crate) const SHARED_SWEEP_PAGES_PER_BUDGET: usize = 8;

/// The mark budget contributed by one live worker.
pub(crate) const SHARED_MARK_BUDGET_PER_WORKER: usize = 32;

/// The sweep budget contributed by one live worker.
pub(crate) const SHARED_SWEEP_BUDGET_PER_WORKER: usize = 64;

/// The local shared-edge scan budget contributed by one live worker.
pub(crate) const SHARED_EDGE_SCAN_BUDGET_PER_WORKER: usize = 16;

/// The minimum shared mark budget in one step.
pub(crate) const MIN_SHARED_MARK_BUDGET: usize = 64;

/// The maximum shared mark budget in one step.
pub(crate) const MAX_SHARED_MARK_BUDGET: usize = 4096;

/// The minimum shared sweep budget in one step.
pub(crate) const MIN_SHARED_SWEEP_BUDGET: usize = 128;

/// The maximum shared sweep budget in one step.
pub(crate) const MAX_SHARED_SWEEP_BUDGET: usize = 8192;

/// The minimum local shared-edge scan budget in one step.
pub(crate) const MIN_SHARED_EDGE_SCAN_BUDGET: usize = 32;

/// The maximum local shared-edge scan budget in one step.
pub(crate) const MAX_SHARED_EDGE_SCAN_BUDGET: usize = 1024;
