/// Configuration options for the machine interpreter.
#[derive(Debug, Clone)]
pub struct MachineOptions {
    /// Maximum call stack depth before stack overflow error.
    /// Default: 1024
    pub max_stack_depth: usize,

    /// Maximum number of heap cells before allocation fails.
    /// Default: 100_000 (roughly 10MB depending on cell size)
    pub max_heap_cells: usize,

    /// Maximum number of instructions to execute before timeout.
    /// None means no limit (use with caution).
    pub max_steps: Option<u64>,
}

impl Default for MachineOptions {
    fn default() -> Self {
        Self {
            max_stack_depth: 1024,
            max_heap_cells: 100_000,
            max_steps: Some(10_000_000),
        }
    }
}

impl MachineOptions {
    /// Create options with no step limit (for trusted code).
    pub fn unlimited_steps() -> Self {
        Self {
            max_steps: None,
            ..Default::default()
        }
    }

    /// Create options for testing with smaller limits.
    pub fn for_testing() -> Self {
        Self {
            max_stack_depth: 100,
            max_heap_cells: 1000,
            max_steps: Some(100_000),
        }
    }
}
