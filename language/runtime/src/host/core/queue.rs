use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::{Condvar, Mutex, MutexGuard};

use super::HostEvent;
use crate::diagnostic::RuntimeResult;
use crate::runtime::poller::HostPollerWakeHandle;

/// Shared host event queue for adapter event delivery.
#[derive(Debug, Clone)]
pub(crate) struct HostEventQueue {
    /// Shared queue state and synchronization primitives.
    state: Arc<HostEventQueueState>,
    /// Shared out-of-band wake handle for blocked pollers.
    wake_handle: Arc<HostEventQueueWakeHandle>,
}

/// Shared host event queue state.
#[derive(Debug, Default)]
struct HostEventQueueState {
    /// Event payload queue and wake sequence metadata.
    queue: Mutex<HostEventQueuePayload>,
    /// Condition variable for blocking poll operations.
    wake: Condvar,
}

/// Host event queue payload protected by one mutex.
#[derive(Debug, Default)]
struct HostEventQueuePayload {
    /// Pending host events.
    events: VecDeque<HostEvent>,
    /// Monotonic wake sequence for wake-without-event notifications.
    wake_sequence: u64,
    /// Queue capacity when one bounded policy is configured.
    capacity: Option<usize>,
    /// Number of dropped events since the last read.
    dropped_events_since_read: u64,
}

/// Shared wake handle for one host event queue.
#[derive(Debug)]
struct HostEventQueueWakeHandle {
    /// Shared queue state used for wake notifications.
    state: Arc<HostEventQueueState>,
}

impl HostEventQueue {
    /// Create one empty host event queue.
    pub(crate) fn new() -> Self {
        let state = Arc::new(HostEventQueueState::default());
        let wake_handle = Arc::new(HostEventQueueWakeHandle {
            state: Arc::clone(&state),
        });

        Self { state, wake_handle }
    }

    /// Configure one optional queue capacity.
    pub(crate) fn configure(&self, capacity: Option<usize>) {
        let mut payload = self.state.queue.lock();
        payload.capacity = capacity;
    }

    /// Take the number of dropped events observed since the last read.
    pub(crate) fn take_dropped_event_count(&self) -> u64 {
        let mut payload = self.state.queue.lock();
        let dropped_events_since_read = payload.dropped_events_since_read;
        payload.dropped_events_since_read = 0;

        dropped_events_since_read
    }

    /// Enqueue one host event and wake blocked pollers.
    pub(crate) fn enqueue(&self, event: HostEvent) {
        let mut payload = self.state.queue.lock();

        // coalesce latest-state semantic events before capacity checks
        coalesce_semantic_event(&mut payload.events, &event);

        // enforce bounded capacity for drop-eligible events
        let should_enqueue = enforce_capacity_before_enqueue(&mut payload, &event);
        if !should_enqueue {
            return;
        }

        payload.events.push_back(event);
        payload.wake_sequence = payload.wake_sequence.wrapping_add(1);
        self.state.wake.notify_all();
    }

    /// Return one shared wake handle for this queue.
    pub(crate) fn wake_handle(&self) -> Arc<dyn HostPollerWakeHandle> {
        self.wake_handle.clone()
    }

    /// Poll queued host events with one optional timeout in nanoseconds.
    pub(crate) fn poll_events(&self, timeout_nanos: Option<u64>) -> RuntimeResult<Vec<HostEvent>> {
        let mut payload = self.state.queue.lock();
        let initial_wake_sequence = payload.wake_sequence;

        if !payload.events.is_empty() {
            return Ok(drain_events(&mut payload));
        }

        if let Some(0) = timeout_nanos {
            return Ok(Vec::new());
        }

        match timeout_nanos {
            Some(timeout_nanos) => {
                wait_with_timeout(
                    &self.state,
                    &mut payload,
                    initial_wake_sequence,
                    timeout_nanos,
                );
            }
            None => {
                wait_without_timeout(&self.state, &mut payload, initial_wake_sequence);
            }
        }

        Ok(drain_events(&mut payload))
    }
}

