use destack_vm as vm;
use destack_vm::Isolate;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::engine::{
    Engine, EngineContinuation, EngineOutcome, RuntimeOutput, RuntimeValue, VmEntry,
};

/// VM engine implementation for the runtime.
impl Engine for Isolate {
    type Entry = VmEntry;
    type Output = RuntimeOutput;
    type Value = RuntimeValue;

    /// Run a VM entrypoint by name.
    fn run(
        &mut self,
        entry: &VmEntry,
        args: &[RuntimeValue],
    ) -> RuntimeResult<EngineOutcome<RuntimeOutput, RuntimeValue>> {
        let outcome = self
            .run_function_by_name_yielding(&entry.name, args)
            .map_err(Box::<RuntimeError>::from)?;
        Ok(map_vm_outcome(outcome))
    }

    /// Resume a VM continuation.
    fn resume(
        &mut self,
        continuation: EngineContinuation,
        value: RuntimeValue,
    ) -> RuntimeResult<EngineOutcome<RuntimeOutput, RuntimeValue>> {
        let EngineContinuation::Vm(continuation) = continuation else {
            return Err(RuntimeError::Internal {
                message: "vm engine cannot resume native continuation".to_string(),
            }
            .boxed());
        };
        let outcome = self
            .resume(continuation, value)
            .map_err(Box::<RuntimeError>::from)?;
        Ok(map_vm_outcome(outcome))
    }
}

/// Convert a VM execution outcome into a runtime engine outcome.
fn map_vm_outcome(outcome: vm::ExecutionOutcome) -> EngineOutcome<RuntimeOutput, RuntimeValue> {
    match outcome {
        vm::ExecutionOutcome::Completed { output } => EngineOutcome::Completed { output },
        vm::ExecutionOutcome::Yielded { yielded } => EngineOutcome::Yielded {
            continuation: EngineContinuation::Vm(yielded.continuation),
            value: yielded.value,
        },
    }
}
