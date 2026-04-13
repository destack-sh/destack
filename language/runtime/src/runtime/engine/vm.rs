use destack_core::CaptureMode;
use destack_engine::{Continuation, ExecutionOutcome};
use destack_vm::Isolate;
use {destack_heap as heap, destack_vm as vm};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::runtime::engine::{Engine, EngineImage, EngineSnapshot, Entry, LiveContinuation};

/// VM engine implementation for one agent.
impl Engine for Isolate {
    /// Return the encoded managed-reference width required by this VM isolate.
    fn heap_managed_reference_bytes(&self) -> u8 {
        Isolate::heap_managed_reference_bytes(self)
    }

    /// Run a VM entrypoint by name.
    fn run(
        &mut self,
        memory: &mut heap::MemoryContext<'_>,
        entry: &Entry,
        args: &[heap::Value],
    ) -> RuntimeResult<ExecutionOutcome<LiveContinuation>> {
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
    ) -> RuntimeResult<ExecutionOutcome<LiveContinuation>> {
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

    /// Validate that one VM continuation supports one capture mode.
    fn validate_capture_mode(
        &self,
        continuation: &LiveContinuation,
        mode: CaptureMode,
    ) -> RuntimeResult<()> {
        let _ = mode;

        let LiveContinuation::Vm(_) = continuation else {
            return Err(RuntimeError::EngineContinuationMismatch {
                engine: "vm".to_string(),
                continuation: "native".to_string(),
            }
            .boxed());
        };

        Ok(())
    }

    /// Reject repeatable dispatch for VM continuations.
    fn clone_for_repeatable_dispatch(
        &self,
        continuation: &LiveContinuation,
    ) -> RuntimeResult<LiveContinuation> {
        let LiveContinuation::Vm(_) = continuation else {
            return Err(RuntimeError::EngineContinuationMismatch {
                engine: "vm".to_string(),
                continuation: "native".to_string(),
            }
            .boxed());
        };

        Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "watch.runnable",
            "vm continuations are not supported for event loop watches",
        ))
        .boxed())
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
        continuation: &LiveContinuation,
    ) -> RuntimeResult<Continuation> {
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
