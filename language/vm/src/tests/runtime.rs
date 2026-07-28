use std::mem;

use destack_program as program;

/// One runtime boundary call made by bytecode execution.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum RuntimeCall {
    /// Settle one waiter with a value.
    Queue {
        /// Waiter passed to the runtime.
        waiter: program::Waiter,
        /// Value passed to the runtime.
        value: program::Value,
    },
    /// Cancel one waiter.
    CancelWaiter(program::Waiter),
    /// Create one completed task.
    Resolve {
        /// Task returned by the runtime.
        task: program::Task,
        /// Completed task value.
        value: program::Value,
    },
    /// Start one eager task.
    Start(program::Task),
    /// Suspend one eager task.
    Suspend {
        /// Task passed to the runtime.
        task: program::Task,
        /// Waiter returned by the runtime.
        waiter: program::Waiter,
        /// Continuation passed to the runtime.
        continuation: program::Continuation,
    },
    /// Park one waiter on a task.
    Park {
        /// Task passed to the runtime.
        task: program::Task,
        /// Waiter passed to the runtime.
        waiter: program::Waiter,
    },
    /// Request task cancellation.
    CancelTask(program::Task),
    /// Query task cancellation.
    IsCancelled(program::Task),
    /// Detach one task result.
    Detach(program::Task),
    /// Finish one eager task.
    Finish {
        /// Task passed to the runtime.
        task: program::Task,
        /// Terminal task outcome.
        outcome: program::TaskOutcome,
    },
}

/// Strict runtime probe used by bytecode execution tests.
#[derive(Debug, Default)]
pub(crate) struct TestRuntime {
    /// Next task slot issued by this runtime.
    next_task_index: u32,
    /// Next waiter slot issued by this runtime.
    next_waiter_index: u32,
    /// Runtime calls in execution order.
    calls: Vec<RuntimeCall>,
}

impl TestRuntime {
    /// Return and clear recorded runtime calls.
    pub(crate) fn take_calls(&mut self) -> Vec<RuntimeCall> {
        mem::take(&mut self.calls)
    }
}

impl program::Runtime for TestRuntime {
    /// Record one waiter settlement.
    fn queue_waiter(
        &mut self,
        waiter: program::Waiter,
        value: program::Value,
    ) -> program::Result<bool> {
        self.calls.push(RuntimeCall::Queue { waiter, value });

        Ok(true)
    }

    /// Record one waiter cancellation.
    fn cancel_waiter(&mut self, waiter: program::Waiter) -> program::Result<bool> {
        self.calls.push(RuntimeCall::CancelWaiter(waiter));

        Ok(true)
    }

    /// Record one completed task creation.
    fn resolve_task(&mut self, value: program::Value) -> program::Task {
        let task = program::Task::new(self.next_task_index, 1);
        self.next_task_index += 1;
        self.calls.push(RuntimeCall::Resolve { task, value });

        task
    }

    /// Record one eager task creation.
    fn start_task(&mut self) -> program::Task {
        let task = program::Task::new(self.next_task_index, 1);
        self.next_task_index += 1;
        self.calls.push(RuntimeCall::Start(task));

        task
    }

    /// Record one task cancellation request.
    fn cancel_task(&mut self, task: program::Task) -> program::Result<()> {
        self.calls.push(RuntimeCall::CancelTask(task));

        Ok(())
    }

    /// Record one eager task suspension.
    fn suspend_task(
        &mut self,
        task: program::Task,
        continuation: program::Continuation,
    ) -> program::Result<program::Waiter> {
        let waiter = program::Waiter::new(self.next_waiter_index, 1);
        self.next_waiter_index += 1;
        self.calls.push(RuntimeCall::Suspend {
            task,
            waiter,
            continuation,
        });

        Ok(waiter)
    }

    /// Record one task result waiter.
    fn park_task(&mut self, task: program::Task, waiter: program::Waiter) -> program::Result<()> {
        self.calls.push(RuntimeCall::Park { task, waiter });

        Ok(())
    }

    /// Record one cancellation query.
    fn is_task_cancelled(&mut self, task: program::Task) -> program::Result<bool> {
        self.calls.push(RuntimeCall::IsCancelled(task));

        Ok(false)
    }

    /// Record one detached task result.
    fn detach_task(&mut self, task: program::Task) -> program::Result<()> {
        self.calls.push(RuntimeCall::Detach(task));

        Ok(())
    }

    /// Record one terminal task outcome.
    fn finish_task(
        &mut self,
        task: program::Task,
        outcome: program::TaskOutcome,
    ) -> program::Result<()> {
        self.calls.push(RuntimeCall::Finish { task, outcome });

        Ok(())
    }
}
