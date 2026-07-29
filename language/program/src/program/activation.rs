use std::fmt;

use crate::{Binding, Continuation, Memory, Task, TaskOutcome, Value, Waiter, Word};

/// Runtime services available to executing program code.
pub trait Runtime {
    /// Runtime operation failure.
    type Error;

    /// Call one linked runtime binding.
    fn call_binding(
        &mut self,
        memory: Memory<'_>,
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

    /// Suspend one running task and return its runtime waiter.
    fn suspend_task(
        &mut self,
        task: Task,
        continuation: Continuation,
    ) -> Result<Waiter, Self::Error>;

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
        }
    }
}

impl<R> fmt::Debug for Activation<'_, '_, R>
where
    R: Runtime + ?Sized,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Activation")
            .field("runtime", &true)
            .field("memory", &self.memory)
            .finish()
    }
}
