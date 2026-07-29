use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::{Condvar, Mutex, MutexGuard};

use crate::diagnostic::RuntimeResult;
use crate::host::{Host, HostEvent};

/// Shared queue for host event delivery.
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
    /// Wake sequence for wake-without-event notifications.
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

    /// Advance host-owned events into this queue.
    pub(crate) fn advance(&self, host: &dyn Host) -> RuntimeResult<()> {
        host.advance_events()?;
        let events = host.collect_events()?;
        self.enqueue(events);

        Ok(())
    }

    /// Poll host events with one optional timeout in nanoseconds.
    pub(crate) fn poll(
        &self,
        host: &dyn Host,
        timeout_nanos: Option<u64>,
    ) -> RuntimeResult<Vec<HostEvent>> {
        self.advance(host)?;

        Ok(self.wait(timeout_nanos))
    }

    /// Wait for queued host events with one optional timeout in nanoseconds.
    fn wait(&self, timeout_nanos: Option<u64>) -> Vec<HostEvent> {
        let mut payload = self.state.queue.lock();
        let initial_wake_sequence = payload.wake_sequence;

        if !payload.events.is_empty() {
            return payload.drain();
        }

        if let Some(0) = timeout_nanos {
            return Vec::new();
        }

        self.state
            .wait(&mut payload, initial_wake_sequence, timeout_nanos);

        payload.drain()
    }
}

impl QueueState {
    /// Wait until events arrive, one wake is observed, or the optional timeout elapses.
    fn wait(
        &self,
        payload: &mut MutexGuard<'_, QueuePayload>,
        initial_wake_sequence: u64,
        timeout_nanos: Option<u64>,
    ) {
        let Some(timeout_nanos) = timeout_nanos else {
            while payload.events.is_empty() && payload.wake_sequence == initial_wake_sequence {
                self.wake.wait(payload);
            }

            return;
        };
        let deadline = Instant::now() + Duration::from_nanos(timeout_nanos);

        while payload.events.is_empty() && payload.wake_sequence == initial_wake_sequence {
            let now = Instant::now();
            if now >= deadline {
                break;
            }

            let remaining = deadline.saturating_duration_since(now);
            self.wake.wait_for(payload, remaining);
        }
    }
}

impl QueuePayload {
    /// Drain all queued events.
    fn drain(&mut self) -> Vec<HostEvent> {
        self.events.drain(..).collect()
    }
}
