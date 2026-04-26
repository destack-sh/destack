use serde::{Deserialize, Serialize};

use crate::{HeapError, HeapResult};

/// The default proportional heap growth target after one cycle.
const DEFAULT_GROWTH_PERCENT: u32 = 100;
/// The standard heap trigger as a percentage of the current goal.
const DEFAULT_TRIGGER_PERCENT: u32 = 75;
/// The default heap pacing floor.
const DEFAULT_MINIMUM_HEAP_BYTES: u64 = 4 * 1024 * 1024;

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
}

impl GcOptions {
    /// Build the default collector configuration for one heap.
    pub fn local() -> Self {
        Self {
            growth_percent: DEFAULT_GROWTH_PERCENT,
            trigger_percent: DEFAULT_TRIGGER_PERCENT,
            soft_limit_bytes: None,
            minimum_heap_bytes: Some(DEFAULT_MINIMUM_HEAP_BYTES),
        }
    }

    /// Build the default collector configuration for one shared heap.
    pub fn shared() -> Self {
        Self {
            growth_percent: DEFAULT_GROWTH_PERCENT,
            trigger_percent: DEFAULT_TRIGGER_PERCENT,
            soft_limit_bytes: None,
            minimum_heap_bytes: Some(DEFAULT_MINIMUM_HEAP_BYTES),
        }
    }

    /// Validate this collector configuration.
    pub fn validate(self) -> HeapResult<Self> {
        if self.trigger_percent > 100 {
            return Err(HeapError::InvalidGcTriggerPercent {
                percent: self.trigger_percent,
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
}

impl GcPacer {
    /// Refresh pacing targets from one collector config and live heap size.
    pub fn update(&mut self, options: GcOptions, live_bytes: u64) {
        let min_bytes = options.minimum_heap_bytes.unwrap_or(0);
        let growth = live_bytes.saturating_mul(u64::from(options.growth_percent)) / 100;
        let mut goal_bytes = live_bytes.saturating_add(growth).max(min_bytes);

        if let Some(soft_limit_bytes) = options.soft_limit_bytes {
            goal_bytes = goal_bytes.min(soft_limit_bytes);
        }

        let trigger_bytes = goal_bytes.saturating_mul(u64::from(options.trigger_percent)) / 100;

        self.live_bytes = live_bytes;
        self.goal_bytes = goal_bytes;
        self.trigger_bytes = trigger_bytes.max(min_bytes);
    }

    /// Return whether the current heap size should start one cycle.
    pub fn should_start(&self, heap_bytes: u64) -> bool {
        heap_bytes >= self.trigger_bytes && self.goal_bytes > 0
    }

    /// Return whether the current heap size should force one full cycle.
    pub fn should_collect_full(&self, heap_bytes: u64) -> bool {
        heap_bytes >= self.goal_bytes && self.goal_bytes > 0
    }
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
    /// Total active allocator bytes after the collection.
    pub active_bytes: u64,
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
    pub fn record_cycle(&mut self, kind: GcKind, stats: GcStats) -> HeapResult<()> {
        self.completed_cycles =
            self.completed_cycles
                .checked_add(1)
                .ok_or(HeapError::InvariantOverflow {
                    context: "gc cycle count",
                })?;
        self.last_kind = Some(kind);
        self.last_stats = Some(stats);

        Ok(())
    }
}
