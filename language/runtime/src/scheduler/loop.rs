use std::collections::{BTreeMap, VecDeque};

use destack_memory::MemoryMap;
use destack_program as program;

use super::task::TaskTable;
use super::timer::TimerQueue;
use super::waiter::WaiterTable;
use super::{Callback, Invocation, Runnable, RunnableId, Wake, WakeKey};
use crate::diagnostic::{RuntimeError, RuntimeResult};

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
    /// Suspended language waiters.
    pub(super) waiters: WaiterTable,
    /// Eager asynchronous tasks.
    pub(super) task_table: TaskTable,
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

    /// Pop one value awaiting generated destruction.
    pub(crate) fn pop_drop(&mut self) -> Option<program::Value> {
        self.drops.pop()
    }

    /// Retain one released value until generated destruction can execute.
    pub(crate) fn release(&mut self, value: program::Value) {
        self.drops.push(value);
    }

    /// Clear scheduler state and release every suspended continuation.
    pub(crate) fn clear(&mut self, memory: &MemoryMap) -> RuntimeResult<()> {
        let tasks = std::mem::take(&mut self.tasks);
        let microtasks = std::mem::take(&mut self.microtasks);
        let waiters = std::mem::take(&mut self.waiters);

        // clear scheduler state before releasing its continuation ranges
        self.timers = TimerQueue::default();
        self.wakes.clear();
        self.wake_waiters.clear();
        self.task_table = TaskTable::default();
        self.drops.clear();
        self.next_runnable_id = 0;

        // release every queued and parked continuation even when one release fails
        let mut error = None;
        for runnable in tasks.into_iter().chain(microtasks) {
            if let Err(current) = runnable.release(memory)
                && error.is_none()
            {
                error = Some(current);
            }
        }
        if let Err(current) = waiters.release(memory).map_err(Box::<RuntimeError>::from)
            && error.is_none()
        {
            error = Some(current);
        }

        if let Some(error) = error {
            return Err(error);
        }

        Ok(())
    }

    /// Identify one function invocation for queue execution.
    fn identify(&mut self, invocation: Invocation) -> Runnable {
        let id = RunnableId::new(self.next_runnable_id);
        self.next_runnable_id += 1;

        Runnable::new(id, invocation)
    }
}
