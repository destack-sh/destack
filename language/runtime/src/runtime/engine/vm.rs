use destack_vm::Isolate;
use {destack_heap as heap, destack_vm as vm};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::engine::{
    Engine, EngineContinuation, EngineOutcome, EngineOutput, EngineStats, Entry,
};

/// VM engine implementation for one agent.
impl Engine for Isolate {
    /// Run a VM entrypoint by name.
    fn run(&mut self, entry: &Entry, args: &[heap::Value]) -> RuntimeResult<EngineOutcome> {
        let Entry::Vm { name } = entry else {
            return Err(RuntimeError::Internal {
                message: format!("vm engine cannot run non-vm entry '{}'", entry.name()),
            }
            .boxed());
        };
        let outcome = self
            .run_function_by_name_yielding(name, args)
            .map_err(Box::<RuntimeError>::from)?;
        Ok(map_vm_outcome(outcome))
    }

    /// Resume a VM continuation.
    fn resume(
        &mut self,
        continuation: EngineContinuation,
        value: heap::Value,
    ) -> RuntimeResult<EngineOutcome> {
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
fn map_vm_outcome(outcome: vm::ExecutionOutcome) -> EngineOutcome {
    match outcome {
        vm::ExecutionOutcome::Completed { output } => EngineOutcome::Completed {
            output: map_vm_output(output),
        },
        vm::ExecutionOutcome::Yielded { yielded } => EngineOutcome::Yielded {
            continuation: EngineContinuation::Vm(yielded.continuation),
            value: yielded.value,
        },
    }
}

/// Convert one VM execution output into one runtime engine output.
fn map_vm_output(output: vm::ExecutionOutput) -> EngineOutput {
    EngineOutput {
        value: output.value,
        stats: EngineStats {
            mir_instructions_executed: output.statistics.mir_instructions_executed,
            threaded_instructions_executed: output.statistics.threaded_instructions_executed,
            calls_made: output.statistics.calls_made,
            max_stack_depth: output.statistics.max_stack_depth,
            heap_allocations: output.statistics.heap_allocations,
            branches: output.statistics.branches(),
            loads: output.statistics.loads(),
            stores: output.statistics.stores(),
        },
        heap_cells: output.heap_cells,
        raw_heap_cells: output.raw_heap_cells,
    }
}
