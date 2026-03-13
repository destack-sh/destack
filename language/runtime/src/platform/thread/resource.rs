use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex};

/// Shared completion state for one spawned thread.
pub(crate) struct ThreadCompletion {
    /// Machine-word result published by the thread entry callback.
    pub(crate) exit_code: AtomicU64,
}

/// Lifecycle state for one joinable thread handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ThreadLifecycleState {
    /// The handle is still available for join or detach.
    Joinable,
    /// One join operation is currently consuming the handle.
    Joining,
    /// One detach operation is currently consuming the handle.
    Detaching,
}

/// Marker payload for one thread-local key resource.
#[derive(Debug)]
pub(crate) struct ThreadLocalResource {
    /// Native host TLS key.
    #[cfg(unix)]
    pub(crate) key: libc::pthread_key_t,
    /// Native host TLS key.
    #[cfg(windows)]
    pub(crate) key: u32,
    /// Native host TLS key.
    #[cfg(not(any(unix, windows)))]
    pub(crate) key: u64,
}

/// Shared payload for one spawned thread handle.
pub(crate) struct ThreadResource {
    /// Native host thread handle used for join and detach operations.
    #[cfg(unix)]
    pub(crate) native_handle: libc::pthread_t,
    /// Native host thread handle used for join and detach operations.
    #[cfg(windows)]
    pub(crate) native_handle: windows_sys::Win32::Foundation::HANDLE,
    /// Shared completion state used for join results.
    pub(crate) completion: Arc<ThreadCompletion>,
    /// Current lifecycle state for this handle.
    pub(crate) lifecycle: Mutex<ThreadLifecycleState>,
}
