use super::{LimitOptions, TEST_MAX_INSTRUCTIONS, TEST_MAX_STACK_DEPTH, TEST_STACK_BYTES};
use destack_heap::{HeapOptions, SharedHeapOptions};
use serde::{Deserialize, Serialize};

/// Configuration options for a VM machine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MachineOptions {
    /// Resource limits and execution budgets.
    pub limits: LimitOptions,
    /// Local heap shape used to lower allocation sites.
    pub heap: HeapOptions,
    /// Shared heap shape used to lower allocation sites.
    pub shared_heap: SharedHeapOptions,
}

impl Default for MachineOptions {
    fn default() -> Self {
        Self {
            limits: LimitOptions::default(),
            heap: HeapOptions::local(),
            shared_heap: SharedHeapOptions::default(),
        }
    }
}

impl MachineOptions {
    /// Create options with no step limit for trusted code.
    pub fn unbounded() -> Self {
        Self {
            limits: LimitOptions {
                max_instructions: None,
                ..LimitOptions::default()
            },
            ..Self::default()
        }
    }

    /// Create options for testing with smaller limits.
    pub fn test() -> Self {
        Self {
            limits: LimitOptions {
                max_stack_depth: TEST_MAX_STACK_DEPTH,
                stack_bytes: TEST_STACK_BYTES,
                max_instructions: Some(TEST_MAX_INSTRUCTIONS),
            },
            ..Self::default()
        }
    }
}
