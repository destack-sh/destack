use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::{Condvar, Mutex, MutexGuard};
use tracing::error;

use crate::diagnostic::RuntimeResult;
use crate::host::core::event::{HostEvent, HostEventKind};
use crate::host::core::registry::{HostEventObserver, HostRuntimeId, RuntimeIngressObserver};
use crate::runtime::poller::PollerWakeHandle;

/// Shared host event queue for adapter event delivery.
#[derive(Debug, Clone)]
pub(crate) struct HostQueue {
    /// Shared queue state and synchronization primitives.
    state: Arc<HostQueueState>,
}

/// Shared host event queue state.
#[derive(Debug, Default)]
struct HostQueueState {
    /// Event payload queue and wake sequence metadata.
    queue: Mutex<HostQueuePayload>,
    /// Runtime-owned ingress observers for this queue.
    runtime_ingress_observers: Mutex<Vec<Arc<dyn RuntimeIngressObserver>>>,
    /// Host semantic event observers for this queue.
    host_event_observers: Mutex<Vec<Arc<dyn HostEventObserver>>>,
    /// Condition variable for blocking poll operations.
    wake: Condvar,
}

/// Host event queue payload protected by one mutex.
#[derive(Debug, Default)]
struct HostQueuePayload {
    /// Pending host events.
    events: VecDeque<HostEvent>,
    /// Monotonic wake sequence for wake-without-event notifications.
    wake_sequence: u64,
    /// Queue capacity when one bounded policy is configured.
    capacity: Option<usize>,
    /// Number of dropped events since the last read.
    dropped_events_since_read: u64,
}

impl HostQueue {
    /// Create one empty host event queue.
    pub(crate) fn new(_host_runtime_id: HostRuntimeId) -> Self {
        Self {
            state: Arc::new(HostQueueState::default()),
        }
    }

    /// Configure one optional queue capacity.
    pub(crate) fn configure_capacity(&self, capacity: Option<usize>) {
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
        let event_observer_event = event.clone();

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
        drop(payload);

        // host event observers receive the original event stream independently
        if let Err(error) = self.dispatch_host_event(&event_observer_event) {
            error!(?error, "host event observer dispatch failed");
        }
    }

