use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::{Condvar, Mutex, MutexGuard};

use crate::diagnostic::RuntimeResult;
use crate::host::HostEvent;
use crate::host::poller::PollerWakeHandle;

/// Shared host event queue for host event delivery.
#[derive(Debug, Clone)]
pub(crate) struct HostQueue {
    /// Shared queue state and synchronization primitives.
    state: Arc<QueueState>,
}

/// Shared host event queue state.
#[derive(Debug, Default)]
struct QueueState {
    /// Event payload queue and wake sequence metadata.
    queue: Mutex<QueuePayload>,
    /// Condition variable for blocking poll operations.
    wake: Condvar,
}

/// Host event queue payload protected by one mutex.
#[derive(Debug, Default)]
struct QueuePayload {
    /// Pending host events.
    events: VecDeque<HostEvent>,
    /// Monotonic wake sequence for wake-without-event notifications.
    wake_sequence: u64,
}

impl HostQueue {
    /// Create one empty host event queue.
    pub(crate) fn new() -> Self {
        Self {
            state: Arc::new(QueueState::default()),
        }
    }

    /// Enqueue host events and wake blocked pollers.
    pub(crate) fn enqueue(&self, events: Vec<HostEvent>) {
        if events.is_empty() {
            return;
        }

        let mut payload = self.state.queue.lock();
        payload.events.extend(events);
        payload.wake_sequence = payload.wake_sequence.wrapping_add(1);
        self.state.wake.notify_all();
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
    state: &QueueState,
    payload: &mut MutexGuard<'_, QueuePayload>,
    initial_wake_sequence: u64,
) {
    while payload.events.is_empty() && payload.wake_sequence == initial_wake_sequence {
        state.wake.wait(payload);
    }
}

/// Wait on one queue until events arrive, one wake is observed, or timeout elapses.
fn wait_with_timeout(
    state: &QueueState,
    payload: &mut MutexGuard<'_, QueuePayload>,
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
fn drain_events(payload: &mut QueuePayload) -> Vec<HostEvent> {
    payload.events.drain(..).collect()
}
