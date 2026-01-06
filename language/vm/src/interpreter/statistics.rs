/// Statistics collected during interpreter execution.
#[derive(Debug, Clone, Default)]
pub struct Statistics {
    /// Total number of instructions executed.
    pub instructions_executed: u64,
    /// Total number of function calls made.
    pub calls_made: u64,
    /// Maximum call stack depth reached.
    pub max_stack_depth: usize,
    /// Number of heap allocations performed.
    pub heap_allocations: u64,
    /// Number of branch and switch instructions executed.
    pub branches: u64,
    /// Number of load instructions executed.
    pub loads: u64,
    /// Number of store instructions executed.
    pub stores: u64,
}

impl Statistics {
    /// Create new empty statistics.
    pub fn new() -> Self {
        Self::default()
    }

    /// Reset all statistics to zero.
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}