impl HostPollerWakeHandle for HostEventQueueWakeHandle {
    fn wake(&self) -> RuntimeResult<()> {
        let mut payload = self.state.queue.lock();
        payload.wake_sequence = payload.wake_sequence.wrapping_add(1);
        self.state.wake.notify_all();

        Ok(())
    }
}

/// Wait on one queue until events arrive or one wake is observed.
fn wait_without_timeout(
    state: &HostEventQueueState,
    payload: &mut MutexGuard<'_, HostEventQueuePayload>,
    initial_wake_sequence: u64,
) {
    while payload.events.is_empty() && payload.wake_sequence == initial_wake_sequence {
        state.wake.wait(payload);
    }
}

/// Wait on one queue until events arrive, one wake is observed, or timeout elapses.
fn wait_with_timeout(
    state: &HostEventQueueState,
    payload: &mut MutexGuard<'_, HostEventQueuePayload>,
    initial_wake_sequence: u64,
    timeout_nanos: u64,
) {
    let wait_timeout = Duration::from_nanos(timeout_nanos);
    let deadline = Instant::now() + wait_timeout;

    while payload.events.is_empty() && payload.wake_sequence == initial_wake_sequence {
        let now = Instant::now();
        if now >= deadline {
            break;
        }

        let remaining = deadline.saturating_duration_since(now);
        state.wake.wait_for(payload, remaining);
    }
}

/// Drain all queued events into one output vector.
fn drain_events(payload: &mut HostEventQueuePayload) -> Vec<HostEvent> {
    payload.events.drain(..).collect()
}

/// Enforce queue capacity and return whether the new event should be enqueued.
fn enforce_capacity_before_enqueue(payload: &mut HostEventQueuePayload, event: &HostEvent) -> bool {
    let Some(capacity) = payload.capacity else {
        return true;
    };

    // permission results must stay lossless
    if event.is_lossless() {
        return true;
    }

    while payload.events.len() >= capacity {
        // remove the oldest drop-eligible event first
        let Some(index) = oldest_drop_eligible_event_index(&payload.events) else {
            // coalesced state events should still be accepted when only lossless events are queued
            if event.is_coalescing() {
                return true;
            }

            payload.dropped_events_since_read = payload.dropped_events_since_read.saturating_add(1);
            return false;
        };

        let _ = payload.events.remove(index);
        payload.dropped_events_since_read = payload.dropped_events_since_read.saturating_add(1);
    }

    true
}

/// Remove stale semantic events that are modeled as latest-state signals.
fn coalesce_semantic_event(events: &mut VecDeque<HostEvent>, event: &HostEvent) {
    let Some(coalescing_key) = event.coalescing_key() else {
        return;
    };

    events.retain(|queued_event| queued_event.coalescing_key() != Some(coalescing_key));
}

