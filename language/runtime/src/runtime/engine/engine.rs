use std::any::Any;

use destack_core::CaptureMode;
use destack_engine::{Continuation, ExecutionOutcome};
use destack_heap as heap;

use super::{EngineImage, EngineSnapshot, Entry, LiveContinuation};
use crate::diagnostic::RuntimeResult;

/// Execution engine used by one agent event loop.
pub trait Engine: Any + Send {
    /// Return the encoded managed-reference width required by this engine.
    fn heap_managed_reference_bytes(&self) -> u8;

    /// Run the entrypoint function.
    fn run(
        &mut self,
        memory: &mut heap::MemoryContext<'_>,
        entry: &Entry,
        args: &[heap::Value],
    ) -> RuntimeResult<ExecutionOutcome<LiveContinuation>>;

    /// Run one replayable entrypoint descriptor.
    fn run_replayable_entry(
        &mut self,
        memory: &mut heap::MemoryContext<'_>,
        entry: &Entry,
        args: &[heap::Value],
    ) -> RuntimeResult<ExecutionOutcome<LiveContinuation>>;

    /// Resume execution from a continuation.
    fn resume(
        &mut self,
        memory: &mut heap::MemoryContext<'_>,
        continuation: LiveContinuation,
        value: heap::Value,
    ) -> RuntimeResult<ExecutionOutcome<LiveContinuation>>;

    /// Validate whether one live continuation supports the requested capture mode.
    fn validate_capture_mode(
        &self,
        continuation: &LiveContinuation,
        mode: CaptureMode,
    ) -> RuntimeResult<()>;

    /// Clone one continuation for repeatable event-loop watch dispatch.
    fn clone_for_repeatable_dispatch(
        &self,
        continuation: &LiveContinuation,
    ) -> RuntimeResult<LiveContinuation>;

    /// Capture one immutable engine image while the world is checkpoint-ready.
    fn image(&mut self) -> RuntimeResult<EngineImage>;

    /// Restore one immutable engine image while the world is checkpoint-ready.
    fn restore_image(&mut self, heap: &mut heap::Heap, image: &EngineImage) -> RuntimeResult<()>;

    /// Capture one continuation as one immutable continuation image.
    fn continuation_image(
        &mut self,
        continuation: &LiveContinuation,
    ) -> RuntimeResult<Continuation>;

    /// Restore one continuation from one immutable continuation image.
    fn restore_continuation_image(
        &mut self,
        image: &Continuation,
    ) -> RuntimeResult<LiveContinuation>;

    /// Capture one serialized engine snapshot while the world is checkpoint-ready.
    fn snapshot(&mut self) -> RuntimeResult<EngineSnapshot>;

    /// Restore one serialized engine snapshot while the world is checkpoint-ready.
    fn restore_snapshot(
        &mut self,
        heap: &mut heap::Heap,
        snapshot: &EngineSnapshot,
    ) -> RuntimeResult<()>;
}
