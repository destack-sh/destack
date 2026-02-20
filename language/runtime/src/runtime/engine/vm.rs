use destack_vm as vm;
use destack_vm::Isolate;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::engine::{Engine, EngineOutcome, VmEntry};

/// VM engine implementation for the runtime.
impl Engine for Isolate {
    type Entry = VmEntry;
    type Output = vm::ExecutionOutput;
    type Continuation = vm::Continuation;
    type Value = vm::Value;

    /// Run a VM entrypoint by name.
    fn run(
        &mut self,
        entry: &VmEntry,
        args: &[vm::Value],
    ) -> RuntimeResult<EngineOutcome<vm::ExecutionOutput, vm::Continuation, vm::Value>> {
        let outcome = self
            .run_function_by_name_yielding(&entry.name, args)
            .map_err(Box::<RuntimeError>::from)?;
        Ok(map_vm_outcome(outcome))
    }

    /// Resume a VM continuation.
    fn resume(
        &mut self,
        continuation: vm::Continuation,
        value: vm::Value,
    ) -> RuntimeResult<EngineOutcome<vm::ExecutionOutput, vm::Continuation, vm::Value>> {
        let outcome = self
            .resume(continuation, value)
            .map_err(Box::<RuntimeError>::from)?;
        Ok(map_vm_outcome(outcome))
    }
}

/// Convert a VM execution outcome into a runtime engine outcome.
fn map_vm_outcome(
    outcome: vm::ExecutionOutcome,
) -> EngineOutcome<vm::ExecutionOutput, vm::Continuation, vm::Value> {
    match outcome {
        vm::ExecutionOutcome::Completed { output } => EngineOutcome::Completed { output },
        vm::ExecutionOutcome::Yielded { yielded } => EngineOutcome::Yielded {
            continuation: yielded.continuation,
            value: yielded.value,
        },
    }
}