/// Return the oldest event index that is eligible for drop-on-pressure policy.
fn oldest_drop_eligible_event_index(events: &VecDeque<HostEvent>) -> Option<usize> {
    events
        .iter()
        .position(|queued_event| !queued_event.is_lossless())
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;
    use std::thread;
    use std::time::{Duration, Instant};

    use crate::host::{HostEvent, HostLifecycleEvent, HostLifecycleState, HostPermissionEvent};
    use crate::platform::ResourceId;
    use crate::runtime::poller::{
        PollerEvent, PollerEventFlags, PollerEventMask, PollerEventPayload, PollerEventSource,
        PollerToken,
    };

    use super::HostEventQueue;

    fn lifecycle_event(state: HostLifecycleState) -> HostEvent {
        HostEvent::Lifecycle(HostLifecycleEvent { state })
    }

    fn permission_event(permission: &str, granted: bool) -> HostEvent {
        HostEvent::Permission(HostPermissionEvent {
            permission: permission.to_string(),
            granted,
        })
    }

    fn poller_event(token: u64) -> HostEvent {
        HostEvent::Poller(PollerEvent {
            resource_id: ResourceId(7),
            source: PollerEventSource::Io,
            mask: PollerEventMask::READABLE,
            flags: PollerEventFlags::NONE,
            token: PollerToken(token),
            payload: PollerEventPayload::Io { data: 0 },
        })
    }

    #[test]
    fn test_poll_events_drains_enqueued_events() {
        let queue = HostEventQueue::new();
        queue.enqueue(lifecycle_event(HostLifecycleState::Running));

        let events = queue.poll_events(Some(0)).unwrap();

        assert_eq!(events.len(), 1);
    }

    #[test]
    fn test_poll_events_returns_empty_after_timeout() {
        let queue = HostEventQueue::new();
        let start = Instant::now();

        let events = queue.poll_events(Some(10_000_000)).unwrap();

        assert!(events.is_empty());
        assert!(start.elapsed() >= Duration::from_millis(9));
    }

    #[test]
    fn test_wake_handle_interrupts_blocking_poll() {
        let queue = HostEventQueue::new();
        let queue_for_thread = queue.clone();
        let wake_handle = queue.wake_handle();
        let (sender, receiver) = mpsc::channel();

        thread::spawn(move || {
            let start = Instant::now();
            let events = queue_for_thread.poll_events(None).unwrap();
            sender.send((events, start.elapsed())).unwrap();
        });

        thread::sleep(Duration::from_millis(10));
        wake_handle.wake().unwrap();

        let (events, elapsed) = receiver.recv_timeout(Duration::from_secs(1)).unwrap();
        assert!(events.is_empty());
        assert!(elapsed < Duration::from_millis(500));
    }

    #[test]
    fn test_configure_capacity_drops_oldest_when_full() {
        let queue = HostEventQueue::new();
        queue.configure(Some(1));

        queue.enqueue(poller_event(11));
        queue.enqueue(poller_event(12));

        let events = queue.poll_events(Some(0)).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], poller_event(12));
    }

    #[test]
    fn test_take_dropped_event_count_resets_counter() {
        let queue = HostEventQueue::new();
        queue.configure(Some(1));

        queue.enqueue(poller_event(11));
        queue.enqueue(poller_event(12));

        let dropped_first = queue.take_dropped_event_count();
        let dropped_second = queue.take_dropped_event_count();
        assert_eq!(dropped_first, 1);
        assert_eq!(dropped_second, 0);
    }

    #[test]
    fn test_coalesces_lifecycle_events_to_latest_state() {
        let queue = HostEventQueue::new();

        queue.enqueue(lifecycle_event(HostLifecycleState::Initializing));
        queue.enqueue(lifecycle_event(HostLifecycleState::Running));

        let events = queue.poll_events(Some(0)).unwrap();
        assert_eq!(events, vec![lifecycle_event(HostLifecycleState::Running)]);
        assert_eq!(queue.take_dropped_event_count(), 0);
    }

    #[test]
    fn test_permission_events_stay_lossless_under_capacity_pressure() {
        let queue = HostEventQueue::new();
        queue.configure(Some(1));

        queue.enqueue(permission_event("camera", false));
        queue.enqueue(permission_event("microphone", true));

        let events = queue.poll_events(Some(0)).unwrap();
        assert_eq!(
            events,
            vec![
                permission_event("camera", false),
                permission_event("microphone", true),
            ]
        );
        assert_eq!(queue.take_dropped_event_count(), 0);
    }

    #[test]
    fn test_coalescing_events_can_overflow_lossless_permission_capacity() {
        let queue = HostEventQueue::new();
        queue.configure(Some(1));

        queue.enqueue(permission_event("camera", false));
        queue.enqueue(lifecycle_event(HostLifecycleState::Running));
        queue.enqueue(lifecycle_event(HostLifecycleState::Paused));

        let events = queue.poll_events(Some(0)).unwrap();
        assert_eq!(
            events,
            vec![
                permission_event("camera", false),
                lifecycle_event(HostLifecycleState::Paused),
            ]
        );
        assert_eq!(queue.take_dropped_event_count(), 0);
    }

    #[test]
    fn test_drops_poller_event_when_only_lossless_events_are_queued() {
        let queue = HostEventQueue::new();
        queue.configure(Some(1));

        queue.enqueue(permission_event("camera", false));
        queue.enqueue(poller_event(12));

        let events = queue.poll_events(Some(0)).unwrap();
        assert_eq!(events, vec![permission_event("camera", false)]);
        assert_eq!(queue.take_dropped_event_count(), 1);
    }
}
