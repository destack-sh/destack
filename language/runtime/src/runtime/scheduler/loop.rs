use std::collections::VecDeque;

use destack_workspace::SchedulerOptions;
use parking_lot::Mutex;

use super::{Microtask, MicrotaskId, Runnable, Task, TaskId, Timer, TimerQueue};
use crate::diagnostic::RuntimeResult;
use crate::platform::poller::{PlatformEventPayload, ProcessStatus};
use crate::platform::{PlatformEvent, PlatformEventSource, PlatformPoller, ResourceId};

/// Event loop for task queues, microtasks, timers, and platform events.
#[derive(Debug, Default)]
pub struct EventLoop {
    // NOTE #Incomplete: use queue priorities, budgets, and deterministic ordering rules
    /// Pending macrotasks.
    tasks: VecDeque<Task>,
    /// Pending microtasks that drain before macrotasks.
    microtasks: VecDeque<Microtask>,
    /// Pending platform events.
    events: VecDeque<PlatformEvent>,
    /// Ready timers waiting for dispatch.
    ready_timers: VecDeque<Timer>,
    /// Timer queue for scheduled callbacks.
    timers: Mutex<TimerQueue>,
    /// Next task identifier to issue.
    next_task_id: u64,
    /// Next microtask identifier to issue.
    next_microtask_id: u64,
    /// Next task queue sequence identifier to issue.
    next_sequence: u64,
    /// Configured event loop options.
    pub options: SchedulerOptions,
}

impl EventLoop {
    /// Configure event loop options.
    pub fn configure(&mut self, options: SchedulerOptions) {
        self.options = options;
    }

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
        let mut events = events;
        sort_platform_events(&mut events);
        self.events.extend(events);
    }

    /// Pop the next runnable item from the event loop.
    pub fn next_runnable(&mut self, now_nanos: u64) -> RuntimeResult<Option<Runnable>> {
        // always drain microtasks first
        if let Some(microtask) = self.microtasks.pop_front() {
            return Ok(Some(Runnable::Microtask(microtask)));
        }

        // move ready timers into the dispatch queue
        self.enqueue_ready_timers(now_nanos)?;
        if let Some(timer) = self.ready_timers.pop_front() {
            return Ok(Some(Runnable::Timer(timer)));
        }

        // dispatch external events before regular tasks
        if let Some(event) = self.events.pop_front() {
            return Ok(Some(Runnable::Event(event)));
        }

        Ok(self.tasks.pop_front().map(Runnable::Task))
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

    /// Allocate the next task queue sequence identifier.
    pub fn next_sequence(&mut self) -> u64 {
        let sequence = self.next_sequence;
        self.next_sequence = self.next_sequence.wrapping_add(1);
        sequence
    }

    /// Pop the next microtask if available.
    pub fn pop_microtask(&mut self) -> Option<Microtask> {
        self.microtasks.pop_front()
    }

    /// Report whether any microtasks are pending.
    pub fn has_microtasks(&self) -> bool {
        !self.microtasks.is_empty()
    }

    /// Report whether any work remains in the event loop.
    pub fn has_pending_work(&self) -> bool {
        if !self.tasks.is_empty()
            || !self.microtasks.is_empty()
            || !self.events.is_empty()
            || !self.ready_timers.is_empty()
        {
            return true;
        }

        let queue = self.timers.lock();
        queue.has_pending_timers()
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

    /// Enqueue ready timers from the timer queue.
    pub fn enqueue_ready_timers(&mut self, now_nanos: u64) -> RuntimeResult<()> {
        let ready = self.poll_timers(now_nanos)?;
        self.ready_timers.extend(ready);
        Ok(())
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
            self.enqueue_events(events);
        }

        Ok(count)
    }
}

/// Sort platform events into a deterministic order.
fn sort_platform_events(events: &mut [PlatformEvent]) {
    // ensure deterministic ordering for platform events
    events.sort_by_key(|event| {
        (
            source_order(event.source),
            event.token.0,
            event.resource_id.0,
            event.mask.0,
            event.flags.0,
            payload_sort_key(event.payload),
        )
    });
}

/// Map event sources into a deterministic ordering key.
fn source_order(source: PlatformEventSource) -> u8 {
    match source {
        PlatformEventSource::Io => 0,
        PlatformEventSource::Signal => 1,
        PlatformEventSource::Process => 2,
        PlatformEventSource::Timer => 3,
    }
}

/// Build an ordering key for event payload data.
fn payload_sort_key(payload: PlatformEventPayload) -> u64 {
    // pack event payload data into a deterministic ordering key
    match payload {
        PlatformEventPayload::Io { data } => data,
        PlatformEventPayload::Signal { signal } => signal as u64,
        PlatformEventPayload::Process { pid, status } => {
            let status_key = match status {
                ProcessStatus::Exited { code } => (0u64, code as u64),
                ProcessStatus::Signaled { signal, core_dump } => {
                    (1u64, (signal as u64) << 1 | core_dump as u64)
                }
                ProcessStatus::Stopped { signal } => (2u64, signal as u64),
                ProcessStatus::Continued => (3u64, 0),
            };
            ((pid as u64) << 32) | (status_key.0 << 16) | status_key.1
        }
        PlatformEventPayload::Timer { deadline_nanos } => deadline_nanos,
    }
}

#[cfg(test)]
mod tests {
    use destack_vm as vm;

    use super::{EventLoop, Runnable};
    use crate::runtime::engine::{EngineContinuation, NativeContinuation};
    use crate::runtime::scheduler::{Microtask, MicrotaskId, Task, TaskId, TaskState};

    /// Ensures microtasks run before macrotasks in the event loop.
    #[test]
    fn test_microtasks_run_first() {
        // set up an event loop with one task and one microtask
        let mut event_loop = EventLoop::default();

        let task = Task {
            id: TaskId::new(1),
            runnable: EngineContinuation::Native(NativeContinuation::new(11)),
            resume_value: vm::Value::VOID,
            state: TaskState::Ready,
            priority: 0,
        };
        let microtask = Microtask {
            id: MicrotaskId::new(1),
            runnable: EngineContinuation::Native(NativeContinuation::new(22)),
            resume_value: vm::Value::VOID,
            state: TaskState::Ready,
        };

        event_loop.enqueue_task(task);
        event_loop.enqueue_microtask(microtask);

        // microtasks should be dequeued first
        let first = event_loop.next_runnable(0).expect("event loop should run");
        assert!(matches!(first, Some(Runnable::Microtask(_))));

        // remaining item should be the task
        let second = event_loop.next_runnable(0).expect("event loop should run");
        assert!(matches!(second, Some(Runnable::Task(_))));
    }
}
