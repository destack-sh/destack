use rustc_hash::FxHashMap;
use std::collections::VecDeque;

use super::timer::TimerQueue;
use super::{Microtask, MicrotaskId, Task, TaskId, Waiter, Wake, WakeKey};
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::HostEvent;
use crate::host::poller::PollerEvent;

/// Event loop for tasks, microtasks, timers, waiters, and wakes.
#[derive(Debug, Default)]
pub struct EventLoop {
    /// Pending macrotasks.
    pub(super) tasks: VecDeque<Task>,
    /// Pending microtasks that drain before macrotasks.
    pub(super) microtasks: VecDeque<Microtask>,
    /// Pending external wakes.
    pub(super) wakes: VecDeque<Wake>,
    /// Timer queue for scheduled timer fires.
    pub(super) timers: TimerQueue,
    /// Suspended continuations keyed by their wake source.
    pub(super) waiters: FxHashMap<WakeKey, Waiter>,

    /// Next task identifier to issue.
    pub(super) next_task_id: u64,
    /// Next microtask identifier to issue.
    pub(super) next_microtask_id: u64,
}

impl EventLoop {
    /// Enqueue a macrotask for execution.
    pub fn enqueue_task(&mut self, task: Task) {
        // insert higher priority tasks ahead of lower priority tasks
        let insert_at = self
            .tasks
            .iter()
            .position(|queued| queued.priority < task.priority)
            .unwrap_or(self.tasks.len());
        self.tasks.insert(insert_at, task);
    }

    /// Enqueue a microtask for execution.
    pub fn enqueue_microtask(&mut self, microtask: Microtask) {
        self.microtasks.push_back(microtask);
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

    /// Allocate the next task identifier.
    pub fn next_task_id(&mut self) -> RuntimeResult<TaskId> {
        let id = TaskId::new(self.next_task_id);
        self.next_task_id = self.next_task_id.checked_add(1).ok_or_else(|| {
            RuntimeError::Internal {
                message: "event loop task identifier space exhausted".to_string(),
            }
            .boxed()
        })?;

        Ok(id)
    }

    /// Allocate the next microtask identifier.
    pub fn next_microtask_id(&mut self) -> RuntimeResult<MicrotaskId> {
        let id = MicrotaskId::new(self.next_microtask_id);
        self.next_microtask_id = self.next_microtask_id.checked_add(1).ok_or_else(|| {
            RuntimeError::Internal {
                message: "event loop microtask identifier space exhausted".to_string(),
            }
            .boxed()
        })?;

        Ok(id)
    }

    /// Pop the next microtask if available.
    pub fn pop_microtask(&mut self) -> Option<Microtask> {
        self.microtasks.pop_front()
    }

    /// Pop the next macrotask if available.
    pub fn pop_task(&mut self) -> Option<Task> {
        self.tasks.pop_front()
    }

    /// Report whether any microtasks are pending.
    pub fn has_microtasks(&self) -> bool {
        !self.microtasks.is_empty()
    }

    /// Return whether this event loop currently retains any queued or suspended work.
    pub(super) fn is_quiescent(&self) -> bool {
        if !self.tasks.is_empty() || !self.microtasks.is_empty() || !self.wakes.is_empty() {
            return false;
        }

        if self.timers.has_pending_timers() {
            return false;
        }

        self.waiters.is_empty()
    }
}
