use destack_vm as vm;

use super::EngineContinuation;
use crate::diagnostic::RuntimeResult;

/// Runtime value passed across engine yields.
pub type RuntimeValue = vm::Value;

/// Runtime output produced when execution completes.
pub type RuntimeOutput = vm::ExecutionOutput;

/// Execution outcome produced by a runtime engine.
#[derive(Debug)]
pub enum EngineOutcome<Output, Value> {
    /// Execution completed with a result.
    Completed { output: Output },
    /// Execution yielded a continuation and resume value.
    Yielded {
        continuation: EngineContinuation,
        value: Value,
    },
}

/// Execution engine used by the runtime event loop.
pub trait Engine {
    /// Entry point handle for this engine.
    type Entry;
    /// Output value produced when execution completes.
    type Output;
    /// Value type passed across yields.
    type Value;

    /// Run the entrypoint function.
    fn run(
        &mut self,
        entry: &Self::Entry,
        args: &[Self::Value],
    ) -> RuntimeResult<EngineOutcome<Self::Output, Self::Value>>;

    /// Resume execution from a continuation.
    fn resume(
        &mut self,
        continuation: EngineContinuation,
        value: Self::Value,
    ) -> RuntimeResult<EngineOutcome<Self::Output, Self::Value>>;
}
