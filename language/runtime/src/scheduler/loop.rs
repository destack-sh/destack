use std::collections::VecDeque;

use parking_lot::Mutex;

use crate::diagnostic::RuntimeResult;
use crate::platform::{PlatformEvent, ResourceId};
use crate::scheduler::{Microtask, MicrotaskId, Task, TaskId, Timer, TimerQueue};

/// Event loop state for tasks, microtasks, and timers.
#[derive(Debug, Default)]
pub struct EventLoop {
    /// Pending macrotasks.
    pub tasks: VecDeque<Task>,
    /// Pending microtasks drained between tasks.
    pub microtasks: VecDeque<Microtask>,
    /// Pending external events.
    pub events: Vec<PlatformEvent>,
    /// Timer queue for scheduled callbacks.
    pub timers: Mutex<TimerQueue>,
    /// Next task identifier to issue.
    pub next_task_id: u64,
    /// Next microtask identifier to issue.
    pub next_microtask_id: u64,
}

impl EventLoop {
    /// Borrow the timer queue.
    pub fn timers(&self) -> &Mutex<TimerQueue> {
        &self.timers
    }

    /// Enqueue a macrotask for execution.
    pub fn enqueue_task(&mut self, task: Task) {
        self.tasks.push_back(task);
    }

    /// Enqueue a microtask for execution.
    pub fn enqueue_microtask(&mut self, microtask: Microtask) {
        self.microtasks.push_back(microtask);
    }

    /// Enqueue external events.
    pub fn enqueue_events(&mut self, events: Vec<PlatformEvent>) {
        self.events.extend(events);
    }

    /// Drain queued external events.
    pub fn drain_events(&mut self) -> Vec<PlatformEvent> {
        std::mem::take(&mut self.events)
    }

    /// Allocate the next task identifier.
    pub fn next_task_id(&mut self) -> TaskId {
        let id = TaskId::new(self.next_task_id);
        self.next_task_id = self.next_task_id.wrapping_add(1);
        id
    }

    /// Allocate the next microtask identifier.
    pub fn next_microtask_id(&mut self) -> MicrotaskId {
        let id = MicrotaskId::new(self.next_microtask_id);
        self.next_microtask_id = self.next_microtask_id.wrapping_add(1);
        id
    }

    /// Report whether any work remains in the event loop.
    pub fn has_pending_work(&self) -> bool {
        if !self.tasks.is_empty() || !self.microtasks.is_empty() || !self.events.is_empty() {
            return true;
        }

        let queue = self.timers.lock();
        !queue.timers.is_empty()
    }

    /// Schedule a timer in the runtime queue.
    pub fn schedule_timer(&self, timer: Timer) -> RuntimeResult<()> {
        let mut queue = self.timers.lock();
        queue.schedule(timer);
        Ok(())
    }

    /// Cancel a timer by handle.
    pub fn cancel_timer(&self, handle: ResourceId) -> RuntimeResult<()> {
        let mut queue = self.timers.lock();
        queue.cancel(handle);
        Ok(())
    }

    /// Drain timers that are ready at the given time.
    pub fn poll_timers(&self, now_nanos: u64) -> RuntimeResult<Vec<Timer>> {
        let mut queue = self.timers.lock();
        let ready = queue.poll_ready(now_nanos);
        Ok(ready)
    }
}
