use super::*;

/// Runtime-owned network watch stream.
#[derive(Debug)]
pub(crate) struct NetworkWatchStream {
    /// Last emitted network snapshot.
    last_state: Mutex<NetworkState>,
    /// Shared event queue state for this stream.
    queue: RuntimeEventQueue<NetworkEvent>,
    /// Sequence number for the next event in this stream.
    next_sequence: AtomicU64,
}

impl NetworkWatchStream {
    /// Create one network watch stream seeded from the current snapshot.
    pub(crate) fn new(initial_state: NetworkState) -> Self {
        Self {
            last_state: Mutex::new(initial_state),
            queue: RuntimeEventQueue::default(),
            next_sequence: AtomicU64::new(0),
        }
    }

    /// Publish one changed network snapshot to this stream.
    pub(crate) fn publish_state(&self, next_state: NetworkState) {
        if self.is_closed() {
            return;
        }

        let mut last_state = self.last_state.lock();
        if *last_state == next_state {
            return;
        }

        let timestamp_ns = monotonic_now_ns();
        let sequence = self.next_sequence.fetch_add(1, Ordering::Relaxed);
        *last_state = next_state;

        self.queue.push(NetworkEvent {
            timestamp_ns,
            sequence,
            state: next_state,
        });
    }

    /// Try to take one queued network event.
    pub(crate) fn try_take(&self) -> Option<NetworkEvent> {
        self.queue.try_take()
    }

    /// Return whether this stream is closed.
    pub(crate) fn is_closed(&self) -> bool {
        self.queue.is_closed()
    }

    /// Close this stream and wake blocked readers.
    pub(crate) fn close(&self) {
        self.queue.close();
    }

    /// Wait once for queued events or timeout.
    pub(crate) fn wait_once(&self, duration: Duration) {
        self.queue.wait_once(duration);
    }
}
