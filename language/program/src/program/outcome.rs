use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use super::ProgramPoint;

/// Result of running program code.
#[derive(Debug)]
pub enum Outcome<C, O, Y = O> {
    /// Execution completed with a result.
    Completed {
        /// The completed execution value.
        value: O,
    },
    /// Execution yielded a continuation and resume value.
    Yielded {
        /// The continuation used to resume execution.
        continuation: C,
        /// The value yielded to the caller.
        value: Y,
    },
    /// Execution stopped for host inspection.
    Stopped {
        /// The continuation used to continue execution.
        continuation: C,
        /// The reason execution stopped.
        reason: StopReason,
    },
}

/// Reason execution stopped before completion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum StopReason {
    /// Explicit program breakpoint instruction.
    Instruction {
        /// Program point that stopped execution.
        point: ProgramPoint,
    },
    /// Runtime breakpoint.
    Breakpoint {
        /// Breakpoint that stopped execution.
        breakpoint_id: BreakpointId,
    },
}

/// Runtime breakpoint identifier.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct BreakpointId(u64);

impl BreakpointId {
    /// Create one breakpoint identifier.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Return the raw breakpoint identifier value.
    pub const fn get(self) -> u64 {
        self.0
    }
}
