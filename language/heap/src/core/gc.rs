use serde::{Deserialize, Serialize};

use crate::{HeapError, HeapResult};

/// The default proportional heap growth target after one cycle.
pub const DEFAULT_GC_GROWTH_PERCENT: u32 = 100;
/// The standard heap trigger as a percentage of the current goal.
pub const DEFAULT_GC_TRIGGER_PERCENT: u32 = 75;
/// The default heap pacing floor.
pub const DEFAULT_GC_MINIMUM_HEAP_BYTES: u64 = 4 * 1024 * 1024;
/// The default minimum collector work for one safepoint.
pub const DEFAULT_GC_MINIMUM_WORK_BYTES: usize = 64 * 1024;
/// The default smoothing weight for observed collector work.
const GC_WORK_ESTIMATE_OLD_WEIGHT: u64 = 7;
/// The default smoothing divisor for observed collector work.
const GC_WORK_ESTIMATE_WEIGHT_TOTAL: u64 = 8;

/// Collector configuration for one heap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GcOptions {
    /// The proportional heap growth target after one cycle.
    pub growth_percent: u32,
    /// The heap trigger as a percentage of the current goal.
    pub trigger_percent: u32,
    /// Optional soft memory limit in bytes.
    pub soft_limit_bytes: Option<u64>,
    /// Optional minimum heap floor in bytes.
    pub minimum_heap_bytes: Option<u64>,
    /// Minimum collector work for one safepoint.
    pub minimum_work_bytes: usize,
}

impl GcOptions {
    /// Build the default collector configuration for one heap.
    pub fn local() -> Self {
        Self {
            growth_percent: DEFAULT_GC_GROWTH_PERCENT,
            trigger_percent: DEFAULT_GC_TRIGGER_PERCENT,
            soft_limit_bytes: None,
            minimum_heap_bytes: Some(DEFAULT_GC_MINIMUM_HEAP_BYTES),
            minimum_work_bytes: DEFAULT_GC_MINIMUM_WORK_BYTES,
        }
    }

    /// Build the default collector configuration for one shared heap.
    pub fn shared() -> Self {
        Self {
            growth_percent: DEFAULT_GC_GROWTH_PERCENT,
            trigger_percent: DEFAULT_GC_TRIGGER_PERCENT,
            soft_limit_bytes: None,
            minimum_heap_bytes: Some(DEFAULT_GC_MINIMUM_HEAP_BYTES),
            minimum_work_bytes: DEFAULT_GC_MINIMUM_WORK_BYTES,
        }
    }

    /// Validate this collector configuration.
    pub fn validate(self) -> HeapResult<Self> {
        if self.trigger_percent > 100 {
            return Err(HeapError::InvalidGcTriggerPercent {
                percent: self.trigger_percent,
            });
        }

        if self.minimum_work_bytes == 0 {
            return Err(HeapError::InvalidGcMinimumWorkBytes {
                bytes: self.minimum_work_bytes,
            });
        }

        Ok(self)
    }
}

/// Derived pacing targets for one heap collector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct GcPacer {
    /// The live heap bytes used to derive the current targets.
    pub live_bytes: u64,
    /// The current heap goal in bytes.
    pub goal_bytes: u64,
    /// The current collection trigger in bytes.
    pub trigger_bytes: u64,
    /// The estimated collector work for one complete cycle.
    pub estimated_work_bytes: u64,
    /// The estimated collector work not yet issued to collector steps.
    pub remaining_work_bytes: u64,
    /// The pending collector assist debt in work bytes.
    pub assist_debt_bytes: u64,
}

/// GC pressure requested by one pacer snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GcPressure {
    /// No collection work is currently requested.
    Idle,
    /// A regular collection cycle is requested.
    Cycle,
    /// A full collection cycle should start.
    Full,
}

impl GcPacer {
    /// Derive pacing targets from the last completed live heap size.
    pub fn set_live_bytes(&mut self, options: GcOptions, live_bytes: u64) {
        // heap goal
        let mut min_bytes = 0;
        if let Some(bytes) = options.minimum_heap_bytes {
            min_bytes = bytes;
        }
        let growth = live_bytes * u64::from(options.growth_percent) / 100;
        let mut goal_bytes = (live_bytes + growth).max(min_bytes);

        // apply the configured soft ceiling after normal growth pacing
        if let Some(soft_limit_bytes) = options.soft_limit_bytes {
            goal_bytes = goal_bytes.min(soft_limit_bytes);
        }

        // collection trigger
        let trigger_bytes = goal_bytes * u64::from(options.trigger_percent) / 100;

        self.live_bytes = live_bytes;
        self.goal_bytes = goal_bytes;
        self.trigger_bytes = trigger_bytes.max(min_bytes);

        // initial work estimate
        if self.estimated_work_bytes == 0 {
            self.estimated_work_bytes = initial_work_estimate(options, live_bytes);
        }
    }

    /// Start one paced collection cycle from the current heap size.
    pub fn begin_cycle(&mut self, options: GcOptions, heap_bytes: u64) {
        if self.goal_bytes == 0 && heap_bytes == 0 {
            self.set_live_bytes(options, heap_bytes);
        }

        // the new cycle owns all outstanding collector work
        self.remaining_work_bytes = self
            .estimated_work_bytes
            .max(options.minimum_work_bytes as u64);
        self.assist_debt_bytes = 0;
    }

    /// Record one completed collection cycle and refresh future pacing.
    pub fn record_cycle(&mut self, options: GcOptions, stats: GcStats) {
        let observed_work_bytes = observed_cycle_work(options, stats);
        self.estimated_work_bytes =
            smooth_work_estimate(self.estimated_work_bytes, observed_work_bytes);
        self.remaining_work_bytes = 0;
        self.assist_debt_bytes = 0;

        self.set_live_bytes(options, stats.allocated_bytes);
    }

