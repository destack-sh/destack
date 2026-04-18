use destack_core::CaptureMode;
use destack_engine::{Continuation, ExecutionOutcome};
use destack_vm::Isolate;
use {destack_heap as heap, destack_vm as vm};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::engine::{Engine, EngineImage, EngineLayout, Entry, LiveContinuation};

/// VM engine implementation for one worker.
impl Engine for Isolate {
    /// Run a VM entrypoint by name.
    fn run(
        &mut self,
        memory: &mut vm::MemoryContext<'_>,
        entry: &Entry,
        args: &[heap::Value],
    ) -> RuntimeResult<ExecutionOutcome<LiveContinuation>> {
        let outcome = self
            .run_function_by_name_yielding(memory, entry.name(), args)
            .map_err(Box::<RuntimeError>::from)?;
        Ok(map_vm_outcome(outcome))
    }

    /// Resume a VM continuation.
    fn resume(
        &mut self,
        memory: &mut vm::MemoryContext<'_>,
        continuation: LiveContinuation,
        value: heap::Value,
    ) -> RuntimeResult<ExecutionOutcome<LiveContinuation>> {
        let LiveContinuation::Vm(continuation) = continuation else {
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

    /// Fork one live VM engine over one already-forked heap.
    fn fork(&mut self, heap: &mut heap::Heap) -> RuntimeResult<Box<dyn Engine>> {
        let _ = heap;

        let isolate = Isolate::fork(self).map_err(Box::<RuntimeError>::from)?;

        Ok(Box::new(isolate))
    }

    /// Restore one immutable VM image.
    fn restore_image(&mut self, heap: &mut heap::Heap, image: &EngineImage) -> RuntimeResult<()> {
        let EngineImage::Vm(image) = image;

        Isolate::restore_image(self, heap, image).map_err(Box::<RuntimeError>::from)
    }

    /// Capture one continuation as one immutable VM continuation image.
    fn continuation_image(
        &mut self,
        continuation: &LiveContinuation,
        mode: CaptureMode,
    ) -> RuntimeResult<Continuation> {
        let _ = mode;

        let LiveContinuation::Vm(continuation) = continuation else {
            return Err(RuntimeError::EngineContinuationMismatch {
                engine: "vm".to_string(),
                continuation: "native".to_string(),
            }
            .boxed());
        };

        Isolate::continuation_image(self, continuation).map_err(Box::<RuntimeError>::from)
    }

    /// Restore one continuation from one immutable VM continuation image.
    fn restore_continuation_image(
        &mut self,
        image: &Continuation,
    ) -> RuntimeResult<LiveContinuation> {
        let continuation =
            Isolate::restore_continuation_image(self, image).map_err(Box::<RuntimeError>::from)?;

        Ok(LiveContinuation::Vm(continuation))
    }

    /// Capture one serialized VM image.
    fn snapshot(&mut self) -> RuntimeResult<EngineImage> {
        let snapshot = Isolate::snapshot(self).map_err(Box::<RuntimeError>::from)?;

        Ok(EngineImage::Vm(std::sync::Arc::new(snapshot)))
    }

    /// Restore one serialized VM image.
    fn restore_snapshot(
        &mut self,
        heap: &mut heap::Heap,
        snapshot: &EngineImage,
    ) -> RuntimeResult<()> {
        let EngineImage::Vm(snapshot) = snapshot;

        Isolate::restore_snapshot(self, heap, snapshot).map_err(Box::<RuntimeError>::from)
    }
}

/// VM heap-layout metadata.
impl EngineLayout for Isolate {
    /// Return the managed-reference width required by this VM isolate.
    fn managed_reference_bytes(&self) -> u8 {
        Isolate::heap_managed_reference_bytes(self)
    }
}

/// Convert one VM execution outcome into one engine outcome.
fn map_vm_outcome(outcome: vm::ExecutionOutcome) -> ExecutionOutcome<LiveContinuation> {
    match outcome {
        vm::ExecutionOutcome::Completed { output } => ExecutionOutcome::Completed { output },
        vm::ExecutionOutcome::Yielded { yielded } => ExecutionOutcome::Yielded {
            yielded: destack_engine::ExecutionYield {
                continuation: LiveContinuation::Vm(yielded.continuation),
                value: yielded.value,
            },
        },
    }
}
