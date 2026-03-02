use destack_heap as heap;

use super::{EngineContinuation, EngineTelemetry};
use crate::diagnostic::RuntimeResult;

/// Engine output produced when execution completes.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EngineOutput {
    /// Return value of the executed entrypoint.
    pub value: heap::Value,
    /// Execution telemetry payload.
    pub telemetry: EngineTelemetry,
    /// Number of managed heap cells at end of execution.
    pub heap_cells: usize,
    /// Number of raw heap cells at end of execution.
    pub raw_heap_cells: usize,
}

/// Execution outcome produced by one engine.
#[derive(Debug)]
pub enum EngineOutcome {
    /// Execution completed with a result.
    Completed { output: EngineOutput },
    /// Execution yielded a continuation and resume value.
    Yielded {
        continuation: EngineContinuation,
        value: heap::Value,
    },
}

/// Execution engine used by one agent event loop.
pub trait Engine {
    /// Entry point handle for this engine.
    type Entry;

    /// Run the entrypoint function.
    fn run(&mut self, entry: &Self::Entry, args: &[heap::Value]) -> RuntimeResult<EngineOutcome>;

    /// Resume execution from a continuation.
    fn resume(
        &mut self,
        continuation: EngineContinuation,
        value: heap::Value,
    ) -> RuntimeResult<EngineOutcome>;
}
