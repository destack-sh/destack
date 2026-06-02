use destack_heap as heap;
use serde::{Deserialize, Serialize};

use crate::{EngineCall, EngineMemory, StaticSpace, Value};

/// Execution engine for one worker.
pub trait Engine: Send {
    /// Suspended execution state produced by this engine.
    type Continuation;
    /// Captured execution state produced by this engine.
    type Image;
    /// Engine error.
    type Error;

    /// Initialize worker static memory.
    fn initialize(&mut self, context: EngineMemory<'_>) -> Result<(), Self::Error>;

    /// Run one entrypoint.
    fn run(
        &mut self,
        context: EngineCall<'_>,
        entry: EntryPoint,
        args: &[Value],
    ) -> Result<Outcome<Self::Continuation, Value>, Self::Error>;

    /// Resume one continuation.
    fn resume(
        &mut self,
        context: EngineCall<'_>,
        continuation: Self::Continuation,
        value: Value,
    ) -> Result<Outcome<Self::Continuation, Value>, Self::Error>;

    /// Fork this engine over forked memory.
    fn fork(&self, context: EngineMemory<'_>) -> Result<Self, Self::Error>
    where
        Self: Sized;

    /// Capture one execution image.
    fn image(&self, context: EngineMemory<'_>) -> Result<Self::Image, Self::Error>;

    /// Restore one execution image.
    fn restore(
        &mut self,
        context: EngineMemory<'_>,
        image: &Self::Image,
    ) -> Result<(), Self::Error>;

    /// Visit mutable heap root slots from active engine state.
    fn visit_root_slots(
        &mut self,
        statics: &mut StaticSpace,
        visit: &mut dyn FnMut(heap::RootSlot<'_>) -> heap::HeapResult<()>,
    ) -> Result<(), Self::Error>;

    /// Visit mutable heap root slots from one live continuation.
    fn visit_continuation_root_slots(
        &mut self,
        continuation: &mut Self::Continuation,
        visit: &mut dyn FnMut(heap::RootSlot<'_>) -> heap::HeapResult<()>,
    ) -> Result<(), Self::Error>;
}

/// One engine id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct EngineId(pub u64);

impl EngineId {
    /// Create one engine id.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Return the raw engine id.
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// One program entrypoint id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EntryPoint(u32);

impl EntryPoint {
    /// Create one program entrypoint.
    pub const fn new(index: u32) -> Self {
        Self(index)
    }

    /// Return the entrypoint index.
    pub const fn index(self) -> u32 {
        self.0
    }
}

/// Result of running an engine.
#[derive(Debug)]
pub enum Outcome<C, O, Y = O> {
    /// Execution completed with a result.
    Completed {
        /// The completed execution value.
        value: O,
    },
    /// Execution yielded a continuation and resume value.
    Yielded {
        /// The continuation used to resume execution.
        continuation: C,
        /// The value yielded to the caller.
        value: Y,
    },
}
