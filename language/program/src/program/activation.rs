use std::fmt;

use crate::{Continuation, Memory, Result, Task, TaskOutcome, Value, Waiter};

/// Runtime services available to executing program code.
pub trait Runtime {
    /// Queue one suspended waiter and return whether this operation settled it.
    fn queue_waiter(&mut self, waiter: Waiter, value: Value) -> Result<bool>;

    /// Cancel one suspended waiter and return whether this operation settled it.
    fn cancel_waiter(&mut self, waiter: Waiter) -> Result<bool>;

    /// Create one already completed task.
    fn resolve_task(&mut self, value: Value) -> Task;

    /// Start one running task.
    fn start_task(&mut self) -> Task;

    /// Suspend one running task and return its runtime waiter.
    fn suspend_task(&mut self, task: Task, continuation: Continuation) -> Result<Waiter>;

    /// Park one waiter until a task completes or is cancelled.
    fn park_task(&mut self, task: Task, waiter: Waiter) -> Result<()>;

    /// Request cooperative cancellation of one task.
    fn cancel_task(&mut self, task: Task) -> Result<()>;

    /// Return whether cooperative cancellation was requested for one running task.
    fn is_task_cancelled(&mut self, task: Task) -> Result<bool>;

    /// Detach one task result.
    fn detach_task(&mut self, task: Task) -> Result<()>;

    /// Finish one running task.
    fn finish_task(&mut self, task: Task, outcome: TaskOutcome) -> Result<()>;
}

/// One active program execution.
pub struct Activation<'runtime, 'memory> {
    /// Runtime operations available to executing code.
    pub runtime: &'runtime mut dyn Runtime,
    /// Memory available to this call.
    pub memory: Memory<'memory>,
}

impl Activation<'_, '_> {
    /// Reborrow this activation for one nested machine call.
    pub fn reborrow(&mut self) -> Activation<'_, '_> {
        Activation {
            runtime: &mut *self.runtime,
            memory: self.memory.reborrow(),
        }
    }
}

impl fmt::Debug for Activation<'_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Activation")
            .field("runtime", &true)
            .field("memory", &self.memory)
            .finish()
    }
}
