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

/// One executable instruction stop point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InstructionStop {
    /// The executable point that can stop.
    pub point: ProgramPoint,
    /// The reason execution stops at this point.
    pub reason: StopReason,
}

/// Active executable stops sorted by program point.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StopSet {
    /// Active instruction stop points.
    instructions: Vec<InstructionStop>,
}

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

impl InstructionStop {
    /// Create one instruction stop point.
    pub const fn new(point: ProgramPoint, reason: StopReason) -> Self {
        Self { point, reason }
    }
}

impl StopSet {
    /// Create one sorted stop set.
    pub fn new(mut instructions: Vec<InstructionStop>) -> Self {
        instructions.sort_by_key(|stop| (stop.point, stop.reason.sort_key()));

        Self { instructions }
    }

    /// Return whether the set has no active stops.
    pub fn is_empty(&self) -> bool {
        self.instructions.is_empty()
    }

    /// Return the stop reason at one executable point.
    pub fn reason_at(
        &self,
        point: ProgramPoint,
        skip_breakpoint: Option<BreakpointId>,
    ) -> Option<StopReason> {
        let mut index = self
            .instructions
            .partition_point(|instruction| instruction.point < point);

        // find the first matching stop that is not being skipped for continue
        while let Some(instruction) = self.instructions.get(index) {
            if instruction.point != point {
                return None;
            }
            if instruction.reason.is_breakpoint(skip_breakpoint) {
                index += 1;
                continue;
            }

            return Some(instruction.reason);
        }

        None
    }
}

impl StopReason {
    /// Return a stable sorting key for deterministic stop selection.
    const fn sort_key(self) -> (u8, u64) {
        match self {
            Self::Instruction { .. } => (0, 0),
            Self::Breakpoint { breakpoint_id } => (1, breakpoint_id.get()),
        }
    }

    /// Return whether this reason is the breakpoint being skipped.
    const fn is_breakpoint(self, skipped: Option<BreakpointId>) -> bool {
        match (self, skipped) {
            (Self::Breakpoint { breakpoint_id }, Some(skipped)) => {
                breakpoint_id.get() == skipped.get()
            }
            (Self::Instruction { .. }, _) | (Self::Breakpoint { .. }, None) => false,
        }
    }
}
