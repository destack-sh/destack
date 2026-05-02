use super::{
    CheckOptions, LimitOptions, TEST_MAX_INSTRUCTIONS, TEST_MAX_STACK_BYTES, TEST_MAX_STACK_DEPTH,
};
use serde::{Deserialize, Serialize};

/// Configuration options for a VM isolate.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IsolateOptions {
    /// Runtime check configuration.
    pub checks: CheckOptions,
    /// Resource limits and execution budgets.
    pub limits: LimitOptions,
}

impl IsolateOptions {
    /// Create options with no step limit for trusted code.
    pub fn unbounded() -> Self {
        // use default options without instruction limits
        let mut options = Self::default();
        options.limits.max_instructions = None;
        options
    }

    /// Create options for testing with smaller limits.
    pub fn test() -> Self {
        // use strict settings with tighter resource limits
        let mut options = Self::default();
        options.limits.max_stack_depth = TEST_MAX_STACK_DEPTH;
        options.limits.max_stack_bytes = TEST_MAX_STACK_BYTES;
        options.limits.max_instructions = Some(TEST_MAX_INSTRUCTIONS);
        options.checks = CheckOptions::debug();
        options
    }

    /// Create options for debug execution.
    pub fn debug() -> Self {
        // use strict runtime checks
        let mut options = Self::default();
        options.checks = CheckOptions::debug();
        options
    }

    /// Create options for comptime execution.
    pub fn comptime() -> Self {
        Self::default()
    }
}
