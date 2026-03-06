use std::collections::VecDeque;

use destack_heap as heap;
use destack_workspace::{SchedulerOptions, SchedulerPolicy};
use parking_lot::Mutex;
use rustc_hash::{FxHashMap, FxHashSet};

use super::{Microtask, MicrotaskId, Runnable, Task, TaskId, Timer, TimerQueue};
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::{HostEvent, HostEventKind};
use crate::platform::{PlatformError, ResourceId};
use crate::runtime::engine::EngineContinuation;
use crate::runtime::poller::{
    HostPoller, PollerEvent, PollerEventPayload, PollerEventSource, PollerProcessStatus,
    PollerToken,
};
use crate::runtime::time::{Nanos, WorldInstant};
use crate::runtime::{DropCounts, DropReason};

/// Default host semantic dispatch batch size before forcing one poller event.
const DEFAULT_HOST_EVENT_BUDGET: u64 = 32;

/// Watch payload that can be dispatched as one event loop task.
#[derive(Debug)]
pub struct EventLoopWatch {
    /// Runnable continuation to execute when dispatched.
    pub runnable: EngineContinuation,
    /// Resume value passed into the continuation.
    pub resume_value: heap::Value,
    /// Task priority used when queueing watched tasks.
    pub priority: u8,
}

/// Event loop for task queues, microtasks, timers, and platform events.
#[derive(Debug, Default)]
pub struct EventLoop {
    /// Pending macrotasks.
    tasks: VecDeque<Task>,
    /// Pending microtasks that drain before macrotasks.
    microtasks: VecDeque<Microtask>,
    /// Pending platform events.
    events: VecDeque<PollerEvent>,
    /// Pending host semantic events.
    host_events: VecDeque<HostEvent>,
    /// Ready timers waiting for dispatch.
    ready_timers: VecDeque<Timer>,
    /// Timer queue for scheduled timer fires.
    timers: Mutex<TimerQueue>,
    /// Timer handles canceled after scheduling and before dispatch.
    canceled_timers: Mutex<FxHashSet<ResourceId>>,

    /// Timer watch dispatch table keyed by timer handle.
    timer_watches: FxHashMap<ResourceId, EventLoopWatch>,
    /// External event watch dispatch table keyed by poller token.
    poller_event_watches: FxHashMap<PollerToken, EventLoopWatch>,
    /// Host event watch dispatch table keyed by host event kind.
    host_event_watches: FxHashMap<HostEventKind, EventLoopWatch>,
    /// Configured event loop options.
    options: SchedulerOptions,

    /// Next task identifier to issue.
    next_task_id: u64,
    /// Next microtask identifier to issue.
    next_microtask_id: u64,
    /// Next task queue sequence identifier to issue.
    next_sequence: u64,
    /// Drop accounting at the event-loop boundary.
    drop_counts: DropCounts,
    /// Number of host semantic events dispatched since the last poller event.
    host_events_since_poller: u64,
}

impl EventLoop {
    /// Configure event loop options.
    pub fn configure(&mut self, options: SchedulerOptions) -> RuntimeResult<()> {
        // validate options before applying them
        validate_scheduler_options(&options)?;
        self.options = options;

        Ok(())
    }

    /// Borrow the configured scheduler options.
    pub fn options(&self) -> &SchedulerOptions {
        &self.options
    }

    /// Borrow the timer queue.
    pub fn timers(&self) -> &Mutex<TimerQueue> {
        &self.timers
    }

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

    /// Enqueue external events.
    pub fn enqueue_events(&mut self, events: Vec<PollerEvent>) {
        let mut events = events;
        sort_platform_events(&mut events);
        self.events.extend(events);
    }

    /// Enqueue host semantic events.
    pub fn enqueue_host_events(&mut self, events: Vec<HostEvent>) {
        self.host_events.extend(events);
    }

