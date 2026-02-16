use std::collections::VecDeque;

use parking_lot::Mutex;

use crate::diagnostic::RuntimeResult;
use crate::platform::poller::{PlatformEventPayload, ProcessStatus};
use crate::platform::{PlatformEvent, PlatformEventSource, ResourceId};
use crate::scheduler::{Microtask, MicrotaskId, Task, TaskId, Timer, TimerQueue};

/// Event loop state for tasks, microtasks, and timers.
#[derive(Debug, Default)]
pub struct EventLoop {
    // NOTE #Incomplete: enforce queue priorities, budgets, and deterministic ordering
    /// Pending macrotasks.
    pub tasks: VecDeque<Task>,
    /// Pending microtasks (drained between tasks).
    pub microtasks: VecDeque<Microtask>,
    /// Pending external events.
    pub events: VecDeque<PlatformEvent>,
    /// Ready timers waiting for dispatch.
    pub ready_timers: VecDeque<Timer>,
    /// Timer queue for scheduled callbacks.
    pub timers: Mutex<TimerQueue>,
    /// Next task identifier to issue.
    pub next_task_id: u64,
    /// Next microtask identifier to issue.
    pub next_microtask_id: u64,
    /// Next scheduler sequence identifier to issue.
    pub next_sequence: u64,
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
        let mut events = events;
        sort_platform_events(&mut events);
        self.events.extend(events);
    }

    /// Drain queued external events.
    pub fn drain_events(&mut self) -> VecDeque<PlatformEvent> {
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

    /// Allocate the next scheduler sequence identifier.
    pub fn next_sequence(&mut self) -> u64 {
        let sequence = self.next_sequence;
        self.next_sequence = self.next_sequence.wrapping_add(1);
        sequence
    }

    /// Pop the next microtask if available.
    pub fn pop_microtask(&mut self) -> Option<Microtask> {
        self.microtasks.pop_front()
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
