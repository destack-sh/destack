use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

use parking_lot::{Condvar, Mutex};

/// Runtime-owned retained event-log state for one event stream.
#[derive(Debug)]
pub struct RuntimeEventLog<T> {
    /// The sequence for the first retained live record.
    pub first_sequence: u64,
    /// The sequence to assign to the next live record.
    pub next_sequence: u64,
    /// The retained live event records.
    pub records: VecDeque<T>,
}

impl<T> Default for RuntimeEventLog<T> {
    /// Build one empty event log.
    fn default() -> Self {
        Self {
            first_sequence: 1,
            next_sequence: 1,
            records: VecDeque::new(),
        }
    }
}

/// Runtime-owned registry of live event streams.
#[derive(Debug)]
pub struct RuntimeStreamRegistry<T> {
    /// The monotonic identifier source for registered streams.
    next_stream_id: AtomicU64,
    /// The registered stream values keyed by stable stream id.
    streams: Mutex<HashMap<u64, Arc<T>>>,
}

impl<T> Default for RuntimeStreamRegistry<T> {
    /// Build one empty stream registry.
    fn default() -> Self {
        Self {
            next_stream_id: AtomicU64::new(1),
            streams: Mutex::new(HashMap::new()),
        }
    }
}

impl<T> RuntimeStreamRegistry<T> {
    /// Allocate one stable stream identifier.
    pub fn next_stream_id(&self) -> u64 {
        self.next_stream_id.fetch_add(1, Ordering::Relaxed)
    }

    /// Register one live stream value.
    pub fn register(&self, stream_id: u64, stream: Arc<T>) {
        self.streams.lock().insert(stream_id, stream);
    }

    /// Unregister one live stream value.
    pub fn unregister(&self, stream_id: u64) -> Option<Arc<T>> {
        self.streams.lock().remove(&stream_id)
    }

    /// Return one snapshot of the live stream values.
    pub fn snapshot(&self) -> Vec<Arc<T>> {
        self.streams.lock().values().cloned().collect()
    }
}

/// Runtime-owned optional snapshot cache for one normalized domain.
#[derive(Debug, Default)]
pub struct RuntimeSnapshotCache<T> {
    /// The cached snapshot value when one baseline exists.
    snapshot: Mutex<Option<T>>,
}

impl<T> RuntimeSnapshotCache<T> {
    /// Initialize the cache when no baseline exists yet.
    pub fn initialize(&self, snapshot: T) -> bool {
        let mut cached_snapshot = self.snapshot.lock();

        // preserve the existing baseline once publication has started
        if cached_snapshot.is_some() {
            return false;
        }

        *cached_snapshot = Some(snapshot);
        true
    }

    /// Replace the cache with one fresh baseline.
    pub fn reset(&self, snapshot: T) {
        let mut cached_snapshot = self.snapshot.lock();

        *cached_snapshot = Some(snapshot);
    }

    /// Replace the cache and return one delta built from the previous baseline.
    pub fn replace<R>(&self, snapshot: T, build_delta: impl FnOnce(&T, &T) -> R) -> Option<R> {
        let mut cached_snapshot = self.snapshot.lock();
        let Some(previous_snapshot) = cached_snapshot.as_ref() else {
            *cached_snapshot = Some(snapshot);
            return None;
        };

        let delta = build_delta(previous_snapshot, &snapshot);
        *cached_snapshot = Some(snapshot);
        Some(delta)
    }
}

/// Runtime-owned blocking event queue for one live stream.
#[derive(Debug)]
pub struct RuntimeEventQueue<T> {
    /// Pending events for this queue.
    events: Mutex<VecDeque<T>>,
    /// Wake primitive for blocking readers.
    wake: Condvar,
    /// Whether this queue has been closed.
    is_closed: AtomicBool,
}

impl<T> Default for RuntimeEventQueue<T> {
    /// Build one empty event queue.
    fn default() -> Self {
        Self {
            events: Mutex::new(VecDeque::new()),
            wake: Condvar::new(),
            is_closed: AtomicBool::new(false),
        }
    }
}

impl<T> RuntimeEventQueue<T> {
    /// Push one event and wake blocked readers.
    pub fn push(&self, event: T) {
        if self.is_closed.load(Ordering::Relaxed) {
            return;
        }

        let mut events = self.events.lock();
        events.push_back(event);
        self.wake.notify_all();
    }

    /// Try to take one queued event.
    pub fn try_take(&self) -> Option<T> {
        let mut events = self.events.lock();
        events.pop_front()
    }

    /// Return whether this queue has been closed.
    pub fn is_closed(&self) -> bool {
        self.is_closed.load(Ordering::Relaxed)
    }

    /// Close this queue and wake blocked readers.
    pub fn close(&self) {
        self.is_closed.store(true, Ordering::Relaxed);
        self.wake.notify_all();
    }

    /// Wait once for queued events or timeout.
    pub fn wait_once(&self, duration: Duration) {
        let mut events = self.events.lock();

        if events.is_empty() {
            self.wake.wait_for(&mut events, duration);
        }
    }
}