    /// Return one shared wake handle for this queue.
    pub(crate) fn poll_wake_handle(&self) -> Arc<dyn PollerWakeHandle> {
        Arc::new(self.clone())
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

    /// Register one observer and snapshot queued events under one queue lock.
    pub(crate) fn register_observer_and_snapshot(
        &self,
        observer: &Arc<dyn HostEventObserver>,
    ) -> Vec<HostEvent> {
        let payload = self.state.queue.lock();

        // register while the queue lock is held so future enqueues cannot race the snapshot
        register_observer(&self.state.host_event_observers, observer);

        payload.events.iter().cloned().collect()
    }

    /// Register one runtime ingress observer for this queue.
    pub(crate) fn register_runtime_ingress_observer(
        &self,
        observer: &Arc<dyn RuntimeIngressObserver>,
    ) {
        register_observer(&self.state.runtime_ingress_observers, observer);
    }

    /// Dispatch one host event to queue-owned observers.
    pub(crate) fn dispatch_host_event(&self, event: &HostEvent) -> RuntimeResult<()> {
        let observers = self.state.host_event_observers.lock().clone();

        for observer in observers {
            observer.observe_host_event(event)?;
        }

        Ok(())
    }

    /// Service queue-owned runtime ingress observers.
    pub(crate) fn process_runtime_ingress(&self) -> RuntimeResult<()> {
        let observers = self.state.runtime_ingress_observers.lock().clone();

        for observer in observers {
            observer.process_runtime_ingress()?;
        }

        Ok(())
    }
}

impl PollerWakeHandle for HostQueue {
    fn wake(&self) -> RuntimeResult<()> {
        let mut payload = self.state.queue.lock();
        payload.wake_sequence = payload.wake_sequence.wrapping_add(1);
        self.state.wake.notify_all();

        Ok(())
    }
}

/// Wait on one queue until events arrive or one wake is observed.
fn wait_without_timeout(
    state: &HostQueueState,
    payload: &mut MutexGuard<'_, HostQueuePayload>,
    initial_wake_sequence: u64,
) {
    while payload.events.is_empty() && payload.wake_sequence == initial_wake_sequence {
        state.wake.wait(payload);
    }
}

/// Wait on one queue until events arrive, one wake is observed, or timeout elapses.
fn wait_with_timeout(
    state: &HostQueueState,
    payload: &mut MutexGuard<'_, HostQueuePayload>,
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
fn drain_events(payload: &mut HostQueuePayload) -> Vec<HostEvent> {
    payload.events.drain(..).collect()
}

/// Enforce queue capacity and return whether the new event should be enqueued.
fn enforce_capacity_before_enqueue(payload: &mut HostQueuePayload, event: &HostEvent) -> bool {
    let Some(capacity) = payload.capacity else {
        return true;
    };

    // drop older drop-eligible events before accepting new pressure
    while payload.events.len() >= capacity {
        // remove the oldest drop-eligible event first
        let Some(index) = oldest_drop_eligible_event_index(&payload.events) else {
            // allow overflow when only lossless events remain, or when a coalescing
            // signal must sit alongside them
            if is_lossless_event(event) || is_coalescing_event(event) {
                return true;
            }

            payload.dropped_events_since_read = payload.dropped_events_since_read.saturating_add(1);
            return false;
        };

        payload.events.remove(index);
        payload.dropped_events_since_read = payload.dropped_events_since_read.saturating_add(1);
    }

    true
}

/// Remove stale semantic events that are modeled as latest-state signals.
fn coalesce_semantic_event(events: &mut VecDeque<HostEvent>, event: &HostEvent) {
    let Some(event_coalescing_key) = coalescing_key(event) else {
        return;
    };

    events.retain(|queued_event| coalescing_key(queued_event) != Some(event_coalescing_key));
}

/// Return the oldest event index that is eligible for drop-on-pressure policy.
fn oldest_drop_eligible_event_index(events: &VecDeque<HostEvent>) -> Option<usize> {
    events
        .iter()
        .position(|queued_event| !is_lossless_event(queued_event))
}

/// Return whether this event must be handled losslessly.
fn is_lossless_event(event: &HostEvent) -> bool {
    matches!(
        event.kind(),
        HostEventKind::Intent
            | HostEventKind::Background
            | HostEventKind::Media
            | HostEventKind::Notification
            | HostEventKind::Permission
    )
}

/// Return whether this event uses latest-state coalescing semantics.
fn is_coalescing_event(event: &HostEvent) -> bool {
    matches!(
        event.kind(),
        HostEventKind::Lifecycle
            | HostEventKind::Interruption
            | HostEventKind::MemoryPressure
            | HostEventKind::ThermalState
            | HostEventKind::PowerMode
            | HostEventKind::WallClock
    )
}

/// Return one coalescing identity key for this event when applicable.
fn coalescing_key(event: &HostEvent) -> Option<HostEventKind> {
    if !is_coalescing_event(event) {
        return None;
    }

    Some(event.kind())
}

/// Register one queue-owned observer when it is not already present.
fn register_observer<T: ?Sized + 'static>(observers: &Mutex<Vec<Arc<T>>>, observer: &Arc<T>) {
    let observer_pointer = Arc::as_ptr(observer) as *const ();
    let mut observers = observers.lock();
    let is_registered = observers
        .iter()
        .any(|existing| Arc::as_ptr(existing) as *const () == observer_pointer);

    if !is_registered {
        observers.push(Arc::clone(observer));
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::{Arc, mpsc};
    use std::thread;
    use std::time::{Duration, Instant};

    use crate::diagnostic::RuntimeResult;
    use crate::host::core::queue::HostQueue;
    use crate::host::core::registry::{HostEventObserver, next_host_runtime_id};
    use crate::host::{HostEvent, HostLifecycleEvent, HostLifecycleState, HostPermissionEvent};

    fn lifecycle_event(state: HostLifecycleState) -> HostEvent {
        HostEvent::Lifecycle(HostLifecycleEvent { state })
    }

    fn permission_event(permission: &str, granted: bool) -> HostEvent {
        HostEvent::Permission(HostPermissionEvent {
            permission: permission.to_string(),
            granted,
        })
    }

    fn queue() -> HostQueue {
        HostQueue::new(next_host_runtime_id())
    }

    #[derive(Debug)]
    struct TestHostEventObserver {
        callback_count: Arc<AtomicU64>,
    }

    impl HostEventObserver for TestHostEventObserver {
        fn observe_host_event(&self, _event: &HostEvent) -> RuntimeResult<()> {
            self.callback_count.fetch_add(1, Ordering::Relaxed);

            Ok(())
        }
    }

    #[test]
    fn test_poll_events_drains_enqueued_events() {
        let queue = queue();
        queue.enqueue(lifecycle_event(HostLifecycleState::Running));

        let events = queue.poll_events(Some(0)).unwrap();

        assert_eq!(events.len(), 1);
    }

    #[test]
    fn test_poll_events_returns_empty_after_timeout() {
        let queue = queue();
        let start = Instant::now();

        let events = queue.poll_events(Some(10_000_000)).unwrap();

        assert!(events.is_empty());
        assert!(start.elapsed() >= Duration::from_millis(9));
    }

    #[test]
    fn test_wake_handle_interrupts_blocking_poll() {
        let queue = queue();
        let queue_for_thread = queue.clone();
        let wake_handle = queue.poll_wake_handle();
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
        let queue = queue();
        queue.configure_capacity(Some(1));

        queue.enqueue(lifecycle_event(HostLifecycleState::Paused));
        queue.enqueue(permission_event("camera", true));

        let events = queue.poll_events(Some(0)).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], permission_event("camera", true));
    }

