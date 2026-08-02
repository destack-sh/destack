use std::fmt;

use destack_heap::{HeapResult, RootSlot};

use crate::{Binding, Context, Continuation, Memory, Task, TaskOutcome, Value, Waiter, Word};

/// Action returned by one runtime poll.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Poll {
    /// Continue execution in the current engine.
    Continue,
    /// Retain execution and return control to the host.
    Pause,
    /// Retain execution for transfer to another engine.
    Deoptimize,
}

/// Mutable roots retained by one active execution engine.
pub trait RootSet {
    /// Root traversal failure.
    type Error;

    /// Visit every mutable root retained by the active engine.
    fn visit(
        &mut self,
        visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
    ) -> Result<(), Self::Error>;
}

impl<E, F> RootSet for F
where
    F: FnMut(&mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>) -> Result<(), E>,
{
    type Error = E;

    /// Visit every mutable root retained by the closure.
    fn visit(
        &mut self,
        visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
    ) -> Result<(), Self::Error> {
        self(visit)
    }
}

/// Runtime services available to executing program code.
pub trait Runtime {
    /// Runtime operation failure.
    type Error;

    /// Return whether runtime work is pending.
    fn is_poll_requested(&self) -> bool;

    /// Service pending runtime work against the active roots.
    fn poll(
        &mut self,
        memory: Memory<'_>,
        roots: &mut dyn RootSet<Error = Self::Error>,
    ) -> Result<Poll, Self::Error>;

    /// Call one linked runtime binding.
    fn call_binding(
        &mut self,
        memory: Memory<'_>,
        context: Context,
        binding: &Binding,
        arguments: &[Word],
        result: &mut [Word],
    ) -> Result<(), Self::Error>;

    /// Queue one suspended waiter and return whether this operation settled it.
    fn queue_waiter(&mut self, waiter: Waiter, value: Value) -> Result<bool, Self::Error>;

    /// Cancel one suspended waiter and return whether this operation settled it.
    fn cancel_waiter(&mut self, waiter: Waiter) -> Result<bool, Self::Error>;

    /// Create one already completed task.
    fn resolve_task(&mut self, value: Value) -> Task;

    /// Start one running task.
    fn start_task(&mut self) -> Task;

    /// Suspend one running task or return its continuation unchanged.
    fn suspend_task(
        &mut self,
        task: Task,
        continuation: Continuation,
    ) -> Result<Waiter, (Self::Error, Continuation)>;

    /// Park one waiter until a task completes or is cancelled.
    fn park_task(&mut self, task: Task, waiter: Waiter) -> Result<(), Self::Error>;

    /// Request cooperative cancellation of one task.
    fn cancel_task(&mut self, task: Task) -> Result<(), Self::Error>;

    /// Return whether cooperative cancellation was requested for one running task.
    fn is_task_cancelled(&mut self, task: Task) -> Result<bool, Self::Error>;

    /// Detach one task result.
    fn detach_task(&mut self, task: Task) -> Result<(), Self::Error>;

    /// Finish one running task.
    fn finish_task(&mut self, task: Task, outcome: TaskOutcome) -> Result<(), Self::Error>;
}

/// One active program execution.
pub struct Activation<'runtime, 'memory, R>
where
    R: Runtime + ?Sized,
{
    /// Runtime operations available to executing code.
    pub runtime: &'runtime mut R,
    /// Memory available to this call.
    pub memory: Memory<'memory>,
    /// Current dynamically scoped execution context.
    pub context: &'runtime mut Context,
}

impl<R> Activation<'_, '_, R>
where
    R: Runtime + ?Sized,
{
    /// Reborrow this activation for one nested machine call.
    pub fn reborrow(&mut self) -> Activation<'_, '_, R> {
        Activation {
            runtime: &mut *self.runtime,
            memory: self.memory.reborrow(),
            context: &mut *self.context,
        }
    }
}

impl<R> fmt::Debug for Activation<'_, '_, R>
where
    R: Runtime + ?Sized,
{
    /// Format one active program execution.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Activation")
            .field("runtime", &true)
            .field("memory", &self.memory)
            .field("context", &self.context)
            .finish()
    }
}
