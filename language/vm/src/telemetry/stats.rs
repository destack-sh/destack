use destack_engine::RunStats;
use serde::{Deserialize, Serialize};

/// Statistics collected during interpreter execution.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Statistics {
    /// Total number of MIR instructions executed (original IR).
    pub mir_instructions_executed: u64,
    /// Total number of lowered instructions executed.
    pub lowered_instructions_executed: u64,
    /// Total number of function calls made.
    pub calls_made: u64,
    /// Maximum call stack depth reached.
    pub max_stack_depth: usize,
    /// Number of heap allocations performed (always tracked for gc decisions).
    pub heap_allocations: u64,

    // detailed statistics: only tracked when the `stats` feature is enabled
    /// Number of branch and switch instructions executed.
    #[cfg(feature = "stats")]
    pub branches: u64,
    /// Number of load instructions executed.
    #[cfg(feature = "stats")]
    pub loads: u64,
    /// Number of store instructions executed.
    #[cfg(feature = "stats")]
    pub stores: u64,
}

/// Increment a statistic counter (no-op when stats feature is disabled).
#[cfg(feature = "stats")]
macro_rules! stat_inc {
    ($stats:expr, $field:ident) => {
        $stats.$field += 1
    };
}

/// Increment a statistic counter (no-op when stats feature is disabled).
#[cfg(not(feature = "stats"))]
macro_rules! stat_inc {
    ($stats:expr, $field:ident) => {};
}

pub(crate) use stat_inc;

impl Statistics {
    /// Create new empty statistics.
    pub fn new() -> Self {
        Self::default()
    }

    /// Reset all statistics to zero.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Get branch count (0 when stats feature is disabled).
    #[inline]
    pub fn branches(&self) -> u64 {
        #[cfg(feature = "stats")]
        {
            self.branches
        }
        #[cfg(not(feature = "stats"))]
        {
            0
        }
    }

    /// Get load count (0 when stats feature is disabled).
    #[inline]
    pub fn loads(&self) -> u64 {
        #[cfg(feature = "stats")]
        {
            self.loads
        }
        #[cfg(not(feature = "stats"))]
        {
            0
        }
    }

    /// Get store count (0 when stats feature is disabled).
    #[inline]
    pub fn stores(&self) -> u64 {
        #[cfg(feature = "stats")]
        {
            self.stores
        }
        #[cfg(not(feature = "stats"))]
        {
            0
        }
    }
}

impl From<Statistics> for RunStats {
    fn from(stats: Statistics) -> Self {
        Self {
            mir_instructions_executed: stats.mir_instructions_executed,
            lowered_instructions_executed: stats.lowered_instructions_executed,
            calls_made: stats.calls_made,
            max_stack_depth: stats.max_stack_depth,
            heap_allocations: stats.heap_allocations,
            branches: stats.branches(),
            loads: stats.loads(),
            stores: stats.stores(),
        }
    }
}

impl From<&Statistics> for RunStats {
    fn from(stats: &Statistics) -> Self {
        Self {
            mir_instructions_executed: stats.mir_instructions_executed,
            lowered_instructions_executed: stats.lowered_instructions_executed,
            calls_made: stats.calls_made,
            max_stack_depth: stats.max_stack_depth,
            heap_allocations: stats.heap_allocations,
            branches: stats.branches(),
            loads: stats.loads(),
            stores: stats.stores(),
        }
    }
}

impl From<RunStats> for Statistics {
    fn from(stats: RunStats) -> Self {
        Self {
            mir_instructions_executed: stats.mir_instructions_executed,
            lowered_instructions_executed: stats.lowered_instructions_executed,
            calls_made: stats.calls_made,
            max_stack_depth: stats.max_stack_depth,
            heap_allocations: stats.heap_allocations,
            #[cfg(feature = "stats")]
            branches: stats.branches,
            #[cfg(feature = "stats")]
            loads: stats.loads,
            #[cfg(feature = "stats")]
            stores: stats.stores,
        }
    }
}