    /// Pop the next runnable item from the event loop.
    pub fn next_runnable(
        &mut self,
        wall_now_nanos: u64,
        mono_now_nanos: u64,
    ) -> RuntimeResult<Option<Runnable>> {
        // always drain microtasks first
        if let Some(microtask) = self.microtasks.pop_front() {
            return Ok(Some(Runnable::Microtask(microtask)));
        }

        // move due timers into the dispatch queue
        self.enqueue_due_timers(Nanos::new(wall_now_nanos), Nanos::new(mono_now_nanos))?;
        while let Some(timer) = self.ready_timers.pop_front() {
            // drop canceled timers that were already promoted into the ready queue
            if self.canceled_timers.lock().remove(&timer.handle) {
                continue;
            }

            return Ok(Some(Runnable::Timer(timer)));
        }

        // dispatch host semantic events first while under the fairness budget
        if should_dispatch_host_event_first(
            &self.host_events,
            &self.events,
            self.host_events_since_poller,
            self.options.host_event_budget,
        ) && let Some(host_event) = self.host_events.pop_front()
        {
            self.host_events_since_poller = self.host_events_since_poller.saturating_add(1);
            return Ok(Some(Runnable::HostEvent(host_event)));
        }

        // dispatch one poller event and reset host fairness streak
        if let Some(event) = self.events.pop_front() {
            self.host_events_since_poller = 0;
            return Ok(Some(Runnable::PollerEvent(event)));
        }

        // dispatch remaining host semantic events when no poller event is pending
        if let Some(host_event) = self.host_events.pop_front() {
            self.host_events_since_poller = self.host_events_since_poller.saturating_add(1);
            return Ok(Some(Runnable::HostEvent(host_event)));
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

    /// Pop the next macrotask if available.
    pub fn pop_task(&mut self) -> Option<Task> {
        self.tasks.pop_front()
    }

    /// Report whether any microtasks are pending.
    pub fn has_microtasks(&self) -> bool {
        !self.microtasks.is_empty()
    }

    /// Report whether one runnable item is already ready without advancing time.
    pub fn has_runnable_work(&self) -> bool {
        !self.tasks.is_empty()
            || !self.microtasks.is_empty()
            || !self.events.is_empty()
            || !self.host_events.is_empty()
            || !self.ready_timers.is_empty()
    }

    /// Report whether any work remains in the event loop.
    pub fn has_pending_work(&self) -> bool {
        if self.has_runnable_work()
            || !self.poller_event_watches.is_empty()
            || !self.host_event_watches.is_empty()
        {
            return true;
        }

        let queue = self.timers.lock();
        queue.has_pending_timers()
    }

    /// Schedule a timer in the runtime queue.
    pub fn schedule_timer(&self, timer: Timer) -> RuntimeResult<()> {
        // normalize timer deadlines so scheduling stays deterministic
        let deadline = super::TimerDeadline {
            clock: timer.deadline.clock,
            at: self.normalize_deadline(timer.deadline.at),
        };
        let interval = timer.interval.map(|interval| {
            if interval.get() <= 1 {
                return interval;
            }

            let interval = self.normalize_deadline(interval);
            if interval.get() == 0 {
                return Nanos::new(1);
            }

            interval
        });
        let timer = Timer {
            handle: timer.handle,
            deadline,
            interval,
        };

        self.canceled_timers.lock().remove(&timer.handle);
        let mut queue = self.timers.lock();
        queue.schedule(timer);
        Ok(())
    }

    /// Cancel a timer by handle.
    pub fn cancel_timer(&self, handle: ResourceId) -> RuntimeResult<()> {
        let mut queue = self.timers.lock();
        queue.cancel(handle);
        self.canceled_timers.lock().insert(handle);
        Ok(())
    }

    /// Register one timer watch.
    pub fn watch_timer(&mut self, handle: ResourceId, watch: EventLoopWatch) -> RuntimeResult<()> {
        // only native continuations can be cloned for repeated dispatch
        validate_watch(&watch)?;
        self.timer_watches.insert(handle, watch);

        Ok(())
    }

    /// Remove the timer watch registered for one timer handle.
    pub fn unwatch_timer(&mut self, handle: ResourceId) -> Option<EventLoopWatch> {
        self.timer_watches.remove(&handle)
    }

    /// Register one event watch.
    pub fn watch_event(&mut self, token: PollerToken, watch: EventLoopWatch) -> RuntimeResult<()> {
        // only native continuations can be cloned for repeated dispatch
        validate_watch(&watch)?;
        self.poller_event_watches.insert(token, watch);

        Ok(())
    }

    /// Remove the event watch registered for one poller token.
    pub fn unwatch_event(&mut self, token: PollerToken) -> Option<EventLoopWatch> {
        self.poller_event_watches.remove(&token)
    }

    /// Return whether one event watch is registered for the given token.
    pub fn watches_event(&self, token: PollerToken) -> bool {
        self.poller_event_watches.contains_key(&token)
    }

    /// Register one host semantic event watch.
    pub fn watch_host_event(
        &mut self,
        kind: HostEventKind,
        watch: EventLoopWatch,
    ) -> RuntimeResult<()> {
        // only native continuations can be cloned for repeated dispatch
        validate_watch(&watch)?;
        self.host_event_watches.insert(kind, watch);

        Ok(())
    }

    /// Remove the host semantic event watch registered for one kind.
    pub fn unwatch_host_event(&mut self, kind: HostEventKind) -> Option<EventLoopWatch> {
        self.host_event_watches.remove(&kind)
    }

    /// Return whether one host semantic watch is registered for the given kind.
    pub fn watches_host_event(&self, kind: HostEventKind) -> bool {
        self.host_event_watches.contains_key(&kind)
    }

    /// Build one task for a fired timer watch.
    pub fn task_for_timer(&mut self, timer: Timer) -> Option<Task> {
        let watch = self.timer_watches.get(&timer.handle)?;
        let EngineContinuation::Native(native) = watch.runnable else {
            return None;
        };
        let watch = EventLoopWatch {
            runnable: EngineContinuation::Native(native),
            resume_value: watch.resume_value,
            priority: watch.priority,
        };

        Some(self.task_for_watch(watch))
    }

    /// Build one task for one external event watch.
    pub fn task_for_event(&mut self, event: PollerEvent) -> Option<Task> {
        let watch = self.poller_event_watches.get(&event.token)?;
        let EngineContinuation::Native(native) = watch.runnable else {
            return None;
        };
        let watch = EventLoopWatch {
            runnable: EngineContinuation::Native(native),
            resume_value: watch.resume_value,
            priority: watch.priority,
        };

        Some(self.task_for_watch(watch))
    }

    /// Build one task for one host semantic event watch.
    pub fn task_for_host_event(&mut self, event: HostEvent) -> Option<Task> {
        let kind = event.kind()?;
        let watch = self.host_event_watches.get(&kind)?;
        let EngineContinuation::Native(native) = watch.runnable else {
            return None;
        };
        let watch = EventLoopWatch {
            runnable: EngineContinuation::Native(native),
            resume_value: watch.resume_value,
            priority: watch.priority,
        };

        Some(self.task_for_watch(watch))
    }

    /// Drain timers that are ready at the given time.
    pub fn poll_timers(&self, wall_now: Nanos, mono_now: Nanos) -> RuntimeResult<Vec<Timer>> {
        let mut queue = self.timers.lock();
        let ready = queue.poll_ready(wall_now, mono_now);
        Ok(ready)
    }

    /// Enqueue timers that are due at the current wall and monotonic timestamps.
    pub fn enqueue_due_timers(&mut self, wall_now: Nanos, mono_now: Nanos) -> RuntimeResult<()> {
        let ready = self.poll_timers(wall_now, mono_now)?;
        self.ready_timers.extend(ready);
        Ok(())
    }

    /// Poll the platform poller and enqueue events.
    pub fn poll_poller(
        &mut self,
        poller: &mut dyn HostPoller,
        timeout_nanos: Option<u64>,
    ) -> RuntimeResult<usize> {
        let events = poller.poll(timeout_nanos)?;
        let count = events.len();
        if count > 0 {
            self.enqueue_events(events);
        }

        Ok(count)
    }

    /// Return the next wall deadline when any timer can become runnable.
    pub fn next_deadline(&self, wall_now: Nanos, mono_now: Nanos) -> Option<WorldInstant> {
        // ready timers are runnable immediately
        if !self.ready_timers.is_empty() {
            return Some(WorldInstant::from_nanos(wall_now));
        }

        // map wall and monotonic timer deadlines into one wall-clock wakeup
        let mut queue = self.timers.lock();
        let (wall_deadline, mono_deadline) = queue.next_deadlines();
        let wall_deadline = wall_deadline
            .filter(|deadline| *deadline > wall_now)
            .map(WorldInstant::from_nanos);
        let mono_deadline = mono_deadline
            .filter(|deadline| *deadline > mono_now)
            .map(|deadline| {
                let delta = deadline.saturating_sub(mono_now);
                WorldInstant::from_nanos(wall_now.saturating_add(delta))
            });

        match (wall_deadline, mono_deadline) {
            (Some(wall_deadline), Some(mono_deadline)) => Some(wall_deadline.min(mono_deadline)),
            (Some(wall_deadline), None) => Some(wall_deadline),
            (None, Some(mono_deadline)) => Some(mono_deadline),
            (None, None) => None,
        }
    }

    /// Return the timeout until the next timer is ready in any clock domain.
    pub fn timeout_until_next_timer(&self, wall_now: Nanos, mono_now: Nanos) -> Option<Nanos> {
        self.next_deadline(wall_now, mono_now)
            .map(|deadline| deadline.saturating_sub(WorldInstant::from_nanos(wall_now)))
    }

    /// Record one or more drops observed by the event loop.
    pub fn record_drop(&mut self, reason: DropReason, count: u64) {
        self.drop_counts.record(reason, count);
    }

    /// Return drop accounting observed by the event loop.
    pub const fn drop_counts(&self) -> DropCounts {
        self.drop_counts
    }

    /// Build one task from one watch payload.
    fn task_for_watch(&mut self, watch: EventLoopWatch) -> Task {
        let task_id = self.next_task_id();
        Task {
            id: task_id,
            runnable: watch.runnable,
            resume_value: watch.resume_value,
            status: super::TaskStatus::Ready,
            priority: watch.priority,
        }
    }

    /// Normalize one timer deadline using scheduler options.
    fn normalize_deadline(&self, deadline: Nanos) -> Nanos {
        // quantize to timer resolution first
        let mut normalized = deadline;
        if let Some(timer_resolution_ns) = self.options.timer_resolution_ns {
            normalized = round_up_deadline(normalized, Nanos::new(timer_resolution_ns));
        }

        // then quantize to the configured coalescing window
        if let Some(max_timer_coalesce_ns) = self.options.max_timer_coalesce_ns {
            normalized = round_up_deadline(normalized, Nanos::new(max_timer_coalesce_ns));
        }

        normalized
    }
}

/// Round one deadline up to one deterministic quantum.
fn round_up_deadline(deadline: Nanos, quantum: Nanos) -> Nanos {
    if quantum.get() <= 1 {
        return deadline;
    }

    let remainder = deadline.get() % quantum.get();
    if remainder == 0 {
        return deadline;
    }

    deadline.saturating_add(Nanos::new(quantum.get().saturating_sub(remainder)))
}

/// Validate one scheduler options payload.
fn validate_scheduler_options(options: &SchedulerOptions) -> RuntimeResult<()> {
    // enforce the currently implemented queue policy
    if options.policy != SchedulerPolicy::Fifo {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "options.scheduler.policy",
            "only fifo scheduling policy is currently supported",
        ))
        .boxed());
    }

    // reject unsupported pool options until worker pools land
    if options.worker_threads.is_some()
        || options.io_threads.is_some()
        || options.blocking_threads.is_some()
        || options.max_tasks.is_some()
    {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "options.scheduler",
            "worker and pool options are not implemented yet",
        ))
        .boxed());
    }

    // reject unsupported preemption until engine preempt points land
    if options.preempt_interval_ns.is_some() {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "options.scheduler.preempt_interval_ns",
            "preempt interval is not implemented yet",
        ))
        .boxed());
    }

    // reject invalid budget and timing values
    if matches!(options.tick_budget_ns, Some(0))
        || matches!(options.microtask_budget, Some(0))
        || matches!(options.host_event_budget, Some(0))
        || matches!(options.max_microtask_depth, Some(0))
        || matches!(options.timer_resolution_ns, Some(0))
        || matches!(options.max_timer_coalesce_ns, Some(0))
    {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "options.scheduler",
            "scheduler budget and timer values must be greater than zero",
        ))
        .boxed());
    }

    Ok(())
}

