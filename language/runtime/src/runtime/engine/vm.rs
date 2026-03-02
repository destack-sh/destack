use destack_vm as vm;
use destack_vm::Isolate;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::engine::{
    Engine, EngineContinuation, EngineOutcome, AgentOutput, AgentValue, VmEntry,
};

/// VM engine implementation for one agent.
impl Engine for Isolate {
    type Entry = VmEntry;
    type Output = AgentOutput;
    type Value = AgentValue;

    /// Run a VM entrypoint by name.
    fn run(
        &mut self,
        entry: &VmEntry,
        args: &[AgentValue],
    ) -> RuntimeResult<EngineOutcome<AgentOutput, AgentValue>> {
        let outcome = self
            .run_function_by_name_yielding(&entry.name, args)
            .map_err(Box::<RuntimeError>::from)?;
        Ok(map_vm_outcome(outcome))
    }

    /// Resume a VM continuation.
    fn resume(
        &mut self,
        continuation: EngineContinuation,
        value: AgentValue,
    ) -> RuntimeResult<EngineOutcome<AgentOutput, AgentValue>> {
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

/// Convert one VM execution outcome into one engine outcome.
fn map_vm_outcome(outcome: vm::ExecutionOutcome) -> EngineOutcome<AgentOutput, AgentValue> {
    match outcome {
        vm::ExecutionOutcome::Completed { output } => EngineOutcome::Completed { output },
        vm::ExecutionOutcome::Yielded { yielded } => EngineOutcome::Yielded {
            continuation: EngineContinuation::Vm(yielded.continuation),
            value: yielded.value,
        },
    }
}
