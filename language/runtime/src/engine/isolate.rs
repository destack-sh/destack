use destack_vm as vm;
use destack_vm::Isolate;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::engine::{Engine, EngineOutcome, EntryPoint};

impl Engine for Isolate {
    /// Run a VM entrypoint by name.
    fn run(&mut self, entry: &EntryPoint, args: &[vm::Value]) -> RuntimeResult<EngineOutcome> {
        let name = match entry {
            EntryPoint::Vm { name } => name,
            EntryPoint::Native { name, .. } => {
                return Err(RuntimeError::internal(format!(
                    "native entrypoint {name} is not available in the VM engine"
                ))
                .boxed());
            }
        };
        let outcome = self
            .run_function_by_name_yielding(name, args)
            .map_err(Box::<RuntimeError>::from)?;
        Ok(map_vm_outcome(outcome))
    }

    /// Resume a VM continuation.
    fn resume(
        &mut self,
        continuation: vm::Continuation,
        value: vm::Value,
    ) -> RuntimeResult<EngineOutcome> {
        let outcome = self
            .resume(continuation, value)
            .map_err(Box::<RuntimeError>::from)?;
        Ok(map_vm_outcome(outcome))
    }
}

/// Convert a VM execution outcome into a runtime engine outcome.
fn map_vm_outcome(outcome: vm::ExecutionOutcome) -> EngineOutcome {
    match outcome {
        vm::ExecutionOutcome::Completed { output } => EngineOutcome::Completed { output },
        vm::ExecutionOutcome::Yielded { yielded } => EngineOutcome::Yielded {
            continuation: yielded.continuation,
            value: yielded.value,
        },
    }
}
