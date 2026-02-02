use super::{Microtask, MicrotaskId, Task, TaskId, Timer, TimerQueue};
use crate::diagnostic::RuntimeResult;
use crate::platform::{PlatformEvent, PlatformPoller, ResourceId};
use crate::scheduler::EventLoop;
use parking_lot::Mutex;

/// Runnable item returned by the scheduler.
#[derive(Debug)]
pub enum ScheduledItem {
    /// A macrotask selected for execution.
    Task(Task),
    /// A microtask selected for execution.
    Microtask(Microtask),
}

/// Scheduler for task queues and event loops.
#[derive(Debug, Default)]
pub struct Scheduler {
    /// Event loop queues managed by the scheduler.
    pub event_loop: EventLoop,
}

impl Scheduler {
    /// Borrow the timer queue.
    pub fn timers(&self) -> &Mutex<TimerQueue> {
        self.event_loop.timers()
    }

    /// Enqueue a macrotask for execution.
    pub fn enqueue_task(&mut self, task: Task) {
        self.event_loop.enqueue_task(task);
    }

    /// Enqueue a microtask for execution.
    pub fn enqueue_microtask(&mut self, microtask: Microtask) {
        self.event_loop.enqueue_microtask(microtask);
    }

    /// Enqueue external events.
    pub fn enqueue_events(&mut self, events: Vec<PlatformEvent>) {
        self.event_loop.enqueue_events(events);
    }

    /// Pop the next runnable item from the scheduler.
    pub fn next_runnable(&mut self) -> Option<ScheduledItem> {
        if let Some(microtask) = self.event_loop.microtasks.pop_front() {
            return Some(ScheduledItem::Microtask(microtask));
        }

        self.event_loop.tasks.pop_front().map(ScheduledItem::Task)
    }

    /// Allocate the next task identifier.
    pub fn next_task_id(&mut self) -> TaskId {
        self.event_loop.next_task_id()
    }

    /// Allocate the next microtask identifier.
    pub fn next_microtask_id(&mut self) -> MicrotaskId {
        self.event_loop.next_microtask_id()
    }

    /// Report whether any work remains in the scheduler.
    pub fn has_pending_work(&self) -> bool {
        self.event_loop.has_pending_work()
    }

    /// Schedule a timer in the runtime queue.
    pub fn schedule_timer(&self, timer: Timer) -> RuntimeResult<()> {
        self.event_loop.schedule_timer(timer)
    }

    /// Cancel a timer by handle.
    pub fn cancel_timer(&self, handle: ResourceId) -> RuntimeResult<()> {
        self.event_loop.cancel_timer(handle)
    }

    /// Drain timers that are ready at the given time.
    pub fn poll_timers(&self, now_nanos: u64) -> RuntimeResult<Vec<Timer>> {
        self.event_loop.poll_timers(now_nanos)
    }

    /// Poll the platform poller and enqueue events.
    pub fn poll_poller(
        &mut self,
        poller: &mut dyn PlatformPoller,
        timeout_nanos: Option<u64>,
    ) -> RuntimeResult<usize> {
        let events = poller.poll(timeout_nanos)?;
        let count = events.len();
        if count > 0 {
            self.event_loop.enqueue_events(events);
        }

        Ok(count)
    }
}
