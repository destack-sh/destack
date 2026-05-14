use super::{TimerWake, Wake};
use crate::diagnostic::RuntimeResult;
use crate::host::poller::{
    HostPoller, PollerEvent, PollerEventPayload, PollerEventSource, PollerProcessStatus,
};
use crate::runtime::time::{Instant, Nanos};

use super::EventLoop;

impl EventLoop {
    /// Pop the next wake from the event loop.
    pub fn next_wake(&mut self, wall_now: Nanos, mono_now: Nanos) -> RuntimeResult<Option<Wake>> {
        if let Some(timer) = self.pop_ready_timer(wall_now, mono_now)? {
            return Ok(Some(Wake::Timer(TimerWake::new(timer.resource_id))));
        }

        Ok(self.wakes.pop_front())
    }

    /// Report whether one runnable item is already ready without advancing time.
    pub fn has_ready_work(&self) -> bool {
        !self.tasks.is_empty() || !self.microtasks.is_empty() || !self.wakes.is_empty()
    }

    /// Report whether any work remains in the event loop, including future wakes.
    pub fn has_pending_work(&self) -> bool {
        if self.has_ready_work() || !self.waiters.is_empty() {
            return true;
        }

        self.timers.has_pending_timers()
    }

    /// Poll the host poller and enqueue resource wakes.
    pub fn poll_poller(
        &mut self,
        poller: &mut dyn HostPoller,
        timeout_nanos: Option<u64>,
    ) -> RuntimeResult<usize> {
        let events = poller.poll(timeout_nanos)?;
        let count = events.len();
        if count > 0 {
            self.enqueue_poller_wakes(events);
        }

        Ok(count)
    }

    /// Return the next wall deadline when any timer can become runnable.
    pub fn next_deadline(&mut self, wall_now: Nanos, mono_now: Nanos) -> Option<Instant> {
        // due timers are runnable immediately
        if self.timers.has_ready(wall_now, mono_now) {
            return Some(Instant::from_nanos(wall_now));
        }

        // map wall and monotonic timer deadlines into one wall-clock wakeup
        let (wall_deadline, mono_deadline) = self.timers.next_deadlines();
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
    pub fn timeout_until_next_timer(&mut self, wall_now: Nanos, mono_now: Nanos) -> Option<Nanos> {
        self.next_deadline(wall_now, mono_now)
            .map(|deadline| deadline.saturating_sub(Instant::from_nanos(wall_now)))
    }

    /// Sort poller events before converting them into resource wakes.
    pub(super) fn sort_poller_wakes(&self, events: &mut [PollerEvent]) {
        // deterministic wake order
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
    use super::EventLoop;
    use crate::host::poller::{
        PollerEvent, PollerEventFlags, PollerEventMask, PollerEventPayload, PollerEventSource,
        PollerToken,
    };
    use crate::host::{HostEvent, LifecycleEvent, LifecycleSourceKind, LifecycleState, ResourceId};
    use crate::runtime::WorkerId;
    use crate::runtime::scheduler::{Readiness, Wake};
    use crate::runtime::time::Nanos;

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
    fn test_next_wake_dequeues_ingress_fifo() {
        let mut event_loop = EventLoop::default();

        event_loop.enqueue_poller_wakes(vec![io_poller_event(8)]);
        event_loop.enqueue_host_wakes(vec![HostEvent::Lifecycle(LifecycleEvent {
            source_kind: LifecycleSourceKind::Application,
            state: LifecycleState::Running,
        })]);

        let first = event_loop.next_wake(Nanos::new(0), Nanos::new(0)).unwrap();
        let second = event_loop.next_wake(Nanos::new(0), Nanos::new(0)).unwrap();

        assert!(matches!(
            first,
            Some(Wake::Resource(wake)) if wake.readiness == Readiness::Readable
        ));
        assert!(matches!(
            second,
            Some(Wake::Host(wake)) if wake.event == HostEvent::Lifecycle(LifecycleEvent {
                source_kind: LifecycleSourceKind::Application,
                state: LifecycleState::Running
            })
        ));
    }

    #[test]
    fn test_next_wake_selects_due_timer_before_ingress() {
        let mut event_loop = EventLoop::default();
        event_loop.enqueue_poller_wakes(vec![io_poller_event(9)]);
        event_loop
            .schedule_timer(crate::runtime::scheduler::ScheduledTimer {
                resource_id: ResourceId::new(TEST_WORKER_ID, 2),
                deadline: crate::runtime::scheduler::TimerDeadline {
                    clock: crate::host::time::TimerClock::Wall,
                    at: Nanos::new(0),
                },
                interval: None,
            })
            .unwrap();

        let first = event_loop.next_wake(Nanos::new(0), Nanos::new(0)).unwrap();
        let second = event_loop.next_wake(Nanos::new(0), Nanos::new(0)).unwrap();

        assert!(matches!(first, Some(Wake::Timer(_))));
        assert!(matches!(
            second,
            Some(Wake::Resource(wake)) if wake.readiness == Readiness::Readable
        ));
    }
}