/// Validate one watch payload for repeatable dispatch.
fn validate_watch(watch: &EventLoopWatch) -> RuntimeResult<()> {
    if matches!(watch.runnable, EngineContinuation::Vm(_)) {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "watch.runnable",
            "vm continuations are not supported for event loop watches",
        ))
        .boxed());
    }

    Ok(())
}

/// Sort platform events into a deterministic order.
fn sort_platform_events(events: &mut [PollerEvent]) {
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
fn source_order(source: PollerEventSource) -> u8 {
    match source {
        PollerEventSource::Io => 0,
        PollerEventSource::Signal => 1,
        PollerEventSource::Process => 2,
        PollerEventSource::Timer => 3,
    }
}

/// Build an ordering key for event payload data.
fn payload_sort_key(payload: PollerEventPayload) -> u64 {
    // pack event payload data into a deterministic ordering key
    match payload {
        PollerEventPayload::Io { data } => data,
        PollerEventPayload::Signal { signal } => signal as u64,
        PollerEventPayload::Process { pid, status } => {
            let status_key = match status {
                PollerProcessStatus::Exited { code } => (0u64, code as u64),
                PollerProcessStatus::Signaled { signal, core_dump } => {
                    (1u64, (signal as u64) << 1 | core_dump as u64)
                }
                PollerProcessStatus::Stopped { signal } => (2u64, signal as u64),
                PollerProcessStatus::Continued => (3u64, 0),
            };
            ((pid as u64) << 32) | (status_key.0 << 16) | status_key.1
        }
        PollerEventPayload::Timer { deadline_nanos } => deadline_nanos,
    }
}

/// Return whether host semantic events should dispatch before poller events.
fn should_dispatch_host_event_first(
    host_events: &VecDeque<HostEvent>,
    poller_events: &VecDeque<PollerEvent>,
    host_events_since_poller: u64,
    host_event_budget: Option<u64>,
) -> bool {
    if host_events.is_empty() {
        return false;
    }

    if poller_events.is_empty() {
        return true;
    }

    host_events_since_poller < host_event_budget.unwrap_or(DEFAULT_HOST_EVENT_BUDGET)
}

#[cfg(test)]
mod tests {
    use super::EventLoop;
    use crate::host::{HostEvent, HostLifecycleEvent, HostLifecycleState};
    use crate::platform::ResourceId;
    use crate::runtime::DropReason;
    use crate::runtime::poller::{
        PollerEvent, PollerEventFlags, PollerEventMask, PollerEventPayload, PollerEventSource,
        PollerToken,
    };
    use crate::runtime::scheduler::Runnable;

    fn io_poller_event(token: u64) -> PollerEvent {
        PollerEvent {
            resource_id: ResourceId(1),
            source: PollerEventSource::Io,
            mask: PollerEventMask::READABLE,
            flags: PollerEventFlags::NONE,
            token: PollerToken(token),
            payload: PollerEventPayload::Io { data: 0 },
        }
    }

    #[test]
    fn test_next_runnable_dequeues_host_event_before_poller_event() {
        let mut event_loop = EventLoop::default();

        event_loop.enqueue_events(vec![io_poller_event(8)]);
        event_loop.enqueue_host_events(vec![HostEvent::Lifecycle(HostLifecycleEvent {
            state: HostLifecycleState::Running,
        })]);

        let first = event_loop.next_runnable(0, 0).unwrap();
        let second = event_loop.next_runnable(0, 0).unwrap();

        assert!(matches!(
            first,
            Some(Runnable::HostEvent(HostEvent::Lifecycle(
                HostLifecycleEvent {
                    state: HostLifecycleState::Running
                }
            )))
        ));
        assert!(matches!(second, Some(Runnable::PollerEvent(_))));
    }

    #[test]
    fn test_next_runnable_interleaves_after_host_event_budget() {
        let mut event_loop = EventLoop::default();
        event_loop
            .configure(destack_workspace::SchedulerOptions {
                host_event_budget: Some(1),
                ..destack_workspace::SchedulerOptions::default()
            })
            .unwrap();

        event_loop.enqueue_host_events(vec![
            HostEvent::Lifecycle(HostLifecycleEvent {
                state: HostLifecycleState::Running,
            }),
            HostEvent::Lifecycle(HostLifecycleEvent {
                state: HostLifecycleState::Stopped,
            }),
        ]);
        event_loop.enqueue_events(vec![io_poller_event(9)]);

        let first = event_loop.next_runnable(0, 0).unwrap();
        let second = event_loop.next_runnable(0, 0).unwrap();
        let third = event_loop.next_runnable(0, 0).unwrap();

        assert!(matches!(first, Some(Runnable::HostEvent(_))));
        assert!(matches!(second, Some(Runnable::PollerEvent(_))));
        assert!(matches!(third, Some(Runnable::HostEvent(_))));
    }

    #[test]
    fn test_drop_counts_tracks_unwatched_dispatch() {
        let mut event_loop = EventLoop::default();

        event_loop.record_drop(DropReason::UnwatchedDispatch, 1);

        let drop_counts = event_loop.drop_counts();
        assert_eq!(drop_counts.count(DropReason::UnwatchedDispatch), 1);
        assert_eq!(drop_counts.count(DropReason::QueuePressure), 0);
        assert_eq!(drop_counts.total(), 1);
    }
}
