use destack_core::CaptureMode;
use destack_vm::Isolate;
use {destack_engine as engine, destack_heap as heap, destack_vm as vm};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::engine::{
    Continuation, Engine, EngineImage, Entry, LiveContinuation, RunOutcome,
};
use crate::runtime::memory::RootVisitor;

/// VM root sink bridged into one runtime root sink.
struct VmRootVisitor<'a, 'b> {
    /// The runtime sink.
    roots: &'a mut RootVisitor<'b>,
}

impl vm::RootVisitor for VmRootVisitor<'_, '_> {
    fn push_heap(&mut self, reference: heap::HeapReference) {
        self.roots.push_heap(reference);
    }

    fn push_shared(&mut self, reference: heap::SharedHeapReference) {
        self.roots.push_shared(reference);
    }
}

/// VM engine implementation for one worker.
impl Engine for Isolate {
    /// Run a VM entrypoint by name.
    fn run(
        &mut self,
        heap: &mut heap::Heap,
        shared: &heap::SharedHeap,
        entry: &Entry,
        args: &[vm::Value],
    ) -> RuntimeResult<RunOutcome<LiveContinuation>> {
        let outcome = self
            .run_function_by_name_yielding(heap, shared, entry.name(), args)
            .map_err(Box::<RuntimeError>::from)?;
        map_vm_outcome(outcome)
    }

    /// Resume a VM continuation.
    fn resume(
        &mut self,
        heap: &mut heap::Heap,
        shared: &heap::SharedHeap,
        continuation: LiveContinuation,
        value: engine::MaterializedValue,
    ) -> RuntimeResult<RunOutcome<LiveContinuation>> {
        let LiveContinuation::Vm(continuation) = continuation else {
            return Err(RuntimeError::EngineContinuationMismatch {
                engine: "vm".to_string(),
                continuation: "native".to_string(),
            }
            .boxed());
        };
        let outcome = self
            .resume(heap, shared, continuation, value)
            .map_err(Box::<RuntimeError>::from)?;
        map_vm_outcome(outcome)
    }

    /// Visit roots from active VM state.
    fn visit_roots(&mut self, roots: &mut RootVisitor<'_>) -> RuntimeResult<()> {
        let mut sink = VmRootVisitor { roots };

        self.visit_state_roots(&[], &mut sink)
            .map_err(Box::<RuntimeError>::from)
    }

    /// Visit roots from one live VM continuation.
    fn visit_live_continuation_roots(
        &mut self,
        continuation: &LiveContinuation,
        roots: &mut RootVisitor<'_>,
    ) -> RuntimeResult<()> {
        let LiveContinuation::Vm(continuation) = continuation else {
            return Err(RuntimeError::EngineContinuationMismatch {
                engine: "vm".to_string(),
                continuation: "native".to_string(),
            }
            .boxed());
        };

        let mut sink = VmRootVisitor { roots };

        self.visit_continuation_roots(continuation, &mut sink)
            .map_err(Box::<RuntimeError>::from)
    }

    /// Visit roots from one captured VM continuation image.
    fn visit_continuation_image_roots(
        &mut self,
        continuation: &Continuation,
        roots: &mut RootVisitor<'_>,
    ) -> RuntimeResult<()> {
        let mut sink = VmRootVisitor { roots };

        self.visit_image_roots(continuation, &mut sink)
            .map_err(Box::<RuntimeError>::from)
    }

    /// Visit roots from one materialized VM boundary value.
    fn visit_materialized_value_roots(
        &mut self,
        value: &engine::MaterializedValue,
        roots: &mut RootVisitor<'_>,
    ) -> RuntimeResult<()> {
        let mut sink = VmRootVisitor { roots };

        Isolate::visit_materialized_value_roots(self, value, &mut sink)
            .map_err(Box::<RuntimeError>::from)
    }

    /// Stabilize one live VM continuation before it escapes into runtime owned state.
    fn stabilize_live_continuation(
        &mut self,
        heap: &mut heap::Heap,
        continuation: &mut LiveContinuation,
    ) -> RuntimeResult<()> {
        let LiveContinuation::Vm(continuation) = continuation else {
            return Ok(());
        };

        Isolate::stabilize_boundary_continuation(self, heap, continuation)
            .map_err(Box::<RuntimeError>::from)
    }

    /// Stabilize one materialized VM boundary value before it escapes into runtime owned state.
    fn stabilize_materialized_value(
        &mut self,
        heap: &mut heap::Heap,
        value: &mut engine::MaterializedValue,
    ) -> RuntimeResult<()> {
        Isolate::stabilize_boundary_value(self, heap, value).map_err(Box::<RuntimeError>::from)
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
        let snapshot = Isolate::image(self).map_err(Box::<RuntimeError>::from)?;

        Ok(EngineImage::Vm(std::sync::Arc::new(snapshot)))
    }

    /// Restore one serialized VM image.
    fn restore_snapshot(
        &mut self,
        heap: &mut heap::Heap,
        snapshot: &EngineImage,
    ) -> RuntimeResult<()> {
        let EngineImage::Vm(snapshot) = snapshot;

        Isolate::restore_image(self, heap, snapshot).map_err(Box::<RuntimeError>::from)
    }
}

/// Convert one VM execution outcome into one engine outcome.
fn map_vm_outcome(outcome: vm::RunOutcome) -> RuntimeResult<RunOutcome<LiveContinuation>> {
    match outcome {
        vm::RunOutcome::Completed { output } => Ok(RunOutcome::Completed { output }),
        vm::RunOutcome::Yielded {
            continuation,
            value,
        } => Ok(RunOutcome::Yielded {
            continuation: LiveContinuation::Vm(continuation),
            value,
        }),
    }
}
