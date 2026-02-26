use std::collections::VecDeque;

use destack_workspace::{SchedulerOptions, SchedulerPolicy};
use parking_lot::Mutex;
use rustc_hash::{FxHashMap, FxHashSet};

use super::{Microtask, MicrotaskId, Runnable, Task, TaskId, Timer, TimerQueue};
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::{PlatformError, ResourceId};
use crate::runtime::engine::{EngineContinuation, RuntimeValue};
use crate::runtime::host::{HostEvent, HostEventKind};
use crate::runtime::poller::{
    HostPoller, PollerEvent, PollerEventPayload, PollerEventSource, PollerProcessStatus,
    PollerToken,
};

/// Default host semantic dispatch batch size before forcing one poller event.
const DEFAULT_HOST_EVENT_BUDGET: u64 = 32;

/// Watch payload that can be dispatched as one event loop task.
#[derive(Debug)]
pub struct EventLoopWatch {
    /// Runnable continuation to execute when dispatched.
    pub runnable: EngineContinuation,
    /// Resume value passed into the continuation.
    pub resume_value: RuntimeValue,
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
    /// Number of dropped events with no registered dispatch watch.
    dropped_unwatched_dispatch_events: u64,
    /// Number of dropped host queue events due to queue pressure policy.
    dropped_host_queue_events: u64,
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

        // move ready timers into the dispatch queue
        self.enqueue_ready_timers(wall_now_nanos, mono_now_nanos)?;
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

    /// Report whether any work remains in the event loop.
    pub fn has_pending_work(&self) -> bool {
        if !self.tasks.is_empty()
            || !self.microtasks.is_empty()
            || !self.events.is_empty()
            || !self.host_events.is_empty()
            || !self.ready_timers.is_empty()
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
        let fire_at_nanos = self.normalize_deadline(timer.fire_at_nanos);
        let interval_nanos = timer.interval_nanos.map(|interval| {
            if interval <= 1 {
                return interval;
            }

            self.normalize_deadline(interval).max(1)
        });
        let timer = Timer {
            clock: timer.clock,
            handle: timer.handle,
            fire_at_nanos,
            interval_nanos,
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
    pub fn poll_timers(
        &self,
        wall_now_nanos: u64,
        mono_now_nanos: u64,
    ) -> RuntimeResult<Vec<Timer>> {
        let mut queue = self.timers.lock();
        let ready = queue.poll_ready(wall_now_nanos, mono_now_nanos);
        Ok(ready)
    }

    /// Enqueue ready timers from the timer queue.
    pub fn enqueue_ready_timers(
        &mut self,
        wall_now_nanos: u64,
        mono_now_nanos: u64,
    ) -> RuntimeResult<()> {
        let ready = self.poll_timers(wall_now_nanos, mono_now_nanos)?;
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

    /// Return next wall and monotonic timer deadlines when they exist.
    pub fn next_timer_deadlines(&self) -> (Option<u64>, Option<u64>) {
        let mut queue = self.timers.lock();
        queue.next_deadlines()
    }

    /// Return the timeout until the next timer is ready in any clock domain.
    pub fn timeout_until_next_timer(
        &self,
        wall_now_nanos: u64,
        mono_now_nanos: u64,
    ) -> Option<u64> {
        if !self.ready_timers.is_empty() {
            return Some(0);
        }

        let (wall_deadline, mono_deadline) = self.next_timer_deadlines();
        let wall_timeout = wall_deadline.map(|deadline| deadline.saturating_sub(wall_now_nanos));
        let mono_timeout = mono_deadline.map(|deadline| deadline.saturating_sub(mono_now_nanos));

        match (wall_timeout, mono_timeout) {
            (Some(wall_timeout), Some(mono_timeout)) => Some(wall_timeout.min(mono_timeout)),
            (Some(wall_timeout), None) => Some(wall_timeout),
            (None, Some(mono_timeout)) => Some(mono_timeout),
            (None, None) => None,
        }
    }

    /// Record one dropped event with no registered dispatch watch.
    pub fn record_dropped_unwatched_dispatch_event(&mut self) {
        self.dropped_unwatched_dispatch_events =
            self.dropped_unwatched_dispatch_events.saturating_add(1);
    }

    /// Record dropped host queue events reported by the host adapter.
    pub fn record_dropped_host_queue_events(&mut self, dropped_count: u64) {
        self.dropped_host_queue_events =
            self.dropped_host_queue_events.saturating_add(dropped_count);
    }

    /// Return the number of dropped events with no registered dispatch watch.
    pub const fn dropped_unwatched_dispatch_events(&self) -> u64 {
        self.dropped_unwatched_dispatch_events
    }

    /// Return the number of dropped host queue events due to queue pressure.
    pub const fn dropped_host_queue_events(&self) -> u64 {
        self.dropped_host_queue_events
    }

    /// Return the total number of dropped dispatch-visible events.
    pub const fn dropped_dispatch_events(&self) -> u64 {
        self.dropped_unwatched_dispatch_events
            .saturating_add(self.dropped_host_queue_events)
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
    fn normalize_deadline(&self, deadline_nanos: u64) -> u64 {
        // quantize to timer resolution first
        let mut normalized = deadline_nanos;
        if let Some(timer_resolution_ns) = self.options.timer_resolution_ns {
            normalized = round_up_deadline(normalized, timer_resolution_ns);
        }

        // then quantize to the configured coalescing window
        if let Some(max_timer_coalesce_ns) = self.options.max_timer_coalesce_ns {
            normalized = round_up_deadline(normalized, max_timer_coalesce_ns);
        }

        normalized
    }
}

/// Round one deadline up to one deterministic quantum.
fn round_up_deadline(deadline_nanos: u64, quantum_nanos: u64) -> u64 {
    if quantum_nanos <= 1 {
        return deadline_nanos;
    }

    let remainder = deadline_nanos % quantum_nanos;
    if remainder == 0 {
        return deadline_nanos;
    }

    deadline_nanos.saturating_add(quantum_nanos.saturating_sub(remainder))
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
    use crate::platform::ResourceId;
    use crate::runtime::host::{HostEvent, HostLifecycleEvent, HostLifecycleState};
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
    fn test_dropped_dispatch_counters_track_sources_separately() {
        let mut event_loop = EventLoop::default();

        event_loop.record_dropped_unwatched_dispatch_event();
        event_loop.record_dropped_host_queue_events(3);

        assert_eq!(event_loop.dropped_unwatched_dispatch_events(), 1);
        assert_eq!(event_loop.dropped_host_queue_events(), 3);
        assert_eq!(event_loop.dropped_dispatch_events(), 4);
    }
}
