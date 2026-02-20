use crate::diagnostic::RuntimeResult;

/// Execution outcome produced by a runtime engine.
#[derive(Debug)]
pub enum EngineOutcome<Output, Continuation, Value> {
    /// Execution completed with a result.
    Completed { output: Output },
    /// Execution yielded a continuation and resume value.
    Yielded {
        continuation: Continuation,
        value: Value,
    },
}

/// Execution engine used by the runtime event loop.
pub trait Engine {
    /// Entry point handle for this engine.
    type Entry;
    /// Output value produced when execution completes.
    type Output;
    /// Continuation type used for yielding execution.
    type Continuation;
    /// Value type passed across yields.
    type Value;

    /// Run the entrypoint function.
    fn run(
        &mut self,
        entry: &Self::Entry,
        args: &[Self::Value],
    ) -> RuntimeResult<EngineOutcome<Self::Output, Self::Continuation, Self::Value>>;

    /// Resume execution from a continuation.
    fn resume(
        &mut self,
        continuation: Self::Continuation,
        value: Self::Value,
    ) -> RuntimeResult<EngineOutcome<Self::Output, Self::Continuation, Self::Value>>;
}
