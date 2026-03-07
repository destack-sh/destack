use std::any::Any;

use destack_heap as heap;

use super::{EngineContinuation, EngineSnapshot, EngineStats, Entry};
use crate::diagnostic::RuntimeResult;

/// Engine output produced when execution completes.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EngineOutput {
    /// Return value of the executed entrypoint.
    pub value: heap::Value,
    /// Execution statistics payload.
    pub stats: EngineStats,
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
pub trait Engine: Any {
    /// Run the entrypoint function.
    fn run(&mut self, entry: &Entry, args: &[heap::Value]) -> RuntimeResult<EngineOutcome>;

    /// Resume execution from a continuation.
    fn resume(
        &mut self,
        continuation: EngineContinuation,
        value: heap::Value,
    ) -> RuntimeResult<EngineOutcome>;

    /// Capture one durable engine snapshot while the world is checkpoint-ready.
    fn snapshot(&mut self) -> RuntimeResult<EngineSnapshot>;

    /// Restore one durable engine snapshot while the world is checkpoint-ready.
    fn restore(&mut self, snapshot: &EngineSnapshot) -> RuntimeResult<()>;
}
