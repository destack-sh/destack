use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use parking_lot::Mutex;

/// Control flags for protocol servers.
#[derive(Debug, Clone)]
pub struct ProtocolServerControl {
    /// Shared shutdown flag.
    shutdown: Arc<AtomicBool>,
    /// Activity tracker for connection leases.
    activity: Arc<ProtocolServerActivity>,
}

impl ProtocolServerControl {
    /// Create a new control handle.
    pub fn new(shutdown: Arc<AtomicBool>) -> Self {
        Self::with_activity(shutdown, Arc::new(ProtocolServerActivity::default()))
    }

    /// Create a new control handle with explicit activity tracking.
    pub fn with_activity(shutdown: Arc<AtomicBool>, activity: Arc<ProtocolServerActivity>) -> Self {
        Self { shutdown, activity }
    }

    /// Request a daemon shutdown.
    pub fn request_shutdown(&self) {
        self.shutdown.store(true, Ordering::SeqCst);
    }

    /// Register a connection lease.
    pub fn register_connection(&self) {
        self.activity.register_connection();
    }

    /// Release a connection lease.
    pub fn unregister_connection(&self) {
        self.activity.unregister_connection();
    }

    /// Register a root handle lease.
    pub fn register_handle(&self) {
        self.activity.register_handle();
    }

    /// Release a root handle lease.
    pub fn unregister_handle(&self) {
        self.activity.unregister_handle();
    }

    /// Mark activity on the connection.
    pub fn touch_activity(&self) {
        self.activity.touch();
    }

    /// Return true if shutdown has been requested.
    pub fn is_shutting_down(&self) -> bool {
        self.shutdown.load(Ordering::SeqCst)
    }

    /// Return true when the server should shut down.
    pub fn should_shutdown(&self) -> bool {
        self.activity.should_shutdown()
    }
}

impl Default for ProtocolServerControl {
    /// Return a default control handle.
    fn default() -> Self {
        Self::new(Arc::new(AtomicBool::new(false)))
    }
}

/// Activity tracker for server shutdown policy.
#[derive(Debug)]
pub struct ProtocolServerActivity {
    /// Tracked activity state.
    state: Mutex<ProtocolServerActivityState>,
    /// Idle timeout for shutdown.
    idle_shutdown: Option<Duration>,
}

impl ProtocolServerActivity {
    /// Create a new activity tracker.
    pub fn new(idle_shutdown: Option<Duration>) -> Self {
        Self {
            state: Mutex::new(ProtocolServerActivityState::new()),
            idle_shutdown,
        }
    }

    /// Register a connection lease.
    pub fn register_connection(&self) {
        let mut state = self.state.lock();
        state.active_connections = state.active_connections.saturating_add(1);
        state.last_activity = Instant::now();
    }

    /// Release a connection lease.
    pub fn unregister_connection(&self) {
        let mut state = self.state.lock();
        state.active_connections = state.active_connections.saturating_sub(1);
        state.last_activity = Instant::now();
    }

    /// Register a root handle lease.
    pub fn register_handle(&self) {
        let mut state = self.state.lock();
        state.active_handles = state.active_handles.saturating_add(1);
        state.last_activity = Instant::now();
    }

    /// Release a root handle lease.
    pub fn unregister_handle(&self) {
        let mut state = self.state.lock();
        state.active_handles = state.active_handles.saturating_sub(1);
        state.last_activity = Instant::now();
    }

    /// Record activity on the server.
    pub fn touch(&self) {
        self.state.lock().last_activity = Instant::now();
    }

    /// Return true when the daemon should shut down for idleness.
    pub fn should_shutdown(&self) -> bool {
        let Some(idle_shutdown) = self.idle_shutdown else {
            return false;
        };

        let state = self.state.lock();
        if state.active_connections > 0 || state.active_handles > 0 {
            return false;
        }

        state.last_activity.elapsed() >= idle_shutdown
    }
}

impl Default for ProtocolServerActivity {
    /// Return default activity tracking state.
    fn default() -> Self {
        Self::new(None)
    }
}

/// Activity state used for idle shutdown checks.
#[derive(Debug)]
struct ProtocolServerActivityState {
    /// Number of active connections.
    active_connections: usize,
    /// Number of active root handles.
    active_handles: usize,
    /// Last activity timestamp.
    last_activity: Instant,
}

impl ProtocolServerActivityState {
    /// Create a fresh activity state.
    fn new() -> Self {
        Self {
            active_connections: 0,
            active_handles: 0,
            last_activity: Instant::now(),
        }
    }
}
