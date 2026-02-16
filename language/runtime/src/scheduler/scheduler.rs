use super::{Microtask, MicrotaskId, Task, TaskId, Timer, TimerQueue};
use crate::diagnostic::RuntimeResult;
use crate::platform::{PlatformEvent, PlatformPoller, ResourceId};
use crate::scheduler::EventLoop;
use destack_workspace::SchedulerOptions;
use parking_lot::Mutex;

/// Runnable item returned by the scheduler.
#[derive(Debug)]
pub enum Runnable {
    /// A macrotask selected for execution.
    Task(Task),
    /// A microtask selected for execution.
    Microtask(Microtask),
    /// A timer ready to fire.
    Timer(Timer),
    /// An external platform event.
    Event(PlatformEvent),
}

/// Scheduler for task queues and event loops.
#[derive(Debug, Default)]
pub struct Scheduler {
    // NOTE #Incomplete: use task priorities, queue budgets, and deterministic ordering rules
    /// Event loop queues managed by the scheduler.
    pub event_loop: EventLoop,
    /// Configured scheduler options.
    pub options: SchedulerOptions,
}

impl Scheduler {
    /// Configure scheduler options.
    pub fn configure(&mut self, options: SchedulerOptions) {
        self.options = options;
    }

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
    pub fn next_runnable(&mut self, now_nanos: u64) -> RuntimeResult<Option<Runnable>> {
        // always drain microtasks first
        if let Some(microtask) = self.event_loop.microtasks.pop_front() {
            return Ok(Some(Runnable::Microtask(microtask)));
        }

        // move ready timers into the dispatch queue
        self.event_loop.enqueue_ready_timers(now_nanos)?;
        if let Some(timer) = self.event_loop.ready_timers.pop_front() {
            return Ok(Some(Runnable::Timer(timer)));
        }

        // dispatch external events before regular tasks
        if let Some(event) = self.event_loop.events.pop_front() {
            return Ok(Some(Runnable::Event(event)));
        }

        Ok(self.event_loop.tasks.pop_front().map(Runnable::Task))
    }

    /// Allocate the next task identifier.
    pub fn next_task_id(&mut self) -> TaskId {
        self.event_loop.next_task_id()
    }

    /// Allocate the next microtask identifier.
    pub fn next_microtask_id(&mut self) -> MicrotaskId {
        self.event_loop.next_microtask_id()
    }

    /// Allocate the next scheduler sequence identifier.
    pub fn next_sequence(&mut self) -> u64 {
        self.event_loop.next_sequence()
    }

    /// Pop the next microtask if available.
    pub fn pop_microtask(&mut self) -> Option<Microtask> {
        self.event_loop.pop_microtask()
    }

    /// Report whether any microtasks are pending.
    pub fn has_microtasks(&self) -> bool {
        !self.event_loop.microtasks.is_empty()
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

#[cfg(test)]
mod tests {
    use destack_vm as vm;

    use super::{Runnable, Scheduler};
    use crate::scheduler::{
        Microtask, MicrotaskId, NativeContinuation, PlatformRunnable, Task, TaskId, TaskState,
    };

    /// Ensures microtasks run before macrotasks in the scheduler.
    #[test]
    fn test_microtasks_run_first() {
        // set up a scheduler with one task and one microtask
        let mut scheduler = Scheduler::default();

        let task = Task {
            id: TaskId::new(1),
            runnable: PlatformRunnable::Native(NativeContinuation::new(11)),
            resume_value: vm::Value::VOID,
            state: TaskState::Ready,
            priority: 0,
        };
        let microtask = Microtask {
            id: MicrotaskId::new(1),
            runnable: PlatformRunnable::Native(NativeContinuation::new(22)),
            resume_value: vm::Value::VOID,
            state: TaskState::Ready,
        };

        scheduler.enqueue_task(task);
        scheduler.enqueue_microtask(microtask);

        // microtasks should be dequeued first
        let first = scheduler.next_runnable(0).expect("scheduler should run");
        assert!(matches!(first, Some(Runnable::Microtask(_))));

        // remaining item should be the task
        let second = scheduler.next_runnable(0).expect("scheduler should run");
        assert!(matches!(second, Some(Runnable::Task(_))));
    }
}
