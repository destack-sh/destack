use serde::{Deserialize, Serialize};

use super::{DEFAULT_MAX_INSTRUCTIONS, DEFAULT_MAX_STACK_BYTES, DEFAULT_MAX_STACK_DEPTH};

/// Resource limits for a VM isolate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimitOptions {
    /// The maximum call stack depth before a stack overflow error.
    pub max_stack_depth: usize,
    /// The maximum byte width of the VM stack arena.
    pub max_stack_bytes: usize,
    /// The maximum number of instructions to execute before timeout.
    /// None means no limit, use with caution.
    pub max_instructions: Option<u64>,
}

impl Default for LimitOptions {
    fn default() -> Self {
        // use default runtime limits
        Self {
            max_stack_depth: DEFAULT_MAX_STACK_DEPTH,
            max_stack_bytes: DEFAULT_MAX_STACK_BYTES,
            max_instructions: Some(DEFAULT_MAX_INSTRUCTIONS),
        }
    }
}
