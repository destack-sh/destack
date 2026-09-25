use std::{error, fmt};

use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

/// One exhausted machine resource.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum ResourceError {
    /// The VM stack byte limit was exceeded.
    StackOverflow,
    /// The VM frame depth limit was exceeded.
    FrameLimitExceeded,
    /// The instruction execution limit was exceeded.
    InstructionLimitExceeded,
    /// World memory could not reserve or materialize VM stack storage.
    MemoryExhausted,
}

impl fmt::Display for ResourceError {
    /// Format one exhausted machine resource.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let resource = match self {
            Self::StackOverflow => "stack bytes",
            Self::FrameLimitExceeded => "frame depth",
            Self::InstructionLimitExceeded => "instruction count",
            Self::MemoryExhausted => "world memory",
        };

        formatter.write_str(resource)
    }
}

impl error::Error for ResourceError {}
