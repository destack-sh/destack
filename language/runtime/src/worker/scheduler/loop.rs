use std::collections::{BTreeMap, VecDeque};

use destack_program as program;

use super::task::TaskTable;
use super::timer::TimerQueue;
use super::waiter::WaiterTable;
use super::{Callback, Invocation, Runnable, RunnableId, Wake, WakeKey};
use crate::host::HostEvent;
use crate::host::poller::PollerEvent;

/// Event loop for tasks, microtasks, timers, waiters, and wakes.
#[derive(Debug, Default)]
pub struct EventLoop {
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
    pub fn enqueue_task(&mut self, invocation: Invocation) -> RunnableId {
        let runnable = self.identify(invocation);
        let id = runnable.id;
        self.tasks.push_back(runnable);

        id
    }

    /// Enqueue a microtask for execution.
    pub fn enqueue_microtask(&mut self, invocation: Invocation) -> RunnableId {
        let runnable = self.identify(invocation);
        let id = runnable.id;
        self.microtasks.push_back(runnable);

        id
    }

    /// Enqueue one wake.
    pub fn enqueue_wake(&mut self, wake: Wake) {
        self.wakes.push_back(wake);
    }

    /// Enqueue poller wakes.
    pub fn enqueue_poller_wakes(&mut self, events: Vec<PollerEvent>) {
        let mut events = events;
        self.sort_poller_wakes(&mut events);
        self.wakes.extend(
            events
                .into_iter()
                .map(super::ResourceWake::poller)
                .map(Wake::Resource),
        );
    }

    /// Enqueue host wakes.
    pub fn enqueue_host_wakes(&mut self, events: Vec<HostEvent>) {
        self.wakes
            .extend(events.into_iter().map(super::HostWake::new).map(Wake::Host));
    }

    /// Pop the next microtask if available.
    pub fn pop_microtask(&mut self) -> Option<Runnable> {
        self.microtasks.pop_front()
    }

    /// Pop the next task if available.
    pub fn pop_task(&mut self) -> Option<Runnable> {
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

    /// Report whether any microtasks are pending.
    pub fn has_microtasks(&self) -> bool {
        !self.microtasks.is_empty()
    }

    /// Identify one function invocation for queue execution.
    fn identify(&mut self, invocation: Invocation) -> Runnable {
        let id = RunnableId::new(self.next_runnable_id);
        self.next_runnable_id += 1;

        Runnable::new(id, invocation)
    }
}
