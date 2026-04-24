use std::any::Any;

use destack_core::CaptureMode;
use {destack_engine as engine, destack_heap as heap, destack_vm as vm};

use super::{Continuation, EngineImage, Entry, LiveContinuation, RunOutcome};
use crate::diagnostic::RuntimeResult;
use crate::runtime::memory::RootVisitor;

/// Execution engine used by one worker event loop.
pub trait Engine: Any + Send {
    /// Run the entrypoint function.
    fn run(
        &mut self,
        heap: &mut heap::Heap,
        shared: &heap::SharedHeap,
        entry: &Entry,
        args: &[vm::Value],
    ) -> RuntimeResult<RunOutcome<LiveContinuation>>;

    /// Resume execution from a continuation.
    fn resume(
        &mut self,
        heap: &mut heap::Heap,
        shared: &heap::SharedHeap,
        continuation: LiveContinuation,
        value: engine::MaterializedValue,
    ) -> RuntimeResult<RunOutcome<LiveContinuation>>;

    /// Visit roots from active engine-owned state.
    fn visit_roots(&mut self, _roots: &mut RootVisitor<'_>) -> RuntimeResult<()> {
        Ok(())
    }

    /// Visit roots from one live continuation.
    fn visit_live_continuation_roots(
        &mut self,
        _continuation: &LiveContinuation,
        _roots: &mut RootVisitor<'_>,
    ) -> RuntimeResult<()> {
        Ok(())
    }

    /// Visit roots from one captured continuation image.
    fn visit_continuation_image_roots(
        &mut self,
        _continuation: &Continuation,
        _roots: &mut RootVisitor<'_>,
    ) -> RuntimeResult<()> {
        Ok(())
    }

    /// Visit roots from one materialized boundary value.
    fn visit_materialized_value_roots(
        &mut self,
        _value: &engine::MaterializedValue,
        _roots: &mut RootVisitor<'_>,
    ) -> RuntimeResult<()> {
        Ok(())
    }

    /// Stabilize one live continuation before it escapes into runtime owned state.
    fn stabilize_live_continuation(
        &mut self,
        _heap: &mut heap::Heap,
        _continuation: &mut LiveContinuation,
    ) -> RuntimeResult<()> {
        Ok(())
    }

    /// Stabilize one materialized boundary value before it escapes into runtime owned state.
    fn stabilize_materialized_value(
        &mut self,
        _heap: &mut heap::Heap,
        _value: &mut engine::MaterializedValue,
    ) -> RuntimeResult<()> {
        Ok(())
    }

    /// Fork one live engine over one already-forked heap.
    fn fork(&mut self, heap: &mut heap::Heap) -> RuntimeResult<Box<dyn Engine>>;

    /// Capture one immutable engine image while the world is checkpoint-ready.
    fn image(&mut self) -> RuntimeResult<EngineImage>;

    /// Restore one immutable engine image while the world is checkpoint-ready.
    fn restore_image(&mut self, heap: &mut heap::Heap, image: &EngineImage) -> RuntimeResult<()>;

    /// Capture one continuation as one immutable continuation image.
    fn continuation_image(
        &mut self,
        continuation: &LiveContinuation,
        mode: CaptureMode,
    ) -> RuntimeResult<Continuation>;

    /// Restore one continuation from one immutable continuation image.
    fn restore_continuation_image(
        &mut self,
        image: &Continuation,
    ) -> RuntimeResult<LiveContinuation>;

    /// Capture one serialized engine image while the world is checkpoint-ready.
    fn snapshot(&mut self) -> RuntimeResult<EngineImage>;

    /// Restore one serialized engine image while the world is checkpoint-ready.
    fn restore_snapshot(
        &mut self,
        heap: &mut heap::Heap,
        snapshot: &EngineImage,
    ) -> RuntimeResult<()>;
}

impl<E> Engine for Box<E>
where
    E: Engine + ?Sized,
{
    fn run(
        &mut self,
        heap: &mut heap::Heap,
        shared: &heap::SharedHeap,
        entry: &Entry,
        args: &[vm::Value],
    ) -> RuntimeResult<RunOutcome<LiveContinuation>> {
        (**self).run(heap, shared, entry, args)
    }

    fn resume(
        &mut self,
        heap: &mut heap::Heap,
        shared: &heap::SharedHeap,
        continuation: LiveContinuation,
        value: engine::MaterializedValue,
    ) -> RuntimeResult<RunOutcome<LiveContinuation>> {
        (**self).resume(heap, shared, continuation, value)
    }

    fn visit_roots(&mut self, roots: &mut RootVisitor<'_>) -> RuntimeResult<()> {
        (**self).visit_roots(roots)
    }

    fn visit_live_continuation_roots(
        &mut self,
        continuation: &LiveContinuation,
        roots: &mut RootVisitor<'_>,
    ) -> RuntimeResult<()> {
        (**self).visit_live_continuation_roots(continuation, roots)
    }

    fn visit_continuation_image_roots(
        &mut self,
        continuation: &Continuation,
        roots: &mut RootVisitor<'_>,
    ) -> RuntimeResult<()> {
        (**self).visit_continuation_image_roots(continuation, roots)
    }

    fn visit_materialized_value_roots(
        &mut self,
        value: &engine::MaterializedValue,
        roots: &mut RootVisitor<'_>,
    ) -> RuntimeResult<()> {
        (**self).visit_materialized_value_roots(value, roots)
    }

    fn stabilize_live_continuation(
        &mut self,
        heap: &mut heap::Heap,
        continuation: &mut LiveContinuation,
    ) -> RuntimeResult<()> {
        (**self).stabilize_live_continuation(heap, continuation)
    }

    fn stabilize_materialized_value(
        &mut self,
        heap: &mut heap::Heap,
        value: &mut engine::MaterializedValue,
    ) -> RuntimeResult<()> {
        (**self).stabilize_materialized_value(heap, value)
    }

    fn fork(&mut self, heap: &mut heap::Heap) -> RuntimeResult<Box<dyn Engine>> {
        (**self).fork(heap)
    }

    fn image(&mut self) -> RuntimeResult<EngineImage> {
        (**self).image()
    }

    fn restore_image(&mut self, heap: &mut heap::Heap, image: &EngineImage) -> RuntimeResult<()> {
        (**self).restore_image(heap, image)
    }

    fn continuation_image(
        &mut self,
        continuation: &LiveContinuation,
        mode: CaptureMode,
    ) -> RuntimeResult<Continuation> {
        (**self).continuation_image(continuation, mode)
    }

    fn restore_continuation_image(
        &mut self,
        image: &Continuation,
    ) -> RuntimeResult<LiveContinuation> {
        (**self).restore_continuation_image(image)
    }

    fn snapshot(&mut self) -> RuntimeResult<EngineImage> {
        (**self).snapshot()
    }

    fn restore_snapshot(
        &mut self,
        heap: &mut heap::Heap,
        snapshot: &EngineImage,
    ) -> RuntimeResult<()> {
        (**self).restore_snapshot(heap, snapshot)
    }
}
