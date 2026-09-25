use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

use super::{
    DEFAULT_MAX_FRAMES, DEFAULT_MAX_INSTRUCTIONS, DEFAULT_STACK_BYTES, TEST_MAX_FRAMES,
    TEST_MAX_INSTRUCTIONS, TEST_STACK_BYTES,
};

/// Resource limits for one VM machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct MachineLimits {
    /// The maximum number of active call frames.
    pub max_frames: usize,
    /// The byte limit for the VM frame stack.
    pub stack_bytes: usize,
    /// The instruction limit, or none for unbounded execution.
    pub max_instructions: Option<u64>,
}

impl Default for MachineLimits {
    /// Create the default machine limits.
    fn default() -> Self {
        Self {
            max_frames: DEFAULT_MAX_FRAMES,
            stack_bytes: DEFAULT_STACK_BYTES,
            max_instructions: Some(DEFAULT_MAX_INSTRUCTIONS),
        }
    }
}

impl MachineLimits {
    /// Create limits with no instruction budget.
    pub fn unbounded() -> Self {
        Self {
            max_instructions: None,
            ..Self::default()
        }
    }

    /// Create smaller limits for deterministic tests.
    pub const fn test() -> Self {
        Self {
            max_frames: TEST_MAX_FRAMES,
            stack_bytes: TEST_STACK_BYTES,
            max_instructions: Some(TEST_MAX_INSTRUCTIONS),
        }
    }
}
