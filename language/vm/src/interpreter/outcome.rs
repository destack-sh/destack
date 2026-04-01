use super::Continuation;

/// Output from executing MIR code.
pub type ExecutionOutput = destack_engine::ExecutionOutput;

/// Yield result from a suspended coroutine execution.
pub type ExecutionYield = destack_engine::ExecutionYield<Continuation>;

/// Outcome from a coroutine-capable execution entry.
pub type ExecutionOutcome = destack_engine::ExecutionOutcome<Continuation>;
