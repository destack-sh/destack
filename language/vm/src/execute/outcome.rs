use crate::memory::Value;
use crate::telemetry::Statistics;

use super::Continuation;

/// Output from executing MIR code.
#[derive(Debug, Clone)]
pub struct ExecutionOutput {
    /// The return value of the executed function.
    pub value: Value,
    /// Statistics from this execution.
    pub statistics: Statistics,
    /// Number of managed heap cells at end of execution.
    pub heap_cells: usize,
    /// Number of raw heap cells at end of execution.
    pub raw_heap_cells: usize,
}

/// Yield result from a suspended coroutine execution.
#[derive(Debug)]
pub struct ExecutionYield {
    /// The value yielded to the caller.
    pub value: Value,
    /// The continuation used to resume execution.
    pub continuation: Continuation,
}

/// Outcome from a coroutine-capable execution entry.
#[derive(Debug)]
pub enum ExecutionOutcome {
    /// Execution completed with a final result.
    Completed {
        /// Completed execution output.
        output: ExecutionOutput,
    },
    /// Execution suspended with a yielded value.
    Yielded {
        /// Yield information for the suspended execution.
        yielded: ExecutionYield,
    },
}
