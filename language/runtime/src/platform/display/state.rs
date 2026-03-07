use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

#[cfg(target_os = "macos")]
use super::unix::AppKitRuntimeState;
#[cfg(target_os = "linux")]
use super::unix::{WaylandRuntimeState, X11RuntimeState};
#[cfg(windows)]
use super::windows::Win32RuntimeState;

/// Runtime-owned retained event-log state for one display event lane.
#[derive(Debug)]
pub(crate) struct RuntimeEventLog<T> {
    /// Sequence for the first retained live record.
    pub(crate) first_sequence: u64,
    /// Sequence to assign to the next live record.
    pub(crate) next_sequence: u64,
    /// Retained live event records.
    pub(crate) records: VecDeque<T>,
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

/// Runtime-owned registry of live display event streams.
#[derive(Debug)]
pub(crate) struct RuntimeStreamRegistry<T> {
    /// Monotonic identifier source for registered streams.
    next_stream_id: AtomicU64,
    /// Registered stream values keyed by stable stream id.
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
    pub(crate) fn next_stream_id(&self) -> u64 {
        self.next_stream_id.fetch_add(1, Ordering::Relaxed)
    }

    /// Register one live stream value.
    pub(crate) fn register(&self, stream_id: u64, stream: Arc<T>) {
        self.streams
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .insert(stream_id, stream);
    }

    /// Unregister one live stream value.
    pub(crate) fn unregister(&self, stream_id: u64) -> Option<Arc<T>> {
        self.streams
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .remove(&stream_id)
    }

    /// Return one snapshot of the live stream values.
    pub(crate) fn snapshot(&self) -> Vec<Arc<T>> {
        self.streams
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .values()
            .cloned()
            .collect()
    }
}

/// Runtime-owned optional snapshot cache for one display backend domain.
#[derive(Debug, Default)]
pub(crate) struct RuntimeSnapshotCache<T> {
    /// Cached snapshot value when one baseline exists.
    snapshot: Mutex<Option<T>>,
}

impl<T> RuntimeSnapshotCache<T> {
    /// Initialize the cache when no baseline exists yet.
    pub(crate) fn initialize(&self, snapshot: T) -> bool {
        let mut cached_snapshot = self
            .snapshot
            .lock()
            .unwrap_or_else(|error| error.into_inner());

        // preserve the existing baseline once publication has started
        if cached_snapshot.is_some() {
            return false;
        }

        *cached_snapshot = Some(snapshot);
        true
    }

    /// Replace the cache with one fresh baseline.
    pub(crate) fn reset(&self, snapshot: T) {
        let mut cached_snapshot = self
            .snapshot
            .lock()
            .unwrap_or_else(|error| error.into_inner());

        *cached_snapshot = Some(snapshot);
    }

    /// Replace the cache and return one delta built from the previous baseline.
    pub(crate) fn replace<R>(
        &self,
        snapshot: T,
        build_delta: impl FnOnce(&T, &T) -> R,
    ) -> Option<R> {
        let mut cached_snapshot = self
            .snapshot
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let Some(previous_snapshot) = cached_snapshot.as_ref() else {
            *cached_snapshot = Some(snapshot);
            return None;
        };

        let delta = build_delta(previous_snapshot, &snapshot);
        *cached_snapshot = Some(snapshot);
        Some(delta)
    }
}

/// Runtime-owned display module state.
#[derive(Default)]
pub(crate) struct PlatformDisplayState {
    /// Runtime-owned windows display-event state.
    #[cfg(windows)]
    win32_runtime_state: OnceLock<Arc<Win32RuntimeState>>,
    /// Runtime-owned linux x11 state.
    #[cfg(target_os = "linux")]
    x11_runtime_state: OnceLock<Arc<X11RuntimeState>>,
    /// Runtime-owned linux wayland state.
    #[cfg(target_os = "linux")]
    wayland_runtime_state: OnceLock<Arc<WaylandRuntimeState>>,
    /// Runtime-owned macOS appkit state.
    #[cfg(target_os = "macos")]
    appkit_runtime_state: OnceLock<Arc<AppKitRuntimeState>>,
}

impl std::fmt::Debug for PlatformDisplayState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PlatformDisplayState")
            .finish_non_exhaustive()
    }
}

impl PlatformDisplayState {
    /// Return runtime-owned Win32 display state.
    #[cfg(windows)]
    pub(crate) fn win32_runtime_state(
        &self,
        initialize: impl FnOnce() -> Win32RuntimeState,
    ) -> Arc<Win32RuntimeState> {
        Arc::clone(
            self.win32_runtime_state
                .get_or_init(|| Arc::new(initialize())),
        )
    }

    /// Return existing Win32 display state when it was initialized already.
    #[cfg(windows)]
    pub(crate) fn existing_win32_runtime_state(&self) -> Option<Arc<Win32RuntimeState>> {
        self.win32_runtime_state.get().map(Arc::clone)
    }

    /// Return runtime-owned linux x11 display state.
    #[cfg(target_os = "linux")]
    pub(crate) fn x11_runtime_state(
        &self,
        initialize: impl FnOnce() -> X11RuntimeState,
    ) -> Arc<X11RuntimeState> {
        Arc::clone(
            self.x11_runtime_state
                .get_or_init(|| Arc::new(initialize())),
        )
    }

    /// Return existing linux x11 display state when it was initialized already.
    #[cfg(target_os = "linux")]
    pub(crate) fn existing_x11_runtime_state(&self) -> Option<Arc<X11RuntimeState>> {
        self.x11_runtime_state.get().map(Arc::clone)
    }

    /// Return runtime-owned linux wayland display state.
    #[cfg(target_os = "linux")]
    pub(crate) fn wayland_runtime_state(
        &self,
        initialize: impl FnOnce() -> WaylandRuntimeState,
    ) -> Arc<WaylandRuntimeState> {
        Arc::clone(
            self.wayland_runtime_state
                .get_or_init(|| Arc::new(initialize())),
        )
    }

    /// Return existing linux wayland display state when it was initialized already.
    #[cfg(target_os = "linux")]
    pub(crate) fn existing_wayland_runtime_state(&self) -> Option<Arc<WaylandRuntimeState>> {
        self.wayland_runtime_state.get().map(Arc::clone)
    }

    /// Return runtime-owned macOS appkit display state.
    #[cfg(target_os = "macos")]
    pub(crate) fn appkit_runtime_state(
        &self,
        initialize: impl FnOnce() -> AppKitRuntimeState,
    ) -> Arc<AppKitRuntimeState> {
        Arc::clone(
            self.appkit_runtime_state
                .get_or_init(|| Arc::new(initialize())),
        )
    }
}
