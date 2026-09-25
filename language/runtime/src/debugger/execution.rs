use serde::{Deserialize, Serialize};
use tspp_program as program;
use tspp_serde::Reflect;

use crate::runtime::RuntimeId;
use crate::worker::WorkerId;

/// World-local execution selected for debugger control.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum Execution {
    /// Every Runtime in the World.
    World,
    /// One Runtime.
    Runtime {
        /// Selected Runtime.
        runtime_id: RuntimeId,
    },
    /// One Worker.
    Worker {
        /// Selected Runtime.
        runtime_id: RuntimeId,
        /// Selected Worker.
        worker_id: WorkerId,
    },
    /// One Fiber.
    Fiber {
        /// Selected Runtime.
        runtime_id: RuntimeId,
        /// Selected Worker.
        worker_id: WorkerId,
        /// Selected Fiber.
        fiber_id: program::FiberId,
    },
}

/// Debugger step granularity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum Step {
    /// Execute one Program instruction.
    Instruction,
    /// Stop inside the next entered call.
    Into,
    /// Stop at the next point in the current Frame.
    Over,
    /// Stop after the current Frame returns.
    Out,
}
