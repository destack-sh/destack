use super::{LimitOptions, TEST_MAX_INSTRUCTIONS, TEST_MAX_STACK_BYTES, TEST_MAX_STACK_DEPTH};
use serde::{Deserialize, Serialize};

/// Configuration options for a VM isolate.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IsolateOptions {
    /// Resource limits and execution budgets.
    pub limits: LimitOptions,
}

impl IsolateOptions {
    /// Create options with no step limit for trusted code.
    pub fn unbounded() -> Self {
        Self {
            limits: LimitOptions {
                max_instructions: None,
                ..LimitOptions::default()
            },
        }
    }

    /// Create options for testing with smaller limits.
    pub fn test() -> Self {
        Self {
            limits: LimitOptions {
                max_stack_depth: TEST_MAX_STACK_DEPTH,
                max_stack_bytes: TEST_MAX_STACK_BYTES,
                max_instructions: Some(TEST_MAX_INSTRUCTIONS),
            },
        }
    }

    /// Create options for debug execution.
    pub fn debug() -> Self {
        Self::default()
    }

    /// Create options for comptime execution.
    pub fn comptime() -> Self {
        Self::default()
    }
}
