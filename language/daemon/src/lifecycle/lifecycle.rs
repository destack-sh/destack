use std::collections::HashSet;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crossbeam_channel::{Receiver, Sender, TrySendError, bounded};
use parking_lot::Mutex;

/// Shared lifecycle of one daemon process.
#[derive(Debug, Clone)]
pub(crate) struct DaemonLifecycle {
    /// Mutable process state.
    state: Arc<Mutex<LifecycleState>>,
    /// Wake signal for lifecycle transitions.
    notifications: Receiver<()>,
    /// Sender paired with the wake signal.
    notification_sender: Sender<()>,
}

impl DaemonLifecycle {
    /// Create one daemon lifecycle.
    pub(crate) fn new(idle_timeout: Option<Duration>) -> Self {
        let (notification_sender, notifications) = bounded(1);
        let state = LifecycleState {
            is_shutdown: false,
            next_connection: 1,
            connections: HashSet::new(),
            shutdown_connection: None,
            idle_timeout,
            last_activity: Instant::now(),
        };

        Self {
            state: Arc::new(Mutex::new(state)),
            notifications,
            notification_sender,
        }
    }

    /// Request orderly daemon shutdown.
    pub(crate) fn shutdown(&self) {
        self.state.lock().is_shutdown = true;
        self.notify();
    }

    /// Request shutdown after one calling connection disconnects.
    pub(crate) fn shutdown_after(&self, connection: ConnectionId) {
        let mut state = self.state.lock();
        if !state.is_shutdown {
            state.is_shutdown = true;
            state.shutdown_connection = Some(connection);
        }
        drop(state);
        self.notify();
    }

    /// Register one accepted connection until its returned activity is dropped.
    pub(crate) fn connect(&self) -> ConnectionActivity {
        let mut state = self.state.lock();
        let identifier = ConnectionId(state.next_connection);
        state.next_connection += 1;
        state.connections.insert(identifier);
        state.last_activity = Instant::now();
        drop(state);
        self.notify();

        ConnectionActivity {
            identifier,
            lifecycle: self.clone(),
        }
    }

    /// Return whether shutdown was requested.
    pub(crate) fn is_shutdown(&self) -> bool {
        self.state.lock().is_shutdown
    }

    /// Return whether the connection that requested shutdown remains active.
    pub(crate) fn is_shutdown_connection_active(&self) -> bool {
        let state = self.state.lock();
        state
            .shutdown_connection
            .is_some_and(|connection| state.connections.contains(&connection))
    }

    /// Return the deadline for the current idle period.
    pub(crate) fn idle_deadline(&self) -> Option<Instant> {
        let state = self.state.lock();
        if !state.connections.is_empty() {
            return None;
        }
        let timeout = state.idle_timeout?;

        Some(state.last_activity + timeout)
    }

    /// Shut down when the current idle deadline has elapsed.
    pub(crate) fn shutdown_if_idle(&self) -> bool {
        let Some(deadline) = self.idle_deadline() else {
            return false;
        };
        if Instant::now() < deadline {
            return false;
        }

        self.shutdown();

        true
    }

    /// Return lifecycle transition notifications.
    pub(crate) const fn notifications(&self) -> &Receiver<()> {
        &self.notifications
    }

    /// Release one live connection.
    fn disconnect(&self, connection: ConnectionId) {
        let mut state = self.state.lock();
        state.connections.remove(&connection);
        state.last_activity = Instant::now();
        drop(state);
        self.notify();
    }

    /// Wake the daemon after a lifecycle transition.
    fn notify(&self) {
        match self.notification_sender.try_send(()) {
            Ok(()) | Err(TrySendError::Full(())) => {}
            Err(TrySendError::Disconnected(())) => {}
        }
    }
}

/// Activity registration owned by one live daemon connection.
#[derive(Debug)]
pub(crate) struct ConnectionActivity {
    /// This connection's process-local identifier.
    identifier: ConnectionId,
    /// Shared daemon lifecycle.
    lifecycle: DaemonLifecycle,
}

impl ConnectionActivity {
    /// Return this connection's process-local identifier.
    pub(crate) const fn identifier(&self) -> ConnectionId {
        self.identifier
    }
}

impl Drop for ConnectionActivity {
    /// Release this connection's activity registration.
    fn drop(&mut self) {
        self.lifecycle.disconnect(self.identifier);
    }
}

/// Process-local identity of one live daemon connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct ConnectionId(u64);

/// Mutable daemon lifecycle values.
#[derive(Debug)]
struct LifecycleState {
    /// Whether orderly shutdown was requested.
    is_shutdown: bool,
    /// Next process-local connection identifier.
    next_connection: u64,
    /// Live RPC connections.
    connections: HashSet<ConnectionId>,
    /// Connection whose completed shutdown response gates teardown.
    shutdown_connection: Option<ConnectionId>,
    /// Optional duration before idle shutdown.
    idle_timeout: Option<Duration>,
    /// Last connection transition.
    last_activity: Instant,
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::DaemonLifecycle;

    /// Keep a daemon active while any connection remains.
    #[test]
    fn test_track_connections() {
        let lifecycle = DaemonLifecycle::new(Some(Duration::ZERO));
        let connection = lifecycle.connect();
        assert!(!lifecycle.shutdown_if_idle());

        drop(connection);
        assert!(lifecycle.shutdown_if_idle());
    }

    /// Retain shutdown until its exact calling connection disconnects.
    #[test]
    fn test_wait_for_shutdown_connection() {
        let lifecycle = DaemonLifecycle::new(None);
        let other = lifecycle.connect();
        let shutdown = lifecycle.connect();
        lifecycle.shutdown_after(shutdown.identifier());

        drop(other);
        assert!(lifecycle.is_shutdown_connection_active());

        drop(shutdown);
        assert!(!lifecycle.is_shutdown_connection_active());
    }
}
