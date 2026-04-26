use destack_heap::Heap;

use crate::{Context, Entry, Outcome, Value};

/// Execution backend for one worker.
pub trait Engine: Send {
    /// Live continuation produced by this backend.
    type Continuation;
    /// In-memory execution image produced by this backend.
    type Image;
    /// Error returned by this backend.
    type Error;

    /// Initialize worker-owned static bytes.
    fn initialize(&mut self, _context: Context<'_>) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Run one entrypoint.
    fn run(
        &mut self,
        context: Context<'_>,
        entry: &Entry,
        args: &[Value],
    ) -> Result<Outcome<Self::Continuation, Value>, Self::Error>;

    /// Resume one continuation.
    fn resume(
        &mut self,
        context: Context<'_>,
        continuation: Self::Continuation,
        value: Value,
    ) -> Result<Outcome<Self::Continuation, Value>, Self::Error>;

    /// Fork this backend over one already-forked heap.
    fn fork(&mut self, heap: &mut Heap) -> Result<Self, Self::Error>
    where
        Self: Sized;

    /// Capture one immutable execution image.
    fn image(&mut self) -> Result<Self::Image, Self::Error>;

    /// Restore one immutable execution image.
    fn restore(&mut self, heap: &mut Heap, image: &Self::Image) -> Result<(), Self::Error>;
}
