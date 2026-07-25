use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use super::{Continuation, FunctionId, ProgramPoint, WatchpointId};

/// Result of running program code.
#[derive(Debug)]
pub enum Outcome<T> {
    /// Execution completed with a result.
    Completed {
        /// The completed execution value.
        value: T,
    },
    /// Execution awaited one asynchronous value.
    Awaited {
        /// The concrete park implementation selected by the awaitable type.
        park: FunctionId,
        /// The asynchronous value passed to the park implementation.
        awaitable: T,
        /// The continuation resumed when the asynchronous value settles.
        continuation: Continuation,
    },
    /// Execution yielded one generator value.
    Yielded {
        /// The value yielded to the generator owner.
        value: T,
        /// The continuation resumed by the next generator command.
        continuation: Continuation,
    },
    /// Execution stopped for host inspection.
    Stopped {
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
        /// Program point that stopped execution.
        point: ProgramPoint,
    },
    /// Runtime watchpoint.
    Watchpoint {
        /// Watchpoint that stopped execution.
        watchpoint_id: WatchpointId,
        /// Program point that accessed watched memory.
        point: ProgramPoint,
    },
}

/// Runtime breakpoint identifier.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct BreakpointId(u64);

/// One program instruction stop point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InstructionStop {
    /// The program point that can stop.
    pub point: ProgramPoint,
    /// The reason execution stops at this point.
    pub reason: StopReason,
}

/// One stop skipped once when continuing retained execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ResumeSkip {
    /// Program point that produced the retained stop.
    pub point: ProgramPoint,
    /// Reason that produced the retained stop.
    pub reason: StopReason,
}

/// Active program stops sorted by program point.
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

    /// Return the stop reason at one program point.
    pub fn reason_at(
        &self,
        point: ProgramPoint,
        resume_skip: Option<ResumeSkip>,
    ) -> Option<StopReason> {
        let mut index = self
            .instructions
            .partition_point(|instruction| instruction.point < point);

        // find the first matching stop that is not being skipped for continue
        while let Some(instruction) = self.instructions.get(index) {
            if instruction.point != point {
                return None;
            }
            if resume_skip.is_some_and(|skip| skip.selects(point, instruction.reason)) {
                index += 1;
                continue;
            }

            return Some(instruction.reason);
        }

        None
    }
}

impl StopReason {
    /// Return the program point that produced this stop.
    pub const fn point(self) -> ProgramPoint {
        match self {
            Self::Instruction { point }
            | Self::Breakpoint { point, .. }
            | Self::Watchpoint { point, .. } => point,
        }
    }

    /// Return the stop skipped when resuming from this stop.
    pub const fn resume_skip(self) -> Option<ResumeSkip> {
        match self {
            Self::Breakpoint { .. } => Some(ResumeSkip {
                point: self.point(),
                reason: self,
            }),
            Self::Instruction { .. } | Self::Watchpoint { .. } => None,
        }
    }

    /// Return a stable sorting key for deterministic stop selection.
    const fn sort_key(self) -> (u8, u64) {
        match self {
            Self::Instruction { .. } => (0, 0),
            Self::Breakpoint { breakpoint_id, .. } => (1, breakpoint_id.get()),
            Self::Watchpoint { watchpoint_id, .. } => (2, watchpoint_id.get()),
        }
    }
}

impl ResumeSkip {
    /// Return whether this skip covers one stop at one point.
    pub fn selects(self, point: ProgramPoint, reason: StopReason) -> bool {
        if self.point != point {
            return false;
        }

        match self.reason {
            StopReason::Breakpoint { .. } => matches!(reason, StopReason::Breakpoint { .. }),
            StopReason::Instruction { .. } | StopReason::Watchpoint { .. } => false,
        }
    }
}