    #[test]
    fn test_take_dropped_event_count_resets_counter() {
        let queue = queue();
        queue.configure_capacity(Some(1));

        queue.enqueue(lifecycle_event(HostLifecycleState::Paused));
        queue.enqueue(permission_event("camera", true));

        let dropped_first = queue.take_dropped_event_count();
        let dropped_second = queue.take_dropped_event_count();
        assert_eq!(dropped_first, 1);
        assert_eq!(dropped_second, 0);
    }

    #[test]
    fn test_coalesces_lifecycle_events_to_latest_state() {
        let queue = queue();

        queue.enqueue(lifecycle_event(HostLifecycleState::Initializing));
        queue.enqueue(lifecycle_event(HostLifecycleState::Running));

        let events = queue.poll_events(Some(0)).unwrap();
        assert_eq!(events, vec![lifecycle_event(HostLifecycleState::Running)]);
        assert_eq!(queue.take_dropped_event_count(), 0);
    }

    #[test]
    fn test_register_observer_and_snapshot_does_not_replay_existing_events_live() {
        let queue = queue();
        let callback_count = Arc::new(AtomicU64::new(0));
        let observer: Arc<dyn HostEventObserver> = Arc::new(TestHostEventObserver {
            callback_count: Arc::clone(&callback_count),
        });

        queue.enqueue(lifecycle_event(HostLifecycleState::Paused));

        let events = queue.register_observer_and_snapshot(&observer);
        assert_eq!(events.len(), 1);
        assert_eq!(callback_count.load(Ordering::Relaxed), 0);

        queue.enqueue(lifecycle_event(HostLifecycleState::Running));
        assert_eq!(callback_count.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_permission_events_stay_lossless_under_capacity_pressure() {
        let queue = queue();
        queue.configure_capacity(Some(1));

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
        let queue = queue();
        queue.configure_capacity(Some(1));

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
    fn test_coalescing_event_overflows_lossless_capacity_without_drops() {
        let queue = queue();
        queue.configure_capacity(Some(1));

        queue.enqueue(permission_event("camera", false));
        queue.enqueue(lifecycle_event(HostLifecycleState::Running));

        let events = queue.poll_events(Some(0)).unwrap();
        assert_eq!(
            events,
            vec![
                permission_event("camera", false),
                lifecycle_event(HostLifecycleState::Running),
            ]
        );
        assert_eq!(queue.take_dropped_event_count(), 0);
    }
}
