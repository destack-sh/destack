use destack_engine::{ExecutionOutcome, ExecutionOutput, ExecutionStats};
use destack_vm::Isolate;
use {destack_heap as heap, destack_vm as vm};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::engine::{
    Engine, EngineContinuation, EngineContinuationImage, EngineImage, EngineSnapshot, Entry,
};

/// VM engine implementation for one agent.
impl Engine for Isolate {
    /// Run a VM entrypoint by name.
    fn run(
        &mut self,
        memory: &mut heap::MemoryContext<'_>,
        entry: &Entry,
        args: &[heap::Value],
    ) -> RuntimeResult<ExecutionOutcome<EngineContinuation>> {
        let outcome = self
            .run_function_by_name_yielding(memory, entry.name(), args)
            .map_err(Box::<RuntimeError>::from)?;
        Ok(map_vm_outcome(outcome))
    }

    /// Run one replayable VM entrypoint by name.
    fn run_replayable_entry(
        &mut self,
        memory: &mut heap::MemoryContext<'_>,
        entry: &Entry,
        args: &[heap::Value],
    ) -> RuntimeResult<ExecutionOutcome<EngineContinuation>> {
        let name = entry.name();
        let outcome = self
            .run_function_by_name_yielding(memory, name, args)
            .map_err(Box::<RuntimeError>::from)?;
        Ok(map_vm_outcome(outcome))
    }

    /// Resume a VM continuation.
    fn resume(
        &mut self,
        memory: &mut heap::MemoryContext<'_>,
        continuation: EngineContinuation,
        value: heap::Value,
    ) -> RuntimeResult<ExecutionOutcome<EngineContinuation>> {
        let EngineContinuation::Vm(continuation) = continuation else {
            return Err(RuntimeError::EngineContinuationMismatch {
                engine: "vm".to_string(),
                continuation: "native".to_string(),
            }
            .boxed());
        };
        let outcome = self
            .resume(memory, continuation, value)
            .map_err(Box::<RuntimeError>::from)?;
        Ok(map_vm_outcome(outcome))
    }

    /// Capture one immutable VM image.
    fn image(&mut self) -> RuntimeResult<EngineImage> {
        let image = Isolate::image(self).map_err(Box::<RuntimeError>::from)?;

        Ok(EngineImage::Vm(std::sync::Arc::new(image)))
    }

    /// Restore one immutable VM image.
    fn restore_image(&mut self, heap: &mut heap::Heap, image: &EngineImage) -> RuntimeResult<()> {
        let EngineImage::Vm(image) = image;

        Isolate::restore_image(self, heap, image).map_err(Box::<RuntimeError>::from)
    }

    /// Capture one continuation as one immutable VM continuation image.
    fn continuation_image(
        &mut self,
        continuation: &EngineContinuation,
    ) -> RuntimeResult<EngineContinuationImage> {
        let EngineContinuation::Vm(continuation) = continuation else {
            return Err(RuntimeError::EngineContinuationMismatch {
                engine: "vm".to_string(),
                continuation: "native".to_string(),
            }
            .boxed());
        };

        Ok(EngineContinuationImage::Vm(Isolate::continuation_image(
            self,
            continuation,
        )))
    }

    /// Restore one continuation from one immutable VM continuation image.
    fn restore_continuation_image(
        &mut self,
        image: &EngineContinuationImage,
    ) -> RuntimeResult<EngineContinuation> {
        let EngineContinuationImage::Vm(image) = image else {
            return Err(RuntimeError::EngineContinuationMismatch {
                engine: "vm".to_string(),
                continuation: "native".to_string(),
            }
            .boxed());
        };

        let continuation =
            Isolate::restore_continuation_image(self, image).map_err(Box::<RuntimeError>::from)?;

        Ok(EngineContinuation::Vm(continuation))
    }

    /// Capture one serialized VM snapshot.
    fn snapshot(&mut self) -> RuntimeResult<EngineSnapshot> {
        let snapshot = Isolate::snapshot(self).map_err(Box::<RuntimeError>::from)?;

        Ok(EngineSnapshot::Vm(snapshot))
    }

    /// Restore one serialized VM snapshot.
    fn restore_snapshot(
        &mut self,
        heap: &mut heap::Heap,
        snapshot: &EngineSnapshot,
    ) -> RuntimeResult<()> {
        let EngineSnapshot::Vm(snapshot) = snapshot;

        Isolate::restore_snapshot(self, heap, snapshot).map_err(Box::<RuntimeError>::from)
    }
}

/// Convert one VM execution outcome into one engine outcome.
fn map_vm_outcome(outcome: vm::ExecutionOutcome) -> ExecutionOutcome<EngineContinuation> {
    match outcome {
        vm::ExecutionOutcome::Completed { output } => ExecutionOutcome::Completed {
            output: map_vm_output(output),
        },
        vm::ExecutionOutcome::Yielded { yielded } => ExecutionOutcome::Yielded {
            yielded: destack_engine::ExecutionYield {
                continuation: EngineContinuation::Vm(yielded.continuation),
                value: yielded.value,
            },
        },
    }
}

/// Convert one VM execution output into one runtime engine output.
fn map_vm_output(output: vm::ExecutionOutput) -> ExecutionOutput {
    ExecutionOutput {
        value: output.value,
        stats: ExecutionStats {
            mir_instructions_executed: output.statistics.mir_instructions_executed,
            lowered_instructions_executed: output.statistics.lowered_instructions_executed,
            calls_made: output.statistics.calls_made,
            max_stack_depth: output.statistics.max_stack_depth,
            heap_allocations: output.statistics.heap_allocations,
            branches: output.statistics.branches(),
            loads: output.statistics.loads(),
            stores: output.statistics.stores(),
        },
        managed_allocation_count: output.managed_allocation_count,
        raw_allocation_count: output.raw_allocation_count,
    }
}
