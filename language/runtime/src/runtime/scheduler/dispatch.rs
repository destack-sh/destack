use super::Runnable;
use crate::diagnostic::RuntimeResult;
use crate::host::poller::{
    HostPoller, PollerEvent, PollerEventPayload, PollerEventSource, PollerProcessStatus,
};
use crate::runtime::time::{Instant, Nanos};

use super::EventLoop;

/// Default host dispatch batch size before forcing one poller event.
const DEFAULT_HOST_EVENT_BUDGET: u64 = 32;

impl EventLoop {
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
        while let Some(timer) = self.ready_timers.lock().pop_front() {
            // drop canceled timers that were already promoted into the ready queue
            if self.canceled_timers.lock().remove(&timer.handle) {
                continue;
            }

            return Ok(Some(Runnable::Timer(timer)));
        }

        // dispatch host events first while under the fairness budget
        if self.should_dispatch_host_event_first()
            && let Some(host_event) = self.host_events.pop_front()
        {
            self.host_events_since_poller = self.host_events_since_poller.saturating_add(1);
            return Ok(Some(Runnable::HostEvent(host_event)));
        }

        // dispatch one poller event and reset host fairness streak
        if let Some(event) = self.poller_events.pop_front() {
            self.host_events_since_poller = 0;
            return Ok(Some(Runnable::PollerEvent(event)));
        }

        // dispatch remaining host events when no poller event is pending
        if let Some(host_event) = self.host_events.pop_front() {
            self.host_events_since_poller = self.host_events_since_poller.saturating_add(1);
            return Ok(Some(Runnable::HostEvent(host_event)));
        }

        Ok(self.tasks.pop_front().map(Runnable::Task))
    }

    /// Report whether one runnable item is already ready without advancing time.
    pub fn has_ready_work(&self) -> bool {
        !self.tasks.is_empty()
            || !self.microtasks.is_empty()
            || !self.poller_events.is_empty()
            || !self.host_events.is_empty()
            || self.has_dispatchable_ready_timers()
    }

    /// Report whether any work remains in the event loop, including future wakes.
    pub fn has_pending_work(&self) -> bool {
        if self.has_ready_work()
            || !self.poller_event_watches.is_empty()
            || !self.host_event_watches.is_empty()
        {
            return true;
        }

        let queue = self.timers.lock();
        queue.has_pending_timers()
    }

    /// Poll the host poller and enqueue events.
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
    pub fn next_deadline(&self, wall_now: Nanos, mono_now: Nanos) -> Option<Instant> {
        // ready timers are runnable immediately
        if self.has_dispatchable_ready_timers() {
            return Some(Instant::from_nanos(wall_now));
        }

        // map wall and monotonic timer deadlines into one wall-clock wakeup
        let mut queue = self.timers.lock();
        let (wall_deadline, mono_deadline) = queue.next_deadlines();
        let wall_deadline = wall_deadline
            .filter(|deadline| *deadline > wall_now)
            .map(Instant::from_nanos);
        let mono_deadline = mono_deadline
            .filter(|deadline| *deadline > mono_now)
            .map(|deadline| {
                let delta = deadline.saturating_sub(mono_now);
                Instant::from_nanos(wall_now.saturating_add(delta))
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
            .map(|deadline| deadline.saturating_sub(Instant::from_nanos(wall_now)))
    }

    /// Return whether one ready timer remains dispatchable after cancelation filtering.
    fn has_dispatchable_ready_timers(&self) -> bool {
        let ready_timers = self.ready_timers.lock();
        if ready_timers.is_empty() {
            return false;
        }

        let canceled_timers = self.canceled_timers.lock();
        ready_timers
            .iter()
            .any(|timer| !canceled_timers.contains(&timer.handle))
    }

    /// Sort host events into a deterministic order.
    pub(super) fn sort_host_events(&self, events: &mut [PollerEvent]) {
        // ensure deterministic ordering for host events
        events.sort_by_key(|event| {
            (
                self.source_order(event.source),
                event.token.0,
                event.resource_id.local_id,
                event.mask.0,
                event.flags.0,
                self.payload_sort_key(event.payload),
            )
        });
    }

    /// Return whether host events should dispatch before poller events.
    pub(super) fn should_dispatch_host_event_first(&self) -> bool {
        if self.host_events.is_empty() {
            return false;
        }

        if self.poller_events.is_empty() {
            return true;
        }

        self.host_events_since_poller
            < self
                .options
                .host_event_budget
                .unwrap_or(DEFAULT_HOST_EVENT_BUDGET)
    }

    /// Map one event source into a deterministic ordering key.
    fn source_order(&self, source: PollerEventSource) -> u8 {
        match source {
            PollerEventSource::Io => 0,
            PollerEventSource::Signal => 1,
            PollerEventSource::Process => 2,
            PollerEventSource::Timer => 3,
        }
    }

    /// Build one ordering key for event payload data.
    fn payload_sort_key(&self, payload: PollerEventPayload) -> u64 {
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
}

#[cfg(test)]
mod tests {
    use destack_workspace::SchedulerOptions;

    use super::EventLoop;
    use crate::host::poller::{
        PollerEvent, PollerEventFlags, PollerEventMask, PollerEventPayload, PollerEventSource,
        PollerToken,
    };
    use crate::host::{HostEvent, LifecycleEvent, LifecycleSourceKind, LifecycleState, ResourceId};
    use crate::runtime::WorkerId;
    use crate::runtime::scheduler::Runnable;

    const TEST_WORKER_ID: WorkerId = WorkerId(1);

    fn io_poller_event(token: u64) -> PollerEvent {
        PollerEvent {
            resource_id: ResourceId::new(TEST_WORKER_ID, 1),
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
        event_loop.enqueue_host_events(vec![HostEvent::Lifecycle(LifecycleEvent {
            source_kind: LifecycleSourceKind::Application,
            state: LifecycleState::Running,
        })]);

        let first = event_loop.next_runnable(0, 0).unwrap();
        let second = event_loop.next_runnable(0, 0).unwrap();

        assert!(matches!(
            first,
            Some(Runnable::HostEvent(HostEvent::Lifecycle(LifecycleEvent {
                source_kind: LifecycleSourceKind::Application,
                state: LifecycleState::Running
            })))
        ));
        assert!(matches!(second, Some(Runnable::PollerEvent(_))));
    }

    #[test]
    fn test_next_runnable_interleaves_after_host_event_budget() {
        let mut event_loop = EventLoop::default();
        event_loop
            .configure(SchedulerOptions {
                host_event_budget: Some(1),
                ..SchedulerOptions::default()
            })
            .unwrap();

        event_loop.enqueue_host_events(vec![
            HostEvent::Lifecycle(LifecycleEvent {
                source_kind: LifecycleSourceKind::Application,
                state: LifecycleState::Running,
            }),
            HostEvent::Lifecycle(LifecycleEvent {
                source_kind: LifecycleSourceKind::Application,
                state: LifecycleState::Stopped,
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
}
