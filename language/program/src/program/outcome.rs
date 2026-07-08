use serde::{Deserialize, Serialize};

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StopReason {
    /// Debugger breakpoint.
    Breakpoint,
}
