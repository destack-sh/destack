use std::any::Any;

use destack_core::CaptureMode;
use destack_engine::{Continuation, ExecutionOutcome};
use {destack_heap as heap, destack_vm as vm};

use super::{EngineImage, Entry, LiveContinuation};
use crate::diagnostic::RuntimeResult;

/// Execution engine used by one worker event loop.
pub trait Engine: Any + Send {
    /// Run the entrypoint function.
    fn run(
        &mut self,
        memory: &mut vm::MemoryContext<'_>,
        entry: &Entry,
        args: &[heap::Value],
    ) -> RuntimeResult<ExecutionOutcome<LiveContinuation>>;

    /// Resume execution from a continuation.
    fn resume(
        &mut self,
        memory: &mut vm::MemoryContext<'_>,
        continuation: LiveContinuation,
        value: heap::Value,
    ) -> RuntimeResult<ExecutionOutcome<LiveContinuation>>;

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

/// Heap-layout metadata required while constructing one live engine.
pub trait EngineLayout {
    /// Return the managed-reference width required by this engine.
    fn managed_reference_bytes(&self) -> u8;
}

impl<E> Engine for Box<E>
where
    E: Engine + ?Sized,
{
    fn run(
        &mut self,
        memory: &mut vm::MemoryContext<'_>,
        entry: &Entry,
        args: &[heap::Value],
    ) -> RuntimeResult<ExecutionOutcome<LiveContinuation>> {
        (**self).run(memory, entry, args)
    }

    fn resume(
        &mut self,
        memory: &mut vm::MemoryContext<'_>,
        continuation: LiveContinuation,
        value: heap::Value,
    ) -> RuntimeResult<ExecutionOutcome<LiveContinuation>> {
        (**self).resume(memory, continuation, value)
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

impl<E> EngineLayout for Box<E>
where
    E: EngineLayout + ?Sized,
{
    fn managed_reference_bytes(&self) -> u8 {
        (**self).managed_reference_bytes()
    }
}
