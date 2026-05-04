use serde::{Deserialize, Serialize};

use crate::{Context, Value};

/// Execution engine for one worker.
pub trait Engine: Send {
    /// Suspended execution state produced by this engine.
    type Continuation;
    /// Captured execution state produced by this engine.
    type Image;
    /// Engine error.
    type Error;

    /// Initialize worker static memory.
    fn initialize(&mut self, context: Context<'_>) -> Result<(), Self::Error>;

    /// Run one entrypoint.
    fn run(
        &mut self,
        context: Context<'_>,
        entry: Entry,
        args: &[Value],
    ) -> Result<Outcome<Self::Continuation, Value>, Self::Error>;

    /// Resume one continuation.
    fn resume(
        &mut self,
        context: Context<'_>,
        continuation: Self::Continuation,
        value: Value,
    ) -> Result<Outcome<Self::Continuation, Value>, Self::Error>;

    /// Fork this engine over forked memory.
    fn fork(&self, context: Context<'_>) -> Result<Self, Self::Error>
    where
        Self: Sized;

    /// Capture one execution image.
    fn image(&self, context: Context<'_>) -> Result<Self::Image, Self::Error>;

    /// Restore one execution image.
    fn restore(&mut self, context: Context<'_>, image: &Self::Image) -> Result<(), Self::Error>;
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

/// One program entrypoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Entry(u32);

impl Entry {
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