    /// Return the collection pressure for the current heap size.
    pub fn pressure(&self, heap_bytes: u64) -> GcPressure {
        if self.goal_bytes == 0 && heap_bytes > 0 {
            return GcPressure::Cycle;
        }

        if heap_bytes >= self.goal_bytes && self.goal_bytes > 0 {
            return GcPressure::Full;
        }

        if heap_bytes >= self.trigger_bytes && self.goal_bytes > 0 {
            return GcPressure::Cycle;
        }

        GcPressure::Idle
    }

    /// Charge one allocation against the current collector runway.
    pub fn charge_allocation(&mut self, options: GcOptions, byte_len: usize) {
        let debt_bytes = self.allocation_debt_bytes(options, byte_len);

        self.assist_debt_bytes += debt_bytes;
    }

    /// Return the collector work owed by one allocation.
    pub fn allocation_debt_bytes(&self, options: GcOptions, byte_len: usize) -> u64 {
        if byte_len == 0 {
            return 0;
        }

        // clamp degenerate soft-limit targets to one byte of runway
        let runway_bytes = if self.goal_bytes > self.trigger_bytes {
            self.goal_bytes - self.trigger_bytes
        } else {
            1
        };
        let work_bytes = self
            .estimated_work_bytes
            .max(options.minimum_work_bytes as u64);
        let debt_bytes = (byte_len as u128 * work_bytes as u128).div_ceil(runway_bytes as u128);

        debt_bytes.max(1).min(u128::from(u64::MAX)) as u64
    }

    /// Consume pending collector assist debt as bytes.
    pub fn take_assist_budget_bytes(&mut self, budget_bytes: usize) -> usize {
        let debt_bytes = self.assist_debt_bytes.min(budget_bytes as u64);

        self.assist_debt_bytes -= debt_bytes;
        self.consume_work(debt_bytes);

        debt_bytes as usize
    }

    /// Return the base work budget for one collector increment.
    pub fn base_budget_bytes(&self, options: GcOptions, worker_count: usize) -> usize {
        let worker_count = worker_count.max(1) as u64;
        let minimum_bytes = options.minimum_work_bytes as u64 * worker_count;

        if self.remaining_work_bytes == 0 {
            return minimum_bytes as usize;
        }

        self.remaining_work_bytes.min(minimum_bytes) as usize
    }

    /// Return one byte budget plus pending allocation assist work.
    pub fn budget_bytes(&mut self, options: GcOptions, worker_count: usize) -> usize {
        let base_bytes = self.base_budget_bytes(options, worker_count);
        let assist_bytes = self.take_assist_budget_bytes(base_bytes);
        let budget_bytes = base_bytes + assist_bytes;

        // the caller owns the returned work quantum
        self.consume_work(base_bytes as u64);

        budget_bytes
    }

    /// Consume one issued collector work budget.
    pub fn consume_work(&mut self, budget_bytes: u64) {
        // clamp overspent work at zero
        if budget_bytes >= self.remaining_work_bytes {
            self.remaining_work_bytes = 0;

            return;
        }

        self.remaining_work_bytes -= budget_bytes;
    }
}

/// Return the initial collector work estimate for one heap size.
fn initial_work_estimate(options: GcOptions, live_bytes: u64) -> u64 {
    live_bytes.max(options.minimum_work_bytes as u64)
}

/// Return the observed collector work from one completed cycle.
fn observed_cycle_work(options: GcOptions, stats: GcStats) -> u64 {
    let swept_bytes = stats.allocated_bytes + stats.freed_bytes;
    let retained_bytes = stats.retained_bytes;
    let observed_bytes = swept_bytes.max(retained_bytes);

    observed_bytes.max(options.minimum_work_bytes as u64)
}

/// Return one smoothed collector work estimate.
fn smooth_work_estimate(previous_bytes: u64, observed_bytes: u64) -> u64 {
    if previous_bytes == 0 {
        return observed_bytes;
    }

    let previous_weighted = u128::from(previous_bytes) * u128::from(GC_WORK_ESTIMATE_OLD_WEIGHT);
    let observed_weighted = u128::from(observed_bytes);
    let smoothed_bytes =
        (previous_weighted + observed_weighted) / u128::from(GC_WORK_ESTIMATE_WEIGHT_TOTAL);

    smoothed_bytes.min(u128::from(u64::MAX)) as u64
}

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
    /// Total retained allocator bytes after the collection.
    pub retained_bytes: u64,
}

/// Result of one bounded collector increment.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum GcProgress {
    /// No collector work was available.
    #[default]
    Idle,
    /// Collector work ran but the cycle is not complete.
    Active,
    /// One collection cycle completed.
    Complete(GcStats),
}

impl GcProgress {
    /// Return whether this increment did collector work.
    pub const fn made_progress(self) -> bool {
        !matches!(self, Self::Idle)
    }

    /// Return the completed cycle stats when this increment finished a cycle.
    pub const fn completed_stats(self) -> Option<GcStats> {
        match self {
            Self::Complete(stats) => Some(stats),
            Self::Idle | Self::Active => None,
        }
    }
}

/// GC summary tracked across collection cycles.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct GcState {
    /// Number of completed GC cycles.
    pub completed_cycles: u64,
    /// The scope of the last completed cycle.
    pub last_kind: Option<GcKind>,
    /// The summary statistics for the last completed cycle.
    pub last_stats: Option<GcStats>,
}

impl GcState {
    /// Record one completed GC cycle.
    pub fn record_cycle(&mut self, kind: GcKind, stats: GcStats) {
        self.completed_cycles += 1;
        self.last_kind = Some(kind);
        self.last_stats = Some(stats);
    }
}
