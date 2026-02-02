use destack_vm as vm;

use crate::diagnostic::RuntimeResult;

use super::EntryPoint;

/// Execution outcome produced by a runtime engine.
#[derive(Debug)]
pub enum EngineOutcome {
    /// Execution completed with a result.
    Completed { output: vm::ExecutionOutput },
    /// Execution yielded a continuation and resume value.
    Yielded {
        continuation: vm::Continuation,
        value: vm::Value,
    },
}

/// Execution engine used by the runtime scheduler.
pub trait Engine {
    /// Run the entrypoint function.
    fn run(&mut self, entry: &EntryPoint, args: &[vm::Value]) -> RuntimeResult<EngineOutcome>;

    /// Resume execution from a continuation.
    fn resume(
        &mut self,
        continuation: vm::Continuation,
        value: vm::Value,
    ) -> RuntimeResult<EngineOutcome>;
}
