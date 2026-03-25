use destack_heap as heap;

use crate::ExecutionStats;

/// Output from one execution step that completed normally.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ExecutionOutput {
    /// Return value of the executed entrypoint.
    pub value: heap::Value,
    /// Execution statistics payload.
    pub stats: ExecutionStats,
    /// Number of managed allocations at end of execution.
    pub managed_allocation_count: usize,
    /// Number of raw allocations at end of execution.
    pub raw_allocation_count: usize,
}

/// Yield result from one suspended execution step.
#[derive(Debug)]
pub struct ExecutionYield<C> {
    /// The continuation used to resume execution.
    pub continuation: C,
    /// The value yielded to the caller.
    pub value: heap::Value,
}

/// Execution outcome produced by one backend.
#[derive(Debug)]
pub enum ExecutionOutcome<C> {
    /// Execution completed with a result.
    Completed {
        /// Completed execution output.
        output: ExecutionOutput,
    },
    /// Execution yielded a continuation and resume value.
    Yielded {
        /// Yield information for the suspended execution.
        yielded: ExecutionYield<C>,
    },
}
