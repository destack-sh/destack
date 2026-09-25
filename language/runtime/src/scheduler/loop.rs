use std::collections::{BTreeMap, VecDeque};

use tspp_program as program;
use tspp_vm as vm;

use super::fiber::FiberTable;
use super::timer::TimerQueue;
use super::{Callback, Invocation, Runnable, RunnableId, Wake, WakeKey};
use crate::diagnostic::RuntimeResult;

/// Event loop for tasks, microtasks, timers, waiters, and wakes.
#[derive(Debug, Default)]
pub(crate) struct EventLoop {
    /// Pending tasks.
    pub(super) tasks: VecDeque<Runnable>,
    /// Pending microtasks that drain before tasks.
    pub(super) microtasks: VecDeque<Runnable>,
    /// Timer queue for scheduled timer fires.
    pub(super) timers: TimerQueue,
    /// Pending external wakes.
    pub(super) wakes: VecDeque<Wake>,
    /// Repeatable callbacks keyed by their external wake source.
    pub(super) wake_waiters: BTreeMap<WakeKey, Callback>,
    /// Live fibers with their parked executions.
    pub(super) fibers: FiberTable,
    /// Values released by scheduler transitions and awaiting destruction.
    pub(super) drops: Vec<program::Value>,
    /// Next runnable identifier to issue.
    pub(super) next_runnable_id: u64,
}

impl EventLoop {
    /// Enqueue a task for execution.
    pub(crate) fn enqueue_task(&mut self, invocation: Invocation) -> RunnableId {
        let runnable = self.identify(invocation);
        let id = runnable.id;
        self.tasks.push_back(runnable);

        id
    }

    /// Enqueue a microtask for execution.
    pub(crate) fn enqueue_microtask(&mut self, invocation: Invocation) -> RunnableId {
        let runnable = self.identify(invocation);
        let id = runnable.id;
        self.microtasks.push_back(runnable);

        id
    }

    /// Enqueue one wake.
    pub(crate) fn enqueue_wake(&mut self, wake: Wake) {
        self.wakes.push_back(wake);
    }

    /// Pop the next microtask if available.
    pub(crate) fn pop_microtask(&mut self) -> Option<Runnable> {
        self.microtasks.pop_front()
    }

    /// Pop the next task if available.
    pub(crate) fn pop_task(&mut self) -> Option<Runnable> {
        self.tasks.pop_front()
    }

    /// Insert one running fiber and return its identity.
    pub(crate) fn insert_fiber(&mut self) -> program::FiberId {
        self.fibers.insert()
    }

    /// Park one running fiber with its retained execution.
    pub(crate) fn park_fiber(
        &mut self,
        fiber_id: program::FiberId,
        execution: vm::Fiber,
    ) -> RuntimeResult<()> {
        // a wake that raced the park settles the fiber immediately
        if let Some(value) = self.fibers.park(fiber_id, execution)? {
            self.enqueue_microtask(Invocation::wake(fiber_id, value));
        }

        Ok(())
    }

    /// Take one wake buffered before the running fiber parked.
    pub(crate) fn take_pending_wake(
        &mut self,
        fiber_id: program::FiberId,
    ) -> RuntimeResult<Option<program::Value>> {
        self.fibers.take_pending(fiber_id)
    }

    /// Deliver one wake, buffering it until the target fiber parks.
    pub(crate) fn wake_fiber(
        &mut self,
        fiber_id: program::FiberId,
        value: program::Value,
    ) -> RuntimeResult<()> {
        if let Some(value) = self.fibers.wake(fiber_id, value)? {
            self.enqueue_microtask(Invocation::wake(fiber_id, value));
        }

        Ok(())
    }

    /// Take one woken fiber's execution for resumption.
    pub(crate) fn resume_fiber(&mut self, fiber_id: program::FiberId) -> RuntimeResult<vm::Fiber> {
        self.fibers.resume(fiber_id)
    }

    /// Remove one completed fiber and release any undelivered wake.
    pub(crate) fn retire_fiber(&mut self, fiber_id: program::FiberId) -> RuntimeResult<()> {
        if let Some(pending) = self.fibers.remove(fiber_id)? {
            self.release(pending);
        }

        Ok(())
    }

    /// Iterate every parked or ready fiber execution.
    pub(crate) fn executions_mut(&mut self) -> impl Iterator<Item = &mut vm::Fiber> {
        self.fibers.executions_mut()
    }

    /// Pop one value awaiting generated destruction.
    pub(crate) fn pop_drop(&mut self) -> Option<program::Value> {
        self.drops.pop()
    }

    /// Retain one released value until generated destruction can execute.
    pub(crate) fn release(&mut self, value: program::Value) {
        self.drops.push(value);
    }

    /// Clear scheduler state and release every queued value.
    pub(crate) fn clear(&mut self) -> Vec<program::Value> {
        let tasks = std::mem::take(&mut self.tasks);
        let microtasks = std::mem::take(&mut self.microtasks);

        // clear scheduler state before draining its queued values
        self.timers = TimerQueue::default();
        self.wakes.clear();
        self.wake_waiters.clear();
        self.fibers = FiberTable::default();
        self.next_runnable_id = 0;

        // release queued wake values with any values already awaiting destruction
        let mut released = std::mem::take(&mut self.drops);
        released.extend(
            tasks
                .into_iter()
                .chain(microtasks)
                .filter_map(Runnable::release),
        );

        released
    }

    /// Identify one function invocation for queue execution.
    fn identify(&mut self, invocation: Invocation) -> Runnable {
        let id = RunnableId::new(self.next_runnable_id);
        self.next_runnable_id += 1;

        Runnable::new(id, invocation)
    }
}
