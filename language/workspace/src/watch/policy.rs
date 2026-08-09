use std::time::Duration;

/// Default watch coalescing window in milliseconds.
const DEFAULT_COALESCE_WINDOW_MILLISECONDS: u64 = 50;

/// Default maximum watch batch size.
const DEFAULT_BATCH_SIZE: usize = 1024;

/// Policy for batching workspace watch events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WatchPolicy {
    /// The maximum time to coalesce events into a batch.
    pub coalesce_window: Duration,
    /// The maximum number of events and status updates per batch.
    pub max_batch_size: usize,
}

impl Default for WatchPolicy {
    /// Return the default watch policy.
    fn default() -> Self {
        Self {
            coalesce_window: Duration::from_millis(DEFAULT_COALESCE_WINDOW_MILLISECONDS),
            max_batch_size: DEFAULT_BATCH_SIZE,
        }
    }
}
