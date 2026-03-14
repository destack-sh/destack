use std::any::Any;

use destack_heap as heap;

use super::{
    EngineContinuation, EngineContinuationImage, EngineImage, EngineSnapshot, EngineStats, Entry,
};
use crate::diagnostic::RuntimeResult;
use crate::runtime::engine::EntryReference;

/// Engine output produced when execution completes.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EngineOutput {
    /// Return value of the executed entrypoint.
    pub value: heap::Value,
    /// Execution statistics payload.
    pub stats: EngineStats,
    /// Number of managed allocations at end of execution.
    pub managed_allocation_count: usize,
    /// Number of raw allocations at end of execution.
    pub raw_allocation_count: usize,
}

/// Execution outcome produced by one engine.
#[derive(Debug)]
pub enum EngineOutcome {
    /// Execution completed with a result.
    Completed { output: EngineOutput },
    /// Execution yielded a continuation and resume value.
    Yielded {
        continuation: EngineContinuation,
        value: heap::Value,
    },
}

/// Execution engine used by one agent event loop.
pub trait Engine: Any {
    /// Run the entrypoint function.
    fn run(
        &mut self,
        memory: &mut heap::AgentMemory<'_>,
        entry: &Entry,
        args: &[heap::Value],
    ) -> RuntimeResult<EngineOutcome>;

    /// Run one replayable entrypoint descriptor.
    fn run_replayable_entry(
        &mut self,
        memory: &mut heap::AgentMemory<'_>,
        entry: &EntryReference,
        args: &[heap::Value],
    ) -> RuntimeResult<EngineOutcome>;

    /// Resume execution from a continuation.
    fn resume(
        &mut self,
        memory: &mut heap::AgentMemory<'_>,
        continuation: EngineContinuation,
        value: heap::Value,
    ) -> RuntimeResult<EngineOutcome>;

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
