use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use super::DaemonActivity;

/// Shared lifecycle of one daemon process.
#[derive(Debug, Clone)]
pub struct DaemonLifecycle {
    /// Whether orderly shutdown was requested.
    is_shutdown: Arc<AtomicBool>,
    /// Connection activity.
    activity: Arc<DaemonActivity>,
}

impl DaemonLifecycle {
    /// Create one daemon lifecycle.
    pub fn new(is_shutdown: Arc<AtomicBool>, activity: Arc<DaemonActivity>) -> Self {
        Self {
            is_shutdown,
            activity,
        }
    }

    /// Request orderly daemon shutdown.
    pub fn shutdown(&self) {
        self.is_shutdown.store(true, Ordering::Release);
    }

    /// Register one accepted connection.
    pub fn connect(&self) {
        self.activity.connect();
    }

    /// Release one terminated connection.
    pub fn disconnect(&self) {
        self.activity.disconnect();
    }

    /// Return whether shutdown was requested.
    pub fn is_shutdown(&self) -> bool {
        self.is_shutdown.load(Ordering::Acquire)
    }

    /// Shut down when the configured idle timeout has elapsed.
    pub fn shutdown_if_idle(&self) -> bool {
        if self.activity.is_idle() {
            self.shutdown();

            true
        } else {
            false
        }
    }
}
