use std::any::Any;

use destack_engine::ExecutionOutcome;
use destack_heap as heap;

use super::{EngineContinuation, EngineContinuationImage, EngineImage, EngineSnapshot, Entry};
use crate::diagnostic::RuntimeResult;

/// Execution engine used by one agent event loop.
pub trait Engine: Any {
    /// Run the entrypoint function.
    fn run(
        &mut self,
        memory: &mut heap::MemoryContext<'_>,
        entry: &Entry,
        args: &[heap::Value],
    ) -> RuntimeResult<ExecutionOutcome<EngineContinuation>>;

    /// Run one replayable entrypoint descriptor.
    fn run_replayable_entry(
        &mut self,
        memory: &mut heap::MemoryContext<'_>,
        entry: &Entry,
        args: &[heap::Value],
    ) -> RuntimeResult<ExecutionOutcome<EngineContinuation>>;

    /// Resume execution from a continuation.
    fn resume(
        &mut self,
        memory: &mut heap::MemoryContext<'_>,
        continuation: EngineContinuation,
        value: heap::Value,
    ) -> RuntimeResult<ExecutionOutcome<EngineContinuation>>;

    /// Capture one immutable engine image while the world is checkpoint-ready.
    fn image(&mut self) -> RuntimeResult<EngineImage>;

    /// Restore one immutable engine image while the world is checkpoint-ready.
    fn restore_image(&mut self, heap: &mut heap::Heap, image: &EngineImage) -> RuntimeResult<()>;

    /// Capture one continuation as one immutable continuation image.
    fn continuation_image(
        &mut self,
        continuation: &EngineContinuation,
    ) -> RuntimeResult<EngineContinuationImage>;

    /// Restore one continuation from one immutable continuation image.
    fn restore_continuation_image(
        &mut self,
        image: &EngineContinuationImage,
    ) -> RuntimeResult<EngineContinuation>;

    /// Capture one serialized engine snapshot while the world is checkpoint-ready.
    fn snapshot(&mut self) -> RuntimeResult<EngineSnapshot>;

    /// Restore one serialized engine snapshot while the world is checkpoint-ready.
    fn restore_snapshot(
        &mut self,
        heap: &mut heap::Heap,
        snapshot: &EngineSnapshot,
    ) -> RuntimeResult<()>;
}
